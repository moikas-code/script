use crate::error::{Error, ErrorKind};
use crate::source::SourceLocation;
use std::fmt;
use std::path::PathBuf;

pub type ModuleResult<T> = Result<T, ModuleError>;

/// Module system specific errors
#[derive(Debug, Clone)]
pub struct ModuleError {
    pub kind: ModuleErrorKind,
    pub message: String,
    pub location: Option<SourceLocation>,
    pub module_path: Option<String>,
    pub file_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModuleErrorKind {
    /// Module not found in any search path
    NotFound,
    /// Circular dependency detected
    CircularDependency,
    /// Invalid module path format
    InvalidPath,
    /// File system error (permission, not exists, etc.)
    FileSystem,
    /// Parse error in module file
    ParseError,
    /// Import error (symbol not found, etc.)
    ImportError,
    /// Cache error
    CacheError,
    /// Configuration error
    ConfigError,
}

impl ModuleError {
    pub fn new(kind: ModuleErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            location: None,
            module_path: None,
            file_path: None,
        }
    }

    pub fn not_found(module_path: impl Into<String>) -> Self {
        let path = module_path.into();
        Self::new(
            ModuleErrorKind::NotFound,
            format!("Module '{}' not found in any search path", path),
        )
        .with_module_path(path)
    }

    pub fn circular_dependency(
        stack: &[crate::module::ModulePath],
        current: &crate::module::ModulePath,
    ) -> Self {
        let cycle_start = stack
            .iter()
            .position(|module| module == current)
            .unwrap_or(0);

        let cycle: Vec<String> = stack[cycle_start..]
            .iter()
            .map(|m| m.to_string())
            .chain(std::iter::once(current.to_string()))
            .collect();

        Self::new(
            ModuleErrorKind::CircularDependency,
            format!("Circular dependency detected: {}", cycle.join(" -> ")),
        )
        .with_module_path(current.to_string())
    }

    pub fn invalid_path(path: impl Into<String>, reason: impl Into<String>) -> Self {
        let path_str = path.into();
        Self::new(
            ModuleErrorKind::InvalidPath,
            format!("Invalid module path '{}': {}", path_str, reason.into()),
        )
        .with_module_path(path_str)
    }

    pub fn file_system(path: impl Into<PathBuf>, error: std::io::Error) -> Self {
        let path_buf = path.into();
        Self::new(
            ModuleErrorKind::FileSystem,
            format!("File system error for '{}': {}", path_buf.display(), error),
        )
        .with_file_path(path_buf)
    }

    pub fn parse_error(module_path: impl Into<String>, error: impl Into<String>) -> Self {
        let path = module_path.into();
        Self::new(
            ModuleErrorKind::ParseError,
            format!("Parse error in module '{}': {}", path, error.into()),
        )
        .with_module_path(path)
    }

    pub fn import_error(module_path: impl Into<String>, symbol: impl Into<String>) -> Self {
        let path = module_path.into();
        let symbol_name = symbol.into();
        Self::new(
            ModuleErrorKind::ImportError,
            format!("Symbol '{}' not found in module '{}'", symbol_name, path),
        )
        .with_module_path(path)
    }

    pub fn cache_error(message: impl Into<String>) -> Self {
        Self::new(ModuleErrorKind::CacheError, message)
    }

    pub fn config_error(message: impl Into<String>) -> Self {
        Self::new(ModuleErrorKind::ConfigError, message)
    }

    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }

    pub fn with_module_path(mut self, path: impl Into<String>) -> Self {
        self.module_path = Some(path.into());
        self
    }

    pub fn with_file_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.file_path = Some(path.into());
        self
    }

    /// Convert to the main error type for compatibility
    pub fn into_error(self) -> Error {
        let mut error = Error::new(ErrorKind::ParseError, self.message);

        if let Some(location) = self.location {
            error = error.with_location(location);
        }

        if let Some(file_path) = self.file_path {
            error = error.with_file_name(file_path.to_string_lossy());
        } else if let Some(module_path) = self.module_path {
            error = error.with_file_name(module_path);
        }

        error
    }
}

impl fmt::Display for ModuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let error_type = match self.kind {
            ModuleErrorKind::NotFound => "Module Not Found",
            ModuleErrorKind::CircularDependency => "Circular Dependency",
            ModuleErrorKind::InvalidPath => "Invalid Path",
            ModuleErrorKind::FileSystem => "File System Error",
            ModuleErrorKind::ParseError => "Parse Error",
            ModuleErrorKind::ImportError => "Import Error",
            ModuleErrorKind::CacheError => "Cache Error",
            ModuleErrorKind::ConfigError => "Configuration Error",
        };

        write!(f, "{}: {}", error_type, self.message)?;

        if let Some(location) = &self.location {
            if let Some(module_path) = &self.module_path {
                write!(f, "\n    --> {}:{}", module_path, location)?;
            } else {
                write!(f, "\n    --> {}", location)?;
            }
        } else if let Some(module_path) = &self.module_path {
            write!(f, "\n    --> {}", module_path)?;
        } else if let Some(file_path) = &self.file_path {
            write!(f, "\n    --> {}", file_path.display())?;
        }

        Ok(())
    }
}

impl std::error::Error for ModuleError {}

impl From<ModuleError> for Error {
    fn from(error: ModuleError) -> Self {
        error.into_error()
    }
}

impl From<std::io::Error> for ModuleError {
    fn from(error: std::io::Error) -> Self {
        ModuleError::new(ModuleErrorKind::FileSystem, format!("I/O error: {}", error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::ModulePath;

    #[test]
    fn test_module_error_creation() {
        let error = ModuleError::not_found("test.module");
        assert_eq!(error.kind, ModuleErrorKind::NotFound);
        assert!(error.message.contains("test.module"));
        assert_eq!(error.module_path, Some("test.module".to_string()));
    }

    #[test]
    fn test_circular_dependency_error() {
        let mod_a = ModulePath::from_string("a").unwrap();
        let mod_b = ModulePath::from_string("b").unwrap();
        let mod_c = ModulePath::from_string("c").unwrap();

        let stack = vec![mod_a, mod_b.clone()];
        let error = ModuleError::circular_dependency(&stack, &mod_c);

        assert_eq!(error.kind, ModuleErrorKind::CircularDependency);
        assert!(error.message.contains("a -> b -> c"));
    }

    #[test]
    fn test_error_conversion() {
        let module_error = ModuleError::parse_error("test", "syntax error");
        let error: Error = module_error.into();

        assert_eq!(error.kind, ErrorKind::ParseError);
        assert!(error.message.contains("syntax error"));
    }

    #[test]
    fn test_error_chaining() {
        let error = ModuleError::invalid_path("invalid..path", "double dots not allowed")
            .with_module_path("test.module")
            .with_location(SourceLocation::new(1, 1, 0));

        assert_eq!(error.kind, ModuleErrorKind::InvalidPath);
        assert!(error.location.is_some());
        assert_eq!(error.module_path, Some("test.module".to_string()));
    }
}
