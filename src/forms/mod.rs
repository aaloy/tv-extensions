// (C) 2025 - Enzo Lombardi

//! Form layout - build a dialog from a list of fields, with no coordinates.
//!
//! A [`Form`] lays out a [`Dialog`] for you: labels in one column, fields in
//! the next, buttons centred along the bottom, and the dialog sized to fit
//! and centred on the screen. You say *what* goes in the form, in order; the
//! form decides *where* it goes.
//!
//! ```
//! use turbo_vision::core::geometry::Rect;
//! use turbo_vision::views::GroupLike;
//! use turbo_vision::views::checkbox::CheckBox;
//! use tv_extensions::forms::{Form, size};
//! use turbo_vision::views::input_line::InputLine;
//!
//! let mut form = Form::new("Customer");
//! // No size: the field stretches to the width of the field column.
//! let name = form.field("~N~ame", InputLine::new(Rect::default(), 40));
//!
//! // A titled box around the rows up to `end_group`.
//! form.group("Address");
//! let street = form.field("~S~treet", InputLine::new(Rect::default(), 60));
//! // Several fields on one line; a size keeps it (8 columns, 1 row).
//! let mut line = form.line();
//! let city = line.field("~C~ity", InputLine::new(Rect::default(), 40));
//! let zip = line.field("~Z~IP", InputLine::new(size(8, 1), 8));
//! form.end_group();
//!
//! // A row with no label, aligned with the fields.
//! let vip = form.field("", CheckBox::new(Rect::default(), "VIP customer"));
//! form.ok_cancel();
//! let mut dialog = form.build();
//!
//! // Run it with `dialog.execute(&mut app)`; then read the values back
//! // through the handles the form returned:
//! let name_text = dialog.get(name).map(|f| f.text().to_string());
//! # assert_eq!(name_text.as_deref(), Some(""));
//! # let _ = (street, city, zip, vip);
//! ```
//!
//! # Layout rules
//!
//! - **Rows** go top to bottom in the order they are added, with
//!   [`spacing`](Form::spacing) blank rows between them (1 by default).
//! - **Labels** ([`field`](Form::field)) sit to the left of their field, in
//!   a column as wide as the longest label, or above it with
//!   [`label_position`](Form::label_position). The column can be
//!   right-aligned with [`label_align`](Form::label_align). A label is linked
//!   to its field: clicking it, or pressing Alt and its `~`-marked letter,
//!   focuses the field. An empty label leaves its place blank, so the view
//!   still lines up with the fields.
//! - **Sizes** come from the view: build it with `size(width, height)` to fix
//!   its size, or with `Rect::default()` to let it stretch to the width it is
//!   given, one row high. Stretched views also follow the dialog's width when
//!   it is resized.
//! - **Lines** ([`line`](Form::line)) put several fields side by side. The
//!   first field's label is in the label column; the others' labels sit just
//!   before their fields. Stretched fields in a line share the width left.
//! - **Groups** ([`group`](Form::group) ... [`end_group`](Form::end_group))
//!   draw a titled box around their rows, as wide as the form. A group lines
//!   up its own labels, and groups can nest.
//! - **Tabs** ([`tab`](Form::tab) ... [`end_tabs`](Form::end_tabs)) put the
//!   rows that follow each `tab` on a page of a tab pane as wide as the form
//!   and as high as its tallest page. Fields on a page are not children of
//!   the dialog: reach them through a record form's bindings.
//! - **Full-width rows** ([`row`](Form::row)) start at the label column and
//!   span the whole form or group; [`section`](Form::section) adds a heading
//!   with a blank row above it.
//! - **Buttons** sit on one row at the bottom, centred (or right-aligned with
//!   [`button_align`](Form::button_align)), each at least 10 columns wide.
//! - **The dialog** is sized to fit its contents and its title, and centred
//!   on the desktop when it is executed or added to the desktop.
//!
//! # Editing records
//!
//! [`Form::for_record`] makes a form for a struct of yours, such as a database
//! row: each field is bound to one of its members, the values are converted
//! and validated, errors are shown in the dialog, and an [`Editor`] returns
//! the edited struct. See the [`data`] module.
//!
//! Every view keeps its own behaviour: validators, history lists, colours.
//! The form only sets each view's position and size, through
//! [`View::set_bounds`], once [`build`](Form::build) is called.

use turbo_vision::core::command::{CM_CANCEL, CM_OK, CommandId};
use turbo_vision::core::geometry::Rect;
use turbo_vision::core::state::{Grow, Options};
use turbo_vision::views::button::Button;
use turbo_vision::views::dialog::Dialog;
use turbo_vision::views::group::Group as ViewGroup;
use turbo_vision::views::group::GroupLike;
use turbo_vision::views::group_box::GroupBox;
use turbo_vision::views::handle::Handle;
use turbo_vision::views::label::Label;
use turbo_vision::views::static_text::StaticText;
use turbo_vision::views::tabbed_pane::TabbedPane;
use turbo_vision::views::view::{View, ViewId};

pub mod data;
#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
pub mod json;
pub use data::{Editor, Field, FieldError, FieldId, TextValue, ValidationErrors};

/// Columns between the dialog's frame and the form's contents, each side.
const MARGIN_X: i16 = 1;
/// Blank rows between the top of the frame and the first row.
const MARGIN_TOP: i16 = 1;
/// Columns between a label and its field.
const LABEL_GAP: i16 = 1;
/// Columns between two fields on one line.
const CELL_GAP: i16 = 2;
/// Narrowest a stretched field gets when it shares a line with others.
const MIN_LINE_FIELD: i16 = 10;
/// Columns between a group's frame and its contents: the border and a space.
const GROUP_PAD_X: i16 = 2;
/// Columns a group's frame needs besides its title: corners and padding.
const GROUP_TITLE_CHROME: i16 = 6;
/// Columns between a tab page's border and its contents.
const PAGE_PAD_X: i16 = 1;
/// Rows a tab pane draws above its pages (the strip and its rule).
const TAB_HEADER_ROWS: i16 = 3;
/// Columns a tab's title takes besides its text: padding and a divider.
const TAB_TITLE_CHROME: i16 = 4;
/// Columns between two buttons.
const BUTTON_GAP: i16 = 2;
/// Narrowest button, as in Borland's standard dialogs.
const MIN_BUTTON_WIDTH: i16 = 10;
/// Rows a button takes: its face and its shadow.
const BUTTON_HEIGHT: i16 = 2;
/// Columns the frame needs besides the title: corners, close box, padding.
const TITLE_CHROME: i16 = 12;

/// A size, for a view that should keep it in a [`Form`]:
/// `InputLine::new(size(8, 1), 8)` is a field 8 columns wide and 1 row high.
///
/// The form only reads the width and height; the position is its own.
#[must_use]
pub fn size(width: i16, height: i16) -> Rect {
    Rect::new(0, 0, width, height)
}

/// Where a [`Form`] puts its buttons along the bottom row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonAlign {
    /// Centred under the form (the default).
    #[default]
    Center,
    /// Against the right edge, as many modern dialogs do. Right-aligned
    /// buttons also stay against that edge when the dialog is resized.
    Right,
}

/// Where a [`Form`] puts each field's label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabelPosition {
    /// To the left of the field, in a column of its own (the default).
    #[default]
    Left,
    /// On the row above the field, starting at the field's left edge. Takes
    /// more rows and fewer columns: good for narrow forms and long labels.
    Above,
}

/// How a [`Form`] aligns the label column when labels are on the left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabelAlign {
    /// Labels start at the left edge of the column (the default).
    #[default]
    Left,
    /// Labels end against their fields, as in many business forms.
    Right,
}

