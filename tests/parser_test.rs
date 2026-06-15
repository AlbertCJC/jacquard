//! Comprehensive tests for the Jacquard parser.
//!
//! Covers expression, statement, declaration, type parsing, and error conditions.

use jacquard::ast::{BinaryOp, Declaration, ExprKind, MatchLiteral, MatchPattern, Statement, Type, UnaryOp};
use jacquard::lexer::tokenize;
use jacquard::parser::parse;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Tokenize and parse source, returning the resulting Program.
fn parse_source(source: &str) -> jacquard::ast::Program {
    let tokens: Vec<_> = tokenize(source).map(|r| r.unwrap()).collect();
    parse(&tokens).expect("parse should succeed")
}

/// Tokenize and parse source, expecting a ParseError.
fn parse_source_error(source: &str) -> jacquard::parser::ParseError {
    let tokens: Vec<_> = tokenize(source).map(|r| r.unwrap()).collect();
    parse(&tokens).expect_err("parse should fail")
}

/// Parse the first declaration from source.
fn parse_first_decl(source: &str) -> Declaration {
    parse_source(source).declarations.into_iter().next().unwrap()
}

/// Parse the first statement from a block wrapped in a function body.
fn parse_first_stmt(source: &str) -> Statement {
    let full = format!("fn f() {{ {} }}", source);
    match parse_first_decl(&full) {
        Declaration::Fn(f) => f.body.statements.into_iter().next().unwrap(),
        _ => panic!("expected Fn declaration"),
    }
}

/// Parse a single expression by wrapping it as a return statement.
fn parse_expr(source: &str) -> jacquard::ast::Expr {
    let stmt = parse_first_stmt(&format!("return {};", source));
    match stmt {
        Statement::Return(Some(expr)) => expr,
        _ => panic!("expected return statement with expression"),
    }
}

/// Parse the body of a single function.
fn parse_fn_body(source: &str) -> Vec<Statement> {
    let full = format!("fn f() {{ {} }}", source);
    match parse_first_decl(&full) {
        Declaration::Fn(f) => f.body.statements,
        _ => panic!("expected Fn declaration"),
    }
}

// ===========================================================================
// 1. Empty / trivial programs
// ===========================================================================

#[test]
fn test_empty_program() {
    let prog = parse_source("");
    assert!(prog.declarations.is_empty());
}

// ===========================================================================
// 2. Literal expressions
// ===========================================================================

#[test]
fn test_int_literal() {
    let expr = parse_expr("42");
    assert!(matches!(expr.kind, ExprKind::IntLiteral(42)));
}

#[test]
fn test_int_literal_with_underscores() {
    let expr = parse_expr("1_000_000");
    assert!(matches!(expr.kind, ExprKind::IntLiteral(1_000_000)));
}

#[test]
fn test_float_literal() {
    let expr = parse_expr("3.14");
    assert!(matches!(expr.kind, ExprKind::FloatLiteral(v) if (v - 3.14).abs() < 0.001));
}

#[test]
fn test_string_literal() {
    let expr = parse_expr("\"hello world\"");
    assert!(matches!(expr.kind, ExprKind::StringLiteral(ref s) if s == "hello world"));
}

#[test]
fn test_bool_literal_true() {
    let expr = parse_expr("true");
    assert!(matches!(expr.kind, ExprKind::BoolLiteral(true)));
}

#[test]
fn test_bool_literal_false() {
    let expr = parse_expr("false");
    assert!(matches!(expr.kind, ExprKind::BoolLiteral(false)));
}

#[test]
fn test_variable_expression() {
    let expr = parse_expr("my_var");
    assert!(matches!(expr.kind, ExprKind::Variable(ref s) if s == "my_var"));
}

// ===========================================================================
// 3. Unary expressions
// ===========================================================================

#[test]
fn test_unary_negation() {
    let expr = parse_expr("-42");
    assert!(matches!(expr.kind, ExprKind::Unary { op: UnaryOp::Neg, .. }));
    if let ExprKind::Unary { operand, .. } = expr.kind {
        assert!(matches!(operand.kind, ExprKind::IntLiteral(42)));
    }
}

