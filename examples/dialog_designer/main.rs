// (C) 2026 - Antoni Aloy

//! The dialog designer: lay out a Turbo Vision dialog with the mouse and
//! keyboard, try it, and save it as JSON for `tv_extensions::dialog_spec`.
//!
//! ```sh
//! cargo run --example dialog_designer --features native,dialog -- [file.json]
//! ```
//!
//! The dialog being designed sits on the desktop, drawn by its real
//! controls. Click a control to select it, drag it to move it, drag its
//! yellow corner to resize it; drag the dialog's own corner to resize the
//! dialog. Ins (or a right click) inserts a control, Enter (or a double
//! click) edits its properties, F9 runs the dialog for real. Help > Keys
//! lists every key.

mod help;
mod keys;
mod model;
mod props;
mod surface;
mod templates;
mod tree;

#[cfg(test)]
use model::Place;
use model::{Arrange, Design, KINDS, SECTION_ENDS, SECTION_NAMES, Sel, section};
use serde_json::Value;
use std::cell::RefCell;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use surface::{Commands, Surface};
use tree::ControlTree;
use turbo_vision::app::{AppHandler, Application};
use turbo_vision::core::command::{CM_CANCEL, CM_NO, CM_OK, CM_QUIT, CM_USER, CM_YES, CommandId};
use turbo_vision::core::event::{Event, EventType, KB_ALT_X};
use turbo_vision::core::geometry::{Point, Rect};
use turbo_vision::core::menu_data::{Menu, MenuItem, MenuItemBuilder};
use turbo_vision::core::state::Grow;
use turbo_vision::core::status_data::StatusItemBuilder;
use turbo_vision::views::button::Button;
use turbo_vision::views::dialog::Dialog;
use turbo_vision::views::file_dialog::FileDialog;
use turbo_vision::views::listbox::ListBox;
use turbo_vision::views::menu_bar::{MenuBar, SubMenu};
use turbo_vision::views::msgbox::{confirmation_box, input_box, message_box_error, message_box_ok};
use turbo_vision::views::static_text::StaticText;
use turbo_vision::views::status_line::StatusLine;
use turbo_vision::views::text_viewer::TextViewer;
use turbo_vision::views::window::Window;
use turbo_vision::views::{GroupLike, Handle, View};
use tv_extensions::dialog_spec::{Control, DialogSpec, JsonDialog, Kind};
use tv_extensions::popup_menu;

const CM_NEW: CommandId = CM_USER;
const CM_OPEN_SPEC: CommandId = CM_USER + 1;
const CM_SAVE_SPEC: CommandId = CM_USER + 2;
const CM_SAVE_AS: CommandId = CM_USER + 3;
const CM_EXIT: CommandId = CM_USER + 4;
const CM_UNDO: CommandId = CM_USER + 5;
const CM_REDO: CommandId = CM_USER + 6;
const CM_PROPERTIES: CommandId = CM_USER + 7;
const CM_DIALOG_PROPERTIES: CommandId = CM_USER + 8;
const CM_DELETE: CommandId = CM_USER + 9;
const CM_DUPLICATE: CommandId = CM_USER + 10;
const CM_EARLIER: CommandId = CM_USER + 11;
const CM_LATER: CommandId = CM_USER + 12;
const CM_TEST: CommandId = CM_USER + 13;
const CM_SHOW_JSON: CommandId = CM_USER + 14;
const CM_CODE: CommandId = CM_USER + 15;
const CM_TAB_ORDER: CommandId = CM_USER + 16;
const CM_INSPECTOR: CommandId = CM_USER + 17;
const CM_INSERT_MENU: CommandId = CM_USER + 18;
const CM_KEYS: CommandId = CM_USER + 19;
const CM_ABOUT: CommandId = CM_USER + 20;
const CM_NEW_TEMPLATE: CommandId = CM_USER + 21;
const CM_SAVE_TEMPLATE: CommandId = CM_USER + 22;
const CM_MOVE_DROPPED: CommandId = CM_USER + 23;
/// `CM_INSERT_SECTION + i` opens the insert menu of section `i`.
const CM_INSERT_SECTION: CommandId = CM_USER + 80;
const CM_MOVE_MENU: CommandId = CM_USER + 24;
const CM_SET_ORDER: CommandId = CM_USER + 25;
const CM_HELP_ON: CommandId = CM_USER + 26;
const CM_HELP_CONTENTS: CommandId = CM_USER + 27;
const CM_TUTORIAL: CommandId = CM_USER + 28;
/// Arrange: the alignments in [`ALIGNMENTS`]' order, then same width,
/// same height, distribute across, distribute down.
const CM_ALIGN: CommandId = CM_USER + 30;
const CM_SAME_WIDTH: CommandId = CM_USER + 36;
const CM_SAME_HEIGHT: CommandId = CM_USER + 37;
const CM_DISTRIBUTE_ACROSS: CommandId = CM_USER + 38;
const CM_DISTRIBUTE_DOWN: CommandId = CM_USER + 39;
const ALIGNMENTS: [Arrange; 6] = [
    Arrange::Left,
    Arrange::Right,
    Arrange::Center,
    Arrange::Top,
    Arrange::Bottom,
    Arrange::Middle,
];
/// `CM_MOVE_TO + i` moves the selection to the i-th target of the move menu.
const CM_MOVE_TO: CommandId = CM_USER + 100;
/// `CM_INSERT + i` inserts `KINDS[i]`.
const CM_INSERT: CommandId = CM_USER + 50;

const INSPECTOR_WIDTH: i16 = 32;

/// The inspector window: the tree of controls over the selection's JSON.
#[derive(Clone, Copy)]
struct Inspector {
    window: Handle<Window>,
    tree: Handle<ControlTree>,
    text: Handle<TextViewer>,
}

/// The designer: the design, the views that show it, and its commands.
struct Designer {
    design: Rc<RefCell<Design>>,
    surface: Handle<Surface>,
    inspector: Option<Inspector>,
    /// The design revision the inspector shows.
    shown: u64,
}

