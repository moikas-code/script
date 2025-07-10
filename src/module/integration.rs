use crate::lexer::Lexer;
use crate::module::{
    ImportPath, ModuleContext, ModuleContextStack, ModuleError, ModuleLoadContext, ModulePath,
    ModuleRegistry, ModuleResolver, ModuleResult, ModuleSandbox, ModuleSecurityContext,
    ModuleSecurityManager, PermissionManager, ResolvedModule, SandboxConfig,
};
use crate::parser::{Parser, Program};
use crate::security::ModuleSecurityEnforcer;
use crate::semantic::{
    EnumVariantType, GenericInstantiation, SemanticAnalyzer, SymbolKind, SymbolTable,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Integration point for module system with the compilation pipeline
pub struct ModuleCompilationPipeline {
    registry: ModuleRegistry,
    resolver: Box<dyn ModuleResolver>,
    semantic_analyzer: SemanticAnalyzer,
    loaded_modules: HashMap<ModulePath, CompiledModule>,
    compilation_order: Vec<ModulePath>,
    /// Security manager for module operations
    security_manager: Arc<Mutex<ModuleSecurityManager>>,
    /// Permission manager for fine-grained access control
    permission_manager: Arc<PermissionManager>,
    /// Module context stack for error reporting
    context_stack: ModuleContextStack,
    /// Security enforcer for runtime checks
    security_enforcer: Arc<Mutex<ModuleSecurityEnforcer>>,
}

/// Represents a compiled module with its artifacts
#[derive(Debug, Clone)]
pub struct CompiledModule {
    pub module: ResolvedModule,
    pub ast: Program,
    pub symbol_table: SymbolTable,
    pub dependencies: Vec<ModulePath>,
    pub exports: ModuleExports,
    pub compilation_time: std::time::Instant,
}

/// Exports from a compiled module with complete type information
#[derive(Debug, Clone)]
pub struct ModuleExports {
    /// Symbol table containing all exported symbols with their complete type info
    pub symbols: SymbolTable,
    /// Type definitions exported from this module (structs, enums, type aliases)
    pub type_definitions: HashMap<String, TypeDefinitionInfo>,
    /// Function exports with complete signatures and generic parameters
    pub functions: HashMap<String, FunctionExportInfo>,
    /// Variable/constant exports with their types and values
    pub variables: HashMap<String, VariableExportInfo>,
    /// Re-exports from other modules
    pub re_exports: HashMap<String, ReExportInfo>,
}

/// Complete information about an exported function
#[derive(Debug, Clone)]
pub struct FunctionExportInfo {
    /// Function name
    pub name: String,
    /// Complete type signature
    pub signature: crate::semantic::FunctionSignature,
    /// Generic parameters if any
    pub generic_params: Option<crate::parser::GenericParams>,
    /// Whether the function is async
    pub is_async: bool,
    /// Documentation if available
    pub documentation: Option<String>,
    /// Visibility level
    pub visibility: ExportVisibility,
}

/// Complete information about an exported variable/constant
#[derive(Debug, Clone)]
pub struct VariableExportInfo {
    /// Variable name
    pub name: String,
    /// Complete type information
    pub type_info: crate::types::Type,
    /// Whether it's mutable
    pub is_mutable: bool,
    /// Initial value representation for constants
    pub initial_value: Option<String>,
    /// Documentation if available  
    pub documentation: Option<String>,
    /// Visibility level
    pub visibility: ExportVisibility,
}

/// Information about exported type definitions
#[derive(Debug, Clone)]
pub struct TypeDefinitionInfo {
    /// Type name
    pub name: String,
    /// Kind of type definition
    pub kind: TypeDefinitionKind,
    /// Generic parameters if any
    pub generic_params: Option<crate::parser::GenericParams>,
    /// Documentation if available
    pub documentation: Option<String>,
    /// Visibility level
    pub visibility: ExportVisibility,
}

/// Different kinds of type definitions
#[derive(Debug, Clone)]
pub enum TypeDefinitionKind {
    /// Struct definition with fields
    Struct { fields: Vec<StructFieldInfo> },
    /// Enum definition with variants
    Enum { variants: Vec<EnumVariantInfo> },
    /// Type alias
    Alias { target_type: crate::types::Type },
}

/// Information about a struct field
#[derive(Debug, Clone)]
pub struct StructFieldInfo {
    pub name: String,
    pub type_info: crate::types::Type,
    pub documentation: Option<String>,
}

/// Information about an enum variant
#[derive(Debug, Clone)]
pub struct EnumVariantInfo {
    pub name: String,
    pub fields: crate::parser::EnumVariantFields,
    pub documentation: Option<String>,
}

/// Information about re-exported items
#[derive(Debug, Clone)]
pub struct ReExportInfo {
    /// Local name for the re-export
    pub local_name: String,
    /// Original module path
    pub source_module: crate::module::ModulePath,
    /// Original name in source module
    pub source_name: String,
    /// Type of the re-exported item
    pub item_type: ReExportType,
}

/// Types of items that can be re-exported
#[derive(Debug, Clone)]
pub enum ReExportType {
    Function,
    Variable,
    Type,
    Module,
}

/// Visibility levels for exports
#[derive(Debug, Clone, PartialEq)]
pub enum ExportVisibility {
    /// Public export (default)
    Public,
    /// Module-private (not typically exported, but for completeness)
    Private,
}

/// Internal representation of import specifications for symbol merging
#[derive(Debug, Clone)]
enum ImportSpecification {
    /// Named import: import { name as alias }
    Named { name: String, alias: Option<String> },
    /// Namespace import: import * as alias
    Namespace { alias: String },
    /// Wildcard import: import *
    Wildcard,
}

/// Cross-module type validator for ensuring type safety across module boundaries
struct CrossModuleTypeValidator<'a> {
    /// Reference to all loaded modules for type lookup
    loaded_modules: &'a HashMap<ModulePath, CompiledModule>,
}

impl<'a> CrossModuleTypeValidator<'a> {
    fn new(loaded_modules: &'a HashMap<ModulePath, CompiledModule>) -> Self {
        Self { loaded_modules }
    }

    /// Validate a statement for cross-module type consistency
    fn validate_statement(
        &mut self,
        stmt: &crate::parser::Stmt,
        current_module: &ModulePath,
    ) -> ModuleResult<()> {
        use crate::parser::StmtKind;

        match &stmt.kind {
            StmtKind::Function {
                params,
                ret_type,
                body,
                ..
            } => {
                // Validate function parameters and return type
                self.validate_function_signature(params, ret_type, current_module)?;
                self.validate_block(body, current_module)?;
            }
            StmtKind::Let { type_ann, init, .. } => {
                // Validate variable type annotation and initialization
                if let Some(type_ann) = type_ann {
                    self.validate_type_annotation(type_ann, current_module)?;
                }
                if let Some(init_expr) = init {
                    self.validate_expression(init_expr, current_module)?;
                }
            }
            StmtKind::Expression(expr) => {
                self.validate_expression(expr, current_module)?;
            }
            StmtKind::Return(Some(expr)) => {
                self.validate_expression(expr, current_module)?;
            }
            StmtKind::While { condition, body } => {
                self.validate_expression(condition, current_module)?;
                self.validate_block(body, current_module)?;
            }
            StmtKind::For { iterable, body, .. } => {
                self.validate_expression(iterable, current_module)?;
                self.validate_block(body, current_module)?;
            }
            _ => {
                // Other statement types don't require cross-module validation
            }
        }

        Ok(())
    }

    /// Validate function signature for cross-module type consistency
    fn validate_function_signature(
        &self,
        params: &[crate::parser::Param],
        ret_type: &Option<crate::parser::TypeAnn>,
        current_module: &ModulePath,
    ) -> ModuleResult<()> {
        // Validate parameter types
        for param in params {
            self.validate_type_annotation(&param.type_ann, current_module)?;
        }

        // Validate return type
        if let Some(ret_type) = ret_type {
            self.validate_type_annotation(ret_type, current_module)?;
        }

        Ok(())
    }

