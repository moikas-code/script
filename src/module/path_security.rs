//! Path security validation for module resolution
//!
//! This module provides comprehensive path validation to prevent
//! path traversal attacks, symlink exploits, and other path-based vulnerabilities.

use crate::module::{ModuleError, ModuleResult};
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

/// Path security validator with strict validation rules
#[derive(Debug)]
pub struct PathSecurityValidator {
    /// Absolute path to project root
    project_root: PathBuf,
    /// Maximum allowed path length
    max_path_length: usize,
    /// Allowed file extensions
    allowed_extensions: HashSet<String>,
    /// Forbidden path patterns
    forbidden_patterns: Vec<String>,
    /// Whether to follow symlinks
    allow_symlinks: bool,
}

impl PathSecurityValidator {
    /// Create a new path security validator
    pub fn new(project_root: PathBuf) -> ModuleResult<Self> {
        // Ensure project root is absolute and canonical
        let canonical_root = project_root.canonicalize().map_err(|e| {
            ModuleError::security_violation(format!("Failed to canonicalize project root: {}", e))
        })?;

        let mut allowed_extensions = HashSet::new();
        allowed_extensions.insert("script".to_string());
        allowed_extensions.insert("scripts".to_string());

        let forbidden_patterns = vec![
            "..".to_string(),
            "~".to_string(),
            "$".to_string(),
            "%".to_string(),
            "\\".to_string(),
        ];

        Ok(PathSecurityValidator {
            project_root: canonical_root,
            max_path_length: 255,
            allowed_extensions,
            forbidden_patterns,
            allow_symlinks: false,
        })
    }

    /// Validate a module path for security issues
    pub fn validate_module_path(&self, path: &str) -> ModuleResult<PathBuf> {
        // Step 1: Basic validation
        self.validate_path_format(path)?;

        // Step 2: Check for forbidden patterns
        self.check_forbidden_patterns(path)?;

        // Step 3: Construct safe path
        let safe_path = self.construct_safe_path(path)?;

        // Step 4: Validate file extension
        self.validate_extension(&safe_path)?;

        // Step 5: Ensure path stays within project bounds
        self.validate_within_bounds(&safe_path)?;

        // Step 6: Check symlink safety
        if !self.allow_symlinks {
            self.validate_no_symlinks(&safe_path)?;
        }

        Ok(safe_path)
    }

    /// Validate basic path format
    fn validate_path_format(&self, path: &str) -> ModuleResult<()> {
        // Check path length
        if path.is_empty() {
            return Err(ModuleError::security_violation("Empty module path"));
        }

        if path.len() > self.max_path_length {
            return Err(ModuleError::security_violation(format!(
                "Path too long: {} (max: {})",
                path.len(),
                self.max_path_length
            )));
        }

        // Reject absolute paths
        if path.starts_with('/') || path.starts_with('\\') {
            return Err(ModuleError::security_violation(
                "Absolute paths are not allowed",
            ));
        }

        // Check for null bytes
        if path.contains('\0') {
            return Err(ModuleError::security_violation("Path contains null bytes"));
        }

        // Validate characters
        for ch in path.chars() {
            match ch {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '.' | '/' => {}
                _ => {
                    return Err(ModuleError::security_violation(format!(
                        "Invalid character in path: '{}'",
                        ch
                    )))
                }
            }
        }

        Ok(())
    }

    /// Check for forbidden patterns
    fn check_forbidden_patterns(&self, path: &str) -> ModuleResult<()> {
        for pattern in &self.forbidden_patterns {
            if path.contains(pattern) {
                return Err(ModuleError::security_violation(format!(
                    "Forbidden pattern '{}' in path",
                    pattern
                )));
            }
        }

        // Additional checks for path traversal attempts
        let components: Vec<&str> = path.split('/').collect();
        for component in components {
            if component == ".." || component == "." {
                return Err(ModuleError::security_violation(
                    "Path traversal attempt detected",
                ));
            }

            if component.is_empty() {
                return Err(ModuleError::security_violation("Empty path component"));
            }
        }

        Ok(())
    }

    /// Construct a safe path within project bounds
    fn construct_safe_path(&self, path: &str) -> ModuleResult<PathBuf> {
        let mut safe_path = self.project_root.clone();

        // Parse path components safely
        let path_buf = PathBuf::from(path);
        for component in path_buf.components() {
            match component {
                Component::Normal(name) => {
                    safe_path.push(name);
                }
                _ => {
                    return Err(ModuleError::security_violation(format!(
                        "Invalid path component: {:?}",
                        component
                    )));
                }
            }
        }

        // Add .script extension if not present
        if safe_path.extension().is_none() {
            safe_path.set_extension("script");
        }

        Ok(safe_path)
    }

    /// Validate file extension
    fn validate_extension(&self, path: &Path) -> ModuleResult<()> {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| ModuleError::security_violation("Invalid or missing file extension"))?;

        if !self.allowed_extensions.contains(extension) {
            return Err(ModuleError::security_violation(format!(
                "Forbidden file extension: .{}",
                extension
            )));
        }