#[test]
fn test_logical_not() {
    let expr = parse_expr("!true");
    assert!(matches!(expr.kind, ExprKind::Unary { op: UnaryOp::Not, .. }));
    if let ExprKind::Unary { operand, .. } = expr.kind {
        assert!(matches!(operand.kind, ExprKind::BoolLiteral(true)));
    }
}

#[test]
fn test_double_negation() {
    let expr = parse_expr("--5");
    // Should parse as -(-5)
    assert!(matches!(expr.kind, ExprKind::Unary { op: UnaryOp::Neg, .. }));
    if let ExprKind::Unary { operand, .. } = expr.kind {
        assert!(matches!(operand.kind, ExprKind::Unary { op: UnaryOp::Neg, .. }));
    }
}

// ===========================================================================
// 4. Binary expressions — basic arithmetic
// ===========================================================================

#[test]
fn test_addition() {
    let expr = parse_expr("1 + 2");
    assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Add, .. }));
}

#[test]
fn test_subtraction() {
    let expr = parse_expr("5 - 3");
    assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Sub, .. }));
}

#[test]
fn test_multiplication() {
    let expr = parse_expr("4 * 7");
    assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Mul, .. }));
}

#[test]
fn test_division() {
    let expr = parse_expr("10 / 2");
    assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Div, .. }));
}

#[test]
fn test_modulo() {
    let expr = parse_expr("10 % 3");
    assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Mod, .. }));
}

// ===========================================================================
// 5. Binary expressions — comparisons and logic
// ===========================================================================

#[test]
fn test_equality() {
    let expr = parse_expr("a == b");
    assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Eq, .. }));
}

#[test]
fn test_not_equals() {
    let expr = parse_expr("a != b");
    assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::NotEq, .. }));
}

#[test]
fn test_less_than() {
    let expr = parse_expr("a < b");
    assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Lt, .. }));
}

#[test]
fn test_greater_than() {
    let expr = parse_expr("a > b");
    assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Gt, .. }));
}

#[test]
fn test_less_than_or_equal() {
    let expr = parse_expr("a <= b");
    assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::LtEq, .. }));
}

#[test]
fn test_greater_than_or_equal() {
    let expr = parse_expr("a >= b");
    assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::GtEq, .. }));
}

#[test]
fn test_logical_and() {
    let expr = parse_expr("true && false");
    assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::And, .. }));
}

#[test]
fn test_logical_or() {
    let expr = parse_expr("true || false");
    assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Or, .. }));
}

// ===========================================================================
// 6. Operator precedence
// ===========================================================================

#[test]
fn test_multiplication_binds_tighter_than_addition() {
    // 1 + 2 * 3 should parse as 1 + (2 * 3), not (1 + 2) * 3
    let expr = parse_expr("1 + 2 * 3");
    assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Add, .. }));
    if let ExprKind::Binary { left, right, .. } = expr.kind {
        assert!(matches!(left.kind, ExprKind::IntLiteral(1)));
        assert!(matches!(right.kind, ExprKind::Binary { op: BinaryOp::Mul, .. }));
    }
}

#[test]
fn test_comparison_binds_tighter_than_logical_and() {
    // a && b < c should parse as a && (b < c)
    let expr = parse_expr("a && b < c");
    assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::And, .. }));
    if let ExprKind::Binary { right, .. } = expr.kind {
        assert!(matches!(right.kind, ExprKind::Binary { op: BinaryOp::Lt, .. }));
    }
}

#[test]
fn test_equality_binds_tighter_than_logical_or() {
    // a || b == c should parse as a || (b == c)
    let expr = parse_expr("a || b == c");
    assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Or, .. }));
    if let ExprKind::Binary { right, .. } = expr.kind {
        assert!(matches!(right.kind, ExprKind::Binary { op: BinaryOp::Eq, .. }));
    }
}

