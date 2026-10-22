// (C) 2026 - Antoni Aloy

//! The runtime: a [`DialogSpec`] built into a live dialog, its values as
//! JSON, validation shown in the dialog, and actions bound in code.

use super::spec::{Align, Control, DialogSpec, Kind, SpecError, ValidatorSpec};
use crate::autocomplete::AutoCompleteBuilder;
use crate::dual_list::{DualList, DualListBuilder};
use crate::forms::data::{ErrorLine, next_combo_id};
use crate::forms::{find_view_mut, focus_view, focused_view};
use crate::{AutoComplete, dual_list};
use serde_json::{Map, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::rc::Rc;
use turbo_vision::app::{Application, ModalTick};
use turbo_vision::core::command::{CM_CANCEL, CM_NO, CM_OK, CM_YES, CommandId};
use turbo_vision::core::event::{Event, EventType};
use turbo_vision::core::geometry::{Point, Rect};
use turbo_vision::core::state::Options;
use turbo_vision::terminal::Terminal;
use turbo_vision::views::button::Button;
use turbo_vision::views::checkbox::CheckBox;
use turbo_vision::views::cluster_group::{CheckBoxes, RadioButtons};
use turbo_vision::views::combo_box::ComboBox;
use turbo_vision::views::dialog::Dialog;
use turbo_vision::views::group::{Group, GroupLike};
use turbo_vision::views::group_box::GroupBox;
use turbo_vision::views::history::History;
use turbo_vision::views::input_line::InputLine;
use turbo_vision::views::label::Label;
use turbo_vision::views::listbox::ListBox;
use turbo_vision::views::lookup_validator::LookupValidator;
use turbo_vision::views::memo::Memo;
use turbo_vision::views::msgbox::message_box_ok;
use turbo_vision::views::picture_validator::PictureValidator;
use turbo_vision::views::progress_bar::ProgressBar;
use turbo_vision::views::slider::Slider;
use turbo_vision::views::spinner::Spinner;
use turbo_vision::views::static_text::StaticText;
use turbo_vision::views::tabbed_pane::TabbedPane;
use turbo_vision::views::table::{Column, Table};
use turbo_vision::views::tooltip::Tooltip;
use turbo_vision::views::validator::{FilterValidator, RangeValidator, ValidatorRef};
use turbo_vision::views::view::{View, ViewCore, ViewId};

/// The first command number given to actions. Far above `CM_USER`, so it
/// stays clear of the commands an application numbers itself.
const ACTION_BASE: CommandId = 0xF000;

/// What an action wants the dialog to do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    /// Stay open: the action did its work (filled a field, showed a message).
    Stay,
    /// Close now, without validating, and return the action's name.
    Close,
    /// Validate, as OK does, and close if everything passes.
    Submit,
}

/// How a dialog was closed, when it was not cancelled.
#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    /// `"ok"`, `"yes"`, `"no"`, or the name of the action that closed it.
    pub action: String,
    /// Every named control's value, keyed by name, in the spec's order.
    pub data: Value,
}

/// What validation found wrong: messages against controls, by name, or
/// against the dialog as a whole.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Errors {
    list: Vec<(Option<String>, String)>,
}

impl Errors {
    /// No errors yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Report `message` against the control named `field`.
    pub fn add(&mut self, field: &str, message: impl Into<String>) {
        self.list.push((Some(field.to_string()), message.into()));
    }

    /// Report `message` against the dialog as a whole.
    pub fn add_form(&mut self, message: impl Into<String>) {
        self.list.push((None, message.into()));
    }

    /// Whether nothing was reported.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// How many messages were reported.
    #[must_use]
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Every message, with the name of its control (`None` for the dialog).
    pub fn iter(&self) -> impl Iterator<Item = (Option<&str>, &str)> {
        self.list
            .iter()
            .map(|(field, message)| (field.as_deref(), message.as_str()))
    }

    /// The first message reported against `field`.
    #[must_use]
    pub fn field(&self, field: &str) -> Option<&str> {
        self.iter()
            .find(|(f, _)| *f == Some(field))
            .map(|(_, message)| message)
    }
}

impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let messages: Vec<&str> = self.iter().map(|(_, m)| m).collect();
        f.write_str(&messages.join("; "))
    }
}

impl std::error::Error for Errors {}

type Handler = Box<dyn FnMut(&mut ActionContext<'_>) -> Flow>;
type FieldRule = Box<dyn Fn(&Value) -> Result<(), String>>;
type DataRule = Box<dyn Fn(&Map<String, Value>, &mut Errors)>;

/// A named control with a value.
struct Field {
    name: String,
    id: ViewId,
    kind: Kind,
    /// The label linked to it, which turns red while it is invalid.
    label: Option<ViewId>,
    /// How messages call it: its label's text, or its name.
    title: String,
}

/// A dialog built from a [`DialogSpec`]: run it, fill it and read it as
/// JSON, and bind validation rules and actions to it by name.
///
/// See the [module documentation](super) for a walk-through.
pub struct JsonDialog {
    spec: DialogSpec,
    dialog: Dialog,
    fields: Vec<Field>,
    /// Every action the spec names; action `i` sends `ACTION_BASE + i`.
    actions: Vec<String>,
    handlers: HashMap<String, Handler>,
    rules: Vec<(String, FieldRule)>,
    data_rules: Vec<DataRule>,
    /// Inputs with a custom validator: its name, the input, and whether
    /// code has bound it.
    custom: Vec<(String, ViewId, bool)>,
    /// Change actions heard since the last turn of the modal loop.
    changes: Rc<RefCell<Vec<CommandId>>>,
    error_line: Option<ViewId>,
}

impl fmt::Debug for JsonDialog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JsonDialog")
            .field("title", &self.spec.title)
            .field(
                "fields",
                &self.fields.iter().map(|f| &f.name).collect::<Vec<_>>(),
            )
            .finish_non_exhaustive()
    }
}

