# Jacquard Compiler — Full Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a working Jacquard compiler that lexes, parses, type-checks, and transpiles `.jac` files to C++ with an embedded cooperative runtime.

**Architecture:** Pipeline compiler in Rust: Source → Lexer (iterator) → Parser (hand-written recursive descent + Pratt expressions) → AST → Type Checker (bidirectional inference) → C++ Codegen (switch-case state machines for async). Output is `.jq.h`/`.jq.cpp` pairs plus a `jacquard_runtime.h` header.

**Tech Stack:** Rust (compiler), insta (snapshot testing), C++11+ (target output)

**Spec:** `docs/superpowers/specs/2026-06-14-compiler-design.md`

---

## File Map

| File | Responsibility |
|---|---|
| `Cargo.toml` | Project manifest, dependencies |
| `src/main.rs` | CLI entry point |
| `src/lib.rs` | Library root, pipeline orchestration |
| `src/lexer/mod.rs` | Lexer: iterator over source, tokenize() fn |
| `src/lexer/token.rs` | Token, TokenKind (~45 variants), Span, TokenCategory |
| `src/parser/mod.rs` | Parser module root, parse() entry point |
| `src/parser/error.rs` | ParseError type |
| `src/parser/expressions.rs` | Pratt parser: ParserState, Precedence, prefix/infix/postfix |
| `src/ast/mod.rs` | AST module root |
| `src/ast/nodes.rs` | All AST node types: Program, Declaration, Statement, Expr, Type |
| `src/ast/visit.rs` | Visitor trait for AST traversal |
| `src/types/mod.rs` | Type system module root, check() entry point |
| `src/types/ir.rs` | Type enum, TypeVarTable, TypeVarState (union-find unification) |
| `src/types/unify.rs` | unify() function — occurs check, MGU via union-find |
| `src/types/infer.rs` | Bidirectional inference: infer_expr(), check_expr(), solve() |
| `src/codegen/mod.rs` | Codegen module root, generate() entry point |
| `src/codegen/cpp.rs` | C++ code generation from typed AST |
| `src/codegen/state_machine.rs` | Async task → switch-case state machine lowering |
| `src/codegen/mangling.rs` | Name mangling: `_jq_{module}__{name}_{args}_{ret}` |
| `runtime/jacquard_runtime.h` | C++ runtime header: Task, Runtime, Poll, spawn, tick |
| `tests/lexer_test.rs` | Lexer token output, span tracking |
| `tests/parser_test.rs` | Parser: declarations, statements, expressions |
| `tests/ast_test.rs` | CST→AST lowering |
| `tests/type_test.rs` | Type inference, unification |
| `tests/codegen_test.rs` | C++ output snapshots |

**Commands:**
- `cargo build` — compile
- `cargo test` — all tests
- `cargo test --test lexer_test` — lexer only
- `cargo test --test parser_test` — parser only
- `cargo test --test type_test` — types only
- `cargo test --test codegen_test` — codegen only
- `cargo run -- compile input.jac` — compile a file

---

## Phase 1: Scaffold

### Task 1: Initialize Rust project with module structure

**Files:**
- Create: `Cargo.toml`, `src/main.rs`, `src/lib.rs`
- Create: `src/lexer/mod.rs`, `src/parser/mod.rs`, `src/ast/mod.rs`, `src/types/mod.rs`, `src/codegen/mod.rs`
- Create: `tests/` directory

- [ ] **Step 1: Run `cargo init`**

```bash
cargo init --lib --name jacquard "E:/Saved Projects/Programming language"
```

- [ ] **Step 2: Add dependencies to Cargo.toml**

```toml
[package]
name = "jacquard"
version = "0.1.0"
edition = "2021"

[dev-dependencies]
insta = { version = "1", features = ["yaml"] }
```

- [ ] **Step 3: Write src/main.rs** — CLI that reads a `.jac` file, calls `jacquard::compile()`, writes `.jq.h`/`.jq.cpp` output files. Uses `std::env::args` to parse subcommands (`compile <file>`).

