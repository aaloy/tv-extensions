// (C) 2026 - Antoni Aloy

//! Reacting while the dialog is open: `on_change` names an action, and the
//! code bound to that name runs each time the person changes the control.
//!
//! ```sh
//! cargo run --example dialog_on_change --features native,dialog
//! ```
//!
//! `examples/dialog_specs/volume.json` only names its controls (`mode`,
//! `volume`, `meter`); `on_change(name, ...)` runs code when the control of
//! that name changes, whatever its type. The same call works for an input,
//! a list, a check box...

use serde_json::json;
use tv_extensions::dialog_spec::{Flow, JsonDialog};

fn main() -> turbo_vision::core::error::Result<()> {
    let mut app = turbo_vision::app::Application::new()?;
    let mut dialog = match JsonDialog::load("examples/dialog_specs/volume.json") {
        Ok(dialog) => dialog,
        Err(e) => {
            drop(app);
            eprintln!("{e}");
            return Ok(());
        }
    };

    // The slider moved: the meter follows it.
    dialog.on_change("volume", |ctx| {
        if let Some(level) = ctx.value("volume") {
            let _ = ctx.set_value("meter", &level);
        }
        Flow::Stay
    });

    // The mode changed: set the slider and the meter to its usual level.
    dialog.on_change("mode", |ctx| {
        let level = if ctx.value("mode") == Some(json!(1)) {
            80
        } else {
            20
        };
        let _ = ctx.set_value("volume", &json!(level));
        let _ = ctx.set_value("meter", &json!(level));
        Flow::Stay
    });

    let answer = dialog.run(&mut app);
    drop(app); // give the terminal back before printing
    match answer {
        Some(answer) => println!(
            "{}",
            serde_json::to_string_pretty(&answer.data).unwrap_or_default()
        ),
        None => println!("cancelled"),
    }
    Ok(())
}
