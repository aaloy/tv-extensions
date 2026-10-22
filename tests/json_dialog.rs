// (C) 2026 - Antoni Aloy

//! Designed dialogs at run time: built from a spec, filled and read as JSON,
//! validated, and driven by a person's keystrokes.

use serde_json::{Value, json};
use std::cell::RefCell;
use std::rc::Rc;
use turbo_vision::app::Application;
use turbo_vision::core::event::{Event, KB_DOWN, KB_ESC, KB_TAB};
use turbo_vision::core::event::{KB_ALT_L, KB_ALT_O, KB_ALT_S};
use turbo_vision::test_util::test_terminal;
use turbo_vision::views::View;
use turbo_vision::views::button::{Button, set_press_animation};
use turbo_vision::views::input_line::InputLine;
use turbo_vision::views::label::Label;
use turbo_vision::views::validator::FilterValidator;
use tv_extensions::dialog_spec::{DialogSpec, Flow, JsonDialog};

const EVERYTHING: &str = include_str!("../examples/dialog_specs/everything.json");

/// A customer dialog: a required name, an email with a rule from code, a
/// kind with an on-change action, a lookup button and OK/Cancel.
const CUSTOMER: &str = r#"{
  "title": "Customer",
  "width": 50,
  "height": 14,
  "controls": [
    { "type": "label", "x": 2, "y": 1, "w": 8, "h": 1, "text": "~N~ame", "link": "name" },
    { "type": "input", "name": "name", "x": 11, "y": 1, "w": 30, "h": 1, "required": true },
    { "type": "label", "x": 2, "y": 3, "w": 8, "h": 1, "text": "~E~mail", "link": "email" },
    { "type": "input", "name": "email", "x": 11, "y": 3, "w": 30, "h": 1 },
    { "type": "radio_buttons", "name": "kind", "x": 11, "y": 5, "w": 20, "h": 2,
      "items": ["Retail", "Wholesale"], "on_change": "kind_changed" },
    { "type": "error_line", "x": 2, "y": 8, "w": 44, "h": 1 },
    { "type": "button", "x": 2, "y": 9, "w": 12, "h": 2, "text": "~L~ookup", "command": "lookup" },
    { "type": "button", "x": 15, "y": 9, "w": 10, "h": 2, "text": "~S~ave", "command": "save" },
    { "type": "button", "x": 26, "y": 9, "w": 10, "h": 2, "text": "~O~K", "command": "ok", "default": true },
    { "type": "button", "x": 37, "y": 9, "w": 10, "h": 2, "text": "Cancel", "command": "cancel" }
  ]
}"#;

fn customer() -> JsonDialog {
    JsonDialog::from_json(CUSTOMER).unwrap()
}

fn app() -> Application {
    set_press_animation(std::time::Duration::ZERO);
    Application::with_terminal(test_terminal(100, 40))
}

/// Queue `text` as typed characters, then the special keys.
fn type_text(app: &mut Application, text: &str) {
    let tx = app.terminal.event_injector();
    for c in text.chars() {
        tx.send(Event::text(c)).unwrap();
    }
}

fn key(app: &mut Application, code: u16) {
    app.terminal
        .event_injector()
        .send(Event::keyboard(code))
        .unwrap();
}

#[test]
fn every_kind_builds_and_reports_its_starting_value() {
    let mut dialog = JsonDialog::from_json(EVERYTHING).unwrap();
    assert_eq!(
        dialog.data(),
        json!({
            "name": "",
            "notes": "a\nb",
            "vip": true,
            "extras": [1],
            "kind": 1,
            "country": 0,
            "tags": [],
            "age": 30,
            "volume": 5,
            "progress": 40,
            "orders": 0,
            "city": "",
            "outline": "pear",
            "fruit": "",
            "team": ["bob"]
        })
    );
}