/// A view's size as the form sees it: `None` for a width that stretches.
#[derive(Debug, Clone, Copy)]
struct Extent {
    width: Option<i16>,
    height: i16,
}

impl Extent {
    fn of(view: &dyn View) -> Self {
        let b = view.bounds();
        Self {
            width: (b.width() > 0).then_some(b.width()),
            height: b.height().max(1),
        }
    }
}

/// One field of a line, with its label if it has one.
struct Cell {
    label: Option<(ViewId, i16)>,
    view: ViewId,
    extent: Extent,
}

enum Item {
    /// Fields side by side; a plain labelled field is a line of one.
    Line(Vec<Cell>),
    /// A view spanning the whole form or group.
    Full { view: ViewId, extent: Extent },
    /// A heading over the rows that follow.
    Section { view: ViewId, width: i16 },
    /// Extra blank rows.
    Gap(i16),
    /// A titled box around rows of its own.
    Group(Group),
    /// Pages of rows under a strip of tabs.
    Tabs(Tabs),
}

/// A tab pane and the rows of each of its pages.
struct Tabs {
    pane: ViewId,
    /// Columns the strip of titles needs.
    titles_width: i16,
    pages: Vec<Vec<Item>>,
}

/// A tab page being built: its title, the container its views go into, and
/// its rows.
struct PageBuild {
    title: String,
    group: ViewGroup,
    items: Vec<Item>,
}

/// The tab pane being built.
struct TabsBuild {
    /// How many groups were open when the tabs started; they stay open
    /// around the tabs.
    base: usize,
    pages: Vec<PageBuild>,
}

/// A group: its frame, its title's width and its rows.
struct Group {
    frame: ViewId,
    title_width: i16,
    items: Vec<Item>,
}

struct FormButton {
    view: ViewId,
    width: i16,
}

/// The settings the layout reads.
#[derive(Clone, Copy)]
struct Style {
    spacing: i16,
    min_field_width: i16,
    label_position: LabelPosition,
    label_align: LabelAlign,
}

/// Lays out a [`Dialog`] from a list of labelled fields, with no
/// coordinates. See the [module documentation](self) for the rules and an
/// example.
///
/// Add rows in order with [`field`](Self::field), [`line`](Self::line),
/// [`row`](Self::row), [`section`](Self::section) and [`gap`](Self::gap),
/// wrap some in [`group`](Self::group) ... [`end_group`](Self::end_group),
/// add buttons with [`button`](Self::button),
/// [`default_button`](Self::default_button) or [`ok_cancel`](Self::ok_cancel),
/// then call [`build`](Self::build) for the finished dialog. The methods that
/// add a view return its typed [`Handle`]: keep it to read the view back from
/// the dialog with [`GroupLike::get`] after the dialog has run.
///
/// `Form` (that is, `Form<()>`) only lays views out. A form for a record type,
/// [`Form::for_record`], also binds fields to the record's members and
/// validates them: see the [`data`] module.
pub struct Form<R = ()> {
    dialog: Dialog,
    title_width: i16,
    items: Vec<Item>,
    /// Groups started and not yet ended, innermost last.
    open: Vec<Group>,
    /// The tab pane started and not yet ended.
    tabs: Option<TabsBuild>,
    /// Every tab pane added to the dialog, to find the views on their pages.
    panes: Vec<ViewId>,
    buttons: Vec<FormButton>,
    style: Style,
    button_align: ButtonAlign,
    resizable: bool,
    /// Fields bound to the record's members, in the order they were added.
    bindings: Vec<Box<dyn data::Binding<R>>>,
    /// Rules over the whole record, run once every field is valid.
    record_rules: Vec<data::RecordRule<R>>,
    /// The row that shows validation errors, in an editor.
    error_line: Option<ViewId>,
}

impl Form {
    /// Start a form for a dialog titled `title`.
    #[must_use]
    pub fn new(title: &str) -> Self {
        Self::with_title(title)
    }

    /// Lay the form out and return the finished dialog: sized to fit,
    /// centred when it is executed or added to the desktop, with the first
    /// field focused.
    ///
    /// The handles returned while building stay valid: use them with
    /// [`GroupLike::get`] / [`GroupLike::get_mut`] on the returned dialog.
    #[must_use]
    pub fn build(mut self) -> Dialog {
        self.lay_out();
        self.dialog
    }
}

impl<R> Form<R> {
    /// An empty form for a dialog titled `title`.
    fn with_title(title: &str) -> Self {
        Self {
            // A placeholder size; `build` gives the dialog its real one.
            dialog: Dialog::new(size(4, 4), title),
            title_width: display_width(title),
            items: Vec::new(),
            open: Vec::new(),
            tabs: None,
            panes: Vec::new(),
            buttons: Vec::new(),
            style: Style {
                spacing: 1,
                min_field_width: 20,
                label_position: LabelPosition::Left,
                label_align: LabelAlign::Left,
            },
            button_align: ButtonAlign::Center,
            resizable: false,
            bindings: Vec::new(),
            record_rules: Vec::new(),
            error_line: None,
        }
    }

    /// Blank rows between two rows of the form (default 1; 0 packs them).
    pub fn spacing(&mut self, rows: i16) -> &mut Self {
        self.style.spacing = rows.max(0);
        self
    }

    /// The narrowest a stretched field on a line of its own may be (default
    /// 20). A wider sized field widens the column for the others.
    pub fn field_width(&mut self, width: i16) -> &mut Self {
        self.style.min_field_width = width.max(1);
        self
    }

    /// Where labels go: to the left of their fields (the default) or above.
    /// Applies to the whole form.
    pub fn label_position(&mut self, position: LabelPosition) -> &mut Self {
        self.style.label_position = position;
        self
    }

    /// How the label column is aligned when labels are on the left: left
    /// (the default) or right, against the fields. Applies to the whole form.
    pub fn label_align(&mut self, align: LabelAlign) -> &mut Self {
        self.style.label_align = align;
        self
    }

    /// Where the buttons go along the bottom row (default centred).
    pub fn button_align(&mut self, align: ButtonAlign) -> &mut Self {
        self.button_align = align;
        self
    }

    /// Let the user resize the dialog (default off). Stretched fields,
    /// full-width rows and groups follow its width; the buttons stay on the
    /// bottom row.
    pub fn resizable(&mut self, resizable: bool) -> &mut Self {
        self.resizable = resizable;
        self
    }