impl JsonDialog {
    /// Build the dialog `spec` describes.
    ///
    /// # Errors
    ///
    /// The spec breaks a rule of [`DialogSpec::check`].
    pub fn new(spec: &DialogSpec) -> Result<Self, SpecError> {
        spec.check()?;
        Ok(Builder::new(spec).build())
    }

    /// Read a spec from JSON and build its dialog.
    ///
    /// # Errors
    ///
    /// As [`DialogSpec::from_json`].
    pub fn from_json(text: &str) -> Result<Self, SpecError> {
        Self::new(&DialogSpec::from_json(text)?)
    }

    /// Read the spec saved in the file at `path` and build its dialog.
    ///
    /// # Errors
    ///
    /// As [`DialogSpec::load`].
    pub fn load(path: impl AsRef<Path>) -> Result<Self, SpecError> {
        Self::new(&DialogSpec::load(path)?)
    }

    /// The spec the dialog was built from.
    #[must_use]
    pub fn spec(&self) -> &DialogSpec {
        &self.spec
    }

    /// The turbo-vision dialog.
    #[must_use]
    pub fn dialog(&self) -> &Dialog {
        &self.dialog
    }

    /// The turbo-vision dialog, mutably: add views of your own, change
    /// its title.
    pub fn dialog_mut(&mut self) -> &mut Dialog {
        &mut self.dialog
    }

