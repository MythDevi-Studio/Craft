//! Syntax highlighter: tokenizes lines of code for coloring.
//!
//! This is a lightweight, stateful tokenizer. It processes one line at a time
//! and carries state (like "inside a block comment") between lines. It's not
//! a real parser — it uses simple pattern matching — but it's fast and good
//! enough for display purposes.

use crate::editor::document::Language;

use super::{Token, TokenType};

/// State carried between lines during tokenization.
#[derive(Clone, Copy, Default)]
struct LineState {
    /// Whether we're inside a block comment.
    in_block_comment: bool,
}

/// The syntax highlighter.
///
/// Holds the current language and any inter-line state. Call `tokenize_line`
/// for each line of text to get a list of tokens.
pub struct Highlighter {
    language: Language,
    state: LineState,
}

impl Highlighter {
    /// Create a highlighter for the given language.
    pub fn new(language: Language) -> Self {
        Self {
            language,
            state: LineState::default(),
        }
    }

    /// Reset the highlighter state (e.g., when the buffer changes drastically).
    pub fn reset(&mut self) {
        self.state = LineState::default();
    }

    /// Change the language being highlighted.
    pub fn set_language(&mut self, language: Language) {
        self.language = language;
        self.state = LineState::default();
    }

    /// Tokenize a single line of text.
    ///
    /// Returns a list of tokens covering the entire line.
    pub fn tokenize_line(&mut self, line: &str) -> Vec<Token> {
        match self.language {
            Language::Rust => self.tokenize_rust(line),
            Language::JavaScript | Language::TypeScript => self.tokenize_js(line),
            Language::Python => self.tokenize_python(line),
            Language::Json => self.tokenize_json(line),
            Language::Css => self.tokenize_css(line),
            Language::Markdown => self.tokenize_markdown(line),
            Language::Html => self.tokenize_html(line),
            Language::PlainText => vec![Token::new(0, line.len(), TokenType::Text)],
        }
    }

    // ── Rust ──────────────────────────────────────────────────────────

    fn tokenize_rust(&mut self, line: &str) -> Vec<Token> {
        let keywords = [
            "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
            "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod",
            "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super",
            "trait", "true", "type", "unsafe", "use", "where", "while", "yield",
        ];
        let control = ["break", "continue", "else", "for", "if", "loop", "match", "return", "while"];
        let types = [
            "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
            "f32", "f64", "bool", "char", "str", "String", "Vec", "Option", "Result", "Box",
        ];
        let macros = ["println!", "eprintln!", "dbg!", "format!", "vec!", "todo!", "unimplemented!", "panic!"];

        self.tokenize_generic(line, &keywords, &control, &types, &macros, "//", "/*", "*/", true)
    }

    // ── JavaScript / TypeScript ─────────────────────────────────────────

    fn tokenize_js(&mut self, line: &str) -> Vec<Token> {
        let keywords = [
            "abstract", "async", "await", "break", "case", "catch", "class", "const", "continue",
            "debugger", "default", "delete", "do", "else", "enum", "export", "extends", "finally",
            "for", "from", "function", "get", "if", "implements", "import", "in", "instanceof",
            "interface", "let", "new", "of", "package", "private", "protected", "public",
            "return", "set", "static", "super", "switch", "this", "throw", "try", "type",
            "typeof", "undefined", "var", "void", "while", "with", "yield",
        ];
        let control = ["break", "case", "catch", "continue", "default", "do", "else", "finally", "for", "if", "return", "switch", "throw", "try", "while"];
        let types = [
            "boolean", "number", "string", "void", "any", "unknown", "never", "object",
            "Array", "Map", "Set", "Promise", "Date", "RegExp", "Error",
        ];

        self.tokenize_generic(line, &keywords, &control, &types, &[], "//", "/*", "*/", true)
    }

    // ── Python ────────────────────────────────────────────────────────

    fn tokenize_python(&mut self, line: &str) -> Vec<Token> {
        let keywords = [
            "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class",
            "continue", "def", "del", "elif", "else", "except", "finally", "for", "from",
            "global", "if", "import", "in", "is", "lambda", "nonlocal", "not", "or", "pass",
            "raise", "return", "try", "while", "with", "yield", "match", "case",
        ];
        let control = ["break", "continue", "else", "for", "if", "pass", "raise", "return", "try", "while", "yield"];
        let types = ["int", "float", "str", "bool", "list", "dict", "tuple", "set", "bytes", "object", "type"];

        self.tokenize_generic(line, &keywords, &control, &types, &[], "#", "\"\"\"", "\"\"\"", false)
    }