#[test]
fn data_goes_in_and_comes_back_out() {
    let mut dialog = JsonDialog::from_json(EVERYTHING).unwrap();
    let data = json!({
        "name": "ada",
        "notes": "x",
        "vip": false,
        "extras": [0, 1],
        "kind": 0,
        "country": null,
        "tags": [1],
        "age": 99,
        "volume": 2,
        "progress": 100,
        "orders": 0,
        "city": "London",
        "outline": "pear",
        "fruit": "Apple",
        "team": ["ann", "bob"]
    });
    dialog.set_data(&data).unwrap();
    assert_eq!(dialog.data(), data);
}

#[test]
fn setting_data_refuses_unknown_names_and_wrong_types() {
    let mut dialog = customer();
    let err = dialog.set_data(&json!({ "nmae": "x" })).unwrap_err();
    assert!(err.to_string().contains("nmae"), "{err}");
    let err = dialog.set_value("kind", &json!("Retail")).unwrap_err();
    assert!(err.to_string().contains("kind"), "{err}");
}

#[test]
fn controls_are_reached_by_name_and_type() {
    let mut dialog = JsonDialog::from_json(EVERYTHING).unwrap();
    dialog
        .view_mut::<InputLine>("name")
        .unwrap()
        .set_text("typed");
    assert_eq!(dialog.value("name"), Some(json!("typed")));
    // On a tab page too.
    dialog
        .view_mut::<InputLine>("city")
        .unwrap()
        .set_text("Rome");
    assert_eq!(dialog.value("city"), Some(json!("Rome")));
    // Controls with no value too, once named.
    assert!(
        dialog
            .view_mut::<turbo_vision::views::tabbed_pane::TabbedPane>("pages")
            .is_some()
    );
    // The wrong type is simply not found.
    assert!(dialog.view_mut::<Button>("name").is_none());
    assert!(dialog.view_mut::<InputLine>("nobody").is_none());
}

#[test]
fn ok_returns_what_the_person_typed() {
    let mut app = app();
    let mut dialog = customer();
    type_text(&mut app, "Ada");
    key(&mut app, KB_ALT_O);
    let answer = dialog.run(&mut app).expect("not cancelled");
    assert_eq!(answer.action, "ok");
    assert_eq!(answer.data["name"], json!("Ada"));
    assert_eq!(answer.data["kind"], json!(0));
}

#[test]
fn escape_cancels() {
    let mut app = app();
    let mut dialog = customer();
    key(&mut app, KB_ESC);
    assert!(dialog.run(&mut app).is_none());
}

#[test]
fn a_required_field_keeps_the_dialog_open_and_says_why() {
    let mut app = app();
    let mut dialog = customer();
    // OK with no name: refused. Then type the name and OK again.
    key(&mut app, KB_ALT_O);
    type_text(&mut app, "Ada");
    key(&mut app, KB_ALT_O);
    let answer = dialog.run(&mut app).expect("closed by the second OK");
    assert_eq!(answer.data["name"], json!("Ada"));

    // The same refusal, looked at without running the dialog.
    let mut dialog = customer();
    let errors = dialog.validate().unwrap_err();
    assert_eq!(errors.field("name"), Some("Name is required"));
    dialog.show_errors(&errors);
    assert_eq!(dialog.error_text(), "Name is required");
    assert!(dialog.label_of("name").is_some_and(Label::is_error));
}

#[test]
fn rules_from_code_check_a_field_and_the_whole_dialog() {
    let mut dialog = customer();
    dialog.rule("email", |v| match v.as_str() {
        Some(e) if !e.is_empty() && !e.contains('@') => Err("Email must contain @".into()),
        _ => Ok(()),
    });
    dialog.validate_data(|data, errors| {
        if data["kind"] == json!(1) && data["email"] == json!("") {
            errors.add("email", "Wholesale customers need an email");
        }
    });
    dialog
        .set_data(&json!({ "name": "Ada", "email": "nope" }))
        .unwrap();
    let errors = dialog.validate().unwrap_err();
    assert_eq!(errors.field("email"), Some("Email must contain @"));

    dialog.set_data(&json!({ "email": "", "kind": 1 })).unwrap();
    let errors = dialog.validate().unwrap_err();
    assert_eq!(
        errors.field("email"),
        Some("Wholesale customers need an email")
    );

    dialog
        .set_value("email", &json!("ada@example.org"))
        .unwrap();
    assert!(dialog.validate().is_ok());
}

