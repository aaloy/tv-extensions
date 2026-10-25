// (C) 2025 - Enzo Lombardi

//! Form data - bind fields to a record, validate them, and edit records.
//!
//! A form for a record type, [`Form::for_record`], binds each field to one of
//! the record's members through a *lens*: a closure from the record to the
//! member, `|c| &mut c.name`. The same closure lets the form fill the field
//! from the record and write the edited value back, with the member's own
//! type: a `u32` field holds a `u32`, an `Option<NaiveDate>` field an
//! optional date.
//!
//! ```no_run
//! use turbo_vision::app::Application;
//! use tv_extensions::forms::{Form, ValidationErrors};
//!
//! #[derive(Clone, Default)]
//! struct Customer {
//!     id: i64,               // not in the form: kept as it was
//!     name: String,
//!     email: Option<String>, // empty field = None
//!     age: u32,              // must parse as a whole number
//!     vip: bool,
//! }
//!
//! # fn main() -> turbo_vision::core::error::Result<()> {
//! # let mut app = Application::with_terminal(turbo_vision::test_util::test_terminal(80, 25));
//! let mut form = Form::<Customer>::for_record("Customer");
//! form.input("~N~ame", |c| &mut c.name).required();
//! let email = form
//!     .input("~E~mail", |c| &mut c.email)
//!     .validate(|e| match e {
//!         Some(e) if !e.contains('@') => Err("Email must contain @".into()),
//!         _ => Ok(()),
//!     })
//!     .id();
//! form.input("~A~ge", |c| &mut c.age);
//! form.check("~V~IP customer", |c| &mut c.vip);
//! form.validate_record(move |c, errors| {
//!     if c.vip && c.email.is_none() {
//!         errors.add(email, "VIP customers need an email");
//!     }
//! });
//! form.ok_cancel();
//! let mut editor = form.build_editor();
//!
//! // `Some` once every field is valid and OK was pressed; `None` on Cancel.
//! if let Some(customer) = editor.edit(&mut app, Customer::default()) {
//!     // save it
//! #   let _ = customer;
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Validation
//!
//! When a button other than Cancel closes the dialog, the editor reads every
//! field in order:
//!
//! 1. **Required**: a field marked [`required`](Field::required) must not be
//!    empty (a check box must be checked).
//! 2. **Conversion**: the text must convert to the member's type
//!    ([`TextValue`]); "Age must be a whole number" otherwise. An empty field
//!    whose type has no empty value (a number, a date) is reported as
//!    required.
//! 3. **Field rules**: each [`validate`](Field::validate) closure, in order.
//! 4. **Record rules**: once every field is valid, each
//!    [`validate_record`](Form::validate_record) closure sees the whole
//!    record, for rules that involve several fields.
//! 5. **Submit**: [`Editor::edit_with`] hands the record to your closure, to
//!    save it; any errors it returns (a duplicate key, say) are shown too.
//!
//! If anything fails, the dialog stays open: the labels of the invalid fields
//! turn red, the error line above the buttons shows the message for the
//! focused field (or the first one), and the focus moves to the first
//! invalid field. From then on the errors are checked again whenever the
//! focus moves, so they clear as they are fixed.
//!
//! Messages are complete sentences. The form writes its own from the field's
//! label ("Name is required"); the messages your rules return are shown as
//! they are.

use super::{Cell, Form, Item, Line, find_view_mut, focus_view, focused_view, size};
use std::cell::Cell as StdCell;
use std::fmt;
use turbo_vision::app::{Application, ModalTick};
use turbo_vision::core::command::{CM_CANCEL, CommandId};
use turbo_vision::core::draw::DrawBuffer;
use turbo_vision::core::event::Event;
use turbo_vision::core::geometry::Rect;
use turbo_vision::core::palette::{STATIC_TEXT_NORMAL, TvColor};
use turbo_vision::terminal::Terminal;
use turbo_vision::views::checkbox::CheckBox;
use turbo_vision::views::combo_box::ComboBox;
use turbo_vision::views::dialog::Dialog;
use turbo_vision::views::group::GroupLike;
use turbo_vision::views::input_line::InputLine;
use turbo_vision::views::label::Label;
use turbo_vision::views::memo::Memo;
use turbo_vision::views::view::{View, ViewCore, ViewId, write_line_to_terminal};

// ---- identities and errors -------------------------------------------------

/// Names one bound field of a form, to attach an error to it.
///
/// Every binding method returns a [`Field`]; its [`id`](Field::id) is this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldId(ViewId);

impl FieldId {
    /// The id of the field's view in the dialog, for
    /// [`GroupLike::child_by_id`].
    #[must_use]
    pub fn view_id(self) -> ViewId {
        self.0
    }
}

/// One validation message, about one field or about the whole form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError {
    /// The field it is about; `None` for the form as a whole.
    pub field: Option<FieldId>,
    /// A complete sentence, shown as it is.
    pub message: String,
}

impl FieldError {
    /// An error about `field`.
    pub fn new(field: FieldId, message: impl Into<String>) -> Self {
        Self {
            field: Some(field),
            message: message.into(),
        }
    }

    /// An error about the form as a whole.
    pub fn form(message: impl Into<String>) -> Self {
        Self {
            field: None,
            message: message.into(),
        }
    }
}

/// The errors found in a form, in the order they were found.
///
/// Record rules add to it; a submit closure returns one to keep the dialog
/// open. `Err(FieldError::new(..).into())` makes one from a single error.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValidationErrors {
    errors: Vec<FieldError>,
}

impl ValidationErrors {
    /// No errors.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an error about `field`.
    pub fn add(&mut self, field: FieldId, message: impl Into<String>) {
        self.errors.push(FieldError::new(field, message));
    }

    /// Add an error about the form as a whole.
    pub fn add_form(&mut self, message: impl Into<String>) {
        self.errors.push(FieldError::form(message));
    }

    /// Whether there are no errors.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// How many errors there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Every error, in order.
    pub fn iter(&self) -> impl Iterator<Item = &FieldError> {
        self.errors.iter()
    }

    /// The first message about `field`, if any.
    #[must_use]
    pub fn field(&self, field: FieldId) -> Option<&str> {
        self.errors
            .iter()
            .find(|e| e.field == Some(field))
            .map(|e| e.message.as_str())
    }

    /// `Ok(())` when there are no errors, else `Err(self)`.
    ///
    /// # Errors
    ///
    /// `self`, when it holds any error.
    pub fn into_result(self) -> Result<(), Self> {
        if self.is_empty() { Ok(()) } else { Err(self) }
    }
}

impl From<FieldError> for ValidationErrors {
    fn from(error: FieldError) -> Self {
        Self {
            errors: vec![error],
        }
    }
}