    // ── JSON ──────────────────────────────────────────────────────────

    fn tokenize_json(&mut self, line: &str) -> Vec<Token> {
        let keywords = ["true", "false", "null"];
        let mut tokens = Vec::new();
        let bytes = line.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            let c = bytes[i] as char;
            match c {
                ' ' | '\t' | '\r' => {
                    i += 1;
                }
                '"' => {
                    let start = i;
                    i += 1;
                    while i < bytes.len() {
                        if bytes[i] == b'\\' {
                            i += 2;
                        } else if bytes[i] == b'"' {
                            i += 1;
                            break;
                        } else {
                            i += 1;
                        }
                    }
                    // Check if this is a key (followed by :)
                    let mut j = i;
                    while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                        j += 1;
                    }
                    let is_key = j < bytes.len() && bytes[j] == b':';
                    let kind = if is_key {
                        TokenType::Function
                    } else {
                        TokenType::String
                    };
                    tokens.push(Token::new(start, i, kind));
                }
                '{' | '}' | '[' | ']' | ':' | ',' => {
                    tokens.push(Token::new(i, i + 1, TokenType::Punctuation));
                    i += 1;
                }
                '-' | '0'..='9' => {
                    let start = i;
                    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() || bytes[i] == b'.' || bytes[i] == b'e' || bytes[i] == b'E' || bytes[i] == b'+' || bytes[i] == b'-' {
                        i += 1;
                    }
                    tokens.push(Token::new(start, i, TokenType::Number));
                }
                _ => {
                    let start = i;
                    while i < bytes.len() && !matches!(bytes[i] as char, ' ' | '\t' | '"' | '{' | '}' | '[' | ']' | ':' | ',') {
                        i += 1;
                    }
                    let word = &line[start..i];
                    if keywords.contains(&word) {
                        tokens.push(Token::new(start, i, TokenType::Boolean));
                    } else {
                        tokens.push(Token::new(start, i, TokenType::Text));
                    }
                }
            }
        }
        if tokens.is_empty() {
            tokens.push(Token::new(0, line.len(), TokenType::Text));
        }
        tokens
    }

    // ── CSS ───────────────────────────────────────────────────────────

    fn tokenize_css(&mut self, line: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let bytes = line.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            let c = bytes[i] as char;
            match c {
                ' ' | '\t' => {
                    i += 1;
                }
                '/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                    let start = i;
                    i += 2;
                    while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                        i += 1;
                    }
                    if i + 1 < bytes.len() {
                        i += 2;
                    }
                    tokens.push(Token::new(start, i, TokenType::Comment));
                }
                '{' | '}' | ';' | ':' | ',' | '(' | ')' => {
                    tokens.push(Token::new(i, i + 1, TokenType::Punctuation));
                    i += 1;
                }
                '#' | '.' => {
                    let start = i;
                    i += 1;
                    while i < bytes.len() && (bytes[i] as char).is_alphanumeric() || bytes[i] == b'-' {
                        i += 1;
                    }
                    tokens.push(Token::new(start, i, TokenType::Function));
                }
                '@' => {
                    let start = i;
                    i += 1;
                    while i < bytes.len() && (bytes[i] as char).is_alphanumeric() || bytes[i] == b'-' {
                        i += 1;
                    }
                    tokens.push(Token::new(start, i, TokenType::Attribute));
                }
                ':' if i + 1 < bytes.len() && bytes[i + 1] == b':' => {
                    let start = i;
                    i += 2;
                    while i < bytes.len() && (bytes[i] as char).is_alphabetic() {
                        i += 1;
                    }
                    tokens.push(Token::new(start, i, TokenType::Keyword));
                }
                _ => {
                    let start = i;
                    while i < bytes.len() && !matches!(bytes[i] as char, ' ' | '\t' | '{' | '}' | ';' | ':' | ',' | '(' | ')' | '#') {
                        i += 1;
                    }
                    if i == start {
                        i += 1;
                    }
                    tokens.push(Token::new(start, i, TokenType::Identifier));
                }
            }
        }
        if tokens.is_empty() {
            tokens.push(Token::new(0, line.len(), TokenType::Text));
        }
        tokens
    }

    // ── Markdown ──────────────────────────────────────────────────────

    fn tokenize_markdown(&mut self, line: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        if line.starts_with('#') {
            tokens.push(Token::new(0, line.len(), TokenType::Keyword));
        } else if line.starts_with("```") {
            tokens.push(Token::new(0, line.len(), TokenType::Attribute));
        } else if line.starts_with("- ") || line.starts_with("* ") || line.starts_with("> ") {
            tokens.push(Token::new(0, 2, TokenType::Operator));
            tokens.push(Token::new(2, line.len(), TokenType::Text));
        } else if line.contains("**") || line.contains("*") || line.contains("`") {
            // Simplified: just mark the whole line as text
            tokens.push(Token::new(0, line.len(), TokenType::Text));
        } else {
            tokens.push(Token::new(0, line.len(), TokenType::Text));
        }
        tokens
    }

    // ── HTML ──────────────────────────────────────────────────────────

    fn tokenize_html(&mut self, line: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let bytes = line.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            if bytes[i] == b'<' {
                let start = i;
                i += 1;
                while i < bytes.len() && bytes[i] != b'>' {
                    i += 1;
                }
                if i < bytes.len() {
                    i += 1;
                }
                tokens.push(Token::new(start, i, TokenType::Keyword));
            } else if bytes[i] == b'"' {
                let start = i;
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    i += 1;
                }
                if i < bytes.len() {
                    i += 1;
                }
                tokens.push(Token::new(start, i, TokenType::String));
            } else {
                let start = i;
                while i < bytes.len() && bytes[i] != b'<' && bytes[i] != b'"' {
                    i += 1;
                }
                tokens.push(Token::new(start, i, TokenType::Text));
            }
        }
        if tokens.is_empty() {
            tokens.push(Token::new(0, line.len(), TokenType::Text));
        }
        tokens
    }

    // ── Generic tokenizer ─────────────────────────────────────────────
    //
    // Shared logic for C-like languages (Rust, JS/TS, Python). Uses simple
    // state-machine tokenization with the provided language-specific keyword
    // lists and comment delimiters.

    #[allow(clippy::too_many_arguments)]
    fn tokenize_generic(
        &mut self,
        line: &str,
        keywords: &[&str],
        control: &[&str],
        types: &[&str],
        macros: &[&str],
        line_comment: &str,
        block_comment_start: &str,
        block_comment_end: &str,
        has_lifetimes: bool,
    ) -> Vec<Token> {
        let bytes = line.as_bytes();
        let mut tokens = Vec::new();
        let mut i = 0;

        // Handle continuation of block comment from previous line
        if self.state.in_block_comment {
            let start = 0;
            if let Some(end_idx) = find_subslice(bytes, block_comment_end.as_bytes()) {
                i = end_idx + block_comment_end.len();
                self.state.in_block_comment = false;
                tokens.push(Token::new(start, i, TokenType::Comment));
                // Continue tokenizing the rest of the line
            } else {
                // Whole line is still in block comment
                tokens.push(Token::new(0, bytes.len(), TokenType::Comment));
                return tokens;
            }
        }

        while i < bytes.len() {
            let c = bytes[i] as char;

            // Skip whitespace
            if c.is_whitespace() {
                i += 1;
                continue;
            }

            // Line comments
            if line[i..].starts_with(line_comment) {
                tokens.push(Token::new(i, bytes.len(), TokenType::Comment));
                i = bytes.len();
                break;
            }

            // Doc comments (/// or //!)
            if line[i..].starts_with(&format!("{line_comment}{line_comment}")) || line[i..].starts_with(&format!("{line_comment}!")) {
                tokens.push(Token::new(i, bytes.len(), TokenType::DocComment));
                i = bytes.len();
                break;
            }

            // Block comments
            if line[i..].starts_with(block_comment_start) {
                if let Some(end_idx) = find_subslice(&bytes[i + block_comment_start.len()..], block_comment_end.as_bytes()) {
                    let end = i + block_comment_start.len() + end_idx + block_comment_end.len();
                    tokens.push(Token::new(i, end, TokenType::Comment));
                    i = end;
                    continue;
                } else {
                    // Block comment extends to next line
                    tokens.push(Token::new(i, bytes.len(), TokenType::Comment));
                    self.state.in_block_comment = true;
                    break;
                }
            }

            // String literals
            if c == '"' || c == '\'' {
                // Check if it's a char literal or a lifetime
                if c == '\'' && has_lifetimes {
                    // Could be a lifetime ('a) or a char ('x')
                    let mut j = i + 1;
                    if j < bytes.len() && (bytes[j] as char).is_alphabetic() {
                        j += 1;
                        while j < bytes.len() && (bytes[j] as char).is_alphanumeric() || bytes[j] == b'_' {
                            j += 1;
                        }
                        if j < bytes.len() && bytes[j] == b'\'' {
                            // Char literal
                            tokens.push(Token::new(i, j + 1, TokenType::String));
                            i = j + 1;
                            continue;
                        } else {
                            // Lifetime
                            tokens.push(Token::new(i, j, TokenType::Lifetime));
                            i = j;
                            continue;
                        }
                    }
                }

                let start = i;
                let quote = c;
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i += 2;
                    } else if bytes[i] as char == quote {
                        i += 1;
                        break;
                    } else {
                        i += 1;
                    }
                }
                tokens.push(Token::new(start, i, TokenType::String));
                continue;
            }

            // Numbers
            if c.is_ascii_digit() || (c == '-' && i + 1 < bytes.len() && (bytes[i + 1] as char).is_ascii_digit()) {
                let start = i;
                if c == '-' {
                    i += 1;
                }
                // Handle hex, octal, binary
                if i + 1 < bytes.len() && bytes[i] == b'0' && (bytes[i + 1] == b'x' || bytes[i + 1] == b'X' || bytes[i + 1] == b'b' || bytes[i + 1] == b'o' || bytes[i + 1] == b'B' || bytes[i + 1] == b'O') {
                    i += 2;
                    while i < bytes.len() && (bytes[i] as char).is_alphanumeric() || bytes[i] == b'_' {
                        i += 1;
                    }
                } else {
                    while i < bytes.len() && ((bytes[i] as char).is_ascii_digit() || bytes[i] == b'.' || bytes[i] == b'e' || bytes[i] == b'E' || bytes[i] == b'_') {
                        i += 1;
                    }
                    // Handle numeric suffixes like 10u32, 3.14f64
                    while i < bytes.len() && (bytes[i] as char).is_alphabetic() {
                        i += 1;
                    }
                }
                tokens.push(Token::new(start, i, TokenType::Number));
                continue;
            }

            // Attributes / decorators
            if c == '#' || c == '@' {
                let start = i;
                i += 1;
                if i < bytes.len() && bytes[i] == b'[' {
                    while i < bytes.len() && bytes[i] != b']' {
                        i += 1;
                    }
                    if i < bytes.len() {
                        i += 1;
                    }
                }
                tokens.push(Token::new(start, i, TokenType::Attribute));
                continue;
            }

            // Identifiers and keywords
            if c.is_alphabetic() || c == '_' {
                let start = i;
                i += 1;
                while i < bytes.len() && (bytes[i] as char).is_alphanumeric() || bytes[i] == b'_' {
                    i += 1;
                }
                let word = &line[start..i];

                // Check for macro (ends with !)
                let is_macro = i < bytes.len() && bytes[i] == b'!';
                if is_macro && macros.contains(&word) {
                    i += 1; // consume the !
                    tokens.push(Token::new(start, i, TokenType::Macro));
                    continue;
                }

                if control.contains(&word) {
                    tokens.push(Token::new(start, i, TokenType::ControlKeyword));
                } else if keywords.contains(&word) {
                    tokens.push(Token::new(start, i, TokenType::Keyword));
                } else if word == "true" || word == "false" || word == "None" {
                    tokens.push(Token::new(start, i, TokenType::Boolean));
                } else if types.contains(&word) {
                    tokens.push(Token::new(start, i, TokenType::Type));
                } else if word.chars().next().map_or(false, |c| c.is_uppercase()) {
                    tokens.push(Token::new(start, i, TokenType::Type));
                } else {
                    // Check if it's a function call (next non-space char is '(')
                    let mut j = i;
                    while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                        j += 1;
                    }
                    if j < bytes.len() && bytes[j] == b'(' {
                        tokens.push(Token::new(start, i, TokenType::Function));
                    } else {
                        tokens.push(Token::new(start, i, TokenType::Identifier));
                    }
                }
                continue;
            }

            // Operators and punctuation
            if "+-*/%=<>!&|^~?".contains(c) {
                let start = i;
                i += 1;
                // Multi-char operators
                while i < bytes.len() && "=<>|&".contains(bytes[i] as char) {
                    i += 1;
                }
                tokens.push(Token::new(start, i, TokenType::Operator));
                continue;
            }

            if "{}()[];:,".contains(c) {
                tokens.push(Token::new(i, i + 1, TokenType::Punctuation));
                i += 1;
                continue;
            }

            // Default: single character text
            tokens.push(Token::new(i, i + 1, TokenType::Text));
            i += 1;
        }

        if tokens.is_empty() {
            tokens.push(Token::new(0, 0, TokenType::Text));
        }
        tokens
    }
}

/// Find the first occurrence of a byte pattern in a slice.
fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    (0..=haystack.len() - needle.len()).find(|&i| &haystack[i..i + needle.len()] == needle)
}
