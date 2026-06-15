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

// ---------------------------------------------------------------------------
// Full pipeline integration tests (lex → parse → type-check → codegen)
// ---------------------------------------------------------------------------

mod integration_tests {

    #[test]
    fn test_full_pipeline_simple_fn() {
        let source = "fn add(x: i32, y: i32) -> i32 { return x + y; }";
        let result = jacquard::compile(source, "test");
        assert!(result.is_ok(), "pipeline should succeed, got: {:?}", result.err());
        let output = result.unwrap();
        assert!(!output.source.is_empty(), "source should not be empty");
        assert!(output.source.contains("add"), "source should contain function name");
    }

    #[test]
    fn test_full_pipeline_struct_and_fn() {
        let source = r#"
            pub struct Point { x: f32, y: f32 }
            pub fn distance(a: Point, b: Point) -> f64 { return 0.0; }
        "#;
        let result = jacquard::compile(source, "geometry");
        assert!(result.is_ok(), "pipeline should succeed, got: {:?}", result.err());
        let output = result.unwrap();
        assert!(output.header.contains("Point"), "header should contain struct");
        assert!(!output.header.is_empty(), "header should not be empty");
    }

    #[test]
    fn test_full_pipeline_task() {
        let source = r#"
            task countdown(n: i32) -> void {
                let i = n;
                return;
            }
        "#;
        let result = jacquard::compile(source, "tasks");
        assert!(result.is_ok(), "task pipeline should succeed, got: {:?}", result.err());
        let output = result.unwrap();
        assert!(output.source.contains("_state"), "task should have state counter");
        assert!(output.source.contains("tick"), "task should have tick() method");
    }

    #[test]
    fn test_full_pipeline_workflow() {
        let source = r#"
            workflow main {
                return;
            }
        "#;
        let result = jacquard::compile(source, "app");
        assert!(result.is_ok(), "workflow pipeline should succeed, got: {:?}", result.err());
        let output = result.unwrap();
        assert!(output.source.contains("Workflow_main"), "should contain workflow struct");
    }

    #[test]
    fn test_full_pipeline_generic_struct() {
        let source = r#"
            pub struct Pair<A, B> { first: A, second: B }
        "#;
        let result = jacquard::compile(source, "generic");
        assert!(result.is_ok(), "pipeline should succeed, got: {:?}", result.err());
        let output = result.unwrap();
        assert!(output.header.contains("Pair"), "header should contain Pair");
        assert!(output.header.contains("template"), "header should have template");
    }

    #[test]
    fn test_full_pipeline_tagged_union_enum() {
        let source = r#"
            pub enum Option<T> { Some(T), None }
        "#;
        let result = jacquard::compile(source, "adt");
        assert!(result.is_ok(), "pipeline should succeed, got: {:?}", result.err());
        let output = result.unwrap();
        assert!(output.header.contains("Option"), "header should contain Option");
        assert!(output.header.contains("Tag"), "enum should have Tag enum class");
    }

    #[test]
    fn test_full_pipeline_type_error_is_caught() {
        let source = r#"
            fn bad(x: i32) -> string { return x; }
        "#;
        let result = jacquard::compile(source, "err_test");
        assert!(result.is_err(), "should fail type checking");
        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(err_msg.contains("type error") || err_msg.contains("mismatch")
            || err_msg.contains("Type"),
            "error should mention type, got: {}", err_msg);
    }

    #[test]
    fn test_full_pipeline_if_condition_type_error() {
        let source = r#"
            fn test() -> void { if 42 { return; } return; }
        "#;
        let result = jacquard::compile(source, "err_test");
        assert!(result.is_err(), "should fail: if condition must be bool");
    }

    #[test]
    fn test_full_pipeline_undeclared_variable_error() {
        let source = r#"
            fn test() -> void { let x = y; return; }
        "#;
        let result = jacquard::compile(source, "err_test");
        assert!(result.is_err(), "should fail: y is undeclared");
    }
}