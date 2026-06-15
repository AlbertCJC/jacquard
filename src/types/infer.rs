//! Bidirectional type inference for Jacquard.
//!
//! Implements bottom-up inference (`infer_expr`) and top-down checking
//! (`check_expr`). Unification is local to each function body, so type
//! errors localize to the function boundary.
//!
//! See `docs/superpowers/specs/2026-06-14-compiler-design.md` §1.1

use crate::ast;
use crate::types::ir::{Type, TypeVarTable, TypeError};

/// Symbol table mapping names to their inferred or declared types.
#[derive(Debug, Clone)]
pub struct TypeEnv {
    /// Variable bindings: `x: i32`, `y: bool`, etc.
    vars: Vec<(String, Type)>,
    /// Known type names (structs, enums). Maps name → (type_params, definition).
    types: Vec<(String, Vec<String>)>,
}

impl TypeEnv {
    pub fn new() -> Self {
        TypeEnv {
            vars: Vec::new(),
            types: Vec::new(),
        }
    }

    /// Insert a variable binding.
    pub fn insert(&mut self, name: String, ty: Type) {
        // Remove any existing binding with this name (shadowing).
        self.vars.retain(|(n, _)| n != &name);
        self.vars.push((name, ty));
    }

    /// Look up a variable's type. Returns `None` if not in scope.
    pub fn lookup(&self, name: &str) -> Option<&Type> {
        self.vars.iter().rev().find_map(|(n, ty)| {
            if n == name { Some(ty) } else { None }
        })
    }

    /// Register a type name (struct/enum declaration).
    pub fn register_type(&mut self, name: String, type_params: Vec<String>) {
        self.types.push((name, type_params));
    }

    /// Check if a name is a known type.
    pub fn is_type_name(&self, name: &str) -> bool {
        self.types.iter().any(|(n, _)| n == name)
    }
}

// ---------------------------------------------------------------------------
// AST Type → IR Type conversion
// ---------------------------------------------------------------------------

/// Convert an AST type annotation into an IR type.
///
/// Resolves named types against the type environment. For example,
/// `Type::Named("i32")` becomes `Type::I32`, while `Type::Named("MyStruct")`
/// resolves to `Type::Named("MyStruct")` if it was registered.
fn ast_type_to_ir(
    ty: &ast::Type,
    env: &TypeEnv,
    _table: &mut TypeVarTable,
) -> Result<Type, TypeError> {
    match ty {
        ast::Type::Named(name) => {
            // Check for built-in primitives first.
            match name.as_str() {
                "i8" => Ok(Type::I8),
                "i16" => Ok(Type::I16),
                "i32" => Ok(Type::I32),
                "i64" => Ok(Type::I64),
                "u8" => Ok(Type::U8),
                "u16" => Ok(Type::U16),
                "u32" => Ok(Type::U32),
                "u64" => Ok(Type::U64),
                "f32" => Ok(Type::F32),
                "f64" => Ok(Type::F64),
                "bool" => Ok(Type::Bool),
                "string" => Ok(Type::String),
                "void" => Ok(Type::Void),
                // User-defined type
                _ => Ok(Type::Named(name.clone())),
            }
        }
        ast::Type::Generic { name, args } => {
            let ir_args: Result<Vec<Type>, TypeError> = args
                .iter()
                .map(|a| ast_type_to_ir(a, env, _table))
                .collect();
            Ok(Type::Generic(name.clone(), ir_args?))
        }
        ast::Type::Function { params, ret } => {
            let ir_params: Result<Vec<Type>, TypeError> = params
                .iter()
                .map(|p| ast_type_to_ir(p, env, _table))
                .collect();
            let ir_ret = ast_type_to_ir(ret, env, _table)?;
            Ok(Type::Function(ir_params?, Box::new(ir_ret)))
        }
        ast::Type::Tuple(elements) => {
            let ir_elements: Result<Vec<Type>, TypeError> = elements
                .iter()
                .map(|e| ast_type_to_ir(e, env, _table))
                .collect();
            Ok(Type::Tuple(ir_elements?))
        }
    }
}

// ---------------------------------------------------------------------------
// Expression inference (bottom-up)
// ---------------------------------------------------------------------------

