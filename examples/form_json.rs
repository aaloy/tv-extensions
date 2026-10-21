//! Form JSON Demo - a form described as JSON, answered as JSON.
//!
//! What an agent or an MCP tool does to ask a person something: it passes a
//! spec, the person fills the form in, and the answers come back as JSON (or
//! the person cancels).
//!
//! Run with: `cargo run --example form_json --features native,json`
//! With your own spec: `cargo run --example form_json --features native,json -- spec.json`
//! The answers are printed on stdout after the screen is restored.

// (C) 2026 - Antoni Aloy

use serde_json::{Value, json};
use turbo_vision::app::Application;
use tv_extensions::forms::json::{FormError, run};

fn demo_spec() -> Value {
    json!({
        "title": "New customer",
        "fields": [
            { "name": "name", "type": "text", "label": "Name", "required": true, "max_len": 40 },
            { "name": "email", "type": "text", "required": true },
            { "name": "age", "type": "integer", "min": 0, "max": 120 },
            { "name": "since", "type": "date", "default": "2026-10-21" },
            { "type": "section", "label": "Account" },
            { "name": "kind", "type": "choice", "default": "retail",
              "options": [
                  { "label": "Retail", "value": "retail" },
                  { "label": "Wholesale", "value": "wholesale" }
              ] },
            { "name": "credit", "type": "number", "min": 0 },
            { "name": "vip", "type": "boolean", "label": "VIP customer" },
            { "name": "notes", "type": "memo", "rows": 3 }
        ]
    })
}

fn main() -> turbo_vision::core::error::Result<()> {
    let spec = match std::env::args().nth(1) {
        Some(path) => {
            let text = std::fs::read_to_string(&path)?;
            serde_json::from_str(&text)
                .map_err(|e| std::io::Error::other(format!("{path}: {e}")))?
        }
        None => demo_spec(),
    };

    let result = {
        let mut app = Application::new()?;
        run(&mut app, &spec)
    }; // the application is dropped here: the terminal is restored

    match result {
        Ok(answers) => println!(
            "{}",
            serde_json::to_string_pretty(&answers).unwrap_or_default()
        ),
        Err(FormError::Cancelled) => eprintln!("cancelled"),
        Err(FormError::Spec(why)) => eprintln!("invalid form: {why}"),
    }
    Ok(())
}
