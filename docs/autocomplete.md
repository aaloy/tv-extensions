# AutoComplete

`AutoComplete` is a text field that filters a suggestion list as you type,
like the typeahead fields of web toolkits. Once enough characters are typed,
a list of the items containing them opens under the field, with the matched
text in its own colour. It is always available, with no feature flag.

The field works in one of two ways, chosen with `require_match`:

| | Free text (default) | Must match (`require_match(true)`) |
|---|---|---|
| What the list does | Suggests | Offers the only valid values |
| Typed text that is not an item | Kept | Kept, so the user sees what to fix; OK is refused |
| Text that matches an item in another case (`méxico`) | Kept as typed | Changed to the item's spelling (`México`) when the field loses focus |
| Empty field | Allowed | Refused: a value is required |
| OK in a dialog | Always closes | Refused, with an error line under the field, until it holds an item |
| Typical use | Search boxes, names, tags, anything open-ended | Countries, codes, any value from a fixed set |

## Free text

The default. The list is a convenience: the user can pick a suggestion or
type anything else.

```rust
use std::{cell::RefCell, rc::Rc};
use turbo_vision::core::geometry::Rect;
use tv_extensions::AutoCompleteBuilder;

let fruit = Rc::new(RefCell::new(String::new()));
let field = AutoCompleteBuilder::new()
    .bounds(Rect::new(12, 11, 42, 12))
    .items(["Apple", "Banana", "Cherry"])
    .data(fruit.clone())
    .build();
// After the dialog closes, `fruit` holds whatever the user typed or picked.
```

## Must match

With `require_match(true)` a value from the list is required. It follows
Borland's `TInputLine` with a `TStringLookupValidator`:

- **OK or Enter in a dialog** runs the dialog's `valid()` check, which asks
  every control. The field refuses an empty value or text that is not an
  item, so the dialog stays open, and an error line appears under the field
  in Borland's error colours (white on light red). It reads "Choose a value
  from the list" unless `error_message` says otherwise, and it stays until
  the user edits the field or picks a suggestion. When the field still has
  the focus, the list also opens on the closest matches. Cancel always
  closes.
- **Leaving the field** (Tab, or a click elsewhere) only fixes the spelling
  of a match: `méxico` becomes `México`. Other text is left as typed, so the
  user sees what to fix when OK refuses it.

Borland's validator reports with a message box. Here `valid()` has no access
to the application to run one (core's own validators leave their `error()`
empty for that reason), so the field shows the error itself; being part of
the field, it stays visible while the focus is on the OK button.

```rust
use std::{cell::RefCell, rc::Rc};
use turbo_vision::core::geometry::Rect;
use tv_extensions::AutoCompleteBuilder;

let country = Rc::new(RefCell::new(String::new()));
let field = AutoCompleteBuilder::new()
    .bounds(Rect::new(12, 3, 42, 4))
    .items(["Austria", "Australia", "México"])
    .data(country.clone())
    .require_match(true)
    .error_message("Choose a country from the list")
    .build();
// When the dialog closes with OK, `country` is one of the items.
```

The same switch exists on the view itself: `set_require_match(true)`.

## Keys and mouse

| Key | Action |
|-----|--------|
| Printable characters (ASCII and Latin-1) | Type into the field, filter the list |
| Down | Open the list, or move the highlight down |
| Up | Move the highlight up |
| Enter | Accept the highlighted suggestion; with the list closed, the dialog's default button |
| Esc | Close the list without changing the text; with the list closed, the dialog's handling |
| Left/Right/Home/End/Backspace/Delete | Ordinary text editing |

Clicking a suggestion accepts it, and clicking the field opens the list.
Matching ignores case, including accented Latin-1 letters such as `é` or
`ñ`; characters beyond Latin-1 cannot be typed, since turbo-vision's key
codes cannot tell them from special keys.

## Layout and colours

Give the field a one-row `bounds`. While the error line or the list is
showing, the view's bounds grow downward over them, so the owning group draws it and sends clicks on it to
the field; they shrink back when it closes. Leave room below the field for
the list (`max_drop_rows`, default 6), or add the field after any control the
list may open over, since children are drawn and hit-tested in the order they
were added.

Colours come from the owner's palette, like core's controls: the field uses
the input-line entries; the list uses the list-box entries in a dialog and
the window's own text colours in a window.

## Options

| Builder | Setter | Default | Meaning |
|---|---|---|---|
| `items` | `set_items` | empty | The suggestions |
| `data` | — | a new empty string | The shared `Rc<RefCell<String>>` holding the text |
| `require_match` | `set_require_match` | `false` | Must the value be one of the items? |
| `error_message` | `set_error_message` | "Choose a value from the list" | The error line shown when OK is refused (cut to the field's width) |
| `min_chars` | `set_min_chars` | 1 | Characters typed before the list opens; 0 opens it on any edit |
| `max_drop_rows` | `set_max_drop_rows` | 6 | Rows shown before the list scrolls |
| `max_length` | `set_max_length` | 255 | Longest text accepted |
| `on_select` | `set_on_select` | 0 (none) | Command broadcast when a suggestion is accepted |

## Example

`examples/autocomplete.rs` shows both modes in one dialog: Country must
match, Fruit takes free text. Press OK with Country empty, or with a country
that is not in the list, to see the error line; then pick one and press OK
again.

```sh
cargo run --example autocomplete --features native
```