    /// Add a labelled field on a row of its own.
    ///
    /// Mark the label's hot key with `~`, as in `"~N~ame"`: Alt+N then
    /// focuses the field. An empty label leaves the label's place blank.
    /// `view` keeps the size it was built with, or stretches to the field
    /// column if it was built with no width (`Rect::default()`).
    pub fn field<T: View + 'static>(&mut self, label: &str, view: T) -> Handle<T> {
        let (handle, cell) = self.cell(label, view);
        self.items_mut().push(Item::Line(vec![cell]));
        handle
    }

    /// Start a line of fields side by side; add them with [`Line::field`].
    ///
    /// ```
    /// # use turbo_vision::core::geometry::Rect;
    /// # use tv_extensions::forms::{Form, size};
    /// # use turbo_vision::views::input_line::InputLine;
    /// # let mut form = Form::new("T");
    /// let mut line = form.line();
    /// let city = line.field("~C~ity", InputLine::new(Rect::default(), 40));
    /// let zip = line.field("~Z~IP", InputLine::new(size(8, 1), 8));
    /// ```
    ///
    /// The line ends when the next row is added. The first field's label goes
    /// in the label column; the others' labels sit just before their fields.
    /// Stretched fields share the width the sized ones leave.
    pub fn line(&mut self) -> Line<'_, R> {
        self.items_mut().push(Item::Line(Vec::new()));
        Line { form: self }
    }

    /// Add a view spanning the whole form (or group), with no label: a check
    /// box with a long caption, a note, a list. It keeps its size, or
    /// stretches to the full width if it was built with no width.
    pub fn row<T: View + 'static>(&mut self, view: T) -> Handle<T> {
        let extent = Extent::of(&view);
        let handle = Handle::from_id(self.add_view(Box::new(view)));
        self.items_mut().push(Item::Full {
            view: handle.id(),
            extent,
        });
        handle
    }

    /// Add a heading over the rows that follow, with a blank row above it
    /// (unless it is the first row).
    pub fn section(&mut self, title: &str) -> &mut Self {
        let view = self.add_view(Box::new(StaticText::new(Rect::default(), title)));
        let width = display_width(title);
        self.items_mut().push(Item::Section { view, width });
        self
    }

    /// Add `rows` blank rows (on top of the usual spacing).
    pub fn gap(&mut self, rows: i16) -> &mut Self {
        self.items_mut().push(Item::Gap(rows.max(0)));
        self
    }

    /// Start a group: the rows added until [`end_group`](Self::end_group)
    /// are drawn inside a box titled `title` (empty for none), as wide as the
    /// form. The group lines up its own labels. Groups can nest; `build`
    /// closes any group left open.
    pub fn group(&mut self, title: &str) -> &mut Self {
        let frame = self.add_view(Box::new(GroupBox::new(Rect::default(), title)));
        self.open.push(Group {
            frame,
            title_width: display_width(title),
            items: Vec::new(),
        });
        self
    }

    /// End the group started last. Does nothing when no group is open.
    pub fn end_group(&mut self) -> &mut Self {
        let base = self.tabs.as_ref().map_or(0, |t| t.base);
        if self.open.len() > base
            && let Some(group) = self.open.pop()
        {
            self.items_mut().push(Item::Group(group));
        }
        self
    }

    /// Start a strip of tabs: the rows added after each [`tab`](Self::tab)
    /// until [`end_tabs`](Self::end_tabs) go on that tab's page, and only the
    /// active page is shown. The pane is as wide as the form and as high as
    /// its tallest page. Tabs cannot nest, and the fields on a page are
    /// reached through a record form's bindings, not through `Handle`s on
    /// the dialog.
    pub fn tabs(&mut self) -> &mut Self {
        if self.tabs.is_none() {
            self.tabs = Some(TabsBuild {
                base: self.open.len(),
                pages: Vec::new(),
            });
        }
        self
    }

    /// Start a page titled `title` (a `~` marks its Alt hot key), closing
    /// any group left open on the previous page. Starts the tabs if
    /// [`tabs`](Self::tabs) was not called.
    pub fn tab(&mut self, title: &str) -> &mut Self {
        self.tabs();
        let base = self.tabs.as_ref().map_or(0, |t| t.base);
        while self.open.len() > base {
            self.end_group();
        }
        if let Some(tabs) = self.tabs.as_mut() {
            tabs.pages.push(PageBuild {
                title: title.to_string(),
                group: ViewGroup::new(Rect::default()),
                items: Vec::new(),
            });
        }
        self
    }

    /// End the tabs started last. Does nothing when none are open or no page
    /// was added.
    pub fn end_tabs(&mut self) -> &mut Self {
        let Some(base) = self.tabs.as_ref().map(|t| t.base) else {
            return self;
        };
        while self.open.len() > base {
            self.end_group();
        }
        let Some(build) = self.tabs.take() else {
            return self;
        };
        if build.pages.is_empty() {
            return self;
        }
        let mut pane = TabbedPane::new(Rect::default());
        let mut titles_width = 0;
        let mut pages = Vec::new();
        for page in build.pages {
            titles_width += display_width(&page.title) + TAB_TITLE_CHROME;
            pane.add_page(&page.title, page.group);
            pages.push(page.items);
        }
        let pane = self.dialog.add_boxed(Box::new(pane));
        self.panes.push(pane);
        self.items_mut().push(Item::Tabs(Tabs {
            pane,
            titles_width,
            pages,
        }));
        self
    }

    /// Add a button on the bottom row that sends `command` when pressed.
    /// Buttons appear left to right in the order they are added.
    pub fn button(&mut self, title: &str, command: CommandId) -> Handle<Button> {
        self.add_button(title, command, false)
    }

    /// Add the default button: the one Enter presses when the focused
    /// control is not a button.
    pub fn default_button(&mut self, title: &str, command: CommandId) -> Handle<Button> {
        self.add_button(title, command, true)
    }

    /// Add the usual pair: **OK** (`CM_OK`, the default) and **Cancel**
    /// (`CM_CANCEL`). Either one closes the dialog, and `execute` returns
    /// its command.
    pub fn ok_cancel(&mut self) -> &mut Self {
        self.default_button("~O~K", CM_OK);
        self.button("Cancel", CM_CANCEL);
        self
    }

    fn add_button(&mut self, title: &str, command: CommandId, is_default: bool) -> Handle<Button> {
        let handle =
            self.dialog
                .add_typed(Button::new(Rect::default(), title, command, is_default));
        self.buttons.push(FormButton {
            view: handle.id(),
            width: (display_width(title) + 4).max(MIN_BUTTON_WIDTH),
        });
        handle
    }

    /// Add `view` to the dialog with its label, linked to it.
    fn cell<T: View + 'static>(&mut self, label: &str, view: T) -> (Handle<T>, Cell) {
        let extent = Extent::of(&view);
        let handle = Handle::from_id(self.add_view(Box::new(view)));
        let label = (!label.is_empty()).then(|| {
            let mut l = Label::new(Rect::default(), label);
            l.set_link(handle.id());
            (self.add_view(Box::new(l)), display_width(label))
        });
        let cell = Cell {
            label,
            view: handle.id(),
            extent,
        };
        (handle, cell)
    }

    /// Where new rows go: the innermost open group, or the form itself.
    fn items_mut(&mut self) -> &mut Vec<Item> {
        let base = self.tabs.as_ref().map_or(0, |t| t.base);
        if self.open.len() <= base
            && let Some(page) = self.tabs.as_mut().and_then(|t| t.pages.last_mut())
        {
            return &mut page.items;
        }
        match self.open.last_mut() {
            Some(group) => &mut group.items,
            None => &mut self.items,
        }
    }

    /// Add `view` where new views go: the open tab page, or the dialog.
    fn add_view(&mut self, view: Box<dyn View>) -> ViewId {
        match self.tabs.as_mut().and_then(|t| t.pages.last_mut()) {
            Some(page) => page.group.add_boxed(view),
            None => self.dialog.add_boxed(view),
        }
    }

    /// Close any open group, place every view, size the dialog and focus its
    /// first field.
    fn lay_out(&mut self) {
        self.end_tabs();
        while !self.open.is_empty() {
            self.end_group();
        }
        let layout = self.measure();

        // Size the dialog first. Its children carry no grow bits yet, so the
        // resize leaves them alone; they are placed just below.
        self.dialog
            .set_bounds(size(layout.width + 2, layout.height + 2));
        for (id, bounds, grow) in &layout.places {
            if let Some(view) = find_view_mut(&mut self.dialog, *id) {
                view.set_bounds(*bounds);
                if self.resizable {
                    view.set_grow_mode(*grow);
                }
            }
        }

        self.dialog
            .set_options(self.dialog.options() | Options::CENTERED);
        self.dialog.set_resizable(self.resizable);
        self.dialog.set_initial_focus();
    }

    /// Work out every view's place, and the size of the dialog's interior.
    fn measure(&self) -> Layout {
        let style = self.style;
        let gaps = i16::try_from(self.buttons.len().saturating_sub(1)).unwrap_or(0);
        let buttons = self.buttons.iter().map(|b| b.width).sum::<i16>() + BUTTON_GAP * gaps;
        let content = min_width(&self.items, style)
            .max(buttons)
            .max(self.title_width + TITLE_CHROME - 2 - 2 * MARGIN_X)
            .max(1);

        let mut places = Vec::new();
        let area = Area {
            x: MARGIN_X,
            y: MARGIN_TOP,
            width: content,
        };
        let mut rows_end = place(&self.items, area, style, &mut places);
        // An editor's error line: after a blank row, just above the buttons.
        if let Some(line) = self.error_line {
            let y = rows_end + 1;
            let r = Rect::new(MARGIN_X, y, MARGIN_X + content, y + 1);
            places.push((line, r, Grow::HI_X | Grow::LO_Y | Grow::HI_Y));
            rows_end = y;
        }
        // A blank row under the rows (the error line is one), and the
        // buttons below it if any.
        let height = if self.buttons.is_empty() {
            rows_end + 1
        } else {
            let top = if self.error_line.is_some() {
                rows_end + 1
            } else if rows_end == MARGIN_TOP {
                MARGIN_TOP
            } else {
                rows_end + 1
            };
            self.place_buttons(content, buttons, top, &mut places);
            top + BUTTON_HEIGHT
        };
        Layout {
            width: content + 2 * MARGIN_X,
            height,
            places,
        }
    }

    /// Place the buttons on one row starting at `top`.
    fn place_buttons(&self, content: i16, buttons: i16, top: i16, places: &mut Vec<Place>) {
        let (offset, grow) = match self.button_align {
            ButtonAlign::Center => ((content - buttons) / 2, Grow::LO_Y | Grow::HI_Y),
            ButtonAlign::Right => (content - buttons, Grow::ALL),
        };
        let mut x = MARGIN_X + offset;
        for b in &self.buttons {
            let r = Rect::new(x, top, x + b.width, top + BUTTON_HEIGHT);
            places.push((b.view, r, grow));
            x += b.width + BUTTON_GAP;
        }
    }
}

