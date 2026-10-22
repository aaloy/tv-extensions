// (C) 2026 - Antoni Aloy

//! The design being edited: the spec, what is selected, and every edit the
//! designer makes to them, with undo. No drawing and no terminal, so it is
//! tested on its own.

use std::path::PathBuf;
use turbo_vision::core::geometry::{Point, Rect};
use turbo_vision::views::tabbed_pane::TabbedPane;
use tv_extensions::dialog_spec::{
    Align, ColumnSpec, Control, DialogSpec, Item, Kind, PageSpec, SpecError,
};

/// The smallest dialog the designer makes, frame included.
pub const MIN_DIALOG: (i16, i16) = (16, 6);

/// How many edits undo remembers.
const UNDO_DEPTH: usize = 200;

/// What is selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sel {
    /// The dialog itself.
    Dialog,
    /// A control of the dialog, by index.
    Top(usize),
    /// A control on a tab page: the tabs' index, the page, the control.
    Page {
        tabs: usize,
        page: usize,
        index: usize,
    },
}

/// Every control type the designer inserts: the JSON type and its menu
/// text, in menu order.
pub const KINDS: [(&str, &str); 19] = [
    ("label", "~L~abel"),
    ("static_text", "~S~tatic text"),
    ("button", "~B~utton"),
    ("input", "~I~nput line"),
    ("memo", "~M~emo"),
    ("check_box", "~C~heck box"),
    ("check_boxes", "Check bo~x~es"),
    ("radio_buttons", "~R~adio buttons"),
    ("combo_box", "C~o~mbo box"),
    ("list_box", "Lis~t~ box"),
    ("spinner", "S~p~inner"),
    ("slider", "Sli~d~er"),
    ("progress_bar", "Pro~g~ress bar"),
    ("table", "T~a~ble"),
    ("group_box", "Gro~u~p box"),
    ("tabs", "Ta~b~s"),
    ("auto_complete", "Auto-co~m~plete"),
    ("dual_list", "Dual ~l~ist"),
    ("error_line", "~E~rror line"),
];

/// The design: a spec, a selection, the file it lives in, and undo.
#[derive(Debug, Clone)]
pub struct Design {
    pub spec: DialogSpec,
    pub sel: Sel,
    pub path: Option<PathBuf>,
    /// Changed since it was last saved or loaded.
    pub dirty: bool,
    /// Grows on every change to the spec or the selection, so views know
    /// when to refresh.
    pub revision: u64,
    undo: Vec<(DialogSpec, Sel)>,
    redo: Vec<(DialogSpec, Sel)>,
}

impl Default for Design {
    fn default() -> Self {
        Self::new(DialogSpec::new("New dialog", 50, 14), None)
    }
}

