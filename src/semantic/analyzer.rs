use crate::inference::{type_ann_to_type, InferenceContext};
use crate::parser::{
    BinaryOp, Block, ExportKind, Expr, ExprKind, GenericParams, ImportSpecifier, Literal, Param,
    Program, Stmt, StmtKind, TraitBound, TypeAnn, UnaryOp,
};
use crate::source::Span;
use crate::types::Type;
use crate::Result;
use std::collections::HashMap;

use super::error::{SemanticError, SemanticErrorKind};
use super::memory_safety::{MemorySafetyContext, MemorySafetyViolation};
use super::symbol::FunctionSignature;
use super::symbol_table::SymbolTable;

/// Context for the current analysis
#[derive(Debug)]
struct AnalysisContext {
    /// Current function return type (None if not in a function)
    current_function_return: Option<Type>,
    /// Whether we're currently in a loop
    in_loop: bool,
    /// Whether the current function is marked as @const
    _in_const_function: bool,
    /// Whether we're currently in an async function
    in_async_function: bool,
    /// Generic parameters in the current scope
    generic_params: Option<crate::parser::GenericParams>,
}

impl AnalysisContext {
    fn new() -> Self {
        AnalysisContext {
            current_function_return: None,
            in_loop: false,
            _in_const_function: false,
            in_async_function: false,
            generic_params: None,
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
    /// Memory safety analysis context
    memory_safety_ctx: MemorySafetyContext,
    /// Stack of analysis contexts
    context_stack: Vec<AnalysisContext>,
    /// Collected errors
    errors: Vec<SemanticError>,
    /// Whether memory safety analysis is enabled
    memory_safety_enabled: bool,
}

impl SemanticAnalyzer {
    /// Create a new semantic analyzer
    pub fn new() -> Self {
        SemanticAnalyzer {
            symbol_table: SymbolTable::new(),
            inference_ctx: InferenceContext::new(),
            memory_safety_ctx: MemorySafetyContext::new(),
            context_stack: vec![AnalysisContext::new()],
            errors: Vec::new(),
            memory_safety_enabled: true,
        }
    }

    /// Create a new semantic analyzer with memory safety analysis disabled
    pub fn new_without_memory_safety() -> Self {
        let mut analyzer = Self::new();
        analyzer.memory_safety_enabled = false;
        analyzer
    }

    /// Enable or disable memory safety analysis
    pub fn set_memory_safety_enabled(&mut self, enabled: bool) {
        self.memory_safety_enabled = enabled;
    }

    /// Get the current analysis context
    fn current_context(&self) -> &AnalysisContext {
        self.context_stack
            .last()
            .expect("context stack should never be empty")
    }

    /// Get the current analysis context mutably
    fn current_context_mut(&mut self) -> &mut AnalysisContext {
        self.context_stack
            .last_mut()
            .expect("context stack should never be empty")
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

    /// Add memory safety violations as semantic errors
    fn add_memory_safety_violations(&mut self, violations: Vec<MemorySafetyViolation>) {
        for violation in violations {
            let span = self.get_violation_span(&violation);
            let error = SemanticError::memory_safety_violation(violation, span);
            self.add_error(error);
        }
    }

    /// Extract span from memory safety violation
    fn get_violation_span(&self, violation: &MemorySafetyViolation) -> Span {
        match violation {
            MemorySafetyViolation::UseAfterFree { use_span, .. } => *use_span,
            MemorySafetyViolation::DoubleFree { second_free, .. } => *second_free,
            MemorySafetyViolation::UseOfUninitialized { use_span, .. } => *use_span,
            MemorySafetyViolation::NullDereference { span, .. } => *span,
            MemorySafetyViolation::BufferOverflow { index_span, .. } => *index_span,
            MemorySafetyViolation::ConflictingBorrow { new_borrow, .. } => *new_borrow,
            MemorySafetyViolation::UseOfMoved { use_span, .. } => *use_span,
            MemorySafetyViolation::LifetimeExceeded { use_span, .. } => *use_span,
            MemorySafetyViolation::PotentialLeak { allocated_span, .. } => *allocated_span,
        }
    }

    /// Add an error with enhanced context information
    fn add_enhanced_error(&mut self, error: SemanticError, source_context: Option<&str>) {
        let enhanced_error = if let Some(context) = source_context {
            error.with_note(format!("Source context: {}", context))
        } else {
            error
        };
        self.errors.push(enhanced_error);
    }

    /// Analyze a program
    pub fn analyze_program(&mut self, program: &Program) -> Result<()> {
        // Add built-in functions to the global scope
        self.add_builtins()?;

        // Analyze all statements
        for stmt in &program.statements {
            self.analyze_stmt(stmt)?;
        }

        // Finalize memory safety analysis
        if self.memory_safety_enabled {
            self.finalize_memory_safety_analysis();
        }

        // Check for unused symbols
        let unused_symbols: Vec<_> = self
            .symbol_table
            .get_unused_symbols()
            .into_iter()
            .map(|s| (s.name.clone(), s.def_span))
            .collect();

        for (_name, _span) in unused_symbols {
            // For now, we'll skip reporting unused variables as errors
            // This would normally be a warning, not an error
            continue;
        }

        // Don't return errors here - let the caller check via errors() method
        // This allows us to collect and report all errors at once
        Ok(())
    }

    /// Finalize memory safety analysis and collect any remaining violations
    fn finalize_memory_safety_analysis(&mut self) {
        // Collect all remaining memory safety violations
        let violations = self.memory_safety_ctx.violations().to_vec();
        self.add_memory_safety_violations(violations);
    }

    /// Add built-in functions
    fn add_builtins(&mut self) -> Result<()> {
        // print function: (unknown) -> void
        let print_sig = FunctionSignature {
            generic_params: None,
            params: vec![("value".to_string(), Type::Unknown)],
            return_type: Type::Unknown, // void
            is_const: false,
            is_async: false,
            generic_params: vec![],
        };
        self.symbol_table
            .define_function(
                "print".to_string(),
                print_sig,
                crate::source::Span::single(crate::source::SourceLocation::initial()),
            )
            .map_err(|e| {
                SemanticError::new(
                    SemanticErrorKind::DuplicateFunction(e),
                    crate::source::Span::single(crate::source::SourceLocation::initial()),
                )
                .into_error()
            })?;

        // len function: ([T]) -> i32
        let len_sig = FunctionSignature {
            generic_params: None,
            params: vec![("array".to_string(), Type::Array(Box::new(Type::Unknown)))],
            return_type: Type::I32,
            is_const: true,
            is_async: false,
            generic_params: vec![],
        };
        self.symbol_table
            .define_function(
                "len".to_string(),
                len_sig,
                crate::source::Span::single(crate::source::SourceLocation::initial()),
            )
            .map_err(|e| {
                SemanticError::new(
                    SemanticErrorKind::DuplicateFunction(e),
                    crate::source::Span::single(crate::source::SourceLocation::initial()),
                )
                .into_error()
            })?;

        Ok(())
    }

    /// Analyze a statement
    fn analyze_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        // Check for @const attribute on functions and variables
        let has_const_attr = stmt.attributes.iter().any(|attr| attr.name == "const");

        match &stmt.kind {
            StmtKind::Let {
                name,
                type_ann,
                init,
            } => {
                self.analyze_let_with_attributes(
                    name,
                    type_ann.as_ref(),
                    init.as_ref(),
                    has_const_attr,
                    stmt.span,
                )?;
            }
            StmtKind::Function {
                name,
                params,
                ret_type,
                body,
                is_async,
                generic_params,
            } => {
                self.analyze_function_with_attributes(
                    name,
                    generic_params.as_ref(),
                    params,
                    ret_type.as_ref(),
                    body,
                    *is_async,
                    has_const_attr,
                    stmt.span,
                )?;
            }
            StmtKind::Import { imports, module } => {
                self.analyze_import_stmt(imports, module, stmt.span)?;
            }
            StmtKind::Export { export } => {
                self.analyze_export_stmt(export, stmt.span)?;
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
            StmtKind::For {
                variable,
                iterable,
                body,
            } => {
                self.analyze_for(variable, iterable, body)?;
            }
            StmtKind::Struct {
                name,
                generic_params,
                fields,
            } => {
                // TODO: Implement struct definition analysis
                // For now, just suppress warnings
                let _ = (name, generic_params, fields);
            }
            StmtKind::Enum {
                name,
                generic_params,
                variants,
            } => {
                // TODO: Implement enum definition analysis
                // For now, just suppress warnings
                let _ = (name, generic_params, variants);
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
            let declared_type = type_ann_to_type(type_ann);

            // If there's also an initializer, check type compatibility
            if let Some(init_expr) = init {
                let init_type = self.analyze_expr(init_expr)?;

                // Check type compatibility
                if !declared_type.is_unknown()
                    && !init_type.is_unknown()
                    && !self.is_assignable_to(&init_type, &declared_type)
                {
                    self.add_error(
                        SemanticError::type_mismatch(
                            declared_type.clone(),
                            init_type.clone(),
                            span,
                        )
                        .with_note(format!(
                            "Cannot initialize variable '{}' of type {} with value of type {}",
                            name, declared_type, init_type
                        )),
                    );
                }
            }

            declared_type
        } else if let Some(init_expr) = init {
            // Infer type from initializer
            self.analyze_expr(init_expr)?
        } else {
            // No type annotation and no initializer
            self.add_error(
                SemanticError::new(
                    SemanticErrorKind::TypeMismatch {
                        expected: Type::Unknown,
                        found: Type::Unknown,
                    },
                    span,
                )
                .with_note(
                    "variables must have either a type annotation or an initializer".to_string(),
                ),
            );
            Type::Unknown
        };

        // Define the variable
        match self
            .symbol_table
            .define_variable(name.to_string(), ty.clone(), span, true)
        {
            Ok(symbol_id) => {
                // If there's an initializer, mark the variable as used
                if init.is_some() {
                    self.symbol_table.mark_used(symbol_id);
                }

                // Memory safety analysis
                if self.memory_safety_enabled {
                    if let Err(err) = self.memory_safety_ctx.define_variable(
                        name.to_string(),
                        ty,
                        true, // Assume mutable for now - would be determined by syntax
                        span,
                    ) {
                        // This is a duplicate definition, but not a memory safety issue per se
                        // The symbol table already handles this
                    }

                    // Initialize variable if there's an initializer
                    if init.is_some() {
                        if let Err(_) = self.memory_safety_ctx.initialize_variable(name, span) {
                            // Variable not found - shouldn't happen
                        }
                    }
                }
            }
            Err(err) => {
                self.add_error(SemanticError::duplicate_variable(name, span).with_note(err));
            }
        }

        Ok(())
    }

    /// Analyze a function definition
    fn analyze_function(
        &mut self,
        name: &str,
        generic_params: Option<&crate::parser::GenericParams>,
        params: &[Param],
        ret_type: Option<&TypeAnn>,
        body: &Block,
        is_async: bool,
        span: crate::source::Span,
    ) -> Result<()> {
        // Convert parameter types
        let param_types: Vec<(String, Type)> = params
            .iter()
            .map(|p| (p.name.clone(), type_ann_to_type(&p.type_ann)))
            .collect();

        // Convert return type
        let base_return_type = ret_type.map(type_ann_to_type).unwrap_or(Type::Unknown);

        // For async functions, wrap return type in Future<T>
        let return_type = if is_async {
            Type::Future(Box::new(base_return_type.clone()))
        } else {
            base_return_type.clone()
        };

        // Create function signature
        let signature = FunctionSignature {
            generic_params: generic_params.cloned(),
            params: param_types.clone(),
            return_type: return_type.clone(),
            is_const: false, // Legacy method - const handled in new method
            is_async,
            generic_params: vec![], // TODO: Implement generic parameter conversion
        };

        // Define the function
        match self
            .symbol_table
            .define_function(name.to_string(), signature, span)
        {
            Ok(_) => {}
            Err(err) => {
                self.add_error(
                    SemanticError::new(
                        SemanticErrorKind::DuplicateFunction(name.to_string()),
                        span,
                    )
                    .with_note(err),
                );
            }
        }

        // Enter function scope
        self.symbol_table.enter_scope();
        self.inference_ctx.push_scope();

        // Push function context with generic parameters
        // For async functions, we need to check returns against the unwrapped type
        let mut func_context = AnalysisContext {
            current_function_return: Some(base_return_type),
            in_loop: false,
            _in_const_function: false, // TODO: Support @const
            in_async_function: is_async,
            generic_params: generic_params.cloned(),
        };

        // Extract generic parameters if present
        if let Some(generics) = generic_params {
            for param in &generics.params {
                func_context
                    .generic_params
                    .insert(param.name.clone(), param.bounds.clone());
            }
        }

        self.push_context(func_context);

        // Define generic type parameters in scope
        if let Some(generics) = generic_params {
            for generic_param in &generics.params {
                // Add type parameter to type environment
                // For now, we'll treat type parameters as type placeholders
                // Future: track bounds and constraints
                self.inference_ctx.define_type_param(&generic_param.name);
            }
        }

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
                    self.add_error(
                        SemanticError::duplicate_variable(&param.name, param.type_ann.span)
                            .with_note(err),
                    );
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

    /// Analyze a function definition with attribute support
    fn analyze_function_with_attributes(
        &mut self,
        name: &str,
        generic_params: Option<&crate::parser::GenericParams>,
        params: &[Param],
        ret_type: Option<&TypeAnn>,
        body: &Block,
        is_async: bool,
        is_const: bool,
        span: crate::source::Span,
    ) -> Result<()> {
        // Validate const function constraints
        if is_const {
            self.validate_const_function_constraints(name, params, ret_type, body, is_async, span)?;
        }

        // Convert parameter types
        let param_types: Vec<(String, Type)> = params
            .iter()
            .map(|p| (p.name.clone(), type_ann_to_type(&p.type_ann)))
            .collect();

        // Convert return type
        let base_return_type = ret_type.map(type_ann_to_type).unwrap_or(Type::Unknown);

        // For async functions, wrap return type in Future<T>
        let return_type = if is_async {
            Type::Future(Box::new(base_return_type.clone()))
        } else {
            base_return_type.clone()
        };

        // Create function signature with const flag and generic params
        let signature = FunctionSignature {
            generic_params: generic_params.cloned(),
            params: param_types.clone(),
            return_type: return_type.clone(),
            is_const,
            is_async,
            generic_params: vec![], // TODO: Implement generic parameter conversion
        };

        // Define the function
        match self
            .symbol_table
            .define_function(name.to_string(), signature, span)
        {
            Ok(_) => {}
            Err(err) => {
                self.add_error(
                    SemanticError::new(
                        SemanticErrorKind::DuplicateFunction(name.to_string()),
                        span,
                    )
                    .with_note(err),
                );
            }
        }

        // Enter function scope
        self.symbol_table.enter_scope();
        self.inference_ctx.push_scope();

        // Push function context with const flag and generic params
        let func_context = AnalysisContext {
            current_function_return: Some(base_return_type),
            in_loop: false,
            _in_const_function: is_const,
            in_async_function: is_async,
            generic_params: generic_params.cloned(),
        };

        // Extract generic parameters if present
        if let Some(generics) = generic_params {
            for param in &generics.params {
                func_context
                    .generic_params
                    .insert(param.name.clone(), param.bounds.clone());
            }
        }

        self.push_context(func_context);

        // Define generic type parameters in scope
        if let Some(generics) = generic_params {
            for generic_param in &generics.params {
                // Add type parameter to type environment
                // For now, we'll treat type parameters as type placeholders
                // Future: track bounds and constraints
                self.inference_ctx.define_type_param(&generic_param.name);
            }
        }

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
                    self.add_error(
                        SemanticError::duplicate_variable(&param.name, param.type_ann.span)
                            .with_note(err),
                    );
                }
            }
        }

        // Analyze function body with const validation if needed
        if is_const {
            self.analyze_const_function_body(body)?;
        } else {
            self.analyze_block(body)?;
        }

        // Exit function context
        self.pop_context();
        self.symbol_table.exit_scope();
        self.inference_ctx.pop_scope();

        Ok(())
    }

    /// Analyze a let statement with attribute support
    fn analyze_let_with_attributes(
        &mut self,
        name: &str,
        type_ann: Option<&TypeAnn>,
        init: Option<&Expr>,
        is_const: bool,
        span: crate::source::Span,
    ) -> Result<()> {
        // For @const variables, validate that initializer is a const expression
        if is_const {
            if let Some(init_expr) = init {
                self.validate_const_expression(init_expr)?;
            } else {
                self.add_error(SemanticError::const_function_violation(
                    "@const variables must have an initializer",
                    span,
                ));
            }
        }

        // Delegate to existing analyze_let method
        self.analyze_let(name, type_ann, init, span)
    }

    /// Analyze a return statement
    fn analyze_return(&mut self, expr: Option<&Expr>, span: crate::source::Span) -> Result<()> {
        // Check if we're in a function and get the expected return type
        let expected_type = {
            let ctx = self.current_context();
            if ctx.current_function_return.is_none() {
                self.add_error(SemanticError::new(
                    SemanticErrorKind::ReturnOutsideFunction,
                    span,
                ));
                return Ok(());
            }
            ctx.current_function_return.clone().unwrap()
        };

        match expr {
            // Return with expression
            Some(expr) => {
                // Analyze the expression and get its type
                let actual_type = self.analyze_expr(expr)?;

                // Check if we're returning a value from a void function
                if expected_type == Type::Unknown && actual_type != Type::Unknown {
                    // For now, we'll treat Unknown as void when it's the expected return type
                    // This might need adjustment based on your language semantics
                    self.add_error(
                        SemanticError::return_type_mismatch(Type::Unknown, actual_type, span)
                            .with_note(
                                "function has no return type annotation, cannot return a value"
                                    .to_string(),
                            ),
                    );
                } else if !actual_type.is_assignable_to(&expected_type) {
                    // Type mismatch
                    self.add_error(SemanticError::return_type_mismatch(
                        expected_type,
                        actual_type,
                        span,
                    ));
                }
            }
            // Return without expression
            None => {
                // Check if function expects a return value
                if expected_type != Type::Unknown {
                    self.add_error(
                        SemanticError::return_type_mismatch(expected_type, Type::Unknown, span)
                            .with_note("missing return value".to_string()),
                    );
                }
            }
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
                self.add_error(
                    SemanticError::duplicate_variable(variable, iterable.span).with_note(err),
                );
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

    /// Analyze an import statement
    fn analyze_import_stmt(
        &mut self,
        specifiers: &Vec<ImportSpecifier>,
        source: &str,
        span: crate::source::Span,
    ) -> Result<()> {
        // Process the import through the symbol table
        match self.symbol_table.process_import(specifiers, source, span) {
            Ok(()) => {}
            Err(err) => {
                // Convert symbol table errors to semantic errors
                let semantic_error = if err.contains("not found") {
                    if err.contains("Symbol") {
                        // Extract symbol name from error message for better error reporting
                        let symbol_name = err.split('\'').nth(1).unwrap_or("unknown");
                        SemanticError::undefined_import(symbol_name, source, span)
                    } else {
                        SemanticError::module_not_found(source, span)
                    }
                } else if err.contains("already imported") {
                    let symbol_name = err.split('\'').nth(1).unwrap_or("unknown");
                    SemanticError::conflicting_import(symbol_name, span)
                } else {
                    SemanticError::module_error(&err, span)
                };

                self.add_error(semantic_error);
            }
        }

        Ok(())
    }

    /// Analyze an export statement
    fn analyze_export_stmt(&mut self, kind: &ExportKind, span: crate::source::Span) -> Result<()> {
        // Handle different export kinds
        match kind {
            ExportKind::Function {
                name,
                params,
                ret_type,
                body,
                is_async,
            } => {
                // Analyze the function first
                // Export doesn't support generic params yet
                self.analyze_function(
                    name,
                    None,
                    params,
                    ret_type.as_ref(),
                    body,
                    *is_async,
                    span,
                )?;

                // Then mark it as exported
                if let Err(err) = self.symbol_table.process_export(kind, span) {
                    self.add_error(SemanticError::module_error(&err, span));
                }
            }
            ExportKind::Variable {
                name,
                type_ann,
                init,
            } => {
                // Analyze the variable first
                self.analyze_let(name, type_ann.as_ref(), init.as_ref(), span)?;

                // Then mark it as exported
                if let Err(err) = self.symbol_table.process_export(kind, span) {
                    self.add_error(SemanticError::module_error(&err, span));
                }
            }
            ExportKind::Named { specifiers } => {
                // For named exports, verify all symbols exist before exporting
                for spec in specifiers {
                    if self.symbol_table.lookup(&spec.name).is_none() {
                        self.add_error(SemanticError::undefined_variable(&spec.name, span));
                    }
                }

                if let Err(err) = self.symbol_table.process_export(kind, span) {
                    self.add_error(SemanticError::module_error(&err, span));
                }
            }
            ExportKind::Default { expr } => {
                // Analyze the default export expression
                self.analyze_expr(expr)?;

                // Process the export through the symbol table
                if let Err(err) = self.symbol_table.process_export(kind, span) {
                    self.add_error(SemanticError::module_error(&err, span));
                }
            }
            ExportKind::Declaration(stmt) => {
                // Analyze the declaration being exported
                self.analyze_stmt(stmt)?;

                // Process the export through the symbol table
                if let Err(err) = self.symbol_table.process_export(kind, span) {
                    self.add_error(SemanticError::module_error(&err, span));
                }
            }
        }

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
            ExprKind::Member { object, property } => {
                self.analyze_member(object, property, expr.span)
            }
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => self.analyze_if(condition, then_branch, else_branch.as_deref(), expr.span),
            ExprKind::Block(block) => self.analyze_block_expr(block),
            ExprKind::Array(elements) => self.analyze_array(elements, expr.span),
            ExprKind::Assign { target, value } => self.analyze_assign(target, value, expr.span),
            ExprKind::Match {
                expr: match_expr,
                arms,
            } => self.analyze_match(match_expr, arms, expr.span),
            ExprKind::Await { expr: inner } => self.analyze_await(inner, expr.span),
            ExprKind::ListComprehension { .. } => {
                // List comprehensions not yet implemented
                // TODO: Implement proper list comprehension analysis
                Ok(Type::Array(Box::new(Type::Unknown)))
            }
            ExprKind::GenericConstructor { name, type_args } => {
                // For now, treat generic constructors as named types
                // TODO: Implement proper generic type analysis and resolution
                let _ = type_args; // suppress warning
                Ok(Type::Named(name.clone()))
            }
            ExprKind::StructConstructor { name, fields } => {
                // TODO: Implement struct constructor analysis
                // For now, analyze field expressions and return struct type
                for (_, field_expr) in fields {
                    self.analyze_expr(field_expr)?;
                }
                Ok(Type::Named(name.clone()))
            }
            ExprKind::EnumConstructor {
                enum_name,
                variant,
                args,
            } => {
                // TODO: Implement enum constructor analysis
                // For now, analyze arguments and return enum type
                match args {
                    crate::parser::EnumConstructorArgs::Unit => {}
                    crate::parser::EnumConstructorArgs::Tuple(exprs) => {
                        for arg_expr in exprs {
                            self.analyze_expr(arg_expr)?;
                        }
                    }
                    crate::parser::EnumConstructorArgs::Struct(fields) => {
                        for (_, field_expr) in fields {
                            self.analyze_expr(field_expr)?;
                        }
                    }
                }
                if let Some(enum_name) = enum_name {
                    Ok(Type::Named(enum_name.clone()))
                } else {
                    // Unqualified variant - would need context to resolve
                    Ok(Type::Named(variant.clone()))
                }
            }
        }
    }

    /// Analyze a literal
    fn analyze_literal(&mut self, lit: &Literal) -> Result<Type> {
        Ok(match lit {
            Literal::Number(n) => {
                // For now, treat integers as i32 and floats as f32
                if n.fract() == 0.0 && *n >= i32::MIN as f64 && *n <= i32::MAX as f64 {
                    Type::I32
                } else {
                    Type::F32
                }
            }
            Literal::String(_) => Type::String,
            Literal::Boolean(_) => Type::Bool,
            Literal::Null => Type::Option(Box::new(Type::Unknown)),
        })
    }

    /// Analyze an identifier
    fn analyze_identifier(&mut self, name: &str, span: crate::source::Span) -> Result<Type> {
        // First check if it's a type parameter in the current generic context
        let ctx = self.current_context();
        if ctx.generic_params.contains_key(name) {
            // This is a type parameter reference
            return Ok(Type::TypeParam(name.to_string()));
        }

        // Use module-aware lookup
        if let Some(symbol) = self.symbol_table.lookup_with_modules(name) {
            let symbol_id = symbol.id;
            let ty = symbol.ty.clone();

            // Mark as used
            self.symbol_table.mark_used(symbol_id);

            // Memory safety analysis
            if self.memory_safety_enabled {
                match self.memory_safety_ctx.use_variable(name, span) {
                    Ok(_) => {
                        // Variable use is memory-safe
                    }
                    Err(violation) => {
                        self.add_memory_safety_violations(vec![violation]);
                    }
                }
            }

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

        // Check type compatibility for the operation
        Ok(match op {
            // Arithmetic operations: require numeric types
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
                // Check if both operands are numeric or unknown
                let left_numeric = left_type.is_numeric() || left_type == Type::Unknown;
                let right_numeric = right_type.is_numeric() || right_type == Type::Unknown;

                if !left_numeric || !right_numeric {
                    self.add_error(SemanticError::invalid_binary_operation(
                        &op.to_string(),
                        left_type.clone(),
                        right_type.clone(),
                        span,
                    ).with_note(format!(
                        "arithmetic operations require numeric types (i32 or f32), but found {} and {}",
                        left_type, right_type
                    )));
                    return Ok(Type::Unknown);
                }

                // If both are unknown, result is unknown
                if left_type == Type::Unknown && right_type == Type::Unknown {
                    Type::Unknown
                } else if left_type == Type::Unknown {
                    // Left is unknown, use right type
                    right_type
                } else if right_type == Type::Unknown {
                    // Right is unknown, use left type
                    left_type
                } else if left_type == right_type {
                    // Both are the same numeric type
                    left_type
                } else {
                    // Mixed numeric types (i32 and f32)
                    // For now, we don't allow implicit conversions
                    self.add_error(
                        SemanticError::type_mismatch(left_type.clone(), right_type.clone(), span)
                            .with_note(format!(
                        "cannot perform {} operation between {} and {} without explicit conversion",
                        op.to_string(), left_type, right_type
                    )),
                    );
                    Type::Unknown
                }
            }

            // Comparison operations: require numeric types
            BinaryOp::Less | BinaryOp::Greater | BinaryOp::LessEqual | BinaryOp::GreaterEqual => {
                // Check if both operands are numeric
                let left_numeric = left_type.is_numeric() || left_type == Type::Unknown;
                let right_numeric = right_type.is_numeric() || right_type == Type::Unknown;

                if !left_numeric || !right_numeric {
                    self.add_error(SemanticError::invalid_binary_operation(
                        &op.to_string(),
                        left_type.clone(),
                        right_type.clone(),
                        span,
                    ).with_note(format!(
                        "comparison operations require numeric types (i32 or f32), but found {} and {}",
                        left_type, right_type
                    )));
                    return Ok(Type::Bool);
                }

                // Check that types are compatible (same type or unknown)
                if left_type != Type::Unknown
                    && right_type != Type::Unknown
                    && left_type != right_type
                {
                    self.add_error(SemanticError::type_mismatch(
                        left_type.clone(),
                        right_type.clone(),
                        span,
                    ).with_note(format!(
                        "comparison operations require operands of the same type, but found {} and {}",
                        left_type, right_type
                    )));
                }

                Type::Bool
            }

            // Equality operations: require same type
            BinaryOp::Equal | BinaryOp::NotEqual => {
                // Allow comparison of any types, but they should be the same
                if left_type != Type::Unknown
                    && right_type != Type::Unknown
                    && left_type != right_type
                {
                    self.add_error(SemanticError::type_mismatch(
                        left_type.clone(),
                        right_type.clone(),
                        span,
                    ).with_note(format!(
                        "equality operations require operands of the same type, but found {} and {}",
                        left_type, right_type
                    )));
                }

                Type::Bool
            }

            // Logical operations: require bool types
            BinaryOp::And | BinaryOp::Or => {
                // Check if left operand is bool
                if left_type != Type::Bool && left_type != Type::Unknown {
                    self.add_error(
                        SemanticError::type_mismatch(Type::Bool, left_type.clone(), left.span)
                            .with_note(format!(
                        "logical {} operation requires bool operands, but left operand has type {}",
                        op.to_string(), left_type
                    )),
                    );
                }

                // Check if right operand is bool
                if right_type != Type::Bool && right_type != Type::Unknown {
                    self.add_error(SemanticError::type_mismatch(
                        Type::Bool,
                        right_type.clone(),
                        right.span,
                    ).with_note(format!(
                        "logical {} operation requires bool operands, but right operand has type {}",
                        op.to_string(), right_type
                    )));
                }

                Type::Bool
            }
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
            UnaryOp::Minus => {
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
            // Analyze arguments and collect their types
            let arg_types: Vec<Type> = args
                .iter()
                .map(|arg| self.analyze_expr(arg))
                .collect::<Result<Vec<_>>>()?;

            // Check if function exists at all (including imports)
            let symbol_info = self
                .symbol_table
                .lookup_with_modules(name)
                .map(|s| (s.id, s.ty.clone(), s.function_signature().cloned()));

            if let Some((func_id, symbol_type, maybe_signature)) = symbol_info {
                if let Some(signature) = maybe_signature {
                    // Handle generic functions
                    let instantiated_signature = if signature.generic_params.is_some() {
                        // TODO: Implement generic type instantiation
                        // For now, we'll create a simple instantiation that replaces
                        // type parameters with the actual argument types
                        self.instantiate_generic_function(&signature, &arg_types)?
                    } else {
                        signature.clone()
                    };

                    // Check argument count
                    if args.len() != instantiated_signature.params.len() {
                        self.add_error(
                            SemanticError::argument_count_mismatch(
                                instantiated_signature.params.len(),
                                args.len(),
                                span,
                            )
                            .with_note(format!(
                                "function '{}' expects {} argument{}, but {} {} provided",
                                name,
                                instantiated_signature.params.len(),
                                if instantiated_signature.params.len() == 1 {
                                    ""
                                } else {
                                    "s"
                                },
                                args.len(),
                                if args.len() == 1 { "was" } else { "were" }
                            )),
                        );

                        // Mark function as used even if there's an error
                        self.symbol_table.mark_used(func_id);
                        return Ok(instantiated_signature.return_type.clone());
                    }

                    // Check each argument type
                    for (i, ((param_name, param_type), (arg, arg_type))) in instantiated_signature
                        .params
                        .iter()
                        .zip(args.iter().zip(arg_types.iter()))
                        .enumerate()
                    {
                        // Skip type checking if either type is Unknown (gradual typing)
                        if *param_type == Type::Unknown || *arg_type == Type::Unknown {
                            continue;
                        }

                        // Check if argument type is assignable to parameter type
                        if !arg_type.is_assignable_to(param_type) {
                            self.add_error(
                                SemanticError::type_mismatch(
                                    param_type.clone(),
                                    arg_type.clone(),
                                    arg.span,
                                )
                                .with_note(format!(
                                    "argument {} to function '{}' has wrong type",
                                    i + 1,
                                    name
                                ))
                                .with_note(format!(
                                    "parameter '{}' expects type {}, but argument has type {}",
                                    param_name, param_type, arg_type
                                )),
                            );
                        }
                    }

                    // Mark function as used
                    self.symbol_table.mark_used(func_id);

                    // Return the function's return type even if there were argument type errors
                    return Ok(instantiated_signature.return_type.clone());
                } else {
                    // Symbol exists but is not a function
                    self.add_error(
                        SemanticError::new(SemanticErrorKind::NotCallable(symbol_type), span)
                            .with_note(format!("'{}' is not a function", name)),
                    );
                    return Ok(Type::Unknown);
                }
            } else {
                // Function doesn't exist
                self.add_error(SemanticError::undefined_function(name, span));
                return Ok(Type::Unknown);
            }
        }

        // General case: callee is an expression
        let callee_type = self.analyze_expr(callee)?;

        // Analyze arguments
        let arg_types: Vec<Type> = args
            .iter()
            .map(|arg| self.analyze_expr(arg))
            .collect::<Result<Vec<_>>>()?;

        // Check if callable
        match &callee_type {
            Type::Function { params, ret } => {
                // Check argument count
                if args.len() != params.len() {
                    self.add_error(SemanticError::argument_count_mismatch(
                        params.len(),
                        args.len(),
                        span,
                    ));
                    return Ok((**ret).clone());
                }

                // Check argument types
                for (i, (param_type, (arg, arg_type))) in params
                    .iter()
                    .zip(args.iter().zip(arg_types.iter()))
                    .enumerate()
                {
                    // Skip type checking if either type is Unknown
                    if *param_type == Type::Unknown || *arg_type == Type::Unknown {
                        continue;
                    }

                    if !arg_type.is_assignable_to(param_type) {
                        self.add_error(
                            SemanticError::type_mismatch(
                                param_type.clone(),
                                arg_type.clone(),
                                arg.span,
                            )
                            .with_note(format!("argument {} has wrong type", i + 1)),
                        );
                    }
                }

                Ok((**ret).clone())
            }
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
                self.add_error(SemanticError::invalid_member_access(
                    object_type.clone(),
                    span,
                ));
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
        span: crate::source::Span,
    ) -> Result<Type> {
        // Analyze condition - must be boolean
        let cond_type = self.analyze_expr(condition)?;
        if cond_type != Type::Bool && cond_type != Type::Unknown {
            self.add_error(
                SemanticError::type_mismatch(Type::Bool, cond_type.clone(), condition.span)
                    .with_note(format!(
                        "if condition must be a boolean expression, but found {}",
                        cond_type
                    )),
            );
        }

        // Analyze then branch
        let then_type = self.analyze_expr(then_branch)?;

        if let Some(else_expr) = else_branch {
            // Analyze else branch
            let else_type = self.analyze_expr(else_expr)?;

            // Check that both branches have compatible types
            match (&then_type, &else_type) {
                // If either type is Unknown, use the other type (gradual typing)
                (Type::Unknown, _) => Ok(else_type),
                (_, Type::Unknown) => Ok(then_type),

                // If types are the same, that's the result type
                (t1, t2) if t1 == t2 => Ok(then_type),

                // Otherwise, types are incompatible
                _ => {
                    self.add_error(
                        SemanticError::new(
                            SemanticErrorKind::TypeMismatch {
                                expected: then_type.clone(),
                                found: else_type.clone(),
                            },
                            else_expr.span,
                        )
                        .with_note(format!(
                            "if expression branches must have compatible types"
                        ))
                        .with_note(format!(
                            "then branch has type {}, but else branch has type {}",
                            then_type, else_type
                        ))
                        .with_note(format!(
                            "hint: when using if as an expression, both branches must return the same type"
                        ))
                    );

                    // Return Unknown to prevent cascading errors
                    Ok(Type::Unknown)
                }
            }
        } else {
            // If expression without else branch has unit type (Unknown)
            // This is because the expression doesn't produce a value in all paths
            Ok(Type::Unknown)
        }
    }

    /// Analyze a block expression
    fn analyze_block_expr(&mut self, block: &Block) -> Result<Type> {
        // Enter new scope
        self.symbol_table.enter_scope();
        self.inference_ctx.push_scope();

        if self.memory_safety_enabled {
            self.memory_safety_ctx.enter_scope();
        }

        // Analyze block
        self.analyze_block(block)?;

        // Exit scope
        self.symbol_table.exit_scope();
        self.inference_ctx.pop_scope();

        if self.memory_safety_enabled {
            // Use block end as scope end (approximate)
            let scope_end =
                block
                    .final_expr
                    .as_ref()
                    .map(|e| e.span)
                    .unwrap_or_else(|| {
                        block.statements.last().map(|s| s.span).unwrap_or(
                            crate::source::Span::single(crate::source::SourceLocation::initial()),
                        )
                    });
            self.memory_safety_ctx.exit_scope(scope_end);
        }

        // Block type is the type of the final expression
        // TODO: Implement proper block typing
        Ok(Type::Unknown)
    }

    /// Analyze an array expression
    fn analyze_array(&mut self, elements: &[Expr], span: crate::source::Span) -> Result<Type> {
        if elements.is_empty() {
            // Empty array has unknown element type
            return Ok(Type::Array(Box::new(Type::Unknown)));
        }

        // Analyze all elements
        let mut element_types: Vec<(Type, Span)> = Vec::new();
        for element in elements {
            let elem_type = self.analyze_expr(element)?;
            element_types.push((elem_type, element.span));
        }

        // Find the unified type for all elements
        let unified_type = self.unify_array_element_types(&element_types, span)?;

        Ok(Type::Array(Box::new(unified_type)))
    }

    /// Unify array element types to find a common type
    fn unify_array_element_types(
        &mut self,
        element_types: &[(Type, Span)],
        _array_span: Span,
    ) -> Result<Type> {
        if element_types.is_empty() {
            return Ok(Type::Unknown);
        }

        // Start with the first element's type as the expected type
        let first_type = element_types[0].0.clone();
        let mut has_errors = false;

        // Try to unify with each subsequent element
        for (i, (elem_type, elem_span)) in element_types.iter().enumerate().skip(1) {
            match (&first_type, elem_type) {
                // If either type is Unknown, continue (gradual typing)
                (Type::Unknown, _) | (_, Type::Unknown) => {}

                // If types are equal, continue
                (t1, t2) if t1 == t2 => {}

                // Otherwise, types are incompatible
                _ => {
                    // Report error showing which elements have incompatible types
                    self.add_error(
                        SemanticError::new(
                            SemanticErrorKind::TypeMismatch {
                                expected: first_type.clone(),
                                found: elem_type.clone(),
                            },
                            *elem_span,
                        )
                        .with_note(format!("array elements must have the same type"))
                        .with_note(format!(
                            "element at index 0 has type {}, but element at index {} has type {}",
                            first_type, i, elem_type
                        )),
                    );

                    has_errors = true;
                }
            }
        }

        // If we had errors, return Unknown to prevent cascading errors
        if has_errors {
            Ok(Type::Unknown)
        } else {
            // Return the unified type, handling Unknown types appropriately
            if first_type == Type::Unknown {
                // Find the first non-Unknown type if any
                for (elem_type, _) in element_types {
                    if *elem_type != Type::Unknown {
                        return Ok(elem_type.clone());
                    }
                }
            }
            Ok(first_type)
        }
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
                if let Some(symbol) = self.symbol_table.lookup_with_modules(name) {
                    let symbol_id = symbol.id;
                    let is_mutable = symbol.is_mutable;
                    let target_type = symbol.ty.clone();

                    if !is_mutable {
                        self.add_error(SemanticError::assignment_to_immutable(name, span));
                    }

                    self.symbol_table.mark_used(symbol_id);

                    // Check type compatibility
                    if !value_type.is_assignable_to(&target_type) {
                        self.add_error(SemanticError::type_mismatch(
                            target_type,
                            value_type.clone(),
                            span,
                        ));
                    }
                } else {
                    self.add_error(SemanticError::undefined_variable(name, target.span));
                }
            }
            ExprKind::Index { object, index } => {
                let target_type = self.analyze_index(object, index, target.span)?;

                // Check type compatibility for array element assignment
                if !value_type.is_assignable_to(&target_type) {
                    self.add_error(SemanticError::type_mismatch(
                        target_type,
                        value_type.clone(),
                        span,
                    ));
                }
            }
            ExprKind::Member { object, property } => {
                let target_type = self.analyze_member(object, property, target.span)?;

                // Check type compatibility for member assignment
                if !value_type.is_assignable_to(&target_type) {
                    self.add_error(SemanticError::type_mismatch(
                        target_type,
                        value_type.clone(),
                        span,
                    ));
                }
            }
            _ => {
                self.add_error(SemanticError::invalid_assignment_target(target.span));
            }
        }

        Ok(value_type)
    }

    /// Analyze a match expression
    fn analyze_match(
        &mut self,
        expr: &Expr,
        arms: &[crate::parser::MatchArm],
        span: crate::source::Span,
    ) -> Result<Type> {
        // Analyze the expression being matched
        let expr_type = self.analyze_expr(expr)?;

        if arms.is_empty() {
            self.add_error(
                SemanticError::new(
                    SemanticErrorKind::InvalidOperation {
                        op: "match".to_string(),
                        ty: Type::Unknown,
                    },
                    expr.span,
                )
                .with_note("Match expression must have at least one arm".to_string()),
            );
            return Ok(Type::Unknown);
        }

        // Check pattern exhaustiveness
        let exhaustiveness_result =
            super::pattern_exhaustiveness::check_exhaustiveness(arms, &expr_type, expr.span);

        if !exhaustiveness_result.is_exhaustive {
            let missing_patterns = exhaustiveness_result.missing_patterns.join(", ");
            let mut error = SemanticError::new(SemanticErrorKind::NonExhaustivePatterns, span)
                .with_note(format!(
                    "Pattern matching is not exhaustive. Missing patterns: {}",
                    missing_patterns
                ))
                .with_help(
                    "Consider adding a wildcard pattern `_` to handle all remaining cases"
                        .to_string(),
                );

            // Add a note if guards are present
            if exhaustiveness_result.has_guards {
                error = error.with_note(
                    "Note: Patterns with guards are not considered exhaustive because guards can fail at runtime".to_string()
                );
            }

            self.add_error(error);
        }

        // Report redundant patterns
        for (index, pattern_span) in exhaustiveness_result.redundant_patterns {
            self.add_error(
                SemanticError::new(SemanticErrorKind::RedundantPattern, pattern_span)
                    .with_note(format!("Pattern {} is unreachable", index + 1))
                    .with_help("Remove this pattern or reorder the match arms".to_string()),
            );
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

    /// Analyze an await expression
    fn analyze_await(&mut self, expr: &Expr, span: crate::source::Span) -> Result<Type> {
        // Check if we're in an async function
        let ctx = self.current_context();
        if !ctx.in_async_function {
            self.add_error(
                SemanticError::new(
                    SemanticErrorKind::InvalidOperation {
                        op: "await".to_string(),
                        ty: Type::Unknown,
                    },
                    span,
                )
                .with_note("'await' can only be used inside async functions".to_string()),
            );
            return Ok(Type::Unknown);
        }

        // Analyze the expression being awaited
        let expr_type = self.analyze_expr(expr)?;

        // Check if the expression is a Future
        if let Type::Future(inner_type) = expr_type {
            // Return the inner type
            Ok((*inner_type).clone())
        } else if expr_type == Type::Unknown {
            // If the type is unknown, we can't verify it's a Future
            Ok(Type::Unknown)
        } else {
            // Error: await can only be used on Future types
            self.add_error(
                SemanticError::new(
                    SemanticErrorKind::TypeMismatch {
                        expected: Type::Future(Box::new(Type::Unknown)),
                        found: expr_type.clone(),
                    },
                    span,
                )
                .with_note("'await' can only be used on Future types".to_string()),
            );
            Ok(Type::Unknown)
        }
    }

    /// Analyze a pattern and add bindings to the symbol table
    fn analyze_pattern(
        &mut self,
        pattern: &crate::parser::Pattern,
        expected_type: &Type,
    ) -> Result<()> {
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
                        self.add_error(
                            SemanticError::duplicate_variable(name, pattern.span).with_note(err),
                        );
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

    /// Get the memory safety context
    pub fn memory_safety_context(&self) -> &MemorySafetyContext {
        &self.memory_safety_ctx
    }

    /// Get memory safety violations
    pub fn memory_safety_violations(&self) -> &[MemorySafetyViolation] {
        self.memory_safety_ctx.violations()
    }

    /// Extract the type information collected during analysis
    /// Returns a HashMap mapping expression IDs to their inferred types
    pub fn extract_type_info(&self) -> HashMap<usize, Type> {
        // For now, we'll return an empty map since we're not tracking expression IDs yet
        // In a complete implementation, we would:
        // 1. Assign unique IDs to each expression node during parsing
        // 2. Store the inferred type for each expression ID during analysis
        // 3. Return the collected type information here
        HashMap::new()
    }

    /// Take ownership of the symbol table
    pub fn into_symbol_table(self) -> SymbolTable {
        self.symbol_table
    }

    /// Check if one type can be assigned to another type
    /// This follows the same logic as in analyze_assign method:
    /// - Unknown type can be assigned to/from any type (gradual typing)
    /// - Otherwise, types must be equal
    pub fn is_assignable_to(&self, source_type: &Type, target_type: &Type) -> bool {
        match (source_type, target_type) {
            // Unknown type can be assigned to/from any type
            (Type::Unknown, _) | (_, Type::Unknown) => true,
            // Otherwise, types must be equal
            _ => source_type == target_type,
        }
    }

    /// Validate constraints for @const functions
    fn validate_const_function_constraints(
        &mut self,
        _name: &str,
        _params: &[Param],
        _ret_type: Option<&TypeAnn>,
        _body: &Block,
        is_async: bool,
        span: crate::source::Span,
    ) -> Result<()> {
        // Const functions cannot be async
        if is_async {
            self.add_error(SemanticError::const_function_violation(
                "@const functions cannot be async",
                span,
            ));
        }

        Ok(())
    }

    /// Analyze the body of a @const function with special validation
    fn analyze_const_function_body(&mut self, body: &Block) -> Result<()> {
        // Save current const context
        let was_in_const = self.current_context()._in_const_function;
        self.current_context_mut()._in_const_function = true;

        // Analyze each statement in the body
        for stmt in &body.statements {
            self.validate_const_statement(stmt)?;
            self.analyze_stmt(stmt)?;
        }

        // Analyze final expression if present
        if let Some(expr) = &body.final_expr {
            self.validate_const_expression(expr)?;
            self.analyze_expr(expr)?;
        }

        // Restore const context
        self.current_context_mut()._in_const_function = was_in_const;
        Ok(())
    }

    /// Validate that a statement is allowed in a @const function
    fn validate_const_statement(&mut self, stmt: &Stmt) -> Result<()> {
        match &stmt.kind {
            StmtKind::Let { init, .. } => {
                // Let statements are allowed, but initializer must be const
                if let Some(expr) = init {
                    self.validate_const_expression(expr)?;
                }
            }
            StmtKind::Return(expr) => {
                // Return statements are allowed, but expression must be const
                if let Some(expr) = expr {
                    self.validate_const_expression(expr)?;
                }
            }
            StmtKind::Expression(expr) => {
                // Expression statements are allowed, but must be const
                self.validate_const_expression(expr)?;
            }
            StmtKind::While { condition, body } => {
                // While loops are allowed with restrictions
                self.validate_const_expression(condition)?;
                self.validate_const_block(body)?;
            }
            StmtKind::For { iterable, body, .. } => {
                // For loops are allowed with restrictions
                self.validate_const_expression(iterable)?;
                self.validate_const_block(body)?;
            }
            StmtKind::Function { .. } => {
                // Nested functions are not allowed in const functions
                self.add_error(SemanticError::const_function_violation(
                    "nested functions not allowed in @const functions",
                    stmt.span,
                ));
            }
            StmtKind::Import { .. } | StmtKind::Export { .. } => {
                // Import/export statements not allowed in function bodies
                self.add_error(SemanticError::const_function_violation(
                    "import/export statements not allowed in function bodies",
                    stmt.span,
                ));
            }
            StmtKind::Struct { .. } | StmtKind::Enum { .. } => {
                // Struct/enum definitions not allowed in function bodies
                self.add_error(SemanticError::const_function_violation(
                    "struct/enum definitions not allowed in function bodies",
                    stmt.span,
                ));
            }
        }
        Ok(())
    }

    /// Validate that a block is allowed in a @const function
    fn validate_const_block(&mut self, block: &Block) -> Result<()> {
        for stmt in &block.statements {
            self.validate_const_statement(stmt)?;
        }
        if let Some(expr) = &block.final_expr {
            self.validate_const_expression(expr)?;
        }
        Ok(())
    }

    /// Validate that an expression is allowed in a @const function
    fn validate_const_expression(&mut self, expr: &Expr) -> Result<()> {
        match &expr.kind {
            // These are always allowed in const functions
            ExprKind::Literal(_) | ExprKind::Identifier(_) => Ok(()),

            // Binary and unary operations are allowed if operands are const
            ExprKind::Binary { left, right, .. } => {
                self.validate_const_expression(left)?;
                self.validate_const_expression(right)?;
                Ok(())
            }
            ExprKind::Unary { expr, .. } => {
                self.validate_const_expression(expr)?;
                Ok(())
            }

            // Array literals are allowed if all elements are const
            ExprKind::Array(elements) => {
                for elem in elements {
                    self.validate_const_expression(elem)?;
                }
                Ok(())
            }

            // Function calls are only allowed if calling other @const functions
            ExprKind::Call { callee, args } => {
                // Validate all arguments are const expressions
                for arg in args {
                    self.validate_const_expression(arg)?;
                }

                // Check if the function being called is @const
                if let ExprKind::Identifier(name) = &callee.kind {
                    if let Some(symbol) = self.symbol_table.lookup_with_modules(name) {
                        if let Some(sig) = symbol.function_signature() {
                            if !sig.is_const {
                                self.add_error(
                                    SemanticError::const_function_violation(
                                        &format!("@const functions can only call other @const functions, but '{}' is not @const", name),
                                        expr.span,
                                    )
                                );
                            }
                        }
                    }
                }
                Ok(())
            }

            // If expressions are allowed if all branches are const
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.validate_const_expression(condition)?;
                self.validate_const_expression(then_branch)?;
                if let Some(else_expr) = else_branch {
                    self.validate_const_expression(else_expr)?;
                }
                Ok(())
            }

            // Block expressions are allowed if the block is const
            ExprKind::Block(block) => {
                self.validate_const_block(block)?;
                Ok(())
            }

            // Index operations are allowed if both object and index are const
            ExprKind::Index { object, index } => {
                self.validate_const_expression(object)?;
                self.validate_const_expression(index)?;
                Ok(())
            }

            // Assignment is generally not allowed in const functions
            ExprKind::Assign { .. } => {
                self.add_error(SemanticError::const_function_violation(
                    "assignment expressions not allowed in @const functions",
                    expr.span,
                ));
                Ok(())
            }

            // Member access, await, match, and list comprehensions need special handling
            ExprKind::Member { object, .. } => {
                self.validate_const_expression(object)?;
                // TODO: Validate that member access is on const-evaluable objects
                Ok(())
            }

            ExprKind::Await { .. } => {
                self.add_error(SemanticError::const_function_violation(
                    "await expressions not allowed in @const functions",
                    expr.span,
                ));
                Ok(())
            }

            ExprKind::Match {
                expr: match_expr,
                arms,
            } => {
                self.validate_const_expression(match_expr)?;
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        self.validate_const_expression(guard)?;
                    }
                    self.validate_const_expression(&arm.body)?;
                }
                Ok(())
            }

            ExprKind::ListComprehension { .. } => {
                // List comprehensions are complex - for now, disallow in const functions
                self.add_error(SemanticError::const_function_violation(
                    "list comprehensions not yet supported in @const functions",
                    expr.span,
                ));
                Ok(())
            }
            ExprKind::GenericConstructor {
                name: _,
                type_args: _,
            } => {
                // Generic constructors should be evaluated at compile time for @const functions
                // For now, allow them as they are essentially type-parameterized constructors
                Ok(())
            }

            ExprKind::StructConstructor { fields, .. } => {
                // Struct constructors are allowed if all field values are const
                for (_, field_expr) in fields {
                    self.validate_const_expression(field_expr)?;
                }
                Ok(())
            }

            ExprKind::EnumConstructor { args, .. } => {
                // Enum constructors are allowed if all arguments are const
                match args {
                    crate::parser::EnumConstructorArgs::Unit => Ok(()),
                    crate::parser::EnumConstructorArgs::Tuple(exprs) => {
                        for arg_expr in exprs {
                            self.validate_const_expression(arg_expr)?;
                        }
                        Ok(())
                    }
                    crate::parser::EnumConstructorArgs::Struct(fields) => {
                        for (_, field_expr) in fields {
                            self.validate_const_expression(field_expr)?;
                        }
                        Ok(())
                    }
                }
            }
        }
    }

    /// Instantiate a generic function with concrete types based on call arguments
    fn instantiate_generic_function(
        &mut self,
        signature: &FunctionSignature,
        arg_types: &[Type],
    ) -> Result<FunctionSignature> {
        let generic_params = signature.generic_params.as_ref().unwrap();

        // Create a type substitution map
        let mut type_substitutions = HashMap::new();

        // For simple case, we'll infer type parameters from arguments
        // This is a simplified version - real implementation would use unification
        for (i, (param_name, param_type)) in signature.params.iter().enumerate() {
            if let Some(arg_type) = arg_types.get(i) {
                // If parameter type is a type parameter, record the substitution
                if let Type::TypeParam(type_param_name) = param_type {
                    type_substitutions.insert(type_param_name.clone(), arg_type.clone());
                }
                // TODO: Handle more complex cases like Vec<T>, Option<T>, etc.
            }
        }

        // Apply substitutions to create instantiated signature
        let instantiated_params = signature
            .params
            .iter()
            .map(|(name, ty)| {
                let instantiated_type = self.substitute_type(ty, &type_substitutions);
                (name.clone(), instantiated_type)
            })
            .collect();

        let instantiated_return = self.substitute_type(&signature.return_type, &type_substitutions);

        Ok(FunctionSignature {
            generic_params: None, // Instantiated functions have no generic params
            params: instantiated_params,
            return_type: instantiated_return,
            is_const: signature.is_const,
            is_async: signature.is_async,
        })
    }

    /// Substitute type parameters in a type
    fn substitute_type(&self, ty: &Type, substitutions: &HashMap<String, Type>) -> Type {
        match ty {
            Type::TypeParam(name) => substitutions
                .get(name)
                .cloned()
                .unwrap_or_else(|| ty.clone()),
            Type::Array(elem) => Type::Array(Box::new(self.substitute_type(elem, substitutions))),
            Type::Function { params, ret } => Type::Function {
                params: params
                    .iter()
                    .map(|t| self.substitute_type(t, substitutions))
                    .collect(),
                ret: Box::new(self.substitute_type(ret, substitutions)),
            },
            Type::Generic { name, args } => Type::Generic {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|t| self.substitute_type(t, substitutions))
                    .collect(),
            },
            // Other types remain unchanged
            _ => ty.clone(),
        }
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
        let analyzer = analyze_program(
            r#"
            let x: i32 = 1;
            {
                let x: f32 = 2.0;
            }
        "#,
        )
        .unwrap();

        // Should complete without errors
        assert!(analyzer.errors().is_empty());
    }

    #[test]
    fn test_duplicate_variable_error() {
        let result = analyze_program(
            r#"
            let x: i32 = 1;
            let x: f32 = 2.0;
        "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_return_type_checking() {
        // Test correct return type
        let analyzer = analyze_program(
            r#"
            fn get_number() -> i32 {
                return 42;
            }
        "#,
        )
        .unwrap();
        assert!(analyzer.errors().is_empty());

        // Test return without value in void function
        let analyzer = analyze_program(
            r#"
            fn do_nothing() {
                return;
            }
        "#,
        )
        .unwrap();
        assert!(analyzer.errors().is_empty());
    }

    #[test]
    fn test_return_type_mismatch() {
        // Test returning wrong type
        let result = analyze_program(
            r#"
            fn get_number() -> i32 {
                return "hello";
            }
        "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_return_value() {
        // Test missing return value
        let result = analyze_program(
            r#"
            fn get_number() -> i32 {
                return;
            }
        "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_return_value_in_void_function() {
        // Test returning value from void function
        let result = analyze_program(
            r#"
            fn do_nothing() {
                return 42;
            }
        "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_return_outside_function() {
        // Test return outside function
        let result = analyze_program("return 42;");
        assert!(result.is_err());
    }

    #[test]
    fn test_async_function_return() {
        // Test async function returns Future type
        let analyzer = analyze_program(
            r#"
            async fn fetch_data() -> i32 {
                return 42;
            }
        "#,
        )
        .unwrap();
        assert!(analyzer.errors().is_empty());
    }

    #[test]
    fn test_is_assignable_to() {
        let analyzer = SemanticAnalyzer::new();

        // Test same types are assignable
        assert!(analyzer.is_assignable_to(&Type::I32, &Type::I32));
        assert!(analyzer.is_assignable_to(&Type::F32, &Type::F32));
        assert!(analyzer.is_assignable_to(&Type::Bool, &Type::Bool));
        assert!(analyzer.is_assignable_to(&Type::String, &Type::String));

        // Test different types are not assignable
        assert!(!analyzer.is_assignable_to(&Type::I32, &Type::F32));
        assert!(!analyzer.is_assignable_to(&Type::F32, &Type::I32));
        assert!(!analyzer.is_assignable_to(&Type::Bool, &Type::String));
        assert!(!analyzer.is_assignable_to(&Type::String, &Type::Bool));

        // Test Unknown type can be assigned to/from any type
        assert!(analyzer.is_assignable_to(&Type::Unknown, &Type::I32));
        assert!(analyzer.is_assignable_to(&Type::I32, &Type::Unknown));
        assert!(analyzer.is_assignable_to(&Type::Unknown, &Type::F32));
        assert!(analyzer.is_assignable_to(&Type::F32, &Type::Unknown));
        assert!(analyzer.is_assignable_to(&Type::Unknown, &Type::Bool));
        assert!(analyzer.is_assignable_to(&Type::Bool, &Type::Unknown));
        assert!(analyzer.is_assignable_to(&Type::Unknown, &Type::String));
        assert!(analyzer.is_assignable_to(&Type::String, &Type::Unknown));
        assert!(analyzer.is_assignable_to(&Type::Unknown, &Type::Unknown));

        // Test complex types
        let array_i32 = Type::Array(Box::new(Type::I32));
        let array_f32 = Type::Array(Box::new(Type::F32));
        assert!(analyzer.is_assignable_to(&array_i32, &array_i32));
        assert!(!analyzer.is_assignable_to(&array_i32, &array_f32));
        assert!(analyzer.is_assignable_to(&array_i32, &Type::Unknown));
        assert!(analyzer.is_assignable_to(&Type::Unknown, &array_i32));
    }
}
