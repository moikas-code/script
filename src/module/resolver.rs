use crate::module::{
    ImportPath, ModuleError, ModuleLoadContext, ModuleMetadata, ModulePath, ModuleResult,
    ResolvedModule,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Trait for module resolution strategies
pub trait ModuleResolver {
    /// Resolve a module by its import path
    fn resolve_module(
        &mut self,
        import_path: &ImportPath,
        context: &ModuleLoadContext,
    ) -> ModuleResult<ResolvedModule>;

    /// Check if a module exists without fully loading it
    fn module_exists(&self, import_path: &ImportPath, context: &ModuleLoadContext) -> bool;

    /// Get search paths used by this resolver
    fn search_paths(&self) -> &[PathBuf];

    /// Add a search path to the resolver
    fn add_search_path(&mut self, path: PathBuf);
}

/// File system based module resolver
#[derive(Debug)]
pub struct FileSystemResolver {
    config: ModuleResolverConfig,
    search_paths: Vec<PathBuf>,
    visited_paths: HashSet<PathBuf>,
}

/// Configuration for the module resolver
#[derive(Debug, Clone)]
pub struct ModuleResolverConfig {
    pub search_stdlib: bool,
    pub search_external: bool,
    pub follow_symlinks: bool,
    pub max_depth: usize,
    pub file_extensions: Vec<String>,
    pub module_file_names: Vec<String>,
    pub case_sensitive: bool,
}

impl FileSystemResolver {
    pub fn new(config: ModuleResolverConfig) -> Self {
        let mut search_paths = Vec::new();

        // Add standard library path if configured
        if config.search_stdlib {
            if let Ok(stdlib_path) = std::env::var("SCRIPT_STDLIB_PATH") {
                search_paths.push(PathBuf::from(stdlib_path));
            } else {
                // Default stdlib location
                if let Ok(current_exe) = std::env::current_exe() {
                    if let Some(exe_dir) = current_exe.parent() {
                        search_paths.push(exe_dir.join("stdlib"));
                    }
                }
            }
        }

        Self {
            config,
            search_paths,
            visited_paths: HashSet::new(),
        }
    }

    /// Resolve an import path to a module path
    pub fn resolve_import_path(
        &self,
        import_path: &ImportPath,
        context: &ModuleLoadContext,
    ) -> ModuleResult<ModulePath> {
        import_path.resolve(&context.current_module)
    }

    /// Find the file path for a module
    pub fn find_module_file(
        &self,
        module_path: &ModulePath,
        context: &ModuleLoadContext,
    ) -> ModuleResult<PathBuf> {
        // Check if it's a standard library module
        if module_path.is_std() {
            return self.find_stdlib_module(module_path);
        }

        // Check in project search paths
        for search_path in &context.search_paths {
            if let Some(file_path) = self.try_find_in_path(module_path, search_path) {
                return Ok(file_path);
            }
        }

        // Check in global search paths
        for search_path in &self.search_paths {
            if let Some(file_path) = self.try_find_in_path(module_path, search_path) {
                return Ok(file_path);
            }
        }

        Err(ModuleError::not_found(module_path.to_string()))
    }

    fn find_stdlib_module(&self, module_path: &ModulePath) -> ModuleResult<PathBuf> {
        // Remove 'std' prefix for file system lookup
        let segments = &module_path.segments()[1..];
        let stdlib_module = ModulePath::new(segments.to_vec(), true)?;

        for search_path in &self.search_paths {
            if let Some(file_path) = self.try_find_in_path(&stdlib_module, search_path) {
                return Ok(file_path);
            }
        }

        Err(ModuleError::not_found(module_path.to_string()))
    }

    fn try_find_in_path(&self, module_path: &ModulePath, base_path: &Path) -> Option<PathBuf> {
        let possible_paths = module_path.possible_file_paths(base_path);

        for path in possible_paths {
            if path.exists() && path.is_file() {
                // Check if we should follow symlinks
                if !self.config.follow_symlinks && self.is_symlink(&path) {
                    continue;
                }
                return Some(path);
            }
        }

        None
    }

    fn is_symlink(&self, path: &Path) -> bool {
        std::fs::symlink_metadata(path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false)
    }

    fn load_module_source(&self, file_path: &Path) -> ModuleResult<String> {
        std::fs::read_to_string(file_path).map_err(|e| ModuleError::file_system(file_path, e))
    }

    fn extract_dependencies(&self, _source: &str) -> Vec<ImportPath> {
        // In a real implementation, this would parse the source code
        // to extract import statements and return the imported paths
        // For now, return empty vector
        Vec::new()
    }

    fn create_module_metadata(&self, module_path: &ModulePath, file_path: &Path) -> ModuleMetadata {
        let mut metadata = ModuleMetadata::default();
        metadata.name = module_path.module_name().to_string();

        if let Ok(file_metadata) = std::fs::metadata(file_path) {
            metadata.file_size = file_metadata.len();
        }

        metadata
    }
}

impl ModuleResolver for FileSystemResolver {
    fn resolve_module(
        &mut self,
        import_path: &ImportPath,
        context: &ModuleLoadContext,
    ) -> ModuleResult<ResolvedModule> {
        // Resolve the import path to a module path
        let module_path = self.resolve_import_path(import_path, context)?;

        // Find the file for this module
        let file_path = self.find_module_file(&module_path, context)?;

        // Check for circular imports
        if self.visited_paths.contains(&file_path) {
            return Err(ModuleError::circular_dependency(
                &context.loading_stack,
                &module_path,
            ));
        }

        // Mark as visited
        self.visited_paths.insert(file_path.clone());

        // Load the module source
        let source = self.load_module_source(&file_path)?;

        // Create metadata
        let metadata = self.create_module_metadata(&module_path, &file_path);

        // Create resolved module
        let mut resolved_module =
            ResolvedModule::new(module_path, file_path.clone(), source.clone(), metadata);

        // Extract dependencies
        let dependencies = self.extract_dependencies(&source);
        for dep in dependencies {
            resolved_module.add_dependency(dep);
        }

        // Remove from visited (for future resolution cycles)
        self.visited_paths.remove(&file_path);

        Ok(resolved_module)
    }

    fn module_exists(&self, import_path: &ImportPath, context: &ModuleLoadContext) -> bool {
        if let Ok(module_path) = self.resolve_import_path(import_path, context) {
            self.find_module_file(&module_path, context).is_ok()
        } else {
            false
        }
    }

    fn search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }

    fn add_search_path(&mut self, path: PathBuf) {
        if !self.search_paths.contains(&path) {
            self.search_paths.push(path);
        }
    }
}

