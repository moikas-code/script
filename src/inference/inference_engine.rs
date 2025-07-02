use super::{type_ann_to_type, Constraint, InferenceContext};
use crate::error::{Error, ErrorKind};
use crate::parser::{
    BinaryOp, Block, Expr, ExprKind, Literal, Pattern, PatternKind, Program, Stmt, StmtKind,
    UnaryOp,
};
use crate::source::Span;
use crate::types::Type;
use std::collections::HashMap;

/// Result of type inference for a program
#[derive(Debug)]
pub struct InferenceResult {
    /// Type of each expression in the program
    pub expr_types: HashMap<Span, Type>,
    /// Type of each statement in the program  
    pub stmt_types: HashMap<Span, Type>,
    /// Final substitution after solving all constraints
    pub substitution: super::Substitution,
}

/// The main type inference engine
pub struct InferenceEngine {
    context: InferenceContext,
    /// Store inferred types for expressions
    expr_types: HashMap<Span, Type>,
    /// Store inferred types for statements
    stmt_types: HashMap<Span, Type>,
}

impl InferenceEngine {
    /// Create a new inference engine
    pub fn new() -> Self {
        InferenceEngine {
            context: InferenceContext::new(),
            expr_types: HashMap::new(),
            stmt_types: HashMap::new(),
        }
    }

    /// Infer types for a program
    pub fn infer_program(&mut self, program: &Program) -> Result<InferenceResult, Error> {
        // Initialize built-in functions
        self.initialize_builtins();

        // Infer types for all statements
        for stmt in &program.statements {
            self.infer_stmt(stmt)?;
        }

        // Solve all collected constraints
        self.context.solve_constraints()?;

        // Apply final substitution to all inferred types
        self.apply_final_substitution();

        Ok(InferenceResult {
            expr_types: self.expr_types.clone(),
            stmt_types: self.stmt_types.clone(),
            substitution: self.context.substitution.clone(),
        })
    }

    /// Initialize built-in functions and types
    fn initialize_builtins(&mut self) {
        // Add print function: (unknown) -> ()
        let print_type = Type::Function {
            params: vec![Type::Unknown],
            ret: Box::new(Type::Unknown), // Unit type would be better, but we don't have it yet
        };
        self.context
            .type_env_mut()
            .define("print".to_string(), print_type);
    }

