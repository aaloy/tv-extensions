// (C) 2026 - Antoni Aloy
//! The gallery's list of demos.
//!
//! Each entry ties a demo file to what the gallery needs to show it. The
//! file's own text is the code shown (`include_str!`), so adding a demo is:
//! write `demos/<name>.rs`, add a `pub mod` line in `demos/mod.rs`, and an
//! entry here.
//!
//! The list groups the demos by the kind of component, in the order of
//! [`Group`], and orders them by name within a group.

use super::demos;
use super::panel::Panel;
use std::ops::RangeInclusive;
use turbo_vision::app::Application;
use turbo_vision::core::command::{CM_USER, CommandId};

/// The commands demos may use; the gallery's own sit below them. Demos
/// share them, one at a time, so the gallery enables them all again before
/// it shows a demo: a demo that disables one must not grey out the next
/// demo's.
pub const DEMO_COMMANDS: RangeInclusive<CommandId> = CM_USER + 100..=CM_USER + 199;

/// The columns a demo may use: the panel's width on an 80-column terminal,
/// inside its "Try it" box.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the tests check every demo against it")
)]
pub const PANEL_WIDTH: i16 = 48;

/// The kinds of component the list groups the demos under, in list order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    /// Views to put in a dialog or window.
    Views,
    /// Menus that open where the application says.
    Menus,
    /// Views that show output and logs.
    Output,
    /// Pictures in a text UI.
    Graphics,
    /// Applications stepped by a host, and the keys it sends them.
    Hosting,
}

impl Group {
    /// The heading shown above the group in the list.
    pub fn title(self) -> &'static str {
        match self {
            Group::Views => "Views",
            Group::Menus => "Menus",
            Group::Output => "Output and logs",
            Group::Graphics => "Graphics",
            Group::Hosting => "Host-driven",
        }
    }
}

/// One component's demo.
pub struct Demo {
    /// The name in the list and the panel's title.
    pub name: &'static str,
    /// The kind of component, which the list groups it under.
    pub group: Group,
    /// The module or item, under `tv_extensions::`.
    pub module: &'static str,
    /// The demo file, as written: a `//!` header, then the code.
    pub source: &'static str,
    /// Rows the live demo takes.
    pub height: i16,
    /// Put the demo's views in the panel, from `(0, 0)` and at most
    /// [`PANEL_WIDTH`] wide.
    pub build: fn(&mut Panel),
    /// React to the demo's commands; `true` when it handled one.
    pub handle: Option<fn(&mut Application, CommandId) -> bool>,
}

/// Every demo, in the order of the list: by group, then by name.
pub const DEMOS: &[Demo] = &[
    Demo {
        name: "DualList",
        group: Group::Views,
        module: "dual_list",
        source: include_str!("demos/dual_list.rs"),
        height: 10,
        build: demos::dual_list::build,
        handle: None,
    },
    Demo {
        name: "ScrollPane",
        group: Group::Views,
        module: "scroll_pane",
        source: include_str!("demos/scroll_pane.rs"),
        height: 5,
        build: demos::scroll_pane::build,
        handle: None,
    },
    Demo {
        name: "Popup menu",
        group: Group::Menus,
        module: "popup_menu",
        source: include_str!("demos/popup_menu.rs"),
        height: 2,
        build: demos::popup_menu::build,
        handle: Some(demos::popup_menu::handle),
    },
    Demo {
        name: "LogWindow",
        group: Group::Output,
        module: "log::LogWindow",
        source: include_str!("demos/log_window.rs"),
        height: 2,
        build: demos::log_window::build,
        handle: Some(demos::log_window::handle),
    },
    Demo {
        name: "TerminalWidget",
        group: Group::Output,
        module: "log::TerminalWidget",
        source: include_str!("demos/terminal_widget.rs"),
        height: 6,
        build: demos::terminal_widget::build,
        handle: Some(demos::terminal_widget::handle),
    },
    Demo {
        name: "AnsiBackground",
        group: Group::Graphics,
        module: "graphics::AnsiBackground",
        source: include_str!("demos/ansi_background.rs"),
        height: 5,
        build: demos::ansi_background::build,
        handle: Some(demos::ansi_background::handle),
    },
    Demo {
        name: "CSV editor",
        group: Group::Hosting,
        module: "csv",
        source: include_str!("demos/csv_editor.rs"),
        height: 2,
        build: demos::csv_editor::build,
        handle: Some(demos::csv_editor::handle),
    },
    Demo {
        name: "Host-driven app",
        group: Group::Hosting,
        module: "host",
        source: include_str!("demos/host_app.rs"),
        height: 7,
        build: demos::host_app::build,
        handle: None,
    },
    Demo {
        name: "Key names",
        group: Group::Hosting,
        module: "keys",
        source: include_str!("demos/key_names.rs"),
        height: 2,
        build: demos::key_names::build,
        handle: Some(demos::key_names::handle),
    },
];

/// The line of a header that names related components.
const SEE_ALSO: &str = "See also:";

/// Split a demo file into its `//!` header, as plain text, and the code
/// after it.
pub fn split_source(source: &str) -> (String, String) {
    let mut how = Vec::new();
    let mut lines = source.lines().peekable();
    while let Some(line) = lines.next_if(|l| l.starts_with("//!")) {
        how.push(line.trim_start_matches("//!").trim_start().to_string());
    }
    // The blank line between the header and the code.
    lines.next_if(|l| l.trim().is_empty());
    let code: Vec<&str> = lines.collect();
    (how.join("\n"), code.join("\n"))
}

/// Take the "See also:" line out of a header: the header without it, and
/// the names it lists.
pub fn split_see_also(how: &str) -> (String, Vec<String>) {
    let mut related = Vec::new();
    let mut text = Vec::new();
    for line in how.lines() {
        match line.strip_prefix(SEE_ALSO) {
            Some(names) => related.extend(
                names
                    .split(',')
                    .map(|name| name.trim().trim_end_matches('.').to_string())
                    .filter(|name| !name.is_empty()),
            ),
            None => text.push(line),
        }
    }
    (text.join("\n").trim_end().to_string(), related)
}

/// The demo a "See also" name refers to: the one with that name, or whose
/// name lists it (`Label` is the `StaticText, Label` demo).
pub fn find(name: &str) -> Option<usize> {
    DEMOS
        .iter()
        .position(|d| d.name == name || d.name.split(", ").any(|part| part == name))
}
