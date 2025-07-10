use crate::error::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};

/// Represents a module path in Script compilation context
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CompilationModulePath {
    /// The components of the module path (e.g., ["std", "io"])
    pub components: Vec<String>,
}

impl CompilationModulePath {
    /// Create a module path from a string like "std.io"
    pub fn from_str(s: &str) -> Self {
        CompilationModulePath {
            components: s.split('.').map(|s| s.to_string()).collect(),
        }
    }

    /// Convert to a file system path
    pub fn to_fs_path(&self) -> PathBuf {
        let mut path = PathBuf::new();
        for component in &self.components {
            path.push(component);
        }
        path.set_extension("script");
        path
    }

    /// Get the module name (last component)
    pub fn module_name(&self) -> &str {
        self.components.last().map(|s| s.as_str()).unwrap_or("")
    }
}

/// Resolves and loads Script modules
#[derive(Debug)]
pub struct ModuleLoader {
    /// Search paths for modules
    search_paths: Vec<PathBuf>,
    /// Cache of loaded module paths
    loaded_modules: Vec<PathBuf>,
}

impl ModuleLoader {
    /// Create a new module loader
    pub fn new() -> Self {
        ModuleLoader {
            search_paths: vec![
                PathBuf::from("."), // Current directory
                                    // Add more default paths as needed
            ],
            loaded_modules: Vec::new(),
        }
    }

    /// Add a search path
    pub fn add_search_path(&mut self, path: PathBuf) {
        if !self.search_paths.contains(&path) {
            self.search_paths.push(path);
        }
    }

    /// Resolve a module import to a file path
    pub fn resolve_module(&self, import_path: &str, from_file: Option<&Path>) -> Result<PathBuf> {
        // Handle relative imports (starting with . or ..)
        if import_path.starts_with('.') {
            return self.resolve_relative_import(import_path, from_file);
        }

        // Handle absolute imports
        let module_path = CompilationModulePath::from_str(import_path);
        let fs_path = module_path.to_fs_path();

        // Search in all search paths
        for search_path in &self.search_paths {
            let full_path = search_path.join(&fs_path);
            if full_path.exists() && full_path.is_file() {
                return Ok(full_path);
            }
        }

        // If we have a from_file, also check relative to that file's directory
        if let Some(from) = from_file {
            if let Some(parent) = from.parent() {
                let full_path = parent.join(&fs_path);
                if full_path.exists() && full_path.is_file() {
                    return Ok(full_path);
                }
            }
        }

        Err(Error::new(
            ErrorKind::FileError,
            format!("Module '{}' not found in search paths", import_path),
        ))
    }

    /// Resolve a relative import
    fn resolve_relative_import(
        &self,
        import_path: &str,
        from_file: Option<&Path>,
    ) -> Result<PathBuf> {
        let from_file = from_file.ok_or_else(|| {
            Error::new(
                ErrorKind::CompilationError,
                "Relative imports require a source file context",
            )
        })?;

        let parent = from_file.parent().ok_or_else(|| {
            Error::new(
                ErrorKind::CompilationError,
                "Cannot resolve relative import from root",
            )
        })?;

        // Parse the relative path
        let mut current = parent.to_path_buf();
        let parts: Vec<&str> = import_path.split('.').collect();

        for (i, part) in parts.iter().enumerate() {
            if *part == "" && i == 0 {
                // Leading dot means current directory
                continue;
            } else if *part == "" && i > 0 {
                // Double dot means parent directory
                current = current
                    .parent()
                    .ok_or_else(|| {
                        Error::new(
                            ErrorKind::CompilationError,
                            "Relative import goes above root directory",
                        )
                    })?
                    .to_path_buf();
            } else {
                // Regular module name
                current.push(part);
            }
        }

        current.set_extension("script");

        if !current.exists() {
            return Err(Error::new(
                ErrorKind::FileError,
                format!(
                    "Module '{}' not found at '{}'",
                    import_path,
                    current.display()
                ),
            ));
        }

        Ok(current)
    }

    /// Check if a module has already been loaded
    pub fn is_loaded(&self, path: &Path) -> bool {
        self.loaded_modules.iter().any(|p| p == path)
    }

    /// Mark a module as loaded
    pub fn mark_loaded(&mut self, path: PathBuf) {
        if !self.is_loaded(&path) {
            self.loaded_modules.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_path() {
        let path = CompilationModulePath::from_str("std.io.file");
        assert_eq!(path.components, vec!["std", "io", "file"]);
        assert_eq!(path.module_name(), "file");
        assert_eq!(path.to_fs_path(), PathBuf::from("std/io/file.script"));
    }

    #[test]
    fn test_simple_module_path() {
        let path = CompilationModulePath::from_str("math");
        assert_eq!(path.components, vec!["math"]);
        assert_eq!(path.module_name(), "math");
        assert_eq!(path.to_fs_path(), PathBuf::from("math.script"));
    }
}
