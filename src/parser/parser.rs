use super::ast::*;
use crate::{
    error::{Error, ErrorKind, Result},
    lexer::{Token, TokenKind},
    source::{SourceLocation, Span},
};

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    next_expr_id: usize, // Counter for generating unique expression IDs
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            current: 0,
            next_expr_id: 0,
        }
    }

    /// Generate a unique expression ID
    fn next_expr_id(&mut self) -> usize {
        let id = self.next_expr_id;
        self.next_expr_id += 1;
        id
    }

    /// Helper function to create an expression with auto-generated ID
    fn create_expr(&mut self, kind: ExprKind, span: Span) -> Expr {
        Expr {
            kind,
            span,
            id: self.next_expr_id(),
        }
    }

    pub fn parse(&mut self) -> Result<Program> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            // Skip newlines at statement level
            if self.match_token(&TokenKind::Newline) {
                continue;
            }

            statements.push(self.parse_statement()?);
        }

        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> Result<Stmt> {
        let start = self.current_location();

        // Parse attributes
        let mut attributes = Vec::new();
        while self.check(&TokenKind::At) {
            attributes.push(self.parse_attribute()?);
            // Skip optional newline after attribute
            self.match_token(&TokenKind::Newline);
        }

        let kind = if self.match_token(&TokenKind::Let) {
            self.parse_let_statement()?
        } else if self.match_token(&TokenKind::Async) {
            // Look ahead for fn keyword
            if self.check(&TokenKind::Fn) {
                self.advance(); // consume fn
                self.parse_async_function_statement()?
            } else {
                return Err(self.error("Expected 'fn' after 'async'"));
            }
        } else if self.match_token(&TokenKind::Fn) {
            self.parse_function_statement()?
        } else if self.match_token(&TokenKind::Return) {
            self.parse_return_statement()?
        } else if self.match_token(&TokenKind::While) {
            self.parse_while_statement()?
        } else if self.match_token(&TokenKind::For) {
            self.parse_for_statement()?
        } else if self.match_token(&TokenKind::Import) {
            self.parse_import_statement()?
        } else if self.match_token(&TokenKind::Export) {
            self.parse_export_statement()?
        } else if self.match_token(&TokenKind::Struct) {
            self.parse_struct_declaration()?
        } else if self.match_token(&TokenKind::Enum) {
            self.parse_enum_declaration()?
        } else if self.match_token(&TokenKind::Impl) {
            self.parse_impl_block()?
        } else {
            StmtKind::Expression(self.parse_expression()?)
        };

        let end = self.previous_location();
        let span = Span::new(start, end);

        // Consume optional semicolon or newline
        let _ = self.match_token(&TokenKind::Semicolon) || self.match_token(&TokenKind::Newline);

        Ok(Stmt {
            kind,
            span,
            attributes,
        })
    }

    fn parse_let_statement(&mut self) -> Result<StmtKind> {
        let name = self.consume_identifier("Expected variable name")?;

        let type_ann = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        let init = if self.match_token(&TokenKind::Equals) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(StmtKind::Let {
            name,
            type_ann,
            init,
        })
    }

    fn parse_function_statement(&mut self) -> Result<StmtKind> {
        self.parse_function_common(false)
    }

    fn parse_async_function_statement(&mut self) -> Result<StmtKind> {
        self.parse_function_common(true)
    }

    fn parse_function_common(&mut self, is_async: bool) -> Result<StmtKind> {
        let name = self.consume_identifier("Expected function name")?;

        // Parse generic parameters if present
        let generic_params = if self.check(&TokenKind::Less) {
            Some(self.parse_generic_parameters()?)
        } else {
            None
        };

        self.consume(&TokenKind::LeftParen, "Expected '(' after function name")?;

        let mut params = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            loop {
                let param_name = self.consume_identifier("Expected parameter name")?;
                self.consume(&TokenKind::Colon, "Expected ':' after parameter name")?;
                let param_type = self.parse_type_annotation()?;

                params.push(Param {
                    name: param_name,
                    type_ann: param_type,
                });

                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
        }

        self.consume(&TokenKind::RightParen, "Expected ')' after parameters")?;

        let ret_type = if self.match_token(&TokenKind::Arrow) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        // Parse where clause if present
        let where_clause = if self.check(&TokenKind::Where) {
            Some(self.parse_where_clause()?)
        } else {
            None
        };

        self.consume(&TokenKind::LeftBrace, "Expected '{' before function body")?;
        let body = self.parse_block()?;

        Ok(StmtKind::Function {
            name,
            generic_params,
            params,
            ret_type,
            where_clause,
            body,
            is_async,
        })
    }

    fn parse_return_statement(&mut self) -> Result<StmtKind> {
        let expr = if self.check(&TokenKind::Semicolon)
            || self.check(&TokenKind::Newline)
            || self.is_at_end()
        {
            None
        } else {
            Some(self.parse_expression()?)
        };

        Ok(StmtKind::Return(expr))
    }

    /// Parse generic parameters: <T>, <T: Clone>, <T, U: Debug + Send>
    fn parse_generic_parameters(&mut self) -> Result<GenericParams> {
        let start = self.current_location();
        self.consume(&TokenKind::Less, "Expected '<' to start generic parameters")?;

        let mut params = Vec::new();

        // Handle empty generic params case: fn foo<>()
        if self.check(&TokenKind::Greater) {
            self.advance(); // consume '>'
            let span = Span::new(start, self.previous_location());
            return Ok(GenericParams { params, span });
        }

        // Parse first generic parameter
        params.push(self.parse_generic_param()?);

        // Parse remaining generic parameters
        while self.match_token(&TokenKind::Comma) {
            // Allow trailing comma: fn foo<T,>()
            if self.check(&TokenKind::Greater) {
                break;
            }
            params.push(self.parse_generic_param()?);
        }

        self.consume(
            &TokenKind::Greater,
            "Expected '>' to close generic parameters",
        )?;

        let span = Span::new(start, self.previous_location());
        Ok(GenericParams { params, span })
    }

    /// Parse a single generic parameter with optional trait bounds
    fn parse_generic_param(&mut self) -> Result<GenericParam> {
        let start = self.current_location();
        let name = self.consume_identifier("Expected generic parameter name")?;

        let mut bounds = Vec::new();

        // Check for trait bounds after ':'
        if self.match_token(&TokenKind::Colon) {
            // Parse first trait bound
            bounds.push(self.parse_trait_bound()?);

            // Parse additional bounds with '+'
            while self.match_token(&TokenKind::Plus) {
                bounds.push(self.parse_trait_bound()?);
            }
        }

        let span = Span::new(start, self.previous_location());
        Ok(GenericParam { name, bounds, span })
    }

    /// Parse a trait bound (e.g., Clone, Debug, Send)
    fn parse_trait_bound(&mut self) -> Result<TraitBound> {
        let start = self.current_location();
        let trait_name = self.consume_identifier("Expected trait name")?;
        let span = Span::new(start, self.previous_location());
        Ok(TraitBound { trait_name, span })
    }

    fn parse_while_statement(&mut self) -> Result<StmtKind> {
        let condition = self.parse_expression()?;
        self.consume(&TokenKind::LeftBrace, "Expected '{' after while condition")?;
        let body = self.parse_block()?;

        Ok(StmtKind::While { condition, body })
    }

    fn parse_for_statement(&mut self) -> Result<StmtKind> {
        let variable = self.consume_identifier("Expected variable name in for loop")?;

        // Check for "in" keyword
        if !self.match_token(&TokenKind::In) {
            return Err(self.error("Expected 'in' in for loop"));
        }

        let iterable = self.parse_expression()?;
        self.consume(&TokenKind::LeftBrace, "Expected '{' after for loop header")?;
        let body = self.parse_block()?;

        Ok(StmtKind::For {
            variable,
            iterable,
            body,
        })
    }

    fn parse_import_statement(&mut self) -> Result<StmtKind> {
        let mut specifiers = Vec::new();

        // Parse import specifiers
        if self.check(&TokenKind::Star) {
            // Namespace import: import * as name
            self.advance(); // consume '*'
            self.consume(
                &TokenKind::As,
                "Expected 'as' after '*' in namespace import",
            )?;
            let alias = self.consume_identifier("Expected alias name after 'as'")?;
            specifiers.push(ImportSpecifier::Namespace { alias });
        } else if self.check(&TokenKind::LeftBrace) {
            // Named imports: import { a, b as c }
            self.advance(); // consume '{'

            while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
                let name = self.consume_identifier("Expected import name")?;
                let alias = if self.match_token(&TokenKind::As) {
                    Some(self.consume_identifier("Expected alias after 'as'")?)
                } else {
                    None
                };
                specifiers.push(ImportSpecifier::Named { name, alias });

                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }

            self.consume(&TokenKind::RightBrace, "Expected '}' after named imports")?;
        } else {
            // Default import: import name
            let name = self.consume_identifier("Expected import name")?;
            specifiers.push(ImportSpecifier::Default { name });
        }

        self.consume(&TokenKind::From, "Expected 'from' after import specifiers")?;
        let source = self.consume_string("Expected module path after 'from'")?;

        Ok(StmtKind::Import {
            imports: specifiers,
            module: source,
        })
    }

    fn parse_export_statement(&mut self) -> Result<StmtKind> {
        let kind = if self.match_token(&TokenKind::Async) {
            // Export async function: export async fn name() {}
            self.consume(&TokenKind::Fn, "Expected 'fn' after 'async' in export")?;
            let is_async = true;
            let name = self.consume_identifier("Expected function name")?;
            self.consume(&TokenKind::LeftParen, "Expected '(' after function name")?;

            let mut params = Vec::new();
            if !self.check(&TokenKind::RightParen) {
                loop {
                    let param_name = self.consume_identifier("Expected parameter name")?;
                    self.consume(&TokenKind::Colon, "Expected ':' after parameter name")?;
                    let param_type = self.parse_type_annotation()?;

                    params.push(Param {
                        name: param_name,
                        type_ann: param_type,
                    });

                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
            }

            self.consume(&TokenKind::RightParen, "Expected ')' after parameters")?;

            let ret_type = if self.match_token(&TokenKind::Arrow) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };

            self.consume(&TokenKind::LeftBrace, "Expected '{' before function body")?;
            let body = self.parse_block()?;

            ExportKind::Function {
                name,
                params,
                ret_type,
                body,
                is_async,
            }
        } else if self.match_token(&TokenKind::Fn) {
            // Export function: export fn name() {}
            let is_async = false;
            let name = self.consume_identifier("Expected function name")?;
            self.consume(&TokenKind::LeftParen, "Expected '(' after function name")?;

            let mut params = Vec::new();
            if !self.check(&TokenKind::RightParen) {
                loop {
                    let param_name = self.consume_identifier("Expected parameter name")?;
                    self.consume(&TokenKind::Colon, "Expected ':' after parameter name")?;
                    let param_type = self.parse_type_annotation()?;

                    params.push(Param {
                        name: param_name,
                        type_ann: param_type,
                    });

                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
            }

            self.consume(&TokenKind::RightParen, "Expected ')' after parameters")?;

            let ret_type = if self.match_token(&TokenKind::Arrow) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };

            self.consume(&TokenKind::LeftBrace, "Expected '{' before function body")?;
            let body = self.parse_block()?;

            ExportKind::Function {
                name,
                params,
                ret_type,
                body,
                is_async,
            }
        } else if self.match_token(&TokenKind::Let) {
            // Export variable: export let name = value
            let name = self.consume_identifier("Expected variable name")?;

            let type_ann = if self.match_token(&TokenKind::Colon) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };

            let init = if self.match_token(&TokenKind::Equals) {
                Some(self.parse_expression()?)
            } else {
                None
            };

            ExportKind::Variable {
                name,
                type_ann,
                init,
            }
        } else if self.check(&TokenKind::LeftBrace) {
            // Named exports: export { a, b as c }
            self.advance(); // consume '{'

            let mut names = Vec::new();
            while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
                let name = self.consume_identifier("Expected export name")?;
                let alias = if self.match_token(&TokenKind::As) {
                    Some(self.consume_identifier("Expected alias after 'as'")?)
                } else {
                    None
                };
                names.push(ExportSpecifier { name, alias });

                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }

            self.consume(&TokenKind::RightBrace, "Expected '}' after named exports")?;
            ExportKind::Named { specifiers: names }
        } else {
            // Default export: export default expr
            self.consume_keyword(
                "default",
                "Expected 'default' or declaration after 'export'",
            )?;
            let expr = self.parse_expression()?;
            ExportKind::Default { expr }
        };

        Ok(StmtKind::Export { export: kind })
    }

    fn parse_struct_declaration(&mut self) -> Result<StmtKind> {
        let name = self.consume_identifier("Expected struct name")?;

        // Parse generic parameters if present
        let generic_params = if self.check(&TokenKind::Less) {
            Some(self.parse_generic_parameters()?)
        } else {
            None
        };

        // Parse where clause if present
        let where_clause = if self.check(&TokenKind::Where) {
            Some(self.parse_where_clause()?)
        } else {
            None
        };

        self.consume(&TokenKind::LeftBrace, "Expected '{' after struct name")?;

        let mut fields = Vec::new();

        // Parse struct fields
        while !self.check(&TokenKind::RightBrace) {
            // Skip newlines
            if self.match_token(&TokenKind::Newline) {
                continue;
            }

            let field_start = self.current_location();
            let field_name = self.consume_identifier("Expected field name")?;
            self.consume(&TokenKind::Colon, "Expected ':' after field name")?;
            let field_type = self.parse_type_annotation()?;
            let field_end = self.previous_location();

            fields.push(StructField {
                name: field_name,
                type_ann: field_type,
                span: Span::new(field_start, field_end),
            });

            // Handle comma or newline as field separator
            if !self.check(&TokenKind::RightBrace) {
                if !self.match_token(&TokenKind::Comma) && !self.match_token(&TokenKind::Newline) {
                    return Err(self.error("Expected ',' or newline after struct field"));
                }
                // Skip additional newlines
                while self.match_token(&TokenKind::Newline) {}
            }
        }

        self.consume(&TokenKind::RightBrace, "Expected '}' after struct fields")?;

        Ok(StmtKind::Struct {
            name,
            generic_params,
            fields,
            where_clause,
        })
    }

    fn parse_impl_block(&mut self) -> Result<StmtKind> {
        // Parse generic parameters if present (e.g., impl<T>)
        let generic_params = if self.check(&TokenKind::Less) {
            Some(self.parse_generic_parameters()?)
        } else {
            None
        };

        // Parse the type name (potentially with generic arguments)
        let type_name = self.consume_identifier("Expected type name after 'impl'")?;

        // NOTE: Generic parsing for impl blocks is not yet implemented
        // Currently only simple type names are supported

        // Parse where clause if present
        let where_clause = if self.check(&TokenKind::Where) {
            Some(self.parse_where_clause()?)
        } else {
            None
        };

        self.consume(&TokenKind::LeftBrace, "Expected '{' after impl type")?;

        let mut methods = Vec::new();

        // Parse methods
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            // Skip newlines
            if self.match_token(&TokenKind::Newline) {
                continue;
            }

            // Parse attributes for the method
            let mut attributes = Vec::new();
            while self.check(&TokenKind::At) {
                attributes.push(self.parse_attribute()?);
                // Skip optional newline after attribute
                self.match_token(&TokenKind::Newline);
            }

            // Parse method
            let method = self.parse_method()?;
            methods.push(method);
        }

        self.consume(&TokenKind::RightBrace, "Expected '}' after impl methods")?;

        let impl_start = self
            .tokens
            .get(0)
            .map(|t| t.span.start)
            .unwrap_or_else(|| SourceLocation::initial());
        let impl_block = ImplBlock {
            type_name,
            generic_params,
            methods,
            where_clause,
            span: Span::new(impl_start, self.previous_location()),
        };

        Ok(StmtKind::Impl(impl_block))
    }

    fn parse_enum_declaration(&mut self) -> Result<StmtKind> {
        let name = self.consume_identifier("Expected enum name")?;

        // Parse generic parameters if present
        let generic_params = if self.check(&TokenKind::Less) {
            Some(self.parse_generic_parameters()?)
        } else {
            None
        };

        // Parse where clause if present
        let where_clause = if self.check(&TokenKind::Where) {
            Some(self.parse_where_clause()?)
        } else {
            None
        };

        self.consume(&TokenKind::LeftBrace, "Expected '{' after enum name")?;

        let mut variants = Vec::new();

        // Parse enum variants
        while !self.check(&TokenKind::RightBrace) {
            // Skip newlines
            if self.match_token(&TokenKind::Newline) {
                continue;
            }

            let variant_start = self.current_location();
            let variant_name = self.consume_identifier("Expected variant name")?;

            // Parse variant fields
            let fields = if self.match_token(&TokenKind::LeftParen) {
                // Tuple variant: Some(T), Error(String, i32)
                let mut types = Vec::new();

                if !self.check(&TokenKind::RightParen) {
                    loop {
                        types.push(self.parse_type_annotation()?);

                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                }

                self.consume(
                    &TokenKind::RightParen,
                    "Expected ')' after tuple variant fields",
                )?;
                EnumVariantFields::Tuple(types)
            } else if self.match_token(&TokenKind::LeftBrace) {
                // Struct variant: Point { x: i32, y: i32 }
                let mut fields = Vec::new();

                while !self.check(&TokenKind::RightBrace) {
                    // Skip newlines
                    if self.match_token(&TokenKind::Newline) {
                        continue;
                    }

                    let field_start = self.current_location();
                    let field_name = self.consume_identifier("Expected field name")?;
                    self.consume(&TokenKind::Colon, "Expected ':' after field name")?;
                    let field_type = self.parse_type_annotation()?;
                    let field_end = self.previous_location();

                    fields.push(StructField {
                        name: field_name,
                        type_ann: field_type,
                        span: Span::new(field_start, field_end),
                    });

                    if !self.check(&TokenKind::RightBrace) {
                        if !self.match_token(&TokenKind::Comma)
                            && !self.match_token(&TokenKind::Newline)
                        {
                            return Err(self.error("Expected ',' or newline after struct field"));
                        }
                        // Skip additional newlines
                        while self.match_token(&TokenKind::Newline) {}
                    }
                }

                self.consume(
                    &TokenKind::RightBrace,
                    "Expected '}' after struct variant fields",
                )?;
                EnumVariantFields::Struct(fields)
            } else {
                // Unit variant: None, Empty
                EnumVariantFields::Unit
            };

            let variant_end = self.previous_location();

            variants.push(EnumVariant {
                name: variant_name,
                fields,
                span: Span::new(variant_start, variant_end),
            });

            // Handle comma or newline as variant separator
            if !self.check(&TokenKind::RightBrace) {
                if !self.match_token(&TokenKind::Comma) && !self.match_token(&TokenKind::Newline) {
                    return Err(self.error("Expected ',' or newline after enum variant"));
                }
                // Skip additional newlines
                while self.match_token(&TokenKind::Newline) {}
            }
        }

        self.consume(&TokenKind::RightBrace, "Expected '}' after enum variants")?;

        Ok(StmtKind::Enum {
            name,
            generic_params,
            variants,
            where_clause,
        })
    }

    fn parse_method(&mut self) -> Result<Method> {
        let start = self.current_location();

        // Check for async
        let is_async = self.match_token(&TokenKind::Async);
        if is_async {
            self.consume(&TokenKind::Fn, "Expected 'fn' after 'async'")?;
        } else {
            self.consume(&TokenKind::Fn, "Expected 'fn' for method")?;
        }

        let name = self.consume_identifier("Expected method name")?;

        // Parse generic parameters if present
        let generic_params = if self.check(&TokenKind::Less) {
            Some(self.parse_generic_parameters()?)
        } else {
            None
        };

        self.consume(&TokenKind::LeftParen, "Expected '(' after method name")?;

        let mut params = Vec::new();

        // Parse parameters (including self parameter)
        if !self.check(&TokenKind::RightParen) {
            loop {
                // Check for self parameter
                if let Some(token) = self.peek_identifier() {
                    if token == "self" {
                        self.advance(); // consume "self"

                        // For now, treat self as a special parameter with type "Self"
                        params.push(Param {
                            name: "self".to_string(),
                            type_ann: TypeAnn {
                                kind: TypeKind::Named("Self".to_string()),
                                span: Span::new(self.previous_location(), self.previous_location()),
                            },
                        });
                    } else {
                        // Regular parameter
                        let param_name = self.consume_identifier("Expected parameter name")?;
                        self.consume(&TokenKind::Colon, "Expected ':' after parameter name")?;
                        let param_type = self.parse_type_annotation()?;

                        params.push(Param {
                            name: param_name,
                            type_ann: param_type,
                        });
                    }
                } else {
                    // Regular parameter
                    let param_name = self.consume_identifier("Expected parameter name")?;
                    self.consume(&TokenKind::Colon, "Expected ':' after parameter name")?;
                    let param_type = self.parse_type_annotation()?;

                    params.push(Param {
                        name: param_name,
                        type_ann: param_type,
                    });
                }

                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
        }

        self.consume(&TokenKind::RightParen, "Expected ')' after parameters")?;

        let ret_type = if self.match_token(&TokenKind::Arrow) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        // Parse where clause if present
        let where_clause = if self.check(&TokenKind::Where) {
            Some(self.parse_where_clause()?)
        } else {
            None
        };

        self.consume(&TokenKind::LeftBrace, "Expected '{' before method body")?;
        let body = self.parse_block()?;

        let span = Span::new(start, self.previous_location());
        Ok(Method {
            name,
            generic_params,
            params,
            ret_type,
            where_clause,
            body,
            is_async,
            span,
        })
    }

    fn parse_block(&mut self) -> Result<Block> {
        let mut statements = Vec::new();
        let mut final_expr = None;

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            if self.match_token(&TokenKind::Newline) {
                continue;
            }

            let start_pos = self.current;
            let stmt = self.parse_statement()?;

            // Check if this could be a final expression
            if self.check(&TokenKind::RightBrace) {
                match stmt.kind {
                    StmtKind::Expression(expr) => {
                        // This is a final expression if no semicolon was consumed
                        if self.current == start_pos + 1
                            || (self.current > 0
                                && !matches!(
                                    self.tokens[self.current - 1].kind,
                                    TokenKind::Semicolon
                                ))
                        {
                            final_expr = Some(Box::new(expr));
                            break;
                        } else {
                            statements.push(Stmt {
                                kind: StmtKind::Expression(expr),
                                span: stmt.span,
                                attributes: vec![],
                            });
                        }
                    }
                    _ => {
                        statements.push(stmt);
                    }
                }
            } else {
                statements.push(stmt);
            }
        }

        self.consume(&TokenKind::RightBrace, "Expected '}' after block")?;

        Ok(Block {
            statements,
            final_expr,
        })
    }

    // Expression parsing with Pratt parsing
    pub fn parse_expression(&mut self) -> Result<Expr> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr> {
        let expr = self.parse_or()?;

        if self.match_token(&TokenKind::Equals) {
            let value = self.parse_assignment()?;
            let span = Span::new(expr.span.start, value.span.end);
            return Ok(self.create_expr(
                ExprKind::Assign {
                    target: Box::new(expr),
                    value: Box::new(value),
                },
                span,
            ));
        }

        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Expr> {
        let mut expr = self.parse_and()?;

        while self.match_token(&TokenKind::Or) {
            let op = BinaryOp::Or;
            let right = self.parse_and()?;
            let span = Span::new(expr.span.start, right.span.end);
            expr = self.create_expr(
                ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
                span,
            );
        }

        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr> {
        let mut expr = self.parse_equality()?;

        while self.match_token(&TokenKind::And) {
            let op = BinaryOp::And;
            let right = self.parse_equality()?;
            let span = Span::new(expr.span.start, right.span.end);
            expr = self.create_expr(
                ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
                span,
            );
        }

        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expr> {
        let mut expr = self.parse_comparison()?;

        while let Some(op) = self.match_binary_op(&[TokenKind::EqualsEquals, TokenKind::BangEquals])
        {
            let right = self.parse_comparison()?;
            let span = Span::new(expr.span.start, right.span.end);
            expr = self.create_expr(
                ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
                span,
            );
        }

        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr> {
        let mut expr = self.parse_addition()?;

        while let Some(op) = self.match_binary_op(&[
            TokenKind::Less,
            TokenKind::Greater,
            TokenKind::LessEquals,
            TokenKind::GreaterEquals,
        ]) {
            let right = self.parse_addition()?;
            let span = Span::new(expr.span.start, right.span.end);
            expr = self.create_expr(
                ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
                span,
            );
        }

        Ok(expr)
    }

    fn parse_addition(&mut self) -> Result<Expr> {
        let mut expr = self.parse_multiplication()?;

        while let Some(op) = self.match_binary_op(&[TokenKind::Plus, TokenKind::Minus]) {
            let right = self.parse_multiplication()?;
            let span = Span::new(expr.span.start, right.span.end);
            expr = self.create_expr(
                ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
                span,
            );
        }

        Ok(expr)
    }

    fn parse_multiplication(&mut self) -> Result<Expr> {
        let mut expr = self.parse_unary()?;

        while let Some(op) =
            self.match_binary_op(&[TokenKind::Star, TokenKind::Slash, TokenKind::Percent])
        {
            let right = self.parse_unary()?;
            let span = Span::new(expr.span.start, right.span.end);
            expr = self.create_expr(
                ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
                span,
            );
        }

        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr> {
        let start = self.current_location();

        if let Some(op) = self.match_unary_op(&[TokenKind::Bang, TokenKind::Minus]) {
            let expr = self.parse_unary()?;
            let span = Span::new(start, expr.span.end);
            return Ok(self.create_expr(
                ExprKind::Unary {
                    op,
                    expr: Box::new(expr),
                },
                span,
            ));
        }

        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.match_token(&TokenKind::LeftParen) {
                expr = self.finish_call(expr)?;
            } else if self.match_token(&TokenKind::LeftBracket) {
                let index = self.parse_expression()?;
                self.consume(&TokenKind::RightBracket, "Expected ']' after array index")?;
                let span = Span::new(expr.span.start, self.previous_location());
                expr = self.create_expr(
                    ExprKind::Index {
                        object: Box::new(expr),
                        index: Box::new(index),
                    },
                    span,
                );
            } else if self.match_token(&TokenKind::Dot) {
                let property = self.consume_identifier("Expected property name after '.'")?;
                let span = Span::new(expr.span.start, self.previous_location());
                expr = self.create_expr(
                    ExprKind::Member {
                        object: Box::new(expr),
                        property,
                    },
                    span,
                );
            } else if self.match_token(&TokenKind::Question) {
                let span = Span::new(expr.span.start, self.previous_location());
                expr = self.create_expr(
                    ExprKind::ErrorPropagation {
                        expr: Box::new(expr),
                    },
                    span,
                );
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn finish_call(&mut self, callee: Expr) -> Result<Expr> {
        let mut args = Vec::new();

        if !self.check(&TokenKind::RightParen) {
            loop {
                args.push(self.parse_expression()?);
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
        }

        self.consume(&TokenKind::RightParen, "Expected ')' after arguments")?;
        let span = Span::new(callee.span.start, self.previous_location());

        Ok(self.create_expr(
            ExprKind::Call {
                callee: Box::new(callee),
                args,
            },
            span,
        ))
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        let start = self.current_location();

        // Numbers
        if let Some(token) = self.match_token_if(|t| matches!(t, TokenKind::Number(_))) {
            if let TokenKind::Number(n) = token.kind {
                let span = Span::new(start, self.previous_location());
                return Ok(self.create_expr(ExprKind::Literal(Literal::Number(n)), span));
            }
        }

        // Strings
        if let Some(token) = self.match_token_if(|t| matches!(t, TokenKind::String(_))) {
            if let TokenKind::String(s) = token.kind {
                let span = Span::new(start, self.previous_location());
                return Ok(self.create_expr(ExprKind::Literal(Literal::String(s)), span));
            }
        }

        // Booleans
        if self.match_token(&TokenKind::True) {
            let span = Span::new(start, self.previous_location());
            return Ok(self.create_expr(ExprKind::Literal(Literal::Boolean(true)), span));
        }

        if self.match_token(&TokenKind::False) {
            let span = Span::new(start, self.previous_location());
            return Ok(self.create_expr(ExprKind::Literal(Literal::Boolean(false)), span));
        }

        // Identifiers (including generic constructors like Vec<T>)
        if let Some(token) = self.match_token_if(|t| matches!(t, TokenKind::Identifier(_))) {
            if let TokenKind::Identifier(name) = token.kind {
                // Check for generic type arguments
                if self.check(&TokenKind::Less) {
                    // This is a generic constructor like Vec<i32>
                    let type_args = self.parse_generic_args()?;
                    let span = Span::new(start, self.previous_location());
                    return Ok(
                        self.create_expr(ExprKind::GenericConstructor { name, type_args }, span)
                    );
                } else {
                    // Regular identifier
                    let span = Span::new(start, self.previous_location());
                    return Ok(self.create_expr(ExprKind::Identifier(name), span));
                }
            }
        }

        // Grouped expressions
        if self.match_token(&TokenKind::LeftParen) {
            let expr = self.parse_expression()?;
            self.consume(&TokenKind::RightParen, "Expected ')' after expression")?;
            return Ok(expr);
        }

        // Closure expressions (|param| body)
        if self.match_token(&TokenKind::Pipe) {
            return self.parse_closure_expression(start);
        }

        // If expressions
        if self.match_token(&TokenKind::If) {
            return self.parse_if_expression();
        }

        // Match expressions
        if self.match_token(&TokenKind::Match) {
            return self.parse_match_expression();
        }

        // Await expressions
        if self.match_token(&TokenKind::Await) {
            let expr = Box::new(self.parse_expression()?);
            let span = Span::new(start, self.previous_location());
            return Ok(self.create_expr(ExprKind::Await { expr }, span));
        }

        // Try-catch expressions
        if self.match_token(&TokenKind::Try) {
            return self.parse_try_catch_expression();
        }

        // Block expressions
        if self.match_token(&TokenKind::LeftBrace) {
            let block = self.parse_block()?;
            let span = Span::new(start, self.previous_location());
            return Ok(self.create_expr(ExprKind::Block(block), span));
        }

        // Array literals and list comprehensions
        if self.match_token(&TokenKind::LeftBracket) {
            // Try to parse as a list comprehension first
            let checkpoint = self.current;

            if !self.check(&TokenKind::RightBracket) {
                // Parse first element
                let first_expr = self.parse_expression()?;

                // Check if it's a list comprehension
                if self.match_token(&TokenKind::For) {
                    // It's a list comprehension: [expr for var in iterable if condition]
                    let variable = self.consume_identifier("Expected variable name after 'for'")?;
                    self.consume(&TokenKind::In, "Expected 'in' after variable")?;
                    let iterable = Box::new(self.parse_expression()?);

                    let condition = if self.match_token(&TokenKind::If) {
                        Some(Box::new(self.parse_expression()?))
                    } else {
                        None
                    };

                    self.consume(
                        &TokenKind::RightBracket,
                        "Expected ']' after list comprehension",
                    )?;
                    let span = Span::new(start, self.previous_location());
                    return Ok(self.create_expr(
                        ExprKind::ListComprehension {
                            element: Box::new(first_expr),
                            variable,
                            iterable,
                            condition,
                        },
                        span,
                    ));
                } else {
                    // It's a regular array literal
                    let mut elements = vec![first_expr];

                    while self.match_token(&TokenKind::Comma) {
                        if self.check(&TokenKind::RightBracket) {
                            break;
                        }
                        elements.push(self.parse_expression()?);
                    }

                    self.consume(
                        &TokenKind::RightBracket,
                        "Expected ']' after array elements",
                    )?;
                    let span = Span::new(start, self.previous_location());
                    return Ok(self.create_expr(ExprKind::Array(elements), span));
                }
            } else {
                // Empty array
                self.consume(
                    &TokenKind::RightBracket,
                    "Expected ']' after array elements",
                )?;
                let span = Span::new(start, self.previous_location());
                return Ok(self.create_expr(ExprKind::Array(Vec::new()), span));
            }
        }

        Err(self.error("Expected expression"))
    }

    fn parse_if_expression(&mut self) -> Result<Expr> {
        let start = self.tokens[self.current - 1].span.start; // 'if' was already consumed

        let condition = Box::new(self.parse_expression()?);
        self.consume(&TokenKind::LeftBrace, "Expected '{' after if condition")?;

        // Parse the then branch as a block
        let block = self.parse_block()?;
        let then_branch = Box::new(self.create_expr(
            ExprKind::Block(block),
            Span::new(
                self.tokens[self.current - 1].span.start,
                self.previous_location(),
            ),
        ));

        let else_branch = if self.match_token(&TokenKind::Else) {
            if self.match_token(&TokenKind::If) {
                // else if
                Some(Box::new(self.parse_if_expression()?))
            } else {
                self.consume(&TokenKind::LeftBrace, "Expected '{' after else")?;
                let block = self.parse_block()?;
                Some(Box::new(self.create_expr(
                    ExprKind::Block(block),
                    Span::new(
                        self.tokens[self.current - 1].span.start,
                        self.previous_location(),
                    ),
                )))
            }
        } else {
            None
        };

        let span = Span::new(start, self.previous_location());

        Ok(self.create_expr(
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            },
            span,
        ))
    }

    fn parse_match_expression(&mut self) -> Result<Expr> {
        let start = self.tokens[self.current - 1].span.start; // 'match' was already consumed

        let expr = Box::new(self.parse_expression()?);
        self.consume(&TokenKind::LeftBrace, "Expected '{' after match expression")?;

        let mut arms = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            // Skip newlines before parsing patterns
            while self.match_token(&TokenKind::Newline) {
                // Continue skipping newlines
            }

            // Check if we've reached the end after skipping newlines
            if self.check(&TokenKind::RightBrace) || self.is_at_end() {
                break;
            }

            let pattern = self.parse_pattern()?;

            // Optional guard
            let guard = if self.match_token(&TokenKind::If) {
                Some(self.parse_expression()?)
            } else {
                None
            };

            self.consume(&TokenKind::DoubleArrow, "Expected '=>' after match pattern")?;
            let body = self.parse_expression()?;

            arms.push(MatchArm {
                pattern,
                guard,
                body,
            });

            // Allow optional comma after match arm
            if self.check(&TokenKind::Comma) {
                self.advance();
            }
        }

        self.consume(&TokenKind::RightBrace, "Expected '}' after match arms")?;
        let span = Span::new(start, self.previous_location());

        Ok(self.create_expr(ExprKind::Match { expr, arms }, span))
    }

    fn parse_try_catch_expression(&mut self) -> Result<Expr> {
        let start = self.tokens[self.current - 1].span.start; // 'try' was already consumed

        // Parse the try block
        self.consume(&TokenKind::LeftBrace, "Expected '{' after 'try'")?;
        let try_block = self.parse_block()?;
        let try_expr = Box::new(self.create_expr(
            ExprKind::Block(try_block),
            Span::new(start, self.previous_location()),
        ));

        let mut catch_clauses = Vec::new();

        // Parse catch clauses
        while self.match_token(&TokenKind::Catch) {
            catch_clauses.push(self.parse_catch_clause()?);
        }

        // Parse optional finally block
        let finally_block = if self.match_token(&TokenKind::Finally) {
            self.consume(&TokenKind::LeftBrace, "Expected '{' after 'finally'")?;
            Some(self.parse_block()?)
        } else {
            None
        };

        // Require at least one catch clause if no finally block
        if catch_clauses.is_empty() && finally_block.is_none() {
            return Err(
                self.error("try expression must have at least one catch clause or a finally block")
            );
        }

        let span = Span::new(start, self.previous_location());
        Ok(self.create_expr(
            ExprKind::TryCatch {
                try_expr,
                catch_clauses,
                finally_block,
            },
            span,
        ))
    }

    fn parse_catch_clause(&mut self) -> Result<CatchClause> {
        let start = self.current_location();

        let mut var = None;
        let mut error_type = None;
        let mut condition = None;

        // Parse optional variable binding and type constraint
        if self.match_token(&TokenKind::LeftParen) {
            // Parse variable name
            var = Some(self.consume_identifier("Expected variable name in catch clause")?);

            // Parse optional type constraint
            if self.match_token(&TokenKind::Colon) {
                error_type = Some(self.parse_type_annotation()?);
            }

            self.consume(
                &TokenKind::RightParen,
                "Expected ')' after catch parameters",
            )?;
        }

        // Parse optional condition
        if self.match_token(&TokenKind::If) {
            condition = Some(self.parse_expression()?);
        }

        // Parse catch body
        self.consume(&TokenKind::LeftBrace, "Expected '{' after catch clause")?;
        let handler = self.parse_block()?;

        let span = Span::new(start, self.previous_location());
        Ok(CatchClause {
            var,
            error_type,
            condition,
            handler,
            span,
        })
    }

    fn parse_pattern(&mut self) -> Result<Pattern> {
        self.parse_or_pattern()
    }

    fn parse_or_pattern(&mut self) -> Result<Pattern> {
        let start = self.current_location();
        let mut patterns = vec![self.parse_primary_pattern()?];

        // Check for or-pattern (|)
        while self.match_token(&TokenKind::Pipe) {
            patterns.push(self.parse_primary_pattern()?);
        }

        if patterns.len() == 1 {
            Ok(patterns.into_iter().next().unwrap())
        } else {
            let span = Span::new(start, self.previous_location());
            Ok(Pattern {
                kind: PatternKind::Or(patterns),
                span,
            })
        }
    }

    fn parse_primary_pattern(&mut self) -> Result<Pattern> {
        let start = self.current_location();

        // Wildcard pattern
        if self.match_token(&TokenKind::Underscore) {
            let span = Span::new(start, self.previous_location());
            return Ok(Pattern {
                kind: PatternKind::Wildcard,
                span,
            });
        }

        // Literal patterns
        if let Some(token) = self.match_token_if(|t| matches!(t, TokenKind::Number(_))) {
            if let TokenKind::Number(n) = token.kind {
                let span = Span::new(start, self.previous_location());
                return Ok(Pattern {
                    kind: PatternKind::Literal(Literal::Number(n)),
                    span,
                });
            }
        }

        if let Some(token) = self.match_token_if(|t| matches!(t, TokenKind::String(_))) {
            if let TokenKind::String(s) = token.kind {
                let span = Span::new(start, self.previous_location());
                return Ok(Pattern {
                    kind: PatternKind::Literal(Literal::String(s)),
                    span,
                });
            }
        }

        if self.match_token(&TokenKind::True) {
            let span = Span::new(start, self.previous_location());
            return Ok(Pattern {
                kind: PatternKind::Literal(Literal::Boolean(true)),
                span,
            });
        }

        if self.match_token(&TokenKind::False) {
            let span = Span::new(start, self.previous_location());
            return Ok(Pattern {
                kind: PatternKind::Literal(Literal::Boolean(false)),
                span,
            });
        }

        // Array destructuring
        if self.match_token(&TokenKind::LeftBracket) {
            let mut patterns = Vec::new();

            if !self.check(&TokenKind::RightBracket) {
                loop {
                    patterns.push(self.parse_primary_pattern()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
            }

            self.consume(&TokenKind::RightBracket, "Expected ']' after array pattern")?;
            let span = Span::new(start, self.previous_location());

            return Ok(Pattern {
                kind: PatternKind::Array(patterns),
                span,
            });
        }

        // Object destructuring
        if self.match_token(&TokenKind::LeftBrace) {
            let mut fields = Vec::new();

            if !self.check(&TokenKind::RightBrace) {
                loop {
                    if let Some(token) =
                        self.match_token_if(|t| matches!(t, TokenKind::Identifier(_)))
                    {
                        if let TokenKind::Identifier(key) = token.kind {
                            let pattern = if self.match_token(&TokenKind::Colon) {
                                Some(self.parse_primary_pattern()?)
                            } else {
                                None
                            };

                            fields.push((key, pattern));
                        }
                    } else {
                        return Err(self.error("Expected identifier in object pattern"));
                    }

                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
            }

            self.consume(&TokenKind::RightBrace, "Expected '}' after object pattern")?;
            let span = Span::new(start, self.previous_location());

            return Ok(Pattern {
                kind: PatternKind::Object(fields),
                span,
            });
        }

        // Identifier pattern or enum constructor
        if let Some(token) = self.match_token_if(|t| matches!(t, TokenKind::Identifier(_))) {
            if let TokenKind::Identifier(name) = token.kind {
                // Check if this is an enum constructor pattern
                // Could be: Some(x), Option::Some(x), or just Some/None

                let mut enum_name = None;
                let mut variant = name.clone();

                // Check for qualified enum pattern like Option::Some
                if self.match_token(&TokenKind::ColonColon) {
                    enum_name = Some(name);

                    if let Some(token) =
                        self.match_token_if(|t| matches!(t, TokenKind::Identifier(_)))
                    {
                        if let TokenKind::Identifier(v) = token.kind {
                            variant = v;
                        }
                    } else {
                        return Err(self.error("Expected variant name after '::'"));
                    }
                }

                // Check if this is a constructor with arguments
                if self.check(&TokenKind::LeftParen) {
                    self.advance(); // consume '('

                    let mut args = Vec::new();
                    if !self.check(&TokenKind::RightParen) {
                        loop {
                            args.push(self.parse_pattern()?);
                            if !self.match_token(&TokenKind::Comma) {
                                break;
                            }
                        }
                    }

                    self.consume(
                        &TokenKind::RightParen,
                        "Expected ')' after enum constructor arguments",
                    )?;
                    let span = Span::new(start, self.previous_location());

                    return Ok(Pattern {
                        kind: PatternKind::EnumConstructor {
                            enum_name,
                            variant,
                            args: Some(args),
                        },
                        span,
                    });
                }

                // Check if this could be a unit variant (capitalized identifier)
                // Heuristic: if it starts with uppercase, treat as enum constructor
                if variant
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                {
                    let span = Span::new(start, self.previous_location());
                    return Ok(Pattern {
                        kind: PatternKind::EnumConstructor {
                            enum_name,
                            variant,
                            args: None,
                        },
                        span,
                    });
                }

                // Otherwise, it's just a regular identifier pattern
                let span = Span::new(start, self.previous_location());
                return Ok(Pattern {
                    kind: PatternKind::Identifier(variant),
                    span,
                });
            }
        }

        Err(self.error("Expected pattern"))
    }

    fn parse_type_annotation(&mut self) -> Result<TypeAnn> {
        let start = self.current_location();

        // Reference type
        if self.match_token(&TokenKind::Ampersand) {
            let mutable = self.match_token(&TokenKind::Mut);
            let inner = self.parse_type_annotation()?;
            let span = Span::new(start, self.previous_location());
            return Ok(TypeAnn {
                kind: TypeKind::Reference {
                    mutable,
                    inner: Box::new(inner),
                },
                span,
            });
        }

        // Array type
        if self.match_token(&TokenKind::LeftBracket) {
            let elem_type = self.parse_type_annotation()?;
            self.consume(
                &TokenKind::RightBracket,
                "Expected ']' after array element type",
            )?;
            let span = Span::new(start, self.previous_location());
            return Ok(TypeAnn {
                kind: TypeKind::Array(Box::new(elem_type)),
                span,
            });
        }

        // Function type or Tuple type
        if self.match_token(&TokenKind::LeftParen) {
            let mut types = Vec::new();

            if !self.check(&TokenKind::RightParen) {
                loop {
                    types.push(self.parse_type_annotation()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
            }

            self.consume(&TokenKind::RightParen, "Expected ')' after types")?;

            // Check if this is a function type (has arrow)
            if self.check(&TokenKind::Arrow) {
                self.advance(); // consume arrow
                let ret = Box::new(self.parse_type_annotation()?);
                let span = Span::new(start, self.previous_location());
                return Ok(TypeAnn {
                    kind: TypeKind::Function { params: types, ret },
                    span,
                });
            } else {
                // It's a tuple type
                let span = Span::new(start, self.previous_location());
                return Ok(TypeAnn {
                    kind: TypeKind::Tuple(types),
                    span,
                });
            }
        }

        // Named type, Generic type, or Type parameter
        let name = self.consume_identifier("Expected type name")?;

        // Check for generic type arguments
        if self.check(&TokenKind::Less) {
            // This is a generic type like Vec<T> or HashMap<K, V>
            let args = self.parse_generic_args()?;
            let span = Span::new(start, self.previous_location());

            Ok(TypeAnn {
                kind: TypeKind::Generic { name, args },
                span,
            })
        } else {
            // Determine if this should be a type parameter or named type
            let span = Span::new(start, self.previous_location());
            let kind = if self.is_type_parameter(&name) {
                TypeKind::TypeParam(name)
            } else {
                TypeKind::Named(name)
            };

            Ok(TypeAnn { kind, span })
        }
    }

    /// Parse generic type arguments: <T>, <K, V>, <Option<T>, Result<E>>
    fn parse_generic_args(&mut self) -> Result<Vec<TypeAnn>> {
        self.consume(&TokenKind::Less, "Expected '<' to start generic arguments")?;

        let mut args = Vec::new();

        // Handle empty generic args case: Foo<>
        if self.check(&TokenKind::Greater) {
            self.advance(); // consume '>'
            return Ok(args);
        }

        // Parse first type argument
        args.push(self.parse_type_annotation()?);

        // Parse remaining type arguments
        while self.match_token(&TokenKind::Comma) {
            // Allow trailing comma: Foo<T,>
            if self.check(&TokenKind::Greater) {
                break;
            }
            args.push(self.parse_type_annotation()?);
        }

        self.consume(
            &TokenKind::Greater,
            "Expected '>' to close generic arguments",
        )?;

        Ok(args)
    }

    /// Determine if an identifier should be treated as a type parameter
    /// Heuristic: single uppercase letter, or starts with uppercase and is short (<=3 chars)
    /// Common patterns: T, U, K, V, TKey, TValue, etc.
    fn is_type_parameter(&self, name: &str) -> bool {
        if name.is_empty() {
            return false;
        }

        // Single uppercase letter (T, U, K, V, etc.)
        if name.len() == 1 {
            return name.chars().next().unwrap().is_uppercase();
        }

        // Short names starting with uppercase (TKey, TValue, etc.)
        if name.len() <= 3 && name.chars().next().unwrap().is_uppercase() {
            return true;
        }

        // Common type parameter patterns
        matches!(
            name,
            "T" | "U"
                | "K"
                | "V"
                | "E"
                | "R"
                | "Self"
                | "TKey"
                | "TValue"
                | "TItem"
                | "TResult"
                | "TError"
        )
    }

    /// Parse where clause: where T: Clone, U: Debug + Send
    fn parse_where_clause(&mut self) -> Result<WhereClause> {
        let start = self.current_location();
        self.consume(&TokenKind::Where, "Expected 'where'")?;

        let mut predicates = Vec::new();

        // Parse first where predicate
        predicates.push(self.parse_where_predicate()?);

        // Parse remaining where predicates
        while self.match_token(&TokenKind::Comma) {
            // Allow trailing comma: where T: Clone,
            if !self.check(&TokenKind::LeftBrace) && !self.is_at_end() {
                predicates.push(self.parse_where_predicate()?);
            } else {
                break;
            }
        }

        let span = Span::new(start, self.previous_location());
        Ok(WhereClause { predicates, span })
    }

    /// Parse a where predicate: T: Clone + Send, U: Debug
    fn parse_where_predicate(&mut self) -> Result<WherePredicate> {
        let start = self.current_location();

        // Parse the type (usually a type parameter)
        let type_ = self.parse_type_annotation()?;
        self.consume(&TokenKind::Colon, "Expected ':' after type")?;

        let mut bounds = Vec::new();

        // Parse first trait bound
        bounds.push(self.parse_trait_bound()?);

        // Parse additional bounds with '+'
        while self.match_token(&TokenKind::Plus) {
            bounds.push(self.parse_trait_bound()?);
        }

        let span = Span::new(start, self.previous_location());
        Ok(WherePredicate {
            type_,
            bounds,
            span,
        })
    }

    // Helper methods
    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn match_token_if<F>(&mut self, predicate: F) -> Option<Token>
    where
        F: Fn(&TokenKind) -> bool,
    {
        if !self.is_at_end() && predicate(&self.peek().kind) {
            Some(self.advance())
        } else {
            None
        }
    }

    fn match_binary_op(&mut self, kinds: &[TokenKind]) -> Option<BinaryOp> {
        for kind in kinds {
            if self.check(kind) {
                let op = BinaryOp::from_token(kind)?;
                self.advance();
                return Some(op);
            }
        }
        None
    }

    fn match_unary_op(&mut self, kinds: &[TokenKind]) -> Option<UnaryOp> {
        for kind in kinds {
            if self.check(kind) {
                let op = UnaryOp::from_token(kind)?;
                self.advance();
                return Some(op);
            }
        }
        None
    }

    fn parse_attribute(&mut self) -> Result<Attribute> {
        let start = self.current_location();
        self.consume(&TokenKind::At, "Expected '@'")?;

        let name = self.consume_identifier("Expected attribute name")?;
        let mut args = Vec::new();

        if self.match_token(&TokenKind::LeftParen) {
            while !self.check(&TokenKind::RightParen) && !self.is_at_end() {
                // Parse either a simple identifier or key=value pair
                let arg_start = self.current;
                let key = self.consume_identifier("Expected identifier in attribute arguments")?;

                let arg = if self.match_token(&TokenKind::Equals) {
                    // Parse value - can be string literal or identifier
                    let token_kind = self.peek().kind.clone();
                    match token_kind {
                        TokenKind::String(value) => {
                            self.advance();
                            format!("{} = \"{}\"", key, value)
                        }
                        TokenKind::Identifier(value) => {
                            self.advance();
                            format!("{} = {}", key, value)
                        }
                        TokenKind::Number(n) => {
                            self.advance();
                            format!("{} = {}", key, n)
                        }
                        _ => {
                            return Err(self.error("Expected value after '=' in attribute"));
                        }
                    }
                } else {
                    // Just a simple identifier
                    key
                };

                args.push(arg);

                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
            self.consume(
                &TokenKind::RightParen,
                "Expected ')' after attribute arguments",
            )?;
        }

        let end = self.previous_location();
        Ok(Attribute {
            name,
            args,
            span: Span::new(start, end),
        })
    }

    fn check(&self, kind: &TokenKind) -> bool {
        if self.is_at_end() {
            false
        } else {
            std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
        }
    }

    fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn peek_identifier(&self) -> Option<String> {
        if let TokenKind::Identifier(ref name) = self.peek().kind {
            Some(name.clone())
        } else {
            None
        }
    }

    fn previous(&self) -> Token {
        self.tokens[self.current - 1].clone()
    }

    fn consume(&mut self, kind: &TokenKind, message: &str) -> Result<Token> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(self.error(message))
        }
    }

    fn consume_identifier(&mut self, message: &str) -> Result<String> {
        if let Some(token) = self.match_token_if(|t| matches!(t, TokenKind::Identifier(_))) {
            if let TokenKind::Identifier(name) = token.kind {
                return Ok(name);
            }
        }
        Err(self.error(message))
    }

    fn consume_string(&mut self, message: &str) -> Result<String> {
        if let Some(token) = self.match_token_if(|t| matches!(t, TokenKind::String(_))) {
            if let TokenKind::String(s) = token.kind {
                return Ok(s);
            }
        }
        Err(self.error(message))
    }

    fn consume_keyword(&mut self, keyword: &str, message: &str) -> Result<()> {
        if let Some(_token) =
            self.match_token_if(|t| matches!(t, TokenKind::Identifier(ref name) if name == keyword))
        {
            return Ok(());
        }
        Err(self.error(message))
    }

    fn current_location(&self) -> SourceLocation {
        self.peek().span.start
    }

    fn previous_location(&self) -> SourceLocation {
        self.previous().span.end
    }

    fn error(&self, message: &str) -> Error {
        let location = if self.is_at_end() {
            self.previous().span.end
        } else {
            self.peek().span.start
        };

        Error::new(ErrorKind::ParseError, message).with_location(location)
    }

    /// Parse a closure expression: |param1, param2| body
    fn parse_closure_expression(&mut self, start: SourceLocation) -> Result<Expr> {
        let mut parameters = Vec::new();

        // Parse parameters
        if !self.check(&TokenKind::Pipe) {
            loop {
                // Parse parameter name
                let param_name = self.consume_identifier("Expected parameter name")?;

                // Check for optional type annotation
                let type_ann = if self.match_token(&TokenKind::Colon) {
                    Some(self.parse_type_annotation()?)
                } else {
                    None
                };

                parameters.push(ClosureParam {
                    name: param_name,
                    type_ann,
                });

                // Check for more parameters
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
        }

        // Consume closing pipe
        self.consume(&TokenKind::Pipe, "Expected '|' after closure parameters")?;

        // Parse closure body
        let body = Box::new(self.parse_expression()?);

        let span = Span::new(start, self.previous_location());
        Ok(self.create_expr(ExprKind::Closure { parameters, body }, span))
    }

    // Error recovery
    #[allow(dead_code)]
    fn synchronize(&mut self) {
        self.advance();

        while !self.is_at_end() {
            if matches!(
                self.previous().kind,
                TokenKind::Semicolon | TokenKind::Newline
            ) {
                return;
            }

            match self.peek().kind {
                TokenKind::Fn
                | TokenKind::Let
                | TokenKind::For
                | TokenKind::If
                | TokenKind::While
                | TokenKind::Return => return,
                _ => {}
            }

            self.advance();
        }
    }
}
