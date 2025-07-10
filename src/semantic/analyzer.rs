use crate::error::ErrorKind;
use crate::inference::{type_ann_to_type, InferenceContext};
use crate::parser::{
<<<<<<< HEAD
    BinaryOp, Block, ExportKind, Expr, ExprKind, GenericParams, ImportSpecifier, Literal, Param,
    Program, Stmt, StmtKind, TraitBound, TypeAnn, UnaryOp,
=======
    BinaryOp, Block, ExportKind, Expr, ExprKind, ImportSpecifier, Literal, Param, Program, Stmt,
    StmtKind, TypeAnn, TypeKind, UnaryOp, ImplBlock, Method, GenericParams,
>>>>>>> 289b5f6 (feat: Complete generic system implementation with full compilation pipeline)
};
use crate::source::Span;
use crate::types::Type;
use crate::Result;
use std::collections::HashMap;
use std::path::PathBuf;

use super::capture_analysis::{CaptureAnalyzer, CaptureInfo};
use super::error::{SemanticError, SemanticErrorKind};
use super::memory_safety::{MemorySafetyContext, MemorySafetyViolation};
use super::module_loader_integration::ModuleLoaderIntegration;
use super::symbol::{
    EnumInfo, EnumVariantInfo, EnumVariantType, FunctionSignature, Symbol, SymbolKind,
};
use super::symbol_table::SymbolTable;

/// Convert a Type to TypeAnn for interface compatibility
fn type_to_type_ann(ty: &Type) -> TypeAnn {
    // Create a dummy span for the conversion
    let dummy_span = Span::new(
        crate::source::SourceLocation::new(0, 0, 0),
        crate::source::SourceLocation::new(0, 0, 0),
    );

    let kind = match ty {
        Type::I32 => TypeKind::Named("i32".to_string()),
        Type::F32 => TypeKind::Named("f32".to_string()),
        Type::Bool => TypeKind::Named("bool".to_string()),
        Type::String => TypeKind::Named("string".to_string()),
        Type::Unknown => TypeKind::Named("unknown".to_string()),
        Type::Never => TypeKind::Named("never".to_string()),
        Type::Named(name) => TypeKind::Named(name.clone()),
        Type::TypeParam(name) => TypeKind::TypeParam(name.clone()),
        Type::Generic { name, args } => TypeKind::Generic {
            name: name.clone(),
            args: args.iter().map(type_to_type_ann).collect(),
        },
        Type::Array(elem) => TypeKind::Array(Box::new(type_to_type_ann(elem))),
        Type::Tuple(types) => TypeKind::Tuple(types.iter().map(type_to_type_ann).collect()),
        Type::Reference { mutable, inner } => TypeKind::Reference {
            mutable: *mutable,
            inner: Box::new(type_to_type_ann(inner)),
        },
        Type::Function { params, ret } => TypeKind::Function {
            params: params.iter().map(type_to_type_ann).collect(),
            ret: Box::new(type_to_type_ann(ret)),
        },
        Type::Option(inner) => TypeKind::Generic {
            name: "Option".to_string(),
            args: vec![type_to_type_ann(inner)],
        },
        Type::Result { ok, err } => TypeKind::Generic {
            name: "Result".to_string(),
            args: vec![type_to_type_ann(ok), type_to_type_ann(err)],
        },
        Type::Future(inner) => TypeKind::Generic {
            name: "Future".to_string(),
            args: vec![type_to_type_ann(inner)],
        },
        Type::TypeVar(_) => TypeKind::Named("unknown".to_string()), // Type variables become unknown
        Type::Struct { name, .. } => TypeKind::Named(name.clone()),
    };

    TypeAnn {
        kind,
        span: dummy_span,
    }
}

/// Represents a generic function instantiation for monomorphization
#[derive(Debug, Clone)]
pub struct GenericInstantiation {
    /// Name of the generic function
    pub function_name: String,
    /// Concrete type arguments
    pub type_args: Vec<Type>,
    /// Location where instantiation occurred
    pub span: Span,
}

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
    /// Generic parameter names in current scope
    generic_param_names: Vec<String>,
}

impl AnalysisContext {
    fn new() -> Self {
        AnalysisContext {
            current_function_return: None,
            in_loop: false,
            _in_const_function: false,
            in_async_function: false,
            generic_params: None,
            generic_param_names: Vec::new(),
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
    /// Impl blocks for method resolution
    impl_blocks: Vec<ImplBlock>,
    /// Method resolution cache
    method_cache: HashMap<(String, String), Vec<Method>>, // (type_name, method_name) -> methods
    /// Generic instantiations for monomorphization
    generic_instantiations: Vec<GenericInstantiation>,
    /// Type information for expressions (maps expression ID to type)
    type_info: HashMap<usize, Type>,
    /// Module loader integration for handling imports
    module_loader: ModuleLoaderIntegration,
    /// Capture information for closures (maps closure expression ID to captures)
    closure_captures: HashMap<usize, Vec<CaptureInfo>>,
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
            impl_blocks: Vec::new(),
            method_cache: HashMap::new(),
            generic_instantiations: Vec::new(),
            type_info: HashMap::new(),
            module_loader: ModuleLoaderIntegration::new(),
            closure_captures: HashMap::new(),
        }
    }

    /// Create a new semantic analyzer with a shared symbol table
    pub fn with_symbol_table(symbol_table: SymbolTable) -> Self {
        SemanticAnalyzer {
            symbol_table,
            inference_ctx: InferenceContext::new(),
            memory_safety_ctx: MemorySafetyContext::new(),
            context_stack: vec![AnalysisContext::new()],
            errors: Vec::new(),
            memory_safety_enabled: true,
            impl_blocks: Vec::new(),
            method_cache: HashMap::new(),
            generic_instantiations: Vec::new(),
            type_info: HashMap::new(),
            module_loader: ModuleLoaderIntegration::new(),
            closure_captures: HashMap::new(),
        }
    }

    /// Create a new semantic analyzer with memory safety analysis disabled
    pub fn new_without_memory_safety() -> Self {
        let mut analyzer = Self::new();
        analyzer.memory_safety_enabled = false;
        analyzer
    }

    /// Set the current file for resolving relative imports
    pub fn set_current_file(&mut self, file: Option<PathBuf>) {
        self.module_loader.set_current_file(file);
    }

    /// Add a search path for module resolution
    pub fn add_module_search_path(&mut self, path: PathBuf) {
        self.module_loader.add_search_path(path);
    }

    /// Enable or disable memory safety analysis
    pub fn set_memory_safety_enabled(&mut self, enabled: bool) {
        self.memory_safety_enabled = enabled;
    }

