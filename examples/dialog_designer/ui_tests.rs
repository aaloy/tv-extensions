// (C) 2026 - Antoni Aloy

//! The designer driven as a person would: keys and mouse in, the design
//! and the screen checked.

use super::*;
use turbo_vision::core::event::{KB_DOWN, KB_ENTER, KB_INS, KB_LEFT, KB_RIGHT, KB_UP, MB_LEFT_BUTTON};
use turbo_vision::test_util::test_terminal;
use turbo_vision::views::button::set_press_animation;

const W: u16 = 110;
const H: u16 = 32;

fn app() -> Application {
    set_press_animation(std::time::Duration::ZERO);
    let mut app = Application::with_terminal(test_terminal(W, H));
    let (width, height) = app.terminal.size();
    app.set_menu_bar(menu_bar(width));
    app.set_status_line(status_line(width, height));
    app
}

fn screen(app: &Application) -> String {
    let (w, h) = app.terminal.size();
    let mut out = String::new();
    for y in 0..h {
        for x in 0..w {
            let ch = app.terminal.read_cell(x, y).map_or(' ', |c| c.ch);
            out.push(ch);
        }
        out.push('\n');
    }
    out
}

/// One event through the application, then a redraw.
fn step(app: &mut Application, designer: &mut Designer, event: Event) {
    app.step(designer, Some(event));
    designer.refresh(app);
    app.draw();
}

fn key(app: &mut Application, designer: &mut Designer, code: u16) {
    step(app, designer, Event::keyboard(code));
}

fn command(app: &mut Application, designer: &mut Designer, command: CommandId) {
    step(app, designer, Event::command(command));
}

/// The screen position of the cell inside the surface's frame at `inner`.
fn on_surface(app: &Application, designer: &Designer, inner: Point) -> Point {
    let corner = app.desktop.get(designer.surface).unwrap().bounds().a;
    let desk = app.desktop.bounds().a;
    Point::new(
        desk.x + corner.x + 1 + inner.x,
        desk.y + corner.y + 1 + inner.y,
    )
}

fn mouse(app: &mut Application, designer: &mut Designer, what: EventType, at: Point) {
    step(app, designer, Event::mouse(what, at, MB_LEFT_BUTTON, false));
}

fn customer() -> Design {
    let spec = DialogSpec::from_json(include_str!("../dialog_specs/customer.json")).unwrap();
    Design::new(spec, None)
}

#[test]
fn the_design_is_drawn_by_its_real_controls() {
    let mut app = app();
    let mut designer = open_designer(&mut app, customer());
    designer.refresh(&mut app);
    app.draw();
    let text = screen(&app);
    println!("{text}");
    assert!(text.contains("Customer"), "{text}");
    assert!(text.contains("Name"), "{text}");
    assert!(text.contains("Inspector"), "{text}");
}

#[test]
fn ins_opens_the_insert_menu_and_picks_a_control() {
    let mut app = app();
    let mut designer = open_designer(&mut app, Design::default());
    // The popup menu runs its own loop: queue its keys first.
    let tx = app.terminal.event_injector();
    for _ in 0..3 {
        tx.send(Event::keyboard(KB_DOWN)).unwrap();
    }
    tx.send(Event::keyboard(KB_ENTER)).unwrap();
    key(&mut app, &mut designer, KB_INS);
    let design = designer.design.borrow();
    assert_eq!(design.selected().unwrap().type_name(), "input");
}

#[test]
fn arrows_move_the_selected_control() {
    let mut app = app();
    let mut designer = open_designer(&mut app, Design::default());
    command(&mut app, &mut designer, CM_INSERT + 3); // input
    key(&mut app, &mut designer, KB_RIGHT);
    key(&mut app, &mut designer, KB_RIGHT);
    key(&mut app, &mut designer, KB_DOWN);
    let c = designer.design.borrow().selected().cloned().unwrap();
    assert_eq!((c.x, c.y), (4, 2));
    // Undo is one step per key.
    command(&mut app, &mut designer, CM_UNDO);
    let c = designer.design.borrow().selected().cloned().unwrap();
    assert_eq!((c.x, c.y), (4, 1));
}

#[test]
fn dragging_moves_a_control_and_its_corner_resizes_it() {
    let mut app = app();
    let mut designer = open_designer(&mut app, Design::default());
    command(&mut app, &mut designer, CM_INSERT + 3); // input at (2, 1), 20 wide
    let grab = on_surface(&app, &designer, Point::new(5, 1));
    let drop_at = on_surface(&app, &designer, Point::new(9, 4));
    mouse(&mut app, &mut designer, EventType::MouseDown, grab);
    mouse(&mut app, &mut designer, EventType::MouseMove, drop_at);
    mouse(&mut app, &mut designer, EventType::MouseUp, drop_at);
    let c = designer.design.borrow().selected().cloned().unwrap();
    assert_eq!((c.x, c.y), (6, 4));

    let corner = on_surface(&app, &designer, Point::new(6 + 19, 4));
    let wider = on_surface(&app, &designer, Point::new(6 + 29, 4));
    mouse(&mut app, &mut designer, EventType::MouseDown, corner);
    mouse(&mut app, &mut designer, EventType::MouseMove, wider);
    mouse(&mut app, &mut designer, EventType::MouseUp, wider);
    let c = designer.design.borrow().selected().cloned().unwrap();
    assert_eq!(c.w, 30);
}

