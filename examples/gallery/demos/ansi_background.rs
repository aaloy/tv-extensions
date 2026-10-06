//! ANSI art drawn as a view: text with colour escape sequences, as from a
//! .ans file, parsed into cells. 16 colours, the 256-colour palette and
//! true colour are read. It is made for the desktop's background, behind
//! the windows, and centres its picture in its bounds. Night switches
//! between day and night.
//!
//! Parameters:
//! - `AnsiBackground::from_string(bounds, text, attr)`: `attr` colours the
//!   cells the picture does not cover; `from_file(bounds, path, attr)`
//!   reads a .ans file.
//! - `centered(on)`, `center_x(on)`, `center_y(on)`: where the picture
//!   sits when it is smaller than the view.
//! - `set_content(text)`, `load_file(path)`, `set_image(image)`: show
//!   another picture.
//! - `AnsiBackgroundBuilder`: the same, built step by step.
//!
//! See also: TerminalWidget, ScrollPane

use crate::panel::Panel;
use std::cell::RefCell;
use std::rc::Rc;
use turbo_vision::app::Application;
use turbo_vision::core::command::{CM_USER, CommandId};
use turbo_vision::core::geometry::Rect;
use turbo_vision::core::palette::{Attr, TvColor};
use turbo_vision::views::button::Button;
use turbo_vision::views::shared::Shared;
use tv_extensions::graphics::AnsiBackground;

const SWAP: CommandId = CM_USER + 100;

thread_local! {
    /// The picture, shared so the handler can change it; and which it is.
    static PICTURE: RefCell<Option<(Rc<RefCell<AnsiBackground>>, bool)>> =
        const { RefCell::new(None) };
}

/// The sky, sun or moon, and sea, in 256-colour and true-colour escapes.
fn scene(night: bool) -> String {
    let (sky, disc) = if night {
        ("\x1b[48;5;17m", "\x1b[38;2;230;230;210m\x1b[48;5;17m")
    } else {
        ("\x1b[48;5;54m", "\x1b[38;2;255;200;0m\x1b[48;5;54m")
    };
    let sea = "\x1b[48;5;24m\x1b[38;5;45m";
    let reset = "\x1b[0m";
    let (top, middle) = ("\u{2584}".repeat(4), "\u{2588}".repeat(8));
    [
        format!("{sky}{:30}{reset}", ""),
        format!("{sky}{:11}{disc} {top} {sky}{:13}{reset}", "", ""),
        format!("{sky}{:10}{disc}{middle}{sky}{:12}{reset}", "", ""),
        format!("{sea}{}{reset}", "~\u{2248}~".repeat(10)),
        format!("{sea}{}{reset}", "\u{2248}~~".repeat(10)),
    ]
    .join("\n")
}

pub fn build(panel: &mut Panel) {
    let backdrop = Attr::new(TvColor::DarkGray, TvColor::Black);
    let picture =
        AnsiBackground::from_string(Rect::new(0, 0, 32, 5), &scene(false), backdrop).centered(true);
    let picture = Rc::new(RefCell::new(picture));
    PICTURE.set(Some((Rc::clone(&picture), false)));
    panel.add(Shared::new(picture));
    panel.add(Button::new(Rect::new(34, 0, 46, 2), "~N~ight", SWAP, true));
}

pub fn handle(_app: &mut Application, command: CommandId) -> bool {
    if command != SWAP {
        return false;
    }
    PICTURE.with_borrow_mut(|shown| {
        if let Some((picture, night)) = shown {
            *night = !*night;
            picture.borrow_mut().set_content(&scene(*night));
        }
    });
    true
}
