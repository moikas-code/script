use crate::lexer::Lexer;
use crate::module::{
    ImportPath, ModuleError, ModuleLoadContext, ModulePath, ModuleRegistry, ModuleResolver,
    ModuleResult, ResolvedModule,
};
use crate::parser::{Parser, Program};
use crate::semantic::{SemanticAnalyzer, SymbolTable};
use std::collections::HashMap;

/// Integration point for module system with the compilation pipeline
pub struct ModuleCompilationPipeline {
    registry: ModuleRegistry,
    resolver: Box<dyn ModuleResolver>,
    semantic_analyzer: SemanticAnalyzer,
    loaded_modules: HashMap<ModulePath, CompiledModule>,
    compilation_order: Vec<ModulePath>,
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

/// Exports from a compiled module
#[derive(Debug, Clone)]
pub struct ModuleExports {
    pub symbols: SymbolTable,
    pub types: HashMap<String, crate::types::Type>,
    pub functions: Vec<String>,
    pub constants: HashMap<String, String>, // Name -> value representation
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
        Self {
            registry,
            resolver,
            semantic_analyzer,
            loaded_modules: HashMap::new(),
            compilation_order: Vec::new(),
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

        // Resolve the module
        let import_path = ImportPath::new(module_path.to_string())?;
        let resolved_module = self.resolver.resolve_module(&import_path, context)?;

        // Check registry cache if enabled
        if config.enable_caching && self.registry.is_registered(&resolved_module.path) {
            if let Some(cached_module) = self.registry.get_module(&resolved_module.path) {
                // Create compiled module from cache
                let compiled = self.create_compiled_from_cache(cached_module)?;
                self.loaded_modules.insert(module_path.clone(), compiled);
                return Ok(());
            }
        }

        // Parse the module
        let ast = self.parse_module(&resolved_module)?;

        // Create module scope with imports
        let mut module_scope = self.create_module_scope(&resolved_module, &ast)?;

        // Perform semantic analysis
        self.semantic_analyzer
            .analyze_program(&ast)
            .map_err(|e| ModuleError::parse_error(module_path.to_string(), e.to_string()))?;

        // Extract exports
        let exports = self.extract_module_exports(&ast, &module_scope)?;

        // Create compiled module
        let compiled_module = CompiledModule {
            module: resolved_module.clone(),
            ast,
            symbol_table: module_scope,
            dependencies: self.extract_dependencies(&resolved_module),
            exports,
            compilation_time: compilation_start,
        };

        // Register in registry
        self.registry.register_module(resolved_module)?;

        // Store compiled module
        self.loaded_modules
            .insert(module_path.clone(), compiled_module);

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
        let lexer = Lexer::new(&module.source);
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
        _symbol_table: &mut SymbolTable,
        _import: &ImportPath,
    ) -> ModuleResult<()> {
        // In a real implementation, this would:
        // 1. Resolve the import to a module
        // 2. Get the exports from that module
        // 3. Add the imported symbols to the current symbol table

        // For now, this is a placeholder
        Ok(())
    }

    fn extract_module_exports(
        &self,
        _ast: &Program,
        symbol_table: &SymbolTable,
    ) -> ModuleResult<ModuleExports> {
        // In a real implementation, this would extract exported symbols from the AST
        Ok(ModuleExports {
            symbols: symbol_table.clone(),
            types: HashMap::new(),
            functions: Vec::new(),
            constants: HashMap::new(),
        })
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
            types: HashMap::new(),
            functions: Vec::new(),
            constants: HashMap::new(),
        };

        assert_eq!(exports.functions.len(), 0);
        assert_eq!(exports.constants.len(), 0);
    }
}
