// (C) 2026 - Antoni Aloy

//! A text field that filters a suggestion list as you type.
//!
//! [`AutoComplete`] behaves like a plain input line, but once enough
//! characters are typed a drop-down of matching suggestions opens under the
//! field, with the matched text picked out in its own colour. Up/Down (or
//! the mouse) move the highlight, Enter accepts it, and Esc closes the list
//! without changing the text. The text is shared through an
//! `Rc<RefCell<String>>`, like core's `InputLine`, so the caller can read it
//! after the dialog closes.
//!
//! While the list is open the view's bounds grow downward to cover it, so
//! the owning group draws it and routes clicks on it to this view. Add the
//! field after (on top of) any sibling the list may cover, or leave room
//! below it.
//!
//! # Keys
//!
//! | Key | Action |
//! |-----|--------|
//! | Printable characters (ASCII and Latin-1) | Type into the field, filter the list |
//! | Down | Open the list, or move the highlight down |
//! | Up | Move the highlight up |
//! | Enter | Accept the highlighted suggestion |
//! | Esc | Close the list without changing the text |
//! | Left/Right/Home/End/Backspace/Delete | Ordinary text editing |
//!
//! Clicking a suggestion accepts it, clicking the field opens the list, and
//! the list closes when the field loses focus.
//!
//! # Example
//!
//! ```
//! use std::{cell::RefCell, rc::Rc};
//! use turbo_vision::core::geometry::Rect;
//! use tv_extensions::AutoComplete;
//!
//! let text = Rc::new(RefCell::new(String::new()));
//! let fruits = ["Apple", "Apricot", "Banana", "Blueberry", "Cherry"]
//!     .into_iter()
//!     .map(String::from)
//!     .collect();
//! let auto = AutoComplete::new(Rect::new(2, 2, 32, 3), fruits, text.clone());
//! assert!(!auto.is_open());
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use turbo_vision::core::command::CommandId;
use turbo_vision::core::draw::DrawBuffer;
use turbo_vision::core::event::{
    Event, EventType, KB_BACKSPACE, KB_DEL, KB_DOWN, KB_END, KB_ENTER, KB_ESC, KB_ESC_ESC, KB_HOME,
    KB_LEFT, KB_RIGHT, KB_UP, MB_LEFT_BUTTON,
};
use turbo_vision::core::geometry::{Point, Rect};
use turbo_vision::core::palette::{Attr, Palette};
use turbo_vision::core::state::State;
use turbo_vision::terminal::Terminal;
use turbo_vision::views::view::write_line_to_terminal;
use turbo_vision::views::{View, ViewCore};

// Palette indices. Entries 1-3 are the field, as in core's CP_INPUT_LINE;
// 4-6 are the list rows: normal, highlighted, and the matched text.
const FIELD_NORMAL: u8 = 1;
const FIELD_FOCUSED: u8 = 2;
const FIELD_SELECTED: u8 = 3;
const LIST_NORMAL: u8 = 4;
const LIST_SELECTED: u8 = 5;
const LIST_MATCH: u8 = 6;

/// Palette inside a dialog: the field maps like `InputLine`, the list like
/// `ListBox` (26 normal, 27 selected, 28 the divider colour for matches).
const CP_AUTOCOMPLETE: &[u8] = &[19, 19, 20, 26, 27, 28];

/// Palette inside a window. A window's palette has 8 entries, so the dialog
/// list indices would fall through to the app palette; use the window's own
/// text pair instead (6 normal, 7 selected, 3 for matches), as core's
/// `Table` does with `CP_TABLE_WINDOW`.
const CP_AUTOCOMPLETE_WINDOW: &[u8] = &[19, 19, 20, 6, 7, 3];

/// Converts a char index into a byte offset, clamping to the end of `text`.
fn byte_offset(text: &str, char_idx: usize) -> usize {
    text.char_indices()
        .nth(char_idx)
        .map_or(text.len(), |(i, _)| i)
}

fn char_len(text: &str) -> usize {
    text.chars().count()
}

/// A count of cells as a screen coordinate, saturating.
fn coord(n: usize) -> i16 {
    i16::try_from(n).unwrap_or(i16::MAX)
}

