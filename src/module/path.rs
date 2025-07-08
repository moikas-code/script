use crate::module::{ModuleError, ModuleResult};
use crate::error::{Error, Result};
use std::fmt;
use std::path::{Path, PathBuf};

/// Represents a fully qualified module path
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModulePath {
    segments: Vec<String>,
    is_absolute: bool,
}

impl ModulePath {
    /// Create a new module path from segments
    pub fn new(segments: Vec<String>, is_absolute: bool) -> ModuleResult<Self> {
        if segments.is_empty() {
            return Err(ModuleError::invalid_path("", "empty module path"));
        }

        for segment in &segments {
            if segment.is_empty() {
                return Err(ModuleError::invalid_path(
                    segments.join("."),
                    "empty segment in module path",
                ));
            }

            if !is_valid_identifier(segment) {
                return Err(ModuleError::invalid_path(
                    segments.join("."),
                    format!("invalid identifier '{}'", segment),
                ));
            }
        }

        Ok(Self {
            segments,
            is_absolute,
        })
    }

    /// Parse a module path from a string
    pub fn from_string(path: impl AsRef<str>) -> ModuleResult<Self> {
        let path_str = path.as_ref();

        if path_str.is_empty() {
            return Err(ModuleError::invalid_path(path_str, "empty module path"));
        }

        let is_absolute = !path_str.starts_with("./") && !path_str.starts_with("../");
        let segments: Vec<String> = path_str.split('.').map(|s| s.to_string()).collect();

        Self::new(segments, is_absolute)
    }

    /// Create a standard library module path
    pub fn std_module(path: impl AsRef<str>) -> ModuleResult<Self> {
        let path_str = format!("std.{}", path.as_ref());
        Self::from_string(path_str)
    }

    /// Get the segments of the module path
    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    /// Check if this is an absolute path
    pub fn is_absolute(&self) -> bool {
        self.is_absolute
    }

    /// Check if this is a standard library module
    pub fn is_std(&self) -> bool {
        self.segments.first().map_or(false, |s| s == "std")
    }

    /// Check if this is an external package module
    pub fn is_external(&self) -> bool {
        self.is_absolute && !self.is_std() && !self.is_local_project()
    }

    /// Check if this is a local project module
    pub fn is_local_project(&self) -> bool {
        // For now, consider anything not std as local project
        // This can be enhanced with package registry information
        !self.is_std()
    }

    /// Get the package name (first segment for external modules)
    pub fn package_name(&self) -> Option<&str> {
        if self.is_external() {
            self.segments.first().map(|s| s.as_str())
        } else {
            None
        }
    }

    /// Get the module name (last segment)
    pub fn module_name(&self) -> &str {
        self.segments.last().expect("Module path should have at least one segment - this is guaranteed by constructor validation")
    }

    /// Get the parent module path
    pub fn parent(&self) -> Option<ModulePath> {
        if self.segments.len() <= 1 {
            return None;
        }

        let parent_segments = self.segments[..self.segments.len() - 1].to_vec();
        Some(ModulePath {
            segments: parent_segments,
            is_absolute: self.is_absolute,
        })
    }

    /// Join with another path segment
    pub fn join(&self, segment: impl AsRef<str>) -> ModuleResult<ModulePath> {
        let segment_str = segment.as_ref();
        if !is_valid_identifier(segment_str) {
            return Err(ModuleError::invalid_path(
                format!("{}.{}", self, segment_str),
                format!("invalid identifier '{}'", segment_str),
            ));
        }

        let mut new_segments = self.segments.clone();
        new_segments.push(segment_str.to_string());

        Ok(ModulePath {
            segments: new_segments,
            is_absolute: self.is_absolute,
        })
    }

    /// Convert to file path within a source directory
    pub fn to_file_path(&self, base_dir: &Path) -> PathBuf {
        let mut path = base_dir.to_path_buf();

        for segment in &self.segments {
            path = path.join(segment);
        }

        path.with_extension("script")
    }

    /// Convert to directory module path (mod.script)
    pub fn to_dir_module_path(&self, base_dir: &Path) -> PathBuf {
        let mut path = base_dir.to_path_buf();

        for segment in &self.segments {
            path = path.join(segment);
        }

        path.join("mod.script")
    }

