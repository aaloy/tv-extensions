// (C) 2026 - Antoni Aloy

//! Forms described as JSON, answered as JSON.
//!
//! A program that cannot draw a dialog itself, an agent or an MCP tool, says
//! what it needs to ask in JSON; a person fills the form in the terminal;
//! the program gets the answers back as JSON, or a cancel. It is a thin
//! front end over the record forms of [`Form`]: the same validation, the
//! same errors shown in the dialog, the same keyboard and mouse.
//!
//! ```no_run
//! use serde_json::json;
//! use turbo_vision::app::Application;
//! use tv_extensions::forms::json::{FormError, run};
//!
//! # fn main() -> turbo_vision::core::error::Result<()> {
//! # let mut app = Application::with_terminal(turbo_vision::test_util::test_terminal(80, 25));
//! let spec = json!({
//!     "title": "New customer",
//!     "fields": [
//!         { "name": "name",  "type": "text",    "label": "Name", "required": true },
//!         { "name": "age",   "type": "integer", "min": 0, "max": 120 },
//!         { "name": "kind",  "type": "choice",  "options": ["Retail", "Wholesale"] },
//!         { "name": "vip",   "type": "boolean", "label": "VIP customer" }
//!     ]
//! });
//! match run(&mut app, &spec) {
//!     Ok(answers) => println!("{answers}"), // {"name":"Ada","age":36,"kind":"Retail","vip":false}
//!     Err(FormError::Cancelled) => println!("the person cancelled"),
//!     Err(FormError::Spec(why)) => eprintln!("bad form: {why}"),
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # The spec
//!
//! An object with a `title` (default `"Form"`) and a list of `fields`. Any
//! other key is refused, so a typo is an error rather than silently ignored.
//! Each field is an object with a `type`:
//!
//! | `type` | Shows | Answer |
//! |--------|-------|--------|
//! | `text` | one input line | string |
//! | `memo` | several lines (`rows`, default 4) | string |
//! | `integer` | an input line that must be a whole number, or a spinner or slider with `"widget"` | number, or `null` when left empty |
//! | `number` | an input line that must be a number | number, or `null` when left empty |
//! | `boolean` | a check box | `true` / `false` |
//! | `choice` | a drop-down of `options`, or radio buttons with `"widget": "radio"` | the chosen option's value |
//! | `multichoice` | one check box per option | the list of the ticked options' values |
//! | `date` | an input line for `YYYY-MM-DD` (feature `chrono`) | `"YYYY-MM-DD"`, or `null` |
//! | `section` | a heading, no answer | |
//! | `note` | a few lines of text (`text`), no answer | |
//! | `group` | a titled box around `fields` | |
//! | `row` | `fields` side by side on one line | |
//!
//! Every field except `section` has a unique `name`, the key of its answer.
//! Optional keys: `label` (the name with the first letter capitalised by
//! default; a `~` marks the Alt hot key, and one is picked for you when there
//! is none), `required`, `default` (the value shown at first), `max_len`
//! (`text`), `rows` (`memo`), `min` and `max` (`integer`, `number`), and
//! `options` and `widget` (`choice`; `"dropdown"`, the default, or `"radio"`, at most 32 options): each option a string, number or boolean standing for itself,
//! or `{ "label": "Wholesale", "value": 2 }`. A `choice` must be answered; a
//! `boolean` with `required: true` must be checked.
//!
//! A `text` with a `mask` is typed against a picture (`#` digit, `@` letter,
//! `&` upper-case letter, `!` any character; other characters are filled in
//! for the person) and is refused until it fits; empty is allowed unless
//! `required`. `integer` takes `"widget": "input"` (the default), `"spinner"` or
//! `"slider"`; the last two need whole-number `min` and `max`, start on
//! `default` or `min`, and always have a value. A `multichoice` has the
//! `choice` options and a `default` list; `required` means at least one is
//! ticked.
//!
//! # Layout
//!
//! `group` (with an optional `label` as its title) and `row` hold a `fields`
//! list of their own. Groups nest and can hold rows; a row holds plain
//! fields (no `memo`, `note`, `group`, `row` or `section`). The answers stay
//! one flat object whatever the nesting, so field names are unique across
//! the whole form. At the top of the spec, `labels` is `"left"` (default) or
//! `"above"`, and `label_align` is `"left"` (default) or `"right"`.
//!
//! The person cannot close the form with OK while a field is invalid: the
//! errors show in the dialog and the focus moves to the first one.

use super::{Editor, Field, Form, LabelAlign, LabelPosition, Line, TextValue, size};
use serde_json::{Map, Number, Value};
use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt;
use std::rc::Rc;
use turbo_vision::app::Application;
use turbo_vision::views::View;
use turbo_vision::views::cluster_group::{CheckBoxes, MAX_CLUSTER_ITEMS, RadioButtons};
use turbo_vision::views::input_line::InputLine;
use turbo_vision::views::picture_validator::PictureValidator;
use turbo_vision::views::slider::Slider;
use turbo_vision::views::spinner::Spinner;
use turbo_vision::views::static_text::StaticText;

/// Why a form gave no answers.
#[derive(Debug, Clone, PartialEq)]
pub enum FormError {
    /// The person cancelled (Cancel, Esc or the close box).
    Cancelled,
    /// The spec is not a valid form. The message says where and what, in
    /// words meant for whoever wrote the spec, such as
    /// `fields[2].type: unknown type "txt"`.
    Spec(String),
}

impl fmt::Display for FormError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => f.write_str("the form was cancelled"),
            Self::Spec(why) => write!(f, "invalid form: {why}"),
        }
    }
}

impl std::error::Error for FormError {}

/// One field's value while the form is open. A record form edits a struct,
/// so the answers live in a list of these, one per field.
#[derive(Debug, Clone)]
enum Slot {
    Text(String),
    Int(Option<i64>),
    Num(Option<f64>),
    Bool(bool),
    Choice(Value),
    Multi(Vec<Value>),
    #[cfg(feature = "chrono")]
    Date(Option<chrono::NaiveDate>),
}

/// The record a JSON form edits.
#[derive(Debug, Clone)]
struct Values(Vec<Slot>);

macro_rules! lens {
    ($name:ident, $variant:ident, $ty:ty) => {
        fn $name(values: &mut Values, index: usize) -> &mut $ty {
            match &mut values.0[index] {
                Slot::$variant(value) => value,
                other => unreachable!(
                    "slot {index} holds {other:?}, not a {}",
                    stringify!($variant)
                ),
            }
        }
    };
}
lens!(text_of, Text, String);
lens!(int_of, Int, Option<i64>);
lens!(num_of, Num, Option<f64>);
lens!(bool_of, Bool, bool);
lens!(choice_of, Choice, Value);
lens!(multi_of, Multi, Vec<Value>);
#[cfg(feature = "chrono")]
lens!(date_of, Date, Option<chrono::NaiveDate>);