/// Infer the type of an expression (bottom-up mode).
///
/// Returns the inferred type, or a `TypeError` if inference fails
/// (e.g., undeclared variable, type mismatch in sub-expression).
pub fn infer_expr(
    expr: &ast::Expr,
    env: &mut TypeEnv,
    table: &mut TypeVarTable,
) -> Result<Type, TypeError> {
    let ty = match &expr.kind {
        ast::ExprKind::IntLiteral(_) => Type::I32,
        ast::ExprKind::FloatLiteral(_) => Type::F64,
        ast::ExprKind::StringLiteral(_) => Type::String,
        ast::ExprKind::BoolLiteral(_) => Type::Bool,

        ast::ExprKind::Variable(name) => {
            match env.lookup(name) {
                Some(ty) => ty.clone(),
                None => {
                    return Err(TypeError {
                        message: format!("use of undeclared variable `{}`", name),
                        span: Some((expr.span.start, expr.span.end)),
                    });
                }
            }
        }

        ast::ExprKind::Binary { op, left, right } => {
            let left_ty = infer_expr(left, env, table)?;
            let right_ty = infer_expr(right, env, table)?;
            use ast::BinaryOp::*;
            match op {
                // Arithmetic: both operands must match, result is same type
                Add | Sub | Mul | Div | Mod => {
                    table.unify(&left_ty, &right_ty)?;
                    left_ty
                }
                // Comparison: both operands must match, result is always Bool
                Eq | NotEq | Lt | Gt | LtEq | GtEq => {
                    table.unify(&left_ty, &right_ty)?;
                    Type::Bool
                }
                // Logical: both operands must be Bool, result is Bool
                And | Or => {
                    table.unify(&left_ty, &Type::Bool)?;
                    table.unify(&right_ty, &Type::Bool)?;
                    Type::Bool
                }
            }
        }

        ast::ExprKind::Unary { op, operand } => {
            let operand_ty = infer_expr(operand, env, table)?;
            match op {
                ast::UnaryOp::Neg => {
                    // Arithmetic negation: operand must be numeric
                    table.unify(&operand_ty, &Type::I32)?;
                    Type::I32
                }
                ast::UnaryOp::Not => {
                    // Logical not: operand must be Bool
                    table.unify(&operand_ty, &Type::Bool)?;
                    Type::Bool
                }
            }
        }

        ast::ExprKind::Paren(inner) => infer_expr(inner, env, table)?,

        ast::ExprKind::Call { callee, args } => {
            let fn_ty = infer_expr(callee, env, table)?;
            // Create expected function type from arguments + fresh return var
            let arg_types: Vec<Type> = args
                .iter()
                .map(|a| infer_expr(a, env, table))
                .collect::<Result<Vec<_>, _>>()?;
            let ret_var = table.new_var();
            let expected_fn = Type::Function(arg_types, Box::new(Type::Var(ret_var)));
            table.unify(&fn_ty, &expected_fn)?;
            // Resolve the return type variable
            table.resolve_type(ret_var).unwrap_or(Type::Error)
        }

        ast::ExprKind::FieldAccess { object, field } => {
            let _obj_ty = infer_expr(object, env, table)?;
            // Stub: field access produces a fresh variable. Full struct field
            // lookup will be implemented when we track struct definitions
            // with their field types.
            let _field_name = field.clone();
            let var = table.new_var();
            Type::Var(var)
        }

        ast::ExprKind::Await(inner) => {
            // await expr: infer the inner expression
            let inner_ty = infer_expr(inner, env, table)?;
            inner_ty
        }

        ast::ExprKind::Match { expr: match_expr, arms } => {
            let _match_ty = infer_expr(match_expr, env, table)?;
            // Infer each arm body and unify them
            let result_var = table.new_var();
            for arm in arms {
                let arm_ty = infer_expr(&arm.body, env, table)?;
                table.unify(&arm_ty, &Type::Var(result_var))?;
            }
            table.resolve_type(result_var).unwrap_or(Type::Error)
        }

        ast::ExprKind::ArrayLiteral(elements) => {
            if elements.is_empty() {
                // Empty array: fresh type variable (will be constrained by usage)
                let var = table.new_var();
                Type::Generic("Array".to_string(), vec![Type::Var(var)])
            } else {
                // Infer element type from first element, then unify rest
                let elem_ty = infer_expr(&elements[0], env, table)?;
                for elem in &elements[1..] {
                    let other_ty = infer_expr(elem, env, table)?;
                    table.unify(&elem_ty, &other_ty)?;
                }
                Type::Generic("Array".to_string(), vec![elem_ty])
            }
        }

        ast::ExprKind::MapLiteral(_entries) => {
            // Stub: Map<K, V> with fresh type variables
            let key_var = table.new_var();
            let val_var = table.new_var();
            Type::Generic(
                "Map".to_string(),
                vec![Type::Var(key_var), Type::Var(val_var)],
            )
        }
    };

    Ok(ty)
}