#[test]
fn clicking_selects_and_clicking_nothing_selects_the_dialog() {
    let mut app = app();
    let mut designer = open_designer(&mut app, customer());
    let name = designer.design.borrow().spec.find("name").unwrap().clone();
    let at = on_surface(&app, &designer, Point::new(name.x + 1, name.y));
    mouse(&mut app, &mut designer, EventType::MouseDown, at);
    mouse(&mut app, &mut designer, EventType::MouseUp, at);
    assert_eq!(designer.design.borrow().selected().unwrap().name, "name");
}

/// Queue `text` as typed characters.
fn queue_text(app: &mut Application, text: &str) {
    let tx = app.terminal.event_injector();
    for c in text.chars() {
        tx.send(Event::text(c)).unwrap();
    }
}

fn queue_key(app: &mut Application, code: u16) {
    app.terminal
        .event_injector()
        .send(Event::keyboard(code))
        .unwrap();
}

#[test]
fn enter_opens_the_property_sheet_and_ok_applies_it() {
    use turbo_vision::core::event::KB_ALT_O;
    let mut app = app();
    let mut designer = open_designer(&mut app, customer());
    designer.design.borrow_mut().select(Sel::Top(1)); // the name input
    // The sheet opens on its General page, on "Starting text".
    queue_text(&mut app, "Ada");
    queue_key(&mut app, KB_ALT_O);
    key(&mut app, &mut designer, KB_ENTER);
    let design = designer.design.borrow();
    match &design.selected().unwrap().kind {
        Kind::Input { text, .. } => assert_eq!(text, "Ada"),
        other => panic!("{other:?}"),
    }
    assert!(design.dirty);
}

#[test]
fn f9_runs_the_dialog_and_reports_its_answer() {
    use turbo_vision::core::event::{KB_ALT_O, KB_F9};
    let mut app = app();
    let mut designer = open_designer(&mut app, customer());
    // The dialog under test: a name, then OK. Then OK on the report.
    queue_text(&mut app, "Ada");
    queue_key(&mut app, KB_ALT_O);
    queue_key(&mut app, KB_ENTER);
    key(&mut app, &mut designer, KB_F9);
    // Testing changes nothing in the design.
    assert!(!designer.design.borrow().dirty);
}

#[test]
fn exit_asks_before_dropping_changes() {
    use turbo_vision::core::event::{KB_ALT_N, KB_ESC};
    let mut app = app();
    let mut designer = open_designer(&mut app, Design::default());
    command(&mut app, &mut designer, CM_INSERT); // a label: unsaved now
    app.running = true;
    // Esc on the question: cancel, keep running.
    queue_key(&mut app, KB_ESC);
    key(&mut app, &mut designer, KB_ALT_X);
    assert!(app.running);
    // No: drop the changes and leave.
    queue_key(&mut app, KB_ALT_N);
    key(&mut app, &mut designer, KB_ALT_X);
    assert!(!app.running);
}

#[test]
fn the_code_skeleton_binds_what_the_dialog_names() {
    let design = customer();
    let code = skeleton(&design.spec, Some(Path::new("customer.json")));
    assert!(
        code.contains("JsonDialog::load(\"customer.json\")"),
        "{code}"
    );
    assert!(code.contains("dialog.on(\"lookup\""), "{code}");
    assert!(code.contains("dialog.on(\"kind_changed\""), "{code}");
    assert!(code.contains("fn customer_dialog("), "{code}");
}

#[test]
#[ignore = "writes the skeleton for a compile check: cargo test ... -- --ignored"]
fn write_skeleton_for_compile_check() {
    let design = customer();
    let code = skeleton(
        &design.spec,
        Some(Path::new("examples/dialog_specs/customer.json")),
    );
    std::fs::write(
        std::env::var("SKELETON_OUT").unwrap(),
        format!("{code}\nfn main() {{\n    let _ = customer_dialog;\n}}\n"),
    )
    .unwrap();
}

#[test]
fn tab_pages_and_the_extras_are_drawn_with_the_tab_order() {
    let mut app = app();
    let spec = DialogSpec::from_json(include_str!("../dialog_specs/everything.json")).unwrap();
    let mut designer = open_designer(&mut app, Design::new(spec, None));
    command(&mut app, &mut designer, CM_TAB_ORDER);
    let text = screen(&app);
    for expected in ["One", "Two", "Chosen", "Filter:", "Qty", "40%", " 21"] {
        assert!(text.contains(expected), "{expected} missing:\n{text}");
    }
}

