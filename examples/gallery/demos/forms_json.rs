//! A form described as JSON and answered as JSON, for a program that cannot
//! draw a dialog itself: an agent or an MCP tool says what to ask, a person
//! fills the form in, and the program gets the answers back as one JSON
//! object, or a cancel. The checks are the record forms': a required field
//! left blank or a number out of range keeps the dialog open, with the error
//! under the field. The full guide is the "Forms as JSON" part of
//! docs/forms.md.
//!
//! Parameters:
//! - `run(app, &spec)`: builds the form, runs it, and returns the answers
//!   keyed by field name, `FormError::Cancelled`, or `FormError::Spec`
//!   naming what is wrong with the spec.
//! - `JsonForm::from_spec(&spec)` then `run(app)`: the same, built once and
//!   run as many times as you like.
//! - `title`: the dialog's title. `fields`: one object per field, with a
//!   `name` (the answer's key), a `type` (text, integer, number, boolean,
//!   choice, multichoice, memo, date, ...), and an optional `label`,
//!   `default`, `required`, `min` and `max`.
//! - `options`: a choice's values, plain or as `{ label, value }`;
//!   `"widget": "radio"` shows them as radio buttons.
//!
//! See also: Form, DualList, AutoComplete

use crate::panel::Panel;
use serde_json::json;
use turbo_vision::app::Application;
use turbo_vision::core::command::{CM_USER, CommandId};
use turbo_vision::core::geometry::Rect;
use turbo_vision::views::button::Button;
use turbo_vision::views::msgbox::message_box_ok;
use tv_extensions::forms::json::{FormError, run};

const OPEN: CommandId = CM_USER + 100;

pub fn build(panel: &mut Panel) {
    panel.add(Button::new(
        Rect::new(0, 0, 22, 2),
        "~O~pen the JSON form",
        OPEN,
        true,
    ));
}

pub fn handle(app: &mut Application, command: CommandId) -> bool {
    if command != OPEN {
        return false;
    }
    let spec = json!({
        "title": "New customer",
        "fields": [
            { "name": "name", "type": "text", "label": "Name", "required": true },
            { "name": "age", "type": "integer", "min": 0, "max": 120 },
            {
                "name": "kind", "type": "choice", "widget": "radio",
                "options": ["Retail", "Wholesale"]
            },
            { "name": "vip", "type": "boolean", "label": "VIP customer" }
        ]
    });
    match run(app, &spec) {
        Ok(answers) => {
            message_box_ok(app, &format!("Answers: {answers}"));
        }
        Err(FormError::Cancelled) => {}
        Err(FormError::Spec(why)) => {
            message_box_ok(app, &format!("Bad form: {why}"));
        }
    }
    true
}
