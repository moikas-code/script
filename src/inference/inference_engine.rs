use super::{type_ann_to_type, Constraint, InferenceContext};
use crate::compilation::resource_limits::{ResourceLimits, ResourceMonitor};
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
    pub substitution: super::OptimizedSubstitution,
}

/// The main type inference engine
pub struct InferenceEngine {
    context: InferenceContext,
    /// Store inferred types for expressions
    expr_types: HashMap<Span, Type>,
    /// Store inferred types for statements
    stmt_types: HashMap<Span, Type>,
    /// Resource monitor for DoS protection
    resource_monitor: ResourceMonitor,
    /// Recursion depth tracking for stack overflow protection
    recursion_depth: usize,
}

impl InferenceEngine {
    /// Create a new inference engine with default resource limits
    pub fn new() -> Self {
        Self::with_resource_limits(ResourceLimits::production())
    }

    /// Create a new inference engine with custom resource limits
    pub fn with_resource_limits(limits: ResourceLimits) -> Self {
        InferenceEngine {
            context: InferenceContext::new(),
            expr_types: HashMap::new(),
            stmt_types: HashMap::new(),
            resource_monitor: ResourceMonitor::new(limits),
            recursion_depth: 0,
        }
    }

    /// Create a new inference engine for development (more permissive limits)
    pub fn for_development() -> Self {
        Self::with_resource_limits(ResourceLimits::development())
    }

    /// Create a new inference engine for testing (very permissive limits)
    pub fn for_testing() -> Self {
        Self::with_resource_limits(ResourceLimits::testing())
    }

    /// Infer types for a program
    pub fn infer_program(&mut self, program: &Program) -> Result<InferenceResult, Error> {
        // Start resource monitoring for type inference phase
        self.resource_monitor.start_phase("type_inference");

        // Initialize built-in functions
        self.initialize_builtins();

        // Infer types for all statements with resource monitoring
        for (i, stmt) in program.statements.iter().enumerate() {
            // Check resource limits every 100 statements
            if i % 100 == 0 {
                self.resource_monitor
                    .check_phase_timeout("type_inference")?;
                self.resource_monitor.check_total_timeout()?;
                self.resource_monitor
                    .check_iteration_limit("statement_inference", 100)?;

                // Check memory usage every 100 statements
                self.resource_monitor.check_system_memory()?;
            }

            self.infer_stmt(stmt)?;
        }

        // Solve all collected constraints with monitoring
        self.resource_monitor.start_phase("constraint_solving");
        self.context.solve_constraints()?;
        self.resource_monitor.end_phase("constraint_solving");

        // Apply final substitution to all inferred types
        self.apply_final_substitution();

        self.resource_monitor.end_phase("type_inference");

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
        // Track recursion depth for stack overflow protection
        self.recursion_depth += 1;
        self.resource_monitor
            .check_recursion_depth("stmt_inference", self.recursion_depth)?;

        let result = self.infer_stmt_impl(stmt);

        self.recursion_depth -= 1;
        result
    }

    /// Internal implementation of statement type inference
    fn infer_stmt_impl(&mut self, stmt: &Stmt) -> Result<Type, Error> {
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
                    // Generate fresh type variable with resource tracking
                    self.resource_monitor.add_type_variable()?;
                    self.context.fresh_type_var()
                };

                // If there's an initializer, infer its type and constrain
                if let Some(init_expr) = init {
                    let init_type = self.infer_expr(init_expr)?;
                    self.resource_monitor.add_constraint()?;
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
                generic_params: _generic_params, // TODO: Handle generic parameters
                params,
                ret_type,
                body,
                is_async,
                where_clause: _where_clause, // TODO: Handle where clause
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

            StmtKind::Struct { name, .. } => {
                // TODO: Implement struct type inference
                // For now, register as a named type
                Type::Named(name.clone())
            }

            StmtKind::Enum { name, .. } => {
                // TODO: Implement enum type inference
                // For now, register as a named type
                Type::Named(name.clone())
            }

            StmtKind::Impl(_) => {
                // TODO: Implement impl block type inference
                Type::Unknown
            }
        };

        self.stmt_types.insert(stmt.span, ty.clone());
        Ok(ty)
    }

    /// Infer type for an expression
    fn infer_expr(&mut self, expr: &Expr) -> Result<Type, Error> {
        // Track recursion depth for stack overflow protection
        self.recursion_depth += 1;
        self.resource_monitor
            .check_recursion_depth("expr_inference", self.recursion_depth)?;

        // Periodically check resource limits during deep recursion
        if self.recursion_depth % 50 == 0 {
            self.resource_monitor
                .check_phase_timeout("type_inference")?;
            self.resource_monitor.check_total_timeout()?;
        }

        let result = self.infer_expr_impl(expr);

        self.recursion_depth -= 1;
        result
    }

    /// Internal implementation of expression type inference
    fn infer_expr_impl(&mut self, expr: &Expr) -> Result<Type, Error> {
        let ty = match &expr.kind {
            ExprKind::Literal(lit) => match lit {
                Literal::Number(_) => {
                    // Use a fresh type variable for numeric literals
                    // This allows them to unify with either i32 or f32
                    self.resource_monitor.add_type_variable()?;
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
                        self.resource_monitor.add_constraint()?;
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
                property: _property,
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
                // Generic constructors are treated as named types
                // NOTE: With monomorphization complete, this simplified approach may be sufficient
                let _type_args = type_args; // suppress warning
                Type::Named(name.clone())
            }

            ExprKind::StructConstructor { name, .. } => {
                // TODO: Implement struct constructor type inference
                Type::Named(name.clone())
            }

            ExprKind::EnumConstructor { variant, .. } => {
                // TODO: Implement enum constructor type inference
                Type::Named(variant.clone())
            }

            ExprKind::ErrorPropagation { expr } => {
                // Infer type for the inner expression
                let inner_ty = self.infer_expr(expr)?;

                // Verify that the inner expression is a Result<T, E> or Option<T>
                match &inner_ty {
                    Type::Result { ok, .. } => ok.as_ref().clone(),
                    Type::Option(inner) => inner.as_ref().clone(),
                    _ => {
                        return Err(Error::new(
                            ErrorKind::TypeError,
                            format!("Error propagation (?) can only be used with Result or Option types, found: {:?}", inner_ty),
                        ));
                    }
                }
            }

            ExprKind::TryCatch { try_expr, .. } => {
                // The type of a try-catch expression is the type of the try expression
                self.infer_expr(try_expr)?
            }

            ExprKind::Closure { parameters, body } => {
                // Create function type for closure with resource monitoring
                let param_types: Vec<Type> = parameters
                    .iter()
                    .map(|param| {
                        if let Some(ref type_ann) = param.type_ann {
                            crate::types::conversion::type_from_ast(type_ann)
                        } else {
                            // Use fresh type variable for untyped parameters
                            self.resource_monitor
                                .add_type_variable()
                                .unwrap_or_else(|_| {
                                    // If we hit resource limits, fall back to Unknown type
                                    ()
                                });
                            self.context.fresh_type_var()
                        }
                    })
                    .collect();

                let return_type = self.infer_expr(body)?;

                Type::Function {
                    params: param_types,
                    ret: Box::new(return_type),
                }
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
            PatternKind::EnumConstructor {
                enum_name,
                variant: _variant,
                args,
            } => {
                // For enum patterns, we need to check that the enum variant is compatible
                // with the expected type and that the arguments match the variant's fields

                // This is a placeholder implementation - in a full implementation,
                // we would look up the enum definition and check variant compatibility
                match expected_type {
                    Type::Named(name) => {
                        // Check if the pattern matches the expected enum type
                        if let Some(enum_name) = enum_name {
                            if enum_name != name {
                                return Err(Error::new(
                                    ErrorKind::TypeError,
                                    format!("Pattern expects enum {}, but got {}", enum_name, name),
                                ));
                            }
                        }
                        // Check arguments if present
                        if let Some(pattern_args) = args {
                            for arg_pattern in pattern_args {
                                self.check_pattern_compatibility(arg_pattern, &Type::Unknown)?;
                            }
                        }
                        Ok(())
                    }
                    _ => {
                        // For non-enum types, this pattern is not compatible
                        Err(Error::new(
                            ErrorKind::TypeError,
                            format!("Enum pattern cannot match non-enum type {}", expected_type),
                        ))
                    }
                }
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
        let lexer = Lexer::new(input).unwrap();
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
