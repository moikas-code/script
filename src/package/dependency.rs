/// Dependency specification and resolution for Script packages
///
/// This module handles:
/// - Dependency specification parsing from manifests
/// - Dependency graph construction and validation
/// - Dependency resolution algorithms
/// - Circular dependency detection
use super::{PackageError, PackageResult, Version, VersionConstraint};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

/// Dependency specification as it appears in manifests
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DependencySpec {
    /// Simple version string ("1.2.3")
    Simple(String),

    /// Detailed dependency specification
    Detailed {
        /// Version constraint
        version: Option<String>,

        /// Git repository URL
        git: Option<String>,

        /// Git branch
        branch: Option<String>,

        /// Git tag
        tag: Option<String>,

        /// Git revision/commit hash
        rev: Option<String>,

        /// Local path
        path: Option<PathBuf>,

        /// Registry name (for alternative registries)
        registry: Option<String>,

        /// Package name (if different from dependency key)
        package: Option<String>,

        /// Features to enable
        #[serde(default)]
        features: Vec<String>,

        /// Whether this dependency is optional
        #[serde(default)]
        optional: bool,

        /// Whether to enable default features
        #[serde(default = "default_true")]
        default_features: bool,

        /// Target platform specification
        target: Option<String>,
    },
}

fn default_true() -> bool {
    true
}

impl DependencySpec {
    /// Convert to a resolved dependency
    pub fn resolve(&self, name: &str) -> PackageResult<Dependency> {
        match self {
            Self::Simple(version) => {
                let version_constraint = VersionConstraint::parse(version)?;
                Ok(Dependency {
                    name: name.to_string(),
                    kind: DependencyKind::Registry,
                    version_constraint,
                    features: Vec::new(),
                    optional: false,
                    default_features: true,
                    target: None,
                })
            }
            Self::Detailed {
                version,
                git,
                branch,
                tag,
                rev,
                path,
                registry: _,
                package,
                features,
                optional,
                default_features,
                target,
            } => {
                let actual_name = package.as_ref().unwrap_or(&name.to_string()).clone();

                let kind = if let Some(git_url) = git {
                    DependencyKind::Git {
                        url: git_url.clone(),
                        branch: branch.clone(),
                        tag: tag.clone(),
                        rev: rev.clone(),
                    }
                } else if let Some(local_path) = path {
                    DependencyKind::Path {
                        path: local_path.clone(),
                    }
                } else {
                    DependencyKind::Registry
                };

                let version_constraint = if let Some(version_str) = version {
                    VersionConstraint::parse(version_str)?
                } else {
                    // For non-registry dependencies, use a wildcard constraint
                    VersionConstraint::Wildcard(0, None)
                };

                Ok(Dependency {
                    name: actual_name,
                    kind,
                    version_constraint,
                    features: features.clone(),
                    optional: *optional,
                    default_features: *default_features,
                    target: target.clone(),
                })
            }
        }
    }

    /// Check if this is a registry dependency
    pub fn is_registry(&self) -> bool {
        match self {
            Self::Simple(_) => true,
            Self::Detailed { git, path, .. } => git.is_none() && path.is_none(),
        }
    }

    /// Check if this is a git dependency
    pub fn is_git(&self) -> bool {
        match self {
            Self::Simple(_) => false,
            Self::Detailed { git, .. } => git.is_some(),
        }
    }

    /// Check if this is a path dependency
    pub fn is_path(&self) -> bool {
        match self {
            Self::Simple(_) => false,
            Self::Detailed { path, .. } => path.is_some(),
        }
    }
}

/// Resolved dependency with all information needed for installation
#[derive(Debug, Clone, PartialEq)]
pub struct Dependency {
    pub name: String,
    pub kind: DependencyKind,
    pub version_constraint: VersionConstraint,
    pub features: Vec<String>,
    pub optional: bool,
    pub default_features: bool,
    pub target: Option<String>,
}

impl Dependency {
    /// Create a simple registry dependency
    pub fn registry(name: impl Into<String>, version: VersionConstraint) -> Self {
        Self {
            name: name.into(),
            kind: DependencyKind::Registry,
            version_constraint: version,
            features: Vec::new(),
            optional: false,
            default_features: true,
            target: None,
        }
    }

