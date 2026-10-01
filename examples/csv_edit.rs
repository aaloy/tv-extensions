//! CSV editor on the local terminal.
//!
//! Drives a host-driven [`csv::Session`] from a real terminal: each key goes to
//! the session, each frame is copied cell by cell to the screen.
//!
//! Run with:
//!   `cargo run --example csv_edit --features csv,native -- [DIR] [FILE.csv]`
//! DIR defaults to the current directory; with no FILE a new document opens.

// (C) 2026 - Enzo Lombardi

use std::io;
use std::time::Duration;
use turbo_vision::core::command::CM_REDRAW;
use turbo_vision::core::event::EventType;
use turbo_vision::terminal::Terminal;
use tv_extensions::csv::{FsDisk, Session};

fn main() -> io::Result<()> {
    let mut args = std::env::args().skip(1);
    let dir = args.next().unwrap_or_else(|| ".".into());
    let file = args.next().unwrap_or_default();

    let mut terminal = Terminal::init().map_err(io::Error::other)?;
    let (w, h) = terminal.size();
    let w = u16::try_from(w).unwrap_or(0);
    let h = u16::try_from(h).unwrap_or(0);
    let mut session = Session::open(w, h, &file, Box::new(FsDisk::new(dir)));

    let mut close_line = None;
    let result = (|| -> io::Result<()> {
        loop {
            let (w, h) = terminal.size();
            let w = u16::try_from(w).unwrap_or(0);
            let h = u16::try_from(h).unwrap_or(0);
            session.step(w, h);
            for (y, row) in session.buffer().iter().enumerate() {
                for (x, cell) in row.iter().enumerate() {
                    let x_i16 = i16::try_from(x).unwrap_or(i16::MAX);
                    let y_i16 = i16::try_from(y).unwrap_or(i16::MAX);
                    terminal.write_cell(x_i16, y_i16, *cell);
                }
            }
            terminal.flush()?;
            if let Some(event) = terminal.poll_event(Duration::from_millis(50))? {
                if event.what == EventType::Broadcast && event.command == CM_REDRAW {
                    if let Ok((w, h)) = terminal.backend_size() {
                        let (cur_w, cur_h) = terminal.size();
                        if w != cur_w || h != cur_h {
                            terminal.resize(u16::try_from(w).unwrap_or(0), u16::try_from(h).unwrap_or(0));
                        }
                    }
                } else if let Some(line) = session.key(event) {
                    close_line = Some(line);
                    break;
                }
            }
        }
        Ok(())
    })();
    terminal.shutdown().map_err(io::Error::other)?;
    result?;
    if let Some(line) = close_line {
        println!("{line}");
    }
    Ok(())
}
