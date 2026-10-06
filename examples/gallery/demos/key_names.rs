//! Key events from key names such as "ctrl-s", "shift-tab" or "f10", the
//! form a host that is not a terminal (a web page, a WASM host) reports
//! keys in. Type a name and press Translate to see the event Turbo Vision
//! gets.
//!
//! Parameters:
//! - `translate(code, text)`: `code` is the name, lowercase, with any of
//!   "ctrl-", "alt-" and "shift-" in front; `text` is the character typed,
//!   with its case, for a plain key. It returns `None` for a key Turbo
//!   Vision has no name for.
//! - `is_ctrl(&event)`: whether the event carries Control; Ctrl+Ins and
//!   Ins share a key code, so only the modifiers tell them apart.
//!
//! See also: Host-driven app, CSV editor

use crate::panel::Panel;
use std::cell::RefCell;
use std::rc::Rc;
use turbo_vision::app::Application;
use turbo_vision::core::command::{CM_USER, CommandId};
use turbo_vision::core::geometry::Rect;
use turbo_vision::views::button::Button;
use turbo_vision::views::input_line::InputLine;
use turbo_vision::views::label::Label;
use turbo_vision::views::msgbox::message_box_ok;
use turbo_vision::views::shared::Shared;
use tv_extensions::keys::{is_ctrl, translate};

const TRANSLATE: CommandId = CM_USER + 100;

thread_local! {
    /// The input, shared so the handler can read what was typed.
    static NAME: RefCell<Option<Rc<RefCell<InputLine>>>> = const { RefCell::new(None) };
}

pub fn build(panel: &mut Panel) {
    let mut input = InputLine::new(Rect::new(10, 0, 30, 1), 20);
    input.set_text("ctrl-s");
    let input = Rc::new(RefCell::new(input));
    NAME.set(Some(Rc::clone(&input)));
    let id = panel.add(Shared::new(input));
    let mut label = Label::new(Rect::new(0, 0, 10, 1), "~K~ey name");
    label.set_link(id);
    panel.add(label);
    panel.add(Button::new(
        Rect::new(32, 0, 46, 2),
        "~T~ranslate",
        TRANSLATE,
        true,
    ));
}

pub fn handle(app: &mut Application, command: CommandId) -> bool {
    if command != TRANSLATE {
        return false;
    }
    let name = NAME.with_borrow(|input| input.as_ref().map(|i| i.borrow().get_text()));
    let name = name.unwrap_or_default();
    let text = match translate(name.trim(), None) {
        Some(event) => format!(
            "\"{}\" is key code {:#06x}{}.",
            name.trim(),
            event.key_code,
            if is_ctrl(&event) {
                ", with Control"
            } else {
                ""
            }
        ),
        None => format!("Turbo Vision has no key named \"{}\".", name.trim()),
    };
    message_box_ok(app, &text);
    true
}
