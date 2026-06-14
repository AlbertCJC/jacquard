//! Lexer (tokenizer) for Jacquard source code.
//!
//! Converts a raw source string into a stream of tokens with span information.
//! The lexer is a pure iterator — it yields tokens on demand without buffering
//! the entire token stream into memory.
//!
//! ## Token types
//! Approximately 59 token types covering:
//! - Keywords: `task`, `workflow`, `fn`, `let`, `if`, `else`, `return`, etc.
//! - Literals: integers, floats, strings, booleans
//! - Operators and punctuation
//! - Identifiers
//! - Comments (stripped, not emitted as tokens)

mod token;

pub use token::{Span, Token, TokenCategory, TokenKind};

/// A lexing error with a human-readable message and source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

/// The Jacquard lexer — an iterator that produces tokens from source text.
///
/// Uses a peekable character iterator for single-char lookahead. Tracks
/// line/column position for error reporting and span construction.
pub struct Lexer<'a> {
    /// The original source text (needed for span construction and EOF position).
    source: &'a str,
    /// Peekable character iterator yielding `(byte_offset, char)` pairs.
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    /// Current line number (1-based).
    line: usize,
    /// Current column number (1-based).
    col: usize,
    /// Byte offset where the current token being lexed starts.
    start_offset: usize,
    /// Line number where the current token starts.
    start_line: usize,
    /// Column number where the current token starts.
    start_col: usize,
    /// Whether we have already emitted the EOF token.
    eof_emitted: bool,
}

/// Create a new lexer for the given source string.
pub fn tokenize(source: &str) -> Lexer<'_> {
    Lexer::new(source)
}

impl<'a> Lexer<'a> {
    /// Create a new lexer from source text.
    pub fn new(source: &'a str) -> Self {
        Lexer {
            source,
            chars: source.char_indices().peekable(),
            line: 1,
            col: 1,
            start_offset: 0,
            start_line: 1,
            start_col: 1,
            eof_emitted: false,
        }
    }

    // ------------------------------------------------------------------
    // Iterator helpers
    // ------------------------------------------------------------------

    /// Peek at the next character without consuming it.
    fn peek_char(&mut self) -> Option<(usize, char)> {
        self.chars.peek().copied()
    }

    /// Advance one character, updating line/column tracking.
    ///
    /// Returns the byte offset and character, or `None` at end-of-input.
    fn advance(&mut self) -> Option<(usize, char)> {
        match self.chars.next() {
            Some((offset, '\n')) => {
                self.line += 1;
                self.col = 1;
                Some((offset, '\n'))
            }
            Some((offset, ch)) => {
                self.col += 1;
                Some((offset, ch))
            }
            None => None,
        }
    }

    /// Build a span from `self.start_offset` to `end`, using the saved start
    /// line/column.
    fn current_span(&self, end: usize) -> Span {
        Span::new(self.start_offset, end, self.start_line, self.start_col)
    }

    /// Build a `LexError` with the given message, spanning to `end`.
    fn error(&self, msg: impl Into<String>, end: usize) -> LexError {
        LexError {
            message: msg.into(),
            span: Span::new(self.start_offset, end, self.start_line, self.start_col),
        }
    }

    // ------------------------------------------------------------------
    // Trivia skipping
    // ------------------------------------------------------------------

    /// Skip whitespace (spaces, tabs, `\r`, `\n`) and `//`-style line comments.
    fn skip_trivia(&mut self) {
        loop {
            let (_, ch) = match self.peek_char() {
                Some(val) => val,
                None => break,
            };

            match ch {
                ' ' | '\t' | '\r' | '\n' => {
                    self.advance();
                }
                '/' => {
                    // Need two-char lookahead for `//`. Clone the iterator to
                    // peek at the character after `/` without consuming it.
                    let mut clone = self.chars.clone();
                    clone.next(); // skip the `/`
                    if clone.next().map(|(_, c)| c) == Some('/') {
                        // Line comment — consume until newline or EOF.
                        self.advance(); // consume `/`
                        self.advance(); // consume second `/`
                        loop {
                            match self.peek_char() {
                                Some((_, '\n')) | None => break,
                                _ => {
                                    self.advance();
                                }
                            }
                        }
                    } else {
                        // Not a comment — leave the `/` for the operator handler.
                        break;
                    }
                }
                _ => break,
            }
        }
    }

    // ------------------------------------------------------------------
    // Token-level lexing helpers
    // ------------------------------------------------------------------

    /// Lex an identifier or keyword: `[a-zA-Z_][a-zA-Z0-9_]*`.
    fn lex_identifier(&mut self, first: char) -> Token {
        let mut lexeme = String::new();
        lexeme.push(first);

        while let Some((_, ch)) = self.peek_char() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                lexeme.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        let end = self.start_offset + lexeme.len();
        let kind = keyword_or_bool(lexeme.as_str());

        Token {
            kind,
            span: self.current_span(end),
            lexeme,
        }
    }

