// (C) 2026 - Antoni Aloy

//! The dialog tutorial's finished program (`docs/dialog-tutorial.md`): an
//! order form loaded from JSON, with events, an action and rules.
//!
//! ```sh
//! cargo run --example dialog_order --features native,dialog
//! ```
//!
//! Pick an item and a quantity and the total follows; Clear resets the form;
//! OK refuses a gift with no note. `cargo test --example dialog_order
//! --features native,dialog` drives the same dialog with keystrokes.

use serde_json::json;
use turbo_vision::app::Application;
use tv_extensions::dialog_spec::{ActionContext, Flow, JsonDialog};

/// What one of each item costs, in the order of the `item` combo box.
const PRICES: [i64; 3] = [2, 5, 3];

/// The form as the JSON draws it, with its behaviour bound by name.
fn order_dialog() -> JsonDialog {
    let mut dialog =
        JsonDialog::load("examples/dialog_specs/order.json").expect("order.json is a valid dialog");

    // Starting values.
    dialog
        .set_data(&json!({ "qty": 1, "total": "2" }))
        .expect("the names exist and the values fit");

    // EVENTS: the total follows the item and the quantity.
    dialog.on_change("item", recompute);
    dialog.on_change("qty", recompute);

    // ACTION: the Clear button puts the form back and stays open.
    dialog.on("clear", |ctx| {
        let _ = ctx.set_data(&json!({
            "item": 0, "qty": 1, "total": "2", "gift": false, "note": ""
        }));
        Flow::Stay
    });

    // RULES, checked when OK is pressed.
    dialog.rule("note", |value| match value.as_str() {
        Some(note) if note.chars().count() > 30 => Err("Keep the note under 30 characters".into()),
        _ => Ok(()),
    });
    dialog.validate_data(|data, errors| {
        if data["gift"] == json!(true) && data["note"] == json!("") {
            errors.add("note", "A gift needs a note");
        }
    });

    // Everything the JSON names is bound.
    debug_assert!(dialog.unbound().is_empty(), "{:?}", dialog.unbound());
    dialog
}

/// Work the total out from the item and the quantity.
fn recompute(ctx: &mut ActionContext<'_>) -> Flow {
    let item = ctx
        .value("item")
        .and_then(|v| v.as_u64())
        .and_then(|i| usize::try_from(i).ok())
        .unwrap_or(0);
    let qty = ctx.value("qty").and_then(|v| v.as_i64()).unwrap_or(1);
    let total = PRICES.get(item).copied().unwrap_or(0) * qty;
    let _ = ctx.set_value("total", &json!(total.to_string()));
    Flow::Stay
}

fn main() -> turbo_vision::core::error::Result<()> {
    let mut app = Application::new()?;
    let mut dialog = order_dialog();
    let answer = dialog.run(&mut app);
    drop(app); // give the terminal back before printing
    match answer {
        Some(answer) => println!("{}: {}", answer.action, answer.data),
        None => println!("cancelled"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use turbo_vision::core::event::{Event, KB_ALT_C, KB_ALT_O, KB_ESC, KB_TAB, KB_UP};
    use turbo_vision::test_util::test_terminal;
    use turbo_vision::views::button::set_press_animation;

    fn app() -> Application {
        set_press_animation(std::time::Duration::ZERO);
        Application::with_terminal(test_terminal(100, 40))
    }

    /// Queue keys for the dialog to read when it runs.
    fn keys(app: &mut Application, codes: &[u16]) {
        let tx = app.terminal.event_injector();
        for &code in codes {
            tx.send(Event::keyboard(code)).unwrap();
        }
    }

    #[test]
    fn the_json_is_valid_and_everything_it_names_is_bound() {
        assert!(order_dialog().unbound().is_empty());
    }

    #[test]
    fn the_total_follows_the_quantity() {
        let mut app = app();
        let mut dialog = order_dialog();
        // Tab to the quantity, step it up once, accept.
        keys(&mut app, &[KB_TAB, KB_UP, KB_ALT_O]);
        let answer = dialog.run(&mut app).expect("accepted");
        assert_eq!(answer.action, "ok");
        assert_eq!(answer.data["qty"], json!(2));
        assert_eq!(answer.data["total"], json!("4"));
    }

    #[test]
    fn clear_puts_the_form_back_and_stays_open() {
        let mut app = app();
        let mut dialog = order_dialog();
        keys(&mut app, &[KB_TAB, KB_UP, KB_UP, KB_ALT_C, KB_ALT_O]);
        let answer = dialog.run(&mut app).expect("accepted");
        assert_eq!(answer.data["qty"], json!(1));
        assert_eq!(answer.data["total"], json!("2"));
    }

    #[test]
    fn a_gift_with_no_note_is_refused() {
        let mut app = app();
        let mut dialog = order_dialog();
        // Tab to Gift (item, qty, total, gift) and Space ticks it. OK is refused by the rule, so the dialog is
        // still open when Esc comes.
        keys(&mut app, &[KB_TAB, KB_TAB, KB_TAB]);
        app.terminal
            .event_injector()
            .send(Event::text(' '))
            .unwrap();
        keys(&mut app, &[KB_ALT_O, KB_ESC]);
        assert!(dialog.run(&mut app).is_none());
    }
}
