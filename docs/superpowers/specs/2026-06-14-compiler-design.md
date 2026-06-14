# Jacquard Compiler Design — Phase 1.5/2

**Date:** 2026-06-14
**Status:** Approved
**Scope:** Type system, codegen, runtime embedding, module system, error handling

## Overview

This document specifies the design of the Jacquard compiler's type system, code generation strategy, runtime embedding contract, module system, and error handling model. These build on the Phase 1 frontend (lexer, parser, AST) and complete the compiler pipeline design.

---

## 1. Type System

### 1.1 Type Inference: Bidirectional

Jacquard uses **bidirectional type inference** — the standard approach in modern languages (Rust, Swift, TypeScript).

- **Function signatures are annotated** by the programmer (parameters and return type).
- **Function bodies are inferred** — types flow from expressions without annotation.
- **Unification is local to each function** — type errors localize to the function boundary, never across the program.

**Two inference modes per expression:**

| Mode | Direction | Example | When Used |
|---|---|---|---|
| **Checking** | Top-down | `let x: i32 = expr;` | Expected type known from context |
| **Inference** | Bottom-up | `let x = 42;` | No expected type, infer from expression |

When the two modes meet (e.g., `let x: i32 = some_expr()`), the type checker unifies the expected type with the inferred type. A mismatch produces a localized error at that expression.

**Why not global H-M:** Global inference produces error messages at distant unification sites ("type mismatch at line 147" for a bug at line 12). Game engine developers need precise, local error messages.

### 1.2 ADT Representation: Hand-Rolled Tagged Unions

Sum types (enums with payloads) compile to hand-rolled tagged unions in C++.

```
Jacquard:                              C++ output:
enum Option<T> {                       template<typename T>
    Some(T),                           struct Option {
    None                                   enum class Tag { Some, None };
}                                          Tag _tag;
                                           union {
fn unwrap_or<T>(opt: Option<T>,                T some;
    default: T) -> T {                     };
    match opt {                         };
        Some(val) => val,
        None => default                 template<typename T>
    }                                   T unwrap_or(Option<T> opt, T def) {
}                                          switch (opt._tag) {
                                              case Option<T>::Tag::Some:
                                                  return opt._payload.some;
                                              case Option<T>::Tag::None:
                                                  return def;
                                          }
                                      }
```

- **Match compiles to `switch`** on the tag field → jump table dispatch (fastest path).
- **No `std::variant`** — zero-dependency output, works with any C++11+ compiler.
- **If recursive/nested ADTs later require `std::variant`**, a future optimization pass can introduce it selectively. Starting with tagged unions keeps the output simple, portable, and predictable.

### 1.3 Generics

**Syntax:** Angle brackets (Rust/Swift style).

```
fn identity<T>(x: T) -> T { return x; }
struct Pair<A, B> { first: A, second: B }
enum Option<T> { Some(T), None }
```

**Implementation:** Monomorphized via C++ templates. Each concrete instantiation generates a separate template instantiation in the C++ output. Zero runtime overhead.

**Constraints:** No trait bounds in v1. Type errors surface at C++ template instantiation time. Trait/typeclass bounds deferred to post-prototype.

**Name mangling for generics:**
```
fn identity<T>(x: T) -> T
→ template<typename T> T _jq_module__identity_T_T(T x) { ... }
```

### 1.4 Type IR (Compiler Internal)

The compiler represents types internally as a Rust enum:

```rust
enum Type {
    // Primitives
    I8, I16, I32, I64,
    U8, U16, U32, U64,
    F32, F64,
    Bool,
    String,
    Void,

    // Compound
    Function(Vec<Type>, Box<Type>),  // (params...) -> return
    Tuple(Vec<Type>),

    // User-defined
    Named(Symbol),                    // enum/struct name → resolved via symbol table
    Generic(Symbol, Vec<Type>),       // Option<int>, Result<string, Error>

    // Inference (bidirectional)
    Var(TypeVarId),                   // type variable, unified during inference
    Error,                            // poison pill — suppresses cascading errors
}
```

**Unification table:**
```rust
enum TypeVarState {
    Unbound(u32),         // fresh var #N, not yet known
    Bound(Type),          // resolved to a concrete type
    Link(TypeVarId),      // chain to another var (union-find)
}
```