- [ ] **Step 4: Write src/lib.rs** — Declare modules (`lexer`, `parser`, `ast`, `types`, `codegen`). Define `compile(source: &str, module_name: &str) -> Result<CompileOutput, CompileError>` that runs the full pipeline. Define `CompileOutput { header, source }` and `CompileError` enum with variants for each phase.

- [ ] **Step 5: Create placeholder `mod.rs` files** for each module (just doc comments).

- [ ] **Step 6: Build and verify**

```bash
cargo build
```
Expected: Compiles cleanly.

- [ ] **Step 7: Commit**

```bash
git add -A && git commit -m "feat: scaffold Jacquard compiler project"
```

---

## Phase 2: Lexer

### Task 2: Define token types and Span

**Files:**
- Create: `src/lexer/token.rs`
- Modify: `src/lexer/mod.rs`

- [ ] **Step 1: Write `token.rs`** — Define:
  - `Token { kind: TokenKind, span: Span, lexeme: String }`
  - `TokenKind` enum with all ~45 variants (keywords, literals, delimiters, operators, special)
  - `TokenCategory` enum with `category()` method on TokenKind
  - `Span { start: usize, end: usize, line: usize, col: usize }` with `new()` and `len()`
  - All types derive `Debug, Clone, PartialEq`

- [ ] **Step 2: Write `mod.rs` skeleton** — Define `Lexer` struct holding source `&str`, peekable char iterator, line/col tracking, start_offset. Define `LexError { message, span }`. Export `tokenize(source) -> Lexer` function. Add private fields and `new()`.

- [ ] **Step 3: Implement `Iterator` for `Lexer`** — Item = `Result<Token, LexError>`. Each call to `next()`: skip trivia (whitespace, `//` comments), peek next char, dispatch to lex_identifier, lex_number, lex_string, or lex_operator_or_delimiter. Return `TokenKind::Eof` at end, then `None`.

- [ ] **Step 4: Implement keyword/identifier lexing** — Build keyword map (`"task" => Task`, `"fn" => Fn`, etc.), match for `true`/`false` → `BoolLiteral`. Identifiers: `[a-zA-Z_][a-zA-Z0-9_]*`.

- [ ] **Step 5: Implement number lexing** — Integers: `[0-9][0-9_]*`. Floats: `[0-9][0-9_]*\.[0-9][0-9_]*`. Skip `_` digit separators.

- [ ] **Step 6: Implement string lexing** — `"..."` with `\"` escape handling. Unterminated string → `LexError`.

- [ ] **Step 7: Implement operator/delimiter lexing** — Single-char: `( ) { } [ ] , ; : . + - * / %`. Multi-char: `-> => == != <= >= && ||`. Use `peek_char()` for lookahead.

- [ ] **Step 8: Build**

```bash
cargo build
```
Expected: Compiles.

### Task 3: Write lexer tests

**Files:**
- Create: `tests/lexer_test.rs`

- [ ] **Step 1: Write test functions** covering:
  - All 19 keywords tokenize correctly
  - Identifiers: `x`, `my_var`, `snake_case`, `CamelCase`, `_private`
  - Integer literals: `0`, `42`, `1000000`, `1_000_000`
  - Float literals: `3.14`, `0.5`, `1.0`
  - String literals (including escaped quote)
  - Bool literals: `true`, `false`
  - All operators and delimiters in order
  - Span tracking: verify line/col numbers for multi-line input
  - Line comments: `// comment\nfn` → only `fn` token
  - EOF token always present
  - Empty input → just EOF

- [ ] **Step 2: Run tests**

