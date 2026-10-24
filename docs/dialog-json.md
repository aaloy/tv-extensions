# Dialog JSON: a reference for writing dialogs

This page is a complete, self-contained description of the JSON that
`tv_extensions::dialog_spec` turns into a Turbo Vision dialog. It is written
so that a person, or a coding agent, can write a working dialog file from
nothing but this page, check it, and use it from Rust.

Contents: [the loop](#the-loop) · [a complete example](#a-complete-example) ·
[layout](#layout) · [the dialog](#the-dialog-object) ·
[the controls](#the-controls) · [validators](#validators) ·
[data in and out](#data-in-and-out) · [the Rust side](#the-rust-side) ·
[mistakes](#mistakes-to-avoid) · [recipes](#recipes)

Every block marked `json dialog` or `json control` on this page is checked by
the crate's tests, so what you copy loads.

## The loop

1. **Write** the JSON file.
2. **Check it**: `cargo run --example dialog_check --features native,dialog -- FILE.json --render`
   - A file that does not load prints `error: ...` naming the control and the
     key at fault, and exits with status 1. Fix and run again.
   - A file that loads prints the data it returns, the names your code must
     bind, **warnings** (a control that does not fit, two that overlap, a
     button too narrow, no OK button, a value control with no name) and, with
     `--render`, a text picture of the dialog as it will look.
3. **Look at the picture** and adjust positions until it looks right.
4. **Use it** from Rust (see [the Rust side](#the-rust-side)).

The designer (`cargo run --example dialog_designer --features native,dialog -- FILE.json`)
reads and writes the same files, so a file written by hand can be refined
there, and the other way round.

## A complete example

```json dialog
{
  "version": 1,
  "title": "Contact",
  "width": 56,
  "height": 15,
  "controls": [
    { "type": "label", "x": 2, "y": 1, "w": 8, "h": 1, "text": "~N~ame", "link": "name" },
    { "type": "input", "name": "name", "x": 11, "y": 1, "w": 40, "h": 1, "max_len": 60, "required": true },

    { "type": "label", "x": 2, "y": 3, "w": 8, "h": 1, "text": "~E~mail", "link": "email" },
    { "type": "input", "name": "email", "x": 11, "y": 3, "w": 40, "h": 1, "max_len": 80,
      "hint": "Where we reply" },

    { "type": "label", "x": 2, "y": 5, "w": 8, "h": 1, "text": "~T~opic", "link": "topic" },
    { "type": "combo_box", "name": "topic", "x": 11, "y": 5, "w": 24, "h": 1,
      "items": ["Question", "Bug report", "Idea"], "selected": 0 },

    { "type": "check_box", "name": "copy", "x": 11, "y": 7, "w": 30, "h": 1,
      "text": "Send me a ~c~opy", "checked": true },

    { "type": "error_line", "x": 2, "y": 9, "w": 50, "h": 1 },

    { "type": "button", "x": 14, "y": 10, "w": 10, "h": 2, "text": "~O~K", "command": "ok", "default": true },
    { "type": "button", "x": 28, "y": 10, "w": 12, "h": 2, "text": "Cancel", "command": "cancel" }
  ]
}
```

`dialog_check --render` draws it like this, and `run` returns the data
`{"name": "...", "email": "...", "topic": 0, "copy": true}`:

```text
╔═[■]═══════════════════ Contact ══════════════════════╗
║                                                      ║
║  Name                                                ║
║                                                      ║
║  Email                                               ║
║                                                      ║
║  Topic     Question              ▼                   ║
║                                                      ║
║           [X] Send me a copy                         ║
║                                                      ║
║                                                      ║
║                 OK    ▀      Cancel   ▀              ║
║               ▄▄▄▄▄▄▄▄▄     ▄▄▄▄▄▄▄▄▄▄▄              ║
║                                                      ║
╚══════════════════════════════════════════════════════╝
```

The picture has characters only, no colours: an empty input shows as blank
space where it is (after its label), a button's shadow as `▀` and `▄`, and
the error line as blank until something is wrong.

## Layout

The screen is a grid of character cells. Everything is measured in cells.

- **Dialog size**: `width` and `height` are the **outer** size, frame
  included. The usable area inside the frame is `width - 2` by `height - 2`.
- **Control places**: `x` is the column and `y` the row of a control's
  top-left cell, counted from the top-left cell **inside** the frame (so
  `x: 0, y: 0` is just inside the frame). `w` and `h` are its width and
  height in cells. A control must fit: `x + w <= width - 2` and
  `y + h <= height - 2`.
- **Inside a container**, places are measured from the container's own inner
  corner: for a group box, the cell inside its frame; for a tab page, the
  top-left of the page area under the tab headers. A group box's inner size
  is `w - 2` by `h - 2`; a tab page's is smaller than the tabs control,
  because the headers and the frame take rows and columns (`dialog_check`
  knows the exact area and warns when something does not fit; start inner
  controls at `x: 1, y: 1`).
- **Tab order** is the order of the `controls` array (and of each container's
  list). Put controls in the order a person should Tab through them.

### Conventions that look right

| What | Convention |
|------|------------|
| Rows | One row for a field, one blank row between fields: `y` = 1, 3, 5, ... |
| Label and field | Label at `x: 2, w: 8` (or the longest label + 1), field at `x: 11` on the same `y` |
| Margins | Start at `x: 2`; keep 2 cells free at the right |
| Buttons | `h: 2` (the second row holds the shadow), `w` at least the visible text length + 4 (`10` for "OK", `12` for "Cancel"), side by side with 2 cells between, on the last two rows above the bottom margin |
| Error line | `x: 2`, `w: width - 6`, `h: 1`, just above the buttons |
| Dialog height | Last control's `y + h`, plus 1 free row, plus 2 for the frame |
| Dialog width | Widest control's `x + w`, plus 2 free, plus 2 for the frame; never narrower than the title + 8 |

### Usual sizes

| Control | `w` × `h` |
|---------|-----------|
| `input`, `combo_box`, `auto_complete`, `spinner`, `slider`, `progress_bar` | 20 × 1 (spinner 10 × 1); `h` is always 1 |
| `check_box` | caption length + 4 × 1 |
| `check_boxes`, `radio_buttons` | longest item + 6 × number of items |
| `memo` | 30 × 4 or more |
| `list_box` | 20 × 5 or more |
| `table` | 30 × 6 or more (header and frame included) |
| `tree` | 30 × 8 |
| `dual_list` | 44 × 8 (two lists side by side) |
| `group_box` | 30 × 6 |
| `tabs` | 40 × 10 |
| `button` | text + 4 × 2 |
| `line` | w × 1 horizontal, 1 × h vertical, larger for a rectangle |

## The dialog object

The file is one JSON object. **Unknown keys are errors** (a typo such as
`"maxlen"` is refused, naming the key), so use only the keys below.

| Key | Meaning |
|-----|---------|
| `version` | The format, `1`. Optional; written by the designer. A file with a newer version is refused. |
| `title` | The title in the frame. Default empty. |
| `width`, `height` | **Required.** Outer size, frame included. |
| `x`, `y` | Where the dialog's top-left corner is on the desktop. Centred when absent. |
| `resizable` | The person may resize it. Default `false`. |
| `controls` | The controls, in tab order. Default empty. |

### Rules checked when the file loads

A file that breaks one of these is refused with a message naming the
control:

- `type` is one of the 21 below, and each key belongs to its type;
- a place has all of `x`, `y`, `w`, `h`; sizes are not negative;
- `name`s are **unique across the whole dialog**, tab pages included;
- a label's `link` is the `name` of a control that exists;
- tabs do not hold other tabs (group boxes may nest, and may sit in tabs);
- a `selected`/`checked` index is inside `items` (at most 32 items in a check
  box or radio button group);
- `min` is not above `max` (spinner, slider, range validator);
- a button's `command` is not empty;
- tree node keys are unique, and `selected` is one of them.

## The controls

Every control is **one flat object**:

```json
{ "type": "input", "name": "email", "x": 11, "y": 3, "w": 40, "h": 1, "hint": "Where we reply", "max_len": 80 }
```

| Common key | Meaning |
|------------|---------|
| `type` | **Required.** One of the names below. |
| `name` | How your code finds the control, and the key of its value in the data. Optional: a control with no name is decoration or cannot be reached. **Give every control that has a value a name**, or its value is not returned. Letters, digits and `_` keep names easy to use. |
| `x`, `y`, `w`, `h` | **Required.** Place and size, as above. |
| `hint` | Optional hover text. |

After those come the keys of the type. Below, **bold** keys are required, the
others are optional with the default shown, and *value* is what the control
contributes to the data. A `~` before a letter in a caption makes that
letter an Alt hot key (`"~N~ame"`: Alt+N).

### Text and decoration

**`label`** — a caption. *Value: none.*

| Key | Meaning |
|-----|---------|
| **`text`** | The caption; `~` marks the hot key. |
| `link` | The `name` of the control it captions. Alt + the hot key (or a click) focuses that control, and the label turns red while that control is invalid. Always link a label that names a field. |
| `color` | One of `black`, `blue`, `green`, `cyan`, `red`, `magenta`, `brown`, `light_gray`, `dark_gray`, `light_blue`, `light_green`, `light_cyan`, `light_red`, `light_magenta`, `yellow`, `white`. |

```json control
{ "type": "label", "x": 2, "y": 1, "w": 8, "h": 1, "text": "~N~ame", "link": "name" }
```

(Any `link` must name a control that exists, so the check wraps this
snippet in a dialog with an input called `name`.)

**`static_text`** — fixed text, wrapped to its width; `\n` starts a new
line. *Value: none.* Keys: **`text`**, `centered` (`false`).

```json control
{ "type": "static_text", "x": 2, "y": 1, "w": 40, "h": 2, "text": "Fill in the form.\nFields in red need attention.", "centered": false }
```

**`line`** — a rule or rectangle drawn with line characters. One row tall it
is a horizontal rule, one column wide a vertical one, larger a rectangle with
corners. *Value: none.* Keys: `style` (`single`, `double`, `heavy`, `round`,
`block`; default `single`), `color` (as for a label).

```json control
{ "type": "line", "x": 2, "y": 7, "w": 40, "h": 1, "style": "double" }
```

### Typing a value

**`input`** — one line of text. *Value: a string.*

| Key | Meaning |
|-----|---------|
| `text` | Text shown at first. |
| `max_len` | Most characters it takes; default `255`. |
| `required` | Must not be left blank; default `false`. |
| `validator` | What it accepts as it is typed; see [validators](#validators). |
| `history` | A number (id). Inputs with the same id share a drop-down of earlier entries, added to the right of the field. |

```json control
{ "type": "input", "name": "code", "x": 11, "y": 1, "w": 12, "h": 1, "max_len": 6, "required": true,
  "validator": { "type": "picture", "mask": "@@####" } }
```

**`auto_complete`** — an input that suggests matching items as you type.
*Value: a string.* Keys: **`items`** (the suggestions), `text`, `max_len`
(`255`), `min_chars` (`1`, characters typed before suggestions appear),
`required`, `require_match` (text that is not blank must be one of the
items).

```json control
{ "type": "auto_complete", "name": "city", "x": 11, "y": 1, "w": 24, "h": 1, "items": ["Madrid", "Málaga", "Mallorca"], "require_match": true }
```

**`memo`** — several lines of text. *Value: a string.* Keys: `text` (`\n`
between lines), `max_len`, `required`, `read_only` (shown, not edited).

```json control
{ "type": "memo", "name": "notes", "x": 2, "y": 1, "w": 40, "h": 4 }
```

**`spinner`** — a whole number with steppers. *Value: a number.*

| Key | Meaning |
|-----|---------|
| **`max`** | The highest value (required). |
| `min` | The lowest; default `0`. |
| `value` | Value at first; default `0`. |
| `step` | What one step adds; default `1`. |
| `suffix` | Text after the number, such as `"%"`. |
| `wrap` | Stepping past an end goes to the other; default `false`. |

```json control
{ "type": "spinner", "name": "age", "x": 11, "y": 1, "w": 10, "h": 1, "min": 0, "max": 120, "value": 30, "suffix": " y" }
```

**`slider`** — a whole number on a track. *Value: a number.* Keys: **`max`**,
`min` (`0`), `value` (`0`), `step` (`1`).

```json control
{ "type": "slider", "name": "volume", "x": 11, "y": 1, "w": 30, "h": 1, "min": 0, "max": 100, "value": 40, "step": 5 }
```

### Options

**`check_box`** — one on/off option. *Value: `true`/`false`.* Keys:
**`text`**, `checked` (`false`), `required` (it must be checked: an "I agree"
box).

```json control
{ "type": "check_box", "name": "agree", "x": 2, "y": 1, "w": 30, "h": 1, "text": "I ~a~gree", "required": true }
```

**`check_boxes`** — several on/off options in one control, one per row.
*Value: the list of checked indices, such as `[0, 2]`.* Keys: **`items`**
(captions; at most 32), `checked` (indices checked at first).

```json control
{ "type": "check_boxes", "name": "extras", "x": 2, "y": 1, "w": 24, "h": 3, "items": ["~W~i-Fi", "~P~arking", "~B~reakfast"], "checked": [0] }
```

**`radio_buttons`** — exactly one choice of several, one per row. *Value: the
chosen index.* Keys: **`items`** (at most 32), `selected` (index chosen at
first; default `0`).

```json control
{ "type": "radio_buttons", "name": "kind", "x": 11, "y": 1, "w": 20, "h": 2, "items": ["~R~etail", "~W~holesale"], "selected": 0 }
```

### Choosing from lists

**`combo_box`** — a drop-down list. *Value: an index, or `null` when nothing
is chosen.* Keys: **`items`**, `selected` (none when absent), `required`.

```json control
{ "type": "combo_box", "name": "country", "x": 11, "y": 1, "w": 20, "h": 1, "items": ["Spain", "Italy", "France"], "selected": 0 }
```

**`list_box`** — a scrolling list. *Value: an index or `null`; with `multi`,
a list of indices.* Keys: **`items`**, `selected` (index), `multi` (several
items can be marked with Space or Insert; default `false`).

```json control
{ "type": "list_box", "name": "tags", "x": 2, "y": 1, "w": 24, "h": 5, "items": ["red", "green", "blue"], "multi": true }
```

**`tree`** — expandable nodes, one selected. *Value: the key of the selected
node (the first at first), or `null` for no nodes.* Keys: **`nodes`**,
`selected` (a key).

A node is a string (its own label and key), or an object `{ "label", "key",
"children", "expanded" }`: `label` is required, `key` defaults to the label,
`children` is a list of nodes, `expanded` shows the children at first. **Node
keys are unique across the whole tree.**

```json control
{ "type": "tree", "name": "place", "x": 2, "y": 1, "w": 30, "h": 8,
  "nodes": [ { "label": "Europe", "expanded": true, "children": ["Spain", { "key": "it", "label": "Italy" }] }, "Asia" ],
  "selected": "it" }
```

**`table`** — rows and columns. *Value: the selected row's index, or `null`.*
Keys: **`columns`** (each `{ "title", "width", "align" }`; `align` is `left`
(default) or `right`), `rows` (a list of rows, each a list of strings, one per
column; code usually fills them with `set_rows`), `header` (`true`),
`separators` (lines between columns; `false`).

```json control
{ "type": "table", "name": "orders", "x": 2, "y": 1, "w": 34, "h": 6,
  "columns": [ { "title": "Item", "width": 20 }, { "title": "Qty", "width": 6, "align": "right" } ],
  "rows": [ ["Pen", "2"], ["Ink", "1"] ], "separators": true }
```

**`dual_list`** — two lists, available and chosen, to pick a subset.
*Value: the list of chosen keys.* Keys: **`items`** (strings, or
`{ "key", "label" }`), `chosen` (keys chosen at first), `titles` (a pair of
strings, `["Available", "Chosen"]` by default), `min` (at least this many
chosen), `max` (at most).

```json control
{ "type": "dual_list", "name": "team", "x": 2, "y": 1, "w": 44, "h": 8,
  "items": ["ann", { "key": "bob", "label": "Bob B." }, "cy"], "chosen": ["bob"], "titles": ["People", "On the team"], "min": 1 }
```

### Containers

Controls inside a container are placed from its inner corner and move with
it. They take part in the data like any other.

**`group_box`** — a titled frame. *Value: none.* Keys: `title`, `controls`
(the controls inside; group boxes can nest).

```json control
{ "type": "group_box", "x": 2, "y": 1, "w": 40, "h": 5, "title": "Delivery", "controls": [
    { "type": "radio_buttons", "name": "speed", "x": 2, "y": 1, "w": 20, "h": 2, "items": ["Normal", "Express"] } ] }
```

**`tabs`** — pages of controls, one shown at a time. *Value: none.* Keys:
**`pages`** (each `{ "title", "controls" }`; a `~` in the title is the tab's
Alt hot key), `active` (the page shown at first; `0`). Tabs do not hold other
tabs.

```json control
{ "type": "tabs", "x": 2, "y": 1, "w": 44, "h": 9, "pages": [
    { "title": "~G~eneral", "controls": [ { "type": "input", "name": "city", "x": 1, "y": 1, "w": 24, "h": 1 } ] },
    { "title": "~N~otes", "controls": [ { "type": "memo", "name": "notes", "x": 1, "y": 1, "w": 38, "h": 4 } ] } ] }
```

### Actions and feedback

**`button`** — a push button. *Value: none.*

| Key | Meaning |
|-----|---------|
| **`text`** | The caption; `~` marks the hot key. |
| `command` | `"ok"` (the default), `"cancel"`, `"yes"`, `"no"`, or the **name of an action** your code binds (see [actions](#actions)). |
| `default` | Enter presses it; default `false`. Give one button this. |

`ok` and `yes` validate the dialog first and close only when it passes;
`cancel` and `no` close at once with no data. An action name runs your code
(or, unbound, validates and closes like OK and tells you which button it was).

```json control
{ "type": "button", "x": 14, "y": 10, "w": 10, "h": 2, "text": "~O~K", "command": "ok", "default": true }
```

**`progress_bar`** — progress towards `max`, set by code. *Value: a number.*
Keys: `max` (`100`), `value` (`0`), `show_percent` (`true`).

```json control
{ "type": "progress_bar", "name": "meter", "x": 2, "y": 1, "w": 30, "h": 1, "max": 100, "value": 40 }
```

**`error_line`** — one row where validation messages are shown. *Value:
none; no other keys.* Without one, the first message is shown in a message
box. Put one just above the buttons.

```json control
{ "type": "error_line", "x": 2, "y": 9, "w": 50, "h": 1 }
```

## Validators

An input's `validator` checks what is typed. Its `type` is one of:

| Validator | Accepts |
|-----------|---------|
| `{ "type": "range", "min": 1, "max": 99 }` | a whole number in the range |
| `{ "type": "filter", "chars": "0123456789" }` | only those characters |
| `{ "type": "picture", "mask": "##/##/####" }` | a picture: `#` digit, `@` letter, `&` upper-case letter, `!` any character; other characters are filled in for the person |
| `{ "type": "lookup", "values": ["EUR", "USD"] }` | one of the values |
| `{ "type": "custom", "name": "iban" }` | what the validator your code registers as `iban` accepts |

`required` on the input (or `required` on other controls, `min`/`max` on a
dual list) are checked when the dialog is accepted, with messages such as
"Email is required", named after the control's linked label.

## Data in and out

The dialog's **data** is one JSON object with an entry for every **named**
control that has a value, in spec order, keyed by `name`:

```json
{ "name": "Ada", "email": "ada@example.org", "topic": 1, "copy": true, "extras": [0, 2], "place": "it" }
```

The value of each type is in the control reference above (an index for
choices, a list of indices for `check_boxes` and a multi `list_box`, a list
of keys for `dual_list`, a key for `tree`, `true`/`false` for a check box,
a string for text, a number for spinner, slider and progress bar).

Before running, `set_data` takes the same shape (or any part of it) to fill
the dialog; it refuses names that are not controls and values of the wrong
type. After running, `run` returns the data of the accepted dialog.

## The Rust side

Add the crate with the features it needs (`dialog`, plus `native` for a real
terminal), then:

```rust,ignore
use serde_json::json;
use turbo_vision::app::Application;
use tv_extensions::dialog_spec::JsonDialog;

fn main() -> turbo_vision::core::error::Result<()> {
    let mut app = Application::new()?;
    let mut dialog = JsonDialog::load("contact.json").expect("contact.json is a valid dialog");
    dialog.set_data(&json!({ "topic": 0, "copy": true })).unwrap();
    let answer = dialog.run(&mut app);   // None when cancelled
    drop(app);                           // give the terminal back before printing
    match answer {
        Some(a) => println!("{} {}", a.action, a.data),   // "ok" and the data
        None => println!("cancelled"),
    }
    Ok(())
}
```

`JsonDialog::from_json(include_str!("contact.json"))` embeds the file in the
program instead. `run` returns `None` when the person cancels (Cancel, Esc,
the close box); otherwise an `Answer { action, data }` where `action` is
`"ok"`, `"yes"`, `"no"`, or the action that closed the dialog. The same
`JsonDialog` can run again and again.

Everything below is optional: it is for what the JSON cannot say.

### Rules

```rust,ignore
// One field. Runs when the dialog is accepted, after the spec's own checks.
dialog.rule("email", |value| match value.as_str() {
    Some(e) if !e.is_empty() && !e.contains('@') => Err("Email must contain @".into()),
    _ => Ok(()),
});

// Across fields, once each is valid alone.
dialog.validate_data(|data, errors| {
    if data["start"].as_i64() > data["end"].as_i64() {
        errors.add("end", "The end comes before the start");   // blames a control
        // errors.add_form("...") blames the dialog as a whole
    }
});
```

While anything fails the dialog stays open: labels of the controls at fault
turn red, the error line shows the message, the focus goes to the first
control at fault (bringing its tab page forward) and the errors clear as they
are fixed. `dialog.validate()` runs the same checks without showing the
dialog, which is handy in tests.

### Actions

A button whose `command` is not `ok`, `cancel`, `yes` or `no` names an action.
Bind code to it:

```rust,ignore
use tv_extensions::dialog_spec::Flow;

dialog.on("lookup", |ctx| {
    let name = ctx.value("name").unwrap_or_default();
    ctx.set_data(&json!({ "email": "found@example.org" })).unwrap();
    Flow::Stay
});
```

The handler's `ctx` offers `value(name)`, `set_value(name, &json)`, `data()`,
`set_data(&json)`, `view_mut::<T>(name)` (the turbo-vision view of a control,
such as `view_mut::<CheckBox>("vip")`), `action()` and `app` (for message
boxes and other dialogs). What it returns decides what happens:

| `Flow` | The dialog |
|--------|------------|
| `Stay` | stays open |
| `Submit` | validates as OK does, closes if it passes |
| `Close` | closes at once, without validating |

An action with no handler validates and closes like OK, and `answer.action`
says which button it was, so a dialog with "Save" and "Save and new" buttons
needs no handlers: read `answer.action` afterwards.

### Reacting to a change

```rust,ignore
// Works for any named control with a value, whatever its type.
dialog.on_change("volume", |ctx| {
    let level = ctx.value("volume").unwrap();
    ctx.set_value("meter", &level).unwrap();
    Flow::Stay
});
```

`on_change(name, handler)` runs each time the person changes the control
(an input as they type, a check box as they tick it, a list as they choose).
What the code itself sets does not count as a change, so a handler can set
other controls, its own included, without looping. Nothing is needed in the
JSON.

### Validators of your own

Register a validator named in the JSON (`{ "type": "custom", "name": "iban" }`):

```rust,ignore
dialog.validator("iban", Rc::new(RefCell::new(Iban)));   // Iban implements turbo_vision's Validator
```

### Nothing left unbound

`dialog.unbound()` lists what the JSON names that no code has bound
(`action "lookup"`, `validator "iban"`). Assert it is empty in a test so a
renamed button cannot go unnoticed.

## Mistakes to avoid

- **Inventing keys.** Only the keys on this page exist. `"label"` on a button
  or input is wrong (`text` is the caption; an input's caption is a separate
  `label` control); `"id"`, `"value"` on an input, `"options"`, `"title"` on
  a button are all refused.
- **Positions measured from the outer frame.** `x: 0, y: 0` is the cell just
  inside the frame, not on it. Run `dialog_check` and fix its "does not fit"
  warnings.
- **Buttons one row tall or too narrow.** Use `h: 2` and `w` at least the
  text + 4.
- **Forgetting names.** A control with a value and no `name` is not in the
  data and cannot be validated by code.
- **Duplicate names**, including the same name on two tab pages.
- **Labels without `link`.** The hot key then does nothing.
- **Hot keys that repeat.** Two `~N~` captions in one dialog fight for Alt+N;
  use a different letter for each.
- **No `ok` button, or no `default` one.** Give the OK button
  `"default": true` so Enter accepts the dialog; `dialog_check` warns when
  nothing has the command `ok`.
- **Putting values in the wrong form.** A `radio_buttons` `selected` and a
  `combo_box` `selected` are indices (numbers), a `tree`'s is a key (string),
  a `dual_list`'s `chosen` is a list of keys.
- **Nesting tabs inside tabs.**
- **Overlapping controls.** Leave a blank row between fields; a radio group of
  *n* items is *n* rows tall.
- **Trailing commas and comments.** The file is strict JSON.

## Recipes

### Login

```json dialog
{
  "version": 1,
  "title": "Sign in",
  "width": 44,
  "height": 12,
  "controls": [
    { "type": "label", "x": 2, "y": 1, "w": 10, "h": 1, "text": "~U~ser", "link": "user" },
    { "type": "input", "name": "user", "x": 13, "y": 1, "w": 26, "h": 1, "required": true },
    { "type": "label", "x": 2, "y": 3, "w": 10, "h": 1, "text": "~P~assword", "link": "password" },
    { "type": "input", "name": "password", "x": 13, "y": 3, "w": 26, "h": 1, "required": true },
    { "type": "check_box", "name": "remember", "x": 13, "y": 5, "w": 22, "h": 1, "text": "~R~emember me" },
    { "type": "error_line", "x": 2, "y": 6, "w": 38, "h": 1 },
    { "type": "button", "x": 8, "y": 7, "w": 10, "h": 2, "text": "~O~K", "command": "ok", "default": true },
    { "type": "button", "x": 22, "y": 7, "w": 12, "h": 2, "text": "Cancel", "command": "cancel" }
  ]
}
```

### Settings with tabs and a group box

```json dialog
{
  "version": 1,
  "title": "Settings",
  "width": 60,
  "height": 18,
  "controls": [
    { "type": "tabs", "x": 1, "y": 0, "w": 56, "h": 12, "pages": [
      { "title": "~G~eneral", "controls": [
        { "type": "label", "x": 1, "y": 1, "w": 8, "h": 1, "text": "~N~ame", "link": "name" },
        { "type": "input", "name": "name", "x": 10, "y": 1, "w": 30, "h": 1 },
        { "type": "check_box", "name": "autosave", "x": 10, "y": 3, "w": 24, "h": 1, "text": "~A~utosave", "checked": true },
        { "type": "label", "x": 1, "y": 5, "w": 8, "h": 1, "text": "~I~nterval", "link": "interval" },
        { "type": "spinner", "name": "interval", "x": 10, "y": 5, "w": 12, "h": 1, "min": 1, "max": 60, "value": 5, "suffix": " min" }
      ] },
      { "title": "~D~isplay", "controls": [
        { "type": "group_box", "x": 1, "y": 0, "w": 36, "h": 6, "title": "Theme", "controls": [
          { "type": "radio_buttons", "name": "theme", "x": 2, "y": 1, "w": 20, "h": 3, "items": ["~B~lue", "~G~reen", "~M~ono"] }
        ] }
      ] }
    ] },
    { "type": "error_line", "x": 2, "y": 12, "w": 52, "h": 1 },
    { "type": "button", "x": 16, "y": 13, "w": 10, "h": 2, "text": "~O~K", "command": "ok", "default": true },
    { "type": "button", "x": 30, "y": 13, "w": 12, "h": 2, "text": "Cancel", "command": "cancel" }
  ]
}
```

### A form that reacts: volume and level

`examples/dialog_specs/volume.json` is a radio group, a slider and a progress
bar; `examples/dialog_on_change.rs` binds `on_change("volume", ...)` and
`on_change("mode", ...)` so that the bar follows the slider and the mode sets
both. Other complete examples: `examples/dialog_specs/customer.json` with
`examples/dialog_from_json.rs` (rules, an action, a cross-field rule), and
`examples/dialog_specs/everything.json` (one control of every type).

## Where else to look

- `docs/dialog-designer.md`: the designer, its keys, templates, and the
  generated code.
- `src/dialog_spec/mod.rs` (rustdoc): the Rust API.
- `templates/*.json`: nine small dialogs (login, contact form, find and
  replace, settings, confirm, about, list picker, table browser, progress)
  to start from.