/// A screen coordinate as a count of cells; negative becomes zero.
fn cells(n: i16) -> usize {
    usize::try_from(n).unwrap_or(0)
}

/// The character a key code types, if it is printable. Key codes carry the
/// character itself for plain keys; special keys live at 0x0E08 and up, so
/// only ASCII and the printable Latin-1 range are unambiguous.
fn typed_char(key_code: u16) -> Option<char> {
    if (0x20..0x7F).contains(&key_code) || (0xA0..=0xFF).contains(&key_code) {
        char::from_u32(u32::from(key_code))
    } else {
        None
    }
}

fn lower(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// Case-insensitive position (in chars) of `needle` inside `haystack`, if any.
fn find_match(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }
    let hay: Vec<char> = haystack.chars().map(lower).collect();
    let pat: Vec<char> = needle.chars().map(lower).collect();
    if pat.len() > hay.len() {
        return None;
    }
    hay.windows(pat.len()).position(|w| w == pat.as_slice())
}

/// A text field that filters `items` to the ones matching the typed text,
/// showing the matches in a drop-down below the field.
///
/// Give it a one-row `bounds`; the view grows downward by the list's height
/// while the list is open and shrinks back when it closes.
#[derive(Debug)]
pub struct AutoComplete {
    core: ViewCore,
    items: Vec<String>,
    text: Rc<RefCell<String>>,
    cursor_pos: usize,
    first_pos: usize,
    sel_start: usize,
    sel_end: usize,
    max_length: usize,
    /// Minimum characters typed before the drop-down appears.
    min_chars: usize,
    max_drop_rows: usize,
    open: bool,
    /// Indices into `items` that match the current text, in `items` order.
    filtered: Vec<usize>,
    /// Index into `filtered` that is highlighted.
    highlighted: usize,
    /// First visible row of `filtered`, for scrolling long lists.
    top: usize,
    /// Command broadcast when a suggestion is accepted. Zero sends none.
    on_select: CommandId,
}

impl AutoComplete {
    /// Create an autocomplete field over `items`, sharing its text in `data`.
    #[must_use]
    pub fn new(bounds: Rect, items: Vec<String>, data: Rc<RefCell<String>>) -> Self {
        let mut auto = Self {
            core: ViewCore {
                bounds,
                state: State::empty(),
                ..ViewCore::default()
            },
            items,
            text: data,
            cursor_pos: 0,
            first_pos: 0,
            sel_start: 0,
            sel_end: 0,
            max_length: 255,
            min_chars: 1,
            max_drop_rows: 6,
            open: false,
            filtered: Vec::new(),
            highlighted: 0,
            top: 0,
            on_select: 0,
        };
        auto.cursor_pos = char_len(&auto.text.borrow());
        auto.refilter();
        auto
    }

    /// Longest text the field accepts (default 255).
    pub fn set_max_length(&mut self, max_length: usize) {
        self.max_length = max_length;
    }

    /// Characters the user must type before the list appears (default 1).
    /// Zero shows the full item list as soon as the field is edited.
    pub fn set_min_chars(&mut self, min_chars: usize) {
        self.min_chars = min_chars;
    }

    /// Caps how many rows the drop-down shows before it scrolls (default 6).
    pub fn set_max_drop_rows(&mut self, rows: usize) {
        self.max_drop_rows = rows.max(1);
        self.sync_height();
    }

    /// Command broadcast when a suggestion is accepted. Zero, the default,
    /// sends none.
    pub fn set_on_select(&mut self, command: CommandId) {
        self.on_select = command;
    }

    /// Replace the suggestion list and re-filter against the current text.
    pub fn set_items(&mut self, items: Vec<String>) {
        self.items = items;
        self.refilter();
    }

    /// Current field text.
    #[must_use]
    pub fn value(&self) -> String {
        self.text.borrow().clone()
    }

