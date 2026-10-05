# tv-extensions

Extensions for [turbo-vision](https://github.com/aovestdipaperino/turbo-vision-4-rust), the Rust port of Borland's Turbo Vision. The crate holds the niche features that moved out of turbo-vision core — a CSV table editor, a logging window, Kitty/ANSI graphics, remote input over TCP, an SSH server, `ScrollPane` and popup menus — plus an `AutoComplete` field, a `DualList` picker and host-driven embedding, for a host that owns the screen and the event loop and steps the application one frame at a time.

## Why a separate crate

Turbo Vision's core is a port of a framework whose design was settled in 1990, and it earns its keep by staying small and stable. Host-driven embedding (in a host such as a plank WASM frame, where the host owns the screen and the event loop) and the extra widgets and protocols above are newer and still-moving use cases, each with its own pace of change, so keeping them here lets them change and version on their own. Core carries only the public API and the small set of hooks this crate builds on: `Backend::is_host_driven` and `Application::step` for host-driven embedding, plus `Terminal::event_injector`, the capture hook, `Terminal::write_raw`, and the public `InputParser` that the remote-input, graphics and SSH modules write and read through.

The crate depends on turbo-vision without its `native` feature, so it builds for `wasm32-wasip1`.

## Getting started

```toml
[dependencies]
turbo-vision = { version = "4.0.1", default-features = false }
tv-extensions = "0.3"
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
- [AutoComplete](autocomplete.md): a text field that filters a suggestion list as you type, taking free text or requiring one of the items (always available, no feature flag).
- [DualList](dual-list.md): two lists side by side for picking a subset of items, with filters (always available, no feature flag).
- [CSV editor](csv.md) (feature `csv`): a table editor, host-driven like `host`.
- [Log window](log.md) (feature `log`): a scrolling output pane, and a window that shows `tracing` events in it.
- [Graphics](graphics.md) (feature `graphics`): ANSI-art backgrounds and bitmap images over the Kitty graphics protocol.
- [Remote input](remote-input.md) (feature `remote-input`): key chords and mouse clicks injected over TCP, for testing and automation.
- [SSH server](ssh.md) (feature `ssh`): serving a turbo-vision application over SSH.
