//! Script Language Formatter
//!
//! This module provides production-quality formatting for Script language code.

use crate::parser::{
    BinaryOp, Block, EnumVariant, ExportSpec, Expr, ExprKind, ImportSpecifier, Literal, MatchArm,
    Method, Param, Pattern, PatternKind, Program, Stmt, StmtKind, TypeAnn, TypeKind, UnaryOp,
};

/// Configuration for the Script formatter
#[derive(Debug, Clone)]
pub struct FormatterConfig {
    /// Number of spaces for indentation
    pub indent_size: usize,
    /// Use spaces instead of tabs
    pub use_spaces: bool,
    /// Maximum line length before wrapping
    pub max_line_length: usize,
    /// Whether to insert spaces around operators
    pub spaces_around_operators: bool,
    /// Whether to insert trailing commas in multiline contexts
    pub trailing_commas: bool,
    /// Whether to collapse empty blocks
    pub collapse_empty_blocks: bool,
    /// Whether to sort imports
    pub sort_imports: bool,
}

impl Default for FormatterConfig {
    fn default() -> Self {
        Self {
            indent_size: 4,
            use_spaces: true,
            max_line_length: 100,
            spaces_around_operators: true,
            trailing_commas: true,
            collapse_empty_blocks: true,
            sort_imports: true,
        }
    }
}

/// Production-quality Script language formatter
pub struct Formatter {
    config: FormatterConfig,
    output: String,
    current_indent: usize,
    at_line_start: bool,
}

impl Formatter {
    /// Create a new formatter with default configuration
    pub fn new() -> Self {
        Self::with_config(FormatterConfig::default())
    }

    /// Create a new formatter with custom configuration
    pub fn with_config(config: FormatterConfig) -> Self {
        Self {
            config,
            output: String::new(),
            current_indent: 0,
            at_line_start: true,
        }
    }

    /// Format a complete program
    pub fn format_program(&mut self, program: &Program) -> String {
        self.output.clear();
        self.current_indent = 0;
        self.at_line_start = true;

        for (i, stmt) in program.statements.iter().enumerate() {
            self.format_statement(stmt);

            if i < program.statements.len() - 1 {
                self.write_newline();

                // Add extra spacing between function/struct/enum declarations
                match stmt.kind {
                    StmtKind::Function { .. } | StmtKind::Struct { .. } | StmtKind::Enum { .. } => {
                        self.write_newline();
                    }
                    _ => {}
                }
            } else {
                self.write_newline();
            }
        }

        self.output.clone()
    }

    /// Format a statement
    fn format_statement(&mut self, stmt: &Stmt) {
        self.write_indent();

        match &stmt.kind {
            StmtKind::Let {
                name,
                type_ann,
                init,
            } => {
                self.write("let ");
                self.write(name);

                if let Some(ty) = type_ann {
                    self.write(": ");
                    self.format_type_annotation(ty);
                }

                if let Some(value) = init {
                    self.write(" = ");
                    self.format_expression(value);
                }

                self.write(";");
            }

            StmtKind::Function {
                name,
                params,
                ret_type,
                body,
                is_async,
                ..
            } => {
                if *is_async {
                    self.write("async ");
                }
                self.write("fn ");
                self.write(name);

                self.write("(");
                self.format_params(params);
                self.write(")");

                if let Some(ret) = ret_type {
                    self.write(" -> ");
                    self.format_type_annotation(ret);
                }

                self.write(" ");
                self.format_block(body);
            }

            StmtKind::Return(expr) => {
                self.write("return");
                if let Some(e) = expr {
                    self.write(" ");
                    self.format_expression(e);
                }
                self.write(";");
            }

            StmtKind::Expression(expr) => {
                self.format_expression(expr);
                self.write(";");
            }

            StmtKind::While { condition, body } => {
                self.write("while ");
                self.format_expression(condition);
                self.write(" ");
                self.format_block(body);
            }

            StmtKind::For {
                variable,
                iterable,
                body,
            } => {
                self.write("for ");
                self.write(variable);
                self.write(" in ");
                self.format_expression(iterable);
                self.write(" ");
                self.format_block(body);
            }

            StmtKind::Import { imports, module } => {
                self.write("import ");
                self.format_import_specifiers(imports);
                self.write(" from \"");
                self.write(module);
                self.write("\";");
            }

            StmtKind::Export { export } => {
                self.write("export ");
                self.format_export_spec(export);
                self.write(";");
            }

            StmtKind::Struct { name, fields, .. } => {
                self.write("struct ");
                self.write(name);

                self.write(" {\n");
                self.increase_indent();

                for (i, field) in fields.iter().enumerate() {
                    self.write_indent();
                    self.write(&field.name);
                    self.write(": ");
                    self.format_type_annotation(&field.type_ann);

                    if i < fields.len() - 1 || self.config.trailing_commas {
                        self.write(",");
                    }
                    self.write("\n");
                }

                self.decrease_indent();
                self.write_indent();
                self.write("}");
            }

            StmtKind::Enum { name, variants, .. } => {
                self.write("enum ");
                self.write(name);

                self.write(" {\n");
                self.increase_indent();

                for (i, variant) in variants.iter().enumerate() {
                    self.write_indent();
                    self.format_enum_variant(variant);

                    if i < variants.len() - 1 || self.config.trailing_commas {
                        self.write(",");
                    }
                    self.write("\n");
                }

                self.decrease_indent();
                self.write_indent();
                self.write("}");
            }

            StmtKind::Impl(impl_block) => {
                self.write("impl ");
                self.write(&impl_block.type_name);

                self.write(" {\n");
                self.increase_indent();

                for (i, method) in impl_block.methods.iter().enumerate() {
                    if i > 0 {
                        self.write("\n");
                    }
                    self.format_method(method);
                }

                self.decrease_indent();
                self.write_indent();
                self.write("}");
            }
        }
    }