    /// Get all possible file paths for this module
    pub fn possible_file_paths(&self, base_dir: &Path) -> Vec<PathBuf> {
        vec![
            self.to_file_path(base_dir),
            self.to_dir_module_path(base_dir),
        ]
    }
}

impl fmt::Display for ModulePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.segments.join("."))
    }
}

/// Represents an import path, which can be relative or absolute
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImportPath {
    pub kind: ImportKind,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImportKind {
    Absolute, // foo.bar
    Relative, // ./foo or ../foo
    Super,    // super.foo
    Crate,    // crate.foo
    Self_,    // self.foo
}

impl ImportPath {
    pub fn new(path: impl Into<String>) -> ModuleResult<Self> {
        let path_str = path.into();

        if path_str.is_empty() {
            return Err(ModuleError::invalid_path(&path_str, "empty import path"));
        }

        let kind = if path_str.starts_with("./") {
            ImportKind::Relative
        } else if path_str.starts_with("../") {
            ImportKind::Relative
        } else if path_str.starts_with("super.") {
            ImportKind::Super
        } else if path_str.starts_with("crate.") {
            ImportKind::Crate
        } else if path_str.starts_with("self.") {
            ImportKind::Self_
        } else {
            ImportKind::Absolute
        };

        Ok(Self {
            kind,
            path: path_str,
        })
    }

    /// Resolve this import path relative to a current module
    pub fn resolve(&self, current_module: &ModulePath) -> ModuleResult<ModulePath> {
        match &self.kind {
            ImportKind::Absolute => ModulePath::from_string(&self.path),
            ImportKind::Relative => self.resolve_relative(current_module),
            ImportKind::Super => self.resolve_super(current_module),
            ImportKind::Crate => self.resolve_crate(),
            ImportKind::Self_ => self.resolve_self(current_module),
        }
    }

    fn resolve_relative(&self, current_module: &ModulePath) -> ModuleResult<ModulePath> {
        let path = &self.path;
        let mut current = current_module
            .parent()
            .unwrap_or_else(|| ModulePath::new(vec!["root".to_string()], true)
                .expect("Failed to create fallback root module path"));

        if path.starts_with("./") {
            let relative_part = &path[2..];
            if !relative_part.is_empty() {
                return current.join(relative_part);
            }
            return Ok(current);
        }

        if path.starts_with("../") {
            let mut remaining = path.as_str();
            while remaining.starts_with("../") {
                remaining = &remaining[3..];
                current = current
                    .parent()
                    .unwrap_or_else(|| ModulePath::new(vec!["root".to_string()], true)
                        .expect("Failed to create fallback root module path"));
            }

            if !remaining.is_empty() {
                return current.join(remaining);
            }
            return Ok(current);
        }

        Err(ModuleError::invalid_path(path, "invalid relative path"))
    }

    fn resolve_super(&self, current_module: &ModulePath) -> ModuleResult<ModulePath> {
        if let Some(parent) = current_module.parent() {
            let super_path = &self.path[6..]; // Remove "super."
            if super_path.is_empty() {
                Ok(parent)
            } else {
                parent.join(super_path)
            }
        } else {
            Err(ModuleError::invalid_path(
                &self.path,
                "no parent module for super",
            ))
        }
    }

    fn resolve_crate(&self) -> ModuleResult<ModulePath> {
        let crate_path = &self.path[6..]; // Remove "crate."
        ModulePath::from_string(crate_path)
    }

    fn resolve_self(&self, current_module: &ModulePath) -> ModuleResult<ModulePath> {
        let self_path = &self.path[5..]; // Remove "self."
        if self_path.is_empty() {
            Ok(current_module.clone())
        } else {
            current_module.join(self_path)
        }
    }
}

impl fmt::Display for ImportPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path)
    }
}

/// Relative path utilities for path resolution
#[derive(Debug, Clone)]
pub struct RelativePath;

impl RelativePath {
    /// Normalize a path by resolving . and .. components
    pub fn normalize(path: &Path) -> PathBuf {
        let mut components = Vec::new();

        for component in path.components() {
            match component {
                std::path::Component::Normal(name) => {
                    components.push(name);
                }
                std::path::Component::ParentDir => {
                    if !components.is_empty() {
                        components.pop();
                    }
                }
                std::path::Component::CurDir => {
                    // Skip current directory references
                }
                _ => {
                    // Preserve other components (root, prefix, etc.)
                    return path.to_path_buf();
                }
            }
        }

        components.into_iter().collect()
    }

