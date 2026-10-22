// (C) 2026 - Antoni Aloy

//! The design surface: the dialog being designed, drawn by its real
//! controls but inert, with the selection marked on top. The mouse selects,
//! drags and resizes; the keyboard moves, resizes and walks the controls.

use crate::model::{Design, Sel};
use std::cell::RefCell;
use std::rc::Rc;
use turbo_vision::core::command::CommandId;
use turbo_vision::core::draw::Cell;
use turbo_vision::core::event::{
    Event, EventType, KB_DEL, KB_DOWN, KB_ENTER, KB_ESC, KB_LEFT, KB_PGDN, KB_PGUP, KB_RIGHT,
    KB_SHIFT_TAB, KB_TAB, KB_UP, MB_LEFT_BUTTON, MB_RIGHT_BUTTON,
};
use turbo_vision::core::geometry::{Point, Rect};
use turbo_vision::core::keys::KeyModifiers;
use turbo_vision::core::palette::{Attr, TvColor};
use turbo_vision::core::state::State;
use turbo_vision::terminal::Terminal;
use turbo_vision::views::tabbed_pane::TabbedPane;
use turbo_vision::views::view::{View, ViewCore, draw_child};
use tv_extensions::dialog_spec::{DialogSpec, JsonDialog, Kind};

/// The commands the surface sends the application.
pub struct Commands {
    pub properties: CommandId,
    pub delete: CommandId,
    pub insert_menu: CommandId,
}

/// What a mouse drag is doing.
#[derive(Debug, Clone, Copy)]
enum Drag {
    /// Moving `sel`, held at `grab` from its corner.
    Move { sel: Sel, grab: Point },
    /// Resizing `sel` by its bottom-right corner.
    Resize { sel: Sel },
    /// Resizing the dialog by its frame's bottom-right corner.
    Dialog,
}

/// The surface: a view on the desktop exactly as big as the dialog.
pub struct Surface {
    core: ViewCore,
    design: Rc<RefCell<Design>>,
    commands: Commands,
    /// The dialog built from the spec, redrawn as it is; rebuilt when the
    /// spec changes.
    live: Option<JsonDialog>,
    /// Why the spec could not be built, shown instead of the dialog.
    broken: Option<String>,
    built: Option<DialogSpec>,
    drag: Option<Drag>,
    /// Number each control with its place in tab order.
    pub show_order: bool,
}

impl Surface {
    /// A surface for `design`, its corner at `at` on the desktop.
    pub fn new(at: Point, design: Rc<RefCell<Design>>, commands: Commands) -> Self {
        let (w, h) = {
            let d = design.borrow();
            (d.spec.width, d.spec.height)
        };
        let mut core = ViewCore::new(Rect::new(at.x, at.y, at.x + w, at.y + h));
        core.state = State::empty();
        Self {
            core,
            design,
            commands,
            live: None,
            broken: None,
            built: None,
            drag: None,
            show_order: false,
        }
    }

    /// Follow the spec's size, keeping the corner where it is.
    pub fn sync_bounds(&mut self) {
        let (w, h) = {
            let d = self.design.borrow();
            (d.spec.width, d.spec.height)
        };
        let a = self.core.bounds.a;
        self.core.bounds = Rect::new(a.x, a.y, a.x + w, a.y + h);
    }

    /// Build the live dialog again if the spec changed since.
    fn rebuild(&mut self) {
        let spec = self.design.borrow().spec.clone();
        if self.built.as_ref() == Some(&spec) {
            return;
        }
        // Drawn at the surface's corner, whatever the spec says.
        let mut at_origin = spec.clone();
        at_origin.x = Some(0);
        at_origin.y = Some(0);
        match JsonDialog::new(&at_origin) {
            Ok(live) => {
                self.live = Some(live);
                self.broken = None;
            }
            Err(e) => {
                self.live = None;
                self.broken = Some(e.to_string());
            }
        }
        self.built = Some(spec);
    }

    /// A key or mouse action became a command for the application.
    fn send(event: &mut Event, command: CommandId) {
        *event = Event::command(command);
    }

