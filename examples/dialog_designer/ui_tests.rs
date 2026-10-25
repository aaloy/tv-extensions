// (C) 2026 - Antoni Aloy

//! The designer driven as a person would: keys and mouse in, the design
//! and the screen checked.

use super::*;
use turbo_vision::core::event::{
    KB_DEL, KB_DOWN, KB_ENTER, KB_HOME, KB_LEFT, KB_RIGHT, KB_UP, MB_LEFT_BUTTON,
};
use turbo_vision::core::palette::Attr;
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
    command(&mut app, &mut designer, CM_INSERT_MENU);
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
    designer.design.borrow_mut().select(Sel::top(1)); // the name input
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
    assert!(code.contains("// dialog.on_change(\"name\""), "{code}");
    // The stub says which control it is for.
    assert!(code.contains("Runs when \"kind\" changes"), "{code}");
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
    designer.design.borrow_mut().select(Sel::top(3));
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

    designer.design.borrow_mut().select(Sel::top(3));
    designer.refresh(&mut app);
    let row = tree(&mut app, &designer).current().unwrap();
    assert_eq!(row.sel, Sel::top(3));
}

#[test]
fn moving_in_the_tree_selects_in_the_design() {
    let mut app = app();
    let mut designer = open_designer(&mut app, customer());
    designer.refresh(&mut app);
    designer.design.borrow_mut().select(Sel::top(1));
    designer.refresh(&mut app);
    tree(&mut app, &designer).handle_event(&mut Event::keyboard(KB_DOWN));
    designer.refresh(&mut app);
    assert_eq!(designer.design.borrow().sel, Sel::top(2));
    // And the tree stays where the person put it.
    assert_eq!(
        tree(&mut app, &designer).current().unwrap().sel,
        Sel::top(2)
    );
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
    assert!(
        found.iter().any(|e| e.page == Some((Place::top(16), 1))),
        "{found:?}"
    );
    assert!(
        found.iter().any(|e| e
            .sel
            .path()
            .is_some_and(|p| p.depth() == 2 && p.slot() == 1)),
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

#[test]
fn the_insert_menu_has_a_line_between_its_sections() {
    let items = insert_items();
    assert_eq!(items.len(), KINDS.len() + SECTION_ENDS.len());
    // Down skips the lines: thirteen steps from the first item is the tree.
    let mut app = app();
    let mut designer = open_designer(&mut app, Design::default());
    let tx = app.terminal.event_injector();
    for _ in 0..13 {
        tx.send(Event::keyboard(KB_DOWN)).unwrap();
    }
    tx.send(Event::keyboard(KB_ENTER)).unwrap();
    command(&mut app, &mut designer, CM_INSERT_MENU);
    let design = designer.design.borrow();
    assert_eq!(design.selected().unwrap().type_name(), "tree");
}

fn queue(app: &mut Application, keys: &[u16]) {
    let tx = app.terminal.event_injector();
    for &k in keys {
        tx.send(Event::keyboard(k)).unwrap();
    }
}

fn sample_templates() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    templates::save(tmp.path(), "alpha", &DialogSpec::new("First", 40, 10)).unwrap();
    templates::save(tmp.path(), "beta", &DialogSpec::new("Second", 50, 12)).unwrap();
    tmp
}

#[test]
fn the_manager_hands_back_the_template_that_is_picked() {
    let mut app = app();
    let tmp = sample_templates();
    queue(&mut app, &[KB_DOWN, KB_ENTER]);
    let spec = manage_templates(&mut app, tmp.path()).expect("a template was used");
    assert_eq!(spec.title, "Second");
}

#[test]
fn the_manager_closes_without_a_template_on_escape() {
    let mut app = app();
    let tmp = sample_templates();
    queue(&mut app, &[turbo_vision::core::event::KB_ESC]);
    assert!(manage_templates(&mut app, tmp.path()).is_none());
}

#[test]
fn a_dialog_is_saved_as_a_template() {
    let mut app = app();
    let mut designer = open_designer(&mut app, customer());
    let tmp = tempfile::tempdir().unwrap();
    // The name box starts with the dialog's title; accept it.
    queue(&mut app, &[KB_ENTER, KB_ENTER]);
    designer.save_template_in(&mut app, tmp.path());
    let saved = templates::list(tmp.path());
    assert_eq!(saved.len(), 1, "{saved:?}");
    assert_eq!(saved[0].spec.title, "Customer");
    assert_eq!(saved[0].spec, designer.design.borrow().spec);
}

fn group_design() -> Design {
    let spec = DialogSpec::from_json(
        r#"{ "title": "G", "width": 50, "height": 16, "controls": [
            { "type": "group_box", "x": 2, "y": 1, "w": 30, "h": 8, "title": "Box", "controls": [
                { "type": "input", "name": "inside", "x": 1, "y": 1, "w": 10, "h": 1 }
            ] },
            { "type": "input", "name": "outside", "x": 2, "y": 10, "w": 10, "h": 1 }
        ] }"#,
    )
    .unwrap();
    Design::new(spec, None)
}

