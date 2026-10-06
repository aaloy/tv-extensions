// (C) 2026 - Antoni Aloy
//! One file per component. Each file starts with a `//!` header, shown in
//! the gallery as "How it works"; the rest of the file is shown as the code.
//!
//! The header has three parts, a blank `//!` line between them: what the
//! component does and its keys; "Parameters:", one "- " item per
//! constructor argument or setter, saying what it changes; and a "See
//! also:" line naming up to three related demos, which the gallery shows
//! as buttons. The tests check all three.

// The headers are shown as plain text in the gallery, where backticks would
// appear as they are; key names like PgUp are not code.
#![allow(
    clippy::doc_markdown,
    reason = "demo headers are displayed as plain text in the gallery"
)]

pub mod ansi_background;
pub mod autocomplete;
pub mod csv_editor;
pub mod dual_list;
pub mod host_app;
pub mod key_names;
pub mod log_window;
pub mod popup_menu;
pub mod scroll_pane;
pub mod terminal_widget;