#[test]
fn an_action_can_fill_fields_and_keep_the_dialog_open() {
    let mut app = app();
    let mut dialog = customer();
    dialog.on("lookup", |ctx| {
        ctx.set_value("name", &json!("Grace")).unwrap();
        ctx.set_value("email", &json!("grace@navy.mil")).unwrap();
        Flow::Stay
    });
    key(&mut app, KB_ALT_L);
    key(&mut app, KB_ALT_O);
    let answer = dialog.run(&mut app).unwrap();
    assert_eq!(answer.action, "ok");
    assert_eq!(answer.data["name"], json!("Grace"));
    assert_eq!(answer.data["email"], json!("grace@navy.mil"));
}

#[test]
fn an_unbound_action_closes_like_ok_with_its_own_name() {
    let mut app = app();
    let mut dialog = customer();
    // Save validates first: refused while the name is empty.
    key(&mut app, KB_ALT_S);
    type_text(&mut app, "Ada");
    key(&mut app, KB_ALT_S);
    let answer = dialog.run(&mut app).unwrap();
    assert_eq!(answer.action, "save");
    assert_eq!(answer.data["name"], json!("Ada"));
}

#[test]
fn an_action_can_close_without_validating() {
    let mut app = app();
    let mut dialog = customer();
    dialog.on("lookup", |_| Flow::Close);
    key(&mut app, KB_ALT_L);
    let answer = dialog.run(&mut app).unwrap();
    assert_eq!(answer.action, "lookup");
    assert_eq!(answer.data["name"], json!(""));
}

#[test]
fn an_action_can_submit() {
    let mut app = app();
    let mut dialog = customer();
    let seen = Rc::new(RefCell::new(Vec::<Value>::new()));
    let log = Rc::clone(&seen);
    dialog.on("lookup", move |ctx| {
        log.borrow_mut().push(ctx.data());
        Flow::Submit
    });
    // Refused (no name), then accepted.
    key(&mut app, KB_ALT_L);
    type_text(&mut app, "Ada");
    key(&mut app, KB_ALT_L);
    let answer = dialog.run(&mut app).unwrap();
    assert_eq!(answer.action, "lookup");
    assert_eq!(seen.borrow().len(), 2);
}

#[test]
fn a_change_runs_its_action() {
    let mut app = app();
    let mut dialog = customer();
    let changes = Rc::new(RefCell::new(Vec::<Value>::new()));
    let log = Rc::clone(&changes);
    dialog.on("kind_changed", move |ctx| {
        log.borrow_mut().push(ctx.value("kind").unwrap());
        Flow::Stay
    });
    // Name, email, then the radio buttons: Down and Space pick Wholesale.
    type_text(&mut app, "Ada");
    key(&mut app, KB_TAB);
    key(&mut app, KB_TAB);
    key(&mut app, KB_DOWN);
    type_text(&mut app, " ");
    key(&mut app, KB_ALT_O);
    let answer = dialog.run(&mut app).unwrap();
    assert_eq!(answer.data["kind"], json!(1));
    assert_eq!(*changes.borrow(), vec![json!(1)]);
}