    /// True while the suggestion list is showing.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.open && self.is_focused()
    }

    /// Close a list left open by a focus loss. Not done in `set_focus`
    /// itself: a group gives focus on a click by clearing every child's
    /// focus and then focusing the clicked one, before passing the click
    /// on, so closing there would drop the list under the very click that
    /// picks from it. This runs on the next event or draw instead, which
    /// still hides the list before a field that lost focus is drawn again.
    fn close_if_blurred(&mut self) {
        if self.open && !self.is_focused() {
            self.close_list();
        }
    }

    /// Recompute `filtered` from `items` against the current text, and
    /// decide whether the list should be showing. It only opens while the
    /// field has focus, so refilling an idle field does not pop the list up.
    fn refilter(&mut self) {
        let query = self.text.borrow().clone();
        self.filtered = if query.is_empty() {
            (0..self.items.len()).collect()
        } else {
            self.items
                .iter()
                .enumerate()
                .filter(|(_, item)| find_match(item, &query).is_some())
                .map(|(i, _)| i)
                .collect()
        };
        self.highlighted = 0;
        self.top = 0;
        self.open = self.is_focused() && char_len(&query) >= self.min_chars;
        self.sync_height();
    }

    fn drop_rows(&self) -> usize {
        if self.filtered.is_empty() {
            1 // the "No matches" row
        } else {
            self.filtered.len().min(self.max_drop_rows)
        }
    }

    /// Grow the bounds over the open list, or shrink them back to the field
    /// row. The owning group routes mouse events by bounds, so this is what
    /// lets a click on a suggestion reach this view.
    fn sync_height(&mut self) {
        let rows = if self.open { self.drop_rows() } else { 0 };
        let top = self.core.bounds.a.y;
        self.core.bounds.b.y = top.saturating_add(1).saturating_add(coord(rows));
    }

    /// The open drop-down, in this view's own space.
    fn drop_bounds(&self) -> Rect {
        Rect::new(0, 1, self.width_cells_i16(), 1 + coord(self.drop_rows()))
    }

    fn width(&self) -> usize {
        cells(self.core.bounds.width_clamped())
    }

    fn width_cells_i16(&self) -> i16 {
        self.core.bounds.width_clamped()
    }

    fn open_list(&mut self) {
        if !self.filtered.is_empty() {
            self.open = true;
            self.sync_height();
        }
    }

    fn close_list(&mut self) {
        self.open = false;
        self.sync_height();
    }

    /// Scroll so the highlight stays visible.
    fn scroll_into_view(&mut self) {
        let rows = self.max_drop_rows;
        if self.highlighted < self.top {
            self.top = self.highlighted;
        } else if self.highlighted >= self.top + rows {
            self.top = self.highlighted + 1 - rows;
        }
    }

    fn highlight_next(&mut self) {
        if self.highlighted + 1 < self.filtered.len() {
            self.highlighted += 1;
        }
        self.scroll_into_view();
    }

    fn highlight_prev(&mut self) {
        self.highlighted = self.highlighted.saturating_sub(1);
        self.scroll_into_view();
    }

    /// Accept the highlighted suggestion: write it into the shared text,
    /// move the cursor to the end, and close the list.
    fn commit(&mut self) {
        let chosen = self
            .filtered
            .get(self.highlighted)
            .and_then(|&i| self.items.get(i))
            .cloned();
        if let Some(chosen) = chosen {
            self.cursor_pos = char_len(&chosen);
            *self.text.borrow_mut() = chosen;
            self.sel_start = 0;
            self.sel_end = 0;
            self.first_pos = 0;
            self.make_cursor_visible();
        }
        self.close_list();
    }

    /// The event a commit leaves behind: the `on_select` broadcast, or a
    /// cleared event when no command is set.
    fn finish_commit(&self, event: &mut Event) {
        if self.on_select == 0 {
            event.clear();
        } else {
            *event = Event::broadcast_with_info(self.on_select, 0);
        }
    }

    fn has_selection(&self) -> bool {
        self.sel_start != self.sel_end
    }

    fn clear_selection(&mut self) {
        self.sel_start = 0;
        self.sel_end = 0;
    }

    fn select_all(&mut self) {
        let len = char_len(&self.text.borrow());
        self.sel_start = 0;
        self.sel_end = len;
        self.cursor_pos = len;
    }

    fn delete_selection(&mut self) {
        if !self.has_selection() {
            return;
        }
        let start = self.sel_start.min(self.sel_end);
        let end = self.sel_start.max(self.sel_end);
        {
            let mut text = self.text.borrow_mut();
            let byte_start = byte_offset(&text, start);
            let byte_end = byte_offset(&text, end);
            text.replace_range(byte_start..byte_end, "");
        }
        self.cursor_pos = start;
        self.clear_selection();
    }

    fn make_cursor_visible(&mut self) {
        let width = self.width();
        if self.cursor_pos < self.first_pos {
            self.first_pos = self.cursor_pos;
        } else if width > 0 && self.cursor_pos >= self.first_pos + width {
            self.first_pos = self.cursor_pos - width + 1;
        }
    }

    /// Index into `filtered` under a point in own coordinates, if the point
    /// is on a visible suggestion row.
    fn item_at(&self, pos: Point) -> Option<usize> {
        let drop = self.drop_bounds();
        if !drop.contains(pos) || self.filtered.is_empty() {
            return None;
        }
        let idx = self.top + cells(pos.y - drop.a.y);
        (idx < self.filtered.len()).then_some(idx)
    }

    /// The palette for the kind of owner this view sits in: the dialog one,
    /// or the window one when the owner's palette is too short for it.
    fn palette_slice(&self) -> &'static [u8] {
        let needed = usize::from(CP_AUTOCOMPLETE.iter().copied().max().unwrap_or(0));
        match self
            .core
            .palette_chain
            .as_ref()
            .and_then(turbo_vision::core::palette_chain::PaletteChainNode::nearest_palette_len)
        {
            Some(len) if len < needed => CP_AUTOCOMPLETE_WINDOW,
            _ => CP_AUTOCOMPLETE,
        }
    }

    fn draw_field(&self, terminal: &mut Terminal, text: &str) {
        let width = self.width();
        let field_attr = self.map_color(if self.is_focused() {
            FIELD_FOCUSED
        } else {
            FIELD_NORMAL
        });
        let sel_attr = self.map_color(FIELD_SELECTED);

        let mut buf = DrawBuffer::new(width);
        buf.move_char(0, ' ', field_attr, width);
        let (sel_lo, sel_hi) = (
            self.sel_start.min(self.sel_end),
            self.sel_start.max(self.sel_end),
        );
        for (i, ch) in text.chars().skip(self.first_pos).take(width).enumerate() {
            let pos = self.first_pos + i;
            let attr = if (sel_lo..sel_hi).contains(&pos) {
                sel_attr
            } else {
                field_attr
            };
            buf.move_char(i, ch, attr, 1);
        }
        write_line_to_terminal(terminal, 0, 0, &buf);
    }

    fn draw_list(&self, terminal: &mut Terminal, query: &str) {
        let width = self.width();
        let normal_attr = self.map_color(LIST_NORMAL);
        let selected_attr = self.map_color(LIST_SELECTED);
        let match_attr = self.map_color(LIST_MATCH);
        // On the highlighted row keep its background and only take the
        // match colour's foreground, so the row still reads as selected.
        let selected_match_attr = Attr {
            fg: match_attr.fg,
            ..selected_attr
        };

        if self.filtered.is_empty() {
            let mut row = DrawBuffer::new(width);
            row.move_char(0, ' ', normal_attr, width);
            row.move_str(0, "No matches", normal_attr);
            write_line_to_terminal(terminal, 0, 1, &row);
            return;
        }

        let match_len = char_len(query);
        for row in 0..self.drop_rows() {
            let filtered_idx = self.top + row;
            let Some(item) = self
                .filtered
                .get(filtered_idx)
                .and_then(|&i| self.items.get(i))
            else {
                break;
            };
            let (row_attr, hl_attr) = if filtered_idx == self.highlighted {
                (selected_attr, selected_match_attr)
            } else {
                (normal_attr, match_attr)
            };

            let mut buf = DrawBuffer::new(width);
            buf.move_char(0, ' ', row_attr, width);
            let match_start = find_match(item, query);
            for (i, ch) in item.chars().take(width).enumerate() {
                let attr = match match_start {
                    Some(start) if (start..start + match_len).contains(&i) => hl_attr,
                    _ => row_attr,
                };
                buf.move_char(i, ch, attr, 1);
            }
            write_line_to_terminal(terminal, 0, 1 + coord(row), &buf);
        }
    }

    fn handle_mouse_down(&mut self, event: &mut Event) {
        if event.mouse.buttons & MB_LEFT_BUTTON == 0 {
            return;
        }
        let pos = event.mouse.pos;
        if self.open
            && let Some(idx) = self.item_at(pos)
        {
            self.highlighted = idx;
            self.commit();
            self.finish_commit(event);
            return;
        }
        let on_field = pos.y == 0 && (0..self.width_cells_i16()).contains(&pos.x);
        if on_field {
            if !self.open {
                self.open_list();
            }
            event.clear();
        } else if self.open {
            // A click below the visible rows of the open list.
            self.close_list();
            event.clear();
        }
    }

    /// Insert `ch` at the cursor, replacing any selection.
    fn insert_char(&mut self, ch: char) -> bool {
        if self.has_selection() {
            self.delete_selection();
        }
        if char_len(&self.text.borrow()) >= self.max_length {
            return false;
        }
        {
            let mut text = self.text.borrow_mut();
            let at = byte_offset(&text, self.cursor_pos);
            text.insert(at, ch);
        }
        self.cursor_pos += 1;
        true
    }

    /// Text-editing keys. Returns true when the key was consumed.
    fn handle_edit_key(&mut self, key_code: u16) -> bool {
        match key_code {
            KB_BACKSPACE => {
                if self.has_selection() {
                    self.delete_selection();
                } else if self.cursor_pos > 0 {
                    let mut text = self.text.borrow_mut();
                    let at = byte_offset(&text, self.cursor_pos - 1);
                    text.remove(at);
                    drop(text);
                    self.cursor_pos -= 1;
                } else {
                    return false;
                }
                self.make_cursor_visible();
                self.refilter();
            }
            KB_DEL => {
                if self.has_selection() {
                    self.delete_selection();
                } else if self.cursor_pos < char_len(&self.text.borrow()) {
                    let mut text = self.text.borrow_mut();
                    let at = byte_offset(&text, self.cursor_pos);
                    text.remove(at);
                } else {
                    return false;
                }
                self.make_cursor_visible();
                self.refilter();
            }
            KB_LEFT | KB_RIGHT | KB_HOME | KB_END => {
                let len = char_len(&self.text.borrow());
                self.cursor_pos = match key_code {
                    KB_LEFT => self.cursor_pos.saturating_sub(1),
                    KB_RIGHT => (self.cursor_pos + 1).min(len),
                    KB_HOME => 0,
                    _ => len,
                };
                self.clear_selection();
                self.make_cursor_visible();
            }
            _ => {
                let Some(ch) = typed_char(key_code) else {
                    return false;
                };
                if !self.insert_char(ch) {
                    return false;
                }
                self.make_cursor_visible();
                self.refilter();
            }
        }
        true
    }

    fn handle_key(&mut self, event: &mut Event) {
        match event.key_code {
            KB_DOWN => {
                if self.open {
                    self.highlight_next();
                } else {
                    self.open_list();
                }
                event.clear();
            }
            KB_UP if self.open => {
                self.highlight_prev();
                event.clear();
            }
            // With the list closed, Enter falls through to the dialog's
            // default button.
            KB_ENTER if self.open => {
                self.commit();
                self.finish_commit(event);
            }
            // With the list closed, Esc falls through (Esc-Esc closes the
            // dialog).
            KB_ESC | KB_ESC_ESC if self.open => {
                self.close_list();
                event.clear();
            }
            key_code => {
                if self.handle_edit_key(key_code) {
                    event.clear();
                }
            }
        }
    }
}

