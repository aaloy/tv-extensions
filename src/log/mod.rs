// (C) 2026 - Enzo Lombardi

//! A scrolling output pane ([`TerminalWidget`]) and a window that shows
//! `tracing` events in it ([`LogWindow`], [`LogSubscriber`]).

mod log_window;
mod terminal_widget;

pub use log_window::*;
pub use terminal_widget::*;
