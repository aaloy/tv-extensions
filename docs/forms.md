# Building Forms with `Form`

`Form` (in `tv_extensions::forms`) builds a data-entry dialog from a list
of labelled fields. You never write a coordinate: you say **what** goes in the
form, in order, and the form decides **where** it goes, how big the dialog is,
and centres it on the screen.

Without it, every label, field and button needs a hand-computed `Rect`, and the
dialog's size has to be worked out to match. With it, a form is a list.

## Quick start

A complete program (`cargo run --example form_layout --features native` runs a fuller version):

```rust
use turbo_vision::app::Application;
use turbo_vision::core::command::CM_OK;
use turbo_vision::core::geometry::Rect;
use turbo_vision::views::GroupLike; // brings `dialog.get(handle)` into scope
use tv_extensions::forms::{Form, size};
use turbo_vision::views::input_line::InputLine;

fn main() -> turbo_vision::core::error::Result<()> {
    let mut app = Application::new()?;

    let mut form = Form::new("New Customer");
    let name = form.field("~N~ame", InputLine::new(Rect::default(), 40));
    let zip = form.field("~Z~IP code", InputLine::new(size(8, 1), 8));
    form.ok_cancel();
    let mut dialog = form.build();

    if dialog.execute(&mut app) == CM_OK {
        let name = dialog.get(name).map(|f| f.text().to_string()).unwrap_or_default();
        let zip = dialog.get(zip).map(|f| f.text().to_string()).unwrap_or_default();
        println!("{name}, {zip}");
    }
    Ok(())
}
```

Three steps, always the same:

1. **Describe** the form: `Form::new(title)`, then add rows and buttons in
   order. Each call that adds a view returns a typed `Handle`.
2. **Build** it: `form.build()` returns an ordinary `Dialog`.
3. **Run and read**: `dialog.execute(&mut app)` returns the command of the
   button that closed it; `dialog.get(handle)` gives back each view, with its
   own type, to read its value.