impl Extend<FieldError> for ValidationErrors {
    fn extend<I: IntoIterator<Item = FieldError>>(&mut self, iter: I) {
        self.errors.extend(iter);
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let messages: Vec<&str> = self.errors.iter().map(|e| e.message.as_str()).collect();
        f.write_str(&messages.join("; "))
    }
}

impl std::error::Error for ValidationErrors {}

// ---- values that live in an input line --------------------------------------

/// A type an input line can hold: how to show it as text, and how to read it
/// back. [`Form::input`] takes a member of any such type.
///
/// Implemented for `String`, every integer and float type, `chrono`'s
/// `NaiveDate` (`2026-10-03`) and `NaiveTime` (`14:30`), and `Option` of any of
/// them, where an empty field is `None`. Implement it for your own types (a
/// decimal amount, an id) to bind them the same way.
pub trait TextValue: Sized {
    /// The text the field shows for this value.
    fn to_text(&self) -> String;

    /// Read the value from the field's text. On failure, say what the text
    /// should be, to finish the sentence "Age ...": `"must be a whole
    /// number"`.
    ///
    /// # Errors
    ///
    /// That message, when the text is not a valid value.
    fn from_text(text: &str) -> Result<Self, String>;

    /// How wide the field is, or `None` to stretch to its column (default).
    #[must_use]
    fn width() -> Option<i16> {
        None
    }

    /// The most characters the field accepts (default 255).
    #[must_use]
    fn max_len() -> usize {
        255
    }
}

impl TextValue for String {
    fn to_text(&self) -> String {
        self.clone()
    }

    fn from_text(text: &str) -> Result<Self, String> {
        Ok(text.to_string())
    }
}

macro_rules! whole_numbers {
    ($($t:ty),*) => {$(
        impl TextValue for $t {
            fn to_text(&self) -> String {
                self.to_string()
            }
            fn from_text(text: &str) -> Result<Self, String> {
                text.trim()
                    .parse()
                    .ok()
                    .ok_or_else(|| "must be a whole number".to_string())
            }
            fn width() -> Option<i16> {
                Some(12)
            }
            fn max_len() -> usize {
                20
            }
        }
    )*};
}
whole_numbers!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);

macro_rules! numbers {
    ($($t:ty),*) => {$(
        impl TextValue for $t {
            fn to_text(&self) -> String {
                self.to_string()
            }
            fn from_text(text: &str) -> Result<Self, String> {
                text.trim()
                    .parse()
                    .ok()
                    .ok_or_else(|| "must be a number".to_string())
            }
            fn width() -> Option<i16> {
                Some(14)
            }
            fn max_len() -> usize {
                32
            }
        }
    )*};
}
numbers!(f32, f64);

#[cfg(feature = "chrono")]
impl TextValue for chrono::NaiveDate {
    fn to_text(&self) -> String {
        self.format("%Y-%m-%d").to_string()
    }

    fn from_text(text: &str) -> Result<Self, String> {
        chrono::NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d")
            .ok()
            .ok_or_else(|| "must be a date like 2026-12-31".to_string())
    }

    fn width() -> Option<i16> {
        Some(12)
    }

    fn max_len() -> usize {
        10
    }
}

#[cfg(feature = "chrono")]
impl TextValue for chrono::NaiveTime {
    fn to_text(&self) -> String {
        self.format("%H:%M").to_string()
    }

    fn from_text(text: &str) -> Result<Self, String> {
        let text = text.trim();
        chrono::NaiveTime::parse_from_str(text, "%H:%M")
            .or_else(|_| chrono::NaiveTime::parse_from_str(text, "%H:%M:%S"))
            .ok()
            .ok_or_else(|| "must be a time like 14:30".to_string())
    }

    fn width() -> Option<i16> {
        Some(10)
    }

    fn max_len() -> usize {
        8
    }
}

impl<T: TextValue> TextValue for Option<T> {
    fn to_text(&self) -> String {
        self.as_ref().map(T::to_text).unwrap_or_default()
    }

    fn from_text(text: &str) -> Result<Self, String> {
        if text.trim().is_empty() {
            Ok(None)
        } else {
            T::from_text(text).map(Some)
        }
    }

    fn width() -> Option<i16> {
        T::width()
    }

    fn max_len() -> usize {
        T::max_len()
    }
}

// ---- bindings --------------------------------------------------------------

/// What a field holds, as read from its control.
struct Raw<T> {
    /// Nothing entered: empty text, no choice, an unchecked box.
    empty: bool,
    /// The value, or what the input should be.
    value: Result<T, String>,
}

/// A field rule: `Err` with a message rejects the value.
type Check<T> = Box<dyn Fn(&T) -> Result<(), String>>;
/// The way from a record to one of its members.
type Lens<R, T> = Box<dyn Fn(&mut R) -> &mut T>;
/// How a field reads its value from control `V`.
type Reader<V, T> = Box<dyn Fn(&V) -> Raw<T>>;
/// How a field shows a value in control `V`.
type Writer<V, T> = Box<dyn Fn(&mut V, &T)>;

/// A field's rules.
struct Rules<T> {
    required: bool,
    required_message: Option<String>,
    invalid_message: Option<String>,
    checks: Vec<Check<T>>,
}

impl<T> Default for Rules<T> {
    fn default() -> Self {
        Self {
            required: false,
            required_message: None,
            invalid_message: None,
            checks: Vec::new(),
        }
    }
}

/// A rule over the whole record.
pub(crate) type RecordRule<R> = Box<dyn Fn(&R, &mut ValidationErrors)>;

/// A field bound to a record member, with its type forgotten.
pub(crate) trait Binding<R> {
    fn id(&self) -> FieldId;
    /// The field's label, to mark it while the field is invalid.
    fn label(&self) -> Option<ViewId>;
    /// Show the record's member in the field.
    fn load(&self, record: &mut R, dialog: &mut Dialog);
    /// Check the field and write its value into the record; the error
    /// message otherwise.
    fn store(&self, dialog: &mut Dialog, record: &mut R) -> Result<(), String>;
}

/// A field of control type `V` bound to a member of type `T`.
struct Bound<R, V, T> {
    id: FieldId,
    label: Option<ViewId>,
    /// The field's name in messages: its label without `~` and `:`.
    name: String,
    lens: Lens<R, T>,
    read: Reader<V, T>,
    write: Writer<V, T>,
    rules: Rules<T>,
}