    /// The names of the controls that have a value, in the spec's order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.fields.iter().map(|f| f.name.as_str())
    }

    /// The control named `name` as its turbo-vision type, such as
    /// `view_mut::<InputLine>("email")`; `None` when there is no such
    /// control or it is not a `V`.
    pub fn view_mut<V: View + 'static>(&mut self, name: &str) -> Option<&mut V> {
        let id = self.id_of(name)?;
        find_view_mut(&mut self.dialog, id)?
            .as_any_mut()
            .downcast_mut()
    }

    /// The value of the control named `name`, as JSON (see the module
    /// documentation for each type's value).
    pub fn value(&mut self, name: &str) -> Option<Value> {
        let field = self.fields.iter().find(|f| f.name == name)?;
        Some(read(&mut self.dialog, field))
    }

    /// Show `value` in the control named `name`.
    ///
    /// # Errors
    ///
    /// No control has that name, or the value is not of its type.
    pub fn set_value(&mut self, name: &str, value: &Value) -> Result<(), SpecError> {
        set_value(&mut self.dialog, &self.fields, name, value)
    }

    /// Every named control's value, as a JSON object in the spec's order.
    pub fn data(&mut self) -> Value {
        data(&mut self.dialog, &self.fields)
    }

    /// Show each value of the JSON object `data` in the control of that
    /// name. Controls not in `data` keep their values.
    ///
    /// # Errors
    ///
    /// `data` is not an object, names a control that does not exist, or
    /// holds a value of the wrong type. Values before the bad one are set.
    pub fn set_data(&mut self, data: &Value) -> Result<(), SpecError> {
        set_data(&mut self.dialog, &self.fields, data)
    }

    /// Bind `validator` to every input whose spec names it:
    /// `"validator": { "type": "custom", "name": "<name>" }`. It checks
    /// keystrokes as they are typed, like the built-in validators.
    #[allow(
        clippy::needless_pass_by_value,
        reason = "a validator is made for the call; one input or many share it"
    )]
    pub fn validator(&mut self, name: &str, validator: ValidatorRef) -> &mut Self {
        for (custom, id, bound) in &mut self.custom {
            if custom == name
                && let Some(input) = find_view_mut(&mut self.dialog, *id)
                    .and_then(|v| v.as_any_mut().downcast_mut::<InputLine>())
            {
                input.set_validator(Rc::clone(&validator));
                *bound = true;
            }
        }
        self
    }

    /// Add a rule for the control named `field`: given its value, return
    /// `Err(message)` when it is not acceptable. Rules run when the dialog
    /// is accepted, after the spec's own checks (`required`, validators)
    /// pass for that control.
    pub fn rule(
        &mut self,
        field: &str,
        rule: impl Fn(&Value) -> Result<(), String> + 'static,
    ) -> &mut Self {
        self.rules.push((field.to_string(), Box::new(rule)));
        self
    }

    /// Add a rule over all the data, for checks that involve several
    /// controls. It runs once every control is valid; report problems in
    /// the [`Errors`] it is given.
    pub fn validate_data(
        &mut self,
        rule: impl Fn(&Map<String, Value>, &mut Errors) + 'static,
    ) -> &mut Self {
        self.data_rules.push(Box::new(rule));
        self
    }

    /// Bind `handler` to the action `action`: a button whose `command` is
    /// that name, or a control whose `on_change` is. What it returns
    /// decides whether the dialog stays open ([`Flow`]).
    pub fn on(
        &mut self,
        action: &str,
        handler: impl FnMut(&mut ActionContext<'_>) -> Flow + 'static,
    ) -> &mut Self {
        self.handlers.insert(action.to_string(), Box::new(handler));
        self
    }

    /// What the spec names that no code has bound yet, such as
    /// `action "lookup"` or `validator "iban"`, sorted. Handy in a test,
    /// so a renamed button cannot go unnoticed.
    ///
    /// An action with no handler still works: its button validates and
    /// closes the dialog like OK, returning the action's name.
    #[must_use]
    pub fn unbound(&self) -> Vec<String> {
        let mut missing: Vec<String> = self
            .actions
            .iter()
            .filter(|a| !self.handlers.contains_key(*a))
            .map(|a| format!("action \"{a}\""))
            .chain(
                self.custom
                    .iter()
                    .filter(|(_, _, bound)| !bound)
                    .map(|(name, _, _)| format!("validator \"{name}\"")),
            )
            .collect();
        missing.sort();
        missing.dedup();
        missing
    }

    /// Check every control and rule, as accepting the dialog does, and
    /// return the data when all pass. Shows nothing; see
    /// [`show_errors`](Self::show_errors).
    ///
    /// # Errors
    ///
    /// Everything that failed.
    pub fn validate(&mut self) -> Result<Value, Errors> {
        validate(
            &mut self.dialog,
            &self.fields,
            &self.rules,
            &self.data_rules,
        )
    }

    /// Show `errors` in the dialog: the labels of the controls at fault
    /// turn red, the error line shows the message, and the focus moves to
    /// the first control at fault. An empty list clears what was shown.
    pub fn show_errors(&mut self, errors: &Errors) {
        focus_first(&mut self.dialog, &self.fields, errors);
        display(&mut self.dialog, &self.fields, self.error_line, errors);
    }

    /// The text on the error line now (empty with no error line).
    #[must_use]
    pub fn error_text(&self) -> String {
        self.error_line
            .and_then(|id| self.dialog.child_by_id(id))
            .and_then(|v| v.as_any().downcast_ref::<ErrorLine>())
            .map(|line| line.text.clone())
            .unwrap_or_default()
    }

    /// The label linked to the control named `name`.
    pub fn label_of(&mut self, name: &str) -> Option<&Label> {
        let id = self.fields.iter().find(|f| f.name == name)?.label?;
        find_view_mut(&mut self.dialog, id)?
            .as_any()
            .downcast_ref::<Label>()
    }

    /// Run the dialog until the person closes it. Returns `None` when they
    /// cancel (Cancel, Esc, the close box), otherwise how it was closed
    /// and the data.
    ///
    /// OK and Yes validate first: while anything fails, the dialog stays
    /// open showing why. A button bound to an action runs its handler,
    /// which decides ([`Flow`]); an unbound one validates and closes like
    /// OK. Change actions run as the person changes their control.
    #[allow(
        clippy::too_many_lines,
        reason = "the modal loop's one tick, every way of closing in one place"
    )]
    pub fn run(&mut self, app: &mut Application) -> Option<Answer> {
        // Values set before the run are shown with no stale errors.
        display(
            &mut self.dialog,
            &self.fields,
            self.error_line,
            &Errors::new(),
        );
        self.dialog.prepare_modal(app);
        self.changes.borrow_mut().clear();

        let Self {
            dialog,
            fields,
            actions,
            handlers,
            rules,
            data_rules,
            changes,
            error_line,
            ..
        } = self;
        let fields: &[Field] = fields;
        let (rules, data_rules) = (&*rules, &*data_rules);
        let error_line = *error_line;
        // Once errors have been shown, check again whenever the focus moves.
        let mut live = false;
        let mut focus = focused_view(dialog);

        let submit = |app: &mut Application, dialog: &mut Dialog, live: &mut bool| -> bool {
            match validate(dialog, fields, rules, data_rules) {
                Ok(_) => true,
                Err(errors) => {
                    focus_first(dialog, fields, &errors);
                    display(dialog, fields, error_line, &errors);
                    if error_line.is_none()
                        && let Some((_, message)) = errors.iter().next()
                    {
                        message_box_ok(app, message);
                    }
                    *live = true;
                    false
                }
            }
        };

        let command = app.execute_modal(dialog, |app, dialog| {
            let heard: Vec<CommandId> = changes.borrow_mut().drain(..).collect();
            for command in heard {
                let Some(action) = action_name(actions, command) else {
                    continue;
                };
                let Some(handler) = handlers.get_mut(action) else {
                    continue;
                };
                let flow = handler(&mut ActionContext {
                    app,
                    action,
                    dialog,
                    fields,
                });
                match flow {
                    Flow::Stay => {}
                    Flow::Close => return ModalTick::End(command),
                    Flow::Submit => {
                        if submit(app, dialog, &mut live) {
                            dialog.end_modal(command);
                        }
                    }
                }
            }

            let end = dialog.end_state();
            match end {
                0 => {
                    if live {
                        let now = focused_view(dialog);
                        if now != focus {
                            focus = now;
                            let errors = validate(dialog, fields, rules, data_rules)
                                .err()
                                .unwrap_or_default();
                            display(dialog, fields, error_line, &errors);
                        }
                    }
                }
                CM_CANCEL | CM_NO => {}
                CM_OK | CM_YES => {
                    if !submit(app, dialog, &mut live) {
                        dialog.end_modal(0);
                    }
                }
                command => {
                    let Some(action) = action_name(actions, command) else {
                        return ModalTick::Continue;
                    };
                    let flow = match handlers.get_mut(action) {
                        Some(handler) => handler(&mut ActionContext {
                            app,
                            action,
                            dialog,
                            fields,
                        }),
                        None => Flow::Submit,
                    };
                    match flow {
                        Flow::Stay => dialog.end_modal(0),
                        Flow::Close => return ModalTick::End(command),
                        Flow::Submit => {
                            if !submit(app, dialog, &mut live) {
                                dialog.end_modal(0);
                            }
                        }
                    }
                }
            }
            if live && dialog.end_state() == 0 {
                focus = focused_view(dialog);
            }
            ModalTick::Continue
        });

        if command == CM_CANCEL {
            return None;
        }
        let action = match command {
            CM_OK => "ok".to_string(),
            CM_YES => "yes".to_string(),
            CM_NO => "no".to_string(),
            other => {
                action_name(&self.actions, other).map_or_else(|| other.to_string(), str::to_string)
            }
        };
        Some(Answer {
            action,
            data: self.data(),
        })
    }

    fn id_of(&self, name: &str) -> Option<ViewId> {
        self.fields.iter().find(|f| f.name == name).map(|f| f.id)
    }
}

