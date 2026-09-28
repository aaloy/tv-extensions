//! `Grid`: a `Table` with a `│` in each gap between columns.

use turbo_vision::app::Application;
use turbo_vision::core::event::{Event, EventType, MB_LEFT_BUTTON};
use turbo_vision::core::geometry::{Point, Rect};
use turbo_vision::core::palette::Attr;
use turbo_vision::core::palette::palettes::{
    CP_APP_COLOR, CP_BLUE_WINDOW, CP_GRAY_DIALOG, CP_LISTBOX, CP_TABLE_WINDOW,
};
use turbo_vision::views::dialog::Dialog;
use turbo_vision::views::table::{Column, Table};
use turbo_vision::views::window::WindowBuilder;
use turbo_vision::views::{GroupLike, View};
use tv_extensions::grid::{Grid, SEPARATOR};
use tv_extensions::host;

fn app(w: u16, h: u16) -> Application {
    let (mut app, _input) = host::app(w, h);
    let (sw, sh) = app.terminal.size();
    app.desktop.set_bounds(Rect::new(0, 0, sw, sh));
    app
}

/// Resolve a table palette slot through an owner palette to the app.
fn resolve(table: &[u8], owner: &[u8], slot: usize) -> Attr {
    let owner_index = table[slot - 1] as usize;
    let app_index = owner[owner_index - 1] as usize;
    Attr::from_u8(CP_APP_COLOR[app_index - 1])
}

fn columns() -> Vec<Column> {
    vec![
        Column::new("A", 4),
        Column::new("B", 3),
        Column::new("C", 5),
    ]
}

fn rows() -> Vec<Vec<String>> {
    vec![
        vec!["a0".into(), "b0".into(), "c0".into()],
        vec!["a1".into(), "b1".into(), "c1".into()],
        vec!["a2".into(), "b2".into(), "c2".into()],
    ]
}

/// Columns 4, 3 and 5 wide at x 0, 5 and 9; gaps at x 4 and 8. Row 1 and
/// column 1 are selected.
fn grid() -> Grid {
    let mut g = Grid::new(Rect::new(0, 0, 20, 4), 0);
    g.set_columns(columns());
    g.set_rows(rows());
    g.set_selected_row(1);
    g.set_selected_col(1);
    g
}

const GAPS: [usize; 2] = [4, 8];

/// Screen cell of table line `y`: the window interior starts at (1, 1).
fn cell(app: &Application, x: usize, y: usize) -> (char, Attr) {
    let c = app.terminal.buffer()[1 + y][1 + x];
    (c.ch, c.attr)
}

fn in_blue_window(grid: Grid) -> Application {
    let mut app = app(40, 10);
    let mut window = WindowBuilder::new()
        .bounds(Rect::new(0, 0, 30, 8))
        .title("T")
        .build();
    window.add(grid);
    app.desktop.add(window);
    host::pump(&mut app, &mut ());
    app
}

#[test]
fn the_separator_sits_in_every_gap() {
    let app = in_blue_window(grid());
    let header = resolve(CP_TABLE_WINDOW, CP_BLUE_WINDOW, 4);
    let normal = resolve(CP_TABLE_WINDOW, CP_BLUE_WINDOW, 2);
    let selected = resolve(CP_TABLE_WINDOW, CP_BLUE_WINDOW, 3);
    for x in GAPS {
        assert_eq!(cell(&app, x, 0), (SEPARATOR, header), "header gap at x {x}");
        assert_eq!(cell(&app, x, 1), (SEPARATOR, normal), "row gap at x {x}");
        assert_eq!(
            cell(&app, x, 2),
            (SEPARATOR, selected),
            "selected gap at x {x}"
        );
        assert_eq!(cell(&app, x, 3), (SEPARATOR, normal), "last row at x {x}");
    }
    // Columns keep their places: text still starts where it did.
    assert_eq!(cell(&app, 0, 1).0, 'a');
    assert_eq!(cell(&app, 5, 1).0, 'b');
    assert_eq!(cell(&app, 9, 1).0, 'c');
}

#[test]
fn no_separator_after_the_last_column() {
    let app = in_blue_window(grid());
    for y in 0..4 {
        for x in 14..20 {
            assert_ne!(
                cell(&app, x, y).0,
                SEPARATOR,
                "past the last column at ({x}, {y})"
            );
        }
    }
}

#[test]
fn the_selected_row_gap_takes_the_bar_colour() {
    let app = in_blue_window(grid());
    let bar = cell(&app, 0, 2).1;
    for x in GAPS {
        assert_eq!(cell(&app, x, 2).1, bar, "gap at x {x} is part of the bar");
    }
    // The focused cell stands out, and its neighbouring gaps stay in the bar.
    assert_ne!(cell(&app, 5, 2).1, bar);
}

