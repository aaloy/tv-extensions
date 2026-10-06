// (C) 2026 - Antoni Aloy

//! Two lists side by side for picking a subset of items.
//!
//! [`DualList`] works like Django admin's `filter_horizontal`
//! widget. The left list holds the available items and the right list the
//! chosen ones. Each list has a filter field above it ("Filter:", or a
//! label of your own), and the buttons
//! between the lists move items across. The view is built from core
//! controls (two `InputLine`s, two multi-select `ListBox`es and four
//! `Button`s) in a group of its own, as `ScrollPane` is.
//!
//! Each item is a key and a label. The lists show the labels, and the keys
//! of the chosen items are shared through an `Rc<RefCell<Vec<K>>>`, the
//! way core's `InputLine` shares its text: fill it before the dialog runs
//! to preselect items, read it after the dialog closes. It is kept up to
//! date on every move.
//!
//! ```text
//!  Available (4)                 Chosen (1)
//!  Filter: [        ]            Filter: [        ]
//!  ┌────────────────┐   [ > ]    ┌────────────────┐
//!  │   Apple        │   [>> ]    │   Banana       │
//!  │ ■ Cherry       │   [ < ]    │                │
//!  │   ...          │   [<< ]    │                │
//! ```
//!
//! # Keys and mouse
//!
//! | Input | Action |
//! |-------|--------|
//! | Typing in a filter field | Show only the items whose label contains the text, ignoring case |
//! | Down in a filter field | Move to the list under it |
//! | Alt+letter marked in a title (`"~T~oppings"`) | Move to that list's filter field, while the focus is in the view |
//! | Click on a filter label | Move to its filter field |
//! | Space in a list | Mark or unmark the focused item |
//! | Shift+click in a list | Mark the run from the last clicked item |
//! | Enter or double-click in a list | Move the marked items to the other list, or the focused one if none is marked |
//! | `>` / `<` | Move the marked (or focused) items right / left |
//! | `>>` / `<<` | Move every item the filter shows right / left |
//! | Tab / Shift+Tab | Step through the parts, then on to the dialog's next control |
//!
//! A title hotkey cannot pull the focus into the view from another control:
//! a view does not know the id its dialog gave it. For that, add an
//! ordinary `Label` to the dialog linked to the view; the first time in,
//! the focus lands on the left filter.
//!
//! Moving all respects the filter, as in Django: with "an" typed above the
//! left list, `>>` chooses only the items containing "an".
//!
//! The available list always keeps the items' own order. The chosen list
//! does too, unless [`DualListBuilder::keep_chosen_order`] is set, in which
//! case it keeps the order the items were chosen in (for example, to let
//! the user pick columns in the order they should appear).
//!
//! # Validation
//!
//! [`DualListBuilder::min_chosen`] and [`DualListBuilder::max_chosen`]
//! (both off by default; [`DualListBuilder::required`] is `min_chosen(1)`)
//! bound how many items can be chosen. A move that would choose more than
//! the maximum is refused; fewer than the minimum is checked by `valid()`,
//! so OK (or closing a modeless window) is refused. Either way the header
//! of the chosen list turns into an error line in Borland's error colours
//! until the next move. Cancel always closes.
//!
//! # Example
//!
//! ```
//! use std::{cell::RefCell, rc::Rc};
//! use turbo_vision::core::command::CM_OK;
//! use turbo_vision::core::geometry::Rect;
//! use turbo_vision::views::View;
//! use tv_extensions::DualListBuilder;
//!
//! // Banana starts out chosen.
//! let chosen = Rc::new(RefCell::new(vec![2]));
//! let mut list = DualListBuilder::new()
//!     .bounds(Rect::new(2, 2, 50, 14))
//!     .items([(1, "Apple"), (2, "Banana"), (3, "Cherry")])
//!     .data(chosen.clone())
//!     .max_chosen(1)
//!     .build();
//! assert_eq!(list.chosen_keys(), vec![2]);
//!
//! // Two chosen items are one too many for OK.
//! list.set_chosen(&[1, 2]);
//! assert_eq!(*chosen.borrow(), vec![1, 2]);
//! assert!(!list.valid(CM_OK));
//! ```

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use turbo_vision::core::command::{CM_CANCEL, CommandId};
use turbo_vision::core::draw::DrawBuffer;
use turbo_vision::core::event::{
    Event, EventType, KB_ALT_A, KB_ALT_B, KB_ALT_C, KB_ALT_D, KB_ALT_E, KB_ALT_F, KB_ALT_G,
    KB_ALT_H, KB_ALT_I, KB_ALT_J, KB_ALT_K, KB_ALT_L, KB_ALT_M, KB_ALT_N, KB_ALT_O, KB_ALT_P,
    KB_ALT_Q, KB_ALT_R, KB_ALT_S, KB_ALT_T, KB_ALT_U, KB_ALT_V, KB_ALT_W, KB_ALT_X, KB_ALT_Y,
    KB_ALT_Z, KB_DOWN, KB_SHIFT_TAB, KB_TAB, MB_LEFT_BUTTON,
};
use turbo_vision::core::geometry::Rect;
use turbo_vision::core::palette::{Attr, Palette};
use turbo_vision::core::palette_chain::PaletteChainNode;
use turbo_vision::core::state::State;
use turbo_vision::terminal::Terminal;
use turbo_vision::views::button::Button;
use turbo_vision::views::group::Group;
use turbo_vision::views::input_line::InputLine;
use turbo_vision::views::listbox::ListBox;
use turbo_vision::views::view::write_line_to_terminal;
use turbo_vision::views::{GroupLike, View, ViewCore};

// The children, in the order they are added to the group, which is also
// the Tab order.
const AVAILABLE_FILTER: usize = 0;
const AVAILABLE_LIST: usize = 1;
// 2-5: the arrow buttons, `>`, `>>`, `<`, `<<`.
const CHOSEN_FILTER: usize = 6;
const CHOSEN_LIST: usize = 7;

// Commands the children send to the view. They never leave it: the view
// acts on them and clears the event (or turns it into `on_change`), and a
// command that reaches the view from outside is not taken for one of them.
const CMD_ADD: CommandId = 0xFD00;
const CMD_ADD_ALL: CommandId = 0xFD01;
const CMD_REMOVE: CommandId = 0xFD02;
const CMD_REMOVE_ALL: CommandId = 0xFD03;
/// Enter or double-click in the available list.
const CMD_PICK_AVAILABLE: CommandId = 0xFD04;
/// Enter or double-click in the chosen list.
const CMD_PICK_CHOSEN: CommandId = 0xFD05;

