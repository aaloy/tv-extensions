# tv-extensions

Extensions for [turbo-vision](https://github.com/aovestdipaperino/turbo-vision-4-rust), the Rust port of Borland's Turbo Vision. The crate holds the pieces that let a host embed a turbo-vision application and step it one frame at a time.

## Why a separate crate

Turbo Vision's core is a port of a framework whose design was settled in 1990, and it earns its keep by staying small and stable. Embedding it in a host such as a plank WASM frame, where the host owns the screen and the event loop, is a newer and still moving use case, and so are the extra widgets an embedder tends to want. Keeping them here lets them change and version on their own, while core carries only the two hooks they need: `Backend::is_host_driven`, so a backend can say that nobody may block waiting for input, and `Application::step`, which runs one pass of the event loop.

The crate depends on turbo-vision without its `native` feature, so it builds for `wasm32-wasip1`.

## Getting started

```toml
[dependencies]
turbo-vision = { version = "3", default-features = false }
tv-extensions = { git = "https://github.com/aovestdipaperino/tv-extensions" }
```

A host-driven application never runs its own loop. The host pushes events into a `HostInput`, calls `pump` once per frame, and reads the finished cells out of the terminal buffer:

```rust
use turbo_vision::core::event::Event;
use tv_extensions::host;

let (mut app, input) = host::app(80, 24);
input.push(Event::command(1234));
let running = host::pump(&mut app, &mut ());
let cells = app.terminal.buffer();
```

[Host-driven apps](host.md) explains the model, and why calls that would block are refused.

## Modules

- [Host-driven apps](host.md): the embedding model above, in full.
- [ScrollPane and popup menus](scroll-pane.md): a scrolling interior for oversized dialogs, and modal context menus (always available, no feature flag).
- [CSV editor](csv.md) (feature `csv`): a table editor, host-driven like `host`.
- [Log window](log.md) (feature `log`): a scrolling output pane, and a window that shows `tracing` events in it.
- [Graphics](graphics.md) (feature `graphics`): ANSI-art backgrounds and bitmap images over the Kitty graphics protocol.
- [Remote input](remote-input.md) (feature `remote-input`): key chords and mouse clicks injected over TCP, for testing and automation.
- [SSH server](ssh.md) (feature `ssh`): serving a turbo-vision application over SSH.