/// The `dialog_on_change` example: changing the mode moves the slider and
/// the meter, which the handler does with `set_value`. Those are the
/// code's changes, not the person's, so the slider's own handler is not run.
#[test]
fn a_change_can_set_other_controls() {
    let mut app = app();
    let mut dialog = JsonDialog::load("examples/dialog_specs/volume.json").unwrap();
    dialog.on_change("mode", |ctx| {
        let level = if ctx.value("mode") == Some(json!(1)) { 80 } else { 20 };
        ctx.set_value("volume", &json!(level)).unwrap();
        ctx.set_value("meter", &json!(level)).unwrap();
        Flow::Stay
    });
    dialog.on_change("volume", |ctx| {
        let level = ctx.value("volume").unwrap();
        ctx.set_value("meter", &level).unwrap();
        Flow::Stay
    });
    assert!(dialog.unbound().is_empty());
    // The first control is the mode: Down and Space pick Loud.
    key(&mut app, KB_DOWN);
    type_text(&mut app, " ");
    key(&mut app, KB_ALT_O);
    let answer = dialog.run(&mut app).unwrap();
    assert_eq!(answer.data["mode"], json!(1));
    assert_eq!(answer.data["volume"], json!(80));
    assert_eq!(answer.data["meter"], json!(80));
}

#[test]
fn any_control_can_be_watched_by_name() {
    let mut app = app();
    let mut dialog = customer();
    let typed = Rc::new(RefCell::new(Vec::<Value>::new()));
    let log = Rc::clone(&typed);
    // An input has no on_change in the JSON: it is watched by its name.
    dialog.on_change("name", move |ctx| {
        log.borrow_mut().push(ctx.value("name").unwrap());
        Flow::Stay
    });
    type_text(&mut app, "Ad");
    key(&mut app, KB_ALT_O);
    let answer = dialog.run(&mut app);
    assert!(answer.is_some(), "the name was typed");
    assert_eq!(*typed.borrow(), vec![json!("A"), json!("Ad")]);
}

#[test]
#[should_panic(expected = "no control with a value is named")]
fn watching_a_name_that_is_not_a_control_is_refused() {
    customer().on_change("nobody", |_| Flow::Stay);
}

#[test]
fn a_second_watcher_of_a_name_replaces_the_first() {
    let mut app = app();
    let mut dialog = customer();
    let calls = Rc::new(RefCell::new(Vec::<&str>::new()));
    let (first, second) = (Rc::clone(&calls), Rc::clone(&calls));
    dialog.on_change("name", move |_| {
        first.borrow_mut().push("first");
        Flow::Stay
    });
    dialog.on_change("name", move |_| {
        second.borrow_mut().push("second");
        Flow::Stay
    });
    type_text(&mut app, "A");
    key(&mut app, KB_ALT_O);
    let _ = dialog.run(&mut app);
    assert_eq!(*calls.borrow(), vec!["second"]);
}

#[test]
fn a_watcher_can_close_without_validating() {
    let mut app = app();
    let mut dialog = customer();
    dialog.on_change("email", |_| Flow::Close);
    // The name is required and empty, yet typing in the email closes.
    key(&mut app, KB_TAB);
    type_text(&mut app, "x");
    let answer = dialog.run(&mut app).expect("closed");
    assert_eq!(answer.action, "ok");
    assert_eq!(answer.data["name"], json!(""));
    assert_eq!(answer.data["email"], json!("x"));
}

#[test]
fn a_watcher_can_submit_and_validation_still_applies() {
    let mut app = app();
    let mut dialog = customer();
    dialog.on_change("email", |_| Flow::Submit);
    // The name is empty: the submit is refused and the dialog stays open
    // until Esc.
    key(&mut app, KB_TAB);
    type_text(&mut app, "x");
    key(&mut app, KB_ESC);
    assert!(dialog.run(&mut app).is_none());

    // With a name it is accepted at once, and no later handler runs.
    let mut app = self::app();
    let mut dialog = customer();
    let after = Rc::new(RefCell::new(0));
    let counted = Rc::clone(&after);
    dialog.on_change("email", |_| Flow::Submit);
    dialog.on_change("kind", move |_| {
        *counted.borrow_mut() += 1;
        Flow::Stay
    });
    type_text(&mut app, "Ada");
    key(&mut app, KB_TAB);
    type_text(&mut app, "x");
    let answer = dialog.run(&mut app).expect("accepted");
    assert_eq!(answer.data["name"], json!("Ada"));
    assert_eq!(*after.borrow(), 0);
}

