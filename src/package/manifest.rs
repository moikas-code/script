/// Package manifest (script.toml) parsing and validation
///
/// This module handles the parsing of Script package manifests, which define
/// package metadata, dependencies, build configuration, and project structure.
use super::{DependencySpec, PackageError, PackageMetadata, PackageResult, Version};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Complete package manifest structure representing script.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    pub package: PackageConfig,

    #[serde(default)]
    pub dependencies: HashMap<String, DependencySpec>,

    #[serde(default, rename = "dev-dependencies")]
    pub dev_dependencies: HashMap<String, DependencySpec>,

    #[serde(default, rename = "build-dependencies")]
    pub build_dependencies: HashMap<String, DependencySpec>,

    #[serde(default)]
    pub features: HashMap<String, Vec<String>>,

    pub lib: Option<LibraryConfig>,

    #[serde(default)]
    pub bin: Vec<BinaryConfig>,

    pub build: Option<BuildConfig>,

    pub workspace: Option<WorkspaceConfig>,

    #[serde(default)]
    pub target: HashMap<String, TargetConfig>,

    #[serde(default)]
    pub profile: HashMap<String, ProfileConfig>,
}

impl PackageManifest {
    /// Create a new manifest with the given package name
    pub fn new(name: &str) -> Self {
        Self {
            package: PackageConfig {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                authors: Vec::new(),
                description: None,
                edition: default_edition(),
                license: None,
                license_file: None,
                repository: None,
                homepage: None,
                documentation: None,
                keywords: Vec::new(),
                categories: Vec::new(),
                readme: None,
                include: Vec::new(),
                exclude: Vec::new(),
                publish: true,
                metadata: HashMap::new(),
            },
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            build_dependencies: HashMap::new(),
            features: HashMap::new(),
            lib: None,
            bin: Vec::new(),
            build: None,
            workspace: None,
            target: HashMap::new(),
            profile: HashMap::new(),
        }
    }

