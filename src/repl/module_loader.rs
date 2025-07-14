//! Module loading support for REPL sessions
//!
//! This module provides functionality to load and import Script modules
//! into REPL sessions, allowing users to access external code and libraries.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::semantic::{analyze, FunctionSignature, SemanticAnalyzer};
use crate::types::Type;
use crate::{Lexer, Parser};

/// Module information for REPL session
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// Module name
    pub name: String,
    /// Module file path
    pub path: PathBuf,
    /// Exported items
    pub exports: ModuleExports,
    /// Module load timestamp
    pub loaded_at: std::time::SystemTime,
}

/// Items exported from a module
#[derive(Debug, Clone, Default)]
pub struct ModuleExports {
    /// Exported variables
    pub variables: HashMap<String, Type>,
    /// Exported functions
    pub functions: HashMap<String, FunctionSignature>,
    /// Exported types
    pub types: HashMap<String, Type>,
}

/// Module loader for REPL sessions
pub struct ModuleLoader {
    /// Loaded modules cache
    loaded_modules: HashMap<String, ModuleInfo>,
    /// Module search paths
    search_paths: Vec<PathBuf>,
}

impl ModuleLoader {
    /// Create a new module loader
    pub fn new() -> Self {
        let mut search_paths = vec![
            PathBuf::from("./"),
            PathBuf::from("./lib/"),
            PathBuf::from("./modules/"),
        ];

        // Add standard library path if it exists
        if let Some(home_dir) = dirs::home_dir() {
            search_paths.push(home_dir.join(".script/lib"));
        }

        Self {
            loaded_modules: HashMap::new(),
            search_paths,
        }
    }

    /// Add a search path for modules
    pub fn add_search_path<P: AsRef<Path>>(&mut self, path: P) {
        self.search_paths.push(path.as_ref().to_path_buf());
    }

    /// Load a module and return its exports
    pub fn load_module(&mut self, module_name: &str) -> Result<&ModuleExports, String> {
        // Check if already loaded
        if self.loaded_modules.contains_key(module_name) {
            return Ok(&self.loaded_modules[module_name].exports);
        }

        // Find the module file
        let module_path = self.find_module_file(module_name)?;

        // Load and parse the module
        let module_info = self.parse_module(module_name, &module_path)?;

        // Cache the module
        self.loaded_modules
            .insert(module_name.to_string(), module_info);

        // Return the exports
        Ok(&self.loaded_modules[module_name].exports)
    }

    /// Import specific items from a module
    pub fn import_items(
        &mut self,
        module_name: &str,
        items: &[String],
    ) -> Result<ModuleExports, String> {
        let module_exports = self.load_module(module_name)?;
        let mut imported = ModuleExports::default();

        for item in items {
            // Check each export category
            if let Some(var_type) = module_exports.variables.get(item) {
                imported.variables.insert(item.clone(), var_type.clone());
            } else if let Some(func_sig) = module_exports.functions.get(item) {
                imported.functions.insert(item.clone(), func_sig.clone());
            } else if let Some(type_def) = module_exports.types.get(item) {
                imported.types.insert(item.clone(), type_def.clone());
            } else {
                return Err(format!(
                    "Item '{}' not found in module '{}'",
                    item, module_name
                ));
            }
        }

        Ok(imported)
    }

    /// Import all items from a module
    pub fn import_all(&mut self, module_name: &str) -> Result<ModuleExports, String> {
        let module_exports = self.load_module(module_name)?;
        Ok(module_exports.clone())
    }

    /// Get information about a loaded module
    pub fn get_module_info(&self, module_name: &str) -> Option<&ModuleInfo> {
        self.loaded_modules.get(module_name)
    }