#[test]
fn what_a_handler_sets_in_another_watched_control_is_not_a_change() {
    let mut app = app();
    let mut dialog = customer();
    let heard = Rc::new(RefCell::new(Vec::<&str>::new()));
    let (name_log, email_log) = (Rc::clone(&heard), Rc::clone(&heard));
    dialog.on_change("name", move |ctx| {
        name_log.borrow_mut().push("name");
        let _ = ctx.set_value("email", &json!("set by code"));
        Flow::Stay
    });
    dialog.on_change("email", move |_| {
        email_log.borrow_mut().push("email");
        Flow::Stay
    });
    type_text(&mut app, "A");
    key(&mut app, KB_ALT_O);
    let answer = dialog.run(&mut app).expect("accepted");
    assert_eq!(answer.data["email"], json!("set by code"));
    assert_eq!(*heard.borrow(), vec!["name"]);
}

#[test]
fn a_handlers_own_changes_do_not_start_a_loop() {
    let mut app = app();
    let mut dialog = customer();
    let runs = Rc::new(RefCell::new(0));
    let count = Rc::clone(&runs);
    dialog.on_change("name", move |ctx| {
        *count.borrow_mut() += 1;
        // Always a different value: a loop if it counted as a change.
        let _ = ctx.set_value("name", &json!(format!("x{}", *count.borrow())));
        Flow::Stay
    });
    type_text(&mut app, "A");
    key(&mut app, KB_ALT_O);
    let _ = dialog.run(&mut app);
    assert_eq!(*runs.borrow(), 1);
}

#[test]
fn a_custom_validator_is_bound_by_name() {
    let spec = r#"{ "width": 40, "height": 8, "controls": [
        { "type": "label", "x": 1, "y": 1, "w": 6, "h": 1, "text": "~C~ode", "link": "code" },
        { "type": "input", "name": "code", "x": 8, "y": 1, "w": 10, "h": 1,
          "validator": { "type": "custom", "name": "digits" } } ] }"#;
    let mut dialog = JsonDialog::from_json(spec).unwrap();
    assert_eq!(dialog.unbound(), vec!["validator \"digits\"".to_string()]);
    dialog.validator(
        "digits",
        Rc::new(RefCell::new(FilterValidator::new("0123456789"))),
    );
    assert!(dialog.unbound().is_empty());
    dialog.set_value("code", &json!("12a")).unwrap();
    let errors = dialog.validate().unwrap_err();
    assert_eq!(errors.field("code"), Some("Code is not valid"));
}

#[test]
fn unbound_lists_actions_without_code() {
    let mut dialog = customer();
    assert_eq!(
        dialog.unbound(),
        vec![
            "action \"kind_changed\"".to_string(),
            "action \"lookup\"".to_string(),
            "action \"save\"".to_string()
        ]
    );
    dialog.on("lookup", |_| Flow::Stay);
    assert_eq!(dialog.unbound().len(), 2);
}

#[test]
fn the_dialog_has_the_spec_size_and_title() {
    let dialog = customer();
    let bounds = dialog.dialog().bounds();
    assert_eq!((bounds.width(), bounds.height()), (50, 14));
    assert_eq!(dialog.spec().title, "Customer");
}

#[test]
fn a_spec_can_be_used_directly() {
    let spec = DialogSpec::from_json(CUSTOMER).unwrap();
    let mut dialog = JsonDialog::new(&spec).unwrap();
    assert_eq!(dialog.value("name"), Some(json!("")));
}

#[test]
fn the_example_specs_load_and_name_what_the_example_binds() {
    let mut customer =
        JsonDialog::from_json(include_str!("../examples/dialog_specs/customer.json")).unwrap();
    // examples/dialog_from_json.rs binds exactly these.
    assert_eq!(
        customer.unbound(),
        vec![
            "action \"kind_changed\"".to_string(),
            "action \"lookup\"".to_string()
        ]
    );
    assert_eq!(
        customer.data(),
        json!({ "name": "", "email": "", "country": 0, "kind": 0, "vip": false })
    );
}