/// A form built from a JSON spec: run it with [`run`](Self::run), once or
/// many times.
pub struct JsonForm {
    editor: Editor<Values>,
    initial: Values,
    /// Each answer's key and the slot that holds it, in the spec's order.
    answers: Vec<(String, usize)>,
}

/// Build the form `spec` describes, let the person fill it in, and return the
/// answers as a JSON object. A shorthand for [`JsonForm::from_spec`] then
/// [`JsonForm::run`].
///
/// # Errors
///
/// [`FormError::Spec`] when `spec` is not a valid form, [`FormError::Cancelled`]
/// when the person cancels.
pub fn run(app: &mut Application, spec: &Value) -> Result<Value, FormError> {
    JsonForm::from_spec(spec)?.run(app)
}

impl JsonForm {
    /// Build the form `spec` describes.
    ///
    /// # Errors
    ///
    /// [`FormError::Spec`] naming the first thing wrong with `spec`.
    pub fn from_spec(spec: &Value) -> Result<Self, FormError> {
        build(spec).map_err(FormError::Spec)
    }

    /// Show the form and return the answers as a JSON object keyed by field
    /// name, in the spec's order. The form can be run again; it starts from
    /// the defaults each time.
    ///
    /// # Errors
    ///
    /// [`FormError::Cancelled`] when the person cancels.
    pub fn run(&mut self, app: &mut Application) -> Result<Value, FormError> {
        match self.editor.edit(app, self.initial.clone()) {
            Some(values) => Ok(self.to_json(&values)),
            None => Err(FormError::Cancelled),
        }
    }

    fn to_json(&self, values: &Values) -> Value {
        let mut out = Map::new();
        for (name, index) in &self.answers {
            out.insert(name.clone(), slot_json(&values.0[*index]));
        }
        Value::Object(out)
    }

    /// The answers the form holds at its defaults, if they pass the rules.
    #[cfg(test)]
    fn read(&mut self) -> Result<Value, super::ValidationErrors> {
        self.editor.load(self.initial.clone());
        self.editor.read().map(|values| self.to_json(&values))
    }
}

fn slot_json(slot: &Slot) -> Value {
    match slot {
        Slot::Text(text) => Value::String(text.clone()),
        Slot::Int(n) => n.map_or(Value::Null, |n| Value::Number(n.into())),
        Slot::Num(n) => n
            .and_then(Number::from_f64)
            .map_or(Value::Null, Value::Number),
        Slot::Bool(b) => Value::Bool(*b),
        Slot::Choice(value) => value.clone(),
        Slot::Multi(values) => Value::Array(values.clone()),
        #[cfg(feature = "chrono")]
        Slot::Date(date) => date.map_or(Value::Null, |d| {
            Value::String(d.format("%Y-%m-%d").to_string())
        }),
    }
}

const SPEC_KEYS: &[&str] = &["title", "fields", "labels", "label_align"];
const FIELD_KEYS: &[&str] = &[
    "name", "type", "label", "required", "default", "max_len", "rows", "min", "max", "options",
    "widget", "text", "fields", "mask",
];
const TYPES: &str =
    "text, memo, integer, number, boolean, choice, multichoice, date, section, note, group, row";

/// The widest a spinner or slider is drawn.
const SPINNER_WIDTH: i16 = 12;
const SLIDER_WIDTH: i16 = 30;

/// What building the form has collected so far.
struct Ctx {
    hot_keys: HotKeys,
    slots: Vec<Slot>,
    /// Each answer's key and its slot, in the spec's order.
    answers: Vec<(String, usize)>,
    names: HashSet<String>,
}

/// Where a field goes: on a row of its own, or on a line beside others.
enum Target<'a, 'b> {
    Form(&'a mut Form<Values>),
    Line(&'a mut Line<'b, Values>),
}

impl Target<'_, '_> {
    fn input<T: TextValue + 'static>(
        &mut self,
        label: &str,
        lens: impl Fn(&mut Values) -> &mut T + 'static,
    ) -> Field<'_, Values, T> {
        match self {
            Self::Form(f) => f.input(label, lens),
            Self::Line(l) => l.input(label, lens),
        }
    }

    fn check(
        &mut self,
        caption: &str,
        lens: impl Fn(&mut Values) -> &mut bool + 'static,
    ) -> Field<'_, Values, bool> {
        match self {
            Self::Form(f) => f.check(caption, lens),
            Self::Line(l) => l.check(caption, lens),
        }
    }

    fn choice(
        &mut self,
        label: &str,
        options: Vec<(String, Value)>,
        lens: impl Fn(&mut Values) -> &mut Value + 'static,
    ) -> Field<'_, Values, Value> {
        match self {
            Self::Form(f) => f.choice(label, options, lens),
            Self::Line(l) => l.choice(label, options, lens),
        }
    }

    fn bind<V: View + 'static, T: 'static>(
        &mut self,
        label: &str,
        view: V,
        lens: impl Fn(&mut Values) -> &mut T + 'static,
        read: impl Fn(&V) -> Result<T, String> + 'static,
        write: impl Fn(&mut V, &T) + 'static,
    ) -> Field<'_, Values, T> {
        match self {
            Self::Form(f) => f.bind(label, view, lens, read, write),
            Self::Line(l) => l.bind(label, view, lens, read, write),
        }
    }
}

fn build(spec: &Value) -> Result<JsonForm, String> {
    let spec = spec.as_object().ok_or("the form must be a JSON object")?;
    reject_unknown(spec, SPEC_KEYS, "")?;
    let title = match spec.get("title") {
        None => "Form",
        Some(Value::String(title)) => title,
        Some(_) => return Err("title: must be a string".into()),
    };
    let fields = spec
        .get("fields")
        .and_then(Value::as_array)
        .ok_or("fields: a list of fields is required")?;
    if fields.is_empty() {
        return Err("fields: the form has no fields".into());
    }

    let mut form = Form::<Values>::for_record(title);
    match str_key(spec, "labels", "")? {
        None | Some("left") => {}
        Some("above") => {
            form.label_position(LabelPosition::Above);
        }
        Some(other) => {
            return Err(format!(
                "labels: unknown placement \"{other}\"; expected left or above"
            ));
        }
    }
    match str_key(spec, "label_align", "")? {
        None | Some("left") => {}
        Some("right") => {
            form.label_align(LabelAlign::Right);
        }
        Some(other) => {
            return Err(format!(
                "label_align: unknown alignment \"{other}\"; expected left or right"
            ));
        }
    }

    let mut ctx = Ctx {
        hot_keys: HotKeys::new(fields),
        slots: Vec::new(),
        answers: Vec::new(),
        names: HashSet::new(),
    };
    add_items(&mut form, fields, "fields", &mut ctx)?;
    if ctx.answers.is_empty() {
        return Err("fields: the form has no field to answer".into());
    }

    form.ok_cancel();
    Ok(JsonForm {
        editor: form.build_editor(),
        initial: Values(ctx.slots),
        answers: ctx.answers,
    })
}