impl Default for ModuleResolverConfig {
    fn default() -> Self {
        Self {
            search_stdlib: true,
            search_external: true,
            follow_symlinks: true,
            max_depth: 10,
            file_extensions: vec!["script".to_string()],
            module_file_names: vec!["mod.script".to_string(), "index.script".to_string()],
            case_sensitive: cfg!(not(target_os = "windows")),
        }
    }
}

/// Composite resolver that tries multiple resolution strategies
pub struct CompositeResolver {
    resolvers: Vec<Box<dyn ModuleResolver>>,
}

impl CompositeResolver {
    pub fn new() -> Self {
        Self {
            resolvers: Vec::new(),
        }
    }

    pub fn add_resolver(&mut self, resolver: Box<dyn ModuleResolver>) {
        self.resolvers.push(resolver);
    }

    pub fn with_filesystem(mut self, config: ModuleResolverConfig) -> Self {
        self.add_resolver(Box::new(FileSystemResolver::new(config)));
        self
    }
}

impl ModuleResolver for CompositeResolver {
    fn resolve_module(
        &mut self,
        import_path: &ImportPath,
        context: &ModuleLoadContext,
    ) -> ModuleResult<ResolvedModule> {
        let mut last_error = None;

        for resolver in &mut self.resolvers {
            match resolver.resolve_module(import_path, context) {
                Ok(module) => return Ok(module),
                Err(e) => last_error = Some(e),
            }
        }

        Err(last_error.unwrap_or_else(|| ModuleError::not_found(import_path.to_string())))
    }

    fn module_exists(&self, import_path: &ImportPath, context: &ModuleLoadContext) -> bool {
        self.resolvers
            .iter()
            .any(|resolver| resolver.module_exists(import_path, context))
    }

    fn search_paths(&self) -> &[PathBuf] {
        // Return paths from the first resolver (for simplicity)
        if let Some(resolver) = self.resolvers.first() {
            resolver.search_paths()
        } else {
            &[]
        }
    }

