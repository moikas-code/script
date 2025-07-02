/// Package manifest and dependency management system for Script language
///
/// This module provides comprehensive package management functionality including:
/// - Package manifest parsing (script.toml)
/// - Dependency resolution with semantic versioning
/// - Package registry integration
/// - Local package caching and management
use serde::{Deserialize, Serialize};

mod cache;
mod dependency;
mod http_client;
mod manifest;
mod registry;
mod resolver;
mod version;

pub use cache::{CacheConfig, CacheEntry, CacheManager, PackageCache};
pub use dependency::{
    Dependency, DependencyGraph, DependencyKind, DependencyResolver, DependencySpec,
    ResolutionResult,
};
pub use manifest::{BinaryConfig, BuildConfig, LibraryConfig, PackageConfig, PackageManifest};
pub use registry::{PackageInfo, PackageRegistry, PublishResult, RegistryClient};
pub use resolver::{PackageResolver, PackageSource, ResolverConfig};
pub use version::{Version, VersionConstraint, VersionSpec};

use crate::error::Error;
use std::collections::HashMap;
use std::path::PathBuf;

/// Result type for package operations
pub type PackageResult<T> = Result<T, PackageError>;

/// Comprehensive error types for package management operations
#[derive(Debug, thiserror::Error)]
pub enum PackageError {
    #[error("Manifest parsing error: {0}")]
    ManifestParse(String),

    #[error("Dependency resolution failed: {0}")]
    DependencyResolution(String),

    #[error("Package not found: {name}")]
    PackageNotFound { name: String },

    #[error("Version constraint conflict: {constraint} for package {name}")]
    VersionConflict { name: String, constraint: String },

    #[error("Circular dependency detected: {cycle}")]
    CircularDependency { cycle: String },

    #[error("Registry error: {0}")]
    Registry(String),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parsing error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("Semantic version error: {0}")]
    SemVer(#[from] semver::Error),
}

impl From<PackageError> for Error {
    fn from(err: PackageError) -> Self {
        Error::package(err.to_string())
    }
}

/// Package metadata containing essential information about a package
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackageMetadata {
    pub name: String,
    pub version: Version,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
}

impl PackageMetadata {
    pub fn new(name: impl Into<String>, version: Version) -> Self {
        Self {
            name: name.into(),
            version,
            authors: Vec::new(),
            description: None,
            license: None,
            repository: None,
            homepage: None,
            documentation: None,
            keywords: Vec::new(),
            categories: Vec::new(),
        }
    }
}

/// Package project structure containing all project information
#[derive(Debug, Clone)]
pub struct Package {
    pub manifest: PackageManifest,
    pub root_path: PathBuf,
    pub source_files: Vec<PathBuf>,
    pub dependencies: DependencyGraph,
    pub lock_file: Option<LockFile>,
}

impl Package {
    /// Create a new package from a manifest file
    pub fn from_manifest_file(manifest_path: impl Into<PathBuf>) -> PackageResult<Self> {
        let manifest_path = manifest_path.into();
        let root_path = manifest_path
            .parent()
            .ok_or_else(|| PackageError::ManifestParse("Invalid manifest path".to_string()))?
            .to_path_buf();

        let manifest = PackageManifest::from_file(&manifest_path)?;
        let source_files = Self::discover_source_files(&root_path, &manifest)?;

        Ok(Self {
            manifest,
            root_path,
            source_files,
            dependencies: DependencyGraph::new(),
            lock_file: None,
        })
    }

    /// Discover all source files in the package
    fn discover_source_files(
        root: &PathBuf,
        manifest: &PackageManifest,
    ) -> PackageResult<Vec<PathBuf>> {
        let mut files = Vec::new();
        let src_dir = root.join("src");

        if src_dir.exists() {
            for entry in walkdir::WalkDir::new(&src_dir) {
                let entry = entry.map_err(|e| {
                    PackageError::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
                })?;
                if entry.path().extension().and_then(|s| s.to_str()) == Some("script") {
                    files.push(entry.path().to_path_buf());
                }
            }
        }

        // Add explicit library and binary paths from manifest
        if let Some(ref lib) = manifest.lib {
            let lib_path = root.join(&lib.path);
            if lib_path.exists() {
                files.push(lib_path);
            }
        }

        for bin in &manifest.bin {
            let bin_path = root.join(&bin.path);
            if bin_path.exists() {
                files.push(bin_path);
            }
        }

        Ok(files)
    }

    /// Get the main library entry point
    pub fn lib_entry_point(&self) -> Option<PathBuf> {
        self.manifest
            .lib
            .as_ref()
            .map(|lib| self.root_path.join(&lib.path))
    }

    /// Get all binary entry points
    pub fn bin_entry_points(&self) -> Vec<PathBuf> {
        self.manifest
            .bin
            .iter()
            .map(|bin| self.root_path.join(&bin.path))
            .collect()
    }

    /// Check if package is a workspace
    pub fn is_workspace(&self) -> bool {
        self.manifest.workspace.is_some()
    }
}

