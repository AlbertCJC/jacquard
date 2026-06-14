//! Type system for Jacquard.
//!
//! ## Type inference
//! Uses Hindley-Milner type inference with let-polymorphism. The type checker
//! unifies type variables, infers generic parameters, and reports type errors
//! with source-span locations.
//!
//! ## Type representation
//! Types are represented as an enum mirroring the language's type constructs:
//! primitives (`int`, `float`, `bool`, `string`), compound types (task handles,
//! futures, arrays), and user-defined types (structs, enums).
//!
//! ## Integration
//! The type checker consumes the lowered AST and either:
//! - Produces a fully-typed AST (with type annotations on every node), or
//! - Returns a `CompileError::Type` with a descriptive message.