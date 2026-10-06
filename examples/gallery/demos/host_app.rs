//! A whole Turbo Vision application stepped by its host, one frame at a
//! time: the Try it box holds a second Application, with its own desktop
//! and window, drawn by the gallery from its terminal buffer. Keys typed and
//! mouse clicks here are pushed into it and pumped. F6 is left to the
//! gallery.
//!
//! This is how a WASM guest runs, or anything that owns the screen and the
//! event loop: nothing waits, and nothing is written to a terminal.
//!
//! Parameters:
//! - `host::app(w, h)`: an application on a `HostBackend` of `w` by `h`
//!   cells, and the `HostInput` that feeds it.
//! - `input.push(event)`: queue an event; `input.set_size(w, h)`: the size
//!   the next frame lays out for.
//! - `pump(&mut app, &mut handler)`: handle what is queued, idle once and
//!   draw; `false` once the application has quit.
//! - `app.terminal.buffer()`: the finished cells, row by row.
//! - `input.cursor()`: where the application shows its text cursor, for a
//!   host that draws its own.
//!
//! See also: CSV editor, Key names

use crate::panel::Panel;
use turbo_vision::app::Application;
use turbo_vision::core::draw::Cell;
use turbo_vision::core::event::{Event, EventType, KB_F6, KB_SHIFT_F6};
use turbo_vision::core::geometry::Rect;
use turbo_vision::core::palette::Palette;
use turbo_vision::terminal::Terminal;
use turbo_vision::views::input_line::InputLine;
use turbo_vision::views::label::Label;
use turbo_vision::views::static_text::StaticText;
use turbo_vision::views::window::Window;
use turbo_vision::views::{GroupLike, View, ViewCore};
use tv_extensions::host::{self, HostInput, pump};

/// An application a view can show: it takes keys and the mouse, draws
/// frames and hands back its cells.
pub trait Guest {
    /// Handle one event, a key or the mouse (its position in the guest's
    /// cells); `false` once the guest has quit.
    fn event(&mut self, event: Event) -> bool;
    /// Draw a frame at `width` by `height`.
    fn frame(&mut self, width: u16, height: u16);
    /// The last frame's cells.
    fn screen(&self) -> &[Vec<Cell>];
    /// Where the guest shows its text cursor.
    fn cursor(&self) -> Option<(u16, u16)>;
}

/// A view that shows a [`Guest`] and passes it the keys and the mouse
/// events it gets.
pub struct Embedded<G> {
    core: ViewCore,
    guest: G,
    size: (u16, u16),
}

impl<G: Guest> Embedded<G> {
    pub fn new(bounds: Rect, guest: G) -> Self {
        Self {
            core: ViewCore {
                bounds,
                ..ViewCore::default()
            },
            guest,
            size: (0, 0),
        }
    }
}

impl<G: Guest + 'static> View for Embedded<G> {
    fn core(&self) -> &ViewCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut ViewCore {
        &mut self.core
    }

    fn draw(&mut self, terminal: &mut Terminal) {
        // The guest is as big as the view; a resized view resizes it.
        let e = self.extent();
        let size = (
            u16::try_from(e.width()).unwrap_or(0),
            u16::try_from(e.height()).unwrap_or(0),
        );
        if size != self.size {
            self.guest.frame(size.0, size.1);
            self.size = size;
        }
        for (y, row) in (0..e.height()).zip(self.guest.screen()) {
            let width = usize::from(size.0).min(row.len());
            terminal.write_line(0, y, &row[..width]);
        }
    }

    fn handle_event(&mut self, event: &mut Event) {
        match event.what {
            // The keys the focus brings; F6 is the gallery's.
            EventType::Keyboard
                if self.is_focused() && !matches!(event.key_code, KB_F6 | KB_SHIFT_F6) => {}
            // The owner hands the view the mouse events over it, in its own
            // space: that is the guest's screen, cell for cell. A press
            // also focuses the view first, as for any control.
            EventType::MouseDown
            | EventType::MouseUp
            | EventType::MouseMove
            | EventType::MouseWheelUp
            | EventType::MouseWheelDown => {}
            _ => return,
        }
        self.guest.event(*event);
        event.clear();
    }

    fn can_focus(&self) -> bool {
        true
    }

    fn update_cursor(&self, terminal: &mut Terminal) {
        match self.guest.cursor().filter(|_| self.is_focused()) {
            Some((x, y)) => {
                let (x, y) = (i16::try_from(x), i16::try_from(y));
                if let (Ok(x), Ok(y)) = (x, y) {
                    let _ = terminal.show_cursor(x, y);
                }
            }
            None => {
                let _ = terminal.hide_cursor();
            }
        }
    }

    fn get_palette(&self) -> Option<Palette> {
        None
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// The guest: an application with one window, pumped by the gallery.
pub struct Notes {
    app: Application,
    input: HostInput,
}

impl Notes {
    pub fn new(width: u16, height: u16) -> Self {
        let (mut app, input) = host::app(width, height);
        // Without a menu bar and status line the desktop leaves a row free
        // at the top and bottom: give it the whole screen.
        let (w, h) = app.terminal.size();
        app.desktop.set_bounds(Rect::new(0, 0, w, h));
        let mut window = Window::new(Rect::new(0, 0, w, h), "Guest application");
        window.add(StaticText::new(
            Rect::new(1, 0, w - 3, 2),
            "A second Application, run by the gallery.",
        ));
        let name = window.add(InputLine::new(Rect::new(9, 3, w - 4, 4), 40));
        let mut label = Label::new(Rect::new(1, 3, 9, 4), "~N~ame");
        label.set_link(name);
        window.add(label);
        window.set_initial_focus();
        app.desktop.add(window);
        Self { app, input }
    }
}

impl Guest for Notes {
    fn event(&mut self, event: Event) -> bool {
        self.input.push(event);
        pump(&mut self.app, &mut ())
    }

    fn frame(&mut self, width: u16, height: u16) {
        self.input.set_size(width, height);
        pump(&mut self.app, &mut ());
    }

    fn screen(&self) -> &[Vec<Cell>] {
        self.app.terminal.buffer()
    }

    fn cursor(&self) -> Option<(u16, u16)> {
        self.input.cursor()
    }
}

pub fn build(panel: &mut Panel) {
    panel.add(Embedded::new(Rect::new(0, 0, 46, 7), Notes::new(46, 7)));
}