    /// Infer type for a statement
    fn infer_stmt(&mut self, stmt: &Stmt) -> Result<Type, Error> {
        let ty = match &stmt.kind {
            StmtKind::Let {
                name,
                type_ann,
                init,
            } => {
                let var_type = if let Some(ann) = type_ann {
                    // Use explicit type annotation
                    type_ann_to_type(ann)
                } else {
                    // Generate fresh type variable
                    self.context.fresh_type_var()
                };

                // If there's an initializer, infer its type and constrain
                if let Some(init_expr) = init {
                    let init_type = self.infer_expr(init_expr)?;
                    self.context.add_constraint(Constraint::equality(
                        var_type.clone(),
                        init_type,
                        stmt.span,
                    ));
                }

                // Add variable to environment
                self.context
                    .type_env_mut()
                    .define(name.clone(), var_type.clone());

                var_type
            }

            StmtKind::Function {
                name,
                generic_params: _, // TODO: Handle generic parameters
                params,
                ret_type,
                body,
                is_async,
            } => {
                // Enter new scope for function body
                self.context.push_scope();

                // Process parameters
                let mut param_types = Vec::new();
                for param in params {
                    let param_type = type_ann_to_type(&param.type_ann);
                    param_types.push(param_type.clone());
                    self.context
                        .type_env_mut()
                        .define(param.name.clone(), param_type);
                }

                // Infer body type
                let body_type = self.infer_block(body)?;

                // Determine return type
                let base_return_type = if let Some(ret_ann) = ret_type {
                    let declared_ret = type_ann_to_type(ret_ann);
                    // Constrain body type to match declared return type
                    self.context.add_constraint(Constraint::equality(
                        declared_ret.clone(),
                        body_type,
                        stmt.span,
                    ));
                    declared_ret
                } else {
                    body_type
                };

                // Wrap return type in Future<T> if async
                let return_type = if *is_async {
                    Type::Future(Box::new(base_return_type))
                } else {
                    base_return_type
                };

                // Exit function scope
                self.context.pop_scope();

                // Create function type
                let fn_type = Type::Function {
                    params: param_types,
                    ret: Box::new(return_type),
                };

                // Add function to environment
                self.context
                    .type_env_mut()
                    .define(name.clone(), fn_type.clone());

                fn_type
            }

            StmtKind::Return(expr_opt) => {
                if let Some(expr) = expr_opt {
                    self.infer_expr(expr)?
                } else {
                    Type::Unknown // Unit type would be better
                }
            }

            StmtKind::Expression(expr) => self.infer_expr(expr)?,

            StmtKind::While { condition, body } => {
                // Condition must be boolean
                let cond_type = self.infer_expr(condition)?;
                self.context.add_constraint(Constraint::equality(
                    Type::Bool,
                    cond_type,
                    condition.span,
                ));

                // Infer body type
                self.infer_block(body)?;

                Type::Unknown // Unit type would be better
            }

            StmtKind::For {
                variable,
                iterable,
                body,
            } => {
                // Infer iterable type
                let iter_type = self.infer_expr(iterable)?;

                // For now, assume iterables are arrays
                let elem_type = self.context.fresh_type_var();
                self.context.add_constraint(Constraint::equality(
                    Type::Array(Box::new(elem_type.clone())),
                    iter_type,
                    iterable.span,
                ));

                // Enter new scope for loop body
                self.context.push_scope();

                // Add loop variable with element type
                self.context
                    .type_env_mut()
                    .define(variable.clone(), elem_type);

                // Infer body type
                self.infer_block(body)?;

                // Exit loop scope
                self.context.pop_scope();

                Type::Unknown // Unit type would be better
            }

            StmtKind::Import { .. } => {
                // Import statements don't produce values
                Type::Unknown
            }

            StmtKind::Export { .. } => {
                // Export statements don't produce values
                Type::Unknown
            }
        };

        self.stmt_types.insert(stmt.span, ty.clone());
        Ok(ty)
    }