#[test]
fn the_tree_nests_what_a_group_box_holds() {
    let mut app = app();
    let mut designer = open_designer(&mut app, group_design());
    designer.refresh(&mut app);
    let labels: Vec<String> = tree(&mut app, &designer)
        .entries()
        .iter()
        .map(|e| e.label.clone())
        .collect();
    assert_eq!(
        labels,
        [
            "Dialog \"G\"",
            "group_box \"Box\"",
            "input \"inside\"",
            "input \"outside\""
        ]
    );
}

#[test]
fn dragging_a_row_onto_a_container_reparents_the_control() {
    let mut app = app();
    let mut designer = open_designer(&mut app, group_design());
    designer.refresh(&mut app);
    // Rows by line: 0 dialog, 1 group box, 2 inside, 3 outside.
    let tree_at = |line: i16| Point::new(2, line);
    let mut press = Event::mouse(EventType::MouseDown, tree_at(3), MB_LEFT_BUTTON, false);
    tree(&mut app, &designer).handle_event(&mut press);
    let mut release = Event::mouse(EventType::MouseUp, tree_at(1), MB_LEFT_BUTTON, false);
    tree(&mut app, &designer).handle_event(&mut release);
    assert_eq!(release.what, EventType::Command);
    command(&mut app, &mut designer, release.command);
    let d = designer.design.borrow();
    assert_eq!(d.spec.controls.len(), 1, "it left the dialog's own list");
    let Kind::GroupBox { controls, .. } = &d.spec.controls[0].kind else {
        unreachable!()
    };
    assert_eq!(controls.len(), 2);
    assert_eq!(controls[1].name, "outside");
}

#[test]
fn dropping_on_a_plain_control_changes_nothing() {
    let mut app = app();
    let mut designer = open_designer(&mut app, group_design());
    designer.refresh(&mut app);
    let mut press = Event::mouse(
        EventType::MouseDown,
        Point::new(2, 3),
        MB_LEFT_BUTTON,
        false,
    );
    tree(&mut app, &designer).handle_event(&mut press);
    let mut release = Event::mouse(EventType::MouseUp, Point::new(2, 2), MB_LEFT_BUTTON, false);
    tree(&mut app, &designer).handle_event(&mut release);
    assert_ne!(release.what, EventType::Command);
    assert_eq!(designer.design.borrow().spec.controls.len(), 2);
}

#[test]
fn the_move_menu_lists_the_other_containers() {
    let mut app = app();
    let mut designer = open_designer(&mut app, group_design());
    designer.refresh(&mut app);
    designer.design.borrow_mut().select(Sel::top(1)); // outside
    let targets = designer.design.borrow().move_targets();
    assert_eq!(targets.len(), 1, "{targets:?}");
    assert!(targets[0].1.contains("Box"), "{targets:?}");
    queue(&mut app, &[KB_ENTER]);
    command(&mut app, &mut designer, CM_MOVE_MENU);
    assert_eq!(designer.design.borrow().spec.controls.len(), 1);
    assert_eq!(designer.design.borrow().sel.path().unwrap().depth(), 2);
}

