// (C) 2026 - Antoni Aloy

//! The design being edited: the spec, what is selected, and every edit the
//! designer makes to them, with undo. No drawing and no terminal, so it is
//! tested on its own.

use std::path::PathBuf;
use turbo_vision::core::geometry::{Point, Rect};
use turbo_vision::views::tabbed_pane::TabbedPane;
use tv_extensions::dialog_spec::{
    Align, ColumnSpec, Control, DialogSpec, Item, Kind, LineStyle, PageSpec, SpecError, TreeNode,
};

/// The smallest dialog the designer makes, frame included.
pub const MIN_DIALOG: (i16, i16) = (16, 6);

/// How many edits undo remembers.
const UNDO_DEPTH: usize = 200;

/// How deep controls nest: a control on a page of tabs inside a group box
/// inside the dialog is three steps down.
pub const MAX_DEPTH: usize = 4;

/// Where a control is: from the dialog down, each step names the list of
/// controls to look in (`slot`: a group box has one, tabs have one per
/// page; the first step's is always 0, the dialog's own list) and the
/// control's index in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Place {
    steps: [(usize, usize); MAX_DEPTH],
    len: usize,
}

impl Place {
    /// The control at `index` in the dialog's own list.
    pub fn top(index: usize) -> Self {
        let mut steps = [(0, 0); MAX_DEPTH];
        steps[0] = (0, index);
        Self { steps, len: 1 }
    }

    /// The control at `index` in list `slot` of this control; `None` if
    /// that is nested too deep.
    pub fn child(self, slot: usize, index: usize) -> Option<Self> {
        if self.len >= MAX_DEPTH {
            return None;
        }
        let mut path = self;
        path.steps[self.len] = (slot, index);
        path.len += 1;
        Some(path)
    }

    pub fn steps(&self) -> &[(usize, usize)] {
        &self.steps[..self.len]
    }

    pub fn depth(&self) -> usize {
        self.len
    }

    /// The control's index in its list.
    pub fn index(&self) -> usize {
        self.steps[self.len - 1].1
    }

    /// The list of its container it is in.
    pub fn slot(&self) -> usize {
        self.steps[self.len - 1].0
    }

    /// The same place in the list, at `index`.
    pub fn with_index(self, index: usize) -> Self {
        let mut path = self;
        path.steps[self.len - 1].1 = index;
        path
    }

    /// The control that holds this one, `None` at the top level.
    pub fn owner(&self) -> Option<Self> {
        (self.len > 1).then(|| Self {
            steps: self.steps,
            len: self.len - 1,
        })
    }

    /// Whether `other` is this control or inside it.
    pub fn contains(&self, other: &Self) -> bool {
        other.len >= self.len && other.steps[..self.len] == self.steps[..self.len]
    }

    /// This path, for a control still there after the control at `removed`
    /// is taken out of its list: later siblings move up one.
    pub fn after_removal_of(self, removed: &Self) -> Self {
        let d = removed.len;
        let same_list = self.len >= d
            && self.steps[..d - 1] == removed.steps[..d - 1]
            && self.steps[d - 1].0 == removed.steps[d - 1].0;
        if same_list && self.steps[d - 1].1 > removed.steps[d - 1].1 {
            let mut path = self;
            path.steps[d - 1].1 -= 1;
            path
        } else {
            self
        }
    }

    /// The list this control is in.
    pub fn container(&self) -> Container {
        match self.owner() {
            None => Container::Dialog,
            Some(owner) => Container::In {
                owner,
                slot: self.slot(),
            },
        }
    }
}

/// A list of controls: the dialog's, or one inside a control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Container {
    Dialog,
    /// List `slot` of the control at `owner`: a group box's, or a tab page.
    In {
        owner: Place,
        slot: usize,
    },
}

impl Container {
    /// The path of the control at `index` in this list.
    pub fn at(self, index: usize) -> Option<Place> {
        match self {
            Self::Dialog => Some(Place::top(index)),
            Self::In { owner, slot } => owner.child(slot, index),
        }
    }
}

/// Where Arrange lines the selection up: to the edges or the middle of the
/// box around the controls selected, or of the container for one alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrange {
    Left,
    Right,
    /// The middle, across.
    Center,
    Top,
    Bottom,
    /// The middle, down.
    Middle,
}

/// What is selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sel {
    /// The dialog itself.
    Dialog,
    /// A control, wherever it is.
    At(Place),
}

impl Sel {
    /// The control at `index` in the dialog's own list.
    #[cfg(test)]
    pub fn top(index: usize) -> Self {
        Self::At(Place::top(index))
    }

    /// A control on page `page` of the tabs at `tabs` in the dialog's list.
    #[cfg(test)]
    pub fn page(tabs: usize, page: usize, index: usize) -> Self {
        Self::At(Place::top(tabs).child(page, index).expect("two steps fit"))
    }

    pub fn path(self) -> Option<Place> {
        match self {
            Self::Dialog => None,
            Self::At(path) => Some(path),
        }
    }
}

/// Every control type the designer inserts: the JSON type and its menu
/// text, in menu order, grouped by what they are for (see
/// [`SECTION_ENDS`]).
pub const KINDS: [(&str, &str); 21] = [
    // Text and decoration.
    ("label", "~L~abel"),
    ("static_text", "~S~tatic text"),
    ("line", "Li~n~e"),
    // Typing and picking a value.
    ("input", "~I~nput line"),
    ("auto_complete", "Auto-co~m~plete"),
    ("memo", "~M~emo"),
    ("spinner", "S~p~inner"),
    ("slider", "Sli~d~er"),
    // Options.
    ("check_box", "~C~heck box"),
    ("check_boxes", "Check bo~x~es"),
    ("radio_buttons", "~R~adio buttons"),
    // Choosing from lists.
    ("combo_box", "C~o~mbo box"),
    ("list_box", "Lis~t~ box"),
    ("tree", "Tree vie~w~"),
    ("table", "T~a~ble"),
    ("dual_list", "Dual ~l~ist"),
    // Containers.
    ("group_box", "Gro~u~p box"),
    ("tabs", "Ta~b~s"),
    // Actions and feedback.
    ("button", "~B~utton"),
    ("progress_bar", "Pro~g~ress bar"),
    ("error_line", "~E~rror line"),
];