#[test]
#[ignore = "prints the screen for the docs"]
fn print_for_docs() {
    set_press_animation(std::time::Duration::ZERO);
    let mut app = Application::with_terminal(test_terminal(90, 22));
    let (width, height) = app.terminal.size();
    app.set_menu_bar(menu_bar(width));
    app.set_status_line(status_line(width, height));
    let mut designer = open_designer(&mut app, customer());
    designer.design.borrow_mut().select(Sel::Top(3));
    designer.refresh(&mut app);
    app.draw();
    println!("{}", screen(&app));
}

/// The tree of controls in the inspector.
fn tree<'a>(app: &'a mut Application, designer: &Designer) -> &'a mut ControlTree {
    let inspector = designer.inspector.expect("the inspector is open");
    app.desktop
        .get_mut(inspector.window)
        .and_then(|w| w.get_mut(inspector.tree))
        .expect("the tree is in the inspector")
}

#[test]
fn the_tree_lists_the_controls_and_follows_the_selection() {
    let mut app = app();
    let mut designer = open_designer(&mut app, customer());
    designer.refresh(&mut app);
    app.draw();
    let text = screen(&app);
    assert!(text.contains("input \"name\""), "{text}");
    assert!(text.contains("button \"Cancel\""), "{text}");

    designer.design.borrow_mut().select(Sel::Top(3));
    designer.refresh(&mut app);
    let row = tree(&mut app, &designer).current().unwrap();
    assert_eq!(row.sel, Sel::Top(3));
}

#[test]
fn moving_in_the_tree_selects_in_the_design() {
    let mut app = app();
    let mut designer = open_designer(&mut app, customer());
    designer.refresh(&mut app);
    designer.design.borrow_mut().select(Sel::Top(1));
    designer.refresh(&mut app);
    tree(&mut app, &designer).handle_event(&mut Event::keyboard(KB_DOWN));
    designer.refresh(&mut app);
    assert_eq!(designer.design.borrow().sel, Sel::Top(2));
    // And the tree stays where the person put it.
    assert_eq!(tree(&mut app, &designer).current().unwrap().sel, Sel::Top(2));
}

#[test]
fn enter_in_the_tree_asks_for_the_properties() {
    let mut app = app();
    let mut designer = open_designer(&mut app, customer());
    designer.refresh(&mut app);
    let mut event = Event::keyboard(KB_ENTER);
    tree(&mut app, &designer).handle_event(&mut event);
    assert_eq!(event.what, EventType::Command);
    assert_eq!(event.command, CM_PROPERTIES);
}

#[test]
fn tab_pages_are_rows_with_their_controls() {
    let spec = DialogSpec::from_json(include_str!("../dialog_specs/everything.json")).unwrap();
    let mut app = app();
    let mut designer = open_designer(&mut app, Design::new(spec, None));
    designer.refresh(&mut app);
    let found = tree(&mut app, &designer).entries().to_vec();
    assert!(found.iter().any(|e| e.page == Some((16, 1))), "{found:?}");
    assert!(
        found
            .iter()
            .any(|e| matches!(e.sel, Sel::Page { page: 1, .. })),
        "{found:?}"
    );
}

fn surface_corner(app: &Application, designer: &Designer) -> Point {
    app.desktop.get(designer.surface).unwrap().bounds().a
}

#[test]
fn dragging_the_frame_moves_the_whole_surface() {
    let mut app = app();
    let mut designer = open_designer(&mut app, Design::default());
    designer.refresh(&mut app);
    let start = surface_corner(&app, &designer);
    let grab = on_surface(&app, &designer, Point::new(10, -1));
    mouse(&mut app, &mut designer, EventType::MouseDown, grab);
    // Several moves: the corner moves under the mouse, and must not drift.
    for (dx, dy) in [(3, 1), (5, 2), (8, 3)] {
        let to = Point::new(grab.x + dx, grab.y + dy);
        mouse(&mut app, &mut designer, EventType::MouseMove, to);
    }
    mouse(
        &mut app,
        &mut designer,
        EventType::MouseUp,
        Point::new(grab.x + 8, grab.y + 3),
    );
    let now = surface_corner(&app, &designer);
    assert_eq!((now.x - start.x, now.y - start.y), (8, 3));
    // Moving the surface is not an edit.
    assert!(!designer.design.borrow().dirty);
    assert_eq!(designer.design.borrow().sel, Sel::Dialog);
}

#[test]
fn the_surface_stays_on_the_desktop() {
    let mut app = app();
    let mut designer = open_designer(&mut app, Design::default());
    designer.refresh(&mut app);
    let desk = app.desktop.get_bounds();
    for _ in 0..300 {
        key(&mut app, &mut designer, KB_LEFT);
        key(&mut app, &mut designer, KB_UP);
    }
    let a = surface_corner(&app, &designer);
    assert_eq!((a.x, a.y), (0, 0));
    for _ in 0..300 {
        key(&mut app, &mut designer, KB_RIGHT);
        key(&mut app, &mut designer, KB_DOWN);
    }
    let b = app.desktop.get(designer.surface).unwrap().bounds();
    assert_eq!((b.b.x, b.b.y), (desk.width(), desk.height()));
}