    fn mouse_down(&mut self, event: &mut Event) {
        let pos = event.mouse.pos;
        let (w, h) = (self.core.bounds.width(), self.core.bounds.height());
        let shared = Rc::clone(&self.design);
        let mut design = shared.borrow_mut();

        if event.mouse.buttons & MB_RIGHT_BUTTON != 0 {
            let inner = Point::new(pos.x - 1, pos.y - 1);
            let sel = design.hit(inner);
            design.select(sel);
            drop(design);
            Self::send(event, self.commands.insert_menu);
            return;
        }
        if event.mouse.buttons & MB_LEFT_BUTTON == 0 {
            return;
        }
        // The frame's bottom-right corner resizes the dialog.
        if pos == Point::new(w - 1, h - 1) {
            design.select(Sel::Dialog);
            design.begin_drag();
            self.drag = Some(Drag::Dialog);
            self.set_state_flag(State::DRAGGING, true);
            event.clear();
            return;
        }
        let inner = Point::new(pos.x - 1, pos.y - 1);
        let (iw, ih) = design.spec.inner_size();
        if inner.x < 0 || inner.y < 0 || inner.x >= iw || inner.y >= ih {
            design.select(Sel::Dialog);
            event.clear();
            return;
        }
        let sel = design.hit(inner);

        // A click on a tab pane's strip shows that page.
        if let Sel::Top(i) = sel
            && let Some(c) = design.spec.controls.get(i)
            && matches!(c.kind, Kind::Tabs { .. })
            && inner.y - c.y < 3
        {
            let local = Point::new(inner.x - c.x, inner.y - c.y);
            let name = c.name.clone();
            if let Some(pane) = self
                .live
                .as_mut()
                .and_then(|live| live.view_mut::<TabbedPane>(&name))
            {
                let mut click = Event::mouse(EventType::MouseDown, local, MB_LEFT_BUTTON, false);
                pane.handle_event(&mut click);
                let page = pane.active();
                design.show_page(i, page);
            }
        }

        let double = event.mouse.double_click;
        design.select(sel);
        if double {
            drop(design);
            Self::send(event, self.commands.properties);
            return;
        }
        if sel != Sel::Dialog {
            let r = design.rect_of(sel);
            let corner = Point::new(r.b.x - 1, r.b.y - 1);
            design.begin_drag();
            self.drag = Some(if inner == corner && (r.width() > 1 || r.height() > 1) {
                Drag::Resize { sel }
            } else {
                Drag::Move {
                    sel,
                    grab: Point::new(inner.x - r.a.x, inner.y - r.a.y),
                }
            });
            self.set_state_flag(State::DRAGGING, true);
        }
        event.clear();
    }

    fn mouse_move(&mut self, event: &mut Event) {
        let Some(drag) = self.drag else {
            return;
        };
        let pos = event.mouse.pos;
        let inner = Point::new(pos.x - 1, pos.y - 1);
        let mut design = self.design.borrow_mut();
        match drag {
            Drag::Dialog => {
                design.set_dialog_size(pos.x + 1, pos.y + 1);
            }
            Drag::Move { sel, grab } => {
                let o = design.container_origin(sel);
                if let Some(c) = design.control(sel) {
                    let place = (inner.x - grab.x - o.x, inner.y - grab.y - o.y, c.w, c.h);
                    design.set_place(sel, place);
                }
            }
            Drag::Resize { sel } => {
                let o = design.container_origin(sel);
                if let Some(c) = design.control(sel) {
                    let place = (c.x, c.y, inner.x - o.x - c.x + 1, inner.y - o.y - c.y + 1);
                    design.set_place(sel, place);
                }
            }
        }
        drop(design);
        self.sync_bounds();
        event.clear();
    }

    fn mouse_up(&mut self, event: &mut Event) {
        if self.drag.take().is_some() {
            self.design.borrow_mut().end_drag();
            self.set_state_flag(State::DRAGGING, false);
            event.clear();
        }
    }

