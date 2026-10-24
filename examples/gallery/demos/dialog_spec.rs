//! A dialog laid out in the dialog designer and saved as JSON, built at run
//! time: every control sits where the designer put it, and the code only
//! names what to react to. Here moving the slider drives the meter, and the
//! mode sets both to its usual level. OK returns every named control's
//! value as one JSON object. The full guide is docs/dialog-json.md.
//!
//! Parameters:
//! - `JsonDialog::from_json(text)`, or `load(path)` for a file the designer
//!   saved: the dialog, or a `SpecError` naming the first rule the spec
//!   breaks.
//! - `on_change(name, handler)`: runs each time the person changes the
//!   control of that name, whatever its type; `ctx.value(name)` and
//!   `ctx.set_value(name, &value)` read and set any named control.
//!   `Flow::Stay` keeps the dialog open.
//! - `on(action, handler)`: runs when a button with that `action` is
//!   pressed. `validator(name, v)` and `rule(..)` add checks run on OK.
//! - `run(app)`: shows the dialog; `Some(answer)` with `answer.action` and
//!   `answer.data`, or `None` when cancelled.
//!
//! See also: Form from JSON, Form

use crate::panel::Panel;
use serde_json::json;
use turbo_vision::app::Application;
use turbo_vision::core::command::{CM_USER, CommandId};
use turbo_vision::core::geometry::Rect;
use turbo_vision::views::button::Button;
use turbo_vision::views::msgbox::message_box_ok;
use tv_extensions::dialog_spec::{Flow, JsonDialog};

const OPEN: CommandId = CM_USER + 100;

/// What the designer saves: examples/dialog_specs/volume.json.
const SPEC: &str = include_str!("../../dialog_specs/volume.json");

pub fn build(panel: &mut Panel) {
    panel.add(Button::new(
        Rect::new(0, 0, 24, 2),
        "~O~pen the JSON dialog",
        OPEN,
        true,
    ));
}

pub fn handle(app: &mut Application, command: CommandId) -> bool {
    if command != OPEN {
        return false;
    }
    let mut dialog = match JsonDialog::from_json(SPEC) {
        Ok(dialog) => dialog,
        Err(e) => {
            message_box_ok(app, &format!("Bad dialog: {e}"));
            return true;
        }
    };
    // The slider moved: the meter follows it.
    dialog.on_change("volume", |ctx| {
        if let Some(level) = ctx.value("volume") {
            let _ = ctx.set_value("meter", &level);
        }
        Flow::Stay
    });
    // The mode changed: the slider and the meter go to its usual level.
    dialog.on_change("mode", |ctx| {
        let level = if ctx.value("mode") == Some(json!(1)) {
            80
        } else {
            20
        };
        let _ = ctx.set_value("volume", &json!(level));
        let _ = ctx.set_value("meter", &json!(level));
        Flow::Stay
    });
    if let Some(answer) = dialog.run(app) {
        message_box_ok(app, &format!("Answers: {}", answer.data));
    }
    true
}
