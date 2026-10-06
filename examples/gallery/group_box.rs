// (C) 2026 - Antoni Aloy
//! A titled frame drawn around related controls.
//!
//! turbo-vision's `GroupBox` is not in a published release yet; this is the
//! same view, kept here until it is. It only draws: the controls it
//! surrounds are siblings added after it, so they draw on top of it and keep
//! their own focus order.

use turbo_vision::core::draw::DrawBuffer;
use turbo_vision::core::event::Event;
use turbo_vision::core::geometry::Rect;
use turbo_vision::core::palette::{Palette, STATIC_TEXT_NORMAL, palettes};
use turbo_vision::terminal::Terminal;
use turbo_vision::views::view::write_line_to_terminal;
use turbo_vision::views::{View, ViewCore};

pub struct GroupBox {
    core: ViewCore,
    title: String,
}

impl GroupBox {
    /// A box filling `bounds`, with `title` in its top edge. A `~` in the
    /// title is left out when drawn.
    pub fn new(bounds: Rect, title: &str) -> Self {
        Self {
            core: ViewCore {
                bounds,
                ..ViewCore::default()
            },
            title: title.chars().filter(|&c| c != '~').collect(),
        }
    }
}

impl View for GroupBox {
    fn core(&self) -> &ViewCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut ViewCore {
        &mut self.core
    }

    fn draw(&mut self, terminal: &mut Terminal) {
        let width = usize::try_from(self.core.bounds.width()).unwrap_or(0);
        let height = self.core.bounds.height();
        if width < 2 || height < 2 {
            return;
        }
        let attr = self.map_color(STATIC_TEXT_NORMAL);

        // Top edge: ┌─ Title ───┐, the title cut short if the box is narrow.
        let mut top = DrawBuffer::new(width);
        top.move_char(0, '─', attr, width);
        top.put_char(0, '┌', attr);
        top.put_char(width - 1, '┐', attr);
        if !self.title.is_empty() && width > 6 {
            let title: String = self.title.chars().take(width - 6).collect();
            top.move_str(2, &format!(" {title} "), attr);
        }
        write_line_to_terminal(terminal, 0, 0, &top);

        // Sides, with the inside cleared to the background.
        let mut side = DrawBuffer::new(width);
        side.move_char(0, ' ', attr, width);
        side.put_char(0, '│', attr);
        side.put_char(width - 1, '│', attr);
        for y in 1..height - 1 {
            write_line_to_terminal(terminal, 0, y, &side);
        }

        let mut bottom = DrawBuffer::new(width);
        bottom.move_char(0, '─', attr, width);
        bottom.put_char(0, '└', attr);
        bottom.put_char(width - 1, '┘', attr);
        write_line_to_terminal(terminal, 0, height - 1, &bottom);
    }

    fn handle_event(&mut self, _event: &mut Event) {
        // A box only draws: clicks on it go nowhere.
    }

    fn get_palette(&self) -> Option<Palette> {
        Some(Palette::from_slice(palettes::CP_STATIC_TEXT))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
