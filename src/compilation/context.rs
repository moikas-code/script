use crate::codegen::debug::DebugFlags;
use crate::compilation::resource_limits::{ResourceLimits, ResourceMonitor};
use crate::error::{Error, ErrorKind, Result};
use crate::ir::Module as IrModule;
use crate::lexer::Lexer;
use crate::lowering::AstLowerer;
use crate::parser::{Parser, Program};
use crate::semantic::SymbolTable;
use crate::source::SourceLocation;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Represents a single compilation unit (a .script file)
#[derive(Debug)]
pub struct CompilationUnit {
    /// Path to the source file
    pub path: PathBuf,
    /// Source code content
    pub source: String,
    /// Parsed AST
    pub ast: Option<Program>,
    /// Symbol table for this unit
    pub symbols: Option<SymbolTable>,
    /// Module name (derived from file path)
    pub module_name: String,
}

impl CompilationUnit {
    /// Create a new compilation unit from a file path
    pub fn from_file(path: &Path) -> Result<Self> {
        let source = fs::read_to_string(path).map_err(|e| {
            Error::new(
                ErrorKind::FileError,
                format!("Failed to read file '{}': {path.display(}"), e),
            )
        })?;

        let module_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed")
            .to_string();

        Ok(CompilationUnit {
            path: path.to_path_buf(),
            source,
            ast: None,
            symbols: None,
            module_name,
        })
    }

    /// Parse the source code
    pub fn parse(&mut self) -> Result<()> {
        let lexer = Lexer::new(&self.source)?;
        let (tokens, lex_errors) = lexer.scan_tokens();

        if !lex_errors.is_empty() {
            // Return the first error for now
            if let Some(first_error) = lex_errors.into_iter().next() {
                return Err(first_error.with_file_name(self.path.to_string_lossy().to_string()));
            } else {
                // This should never happen, but handle it gracefully
                return Err(Error::new(
                    ErrorKind::LexerError,
                    "Lexer reported errors but no errors were found",
                )
                .with_file_name(self.path.to_string_lossy().to_string()));
            }
        }

        let mut parser = Parser::new(tokens);
        let ast = parser
            .parse()
            .map_err(|e| e.with_file_name(self.path.to_string_lossy().to_string()))?;

        self.ast = Some(ast);
        Ok(())
    }
}

/// Manages the compilation of multiple Script files
pub struct CompilationContext {
    /// All compilation units indexed by module name
    units: HashMap<String, CompilationUnit>,
    /// Global symbol table
    global_symbols: SymbolTable,
    /// Module symbol tables (for cross-module import resolution)
    module_symbols: HashMap<String, SymbolTable>,
    /// Compilation order (module names in dependency order)
    compilation_order: Vec<String>,
    /// Type information collected during semantic analysis
    type_info: HashMap<usize, crate::types::Type>,
    /// Generic instantiations collected during semantic analysis
    generic_instantiations: Vec<crate::semantic::analyzer::GenericInstantiation>,
    /// Closure capture information collected during semantic analysis
    closure_captures: HashMap<usize, Vec<(String, crate::types::Type, bool)>>,
    /// Package root directory
    package_root: Option<PathBuf>,
    /// Whether to compile in release mode
    release_mode: bool,
    /// Debug compilation flags
    debug_flags: DebugFlags,
    /// Resource monitor for DoS protection
    resource_monitor: ResourceMonitor,
}

impl CompilationContext {
    /// Create a new compilation context
    pub fn new() -> Self {
        Self::with_resource_limits(ResourceLimits::production())
    }

    /// Create a new compilation context with custom resource limits
    pub fn with_resource_limits(limits: ResourceLimits) -> Self {
        CompilationContext {
            units: HashMap::new(),
            global_symbols: SymbolTable::new(),
            module_symbols: HashMap::new(),
            compilation_order: Vec::new(),
            type_info: HashMap::new(),
            generic_instantiations: Vec::new(),
            closure_captures: HashMap::new(),
            package_root: None,
            release_mode: false,
            debug_flags: DebugFlags::default(),
            resource_monitor: ResourceMonitor::new(limits),
        }
    }

    /// Create a new compilation context for development (more permissive limits)
    pub fn for_development() -> Self {
        Self::with_resource_limits(ResourceLimits::development())
    }

    /// Create a new compilation context for testing (very permissive limits)
    pub fn for_testing() -> Self {
        Self::with_resource_limits(ResourceLimits::testing())
    }

    /// Set the package root directory
    pub fn set_package_root(&mut self, root: PathBuf) {
        self.package_root = Some(root);
    }

    /// Set whether to compile in release mode
    pub fn set_release_mode(&mut self, release: bool) {
        self.release_mode = release;
        // Update debug flags based on release mode
        if release {
            self.debug_flags = DebugFlags::release();
        } else {
            self.debug_flags = DebugFlags::debug();
        }
    }

    /// Set debug flags
    pub fn set_debug_flags(&mut self, flags: DebugFlags) {
        self.debug_flags = flags;
    }

    /// Get debug flags
    pub fn debug_flags(&self) -> &DebugFlags {
        &self.debug_flags
    }