/// Width of the arrow buttons, shadow included.
const BUTTON_WIDTH: i16 = 6;
/// Rows the four arrow buttons take: one each plus a shadow row.
const BUTTON_ROWS: i16 = 8;
/// Longest text a filter field takes.
const FILTER_MAX_LENGTH: usize = 64;

// Dialog palette entries for the headers and filter labels, the ones core's
// `Label` maps to: normal text, the highlight while the control it names is
// focused, and the hotkey letter.
const HEADER_NORMAL: u8 = 7;
const HEADER_FOCUSED: u8 = 8;
const HEADER_SHORTCUT: u8 = 9;

/// The text in front of each filter field unless
/// [`DualListBuilder::filter_label`] says otherwise.
const DEFAULT_FILTER_LABEL: &str = "Filter:";

/// Alt+A to Alt+Z, the keys a `~X~` hotkey in a title answers to.
const ALT_LETTERS: [u16; 26] = [
    KB_ALT_A, KB_ALT_B, KB_ALT_C, KB_ALT_D, KB_ALT_E, KB_ALT_F, KB_ALT_G, KB_ALT_H, KB_ALT_I,
    KB_ALT_J, KB_ALT_K, KB_ALT_L, KB_ALT_M, KB_ALT_N, KB_ALT_O, KB_ALT_P, KB_ALT_Q, KB_ALT_R,
    KB_ALT_S, KB_ALT_T, KB_ALT_U, KB_ALT_V, KB_ALT_W, KB_ALT_X, KB_ALT_Y, KB_ALT_Z,
];

/// The Alt key for the `~X~` hotkey in `title`, if it has one (a letter).
fn hotkey(title: &str) -> Option<u16> {
    let (_, rest) = title.split_once('~')?;
    let letter = rest.chars().next()?.to_ascii_uppercase();
    letter
        .is_ascii_uppercase()
        .then(|| ALT_LETTERS[usize::from(letter as u8 - b'A')])
}

/// Borland's `errorAttr` (white on light red); the error line is drawn in it.
const ERROR_ATTR: u8 = 0xCF;

/// One of the two lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Available,
    Chosen,
}

impl Side {
    fn index(self) -> usize {
        match self {
            Side::Available => 0,
            Side::Chosen => 1,
        }
    }

    fn filter_child(self) -> usize {
        match self {
            Side::Available => AVAILABLE_FILTER,
            Side::Chosen => CHOSEN_FILTER,
        }
    }

    fn list_child(self) -> usize {
        match self {
            Side::Available => AVAILABLE_LIST,
            Side::Chosen => CHOSEN_LIST,
        }
    }
}

/// Why `valid()` refused the selection; picks the error line's message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Refusal {
    TooFew,
    TooMany,
}

/// Where the children go for a view of a given size, in the view's own
/// coordinates.
struct Layout {
    side_width: i16,
    /// Columns the filter label takes in front of each filter field.
    label_width: i16,
    chosen_x: i16,
    button_x: i16,
    button_y: i16,
    height: i16,
}

impl Layout {
    fn new(width: i16, height: i16, filter_label: &str) -> Self {
        // A column on each side of the buttons keeps them off the lists
        let side_width = ((width - BUTTON_WIDTH - 2) / 2).max(1);
        let chosen_x = width - side_width;
        // The label and a space, leaving the field at least a few columns
        let label_chars = i16::try_from(filter_label.chars().count()).unwrap_or(i16::MAX);
        let label_width = if label_chars == 0 {
            0
        } else {
            (label_chars + 1).min(side_width - 4).max(0)
        };
        Self {
            side_width,
            label_width,
            chosen_x,
            button_x: side_width + (chosen_x - side_width - BUTTON_WIDTH) / 2,
            // Centre the buttons beside the lists, which start on row 2
            button_y: 2 + ((height - 2 - BUTTON_ROWS) / 2).max(0),
            height,
        }
    }

    fn x(&self, side: Side) -> i16 {
        match side {
            Side::Available => 0,
            Side::Chosen => self.chosen_x,
        }
    }

    fn filter_label(&self, side: Side) -> Rect {
        let x = self.x(side);
        Rect::new(x, 1, x + self.label_width, 2)
    }

    fn filter(&self, side: Side) -> Rect {
        let x = self.x(side);
        Rect::new(x + self.label_width, 1, x + self.side_width, 2)
    }

    fn list(&self, side: Side) -> Rect {
        let x = self.x(side);
        Rect::new(x, 2, x + self.side_width, self.height.max(3))
    }

    /// The `n`th arrow button, top to bottom.
    fn button(&self, n: i16) -> Rect {
        let y = self.button_y + 2 * n;
        Rect::new(self.button_x, y, self.button_x + BUTTON_WIDTH, y + 2)
    }
}

/// Two lists, available and chosen, with filters and buttons to move items
/// between them.
///
/// `K` is the key the caller knows each item by; the lists show the
/// labels. See the [module documentation](self) for keys, ordering and
/// validation.
pub struct DualList<K> {
    core: ViewCore,
    group: Group,
    items: Vec<(K, String)>,
    /// Indices into `items` of the chosen ones, in the chosen list's order.
    chosen: Vec<usize>,
    /// Indices into `items` each list shows, row by row.
    shown: [Vec<usize>; 2],
    /// The filter text each list was last filtered with.
    filters: [String; 2],
    data: Rc<RefCell<Vec<K>>>,
    titles: [String; 2],
    filter_label: String,
    keep_chosen_order: bool,
    min_chosen: usize,
    max_chosen: Option<usize>,
    min_message: Option<String>,
    max_message: Option<String>,
    /// Command broadcast after every move. Zero sends none.
    on_change: CommandId,
    /// Set while `valid()` has refused the selection: the error line shows
    /// until the next move.
    refused: Option<Refusal>,
    /// The child that had the focus when the view lost it, so coming back
    /// lands on it again (Borland's `TGroup` keeps `current` the same way).
    last_focus: Option<usize>,
    palette_chain: Option<PaletteChainNode>,
}

impl<K> fmt::Debug for DualList<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DualList")
            .field("bounds", &self.core.bounds)
            .field("items", &self.items.len())
            .field("chosen", &self.chosen)
            .field("filters", &self.filters)
            .finish_non_exhaustive()
    }
}

