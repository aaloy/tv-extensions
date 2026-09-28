#!/bin/sh
# tv-extensions as an embedder builds it: turbo-vision without its terminal,
# for a WASM guest such as a plank frame. Needs rustup's cargo and
# `rustup target add wasm32-wasip1`.
set -e
cd "$(dirname "$0")/.."
cargo check --lib --target wasm32-wasip1