    /// Compile a single file
    pub fn compile_file(&mut self, path: &Path) -> Result<IrModule> {
        if !path.exists() {
            return Err(Error::new(
                ErrorKind::FileError,
                format!("File '{}' does not exist", path.display()),
            ));
        }

        let mut unit = CompilationUnit::from_file(path)?;
        unit.parse()?;

        let module_name = unit.module_name.clone();
        self.units.insert(module_name.clone(), unit);
        self.compilation_order.push(module_name);

        self.compile_all()
    }

    /// Compile all .script files in a directory
    pub fn compile_directory(&mut self, dir: &Path) -> Result<IrModule> {
        if !dir.is_dir() {
            return Err(Error::new(
                ErrorKind::FileError,
                format!("'{}' is not a directory", dir.display()),
            ));
        }

        // Find all .script files
        let script_files = self.find_script_files(dir)?;

        if script_files.is_empty() {
            return Err(Error::new(
                ErrorKind::FileError,
                format!("No .script files found in '{}'", dir.display()),
            ));
        }

        // Load and parse all files
        for file_path in script_files {
            let mut unit = CompilationUnit::from_file(&file_path)?;
            unit.parse()?;

            let module_name = unit.module_name.clone();
            if self.units.contains_key(&module_name) {
                return Err(Error::new(
                    ErrorKind::CompilationError,
                    format!("Duplicate module name: {module_name}"),
                )
                .with_location(SourceLocation::new(1, 1, 0)));
            }

            self.units.insert(module_name, unit);
        }

        // Analyze dependencies and determine compilation order
        self.analyze_dependencies()?;

        // Compile all modules
        self.compile_all()
    }

