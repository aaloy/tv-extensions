//! Component Gallery - every tv-extensions component, live, with how it works
//! and its code. Modelled on turbo-vision's own gallery.
//!
//! The list on the left names the components, grouped by kind; the panel on
//! the right shows the one under the list's focus: the live component, "How
//! it works" with its parameters, links to related components, and the code
//! that built it. That code is the demo's source file, shown as written, so
//! what you read is what runs.
//!
//! Run with: cargo run --example gallery --features native,csv,log,graphics
//! Arrows move through the list; F6 switches to the panel to try a
//! component (Tab moves between its controls, the text, the links and the
//! code); F6 again goes back.
//!
//! The gallery follows the terminal: resize it and the list keeps its width
//! and takes the new height, while the panel is laid out again for the new
//! size (the "How it works" text is re-wrapped to it).

// (C) 2026 - Antoni Aloy

mod demos;
mod group_box;
mod panel;
mod registry;

use group_box::GroupBox;
use registry::{DEMO_COMMANDS, DEMOS, Demo, Group};
use std::cell::RefCell;
use std::rc::Rc;
use turbo_vision::app::{AppHandler, Application};
use turbo_vision::core::command::{CM_QUIT, CM_USER, CommandId};
use turbo_vision::core::event::Event;
use turbo_vision::core::geometry::{Point, Rect};
use turbo_vision::core::palette::palettes;
use turbo_vision::core::state::{Grow, State};
use turbo_vision::core::status_data::StatusItemBuilder;
use turbo_vision::views::button::Button;
use turbo_vision::views::dialog::Dialog;
use turbo_vision::views::editor::EditorWindow;
use turbo_vision::views::listbox::ListBox;
use turbo_vision::views::scrollbar::ScrollBar;
use turbo_vision::views::shared::Shared;
use turbo_vision::views::static_text::StaticText;
use turbo_vision::views::status_line::StatusLine;
use turbo_vision::views::syntax::RustHighlighter;
use turbo_vision::views::window::Window;
use turbo_vision::views::{GroupLike, Handle, View, ViewId};

/// How wide the component list is.
const LIST_WIDTH: i16 = 24;

/// The first of the gallery's "See also" commands: `SHOW_DEMO + i` shows
/// demo `i`. They sit below the demos' range.
const SHOW_DEMO: CommandId = CM_USER;

/// A row of the component list: a group's heading, or a demo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Row {
    Heading(Group),
    Demo(usize),
}

/// The list's rows: each group's heading, then the group's demos.
fn rows() -> Vec<Row> {
    let mut rows = Vec::new();
    for (index, demo) in DEMOS.iter().enumerate() {
        if index == 0 || DEMOS[index - 1].group != demo.group {
            rows.push(Row::Heading(demo.group));
        }
        rows.push(Row::Demo(index));
    }
    rows
}

/// What the list shows for `row`: a heading is a rule with the group's
/// title in it, so it reads apart from the names under it.
fn row_text(row: Row) -> String {
    match row {
        Row::Heading(group) => {
            let title = format!("\u{2500} {} ", group.title());
            let rule = usize::try_from(LIST_WIDTH - 3).unwrap_or(0);
            let fill = rule.saturating_sub(title.chars().count());
            title + &"\u{2500}".repeat(fill)
        }
        Row::Demo(index) => DEMOS[index].name.to_string(),
    }
}

/// The component list's palette: a blue window's, extended to the entries
/// a list box draws with (26 to 28), which take a dialog's list colours:
/// black on cyan, and white on green for the item under the focus. A list
/// box is made for dialogs (Borland's list viewer palette); a blue window's
/// palette stops short of those entries, and the list came out in light
/// green on gray.
fn list_palette() -> Vec<u8> {
    let mut palette = palettes::CP_BLUE_WINDOW.to_vec();
    // Entries 20 to 25 are not used by the list; they keep the window's
    // normal text colour.
    palette.resize(25, palettes::CP_BLUE_WINDOW[5]);
    palette.extend_from_slice(&palettes::CP_GRAY_DIALOG[25..28]);
    palette
}

/// The gallery's state: where the list is, and which demo the panel shows.
struct Gallery {
    list_window: Handle<Window>,
    list: Handle<ListBox>,
    rows: Vec<Row>,
    /// The list row of the demo shown.
    row: usize,
    panel: Option<ViewId>,
    shown: Option<usize>,
    /// The desktop size the panel was built for.
    laid_out: (i16, i16),
}

