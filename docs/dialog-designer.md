# Dialog designer and JSON dialogs

Lay out a Turbo Vision dialog with the mouse, try it, and save it as JSON;
then load the JSON in an application and give it its validation rules and
actions in code. The designer is in the spirit of TV Dialog Design, with
every control of turbo-vision core plus this crate's `AutoComplete` and
`DualList`, tab pages, group boxes and an error line.

Two parts, feature `dialog`:

- **The designer**, `examples/dialog_designer`: a Turbo Vision application
  that edits a dialog spec.
- **The runtime**, `tv_extensions::dialog_spec`: `DialogSpec`, the JSON
  file, and `JsonDialog`, the live dialog built from it.

```toml
[dependencies]
tv-extensions = { version = "0.3", features = ["dialog"] }
```

## The designer

```sh
cargo run --example dialog_designer --features native,dialog -- customer.json
```

The file is opened if it exists and created on the first save if not.

```text
  File  Edit  Insert  Dialog  View  Help
░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░╔═[■]══════ Inspector ═════[▲]═╗░░
░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░║(not saved)                   ║░░
░░░╔═[■]═════════════════ Customer ═══════════════════╗░║                             ▲║░░
░░░║                                                  ║░║input "email"                ░║░░
░░░║  Name                                            ║░║                             ░║░░
░░░║                                                  ║░║{                            ░║░░
░░░║  Email                                           ║░║  "type": "input",           ░║░░
░░░║                                                  ║░║  "name": "email",           ░║░░
░░░║  Country   Spain             ▼                   ║░║  "x": 11,                   ░║░░
░░░║                                                  ║░║  "y": 3,                    ░║░░
░░░║  Kind     (•) Retail          [ ] VIP customer   ║░║  "w": 36,                   ░║░░
░░░║           ( ) Wholesale                          ║░║  "h": 1,                    ░║░░
░░░║                                                  ║░║  "hint": "Where invoices are░║░░
░░░║                                                  ║░║  "max_len": 80              ░║░░
░░░║    Lookup   ▀              OK    ▀   Cancel   ▀  ║░║}                            ░║░░
░░░║   ▄▄▄▄▄▄▄▄▄▄▄           ▄▄▄▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄▄▄  ║░║                             ░║░░
░░░║                                                  ║░║                             ░║░░
░░░╚══════════════════════════════════════════════════╝░║                             ▼║░░
░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░║◄░░░░░░░░░░░░░░░░░░░░░░░░░░░► ║░░
░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░└══════════════════════════════┘░░
 Alt-X Exit │  F2 Save │  Ins Insert │  F9 Test │  F4 JSON │  Ctrl-O Order │
```

The dialog on the desktop is drawn by its real controls, built from the
spec exactly as an application builds it, so what you see is what runs. The
selected control (here the email input) is highlighted in cyan, and its
bottom-right corner, in yellow, is its resize handle. The inspector on the
right shows the selection as JSON (View > Inspector hides it).

### Mouse and keys