    /// Infer type for an expression
    fn infer_expr(&mut self, expr: &Expr) -> Result<Type, Error> {
        let ty = match &expr.kind {
            ExprKind::Literal(lit) => match lit {
                Literal::Number(_) => {
                    // Use a fresh type variable for numeric literals
                    // This allows them to unify with either i32 or f32
                    let ty = self.context.fresh_type_var();
                    // TODO: Add constraint that it must be numeric
                    ty
                }
                Literal::String(_) => Type::String,
                Literal::Boolean(_) => Type::Bool,
                Literal::Null => Type::Option(Box::new(Type::Unknown)),
            },

            ExprKind::Identifier(name) => {
                if let Some(ty) = self.context.type_env().lookup(name) {
                    ty.clone()
                } else {
                    return Err(Error::new(
                        ErrorKind::TypeError,
                        format!("Undefined variable: {}", name),
                    )
                    .with_location(expr.span.start));
                }
            }

            ExprKind::Binary { left, op, right } => {
                let left_type = self.infer_expr(left)?;
                let right_type = self.infer_expr(right)?;

                match op {
                    // Arithmetic operators
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Mod => {
                        // Both operands must be numeric and same type
                        self.context.add_constraint(Constraint::equality(
                            left_type.clone(),
                            right_type,
                            expr.span,
                        ));

                        // Result is same as operands
                        // TODO: Check that it's actually numeric
                        left_type
                    }

                    // Comparison operators
                    BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::Greater
                    | BinaryOp::LessEqual
                    | BinaryOp::GreaterEqual => {
                        // Operands must be same type
                        self.context
                            .add_constraint(Constraint::equality(left_type, right_type, expr.span));

                        // Result is boolean
                        Type::Bool
                    }

                    // Logical operators
                    BinaryOp::And | BinaryOp::Or => {
                        // Both operands must be boolean
                        self.context.add_constraint(Constraint::equality(
                            Type::Bool,
                            left_type,
                            left.span,
                        ));
                        self.context.add_constraint(Constraint::equality(
                            Type::Bool,
                            right_type,
                            right.span,
                        ));

                        Type::Bool
                    }
                }
            }

            ExprKind::Unary { op, expr: operand } => {
                let operand_type = self.infer_expr(operand)?;

                match op {
                    UnaryOp::Not => {
                        // Operand must be boolean
                        self.context.add_constraint(Constraint::equality(
                            Type::Bool,
                            operand_type,
                            operand.span,
                        ));
                        Type::Bool
                    }
                    UnaryOp::Minus => {
                        // Operand must be numeric
                        // For now, just return the same type
                        // TODO: Verify it's numeric
                        operand_type
                    }
                }
            }

            ExprKind::Call { callee, args } => {
                let callee_type = self.infer_expr(callee)?;

                // Callee must be a function type
                let param_types: Vec<Type> =
                    args.iter().map(|_| self.context.fresh_type_var()).collect();
                let ret_type = self.context.fresh_type_var();

                let expected_fn_type = Type::Function {
                    params: param_types.clone(),
                    ret: Box::new(ret_type.clone()),
                };

                self.context.add_constraint(Constraint::equality(
                    expected_fn_type,
                    callee_type,
                    callee.span,
                ));

                // Infer argument types and constrain
                for (arg, param_type) in args.iter().zip(param_types) {
                    let arg_type = self.infer_expr(arg)?;
                    self.context
                        .add_constraint(Constraint::equality(param_type, arg_type, arg.span));
                }

                ret_type
            }

            ExprKind::Index { object, index } => {
                let object_type = self.infer_expr(object)?;
                let index_type = self.infer_expr(index)?;

                // Index must be i32
                self.context.add_constraint(Constraint::equality(
                    Type::I32,
                    index_type,
                    index.span,
                ));

                // Object must be an array
                let elem_type = self.context.fresh_type_var();
                self.context.add_constraint(Constraint::equality(
                    Type::Array(Box::new(elem_type.clone())),
                    object_type,
                    object.span,
                ));

                elem_type
            }

            ExprKind::Member {
                object,
                property: _,
            } => {
                // For now, just return a fresh type variable
                // TODO: Implement proper struct/object typing
                let _ = self.infer_expr(object)?;
                self.context.fresh_type_var()
            }

            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                // Condition must be boolean
                let cond_type = self.infer_expr(condition)?;
                self.context.add_constraint(Constraint::equality(
                    Type::Bool,
                    cond_type,
                    condition.span,
                ));

                // Infer branch types
                let then_type = self.infer_expr(then_branch)?;

                if let Some(else_expr) = else_branch {
                    let else_type = self.infer_expr(else_expr)?;
                    // Both branches must have same type
                    self.context.add_constraint(Constraint::equality(
                        then_type.clone(),
                        else_type,
                        expr.span,
                    ));
                }

                then_type
            }

            ExprKind::Block(block) => self.infer_block(block)?,

            ExprKind::Array(elements) => {
                if elements.is_empty() {
                    // Empty array with fresh element type
                    Type::Array(Box::new(self.context.fresh_type_var()))
                } else {
                    // All elements must have same type
                    let elem_type = self.infer_expr(&elements[0])?;

                    for element in &elements[1..] {
                        let element_type = self.infer_expr(element)?;
                        self.context.add_constraint(Constraint::equality(
                            elem_type.clone(),
                            element_type,
                            element.span,
                        ));
                    }

                    Type::Array(Box::new(elem_type))
                }
            }

            ExprKind::Assign { target, value } => {
                let target_type = self.infer_expr(target)?;
                let value_type = self.infer_expr(value)?;

                // Value must match target type
                self.context.add_constraint(Constraint::equality(
                    target_type.clone(),
                    value_type,
                    expr.span,
                ));

                target_type
            }

            ExprKind::Match { expr, arms } => {
                // Infer the type of the expression being matched
                let expr_type = self.infer_expr(expr)?;

                if arms.is_empty() {
                    return Err(Error::new(
                        ErrorKind::TypeError,
                        "Match expression cannot have zero arms".to_string(),
                    )
                    .with_location(expr.span.start));
                }

                // Infer the type of the first arm's body
                let first_arm = &arms[0];
                if let Some(guard) = &first_arm.guard {
                    let guard_type = self.infer_expr(guard)?;
                    self.context.add_constraint(Constraint::equality(
                        Type::Bool,
                        guard_type,
                        guard.span,
                    ));
                }
                let first_body_type = self.infer_expr(&first_arm.body)?;

                // Check that all patterns are compatible with the expression type
                // and all arm bodies have the same type
                for arm in arms {
                    // Check pattern compatibility with expression type
                    self.check_pattern_compatibility(&arm.pattern, &expr_type)?;

                    // Check guard type if present
                    if let Some(guard) = &arm.guard {
                        let guard_type = self.infer_expr(guard)?;
                        self.context.add_constraint(Constraint::equality(
                            Type::Bool,
                            guard_type,
                            guard.span,
                        ));
                    }

                    // Check body type consistency
                    let body_type = self.infer_expr(&arm.body)?;
                    self.context.add_constraint(Constraint::equality(
                        first_body_type.clone(),
                        body_type,
                        arm.body.span,
                    ));
                }

                first_body_type
            }

            ExprKind::Await { expr: awaited_expr } => {
                let expr_type = self.infer_expr(awaited_expr)?;

                // Awaited expression must be a Future<T>
                let inner_type = self.context.fresh_type_var();
                self.context.add_constraint(Constraint::equality(
                    Type::Future(Box::new(inner_type.clone())),
                    expr_type,
                    awaited_expr.span,
                ));

                // Result is T
                inner_type
            }

            ExprKind::ListComprehension { .. } => {
                // List comprehensions not yet implemented
                // TODO: Implement proper list comprehension type inference
                Type::Array(Box::new(Type::Unknown))
            }
            ExprKind::GenericConstructor { name, type_args } => {
                // For now, treat generic constructors as named types
                // TODO: Implement proper generic type conversion from AST to inference types
                let _ = type_args; // suppress warning
                Type::Named(name.clone())
            }
        };