fn action_name(actions: &[String], command: CommandId) -> Option<&str> {
    let index = usize::from(command.checked_sub(ACTION_BASE)?);
    actions.get(index).map(String::as_str)
}

/// What an action handler can do while the dialog is open.
pub struct ActionContext<'a> {
    /// The application: show a message box, open another dialog.
    pub app: &'a mut Application,
    action: &'a str,
    dialog: &'a mut Dialog,
    fields: &'a [Field],
}

impl ActionContext<'_> {
    /// The name of the action being run.
    #[must_use]
    pub fn action(&self) -> &str {
        self.action
    }

    /// The value of the control named `name`.
    pub fn value(&mut self, name: &str) -> Option<Value> {
        let field = self.fields.iter().find(|f| f.name == name)?;
        Some(read(self.dialog, field))
    }

    /// Show `value` in the control named `name`.
    ///
    /// # Errors
    ///
    /// As [`JsonDialog::set_value`].
    pub fn set_value(&mut self, name: &str, value: &Value) -> Result<(), SpecError> {
        set_value(self.dialog, self.fields, name, value)
    }

    /// Every named control's value.
    pub fn data(&mut self) -> Value {
        data(self.dialog, self.fields)
    }

    /// Show each value of `data` in the control of that name.
    ///
    /// # Errors
    ///
    /// As [`JsonDialog::set_data`].
    pub fn set_data(&mut self, data: &Value) -> Result<(), SpecError> {
        set_data(self.dialog, self.fields, data)
    }

    /// The control named `name` as its turbo-vision type.
    pub fn view_mut<V: View + 'static>(&mut self, name: &str) -> Option<&mut V> {
        let id = self.fields.iter().find(|f| f.name == name)?.id;
        find_view_mut(self.dialog, id)?.as_any_mut().downcast_mut()
    }

    /// The turbo-vision dialog.
    pub fn dialog_mut(&mut self) -> &mut Dialog {
        self.dialog
    }
}

// ---- values ----------------------------------------------------------------

fn view<V: 'static>(dialog: &mut Dialog, id: ViewId) -> Option<&mut V> {
    find_view_mut(dialog, id)?.as_any_mut().downcast_mut()
}

fn index_value(index: Option<usize>) -> Value {
    index.map_or(Value::Null, Value::from)
}

/// The value of `field`, as JSON.
fn read(dialog: &mut Dialog, field: &Field) -> Value {
    let id = field.id;
    let value = match &field.kind {
        Kind::Input { .. } => view::<InputLine>(dialog, id).map(|v| Value::from(v.text())),
        Kind::Memo { .. } => view::<Memo>(dialog, id).map(|v| Value::from(v.get_text())),
        Kind::CheckBox { .. } => view::<CheckBox>(dialog, id).map(|v| Value::from(v.is_checked())),
        Kind::CheckBoxes { .. } => {
            view::<CheckBoxes>(dialog, id).map(|v| Value::from(v.checked_items()))
        }
        Kind::RadioButtons { .. } => {
            view::<RadioButtons>(dialog, id).map(|v| index_value(v.selected()))
        }
        Kind::ComboBox { .. } => view::<ComboBox>(dialog, id).map(|v| index_value(v.selected())),
        Kind::ListBox { multi: true, .. } => {
            view::<ListBox>(dialog, id).map(|v| Value::from(v.marked_items()))
        }
        Kind::ListBox { .. } => view::<ListBox>(dialog, id).map(|v| index_value(v.get_selection())),
        Kind::Spinner { .. } => view::<Spinner>(dialog, id).map(|v| Value::from(v.value())),
        Kind::Slider { .. } => view::<Slider>(dialog, id).map(|v| Value::from(v.value())),
        Kind::ProgressBar { .. } => view::<ProgressBar>(dialog, id).map(|v| Value::from(v.value())),
        Kind::Table { .. } => view::<Table>(dialog, id).map(|v| index_value(v.selected_row())),
        Kind::AutoComplete { .. } => {
            view::<AutoComplete>(dialog, id).map(|v| Value::from(v.value()))
        }
        Kind::DualList { .. } => {
            view::<DualList<String>>(dialog, id).map(|v| Value::from(v.chosen_keys()))
        }
        _ => None,
    };
    value.unwrap_or(Value::Null)
}

fn data(dialog: &mut Dialog, fields: &[Field]) -> Value {
    Value::Object(
        fields
            .iter()
            .map(|f| (f.name.clone(), read(dialog, f)))
            .collect(),
    )
}

fn set_data(dialog: &mut Dialog, fields: &[Field], data: &Value) -> Result<(), SpecError> {
    let Value::Object(map) = data else {
        return Err(SpecError(format!("data is an object, not {data}")));
    };
    for (name, value) in map {
        set_value(dialog, fields, name, value)?;
    }
    Ok(())
}

