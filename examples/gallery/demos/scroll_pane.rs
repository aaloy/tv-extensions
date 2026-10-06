//! A viewport over a virtual area taller than itself: more controls than
//! fit. Tab moves through them and the pane scrolls the focused one into
//! view; Ctrl+Up and Ctrl+Down, or the mouse wheel, scroll a row at a time.
//!
//! Parameters:
//! - `ScrollPane::new(bounds, virtual_height)`: `bounds` is what shows,
//!   `virtual_height` the rows of the whole area (never less than shown).
//! - `add(Box::new(view), virtual_rect)`: place a view in the virtual
//!   area, from its top-left corner; the pane moves it as it scrolls and
//!   clips it to what shows.
//! - `scroll_to(row)`, `scroll_by(rows)`, `scroll_offset()`: scroll from
//!   your own code, and read where it is.
//! - `group()`, `group_mut()`: the views inside, to reach them again.
//!
//! See also: DualList, CSV editor

use crate::panel::Panel;
use turbo_vision::core::geometry::Rect;
use turbo_vision::views::input_line::InputLine;
use turbo_vision::views::static_text::StaticText;
use tv_extensions::scroll_pane::ScrollPane;

const FIELDS: [&str; 8] = [
    "Name", "Company", "Street", "City", "Postcode", "Country", "Phone", "Email",
];

pub fn build(panel: &mut Panel) {
    // Eight rows of fields in a pane five rows tall.
    let mut pane = ScrollPane::new(Rect::new(0, 0, 40, 5), 8);
    for (y, field) in (0..).zip(FIELDS) {
        let label = Rect::new(0, y, 10, y + 1);
        pane.add(Box::new(StaticText::new(label, field)), label);
        let input = Rect::new(10, y, 38, y + 1);
        pane.add(Box::new(InputLine::new(input, 40)), input);
    }
    panel.add(pane);
}