/// Adds fields to a line started with [`Form::line`].
pub struct Line<'a, R = ()> {
    form: &'a mut Form<R>,
}

impl<R> Line<'_, R> {
    /// Add a labelled field to the right of the line's previous ones. The
    /// rules are those of [`Form::field`]; an empty label leaves no gap.
    pub fn field<T: View + 'static>(&mut self, label: &str, view: T) -> Handle<T> {
        let (handle, cell) = self.form.cell(label, view);
        if let Some(Item::Line(cells)) = self.form.items_mut().last_mut() {
            cells.push(cell);
        }
        handle
    }
}

/// A view, its place in the dialog's interior, and the grow bits it takes
/// when the dialog is resizable.
type Place = (ViewId, Rect, Grow);

/// The finished layout: the interior's size and each view's place in it.
struct Layout {
    width: i16,
    height: i16,
    places: Vec<Place>,
}

/// The space a list of rows is laid out in: its left edge, its first row
/// and its width.
#[derive(Clone, Copy)]
struct Area {
    x: i16,
    y: i16,
    width: i16,
}

/// The width of the label column of a list of rows, gap included: the
/// longest first-field label, or 0 when labels go above their fields.
fn label_column(items: &[Item], style: Style) -> i16 {
    if style.label_position == LabelPosition::Above {
        return 0;
    }
    items
        .iter()
        .filter_map(|item| match item {
            Item::Line(cells) => cells.first()?.label.map(|(_, w)| w),
            _ => None,
        })
        .max()
        .map_or(0, |w| w + LABEL_GAP)
}

/// How narrow a field can be: its own width, or a stretched field's minimum.
fn field_min(cell: &Cell, alone: bool, style: Style) -> i16 {
    cell.extent.width.unwrap_or(if alone {
        style.min_field_width
    } else {
        MIN_LINE_FIELD
    })
}

/// A label's width, or 0 for a cell with none.
fn label_width(cell: &Cell) -> i16 {
    cell.label.map_or(0, |(_, w)| w)
}

/// The columns a cell needs besides its field when labels are on the left:
/// its label, unless it is the first cell (whose label is in the column).
fn inline_label(cell: &Cell, first: bool) -> i16 {
    match (first, cell.label) {
        (false, Some((_, w))) => w + LABEL_GAP,
        _ => 0,
    }
}

/// The narrowest width that fits every row of `items`.
fn min_width(items: &[Item], style: Style) -> i16 {
    let column = label_column(items, style);
    items
        .iter()
        .map(|item| match item {
            Item::Line(cells) => {
                let alone = cells.len() == 1;
                let fields: i16 = cells
                    .iter()
                    .enumerate()
                    .map(|(i, c)| match style.label_position {
                        LabelPosition::Left => inline_label(c, i == 0) + field_min(c, alone, style),
                        LabelPosition::Above => field_min(c, alone, style).max(label_width(c)),
                    })
                    .sum();
                column + fields + CELL_GAP * gaps(cells.len())
            }
            Item::Full { extent, .. } => extent.width.unwrap_or(1),
            Item::Section { width, .. } => *width,
            Item::Gap(_) => 0,
            Item::Group(group) => (min_width(&group.items, style) + 2 * GROUP_PAD_X)
                .max(group.title_width + GROUP_TITLE_CHROME),
            Item::Tabs(tabs) => {
                let pages = tabs.pages.iter().map(|p| min_width(p, style)).max();
                (pages.unwrap_or(0) + 2 + 2 * PAGE_PAD_X).max(tabs.titles_width + 2)
            }
        })
        .max()
        .unwrap_or(0)
}

/// The gaps between `n` things side by side.
fn gaps(n: usize) -> i16 {
    i16::try_from(n.saturating_sub(1)).unwrap_or(0)
}

/// Place `items` top to bottom in `area`; returns the first row below them,
/// or `area.y` if nothing was placed.
fn place(items: &[Item], area: Area, style: Style, places: &mut Vec<Place>) -> i16 {
    let column = label_column(items, style);
    let mut bottom = area.y; // the first free row after the last row
    let mut first = true;
    for item in items {
        if matches!(item, Item::Line(cells) if cells.is_empty()) {
            continue; // a line nobody added a field to
        }
        let mut y = if first {
            area.y
        } else {
            bottom + style.spacing
        };
        match item {
            Item::Line(cells) => {
                bottom = place_line(cells, Area { y, ..area }, column, style, places);
            }
            Item::Full { view, extent } => {
                let (w, grow) = stretch(extent.width, area.width);
                let r = Rect::new(area.x, y, area.x + w, y + extent.height);
                places.push((*view, r, grow));
                bottom = y + extent.height;
            }
            Item::Section { view, width } => {
                if !first {
                    y += 1; // the blank row above a heading
                }
                let r = Rect::new(area.x, y, area.x + width, y + 1);
                places.push((*view, r, Grow::empty()));
                bottom = y + 1;
            }
            Item::Gap(rows) => {
                // Blank rows on top of the spacing on either side of it:
                // the next row adds its own spacing after them.
                bottom += rows;
            }
            Item::Group(group) => {
                let inside = Area {
                    x: area.x + GROUP_PAD_X,
                    y: y + 1,
                    width: area.width - 2 * GROUP_PAD_X,
                };
                // The bottom border goes on the first row below the contents.
                bottom = place(&group.items, inside, style, places).max(inside.y) + 1;
                let r = Rect::new(area.x, y, area.x + area.width, bottom);
                places.push((group.frame, r, Grow::HI_X));
            }
            Item::Tabs(tabs) => {
                // Every page's rows are placed inside its page, whose corner
                // is the pane's inner corner: one blank row under the strip.
                let inside = Area {
                    x: PAGE_PAD_X,
                    y: 1,
                    width: area.width - 2 - 2 * PAGE_PAD_X,
                };
                // The pane goes first: its bounds also move its pages.
                let at = places.len();
                places.push((tabs.pane, Rect::default(), Grow::HI_X));
                let rows = tabs
                    .pages
                    .iter()
                    .map(|page| place(page, inside, style, places).max(inside.y))
                    .max()
                    .unwrap_or(inside.y);
                // A blank row under the rows, then the pane's border.
                bottom = y + TAB_HEADER_ROWS + rows + 2;
                places[at].1 = Rect::new(area.x, y, area.x + area.width, bottom);
            }
        }
        first = false;
    }
    bottom
}

