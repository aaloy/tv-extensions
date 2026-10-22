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
