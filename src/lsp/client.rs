//! LSP client — manages a child process (the language server) and
//! communicates with it via JSON-RPC over stdio.
//!
//! This runs the read loop on a Tokio task so it doesn't block the UI.
//! Incoming messages are queued and drained each frame by the app.

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::mpsc::{self, Receiver, Sender, UnboundedReceiver, UnboundedSender};

use super::types::*;

/// A message received from the language server.
#[derive(Debug)]
pub enum ServerMessage {
    /// A response to a request we made.
    Response { id: u64, result: serde_json::Value },
    /// An error response.
    Error { id: u64, message: String },
    /// A notification pushed by the server (e.g. `textDocument/publishDiagnostics`).
    Notification { method: String, params: serde_json::Value },
}

/// An active LSP server connection.
pub struct LspClient {
    /// Outgoing messages (to be written to the server's stdin).
    tx: UnboundedSender<String>,
    /// Incoming messages received from the server.
    rx: UnboundedReceiver<ServerMessage>,
    /// Next request ID.
    next_id: Arc<AtomicU64>,
    /// Whether the connection has been initialized.
    initialized: bool,
    /// Open document versions.
    doc_versions: HashMap<String, i64>,
}

impl LspClient {
    /// Spawn a language server process and connect to it.
    pub async fn spawn(cmd: &str, args: &[&str]) -> std::io::Result<Self> {
        let mut process = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let stdin = process.stdin.take().expect("No stdin");
        let stdout = process.stdout.take().expect("No stdout");

        let (out_tx, out_rx) = mpsc::unbounded_channel::<String>();
        let (in_tx, in_rx) = mpsc::unbounded_channel::<ServerMessage>();

        // Writer task: pulls messages from out_rx and writes to stdin
        tokio::spawn(Self::write_loop(stdin, out_rx));

        // Reader task: reads from stdout and pushes to in_tx
        tokio::spawn(Self::read_loop(stdout, in_tx));

        Ok(Self {
            tx: out_tx,
            rx: in_rx,
            next_id: Arc::new(AtomicU64::new(1)),
            initialized: false,
            doc_versions: HashMap::new(),
        })
    }

    /// Writer task: encodes messages as LSP frames and writes them to stdin.
    async fn write_loop(
        mut stdin: tokio::process::ChildStdin,
        mut rx: UnboundedReceiver<String>,
    ) {
        while let Some(msg) = rx.recv().await {
            let header = format!("Content-Length: {}\r\n\r\n", msg.len());
            if let Err(e) = stdin.write_all(header.as_bytes()).await {
                log::error!("LSP write error (header): {}", e);
                break;
            }
            if let Err(e) = stdin.write_all(msg.as_bytes()).await {
                log::error!("LSP write error (body): {}", e);
                break;
            }
        }
    }

    /// Reader task: reads LSP frames from stdout and parses them.
    async fn read_loop(
        stdout: tokio::process::ChildStdout,
        tx: UnboundedSender<ServerMessage>,
    ) {
        let mut reader = BufReader::new(stdout);
        loop {
            // Read headers
            let mut content_length: Option<usize> = None;
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line).await {
                    Ok(0) => return, // EOF
                    Ok(_) => {
                        let line = line.trim();
                        if line.is_empty() {
                            break; // End of headers
                        }
                        if let Some(rest) = line.strip_prefix("Content-Length: ") {
                            content_length = rest.trim().parse().ok();
                        }
                    }
                    Err(e) => {
                        log::error!("LSP read error: {}", e);
                        return;
                    }
                }
            }

            let len = match content_length {
                Some(l) => l,
                None => {
                    log::warn!("LSP message without Content-Length");
                    continue;
                }
            };

            let mut body = vec![0u8; len];
            if let Err(e) = reader.read_exact(&mut body).await {
                log::error!("LSP body read error: {}", e);
                return;
            }

