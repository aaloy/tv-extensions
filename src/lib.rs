// (C) 2026 - Enzo Lombardi

//! Extensions for [turbo-vision](https://github.com/aovestdipaperino/turbo-vision-4-rust)
//! that version on their own, so the core stays small and stable.
//!
//! - [`host`]: host-driven applications, for an embedder that owns the
//!   screen and the event loop (a WASM guest such as a plank frame). Push
//!   events into a [`HostInput`], call [`pump`] once per frame, read the
//!   terminal buffer.
//! - [`scroll_pane`]: [`ScrollPane`], a scrolling viewport over a virtual
//!   area larger than its screen bounds (TV Tool Box style).
//! - [`dual_list`]: [`DualList`], two lists side by side for picking a
//!   subset of items, with filters (Django admin's `filter_horizontal`).
//! - [`mod@popup_menu`]: modal context menus and check-mark menu items,
//!   reusing the framework's `MenuBox`.
//! - [`autocomplete`]: [`AutoComplete`], a text field that filters a
//!   suggestion list as you type.
//! - [`keys`]: key events from key names such as "ctrl-s" or "enter", the
//!   form a host that is not a terminal reports keys in.
//! - `csv` (feature `csv`): a CSV table editor, host-driven like [`host`].
//! - `log` (feature `log`): a scrolling output pane and a window that
//!   shows `tracing` events in it.
//! - `graphics` (feature `graphics`): ANSI-art backgrounds and bitmap
//!   images over the Kitty graphics protocol.
//! - `remote_input` (feature `remote-input`): key chords typed over TCP,
//!   for automation.
//! - `ssh` (feature `ssh`): serving a turbo-vision application over SSH.
//!
//! The four feature-gated module names above are plain code spans, not
//! links: with any one feature off, the module does not exist to link to,
//! and an intra-doc link that resolves only with that feature enabled would
//! make `cargo doc` (built with the crate's default, empty feature set)
//! warn about a broken link.
//!
//! Builds without turbo-vision's `native` feature, so the crate compiles
//! for `wasm32-wasip1`.

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod autocomplete;
#[cfg(feature = "csv")]
#[cfg_attr(docsrs, doc(cfg(feature = "csv")))]
pub mod csv;
pub mod dual_list;
#[cfg(feature = "graphics")]
#[cfg_attr(docsrs, doc(cfg(feature = "graphics")))]
pub mod graphics;
pub mod host;
pub mod keys;
#[cfg(feature = "log")]
#[cfg_attr(docsrs, doc(cfg(feature = "log")))]
pub mod log;
pub mod popup_menu;
#[cfg(feature = "remote-input")]
#[cfg_attr(docsrs, doc(cfg(feature = "remote-input")))]
pub mod remote_input;
pub mod scroll_pane;
#[cfg(feature = "ssh")]
#[cfg_attr(docsrs, doc(cfg(feature = "ssh")))]
pub mod ssh;

pub use autocomplete::{AutoComplete, AutoCompleteBuilder};
pub use dual_list::{DualList, DualListBuilder};
pub use host::{HostBackend, HostInput, pump};
pub use popup_menu::{is_menu_item_checked, popup_menu, set_menu_item_checked};
pub use scroll_pane::ScrollPane;
