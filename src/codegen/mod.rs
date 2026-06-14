//! C++ code generation from the typed Jacquard AST.
//!
//! Transforms the fully-typed AST into C++ source code, producing both a
//! header file (`.jq.h`) and an implementation file (`.jq.cpp`).
//!
//! ## Output structure
//! - **Header**: type definitions, forward declarations, inline functions
//! - **Source**: task/workflow implementations, runtime glue code
//!
//! ## Runtime dependency
//! Generated code links against the embedded Jacquard runtime library,
//! which provides the cooperative task scheduler and async primitives.
//!
//! ## Target
//! C++17 or later, with no platform-specific assumptions beyond the runtime.