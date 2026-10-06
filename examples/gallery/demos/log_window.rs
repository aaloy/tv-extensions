//! A window that shows `tracing` events as they happen, each with its time
//! and its level in the level's colour, on black. Building it installs it
//! as the global `tracing` subscriber, so `info!`, `warn!` and the other
//! macros land in it from anywhere in the program. Open it, then log.
//!
//! The subscriber is global, and `tracing` takes one per program: only the
//! first LogWindow built gets the events. Here the window is opened once
//! and brought back to the front after that.
//!
//! Parameters:
//! - `LogWindowBuilder::new().bounds(r)`: the window; `title(text)`, its
//!   title ("Log" by default).
//! - `min_level(level)`: the least severe level shown (TRACE by default).
//! - `max_lines(n)`: how many lines it keeps (10000 by default).
//! - `log(level, text)`: add a line without going through `tracing`;
//!   `clear()` empties it.
//!
//! See also: TerminalWidget, Host-driven app

use crate::panel::Panel;
use std::cell::Cell;
use turbo_vision::app::Application;
use turbo_vision::core::command::{CM_USER, CommandId};
use turbo_vision::core::geometry::Rect;
use turbo_vision::views::ViewId;
use turbo_vision::views::button::Button;
use tv_extensions::log::LogWindowBuilder;

const OPEN: CommandId = CM_USER + 100;
const INFO: CommandId = CM_USER + 101;
const WARN: CommandId = CM_USER + 102;
const ERROR: CommandId = CM_USER + 103;

thread_local! {
    /// The window, once it has been opened.
    static WINDOW: Cell<Option<ViewId>> = const { Cell::new(None) };
}

pub fn build(panel: &mut Panel) {
    panel.add(Button::new(Rect::new(0, 0, 10, 2), "~O~pen", OPEN, true));
    panel.add(Button::new(Rect::new(12, 0, 22, 2), "~I~nfo", INFO, false));
    panel.add(Button::new(Rect::new(24, 0, 34, 2), "~W~arn", WARN, false));
    panel.add(Button::new(
        Rect::new(36, 0, 46, 2),
        "~E~rror",
        ERROR,
        false,
    ));
}

pub fn handle(app: &mut Application, command: CommandId) -> bool {
    match command {
        OPEN => match WINDOW.get() {
            Some(id) if app.desktop.child_by_id(id).is_some() => {
                app.desktop.bring_to_front(id);
            }
            _ => {
                let window = LogWindowBuilder::new()
                    .bounds(Rect::new(26, 12, 78, 22))
                    .title("Log")
                    .build();
                WINDOW.set(Some(app.desktop.add(window)));
                tracing::info!("the log window is the tracing subscriber now");
            }
        },
        INFO => tracing::info!("a button was pressed"),
        WARN => tracing::warn!("disk is 91% full"),
        ERROR => tracing::error!("connection refused (10.0.0.1:5432)"),
        _ => return false,
    }
    true
}
