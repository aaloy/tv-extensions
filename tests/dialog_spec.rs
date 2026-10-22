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

/// The blocks of `docs/dialog-json.md` marked `json dialog` (a whole
/// dialog) or `json control` (one control) are what an agent copies, so they
/// must load.
#[test]
fn the_json_reference_examples_load() {
    let doc = std::fs::read_to_string("docs/dialog-json.md").unwrap();
    let (mut dialogs, mut controls) = (0, 0);
    let mut rest = doc.as_str();
    while let Some(at) = rest.find("```json ") {
        rest = &rest[at + "```json ".len()..];
        let kind = rest.lines().next().unwrap().trim();
        let body_start = rest.find('\n').unwrap() + 1;
        let body = &rest[body_start..rest[body_start..].find("```").unwrap() + body_start];
        let json = match kind {
            "dialog" => {
                dialogs += 1;
                body.to_string()
            }
            "control" => {
                controls += 1;
                // A label's link needs its control.
                format!(
                    r#"{{ "width": 80, "height": 30, "controls": [
                        {{ "type": "input", "name": "name", "x": 0, "y": 0, "w": 5, "h": 1 }},
                        {body} ] }}"#
                )
            }
            other => panic!("unknown block kind {other:?}"),
        };
        let spec = DialogSpec::from_json(&json).unwrap_or_else(|e| panic!("{e}\n{json}"));
        // Saved and read back, it is the same spec.
        assert_eq!(DialogSpec::from_json(&spec.to_json()).unwrap(), spec);
        rest = &rest[body_start..];
    }
    assert!(dialogs >= 3, "{dialogs} dialogs");
    assert!(controls >= 21, "{controls} controls");
}

/// Every control type has a block in the reference.
#[test]
fn the_json_reference_shows_every_control_type() {
    let doc = std::fs::read_to_string("docs/dialog-json.md").unwrap();
    for name in [
        "label",
        "static_text",
        "line",
        "input",
        "auto_complete",
        "memo",
        "spinner",
        "slider",
        "check_box",
        "check_boxes",
        "radio_buttons",
        "combo_box",
        "list_box",
        "tree",
        "table",
        "dual_list",
        "group_box",
        "tabs",
        "button",
        "progress_bar",
        "error_line",
    ] {
        assert!(doc.contains(&format!("{{ \"type\": \"{name}\"")), "{name}");
    }
}

/// The block of a doc fenced with `fence` (such as "```json dialog").
fn block<'a>(doc: &'a str, fence: &str) -> &'a str {
    let at = doc
        .find(fence)
        .unwrap_or_else(|| panic!("no {fence} block"));
    let rest = &doc[at + fence.len()..];
    let start = rest.find('\n').unwrap() + 1;
    &rest[start..rest[start..].find("```").unwrap() + start]
}

/// The tutorial's JSON is the JSON the example loads, and its finished
/// program is the example's code.
#[test]
fn the_tutorial_shows_what_the_example_runs() {
    let doc = std::fs::read_to_string("docs/dialog-tutorial.md").unwrap();
    let shown = DialogSpec::from_json(block(&doc, "```json dialog")).unwrap();
    let file = DialogSpec::load("examples/dialog_specs/order.json").unwrap();
    assert_eq!(shown, file);

    let source = std::fs::read_to_string("examples/dialog_order.rs").unwrap();
    let in_source: Vec<&str> = source.lines().map(str::trim).collect();
    for line in block(&doc, "```rust,final").lines().map(str::trim) {
        if !line.is_empty() {
            assert!(in_source.contains(&line), "not in dialog_order.rs: {line}");
        }
    }
    // The pieces shown on the way are there too.
    for line in [
        "fn recompute(ctx: &mut ActionContext<'_>) -> Flow {",
        "const PRICES: [i64; 3] = [2, 5, 3];",
    ] {
        assert!(doc.contains(line) && source.contains(line), "{line}");
    }
}
