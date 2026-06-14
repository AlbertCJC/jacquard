//! Jacquard compiler — a task-orchestration DSL that transpiles to C++.
//!
//! Pipeline: Source -> Lexer -> Parser(CST) -> AST Lower -> Type Checker -> Codegen -> C++

pub mod lexer;
pub mod parser;
pub mod ast;
pub mod types;
pub mod codegen;

/// The output of a successful compilation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileOutput {
    /// Contents of the generated `.jq.h` header file.
    pub header: String,
    /// Contents of the generated `.jq.cpp` source file.
    pub source: String,
}

/// Errors that can occur during compilation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    /// Lexing / tokenization error.
    Lex(String),
    /// Parsing error (CST construction failed).
    Parse(String),
    /// CST-to-AST lowering error.
    Lower(String),
    /// Type-checking error.
    Type(String),
    /// Code-generation error.
    Codegen(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::Lex(msg) => write!(f, "Lex error: {msg}"),
            CompileError::Parse(msg) => write!(f, "Parse error: {msg}"),
            CompileError::Lower(msg) => write!(f, "Lower error: {msg}"),
            CompileError::Type(msg) => write!(f, "Type error: {msg}"),
            CompileError::Codegen(msg) => write!(f, "Codegen error: {msg}"),
        }
    }
}

impl std::error::Error for CompileError {}

/// Run the full Jacquard compilation pipeline on `source`.
///
/// `module_name` is the base name (without extension) used for output file naming.
///
/// Currently **stubbed** — each phase will be wired in as it is implemented.
pub fn compile(_source: &str, _module_name: &str) -> Result<CompileOutput, CompileError> {
    // Stub: returns empty output. Real implementation will be wired in Task 19.
    Ok(CompileOutput {
        header: String::new(),
        source: String::new(),
    })
}