fn set_value(
    dialog: &mut Dialog,
    fields: &[Field],
    name: &str,
    value: &Value,
) -> Result<(), SpecError> {
    let field = fields
        .iter()
        .find(|f| f.name == name)
        .ok_or_else(|| SpecError(format!("no control with a value is named \"{name}\"")))?;
    write(dialog, field, value).map_err(|expected| {
        SpecError(format!(
            "{} \"{name}\" takes {expected}, not {value}",
            field.kind.type_name()
        ))
    })
}

fn as_index(value: &Value) -> Option<usize> {
    value.as_u64().and_then(|n| usize::try_from(n).ok())
}

fn as_indices(value: &Value) -> Option<Vec<usize>> {
    value.as_array()?.iter().map(as_index).collect()
}

fn as_text(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Null => Some(String::new()),
        _ => None,
    }
}

/// Show `value` in `field`'s control; on a type mismatch, say what it
/// takes.
#[allow(clippy::too_many_lines, reason = "one arm per control kind")]
fn write(dialog: &mut Dialog, field: &Field, value: &Value) -> Result<(), &'static str> {
    let id = field.id;
    let missing = "a control that exists";
    match &field.kind {
        Kind::Input { .. } => {
            let text = as_text(value).ok_or("a string")?;
            view::<InputLine>(dialog, id).ok_or(missing)?.set_text(text);
        }
        Kind::Memo { .. } => {
            let text = as_text(value).ok_or("a string")?;
            view::<Memo>(dialog, id).ok_or(missing)?.set_text(&text);
        }
        Kind::AutoComplete { .. } => {
            let text = as_text(value).ok_or("a string")?;
            view::<AutoComplete>(dialog, id)
                .ok_or(missing)?
                .set_text(text);
        }
        Kind::CheckBox { .. } => {
            let checked = value.as_bool().ok_or("true or false")?;
            view::<CheckBox>(dialog, id)
                .ok_or(missing)?
                .set_checked(checked);
        }
        Kind::CheckBoxes { items, .. } => {
            let checked = as_indices(value)
                .filter(|v| v.iter().all(|&i| i < items.len()))
                .ok_or("a list of item indices")?;
            let boxes = view::<CheckBoxes>(dialog, id).ok_or(missing)?;
            for i in 0..items.len() {
                boxes.set_checked(i, checked.contains(&i));
            }
        }
        Kind::RadioButtons { items, .. } => {
            let index = as_index(value)
                .filter(|&i| i < items.len())
                .ok_or("an item index")?;
            view::<RadioButtons>(dialog, id)
                .ok_or(missing)?
                .set_selected(index);
        }
        Kind::ComboBox { items, .. } => {
            let index = match value {
                Value::Null => None,
                v => Some(
                    as_index(v)
                        .filter(|&i| i < items.len())
                        .ok_or("an item index or null")?,
                ),
            };
            view::<ComboBox>(dialog, id)
                .ok_or(missing)?
                .set_selected(index);
        }
        Kind::ListBox {
            items, multi: true, ..
        } => {
            let marked = as_indices(value)
                .filter(|v| v.iter().all(|&i| i < items.len()))
                .ok_or("a list of item indices")?;
            let list = view::<ListBox>(dialog, id).ok_or(missing)?;
            list.clear_marks();
            for i in marked {
                list.set_marked(i, true);
            }
        }
        Kind::ListBox { items, .. } => match value {
            Value::Null => {}
            v => {
                let index = as_index(v)
                    .filter(|&i| i < items.len())
                    .ok_or("an item index or null")?;
                view::<ListBox>(dialog, id)
                    .ok_or(missing)?
                    .set_selection(index);
            }
        },
        Kind::Spinner { min, max, .. } | Kind::Slider { min, max, .. } => {
            let n = value
                .as_i64()
                .filter(|n| (*min..=*max).contains(n))
                .ok_or("a whole number in its range")?;
            if let Some(s) = view::<Spinner>(dialog, id) {
                s.set_value(n);
            } else {
                view::<Slider>(dialog, id).ok_or(missing)?.set_value(n);
            }
        }
        Kind::ProgressBar { .. } => {
            let n = value.as_u64().ok_or("a whole number")?;
            view::<ProgressBar>(dialog, id).ok_or(missing)?.set_value(n);
        }
        Kind::Table { .. } => match value {
            Value::Null => {}
            v => {
                let row = as_index(v).ok_or("a row index or null")?;
                view::<Table>(dialog, id)
                    .ok_or(missing)?
                    .set_selected_row(row);
            }
        },
        Kind::DualList { .. } => {
            let keys: Vec<String> = value
                .as_array()
                .and_then(|a| a.iter().map(|k| k.as_str().map(str::to_string)).collect())
                .ok_or("a list of keys")?;
            view::<DualList<String>>(dialog, id)
                .ok_or(missing)?
                .set_chosen(&keys);
        }
        _ => return Err("no value"),
    }
    Ok(())
}

// ---- validation ------------------------------------------------------------

fn validate(
    dialog: &mut Dialog,
    fields: &[Field],
    rules: &[(String, FieldRule)],
    data_rules: &[DataRule],
) -> Result<Value, Errors> {
    let mut errors = Errors::new();
    let Value::Object(data) = data(dialog, fields) else {
        unreachable!("data is always an object");
    };
    for field in fields {
        let value = &data[&field.name];
        if let Some(message) = built_in_error(dialog, field, value) {
            errors.add(&field.name, message);
            continue;
        }
        for (_, rule) in rules.iter().filter(|(name, _)| *name == field.name) {
            if let Err(message) = rule(value) {
                errors.add(&field.name, message);
                break;
            }
        }
    }
    if errors.is_empty() {
        for rule in data_rules {
            rule(&data, &mut errors);
        }
    }
    if errors.is_empty() {
        Ok(Value::Object(data))
    } else {
        Err(errors)
    }
}