// ---------------------------------------------------------------------------
// Expression checking (top-down)
// ---------------------------------------------------------------------------

/// Check that an expression has the expected type (top-down mode).
///
/// Infers the expression's type, then unifies it with the expected type.
pub fn check_expr(
    expr: &ast::Expr,
    expected: &Type,
    env: &mut TypeEnv,
    table: &mut TypeVarTable,
) -> Result<(), TypeError> {
    let inferred = infer_expr(expr, env, table)?;
    table.unify(&inferred, expected)
}

// ---------------------------------------------------------------------------
// Statement checking
// ---------------------------------------------------------------------------

fn check_statement(
    stmt: &ast::Statement,
    expected_return: &Type,
    env: &mut TypeEnv,
    table: &mut TypeVarTable,
) -> Result<bool, TypeError> {
    // Returns true if the statement definitely returns (i.e., this path has
    // a return statement). Used for basic reachability checking.
    match stmt {
        ast::Statement::Let(let_stmt) => {
            let value_ty = infer_expr(&let_stmt.value, env, table)?;
            if let Some(ann) = &let_stmt.type_annotation {
                let expected_ty = ast_type_to_ir(ann, env, table)?;
                table.unify(&value_ty, &expected_ty)?;
            }
            env.insert(let_stmt.name.clone(), value_ty);
            Ok(false)
        }

        ast::Statement::Expr(expr) | ast::Statement::ExprStmt(expr) => {
            let _ = infer_expr(expr, env, table)?;
            Ok(false)
        }

        ast::Statement::Return(Some(expr)) => {
            check_expr(expr, expected_return, env, table)?;
            Ok(true)
        }

        ast::Statement::Return(None) => {
            // return without value — must be in a void function
            match expected_return {
                Type::Void => Ok(true),
                _ => Err(TypeError {
                    message: format!(
                        "return without value in function returning {:?}",
                        expected_return
                    ),
                    span: None,
                }),
            }
        }

        ast::Statement::If(if_stmt) => {
            // Condition must be Bool
            check_expr(&if_stmt.condition, &Type::Bool, env, table)?;
            let _then_returns = check_block(&if_stmt.then_branch, expected_return, env, table)?;
            if let Some(else_stmt) = &if_stmt.else_branch {
                let _else_returns = check_statement(else_stmt, expected_return, env, table)?;
            }
            Ok(false)
        }

        ast::Statement::While(while_stmt) => {
            check_expr(&while_stmt.condition, &Type::Bool, env, table)?;
            let _ = check_block(&while_stmt.body, expected_return, env, table)?;
            Ok(false)
        }

        ast::Statement::For(for_stmt) => {
            let _iter_ty = infer_expr(&for_stmt.iterable, env, table)?;
            // The loop variable gets the element type of the iterable.
            // For now, give it a fresh variable.
            let elem_var = table.new_var();
            env.insert(for_stmt.variable.clone(), Type::Var(elem_var));
            let _ = check_block(&for_stmt.body, expected_return, env, table)?;
            Ok(false)
        }

        ast::Statement::Block(block) => {
            let _ = check_block(block, expected_return, env, table)?;
            Ok(false)
        }
    }
}

fn check_block(
    block: &ast::Block,
    expected_return: &Type,
    env: &mut TypeEnv,
    table: &mut TypeVarTable,
) -> Result<bool, TypeError> {
    let mut has_return = false;
    for stmt in &block.statements {
        has_return = check_statement(stmt, expected_return, env, table)? || has_return;
    }
    Ok(has_return)
}

// ---------------------------------------------------------------------------
// Declaration checking
// ---------------------------------------------------------------------------

fn check_fn_decl(
    decl: &ast::FnDecl,
    env: &mut TypeEnv,
    table: &mut TypeVarTable,
) -> Result<(), TypeError> {
    // Convert parameter types to IR
    let param_types: Vec<Type> = decl
        .params
        .iter()
        .map(|p| ast_type_to_ir(&p.ty, env, table))
        .collect::<Result<Vec<_>, _>>()?;

    // Convert return type
    let return_type = ast_type_to_ir(&decl.return_type, env, table)?;

    // Insert parameters into environment
    let mut body_env = env.clone();
    for (param, ty) in decl.params.iter().zip(param_types.iter()) {
        body_env.insert(param.name.clone(), ty.clone());
    }

    // Check the function body
    check_block(&decl.body, &return_type, &mut body_env, table)?;

    Ok(())
}

