// (C) 2026 - Antoni Aloy

//! Dialogs designed with the dialog designer, saved as JSON, and brought to
//! life at run time with validation and actions bound in code.
//!
//! Turbo Vision dialogs are usually laid out in code, a `Rect` per control.
//! Here the layout lives in a JSON file instead: draw the dialog in the
//! designer (`cargo run --example dialog_designer --features native,dialog`),
//! save it, and load it in the application. The JSON says *what* is on the
//! form and *where*; the code says what the JSON cannot: rules that need
//! logic, and what the buttons do.
//!
//! - [`DialogSpec`] is the file: the dialog's size and title and its
//!   [`Control`]s. Read it with [`DialogSpec::from_json`] or
//!   [`DialogSpec::load`], write it with [`DialogSpec::to_json`] or
//!   [`DialogSpec::save`].
//! - [`JsonDialog`] is the live dialog built from it: fill and read it as
//!   JSON, bind rules and actions to its controls by name, and
//!   [`run`](JsonDialog::run) it.
//!
//! ```no_run
//! use serde_json::json;
//! use turbo_vision::app::Application;
//! use tv_extensions::dialog_spec::{Flow, JsonDialog};
//!
//! # fn main() -> Result<(), tv_extensions::dialog_spec::SpecError> {
//! # let mut app = Application::with_terminal(turbo_vision::test_util::test_terminal(80, 25));
//! let mut dialog = JsonDialog::load("customer.json")?;
//! dialog.set_data(&json!({ "name": "Ada" }))?;
//!
//! // A rule for one control, and one across controls.
//! dialog.rule("email", |v| match v.as_str() {
//!     Some(e) if !e.is_empty() && !e.contains('@') => Err("Email must contain @".into()),
//!     _ => Ok(()),
//! });
//! dialog.validate_data(|data, errors| {
//!     if data["kind"] == json!(1) && data["email"] == json!("") {
//!         errors.add("email", "Wholesale customers need an email");
//!     }
//! });
//!
//! // What the button whose "command" is "lookup" does.
//! dialog.on("lookup", |ctx| {
//!     ctx.set_value("email", &json!("ada@example.org")).unwrap();
//!     Flow::Stay
//! });
//!
//! if let Some(answer) = dialog.run(&mut app) {
//!     // answer.action is "ok" (or the action that closed the dialog);
//!     // answer.data is {"name": "Ada", "email": "...", "kind": 0, ...}
//!     println!("{} {}", answer.action, answer.data);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # The JSON
//!
//! ```json
//! {
//!   "version": 1,
//!   "title": "Customer",
//!   "width": 52, "height": 16,
//!   "controls": [
//!     { "type": "label", "x": 2, "y": 1, "w": 8, "h": 1, "text": "~N~ame", "link": "name" },
//!     { "type": "input", "name": "name", "x": 11, "y": 1, "w": 36, "h": 1, "required": true },
//!     { "type": "button", "x": 24, "y": 11, "w": 11, "h": 2, "text": "~O~K", "default": true }
//!   ]
//! }
//! ```
//!
//! The dialog: `title`, `width` and `height` (outer size, frame included),
//! optional `x` and `y` on the desktop (centred when absent), `resizable`,
//! and `controls` in tab order. Every control has a `type`, an optional
//! `name`, its place `x`, `y`, `w`, `h` measured from the corner inside the
//! frame (or inside a tab page), an optional hover `hint`, and the keys of
//! its type. Unknown types and keys are refused, so a typo is an error
//! rather than silently ignored; keys left at their default are left out
//! when saved.
//!
//! | `type` | Keys (`= default`) | Value in the data |
//! |--------|-----------------------------|-------------------|
//! | `label` | `text`, `link` (a control's name) | — |
//! | `static_text` | `text`, `centered` | — |
//! | `button` | `text`, `command` (`"ok"`; or `"cancel"`, `"yes"`, `"no"`, or an action name), `default` | — |
//! | `input` | `text`, `max_len = 255`, `required`, `validator`, `history` (an id) | string |
//! | `memo` | `text`, `max_len`, `required`, `read_only` | string |
//! | `check_box` | `text`, `checked`, `required` (must be checked) | `true`/`false` |
//! | `check_boxes` | `items`, `checked` (indices), `on_change` | list of checked indices |
//! | `radio_buttons` | `items`, `selected = 0`, `on_change` | index |
//! | `combo_box` | `items`, `selected`, `required`, `on_change` | index or `null` |
//! | `list_box` | `items`, `selected`, `multi` | index or `null`; list of indices when `multi` |
//! | `spinner` | `min = 0`, `max`, `value`, `step = 1`, `suffix`, `wrap`, `on_change` | number |
//! | `slider` | `min = 0`, `max`, `value`, `step = 1`, `on_change` | number |
//! | `progress_bar` | `max = 100`, `value`, `show_percent = true` | number |
//! | `table` | `columns` (`title`, `width`, `align`), `rows`, `header = true`, `separators` | selected row or `null` |
//! | `group_box` | `title` (a frame drawn around the controls after it) | — |
//! | `tabs` | `pages` (each `title` and `controls`), `active` | — |
//! | `auto_complete` | `items`, `text`, `max_len`, `min_chars = 1`, `required`, `require_match` | string |
//! | `dual_list` | `items` (strings, or `{"key", "label"}`), `chosen` (keys), `titles`, `min`, `max` | list of chosen keys |
//! | `tree` | `nodes` (strings, or `{"label", "key", "children", "expanded"}`), `selected` (a key) | the key of the selected node (the first at first), or null |
//! | `line` | `style`: `single`, `double`, `heavy`, `round`, `block`; one row tall it is a row, one column wide a column, larger a rectangle | — |
//! | `error_line` | — (where validation messages show) | — |
//!
//! A `validator` on an input checks keystrokes as they are typed:
//! `{"type": "range", "min": 1, "max": 99}`, `{"type": "filter", "chars":
//! "0123456789"}`, `{"type": "picture", "mask": "##/##/####"}`,
//! `{"type": "lookup", "values": [...]}`, or `{"type": "custom", "name":
//! "iban"}` for one your code registers with [`JsonDialog::validator`].
//!
//! [`DialogSpec::check`] enforces what JSON alone cannot: names are unique
//! (across tab pages too), a label links to a control that exists, tabs do
//! not nest, a selection is one of the items, and a range is not upside
//! down.
//!
//! # The data
//!
//! [`JsonDialog::data`] is one flat JSON object: each named control that
//! has a value, keyed by its name, in the spec's order, controls on tab
//! pages included. [`JsonDialog::set_data`] takes the same shape, or any
//! part of it, and refuses unknown names and values of the wrong type.
//! [`JsonDialog::value`] and [`JsonDialog::set_value`] do one control.
//!
//! For anything the data does not cover, reach the control itself by name,
//! as its turbo-vision type: `dialog.view_mut::<Table>("orders")` to fill a
//! table's rows, `dialog.view_mut::<Button>(...)` for a named button.
//!
//! # Validation
//!
//! When the dialog is accepted (OK, Yes, or an action that submits), each
//! named control is checked in tab order:
//!
//! 1. the spec's own rules: `required`, an input's `validator`, a check
//!    box that must be checked, a combo box that must have a choice;
//! 2. the rules bound to it with [`JsonDialog::rule`];
//!
//! and, once every control passes, the rules over all the data bound with
//! [`JsonDialog::validate_data`]. `auto_complete` and `dual_list` check
//! their own `required`, `require_match`, `min` and `max`, showing the
//! error under themselves.
//!
//! If anything fails, the dialog stays open: the labels linked to the
//! controls at fault turn red, the `error_line` shows the message for the
//! focused control (or the first, with how many more), and the focus moves
//! to the first control at fault, bringing its tab page forward. From then
//! on the errors are checked again as the focus moves, so they clear as
//! they are fixed. Without an `error_line` the first message is shown in a
//! message box. Messages name controls by their label's text ("Name is
//! required"); messages your rules return are shown as they are.
//! [`JsonDialog::validate`] runs the same checks without running the
//! dialog, for tests and for code that fills the dialog itself.
//!
//! # Actions
//!
//! A button's `command` that is not `ok`, `cancel`, `yes` or `no` names an
//! action, and so does a control's `on_change`. Bind code to it with
//! [`JsonDialog::on`]; the handler gets an [`ActionContext`] (the
//! application, for message boxes and other dialogs, and the dialog's
//! values and controls) and returns a [`Flow`]:
//!
//! - [`Flow::Stay`]: the dialog stays open (a lookup filled some fields);
//! - [`Flow::Submit`]: validate as OK does, and close if all passes;
//! - [`Flow::Close`]: close at once, without validating ("Discard").
//!
//! A button whose action has no handler validates and closes like OK, and
//! [`Answer::action`] tells which button it was, so a dialog with "Save"
//! and "Save and new" needs no handlers at all. [`JsonDialog::unbound`]
//! lists every action and custom validator the spec names that no code has
//! bound yet: assert it is empty in a test, and a button renamed in the
//! designer cannot go unnoticed.
//!
//! # Extending
//!
//! The JSON covers the controls and the rules that are data. Everything
//! else is ordinary Rust on the built dialog:
//!
//! - **Validators**: implement turbo-vision's `Validator` trait and
//!   register it by name with [`JsonDialog::validator`].
//! - **Rules** of any complexity, with [`JsonDialog::rule`] and
//!   [`JsonDialog::validate_data`]; report against a control
//!   ([`Errors::add`]) or the whole dialog ([`Errors::add_form`]).
//! - **Behaviour**: actions on buttons and `on_change`, and
//!   [`JsonDialog::view_mut`] for any control's own API.
//! - **More views**: [`JsonDialog::dialog_mut`] is a plain turbo-vision
//!   `Dialog`; add views of your own to it.

mod line;
mod runtime;
mod spec;

pub use runtime::{ActionContext, Answer, Errors, Flow, JsonDialog};
pub use spec::{
    Align, ColumnSpec, Control, DialogSpec, Item, Kind, LineStyle, MAX_CLUSTER_ITEMS, PageSpec,
    SpecError, TreeNode, VERSION, ValidatorSpec,
};