impl<R, V: View + 'static, T> Binding<R> for Bound<R, V, T> {
    fn id(&self) -> FieldId {
        self.id
    }

    fn label(&self) -> Option<ViewId> {
        self.label
    }

    fn load(&self, record: &mut R, dialog: &mut Dialog) {
        let value = (self.lens)(record);
        if let Some(view) = view_mut::<V>(dialog, self.id.0) {
            (self.write)(view, value);
        }
    }

    fn store(&self, dialog: &mut Dialog, record: &mut R) -> Result<(), String> {
        let Some(view) = view_mut::<V>(dialog, self.id.0) else {
            return Ok(());
        };
        let raw = (self.read)(view);
        let required = || {
            self.rules
                .required_message
                .clone()
                .unwrap_or_else(|| format!("{} is required", self.name))
        };
        if raw.empty && self.rules.required {
            return Err(required());
        }
        let value = match raw.value {
            Ok(value) => value,
            // Empty, and the type has no empty value: a number, a date.
            Err(_) if raw.empty => return Err(required()),
            Err(should) => {
                return Err(self
                    .rules
                    .invalid_message
                    .clone()
                    .unwrap_or_else(|| format!("{} {should}", self.name)));
            }
        };
        for check in &self.rules.checks {
            check(&value)?;
        }
        *(self.lens)(record) = value;
        Ok(())
    }
}

/// A binding still being configured by a [`Field`].
trait Pending<R, T> {
    fn rules(&mut self) -> &mut Rules<T>;
    fn into_binding(self: Box<Self>) -> Box<dyn Binding<R>>;
}

impl<R: 'static, V: View + 'static, T: 'static> Pending<R, T> for Bound<R, V, T> {
    fn rules(&mut self) -> &mut Rules<T> {
        &mut self.rules
    }

    fn into_binding(self: Box<Self>) -> Box<dyn Binding<R>> {
        self
    }
}

/// A bound field being added to a form. Set its rules by chaining, then let
/// it go: it joins the form at the end of the statement.
///
/// ```
/// # use tv_extensions::forms::Form;
/// # #[derive(Clone, Default)] struct Customer { name: String, age: u32 }
/// # let mut form = Form::<Customer>::for_record("T");
/// form.input("~N~ame", |c| &mut c.name).required();
/// let age = form
///     .input("~A~ge", |c| &mut c.age)
///     .validate(|a| if *a < 150 { Ok(()) } else { Err("Age must be under 150".into()) })
///     .id();
/// # let _ = age;
/// ```
pub struct Field<'a, R, T> {
    form: &'a mut Form<R>,
    id: FieldId,
    bound: Option<Box<dyn Pending<R, T>>>,
    cell: Option<Cell>,
    in_line: bool,
}

#[allow(
    clippy::return_self_not_must_use,
    reason = "dropping the field is how it joins the form: `form.input(..).required();` is complete"
)]
#[allow(
    clippy::must_use_candidate,
    reason = "a Field adds itself to the form when dropped, so `form.input(..).required();` is a complete statement"
)]
impl<R, T> Field<'_, R, T> {
    /// The field's id, to attach errors to it from a record rule or a
    /// submit closure.
    #[must_use]
    pub fn id(&self) -> FieldId {
        self.id
    }

    /// The field must not be left empty ("Name is required"); a check box
    /// must be checked.
    pub fn required(mut self) -> Self {
        self.rules().required = true;
        self
    }

    /// Like [`required`](Self::required), with your own message.
    pub fn required_with(mut self, message: impl Into<String>) -> Self {
        let rules = self.rules();
        rules.required = true;
        rules.required_message = Some(message.into());
        self
    }

    /// Your own message for text that does not convert to the field's type,
    /// in place of "Age must be a whole number".
    pub fn invalid_with(mut self, message: impl Into<String>) -> Self {
        self.rules().invalid_message = Some(message.into());
        self
    }

    /// Check the value: `Err` with a complete message rejects it. Runs after
    /// the field is converted to its type; several rules run in order.
    pub fn validate(mut self, check: impl Fn(&T) -> Result<(), String> + 'static) -> Self {
        self.rules().checks.push(Box::new(check));
        self
    }

    /// Make the field `width` columns wide instead of its default.
    pub fn width(mut self, width: i16) -> Self {
        if let Some(cell) = self.cell.as_mut() {
            cell.extent.width = Some(width.max(1));
        }
        self
    }

    /// The most characters an input field accepts. No effect on other
    /// controls.
    pub fn max_len(self, max_len: usize) -> Self {
        if let Some(line) = view_mut::<InputLine>(&mut self.form.dialog, self.id.0) {
            line.set_max_length(max_len);
        }
        self
    }

    fn rules(&mut self) -> &mut Rules<T> {
        self.bound
            .as_mut()
            .expect("a field is configured before it joins the form")
            .rules()
    }
}

impl<R, T> Drop for Field<'_, R, T> {
    fn drop(&mut self) {
        let (Some(bound), Some(cell)) = (self.bound.take(), self.cell.take()) else {
            return;
        };
        let items = self.form.items_mut();
        match items.last_mut() {
            Some(Item::Line(cells)) if self.in_line => cells.push(cell),
            _ => items.push(Item::Line(vec![cell])),
        }
        self.form.bindings.push(bound.into_binding());
    }
}

// ---- binding methods -------------------------------------------------------

