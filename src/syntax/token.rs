//! Token types for syntax highlighting.
//!
//! Each token type maps to a color in the theme. The highlighter produces
//! a sequence of tokens per line, and the editor view paints them
//! accordingly.

/// The kind of a syntax token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenType {
    /// Default text with no specific classification.
    Text,
    /// Language keywords (if, let, fn, etc.).
    Keyword,
    /// Control-flow keywords (if, else, while, return).
    ControlKeyword,
    /// Type names (String, Vec, etc.).
    Type,
    /// Function names at call or definition sites.
    Function,
    /// String literals.
    String,
    /// Numeric literals.
    Number,
    /// Comments (line and block).
    Comment,
    /// Doc comments.
    DocComment,
    /// Operators (+, -, *, =, etc.).
    Operator,
    /// Punctuation (parens, brackets, commas, etc.).
    Punctuation,
    /// Identifier that's not a keyword or type.
    Identifier,
    /// Macro invocations.
    Macro,
    /// Attribute annotations (#[...]).
    Attribute,
    /// Lifetime or generic parameter ('a, <T>).
    Lifetime,
    /// Boolean and null literals (true, false, null, None).
    Boolean,
    /// Constant / static variable.
    Constant,
}

impl TokenType {
    /// Get the theme key name for this token type.
    pub fn theme_key(self) -> &'static str {
        match self {
            TokenType::Text => "text",
            TokenType::Keyword => "keyword",
            TokenType::ControlKeyword => "control_keyword",
            TokenType::Type => "type",
            TokenType::Function => "function",
            TokenType::String => "string",
            TokenType::Number => "number",
            TokenType::Comment => "comment",
            TokenType::DocComment => "doc_comment",
            TokenType::Operator => "operator",
            TokenType::Punctuation => "punctuation",
            TokenType::Identifier => "identifier",
            TokenType::Macro => "macro",
            TokenType::Attribute => "attribute",
            TokenType::Lifetime => "lifetime",
            TokenType::Boolean => "boolean",
            TokenType::Constant => "constant",
        }
    }
}

/// A single syntax token with its byte span within a line.
#[derive(Clone, Copy, Debug)]
pub struct Token {
    /// Start byte offset within the line.
    pub start: usize,
    /// End byte offset within the line (exclusive).
    pub end: usize,
    /// The token type.
    pub kind: TokenType,
}

impl Token {
    pub fn new(start: usize, end: usize, kind: TokenType) -> Self {
        Self { start, end, kind }
    }

    /// Length of the token in bytes.
    #[inline]
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}
