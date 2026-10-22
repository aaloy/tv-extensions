// (C) 2026 - Antoni Aloy

//! A rule across a dialog: a row or column of line-drawing characters.

use turbo_vision::core::draw::DrawBuffer;
use turbo_vision::core::event::Event;
use turbo_vision::core::geometry::Rect;
use turbo_vision::core::palette::STATIC_TEXT_NORMAL;
use turbo_vision::terminal::Terminal;
use turbo_vision::views::view::{View, ViewCore, write_line_to_terminal};

/// A horizontal or vertical line, drawn across the whole of its bounds.
pub(crate) struct LineView {
    core: ViewCore,
    vertical: bool,
    double: bool,
}

impl LineView {
    pub(crate) fn new(bounds: Rect, vertical: bool, double: bool) -> Self {
        Self {
            core: ViewCore::new(bounds),
            vertical,
            double,
        }
    }

    fn glyph(&self) -> char {
        match (self.vertical, self.double) {
            (false, false) => '─',
            (false, true) => '═',
            (true, false) => '│',
            (true, true) => '║',
        }
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
        let glyph = self.glyph();
        let mut buf = DrawBuffer::new(width);
        buf.move_char(0, glyph, attr, width);
        for row in 0..height {
            let Ok(y) = i16::try_from(row) else { break };
            if self.vertical || row == 0 {
                write_line_to_terminal(terminal, 0, y, &buf);
            }
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