impl<R: 'static> Form<R> {
    /// Start a form that edits records of type `R`:
    /// `Form::<Customer>::for_record("Customer")`. Bind fields with
    /// [`input`](Self::input), [`check`](Self::check), [`memo`](Self::memo),
    /// [`choice`](Self::choice) and [`bind`](Self::bind); the layout methods
    /// work as on any form. Finish with [`build_editor`](Self::build_editor).
    #[must_use]
    pub fn for_record(title: &str) -> Self {
        Self::with_title(title)
    }

    /// Add an input line bound to a member of any [`TextValue`] type: text,
    /// a number, a date, or an `Option` of one.
    ///
    /// Numbers and dates get a field of their own width; text stretches.
    pub fn input<T: TextValue + 'static>(
        &mut self,
        label: &str,
        lens: impl Fn(&mut R) -> &mut T + 'static,
    ) -> Field<'_, R, T> {
        let (view, read, write) = input_parts::<T>();
        self.bound(false, label, label, view, lens, read, write)
    }

    /// Add a check box bound to a `bool` member, lined up with the fields.
    /// `caption` is its text; [`required`](Field::required) means it must
    /// be checked.
    pub fn check(
        &mut self,
        caption: &str,
        lens: impl Fn(&mut R) -> &mut bool + 'static,
    ) -> Field<'_, R, bool> {
        let (view, read, write) = check_parts(caption);
        let mut field = self.bound(false, "", caption, view, lens, read, write);
        field.rules().required_message = Some(format!("{} must be checked", field_name(caption)));
        field
    }

    /// Add a multi-line text box `rows` high, bound to a `String` member.
    pub fn memo(
        &mut self,
        label: &str,
        rows: i16,
        lens: impl Fn(&mut R) -> &mut String + 'static,
    ) -> Field<'_, R, String> {
        let view = Memo::new(size(0, rows.max(1)));
        let read = |m: &Memo| {
            let text = m.get_text();
            Raw {
                empty: text.trim().is_empty(),
                value: Ok(text),
            }
        };
        let write = |m: &mut Memo, t: &String| m.set_text(t);
        self.bound(false, label, label, view, lens, read, write)
    }

    /// Add a drop-down list of `options`, each a caption and the value it
    /// stands for, bound to a member of that type: an enum, a foreign key.
    ///
    /// The field starts on the option equal to the record's value, or none.
    /// Leaving no option chosen is reported as required.
    pub fn choice<T, S>(
        &mut self,
        label: &str,
        options: impl IntoIterator<Item = (S, T)>,
        lens: impl Fn(&mut R) -> &mut T + 'static,
    ) -> Field<'_, R, T>
    where
        T: Clone + PartialEq + 'static,
        S: Into<String>,
    {
        let (view, read, write) = choice_parts(options);
        self.bound(false, label, label, view, lens, read, write)
    }

    /// Bind any control: `read` takes the value from it (or says what it
    /// should be), `write` shows a value in it. For a `Spinner`, a slider,
    /// radio buttons, or a control of your own.
    pub fn bind<V: View + 'static, T: 'static>(
        &mut self,
        label: &str,
        view: V,
        lens: impl Fn(&mut R) -> &mut T + 'static,
        read: impl Fn(&V) -> Result<T, String> + 'static,
        write: impl Fn(&mut V, &T) + 'static,
    ) -> Field<'_, R, T> {
        let read = move |v: &V| Raw {
            empty: false,
            value: read(v),
        };
        self.bound(false, label, label, view, lens, read, write)
    }

    /// Add a rule over the whole record, for checks that involve several
    /// fields. It runs once every field is valid; add errors to `errors`,
    /// against a field ([`Field::id`]) or the form.
    pub fn validate_record(
        &mut self,
        rule: impl Fn(&R, &mut ValidationErrors) + 'static,
    ) -> &mut Self {
        self.record_rules.push(Box::new(rule));
        self
    }

    /// Lay the form out and return its [`Editor`]: the dialog, plus the
    /// bindings that fill it from a record and read it back.
    #[must_use]
    pub fn build_editor(mut self) -> Editor<R>
    where
        R: Clone,
    {
        let error_line = self.dialog.add(ErrorLine::new());
        self.error_line = Some(error_line);
        self.lay_out();
        Editor {
            dialog: self.dialog,
            bindings: self.bindings,
            rules: self.record_rules,
            error_line,
            record: None,
            errors: ValidationErrors::new(),
            command: 0,
        }
    }

    /// Add `view` with `label`, bound through `lens`; `name` names it in
    /// messages.
    #[allow(
        clippy::too_many_arguments,
        reason = "the one place every binding goes through; each part differs per control"
    )]
    fn bound<V: View + 'static, T: 'static>(
        &mut self,
        in_line: bool,
        label: &str,
        name: &str,
        view: V,
        lens: impl Fn(&mut R) -> &mut T + 'static,
        read: impl Fn(&V) -> Raw<T> + 'static,
        write: impl Fn(&mut V, &T) + 'static,
    ) -> Field<'_, R, T> {
        let (handle, cell) = self.cell(label, view);
        let id = FieldId(handle.id());
        let bound = Bound {
            id,
            label: cell.label.map(|(label, _)| label),
            name: field_name(name),
            lens: Box::new(lens),
            read: Box::new(read),
            write: Box::new(write),
            rules: Rules::default(),
        };
        Field {
            form: self,
            id,
            bound: Some(Box::new(bound)),
            cell: Some(cell),
            in_line,
        }
    }
}

impl<R: 'static> Line<'_, R> {
    /// [`Form::input`], to the right of the line's previous fields.
    pub fn input<T: TextValue + 'static>(
        &mut self,
        label: &str,
        lens: impl Fn(&mut R) -> &mut T + 'static,
    ) -> Field<'_, R, T> {
        let (view, read, write) = input_parts::<T>();
        self.form.bound(true, label, label, view, lens, read, write)
    }

    /// [`Form::check`], to the right of the line's previous fields.
    pub fn check(
        &mut self,
        caption: &str,
        lens: impl Fn(&mut R) -> &mut bool + 'static,
    ) -> Field<'_, R, bool> {
        let (view, read, write) = check_parts(caption);
        let mut field = self.form.bound(true, "", caption, view, lens, read, write);
        field.rules().required_message = Some(format!("{} must be checked", field_name(caption)));
        field
    }

    /// [`Form::choice`], to the right of the line's previous fields.
    pub fn choice<T, S>(
        &mut self,
        label: &str,
        options: impl IntoIterator<Item = (S, T)>,
        lens: impl Fn(&mut R) -> &mut T + 'static,
    ) -> Field<'_, R, T>
    where
        T: Clone + PartialEq + 'static,
        S: Into<String>,
    {
        let (view, read, write) = choice_parts(options);
        self.form.bound(true, label, label, view, lens, read, write)
    }

    /// [`Form::bind`], to the right of the line's previous fields.
    pub fn bind<V: View + 'static, T: 'static>(
        &mut self,
        label: &str,
        view: V,
        lens: impl Fn(&mut R) -> &mut T + 'static,
        read: impl Fn(&V) -> Result<T, String> + 'static,
        write: impl Fn(&mut V, &T) + 'static,
    ) -> Field<'_, R, T> {
        let read = move |v: &V| Raw {
            empty: false,
            value: read(v),
        };
        self.form.bound(true, label, label, view, lens, read, write)
    }
}

/// An input line for `T`, and how to read and fill it.
fn input_parts<T: TextValue + 'static>() -> (InputLine, Reader<InputLine, T>, Writer<InputLine, T>)
{
    let bounds = T::width().map_or(Rect::default(), |w| size(w, 1));
    let view = InputLine::new(bounds, T::max_len());
    let read = |line: &InputLine| {
        let text = line.text();
        Raw {
            empty: text.trim().is_empty(),
            // An attached validator (picture, range, ...) has its say first.
            value: if line.is_valid() {
                T::from_text(text)
            } else {
                Err("is not valid".to_string())
            },
        }
    };
    let write = |line: &mut InputLine, value: &T| line.set_text(value.to_text());
    (view, Box::new(read), Box::new(write))
}