/// Fail when `field` has a key its kind does not use.
fn only(field: &Map<String, Value>, allowed: &[&str], at: &str) -> Result<(), String> {
    reject_unknown(field, allowed, &format!("{at}."))
}

/// The nested `fields` list of a group or a row.
fn children<'a>(field: &'a Map<String, Value>, at: &str) -> Result<&'a [Value], String> {
    field
        .get("fields")
        .and_then(Value::as_array)
        .filter(|list| !list.is_empty())
        .map(Vec::as_slice)
        .ok_or(format!("{at}.fields: a list of fields is required"))
}

/// Add each item of `items` to the form: a field, a heading, a note, a group
/// or a row.
fn add_items(
    form: &mut Form<Values>,
    items: &[Value],
    path: &str,
    ctx: &mut Ctx,
) -> Result<(), String> {
    for (position, item) in items.iter().enumerate() {
        let at = format!("{path}[{position}]");
        let field = item.as_object().ok_or(format!("{at}: must be an object"))?;
        reject_unknown(field, FIELD_KEYS, &format!("{at}."))?;
        let kind = str_key(field, "type", &at)?.ok_or(format!("{at}.type: required ({TYPES})"))?;
        match kind {
            "section" => {
                only(field, &["type", "label"], &at)?;
                let label = str_key(field, "label", &at)?
                    .ok_or(format!("{at}.label: a section needs a label"))?;
                form.section(&plain(label));
            }
            "note" => {
                only(field, &["type", "text"], &at)?;
                let text = str_key(field, "text", &at)?
                    .ok_or(format!("{at}.text: a note needs its text"))?;
                let rows = i16::try_from(text.lines().count().max(1)).unwrap_or(i16::MAX);
                form.row(StaticText::new(size(0, rows), text));
            }
            "group" => {
                only(field, &["type", "label", "fields"], &at)?;
                let title = str_key(field, "label", &at)?.unwrap_or("");
                let inner = children(field, &at)?;
                form.group(&plain(title));
                add_items(form, inner, &format!("{at}.fields"), ctx)?;
                form.end_group();
            }
            "row" => {
                only(field, &["type", "fields"], &at)?;
                let inner = children(field, &at)?;
                let mut line = form.line();
                for (i, child) in inner.iter().enumerate() {
                    let child_at = format!("{at}.fields[{i}]");
                    let child = child
                        .as_object()
                        .ok_or(format!("{child_at}: must be an object"))?;
                    reject_unknown(child, FIELD_KEYS, &format!("{child_at}."))?;
                    if let Some(kind) = str_key(child, "type", &child_at)?
                        && matches!(kind, "memo" | "section" | "note" | "group" | "row")
                    {
                        return Err(format!(
                            "{child_at}.type: a {kind} cannot be in a row; put it outside"
                        ));
                    }
                    add_field(&mut Target::Line(&mut line), child, &child_at, ctx)?;
                }
            }
            _ => add_field(&mut Target::Form(form), field, &at, ctx)?,
        }
    }
    Ok(())
}

