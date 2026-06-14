//! AST Visitor trait with default recursive walking implementations.
//!
//! Implementors override only the methods they care about; the defaults
//! handle walking into children for every node type.

use super::nodes::*;

/// A visitor that can walk every node in the Jacquard AST.
///
/// Every method has a **default implementation** that recurses into child
/// nodes.  Override individual methods to add behaviour at specific points
/// while still letting the default walker handle the rest of the tree.
pub trait Visitor {
    // -- Top level ---------------------------------------------------------

    fn visit_program(&mut self, program: &Program) {
        for decl in &program.declarations {
            self.visit_declaration(decl);
        }
    }

    // -- Declarations ------------------------------------------------------

    fn visit_declaration(&mut self, decl: &Declaration) {
        match decl {
            Declaration::Fn(d) => self.visit_fn_decl(d),
            Declaration::Task(d) => self.visit_task_decl(d),
            Declaration::Workflow(d) => self.visit_workflow_decl(d),
            Declaration::Struct(d) => self.visit_struct_decl(d),
            Declaration::Enum(d) => self.visit_enum_decl(d),
            Declaration::Import(d) => self.visit_import_decl(d),
            Declaration::ExternFn(d) => self.visit_extern_fn_decl(d),
            Declaration::ExportFn(d) => self.visit_export_fn_decl(d),
        }
    }

    fn visit_fn_decl(&mut self, decl: &FnDecl) {
        self.visit_block(&decl.body);
    }

    fn visit_task_decl(&mut self, decl: &TaskDecl) {
        self.visit_block(&decl.body);
    }

    fn visit_workflow_decl(&mut self, decl: &WorkflowDecl) {
        self.visit_block(&decl.body);
    }

    fn visit_struct_decl(&mut self, _decl: &StructDecl) {
        // No-op: struct fields are type-level only, no nested nodes to walk.
    }

    fn visit_enum_decl(&mut self, _decl: &EnumDecl) {
        // No-op: enum variants are type-level only, no nested nodes to walk.
    }

    fn visit_import_decl(&mut self, _decl: &ImportDecl) {
        // No-op: imports are resolved by name, no nested nodes to walk.
    }

    fn visit_extern_fn_decl(&mut self, _decl: &ExternFnDecl) {
        // No-op: extern signatures have no body.
    }

    fn visit_export_fn_decl(&mut self, decl: &ExportFnDecl) {
        self.visit_block(&decl.body);
    }

    // -- Blocks & statements -----------------------------------------------

    fn visit_block(&mut self, block: &Block) {
        for stmt in &block.statements {
            self.visit_statement(stmt);
        }
    }

    fn visit_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Let(ls) => {
                self.visit_expr(&ls.value);
            }
            Statement::Expr(e) => self.visit_expr(e),
            Statement::Return(Some(e)) => self.visit_expr(e),
            Statement::Return(None) => {
                // bare `return;` — nothing to walk
            }
            Statement::If(s) => {
                self.visit_expr(&s.condition);
                self.visit_block(&s.then_branch);
                if let Some(ref else_stmt) = s.else_branch {
                    self.visit_statement(else_stmt);
                }
            }
            Statement::While(s) => {
                self.visit_expr(&s.condition);
                self.visit_block(&s.body);
            }
            Statement::For(s) => {
                self.visit_expr(&s.iterable);
                self.visit_block(&s.body);
            }
            Statement::ExprStmt(e) => self.visit_expr(e),
        }
    }

    // -- Expressions -------------------------------------------------------

    /// Dispatch on the expression kind and recurse into children.
    ///
    /// Leaf expressions (literals, variables) are no-ops by default.
    /// Compound expressions walk their sub-expressions automatically.
    fn visit_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            // Leaf nodes — nothing to recurse into.
            ExprKind::IntLiteral(_)
            | ExprKind::FloatLiteral(_)
            | ExprKind::StringLiteral(_)
            | ExprKind::BoolLiteral(_)
            | ExprKind::Variable(_) => {}

            // Binary: walk left, then right.
            ExprKind::Binary { left, right, .. } => {
                self.visit_expr(left);
                self.visit_expr(right);
            }

            // Unary: walk the operand.
            ExprKind::Unary { operand, .. } => {
                self.visit_expr(operand);
            }

            // Call: walk the callee, then every argument.
            ExprKind::Call { callee, args } => {
                self.visit_expr(callee);
                for arg in args {
                    self.visit_expr(arg);
                }
            }

            // Field access: walk the receiver object.
            ExprKind::FieldAccess { object, .. } => {
                self.visit_expr(object);
            }

            // Await: walk the inner expression.
            ExprKind::Await(inner) => {
                self.visit_expr(inner);
            }

            // Match: walk the scrutinee, then every arm body.
            ExprKind::Match { expr: scrutinee, arms } => {
                self.visit_expr(scrutinee);
                for arm in arms {
                    self.visit_expr(&arm.body);
                }
            }

            // Paren: walk the inner expression.
            ExprKind::Paren(inner) => {
                self.visit_expr(inner);
            }

            // Array literal: walk every element.
            ExprKind::ArrayLiteral(elements) => {
                for elem in elements {
                    self.visit_expr(elem);
                }
            }

            // Map literal: walk every key and value.
            ExprKind::MapLiteral(entries) => {
                for (key, value) in entries {
                    self.visit_expr(key);
                    self.visit_expr(value);
                }
            }
        }
    }
}