impl<K: Clone + PartialEq + 'static> DualList<K> {
    /// Create the view over `items`, sharing the chosen keys in `data`.
    ///
    /// The keys already in `data` that belong to an item start out chosen;
    /// the rest are dropped. Give it at least 10 rows so the four buttons
    /// fit beside the lists.
    #[must_use]
    pub fn new(bounds: Rect, items: Vec<(K, String)>, data: Rc<RefCell<Vec<K>>>) -> Self {
        let layout = Layout::new(bounds.width(), bounds.height(), DEFAULT_FILTER_LABEL);
        let mut group = Group::new(Rect::new(0, 0, bounds.width(), bounds.height()));
        for side in [Side::Available, Side::Chosen] {
            if side == Side::Chosen {
                for (n, (title, command)) in (0..).zip([
                    (">", CMD_ADD),
                    (">>", CMD_ADD_ALL),
                    ("<", CMD_REMOVE),
                    ("<<", CMD_REMOVE_ALL),
                ]) {
                    group.add(Button::new(layout.button(n), title, command, false));
                }
            }
            group.add(InputLine::new(layout.filter(side), FILTER_MAX_LENGTH));
            let command = match side {
                Side::Available => CMD_PICK_AVAILABLE,
                Side::Chosen => CMD_PICK_CHOSEN,
            };
            let mut list = ListBox::new(layout.list(side), command);
            list.set_multi_select(true);
            group.add(list);
        }

        let mut view = Self {
            core: ViewCore {
                bounds,
                state: State::empty(),
                ..ViewCore::default()
            },
            group,
            items: Vec::new(),
            chosen: Vec::new(),
            shown: [Vec::new(), Vec::new()],
            filters: [String::new(), String::new()],
            data,
            titles: ["Available".to_string(), "Chosen".to_string()],
            filter_label: DEFAULT_FILTER_LABEL.to_string(),
            keep_chosen_order: false,
            min_chosen: 0,
            max_chosen: None,
            min_message: None,
            max_message: None,
            on_change: 0,
            refused: None,
            last_focus: None,
            palette_chain: None,
        };
        let keys = view.data.borrow().clone();
        view.items = items;
        view.set_chosen(&keys);
        view
    }

    /// Replace the items. Chosen keys that still belong to an item stay
    /// chosen.
    pub fn set_items(&mut self, items: Vec<(K, String)>) {
        let keys = self.chosen_keys();
        self.items = items;
        self.set_chosen(&keys);
    }

    /// Choose exactly the items with these keys, in this order when
    /// [`set_keep_chosen_order`](Self::set_keep_chosen_order) is on. Keys
    /// that belong to no item are ignored.
    pub fn set_chosen(&mut self, keys: &[K]) {
        self.chosen.clear();
        for key in keys {
            if let Some(i) = self.items.iter().position(|(k, _)| k == key)
                && !self.chosen.contains(&i)
            {
                self.chosen.push(i);
            }
        }
        if !self.keep_chosen_order {
            self.chosen.sort_unstable();
        }
        self.changed();
    }

    /// The keys of the chosen items, in the chosen list's order.
    #[must_use]
    pub fn chosen_keys(&self) -> Vec<K> {
        self.chosen
            .iter()
            .map(|&i| self.items[i].0.clone())
            .collect()
    }

    /// How many items are chosen.
    #[must_use]
    pub fn chosen_count(&self) -> usize {
        self.chosen.len()
    }

    /// The headers over the two lists (default "Available" and "Chosen").
    /// Each is followed by the number of items its list shows. Mark a
    /// letter with tildes, as in `"~T~oppings"`, and Alt+that letter moves
    /// to the list's filter field while the focus is in the view.
    pub fn set_titles(&mut self, available: impl Into<String>, chosen: impl Into<String>) {
        self.titles = [available.into(), chosen.into()];
    }

    /// The text in front of each filter field (default "Filter:"). An
    /// empty text leaves the fields the whole width.
    pub fn set_filter_label(&mut self, label: impl Into<String>) {
        self.filter_label = label.into();
        self.place_children();
    }

    /// Keep the chosen list in the order items were chosen instead of the
    /// items' own order (off by default).
    pub fn set_keep_chosen_order(&mut self, keep: bool) {
        self.keep_chosen_order = keep;
        if !keep {
            self.chosen.sort_unstable();
            self.changed();
        }
    }

    /// The fewest items OK accepts (default 0).
    pub fn set_min_chosen(&mut self, min: usize) {
        self.min_chosen = min;
    }

    /// The most items that can be chosen (default no limit): a move that
    /// would choose more is refused and shows the error line.
    pub fn set_max_chosen(&mut self, max: Option<usize>) {
        self.max_chosen = max;
    }

    /// The error line when fewer than the minimum are chosen (default
    /// "Choose at least N items").
    pub fn set_min_message(&mut self, message: impl Into<String>) {
        self.min_message = Some(message.into());
    }

    /// The error line when more than the maximum are chosen (default
    /// "Choose at most N items").
    pub fn set_max_message(&mut self, message: impl Into<String>) {
        self.max_message = Some(message.into());
    }

    /// Broadcast `command` after every move (zero, the default, sends
    /// none).
    pub fn set_on_change(&mut self, command: CommandId) {
        self.on_change = command;
    }

    /// True while the error line is showing: `valid()` or a move past the
    /// maximum was refused, and nothing has moved since.
    #[must_use]
    pub fn shows_error(&self) -> bool {
        self.refused.is_some()
    }

    fn list(&self, side: Side) -> &ListBox {
        self.group
            .child_at(side.list_child())
            .as_any()
            .downcast_ref::<ListBox>()
            .expect("the list child is a ListBox")
    }

    fn list_mut(&mut self, side: Side) -> &mut ListBox {
        self.group
            .child_at_mut(side.list_child())
            .as_any_mut()
            .downcast_mut::<ListBox>()
            .expect("the list child is a ListBox")
    }

    fn filter_text(&self, side: Side) -> String {
        self.group
            .child_at(side.filter_child())
            .as_any()
            .downcast_ref::<InputLine>()
            .expect("the filter child is an InputLine")
            .get_text()
    }

    /// The items on `side` before filtering, in display order.
    fn side_items(&self, side: Side) -> Vec<usize> {
        match side {
            Side::Available => {
                let mut chosen = vec![false; self.items.len()];
                for &i in &self.chosen {
                    chosen[i] = true;
                }
                (0..self.items.len()).filter(|&i| !chosen[i]).collect()
            }
            Side::Chosen => self.chosen.clone(),
        }
    }

    /// Refill the list on `side` from the model and its filter, keeping the
    /// focus on the same item, or on the same row if that item has gone.
    fn refill(&mut self, side: Side) {
        let filter = self.filters[side.index()].to_lowercase();
        let rows: Vec<usize> = self
            .side_items(side)
            .into_iter()
            .filter(|&i| self.items[i].1.to_lowercase().contains(&filter))
            .collect();
        let old_row = self.list(side).get_selection();
        let old_item = old_row.and_then(|r| self.shown[side.index()].get(r).copied());
        let labels = rows.iter().map(|&i| self.items[i].1.clone()).collect();
        let new_row = old_item
            .and_then(|item| rows.iter().position(|&i| i == item))
            .or(old_row.map(|r| r.min(rows.len().saturating_sub(1))));
        let list = self.list_mut(side);
        list.set_items(labels);
        if let Some(row) = new_row {
            list.set_selection(row);
        }
        self.shown[side.index()] = rows;
    }

    /// Refill both lists and publish the chosen keys after the model
    /// changed.
    fn changed(&mut self) {
        self.refill(Side::Available);
        self.refill(Side::Chosen);
        *self.data.borrow_mut() = self.chosen_keys();
        self.refused = None;
    }

    /// Refilter a list whose filter text was edited.
    fn sync_filters(&mut self) {
        for side in [Side::Available, Side::Chosen] {
            let text = self.filter_text(side);
            if text != self.filters[side.index()] {
                self.filters[side.index()] = text;
                self.refill(side);
            }
        }
    }

    /// The rows Enter or a single arrow moves: the marked ones, or the
    /// focused one when none is marked.
    fn picked_rows(&self, side: Side) -> Vec<usize> {
        let list = self.list(side);
        let mut rows = list.marked_items();
        if rows.is_empty() {
            rows.extend(list.get_selection());
        }
        rows.sort_unstable();
        rows
    }

    /// Move the items at `rows` of the list on `from` to the other list.
    /// Returns whether anything moved.
    fn move_rows(&mut self, from: Side, rows: &[usize]) -> bool {
        let picked: Vec<usize> = rows
            .iter()
            .filter_map(|&r| self.shown[from.index()].get(r).copied())
            .collect();
        if picked.is_empty() {
            return false;
        }
        match from {
            Side::Available => {
                // Borland's validators reject input that can never be valid
                // as it is typed (IsValidInput) and leave completeness to
                // valid() (IsValid). Too many is the first kind: refuse the
                // move here, or the selection could only be fixed by moving
                // items back, and every valid() — a modeless window's close
                // button too — would be vetoed until then.
                if self
                    .max_chosen
                    .is_some_and(|max| self.chosen.len() + picked.len() > max)
                {
                    self.refused = Some(Refusal::TooMany);
                    return false;
                }
                self.chosen.extend(picked);
                if !self.keep_chosen_order {
                    self.chosen.sort_unstable();
                }
            }
            Side::Chosen => self.chosen.retain(|i| !picked.contains(i)),
        }
        self.changed();
        true
    }

    /// Act on a command one of the children sent. Returns false for any
    /// other command.
    fn run_command(&mut self, command: CommandId) -> bool {
        let (from, all) = match command {
            CMD_ADD | CMD_PICK_AVAILABLE => (Side::Available, false),
            CMD_ADD_ALL => (Side::Available, true),
            CMD_REMOVE | CMD_PICK_CHOSEN => (Side::Chosen, false),
            CMD_REMOVE_ALL => (Side::Chosen, true),
            _ => return false,
        };
        let rows: Vec<usize> = if all {
            (0..self.shown[from.index()].len()).collect()
        } else {
            self.picked_rows(from)
        };
        self.move_rows(from, &rows)
    }

    fn check(&self) -> Option<Refusal> {
        let n = self.chosen.len();
        if n < self.min_chosen {
            Some(Refusal::TooFew)
        } else if self.max_chosen.is_some_and(|max| n > max) {
            Some(Refusal::TooMany)
        } else {
            None
        }
    }

    fn error_message(&self, refusal: Refusal) -> String {
        let (custom, word, n) = match refusal {
            Refusal::TooFew => (&self.min_message, "least", self.min_chosen),
            Refusal::TooMany => (&self.max_message, "most", self.max_chosen.unwrap_or(0)),
        };
        match custom {
            Some(message) => message.clone(),
            None if n == 1 => format!("Choose at {word} one item"),
            None => format!("Choose at {word} {n} items"),
        }
    }

    /// The child holding the focus inside the view, if any.
    fn focused_child(&self) -> Option<usize> {
        (0..self.group.len()).find(|&i| self.group.child_at(i).is_focused())
    }

    /// The children Tab stops on, in order.
    fn focusable_children(&self) -> Vec<usize> {
        (0..self.group.len())
            .filter(|&i| {
                let child = self.group.child_at(i);
                child.can_focus() && !child.state().contains(State::DISABLED)
            })
            .collect()
    }

    fn header_text(&self, side: Side) -> String {
        let shown = self.shown[side.index()].len();
        let total = self.side_items(side).len();
        let title = &self.titles[side.index()];
        if shown == total {
            format!(" {title} ({total})")
        } else {
            format!(" {title} ({shown} of {total})")
        }
    }

    fn draw_headers(&self, terminal: &mut Terminal) {
        let layout = self.layout();
        let width = usize::try_from(layout.side_width).unwrap_or(0);
        let focused = if self.is_focused() {
            self.focused_child()
        } else {
            None
        };
        for side in [Side::Available, Side::Chosen] {
            let (text, attr) = match self.refused {
                Some(refusal) if side == Side::Chosen => (
                    format!(" {}", self.error_message(refusal)),
                    Attr::from_u8(ERROR_ATTR),
                ),
                _ => {
                    let lit =
                        focused.is_some_and(|i| i == side.filter_child() || i == side.list_child());
                    let color = if lit { HEADER_FOCUSED } else { HEADER_NORMAL };
                    (self.header_text(side), self.map_color(color))
                }
            };
            let mut buf = DrawBuffer::new(width);
            buf.move_char(0, ' ', attr, width);
            let shortcut = if self.refused.is_some() && side == Side::Chosen {
                attr
            } else {
                self.map_color(HEADER_SHORTCUT)
            };
            buf.move_str_with_shortcut(0, &text, attr, shortcut);
            write_line_to_terminal(terminal, layout.x(side), 0, &buf);
        }
    }

    /// The text in front of each filter field, lit while its field has the
    /// focus, as a core `Label` is.
    fn draw_filter_labels(&self, terminal: &mut Terminal) {
        let layout = self.layout();
        let width = usize::try_from(layout.label_width).unwrap_or(0);
        if width == 0 {
            return;
        }
        let focused = if self.is_focused() {
            self.focused_child()
        } else {
            None
        };
        for side in [Side::Available, Side::Chosen] {
            let lit = focused == Some(side.filter_child());
            let attr = self.map_color(if lit { HEADER_FOCUSED } else { HEADER_NORMAL });
            let mut buf = DrawBuffer::new(width);
            buf.move_char(0, ' ', attr, width);
            buf.move_str(0, &self.filter_label, attr);
            write_line_to_terminal(terminal, layout.x(side), 1, &buf);
        }
    }

    fn layout(&self) -> Layout {
        Layout::new(
            self.core.bounds.width(),
            self.core.bounds.height(),
            &self.filter_label,
        )
    }

    /// Put each child where the layout wants it for the current size.
    fn place_children(&mut self) {
        let layout = self.layout();
        let places = [
            layout.filter(Side::Available),
            layout.list(Side::Available),
            layout.button(0),
            layout.button(1),
            layout.button(2),
            layout.button(3),
            layout.filter(Side::Chosen),
            layout.list(Side::Chosen),
        ];
        for (i, rect) in places.into_iter().enumerate() {
            self.group.child_at_mut(i).set_bounds(rect);
        }
    }
}