#[test]
fn the_insert_menu_fits_on_screen_so_its_last_item_can_be_clicked() {
    let mut app = app();
    let mut designer = open_designer(&mut app, Design::default());
    designer.refresh(&mut app);
    let rows = i16::try_from(KINDS.len() + SECTION_ENDS.len() + 2).unwrap();
    let at = designer.popup_origin(&app, rows);
    let (_, screen_h) = app.terminal.size();
    assert!(at.y + rows < screen_h, "the menu ends below the screen");
    // The last item, "Error line", by the mouse: pressed and released as a
    // terminal sends them.
    let last = Point::new(at.x + 3, at.y + rows - 2);
    let tx = app.terminal.event_injector();
    tx.send(Event::mouse(
        EventType::MouseDown,
        last,
        MB_LEFT_BUTTON,
        false,
    ))
    .unwrap();
    tx.send(Event::mouse(EventType::MouseUp, last, 0, false))
        .unwrap();
    tx.send(Event::keyboard(turbo_vision::core::event::KB_ESC))
        .unwrap();
    command(&mut app, &mut designer, CM_INSERT_MENU);
    let d = designer.design.borrow();
    assert_eq!(
        d.selected()
            .map(tv_extensions::dialog_spec::Control::type_name),
        Some("error_line")
    );
}

#[test]
fn on_a_short_screen_the_insert_menu_asks_for_the_section_first() {
    set_press_animation(std::time::Duration::ZERO);
    let mut app = Application::with_terminal(test_terminal(110, 24));
    let (width, height) = app.terminal.size();
    app.set_menu_bar(menu_bar(width));
    app.set_status_line(status_line(width, height));
    let mut designer = open_designer(&mut app, Design::default());
    designer.refresh(&mut app);
    // Text and lines, then its third item: the line.
    queue(&mut app, &[KB_ENTER, KB_DOWN, KB_DOWN, KB_ENTER]);
    command(&mut app, &mut designer, CM_INSERT_MENU);
    let d = designer.design.borrow();
    assert_eq!(
        d.selected()
            .map(tv_extensions::dialog_spec::Control::type_name),
        Some("line")
    );
}

fn attr_at(app: &Application, designer: &Designer, inner: Point) -> Attr {
    let p = on_surface(app, designer, inner);
    app.terminal.read_cell(p.x, p.y).unwrap().attr
}

#[test]
fn a_selected_container_is_marked_by_its_frame_only() {
    let mut app = app();
    let mut designer = open_designer(&mut app, group_design());
    designer.design.borrow_mut().select(Sel::top(0));
    designer.refresh(&mut app);
    app.draw();
    let r = designer.design.borrow().rect_of(Sel::top(0));
    let marked = attr_at(&app, &designer, r.a).bg;
    // The frame is marked; a cell inside the box is not.
    assert_eq!(
        attr_at(&app, &designer, Point::new(r.a.x + 5, r.a.y)).bg,
        marked
    );
    assert_eq!(
        attr_at(&app, &designer, Point::new(r.a.x, r.a.y + 3)).bg,
        marked
    );
    assert_ne!(
        attr_at(&app, &designer, Point::new(r.a.x + 15, r.a.y + 4)).bg,
        marked
    );
    // A plain control is marked all over.
    designer.design.borrow_mut().select(Sel::top(1));
    designer.refresh(&mut app);
    app.draw();
    let r = designer.design.borrow().rect_of(Sel::top(1));
    assert_eq!(
        attr_at(&app, &designer, Point::new(r.a.x + 3, r.a.y)).bg,
        marked
    );
}