/// A check box captioned `caption`, and how to read and fill it.
fn check_parts(caption: &str) -> (CheckBox, Reader<CheckBox, bool>, Writer<CheckBox, bool>) {
    let view = CheckBox::new(Rect::default(), caption);
    let read = |b: &CheckBox| Raw {
        empty: !b.is_checked(),
        value: Ok(b.is_checked()),
    };
    let write = |b: &mut CheckBox, checked: &bool| b.set_checked(*checked);
    (view, Box::new(read), Box::new(write))
}

/// A combo box of `options`, and how to read and fill it.
fn choice_parts<T: Clone + PartialEq + 'static, S: Into<String>>(
    options: impl IntoIterator<Item = (S, T)>,
) -> (ComboBox, Reader<ComboBox, T>, Writer<ComboBox, T>) {
    let (captions, values): (Vec<String>, Vec<T>) =
        options.into_iter().map(|(c, v)| (c.into(), v)).unzip();
    let view = ComboBox::with_items(Rect::default(), next_combo_id(), captions);
    let read_values = values.clone();
    let read = move |c: &ComboBox| match c.selected().and_then(|i| read_values.get(i)) {
        Some(value) => Raw {
            empty: false,
            value: Ok(value.clone()),
        },
        None => Raw {
            empty: true,
            value: Err("must be chosen".to_string()),
        },
    };
    let write = move |c: &mut ComboBox, value: &T| {
        c.set_selected(values.iter().position(|v| v == value));
    };
    (view, Box::new(read), Box::new(write))
}

thread_local! {
    /// Ids for the combo boxes forms create, counting down from the top of
    /// the range so they stay clear of the small ids applications pick.
    static NEXT_COMBO_ID: StdCell<u16> = const { StdCell::new(u16::MAX) };
}

/// A combo box id for a form's choice field.
pub(crate) fn next_combo_id() -> u16 {
    NEXT_COMBO_ID.with(|next| {
        let id = next.get();
        next.set(if id <= 0xC000 { u16::MAX } else { id - 1 });
        id
    })
}

/// A field's name in messages: its label or caption without `~`, the
/// trailing `:` or `*`, and spaces.
fn field_name(label: &str) -> String {
    let name: String = label.chars().filter(|&c| c != '~').collect();
    let name = name.trim().trim_end_matches([':', '*']).trim();
    if name.is_empty() {
        "This field".to_string()
    } else {
        name.to_string()
    }
}

#[cfg(test)]
fn view_ref<V: 'static>(dialog: &Dialog, id: ViewId) -> Option<&V> {
    dialog.child_by_id(id)?.as_any().downcast_ref()
}

fn view_mut<V: 'static>(dialog: &mut Dialog, id: ViewId) -> Option<&mut V> {
    find_view_mut(dialog, id)?.as_any_mut().downcast_mut()
}

// ---- the editor ------------------------------------------------------------

/// A form for records of type `R`, ready to edit them: made by
/// [`Form::build_editor`].
///
/// [`edit`](Self::edit) fills the dialog from a record, runs it, and returns
/// the edited record once it is valid. The same editor can edit one record
/// after another.
pub struct Editor<R> {
    dialog: Dialog,
    bindings: Vec<Box<dyn Binding<R>>>,
    rules: Vec<RecordRule<R>>,
    error_line: ViewId,
    /// The record being edited, as loaded: members without a field keep
    /// their values from it.
    record: Option<R>,
    errors: ValidationErrors,
    command: CommandId,
}

impl<R: Clone + 'static> Editor<R> {
    /// Edit `record` in the dialog. Returns the edited record when a button
    /// other than Cancel closes the dialog and every field and record rule
    /// passes; `None` when the user cancels (Cancel, Esc, the close box).
    pub fn edit(&mut self, app: &mut Application, record: R) -> Option<R> {
        self.edit_with(app, record, |_| Ok(()))
    }

    /// Like [`edit`](Self::edit), and hands the valid record to `submit`
    /// before closing: save it there. If `submit` returns errors (a duplicate
    /// key, a failed constraint) they are shown and the dialog stays open.
    ///
    /// ```no_run
    /// # use turbo_vision::app::Application;
    /// # use tv_extensions::forms::{Form, FieldError};
    /// # #[derive(Clone, Default)] struct Customer { email: String }
    /// # fn insert(_: &Customer) -> Result<(), String> { Ok(()) }
    /// # let mut app = Application::with_terminal(turbo_vision::test_util::test_terminal(80, 25));
    /// # let mut form = Form::<Customer>::for_record("T");
    /// let email = form.input("~E~mail", |c| &mut c.email).required().id();
    /// form.ok_cancel();
    /// let mut editor = form.build_editor();
    /// let saved = editor.edit_with(&mut app, Customer::default(), |c| {
    ///     insert(c).map_err(|_| FieldError::new(email, "That email is already registered").into())
    /// });
    /// ```
    pub fn edit_with(
        &mut self,
        app: &mut Application,
        record: R,
        submit: impl FnMut(&R) -> Result<(), ValidationErrors>,
    ) -> Option<R> {
        self.edit_synced(app, record, |_| false, submit)
    }

    /// Like [`edit_with`](Self::edit_with), and calls `sync` each time the
    /// focus leaves a field, with the record as the fields now describe it
    /// (only when every field reads). `sync` may change the record, for
    /// fields that depend on each other (the end of one stretch is the
    /// start of the next); it returns whether it did, and the fields then
    /// show the new values.
    pub fn edit_synced(
        &mut self,
        app: &mut Application,
        record: R,
        mut sync: impl FnMut(&mut R) -> bool,
        mut submit: impl FnMut(&R) -> Result<(), ValidationErrors>,
    ) -> Option<R> {
        self.load(record);
        self.dialog.prepare_modal(app);

        let Self {
            dialog,
            bindings,
            rules,
            error_line,
            record,
            errors,
            command,
        } = self;
        let base = record.as_ref()?;
        let mut saved = None;
        // Once errors have been shown, check again whenever the focus moves.
        let mut live = false;
        let mut focus = focused_view(dialog);

        *command = app.execute_modal(dialog, |_, dialog| {
            let end = dialog.end_state();
            if end != 0 && end != CM_CANCEL {
                let result =
                    read_record(bindings, rules, base, dialog).and_then(|r| submit(&r).map(|()| r));
                match result {
                    Ok(r) => saved = Some(r),
                    Err(found) => {
                        *errors = found;
                        focus_first_error(dialog, errors);
                        display(dialog, bindings, *error_line, errors);
                        dialog.end_modal(0); // stay open
                        live = true;
                        focus = focused_view(dialog);
                    }
                }
            } else {
                let now = focused_view(dialog);
                if now != focus {
                    focus = now;
                    if let Ok(mut current) = read_record(bindings, rules, base, dialog)
                        && sync(&mut current)
                    {
                        for binding in bindings.iter() {
                            binding.load(&mut current, dialog);
                        }
                    }
                    if live {
                        *errors = read_record(bindings, rules, base, dialog)
                            .err()
                            .unwrap_or_default();
                        display(dialog, bindings, *error_line, errors);
                    }
                }
            }
            ModalTick::Continue
        });
        if *command == CM_CANCEL { None } else { saved }
    }

    /// Fill the fields from `record`, clearing any errors shown. Members the
    /// form has no field for are kept as they are in `record`.
    pub fn load(&mut self, mut record: R) {
        for binding in &self.bindings {
            binding.load(&mut record, &mut self.dialog);
        }
        self.record = Some(record);
        self.errors = ValidationErrors::new();
        display(
            &mut self.dialog,
            &self.bindings,
            self.error_line,
            &self.errors,
        );
    }

    /// The record as the fields now describe it, if every field and record
    /// rule passes; the errors otherwise. Does not run the dialog, so it also
    /// serves tests and code that fills the fields itself.
    ///
    /// # Errors
    ///
    /// The errors of every field and record rule that failed.
    ///
    /// # Panics
    ///
    /// If no record was loaded yet ([`load`](Self::load) or `edit`).
    pub fn read(&mut self) -> Result<R, ValidationErrors> {
        let base = self
            .record
            .as_ref()
            .expect("Editor::read needs a record: call load or edit first");
        read_record(&self.bindings, &self.rules, base, &mut self.dialog)
    }

    /// Show `errors` in the dialog: invalid fields' labels in red, the
    /// message on the error line, the focus on the first invalid field.
    pub fn show_errors(&mut self, errors: ValidationErrors) {
        self.errors = errors;
        focus_first_error(&mut self.dialog, &self.errors);
        display(
            &mut self.dialog,
            &self.bindings,
            self.error_line,
            &self.errors,
        );
    }

    /// The errors shown now (empty when none).
    pub fn errors(&self) -> &ValidationErrors {
        &self.errors
    }

    /// The command of the button that closed the dialog last, to tell
    /// several accepting buttons apart ("Save" from "Save and new").
    pub fn command(&self) -> CommandId {
        self.command
    }

    /// The dialog, to add views of your own or look at a field's view.
    pub fn dialog(&self) -> &Dialog {
        &self.dialog
    }

    /// The dialog, mutably: fill a field by hand, change a control.
    pub fn dialog_mut(&mut self) -> &mut Dialog {
        &mut self.dialog
    }
}

