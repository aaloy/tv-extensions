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

mod model;
mod props;
mod surface;

use model::{Design, KINDS, Sel};
use serde_json::Value;
use std::cell::RefCell;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use surface::{Commands, Surface};
use turbo_vision::app::{AppHandler, Application};
use turbo_vision::core::command::{CM_OK, CM_QUIT, CM_USER, CM_YES, CommandId};
use turbo_vision::core::event::{Event, EventType, KB_ALT_X};
use turbo_vision::core::geometry::{Point, Rect};
use turbo_vision::core::menu_data::{Menu, MenuItem, MenuItemBuilder};
use turbo_vision::core::state::Grow;
use turbo_vision::core::status_data::StatusItemBuilder;
use turbo_vision::views::button::Button;
use turbo_vision::views::dialog::Dialog;
use turbo_vision::views::file_dialog::FileDialog;
use turbo_vision::views::menu_bar::{MenuBar, SubMenu};
use turbo_vision::views::msgbox::{confirmation_box, input_box, message_box_error, message_box_ok};
use turbo_vision::views::status_line::StatusLine;
use turbo_vision::views::text_viewer::TextViewer;
use turbo_vision::views::window::Window;
use turbo_vision::views::{GroupLike, Handle, View};
use tv_extensions::dialog_spec::{DialogSpec, JsonDialog, Kind};
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
/// `CM_INSERT + i` inserts `KINDS[i]`.
const CM_INSERT: CommandId = CM_USER + 50;

const INSPECTOR_WIDTH: i16 = 32;

const KEYS: &str = "\
Mouse
  Click            select a control
  Drag             move it
  Drag the yellow  resize it (or the dialog)
    corner
  Double click     properties
  Right click      insert a control

Keys
  Tab / Shift+Tab  next / previous control
  Esc              select the container
  Arrows           move one cell
  Shift/Ctrl+Arrow resize one cell
  Enter            properties
  Ins              insert a control
  Del              delete
  Ctrl+D           duplicate
  F7 / F8          earlier / later in tab order
  Ctrl+PgUp/PgDn   previous / next tab page
  Ctrl+O           show tab order
  Ctrl+Z / Ctrl+Y  undo / redo
  F9               test the dialog
  F4               show the JSON
  Ctrl+G           save the code to use it
  F2 / F3          save / open";