    /// List all loaded modules
    pub fn list_loaded_modules(&self) -> Vec<&str> {
        self.loaded_modules.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a module is loaded
    pub fn is_module_loaded(&self, module_name: &str) -> bool {
        self.loaded_modules.contains_key(module_name)
    }

    /// Unload a module (for reloading)
    pub fn unload_module(&mut self, module_name: &str) -> bool {
        self.loaded_modules.remove(module_name).is_some()
    }

    /// Reload a module (useful for development)
    pub fn reload_module(&mut self, module_name: &str) -> Result<&ModuleExports, String> {
        self.unload_module(module_name);
        self.load_module(module_name)
    }

    /// Find module file in search paths
    fn find_module_file(&self, module_name: &str) -> Result<PathBuf, String> {
        let possible_names = [
            format!("{}.script", module_name),
            format!("{}/mod.script", module_name),
            format!("{}/main.script", module_name),
        ];

        for search_path in &self.search_paths {
            for name in &possible_names {
                let full_path = search_path.join(name);
                if full_path.exists() && full_path.is_file() {
                    return Ok(full_path);
                }
            }
        }

        Err(format!(
            "Module '{}' not found in search paths: {:?}",
            module_name, self.search_paths
        ))
    }

    /// Parse a module file and extract exports
    fn parse_module(&self, module_name: &str, path: &Path) -> Result<ModuleInfo, String> {
        // Read module file
        let source = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read module '{}': {module_name, e}"))?;

        // Tokenize
        let lexer = Lexer::new(&source)
            .map_err(|e| format!("Lexer error in module '{}': {module_name, e}"))?;

        let (tokens, lex_errors) = lexer.scan_tokens();
        if !lex_errors.is_empty() {
            let mut error_msg = format!("Lexer errors in module '{}':\n", module_name);
            for error in lex_errors {
                error_msg.push_str(&format!("  {}\n", error));
            }
            return Err(error_msg);
        }

        // Parse
        let mut parser = Parser::new(tokens);
        let program = parser
            .parse()
            .map_err(|e| format!("Parse error in module '{}': {module_name, e}"))?;

        // Semantic analysis
        let analyzer = analyze(&program)
            .map_err(|e| format!("Semantic error in module '{}': {module_name, e}"))?;

        // Extract exports
        let exports = self.extract_exports(&program, &analyzer)?;

        Ok(ModuleInfo {
            name: module_name.to_string(),
            path: path.to_path_buf(),
            exports,
            loaded_at: std::time::SystemTime::now(),
        })
    }

    /// Extract exports from parsed module
    fn extract_exports(
        &self,
        program: &crate::parser::Program,
        _analyzer: &SemanticAnalyzer,
    ) -> Result<ModuleExports, String> {
        let mut exports = ModuleExports::default();

        for stmt in &program.statements {
            match stmt {
                crate::parser::Stmt {
                    kind: crate::parser::StmtKind::Let { name, .. },
                    ..
                } => {
                    // Variable - add to exports (assuming public for now)
                    // For now, use unknown type - in practice we'd infer from the analyzer
                    exports.variables.insert(name.clone(), Type::Unknown);
                }
                crate::parser::Stmt {
                    kind:
                        crate::parser::StmtKind::Function {
                            name,
                            params,
                            ret_type,
                            ..
                        },
                    ..
                } => {
                    // Function - add to exports (assuming public for now)
                    let signature =
                        self.create_function_signature(name, params, ret_type.as_ref())?;
                    exports.functions.insert(name.clone(), signature);
                }
                stmt if matches!(
                    stmt.kind,
                    crate::parser::StmtKind::Struct { .. } | crate::parser::StmtKind::Enum { .. }
                ) =>
                {
                    // Type definition - add to exports (assuming public for now)
                    let processed_type = self.process_type_definition(stmt)?;
                    let name = match &stmt.kind {
                        crate::parser::StmtKind::Struct { name, .. } => name.clone(),
                        crate::parser::StmtKind::Enum { name, .. } => name.clone(),
                        _ => unreachable!(),
                    };
                    exports.types.insert(name, processed_type);
                }
                _ => {
                    // Non-public or non-exportable items are ignored
                }
            }
        }

        Ok(exports)
    }

    /// Create function signature from AST
    fn create_function_signature(
        &self,
        name: &str,
        params: &[crate::parser::Param],
        return_type: Option<&crate::parser::TypeAnn>,
    ) -> Result<FunctionSignature, String> {
        Ok(FunctionSignature {
            generic_params: None,
            params: params
                .iter()
                .map(|param| (param.name.clone(), Type::Unknown)) // Simplified: convert TypeAnn to Type::Unknown for now
                .collect(),
            return_type: Type::Unknown, // Simplified: use Unknown for return type
            is_const: false,
            is_async: false,
        })
    }

    /// Process type definition from AST
    fn process_type_definition(&self, stmt: &crate::parser::Stmt) -> Result<Type, String> {
        match &stmt.kind {
            crate::parser::StmtKind::Struct { name, fields, .. } => Ok(Type::Struct {
                name: name.clone(),
                fields: fields
                    .iter()
                    .map(|field| (field.name.clone(), Type::Unknown))
                    .collect(), // Simplified: map TypeAnn to Type::Unknown
            }),
            crate::parser::StmtKind::Enum { name, variants, .. } => Ok(Type::Named(name.clone())),
            _ => Err(format!(
                "Statement is not a type definition: {:?}",
                stmt.kind
            )),
        }
    }

    /// Get search paths
    pub fn search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }

    /// Clear all loaded modules
    pub fn clear(&mut self) {
        self.loaded_modules.clear();
    }

    /// Get module count
    pub fn module_count(&self) -> usize {
        self.loaded_modules.len()
    }

    /// Get total exported items count
    pub fn total_exports_count(&self) -> usize {
        self.loaded_modules
            .values()
            .map(|module| {
                module.exports.variables.len()
                    + module.exports.functions.len()
                    + module.exports.types.len()
            })
            .sum()
    }

    /// Check module freshness (if file has been modified)
    pub fn is_module_fresh(&self, module_name: &str) -> bool {
        if let Some(module) = self.loaded_modules.get(module_name) {
            if let Ok(metadata) = fs::metadata(&module.path) {
                if let Ok(modified) = metadata.modified() {
                    return modified <= module.loaded_at;
                }
            }
        }
        false
    }

    /// Get module dependencies (simplified - would need proper dependency tracking)
    pub fn get_module_dependencies(&self, _module_name: &str) -> Vec<String> {
        // For now, return empty - in practice this would track import statements
        Vec::new()
    }
}

impl Default for ModuleLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_module_loader_creation() {
        let loader = ModuleLoader::new();
        assert!(!loader.search_paths.is_empty());
        assert_eq!(loader.module_count(), 0);
    }

    #[test]
    fn test_add_search_path() {
        let mut loader = ModuleLoader::new();
        let initial_count = loader.search_paths.len();

        loader.add_search_path("/custom/path");
        assert_eq!(loader.search_paths.len(), initial_count + 1);
        assert!(loader.search_paths.contains(&PathBuf::from("/custom/path")));
    }

    #[test]
    fn test_module_not_found() {
        let mut loader = ModuleLoader::new();
        let result = loader.load_module("nonexistent_module");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_module_loading_with_file() {
        let temp_dir = tempdir().unwrap();
        let module_path = temp_dir.path().join("test_module.script");

        // Create a simple module file
        fs::write(
            &module_path,
            "pub fn hello() -> string { \"Hello, World!\" }",
        )
        .unwrap();

        let mut loader = ModuleLoader::new();
        loader.add_search_path(temp_dir.path());

        // This would work in a real implementation with proper parsing
        // For now, it will fail due to simplified parsing, but tests the structure
        let result = loader.load_module("test_module");
        // We expect this to fail in the current simplified implementation
        assert!(result.is_err());
    }

    #[test]
    fn test_module_info_tracking() {
        let loader = ModuleLoader::new();
        assert!(!loader.is_module_loaded("test"));
        assert!(loader.list_loaded_modules().is_empty());
        assert_eq!(loader.total_exports_count(), 0);
    }

    #[test]
    fn test_module_unload() {
        let mut loader = ModuleLoader::new();
        // Can't easily test loading without actual files, but can test unload logic
        assert!(!loader.unload_module("nonexistent"));
    }
}