/// Read every field into a copy of `base`, then run the record rules.
fn read_record<R: Clone>(
    bindings: &[Box<dyn Binding<R>>],
    rules: &[RecordRule<R>],
    base: &R,
    dialog: &mut Dialog,
) -> Result<R, ValidationErrors> {
    let mut record = base.clone();
    let mut errors = ValidationErrors::new();
    for binding in bindings {
        if let Err(message) = binding.store(dialog, &mut record) {
            errors.add(binding.id(), message);
        }
    }
    if errors.is_empty() {
        for rule in rules {
            rule(&record, &mut errors);
        }
    }
    errors.into_result().map(|()| record)
}

/// Move the focus to the first field with an error.
fn focus_first_error(dialog: &mut Dialog, errors: &ValidationErrors) {
    if let Some(field) = errors.iter().find_map(|e| e.field) {
        focus_view(dialog, field.0);
    }
}

/// Mark the invalid fields' labels and put the right message on the error
/// line: the focused field's, or the first one.
fn display<R>(
    dialog: &mut Dialog,
    bindings: &[Box<dyn Binding<R>>],
    error_line: ViewId,
    errors: &ValidationErrors,
) {
    for binding in bindings {
        if let Some(label) = binding.label()
            && let Some(label) = view_mut::<Label>(dialog, label)
        {
            label.set_error(errors.field(binding.id()).is_some());
        }
    }
    let focused = focused_view(dialog).map(FieldId);
    let text = summary(errors, focused);
    if let Some(line) = view_mut::<ErrorLine>(dialog, error_line) {
        line.text = text;
    }
}

/// The error line's text: the focused field's message, or the first, and how
/// many more there are.
fn summary(errors: &ValidationErrors, focused: Option<FieldId>) -> String {
    let shown = focused
        .and_then(|f| errors.field(f))
        .or_else(|| errors.iter().next().map(|e| e.message.as_str()));
    match shown {
        None => String::new(),
        Some(message) if errors.len() > 1 => {
            format!("{message} (+{} more)", errors.len() - 1)
        }
        Some(message) => message.to_string(),
    }
}

/// The row above an editor's buttons that shows the current error, in red.
pub(crate) struct ErrorLine {
    core: ViewCore,
    pub(crate) text: String,
}

impl ErrorLine {
    pub(crate) fn new() -> Self {
        Self {
            core: ViewCore {
                bounds: size(0, 1),
                palette_chain: None,
                ..ViewCore::default()
            },
            text: String::new(),
        }
    }
}

impl View for ErrorLine {
    fn core(&self) -> &ViewCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut ViewCore {
        &mut self.core
    }

    fn draw(&mut self, terminal: &mut Terminal) {
        let width = usize::try_from(self.core.bounds.width_clamped()).unwrap_or(0);
        let mut attr = self.map_color(STATIC_TEXT_NORMAL);
        attr.fg = TvColor::Red;
        let mut buf = DrawBuffer::new(width);
        buf.move_char(0, ' ', attr, width);
        let text: String = self.text.chars().take(width).collect();
        buf.move_str(0, &text, attr);
        write_line_to_terminal(terminal, 0, 0, &buf);
    }

    fn handle_event(&mut self, _event: &mut Event) {}