impl View for AutoComplete {
    fn core(&self) -> &ViewCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut ViewCore {
        &mut self.core
    }

    fn can_focus(&self) -> bool {
        true
    }

    fn set_focus(&mut self, focused: bool) {
        let was_focused = self.is_focused();
        self.set_state_flag(State::FOCUSED, focused);
        if focused && !was_focused {
            self.select_all();
        } else if !focused {
            self.clear_selection();
        }
    }

    fn draw(&mut self, terminal: &mut Terminal) {
        self.close_if_blurred();
        if self.width() == 0 {
            return;
        }
        let text = self.text.borrow().clone();
        self.draw_field(terminal, &text);
        if self.open {
            self.draw_list(terminal, &text);
        }
    }

    fn handle_event(&mut self, event: &mut Event) {
        self.close_if_blurred();
        match event.what {
            EventType::MouseDown => self.handle_mouse_down(event),
            EventType::Keyboard if self.is_focused() => self.handle_key(event),
            _ => {}
        }
    }

    fn update_cursor(&self, terminal: &mut Terminal) {
        if self.is_focused() {
            let x = coord(self.cursor_pos.saturating_sub(self.first_pos));
            let _ = terminal.show_cursor(x, 0);
        } else {
            let _ = terminal.hide_cursor();
        }
    }

