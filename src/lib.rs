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
//! - [`popup_menu`]: modal context menus and check-mark menu items, reusing
//!   the framework's `MenuBox`.
//!
//! Builds without turbo-vision's `native` feature, so the crate compiles
//! for `wasm32-wasip1`.

pub mod host;
pub mod popup_menu;
pub mod scroll_pane;

pub use host::{HostBackend, HostInput, pump};
pub use popup_menu::{is_menu_item_checked, popup_menu, set_menu_item_checked};
pub use scroll_pane::ScrollPane;
