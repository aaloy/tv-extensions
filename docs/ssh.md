# SSH server

`ssh` (feature `ssh`, implies `native`) serves a turbo-vision application
over SSH, built on [`russh`](https://docs.rs/russh). Each connection gets
its own `Terminal`, backed by `SshBackend`, so the TUI runs exactly as it
would locally, with input and output carried over the SSH channel instead
of a local PTY.

## Quick start

```rust,ignore
use tv_extensions::ssh::{SshServer, SshServerConfig};
use turbo_vision::terminal::Terminal;

#[tokio::main]
async fn main() {
    let config = SshServerConfig::new()
        .bind_addr("0.0.0.0:2222")
        .load_or_generate_key("ssh_host_key")
        .auth_password_fn(|user, password| user == "demo" && password == "demo");

    let server = SshServer::new(config, || {
        Box::new(|backend| {
            let _terminal = Terminal::with_backend(backend).unwrap();
            // run the application with `_terminal`...
        })
    });

    server.run().await.unwrap();
}
```

See `examples/ssh_server.rs` for a full server showing a modal "Quit"
dialog to each connecting client:

```sh
cargo run --example ssh_server --features ssh
ssh -p 2222 user@localhost
```

The `ssh_server` example does not install a `log` logger, so any `log`
output from the SSH layer (including from `remote_input`, if also enabled)
goes nowhere unless the hosting application installs one of its own.

`run_ssh_server(addr, factory)` is a shorthand for `SshServerConfig::new()`
with a generated key plus `SshServer::new(..).run()`, for a server with no
further configuration.

## Authentication

`SshServerConfig` rejects every authentication attempt until told
otherwise. Set `auth_password_fn(|user, password| ...)` and/or
`auth_publickey_fn(|user, key| ...)` to check credentials, or call
`allow_anonymous()` to accept everything — demos and trusted networks only;
it logs a warning when used, and the `ssh_server` example relies on it.

## Host keys

`generate_key()` adds a random Ed25519 key for the life of the process;
`load_or_generate_key(path)` loads a key saved at `path`, or generates and
saves one if it doesn't exist yet, so the server's host key — and so its
fingerprint, which SSH clients cache and warn about on change — stays
stable across restarts.

## Architecture

Each SSH channel gets an `SshBackend` (implements
`turbo_vision::terminal::Backend` over channels to the async SSH handler)
and an `SshSessionHandle` (the handler's side: `process_input` parses
client bytes into events, `resize` updates the shared terminal size and
broadcasts a redraw, `try_recv_output` drains bytes to send back to the
client). `TuiHandler` is the `russh::server::Handler` that wires a new
connection's channel, PTY and shell/exec requests to one `SshBackend` pair
and runs the application factory on a blocking task once the shell starts.
