// (C) 2026 - Antoni Aloy

//! Dialog specs: the JSON a designed dialog is saved as, read back, checked.

use tv_extensions::dialog_spec::{Control, DialogSpec, Kind, ValidatorSpec};

/// A dialog with one control of every kind.
const EVERYTHING: &str = include_str!("../examples/dialog_specs/everything.json");

fn everything() -> DialogSpec {
    DialogSpec::from_json(EVERYTHING).expect("the sample is a valid spec")
}

#[test]
fn every_kind_is_read() {
    let spec = everything();
    assert_eq!(spec.title, "Everything");
    assert_eq!((spec.width, spec.height), (76, 26));
    assert_eq!(spec.controls.len(), 22);

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
    assert!(matches!(
        spec.find("city").unwrap().kind,
        Kind::Input { .. }
    ));
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
    let order: Vec<usize> = [
        "\"type\"", "\"name\"", "\"x\"", "\"y\"", "\"w\"", "\"h\"", "\"text\"",
    ]
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