            let body = match String::from_utf8(body) {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("LSP non-UTF8 response: {}", e);
                    continue;
                }
            };

            match serde_json::from_str::<serde_json::Value>(&body) {
                Ok(json) => {
                    let msg = parse_server_message(json);
                    if let Some(msg) = msg {
                        let _ = tx.send(msg);
                    }
                }
                Err(e) => {
                    log::warn!("LSP JSON parse error: {} | body: {}", e, &body[..body.len().min(200)]);
                }
            }
        }
    }

    /// Send a JSON-RPC request with an auto-incremented ID.
    /// Returns the ID that was used (to match the future response).
    pub fn request(&self, method: &str, params: serde_json::Value) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let msg = LspMessage::request(id, method, params);
        self.send(msg);
        id
    }

    /// Send a JSON-RPC notification (no response expected).
    pub fn notify(&self, method: &str, params: serde_json::Value) {
        let msg = LspMessage::notification(method, params);
        self.send(msg);
    }

    fn send(&self, msg: LspMessage) {
        match serde_json::to_string(&msg) {
            Ok(json) => {
                let _ = self.tx.send(json);
            }
            Err(e) => {
                log::error!("LSP serialization error: {}", e);
            }
        }
    }

    /// Initialize the server (must be called before any other requests).
    pub fn initialize(&mut self, root_uri: Option<String>) {
        let params = serde_json::json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "capabilities": {
                "textDocument": {
                    "completion": {
                        "completionItem": { "snippetSupport": false }
                    },
                    "hover": {},
                    "definition": {},
                    "publishDiagnostics": {}
                },
                "workspace": {
                    "workspaceFolders": true
                }
            }
        });
        self.request("initialize", params);
    }

    /// Confirm initialization (send after receiving the initialize response).
    pub fn initialized(&mut self) {
        self.initialized = true;
        self.notify("initialized", serde_json::json!({}));
    }

    /// Notify the server that a document was opened.
    pub fn did_open(&mut self, uri: &str, language_id: &str, text: &str) {
        let version = self.doc_versions.entry(uri.to_string()).or_insert(0);
        *version += 1;
        let v = *version;
        self.notify(
            "textDocument/didOpen",
            serde_json::json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id,
                    "version": v,
                    "text": text
                }
            }),
        );
    }

    /// Notify the server that a document changed (full content sync).
    pub fn did_change(&mut self, uri: &str, text: &str) {
        let version = self.doc_versions.entry(uri.to_string()).or_insert(0);
        *version += 1;
        let v = *version;
        self.notify(
            "textDocument/didChange",
            serde_json::json!({
                "textDocument": { "uri": uri, "version": v },
                "contentChanges": [{ "text": text }]
            }),
        );
    }

    /// Request completions at a position.
    pub fn completion(&self, uri: &str, line: usize, character: usize) -> u64 {
        self.request(
            "textDocument/completion",
            serde_json::json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
        )
    }

    /// Request hover information at a position.
    pub fn hover(&self, uri: &str, line: usize, character: usize) -> u64 {
        self.request(
            "textDocument/hover",
            serde_json::json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
        )
    }

    /// Request go-to-definition.
    pub fn definition(&self, uri: &str, line: usize, character: usize) -> u64 {
        self.request(
            "textDocument/definition",
            serde_json::json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
        )
    }

    /// Drain all pending incoming messages from the server.
    pub fn drain_messages(&mut self) -> Vec<ServerMessage> {
        let mut msgs = Vec::new();
        while let Ok(msg) = self.rx.try_recv() {
            msgs.push(msg);
        }
        msgs
    }

    /// Whether the client is initialized and ready.
    pub fn is_ready(&self) -> bool {
        self.initialized
    }
}

fn parse_server_message(json: serde_json::Value) -> Option<ServerMessage> {
    if let Some(method) = json.get("method").and_then(|m| m.as_str()) {
        // Notification or server-initiated request
        let params = json.get("params").cloned().unwrap_or(serde_json::Value::Null);
        return Some(ServerMessage::Notification {
            method: method.to_string(),
            params,
        });
    }

    if let Some(id_val) = json.get("id") {
        let id = id_val.as_u64().unwrap_or(0);
        if let Some(error) = json.get("error") {
            let msg = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error")
                .to_string();
            return Some(ServerMessage::Error { id, message: msg });
        }
        if let Some(result) = json.get("result") {
            return Some(ServerMessage::Response {
                id,
                result: result.clone(),
            });
        }
    }

    None
}
