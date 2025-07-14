use crate::compilation::module_loader::ModuleLoader;
use crate::error::{Error, ErrorKind, Result};
use crate::lexer::Lexer;
use crate::parser::{Parser, Program};
use crate::semantic::symbol_table::SymbolTable;
use crate::source::Span;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Provides integration between the semantic analyzer and module loading system
#[derive(Debug)]
pub struct ModuleLoaderIntegration {
    /// Module loader for resolving module paths
    module_loader: ModuleLoader,
    /// Cache of loaded and parsed modules
    loaded_modules: Arc<Mutex<HashMap<String, LoadedModule>>>,
    /// Current file path for resolving relative imports
    current_file: Option<PathBuf>,
}

/// Represents a loaded and parsed module
#[derive(Debug, Clone)]
pub struct LoadedModule {
    /// The parsed AST
    pub ast: Program,
    /// The module's symbol table
    pub symbol_table: SymbolTable,
    /// The file path
    pub file_path: PathBuf,
    /// The module name
    pub module_name: String,
}

impl ModuleLoaderIntegration {
    /// Create a new module loader integration
    pub fn new() -> Self {
        Self {
            module_loader: ModuleLoader::new(),
            loaded_modules: Arc::new(Mutex::new(HashMap::new())),
            current_file: None,
        }
    }

    /// Set the current file for resolving relative imports
    pub fn set_current_file(&mut self, file: Option<PathBuf>) {
        self.current_file = file;
    }

    /// Load and parse a module, returning its symbol table
    pub fn load_module(&mut self, module_path: &str, span: Span) -> Result<SymbolTable> {
        // Check if module is already loaded
        {
            let loaded = self.loaded_modules.lock().unwrap();
            if let Some(module) = loaded.get(module_path) {
                return Ok(module.symbol_table.clone());
            }
        }

        // Resolve module path to file path
        let file_path = self
            .module_loader
            .resolve_module(module_path, self.current_file.as_deref())
            .map_err(|e| {
                Error::new(
                    ErrorKind::ModuleError,
                    format!("Failed to resolve module '{}': {module_path, e}"),
                )
                .with_location(span.start)
            })?;

        // Load and parse the module
        let loaded_module = self.load_and_parse_module(&file_path, module_path, span)?;

        // Cache the loaded module
        let symbol_table = loaded_module.symbol_table.clone();
        {
            let mut loaded = self.loaded_modules.lock().unwrap();
            loaded.insert(module_path.to_string(), loaded_module);
        }

        Ok(symbol_table)
    }

    /// Load and parse a module from a file path
    fn load_and_parse_module(
        &self,
        file_path: &Path,
        module_name: &str,
        span: Span,
    ) -> Result<LoadedModule> {
        // Read the source file
        let source = fs::read_to_string(file_path).map_err(|e| {
            Error::new(
                ErrorKind::FileError,
                format!("Failed to read module '{}': {module_name, e}"),
            )
            .with_location(span.start)
        })?;

        // Lex the source
        let lexer = Lexer::new(&source)?;
        let (tokens, lex_errors) = lexer.scan_tokens();

        if !lex_errors.is_empty() {
            return Err(lex_errors[0]
                .clone()
                .with_file_name(file_path.to_string_lossy().to_string())
                .with_location(span.start));
        }

        // Parse the tokens
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().map_err(|e| {
            e.with_file_name(file_path.to_string_lossy().to_string())
                .with_location(span.start)
        })?;

        // Create a new semantic analyzer for the module
        let mut analyzer = crate::semantic::SemanticAnalyzer::new();

        // Set up module context
        let old_file = file_path.to_path_buf();
        analyzer.set_current_file(Some(old_file.clone()));

        // Analyze the module
        analyzer.analyze_program(&ast).map_err(|e| {
            e.with_file_name(file_path.to_string_lossy().to_string())
                .with_location(span.start)
        })?;

        // Check for semantic errors
        let errors = analyzer.errors();
        if !errors.is_empty() {
            return Err(errors[0]
                .clone()
                .into_error()
                .with_file_name(file_path.to_string_lossy().to_string())
                .with_location(span.start));
        }

        // Extract the symbol table
        let symbol_table = analyzer.symbol_table().clone();

        Ok(LoadedModule {
            ast,
            symbol_table,
            file_path: file_path.to_path_buf(),
            module_name: module_name.to_string(),
        })
    }

    /// Get all loaded modules
    pub fn get_loaded_modules(&self) -> HashMap<String, LoadedModule> {
        self.loaded_modules.lock().unwrap().clone()
    }

    /// Check if a module has been loaded
    pub fn is_module_loaded(&self, module_path: &str) -> bool {
        self.loaded_modules
            .lock()
            .unwrap()
            .contains_key(module_path)
    }

    /// Add a search path for module resolution
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.module_loader.add_search_path(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_simple_module() {
        // Create temporary directory and module file
        let temp_dir = TempDir::new().unwrap();
        let module_path = temp_dir.path().join("test_module.script");

        let module_content = r#"
export fn add(a: i32, b: i32) -> i32 {
    a + b
}

export let PI = 3.14159;
"#;

        fs::write(&module_path, module_content).unwrap();

        // Create module loader
        let mut loader = ModuleLoaderIntegration::new();
        loader.add_search_path(temp_dir.path().to_path_buf());

        // Load the module
        let dummy_span = Span::new(
            crate::source::SourceLocation::new(0, 0, 0),
            crate::source::SourceLocation::new(0, 0, 0),
        );

        let symbol_table = loader.load_module("test_module", dummy_span).unwrap();

        // Verify exports are in the symbol table
        assert!(symbol_table.lookup("add").is_some());
        assert!(symbol_table.lookup("PI").is_some());
    }
}
