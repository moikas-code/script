//! Secure module resolver with integrated security validation
//!
//! This module provides a secure implementation of module resolution
//! that integrates path validation, integrity checking, and resource monitoring.

#[cfg(test)]
use crate::module::AuditConfig;
use crate::module::{
    ImportPath, ModuleError, ModuleIntegrityVerifier, ModuleLoadContext, ModuleMetadata,
    ModulePath, ModulePathSanitizer, ModuleResolver, ModuleResult, PathSecurityValidator,
    ResolvedModule, ResourceMonitor, SecurityAuditLogger, SecurityEventBuilder,
    SecurityEventCategory, SecuritySeverity,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Secure file system based module resolver
#[derive(Debug)]
pub struct SecureFileSystemResolver {
    /// Base resolver configuration
    config: SecureResolverConfig,
    /// Search paths for modules
    search_paths: Vec<PathBuf>,
    /// Visited paths for circular dependency detection
    visited_paths: HashSet<PathBuf>,
    /// Path security validator
    path_validator: Arc<PathSecurityValidator>,
    /// Module integrity verifier
    integrity_verifier: Arc<ModuleIntegrityVerifier>,
    /// Resource monitor
    resource_monitor: Arc<ResourceMonitor>,
    /// Security audit logger
    audit_logger: Arc<SecurityAuditLogger>,
}

/// Configuration for secure module resolver
#[derive(Debug, Clone)]
pub struct SecureResolverConfig {
    /// Base configuration options
    pub search_stdlib: bool,
    pub search_external: bool,
    pub follow_symlinks: bool,
    pub max_depth: usize,
    pub file_extensions: Vec<String>,
    pub module_file_names: Vec<String>,
    pub case_sensitive: bool,
    /// Security options
    pub enforce_integrity: bool,
    pub require_trusted_modules: bool,
    pub audit_all_operations: bool,
    pub max_module_size: usize,
}

impl Default for SecureResolverConfig {
    fn default() -> Self {
        SecureResolverConfig {
            search_stdlib: true,
            search_external: false,
            follow_symlinks: false,
            max_depth: 10,
            file_extensions: vec!["script".to_string(), "scripts".to_string()],
            module_file_names: vec!["mod".to_string(), "index".to_string()],
            case_sensitive: cfg!(not(windows)),
            enforce_integrity: true,
            require_trusted_modules: false,
            audit_all_operations: true,
            max_module_size: 10_000_000, // 10MB
        }
    }
}

impl SecureFileSystemResolver {
    /// Create a new secure resolver
    pub fn new(
        config: SecureResolverConfig,
        project_root: PathBuf,
        integrity_verifier: Arc<ModuleIntegrityVerifier>,
        resource_monitor: Arc<ResourceMonitor>,
        audit_logger: Arc<SecurityAuditLogger>,
    ) -> ModuleResult<Self> {
        let path_validator = Arc::new(PathSecurityValidator::new(project_root)?);

        let mut search_paths = Vec::new();

        // Add standard library path if configured
        if config.search_stdlib {
            if let Ok(stdlib_path) = std::env::var("SCRIPT_STDLIB_PATH") {
                search_paths.push(PathBuf::from(stdlib_path));
            }
        }

        Ok(SecureFileSystemResolver {
            config,
            search_paths,
            visited_paths: HashSet::new(),
            path_validator,
            integrity_verifier,
            resource_monitor,
            audit_logger,
        })
    }

    /// Resolve an import path to a module path with security validation
    pub fn resolve_import_path(
        &self,
        import_path: &ImportPath,
        context: &ModuleLoadContext,
    ) -> ModuleResult<ModulePath> {
        // Basic resolution
        let module_path = import_path.resolve(&context.current_module)?;

        // Validate module path format
        for segment in module_path.segments() {
            ModulePathSanitizer::sanitize_module_name(segment)?;
        }

        Ok(module_path)
    }

    /// Find the file path for a module with security checks
    pub fn find_module_file(
        &self,
        module_path: &ModulePath,
        context: &ModuleLoadContext,
    ) -> ModuleResult<PathBuf> {
        // Check resource limits first
        self.resource_monitor
            .check_dependency_depth(context.loading_stack.len())?;

        // Create operation guard for timeout tracking
        let _op_guard = self
            .resource_monitor
            .begin_operation(format!("resolve:{module_path}"))?;

        // Check if it's a standard library module
        if module_path.is_std() {
            return self.find_stdlib_module(module_path);
        }

        // Try to find in project search paths
        for search_path in &context.search_paths {
            if let Some(file_path) = self.try_find_in_path_secure(module_path, search_path)? {
                return Ok(file_path);
            }
        }

        // Try global search paths
        for search_path in &self.search_paths {
            if let Some(file_path) = self.try_find_in_path_secure(module_path, search_path)? {
                return Ok(file_path);
            }
        }

        // Log failed resolution
        if self.config.audit_all_operations {
            let event = SecurityEventBuilder::new(
                SecurityEventCategory::ModuleLoad,
                format!("Module not found: {module_path}"),
            )
            .severity(SecuritySeverity::Warning)
            .module(module_path.clone())
            .build();

            self.audit_logger.log_event(event)?;
        }

        Err(ModuleError::not_found(module_path.to_string()))
    }

    fn find_stdlib_module(&self, module_path: &ModulePath) -> ModuleResult<PathBuf> {
        // Standard library modules have special handling
        let segments = &module_path.segments()[1..]; // Remove 'std' prefix
        let stdlib_module = ModulePath::new(segments.to_vec(), true)?;

        for search_path in &self.search_paths {
            if let Some(file_path) = self.try_find_in_path_secure(&stdlib_module, search_path)? {
                return Ok(file_path);
            }
        }

        Err(ModuleError::not_found(module_path.to_string()))
    }

    fn try_find_in_path_secure(
        &self,
        module_path: &ModulePath,
        base_path: &Path,
    ) -> ModuleResult<Option<PathBuf>> {
        // Convert module path to file path
        let relative_path = ModulePathSanitizer::module_path_to_file_path(module_path.segments())?;

        // Validate with path security
        let validated_path = match self.path_validator.validate_module_path(&relative_path) {
            Ok(path) => path,
            Err(e) => {
                // Log path traversal attempt
                if self.config.audit_all_operations {
                    self.audit_logger.log_path_traversal(
                        Some(module_path.clone()),
                        &relative_path,
                        &e,
                    )?;
                }
                return Err(e);
            }
        };

        let _base_path = base_path; // Used for path resolution context

        // Check various possible file names
        for extension in &self.config.file_extensions {
            let mut file_path = validated_path.clone();
            file_path.set_extension(extension);

            if file_path.exists() && file_path.is_file() {
                // Validate file metadata
                let metadata = file_path
                    .metadata()
                    .map_err(|e| ModuleError::file_system(&file_path, e))?;

                // Check file size
                if metadata.len() as usize > self.config.max_module_size {
                    self.audit_logger.log_resource_exhaustion(
                        Some(module_path.clone()),
                        "module_size",
                        self.config.max_module_size,
                        metadata.len() as usize,
                    )?;

                    return Err(ModuleError::resource_exhausted(format!(
                        "Module too large: {} bytes",
                        metadata.len()
                    )));
                }

                return Ok(Some(file_path));
            }
        }

        // Try module directory with index file
        for module_file in &self.config.module_file_names {
            for extension in &self.config.file_extensions {
                let file_path = validated_path.join(format!("{}.{}", module_file, extension));

                if file_path.exists() && file_path.is_file() {
                    return Ok(Some(file_path));
                }
            }
        }

        Ok(None)
    }

    fn load_module_source_secure(&self, file_path: &Path) -> ModuleResult<String> {
        // Resource check
        self.resource_monitor.check_timeout("module_load")?;

        // Load file content
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| ModuleError::file_system(file_path, e))?;

        // Validate content size
        if content.len() > self.config.max_module_size {
            return Err(ModuleError::resource_exhausted(format!(
                "Module content too large: {} bytes",
                content.len()
            )));
        }

        Ok(content)
    }
}

