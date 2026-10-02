//! The host-driven backend and pump, ported from turbo-vision core's
//! `tests/host_driven.rs` when they moved to this crate.

use turbo_vision::app::{AppHandler, Application};
use turbo_vision::core::command::CommandId;
use turbo_vision::core::event::Event;
use turbo_vision::core::geometry::Rect;
use turbo_vision::core::keys::{KeyCode, KeyEvent, KeyModifiers};
use turbo_vision::terminal::{Backend, Terminal};
use turbo_vision::views::input_line::InputLine;
use turbo_vision::views::window::WindowBuilder;
use turbo_vision::views::{GroupLike, View};
use tv_extensions::host::{self, HostBackend, HostInput, pump};

fn app(w: u16, h: u16) -> (Application, HostInput) {
    let (mut app, input) = host::app(w, h);
    // Without a menu bar and status line the desktop keeps a spare row at
    // the top and bottom (Borland: TProgram::initDeskTop's r.a.y++ and
    // r.b.y--); give it the whole screen so window rows are screen rows.
    let (sw, sh) = app.terminal.size();
    app.desktop.set_bounds(Rect::new(0, 0, sw, sh));
    (app, input)
}

fn key(c: char) -> Event {
    Event::from_crossterm_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()))
}

fn row_text(app: &Application, y: usize) -> String {
    app.terminal.buffer()[y].iter().map(|c| c.ch).collect()
}

#[test]
fn a_host_backend_says_it_is_host_driven() {
    let (backend, _input) = HostBackend::new(10, 5);
    assert!(backend.is_host_driven());
    let (app, _input) = host::app(10, 5);
    assert!(app.is_host_driven());
    assert_eq!(app.terminal.size(), (10, 5));
}

#[test]
fn host_input_counts_what_is_queued() {
    let (_backend, input) = HostBackend::new(10, 5);
    assert!(input.is_empty());
    input.push(key('a'));
    assert_eq!(input.len(), 1);
    assert!(!input.is_empty());
}

#[test]
fn pump_draws_without_blocking_and_reports_running() {
    let (mut app, _input) = app(40, 10);
    let mut window = WindowBuilder::new()
        .bounds(Rect::new(0, 0, 40, 10))
        .title("Hello")
        .build();
    window.add(InputLine::new(Rect::new(1, 1, 30, 2), 50));
    app.desktop.add(window);
    assert!(pump(&mut app, &mut ()));
    assert!(
        row_text(&app, 0).contains("Hello"),
        "{:?}",
        row_text(&app, 0)
    );
}

#[test]
fn pushed_keys_reach_the_focused_view() {
    let (mut app, input) = app(40, 10);
    let mut window = WindowBuilder::new()
        .bounds(Rect::new(0, 0, 40, 10))
        .title("T")
        .build();
    // Children sit in the window's interior, inside the frame: interior row 0
    // is screen row 1.
    window.add(InputLine::new(Rect::new(1, 0, 30, 1), 50));
    app.desktop.add(window);
    for c in "Hi".chars() {
        input.push(key(c));
    }
    assert_eq!(input.len(), 2);
    pump(&mut app, &mut ());
    assert_eq!(input.len(), 0, "pump drains the queue");
    assert!(row_text(&app, 1).contains("Hi"), "{:?}", row_text(&app, 1));
}

#[test]
fn a_handler_sees_commands_during_pump() {
    struct Seen(Vec<CommandId>);
    impl AppHandler for Seen {
        fn handle_command(&mut self, _: &mut Application, c: CommandId, _: &Event) -> bool {
            self.0.push(c);
            true
        }
    }
    let (mut app, input) = app(40, 10);
    input.push(Event::command(1234));
    let mut seen = Seen(Vec::new());
    pump(&mut app, &mut seen);
    assert_eq!(seen.0, vec![1234]);
}

#[test]
fn set_size_resizes_on_the_next_pump() {
    let (mut app, input) = app(40, 10);
    input.set_size(60, 20);
    pump(&mut app, &mut ());
    assert_eq!(app.terminal.size(), (60, 20));
}

#[test]
fn a_pumped_show_history_command_returns_promptly_without_popping_up() {
    use turbo_vision::core::command::CM_SHOW_HISTORY;

    let (mut app, input) = app(40, 10);
    input.push(Event::command(CM_SHOW_HISTORY));
    let started = std::time::Instant::now();
    pump(&mut app, &mut ());
    assert!(started.elapsed().as_millis() < 500);
}

#[test]
fn a_pumped_show_dropdown_command_returns_promptly_without_popping_up() {
    use turbo_vision::core::command::CM_SHOW_DROPDOWN;

    let (mut app, input) = app(40, 10);
    input.push(Event::command(CM_SHOW_DROPDOWN));
    let started = std::time::Instant::now();
    pump(&mut app, &mut ());
    assert!(started.elapsed().as_millis() < 500);
}

#[test]
fn pump_reports_false_once_the_application_quits() {
    use turbo_vision::core::command::CM_QUIT;

    let (mut app, input) = app(40, 10);
    input.push(Event::command(CM_QUIT));
    assert!(!pump(&mut app, &mut ()));
    assert!(!app.running);
}

#[test]
fn a_terminal_over_a_host_backend_builds() {
    let (backend, _input) = HostBackend::new(12, 3);
    let terminal = Terminal::with_backend(Box::new(backend)).expect("terminal");
    assert_eq!(terminal.size(), (12, 3));
}

#[test]
fn host_tracks_the_cursor() {
    let (mut app, input) = app(40, 10);
    pump(&mut app, &mut ());
    assert_eq!(input.cursor(), None, "nothing focused shows a cursor yet");

    let mut window = WindowBuilder::new()
        .bounds(Rect::new(0, 0, 40, 10))
        .title("T")
        .build();
    // Interior row/col 0, so the screen position is the window's origin
    // plus one cell for the frame border.
    window.add(InputLine::new(Rect::new(0, 0, 30, 1), 50));
    app.desktop.add(window);
    for c in "Hi".chars() {
        input.push(key(c));
    }
    pump(&mut app, &mut ());
    let (x, y) = input
        .cursor()
        .expect("a focused input line reports a cursor");
    assert_eq!(y, 1, "window(0) + border(1) + interior row(0)");
    assert_eq!(x, 1 + 2, "the input line's screen column plus the typed length");
}