**Editing a struct, such as a database row?** Use a record form instead:
`Form::<Customer>::for_record(...)` binds each field to a member of your
struct, converts and validates the values, shows errors in the dialog and
gives you back the edited struct. See [Editing records](#editing-records).

## What the layout looks like

`cargo run --example form_layout --features native` builds this form: a few fields, an address
group with two fields on one line, a check box, a notes box and OK/Cancel.

```text
╔═[■]════════ New Customer ═══════════╗
║                                     ║
║  Name ____________________________  ║   <- label column | field column
║                                     ║   <- spacing: 1 blank row
║ Email ____________________________  ║   <- labels right-aligned (LabelAlign::Right)
║                                     ║
║ ┌─ Address ───────────────────────┐ ║   <- form.group("Address")
║ │ Street ________________________ │ ║   <- a group lines up its own labels
║ │                                 │ ║
║ │   City ____________  ZIP ______ │ ║   <- form.line(): two fields on one row;
║ └─────────────────────────────────┘ ║      ZIP built with size(8, 1) keeps 8 columns,
║                                     ║      City stretches into the rest
║       [ ] VIP customer              ║   <- empty label: lines up with the fields
║                                     ║
║                                     ║   <- a section adds one more blank row
║ Notes                               ║   <- form.section("Notes")
║                                     ║
║ ___________________________________ ║   <- form.row(...): spans the whole form,
║ ___________________________________ ║      3 rows because built with size(0, 3)
║ ___________________________________ ║
║                                     ║
║          OK    ▀   Cancel  ▀        ║   <- buttons: centred, on the bottom row
║        ▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄        ║
╚═════════════════════════════════════╝
```

(`____` marks where an input field is; on screen it is a coloured bar.)

With `form.label_position(LabelPosition::Above)`
(`cargo run --example form_layout --features native -- --above`) each label moves to the row
above its field, and the form gets narrower and taller:

```text
║ ┌─ Address ────────────┐ ║
║ │ Street               │ ║
║ │ ____________________ │ ║
║ │                      │ ║
║ │ City        ZIP      │ ║
║ │ __________  ________ │ ║
║ └──────────────────────┘ ║
```

### The rules

- **Rows** go top to bottom in the order you add them, one blank row apart
  (change it with `form.spacing(n)`).
- **Labels** go to the left of their fields, in a column as wide as the
  longest label (or above them, see [Where labels go](#where-labels-go)). Each
  label is linked to its field: clicking the label, or pressing Alt and the
  letter marked with `~` (`"~N~ame"` → Alt+N), focuses the field.
- **Fields** start right after the label column, all at the same column.
- **Sizes come from the view.** How you build the view decides its size:

  | Build the view with      | Width                                    | Height |
  |--------------------------|------------------------------------------|--------|
  | `Rect::default()`        | stretches to fill the width it is given  | 1 row  |
  | `size(w, h)`             | exactly `w` columns                      | `h` rows |
  | `size(0, h)`             | stretches to fill the width it is given  | `h` rows |

  A field alone on its row is at least 20 columns wide (`form.field_width(n)`
  changes that), and the field column widens to fit the widest sized field.
  The position you give a view is ignored: the form sets it.
- **Lines** (`form.line()`) put several fields on one row, left to right. The
  first field's label is in the label column; each other field has its label
  just before it. Stretched fields on a line share the width the sized ones
  leave, each at least 10 columns.
- **Groups** (`form.group("Title")` … `form.end_group()`) draw a titled box,
  as wide as the form, around the rows added in between. A group lines up its
  own labels, independently of the rows outside it. Groups can be nested.
- **Tabs** (`form.tab("~G~eneral")`, then the rows of that page, then the
  next `form.tab(..)` … `form.end_tabs()`) put each page's rows under a strip
  of tabs. The pane is as wide as the form and as high as its tallest page;
  F6 or Ctrl+PgDn change page, and so does Alt plus a `~` letter of a title.
  Groups and lines work inside a page; tabs do not nest. Fields on a page
  are not children of the dialog, so read them through a record form's
  bindings, not `dialog.get(handle)`. An error on another page brings that
  page forward.
- **Full-width rows** (`form.row(view)`) start at the label column and span
  the whole form (or group). Use them for a long check box caption, a note, a
  list.
- **Sections** (`form.section("Address")`) put a plain heading over the rows
  that follow, with an extra blank row above it. Use a group instead when the
  rows should be boxed.
- **Buttons** go on one row at the bottom, centred, in the order added, each
  at least 10 columns wide. `form.button_align(ButtonAlign::Right)` puts them
  against the right edge.
- **The dialog** is sized to fit everything, including its title, and is
  centred on the desktop when you `execute` it or add it to the desktop. The
  first field has the focus; Tab moves through the fields in the order they
  were added: top to bottom, and left to right along a line.

## Recipes

### Read values back

Keep the handle each call returns, then ask the built dialog for the view.
`get` returns `Option<&T>` of the view's own type:

```rust
let name = form.field("~N~ame", InputLine::new(Rect::default(), 40));
let vip = form.field("", CheckBox::new(Rect::default(), "VIP customer"));
let notes = form.row(Memo::new(size(0, 4)));
let mut dialog = form.build();
// ... dialog.execute(&mut app) ...
let name: String = dialog.get(name).map(|f| f.text().to_string()).unwrap_or_default();
let vip: bool = dialog.get(vip).is_some_and(CheckBox::is_checked);
let notes: String = dialog.get(notes).map(Memo::get_text).unwrap_or_default();
```

`get` needs `use turbo_vision::views::GroupLike;`. It returns `None` only if
the handle came from a different dialog.

### Pre-fill values (editing an existing record)

Set the value on the view before adding it, or through `get_mut` afterwards:

```rust
let mut input = InputLine::new(Rect::default(), 40);
input.set_text("Ada Lovelace");
let name = form.field("~N~ame", input);

let mut dialog = form.build();
if let Some(f) = dialog.get_mut(name) {
    f.set_text("Grace Hopper");
}
```

### Several fields on one line

```rust
let mut line = form.line();
let city = line.field("~C~ity", InputLine::new(Rect::default(), 40));
let zip = line.field("~Z~IP", InputLine::new(size(8, 1), 8));
```

The line ends when you add the next row. Each `line.field(...)` returns a
handle, like `form.field(...)`. Give the narrow fields a size and leave the
main one stretched: it takes the rest of the row.

### Group related fields in a box

```rust
form.group("Address");
let street = form.field("~S~treet", InputLine::new(Rect::default(), 60));
let mut line = form.line();
let city = line.field("~C~ity", InputLine::new(Rect::default(), 40));
let zip = line.field("~Z~IP", InputLine::new(size(8, 1), 8));
form.end_group();
```

Everything between `group` and `end_group` goes inside the box: fields,
lines, rows, sections, even another group. A group you forget to end is
closed by `build()`. The box itself never takes the focus; it is a
`GroupBox` view, which you can also use on its own in hand-built dialogs.

### Where labels go

Two settings choose where labels go. Both apply to the whole form, groups
included, and can be called at any point before `build()`:

```rust
use tv_extensions::forms::{LabelAlign, LabelPosition};

form.label_position(LabelPosition::Above); // label on the row above its field
form.label_align(LabelAlign::Right);       // labels on the left, ending at their fields
```

`cargo run --example form_labels --features native` opens one contact form in each style, so you
can compare them; only these calls differ. Here is that form (fields shown as
`____`): a line with two fields, two fields of their own, and an address group
with another line.

**Labels on the left** (the default; Borland's own style). One label column,
labels against its left edge:

```text
╔═[■]═══════════════ Contact ═════════════════╗
║                                             ║
║ First name __________  Last name __________ ║
║                                             ║
║ Email      ________________________________ ║
║                                             ║
║ Phone      _______________                  ║
║                                             ║
║ ┌─ Address ───────────────────────────────┐ ║
║ │ Street ________________________________ │ ║
║ │                                         │ ║
║ │ City   __________________  ZIP ________ │ ║
║ └─────────────────────────────────────────┘ ║
║                                             ║
║              OK    ▀   Cancel  ▀            ║
║            ▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄            ║
╚═════════════════════════════════════════════╝
```

**Labels on the left, right-aligned**: `form.label_align(LabelAlign::Right)`.
The same column, but each label ends against its field, so short labels stay
next to what they name:

```text
╔═[■]═══════════════ Contact ═════════════════╗
║                                             ║
║ First name __________  Last name __________ ║
║                                             ║
║      Email ________________________________ ║
║                                             ║
║      Phone _______________                  ║
║                                             ║
║ ┌─ Address ───────────────────────────────┐ ║
║ │ Street ________________________________ │ ║
║ │                                         │ ║
║ │   City __________________  ZIP ________ │ ║
║ └─────────────────────────────────────────┘ ║
║                                             ║
║              OK    ▀   Cancel  ▀            ║
║            ▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄            ║
╚═════════════════════════════════════════════╝
```

**Labels above**: `form.label_position(LabelPosition::Above)`. Each label on
the row above its field; there is no label column, so the form is narrower and
taller:

```text
╔═[■]═════ Contact ════════╗
║                          ║
║ First name   Last name   ║
║ ___________  ___________ ║
║                          ║
║ Email                    ║
║ ________________________ ║
║                          ║
║ Phone                    ║
║ _______________          ║
║                          ║
║ ┌─ Address ────────────┐ ║
║ │ Street               │ ║
║ │ ____________________ │ ║
║ │                      │ ║
║ │ City        ZIP      │ ║
║ │ __________  ________ │ ║
║ └──────────────────────┘ ║
║                          ║
║     OK    ▀   Cancel  ▀  ║
║   ▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄  ║
╚══════════════════════════╝
```

Things to notice in all three:

- In a line, only the first field's label is in the label column; the others
  (`Last name`, `ZIP`) sit just before their own fields.
- A group lines up its own labels: `Street` and `City` form their own column,
  narrower than `First name`'s.
- A field built with a size (`Phone`, `ZIP`) keeps it in every style; the
  stretched ones adapt.

Which one to pick:

| Style | Good for |
|-------|----------|
| left (default) | most forms; labels of similar length |
| left, right-aligned | labels of very different lengths, so short ones stay next to their fields |
| above | narrow dialogs, long labels, several fields on a line |

`label_align` only matters when labels are on the left.

### Validate input

Views keep everything they had before joining the form. Give an input line a
validator as usual:

```rust
use std::cell::RefCell;
use std::rc::Rc;
use turbo_vision::views::validator::RangeValidator;

let age = form.field(
    "~A~ge",
    InputLine::with_validator(size(4, 1), 3, Rc::new(RefCell::new(RangeValidator::new(0, 150)))),
);
```

### Check boxes and other unlabelled views

An empty label leaves the label column blank, so the view lines up under the
fields:

```rust
form.field("", CheckBox::new(Rect::default(), "Send newsletter"));
```

To put it at the left edge instead, use `form.row(...)`.

### Long text, lists, anything taller than one row

Give the view a height; leave the width at 0 to stretch it:

```rust
form.section("Notes");
let notes = form.row(Memo::new(size(0, 5))); // full width, 5 rows
```

### Compact forms, extra space

```rust
form.spacing(0); // no blank rows between rows
form.gap(1);     // one extra blank row here
```

### Custom buttons

`execute` returns the command of the button that closed the dialog. By default
any button you add closes it (`CloseOn::StandardAndButtons`):

```rust
use turbo_vision::core::command::{CM_CANCEL, CommandId};
use tv_extensions::forms::ButtonAlign;

const CM_SAVE_AND_NEW: CommandId = 1001;

form.default_button("~S~ave", CM_OK);
form.button("Save and ~n~ew", CM_SAVE_AND_NEW);
form.button("Cancel", CM_CANCEL);
form.button_align(ButtonAlign::Right);
// ...
match dialog.execute(&mut app) {
    CM_OK => { /* save */ }
    CM_SAVE_AND_NEW => { /* save, then open a fresh form */ }
    _ => { /* cancelled */ }
}
```

The default button is the one Enter presses when the focused control is not a
button. `form.ok_cancel()` adds **OK** (`CM_OK`, default) and **Cancel**
(`CM_CANCEL`).

Alt plus a button's `~` letter presses it from anywhere in the form (Alt+O for
**~O~K**). The letter alone works too, but only when the focused control does
not take typed text: in an input line it is typed instead.

### Resizable forms

```rust
form.resizable(true);
```

The user can then resize the dialog from its corner. Stretched fields and
full-width rows follow its width; the buttons stay on the bottom row. Fields
built with a fixed size keep it.

### Non-modal forms

`build()` returns a normal `Dialog`, so it can also live on the desktop like a
window instead of running modally: `app.desktop.add(dialog)`. It is centred
there too.

## Editing records

A record form edits a struct of yours, such as a row loaded from a database.
Each field is bound to one member of the struct through a *lens*, a closure
that leads from the record to the member: `|c| &mut c.name`. The form uses it
both ways: to show the member when the dialog opens, and to write the edited
value back. Values keep their Rust types: a `u32` member gets a `u32`, an
`Option<NaiveDate>` member an optional date.

### A complete record editor

```rust
use turbo_vision::app::Application;
use tv_extensions::forms::Form;

#[derive(Clone, Default)]
struct Customer {
    id: i64,               // no field for it: kept as it was
    name: String,
    email: Option<String>, // an empty field is None
    age: u32,              // must be a whole number
    vip: bool,
}

fn main() -> turbo_vision::core::error::Result<()> {
    let mut app = Application::new()?;

    let mut form = Form::<Customer>::for_record("Customer");
    form.input("~N~ame", |c| &mut c.name).required();
    form.input("~E~mail", |c| &mut c.email);
    form.input("~A~ge", |c| &mut c.age);
    form.check("~V~IP customer", |c| &mut c.vip);
    form.ok_cancel();
    let mut editor = form.build_editor();

    // Some(edited) once every field is valid and OK was pressed; None on Cancel.
    if let Some(customer) = editor.edit(&mut app, Customer::default()) {
        println!("{} is {}", customer.name, customer.age);
    }
    Ok(())
}
```

It is the same `Form` as before, so lines, groups, sections, label positions
and buttons all work as described above. Three things differ:

1. **Start** with `Form::<YourStruct>::for_record(title)`.
2. **Bind** fields with `input`, `check`, `memo`, `choice` or `bind` (below)
   instead of `field`. Each one takes a label and a lens, and its result takes
   rules such as `.required()`.
3. **Finish** with `form.build_editor()`. The `Editor` it returns opens the
   dialog with `edit(&mut app, record)`, and can be used again for the next
   record.

`edit` returns `Some(record)` built from a copy of the record you passed in,
so members without a field (an `id`, timestamps) keep their values.

### Binding fields

| Method | Control | Member type |
|--------|---------|-------------|
| `form.input(label, lens)` | input line | any `TextValue`: `String`, integers, floats, `NaiveDate`, `NaiveTime`, and `Option` of any of them |
| `form.check(caption, lens)` | check box, lined up with the fields | `bool` |
| `form.memo(label, rows, lens)` | multi-line text box, `rows` high | `String` |
| `form.choice(label, options, lens)` | drop-down list | any `T: Clone + PartialEq` (an enum, a foreign key) |
| `form.bind(label, view, lens, read, write)` | any control you build | any `T` |

Inside a line, `line.input`, `line.check`, `line.choice` and `line.bind` do the
same thing side by side:

```rust
let mut line = form.line();
line.input("~P~hone", |c| &mut c.phone);
line.input("~B~irthday", |c| &mut c.birthday);
```

`choice` takes `(caption, value)` pairs and starts on the option equal to the
member's value:

```rust
#[derive(Clone, Copy, PartialEq)]
enum Plan { Free, Pro, Team }

form.choice(
    "P~l~an",
    [("Free", Plan::Free), ("Pro", Plan::Pro), ("Team", Plan::Team)],
    |c| &mut c.plan,
);
```

For a foreign key, build the pairs from the other table:
`customers.iter().map(|c| (c.name.clone(), c.id))`.

### How field types are read

`input` converts the text to the member's type and back. On bad text the
message says what was expected:

| Member type | Field width | Empty field | Bad text says |
|-------------|-------------|-------------|---------------|
| `String` | stretches | `""` | (anything is accepted) |
| `i8` … `u128`, `isize`, `usize` | 12 | "… is required" | "Age must be a whole number" |
| `f32`, `f64` | 14 | "… is required" | "Price must be a number" |
| `NaiveDate` | 12, `2026-12-31` | "… is required" | "Since must be a date like 2026-12-31" |
| `NaiveTime` | 10, `14:30` or `14:30:05` | "… is required" | "Start must be a time like 14:30" |
| `Option<T>` | as `T` | `None` | as `T` |

Spaces around numbers are ignored. `.width(n)` changes a field's width. If
the input line has a validator (a picture, a range), it is checked too: "Code
is not valid".

### Rules

Chain rules onto the field when you bind it:

```rust
form.input("~N~ame", |c| &mut c.name).required();
form.input("~E~mail", |c| &mut c.email)
    .required_with("We need an email to send the invoice")
    .validate(|e| if e.contains('@') { Ok(()) } else { Err("Email must contain @".into()) });
form.input("~S~eats", |c| &mut c.seats)
    .width(6)
    .invalid_with("Seats is a number of people")
    .validate(|s| if (1..=500).contains(s) { Ok(()) } else { Err("Seats must be between 1 and 500".into()) });
form.check("I ~a~ccept the terms", |c| &mut c.accepted).required(); // must be checked
```

| Rule | Meaning |
|------|---------|
| `.required()` | Must not be empty; a check box must be checked; a choice must be made. Message: "Name is required" ("… must be checked" for a check box). |
| `.required_with(msg)` | Same, with your message. |
| `.invalid_with(msg)` | Your message when the text does not convert to the type. |
| `.validate(\|value\| ...)` | Your check on the converted value: `Err(message)` rejects it. Several run in order. |
| `.width(n)` / `.max_len(n)` | The field's width / the most characters it accepts. |

The form writes its own messages from the field's label, leaving out the `~`,
a trailing `:` and `*`. Your rules return complete sentences, shown as they
are.

**Rules across fields** see the whole record, once every field is valid. Keep
the id of the field the error belongs to:

```rust
use tv_extensions::forms::ValidationErrors;

let seats = form.input("~S~eats", |c| &mut c.seats).id();
form.validate_record(move |c, errors: &mut ValidationErrors| {
    if c.plan == Plan::Team && c.seats < 2 {
        errors.add(seats, "A Team plan needs at least 2 seats");
    }
    if c.name == "root" {
        errors.add_form("That name is reserved"); // about the form, not one field
    }
});
```

### What the user sees

When a button other than Cancel closes the dialog, the editor reads every
field and runs every rule. If anything fails, the dialog stays open:

```text
╔═[■]════════════ New Customer ═══════════════╗
║                                             ║
║ ┌─ Contact ───────────────────────────────┐ ║
║ │ Name                                    │ ║   <- label in red: invalid
║ │                                         │ ║
║ │ Email                                   │ ║   <- red too
║ │                                         │ ║
║ │ Phone             Birthday              │ ║
║ └─────────────────────────────────────────┘ ║
║                   ...                       ║
║ Name is required (+1 more)                  ║   <- the error line, in red
║              OK    ▀   Cancel  ▀            ║
║            ▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄            ║
╚═════════════════════════════════════════════╝
```

- The labels of the invalid fields turn red.
- The error line, just above the buttons, shows the message for the focused
  field (or the first one) and how many more there are.
- The focus moves to the first invalid field.
- From then on, every time the focus moves the form is checked again, so
  errors disappear as they are fixed.

Cancel, Esc and the close box always close the dialog, without checking.

### Saving to a database

`edit_with` hands the valid record to your closure before the dialog closes.
Save it there. If the save fails, return the errors: they are shown like any
other, and the dialog stays open so the user can fix them.

```rust
use tv_extensions::forms::FieldError;

let email = form.input("~E~mail", |c| &mut c.email).required().id();
form.ok_cancel();
let mut editor = form.build_editor();

let saved = editor.edit_with(&mut app, customer, |c| {
    db.save(c).map_err(|e| match e {
        DbError::DuplicateEmail => FieldError::new(email, "That email is already registered").into(),
        other => FieldError::form(format!("Could not save: {other}")).into(),
    })
});
```

The closure may borrow your connection, since it does not need to be
`'static`. Errors about the whole form (`FieldError::form`) show on the error
line without marking a field.

**Editing an existing row** is the same call with the loaded row:
`editor.edit_with(&mut app, row, |r| db.update(r))`. Its `id` comes back
untouched, because the form has no field for it.

**Several accepting buttons**: any button except Cancel validates and saves.
`editor.command()` tells which one closed the dialog:

```rust
form.default_button("~S~ave", CM_OK);
form.button("Save and ~n~ew", CM_SAVE_AND_NEW);
form.button("Cancel", CM_CANCEL);
// ...
while let Some(c) = editor.edit_with(&mut app, Customer::default(), |c| db.insert(c)) {
    if editor.command() != CM_SAVE_AND_NEW {
        break;
    }
}
```

**Fields that depend on each other**: `edit_synced` takes one more closure,
called each time the focus leaves a field, with the record as the fields now
describe it (only when every field reads). Change the record there and
return `true`, and the fields show the new values; return `false` to leave
them alone. For example, the end of one stretch is the start of the next:

```rust
let saved = editor.edit_synced(
    &mut app,
    trip,
    |t| {
        let changed = t.second.start != t.first.end;
        t.second.start = t.first.end;
        changed
    },
    |t| db.save(t),
);
```

`cargo run --example form_record --features native` runs a complete example. An in-memory table
plays the database and rejects a duplicate email. The example covers optional
fields, a date, an enum choice, rules within one field and across fields, and
the save loop.

### Your own field types

Implement `TextValue` for a type of yours to bind it with `input`, for
example a money amount kept in cents:

```rust
use tv_extensions::forms::TextValue;

#[derive(Clone, Copy, PartialEq)]
struct Cents(i64);

impl TextValue for Cents {
    fn to_text(&self) -> String {
        let sign = if self.0 < 0 { "-" } else { "" };
        let cents = self.0.unsigned_abs();
        format!("{sign}{}.{:02}", cents / 100, cents % 100)
    }
    fn from_text(text: &str) -> Result<Self, String> {
        let amount: f64 = text.trim().parse().map_err(|_e| "must be an amount like 12.50".to_string())?;
        Ok(Cents((amount * 100.0).round() as i64))
    }
    fn width() -> Option<i16> {
        Some(12)
    }
}
```

`from_text` returns what the text should be, finishing the sentence that
starts with the field's name. `Option<Cents>` works at once.

### Other controls

`bind` takes any control and two closures: one reads the value (or says what
it should be), the other shows a value. For example, a `Spinner` for a `u8`:

```rust
use turbo_vision::views::spinner::Spinner;

form.bind(
    "~R~ating",
    Spinner::new(size(8, 1), 1, 5),
    |c| &mut c.rating,
    |s: &Spinner| u8::try_from(s.value()).map_err(|_e| "must be 1 to 5".to_string()),
    |s: &mut Spinner, v: &u8| {
        s.set_value(i64::from(*v));
    },
);
```

### Testing a form without a terminal

The editor can run without its dialog: `load` fills the fields and `read`
checks them and returns the record or the errors. Fill fields through the
dialog to simulate input:

```rust
let mut form = Form::<Customer>::for_record("Customer");
let name = form.input("~N~ame", |c| &mut c.name).required().id();
let mut editor = form.build_editor();

editor.load(Customer::default());
let errors = editor.read().err().expect("an empty name is refused");
assert_eq!(errors.field(name), Some("Name is required"));

let line = editor
    .dialog_mut()
    .child_by_id_mut(name.view_id())
    .and_then(|v| v.as_any_mut().downcast_mut::<InputLine>())
    .unwrap();
line.set_text("Grace");
assert_eq!(editor.read().unwrap().name, "Grace");
```

## API reference

All in `tv_extensions::forms`.

| Call | What it does | Returns |
|------|--------------|---------|
| `Form::new(title)` | Start a form for a dialog titled `title`. | `Form` |
| `form.field(label, view)` | A labelled row. `label` may mark a hot key with `~`; `""` for no label. | `Handle<T>` |
| `form.line()` | Start a line of fields side by side. | `Line` |
| `line.field(label, view)` | Add a field to the line, right of the previous one. | `Handle<T>` |
| `form.row(view)` | A row spanning the whole form (or group), no label. | `Handle<T>` |
| `form.group(title)` | Start a titled box around the rows that follow. | `&mut Form` |
| `form.end_group()` | End the innermost open group. | `&mut Form` |
| `form.tab(title)` / `form.end_tabs()` | Start a tab page (and the tabs); end the tab pane. | `&mut Form` |
| `form.section(title)` | A heading, with an extra blank row above. | `&mut Form` |
| `form.gap(rows)` | Extra blank rows. | `&mut Form` |
| `form.button(title, command)` | A button on the bottom row. | `Handle<Button>` |
| `form.default_button(title, command)` | The button Enter presses. | `Handle<Button>` |
| `form.ok_cancel()` | **OK** (`CM_OK`, default) and **Cancel** (`CM_CANCEL`). | `&mut Form` |
| `form.spacing(rows)` | Blank rows between rows (default 1). | `&mut Form` |
| `form.field_width(cols)` | Narrowest field column (default 20). | `&mut Form` |
| `form.label_position(pos)` | `LabelPosition::Left` (default) or `LabelPosition::Above`. | `&mut Form` |
| `form.label_align(align)` | `LabelAlign::Left` (default) or `LabelAlign::Right`. | `&mut Form` |
| `form.button_align(align)` | `ButtonAlign::Center` (default) or `ButtonAlign::Right`. | `&mut Form` |
| `form.resizable(yes)` | Let the user resize the dialog (default off). | `&mut Form` |
| `form.build()` | Lay out and return the dialog. | `Dialog` |
| `size(w, h)` | A size for a view that should keep it: `Rect::new(0, 0, w, h)`. | `Rect` |

Record forms (`Form::<R>::for_record`) add:

| Call | What it does | Returns |
|------|--------------|---------|
| `Form::<R>::for_record(title)` | Start a form that edits records of type `R`. | `Form<R>` |
| `form.input(label, lens)` | Input line bound to a `TextValue` member. | `Field` |
| `form.check(caption, lens)` | Check box bound to a `bool`. | `Field` |
| `form.memo(label, rows, lens)` | Multi-line text bound to a `String`. | `Field` |
| `form.choice(label, options, lens)` | Drop-down list of `(caption, value)` pairs. | `Field` |
| `form.bind(label, view, lens, read, write)` | Any control. | `Field` |
| `line.input` / `line.check` / `line.choice` / `line.bind` | The same, side by side on a line. | `Field` |
| `field.required()` / `.required_with(msg)` | Must not be empty. | `Field` |
| `field.invalid_with(msg)` | Message for text that does not convert. | `Field` |
| `field.validate(\|v\| ...)` | A check on the value. | `Field` |
| `field.width(n)` / `.max_len(n)` | Width / most characters. | `Field` |
| `field.id()` | The field's id, for errors. | `FieldId` |
| `form.validate_record(\|r, errors\| ...)` | A rule across fields. | `&mut Form<R>` |
| `form.build_editor()` | Lay out and return the editor (`R: Clone`). | `Editor<R>` |
| `editor.edit(&mut app, record)` | Run it; the edited record once valid. | `Option<R>` |
| `editor.edit_with(&mut app, record, \|r\| save(r))` | The same, saving before closing. | `Option<R>` |
| `editor.edit_synced(&mut app, record, \|r\| sync(r), \|r\| save(r))` | The same, and `sync` may change the record each time the focus leaves a field. | `Option<R>` |
| `editor.load(record)` / `editor.read()` (needs `&mut`) | Fill the fields / check and read them, without running. | `()` / `Result<R, ValidationErrors>` |
| `editor.show_errors(errors)` / `editor.errors()` | Show errors / the errors shown. | |
| `editor.command()` | The button that closed the dialog last. | `CommandId` |
| `editor.dialog()` / `editor.dialog_mut()` | The dialog. | `&Dialog` |
| `ValidationErrors::add(field, msg)` / `add_form(msg)` / `field(id)` | Collect errors / look one up. | |
| `FieldError::new(field, msg)` / `FieldError::form(msg)` | One error; `.into()` makes `ValidationErrors`. | `FieldError` |

On the built dialog (with `use turbo_vision::views::GroupLike;`):

| Call | What it does |
|------|--------------|
| `dialog.execute(&mut app)` | Run it modally; returns the closing button's command. |
| `dialog.get(handle)` / `dialog.get_mut(handle)` | The view, with its own type. |

The settings (`spacing`, `field_width`, `label_position`, `label_align`,
`button_align`, `resizable`) can be called at any point before `build()`;
they apply to the whole form.

## Rules at a glance (for code generators and agents)

Follow these and the result needs no coordinates and no adjustment:

1. Create views with `Rect::default()` (stretch, 1 row) or `size(w, h)` (fixed
   size). Never compute a position: the form overwrites it.
2. Add rows in visual order, top to bottom. Add buttons in left-to-right order.
3. Keep every `Handle` you will read later; read values only after
   `dialog.execute(...)` returns, with `dialog.get(handle)`.
4. Mark each label's hot key with `~` around one letter, and use a different
   letter for each label in the form and each button.
5. Use `form.field("", view)` for a view aligned with the fields,
   `form.row(view)` for one spanning the form.
6. For fields on one row, call `form.line()` once and add each field with
   `line.field(...)`; add the next row with `form.*` as usual. Give every
   field but the main one a `size(w, 1)`.
7. Wrap related rows in `form.group("Title")` … `form.end_group()`; pair
   every `group` with an `end_group`.
   For tabs, `form.tab(title)` per page and one `form.end_tabs()`.
8. Exactly one `default_button` (or `ok_cancel()`) per form.
9. Compare `execute`'s result with the button commands you added; `CM_CANCEL`
   also comes back when the user presses Esc or closes the window.
10. Bring `turbo_vision::views::GroupLike` into scope to use `get` / `get_mut`.

For record forms:

11. Derive `Clone` on the record. Start with `Form::<Record>::for_record`,
    bind each member with the method for its type (table above), and finish
    with `build_editor()`.
12. Lenses are `|r| &mut r.member`, one member each. Do not compute values in
    them.
13. Use `Option<T>` members for fields that may be left empty; mark the rest
    `.required()` if an empty value (`""`, unchecked) is not acceptable.
14. Write messages as complete sentences that start with the field's name,
    like the built-in ones: "Email must contain @".
15. Keep the `FieldId` (`.id()`) of every field a record rule or the save
    closure reports errors on; report errors about no single field with
    `add_form` / `FieldError::form`.
16. Save inside `edit_with`'s closure and turn database errors into
    `FieldError`s there; never save after `edit` returns if the save can fail.

## Limitations

- Label position and alignment apply to the whole form, not per group.
- The form's own messages are in English. Replace them per field with
  `required_with` and `invalid_with`, or for a type in its `TextValue::from_text`.
- After a failed check, fields are checked again when the focus moves, not on
  every key. Errors returned by the save closure stay until then too.
- Groups are as wide as the form: two groups cannot sit side by side.
- A line's fields after the first do not line up with the fields of other
  lines; only the first field of each row is in the field column.
- The layout is computed once, at `build()`. Views added to the dialog
  afterwards (with `dialog.add`) are placed by hand, in the dialog's interior
  coordinates.
- A `CheckBox` does not react to a `~` hot key in its own caption yet. If it
  needs one, give it a form label instead:
  `form.field("~V~IP", CheckBox::new(Rect::default(), "Yes"))` makes Alt+V
  focus it, and Space then ticks it.

## See also

- `examples/form_layout.rs`: the complete form shown at the top.
- `examples/form_labels.rs`: one form in each label style.
- `examples/form_record.rs`: a record editor saving to an in-memory table.
- [Chapter 5 – Creating Data-Entry Forms](user-guide/Chapter-05-Creating-Data-Entry-Forms.md):
  the same controls placed by hand, and how dialogs work underneath.
- [Chapter 13 – Data Validation](user-guide/Chapter-13-Data-Validation.md):
  the validators you can attach to input lines.

## Forms as JSON (feature `json`)

A program that cannot draw a dialog itself, an agent or an MCP tool, can
describe a form as JSON, let a person fill it in, and get the answers back as
JSON or a cancel:

```rust,no_run
use serde_json::json;
use turbo_vision::app::Application;
use tv_extensions::forms::json::{FormError, run};

# fn main() -> turbo_vision::core::error::Result<()> {
let mut app = Application::new()?;
let spec = json!({
    "title": "New customer",
    "fields": [
        { "name": "name", "type": "text", "required": true },
        { "name": "age",  "type": "integer", "min": 0, "max": 120 },
        { "name": "kind", "type": "choice", "options": ["Retail", "Wholesale"] },
        { "name": "vip",  "type": "boolean", "label": "VIP customer" }
    ]
});
match run(&mut app, &spec) {
    Ok(answers) => println!("{answers}"), // {"name":"Ada","age":36,"kind":"Retail","vip":false}
    Err(FormError::Cancelled) => println!("cancelled"),
    Err(FormError::Spec(why)) => eprintln!("bad form: {why}"),
}
# Ok(())
# }
```

It is the record form above with a JSON front end, so the person gets the
same validation, the same errors in the dialog and the same keyboard and mouse.
Field types are `text`, `memo`, `integer`, `number`, `boolean`, `choice`,
`multichoice` (check boxes, answering a list), `date` and `section` (a
heading). Layout items are `note` (some text), `group` (a titled box) and
`row` (fields side by side) and `tabs` (pages of fields). Per field: `name` (the answer's key),
`label`, `required`, `default`, and by type `max_len`, `rows`, `min`, `max`,
`options`, `widget`. A `choice` is a drop-down by default; `"widget": "radio"`
shows the options as radio buttons (up to 32; arrows move, Space picks), and an
`integer` with `"widget": "spinner"` or `"slider"` (and whole `min`/`max`)
steps with the arrows. `group` and `row` take a `fields` list of their own and
nest; the answers stay one flat object. Top-level `labels` (`left` or `above`)
and `label_align` (`left` or `right`) set how labels sit. A `tabs` item has `pages`, each with a `label` and its own `fields`; the answers stay flat, and an error on another page brings that page forward. A `text` with a `mask` such as `"##/##/####"` is typed against that picture. Labels without a `~` get an Alt hot key picked for them. Unknown
keys are refused, and a bad spec comes back as `FormError::Spec` with a
message such as `fields[2].type: unknown type "txt"; expected one of ...`, so
the agent that wrote it can fix it. See the module documentation of
`tv_extensions::forms::json` for the full spec.

`cargo run --example form_json --features native,json` runs the customer form
and prints the answers; add `-- deploy`, `-- survey` or `-- layout` (groups, rows, spinner, slider,
multichoice) or `-- tabs` for the other specs in
`examples/form_json_specs/`, or `-- your-spec.json`. `form_json_agent` plays
the calling side: it sends a spec with a mistake, reads the error message the
way a model would, repairs the spec, and acts on the answer or the cancel.
`tests/forms_json.rs` drives every field type through the dialog with
keystrokes.