/// Place one line of cells at `area.y`, its fields starting after the label
/// column `column`; returns the first row below the line.
fn place_line(
    cells: &[Cell],
    area: Area,
    column: i16,
    style: Style,
    places: &mut Vec<Place>,
) -> i16 {
    let above = style.label_position == LabelPosition::Above;

    // The columns taken by labels, sized fields and gaps; the stretched
    // fields share what is left.
    let fixed: i16 = cells
        .iter()
        .enumerate()
        .map(|(i, c)| match (above, c.extent.width) {
            (false, w) => inline_label(c, i == 0) + w.unwrap_or(0),
            (true, Some(w)) => w.max(label_width(c)),
            (true, None) => 0,
        })
        .sum();
    let stretched = cells.iter().filter(|c| c.extent.width.is_none()).count();
    let mut shares = shares(
        area.width - column - fixed - CELL_GAP * gaps(cells.len()),
        stretched,
    );
    // The last stretched field takes a resize; what is right of it moves.
    let growing = cells.iter().rposition(|c| c.extent.width.is_none());

    let field_y = area.y + i16::from(above && cells.iter().any(|c| c.label.is_some()));
    let mut x = area.x + column;
    let mut bottom = field_y + 1;
    for (i, cell) in cells.iter().enumerate() {
        let (field_grow, label_grow) = match growing {
            Some(k) if i == k => (Grow::HI_X, Grow::empty()),
            Some(k) if i > k => (Grow::LO_X | Grow::HI_X, Grow::LO_X | Grow::HI_X),
            _ => (Grow::empty(), Grow::empty()),
        };
        let width = match cell.extent.width {
            Some(w) => w,
            // Above its field, a label is never cut by a narrow share.
            None if above => shares.next().unwrap_or(1).max(label_width(cell)),
            None => shares.next().unwrap_or(1),
        }
        .max(1);

        if let Some((label, w)) = cell.label {
            let r = if above {
                Rect::new(x, area.y, x + w, area.y + 1)
            } else if i == 0 {
                // In the label column, against its left or right edge.
                let left = match style.label_align {
                    LabelAlign::Left => area.x,
                    LabelAlign::Right => area.x + column - LABEL_GAP - w,
                };
                Rect::new(left, area.y, left + w, area.y + 1)
            } else {
                let r = Rect::new(x, area.y, x + w, area.y + 1);
                x += w + LABEL_GAP;
                r
            };
            places.push((label, r, label_grow));
        }
        let r = Rect::new(x, field_y, x + width, field_y + cell.extent.height);
        places.push((cell.view, r, field_grow));
        bottom = bottom.max(field_y + cell.extent.height);
        let cell_width = if above {
            width.max(label_width(cell))
        } else {
            width
        };
        x += cell_width + CELL_GAP;
    }
    bottom
}

/// `spare` columns split between `n` stretched fields: equal shares, the
/// remainder to the last.
fn shares(spare: i16, n: usize) -> impl Iterator<Item = i16> {
    let count = i16::try_from(n).unwrap_or(1).max(1);
    let each = spare / count;
    let last = spare - each * (count - 1);
    (0..n).map(move |i| if i + 1 == n { last } else { each })
}

/// A view's width in a column `column` wide: its own, or the column's (and
/// then it follows the dialog's width).
fn stretch(width: Option<i16>, column: i16) -> (i16, Grow) {
    match width {
        Some(w) => (w, Grow::empty()),
        None => (column, Grow::HI_X),
    }
}

/// The view `id` in the dialog or on one of its tab pages.
pub(crate) fn find_view_mut(dialog: &mut Dialog, id: ViewId) -> Option<&mut (dyn View + '_)> {
    if dialog.child_by_id(id).is_some() {
        return dialog.child_by_id_mut(id);
    }
    let (pane, page) = locate(dialog, id)?;
    let pane = dialog.group_mut().child_at_mut(pane);
    let pane = pane.as_any_mut().downcast_mut::<TabbedPane>()?;
    pane.page_mut(page)?.child_by_id_mut(id)
}

/// Where `id` is among the dialog's tab panes: the pane's index in the
/// dialog and the page's in the pane.
fn locate(dialog: &mut Dialog, id: ViewId) -> Option<(usize, usize)> {
    for i in 0..dialog.group().len() {
        let child = dialog.group_mut().child_at_mut(i);
        let Some(pane) = child.as_any_mut().downcast_mut::<TabbedPane>() else {
            continue;
        };
        for page in 0..pane.page_count() {
            if pane
                .page_mut(page)
                .is_some_and(|p| p.child_by_id(id).is_some())
            {
                return Some((i, page));
            }
        }
    }
    None
}

/// Give the focus to the view `id`, showing its tab page first if it is on
/// one. Returns whether it was found.
pub(crate) fn focus_view(dialog: &mut Dialog, id: ViewId) -> bool {
    if dialog.group_mut().focus_by_view_id(id) {
        return true;
    }
    let Some((pane, page)) = locate(dialog, id) else {
        return false;
    };
    let Some(pane_id) = dialog.group().view_id_at(pane) else {
        return false;
    };
    if let Some(p) = dialog
        .group_mut()
        .child_at_mut(pane)
        .as_any_mut()
        .downcast_mut::<TabbedPane>()
    {
        p.set_active(page);
    }
    dialog.group_mut().focus_by_view_id(pane_id);
    dialog
        .group_mut()
        .child_at_mut(pane)
        .as_any_mut()
        .downcast_mut::<TabbedPane>()
        .and_then(|p| p.page_mut(page))
        .is_some_and(|p| p.focus_by_view_id(id))
}

/// The view with the focus, looking inside the active tab page.
pub(crate) fn focused_view(dialog: &mut Dialog) -> Option<ViewId> {
    let id = dialog.group().focused_view_id()?;
    let inside = dialog
        .child_by_id_mut(id)
        .and_then(|v| v.as_any_mut().downcast_mut::<TabbedPane>())
        .and_then(|pane| pane.active_page_mut())
        .and_then(|page| page.focused_view_id());
    inside.or(Some(id))
}

