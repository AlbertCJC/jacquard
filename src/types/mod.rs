//! Type system for Jacquard.
//!
//! ## Type inference
//! Uses bidirectional type inference — function signatures are annotated,
//! function bodies are inferred. Unification is local to each function,
//! so type errors localize to the function boundary.
//!
//! ## Type representation
//! Types are represented as an enum mirroring the language's type constructs:
//! primitives (`int`, `float`, `bool`, `string`), compound types (functions,
//! tuples), and user-defined types (named, generic).
//!
//! ## Integration
//! The type checker consumes the AST and either:
//! - Produces a fully-typed AST (with type annotations on every node), or
//! - Returns a `TypeError` with a descriptive message.

pub mod ir;
mod infer;
mod unify;

pub use ir::{Type, TypeVarState, TypeVarTable, TypeError};
pub use infer::{TypeEnv, infer_expr, check_expr, check_program};