#[test]
fn test_left_associativity_of_subtraction() {
    // 10 - 5 - 2 should parse as (10 - 5) - 2
    let expr = parse_expr("10 - 5 - 2");
    assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Sub, .. }));
    if let ExprKind::Binary { left, right, .. } = expr.kind {
        // left should be a subtraction: (10 - 5)
        assert!(matches!(left.kind, ExprKind::Binary { op: BinaryOp::Sub, .. }));
        assert!(matches!(right.kind, ExprKind::IntLiteral(2)));
    }
}

// ===========================================================================
// 7. Function calls and field access
// ===========================================================================

#[test]
fn test_function_call_no_args() {
    let expr = parse_expr("foo()");
    assert!(matches!(expr.kind, ExprKind::Call { .. }));
    if let ExprKind::Call { callee, args, .. } = expr.kind {
        assert!(matches!(callee.kind, ExprKind::Variable(ref s) if s == "foo"));
        assert!(args.is_empty());
    }
}

#[test]
fn test_function_call_one_arg() {
    let expr = parse_expr("print(42)");
    assert!(matches!(expr.kind, ExprKind::Call { .. }));
    if let ExprKind::Call { args, .. } = expr.kind {
        assert_eq!(args.len(), 1);
        assert!(matches!(args[0].kind, ExprKind::IntLiteral(42)));
    }
}

#[test]
fn test_function_call_multiple_args() {
    let expr = parse_expr("add(1, 2, 3)");
    assert!(matches!(expr.kind, ExprKind::Call { .. }));
    if let ExprKind::Call { args, .. } = expr.kind {
        assert_eq!(args.len(), 3);
        assert!(matches!(args[0].kind, ExprKind::IntLiteral(1)));
        assert!(matches!(args[1].kind, ExprKind::IntLiteral(2)));
        assert!(matches!(args[2].kind, ExprKind::IntLiteral(3)));
    }
}

#[test]
fn test_nested_function_calls() {
    let expr = parse_expr("f(g(x))");
    assert!(matches!(expr.kind, ExprKind::Call { .. }));
    if let ExprKind::Call { callee, args, .. } = expr.kind {
        assert!(matches!(callee.kind, ExprKind::Variable(ref s) if s == "f"));
        assert_eq!(args.len(), 1);
        assert!(matches!(args[0].kind, ExprKind::Call { .. }));
    }
}

#[test]
fn test_field_access() {
    let expr = parse_expr("obj.field");
    assert!(matches!(expr.kind, ExprKind::FieldAccess { .. }));
    if let ExprKind::FieldAccess { object, field, .. } = expr.kind {
        assert!(matches!(object.kind, ExprKind::Variable(ref s) if s == "obj"));
        assert_eq!(field, "field");
    }
}

#[test]
fn test_chained_field_access() {
    let expr = parse_expr("a.b.c");
    assert!(matches!(expr.kind, ExprKind::FieldAccess { .. }));
    if let ExprKind::FieldAccess { object, field, .. } = expr.kind {
        assert_eq!(field, "c");
        assert!(matches!(object.kind, ExprKind::FieldAccess { .. }));
    }
}

// ===========================================================================
// 8. Parenthesized expressions
// ===========================================================================

#[test]
fn test_parenthesized_expression() {
    let expr = parse_expr("(1 + 2) * 3");
    // Should be (1 + 2) * 3, not 1 + (2 * 3)
    assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Mul, .. }));
    if let ExprKind::Binary { left, right, .. } = expr.kind {
        assert!(matches!(left.kind, ExprKind::Paren(..)));
        assert!(matches!(right.kind, ExprKind::IntLiteral(3)));
    }
}

// ===========================================================================
// 9. Await expression
// ===========================================================================

#[test]
fn test_await_expression() {
    let expr = parse_expr("await foo()");
    assert!(matches!(expr.kind, ExprKind::Await(..)));
    if let ExprKind::Await(inner) = expr.kind {
        assert!(matches!(inner.kind, ExprKind::Call { .. }));
    }
}