#[test]
fn backspace_deletes_like_del() {
    use turbo_vision::core::event::{KB_BACKSPACE, KB_DEL, KB_TAB};
    let mut app = app();
    let mut designer = open_designer(&mut app, customer());
    designer.refresh(&mut app);
    for code in [KB_DEL, KB_BACKSPACE] {
        key(&mut app, &mut designer, KB_TAB);
        let before = designer.design.borrow().spec.controls.len();
        key(&mut app, &mut designer, code);
        let after = designer.design.borrow().spec.controls.len();
        assert_eq!(after, before - 1, "{code:#x}");
    }
}

#[test]
fn the_tab_order_dialog_reorders_the_siblings() {
    use turbo_vision::core::event::{KB_ALT_D, KB_ALT_U};
    let mut app = app();
    let mut designer = open_designer(&mut app, customer());
    designer.design.borrow_mut().select(Sel::top(1));
    let names = |d: &Designer| -> Vec<String> {
        d.design
            .borrow()
            .spec
            .controls
            .iter()
            .map(|c| c.name.clone())
            .collect()
    };
    let before = names(&designer);
    // Down twice, up once, then OK: the control ends one place later.
    queue(&mut app, &[KB_ALT_D, KB_ALT_D, KB_ALT_U, KB_ENTER]);
    command(&mut app, &mut designer, CM_SET_ORDER);
    let after = names(&designer);
    assert_eq!(after[2], before[1], "{before:?} -> {after:?}");
    assert_eq!(after[1], before[2]);
    assert_eq!(designer.design.borrow().sel, Sel::top(2));
}

#[test]
fn cancelling_the_tab_order_changes_nothing() {
    use turbo_vision::core::event::{KB_ALT_D, KB_ESC};
    let mut app = app();
    let mut designer = open_designer(&mut app, customer());
    designer.design.borrow_mut().select(Sel::top(1));
    let before = designer.design.borrow().spec.clone();
    queue(&mut app, &[KB_ALT_D, KB_ESC]);
    command(&mut app, &mut designer, CM_SET_ORDER);
    assert_eq!(designer.design.borrow().spec, before);
}

#[test]
fn f1_shows_help_on_the_selected_control() {
    use turbo_vision::core::event::KB_F1;
    let mut app = app();
    let mut designer = open_designer(&mut app, customer());
    designer.design.borrow_mut().select(Sel::top(1)); // the name input
    // The help window is modal: queue the key that closes it, then look at
    // the screen the moment it is up.
    queue(&mut app, &[KB_ENTER]);
    key(&mut app, &mut designer, KB_F1);
}

#[test]
fn help_contents_shows_a_topic_and_comes_back_to_the_list() {
    use turbo_vision::core::event::{KB_DOWN, KB_ESC};
    let mut app = app();
    let mut designer = open_designer(&mut app, customer());
    // View the second topic, close its text, then close the contents.
    queue(&mut app, &[KB_DOWN, KB_ENTER, KB_ENTER, KB_ESC]);
    command(&mut app, &mut designer, CM_HELP_CONTENTS);
}

#[test]
fn the_tutorial_goes_forward_and_back() {
    use turbo_vision::core::event::{KB_ALT_B, KB_ALT_N, KB_ESC};
    let mut app = app();
    let mut designer = open_designer(&mut app, customer());
    // Next, Next, Back, then Close.
    queue(&mut app, &[KB_ALT_N, KB_ALT_N, KB_ALT_B, KB_ESC]);
    command(&mut app, &mut designer, CM_TUTORIAL);
}

/// A design of three inputs, 10 wide: two on the left, one apart.
fn three_inputs() -> Design {
    let mut design = Design::default();
    for place in [(5, 2, 10, 1), (5, 5, 10, 1), (30, 8, 10, 1)] {
        let sel = design.insert("input");
        design.set_place(sel, place);
    }
    design.select(Sel::Dialog);
    design
}

fn places(designer: &Designer) -> Vec<(i16, i16)> {
    let design = designer.design.borrow();
    design.spec.controls.iter().map(|c| (c.x, c.y)).collect()
}

