//! Name mangling for Jacquard symbols.
//!
//! Produces predictable C-compatible symbol names in the format:
//! `_jq_{module}__{name}_{abbrev_params}_{abbrev_ret}`
//!
//! ## Abbreviations
//! | Jacquard | C++ mangled |
//! |----------|-------------|
//! | i8..i64  | i8..i64     |
//! | u8..u64  | u8..u64     |
//! | f32, f64 | f32, f64    |
//! | bool     | bool        |
//! | string   | string      |
//! | void     | void        |
//! | T        | T           (generic placeholder)

/// Mangle a function name into its C++ linkage symbol.
///
/// `params` and `ret` are the abbreviated type names (e.g., `"i32"`, `"void"`).
pub fn mangle_fn(module: &str, name: &str, params: &[&str], ret: &str) -> String {
    let mut mangled = format!("_jq_{}__{}", module, name);

    for param in params {
        mangled.push('_');
        mangled.push_str(&mangle_type_name(param));
    }
    // Separator between params and return type
    mangled.push('_');
    mangled.push_str(&mangle_type_name(ret));

    mangled
}

/// Mangle a task struct name.
///
/// Tasks are compiled to C++ structs with PascalCase naming. The mangled
/// name follows the same convention as regular functions but with a `Task`
/// prefix convention.
pub fn mangle_task(module: &str, name: &str, params: &[&str], ret: &str) -> String {
    // Tasks use the same mangling as functions for the struct name
    mangle_fn(module, name, params, ret)
}

/// Convert a type name to its mangled abbreviation.
fn mangle_type_name(name: &str) -> String {
    // Pass through common primitive names directly
    match name {
        "i8" | "i16" | "i32" | "i64" => name.to_string(),
        "u8" | "u16" | "u32" | "u64" => name.to_string(),
        "f32" | "f64" => name.to_string(),
        "bool" => name.to_string(),
        "string" => name.to_string(),
        "void" => name.to_string(),
        // Generic placeholder
        "T" => name.to_string(),
        // User-defined types: pass name through (PascalCase expected)
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mangle_simple() {
        assert_eq!(
            mangle_fn("test", "add", &["i32", "i32"], "i32"),
            "_jq_test__add_i32_i32_i32"
        );
    }

    #[test]
    fn test_mangle_no_params() {
        assert_eq!(
            mangle_fn("module", "init", &[], "void"),
            "_jq_module__init_void"
        );
    }

    #[test]
    fn test_mangle_generic() {
        assert_eq!(
            mangle_fn("core", "identity", &["T"], "T"),
            "_jq_core__identity_T_T"
        );
    }
}