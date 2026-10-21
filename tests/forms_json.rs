// (C) 2026 - Antoni Aloy

//! JSON forms through the public API: a person's keystrokes in, JSON out.
//!
//! Each test builds a spec, queues the keys a person would press, runs the
//! form on a test terminal, and checks the answers. The example specs under
//! `examples/form_json_specs/` are checked here too, so they cannot rot.

use serde_json::{Value, json};
use turbo_vision::app::Application;
use turbo_vision::core::event::{Event, KB_ALT_O, KB_BACKSPACE, KB_DOWN, KB_ENTER, KB_ESC, KB_TAB};
use turbo_vision::test_util::test_terminal;
use tv_extensions::forms::json::{FormError, JsonForm, run};

const CUSTOMER: &str = include_str!("../examples/form_json_specs/customer.json");
const DEPLOY: &str = include_str!("../examples/form_json_specs/deploy.json");
const SURVEY: &str = include_str!("../examples/form_json_specs/survey.json");

/// One thing a person does.
enum Key {
    /// Type these characters into the focused field.
    Type(&'static str),
    Tab,
    Down,
    Enter,
    Backspace(usize),
    Esc,
    /// Alt+O: press OK.
    Ok,
}
use Key::{Backspace, Down, Enter, Esc, Ok as PressOk, Tab, Type};

fn app() -> Application {
    // Tall enough for the longest example form.
    Application::with_terminal(test_terminal(100, 50))
}

fn send(app: &mut Application, keys: &[Key]) {
    let tx = app.terminal.event_injector();
    for key in keys {
        match key {
            Type(text) => {
                for c in text.chars() {
                    tx.send(Event::text(c)).unwrap();
                }
            }
            Tab => tx.send(Event::keyboard(KB_TAB)).unwrap(),
            Down => tx.send(Event::keyboard(KB_DOWN)).unwrap(),
            Enter => tx.send(Event::keyboard(KB_ENTER)).unwrap(),
            Backspace(n) => {
                for _ in 0..*n {
                    tx.send(Event::keyboard(KB_BACKSPACE)).unwrap();
                }
            }
            Esc => tx.send(Event::keyboard(KB_ESC)).unwrap(),
            PressOk => tx.send(Event::keyboard(KB_ALT_O)).unwrap(),
        }
    }
}

/// Run `spec` with the person pressing `keys`.
fn answer(spec: &Value, keys: &[Key]) -> Result<Value, FormError> {
    let mut app = app();
    send(&mut app, keys);
    run(&mut app, spec)
}

fn spec(text: &str) -> Value {
    serde_json::from_str(text).expect("the example spec is JSON")
}

// ---- the example specs ------------------------------------------------

#[test]
fn every_example_spec_builds() {
    for (name, text) in [
        ("customer", CUSTOMER),
        ("deploy", DEPLOY),
        ("survey", SURVEY),
    ] {
        if let Err(e) = JsonForm::from_spec(&spec(text)) {
            panic!("{name}.json: {e}");
        }
    }
}

#[test]
fn the_customer_form_is_filled_in_field_by_field() {
    let answers = answer(
        &spec(CUSTOMER),
        &[
            Type("Ada Lovelace"),
            Tab,
            Type("ada@example.com"),
            Tab,
            Type("36"),
            Tab,  // since: keeps its default
            Tab,  // kind
            Down, // Wholesale
            Tab,
            Type("2.5"), // credit
            Tab,
            Type(" "), // vip: Space ticks the box
            Tab,
            Type("first line"),
            Enter,
            Type("second"),
            PressOk,
        ],
    )
    .expect("answers");
    assert_eq!(
        answers,
        json!({
            "name": "Ada Lovelace",
            "email": "ada@example.com",
            "age": 36,
            "since": "2026-10-21",
            "kind": "wholesale",
            "credit": 2.5,
            "vip": true,
            "notes": "first line\nsecond"
        })
    );
}

#[test]
fn deploy_needs_a_reason_and_a_ticked_backup_box() {
    let answers = answer(
        &spec(DEPLOY),
        &[
            Type(""), // environment keeps "staging"
            Tab,
            PressOk, // refused: no reason
            Type("hotfix"),
            Tab,     // replicas
            Tab,     // backup
            PressOk, // refused: the box must be ticked
            Type(" "),
            PressOk,
        ],
    )
    .expect("answers");
    assert_eq!(
        answers,
        json!({
            "environment": "staging",
            "reason": "hotfix",
            "replicas": 3,
            "i_have_a_backup": true
        })
    );
}

#[test]
fn the_survey_leaves_optional_fields_null() {
    let answers = answer(
        &spec(SURVEY),
        &[
            Down, // role: Developer (the first option is chosen by Down)
            Tab,
            Tab, // years
            Tab, // born
            Type("4"),
            PressOk,
        ],
    )
    .expect("answers");
    assert_eq!(answers["role"], json!(1));
    assert_eq!(answers["years"], Value::Null);
    assert_eq!(answers["born"], Value::Null);
    assert_eq!(answers["rating"], json!(4));
    assert_eq!(answers["comments"], json!(""));
    assert_eq!(answers["contact_me"], json!(false));
}

// ---- validation keeps the form open -----------------------------------

#[test]
fn a_value_out_of_range_is_refused_until_it_is_fixed() {
    let spec = json!({ "fields": [{ "name": "age", "type": "integer", "min": 0, "max": 120 }] });
    let answers = answer(
        &spec,
        &[
            Type("200"),
            PressOk, // refused: too large
            Backspace(3),
            Type("30"),
            PressOk,
        ],
    )
    .expect("answers");
    assert_eq!(answers, json!({ "age": 30 }));
}

#[test]
fn a_choice_must_be_answered() {
    let spec = json!({ "fields": [{ "name": "c", "type": "choice", "options": ["a", "b"] }] });
    let answers = answer(&spec, &[PressOk, Down, Down, PressOk]).expect("answers");
    assert_eq!(answers, json!({ "c": "b" }));
}

#[test]
fn text_beyond_ascii_comes_back_intact() {
    let spec = json!({ "fields": [{ "name": "t", "type": "text" }] });
    let answers = answer(&spec, &[Type("Ñandú ě €"), PressOk]).expect("answers");
    assert_eq!(answers, json!({ "t": "Ñandú ě €" }));
}

#[test]
fn max_len_stops_the_typing() {
    let spec = json!({ "fields": [{ "name": "t", "type": "text", "max_len": 3 }] });
    let answers = answer(&spec, &[Type("abcdef"), PressOk]).expect("answers");
    assert_eq!(answers, json!({ "t": "abc" }));
}

#[test]
fn a_number_accepts_decimals_and_a_date_a_calendar_day() {
    let spec = json!({ "fields": [
        { "name": "n", "type": "number" },
        { "name": "d", "type": "date" }
    ]});
    let answers =
        answer(&spec, &[Type("-0.25"), Tab, Type("2026-02-28"), PressOk]).expect("answers");
    assert_eq!(answers, json!({ "n": -0.25, "d": "2026-02-28" }));
}

#[test]
fn an_impossible_date_is_refused() {
    let spec = json!({ "fields": [{ "name": "d", "type": "date" }] });
    let answers = answer(
        &spec,
        &[
            Type("2026-02-30"),
            PressOk, // refused: no 30th of February
            Backspace(2),
            Type("28"),
            PressOk,
        ],
    )
    .expect("answers");
    assert_eq!(answers, json!({ "d": "2026-02-28" }));
}

// ---- cancelling, reusing, ordering ------------------------------------

#[test]
fn escape_cancels_whatever_was_typed() {
    let spec = json!({ "fields": [{ "name": "t", "type": "text", "required": true }] });
    assert_eq!(
        answer(&spec, &[Type("half"), Esc]),
        Err(FormError::Cancelled)
    );
}

#[test]
fn a_bad_spec_is_reported_before_anything_is_shown() {
    // No keys are queued: if the form opened, the test would hang.
    let spec = json!({ "fields": [{ "name": "t", "type": "txt" }] });
    let Err(FormError::Spec(why)) = answer(&spec, &[]) else {
        panic!("expected a spec error");
    };
    assert!(
        why.starts_with("fields[0].type: unknown type \"txt\""),
        "{why}"
    );
}

#[test]
fn a_form_runs_again_from_its_defaults() {
    let mut form = JsonForm::from_spec(&json!({ "fields": [
        { "name": "a", "type": "text", "default": "x" }
    ]}))
    .unwrap();
    let mut app = app();
    send(&mut app, &[Type("1"), PressOk]);
    assert_eq!(form.run(&mut app), Ok(json!({ "a": "x1" })));
    send(&mut app, &[Esc]);
    assert_eq!(form.run(&mut app), Err(FormError::Cancelled));
    send(&mut app, &[PressOk]);
    assert_eq!(form.run(&mut app), Ok(json!({ "a": "x" })), "no leftovers");
}

#[test]
fn answers_follow_the_specs_order_not_the_alphabet() {
    let spec = json!({ "fields": [
        { "name": "zebra", "type": "text" },
        { "type": "section", "label": "Middle" },
        { "name": "apple", "type": "text" },
        { "name": "mango", "type": "boolean" }
    ]});
    let answers = answer(&spec, &[PressOk]).expect("answers");
    let keys: Vec<_> = answers.as_object().unwrap().keys().cloned().collect();
    assert_eq!(keys, ["zebra", "apple", "mango"]);
}

#[test]
fn many_fields_with_hot_keys_still_take_the_keyboard() {
    // Twelve fields all named alike: the hot keys picked for them are
    // unique, and the form still works.
    let fields: Vec<Value> = (0..12)
        .map(|i| json!({ "name": format!("field_{i}"), "type": "text" }))
        .collect();
    let answers = answer(&json!({ "fields": fields }), &[Type("x"), PressOk]).expect("answers");
    assert_eq!(answers["field_0"], json!("x"));
    assert_eq!(answers.as_object().unwrap().len(), 12);
}

// ---- bad specs --------------------------------------------------------

#[test]
fn bad_specs_say_where_and_what() {
    let cases: &[(Value, &str)] = &[
        (json!("form"), "the form must be a JSON object"),
        (
            json!({ "title": 3, "fields": [{}] }),
            "title: must be a string",
        ),
        (
            json!({ "fields": "name" }),
            "fields: a list of fields is required",
        ),
        (json!({ "fields": [3] }), "fields[0]: must be an object"),
        (
            json!({ "fields": [{ "name": "a" }] }),
            "fields[0].type: required",
        ),
        (
            json!({ "fields": [{ "name": "a", "type": "text", "label": 5 }] }),
            "fields[0].label: must be a string",
        ),
        (
            json!({ "fields": [{ "name": "a", "type": "text", "required": "yes" }] }),
            "fields[0].required: must be true or false",
        ),
        (
            json!({ "fields": [{ "name": "a", "type": "text", "max_len": 0 }] }),
            "fields[0].max_len: must be a positive whole number",
        ),
        (
            json!({ "fields": [{ "name": "a", "type": "integer", "min": "low" }] }),
            "fields[0].min: must be a number",
        ),
        (
            json!({ "fields": [{ "name": "a", "type": "integer", "default": 1.5 }] }),
            "fields[0].default: must be a whole number",
        ),
        (
            json!({ "fields": [{ "name": "a", "type": "boolean", "default": "yes" }] }),
            "fields[0].default: must be true or false",
        ),
        (
            json!({ "fields": [{ "name": "a", "type": "choice", "options": [] }] }),
            "fields[0].options: a choice needs a list of options",
        ),
        (
            json!({ "fields": [{ "name": "a", "type": "choice", "options": ["x", "x"] }] }),
            "fields[0].options[1]: the value \"x\" is used twice",
        ),
        (
            json!({ "fields": [{ "name": "a", "type": "choice", "options": ["x"], "default": "y" }] }),
            "fields[0].default: is not one of the options' values",
        ),
        (
            json!({ "fields": [{ "name": "a", "type": "choice", "options": [{ "label": "L", "value": null }] }] }),
            "fields[0].options[0].value: must be a string, number or boolean",
        ),
        (
            json!({ "fields": [{ "name": "a", "type": "choice", "options": [[1]] }] }),
            "fields[0].options[0]: must be a string, number, boolean or object",
        ),
        (
            json!({ "fields": [{ "type": "section" }] }),
            "fields[0].label: a section needs a label",
        ),
        (
            json!({ "fields": [{ "name": "", "type": "text" }] }),
            "fields[0].name: required",
        ),
    ];
    for (spec, expected) in cases {
        let Err(FormError::Spec(why)) = JsonForm::from_spec(spec) else {
            panic!("{spec} was accepted");
        };
        assert!(
            why.contains(expected),
            "{spec}\n  gave: {why}\n  wanted: {expected}"
        );
    }
}

#[test]
fn form_errors_read_well() {
    assert_eq!(FormError::Cancelled.to_string(), "the form was cancelled");
    assert_eq!(
        FormError::Spec("fields: x".into()).to_string(),
        "invalid form: fields: x"
    );
}
