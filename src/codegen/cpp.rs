//! C++ code generation from the typed Jacquard AST.
//!
//! Transforms AST declarations into C++ source code, producing:
//! - A header file (`.jq.h`) with type definitions and forward declarations
//! - A source file (`.jq.cpp`) with function/task implementations
//!
//! ## Output structure
//! - **Header**: type definitions, forward declarations, inline functions
//! - **Source**: task/workflow implementations, non-inline functions

use crate::ast;
use crate::codegen::mangling;

/// Writes indented C++ code to an internal buffer.
struct CppWriter {
    output: String,
    indent_level: usize,
}

impl CppWriter {
    fn new() -> Self {
        CppWriter {
            output: String::new(),
            indent_level: 0,
        }
    }

    fn indent(&mut self) {
        self.indent_level += 1;
    }

    fn dedent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    fn writeln(&mut self, line: &str) {
        if line.is_empty() {
            self.output.push('\n');
        } else {
            for _ in 0..self.indent_level {
                self.output.push_str("    ");
            }
            self.output.push_str(line);
            self.output.push('\n');
        }
    }

    fn into_string(self) -> String {
        self.output
    }
}

// ---------------------------------------------------------------------------
// Type mapping: Jacquard → C++
// ---------------------------------------------------------------------------

fn map_type_to_cpp(ty: &ast::Type) -> String {
    match ty {
        ast::Type::Named(name) => match name.as_str() {
            "i8" => "int8_t".to_string(),
            "i16" => "int16_t".to_string(),
            "i32" => "int32_t".to_string(),
            "i64" => "int64_t".to_string(),
            "u8" => "uint8_t".to_string(),
            "u16" => "uint16_t".to_string(),
            "u32" => "uint32_t".to_string(),
            "u64" => "uint64_t".to_string(),
            "f32" => "float".to_string(),
            "f64" => "double".to_string(),
            "bool" => "bool".to_string(),
            "string" => "std::string".to_string(),
            "void" => "void".to_string(),
            other => other.to_string(),
        },
        ast::Type::Generic { name, args } => {
            let args_str: Vec<String> = args.iter().map(map_type_to_cpp).collect();
            format!("{}<{}>", name, args_str.join(", "))
        }
        ast::Type::Function { params, ret } => {
            let params_str: Vec<String> = params.iter().map(map_type_to_cpp).collect();
            format!(
                "std::function<{}({})>",
                map_type_to_cpp(ret),
                params_str.join(", ")
            )
        }
        ast::Type::Tuple(elements) => {
            let elems: Vec<String> = elements.iter().map(map_type_to_cpp).collect();
            format!("std::tuple<{}>", elems.join(", "))
        }
    }
}

// ---------------------------------------------------------------------------
// Expression emission
// ---------------------------------------------------------------------------

fn emit_expr(expr: &ast::Expr, w: &mut CppWriter) {
    match &expr.kind {
        ast::ExprKind::IntLiteral(n) => w.output.push_str(&n.to_string()),
        ast::ExprKind::FloatLiteral(n) => w.output.push_str(&n.to_string()),
        ast::ExprKind::StringLiteral(s) => {
            w.output.push('"');
            w.output.push_str(s);
            w.output.push('"');
        }
        ast::ExprKind::BoolLiteral(b) => {
            w.output.push_str(if *b { "true" } else { "false" });
        }
        ast::ExprKind::Variable(name) => w.output.push_str(name),
        ast::ExprKind::Binary { op, left, right } => {
            emit_expr(left, w);
            w.output.push(' ');
            w.output.push_str(match op {
                ast::BinaryOp::Add => "+",
                ast::BinaryOp::Sub => "-",
                ast::BinaryOp::Mul => "*",
                ast::BinaryOp::Div => "/",
                ast::BinaryOp::Mod => "%",
                ast::BinaryOp::Eq => "==",
                ast::BinaryOp::NotEq => "!=",
                ast::BinaryOp::Lt => "<",
                ast::BinaryOp::Gt => ">",
                ast::BinaryOp::LtEq => "<=",
                ast::BinaryOp::GtEq => ">=",
                ast::BinaryOp::And => "&&",
                ast::BinaryOp::Or => "||",
            });
            w.output.push(' ');
            emit_expr(right, w);
        }
        ast::ExprKind::Unary { op, operand } => {
            w.output.push_str(match op {
                ast::UnaryOp::Neg => "-",
                ast::UnaryOp::Not => "!",
            });
            emit_expr(operand, w);
        }
        ast::ExprKind::Call { callee, args } => {
            emit_expr(callee, w);
            w.output.push('(');
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    w.output.push_str(", ");
                }
                emit_expr(arg, w);
            }
            w.output.push(')');
        }
        ast::ExprKind::FieldAccess { object, field } => {
            emit_expr(object, w);
            w.output.push('.');
            w.output.push_str(field);
        }
        ast::ExprKind::Paren(inner) => {
            w.output.push('(');
            emit_expr(inner, w);
            w.output.push(')');
        }
        ast::ExprKind::Await(inner) => {
            // Await becomes nothing special in C++ — the state machine handles it
            w.output.push_str("co_await ");
            emit_expr(inner, w);
        }
        ast::ExprKind::Match { expr: match_expr, arms } => {
            w.writeln("([&]() -> auto {");
            w.indent();
            w.output.push_str("auto _match_val = ");
            emit_expr(match_expr, w);
            w.writeln(";");
            for arm in arms {
                w.writeln("// TODO: match pattern emission");
                let _ = arm;
            }
            w.writeln("return _match_val;");
            w.dedent();
            w.output.push_str("})()");
        }
        ast::ExprKind::ArrayLiteral(_elements) => {
            w.output.push_str("{}");
        }
        ast::ExprKind::MapLiteral(_entries) => {
            w.output.push_str("{}");
        }
    }
}

