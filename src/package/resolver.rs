/// Package resolution and source management for Script packages
///
/// This module handles:
/// - Package source resolution (registry, git, path)
/// - Package downloading and caching
/// - Source verification and integrity checks
use super::{Dependency, DependencyKind, PackageError, PackageMetadata, PackageResult, Version};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Package resolver for handling different package sources
pub struct PackageResolver {
    config: ResolverConfig,
    sources: HashMap<String, Box<dyn PackageSource>>,
}

impl PackageResolver {
    /// Create a new package resolver with configuration
    pub fn new(config: ResolverConfig) -> Self {
        let mut resolver = Self {
            config,
            sources: HashMap::new(),
        };

        // Register default sources
        resolver.register_source("registry", Box::new(RegistrySource::new()));
        resolver.register_source("git", Box::new(GitSource::new()));
        resolver.register_source("path", Box::new(PathSource::new()));

        resolver
    }

    /// Register a package source
    pub fn register_source(&mut self, name: impl Into<String>, source: Box<dyn PackageSource>) {
        self.sources.insert(name.into(), source);
    }

    /// Resolve a package from its dependency specification
    pub fn resolve_package(&self, dependency: &Dependency) -> PackageResult<ResolvedPackage> {
        match &dependency.kind {
            DependencyKind::Registry => {
                let source = self.sources.get("registry").ok_or_else(|| {
                    PackageError::DependencyResolution("Registry source not available".to_string())
                })?;
                source.resolve_package(dependency)
            }
            DependencyKind::Git { .. } => {
                let source = self.sources.get("git").ok_or_else(|| {
                    PackageError::DependencyResolution("Git source not available".to_string())
                })?;
                source.resolve_package(dependency)
            }
            DependencyKind::Path { .. } => {
                let source = self.sources.get("path").ok_or_else(|| {
                    PackageError::DependencyResolution("Path source not available".to_string())
                })?;
                source.resolve_package(dependency)
            }
        }
    }

    /// Download a package to local cache
    pub fn download_package(
        &self,
        dependency: &Dependency,
        cache_dir: &Path,
    ) -> PackageResult<PathBuf> {
        let resolved = self.resolve_package(dependency)?;
        let package_dir = cache_dir.join(&resolved.cache_key());

        if package_dir.exists() && self.verify_package(&package_dir, &resolved)? {
            return Ok(package_dir);
        }

        std::fs::create_dir_all(&package_dir)?;

        match &dependency.kind {
            DependencyKind::Registry => {
                self.download_registry_package(&resolved, &package_dir)?;
            }
            DependencyKind::Git { .. } => {
                self.download_git_package(&resolved, &package_dir)?;
            }
            DependencyKind::Path { path } => {
                self.copy_path_package(path, &package_dir)?;
            }
        }

        Ok(package_dir)
    }

    fn download_registry_package(
        &self,
        resolved: &ResolvedPackage,
        target_dir: &Path,
    ) -> PackageResult<()> {
        // In a real implementation, this would download from the registry
        // For now, we'll simulate it
        println!(
            "Downloading {} {} from registry",
            resolved.name, resolved.version
        );

        // Create a basic package structure
        std::fs::create_dir_all(target_dir.join("src"))?;
        std::fs::write(
            target_dir.join("script.toml"),
            format!(
                r#"[package]
name = "{}"
version = "{}"
"#,
                resolved.name, resolved.version
            ),
        )?;
        std::fs::write(
            target_dir.join("src").join("lib.script"),
            "// Auto-generated library file\n",
        )?;

        Ok(())
    }

    fn download_git_package(
        &self,
        resolved: &ResolvedPackage,
        target_dir: &Path,
    ) -> PackageResult<()> {
        // In a real implementation, this would clone/pull from git
        println!("Cloning {} from git", resolved.source_url);

        // For now, create a placeholder
        std::fs::create_dir_all(target_dir.join("src"))?;
        std::fs::write(
            target_dir.join("script.toml"),
            format!(
                r#"[package]
name = "{}"
version = "{}"
"#,
                resolved.name, resolved.version
            ),
        )?;
        std::fs::write(
            target_dir.join("src").join("lib.script"),
            "// Git package placeholder\n",
        )?;

        Ok(())
    }