    /// Create a git dependency
    pub fn git(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: DependencyKind::Git {
                url: url.into(),
                branch: None,
                tag: None,
                rev: None,
            },
            version_constraint: VersionConstraint::Wildcard(0, None),
            features: Vec::new(),
            optional: false,
            default_features: true,
            target: None,
        }
    }

    /// Create a path dependency
    pub fn path(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            kind: DependencyKind::Path { path: path.into() },
            version_constraint: VersionConstraint::Wildcard(0, None),
            features: Vec::new(),
            optional: false,
            default_features: true,
            target: None,
        }
    }

    /// Check if a version satisfies this dependency
    pub fn is_satisfied_by(&self, version: &Version) -> bool {
        self.version_constraint.matches(version)
    }

    /// Get unique identifier for this dependency
    pub fn id(&self) -> String {
        match &self.kind {
            DependencyKind::Registry => self.name.clone(),
            DependencyKind::Git {
                url,
                branch,
                tag,
                rev,
            } => {
                let mut id = format!("git+{}", url);
                if let Some(branch) = branch {
                    id.push_str(&format!("#branch={}", branch));
                } else if let Some(tag) = tag {
                    id.push_str(&format!("#tag={}", tag));
                } else if let Some(rev) = rev {
                    id.push_str(&format!("#rev={}", rev));
                }
                id
            }
            DependencyKind::Path { path } => {
                format!("path+{}", path.display())
            }
        }
    }
}

/// Types of dependency sources
#[derive(Debug, Clone, PartialEq)]
pub enum DependencyKind {
    /// Package from registry
    Registry,

    /// Git repository dependency
    Git {
        url: String,
        branch: Option<String>,
        tag: Option<String>,
        rev: Option<String>,
    },

    /// Local path dependency
    Path { path: PathBuf },
}

/// Dependency graph representing all package dependencies
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// Map of package names to their dependencies
    dependencies: HashMap<String, Vec<Dependency>>,

    /// Resolved versions for each package
    resolved_versions: HashMap<String, Version>,

    /// Topologically sorted build order
    build_order: Vec<String>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            resolved_versions: HashMap::new(),
            build_order: Vec::new(),
        }
    }

    /// Add a dependency to the graph
    pub fn add_dependency(&mut self, package: String, dependency: Dependency) {
        self.dependencies
            .entry(package)
            .or_insert_with(Vec::new)
            .push(dependency);
    }

    /// Get dependencies for a package
    pub fn get_dependencies(&self, package: &str) -> Option<&Vec<Dependency>> {
        self.dependencies.get(package)
    }

    /// Get all dependencies as iterator
    pub fn dependencies(&self) -> impl Iterator<Item = (&String, &Vec<Dependency>)> {
        self.dependencies.iter()
    }

    /// Set resolved version for a package
    pub fn set_resolved_version(&mut self, package: String, version: Version) {
        self.resolved_versions.insert(package, version);
    }

    /// Get resolved version for a package
    pub fn get_resolved_version(&self, package: &str) -> Option<&Version> {
        self.resolved_versions.get(package)
    }

    /// Validate the dependency graph for circular dependencies
    pub fn validate(&self) -> PackageResult<()> {
        self.detect_cycles()?;
        Ok(())
    }

    /// Detect circular dependencies using DFS
    fn detect_cycles(&self) -> PackageResult<()> {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();
        let mut path = Vec::new();

        for package in self.dependencies.keys() {
            if !visited.contains(package) {
                self.dfs_cycle_detection(package, &mut visited, &mut stack, &mut path)?;
            }
        }

        Ok(())
    }

    fn dfs_cycle_detection(
        &self,
        package: &str,
        visited: &mut HashSet<String>,
        stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> PackageResult<()> {
        visited.insert(package.to_string());
        stack.insert(package.to_string());
        path.push(package.to_string());

        if let Some(deps) = self.dependencies.get(package) {
            for dep in deps {
                if stack.contains(&dep.name) {
                    // Found a cycle
                    let cycle_start = path.iter().position(|p| p == &dep.name).unwrap();
                    let cycle: Vec<_> = path[cycle_start..].iter().cloned().collect();
                    return Err(PackageError::CircularDependency {
                        cycle: cycle.join(" -> "),
                    });
                }

                if !visited.contains(&dep.name) {
                    self.dfs_cycle_detection(&dep.name, visited, stack, path)?;
                }
            }
        }

        stack.remove(package);
        path.pop();
        Ok(())
    }

    /// Compute topological ordering for build dependencies
    pub fn compute_build_order(&mut self) -> PackageResult<()> {
        let mut in_degree = HashMap::new();
        let mut graph = HashMap::new();

        // Initialize in-degree counts and adjacency list
        for (package, deps) in &self.dependencies {
            in_degree.entry(package.clone()).or_insert(0);
            graph.entry(package.clone()).or_insert_with(Vec::new);

            for dep in deps {
                in_degree.entry(dep.name.clone()).or_insert(0);
                graph.entry(dep.name.clone()).or_insert_with(Vec::new);
                graph.get_mut(&dep.name).unwrap().push(package.clone());
                *in_degree.get_mut(package).unwrap() += 1;
            }
        }

        // Kahn's algorithm for topological sorting
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        // Find all nodes with no incoming edges
        for (package, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(package.clone());
            }
        }

        while let Some(package) = queue.pop_front() {
            result.push(package.clone());

            if let Some(dependents) = graph.get(&package) {
                for dependent in dependents {
                    let degree = in_degree.get_mut(dependent).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(dependent.clone());
                    }
                }
            }
        }

        if result.len() != in_degree.len() {
            return Err(PackageError::CircularDependency {
                cycle: "Circular dependency detected in graph".to_string(),
            });
        }

        self.build_order = result;
        Ok(())
    }

    /// Get the build order
    pub fn build_order(&self) -> &[String] {
        &self.build_order
    }

    /// Check if the graph is empty
    pub fn is_empty(&self) -> bool {
        self.dependencies.is_empty()
    }

    /// Get total number of packages in the graph
    pub fn package_count(&self) -> usize {
        self.dependencies.len()
    }
}