    /// Lex a numeric literal: `[0-9][0-9_]*` or `[0-9][0-9_]*\.[0-9][0-9_]*`.
    /// Underscores are allowed as digit separators within the number.
    fn lex_number(&mut self, first: char) -> Result<Token, LexError> {
        let mut lexeme = String::new();
        lexeme.push(first);

        // Consume integer part: digits and underscores.
        while let Some((_, ch)) = self.peek_char() {
            if ch.is_ascii_digit() || ch == '_' {
                lexeme.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Check for fractional part: `.` followed by a digit.
        let mut is_float = false;
        if self.peek_char().map(|(_, c)| c) == Some('.') {
            // Check the character after the dot.
            let mut clone = self.chars.clone();
            clone.next(); // skip `.`
            if clone.next().map(|(_, c)| c.is_ascii_digit()) == Some(true) {
                // It is a float literal — consume the dot and fractional part.
                is_float = true;
                lexeme.push('.');
                self.advance(); // consume `.`

                while let Some((_, ch)) = self.peek_char() {
                    if ch.is_ascii_digit() || ch == '_' {
                        lexeme.push(ch);
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
        }

        let end = self.start_offset + lexeme.len();
        let kind = if is_float {
            TokenKind::FloatLiteral
        } else {
            TokenKind::IntLiteral
        };

        Ok(Token {
            kind,
            span: self.current_span(end),
            lexeme,
        })
    }

    /// Lex a string literal: `"..."` with `\"` escape support.
    fn lex_string(&mut self) -> Result<Token, LexError> {
        // The opening `"` has already been consumed by `next()`.
        // start_offset points to the `"`, so we need to include it in the
        // lexeme.
        let mut lexeme = String::from("\"");

        loop {
            match self.advance() {
                None => {
                    // Unterminated string.
                    let end = self.start_offset + lexeme.len();
                    return Err(self.error("unterminated string literal", end));
                }
                Some((_, '"')) => {
                    lexeme.push('"');
                    break;
                }
                Some((_, '\\')) => {
                    // Escape sequence — look at the next character.
                    lexeme.push('\\');
                    match self.advance() {
                        None => {
                            let end = self.start_offset + lexeme.len();
                            return Err(self.error("unterminated string literal", end));
                        }
                        Some((_, ch)) => {
                            lexeme.push(ch);
                        }
                    }
                }
                Some((_, ch)) => {
                    lexeme.push(ch);
                }
            }
        }

        let end = self.start_offset + lexeme.len();
        Ok(Token {
            kind: TokenKind::StringLiteral,
            span: self.current_span(end),
            lexeme,
        })
    }

    /// Lex an operator or delimiter character.
    ///
    /// The first character has already been consumed. Returns `None` if the
    /// character is not a recognized operator/delimiter start.
    fn lex_operator_or_delimiter(&mut self, ch: char) -> Option<Token> {
        let kind = match ch {
            // Single-character delimiters.
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            ',' => TokenKind::Comma,
            ';' => TokenKind::Semicolon,
            ':' => TokenKind::Colon,
            '.' => TokenKind::Dot,
            // Single-character operators.
            '+' => TokenKind::Plus,
            '*' => TokenKind::Star,
            '%' => TokenKind::Percent,
            '?' => TokenKind::Question,
            // Standalone `/` — `//` is already handled by skip_trivia.
            '/' => TokenKind::Slash,

            // Compound: `->` arrow or `-` minus.
            '-' => {
                if self.peek_char().map(|(_, c)| c) == Some('>') {
                    self.advance();
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }
            // Compound: `==`, `=>`, or `=` assignment.
            '=' => match self.peek_char().map(|(_, c)| c) {
                Some('=') => {
                    self.advance();
                    TokenKind::EqEq
                }
                Some('>') => {
                    self.advance();
                    TokenKind::FatArrow
                }
                _ => TokenKind::Eq,
            },
            // Compound: `<=` or `<` angle bracket.
            '<' => {
                if self.peek_char().map(|(_, c)| c) == Some('=') {
                    self.advance();
                    TokenKind::LtEq
                } else {
                    TokenKind::LAngle
                }
            }
            // Compound: `>=` or `>` angle bracket.
            '>' => {
                if self.peek_char().map(|(_, c)| c) == Some('=') {
                    self.advance();
                    TokenKind::GtEq
                } else {
                    TokenKind::RAngle
                }
            }
            // Compound: `!=` or `!` not.
            '!' => {
                if self.peek_char().map(|(_, c)| c) == Some('=') {
                    self.advance();
                    TokenKind::NotEq
                } else {
                    TokenKind::Not
                }
            }
            // Compound: `&&` or `&` ampersand.
            '&' => {
                if self.peek_char().map(|(_, c)| c) == Some('&') {
                    self.advance();
                    TokenKind::And
                } else {
                    TokenKind::Ampersand
                }
            }
            // Compound: `||` or `|` pipe.
            '|' => {
                if self.peek_char().map(|(_, c)| c) == Some('|') {
                    self.advance();
                    TokenKind::Or
                } else {
                    TokenKind::Pipe
                }
            }

            // Not a recognized operator or delimiter start character.
            _ => return None,
        };

        let lexeme = token_kind_lexeme(kind, ch);
        let end = self.start_offset + lexeme.len();

        Some(Token {
            kind,
            span: self.current_span(end),
            lexeme,
        })
    }
}

// ------------------------------------------------------------------
// Iterator implementation
// ------------------------------------------------------------------

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        // 1. Skip whitespace and comments.
        self.skip_trivia();

        // 2. Record the start of the next token.
        match self.peek_char() {
            Some((offset, _)) => {
                self.start_offset = offset;
            }
            None => {
                self.start_offset = self.source.len();
            }
        }
        self.start_line = self.line;
        self.start_col = self.col;

        // 3. Consume the first character and dispatch.
        let (_, ch) = match self.advance() {
            Some(val) => val,
            None => {
                // End of input — emit EOF once.
                if self.eof_emitted {
                    return None;
                }
                self.eof_emitted = true;
                let span = Span::new(self.start_offset, self.start_offset, self.line, self.col);
                return Some(Ok(Token {
                    kind: TokenKind::Eof,
                    span,
                    lexeme: String::new(),
                }));
            }
        };

        // 4. Dispatch based on the first character.
        if ch.is_ascii_alphabetic() || ch == '_' {
            Some(Ok(self.lex_identifier(ch)))
        } else if ch.is_ascii_digit() {
            Some(self.lex_number(ch))
        } else if ch == '"' {
            Some(self.lex_string())
        } else if is_operator_start(ch) {
            match self.lex_operator_or_delimiter(ch) {
                Some(token) => Some(Ok(token)),
                None => {
                    let end = self.start_offset + 1;
                    Some(Err(self.error(
                        format!("unexpected character: '{ch}'"),
                        end,
                    )))
                }
            }
        } else {
            let end = self.start_offset + 1;
            Some(Err(self.error(
                format!("unexpected character: '{ch}'"),
                end,
            )))
        }
    }
}

// ------------------------------------------------------------------
// Helpers (free functions)
// ------------------------------------------------------------------

/// Returns `true` if `ch` can start an operator or delimiter token.
///
/// Note: `/` returns `true` here, but `//` comments are already consumed
/// by `skip_trivia` before this is called. `/` appearing here means a
/// standalone division operator.
fn is_operator_start(ch: char) -> bool {
    matches!(
        ch,
        '(' | ')'
            | '{'
            | '}'
            | '['
            | ']'
            | '<'
            | '>'
            | ','
            | ';'
            | ':'
            | '.'
            | '-'
            | '+'
            | '*'
            | '/'
            | '%'
            | '='
            | '!'
            | '&'
            | '|'
            | '?'
    )
}

/// Build the lexeme string for a token kind, given the fallback single
/// character for single-character operators and delimiters.
fn token_kind_lexeme(kind: TokenKind, fallback: char) -> String {
    match kind {
        TokenKind::Arrow => "->".into(),
        TokenKind::FatArrow => "=>".into(),
        TokenKind::EqEq => "==".into(),
        TokenKind::NotEq => "!=".into(),
        TokenKind::LtEq => "<=".into(),
        TokenKind::GtEq => ">=".into(),
        TokenKind::And => "&&".into(),
        TokenKind::Or => "||".into(),
        _ => fallback.to_string(),
    }
}

/// Map an identifier string to the appropriate keyword or literal token kind.
fn keyword_or_bool(lexeme: &str) -> TokenKind {
    match lexeme {
        "task" => TokenKind::Task,
        "workflow" => TokenKind::Workflow,
        "fn" => TokenKind::Fn,
        "let" => TokenKind::Let,
        "if" => TokenKind::If,
        "else" => TokenKind::Else,
        "while" => TokenKind::While,
        "for" => TokenKind::For,
        "return" => TokenKind::Return,
        "match" => TokenKind::Match,
        "await" => TokenKind::Await,
        "async" => TokenKind::Async,
        "pub" => TokenKind::Pub,
        "extern" => TokenKind::Extern,
        "export" => TokenKind::Export,
        "import" => TokenKind::Import,
        "enum" => TokenKind::Enum,
        "struct" => TokenKind::Struct,
        "type" => TokenKind::Type,
        "parallel" => TokenKind::Parallel,
        "true" | "false" => TokenKind::BoolLiteral,
        _ => TokenKind::Identifier,
    }
}