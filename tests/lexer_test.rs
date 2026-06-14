//! Comprehensive tests for the Jacquard lexer.
//!
//! Covers all token kinds, span tracking, edge cases, and error conditions.

use jacquard::lexer::{tokenize, Token, TokenKind};

// ---------------------------------------------------------------------------
// Helper: collect all tokens from source, unwrapping results.
// ---------------------------------------------------------------------------

fn tokenize_all(source: &str) -> Vec<Token> {
    tokenize(source).map(|r| r.unwrap()).collect()
}

// Helper: get just the TokenKind slice from a source string.
fn token_kinds(source: &str) -> Vec<TokenKind> {
    tokenize_all(source).iter().map(|t| t.kind).collect()
}

// ===========================================================================
// 1. Keywords — all 19 keywords tokenize correctly
// ===========================================================================

#[test]
fn test_all_19_keywords() {
    // Each keyword in a separate invocation so we can assert kind + lexeme
    // precisely.
    let cases: Vec<(&str, TokenKind)> = vec![
        ("task", TokenKind::Task),
        ("workflow", TokenKind::Workflow),
        ("fn", TokenKind::Fn),
        ("let", TokenKind::Let),
        ("if", TokenKind::If),
        ("else", TokenKind::Else),
        ("while", TokenKind::While),
        ("for", TokenKind::For),
        ("return", TokenKind::Return),
        ("match", TokenKind::Match),
        ("await", TokenKind::Await),
        ("async", TokenKind::Async),
        ("pub", TokenKind::Pub),
        ("extern", TokenKind::Extern),
        ("export", TokenKind::Export),
        ("import", TokenKind::Import),
        ("enum", TokenKind::Enum),
        ("struct", TokenKind::Struct),
        ("type", TokenKind::Type),
        ("parallel", TokenKind::Parallel),
    ];

    for (source, expected_kind) in &cases {
        let tokens = tokenize_all(source);
        // Each input is a single keyword, so we expect [keyword, EOF].
        assert_eq!(
            tokens.len(),
            2,
            "expected 2 tokens (keyword + EOF) for '{source}'"
        );
        assert_eq!(
            tokens[0].kind, *expected_kind,
            "wrong TokenKind for '{source}'"
        );
        assert_eq!(
            tokens[0].lexeme, *source,
            "wrong lexeme for '{source}'"
        );
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }
}

// ===========================================================================
// 2. Identifiers
// ===========================================================================

#[test]
fn test_identifiers() {
    let source = "x my_var snake_case CamelCase _private";
    let tokens = tokenize_all(source);

    let expected_lexemes = vec!["x", "my_var", "snake_case", "CamelCase", "_private"];
    let non_eof: Vec<&Token> = tokens.iter().filter(|t| t.kind != TokenKind::Eof).collect();

    assert_eq!(non_eof.len(), expected_lexemes.len());

    for (token, expected_lexeme) in non_eof.iter().zip(expected_lexemes.iter()) {
        assert_eq!(
            token.kind,
            TokenKind::Identifier,
            "expected Identifier for '{expected_lexeme}'"
        );
        assert_eq!(token.lexeme, *expected_lexeme);
    }

    // Last token must be EOF.
    assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);
}

// ===========================================================================
// 3. Integer literals
// ===========================================================================

#[test]
fn test_integer_literals() {
    let source = "0 42 1000000 1_000_000";
    let tokens = tokenize_all(source);

    let expected_lexemes = vec!["0", "42", "1000000", "1_000_000"];
    let non_eof: Vec<&Token> = tokens.iter().filter(|t| t.kind != TokenKind::Eof).collect();

    assert_eq!(non_eof.len(), expected_lexemes.len());

    for (token, expected_lexeme) in non_eof.iter().zip(expected_lexemes.iter()) {
        assert_eq!(
            token.kind,
            TokenKind::IntLiteral,
            "expected IntLiteral for '{expected_lexeme}'"
        );
        assert_eq!(token.lexeme, *expected_lexeme);
    }

    assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);
}

// ===========================================================================
// 4. Float literals
// ===========================================================================