    fn key(&mut self, event: &mut Event) {
        let mods = event.key_modifiers;
        let sizing = mods.intersects(KeyModifiers::SHIFT | KeyModifiers::CONTROL);
        let mut design = self.design.borrow_mut();
        let step = |dx: i16, dy: i16, design: &mut Design| {
            if sizing {
                design.resize_by(dx, dy);
            } else {
                design.move_by(dx, dy);
            }
        };
        match event.key_code {
            KB_LEFT => step(-1, 0, &mut design),
            KB_RIGHT => step(1, 0, &mut design),
            KB_UP => step(0, -1, &mut design),
            KB_DOWN => step(0, 1, &mut design),
            KB_TAB => design.select_next(true),
            KB_SHIFT_TAB => design.select_next(false),
            KB_ESC => design.select_parent(),
            KB_PGUP | KB_PGDN if mods.contains(KeyModifiers::CONTROL) => {
                let forward = event.key_code == KB_PGDN;
                let sel = design.sel;
                if let Some(tabs) = design.tabs_of(sel)
                    && let Kind::Tabs { pages, active } = &design.spec.controls[tabs].kind
                    && !pages.is_empty()
                {
                    let len = pages.len();
                    let next = if forward {
                        (active + 1) % len
                    } else {
                        (active + len - 1) % len
                    };
                    design.show_page(tabs, next);
                }
            }
            KB_ENTER => {
                drop(design);
                Self::send(event, self.commands.properties);
                return;
            }
            KB_DEL => {
                drop(design);
                Self::send(event, self.commands.delete);
                return;
            }
            _ => return,
        }
        drop(design);
        self.sync_bounds();
        event.clear();
    }

    /// Recolour the cells of `r` (in the surface's space) with `attr`.
    fn paint(terminal: &mut Terminal, r: Rect, attr: impl Fn(Attr) -> Attr) {
        for y in r.a.y..r.b.y {
            for x in r.a.x..r.b.x {
                if let Some(cell) = terminal.read_cell(x, y) {
                    terminal.write_cell(x, y, Cell::new(cell.ch, attr(cell.attr)));
                }
            }
        }
    }

    fn draw_marks(&self, terminal: &mut Terminal) {
        let design = self.design.borrow();
        let selected = |a: Attr| Attr {
            fg: TvColor::Black,
            bg: TvColor::LightCyan,
            ..a
        };
        let handle = |a: Attr| Attr {
            fg: TvColor::Black,
            bg: TvColor::Yellow,
            ..a
        };
        let (w, h) = (self.core.bounds.width(), self.core.bounds.height());
        match design.sel {
            Sel::Dialog => {
                Self::paint(terminal, Rect::new(w - 1, h - 1, w, h), handle);
            }
            sel => {
                let mut r = design.rect_of(sel);
                r.move_by(1, 1);
                Self::paint(terminal, r, selected);
                Self::paint(
                    terminal,
                    Rect::new(r.b.x - 1, r.b.y - 1, r.b.x, r.b.y),
                    handle,
                );
            }
        }
        if self.show_order {
            let number = Attr::new(TvColor::White, TvColor::Magenta);
            for (n, sel) in design.order().into_iter().skip(1).enumerate() {
                let r = design.rect_of(sel);
                for (i, ch) in (n + 1).to_string().chars().enumerate() {
                    let x = r.a.x + 1 + i16::try_from(i).unwrap_or(0);
                    terminal.write_cell(x, r.a.y + 1, Cell::new(ch, number));
                }
            }
        }
    }
}

impl View for Surface {
    fn core(&self) -> &ViewCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut ViewCore {
        &mut self.core
    }

    fn draw(&mut self, terminal: &mut Terminal) {
        self.rebuild();
        if let Some(live) = &mut self.live {
            draw_child(terminal, live.dialog_mut());
        } else if let Some(why) = &self.broken {
            let attr = Attr::new(TvColor::White, TvColor::Red);
            let width = usize::try_from(self.core.bounds.width()).unwrap_or(0);
            let text: String = format!(" {why}").chars().take(width).collect();
            for (x, ch) in text.chars().enumerate() {
                terminal.write_cell(i16::try_from(x).unwrap_or(0), 0, Cell::new(ch, attr));
            }
        }
        self.draw_marks(terminal);
    }

    fn handle_event(&mut self, event: &mut Event) {
        match event.what {
            EventType::MouseDown => self.mouse_down(event),
            EventType::MouseMove | EventType::MouseAuto => self.mouse_move(event),
            EventType::MouseUp => self.mouse_up(event),
            EventType::Keyboard => self.key(event),
            _ => {}
        }
    }

    fn can_focus(&self) -> bool {
        true
    }

    fn update_cursor(&self, terminal: &mut Terminal) {
        let _ = terminal.hide_cursor();
    }

    fn get_palette(&self) -> Option<turbo_vision::core::palette::Palette> {
        None
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