fn ctrl_click(app: &mut Application, designer: &mut Designer, at: Point) {
    use turbo_vision::core::keys::KeyModifiers;
    let mut press = Event::mouse(EventType::MouseDown, at, MB_LEFT_BUTTON, false);
    press.key_modifiers = KeyModifiers::CONTROL;
    step(app, designer, press);
    mouse(app, designer, EventType::MouseUp, at);
}

#[test]
fn a_line_drawn_as_a_box_is_marked_by_its_edge() {
    use turbo_vision::core::palette::TvColor;
    let mut design = Design::default();
    let sel = design.insert("line");
    design.set_place(sel, (2, 1, 10, 4));
    design.select(sel);
    let mut app = app();
    let mut designer = open_designer(&mut app, design);
    designer.refresh(&mut app);
    app.draw();
    let bg = |app: &Application, at: Point| {
        app.terminal
            .read_cell(at.x, at.y)
            .map(|c| c.attr.bg)
            .unwrap()
    };
    let edge = on_surface(&app, &designer, Point::new(2, 2));
    let inside = on_surface(&app, &designer, Point::new(5, 2));
    assert_eq!(bg(&app, edge), TvColor::LightCyan);
    assert_ne!(bg(&app, inside), TvColor::LightCyan);
}

#[test]
fn ctrl_click_selects_several_and_a_drag_moves_them_together() {
    let mut app = app();
    let mut designer = open_designer(&mut app, three_inputs());
    let first = on_surface(&app, &designer, Point::new(6, 2));
    let second = on_surface(&app, &designer, Point::new(6, 5));
    mouse(&mut app, &mut designer, EventType::MouseDown, first);
    mouse(&mut app, &mut designer, EventType::MouseUp, first);
    ctrl_click(&mut app, &mut designer, second);
    assert_eq!(designer.design.borrow().selection().len(), 2);

    // Drag the second: both go 3 right and 1 down, the third stays.
    let to = on_surface(&app, &designer, Point::new(9, 6));
    mouse(&mut app, &mut designer, EventType::MouseDown, second);
    mouse(&mut app, &mut designer, EventType::MouseMove, to);
    mouse(&mut app, &mut designer, EventType::MouseUp, to);
    assert_eq!(places(&designer), [(8, 3), (8, 6), (30, 8)]);
    assert_eq!(designer.design.borrow().selection().len(), 2);
    // One drag, one undo step.
    command(&mut app, &mut designer, CM_UNDO);
    assert_eq!(places(&designer), [(5, 2), (5, 5), (30, 8)]);
}

#[test]
fn ctrl_click_on_a_selected_control_takes_it_out() {
    let mut app = app();
    let mut designer = open_designer(&mut app, three_inputs());
    let first = on_surface(&app, &designer, Point::new(6, 2));
    let second = on_surface(&app, &designer, Point::new(6, 5));
    mouse(&mut app, &mut designer, EventType::MouseDown, first);
    mouse(&mut app, &mut designer, EventType::MouseUp, first);
    ctrl_click(&mut app, &mut designer, second);
    ctrl_click(&mut app, &mut designer, first);
    let design = designer.design.borrow();
    assert_eq!(design.selection(), [Sel::top(1)]);
}

#[test]
fn a_plain_click_on_one_of_several_selects_it_alone() {
    let mut app = app();
    let mut designer = open_designer(&mut app, three_inputs());
    let first = on_surface(&app, &designer, Point::new(6, 2));
    let second = on_surface(&app, &designer, Point::new(6, 5));
    mouse(&mut app, &mut designer, EventType::MouseDown, first);
    mouse(&mut app, &mut designer, EventType::MouseUp, first);
    ctrl_click(&mut app, &mut designer, second);
    mouse(&mut app, &mut designer, EventType::MouseDown, second);
    mouse(&mut app, &mut designer, EventType::MouseUp, second);
    assert_eq!(designer.design.borrow().selection(), [Sel::top(1)]);
}