#[test]
fn a_line_is_a_row_a_column_or_a_rectangle_with_corners() {
    let mut dialog = JsonDialog::from_json(
        r#"{ "title": "Lines", "width": 40, "height": 16, "controls": [
            { "type": "line", "x": 2, "y": 1, "w": 10, "h": 1 },
            { "type": "line", "x": 2, "y": 3, "w": 10, "h": 1, "style": "heavy" },
            { "type": "line", "x": 15, "y": 1, "w": 1, "h": 5, "style": "heavy" },
            { "type": "line", "x": 20, "y": 1, "w": 6, "h": 4, "style": "round" },
            { "type": "line", "x": 20, "y": 7, "w": 4, "h": 3, "style": "block" },
            { "type": "line", "x": 2, "y": 7, "w": 8, "h": 4, "style": "double" }
        ] }"#,
    )
    .unwrap();
    // A line has no value in the data.
    assert_eq!(dialog.data(), json!({}));
    let mut app = app();
    dialog.dialog_mut().draw(&mut app.terminal);
    let rows: Vec<String> = (0..16)
        .map(|y| app.terminal.buffer()[y].iter().map(|c| c.ch).collect())
        .collect();
    let all = rows.join("\n");
    assert!(all.contains("──────────"), "single row\n{all}");
    assert!(all.contains("━━━━━━━━━━"), "heavy row\n{all}");
    assert_eq!(all.matches('┃').count(), 5, "a heavy column five rows tall");
    assert!(all.contains("╭────╮"), "round top\n{all}");
    assert!(all.contains("╰────╯"), "round bottom\n{all}");
    assert!(all.contains("╔══════╗"), "double top\n{all}");
    assert!(all.contains("╚══════╝"), "double bottom\n{all}");
    assert_eq!(all.matches('█').count(), 4 + 4 + 2, "a block box is a ring of blocks");
}

#[test]
fn a_tree_selects_a_node_by_key_even_below_a_collapsed_one() {
    let mut dialog = JsonDialog::from_json(
        r#"{ "title": "Tree", "width": 40, "height": 14, "controls": [
            { "type": "tree", "name": "t", "x": 2, "y": 1, "w": 30, "h": 8, "nodes": [
                { "label": "A", "children": [
                    { "label": "B", "key": "b", "children": ["C"] }
                ] },
                "D"
            ] }
        ] }"#,
    )
    .unwrap();
    // The first node is selected unless the spec says otherwise.
    assert_eq!(dialog.data(), json!({ "t": "A" }));
    dialog.set_value("t", &json!("C")).unwrap();
    assert_eq!(dialog.value("t"), Some(json!("C")));
    dialog.set_value("t", &json!("D")).unwrap();
    assert_eq!(dialog.value("t"), Some(json!("D")));
    assert!(dialog.set_value("t", &json!("nope")).is_err());
    assert!(dialog.set_value("t", &json!(3)).is_err());
}

#[test]
fn a_tree_is_saved_as_plainly_as_it_was_written() {
    let json = r#"{ "type": "tree", "name": "t", "x": 0, "y": 0, "w": 20, "h": 5,
        "nodes": ["A", { "label": "B", "key": "b", "children": ["C"], "expanded": true }],
        "selected": "C" }"#;
    let control: tv_extensions::dialog_spec::Control = serde_json::from_str(json).unwrap();
    let saved = serde_json::to_value(&control).unwrap();
    assert_eq!(saved["nodes"][0], json!("A"));
    assert_eq!(
        saved["nodes"][1],
        json!({ "key": "b", "label": "B", "children": ["C"], "expanded": true })
    );
}

