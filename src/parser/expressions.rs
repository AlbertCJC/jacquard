//! Parser state and expression/declaration/statement parsing for Jacquard.
//!
//! This module contains:
//! - `Precedence` — operator precedence levels for Pratt parsing.
//! - `ParserState` — token-cursor wrapper with lookahead helpers.
//! - Pratt expression parser (`parse_expr`, `parse_prefix`).
//! - Declaration parsers (fn, task, workflow, struct, enum, import, extern, export).
//! - Statement parsers (let, if, while, for, return, expression-statement).
//! - Type parser.

use crate::ast::*;
use crate::lexer::{Span, Token, TokenKind};
use crate::parser::error::ParseError;

// ============================================================================
// Precedence
// ============================================================================

/// Operator precedence levels, ordered lowest-to-highest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    Lowest = 0,
    LogicalOr = 1,  // ||
    LogicalAnd = 2, // &&
    Equality = 3,   // == !=
    Comparison = 4, // < > <= >=
    Sum = 5,        // + -
    Product = 6,    // * / %
    Prefix = 7,     // unary -expr, !expr
    Call = 8,       // expr(), expr.field
}

impl Precedence {
    /// Return the next-higher precedence level.
    pub fn next(self) -> Self {
        match self {
            Precedence::Lowest => Precedence::LogicalOr,
            Precedence::LogicalOr => Precedence::LogicalAnd,
            Precedence::LogicalAnd => Precedence::Equality,
            Precedence::Equality => Precedence::Comparison,
            Precedence::Comparison => Precedence::Sum,
            Precedence::Sum => Precedence::Product,
            Precedence::Product => Precedence::Prefix,
            Precedence::Prefix => Precedence::Call,
            Precedence::Call => Precedence::Call, // terminal — no higher level
        }
    }

    /// Map a binary-operator token kind to its precedence, if applicable.
    pub fn from_binary_op(kind: TokenKind) -> Option<Precedence> {
        match kind {
            TokenKind::Or => Some(Precedence::LogicalOr),
            TokenKind::And => Some(Precedence::LogicalAnd),
            TokenKind::EqEq | TokenKind::NotEq => Some(Precedence::Equality),
            TokenKind::LAngle | TokenKind::RAngle | TokenKind::LtEq | TokenKind::GtEq => {
                Some(Precedence::Comparison)
            }
            TokenKind::Plus | TokenKind::Minus => Some(Precedence::Sum),
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Some(Precedence::Product),
            _ => None,
        }
    }

    /// Map a binary-operator token kind to the corresponding `BinaryOp`.
    pub fn binary_op(kind: TokenKind) -> Option<BinaryOp> {
        match kind {
            TokenKind::Plus => Some(BinaryOp::Add),
            TokenKind::Minus => Some(BinaryOp::Sub),
            TokenKind::Star => Some(BinaryOp::Mul),
            TokenKind::Slash => Some(BinaryOp::Div),
            TokenKind::Percent => Some(BinaryOp::Mod),
            TokenKind::EqEq => Some(BinaryOp::Eq),
            TokenKind::NotEq => Some(BinaryOp::NotEq),
            TokenKind::LAngle => Some(BinaryOp::Lt),
            TokenKind::RAngle => Some(BinaryOp::Gt),
            TokenKind::LtEq => Some(BinaryOp::LtEq),
            TokenKind::GtEq => Some(BinaryOp::GtEq),
            TokenKind::And => Some(BinaryOp::And),
            TokenKind::Or => Some(BinaryOp::Or),
            _ => None,
        }
    }
}

// ============================================================================
// ParserState
// ============================================================================

/// Token-cursor state shared by all parsing methods.
pub struct ParserState<'a> {
    tokens: &'a [Token],
    position: usize,
}

// ---------------------------------------------------------------------------
// Basic cursor methods
// ---------------------------------------------------------------------------

impl<'a> ParserState<'a> {
    /// Create a new parser positioned at the first token.
    pub fn new(tokens: &'a [Token]) -> Self {
        ParserState {
            tokens,
            position: 0,
        }
    }

    /// Peek at the current token without consuming it.
    pub fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    /// Peek at the current token kind.
    pub fn peek_kind(&self) -> Option<TokenKind> {
        self.peek().map(|t| t.kind)
    }

    /// Peek at the current token's lexeme.
    pub fn peek_lexeme(&self) -> Option<&str> {
        self.peek().map(|t| t.lexeme.as_str())
    }