impl Gallery {
    fn open(app: &mut Application) -> Self {
        let desk = desktop_size(app);
        let mut window = Window::new(Rect::new(0, 0, LIST_WIDTH, desk.1), "Components");
        window.set_custom_palette(list_palette());
        no_shadow(&mut window);
        // A resize changes the list's height, never its width.
        window.set_grow_mode(Grow::HI_Y);
        let mut list = ListBox::new(Rect::new(0, 0, LIST_WIDTH - 2, desk.1 - 2), 0);
        list.set_grow_mode(Grow::HI_Y);
        let rows = rows();
        list.set_items(rows.iter().map(|&row| row_text(row)).collect());
        let list = window.add_typed(list);
        let list_window = app.desktop.add_typed(window);
        Self {
            list_window,
            list,
            rows,
            row: 0,
            panel: None,
            shown: None,
            laid_out: desk,
        }
    }

    /// The list row under the list's focus.
    fn selected(&self, app: &Application) -> Option<usize> {
        app.desktop
            .get(self.list_window)?
            .get(self.list)?
            .get_selection()
    }

    /// Put the list's focus on `row`.
    fn select(&self, app: &mut Application, row: usize) {
        if let Some(list) = app
            .desktop
            .get_mut(self.list_window)
            .and_then(|window| window.get_mut(self.list))
        {
            list.set_selection(row);
        }
    }

    /// The list row of demo `index`.
    fn row_of(&self, index: usize) -> usize {
        self.rows
            .iter()
            .position(|&row| row == Row::Demo(index))
            .unwrap_or(0)
    }

    /// Show demo `index` in a fresh panel, keeping the focus in the list.
    fn show(&mut self, app: &mut Application, index: usize) {
        if let Some(old) = self.panel.take() {
            app.desktop.remove_child_by_id(old);
        }
        for command in DEMO_COMMANDS {
            app.enable_command(command);
        }
        self.row = self.row_of(index);
        self.select(app, self.row);
        let (width, height) = desktop_size(app);
        let bounds = Rect::new(LIST_WIDTH, 0, width, height);
        self.panel = Some(app.desktop.add(panel(&DEMOS[index], bounds)));
        self.shown = Some(index);
        self.laid_out = (width, height);
        app.desktop.bring_to_front(self.list_window.id());
        app.needs_redraw();
    }
}

impl AppHandler for Gallery {
    fn idle(&mut self, app: &mut Application) {
        let Some(mut row) = self.selected(app) else {
            return;
        };
        // A heading is not a demo: step over it the way the focus moved,
        // so Up from a group's first demo reaches the group above.
        if matches!(self.rows.get(row), Some(Row::Heading(_))) {
            row = if row < self.row && row > 0 {
                row - 1
            } else {
                row + 1
            };
            self.select(app, row);
        }
        let Some(&Row::Demo(index)) = self.rows.get(row) else {
            return;
        };
        // The panel follows the list, and the terminal: grow modes
        // stretch it at once, but its text was wrapped to the old width, so
        // it is built again for the new size.
        let resized = desktop_size(app) != self.laid_out;
        if resized || Some(index) != self.shown {
            self.show(app, index);
        }
    }

    fn handle_command(&mut self, app: &mut Application, command: CommandId, _: &Event) -> bool {
        // A "See also" link moves the list to its demo; the next idle tick
        // shows it, once the panel that sent the command is done with it.
        if let Some(index) = command
            .checked_sub(SHOW_DEMO)
            .map(usize::from)
            .filter(|&i| i < DEMOS.len())
        {
            self.select(app, self.row_of(index));
            return true;
        }
        // Only the shown demo is asked about its commands.
        self.shown
            .and_then(|i| DEMOS[i].handle)
            .is_some_and(|handle| handle(app, command))
    }

    fn window_closed(&mut self, _app: &mut Application, id: ViewId) {
        // A closed panel is rebuilt on the next idle tick.
        if Some(id) == self.panel {
            self.panel = None;
            self.shown = None;
        }
    }
}

/// Side by side, the list and the panel fill the desktop: without a shadow
/// a window may reach its edges.
fn no_shadow(window: &mut impl View) {
    window.set_state(window.state() & !State::SHADOW);
}