| Do | Mouse | Keys |
|----|-------|------|
| Select a control | click it | Tab / Shift+Tab, in tab order |
| Select the container | click empty space | Esc (a page's control → its tabs → the dialog) |
| Move it | drag it | arrows |
| Resize it | drag its yellow corner | Shift+arrows or Ctrl+arrows |
| Resize the dialog | drag the frame's bottom-right corner | select the dialog, then Shift+arrows |
| Insert a control | right click | Ins, or the Insert menu |
| Edit its properties | double click | Enter |
| Delete / duplicate | | Del / Ctrl+D |
| Earlier / later in tab order | | F7 / F8 |
| Show the tab order | | Ctrl+O |
| Show another tab page | click its tab | Ctrl+PgUp / Ctrl+PgDn |
| Undo / redo | | Ctrl+Z / Ctrl+Y |
| Test the dialog | | F9 |
| Show the JSON | | F4 |
| Save code to use it... | | Ctrl+G |
| Save / open / new | | F2 / F3 / Ctrl+N |

A drag is one undo step. Moves and resizes keep a control inside its
container, the dialog or its tab page.

### Inserting

A new control goes after the selected one in tab order, just below it, and
gets a name of its type (`input1`, `combo2`). Two shortcuts follow how
dialogs are built:

- **After a label**, a control with a value goes to the label's right on the
  same row, and the label is linked to it: insert a label, then an input,
  and you have a captioned field whose Alt hot key works.
- **With a tab pane selected** (or a control on one), the new control goes
  on the page being shown. Click a tab, or press Ctrl+PgUp/PgDn, to fill
  another page.

Buttons come as OK first (the default button), then Cancel, then actions
named `action1`, `action2`, …

### Properties

Enter opens the selected control's property sheet; with the dialog
selected, the dialog's (title, size, resizable, centred or placed). A sheet
has a **General** page for the control's own settings, a **Place** page for
its name, position, size and hover hint, and, for an input, a
**Validator** page. Lists (items, tab pages, table columns and rows) are
edited one per line. Renaming a control carries along the labels linked to
it; a sheet that would break a rule (a duplicate name, a missing action
name) says why and opens again with what you typed.

### Testing

F9 runs the dialog as an application would: type, tab, press its buttons.
Its validation works (required fields, validators, the error line), and when
it closes the designer shows how it closed and the data it returned. Actions
no code has bound yet close the dialog like OK, and the report lists them,
with any custom validators the spec names.

### Using it from code

Ctrl+G asks, in a file dialog, for a folder and a name, and writes Rust
that loads the file and binds every action and custom validator it names,
with a sample field rule: add it to the application and fill in the bodies.

## The JSON

```json
{
  "version": 1,
  "title": "Customer",
  "width": 52,
  "height": 16,
  "controls": [
    { "type": "label", "x": 2, "y": 1, "w": 8, "h": 1, "text": "~N~ame", "link": "name" },
    { "type": "input", "name": "name", "x": 11, "y": 1, "w": 36, "h": 1, "max_len": 60, "required": true },
    { "type": "radio_buttons", "name": "kind", "x": 11, "y": 7, "w": 18, "h": 2,
      "items": ["~R~etail", "~W~holesale"], "on_change": "kind_changed" },
    { "type": "error_line", "x": 2, "y": 10, "w": 46, "h": 1 },
    { "type": "button", "x": 2, "y": 11, "w": 12, "h": 2, "text": "~L~ookup", "command": "lookup" },
    { "type": "button", "x": 24, "y": 11, "w": 11, "h": 2, "text": "~O~K", "default": true },
    { "type": "button", "x": 36, "y": 11, "w": 12, "h": 2, "text": "Cancel", "command": "cancel" }
  ]
}
```

The dialog has a `title`, `width` and `height` (its outer size, frame
included), optional `x` and `y` on the desktop (centred when absent),
`resizable`, and its `controls` in tab order. Each control is one flat
object: `type`, an optional `name`, its place `x`, `y`, `w`, `h` measured
from the corner inside the frame (or inside its tab page), an optional
`hint`, then the keys of its type. A `~` in a caption marks its Alt hot key.

| `type` | Keys (`= default`) | Value in the data |
|--------|-----------------------------|-------------------|
| `label` | `text`, `link` (a control's name) | — |
| `static_text` | `text`, `centered` | — |
| `button` | `text`, `command` (`"ok"`; or `"cancel"`, `"yes"`, `"no"`, or an action name), `default` | — |
| `input` | `text`, `max_len = 255`, `required`, `validator`, `history` (an id) | string |
| `memo` | `text`, `max_len`, `required`, `read_only` | string |
| `check_box` | `text`, `checked`, `required` (must be checked) | `true`/`false` |
| `check_boxes` | `items`, `checked` (indices), `on_change` | list of checked indices |
| `radio_buttons` | `items`, `selected = 0`, `on_change` | index |
| `combo_box` | `items`, `selected`, `required`, `on_change` | index or `null` |
| `list_box` | `items`, `selected`, `multi` | index or `null`; list of indices when `multi` |
| `spinner` | `min = 0`, `max`, `value`, `step = 1`, `suffix`, `wrap`, `on_change` | number |
| `slider` | `min = 0`, `max`, `value`, `step = 1`, `on_change` | number |
| `progress_bar` | `max = 100`, `value`, `show_percent = true` | number |
| `table` | `columns` (`title`, `width`, `align`), `rows`, `header = true`, `separators` | selected row or `null` |
| `group_box` | `title`: a frame drawn around the controls listed after it | — |
| `tabs` | `pages` (each a `title` and its `controls`), `active` | — |
| `auto_complete` | `items`, `text`, `max_len`, `min_chars = 1`, `required`, `require_match` | string |
| `dual_list` | `items` (strings, or `{"key", "label"}`), `chosen` (keys), `titles`, `min`, `max` | list of chosen keys |
| `line` | `vertical`, `double` (both default false): a rule of line characters along its width, or height if vertical | — |
| `error_line` | none: validation messages show here | — |

An input's `validator` checks keystrokes as they are typed:

| Validator | Accepts |
|-----------|---------|
| `{"type": "range", "min": 1, "max": 99}` | a whole number in the range |
| `{"type": "filter", "chars": "0123456789"}` | only these characters |
| `{"type": "picture", "mask": "##/##/####"}` | a picture: `#` digit, `@` letter, `&` upper-case, `!` any; other characters are filled in |
| `{"type": "lookup", "values": ["EUR", "USD"]}` | one of the values |
| `{"type": "custom", "name": "iban"}` | whatever the validator your code registers as `iban` accepts |

Unknown types and keys are refused, so a typo in a hand-edited file is an
error rather than silently ignored ("input \"a\": unknown field `maxlen`,
expected one of `text`, `max_len`, ..."). Names are unique across the whole
dialog, tab pages included, labels link to controls that exist, and tabs do
not hold other tabs. Values left at their defaults are left out when saved,
and `version` lets a later format tell old files apart.

## Using a designed dialog

```rust,ignore
use serde_json::json;
use tv_extensions::dialog_spec::JsonDialog;

let mut dialog = JsonDialog::load("customer.json")?;
dialog.set_data(&json!({ "name": "Ada", "kind": 1 }))?;
match dialog.run(&mut app) {
    Some(answer) => save(answer.action, answer.data),
    None => {} // cancelled
}
```

`run` returns `None` when the person cancels (Cancel, Esc, the close box),
otherwise an `Answer`: `action` is `"ok"`, `"yes"`, `"no"`, or the action
that closed the dialog, and `data` is every named control's value, one flat
object in the spec's order:

```json
{ "name": "Ada", "email": "ada@example.org", "country": 0, "kind": 1, "vip": false }
```

`set_data` takes the same shape, or any part of it; it refuses names that
are not controls and values of the wrong type. `value` and `set_value` do
one control. The same `JsonDialog` can run again and again.

To embed the JSON in the program instead of reading a file:

```rust,ignore
let mut dialog = JsonDialog::from_json(include_str!("customer.json"))?;
```

`examples/dialog_from_json.rs` is a complete program around
`examples/dialog_specs/customer.json`.

## Validation

When the dialog is accepted, each named control is checked in tab order:
first the spec's own rules (`required`, an input's `validator`, a check box
that must be checked), then the rules you bind to it; once every control
passes, the rules over all the data. If anything fails the dialog stays
open: the labels of the controls at fault turn red, the error line shows the
focused control's message (or the first, "+2 more"), and the focus moves to
the first control at fault, bringing its tab page forward. The errors are
checked again as the focus moves, so they clear as they are fixed. With no
error line, the first message is shown in a message box.

### A rule for one control

```rust,ignore
dialog.rule("email", |value| match value.as_str() {
    Some(e) if !e.is_empty() && !e.contains('@') => Err("Email must contain @".into()),
    _ => Ok(()),
});
```

The rule gets the control's value as JSON (the type in the table above) and
returns the message to show. It runs only when the spec's own checks pass
for that control, so a required field left blank says "Email is required",
not your message.

### A rule across controls

```rust,ignore
dialog.validate_data(|data, errors| {
    if data["kind"] == json!(1) && data["email"] == json!("") {
        errors.add("email", "Wholesale customers need an email");
    }
    if data["start"].as_i64() > data["end"].as_i64() {
        errors.add_form("The start comes after the end");
    }
});
```

`errors.add(name, message)` blames a control (its label turns red, the
focus goes to it); `errors.add_form(message)` blames the dialog as a whole.

### A validator of your own

A validator checks the text as it is typed, like the built-in ones. Write
it against turbo-vision's `Validator` trait, refer to it by name in the
JSON (`"validator": {"type": "custom", "name": "iban"}`), and register it:

```rust,ignore
use std::{cell::RefCell, rc::Rc};
use turbo_vision::views::validator::Validator;

struct Iban;

impl Validator for Iban {
    /// The whole value, checked when the dialog is accepted.
    fn is_valid(&self, input: &str) -> bool {
        let s: String = input.chars().filter(|c| !c.is_whitespace()).collect();
        s.len() >= 15 && s[..2].chars().all(|c| c.is_ascii_uppercase()) && checksum_ok(&s)
    }
    /// Each keystroke: let partial input through, refuse impossible characters.
    fn is_valid_input(&self, input: &str, _append: bool) -> bool {
        input.chars().all(|c| c.is_ascii_alphanumeric() || c == ' ')
    }
    fn error(&self) {}
}

dialog.validator("iban", Rc::new(RefCell::new(Iban)));
```

Override `is_valid_input`: by default it is `is_valid`, which would refuse
every keystroke of a value that is only valid once complete. A value the
validator refuses is reported as "IBAN is not valid", named after the
input's label.

### Checking without running

`dialog.validate()` runs every check without running the dialog and returns
the data or the errors; `dialog.show_errors(&errors)` shows errors in the
dialog. Both are handy in tests and for code that fills a dialog itself.

## Actions

A button whose `command` is not `ok`, `cancel`, `yes` or `no` names an
action; so does a control's `on_change`. Bind code to an action with `on`:

```rust,ignore
use tv_extensions::dialog_spec::Flow;

dialog.on("lookup", |ctx| {
    let name = ctx.value("name").unwrap_or_default();
    match find_customer(name.as_str().unwrap_or_default()) {
        Some(c) => ctx.set_data(&json!({ "email": c.email, "kind": c.kind })).unwrap(),
        None => {
            message_box_ok(ctx.app, "No customer by that name.");
        }
    }
    Flow::Stay
});
```

The handler gets an `ActionContext`: `value`, `set_value`, `data`,
`set_data`, `view_mut` for any control by name, `action()` for the action's
name, and `app` for message boxes and other dialogs. What it returns decides
what happens next:

| `Flow` | The dialog |
|--------|------------|
| `Stay` | stays open: the action did its work |
| `Submit` | validates as OK does, and closes if everything passes |
| `Close` | closes at once, without validating ("Discard", "Skip") |

An action with no handler validates and closes like OK, and `answer.action`
says which button it was. A dialog with "Save" and "Save and new" buttons
needs no handlers at all:

```rust,ignore
if let Some(answer) = dialog.run(&mut app) {
    store(&answer.data);
    if answer.action == "save_and_new" { /* open it again, empty */ }
}
```

### Reacting to a change

```rust,ignore
// "on_change": "kind_changed" on the radio buttons
dialog.on("kind_changed", |ctx| {
    if ctx.value("kind") == Some(json!(1))
        && let Some(vip) = ctx.view_mut::<CheckBox>("vip")
    {
        vip.set_checked(false);
    }
    Flow::Stay
});
```

`on_change` is available on check boxes, radio buttons, combo boxes,
spinners and sliders. The handler runs as the person changes the control.

### Nothing left unbound

```rust,ignore
assert!(dialog.unbound().is_empty(), "{:?}", dialog.unbound());
```

`unbound` lists every action and custom validator the spec names that no
code has bound, such as `action "lookup"`. In a test, it catches a button
renamed in the designer but not in the code.

## Beyond the JSON

The built dialog is ordinary turbo-vision:

- **Any control's own API**, by name and type:
  `dialog.view_mut::<Table>("orders").unwrap().set_rows(rows)` fills a table
  from a database; `view_mut::<ListBox>`, `view_mut::<ProgressBar>` and the
  rest work the same way, for controls on tab pages too.
- **More views**: `dialog.dialog_mut()` is a plain `Dialog`; add your own.
- **Nothing hidden**: `DialogSpec` is a plain Rust value. Build or change a
  spec in code (`DialogSpec::new`, `Control::new`) and turn it into a dialog
  with `JsonDialog::new(&spec)`, or save it for the designer.

## Testing a designed dialog

Dialogs run on turbo-vision's test terminal, with the keys a person would
press queued up front:

```rust,ignore
use turbo_vision::core::event::{Event, KB_ALT_O};
use turbo_vision::views::button::set_press_animation;

set_press_animation(std::time::Duration::ZERO);
let mut app = Application::with_terminal(turbo_vision::test_util::test_terminal(80, 25));
let keys = app.terminal.event_injector();
for c in "Ada".chars() {
    keys.send(Event::text(c)).unwrap();
}
keys.send(Event::keyboard(KB_ALT_O)).unwrap();

let mut dialog = JsonDialog::load("customer.json").unwrap();
let answer = dialog.run(&mut app).unwrap();
assert_eq!(answer.data["name"], "Ada");
```

`tests/json_dialog.rs` has many more, and the designer itself is tested the
same way (`cargo test --example dialog_designer --features native,dialog`).
