# tv-extensions

Extensions for [turbo-vision](https://github.com/aovestdipaperino/turbo-vision-4-rust), the Rust port of Borland's Turbo Vision. This crate holds the pieces that let a host embed a turbo-vision application and step it one frame at a time.

## Why a separate crate

Turbo Vision's core is a port of a framework whose design was settled in 1990, and it earns its keep by staying small and stable. Embedding it in a host such as a plank WASM frame, where the host owns the screen and the event loop, is a newer and still moving use case, and so are the extra widgets an embedder tends to want. Keeping them here lets them change and version on their own, while core carries only the two hooks they need: `Backend::is_host_driven`, so a backend can say that nobody may block waiting for input, and `Application::step`, which runs one pass of the event loop.

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

## Grid retirement

`Grid` has been retired. The core `Table` widget now draws the same separators itself: call `Table::set_separators(true)` to get the column separators in each line's own colour.

## Building and testing

```sh
cargo test
cargo clippy --all-targets -- -D warnings
sh scripts/check-wasm.sh
```

The documentation site lives in `website/` and is built with `mkdocs build -f website/mkdocs.yml` from the pages in `docs/`.

## License

MIT, see [LICENSE](LICENSE).