    fn get_palette(&self) -> Option<Palette> {
        Some(Palette::from_slice(self.palette_slice()))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Builder for [`AutoComplete`] with a fluent API.
///
/// ```
/// use turbo_vision::core::geometry::Rect;
/// use tv_extensions::AutoCompleteBuilder;
///
/// let auto = AutoCompleteBuilder::new()
///     .bounds(Rect::new(2, 2, 32, 3))
///     .items(["Red", "Green", "Blue"])
///     .min_chars(2)
///     .build();
/// assert_eq!(auto.value(), "");
/// ```
#[derive(Debug)]
pub struct AutoCompleteBuilder {
    bounds: Option<Rect>,
    items: Vec<String>,
    data: Option<Rc<RefCell<String>>>,
    max_length: usize,
    min_chars: usize,
    max_drop_rows: usize,
    on_select: CommandId,
}

impl AutoCompleteBuilder {
    /// A builder with the [`AutoComplete`] defaults and no bounds yet.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bounds: None,
            items: Vec::new(),
            data: None,
            max_length: 255,
            min_chars: 1,
            max_drop_rows: 6,
            on_select: 0,
        }
    }

    /// The field's one-row bounds (required).
    #[must_use]
    pub fn bounds(mut self, bounds: Rect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    /// The suggestions to filter.
    #[must_use]
    pub fn items<I: Into<String>>(mut self, items: impl IntoIterator<Item = I>) -> Self {
        self.items = items.into_iter().map(Into::into).collect();
        self
    }

    /// The shared text; a fresh empty one when not set.
    #[must_use]
    pub fn data(mut self, data: Rc<RefCell<String>>) -> Self {
        self.data = Some(data);
        self
    }

    /// See [`AutoComplete::set_max_length`].
    #[must_use]
    pub fn max_length(mut self, max_length: usize) -> Self {
        self.max_length = max_length;
        self
    }

    /// See [`AutoComplete::set_min_chars`].
    #[must_use]
    pub fn min_chars(mut self, min_chars: usize) -> Self {
        self.min_chars = min_chars;
        self
    }

    /// See [`AutoComplete::set_max_drop_rows`].
    #[must_use]
    pub fn max_drop_rows(mut self, rows: usize) -> Self {
        self.max_drop_rows = rows;
        self
    }

    /// See [`AutoComplete::set_on_select`].
    #[must_use]
    pub fn on_select(mut self, command: CommandId) -> Self {
        self.on_select = command;
        self
    }

    /// Build the field.
    ///
    /// # Panics
    ///
    /// Panics if `bounds` was not set.
    #[must_use]
    pub fn build(self) -> AutoComplete {
        let bounds = self.bounds.expect("AutoComplete bounds must be set");
        let data = self
            .data
            .unwrap_or_else(|| Rc::new(RefCell::new(String::new())));
        let mut auto = AutoComplete::new(bounds, self.items, data);
        auto.set_max_length(self.max_length);
        auto.set_min_chars(self.min_chars);
        auto.set_max_drop_rows(self.max_drop_rows);
        auto.set_on_select(self.on_select);
        auto
    }

    /// Build the field in a `Box`.
    ///
    /// # Panics
    ///
    /// Panics if `bounds` was not set.
    #[must_use]
    pub fn build_boxed(self) -> Box<AutoComplete> {
        Box::new(self.build())
    }
}

impl Default for AutoCompleteBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use turbo_vision::views::GroupLike;
    use turbo_vision::views::group::Group;

