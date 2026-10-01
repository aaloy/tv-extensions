//! CSV editor on the local terminal.
//!
//! Drives a host-driven csv::Session from a real terminal: each key goes to
//! the session, each frame is copied cell by cell to the screen.
//!
//! Run with:
//!   cargo run --example csv_edit --features csv,native -- [DIR] [FILE.csv]
//! DIR defaults to the current directory; with no FILE a new document opens.

// (C) 2026 - Enzo Lombardi

use std::io;
use std::time::Duration;
use turbo_vision::terminal::Terminal;
use tv_extensions::csv::{FsDisk, Session};

fn main() -> io::Result<()> {
    let mut args = std::env::args().skip(1);
    let dir = args.next().unwrap_or_else(|| ".".into());
    let file = args.next().unwrap_or_default();

    let mut terminal = Terminal::init().map_err(|e| io::Error::other(e))?;
    let (w, h) = terminal.size();
    let mut session = Session::open(w as u16, h as u16, &file, Box::new(FsDisk::new(dir)));

    let result = (|| -> io::Result<()> {
        loop {
            let (w, h) = terminal.size();
            session.step(w as u16, h as u16);
            for (y, row) in session.buffer().iter().enumerate() {
                for (x, cell) in row.iter().enumerate() {
                    let x_i16 = i16::try_from(x).unwrap_or(i16::MAX);
                    let y_i16 = i16::try_from(y).unwrap_or(i16::MAX);
                    terminal.write_cell(x_i16, y_i16, *cell);
                }
            }
            terminal.flush()?;
            if let Some(event) = terminal.poll_event(Duration::from_millis(50))? {
                if let Some(line) = session.key(event) {
                    terminal.shutdown().map_err(|e| io::Error::other(e))?;
                    println!("{line}");
                    return Ok(());
                }
            }
        }
    })();
    terminal.shutdown().map_err(|e| io::Error::other(e))?;
    result
}