impl Designer {
    fn surface_mut<'a>(&self, app: &'a mut Application) -> Option<&'a mut Surface> {
        app.desktop.get_mut(self.surface)
    }

    /// After a change from outside the surface: follow the dialog's size
    /// and refresh the inspector.
    fn refresh(&mut self, app: &mut Application) {
        let desk = app.desktop.get_bounds();
        let area = Rect::new(0, 0, desk.width(), desk.height());
        if let Some(surface) = self.surface_mut(app) {
            surface.area = area;
            surface.sync_bounds();
        }
        self.follow_tree(app);
        let revision = self.design.borrow().revision;
        if revision != self.shown {
            self.shown = revision;
            let text = inspector_text(&self.design.borrow());
            if let Some(inspector) = self.inspector
                && let Some(viewer) = app
                    .desktop
                    .get_mut(inspector.window)
                    .and_then(|w| w.get_mut(inspector.text))
            {
                viewer.set_text(&text);
            }
        }
    }

    fn tree_mut<'a>(&self, app: &'a mut Application) -> Option<&'a mut ControlTree> {
        let inspector = self.inspector?;
        app.desktop
            .get_mut(inspector.window)
            .and_then(|w| w.get_mut(inspector.tree))
    }

    /// Select in the design what the person moved to in the tree, then
    /// bring the tree up to date with the design.
    fn follow_tree(&mut self, app: &mut Application) {
        let Some(tree) = self.tree_mut(app) else {
            return;
        };
        let mut design = self.design.borrow_mut();
        if let Some(entry) = tree.moved() {
            if let Some((tabs, page)) = entry.page {
                design.show_page(tabs, page);
            } else {
                design.reveal(entry.sel);
            }
            design.select(entry.sel);
        }
        tree.follow(&design);
    }

    /// Whether unsaved changes may be dropped: saved, or the person says so.
    fn may_discard(&mut self, app: &mut Application) -> bool {
        if !self.design.borrow().dirty {
            return true;
        }
        match confirmation_box(app, "The dialog has unsaved changes. Save them?") {
            CM_YES => self.save(app, false),
            command => command == turbo_vision::core::command::CM_NO,
        }
    }

    /// Save, asking for a file when there is none (or `ask` is set).
    fn save(&mut self, app: &mut Application, ask: bool) -> bool {
        let known = self.design.borrow().path.clone();
        let path = match known {
            Some(path) if !ask => path,
            _ => {
                let Some(path) = Self::ask_save_path(app, known.as_deref()) else {
                    return false;
                };
                path
            }
        };
        let saved = self.design.borrow().spec.save(&path);
        match saved {
            Ok(()) => {
                self.design.borrow_mut().saved(path);
                true
            }
            Err(e) => {
                message_box_error(app, &e.to_string());
                false
            }
        }
    }

    /// Ask where to save the JSON, in a file dialog that starts in the
    /// folder of the file it was last saved to. A name without an extension
    /// gets `.json`; another file already there is replaced only if the
    /// person says so.
    fn ask_save_path(app: &mut Application, known: Option<&Path>) -> Option<PathBuf> {
        let start = known
            .and_then(Path::parent)
            .filter(|p| p.is_dir())
            .map(Path::to_path_buf);
        let (w, h) = app.terminal.size();
        let bounds = centred(Rect::new(0, 0, w, h), 60.min(w), 20.min(h));
        let mut dialog = FileDialog::new(bounds, "Save dialog as", "*.json", start)
            .with_button_label("~S~ave")
            .build();
        let mut path = dialog.execute(app)?;
        if path.extension().is_none() {
            path.set_extension("json");
        }
        if path.exists() && known != Some(path.as_path()) {
            let question = format!("{} exists. Replace it?", path.display());
            if confirmation_box(app, &question) != CM_YES {
                return None;
            }
        }
        Some(path)
    }

    fn open(&mut self, app: &mut Application) {
        if !self.may_discard(app) {
            return;
        }
        let (w, h) = app.terminal.size();
        let bounds = centred(Rect::new(0, 0, w, h), 60.min(w), 20.min(h));
        let mut dialog = FileDialog::new(bounds, "Open dialog", "*.json", None).build();
        if let Some(path) = dialog.execute(app) {
            self.load(app, &path);
        }
    }

    /// Start a new dialog from a template chosen in the template manager.
    fn new_from_template(&mut self, app: &mut Application) {
        if !self.may_discard(app) {
            return;
        }
        if let Some(spec) = manage_templates(app, &templates::folder()) {
            *self.design.borrow_mut() = Design::new(spec, None);
        }
    }

    /// Keep the dialog as a template, under a name asked for.
    fn save_template(&mut self, app: &mut Application) {
        self.save_template_in(app, &templates::folder());
    }

    fn save_template_in(&mut self, app: &mut Application, dir: &Path) {
        let (spec, start) = {
            let d = self.design.borrow();
            let from_file = d
                .path
                .as_ref()
                .and_then(|p| p.file_stem())
                .map(|s| s.to_string_lossy().into_owned());
            (
                d.spec.clone(),
                from_file.unwrap_or_else(|| templates::file_stem(&d.spec.title)),
            )
        };
        let Some(name) = input_box(app, "Save as template", "Template ~n~ame", &start, 60) else {
            return;
        };
        if templates::exists(dir, &name) {
            let question = format!("A template called \"{}\" exists. Replace it?", name.trim());
            if confirmation_box(app, &question) != CM_YES {
                return;
            }
        }
        match templates::save(dir, &name, &spec) {
            Ok(path) => {
                message_box_ok(app, &format!("Template saved in {}", path.display()));
            }
            Err(e) => {
                message_box_error(app, &e);
            }
        }
    }

    /// Write the Rust that uses the dialog to a file chosen in a file
    /// dialog, which starts in the folder of the dialog's JSON.
    fn save_code(&mut self, app: &mut Application) {
        let (code, start) = {
            let d = self.design.borrow();
            let start = d
                .path
                .as_deref()
                .and_then(Path::parent)
                .filter(|p| p.is_dir())
                .map(Path::to_path_buf);
            (skeleton(&d.spec, d.path.as_deref()), start)
        };
        let (w, h) = app.terminal.size();
        let bounds = centred(Rect::new(0, 0, w, h), 60.min(w), 20.min(h));
        let mut dialog = FileDialog::new(bounds, "Save code as", "*.rs", start)
            .with_button_label("~S~ave")
            .build();
        let Some(mut path) = dialog.execute(app) else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension("rs");
        }
        if path.exists() {
            let question = format!("{} exists. Replace it?", path.display());
            if confirmation_box(app, &question) != turbo_vision::core::command::CM_YES {
                return;
            }
        }
        match std::fs::write(&path, code) {
            Ok(()) => {
                message_box_ok(app, &format!("Code written to {}", path.display()));
            }
            Err(e) => {
                message_box_error(app, &e.to_string());
            }
        }
    }

    fn load(&mut self, app: &mut Application, path: &Path) {
        match DialogSpec::load(path) {
            Ok(spec) => {
                *self.design.borrow_mut() = Design::new(spec, Some(path.to_path_buf()));
            }
            Err(e) => {
                message_box_error(app, &e.to_string());
            }
        }
    }

    /// Run the dialog as it will run in an application, then show what it
    /// answered.
    fn test(app: &mut Application, spec: &DialogSpec) {
        let mut dialog = match JsonDialog::new(spec) {
            Ok(dialog) => dialog,
            Err(e) => {
                message_box_error(app, &e.to_string());
                return;
            }
        };
        let unbound = dialog.unbound();
        let report = match dialog.run(app) {
            None => "Cancelled.".to_string(),
            Some(answer) => format!(
                "Closed by: {}\n\n{}",
                answer.action,
                serde_json::to_string_pretty(&answer.data).unwrap_or_default()
            ),
        };
        let note = if unbound.is_empty() {
            String::new()
        } else {
            format!(
                "\n\nTo bind in code ({}):\n  {}",
                "unbound actions close like OK",
                unbound.join("\n  ")
            )
        };
        show_text(app, "Test result", &format!("{report}{note}"));
    }

    /// The screen position just under the selection, for a popup menu.
    fn popup_point(&self, app: &Application) -> Point {
        let design = self.design.borrow();
        let r = design.rect_of(design.sel);
        let corner = app
            .desktop
            .get(self.surface)
            .map_or(Point::new(0, 0), |s| s.bounds().a);
        let desk = app.desktop.bounds().a;
        let y = if design.sel == Sel::Dialog {
            r.a.y
        } else {
            r.b.y
        };
        Point::new(desk.x + corner.x + 1 + r.a.x, desk.y + corner.y + 1 + y)
    }

    /// Ask where to move the selected control: another container.
    fn move_menu(&mut self, app: &mut Application) {
        let targets = self.design.borrow().move_targets();
        if targets.is_empty() {
            message_box_ok(app, "There is no other container to move it to.");
            return;
        }
        let items = targets
            .iter()
            .zip(0..)
            .map(|((_, label), i)| {
                let label: String = label.chars().filter(|&c| c != '~').collect();
                item(&label, CM_MOVE_TO + i, None)
            })
            .collect();
        let at = self.popup_point(app);
        if let Some(command) = popup_menu(&mut app.terminal, at, Menu::from_items(items))
            && let Some((container, _)) = targets.get(usize::from(command - CM_MOVE_TO))
        {
            self.design.borrow_mut().move_selected(*container);
        }
    }

    /// Where a popup menu of `rows` rows opens: under the selection, moved
    /// up as far as it takes to fit above the status line, so every item
    /// can be reached with the mouse.
    fn popup_origin(&self, app: &Application, rows: i16) -> Point {
        let mut at = self.popup_point(app);
        let (_, screen_h) = app.terminal.size();
        let top = app.desktop.bounds().a.y;
        at.y = at.y.min(screen_h - 1 - rows).max(top);
        at
    }

    /// Open the insert menu under the selection: every control type with a
    /// line between the sections, or, on a screen too short for that, the
    /// sections first.
    fn set_order(&mut self, app: &mut Application) {
        let group = self.design.borrow().order_group();
        let Some((rows, at)) = group else {
            message_box_ok(
                app,
                "Select a control first: its tab order is among its neighbours.",
            );
            return;
        };
        if rows.len() < 2 {
            message_box_ok(
                app,
                "It is the only control here: there is no order to set.",
            );
            return;
        }
        if let Some(order) = edit_order(app, &rows, at) {
            self.design.borrow_mut().set_order(&order);
        }
    }

    fn insert_menu(&mut self, app: &mut Application) {
        let rows = i16::try_from(KINDS.len() + SECTION_ENDS.len() + 2).unwrap_or(i16::MAX);
        let (_, screen_h) = app.terminal.size();
        if rows + 2 <= screen_h {
            let at = self.popup_origin(app, rows);
            if let Some(command) =
                popup_menu(&mut app.terminal, at, Menu::from_items(insert_items()))
            {
                self.insert(command);
            }
        } else {
            self.insert_section_menu(app, None);
        }
    }

    /// Pick the section of controls, then the control. With `section`
    /// given, only the controls of that section.
    fn insert_section_menu(&mut self, app: &mut Application, only: Option<usize>) {
        let sections = |n: usize| -> Vec<MenuItem> {
            section(n)
                .map(|i| {
                    let command = CM_INSERT + CommandId::try_from(i).unwrap_or(0);
                    item(KINDS[i].1, command, None)
                })
                .collect()
        };
        let n = if let Some(n) = only {
            n
        } else {
            let rows = i16::try_from(SECTION_NAMES.len() + 2).unwrap_or(i16::MAX);
            let at = self.popup_origin(app, rows);
            let names = SECTION_NAMES
                .iter()
                .zip(0..)
                .map(|(name, i)| item(name, CM_INSERT_SECTION + i, None))
                .collect();
            match popup_menu(&mut app.terminal, at, Menu::from_items(names)) {
                Some(c) => usize::from(c - CM_INSERT_SECTION),
                None => return,
            }
        };
        let items = sections(n);
        let rows = i16::try_from(items.len() + 2).unwrap_or(i16::MAX);
        let at = self.popup_origin(app, rows);
        if let Some(command) = popup_menu(&mut app.terminal, at, Menu::from_items(items)) {
            self.insert(command);
        }
    }

    fn insert(&mut self, command: CommandId) {
        if let Some((type_name, _)) = KINDS.get(usize::from(command - CM_INSERT)) {
            self.design.borrow_mut().insert(type_name);
        }
    }

    fn toggle_inspector(&mut self, app: &mut Application) {
        if let Some(inspector) = self.inspector.take() {
            app.desktop.remove_child_by_id(inspector.window.id());
        } else {
            self.inspector = Some(open_inspector(app));
            self.shown = u64::MAX;
        }
    }
}

