// (C) 2026 - Enzo Lombardi

//! Extensions for [turbo-vision](https://github.com/aovestdipaperino/turbo-vision-4-rust)
//! that version on their own, so the core stays small and stable.
//!
//! - [`host`]: host-driven applications, for an embedder that owns the
//!   screen and the event loop (a WASM guest such as a plank frame). Push
//!   events into a [`HostInput`], call [`pump`] once per frame, read the
//!   terminal buffer.
//! - [`capture`] (feature `capture`): Ctrl+F12 PNG and F12 ANSI screen
//!   captures, installed on the terminal's capture hook.
//! - [`scroll_pane`]: [`ScrollPane`], a scrolling viewport over a virtual
//!   area larger than its screen bounds (TV Tool Box style).
//! - [`popup_menu`]: modal context menus and check-mark menu items, reusing
//!   the framework's `MenuBox`.
//! - [`keys`]: key events from key names such as "ctrl-s" or "enter", the
//!   form a host that is not a terminal reports keys in.
//! - [`csv`] (feature `csv`): a CSV table editor, host-driven like [`host`].
//! - [`log`] (feature `log`): a scrolling output pane and a window that
//!   shows `tracing` events in it.
//! - [`graphics`] (feature `graphics`): ANSI-art backgrounds and bitmap
//!   images over the Kitty graphics protocol.
//! - [`remote_input`] (feature `remote-input`): key chords typed over TCP,
//!   for automation.
//! - [`ssh`] (feature `ssh`): serving a turbo-vision application over SSH.
//!
//! Builds without turbo-vision's `native` feature, so the crate compiles
//! for `wasm32-wasip1`.

#[cfg(feature = "capture")]
pub mod capture;
#[cfg(feature = "csv")]
pub mod csv;
#[cfg(feature = "graphics")]
pub mod graphics;
pub mod host;
pub mod keys;
#[cfg(feature = "log")]
pub mod log;
pub mod popup_menu;
#[cfg(feature = "remote-input")]
pub mod remote_input;
pub mod scroll_pane;
#[cfg(feature = "ssh")]
pub mod ssh;

pub use host::{HostBackend, HostInput, pump};
pub use popup_menu::{is_menu_item_checked, popup_menu, set_menu_item_checked};
pub use scroll_pane::ScrollPane;
