# Changelog

## [0.3.1] - 2026-10-02

Requires turbo-vision 4.0.2, which sends the cursor to the backend on
`flush` rather than at once.

### Fixed

- `host::pump` flushes after its final draw, so `HostInput::cursor()` (and
  `csv::Session::cursor()`) always match the frame in `Terminal::buffer`,
  including a cursor requested during the pump's own idle tick.
- `examples/csv_edit.rs` sets the cursor before flushing, so it no longer
  trails the typed text by one frame.

## [0.3.0] - 2026-10-02

Requires turbo-vision 4.0.1: the cursor below depends on its fix forwarding
`Desktop::update_cursor` to the focused window (4.0.0's `Desktop` never
called it, so no window's cursor ever reached the backend).

### Added

- `host`: `HostInput::cursor()` reports the screen cell where the
  application shows its text cursor after the last pump, or `None` while it
  hides it, so a host-driven embedder can draw one itself.
  `csv::Session::cursor()` exposes the same thing for the CSV editor;
  `examples/csv_edit.rs` now shows or hides the real terminal's cursor from
  it each frame.

## [0.2.0] - 2026-10-01

Tracks turbo-vision 4.0 from crates.io. Forwards the `screenshot` feature.

### Added

- `autocomplete`: `AutoComplete` and `AutoCompleteBuilder`, a text field
  that filters a suggestion list as you type and highlights the matched
  text. Keyboard and mouse selection, ASCII and Latin-1 input, colours from
  the owner's palette (dialog or window), and an optional `on_select`
  broadcast. Free text by default; `require_match` restricts the value to
  the items, reverting other text on leaving the field and refusing it in
  `valid()` so a dialog's OK cannot close on it. From the standalone `tvauto` crate. Example: `autocomplete`.
- `host`: host-driven applications for an embedder that owns the screen and
  the event loop (a WASM guest such as a plank frame) — `HostBackend`,
  `HostInput`, `pump`. Builds without turbo-vision's `native` feature, so
  the crate compiles for `wasm32-wasip1`.
- `scroll_pane`, `popup_menu`: `ScrollPane` (a scrolling interior for
  oversized dialogs) and modal popup/check-mark menus, from
  turbo-vision-extras.
- `keys`: key events from key names such as `"ctrl-s"` or `"enter"`, from
  plank-csvedit, for a host that reports keys as strings rather than
  terminal escape sequences.
- `csv` (feature `csv`): a CSV table editor — `Session`, `CsvDoc`,
  `Disk`/`FsDisk`/`MemDisk` — from plank-csvedit, without the plank
  dependency.
- `log` (feature `log`): `TerminalWidget` (a scrolling output pane) and
  `LogWindow`/`LogSubscriber` (a window that shows `tracing` events), from
  turbo-vision core.
- `graphics` (feature `graphics`): `KittyImage` and the Kitty protocol
  helpers, and `AnsiBackground`/`AnsiImage`, from turbo-vision core, writing
  straight to the terminal with `Terminal::write_raw`.
- `remote_input` (feature `remote-input`, implies `native`): key chords and
  mouse clicks injected over a local TCP port, for testing and automation,
  on top of core's `Terminal::event_injector`.
- `ssh` (feature `ssh`, implies `native`): serving a turbo-vision
  application over SSH (`SshServer`, `SshBackend`, `TuiHandler`), from
  turbo-vision core, built on `russh`.
- Examples: `controls`, `lazy_data`, `scroll_and_popup`, `csv_edit`,
  `kitty_image`, `kitty_background`, `kitty_biorhythm`, `desktop_logo`,
  `log_window`, `terminal_widget`, `ssh_server`. `desktop_logo` is a
  turbo-vision core example, carried over here to demonstrate the
  `graphics` feature; `csv_edit` is new, driving `csv::Session` from a
  real terminal. The extras demos (`controls`, `lazy_data`,
  `scroll_and_popup`) are rewritten on core widgets rather than the retired
  `turbo-vision-extras` API.

### Removed

- `Grid` was retired. The core `Table` widget now draws the same column
  separators itself: `Table::set_separators(true)`.

### Changed

- `tv-extensions` now depends on turbo-vision 4.0 from crates.io, which
  carries the host-driven hooks (`Backend::is_host_driven`,
  `Application::step`) this crate depends on, and forwards core's
  `screenshot` feature.
- `tv-extensions` is now the home for every niche feature that left
  turbo-vision core: the CSV editor, the logging window, Kitty/ANSI
  graphics, remote input, SSH, `ScrollPane` and popup menus, plus
  host-driven embedding for a host that owns the screen and the event loop.
  All of it is built on core's public API and the hooks core exposes for
  exactly this crate: `Terminal::event_injector`, the capture hook,
  `Terminal::write_raw`, the public `InputParser`, and host-driven
  `Backend::is_host_driven`/`Application::step`.
