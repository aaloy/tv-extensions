//! Two lists side by side for picking a subset: the left holds what is
//! available, the right what is chosen. Type in a filter field to narrow
//! the list under it, Down to move into the list. Space marks items; Enter,
//! a double click or > moves them across; >> and << move everything the
//! filter shows. Alt and a title's ~letter~ jumps to its filter.
//!
//! Parameters:
//! - `DualListBuilder::new().bounds(r)`: at least 10 rows, so the four
//!   buttons fit beside the lists.
//! - `items(iter)`: (key, label) pairs; the lists show the labels and keep
//!   their order.
//! - `data(rc)`: the chosen keys, shared as `Rc<RefCell<Vec<K>>>`; fill it
//!   first to preselect, read it after the dialog closes.
//! - `titles(available, chosen)`: the headers, with a ~hot~ key each.
//! - `min_chosen(n)`, `max_chosen(n)`, `required(true)`: bound how many
//!   can be chosen; a move past the maximum is refused, too few refuses
//!   OK. A refusal turns the chosen header into an error line, with
//!   `min_message(text)` and `max_message(text)`.
//! - `keep_chosen_order(true)`: the chosen list keeps the order items
//!   were picked in.
//! - `on_change(command)`: broadcast after every move.
//!
//! See also: ScrollPane, Popup menu

use crate::panel::Panel;
use std::cell::RefCell;
use std::rc::Rc;
use turbo_vision::core::geometry::Rect;
use tv_extensions::dual_list::DualListBuilder;

const PLANETS: [&str; 8] = [
    "Mercury", "Venus", "Earth", "Mars", "Jupiter", "Saturn", "Uranus", "Neptune",
];

pub fn build(panel: &mut Panel) {
    // Earth and Mars start out chosen; keys are 1 to 8.
    let chosen = Rc::new(RefCell::new(vec![3, 4]));
    panel.add(
        DualListBuilder::new()
            .bounds(Rect::new(0, 0, 46, 10))
            .items((1..).zip(PLANETS))
            .data(chosen)
            .titles("~P~lanets", "~V~isited")
            .max_chosen(4)
            .max_message("Four at most")
            .build(),
    );
}
