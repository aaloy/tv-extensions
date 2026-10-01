# Remote input

`remote-input` (feature `remote-input`, implies `native`) is a testing and
automation aid: when enabled, turbo-vision listens on a local TCP port and
converts incoming text lines into keyboard and mouse events, injected into
the application's event loop exactly as if they had been typed or clicked.
It is off by default.

## Enabling it

```rust,no_run
use turbo_vision::terminal::Terminal;

let mut terminal = Terminal::init()?;
tv_extensions::remote_input::enable(&mut terminal, 8888)?;
# Ok::<(), std::io::Error>(())
```

Or read the port from the environment — `enable_from_env` does nothing and
returns `Ok(false)` when `TV_REMOTE_KEYS` is unset or not a `u16`:

```rust,no_run
# use turbo_vision::terminal::Terminal;
# let mut terminal = Terminal::init()?;
tv_extensions::remote_input::enable_from_env(&mut terminal)?;
# Ok::<(), std::io::Error>(())
```

The listener binds to `127.0.0.1` only, so it is reachable from the local
machine but not from the network.

## Protocol

Each line is either a mouse click — `CLICK x y` / `RCLICK x y` / `MCLICK x y`
(0-indexed cell coordinates; left, right, middle button) — or one or more
whitespace-separated key chords, parsed by
`turbo_vision::core::event::parse_key_chord`:

```bash
printf 'CTRL+F12\n'    | nc 127.0.0.1 8888   # key chord(s)
printf 'CLICK 28 23\n' | nc 127.0.0.1 8888   # left-click at cell (28, 23)
```

A blank or unparseable line yields nothing; an unparseable chord on an
otherwise valid line is logged and skipped.

## Screenshots

This module does not special-case `Ctrl+F12` or `F12`; an injected chord is
served by whatever already handles that key in the running application.
By default that is turbo-vision core's built-in capture — a PNG screenshot
when core's `screenshot` feature is enabled, an ANSI text dump of the screen
otherwise — or, if the application installed one, its own capture hook via
`Terminal::set_capture_hook`. Either way, `remote_input` only gets the key
to the application; it has no part in what the key then does.