    fn get_palette(&self) -> Option<turbo_vision::core::palette::Palette> {
        use turbo_vision::core::palette::{Palette, palettes};
        Some(Palette::from_slice(palettes::CP_STATIC_TEXT))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use turbo_vision::core::command::CM_OK;
    use turbo_vision::core::event::{KB_ALT_O, KB_ESC};

    #[derive(Debug, Clone, Default, PartialEq)]
    enum Kind {
        #[default]
        Retail,
        Wholesale,
    }

    #[derive(Debug, Clone, Default, PartialEq)]
    struct Customer {
        id: i64,
        name: String,
        email: Option<String>,
        age: u32,
        discount: Option<u32>,
        since: Option<NaiveDate>,
        vip: bool,
        kind: Kind,
        notes: String,
    }

    fn ada() -> Customer {
        Customer {
            id: 7,
            name: "Ada".into(),
            email: Some("ada@example.org".into()),
            age: 36,
            discount: None,
            since: NaiveDate::from_ymd_opt(1843, 7, 1),
            vip: true,
            kind: Kind::Wholesale,
            notes: "First programmer".into(),
        }
    }

    /// Every field id, by name.
    struct Ids {
        name: FieldId,
        email: FieldId,
        age: FieldId,
        discount: FieldId,
        since: FieldId,
        vip: FieldId,
        kind: FieldId,
    }

    fn customer_form() -> (Form<Customer>, Ids) {
        let mut form = Form::<Customer>::for_record("Customer");
        let name = form.input("~N~ame", |c| &mut c.name).required().id();
        let email = form
            .input("~E~mail", |c| &mut c.email)
            .validate(|e| match e {
                Some(e) if !e.contains('@') => Err("Email must contain @".into()),
                _ => Ok(()),
            })
            .id();
        let mut line = form.line();
        let age = line.input("~A~ge", |c| &mut c.age).id();
        let discount = line.input("~D~iscount", |c| &mut c.discount).id();
        let since = form.input("~S~ince", |c| &mut c.since).id();
        let vip = form.check("~V~IP", |c| &mut c.vip).id();
        let kind = form
            .choice(
                "~K~ind",
                [("Retail", Kind::Retail), ("Wholesale", Kind::Wholesale)],
                |c| &mut c.kind,
            )
            .id();
        form.memo("No~t~es", 3, |c| &mut c.notes);
        form.validate_record(move |c, errors| {
            if c.vip && c.email.is_none() {
                errors.add(email, "VIP customers need an email");
            }
        });
        form.ok_cancel();
        let ids = Ids {
            name,
            email,
            age,
            discount,
            since,
            vip,
            kind,
        };
        (form, ids)
    }

    fn editor() -> (Editor<Customer>, Ids) {
        let (form, ids) = customer_form();
        (form.build_editor(), ids)
    }

    fn text(e: &Editor<Customer>, id: FieldId) -> String {
        view_ref::<InputLine>(e.dialog(), id.0)
            .unwrap()
            .text()
            .to_string()
    }

    fn set_text(e: &mut Editor<Customer>, id: FieldId, text: &str) {
        view_mut::<InputLine>(e.dialog_mut(), id.0)
            .unwrap()
            .set_text(text);
    }

    fn message(e: &mut Editor<Customer>, id: FieldId) -> Option<String> {
        e.read().err()?.field(id).map(str::to_string)
    }

    #[test]
    fn load_shows_each_member_in_its_field() {
        let (mut e, ids) = editor();
        e.load(ada());
        assert_eq!(text(&e, ids.name), "Ada");
        assert_eq!(text(&e, ids.email), "ada@example.org");
        assert_eq!(text(&e, ids.age), "36");
        assert_eq!(text(&e, ids.discount), "", "None shows as empty");
        assert_eq!(text(&e, ids.since), "1843-07-01");
        assert!(
            view_ref::<CheckBox>(e.dialog(), ids.vip.0)
                .unwrap()
                .is_checked()
        );
        let kind = view_ref::<ComboBox>(e.dialog(), ids.kind.0).unwrap();
        assert_eq!(kind.selected(), Some(1));
    }

    #[test]
    fn read_returns_the_edited_record_and_keeps_unbound_members() {
        let (mut e, ids) = editor();
        e.load(ada());
        set_text(&mut e, ids.name, "Ada Lovelace");
        set_text(&mut e, ids.discount, " 15 ");
        set_text(&mut e, ids.since, "");
        let read = e.read().expect("valid");
        assert_eq!(read.id, 7, "no field for the id: kept from the record");
        assert_eq!(read.name, "Ada Lovelace");
        assert_eq!(read.discount, Some(15));
        assert_eq!(read.since, None, "empty optional field is None");
        assert_eq!(read.notes, "First programmer");
        assert_eq!(read.kind, Kind::Wholesale);
    }

    #[test]
    fn an_unchanged_form_reads_back_the_same_record() {
        let (mut e, _) = editor();
        e.load(ada());
        assert_eq!(e.read(), Ok(ada()));
    }

    #[test]
    fn required_fields_and_values_without_an_empty_form_must_be_filled() {
        let (mut e, ids) = editor();
        e.load(ada());
        set_text(&mut e, ids.name, "   ");
        set_text(&mut e, ids.age, "");
        assert_eq!(
            message(&mut e, ids.name).as_deref(),
            Some("Name is required")
        );
        assert_eq!(message(&mut e, ids.age).as_deref(), Some("Age is required"));
        assert_eq!(
            message(&mut e, ids.discount),
            None,
            "optional: empty is fine"
        );
    }

    #[test]
    fn text_that_does_not_convert_says_what_it_should_be() {
        let (mut e, ids) = editor();
        e.load(ada());
        set_text(&mut e, ids.age, "thirty");
        set_text(&mut e, ids.since, "July 1843");
        assert_eq!(
            message(&mut e, ids.age).as_deref(),
            Some("Age must be a whole number")
        );
        assert_eq!(
            message(&mut e, ids.since).as_deref(),
            Some("Since must be a date like 2026-12-31")
        );
    }

    #[test]
    fn field_rules_and_custom_messages_are_shown_as_given() {
        let mut form = Form::<Customer>::for_record("T");
        let name = form
            .input("~N~ame", |c| &mut c.name)
            .required_with("Who is it?")
            .id();
        let age = form
            .input("~A~ge", |c| &mut c.age)
            .invalid_with("Age is a number of years")
            .validate(|a| {
                if *a < 150 {
                    Ok(())
                } else {
                    Err("Nobody is that old".into())
                }
            })
            .id();
        let mut e = form.build_editor();
        e.load(Customer::default());
        set_text(&mut e, age, "x");
        let errors = e.read().unwrap_err();
        assert_eq!(errors.field(name), Some("Who is it?"));
        assert_eq!(errors.field(age), Some("Age is a number of years"));
        set_text(&mut e, age, "200");
        assert_eq!(e.read().unwrap_err().field(age), Some("Nobody is that old"));
    }

    #[test]
    fn record_rules_run_once_the_fields_are_valid() {
        let (mut e, ids) = editor();
        let mut c = ada();
        c.email = None;
        e.load(c);
        assert_eq!(
            message(&mut e, ids.email).as_deref(),
            Some("VIP customers need an email")
        );
        // With a field error, the record rule does not run.
        set_text(&mut e, ids.age, "?");
        let errors = e.read().unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors.field(ids.email).is_none());
    }