    /// Parse manifest from TOML file
    pub fn from_file(path: impl AsRef<Path>) -> PackageResult<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| PackageError::ManifestParse(format!("Failed to read manifest: {e}")))?;

        Self::from_str(&content)
    }

    /// Parse manifest from TOML string
    pub fn from_str(content: &str) -> PackageResult<Self> {
        let manifest: PackageManifest = toml::from_str(content)
            .map_err(|e| PackageError::ManifestParse(format!("TOML parse error: {e}")))?;

        manifest.validate()?;
        Ok(manifest)
    }

    /// Save manifest to TOML file
    pub fn to_file(&self, path: impl AsRef<Path>) -> PackageResult<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| PackageError::ManifestParse(format!("TOML serialize error: {e}")))?;

        std::fs::write(path, content)
            .map_err(|e| PackageError::ManifestParse(format!("Failed to write manifest: {e}")))?;

        Ok(())
    }

    /// Validate the manifest for consistency and required fields
    pub fn validate(&self) -> PackageResult<()> {
        // Validate package name
        if self.package.name.is_empty() {
            return Err(PackageError::ManifestParse(
                "Package name cannot be empty".to_string(),
            ));
        }

        if !is_valid_package_name(&self.package.name) {
            return Err(PackageError::ManifestParse(format!(
                "Invalid package name: {}",
                self.package.name
            )));
        }

        // Validate version
        self.package
            .version
            .parse::<semver::Version>()
            .map_err(|e| PackageError::ManifestParse(format!("Invalid version: {e}")))?;

        // Validate library path if present
        if let Some(ref lib) = self.lib {
            if lib.path.as_os_str().is_empty() {
                return Err(PackageError::ManifestParse(
                    "Library path cannot be empty".to_string(),
                ));
            }
        }

        // Validate binary configurations
        for (i, bin) in self.bin.iter().enumerate() {
            if bin.name.is_empty() {
                return Err(PackageError::ManifestParse(format!(
                    "Binary {} name cannot be empty",
                    i
                )));
            }
            if bin.path.as_os_str().is_empty() {
                return Err(PackageError::ManifestParse(format!(
                    "Binary {} path cannot be empty",
                    bin.name
                )));
            }
        }

        // Validate feature flags
        for (feature_name, deps) in &self.features {
            if feature_name.is_empty() {
                return Err(PackageError::ManifestParse(
                    "Feature name cannot be empty".to_string(),
                ));
            }

            for dep in deps {
                if dep.contains('/')
                    && !self
                        .dependencies
                        .contains_key(dep.split('/').next().unwrap())
                {
                    return Err(PackageError::ManifestParse(format!(
                        "Feature {} references unknown dependency: {}",
                        feature_name, dep
                    )));
                }
            }
        }

        Ok(())
    }

    /// Get package metadata
    pub fn metadata(&self) -> PackageMetadata {
        let version = Version::parse(&self.package.version).unwrap_or_default();

        let mut metadata = PackageMetadata::new(&self.package.name, version);
        metadata.authors = self.package.authors.clone();
        metadata.description = self.package.description.clone();
        metadata.license = self.package.license.clone();
        metadata.repository = self.package.repository.clone();
        metadata.homepage = self.package.homepage.clone();
        metadata.documentation = self.package.documentation.clone();
        metadata.keywords = self.package.keywords.clone();
        metadata.categories = self.package.categories.clone();

        metadata
    }

    /// Get all dependencies (including dev and build dependencies)
    pub fn all_dependencies(&self) -> HashMap<String, &DependencySpec> {
        let mut all_deps = HashMap::new();

        for (name, spec) in &self.dependencies {
            all_deps.insert(name.clone(), spec);
        }

        for (name, spec) in &self.dev_dependencies {
            all_deps.insert(name.clone(), spec);
        }

        for (name, spec) in &self.build_dependencies {
            all_deps.insert(name.clone(), spec);
        }

        all_deps
    }

    /// Check if package has a specific feature
    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.contains_key(feature)
    }

    /// Get dependencies for a specific feature
    pub fn feature_dependencies(&self, feature: &str) -> Vec<&str> {
        self.features
            .get(feature)
            .map(|deps| deps.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }
}

/// Package configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfig {
    pub name: String,
    pub version: String,

    #[serde(default)]
    pub authors: Vec<String>,

    pub description: Option<String>,

    #[serde(default = "default_edition")]
    pub edition: String,

    pub license: Option<String>,

    #[serde(rename = "license-file")]
    pub license_file: Option<PathBuf>,

    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,

    #[serde(default)]
    pub keywords: Vec<String>,

    #[serde(default)]
    pub categories: Vec<String>,

    #[serde(default)]
    pub readme: Option<PathBuf>,

    #[serde(default)]
    pub include: Vec<String>,

    #[serde(default)]
    pub exclude: Vec<String>,

    #[serde(default)]
    pub publish: bool,

    #[serde(default)]
    pub metadata: HashMap<String, toml::Value>,
}

fn default_edition() -> String {
    "2024".to_string()
}

/// Library configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryConfig {
    pub name: Option<String>,

    #[serde(default = "default_lib_path")]
    pub path: PathBuf,

    #[serde(default)]
    pub test: bool,

    #[serde(default)]
    pub doctest: bool,

    #[serde(default)]
    pub bench: bool,

    #[serde(default)]
    pub doc: bool,

    #[serde(default)]
    pub plugin: bool,

    #[serde(default)]
    pub proc_macro: bool,

    #[serde(default)]
    pub harness: bool,

    #[serde(default)]
    pub edition: Option<String>,
}

fn default_lib_path() -> PathBuf {
    PathBuf::from("src/lib.script")
}

impl Default for LibraryConfig {
    fn default() -> Self {
        Self {
            name: None,
            path: default_lib_path(),
            test: true,
            doctest: true,
            bench: true,
            doc: true,
            plugin: false,
            proc_macro: false,
            harness: true,
            edition: None,
        }
    }
}