/// Dependency resolver for computing dependency graphs
pub struct DependencyResolver<'a> {
    cache: &'a dyn DependencyPackageCache,
    registry: &'a dyn DependencyPackageRegistry,
}

impl<'a> DependencyResolver<'a> {
    pub fn new(
        cache: &'a dyn DependencyPackageCache,
        registry: &'a dyn DependencyPackageRegistry,
    ) -> Self {
        Self { cache, registry }
    }

    /// Resolve dependencies from a specification map
    pub fn resolve(
        &mut self,
        dependencies: &HashMap<String, DependencySpec>,
    ) -> PackageResult<DependencyGraph> {
        let mut graph = DependencyGraph::new();
        let mut resolved_packages = HashMap::new();

        // Convert specs to dependencies
        let mut deps_to_resolve = Vec::new();
        for (name, spec) in dependencies {
            let dependency = spec.resolve(name)?;
            deps_to_resolve.push(dependency);
        }

        // Resolve each dependency recursively
        while let Some(dep) = deps_to_resolve.pop() {
            if resolved_packages.contains_key(&dep.name) {
                continue;
            }

            let version = self.resolve_version(&dep)?;
            graph.set_resolved_version(dep.name.clone(), version.clone());

            // Add transitive dependencies
            let transitive_deps = self.get_package_dependencies(&dep.name, &version)?;
            resolved_packages.insert(dep.name.clone(), version);
            for (trans_name, trans_spec) in transitive_deps {
                let trans_dep = trans_spec.resolve(&trans_name)?;
                graph.add_dependency(dep.name.clone(), trans_dep.clone());

                if !resolved_packages.contains_key(&trans_name) {
                    deps_to_resolve.push(trans_dep);
                }
            }
        }

        graph.validate()?;
        graph.compute_build_order()?;

        Ok(graph)
    }

    fn resolve_version(&self, dependency: &Dependency) -> PackageResult<Version> {
        match &dependency.kind {
            DependencyKind::Registry => {
                // Get available versions from registry
                let available_versions = self.registry.get_versions(&dependency.name)?;

                // Find the highest version that satisfies the constraint
                dependency
                    .version_constraint
                    .highest_matching(&available_versions)
                    .cloned()
                    .ok_or_else(|| {
                        PackageError::DependencyResolution(format!(
                            "No version of {} satisfies {}",
                            dependency.name, dependency.version_constraint
                        ))
                    })
            }
            DependencyKind::Git { .. } => {
                // For git dependencies, we would need to clone/fetch and determine version
                // For now, return a default version
                Ok(Version::new(0, 1, 0))
            }
            DependencyKind::Path { .. } => {
                // For path dependencies, read the local manifest
                // For now, return a default version
                Ok(Version::new(0, 1, 0))
            }
        }
    }

    fn get_package_dependencies(
        &self,
        _name: &str,
        _version: &Version,
    ) -> PackageResult<HashMap<String, DependencySpec>> {
        // This would fetch the package manifest and extract its dependencies
        // For now, return empty dependencies
        Ok(HashMap::new())
    }
}

/// Result of dependency resolution
#[derive(Debug, Clone)]
pub struct ResolutionResult {
    pub graph: DependencyGraph,
    pub conflicts: Vec<VersionConflict>,
    pub warnings: Vec<String>,
}

impl ResolutionResult {
    pub fn new(graph: DependencyGraph) -> Self {
        Self {
            graph,
            conflicts: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn with_conflicts(mut self, conflicts: Vec<VersionConflict>) -> Self {
        self.conflicts = conflicts;
        self
    }

    pub fn with_warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings = warnings;
        self
    }

    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }
}

/// Version conflict information
#[derive(Debug, Clone)]
pub struct VersionConflict {
    pub package: String,
    pub required_by: Vec<(String, VersionConstraint)>,
    pub resolved_version: Option<Version>,
}