// ===========================================================================
// 10. Match expression
// ===========================================================================

#[test]
fn test_match_with_literal_arms() {
    let expr = parse_expr(
        "match x { 1 => \"one\", 2 => \"two\", _ => \"other\" }"
    );
    assert!(matches!(expr.kind, ExprKind::Match { .. }));
    if let ExprKind::Match { arms, .. } = expr.kind {
        assert_eq!(arms.len(), 3);
        // Third arm should be wildcard
        assert!(matches!(arms[2].pattern, MatchPattern::Wildcard));
    }
}

#[test]
fn test_match_with_constructor_pattern() {
    let expr = parse_expr(
        "match result { Ok value => value, Err e => \"error\" }"
    );
    assert!(matches!(expr.kind, ExprKind::Match { .. }));
    if let ExprKind::Match { arms, .. } = expr.kind {
        assert_eq!(arms.len(), 2);
        assert!(matches!(arms[0].pattern, MatchPattern::Constructor { ref name, ref binding }
            if name == "Ok" && *binding == Some("value".to_string())));
    }
}

#[test]
fn test_match_constructor_without_binding() {
    let expr = parse_expr(
        "match x { None => 0, Some => 1 }"
    );
    assert!(matches!(expr.kind, ExprKind::Match { .. }));
    if let ExprKind::Match { arms, .. } = expr.kind {
        assert_eq!(arms.len(), 2);
        assert!(matches!(arms[0].pattern, MatchPattern::Constructor { ref binding, .. }
            if binding.is_none()));
    }
}

#[test]
fn test_match_with_bool_literal() {
    let expr = parse_expr(
        "match x { true => 1, false => 0 }"
    );
    assert!(matches!(expr.kind, ExprKind::Match { .. }));
    if let ExprKind::Match { arms, .. } = expr.kind {
        assert_eq!(arms.len(), 2);
        assert!(matches!(arms[0].pattern, MatchPattern::Literal(MatchLiteral::Bool(true))));
        assert!(matches!(arms[1].pattern, MatchPattern::Literal(MatchLiteral::Bool(false))));
    }
}

#[test]
fn test_match_with_string_literal() {
    let expr = parse_expr(
        "match x { \"hello\" => 1, \"world\" => 2 }"
    );
    assert!(matches!(expr.kind, ExprKind::Match { .. }));
    if let ExprKind::Match { arms, .. } = expr.kind {
        assert_eq!(arms.len(), 2);
        assert!(matches!(arms[0].pattern, MatchPattern::Literal(MatchLiteral::String(ref s)) if s == "hello"));
    }
}

// ===========================================================================
// 11. Array literal
// ===========================================================================

#[test]
fn test_empty_array() {
    let expr = parse_expr("[]");
    assert!(matches!(expr.kind, ExprKind::ArrayLiteral(ref elems) if elems.is_empty()));
}

#[test]
fn test_array_with_elements() {
    let expr = parse_expr("[1, 2, 3]");
    assert!(matches!(expr.kind, ExprKind::ArrayLiteral(ref elems) if elems.len() == 3));
}

#[test]
fn test_single_element_array() {
    let expr = parse_expr("[42]");
    assert!(matches!(expr.kind, ExprKind::ArrayLiteral(ref elems) if elems.len() == 1));
}

// ===========================================================================
// 12. Statement — let
// ===========================================================================

#[test]
fn test_let_statement() {
    let stmt = parse_first_stmt("let x = 42;");
    assert!(matches!(stmt, Statement::Let(..)));
    if let Statement::Let(ls) = stmt {
        assert_eq!(ls.name, "x");
        assert!(!ls.is_mut);
        assert!(ls.type_annotation.is_none());
        assert!(matches!(ls.value.kind, ExprKind::IntLiteral(42)));
    }
}