#[test]
fn test_float_literals() {
    let source = "3.14 0.5 1.0";
    let tokens = tokenize_all(source);

    let expected_lexemes = vec!["3.14", "0.5", "1.0"];
    let non_eof: Vec<&Token> = tokens.iter().filter(|t| t.kind != TokenKind::Eof).collect();

    assert_eq!(non_eof.len(), expected_lexemes.len());

    for (token, expected_lexeme) in non_eof.iter().zip(expected_lexemes.iter()) {
        assert_eq!(
            token.kind,
            TokenKind::FloatLiteral,
            "expected FloatLiteral for '{expected_lexeme}'"
        );
        assert_eq!(token.lexeme, *expected_lexeme);
    }

    assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);
}

// ---------------------------------------------------------------------------
// Edge case: a dot NOT followed by a digit stays an IntLiteral + Dot
// ---------------------------------------------------------------------------

#[test]
fn test_int_followed_by_dot_is_not_float() {
    let source = "42.method()";
    let kinds = token_kinds(source);
    assert_eq!(
        kinds,
        vec![
            TokenKind::IntLiteral,  // 42
            TokenKind::Dot,         // .
            TokenKind::Identifier,  // method
            TokenKind::LParen,      // (
            TokenKind::RParen,      // )
            TokenKind::Eof,
        ]
    );
}

// ===========================================================================
// 5. String literals (including escaped quote)
// ===========================================================================

#[test]
fn test_string_literals() {
    // Simple string
    {
        let tokens = tokenize_all("\"hello world\"");
        assert_eq!(tokens.len(), 2); // string + EOF
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[0].lexeme, "\"hello world\"");
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }

    // Empty string
    {
        let tokens = tokenize_all("\"\"");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[0].lexeme, "\"\"");
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }

    // Escaped quote
    {
        let tokens = tokenize_all("\"escaped\\\"quote\"");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[0].lexeme, "\"escaped\\\"quote\"");
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }

    // Escaped backslash
    {
        let tokens = tokenize_all("\"path\\\\to\\\\file\"");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[0].lexeme, "\"path\\\\to\\\\file\"");
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }
}

// ===========================================================================
// 6. Bool literals — true / false are BoolLiteral, not keywords
// ===========================================================================

#[test]
fn test_bool_literals() {
    // true
    {
        let tokens = tokenize_all("true");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::BoolLiteral);
        assert_eq!(tokens[0].lexeme, "true");
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }

    // false
    {
        let tokens = tokenize_all("false");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::BoolLiteral);
        assert_eq!(tokens[0].lexeme, "false");
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }

    // true and false are NOT identifiers
    {
        let tokens = tokenize_all("true false");
        let non_eof: Vec<&Token> = tokens.iter().filter(|t| t.kind != TokenKind::Eof).collect();
        for token in &non_eof {
            assert_eq!(token.kind, TokenKind::BoolLiteral);
            assert_ne!(token.kind, TokenKind::Identifier);
        }
    }
}

// ===========================================================================
// 7. All operators and delimiters
// ===========================================================================

#[test]
fn test_all_operators_and_delimiters() {
    // Feed a string containing every operator and delimiter from the task,
    // space-separated where necessary for single-char tokens.
    let source = "+ - * / % = == != < > <= >= && || ! -> => ( ) { } [ ] , ; : . ?";
    let tokens = tokenize_all(source);

    let expected: Vec<TokenKind> = vec![
        TokenKind::Plus,
        TokenKind::Minus,
        TokenKind::Star,
        TokenKind::Slash,
        TokenKind::Percent,
        TokenKind::Eq,
        TokenKind::EqEq,
        TokenKind::NotEq,
        TokenKind::LAngle,
        TokenKind::RAngle,
        TokenKind::LtEq,
        TokenKind::GtEq,
        TokenKind::And,
        TokenKind::Or,
        TokenKind::Not,
        TokenKind::Arrow,
        TokenKind::FatArrow,
        TokenKind::LParen,
        TokenKind::RParen,
        TokenKind::LBrace,
        TokenKind::RBrace,
        TokenKind::LBracket,
        TokenKind::RBracket,
        TokenKind::Comma,
        TokenKind::Semicolon,
        TokenKind::Colon,
        TokenKind::Dot,
        TokenKind::Question,
        TokenKind::Eof,
    ];

    let kinds: Vec<TokenKind> = tokens.iter().map(|t| t.kind).collect();
    assert_eq!(kinds, expected, "unexpected operator/delimiter tokenization");
}

