// (C) 2026 - Antoni Aloy

//! Reacting while the dialog is open: `on_change` names an action, and the
//! code bound to that name runs each time the person changes the control.
//!
//! ```sh
//! cargo run --example dialog_on_change --features native,dialog
//! ```
//!
//! In `examples/dialog_specs/volume.json` the slider `volume` has
//! `"on_change": "volume_changed"` and the radio buttons `mode` have
//! `"on_change": "mode_changed"`. In the designer that is just a name typed
//! into the control's "On change action"; the code is below.

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
    dialog.on("volume_changed", |ctx| {
        if let Some(level) = ctx.value("volume") {
            let _ = ctx.set_value("meter", &level);
        }
        Flow::Stay
    });

    // The mode changed: set the slider and the meter to its usual level.
    dialog.on("mode_changed", |ctx| {
        let level = if ctx.value("mode") == Some(json!(1)) {
            80
        } else {
            20
        };
        let _ = ctx.set_value("volume", &json!(level));
        let _ = ctx.set_value("meter", &json!(level));
        Flow::Stay
    });

    // Every action the JSON names is bound.
    debug_assert!(dialog.unbound().is_empty(), "{:?}", dialog.unbound());

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
