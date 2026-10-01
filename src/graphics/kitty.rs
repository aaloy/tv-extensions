// (C) 2026 - Enzo Lombardi

//! Kitty graphics protocol helpers that write straight to the terminal with
//! [`Terminal::write_raw`], outside the double-buffered cell grid.
//!
//! Ported from `Terminal::supports_kitty_graphics`, `Terminal::delete_kitty_image`
//! and `Terminal::clear_kitty_images` in turbo-vision core's `src/terminal/mod.rs`.

use std::io;
use turbo_vision::terminal::Terminal;

/// Check if the terminal supports Kitty graphics protocol.
///
/// This is a heuristic check based on the `TERM` environment variable
/// and known terminal capabilities. Returns `true` for terminals known
/// to support Kitty graphics (kitty, wezterm, ghostty, etc.).
pub fn supports_kitty_graphics() -> bool {
    // Check TERM environment variable for known Kitty-compatible terminals
    if let Ok(term) = std::env::var("TERM") {
        let term_lower = term.to_lowercase();
        if term_lower.contains("kitty")
            || term_lower.contains("wezterm")
            || term_lower.contains("ghostty")
        {
            return true;
        }
    }

    // Check TERM_PROGRAM for terminals that set it
    if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
        let prog_lower = term_program.to_lowercase();
        if prog_lower.contains("kitty")
            || prog_lower.contains("wezterm")
            || prog_lower.contains("ghostty")
        {
            return true;
        }
    }

    // Check for KITTY_WINDOW_ID (set by Kitty terminal)
    if std::env::var("KITTY_WINDOW_ID").is_ok() {
        return true;
    }

    false
}

/// Delete a Kitty graphics image by ID.
///
/// Sends a command to remove a previously transmitted image from the
/// terminal's memory.
///
/// # Arguments
///
/// * `terminal` - The terminal to send the command to.
/// * `image_id` - The ID of the image to delete.
///
/// # Errors
///
/// Returns an error if writing to the terminal fails.
pub fn delete_kitty_image(terminal: &mut Terminal, image_id: u32) -> io::Result<()> {
    let cmd = format!("\x1b_Ga=d,d=I,i={image_id},q=2;\x1b\\");
    terminal.write_raw(cmd.as_bytes())
}

/// Clear all Kitty graphics images from the terminal.
///
/// # Errors
///
/// Returns an error if writing to the terminal fails.
pub fn clear_kitty_images(terminal: &mut Terminal) -> io::Result<()> {
    // Delete all images: a=d,d=A (delete all)
    terminal.write_raw(b"\x1b_Ga=d,d=A,q=2;\x1b\\")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use turbo_vision::core::event::Event;
    use turbo_vision::terminal::Backend;

    /// A backend that keeps every byte the terminal writes, so a test can
    /// see what a kitty helper actually sent.
    struct RecordingBackend {
        written: Arc<Mutex<Vec<u8>>>,
    }

    impl Backend for RecordingBackend {
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
        fn init(&mut self) -> io::Result<()> {
            Ok(())
        }
        fn cleanup(&mut self) -> io::Result<()> {
            Ok(())
        }
        fn size(&self) -> io::Result<(u16, u16)> {
            Ok((4, 1))
        }
        fn poll_event(&mut self, _timeout: Duration) -> io::Result<Option<Event>> {
            Ok(None)
        }
        fn write_raw(&mut self, data: &[u8]) -> io::Result<()> {
            self.written
                .lock()
                .expect("no test holds this lock")
                .extend_from_slice(data);
            Ok(())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
        fn show_cursor(&mut self, _x: u16, _y: u16) -> io::Result<()> {
            Ok(())
        }
        fn hide_cursor(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// A terminal over a [`RecordingBackend`], plus a handle to read back
    /// whatever it was sent.
    fn recording_terminal() -> (Terminal, Arc<Mutex<Vec<u8>>>) {
        let written = Arc::new(Mutex::new(Vec::new()));
        let terminal = Terminal::with_backend(Box::new(RecordingBackend {
            written: Arc::clone(&written),
        }))
        .expect("the recording backend never fails to initialise");
        (terminal, written)
    }

    #[test]
    fn delete_kitty_image_sends_the_delete_command() {
        let (mut terminal, written) = recording_terminal();
        written.lock().unwrap().clear();
        delete_kitty_image(&mut terminal, 7).unwrap();
        let sent = String::from_utf8(written.lock().unwrap().clone()).unwrap();
        assert_eq!(sent, "\x1b_Ga=d,d=I,i=7,q=2;\x1b\\");
    }

    #[test]
    fn clear_kitty_images_sends_the_delete_all_command() {
        let (mut terminal, written) = recording_terminal();
        written.lock().unwrap().clear();
        clear_kitty_images(&mut terminal).unwrap();
        let sent = String::from_utf8(written.lock().unwrap().clone()).unwrap();
        assert_eq!(sent, "\x1b_Ga=d,d=A,q=2;\x1b\\");
    }
}
