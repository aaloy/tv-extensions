// (C) 2026 - Antoni Aloy

//! The controls of the dialog as a tree: the dialog, its controls, and the
//! pages of its tabs with the controls on each. Moving in it selects in
//! the design; selecting in the design moves in it.

use crate::model::{Design, Sel};
use std::cell::RefCell;
use std::rc::Rc;
use turbo_vision::core::command::CommandId;
use turbo_vision::core::event::{Event, EventType, KB_ENTER};
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
    /// For the row of a tab page: the tabs' index and the page, to show.
    pub page: Option<(usize, usize)>,
    pub label: String,
}

type Rows = Vec<Rc<RefCell<Node<Entry>>>>;

/// Text without its `~` hot-key marks.
fn plain(text: &str) -> String {
    text.chars().filter(|&c| c != '~').collect()
}

/// The rows of `design`, parents before children.
fn rows(design: &Design) -> Rows {
    let node = |entry: Entry, children: Rows| {
        let mut node = Node::with_children(entry, children);
        node.expanded = true;
        Rc::new(RefCell::new(node))
    };
    let top = design
        .spec
        .controls
        .iter()
        .enumerate()
        .map(|(i, control)| {
            let pages = match &control.kind {
                Kind::Tabs { pages, .. } => pages
                    .iter()
                    .enumerate()
                    .map(|(p, page)| {
                        let controls = page
                            .controls
                            .iter()
                            .enumerate()
                            .map(|(index, c)| {
                                let sel = Sel::Page {
                                    tabs: i,
                                    page: p,
                                    index,
                                };
                                node(
                                    Entry {
                                        sel,
                                        page: None,
                                        label: plain(&c.describe()),
                                    },
                                    Vec::new(),
                                )
                            })
                            .collect();
                        let title = plain(&page.title);
                        node(
                            Entry {
                                sel: Sel::Top(i),
                                page: Some((i, p)),
                                label: format!("page \"{title}\""),
                            },
                            controls,
                        )
                    })
                    .collect(),
                _ => Vec::new(),
            };
            node(
                Entry {
                    sel: Sel::Top(i),
                    page: None,
                    label: plain(&control.describe()),
                },
                pages,
            )
        })
        .collect();
    vec![node(
        Entry {
            sel: Sel::Dialog,
            page: None,
            label: format!("Dialog \"{}\"", design.spec.title),
        },
        top,
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
}

impl ControlTree {
    pub fn new(bounds: turbo_vision::core::geometry::Rect, properties: CommandId) -> Self {
        Self {
            inner: OutlineViewer::new(bounds, |entry: &Entry| entry.label.clone()),
            properties,
            shown: Vec::new(),
            synced: None,
        }
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
        self.inner.handle_event(event);
        if double {
            *event = Event::command(self.properties);
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