#[test]
fn a_band_selects_what_it_covers_and_del_deletes_them() {
    use turbo_vision::core::event::KB_DEL;
    let mut app = app();
    let mut designer = open_designer(&mut app, three_inputs());
    // From empty space above the first input to below the second.
    let from = on_surface(&app, &designer, Point::new(3, 1));
    let to = on_surface(&app, &designer, Point::new(20, 6));
    mouse(&mut app, &mut designer, EventType::MouseDown, from);
    mouse(&mut app, &mut designer, EventType::MouseMove, to);
    mouse(&mut app, &mut designer, EventType::MouseUp, to);
    assert_eq!(
        designer.design.borrow().selection(),
        [Sel::top(0), Sel::top(1)]
    );
    key(&mut app, &mut designer, KB_RIGHT);
    assert_eq!(places(&designer), [(6, 2), (6, 5), (30, 8)]);

    key(&mut app, &mut designer, KB_DEL);
    assert_eq!(places(&designer), [(30, 8)]);
    command(&mut app, &mut designer, CM_UNDO);
    assert_eq!(places(&designer).len(), 3);
}

#[test]
fn a_band_over_nothing_selects_the_dialog() {
    let mut app = app();
    let mut designer = open_designer(&mut app, three_inputs());
    designer.design.borrow_mut().select(Sel::top(0));
    let from = on_surface(&app, &designer, Point::new(20, 1));
    let to = on_surface(&app, &designer, Point::new(25, 3));
    mouse(&mut app, &mut designer, EventType::MouseDown, from);
    mouse(&mut app, &mut designer, EventType::MouseMove, to);
    mouse(&mut app, &mut designer, EventType::MouseUp, to);
    assert_eq!(designer.design.borrow().selection(), [Sel::Dialog]);
}

#[test]
fn controls_moved_together_stop_at_the_edge_together() {
    let mut design = three_inputs();
    design.select(Sel::top(0));
    design.toggle(Sel::top(1));
    // The first is 5 from the left edge: a move of 8 goes only 5.
    assert!(design.move_by(-8, 0));
    let xs: Vec<i16> = design.spec.controls.iter().map(|c| c.x).collect();
    assert_eq!(xs, [0, 0, 30]);
    assert!(!design.move_by(-1, 0));
}

/// [`three_inputs`] with all three selected.
fn all_three() -> Design {
    let mut design = three_inputs();
    design.select(Sel::top(0));
    design.toggle(Sel::top(1));
    design.toggle(Sel::top(2));
    design
}

fn xs(design: &Design) -> Vec<i16> {
    design.spec.controls.iter().map(|c| c.x).collect()
}

fn ys(design: &Design) -> Vec<i16> {
    design.spec.controls.iter().map(|c| c.y).collect()
}

#[test]
fn align_lines_the_selection_up_on_the_box_around_it() {
    // The box runs from 5 to 40 across and from 2 to 9 down.
    let mut design = all_three();
    assert!(design.align(Arrange::Right));
    assert_eq!(xs(&design), [30, 30, 30]);
    assert!(
        !design.align(Arrange::Left),
        "the box is where they are now"
    );

    let mut design = all_three();
    assert!(design.align(Arrange::Left));
    assert_eq!(xs(&design), [5, 5, 5]);
    let mut design = all_three();
    assert!(design.align(Arrange::Center));
    assert_eq!(xs(&design), [17, 17, 17]);
    let mut design = all_three();
    assert!(design.align(Arrange::Top));
    assert_eq!(ys(&design), [2, 2, 2]);
    let mut design = all_three();
    assert!(design.align(Arrange::Bottom));
    assert_eq!(ys(&design), [8, 8, 8]);
    let mut design = all_three();
    assert!(design.align(Arrange::Middle));
    assert_eq!(ys(&design), [5, 5, 5]);
}

