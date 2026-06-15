//! Type system tests for Jacquard.
//!
//! Covers: Type IR, TypeVarTable, unification, bidirectional inference.

use jacquard::types::{Type, TypeVarTable};

// ---------------------------------------------------------------------------
// Type IR tests
// ---------------------------------------------------------------------------

#[test]
fn test_primitive_types_are_distinct() {
    // Different primitives should not be equal.
    assert!(Type::I32 != Type::Bool);
    assert!(Type::I32 != Type::F64);
    assert!(Type::String != Type::Void);
    // Same primitives should be equal.
    assert_eq!(Type::I32, Type::I32);
    assert_eq!(Type::Bool, Type::Bool);
}

#[test]
fn test_function_type_construction() {
    let fn_type = Type::Function(
        vec![Type::I32, Type::Bool],
        Box::new(Type::String),
    );
    // Verify it was constructed
    match &fn_type {
        Type::Function(params, ret) => {
            assert_eq!(params.len(), 2);
            assert_eq!(params[0], Type::I32);
            assert_eq!(params[1], Type::Bool);
            assert_eq!(**ret, Type::String);
        }
        _ => panic!("expected Function type"),
    }
}

#[test]
fn test_tuple_type_construction() {
    let tuple_type = Type::Tuple(vec![Type::I32, Type::F64, Type::Bool]);
    match &tuple_type {
        Type::Tuple(elements) => {
            assert_eq!(elements.len(), 3);
            assert_eq!(elements[0], Type::I32);
        }
        _ => panic!("expected Tuple type"),
    }
}

#[test]
fn test_named_and_generic_types() {
    let named = Type::Named("MyStruct".to_string());
    match &named {
        Type::Named(name) => assert_eq!(name, "MyStruct"),
        _ => panic!("expected Named type"),
    }

    let generic = Type::Generic(
        "Option".to_string(),
        vec![Type::I32],
    );
    match &generic {
        Type::Generic(name, args) => {
            assert_eq!(name, "Option");
            assert_eq!(args.len(), 1);
            assert_eq!(args[0], Type::I32);
        }
        _ => panic!("expected Generic type"),
    }
}

#[test]
fn test_type_var_construction() {
    let var = Type::Var(0);
    match var {
        Type::Var(id) => assert_eq!(id, 0),
        _ => panic!("expected Var type"),
    }
}

#[test]
fn test_error_type_is_error() {
    // Error type should exist and be distinct from primitives
    assert!(Type::Error != Type::I32);
    assert_eq!(Type::Error, Type::Error);
}

// ---------------------------------------------------------------------------
// TypeVarTable tests (union-find with path compression)
// ---------------------------------------------------------------------------

#[test]
fn test_new_var_is_unbound() {
    let mut table = TypeVarTable::new();
    let id = table.new_var();
    // Fresh variable should be unbound
    let state = table.resolve(id);
    assert!(state.is_unbound(), "new var should be unbound, got: {:?}", state);
}

#[test]
fn test_bind_var_sets_type() {
    let mut table = TypeVarTable::new();
    let id = table.new_var();
    table.bind(id, Type::I32);
    // After binding, should be bound to I32
    let state = table.resolve(id);
    assert!(state.is_bound(), "var should be bound, got: {:?}", state);
}

#[test]
fn test_link_and_resolve_vars() {
    let mut table = TypeVarTable::new();
    let a = table.new_var();
    let b = table.new_var();
    table.union(a, b);

    // After union, they should resolve to the same underlying var
    let ra = table.find_root(a);
    let rb = table.find_root(b);
    assert_eq!(ra, rb);
}

// ---------------------------------------------------------------------------
// Unification tests
// ---------------------------------------------------------------------------

#[test]
fn test_unify_identical_primitives() {
    let mut table = TypeVarTable::new();
    let result = table.unify(&Type::I32, &Type::I32);
    assert!(result.is_ok());
}

#[test]
fn test_unify_different_primitives_fails() {
    let mut table = TypeVarTable::new();
    let result = table.unify(&Type::I32, &Type::Bool);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("mismatch"));
}

#[test]
fn test_unify_var_with_concrete_type() {
    let mut table = TypeVarTable::new();
    let var = table.new_var();
    let result = table.unify(&Type::Var(var), &Type::I32);
    assert!(result.is_ok());
    // Var should now be bound to I32
    assert_eq!(table.resolve_type(var), Some(Type::I32));
}

#[test]
fn test_unify_two_vars() {
    let mut table = TypeVarTable::new();
    let a = table.new_var();
    let b = table.new_var();
    let result = table.unify(&Type::Var(a), &Type::Var(b));
    assert!(result.is_ok());
    // They should now resolve to the same root
    assert_eq!(table.find_root(a), table.find_root(b));
}

#[test]
fn test_unify_function_types() {
    let mut table = TypeVarTable::new();
    let fn1 = Type::Function(
        vec![Type::I32, Type::Bool],
        Box::new(Type::String),
    );
    let fn2 = Type::Function(
        vec![Type::I32, Type::Bool],
        Box::new(Type::String),
    );
    let result = table.unify(&fn1, &fn2);
    assert!(result.is_ok());
}