// ---------------------------------------------------------------------------
// Verify lexemes for compound and single-character operators
// ---------------------------------------------------------------------------

#[test]
fn test_operator_delimiter_lexemes() {
    let source = "-> => == != <= >= && || + - * / % = < > ! ( ) { } [ ] , ; : . ?";
    let tokens = tokenize_all(source);

    let expected_lexemes: Vec<&str> = vec![
        "->", "=>", "==", "!=", "<=", ">=", "&&", "||", "+", "-", "*", "/", "%", "=", "<", ">",
        "!", "(", ")", "{", "}", "[", "]", ",", ";", ":", ".", "?",
    ];

    let non_eof: Vec<&Token> = tokens.iter().filter(|t| t.kind != TokenKind::Eof).collect();
    assert_eq!(non_eof.len(), expected_lexemes.len());

    for (token, expected_lexeme) in non_eof.iter().zip(expected_lexemes.iter()) {
        assert_eq!(
            token.lexeme, *expected_lexeme,
            "wrong lexeme for {:?}",
            token.kind
        );
    }
}

// ===========================================================================
// 8. Span tracking — verify line/col across multi-line input
// ===========================================================================

#[test]
fn test_span_tracking_multiline() {
    let source = "fn main() {\n    let x = 42;\n}";
    let tokens = tokenize_all(source);

    // Expected: kind, line, col, lexeme
    #[derive(Debug)]
    struct Expected {
        kind: TokenKind,
        line: usize,
        col: usize,
        lexeme: &'static str,
    }

    let expected = vec![
        Expected { kind: TokenKind::Fn,          line: 1, col: 1,  lexeme: "fn" },
        Expected { kind: TokenKind::Identifier,  line: 1, col: 4,  lexeme: "main" },
        Expected { kind: TokenKind::LParen,       line: 1, col: 8,  lexeme: "(" },
        Expected { kind: TokenKind::RParen,       line: 1, col: 9,  lexeme: ")" },
        Expected { kind: TokenKind::LBrace,       line: 1, col: 11, lexeme: "{" },
        Expected { kind: TokenKind::Let,          line: 2, col: 5,  lexeme: "let" },
        Expected { kind: TokenKind::Identifier,   line: 2, col: 9,  lexeme: "x" },
        Expected { kind: TokenKind::Eq,           line: 2, col: 11, lexeme: "=" },
        Expected { kind: TokenKind::IntLiteral,   line: 2, col: 13, lexeme: "42" },
        Expected { kind: TokenKind::Semicolon,    line: 2, col: 15, lexeme: ";" },
        Expected { kind: TokenKind::RBrace,       line: 3, col: 1,  lexeme: "}" },
        Expected { kind: TokenKind::Eof,          line: 3, col: 2,  lexeme: "" },
    ];

    assert_eq!(
        tokens.len(),
        expected.len(),
        "token count mismatch"
    );

    for (i, (token, exp)) in tokens.iter().zip(expected.iter()).enumerate() {
        assert_eq!(
            token.kind, exp.kind,
            "token {i}: expected {:?}, got {:?}",
            exp.kind, token.kind
        );
        assert_eq!(
            token.span.line, exp.line,
            "token {i} ({:?}): expected line {}, got {}",
            token.kind, exp.line, token.span.line
        );
        assert_eq!(
            token.span.col, exp.col,
            "token {i} ({:?}): expected col {}, got {}",
            token.kind, exp.col, token.span.col
        );
        assert_eq!(
            token.lexeme, exp.lexeme,
            "token {i} ({:?}): expected lexeme '{}', got '{}'",
            token.kind, exp.lexeme, token.lexeme
        );
    }
}

// ===========================================================================
// 9. Line comments — stripped, not emitted as tokens
// ===========================================================================

#[test]
fn test_line_comments_are_stripped() {
    // Comment followed by a keyword — only the keyword (and EOF) should appear.
    let source = "// this is a comment\nfn";
    let tokens = tokenize_all(source);

    assert_eq!(tokens.len(), 2, "expected only fn + EOF, got {tokens:?}");
    assert_eq!(tokens[0].kind, TokenKind::Fn);
    assert_eq!(tokens[0].lexeme, "fn");
    assert_eq!(tokens[1].kind, TokenKind::Eof);
}

