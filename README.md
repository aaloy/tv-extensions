# tv-extensions

<p align="center">
  <a href="https://crates.io/crates/tv-extensions"><img src="https://img.shields.io/crates/v/tv-extensions?style=flat-square&labelColor=101010" alt="crates.io"></a>
  <a href="https://docs.rs/tv-extensions"><img src="https://img.shields.io/docsrs/tv-extensions?style=flat-square&labelColor=101010" alt="docs.rs"></a>
</p>

<img src="https://raw.githubusercontent.com/aovestdipaperino/tv-extensions/main/logo.png" alt="tv-extensions logo" width="384" align="right" />

Extensions for [turbo-vision](https://github.com/aovestdipaperino/turbo-vision-4-rust), the Rust port of Borland's Turbo Vision. This crate holds the niche features that moved out of turbo-vision core — a CSV table editor, a logging window, Kitty/ANSI graphics, remote input over TCP, an SSH server, `ScrollPane` and popup menus — plus an `AutoComplete` field, a `DualList` picker and host-driven embedding, for a host that owns the screen and the event loop and steps the application one frame at a time.

## Getting started

```toml
[dependencies]
turbo-vision = { version = "4.0.1", default-features = false }
tv-extensions = "0.4"
```

## Component gallery

```sh
cargo run --example gallery --features native,csv,log,graphics,json
```

Every component, live, beside how it works, its parameters and the code that built it, as in turbo-vision's own gallery. The list on the left is grouped by kind; F6 moves into the panel to try a component, and F6 again goes back. The code shown is the demo's own source file (`examples/gallery/demos/`), so what you read is what runs. The CSV editor and the host-driven demo run a second, host-driven application inside a view of the gallery.

## Why a separate crate

Turbo Vision's core is a port of a framework whose design was settled in 1990, and it earns its keep by staying small and stable. Host-driven embedding (in a host such as a plank WASM frame, where the host owns the screen and the event loop) and the extra widgets and protocols above are newer and still-moving use cases, each with its own pace of change, so keeping them here lets them change and version on their own. Core carries only the public API and the small set of hooks this crate builds on: `Backend::is_host_driven` and `Application::step` for host-driven embedding, plus `Terminal::event_injector`, the capture hook, `Terminal::write_raw`, and the public `InputParser` that the remote-input, graphics and SSH modules write and read through.

The crate depends on turbo-vision without its `native` feature, so it builds for `wasm32-wasip1` (`scripts/check-wasm.sh` checks that).

## A host-driven application

A host-driven application never runs its own loop. The host pushes events into a `HostInput`, calls `pump` once per frame, and reads the finished cells out of the terminal buffer.

```rust
use turbo_vision::core::event::Event;
use turbo_vision::core::geometry::Rect;
use turbo_vision::views::window::WindowBuilder;
use tv_extensions::host;

let (mut app, input) = host::app(80, 24);
app.desktop.add(WindowBuilder::new().bounds(Rect::new(0, 0, 40, 10)).title("Hello").build());

// Each frame: hand over what the user did, step, then paint the buffer.
input.push(Event::command(1234));
let running = host::pump(&mut app, &mut ());
let cells = app.terminal.buffer();
```

`pump` returns `false` once the application quits. Calls that would need a nested event loop, such as `exec_view` or a history popup, return `CM_CANCEL` instead of blocking; [docs/host.md](docs/host.md) explains why.

## ScrollPane

A viewport over a virtual area larger than its own screen bounds — TV Tool Box's scrolling dialog boxes. Always available, no feature flag. [docs/scroll-pane.md](docs/scroll-pane.md) has the rest.

```rust
use turbo_vision::core::geometry::Rect;
use tv_extensions::ScrollPane;

// A 40x10 window over a 40x30 virtual form
let pane = ScrollPane::new(Rect::new(0, 0, 40, 10), 30);
assert_eq!(pane.scroll_offset(), 0);
```

## Popup menus

A modal context menu run at a point, reusing the framework's `MenuBox`, plus check-mark menu items. Always available, no feature flag. See [docs/scroll-pane.md](docs/scroll-pane.md).

```rust,ignore
use tv_extensions::popup_menu;

match popup_menu(&mut app.terminal, position, menu) {
    Some(command) => { /* dispatch command */ }
    None => { /* dismissed */ }
}
```

## AutoComplete

A text field that filters a suggestion list as you type, with the matched text highlighted. Up/Down or the mouse pick a suggestion, Enter accepts it, Esc closes the list. Matching ignores case and accepts Latin-1 letters such as `é` or `ñ`. Always available, no feature flag. [docs/autocomplete.md](docs/autocomplete.md) has the rest.

Two independent rules, both off by default, decide what a dialog's OK accepts:

- `required(true)`: the field may not be blank.
- `require_match(true)`: text that is not blank must be one of the items, like Borland's `TStringLookupValidator`.

| `required` | `require_match` | Accepts |
|---|---|---|
| no | no | anything, blank included (free text) |
| yes | no | anything but blank; the list only suggests |
| no | yes | blank, or one of the items |
| yes | yes | one of the items |

A refused value keeps the dialog open and shows an error line under the field ("A value is required" or "Choose a value from the list", or your own text via `required_message` / `match_message`) until it is fixed. Leaving a field that must match fixes the spelling of a match (`méxico` → `México`).

```rust
use std::{cell::RefCell, rc::Rc};
use turbo_vision::core::geometry::Rect;
use tv_extensions::AutoCompleteBuilder;

// A country must be chosen: required, and from the list
let country = Rc::new(RefCell::new(String::new()));
let country_field = AutoCompleteBuilder::new()
    .bounds(Rect::new(12, 3, 42, 4))
    .items(["Austria", "Australia", "México"])
    .data(country.clone())
    .required(true)
    .require_match(true)
    .match_message("Choose a country from the list")
    .build();

// Optional, but if given it must be a listed fruit
let fruit = Rc::new(RefCell::new(String::new()));
let fruit_field = AutoCompleteBuilder::new()
    .bounds(Rect::new(12, 8, 42, 9))
    .items(["Apple", "Banana", "Cherry"])
    .data(fruit.clone())
    .require_match(true)
    .build();
```

`cargo run --example autocomplete --features native` shows the combinations in one dialog.

## Forms

`Form` lays a dialog out from labelled fields, with no coordinates: labels in one column, fields in the next, lines of fields, titled groups, label placement, and buttons along the bottom, the dialog sized to fit and centred. `Form::<T>::for_record` binds the fields to a struct of yours (a database row, say) through typed fields, required fields, rules and a save step, and shows errors in the dialog. Always available, no feature flag; date and time fields (`NaiveDate`, `NaiveTime`) need `chrono`, on by default. [docs/forms.md](docs/forms.md) has the rest, and `cargo run --example form_layout --features native`, `form_labels` and `form_record` show it.

## Dialog designer and JSON dialogs (feature `dialog`)

Lay a dialog out with the mouse in the dialog designer, in the spirit of TV Dialog Design, try it, and save it as JSON; then load it in the application and give it its rules and actions in code. Every core dialog control is there, plus `AutoComplete`, `DualList`, tab pages, group boxes and an error line. Dialogs can be kept as templates (samples are in `templates/`). [docs/dialog-designer.md](docs/dialog-designer.md) has the designer's keys, the JSON format, and how to extend a dialog with validation and actions. [docs/dialog-json.md](docs/dialog-json.md) is a self-contained reference to the JSON (every control, layout rules, the Rust side, common mistakes), written so a person or a coding agent can write a dialog from it alone; `cargo run --example dialog_check --features native,dialog -- FILE.json --render` checks a file and draws it as text.

```sh
cargo run --example dialog_designer --features native,dialog -- customer.json
```

```rust,ignore
use serde_json::json;
use tv_extensions::dialog_spec::{Flow, JsonDialog};

let mut dialog = JsonDialog::load("customer.json")?;
dialog.set_data(&json!({ "name": "Ada" }))?;
dialog.rule("email", |v| if v.as_str().is_some_and(|e| e.contains('@')) { Ok(()) } else { Err("Email must contain @".into()) });
dialog.on("lookup", |ctx| { ctx.set_value("email", &json!("ada@example.org")).unwrap(); Flow::Stay });
if let Some(answer) = dialog.run(&mut app) {
    println!("{} {}", answer.action, answer.data); // ok {"name":"Ada","email":"ada@example.org",...}
}
```

## DualList

Two lists side by side for picking a subset of items, like Django admin's `filter_horizontal`: available items on the left, chosen ones on the right, a filter field over each, and `>` `>>` `<` `<<` buttons between them. Space marks items; Enter or a double-click moves them. Each item is a key and a label, and the chosen keys come back through an `Rc<RefCell<Vec<K>>>`. Always available, no feature flag. [docs/dual-list.md](docs/dual-list.md) has the rest.

```rust
use std::{cell::RefCell, rc::Rc};
use turbo_vision::core::geometry::Rect;
use tv_extensions::DualListBuilder;

// Preselect Basil; OK needs one to five toppings
let chosen = Rc::new(RefCell::new(vec![104]));
let toppings = DualListBuilder::new()
    .bounds(Rect::new(2, 1, 62, 16))
    .items([(101, "Anchovies"), (104, "Basil"), (115, "Mozzarella")])
    .data(chosen.clone())
    .titles("Toppings", "On the pizza")
    .min_chosen(1)
    .max_chosen(5)
    .build();
// After the dialog closes, `chosen` holds the chosen ids.
```

`cargo run --example dual_list --features native` shows it in a dialog, and `cargo run --example dual_list_form --features native` as one field of a larger form (a Django-style "Change user" page).

## Key translation

Turns a key name such as `"ctrl-s"` or `"enter"` into the `Event` turbo-vision expects — the form a host that is not a terminal (a web page, a WASM host) reports keys in. Always available, no feature flag.

```rust
use tv_extensions::keys::translate;

let ctrl_s = translate("ctrl-s", None).unwrap();
let enter = translate("enter", None).unwrap();
```

## CSV editor (feature `csv`)

A CSV table editor — a `Session` holding one document in a window with a menu bar, a status line and dialogs for editing cells, columns and rows. Host-driven like `host`. See [docs/csv.md](docs/csv.md).

```rust
use tv_extensions::csv::{MemDisk, Session};

let mut session = Session::open(80, 24, "", Box::new(MemDisk::default()));
session.step(80, 24);
let _cells = session.buffer();
```

## Log window (feature `log`)

A scrolling output pane (`TerminalWidget`) and a window that shows `tracing` events in it (`LogWindow`): once built, `tracing::info!()`/`debug!()`/`warn!()`/`error!()` route straight to the window. See [docs/log.md](docs/log.md).

```rust,ignore
use turbo_vision::core::geometry::Rect;
use tv_extensions::log::LogWindowBuilder;

let log_window = LogWindowBuilder::new()
    .bounds(Rect::new(0, 0, 80, 15))
    .title("Log")
    .build();
app.desktop.add(log_window);
tracing::info!("Application started");
```

## Graphics (feature `graphics`)

ANSI-art backgrounds (`AnsiBackground`) and bitmap images over the Kitty graphics protocol (`KittyImage`), writing straight to the terminal with `Terminal::write_raw`. See [docs/graphics.md](docs/graphics.md).

```rust,no_run
use turbo_vision::core::geometry::Rect;
use tv_extensions::graphics::KittyImage;

let image = KittyImage::from_file(Rect::new(5, 2, 45, 22), "logo.png")?;
# Ok::<(), std::io::Error>(())
```

## Remote input (feature `remote-input`)

Key chords and mouse clicks typed over a local TCP port, injected into the event loop — a testing and automation aid, off by default. See [docs/remote-input.md](docs/remote-input.md).

```rust,no_run
use turbo_vision::terminal::Terminal;

let mut terminal = Terminal::init()?;
tv_extensions::remote_input::enable(&mut terminal, 8888)?;
# Ok::<(), std::io::Error>(())
```

## SSH server (feature `ssh`)

Serving a turbo-vision application over SSH, built on `russh`: each connection gets its own `Terminal`, backed by an `SshBackend`. See [docs/ssh.md](docs/ssh.md).

```rust,ignore
use tv_extensions::ssh::{SshServer, SshServerConfig};
use turbo_vision::terminal::Terminal;

let config = SshServerConfig::new().bind_addr("0.0.0.0:2222").generate_key();
let server = SshServer::new(config, || {
    Box::new(|backend| {
        let _terminal = Terminal::with_backend(backend).unwrap();
    })
});
```

## Features

| Feature | Enables | Implies |
| --- | --- | --- |
| *(none)* | `host`, `scroll_pane`, `popup_menu`, `autocomplete`, `dual_list`, `forms`, `keys` — builds for `wasm32-wasip1` | |
| `chrono` (default) | date and time fields in record forms | |
| `json` | `forms::json`: forms described and answered as JSON | |
| `dialog` | the `dialog_spec` module: designed dialogs saved as JSON, the dialog designer example | `json` |
| `native` | a real terminal in turbo-vision (crossterm, OS clipboard) | |
| `csv` | the `csv` module: the CSV table editor | |
| `log` | the `log` module: `LogWindow`, `TerminalWidget` | |
| `graphics` | the `graphics` module: Kitty images, ANSI backgrounds | |
| `remote-input` | the `remote_input` module: TCP key/mouse injection | `native` |
| `ssh` | the `ssh` module: serving an application over SSH | `native` |

## Grid retirement

`Grid` has been retired. The core `Table` widget now draws the same separators itself: call `Table::set_separators(true)` to get the column separators in each line's own colour.

## Building and testing

```sh
cargo test
cargo test --example gallery --features native,csv,log,graphics,json
cargo clippy --all-features --all-targets -- -D warnings
sh scripts/check-wasm.sh
```

The gallery's tests check that every demo builds inside its panel, explains itself and links to related demos, and that every module has a demo or a reason not to.

`-D warnings` is the bar: clippy's `pedantic` lints are on (see `Cargo.toml`)
and both the default-features build (which is what the global pre-commit hook
enforces) and `--all-features` are clean under it. The cast family and
`too_many_lines` are allowed crate-wide for the `i16` geometry and the
fixed-width wire formats.

The pages in `docs/` are published as the [Extensions section of tv.enzolombardi.net](https://tv.enzolombardi.net/extensions/), copied there by turbo-vision's `website/sync_docs.py`. `mkdocs build -f website/mkdocs.yml` previews them locally.

## License

MIT, see [LICENSE](LICENSE).