#[test]
fn a_group_box_holds_controls_placed_from_inside_its_frame() {
    let mut dialog = JsonDialog::from_json(
        r#"{ "title": "Group", "width": 40, "height": 16, "controls": [
            { "type": "group_box", "x": 2, "y": 2, "w": 30, "h": 8, "title": "Who", "controls": [
                { "type": "input", "name": "name", "x": 1, "y": 1, "w": 20, "h": 1 },
                { "type": "group_box", "x": 1, "y": 3, "w": 20, "h": 4, "title": "More", "controls": [
                    { "type": "input", "name": "email", "x": 2, "y": 1, "w": 10, "h": 1 }
                ] }
            ] }
        ] }"#,
    )
    .unwrap();
    // The data finds controls however deep they are.
    assert_eq!(dialog.data(), json!({ "name": "", "email": "" }));
    // Inside the frame: the box's corner, one in for the frame, the place.
    let name = dialog.view_mut::<InputLine>("name").unwrap().bounds().a;
    assert_eq!((name.x, name.y), (2 + 1 + 1, 2 + 1 + 1));
    let email = dialog.view_mut::<InputLine>("email").unwrap().bounds().a;
    assert_eq!((email.x, email.y), (2 + 1 + 1 + 1 + 2, 2 + 1 + 3 + 1 + 1));
}

#[test]
fn tabs_may_sit_in_a_group_but_not_in_tabs() {
    let ok = r#"{ "title": "T", "width": 40, "height": 16, "controls": [
        { "type": "group_box", "x": 1, "y": 1, "w": 30, "h": 10, "controls": [
            { "type": "tabs", "name": "t", "x": 1, "y": 1, "w": 20, "h": 6,
              "pages": [{ "title": "One", "controls": [] }] }
        ] }
    ] }"#;
    assert!(DialogSpec::from_json(ok).is_ok());
    let bad = r#"{ "title": "T", "width": 40, "height": 16, "controls": [
        { "type": "tabs", "name": "t", "x": 1, "y": 1, "w": 30, "h": 10, "pages": [
            { "title": "One", "controls": [
                { "type": "group_box", "x": 0, "y": 0, "w": 20, "h": 6, "controls": [
                    { "type": "tabs", "name": "u", "x": 0, "y": 0, "w": 10, "h": 4,
                      "pages": [{ "title": "Two", "controls": [] }] }
                ] }
            ] }
        ] }
    ] }"#;
    assert!(DialogSpec::from_json(bad).unwrap_err().0.contains("tabs cannot hold other tabs"));
}

#[test]
fn a_label_and_a_line_can_be_coloured() {
    use turbo_vision::core::palette::TvColor;
    let mut dialog = JsonDialog::from_json(
        r#"{ "title": "Colours", "width": 40, "height": 10, "controls": [
            { "type": "label", "x": 2, "y": 1, "w": 10, "h": 1, "text": "~N~ame", "color": "red" },
            { "type": "line", "x": 2, "y": 3, "w": 10, "h": 1, "color": "yellow" },
            { "type": "label", "x": 2, "y": 5, "w": 10, "h": 1, "text": "~P~lain" }
        ] }"#,
    )
    .unwrap();
    let mut app = app();
    dialog.dialog_mut().draw(&mut app.terminal);
    let (w, h) = app.terminal.size();
    // The first cell of each run of `text` on screen.
    let find = |text: &str| -> (i16, i16) {
        for y in 0..h {
            let row: String = (0..w)
                .map(|x| app.terminal.read_cell(x, y).map_or(' ', |c| c.ch))
                .collect();
            if let Some(x) = row.find(text) {
                return (i16::try_from(x).unwrap(), y);
            }
        }
        panic!("{text} is not on screen");
    };
    let fg = |at: (i16, i16)| app.terminal.read_cell(at.0, at.1).unwrap().attr.fg;
    let (x, y) = find("ame");
    assert_eq!(fg((x, y)), TvColor::Red, "the label's text");
    let (px, py) = find("lain");
    assert_eq!(fg(find("──────────")), TvColor::Yellow, "the line");
    assert_ne!(fg((px, py)), TvColor::Red, "an uncoloured label is as it was");
    // The colour is saved by name, and only when there is one.
    let json = serde_json::to_string(dialog.spec()).unwrap();
    assert!(json.contains("\"color\":\"red\"") && json.contains("\"color\":\"yellow\""));
    assert_eq!(json.matches("color").count(), 2);
}
