// (C) 2026 - Enzo Lombardi

//! SSH server support for turbo-vision applications.
//!
//! This module provides infrastructure for serving turbo-vision TUI applications
//! over SSH connections using the russh library.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                        SSH Server                                │
//! ├──────────────────────────────────────────────────────────────────┤
//! │                                                                  │
//! │  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐          │
//! │  │ SSH Client  │    │ SSH Client  │    │ SSH Client  │   ...    │
//! │  │ Connection  │    │ Connection  │    │ Connection  │          │
//! │  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘          │
//! │         │                  │                  │                  │
//! │         ▼                  ▼                  ▼                  │
//! │  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐          │
//! │  │ TuiHandler  │    │ TuiHandler  │    │ TuiHandler  │          │
//! │  │ (per conn)  │    │ (per conn)  │    │ (per conn)  │          │
//! │  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘          │
//! │         │                  │                  │                  │
//! │         ▼                  ▼                  ▼                  │
//! │  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐          │
//! │  │   TUI App   │    │   TUI App   │    │   TUI App   │          │
//! │  │ (Terminal)  │    │ (Terminal)  │    │ (Terminal)  │          │
//! │  └─────────────┘    └─────────────┘    └─────────────┘          │
//! │                                                                  │
//! └──────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use tv_extensions::ssh::{SshServer, SshServerConfig};
//! use turbo_vision::terminal::Terminal;
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = SshServerConfig::new()
//!         .bind_addr("0.0.0.0:2222");
//!
//!     let server = SshServer::new(config, |backend| {
//!         // Create your TUI application with the SSH backend
//!         let terminal = Terminal::with_backend(backend).unwrap();
//!         // Run your app...
//!     });
//!
//!     server.run().await.unwrap();
//! }
//! ```

mod backend;
mod handler;
mod server;

pub use backend::{SshBackend, SshSessionBuilder, SshSessionHandle};
pub use handler::{TuiHandler, TuiSession};
pub use server::{
    AppFactory, PasswordAuthFn, PublicKeyAuthFn, SshAuthPolicy, SshServer, SshServerConfig,
    run_ssh_server,
};
