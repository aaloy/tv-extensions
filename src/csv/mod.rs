// (C) 2026 - Enzo Lombardi

//! A CSV table editor: a [`Session`] holds one document in a window with a
//! menu bar, a status line and dialogs for editing cells, columns and rows.
//!
//! The session is host-driven (see [`crate::host`]): the caller pushes one key
//! with [`Session::key`], asks for a frame with [`Session::step`], and reads
//! the finished cells from [`Session::buffer`]. Documents live on a
//! [`Disk`]: [`FsDisk`] for a directory, [`MemDisk`] for tests.

pub mod commands;
mod dialogs;
pub mod disk;
pub mod doc;
pub mod editor;
pub mod format;

pub use disk::{Disk, FsDisk, MemDisk};
pub use doc::CsvDoc;
pub use editor::{OPEN_DIALOG_ARG, Session};