/// Which widget a field asks for, checked against the ones its type offers.
fn widget<'a>(
    field: &'a Map<String, Value>,
    kind: &str,
    at: &str,
) -> Result<Option<&'a str>, String> {
    let offered: &[&str] = match kind {
        "choice" => &["dropdown", "radio"],
        "integer" => &["input", "spinner", "slider"],
        _ => {
            return if field.contains_key("widget") {
                Err(format!("{at}.widget: a {kind} has no choice of widget"))
            } else {
                Ok(None)
            };
        }
    };
    match str_key(field, "widget", at)? {
        None => Ok(None),
        Some(w) if offered.contains(&w) => Ok(Some(w)),
        Some(other) => Err(format!(
            "{at}.widget: unknown widget \"{other}\"; expected {}",
            offered.join(", ")
        )),
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "one match arm per field type reads better than a function each"
)]
fn add_field(
    target: &mut Target<'_, '_>,
    field: &Map<String, Value>,
    at: &str,
    ctx: &mut Ctx,
) -> Result<(), String> {
    let kind = str_key(field, "type", at)?.ok_or(format!("{at}.type: required ({TYPES})"))?;
    if matches!(kind, "section" | "note" | "group" | "row") {
        return Err(format!("{at}.type: a {kind} cannot be here"));
    }
    if field.contains_key("text") || field.contains_key("fields") {
        let key = if field.contains_key("text") {
            "text"
        } else {
            "fields"
        };
        return Err(format!("{at}.{key}: a {kind} has no {key}"));
    }
    let name = str_key(field, "name", at)?
        .filter(|name| !name.is_empty())
        .ok_or(format!("{at}.name: required"))?;
    if !ctx.names.insert(name.to_string()) {
        return Err(format!("{at}.name: \"{name}\" is used twice"));
    }
    if field.contains_key("mask") && kind != "text" {
        return Err(format!("{at}.mask: only a text can have a mask"));
    }
    let widget = widget(field, kind, at)?;
    let label = ctx.hot_keys.label(field, name, at)?;
    let plain_label = plain(&label);
    let required = bool_key(field, "required", at)?.unwrap_or(false);
    let default = field.get("default");
    let index = ctx.slots.len();

    match kind {
        "text" => {
            let value = match default {
                None => String::new(),
                Some(Value::String(s)) => s.clone(),
                Some(_) => return Err(format!("{at}.default: must be a string")),
            };
            ctx.slots.push(Slot::Text(value));
            if let Some(mask) = str_key(field, "mask", at)? {
                if field.contains_key("max_len") {
                    return Err(format!(
                        "{at}.max_len: a masked text is as long as its mask"
                    ));
                }
                add_masked(target, &label, mask, index, required, at)?;
            } else {
                let mut f = target.input(&label, move |r| text_of(r, index));
                if required {
                    f = f.required();
                }
                if let Some(max) = uint_key(field, "max_len", at)? {
                    f.max_len(max);
                }
            }
        }
        "memo" => {
            let value = match default {
                None => String::new(),
                Some(Value::String(s)) => s.clone(),
                Some(_) => return Err(format!("{at}.default: must be a string")),
            };
            ctx.slots.push(Slot::Text(value));
            let rows = uint_key(field, "rows", at)?.unwrap_or(4);
            let rows = i16::try_from(rows).map_err(|_| format!("{at}.rows: too large"))?;
            let Target::Form(form) = target else {
                return Err(format!("{at}.type: a memo cannot be in a row"));
            };
            let f = form.memo(&label, rows, move |r| text_of(r, index));
            if required {
                f.required();
            }
        }
        "integer" if matches!(widget, Some("spinner" | "slider")) => {
            let (min, max) = whole_range(field, at, widget.unwrap_or_default())?;
            let start = match default {
                None => min,
                Some(v) => v
                    .as_i64()
                    .filter(|n| (min..=max).contains(n))
                    .ok_or(format!(
                        "{at}.default: must be a whole number from {min} to {max}"
                    ))?,
            };
            ctx.slots.push(Slot::Int(Some(start)));
            if widget == Some("spinner") {
                target.bind(
                    &label,
                    Spinner::new(size(SPINNER_WIDTH, 1), min, max),
                    move |r| int_of(r, index),
                    |s: &Spinner| Ok(Some(s.value())),
                    move |s: &mut Spinner, v: &Option<i64>| {
                        s.set_value(v.unwrap_or(min));
                    },
                );
            } else {
                target.bind(
                    &label,
                    Slider::new(size(SLIDER_WIDTH, 1), min, max),
                    move |r| int_of(r, index),
                    |s: &Slider| Ok(Some(s.value())),
                    move |s: &mut Slider, v: &Option<i64>| {
                        s.set_value(v.unwrap_or(min));
                    },
                );
            }
        }
        "integer" => {
            let value = match default {
                None | Some(Value::Null) => None,
                Some(v) => Some(
                    v.as_i64()
                        .ok_or(format!("{at}.default: must be a whole number"))?,
                ),
            };
            ctx.slots.push(Slot::Int(value));
            let min = limit_key(field, "min", at)?;
            let max = limit_key(field, "max", at)?;
            let mut f = target.input(&label, move |r| int_of(r, index));
            if required {
                f = f.required();
            }
            #[allow(
                clippy::cast_precision_loss,
                reason = "limits are compared as f64; whole numbers beyond 2^53 are not a form's concern"
            )]
            f.validate(move |v: &Option<i64>| {
                range_check(v.map(|n| n as f64), min, max, &plain_label)
            });
        }
        "number" => {
            let value = match default {
                None | Some(Value::Null) => None,
                Some(v) => Some(
                    v.as_f64()
                        .ok_or(format!("{at}.default: must be a number"))?,
                ),
            };
            ctx.slots.push(Slot::Num(value));
            let min = limit_key(field, "min", at)?;
            let max = limit_key(field, "max", at)?;
            let mut f = target.input(&label, move |r| num_of(r, index));
            if required {
                f = f.required();
            }
            f.validate(move |v: &Option<f64>| range_check(*v, min, max, &plain_label));
        }
        "boolean" => {
            let value = match default {
                None => false,
                Some(Value::Bool(b)) => *b,
                Some(_) => return Err(format!("{at}.default: must be true or false")),
            };
            ctx.slots.push(Slot::Bool(value));
            let f = target.check(&label, move |r| bool_of(r, index));
            if required {
                f.required();
            }
        }
        "choice" => {
            let options = options_key(field, at)?;
            let value = match default {
                None => Value::Null,
                Some(v) if options.iter().any(|(_, o)| o == v) => v.clone(),
                Some(_) => {
                    return Err(format!("{at}.default: is not one of the options' values"));
                }
            };
            ctx.slots.push(Slot::Choice(value));
            if widget == Some("radio") {
                add_radio(target, &label, options, index, at)?;
            } else {
                target.choice(&label, options, move |r| choice_of(r, index));
            }
        }
        "multichoice" => {
            let options = options_key(field, at)?;
            let chosen = match default {
                None => Vec::new(),
                Some(Value::Array(list)) => {
                    for v in list {
                        if !options.iter().any(|(_, o)| o == v) {
                            return Err(format!(
                                "{at}.default: {v} is not one of the options' values"
                            ));
                        }
                    }
                    list.clone()
                }
                Some(_) => return Err(format!("{at}.default: must be a list of values")),
            };
            ctx.slots.push(Slot::Multi(chosen));
            add_multi(target, &label, options, index, required, at)?;
        }
        #[cfg(feature = "chrono")]
        "date" => {
            let value = match default {
                None | Some(Value::Null) => None,
                Some(Value::String(s)) => Some(
                    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                        .map_err(|_| format!("{at}.default: must be YYYY-MM-DD"))?,
                ),
                Some(_) => return Err(format!("{at}.default: must be YYYY-MM-DD")),
            };
            ctx.slots.push(Slot::Date(value));
            let f = target.input(&label, move |r| date_of(r, index));
            if required {
                f.required();
            }
        }
        other => {
            let hint = if other == "date" {
                "dates need the `chrono` feature".to_string()
            } else {
                format!("expected one of {TYPES}")
            };
            return Err(format!("{at}.type: unknown type \"{other}\"; {hint}"));
        }
    }
    ctx.answers.push((name.to_string(), index));
    Ok(())
}

/// The whole-number `min` and `max` a spinner or slider needs.
fn whole_range(field: &Map<String, Value>, at: &str, widget: &str) -> Result<(i64, i64), String> {
    let get = |key: &str| {
        field
            .get(key)
            .and_then(Value::as_i64)
            .ok_or(format!("{at}.{key}: a {widget} needs a whole number {key}"))
    };
    let (min, max) = (get("min")?, get("max")?);
    if min > max {
        return Err(format!("{at}.min: is greater than max"));
    }
    Ok((min, max))
}

/// A choice shown as radio buttons, one per option, bound to the same slot a
/// drop-down would use.
fn add_radio(
    target: &mut Target<'_, '_>,
    label: &str,
    options: Vec<(String, Value)>,
    index: usize,
    at: &str,
) -> Result<(), String> {
    if options.len() > MAX_CLUSTER_ITEMS {
        return Err(format!(
            "{at}.options: radio buttons take at most {MAX_CLUSTER_ITEMS} options; use the dropdown widget"
        ));
    }
    let captions: Vec<String> = options.iter().map(|(caption, _)| caption.clone()).collect();
    let values: Vec<Value> = options.into_iter().map(|(_, value)| value).collect();
    let rows = i16::try_from(captions.len()).unwrap_or(i16::MAX);
    let for_write = values.clone();
    target.bind(
        label,
        RadioButtons::new(size(0, rows), captions),
        move |r| choice_of(r, index),
        // Nothing selected is the bitmask 0, which `RadioButtons::selected`
        // does not report as none.
        move |radio: &RadioButtons| match radio.value() {
            0 => Err("must be chosen".to_string()),
            bits => Ok(values[bits.trailing_zeros() as usize].clone()),
        },
        move |radio: &mut RadioButtons, value: &Value| {
            let bits = for_write
                .iter()
                .position(|v| v == value)
                .map_or(0, |i| 1_u32 << i);
            radio.set_value(bits);
        },
    );
    Ok(())
}