impl<K: Clone + PartialEq + 'static> View for DualList<K> {
    fn core(&self) -> &ViewCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut ViewCore {
        &mut self.core
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.core.bounds = bounds;
        self.group.set_bounds(self.extent());
        self.place_children();
    }

    fn draw(&mut self, terminal: &mut Terminal) {
        // Hand the inner group this view's chain node, as ScrollPane does:
        // the view has no palette of its own, so the children resolve their
        // colours through the dialog it sits in.
        let node = PaletteChainNode::new(self.get_palette(), self.palette_chain.clone());
        self.group.set_palette_chain(Some(node));
        terminal.push_clip(self.extent());
        self.group.draw(terminal);
        self.draw_headers(terminal);
        self.draw_filter_labels(terminal);
        terminal.pop_clip();
    }

    fn handle_event(&mut self, event: &mut Event) {
        // A click on a filter label focuses its field, as on a core Label
        if event.what == EventType::MouseDown && event.mouse.buttons & MB_LEFT_BUTTON != 0 {
            let layout = self.layout();
            for side in [Side::Available, Side::Chosen] {
                if layout.filter_label(side).contains(event.mouse.pos) {
                    self.group.set_focus_to(side.filter_child());
                    event.clear();
                    return;
                }
            }
        }

        if event.what == EventType::Keyboard {
            for side in [Side::Available, Side::Chosen] {
                if hotkey(&self.titles[side.index()]) == Some(event.key_code) {
                    self.group.set_focus_to(side.filter_child());
                    event.clear();
                    return;
                }
            }
            let focused = self.focused_child();
            match event.key_code {
                // The inner group would wrap Tab around inside the view; at
                // the last (or first) part, leave it to the owner instead so
                // the focus moves on to the dialog's next control.
                KB_TAB | KB_SHIFT_TAB => {
                    let stops = self.focusable_children();
                    let edge = if event.key_code == KB_TAB {
                        stops.last()
                    } else {
                        stops.first()
                    };
                    if focused.is_some() && focused == edge.copied() {
                        return;
                    }
                }
                KB_DOWN => {
                    for side in [Side::Available, Side::Chosen] {
                        if focused == Some(side.filter_child()) {
                            self.group.set_focus_to(side.list_child());
                            event.clear();
                            return;
                        }
                    }
                }
                _ => {}
            }
        }

        // Only a command a child made out of this event is one of ours.
        let was_command = event.what == EventType::Command;
        self.group.handle_event(event);
        if !was_command && event.what == EventType::Command {
            let command = event.command;
            if is_own_command(command) {
                let moved = self.run_command(command);
                if moved && self.on_change != 0 {
                    *event = Event::broadcast_with_info(self.on_change, 0);
                } else {
                    event.clear();
                }
            }
        }
        self.sync_filters();
    }

    fn can_focus(&self) -> bool {
        true
    }

    fn set_focus(&mut self, focused: bool) {
        self.set_state_flag(State::FOCUSED, focused);
        if focused {
            match self.last_focus {
                Some(i) => self.group.set_focus_to(i),
                None => self.group.set_initial_focus(),
            }
        } else {
            if let Some(i) = self.focused_child() {
                self.last_focus = Some(i);
            }
            self.group.clear_all_focus();
        }
    }

    fn update_cursor(&self, terminal: &mut Terminal) {
        self.group.update_cursor(terminal);
    }

    fn valid(&mut self, command: CommandId) -> bool {
        if command == CM_CANCEL {
            return true;
        }
        if !self.group.valid(command) {
            return false;
        }
        // Borland's validators show a message box here, but `valid()` has
        // no access to the application to run one. The view turns the
        // chosen list's header into the error line instead.
        self.refused = self.check();
        self.refused.is_none()
    }

    fn get_palette(&self) -> Option<Palette> {
        None
    }

    fn set_palette_chain(&mut self, node: Option<PaletteChainNode>) {
        self.palette_chain = node;
    }

    fn get_palette_chain(&self) -> Option<&PaletteChainNode> {
        self.palette_chain.as_ref()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// True for the commands the view's own children send.
fn is_own_command(command: CommandId) -> bool {
    (CMD_ADD..=CMD_PICK_CHOSEN).contains(&command)
}

/// Builds a [`DualList`].
///
/// # Examples
///
/// ```
/// use turbo_vision::core::geometry::Rect;
/// use tv_extensions::DualListBuilder;
///
/// let list = DualListBuilder::new()
///     .bounds(Rect::new(2, 2, 60, 16))
///     .items([("es", "Spain"), ("fr", "France"), ("it", "Italy")])
///     .titles("Countries", "Visited")
///     .required(true)
///     .build();
/// assert_eq!(list.chosen_count(), 0);
/// ```
pub struct DualListBuilder<K> {
    bounds: Option<Rect>,
    items: Vec<(K, String)>,
    data: Option<Rc<RefCell<Vec<K>>>>,
    titles: Option<(String, String)>,
    filter_label: Option<String>,
    keep_chosen_order: bool,
    min_chosen: usize,
    max_chosen: Option<usize>,
    min_message: Option<String>,
    max_message: Option<String>,
    on_change: CommandId,
}

impl<K: Clone + PartialEq + 'static> DualListBuilder<K> {
    /// A builder with the [`DualList`] defaults and no bounds yet.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bounds: None,
            items: Vec::new(),
            data: None,
            titles: None,
            filter_label: None,
            keep_chosen_order: false,
            min_chosen: 0,
            max_chosen: None,
            min_message: None,
            max_message: None,
            on_change: 0,
        }
    }

    /// The view's bounds. Required; at least 10 rows tall.
    #[must_use]
    pub fn bounds(mut self, bounds: Rect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    /// The items, as (key, label) pairs, in the order the lists show them.
    #[must_use]
    pub fn items<L: Into<String>>(mut self, items: impl IntoIterator<Item = (K, L)>) -> Self {
        self.items = items.into_iter().map(|(k, l)| (k, l.into())).collect();
        self
    }

    /// Share the chosen keys in `data`; the keys already in it start out
    /// chosen. Without it the view keeps them to itself, for
    /// [`DualList::chosen_keys`].
    #[must_use]
    pub fn data(mut self, data: Rc<RefCell<Vec<K>>>) -> Self {
        self.data = Some(data);
        self
    }

    /// See [`DualList::set_titles`].
    #[must_use]
    pub fn titles(mut self, available: impl Into<String>, chosen: impl Into<String>) -> Self {
        self.titles = Some((available.into(), chosen.into()));
        self
    }

    /// See [`DualList::set_filter_label`].
    #[must_use]
    pub fn filter_label(mut self, label: impl Into<String>) -> Self {
        self.filter_label = Some(label.into());
        self
    }

    /// See [`DualList::set_keep_chosen_order`].
    #[must_use]
    pub fn keep_chosen_order(mut self, keep: bool) -> Self {
        self.keep_chosen_order = keep;
        self
    }

    /// At least one item must be chosen: `min_chosen(1)`, or
    /// `min_chosen(0)` when false.
    #[must_use]
    pub fn required(mut self, required: bool) -> Self {
        self.min_chosen = usize::from(required);
        self
    }

    /// See [`DualList::set_min_chosen`].
    #[must_use]
    pub fn min_chosen(mut self, min: usize) -> Self {
        self.min_chosen = min;
        self
    }

    /// See [`DualList::set_max_chosen`].
    #[must_use]
    pub fn max_chosen(mut self, max: usize) -> Self {
        self.max_chosen = Some(max);
        self
    }

    /// See [`DualList::set_min_message`].
    #[must_use]
    pub fn min_message(mut self, message: impl Into<String>) -> Self {
        self.min_message = Some(message.into());
        self
    }

    /// See [`DualList::set_max_message`].
    #[must_use]
    pub fn max_message(mut self, message: impl Into<String>) -> Self {
        self.max_message = Some(message.into());
        self
    }

    /// See [`DualList::set_on_change`].
    #[must_use]
    pub fn on_change(mut self, command: CommandId) -> Self {
        self.on_change = command;
        self
    }

    /// Build the view.
    ///
    /// # Panics
    ///
    /// If no bounds were set.
    #[must_use]
    pub fn build(self) -> DualList<K> {
        let bounds = self.bounds.expect("DualList bounds must be set");
        let data = self.data.unwrap_or_default();
        // Read the preselected keys first: `new` without items publishes an
        // empty selection into `data`.
        let keys = data.borrow().clone();
        let mut view = DualList::new(bounds, Vec::new(), data);
        // Set the order before choosing, so preselected keys keep theirs
        view.keep_chosen_order = self.keep_chosen_order;
        view.items = self.items;
        view.set_chosen(&keys);
        if let Some((available, chosen)) = self.titles {
            view.set_titles(available, chosen);
        }
        if let Some(label) = self.filter_label {
            view.set_filter_label(label);
        }
        view.min_chosen = self.min_chosen;
        view.max_chosen = self.max_chosen;
        view.min_message = self.min_message;
        view.max_message = self.max_message;
        view.on_change = self.on_change;
        view
    }

    /// Build the view, boxed.
    #[must_use]
    pub fn build_boxed(self) -> Box<DualList<K>> {
        Box::new(self.build())
    }
}

