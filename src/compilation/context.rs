use crate::codegen::debug::DebugFlags;
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
                format!("Failed to read file '{}': {}", path.display(), e),
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
        let lexer = Lexer::new(&self.source);
        let (tokens, lex_errors) = lexer.scan_tokens();

        if !lex_errors.is_empty() {
            // Return the first error for now
            return Err(lex_errors
                .into_iter()
                .next()
                .unwrap()
                .with_file_name(self.path.to_string_lossy().to_string()));
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
    /// Compilation order (module names in dependency order)
    compilation_order: Vec<String>,
    /// Type information collected during semantic analysis
    type_info: HashMap<usize, crate::types::Type>,
    /// Package root directory
    package_root: Option<PathBuf>,
    /// Whether to compile in release mode
    release_mode: bool,
    /// Debug compilation flags
    debug_flags: DebugFlags,
}

impl CompilationContext {
    /// Create a new compilation context
    pub fn new() -> Self {
        CompilationContext {
            units: HashMap::new(),
            global_symbols: SymbolTable::new(),
            compilation_order: Vec::new(),
            type_info: HashMap::new(),
            package_root: None,
            release_mode: false,
            debug_flags: DebugFlags::default(),
        }
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
                    format!("Duplicate module name: {}", module_name),
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
                format!("Failed to read directory '{}': {}", dir.display(), e),
            )
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                Error::new(
                    ErrorKind::FileError,
                    format!("Failed to read directory entry: {}", e),
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

    /// Analyze module dependencies to determine compilation order
    fn analyze_dependencies(&mut self) -> Result<()> {
        // For now, use a simple topological sort based on import statements
        let mut order = Vec::new();
        let mut visited = HashMap::new();
        let mut temp_visited = HashMap::new();

        for module_name in self.units.keys() {
            if !visited.contains_key(module_name) {
                self.visit_module(module_name, &mut visited, &mut temp_visited, &mut order)?;
            }
        }

        self.compilation_order = order;
        Ok(())
    }

    /// DFS visit for topological sort
    fn visit_module(
        &self,
        module_name: &str,
        visited: &mut HashMap<String, bool>,
        temp_visited: &mut HashMap<String, bool>,
        order: &mut Vec<String>,
    ) -> Result<()> {
        if temp_visited.get(module_name).copied().unwrap_or(false) {
            return Err(Error::new(
                ErrorKind::CompilationError,
                format!(
                    "Circular dependency detected involving module '{}'",
                    module_name
                ),
            ));
        }

        if visited.get(module_name).copied().unwrap_or(false) {
            return Ok(());
        }

        temp_visited.insert(module_name.to_string(), true);

        // Get dependencies from import statements
        if let Some(unit) = self.units.get(module_name) {
            if let Some(ast) = &unit.ast {
                for stmt in &ast.statements {
                    if let crate::parser::StmtKind::Import { module, .. } = &stmt.kind {
                        // Extract module name from import path
                        let dep_name = Path::new(module)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or(module);

                        // Only process if it's an internal module
                        if self.units.contains_key(dep_name) {
                            self.visit_module(dep_name, visited, temp_visited, order)?;
                        }
                    }
                }
            }
        }

        temp_visited.insert(module_name.to_string(), false);
        visited.insert(module_name.to_string(), true);
        order.push(module_name.to_string());

        Ok(())
    }

    /// Compile all loaded modules
    fn compile_all(&mut self) -> Result<IrModule> {
        // Perform semantic analysis on all modules
        for module_name in &self.compilation_order.clone() {
            self.analyze_module(module_name)?;
        }

        // Lower all modules to IR
        let mut lowerer = AstLowerer::new(self.global_symbols.clone(), self.type_info.clone());

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

        lowerer.lower_program(&combined_program)
    }

    /// Perform semantic analysis on a single module
    fn analyze_module(&mut self, module_name: &str) -> Result<()> {
        let unit = self.units.get(module_name).ok_or_else(|| {
            Error::new(
                ErrorKind::CompilationError,
                format!("Module '{}' not found", module_name),
            )
        })?;

        if let Some(ast) = &unit.ast {
            // Create a semantic analyzer for this module
            let mut analyzer = crate::semantic::SemanticAnalyzer::new();

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

            // For now, we don't merge symbol tables as that would require
            // more sophisticated module system support
            // TODO: Implement proper module-aware symbol table merging
        }

        Ok(())
    }
}