    #[test]
    fn a_choice_left_empty_is_required() {
        let (mut e, ids) = editor();
        e.load(ada());
        view_mut::<ComboBox>(e.dialog_mut(), ids.kind.0)
            .unwrap()
            .set_selected(None);
        assert_eq!(
            message(&mut e, ids.kind).as_deref(),
            Some("Kind is required")
        );
        view_mut::<ComboBox>(e.dialog_mut(), ids.kind.0)
            .unwrap()
            .set_selected(Some(0));
        assert_eq!(e.read().unwrap().kind, Kind::Retail);
    }

    #[test]
    fn a_required_check_box_must_be_checked() {
        #[derive(Clone, Default)]
        struct Terms {
            accepted: bool,
        }
        let mut form = Form::<Terms>::for_record("T");
        let accepted = form
            .check("I ~a~ccept the terms", |t| &mut t.accepted)
            .required()
            .id();
        let mut e = form.build_editor();
        e.load(Terms::default());
        assert_eq!(
            e.read().err().unwrap().field(accepted),
            Some("I accept the terms must be checked")
        );
    }

    #[test]
    fn an_input_line_validator_has_its_say() {
        use std::cell::RefCell;
        use std::rc::Rc;
        use turbo_vision::views::validator::FilterValidator;
        let (mut e, ids) = editor();
        e.load(ada());
        view_mut::<InputLine>(e.dialog_mut(), ids.name.0)
            .unwrap()
            .set_validator(Rc::new(RefCell::new(FilterValidator::new(
                "abcdefghijklmnopqrstuvwxyz",
            ))));
        set_text(&mut e, ids.name, "ada");
        e.read().expect("lower case passes the filter");
        set_text(&mut e, ids.name, "Ada 2");
        assert_eq!(
            message(&mut e, ids.name).as_deref(),
            Some("Name is not valid")
        );
    }

    #[test]
    fn showing_errors_marks_the_labels_fills_the_error_line_and_focuses_the_first() {
        let (mut e, ids) = editor();
        e.load(ada());
        set_text(&mut e, ids.name, "");
        set_text(&mut e, ids.age, "x");
        let errors = e.read().unwrap_err();
        e.show_errors(errors);

        let label_error = |e: &Editor<Customer>, id: FieldId| {
            let b = e.bindings.iter().find(|b| b.id() == id).unwrap();
            view_ref::<Label>(e.dialog(), b.label().unwrap())
                .unwrap()
                .is_error()
        };
        assert!(label_error(&e, ids.name));
        assert!(label_error(&e, ids.age));
        assert!(!label_error(&e, ids.email));
        assert_eq!(e.dialog().group().focused_view_id(), Some(ids.name.0));
        let line = view_ref::<ErrorLine>(e.dialog(), e.error_line).unwrap();
        assert_eq!(line.text, "Name is required (+1 more)");

        e.load(ada());
        assert!(!label_error(&e, ids.name), "loading clears them");
        assert_eq!(
            view_ref::<ErrorLine>(e.dialog(), e.error_line)
                .unwrap()
                .text,
            ""
        );
    }

    #[test]
    fn the_error_line_follows_the_focused_field() {
        let mut errors = ValidationErrors::new();
        let (a, b) = (FieldId(ViewId::from_u16(1)), FieldId(ViewId::from_u16(2)));
        errors.add(a, "A is wrong");
        errors.add(b, "B is wrong");
        errors.add_form("The form is wrong");
        assert_eq!(summary(&errors, Some(b)), "B is wrong (+2 more)");
        assert_eq!(summary(&errors, None), "A is wrong (+2 more)");
        assert_eq!(summary(&ValidationErrors::new(), Some(a)), "");
    }

    #[test]
    fn field_names_drop_hot_key_marks_colons_and_stars() {
        assert_eq!(field_name("~N~ame:"), "Name");
        assert_eq!(field_name(" E-~m~ail * "), "E-mail");
        assert_eq!(field_name(""), "This field");
    }

    #[test]
    fn options_accept_empty_text_and_numbers_trim_spaces() {
        assert_eq!(Option::<u32>::from_text("  "), Ok(None));
        assert_eq!(Option::<u32>::from_text(" 4 "), Ok(Some(4)));
        assert_eq!(i32::from_text(" -3 "), Ok(-3));
        assert_eq!(u8::from_text("300"), Err("must be a whole number".into()));
        assert_eq!(f64::from_text("2.5"), Ok(2.5));
        assert_eq!(
            chrono::NaiveTime::from_text("9:05"),
            Ok(chrono::NaiveTime::from_hms_opt(9, 5, 0).unwrap())
        );
        assert_eq!(Some(1.5f32).to_text(), "1.5");
    }

    // ---- the whole cycle, through the modal loop ----

    fn app() -> Application {
        turbo_vision::views::button::set_press_animation(std::time::Duration::ZERO);
        Application::with_terminal(turbo_vision::test_util::test_terminal(100, 40))
    }

    #[test]
    fn edit_keeps_the_dialog_open_until_the_record_is_valid() {
        let mut app = app();
        let (mut e, _) = editor();
        let mut c = ada();
        c.name.clear();
        let tx = app.terminal.event_injector();
        tx.send(Event::keyboard(KB_ALT_O)).unwrap(); // refused: no name
        tx.send(Event::text('B')).unwrap(); // typed into Name, focused by the refusal
        tx.send(Event::keyboard(KB_ALT_O)).unwrap(); // accepted
        let saved = e.edit(&mut app, c).expect("saved");
        assert_eq!(saved.name, "B");
        assert_eq!(saved.id, 7);
        assert_eq!(e.command(), CM_OK);
    }

    #[test]
    fn cancel_returns_nothing_whatever_the_fields_hold() {
        let mut app = app();
        let (mut e, _) = editor();
        let mut c = ada();
        c.name.clear();
        let tx = app.terminal.event_injector();
        tx.send(Event::keyboard(KB_ESC)).unwrap();
        assert_eq!(e.edit(&mut app, c), None);
        assert_eq!(e.command(), CM_CANCEL);
    }

    #[test]
    fn a_failed_save_shows_its_errors_and_keeps_the_dialog_open() {
        let mut app = app();
        let (mut e, ids) = editor();
        let tx = app.terminal.event_injector();
        tx.send(Event::keyboard(KB_ALT_O)).unwrap(); // the save fails
        tx.send(Event::keyboard(KB_ALT_O)).unwrap(); // the save succeeds
        let mut attempts = 0;
        let email = ids.email;
        let saved = e.edit_with(&mut app, ada(), |_| {
            attempts += 1;
            if attempts == 1 {
                Err(FieldError::new(email, "That email is taken").into())
            } else {
                Ok(())
            }
        });
        assert_eq!(attempts, 2);
        assert_eq!(saved, Some(ada()));
    }
}
