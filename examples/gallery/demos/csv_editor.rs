//! A CSV table editor: a table in a window, with a menu bar, a status line
//! and dialogs to edit a cell, rename a column and confirm a discard. It is
//! host-driven, so the gallery runs it inside a view of its own, as the
//! Host-driven app page shows. Open it, then Enter or a double click edits
//! a cell, Ctrl+R inserts a row, F10 or a click opens the menus and Ctrl+Q
//! leaves.
//!
//! Parameters:
//! - `Session::open(w, h, name, disk)`: an editor of `w` by `h` cells over
//!   the file `name` on `disk` (an empty name starts a blank table);
//!   `open_bridged` lets a file whose first header is "#" lock its shape.
//! - `key(event)`: handle one event, a key or the mouse; `Some(line)` once
//!   the editor wants to close, with a line saying what happened.
//! - `step(w, h)`: draw a frame, laid out again after a resize;
//!   `buffer()` and `cursor()` read it back.
//! - `doc()`: the table being edited; `into_disk()` hands the disk back.
//! - `MemDisk`, `FsDisk::new(dir)`: where files live, in memory or in a
//!   directory it never leaves.
//!
//! See also: Host-driven app, Key names

use crate::demos::host_app::{Embedded, Guest};
use crate::panel::Panel;
use turbo_vision::app::Application;
use turbo_vision::core::command::{CM_USER, CommandId};
use turbo_vision::core::draw::Cell;
use turbo_vision::core::event::Event;
use turbo_vision::core::geometry::Rect;
use turbo_vision::core::state::Grow;
use turbo_vision::views::button::Button;
use turbo_vision::views::window::Window;
use turbo_vision::views::{GroupLike, View};
use tv_extensions::csv::{Disk, MemDisk, Session};

const OPEN: CommandId = CM_USER + 100;

const PLANETS: &str = "\
Planet,Moons,Day (hours),Rings
Mercury,0,4222.6,no
Venus,0,2802.0,no
Earth,1,24.0,no
Mars,2,24.7,no
Jupiter,95,9.9,yes
Saturn,146,10.7,yes
Uranus,28,17.2,yes
Neptune,16,16.1,yes
";

impl Guest for Session {
    fn event(&mut self, event: Event) -> bool {
        Session::key(self, event).is_none()
    }

    fn frame(&mut self, width: u16, height: u16) {
        self.step(width, height);
    }

    fn screen(&self) -> &[Vec<Cell>] {
        self.buffer()
    }

    fn cursor(&self) -> Option<(u16, u16)> {
        Session::cursor(self)
    }
}

pub fn build(panel: &mut Panel) {
    panel.add(Button::new(
        Rect::new(0, 0, 22, 2),
        "~O~pen planets.csv",
        OPEN,
        true,
    ));
}

pub fn handle(app: &mut Application, command: CommandId) -> bool {
    if command != OPEN {
        return false;
    }
    let mut disk = MemDisk::default();
    let _ = disk.write("planets.csv", PLANETS);

    // The editor needs 60 columns; it fills the window and follows it.
    let (w, h) = (70, 18);
    let mut window = Window::new(Rect::new(4, 1, 4 + w + 2, 1 + h + 2), "planets.csv");
    let session = Session::open(
        w.unsigned_abs(),
        h.unsigned_abs(),
        "planets.csv",
        Box::new(disk),
    );
    let mut editor = Embedded::new(Rect::new(0, 0, w, h), session);
    editor.set_grow_mode(Grow::ALL);
    window.add(editor);
    window.set_initial_focus();
    app.desktop.add(window);
    true
}
