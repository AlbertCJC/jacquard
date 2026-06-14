//! Lexer (tokenizer) for Jacquard source code.
//!
//! Converts a raw source string into a stream of tokens with span information.
//! The lexer is a pure iterator — it yields tokens on demand without buffering
//! the entire token stream into memory.
//!
//! ## Token types
//! Approximately 45 token types covering:
//! - Keywords: `task`, `workflow`, `fn`, `let`, `if`, `else`, `return`, etc.
//! - Literals: integers, floats, strings, booleans
//! - Operators and punctuation
//! - Identifiers
//! - Comments (stripped, not emitted as tokens)