- Union-find with path compression for O(α(n)) unification.
- Occurs check prevents infinite types (`T = T → T` is rejected).
- `Error` type acts as a poison pill: once a sub-expression has a type error, the parent expression skips further checking to avoid cascading false positives.

---

## 2. Codegen Strategy

### 2.1 Direct Mappings

Most Jacquard constructs lower directly to equivalent C++:

| Jacquard | C++ |
|---|---|
| `i8..i64, u8..u64, f32, f64` | `int8_t..int64_t, uint8_t..uint64_t, float, double` |
| `bool` | `bool` |
| `string` | `std::string` (configurable via `--string-type`) |
| `fn add(x: i32, y: i32) -> i32 { ... }` | `int32_t add(int32_t x, int32_t y) { ... }` |
| `if/else, while, for, return` | `if/else, while, for, return` |
| `struct Point { x: f32, y: f32 }` | `struct Point { float x; float y; };` |
| `match expr { patterns }` | `switch (expr._tag) { cases }` |
| `fn id<T>(x: T) -> T { ... }` | `template<typename T> T id(T x) { ... }` |

### 2.2 Async Task → State Machine

Tasks using `await` are lowered to switch-case state machines. Each `await` point becomes a state transition.

**Jacquard source:**
```
task load_assets() -> void {
    let tex = await load("tex.png");
    let sfx = await load("sfx.wav");
    register(tex, sfx);
}
```

**Generated C++:**
```cpp
struct Task_load_assets {
    int _state = 0;
    Texture tex; Sound sfx;
    Task<Texture> _t0; Task<Sound> _t1;

    bool tick() {
        switch (_state) {
            case 0:
                _t0 = load("tex.png");
                _state = 1;
                return false; // not done
            case 1:
                if (!_t0.ready()) return false;
                tex = _t0.value();
                _t1 = load("sfx.wav");
                _state = 2; return false;
            case 2:
                if (!_t1.ready()) return false;
                sfx = _t1.value();
                register(tex, sfx);
                return true; // done
        }
    }
};
```

- **Local variables become struct fields** — all state is captured in the generated struct.
- **`tick()` returns `false`** while the task is in progress, **`true`** when complete.
- **`await` is the only suspension point** — synchronous code between awaits runs to completion in that tick.

### 2.3 `?` Operator Lowering (Async)

The `?` operator on `Result<T, E>` compiles to an early-exit pattern in the state machine:

```
await fallible_operation()?;
```

Becomes:
```cpp
case N:
    if (!_tN.ready()) return false;
    auto _tmp = _tN.value();
    if (_tmp.is_err()) {
        _result = Result<...>::Err(_tmp.unwrap_err());
        return true; // done, with error
    }
    // ... continue with _tmp.unwrap()
```

- Sync `?` (no await) is a direct `if (result.is_err())` check — no state transition needed.
- Each task struct has a `_result` field acting as the early-return slot.

### 2.4 Name Mangling

Predictable, human-readable scheme: `_jq_{module}__{name}_{argtypes}_{return}`

```
player.jac :: fn greet(name: string) -> void
→ _jq_player__greet_str_void

engine.jac :: fn process(data: i32, flag: bool) -> f64
→ _jq_engine__process_i32_bool_f64
```

- Module name from filename (minus `.jac`).
- Primitive types abbreviated: `i8`, `i32`, `str`, `bool`, `f64`, `void`.
- Generic functions use `T` placeholders — C++ handles template instantiation mangling.
- Task types use PascalCase: `Task_load_assets` (convention for C++ struct).

### 2.5 Output Structure

Each Jacquard module produces one header/source pair:

```
player.jac  →  player.jq.h  +  player.jq.cpp
engine.jac  →  engine.jq.h  +  engine.jq.cpp
```

Plus one runtime header shipped with the compiler:

```
jacquard_runtime.h   // Task<T>, Runtime, Poll<T>, spawn(), tick()
```

---

## 3. Runtime Embedding

### 3.1 Host Engine Contract

The host engine (game engine, web server) owns the event loop and calls into the Jacquard runtime each frame/tick:

```cpp
#include "jacquard_runtime.h"
#include "game.jq.h"

int main() {
    jq::Runtime rt;
    rt.spawn<Task_game_loop>();

    while (running) {
        float dt = get_delta_time();
        rt.tick(dt);          // advance all spawned tasks
        render_frame();
    }
}
```

### 3.2 Runtime API

