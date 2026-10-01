// (C) 2026 - Enzo Lombardi

//! Extensions for [turbo-vision](https://github.com/aovestdipaperino/turbo-vision-4-rust)
//! that version on their own, so the core stays small and stable.
//!
//! - [`host`]: host-driven applications, for an embedder that owns the
//!   screen and the event loop (a WASM guest such as a plank frame). Push
//!   events into a [`HostInput`], call [`pump`] once per frame, read the
//!   terminal buffer.
//!
//! Builds without turbo-vision's `native` feature, so the crate compiles
//! for `wasm32-wasip1`.

pub mod host;

pub use host::{HostBackend, HostInput, pump};