```bash
cargo test --test lexer_test
```
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: implement lexer with full token set and tests"
```

---

## Phase 3: AST Definitions

### Task 4: Define AST node types

**Files:**
- Create: `src/ast/nodes.rs`
- Modify: `src/ast/mod.rs`

- [ ] **Step 1: Write `nodes.rs`** — Define all AST types per the spec. Key types:
  - `Program { declarations: Vec<Declaration>, span }`
  - `Declaration` enum: `Fn(FnDecl)`, `Task(TaskDecl)`, `Workflow(WorkflowDecl)`, `Struct(StructDecl)`, `Enum(EnumDecl)`, `Import(ImportDecl)`, `ExternFn(ExternFnDecl)`, `ExportFn(ExportFnDecl)`
  - `FnDecl { name, type_params, params, return_type, body, is_pub, span }`
  - `TaskDecl { name, params, return_type, body, span }`
  - `WorkflowDecl { name, body, span }` — always returns void
  - `StructDecl/EnumDecl` with fields/variants, type params, is_pub
  - `Type` enum: `Named(String)`, `Generic { name, args }`, `Function { params, ret }`, `Tuple(Vec<Type>)`
  - `Block { statements: Vec<Statement>, span }`
  - `Statement` enum: `Let`, `Expr`, `Return`, `If`, `While`, `For`, `ExprStmt`
  - `Expr { kind: ExprKind, span }` — all expression variants
  - `ExprKind` enum: literals, variable, binary/unary ops, call, field access, await, match, paren, array/map literals
  - `BinaryOp` enum (Add..Or), `UnaryOp` (Neg, Not)
  - `MatchArm`, `MatchPattern`, `MatchLiteral`
  - All types derive `Debug, Clone`

### Task 5: Define Visitor trait

**Files:**
- Create: `src/ast/visit.rs`
- Modify: `src/ast/mod.rs`

- [ ] **Step 1: Write `visit.rs`** — Define `Visitor` trait with default-noop methods for every AST node type. Each method walks into children by default. Methods: `visit_program`, `visit_declaration`, `visit_fn_decl`, `visit_block`, `visit_statement`, `visit_expr`, etc. The expression visitor dispatches on `ExprKind` and recurses into sub-expressions.

- [ ] **Step 2: Build and commit**

```bash
cargo build && git add -A && git commit -m "feat: define AST node types and Visitor trait"
```

---

## Phase 4: Parser

### Task 6: Define ParseError and ParserState skeleton

**Files:**
- Create: `src/parser/error.rs`, `src/parser/mod.rs`

- [ ] **Step 1: Write `error.rs`** — `ParseError { message, span, expected: Vec<TokenKind>, found: Option<TokenKind> }` with `new()`, `expected()` constructors, and `Display` impl.

- [ ] **Step 2: Write `mod.rs`** — Define `parse(tokens: &[Token]) -> Result<Program, ParseError>` entry point. Import expression parser.

### Task 7: Implement Pratt expression parser

**Files:**
- Create: `src/parser/expressions.rs`

- [ ] **Step 1: Define `Precedence` enum** — 9 levels from Lowest(0) to Call(8). Methods: `next()`, `from_binary_op(TokenKind) -> Option<Self>`, `binary_op(TokenKind) -> Option<BinaryOp>`.

- [ ] **Step 2: Define `ParserState` struct** — Wraps `&[Token]` with `position: usize`. Methods: `peek()`, `peek_kind()`, `advance()`, `expect(kind)`, `current_span()`.

- [ ] **Step 3: Implement `parse_expr(precedence)`** — Pratt algorithm:
  1. Call `parse_prefix()` for the atom
  2. Loop: peek next token, if binary op with precedence >= current, advance and parse right at `next()` precedence
  3. Loop: peek for postfix (`(args)` for call, `.field` for access)
  Return the built expression.

- [ ] **Step 4: Implement `parse_prefix()`** — Handle: int/float/string/bool literals, identifiers, `-expr`/`!expr` (unary), `(expr)`, `await expr`, `match expr { arms }`, `[array]`.

- [ ] **Step 5: Implement `parse_match_expr()` and `parse_match_pattern()`** — Match: `match expr { Pattern => body, ... }`. Patterns: constructor `Name(val)`, constructor `Name`, literal, wildcard.

- [ ] **Step 6: Implement `parse_array_literal()`** — `[elem, elem, ...]`

- [ ] **Step 7: Build**

```bash
cargo build
```

### Task 8: Implement declaration and statement parser

**Files:**
- Create: `src/parser/grammar.rs` (or add to `src/parser/mod.rs`)
- Modify: `src/parser/mod.rs`

- [ ] **Step 1: Implement `parse_program()`** — Loop until EOF, call `parse_declaration()` for each. Collect into `Program`.

- [ ] **Step 2: Implement `parse_declaration()`** — Dispatch on first token:
  - `pub` → parse next: `fn` → FnDecl, `struct` → StructDecl, `enum` → EnumDecl
  - `fn` → private FnDecl
  - `task` → TaskDecl
  - `workflow` → WorkflowDecl
  - `struct` → private StructDecl
  - `enum` → private EnumDecl
  - `import` → ImportDecl
  - `extern` → ExternFnDecl
  - `export` → ExportFnDecl

- [ ] **Step 3: Implement `parse_fn_decl(is_pub)`** — Parse: optional `<T, U>` type params, `(` params `)`, `->` return type, `{` body `}`. Params: `name: Type` separated by commas.

- [ ] **Step 4: Implement `parse_task_decl()`** and `parse_workflow_decl()`** — Same as fn but task has `task` keyword, workflow has no params/return (always void).

- [ ] **Step 5: Implement `parse_struct_decl(is_pub)`** — Parse: name, optional `<T>`, `{` fields `}`. Fields: `name: Type` separated by commas.

- [ ] **Step 6: Implement `parse_enum_decl(is_pub)`** — Parse: name, optional `<T>`, `{` variants `}`. Variants: `Name` or `Name(Type)` separated by commas.

- [ ] **Step 7: Implement `parse_block()` and `parse_statement()`** — Block: `{ statements }`. Statements: `let`, `return`, `if`, `while`, `for`, or expression statement. Dispatch based on first token kind.

- [ ] **Step 8: Implement `parse_if/while/for/let/return`** — Standard C-like syntax.

- [ ] **Step 9: Implement `parse_type()`** — Parse type expressions: `i32`, `Option<T>`, `(A, B) -> C`, `(A, B)`.

- [ ] **Step 10: Build**

```bash
cargo build
```

### Task 9: Write parser tests

**Files:**
- Create: `tests/parser_test.rs`

- [ ] **Step 1: Write test for parsing a simple function**

```rust
#[test]
fn test_parse_simple_fn() {
    let source = "fn add(x: i32, y: i32) -> i32 { return x + y; }";
    let tokens: Vec<_> = jacquard::lexer::tokenize(source)
        .map(|t| t.unwrap())
        .collect();
    let program = jacquard::parser::parse(&tokens).unwrap();
    assert_eq!(program.declarations.len(), 1);
    // Verify it's a function named "add" with 2 params
}
```

- [ ] **Step 2: Write tests** covering:
  - Function declaration (public + private, with/without generics)
  - Task declaration with await
  - Workflow declaration
  - Struct and enum declarations
  - Import declarations
  - Extern/export function declarations
  - Let statement with and without type annotation
  - If/else, while, for statements
  - Return statement with and without value
  - Binary expression precedence (e.g., `1 + 2 * 3` → `+(1, *(2, 3))`)
  - Unary expressions (`-x`, `!flag`)
  - Function calls with arguments
  - Field access (`obj.field`)
  - Match expressions
  - Array literals
  - Await expressions

- [ ] **Step 3: Run tests**

```bash
cargo test --test parser_test
```
Expected: All pass.

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat: implement parser with Pratt expressions and tests"
```

