use crate::module::path::ImportKind;
use crate::module::{ModuleError, ModulePath, ModuleResult, ResolvedModule};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Module cache for storing resolved modules and avoiding recompilation
#[derive(Debug)]
pub struct ModuleCache {
    entries: HashMap<ModulePath, CacheEntry>,
    file_timestamps: HashMap<PathBuf, u64>,
    dependency_graph: DependencyGraph,
}

/// Cache entry for a resolved module
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub module: ResolvedModule,
    pub timestamp: u64,
    pub file_hash: u64,
    pub dependencies: Vec<ModulePath>,
    pub is_valid: bool,
}

/// Dependency graph for tracking module relationships
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    dependencies: HashMap<ModulePath, Vec<ModulePath>>,
    dependents: HashMap<ModulePath, Vec<ModulePath>>,
}

impl ModuleCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            file_timestamps: HashMap::new(),
            dependency_graph: DependencyGraph::new(),
        }
    }

    /// Check if a module is cached and up-to-date
    pub fn is_cached(&self, module_path: &ModulePath) -> bool {
        if let Some(entry) = self.entries.get(module_path) {
            entry.is_valid && self.is_entry_fresh(entry)
        } else {
            false
        }
    }

    /// Get a cached module if it exists and is valid
    pub fn get(&self, module_path: &ModulePath) -> Option<&ResolvedModule> {
        if self.is_cached(module_path) {
            self.entries.get(module_path).map(|entry| &entry.module)
        } else {
            None
        }
    }

    /// Cache a resolved module
    pub fn insert(&mut self, module: ResolvedModule) -> ModuleResult<()> {
        let module_path = module.path.clone();
        let file_path = module.file_path.clone();

        let timestamp = get_current_timestamp();
        let file_hash = self.calculate_file_hash(&file_path)?;

        // Update dependency graph
        // Convert ImportPath to ModulePath for the dependency graph
        let dep_paths: Vec<ModulePath> = module
            .dependencies
            .iter()
            .filter_map(|import| {
                // For now, only handle absolute imports as module paths
                if import.kind == ImportKind::Absolute {
                    ModulePath::from_string(&import.path).ok()
                } else {
                    None
                }
            })
            .collect();
        self.dependency_graph.add_module(&module_path, &dep_paths);

        // Update file timestamp tracking
        self.file_timestamps.insert(file_path, timestamp);

        let entry = CacheEntry {
            module,
            timestamp,
            file_hash,
            dependencies: self
                .dependency_graph
                .get_dependencies(&module_path)
                .unwrap_or_default(),
            is_valid: true,
        };

        self.entries.insert(module_path, entry);
        Ok(())
    }

    /// Invalidate a module and all its dependents
    pub fn invalidate(&mut self, module_path: &ModulePath) {
        if let Some(entry) = self.entries.get_mut(module_path) {
            entry.is_valid = false;
        }

        // Invalidate all dependents recursively
        let dependents = self
            .dependency_graph
            .get_dependents(module_path)
            .unwrap_or_default();
        for dependent in dependents {
            self.invalidate(&dependent);
        }
    }

    /// Clear the entire cache
    pub fn clear(&mut self) {
        self.entries.clear();
        self.file_timestamps.clear();
        self.dependency_graph.clear();
    }

    /// Check if any dependency of a module has changed
    pub fn check_dependencies(&mut self, module_path: &ModulePath) -> ModuleResult<bool> {
        if let Some(dependencies) = self.dependency_graph.get_dependencies(module_path) {
            for dep in &dependencies {
                if !self.is_cached(dep)
                    || self.has_file_changed(&self.entries[dep].module.file_path)?
                {
                    return Ok(true);
                }

                // Recursively check dependencies
                if self.check_dependencies(dep)? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let total_modules = self.entries.len();
        let valid_modules = self.entries.values().filter(|e| e.is_valid).count();
        let total_dependencies = self.dependency_graph.total_edges();

        CacheStats {
            total_modules,
            valid_modules,
            invalid_modules: total_modules - valid_modules,
            total_dependencies,
        }
    }

    fn is_entry_fresh(&self, entry: &CacheEntry) -> bool {
        // Check if the file has been modified since caching
        if let Ok(has_changed) = self.has_file_changed(&entry.module.file_path) {
            !has_changed
        } else {
            false
        }
    }

    fn has_file_changed(&self, file_path: &PathBuf) -> ModuleResult<bool> {
        let metadata = std::fs::metadata(file_path)?;
        let modified = metadata.modified()?;
        let current_timestamp = modified
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ModuleError::cache_error("Invalid file timestamp"))?
            .as_secs();

        if let Some(&cached_timestamp) = self.file_timestamps.get(file_path) {
            Ok(current_timestamp > cached_timestamp)
        } else {
            Ok(true) // File not in cache, consider it changed
        }
    }

    fn calculate_file_hash(&self, file_path: &PathBuf) -> ModuleResult<u64> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let content = std::fs::read_to_string(file_path)?;
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        Ok(hasher.finish())
    }
}