// ---------------------------------------------------------------------------
// Statement emission
// ---------------------------------------------------------------------------

fn emit_statement(stmt: &ast::Statement, w: &mut CppWriter) {
    match stmt {
        ast::Statement::Let(let_stmt) => {
            let cpp_type = let_stmt
                .type_annotation
                .as_ref()
                .map(|t| map_type_to_cpp(t))
                .unwrap_or_else(|| "auto".to_string());
            w.output.push_str(&format!("{} {} = ", cpp_type, let_stmt.name));
            emit_expr(&let_stmt.value, w);
            w.writeln(";");
        }
        ast::Statement::Expr(expr) | ast::Statement::ExprStmt(expr) => {
            emit_expr(expr, w);
            w.writeln(";");
        }
        ast::Statement::Return(Some(expr)) => {
            w.output.push_str("return ");
            emit_expr(expr, w);
            w.writeln(";");
        }
        ast::Statement::Return(None) => {
            w.writeln("return;");
        }
        ast::Statement::If(if_stmt) => {
            w.output.push_str("if (");
            emit_expr(&if_stmt.condition, w);
            w.writeln(") {");
            emit_block(&if_stmt.then_branch, w);
            if let Some(else_stmt) = &if_stmt.else_branch {
                w.writeln("} else {");
                emit_statement(else_stmt, w);
            }
            w.writeln("}");
        }
        ast::Statement::While(while_stmt) => {
            w.output.push_str("while (");
            emit_expr(&while_stmt.condition, w);
            w.writeln(") {");
            emit_block(&while_stmt.body, w);
            w.writeln("}");
        }
        ast::Statement::For(for_stmt) => {
            w.output.push_str(&format!(
                "for (auto& {} : ",
                for_stmt.variable
            ));
            emit_expr(&for_stmt.iterable, w);
            w.writeln(") {");
            emit_block(&for_stmt.body, w);
            w.writeln("}");
        }
        ast::Statement::Block(block) => {
            emit_block(block, w);
        }
    }
}

fn emit_block(block: &ast::Block, w: &mut CppWriter) {
    for stmt in &block.statements {
        w.indent();
        emit_statement(stmt, w);
        w.dedent();
    }
}

// ---------------------------------------------------------------------------
// Declaration emission
// ---------------------------------------------------------------------------

fn emit_struct_decl(decl: &ast::StructDecl, w: &mut CppWriter) {
    if decl.type_params.is_empty() {
        w.writeln(&format!("struct {} {{", decl.name));
    } else {
        let params = decl.type_params.join(", ");
        w.writeln(&format!("template<typename {}>", params));
        w.writeln(&format!("struct {} {{", decl.name));
    }
    w.indent();
    for field in &decl.fields {
        let cpp_type = map_type_to_cpp(&field.ty);
        w.writeln(&format!("{} {};", cpp_type, field.name));
    }
    w.dedent();
    w.writeln("};");
    w.writeln("");
}

