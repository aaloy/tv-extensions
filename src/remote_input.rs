// (C) 2026 - Enzo Lombardi

//! Remote keyboard input over TCP (disabled by default).
//!
//! This is a testing/automation aid: when enabled, this module listens on a
//! local TCP port and converts incoming text lines into keyboard events that
//! are injected into the application's event loop, exactly as if the keys had
//! been pressed.
//!
//! Each line may contain one or more whitespace-separated key chords; the
//! chords are parsed by [`turbo_vision::core::event::parse_key_chord`]
//! and queued in order. For example, sending the line:
//!
//! ```text
//! CTRL+F12 ALT+X
//! ```
//!
//! injects a `Ctrl+F12` press (e.g. take a screenshot) followed by `Alt+X`.
//!
//! The listener binds to `127.0.0.1` only, so it is reachable from the local
//! machine but not from the network. It is **off by default** and must be
//! enabled explicitly via [`enable`] or [`enable_from_env`].
//!
//! # Example
//!
//! ```bash
//! # With remote input enabled on port 8888:
//! printf 'CTRL+F12\n'   | nc 127.0.0.1 8888   # key chord(s)
//! printf 'CLICK 28 23\n' | nc 127.0.0.1 8888   # left-click at cell (28,23)
//! ```
//!
//! Mouse lines use `CLICK x y` (left button), `RCLICK x y`, or `MCLICK x y`
//! with 0-indexed cell coordinates; any other line is parsed as key chords.

use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::sync::mpsc::Sender;
use std::thread;

use turbo_vision::core::event::{
    Event, EventType, MB_LEFT_BUTTON, MB_MIDDLE_BUTTON, MB_RIGHT_BUTTON, parse_key_chord,
};
use turbo_vision::core::geometry::Point;
use turbo_vision::terminal::Terminal;

/// Parse one protocol line into the events it should inject.
///
/// Two line forms are recognized:
/// * Mouse: `CLICK x y` (left button), `RCLICK x y`, or `MCLICK x y` — emits a
///   button-down then button-up at the given 0-indexed cell coordinates.
/// * Keys: anything else is treated as whitespace-separated key chords (see
///   [`parse_key_chord`]), e.g. `CTRL+F12 ALT+X`.
///
/// Returns an empty vector for a blank or unparseable line.
pub fn parse_line(line: &str) -> Vec<Event> {
    let mut tokens = line.split_whitespace();
    let Some(first) = tokens.next() else {
        return Vec::new();
    };

    let button = match first.to_ascii_uppercase().as_str() {
        "CLICK" | "LCLICK" => Some(MB_LEFT_BUTTON),
        "RCLICK" => Some(MB_RIGHT_BUTTON),
        "MCLICK" => Some(MB_MIDDLE_BUTTON),
        _ => None,
    };

    if let Some(button) = button {
        let coords: Vec<&str> = tokens.collect();
        if let [xs, ys] = coords[..] {
            if let (Ok(x), Ok(y)) = (xs.parse::<i16>(), ys.parse::<i16>()) {
                let pos = Point::new(x, y);
                return vec![
                    Event::mouse(EventType::MouseDown, pos, button, false),
                    Event::mouse(EventType::MouseUp, pos, 0, false),
                ];
            }
        }
        return Vec::new(); // malformed mouse line
    }

    // Otherwise: one or more key chords.
    line.split_whitespace()
        .filter_map(parse_key_chord)
        .collect()
}

/// Bind a TCP listener on `127.0.0.1:port` and forward parsed key events to `tx`.
///
/// The listener and per-connection handlers run on detached background threads,
/// so this returns as soon as the socket is bound. Each accepted connection is
/// read line by line; every whitespace-separated chord on a line is parsed and,
/// if valid, sent through `tx`. Unparseable chords are logged and skipped.
///
/// # Errors
///
/// Returns an error if the port cannot be bound (e.g. already in use).
pub fn spawn(port: u16, tx: Sender<Event>) -> std::io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    log::info!("Remote key input listening on 127.0.0.1:{port}");

    thread::spawn(move || {
        for stream in listener.incoming() {
            let stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("Remote key input: accept failed: {e}");
                    continue;
                }
            };
            let tx = tx.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stream);
                for line in reader.lines() {
                    let Ok(line) = line else { break };
                    let events = parse_line(&line);
                    if events.is_empty() && !line.trim().is_empty() {
                        log::warn!("Remote input: unparseable line {line:?}");
                    }
                    for event in events {
                        // If the receiver is gone the app has exited; stop.
                        if tx.send(event).is_err() {
                            return;
                        }
                    }
                }
            });
        }
    });

    Ok(())
}

