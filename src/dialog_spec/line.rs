// (C) 2026 - Antoni Aloy

//! A rule across a dialog: a row or column of line-drawing characters.

use turbo_vision::core::draw::DrawBuffer;
use turbo_vision::core::event::Event;
use turbo_vision::core::geometry::Rect;
use turbo_vision::core::palette::STATIC_TEXT_NORMAL;
use turbo_vision::terminal::Terminal;
use super::spec::LineStyle;
use turbo_vision::views::view::{View, ViewCore, write_line_to_terminal};

/// A row, a column, or a rectangle of line characters, drawn across the
/// whole of its bounds.
pub(crate) struct LineView {
    core: ViewCore,
    style: LineStyle,
}

/// The characters of a style: horizontal, vertical, and the four corners
/// (top left, top right, bottom left, bottom right).
fn glyphs(style: LineStyle) -> (char, char, [char; 4]) {
    match style {
        LineStyle::Single => ('─', '│', ['┌', '┐', '└', '┘']),
        LineStyle::Double => ('═', '║', ['╔', '╗', '╚', '╝']),
        LineStyle::Heavy => ('━', '┃', ['┏', '┓', '┗', '┛']),
        LineStyle::Round => ('─', '│', ['╭', '╮', '╰', '╯']),
        LineStyle::Block => ('█', '█', ['█'; 4]),
    }
}

impl LineView {
    pub(crate) fn new(bounds: Rect, style: LineStyle) -> Self {
        Self {
            core: ViewCore::new(bounds),
            style,
        }
    }

    /// The characters of row `y` of a `width` x `height` shape.
    fn row(&self, y: usize, width: usize, height: usize) -> Vec<char> {
        let (h, v, [tl, tr, bl, br]) = glyphs(self.style);
        if width == 1 && height > 1 {
            return vec![v];
        }
        if height == 1 || y == 0 || y + 1 == height {
            let mut row = vec![h; width];
            if height > 1 && width > 1 {
                let (left, right) = if y == 0 { (tl, tr) } else { (bl, br) };
                row[0] = left;
                row[width - 1] = right;
            }
            return row;
        }
        // The sides of a rectangle: the cells between them are left alone.
        let mut row = vec![v];
        row.resize(width, ' ');
        row[width - 1] = v;
        row
    }
}

impl View for LineView {
    fn core(&self) -> &ViewCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut ViewCore {
        &mut self.core
    }

    fn draw(&mut self, terminal: &mut Terminal) {
        let width = usize::try_from(self.core.bounds.width_clamped()).unwrap_or(0);
        let height = usize::try_from(self.core.bounds.height_clamped()).unwrap_or(0);
        let attr = self.map_color(STATIC_TEXT_NORMAL);
        for y in 0..height {
            let Ok(line) = i16::try_from(y) else { break };
            let row = self.row(y, width, height);
            let sides_only = width > 1 && height > 2 && y > 0 && y + 1 < height;
            if sides_only {
                for (x, ch) in [(0, row[0]), (width - 1, row[width - 1])] {
                    let mut buf = DrawBuffer::new(1);
                    buf.put_char(0, ch, attr);
                    let Ok(x) = i16::try_from(x) else { break };
                    write_line_to_terminal(terminal, x, line, &buf);
                }
                continue;
            }
            let mut buf = DrawBuffer::new(width);
            for (x, ch) in row.into_iter().enumerate() {
                buf.put_char(x, ch, attr);
            }
            write_line_to_terminal(terminal, 0, line, &buf);
        }
    }

    fn handle_event(&mut self, _event: &mut Event) {}

    fn get_palette(&self) -> Option<turbo_vision::core::palette::Palette> {
        use turbo_vision::core::palette::{Palette, palettes};
        Some(Palette::from_slice(palettes::CP_STATIC_TEXT))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
