//! A text field that filters a suggestion list as you type, with the
//! matched text picked out. Down opens the list or moves the highlight, Up
//! moves it back, Enter or a click accepts a suggestion, Esc closes the
//! list and keeps the text. Matching ignores case, and any character one
//! cell wide can be typed: é, ñ, €, ł, Greek, Cyrillic.
//!
//! Parameters:
//! - `AutoCompleteBuilder::new().bounds(r)`: one row; while the list is
//!   open the view grows down to cover it, so leave room below.
//! - `items(iter)`: the suggestions, in the order the list shows them.
//! - `data(rc)`: the text, shared as `Rc<RefCell<String>>`; fill it first
//!   to start with a value, read it after the dialog closes.
//! - `min_chars(n)`: characters typed before the list opens (default 1).
//! - `max_drop_rows(n)`: rows the list shows at most (default 6).
//! - `required(true)`: the field may not be blank.
//! - `require_match(true)`: text that is not blank must be an item. A
//!   refused value keeps the dialog open and shows an error line under the
//!   field, with `required_message(text)` and `match_message(text)`.
//! - `on_select(command)`: broadcast when a suggestion is accepted.
//!
//! See also: DualList, ScrollPane

use crate::panel::Panel;
use std::cell::RefCell;
use std::rc::Rc;
use turbo_vision::core::geometry::Rect;
use turbo_vision::views::static_text::StaticText;
use tv_extensions::autocomplete::AutoCompleteBuilder;

const PLANETS: [&str; 8] = [
    "Mercury", "Venus", "Earth", "Mars", "Jupiter", "Saturn", "Uranus", "Neptune",
];

pub fn build(panel: &mut Panel) {
    panel.add(StaticText::new(Rect::new(0, 0, 8, 1), "Planet:"));
    // A planet must be given, and it must be one of the eight.
    let planet = Rc::new(RefCell::new(String::new()));
    panel.add(
        AutoCompleteBuilder::new()
            .bounds(Rect::new(8, 0, 32, 1))
            .items(PLANETS)
            .data(planet)
            .max_drop_rows(4)
            .required(true)
            .require_match(true)
            .match_message("Not a planet")
            .build(),
    );
}