    fn copy_path_package(&self, source_path: &Path, target_dir: &Path) -> PackageResult<()> {
        // Copy the entire directory structure
        self.copy_dir_recursive(source_path, target_dir)?;
        Ok(())
    }

    fn copy_dir_recursive(&self, src: &Path, dst: &Path) -> PackageResult<()> {
        if !src.exists() {
            return Err(PackageError::DependencyResolution(format!(
                "Source path does not exist: {}",
                src.display()
            )));
        }

        if src.is_file() {
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(src, dst)?;
            return Ok(());
        }

        std::fs::create_dir_all(dst)?;

        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                self.copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                std::fs::copy(&src_path, &dst_path)?;
            }
        }

        Ok(())
    }

    fn verify_package(
        &self,
        package_dir: &Path,
        resolved: &ResolvedPackage,
    ) -> PackageResult<bool> {
        // Verify package integrity using checksums
        if let Some(expected_checksum) = &resolved.checksum {
            let actual_checksum = self.compute_package_checksum(package_dir)?;
            Ok(actual_checksum == *expected_checksum)
        } else {
            // If no checksum is available, just check if the manifest exists
            Ok(package_dir.join("script.toml").exists())
        }
    }

    fn compute_package_checksum(&self, package_dir: &Path) -> PackageResult<String> {
        let mut hasher = Sha256::new();
        self.hash_directory(&mut hasher, package_dir)?;
        Ok(format!("{:x}", hasher.finalize()))
    }

    fn hash_directory(&self, hasher: &mut Sha256, dir: &Path) -> PackageResult<()> {
        let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let path = entry.path();
            let file_name = entry.file_name();

            hasher.update(file_name.to_string_lossy().as_bytes());

            if path.is_file() {
                let content = std::fs::read(&path)?;
                hasher.update(&content);
            } else if path.is_dir() {
                self.hash_directory(hasher, &path)?;
            }
        }

        Ok(())
    }
}

/// Configuration for the package resolver
#[derive(Debug, Clone)]
pub struct ResolverConfig {
    pub registry_url: String,
    pub cache_dir: PathBuf,
    pub timeout_seconds: u64,
    pub verify_checksums: bool,
    pub allow_insecure: bool,
    pub proxy_url: Option<String>,
    pub user_agent: String,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("script")
            .join("packages");

