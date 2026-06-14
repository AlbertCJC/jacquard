//! Token types for the Jacquard lexer.
//!
//! Defines the `Token` struct, `TokenKind` enum (~45 variants), `TokenCategory`,
//! and `Span` for source-location tracking.

/// A span of source code, tracking byte offsets and line/column position.
///
/// `start` and `end` are byte offsets into the original source string.
/// `line` and `col` are 1-based line and column numbers where the token starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub col: usize,
}

impl Span {
    /// Create a new span with the given bounds.
    pub fn new(start: usize, end: usize, line: usize, col: usize) -> Self {
        Span {
            start,
            end,
            line,
            col,
        }
    }

    /// The length of this span in bytes.
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Returns true if this span is empty (zero-length).
    pub fn is_empty(&self) -> bool {
        self.end == self.start
    }
}

/// A single token produced by the lexer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// The kind of token.
    pub kind: TokenKind,
    /// Source location of this token.
    pub span: Span,
    /// The exact source text that produced this token.
    pub lexeme: String,
}

/// Every possible kind of token the lexer can produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    // -- Keywords -----------------------------------------------------------
    Task,
    Workflow,
    Fn,
    Let,
    If,
    Else,
    While,
    For,
    Return,
    Match,
    Await,
    Async,
    Pub,
    Extern,
    Export,
    Import,
    Enum,
    Struct,
    Type,
    Parallel,

    // -- Literals -----------------------------------------------------------
    IntLiteral,
    FloatLiteral,
    StringLiteral,
    BoolLiteral,
    Identifier,

    // -- Delimiters ---------------------------------------------------------
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LAngle,
    RAngle,
    Comma,
    Semicolon,
    Colon,
    Dot,
    Arrow,
    FatArrow,

    // -- Operators ----------------------------------------------------------
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    EqEq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    Not,
    Pipe,
    Ampersand,
    Question,

    // -- Special ------------------------------------------------------------
    Eof,
    Error,
}

/// Broad categories for token kinds, used for parser dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenCategory {
    Keyword,
    Literal,
    Identifier,
    Delimiter,
    Operator,
    Special,
    Error,
}

impl TokenKind {
    /// Returns the broad category of this token kind.
    pub fn category(self) -> TokenCategory {
        match self {
            TokenKind::Task
            | TokenKind::Workflow
            | TokenKind::Fn
            | TokenKind::Let
            | TokenKind::If
            | TokenKind::Else
            | TokenKind::While
            | TokenKind::For
            | TokenKind::Return
            | TokenKind::Match
            | TokenKind::Await
            | TokenKind::Async
            | TokenKind::Pub
            | TokenKind::Extern
            | TokenKind::Export
            | TokenKind::Import
            | TokenKind::Enum
            | TokenKind::Struct
            | TokenKind::Type
            | TokenKind::Parallel => TokenCategory::Keyword,

            TokenKind::IntLiteral
            | TokenKind::FloatLiteral
            | TokenKind::StringLiteral
            | TokenKind::BoolLiteral => TokenCategory::Literal,

            TokenKind::Identifier => TokenCategory::Identifier,

            TokenKind::LParen
            | TokenKind::RParen
            | TokenKind::LBrace
            | TokenKind::RBrace
            | TokenKind::LBracket
            | TokenKind::RBracket
            | TokenKind::LAngle
            | TokenKind::RAngle
            | TokenKind::Comma
            | TokenKind::Semicolon
            | TokenKind::Colon
            | TokenKind::Dot
            | TokenKind::Arrow
            | TokenKind::FatArrow => TokenCategory::Delimiter,

            TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent
            | TokenKind::Eq
            | TokenKind::EqEq
            | TokenKind::NotEq
            | TokenKind::Lt
            | TokenKind::Gt
            | TokenKind::LtEq
            | TokenKind::GtEq
            | TokenKind::And
            | TokenKind::Or
            | TokenKind::Not
            | TokenKind::Pipe
            | TokenKind::Ampersand
            | TokenKind::Question => TokenCategory::Operator,

            TokenKind::Eof => TokenCategory::Special,

            TokenKind::Error => TokenCategory::Error,
        }
    }
}