fn emit_enum_decl(decl: &ast::EnumDecl, w: &mut CppWriter) {
    // Tagged union approach
    if decl.type_params.is_empty() {
        w.writeln(&format!("struct {} {{", decl.name));
    } else {
        let params = decl.type_params.join(", ");
        w.writeln(&format!("template<typename {}>", params));
        w.writeln(&format!("struct {} {{", decl.name));
    }
    w.indent();
    // Emit the Tag enum
    w.writeln("enum class Tag {");
    w.indent();
    for variant in &decl.variants {
        w.writeln(&format!("{},", variant.name));
    }
    w.dedent();
    w.writeln("};");
    w.writeln("Tag _tag;");
    // Emit the union of payloads
    let has_payloads = decl.variants.iter().any(|v| v.payload.is_some());
    if has_payloads {
        w.writeln("union {");
        w.indent();
        for variant in &decl.variants {
            if let Some(payload) = &variant.payload {
                let cpp_type = map_type_to_cpp(payload);
                w.writeln(&format!(
                    "{} {};",
                    cpp_type,
                    variant.name.to_lowercase()
                ));
            }
        }
        w.dedent();
        w.writeln("};");
    }
    w.dedent();
    w.writeln("};");
    w.writeln("");
}

fn emit_fn_decl(decl: &ast::FnDecl, module: &str, is_header: bool, w: &mut CppWriter) {
    let cpp_ret = map_type_to_cpp(&decl.return_type);
    let fn_name = if is_header && decl.is_pub {
        mangling::mangle_fn(module, &decl.name, &[], "")
            .trim_end_matches('_')
            .to_string()
    } else {
        decl.name.clone()
    };

    // Template parameters
    if !decl.type_params.is_empty() {
        let params = decl.type_params.join(", ");
        w.writeln(&format!("template<typename {}>", params));
    }

    // Function signature
    w.output.push_str(&format!("{} {}(", cpp_ret, fn_name));
    for (i, param) in decl.params.iter().enumerate() {
        if i > 0 {
            w.output.push_str(", ");
        }
        let param_type = map_type_to_cpp(&param.ty);
        w.output.push_str(&format!("{} {}", param_type, param.name));
    }
    w.writeln(") {");

    // Body
    w.indent();
    emit_block(&decl.body, w);
    w.dedent();

    w.writeln("}");
    w.writeln("");
}

fn emit_export_fn_decl(decl: &ast::ExportFnDecl, _module: &str, w: &mut CppWriter) {
    let cpp_ret = map_type_to_cpp(&decl.return_type);

    w.output.push_str(&format!("{} {}(", cpp_ret, decl.name));
    for (i, param) in decl.params.iter().enumerate() {
        if i > 0 {
            w.output.push_str(", ");
        }
        let param_type = map_type_to_cpp(&param.ty);
        w.output.push_str(&format!("{} {}", param_type, param.name));
    }
    w.writeln(") {");

    w.indent();
    emit_block(&decl.body, w);
    w.dedent();

    w.writeln("}");
    w.writeln("");
}

fn emit_extern_fn_decl(decl: &ast::ExternFnDecl, w: &mut CppWriter) {
    let cpp_ret = map_type_to_cpp(&decl.return_type);

    w.output.push_str(&format!("extern {} {}(", cpp_ret, decl.name));
    for (i, param) in decl.params.iter().enumerate() {
        if i > 0 {
            w.output.push_str(", ");
        }
        let param_type = map_type_to_cpp(&param.ty);
        w.output.push_str(&format!("{} {}", param_type, param.name));
    }
    w.writeln(");");
    w.writeln("");
}

fn emit_task_decl(decl: &ast::TaskDecl, _module: &str, w: &mut CppWriter) {
    // Tasks become structs with tick() method
    w.writeln(&format!("struct Task_{} {{", decl.name));
    w.indent();
    w.writeln("int _state = 0;");
    for param in &decl.params {
        let param_type = map_type_to_cpp(&param.ty);
        w.writeln(&format!("{} {};", param_type, param.name));
    }
    w.writeln("");
    w.writeln("bool tick(float dt) {");
    w.indent();
    w.writeln("switch (_state) {");
    w.indent();
    w.writeln("case 0:");
    w.indent();
    emit_block(&decl.body, w);
    w.writeln("return true; // done");
    w.dedent();
    w.dedent();
    w.writeln("}");
    w.dedent();
    w.writeln("}");
    w.dedent();
    w.writeln("};");
    w.writeln("");
}

