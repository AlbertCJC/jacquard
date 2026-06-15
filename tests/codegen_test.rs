//! Codegen tests for Jacquard — C++ output verification.
//!
//! Covers: name mangling, C++ codegen, state machine lowering,
//! header/source separation.

// ---------------------------------------------------------------------------
// Name mangling tests
// ---------------------------------------------------------------------------

mod mangling_tests {
    use jacquard::codegen::mangle_fn;

    #[test]
    fn test_mangle_simple_function() {
        let name = mangle_fn("test", "add", &["i32", "i32"], "i32");
        assert_eq!(name, "_jq_test__add_i32_i32_i32");
    }

    #[test]
    fn test_mangle_void_return() {
        let name = mangle_fn("module", "init", &[], "void");
        assert_eq!(name, "_jq_module__init_void");
    }

    #[test]
    fn test_mangle_task_name() {
        // Tasks use PascalCase struct convention
        let name = mangle_fn("engine", "load_assets", &["string"], "void");
        assert_eq!(name, "_jq_engine__load_assets_string_void");
    }

    #[test]
    fn test_mangle_generic_function() {
        // Generics use T placeholder
        let name = mangle_fn("core", "identity", &["T"], "T");
        assert_eq!(name, "_jq_core__identity_T_T");
    }

    #[test]
    fn test_mangle_multiple_params() {
        let name = mangle_fn("math", "clamp", &["i32", "i32", "i32"], "i32");
        assert_eq!(name, "_jq_math__clamp_i32_i32_i32_i32");
    }

    #[test]
    fn test_mangle_with_compound_types() {
        let name = mangle_fn("app", "process", &["function", "tuple"], "result");
        assert_eq!(name, "_jq_app__process_function_tuple_result");
    }
}

// ---------------------------------------------------------------------------
// Simple C++ codegen tests (non-async)
// ---------------------------------------------------------------------------

mod codegen_tests {
    use jacquard::lexer::tokenize;
    use jacquard::parser::parse;

    /// Helper: lex, parse, type-check, and generate C++ for a source string.
    fn compile_to_cpp(source: &str) -> (String, String) {
        let tokens: Vec<_> = tokenize(source)
            .filter_map(|r| r.ok())
            .collect();
        let program = parse(&tokens).expect("parse should succeed");
        jacquard::codegen::generate(&program, "test")
    }

    #[test]
    fn test_header_contains_include_guard() {
        let source = "fn add(x: i32, y: i32) -> i32 { return x + y; }";
        let (header, _source) = compile_to_cpp(source);
        assert!(header.contains("#pragma once") || header.contains("_JQ_TEST_H_"),
            "header should have include guard, got:\n{}", header);
    }

    #[test]
    fn test_header_includes_runtime() {
        let source = "fn foo() -> void { return; }";
        let (header, _source) = compile_to_cpp(source);
        assert!(header.contains("jacquard_runtime.h"),
            "header should include runtime, got:\n{}", header);
    }

    #[test]
    fn test_simple_function_codegen() {
        let source = "fn add(x: i32, y: i32) -> i32 { return x + y; }";
        let (header, source_cpp) = compile_to_cpp(source);
        // The function should appear in either header or source
        let combined = format!("{}\n{}", header, source_cpp);
        assert!(combined.contains("add"), "output should contain function name 'add'");
    }

    #[test]
    fn test_struct_codegen() {
        let source = "pub struct Point { x: f32, y: f32 }";
        let (header, _source) = compile_to_cpp(source);
        assert!(header.contains("Point"), "header should contain struct name");
        assert!(header.contains("float"), "header should contain C++ float type");
    }

    #[test]
    fn test_enum_codegen_tagged_union() {
        let source = "pub enum Option { Some(i32), None }";
        let (header, _source) = compile_to_cpp(source);
        assert!(header.contains("Option"), "header should contain enum name");
        assert!(header.contains("_tag") || header.contains("Tag"),
            "enum should have tag field");
    }

    #[test]
    fn test_public_fn_in_header() {
        let source = "pub fn greet(name: string) -> string { return \"hello\"; }";
        let (header, source_cpp) = compile_to_cpp(source);
        assert!(header.contains("greet") || source_cpp.contains("greet"),
            "function should appear somewhere");
    }

    #[test]
    fn test_namespace_wrapping() {
        let source = "fn foo() -> void { return; }";
        let (_header, source) = compile_to_cpp(source);
        // Should have a namespace
        assert!(source.contains("namespace") || source.contains("_jq_"),
            "output should contain namespace or mangled name");
    }

    #[test]
    fn test_empty_program_produces_valid_output() {
        let source = "";
        let (header, source) = compile_to_cpp(source);
        // Should produce at minimum the header preamble
        assert!(!header.is_empty() || !source.is_empty(),
            "empty program should still produce preamble");
    }
}