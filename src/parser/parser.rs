use crate::{
    error::{Error, ErrorKind, Result},
    lexer::{Token, TokenKind},
    source::{Span, SourceLocation},
};
use super::ast::*;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
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
        
        let kind = if self.match_token(&TokenKind::Let) {
            self.parse_let_statement()?
        } else if self.match_token(&TokenKind::Fn) {
            self.parse_function_statement()?
        } else if self.match_token(&TokenKind::Return) {
            self.parse_return_statement()?
        } else if self.match_token(&TokenKind::While) {
            self.parse_while_statement()?
        } else if self.match_token(&TokenKind::For) {
            self.parse_for_statement()?
        } else {
            StmtKind::Expression(self.parse_expression()?)
        };
        
        let end = self.previous_location();
        let span = Span::new(start, end);
        
        // Consume optional semicolon or newline
        let _ = self.match_token(&TokenKind::Semicolon) || self.match_token(&TokenKind::Newline);
        
        Ok(Stmt { kind, span })
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
        
        Ok(StmtKind::Let { name, type_ann, init })
    }

    fn parse_function_statement(&mut self) -> Result<StmtKind> {
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
        
        Ok(StmtKind::Function { name, params, ret_type, body })
    }

    fn parse_return_statement(&mut self) -> Result<StmtKind> {
        let expr = if self.check(&TokenKind::Semicolon) || self.check(&TokenKind::Newline) || self.is_at_end() {
            None
        } else {
            Some(self.parse_expression()?)
        };
        
        Ok(StmtKind::Return(expr))
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
        if self.match_token_if(|t| matches!(t, TokenKind::Identifier(s) if s == "in")).is_none() {
            return Err(self.error("Expected 'in' in for loop"));
        }
        
        let iterable = self.parse_expression()?;
        self.consume(&TokenKind::LeftBrace, "Expected '{' after for loop header")?;
        let body = self.parse_block()?;
        
        Ok(StmtKind::For { variable, iterable, body })
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
                        if self.current == start_pos + 1 || 
                           (self.current > 0 && !matches!(self.tokens[self.current - 1].kind, TokenKind::Semicolon)) {
                            final_expr = Some(Box::new(expr));
                            break;
                        } else {
                            statements.push(Stmt {
                                kind: StmtKind::Expression(expr),
                                span: stmt.span,
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
        
        Ok(Block { statements, final_expr })
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
            return Ok(Expr {
                kind: ExprKind::Assign {
                    target: Box::new(expr),
                    value: Box::new(value),
                },
                span,
            });
        }
        
        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Expr> {
        let mut expr = self.parse_and()?;
        
        while self.match_token(&TokenKind::Or) {
            let op = BinaryOp::Or;
            let right = self.parse_and()?;
            let span = Span::new(expr.span.start, right.span.end);
            expr = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
                span,
            };
        }
        
        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr> {
        let mut expr = self.parse_equality()?;
        
        while self.match_token(&TokenKind::And) {
            let op = BinaryOp::And;
            let right = self.parse_equality()?;
            let span = Span::new(expr.span.start, right.span.end);
            expr = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
                span,
            };
        }
        
        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expr> {
        let mut expr = self.parse_comparison()?;
        
        while let Some(op) = self.match_binary_op(&[TokenKind::EqualsEquals, TokenKind::BangEquals]) {
            let right = self.parse_comparison()?;
            let span = Span::new(expr.span.start, right.span.end);
            expr = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
                span,
            };
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
            expr = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
                span,
            };
        }
        
        Ok(expr)
    }

    fn parse_addition(&mut self) -> Result<Expr> {
        let mut expr = self.parse_multiplication()?;
        
        while let Some(op) = self.match_binary_op(&[TokenKind::Plus, TokenKind::Minus]) {
            let right = self.parse_multiplication()?;
            let span = Span::new(expr.span.start, right.span.end);
            expr = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
                span,
            };
        }
        
        Ok(expr)
    }

    fn parse_multiplication(&mut self) -> Result<Expr> {
        let mut expr = self.parse_unary()?;
        
        while let Some(op) = self.match_binary_op(&[TokenKind::Star, TokenKind::Slash, TokenKind::Percent]) {
            let right = self.parse_unary()?;
            let span = Span::new(expr.span.start, right.span.end);
            expr = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
                span,
            };
        }
        
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr> {
        let start = self.current_location();
        
        if let Some(op) = self.match_unary_op(&[TokenKind::Bang, TokenKind::Minus]) {
            let expr = self.parse_unary()?;
            let span = Span::new(start, expr.span.end);
            return Ok(Expr {
                kind: ExprKind::Unary {
                    op,
                    expr: Box::new(expr),
                },
                span,
            });
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
                expr = Expr {
                    kind: ExprKind::Index {
                        object: Box::new(expr),
                        index: Box::new(index),
                    },
                    span,
                };
            } else if self.match_token(&TokenKind::Dot) {
                let property = self.consume_identifier("Expected property name after '.'")?;
                let span = Span::new(expr.span.start, self.previous_location());
                expr = Expr {
                    kind: ExprKind::Member {
                        object: Box::new(expr),
                        property,
                    },
                    span,
                };
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
        
        Ok(Expr {
            kind: ExprKind::Call {
                callee: Box::new(callee),
                args,
            },
            span,
        })
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        let start = self.current_location();
        
        // Numbers
        if let Some(token) = self.match_token_if(|t| matches!(t, TokenKind::Number(_))) {
            if let TokenKind::Number(n) = token.kind {
                let span = Span::new(start, self.previous_location());
                return Ok(Expr {
                    kind: ExprKind::Literal(Literal::Number(n)),
                    span,
                });
            }
        }
        
        // Strings
        if let Some(token) = self.match_token_if(|t| matches!(t, TokenKind::String(_))) {
            if let TokenKind::String(s) = token.kind {
                let span = Span::new(start, self.previous_location());
                return Ok(Expr {
                    kind: ExprKind::Literal(Literal::String(s)),
                    span,
                });
            }
        }
        
        // Booleans
        if self.match_token(&TokenKind::True) {
            let span = Span::new(start, self.previous_location());
            return Ok(Expr {
                kind: ExprKind::Literal(Literal::Boolean(true)),
                span,
            });
        }
        
        if self.match_token(&TokenKind::False) {
            let span = Span::new(start, self.previous_location());
            return Ok(Expr {
                kind: ExprKind::Literal(Literal::Boolean(false)),
                span,
            });
        }
        
        // Identifiers
        if let Some(token) = self.match_token_if(|t| matches!(t, TokenKind::Identifier(_))) {
            if let TokenKind::Identifier(name) = token.kind {
                let span = Span::new(start, self.previous_location());
                return Ok(Expr {
                    kind: ExprKind::Identifier(name),
                    span,
                });
            }
        }
        
        // Grouped expressions
        if self.match_token(&TokenKind::LeftParen) {
            let expr = self.parse_expression()?;
            self.consume(&TokenKind::RightParen, "Expected ')' after expression")?;
            return Ok(expr);
        }
        
        // If expressions
        if self.match_token(&TokenKind::If) {
            return self.parse_if_expression();
        }
        
        // Match expressions
        if self.match_token(&TokenKind::Match) {
            return self.parse_match_expression();
        }
        
        // Block expressions
        if self.match_token(&TokenKind::LeftBrace) {
            let block = self.parse_block()?;
            let span = Span::new(start, self.previous_location());
            return Ok(Expr {
                kind: ExprKind::Block(block),
                span,
            });
        }
        
        // Array literals
        if self.match_token(&TokenKind::LeftBracket) {
            let mut elements = Vec::new();
            
            if !self.check(&TokenKind::RightBracket) {
                loop {
                    elements.push(self.parse_expression()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
            }
            
            self.consume(&TokenKind::RightBracket, "Expected ']' after array elements")?;
            let span = Span::new(start, self.previous_location());
            
            return Ok(Expr {
                kind: ExprKind::Array(elements),
                span,
            });
        }
        
        Err(self.error("Expected expression"))
    }

    fn parse_if_expression(&mut self) -> Result<Expr> {
        let start = self.tokens[self.current - 1].span.start; // 'if' was already consumed
        
        let condition = Box::new(self.parse_expression()?);
        self.consume(&TokenKind::LeftBrace, "Expected '{' after if condition")?;
        
        // Parse the then branch as a block
        let block = self.parse_block()?;
        let then_branch = Box::new(Expr {
            kind: ExprKind::Block(block),
            span: Span::new(self.tokens[self.current - 1].span.start, self.previous_location()),
        });
        
        let else_branch = if self.match_token(&TokenKind::Else) {
            if self.match_token(&TokenKind::If) {
                // else if
                Some(Box::new(self.parse_if_expression()?))
            } else {
                self.consume(&TokenKind::LeftBrace, "Expected '{' after else")?;
                let block = self.parse_block()?;
                Some(Box::new(Expr {
                    kind: ExprKind::Block(block),
                    span: Span::new(self.tokens[self.current - 1].span.start, self.previous_location()),
                }))
            }
        } else {
            None
        };
        
        let span = Span::new(start, self.previous_location());
        
        Ok(Expr {
            kind: ExprKind::If {
                condition,
                then_branch,
                else_branch,
            },
            span,
        })
    }

    fn parse_match_expression(&mut self) -> Result<Expr> {
        let start = self.tokens[self.current - 1].span.start; // 'match' was already consumed
        
        let expr = Box::new(self.parse_expression()?);
        self.consume(&TokenKind::LeftBrace, "Expected '{' after match expression")?;
        
        let mut arms = Vec::new();
        
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
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
        
        Ok(Expr {
            kind: ExprKind::Match { expr, arms },
            span,
        })
    }

    fn parse_pattern(&mut self) -> Result<Pattern> {
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
                    patterns.push(self.parse_pattern()?);
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
                    if let Some(token) = self.match_token_if(|t| matches!(t, TokenKind::Identifier(_))) {
                        if let TokenKind::Identifier(key) = token.kind {
                            let pattern = if self.match_token(&TokenKind::Colon) {
                                Some(self.parse_pattern()?)
                            } else {
                                None
                            };
                            
                            fields.push(ObjectPatternField { key, pattern });
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
        
        // Identifier pattern (variable binding)
        if let Some(token) = self.match_token_if(|t| matches!(t, TokenKind::Identifier(_))) {
            if let TokenKind::Identifier(name) = token.kind {
                let span = Span::new(start, self.previous_location());
                return Ok(Pattern {
                    kind: PatternKind::Identifier(name),
                    span,
                });
            }
        }
        
        Err(self.error("Expected pattern"))
    }

    fn parse_type_annotation(&mut self) -> Result<TypeAnn> {
        let start = self.current_location();
        
        // Array type
        if self.match_token(&TokenKind::LeftBracket) {
            let elem_type = self.parse_type_annotation()?;
            self.consume(&TokenKind::RightBracket, "Expected ']' after array element type")?;
            let span = Span::new(start, self.previous_location());
            return Ok(TypeAnn {
                kind: TypeKind::Array(Box::new(elem_type)),
                span,
            });
        }
        
        // Function type
        if self.match_token(&TokenKind::LeftParen) {
            let mut params = Vec::new();
            
            if !self.check(&TokenKind::RightParen) {
                loop {
                    params.push(self.parse_type_annotation()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
            }
            
            self.consume(&TokenKind::RightParen, "Expected ')' after function parameter types")?;
            self.consume(&TokenKind::Arrow, "Expected '->' in function type")?;
            let ret = Box::new(self.parse_type_annotation()?);
            
            let span = Span::new(start, self.previous_location());
            return Ok(TypeAnn {
                kind: TypeKind::Function { params, ret },
                span,
            });
        }
        
        // Named type
        let name = self.consume_identifier("Expected type name")?;
        let span = Span::new(start, self.previous_location());
        
        Ok(TypeAnn {
            kind: TypeKind::Named(name),
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
        
        Error::new(ErrorKind::ParseError, message)
            .with_location(location)
    }

    // Error recovery
    #[allow(dead_code)]
    fn synchronize(&mut self) {
        self.advance();
        
        while !self.is_at_end() {
            if matches!(self.previous().kind, TokenKind::Semicolon | TokenKind::Newline) {
                return;
            }
            
            match self.peek().kind {
                TokenKind::Fn | TokenKind::Let | TokenKind::For |
                TokenKind::If | TokenKind::While | TokenKind::Return => return,
                _ => {}
            }
            
            self.advance();
        }
    }
}