    /// Validate a type annotation for cross-module consistency
    fn validate_type_annotation(
        &self,
        type_ann: &crate::parser::TypeAnn,
        current_module: &ModulePath,
    ) -> ModuleResult<()> {
        use crate::parser::TypeKind;

        match &type_ann.kind {
            TypeKind::Named(name) => {
                // Check if this is a type from another module
                self.validate_type_reference(name, current_module)?;
            }
            TypeKind::Generic { name, args } => {
                // Validate the generic type and its arguments
                self.validate_type_reference(name, current_module)?;
                for arg in args {
                    self.validate_type_annotation(arg, current_module)?;
                }
            }
            TypeKind::Array(elem_type) => {
                self.validate_type_annotation(elem_type, current_module)?;
            }
            TypeKind::Function { params, ret } => {
                for param_type in params {
                    self.validate_type_annotation(param_type, current_module)?;
                }
                self.validate_type_annotation(ret, current_module)?;
            }
            TypeKind::Tuple(types) => {
                for type_ann in types {
                    self.validate_type_annotation(type_ann, current_module)?;
                }
            }
            TypeKind::Reference { inner, .. } => {
                self.validate_type_annotation(inner, current_module)?;
            }
            TypeKind::TypeParam(_) => {
                // Type parameters are valid in generic contexts
            }
        }

        Ok(())
    }