/// Re-flow `text` to lines at most `width` wide: lines that follow each
/// other form one paragraph, and a blank line starts a new one. A line
/// starting with "- " is a list item: it starts a line of its own, and
/// its continuation lines are indented under its text.
fn wrap(text: &str, width: usize) -> String {
    let shown = |s: &str| s.chars().filter(|&c| c != '~').count();
    let mut out: Vec<String> = Vec::new();
    for paragraph in text.split("\n\n") {
        // The paragraph's items: its text, or each "- " item and the lines
        // that continue it.
        let mut items: Vec<String> = Vec::new();
        for line in paragraph.lines() {
            match items.last_mut() {
                Some(item) if !line.starts_with("- ") => {
                    item.push(' ');
                    item.push_str(line);
                }
                _ => items.push(line.to_string()),
            }
        }
        for item in items {
            let indent = if item.starts_with("- ") { "  " } else { "" };
            let mut line = String::new();
            for word in item.split_whitespace() {
                if !line.is_empty() && shown(&line) + 1 + shown(word) > width {
                    out.push(std::mem::take(&mut line));
                    line.push_str(indent);
                }
                if !line.trim_start().is_empty() {
                    line.push(' ');
                }
                line.push_str(word);
            }
            out.push(line);
        }
        out.push(String::new());
    }
    out.pop();
    out.join("\n")
}

/// The desktop's width and height.
fn desktop_size(app: &Application) -> (i16, i16) {
    let b = app.desktop.get_bounds();
    (b.width(), b.height())
}

/// Rows the code may shrink to and still be worth showing: two lines of
/// code and the scroll bar under them.
const MIN_CODE_ROWS: i16 = 3;

/// The panel for `demo`, top to bottom: the live component in a "Try it"
/// box; "How it works", with the parameters, in a box of its own; links to
/// related components; and the code, in a "Code" box. The boxes keep the
/// explanation and the code apart.
fn panel(demo: &Demo, bounds: Rect) -> Dialog {
    let mut dialog = Dialog::new(bounds, demo.name);
    no_shadow(&mut dialog);
    // Until it is rebuilt after a resize, the panel stretches with the
    // desktop: its right and bottom edges follow the terminal's.
    dialog.set_grow_mode(Grow::HI_X | Grow::HI_Y);
    let inner = bounds.width() - 2;
    let rows = bounds.height() - 2;
    let (how, code) = registry::split_source(demo.source);
    let (how, related) = registry::split_see_also(&how);
    // Inside a box, one column of margin on each side and one for the
    // scroll bar a long text gets.
    let how = wrap(&how, usize::try_from(inner - 7).unwrap_or(1));

    // Module path on the first row.
    dialog.add(StaticText::new(
        Rect::new(1, 0, inner - 1, 1),
        &format!("tv_extensions::{}", demo.module),
    ));

    // The live component, inside a box.
    let top = 1;
    let box_height = demo.height + 2;
    let mut try_it = GroupBox::new(
        Rect::new(1, top, inner - 1, top + box_height),
        "Try it (F6)",
    );
    try_it.set_grow_mode(Grow::HI_X);
    dialog.add(try_it);
    (demo.build)(&mut panel::Panel::new(&mut dialog, Point::new(3, top + 1)));

    // The rows left share out between the two boxes' contents: the text
    // takes what it needs, up to half when the code needs the rest.
    let how_top = top + box_height;
    let links = if related.is_empty() { 0 } else { 2 };
    let content = (rows - how_top - links - 4).max(1);
    let how_lines = i16::try_from(how.lines().count()).unwrap_or(1).max(1);
    let code_lines = i16::try_from(code.lines().count()).unwrap_or(1) + 1;
    let mut how_rows = how_lines.min(content - code_lines.min(content / 2)).max(1);
    let mut code_rows = content - how_rows;
    if code_rows < MIN_CODE_ROWS {
        // No room for the code: the text takes its box too.
        how_rows = (content + 2).min(how_lines).max(1);
        code_rows = 0;
    }

    // How it works: the demo file's `//!` header, scrolling when it is
    // longer than its box.
    let mut how_box = GroupBox::new(
        Rect::new(1, how_top, inner - 1, how_top + how_rows + 2),
        "How it works",
    );
    how_box.set_grow_mode(Grow::HI_X);
    dialog.add(how_box);
    if how_lines <= how_rows {
        dialog.add(StaticText::new(
            Rect::new(3, how_top + 1, inner - 3, how_top + 1 + how_rows),
            &how,
        ));
    } else {
        // Static text hides the hot-key tildes, so the scrolling text does
        // too; it is wrapped, so it needs no horizontal scroll bar.
        add_text(
            &mut dialog,
            Rect::new(3, how_top + 1, inner - 2, how_top + 1 + how_rows),
            &how.replace('~', ""),
        );
    }

    // See also: a button per related component.
    let links_top = how_top + how_rows + 2;
    if links > 0 {
        add_links(&mut dialog, links_top, inner - 1, &related);
    }

    // The code: the rest of the file, as written.
    if code_rows > 0 {
        let code_top = links_top + links;
        let mut code_box = GroupBox::new(
            Rect::new(1, code_top, inner - 1, code_top + code_rows + 2),
            "Code",
        );
        code_box.set_grow_mode(Grow::HI_X);
        dialog.add(code_box);
        add_code(
            &mut dialog,
            Rect::new(2, code_top + 1, inner - 2, code_top + 1 + code_rows),
            &code,
        );
    }
    dialog
}