#[test]
fn test_comment_at_end_of_file_produces_only_eof() {
    let source = "// just a comment, no newline";
    let tokens = tokenize_all(source);

    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, TokenKind::Eof);
}

#[test]
fn test_code_before_comment() {
    let source = "let x = 1; // inline comment\nlet y = 2;";
    let kinds = token_kinds(source);

    assert_eq!(
        kinds,
        vec![
            TokenKind::Let,
            TokenKind::Identifier,  // x
            TokenKind::Eq,
            TokenKind::IntLiteral,  // 1
            TokenKind::Semicolon,
            // comment should be absent
            TokenKind::Let,
            TokenKind::Identifier,  // y
            TokenKind::Eq,
            TokenKind::IntLiteral,  // 2
            TokenKind::Semicolon,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_slash_not_comment_operator() {
    // Standalone `/` with spaces around it should be a Slash token, not eaten
    // by comment handling.
    let source = "a / b";
    let kinds = token_kinds(source);

    assert_eq!(
        kinds,
        vec![
            TokenKind::Identifier,  // a
            TokenKind::Slash,       // /
            TokenKind::Identifier,  // b
            TokenKind::Eof,
        ]
    );
}

// ===========================================================================
// 10. EOF token always present as the last token
// ===========================================================================

#[test]
fn test_eof_is_last_token() {
    // Single token input
    {
        let tokens = tokenize_all("42");
        assert!(tokens.len() >= 2);
        assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);
    }

    // Multi-token input
    {
        let tokens = tokenize_all("fn main() { return 0; }");
        assert!(tokens.len() >= 2);
        assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);
    }

    // Only one EOF, even if next() is called repeatedly
    {
        let mut lexer = tokenize("x");
        // Collect all tokens including EOF.
        let all: Vec<_> = lexer.by_ref().map(|r| r.unwrap()).collect();
        let eof_count = all.iter().filter(|t| t.kind == TokenKind::Eof).count();
        assert_eq!(eof_count, 1, "there should be exactly one EOF token");

        // Subsequent calls return None.
        assert!(lexer.next().is_none());
    }
}

// ===========================================================================
// 11. Empty input — just one EOF token
// ===========================================================================

#[test]
fn test_empty_input_produces_only_eof() {
    let tokens = tokenize_all("");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, TokenKind::Eof);
    assert_eq!(tokens[0].lexeme, "");
    assert!(tokens[0].span.is_empty());
}

// ---------------------------------------------------------------------------
// Whitespace-only input
// ---------------------------------------------------------------------------

#[test]
fn test_whitespace_only_produces_only_eof() {
    let tokens = tokenize_all("   \n\n\t  \r\n  ");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, TokenKind::Eof);
}

// ===========================================================================
// Bonus: Ampersand and Pipe as standalone operators
// ===========================================================================

#[test]
fn test_standalone_ampersand_and_pipe() {
    // & alone is Ampersand, | alone is Pipe.
    let source = "& |";
    let kinds = token_kinds(source);

    assert_eq!(
        kinds,
        vec![TokenKind::Ampersand, TokenKind::Pipe, TokenKind::Eof]
    );
}

#[test]
fn test_ampersand_and_pipe_lexemes() {
    let tokens = tokenize_all("& |");
    assert_eq!(tokens[0].lexeme, "&");
    assert_eq!(tokens[1].lexeme, "|");
}

// ===========================================================================
// Bonus: Span start/end byte offsets
// ===========================================================================

#[test]
fn test_span_byte_offsets() {
    let source = "fn main";
    let tokens = tokenize_all(source);

    // fn: bytes 0..2
    assert_eq!(tokens[0].span.start, 0);
    assert_eq!(tokens[0].span.end, 2);
    assert_eq!(tokens[0].span.len(), 2);

    // main: bytes 3..7
    assert_eq!(tokens[1].span.start, 3);
    assert_eq!(tokens[1].span.end, 7);
    assert_eq!(tokens[1].span.len(), 4);

    // EOF
    assert_eq!(tokens[2].span.start, 7);
    assert_eq!(tokens[2].span.end, 7);
    assert!(tokens[2].span.is_empty());
}

