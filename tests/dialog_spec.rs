// (C) 2026 - Antoni Aloy

//! Dialog specs: the JSON a designed dialog is saved as, read back, checked.

use tv_extensions::dialog_spec::{Control, DialogSpec, Kind, ValidatorSpec};

/// A dialog with one control of every kind.
const EVERYTHING: &str = r#"{
  "version": 1,
  "title": "Everything",
  "width": 70,
  "height": 22,
  "controls": [
    { "type": "label", "name": "name_label", "x": 2, "y": 1, "w": 8, "h": 1, "text": "~N~ame", "link": "name" },
    { "type": "input", "name": "name", "x": 11, "y": 1, "w": 30, "h": 1, "max_len": 40, "required": true,
      "validator": { "type": "filter", "chars": "abcdefghijklmnopqrstuvwxyz " }, "history": 3 },
    { "type": "static_text", "x": 2, "y": 2, "w": 30, "h": 1, "text": "Fill in the form", "centered": true },
    { "type": "memo", "name": "notes", "x": 2, "y": 3, "w": 30, "h": 3, "text": "a\nb" },
    { "type": "check_box", "name": "vip", "x": 2, "y": 6, "w": 20, "h": 1, "text": "~V~IP", "checked": true },
    { "type": "check_boxes", "name": "extras", "x": 2, "y": 7, "w": 20, "h": 2, "items": ["Wi-Fi", "Parking"], "checked": [1] },
    { "type": "radio_buttons", "name": "kind", "x": 24, "y": 7, "w": 20, "h": 2, "items": ["Retail", "Wholesale"], "selected": 1, "on_change": "kind_changed" },
    { "type": "combo_box", "name": "country", "x": 2, "y": 9, "w": 20, "h": 1, "items": ["Spain", "Italy"], "selected": 0 },
    { "type": "list_box", "name": "tags", "x": 46, "y": 1, "w": 20, "h": 4, "items": ["red", "green"], "multi": true },
    { "type": "spinner", "name": "age", "x": 24, "y": 9, "w": 10, "h": 1, "min": 0, "max": 120, "value": 30 },
    { "type": "slider", "name": "volume", "x": 36, "y": 9, "w": 20, "h": 1, "min": 0, "max": 10, "value": 5 },
    { "type": "progress_bar", "name": "progress", "x": 2, "y": 10, "w": 30, "h": 1, "max": 100, "value": 40 },
    { "type": "table", "name": "orders", "x": 46, "y": 5, "w": 20, "h": 4,
      "columns": [ { "title": "Item", "width": 10 }, { "title": "Qty", "width": 5, "align": "right" } ],
      "rows": [["Pen", "2"]] },
    { "type": "group_box", "x": 1, "y": 11, "w": 66, "h": 6, "title": "More" },
    { "type": "tabs", "name": "pages", "x": 2, "y": 12, "w": 40, "h": 4, "pages": [
        { "title": "~O~ne", "controls": [
          { "type": "input", "name": "city", "x": 1, "y": 0, "w": 20, "h": 1,
            "validator": { "type": "picture", "mask": "@@@@@@" } }
        ] },
        { "title": "~T~wo", "controls": [] }
    ] },
    { "type": "auto_complete", "name": "fruit", "x": 44, "y": 12, "w": 20, "h": 1,
      "items": ["Apple", "Banana"], "require_match": true },
    { "type": "dual_list", "name": "team", "x": 44, "y": 13, "w": 22, "h": 4,
      "items": ["ann", { "key": "bob", "label": "Bob B." }], "chosen": ["bob"] },
    { "type": "error_line", "x": 2, "y": 17, "w": 60, "h": 1 },
    { "type": "button", "x": 20, "y": 18, "w": 10, "h": 2, "text": "~O~K", "command": "ok", "default": true },
    { "type": "button", "x": 32, "y": 18, "w": 12, "h": 2, "text": "~L~ookup", "command": "lookup" }
  ]
}"#;

fn everything() -> DialogSpec {
    DialogSpec::from_json(EVERYTHING).expect("the sample is a valid spec")
}

#[test]
fn every_kind_is_read() {
    let spec = everything();
    assert_eq!(spec.title, "Everything");
    assert_eq!((spec.width, spec.height), (70, 22));
    assert_eq!(spec.controls.len(), 20);

    let name = spec.find("name").unwrap();
    assert_eq!((name.x, name.y, name.w, name.h), (11, 1, 30, 1));
    match &name.kind {
        Kind::Input {
            max_len,
            required,
            validator,
            history,
            ..
        } => {
            assert_eq!(*max_len, 40);
            assert!(*required);
            assert_eq!(
                validator,
                &Some(ValidatorSpec::Filter {
                    chars: "abcdefghijklmnopqrstuvwxyz ".into()
                })
            );
            assert_eq!(*history, Some(3));
        }
        other => panic!("name is not an input: {other:?}"),
    }
    // Controls on a tab page are found by name too.
    assert!(matches!(spec.find("city").unwrap().kind, Kind::Input { .. }));
}