impl AppHandler for Designer {
    fn pre_event(&mut self, _app: &mut Application, event: &mut Event) {
        // Exit asks about unsaved changes first.
        let quit = (event.what == EventType::Keyboard && event.key_code == KB_ALT_X)
            || (event.what == EventType::Command && event.command == CM_QUIT);
        if quit {
            *event = Event::command(CM_EXIT);
        }
    }

    fn handle_command(&mut self, app: &mut Application, command: CommandId, _: &Event) -> bool {
        let design = Rc::clone(&self.design);
        match command {
            CM_NEW => {
                if self.may_discard(app) {
                    *design.borrow_mut() = Design::default();
                }
            }
            CM_NEW_TEMPLATE => self.new_from_template(app),
            CM_SAVE_TEMPLATE => self.save_template(app),
            CM_OPEN_SPEC => self.open(app),
            CM_SAVE_SPEC => {
                self.save(app, false);
            }
            CM_SAVE_AS => {
                self.save(app, true);
            }
            CM_EXIT => {
                if self.may_discard(app) {
                    app.running = false;
                }
            }
            CM_UNDO => {
                design.borrow_mut().undo();
            }
            CM_REDO => {
                design.borrow_mut().redo();
            }
            CM_PROPERTIES => {
                self.follow_tree(app);
                props::edit_control(app, &design);
            }
            CM_DIALOG_PROPERTIES => {
                props::edit_dialog(app, &design);
            }
            CM_DELETE => {
                design.borrow_mut().delete();
            }
            CM_DUPLICATE => {
                design.borrow_mut().duplicate();
            }
            CM_MOVE_MENU => self.move_menu(app),
            CM_MOVE_DROPPED => {
                let dropped = self.tree_mut(app).and_then(ControlTree::take_dropped);
                if let Some((sel, to)) = dropped {
                    let mut d = design.borrow_mut();
                    d.select(sel);
                    d.move_selected(to);
                }
            }
            CM_SAME_WIDTH => {
                design.borrow_mut().same_size(true);
            }
            CM_SAME_HEIGHT => {
                design.borrow_mut().same_size(false);
            }
            CM_DISTRIBUTE_ACROSS => {
                design.borrow_mut().distribute(true);
            }
            CM_DISTRIBUTE_DOWN => {
                design.borrow_mut().distribute(false);
            }
            c if (CM_ALIGN..CM_ALIGN + 6).contains(&c) => {
                design
                    .borrow_mut()
                    .align(ALIGNMENTS[usize::from(c - CM_ALIGN)]);
            }
            CM_EARLIER => {
                design.borrow_mut().reorder(-1);
            }
            CM_LATER => {
                design.borrow_mut().reorder(1);
            }
            CM_SET_ORDER => self.set_order(app),
            CM_HELP_CONTENTS => help_contents(app),
            CM_TUTORIAL => tutorial(app),
            CM_HELP_ON => {
                let text = {
                    let d = design.borrow();
                    help::for_selection(d.selected().map(Control::type_name))
                };
                show_text(app, "Help", &text);
            }
            CM_TEST => {
                let spec = design.borrow().spec.clone();
                Self::test(app, &spec);
            }
            CM_SHOW_JSON => {
                let json = design.borrow().spec.to_json();
                show_text(app, "JSON", &json);
            }
            CM_CODE => self.save_code(app),
            CM_TAB_ORDER => {
                if let Some(surface) = self.surface_mut(app) {
                    surface.show_order = !surface.show_order;
                }
            }
            CM_INSPECTOR => self.toggle_inspector(app),
            CM_INSERT_MENU => self.insert_menu(app),
            CM_KEYS => show_text(app, "Keys", &keys::help(keys::current())),
            CM_ABOUT => {
                message_box_ok(
                    app,
                    "Dialog designer\n\nDesign Turbo Vision dialogs and save them as JSON for tv_extensions::dialog_spec.",
                );
            }
            c if (CM_INSERT..CM_INSERT + 21).contains(&c) => self.insert(c),
            c if (CM_INSERT_SECTION..CM_INSERT_SECTION + 6).contains(&c) => {
                self.insert_section_menu(app, Some(usize::from(c - CM_INSERT_SECTION)));
            }
            _ => return false,
        }
        self.refresh(app);
        true
    }