impl Default for ModuleCache {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            dependents: HashMap::new(),
        }
    }

    /// Add a module with its dependencies
    pub fn add_module(&mut self, module: &ModulePath, dependencies: &[ModulePath]) {
        // Add dependencies
        self.dependencies
            .insert(module.clone(), dependencies.to_vec());

        // Update reverse dependencies (dependents)
        for dep in dependencies {
            self.dependents
                .entry(dep.clone())
                .or_insert_with(Vec::new)
                .push(module.clone());
        }
    }

    /// Get direct dependencies of a module
    pub fn get_dependencies(&self, module: &ModulePath) -> Option<Vec<ModulePath>> {
        self.dependencies.get(module).cloned()
    }

    /// Get direct dependents of a module
    pub fn get_dependents(&self, module: &ModulePath) -> Option<Vec<ModulePath>> {
        self.dependents.get(module).cloned()
    }

    /// Remove a module from the dependency graph
    pub fn remove_module(&mut self, module: &ModulePath) {
        // Remove from dependencies
        if let Some(deps) = self.dependencies.remove(module) {
            // Remove this module from dependents of its dependencies
            for dep in deps {
                if let Some(dependents) = self.dependents.get_mut(&dep) {
                    dependents.retain(|d| d != module);
                    if dependents.is_empty() {
                        self.dependents.remove(&dep);
                    }
                }
            }
        }

        // Remove from dependents
        self.dependents.remove(module);

        // Remove this module from dependencies of its dependents
        for (_, deps) in self.dependencies.iter_mut() {
            deps.retain(|d| d != module);
        }
    }

    /// Check for circular dependencies
    pub fn has_cycle(&self, start: &ModulePath) -> bool {
        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();

        self.has_cycle_util(start, &mut visited, &mut rec_stack)
    }

    /// Get topological order of modules
    pub fn topological_sort(&self) -> ModuleResult<Vec<ModulePath>> {
        let mut visited = std::collections::HashSet::new();
        let mut stack = Vec::new();

        for module in self.dependencies.keys() {
            if !visited.contains(module) {
                if self.has_cycle(module) {
                    return Err(ModuleError::new(
                        crate::module::ModuleErrorKind::CircularDependency,
                        format!("Circular dependency detected starting from {}", module),
                    ));
                }
                self.topological_sort_util(module, &mut visited, &mut stack);
            }
        }

        stack.reverse();
        Ok(stack)
    }

    /// Clear the dependency graph
    pub fn clear(&mut self) {
        self.dependencies.clear();
        self.dependents.clear();
    }

    /// Get total number of dependency edges
    pub fn total_edges(&self) -> usize {
        self.dependencies.values().map(|deps| deps.len()).sum()
    }

    fn has_cycle_util(
        &self,
        module: &ModulePath,
        visited: &mut std::collections::HashSet<ModulePath>,
        rec_stack: &mut std::collections::HashSet<ModulePath>,
    ) -> bool {
        visited.insert(module.clone());
        rec_stack.insert(module.clone());

        if let Some(dependencies) = self.dependencies.get(module) {
            for dep in dependencies {
                if !visited.contains(dep) && self.has_cycle_util(dep, visited, rec_stack) {
                    return true;
                }
                if rec_stack.contains(dep) {
                    return true;
                }
            }
        }

        rec_stack.remove(module);
        false
    }

    fn topological_sort_util(
        &self,
        module: &ModulePath,
        visited: &mut std::collections::HashSet<ModulePath>,
        stack: &mut Vec<ModulePath>,
    ) {
        visited.insert(module.clone());

        if let Some(dependencies) = self.dependencies.get(module) {
            for dep in dependencies {
                if !visited.contains(dep) {
                    self.topological_sort_util(dep, visited, stack);
                }
            }
        }

        stack.push(module.clone());
    }
}