#[test]
fn saving_and_loading_gives_the_same_spec() {
    let spec = everything();
    let again = DialogSpec::from_json(&spec.to_json()).unwrap();
    assert_eq!(again, spec);
}

#[test]
fn saved_controls_start_with_their_type_name_and_place() {
    let json = everything().to_json();
    let first = json.find("\"type\": \"label\"").unwrap();
    let rest = &json[first..];
    let order: Vec<usize> = ["\"type\"", "\"name\"", "\"x\"", "\"y\"", "\"w\"", "\"h\"", "\"text\""]
        .iter()
        .map(|key| rest.find(key).unwrap())
        .collect();
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?} in {rest}");
}

#[test]
fn defaults_are_left_out_when_saved() {
    let spec = DialogSpec {
        controls: vec![Control::new(
            "",
            (2, 2, 10, 2),
            Kind::Button {
                text: "~O~K".into(),
                command: "ok".into(),
                default: false,
            },
        )],
        ..DialogSpec::new("Small", 30, 8)
    };
    let json = spec.to_json();
    assert!(!json.contains("\"default\""), "{json}");
    assert!(!json.contains("\"name\""), "{json}");
    assert!(!json.contains("\"resizable\""), "{json}");
}

#[test]
fn an_unknown_type_is_refused_by_name() {
    let err = DialogSpec::from_json(
        r#"{ "width": 30, "height": 8, "controls": [
            { "type": "txt", "name": "a", "x": 1, "y": 1, "w": 5, "h": 1 } ] }"#,
    )
    .unwrap_err();
    let text = err.to_string();
    assert!(text.contains("txt"), "{text}");
    assert!(text.contains("\"a\""), "{text}");
}

#[test]
fn a_misspelt_key_is_refused() {
    let err = DialogSpec::from_json(
        r#"{ "width": 30, "height": 8, "controls": [
            { "type": "input", "name": "a", "x": 1, "y": 1, "w": 5, "h": 1, "maxlen": 4 } ] }"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("maxlen"), "{err}");

    let err = DialogSpec::from_json(r#"{ "width": 30, "height": 8, "colour": 3 }"#).unwrap_err();
    assert!(err.to_string().contains("colour"), "{err}");
}

#[test]
fn a_control_needs_its_place() {
    let err = DialogSpec::from_json(
        r#"{ "width": 30, "height": 8, "controls": [
            { "type": "input", "name": "a", "x": 1, "y": 1, "w": 5 } ] }"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("`h`"), "{err}");
}

#[test]
fn names_are_unique_across_pages() {
    let err = DialogSpec::from_json(
        r#"{ "width": 40, "height": 12, "controls": [
            { "type": "input", "name": "a", "x": 1, "y": 1, "w": 5, "h": 1 },
            { "type": "tabs", "name": "t", "x": 1, "y": 2, "w": 30, "h": 6, "pages": [
              { "title": "P", "controls": [ { "type": "input", "name": "a", "x": 1, "y": 0, "w": 5, "h": 1 } ] } ] } ] }"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("\"a\""), "{err}");
}

#[test]
fn a_label_links_to_a_control_that_exists() {
    let err = DialogSpec::from_json(
        r#"{ "width": 30, "height": 8, "controls": [
            { "type": "label", "x": 1, "y": 1, "w": 5, "h": 1, "text": "N", "link": "nobody" } ] }"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("nobody"), "{err}");
}

#[test]
fn tabs_do_not_nest() {
    let err = DialogSpec::from_json(
        r#"{ "width": 40, "height": 12, "controls": [
            { "type": "tabs", "name": "t", "x": 1, "y": 1, "w": 30, "h": 8, "pages": [
              { "title": "P", "controls": [
                { "type": "tabs", "name": "u", "x": 0, "y": 0, "w": 20, "h": 5, "pages": [] } ] } ] } ] }"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("tabs"), "{err}");
}

#[test]
fn a_selection_must_be_one_of_the_items() {
    let err = DialogSpec::from_json(
        r#"{ "width": 30, "height": 8, "controls": [
            { "type": "radio_buttons", "name": "r", "x": 1, "y": 1, "w": 10, "h": 2, "items": ["a", "b"], "selected": 2 } ] }"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("\"r\""), "{err}");
}