#[test]
fn test_unify_function_types_mismatched_arity_fails() {
    let mut table = TypeVarTable::new();
    let fn1 = Type::Function(vec![Type::I32], Box::new(Type::Void));
    let fn2 = Type::Function(vec![Type::I32, Type::Bool], Box::new(Type::Void));
    let result = table.unify(&fn1, &fn2);
    assert!(result.is_err());
}

#[test]
fn test_unify_function_types_mismatched_return_fails() {
    let mut table = TypeVarTable::new();
    let fn1 = Type::Function(vec![Type::I32], Box::new(Type::Void));
    let fn2 = Type::Function(vec![Type::I32], Box::new(Type::String));
    let result = table.unify(&fn1, &fn2);
    assert!(result.is_err());
}

#[test]
fn test_unify_generic_with_instantiated() {
    let mut table = TypeVarTable::new();
    // Option<T> with Option<i32>
    let t_var = table.new_var();
    let generic = Type::Generic("Option".to_string(), vec![Type::Var(t_var)]);
    let concrete = Type::Generic("Option".to_string(), vec![Type::I32]);
    let result = table.unify(&generic, &concrete);
    assert!(result.is_ok());
    // t_var should now be bound to I32
    assert_eq!(table.resolve_type(t_var), Some(Type::I32));
}

#[test]
fn test_unify_named_types() {
    let mut table = TypeVarTable::new();
    let result = table.unify(
        &Type::Named("MyStruct".to_string()),
        &Type::Named("MyStruct".to_string()),
    );
    assert!(result.is_ok());
}

#[test]
fn test_unify_different_named_types_fails() {
    let mut table = TypeVarTable::new();
    let result = table.unify(
        &Type::Named("Foo".to_string()),
        &Type::Named("Bar".to_string()),
    );
    assert!(result.is_err());
}

#[test]
fn test_unify_error_type_always_succeeds() {
    let mut table = TypeVarTable::new();
    // Error types should always unify (poison pill)
    let result = table.unify(&Type::Error, &Type::I32);
    assert!(result.is_ok());
    let result = table.unify(&Type::I32, &Type::Error);
    assert!(result.is_ok());
}

// ---------------------------------------------------------------------------
// Inference tests — use the parser to build AST, then type-check it
// ---------------------------------------------------------------------------

mod inference_tests {
    use jacquard::types::{Type, TypeVarTable, TypeEnv};
    use jacquard::lexer::tokenize;
    use jacquard::parser::parse;

    /// Helper: parse source, return the first function declaration's body.
    /// For expression tests, wraps the expression in a simple function.
    fn parse_expr(source: &str) -> jacquard::ast::Expr {
        // Wrap standalone expression in a function body for parsing
        let wrapped = format!("fn _test() -> void {{ {}; }}", source);
        let tokens: Vec<_> = tokenize(&wrapped)
            .filter_map(|r| r.ok())
            .collect();
        let program = parse(&tokens).expect("parse should succeed");
        match &program.declarations[0] {
            jacquard::ast::Declaration::Fn(f) => {
                match &f.body.statements[0] {
                    jacquard::ast::Statement::ExprStmt(e) => (*e).clone(),
                    other => panic!("expected ExprStmt, got {:?}", other),
                }
            }
            other => panic!("expected Fn declaration, got {:?}", other),
        }
    }

    #[test]
    fn test_infer_int_literal() {
        let mut env = TypeEnv::new();
        let mut table = TypeVarTable::new();
        let expr = parse_expr("42");
        let ty = jacquard::types::infer_expr(&expr, &mut env, &mut table).unwrap();
        assert_eq!(ty, Type::I32);
    }

    #[test]
    fn test_infer_float_literal() {
        let mut env = TypeEnv::new();
        let mut table = TypeVarTable::new();
        let expr = parse_expr("3.14");
        let ty = jacquard::types::infer_expr(&expr, &mut env, &mut table).unwrap();
        assert_eq!(ty, Type::F64);
    }

    #[test]
    fn test_infer_string_literal() {
        let mut env = TypeEnv::new();
        let mut table = TypeVarTable::new();
        let expr = parse_expr("\"hello\"");
        let ty = jacquard::types::infer_expr(&expr, &mut env, &mut table).unwrap();
        assert_eq!(ty, Type::String);
    }