    fn fruits() -> Vec<String> {
        ["Apple", "Apricot", "Banana", "Blueberry", "Cherry"]
            .into_iter()
            .map(String::from)
            .collect()
    }

    fn make() -> (AutoComplete, Rc<RefCell<String>>) {
        let data = Rc::new(RefCell::new(String::new()));
        let mut auto = AutoComplete::new(Rect::new(0, 0, 20, 1), fruits(), data.clone());
        auto.set_focus(true);
        (auto, data)
    }

    fn key(code: u16) -> Event {
        Event::keyboard(code)
    }

    fn type_str(view: &mut dyn View, s: &str) {
        for ch in s.chars() {
            let mut ev = key(u16::try_from(u32::from(ch)).unwrap());
            view.handle_event(&mut ev);
        }
    }

    fn shown(auto: &AutoComplete) -> Vec<&str> {
        auto.filtered
            .iter()
            .map(|&i| auto.items[i].as_str())
            .collect()
    }

    #[test]
    fn typing_filters_the_list_case_insensitively() {
        let (mut auto, _) = make();
        type_str(&mut auto, "AP");
        assert!(auto.is_open());
        assert_eq!(shown(&auto), vec!["Apple", "Apricot"]);
    }

    #[test]
    fn no_matches_keeps_the_list_open_with_a_message() {
        let (mut auto, _) = make();
        type_str(&mut auto, "zz");
        assert!(auto.is_open());
        assert!(auto.filtered.is_empty());
    }