impl<K: Clone + PartialEq + 'static> Default for DualListBuilder<K> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use turbo_vision::core::command::CM_OK;
    use turbo_vision::core::event::KB_ENTER;
    // Space, as core ListBox reads it to mark an item
    const KB_SPACE: u16 = 0x0020;
    use turbo_vision::core::geometry::Point;

    const FRUITS: [&str; 5] = ["Apple", "Banana", "Cherry", "Mango", "Orange"];

    /// A 50x12 view at the origin, so its own coordinates are the ones
    /// events carry. Keys are the fruits' positions in `FRUITS`.
    fn make(preselected: Vec<usize>) -> (DualList<usize>, Rc<RefCell<Vec<usize>>>) {
        let data = Rc::new(RefCell::new(preselected));
        let list = DualListBuilder::new()
            .bounds(Rect::new(0, 0, 50, 12))
            .items(FRUITS.iter().copied().enumerate())
            .data(data.clone())
            .build();
        (list, data)
    }

    fn labels(list: &DualList<usize>, side: Side) -> Vec<&str> {
        list.shown[side.index()]
            .iter()
            .map(|&i| list.items[i].1.as_str())
            .collect()
    }

    fn key(list: &mut DualList<usize>, key_code: u16) -> Event {
        let mut event = Event::keyboard(key_code);
        list.handle_event(&mut event);
        event
    }

    fn type_text(list: &mut DualList<usize>, text: &str) {
        for ch in text.chars() {
            key(list, ch as u16);
        }
    }

    fn click(list: &mut DualList<usize>, x: i16, y: i16, double: bool) {
        let pos = Point::new(x, y);
        let mut down = Event::mouse(EventType::MouseDown, pos, MB_LEFT_BUTTON, double);
        list.handle_event(&mut down);
        let mut up = Event::mouse(EventType::MouseUp, pos, 0, false);
        list.handle_event(&mut up);
    }

    fn focus(list: &mut DualList<usize>, child: usize) {
        list.set_focus(true);
        list.group.set_focus_to(child);
    }

    #[test]
    fn preselected_keys_start_chosen_and_unknown_ones_are_dropped() {
        let (list, data) = make(vec![3, 99, 1]);
        assert_eq!(labels(&list, Side::Chosen), ["Banana", "Mango"]);
        assert_eq!(
            labels(&list, Side::Available),
            ["Apple", "Cherry", "Orange"]
        );
        assert_eq!(*data.borrow(), vec![1, 3]);
    }

    #[test]
    fn enter_moves_the_focused_item_and_the_focus_stays_on_its_row() {
        let (mut list, data) = make(vec![]);
        focus(&mut list, AVAILABLE_LIST);
        key(&mut list, KB_DOWN);
        let event = key(&mut list, KB_ENTER);
        assert_eq!(event.what, EventType::Nothing);
        assert_eq!(*data.borrow(), vec![1]);
        // Banana left row 1; Cherry slid up into it
        assert_eq!(list.list(Side::Available).get_selection(), Some(1));
        assert_eq!(labels(&list, Side::Available)[1], "Cherry");
    }

    #[test]
    fn enter_moves_the_marked_items_when_some_are_marked() {
        let (mut list, data) = make(vec![]);
        focus(&mut list, AVAILABLE_LIST);
        key(&mut list, KB_SPACE);
        key(&mut list, KB_DOWN);
        key(&mut list, KB_DOWN);
        key(&mut list, KB_SPACE);
        key(&mut list, KB_DOWN);
        key(&mut list, KB_ENTER);
        assert_eq!(*data.borrow(), vec![0, 2]);
    }

    #[test]
    fn the_filter_narrows_its_list_and_move_all_takes_only_what_it_shows() {
        let (mut list, data) = make(vec![]);
        focus(&mut list, AVAILABLE_FILTER);
        type_text(&mut list, "AN");
        assert_eq!(
            labels(&list, Side::Available),
            ["Banana", "Mango", "Orange"]
        );
        assert_eq!(list.header_text(Side::Available), " Available (3 of 5)");

        assert!(list.run_command(CMD_ADD_ALL));
        assert_eq!(*data.borrow(), vec![1, 3, 4]);
        assert!(labels(&list, Side::Available).is_empty());
    }

    #[test]
    fn down_in_a_filter_moves_to_its_list() {
        let (mut list, _) = make(vec![]);
        focus(&mut list, CHOSEN_FILTER);
        let event = key(&mut list, KB_DOWN);
        assert_eq!(event.what, EventType::Nothing);
        assert_eq!(list.focused_child(), Some(CHOSEN_LIST));
    }

    #[test]
    fn tab_steps_through_the_parts_then_leaves_for_the_owner() {
        let (mut list, _) = make(vec![]);
        list.set_focus(true);
        assert_eq!(list.focused_child(), Some(AVAILABLE_FILTER));
        for expected in 1..=CHOSEN_LIST {
            assert_eq!(key(&mut list, KB_TAB).what, EventType::Nothing);
            assert_eq!(list.focused_child(), Some(expected));
        }
        // At the last part Tab is left for the dialog, and so is Shift+Tab
        // at the first
        assert_eq!(key(&mut list, KB_TAB).what, EventType::Keyboard);
        focus(&mut list, AVAILABLE_FILTER);
        assert_eq!(key(&mut list, KB_SHIFT_TAB).what, EventType::Keyboard);
    }

    #[test]
    fn coming_back_lands_on_the_part_that_had_the_focus() {
        let (mut list, _) = make(vec![]);
        focus(&mut list, CHOSEN_LIST);
        list.set_focus(false);
        assert_eq!(list.focused_child(), None);
        list.set_focus(true);
        assert_eq!(list.focused_child(), Some(CHOSEN_LIST));
    }

    #[test]
    fn the_arrow_buttons_move_items_on_a_click() {
        let (mut list, data) = make(vec![]);
        // 50 columns: lists 21 wide, buttons at x 22..28 from row 3
        let layout = Layout::new(50, 12, DEFAULT_FILTER_LABEL);
        assert_eq!(layout.button(0), Rect::new(22, 3, 28, 5));
        focus(&mut list, AVAILABLE_LIST);
        click(&mut list, 24, 3, false); // >
        assert_eq!(*data.borrow(), vec![0]);
        click(&mut list, 24, 5, false); // >>
        assert_eq!(*data.borrow(), vec![0, 1, 2, 3, 4]);
        click(&mut list, 24, 9, false); // <<
        assert!(data.borrow().is_empty());
    }

    #[test]
    fn a_double_click_moves_the_clicked_item() {
        let (mut list, data) = make(vec![]);
        list.set_focus(true);
        // The list starts on row 2, so row 4 is its third item
        click(&mut list, 3, 4, true);
        assert_eq!(*data.borrow(), vec![2]);
    }

    #[test]
    fn the_filter_fields_sit_after_their_labels() {
        let (mut list, _) = make(vec![]);
        // "Filter:" and a space: the field starts 8 columns in
        assert_eq!(
            list.group.child_at(AVAILABLE_FILTER).bounds(),
            Rect::new(8, 1, 21, 2)
        );
        assert_eq!(
            list.group.child_at(CHOSEN_FILTER).bounds(),
            Rect::new(37, 1, 50, 2)
        );
        list.set_filter_label("");
        assert_eq!(
            list.group.child_at(AVAILABLE_FILTER).bounds(),
            Rect::new(0, 1, 21, 2)
        );
    }

    #[test]
    fn a_click_on_a_filter_label_focuses_its_field() {
        let (mut list, _) = make(vec![]);
        focus(&mut list, AVAILABLE_LIST);
        click(&mut list, 30, 1, false); // the chosen side's "Filter:"
        assert_eq!(list.focused_child(), Some(CHOSEN_FILTER));
    }

    #[test]
    fn a_title_hotkey_moves_to_its_filter() {
        use turbo_vision::core::event::{KB_ALT_O, KB_ALT_T};
        let (mut list, _) = make(vec![]);
        list.set_titles("~T~oppings", "~O~n the pizza");
        focus(&mut list, CHOSEN_LIST);
        assert_eq!(key(&mut list, KB_ALT_T).what, EventType::Nothing);
        assert_eq!(list.focused_child(), Some(AVAILABLE_FILTER));
        key(&mut list, KB_ALT_O);
        assert_eq!(list.focused_child(), Some(CHOSEN_FILTER));
        // Other Alt keys are left for the dialog
        assert_eq!(key(&mut list, KB_ALT_A).what, EventType::Keyboard);
    }

    #[test]
    fn hotkeys_are_read_from_the_tilde_marked_letter() {
        assert_eq!(hotkey("~T~oppings"), Some(KB_ALT_T));
        assert_eq!(hotkey("On the ~p~izza"), Some(KB_ALT_P));
        assert_eq!(hotkey("Toppings"), None);
        assert_eq!(hotkey("~1~st"), None);
    }

    #[test]
    fn a_title_hotkey_draws_in_the_shortcut_colour_without_its_tildes() {
        use turbo_vision::views::GroupLike;
        use turbo_vision::views::dialog::Dialog;

        let list = DualListBuilder::new()
            .bounds(Rect::new(0, 0, 50, 12))
            .items(FRUITS.iter().copied().enumerate())
            .titles("~F~ruits", "Chosen")
            .build();
        let mut dialog = Dialog::new(Rect::new(0, 0, 54, 16), "");
        dialog.add(list);
        let mut term = turbo_vision::test_util::test_terminal(54, 16);
        dialog.draw(&mut term);
        let text: String = (1..12).map(|x| term.read_cell(x, 1).unwrap().ch).collect();
        assert_eq!(text, " Fruits (5)");
        assert_ne!(
            term.read_cell(2, 1).unwrap().attr,
            term.read_cell(3, 1).unwrap().attr
        );
        // The filter label on the next row
        let label: String = (1..8).map(|x| term.read_cell(x, 2).unwrap().ch).collect();
        assert_eq!(label, "Filter:");
    }

    #[test]
    fn a_command_from_outside_is_not_taken_for_a_button() {
        let (mut list, data) = make(vec![]);
        focus(&mut list, AVAILABLE_LIST);
        let mut event = Event::command(CMD_ADD_ALL);
        list.handle_event(&mut event);
        assert!(data.borrow().is_empty());
        assert_eq!(event.what, EventType::Command);
    }

    #[test]
    fn on_change_is_broadcast_after_a_move() {
        let (mut list, _) = make(vec![]);
        list.set_on_change(500);
        focus(&mut list, AVAILABLE_LIST);
        let event = key(&mut list, KB_ENTER);
        assert_eq!(event.what, EventType::Broadcast);
        assert_eq!(event.command, 500);
    }

    #[test]
    fn the_chosen_list_follows_the_items_order_unless_told_otherwise() {
        let (mut list, data) = make(vec![]);
        list.set_chosen(&[4, 0]);
        assert_eq!(*data.borrow(), vec![0, 4]);

        let data = Rc::new(RefCell::new(vec![4, 0]));
        let mut list = DualListBuilder::new()
            .bounds(Rect::new(0, 0, 50, 12))
            .items(FRUITS.iter().copied().enumerate())
            .data(data.clone())
            .keep_chosen_order(true)
            .build();
        assert_eq!(*data.borrow(), vec![4, 0]);
        focus(&mut list, AVAILABLE_LIST);
        key(&mut list, KB_ENTER); // Banana, the first available
        assert_eq!(*data.borrow(), vec![4, 0, 1]);
    }

    #[test]
    fn set_items_keeps_the_chosen_keys_that_still_exist() {
        let (mut list, data) = make(vec![1, 3]);
        list.set_items(vec![(3, "Mango".into()), (7, "Kiwi".into())]);
        assert_eq!(*data.borrow(), vec![3]);
        assert_eq!(labels(&list, Side::Available), ["Kiwi"]);
    }

    #[test]
    fn ok_is_refused_outside_the_bounds_and_cancel_always_closes() {
        let (mut list, _) = make(vec![]);
        list.set_min_chosen(1);
        list.set_max_chosen(Some(2));
        assert!(list.valid(CM_CANCEL));
        assert!(!list.valid(CM_OK));
        assert_eq!(
            list.error_message(Refusal::TooFew),
            "Choose at least one item"
        );

        list.set_chosen(&[0, 1, 2]);
        assert!(!list.valid(CM_OK));
        assert_eq!(
            list.error_message(Refusal::TooMany),
            "Choose at most 2 items"
        );

        list.set_chosen(&[0]);
        assert!(list.valid(CM_OK));
    }

    #[test]
    fn a_move_past_the_maximum_is_refused_so_closing_still_works() {
        use turbo_vision::core::command::CM_CLOSE;
        let (mut list, data) = make(vec![0]);
        list.set_max_chosen(Some(2));
        assert!(!list.run_command(CMD_ADD_ALL), "four more is two too many");
        assert_eq!(*data.borrow(), vec![0]);
        assert!(list.shows_error());
        assert!(list.valid(CM_CLOSE), "nothing invalid was chosen");

        // One more fits, and moving clears the error line.
        focus(&mut list, AVAILABLE_LIST);
        key(&mut list, KB_ENTER);
        assert_eq!(*data.borrow(), vec![0, 1]);
        assert!(!list.shows_error());
        key(&mut list, KB_ENTER);
        assert_eq!(*data.borrow(), vec![0, 1], "a third is refused");
    }

    #[test]
    fn a_refusal_shows_in_the_chosen_header_until_the_next_move() {
        use turbo_vision::views::GroupLike;
        use turbo_vision::views::dialog::Dialog;

        let data = Rc::new(RefCell::new(Vec::new()));
        let list = DualListBuilder::new()
            .bounds(Rect::new(0, 0, 50, 12))
            .items(FRUITS.iter().copied().enumerate())
            .data(data.clone())
            .required(true)
            .min_message("Pick a fruit")
            .build();
        let mut dialog = Dialog::new(Rect::new(0, 0, 54, 16), "");
        dialog.add(list);
        let mut term = turbo_vision::test_util::test_terminal(54, 16);

        // The dialog's frame insets the view by one, so its row 0 is row 1
        let row = |term: &Terminal, x0: i16, len: i16| -> String {
            (x0..x0 + len)
                .map(|x| term.read_cell(x, 1).unwrap().ch)
                .collect()
        };
        dialog.draw(&mut term);
        assert_eq!(row(&term, 1, 14), " Available (5)");
        assert_eq!(row(&term, 30, 11), " Chosen (0)");
        let header_attr = term.read_cell(31, 1).unwrap().attr;

        assert!(!dialog.valid(CM_OK));
        dialog.draw(&mut term);
        assert_eq!(row(&term, 30, 13), " Pick a fruit");
        assert_eq!(
            term.read_cell(31, 1).unwrap().attr,
            Attr::from_u8(ERROR_ATTR)
        );

        let list = dialog
            .child_at_mut(0)
            .as_any_mut()
            .downcast_mut::<DualList<usize>>()
            .unwrap();
        list.set_chosen(&[0]);
        assert!(!list.shows_error());
        dialog.draw(&mut term);
        assert_eq!(row(&term, 30, 11), " Chosen (1)");
        assert_eq!(term.read_cell(31, 1).unwrap().attr, header_attr);
        assert!(dialog.valid(CM_OK));
    }

    #[test]
    fn the_lists_draw_in_the_dialogs_list_colours() {
        use turbo_vision::views::GroupLike;
        use turbo_vision::views::dialog::Dialog;

        // A plain ListBox in a dialog against the view's available list,
        // both unfocused, at the same spot.
        let mut plain = Dialog::new(Rect::new(0, 0, 54, 16), "");
        let mut lb = ListBox::new(Rect::new(0, 2, 21, 12), 0);
        lb.set_items(vec!["x".into()]);
        plain.add(lb);
        let mut term = turbo_vision::test_util::test_terminal(54, 16);
        plain.draw(&mut term);
        let expected = term.read_cell(1, 4).unwrap().attr;

        let (list, _) = make(vec![]);
        let mut dialog = Dialog::new(Rect::new(0, 0, 54, 16), "");
        dialog.add(list);
        let mut term = turbo_vision::test_util::test_terminal(54, 16);
        dialog.draw(&mut term);
        assert_eq!(term.read_cell(1, 4).unwrap().attr, expected);
    }
}
