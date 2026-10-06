//! A text field that filters a suggestion list as you type, the matched
//! text picked out. Down opens the list, Up and Down move through it,
//! Enter or a click takes a suggestion, Esc closes the list and keeps the
//! text. While the list is open the field grows down to cover it.
//!
//! Parameters:
//! - `AutoCompleteBuilder::new().bounds(r)`: one row; leave room under it
//!   for the list, or add the field after what the list covers.
//! - `items(iter)`: the suggestions; matching ignores case.
//! - `data(rc)`: the text, shared as `Rc<RefCell<String>>`.
//! - `min_chars(n)`: characters typed before the list opens (default 1).
//! - `max_drop_rows(n)`: rows the open list takes (default 6).
//! - `required(true)`: may not be blank. `require_match(true)`: text that
//!   is not blank must be an item. Both refuse OK with an error line under
//!   the field, `required_message(text)` and `match_message(text)`.
//! - `on_select(command)`: broadcast when a suggestion is taken.
//!
//! See also: DualList, ScrollPane

use crate::panel::Panel;
use std::cell::RefCell;
use std::rc::Rc;
use turbo_vision::core::geometry::Rect;
use turbo_vision::views::label::Label;
use tv_extensions::autocomplete::AutoCompleteBuilder;

const PLANETS: [&str; 8] = [
    "Mercury", "Venus", "Earth", "Mars", "Jupiter", "Saturn", "Uranus", "Neptune",
];

pub fn build(panel: &mut Panel) {
    let planet = Rc::new(RefCell::new(String::new()));
    // Room for four suggestions under the field.
    let field = panel.add(
        AutoCompleteBuilder::new()
            .bounds(Rect::new(9, 0, 33, 1))
            .items(PLANETS)
            .data(planet)
            .max_drop_rows(4)
            .require_match(true)
            .match_message("Not a planet")
            .build(),
    );
    let mut label = Label::new(Rect::new(0, 0, 8, 1), "~P~lanet");
    label.set_link(field);
    panel.add(label);
}