/// A "See also" caption on row `top`, then a button per name in `related`
/// that shows that demo, as many as fit before column `right`.
fn add_links(dialog: &mut Dialog, top: i16, right: i16, related: &[String]) {
    let caption = "See also:";
    let mut x = 2 + i16::try_from(caption.len()).unwrap_or(0) + 1;
    dialog.add(StaticText::new(Rect::new(2, top, x - 1, top + 1), caption));
    for name in related {
        let Some(index) = registry::find(name) else {
            continue;
        };
        let Ok(offset) = CommandId::try_from(index) else {
            continue;
        };
        // A space each side of the name, and the shadow's column.
        let width = i16::try_from(name.chars().count()).unwrap_or(0) + 3;
        if x + width > right {
            break;
        }
        dialog.add(Button::new(
            Rect::new(x, top, x + width, top + 2),
            name,
            SHOW_DEMO + offset,
            false,
        ));
        x += width + 1;
    }
}

/// Show `text` in `area` of `dialog`: a read-only editor with a scroll bar
/// on its right.
fn add_text(dialog: &mut Dialog, area: Rect, text: &str) {
    let right = area.b.x - 1;
    let mut v_bar = ScrollBar::new_vertical(Rect::new(right, area.a.y, area.b.x, area.b.y));
    v_bar.set_grow_mode(Grow::LO_X | Grow::HI_X);
    let v_bar = Rc::new(RefCell::new(v_bar));
    let mut editor = EditorWindow::with_scrollbars(
        Rect::new(area.a.x, area.a.y, right, area.b.y),
        None,
        Some(Rc::clone(&v_bar)),
        None,
    );
    editor.set_text(text);
    editor.set_read_only(true);
    dialog.add(editor);
    dialog.add(Shared::new(v_bar));
}

/// Show `code` in `area` of `dialog`, coloured as Rust: a read-only editor
/// with a scroll bar on its right and one along its bottom.
fn add_code(dialog: &mut Dialog, area: Rect, code: &str) {
    let (right, bottom) = (area.b.x - 1, area.b.y - 1);
    let mut v_bar = ScrollBar::new_vertical(Rect::new(right, area.a.y, area.b.x, bottom));
    v_bar.set_grow_mode(Grow::LO_X | Grow::HI_X | Grow::HI_Y);
    let mut h_bar = ScrollBar::new_horizontal(Rect::new(area.a.x, bottom, right, area.b.y));
    h_bar.set_grow_mode(Grow::LO_Y | Grow::HI_Y | Grow::HI_X);
    let (v_bar, h_bar) = (Rc::new(RefCell::new(v_bar)), Rc::new(RefCell::new(h_bar)));

    let mut editor = EditorWindow::with_scrollbars(
        Rect::new(area.a.x, area.a.y, right, bottom),
        Some(Rc::clone(&h_bar)),
        Some(Rc::clone(&v_bar)),
        None,
    );
    editor.set_highlighter(Box::new(RustHighlighter::new()));
    editor.set_text(code);
    editor.set_read_only(true);
    dialog.add(editor);
    dialog.add(Shared::new(v_bar));
    dialog.add(Shared::new(h_bar));
}

