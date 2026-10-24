// (C) 2026 - Antoni Aloy

//! A designed dialog in an application: load the JSON the dialog designer
//! saved, bind validation and actions to its controls by name, run it, and
//! use the data.
//!
//! ```sh
//! cargo run --example dialog_from_json --features native,dialog
//! ```
//!
//! `examples/dialog_specs/customer.json` was laid out in the designer
//! (`cargo run --example dialog_designer --features native,dialog --
//! examples/dialog_specs/customer.json`). Everything below is code the
//! JSON cannot say: an email rule, a rule across two fields, what the
//! Lookup button does, and what happens when the customer kind changes.

use serde_json::{Value, json};
use turbo_vision::app::Application;
use turbo_vision::views::checkbox::CheckBox;
use turbo_vision::views::msgbox::message_box_ok;
use tv_extensions::dialog_spec::{Flow, JsonDialog, SpecError};

/// Customers the Lookup button knows, by name.
const KNOWN: [(&str, &str, usize); 2] = [
    ("Ada Lovelace", "ada@example.org", 1),
    ("Grace Hopper", "grace@navy.example", 0),
];

/// Show the customer dialog filled with `start`. `None` when cancelled.
fn edit_customer(app: &mut Application, start: &Value) -> Result<Option<Value>, SpecError> {
    let mut dialog = JsonDialog::load("examples/dialog_specs/customer.json")?;
    dialog.set_data(start)?;

    // A rule for one field: it runs when the dialog is accepted, after the
    // spec's own checks ("name" is required in the JSON).
    dialog.rule("email", |value| match value.as_str() {
        Some(email) if !email.is_empty() && !email.contains('@') => {
            Err("Email must contain @".into())
        }
        _ => Ok(()),
    });

    // A rule across fields, once each field is valid on its own.
    dialog.validate_data(|data, errors| {
        if data["kind"] == json!(1) && data["email"] == json!("") {
            errors.add("email", "Wholesale customers need an email");
        }
        if data["kind"] == json!(1) && data["vip"] == json!(true) {
            errors.add("vip", "Only retail customers can be VIPs");
        }
    });

    // The Lookup button ("command": "lookup"): fill the form from what is
    // known about the name typed, and stay open.
    dialog.on("lookup", |ctx| {
        let name = ctx.value("name").unwrap_or_default();
        let name = name.as_str().unwrap_or_default().to_lowercase();
        match KNOWN
            .iter()
            .find(|(known, _, _)| known.to_lowercase().starts_with(&name) && !name.is_empty())
        {
            Some((full, email, kind)) => {
                ctx.set_data(&json!({ "name": full, "email": email, "kind": kind }))
                    .expect("these names and types match the dialog");
            }
            None => {
                message_box_ok(ctx.app, "No customer by that name.");
            }
        }
        Flow::Stay
    });

    // The kind radio buttons ("on_change": "kind_changed"): only retail
    // customers can be VIPs, so choosing wholesale clears the box. The
    // control itself is reached by name, as its turbo-vision type.
    dialog.on("kind_changed", |ctx| {
        if ctx.value("kind") == Some(json!(1))
            && let Some(vip) = ctx.view_mut::<CheckBox>("vip")
        {
            vip.set_checked(false);
        }
        Flow::Stay
    });

    // Every action and custom validator the JSON names is bound.
    debug_assert!(dialog.unbound().is_empty(), "{:?}", dialog.unbound());

    Ok(dialog.run(app).map(|answer| answer.data))
}

fn main() -> turbo_vision::core::error::Result<()> {
    let mut app = Application::new()?;
    let start = json!({ "name": "", "country": 0, "kind": 0, "vip": false });
    let outcome = edit_customer(&mut app, &start);
    drop(app); // give the terminal back before printing
    match outcome {
        Ok(Some(data)) => println!(
            "{}",
            serde_json::to_string_pretty(&data).unwrap_or_default()
        ),
        Ok(None) => println!("cancelled"),
        Err(e) => eprintln!("{e}"),
    }
    Ok(())
}
