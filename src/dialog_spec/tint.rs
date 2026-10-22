// (C) 2026 - Antoni Aloy

//! Gives the control before it a text colour.

use turbo_vision::core::draw::DrawBuffer;
use turbo_vision::core::event::Event;
use turbo_vision::core::geometry::Rect;
use turbo_vision::core::palette::{Attr, LABEL_NORMAL, TvColor};
use turbo_vision::terminal::Terminal;
use turbo_vision::views::view::{View, ViewCore};

/// A view with no look of its own, placed just after a label: it recolours
/// the label's normal text and leaves the rest as the label drew it (the
/// hot key, and the red of a label whose field has an error).
pub(crate) struct Tint {
    core: ViewCore,
    color: TvColor,
}

impl Tint {
    pub(crate) fn new(bounds: Rect, color: TvColor) -> Self {
        Self {
            core: ViewCore::new(bounds),
            color,
        }
    }
}

impl View for Tint {
    fn core(&self) -> &ViewCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut ViewCore {
        &mut self.core
    }

    fn draw(&mut self, terminal: &mut Terminal) {
        let normal: Attr = self.map_color(LABEL_NORMAL);
        let width = self.core.bounds.width_clamped();
        let height = self.core.bounds.height_clamped();
        for y in 0..height {
            for x in 0..width {
                if let Some(cell) = terminal.read_cell(x, y)
                    && cell.attr == normal
                {
                    let attr = Attr {
                        fg: self.color,
                        ..cell.attr
                    };
                    terminal.write_cell(x, y, turbo_vision::core::draw::Cell::new(cell.ch, attr));
                }
            }
        }
        let _ = DrawBuffer::new(0);
    }

    fn handle_event(&mut self, _event: &mut Event) {}

    fn get_palette(&self) -> Option<turbo_vision::core::palette::Palette> {
        use turbo_vision::core::palette::{Palette, palettes};
        Some(Palette::from_slice(palettes::CP_LABEL))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