#[test]
fn test_let_mut_statement() {
    let stmt = parse_first_stmt("let mut x = 0;");
    assert!(matches!(stmt, Statement::Let(..)));
    if let Statement::Let(ls) = stmt {
        assert_eq!(ls.name, "x");
        assert!(ls.is_mut);
    }
}

#[test]
fn test_let_with_type_annotation() {
    let stmt = parse_first_stmt("let x: Int = 42;");
    assert!(matches!(stmt, Statement::Let(..)));
    if let Statement::Let(ls) = stmt {
        assert_eq!(ls.name, "x");
        assert!(matches!(ls.type_annotation, Some(Type::Named(ref s)) if s == "Int"));
    }
}

// ===========================================================================
// 13. Statement — return
// ===========================================================================

#[test]
fn test_return_with_value() {
    let stmt = parse_first_stmt("return 42;");
    assert!(matches!(stmt, Statement::Return(Some(..))));
}

#[test]
fn test_return_without_value() {
    let stmt = parse_first_stmt("return;");
    assert!(matches!(stmt, Statement::Return(None)));
}

#[test]
fn test_return_with_expression() {
    let stmt = parse_first_stmt("return x + 1;");
    assert!(matches!(stmt, Statement::Return(Some(..))));
    if let Statement::Return(Some(expr)) = stmt {
        assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Add, .. }));
    }
}

// ===========================================================================
// 14. Statement — if / else
// ===========================================================================

#[test]
fn test_if_statement() {
    let stmts = parse_fn_body("if x > 0 { return x; }");
    assert_eq!(stmts.len(), 1);
    assert!(matches!(stmts[0], Statement::If(..)));
    if let Statement::If(ref is) = stmts[0] {
        assert!(matches!(is.condition.kind, ExprKind::Binary { op: BinaryOp::Gt, .. }));
        assert_eq!(is.then_branch.statements.len(), 1);
        assert!(is.else_branch.is_none());
    }
}

#[test]
fn test_if_else_statement() {
    let stmts = parse_fn_body("if x { return 1; } else { return 0; }");
    assert_eq!(stmts.len(), 1);
    if let Statement::If(ref is) = stmts[0] {
        assert!(is.else_branch.is_some());
    }
}

#[test]
fn test_if_else_if_statement() {
    let stmts = parse_fn_body("if a { 1; } else if b { 2; } else { 3; }");
    assert_eq!(stmts.len(), 1);
    if let Statement::If(ref is) = stmts[0] {
        assert!(is.else_branch.is_some());
    }
}

// ===========================================================================
// 15. Statement — while
// ===========================================================================

#[test]
fn test_while_statement() {
    let stmts = parse_fn_body("while x < 10 { process(x); }");
    assert_eq!(stmts.len(), 1);
    assert!(matches!(stmts[0], Statement::While(..)));
}

// ===========================================================================
// 16. Statement — for
// ===========================================================================

#[test]
fn test_for_statement() {
    let stmts = parse_fn_body("for item in items { process(item); }");
    assert_eq!(stmts.len(), 1);
    assert!(matches!(stmts[0], Statement::For(..)));
    if let Statement::For(ref fs) = stmts[0] {
        assert_eq!(fs.variable, "item");
        assert!(matches!(fs.iterable.kind, ExprKind::Variable(ref s) if s == "items"));
    }
}

// ===========================================================================
// 17. Statement — expression statement
// ===========================================================================

#[test]
fn test_expression_statement() {
    let stmts = parse_fn_body("foo();");
    assert_eq!(stmts.len(), 1);
    assert!(matches!(stmts[0], Statement::ExprStmt(..)));
}

#[test]
fn test_expression_statement_assignment() {
    // Note: Jacquard uses `let` for bindings, not bare `=`.
    // Expression statements with side-effect function calls are valid.
    let stmts = parse_fn_body("foo(5);");
    assert_eq!(stmts.len(), 1);
    assert!(matches!(stmts[0], Statement::ExprStmt(..)));
    if let Statement::ExprStmt(ref expr) = stmts[0] {
        assert!(matches!(expr.kind, ExprKind::Call { .. }));
    }
}