#[test]
fn one_control_alone_aligns_to_its_container() {
    // The default dialog is 50 x 14: 48 x 12 inside its frame.
    let mut design = three_inputs();
    design.select(Sel::top(0));
    assert!(design.align(Arrange::Center));
    assert!(design.align(Arrange::Middle));
    let c = &design.spec.controls[0];
    assert_eq!((c.x, c.y), (19, 5));
    assert!(design.align(Arrange::Right));
    assert!(design.align(Arrange::Bottom));
    let c = &design.spec.controls[0];
    assert_eq!((c.x, c.y), (38, 11));
    assert!(!design.align(Arrange::Right), "already there");
}

#[test]
fn same_width_and_height_follow_the_first_selected() {
    let mut design = three_inputs();
    design.set_place(Sel::top(1), (5, 5, 16, 2));
    design.select(Sel::top(1));
    design.toggle(Sel::top(0));
    design.toggle(Sel::top(2));
    assert!(design.same_size(true));
    let sizes: Vec<(i16, i16)> = design.spec.controls.iter().map(|c| (c.w, c.h)).collect();
    assert_eq!(sizes, [(16, 1), (16, 2), (16, 1)]);
    assert!(design.same_size(false));
    let heights: Vec<i16> = design.spec.controls.iter().map(|c| c.h).collect();
    assert_eq!(heights, [2, 2, 2]);
}

#[test]
fn distribute_makes_the_gaps_equal_and_keeps_the_outermost() {
    let mut design = three_inputs();
    design.set_place(Sel::top(1), (12, 3, 10, 1));
    let mut design = {
        design.select(Sel::top(0));
        design.toggle(Sel::top(1));
        design.toggle(Sel::top(2));
        design
    };
    // Down: rows 2, 3 and 8; 6 rows between them, 3 a gap.
    assert!(design.distribute(false));
    assert_eq!(ys(&design), [2, 5, 8]);
    // Across: 5 to 40 holds 30 columns of input and 5 of gap: 3 then 2.
    assert!(design.distribute(true));
    assert_eq!(xs(&design), [5, 18, 30]);
    assert!(!design.distribute(true), "already even");
}

#[test]
fn same_size_needs_two_and_the_dialog_is_not_arranged() {
    let mut design = three_inputs();
    design.set_place(Sel::top(1), (5, 5, 16, 1));
    design.select(Sel::top(0));
    assert!(!design.same_size(true));
    design.toggle(Sel::top(1));
    assert!(design.same_size(true));
    design.select(Sel::Dialog);
    assert!(!design.distribute(true));
    assert!(!design.align(Arrange::Left));
}

#[test]
fn the_arrange_menu_aligns_and_undoes_in_one_step() {
    let mut app = app();
    let mut designer = open_designer(&mut app, all_three());
    command(&mut app, &mut designer, CM_ALIGN + 1); // right edges
    assert_eq!(xs(&designer.design.borrow()), [30, 30, 30]);
    command(&mut app, &mut designer, CM_UNDO);
    assert_eq!(xs(&designer.design.borrow()), [5, 5, 30]);
}

/// A group box and a tab pane, each holding two inputs, 10 wide.
fn containers_design() -> Design {
    let spec = DialogSpec::from_json(
        r#"{ "title": "C", "width": 70, "height": 18, "controls": [
            { "type": "group_box", "x": 1, "y": 1, "w": 30, "h": 10, "title": "Box", "controls": [
                { "type": "input", "name": "a", "x": 2, "y": 1, "w": 10, "h": 1 },
                { "type": "input", "name": "b", "x": 2, "y": 4, "w": 10, "h": 1 }
            ] },
            { "type": "tabs", "x": 33, "y": 1, "w": 34, "h": 12, "pages": [
                { "title": "~O~ne", "controls": [
                    { "type": "input", "name": "c", "x": 2, "y": 1, "w": 10, "h": 1 },
                    { "type": "input", "name": "d", "x": 2, "y": 4, "w": 10, "h": 1 }
                ] }
            ] }
        ] }"#,
    )
    .unwrap();
    Design::new(spec, None)
}