```cpp
namespace jq {

template<typename T>
struct Poll {
    bool ready;
    T value;
};

struct Task {
    virtual bool tick(float dt) = 0;  // true = done
    virtual ~Task() = default;
};

class Runtime {
public:
    template<typename T, typename... Args>
    void spawn(Args&&... args);

    void tick(float delta_time);

private:
    std::vector<std::unique_ptr<Task>> _tasks;
};

} // namespace jq
```

- **`spawn<T>()`** allocates a new task (one heap allocation at spawn time).
- **`tick()`** iterates all tasks, calls `tick(dt)`, removes completed tasks.
- **Tasks are owned by Runtime** via `std::unique_ptr`.

### 3.3 Memory Model

| Property | Decision |
|---|---|
| Ownership | Runtime owns all tasks via `unique_ptr` |
| Allocation | One heap alloc per `spawn()`. `tick()` is allocation-free |
| Collection | No GC. Pure RAII. Host engine's allocator is the only allocator |
| Concurrency | Single-threaded cooperative. `parallel { }` deferred to v2 |
| Determinism | `tick(float dt)` — host pushes time, tasks don't query it. Deterministic and testable |

### 3.4 FFI Boundary

**Jacquard → C++ (calling host functions):**
```
// Jacquard declaration
extern fn play_sound(path: string, volume: f32) -> void;

// Generated C++: direct extern call
extern void play_sound(std::string path, float volume);
```

**C++ → Jacquard (host calling Jacquard):**
```
// Jacquard declaration
export fn on_collision(a: Entity, b: Entity) -> void { ... }

// Generated C++: plain function callable from host
void jq_on_collision(Entity a, Entity b);
```

- `extern fn` = declared in Jacquard, implemented in C++.
- `export fn` = implemented in Jacquard, callable from C++.
- Both map to plain C++ function calls — no boxing, no marshalling overhead.

---

## 4. Module System

### 4.1 Syntax

```
// Import another module
import "player.jac";          // relative to current file
import "engine/physics";      // library path (no extension needed)
import "std/collections";     // standard library
```

### 4.2 Visibility

```
pub fn public_api() -> void { ... }      // exported to other modules
fn internal_helper() -> void { ... }     // module-private (default)

pub struct Vec3 { x: f32, y: f32, z: f32 }
pub enum Status { Ready, Busy, Error(string) }
```

- `pub` = visible outside the module → appears in `.jq.h`.
- Default (no `pub`) = module-private → only in `.jq.cpp`.
- No finer-grained visibility (`pub(crate)`, `protected`) in v1.

### 4.3 File Resolution

1. `"./foo.jac"` → relative to importing file's directory.
2. `"std/collections"` → compiler include path (`-I` flag).
3. Cyclic imports detected at compile time → error.

### 4.4 Namespace

Module name = filename (minus `.jac`) → generated code lives in `_jq_{module}::` namespace.

```
player.jac  →  namespace _jq_player { ... }
```

This prevents symbol collisions across modules without requiring explicit `namespace` declarations in Jacquard source.

---

## 5. Error Handling

### 5.1 Compile-Time Errors

Every diagnostic carries:
- `severity`: `error` | `warning` | `note`
- `span`: byte offsets into source file
- `message`: human-readable description
- `hint` (optional): suggestion for fixing

**Parser error recovery:** skip to `}`, `;`, or next declaration keyword on parse error (already specified in parser design).

**Type error localization:** errors localize to the function where the type mismatch occurs (benefit of bidirectional inference).

**Multiple errors collected per compilation** — don't bail on first error.

### 5.2 Runtime Error Model

```
// Recoverable errors — explicit Result type
task load_config() -> Result<Config, IoError> {
    let data = await read_file("config.json")?;  // ? propagates error
    return parse(data)?;
}

// Unrecoverable — abort the task
panic("unreachable: invalid state");

// Recoverable — match on Result
match load_config() {
    Ok(cfg) => apply(cfg),
    Err(e) => log("config failed: {}", e),
}
```

| Mechanism | Behavior |
|---|---|
| `Result<T, E>` | Explicit success/error union. Compiler enforces handling |
| `?` operator | Propagates `Err(e)` up the call stack. In tasks: early-exit in state machine |
| `panic()` | Calls host-provided panic hook. Default: log + abort the task |
| Task failure | Task returns `Result::Err` or panics → runtime marks as failed. Engine queries `task.status()` |
| No exceptions | Cooperative model — task failure is "done, with error." No unwinding |