/// What the spec itself says is wrong with `field`: a required control
/// left empty, or an input its validator refuses.
fn built_in_error(dialog: &mut Dialog, field: &Field, value: &Value) -> Option<String> {
    let title = &field.title;
    let blank = value.as_str().is_some_and(|s| s.trim().is_empty());
    match &field.kind {
        Kind::Input { required, .. } => {
            if *required && blank {
                return Some(format!("{title} is required"));
            }
            let valid = view::<InputLine>(dialog, field.id).is_none_or(|v| blank || v.is_valid());
            (!valid).then(|| format!("{title} is not valid"))
        }
        Kind::Memo { required: true, .. } if blank => Some(format!("{title} is required")),
        Kind::CheckBox { required: true, .. } if value != &Value::Bool(true) => {
            Some(format!("{title} must be checked"))
        }
        Kind::ComboBox { required: true, .. } if value.is_null() => {
            Some(format!("{title} must be chosen"))
        }
        _ => None,
    }
}

fn focus_first(dialog: &mut Dialog, fields: &[Field], errors: &Errors) {
    let first = errors
        .iter()
        .find_map(|(name, _)| fields.iter().find(|f| Some(f.name.as_str()) == name));
    if let Some(field) = first {
        focus_view(dialog, field.id);
    }
}

/// Mark the labels of the controls at fault and put the right message on
/// the error line: the focused control's, or the first.
fn display(dialog: &mut Dialog, fields: &[Field], error_line: Option<ViewId>, errors: &Errors) {
    for field in fields {
        if let Some(label) = field.label.and_then(|id| view::<Label>(dialog, id)) {
            label.set_error(errors.field(&field.name).is_some());
        }
    }
    let Some(line) = error_line else {
        return;
    };
    let focused = focused_view(dialog)
        .and_then(|id| fields.iter().find(|f| f.id == id))
        .and_then(|f| errors.field(&f.name));
    let shown = focused.or_else(|| errors.iter().next().map(|(_, m)| m));
    let text = match shown {
        None => String::new(),
        Some(message) if errors.len() > 1 => format!("{message} (+{} more)", errors.len() - 1),
        Some(message) => message.to_string(),
    };
    if let Some(line) = view::<ErrorLine>(dialog, line) {
        line.text = text;
    }
}

// ---- building ----------------------------------------------------------------

/// A control's `~`-marked caption as a name for messages.
fn title_of(text: &str) -> String {
    let plain: String = text.chars().filter(|&c| c != '~').collect();
    plain.trim().trim_end_matches([':', '*']).trim().to_string()
}

fn capitalised(name: &str) -> String {
    let words = name.replace(['_', '-'], " ");
    let mut chars = words.chars();
    chars.next().map_or_else(String::new, |c| {
        c.to_uppercase().collect::<String>() + chars.as_str()
    })
}

fn rect(c: &Control) -> Rect {
    Rect::new(c.x, c.y, c.x + c.w, c.y + c.h)
}

fn validator_of(spec: &ValidatorSpec) -> Option<ValidatorRef> {
    Some(match spec {
        ValidatorSpec::Range { min, max } => Rc::new(RefCell::new(RangeValidator::new(*min, *max))),
        ValidatorSpec::Filter { chars } => Rc::new(RefCell::new(FilterValidator::new(chars))),
        ValidatorSpec::Picture { mask } => Rc::new(RefCell::new(PictureValidator::new(mask))),
        ValidatorSpec::Lookup { values } => {
            Rc::new(RefCell::new(LookupValidator::new(values.clone())))
        }
        ValidatorSpec::Custom { .. } => return None,
    })
}

/// Builds a [`JsonDialog`] from a checked spec, control by control.
struct Builder<'a> {
    spec: &'a DialogSpec,
    fields: Vec<Field>,
    actions: Vec<String>,
    custom: Vec<(String, ViewId, bool)>,
    /// Every named control's view, for label links.
    ids: HashMap<String, ViewId>,
    /// Each label with a link, and the name it links to.
    links: Vec<(ViewId, String)>,
    /// Each hint, with its control's place in the dialog's space.
    hints: Vec<(Rect, String)>,
    error_line: Option<ViewId>,
}

impl<'a> Builder<'a> {
    fn new(spec: &'a DialogSpec) -> Self {
        Self {
            spec,
            fields: Vec::new(),
            actions: Vec::new(),
            custom: Vec::new(),
            ids: HashMap::new(),
            links: Vec::new(),
            hints: Vec::new(),
            error_line: None,
        }
    }