fn check_task_decl(
    decl: &ast::TaskDecl,
    env: &mut TypeEnv,
    table: &mut TypeVarTable,
) -> Result<(), TypeError> {
    let param_types: Vec<Type> = decl
        .params
        .iter()
        .map(|p| ast_type_to_ir(&p.ty, env, table))
        .collect::<Result<Vec<_>, _>>()?;

    let return_type = ast_type_to_ir(&decl.return_type, env, table)?;

    let mut body_env = env.clone();
    for (param, ty) in decl.params.iter().zip(param_types.iter()) {
        body_env.insert(param.name.clone(), ty.clone());
    }

    check_block(&decl.body, &return_type, &mut body_env, table)?;
    Ok(())
}

fn check_workflow_decl(
    decl: &ast::WorkflowDecl,
    env: &mut TypeEnv,
    table: &mut TypeVarTable,
) -> Result<(), TypeError> {
    let return_type = Type::Void;
    let mut body_env = env.clone();
    check_block(&decl.body, &return_type, &mut body_env, table)?;
    Ok(())
}

fn check_struct_decl(
    decl: &ast::StructDecl,
    env: &mut TypeEnv,
) {
    // Register the struct type name so it can be used in type annotations.
    env.register_type(decl.name.clone(), decl.type_params.clone());
}

fn check_enum_decl(
    decl: &ast::EnumDecl,
    env: &mut TypeEnv,
) {
    env.register_type(decl.name.clone(), decl.type_params.clone());
}

fn check_extern_fn_decl(
    decl: &ast::ExternFnDecl,
    env: &mut TypeEnv,
    table: &mut TypeVarTable,
) -> Result<(), TypeError> {
    let param_types: Vec<Type> = decl
        .params
        .iter()
        .map(|p| ast_type_to_ir(&p.ty, env, table))
        .collect::<Result<Vec<_>, _>>()?;
    let return_type = ast_type_to_ir(&decl.return_type, env, table)?;
    // Register the function type in the environment
    env.insert(decl.name.clone(), Type::Function(param_types, Box::new(return_type)));
    Ok(())
}

fn check_export_fn_decl(
    decl: &ast::ExportFnDecl,
    env: &mut TypeEnv,
    table: &mut TypeVarTable,
) -> Result<(), TypeError> {
    let param_types: Vec<Type> = decl
        .params
        .iter()
        .map(|p| ast_type_to_ir(&p.ty, env, table))
        .collect::<Result<Vec<_>, _>>()?;
    let return_type = ast_type_to_ir(&decl.return_type, env, table)?;

    let mut body_env = env.clone();
    for (param, ty) in decl.params.iter().zip(param_types.iter()) {
        body_env.insert(param.name.clone(), ty.clone());
    }

    check_block(&decl.body, &return_type, &mut body_env, table)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Program checking (entry point)
// ---------------------------------------------------------------------------

/// Type-check an entire program.
///
/// Walks all declarations in order, performing two passes:
/// 1. Register type names (structs, enums) so they're available for
///    forward references in function signatures.
/// 2. Type-check each declaration body.
pub fn check_program(program: &ast::Program) -> Result<(), TypeError> {
    let mut env = TypeEnv::new();

    // Pass 1: register all type names (struct, enum)
    for decl in &program.declarations {
        match decl {
            ast::Declaration::Struct(s) => check_struct_decl(s, &mut env),
            ast::Declaration::Enum(e) => check_enum_decl(e, &mut env),
            _ => {}
        }
    }

    // Pass 2: type-check each declaration body
    for decl in &program.declarations {
        let mut table = TypeVarTable::new();
        match decl {
            ast::Declaration::Fn(f) => check_fn_decl(f, &mut env, &mut table)?,
            ast::Declaration::Task(t) => check_task_decl(t, &mut env, &mut table)?,
            ast::Declaration::Workflow(w) => check_workflow_decl(w, &mut env, &mut table)?,
            ast::Declaration::Struct(_) | ast::Declaration::Enum(_) => {
                // Already registered in pass 1
            }
            ast::Declaration::Import(_) => {
                // Imports are resolved separately (module system phase)
            }
            ast::Declaration::ExternFn(ext) => {
                check_extern_fn_decl(ext, &mut env, &mut table)?;
            }
            ast::Declaration::ExportFn(exp) => {
                check_export_fn_decl(exp, &mut env, &mut table)?;
            }
        }
    }

    Ok(())
}