---

## Phase 5: Type System

### Task 10: Define Type IR and unification

**Files:**
- Create: `src/types/mod.rs`, `src/types/ir.rs`, `src/types/unify.rs`

- [ ] **Step 1: Write `ir.rs`** — Define `Type` enum per spec section 1.4:
  ```rust
  pub enum Type {
      // Primitives: I8..I64, U8..U64, F32, F64, Bool, String, Void
      // Compound: Function(Vec<Type>, Box<Type>), Tuple(Vec<Type>)
      // User-defined: Named(Symbol), Generic(Symbol, Vec<Type>)
      // Inference: Var(TypeVarId), Error
  }
  ```
  Define `TypeVarId(usize)`, `TypeVarTable { vars: Vec<TypeVarState> }` with `new_var() -> TypeVarId`, `resolve(id) -> TypeVarState`, `bind(id, Type)`, `union(id1, id2)` (union-find with path compression). `TypeVarState` enum: `Unbound(u32)`, `Bound(Type)`, `Link(TypeVarId)`.

- [ ] **Step 2: Write `unify.rs`** — `unify(a: &Type, b: &Type, table: &mut TypeVarTable) -> Result<(), TypeError>`:
  - If either is `Error`, succeed (poison pill)
  - Resolve type vars through links
  - If both are vars: union them
  - If one is var: bind it (with occurs check)
  - Primitives: must match exactly
  - Function types: unify param lists and return types pairwise
  - Named/Generic: must match by name, then unify args
  - Occurs check: reject `T = Function([T], T)` (infinite type)