    #[test]
    fn enter_accepts_the_highlighted_suggestion() {
        let (mut auto, data) = make();
        type_str(&mut auto, "ban");
        let mut ev = key(KB_ENTER);
        auto.handle_event(&mut ev);
        assert!(!auto.is_open());
        assert_eq!(*data.borrow(), "Banana");
        assert_eq!(ev.what, EventType::Nothing);
    }

    #[test]
    fn enter_with_the_list_closed_is_left_for_the_dialog() {
        let (mut auto, _) = make();
        let mut ev = key(KB_ENTER);
        auto.handle_event(&mut ev);
        assert_eq!(ev.what, EventType::Keyboard);
    }

    #[test]
    fn down_moves_the_highlight_then_enter_accepts_it() {
        let (mut auto, data) = make();
        type_str(&mut auto, "a"); // Apple, Apricot, Banana all contain "a"
        let mut ev = key(KB_DOWN);
        auto.handle_event(&mut ev);
        let mut ev = key(KB_ENTER);
        auto.handle_event(&mut ev);
        assert_eq!(*data.borrow(), "Apricot");
    }

    #[test]
    fn escape_closes_without_changing_the_text() {
        let (mut auto, data) = make();
        type_str(&mut auto, "ap");
        let mut ev = key(KB_ESC);
        auto.handle_event(&mut ev);
        assert!(!auto.is_open());
        assert_eq!(*data.borrow(), "ap");
    }

    #[test]
    fn backspace_refilters_and_can_close_the_list() {
        let (mut auto, _) = make();
        type_str(&mut auto, "a");
        assert!(auto.is_open());
        let mut ev = key(KB_BACKSPACE);
        auto.handle_event(&mut ev);
        assert!(!auto.is_open(), "below min_chars again");
        assert_eq!(*auto.text.borrow(), "");
    }

    #[test]
    fn latin1_characters_are_typed_and_matched_case_insensitively() {
        let data = Rc::new(RefCell::new(String::new()));
        let items = ["Málaga", "Mallorca", "Ñandú"].map(String::from).to_vec();
        let mut auto = AutoComplete::new(Rect::new(0, 0, 20, 1), items, data.clone());
        auto.set_focus(true);
        type_str(&mut auto, "ñ");
        assert_eq!(*data.borrow(), "ñ");
        assert_eq!(shown(&auto), vec!["Ñandú"]);
    }

    #[test]
    fn special_keys_are_not_typed() {
        let (mut auto, data) = make();
        for code in [0x0E08, 0x2D00, 0x011B, 0x7F, 0x9F] {
            let mut ev = key(code);
            auto.handle_event(&mut ev);
        }
        assert_eq!(*data.borrow(), "");
    }