    fn add_search_path(&mut self, path: PathBuf) {
        for resolver in &mut self.resolvers {
            resolver.add_search_path(path.clone());
        }
    }
}

impl Default for CompositeResolver {
    fn default() -> Self {
        Self::new().with_filesystem(ModuleResolverConfig::default())
    }
}

/// Module resolution facade that combines resolver with caching
pub struct ModuleResolutionEngine {
    resolver: Box<dyn ModuleResolver>,
    resolution_cache:
        std::collections::HashMap<(ImportPath, ModulePath), ModuleResult<ResolvedModule>>,
}

impl ModuleResolutionEngine {
    pub fn new(resolver: Box<dyn ModuleResolver>) -> Self {
        Self {
            resolver,
            resolution_cache: std::collections::HashMap::new(),
        }
    }

    pub fn resolve_with_cache(
        &mut self,
        import_path: &ImportPath,
        context: &ModuleLoadContext,
    ) -> ModuleResult<ResolvedModule> {
        let cache_key = (import_path.clone(), context.current_module.clone());

        // Check cache first
        if let Some(cached_result) = self.resolution_cache.get(&cache_key) {
            return cached_result.clone();
        }

        // Resolve and cache result
        let result = self.resolver.resolve_module(import_path, context);
        self.resolution_cache.insert(cache_key, result.clone());

        result
    }

    pub fn clear_cache(&mut self) {
        self.resolution_cache.clear();
    }

    pub fn cache_size(&self) -> usize {
        self.resolution_cache.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn create_test_context() -> ModuleLoadContext {
        let current_module = ModulePath::from_string("test.main").unwrap();
        let package_root = env::temp_dir().join("script_test");
        ModuleLoadContext::new(current_module, package_root)
    }

    #[test]
    fn test_filesystem_resolver_creation() {
        let config = ModuleResolverConfig::default();
        let resolver = FileSystemResolver::new(config);

        assert!(!resolver.search_paths.is_empty() || !resolver.config.search_stdlib);
    }

    #[test]
    fn test_import_path_resolution() {
        let resolver = FileSystemResolver::new(ModuleResolverConfig::default());
        let context = create_test_context();

        let absolute_import = ImportPath::new("foo.bar").unwrap();
        let resolved = resolver
            .resolve_import_path(&absolute_import, &context)
            .unwrap();
        assert_eq!(resolved.to_string(), "foo.bar");

        let relative_import = ImportPath::new("./sibling").unwrap();
        let resolved = resolver
            .resolve_import_path(&relative_import, &context)
            .unwrap();
        assert_eq!(resolved.to_string(), "test.sibling");
    }

    #[test]
    fn test_composite_resolver() {
        let mut composite = CompositeResolver::new();
        composite.add_resolver(Box::new(FileSystemResolver::new(
            ModuleResolverConfig::default(),
        )));

        let context = create_test_context();
        let import = ImportPath::new("nonexistent.module").unwrap();

        assert!(!composite.module_exists(&import, &context));
    }

    #[test]
    fn test_resolution_engine_caching() {
        let resolver = Box::new(FileSystemResolver::new(ModuleResolverConfig::default()));
        let mut engine = ModuleResolutionEngine::new(resolver);

        assert_eq!(engine.cache_size(), 0);

        let context = create_test_context();
        let import = ImportPath::new("test.module").unwrap();

        // This will fail because the module doesn't exist, but it will be cached
        let _ = engine.resolve_with_cache(&import, &context);
        assert_eq!(engine.cache_size(), 1);

        engine.clear_cache();
        assert_eq!(engine.cache_size(), 0);
    }

    #[test]
    fn test_module_path_file_resolution() {
        let module_path = ModulePath::from_string("foo.bar").unwrap();
        let base_path = Path::new("/test");

        let file_path = module_path.to_file_path(base_path);
        assert_eq!(file_path, PathBuf::from("/test/foo/bar.script"));

        let dir_path = module_path.to_dir_module_path(base_path);
        assert_eq!(dir_path, PathBuf::from("/test/foo/bar/mod.script"));
    }

    #[test]
    fn test_resolver_config_defaults() {
        let config = ModuleResolverConfig::default();
        assert!(config.search_stdlib);
        assert!(config.search_external);
        assert!(config.follow_symlinks);
        assert_eq!(config.max_depth, 10);
        assert!(config.file_extensions.contains(&"script".to_string()));
    }
}