    fn build(mut self) -> JsonDialog {
        let spec = self.spec;
        let (x, y) = (spec.x.unwrap_or(0), spec.y.unwrap_or(0));
        let mut dialog = Dialog::new(
            Rect::new(x, y, x + spec.width, y + spec.height),
            &spec.title,
        );
        if spec.x.is_none() && spec.y.is_none() {
            dialog.set_options(dialog.options() | Options::CENTERED);
        } else if spec.x.is_none() {
            dialog.set_options(dialog.options() | Options::CENTER_X);
        } else if spec.y.is_none() {
            dialog.set_options(dialog.options() | Options::CENTER_Y);
        }
        dialog.set_resizable(spec.resizable);

        for control in &spec.controls {
            self.add(&mut dialog, control, Point::new(0, 0));
        }
        let changes = Rc::new(RefCell::new(Vec::new()));
        let listened: Vec<CommandId> = (0..self.actions.len()).map(command_of).collect();
        dialog.add(Listener::new(listened, Rc::clone(&changes)));
        if !self.hints.is_empty() {
            let (w, h) = spec.inner_size();
            let mut tips = Tooltip::new(Rect::new(0, 0, w, h));
            for (target, text) in self.hints.drain(..) {
                tips.add_hint(target, text);
            }
            dialog.add(tips);
        }

        // Link the labels, and give each field its label and title.
        for (label, target) in &self.links {
            let Some(&target_id) = self.ids.get(target) else {
                continue;
            };
            if let Some(view) = view::<Label>(&mut dialog, *label) {
                view.set_link(target_id);
            }
            if let Some(field) = self.fields.iter_mut().find(|f| &f.name == target) {
                field.label = Some(*label);
            }
        }
        let captions: HashMap<&str, String> = {
            let mut map = HashMap::new();
            spec.visit(&mut |c| {
                if let Kind::Label {
                    text,
                    link: Some(target),
                } = &c.kind
                {
                    map.entry(target.as_str()).or_insert_with(|| title_of(text));
                }
            });
            map
        };
        for field in &mut self.fields {
            field.title = match (captions.get(field.name.as_str()), &field.kind) {
                (Some(caption), _) if !caption.is_empty() => caption.clone(),
                (_, Kind::CheckBox { text, .. }) if !title_of(text).is_empty() => title_of(text),
                _ => capitalised(&field.name),
            };
        }

        JsonDialog {
            spec: spec.clone(),
            dialog,
            fields: self.fields,
            actions: self.actions,
            handlers: HashMap::new(),
            rules: Vec::new(),
            data_rules: Vec::new(),
            custom: self.custom,
            changes,
            error_line: self.error_line,
        }
    }

    /// The command for the action `name`, adding it on first use.
    fn action(&mut self, name: &str) -> CommandId {
        let index = self
            .actions
            .iter()
            .position(|a| a == name)
            .unwrap_or_else(|| {
                self.actions.push(name.to_string());
                self.actions.len() - 1
            });
        command_of(index)
    }

    fn on_change(&mut self, action: Option<&String>) -> CommandId {
        action.map_or(0, |a| self.action(a))
    }

