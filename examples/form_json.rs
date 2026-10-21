//! Form JSON Demo - a form described as JSON, answered as JSON.
//!
//! What an agent or an MCP tool does to ask a person something: it passes a
//! spec, the person fills the form in, and the answers come back as JSON (or
//! the person cancels). The answers are printed on stdout once the screen is
//! restored.
//!
//! Run with: `cargo run --example form_json --features native,json`
//! One of the specs in `examples/form_json_specs/`: add `-- deploy`,
//! `-- survey`, `-- tabs` or `-- layout` (groups, rows, spinner, slider; the default is `customer`).
//! Your own: `-- path/to/spec.json`

// (C) 2026 - Antoni Aloy

use serde_json::Value;
use turbo_vision::app::Application;
use tv_extensions::forms::json::{FormError, run};

const CUSTOMER: &str = include_str!("form_json_specs/customer.json");
const DEPLOY: &str = include_str!("form_json_specs/deploy.json");
const SURVEY: &str = include_str!("form_json_specs/survey.json");
const LAYOUT: &str = include_str!("form_json_specs/layout.json");
const TABS: &str = include_str!("form_json_specs/tabs.json");

fn load(which: Option<&str>) -> Result<Value, Box<dyn std::error::Error>> {
    let text = match which {
        None | Some("customer") => CUSTOMER.to_string(),
        Some("deploy") => DEPLOY.to_string(),
        Some("survey") => SURVEY.to_string(),
        Some("layout") => LAYOUT.to_string(),
        Some("tabs") => TABS.to_string(),
        Some(path) => std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?,
    };
    Ok(serde_json::from_str(&text)?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let spec = load(std::env::args().nth(1).as_deref())?;

    let result = {
        let mut app = Application::new()?;
        run(&mut app, &spec)
    }; // the application is dropped here: the terminal is restored

    match result {
        Ok(answers) => println!("{}", serde_json::to_string_pretty(&answers)?),
        Err(FormError::Cancelled) => eprintln!("cancelled"),
        Err(FormError::Spec(why)) => eprintln!("invalid form: {why}"),
    }
    Ok(())
}