    /// Advance one token and return it.
    pub fn advance(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.position);
        if token.is_some() {
            self.position += 1;
        }
        token
    }

    /// Consume the current token if its kind matches `kind`, returning `Some`,
    /// or return `None` without advancing.
    pub fn consume_if(&mut self, kind: TokenKind) -> Option<&Token> {
        if self.check(kind) {
            self.advance()
        } else {
            None
        }
    }

    /// Require the current token to have kind `kind`, advancing on success.
    pub fn expect(&mut self, kind: TokenKind) -> Result<&Token, ParseError> {
        match self.peek() {
            Some(token) if token.kind == kind => Ok(self.advance().unwrap()),
            Some(token) => Err(ParseError::expected(
                vec![kind],
                Some(token.kind),
                token.span.clone(),
            )),
            None => Err(ParseError::expected(
                vec![kind],
                None,
                Span::new(0, 0, 0, 0),
            )),
        }
    }

    /// Return the span of the current token, or a zero-span if at EOF.
    pub fn current_span(&self) -> Span {
        self.peek()
            .map(|t| t.span.clone())
            .unwrap_or_else(|| Span::new(0, 0, 0, 0))
    }

    /// True when the token stream is exhausted (EOF).
    pub fn is_at_end(&self) -> bool {
        matches!(self.peek_kind(), None | Some(TokenKind::Eof))
    }

    /// True when the current token has kind `kind`.
    pub fn check(&self, kind: TokenKind) -> bool {
        self.peek_kind() == Some(kind)
    }

    /// True when the current token's lexeme equals `lexeme`.
    pub fn check_lexeme(&self, lexeme: &str) -> bool {
        self.peek_lexeme() == Some(lexeme)
    }
}

// ============================================================================
// Program & Declarations
// ============================================================================

impl<'a> ParserState<'a> {
    /// Top-level entry point: parse all tokens into a `Program` AST.
    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let start_span = self.current_span();
        let mut declarations = Vec::new();

        while !self.is_at_end() {
            declarations.push(self.parse_declaration()?);
        }

        let end_span = self.current_span();
        let span = Span::new(
            start_span.start,
            end_span.end,
            start_span.line,
            start_span.col,
        );

        Ok(Program {
            declarations,
            span,
        })
    }

    /// Parse a single top-level declaration.
    pub fn parse_declaration(&mut self) -> Result<Declaration, ParseError> {
        let kw_token = self.peek().ok_or_else(|| {
            ParseError::new("expected declaration", self.current_span())
        })?;
        let kw_span = kw_token.span.clone();
        let kw_kind = kw_token.kind;

        match kw_kind {
            // ---- visibility prefix -----------------------------------------
            TokenKind::Pub => {
                self.advance();
                let next = self.peek().ok_or_else(|| {
                    ParseError::new(
                        "expected fn, struct, or enum after pub",
                        self.current_span(),
                    )
                })?;
                match next.kind {
                    TokenKind::Fn => {
                        self.advance();
                        Ok(Declaration::Fn(self.parse_fn_decl(true, kw_span)?))
                    }
                    TokenKind::Struct => {
                        self.advance();
                        Ok(Declaration::Struct(self.parse_struct_decl(true, kw_span)?))
                    }
                    TokenKind::Enum => {
                        self.advance();
                        Ok(Declaration::Enum(self.parse_enum_decl(true, kw_span)?))
                    }
                    _ => Err(ParseError::expected(
                        vec![TokenKind::Fn, TokenKind::Struct, TokenKind::Enum],
                        Some(next.kind),
                        next.span.clone(),
                    )),
                }
            }

            // ---- plain keywords --------------------------------------------
            TokenKind::Fn => {
                self.advance();
                Ok(Declaration::Fn(self.parse_fn_decl(false, kw_span)?))
            }
            TokenKind::Task => {
                self.advance();
                Ok(Declaration::Task(self.parse_task_decl(kw_span)?))
            }
            TokenKind::Workflow => {
                self.advance();
                Ok(Declaration::Workflow(self.parse_workflow_decl(kw_span)?))
            }
            TokenKind::Struct => {
                self.advance();
                Ok(Declaration::Struct(self.parse_struct_decl(false, kw_span)?))
            }
            TokenKind::Enum => {
                self.advance();
                Ok(Declaration::Enum(self.parse_enum_decl(false, kw_span)?))
            }
            TokenKind::Import => {
                self.advance();
                Ok(Declaration::Import(self.parse_import_decl()?))
            }
            TokenKind::Extern => {
                self.advance();
                Ok(Declaration::ExternFn(self.parse_extern_fn_decl()?))
            }
            TokenKind::Export => {
                self.advance();
                Ok(Declaration::ExportFn(self.parse_export_fn_decl()?))
            }

            _ => Err(ParseError::expected(
                vec![
                    TokenKind::Pub,
                    TokenKind::Fn,
                    TokenKind::Task,
                    TokenKind::Workflow,
                    TokenKind::Struct,
                    TokenKind::Enum,
                    TokenKind::Import,
                    TokenKind::Extern,
                    TokenKind::Export,
                ],
                Some(kw_kind),
                kw_span,
            )),
        }
    }
}