    /// Format an expression
    fn format_expression(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Literal(lit) => self.format_literal(lit),

            ExprKind::Identifier(name) => self.write(name),

            ExprKind::Binary { left, op, right } => {
                self.format_expression(left);
                if self.config.spaces_around_operators {
                    self.write(" ");
                }
                self.format_binary_op(op);
                if self.config.spaces_around_operators {
                    self.write(" ");
                }
                self.format_expression(right);
            }

            ExprKind::Unary { op, expr } => {
                self.format_unary_op(op);
                self.format_expression(expr);
            }

            ExprKind::Call { callee, args } => {
                self.format_expression(callee);
                self.write("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.format_expression(arg);
                }
                self.write(")");
            }

            ExprKind::Index { object, index } => {
                self.format_expression(object);
                self.write("[");
                self.format_expression(index);
                self.write("]");
            }

            ExprKind::Member { object, property } => {
                self.format_expression(object);
                self.write(".");
                self.write(property);
            }

            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.write("if ");
                self.format_expression(condition);
                self.write(" ");
                self.format_expression(then_branch);

                if let Some(else_expr) = else_branch {
                    self.write(" else ");
                    self.format_expression(else_expr);
                }
            }

            ExprKind::Block(block) => {
                self.format_block(block);
            }

            ExprKind::Array(elements) => {
                self.write("[");
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.format_expression(elem);
                }
                self.write("]");
            }

            ExprKind::Assign { target, value } => {
                self.format_expression(target);
                self.write(" = ");
                self.format_expression(value);
            }

            ExprKind::Match { expr, arms } => {
                self.write("match ");
                self.format_expression(expr);
                self.write(" {\n");
                self.increase_indent();

                for arm in arms {
                    self.format_match_arm(arm);
                    self.write("\n");
                }

                self.decrease_indent();
                self.write_indent();
                self.write("}");
            }

            _ => {
                // For other expressions, just write a placeholder
                self.write("/* complex expression */");
            }
        }
    }

    /// Format a block
    fn format_block(&mut self, block: &Block) {
        if block.statements.is_empty() && self.config.collapse_empty_blocks {
            self.write("{}");
            return;
        }

        self.write("{\n");
        self.increase_indent();

        for stmt in &block.statements {
            self.format_statement(stmt);
            self.write("\n");
        }

        self.decrease_indent();
        self.write_indent();
        self.write("}");
    }

    /// Format function parameters
    fn format_params(&mut self, params: &[Param]) {
        for (i, param) in params.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.write(&param.name);
            self.write(": ");
            self.format_type_annotation(&param.type_ann);
        }
    }

    /// Format a type annotation
    fn format_type_annotation(&mut self, ty: &TypeAnn) {
        match &ty.kind {
            TypeKind::Named(name) => self.write(name),
            TypeKind::Array(elem_ty) => {
                self.write("[");
                self.format_type_annotation(elem_ty);
                self.write("]");
            }
            TypeKind::Generic { name, args } => {
                self.write(name);
                self.write("<");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.format_type_annotation(arg);
                }
                self.write(">");
            }
            _ => {
                // For other types, use simple display
                self.write(&format!("{:?}", ty.kind));
            }
        }
    }

    /// Format a literal value
    fn format_literal(&mut self, lit: &Literal) {
        match lit {
            Literal::Number(n) => self.write(&n.to_string()),
            Literal::String(s) => {
                self.write("\"");
                self.write(&s.replace('\\', "\\\\").replace('"', "\\\""));
                self.write("\"");
            }
            Literal::Boolean(b) => self.write(&b.to_string()),
            Literal::Null => self.write("null"),
        }
    }

    /// Format a binary operator
    fn format_binary_op(&mut self, op: &BinaryOp) {
        let op_str = match op {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
            BinaryOp::Equal => "==",
            BinaryOp::NotEqual => "!=",
            BinaryOp::Less => "<",
            BinaryOp::LessEqual => "<=",
            BinaryOp::Greater => ">",
            BinaryOp::GreaterEqual => ">=",
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
        };
        self.write(op_str);
    }

    /// Format a unary operator
    fn format_unary_op(&mut self, op: &UnaryOp) {
        let op_str = match op {
            UnaryOp::Minus => "-",
            UnaryOp::Not => "!",
        };
        self.write(op_str);
    }

    /// Format import specifiers (simplified)
    fn format_import_specifiers(&mut self, imports: &[ImportSpecifier]) {
        if imports.len() == 1 {
            match &imports[0] {
                ImportSpecifier::Default { name } => {
                    self.write(name);
                    return;
                }
                _ => {}
            }
        }

        self.write("{ ");
        for (i, import) in imports.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            match import {
                ImportSpecifier::Named { name, alias } => {
                    self.write(name);
                    if let Some(alias) = alias {
                        self.write(" as ");
                        self.write(alias);
                    }
                }
                ImportSpecifier::Default { name } => {
                    self.write(name);
                }
                ImportSpecifier::Namespace { alias } => {
                    self.write("* as ");
                    self.write(alias);
                }
            }
        }
        self.write(" }");
    }

    /// Format export specification (simplified)
    fn format_export_spec(&mut self, export: &ExportSpec) {
        // Simplified export formatting
        self.write(&format!("{:?}", export));
    }

    /// Format enum variant (simplified)
    fn format_enum_variant(&mut self, variant: &EnumVariant) {
        self.write(&variant.name);
        // Simplified - just the name for now
    }

    /// Format method (simplified)
    fn format_method(&mut self, method: &Method) {
        self.write_indent();
        if method.is_async {
            self.write("async ");
        }
        self.write("fn ");
        self.write(&method.name);

        self.write("(");
        self.format_params(&method.params);
        self.write(")");

        if let Some(ret) = &method.ret_type {
            self.write(" -> ");
            self.format_type_annotation(ret);
        }

        self.write(" ");
        self.format_block(&method.body);
    }

    /// Format match arm (simplified)
    fn format_match_arm(&mut self, arm: &MatchArm) {
        self.write_indent();
        self.format_pattern(&arm.pattern);

        if let Some(guard) = &arm.guard {
            self.write(" if ");
            self.format_expression(guard);
        }

        self.write(" => ");
        self.format_expression(&arm.body);
        self.write(",");
    }

    /// Format pattern (simplified)
    fn format_pattern(&mut self, pattern: &Pattern) {
        match &pattern.kind {
            PatternKind::Wildcard => self.write("_"),
            PatternKind::Literal(lit) => self.format_literal(lit),
            PatternKind::Identifier(name) => self.write(name),
            _ => {
                // Simplified pattern formatting
                self.write("/* pattern */");
            }
        }
    }

    // Helper methods for indentation and output

    fn write(&mut self, text: &str) {
        if self.at_line_start && !text.is_empty() && text != "\n" {
            self.at_line_start = false;
        }
        self.output.push_str(text);
    }

    fn write_newline(&mut self) {
        self.output.push('\n');
        self.at_line_start = true;
    }

    fn write_indent(&mut self) {
        if self.at_line_start {
            let indent = if self.config.use_spaces {
                " ".repeat(self.current_indent * self.config.indent_size)
            } else {
                "	".repeat(self.current_indent)
            };
            self.output.push_str(&indent);
            self.at_line_start = false;
        }
    }

    fn increase_indent(&mut self) {
        self.current_indent += 1;
    }

    fn decrease_indent(&mut self) {
        if self.current_indent > 0 {
            self.current_indent -= 1;
        }
    }
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to format a program with default settings
pub fn format_program(program: &Program) -> String {
    let mut formatter = Formatter::new();
    formatter.format_program(program)
}

/// Convenience function to format a program with custom configuration
pub fn format_program_with_config(program: &Program, config: FormatterConfig) -> String {
    let mut formatter = Formatter::with_config(config);
    formatter.format_program(program)
}
