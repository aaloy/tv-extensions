# Tutorial: from a JSON file to a working dialog

In this tutorial you build an order form. You draw it in a JSON file, show it
from Rust, and then give it behaviour: the total follows the item and the
quantity (**events**), a Clear button resets the form (an **action**), and a
gift with no note is refused (**rules**). At the end you test it without a
terminal.

The finished program is [`examples/dialog_order.rs`](../examples/dialog_order.rs)
with its dialog in
[`examples/dialog_specs/order.json`](../examples/dialog_specs/order.json).
Run it first to see where you are going:

```sh
cargo run --example dialog_order --features native,dialog
```

Pick an item, step the quantity with the up and down arrows, tick Gift and
press OK with no note. Press Clear. Press OK to see the data it returns.

You need only what the crate already depends on. For the reference to the
JSON format (every control and key) see [dialog-json.md](dialog-json.md).

## The idea

A dialog has two halves, joined by **names**:

- the JSON says *what the form looks like*: its controls, places and
  captions, and gives a `name` to every control your code will read or
  react to;
- the Rust says *what the form does*: it loads the JSON, then attaches code
  to those names.

Nothing in the JSON is code. The only link between the two is a name.

## 1. Draw the form

Create `order.json`:

```json dialog
{
  "version": 1,
  "title": "Order",
  "width": 56,
  "height": 16,
  "controls": [
    { "type": "label", "x": 2, "y": 1, "w": 8, "h": 1, "text": "~I~tem", "link": "item" },
    { "type": "combo_box", "name": "item", "x": 11, "y": 1, "w": 24, "h": 1, "items": ["Pen", "Ink", "Paper"], "selected": 0 },
    { "type": "label", "x": 2, "y": 3, "w": 8, "h": 1, "text": "~Q~ty", "link": "qty" },
    { "type": "spinner", "name": "qty", "x": 11, "y": 3, "w": 10, "h": 1, "min": 1, "max": 99, "value": 1 },
    { "type": "label", "x": 2, "y": 5, "w": 8, "h": 1, "text": "Total" },
    { "type": "input", "name": "total", "x": 11, "y": 5, "w": 12, "h": 1, "hint": "Worked out from the item and the quantity" },
    { "type": "check_box", "name": "gift", "x": 11, "y": 7, "w": 20, "h": 1, "text": "~G~ift" },
    { "type": "label", "x": 2, "y": 9, "w": 8, "h": 1, "text": "~N~ote", "link": "note" },
    { "type": "input", "name": "note", "x": 11, "y": 9, "w": 40, "h": 1 },
    { "type": "error_line", "x": 2, "y": 11, "w": 50, "h": 1 },
    { "type": "button", "x": 4, "y": 12, "w": 12, "h": 2, "text": "~C~lear", "command": "clear" },
    { "type": "button", "x": 20, "y": 12, "w": 10, "h": 2, "text": "~O~K", "command": "ok", "default": true },
    { "type": "button", "x": 34, "y": 12, "w": 12, "h": 2, "text": "Cancel", "command": "cancel" }
  ]
}
```

What matters here:

- **The frame and places.** `width` and `height` are the outer size, frame
  included. Each control's `x`, `y` is its top-left cell counted from just
  *inside* the frame; `w` and `h` are its size in cells. Fields sit on every
  other row (`y` = 1, 3, 5, ...) with the label at `x: 2` and the field at
  `x: 11`.
- **Names are the join.** `item`, `qty`, `total`, `gift` and `note` are the
  keys of the data the dialog returns, and the names your code uses. The
  labels have no name: nobody reaches them. A label's `link` names the control
  it captions, so Alt+I focuses the item.
- **A `~` marks a hot key.** `"~Q~ty"` is Alt+Q.
- **The buttons.** `ok` and `cancel` are built in. `"command": "clear"` is
  neither, so it is an **action**: a name your code will bind.
- **The `error_line`.** Where the form shows what is wrong when OK is refused.
- **Tab order** is the order of `controls`.

You can draw the same file in the designer instead:
`cargo run --example dialog_designer --features native,dialog -- order.json`.

## 2. Check it

Before writing any Rust, ask the checker:

```sh
cargo run --example dialog_check --features native,dialog -- order.json --render
```