**Design principle:** Errors are visible at every boundary. No silent swallowing, no implicit exception propagation across await points.

### 5.3 Panic Hook

The host engine can register a panic handler:

```cpp
jq::Runtime rt;
rt.set_panic_hook([](const char* msg, const char* file, int line) {
    log_error("Jacquard panic: {} at {}:{}", msg, file, line);
    // Engine decides: abort, restart task, ignore
});
```

Default behavior: log to stderr, abort the panicking task.

---

## 6. Standard Library (Built-in Types)

The following types are available in every Jacquard module without explicit import:

| Type | Variants / Description |
|---|---|
| `Option<T>` | `Some(T) \| None` |
| `Result<T, E>` | `Ok(T) \| Err(E)` |
| `Vec<T>` | Dynamic array, grows at end |
| `HashMap<K, V>` | Hash map, `K` must be hashable |
| `String` | Alias for the configured string type |

### 6.1 String Type Configuration

Configurable via compiler flag:

```
--string-type=std::string    (default)
--string-type=jq::String     (custom — must implement jq::StringTrait)
```

If using a custom string type, the host must provide a specialization of `jq::StringTrait` with the required operations (length, concat, substring, etc.).

---

## 7. Primitive Types (Fixed-Width Only)

Jacquard uses **fixed-width types only** — no platform-dependent `int` or `float`.

| Jacquard | C++ | Size |
|---|---|---|
| `i8` | `int8_t` | 1 byte |
| `i16` | `int16_t` | 2 bytes |
| `i32` | `int32_t` | 4 bytes |
| `i64` | `int64_t` | 8 bytes |
| `u8` | `uint8_t` | 1 byte |
| `u16` | `uint16_t` | 2 bytes |
| `u32` | `uint32_t` | 4 bytes |
| `u64` | `uint64_t` | 8 bytes |
| `f32` | `float` | 4 bytes |
| `f64` | `double` | 8 bytes |
| `bool` | `bool` | 1 byte |

---

## 8. `workflow` Construct

A `workflow` is syntactic sugar over a `task` that always returns `void`:

```
workflow startup() -> void {
    await load_assets();
    await connect_server();
    init_ui();
}
```

- **Identical lowering** to `task` — same state machine pattern.
- **Always returns `void`** — a workflow coordinates other tasks; it doesn't compute a value.
- **Separate keyword** for intent signaling — guides the reader and enables future tooling (task dependency graph visualization).
- **Future:** `parallel { ... }` blocks within workflows for concurrent task execution (v2).

---

## 9. Deferred to Future Versions

| Feature | Version | Notes |
|---|---|---|
| Trait/typeclass bounds | v2 | Unconstrained generics in v1 |
| `parallel { ... }` blocks | v2 | Concurrent task execution within workflows |
| `std::variant` for recursive ADTs | v2 (if needed) | Only if tagged unions prove insufficient |
| Finer-grained visibility (`pub(crate)`, etc.) | v2 | `pub`/default sufficient for prototype |
| Module-qualified imports (`import { foo, bar } from "mod"`) | v2 | Whole-module imports in v1 |
| Custom allocator integration | v2 | `unique_ptr` with default allocator in v1 |

---

## 10. Design Decision Registry

| # | Decision | Rationale |
|---|---|---|
| 1 | Bidirectional inference | Localizes errors to function boundaries; balances velocity, error quality, and C++ readability |
| 2 | Tagged unions over `std::variant` | Zero-dependency C++11+ output; predictable memory layout; switch-case dispatch is fastest |
| 3 | Angle-bracket generics, unconstrained | Modern syntax; monomorphization via C++ templates is zero-cost; trait bounds deferred |
| 4 | Union-find type variables | Standard H-M unification data structure; O(α(n)) with path compression |
| 5 | Switch-case state machines | Deterministic; allocation-free tick(); game engines already poll this way |
| 6 | Module-prefixed name mangling | Prevents cross-module symbol collisions; human-readable for debugging |
| 7 | `pub`/default visibility | Simple, familiar; finer-grained visibility deferred |
| 8 | `Result<T,E>` + `?` + `panic()` | Explicit error handling at every boundary; no hidden exception propagation |
| 9 | Fixed-width integers only | Game engines need deterministic sizes; no platform-dependent `int` |
| 10 | `workflow` = `task` syntactic sugar | Simpler implementation; separate keyword preserves intent for future tooling |