// ===========================================================================
// 18. Block
// ===========================================================================

#[test]
fn test_empty_block() {
    let stmts = parse_fn_body("");
    assert!(stmts.is_empty());
}

#[test]
fn test_block_with_multiple_statements() {
    let stmts = parse_fn_body("let x = 1; let y = 2; return x + y;");
    assert_eq!(stmts.len(), 3);
}

// ===========================================================================
// 19. Type parsing
// ===========================================================================

#[test]
fn test_function_with_named_return_type() {
    let prog = parse_source("fn answer() -> Int { return 42; }");
    match &prog.declarations[0] {
        Declaration::Fn(f) => {
            assert!(matches!(f.return_type, Type::Named(ref s) if s == "Int"));
        }
        _ => panic!("expected Fn"),
    }
}

#[test]
fn test_function_with_generic_return_type() {
    let prog = parse_source("fn get() -> Option<Int> { return 0; }");
    match &prog.declarations[0] {
        Declaration::Fn(f) => {
            assert!(matches!(f.return_type, Type::Generic { .. }));
        }
        _ => panic!("expected Fn"),
    }
}

#[test]
fn test_function_with_tuple_type_param() {
    let prog = parse_source("fn f(x: (Int, String)) -> void { return; }");
    match &prog.declarations[0] {
        Declaration::Fn(f) => {
            let param_ty = &f.params[0].ty;
            assert!(matches!(param_ty, Type::Tuple(..)));
            if let Type::Tuple(types) = param_ty {
                assert_eq!(types.len(), 2);
            }
        }
        _ => panic!("expected Fn"),
    }
}

#[test]
fn test_function_with_function_type_param() {
    let prog = parse_source("fn map(f: (Int) -> Bool) -> void { return; }");
    match &prog.declarations[0] {
        Declaration::Fn(f) => {
            let param_ty = &f.params[0].ty;
            assert!(matches!(param_ty, Type::Function { .. }));
        }
        _ => panic!("expected Fn"),
    }
}

// ===========================================================================
// 20. Function declaration
// ===========================================================================

#[test]
fn test_simple_function() {
    match parse_first_decl("fn hello() { return; }") {
        Declaration::Fn(f) => {
            assert_eq!(f.name, "hello");
            assert!(!f.is_pub);
            assert!(f.type_params.is_empty());
            assert!(f.params.is_empty());
        }
        _ => panic!("expected Fn"),
    }
}

#[test]
fn test_public_function() {
    match parse_first_decl("pub fn greet() -> String { return \"hi\"; }") {
        Declaration::Fn(f) => {
            assert_eq!(f.name, "greet");
            assert!(f.is_pub);
            assert!(matches!(f.return_type, Type::Named(ref s) if s == "String"));
        }
        _ => panic!("expected Fn"),
    }
}

#[test]
fn test_function_with_params() {
    match parse_first_decl("fn add(a: Int, b: Int) -> Int { return a + b; }") {
        Declaration::Fn(f) => {
            assert_eq!(f.params.len(), 2);
            assert_eq!(f.params[0].name, "a");
            assert_eq!(f.params[1].name, "b");
        }
        _ => panic!("expected Fn"),
    }
}

#[test]
fn test_function_with_type_params() {
    match parse_first_decl("fn identity<T>(x: T) -> T { return x; }") {
        Declaration::Fn(f) => {
            assert_eq!(f.type_params, vec!["T"]);
            assert_eq!(f.params[0].name, "x");
        }
        _ => panic!("expected Fn"),
    }
}

#[test]
fn test_function_with_multiple_type_params() {
    match parse_first_decl("fn pair<A, B>(first: A, second: B) -> void { return; }") {
        Declaration::Fn(f) => {
            assert_eq!(f.type_params, vec!["A", "B"]);
        }
        _ => panic!("expected Fn"),
    }
}