```text
ok: "Order" 56x16, 13 controls

data returned when accepted (the starting values):
{
  "item": 0,
  "qty": 1,
  "total": "",
  "gift": false,
  "note": ""
}

your code binds: action "clear"

no warnings

╔═[■]════════════════════ Order ═══════════════════════╗
║                                                      ║
║  Item      Pen                   ▼                   ║
║                                                      ║
║  Qty             1▲▼                                 ║
║                                                      ║
║  Total                                               ║
║                                                      ║
║           [ ] Gift                                   ║
║                                                      ║
║  Note                                                ║
║                                                      ║
║                                                      ║
║       Clear   ▀       OK    ▀      Cancel   ▀        ║
║     ▄▄▄▄▄▄▄▄▄▄▄     ▄▄▄▄▄▄▄▄▄     ▄▄▄▄▄▄▄▄▄▄▄        ║
╚══════════════════════════════════════════════════════╝
```

It tells you three useful things: the **data** the dialog will return (one
entry per named control, in the form's order), the **names your code must
bind** (`action "clear"`), and any layout **warnings** (a control that does
not fit, overlaps, a button too narrow). A file with a mistake prints
`error: ...` naming the control and the key, and exits with status 1.

Change something and run it again until the picture is right.

## 3. Show it

The smallest program that runs the dialog:

```rust,ignore
use turbo_vision::app::Application;
use tv_extensions::dialog_spec::JsonDialog;

fn main() -> turbo_vision::core::error::Result<()> {
    let mut app = Application::new()?;
    let mut dialog = JsonDialog::load("order.json").expect("order.json is a valid dialog");
    let answer = dialog.run(&mut app);   // None when cancelled
    drop(app);                           // give the terminal back before printing
    match answer {
        Some(answer) => println!("{}: {}", answer.action, answer.data),
        None => println!("cancelled"),
    }
    Ok(())
}
```

`run` shows the dialog and returns when the person closes it: `None` if they
cancelled (Cancel, Esc, the close box), otherwise an `Answer` with `action`
(`"ok"`) and `data`, the object you saw in step 2 filled with what they chose
and typed. This much already works; everything after is optional behaviour.

## 4. Start values

Fill the form before showing it with `set_data`: any part of the data, by
name. It refuses names that are not controls and values of the wrong type.

```rust,ignore
dialog
    .set_data(&json!({ "qty": 1, "total": "2" }))
    .expect("the names exist and the values fit");
```

## 5. Events: the total follows the form

An **event** is the person changing a control while the dialog is open.
`on_change(name, handler)` runs the handler each time the control called
`name` changes, whatever its type. Nothing is needed in the JSON but the name.

The handler gets a `ctx`: it reads any control by name with `ctx.value(name)`
and changes one with `ctx.set_value(name, &json)`. It returns a `Flow`; `Stay`
keeps the dialog open.

```rust,ignore
use serde_json::json;
use tv_extensions::dialog_spec::{ActionContext, Flow, JsonDialog};

/// What one of each item costs, in the order of the `item` combo box.
const PRICES: [i64; 3] = [2, 5, 3];

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

dialog.on_change("item", recompute);
dialog.on_change("qty", recompute);
```

One function serves two controls. A combo box's value is the index of the
chosen item, a spinner's is a number: the value of each control type is in
the [JSON reference](dialog-json.md). A change the *handler* makes
(`set_value("total", ...)`) is not counted as a change by the person, so a
handler can set any control without starting a loop.

## 6. An action: the Clear button

A button whose `command` is not `ok`, `cancel`, `yes` or `no` is an
**action**. Bind code to its name with `on`:

```rust,ignore
dialog.on("clear", |ctx| {
    let _ = ctx.set_data(&json!({
        "item": 0, "qty": 1, "total": "2", "gift": false, "note": ""
    }));
    Flow::Stay
});
```

What the handler returns says what happens next:

| `Flow` | The dialog |
|--------|------------|
| `Stay` | stays open: the action did its work (here, reset the form) |
| `Submit` | validates as OK does, and closes if everything passes |
| `Close` | closes at once, without validating ("Discard") |

The handler can also show a message box or open another dialog: `ctx.app` is
the application. A button with **no** handler still works: it validates and
closes like OK, and `answer.action` tells you which button it was. A dialog
with "Save" and "Save and new" buttons needs no handlers at all: read
`answer.action` afterwards.

## 7. Rules

The JSON already has some checks (`required`, `min`, `max`, validators). For
what it cannot say, add **rules**. They run when OK is pressed.

A rule for one control:

```rust,ignore
dialog.rule("note", |value| match value.as_str() {
    Some(note) if note.chars().count() > 30 => Err("Keep the note under 30 characters".into()),
    _ => Ok(()),
});
```

A rule across controls, run once each is valid on its own:

```rust,ignore
dialog.validate_data(|data, errors| {
    if data["gift"] == json!(true) && data["note"] == json!("") {
        errors.add("note", "A gift needs a note");
    }
});
```

`errors.add(name, message)` blames a control. When anything fails, the dialog
stays open: the label of the control at fault turns red, the message shows on
the error line, and focus goes to the control. The errors clear as they are
fixed.

## 8. Nothing left unbound

`dialog.unbound()` lists what the JSON names that no code has bound, such as
`action "clear"`. Assert it is empty so a renamed button cannot go unnoticed:

```rust,ignore
debug_assert!(dialog.unbound().is_empty(), "{:?}", dialog.unbound());
```

## 9. The whole program

Put the pieces together. This is `examples/dialog_order.rs`, which `cargo run`
starts:

```rust,final
fn order_dialog() -> JsonDialog {
    let mut dialog =
        JsonDialog::load("examples/dialog_specs/order.json").expect("order.json is a valid dialog");

    dialog
        .set_data(&json!({ "qty": 1, "total": "2" }))
        .expect("the names exist and the values fit");

    dialog.on_change("item", recompute);
    dialog.on_change("qty", recompute);

    dialog.on("clear", |ctx| {
        let _ = ctx.set_data(&json!({
            "item": 0, "qty": 1, "total": "2", "gift": false, "note": ""
        }));
        Flow::Stay
    });

    dialog.rule("note", |value| match value.as_str() {
        Some(note) if note.chars().count() > 30 => Err("Keep the note under 30 characters".into()),
        _ => Ok(()),
    });
    dialog.validate_data(|data, errors| {
        if data["gift"] == json!(true) && data["note"] == json!("") {
            errors.add("note", "A gift needs a note");
        }
    });

    debug_assert!(dialog.unbound().is_empty(), "{:?}", dialog.unbound());
    dialog
}
```

`main` then loads it with `order_dialog()` and calls `run`, as in step 3. It
is a function of its own so that tests can build the same dialog.

## 10. Test it without a terminal

A dialog can be driven by queued keystrokes on a headless terminal, so its
behaviour is tested like any other code. This is how `dialog_order.rs`
checks its own events, action and rule:

```rust,ignore
use turbo_vision::core::event::{Event, KB_ALT_O, KB_TAB, KB_UP};
use turbo_vision::test_util::test_terminal;   // needs turbo-vision's `test-util` feature

#[test]
fn the_total_follows_the_quantity() {
    let mut app = Application::with_terminal(test_terminal(100, 40));
    let mut dialog = order_dialog();
    // Tab to the quantity, step it up once, accept.
    let tx = app.terminal.event_injector();
    for code in [KB_TAB, KB_UP, KB_ALT_O] {
        tx.send(Event::keyboard(code)).unwrap();
    }
    let answer = dialog.run(&mut app).expect("accepted");
    assert_eq!(answer.data["qty"], json!(2));
    assert_eq!(answer.data["total"], json!("4"));
}
```

Run the example's tests with
`cargo test --example dialog_order --features native,dialog`. The three
behaviours have a test each: the total follows the quantity, Clear resets the
form, a gift with no note is refused. For the last, the test ticks Gift and
presses OK; the rule refuses, so the dialog is still open when Esc cancels
it.

## Where next

- Draw the next dialog in the designer (`dialog_designer`), where F1 and
  Help > Tutorial explain each control and how to react to it.
- Look up any control, key or value type in [dialog-json.md](dialog-json.md).
- Validators of your own, tab pages, group boxes, tables and trees are covered
  in [dialog-designer.md](dialog-designer.md) and the examples
  `dialog_from_json.rs` (an action that fills the form, rules) and
  `dialog_on_change.rs` (events that drive other controls).