    /// Add `control` to `group`, whose corner is at `origin` in the
    /// dialog's space.
    #[allow(clippy::too_many_lines, reason = "one arm per control kind")]
    fn add(&mut self, group: &mut impl GroupLike, control: &Control, origin: Point) {
        let bounds = rect(control);
        let id = match &control.kind {
            Kind::Label { text, link } => {
                let id = group.add(Label::new(bounds, text));
                if let Some(target) = link {
                    self.links.push((id, target.clone()));
                }
                id
            }
            Kind::StaticText { text, centered } => group.add(if *centered {
                StaticText::new_centered(bounds, text)
            } else {
                StaticText::new(bounds, text)
            }),
            Kind::Button {
                text,
                command,
                default,
            } => {
                let command = match command.as_str() {
                    "ok" => CM_OK,
                    "cancel" => CM_CANCEL,
                    "yes" => CM_YES,
                    "no" => CM_NO,
                    action => self.action(action),
                };
                group.add(Button::new(bounds, text, command, *default))
            }
            Kind::Input {
                text,
                max_len,
                validator,
                history,
                ..
            } => {
                let mut input = InputLine::new(bounds, *max_len);
                input.set_text(text.clone());
                if let Some(v) = validator.as_ref().and_then(validator_of) {
                    input.set_validator(v);
                }
                let handle = group.add_typed(input);
                if let Some(ValidatorSpec::Custom { name }) = validator {
                    self.custom.push((name.clone(), handle.id(), false));
                }
                if let Some(history) = history {
                    group.add(History::new(
                        Point::new(bounds.b.x, bounds.a.y),
                        *history,
                        handle,
                    ));
                }
                handle.id()
            }
            Kind::Memo {
                text,
                max_len,
                read_only,
                ..
            } => {
                let mut memo = Memo::new(bounds);
                memo.set_text(text);
                memo.set_max_length(*max_len);
                memo.set_read_only(*read_only);
                group.add(memo)
            }
            Kind::CheckBox { text, checked, .. } => {
                let mut b = CheckBox::new(bounds, text);
                b.set_checked(*checked);
                group.add(b)
            }
            Kind::CheckBoxes {
                items,
                checked,
                on_change,
            } => {
                let mut b = CheckBoxes::new(bounds, items.clone());
                for &i in checked {
                    b.set_checked(i, true);
                }
                b.set_on_change(self.on_change(on_change.as_ref()));
                group.add(b)
            }
            Kind::RadioButtons {
                items,
                selected,
                on_change,
            } => {
                let mut b = RadioButtons::new(bounds, items.clone());
                if !items.is_empty() {
                    b.set_selected(*selected);
                }
                b.set_on_change(self.on_change(on_change.as_ref()));
                group.add(b)
            }
            Kind::ComboBox {
                items,
                selected,
                on_change,
                ..
            } => {
                let mut b = ComboBox::with_items(bounds, next_combo_id(), items.clone());
                b.set_selected(*selected);
                b.set_on_change(self.on_change(on_change.as_ref()));
                group.add(b)
            }
            Kind::ListBox {
                items,
                selected,
                multi,
            } => {
                let mut b = ListBox::new(bounds, 0);
                b.set_items(items.clone());
                b.set_multi_select(*multi);
                if let Some(i) = selected {
                    b.set_selection(*i);
                }
                group.add(b)
            }
            Kind::Spinner {
                min,
                max,
                value,
                step,
                suffix,
                wrap,
                on_change,
            } => {
                let mut s = Spinner::new(bounds, *min, *max);
                s.set_value((*value).clamp(*min, *max));
                s.set_step(*step);
                s.set_suffix(suffix.clone());
                s.set_wrap(*wrap);
                s.set_on_change(self.on_change(on_change.as_ref()));
                group.add(s)
            }
            Kind::Slider {
                min,
                max,
                value,
                step,
                on_change,
            } => {
                let mut s = Slider::new(bounds, *min, *max);
                s.set_value((*value).clamp(*min, *max));
                s.set_step(*step);
                s.set_on_change(self.on_change(on_change.as_ref()));
                group.add(s)
            }
            Kind::ProgressBar {
                max,
                value,
                show_percent,
            } => {
                let mut p = ProgressBar::new(bounds, *max);
                p.set_value(*value);
                p.set_show_percent(*show_percent);
                group.add(p)
            }
            Kind::Table {
                columns,
                rows,
                header,
                separators,
            } => {
                let mut t = Table::new(bounds, 0);
                t.set_columns(
                    columns
                        .iter()
                        .map(|c| match c.align {
                            Align::Left => Column::new(c.title.clone(), c.width),
                            Align::Right => Column::right(c.title.clone(), c.width),
                        })
                        .collect(),
                );
                t.set_rows(rows.clone());
                t.set_show_header(*header);
                t.set_separators(*separators);
                group.add(t)
            }
            Kind::GroupBox { title } => group.add(GroupBox::new(bounds, title)),
            Kind::Tabs { pages, active } => {
                let mut pane = TabbedPane::new(bounds);
                let area = pane.page_area();
                let page_origin = Point::new(
                    origin.x + bounds.a.x + area.a.x,
                    origin.y + bounds.a.y + area.a.y,
                );
                for page in pages {
                    let mut group = Group::new(area);
                    for child in &page.controls {
                        self.add(&mut group, child, page_origin);
                    }
                    pane.add_page(&page.title, group);
                }
                pane.set_active(*active);
                group.add(pane)
            }
            Kind::AutoComplete {
                items,
                text,
                max_len,
                min_chars,
                required,
                require_match,
            } => group.add(
                AutoCompleteBuilder::new()
                    .bounds(Rect::new(
                        bounds.a.x,
                        bounds.a.y,
                        bounds.b.x,
                        bounds.a.y + 1,
                    ))
                    .items(items.clone())
                    .data(Rc::new(RefCell::new(text.clone())))
                    .max_length(*max_len)
                    .min_chars(*min_chars)
                    .required(*required)
                    .require_match(*require_match)
                    .build(),
            ),
            Kind::DualList {
                items,
                chosen,
                titles,
                min,
                max,
            } => {
                let mut builder: DualListBuilder<String> = DualListBuilder::new()
                    .bounds(bounds)
                    .items(
                        items
                            .iter()
                            .map(|i| (i.key().to_string(), i.label().to_string())),
                    )
                    .data(Rc::new(RefCell::new(chosen.clone())))
                    .min_chosen(*min);
                if let Some(max) = max {
                    builder = builder.max_chosen(*max);
                }
                if let Some((available, chosen)) = titles {
                    builder = builder.titles(available.clone(), chosen.clone());
                }
                let mut list: dual_list::DualList<String> = builder.build();
                list.set_chosen(chosen);
                group.add(list)
            }
            Kind::ErrorLine {} => {
                let mut line = ErrorLine::new();
                line.set_bounds(bounds);
                let id = group.add(line);
                self.error_line.get_or_insert(id);
                id
            }
        };
        if !control.name.is_empty() {
            self.ids.insert(control.name.clone(), id);
            if control.kind.has_value() {
                self.fields.push(Field {
                    name: control.name.clone(),
                    id,
                    kind: control.kind.clone(),
                    label: None,
                    title: String::new(),
                });
            }
        }
        if let Some(hint) = &control.hint {
            let mut at = bounds;
            at.move_by(origin.x, origin.y);
            self.hints.push((at, hint.clone()));
        }
    }
}

fn command_of(index: usize) -> CommandId {
    ACTION_BASE + CommandId::try_from(index).unwrap_or(CommandId::MAX - ACTION_BASE)
}

/// Hears the change broadcasts of the dialog's controls and queues the
/// ones that name an action, for the modal loop to run. Takes no room and
/// never the focus.
struct Listener {
    core: ViewCore,
    commands: Vec<CommandId>,
    heard: Rc<RefCell<Vec<CommandId>>>,
}

impl Listener {
    fn new(commands: Vec<CommandId>, heard: Rc<RefCell<Vec<CommandId>>>) -> Self {
        Self {
            core: ViewCore::new(Rect::new(0, 0, 0, 0)),
            commands,
            heard,
        }
    }
}

impl View for Listener {
    fn core(&self) -> &ViewCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut ViewCore {
        &mut self.core
    }

    fn draw(&mut self, _terminal: &mut Terminal) {}

    fn get_palette(&self) -> Option<turbo_vision::core::palette::Palette> {
        None
    }

    fn handle_event(&mut self, event: &mut Event) {
        // Never cleared: other views may want the broadcast too.
        if event.what == EventType::Broadcast && self.commands.contains(&event.command) {
            self.heard.borrow_mut().push(event.command);
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
