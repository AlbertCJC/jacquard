//! Abstract Syntax Tree definitions and lowering pass.
//!
//! ## Node hierarchy
//! - `Program` — top-level container for all declarations
//! - `Declaration` — `TaskDecl`, `WorkflowDecl`, `FnDecl`, `ImportDecl`
//! - `Statement` — `LetStmt`, `ExprStmt`, `ReturnStmt`, `IfStmt`, `BlockStmt`, etc.
//! - `Expr` — literals, identifiers, binary/unary ops, calls, member access, etc.
//! - `Type` — `Named`, `Fn`, `Generic`, etc. (see `types` module for type-level types)
//!
//! The AST uses `Box`-based recursive types for simplicity and debuggability.
//! Arena allocation may be introduced later as an optimization.
//!
//! ## Lowering pass
//! Transforms the Concrete Syntax Tree (from the parser) into this AST,
//! resolving desugared constructs and validating structural constraints.