/// The designer: the design, the views that show it, and its commands.
struct Designer {
    design: Rc<RefCell<Design>>,
    surface: Handle<Surface>,
    inspector: Option<(Handle<Window>, Handle<TextViewer>)>,
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
        if let Some(surface) = self.surface_mut(app) {
            surface.sync_bounds();
        }
        let revision = self.design.borrow().revision;
        if revision != self.shown {
            self.shown = revision;
            let text = inspector_text(&self.design.borrow());
            if let Some((window, viewer)) = self.inspector
                && let Some(viewer) = app.desktop.get_mut(window).and_then(|w| w.get_mut(viewer))
            {
                viewer.set_text(&text);
            }
        }
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
                let start = known.as_ref().and_then(|p| p.file_name()).map_or_else(
                    || "dialog.json".to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
                let Some(name) = input_box(app, "Save as", "~F~ile name", &start, 200) else {
                    return false;
                };
                let mut path = PathBuf::from(name.trim());
                if path.extension().is_none() {
                    path.set_extension("json");
                }
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

    /// Open the insert menu under the selection.
    fn insert_menu(&mut self, app: &mut Application) {
        let at = {
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
        };
        let menu = Menu::from_items(
            KINDS
                .iter()
                .zip(0..)
                .map(|((_, text), i)| {
                    MenuItemBuilder::new()
                        .text(*text)
                        .command(CM_INSERT + i)
                        .build()
                })
                .collect(),
        );
        if let Some(command) = popup_menu(&mut app.terminal, at, menu) {
            self.insert(command);
        }
    }

    fn insert(&mut self, command: CommandId) {
        if let Some((type_name, _)) = KINDS.get(usize::from(command - CM_INSERT)) {
            self.design.borrow_mut().insert(type_name);
        }
    }

    fn toggle_inspector(&mut self, app: &mut Application) {
        if let Some((window, _)) = self.inspector.take() {
            app.desktop.remove_child_by_id(window.id());
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
            CM_EARLIER => {
                design.borrow_mut().reorder(-1);
            }
            CM_LATER => {
                design.borrow_mut().reorder(1);
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
            CM_KEYS => show_text(app, "Keys", KEYS),
            CM_ABOUT => {
                message_box_ok(
                    app,
                    "Dialog designer\n\nDesign Turbo Vision dialogs and save them as JSON for tv_extensions::dialog_spec.",
                );
            }
            c if (CM_INSERT..CM_INSERT + 20).contains(&c) => self.insert(c),
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
    let body = match design.selected() {
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

fn open_inspector(app: &mut Application) -> (Handle<Window>, Handle<TextViewer>) {
    let desk = app.desktop.get_bounds();
    let (w, h) = (INSPECTOR_WIDTH, desk.height());
    let mut window = Window::new(Rect::new(desk.width() - w, 0, desk.width(), h), "Inspector");
    window.set_grow_mode(Grow::LO_X | Grow::HI_X | Grow::HI_Y);
    let mut viewer = TextViewer::new(Rect::new(0, 0, w - 2, h - 2)).with_scrollbars(true);
    viewer.set_grow_mode(Grow::HI_X | Grow::HI_Y);
    let viewer = window.add_typed(viewer);
    let window = app.desktop.add_typed(window);
    (window, viewer)
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
            } => action(a),
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
    for a in &actions {
        let _ = write!(
            out,
            "    dialog.on(\"{a}\", |_ctx| {{\n        // _ctx.value(\"name\"), set_value(\"name\", &serde_json::json!(...)), app\n        Flow::Stay\n    }});\n"
        );
    }
    out.push_str("    debug_assert!(dialog.unbound().is_empty(), \"{:?}\", dialog.unbound());\n");
    out.push_str("    let answer = dialog.run(app)?;\n");
    out.push_str("    // answer.action: \"ok\", or the action that closed the dialog\n");
    out.push_str("    Some(answer.data)\n}\n");
    out
}

/// The first input's name, to show a field rule on.
trait FirstInput {
    fn find_first_input(&self) -> Option<String>;
}

impl FirstInput for DialogSpec {
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
    let mut bar = MenuBar::new(Rect::new(0, 0, width, 1));
    bar.add_submenu(SubMenu::new(
        "~F~ile",
        Menu::from_items(vec![
            item("~N~ew", CM_NEW, Some("Ctrl+N")),
            item("~O~pen...", CM_OPEN_SPEC, Some("F3")),
            item("~S~ave", CM_SAVE_SPEC, Some("F2")),
            item("Save ~a~s...", CM_SAVE_AS, None),
            MenuItem::separator(),
            item("E~x~it", CM_EXIT, Some("Alt+X")),
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
            item_shown("De~l~ete", CM_DELETE, "Del"),
            MenuItem::separator(),
            item("~E~arlier in tab order", CM_EARLIER, Some("F7")),
            item("La~t~er in tab order", CM_LATER, Some("F8")),
        ]),
    ));
    let mut insert: Vec<MenuItem> = KINDS
        .iter()
        .zip(0..)
        .map(|((_, text), i)| item(text, CM_INSERT + i, None))
        .collect();
    insert.insert(0, item("~C~hoose...", CM_INSERT_MENU, Some("Ins")));
    insert.insert(1, MenuItem::separator());
    bar.add_submenu(SubMenu::new("~I~nsert", Menu::from_items(insert)));
    bar.add_submenu(SubMenu::new(
        "~D~ialog",
        Menu::from_items(vec![
            item("~P~roperties...", CM_DIALOG_PROPERTIES, None),
            item("~T~est", CM_TEST, Some("F9")),
            MenuItem::separator(),
            item("Show ~J~SON", CM_SHOW_JSON, Some("F4")),
            item("~C~ode to use it...", CM_CODE, Some("Ctrl+G")),
        ]),
    ));
    bar.add_submenu(SubMenu::new(
        "~V~iew",
        Menu::from_items(vec![
            item("~T~ab order", CM_TAB_ORDER, Some("Ctrl+O")),
            item("~I~nspector", CM_INSPECTOR, None),
        ]),
    ));
    bar.add_submenu(SubMenu::new(
        "~H~elp",
        Menu::from_items(vec![
            item("~K~eys", CM_KEYS, None),
            item("~A~bout", CM_ABOUT, None),
        ]),
    ));
    bar
}

fn status_line(width: i16, height: i16) -> StatusLine {
    let entry = |text: &str, key: &str, command: CommandId| {
        StatusItemBuilder::new()
            .text(text)
            .key(key)
            .command(command)
            .build()
    };
    StatusLine::new(
        Rect::new(0, height - 1, width, height),
        vec![
            entry("~Alt-X~ Exit", "Alt+X", CM_EXIT),
            entry("~F2~ Save", "F2", CM_SAVE_SPEC),
            entry("~Ins~ Insert", "Ins", CM_INSERT_MENU),
            entry("~F9~ Test", "F9", CM_TEST),
            entry("~F4~ JSON", "F4", CM_SHOW_JSON),
            entry("~Ctrl-O~ Order", "Ctrl+O", CM_TAB_ORDER),
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