        Self {
            registry_url: "https://packages.script.org".to_string(),
            cache_dir,
            timeout_seconds: 30,
            verify_checksums: true,
            allow_insecure: false,
            proxy_url: None,
            user_agent: format!("script/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

/// Resolved package information
#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    pub name: String,
    pub version: Version,
    pub source_url: String,
    pub checksum: Option<String>,
    pub metadata: PackageMetadata,
}

impl ResolvedPackage {
    pub fn new(name: impl Into<String>, version: Version, source_url: impl Into<String>) -> Self {
        let name_str = name.into();
        Self {
            name: name_str.clone(),
            version: version.clone(),
            source_url: source_url.into(),
            checksum: None,
            metadata: PackageMetadata::new(name_str, version),
        }
    }

    pub fn with_checksum(mut self, checksum: impl Into<String>) -> Self {
        self.checksum = Some(checksum.into());
        self
    }

    pub fn with_metadata(mut self, metadata: PackageMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Generate a cache key for this package
    pub fn cache_key(&self) -> String {
        format!("{}-{}", self.name, self.version)
    }
}

/// Trait for package sources (registry, git, path, etc.)
pub trait PackageSource: Send + Sync {
    /// Resolve a package from a dependency
    fn resolve_package(&self, dependency: &Dependency) -> PackageResult<ResolvedPackage>;

    /// Check if a package exists
    fn package_exists(&self, name: &str, version: &Version) -> PackageResult<bool>;

    /// Get available versions for a package
    fn get_versions(&self, name: &str) -> PackageResult<Vec<Version>>;
}

/// Registry-based package source
pub struct RegistrySource {
    base_url: String,
}

impl RegistrySource {
    pub fn new() -> Self {
        Self {
            base_url: "https://packages.script.org".to_string(),
        }
    }

    pub fn with_url(url: impl Into<String>) -> Self {
        Self {
            base_url: url.into(),
        }
    }
}

impl PackageSource for RegistrySource {
    fn resolve_package(&self, dependency: &Dependency) -> PackageResult<ResolvedPackage> {
        // In a real implementation, this would query the registry API
        let version = Version::new(1, 0, 0); // Placeholder
        let source_url = format!("{}/packages/{}/{}", self.base_url, dependency.name, version);

        Ok(ResolvedPackage::new(
            dependency.name.clone(),
            version,
            source_url,
        ))
    }

    fn package_exists(&self, _name: &str, _version: &Version) -> PackageResult<bool> {
        // In a real implementation, this would check the registry
        Ok(true) // Placeholder
    }

    fn get_versions(&self, _name: &str) -> PackageResult<Vec<Version>> {
        // In a real implementation, this would fetch from registry
        Ok(vec![
            Version::new(1, 0, 0),
            Version::new(1, 1, 0),
            Version::new(2, 0, 0),
        ]) // Placeholder
    }
}

/// Git-based package source
pub struct GitSource;

impl GitSource {
    pub fn new() -> Self {
        Self
    }
}

impl PackageSource for GitSource {
    fn resolve_package(&self, dependency: &Dependency) -> PackageResult<ResolvedPackage> {
        if let DependencyKind::Git {
            url,
            branch,
            tag,
            rev,
        } = &dependency.kind
        {
            let mut source_url = url.clone();

            if let Some(branch) = branch {
                source_url.push_str(&format!("#branch={branch}"));
            } else if let Some(tag) = tag {
                source_url.push_str(&format!("#tag={tag}"));
            } else if let Some(rev) = rev {
                source_url.push_str(&format!("#rev={rev}"));
            }

            Ok(ResolvedPackage::new(
                dependency.name.clone(),
                Version::new(0, 1, 0), // Git packages get default version
                source_url,
            ))
        } else {
            Err(PackageError::DependencyResolution(
                "Git source requires git dependency kind".to_string(),
            ))
        }
    }

    fn package_exists(&self, _name: &str, _version: &Version) -> PackageResult<bool> {
        // For git packages, existence depends on repository accessibility
        Ok(true) // Placeholder
    }

    fn get_versions(&self, _name: &str) -> PackageResult<Vec<Version>> {
        // Git packages typically have one "version" per commit/tag
        Ok(vec![Version::new(0, 1, 0)]) // Placeholder
    }
}

/// Path-based package source
pub struct PathSource;

impl PathSource {
    pub fn new() -> Self {
        Self
    }
}

impl PackageSource for PathSource {
    fn resolve_package(&self, dependency: &Dependency) -> PackageResult<ResolvedPackage> {
        if let DependencyKind::Path { path } = &dependency.kind {
            let source_url = format!("file://{}", path.display());

            // Try to read version from manifest
            let manifest_path = path.join("script.toml");
            let version = if manifest_path.exists() {
                // In a real implementation, parse the manifest
                Version::new(0, 1, 0) // Placeholder
            } else {
                Version::new(0, 1, 0)
            };

            Ok(ResolvedPackage::new(
                dependency.name.clone(),
                version,
                source_url,
            ))
        } else {
            Err(PackageError::DependencyResolution(
                "Path source requires path dependency kind".to_string(),
            ))
        }
    }

    fn package_exists(&self, _name: &str, _version: &Version) -> PackageResult<bool> {
        // For path packages, existence depends on local path
        Ok(true) // Placeholder
    }

    fn get_versions(&self, _name: &str) -> PackageResult<Vec<Version>> {
        // Path packages typically have one version
        Ok(vec![Version::new(0, 1, 0)]) // Placeholder
    }
}

/// Package downloading progress callback
pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send + Sync>;

/// Download manager for handling package downloads
pub struct DownloadManager {
    config: DownloadConfig,
}

impl DownloadManager {
    pub fn new(config: DownloadConfig) -> Self {
        Self { config }
    }

    /// Download a package with progress tracking
    pub fn download_with_progress(
        &self,
        resolved: &ResolvedPackage,
        target_path: &Path,
        progress: Option<ProgressCallback>,
    ) -> PackageResult<()> {
        // In a real implementation, this would handle HTTP downloads with progress
        if let Some(callback) = progress {
            // Simulate progress updates
            callback(0, 100);
            callback(50, 100);
            callback(100, 100);
        }

        // Create target directory
        std::fs::create_dir_all(target_path)?;

        // For now, just create a placeholder file
        std::fs::write(
            target_path.join("package.info"),
            format!(
                "Package: {}\nVersion: {}\nSource: {}\n",
                resolved.name, resolved.version, resolved.source_url
            ),
        )?;

        Ok(())
    }
}

/// Configuration for download operations
#[derive(Debug, Clone)]
pub struct DownloadConfig {
    pub timeout_seconds: u64,
    pub max_retries: u32,
    pub retry_delay_seconds: u64,
    pub concurrent_downloads: usize,
    pub verify_tls: bool,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            max_retries: 3,
            retry_delay_seconds: 1,
            concurrent_downloads: 4,
            verify_tls: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::VersionConstraint;
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_resolver_config_default() {
        let config = ResolverConfig::default();
        assert_eq!(config.registry_url, "https://packages.script.org");
        assert_eq!(config.timeout_seconds, 30);
        assert!(config.verify_checksums);
    }

    #[test]
    fn test_resolved_package_cache_key() {
        let version = Version::new(1, 2, 3);
        let resolved = ResolvedPackage::new("test-package", version, "https://example.com");
        assert_eq!(resolved.cache_key(), "test-package-1.2.3");
    }

    #[test]
    fn test_registry_source() {
        let source = RegistrySource::new();
        let dependency = Dependency::registry(
            "test-pkg",
            VersionConstraint::Compatible(Version::new(1, 0, 0)),
        );

        let resolved = source.resolve_package(&dependency).unwrap();
        assert_eq!(resolved.name, "test-pkg");
        assert!(resolved.source_url.contains("test-pkg"));
    }

    #[test]
    fn test_git_source() {
        let source = GitSource::new();
        let dependency = Dependency::git("git-pkg", "https://github.com/user/repo.git");

        let resolved = source.resolve_package(&dependency).unwrap();
        assert_eq!(resolved.name, "git-pkg");
        assert!(resolved.source_url.contains("github.com"));
    }

    #[test]
    fn test_path_source() {
        let source = PathSource::new();
        let temp_dir = TempDir::new().unwrap();
        let dependency = Dependency::path("path-pkg", temp_dir.path());

        let resolved = source.resolve_package(&dependency).unwrap();
        assert_eq!(resolved.name, "path-pkg");
        assert!(resolved.source_url.starts_with("file://"));
    }

    #[test]
    fn test_package_resolver() {
        let config = ResolverConfig::default();
        let resolver = PackageResolver::new(config);

        let dependency = Dependency::registry(
            "test-pkg",
            VersionConstraint::Compatible(Version::new(1, 0, 0)),
        );

        let resolved = resolver.resolve_package(&dependency).unwrap();
        assert_eq!(resolved.name, "test-pkg");
    }

    #[test]
    fn test_download_manager() {
        let config = DownloadConfig::default();
        let manager = DownloadManager::new(config);

        let version = Version::new(1, 0, 0);
        let resolved = ResolvedPackage::new("test-pkg", version, "https://example.com");
        let temp_dir = TempDir::new().unwrap();

        let progress_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let progress_called_clone = progress_called.clone();

        let progress: ProgressCallback = Box::new(move |current, total| {
            progress_called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            println!("Progress: {current}/{total}");
        });

        manager
            .download_with_progress(&resolved, temp_dir.path(), Some(progress))
            .unwrap();

        assert!(temp_dir.path().join("package.info").exists());
        assert!(progress_called.load(std::sync::atomic::Ordering::SeqCst));
    }
}
