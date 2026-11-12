//! LSP (Language Server Protocol) client integration.
//!
//! Moce communicates with language servers (like rust-analyzer, pyright, etc.)
//! via stdio using the JSON-RPC 2.0 protocol defined in the LSP specification.
//!
//! The client runs the server as a child process and reads/writes messages
//! asynchronously on a Tokio background task. Each frame, the app drains
//! pending messages and applies diagnostics, completions, etc.

pub mod client;
pub mod types;

pub use client::{LspClient, ServerMessage};
pub use types::*;
