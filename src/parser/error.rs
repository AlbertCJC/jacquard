//! Parse error types for the Jacquard parser.

use crate::lexer::{Span, TokenKind};

/// A parsing error with context about what was expected and what was found.
#[derive(Debug, Clone)]
pub struct ParseError {
    /// Human-readable error message.
    pub message: String,
    /// Source span where the error occurred.
    pub span: Span,
    /// Token kinds that would have been valid at this position.
    pub expected: Vec<TokenKind>,
    /// The token kind actually encountered, or `None` for end-of-input.
    pub found: Option<TokenKind>,
}

impl ParseError {
    /// Create a parse error from a free-form message and span.
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        ParseError {
            message: message.into(),
            span,
            expected: vec![],
            found: None,
        }
    }

    /// Create a parse error describing expected tokens and what was found.
    pub fn expected(expected: Vec<TokenKind>, found: Option<TokenKind>, span: Span) -> Self {
        let found_str = match found {
            Some(k) => format!("{k:?}"),
            None => "end of input".to_string(),
        };
        let expected_str = expected
            .iter()
            .map(|k| format!("{k:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        let message = format!("expected one of [{expected_str}], found {found_str}");
        ParseError {
            message,
            span,
            expected,
            found,
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Parse error at {}:{}: {}",
            self.span.line, self.span.col, self.message
        )
    }
}

impl std::error::Error for ParseError {}