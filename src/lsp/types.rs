//! LSP protocol types (a minimal subset of the LSP spec).
//!
//! We only define the types we actually use. If you need more, the full
//! spec is at https://microsoft.github.io/language-server-protocol/specification

use serde::{Deserialize, Serialize};

pub type RequestId = serde_json::Value;

/// An LSP message (JSON-RPC 2.0).
#[derive(Debug, Serialize, Deserialize)]
pub struct LspMessage {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RequestId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<LspError>,
}

impl LspMessage {
    pub fn request(id: u64, method: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(id.into())),
            method: Some(method.into()),
            params: Some(params),
            result: None,
            error: None,
        }
    }

    pub fn notification(method: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: Some(method.into()),
            params: Some(params),
            result: None,
            error: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LspError {
    pub code: i32,
    pub message: String,
}

/// A position in an LSP text document (0-based line and character).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

impl LspPosition {
    pub fn new(line: usize, character: usize) -> Self {
        Self {
            line: line as u32,
            character: character as u32,
        }
    }
}

/// An LSP range.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

/// A text document identifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocumentIdentifier {
    pub uri: String,
}

/// A text document item (for open notifications).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentItem {
    pub uri: String,
    pub language_id: String,
    pub version: i64,
    pub text: String,
}

/// A text document position (identifier + position).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentPositionParams {
    pub text_document: TextDocumentIdentifier,
    pub position: LspPosition,
}

/// A single diagnostic message from the language server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: LspRange,
    pub severity: Option<u8>,
    pub message: String,
    pub source: Option<String>,
}

impl Diagnostic {
    /// Severity: 1=error, 2=warning, 3=info, 4=hint.
    pub fn is_error(&self) -> bool {
        self.severity == Some(1)
    }

    pub fn is_warning(&self) -> bool {
        self.severity == Some(2)
    }
}

/// A completion item from the language server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(rename = "insertText", skip_serializing_if = "Option::is_none")]
    pub insert_text: Option<String>,
}

/// Hover information returned by the language server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hover {
    pub contents: HoverContents,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HoverContents {
    Markup(MarkupContent),
    String(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkupContent {
    pub kind: String,
    pub value: String,
}

/// Convert our editor Position to an LSP position.
pub fn to_lsp_position(pos: crate::editor::Position) -> LspPosition {
    LspPosition::new(pos.line, pos.col)
}

/// Convert a file path to an LSP URI.
pub fn path_to_uri(path: &std::path::Path) -> String {
    format!("file:///{}", path.to_string_lossy().replace('\\', "/"))
}