    /// Get a reference to the symbol table
    pub fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }

    /// Get a mutable reference to the symbol table
    pub fn symbol_table_mut(&mut self) -> &mut SymbolTable {
        &mut self.symbol_table
    }

    /// Register a module's symbols for import resolution
    pub fn register_module(&mut self, module_name: &str, module_symbols: &SymbolTable) {
        // Register the module in our symbol table for import resolution
        self.symbol_table
            .register_module(module_name, module_symbols);
    }

    /// Import symbols from another symbol table (for module imports)
    /// Enhanced version with proper module boundaries and selective imports
    pub fn import_symbols_from(&mut self, source_table: &SymbolTable) -> Result<()> {
        self.import_symbols_from_with_filter(source_table, None, None)
    }

    /// Import symbols from another symbol table with selective filtering
    ///
    /// # Arguments
    /// * `source_table` - The source symbol table to import from
    /// * `symbol_filter` - Optional filter function to select specific symbols
    /// * `module_name` - Optional module name for namespace management
    pub fn import_symbols_from_with_filter(
        &mut self,
        source_table: &SymbolTable,
        symbol_filter: Option<&dyn Fn(&Symbol) -> bool>,
        module_name: Option<&str>,
    ) -> Result<()> {
        // Register the module if a name is provided
        if let Some(name) = module_name {
            self.symbol_table.register_module(name, source_table);
        }

        // Get all symbols from the source table's global scope
        let global_scope_symbols = source_table.get_current_scope_symbols();
        let mut imported_count = 0;
        let mut conflict_count = 0;

        for symbol in global_scope_symbols {
            // Apply filter if provided
            if let Some(filter) = symbol_filter {
                if !filter(symbol) {
                    continue;
                }
            }

            // Check for symbol name conflicts before importing
            if let Some(existing_symbol) = self.symbol_table.lookup(&symbol.name) {
                // Handle conflicts based on symbol types
                if self.should_skip_conflicting_symbol(&symbol, &existing_symbol) {
                    conflict_count += 1;
                    continue;
                }
            }

            // Import the symbol based on its type
            match &symbol.kind {
                super::symbol::SymbolKind::Function(signature) => {
                    match self.symbol_table.define_function(
                        symbol.name.clone(),
                        signature.clone(),
                        symbol.def_span,
                    ) {
                        Ok(_) => imported_count += 1,
                        Err(err) => {
                            // Log conflicts but continue importing other symbols
                            eprintln!(
                                "Warning: Failed to import function '{}': {}",
                                symbol.name, err
                            );
                        }
                    }
                }
                super::symbol::SymbolKind::Variable => {
                    match self.symbol_table.define_variable(
                        symbol.name.clone(),
                        symbol.ty.clone(),
                        symbol.def_span,
                        symbol.is_mutable,
                    ) {
                        Ok(_) => imported_count += 1,
                        Err(err) => {
                            eprintln!(
                                "Warning: Failed to import variable '{}': {}",
                                symbol.name, err
                            );
                        }
                    }
                }
                super::symbol::SymbolKind::Struct(struct_info) => {
                    match self.symbol_table.define_struct(
                        symbol.name.clone(),
                        struct_info.clone(),
                        symbol.def_span,
                    ) {
                        Ok(_) => imported_count += 1,
                        Err(err) => {
                            eprintln!(
                                "Warning: Failed to import struct '{}': {}",
                                symbol.name, err
                            );
                        }
                    }
                }
                super::symbol::SymbolKind::Enum(enum_info) => {
                    match self.symbol_table.define_enum(
                        symbol.name.clone(),
                        enum_info.clone(),
                        symbol.def_span,
                    ) {
                        Ok(_) => imported_count += 1,
                        Err(err) => {
                            eprintln!("Warning: Failed to import enum '{}': {symbol.name, err}");
                        }
                    }
                }
                super::symbol::SymbolKind::Parameter | super::symbol::SymbolKind::BuiltIn => {
                    // Skip parameters and built-ins as they shouldn't be imported
                    continue;
                }
                _ => {
                    // For other symbol types, try to create a generic symbol
                    // This is a fallback for symbol types we haven't explicitly handled
                    eprintln!(
                        "Warning: Skipping import of unsupported symbol type for '{}'",
                        symbol.name
                    );
                }
            }
        }

        if imported_count == 0 && conflict_count == 0 {
            return Err(crate::Error::new(
                ErrorKind::SemanticError,
                "No symbols were imported from source table",
            ));
        }

        Ok(())
    }

    /// Determine if a conflicting symbol should be skipped during import
    fn should_skip_conflicting_symbol(
        &self,
        new_symbol: &Symbol,
        existing_symbol: &Symbol,
    ) -> bool {
        match (&new_symbol.kind, &existing_symbol.kind) {
            // Allow function overloading if signatures are different
            (SymbolKind::Function(new_sig), SymbolKind::Function(existing_sig)) => {
                !new_sig.is_compatible_for_overload(&existing_sig)
            }
            // Always skip if same symbol type and name (no overloading)
            (SymbolKind::Variable, SymbolKind::Variable)
            | (SymbolKind::Struct(_), SymbolKind::Struct(_))
            | (SymbolKind::Enum(_), SymbolKind::Enum(_)) => true,
            // Skip built-ins to avoid overriding them
            (_, SymbolKind::BuiltIn) => true,
            // Allow different symbol types with same name (unusual but possible)
            _ => false,
        }
    }

    /// Import specific symbols by name from another symbol table
    pub fn import_symbols_by_name(
        &mut self,
        source_table: &SymbolTable,
        symbol_names: &[String],
        module_name: Option<&str>,
    ) -> Result<()> {
        let filter = |symbol: &Symbol| symbol_names.contains(&symbol.name);
        self.import_symbols_from_with_filter(source_table, Some(&filter), module_name)
    }

    /// Import all public symbols from another symbol table
    /// Public symbols are those that would be exported from a module
    pub fn import_public_symbols(
        &mut self,
        source_table: &SymbolTable,
        module_name: Option<&str>,
    ) -> Result<()> {
        let filter = |symbol: &Symbol| {
            // Consider symbols "public" if they're not parameters or built-ins
            // In a real implementation, this would check actual visibility modifiers
            !matches!(symbol.kind, SymbolKind::Parameter | SymbolKind::BuiltIn)
        };
        self.import_symbols_from_with_filter(source_table, Some(&filter), module_name)
    }

    /// Register a generic function for potential instantiation
    pub fn register_generic_function(
        &mut self,
        name: &str,
        _generic_params: crate::parser::GenericParams,
    ) {
        // Store the generic function information for later instantiation
        // This is used during cross-module type propagation
        // In a complete implementation, this would be stored in a dedicated registry

        // For now, we'll create a placeholder generic instantiation entry
        // This helps the type system understand that this generic function is available
        let dummy_span = crate::source::Span::dummy();
        let generic_inst = GenericInstantiation {
            function_name: name.to_string(),
            type_args: vec![], // Will be filled when actually instantiated
            span: dummy_span,
        };

        self.generic_instantiations.push(generic_inst);
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

    /// Check if a type is the boolean type
    fn is_bool_type(&self, ty: &Type) -> bool {
        matches!(ty, Type::Bool)
    }

    /// Check if two types are equal
    fn types_equal(&self, a: &Type, b: &Type) -> bool {
        a == b
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
        // Create the full standard library
        let stdlib = crate::stdlib::StdLib::new();

        // Register all stdlib functions with the semantic analyzer
        for function_name in stdlib.function_names() {
            if let Some(stdlib_function) = stdlib.get_function(function_name) {
                let signature = self.convert_stdlib_signature(&stdlib_function.signature);
                self.symbol_table
                    .define_function(
                        function_name.to_string(),
                        signature,
                        crate::source::Span::single(crate::source::SourceLocation::initial()),
                    )
                    .map_err(|e| {
                        SemanticError::new(
                            SemanticErrorKind::DuplicateFunction(e),
                            crate::source::Span::single(crate::source::SourceLocation::initial()),
                        )
                        .into_error()
                    })?;
            }
        }

        // Also add basic print function for backward compatibility
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
            .map_err(|_| {
                // Ignore if already exists
            });

        // Add println function
        let println_sig = FunctionSignature {
            generic_params: None,
            params: vec![("value".to_string(), Type::Unknown)],
            return_type: Type::Unknown, // void
            is_const: false,
            is_async: false,
        };
        self.symbol_table
            .define_function(
                "println".to_string(),
                println_sig,
                crate::source::Span::single(crate::source::SourceLocation::initial()),
            )
            .map_err(|_| {
                // Ignore if already exists
            });

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

        // Add built-in Option<T> enum
        let option_variants = vec![
            EnumVariantInfo {
                name: "None".to_string(),
                variant_type: EnumVariantType::Unit,
            },
            EnumVariantInfo {
                name: "Some".to_string(),
                variant_type: EnumVariantType::Tuple(vec![Type::TypeParam("T".to_string())]),
            },
        ];

        let option_generic_params = crate::parser::GenericParams {
            params: vec![crate::parser::GenericParam {
                name: "T".to_string(),
                bounds: vec![],
                span: crate::source::Span::single(crate::source::SourceLocation::initial()),
            }],
            span: crate::source::Span::single(crate::source::SourceLocation::initial()),
        };

        let option_info = EnumInfo {
            generic_params: Some(option_generic_params),
            variants: option_variants,
            where_clause: None,
        };

        self.symbol_table
            .define_enum(
                "Option".to_string(),
                option_info,
                crate::source::Span::single(crate::source::SourceLocation::initial()),
            )
            .map_err(|e| {
                SemanticError::new(
                    SemanticErrorKind::DuplicateType(e),
                    crate::source::Span::single(crate::source::SourceLocation::initial()),
                )
                .into_error()
            })?;

        // Add built-in Result<T, E> enum
        let result_variants = vec![
            EnumVariantInfo {
                name: "Ok".to_string(),
                variant_type: EnumVariantType::Tuple(vec![Type::TypeParam("T".to_string())]),
            },
            EnumVariantInfo {
                name: "Err".to_string(),
                variant_type: EnumVariantType::Tuple(vec![Type::TypeParam("E".to_string())]),
            },
        ];

        let result_generic_params = crate::parser::GenericParams {
            params: vec![
                crate::parser::GenericParam {
                    name: "T".to_string(),
                    bounds: vec![],
                    span: crate::source::Span::single(crate::source::SourceLocation::initial()),
                },
                crate::parser::GenericParam {
                    name: "E".to_string(),
                    bounds: vec![],
                    span: crate::source::Span::single(crate::source::SourceLocation::initial()),
                },
            ],
            span: crate::source::Span::single(crate::source::SourceLocation::initial()),
        };

        let result_info = EnumInfo {
            generic_params: Some(result_generic_params),
            variants: result_variants,
            where_clause: None,
        };

        self.symbol_table
            .define_enum(
                "Result".to_string(),
                result_info,
                crate::source::Span::single(crate::source::SourceLocation::initial()),
            )
            .map_err(|e| {
                SemanticError::new(
                    SemanticErrorKind::DuplicateType(e),
                    crate::source::Span::single(crate::source::SourceLocation::initial()),
                )
                .into_error()
            })?;

        Ok(())
    }

    /// Convert a stdlib type signature to a semantic analyzer function signature
    fn convert_stdlib_signature(&self, stdlib_type: &Type) -> FunctionSignature {
        // For now, create a simple signature for all stdlib functions
        // In a complete implementation, this would parse the actual Type to extract
        // function parameters and return type
        FunctionSignature {
            generic_params: None,
            params: vec![("args".to_string(), Type::Unknown)],
            return_type: Type::Unknown,
            is_const: false,
            is_async: false,
        }
    }

    /// Analyze a struct definition
    fn analyze_struct_definition(
        &mut self,
        name: &str,
        generic_params: Option<&GenericParams>,
        fields: &[crate::parser::StructField],
        where_clause: Option<&crate::parser::WhereClause>,
        span: Span,
    ) -> Result<()> {
        // Create struct type
        let struct_type = if let Some(generics) = generic_params {
            Type::Generic {
                name: name.to_string(),
                args: generics
                    .params
                    .iter()
                    .map(|p| Type::TypeParam(p.name.clone()))
                    .collect(),
            }
        } else {
            Type::Named(name.to_string())
        };

        // Enter struct scope for generic parameters
        if let Some(generics) = generic_params {
            self.inference_ctx.push_scope();

            // Define generic type parameters
            for param in &generics.params {
                self.inference_ctx.define_type_param(&param.name);

                // Add trait bounds
                if !param.bounds.is_empty() {
                    let bound_names: Vec<String> =
                        param.bounds.iter().map(|b| b.trait_name.clone()).collect();
                    self.inference_ctx.add_generic_bounds(
                        param.name.clone(),
                        bound_names,
                        param.span,
                    );
                }
            }

            // Validate where clause constraints
            if let Some(where_clause) = where_clause {
                for predicate in &where_clause.predicates {
                    if let crate::parser::TypeKind::TypeParam(type_param) = &predicate.type_.kind {
                        // Validate that the type parameter exists
                        let param_exists = generics.params.iter().any(|p| p.name == *type_param);

                        if !param_exists {
                            self.add_error(SemanticError::new(
                                SemanticErrorKind::UndefinedTypeParameter(type_param.clone()),
                                predicate.span,
                            ));
                            continue;
                        }

                        // Add bounds to inference context
                        let bound_names: Vec<String> = predicate
                            .bounds
                            .iter()
                            .map(|b| b.trait_name.clone())
                            .collect();
                        self.inference_ctx.add_generic_bounds(
                            type_param.clone(),
                            bound_names,
                            predicate.span,
                        );
                    }
                }
            }
        }

        // Analyze field types
        for field in fields {
            let field_type = type_ann_to_type(&field.type_ann);

            // Validate field types (check that they're well-formed)
            if let Err(err) = self.validate_type(&field_type, field.type_ann.span) {
                self.add_error(err);
            }
        }

        // Create struct info for symbol table
        let field_info: Vec<(String, Type)> = fields
            .iter()
            .map(|f| (f.name.clone(), type_ann_to_type(&f.type_ann)))
            .collect();

        let struct_info = crate::semantic::symbol::StructInfo {
            generic_params: generic_params.cloned(),
            fields: field_info,
            where_clause: where_clause.cloned(),
        };

        // Register the struct type in the symbol table using a custom method
        match self
            .symbol_table
            .define_struct(name.to_string(), struct_info, span)
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

        // Exit struct scope
        if generic_params.is_some() {
            self.inference_ctx.pop_scope();
        }

        Ok(())
    }

    /// Analyze an enum definition
    fn analyze_enum_definition(
        &mut self,
        name: &str,
        generic_params: Option<&GenericParams>,
        variants: &[crate::parser::EnumVariant],
        where_clause: Option<&crate::parser::WhereClause>,
        span: Span,
    ) -> Result<()> {
        // Create enum type
        let enum_type = if let Some(generics) = generic_params {
            Type::Generic {
                name: name.to_string(),
                args: generics
                    .params
                    .iter()
                    .map(|p| Type::TypeParam(p.name.clone()))
                    .collect(),
            }
        } else {
            Type::Named(name.to_string())
        };

        // Enter enum scope for generic parameters
        if let Some(generics) = generic_params {
            self.inference_ctx.push_scope();

            // Define generic type parameters
            for param in &generics.params {
                self.inference_ctx.define_type_param(&param.name);

                // Add trait bounds
                if !param.bounds.is_empty() {
                    let bound_names: Vec<String> =
                        param.bounds.iter().map(|b| b.trait_name.clone()).collect();
                    self.inference_ctx.add_generic_bounds(
                        param.name.clone(),
                        bound_names,
                        param.span,
                    );
                }
            }

            // Validate where clause constraints
            if let Some(where_clause) = where_clause {
                for predicate in &where_clause.predicates {
                    if let crate::parser::TypeKind::TypeParam(type_param) = &predicate.type_.kind {
                        // Validate that the type parameter exists
                        let param_exists = generics.params.iter().any(|p| p.name == *type_param);

                        if !param_exists {
                            self.add_error(SemanticError::new(
                                SemanticErrorKind::UndefinedTypeParameter(type_param.clone()),
                                predicate.span,
                            ));
                            continue;
                        }

                        // Add bounds to inference context
                        let bound_names: Vec<String> = predicate
                            .bounds
                            .iter()
                            .map(|b| b.trait_name.clone())
                            .collect();
                        self.inference_ctx.add_generic_bounds(
                            type_param.clone(),
                            bound_names,
                            predicate.span,
                        );
                    }
                }
            }
        }

        // Analyze variant types
        for variant in variants {
            match &variant.fields {
                crate::parser::EnumVariantFields::Unit => {
                    // Unit variants have no additional data
                }
                crate::parser::EnumVariantFields::Tuple(field_types) => {
                    // Tuple variants have typed fields
                    for field_type in field_types {
                        let type_ = type_ann_to_type(field_type);
                        if let Err(err) = self.validate_type(&type_, field_type.span) {
                            self.add_error(err);
                        }
                    }
                }
                crate::parser::EnumVariantFields::Struct(fields) => {
                    // Struct variants have named fields
                    for field in fields {
                        let field_type = type_ann_to_type(&field.type_ann);
                        if let Err(err) = self.validate_type(&field_type, field.type_ann.span) {
                            self.add_error(err);
                        }
                    }
                }
            }
        }

        // Create enum variant info for symbol table
        let variant_info: Vec<crate::semantic::symbol::EnumVariantInfo> = variants
            .iter()
            .map(|v| {
                let variant_type = match &v.fields {
                    crate::parser::EnumVariantFields::Unit => {
                        crate::semantic::symbol::EnumVariantType::Unit
                    }
                    crate::parser::EnumVariantFields::Tuple(types) => {
                        crate::semantic::symbol::EnumVariantType::Tuple(
                            types.iter().map(type_ann_to_type).collect(),
                        )
                    }
                    crate::parser::EnumVariantFields::Struct(fields) => {
                        crate::semantic::symbol::EnumVariantType::Struct(
                            fields
                                .iter()
                                .map(|f| (f.name.clone(), type_ann_to_type(&f.type_ann)))
                                .collect(),
                        )
                    }
                };
                crate::semantic::symbol::EnumVariantInfo {
                    name: v.name.clone(),
                    variant_type,
                }
            })
            .collect();

        let enum_info = crate::semantic::symbol::EnumInfo {
            generic_params: generic_params.cloned(),
            variants: variant_info,
            where_clause: where_clause.cloned(),
        };

        // Register the enum type in the symbol table using a custom method
        match self
            .symbol_table
            .define_enum(name.to_string(), enum_info, span)
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

        // Exit enum scope
        if generic_params.is_some() {
            self.inference_ctx.pop_scope();
        }

        Ok(())
    }

    /// Validate that a type is well-formed
    fn validate_type(
        &mut self,
        type_: &Type,
        span: Span,
    ) -> std::result::Result<(), SemanticError> {
        match type_ {
            Type::TypeParam(name) => {
                // Check if the type parameter is in scope
                if let Some(current_context) = self.context_stack.last() {
                    if !current_context.generic_param_names.contains(name) {
                        return Err(SemanticError::new(
                            SemanticErrorKind::UndefinedTypeParameter(name.clone()),
                            span,
                        ));
                    }
                }
                Ok(())
            }
            Type::Array(elem_type) => self.validate_type(elem_type, span),
            Type::Generic { name: _, args } => {
                for arg in args {
                    self.validate_type(arg, span)?;
                }
                Ok(())
            }
            Type::Function { params, ret } => {
                for param in params {
                    self.validate_type(param, span)?;
                }
                self.validate_type(ret, span)
            }
            Type::Option(inner) => self.validate_type(inner, span),
            Type::Result { ok, err } => {
                self.validate_type(ok, span)?;
                self.validate_type(err, span)
            }
            Type::Future(inner) => self.validate_type(inner, span),
            // All other types are valid
            _ => Ok(()),
        }
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
                where_clause: _, // TODO: Handle where clause constraints
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
                where_clause,
            } => {
                // Define the struct type and analyze its fields
                self.analyze_struct_definition(
                    name,
                    generic_params.as_ref(),
                    fields,
                    where_clause.as_ref(),
                    stmt.span,
                )?;
            }
            StmtKind::Enum {
                name,
                generic_params,
                variants,
                where_clause,
            } => {
                // Define the enum type and analyze its variants
                self.analyze_enum_definition(
                    name,
                    generic_params.as_ref(),
                    variants,
                    where_clause.as_ref(),
                    stmt.span,
                )?;
            }
            StmtKind::Impl(impl_block) => {
                self.analyze_impl_block(impl_block)?;
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
                    if let Err(_err) = self.memory_safety_ctx.define_variable(
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
        self.analyze_function_with_where_clause(
            name,
            generic_params,
            params,
            ret_type,
            body,
            is_async,
            None, // No where clause for basic function
            span,
        )
    }

    fn analyze_function_with_where_clause(
        &mut self,
        name: &str,
        generic_params: Option<&crate::parser::GenericParams>,
        params: &[Param],
        ret_type: Option<&TypeAnn>,
        body: &Block,
        is_async: bool,
        where_clause: Option<&crate::parser::WhereClause>,
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

        // Enter memory safety scope for the function
        if self.memory_safety_enabled {
            self.memory_safety_ctx.enter_scope();
        }

        // Push function context with generic parameters
        // For async functions, we need to check returns against the unwrapped type
        let mut func_context = AnalysisContext {
            current_function_return: Some(base_return_type),
            in_loop: false,
            _in_const_function: false, // TODO: Support @const
            in_async_function: is_async,
            generic_params: generic_params.cloned(),
            generic_param_names: generic_params
                .map(|gp| gp.params.iter().map(|p| p.name.clone()).collect())
                .unwrap_or_default(),
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
                // Define type parameter and track bounds
                self.inference_ctx.define_type_param(&generic_param.name);

                // Add trait bounds to inference context
                if !generic_param.bounds.is_empty() {
                    let bound_names: Vec<String> = generic_param
                        .bounds
                        .iter()
                        .map(|b| b.trait_name.clone())
                        .collect();
                    self.inference_ctx.add_generic_bounds(
                        generic_param.name.clone(),
                        bound_names,
                        generic_param.span,
                    );
                }
            }
        }

        // Validate where clause constraints
        if let Some(where_clause) = where_clause {
            for predicate in &where_clause.predicates {
                // For now, we'll only handle type parameter constraints
                if let crate::parser::TypeKind::TypeParam(type_param) = &predicate.type_.kind {
                    // Validate that the type parameter exists
                    if let Some(generics) = generic_params {
                        let param_exists = generics.params.iter().any(|p| p.name == *type_param);

                        if !param_exists {
                            self.add_error(SemanticError::new(
                                SemanticErrorKind::UndefinedTypeParameter(type_param.clone()),
                                predicate.span,
                            ));
                            continue;
                        }
                    }

                    // Add bounds to inference context
                    let bound_names: Vec<String> = predicate
                        .bounds
                        .iter()
                        .map(|b| b.trait_name.clone())
                        .collect();
                    self.inference_ctx.add_generic_bounds(
                        type_param.clone(),
                        bound_names,
                        predicate.span,
                    );
                } else {
                    // For now, skip non-type-parameter predicates
                    // TODO: Handle more complex where clause predicates
                    continue;
                }
            }
        }

        // Define parameters
        for param in params {
            let param_type = type_ann_to_type(&param.type_ann);
            match self.symbol_table.define_parameter(
                param.name.clone(),
                param_type.clone(),
                param.type_ann.span,
            ) {
                Ok(symbol_id) => {
                    // Mark parameters as used
                    self.symbol_table.mark_used(symbol_id);

                    // Memory safety: Define and initialize function parameters
                    if self.memory_safety_enabled {
                        // Define the parameter in memory safety context
                        if let Err(_err) = self.memory_safety_ctx.define_variable(
                            param.name.clone(),
                            param_type,
                            false, // Parameters are immutable by default
                            param.type_ann.span,
                        ) {
                            // This shouldn't happen as symbol table already checked for duplicates
                        }

                        // Function parameters are initialized by the caller
                        if let Err(_err) = self
                            .memory_safety_ctx
                            .initialize_variable(&param.name, param.type_ann.span)
                        {
                            // This shouldn't fail as we just defined the variable
                        }
                    }
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

        // Exit memory safety scope
        if self.memory_safety_enabled {
            self.memory_safety_ctx.exit_scope(span);
        }

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

        // Enter memory safety scope for the function
        if self.memory_safety_enabled {
            self.memory_safety_ctx.enter_scope();
        }

        // Push function context with const flag and generic params
        let func_context = AnalysisContext {
            current_function_return: Some(base_return_type),
            in_loop: false,
            _in_const_function: is_const,
            in_async_function: is_async,
            generic_params: generic_params.cloned(),
            generic_param_names: generic_params
                .map(|gp| gp.params.iter().map(|p| p.name.clone()).collect())
                .unwrap_or_default(),
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
                // Define type parameter and track bounds
                self.inference_ctx.define_type_param(&generic_param.name);

                // Add trait bounds to inference context
                if !generic_param.bounds.is_empty() {
                    let bound_names: Vec<String> = generic_param
                        .bounds
                        .iter()
                        .map(|b| b.trait_name.clone())
                        .collect();
                    self.inference_ctx.add_generic_bounds(
                        generic_param.name.clone(),
                        bound_names,
                        generic_param.span,
                    );
                }
            }
        }

        // Define parameters
        for param in params {
            let param_type = type_ann_to_type(&param.type_ann);
            match self.symbol_table.define_parameter(
                param.name.clone(),
                param_type.clone(),
                param.type_ann.span,
            ) {
                Ok(symbol_id) => {
                    // Mark parameters as used
                    self.symbol_table.mark_used(symbol_id);

                    // Memory safety: Define and initialize function parameters
                    if self.memory_safety_enabled {
                        // Define the parameter in memory safety context
                        if let Err(_err) = self.memory_safety_ctx.define_variable(
                            param.name.clone(),
                            param_type,
                            false, // Parameters are immutable by default
                            param.type_ann.span,
                        ) {
                            // This shouldn't happen as symbol table already checked for duplicates
                        }

                        // Function parameters are initialized by the caller
                        if let Err(_err) = self
                            .memory_safety_ctx
                            .initialize_variable(&param.name, param.type_ann.span)
                        {
                            // This shouldn't fail as we just defined the variable
                        }
                    }
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

        // Exit memory safety scope
        if self.memory_safety_enabled {
            self.memory_safety_ctx.exit_scope(span);
        }

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
        // First, try to load the module if it hasn't been loaded yet
        let module_symbol_table = match self.module_loader.load_module(source, span) {
            Ok(symbol_table) => symbol_table,
            Err(e) => {
                // Convert the error to a semantic error
                let semantic_error = if e.to_string().contains("not found")
                    || e.to_string().contains("Failed to resolve")
                {
                    SemanticError::module_not_found(source, span)
                } else {
                    SemanticError::module_error(&e.to_string(), span)
                };
                self.add_error(semantic_error);
                return Ok(());
            }
        };

        // Register the loaded module with the symbol table
        self.symbol_table
            .register_module(source, &module_symbol_table);

        // Now process the import specifiers
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
        let expr_type = match &expr.kind {
            ExprKind::Literal(lit) => self.analyze_literal(lit),
            ExprKind::Identifier(name) => self.analyze_identifier(name, expr.span),
            ExprKind::Binary { left, op, right } => self.analyze_binary(left, op, right, expr.span),
            ExprKind::Unary { op, expr: inner } => self.analyze_unary(op, inner, expr.span),
            ExprKind::Call { callee, args } => self.analyze_call(callee, args, expr.span),
            ExprKind::Index { object, index } => self.analyze_index(object, index, expr.span),
            ExprKind::Member { object, property } => {
                self.analyze_member_enhanced(object, property, expr.span)
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
                // Analyze generic type constructor
                let concrete_type_args: Vec<Type> =
                    type_args.iter().map(type_ann_to_type).collect();

                // Look up the generic type definition
                if let Some(symbol) = self.symbol_table.lookup(name) {
                    // For now, create a generic type instance
                    Ok(Type::Generic {
                        name: name.clone(),
                        args: concrete_type_args,
                    })
                } else {
                    self.add_error(SemanticError::undefined_variable(name, expr.span));
                    Ok(Type::Unknown)
                }
            }
            ExprKind::StructConstructor { name, fields } => {
                self.analyze_struct_constructor(name, fields, expr.span)
            }
            ExprKind::EnumConstructor {
                enum_name,
                variant,
                args,
            } => self.analyze_enum_constructor(enum_name.as_deref(), variant, args, expr.span),
            ExprKind::ErrorPropagation { expr: inner } => {
                self.analyze_error_propagation(inner, expr.span)
            }
            ExprKind::TryCatch {
                try_expr,
                catch_clauses,
                finally_block,
            } => self.analyze_try_catch(try_expr, catch_clauses, finally_block, expr.span),
            ExprKind::Closure { parameters, body } => {
                self.analyze_closure(parameters, body, expr.id, expr.span)
            }
        };

        // Record the type information for this expression
        if let Ok(ref type_) = expr_type {
            self.record_expression_type(expr.id, type_.clone());
        }

        expr_type
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
                        // Create instantiation and track it for monomorphization
                        let instantiated =
                            self.instantiate_generic_function(&signature, &arg_types)?;

                        // Track this generic instantiation
                        let instantiation = GenericInstantiation {
                            function_name: name.clone(),
                            type_args: arg_types.clone(),
                            span,
                        };
                        self.add_generic_instantiation(instantiation);

                        instantiated
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
        let exhaustiveness_result = super::pattern_exhaustiveness::check_exhaustiveness(
            arms,
            &expr_type,
            expr.span,
            &self.symbol_table,
        );

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

    /// Analyze an error propagation expression (?)
    fn analyze_error_propagation(&mut self, expr: &Expr, span: Span) -> Result<Type> {
        // Analyze the inner expression
        let expr_type = self.analyze_expr(expr)?;

        // Get the current function's return type
        let function_return_type = self
            .current_context()
            .current_function_return
            .clone()
            .unwrap_or(Type::Unknown);

        // Check if the expression is a Result or Option type
        match &expr_type {
            Type::Result { ok, err } => {
                // Check if the function returns a Result type
                match &function_return_type {
                    Type::Result { err: fn_err, .. } => {
                        // Check that error types are compatible
                        if !self.is_assignable_to(err, fn_err) {
                            self.add_error(
                                SemanticError::new(
                                    SemanticErrorKind::TypeMismatch {
                                        expected: fn_err.as_ref().clone(),
                                        found: err.as_ref().clone(),
                                    },
                                    span,
                                )
                                .with_note(format!(
                                    "? operator requires function to return Result with compatible error type"
                                ))
                            );
                        }
                    }
                    _ => {
                        self.add_error(
                            SemanticError::new(
                                SemanticErrorKind::ErrorPropagationInNonResult,
                                span,
                            )
                            .with_note(format!(
                                "? operator can only be used in functions that return Result, \
                                 but this function returns {}",
                                function_return_type
                            )),
                        );
                    }
                }
                // Return the Ok type
                Ok(ok.as_ref().clone())
            }
            Type::Option(inner) => {
                // Check if the function returns an Option or Result type
                match &function_return_type {
                    Type::Option(_) => {
                        // Option -> Option is OK
                    }
                    Type::Result { .. } => {
                        // Option -> Result is OK (None converts to error)
                    }
                    _ => {
                        self.add_error(
                            SemanticError::new(
                                SemanticErrorKind::ErrorPropagationInNonResult,
                                span,
                            )
                            .with_note(format!(
                                "? operator on Option can only be used in functions that return Option or Result, \
                                 but this function returns {}",
                                function_return_type
                            ))
                        );
                    }
                }
                // Return the inner type
                Ok(inner.as_ref().clone())
            }
            _ => {
                self.add_error(
                    SemanticError::new(
                        SemanticErrorKind::InvalidErrorPropagation {
                            actual_type: expr_type.clone(),
                        },
                        span,
                    )
                    .with_note("? operator can only be used on Result or Option types".to_string()),
                );
                Ok(Type::Unknown)
            }
        }
    }

    /// Analyze a closure expression
    fn analyze_closure(
        &mut self,
        parameters: &[ClosureParam],
        body: &Expr,
        expr_id: usize,
        span: Span,
    ) -> Result<Type> {
        // Enter a new scope for the closure
        self.symbol_table.enter_scope();
        self.inference_ctx.push_scope();

        // Define parameters in the closure scope
        let mut param_types = Vec::new();
        for param in parameters {
            let param_type = if let Some(ref type_ann) = param.type_ann {
                type_ann_to_type(type_ann)
            } else {
                Type::Unknown // Will be inferred
            };

            self.symbol_table.define_variable(
                param.name.clone(),
                param_type.clone(),
                span,
                false, // Parameters are immutable by default
            )?;

            param_types.push(param_type);
        }

        // Perform capture analysis before analyzing the body
        let capture_analyzer = CaptureAnalyzer::new(&self.symbol_table);
        let captures =
            capture_analyzer.analyze_closure(parameters, body, self.symbol_table.current_scope());

        // Store capture information
        self.closure_captures.insert(expr_id, captures);

        // Analyze the closure body
        let return_type = self.analyze_expr(body)?;

        // Exit closure scope
        self.symbol_table.exit_scope();
        self.inference_ctx.pop_scope();

        // Return the function type
        Ok(Type::Function {
            params: param_types,
            ret: Box::new(return_type),
        })
    }

    /// Analyze try-catch expression
    fn analyze_try_catch(
        &mut self,
        try_expr: &Expr,
        catch_clauses: &[CatchClause],
        finally_block: &Option<Block>,
        span: crate::source::Span,
    ) -> Result<Type> {
        // Analyze the try expression
        let try_type = self.analyze_expr(try_expr)?;

        // Analyze catch clauses
        let mut catch_types = Vec::new();
        for clause in catch_clauses {
            // If there's a variable binding, add it to the scope
            if let Some(var) = &clause.var {
                // Create an error type for the caught variable
                let error_type = clause
                    .error_type
                    .as_ref()
                    .map(type_ann_to_type)
                    .unwrap_or(Type::String); // Default to String for error messages

                // Add the error variable to scope temporarily
                self.symbol_table.define_variable(
                    var.clone(),
                    error_type.clone(),
                    clause.span,
                    false,
                )?;
            }

            // Analyze the condition if present
            if let Some(condition) = &clause.condition {
                let condition_type = self.analyze_expr(condition)?;
                if !self.is_bool_type(&condition_type) {
                    self.add_error(SemanticError::type_mismatch(
                        Type::Bool,
                        condition_type,
                        clause.span,
                    ));
                }
            }

            // Analyze the catch handler block
            let catch_type = self.analyze_block_expr(&clause.handler)?;
            catch_types.push(catch_type);

            // Remove the error variable from scope
            if let Some(var) = &clause.var {
                // Note: In a more sophisticated implementation, we'd have proper scope management
                // For now, we'll just note that the variable should be removed
            }
        }

        // Analyze finally block if present
        if let Some(finally) = finally_block {
            self.analyze_block(finally)?;
        }

        // The type of the try-catch expression is the union of try type and all catch types
        // For simplicity, we'll use the try type as the primary type
        // In a more sophisticated type system, we'd create a union type
        if catch_types.is_empty() {
            Ok(try_type)
        } else {
            // Check if all catch clauses return the same type as the try expression
            let unified_type = catch_types
                .into_iter()
                .fold(try_type.clone(), |acc, catch_type| {
                    if self.types_equal(&acc, &catch_type) {
                        acc
                    } else {
                        // If types don't match, use the most general type
                        Type::Unknown
                    }
                });
            Ok(unified_type)
        }
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
            PatternKind::EnumConstructor {
                enum_name,
                variant,
                args,
            } => {
                // Verify the enum type matches
                if let Type::Named(type_name) = expected_type {
                    // If enum_name is specified, verify it matches
                    if let Some(specified_enum) = enum_name {
                        if specified_enum != type_name {
                            self.add_error(SemanticError::type_mismatch(
                                Type::Named(type_name.clone()),
                                Type::Named(specified_enum.clone()),
                                pattern.span,
                            ));
                            return Ok(());
                        }
                    }

                    // Look up the enum definition
                    if let Some(symbol) = self.symbol_table.lookup(type_name) {
                        if let SymbolKind::Enum(enum_info) = &symbol.kind {
                            // Find the variant
                            if let Some(variant_info) =
                                enum_info.variants.iter().find(|v| v.name == *variant)
                            {
                                // Check argument patterns match the variant type
                                use crate::semantic::symbol::EnumVariantType;
                                match (&variant_info.variant_type, args) {
                                    (EnumVariantType::Unit, None) => {
                                        // Unit variant with no args - correct
                                    }
                                    (EnumVariantType::Unit, Some(_)) => {
                                        self.add_error(
                                            SemanticError::new(
                                                SemanticErrorKind::InvalidOperation {
                                                    op: "pattern match".to_string(),
                                                    ty: expected_type.clone(),
                                                },
                                                pattern.span,
                                            )
                                            .with_note(format!("Variant '{}' is a unit variant and takes no arguments", variant))
                                        );
                                    }
                                    (EnumVariantType::Tuple(types), Some(arg_patterns)) => {
                                        if types.len() != arg_patterns.len() {
                                            self.add_error(
                                                SemanticError::new(
                                                    SemanticErrorKind::InvalidOperation {
                                                        op: "pattern match".to_string(),
                                                        ty: expected_type.clone(),
                                                    },
                                                    pattern.span,
                                                )
                                                .with_note(format!(
                                                    "Variant '{}' expects {} arguments, but {} were provided",
                                                    variant, types.len(), arg_patterns.len()
                                                ))
                                            );
                                        } else {
                                            // Clone the types to avoid borrowing issues
                                            let types_clone: Vec<_> = types.clone();
                                            // Analyze each argument pattern with its expected type
                                            for (arg_pattern, arg_type) in
                                                arg_patterns.iter().zip(types_clone.iter())
                                            {
                                                self.analyze_pattern(arg_pattern, arg_type)?;
                                            }
                                        }
                                    }
                                    (EnumVariantType::Tuple(_), None) => {
                                        self.add_error(
                                            SemanticError::new(
                                                SemanticErrorKind::InvalidOperation {
                                                    op: "pattern match".to_string(),
                                                    ty: expected_type.clone(),
                                                },
                                                pattern.span,
                                            )
                                            .with_note(format!("Variant '{}' is a tuple variant and requires arguments", variant))
                                        );
                                    }
                                    (EnumVariantType::Struct(_), _) => {
                                        // TODO: Handle struct variant patterns
                                        self.add_error(
                                            SemanticError::new(
                                                SemanticErrorKind::InvalidOperation {
                                                    op: "pattern match".to_string(),
                                                    ty: expected_type.clone(),
                                                },
                                                pattern.span,
                                            )
                                            .with_note(
                                                "Struct variant patterns are not yet implemented"
                                                    .to_string(),
                                            ),
                                        );
                                    }
                                }
                            } else {
                                self.add_error(
                                    SemanticError::new(
                                        SemanticErrorKind::UndefinedVariable(variant.clone()),
                                        pattern.span,
                                    )
                                    .with_note(format!(
                                        "No variant '{}' found in enum '{}'",
                                        variant, type_name
                                    )),
                                );
                            }
                        } else {
                            self.add_error(
                                SemanticError::type_mismatch(
                                    expected_type.clone(),
                                    Type::Unknown,
                                    pattern.span,
                                )
                                .with_note(format!("'{}' is not an enum type", type_name)),
                            );
                        }
                    }
                } else {
                    self.add_error(
                        SemanticError::type_mismatch(
                            expected_type.clone(),
                            Type::Unknown,
                            pattern.span,
                        )
                        .with_note(
                            "Expected an enum type for enum constructor pattern".to_string(),
                        ),
                    );
                }
                Ok(())
            }
        }
    }

    /// Get collected errors
    pub fn errors(&self) -> &[SemanticError] {
        &self.errors
    }

    /// Get the collected generic instantiations
    pub fn generic_instantiations(&self) -> &[GenericInstantiation] {
        &self.generic_instantiations
    }

    /// Get the type information collected during analysis
    pub fn type_info(&self) -> &HashMap<usize, Type> {
        &self.type_info
    }

    /// Get capture information for a closure
    pub fn get_closure_captures(&self, expr_id: usize) -> Option<&Vec<CaptureInfo>> {
        self.closure_captures.get(&expr_id)
    }

    /// Extract all closure captures for lowering
    pub fn extract_closure_captures(&self) -> HashMap<usize, Vec<(String, Type, bool)>> {
        self.closure_captures
            .iter()
            .map(|(expr_id, captures)| {
                let lowerer_captures = captures
                    .iter()
                    .map(|capture| {
                        let is_mutable = matches!(
                            capture.capture_mode,
                            super::capture_analysis::CaptureMode::ByReference
                        );
                        (capture.name.clone(), capture.ty.clone(), is_mutable)
                    })
                    .collect();
                (*expr_id, lowerer_captures)
            })
            .collect()
    }

    /// Add a generic instantiation for tracking
    pub fn add_generic_instantiation(&mut self, instantiation: GenericInstantiation) {
        self.generic_instantiations.push(instantiation);
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
        self.type_info.clone()
    }

    /// Record the type of an expression
    fn record_expression_type(&mut self, expr_id: usize, type_: Type) {
        self.type_info.insert(expr_id, type_);
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
            StmtKind::Struct { .. } | StmtKind::Enum { .. } | StmtKind::Impl(_) => {
                // Struct/enum/impl definitions not allowed in function bodies
                self.add_error(SemanticError::const_function_violation(
                    "struct/enum/impl definitions not allowed in function bodies",
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
                                        format!("@const functions can only call other @const functions, but '{}' is not @const", name),
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

            // Error propagation is not allowed in const functions
            ExprKind::ErrorPropagation { .. } => {
                self.add_error(SemanticError::const_function_violation(
                    "Error propagation (?) is not allowed in @const functions",
                    expr.span,
                ));
                Ok(())
            }

            // Try-catch expressions are not allowed in const functions
            ExprKind::TryCatch { .. } => {
                self.add_error(SemanticError::const_function_violation(
                    "Try-catch expressions are not allowed in @const functions",
                    expr.span,
                ));
                Ok(())
            }
            ExprKind::Closure { .. } => {
                self.add_error(SemanticError::const_function_violation(
                    "Closure expressions are not allowed in @const functions",
                    expr.span,
                ));
                Ok(())
            }
        }
    }

    /// Instantiate a generic function with concrete types based on call arguments
    fn instantiate_generic_function(
        &mut self,
        signature: &FunctionSignature,
        arg_types: &[Type],
    ) -> Result<FunctionSignature> {
        let _generic_params = signature.generic_params.as_ref().unwrap();

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
                // Handle complex generic types
                self.infer_type_substitutions_from_generic(
                    param_type,
                    arg_type,
                    &mut type_substitutions,
                );
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

        // Track this generic instantiation for monomorphization
        if !type_substitutions.is_empty() {
            // We need the function name, but it's not passed to this method
            // TODO: Pass function name to track instantiations properly
            // For now, we'll track in the caller
        }

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

    /// Infer type substitutions from generic types (e.g., Vec<T> matched with Vec<i32>)
    fn infer_type_substitutions_from_generic(
        &self,
        param_type: &Type,
        arg_type: &Type,
        substitutions: &mut HashMap<String, Type>,
    ) {
        match (param_type, arg_type) {
            // Direct type parameter match
            (Type::TypeParam(name), concrete_type) => {
                substitutions.insert(name.clone(), concrete_type.clone());
            }
            // Generic type match (e.g., Vec<T> with Vec<i32>)
            (
                Type::Generic {
                    name: pname,
                    args: pargs,
                },
                Type::Generic {
                    name: aname,
                    args: aargs,
                },
            ) if pname == aname && pargs.len() == aargs.len() => {
                for (parg, aarg) in pargs.iter().zip(aargs.iter()) {
                    self.infer_type_substitutions_from_generic(parg, aarg, substitutions);
                }
            }
            // Array type match
            (Type::Array(pelem), Type::Array(aelem)) => {
                self.infer_type_substitutions_from_generic(pelem, aelem, substitutions);
            }
            // Option type match
            (Type::Option(pinner), Type::Option(ainner)) => {
                self.infer_type_substitutions_from_generic(pinner, ainner, substitutions);
            }
            // Result type match
            (Type::Result { ok: pok, err: perr }, Type::Result { ok: aok, err: aerr }) => {
                self.infer_type_substitutions_from_generic(pok, aok, substitutions);
                self.infer_type_substitutions_from_generic(perr, aerr, substitutions);
            }
            // Function type match
            (
                Type::Function {
                    params: pparams,
                    ret: pret,
                },
                Type::Function {
                    params: aparams,
                    ret: aret,
                },
            ) if pparams.len() == aparams.len() => {
                for (pparam, aparam) in pparams.iter().zip(aparams.iter()) {
                    self.infer_type_substitutions_from_generic(pparam, aparam, substitutions);
                }
                self.infer_type_substitutions_from_generic(pret, aret, substitutions);
            }
            // No match needed for other cases
            _ => {}
        }
    }

    /// Analyze an impl block
    fn analyze_impl_block(&mut self, impl_block: &ImplBlock) -> Result<()> {
        // Store the impl block for method resolution
        self.impl_blocks.push(impl_block.clone());

        // Clear method cache for the target type
        let target_type_name = impl_block.type_name.clone();
        self.method_cache
            .retain(|(type_name, _), _| type_name != &target_type_name);

        // Enter new scope for impl block
        self.symbol_table.enter_scope();
        self.inference_ctx.push_scope();

        // Set up generic context for impl block
        let mut impl_context = AnalysisContext::new();
        impl_context.generic_params = impl_block.generic_params.clone();
        self.push_context(impl_context);

        // Define generic type parameters in scope
        if let Some(generics) = &impl_block.generic_params {
            for generic_param in &generics.params {
                self.inference_ctx.define_type_param(&generic_param.name);
            }
        }

        // Analyze each method in the impl block
        for method in &impl_block.methods {
            // Create a TypeAnn from the type name for method analysis
            let target_type_ann = TypeAnn {
                kind: TypeKind::Named(impl_block.type_name.clone()),
                span: impl_block.span,
            };
            self.analyze_method(method, &target_type_ann)?;
        }

        // Exit impl context
        self.pop_context();
        self.symbol_table.exit_scope();
        self.inference_ctx.pop_scope();

        Ok(())
    }

    /// Analyze a method within an impl block
    fn analyze_method(&mut self, method: &Method, target_type: &TypeAnn) -> Result<()> {
        // Create a unique method name that includes the type
        let method_name = format!("{}::{}", target_type, method.name);

        // Convert parameter types, including self parameter
        let mut param_types = Vec::new();

        // First parameter is always self
        let self_type = type_ann_to_type(target_type);
        param_types.push(("self".to_string(), self_type));

        // Add other parameters
        for param in &method.params {
            let param_type = type_ann_to_type(&param.type_ann);
            param_types.push((param.name.clone(), param_type));
        }

        // Convert return type
        let return_type = method
            .ret_type
            .as_ref()
            .map(type_ann_to_type)
            .unwrap_or(Type::Unknown);

        // Create method signature
        let signature = FunctionSignature {
            generic_params: method.generic_params.clone(),
            params: param_types,
            return_type,
            is_const: false, // TODO: Support @const methods
            is_async: method.is_async,
        };

        // Define the method as a function
        match self
            .symbol_table
            .define_function(method_name, signature, method.span)
        {
            Ok(_) => {}
            Err(err) => {
                self.add_error(
                    SemanticError::new(
                        SemanticErrorKind::DuplicateFunction(method.name.clone()),
                        method.span,
                    )
                    .with_note(err),
                );
            }
        }

        // Enter method scope
        self.symbol_table.enter_scope();
        self.inference_ctx.push_scope();

        // Enter memory safety scope for the method
        if self.memory_safety_enabled {
            self.memory_safety_ctx.enter_scope();
        }

        // Create method context
        let method_context = AnalysisContext {
            current_function_return: Some(
                method
                    .ret_type
                    .as_ref()
                    .map(type_ann_to_type)
                    .unwrap_or(Type::Unknown),
            ),
            in_loop: false,
            _in_const_function: false, // TODO: Support @const methods
            in_async_function: method.is_async,
            generic_params: method.generic_params.clone(),
            generic_param_names: Vec::new(),
        };
        self.push_context(method_context);

        // Define generic type parameters for the method
        if let Some(generics) = &method.generic_params {
            for generic_param in &generics.params {
                self.inference_ctx.define_type_param(&generic_param.name);
            }
        }

        // Define self parameter
        let self_type = type_ann_to_type(target_type);
        match self
            .symbol_table
            .define_parameter("self".to_string(), self_type.clone(), method.span)
        {
            Ok(symbol_id) => {
                self.symbol_table.mark_used(symbol_id);

                // Memory safety: Define and initialize self parameter
                if self.memory_safety_enabled {
                    // Define the self parameter in memory safety context
                    if let Err(_err) = self.memory_safety_ctx.define_variable(
                        "self".to_string(),
                        self_type,
                        false, // self is immutable
                        method.span,
                    ) {
                        // This shouldn't happen as symbol table already checked for duplicates
                    }

                    // self parameter is initialized by the caller
                    if let Err(err) = self
                        .memory_safety_ctx
                        .initialize_variable("self", method.span)
                    {
                        // This shouldn't fail as we just defined the variable
                    }
                }
            }
            Err(err) => {
                self.add_error(
                    SemanticError::duplicate_variable("self", method.span).with_note(err),
                );
            }
        }

        // Define method parameters
        for param in &method.params {
            let param_type = type_ann_to_type(&param.type_ann);
            match self.symbol_table.define_parameter(
                param.name.clone(),
                param_type,
                param.type_ann.span,
            ) {
                Ok(symbol_id) => {
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

        // Analyze method body
        if false {
            // TODO: Support @const methods
            self.analyze_const_function_body(&method.body)?;
        } else {
            self.analyze_block(&method.body)?;
        }

        // Exit method context
        self.pop_context();
        self.symbol_table.exit_scope();
        self.inference_ctx.pop_scope();

        // Exit memory safety scope
        if self.memory_safety_enabled {
            self.memory_safety_ctx.exit_scope(method.span);
        }

        Ok(())
    }

    /// Resolve method calls on types
    fn resolve_method_call(
        &mut self,
        receiver_type: &Type,
        method_name: &str,
        args: &[Expr],
        span: crate::source::Span,
    ) -> Result<Type> {
        // Check cache first
        let cache_key = (receiver_type.to_string(), method_name.to_string());
        if let Some(methods) = self.method_cache.get(&cache_key) {
            if !methods.is_empty() {
                let methods = methods.clone(); // Clone to avoid borrow checker issues
                return self.analyze_method_call_with_methods(
                    receiver_type,
                    method_name,
                    args,
                    &methods,
                    span,
                );
            }
        }

        // Find matching impl blocks
        let mut matching_methods = Vec::new();
        for impl_block in &self.impl_blocks.clone() {
            let impl_target_type = Type::Named(impl_block.type_name.clone());

            // Check if receiver type matches impl target type
            if self.types_match(receiver_type, &impl_target_type) {
                // Find methods with matching name
                for method in &impl_block.methods {
                    if method.name == method_name {
                        matching_methods.push(method.clone());
                    }
                }
            }
        }

        // Cache the result
        self.method_cache
            .insert(cache_key, matching_methods.clone());

        if matching_methods.is_empty() {
            self.add_error(SemanticError::new(
                SemanticErrorKind::MethodNotFound {
                    type_name: receiver_type.to_string(),
                    method_name: method_name.to_string(),
                },
                span,
            ));
            return Ok(Type::Unknown);
        }

        self.analyze_method_call_with_methods(
            receiver_type,
            method_name,
            args,
            &matching_methods,
            span,
        )
    }

    /// Analyze method call with resolved methods
    fn analyze_method_call_with_methods(
        &mut self,
        receiver_type: &Type,
        method_name: &str,
        args: &[Expr],
        methods: &[Method],
        span: crate::source::Span,
    ) -> Result<Type> {
        // For simplicity, use the first matching method
        // In a complete implementation, we would do overload resolution
        if let Some(method) = methods.first() {
            // Analyze arguments
            let arg_types: Vec<Type> = args
                .iter()
                .map(|arg| self.analyze_expr(arg))
                .collect::<Result<Vec<_>>>()?;

            // Check argument count (excluding self)
            if args.len() != method.params.len() {
                self.add_error(
                    SemanticError::argument_count_mismatch(method.params.len(), args.len(), span)
                        .with_note(format!(
                            "method '{}' on type {} expects {} arguments, but {} were provided",
                            method_name,
                            receiver_type,
                            method.params.len(),
                            args.len()
                        )),
                );
            } else {
                // Check each argument type
                for (i, (param, (arg, arg_type))) in method
                    .params
                    .iter()
                    .zip(args.iter().zip(arg_types.iter()))
                    .enumerate()
                {
                    let param_type = type_ann_to_type(&param.type_ann);

                    if !arg_type.is_assignable_to(&param_type) {
                        self.add_error(
                            SemanticError::type_mismatch(
                                param_type.clone(),
                                arg_type.clone(),
                                arg.span,
                            )
                            .with_note(format!(
                                "argument {} to method '{}' has wrong type",
                                i + 1,
                                method_name
                            ))
                            .with_note(format!(
                                "parameter '{}' expects type {}, but argument has type {}",
                                param.name, param_type, arg_type
                            )),
                        );
                    }
                }
            }

            // Return method's return type
            Ok(method
                .ret_type
                .as_ref()
                .map(type_ann_to_type)
                .unwrap_or(Type::Unknown))
        } else {
            self.add_error(SemanticError::new(
                SemanticErrorKind::MethodNotFound {
                    type_name: receiver_type.to_string(),
                    method_name: method_name.to_string(),
                },
                span,
            ));
            Ok(Type::Unknown)
        }
    }

    /// Check if two types match for method resolution
    fn types_match(&self, type1: &Type, type2: &Type) -> bool {
        match (type1, type2) {
            // Exact match
            (t1, t2) if t1 == t2 => true,
            // Unknown matches everything (gradual typing)
            (Type::Unknown, _) | (_, Type::Unknown) => true,
            // Named types match by name
            (Type::Named(name1), Type::Named(name2)) => name1 == name2,
            // Generic types match if base names match
            (Type::Generic { name: name1, .. }, Type::Generic { name: name2, .. }) => {
                name1 == name2
            }
            // Array types match if element types match
            (Type::Array(elem1), Type::Array(elem2)) => self.types_match(elem1, elem2),
            // Other cases don't match
            _ => false,
        }
    }

    /// Enhanced member access analysis with method call support
    fn analyze_member_enhanced(
        &mut self,
        object: &Expr,
        property: &str,
        span: crate::source::Span,
    ) -> Result<Type> {
        let object_type = self.analyze_expr(object)?;

        // Try method resolution first
        if let Ok(method_type) = self.resolve_method_call(&object_type, property, &[], span) {
            return Ok(method_type);
        }

        // Fall back to field access
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

    /// Analyze a struct constructor expression
    fn analyze_struct_constructor(
        &mut self,
        name: &str,
        fields: &[(String, Expr)],
        span: crate::source::Span,
    ) -> Result<Type> {
        // Look up the struct definition and clone the info we need
        let struct_info_opt = self
            .symbol_table
            .lookup(name)
            .and_then(|symbol| symbol.struct_info().cloned());

        if let Some(struct_info) = struct_info_opt {
            // Check if struct has generic parameters
            // Check if struct has generic parameters
            let (final_type, type_args) = if struct_info.generic_params.is_some() {
                // Infer generic type arguments from field values
                let inferred_args =
                    self.infer_struct_type_args(name, &struct_info, fields, span)?;

                // Track this generic instantiation
                if !inferred_args.is_empty() {
                    let instantiation = GenericInstantiation {
                        function_name: format!("{}::new", name), // Constructor convention
                        type_args: inferred_args.clone(),
                        span,
                    };
                    self.add_generic_instantiation(instantiation);
                }

                (
                    Type::Generic {
                        name: name.to_string(),
                        args: inferred_args.clone(),
                    },
                    inferred_args,
                )
            } else {
                (Type::Named(name.to_string()), vec![])
            };

            // Analyze and validate fields
            let mut seen_fields = std::collections::HashSet::new();
            let mut provided_fields = std::collections::HashMap::new();

            for (field_name, field_expr) in fields {
                // Check for duplicate fields
                if !seen_fields.insert(field_name.clone()) {
                    self.add_error(
                        SemanticError::new(
                            SemanticErrorKind::DuplicateField(field_name.clone()),
                            field_expr.span,
                        )
                        .with_note(format!("field '{}' provided multiple times", field_name)),
                    );
                    continue;
                }

                // Analyze field expression
                let field_value_type = self.analyze_expr(field_expr)?;
                provided_fields.insert(field_name.clone(), (field_value_type, field_expr.span));
            }

            // Check that all required fields are provided and types match
            for (expected_field_name, expected_field_type) in &struct_info.fields {
                if let Some((provided_type, field_span)) = provided_fields.get(expected_field_name)
                {
                    // Substitute generic parameters if needed
                    let concrete_field_type = if !type_args.is_empty() {
                        self.substitute_type_params(
                            expected_field_type,
                            &type_args,
                            &struct_info.generic_params,
                        )
                    } else {
                        expected_field_type.clone()
                    };

                    // Check type compatibility
                    if !self.is_assignable_to(provided_type, &concrete_field_type) {
                        self.add_error(
                            SemanticError::type_mismatch(
                                concrete_field_type.clone(),
                                provided_type.clone(),
                                *field_span,
                            )
                            .with_note(format!(
                                "field '{}' expects type {}, but value has type {}",
                                expected_field_name, expected_field_type, provided_type
                            )),
                        );
                    }
                } else {
                    // Field is missing
                    self.add_error(
                        SemanticError::new(
                            SemanticErrorKind::MissingField(expected_field_name.clone()),
                            span,
                        )
                        .with_note(format!(
                            "struct '{}' requires field '{}'",
                            name, expected_field_name
                        )),
                    );
                }
            }

            // Check for extra fields
            for (provided_field_name, _) in &provided_fields {
                let field_exists = struct_info
                    .fields
                    .iter()
                    .any(|(name, _)| name == provided_field_name);

                if !field_exists {
                    self.add_error(
                        SemanticError::new(
                            SemanticErrorKind::UnknownField(provided_field_name.clone()),
                            span,
                        )
                        .with_note(format!(
                            "struct '{}' has no field '{}'",
                            name, provided_field_name
                        )),
                    );
                }
            }

            Ok(final_type)
        } else {
            // Struct doesn't exist
            self.add_error(SemanticError::undefined_type(name, span));
            Ok(Type::Unknown)
        }
    }

    /// Analyze an enum constructor expression
    fn analyze_enum_constructor(
        &mut self,
        enum_name: Option<&str>,
        variant_name: &str,
        args: &crate::parser::EnumConstructorArgs,
        span: crate::source::Span,
    ) -> Result<Type> {
        // For qualified variants (EnumName::Variant), use the provided enum name
        // For unqualified variants, check if it's a built-in constructor
        let enum_name = if let Some(name) = enum_name {
            name
        } else {
            // Check for built-in constructors
            match variant_name {
                "Some" | "None" => "Option",
                "Ok" | "Err" => "Result",
                _ => {
                    // For non-built-in variants, require qualification
                    self.add_error(
                        SemanticError::new(
                            SemanticErrorKind::UnqualifiedEnumVariant(variant_name.to_string()),
                            span,
                        )
                        .with_note("enum variants must be qualified with their enum name (e.g., Color::Red)".to_string())
                    );
                    return Ok(Type::Unknown);
                }
            }
        };

        // Look up the enum definition and clone the info we need
        let enum_info_opt = self
            .symbol_table
            .lookup(enum_name)
            .and_then(|symbol| symbol.enum_info().cloned());

        if let Some(enum_info) = enum_info_opt {
            // Find the variant
            let variant_info = enum_info.variants.iter().find(|v| v.name == variant_name);

            if let Some(variant) = variant_info {
                // Check if enum has generic parameters
                let (final_type, type_args) = if enum_info.generic_params.is_some() {
                    // Infer generic type arguments from variant arguments
                    let inferred_args =
                        self.infer_enum_type_args(enum_name, &enum_info, variant, args, span)?;

                    // Track this generic instantiation
                    if !inferred_args.is_empty() {
                        let instantiation = GenericInstantiation {
                            function_name: format!("{}::{}", enum_name, variant_name),
                            type_args: inferred_args.clone(),
                            span,
                        };
                        self.add_generic_instantiation(instantiation);
                    }

                    (
                        Type::Generic {
                            name: enum_name.to_string(),
                            args: inferred_args.clone(),
                        },
                        inferred_args,
                    )
                } else {
                    (Type::Named(enum_name.to_string()), vec![])
                };

                // Validate variant arguments
                match (&variant.variant_type, args) {
                    (
                        crate::semantic::symbol::EnumVariantType::Unit,
                        crate::parser::EnumConstructorArgs::Unit,
                    ) => {
                        // Unit variant with no args - correct
                    }
                    (
                        crate::semantic::symbol::EnumVariantType::Tuple(expected_types),
                        crate::parser::EnumConstructorArgs::Tuple(arg_exprs),
                    ) => {
                        // Tuple variant - check argument count and types
                        if arg_exprs.len() != expected_types.len() {
                            self.add_error(
                                SemanticError::argument_count_mismatch(
                                    expected_types.len(),
                                    arg_exprs.len(),
                                    span,
                                )
                                .with_note(format!(
                                    "variant '{}::{}' expects {} argument{}, but {} {} provided",
                                    enum_name,
                                    variant_name,
                                    expected_types.len(),
                                    if expected_types.len() == 1 { "" } else { "s" },
                                    arg_exprs.len(),
                                    if arg_exprs.len() == 1 { "was" } else { "were" }
                                )),
                            );
                        } else {
                            // Check each argument type
                            for (i, (expected_type, arg_expr)) in
                                expected_types.iter().zip(arg_exprs.iter()).enumerate()
                            {
                                let arg_type = self.analyze_expr(arg_expr)?;

                                // Substitute generic parameters if needed
                                let concrete_type = if !type_args.is_empty() {
                                    self.substitute_type_params(
                                        expected_type,
                                        &type_args,
                                        &enum_info.generic_params,
                                    )
                                } else {
                                    expected_type.clone()
                                };

                                if !self.is_assignable_to(&arg_type, &concrete_type) {
                                    self.add_error(
                                        SemanticError::type_mismatch(
                                            concrete_type.clone(),
                                            arg_type,
                                            arg_expr.span,
                                        )
                                        .with_note(
                                            format!(
                                                "argument {} to variant '{}::{}' has wrong type",
                                                i + 1,
                                                enum_name,
                                                variant_name
                                            ),
                                        ),
                                    );
                                }
                            }
                        }
                    }
                    (
                        crate::semantic::symbol::EnumVariantType::Struct(expected_fields),
                        crate::parser::EnumConstructorArgs::Struct(field_exprs),
                    ) => {
                        // Struct variant - validate fields
                        let mut seen_fields = std::collections::HashSet::new();
                        let mut provided_fields = std::collections::HashMap::new();

                        for (field_name, field_expr) in field_exprs {
                            if !seen_fields.insert(field_name.clone()) {
                                self.add_error(
                                    SemanticError::new(
                                        SemanticErrorKind::DuplicateField(field_name.clone()),
                                        field_expr.span,
                                    )
                                    .with_note(format!(
                                        "field '{}' provided multiple times",
                                        field_name
                                    )),
                                );
                                continue;
                            }

                            let field_type = self.analyze_expr(field_expr)?;
                            provided_fields
                                .insert(field_name.clone(), (field_type, field_expr.span));
                        }

                        // Check all required fields
                        for (expected_field_name, expected_field_type) in expected_fields {
                            if let Some((provided_type, field_span)) =
                                provided_fields.get(expected_field_name)
                            {
                                // Substitute generic parameters if needed
                                let concrete_type = if !type_args.is_empty() {
                                    self.substitute_type_params(
                                        expected_field_type,
                                        &type_args,
                                        &enum_info.generic_params,
                                    )
                                } else {
                                    expected_field_type.clone()
                                };

                                if !self.is_assignable_to(provided_type, &concrete_type) {
                                    self.add_error(
                                        SemanticError::type_mismatch(
                                            concrete_type,
                                            provided_type.clone(),
                                            *field_span,
                                        )
                                        .with_note(
                                            format!(
                                                "field '{}' expects type {}, but value has type {}",
                                                expected_field_name,
                                                expected_field_type,
                                                provided_type
                                            ),
                                        ),
                                    );
                                }
                            } else {
                                self.add_error(
                                    SemanticError::new(
                                        SemanticErrorKind::MissingField(
                                            expected_field_name.clone(),
                                        ),
                                        span,
                                    )
                                    .with_note(format!(
                                        "variant '{}::{}' requires field '{}'",
                                        enum_name, variant_name, expected_field_name
                                    )),
                                );
                            }
                        }

                        // Check for extra fields
                        for (provided_field_name, _) in &provided_fields {
                            let field_exists = expected_fields
                                .iter()
                                .any(|(name, _)| name == provided_field_name);

                            if !field_exists {
                                self.add_error(
                                    SemanticError::new(
                                        SemanticErrorKind::UnknownField(
                                            provided_field_name.clone(),
                                        ),
                                        span,
                                    )
                                    .with_note(format!(
                                        "variant '{}::{}' has no field '{}'",
                                        enum_name, variant_name, provided_field_name
                                    )),
                                );
                            }
                        }
                    }
                    _ => {
                        // Mismatch between variant type and constructor args
                        let expected_form = match &variant.variant_type {
                            crate::semantic::symbol::EnumVariantType::Unit => "no arguments",
                            crate::semantic::symbol::EnumVariantType::Tuple(_) => "tuple arguments",
                            crate::semantic::symbol::EnumVariantType::Struct(_) => "struct fields",
                        };
                        let provided_form = match args {
                            crate::parser::EnumConstructorArgs::Unit => "no arguments",
                            crate::parser::EnumConstructorArgs::Tuple(_) => "tuple arguments",
                            crate::parser::EnumConstructorArgs::Struct(_) => "struct fields",
                        };

                        self.add_error(SemanticError::new(
                            SemanticErrorKind::VariantFormMismatch {
                                variant: format!("{}::{}", enum_name, variant_name),
                                expected: expected_form.to_string(),
                                found: provided_form.to_string(),
                            },
                            span,
                        ));
                    }
                }

                Ok(final_type)
            } else {
                // Variant doesn't exist in this enum
                self.add_error(SemanticError::new(
                    SemanticErrorKind::UnknownVariant {
                        enum_name: enum_name.to_string(),
                        variant_name: variant_name.to_string(),
                    },
                    span,
                ));
                Ok(Type::Unknown)
            }
        } else {
            // Enum doesn't exist
            self.add_error(SemanticError::undefined_type(enum_name, span));
            Ok(Type::Unknown)
        }
    }

    /// Infer generic type arguments for a struct constructor
    fn infer_struct_type_args(
        &mut self,
        _struct_name: &str,
        struct_info: &crate::semantic::symbol::StructInfo,
        provided_fields: &[(String, Expr)],
        span: crate::source::Span,
    ) -> Result<Vec<Type>> {
        if let Some(generic_params) = &struct_info.generic_params {
            // Use the constructor inference engine
            let mut engine = crate::inference::ConstructorInferenceEngine::new();

            // Initialize type variables for generic parameters
            engine.initialize_generic_params(generic_params);

            // Analyze provided field values and collect their types
            let mut provided_field_types = Vec::new();
            for (field_name, field_expr) in provided_fields {
                let field_type = self.analyze_expr(field_expr)?;
                provided_field_types.push((field_name.clone(), field_type));
            }

            // Convert struct field types to TypeAnn format
            let expected_fields: Vec<(String, crate::parser::TypeAnn)> = struct_info
                .fields
                .iter()
                .map(|(name, ty)| (name.clone(), type_to_type_ann(ty)))
                .collect();

            // Generate constraints
            engine.constrain_struct_fields(&expected_fields, &provided_field_types, span)?;

            // Infer types
            let result = engine.infer(generic_params)?;

            Ok(result.type_args)
        } else {
            Ok(vec![])
        }
    }

    /// Infer generic type arguments for an enum constructor
    fn infer_enum_type_args(
        &mut self,
        _enum_name: &str,
        enum_info: &crate::semantic::symbol::EnumInfo,
        variant: &crate::semantic::symbol::EnumVariantInfo,
        args: &crate::parser::EnumConstructorArgs,
        span: crate::source::Span,
    ) -> Result<Vec<Type>> {
        if let Some(generic_params) = &enum_info.generic_params {
            // Use the constructor inference engine
            let mut engine = crate::inference::ConstructorInferenceEngine::new();

            // Initialize type variables for generic parameters
            engine.initialize_generic_params(generic_params);

            // Collect types based on variant arguments
            let provided_types = match args {
                crate::parser::EnumConstructorArgs::Unit => {
                    // Unit variants have no arguments
                    vec![]
                }
                crate::parser::EnumConstructorArgs::Tuple(exprs) => {
                    // Analyze each expression and collect types
                    let mut types = Vec::new();
                    for expr in exprs {
                        let ty = self.analyze_expr(expr)?;
                        types.push(ty);
                    }
                    types
                }
                crate::parser::EnumConstructorArgs::Struct(fields) => {
                    // For struct variants, we need to match fields by name
                    // But for constraint generation, we just need the types
                    let mut types = Vec::new();
                    for (_, expr) in fields {
                        let ty = self.analyze_expr(expr)?;
                        types.push(ty);
                    }
                    types
                }
            };

            // Get expected types from variant definition
            let expected_types = match &variant.variant_type {
                crate::semantic::symbol::EnumVariantType::Unit => vec![],
                crate::semantic::symbol::EnumVariantType::Tuple(types) => types.clone(),
                crate::semantic::symbol::EnumVariantType::Struct(fields) => {
                    // Extract types from struct fields
                    fields.iter().map(|(_, ty)| ty.clone()).collect()
                }
            };

            // Generate constraints
            engine.constrain_enum_variant_args(&expected_types, &provided_types, span)?;

            // Infer types
            let result = engine.infer(generic_params)?;

            Ok(result.type_args)
        } else {
            Ok(vec![])
        }
    }

    /// Substitute type parameters with concrete types
    fn substitute_type_params(
        &self,
        ty: &Type,
        type_args: &[Type],
        generic_params: &Option<crate::parser::GenericParams>,
    ) -> Type {
        // If no generic params, nothing to substitute
        let generic_params = match generic_params {
            Some(params) => params,
            None => return ty.clone(),
        };

        // Build substitution map from generic parameter names to concrete types
        let mut substitution = HashMap::new();
        for (i, param) in generic_params.params.iter().enumerate() {
            if let Some(concrete_type) = type_args.get(i) {
                substitution.insert(param.name.as_str(), concrete_type);
            }
        }

        // Recursively substitute type parameters
        self.substitute_type_with_map(ty, &substitution)
    }

    /// Helper function to recursively substitute types using a substitution map
    fn substitute_type_with_map(&self, ty: &Type, substitution: &HashMap<&str, &Type>) -> Type {
        match ty {
            Type::TypeParam(name) => {
                // Substitute if we have a mapping for this parameter
                if let Some(&concrete_type) = substitution.get(name.as_str()) {
                    concrete_type.clone()
                } else {
                    ty.clone()
                }
            }
            Type::Array(elem) => {
                Type::Array(Box::new(self.substitute_type_with_map(elem, substitution)))
            }
            Type::Tuple(types) => Type::Tuple(
                types
                    .iter()
                    .map(|t| self.substitute_type_with_map(t, substitution))
                    .collect(),
            ),
            Type::Reference { mutable, inner } => Type::Reference {
                mutable: *mutable,
                inner: Box::new(self.substitute_type_with_map(inner, substitution)),
            },
            Type::Function { params, ret } => Type::Function {
                params: params
                    .iter()
                    .map(|p| self.substitute_type_with_map(p, substitution))
                    .collect(),
                ret: Box::new(self.substitute_type_with_map(ret, substitution)),
            },
            Type::Generic { name, args } => Type::Generic {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|a| self.substitute_type_with_map(a, substitution))
                    .collect(),
            },
            Type::Option(inner) => {
                Type::Option(Box::new(self.substitute_type_with_map(inner, substitution)))
            }
            Type::Result { ok, err } => Type::Result {
                ok: Box::new(self.substitute_type_with_map(ok, substitution)),
                err: Box::new(self.substitute_type_with_map(err, substitution)),
            },
            Type::Future(inner) => {
                Type::Future(Box::new(self.substitute_type_with_map(inner, substitution)))
            }
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
        let lexer = Lexer::new(source).unwrap();
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
