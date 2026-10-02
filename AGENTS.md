# AGENTS.md

This file provides guidance to the agent when working with code in this repository.

## What this is

`tv-extensions`: a single Rust crate (edition 2024) of extensions for
[turbo-vision](https://github.com/aovestdipaperino/turbo-vision-4-rust) —
host-driven embedding, `ScrollPane`, popup menus, key translation, and the
feature-gated `csv`, `log`, `graphics`, `remote-input` and `ssh` modules.
Not a monorepo. The pages in `docs/` are published as the Extensions section
of tv.enzolombardi.net: core's `website/sync_docs.py` copies them from this
checkout, then core's site is built and deployed. `website/` here only
previews them locally.

The core crate lives as a sibling checkout at `/Users/enzo/Code/turbo-vision-4-rust`.
The host-driven hooks this crate builds on (`Backend::is_host_driven`,
`Application::step`, `Terminal::event_injector`, `Terminal::write_raw`,
`InputParser`) live in core — changes to them belong there, and this crate
consumes them from crates.io. The pin is `turbo-vision >= 4.1.0` (4.0.1/4.0.2
carry the `update_cursor`/flush fixes `HostInput::cursor()` depends on;
4.1.0 adds the `Event::typed_char` that `autocomplete` reads).

## Commands

```sh
cargo test                                  # unit + integration + doctests
cargo clippy --all-targets -- -D warnings   # the gate, must stay clean
sh scripts/check-wasm.sh                    # wasm32-wasip1 check
```

- `scripts/check-wasm.sh` needs `rustup target add wasm32-wasip1`. If it
  fails with "can't find crate for `core`", the target is missing.
- Examples are feature-gated via `required-features`; `cargo build --examples`
  silently skips most of them. Run one explicitly, e.g.
  `cargo run --example csv_edit --features csv,native`.
- Docs preview: `mkdocs build -f website/mkdocs.yml` (deps in `website/requirements.txt`).
  To publish, run core's `python3 website/sync_docs.py` and deploy core's site.
- There is no CI. The gate is the global pre-commit hook
  (`core.hooksPath=~/.githooks`): rustfmt on staged `.rs` files plus
  `cargo clippy --workspace --all-targets -- -D warnings`.

## Style rules that differ from the default

- `missing_docs = "warn"`: every public item needs a doc comment.
  `unsafe_code = "forbid"`.
- Clippy `pedantic` is on as warnings and both the default-features build (the
  hook enforces it) and `--all-features --all-targets` are `-D warnings` clean.
  The cast family and `too_many_lines` are allowed in `Cargo.toml` for the
  `i16` geometry and the fixed-width wire formats; fix new lints, don't allow
  anything else.
- turbo-vision is depended on with `default-features = false` so the default
  feature set builds for `wasm32-wasip1`. Never add a default dependency that
  breaks that; terminal/OS things go behind `native` (or `remote-input`/`ssh`,
  which imply it).

## Gotchas

- Host-driven applications never run a nested event loop: calls that would
  block (`exec_view`, history popups) return `CM_CANCEL` instead. See
  `docs/host.md` before changing `host` or `csv::Session`.
- Some modules were copied from turbo-vision core/extras; keep their error
  handling when touching them. As of 2026-10-22 the whole default-features
  tree is clippy `-D warnings` clean — fix new lints, don't allow them.
- Releases: bump `version` in `Cargo.toml` and add a `CHANGELOG.md` section;
  changelog entries state the required turbo-vision version. `Cargo.lock` is
  gitignored; `website/`, `.superpowers/` and `logo.png` are excluded from the
  published package via `[package] exclude`.

## Repo etiquette

- Commit directly to `main`; no branches, no PRs.
- Conventional Commits with a scope: `fix(host): ...`, `feat(csv): ...`,
  `build: ...`, `docs(readme): ...`.
- TDD pairs are committed as separate `test: ... (RED)` then
  `feat: ... (GREEN)` commits.
