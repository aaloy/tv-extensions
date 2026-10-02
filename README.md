# tv-extensions

<p align="center">
  <a href="https://crates.io/crates/tv-extensions"><img src="https://img.shields.io/crates/v/tv-extensions?style=flat-square&labelColor=101010" alt="crates.io"></a>
  <a href="https://docs.rs/tv-extensions"><img src="https://img.shields.io/docsrs/tv-extensions?style=flat-square&labelColor=101010" alt="docs.rs"></a>
</p>

<img src="https://raw.githubusercontent.com/aovestdipaperino/tv-extensions/main/logo.png" alt="tv-extensions logo" width="384" align="right" />

Extensions for [turbo-vision](https://github.com/aovestdipaperino/turbo-vision-4-rust), the Rust port of Borland's Turbo Vision. This crate holds the niche features that moved out of turbo-vision core — a CSV table editor, a logging window, Kitty/ANSI graphics, remote input over TCP, an SSH server, `ScrollPane` and popup menus — plus host-driven embedding, for a host that owns the screen and the event loop and steps the application one frame at a time.

## Getting started

```toml
[dependencies]
turbo-vision = { version = "4.0.1", default-features = false }
tv-extensions = "0.3"
```

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
| *(none)* | `host`, `scroll_pane`, `popup_menu`, `keys` — builds for `wasm32-wasip1` | |
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
cargo clippy --all-targets
sh scripts/check-wasm.sh
```

`-D warnings` is not used: clippy's `pedantic` lints are on as warnings (see `Cargo.toml`), and a few files copied verbatim from turbo-vision core carry pedantic warnings inherited from there as-is. The bar is no clippy *errors*, and no new warnings in hand-written code; `cargo clippy --all-targets` and read the output rather than failing the build on every pedantic nit.

The documentation site lives in `website/` and is built with `mkdocs build -f website/mkdocs.yml` from the pages in `docs/`.

## License

MIT, see [LICENSE](LICENSE).