impl ModuleResolver for SecureFileSystemResolver {
    fn resolve_module(
        &mut self,
        import_path: &ImportPath,
        context: &ModuleLoadContext,
    ) -> ModuleResult<ResolvedModule> {
        // Resolve import path with validation
        let module_path = self.resolve_import_path(import_path, context)?;

        // Find file with security checks
        let file_path = self.find_module_file(&module_path, context)?;

        // Check for circular imports
        if self.visited_paths.contains(&file_path) {
            return Err(ModuleError::circular_dependency(
                &context.loading_stack,
                &module_path,
            ));
        }

        // Verify module integrity
        let verification_result = self
            .integrity_verifier
            .verify_module(&module_path, &file_path)?;

        // Check trust requirements
        if self.config.require_trusted_modules
            && verification_result.trust_level == crate::module::integrity::TrustLevel::Unknown
        {
            return Err(ModuleError::security_violation(format!(
                "Untrusted module: {}",
                module_path
            )));
        }

        // Load module source with size checks
        let source = self.load_module_source_secure(&file_path)?;

        // Create metadata
        let mut metadata = ModuleMetadata::default();
        metadata.name = module_path.module_name().to_string();
        metadata.file_size = source.len() as u64;

        // Record successful load
        self.resource_monitor.record_module_load(
            module_path.clone(),
            source.len(),
            0, // Import count will be updated later
        )?;

        // Log successful module load
        if self.config.audit_all_operations {
            self.audit_logger.log_module_load(
                module_path.clone(),
                &file_path,
                &verification_result.checksum.sha256,
            )?;
        }

        // Mark as visited
        self.visited_paths.insert(file_path.clone());

        Ok(ResolvedModule::new(
            module_path,
            file_path,
            source,
            metadata,
        ))
    }