    #[test]
    fn test_infer_bool_literal() {
        let mut env = TypeEnv::new();
        let mut table = TypeVarTable::new();
        let expr = parse_expr("true");
        let ty = jacquard::types::infer_expr(&expr, &mut env, &mut table).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_infer_variable_from_env() {
        let mut env = TypeEnv::new();
        let mut table = TypeVarTable::new();
        env.insert("x".to_string(), Type::I32);
        let expr = parse_expr("x");
        let ty = jacquard::types::infer_expr(&expr, &mut env, &mut table).unwrap();
        assert_eq!(ty, Type::I32);
    }

    #[test]
    fn test_infer_undeclared_variable_fails() {
        let mut env = TypeEnv::new();
        let mut table = TypeVarTable::new();
        let expr = parse_expr("undeclared_var");
        let result = jacquard::types::infer_expr(&expr, &mut env, &mut table);
        assert!(result.is_err());
    }

    #[test]
    fn test_infer_binary_addition_ints() {
        let mut env = TypeEnv::new();
        let mut table = TypeVarTable::new();
        let expr = parse_expr("1 + 2");
        let ty = jacquard::types::infer_expr(&expr, &mut env, &mut table).unwrap();
        assert_eq!(ty, Type::I32);
    }

    #[test]
    fn test_infer_binary_comparison() {
        let mut env = TypeEnv::new();
        let mut table = TypeVarTable::new();
        let expr = parse_expr("1 < 2");
        let ty = jacquard::types::infer_expr(&expr, &mut env, &mut table).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_infer_unary_negation() {
        let mut env = TypeEnv::new();
        let mut table = TypeVarTable::new();
        let expr = parse_expr("-42");
        let ty = jacquard::types::infer_expr(&expr, &mut env, &mut table).unwrap();
        assert_eq!(ty, Type::I32);
    }

    #[test]
    fn test_infer_unary_not() {
        let mut env = TypeEnv::new();
        let mut table = TypeVarTable::new();
        let expr = parse_expr("!true");
        let ty = jacquard::types::infer_expr(&expr, &mut env, &mut table).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_infer_parenthesized() {
        let mut env = TypeEnv::new();
        let mut table = TypeVarTable::new();
        let expr = parse_expr("(42)");
        let ty = jacquard::types::infer_expr(&expr, &mut env, &mut table).unwrap();
        assert_eq!(ty, Type::I32);
    }

    // -----------------------------------------------------------------------
    // Checking mode (top-down)
    // -----------------------------------------------------------------------

    #[test]
    fn test_check_expr_matches_expected() {
        let mut env = TypeEnv::new();
        let mut table = TypeVarTable::new();
        let expr = parse_expr("42");
        let result = jacquard::types::check_expr(
            &expr, &Type::I32, &mut env, &mut table,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_expr_type_mismatch_fails() {
        let mut env = TypeEnv::new();
        let mut table = TypeVarTable::new();
        let expr = parse_expr("\"hello\"");
        let result = jacquard::types::check_expr(
            &expr, &Type::I32, &mut env, &mut table,
        );
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Full program type checking
    // -----------------------------------------------------------------------

    #[test]
    fn test_check_simple_function() {
        let source = "fn add(x: i32, y: i32) -> i32 { return x + y; }";
        let tokens: Vec<_> = tokenize(source)
            .filter_map(|r| r.ok())
            .collect();
        let program = parse(&tokens).unwrap();
        let result = jacquard::types::check_program(&program);
        assert!(result.is_ok(), "expected ok, got: {:?}", result.err());
    }

    #[test]
    fn test_check_function_return_type_mismatch() {
        let source = "fn bad(x: i32) -> string { return x; }";
        let tokens: Vec<_> = tokenize(source)
            .filter_map(|r| r.ok())
            .collect();
        let program = parse(&tokens).unwrap();
        let result = jacquard::types::check_program(&program);
        assert!(result.is_err(), "expected type mismatch error");
    }

    #[test]
    fn test_check_let_with_type_annotation() {
        let source = "fn test() -> void { let x: i32 = 42; return; }";
        let tokens: Vec<_> = tokenize(source)
            .filter_map(|r| r.ok())
            .collect();
        let program = parse(&tokens).unwrap();
        let result = jacquard::types::check_program(&program);
        assert!(result.is_ok(), "expected ok, got: {:?}", result.err());
    }

    #[test]
    fn test_check_let_type_mismatch() {
        let source = "fn test() -> void { let x: i32 = \"hello\"; return; }";
        let tokens: Vec<_> = tokenize(source)
            .filter_map(|r| r.ok())
            .collect();
        let program = parse(&tokens).unwrap();
        let result = jacquard::types::check_program(&program);
        assert!(result.is_err(), "expected type mismatch error");
    }

    #[test]
    fn test_check_if_condition_must_be_bool() {
        let source = "fn test() -> void { if 42 { return; } return; }";
        let tokens: Vec<_> = tokenize(source)
            .filter_map(|r| r.ok())
            .collect();
        let program = parse(&tokens).unwrap();
        let result = jacquard::types::check_program(&program);
        assert!(result.is_err(), "if condition should be bool");
    }

    #[test]
    fn test_check_if_with_valid_condition() {
        let source = "fn test() -> void { if true { return; } return; }";
        let tokens: Vec<_> = tokenize(source)
            .filter_map(|r| r.ok())
            .collect();
        let program = parse(&tokens).unwrap();
        let result = jacquard::types::check_program(&program);
        assert!(result.is_ok(), "expected ok, got: {:?}", result.err());
    }
}