/// A text that must fit a picture mask (`#` digit, `@` letter, `&` letter
/// forced to upper case, `!` any character; anything else is typed for the
/// person). Empty is allowed unless the field is required.
fn add_masked(
    target: &mut Target<'_, '_>,
    label: &str,
    mask: &str,
    index: usize,
    required: bool,
    at: &str,
) -> Result<(), String> {
    if mask.is_empty() {
        return Err(format!("{at}.mask: must not be empty"));
    }
    let width =
        i16::try_from(mask.chars().count() + 1).map_err(|_| format!("{at}.mask: too long"))?;
    let validator = Rc::new(RefCell::new(PictureValidator::new(mask)));
    let view = InputLine::with_validator(size(width, 1), mask.chars().count(), validator);
    let shown = mask.to_string();
    let name = plain(label);
    target.bind(
        label,
        view,
        move |r| text_of(r, index),
        move |line: &InputLine| {
            let text = line.text();
            if text.trim().is_empty() {
                if required {
                    Err(format!("{name} is required"))
                } else {
                    Ok(String::new())
                }
            } else if line.is_valid() {
                Ok(text.to_string())
            } else {
                Err(format!("must look like {shown}"))
            }
        },
        |line: &mut InputLine, text: &String| line.set_text(text),
    );
    Ok(())
}

/// A multiple choice: one check box per option, answering the list of the
/// ticked options' values in the options' order.
fn add_multi(
    target: &mut Target<'_, '_>,
    label: &str,
    options: Vec<(String, Value)>,
    index: usize,
    required: bool,
    at: &str,
) -> Result<(), String> {
    if options.len() > MAX_CLUSTER_ITEMS {
        return Err(format!(
            "{at}.options: a multichoice takes at most {MAX_CLUSTER_ITEMS} options"
        ));
    }
    let captions: Vec<String> = options.iter().map(|(caption, _)| caption.clone()).collect();
    let values: Vec<Value> = options.into_iter().map(|(_, value)| value).collect();
    let rows = i16::try_from(captions.len()).unwrap_or(i16::MAX);
    let for_write = values.clone();
    target.bind(
        label,
        CheckBoxes::new(size(0, rows), captions),
        move |r| multi_of(r, index),
        move |boxes: &CheckBoxes| {
            let ticked: Vec<Value> = values
                .iter()
                .enumerate()
                .filter(|(i, _)| boxes.is_checked(*i))
                .map(|(_, v)| v.clone())
                .collect();
            if required && ticked.is_empty() {
                Err("at least one must be chosen".to_string())
            } else {
                Ok(ticked)
            }
        },
        move |boxes: &mut CheckBoxes, chosen: &Vec<Value>| {
            for (i, v) in for_write.iter().enumerate() {
                boxes.set_checked(i, chosen.contains(v));
            }
        },
    );
    Ok(())
}

fn range_check(
    value: Option<f64>,
    min: Option<f64>,
    max: Option<f64>,
    name: &str,
) -> Result<(), String> {
    let Some(value) = value else { return Ok(()) };
    match (min, max) {
        (Some(min), _) if value < min => Err(format!("{name} must be at least {min}")),
        (_, Some(max)) if value > max => Err(format!("{name} must be at most {max}")),
        _ => Ok(()),
    }
}

fn reject_unknown(object: &Map<String, Value>, allowed: &[&str], at: &str) -> Result<(), String> {
    match object.keys().find(|key| !allowed.contains(&key.as_str())) {
        Some(key) => Err(format!(
            "{at}{key}: unknown key; expected one of {}",
            allowed.join(", ")
        )),
        None => Ok(()),
    }
}

fn str_key<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    at: &str,
) -> Result<Option<&'a str>, String> {
    match object.get(key) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        Some(_) => Err(format!("{at}.{key}: must be a string")),
    }
}

fn bool_key(object: &Map<String, Value>, key: &str, at: &str) -> Result<Option<bool>, String> {
    match object.get(key) {
        None => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(format!("{at}.{key}: must be true or false")),
    }
}

fn uint_key(object: &Map<String, Value>, key: &str, at: &str) -> Result<Option<usize>, String> {
    match object.get(key) {
        None => Ok(None),
        Some(v) => v
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .filter(|n| *n > 0)
            .map(Some)
            .ok_or(format!("{at}.{key}: must be a positive whole number")),
    }
}

fn limit_key(object: &Map<String, Value>, key: &str, at: &str) -> Result<Option<f64>, String> {
    match object.get(key) {
        None => Ok(None),
        Some(v) => v
            .as_f64()
            .map(Some)
            .ok_or(format!("{at}.{key}: must be a number")),
    }
}

/// A `choice`'s options as captions and values.
fn options_key(object: &Map<String, Value>, at: &str) -> Result<Vec<(String, Value)>, String> {
    let list = object
        .get("options")
        .and_then(Value::as_array)
        .filter(|list| !list.is_empty())
        .ok_or(format!("{at}.options: a choice needs a list of options"))?;
    let mut options = Vec::new();
    for (i, option) in list.iter().enumerate() {
        let (caption, value) = match option {
            Value::String(s) => (s.clone(), option.clone()),
            Value::Number(_) | Value::Bool(_) => (scalar_text(option), option.clone()),
            Value::Object(o) => {
                let extra = o.keys().find(|k| *k != "label" && *k != "value");
                if let Some(extra) = extra {
                    return Err(format!(
                        "{at}.options[{i}].{extra}: unknown key; expected label, value"
                    ));
                }
                let value = o.get("value").cloned();
                let caption = match o.get("label") {
                    Some(Value::String(s)) => s.clone(),
                    Some(_) => return Err(format!("{at}.options[{i}].label: must be a string")),
                    None => value
                        .as_ref()
                        .map(scalar_text)
                        .ok_or(format!("{at}.options[{i}]: needs a label or a value"))?,
                };
                let value = value.unwrap_or_else(|| Value::String(caption.clone()));
                if value.is_null() || value.is_array() || value.is_object() {
                    return Err(format!(
                        "{at}.options[{i}].value: must be a string, number or boolean"
                    ));
                }
                (caption, value)
            }
            _ => {
                return Err(format!(
                    "{at}.options[{i}]: must be a string, number, boolean or object"
                ));
            }
        };
        if options.iter().any(|(_, v)| *v == value) {
            return Err(format!(
                "{at}.options[{i}]: the value {value} is used twice"
            ));
        }
        options.push((caption, value));
    }
    Ok(options)
}