// ===========================================================================
// Bonus: Unterminated string error
// ===========================================================================

#[test]
fn test_unterminated_string_is_error() {
    let results: Vec<_> = tokenize("\"no closing quote").collect();
    // After the error, the lexer still produces an EOF token.
    assert_eq!(results.len(), 2);

    match &results[0] {
        Err(err) => {
            assert!(
                err.message.contains("unterminated"),
                "expected unterminated string error, got: {}",
                err.message
            );
        }
        Ok(tok) => panic!("expected error, got token: {tok:?}"),
    }

    // Second result is EOF.
    assert!(results[1].is_ok());
    assert_eq!(results[1].as_ref().unwrap().kind, TokenKind::Eof);
}

// ===========================================================================
// Bonus: Unexpected character error
// ===========================================================================

#[test]
fn test_unexpected_character_is_error() {
    // '#' is not a valid Jacquard character.
    let results: Vec<_> = tokenize("#").collect();
    // After the error, the lexer still produces an EOF token.
    assert_eq!(results.len(), 2);

    match &results[0] {
        Err(err) => {
            assert!(
                err.message.contains("unexpected character"),
                "expected unexpected character error, got: {}",
                err.message
            );
        }
        Ok(tok) => panic!("expected error, got token: {tok:?}"),
    }

    // Second result is EOF.
    assert!(results[1].is_ok());
    assert_eq!(results[1].as_ref().unwrap().kind, TokenKind::Eof);
}

// ===========================================================================
// Bonus: TokenCategory classification
// ===========================================================================

#[test]
fn test_token_category_classification() {
    use jacquard::lexer::TokenCategory;

    // Keywords
    assert_eq!(TokenKind::Fn.category(), TokenCategory::Keyword);
    assert_eq!(TokenKind::Let.category(), TokenCategory::Keyword);
    assert_eq!(TokenKind::Task.category(), TokenCategory::Keyword);

    // Literals
    assert_eq!(TokenKind::IntLiteral.category(), TokenCategory::Literal);
    assert_eq!(TokenKind::FloatLiteral.category(), TokenCategory::Literal);
    assert_eq!(TokenKind::StringLiteral.category(), TokenCategory::Literal);
    assert_eq!(TokenKind::BoolLiteral.category(), TokenCategory::Literal);

    // Identifier
    assert_eq!(TokenKind::Identifier.category(), TokenCategory::Identifier);

    // Delimiters
    assert_eq!(TokenKind::LParen.category(), TokenCategory::Delimiter);
    assert_eq!(TokenKind::RParen.category(), TokenCategory::Delimiter);
    assert_eq!(TokenKind::Arrow.category(), TokenCategory::Delimiter);

    // Operators
    assert_eq!(TokenKind::Plus.category(), TokenCategory::Operator);
    assert_eq!(TokenKind::EqEq.category(), TokenCategory::Operator);
    assert_eq!(TokenKind::And.category(), TokenCategory::Operator);

    // Special
    assert_eq!(TokenKind::Eof.category(), TokenCategory::Special);

    // Error
    assert_eq!(TokenKind::Error.category(), TokenCategory::Error);
}

// ===========================================================================
// Bonus: Complex line/column tracking across various constructs
// ===========================================================================

#[test]
fn test_line_col_after_string_with_newline() {
    // A string literal containing an actual newline escape is not supported by
    // the current lexer (only \"), but a token following a string on the same
    // line should carry correct column info.
    let source = "\"hello\" 42";
    let tokens = tokenize_all(source);

    // "hello" at col 1
    assert_eq!(tokens[0].span.line, 1);
    assert_eq!(tokens[0].span.col, 1);

    // 42 at col 9 (after "\"hello\"" which is 7 chars + 1 space = col 9)
    assert_eq!(tokens[1].kind, TokenKind::IntLiteral);
    assert_eq!(tokens[1].span.line, 1);
    assert_eq!(tokens[1].span.col, 9);

    assert_eq!(tokens[2].kind, TokenKind::Eof);
}