// ============================================================================
// Declaration sub-parsers
// ============================================================================

impl<'a> ParserState<'a> {
    /// Parse `fn name<T...>(params) -> return_type { body }`.
    /// The `fn` keyword has already been consumed; `kw_span` is its span.
    fn parse_fn_decl(&mut self, is_pub: bool, kw_span: Span) -> Result<FnDecl, ParseError> {
        let name = self.expect(TokenKind::Identifier)?.lexeme.clone();

        let type_params = self.parse_optional_type_params()?;

        self.expect(TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(TokenKind::RParen)?;

        let return_type = self.parse_optional_return_type();

        let body = self.parse_block()?;

        let span = Span::new(
            kw_span.start,
            body.span.end,
            kw_span.line,
            kw_span.col,
        );

        Ok(FnDecl {
            name,
            type_params,
            params,
            return_type,
            body,
            is_pub,
            span,
        })
    }

    /// Parse `task name(params) -> return_type { body }`.
    fn parse_task_decl(&mut self, kw_span: Span) -> Result<TaskDecl, ParseError> {
        let name = self.expect(TokenKind::Identifier)?.lexeme.clone();

        self.expect(TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(TokenKind::RParen)?;

        let return_type = self.parse_optional_return_type();

        let body = self.parse_block()?;

        let span = Span::new(
            kw_span.start,
            body.span.end,
            kw_span.line,
            kw_span.col,
        );

        Ok(TaskDecl {
            name,
            params,
            return_type,
            body,
            span,
        })
    }

    /// Parse `workflow name { body }` (no params, always void return).
    fn parse_workflow_decl(&mut self, kw_span: Span) -> Result<WorkflowDecl, ParseError> {
        let name = self.expect(TokenKind::Identifier)?.lexeme.clone();
        let body = self.parse_block()?;

        let span = Span::new(
            kw_span.start,
            body.span.end,
            kw_span.line,
            kw_span.col,
        );

        Ok(WorkflowDecl { name, body, span })
    }

    /// Parse `struct name<T...> { fields }`.
    fn parse_struct_decl(&mut self, is_pub: bool, kw_span: Span) -> Result<StructDecl, ParseError> {
        let name = self.expect(TokenKind::Identifier)?.lexeme.clone();

        let type_params = self.parse_optional_type_params()?;

        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();

        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            let (fname, fspan) = {
                let t = self.expect(TokenKind::Identifier)?;
                (t.lexeme.clone(), t.span.clone())
            };
            self.expect(TokenKind::Colon)?;
            let ty = self.parse_type()?;
            self.consume_if(TokenKind::Comma);

            fields.push(StructField { name: fname, ty, span: fspan });
        }

        let rbrace = self.expect(TokenKind::RBrace)?;

        let span = Span::new(
            kw_span.start,
            rbrace.span.end,
            kw_span.line,
            kw_span.col,
        );

        Ok(StructDecl {
            name,
            type_params,
            fields,
            is_pub,
            span,
        })
    }

    /// Parse `enum name<T...> { variants }`.
    fn parse_enum_decl(&mut self, is_pub: bool, kw_span: Span) -> Result<EnumDecl, ParseError> {
        let name_token = self.expect(TokenKind::Identifier)?;
        let name = name_token.lexeme.clone();

        let type_params = self.parse_optional_type_params()?;

        self.expect(TokenKind::LBrace)?;
        let mut variants = Vec::new();

        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            let v_tok = self.expect(TokenKind::Identifier)?;
            let vname = v_tok.lexeme.clone();
            let vspan = v_tok.span.clone();

            let payload = if self.check(TokenKind::LParen) {
                self.advance();
                let ty = self.parse_type()?;
                self.expect(TokenKind::RParen)?;
                Some(ty)
            } else {
                None
            };

            self.consume_if(TokenKind::Comma);

            variants.push(EnumVariant {
                name: vname,
                payload,
                span: vspan,
            });
        }

        let rbrace = self.expect(TokenKind::RBrace)?;

        let span = Span::new(
            kw_span.start,
            rbrace.span.end,
            kw_span.line,
            kw_span.col,
        );

        Ok(EnumDecl {
            name,
            type_params,
            variants,
            is_pub,
            span,
        })
    }