#[test]
fn the_separator_follows_a_dialog_palette() {
    let mut app = app(40, 10);
    let mut dialog = Dialog::new(Rect::new(0, 0, 30, 8), "D");
    dialog.add(grid());
    app.desktop.add(dialog);
    host::pump(&mut app, &mut ());
    let normal = resolve(CP_LISTBOX, CP_GRAY_DIALOG, 2);
    let selected = resolve(CP_LISTBOX, CP_GRAY_DIALOG, 3);
    let header = resolve(CP_LISTBOX, CP_GRAY_DIALOG, 4);
    for x in GAPS {
        assert_eq!(cell(&app, x, 0), (SEPARATOR, header));
        assert_eq!(cell(&app, x, 1), (SEPARATOR, normal));
        assert_eq!(cell(&app, x, 2), (SEPARATOR, selected));
    }
}

#[test]
fn without_separators_the_grid_draws_as_its_table() {
    let plain = in_blue_window(grid().without_separators());
    let mut table = Table::new(Rect::new(0, 0, 20, 4), 0);
    table.set_columns(columns());
    table.set_rows(rows());
    table.set_selected_row(1);
    table.set_selected_col(1);
    let mut reference = app(40, 10);
    let mut window = WindowBuilder::new()
        .bounds(Rect::new(0, 0, 30, 8))
        .title("T")
        .build();
    window.add(table);
    reference.desktop.add(window);
    host::pump(&mut reference, &mut ());
    assert_eq!(plain.terminal.buffer(), reference.terminal.buffer());
    for x in GAPS {
        assert_eq!(cell(&plain, x, 1).0, ' ');
    }
}

#[test]
fn separators_are_on_by_default_and_can_be_turned_off() {
    assert!(grid().separators());
    assert!(!grid().without_separators().separators());
    let mut g = grid();
    g.set_separators(false);
    assert!(!g.separators());
    assert!(
        !Grid::from(Table::new(Rect::new(0, 0, 5, 5), 0))
            .without_separators()
            .separators()
    );
    assert!(Grid::from(Table::new(Rect::new(0, 0, 5, 5), 0)).separators());
}

#[test]
fn separators_do_not_move_the_columns() {
    let with = grid();
    let without = grid().without_separators();
    assert_eq!(with.column_offsets(), vec![(0, 4), (5, 3), (9, 5)]);
    assert_eq!(with.column_offsets(), without.column_offsets());
    assert_eq!(with.column_offsets(), with.table().column_offsets());
}

#[test]
fn a_click_left_and_right_of_a_gap_picks_the_same_cells_as_the_table() {
    // Screen line 1 is body row 0. x 3 is the last cell of A, x 5 the first
    // of B; the gap at 4 selects nothing.
    for (x, col) in [(3, 0), (5, 1), (9, 2)] {
        let mut g = grid();
        let mut click = Event::mouse(
            EventType::MouseDown,
            Point::new(x, 1),
            MB_LEFT_BUTTON,
            false,
        );
        g.handle_event(&mut click);
        assert_eq!(
            (g.selected_row(), g.selected_col()),
            (Some(0), col),
            "x {x}"
        );
    }
}

#[test]
fn the_forwarded_table_api_reaches_the_inner_table() {
    let mut g = grid();
    assert_eq!(g.row_count(), 3);
    assert_eq!(g.columns().len(), 3);
    assert_eq!(g.selected_cell(), Some("b1"));
    g.set_selected_row(2);
    g.set_selected_col(0);
    assert_eq!(g.table().selected_row(), Some(2));
    assert_eq!(g.table().selected_col(), 0);
    g.table_mut().set_selected_col(2);
    assert_eq!(g.selected_col(), 2);
    g.add_row(vec!["x".into()]);
    assert_eq!(g.row_count(), 4);
    g.clear_rows();
    assert_eq!(g.row_count(), 0);
    let table: Table = g.into_table();
    assert_eq!(table.columns().len(), 3);
}

#[test]
fn a_grid_is_reached_through_a_typed_handle() {
    let mut window = WindowBuilder::new()
        .bounds(Rect::new(0, 0, 30, 8))
        .title("T")
        .build();
    let handle = window.add_typed(grid());
    let g = window.get_mut(handle).expect("the grid is a child");
    g.set_selected_row(0);
    g.set_bounds(Rect::new(0, 0, 25, 5));
    assert_eq!(window.get(handle).unwrap().bounds(), Rect::new(0, 0, 25, 5));
    assert_eq!(window.get(handle).unwrap().selected_row(), Some(0));
}

#[test]
fn a_scrolled_grid_draws_separators_only_between_visible_columns() {
    // Five 4-wide columns in a 9-wide grid: focusing the last scrolls it in.
    let mut g = Grid::new(Rect::new(0, 0, 9, 3), 0);
    g.set_columns((0..5).map(|i| Column::new(format!("{i}"), 4)).collect());
    g.set_rows(vec![vec!["r".into(); 5]]);
    g.set_selected_row(0);
    g.set_selected_col(4);
    assert_eq!(g.column_offsets(), vec![(0, 4), (5, 4)]);
    let app = in_blue_window(g);
    assert_eq!(cell(&app, 4, 0).0, SEPARATOR);
    assert_eq!(cell(&app, 4, 1).0, SEPARATOR);
    assert_ne!(cell(&app, 9, 1).0, SEPARATOR, "outside the grid");
}