/// Listen on `127.0.0.1:port` and type every received chord into
/// `terminal`, with the chords typed through the terminal's
/// [`Terminal::event_injector`]. Injected Ctrl+F12
/// and F12 key chords are served by turbo-vision's built-in capture
/// (PNG with core's `screenshot` feature, ANSI dump always), or by a
/// capture hook if the application installed one ([`Terminal::set_capture_hook`]).
///
/// # Errors
///
/// Returns an error if the port cannot be bound.
pub fn enable(terminal: &mut Terminal, port: u16) -> std::io::Result<()> {
    spawn(port, terminal.event_injector())
}

/// Parse a `TV_REMOTE_KEYS` value into a port, or `None` if it is unset or
/// not a `u16`.
fn port_from(value: Option<String>) -> Option<u16> {
    value.and_then(|v| v.trim().parse::<u16>().ok())
}

/// [`enable`] on the port in the `TV_REMOTE_KEYS` environment variable.
/// Returns `Ok(false)`, doing nothing, when the variable is unset or not a
/// port number.
///
/// With a turbo-vision whose `Application::new` already reads
/// `TV_REMOTE_KEYS` itself and binds its own listener on that port (true of
/// the pinned core revision this crate depends on today), calling this
/// function afterward tries to bind the same port a second time and gets
/// `AddrInUse`. Either don't call `enable_from_env` with such a core, or
/// unset `TV_REMOTE_KEYS` before `Application::new` and set it again (or
/// call [`enable`] directly) afterward. A core revision that has had its
/// own remote input removed does not have this conflict.
///
/// # Errors
///
/// Returns an error if the port cannot be bound.
pub fn enable_from_env(terminal: &mut Terminal) -> std::io::Result<bool> {
    let Some(port) = port_from(std::env::var("TV_REMOTE_KEYS").ok()) else {
        return Ok(false);
    };
    enable(terminal, port)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use turbo_vision::core::event::{KB_ALT_X, KB_CTRL_F12};

    #[test]
    fn parses_click_into_down_up() {
        let evs = parse_line("CLICK 28 23");
        assert_eq!(evs.len(), 2);
        assert_eq!(evs[0].what, EventType::MouseDown);
        assert_eq!(evs[0].mouse.pos, Point::new(28, 23));
        assert_eq!(evs[0].mouse.buttons, MB_LEFT_BUTTON);
        assert_eq!(evs[1].what, EventType::MouseUp);
    }

    #[test]
    fn parses_key_chords_line() {
        let evs = parse_line("CTRL+F12 ALT+X");
        assert_eq!(evs.len(), 2);
        assert_eq!(evs[0].key_code, KB_CTRL_F12);
        assert_eq!(evs[1].key_code, KB_ALT_X);
    }

    #[test]
    fn blank_and_malformed_yield_nothing() {
        assert!(parse_line("").is_empty());
        assert!(parse_line("   ").is_empty());
        assert!(parse_line("CLICK 1").is_empty());
        assert!(parse_line("CLICK x y").is_empty());
    }

    #[test]
    fn enable_reports_a_port_in_use() {
        let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = taken.local_addr().unwrap().port();
        let mut terminal = turbo_vision::test_util::test_terminal(10, 5);
        assert!(enable(&mut terminal, port).is_err());
    }

    #[test]
    fn a_chord_sent_over_tcp_arrives_through_poll_event() {
        use std::io::Write;
        let probe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let mut terminal = turbo_vision::test_util::test_terminal(10, 5);
        enable(&mut terminal, port).unwrap();
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream.write_all(b"ENTER\n").unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let mut got = None;
        while got.is_none() && std::time::Instant::now() < deadline {
            got = terminal
                .poll_event(std::time::Duration::from_millis(20))
                .unwrap();
        }
        assert_eq!(
            got.map(|e| e.key_code),
            Some(turbo_vision::core::event::KB_ENTER)
        );
    }

    #[test]
    fn enable_from_env_ignores_unset_and_garbage() {
        assert_eq!(port_from(None), None);
        assert_eq!(port_from(Some("not a port".to_string())), None);
        assert_eq!(port_from(Some(" 8080 ".to_string())), Some(8080));
        assert_eq!(port_from(Some("70000".to_string())), None);
    }
}