/// Binary configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryConfig {
    pub name: String,
    pub path: PathBuf,

    #[serde(default)]
    pub test: bool,

    #[serde(default)]
    pub doctest: bool,

    #[serde(default)]
    pub bench: bool,

    #[serde(default)]
    pub doc: bool,

    #[serde(default)]
    pub plugin: bool,

    #[serde(default)]
    pub proc_macro: bool,

    #[serde(default)]
    pub harness: bool,

    #[serde(default)]
    pub edition: Option<String>,

    #[serde(default)]
    pub required_features: Vec<String>,
}

impl Default for BinaryConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            path: PathBuf::from("src/main.script"),
            test: true,
            doctest: true,
            bench: true,
            doc: true,
            plugin: false,
            proc_macro: false,
            harness: true,
            edition: None,
            required_features: Vec::new(),
        }
    }
}

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    #[serde(default)]
    pub target: Option<String>,

    #[serde(default)]
    pub optimization: OptimizationLevel,

    #[serde(default)]
    pub debug: bool,

    #[serde(default)]
    pub overflow_checks: bool,

    #[serde(default)]
    pub lto: bool,

    #[serde(default)]
    pub codegen_units: Option<u32>,

    #[serde(default)]
    pub panic: PanicStrategy,

    #[serde(default)]
    pub incremental: bool,

    #[serde(default)]
    pub split_debuginfo: Option<String>,

    #[serde(default)]
    pub strip: Option<String>,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            target: None,
            optimization: OptimizationLevel::default(),
            debug: true,
            overflow_checks: true,
            lto: false,
            codegen_units: None,
            panic: PanicStrategy::default(),
            incremental: true,
            split_debuginfo: None,
            strip: None,
        }
    }
}

/// Optimization level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OptimizationLevel {
    None,
    Speed,
    Size,
    Aggressive,
}

impl Default for OptimizationLevel {
    fn default() -> Self {
        Self::None
    }
}

/// Panic strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PanicStrategy {
    Unwind,
    Abort,
}

impl Default for PanicStrategy {
    fn default() -> Self {
        Self::Unwind
    }
}

/// Workspace configuration for multi-package projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub members: Vec<String>,

    #[serde(default)]
    pub exclude: Vec<String>,

    #[serde(default)]
    pub resolver: String,

    #[serde(default)]
    pub default_members: Vec<String>,

    #[serde(default)]
    pub dependencies: HashMap<String, DependencySpec>,

    #[serde(default)]
    pub metadata: HashMap<String, toml::Value>,
}

/// Target-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetConfig {
    #[serde(default)]
    pub dependencies: HashMap<String, DependencySpec>,

    #[serde(default, rename = "dev-dependencies")]
    pub dev_dependencies: HashMap<String, DependencySpec>,

    #[serde(default, rename = "build-dependencies")]
    pub build_dependencies: HashMap<String, DependencySpec>,
}

/// Profile configuration for build optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    #[serde(default)]
    pub opt_level: OptimizationLevel,

    #[serde(default)]
    pub debug: bool,

    #[serde(default)]
    pub debug_assertions: bool,

    #[serde(default)]
    pub overflow_checks: bool,

    #[serde(default)]
    pub lto: bool,

    #[serde(default)]
    pub panic: PanicStrategy,

    #[serde(default)]
    pub incremental: bool,

    #[serde(default)]
    pub codegen_units: Option<u32>,

    #[serde(default)]
    pub rpath: bool,
}

