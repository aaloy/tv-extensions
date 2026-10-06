//! A read-only pane of output lines, for build output, program logs or a
//! debug console: lines are appended at the bottom and it scrolls to keep
//! the last one in view. Up, Down, PgUp and PgDn scroll back.
//!
//! Parameters:
//! - `TerminalWidget::new(bounds)`: `with_scrollbar()` adds a bar on the
//!   right.
//! - `append_line(text)`, `append_lines(lines)`, `append_text(text)`: add
//!   lines; `append_text` splits on newlines.
//! - `append_line_colored(text, attr)`: a line in its own colour;
//!   `append_line_spans(spans)`, several colours in one line.
//! - `set_max_lines(n)`: how many lines it keeps; the oldest go first.
//! - `set_auto_scroll(on)`, `scroll_to_bottom()`, `scroll_to_top()`.
//! - `clear()`, `line_count()`.
//!
//! See also: LogWindow, Host-driven app

use crate::panel::Panel;
use std::cell::RefCell;
use std::rc::Rc;
use turbo_vision::app::Application;
use turbo_vision::core::command::{CM_USER, CommandId};
use turbo_vision::core::geometry::Rect;
use turbo_vision::core::palette::{Attr, TvColor};
use turbo_vision::views::button::Button;
use turbo_vision::views::shared::Shared;
use tv_extensions::log::{Span, TerminalWidget};

const BUILD: CommandId = CM_USER + 100;
const CLEAR: CommandId = CM_USER + 101;

thread_local! {
    /// The pane, shared so the handler can append to it.
    static PANE: RefCell<Option<Rc<RefCell<TerminalWidget>>>> = const { RefCell::new(None) };
}

pub fn build(panel: &mut Panel) {
    let mut pane = TerminalWidget::new(Rect::new(0, 0, 32, 6)).with_scrollbar();
    pane.append_line("$ cargo build".to_string());
    let pane = Rc::new(RefCell::new(pane));
    PANE.set(Some(Rc::clone(&pane)));
    panel.add(Shared::new(pane));
    panel.add(Button::new(Rect::new(34, 0, 46, 2), "~B~uild", BUILD, true));
    panel.add(Button::new(
        Rect::new(34, 2, 46, 4),
        "~C~lear",
        CLEAR,
        false,
    ));
}

pub fn handle(_app: &mut Application, command: CommandId) -> bool {
    let Some(pane) = PANE.with_borrow(Clone::clone) else {
        return false;
    };
    let mut pane = pane.borrow_mut();
    match command {
        BUILD => {
            let green = Attr::new(TvColor::LightGreen, TvColor::Black);
            let yellow = Attr::new(TvColor::Yellow, TvColor::Black);
            pane.append_text("   Compiling tv-extensions\n   Compiling gallery");
            pane.append_line_colored("warning: unused variable `x`".to_string(), yellow);
            pane.append_line_spans(vec![
                Span::with_attr("    Finished", green),
                Span::new(" `dev` profile in 1.2s"),
            ]);
        }
        CLEAR => pane.clear(),
        _ => return false,
    }
    true
}