- [ ] **Step 3: Define `TypeError`** — `{ message, span, expected: Option<Type>, found: Option<Type> }`.

### Task 11: Implement bidirectional type inference

**Files:**
- Create: `src/types/infer.rs`
- Modify: `src/types/mod.rs`

- [ ] **Step 1: Define `TypeEnv`** — Symbol table mapping variable names to `Type`. Methods: `insert(name, Type)`, `lookup(name) -> Option<Type>`, `lookup_type(name) -> Option<Type>` (for named types like structs/enums).

- [ ] **Step 2: Implement `infer_expr(expr, env, table) -> Result<Type, TypeError>`** — Bottom-up mode:
  - IntLiteral → I32 (default int type)
  - FloatLiteral → F64 (default float type)
  - StringLiteral → String
  - BoolLiteral → Bool
  - Variable → lookup in env
  - Binary: infer left and right, unify them, return result type
  - Unary: infer operand
  - Call: infer callee (must be function type), unify args with params, return ret type
  - FieldAccess, Await, Match, ArrayLiteral, Paren

- [ ] **Step 3: Implement `check_expr(expr, expected: &Type, env, table) -> Result<(), TypeError>`** — Top-down mode: infer the expression, then unify with expected type. Used for let-with-annotation, return statements, function call arguments where param types are known.

- [ ] **Step 4: Implement `check_fn_decl()`** — Create fresh type vars for generics, insert params into env (with their annotated types), check body against return type. For `task`, check that await expressions are inside task bodies.

- [ ] **Step 5: Implement `check_program()`** — Walk all declarations, type-check each in order. Build up the type environment with struct/enum names first, then check function bodies.

- [ ] **Step 6: Write `mod.rs` entry point** — `check(program: &Program) -> Result<TypedProgram, TypeError>`. Returns a type-annotated version or the original plus a type map.

### Task 12: Write type checker tests

**Files:**
- Create: `tests/type_test.rs`

- [ ] **Step 1: Write tests** covering:
  - Simple function type checking: `fn add(x: i32, y: i32) -> i32 { return x + y; }` ✓
  - Type mismatch error: `fn bad(x: i32) -> string { return x; }` ✗
  - Let inference: `let x = 42;` → x: i32
  - Binary expression types: `1 + 2` → i32, `1.0 + 2.0` → f64
  - Generics: `fn id<T>(x: T) -> T { return x; }`
  - Match expression exhaustiveness warning
  - Await inside task is OK, await inside fn is error
  - Struct field access type checking

- [ ] **Step 2: Run tests**