/// Where each section of [`KINDS`] ends: a menu draws a line there.
pub const SECTION_ENDS: [usize; 5] = [3, 8, 11, 16, 18];

/// The sections' names, for menus that list them.
pub const SECTION_NAMES: [&str; 6] = [
    "~T~ext and lines",
    "~E~ntry fields",
    "~O~ptions",
    "~L~ists",
    "~C~ontainers",
    "~A~ctions and feedback",
];

/// The range of [`KINDS`] section `n` covers.
pub fn section(n: usize) -> std::ops::Range<usize> {
    let start = if n == 0 { 0 } else { SECTION_ENDS[n - 1] };
    let end = SECTION_ENDS.get(n).copied().unwrap_or(KINDS.len());
    start..end
}

/// The design: a spec, a selection, the file it lives in, and undo.
#[derive(Debug, Clone)]
pub struct Design {
    pub spec: DialogSpec,
    pub sel: Sel,
    /// Controls selected along with `sel` (Ctrl+click, or a band dragged
    /// over the dialog): its siblings, which move and are deleted with it.
    also: Vec<Place>,
    pub path: Option<PathBuf>,
    /// Changed since it was last saved or loaded.
    pub dirty: bool,
    /// Grows on every change to the spec or the selection, so views know
    /// when to refresh.
    pub revision: u64,
    undo: Vec<(DialogSpec, Sel)>,
    redo: Vec<(DialogSpec, Sel)>,
    /// A mouse drag is under way: it is one edit, remembered when it began.
    in_drag: bool,
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
            also: Vec::new(),
            path,
            dirty: false,
            revision: 0,
            undo: Vec::new(),
            redo: Vec::new(),
            in_drag: false,
        }
    }

    // ---- reading ----------------------------------------------------------

    /// The selected control, `None` when the dialog is selected.
    pub fn selected(&self) -> Option<&Control> {
        self.control(self.sel)
    }

    /// The control `sel` names.
    pub fn control(&self, sel: Sel) -> Option<&Control> {
        self.at(sel.path()?)
    }

    fn at(&self, path: Place) -> Option<&Control> {
        let mut list = &self.spec.controls;
        let mut found: Option<&Control> = None;
        for (k, &(slot, index)) in path.steps().iter().enumerate() {
            if k > 0 {
                list = found?.child_list(slot)?;
            }
            found = Some(list.get(index)?);
        }
        found
    }

    fn control_mut(&mut self, sel: Sel) -> Option<&mut Control> {
        at_mut(&mut self.spec.controls, sel.path()?.steps())
    }

    /// The list of controls `container` is.
    pub fn list(&self, container: Container) -> Option<&Vec<Control>> {
        match container {
            Container::Dialog => Some(&self.spec.controls),
            Container::In { owner, slot } => self.at(owner)?.child_list(slot),
        }
    }

    fn list_mut(&mut self, container: Container) -> Option<&mut Vec<Control>> {
        match container {
            Container::Dialog => Some(&mut self.spec.controls),
            Container::In { owner, slot } => {
                at_mut(&mut self.spec.controls, owner.steps())?.child_list_mut(slot)
            }
        }
    }

    /// The list of controls `sel` belongs to.
    fn siblings_mut(&mut self, sel: Sel) -> Option<&mut Vec<Control>> {
        self.list_mut(sel.path()?.container())
    }

    /// Where `container` starts inside the dialog's frame, and how big it
    /// is: the dialog's inside, a group box's inside, or a tab page.
    pub fn geometry(&self, container: Container) -> (Point, (i16, i16)) {
        match container {
            Container::Dialog => (Point::new(0, 0), self.spec.inner_size()),
            Container::In { owner, .. } => {
                let Some(c) = self.at(owner) else {
                    return (Point::new(0, 0), (0, 0));
                };
                let (origin, _) = self.geometry(owner.container());
                let area = inner_area(c);
                (
                    Point::new(origin.x + c.x + area.a.x, origin.y + c.y + area.a.y),
                    (area.width(), area.height()),
                )
            }
        }
    }

    /// The size of the area `sel`'s control is placed in: the dialog's
    /// inside, a group box's, or a tab page.
    pub fn container_size(&self, sel: Sel) -> (i16, i16) {
        match sel.path() {
            Some(path) => self.geometry(path.container()).1,
            None => self.spec.inner_size(),
        }
    }

    /// Where `sel`'s container starts, inside the dialog's frame.
    pub fn container_origin(&self, sel: Sel) -> Point {
        match sel.path() {
            Some(path) => self.geometry(path.container()).0,
            None => Point::new(0, 0),
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
    /// Controls inside a container (the shown page of tabs) come before
    /// the container.
    pub fn hit(&self, p: Point) -> Sel {
        self.hit_in(Container::Dialog, p).unwrap_or(Sel::Dialog)
    }

    fn hit_in(&self, container: Container, p: Point) -> Option<Sel> {
        let list = self.list(container)?;
        for (i, c) in list.iter().enumerate().rev() {
            let path = container.at(i)?;
            if !self.rect_of(Sel::At(path)).contains(p) {
                continue;
            }
            if let Some(slot) = shown_slot(c)
                && let Some(inside) = self.hit_in(Container::In { owner: path, slot }, p)
            {
                return Some(inside);
            }
            return Some(Sel::At(path));
        }
        None
    }

    /// Every selectable thing in tab order: the dialog, then each control,
    /// a container followed by what is in it (a tab pane's shown page).
    pub fn order(&self) -> Vec<Sel> {
        let mut order = vec![Sel::Dialog];
        self.order_in(Container::Dialog, &mut order);
        order
    }

    fn order_in(&self, container: Container, out: &mut Vec<Sel>) {
        let Some(list) = self.list(container) else {
            return;
        };
        for (i, c) in list.iter().enumerate() {
            let Some(path) = container.at(i) else {
                continue;
            };
            out.push(Sel::At(path));
            if let Some(slot) = shown_slot(c) {
                self.order_in(Container::In { owner: path, slot }, out);
            }
        }
    }

    /// The tab pane `sel` is, or is inside (the outermost), with its path.
    pub fn tabs_of(&self, sel: Sel) -> Option<Place> {
        let path = sel.path()?;
        (1..=path.depth()).find_map(|n| {
            let mut prefix = path;
            prefix.len = n;
            matches!(self.at(prefix)?.kind, Kind::Tabs { .. }).then_some(prefix)
        })
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
        first_free(prefix, |n| names.iter().any(|x| x == n))
    }

    // ---- selecting ----------------------------------------------------------

    pub fn select(&mut self, sel: Sel) {
        if self.sel != sel || !self.also.is_empty() {
            self.also.clear();
            self.sel = sel;
            self.revision += 1;
        }
    }

    /// Everything selected: `sel` first, then the controls selected along
    /// with it.
    pub fn selection(&self) -> Vec<Sel> {
        let mut all = vec![self.sel];
        all.extend(self.also.iter().map(|&p| Sel::At(p)));
        all
    }

    /// Whether `sel` is selected, alone or along with others.
    pub fn is_selected(&self, sel: Sel) -> bool {
        self.sel == sel || sel.path().is_some_and(|p| self.also.contains(&p))
    }

    /// Add `sel` to the selection, or take it out if it is in it (Ctrl+click).
    /// Only siblings are selected together: a control elsewhere, or the
    /// dialog, is selected alone.
    pub fn toggle(&mut self, sel: Sel) {
        let (Sel::At(path), Sel::At(first)) = (sel, self.sel) else {
            self.select(sel);
            return;
        };
        if path.container() != first.container() {
            self.select(sel);
            return;
        }
        if let Some(i) = self.also.iter().position(|&p| p == path) {
            self.also.remove(i);
        } else if path == first {
            // The first goes; the next one selected takes its place.
            if self.also.is_empty() {
                return;
            }
            self.sel = Sel::At(self.also.remove(0));
        } else {
            self.also.push(path);
        }
        self.revision += 1;
    }

    /// Select the dialog's own controls that lie wholly inside `band` (in
    /// the frame's inside), in list order; the dialog when there are none.
    pub fn select_within(&mut self, band: Rect) {
        let inside: Vec<Place> = (0..self.spec.controls.len())
            .filter_map(|i| Container::Dialog.at(i))
            .filter(|&p| {
                let r = self.rect_of(Sel::At(p));
                r.a.x >= band.a.x && r.a.y >= band.a.y && r.b.x <= band.b.x && r.b.y <= band.b.y
            })
            .collect();
        if let Some((&first, rest)) = inside.split_first() {
            self.sel = Sel::At(first);
            self.also = rest.to_vec();
        } else {
            self.sel = Sel::Dialog;
            self.also.clear();
        }
        self.revision += 1;
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

    /// Select what holds the selection: its container, or the dialog.
    pub fn select_parent(&mut self) {
        let parent = self
            .sel
            .path()
            .and_then(|p| p.owner())
            .map_or(Sel::Dialog, Sel::At);
        self.select(parent);
    }

    // ---- editing --------------------------------------------------------------

    /// Remember the design before an edit, for undo.
    pub fn snapshot(&mut self) {
        if self.in_drag {
            return;
        }
        self.undo.push((self.spec.clone(), self.sel));
        if self.undo.len() > UNDO_DEPTH {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Start a mouse drag: the moves and resizes until
    /// [`end_drag`](Self::end_drag) undo as one.
    pub fn begin_drag(&mut self) {
        self.snapshot();
        self.in_drag = true;
    }

    /// End a mouse drag. A drag that changed nothing leaves no undo step.
    pub fn end_drag(&mut self) {
        self.in_drag = false;
        if let Some((spec, _)) = self.undo.last()
            && *spec == self.spec
        {
            self.undo.pop();
        }
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
        self.also.clear();
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
        self.also.clear();
        self.sel = sel;
        self.changed();
        true
    }

    /// Insert a control of the JSON type `type_name`, select it, and return
    /// where it went.
    ///
    /// It goes inside the selected container (a group box, or the shown
    /// page of tabs), else after the selected control in its own list, else
    /// in the dialog; below the selection, or, after a label with no link,
    /// to the label's right and linked to it. Tabs never go inside tabs.
    pub fn insert(&mut self, type_name: &str) -> Sel {
        let (into, at) = self.insert_point(type_name);
        let (cw, ch) = self.geometry(into).1;
        let mut control = self.default_control(type_name, cw);

        // Place it: right of an unlinked label, below the selection, or at
        // the top left.
        let anchor = match self.sel {
            Sel::At(p) if p.container() == into => self.selected().cloned(),
            _ => None,
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
                let (x, y) = match into {
                    Container::In { owner, .. }
                        if matches!(
                            self.at(owner).map(|c| &c.kind),
                            Some(Kind::GroupBox { .. })
                        ) =>
                    {
                        (1, 0)
                    }
                    _ => (2, 1),
                };
                control.x = x.min(cw - 1).max(0);
                control.y = y.min(ch - 1).max(0);
            }
        }
        fit(&mut control, cw, ch);

        self.snapshot();
        if link_label
            && let Some(label) = self.control_mut(self.sel)
            && let Kind::Label { link, .. } = &mut label.kind
        {
            *link = Some(control.name.clone());
        }
        let list = self.list_mut(into).expect("the container exists");
        let at = at.min(list.len());
        list.insert(at, control);
        let sel = Sel::At(into.at(at).expect("the container has room"));
        self.also.clear();
        self.sel = sel;
        self.changed();
        sel
    }

    /// The list a new `type_name` control goes in, and where in it.
    fn insert_point(&self, type_name: &str) -> (Container, usize) {
        let len_of = |c: Container| self.list(c).map_or(0, Vec::len);
        let Sel::At(mut path) = self.sel else {
            return (Container::Dialog, self.spec.controls.len());
        };
        if type_name == "tabs"
            && let Some(outer) = self.tabs_of(self.sel)
        {
            // Tabs do not nest: go beside the tabs this is in.
            path = outer;
        }
        if let Some(c) = self.at(path)
            && let Some(slot) = shown_slot(c)
            && path.child(slot, 0).is_some()
            && !(type_name == "tabs" && matches!(c.kind, Kind::Tabs { .. }))
        {
            let into = Container::In { owner: path, slot };
            return (into, len_of(into));
        }
        (path.container(), path.index() + 1)
    }

    /// A new control of `type_name` with its usual size and settings, for a
    /// container `width` wide.
    #[allow(clippy::too_many_lines, reason = "one arm per control kind")]
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
                    color: None,
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
                    controls: Vec::new(),
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
            "tree" => (
                named("tree"),
                (30, 8),
                Kind::Tree {
                    nodes: vec![
                        TreeNode {
                            expanded: true,
                            children: vec![TreeNode::new("Leaf"), TreeNode::new("Another leaf")],
                            ..TreeNode::new("Branch")
                        },
                        TreeNode::new("Other"),
                    ],
                    selected: None,
                },
            ),
            "line" => (
                String::new(),
                (20, 1),
                Kind::Line {
                    style: LineStyle::Single,
                    color: None,
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
            let action = first_free("action", |a| commands.iter().any(|c| c == a));
            ("~A~ction".into(), action, false)
        }
    }

    /// Remove the selected control (with what is inside it), and any label
    /// link to it. Selects what came before it. With others selected, they
    /// all go.
    pub fn delete(&mut self) -> bool {
        if !self.also.is_empty() {
            return self.delete_all();
        }
        let sel = self.sel;
        let Some(removed) = self.control(sel).cloned() else {
            return false;
        };
        let Sel::At(path) = sel else {
            return false;
        };
        self.snapshot();
        let list = self.siblings_mut(sel).expect("a control has siblings");
        list.remove(path.index());
        // Names that went away: the control's, and those inside it.
        let mut gone = vec![removed.name.clone()];
        for list in removed.child_lists() {
            visit_all(list, &mut |c| gone.push(c.name.clone()));
        }
        unlink(&mut self.spec.controls, &gone);
        self.also.clear();
        self.sel = if path.index() > 0 {
            Sel::At(path.with_index(path.index() - 1))
        } else {
            path.owner().map_or(Sel::Dialog, Sel::At)
        };
        self.changed();
        true
    }

    fn delete_all(&mut self) -> bool {
        let mut paths: Vec<Place> = self.selection().iter().filter_map(|s| s.path()).collect();
        // From the end of the list, so the indices still to go stay put.
        paths.sort_by_key(|p| std::cmp::Reverse(p.index()));
        let Some(&first) = paths.last() else {
            return false;
        };
        self.snapshot();
        let mut gone = Vec::new();
        for path in &paths {
            let list = self
                .siblings_mut(Sel::At(*path))
                .expect("a control has siblings");
            let removed = list.remove(path.index());
            gone.push(removed.name.clone());
            for list in removed.child_lists() {
                visit_all(list, &mut |c| gone.push(c.name.clone()));
            }
        }
        unlink(&mut self.spec.controls, &gone);
        self.also.clear();
        self.sel = if first.index() > 0 {
            Sel::At(first.with_index(first.index() - 1))
        } else {
            first.owner().map_or(Sel::Dialog, Sel::At)
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
        let Sel::At(path) = sel else {
            return false;
        };
        let (cw, ch) = self.container_size(sel);
        self.rename_copy(&mut copy);
        copy.y += copy.h.max(1);
        fit(&mut copy, cw, ch);
        self.snapshot();
        let list = self.siblings_mut(sel).expect("a control has siblings");
        let index = path.index() + 1;
        list.insert(index, copy);
        self.also.clear();
        self.sel = Sel::At(path.with_index(index));
        self.changed();
        true
    }

    /// Give a copied control, and the controls inside it, names not in
    /// use, keeping links among the copies.
    fn rename_copy(&self, copy: &mut Control) {
        let mut taken = self.names();
        let mut fresh = |name: &str| -> String {
            if name.is_empty() {
                return String::new();
            }
            let stem = name.trim_end_matches(|c: char| c.is_ascii_digit());
            let new = first_free(stem, |n| taken.iter().any(|t| t == n));
            taken.push(new.clone());
            new
        };
        copy.name = fresh(&copy.name);
        let mut renames = Vec::new();
        let slots = copy.child_lists().len();
        for slot in 0..slots {
            if let Some(list) = copy.child_list_mut(slot) {
                for_each_mut(list, &mut |c| {
                    let new = fresh(&c.name);
                    renames.push((c.name.clone(), new.clone()));
                    c.name = new;
                });
            }
        }
        if slots == 0 {
            if let Kind::Label { link, .. } = &mut copy.kind {
                // A copied label captions nothing until it is linked again.
                *link = None;
            }
            return;
        }
        for slot in 0..slots {
            if let Some(list) = copy.child_list_mut(slot) {
                for_each_mut(list, &mut |c| {
                    if let Kind::Label {
                        link: Some(link), ..
                    } = &mut c.kind
                        && let Some((_, new)) = renames.iter().find(|(old, _)| old == link)
                    {
                        *link = new.clone();
                    }
                });
            }
        }
    }

    /// Move the selected control by `(dx, dy)`, kept inside its container.
    /// The dialog does not move: it is placed when it runs.
    /// With others selected, they all move together, as far as the one
    /// nearest the edge can.
    pub fn move_by(&mut self, dx: i16, dy: i16) -> bool {
        if !self.also.is_empty() {
            return self.move_all_by(dx, dy);
        }
        let Some(c) = self.selected() else {
            return false;
        };
        let place = (c.x + dx, c.y + dy, c.w, c.h);
        self.set_place(self.sel, place)
    }

    fn move_all_by(&mut self, mut dx: i16, mut dy: i16) -> bool {
        let (cw, ch) = self.container_size(self.sel);
        let all = self.selection();
        for &sel in &all {
            let Some(c) = self.control(sel) else {
                return false;
            };
            dx = dx.clamp(-c.x, (cw - c.w - c.x).max(0));
            dy = dy.clamp(-c.y, (ch - c.h.max(1) - c.y).max(0));
        }
        if (dx, dy) == (0, 0) {
            return false;
        }
        self.snapshot();
        for sel in all {
            let c = self.control_mut(sel).expect("checked above");
            c.x += dx;
            c.y += dy;
        }
        self.changed();
        true
    }

    /// Line the selected controls up on `how`: to the box around them, or
    /// for one control alone, to its container (so Center centres it in the
    /// dialog). One undo step; `false` when nothing moved.
    pub fn align(&mut self, how: Arrange) -> bool {
        let all = self.selection();
        let Some(rects) = self.places_of(&all) else {
            return false;
        };
        let (cw, ch) = self.container_size(self.sel);
        let (left, top, right, bottom) = if rects.len() == 1 {
            (0, 0, cw, ch)
        } else {
            rects.iter().fold(
                (i16::MAX, i16::MAX, i16::MIN, i16::MIN),
                |(l, t, r, b), &(x, y, w, h)| (l.min(x), t.min(y), r.max(x + w), b.max(y + h)),
            )
        };
        let moved: Vec<(i16, i16)> = rects
            .iter()
            .map(|&(x, y, w, h)| match how {
                Arrange::Left => (left, y),
                Arrange::Right => (right - w, y),
                Arrange::Center => (left + (right - left - w) / 2, y),
                Arrange::Top => (x, top),
                Arrange::Bottom => (x, bottom - h),
                Arrange::Middle => (x, top + (bottom - top - h) / 2),
            })
            .collect();
        self.place_all(&all, &moved, None)
    }

    /// Give the selected controls the width (`across`) or the height of the
    /// first one selected. Needs two or more.
    pub fn same_size(&mut self, across: bool) -> bool {
        let all = self.selection();
        let Some(rects) = self.places_of(&all) else {
            return false;
        };
        if rects.len() < 2 {
            return false;
        }
        let (_, _, w, h) = rects[0];
        let at: Vec<(i16, i16)> = rects.iter().map(|&(x, y, _, _)| (x, y)).collect();
        let sizes: Vec<(i16, i16)> = rects
            .iter()
            .map(|&(_, _, cw, ch)| if across { (w, ch) } else { (cw, h) })
            .collect();
        self.place_all(&all, &at, Some(&sizes))
    }

    /// Space the selected controls evenly, across or down: the first and
    /// the last stay, the others go so the gaps between them are equal.
    /// Needs three or more.
    pub fn distribute(&mut self, across: bool) -> bool {
        let all = self.selection();
        let Some(rects) = self.places_of(&all) else {
            return false;
        };
        let n = rects.len();
        if n < 3 {
            return false;
        }
        // Start and length along the axis, in order along it.
        let span = |&(x, y, w, h): &(i16, i16, i16, i16)| if across { (x, w) } else { (y, h) };
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by_key(|&i| span(&rects[i]).0);
        let (first, _) = span(&rects[order[0]]);
        let (last, last_len) = span(&rects[order[n - 1]]);
        let lengths: i16 = order.iter().map(|&i| span(&rects[i]).1).sum();
        let room = last + last_len - first - lengths;
        let gaps = i16::try_from(n - 1).unwrap_or(i16::MAX);
        let mut moved: Vec<(i16, i16)> = rects.iter().map(|&(x, y, _, _)| (x, y)).collect();
        let mut at = first;
        for (k, &i) in order.iter().enumerate() {
            let k = i16::try_from(k).unwrap_or(i16::MAX);
            moved[i] = if across {
                (at, rects[i].1)
            } else {
                (rects[i].0, at)
            };
            // The cells left over go to the first gaps, one each.
            at += span(&rects[i]).1 + room / gaps + i16::from(k < room % gaps);
        }
        self.place_all(&all, &moved, None)
    }

    /// `(x, y, w, h)` of each of `all`, a control taking at least one row;
    /// `None` when one of them is the dialog.
    fn places_of(&self, all: &[Sel]) -> Option<Vec<(i16, i16, i16, i16)>> {
        all.iter()
            .map(|&sel| self.control(sel).map(|c| (c.x, c.y, c.w, c.h.max(1))))
            .collect()
    }

    /// Put each of `all` at its corner in `at` (and size in `sizes`), kept
    /// inside the container, as one undo step. `false` when nothing changed.
    fn place_all(&mut self, all: &[Sel], at: &[(i16, i16)], sizes: Option<&[(i16, i16)]>) -> bool {
        let (cw, ch) = self.container_size(self.sel);
        let mut changed = Vec::new();
        for (k, &sel) in all.iter().enumerate() {
            let Some(current) = self.control(sel) else {
                return false;
            };
            let mut placed = current.clone();
            (placed.x, placed.y) = at[k];
            if let Some(sizes) = sizes {
                (placed.w, placed.h) = sizes[k];
            }
            fit(&mut placed, cw, ch);
            if placed != *current {
                changed.push((sel, placed));
            }
        }
        if changed.is_empty() {
            return false;
        }
        self.snapshot();
        for (sel, placed) in changed {
            *self.control_mut(sel).expect("checked above") = placed;
        }
        self.changed();
        true
    }

    /// Grow or shrink the selection by `(dw, dh)`: a control within its
    /// container, or the dialog.
    pub fn resize_by(&mut self, dw: i16, dh: i16) -> bool {
        if let Some(c) = self.selected() {
            let place = (c.x, c.y, c.w + dw, c.h + dh);
            self.set_place(self.sel, place)
        } else {
            let (w, h) = (self.spec.width + dw, self.spec.height + dh);
            self.set_dialog_size(w, h)
        }
    }

    /// Put `sel` at `place`, `(x, y, w, h)`, kept inside its container and
    /// at least one cell. Records undo only for a real change.
    pub fn set_place(&mut self, sel: Sel, place: (i16, i16, i16, i16)) -> bool {
        let (width, height) = self.container_size(sel);
        let Some(current) = self.control(sel) else {
            return false;
        };
        let mut moved = current.clone();
        (moved.x, moved.y, moved.w, moved.h) = place;
        fit(&mut moved, width, height);
        if moved == *current {
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
        let Sel::At(path) = sel else {
            return false;
        };
        let index = path.index();
        let len = self.siblings_mut(sel).map_or(0, |l| l.len());
        let Some(to) = index.checked_add_signed(delta).filter(|&to| to < len) else {
            return false;
        };
        self.snapshot();
        self.siblings_mut(sel).expect("checked").swap(index, to);
        self.also.clear();
        self.sel = Sel::At(path.with_index(to));
        self.changed();
        true
    }

    /// The controls the selected one is tab-ordered among, described one
    /// per line, and the selected one's place in them.
    pub fn order_group(&self) -> Option<(Vec<String>, usize)> {
        let Sel::At(path) = self.sel else {
            return None;
        };
        let list = self.list(path.container())?;
        let rows = list
            .iter()
            .map(|c| {
                if c.name.is_empty() {
                    c.type_name().to_string()
                } else {
                    format!("{} {}", c.type_name(), c.name)
                }
            })
            .collect();
        Some((rows, path.index()))
    }

    /// Put the selected control's siblings in `order`: `order[n]` is the
    /// current index of the one that becomes the n-th. The selection stays
    /// on the same control. False when `order` is not a reordering.
    pub fn set_order(&mut self, order: &[usize]) -> bool {
        let Sel::At(path) = self.sel else {
            return false;
        };
        let Some(len) = self.siblings_mut(self.sel).map(|l| l.len()) else {
            return false;
        };
        let mut seen = vec![false; len];
        let valid = order.len() == len
            && order
                .iter()
                .all(|&i| i < len && !std::mem::replace(&mut seen[i], true));
        if !valid || order.iter().enumerate().all(|(n, &i)| n == i) {
            return false;
        }
        self.snapshot();
        let list = self.siblings_mut(self.sel).expect("checked");
        let mut old: Vec<Option<Control>> = std::mem::take(list).into_iter().map(Some).collect();
        *list = order
            .iter()
            .map(|&i| old[i].take().expect("a permutation"))
            .collect();
        let at = order.iter().position(|&i| i == path.index()).unwrap_or(0);
        self.also.clear();
        self.sel = Sel::At(path.with_index(at));
        self.changed();
        true
    }

    /// Show page `page` of the tab pane at `tabs`; it is also the page the
    /// dialog opens on. Not an undoable edit.
    pub fn show_page(&mut self, tabs: Place, page: usize) -> bool {
        let Some(Control {
            kind: Kind::Tabs { pages, active },
            ..
        }) = self.control_mut(Sel::At(tabs))
        else {
            return false;
        };
        if page >= pages.len() || *active == page {
            return false;
        }
        *active = page;
        if let Sel::At(sel) = self.sel
            && tabs.contains(&sel)
            && sel != tabs
        {
            self.also.clear();
            self.sel = Sel::At(tabs);
        }
        self.changed();
        true
    }

    /// Show the pages of tabs that lead to `sel`, so it can be seen.
    pub fn reveal(&mut self, sel: Sel) {
        let Some(path) = sel.path() else {
            return;
        };
        for k in 1..path.depth() {
            let mut tabs = path;
            tabs.len = k;
            if matches!(self.at(tabs).map(|c| &c.kind), Some(Kind::Tabs { .. })) {
                self.show_page(tabs, path.steps()[k].0);
            }
        }
    }

    /// Where the selected control can be moved to, with a name for each:
    /// the dialog, and the group boxes and tab pages it is not in or being.
    pub fn move_targets(&self) -> Vec<(Container, String)> {
        let Sel::At(from) = self.sel else {
            return Vec::new();
        };
        let mut out = Vec::new();
        if self.can_move(from, Container::Dialog) {
            out.push((Container::Dialog, "Dialog".to_string()));
        }
        self.collect_targets(Container::Dialog, from, "", &mut out);
        out
    }

    fn collect_targets(
        &self,
        container: Container,
        from: Place,
        prefix: &str,
        out: &mut Vec<(Container, String)>,
    ) {
        let Some(list) = self.list(container) else {
            return;
        };
        for (i, c) in list.iter().enumerate() {
            let Some(path) = container.at(i) else {
                continue;
            };
            let name = format!("{prefix}{}", c.describe());
            for slot in 0..c.child_lists().len() {
                let into = Container::In { owner: path, slot };
                let label = match &c.kind {
                    Kind::Tabs { pages, .. } => {
                        let title: String =
                            pages[slot].title.chars().filter(|&ch| ch != '~').collect();
                        format!("{name} > page \"{title}\"")
                    }
                    _ => name.clone(),
                };
                if self.can_move(from, into) {
                    out.push((into, label.clone()));
                }
                self.collect_targets(into, from, &format!("{label} > "), out);
            }
        }
    }

    /// Whether the control at `from` can go to the end of `to`: not where
    /// it already is, not inside itself, not tabs in tabs, and not nested
    /// too deep.
    pub fn can_move(&self, from: Place, to: Container) -> bool {
        let Some(control) = self.at(from) else {
            return false;
        };
        if to == from.container() {
            return false;
        }
        let new_depth = match to {
            Container::Dialog => 1,
            Container::In { owner, .. } => {
                if from.contains(&owner) {
                    return false;
                }
                owner.depth() + 1
            }
        };
        if new_depth + height(control) - 1 > MAX_DEPTH {
            return false;
        }
        let mut has_tabs = matches!(control.kind, Kind::Tabs { .. });
        for list in control.child_lists() {
            visit_all(list, &mut |c| {
                has_tabs |= matches!(c.kind, Kind::Tabs { .. });
            });
        }
        if has_tabs
            && let Container::In { owner, .. } = to
            && self.tabs_of(Sel::At(owner)).is_some()
        {
            return false;
        }
        true
    }

    /// Move the selected control into `to`, at the end of its list, kept
    /// inside it. The control stays selected, and its tab page is shown.
    pub fn move_selected(&mut self, to: Container) -> bool {
        let Sel::At(from) = self.sel else {
            return false;
        };
        if !self.can_move(from, to) {
            return false;
        }
        self.snapshot();
        let mut control = self
            .siblings_mut(self.sel)
            .expect("a control has siblings")
            .remove(from.index());
        let to = match to {
            Container::Dialog => Container::Dialog,
            Container::In { owner, slot } => Container::In {
                owner: owner.after_removal_of(&from),
                slot,
            },
        };
        let (cw, ch) = self.geometry(to).1;
        fit(&mut control, cw, ch);
        let list = self.list_mut(to).expect("the target exists");
        list.push(control);
        let index = list.len() - 1;
        let sel = Sel::At(to.at(index).expect("the target has room"));
        self.also.clear();
        self.sel = sel;
        self.reveal(sel);
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
    pub fn replace_selected(&mut self, control: &Control) -> Result<(), SpecError> {
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

/// `stem` followed by the lowest number from 1 that `taken` does not hold.
fn first_free(stem: &str, taken: impl Fn(&str) -> bool) -> String {
    let mut n = 1u32;
    loop {
        let candidate = format!("{stem}{n}");
        if !taken(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

/// Blank rows left between a new control and the one above it.
fn usize_gap(type_name: &str) -> i16 {
    i16::from(!matches!(
        type_name,
        "label" | "input" | "check_box" | "static_text" | "line"
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

/// A container's inside, in the container's own space: the page area of
/// tabs, the space within a group box's frame.
fn inner_area(c: &Control) -> Rect {
    match c.kind {
        Kind::Tabs { .. } => page_area(c),
        Kind::GroupBox { .. } => Rect::new(1, 1, (c.w - 1).max(1), (c.h - 1).max(1)),
        _ => Rect::new(0, 0, 0, 0),
    }
}

/// The list a container shows: the shown page of tabs, a group box's.
fn shown_slot(c: &Control) -> Option<usize> {
    match &c.kind {
        Kind::Tabs { pages, active } if !pages.is_empty() => Some((*active).min(pages.len() - 1)),
        Kind::GroupBox { .. } => Some(0),
        _ => None,
    }
}

/// How many levels a control spans: 1 for one with nothing inside it.
fn height(c: &Control) -> usize {
    1 + c
        .child_lists()
        .iter()
        .flat_map(|l| l.iter())
        .map(height)
        .max()
        .unwrap_or(0)
}

fn at_mut<'a>(list: &'a mut [Control], steps: &[(usize, usize)]) -> Option<&'a mut Control> {
    let (&(_, index), rest) = steps.split_first()?;
    let c = list.get_mut(index)?;
    match rest.first() {
        None => Some(c),
        Some(&(slot, _)) => at_mut(c.child_list_mut(slot)?, rest),
    }
}

fn visit_all(controls: &[Control], f: &mut impl FnMut(&Control)) {
    for c in controls {
        f(c);
        for list in c.child_lists() {
            visit_all(list, f);
        }
    }
}

fn for_each_mut(controls: &mut [Control], f: &mut impl FnMut(&mut Control)) {
    for c in controls {
        f(c);
        let slots = c.child_lists().len();
        for slot in 0..slots {
            if let Some(list) = c.child_list_mut(slot) {
                for_each_mut(list, f);
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
        assert_eq!(sel, Sel::top(0));
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
        let label = d.control(Sel::top(0)).unwrap();
        let input = d.control(Sel::top(1)).unwrap();
        assert_eq!(input.y, label.y);
        assert_eq!(input.x, label.x + label.w + 1);
        assert_eq!(
            label.kind,
            Kind::Label {
                text: "~L~abel".into(),
                link: Some("input1".into()),
                color: None,
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
        assert_eq!(sel, Sel::page(0, 0, 0));
        // Another tab pane is never put on a page.
        assert_eq!(d.insert("tabs"), Sel::top(1));
        d.select(Sel::top(0));
        d.show_page(Place::top(0), 1);
        assert_eq!(d.insert("input"), Sel::page(0, 1, 0));
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
        let pane = d.rect_of(Sel::top(1));
        assert_eq!(d.hit(pane.a), Sel::top(1));
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
        assert_eq!(d.sel, Sel::top(0));
        assert!(matches!(
            d.control(Sel::top(0)).unwrap().kind,
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
        d.select(Sel::top(0));
        assert!(d.duplicate());
        assert_eq!(d.sel, Sel::top(1));
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
                link: Some("input2".into()),
                color: None,
            }
        );
    }

    #[test]
    fn a_whole_tab_order_is_set_at_once() {
        let mut d = Design::default();
        for kind in ["label", "input", "button"] {
            d.insert(kind);
        }
        d.select(Sel::At(Place::top(0)));
        let first = d.selected().unwrap().type_name();
        assert!(!d.set_order(&[0, 0, 1]), "not a permutation");
        assert!(!d.set_order(&[0, 1, 2]), "nothing changes");
        assert!(d.set_order(&[2, 0, 1]));
        assert_eq!(d.sel, Sel::At(Place::top(1)));
        assert_eq!(d.selected().unwrap().type_name(), first);
        assert_eq!(d.spec.controls[0].type_name(), "button");
        assert!(d.undo());
        assert_eq!(d.spec.controls[0].type_name(), first);
    }

    #[test]
    fn tab_order_moves_among_siblings() {
        let mut d = design();
        d.insert("input");
        d.insert("memo");
        assert!(d.reorder(-1));
        assert_eq!(d.sel, Sel::top(0));
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
        assert_eq!(d.sel, Sel::top(0));
        d.select_next(true);
        assert_eq!(d.sel.path().unwrap().depth(), 2);
        d.select_next(true);
        assert_eq!(d.sel, Sel::Dialog);
        d.select_next(false);
        assert_eq!(d.sel.path().unwrap().depth(), 2);
    }

    #[test]
    fn undo_and_redo_walk_the_edits() {
        let mut d = design();
        d.insert("input");
        d.move_by(5, 0);
        assert!(d.undo());
        assert_eq!(d.selected().unwrap().x, 2);
        assert!(d.undo());
        assert_eq!(d.spec.controls.len(), 0);
        assert!(!d.undo());
        assert!(d.redo());
        assert!(d.redo());
        assert_eq!(d.selected().unwrap().x, 7);
    }

    #[test]
    fn a_drag_is_one_undo_step() {
        let mut d = design();
        d.insert("input");
        d.begin_drag();
        d.move_by(1, 0);
        d.move_by(1, 0);
        d.resize_by(3, 0);
        d.end_drag();
        assert!(d.undo());
        assert_eq!(d.selected().unwrap().x, 2);
        assert_eq!(d.selected().unwrap().w, 20);
        // A click with no move leaves nothing to undo.
        d.redo();
        d.begin_drag();
        d.end_drag();
        assert!(d.undo());
        assert_eq!(d.selected().unwrap().x, 2);
    }

    #[test]
    fn renaming_carries_label_links_and_bad_edits_are_refused() {
        let mut d = design();
        d.insert("label");
        d.insert("input");
        let mut c = d.selected().unwrap().clone();
        c.name = "email".into();
        d.replace_selected(&c).unwrap();
        assert!(matches!(
            &d.control(Sel::top(0)).unwrap().kind,
            Kind::Label { link: Some(l), .. } if l == "email"
        ));

        d.insert("input");
        let mut c = d.selected().unwrap().clone();
        c.name = "email".into();
        let before = d.spec.clone();
        assert!(d.replace_selected(&c).is_err());
        assert_eq!(d.spec, before);
    }

    #[test]
    fn controls_inserted_with_a_group_box_selected_go_inside_it() {
        let mut d = design();
        d.insert("group_box");
        let sel = d.insert("input");
        assert_eq!(sel, Sel::At(Place::top(0).child(0, 0).unwrap()));
        let Kind::GroupBox { controls, .. } = &d.control(Sel::top(0)).unwrap().kind else {
            unreachable!()
        };
        assert_eq!(controls.len(), 1);
        // It sits inside the box's frame, and the next goes below it.
        let r = d.rect_of(sel);
        let frame = d.rect_of(Sel::top(0));
        assert!(frame.contains(r.a) && frame.contains(Point::new(r.b.x - 1, r.b.y - 1)));
        let second = d.insert("input");
        assert_eq!(second, Sel::At(Place::top(0).child(0, 1).unwrap()));
        assert!(d.rect_of(second).a.y > r.a.y);
        assert!(d.spec.check().is_ok());
        // With the dialog selected, a control goes in the dialog.
        d.select(Sel::Dialog);
        assert_eq!(d.insert("memo"), Sel::top(1));
    }

    #[test]
    fn containers_nest_a_group_on_a_page_holds_controls() {
        let mut d = design();
        d.insert("tabs");
        let group = d.insert("group_box");
        assert_eq!(group, Sel::page(0, 0, 0));
        let input = d.insert("input");
        let path = input.path().unwrap();
        assert_eq!(path.depth(), 3);
        // Hit testing finds the innermost control.
        let r = d.rect_of(input);
        assert_eq!(d.hit(r.a), input);
        // Tabs never go inside tabs, however deep the selection is.
        let tabs = d.insert("tabs");
        assert_eq!(tabs, Sel::top(1));
        assert!(d.spec.check().is_ok(), "{:?}", d.spec.check());
    }

    #[test]
    fn deleting_inside_a_group_selects_the_one_before_or_the_group() {
        let mut d = design();
        d.insert("group_box");
        let first = d.insert("input");
        d.insert("input");
        assert!(d.delete());
        assert_eq!(d.sel, first);
        assert!(d.delete());
        assert_eq!(d.sel, Sel::top(0));
    }

    #[test]
    fn a_control_moves_to_another_container() {
        let mut d = design();
        d.insert("group_box");
        d.select(Sel::Dialog);
        d.insert("input");
        let input = d.sel;
        let into = Container::In {
            owner: Place::top(0),
            slot: 0,
        };
        assert!(d.move_targets().iter().any(|(c, _)| *c == into));
        assert!(d.move_selected(into));
        assert_eq!(d.sel, Sel::At(Place::top(0).child(0, 0).unwrap()));
        assert_eq!(d.spec.controls.len(), 1);
        // And back out; it is not offered where it already is.
        assert!(!d.move_selected(into));
        assert!(d.move_selected(Container::Dialog));
        assert_eq!(d.sel, Sel::top(1));
        assert_eq!(d.selected().unwrap().name, "input1");
        let _ = input;
        // One undo puts it back inside.
        assert!(d.undo());
        assert_eq!(d.spec.controls.len(), 1);
    }

    #[test]
    fn a_move_is_refused_into_itself_or_tabs_in_tabs() {
        let mut d = design();
        d.insert("group_box");
        d.insert("group_box"); // inside the first
        d.select(Sel::top(0));
        let inner = Container::In {
            owner: Place::top(0).child(0, 0).unwrap(),
            slot: 0,
        };
        assert!(!d.can_move(Place::top(0), inner), "into its own child");
        assert!(d.move_targets().is_empty() || d.move_targets().iter().all(|(c, _)| *c != inner));
        d.select(Sel::Dialog);
        d.insert("tabs");
        let tabs = d.sel;
        d.select(Sel::Dialog);
        d.insert("tabs");
        let other = d.sel.path().unwrap();
        let page = Container::In {
            owner: tabs.path().unwrap(),
            slot: 0,
        };
        assert!(!d.can_move(other, page), "tabs in tabs");
    }

    #[test]
    fn moving_past_a_later_sibling_keeps_the_target() {
        // The target group is after the control in the same list: taking
        // the control out shifts the group's index.
        let mut d = design();
        d.insert("input");
        d.select(Sel::Dialog);
        d.insert("group_box");
        d.select(Sel::top(0));
        let group = Container::In {
            owner: Place::top(1),
            slot: 0,
        };
        assert!(d.move_selected(group));
        assert_eq!(d.spec.controls.len(), 1);
        assert_eq!(d.sel, Sel::At(Place::top(0).child(0, 0).unwrap()));
        assert_eq!(d.selected().unwrap().name, "input1");
    }

    #[test]
    fn duplicating_a_group_copies_what_is_inside_with_fresh_names() {
        let mut d = design();
        d.insert("group_box");
        d.insert("label");
        d.insert("input");
        d.select(Sel::top(0));
        assert!(d.duplicate());
        assert!(d.spec.check().is_ok(), "{:?}", d.spec.check());
        let copy = d.selected().unwrap();
        let Kind::GroupBox { controls, .. } = &copy.kind else {
            unreachable!()
        };
        assert_eq!(controls[1].name, "input2");
        assert_eq!(
            controls[0].kind,
            Kind::Label {
                text: "~L~abel".into(),
                link: Some("input2".into()),
                color: None,
            }
        );
    }
}
