# Changelog

## [Unreleased]

### Added

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

- `tv-extensions` now tracks turbo-vision `main` (pinned by commit, not by
  release), to carry the host-driven hooks (`Backend::is_host_driven`,
  `Application::step`) this crate depends on, ahead of a core release that
  includes them.
- `tv-extensions` is now the home for every niche feature that left
  turbo-vision core: the CSV editor, the logging window, Kitty/ANSI
  graphics, remote input, SSH, `ScrollPane` and popup menus, plus
  host-driven embedding for a host that owns the screen and the event loop.
  All of it is built on core's public API and the hooks core exposes for
  exactly this crate: `Terminal::event_injector`, the capture hook,
  `Terminal::write_raw`, the public `InputParser`, and host-driven
  `Backend::is_host_driven`/`Application::step`.
