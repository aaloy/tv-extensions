// (C) 2026 - Enzo Lombardi

//! [`Grid`]: a turbo-vision [`Table`] with a `│` in each gap between columns.
//!
//! A `Table` already leaves one blank cell after every column. A `Grid`
//! draws its table as usual and then writes [`SEPARATOR`] into those gaps,
//! so columns stay where they were, mouse hit-testing is untouched, and the
//! only thing that changes is how the grid reads.

use turbo_vision::core::command::CommandId;
use turbo_vision::core::draw::Cell;
use turbo_vision::core::event::Event;
use turbo_vision::core::geometry::Rect;
use turbo_vision::core::palette::Palette;
use turbo_vision::core::state::StateFlags;
use turbo_vision::terminal::Terminal;
use turbo_vision::views::View;
use turbo_vision::views::table::{Column, Table};
use turbo_vision::views::view::ViewCore;

/// The character drawn in each gap between two columns.
pub const SEPARATOR: char = '│';

/// A [`Table`] that draws a vertical line between its columns.
///
/// Everything but the separators is the table's: bounds, focus, the palette,
/// keyboard and mouse handling all go straight to the inner table, which
/// [`table`](Self::table) and [`table_mut`](Self::table_mut) expose. The
/// methods an editor usually calls on a table (columns, rows, selection) are
/// forwarded here too, so a `Handle<Grid>` works where a `Handle<Table>` did.
///
/// # Colours
///
/// Each separator takes the colour the table itself drew in that gap cell,
/// read back from the terminal buffer after the table's draw. The table
/// fills every line edge to edge before it writes its cells: the header in
/// the header colour, a body row in the normal colour, and the selected row
/// in the selected colour, so it reads as one bar. Reusing that colour puts
/// the header's separators in the header colour and makes the selected
/// row's separators part of its bar, whatever palette the grid is drawn
/// through (a window's, a dialog's, or a custom one), without the grid
/// knowing anything about palettes. It also means a gap the table did not
/// draw, because it is clipped away, is left alone.
///
/// Separators run the full height of the grid, rows below the last record
/// included, so columns read as continuous lines. They sit only between
/// columns that are on screen: nothing is drawn after the last visible one.
pub struct Grid {
    table: Table,
    separators: bool,
}

impl From<Table> for Grid {
    /// Wraps an existing table, separators on.
    fn from(table: Table) -> Self {
        Self {
            table,
            separators: true,
        }
    }
}

impl Grid {
    /// An empty grid that emits `on_select` when a cell is chosen (Enter or
    /// a double-click), as [`Table::new`] does. Separators are on.
    #[must_use]
    pub fn new(bounds: Rect, on_select: CommandId) -> Self {
        Table::new(bounds, on_select).into()
    }

    /// The same grid with its separators off: it then draws exactly as its
    /// table would.
    #[must_use]
    pub fn without_separators(mut self) -> Self {
        self.separators = false;
        self
    }

    /// Turns the separators on or off.
    pub fn set_separators(&mut self, on: bool) {
        self.separators = on;
    }

    /// Whether separators are drawn. On by default.
    #[must_use]
    pub fn separators(&self) -> bool {
        self.separators
    }

    /// The inner table.
    #[must_use]
    pub fn table(&self) -> &Table {
        &self.table
    }

    /// The inner table, for anything not forwarded here.
    pub fn table_mut(&mut self) -> &mut Table {
        &mut self.table
    }

    /// Unwraps the inner table.
    #[must_use]
    pub fn into_table(self) -> Table {
        self.table
    }

    /// Replaces the columns ([`Table::set_columns`]).
    pub fn set_columns(&mut self, columns: Vec<Column>) {
        self.table.set_columns(columns);
    }

    /// The columns ([`Table::columns`]).
    #[must_use]
    pub fn columns(&self) -> &[Column] {
        self.table.columns()
    }

    /// Replaces the rows ([`Table::set_rows`]).
    pub fn set_rows(&mut self, rows: Vec<Vec<String>>) {
        self.table.set_rows(rows);
    }