    /// Find all .script files in a directory (non-recursive for now)
    fn find_script_files(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        let entries = fs::read_dir(dir).map_err(|e| {
            Error::new(
                ErrorKind::FileError,
                format!("Failed to read directory '{}': {dir.display(}"), e),
            )
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                Error::new(
                    ErrorKind::FileError,
                    format!("Failed to read directory entry: {e}"),
                )
            })?;

            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("script") {
                files.push(path);
            }
        }

        files.sort();
        Ok(files)
    }

    /// Analyze module dependencies to determine compilation order using proper dependency graph
    fn analyze_dependencies(&mut self) -> Result<()> {
        use crate::compilation::dependency_graph::{DependencyAnalyzer, DependencyGraph};

        // Create dependency analyzer with base path
        let base_path = std::env::current_dir().map_err(|e| {
            Error::new(
                ErrorKind::CompilationError,
                format!("Failed to get current directory: {e}"),
            )
        })?;
        let analyzer = DependencyAnalyzer::with_base_path(base_path);

        // Build dependency graph using proper module resolution
        let mut graph = DependencyGraph::new();

        // Add all modules to the graph
        for module_name in self.units.keys() {
            graph.add_module(module_name.clone());
        }

        // Analyze dependencies for each module
        for (module_name, unit) in &self.units {
            if let Some(ast) = &unit.ast {
                let module_path = Some(unit.path.as_path());
                let dependencies = analyzer.analyze(ast, module_path);

                for dep in dependencies {
                    // Only add if it's an internal module
                    if self.units.contains_key(&dep) {
                        graph.add_dependency(module_name.clone(), dep);
                    }
                }
            }
        }

        // Get topological order
        match graph.topological_sort() {
            Ok(order) => {
                self.compilation_order = order;
                Ok(())
            }
            Err(_) => {
                // Detect specific circular dependencies for better error reporting
                let cycle = graph.find_cycle();
                if let Some(cycle_path) = cycle {
                    Err(Error::new(
                        ErrorKind::CompilationError,
                        format!("Circular dependency detected: {cycle_path.join(" -> "}")),
                    ))
                } else {
                    Err(Error::new(
                        ErrorKind::CompilationError,
                        "Circular dependency detected in module imports",
                    ))
                }
            }
        }
    }

    /// Compile all loaded modules
    fn compile_all(&mut self) -> Result<IrModule> {
        // Start resource monitoring for compilation
        self.resource_monitor.start_phase("compilation")?;

        // Perform semantic analysis on all modules with resource monitoring
        self.resource_monitor.start_phase("semantic_analysis")?;
        for (i, module_name) in self.compilation_order.clone().iter().enumerate() {
            // Check resource limits every 10 modules
            if i % 10 == 0 {
                self.resource_monitor
                    .check_phase_timeout("semantic_analysis")?;
                self.resource_monitor.check_total_timeout()?;
                self.resource_monitor
                    .check_iteration_limit("module_analysis", 10)?;

                // Check memory usage periodically
                self.resource_monitor.check_system_memory()?;
            }

            self.analyze_module(module_name)?;
        }
        self.resource_monitor.end_phase("semantic_analysis");

        // Lower all modules to IR with resource monitoring
        self.resource_monitor.start_phase("lowering")?;
        let mut lowerer = AstLowerer::new(
            self.global_symbols.clone(),
            self.type_info.clone(),
            self.generic_instantiations.clone(),
            self.closure_captures.clone(),
        );

        // Create a combined program from all modules
        let mut combined_statements = Vec::new();

        for module_name in &self.compilation_order {
            if let Some(unit) = self.units.get(module_name) {
                if let Some(ast) = &unit.ast {
                    // Add module prefix to function names to avoid conflicts
                    // For now, just add all statements
                    combined_statements.extend(ast.statements.clone());
                }
            }
        }

        let combined_program = Program {
            statements: combined_statements,
        };

        let mut ir_module = lowerer.lower_program(&combined_program)?;
        self.resource_monitor.end_phase("lowering");

        // Monomorphize generic functions if any exist
        if !self.generic_instantiations.is_empty() || self.has_generic_functions(&ir_module) {
            use crate::codegen::MonomorphizationContext;

            self.resource_monitor.start_phase("monomorphization")?;
            let mut mono_context = MonomorphizationContext::new();
            mono_context
                .initialize_from_semantic_analysis(&self.generic_instantiations, &self.type_info);

            mono_context.monomorphize(&mut ir_module)?;
            self.resource_monitor.end_phase("monomorphization");

            // Report monomorphization statistics
            let stats = mono_context.stats();
            if stats.functions_monomorphized > 0 {
                println!(
                    "Monomorphized {} generic functions ({} instantiations, {} duplicates avoided)",
                    stats.functions_monomorphized, stats.type_instantiations, stats.cache_hits
                );
            }
        }

        // Complete resource monitoring for compilation
        self.resource_monitor.end_phase("compilation");

        Ok(ir_module)
    }

    /// Check if the IR module contains generic functions
    fn has_generic_functions(&self, module: &IrModule) -> bool {
        // Check each function for type parameters
        for function in module.functions().values() {
            if self.is_generic_function(function) {
                return true;
            }
        }
        false
    }

    /// Check if a function is generic (has type parameters)
    fn is_generic_function(&self, function: &crate::ir::Function) -> bool {
        // Check parameters
        for param in &function.params {
            if self.has_type_parameter(&param.ty) {
                return true;
            }
        }

        // Check return type
        self.has_type_parameter(&function.return_type)
    }

    /// Check if a type contains type parameters
    fn has_type_parameter(&self, ty: &crate::types::Type) -> bool {
        use crate::types::Type;

        match ty {
            Type::TypeParam(_) => true,
            Type::Array(elem) => self.has_type_parameter(elem),
            Type::Option(inner) => self.has_type_parameter(inner),
            Type::Result { ok, err } => self.has_type_parameter(ok) || self.has_type_parameter(err),
            Type::Function { params, ret } => {
                params.iter().any(|p| self.has_type_parameter(p)) || self.has_type_parameter(ret)
            }
            Type::Generic { args, .. } => args.iter().any(|arg| self.has_type_parameter(arg)),
            Type::Future(inner) => self.has_type_parameter(inner),
            Type::Tuple(types) => types.iter().any(|t| self.has_type_parameter(t)),
            Type::Reference { inner, .. } => self.has_type_parameter(inner),
            _ => false,
        }
    }

    /// Perform semantic analysis on a single module
    fn analyze_module(&mut self, module_name: &str) -> Result<()> {
        let unit = self.units.get(module_name).ok_or_else(|| {
            Error::new(
                ErrorKind::CompilationError,
                format!("Module '{}' not found", module_name),
            )
        })?;

        let file_path = unit.path.clone();

        if let Some(ast) = &unit.ast {
            // Create a semantic analyzer with the shared global symbol table
            let mut analyzer =
                crate::semantic::SemanticAnalyzer::with_symbol_table(self.global_symbols.clone());

            // Set the current file path for resolving relative imports
            analyzer.set_current_file(Some(file_path.clone()));

            // Add the directory containing the current module as a search path
            if let Some(parent) = file_path.parent() {
                analyzer.add_module_search_path(parent.to_path_buf());
            }

            // Add package root as a search path if available
            if let Some(root) = &self.package_root {
                analyzer.add_module_search_path(root.clone());
            }

            // Register all previously analyzed modules for import resolution
            for (prev_module_name, prev_symbol_table) in &self.module_symbols {
                analyzer.register_module(prev_module_name, prev_symbol_table);
            }

            // Analyze the module's AST
            if let Err(error) = analyzer.analyze_program(ast) {
                return Err(error);
            }

            // Check for semantic errors
            let errors = analyzer.errors();
            if !errors.is_empty() {
                // Return the first error
                return Err(errors[0].clone().into_error());
            }

            // Extract type information and merge with global type info
            let module_types = analyzer.extract_type_info();
            self.type_info.extend(module_types);

            // Collect generic instantiations from this module
            let module_instantiations = analyzer.generic_instantiations().to_vec();
            self.generic_instantiations.extend(module_instantiations);

            // Extract closure captures from this module
            let module_captures = analyzer.extract_closure_captures();
            self.closure_captures.extend(module_captures);

            // Save this module's symbol table for future imports
            self.module_symbols
                .insert(module_name.to_string(), analyzer.symbol_table().clone());

            // Update the global symbol table with symbols from this module
            self.global_symbols = analyzer.symbol_table().clone();
        }

        Ok(())
    }
}
