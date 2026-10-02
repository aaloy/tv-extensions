//! CSV editor on the local terminal.
//!
//! Drives a host-driven [`csv::Session`] from a real terminal: each key goes to
//! the session, each frame is copied cell by cell to the screen.
//!
//! Run with:
//!   `cargo run --example csv_edit --features csv,native -- path/to/file.csv`
//!   `cargo run --example csv_edit --features csv,native -- [DIR] [FILE.csv]`
//! A single argument is split into a directory and a file name at its last
//! path separator (so `a/b.csv` opens `b.csv` in `a`, and a bare `b.csv`
//! opens it in the current directory); `DIR FILE` still works as two
//! arguments. With no file name, a new document opens.

// (C) 2026 - Enzo Lombardi

use std::io;
use std::time::Duration;
use turbo_vision::core::command::CM_REDRAW;
use turbo_vision::core::event::EventType;
use turbo_vision::terminal::Terminal;
use tv_extensions::csv::{FsDisk, Session};

/// Splits the command-line arguments into `(dir, file)`: two arguments are
/// `DIR FILE` as-is; a single argument is a path, split at its last path
/// separator (`.../a/b.csv` -> `(".../a", "b.csv")`, `b.csv` -> `(".", "b.csv")`);
/// no arguments means `(".", "")`, a new document in the current directory.
fn dir_and_file(mut args: impl Iterator<Item = String>) -> (String, String) {
    let first = args.next();
    let second = args.next();
    match (first, second) {
        (Some(dir), Some(file)) => (dir, file),
        (Some(path), None) => {
            let p = std::path::Path::new(&path);
            match p.parent().filter(|parent| !parent.as_os_str().is_empty()) {
                Some(parent) => (
                    parent.to_string_lossy().into_owned(),
                    p.file_name()
                        .map_or_else(String::new, |f| f.to_string_lossy().into_owned()),
                ),
                None => (".".to_string(), path),
            }
        }
        (None, _) => (".".to_string(), String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::dir_and_file;

    fn split(args: &[&str]) -> (String, String) {
        dir_and_file(args.iter().map(ToString::to_string))
    }

    #[test]
    fn no_arguments_is_a_new_document_here() {
        assert_eq!(split(&[]), (".".to_string(), String::new()));
    }

    #[test]
    fn a_single_bare_name_opens_it_here() {
        assert_eq!(split(&["file.csv"]), (".".to_string(), "file.csv".to_string()));
    }

    #[test]
    fn a_single_path_is_split_at_its_last_separator() {
        assert_eq!(
            split(&["a/b.csv"]),
            ("a".to_string(), "b.csv".to_string())
        );
        assert_eq!(
            split(&["dir/sub/file.csv"]),
            ("dir/sub".to_string(), "file.csv".to_string())
        );
    }

    #[test]
    fn dir_and_file_arguments_still_work() {
        assert_eq!(
            split(&["dir", "file.csv"]),
            ("dir".to_string(), "file.csv".to_string())
        );
    }
}

fn main() -> io::Result<()> {
    let (dir, file) = dir_and_file(std::env::args().skip(1));

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
            match session.cursor() {
                Some((x, y)) => {
                    let x = i16::try_from(x).unwrap_or(i16::MAX);
                    let y = i16::try_from(y).unwrap_or(i16::MAX);
                    terminal.show_cursor(x, y)?;
                }
                None => terminal.hide_cursor()?,
            }
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