```bash
cargo test --test type_test
```

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: implement type system with bidirectional inference"
```

---

## Phase 6: Codegen

### Task 13: Implement name mangling

**Files:**
- Create: `src/codegen/mod.rs`, `src/codegen/mangling.rs`

- [ ] **Step 1: Write `mangling.rs`** — `mangle_fn(module: &str, name: &str, params: &[Type], ret: &Type) -> String`. Produces `_jq_{module}__{name}_{abbrev_params}_{abbrev_ret}`. Abbreviations: `i8`..`i64`, `u8`..`u64`, `f32`, `f64`, `bool`, `str`, `void`. Mangle `task` names to `Task_{name}` (PascalCase struct convention). Generics use `T` placeholder — C++ handles template instantiation.

### Task 14: Implement C++ codegen for non-async constructs

**Files:**
- Create: `src/codegen/cpp.rs`

- [ ] **Step 1: Define `CppWriter`** — Tracks output `String`, indentation level. Methods: `writeln(line)`, `indent()`, `dedent()`, `emit_header()`, `emit_source()`.

- [ ] **Step 2: Implement direct mappings** per spec section 2.1:
  - Primitives → C++ equivalents: `i32` → `int32_t`, `f64` → `double`, etc.
  - Functions → C++ functions: `fn add(x: i32, y: i32) -> i32` → `int32_t add(int32_t x, int32_t y)`
  - Structs → C++ structs
  - Enums → tagged unions: `enum Option<T>` → `template<typename T> struct Option { enum class Tag { Some, None }; Tag _tag; union { T some; }; }`
  - Match → switch on `_tag`
  - Generics → C++ templates
  - `if/else`, `while`, `for`, `return` → C++ equivalents

- [ ] **Step 3: Implement header/source separation** — `pub` declarations go in `.jq.h`, private in `.jq.cpp`. Module namespace: `namespace _jq_{module} { ... }`. Include guards in header.

- [ ] **Step 4: Implement `extern fn` / `export fn` generation** per spec section 3.4.

### Task 15: Implement async state machine codegen

**Files:**
- Create: `src/codegen/state_machine.rs`

- [ ] **Step 1: Define `StateMachine` struct** — `states: Vec<State>`, `fields: Vec<Field>`. A `State` has an ID, a list of statements (C++ code), a transition (next state or done). A `Field` is a local variable that becomes a struct member.

- [ ] **Step 2: Implement `lower_task(decl: &TaskDecl) -> StateMachine`** — Walk the task body. For each `await` expression, split into a new state. Local variables used across states become struct fields. Generate `_state` counter and `_result` field for error early-exit.

- [ ] **Step 3: Implement C++ codegen for StateMachine** — Generate struct with:
  - `int _state = 0;` field
  - One field per cross-state local variable
  - `_result` field (for `?` early exit)
  - `bool tick()` method: switch on `_state`, each case runs to next await or completion. Returns `false` if not done, `true` if complete.

- [ ] **Step 4: Implement `?` lowering in state machines** — After `await expr?`, generate:
  ```cpp
  auto _tmp = _tN.value();
  if (_tmp.is_err()) {
      _result = ...::Err(_tmp.unwrap_err());
      return true; // done with error
  }
  ```

### Task 16: Implement codegen entry point

**Files:**
- Modify: `src/codegen/mod.rs`

- [ ] **Step 1: Write `generate()` function** — `generate(program: &Program, module_name: &str) -> (String, String)`. Returns (header_content, source_content). Orchestrates: mangling setup → header preamble → declaration codegen (structs, enums, pub fns in header) → source codegen (private fns, task state machines) → includes `jacquard_runtime.h` in header.

### Task 17: Write codegen tests

**Files:**
- Create: `tests/codegen_test.rs`

- [ ] **Step 1: Write tests** covering:
  - Simple function codegen: verify C++ output matches expected pattern
  - Struct codegen: verify fields and `#include <cstdint>` present
  - Enum (tagged union) codegen: verify `enum class Tag`, `_tag`, `union`
  - Generic function: verify `template<typename T>` in output
  - Task state machine: verify `_state` counter, `tick()`, switch-case structure
  - Name mangling: verify `_jq_module__func_i32_void` format
  - `?` lowering: verify `_result` field and `is_err()` check in state machine
  - Header/source split: verify `pub` → header, private → source
  - Full pipeline: source → compile → verify header + source output

- [ ] **Step 2: Run tests**