    /// Check if a path is within a base directory
    pub fn is_within(path: &Path, base: &Path) -> bool {
        let normalized_path = Self::normalize(path);
        let normalized_base = Self::normalize(base);

        normalized_path.starts_with(normalized_base)
    }
}

/// Check if a string is a valid identifier for module segments
fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let mut chars = s.chars();

    // First character must be letter or underscore
    if let Some(first) = chars.next() {
        if !first.is_ascii_alphabetic() && first != '_' {
            return false;
        }
    }

    // Remaining characters must be alphanumeric or underscore
    for ch in chars {
        if !ch.is_ascii_alphanumeric() && ch != '_' {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_path_creation() {
        let path = ModulePath::from_string("foo.bar.baz").unwrap();
        assert_eq!(path.segments(), ["foo", "bar", "baz"]);
        assert!(path.is_absolute());
        assert!(!path.is_std());
        assert_eq!(path.module_name(), "baz");
    }

    #[test]
    fn test_std_module_path() {
        let path = ModulePath::std_module("collections.HashMap").unwrap();
        assert!(path.is_std());
        assert_eq!(path.segments(), ["std", "collections", "HashMap"]);
    }

    #[test]
    fn test_module_path_parent() {
        let path = ModulePath::from_string("foo.bar.baz").unwrap();
        let parent = path.parent().unwrap();
        assert_eq!(parent.segments(), ["foo", "bar"]);

        let root = ModulePath::from_string("foo").unwrap();
        assert!(root.parent().is_none());
    }

    #[test]
    fn test_module_path_join() {
        let path = ModulePath::from_string("foo.bar").unwrap();
        let joined = path.join("baz").unwrap();
        assert_eq!(joined.segments(), ["foo", "bar", "baz"]);
    }

    #[test]
    fn test_import_path_absolute() {
        let import = ImportPath::new("foo.bar").unwrap();
        assert_eq!(import.kind, ImportKind::Absolute);

        let current = ModulePath::from_string("current.module").unwrap();
        let resolved = import.resolve(&current).unwrap();
        assert_eq!(resolved.segments(), ["foo", "bar"]);
    }

    #[test]
    fn test_import_path_relative() {
        let import = ImportPath::new("./sibling").unwrap();
        assert_eq!(import.kind, ImportKind::Relative);

        let current = ModulePath::from_string("parent.current").unwrap();
        let resolved = import.resolve(&current).unwrap();
        assert_eq!(resolved.segments(), ["parent", "sibling"]);
    }

    #[test]
    fn test_import_path_super() {
        let import = ImportPath::new("super.sibling").unwrap();
        assert_eq!(import.kind, ImportKind::Super);

        let current = ModulePath::from_string("grandparent.parent.current").unwrap();
        let resolved = import.resolve(&current).unwrap();
        assert_eq!(resolved.segments(), ["grandparent", "parent", "sibling"]);
    }

    #[test]
    fn test_file_path_conversion() {
        let path = ModulePath::from_string("foo.bar").unwrap();
        let base = Path::new("/src");

        let file_path = path.to_file_path(base);
        assert_eq!(file_path, PathBuf::from("/src/foo/bar.script"));

        let dir_path = path.to_dir_module_path(base);
        assert_eq!(dir_path, PathBuf::from("/src/foo/bar/mod.script"));
    }

    #[test]
    fn test_invalid_identifier() {
        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("123"));
        assert!(!is_valid_identifier("foo-bar"));
        assert!(!is_valid_identifier("foo.bar"));

        assert!(is_valid_identifier("foo"));
        assert!(is_valid_identifier("_private"));
        assert!(is_valid_identifier("foo123"));
        assert!(is_valid_identifier("_"));
    }

    #[test]
    fn test_relative_path_normalize() {
        let path = Path::new("foo/../bar/./baz");
        let normalized = RelativePath::normalize(path);
        assert_eq!(normalized, PathBuf::from("bar/baz"));
    }
}