    fn idle(&mut self, app: &mut Application) {
        self.refresh(app);
    }
}

/// `w` x `h` centred in `area`.
fn centred(area: Rect, w: i16, h: i16) -> Rect {
    let x = area.a.x + (area.width() - w).max(0) / 2;
    let y = area.a.y + (area.height() - h).max(0) / 2;
    Rect::new(x, y, x + w, y + h)
}

/// What the inspector says about the selection.
fn inspector_text(design: &Design) -> String {
    let file = design
        .path
        .as_ref()
        .and_then(|p| p.file_name())
        .map_or_else(
            || "(not saved)".to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
    let dirty = if design.dirty { " *" } else { "" };
    let together = design.selection().len();
    let body = match design.selected() {
        Some(_) if together > 1 => format!(
            "{together} controls selected\n\nArrows or a drag: move them together\nArrange: align, size or space them\nDel: delete them\nA click: select one alone"
        ),
        None => {
            let s = &design.spec;
            format!(
                "Dialog \"{}\"\n{} x {}{}\n{} controls\n\nEnter: dialog properties\nIns: insert a control",
                s.title,
                s.width,
                s.height,
                if s.resizable { ", resizable" } else { "" },
                s.controls.len()
            )
        }
        Some(c) => {
            let json = serde_json::to_value(c).unwrap_or(Value::Null);
            let pretty = serde_json::to_string_pretty(&json).unwrap_or_default();
            format!("{}\n\n{pretty}", c.describe())
        }
    };
    format!("{file}{dirty}\n\n{body}")
}

fn open_inspector(app: &mut Application) -> Inspector {
    let desk = app.desktop.get_bounds();
    let (w, h) = (INSPECTOR_WIDTH, desk.height());
    let mut window = Window::new(Rect::new(desk.width() - w, 0, desk.width(), h), "Inspector");
    window.set_grow_mode(Grow::LO_X | Grow::HI_X | Grow::HI_Y);
    // The tree of controls takes the top half, the selection's JSON the rest.
    let split = ((h - 2) / 2).max(4);
    let mut tree = ControlTree::new(
        Rect::new(0, 0, w - 2, split),
        CM_PROPERTIES,
        CM_MOVE_DROPPED,
    );
    tree.set_grow_mode(Grow::HI_X);
    let tree = window.add_typed(tree);
    let mut viewer = TextViewer::new(Rect::new(0, split, w - 2, h - 2)).with_scrollbars(true);
    viewer.set_grow_mode(Grow::HI_X | Grow::HI_Y);
    let text = window.add_typed(viewer);
    let window = app.desktop.add_typed(window);
    Inspector { window, tree, text }
}

/// A modal box showing `text`, with OK.
fn show_text(app: &mut Application, title: &str, text: &str) {
    let (sw, sh) = app.terminal.size();
    let lines = i16::try_from(text.lines().count()).unwrap_or(i16::MAX);
    let widest = text.lines().map(|l| l.chars().count()).max().unwrap_or(0);
    let w = (i16::try_from(widest).unwrap_or(i16::MAX) + 6).clamp(30, sw - 2);
    let h = (lines + 6).clamp(8, sh - 2);
    let mut dialog = Dialog::new(centred(Rect::new(0, 0, sw, sh), w, h), title);
    let mut viewer = TextViewer::new(Rect::new(1, 0, w - 3, h - 5)).with_scrollbars(true);
    viewer.set_text(text);
    dialog.add(viewer);
    let bx = (w - 2 - 10) / 2;
    dialog.add(Button::new(
        Rect::new(bx, h - 4, bx + 10, h - 2),
        "~O~K",
        CM_OK,
        true,
    ));
    dialog.execute(app);
}

/// Rust that loads the dialog and binds what it names: a starting point to
/// paste into an application.
fn skeleton(spec: &DialogSpec, path: Option<&Path>) -> String {
    let file = path.and_then(|p| p.file_name()).map_or_else(
        || "dialog.json".to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let mut fields = Vec::new();
    let mut actions = Vec::new();
    // Each on_change action with the control it belongs to.
    let mut changes: Vec<(String, String)> = Vec::new();
    let mut validators = Vec::new();
    spec.visit(&mut |c| {
        if c.kind.has_value() && !c.name.is_empty() {
            fields.push(format!("{} ({})", c.name, c.type_name()));
        }
        let mut action = |a: &str| {
            if !actions.iter().any(|x: &String| x == a) {
                actions.push(a.to_string());
            }
        };
        match &c.kind {
            Kind::Button { command, .. }
                if !["ok", "cancel", "yes", "no"].contains(&command.as_str()) =>
            {
                action(command);
            }
            Kind::CheckBoxes {
                on_change: Some(a), ..
            }
            | Kind::RadioButtons {
                on_change: Some(a), ..
            }
            | Kind::ComboBox {
                on_change: Some(a), ..
            }
            | Kind::Spinner {
                on_change: Some(a), ..
            }
            | Kind::Slider {
                on_change: Some(a), ..
            } => {
                action(a);
                changes.push((a.clone(), c.name.clone()));
            }
            Kind::Input {
                validator: Some(tv_extensions::dialog_spec::ValidatorSpec::Custom { name }),
                ..
            } if !validators.contains(name) => validators.push(name.clone()),
            _ => {}
        }
    });
    let fname = spec
        .title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>();
    let fname = if fname.trim_matches('_').is_empty() {
        "dialog".to_string()
    } else {
        fname.trim_matches('_').to_string()
    };
    let mut out = String::new();
    out.push_str("use serde_json::Value;\n");
    out.push_str("use turbo_vision::app::Application;\n");
    out.push_str("use tv_extensions::dialog_spec::{Flow, JsonDialog};\n\n");
    let _ = writeln!(
        out,
        "/// Show \"{}\" and return its data, or None if cancelled.",
        spec.title
    );
    let _ = writeln!(
        out,
        "fn {fname}_dialog(app: &mut Application, start: &Value) -> Option<Value> {{"
    );
    let _ = writeln!(
        out,
        "    let mut dialog = JsonDialog::load(\"{file}\").expect(\"{file} is a valid dialog\");"
    );
    if !fields.is_empty() {
        let _ = writeln!(out, "    // Data keys: {}", fields.join(", "));
    }
    out.push_str("    dialog.set_data(start).expect(\"start matches the dialog\");\n");
    for v in &validators {
        let _ = writeln!(
            out,
            "    dialog.validator(\"{v}\", todo!(\"a ValidatorRef for {v}\"));"
        );
    }
    if let Some(first) = spec.find_first_input() {
        let _ = write!(
            out,
            "    // An example field rule: change it, or remove it.\n    dialog.rule(\"{first}\", |value| match value.as_str() {{\n        Some(\"\") => Err(\"{first} must not be empty\".into()),\n        _ => Ok(()),\n    }});\n"
        );
    }
    out.push_str("    dialog.validate_data(|data, errors| {\n        // Rules over several fields: errors.add(\"name\", \"message\");\n        let _ = (data, errors);\n    });\n");
    if let Some(first) = spec.find_first_field() {
        let _ = write!(
            out,
            "    // React to any named control as the person changes it:\n    // dialog.on_change(\"{first}\", |ctx| {{\n    //     let now = ctx.value(\"{first}\");\n    //     // ctx.set_value(\"other\", &serde_json::json!(...)) changes another control\n    //     Flow::Stay\n    // }});\n"
        );
    }
    for a in &actions {
        let hint = match changes.iter().find(|(action, _)| action == a) {
            Some((_, control)) if !control.is_empty() => format!(
                "        // Runs when \"{control}\" changes: _ctx.value(\"{control}\") is its new value.\n        // _ctx.set_value(\"other\", &serde_json::json!(...)) changes another control.\n"
            ),
            _ => "        // _ctx.value(\"name\"), set_value(\"name\", &serde_json::json!(...)), app\n".to_string(),
        };
        let _ = write!(out, "    dialog.on(\"{a}\", |_ctx| {{\n{hint}        Flow::Stay\n    }});\n");
    }
    out.push_str("    debug_assert!(dialog.unbound().is_empty(), \"{:?}\", dialog.unbound());\n");
    out.push_str("    let answer = dialog.run(app)?;\n");
    out.push_str("    // answer.action: \"ok\", or the action that closed the dialog\n");
    out.push_str("    Some(answer.data)\n}\n");
    out
}

/// The first input's name, to show a field rule on.
trait FirstInput {
    fn find_first_field(&self) -> Option<String>;
    fn find_first_input(&self) -> Option<String>;
}

impl FirstInput for DialogSpec {
    fn find_first_field(&self) -> Option<String> {
        let mut found = None;
        self.visit(&mut |c| {
            if found.is_none()
                && c.kind.has_value()
                && !matches!(c.kind, Kind::ProgressBar { .. })
                && !c.name.is_empty()
            {
                found = Some(c.name.clone());
            }
        });
        found
    }

    fn find_first_input(&self) -> Option<String> {
        let mut found = None;
        self.visit(&mut |c| {
            if found.is_none() && matches!(c.kind, Kind::Input { .. }) && !c.name.is_empty() {
                found = Some(c.name.clone());
            }
        });
        found
    }
}

/// The template manager: the templates in `dir` in a list, to use, rename or
/// delete. The spec of the one to use, if the person picks one.
fn manage_templates(app: &mut Application, dir: &Path) -> Option<DialogSpec> {
    const ROWS: i16 = 11;
    let mut at = 0;
    loop {
        let all = templates::list(dir);
        let (sw, sh) = app.terminal.size();
        let w: i16 = 66.min(sw);
        let h: i16 = (ROWS + 6).min(sh);
        let mut dialog = Dialog::new(centred(Rect::new(0, 0, sw, sh), w, h), "Templates");
        let header = format!(
            "{:<18} {:<22} {:<7} {}",
            "Name", "Dialog", "Size", "Controls"
        );
        dialog.add(StaticText::new(Rect::new(1, 0, w - 3, 1), &header));
        let mut list = ListBox::new(Rect::new(1, 1, w - 3, ROWS), CM_OK);
        list.set_items(all.iter().map(templates::Template::summary).collect());
        if !all.is_empty() {
            list.set_selection(at.min(all.len() - 1));
        }
        let list = dialog.add_typed(list);
        let folder = if all.is_empty() {
            format!("No templates in {}", dir.display())
        } else {
            format!("In {}", dir.display())
        };
        dialog.add(StaticText::new(
            Rect::new(1, ROWS + 1, w - 3, ROWS + 2),
            &folder,
        ));
        let buttons = [
            ("~U~se", CM_OK, 10, true),
            ("~R~ename", CM_YES, 12, false),
            ("~D~elete", CM_NO, 12, false),
            ("Close", CM_CANCEL, 10, false),
        ];
        let total: i16 = buttons.iter().map(|b| b.2 + 2).sum::<i16>() - 2;
        let mut x = ((w - 2 - total) / 2).max(0);
        for (text, command, width, default) in buttons {
            dialog.add(Button::new(
                Rect::new(x, ROWS + 2, x + width, ROWS + 4),
                text,
                command,
                default,
            ));
            x += width + 2;
        }
        dialog.set_initial_focus();
        let command = dialog.execute(app);
        let picked = dialog
            .get(list)
            .and_then(ListBox::get_selection)
            .and_then(|i| all.get(i).map(|t| (i, t.clone())));
        let Some((index, template)) = picked else {
            if command == CM_OK {
                message_box_ok(app, "There is no template to use.");
                continue;
            }
            return None;
        };
        at = index;
        match command {
            CM_OK => return Some(template.spec),
            CM_YES => {
                if let Some(name) = input_box(app, "Rename template", "~N~ame", &template.name, 60)
                    && let Err(e) = templates::rename(dir, &template.name, &name)
                {
                    message_box_error(app, &e);
                }
            }
            CM_NO => {
                let question = format!("Delete the template \"{}\"?", template.name);
                if confirmation_box(app, &question) == CM_YES
                    && let Err(e) = templates::delete(dir, &template.name)
                {
                    message_box_error(app, &e);
                }
            }
            _ => return None,
        }
    }
}

/// A dialog to put `rows` in tab order: the controls in a list, Move up and
/// Move down shift the one under the cursor. Returns the new order as the
/// old indices, `None` when cancelled.
fn edit_order(app: &mut Application, rows: &[String], selected: usize) -> Option<Vec<usize>> {
    let mut order: Vec<usize> = (0..rows.len()).collect();
    let mut at = selected.min(rows.len().saturating_sub(1));
    loop {
        let (sw, sh) = app.terminal.size();
        let list_h = i16::try_from(rows.len()).unwrap_or(i16::MAX).clamp(4, 12);
        let w: i16 = 50.min(sw);
        let h: i16 = (list_h + 7).min(sh);
        let mut dialog = Dialog::new(centred(Rect::new(0, 0, sw, sh), w, h), "Tab order");
        dialog.add(StaticText::new(
            Rect::new(1, 0, w - 3, 1),
            "Tab goes through the controls in this order:",
        ));
        let mut list = ListBox::new(Rect::new(1, 1, w - 3, 1 + list_h), CM_OK);
        list.set_items(
            order
                .iter()
                .enumerate()
                .map(|(n, &i)| format!("{:>2}  {}", n + 1, rows[i]))
                .collect(),
        );
        list.set_selection(at);
        let list = dialog.add_typed(list);
        let buttons = [
            ("Move ~u~p", CM_YES, 12, false),
            ("Move ~d~own", CM_NO, 14, false),
            ("~O~K", CM_OK, 8, true),
            ("Cancel", CM_CANCEL, 10, false),
        ];
        let total: i16 = buttons.iter().map(|b| b.2 + 2).sum::<i16>() - 2;
        let mut x = ((w - 2 - total) / 2).max(0);
        for (text, command, width, default) in buttons {
            dialog.add(Button::new(
                Rect::new(x, list_h + 2, x + width, list_h + 4),
                text,
                command,
                default,
            ));
            x += width + 2;
        }
        dialog.set_initial_focus();
        let command = dialog.execute(app);
        let cursor = dialog
            .get(list)
            .and_then(ListBox::get_selection)
            .unwrap_or(at);
        at = cursor.min(order.len() - 1);
        match command {
            CM_YES if at > 0 => {
                order.swap(at, at - 1);
                at -= 1;
            }
            CM_NO if at + 1 < order.len() => {
                order.swap(at, at + 1);
                at += 1;
            }
            CM_YES | CM_NO => {}
            CM_OK => return Some(order),
            _ => return None,
        }
    }
}

/// Help > Tutorial: the lessons one after another, Back and Next between.
fn tutorial(app: &mut Application) {
    let lessons = help::tutorial(keys::current());
    let mut at = 0;
    loop {
        let (sw, sh) = app.terminal.size();
        let widest = lessons
            .iter()
            .flat_map(|l| l.text.lines())
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(40);
        let tallest = lessons.iter().map(|l| l.text.lines().count()).max().unwrap_or(10);
        let w = (i16::try_from(widest).unwrap_or(i16::MAX) + 6).clamp(46, sw);
        let h = (i16::try_from(tallest).unwrap_or(i16::MAX) + 6).clamp(10, sh);
        let title = format!("Tutorial {}/{}: {}", at + 1, lessons.len(), lessons[at].title);
        let mut dialog = Dialog::new(centred(Rect::new(0, 0, sw, sh), w, h), &title);
        let mut viewer = TextViewer::new(Rect::new(1, 0, w - 3, h - 5)).with_scrollbars(true);
        viewer.set_text(&lessons[at].text);
        dialog.add(viewer);
        let last = at + 1 == lessons.len();
        let buttons = [
            ("~B~ack", CM_NO, 10, false),
            (if last { "~D~one" } else { "~N~ext" }, CM_YES, 10, true),
            ("Close", CM_CANCEL, 10, false),
        ];
        let total: i16 = buttons.iter().map(|b| b.2 + 2).sum::<i16>() - 2;
        let mut x = ((w - 2 - total) / 2).max(0);
        for (text, command, width, default) in buttons {
            dialog.add(Button::new(
                Rect::new(x, h - 4, x + width, h - 2),
                text,
                command,
                default,
            ));
            x += width + 2;
        }
        dialog.set_initial_focus();
        match dialog.execute(app) {
            CM_YES if last => return,
            CM_YES => at += 1,
            CM_NO => at = at.saturating_sub(1),
            _ => return,
        }
    }
}

/// Help > Contents: a list of topics, each shown in turn until Close.
fn help_contents(app: &mut Application) {
    let topics = help::topics(keys::current());
    let mut at = 0;
    loop {
        let (sw, sh) = app.terminal.size();
        let rows = i16::try_from(topics.len()).unwrap_or(i16::MAX).min(sh - 9).max(4);
        let w: i16 = 44.min(sw);
        let h: i16 = rows + 6;
        let mut dialog = Dialog::new(centred(Rect::new(0, 0, sw, sh), w, h), "Help contents");
        let mut list = ListBox::new(Rect::new(1, 0, w - 3, rows), CM_OK);
        list.set_items(topics.iter().map(|t| t.title.clone()).collect());
        list.set_selection(at);
        let list = dialog.add_typed(list);
        let buttons = [("~V~iew", CM_OK, 10, true), ("Close", CM_CANCEL, 10, false)];
        let total: i16 = buttons.iter().map(|b| b.2 + 2).sum::<i16>() - 2;
        let mut x = ((w - 2 - total) / 2).max(0);
        for (text, command, width, default) in buttons {
            dialog.add(Button::new(
                Rect::new(x, rows + 1, x + width, rows + 3),
                text,
                command,
                default,
            ));
            x += width + 2;
        }
        dialog.set_initial_focus();
        if dialog.execute(app) != CM_OK {
            return;
        }
        let Some(picked) = dialog.get(list).and_then(ListBox::get_selection) else {
            return;
        };
        at = picked.min(topics.len() - 1);
        show_text(app, &topics[at].title, &topics[at].text);
    }
}

/// The control types as menu items, a line between the sections.
fn insert_items() -> Vec<MenuItem> {
    let mut items = Vec::new();
    for (i, (_, text)) in KINDS.iter().enumerate() {
        if SECTION_ENDS.contains(&i) {
            items.push(MenuItem::separator());
        }
        let command = CM_INSERT + CommandId::try_from(i).unwrap_or(0);
        items.push(item(text, command, None));
    }
    items
}

fn item(text: &str, command: CommandId, key: Option<&str>) -> MenuItem {
    let b = MenuItemBuilder::new().text(text).command(command);
    match key {
        Some(k) => b.key(k).build(),
        None => b.build(),
    }
}

/// An item that shows a key the design surface handles itself.
fn item_shown(text: &str, command: CommandId, shortcut: &str) -> MenuItem {
    MenuItemBuilder::new()
        .text(text)
        .command(command)
        .shortcut(shortcut)
        .build()
}

fn menu_bar(width: i16) -> MenuBar {
    let k = keys::current();
    let mut bar = MenuBar::new(Rect::new(0, 0, width, 1));
    bar.add_submenu(SubMenu::new(
        "~F~ile",
        Menu::from_items(vec![
            item("~N~ew", CM_NEW, Some("Ctrl+N")),
            item("~O~pen...", CM_OPEN_SPEC, Some(k.open)),
            item("~S~ave", CM_SAVE_SPEC, Some(k.save)),
            item("Save ~a~s...", CM_SAVE_AS, None),
            MenuItem::separator(),
            item("New from ~t~emplate...", CM_NEW_TEMPLATE, None),
            item("Save as te~m~plate...", CM_SAVE_TEMPLATE, None),
            MenuItem::separator(),
            item("E~x~it", CM_EXIT, Some(k.exit)),
        ]),
    ));
    bar.add_submenu(SubMenu::new(
        "~E~dit",
        Menu::from_items(vec![
            item("~U~ndo", CM_UNDO, Some("Ctrl+Z")),
            item("~R~edo", CM_REDO, Some("Ctrl+Y")),
            MenuItem::separator(),
            item_shown("~P~roperties...", CM_PROPERTIES, "Enter"),
            item("~D~uplicate", CM_DUPLICATE, Some("Ctrl+D")),
            item_shown("De~l~ete", CM_DELETE, k.delete),
            MenuItem::separator(),
            item("~E~arlier in tab order", CM_EARLIER, Some(k.earlier)),
            item("La~t~er in tab order", CM_LATER, Some(k.later)),
            MenuItem::separator(),
            item("~M~ove to container...", CM_MOVE_MENU, Some(k.move_to)),
            item("~S~et tab order...", CM_SET_ORDER, Some(k.set_order)),
        ]),
    ));
    bar.add_submenu(SubMenu::new(
        "~A~rrange",
        Menu::from_items(vec![
            item("Align ~l~eft edges", CM_ALIGN, None),
            item("Align ~r~ight edges", CM_ALIGN + 1, None),
            item("Center ~h~orizontally", CM_ALIGN + 2, None),
            item("Align ~t~op edges", CM_ALIGN + 3, None),
            item("Align ~b~ottom edges", CM_ALIGN + 4, None),
            item("Center ~v~ertically", CM_ALIGN + 5, None),
            MenuItem::separator(),
            item("Same ~w~idth", CM_SAME_WIDTH, None),
            item("Same h~e~ight", CM_SAME_HEIGHT, None),
            MenuItem::separator(),
            item("Distribute ~a~cross", CM_DISTRIBUTE_ACROSS, None),
            item("Distribute d~o~wn", CM_DISTRIBUTE_DOWN, None),
        ]),
    ));
    // Short, so every entry can be clicked on any screen: the full list is
    // Ins, and each section opens its own small menu.
    let mut insert = vec![
        item("C~h~oose...", CM_INSERT_MENU, Some(k.insert)),
        MenuItem::separator(),
    ];
    insert.extend(
        SECTION_NAMES
            .iter()
            .zip(0..)
            .map(|(name, i)| item(&format!("{name}..."), CM_INSERT_SECTION + i, None)),
    );
    bar.add_submenu(SubMenu::new("~I~nsert", Menu::from_items(insert)));
    bar.add_submenu(SubMenu::new(
        "~D~ialog",
        Menu::from_items(vec![
            item("~P~roperties...", CM_DIALOG_PROPERTIES, None),
            item("~T~est", CM_TEST, Some(k.test)),
            MenuItem::separator(),
            item("Show ~J~SON", CM_SHOW_JSON, Some(k.json)),
            item("~C~ode to use it...", CM_CODE, Some("Ctrl+G")),
        ]),
    ));
    bar.add_submenu(SubMenu::new(
        "~V~iew",
        Menu::from_items(vec![
            item("~T~ab order", CM_TAB_ORDER, Some(k.order)),
            item("~I~nspector", CM_INSPECTOR, None),
        ]),
    ));
    bar.add_submenu(SubMenu::new(
        "~H~elp",
        Menu::from_items(vec![
            item("~T~utorial...", CM_TUTORIAL, None),
            item("~H~elp on selection", CM_HELP_ON, Some(k.help)),
            item("~C~ontents...", CM_HELP_CONTENTS, None),
            item("~K~eys", CM_KEYS, None),
            item("~A~bout", CM_ABOUT, None),
        ]),
    ));
    bar
}

fn status_line(width: i16, height: i16) -> StatusLine {
    let k = keys::current();
    let entry = |text: &str, key: &str, command: CommandId| {
        StatusItemBuilder::new()
            .text(format!("~{}~ {text}", keys::label(key)))
            .key(key)
            .command(command)
            .build()
    };
    StatusLine::new(
        Rect::new(0, height - 1, width, height),
        vec![
            entry("Exit", k.exit, CM_EXIT),
            entry("Save", k.save, CM_SAVE_SPEC),
            entry("Insert", k.insert, CM_INSERT_MENU),
            entry("Test", k.test, CM_TEST),
            entry("JSON", k.json, CM_SHOW_JSON),
            entry("Order", k.order, CM_TAB_ORDER),
        ],
    )
}

/// Put the design on the desktop, with the inspector when there is room.
fn open_designer(app: &mut Application, design: Design) -> Designer {
    let design = Rc::new(RefCell::new(design));
    let desk = app.desktop.get_bounds();
    let with_inspector = desk.width() >= 50 + INSPECTOR_WIDTH + 4;
    let area_w = if with_inspector {
        desk.width() - INSPECTOR_WIDTH
    } else {
        desk.width()
    };
    let (w, h) = {
        let d = design.borrow();
        (d.spec.width, d.spec.height)
    };
    let at = centred(Rect::new(0, 0, area_w, desk.height()), w, h).a;
    let surface = Surface::new(
        at,
        Rc::clone(&design),
        Commands {
            properties: CM_PROPERTIES,
            delete: CM_DELETE,
            insert_menu: CM_INSERT_MENU,
        },
    );
    let surface = app.desktop.add_typed(surface);
    let inspector = with_inspector.then(|| open_inspector(app));
    // The surface takes the keys.
    app.desktop.bring_to_front(surface.id());

    Designer {
        design,
        surface,
        inspector,
        shown: u64::MAX,
    }
}

fn main() -> turbo_vision::core::error::Result<()> {
    let mut app = Application::new()?;
    let (width, height) = app.terminal.size();
    app.set_menu_bar(menu_bar(width));
    app.set_status_line(status_line(width, height));

    let path = std::env::args().nth(1).map(PathBuf::from);
    let design = match &path {
        Some(p) if p.exists() => match DialogSpec::load(p) {
            Ok(spec) => Design::new(spec, Some(p.clone())),
            Err(e) => {
                drop(app);
                eprintln!("{e}");
                std::process::exit(1);
            }
        },
        Some(p) => Design::new(DialogSpec::new("New dialog", 50, 14), Some(p.clone())),
        None => Design::default(),
    };
    let mut designer = open_designer(&mut app, design);
    designer.refresh(&mut app);
    app.run_with(&mut designer);
    Ok(())
}

#[cfg(test)]
mod ui_tests;