    /// Appends a row ([`Table::add_row`]).
    pub fn add_row(&mut self, row: Vec<String>) {
        self.table.add_row(row);
    }

    /// Removes every row ([`Table::clear_rows`]).
    pub fn clear_rows(&mut self) {
        self.table.clear_rows();
    }

    /// Number of rows ([`Table::row_count`]).
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.table.row_count()
    }

    /// The focused row, if any ([`Table::selected_row`]).
    #[must_use]
    pub fn selected_row(&self) -> Option<usize> {
        self.table.selected_row()
    }

    /// The focused column ([`Table::selected_col`]).
    #[must_use]
    pub fn selected_col(&self) -> usize {
        self.table.selected_col()
    }

    /// The text of the focused cell ([`Table::selected_cell`]).
    #[must_use]
    pub fn selected_cell(&self) -> Option<String> {
        self.table.selected_cell()
    }

    /// Focuses a row ([`Table::set_selected_row`]).
    pub fn set_selected_row(&mut self, row: usize) {
        self.table.set_selected_row(row);
    }

    /// Focuses a column ([`Table::set_selected_col`]).
    pub fn set_selected_col(&mut self, col: usize) {
        self.table.set_selected_col(col);
    }

    /// Whether the header row is drawn ([`Table::set_show_header`]).
    pub fn set_show_header(&mut self, show: bool) {
        self.table.set_show_header(show);
    }

    /// The command Enter or a double-click emits ([`Table::set_on_select`]).
    pub fn set_on_select(&mut self, command: CommandId) {
        self.table.set_on_select(command);
    }

    /// Where each visible column is drawn ([`Table::column_offsets`]): its x
    /// offset from the grid's left edge and its width. The separators do not
    /// change these.
    #[must_use]
    pub fn column_offsets(&self) -> Vec<(usize, u16)> {
        self.table.column_offsets()
    }

    /// Writes a separator into each gap between two visible columns, in the
    /// colour the table drew there. `terminal` is in the grid's own space,
    /// as it is during `draw`.
    fn draw_separators(&self, terminal: &mut Terminal) {
        let extent = self.extent();
        let width = usize::try_from(extent.b.x).unwrap_or(0);
        let offsets = self.table.column_offsets();
        // The gap after the last visible column is not between two columns.
        let gaps = offsets
            .iter()
            .take(offsets.len().saturating_sub(1))
            .map(|&(x, w)| x + usize::from(w))
            .filter(|&gap| gap < width)
            .filter_map(|gap| i16::try_from(gap).ok());
        for x in gaps {
            for y in 0..extent.b.y {
                if let Some(drawn) = terminal.read_cell(x, y) {
                    terminal.write_cell(x, y, Cell::new(SEPARATOR, drawn.attr));
                }
            }
        }
    }
}

impl View for Grid {
    fn core(&self) -> &ViewCore {
        self.table.core()
    }

    fn core_mut(&mut self) -> &mut ViewCore {
        self.table.core_mut()
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.table.set_bounds(bounds);
    }

    fn can_focus(&self) -> bool {
        self.table.can_focus()
    }

    fn state(&self) -> StateFlags {
        self.table.state()
    }

    fn set_state(&mut self, state: StateFlags) {
        self.table.set_state(state);
    }

    fn draw(&mut self, terminal: &mut Terminal) {
        self.table.draw(terminal);
        if self.separators {
            self.draw_separators(terminal);
        }
    }

    fn handle_event(&mut self, event: &mut Event) {
        self.table.handle_event(event);
    }

    fn update_cursor(&self, terminal: &mut Terminal) {
        self.table.update_cursor(terminal);
    }

    fn valid(&mut self, command: CommandId) -> bool {
        self.table.valid(command)
    }

    fn idle(&mut self) {
        self.table.idle();
    }

    fn get_palette(&self) -> Option<Palette> {
        self.table.get_palette()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