    /// Validate a type reference to ensure it exists and is accessible
    fn validate_type_reference(
        &self,
        type_name: &str,
        current_module: &ModulePath,
    ) -> ModuleResult<()> {
        // Check if it's a built-in type
        if matches!(type_name, "i32" | "f32" | "bool" | "string") {
            return Ok(());
        }

        // Check if the type is available in the current module or its imports
        // This is a simplified check - in a real implementation, we'd need to
        // track import relationships and check accessibility

        // For now, we'll just verify that if it references another module,
        // that module exists and exports the type
        if type_name.contains("::") {
            let parts: Vec<&str> = type_name.splitn(2, "::").collect();
            if parts.len() == 2 {
                let module_name = parts[0];
                let type_name = parts[1];

                // Find the referenced module
                if let Some(referenced_module) = self.find_module_by_name(module_name) {
                    // Check if the module exports this type
                    if !referenced_module
                        .exports
                        .type_definitions
                        .contains_key(type_name)
                    {
                        return Err(ModuleError::import_error(
                            current_module.to_string(),
                            format!(
                                "Type '{}' not exported by module '{}'",
                                type_name, module_name
                            ),
                        ));
                    }
                } else {
                    return Err(ModuleError::import_error(
                        current_module.to_string(),
                        format!("Module '{}' not found", module_name),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Find a module by name in the loaded modules
    fn find_module_by_name(&self, module_name: &str) -> Option<&CompiledModule> {
        self.loaded_modules
            .values()
            .find(|module| module.module.path.module_name() == module_name)
    }

    /// Validate an expression for cross-module type consistency
    fn validate_expression(
        &mut self,
        expr: &crate::parser::Expr,
        current_module: &ModulePath,
    ) -> ModuleResult<()> {
        use crate::parser::ExprKind;

        match &expr.kind {
            ExprKind::Call { callee, args } => {
                self.validate_expression(callee, current_module)?;
                for arg in args {
                    self.validate_expression(arg, current_module)?;
                }
            }
            ExprKind::Member {
                object,
                property: _,
            } => {
                self.validate_expression(object, current_module)?;
                // Could add property access validation here
            }
            ExprKind::Index { object, index } => {
                self.validate_expression(object, current_module)?;
                self.validate_expression(index, current_module)?;
            }
            ExprKind::Binary { left, right, .. } => {
                self.validate_expression(left, current_module)?;
                self.validate_expression(right, current_module)?;
            }
            ExprKind::Unary { expr, .. } => {
                self.validate_expression(expr, current_module)?;
            }
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.validate_expression(condition, current_module)?;
                self.validate_expression(then_branch, current_module)?;
                if let Some(else_expr) = else_branch {
                    self.validate_expression(else_expr, current_module)?;
                }
            }
            ExprKind::Block(block) => {
                self.validate_block(block, current_module)?;
            }
            ExprKind::Array(elements) => {
                for element in elements {
                    self.validate_expression(element, current_module)?;
                }
            }
            ExprKind::Match { expr, arms } => {
                self.validate_expression(expr, current_module)?;
                for arm in arms {
                    self.validate_expression(&arm.body, current_module)?;
                    if let Some(guard) = &arm.guard {
                        self.validate_expression(guard, current_module)?;
                    }
                }
            }
            ExprKind::StructConstructor { name: _, fields } => {
                // Could validate struct type exists and field types match
                for (_, field_expr) in fields {
                    self.validate_expression(field_expr, current_module)?;
                }
            }
            ExprKind::EnumConstructor { args, .. } => match args {
                crate::parser::EnumConstructorArgs::Tuple(exprs) => {
                    for expr in exprs {
                        self.validate_expression(expr, current_module)?;
                    }
                }
                crate::parser::EnumConstructorArgs::Struct(fields) => {
                    for (_, field_expr) in fields {
                        self.validate_expression(field_expr, current_module)?;
                    }
                }
                crate::parser::EnumConstructorArgs::Unit => {}
            },
            _ => {
                // Other expression types don't require cross-module validation
            }
        }

        Ok(())
    }

    /// Validate a block for cross-module type consistency
    fn validate_block(
        &mut self,
        block: &crate::parser::Block,
        current_module: &ModulePath,
    ) -> ModuleResult<()> {
        for stmt in &block.statements {
            self.validate_statement(stmt, current_module)?;
        }

        if let Some(final_expr) = &block.final_expr {
            self.validate_expression(final_expr, current_module)?;
        }

        Ok(())
    }

    /// Enhanced validation with complete type information from semantic analysis
    fn validate_statement_with_type_info(
        &mut self,
        stmt: &crate::parser::Stmt,
        current_module: &ModulePath,
        type_info: &HashMap<usize, crate::types::Type>,
        symbol_table: &SymbolTable,
    ) -> ModuleResult<()> {
        // First, do the basic validation
        self.validate_statement(stmt, current_module)?;

        // Now do enhanced validation using type information
        use crate::parser::StmtKind;

        match &stmt.kind {
            StmtKind::Function {
                name,
                params: _,
                ret_type: _,
                ..
            } => {
                // Validate that function types are consistent across modules
                if let Some(symbol) = symbol_table.lookup(name) {
                    if let SymbolKind::Function(ref signature) = symbol.kind {
                        self.validate_function_cross_module_consistency(
                            name,
                            signature,
                            current_module,
                        )?;
                    }
                }
            }
            StmtKind::Let {
                name, type_ann: _, ..
            } => {
                // Validate variable type consistency
                if let Some(symbol) = symbol_table.lookup(name) {
                    self.validate_variable_cross_module_consistency(
                        name,
                        &symbol.ty,
                        current_module,
                    )?;
                }
            }
            _ => {
                // Other statements handled by basic validation
            }
        }

        Ok(())
    }

    /// Validate generic instantiation across modules
    fn validate_generic_instantiation(
        &self,
        instantiation: &GenericInstantiation,
        current_module: &ModulePath,
        loaded_modules: &HashMap<ModulePath, CompiledModule>,
    ) -> ModuleResult<()> {
        // Check if the generic function is available and properly typed
        for (_module_path, compiled_module) in loaded_modules {
            if let Some(function_export) = compiled_module
                .exports
                .functions
                .get(&instantiation.function_name)
            {
                if let Some(generic_params) = &function_export.generic_params {
                    // Validate that the type arguments are compatible
                    if instantiation.type_args.len() != generic_params.params.len() {
                        return Err(ModuleError::type_error(
                            current_module.to_string(),
                            format!(
                                "Generic function '{}' expects {} type arguments, but {} were provided",
                                instantiation.function_name,
                                generic_params.params.len(),
                                instantiation.type_args.len()
                            )
                        ));
                    }

                    // TODO: Add more sophisticated constraint checking
                    // For now, we just ensure the right number of type arguments
                }
                return Ok(()); // Found the function, validation successful
            }
        }

        // Function not found in any module - this might be an error
        Err(ModuleError::type_error(
            current_module.to_string(),
            format!(
                "Generic function '{}' not found in any imported module",
                instantiation.function_name
            ),
        ))
    }

    /// Validate function consistency across modules
    fn validate_function_cross_module_consistency(
        &self,
        function_name: &str,
        signature: &crate::semantic::FunctionSignature,
        current_module: &ModulePath,
    ) -> ModuleResult<()> {
        // Check if this function exists in other modules with different signatures
        for (module_path, compiled_module) in self.loaded_modules {
            if module_path == current_module {
                continue; // Skip current module
            }

            if let Some(other_function) = compiled_module.exports.functions.get(function_name) {
                // Functions with same name should have compatible signatures for overloading
                if !signature.is_compatible_for_overload(&other_function.signature) {
                    return Err(ModuleError::type_error(
                        current_module.to_string(),
                        format!(
                            "Function '{}' has conflicting signature with definition in module '{}'",
                            function_name,
                            module_path.to_string()
                        )
                    ));
                }
            }
        }

        Ok(())
    }

    /// Validate variable consistency across modules
    fn validate_variable_cross_module_consistency(
        &self,
        variable_name: &str,
        variable_type: &crate::types::Type,
        current_module: &ModulePath,
    ) -> ModuleResult<()> {
        // Check if this variable exists in other modules with different types
        for (module_path, compiled_module) in self.loaded_modules {
            if module_path == current_module {
                continue; // Skip current module
            }

            if let Some(other_variable) = compiled_module.exports.variables.get(variable_name) {
                // Variables with same name should have the same type
                if !variable_type.equals(&other_variable.type_info) {
                    return Err(ModuleError::type_error(
                        current_module.to_string(),
                        format!(
                            "Variable '{}' has different type '{}' than in module '{}' where it has type '{}'",
                            variable_name,
                            variable_type.to_string(),
                            module_path.to_string(),
                            other_variable.type_info.to_string()
                        )
                    ));
                }
            }
        }

        Ok(())
    }
}

/// Module compilation configuration
#[derive(Debug, Clone)]
pub struct CompilationConfig {
    pub enable_caching: bool,
    pub incremental_compilation: bool,
    pub parallel_compilation: bool,
    pub max_parallel_jobs: usize,
    pub dependency_validation: bool,
    pub circular_dependency_detection: bool,
}

impl ModuleCompilationPipeline {
    pub fn new(
        registry: ModuleRegistry,
        resolver: Box<dyn ModuleResolver>,
        semantic_analyzer: SemanticAnalyzer,
    ) -> Self {
        let security_manager = Arc::new(Mutex::new(ModuleSecurityManager::new()));
        let permission_manager = Arc::new(PermissionManager::new());
        let security_enforcer = Arc::new(Mutex::new(ModuleSecurityEnforcer::new(
            crate::security::SecurityPolicy::default(),
        )));

        Self {
            registry,
            resolver,
            semantic_analyzer,
            loaded_modules: HashMap::new(),
            compilation_order: Vec::new(),
            security_manager,
            permission_manager,
            context_stack: ModuleContextStack::new(),
            security_enforcer,
        }
    }

    /// Compile a module and all its dependencies
    pub fn compile_module(
        &mut self,
        module_path: &ModulePath,
        context: &ModuleLoadContext,
        config: &CompilationConfig,
    ) -> ModuleResult<()> {
        // Check if already compiled
        if self.loaded_modules.contains_key(module_path) {
            return Ok(());
        }

        // Build dependency graph and determine compilation order
        let compilation_order = self.build_compilation_order(module_path, context, config)?;

        // Compile dependencies first
        for dep_path in &compilation_order {
            if dep_path != module_path && !self.loaded_modules.contains_key(dep_path) {
                self.compile_single_module(dep_path, context, config)?;
            }
        }

        // Compile the target module
        self.compile_single_module(module_path, context, config)?;

        Ok(())
    }

    /// Compile a single module without dependencies
    fn compile_single_module(
        &mut self,
        module_path: &ModulePath,
        context: &ModuleLoadContext,
        config: &CompilationConfig,
    ) -> ModuleResult<()> {
        let compilation_start = std::time::Instant::now();

        // Create module context for enhanced error reporting
        let module_context = ModuleContext::new(module_path.clone());
        self.context_stack.push(module_context);

        // Get or create security context for the module
        let security_context = {
            let mut security_manager = self.security_manager.lock().unwrap();
            security_manager.get_or_create_context(module_path).clone()
        };

        // Register module with security enforcer
        {
            let mut enforcer = self.security_enforcer.lock().unwrap();
            enforcer.register_module(security_context.clone());
        }

        // Resolve the module
        let import_path = ImportPath::new(module_path.to_string())?;
        let resolved_module = self.resolver.resolve_module(&import_path, context)?;

        // Record resolution in context
        if let Some(ctx) = self.context_stack.current_mut() {
            ctx.source_files
                .insert(module_path.clone(), resolved_module.file_path.clone());
        }

        // Check registry cache if enabled
        if config.enable_caching && self.registry.is_registered(&resolved_module.path) {
            if let Some(cached_module) = self.registry.get_module(&resolved_module.path) {
                // Create compiled module from cache
                let compiled = self.create_compiled_from_cache(cached_module)?;
                self.loaded_modules.insert(module_path.clone(), compiled);
                self.context_stack.pop();
                return Ok(());
            }
        }

        // Parse the module
        let ast = self.parse_module(&resolved_module)?;

        // Create module scope with imports
        let module_scope = self.create_module_scope(&resolved_module, &ast)?;

        // Import symbols into the semantic analyzer before analysis
        // Also propagate type information from dependencies
        self.import_symbols_with_type_propagation(&module_scope, &resolved_module)
            .map_err(|e| ModuleError::parse_error(module_path.to_string(), e.to_string()))?;

        // Perform semantic analysis with imported symbols available
        self.semantic_analyzer
            .analyze_program(&ast)
            .map_err(|e| ModuleError::parse_error(module_path.to_string(), e.to_string()))?;

        // Perform cross-module type validation
        self.validate_cross_module_types(&resolved_module, &ast)?;

        // Get the analyzer's symbol table after analysis (includes both imports and local definitions)
        let final_symbol_table = self.semantic_analyzer.symbol_table();

        // Extract exports using the complete symbol table and type information from semantic analysis
        let type_info = self.semantic_analyzer.type_info();
        let generic_instantiations = self.semantic_analyzer.generic_instantiations();
        let exports = self.extract_module_exports(
            &ast,
            final_symbol_table,
            type_info,
            generic_instantiations,
        )?;

        // Create compiled module
        let compiled_module = CompiledModule {
            module: resolved_module.clone(),
            ast,
            symbol_table: final_symbol_table.clone(),
            dependencies: self.extract_dependencies(&resolved_module),
            exports,
            compilation_time: compilation_start,
        };

        // Register in registry
        self.registry.register_module(resolved_module)?;

        // Store compiled module
        self.loaded_modules
            .insert(module_path.clone(), compiled_module);

        // Pop the module context
        self.context_stack.pop();

        Ok(())
    }

    /// Build the compilation order for a module and its dependencies
    fn build_compilation_order(
        &mut self,
        root_module: &ModulePath,
        context: &ModuleLoadContext,
        config: &CompilationConfig,
    ) -> ModuleResult<Vec<ModulePath>> {
        let mut order = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut visiting = std::collections::HashSet::new();

        self.visit_for_compilation_order(
            root_module,
            context,
            config,
            &mut order,
            &mut visited,
            &mut visiting,
        )?;

        Ok(order)
    }

    fn visit_for_compilation_order(
        &mut self,
        module_path: &ModulePath,
        context: &ModuleLoadContext,
        config: &CompilationConfig,
        order: &mut Vec<ModulePath>,
        visited: &mut std::collections::HashSet<ModulePath>,
        visiting: &mut std::collections::HashSet<ModulePath>,
    ) -> ModuleResult<()> {
        if visited.contains(module_path) {
            return Ok(());
        }

        if visiting.contains(module_path) {
            if config.circular_dependency_detection {
                return Err(ModuleError::circular_dependency(&[], module_path));
            } else {
                return Ok(()); // Allow circular dependencies if not configured to detect
            }
        }

        visiting.insert(module_path.clone());

        // Get module dependencies
        let import_path = ImportPath::new(module_path.to_string())?;
        let resolved_module = self.resolver.resolve_module(&import_path, context)?;

        for dep_import in &resolved_module.dependencies {
            let dep_module_path = dep_import.resolve(&context.current_module)?;
            self.visit_for_compilation_order(
                &dep_module_path,
                context,
                config,
                order,
                visited,
                visiting,
            )?;
        }

        visiting.remove(module_path);
        visited.insert(module_path.clone());
        order.push(module_path.clone());

        Ok(())
    }

    fn parse_module(&self, module: &ResolvedModule) -> ModuleResult<Program> {
        let lexer = Lexer::new(&module.source)
            .map_err(|e| ModuleError::parse_error(module.path.to_string(), e.to_string()))?;
        let (tokens, errors) = lexer.scan_tokens();

        if !errors.is_empty() {
            return Err(ModuleError::parse_error(
                module.path.to_string(),
                errors
                    .into_iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
            ));
        }

        let mut parser = Parser::new(tokens);
        parser
            .parse()
            .map_err(|e| ModuleError::parse_error(module.path.to_string(), e.to_string()))
    }

    fn create_module_scope(
        &self,
        module: &ResolvedModule,
        _ast: &Program,
    ) -> ModuleResult<SymbolTable> {
        // Create a new symbol table for this module
        let mut symbol_table = SymbolTable::new();

        // Add imported symbols
        for import in &module.dependencies {
            self.add_imported_symbols(&mut symbol_table, import)?;
        }

        Ok(symbol_table)
    }

    fn add_imported_symbols(
        &self,
        symbol_table: &mut SymbolTable,
        import_path: &ImportPath,
    ) -> ModuleResult<()> {
        // Resolve the import path to get the source module
        let source_module_path = import_path.resolve(&self.get_current_module_path())?;

        // Get the compiled module containing the exports
        if let Some(compiled_module) = self.loaded_modules.get(&source_module_path) {
            self.merge_exported_symbols(symbol_table, &compiled_module.exports, import_path)?;
        } else {
            // Module not yet compiled - this is expected during dependency resolution
            // We'll handle this through proper dependency ordering
        }

        Ok(())
    }

    fn get_current_module_path(&self) -> ModulePath {
        // In a real implementation, this would track the current module being compiled
        // For now, create a dummy path
        ModulePath::from_string("current").unwrap_or_else(|_| {
            // Fallback if path creation fails
            ModulePath::new(vec!["current".to_string()], false).unwrap()
        })
    }

    fn merge_exported_symbols(
        &self,
        symbol_table: &mut SymbolTable,
        exports: &ModuleExports,
        import_path: &ImportPath,
    ) -> ModuleResult<()> {
        // Extract import specifications to determine what to import
        let import_specs = self.parse_import_specifications(import_path)?;

        for import_spec in import_specs {
            match import_spec {
                ImportSpecification::Named { name, alias } => {
                    let symbol_name = alias.unwrap_or(name.clone());
                    self.import_named_symbol(symbol_table, exports, &name, &symbol_name)?;
                }
                ImportSpecification::Namespace { alias } => {
                    self.import_namespace(symbol_table, exports, &alias)?;
                }
                ImportSpecification::Wildcard => {
                    self.import_all_symbols(symbol_table, exports)?;
                }
            }
        }

        Ok(())
    }

    fn parse_import_specifications(
        &self,
        _import_path: &ImportPath,
    ) -> ModuleResult<Vec<ImportSpecification>> {
        // In a complete implementation, this would parse the import statement
        // from the source code to determine what's being imported
        // For now, assume wildcard import
        Ok(vec![ImportSpecification::Wildcard])
    }

    fn import_named_symbol(
        &self,
        symbol_table: &mut SymbolTable,
        exports: &ModuleExports,
        source_name: &str,
        local_name: &str,
    ) -> ModuleResult<()> {
        // Try to find the symbol in different export categories
        if let Some(function_info) = exports.functions.get(source_name) {
            self.add_function_symbol(symbol_table, local_name, function_info)?;
        } else if let Some(variable_info) = exports.variables.get(source_name) {
            self.add_variable_symbol(symbol_table, local_name, variable_info)?;
        } else if let Some(type_info) = exports.type_definitions.get(source_name) {
            self.add_type_symbol(symbol_table, local_name, type_info)?;
        } else {
            return Err(ModuleError::import_error(
                "current",
                format!("Symbol '{}' not found in exports", source_name),
            ));
        }

        Ok(())
    }

    fn import_namespace(
        &self,
        symbol_table: &mut SymbolTable,
        exports: &ModuleExports,
        namespace_name: &str,
    ) -> ModuleResult<()> {
        // Create a namespace symbol that contains all exports
        // This would typically create a special namespace symbol type
        // For now, we'll prefix all imports with the namespace name

        for (name, function_info) in &exports.functions {
            let qualified_name = format!("{}::{}", namespace_name, name);
            self.add_function_symbol(symbol_table, &qualified_name, function_info)?;
        }

        for (name, variable_info) in &exports.variables {
            let qualified_name = format!("{}::{}", namespace_name, name);
            self.add_variable_symbol(symbol_table, &qualified_name, variable_info)?;
        }

        for (name, type_info) in &exports.type_definitions {
            let qualified_name = format!("{}::{}", namespace_name, name);
            self.add_type_symbol(symbol_table, &qualified_name, type_info)?;
        }

        Ok(())
    }

    fn import_all_symbols(
        &self,
        symbol_table: &mut SymbolTable,
        exports: &ModuleExports,
    ) -> ModuleResult<()> {
        // Import all exported symbols directly into the current namespace

        for (name, function_info) in &exports.functions {
            self.add_function_symbol(symbol_table, name, function_info)?;
        }

        for (name, variable_info) in &exports.variables {
            self.add_variable_symbol(symbol_table, name, variable_info)?;
        }

        for (name, type_info) in &exports.type_definitions {
            self.add_type_symbol(symbol_table, name, type_info)?;
        }

        Ok(())
    }

    fn add_function_symbol(
        &self,
        symbol_table: &mut SymbolTable,
        name: &str,
        function_info: &FunctionExportInfo,
    ) -> ModuleResult<()> {
        use crate::source::{SourceLocation, Span};

        // Create a dummy span for imported symbols
        let dummy_span = Span::new(SourceLocation::new(0, 0, 0), SourceLocation::new(0, 0, 0));

        // Use the symbol table's define_function method
        symbol_table
            .define_function(
                name.to_string(),
                function_info.signature.clone(),
                dummy_span,
            )
            .map_err(|e| ModuleError::import_error("current", e))?;

        Ok(())
    }

    fn add_variable_symbol(
        &self,
        symbol_table: &mut SymbolTable,
        name: &str,
        variable_info: &VariableExportInfo,
    ) -> ModuleResult<()> {
        use crate::source::{SourceLocation, Span};

        // Create a dummy span for imported symbols
        let dummy_span = Span::new(SourceLocation::new(0, 0, 0), SourceLocation::new(0, 0, 0));

        // Use the symbol table's define_variable method
        symbol_table
            .define_variable(
                name.to_string(),
                variable_info.type_info.clone(),
                dummy_span,
                variable_info.is_mutable,
            )
            .map_err(|e| ModuleError::import_error("current", e))?;

        Ok(())
    }

    fn add_type_symbol(
        &self,
        symbol_table: &mut SymbolTable,
        name: &str,
        type_info: &TypeDefinitionInfo,
    ) -> ModuleResult<()> {
        use crate::source::{SourceLocation, Span};

        // Create a dummy span for imported symbols
        let dummy_span = Span::new(SourceLocation::new(0, 0, 0), SourceLocation::new(0, 0, 0));

        match &type_info.kind {
            TypeDefinitionKind::Struct { fields } => {
                let struct_fields: Vec<(String, crate::types::Type)> = fields
                    .iter()
                    .map(|field| (field.name.clone(), field.type_info.clone()))
                    .collect();

                let struct_info = crate::semantic::StructInfo {
                    generic_params: type_info.generic_params.clone(),
                    fields: struct_fields,
                    where_clause: None, // Would need to be preserved from original
                };

                symbol_table
                    .define_struct(name.to_string(), struct_info, dummy_span)
                    .map_err(|e| ModuleError::import_error("current", e))?;
            }
            TypeDefinitionKind::Enum { variants } => {
                let enum_variants: Vec<crate::semantic::EnumVariantInfo> = variants
                    .iter()
                    .map(|variant| crate::semantic::EnumVariantInfo {
                        name: variant.name.clone(),
                        variant_type: self.convert_enum_variant_type(&variant.fields),
                    })
                    .collect();

                let enum_info = crate::semantic::EnumInfo {
                    generic_params: type_info.generic_params.clone(),
                    variants: enum_variants,
                    where_clause: None, // Would need to be preserved from original
                };

                symbol_table
                    .define_enum(name.to_string(), enum_info, dummy_span)
                    .map_err(|e| ModuleError::import_error("current", e))?;
            }
            TypeDefinitionKind::Alias { target_type: _ } => {
                // Type aliases would need special handling in the symbol table
                // For now, we'll skip them as they need dedicated API support
                return Err(ModuleError::config_error(
                    "Type alias imports not yet supported",
                ));
            }
        };

        Ok(())
    }

    /// Import symbols with proper type propagation from dependencies
    fn import_symbols_with_type_propagation(
        &mut self,
        module_scope: &SymbolTable,
        resolved_module: &ResolvedModule,
    ) -> ModuleResult<()> {
        // First, do the basic symbol import
        self.semantic_analyzer
            .import_symbols_from(module_scope)
            .map_err(|e| {
                ModuleError::parse_error(resolved_module.path.to_string(), e.to_string())
            })?;

        // Now propagate type information from dependencies
        for dependency in &resolved_module.dependencies {
            self.propagate_dependency_type_info(dependency)?;
        }

        Ok(())
    }

    /// Propagate type information from a dependency to the current module
    fn propagate_dependency_type_info(&mut self, dependency: &ImportPath) -> ModuleResult<()> {
        // Resolve the dependency path to get the compiled module
        let dependency_module_path = dependency.resolve(&self.get_current_module_path())?;

        if let Some(compiled_dependency) = self.loaded_modules.get(&dependency_module_path) {
            // Get the dependency's type information
            let dependency_type_info = compiled_dependency.symbol_table.clone();

            // Register the dependency module with its complete type information
            self.semantic_analyzer
                .register_module(&dependency_module_path.to_string(), &dependency_type_info);

            // Import symbols with complete type information
            self.semantic_analyzer
                .import_public_symbols(
                    &dependency_type_info,
                    Some(&dependency_module_path.to_string()),
                )
                .map_err(|e| {
                    ModuleError::parse_error(dependency_module_path.to_string(), e.to_string())
                })?;

            // Also propagate generic instantiations
            // This ensures that generic types from dependencies are available
            for (name, export_info) in &compiled_dependency.exports.functions {
                if let Some(generic_params) = &export_info.generic_params {
                    // Register generic function for potential instantiation
                    self.semantic_analyzer
                        .register_generic_function(name, generic_params.clone());
                }
            }
        }

        Ok(())
    }

    fn convert_enum_variant_type(
        &self,
        fields: &crate::parser::EnumVariantFields,
    ) -> crate::semantic::EnumVariantType {
        use crate::semantic::EnumVariantType;

        match fields {
            crate::parser::EnumVariantFields::Unit => EnumVariantType::Unit,
            crate::parser::EnumVariantFields::Tuple(types) => {
                let tuple_types: Vec<crate::types::Type> = types
                    .iter()
                    .map(|type_ann| {
                        self.type_ann_to_type(type_ann)
                            .unwrap_or(crate::types::Type::Unknown)
                    })
                    .collect();
                EnumVariantType::Tuple(tuple_types)
            }
            crate::parser::EnumVariantFields::Struct(fields) => {
                let struct_fields: Vec<(String, crate::types::Type)> = fields
                    .iter()
                    .map(|field| {
                        let field_type = self
                            .type_ann_to_type(&field.type_ann)
                            .unwrap_or(crate::types::Type::Unknown);
                        (field.name.clone(), field_type)
                    })
                    .collect();
                EnumVariantType::Struct(struct_fields)
            }
        }
    }

    fn extract_module_exports(
        &self,
        ast: &Program,
        symbol_table: &SymbolTable,
        type_info: &HashMap<usize, crate::types::Type>,
        generic_instantiations: &[GenericInstantiation],
    ) -> ModuleResult<ModuleExports> {
        let mut exports = ModuleExports {
            symbols: symbol_table.clone(),
            type_definitions: HashMap::new(),
            functions: HashMap::new(),
            variables: HashMap::new(),
            re_exports: HashMap::new(),
        };

        // Extract exports from AST statements with type information
        for stmt in &ast.statements {
            self.extract_exports_from_statement(
                stmt,
                &mut exports,
                type_info,
                generic_instantiations,
            )?;
        }

        // Enhance exports with complete type information from symbol table
        self.enhance_exports_with_type_info(&mut exports, symbol_table, type_info)?;

        Ok(exports)
    }

    fn extract_exports_from_statement(
        &self,
        stmt: &crate::parser::Stmt,
        exports: &mut ModuleExports,
        type_info: &HashMap<usize, crate::types::Type>,
        generic_instantiations: &[GenericInstantiation],
    ) -> ModuleResult<()> {
        use crate::parser::StmtKind;

        match &stmt.kind {
            StmtKind::Export { export } => {
                self.process_export_statement(export, exports, type_info, generic_instantiations)?;
            }
            StmtKind::Function {
                name,
                generic_params,
                params,
                ret_type,
                is_async,
                ..
            } => {
                // Check if this function should be exported (based on visibility or export statements)
                if self.should_export_symbol(name) {
                    let mut signature = self.create_function_signature(params, ret_type)?;
                    signature.generic_params = generic_params.clone();
                    signature.is_async = *is_async;

                    let function_info = FunctionExportInfo {
                        name: name.clone(),
                        signature,
                        generic_params: generic_params.clone(),
                        is_async: *is_async,
                        documentation: self.extract_documentation(&stmt.attributes),
                        visibility: ExportVisibility::Public,
                    };
                    exports.functions.insert(name.clone(), function_info);
                }
            }
            StmtKind::Struct {
                name,
                generic_params,
                fields,
                ..
            } => {
                if self.should_export_symbol(name) {
                    let struct_fields = self.convert_struct_fields(fields)?;
                    let type_def = TypeDefinitionInfo {
                        name: name.clone(),
                        kind: TypeDefinitionKind::Struct {
                            fields: struct_fields,
                        },
                        generic_params: generic_params.clone(),
                        documentation: self.extract_documentation(&stmt.attributes),
                        visibility: ExportVisibility::Public,
                    };
                    exports.type_definitions.insert(name.clone(), type_def);
                }
            }
            StmtKind::Enum {
                name,
                generic_params,
                variants,
                ..
            } => {
                if self.should_export_symbol(name) {
                    let enum_variants = self.convert_enum_variants(variants)?;
                    let type_def = TypeDefinitionInfo {
                        name: name.clone(),
                        kind: TypeDefinitionKind::Enum {
                            variants: enum_variants,
                        },
                        generic_params: generic_params.clone(),
                        documentation: self.extract_documentation(&stmt.attributes),
                        visibility: ExportVisibility::Public,
                    };
                    exports.type_definitions.insert(name.clone(), type_def);
                }
            }
            StmtKind::Let {
                name,
                type_ann,
                init,
            } => {
                if self.should_export_symbol(name) {
                    let type_info = if let Some(type_ann) = type_ann {
                        self.type_ann_to_type(type_ann)?
                    } else {
                        crate::types::Type::Unknown // Type inference would determine this
                    };

                    let variable_info = VariableExportInfo {
                        name: name.clone(),
                        type_info,
                        is_mutable: false, // Let bindings are immutable by default
                        initial_value: init.as_ref().map(|_| "expression".to_string()), // Would serialize actual value
                        documentation: self.extract_documentation(&stmt.attributes),
                        visibility: ExportVisibility::Public,
                    };
                    exports.variables.insert(name.clone(), variable_info);
                }
            }
            _ => {
                // Other statement types don't contribute to exports directly
            }
        }

        Ok(())
    }

    fn process_export_statement(
        &self,
        export: &crate::parser::ExportKind,
        exports: &mut ModuleExports,
        type_info: &HashMap<usize, crate::types::Type>,
        generic_instantiations: &[GenericInstantiation],
    ) -> ModuleResult<()> {
        use crate::parser::ExportKind;

        match export {
            ExportKind::Named { specifiers } => {
                for spec in specifiers {
                    // Process named exports - these reference already defined symbols
                    self.process_named_export(spec, exports)?;
                }
            }
            ExportKind::Function {
                name,
                params,
                ret_type,
                is_async,
                ..
            } => {
                let mut signature = self.create_function_signature(params, ret_type)?;
                signature.is_async = *is_async;

                let function_info = FunctionExportInfo {
                    name: name.clone(),
                    signature,
                    generic_params: None, // Export statements don't include generic params
                    is_async: *is_async,
                    documentation: None,
                    visibility: ExportVisibility::Public,
                };
                exports.functions.insert(name.clone(), function_info);
            }
            ExportKind::Variable {
                name,
                type_ann,
                init,
            } => {
                let type_info = if let Some(type_ann) = type_ann {
                    self.type_ann_to_type(type_ann)?
                } else {
                    crate::types::Type::Unknown
                };

                let variable_info = VariableExportInfo {
                    name: name.clone(),
                    type_info,
                    is_mutable: false,
                    initial_value: init.as_ref().map(|_| "expression".to_string()),
                    documentation: None,
                    visibility: ExportVisibility::Public,
                };
                exports.variables.insert(name.clone(), variable_info);
            }
            ExportKind::Default { .. } => {
                // Handle default exports - these are special
                // For now, we'll add them as a special case
            }
            ExportKind::Declaration(stmt) => {
                // Process the declaration and mark it as exported
                self.extract_exports_from_statement(
                    stmt,
                    exports,
                    type_info,
                    generic_instantiations,
                )?;
            }
        }

        Ok(())
    }

    fn process_named_export(
        &self,
        spec: &crate::parser::ExportSpecifier,
        _exports: &mut ModuleExports,
    ) -> ModuleResult<()> {
        // In a complete implementation, this would:
        // 1. Look up the symbol in the current symbol table
        // 2. Determine its type (function, variable, type, etc.)
        // 3. Add it to the appropriate export collection
        // 4. Handle aliasing if spec.alias is Some

        // For now, this is a placeholder
        let _export_name = spec.alias.as_ref().unwrap_or(&spec.name);
        Ok(())
    }

    fn should_export_symbol(&self, _symbol_name: &str) -> bool {
        // In a complete implementation, this would check:
        // 1. Whether the symbol is marked for export
        // 2. Visibility rules (pub, pub(crate), etc.)
        // 3. Export statements that reference this symbol

        // For now, assume all symbols are potentially exportable
        true
    }

    fn create_function_signature(
        &self,
        params: &[crate::parser::Param],
        ret_type: &Option<crate::parser::TypeAnn>,
    ) -> ModuleResult<crate::semantic::FunctionSignature> {
        let param_info: Result<Vec<_>, _> = params
            .iter()
            .map(|param| {
                let param_type = self.type_ann_to_type(&param.type_ann)?;
                Ok::<(String, crate::types::Type), ModuleError>((param.name.clone(), param_type))
            })
            .collect();

        let return_type = if let Some(ret_type) = ret_type {
            self.type_ann_to_type(ret_type)?
        } else {
            crate::types::Type::Unknown
        };

        Ok(crate::semantic::FunctionSignature {
            generic_params: None, // Would be passed from caller
            params: param_info?,
            return_type,
            is_const: false,
            is_async: false, // Would be passed from caller
        })
    }

    fn convert_struct_fields(
        &self,
        fields: &[crate::parser::StructField],
    ) -> ModuleResult<Vec<StructFieldInfo>> {
        fields
            .iter()
            .map(|field| {
                Ok(StructFieldInfo {
                    name: field.name.clone(),
                    type_info: self.type_ann_to_type(&field.type_ann)?,
                    documentation: None, // Would extract from attributes
                })
            })
            .collect()
    }

    fn convert_enum_variants(
        &self,
        variants: &[crate::parser::EnumVariant],
    ) -> ModuleResult<Vec<EnumVariantInfo>> {
        Ok(variants
            .iter()
            .map(|variant| EnumVariantInfo {
                name: variant.name.clone(),
                fields: variant.fields.clone(),
                documentation: None, // Would extract from attributes
            })
            .collect())
    }

    fn extract_documentation(&self, _attributes: &[crate::parser::Attribute]) -> Option<String> {
        // In a complete implementation, this would extract doc comments
        // from attributes like #[doc = "..."]
        None
    }

    fn type_ann_to_type(
        &self,
        type_ann: &crate::parser::TypeAnn,
    ) -> ModuleResult<crate::types::Type> {
        use crate::parser::TypeKind;

        let result = match &type_ann.kind {
            TypeKind::Named(name) => match name.as_str() {
                "i32" => crate::types::Type::I32,
                "f32" => crate::types::Type::F32,
                "bool" => crate::types::Type::Bool,
                "string" => crate::types::Type::String,
                _ => crate::types::Type::Named(name.clone()),
            },
            TypeKind::Array(elem) => {
                let elem_type = self.type_ann_to_type(elem)?;
                crate::types::Type::Array(Box::new(elem_type))
            }
            TypeKind::Function { params, ret } => {
                let param_types: Result<Vec<_>, _> =
                    params.iter().map(|p| self.type_ann_to_type(p)).collect();
                let return_type = self.type_ann_to_type(ret)?;
                crate::types::Type::Function {
                    params: param_types?,
                    ret: Box::new(return_type),
                }
            }
            TypeKind::Generic { name, args } => {
                let arg_types: Result<Vec<_>, _> =
                    args.iter().map(|arg| self.type_ann_to_type(arg)).collect();
                crate::types::Type::Generic {
                    name: name.clone(),
                    args: arg_types?,
                }
            }
            TypeKind::TypeParam(name) => crate::types::Type::TypeParam(name.clone()),
            TypeKind::Tuple(types) => {
                let tuple_types: Result<Vec<_>, _> =
                    types.iter().map(|t| self.type_ann_to_type(t)).collect();
                crate::types::Type::Tuple(tuple_types?)
            }
            TypeKind::Reference { mutable, inner } => {
                let inner_type = self.type_ann_to_type(inner)?;
                crate::types::Type::Reference {
                    mutable: *mutable,
                    inner: Box::new(inner_type),
                }
            }
        };

        Ok(result)
    }

    /// Validate cross-module type consistency
    fn validate_cross_module_types(
        &self,
        module: &ResolvedModule,
        ast: &Program,
    ) -> ModuleResult<()> {
        // Create a cross-module type validator with access to semantic analysis
        let mut validator = CrossModuleTypeValidator::new(&self.loaded_modules);

        // Get the current module's type information from the semantic analyzer
        let module_type_info = self.semantic_analyzer.type_info();
        let module_symbol_table = self.semantic_analyzer.symbol_table();

        // Validate all statements in the AST for cross-module type consistency
        for stmt in &ast.statements {
            validator.validate_statement_with_type_info(
                stmt,
                &module.path,
                module_type_info,
                module_symbol_table,
            )?;
        }

        // Validate cross-module generic instantiations
        let generic_instantiations = self.semantic_analyzer.generic_instantiations();
        for instantiation in generic_instantiations {
            validator.validate_generic_instantiation(
                instantiation,
                &module.path,
                &self.loaded_modules,
            )?;
        }

        Ok(())
    }

    /// Enhance exports with complete type information from semantic analysis
    fn enhance_exports_with_type_info(
        &self,
        exports: &mut ModuleExports,
        symbol_table: &SymbolTable,
        type_info: &HashMap<usize, crate::types::Type>,
    ) -> ModuleResult<()> {
        // Update function exports with resolved types from symbol table
        for (name, func_export) in exports.functions.iter_mut() {
            if let Some(symbol) = symbol_table.lookup(name) {
                if let SymbolKind::Function(ref sig) = symbol.kind {
                    // Update signature with fully resolved types
                    func_export.signature = sig.clone();
                }
            }
        }

        // Update variable exports with resolved types
        for (name, var_export) in exports.variables.iter_mut() {
            if let Some(symbol) = symbol_table.lookup(name) {
                if matches!(symbol.kind, SymbolKind::Variable) {
                    var_export.type_info = symbol.ty.clone();
                    // TODO: Track mutability in symbol table
                    var_export.is_mutable = false;
                }
            }
        }

        // Extract and add type definitions from symbol table
        for (name, symbol) in symbol_table.all_symbols() {
            match &symbol.kind {
                SymbolKind::Struct(struct_info) => {
                    if self.should_export_symbol(name) {
                        let fields = struct_info
                            .fields
                            .iter()
                            .map(|(field_name, field_type)| {
                                StructFieldInfo {
                                    name: field_name.clone(),
                                    type_info: field_type.clone(),
                                    documentation: None, // TODO: Extract from AST attributes
                                }
                            })
                            .collect();

                        let type_def = TypeDefinitionInfo {
                            name: name.clone(),
                            kind: TypeDefinitionKind::Struct { fields },
                            generic_params: struct_info.generic_params.clone(),
                            documentation: None,
                            visibility: ExportVisibility::Public,
                        };
                        exports.type_definitions.insert(name.clone(), type_def);
                    }
                }
                SymbolKind::Enum(enum_info) => {
                    if self.should_export_symbol(name) {
                        let variants = enum_info
                            .variants
                            .iter()
                            .map(|variant| {
                                let fields = match &variant.variant_type {
                                    EnumVariantType::Unit => crate::parser::EnumVariantFields::Unit,
                                    EnumVariantType::Tuple(types) => {
                                        crate::parser::EnumVariantFields::Tuple(
                                            types
                                                .iter()
                                                .map(|t| crate::parser::TypeAnn {
                                                    kind: self.type_to_type_ann_kind(t),
                                                    span: crate::source::Span::dummy(),
                                                })
                                                .collect(),
                                        )
                                    }
                                    EnumVariantType::Struct(fields) => {
                                        let struct_fields = fields
                                            .iter()
                                            .map(|(name, ty)| crate::parser::StructField {
                                                name: name.clone(),
                                                type_ann: crate::parser::TypeAnn {
                                                    kind: self.type_to_type_ann_kind(ty),
                                                    span: crate::source::Span::dummy(),
                                                },
                                                span: crate::source::Span::dummy(),
                                            })
                                            .collect();
                                        crate::parser::EnumVariantFields::Struct(struct_fields)
                                    }
                                };

                                EnumVariantInfo {
                                    name: variant.name.clone(),
                                    fields,
                                    documentation: None,
                                }
                            })
                            .collect();

                        let type_def = TypeDefinitionInfo {
                            name: name.clone(),
                            kind: TypeDefinitionKind::Enum { variants },
                            generic_params: enum_info.generic_params.clone(),
                            documentation: None,
                            visibility: ExportVisibility::Public,
                        };
                        exports.type_definitions.insert(name.clone(), type_def);
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Convert a Type to TypeAnn kind for export representation
    fn type_to_type_ann_kind(&self, ty: &crate::types::Type) -> crate::parser::TypeKind {
        match ty {
            crate::types::Type::I32 => crate::parser::TypeKind::Named("i32".to_string()),
            crate::types::Type::F32 => crate::parser::TypeKind::Named("f32".to_string()),
            crate::types::Type::Bool => crate::parser::TypeKind::Named("bool".to_string()),
            crate::types::Type::String => crate::parser::TypeKind::Named("string".to_string()),
            crate::types::Type::Unknown => crate::parser::TypeKind::Named("unknown".to_string()),
            crate::types::Type::Never => crate::parser::TypeKind::Named("never".to_string()),
            crate::types::Type::Named(name) => crate::parser::TypeKind::Named(name.clone()),
            crate::types::Type::TypeParam(name) => crate::parser::TypeKind::TypeParam(name.clone()),
            crate::types::Type::Generic { name, args } => crate::parser::TypeKind::Generic {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|t| crate::parser::TypeAnn {
                        kind: self.type_to_type_ann_kind(t),
                        span: crate::source::Span::dummy(),
                    })
                    .collect(),
            },
            crate::types::Type::Array(elem) => {
                crate::parser::TypeKind::Array(Box::new(crate::parser::TypeAnn {
                    kind: self.type_to_type_ann_kind(elem),
                    span: crate::source::Span::dummy(),
                }))
            }
            crate::types::Type::Tuple(types) => crate::parser::TypeKind::Tuple(
                types
                    .iter()
                    .map(|t| crate::parser::TypeAnn {
                        kind: self.type_to_type_ann_kind(t),
                        span: crate::source::Span::dummy(),
                    })
                    .collect(),
            ),
            crate::types::Type::Reference { mutable, inner } => {
                crate::parser::TypeKind::Reference {
                    mutable: *mutable,
                    inner: Box::new(crate::parser::TypeAnn {
                        kind: self.type_to_type_ann_kind(inner),
                        span: crate::source::Span::dummy(),
                    }),
                }
            }
            crate::types::Type::Function { params, ret } => crate::parser::TypeKind::Function {
                params: params
                    .iter()
                    .map(|t| crate::parser::TypeAnn {
                        kind: self.type_to_type_ann_kind(t),
                        span: crate::source::Span::dummy(),
                    })
                    .collect(),
                ret: Box::new(crate::parser::TypeAnn {
                    kind: self.type_to_type_ann_kind(ret),
                    span: crate::source::Span::dummy(),
                }),
            },
            crate::types::Type::Option(inner) => crate::parser::TypeKind::Generic {
                name: "Option".to_string(),
                args: vec![crate::parser::TypeAnn {
                    kind: self.type_to_type_ann_kind(inner),
                    span: crate::source::Span::dummy(),
                }],
            },
            crate::types::Type::Result { ok, err } => crate::parser::TypeKind::Generic {
                name: "Result".to_string(),
                args: vec![
                    crate::parser::TypeAnn {
                        kind: self.type_to_type_ann_kind(ok),
                        span: crate::source::Span::dummy(),
                    },
                    crate::parser::TypeAnn {
                        kind: self.type_to_type_ann_kind(err),
                        span: crate::source::Span::dummy(),
                    },
                ],
            },
            crate::types::Type::Future(inner) => crate::parser::TypeKind::Generic {
                name: "Future".to_string(),
                args: vec![crate::parser::TypeAnn {
                    kind: self.type_to_type_ann_kind(inner),
                    span: crate::source::Span::dummy(),
                }],
            },
            crate::types::Type::Struct { name, .. } => crate::parser::TypeKind::Named(name.clone()),
            crate::types::Type::TypeVar(_) => crate::parser::TypeKind::Named("_".to_string()), // Type variable placeholder
        }
    }

    fn extract_dependencies(&self, module: &ResolvedModule) -> Vec<ModulePath> {
        module
            .dependencies
            .iter()
            .filter_map(|import| ModulePath::from_string(&import.path).ok())
            .collect()
    }

    fn create_compiled_from_cache(
        &self,
        _cached_module: &ResolvedModule,
    ) -> ModuleResult<CompiledModule> {
        // In a real implementation, this would recreate the compiled module from cache
        Err(ModuleError::cache_error(
            "Cache reconstruction not implemented",
        ))
    }

    /// Get a compiled module
    pub fn get_compiled_module(&self, module_path: &ModulePath) -> Option<&CompiledModule> {
        self.loaded_modules.get(module_path)
    }

    /// Get all compiled modules
    pub fn get_all_compiled_modules(&self) -> Vec<&CompiledModule> {
        self.loaded_modules.values().collect()
    }

    /// Clear compiled modules (for hot reload, etc.)
    pub fn clear_compiled_modules(&mut self) {
        self.loaded_modules.clear();
        self.compilation_order.clear();
    }

    /// Get compilation statistics
    pub fn get_compilation_stats(&self) -> CompilationStats {
        let total_modules = self.loaded_modules.len();
        let total_dependencies: usize = self
            .loaded_modules
            .values()
            .map(|m| m.dependencies.len())
            .sum();

        let average_compilation_time = if total_modules > 0 {
            let total_time: std::time::Duration = self
                .loaded_modules
                .values()
                .map(|m| m.compilation_time.elapsed())
                .sum();
            total_time / total_modules as u32
        } else {
            std::time::Duration::ZERO
        };

        CompilationStats {
            total_modules,
            total_dependencies,
            average_compilation_time,
            cache_hits: 0,   // Would be tracked in real implementation
            cache_misses: 0, // Would be tracked in real implementation
        }
    }

    /// Execute a module in a sandbox environment
    pub fn execute_sandboxed(
        &mut self,
        module_path: &ModulePath,
        function_name: &str,
        args: Vec<crate::runtime::Value>,
        sandbox_config: Option<SandboxConfig>,
    ) -> ModuleResult<crate::runtime::Value> {
        // Get module's security context
        let security_context = {
            let security_manager = self.security_manager.lock().unwrap();
            security_manager
                .contexts
                .get(module_path)
                .ok_or_else(|| ModuleError::not_found(module_path.to_string()))?
                .clone()
        };

        // Create sandbox
        let config = sandbox_config.unwrap_or_default();
        let mut sandbox = ModuleSandbox::new(module_path.clone(), security_context, config);

        // Execute function in sandbox
        sandbox
            .execute_function(function_name, args)
            .map_err(|e| ModuleError::runtime_error(module_path.to_string(), e.to_string()))
    }

    /// Grant a capability to a module
    pub fn grant_module_capability(
        &mut self,
        module_path: &ModulePath,
        capability: crate::module::ModuleCapability,
    ) -> ModuleResult<()> {
        let mut security_manager = self.security_manager.lock().unwrap();
        let context = security_manager.get_or_create_context(module_path);
        context.grant_capability(capability)
    }

    /// Check if a module import is allowed
    pub fn check_import_permission(
        &self,
        importer: &ModulePath,
        imported: &ModulePath,
    ) -> ModuleResult<()> {
        let security_manager = self.security_manager.lock().unwrap();
        security_manager.check_import_permission(importer, imported)
    }

    /// Get security report for a module
    pub fn get_module_security_info(
        &self,
        module_path: &ModulePath,
    ) -> Option<ModuleSecurityContext> {
        let security_manager = self.security_manager.lock().unwrap();
        security_manager.contexts.get(module_path).cloned()
    }

    /// Set module permissions
    pub fn set_module_permissions(
        &self,
        _module_path: &ModulePath,
        permissions: crate::module::ModulePermissions,
    ) -> ModuleResult<()> {
        self.permission_manager.register_module(permissions);
        Ok(())
    }

    /// Get module context for enhanced error reporting
    pub fn get_module_context(&self, module_path: &ModulePath) -> Option<&ModuleContext> {
        self.context_stack
            .stack()
            .iter()
            .find(|ctx| &ctx.current_module == module_path)
    }

    /// Format an error with full module context
    pub fn format_error_with_context(
        &mut self,
        error: &ModuleError,
        _module_path: &ModulePath,
    ) -> String {
        if let Some(ctx) = self.context_stack.current_mut() {
            let span = crate::source::Span::new(
                crate::source::SourceLocation::new(0, 0, 0),
                crate::source::SourceLocation::new(0, 0, 0),
            );
            ctx.format_error(error, span)
        } else {
            error.to_string()
        }
    }
}

impl Default for CompilationConfig {
    fn default() -> Self {
        Self {
            enable_caching: true,
            incremental_compilation: true,
            parallel_compilation: false, // Disabled by default for simplicity
            max_parallel_jobs: num_cpus::get().min(8),
            dependency_validation: true,
            circular_dependency_detection: true,
        }
    }
}

/// Statistics about module compilation
#[derive(Debug, Clone)]
pub struct CompilationStats {
    pub total_modules: usize,
    pub total_dependencies: usize,
    pub average_compilation_time: std::time::Duration,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

impl CompilationStats {
    pub fn cache_hit_rate(&self) -> f64 {
        let total_accesses = self.cache_hits + self.cache_misses;
        if total_accesses == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total_accesses as f64
        }
    }
}

/// Helper function to create a default compilation pipeline
pub fn create_default_pipeline() -> ModuleCompilationPipeline {
    use crate::module::{FileSystemResolver, ModuleRegistry, ModuleResolverConfig, RegistryConfig};

    let registry = ModuleRegistry::new(RegistryConfig::default());
    let resolver = Box::new(FileSystemResolver::new(ModuleResolverConfig::default()));
    let semantic_analyzer = SemanticAnalyzer::new();

    ModuleCompilationPipeline::new(registry, resolver, semantic_analyzer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::{FileSystemResolver, ModuleRegistry, ModuleResolverConfig, RegistryConfig};

    fn create_test_pipeline() -> ModuleCompilationPipeline {
        let registry = ModuleRegistry::new(RegistryConfig::default());
        let resolver = Box::new(FileSystemResolver::new(ModuleResolverConfig::default()));
        let semantic_analyzer = SemanticAnalyzer::new();

        ModuleCompilationPipeline::new(registry, resolver, semantic_analyzer)
    }

    #[test]
    fn test_pipeline_creation() {
        let pipeline = create_test_pipeline();
        assert_eq!(pipeline.loaded_modules.len(), 0);
        assert_eq!(pipeline.compilation_order.len(), 0);
    }

    #[test]
    fn test_compilation_config_defaults() {
        let config = CompilationConfig::default();
        assert!(config.enable_caching);
        assert!(config.incremental_compilation);
        assert!(config.dependency_validation);
        assert!(config.circular_dependency_detection);
    }

    #[test]
    fn test_compilation_stats() {
        let stats = CompilationStats {
            total_modules: 5,
            total_dependencies: 10,
            average_compilation_time: std::time::Duration::from_millis(100),
            cache_hits: 8,
            cache_misses: 2,
        };

        assert_eq!(stats.cache_hit_rate(), 0.8);
    }

    #[test]
    fn test_module_exports_creation() {
        let symbol_table = SymbolTable::new();
        let exports = ModuleExports {
            symbols: symbol_table,
            type_definitions: HashMap::new(),
            functions: HashMap::new(),
            variables: HashMap::new(),
            re_exports: HashMap::new(),
        };

        assert_eq!(exports.functions.len(), 0);
        assert_eq!(exports.variables.len(), 0);
        assert_eq!(exports.type_definitions.len(), 0);
        assert_eq!(exports.re_exports.len(), 0);
    }
}