/// Where control `name` is, inside the dialog's frame.
fn rect_named(designer: &Designer, name: &str) -> Rect {
    let design = designer.design.borrow();
    let sel = design
        .order()
        .into_iter()
        .find(|&s| design.control(s).is_some_and(|c| c.name == name))
        .unwrap();
    design.rect_of(sel)
}

fn names_selected(designer: &Designer) -> Vec<String> {
    let design = designer.design.borrow();
    design
        .selection()
        .into_iter()
        .filter_map(|s| design.control(s).map(|c| c.name.clone()))
        .collect()
}

#[test]
fn ctrl_click_selects_several_in_a_group_box_and_on_a_tab_page() {
    for (one, two) in [("a", "b"), ("c", "d")] {
        let mut app = app();
        let mut designer = open_designer(&mut app, containers_design());
        let first = rect_named(&designer, one).a;
        let second = rect_named(&designer, two).a;
        let first = on_surface(&app, &designer, Point::new(first.x + 1, first.y));
        let second = on_surface(&app, &designer, Point::new(second.x + 1, second.y));
        mouse(&mut app, &mut designer, EventType::MouseDown, first);
        mouse(&mut app, &mut designer, EventType::MouseUp, first);
        ctrl_click(&mut app, &mut designer, second);
        assert_eq!(names_selected(&designer), [one, two]);
    }
}

#[test]
fn a_band_selects_several_in_a_group_box_and_on_a_tab_page() {
    for (one, two) in [("a", "b"), ("c", "d")] {
        let mut app = app();
        let mut designer = open_designer(&mut app, containers_design());
        let first = rect_named(&designer, one);
        let second = rect_named(&designer, two);
        // From the empty space left of and above the first, inside the
        // container, to past the second.
        let from = on_surface(&app, &designer, Point::new(first.a.x - 1, first.a.y - 1));
        let to = on_surface(&app, &designer, Point::new(second.b.x + 1, second.b.y));
        mouse(&mut app, &mut designer, EventType::MouseDown, from);
        mouse(&mut app, &mut designer, EventType::MouseMove, to);
        mouse(&mut app, &mut designer, EventType::MouseUp, to);
        assert_eq!(names_selected(&designer), [one, two]);
    }
}

#[test]
fn one_or_two_controls_distribute_over_their_container() {
    // Inside the default dialog's frame: 48 x 12.
    let mut design = three_inputs();
    design.select(Sel::top(0));
    design.toggle(Sel::top(1));
    // 28 columns to share out, a gap before, between and after: 10, 9, 9.
    assert!(design.distribute(true));
    assert_eq!(xs(&design)[..2], [10, 29]);
    // Down: 10 rows, 4, 3, 3.
    assert!(design.distribute(false));
    assert_eq!(ys(&design)[..2], [4, 8]);
    assert!(!design.distribute(false), "already even");

    // One alone: centred.
    design.select(Sel::top(2));
    assert!(design.distribute(true));
    assert_eq!(xs(&design)[2], 19);
}

#[test]
fn save_as_asks_for_the_file_in_a_file_dialog() {
    let tmp = tempfile::tempdir().unwrap();
    let first = tmp.path().join("first.json");
    let mut design = Design::default();
    design.saved(first.clone());
    let mut app = app();
    let mut designer = open_designer(&mut app, design);
    // The dialog starts in the folder of the file; type a name, no extension.
    queue_key(&mut app, KB_HOME);
    for _ in 0..10 {
        queue_key(&mut app, KB_DEL);
    }
    queue_text(&mut app, "second");
    queue_key(&mut app, KB_ENTER);
    command(&mut app, &mut designer, CM_SAVE_AS);
    let second = tmp.path().join("second.json");
    assert!(
        second.exists(),
        "{:?}",
        std::fs::read_dir(tmp.path()).unwrap().collect::<Vec<_>>()
    );
    assert_eq!(
        designer.design.borrow().path.as_deref(),
        Some(second.as_path())
    );
}