        Ok(())
    }

    /// Ensure path stays within project bounds
    fn validate_within_bounds(&self, path: &Path) -> ModuleResult<()> {
        // Canonicalize to resolve any remaining .. or symlinks
        let canonical = path.canonicalize().map_err(|e| {
            ModuleError::security_violation(format!("Failed to canonicalize path: {}", e))
        })?;

        // Check if canonical path is within project root
        if !canonical.starts_with(&self.project_root) {
            return Err(ModuleError::security_violation(
                "Path escapes project boundary",
            ));
        }

        Ok(())
    }

    /// Validate no symlinks in path
    fn validate_no_symlinks(&self, path: &Path) -> ModuleResult<()> {
        // Check each component for symlinks
        let mut current = PathBuf::new();
        for component in path.components() {
            current.push(component);

            if current.exists() && current.is_symlink() {
                return Err(ModuleError::security_violation(format!(
                    "Symlink detected in path: {:?}",
                    current
                )));
            }
        }

        Ok(())
    }

    /// Get a safe display path (relative to project root)
    pub fn safe_display_path(&self, path: &Path) -> String {
        if let Ok(relative) = path.strip_prefix(&self.project_root) {
            relative.display().to_string()
        } else {
            "<external>".to_string()
        }
    }
}

/// Module path sanitizer for safe path operations
pub struct ModulePathSanitizer;

impl ModulePathSanitizer {
    /// Sanitize a module identifier for use in paths
    pub fn sanitize_module_name(name: &str) -> ModuleResult<String> {
        if name.is_empty() {
            return Err(ModuleError::security_violation("Empty module name"));
        }

        if name.len() > 64 {
            return Err(ModuleError::security_violation(
                "Module name too long (max: 64 characters)",
            ));
        }

        let mut sanitized = String::with_capacity(name.len());
        for ch in name.chars() {
            match ch {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => sanitized.push(ch),
                '-' | '.' => sanitized.push('_'),
                _ => {
                    return Err(ModuleError::security_violation(format!(
                        "Invalid character in module name: '{}'",
                        ch
                    )))
                }
            }
        }

        // Check for reserved names
        let reserved = ["script", "mod", "lib", "bin", "test", "bench"];
        if reserved.contains(&sanitized.as_str()) {
            return Err(ModuleError::security_violation(format!(
                "Reserved module name: {}",
                sanitized
            )));
        }

        Ok(sanitized)
    }

    /// Convert module path segments to safe file path
    pub fn module_path_to_file_path(segments: &[String]) -> ModuleResult<String> {
        if segments.is_empty() {
            return Err(ModuleError::security_violation("Empty module path"));
        }

        let mut path_parts = Vec::with_capacity(segments.len());
        for segment in segments {
            let sanitized = Self::sanitize_module_name(segment)?;
            path_parts.push(sanitized);
        }

        Ok(path_parts.join("/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_path_traversal_protection() {
        let temp_dir = TempDir::new().unwrap();
        let validator = PathSecurityValidator::new(temp_dir.path().to_path_buf()).unwrap();

        // Test various path traversal attempts
        assert!(validator.validate_module_path("../etc/passwd").is_err());
        assert!(validator.validate_module_path("../../root").is_err());
        assert!(validator.validate_module_path("./../../etc").is_err());
        assert!(validator.validate_module_path("foo/../../../bar").is_err());
        assert!(validator.validate_module_path("foo/..\\..\\bar").is_err());
    }

    #[test]
    fn test_absolute_path_rejection() {
        let temp_dir = TempDir::new().unwrap();
        let validator = PathSecurityValidator::new(temp_dir.path().to_path_buf()).unwrap();

        assert!(validator.validate_module_path("/etc/passwd").is_err());
        assert!(validator
            .validate_module_path("\\windows\\system32")
            .is_err());
        assert!(validator.validate_module_path("C:\\malicious").is_err());
    }

    #[test]
    fn test_forbidden_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let validator = PathSecurityValidator::new(temp_dir.path().to_path_buf()).unwrap();

        assert!(validator.validate_module_path("~/.ssh/id_rsa").is_err());
        assert!(validator.validate_module_path("$HOME/secrets").is_err());
        assert!(validator.validate_module_path("%APPDATA%/config").is_err());
    }

    #[test]
    fn test_valid_module_paths() {
        let temp_dir = TempDir::new().unwrap();
        let validator = PathSecurityValidator::new(temp_dir.path().to_path_buf()).unwrap();

        // Create test directories
        fs::create_dir_all(temp_dir.path().join("src/utils")).unwrap();
        fs::write(temp_dir.path().join("src/utils/helpers.script"), "").unwrap();

        // Test valid paths
        assert!(validator.validate_module_path("src/utils/helpers").is_ok());
        assert!(validator.validate_module_path("lib/core").is_ok());
        assert!(validator.validate_module_path("app/main").is_ok());
    }

    #[test]
    fn test_module_name_sanitization() {
        assert!(ModulePathSanitizer::sanitize_module_name("valid_module123").is_ok());
        assert!(ModulePathSanitizer::sanitize_module_name("ValidModule").is_ok());

        assert!(ModulePathSanitizer::sanitize_module_name("../evil").is_err());
        assert!(ModulePathSanitizer::sanitize_module_name("evil$module").is_err());
        assert!(ModulePathSanitizer::sanitize_module_name("").is_err());
        assert!(ModulePathSanitizer::sanitize_module_name(&"x".repeat(100)).is_err());
    }
}
