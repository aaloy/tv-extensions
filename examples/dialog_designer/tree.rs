// (C) 2026 - Antoni Aloy

//! The controls of the dialog as a tree: the dialog, its controls, and the
//! pages of its tabs with the controls on each. Moving in it selects in
//! the design; selecting in the design moves in it.

use crate::model::{Container, Design, Place, Sel};
use std::cell::RefCell;
use std::rc::Rc;
use turbo_vision::core::command::CommandId;
use turbo_vision::core::event::{Event, EventType, KB_ENTER, MB_LEFT_BUTTON};
use turbo_vision::core::state::State;
use turbo_vision::terminal::Terminal;
use turbo_vision::views::list_viewer::ListViewer;
use turbo_vision::views::outline::{Node, OutlineViewer};
use turbo_vision::views::view::{View, ViewCore};
use tv_extensions::dialog_spec::Kind;

/// What a row of the tree stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The selection it makes in the design.
    pub sel: Sel,
    /// For the row of a tab page: the tabs' path and the page, to show.
    pub page: Option<(Place, usize)>,
    /// The list of controls the row stands for, if it is a container: where
    /// a control dropped on it goes.
    pub holds: Option<Container>,
    pub label: String,
}

type Rows = Vec<Rc<RefCell<Node<Entry>>>>;

/// Text without its `~` hot-key marks.
fn plain(text: &str) -> String {
    text.chars().filter(|&c| c != '~').collect()
}

fn node(entry: Entry, children: Rows) -> Rc<RefCell<Node<Entry>>> {
    let mut node = Node::with_children(entry, children);
    node.expanded = true;
    Rc::new(RefCell::new(node))
}

/// The rows of the controls in `container`, each with what is inside it.
fn rows_in(design: &Design, container: Container) -> Rows {
    let Some(list) = design.list(container) else {
        return Vec::new();
    };
    list.iter()
        .enumerate()
        .filter_map(|(i, control)| {
            let path = container.at(i)?;
            let children = match &control.kind {
                Kind::Tabs { pages, .. } => pages
                    .iter()
                    .enumerate()
                    .map(|(p, page)| {
                        let into = Container::In { owner: path, slot: p };
                        node(
                            Entry {
                                sel: Sel::At(path),
                                page: Some((path, p)),
                                holds: Some(into),
                                label: format!("page \"{}\"", plain(&page.title)),
                            },
                            rows_in(design, into),
                        )
                    })
                    .collect(),
                Kind::GroupBox { .. } => rows_in(design, Container::In { owner: path, slot: 0 }),
                _ => Vec::new(),
            };
            let holds = matches!(control.kind, Kind::GroupBox { .. })
                .then_some(Container::In { owner: path, slot: 0 });
            Some(node(
                Entry {
                    sel: Sel::At(path),
                    page: None,
                    holds,
                    label: plain(&control.describe()),
                },
                children,
            ))
        })
        .collect()
}

/// The rows of `design`, parents before children.
fn rows(design: &Design) -> Rows {
    vec![node(
        Entry {
            sel: Sel::Dialog,
            page: None,
            holds: Some(Container::Dialog),
            label: format!("Dialog \"{}\"", design.spec.title),
        },
        rows_in(design, Container::Dialog),
    )]
}

/// Every entry of `rows`, in display order.
fn flatten(rows: &Rows, out: &mut Vec<Entry>) {
    for row in rows {
        out.push(row.borrow().data.clone());
        flatten(&row.borrow().children, out);
    }
}

/// The tree view of the controls.
pub struct ControlTree {
    inner: OutlineViewer<Entry>,
    /// Sent when Enter or a double click asks for the properties.
    properties: CommandId,
    /// The entries the tree was built from.
    shown: Vec<Entry>,
    /// The row the design and the tree last agreed on.
    synced: Option<Entry>,
    /// Sent when a row is dropped on a container.
    dropped_command: CommandId,
    /// The row the mouse went down on, while it is down.
    grab: Option<Entry>,
    /// What was dropped, and on what, until the designer takes it.
    dropped: Option<(Sel, Container)>,
}

