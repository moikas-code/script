mod audit;
mod cache;
mod context;
mod error;
mod integration;
mod integrity;
mod path;
mod path_security;
mod permissions;
mod registry;
mod resolver;
mod resource_monitor;
mod sandbox;
mod secure_resolver;
mod security;

#[cfg(test)]
mod tests;

pub use audit::{
    AuditConfig, AuditStatistics, SecurityAuditEvent, SecurityAuditLogger, SecurityEventBuilder,
    SecurityEventCategory, SecurityEventContext, SecuritySeverity,
};
pub use cache::{CacheEntry, ModuleCache};
pub use context::{
    ImportResolutionStep, ModuleContext, ModuleContextStack, ModuleDependencyChain,
    PrivateAccessAttempt, VisibilityContext,
};
pub use error::{ModuleError, ModuleErrorKind, ModuleResult};
pub use integration::{
    create_default_pipeline, CompilationConfig, CompiledModule, ExportVisibility,
    FunctionExportInfo, ModuleCompilationPipeline, ModuleExports, ReExportInfo, TypeDefinitionInfo,
    TypeDefinitionKind, VariableExportInfo,
};
pub use integrity::{
    ModuleChecksum, ModuleIntegrityVerifier, ModuleLockFile, ModuleSignature as IntegritySignature,
    TrustLevel as IntegrityTrustLevel, TrustedModuleEntry, TrustedModuleRegistry,
    VerificationRequirements, VerificationResult,
};
pub use path::{ImportPath, ModulePath, RelativePath};
pub use path_security::{ModulePathSanitizer, PathSecurityValidator};
pub use permissions::{
    EnvVarPattern, FFIPermission, FileSystemPermission, HostPattern, LibraryPattern,
    ModuleInteractionPermission, ModulePattern, ModulePermissions, NetworkPermission, PathPattern,
    Permission, PermissionAuditEntry, PermissionContext, PermissionManager, PermissionRule,
    PortRange, ProcessPattern, ProcessPermission, ResourcePermission,
};
pub use registry::{ModuleMetadata, ModuleRegistry, RegistryConfig};
pub use resolver::{FileSystemResolver, ModuleResolver, ModuleResolverConfig};
pub use resource_monitor::{
    ModuleResourceUsage, OperationGuard, ResourceLimits as ResourceMonitorLimits, ResourceMonitor,
    ResourceUsage as ResourceMonitorUsage, ResourceUsageSummary,
};
pub use sandbox::{ExecutionTrace, ModuleSandbox, ResourceUsage, SandboxConfig};
pub use secure_resolver::{SecureFileSystemResolver, SecureResolverConfig};
pub use security::{
    ModuleCapability, ModuleSecurityContext, ModuleSecurityManager, ModuleSignature,
    ResourceLimits, TrustLevel,
};

use crate::source::SourceLocation;
use std::path::PathBuf;

/// Represents a resolved module with its metadata and location
#[derive(Debug, Clone)]
pub struct ResolvedModule {
    pub path: ModulePath,
    pub file_path: PathBuf,
    pub source: String,
    pub metadata: ModuleMetadata,
    pub dependencies: Vec<ImportPath>,
}

impl ResolvedModule {
    pub fn new(
        path: ModulePath,
        file_path: PathBuf,
        source: String,
        metadata: ModuleMetadata,
    ) -> Self {
        Self {
            path,
            file_path,
            source,
            metadata,
            dependencies: Vec::new(),
        }
    }

    pub fn add_dependency(&mut self, dependency: ImportPath) {
        self.dependencies.push(dependency);
    }

    pub fn is_library_module(&self) -> bool {
        self.path.is_std() || self.path.is_external()
    }

    pub fn is_local_module(&self) -> bool {
        !self.is_library_module()
    }
}

/// Module loading context for tracking the current resolution state
#[derive(Debug, Clone)]
pub struct ModuleLoadContext {
    pub current_module: ModulePath,
    pub search_paths: Vec<PathBuf>,
    pub loading_stack: Vec<ModulePath>,
    pub package_root: PathBuf,
}

impl ModuleLoadContext {
    pub fn new(current_module: ModulePath, package_root: PathBuf) -> Self {
        let mut search_paths = vec![package_root.join("src")];

        // Add standard library path if it exists
        if let Ok(stdlib_path) = std::env::var("SCRIPT_STDLIB_PATH") {
            search_paths.push(PathBuf::from(stdlib_path));
        }

        Self {
            current_module,
            search_paths,
            loading_stack: vec![],
            package_root,
        }
    }

    pub fn push_loading(&mut self, module: ModulePath) -> ModuleResult<()> {
        if self.loading_stack.contains(&module) {
            return Err(ModuleError::circular_dependency(
                &self.loading_stack,
                &module,
            ));
        }
        self.loading_stack.push(module);
        Ok(())
    }

    pub fn pop_loading(&mut self) {
        self.loading_stack.pop();
    }

    pub fn with_current_module(&self, module: ModulePath) -> Self {
        let mut context = self.clone();
        context.current_module = module;
        context
    }
}

/// Import specification with optional aliasing and selective imports
#[derive(Debug, Clone, PartialEq)]
pub struct ImportSpec {
    pub path: ImportPath,
    pub alias: Option<String>,
    pub items: ImportItems,
    pub location: SourceLocation,
}

/// Different types of import items
#[derive(Debug, Clone, PartialEq)]
pub enum ImportItems {
    All,                             // import foo.*
    Default,                         // import foo
    Selective(Vec<SelectiveImport>), // import foo.{ bar, baz as qux }
}

/// Selective import with optional aliasing
#[derive(Debug, Clone, PartialEq)]
pub struct SelectiveImport {
    pub name: String,
    pub alias: Option<String>,
}

impl SelectiveImport {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            alias: None,
        }
    }

    pub fn with_alias(name: impl Into<String>, alias: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            alias: Some(alias.into()),
        }
    }

    pub fn effective_name(&self) -> &str {
        self.alias.as_ref().unwrap_or(&self.name)
    }
}