/// Lock file structure for dependency version locking
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LockFile {
    pub version: String,
    pub packages: Vec<LockEntry>,
    pub metadata: HashMap<String, String>,
}

/// Individual entry in the lock file
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LockEntry {
    pub name: String,
    pub version: String,
    pub source: String,
    pub checksum: Option<String>,
    pub dependencies: Vec<String>,
}

impl LockFile {
    /// Create a new lock file
    pub fn new() -> Self {
        Self {
            version: "1".to_string(),
            packages: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Load lock file from disk
    pub fn from_file(path: impl Into<PathBuf>) -> PackageResult<Self> {
        let path = path.into();
        let content = std::fs::read_to_string(&path)?;
        let lock_file: LockFile = toml::from_str(&content)?;
        Ok(lock_file)
    }

    /// Save lock file to disk
    pub fn save_to_file(&self, path: impl Into<PathBuf>) -> PackageResult<()> {
        let path = path.into();
        let content =
            toml::to_string_pretty(self).map_err(|e| PackageError::ManifestParse(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// Package manager for handling all package operations
pub struct PackageManager {
    cache: PackageCache,
    resolver: PackageResolver,
    registry: Box<dyn PackageRegistry>,
}

impl PackageManager {
    /// Create a new package manager with default configuration
    pub fn new() -> PackageResult<Self> {
        let cache = PackageCache::new()?;
        let resolver = PackageResolver::new(ResolverConfig::default());
        let registry = Box::new(RegistryClient::new("https://packages.script.org")?);

        Ok(Self {
            cache,
            resolver,
            registry,
        })
    }

    /// Create a package manager with custom configuration
    pub fn with_config(config: PackageManagerConfig) -> PackageResult<Self> {
        let cache = PackageCache::with_config(config.cache_config)?;
        let resolver = PackageResolver::new(config.resolver_config);
        let registry = Box::new(RegistryClient::new(&config.registry_url)?);

        Ok(Self {
            cache,
            resolver,
            registry,
        })
    }

    /// Resolve dependencies for a package
    pub fn resolve_dependencies(&mut self, package: &Package) -> PackageResult<DependencyGraph> {
        // For now, create a simple dependency graph without full resolution
        // In a complete implementation, this would use the actual resolver
        let mut graph = DependencyGraph::new();

        for (name, spec) in &package.manifest.dependencies {
            let dependency = spec.resolve(name)?;
            graph.add_dependency("root".to_string(), dependency);
        }

        graph.validate()?;
        graph.compute_build_order()?;
        Ok(graph)
    }

    /// Install dependencies for a package
    pub fn install_dependencies(&mut self, package: &mut Package) -> PackageResult<()> {
        let graph = self.resolve_dependencies(package)?;

        for (name, deps) in graph.dependencies() {
            if let Some(version) = graph.get_resolved_version(name) {
                for dep in deps {
                    self.install_dependency(name, dep, version)?;
                }
            }
        }

        package.dependencies = graph;
        Ok(())
    }

    /// Install a single dependency
    fn install_dependency(
        &mut self,
        name: &str,
        dependency: &Dependency,
        resolved_version: &Version,
    ) -> PackageResult<()> {
        if self.cache.has_package(name, resolved_version)? {
            return Ok(());
        }

        match &dependency.kind {
            DependencyKind::Registry => {
                let package_info = self.registry.get_package_info(name)?;
                self.cache.store_package(name, resolved_version, vec![])?;
            }
            DependencyKind::Git { url, rev, .. } => {
                // Git dependency installation logic
                todo!("Git dependency installation")
            }
            DependencyKind::Path { path } => {
                // Path dependency installation logic
                todo!("Path dependency installation")
            }
        }

        Ok(())
    }
}

/// Configuration for the package manager
#[derive(Debug, Clone)]
pub struct PackageManagerConfig {
    pub cache_config: CacheConfig,
    pub resolver_config: ResolverConfig,
    pub registry_url: String,
}

impl Default for PackageManagerConfig {
    fn default() -> Self {
        Self {
            cache_config: CacheConfig::default(),
            resolver_config: ResolverConfig::default(),
            registry_url: "https://packages.script.org".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_metadata_creation() {
        let version = Version::new(1, 0, 0);
        let metadata = PackageMetadata::new("test-package", version.clone());

        assert_eq!(metadata.name, "test-package");
        assert_eq!(metadata.version, version);
        assert!(metadata.authors.is_empty());
        assert!(metadata.description.is_none());
    }

    #[test]
    fn test_lock_file_serialization() {
        let mut lock_file = LockFile::new();
        lock_file.packages.push(LockEntry {
            name: "test-dep".to_string(),
            version: "1.0.0".to_string(),
            source: "registry".to_string(),
            checksum: Some("abcd1234".to_string()),
            dependencies: vec!["sub-dep".to_string()],
        });

        let serialized = toml::to_string(&lock_file).unwrap();
        let deserialized: LockFile = toml::from_str(&serialized).unwrap();

        assert_eq!(lock_file.packages.len(), deserialized.packages.len());
        assert_eq!(lock_file.packages[0].name, deserialized.packages[0].name);
    }
}