```bash
cargo test --test codegen_test
```

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: implement C++ codegen with async state machine lowering"
```

---

## Phase 7: Runtime Header

### Task 18: Write C++ runtime header

**Files:**
- Create: `runtime/jacquard_runtime.h`

- [ ] **Step 1: Write `jacquard_runtime.h`** with:
  - Include guard, `<cstdint>`, `<vector>`, `<memory>`, `<string>`, `<functional>`
  - `namespace jq { ... }`
  - `template<typename T> struct Poll { bool ready; T value; };`
  - `struct Task { virtual bool tick(float dt) = 0; virtual ~Task() = default; };`
  - `class Runtime` with `spawn<T>(Args&&...)`, `tick(float delta_time)`, task status tracking, panic hook (`set_panic_hook(std::function<void(const char*, const char*, int)>)`).
  - `tick()` iterates tasks, calls `tick(dt)`, removes completed ones.
  - Default panic hook: prints to stderr.
  - String trait concept for configurable string type (`--string-type` flag).

- [ ] **Step 2: Verify the header compiles** — Write a minimal C++ test file:

```cpp
#include "../runtime/jacquard_runtime.h"
int main() { jq::Runtime rt; return 0; }
```

```bash
g++ -std=c++11 -c test_runtime.cpp -o /dev/null
```

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: add C++ runtime header (Task, Runtime, Poll, spawn)"
```

---

## Phase 8: Integration & Polish

### Task 19: Wire up the full pipeline

**Files:**
- Modify: `src/lib.rs` — ensure `compile()` calls all phases in order: lex → parse → lower → type check → codegen

- [ ] **Step 1: Implement CST→AST lowering** — If parser produces CST nodes directly, skip. Otherwise add `src/ast/lower.rs` that converts CST nodes to AST types. The parser in this plan produces AST directly, so this may be a no-op or a simple transformation step.

- [ ] **Step 2: Wire pipeline** — Uncomment the full `compile()` function in `lib.rs` with all phases connected. Handle errors at each phase boundary.

- [ ] **Step 3: Write an integration test** — Full pipeline: `task load() -> void { let x = await fetch(); }` → verify generated C++ contains `struct Task_load`, `_state`, `tick()`, `return false`.

```bash
cargo test --test codegen_test
```

### Task 20: Add snapshot testing with insta

**Files:**
- Modify: `tests/codegen_test.rs`

- [ ] **Step 1: Add insta snapshot test** — Compile a full Jacquard program, snapshot the C++ output.

```rust
#[test]
fn snapshot_full_program() {
    let source = r#"
        pub fn greet(name: string) -> string {
            return "hello " + name;
        }

        pub task countdown(n: i32) -> void {
            let i = n;
            while i > 0 {
                await tick();
                i = i - 1;
            }
        }
    "#;
    let output = jacquard::compile(source, "test").unwrap();
    insta::assert_snapshot!(output.header);
    insta::assert_snapshot!(output.source);
}
```

- [ ] **Step 2: Review and accept snapshots**

```bash
cargo test --test codegen_test
cargo insta review
```

- [ ] **Step 3: Run full test suite**

```bash
cargo test
```
Expected: All tests pass across all test files.

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat: wire up full pipeline with snapshot tests"
```

---

## Self-Review Checklist

Before considering the plan complete, verify:

1. **Spec coverage:** Each section of `docs/superpowers/specs/2026-06-14-compiler-design.md` is addressed by at least one task:
   - Section 1 (Type System) → Tasks 10-12
   - Section 2 (Codegen) → Tasks 13-17
   - Section 3 (Runtime) → Task 18
   - Section 4 (Module System) → Task 8 (import parsing), Task 14 (header/source separation)
   - Section 5 (Error Handling) → Tasks 3, 6, 10, 11, 15
   - Section 6 (Standard Library) → Partially in codegen; stdlib types are built-in ADTs
   - Section 7 (Primitive Types) → Task 10 (Type enum), Task 14 (C++ type mappings)
   - Section 8 (workflow) → Task 8 (parse_workflow_decl), Task 15 (same lowering as task)

2. **No placeholders** — Every task has specific steps with code structure descriptions. No TBDs or TODOs.

3. **Type consistency** — Types defined in Task 4 (AST nodes) are used consistently in Tasks 5-17. Token types from Task 2 used in Tasks 6-9.

---

## Execution Handoff

After saving and reviewing this plan, offer the user a choice between:

**1. Subagent-Driven (recommended)** — Dispatch a fresh subagent per task, review between tasks, fast iteration
**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints