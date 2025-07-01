use crate::parser::{Program, Stmt, StmtKind, Expr, ExprKind, Block, Param, TypeAnn, Literal, BinaryOp, UnaryOp};
use crate::types::Type;
use crate::inference::{InferenceContext, type_ann_to_type};
use crate::Result;

use super::symbol_table::SymbolTable;
use super::symbol::FunctionSignature;
use super::error::{SemanticError, SemanticErrorKind};

/// Context for the current analysis
#[derive(Debug)]
struct AnalysisContext {
    /// Current function return type (None if not in a function)
    current_function_return: Option<Type>,
    /// Whether we're currently in a loop
    in_loop: bool,
    /// Whether the current function is marked as @const
    _in_const_function: bool,
}

impl AnalysisContext {
    fn new() -> Self {
        AnalysisContext {
            current_function_return: None,
            in_loop: false,
            _in_const_function: false,
        }
    }
}

/// Semantic analyzer that performs name resolution and type checking
#[derive(Debug)]
pub struct SemanticAnalyzer {
    /// Symbol table for tracking definitions
    symbol_table: SymbolTable,
    /// Type inference context
    inference_ctx: InferenceContext,
    /// Stack of analysis contexts
    context_stack: Vec<AnalysisContext>,
    /// Collected errors
    errors: Vec<SemanticError>,
}

impl SemanticAnalyzer {
    /// Create a new semantic analyzer
    pub fn new() -> Self {
        SemanticAnalyzer {
            symbol_table: SymbolTable::new(),
            inference_ctx: InferenceContext::new(),
            context_stack: vec![AnalysisContext::new()],
            errors: Vec::new(),
        }
    }

    /// Get the current analysis context
    fn current_context(&self) -> &AnalysisContext {
        self.context_stack.last().expect("context stack should never be empty")
    }

    /// Get the current analysis context mutably
    fn current_context_mut(&mut self) -> &mut AnalysisContext {
        self.context_stack.last_mut().expect("context stack should never be empty")
    }

    /// Push a new analysis context
    fn push_context(&mut self, ctx: AnalysisContext) {
        self.context_stack.push(ctx);
    }

    /// Pop an analysis context
    fn pop_context(&mut self) {
        if self.context_stack.len() > 1 {
            self.context_stack.pop();
        }
    }

    /// Add an error
    fn add_error(&mut self, error: SemanticError) {
        self.errors.push(error);
    }

    /// Analyze a program
    pub fn analyze_program(&mut self, program: &Program) -> Result<()> {
        // Add built-in functions to the global scope
        self.add_builtins()?;

        // Analyze all statements
        for stmt in &program.statements {
            self.analyze_stmt(stmt)?;
        }

        // Check for unused symbols
        let unused_symbols: Vec<_> = self.symbol_table.get_unused_symbols()
            .into_iter()
            .map(|s| (s.name.clone(), s.def_span))
            .collect();
        
        for (_name, _span) in unused_symbols {
            // For now, we'll skip reporting unused variables as errors
            // This would normally be a warning, not an error
            continue;
        }

        // Return errors if any
        if !self.errors.is_empty() {
            // For now, return the first error
            // In a real implementation, we might want to return all errors
            return Err(self.errors[0].clone().into_error());
        }

        Ok(())
    }

    /// Add built-in functions
    fn add_builtins(&mut self) -> Result<()> {
        // print function: (unknown) -> void
        let print_sig = FunctionSignature {
            params: vec![("value".to_string(), Type::Unknown)],
            return_type: Type::Unknown, // void
            is_const: false,
            is_async: false,
        };
        self.symbol_table.define_function(
            "print".to_string(),
            print_sig,
            crate::source::Span::single(crate::source::SourceLocation::initial()),
        ).map_err(|e| SemanticError::new(
            SemanticErrorKind::DuplicateFunction(e),
            crate::source::Span::single(crate::source::SourceLocation::initial()),
        ).into_error())?;

        // len function: ([T]) -> i32
        let len_sig = FunctionSignature {
            params: vec![("array".to_string(), Type::Array(Box::new(Type::Unknown)))],
            return_type: Type::I32,
            is_const: true,
            is_async: false,
        };
        self.symbol_table.define_function(
            "len".to_string(),
            len_sig,
            crate::source::Span::single(crate::source::SourceLocation::initial()),
        ).map_err(|e| SemanticError::new(
            SemanticErrorKind::DuplicateFunction(e),
            crate::source::Span::single(crate::source::SourceLocation::initial()),
        ).into_error())?;

        Ok(())
    }

    /// Analyze a statement
    fn analyze_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match &stmt.kind {
            StmtKind::Let { name, type_ann, init } => {
                self.analyze_let(name, type_ann.as_ref(), init.as_ref(), stmt.span)?;
            }
            StmtKind::Function { name, params, ret_type, body } => {
                self.analyze_function(name, params, ret_type.as_ref(), body, stmt.span)?;
            }
            StmtKind::Return(expr) => {
                self.analyze_return(expr.as_ref(), stmt.span)?;
            }
            StmtKind::Expression(expr) => {
                self.analyze_expr(expr)?;
            }
            StmtKind::While { condition, body } => {
                self.analyze_while(condition, body)?;
            }
            StmtKind::For { variable, iterable, body } => {
                self.analyze_for(variable, iterable, body)?;
            }
        }
        Ok(())
    }

    /// Analyze a let statement
    fn analyze_let(
        &mut self,
        name: &str,
        type_ann: Option<&TypeAnn>,
        init: Option<&Expr>,
        span: crate::source::Span,
    ) -> Result<()> {
        // Determine the type
        let ty = if let Some(type_ann) = type_ann {
            type_ann_to_type(type_ann)
        } else if let Some(init_expr) = init {
            // Infer type from initializer
            self.analyze_expr(init_expr)?;
            // For now, use Unknown type
            // In a complete implementation, we'd get the type from type inference
            Type::Unknown
        } else {
            // No type annotation and no initializer
            self.add_error(SemanticError::new(
                SemanticErrorKind::TypeMismatch {
                    expected: Type::Unknown,
                    found: Type::Unknown,
                },
                span,
            ).with_note("variables must have either a type annotation or an initializer".to_string()));
            Type::Unknown
        };

        // Define the variable
        match self.symbol_table.define_variable(name.to_string(), ty, span, true) {
            Ok(symbol_id) => {
                // If there's an initializer, mark the variable as used
                if init.is_some() {
                    self.symbol_table.mark_used(symbol_id);
                }
            }
            Err(err) => {
                self.add_error(SemanticError::duplicate_variable(name, span)
                    .with_note(err));
            }
        }

        Ok(())
    }

    /// Analyze a function definition
    fn analyze_function(
        &mut self,
        name: &str,
        params: &[Param],
        ret_type: Option<&TypeAnn>,
        body: &Block,
        span: crate::source::Span,
    ) -> Result<()> {
        // Convert parameter types
        let param_types: Vec<(String, Type)> = params
            .iter()
            .map(|p| (p.name.clone(), type_ann_to_type(&p.type_ann)))
            .collect();

        // Convert return type
        let return_type = ret_type
            .map(type_ann_to_type)
            .unwrap_or(Type::Unknown);

        // Create function signature
        let signature = FunctionSignature {
            params: param_types.clone(),
            return_type: return_type.clone(),
            is_const: false, // TODO: Support @const functions
            is_async: false,
        };

        // Define the function
        match self.symbol_table.define_function(name.to_string(), signature, span) {
            Ok(_) => {}
            Err(err) => {
                self.add_error(SemanticError::new(
                    SemanticErrorKind::DuplicateFunction(name.to_string()),
                    span,
                ).with_note(err));
            }
        }

        // Enter function scope
        self.symbol_table.enter_scope();
        self.inference_ctx.push_scope();

        // Push function context
        let func_context = AnalysisContext {
            current_function_return: Some(return_type),
            in_loop: false,
            _in_const_function: false, // TODO: Support @const
        };
        self.push_context(func_context);

        // Define parameters
        for param in params {
            let param_type = type_ann_to_type(&param.type_ann);
            match self.symbol_table.define_parameter(
                param.name.clone(),
                param_type,
                param.type_ann.span,
            ) {
                Ok(symbol_id) => {
                    // Mark parameters as used
                    self.symbol_table.mark_used(symbol_id);
                }
                Err(err) => {
                    self.add_error(SemanticError::duplicate_variable(&param.name, param.type_ann.span)
                        .with_note(err));
                }
            }
        }

        // Analyze function body
        self.analyze_block(body)?;

        // TODO: Check for missing return if function has non-void return type

        // Exit function context
        self.pop_context();
        self.symbol_table.exit_scope();
        self.inference_ctx.pop_scope();

        Ok(())
    }

    /// Analyze a return statement
    fn analyze_return(&mut self, expr: Option<&Expr>, span: crate::source::Span) -> Result<()> {
        let ctx = self.current_context();
        
        if ctx.current_function_return.is_none() {
            self.add_error(SemanticError::new(
                SemanticErrorKind::ReturnOutsideFunction,
                span,
            ));
            return Ok(());
        }

        if let Some(expr) = expr {
            self.analyze_expr(expr)?;
            // TODO: Check return type matches
        }

        Ok(())
    }

    /// Analyze a while loop
    fn analyze_while(&mut self, condition: &Expr, body: &Block) -> Result<()> {
        // Analyze condition
        self.analyze_expr(condition)?;

        // Enter loop context
        self.current_context_mut().in_loop = true;

        // Analyze body in new scope
        self.symbol_table.enter_scope();
        self.inference_ctx.push_scope();
        self.analyze_block(body)?;
        self.symbol_table.exit_scope();
        self.inference_ctx.pop_scope();

        // Exit loop context
        self.current_context_mut().in_loop = false;

        Ok(())
    }

    /// Analyze a for loop
    fn analyze_for(&mut self, variable: &str, iterable: &Expr, body: &Block) -> Result<()> {
        // Analyze iterable
        self.analyze_expr(iterable)?;

        // Enter loop scope
        self.symbol_table.enter_scope();
        self.inference_ctx.push_scope();

        // Define loop variable
        // TODO: Infer type from iterable
        let loop_var_type = Type::Unknown;
        match self.symbol_table.define_variable(
            variable.to_string(),
            loop_var_type,
            iterable.span,
            false, // Loop variables are immutable
        ) {
            Ok(symbol_id) => {
                self.symbol_table.mark_used(symbol_id);
            }
            Err(err) => {
                self.add_error(SemanticError::duplicate_variable(variable, iterable.span)
                    .with_note(err));
            }
        }

        // Enter loop context
        self.current_context_mut().in_loop = true;

        // Analyze body
        self.analyze_block(body)?;

        // Exit loop context
        self.current_context_mut().in_loop = false;

        // Exit loop scope
        self.symbol_table.exit_scope();
        self.inference_ctx.pop_scope();

        Ok(())
    }

    /// Analyze a block
    fn analyze_block(&mut self, block: &Block) -> Result<()> {
        // Analyze statements
        for stmt in &block.statements {
            self.analyze_stmt(stmt)?;
        }

        // Analyze final expression
        if let Some(expr) = &block.final_expr {
            self.analyze_expr(expr)?;
        }

        Ok(())
    }

    /// Analyze an expression
    fn analyze_expr(&mut self, expr: &Expr) -> Result<Type> {
        match &expr.kind {
            ExprKind::Literal(lit) => self.analyze_literal(lit),
            ExprKind::Identifier(name) => self.analyze_identifier(name, expr.span),
            ExprKind::Binary { left, op, right } => self.analyze_binary(left, op, right, expr.span),
            ExprKind::Unary { op, expr: inner } => self.analyze_unary(op, inner, expr.span),
            ExprKind::Call { callee, args } => self.analyze_call(callee, args, expr.span),
            ExprKind::Index { object, index } => self.analyze_index(object, index, expr.span),
            ExprKind::Member { object, property } => self.analyze_member(object, property, expr.span),
            ExprKind::If { condition, then_branch, else_branch } => {
                self.analyze_if(condition, then_branch, else_branch.as_deref(), expr.span)
            }
            ExprKind::Block(block) => self.analyze_block_expr(block),
            ExprKind::Array(elements) => self.analyze_array(elements, expr.span),
            ExprKind::Assign { target, value } => self.analyze_assign(target, value, expr.span),
            ExprKind::Match { expr: match_expr, arms } => {
                self.analyze_match(match_expr, arms, expr.span)
            }
        }
    }

    /// Analyze a literal
    fn analyze_literal(&mut self, lit: &Literal) -> Result<Type> {
        Ok(match lit {
            Literal::Number(_) => Type::Unknown, // TODO: Distinguish between i32 and f32
            Literal::String(_) => Type::String,
            Literal::Boolean(_) => Type::Bool,
        })
    }

    /// Analyze an identifier
    fn analyze_identifier(&mut self, name: &str, span: crate::source::Span) -> Result<Type> {
        if let Some(symbol) = self.symbol_table.lookup(name) {
            let symbol_id = symbol.id;
            let ty = symbol.ty.clone();
            
            // Mark as used
            self.symbol_table.mark_used(symbol_id);
            
            Ok(ty)
        } else {
            self.add_error(SemanticError::undefined_variable(name, span));
            Ok(Type::Unknown)
        }
    }

    /// Analyze a binary expression
    fn analyze_binary(
        &mut self,
        left: &Expr,
        op: &BinaryOp,
        right: &Expr,
        span: crate::source::Span,
    ) -> Result<Type> {
        let left_type = self.analyze_expr(left)?;
        let right_type = self.analyze_expr(right)?;

        // TODO: Implement proper type checking for binary operations
        // For now, return a reasonable type based on the operator
        Ok(match op {
            BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Modulo => {
                if (left_type.is_numeric() || left_type == Type::Unknown) && 
                   (right_type.is_numeric() || right_type == Type::Unknown) {
                    if left_type == Type::Unknown && right_type == Type::Unknown {
                        Type::Unknown
                    } else if left_type != Type::Unknown {
                        left_type
                    } else {
                        right_type
                    }
                } else {
                    self.add_error(SemanticError::invalid_binary_operation(
                        &op.to_string(),
                        left_type,
                        right_type,
                        span,
                    ));
                    Type::Unknown
                }
            }
            BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::Less | BinaryOp::Greater |
            BinaryOp::LessEqual | BinaryOp::GreaterEqual => {
                if (left_type.is_comparable() || left_type == Type::Unknown) && 
                   (right_type.is_comparable() || right_type == Type::Unknown) {
                    Type::Bool
                } else {
                    self.add_error(SemanticError::invalid_binary_operation(
                        &op.to_string(),
                        left_type,
                        right_type,
                        span,
                    ));
                    Type::Bool
                }
            }
            BinaryOp::And | BinaryOp::Or => Type::Bool,
        })
    }

    /// Analyze a unary expression
    fn analyze_unary(
        &mut self,
        op: &UnaryOp,
        expr: &Expr,
        span: crate::source::Span,
    ) -> Result<Type> {
        let expr_type = self.analyze_expr(expr)?;

        Ok(match op {
            UnaryOp::Not => {
                if expr_type == Type::Bool || expr_type == Type::Unknown {
                    Type::Bool
                } else {
                    self.add_error(SemanticError::invalid_operation("!", expr_type, span));
                    Type::Bool
                }
            }
            UnaryOp::Negate => {
                if expr_type.is_numeric() || expr_type == Type::Unknown {
                    expr_type
                } else {
                    self.add_error(SemanticError::invalid_operation("-", expr_type, span));
                    Type::Unknown
                }
            }
        })
    }

    /// Analyze a function call
    fn analyze_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        span: crate::source::Span,
    ) -> Result<Type> {
        // Special handling for direct function calls
        if let ExprKind::Identifier(name) = &callee.kind {
            // Analyze arguments
            let arg_types: Vec<Type> = args
                .iter()
                .map(|arg| self.analyze_expr(arg))
                .collect::<Result<Vec<_>>>()?;

            // Look up function with matching signature
            if let Some(func_symbol) = self.symbol_table.lookup_function(name, &arg_types) {
                let func_id = func_symbol.id;
                let return_type = func_symbol.function_signature()
                    .map(|sig| sig.return_type.clone())
                    .unwrap_or(Type::Unknown);
                
                self.symbol_table.mark_used(func_id);
                return Ok(return_type);
            } else {
                // Check if function exists at all
                let candidates = self.symbol_table.lookup_all(name);
                if candidates.is_empty() {
                    self.add_error(SemanticError::undefined_function(name, span));
                } else {
                    // Function exists but no matching overload
                    let func_candidates: Vec<_> = candidates
                        .into_iter()
                        .filter(|s| s.is_function())
                        .collect();
                    
                    if func_candidates.is_empty() {
                        self.add_error(SemanticError::new(
                            SemanticErrorKind::FunctionAsValue(name.to_string()),
                            span,
                        ));
                    } else {
                        // TODO: Better error message with available overloads
                        self.add_error(SemanticError::argument_count_mismatch(
                            func_candidates[0].function_signature().unwrap().params.len(),
                            args.len(),
                            span,
                        ));
                    }
                }
                return Ok(Type::Unknown);
            }
        }

        // General case: callee is an expression
        let callee_type = self.analyze_expr(callee)?;
        
        // Analyze arguments
        for arg in args {
            self.analyze_expr(arg)?;
        }

        // Check if callable
        match &callee_type {
            Type::Function { ret, .. } => Ok((**ret).clone()),
            Type::Unknown => Ok(Type::Unknown),
            _ => {
                self.add_error(SemanticError::not_callable(callee_type, span));
                Ok(Type::Unknown)
            }
        }
    }

    /// Analyze an index expression
    fn analyze_index(
        &mut self,
        object: &Expr,
        index: &Expr,
        span: crate::source::Span,
    ) -> Result<Type> {
        let object_type = self.analyze_expr(object)?;
        let index_type = self.analyze_expr(index)?;

        // Check index type
        if !index_type.is_numeric() && index_type != Type::Unknown {
            self.add_error(SemanticError::invalid_index_type(index_type, span));
        }

        // Check if object is indexable
        match &object_type {
            Type::Array(elem_type) => Ok((**elem_type).clone()),
            Type::String => Ok(Type::String), // Indexing string returns string (char)
            Type::Unknown => Ok(Type::Unknown),
            _ => {
                self.add_error(SemanticError::not_indexable(object_type, span));
                Ok(Type::Unknown)
            }
        }
    }

    /// Analyze a member access
    fn analyze_member(
        &mut self,
        object: &Expr,
        _property: &str,
        span: crate::source::Span,
    ) -> Result<Type> {
        let object_type = self.analyze_expr(object)?;

        // TODO: Implement proper member access when we have structs
        match &object_type {
            Type::Unknown => Ok(Type::Unknown),
            _ => {
                self.add_error(SemanticError::invalid_member_access(object_type.clone(), span));
                Ok(Type::Unknown)
            }
        }
    }

    /// Analyze an if expression
    fn analyze_if(
        &mut self,
        condition: &Expr,
        then_branch: &Expr,
        else_branch: Option<&Expr>,
        _span: crate::source::Span,
    ) -> Result<Type> {
        // Analyze condition
        let cond_type = self.analyze_expr(condition)?;
        if cond_type != Type::Bool && cond_type != Type::Unknown {
            self.add_error(SemanticError::type_mismatch(Type::Bool, cond_type, condition.span));
        }

        // Analyze branches
        let then_type = self.analyze_expr(then_branch)?;
        
        if let Some(else_expr) = else_branch {
            let _else_type = self.analyze_expr(else_expr)?;
            // TODO: Check that branch types are compatible
            Ok(then_type)
        } else {
            // If expression without else branch has unit type
            Ok(Type::Unknown)
        }
    }

    /// Analyze a block expression
    fn analyze_block_expr(&mut self, block: &Block) -> Result<Type> {
        // Enter new scope
        self.symbol_table.enter_scope();
        self.inference_ctx.push_scope();

        // Analyze block
        self.analyze_block(block)?;

        // Exit scope
        self.symbol_table.exit_scope();
        self.inference_ctx.pop_scope();

        // Block type is the type of the final expression
        // TODO: Implement proper block typing
        Ok(Type::Unknown)
    }

    /// Analyze an array expression
    fn analyze_array(&mut self, elements: &[Expr], _span: crate::source::Span) -> Result<Type> {
        if elements.is_empty() {
            // Empty array has unknown element type
            return Ok(Type::Array(Box::new(Type::Unknown)));
        }

        // Analyze all elements
        let element_types: Vec<Type> = elements
            .iter()
            .map(|e| self.analyze_expr(e))
            .collect::<Result<Vec<_>>>()?;

        // TODO: Check that all elements have compatible types
        // For now, use the type of the first element
        Ok(Type::Array(Box::new(element_types[0].clone())))
    }

    /// Analyze an assignment
    fn analyze_assign(
        &mut self,
        target: &Expr,
        value: &Expr,
        span: crate::source::Span,
    ) -> Result<Type> {
        // Analyze value first
        let value_type = self.analyze_expr(value)?;

        // Check assignment target
        match &target.kind {
            ExprKind::Identifier(name) => {
                if let Some(symbol) = self.symbol_table.lookup(name) {
                    let symbol_id = symbol.id;
                    let is_mutable = symbol.is_mutable;
                    
                    if !is_mutable {
                        self.add_error(SemanticError::assignment_to_immutable(name, span));
                    }
                    
                    self.symbol_table.mark_used(symbol_id);
                    
                    // TODO: Check type compatibility
                } else {
                    self.add_error(SemanticError::undefined_variable(name, target.span));
                }
            }
            ExprKind::Index { object, index } => {
                self.analyze_index(object, index, target.span)?;
            }
            ExprKind::Member { object, property } => {
                self.analyze_member(object, property, target.span)?;
            }
            _ => {
                self.add_error(SemanticError::invalid_assignment_target(target.span));
            }
        }

        Ok(value_type)
    }

    /// Analyze a match expression
    fn analyze_match(&mut self, expr: &Expr, arms: &[crate::parser::MatchArm], _span: crate::source::Span) -> Result<Type> {
        // Analyze the expression being matched
        let expr_type = self.analyze_expr(expr)?;
        
        if arms.is_empty() {
            self.add_error(SemanticError::new(
                SemanticErrorKind::InvalidOperation {
                    op: "match".to_string(),
                    ty: Type::Unknown,
                },
                expr.span,
            ).with_note("Match expression must have at least one arm".to_string()));
            return Ok(Type::Unknown);
        }
        
        // Analyze each arm
        let mut result_type = None;
        for arm in arms {
            // Enter new scope for pattern variables
            self.symbol_table.enter_scope();
            
            // Analyze pattern (this would add variables to scope)
            self.analyze_pattern(&arm.pattern, &expr_type)?;
            
            // Analyze guard if present
            if let Some(guard) = &arm.guard {
                let guard_type = self.analyze_expr(guard)?;
                if guard_type != Type::Bool {
                    self.add_error(SemanticError::type_mismatch(
                        Type::Bool,
                        guard_type,
                        guard.span,
                    ));
                }
            }
            
            // Analyze arm body
            let body_type = self.analyze_expr(&arm.body)?;
            
            // Check that all arms have the same return type
            if let Some(expected_type) = &result_type {
                if body_type != *expected_type {
                    self.add_error(SemanticError::type_mismatch(
                        expected_type.clone(),
                        body_type,
                        arm.body.span,
                    ));
                }
            } else {
                result_type = Some(body_type);
            }
            
            // Exit scope
            self.symbol_table.exit_scope();
        }
        
        Ok(result_type.unwrap_or(Type::Unknown))
    }

    /// Analyze a pattern and add bindings to the symbol table
    fn analyze_pattern(&mut self, pattern: &crate::parser::Pattern, expected_type: &Type) -> Result<()> {
        use crate::parser::PatternKind;
        
        match &pattern.kind {
            PatternKind::Wildcard => {
                // Wildcard pattern - no bindings
                Ok(())
            }
            PatternKind::Literal(_literal) => {
                // Literal pattern - check type compatibility in type inference
                Ok(())
            }
            PatternKind::Identifier(name) => {
                // Variable binding - add to symbol table
                match self.symbol_table.define_variable(
                    name.clone(),
                    expected_type.clone(),
                    pattern.span,
                    false, // Pattern variables are immutable by default
                ) {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        self.add_error(SemanticError::duplicate_variable(name, pattern.span)
                            .with_note(err));
                        Ok(())
                    }
                }
            }
            PatternKind::Array(patterns) => {
                // Array destructuring - check each element
                for sub_pattern in patterns {
                    // For now, assume all elements have the same type
                    // TODO: Handle more complex array type checking
                    self.analyze_pattern(sub_pattern, expected_type)?;
                }
                Ok(())
            }
            PatternKind::Object(_fields) => {
                // Object destructuring - TODO: implement properly
                Ok(())
            }
            PatternKind::Or(patterns) => {
                // Or pattern - all alternatives should bind the same variables
                for sub_pattern in patterns {
                    self.analyze_pattern(sub_pattern, expected_type)?;
                }
                Ok(())
            }
        }
    }

    /// Get the symbol table (for testing and debugging)
    pub fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }

    /// Get collected errors
    pub fn errors(&self) -> &[SemanticError] {
        &self.errors
    }
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn analyze_program(source: &str) -> Result<SemanticAnalyzer> {
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.scan_tokens();
        if !errors.is_empty() {
            return Err(errors[0].clone());
        }
        let mut parser = Parser::new(tokens);
        let program = parser.parse()?;
        
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze_program(&program)?;
        Ok(analyzer)
    }

    #[test]
    fn test_variable_definition() {
        let analyzer = analyze_program("let x: i32 = 42;").unwrap();
        assert!(analyzer.symbol_table().lookup("x").is_some());
    }

    #[test]
    fn test_undefined_variable() {
        let result = analyze_program("x + 1;");
        assert!(result.is_err());
    }

    #[test]
    fn test_function_definition() {
        let analyzer = analyze_program("fn add(x: i32, y: i32) -> i32 { x + y }").unwrap();
        assert!(analyzer.symbol_table().lookup("add").is_some());
    }

    #[test]
    fn test_variable_shadowing() {
        let analyzer = analyze_program(r#"
            let x: i32 = 1;
            {
                let x: f32 = 2.0;
            }
        "#).unwrap();
        
        // Should complete without errors
        assert!(analyzer.errors().is_empty());
    }

    #[test]
    fn test_duplicate_variable_error() {
        let result = analyze_program(r#"
            let x: i32 = 1;
            let x: f32 = 2.0;
        "#);
        assert!(result.is_err());
    }
}