    #[test]
    fn mouse_click_on_a_suggestion_accepts_it() {
        let (mut auto, data) = make();
        type_str(&mut auto, "b");
        let mut ev = Event::mouse(
            EventType::MouseDown,
            Point::new(2, 1),
            MB_LEFT_BUTTON,
            false,
        );
        auto.handle_event(&mut ev);
        assert_eq!(*data.borrow(), "Banana");
    }

    #[test]
    fn bounds_cover_the_open_list_and_shrink_when_it_closes() {
        let (mut auto, _) = make();
        assert_eq!(auto.bounds().height(), 1);
        type_str(&mut auto, "a"); // three matches
        assert_eq!(auto.bounds().height(), 4);
        type_str(&mut auto, "pr"); // "apr": Apricot only
        assert_eq!(auto.bounds().height(), 2);
        let mut ev = key(KB_ESC);
        auto.handle_event(&mut ev);
        assert_eq!(auto.bounds().height(), 1);
    }

    #[test]
    fn a_click_on_a_suggestion_reaches_the_field_through_its_group() {
        // The group routes clicks by child bounds, so this only works because
        // the open list is inside the field's bounds.
        let mut group = Group::new(Rect::new(0, 0, 40, 12));
        let data = Rc::new(RefCell::new(String::new()));
        group.add(AutoComplete::new(
            Rect::new(5, 2, 25, 3),
            fruits(),
            data.clone(),
        ));
        group.set_initial_focus();
        type_str(&mut group, "b"); // Banana, Blueberry
        // Second suggestion: row 2 of the list, below the field at y = 2.
        let mut ev = Event::mouse(
            EventType::MouseDown,
            Point::new(7, 4),
            MB_LEFT_BUTTON,
            false,
        );
        group.handle_event(&mut ev);
        assert_eq!(*data.borrow(), "Blueberry");
    }

    #[test]
    fn click_below_the_visible_rows_closes_the_list() {
        let (mut auto, _) = make();
        type_str(&mut auto, "a");
        assert!(auto.is_open());
        let mut ev = Event::mouse(
            EventType::MouseDown,
            Point::new(5, 9),
            MB_LEFT_BUTTON,
            false,
        );
        auto.handle_event(&mut ev);
        assert!(!auto.is_open());
    }

    #[test]
    fn losing_focus_closes_the_list() {
        let (mut auto, _) = make();
        type_str(&mut auto, "a");
        auto.set_focus(false);
        assert!(!auto.is_open());
        let mut ev = Event::broadcast(1);
        auto.handle_event(&mut ev);
        assert_eq!(auto.bounds().height(), 1);
    }

    #[test]
    fn set_items_does_not_open_the_list_on_an_unfocused_field() {
        let data = Rc::new(RefCell::new("Ch".to_string()));
        let mut auto = AutoComplete::new(Rect::new(0, 0, 20, 1), Vec::new(), data);
        auto.set_items(fruits());
        assert!(!auto.is_open());
        assert_eq!(shown(&auto), vec!["Cherry"]);
    }

    #[test]
    fn gaining_focus_selects_all_so_typing_replaces() {
        let data = Rc::new(RefCell::new("Cherry".to_string()));
        let mut auto = AutoComplete::new(Rect::new(0, 0, 20, 1), fruits(), data.clone());
        auto.set_focus(true);
        assert!(auto.has_selection());
        type_str(&mut auto, "x");
        assert_eq!(*data.borrow(), "x");
    }

    #[test]
    fn min_chars_zero_shows_every_item_immediately() {
        let (mut auto, _) = make();
        auto.set_min_chars(0);
        auto.refilter();
        assert!(auto.is_open());
        assert_eq!(auto.filtered.len(), fruits().len());
    }

    #[test]
    fn on_select_command_is_broadcast_when_a_suggestion_is_accepted() {
        let (mut auto, _) = make();
        auto.set_on_select(777);
        type_str(&mut auto, "ch");
        let mut ev = key(KB_ENTER);
        auto.handle_event(&mut ev);
        assert_eq!(ev.what, EventType::Broadcast);
        assert_eq!(ev.command, 777);
    }
}