fn scalar_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// A label without the `~` hot key marks.
fn plain(label: &str) -> String {
    label.replace('~', "")
}

/// Picks an Alt hot key for each label that has none, so that the fields can
/// be reached from the keyboard and no two share a letter.
struct HotKeys {
    /// The letters taken: by explicit `~x~` marks, and `O` by the OK button.
    taken: std::cell::RefCell<HashSet<char>>,
}

impl HotKeys {
    fn new(fields: &[Value]) -> Self {
        fn collect(fields: &[Value], taken: &mut HashSet<char>) {
            for field in fields {
                let kind = field.get("type").and_then(Value::as_str);
                if matches!(kind, Some("group" | "row")) {
                    if let Some(inner) = field.get("fields").and_then(Value::as_array) {
                        collect(inner, taken);
                    }
                } else if !matches!(kind, Some("section" | "note"))
                    && let Some(label) = field.get("label").and_then(Value::as_str)
                    && let Some(c) = marked(label)
                {
                    taken.insert(c);
                }
            }
        }
        let mut taken = HashSet::from(['O']);
        collect(fields, &mut taken);
        Self {
            taken: std::cell::RefCell::new(taken),
        }
    }

    /// The field's label with a hot key: its own, or one picked from its
    /// text. Defaults to the name with the first letter capitalised.
    fn label(&self, field: &Map<String, Value>, name: &str, at: &str) -> Result<String, String> {
        let text = match str_key(field, "label", at)? {
            Some(label) if marked(label).is_some() => return Ok(label.to_string()),
            Some(label) => label.to_string(),
            None => capitalised(name),
        };
        let mut taken = self.taken.borrow_mut();
        for (at, c) in text.char_indices() {
            let upper = c.to_ascii_uppercase();
            if c.is_ascii_alphabetic() && taken.insert(upper) {
                let end = at + c.len_utf8();
                return Ok(format!("{}~{c}~{}", &text[..at], &text[end..]));
            }
        }
        Ok(text)
    }
}

/// The upper-case letter a label marks with `~x~`, if it does.
fn marked(label: &str) -> Option<char> {
    let start = label.find('~')? + 1;
    let rest = &label[start..];
    let mut chars = rest.chars();
    let c = chars.next()?;
    (chars.next() == Some('~')).then(|| c.to_ascii_uppercase())
}

