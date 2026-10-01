// (C) 2026 - Enzo Lombardi

//! Screen captures: Ctrl+F12 saves a PNG of the screen and F12 an ANSI text
//! dump, once [`install`] puts the capture hook on a terminal. Files are named
//! `screenshot-YYYYMMDD-HHMMSS.png` and `screen-YYYYMMDD-HHMMSS.ans`.

pub mod ansi_dump;
pub mod png;

use std::io;
use std::path::{Path, PathBuf};
use turbo_vision::terminal::{CaptureKind, Terminal};

/// Save captures in the current working directory.
pub fn install(terminal: &mut Terminal) {
    install_in(terminal, ".");
}

/// Save captures in `dir`. Replaces any capture hook already installed. A
/// capture that cannot be written is logged and otherwise ignored.
pub fn install_in(terminal: &mut Terminal, dir: impl Into<PathBuf>) {
    let dir = dir.into();
    terminal.set_capture_hook(Box::new(move |kind, terminal| {
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let (path, result) = match kind {
            CaptureKind::Png => {
                let path = dir.join(format!("screenshot-{stamp}.png"));
                let result = save_png(terminal, &path);
                (path, result)
            }
            CaptureKind::Ansi => {
                let path = dir.join(format!("screen-{stamp}.ans"));
                let result = save_ansi(terminal, &path);
                (path, result)
            }
        };
        match result {
            Ok(()) => log::info!("capture saved to {}", path.display()),
            Err(e) => log::warn!("capture to {} failed: {e}", path.display()),
        }
    }));
}

/// Render the terminal's screen to a PNG at `path`, scaled to the terminal's
/// font cell height when the terminal reports one.
///
/// # Errors
///
/// Returns an error if the file cannot be created or written.
pub fn save_png(terminal: &Terminal, path: &Path) -> io::Result<()> {
    // Port of core Terminal::save_screenshot_png: pick the integer scale whose
    // glyph height is closest to the real cell height, clamped to 1..=8.
    let scale = match Terminal::query_font_pixel_size() {
        Some((_, ch)) if ch > 0 => {
            ((usize::from(ch) + png::GLYPH_HEIGHT / 2) / png::GLYPH_HEIGHT).clamp(1, 8)
        }
        _ => 1,
    };
    let (w, h) = terminal.size();
    let w = usize::try_from(w).unwrap_or(0);
    let h = usize::try_from(h).unwrap_or(0);
    png::render_to_png(terminal.buffer(), w, h, scale, path)
}

/// Write the terminal's screen as ANSI-coloured text at `path`.
///
/// # Errors
///
/// Returns an error if the file cannot be created or written.
pub fn save_ansi(terminal: &Terminal, path: &Path) -> io::Result<()> {
    // Port of core Terminal::dump_screen.
    let (w, h) = terminal.size();
    let w = usize::try_from(w).unwrap_or(0);
    let h = usize::try_from(h).unwrap_or(0);
    ansi_dump::dump_buffer_to_file(terminal.buffer(), w, h, &path.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use turbo_vision::terminal::CaptureKind;

    #[test]
    fn a_png_capture_writes_a_png_of_the_screen_size() {
        let dir = tempfile::tempdir().unwrap();
        let mut terminal = turbo_vision::test_util::test_terminal(20, 5);
        super::install_in(&mut terminal, dir.path());
        assert!(terminal.run_capture_hook(CaptureKind::Png));
        let pngs: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with(".png"))
            .collect();
        assert_eq!(pngs.len(), 1);
        let bytes = std::fs::read(pngs[0].path()).unwrap();
        assert_eq!(&bytes[1..4], b"PNG");
        // IHDR width and height, big-endian, at bytes 16..24.
        let w = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let h = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        let expected_w = u32::try_from(20 * super::png::GLYPH_WIDTH).unwrap();
        let expected_h = u32::try_from(5 * super::png::GLYPH_HEIGHT).unwrap();
        assert_eq!((w, h), (expected_w, expected_h));
    }

    #[test]
    fn an_ansi_capture_writes_an_ans_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut terminal = turbo_vision::test_util::test_terminal(20, 5);
        super::install_in(&mut terminal, dir.path());
        assert!(terminal.run_capture_hook(CaptureKind::Ansi));
        let names: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            names
                .iter()
                .any(|n| n.starts_with("screen-") && std::path::Path::new(n)
                    .extension()
                    .is_some_and(|ext| ext == "ans")),
            "{names:?}"
        );
    }

    #[test]
    fn a_capture_into_a_missing_directory_does_not_panic() {
        let mut terminal = turbo_vision::test_util::test_terminal(20, 5);
        super::install_in(&mut terminal, "/nonexistent/tv-extensions-capture-test");
        assert!(terminal.run_capture_hook(CaptureKind::Png));
    }

    #[test]
    fn installing_twice_keeps_one_hook() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let mut terminal = turbo_vision::test_util::test_terminal(20, 5);
        super::install_in(&mut terminal, first.path());
        super::install_in(&mut terminal, second.path());
        terminal.run_capture_hook(CaptureKind::Png);
        assert_eq!(std::fs::read_dir(first.path()).unwrap().count(), 0);
        assert_eq!(std::fs::read_dir(second.path()).unwrap().count(), 1);
    }
}