/// Columns `text` takes on screen: its characters, without the `~` that
/// mark a hot key.
fn display_width(text: &str) -> i16 {
    i16::try_from(text.chars().filter(|&c| c != '~').count()).unwrap_or(i16::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use turbo_vision::core::event::{Event, EventType, KB_ALT_E};
    use turbo_vision::views::checkbox::CheckBox;
    use turbo_vision::views::input_line::InputLine;
    use turbo_vision::views::memo::Memo;

    fn input() -> InputLine {
        InputLine::new(Rect::default(), 40)
    }

    fn bounds<T: View + 'static>(d: &Dialog, h: Handle<T>) -> Rect {
        d.get(h).expect("the handle finds its view").bounds()
    }

    /// Every label in the dialog, with the view it is linked to.
    fn labels(d: &Dialog) -> Vec<(String, Rect, Option<ViewId>)> {
        (0..d.child_count())
            .filter_map(|i| {
                let v = d.child_at(i);
                let l = v.as_any().downcast_ref::<Label>()?;
                Some((l.text().to_string(), l.bounds(), v.label_link()))
            })
            .collect()
    }

    #[test]
    fn labels_share_a_column_and_fields_line_up_beside_them() {
        let mut form = Form::new("T");
        let name = form.field("~N~ame", input());
        let city = form.field("~P~ostal town", input());
        let d = form.build();

        let (n, c) = (bounds(&d, name), bounds(&d, city));
        assert_eq!(n.a.x, c.a.x, "fields share a column");
        // "Postal town" is the longest label: 11 columns, then one gap.
        assert_eq!(n.a.x, MARGIN_X + 11 + LABEL_GAP);
        assert_eq!(
            (n.a.y, c.a.y),
            (MARGIN_TOP, MARGIN_TOP + 2),
            "one blank row between"
        );

        let labels = labels(&d);
        assert_eq!(labels.len(), 2);
        for ((text, rect, link), field) in labels.iter().zip([name.id(), city.id()]) {
            assert_eq!(rect.a.x, MARGIN_X, "{text} starts the label column");
            assert_eq!(*link, Some(field), "{text} is linked to its field");
        }
        assert_eq!(labels[1].1.a.y, c.a.y, "a label sits on its field's row");
    }

    #[test]
    fn a_sized_view_keeps_its_size_and_an_unsized_one_stretches() {
        let mut form = Form::new("T");
        let zip = form.field("~Z~IP", InputLine::new(size(8, 1), 8));
        let name = form.field("~N~ame", input());
        let notes = form.field("N~o~tes", Memo::new(size(0, 3)));
        let d = form.build();

        assert_eq!(bounds(&d, zip).width(), 8);
        assert_eq!(bounds(&d, name).width(), 20, "the default field column");
        let notes = bounds(&d, notes);
        assert_eq!(
            (notes.width(), notes.height()),
            (20, 3),
            "height kept, width stretched"
        );
    }

    #[test]
    fn a_wide_sized_field_widens_the_column_for_the_stretched_ones() {
        let mut form = Form::new("T");
        form.field("A", InputLine::new(size(30, 1), 30));
        let b = form.field("B", input());
        let d = form.build();
        assert_eq!(bounds(&d, b).width(), 30);
    }

    #[test]
    fn the_dialog_is_sized_to_fit_and_centred() {
        let mut form = Form::new("T");
        let name = form.field("~N~ame", input()); // label 4 + gap 1 + field 20
        form.ok_cancel();
        let d = form.build();

        let content = 4 + LABEL_GAP + 20;
        let b = d.bounds();
        assert_eq!(b.width(), content + 2 * MARGIN_X + 2);
        // Row 1 the field, row 2 blank, rows 3-4 the buttons, plus the frame.
        assert_eq!(b.height(), MARGIN_TOP + 1 + 1 + BUTTON_HEIGHT + 2);
        assert!(d.options().contains(Options::CENTERED));
        assert!(
            d.get(name).is_some_and(View::is_focused),
            "first field focused"
        );
    }

    #[test]
    fn a_long_title_widens_the_dialog() {
        let title = "A rather long dialog title";
        let mut form = Form::new(title);
        form.field("X", input());
        let d = form.build();
        assert!(d.bounds().width() >= display_width(title) + TITLE_CHROME);
    }

    #[test]
    fn buttons_are_centred_on_the_bottom_row_and_ok_is_the_default() {
        let mut form = Form::new("T");
        form.field("~N~ame", input());
        form.ok_cancel();
        let d = form.build();

        let buttons: Vec<&Button> = (0..d.child_count())
            .filter_map(|i| d.child_at(i).as_any().downcast_ref::<Button>())
            .collect();
        let [ok, cancel] = buttons[..] else {
            panic!("two buttons")
        };
        assert_eq!((ok.command(), ok.is_default()), (CM_OK, true));
        assert_eq!((cancel.command(), cancel.is_default()), (CM_CANCEL, false));

        let (o, c) = (ok.bounds(), cancel.bounds());
        assert_eq!(o.a.y, c.a.y, "one row");
        assert_eq!(c.a.x, o.b.x + BUTTON_GAP);
        let interior = d.bounds().width() - 2;
        let (left, right) = (o.a.x, interior - c.b.x);
        assert!(
            (left - right).abs() <= 1,
            "centred: {left} left, {right} right"
        );
        assert_eq!(
            c.b.y,
            d.bounds().height() - 2,
            "the last row of the interior"
        );
    }

    #[test]
    fn right_aligned_buttons_end_at_the_margin() {
        let mut form = Form::new("T");
        form.field("~N~ame", input());
        form.button_align(ButtonAlign::Right);
        let close = form.default_button("~C~lose", CM_OK);
        let d = form.build();
        assert_eq!(bounds(&d, close).b.x, d.bounds().width() - 2 - MARGIN_X);
    }

    #[test]
    fn an_empty_label_leaves_the_column_blank_but_keeps_the_alignment() {
        let mut form = Form::new("T");
        let name = form.field("~N~ame", input());
        let vip = form.field("", CheckBox::new(Rect::default(), "~V~IP"));
        let d = form.build();
        assert_eq!(labels(&d).len(), 1, "no label for the check box");
        assert_eq!(bounds(&d, vip).a.x, bounds(&d, name).a.x);
    }

    #[test]
    fn rows_and_sections_span_the_form() {
        let mut form = Form::new("T");
        let name = form.field("~N~ame", input());
        form.section("Notes");
        let notes = form.row(Memo::new(size(0, 3)));
        let d = form.build();

        let (n, m) = (bounds(&d, name), bounds(&d, notes));
        assert_eq!(m.a.x, MARGIN_X);
        assert_eq!(m.b.x, n.b.x, "as wide as the form");
        // Name on row 1; blank, extra blank, heading on row 4; blank; notes.
        assert_eq!(m.a.y, n.a.y + 5);
    }

    #[test]
    fn spacing_and_gaps_set_the_blank_rows() {
        let mut form = Form::new("T");
        form.spacing(0);
        let a = form.field("A", input());
        let b = form.field("B", input());
        form.gap(2);
        let c = form.field("C", input());
        let d = form.build();
        assert_eq!(bounds(&d, b).a.y, bounds(&d, a).a.y + 1, "packed");
        assert_eq!(bounds(&d, c).a.y, bounds(&d, b).a.y + 3, "two blank rows");
    }

    #[test]
    fn a_resizable_form_stretches_its_fields_and_keeps_the_buttons_at_the_bottom() {
        let mut form = Form::new("T");
        form.resizable(true);
        let zip = form.field("~Z~IP", InputLine::new(size(8, 1), 8));
        let name = form.field("~N~ame", input());
        let ok = form.default_button("~O~K", CM_OK);
        let mut d = form.build();

        let (zip0, name0, ok0) = (bounds(&d, zip), bounds(&d, name), bounds(&d, ok));
        let b = d.bounds();
        d.set_bounds(Rect::new(b.a.x, b.a.y, b.b.x + 10, b.b.y + 4));

        assert_eq!(bounds(&d, zip), zip0, "a sized field stays put");
        assert_eq!(bounds(&d, name).width(), name0.width() + 10);
        assert_eq!(
            bounds(&d, ok).a.y,
            ok0.a.y + 4,
            "buttons stay on the bottom row"
        );
    }

    #[test]
    fn a_label_hot_key_focuses_its_field() {
        let mut form = Form::new("T");
        let name = form.field("~N~ame", input());
        let email = form.field("~E~mail", input());
        let mut d = form.build();
        assert!(d.get(name).is_some_and(View::is_focused));

        let mut alt_e = Event::keyboard(KB_ALT_E);
        d.handle_event(&mut alt_e);
        assert!(d.get(email).is_some_and(View::is_focused));
        assert_eq!(alt_e.what, EventType::Nothing);
    }

    #[test]
    fn executing_the_dialog_centres_it_on_the_desktop() {
        let mut app = turbo_vision::app::Application::with_terminal(
            turbo_vision::test_util::test_terminal(80, 25),
        );
        let mut form = Form::new("T");
        form.field("~N~ame", input());
        let mut d = form.build();
        d.prepare_modal(&mut app);

        let (desk, b) = (app.desktop.get_bounds(), d.bounds());
        assert!(
            (b.a.x - desk.a.x - (desk.b.x - b.b.x)).abs() <= 1,
            "centred across"
        );
        assert!(
            (b.a.y - desk.a.y - (desk.b.y - b.b.y)).abs() <= 1,
            "centred down"
        );
    }

    // ---- lines: several fields side by side ----

    #[test]
    fn a_line_puts_fields_side_by_side_with_their_labels_between() {
        let mut form = Form::new("T");
        let street = form.field("~S~treet", input());
        let mut line = form.line();
        let city = line.field("~C~ity", input());
        let zip = line.field("~Z~IP", InputLine::new(size(8, 1), 8));
        let d = form.build();

        let (s, c, z) = (bounds(&d, street), bounds(&d, city), bounds(&d, zip));
        assert_eq!(c.a.y, z.a.y, "one row");
        assert_eq!(c.a.x, s.a.x, "the first field is in the field column");
        assert_eq!(z.width(), 8, "a sized field keeps its size");
        // City, a gap, the ZIP label, a gap, then the ZIP field.
        assert_eq!(z.a.x, c.b.x + CELL_GAP + 3 + LABEL_GAP);
        assert_eq!(z.b.x, s.b.x, "the line ends where the form does");

        let zip_label = labels(&d).into_iter().find(|(t, ..)| t == "~Z~IP").unwrap();
        assert_eq!(zip_label.1.a.x, c.b.x + CELL_GAP);
        assert_eq!(zip_label.2, Some(zip.id()), "linked to its field");
    }

    #[test]
    fn stretched_fields_in_a_line_share_the_width() {
        let mut form = Form::new("T");
        form.field("~A~ddress", InputLine::new(size(40, 1), 40));
        let mut line = form.line();
        let first = line.field("~F~irst", input());
        let last = line.field("~L~ast", input());
        let d = form.build();
        let (f, l) = (bounds(&d, first), bounds(&d, last));
        assert!(
            (f.width() - l.width()).abs() <= 1,
            "{} and {}",
            f.width(),
            l.width()
        );
        assert!(f.width() >= MIN_LINE_FIELD);
    }

    #[test]
    fn a_line_with_no_fields_takes_no_room() {
        let mut form = Form::new("T");
        let a = form.field("A", input());
        let _ = form.line();
        let b = form.field("B", input());
        let d = form.build();
        assert_eq!(bounds(&d, b).a.y, bounds(&d, a).a.y + 2);
    }

    #[test]
    fn tab_goes_through_a_line_left_to_right() {
        let mut form = Form::new("T");
        let mut line = form.line();
        let city = line.field("~C~ity", input());
        let zip = line.field("~Z~IP", InputLine::new(size(8, 1), 8));
        let mut d = form.build();
        assert!(d.get(city).is_some_and(View::is_focused));
        d.handle_event(&mut Event::keyboard(turbo_vision::core::event::KB_TAB));
        assert!(d.get(zip).is_some_and(View::is_focused));
    }

    // ---- groups ----

    fn group_boxes(d: &Dialog) -> Vec<(String, Rect)> {
        (0..d.child_count())
            .filter_map(|i| d.child_at(i).as_any().downcast_ref::<GroupBox>())
            .map(|g| (g.title().to_string(), g.bounds()))
            .collect()
    }

    #[test]
    fn a_group_draws_a_box_around_its_rows() {
        let mut form = Form::new("T");
        let name = form.field("~N~ame", input());
        form.group("Address");
        let street = form.field("~S~treet", input());
        let city = form.field("~C~ity", input());
        form.end_group();
        let after = form.field("~P~hone", input());
        let d = form.build();

        let boxes = group_boxes(&d);
        let [(title, frame)] = boxes.as_slice() else {
            panic!("one group box")
        };
        assert_eq!(title, "Address");
        let (name, street, city, after) = (
            bounds(&d, name),
            bounds(&d, street),
            bounds(&d, city),
            bounds(&d, after),
        );
        assert_eq!(frame.a.y, name.b.y + 1, "one blank row above the box");
        assert_eq!(
            street.a.y,
            frame.a.y + 1,
            "the first row is under the top edge"
        );
        assert_eq!(
            frame.b.y,
            city.b.y + 1,
            "the bottom edge is under the last row"
        );
        assert_eq!(after.a.y, frame.b.y + 1, "one blank row below the box");
        assert_eq!(
            (frame.a.x, frame.b.x),
            (MARGIN_X, name.b.x),
            "as wide as the form"
        );
        assert!(
            street.a.x > frame.a.x && street.b.x < frame.b.x,
            "inside the box"
        );
    }

    #[test]
    fn a_group_lines_up_its_own_labels() {
        let mut form = Form::new("T");
        let outside = form.field("~L~ong outside label", input());
        form.group("G");
        let inside = form.field("~X~", input());
        form.end_group();
        let d = form.build();
        // The group's label column fits "X", not the long label outside.
        assert_eq!(
            bounds(&d, inside).a.x,
            MARGIN_X + GROUP_PAD_X + 1 + LABEL_GAP
        );
        assert!(bounds(&d, outside).a.x > bounds(&d, inside).a.x);
    }

    #[test]
    fn groups_nest_and_build_closes_any_left_open() {
        let mut form = Form::new("T");
        form.group("Outer");
        form.group("Inner");
        let deep = form.field("~D~eep", input());
        form.end_group();
        form.group("Unclosed");
        form.field("~U~", input());
        form.end_group().end_group(); // the second closes "Outer"
        form.end_group(); // nothing open: ignored
        form.group("Left open");
        let last = form.field("~L~ast", input());
        let d = form.build();

        let boxes = group_boxes(&d);
        let frame = |t: &str| boxes.iter().find(|(title, _)| title == t).unwrap().1;
        let (outer, inner) = (frame("Outer"), frame("Inner"));
        assert!(
            inner.a.x > outer.a.x && inner.b.x < outer.b.x,
            "nested inside"
        );
        assert!(inner.a.y > outer.a.y && inner.b.y < outer.b.y);
        assert!(bounds(&d, deep).a.x > inner.a.x);
        let open = frame("Left open");
        assert!(open.a.y > outer.b.y, "after the outer group");
        assert_eq!(open.b.y, bounds(&d, last).b.y + 1, "closed by build");
    }

    #[test]
    fn a_long_group_title_widens_the_form() {
        let mut form = Form::new("T");
        form.group("A very long group title indeed");
        form.field("A", InputLine::new(size(5, 1), 5));
        let d = form.build();
        let (_, frame) = &group_boxes(&d)[0];
        assert!(
            frame.width() >= display_width("A very long group title indeed") + GROUP_TITLE_CHROME
        );
    }

    // ---- label position and alignment ----

    #[test]
    fn labels_above_sit_on_the_row_over_their_fields() {
        let mut form = Form::new("T");
        form.label_position(LabelPosition::Above);
        let name = form.field("~N~ame", input());
        let mut line = form.line();
        let first = line.field("~F~irst", input());
        let last = line.field("~L~ast", input());
        let d = form.build();

        let labels = labels(&d);
        for (text, rect, link) in &labels {
            let field = d.child_by_id(link.unwrap()).unwrap().bounds();
            assert_eq!(
                (rect.a.x, rect.a.y),
                (field.a.x, field.a.y - 1),
                "{text} above"
            );
        }
        let (n, f, l) = (bounds(&d, name), bounds(&d, first), bounds(&d, last));
        assert_eq!(n.a.x, MARGIN_X, "no label column");
        assert_eq!(f.a.y, l.a.y);
        assert_eq!(
            f.a.y,
            n.b.y + 2,
            "label row, then field row, after one blank"
        );
    }

    #[test]
    fn right_aligned_labels_end_against_their_fields() {
        let mut form = Form::new("T");
        form.label_align(LabelAlign::Right);
        let name = form.field("~N~ame", input());
        let email = form.field("~E~mail address", input());
        let d = form.build();
        for (text, rect, link) in labels(&d) {
            let field = d.child_by_id(link.unwrap()).unwrap().bounds();
            assert_eq!(rect.b.x, field.a.x - LABEL_GAP, "{text} ends at its field");
        }
        assert_eq!(bounds(&d, name).a.x, bounds(&d, email).a.x);
    }

    #[test]
    fn resizing_widens_the_last_stretched_field_of_a_line_and_moves_the_rest() {
        let mut form = Form::new("T");
        form.resizable(true);
        form.group("G");
        let mut line = form.line();
        let city = line.field("~C~ity", input());
        let zip = line.field("~Z~IP", InputLine::new(size(8, 1), 8));
        let mut d = form.build();

        let (c0, z0, g0) = (bounds(&d, city), bounds(&d, zip), group_boxes(&d)[0].1);
        let b = d.bounds();
        d.set_bounds(Rect::new(b.a.x, b.a.y, b.b.x + 6, b.b.y));
        assert_eq!(bounds(&d, city).width(), c0.width() + 6);
        assert_eq!(bounds(&d, zip).a.x, z0.a.x + 6, "moved, same size");
        assert_eq!(bounds(&d, zip).width(), 8);
        assert_eq!(
            group_boxes(&d)[0].1.width(),
            g0.width() + 6,
            "the box follows"
        );
    }

    #[test]
    fn alt_and_a_button_letter_presses_it_while_a_field_has_the_focus() {
        turbo_vision::views::button::set_press_animation(std::time::Duration::ZERO);
        let mut form = Form::new("T");
        let name = form.field("~N~ame", input());
        form.ok_cancel();
        let mut d = form.build();
        d.set_state(d.state() | turbo_vision::core::state::State::MODAL);

        // A plain letter is typed into the focused field...
        let mut o = Event::text('o');
        d.handle_event(&mut o);
        assert_eq!(d.get(name).map(InputLine::text), Some("o"));
        assert_eq!(d.end_state(), 0);

        // ...Alt and the letter presses OK.
        let mut alt_o = Event::keyboard(turbo_vision::core::event::KB_ALT_O);
        d.handle_event(&mut alt_o);
        assert_eq!(d.end_state(), CM_OK);
    }

    // ---- tabs ----

    fn panes(d: &Dialog) -> Vec<Rect> {
        (0..d.child_count())
            .filter(|&i| d.child_at(i).as_any().is::<TabbedPane>())
            .map(|i| d.child_at(i).bounds())
            .collect()
    }

    #[test]
    fn tabs_hold_their_pages_fields_and_the_pane_is_as_high_as_the_tallest_page() {
        let mut form = Form::new("T");
        form.tab("~O~ne");
        let a = form.field("~A~", input());
        form.field("~B~", input());
        form.tab("~T~wo");
        let c = form.field("~C~", input());
        form.end_tabs();
        form.ok_cancel();
        let mut dialog = form.build();

        // The fields are on pages, not in the dialog.
        assert!(dialog.get(a).is_none());
        let first = find_view_mut(&mut dialog, a.id())
            .expect("on page one")
            .bounds();
        let third = find_view_mut(&mut dialog, c.id())
            .expect("on page two")
            .bounds();
        assert_eq!(first.a.y, 1, "a blank row under the strip");
        assert_eq!(third.a.y, 1, "every page starts at the same row");

        let pane = panes(&dialog)[0];
        // Strip (3 rows), the taller page (3 rows: two fields and a gap),
        // a blank row and the border.
        assert_eq!(pane.height(), 3 + 1 + 3 + 1 + 1);
        assert_eq!(pane.width(), dialog.bounds().width() - 2 - 2 * MARGIN_X);
    }

    #[test]
    fn the_dialog_is_wide_enough_for_the_tab_titles_and_the_widest_page() {
        let mut form = Form::new("T");
        form.tab("A long first title");
        form.field("x", InputLine::new(size(8, 1), 8));
        form.tab("Second title");
        form.field("y", InputLine::new(size(40, 1), 40));
        form.end_tabs();
        let dialog = form.build();
        let pane = panes(&dialog)[0];
        // The 40-column field, its label and gap, and the pane's borders.
        assert!(pane.width() >= 40 + 2 + 2 * PAGE_PAD_X, "{pane:?}");
        let titles = display_width("A long first title") + display_width("Second title");
        assert!(pane.width() >= titles + 2 * TAB_TITLE_CHROME, "{pane:?}");
    }

    #[test]
    fn rows_after_the_tabs_go_below_the_pane() {
        let mut form = Form::new("T");
        let before = form.field("~F~irst", input());
        form.tab("One");
        form.field("a", input());
        form.end_tabs();
        let after = form.field("~L~ast", input());
        let dialog = form.build();
        let pane = panes(&dialog)[0];
        assert!(bounds(&dialog, before).b.y <= pane.a.y);
        assert!(bounds(&dialog, after).a.y > pane.b.y - 1);
    }

    #[test]
    fn tabs_end_themselves_at_build_and_groups_work_on_a_page() {
        let mut form = Form::new("T");
        form.tab("One");
        form.group("G");
        let a = form.field("a", input());
        let dialog = {
            let mut dialog_form = form;
            dialog_form.tab("Two");
            dialog_form.field("b", input());
            dialog_form.build() // closes the tabs
        };
        assert_eq!(panes(&dialog).len(), 1);
        let mut dialog = dialog;
        assert!(find_view_mut(&mut dialog, a.id()).is_some());
    }

    #[test]
    fn a_resizable_form_stretches_its_pane() {
        let mut form = Form::new("T");
        form.resizable(true);
        form.tab("One");
        form.field("a", input());
        form.end_tabs();
        let mut dialog = form.build();
        let before = panes(&dialog)[0].width();
        let bounds = dialog.bounds();
        dialog.set_bounds(Rect::new(
            bounds.a.x,
            bounds.a.y,
            bounds.b.x + 10,
            bounds.b.y,
        ));
        assert_eq!(panes(&dialog)[0].width(), before + 10);
    }
}