fn emit_workflow_decl(decl: &ast::WorkflowDecl, _module: &str, w: &mut CppWriter) {
    w.writeln(&format!("struct Workflow_{} {{", decl.name));
    w.indent();
    w.writeln("int _state = 0;");
    w.writeln("");
    w.writeln("bool tick() {");
    w.indent();
    w.writeln("switch (_state) {");
    w.indent();
    w.writeln("case 0:");
    w.indent();
    emit_block(&decl.body, w);
    w.writeln("return true; // done");
    w.dedent();
    w.dedent();
    w.writeln("}");
    w.dedent();
    w.writeln("}");
    w.dedent();
    w.writeln("};");
    w.writeln("");
}

// ---------------------------------------------------------------------------
// Generate C++ output from a Program AST
// ---------------------------------------------------------------------------

/// Generate C++ header and source from a parsed Jacquard program.
///
/// Returns `(header_content, source_content)`.
pub fn generate(program: &ast::Program, module: &str) -> (String, String) {
    let mut header = CppWriter::new();
    let mut source = CppWriter::new();

    // --- Header ---
    let header_guard = format!("_JQ_{}_H_", module.to_uppercase());
    header.writeln(&format!("#pragma once"));
    header.writeln(&format!("#ifndef {}", header_guard));
    header.writeln(&format!("#define {}", header_guard));
    header.writeln("");
    header.writeln("#include <cstdint>");
    header.writeln("#include <string>");
    header.writeln("#include <functional>");
    header.writeln("#include <tuple>");
    header.writeln("#include <vector>");
    header.writeln("#include \"jacquard_runtime.h\"");
    header.writeln("");

    let namespace = format!("_jq_{}", module);
    header.writeln(&format!("namespace {} {{", namespace));
    header.writeln("");

    // --- Source ---
    let source_guard = format!("_JQ_{}_IMPL_", module.to_uppercase());
    source.writeln(&format!("#ifndef {}", source_guard));
    source.writeln(&format!("#define {}", source_guard));
    source.writeln("");
    source.writeln(&format!("#include \"{}.jq.h\"", module));
    source.writeln("");
    source.writeln(&format!("namespace {} {{", namespace));
    source.writeln("");

    for decl in &program.declarations {
        match decl {
            ast::Declaration::Fn(fn_decl) => {
                if fn_decl.is_pub {
                    // Declaration in header
                    let cpp_ret = map_type_to_cpp(&fn_decl.return_type);
                    if !fn_decl.type_params.is_empty() {
                        let params_str = fn_decl.type_params.join(", ");
                        header.writeln(&format!("template<typename {}>", params_str));
                    }
                    header.output.push_str(&format!("{} {}(", cpp_ret, fn_decl.name));
                    for (i, param) in fn_decl.params.iter().enumerate() {
                        if i > 0 {
                            header.output.push_str(", ");
                        }
                        header.output.push_str(&format!(
                            "{} {}",
                            map_type_to_cpp(&param.ty),
                            param.name
                        ));
                    }
                    header.writeln(");");
                    header.writeln("");
                    // Implementation in source
                    emit_fn_decl(fn_decl, module, false, &mut source);
                } else {
                    // Private: implementation only in source
                    emit_fn_decl(fn_decl, module, false, &mut source);
                }
            }
            ast::Declaration::Struct(struct_decl) => {
                if struct_decl.is_pub {
                    emit_struct_decl(struct_decl, &mut header);
                } else {
                    emit_struct_decl(struct_decl, &mut source);
                }
            }
            ast::Declaration::Enum(enum_decl) => {
                if enum_decl.is_pub {
                    emit_enum_decl(enum_decl, &mut header);
                } else {
                    emit_enum_decl(enum_decl, &mut source);
                }
            }
            ast::Declaration::Task(task_decl) => {
                emit_task_decl(task_decl, module, &mut source);
            }
            ast::Declaration::Workflow(workflow_decl) => {
                emit_workflow_decl(workflow_decl, module, &mut source);
            }
            ast::Declaration::Import(_) => {
                // Imports are handled by the module system
            }
            ast::Declaration::ExternFn(ext_decl) => {
                emit_extern_fn_decl(ext_decl, &mut header);
            }
            ast::Declaration::ExportFn(exp_decl) => {
                emit_export_fn_decl(exp_decl, module, &mut header);
            }
        }
    }

    // Close namespaces and guards
    header.writeln(&format!("}} // namespace {}", namespace));
    header.writeln("");
    header.writeln(&format!("#endif // {}", header_guard));

    source.writeln(&format!("}} // namespace {}", namespace));
    source.writeln("");
    source.writeln(&format!("#endif // {}", source_guard));

    (header.into_string(), source.into_string())
}