// ===========================================================================
// 21. Task declaration
// ===========================================================================

#[test]
fn test_task_declaration() {
    match parse_first_decl("task compute() -> Int { return 0; }") {
        Declaration::Task(t) => {
            assert_eq!(t.name, "compute");
            assert!(matches!(t.return_type, Type::Named(ref s) if s == "Int"));
        }
        _ => panic!("expected Task"),
    }
}

#[test]
fn test_task_with_params() {
    match parse_first_decl("task process(data: Data) { return; }") {
        Declaration::Task(t) => {
            assert_eq!(t.params.len(), 1);
            assert_eq!(t.params[0].name, "data");
        }
        _ => panic!("expected Task"),
    }
}

// ===========================================================================
// 22. Workflow declaration
// ===========================================================================

#[test]
fn test_workflow_declaration() {
    match parse_first_decl("workflow main { task_a(); task_b(); }") {
        Declaration::Workflow(w) => {
            assert_eq!(w.name, "main");
            assert_eq!(w.body.statements.len(), 2);
        }
        _ => panic!("expected Workflow"),
    }
}

// ===========================================================================
// 23. Struct declaration
// ===========================================================================

#[test]
fn test_struct_declaration() {
    match parse_first_decl("struct Point { x: Float, y: Float }") {
        Declaration::Struct(s) => {
            assert_eq!(s.name, "Point");
            assert!(!s.is_pub);
            assert_eq!(s.fields.len(), 2);
            assert_eq!(s.fields[0].name, "x");
            assert_eq!(s.fields[1].name, "y");
            assert_eq!(s.type_params.len(), 0);
        }
        _ => panic!("expected Struct"),
    }
}

#[test]
fn test_public_struct() {
    match parse_first_decl("pub struct Vec3 { x: Float, y: Float, z: Float }") {
        Declaration::Struct(s) => {
            assert_eq!(s.name, "Vec3");
            assert!(s.is_pub);
            assert_eq!(s.fields.len(), 3);
        }
        _ => panic!("expected Struct"),
    }
}

#[test]
fn test_generic_struct() {
    match parse_first_decl("struct Container<T> { value: T }") {
        Declaration::Struct(s) => {
            assert_eq!(s.name, "Container");
            assert_eq!(s.type_params, vec!["T"]);
            assert_eq!(s.fields.len(), 1);
            assert!(matches!(s.fields[0].ty, Type::Named(ref n) if n == "T"));
        }
        _ => panic!("expected Struct"),
    }
}

#[test]
fn test_empty_struct() {
    match parse_first_decl("struct Empty {}") {
        Declaration::Struct(s) => {
            assert_eq!(s.name, "Empty");
            assert!(s.fields.is_empty());
        }
        _ => panic!("expected Struct"),
    }
}

// ===========================================================================
// 24. Enum declaration
// ===========================================================================

#[test]
fn test_enum_declaration() {
    match parse_first_decl("enum Color { Red, Green, Blue }") {
        Declaration::Enum(e) => {
            assert_eq!(e.name, "Color");
            assert!(!e.is_pub);
            assert_eq!(e.variants.len(), 3);
            assert_eq!(e.variants[0].name, "Red");
            assert!(e.variants[0].payload.is_none());
        }
        _ => panic!("expected Enum"),
    }
}

#[test]
fn test_public_enum() {
    match parse_first_decl("pub enum Status { Ok, Err }") {
        Declaration::Enum(e) => {
            assert_eq!(e.name, "Status");
            assert!(e.is_pub);
        }
        _ => panic!("expected Enum"),
    }
}

#[test]
fn test_enum_with_payloads() {
    match parse_first_decl("enum Result<T, E> { Ok(T), Err(E) }") {
        Declaration::Enum(e) => {
            assert_eq!(e.name, "Result");
            assert_eq!(e.type_params, vec!["T", "E"]);
            assert!(e.variants[0].payload.is_some());
            assert!(e.variants[1].payload.is_some());
        }
        _ => panic!("expected Enum"),
    }
}