/// Validate package name according to Script language conventions
fn is_valid_package_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 64 {
        return false;
    }

    // Must start with letter or underscore
    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return false;
    }

    // Can only contain alphanumeric, underscores, and hyphens
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Create a default manifest for a new package
pub fn create_default_manifest(name: &str, is_lib: bool) -> PackageManifest {
    let mut manifest = PackageManifest {
        package: PackageConfig {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            authors: Vec::new(),
            description: None,
            edition: default_edition(),
            license: None,
            license_file: None,
            repository: None,
            homepage: None,
            documentation: None,
            keywords: Vec::new(),
            categories: Vec::new(),
            readme: None,
            include: Vec::new(),
            exclude: Vec::new(),
            publish: true,
            metadata: HashMap::new(),
        },
        dependencies: HashMap::new(),
        dev_dependencies: HashMap::new(),
        build_dependencies: HashMap::new(),
        features: HashMap::new(),
        lib: None,
        bin: Vec::new(),
        build: None,
        workspace: None,
        target: HashMap::new(),
        profile: HashMap::new(),
    };

    if is_lib {
        manifest.lib = Some(LibraryConfig {
            name: Some(name.to_string()),
            path: default_lib_path(),
            test: true,
            doctest: true,
            bench: true,
            doc: true,
            plugin: false,
            proc_macro: false,
            harness: true,
            edition: None,
        });
    } else {
        manifest.bin.push(BinaryConfig {
            name: name.to_string(),
            path: PathBuf::from("src/main.script"),
            test: true,
            doctest: true,
            bench: true,
            doc: true,
            plugin: false,
            proc_macro: false,
            harness: true,
            edition: None,
            required_features: Vec::new(),
        });
    }

    manifest
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_valid_package_names() {
        assert!(is_valid_package_name("my_package"));
        assert!(is_valid_package_name("my-package"));
        assert!(is_valid_package_name("package123"));
        assert!(is_valid_package_name("_private"));
        assert!(is_valid_package_name("a"));
    }

    #[test]
    fn test_invalid_package_names() {
        assert!(!is_valid_package_name(""));
        assert!(!is_valid_package_name("123package"));
        assert!(!is_valid_package_name("-package"));
        assert!(!is_valid_package_name("package.name"));
        assert!(!is_valid_package_name("package name"));
        assert!(!is_valid_package_name(&"x".repeat(65)));
    }

    #[test]
    fn test_default_manifest_library() {
        let manifest = create_default_manifest("test-lib", true);
        assert_eq!(manifest.package.name, "test-lib");
        assert_eq!(manifest.package.version, "0.1.0");
        assert!(manifest.lib.is_some());
        assert!(manifest.bin.is_empty());
    }

    #[test]
    fn test_default_manifest_binary() {
        let manifest = create_default_manifest("test-bin", false);
        assert_eq!(manifest.package.name, "test-bin");
        assert_eq!(manifest.package.version, "0.1.0");
        assert!(manifest.lib.is_none());
        assert_eq!(manifest.bin.len(), 1);
        assert_eq!(manifest.bin[0].name, "test-bin");
    }

    #[test]
    fn test_manifest_serialization() {
        let manifest = create_default_manifest("test-package", true);
        let toml_str = toml::to_string(&manifest).unwrap();
        let parsed_manifest: PackageManifest = toml::from_str(&toml_str).unwrap();

        assert_eq!(manifest.package.name, parsed_manifest.package.name);
        assert_eq!(manifest.package.version, parsed_manifest.package.version);
    }

    #[test]
    fn test_manifest_file_operations() {
        let manifest = create_default_manifest("test-file", true);
        let temp_file = NamedTempFile::new().unwrap();

        // Save manifest
        manifest.to_file(temp_file.path()).unwrap();

        // Load manifest
        let loaded_manifest = PackageManifest::from_file(temp_file.path()).unwrap();
        assert_eq!(manifest.package.name, loaded_manifest.package.name);
    }

    #[test]
    fn test_manifest_validation() {
        let mut manifest = create_default_manifest("test-validation", true);

        // Valid manifest should pass
        assert!(manifest.validate().is_ok());

        // Invalid name should fail
        manifest.package.name = "".to_string();
        assert!(manifest.validate().is_err());

        // Invalid version should fail
        manifest.package.name = "test".to_string();
        manifest.package.version = "invalid-version".to_string();
        assert!(manifest.validate().is_err());
    }
}