fn capitalised(name: &str) -> String {
    let name = name.replace(['_', '-'], " ");
    let mut chars = name.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use turbo_vision::core::event::{Event, KB_ALT_O, KB_ESC};
    use turbo_vision::test_util::test_terminal;

    fn form(spec: &Value) -> JsonForm {
        JsonForm::from_spec(spec).expect("a valid spec")
    }

    fn why(spec: &Value) -> String {
        match JsonForm::from_spec(spec) {
            Err(FormError::Spec(why)) => why,
            Err(other) => panic!("unexpected {other:?}"),
            Ok(_) => panic!("the spec was accepted"),
        }
    }

    fn app() -> Application {
        Application::with_terminal(test_terminal(100, 40))
    }

    fn customer() -> Value {
        json!({
            "title": "New customer",
            "fields": [
                { "name": "name", "type": "text", "label": "Name", "required": true, "max_len": 20 },
                { "name": "age", "type": "integer", "min": 0, "max": 120 },
                { "name": "ratio", "type": "number", "default": 1.5 },
                { "type": "section", "label": "Account" },
                { "name": "kind", "type": "choice", "options": ["Retail", { "label": "Wholesale", "value": 2 }] },
                { "name": "vip", "type": "boolean", "label": "VIP customer" },
                { "name": "notes", "type": "memo", "rows": 2 }
            ]
        })
    }

    #[test]
    fn defaults_come_back_as_json_in_the_specs_order() {
        let mut spec = customer();
        spec["fields"][0]["default"] = json!("Ada");
        spec["fields"][4]["default"] = json!(2);
        let mut form = form(&spec);
        let answers = form.read().expect("valid");
        assert_eq!(
            answers,
            json!({ "name": "Ada", "age": null, "ratio": 1.5, "kind": 2, "vip": false, "notes": "" })
        );
        let keys: Vec<_> = answers.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["name", "age", "ratio", "kind", "vip", "notes"]);
    }

    #[test]
    fn required_fields_and_an_unchosen_choice_are_refused() {
        let mut f = form(&customer());
        let errors = f.read().expect_err("name and kind are missing");
        assert_eq!(errors.len(), 2, "{errors}");
    }

    #[test]
    fn min_and_max_reject_out_of_range_numbers() {
        let mut spec = customer();
        spec["fields"][0]["default"] = json!("Ada");
        spec["fields"][4]["default"] = json!("Retail");
        spec["fields"][1]["default"] = json!(121);
        let errors = form(&spec).read().expect_err("age is too large");
        assert!(
            errors.to_string().contains("Age must be at most 120"),
            "{errors}"
        );
    }

    #[test]
    fn the_person_fills_it_in_and_the_answers_come_back() {
        let mut app = app();
        let tx = app.terminal.event_injector();
        for c in "Bo".chars() {
            tx.send(Event::text(c)).unwrap(); // the first field has the focus
        }
        tx.send(Event::keyboard(KB_ALT_O)).unwrap();
        let spec = json!({ "fields": [
            { "name": "name", "type": "text", "required": true },
            { "name": "vip", "type": "boolean", "default": true }
        ]});
        assert_eq!(
            run(&mut app, &spec),
            Ok(json!({ "name": "Bo", "vip": true }))
        );
    }

    #[test]
    fn an_invalid_form_stays_open_until_it_is_fixed() {
        let mut app = app();
        let tx = app.terminal.event_injector();
        tx.send(Event::keyboard(KB_ALT_O)).unwrap(); // refused: name is empty
        tx.send(Event::text('x')).unwrap();
        tx.send(Event::keyboard(KB_ALT_O)).unwrap();
        let spec = json!({ "fields": [{ "name": "name", "type": "text", "required": true }] });
        assert_eq!(run(&mut app, &spec), Ok(json!({ "name": "x" })));
    }

    #[test]
    fn escape_cancels() {
        let mut app = app();
        app.terminal
            .event_injector()
            .send(Event::keyboard(KB_ESC))
            .unwrap();
        let spec = json!({ "fields": [{ "name": "a", "type": "text" }] });
        assert_eq!(run(&mut app, &spec), Err(FormError::Cancelled));
    }

    #[test]
    fn a_form_can_run_again_from_its_defaults() {
        let mut app = app();
        let mut f = form(&json!({ "fields": [{ "name": "a", "type": "text", "default": "d" }] }));
        let tx = app.terminal.event_injector();
        tx.send(Event::text('1')).unwrap();
        tx.send(Event::keyboard(KB_ALT_O)).unwrap();
        tx.send(Event::keyboard(KB_ALT_O)).unwrap();
        assert_eq!(f.run(&mut app), Ok(json!({ "a": "d1" })));
        assert_eq!(
            f.run(&mut app),
            Ok(json!({ "a": "d" })),
            "the second run starts over"
        );
    }

    #[test]
    fn hot_keys_are_picked_and_never_repeat() {
        let spec = json!({ "fields": [
            { "name": "name", "type": "text" },
            { "name": "notes", "type": "text", "label": "Notes" },
            { "name": "ok", "type": "text", "label": "~K~ind" },
            { "name": "x", "type": "text", "label": "Zip" }
        ]});
        let hot = HotKeys::new(spec["fields"].as_array().unwrap());
        let label = |i: usize| {
            let field = spec["fields"][i].as_object().unwrap();
            hot.label(field, field["name"].as_str().unwrap(), "f")
                .unwrap()
        };
        assert_eq!(label(0), "~N~ame");
        assert_eq!(
            label(1),
            "No~t~es",
            "N is taken and O belongs to the OK button, so T"
        );
        assert_eq!(label(2), "~K~ind", "an explicit mark is kept");
        assert_eq!(label(3), "~Z~ip");
    }

    fn radio(extra: Value) -> Value {
        let mut field = json!({
            "name": "env", "type": "choice", "widget": "radio",
            "options": ["staging", { "label": "Production", "value": 2 }]
        });
        field
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        json!({ "fields": [field] })
    }

    #[test]
    fn radio_buttons_hold_the_default_and_answer_with_the_options_value() {
        assert_eq!(
            form(&radio(json!({ "default": 2 }))).read().unwrap(),
            json!({ "env": 2 })
        );
        assert_eq!(
            form(&radio(json!({ "default": "staging" })))
                .read()
                .unwrap(),
            json!({ "env": "staging" })
        );
    }

    #[test]
    fn radio_buttons_with_nothing_chosen_are_refused() {
        let errors = form(&radio(json!({}))).read().expect_err("nothing chosen");
        assert!(errors.to_string().contains("must be chosen"), "{errors}");
    }

    #[test]
    fn the_widget_is_checked() {
        assert_eq!(
            why(&radio(json!({ "widget": "slider" }))),
            "fields[0].widget: unknown widget \"slider\"; expected dropdown, radio"
        );
        assert_eq!(
            why(&json!({ "fields": [{ "name": "a", "type": "text", "widget": "radio" }] })),
            "fields[0].widget: a text has no choice of widget"
        );
        let many: Vec<String> = (0..33).map(|i| format!("o{i}")).collect();
        assert!(why(&radio(json!({ "options": many }))).contains("at most 32 options"),);
        // The dropdown takes as many as you like.
        let many: Vec<String> = (0..40).map(|i| format!("o{i}")).collect();
        assert!(
            JsonForm::from_spec(&radio(json!({ "widget": "dropdown", "options": many }))).is_ok()
        );
    }

    #[test]
    fn a_bad_spec_says_where_and_what() {
        assert_eq!(why(&json!([])), "the form must be a JSON object");
        assert_eq!(why(&json!({})), "fields: a list of fields is required");
        assert_eq!(
            why(&json!({ "fields": [] })),
            "fields: the form has no fields"
        );
        assert_eq!(
            why(&json!({ "titel": "x", "fields": [] })),
            "titel: unknown key; expected one of title, fields, labels, label_align"
        );
        assert_eq!(
            why(&json!({ "fields": [{ "name": "a", "type": "txt" }] })),
            format!("fields[0].type: unknown type \"txt\"; expected one of {TYPES}")
        );
        assert_eq!(
            why(&json!({ "fields": [{ "type": "text" }] })),
            "fields[0].name: required"
        );
        assert_eq!(
            why(
                &json!({ "fields": [{ "name": "a", "type": "text" }, { "name": "a", "type": "text" }] })
            ),
            "fields[1].name: \"a\" is used twice"
        );
        assert_eq!(
            why(&json!({ "fields": [{ "name": "a", "type": "choice" }] })),
            "fields[0].options: a choice needs a list of options"
        );
        assert_eq!(
            why(&json!({ "fields": [{ "name": "a", "type": "integer", "default": "x" }] })),
            "fields[0].default: must be a whole number"
        );
        assert_eq!(
            why(&json!({ "fields": [{ "name": "a", "type": "text", "requird": true }] })),
            format!(
                "fields[0].requird: unknown key; expected one of {}",
                FIELD_KEYS.join(", ")
            )
        );
        assert_eq!(
            why(&json!({ "fields": [{ "type": "section", "label": "Only a heading" }] })),
            "fields: the form has no field to answer"
        );
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn dates_are_text_in_and_out() {
        let spec = json!({ "fields": [
            { "name": "since", "type": "date", "default": "1843-07-01" },
            { "name": "until", "type": "date" }
        ]});
        assert_eq!(
            form(&spec).read().unwrap(),
            json!({ "since": "1843-07-01", "until": null })
        );
        assert_eq!(
            why(&json!({ "fields": [{ "name": "d", "type": "date", "default": "1 July" }] })),
            "fields[0].default: must be YYYY-MM-DD"
        );
    }

    // ---- more widgets and layout ----

    fn fields(list: Value) -> Value {
        json!({ "fields": list })
    }

    #[test]
    fn spinners_and_sliders_start_on_their_default_or_min() {
        let spec = fields(json!([
            { "name": "a", "type": "integer", "widget": "spinner", "min": 1, "max": 9 },
            { "name": "b", "type": "integer", "widget": "slider", "min": 0, "max": 100, "default": 40 },
            { "name": "c", "type": "integer", "widget": "input" }
        ]));
        assert_eq!(
            form(&spec).read().unwrap(),
            json!({ "a": 1, "b": 40, "c": null })
        );
    }

    #[test]
    fn spinners_and_sliders_need_a_range_and_a_default_inside_it() {
        let bad = |extra: Value| {
            let mut field = json!({ "name": "a", "type": "integer", "widget": "spinner" });
            field
                .as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            why(&fields(json!([field])))
        };
        assert_eq!(
            bad(json!({})),
            "fields[0].min: a spinner needs a whole number min"
        );
        assert_eq!(
            bad(json!({ "min": 5, "max": 1 })),
            "fields[0].min: is greater than max"
        );
        assert_eq!(
            bad(json!({ "min": 1, "max": 5, "default": 9 })),
            "fields[0].default: must be a whole number from 1 to 5"
        );
        assert_eq!(
            why(&fields(
                json!([{ "name": "a", "type": "integer", "widget": "dial" }])
            )),
            "fields[0].widget: unknown widget \"dial\"; expected input, spinner, slider"
        );
    }

    #[test]
    fn a_multichoice_answers_a_list_in_the_options_order() {
        let spec = fields(json!([
            { "name": "tags", "type": "multichoice", "options": ["a", "b", { "label": "C", "value": 3 }],
              "default": [3, "a"] }
        ]));
        assert_eq!(form(&spec).read().unwrap(), json!({ "tags": ["a", 3] }));
        let none = fields(json!([{ "name": "t", "type": "multichoice", "options": ["a"] }]));
        assert_eq!(form(&none).read().unwrap(), json!({ "t": [] }));
    }

    #[test]
    fn a_required_multichoice_needs_one_ticked() {
        let spec = fields(json!([
            { "name": "t", "type": "multichoice", "options": ["a", "b"], "required": true }
        ]));
        let errors = form(&spec).read().expect_err("none ticked");
        assert!(errors.to_string().contains("at least one"), "{errors}");
        let spec = fields(json!([
            { "name": "t", "type": "multichoice", "options": ["a", "b"], "required": true, "default": ["b"] }
        ]));
        assert_eq!(form(&spec).read().unwrap(), json!({ "t": ["b"] }));
        let bad = fields(json!([
            { "name": "t", "type": "multichoice", "options": ["a"], "default": ["z"] }
        ]));
        assert!(why(&bad).starts_with("fields[0].default: \"z\" is not one of"));
    }

    #[test]
    fn groups_and_rows_nest_and_answers_stay_flat() {
        let spec = fields(json!([
            { "type": "note", "text": "Fill in\nboth parts" },
            { "type": "group", "label": "Person", "fields": [
                { "name": "first", "type": "text", "default": "Ada" },
                { "type": "row", "fields": [
                    { "name": "age", "type": "integer", "default": 36 },
                    { "name": "vip", "type": "boolean" }
                ] },
                { "type": "group", "label": "Inner", "fields": [
                    { "name": "city", "type": "text", "default": "Palma" }
                ] }
            ] },
            { "name": "notes", "type": "memo", "rows": 2 }
        ]));
        assert_eq!(
            form(&spec).read().unwrap(),
            json!({ "first": "Ada", "age": 36, "vip": false, "city": "Palma", "notes": "" })
        );
    }

    #[test]
    fn names_stay_unique_across_groups_and_hot_keys_across_rows() {
        let spec = fields(json!([
            { "name": "a", "type": "text" },
            { "type": "group", "fields": [{ "name": "a", "type": "text" }] }
        ]));
        assert_eq!(why(&spec), "fields[1].fields[0].name: \"a\" is used twice");
    }

    #[test]
    fn layout_items_are_checked() {
        assert_eq!(
            why(&fields(json!([{ "type": "group" }]))),
            "fields[0].fields: a list of fields is required"
        );
        assert_eq!(
            why(&fields(json!([{ "type": "row", "fields": [] }]))),
            "fields[0].fields: a list of fields is required"
        );
        assert_eq!(
            why(&fields(
                json!([{ "type": "row", "fields": [{ "name": "m", "type": "memo" }] }])
            )),
            "fields[0].fields[0].type: a memo cannot be in a row; put it outside"
        );
        assert_eq!(
            why(&fields(
                json!([{ "type": "row", "fields": [{ "type": "row", "fields": [] }] }])
            )),
            "fields[0].fields[0].type: a row cannot be in a row; put it outside"
        );
        assert_eq!(
            why(&fields(json!([{ "type": "note" }]))),
            "fields[0].text: a note needs its text"
        );
        assert!(
            why(&fields(
                json!([{ "type": "note", "text": "x", "name": "n" }])
            ))
            .starts_with("fields[0].name: unknown key; expected one of type, text")
        );
        assert_eq!(
            why(&fields(
                json!([{ "name": "a", "type": "text", "fields": [] }])
            )),
            "fields[0].fields: a text has no fields"
        );
        assert!(
            why(&fields(json!([{ "name": "a", "type": "text" }]))
                .as_object()
                .map(|o| {
                    let mut o = o.clone();
                    o.insert("labels".into(), json!("beside"));
                    Value::Object(o)
                })
                .unwrap())
            .starts_with("labels: unknown placement \"beside\"")
        );
    }

    #[test]
    fn labels_above_and_right_aligned_build() {
        for (labels, align) in [("above", "left"), ("left", "right")] {
            let mut spec = fields(json!([{ "name": "a", "type": "text", "default": "x" }]));
            spec["labels"] = json!(labels);
            spec["label_align"] = json!(align);
            assert_eq!(form(&spec).read().unwrap(), json!({ "a": "x" }));
        }
    }

    #[test]
    fn hot_keys_in_rows_and_groups_do_not_repeat() {
        let spec = fields(json!([
            { "name": "street", "type": "text" },
            { "type": "row", "fields": [
                { "name": "stop", "type": "text" },
                { "name": "sweet", "type": "text", "label": "~S~weet" }
            ] }
        ]));
        assert_eq!(form(&spec).read().unwrap().as_object().unwrap().len(), 3);
    }

    // ---- masks ----

    fn masked(extra: Value) -> Value {
        let mut field = json!({ "name": "phone", "type": "text", "mask": "(###) ###-####" });
        field
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        fields(json!([field]))
    }

    #[test]
    fn a_masked_text_accepts_what_fits_and_refuses_the_rest() {
        let ok = masked(json!({ "default": "(555) 123-4567" }));
        assert_eq!(
            form(&ok).read().unwrap(),
            json!({ "phone": "(555) 123-4567" })
        );
        let bad = masked(json!({ "default": "555" }));
        let errors = form(&bad).read().expect_err("does not fit the mask");
        assert!(
            errors.to_string().contains("must look like (###) ###-####"),
            "{errors}"
        );
    }

    #[test]
    fn an_empty_masked_text_is_fine_unless_required() {
        assert_eq!(
            form(&masked(json!({}))).read().unwrap(),
            json!({ "phone": "" })
        );
        let errors = form(&masked(json!({ "required": true })))
            .read()
            .expect_err("required");
        assert!(errors.to_string().contains("is required"), "{errors}");
    }

    #[test]
    fn a_mask_is_checked_where_it_is_written() {
        assert_eq!(
            why(&masked(json!({ "mask": "" }))),
            "fields[0].mask: must not be empty"
        );
        assert_eq!(
            why(&masked(json!({ "max_len": 5 }))),
            "fields[0].max_len: a masked text is as long as its mask"
        );
        assert_eq!(
            why(&fields(
                json!([{ "name": "n", "type": "integer", "mask": "##" }])
            )),
            "fields[0].mask: only a text can have a mask"
        );
    }
}
