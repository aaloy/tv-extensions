//! Form JSON Agent Demo - the calling side of a JSON form.
//!
//! Plays the part of an agent that needs a person to confirm a deployment.
//! It sends a spec; if the spec is wrong, the error message is exactly what it
//! would show the model that wrote it, so the model can fix the spec and try
//! again; if the person cancels, the agent stops; otherwise it reads the
//! answers and acts on them.
//!
//! The first spec has a deliberate mistake (`"type": "txt"`) to show the
//! recovery.
//!
//! Run with: `cargo run --example form_json_agent --features native,json`

// (C) 2026 - Antoni Aloy

use serde_json::{Value, json};
use turbo_vision::app::Application;
use tv_extensions::forms::json::{FormError, JsonForm};

/// What the "model" sends first, with a typo, and what it sends once told.
fn first_attempt() -> Value {
    json!({
        "title": "Deploy to production?",
        "fields": [
            { "name": "reason", "type": "txt", "required": true },
            { "name": "backup", "type": "boolean", "label": "I have a backup", "required": true }
        ]
    })
}

fn corrected(why: &str) -> Value {
    eprintln!("(the agent was told: {why})");
    let mut spec: Value = first_attempt();
    spec["fields"][0]["type"] = json!("text");
    spec
}

/// Build the form, repairing the spec once if it is refused.
fn build(spec: Value) -> Result<JsonForm, FormError> {
    match JsonForm::from_spec(&spec) {
        Err(FormError::Spec(why)) => JsonForm::from_spec(&corrected(&why)),
        other => other,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut form = build(first_attempt())?;

    let answer = {
        let mut app = Application::new()?;
        form.run(&mut app)
    };

    match answer {
        Ok(answers) => {
            let reason = answers["reason"].as_str().unwrap_or_default();
            println!("Deploying. Reason given: {reason}");
        }
        Err(FormError::Cancelled) => println!("The person said no; nothing deployed."),
        Err(FormError::Spec(why)) => println!("The form was invalid: {why}"),
    }
    Ok(())
}