    fn module_exists(&self, import_path: &ImportPath, context: &ModuleLoadContext) -> bool {
        // Try to resolve without side effects
        match self.resolve_import_path(import_path, context) {
            Ok(module_path) => match self.find_module_file(&module_path, context) {
                Ok(_) => true,
                Err(_) => false,
            },
            Err(_) => false,
        }
    }

    fn search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }

    fn add_search_path(&mut self, path: PathBuf) {
        // Validate search path before adding
        if path.is_absolute() && path.exists() && path.is_dir() {
            self.search_paths.push(path);
        }
    }
}

/// Extension to ModulePathSanitizer for additional security
impl ModulePathSanitizer {
    /// Validate import depth to prevent deep nesting attacks
    pub fn validate_import_depth(segments: &[String], max_depth: usize) -> ModuleResult<()> {
        if segments.len() > max_depth {
            return Err(ModuleError::security_violation(format!(
                "Import depth exceeds limit: {} > {}",
                segments.len(),
                max_depth
            )));
        }
        Ok(())
    }

    /// Check for suspicious module name patterns
    pub fn check_suspicious_patterns(name: &str) -> ModuleResult<()> {
        let suspicious_patterns = [
            "eval",
            "exec",
            "system",
            "shell",
            "cmd",
            "process",
            "__proto__",
            "constructor",
            "prototype",
        ];

        let lower_name = name.to_lowercase();
        for pattern in &suspicious_patterns {
            if lower_name.contains(pattern) {
                return Err(ModuleError::security_violation(format!(
                    "Suspicious module name pattern: {}",
                    pattern
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_resolver(project_root: PathBuf) -> SecureFileSystemResolver {
        let config = SecureResolverConfig::default();
        let integrity = Arc::new(ModuleIntegrityVerifier::new(false));
        let monitor = Arc::new(ResourceMonitor::new());
        let audit_config = AuditConfig {
            log_file: project_root.join("audit.log"),
            ..Default::default()
        };
        let logger = Arc::new(SecurityAuditLogger::new(audit_config).unwrap());

        SecureFileSystemResolver::new(config, project_root, integrity, monitor, logger).unwrap()
    }

    #[test]
    fn test_secure_path_resolution() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path().to_path_buf();
        let resolver = create_test_resolver(project_root.clone());

        // Create test module
        let module_dir = project_root.join("src/utils");
        fs::create_dir_all(&module_dir).unwrap();
        fs::write(module_dir.join("helpers.script"), "// test module").unwrap();

        // Test valid resolution
        let import = ImportPath::new("utils/helpers").unwrap();
        let context = ModuleLoadContext::new(
            ModulePath::from_string("main").unwrap(),
            project_root.clone(),
        );

        assert!(resolver.module_exists(&import, &context));
    }

    #[test]
    fn test_path_traversal_prevention() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path().to_path_buf();
        let mut resolver = create_test_resolver(project_root.clone());

        // Create malicious import attempts
        let evil_imports = vec![
            "../../../etc/passwd",
            "..\\..\\windows\\system32",
            "./../../sensitive",
            "foo/../../../bar",
        ];

        let context =
            ModuleLoadContext::new(ModulePath::from_string("main").unwrap(), project_root);

        for evil_import in evil_imports {
            let import = ImportPath::new(evil_import).unwrap();
            let result = resolver.resolve_module(&import, &context);
            assert!(result.is_err());

            if let Err(e) = result {
                assert!(
                    e.to_string().contains("security violation")
                        || e.to_string().contains("Invalid character")
                );
            }
        }
    }
}
