// (C) 2026 - Enzo Lombardi

//! Host-driven applications: turbo-vision stepped by an embedder that owns
//! the screen and the event loop, such as a plank WASM frame.
//!
//! Events are pushed in through [`HostInput`], and output goes nowhere: the
//! embedder reads the finished cells from
//! [`Terminal::buffer`](turbo_vision::terminal::Terminal::buffer) after each
//! [`pump`].
//!
//! [`HostBackend::poll_event`](Backend::poll_event) never waits. A
//! host-driven application is stepped by its embedder one [`pump`] at a
//! time, so blocking for input would stall the host instead. Because the
//! backend reports [`Backend::is_host_driven`], turbo-vision refuses the
//! calls that would need a nested event loop (`exec_view`, `execute_modal`
//! and the history and dropdown popups) with `CM_CANCEL`, rather than
//! spinning forever inside one pump.

use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use turbo_vision::app::{AppHandler, Application};
use turbo_vision::core::event::Event;
use turbo_vision::terminal::{Backend, Capabilities, Terminal};

#[derive(Debug, Default)]
struct Shared {
    queue: VecDeque<Event>,
    size: (u16, u16),
    cursor: Option<(u16, u16)>,
}

/// Locks the shared state. The critical sections only push, pop or assign,
/// so a panic elsewhere can't leave it half-updated: poisoning is ignored.
fn lock(shared: &Mutex<Shared>) -> MutexGuard<'_, Shared> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The embedder's end: push events, change the size. Clones share one
/// queue, so the handle can live wherever the host receives its input.
#[derive(Debug, Clone)]
pub struct HostInput(Arc<Mutex<Shared>>);

impl HostInput {
    /// Queues one event for the next [`pump`].
    pub fn push(&self, event: Event) {
        lock(&self.0).queue.push_back(event);
    }

    /// Sets the screen size the next [`pump`] lays out for.
    pub fn set_size(&self, w: u16, h: u16) {
        lock(&self.0).size = (w, h);
    }

    /// Events still queued.
    #[must_use]
    pub fn len(&self) -> usize {
        lock(&self.0).queue.len()
    }

    /// True when nothing is queued.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The screen cell where the application showed its text cursor after
    /// the last frame, or `None` while it hides it.
    ///
    /// Turbo-vision has no terminal cursor to defer to under a
    /// [`HostBackend`], so the host draws one itself from this: a real
    /// cursor where the host can place one, or the cell redrawn in reverse
    /// video where it can't.
    #[must_use]
    pub fn cursor(&self) -> Option<(u16, u16)> {
        lock(&self.0).cursor
    }
}

/// The application's end: a [`Backend`] whose input is whatever the host
/// pushed and whose output is the terminal buffer alone.
#[derive(Debug)]
pub struct HostBackend(Arc<Mutex<Shared>>);

impl HostBackend {
    /// A backend of `w` by `h` cells, and the handle that feeds it.
    #[must_use]
    pub fn new(w: u16, h: u16) -> (Self, HostInput) {
        let shared = Arc::new(Mutex::new(Shared {
            queue: VecDeque::new(),
            size: (w, h),
            cursor: None,
        }));
        (Self(Arc::clone(&shared)), HostInput(shared))
    }
}

impl Backend for HostBackend {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn is_host_driven(&self) -> bool {
        true
    }
    fn init(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn cleanup(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn size(&self) -> io::Result<(u16, u16)> {
        Ok(lock(&self.0).size)
    }
    fn poll_event(&mut self, _timeout: Duration) -> io::Result<Option<Event>> {
        Ok(lock(&self.0).queue.pop_front())
    }
    fn write_raw(&mut self, _data: &[u8]) -> io::Result<()> {
        Ok(())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn show_cursor(&mut self, x: u16, y: u16) -> io::Result<()> {
        lock(&self.0).cursor = Some((x, y));
        Ok(())
    }
    fn hide_cursor(&mut self) -> io::Result<()> {
        lock(&self.0).cursor = None;
        Ok(())
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            mouse: false,
            colors_256: true,
            true_color: true,
            ..Capabilities::default()
        }
    }
}

/// Runs one host-driven frame: re-lays out if the host changed the size,
/// handles every queued event, idles once and draws.
///
/// Returns whether the application is still running. It turns `false` once
/// a handler or view quits (`CM_QUIT`, Alt+X), and the host should then stop
/// stepping it and read its final state.
///
/// Nothing here waits. Each event goes through
/// [`Application::step`], which draws a pending redraw first, so a frame
/// that has several queued events leaves the screen as a terminal-driven
/// run of the same keys would.
pub fn pump<H: AppHandler>(app: &mut Application, handler: &mut H) -> bool {
    // The same re-layout a terminal resize gets: crossterm reports one as a
    // CM_REDRAW broadcast, which handle_event routes to handle_redraw.
    if let Ok(size) = app.terminal.backend_size()
        && size != app.terminal.size()
    {
        app.handle_redraw();
    }
    while app.running {
        let Some(event) = app.poll_event_or_quit() else {
            break;
        };
        app.step(handler, Some(event));
    }
    if app.running {
        app.step(handler, None);
    }
    app.draw();
    app.running
}

/// A host-driven [`Application`] of `w` by `h` cells, and the input handle
/// that feeds it.
///
/// The desktop is laid out as turbo-vision lays out any application: without
/// a menu bar or status line it keeps a spare row at the top and bottom, so
/// set those, or the desktop's bounds, as the host's screen needs.
///
/// # Panics
///
/// Never in practice: building a terminal only fails when its backend fails
/// to initialise or report a size, and a [`HostBackend`] does neither.
#[must_use]
pub fn app(w: u16, h: u16) -> (Application, HostInput) {
    let (backend, input) = HostBackend::new(w, h);
    let terminal =
        Terminal::with_backend(Box::new(backend)).expect("a HostBackend never fails to start");
    (Application::with_terminal(terminal), input)
}