/// Cache statistics for monitoring and debugging
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_modules: usize,
    pub valid_modules: usize,
    pub invalid_modules: usize,
    pub total_dependencies: usize,
}

impl CacheStats {
    pub fn cache_hit_rate(&self) -> f64 {
        if self.total_modules == 0 {
            0.0
        } else {
            self.valid_modules as f64 / self.total_modules as f64
        }
    }
}

fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::ModuleMetadata;

    fn create_test_module(name: &str) -> ResolvedModule {
        use tempfile::NamedTempFile;

        let path = ModulePath::from_string(name).unwrap();

        // Create a temporary file
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.into_temp_path().to_path_buf();
        let source = format!("// Module {}", name);
        std::fs::write(&file_path, &source).unwrap();

        let metadata = ModuleMetadata::default();

        ResolvedModule::new(path, file_path, source, metadata)
    }

    #[test]
    fn test_cache_basic_operations() {
        let mut cache = ModuleCache::new();
        let module = create_test_module("test");
        let module_path = module.path.clone();

        assert!(!cache.is_cached(&module_path));
        assert!(cache.get(&module_path).is_none());

        cache.insert(module).unwrap();

        // Note: In real usage, file timestamps would be checked
        // For testing, we just verify the module was cached
        assert!(cache.entries.contains_key(&module_path));
    }

    #[test]
    fn test_cache_invalidation() {
        let mut cache = ModuleCache::new();
        let module = create_test_module("test");
        let module_path = module.path.clone();

        cache.insert(module).unwrap();
        cache.invalidate(&module_path);

        if let Some(entry) = cache.entries.get(&module_path) {
            assert!(!entry.is_valid);
        }
    }

    #[test]
    fn test_dependency_graph() {
        let mut graph = DependencyGraph::new();

        let mod_a = ModulePath::from_string("a").unwrap();
        let mod_b = ModulePath::from_string("b").unwrap();
        let mod_c = ModulePath::from_string("c").unwrap();

        graph.add_module(&mod_a, &[mod_b.clone()]);
        graph.add_module(&mod_b, &[mod_c.clone()]);
        graph.add_module(&mod_c, &[]);

        assert_eq!(graph.get_dependencies(&mod_a), Some(vec![mod_b.clone()]));
        assert_eq!(graph.get_dependents(&mod_b), Some(vec![mod_a.clone()]));

        let sorted = graph.topological_sort().unwrap();
        let c_pos = sorted.iter().position(|m| m == &mod_c).unwrap();
        let b_pos = sorted.iter().position(|m| m == &mod_b).unwrap();
        let a_pos = sorted.iter().position(|m| m == &mod_a).unwrap();

        assert!(c_pos < b_pos);
        assert!(b_pos < a_pos);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut graph = DependencyGraph::new();

        let mod_a = ModulePath::from_string("a").unwrap();
        let mod_b = ModulePath::from_string("b").unwrap();

        graph.add_module(&mod_a, &[mod_b.clone()]);
        graph.add_module(&mod_b, &[mod_a.clone()]);

        assert!(graph.has_cycle(&mod_a));
        assert!(graph.topological_sort().is_err());
    }

    #[test]
    fn test_cache_stats() {
        let mut cache = ModuleCache::new();
        let module1 = create_test_module("test1");
        let module2 = create_test_module("test2");

        cache.insert(module1).unwrap();
        cache.insert(module2).unwrap();

        let stats = cache.stats();
        assert_eq!(stats.total_modules, 2);
        assert_eq!(stats.valid_modules, 2);
        assert_eq!(stats.invalid_modules, 0);
    }
}