        self.expr_types.insert(expr.span, ty.clone());
        Ok(ty)
    }

    /// Infer type for a block
    fn infer_block(&mut self, block: &Block) -> Result<Type, Error> {
        // Enter new scope
        self.context.push_scope();

        // Infer types for all statements
        for stmt in &block.statements {
            self.infer_stmt(stmt)?;
        }

        // Block type is the type of the final expression (if any)
        let block_type = if let Some(final_expr) = &block.final_expr {
            self.infer_expr(final_expr)?
        } else {
            Type::Unknown // Unit type would be better
        };

        // Exit scope
        self.context.pop_scope();

        Ok(block_type)
    }

    /// Check if a pattern is compatible with a given type
    fn check_pattern_compatibility(
        &mut self,
        pattern: &Pattern,
        expected_type: &Type,
    ) -> Result<(), Error> {
        match &pattern.kind {
            PatternKind::Wildcard => {
                // Wildcard matches anything
                Ok(())
            }
            PatternKind::Literal(literal) => {
                let literal_type = match literal {
                    Literal::Number(_) => Type::F32, // TODO: Could be i32 too
                    Literal::String(_) => Type::String,
                    Literal::Boolean(_) => Type::Bool,
                    Literal::Null => Type::Option(Box::new(Type::Unknown)),
                };

                self.context.add_constraint(Constraint::equality(
                    expected_type.clone(),
                    literal_type,
                    pattern.span,
                ));
                Ok(())
            }
            PatternKind::Identifier(name) => {
                // Variable binding pattern - add to environment
                self.context
                    .type_env_mut()
                    .define(name.clone(), expected_type.clone());
                Ok(())
            }
            PatternKind::Array(patterns) => {
                // Expected type should be an array
                let element_type = match expected_type {
                    Type::Array(elem_type) => elem_type.as_ref().clone(),
                    _ => {
                        // Create a fresh type variable for the element type
                        let elem_type = self.context.fresh_type_var();
                        self.context.add_constraint(Constraint::equality(
                            expected_type.clone(),
                            Type::Array(Box::new(elem_type.clone())),
                            pattern.span,
                        ));
                        elem_type
                    }
                };

                // Check each pattern element
                for sub_pattern in patterns {
                    self.check_pattern_compatibility(sub_pattern, &element_type)?;
                }
                Ok(())
            }
            PatternKind::Object(_fields) => {
                // Object destructuring - for now, just check it's an object-like type
                // TODO: Implement proper object pattern matching
                Ok(())
            }
            PatternKind::Or(patterns) => {
                // All patterns in an or-pattern must be compatible with the same type
                for sub_pattern in patterns {
                    self.check_pattern_compatibility(sub_pattern, expected_type)?;
                }
                Ok(())
            }
        }
    }

    /// Apply final substitution to all inferred types
    fn apply_final_substitution(&mut self) {
        // Apply substitution to expression types
        for ty in self.expr_types.values_mut() {
            *ty = self.context.apply_substitution(ty);
        }

        // Apply substitution to statement types
        for ty in self.stmt_types.values_mut() {
            *ty = self.context.apply_substitution(ty);
        }
    }
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn infer_program_str(input: &str) -> Result<InferenceResult, Error> {
        let lexer = Lexer::new(input);
        let (tokens, _errors) = lexer.scan_tokens();
        if !_errors.is_empty() {
            return Err(_errors[0].clone());
        }
        let mut parser = Parser::new(tokens);
        let program = parser.parse()?;

        let mut engine = InferenceEngine::new();
        engine.infer_program(&program)
    }

    #[test]
    fn test_infer_literals() {
        let result = infer_program_str("42; true; \"hello\";").unwrap();

        // Check that literals have correct types
        let expr_types: Vec<_> = result.expr_types.values().collect();
        // Numbers get type variables now
        assert!(expr_types.iter().any(|t| matches!(t, Type::TypeVar(_))));
        assert!(expr_types.contains(&&Type::Bool)); // Boolean
        assert!(expr_types.contains(&&Type::String)); // String
    }

    #[test]
    fn test_infer_variable() {
        let result = infer_program_str("let x = 42; x;").unwrap();

        // Both the literal and variable reference should have type variables
        let types: Vec<_> = result.expr_types.values().collect();
        assert!(
            types
                .iter()
                .filter(|t| matches!(t, Type::TypeVar(_)))
                .count()
                >= 2
        );
    }

    #[test]
    fn test_infer_function() {
        let code = r#"
            fn add(a: i32, b: i32) -> i32 {
                a + b
            }
            add(1, 2);
        "#;

        let result = infer_program_str(code).unwrap();

        // Check function type
        let fn_type = Type::Function {
            params: vec![Type::I32, Type::I32],
            ret: Box::new(Type::I32),
        };
        assert!(result.stmt_types.values().any(|t| t == &fn_type));
    }

    #[test]
    fn test_infer_array() {
        let result = infer_program_str("[1, 2, 3];").unwrap();

        // Check array type - elements are type variables
        assert!(result
            .expr_types
            .values()
            .any(|t| matches!(t, Type::Array(_))));
    }

    #[test]
    fn test_type_mismatch_error() {
        // Boolean in arithmetic
        // TODO: Add numeric constraints for arithmetic operators
        // let result = infer_program_str("true + 1;");
        // assert!(result.is_err());

        // Wrong number of function arguments
        let result = infer_program_str("fn f(x: i32) { x } f(1, 2);");
        assert!(result.is_err());
    }
}
