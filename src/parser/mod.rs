//! Parser for Jacquard source code.
//!
//! Consumes tokens from the lexer and produces an AST directly (no separate CST
//! layer).  The parser uses operator-precedence (Pratt) parsing for expressions
//! and recursive-descent for declarations and statements.
//!
//! ## Architecture
//! - `error` — `ParseError` type (message + span + expected/found token info).
//! - `expressions` — `ParserState` token cursor, Pratt expression parser, and
//!   all declaration/statement/type parsing.
//!
//! The public entry point is [`parse`], which takes a token slice and returns a
//! [`Program`](crate::ast::Program) AST.

pub mod error;
pub mod expressions;

pub use error::ParseError;

use crate::ast::Program;
use crate::lexer::Token;
use expressions::ParserState;

/// Parse a token stream into a `Program` AST.
///
/// This is the main entry point for the parser phase of the compiler pipeline.
pub fn parse(tokens: &[Token]) -> Result<Program, ParseError> {
    ParserState::new(tokens).parse_program()
}