// Trait definitions for external dependencies
pub trait DependencyPackageCache {
    fn has_package(&self, name: &str, version: &Version) -> PackageResult<bool>;
    fn get_package(&self, name: &str, version: &Version) -> PackageResult<Vec<u8>>;
    fn store_package(&self, name: &str, version: &Version, data: Vec<u8>) -> PackageResult<()>;
}

pub trait DependencyPackageRegistry {
    fn get_versions(&self, name: &str) -> PackageResult<Vec<Version>>;
    fn get_package(&self, name: &str, version: &str) -> PackageResult<DependencyPackageInfo>;
}

pub struct DependencyPackageInfo {
    pub name: String,
    pub version: Version,
    pub source: Vec<u8>,
    pub dependencies: HashMap<String, DependencySpec>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_spec_simple() {
        let spec = DependencySpec::Simple("1.2.3".to_string());
        assert!(spec.is_registry());
        assert!(!spec.is_git());
        assert!(!spec.is_path());

        let dep = spec.resolve("test-package").unwrap();
        assert_eq!(dep.name, "test-package");
        assert!(matches!(dep.kind, DependencyKind::Registry));
    }

    #[test]
    fn test_dependency_spec_git() {
        let spec = DependencySpec::Detailed {
            version: None,
            git: Some("https://github.com/user/repo.git".to_string()),
            branch: Some("main".to_string()),
            tag: None,
            rev: None,
            path: None,
            registry: None,
            package: None,
            features: Vec::new(),
            optional: false,
            default_features: true,
            target: None,
        };

        assert!(!spec.is_registry());
        assert!(spec.is_git());
        assert!(!spec.is_path());

        let dep = spec.resolve("git-package").unwrap();
        assert_eq!(dep.name, "git-package");
        assert!(matches!(dep.kind, DependencyKind::Git { .. }));
    }

    #[test]
    fn test_dependency_spec_path() {
        let spec = DependencySpec::Detailed {
            version: None,
            git: None,
            branch: None,
            tag: None,
            rev: None,
            path: Some(PathBuf::from("../local-package")),
            registry: None,
            package: None,
            features: Vec::new(),
            optional: false,
            default_features: true,
            target: None,
        };

        assert!(!spec.is_registry());
        assert!(!spec.is_git());
        assert!(spec.is_path());

        let dep = spec.resolve("local-package").unwrap();
        assert_eq!(dep.name, "local-package");
        assert!(matches!(dep.kind, DependencyKind::Path { .. }));
    }

    #[test]
    fn test_dependency_graph() {
        let mut graph = DependencyGraph::new();

        let dep1 = Dependency::registry(
            "package-a",
            VersionConstraint::Compatible(Version::new(1, 0, 0)),
        );
        let dep2 = Dependency::registry(
            "package-b",
            VersionConstraint::Compatible(Version::new(2, 0, 0)),
        );

        graph.add_dependency("root".to_string(), dep1);
        graph.add_dependency("root".to_string(), dep2);

        assert_eq!(graph.package_count(), 1);
        assert!(!graph.is_empty());

        let deps = graph.get_dependencies("root").unwrap();
        assert_eq!(deps.len(), 2);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut graph = DependencyGraph::new();

        // Create a circular dependency: A -> B -> C -> A
        let dep_a_to_b =
            Dependency::registry("B", VersionConstraint::Compatible(Version::new(1, 0, 0)));
        let dep_b_to_c =
            Dependency::registry("C", VersionConstraint::Compatible(Version::new(1, 0, 0)));
        let dep_c_to_a =
            Dependency::registry("A", VersionConstraint::Compatible(Version::new(1, 0, 0)));

        graph.add_dependency("A".to_string(), dep_a_to_b);
        graph.add_dependency("B".to_string(), dep_b_to_c);
        graph.add_dependency("C".to_string(), dep_c_to_a);

        assert!(graph.validate().is_err());
    }

    #[test]
    fn test_build_order_computation() {
        let mut graph = DependencyGraph::new();

        // A depends on B, B depends on C
        let dep_a_to_b =
            Dependency::registry("B", VersionConstraint::Compatible(Version::new(1, 0, 0)));
        let dep_b_to_c =
            Dependency::registry("C", VersionConstraint::Compatible(Version::new(1, 0, 0)));

        graph.add_dependency("A".to_string(), dep_a_to_b);
        graph.add_dependency("B".to_string(), dep_b_to_c);

        graph.compute_build_order().unwrap();
        let build_order = graph.build_order();

        // C should come before B, B should come before A
        let c_pos = build_order.iter().position(|p| p == "C").unwrap();
        let b_pos = build_order.iter().position(|p| p == "B").unwrap();
        let a_pos = build_order.iter().position(|p| p == "A").unwrap();

        assert!(c_pos < b_pos);
        assert!(b_pos < a_pos);
    }
}
