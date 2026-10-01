//! Lazy Data Example
//! Demonstrates core turbo-vision's lazy data views, which read only the
//! rows on screen instead of materializing the whole collection:
//! - `Table` browsing 100,000 generated rows, via `Table::set_provider`
//! - `ListBox` scrolling a one-million-item provider, via `ListBox::set_provider`
//!
//! Run with:
//!   `cargo run --example lazy_data --features native`
//!
//! Neither control materializes more than the visible viewport, so both
//! open instantly. Arrows/PgUp/PgDn/Home/End navigate; click the other
//! window to switch focus.

// (C) 2026 - Enzo Lombardi

use turbo_vision::app::Application;
use turbo_vision::core::command::CM_QUIT;
use turbo_vision::core::event::KB_ESC_ESC;
use turbo_vision::core::geometry::Rect;
use turbo_vision::core::status_data::StatusItemBuilder;
use turbo_vision::views::listbox::{ListBox, ListProvider};
use turbo_vision::views::status_line::StatusLine;
use turbo_vision::views::table::RowProvider;
use turbo_vision::views::window::WindowBuilder;
use turbo_vision::views::{Column, GroupLike, Table};

/// 100,000 synthetic inventory rows, computed on demand.
struct Inventory;

impl RowProvider for Inventory {
    fn rows(&self) -> usize {
        100_000
    }

    fn cell(&self, row: usize, col: usize) -> String {
        match col {
            0 => format!("{:06}", row + 1),
            1 => format!("Part {}", ["Alpha", "Bravo", "Charlie", "Delta"][row % 4]),
            2 => format!("{}", 3 + (row * 7) % 90),
            3 => format!("{}.{:02}", (row * 13) % 500, (row * 31) % 100),
            _ => String::new(),
        }
    }
}

/// One million lines, computed on demand.
struct Million;

impl ListProvider for Million {
    fn len(&self) -> usize {
        1_000_000
    }

    fn item(&self, index: usize) -> String {
        format!("Log line {:07}: everything is fine", index + 1)
    }
}

fn main() -> turbo_vision::core::error::Result<()> {
    let mut app = Application::new()?;

    let (width, height) = app.terminal.size();
    app.set_status_line(StatusLine::new(
        Rect::new(0, height - 1, width, height),
        vec![
            StatusItemBuilder::new()
                .text("~Alt-X~ Exit")
                .key("Alt+X")
                .command(CM_QUIT)
                .build(),
            StatusItemBuilder::new()
                .text("~Esc-Esc~ Exit")
                .key_code(KB_ESC_ESC)
                .command(CM_QUIT)
                .build(),
        ],
    ));

    // Table over 100k rows
    let mut grid_window = WindowBuilder::new()
        .bounds(Rect::new(2, 2, 46, 18))
        .title("Inventory (100,000 rows)")
        .build();
    let mut table = Table::new(Rect::new(1, 1, 41, 14), 1001);
    table.set_separators(true);
    table.set_columns(vec![
        Column::new("Id", 7),
        Column::new("Name", 14),
        Column::right("Qty", 5),
        Column::right("Price", 9),
    ]);
    table.set_provider(Box::new(Inventory));
    grid_window.add(table);
    grid_window.set_initial_focus();
    app.desktop.add(grid_window);

    // ListBox over a million items
    let mut list_window = WindowBuilder::new()
        .bounds(Rect::new(48, 4, 90, 20))
        .title("Log (1,000,000 lines)")
        .build();
    let mut list = ListBox::new(Rect::new(1, 1, 39, 14), 1002);
    list.set_provider(Box::new(Million));
    list_window.add(list);
    list_window.set_initial_focus();
    app.desktop.add(list_window);

    app.run();
    Ok(())
}
