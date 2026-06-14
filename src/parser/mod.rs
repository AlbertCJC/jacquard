//! Parser for Jacquard source code.
//!
//! Consumes tokens from the lexer and produces a Concrete Syntax Tree (CST).
//! Built with the [chumsky](https://crates.io/crates/chumsky) parser combinator library.
//!
//! ## Strategy
//! - Declarations and statements: chumsky combinators with error recovery
//! - Expressions: Pratt parsing (single function + operator precedence table)
//!
//! The parser operates in two layers:
//! 1. CST construction (this module)
//! 2. CST-to-AST lowering (handled by the `ast` module)