impl Design {
    /// A design of `spec`, saved at `path` if it has a file.
    pub fn new(spec: DialogSpec, path: Option<PathBuf>) -> Self {
        Self {
            spec,
            sel: Sel::Dialog,
            path,
            dirty: false,
            revision: 0,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    // ---- reading ----------------------------------------------------------

    /// The selected control, `None` when the dialog is selected.
    pub fn selected(&self) -> Option<&Control> {
        self.control(self.sel)
    }

    /// The control `sel` names.
    pub fn control(&self, sel: Sel) -> Option<&Control> {
        match sel {
            Sel::Dialog => None,
            Sel::Top(i) => self.spec.controls.get(i),
            Sel::Page { tabs, page, index } => pages(&self.spec.controls, tabs)?
                .get(page)?
                .controls
                .get(index),
        }
    }

    fn control_mut(&mut self, sel: Sel) -> Option<&mut Control> {
        match sel {
            Sel::Dialog => None,
            Sel::Top(i) => self.spec.controls.get_mut(i),
            Sel::Page { tabs, page, index } => pages_mut(&mut self.spec.controls, tabs)?
                .get_mut(page)?
                .controls
                .get_mut(index),
        }
    }

    /// The list of controls `sel` belongs to.
    fn siblings_mut(&mut self, sel: Sel) -> Option<&mut Vec<Control>> {
        match sel {
            Sel::Dialog => None,
            Sel::Top(_) => Some(&mut self.spec.controls),
            Sel::Page { tabs, page, .. } => Some(
                &mut pages_mut(&mut self.spec.controls, tabs)?
                    .get_mut(page)?
                    .controls,
            ),
        }
    }

    /// The size of the area `sel`'s control is placed in: the dialog's
    /// inside, or a tab page.
    pub fn container_size(&self, sel: Sel) -> (i16, i16) {
        match sel {
            Sel::Page { tabs, .. } => self.spec.controls.get(tabs).map_or((0, 0), |t| {
                let area = page_area(t);
                (area.width(), area.height())
            }),
            _ => self.spec.inner_size(),
        }
    }

    /// Where `sel`'s container starts, inside the dialog's frame.
    pub fn container_origin(&self, sel: Sel) -> Point {
        match sel {
            Sel::Page { tabs, .. } => self.spec.controls.get(tabs).map_or(Point::new(0, 0), |t| {
                let area = page_area(t);
                Point::new(t.x + area.a.x, t.y + area.a.y)
            }),
            _ => Point::new(0, 0),
        }
    }

    /// `sel`'s place inside the dialog's frame; the whole inside for the
    /// dialog.
    pub fn rect_of(&self, sel: Sel) -> Rect {
        let Some(c) = self.control(sel) else {
            let (w, h) = self.spec.inner_size();
            return Rect::new(0, 0, w, h);
        };
        let o = self.container_origin(sel);
        Rect::new(
            o.x + c.x,
            o.y + c.y,
            o.x + c.x + c.w,
            o.y + c.y + c.h.max(1),
        )
    }

    /// The front-most control at `p` (inside the frame), or the dialog.
    /// Controls on the shown page of a tab pane come before the pane.
    pub fn hit(&self, p: Point) -> Sel {
        for (i, c) in self.spec.controls.iter().enumerate().rev() {
            if !self.rect_of(Sel::Top(i)).contains(p) {
                continue;
            }
            if let Kind::Tabs { pages, active } = &c.kind
                && let Some(page) = pages.get(*active)
            {
                for (index, _) in page.controls.iter().enumerate().rev() {
                    let sel = Sel::Page {
                        tabs: i,
                        page: *active,
                        index,
                    };
                    if self.rect_of(sel).contains(p) {
                        return sel;
                    }
                }
            }
            return Sel::Top(i);
        }
        Sel::Dialog
    }

    /// Every selectable thing in tab order: the dialog, then each control,
    /// a tab pane followed by the controls of its shown page.
    pub fn order(&self) -> Vec<Sel> {
        let mut order = vec![Sel::Dialog];
        for (i, c) in self.spec.controls.iter().enumerate() {
            order.push(Sel::Top(i));
            if let Kind::Tabs { pages, active } = &c.kind
                && let Some(page) = pages.get(*active)
            {
                order.extend((0..page.controls.len()).map(|index| Sel::Page {
                    tabs: i,
                    page: *active,
                    index,
                }));
            }
        }
        order
    }

    /// The tab pane `sel` is, or is on, with its index.
    pub fn tabs_of(&self, sel: Sel) -> Option<usize> {
        match sel {
            Sel::Top(i) if matches!(self.spec.controls.get(i)?.kind, Kind::Tabs { .. }) => Some(i),
            Sel::Page { tabs, .. } => Some(tabs),
            _ => None,
        }
    }

    /// Every name in use, on tab pages too.
    pub fn names(&self) -> Vec<String> {
        let mut names = Vec::new();
        self.spec.visit(&mut |c| {
            if !c.name.is_empty() {
                names.push(c.name.clone());
            }
        });
        names
    }

    /// `prefix` with the lowest number that makes it a name not in use.
    pub fn unique_name(&self, prefix: &str) -> String {
        let names = self.names();
        (1..)
            .map(|n| format!("{prefix}{n}"))
            .find(|name| !names.contains(name))
            .expect("some number is free")
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    // ---- selecting ----------------------------------------------------------

    pub fn select(&mut self, sel: Sel) {
        if self.sel != sel {
            self.sel = sel;
            self.revision += 1;
        }
    }

    /// Select the next (or previous) thing in tab order, wrapping.
    pub fn select_next(&mut self, forward: bool) {
        let order = self.order();
        let at = order.iter().position(|s| *s == self.sel).unwrap_or(0);
        let len = order.len();
        let next = if forward {
            (at + 1) % len
        } else {
            (at + len - 1) % len
        };
        self.select(order[next]);
    }

    /// Select what holds the selection: a page's tab pane, or the dialog.
    pub fn select_parent(&mut self) {
        let parent = match self.sel {
            Sel::Page { tabs, .. } => Sel::Top(tabs),
            _ => Sel::Dialog,
        };
        self.select(parent);
    }

    // ---- editing --------------------------------------------------------------

    /// Remember the design before an edit, for undo.
    pub fn snapshot(&mut self) {
        self.undo.push((self.spec.clone(), self.sel));
        if self.undo.len() > UNDO_DEPTH {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn changed(&mut self) {
        self.dirty = true;
        self.revision += 1;
    }

    pub fn undo(&mut self) -> bool {
        let Some((spec, sel)) = self.undo.pop() else {
            return false;
        };
        self.redo
            .push((std::mem::replace(&mut self.spec, spec), self.sel));
        self.sel = sel;
        self.changed();
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some((spec, sel)) = self.redo.pop() else {
            return false;
        };
        self.undo
            .push((std::mem::replace(&mut self.spec, spec), self.sel));
        self.sel = sel;
        self.changed();
        true
    }

    /// Insert a control of the JSON type `type_name`, select it, and return
    /// where it went.
    ///
    /// It goes on the shown page when a tab pane (or a control on one) is
    /// selected, else in the dialog; after the selected control in tab
    /// order; below it, or, after a label with no link, to the label's
    /// right and linked to it.
    pub fn insert(&mut self, type_name: &str) -> Sel {
        let on_page = (type_name != "tabs")
            .then(|| self.tabs_of(self.sel))
            .flatten()
            .and_then(|tabs| match &self.spec.controls[tabs].kind {
                Kind::Tabs { pages, active } if !pages.is_empty() => Some((tabs, *active)),
                _ => None,
            });
        // Where the new control goes in its list.
        let (into, at) = match (on_page, self.sel) {
            (Some((tabs, page)), Sel::Page { index, .. }) => (
                Sel::Page {
                    tabs,
                    page,
                    index: 0,
                },
                index + 1,
            ),
            (Some((tabs, page)), _) => {
                let len = pages(&self.spec.controls, tabs).map_or(0, |p| p[page].controls.len());
                (
                    Sel::Page {
                        tabs,
                        page,
                        index: 0,
                    },
                    len,
                )
            }
            (None, Sel::Top(i)) => (Sel::Top(0), i + 1),
            (None, Sel::Page { tabs, .. }) => (Sel::Top(0), tabs + 1),
            (None, Sel::Dialog) => (Sel::Top(0), self.spec.controls.len()),
        };
        let (cw, ch) = self.container_size(into);
        let mut control = self.default_control(type_name, cw);

        // Place it: right of an unlinked label, below the selection, or at
        // the top left.
        let anchor = if self.sel_in(into) {
            self.selected().cloned()
        } else {
            None
        };
        let mut link_label = false;
        match &anchor {
            Some(Control {
                kind: Kind::Label { link: None, .. },
                x,
                y,
                w,
                ..
            }) if control.kind.has_value() => {
                control.x = x + w + 1;
                control.y = *y;
                link_label = true;
            }
            Some(a) => {
                control.x = a.x;
                control.y = a.y + a.h.max(1) + usize_gap(type_name);
            }
            None => {
                control.x = 2.min(cw - 1).max(0);
                control.y = 1.min(ch - 1).max(0);
            }
        }
        fit(&mut control, cw, ch);

        self.snapshot();
        if link_label && let Some(label) = self.control_mut(self.sel) {
            if let Kind::Label { link, .. } = &mut label.kind {
                *link = Some(control.name.clone());
            }
        }
        let list = self.siblings_mut(into).expect("the container exists");
        let at = at.min(list.len());
        list.insert(at, control);
        let sel = match into {
            Sel::Page { tabs, page, .. } => Sel::Page {
                tabs,
                page,
                index: at,
            },
            _ => Sel::Top(at),
        };
        self.sel = sel;
        self.changed();
        sel
    }

    /// Whether the selection is a control in the same list as `into`.
    fn sel_in(&self, into: Sel) -> bool {
        match (self.sel, into) {
            (Sel::Top(_), Sel::Top(_)) => true,
            (
                Sel::Page { tabs, page, .. },
                Sel::Page {
                    tabs: t, page: p, ..
                },
            ) => tabs == t && page == p,
            _ => false,
        }
    }

    /// A new control of `type_name` with its usual size and settings, for a
    /// container `width` wide.
    pub fn default_control(&self, type_name: &str, width: i16) -> Control {
        let named = |prefix: &str| self.unique_name(prefix);
        let strings = |items: &[&str]| items.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
        let (name, size, kind) = match type_name {
            "label" => (
                String::new(),
                (10, 1),
                Kind::Label {
                    text: "~L~abel".into(),
                    link: None,
                },
            ),
            "static_text" => (
                String::new(),
                (20, 1),
                Kind::StaticText {
                    text: "Text".into(),
                    centered: false,
                },
            ),
            "button" => {
                let (text, command, default) = self.next_button();
                (
                    String::new(),
                    (12, 2),
                    Kind::Button {
                        text,
                        command,
                        default,
                    },
                )
            }
            "input" => (
                named("input"),
                (20, 1),
                Kind::Input {
                    text: String::new(),
                    max_len: 255,
                    required: false,
                    validator: None,
                    history: None,
                },
            ),
            "memo" => (
                named("memo"),
                (30, 4),
                Kind::Memo {
                    text: String::new(),
                    max_len: None,
                    required: false,
                    read_only: false,
                },
            ),
            "check_box" => (
                named("check"),
                (20, 1),
                Kind::CheckBox {
                    text: "~C~heck box".into(),
                    checked: false,
                    required: false,
                },
            ),
            "check_boxes" => (
                named("checks"),
                (20, 3),
                Kind::CheckBoxes {
                    items: strings(&["~O~ne", "~T~wo", "T~h~ree"]),
                    checked: Vec::new(),
                    on_change: None,
                },
            ),
            "radio_buttons" => (
                named("radio"),
                (20, 3),
                Kind::RadioButtons {
                    items: strings(&["~O~ne", "~T~wo", "T~h~ree"]),
                    selected: 0,
                    on_change: None,
                },
            ),
            "combo_box" => (
                named("combo"),
                (20, 1),
                Kind::ComboBox {
                    items: strings(&["One", "Two", "Three"]),
                    selected: None,
                    required: false,
                    on_change: None,
                },
            ),
            "list_box" => (
                named("list"),
                (20, 5),
                Kind::ListBox {
                    items: strings(&["One", "Two", "Three"]),
                    selected: None,
                    multi: false,
                },
            ),
            "spinner" => (
                named("spinner"),
                (10, 1),
                Kind::Spinner {
                    min: 0,
                    max: 100,
                    value: 0,
                    step: 1,
                    suffix: String::new(),
                    wrap: false,
                    on_change: None,
                },
            ),
            "slider" => (
                named("slider"),
                (20, 1),
                Kind::Slider {
                    min: 0,
                    max: 100,
                    value: 0,
                    step: 1,
                    on_change: None,
                },
            ),
            "progress_bar" => (
                named("progress"),
                (20, 1),
                Kind::ProgressBar {
                    max: 100,
                    value: 0,
                    show_percent: true,
                },
            ),
            "table" => (
                named("table"),
                (30, 6),
                Kind::Table {
                    columns: vec![
                        ColumnSpec {
                            title: "Name".into(),
                            width: 15,
                            align: Align::Left,
                        },
                        ColumnSpec {
                            title: "Value".into(),
                            width: 10,
                            align: Align::Right,
                        },
                    ],
                    rows: Vec::new(),
                    header: true,
                    separators: false,
                },
            ),
            "group_box" => (
                String::new(),
                (30, 6),
                Kind::GroupBox {
                    title: "Group".into(),
                },
            ),
            "tabs" => (
                named("tabs"),
                (40, 10),
                Kind::Tabs {
                    pages: vec![
                        PageSpec {
                            title: "~F~irst".into(),
                            controls: Vec::new(),
                        },
                        PageSpec {
                            title: "~S~econd".into(),
                            controls: Vec::new(),
                        },
                    ],
                    active: 0,
                },
            ),
            "auto_complete" => (
                named("auto"),
                (20, 1),
                Kind::AutoComplete {
                    items: strings(&["Apple", "Banana", "Cherry"]),
                    text: String::new(),
                    max_len: 255,
                    min_chars: 1,
                    required: false,
                    require_match: false,
                },
            ),
            "dual_list" => (
                named("dual"),
                (44, 8),
                Kind::DualList {
                    items: ["One", "Two", "Three", "Four"]
                        .iter()
                        .map(|s| Item::Plain((*s).to_string()))
                        .collect(),
                    chosen: Vec::new(),
                    titles: None,
                    min: 0,
                    max: None,
                },
            ),
            "error_line" => (String::new(), ((width - 4).max(10), 1), Kind::ErrorLine {}),
            other => panic!("no control type {other}"),
        };
        Control::new(&name, (0, 0, size.0, size.1), kind)
    }

    /// A new button's caption, command and default flag: OK first, then
    /// Cancel, then an action named after the button.
    fn next_button(&self) -> (String, String, bool) {
        let mut commands = Vec::new();
        self.spec.visit(&mut |c| {
            if let Kind::Button { command, .. } = &c.kind {
                commands.push(command.clone());
            }
        });
        if !commands.iter().any(|c| c == "ok") {
            ("~O~K".into(), "ok".into(), true)
        } else if !commands.iter().any(|c| c == "cancel") {
            ("Cancel".into(), "cancel".into(), false)
        } else {
            let action = (1..)
                .map(|n| format!("action{n}"))
                .find(|a| !commands.contains(a))
                .expect("some number is free");
            ("~A~ction".into(), action, false)
        }
    }

    /// Remove the selected control (a tab pane with its pages), and any
    /// label link to it. Selects what came before it.
    pub fn delete(&mut self) -> bool {
        let sel = self.sel;
        let Some(removed) = self.control(sel).cloned() else {
            return false;
        };
        self.snapshot();
        let list = self.siblings_mut(sel).expect("a control has siblings");
        let index = match sel {
            Sel::Top(i) | Sel::Page { index: i, .. } => i,
            Sel::Dialog => unreachable!(),
        };
        list.remove(index);
        // Names that went away: the control's, and its pages' controls'.
        let mut gone = vec![removed.name.clone()];
        if let Kind::Tabs { pages, .. } = &removed.kind {
            gone.extend(
                pages
                    .iter()
                    .flat_map(|p| p.controls.iter().map(|c| c.name.clone())),
            );
        }
        unlink(&mut self.spec.controls, &gone);
        self.sel = match sel {
            Sel::Top(0) | Sel::Dialog => Sel::Dialog,
            Sel::Top(i) => Sel::Top(i - 1),
            Sel::Page { tabs, index: 0, .. } => Sel::Top(tabs),
            Sel::Page { tabs, page, index } => Sel::Page {
                tabs,
                page,
                index: index - 1,
            },
        };
        self.changed();
        true
    }

    /// Copy the selected control just below itself, with fresh names.
    pub fn duplicate(&mut self) -> bool {
        let sel = self.sel;
        let Some(mut copy) = self.control(sel).cloned() else {
            return false;
        };
        let (cw, ch) = self.container_size(sel);
        self.rename_copy(&mut copy);
        copy.y += copy.h.max(1);
        fit(&mut copy, cw, ch);
        self.snapshot();
        let list = self.siblings_mut(sel).expect("a control has siblings");
        let index = match sel {
            Sel::Top(i) | Sel::Page { index: i, .. } => i + 1,
            Sel::Dialog => unreachable!(),
        };
        list.insert(index, copy);
        self.sel = match sel {
            Sel::Page { tabs, page, .. } => Sel::Page { tabs, page, index },
            _ => Sel::Top(index),
        };
        self.changed();
        true
    }

    /// Give a copied control, and its pages' controls, names not in use,
    /// keeping links among the copies.
    fn rename_copy(&self, copy: &mut Control) {
        let mut taken = self.names();
        let mut fresh = |name: &str| -> String {
            if name.is_empty() {
                return String::new();
            }
            let stem = name.trim_end_matches(|c: char| c.is_ascii_digit());
            let new = (1..)
                .map(|n| format!("{stem}{n}"))
                .find(|n| !taken.contains(n))
                .expect("some number is free");
            taken.push(new.clone());
            new
        };
        copy.name = fresh(&copy.name);
        if let Kind::Tabs { pages, .. } = &mut copy.kind {
            let mut renames = Vec::new();
            for c in pages.iter_mut().flat_map(|p| p.controls.iter_mut()) {
                let new = fresh(&c.name);
                renames.push((c.name.clone(), new.clone()));
                c.name = new;
            }
            for c in pages.iter_mut().flat_map(|p| p.controls.iter_mut()) {
                if let Kind::Label {
                    link: Some(link), ..
                } = &mut c.kind
                    && let Some((_, new)) = renames.iter().find(|(old, _)| old == link)
                {
                    *link = new.clone();
                }
            }
        } else if let Kind::Label { link, .. } = &mut copy.kind {
            // A copied label captions nothing until it is linked again.
            *link = None;
        }
    }

    /// Move the selected control by `(dx, dy)`, kept inside its container.
    /// The dialog does not move: it is placed when it runs.
    pub fn move_by(&mut self, dx: i16, dy: i16) -> bool {
        let Some(c) = self.selected() else {
            return false;
        };
        let (x, y, w, h) = (c.x + dx, c.y + dy, c.w, c.h);
        self.set_place(self.sel, x, y, w, h)
    }

    /// Grow or shrink the selection by `(dw, dh)`: a control within its
    /// container, or the dialog.
    pub fn resize_by(&mut self, dw: i16, dh: i16) -> bool {
        match self.selected() {
            Some(c) => {
                let (x, y, w, h) = (c.x, c.y, c.w + dw, c.h + dh);
                self.set_place(self.sel, x, y, w, h)
            }
            None => {
                let (w, h) = (self.spec.width + dw, self.spec.height + dh);
                self.set_dialog_size(w, h)
            }
        }
    }

    /// Put `sel` at `(x, y)` with size `(w, h)`, kept inside its container
    /// and at least one cell. Records undo only for a real change.
    pub fn set_place(&mut self, sel: Sel, x: i16, y: i16, w: i16, h: i16) -> bool {
        let (cw, ch) = self.container_size(sel);
        let Some(c) = self.control(sel) else {
            return false;
        };
        let mut moved = c.clone();
        moved.w = w.clamp(1, cw.max(1));
        moved.h = h.clamp(1, ch.max(1));
        moved.x = x.clamp(0, (cw - moved.w).max(0));
        moved.y = y.clamp(0, (ch - moved.h).max(0));
        if (moved.x, moved.y, moved.w, moved.h) == (c.x, c.y, c.w, c.h) {
            return false;
        }
        self.snapshot();
        *self.control_mut(sel).expect("checked above") = moved;
        self.changed();
        true
    }

    /// Resize the dialog, no smaller than [`MIN_DIALOG`].
    pub fn set_dialog_size(&mut self, w: i16, h: i16) -> bool {
        let (w, h) = (w.max(MIN_DIALOG.0), h.max(MIN_DIALOG.1));
        if (w, h) == (self.spec.width, self.spec.height) {
            return false;
        }
        self.snapshot();
        self.spec.width = w;
        self.spec.height = h;
        self.changed();
        true
    }

    /// Move the selected control one place earlier (`-1`) or later (`1`)
    /// in tab order, among its siblings.
    pub fn reorder(&mut self, delta: isize) -> bool {
        let sel = self.sel;
        let index = match sel {
            Sel::Top(i) | Sel::Page { index: i, .. } => i,
            Sel::Dialog => return false,
        };
        let len = self.siblings_mut(sel).map_or(0, |l| l.len());
        let Some(to) = index.checked_add_signed(delta).filter(|&to| to < len) else {
            return false;
        };
        self.snapshot();
        self.siblings_mut(sel).expect("checked").swap(index, to);
        self.sel = match sel {
            Sel::Page { tabs, page, .. } => Sel::Page {
                tabs,
                page,
                index: to,
            },
            _ => Sel::Top(to),
        };
        self.changed();
        true
    }

    /// Show page `page` of the tab pane at `tabs`; it is also the page the
    /// dialog opens on. Not an undoable edit.
    pub fn show_page(&mut self, tabs: usize, page: usize) -> bool {
        let Some(Control {
            kind: Kind::Tabs { pages, active },
            ..
        }) = self.spec.controls.get_mut(tabs)
        else {
            return false;
        };
        if page >= pages.len() || *active == page {
            return false;
        }
        *active = page;
        if let Sel::Page { tabs: t, .. } = self.sel
            && t == tabs
        {
            self.sel = Sel::Top(tabs);
        }
        self.changed();
        true
    }

    /// Replace the selected control with `control`, as edited in its
    /// properties. A new name is followed by the labels linked to the old
    /// one. Refused, leaving the design as it was, if the result breaks a
    /// spec rule.
    ///
    /// # Errors
    ///
    /// The rule broken.
    pub fn replace_selected(&mut self, control: Control) -> Result<(), SpecError> {
        let sel = self.sel;
        let Some(old) = self.control(sel).cloned() else {
            return Err(SpecError("nothing is selected".into()));
        };
        let mut swap = Design::new(self.spec.clone(), None);
        swap.sel = sel;
        *swap.control_mut(sel).expect("same shape") = control.clone();
        if old.name != control.name && !old.name.is_empty() {
            relink(&mut swap.spec.controls, &old.name, &control.name);
        }
        swap.spec.check()?;
        self.snapshot();
        self.spec = swap.spec;
        self.changed();
        Ok(())
    }

    /// Change the dialog's own settings.
    pub fn set_dialog(
        &mut self,
        title: &str,
        w: i16,
        h: i16,
        resizable: bool,
        at: Option<(i16, i16)>,
    ) {
        self.snapshot();
        self.spec.title = title.to_string();
        self.spec.width = w.max(MIN_DIALOG.0);
        self.spec.height = h.max(MIN_DIALOG.1);
        self.spec.resizable = resizable;
        self.spec.x = at.map(|p| p.0);
        self.spec.y = at.map(|p| p.1);
        self.changed();
    }

    /// The design was saved at `path`.
    pub fn saved(&mut self, path: PathBuf) {
        self.path = Some(path);
        self.dirty = false;
        self.revision += 1;
    }
}

/// Blank rows left between a new control and the one above it.
fn usize_gap(type_name: &str) -> i16 {
    i16::from(!matches!(
        type_name,
        "label" | "input" | "check_box" | "static_text"
    ))
}

/// Shrink and move `c` to fit a `cw` x `ch` container.
fn fit(c: &mut Control, cw: i16, ch: i16) {
    c.w = c.w.clamp(1, cw.max(1));
    c.h = c.h.clamp(1, ch.max(1));
    c.x = c.x.clamp(0, (cw - c.w).max(0));
    c.y = c.y.clamp(0, (ch - c.h).max(0));
}

/// A tab pane's page area, in the pane's space.
pub fn page_area(tabs: &Control) -> Rect {
    TabbedPane::new(Rect::new(0, 0, tabs.w, tabs.h)).page_area()
}

fn pages(controls: &[Control], tabs: usize) -> Option<&Vec<PageSpec>> {
    match &controls.get(tabs)?.kind {
        Kind::Tabs { pages, .. } => Some(pages),
        _ => None,
    }
}

fn pages_mut(controls: &mut [Control], tabs: usize) -> Option<&mut Vec<PageSpec>> {
    match &mut controls.get_mut(tabs)?.kind {
        Kind::Tabs { pages, .. } => Some(pages),
        _ => None,
    }
}

fn for_each_mut(controls: &mut [Control], f: &mut impl FnMut(&mut Control)) {
    for c in controls {
        f(c);
        if let Kind::Tabs { pages, .. } = &mut c.kind {
            for p in pages {
                for_each_mut(&mut p.controls, f);
            }
        }
    }
}

/// Drop every label link to one of `names`.
fn unlink(controls: &mut [Control], names: &[String]) {
    for_each_mut(controls, &mut |c| {
        if let Kind::Label { link, .. } = &mut c.kind
            && link.as_ref().is_some_and(|l| names.contains(l))
        {
            *link = None;
        }
    });
}

/// Point every label link to `old` at `new` (or drop it if `new` is empty).
fn relink(controls: &mut [Control], old: &str, new: &str) {
    for_each_mut(controls, &mut |c| {
        if let Kind::Label { link, .. } = &mut c.kind
            && link.as_deref() == Some(old)
        {
            *link = (!new.is_empty()).then(|| new.to_string());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn design() -> Design {
        Design::default()
    }

    #[test]
    fn inserting_names_places_and_selects() {
        let mut d = design();
        let sel = d.insert("input");
        assert_eq!(sel, Sel::Top(0));
        let c = d.selected().unwrap();
        assert_eq!(c.name, "input1");
        assert_eq!((c.x, c.y), (2, 1));
        // The next one goes below it, after it in tab order.
        d.insert("input");
        let c = d.selected().unwrap();
        assert_eq!(c.name, "input2");
        assert_eq!((c.x, c.y), (2, 2));
        assert!(d.dirty);
    }

    #[test]
    fn a_field_inserted_after_a_label_sits_right_of_it_linked() {
        let mut d = design();
        d.insert("label");
        d.insert("input");
        let label = d.control(Sel::Top(0)).unwrap();
        let input = d.control(Sel::Top(1)).unwrap();
        assert_eq!(input.y, label.y);
        assert_eq!(input.x, label.x + label.w + 1);
        assert_eq!(
            label.kind,
            Kind::Label {
                text: "~L~abel".into(),
                link: Some("input1".into())
            }
        );
        assert!(d.spec.check().is_ok());
    }

    #[test]
    fn buttons_come_as_ok_then_cancel_then_actions() {
        let mut d = design();
        let commands: Vec<String> = (0..3)
            .map(|_| {
                d.insert("button");
                match &d.selected().unwrap().kind {
                    Kind::Button { command, .. } => command.clone(),
                    _ => unreachable!(),
                }
            })
            .collect();
        assert_eq!(commands, ["ok", "cancel", "action1"]);
    }

    #[test]
    fn controls_go_on_the_shown_page_of_selected_tabs() {
        let mut d = design();
        d.insert("tabs");
        let sel = d.insert("check_box");
        assert_eq!(
            sel,
            Sel::Page {
                tabs: 0,
                page: 0,
                index: 0
            }
        );
        // Another tab pane is never put on a page.
        assert_eq!(d.insert("tabs"), Sel::Top(1));
        d.select(Sel::Top(0));
        d.show_page(0, 1);
        assert_eq!(
            d.insert("input"),
            Sel::Page {
                tabs: 0,
                page: 1,
                index: 0
            }
        );
    }

    #[test]
    fn hit_finds_the_front_control_and_page_controls_first() {
        let mut d = design();
        d.insert("group_box");
        d.select(Sel::Dialog);
        d.insert("tabs");
        let page_child = d.insert("input");
        let r = d.rect_of(page_child);
        assert_eq!(d.hit(r.a), page_child);
        // The pane's own strip is the pane.
        let pane = d.rect_of(Sel::Top(1));
        assert_eq!(d.hit(pane.a), Sel::Top(1));
        // A spot nothing covers is the dialog.
        let (w, h) = d.spec.inner_size();
        assert_eq!(d.hit(Point::new(w - 1, h - 1)), Sel::Dialog);
    }

    #[test]
    fn moving_and_resizing_stay_inside_the_container() {
        let mut d = design();
        d.insert("input");
        assert!(d.move_by(-10, -10));
        let c = d.selected().unwrap();
        assert_eq!((c.x, c.y), (0, 0));
        d.resize_by(1000, 0);
        let (w, _) = d.spec.inner_size();
        assert_eq!(d.selected().unwrap().w, w);
        // Nothing to do is not an edit.
        assert!(!d.move_by(-1, 0));
    }

    #[test]
    fn the_dialog_resizes_but_not_below_the_minimum() {
        let mut d = design();
        d.resize_by(-1000, -1000);
        assert_eq!((d.spec.width, d.spec.height), MIN_DIALOG);
    }

    #[test]
    fn deleting_drops_links_and_selects_the_one_before() {
        let mut d = design();
        d.insert("label");
        d.insert("input");
        assert!(d.delete());
        assert_eq!(d.sel, Sel::Top(0));
        assert!(matches!(
            d.control(Sel::Top(0)).unwrap().kind,
            Kind::Label { link: None, .. }
        ));
        assert!(d.spec.check().is_ok());
    }

    #[test]
    fn duplicating_gives_fresh_names() {
        let mut d = design();
        d.insert("tabs");
        d.insert("label");
        d.insert("input");
        d.select(Sel::Top(0));
        assert!(d.duplicate());
        assert_eq!(d.sel, Sel::Top(1));
        assert_eq!(d.selected().unwrap().name, "tabs2");
        assert!(d.spec.check().is_ok(), "{:?}", d.spec.check());
        // The copied page's label links to the copied input.
        let Kind::Tabs { pages, .. } = &d.selected().unwrap().kind else {
            unreachable!()
        };
        assert_eq!(
            pages[0].controls[0].kind,
            Kind::Label {
                text: "~L~abel".into(),
                link: Some("input2".into())
            }
        );
    }

    #[test]
    fn tab_order_moves_among_siblings() {
        let mut d = design();
        d.insert("input");
        d.insert("memo");
        assert!(d.reorder(-1));
        assert_eq!(d.sel, Sel::Top(0));
        assert_eq!(d.selected().unwrap().name, "memo1");
        assert!(!d.reorder(-1));
    }

    #[test]
    fn tab_walks_the_dialog_then_controls_then_page_controls() {
        let mut d = design();
        d.insert("tabs");
        d.insert("input");
        d.select(Sel::Dialog);
        d.select_next(true);
        assert_eq!(d.sel, Sel::Top(0));
        d.select_next(true);
        assert!(matches!(d.sel, Sel::Page { .. }));
        d.select_next(true);
        assert_eq!(d.sel, Sel::Dialog);
        d.select_next(false);
        assert!(matches!(d.sel, Sel::Page { .. }));
    }

    #[test]
    fn undo_and_redo_walk_the_edits() {
        let mut d = design();
        d.insert("input");
        d.move_by(5, 0);
        assert!(d.undo());
        assert_eq!(d.selected().unwrap().x, 2);
        assert!(d.undo());
        assert!(d.spec.controls.is_empty());
        assert!(!d.undo());
        assert!(d.redo());
        assert!(d.redo());
        assert_eq!(d.selected().unwrap().x, 7);
    }

    #[test]
    fn renaming_carries_label_links_and_bad_edits_are_refused() {
        let mut d = design();
        d.insert("label");
        d.insert("input");
        let mut c = d.selected().unwrap().clone();
        c.name = "email".into();
        d.replace_selected(c).unwrap();
        assert!(matches!(
            &d.control(Sel::Top(0)).unwrap().kind,
            Kind::Label { link: Some(l), .. } if l == "email"
        ));

        d.insert("input");
        let mut c = d.selected().unwrap().clone();
        c.name = "email".into();
        let before = d.spec.clone();
        assert!(d.replace_selected(c).is_err());
        assert_eq!(d.spec, before);
    }
}