fn main() -> turbo_vision::core::error::Result<()> {
    let mut app = Application::new()?;
    let (width, height) = app.terminal.size();
    app.set_status_line(StatusLine::new(
        Rect::new(0, height - 1, width, height),
        vec![
            StatusItemBuilder::new()
                .text("~Alt-X~ Exit")
                .key("Alt+X")
                .command(CM_QUIT)
                .build(),
            StatusItemBuilder::new().text("~F6~ List/Panel").build(),
            StatusItemBuilder::new().text("~Tab~ Next control").build(),
        ],
    ));
    let mut gallery = Gallery::open(&mut app);
    gallery.show(&mut app, 0);
    app.run_with(&mut gallery);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use registry::PANEL_WIDTH;

    /// A dialog holding only what `demo` builds, with the panel's corner
    /// at (0, 0).
    fn built(demo: &Demo) -> Dialog {
        // As `Gallery::show` does: a command an earlier demo disabled
        // would grey out this demo's button.
        for command in DEMO_COMMANDS {
            turbo_vision::core::command_set::enable_command(command);
        }
        let mut dialog = Dialog::new(Rect::new(0, 0, PANEL_WIDTH + 2, demo.height + 2), "T");
        (demo.build)(&mut panel::Panel::new(&mut dialog, Point::new(0, 0)));
        dialog
    }

    #[test]
    fn every_demo_builds_inside_its_panel() {
        for demo in DEMOS {
            let live = built(demo);
            assert!(live.child_count() > 0, "{} shows nothing", demo.name);
            for i in 0..live.child_count() {
                let b = live.child_at(i).bounds();
                assert!(
                    b.a.x >= 0 && b.a.y >= 0 && b.b.x <= PANEL_WIDTH && b.b.y <= demo.height,
                    "{}: a view at {b:?} is outside {}x{}",
                    demo.name,
                    PANEL_WIDTH,
                    demo.height
                );
            }
        }
    }

    #[test]
    fn every_demo_explains_how_it_works() {
        for demo in DEMOS {
            let (how, code) = registry::split_source(demo.source);
            assert!(
                how.lines().count() >= 2,
                "{}: write a //! header",
                demo.name
            );
            assert!(
                how.contains("\nParameters:\n- "),
                "{}: list the parameters under \"Parameters:\"",
                demo.name
            );
            assert!(
                how.contains("\nSee also: "),
                "{}: link related components with \"See also:\"",
                demo.name
            );
            assert!(
                code.contains("pub fn build"),
                "{}: no build function",
                demo.name
            );
            assert!(
                !code.starts_with('\n'),
                "{}: one blank line after the header",
                demo.name
            );
        }
    }

    #[test]
    fn a_demo_s_hot_keys_are_unique() {
        for demo in DEMOS {
            let (_, code) = registry::split_source(demo.source);
            let mut seen = std::collections::HashSet::new();
            // Only the views the demo builds into its panel, not the
            // dialogs its handler opens.
            let build = code.split("pub fn handle").next().unwrap_or_default();
            // A hot key is one letter; a longer marker, ~F2~ in a status
            // line item, highlights a key name.
            let hot_keys = build.split('~').skip(1).step_by(2);
            for part in hot_keys.filter(|p| p.chars().count() == 1) {
                let key = part.to_ascii_lowercase();
                assert!(
                    seen.insert(key.clone()),
                    "{}: two ~{key}~ hot keys",
                    demo.name
                );
            }
        }
    }

    #[test]
    fn demo_commands_stay_in_their_range() {
        // CM_USER + 100 to CM_USER + 199 belong to the demos.
        for demo in DEMOS {
            let (_, code) = registry::split_source(demo.source);
            for line in code.lines().filter(|l| l.contains(": CommandId = CM_USER")) {
                let offset: u16 = line
                    .split("CM_USER +")
                    .nth(1)
                    .and_then(|n| n.trim().trim_end_matches(';').parse().ok())
                    .unwrap_or_else(|| panic!("{}: `{line}`", demo.name));
                assert!(
                    DEMO_COMMANDS.contains(&(CM_USER + offset)),
                    "{}: `{line}` is outside {DEMO_COMMANDS:?}",
                    demo.name
                );
            }
        }
    }

    /// Modules with no demo of their own, and why.
    const NOT_DEMOED: &[(&str, &str)] = &[
        (
            "remote_input",
            "key chords typed over TCP, for automation; no view",
        ),
        (
            "ssh",
            "serves an application over SSH; see the ssh_server example",
        ),
    ];

    #[test]
    fn every_module_has_a_demo_or_a_reason() {
        // A demo covers the modules its code imports.
        let src = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
        let mut modules: Vec<String> = std::fs::read_dir(src)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .map(|name| name.trim_end_matches(".rs").to_string())
            .filter(|name| name != "lib")
            .collect();
        modules.sort();
        let imported = |module: &str| {
            let path = format!("tv_extensions::{module}");
            DEMOS.iter().any(|d| {
                d.source.lines().any(|l| {
                    l.split_once(&path).is_some_and(|(_, rest)| {
                        !rest.starts_with(char::is_alphanumeric) && !rest.starts_with('_')
                    })
                })
            })
        };
        for module in &modules {
            let reason = NOT_DEMOED.iter().any(|(m, _)| m == module);
            assert!(
                imported(module) || reason,
                "{module} has no demo: write one, or add it to NOT_DEMOED with a reason"
            );
            assert!(
                !(imported(module) && reason),
                "{module} has a demo now: take it out of NOT_DEMOED"
            );
        }
        for (module, _) in NOT_DEMOED {
            assert!(modules.contains(&module.to_string()), "{module} is gone");
        }
    }

    #[test]
    fn wrap_reflows_paragraphs_to_the_width() {
        let text = "one two three\nfour five\n\nsix";
        assert_eq!(wrap(text, 9), "one two\nthree\nfour five\n\nsix");
        assert_eq!(wrap("press ~O~K now", 6), "press\n~O~K now");
    }

    #[test]
    fn wrap_keeps_list_items_apart_and_indents_them() {
        let text = "Parameters:\n- one two\nthree four\n- five";
        assert_eq!(
            wrap(text, 9),
            "Parameters:\n- one two\n  three\n  four\n- five"
        );
    }

    #[test]
    fn the_list_groups_the_demos_and_orders_them_by_name() {
        for pair in DEMOS.windows(2) {
            let key = |d: &Demo| (d.group, d.name.to_lowercase());
            assert!(
                key(&pair[0]) < key(&pair[1]),
                "{} should come after {}: by group, then by name",
                pair[0].name,
                pair[1].name
            );
        }
        let rows = rows();
        let headings = rows.iter().filter(|r| matches!(r, Row::Heading(_))).count();
        assert_eq!(rows.len(), DEMOS.len() + headings);
        assert!(matches!(rows[0], Row::Heading(Group::Views)));
    }

    #[test]
    fn see_also_names_other_demos_and_every_link_fits() {
        for demo in DEMOS {
            let (how, _) = registry::split_source(demo.source);
            let (text, related) = registry::split_see_also(&how);
            assert!(
                !text.contains("See also"),
                "{}: one See also line",
                demo.name
            );
            assert!(!related.is_empty(), "{}: no related components", demo.name);
            let mut linked = std::collections::HashSet::new();
            for name in &related {
                let index = registry::find(name)
                    .unwrap_or_else(|| panic!("{}: no demo named {name}", demo.name));
                assert!(linked.insert(index), "{}: two links to {name}", demo.name);
                assert_ne!(
                    DEMOS[index].name, demo.name,
                    "{}: links to itself",
                    demo.name
                );
            }
            // On an 80-column terminal every link gets its button.
            let dialog = panel(demo, Rect::new(LIST_WIDTH, 0, 80, 23));
            let links = (0..dialog.child_count())
                .filter(|&i| {
                    let b = dialog.child_at(i).bounds();
                    b.height() == 2 && b.a.y == dialog_links_row(&dialog)
                })
                .count();
            assert!(links >= related.len(), "{}: a link does not fit", demo.name);
        }
    }

    /// The row the "See also" caption is on.
    fn dialog_links_row(dialog: &Dialog) -> i16 {
        (0..dialog.child_count())
            .map(|i| dialog.child_at(i).bounds())
            .find(|b| b.a.x == 2 && b.width() == 9 && b.height() == 1)
            .map_or(-1, |b| b.a.y)
    }

    #[test]
    fn the_list_steps_over_group_headings() {
        let (mut app, _) = app(80, 25);
        let mut gallery = Gallery::open(&mut app);
        gallery.show(&mut app, 0);
        assert_eq!(gallery.row, 1, "the first demo sits under its heading");

        // Down from a group's last demo lands on the next heading: the
        // gallery moves on to the next group's first demo.
        let last = DEMOS.iter().rposition(|d| d.group == Group::Views).unwrap();
        gallery.show(&mut app, last);
        gallery.select(&mut app, gallery.row + 1);
        app.step(&mut gallery, None);
        assert_eq!(gallery.shown, Some(last + 1));

        // Up from it lands on the same heading: back to the group above.
        gallery.select(&mut app, gallery.row - 1);
        app.step(&mut gallery, None);
        assert_eq!(gallery.shown, Some(last));

        // Home lands on the first heading: the first demo.
        gallery.select(&mut app, 0);
        app.step(&mut gallery, None);
        assert_eq!(gallery.shown, Some(0));
    }

    #[test]
    fn the_list_draws_in_a_dialog_s_list_colours() {
        use turbo_vision::core::palette::{Attr, LISTBOX_NORMAL, LISTBOX_SELECTED, TvColor};
        let (mut app, _) = app(80, 25);
        let mut gallery = Gallery::open(&mut app);
        // A view learns its owners' palettes as it is drawn.
        app.step(&mut gallery, None);
        let list = app
            .desktop
            .get(gallery.list_window)
            .and_then(|w| w.get(gallery.list))
            .unwrap();
        assert_eq!(
            list.map_color(LISTBOX_NORMAL),
            Attr::new(TvColor::Black, TvColor::Cyan)
        );
        assert_eq!(
            list.map_color(LISTBOX_SELECTED),
            Attr::new(TvColor::White, TvColor::Green)
        );
    }

    #[test]
    fn a_see_also_link_shows_its_demo() {
        let (mut app, _) = app(80, 25);
        let mut gallery = Gallery::open(&mut app);
        gallery.show(&mut app, 0);
        let table = DEMOS.iter().position(|d| d.name == "DualList").unwrap();
        let command = SHOW_DEMO + CommandId::try_from(table).unwrap();
        assert!(gallery.handle_command(&mut app, command, &Event::nothing()));
        app.step(&mut gallery, None);
        assert_eq!(gallery.shown, Some(table));
        assert_eq!(gallery.selected(&app), Some(gallery.row_of(table)));
    }

    #[test]
    fn the_gallery_s_link_commands_stay_below_the_demos() {
        // Below CM_USER + 100 belongs to the gallery.
        let last = SHOW_DEMO + CommandId::try_from(DEMOS.len()).unwrap();
        assert!(last <= *DEMO_COMMANDS.start());
    }

    #[test]
    fn a_demo_s_controls_can_take_the_focus() {
        for demo in DEMOS {
            let live = built(demo);
            assert!(
                (0..live.child_count()).any(|i| live.child_at(i).can_focus()),
                "{}: nothing to try",
                demo.name
            );
        }
    }

    /// A terminal whose size the test changes, as a user resizing it does.
    struct Resizable(Arc<(AtomicU16, AtomicU16)>);

    impl turbo_vision::terminal::Backend for Resizable {
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
        fn init(&mut self) -> std::io::Result<()> {
            Ok(())
        }
        fn cleanup(&mut self) -> std::io::Result<()> {
            Ok(())
        }
        fn size(&self) -> std::io::Result<(u16, u16)> {
            Ok((
                self.0.0.load(Ordering::SeqCst),
                self.0.1.load(Ordering::SeqCst),
            ))
        }
        fn poll_event(&mut self, _: std::time::Duration) -> std::io::Result<Option<Event>> {
            Ok(None)
        }
        fn write_raw(&mut self, _: &[u8]) -> std::io::Result<()> {
            Ok(())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
        fn show_cursor(&mut self, _: u16, _: u16) -> std::io::Result<()> {
            Ok(())
        }
        fn hide_cursor(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    use std::sync::Arc;
    use std::sync::atomic::{AtomicU16, Ordering};

    /// An application on a [`Resizable`] terminal, and the handle that
    /// changes its size.
    fn app(width: u16, height: u16) -> (Application, Arc<(AtomicU16, AtomicU16)>) {
        let size = Arc::new((AtomicU16::new(width), AtomicU16::new(height)));
        let backend = Box::new(Resizable(Arc::clone(&size)));
        let terminal = turbo_vision::terminal::Terminal::with_backend(backend).unwrap();
        (Application::with_terminal(terminal), size)
    }

    #[test]
    fn the_gallery_follows_a_terminal_resize() {
        let (mut app, size) = app(80, 25);
        let mut gallery = Gallery::open(&mut app);
        gallery.show(&mut app, 0);

        size.0.store(120, Ordering::SeqCst);
        size.1.store(40, Ordering::SeqCst);
        app.step(&mut gallery, None); // idle: the application and the gallery see the new size

        let (width, height) = desktop_size(&app);
        let list = app.desktop.get(gallery.list_window).unwrap().bounds();
        assert_eq!((list.width(), list.height()), (LIST_WIDTH, height));
        let panel = app
            .desktop
            .child_by_id(gallery.panel.unwrap())
            .unwrap()
            .bounds();
        assert_eq!(panel, Rect::new(LIST_WIDTH, 0, width, height));
        assert_eq!(gallery.shown, Some(0), "the same demo is shown again");
    }

    #[test]
    fn the_panel_builds_for_every_demo() {
        for demo in DEMOS {
            let dialog = panel(demo, Rect::new(LIST_WIDTH, 0, 80, 23));
            assert_eq!(dialog.bounds().width(), 80 - LIST_WIDTH);
        }
    }

    fn key(code: turbo_vision::core::keys::KeyCode) -> Event {
        use turbo_vision::core::keys::{KeyEvent, KeyModifiers};
        Event::from_crossterm_key(KeyEvent::new(code, KeyModifiers::empty()))
    }

    /// Row `y` of the screen, as text.
    fn screen_row(app: &Application, y: usize) -> String {
        app.terminal.buffer()[y].iter().map(|c| c.ch).collect()
    }

    #[test]
    fn a_hosted_application_gets_the_keys_but_not_f6() {
        use demos::host_app::{Embedded, Notes};
        use turbo_vision::core::event::EventType;
        use turbo_vision::core::keys::KeyCode;
        let mut view = Embedded::new(Rect::new(0, 0, 46, 7), Notes::new(46, 7));
        view.set_focus(true);
        for c in "Ada".chars() {
            let mut event = key(KeyCode::Char(c));
            view.handle_event(&mut event);
            assert_eq!(event.what, EventType::Nothing, "the guest takes {c}");
        }
        // F6 is the gallery's: it moves between the list and the panel.
        let mut f6 = key(KeyCode::F(6));
        view.handle_event(&mut f6);
        assert_eq!(f6.what, EventType::Keyboard);

        let (mut app, _) = app(80, 25);
        view.draw(&mut app.terminal);
        let name_row = (0..7)
            .map(|y| screen_row(&app, y))
            .find(|r| r.contains("Name"));
        assert!(name_row.is_some_and(|r| r.contains("Ada")));
    }

    #[test]
    fn the_csv_editor_opens_in_a_window_and_takes_keys() {
        use turbo_vision::core::keys::KeyCode;
        let (mut app, _) = app(80, 25);
        let mut gallery = Gallery::open(&mut app);
        let csv = registry::find("CSV editor").unwrap();
        gallery.show(&mut app, csv);
        let open = *DEMO_COMMANDS.start();
        assert!(gallery.handle_command(&mut app, open, &Event::nothing()));
        // Down twice to Earth, Enter to edit it.
        for k in [KeyCode::Down, KeyCode::Down, KeyCode::Enter] {
            app.step(&mut gallery, Some(key(k)));
        }
        app.draw();
        let rows: Vec<String> = (0..25).map(|y| screen_row(&app, y)).collect();
        assert!(
            rows.iter().any(|r| r.contains("Neptune")),
            "the table shows"
        );
        assert!(
            rows.iter().any(|r| r.contains("Edit cell")),
            "Enter edits a cell"
        );
    }

    #[test]
    fn the_csv_editor_takes_the_mouse() {
        use turbo_vision::core::event::{EventType, MB_LEFT_BUTTON};
        let (mut app, _) = app(80, 25);
        let mut gallery = Gallery::open(&mut app);
        gallery.show(&mut app, registry::find("CSV editor").unwrap());
        assert!(gallery.handle_command(&mut app, *DEMO_COMMANDS.start(), &Event::nothing()));
        app.step(&mut gallery, None);
        app.draw();
        let rows = |app: &Application| -> Vec<String> { (0..25).map(|y| screen_row(app, y)).collect() };
        let find = |app: &Application, text: &str| -> Point {
            rows(app)
                .iter()
                .enumerate()
                .find_map(|(y, r)| {
                    let x = r.find(text)?;
                    let col = r[..x].chars().count();
                    Some(Point::new(i16::try_from(col).unwrap(), i16::try_from(y).unwrap()))
                })
                .unwrap_or_else(|| panic!("no {text} on the screen"))
        };
        let click = |app: &mut Application, gallery: &mut Gallery, at: Point, double: bool| {
            let down = Event::mouse(EventType::MouseDown, at, MB_LEFT_BUTTON, double);
            app.step(gallery, Some(down));
            app.step(gallery, Some(Event::mouse(EventType::MouseUp, at, 0, false)));
            app.draw();
        };

        // A click on the guest's menu bar drops its menu down.
        let file = find(&app, "File");
        click(&mut app, &mut gallery, file, false);
        let dropped = rows(&app)[usize::try_from(file.y).unwrap() + 2..].join("\n");
        assert!(dropped.contains("Save"), "the File menu opens:\n{}", rows(&app).join("\n"));
        // A click outside it closes it; a double click on Venus edits it.
        let neptune = find(&app, "Neptune");
        click(&mut app, &mut gallery, neptune, false);
        assert!(!rows(&app).join("\n").contains("Save as"), "the menu closes");
        let venus = find(&app, "Venus");
        click(&mut app, &mut gallery, venus, false);
        click(&mut app, &mut gallery, venus, true);
        assert!(
            rows(&app).iter().any(|r| r.contains("Edit cell")),
            "a double click edits the cell:\n{}",
            rows(&app).join("\n")
        );
    }
}