    /// Parse `import "path";`
    fn parse_import_decl(&mut self) -> Result<ImportDecl, ParseError> {
        let (path, path_span) = {
            let t = self.expect(TokenKind::StringLiteral)?;
            // Strip surrounding quotes.
            (t.lexeme[1..t.lexeme.len() - 1].to_string(), t.span.clone())
        };
        self.expect(TokenKind::Semicolon)?;

        Ok(ImportDecl {
            path,
            span: path_span,
        })
    }

    /// Parse `extern fn name(params) -> return_type;`
    fn parse_extern_fn_decl(&mut self) -> Result<ExternFnDecl, ParseError> {
        self.expect(TokenKind::Fn)?;
        let (name, name_span) = {
            let t = self.expect(TokenKind::Identifier)?;
            (t.lexeme.clone(), t.span.clone())
        };

        self.expect(TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(TokenKind::RParen)?;

        let return_type = self.parse_optional_return_type();

        self.expect(TokenKind::Semicolon)?;

        Ok(ExternFnDecl {
            name,
            params,
            return_type,
            span: name_span,
        })
    }

    /// Parse `export fn name(params) -> return_type { body }`
    fn parse_export_fn_decl(&mut self) -> Result<ExportFnDecl, ParseError> {
        self.expect(TokenKind::Fn)?;
        let (name, name_span) = {
            let t = self.expect(TokenKind::Identifier)?;
            (t.lexeme.clone(), t.span.clone())
        };

        self.expect(TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(TokenKind::RParen)?;

        let return_type = self.parse_optional_return_type();

        let body = self.parse_block()?;

        let span = Span::new(
            name_span.start,
            body.span.end,
            name_span.line,
            name_span.col,
        );

        Ok(ExportFnDecl {
            name,
            params,
            return_type,
            body,
            span,
        })
    }

    // ------------------------------------------------------------------
    // Shared declaration helpers
    // ------------------------------------------------------------------

    /// Parse optional type parameters: `<T, U, ...>`.
    fn parse_optional_type_params(&mut self) -> Result<Vec<String>, ParseError> {
        let mut type_params = Vec::new();
        if self.check(TokenKind::LAngle) {
            self.advance();
            loop {
                let name = {
                    let t = self.expect(TokenKind::Identifier)?;
                    t.lexeme.clone()
                };
                type_params.push(name);
                if self.check(TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect(TokenKind::RAngle)?;
        }
        Ok(type_params)
    }

    /// Parse optional return type: `-> type`. Returns `Named("void")` if absent.
    fn parse_optional_return_type(&mut self) -> Type {
        if self.check(TokenKind::Arrow) {
            self.advance();
            self.parse_type().unwrap_or(Type::Named("void".to_string()))
        } else {
            Type::Named("void".to_string())
        }
    }
}

// ============================================================================
// Blocks & Statements
// ============================================================================

impl<'a> ParserState<'a> {
    /// Parse `{ statements }`. The opening brace is consumed here.
    pub fn parse_block(&mut self) -> Result<Block, ParseError> {
        let lbrace_span = {
            let t = self.expect(TokenKind::LBrace)?;
            t.span.clone()
        };
        let mut statements = Vec::new();

        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }

        let rbrace_end = {
            let t = self.expect(TokenKind::RBrace)?;
            t.span.end
        };

        let span = Span::new(
            lbrace_span.start,
            rbrace_end,
            lbrace_span.line,
            lbrace_span.col,
        );

        Ok(Block { statements, span })
    }

    /// Parse a single statement, dispatching on the first token.
    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        let first = self.peek().ok_or_else(|| {
            ParseError::new("expected statement", self.current_span())
        })?;
        let kw_span = first.span.clone();

        match first.kind {
            TokenKind::Let => {
                self.advance();
                self.parse_let_stmt(kw_span)
            }
            TokenKind::Return => {
                self.advance();
                self.parse_return_stmt()
            }
            TokenKind::If => {
                self.advance();
                self.parse_if_stmt(kw_span)
            }
            TokenKind::While => {
                self.advance();
                self.parse_while_stmt(kw_span)
            }
            TokenKind::For => {
                self.advance();
                self.parse_for_stmt(kw_span)
            }
            _ => {
                // Expression statement.
                let expr = self.parse_expr(Precedence::Lowest)?;
                self.expect(TokenKind::Semicolon)?;
                Ok(Statement::ExprStmt(expr))
            }
        }
    }
}

// ============================================================================
// Statement sub-parsers
// ============================================================================

impl<'a> ParserState<'a> {
    /// Parse `let [mut] name [: type] = value ;`
    fn parse_let_stmt(&mut self, let_span: Span) -> Result<Statement, ParseError> {
        let is_mut = if self.check_lexeme("mut") {
            self.advance();
            true
        } else {
            false
        };

        let var_name = {
            let t = self.expect(TokenKind::Identifier)?;
            t.lexeme.clone()
        };

        let type_annotation = if self.check(TokenKind::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(TokenKind::Eq)?;
        let value = self.parse_expr(Precedence::Lowest)?;
        self.expect(TokenKind::Semicolon)?;

        Ok(Statement::Let(LetStmt {
            name: var_name,
            type_annotation,
            value,
            is_mut,
            span: let_span,
        }))
    }

    /// Parse `return [expr] ;`
    fn parse_return_stmt(&mut self) -> Result<Statement, ParseError> {
        let expr = if self.check(TokenKind::Semicolon) {
            None
        } else {
            let e = self.parse_expr(Precedence::Lowest)?;
            Some(e)
        };

        self.expect(TokenKind::Semicolon)?;

        Ok(Statement::Return(expr))
    }

    /// Parse `if expr { block } [else { block } | else if expr { block } ...]`
    fn parse_if_stmt(&mut self, if_span: Span) -> Result<Statement, ParseError> {
        let condition = self.parse_expr(Precedence::Lowest)?;
        let then_branch = self.parse_block()?;

        let (else_branch, end_span) = if self.check(TokenKind::Else) {
            self.advance(); // consume 'else'
            if self.check(TokenKind::If) {
                let inner_if_span = self.advance().unwrap().span.clone();
                let inner_if = self.parse_if_stmt(inner_if_span)?;
                let inner_end = match &inner_if {
                    Statement::If(ref s) => s.span.clone(),
                    _ => Span::new(0, 0, 0, 0),
                };
                (Some(Box::new(inner_if)), inner_end)
            } else {
                let block = self.parse_block()?;
                let block_end = block.span.clone();
                (
                    Some(Box::new(Statement::Block(block))),
                    block_end,
                )
            }
        } else {
            (None, then_branch.span.clone())
        };

        let span = Span::new(
            if_span.start,
            end_span.end,
            if_span.line,
            if_span.col,
        );

        Ok(Statement::If(IfStmt {
            condition,
            then_branch,
            else_branch,
            span,
        }))
    }

    /// Parse `while expr { block }`
    fn parse_while_stmt(&mut self, while_span: Span) -> Result<Statement, ParseError> {
        let condition = self.parse_expr(Precedence::Lowest)?;
        let body = self.parse_block()?;

        let span = Span::new(
            while_span.start,
            body.span.end,
            while_span.line,
            while_span.col,
        );

        Ok(Statement::While(WhileStmt {
            condition,
            body,
            span,
        }))
    }

    /// Parse `for name in expr { block }`
    fn parse_for_stmt(&mut self, for_span: Span) -> Result<Statement, ParseError> {
        let var_name = {
            let t = self.expect(TokenKind::Identifier)?;
            t.lexeme.clone()
        };

        // `in` is not a keyword token — it appears as an Identifier.
        let next = self.peek().ok_or_else(|| {
            ParseError::new(
                "expected 'in' after for variable",
                self.current_span(),
            )
        })?;
        if next.lexeme != "in" {
            return Err(ParseError::new(
                format!("expected 'in', found '{}'", next.lexeme),
                next.span.clone(),
            ));
        }
        self.advance(); // consume 'in'

        let iterable = self.parse_expr(Precedence::Lowest)?;
        let body = self.parse_block()?;

        let span = Span::new(
            for_span.start,
            body.span.end,
            for_span.line,
            for_span.col,
        );

        Ok(Statement::For(ForStmt {
            variable: var_name,
            iterable,
            body,
            span,
        }))
    }
}

// ============================================================================
// Expression parser (Pratt)
// ============================================================================

impl<'a> ParserState<'a> {
    /// Main entry point for expression parsing using the Pratt algorithm.
    ///
    /// Parses an expression at or above the given precedence level.
    /// Lower-precedence operators are handled by the caller.
    pub fn parse_expr(&mut self, precedence: Precedence) -> Result<Expr, ParseError> {
        let mut left = self.parse_prefix()?;

        loop {
            let token_kind = match self.peek_kind() {
                Some(k) => k,
                None => break,
            };

            // ---- binary (infix) operators -----------------------------------
            if let Some(op_prec) = Precedence::from_binary_op(token_kind) {
                if op_prec >= precedence {
                    let _op_token = self.advance().unwrap();
                    let op = Precedence::binary_op(token_kind).unwrap();
                    let right = self.parse_expr(op_prec.next())?;
                    let span = Span::new(
                        left.span.start,
                        right.span.end,
                        left.span.line,
                        left.span.col,
                    );
                    left = Expr {
                        kind: ExprKind::Binary {
                            op,
                            left: Box::new(left),
                            right: Box::new(right),
                        },
                        span,
                    };
                    continue;
                }
            }

            // ---- postfix operators: call `expr(...)` and field access `expr.field`
            match token_kind {
                TokenKind::LParen => {
                    if precedence <= Precedence::Call {
                        self.advance(); // consume '('
                        left = self.parse_call(left)?;
                        continue;
                    }
                }
                TokenKind::Dot => {
                    if precedence <= Precedence::Call {
                        self.advance(); // consume '.'
                        let (field_name, field_end) = {
                            let current = self.current_span();
                            let t = self
                                .expect(TokenKind::Identifier)
                                .map_err(|_| {
                                    ParseError::new("expected field name after '.'", current)
                                })?;
                            (t.lexeme.clone(), t.span.end)
                        };
                        let span = Span::new(
                            left.span.start,
                            field_end,
                            left.span.line,
                            left.span.col,
                        );
                        left = Expr {
                            kind: ExprKind::FieldAccess {
                                object: Box::new(left),
                                field: field_name,
                            },
                            span,
                        };
                        continue;
                    }
                }
                _ => {}
            }

            break;
        }

        Ok(left)
    }

    /// Parse a prefix expression (literal, identifier, unary op, paren, match, array).
    fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        let token = match self.advance().cloned() {
            Some(t) => t,
            None => {
                return Err(ParseError::new(
                    "unexpected end of input",
                    Span::new(0, 0, 0, 0),
                ));
            }
        };

        match token.kind {
            // -- literals -----------------------------------------------------
            TokenKind::IntLiteral => {
                // Strip underscores that are valid digit separators, then parse.
                let cleaned = token.lexeme.replace('_', "");
                let value: i64 = cleaned.parse().unwrap_or(0);
                Ok(Expr {
                    kind: ExprKind::IntLiteral(value),
                    span: token.span.clone(),
                })
            }
            TokenKind::FloatLiteral => {
                let cleaned = token.lexeme.replace('_', "");
                let value: f64 = cleaned.parse().unwrap_or(0.0);
                Ok(Expr {
                    kind: ExprKind::FloatLiteral(value),
                    span: token.span.clone(),
                })
            }
            TokenKind::StringLiteral => {
                // Strip surrounding quotes (does not process escape sequences yet).
                let inner = &token.lexeme[1..token.lexeme.len() - 1];
                Ok(Expr {
                    kind: ExprKind::StringLiteral(inner.to_string()),
                    span: token.span.clone(),
                })
            }
            TokenKind::BoolLiteral => {
                let value = token.lexeme == "true";
                Ok(Expr {
                    kind: ExprKind::BoolLiteral(value),
                    span: token.span.clone(),
                })
            }

            // -- identifier / variable ---------------------------------------
            TokenKind::Identifier => Ok(Expr {
                kind: ExprKind::Variable(token.lexeme.clone()),
                span: token.span.clone(),
            }),

            // -- unary operators ---------------------------------------------
            TokenKind::Minus => {
                let operand = self.parse_expr(Precedence::Prefix)?;
                let span = Span::new(
                    token.span.start,
                    operand.span.end,
                    token.span.line,
                    token.span.col,
                );
                Ok(Expr {
                    kind: ExprKind::Unary {
                        op: UnaryOp::Neg,
                        operand: Box::new(operand),
                    },
                    span,
                })
            }
            TokenKind::Not => {
                let operand = self.parse_expr(Precedence::Prefix)?;
                let span = Span::new(
                    token.span.start,
                    operand.span.end,
                    token.span.line,
                    token.span.col,
                );
                Ok(Expr {
                    kind: ExprKind::Unary {
                        op: UnaryOp::Not,
                        operand: Box::new(operand),
                    },
                    span,
                })
            }

            // -- grouped expression ------------------------------------------
            TokenKind::LParen => {
                let inner = self.parse_expr(Precedence::Lowest)?;
                let rparen = self.expect(TokenKind::RParen)?;
                let span = Span::new(
                    token.span.start,
                    rparen.span.end,
                    token.span.line,
                    token.span.col,
                );
                Ok(Expr {
                    kind: ExprKind::Paren(Box::new(inner)),
                    span,
                })
            }

            // -- await expression --------------------------------------------
            TokenKind::Await => {
                let inner = self.parse_expr(Precedence::Prefix)?;
                let span = Span::new(
                    token.span.start,
                    inner.span.end,
                    token.span.line,
                    token.span.col,
                );
                Ok(Expr {
                    kind: ExprKind::Await(Box::new(inner)),
                    span,
                })
            }

            // -- match expression --------------------------------------------
            TokenKind::Match => self.parse_match_expr(&token),

            // -- array literal -----------------------------------------------
            TokenKind::LBracket => self.parse_array_literal(&token),

            // -- unexpected --------------------------------------------------
            _ => Err(ParseError::expected(
                vec![
                    TokenKind::IntLiteral,
                    TokenKind::FloatLiteral,
                    TokenKind::StringLiteral,
                    TokenKind::BoolLiteral,
                    TokenKind::Identifier,
                    TokenKind::Minus,
                    TokenKind::Not,
                    TokenKind::LParen,
                    TokenKind::Await,
                    TokenKind::Match,
                    TokenKind::LBracket,
                ],
                Some(token.kind),
                token.span.clone(),
            )),
        }
    }
}

// ============================================================================
// Call, match, array
// ============================================================================

impl<'a> ParserState<'a> {
    /// Parse a function-call: consume `( args... )` after the callee.
    /// The opening `(` has already been consumed by the Pratt loop.
    fn parse_call(&mut self, callee: Expr) -> Result<Expr, ParseError> {
        let start = callee.span.clone();

        let mut args = Vec::new();
        if !self.check(TokenKind::RParen) {
            loop {
                args.push(self.parse_expr(Precedence::Lowest)?);
                if self.check(TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        let rparen = self.expect(TokenKind::RParen)?;
        let span = Span::new(
            start.start,
            rparen.span.end,
            start.line,
            start.col,
        );

        Ok(Expr {
            kind: ExprKind::Call {
                callee: Box::new(callee),
                args,
            },
            span,
        })
    }

    /// Parse `match expr { arms... }`. The `match` keyword has already been consumed.
    fn parse_match_expr(&mut self, match_token: &Token) -> Result<Expr, ParseError> {
        let scrutinee = self.parse_expr(Precedence::Lowest)?;
        self.expect(TokenKind::LBrace)?;

        let mut arms = Vec::new();

        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            let pattern = self.parse_match_pattern()?;
            self.expect(TokenKind::FatArrow)?;
            let body = self.parse_expr(Precedence::Lowest)?;
            self.consume_if(TokenKind::Comma);

            arms.push(MatchArm {
                pattern,
                body,
                span: Span::new(0, 0, 0, 0), // simplified arm-span
            });
        }

        let rbrace = self.expect(TokenKind::RBrace)?;
        let span = Span::new(
            match_token.span.start,
            rbrace.span.end,
            match_token.span.line,
            match_token.span.col,
        );

        Ok(Expr {
            kind: ExprKind::Match {
                expr: Box::new(scrutinee),
                arms,
            },
            span,
        })
    }

    /// Parse a match arm pattern.
    ///
    /// - `Name binding`  → `Constructor { name, binding: Some(binding) }`
    /// - `Name`          → `Constructor { name, binding: None }`
    /// - `_`             → `Wildcard`
    /// - literal         → `Literal(...)`
    /// - anything else   → `Wildcard`
    fn parse_match_pattern(&mut self) -> Result<MatchPattern, ParseError> {
        let (kind, lexeme) = {
            let token = self.peek().ok_or_else(|| {
                ParseError::new("expected match pattern", self.current_span())
            })?;
            (token.kind, token.lexeme.clone())
        };

        match kind {
            TokenKind::Identifier => {
                if lexeme == "_" {
                    self.advance();
                    Ok(MatchPattern::Wildcard)
                } else {
                    self.advance();
                    if self.check(TokenKind::Identifier) {
                        let binding = self.advance().unwrap().lexeme.clone();
                        Ok(MatchPattern::Constructor {
                            name: lexeme,
                            binding: Some(binding),
                        })
                    } else {
                        Ok(MatchPattern::Constructor {
                            name: lexeme,
                            binding: None,
                        })
                    }
                }
            }
            TokenKind::IntLiteral => {
                let cleaned = lexeme.replace('_', "");
                let value: i64 = cleaned.parse().unwrap_or(0);
                self.advance();
                Ok(MatchPattern::Literal(MatchLiteral::Int(value)))
            }
            TokenKind::BoolLiteral => {
                let value = lexeme == "true";
                self.advance();
                Ok(MatchPattern::Literal(MatchLiteral::Bool(value)))
            }
            TokenKind::StringLiteral => {
                let inner = &lexeme[1..lexeme.len() - 1];
                self.advance();
                Ok(MatchPattern::Literal(MatchLiteral::String(
                    inner.to_string(),
                )))
            }
            _ => {
                // Anything unexpected is a wildcard.
                self.advance();
                Ok(MatchPattern::Wildcard)
            }
        }
    }

    /// Parse `[ elem, elem, ... ]`. The opening `[` has already been consumed.
    fn parse_array_literal(&mut self, lbracket: &Token) -> Result<Expr, ParseError> {
        let mut elements = Vec::new();

        if !self.check(TokenKind::RBracket) {
            loop {
                elements.push(self.parse_expr(Precedence::Lowest)?);
                if self.check(TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        let rbracket = self.expect(TokenKind::RBracket)?;
        let span = Span::new(
            lbracket.span.start,
            rbracket.span.end,
            lbracket.span.line,
            lbracket.span.col,
        );

        Ok(Expr {
            kind: ExprKind::ArrayLiteral(elements),
            span,
        })
    }
}

// ============================================================================
// Types & params
// ============================================================================

impl<'a> ParserState<'a> {
    /// Parse a type expression.
    ///
    /// - `Identifier<Type, ...>`  → `Generic`
    /// - `Identifier`             → `Named`
    /// - `(params) -> ret`        → `Function`
    /// - `(tuple, ...)`           → `Tuple`
    /// - `(single)`               → unwrapped to `single`
    pub fn parse_type(&mut self) -> Result<Type, ParseError> {
        if self.check(TokenKind::LParen) {
            self.advance(); // consume '('

            let mut types = Vec::new();
            if !self.check(TokenKind::RParen) {
                loop {
                    types.push(self.parse_type()?);
                    if self.check(TokenKind::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }

            self.expect(TokenKind::RParen)?;

            // Check for function-type arrow.
            if self.check(TokenKind::Arrow) {
                self.advance();
                let ret = self.parse_type()?;
                Ok(Type::Function {
                    params: types,
                    ret: Box::new(ret),
                })
            } else if types.len() == 1 {
                // `(T)` unwraps to just `T`.
                Ok(types.into_iter().next().unwrap())
            } else {
                Ok(Type::Tuple(types))
            }
        } else {
            let name = {
                let t = self.expect(TokenKind::Identifier)?;
                t.lexeme.clone()
            };

            if self.check(TokenKind::LAngle) {
                self.advance();
                let mut args = Vec::new();
                loop {
                    args.push(self.parse_type()?);
                    if self.check(TokenKind::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(TokenKind::RAngle)?;
                Ok(Type::Generic { name, args })
            } else {
                Ok(Type::Named(name))
            }
        }
    }

    /// Parse a comma-separated list of `name: Type` parameters (used inside
    /// declaration argument lists).  The surrounding parens are handled by the
    /// caller.
    fn parse_params(&mut self) -> Result<Vec<Param>, ParseError> {
        let mut params = Vec::new();

        if self.check(TokenKind::RParen) {
            return Ok(params);
        }

        loop {
            let (pname, pspan) = {
                let t = self.expect(TokenKind::Identifier)?;
                (t.lexeme.clone(), t.span.clone())
            };
            self.expect(TokenKind::Colon)?;
            let ty = self.parse_type()?;

            params.push(Param {
                name: pname,
                ty,
                span: pspan,
            });

            if self.check(TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        Ok(params)
    }
}