impl ControlTree {
    pub fn new(
        bounds: turbo_vision::core::geometry::Rect,
        properties: CommandId,
        dropped_command: CommandId,
    ) -> Self {
        Self {
            inner: OutlineViewer::new(bounds, |entry: &Entry| entry.label.clone()),
            properties,
            shown: Vec::new(),
            synced: None,
            dropped_command,
            grab: None,
            dropped: None,
        }
    }

    /// The control dropped on a container since the last call, and the
    /// container it was dropped on.
    pub fn take_dropped(&mut self) -> Option<(Sel, Container)> {
        self.dropped.take()
    }

    /// The row on display line `line`, without moving the tree's row.
    fn entry_at_line(&mut self, line: i16) -> Option<Entry> {
        let row = self.inner.top_item() + usize::try_from(line).ok()?;
        if row >= self.inner.item_count() {
            return None;
        }
        let back = self.inner.focused_item();
        self.inner.set_focused_item(Some(row));
        let entry = self.inner.selected_node().map(|n| n.borrow().data.clone());
        self.inner.set_focused_item(back);
        entry
    }

    /// Every row, in display order.
    #[cfg(test)]
    pub fn entries(&self) -> &[Entry] {
        &self.shown
    }

    /// The row the tree is on.
    #[cfg(test)]
    pub fn current(&self) -> Option<Entry> {
        self.inner.selected_node().map(|n| n.borrow().data.clone())
    }

    /// The row the person moved to since the last call, if they did.
    pub fn moved(&mut self) -> Option<Entry> {
        let now = self.inner.selected_node()?.borrow().data.clone();
        let synced = self.synced.as_ref()?;
        if *synced == now {
            return None;
        }
        self.synced = Some(now.clone());
        Some(now)
    }

    /// Follow the design: rebuild when its controls changed, and move to
    /// its selection when that is not where the tree is.
    pub fn follow(&mut self, design: &Design) {
        let rows = rows(design);
        let mut flat = Vec::new();
        flatten(&rows, &mut flat);
        if flat != self.shown {
            self.shown = flat;
            self.inner.set_roots(rows);
            self.synced = None;
        }
        if self.synced.as_ref().map(|e| e.sel) == Some(design.sel) {
            return;
        }
        let want = |e: &Entry| e.sel == design.sel && e.page.is_none();
        for (i, entry) in self.shown.iter().enumerate() {
            if want(entry) {
                self.inner.set_focused_item(Some(i));
                // Collapsed rows hide this one: what the tree shows is
                // what it keeps.
                if let Some(node) = self.inner.selected_node()
                    && want(&node.borrow().data)
                {
                    self.synced = Some(entry.clone());
                }
                return;
            }
        }
    }
}

impl View for ControlTree {
    fn core(&self) -> &ViewCore {
        self.inner.core()
    }

    fn core_mut(&mut self) -> &mut ViewCore {
        self.inner.core_mut()
    }

    fn draw(&mut self, terminal: &mut Terminal) {
        self.inner.draw(terminal);
    }

    fn handle_event(&mut self, event: &mut Event) {
        let enter = event.what == EventType::Keyboard && event.key_code == KB_ENTER;
        let double = event.what == EventType::MouseDown && event.mouse.double_click;
        if enter {
            *event = Event::command(self.properties);
            return;
        }
        let down = event.what == EventType::MouseDown && event.mouse.buttons & MB_LEFT_BUTTON != 0;
        let up = event.what == EventType::MouseUp;
        let line = event.mouse.pos.y;
        self.inner.handle_event(event);
        if double {
            *event = Event::command(self.properties);
        } else if down {
            // A control can be dragged onto a container: remember what is
            // under the mouse, and capture the button's release.
            self.grab = self.inner.selected_node().map(|n| n.borrow().data.clone());
            self.set_state_flag(State::DRAGGING, true);
        } else if up && let Some(from) = self.grab.take() {
            self.set_state_flag(State::DRAGGING, false);
            if from.page.is_none()
                && from.sel != Sel::Dialog
                && let Some(onto) = self.entry_at_line(line)
                && let Some(container) = onto.holds
                && onto != from
            {
                self.dropped = Some((from.sel, container));
                *event = Event::command(self.dropped_command);
            }
        }
    }

    fn can_focus(&self) -> bool {
        true
    }

    fn get_palette(&self) -> Option<turbo_vision::core::palette::Palette> {
        self.inner.get_palette()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