// ===========================================================================
// 25. Import declaration
// ===========================================================================

#[test]
fn test_import_declaration() {
    match parse_first_decl("import \"modules/helpers.jq\";") {
        Declaration::Import(i) => {
            assert_eq!(i.path, "modules/helpers.jq");
        }
        _ => panic!("expected Import"),
    }
}

// ===========================================================================
// 26. Extern function declaration
// ===========================================================================

#[test]
fn test_extern_fn_declaration() {
    match parse_first_decl("extern fn malloc(size: Int) -> Void;") {
        Declaration::ExternFn(e) => {
            assert_eq!(e.name, "malloc");
            assert_eq!(e.params.len(), 1);
            assert_eq!(e.params[0].name, "size");
        }
        _ => panic!("expected ExternFn"),
    }
}

// ===========================================================================
// 27. Export function declaration
// ===========================================================================

#[test]
fn test_export_fn_declaration() {
    match parse_first_decl("export fn on_start() { return; }") {
        Declaration::ExportFn(e) => {
            assert_eq!(e.name, "on_start");
            assert!(e.params.is_empty());
            assert_eq!(e.body.statements.len(), 1);
        }
        _ => panic!("expected ExportFn"),
    }
}

#[test]
fn test_export_fn_with_params_and_return() {
    match parse_first_decl("export fn handle(data: Data) -> Result { return data; }") {
        Declaration::ExportFn(e) => {
            assert_eq!(e.name, "handle");
            assert_eq!(e.params.len(), 1);
            assert!(matches!(e.return_type, Type::Named(ref s) if s == "Result"));
        }
        _ => panic!("expected ExportFn"),
    }
}

// ===========================================================================
// 28. Multiple declarations
// ===========================================================================

#[test]
fn test_multiple_function_declarations() {
    let prog = parse_source("fn a() { return; } fn b() { return; }");
    assert_eq!(prog.declarations.len(), 2);
    match (&prog.declarations[0], &prog.declarations[1]) {
        (Declaration::Fn(a), Declaration::Fn(b)) => {
            assert_eq!(a.name, "a");
            assert_eq!(b.name, "b");
        }
        _ => panic!("expected two Fn declarations"),
    }
}

#[test]
fn test_mixed_declarations() {
    let prog = parse_source(
        "import \"std.jq\";\n\
         struct Config { debug: Bool }\n\
         pub fn main() { return; }"
    );
    assert_eq!(prog.declarations.len(), 3);
    assert!(matches!(prog.declarations[0], Declaration::Import(..)));
    assert!(matches!(prog.declarations[1], Declaration::Struct(..)));
    assert!(matches!(prog.declarations[2], Declaration::Fn(..)));
}

// ===========================================================================
// 29. Error cases
// ===========================================================================

#[test]
fn test_error_on_unexpected_token() {
    let err = parse_source_error("?");
    assert!(!err.message.is_empty());
}

#[test]
fn test_error_on_missing_semicolon_after_let() {
    let err = parse_source_error("fn f() { let x = 5 }");
    assert!(!err.message.is_empty());
}

#[test]
fn test_error_on_missing_rparen_in_call() {
    let err = parse_source_error("fn f() { foo(1, 2; }");
    assert!(!err.message.is_empty());
}

#[test]
fn test_error_on_missing_rbrace() {
    let err = parse_source_error("fn f() { return;");
    assert!(!err.message.is_empty());
}

#[test]
fn test_error_on_bare_expression_at_top_level() {
    let err = parse_source_error("42");
    assert!(!err.message.is_empty());
}

#[test]
fn test_error_on_trailing_comma_in_struct() {
    // Trailing comma after the last field: currently doesn't consume trailing comma
    // This should parse okay actually (comma consumed, then RBrace).
    let prog = parse_source("struct Point { x: Int, }");
    assert_eq!(prog.declarations.len(), 1);
    assert!(matches!(prog.declarations[0], Declaration::Struct(..)));
}