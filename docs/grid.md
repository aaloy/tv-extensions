# Grid

`Grid` is turbo-vision's `Table` with a vertical line between its columns. A table already leaves a one-cell gap after every column. A grid draws its table exactly as the table would and then writes a `│` into each of those gaps, so columns stay where they were, the widths you set are the widths you get, and mouse clicks pick the same cells. The gap itself still selects nothing.

## Using it

A grid is made the way a table is, with its bounds and the command that Enter or a double-click should emit, and separators are on from the start.

```rust
use turbo_vision::core::geometry::Rect;
use turbo_vision::views::table::Column;
use tv_extensions::Grid;

let mut grid = Grid::new(Rect::new(0, 0, 60, 20), CMD_EDIT_CELL);
grid.set_columns(vec![Column::new("A", 10), Column::new("B", 10)]);
grid.set_rows(rows);
```

`Grid::from(table)` wraps a table you already have, `without_separators()` returns the same grid drawing as a plain table, and `set_separators` switches them at run time. The table methods an editor reaches for are forwarded: `set_columns`, `columns`, `set_rows`, `add_row`, `clear_rows`, `row_count`, `selected_row`, `selected_col`, `selected_cell`, `set_selected_row`, `set_selected_col`, `set_show_header`, `set_on_select` and `column_offsets`. Anything else is on the inner table through `table()` and `table_mut()`, and `into_table()` gives it back.

Because `Grid` is a `View` in its own right, it is added and found again through a typed handle like any other child. An application that kept a `Handle<Table>` changes the type to `Handle<Grid>` and keeps its calls:

```rust
let handle = window.add_typed(grid);
if let Some(g) = window.get_mut(handle) {
    g.set_selected_row(0);
    g.set_bounds(new_bounds);
}
```

## Colours

The grid does not choose a colour for its separators. After the table has drawn, it reads the colour of each gap cell back from the terminal buffer and writes the `│` in that colour. The table fills each line edge to edge before it writes its cells: the header in the header colour, a body row in the list colour, and the selected row in the selected colour so it reads as one bar. So the header's separators come out in the header colour, the selected row's are part of its bar, and every other row's match the row, whether the grid sits in a window, a dialog or anything with its own palette. The focused cell's highlight never covers a gap, so it never leaks into a separator. A gap the table could not draw, because a parent clipped it, is left alone too.

## Where separators go

Separators run the whole height of the grid, header included and rows below the last record too, so the columns read as continuous lines down to the bottom edge. They sit only between two columns that are on screen, as reported by `Table::column_offsets`. There is none after the last visible column. When a wide grid is scrolled sideways, that includes the case where the next column begins exactly at the right edge and so is entirely off screen: the grid's right edge then ends the line instead of a separator.
