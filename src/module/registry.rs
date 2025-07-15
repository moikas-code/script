use crate::module::{ModuleCache, ModulePath, ModuleResult, ResolvedModule};
use std::collections::HashMap;
use std::time::SystemTime;

/// Central registry for managing loaded modules and their metadata
#[derive(Debug)]
pub struct ModuleRegistry {
    modules: HashMap<ModulePath, ResolvedModule>,
    metadata: HashMap<ModulePath, ModuleMetadata>,
    cache: ModuleCache,
    config: RegistryConfig,
}

/// Metadata associated with a module
#[derive(Debug, Clone)]
pub struct ModuleMetadata {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub load_time: SystemTime,
    pub file_size: u64,
    pub exports: Vec<ExportInfo>,
    pub imports: Vec<ImportInfo>,
}

/// Information about exported symbols from a module
#[derive(Debug, Clone)]
pub struct ExportInfo {
    pub name: String,
    pub kind: ExportKind,
    pub visibility: Visibility,
    pub documentation: Option<String>,
}

/// Information about imported symbols into a module
#[derive(Debug, Clone)]
pub struct ImportInfo {
    pub module_path: ModulePath,
    pub imported_name: String,
    pub local_name: String,
    pub kind: ImportKind,
}

/// Types of exports
#[derive(Debug, Clone, PartialEq)]
pub enum ExportKind {
    Function,
    Type,
    Constant,
    Variable,
    Module,
}

/// Types of imports
#[derive(Debug, Clone, PartialEq)]
pub enum ImportKind {
    Named,    // import { foo }
    Default,  // import foo
    Wildcard, // import *
    Aliased,  // import { foo as bar }
}

/// Visibility levels for exports
#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Public,         // pub
    Private,        // default
    Crate,          // pub(crate)
    Super,          // pub(super)
    Module(String), // pub(in module)
}

/// Configuration for the module registry
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    pub cache_enabled: bool,
    pub max_cache_size: usize,
    pub preload_std: bool,
    pub track_dependencies: bool,
    pub enable_hot_reload: bool,
}

impl ModuleRegistry {
    pub fn new(config: RegistryConfig) -> Self {
        Self {
            modules: HashMap::new(),
            metadata: HashMap::new(),
            cache: ModuleCache::new(),
            config,
        }
    }

    /// Register a loaded module in the registry
    pub fn register_module(&mut self, module: ResolvedModule) -> ModuleResult<()> {
        let module_path = module.path.clone();

        // Extract metadata from the module
        let metadata = self.extract_metadata(&module)?;

        // Cache the module if caching is enabled
        if self.config.cache_enabled {
            self.cache.insert(module.clone())?;
        }

        // Store in registry
        self.metadata.insert(module_path.clone(), metadata);
        self.modules.insert(module_path, module);

        Ok(())
    }

    /// Get a module from the registry
    pub fn get_module(&self, module_path: &ModulePath) -> Option<&ResolvedModule> {
        // Try cache first if enabled
        if self.config.cache_enabled {
            if let Some(cached) = self.cache.get(module_path) {
                return Some(cached);
            }
        }

        self.modules.get(module_path)
    }

    /// Check if a module is registered
    pub fn is_registered(&self, module_path: &ModulePath) -> bool {
        self.modules.contains_key(module_path)
            || (self.config.cache_enabled && self.cache.is_cached(module_path))
    }

    /// Get metadata for a module
    pub fn get_metadata(&self, module_path: &ModulePath) -> Option<&ModuleMetadata> {
        self.metadata.get(module_path)
    }

    /// Unregister a module and invalidate its cache
    pub fn unregister_module(&mut self, module_path: &ModulePath) -> ModuleResult<()> {
        self.modules.remove(module_path);
        self.metadata.remove(module_path);

        if self.config.cache_enabled {
            self.cache.invalidate(module_path);
        }

        Ok(())
    }

    /// Get all registered modules
    pub fn list_modules(&self) -> Vec<&ModulePath> {
        self.modules.keys().collect()
    }

    /// Find modules by pattern
    pub fn find_modules(&self, pattern: &str) -> Vec<&ModulePath> {
        self.modules
            .keys()
            .filter(|path| path.to_string().contains(pattern))
            .collect()
    }

    /// Get exports from a module
    pub fn get_exports(&self, module_path: &ModulePath) -> Vec<&ExportInfo> {
        if let Some(metadata) = self.metadata.get(module_path) {
            metadata.exports.iter().collect()
        } else {
            Vec::new()
        }
    }

    /// Get imports of a module
    pub fn get_imports(&self, module_path: &ModulePath) -> Vec<&ImportInfo> {
        if let Some(metadata) = self.metadata.get(module_path) {
            metadata.imports.iter().collect()
        } else {
            Vec::new()
        }
    }

    /// Check if a symbol is exported by a module
    pub fn has_export(&self, module_path: &ModulePath, symbol_name: &str) -> bool {
        if let Some(metadata) = self.metadata.get(module_path) {
            metadata
                .exports
                .iter()
                .any(|export| export.name == symbol_name)
        } else {
            false
        }
    }

    /// Get dependency graph information
    pub fn get_dependencies(&self, module_path: &ModulePath) -> Vec<ModulePath> {
        if let Some(module) = self.modules.get(module_path) {
            module
                .dependencies
                .iter()
                .filter_map(|import_path| {
                    // This would need actual resolution logic in practice
                    ModulePath::from_string(&import_path.path).ok()
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get dependents of a module
    pub fn get_dependents(&self, module_path: &ModulePath) -> Vec<ModulePath> {
        self.modules
            .iter()
            .filter_map(|(path, module)| {
                if module.dependencies.iter().any(|dep| {
                    ModulePath::from_string(&dep.path).ok().as_ref() == Some(module_path)
                }) {
                    Some(path.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Clear the registry
    pub fn clear(&mut self) {
        self.modules.clear();
        self.metadata.clear();
        if self.config.cache_enabled {
            self.cache.clear();
        }
    }

    /// Get registry statistics
    pub fn stats(&self) -> RegistryStats {
        let total_modules = self.modules.len();
        let total_exports: usize = self.metadata.values().map(|m| m.exports.len()).sum();
        let total_imports: usize = self.metadata.values().map(|m| m.imports.len()).sum();

        let cache_stats = if self.config.cache_enabled {
            Some(self.cache.stats())
        } else {
            None
        };

        RegistryStats {
            total_modules,
            total_exports,
            total_imports,
            cache_stats,
        }
    }

    /// Hot reload a module if supported
    pub fn hot_reload(&mut self, module_path: &ModulePath) -> ModuleResult<bool> {
        if !self.config.enable_hot_reload {
            return Ok(false);
        }

        if let Some(module) = self.modules.get(module_path) {
            // Check if file has changed
            let file_path = &module.file_path;
            if file_path.exists() {
                // In a real implementation, we would:
                // 1. Check file modification time
                // 2. Re-parse the module
                // 3. Update the registry
                // 4. Notify dependents

                // For now, just invalidate cache
                if self.config.cache_enabled {
                    self.cache.invalidate(module_path);
                }
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn extract_metadata(&self, module: &ResolvedModule) -> ModuleResult<ModuleMetadata> {
        // In a real implementation, this would parse the module source
        // to extract exports, imports, and documentation

        let file_size = if module.file_path.exists() {
            std::fs::metadata(&module.file_path)
                .map(|m| m.len())
                .unwrap_or(0)
        } else {
            module.source.len() as u64
        };

        Ok(ModuleMetadata {
            name: module.path.module_name().to_string(),
            version: None,
            description: None,
            authors: Vec::new(),
            license: None,
            repository: None,
            load_time: SystemTime::now(),
            file_size,
            exports: Vec::new(), // Would be populated by parsing
            imports: Vec::new(), // Would be populated by parsing
        })
    }
}

impl Default for ModuleMetadata {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: None,
            description: None,
            authors: Vec::new(),
            license: None,
            repository: None,
            load_time: SystemTime::now(),
            file_size: 0,
            exports: Vec::new(),
            imports: Vec::new(),
        }
    }
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            cache_enabled: true,
            max_cache_size: 1000,
            preload_std: true,
            track_dependencies: true,
            enable_hot_reload: false,
        }
    }
}

/// Statistics about the module registry
#[derive(Debug, Clone)]
pub struct RegistryStats {
    pub total_modules: usize,
    pub total_exports: usize,
    pub total_imports: usize,
    pub cache_stats: Option<crate::module::cache::CacheStats>,
}

impl RegistryStats {
    pub fn average_exports_per_module(&self) -> f64 {
        if self.total_modules == 0 {
            0.0
        } else {
            self.total_exports as f64 / self.total_modules as f64
        }
    }

    pub fn average_imports_per_module(&self) -> f64 {
        if self.total_modules == 0 {
            0.0
        } else {
            self.total_imports as f64 / self.total_modules as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_module(name: &str) -> ResolvedModule {
        use tempfile::NamedTempFile;

        let path = ModulePath::from_string(name).unwrap();

        // Create a temporary file
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.into_temp_path().to_path_buf();
        let source = format!("// Module {name}");
        std::fs::write(&file_path, &source).unwrap();

        let metadata = ModuleMetadata::default();

        ResolvedModule::new(path, file_path, source, metadata)
    }

    #[test]
    fn test_registry_basic_operations() {
        let config = RegistryConfig::default();
        let mut registry = ModuleRegistry::new(config);

        let module = create_test_module("test");
        let module_path = module.path.clone();

        assert!(!registry.is_registered(&module_path));

        registry.register_module(module).unwrap();

        assert!(registry.is_registered(&module_path));
        assert!(registry.get_module(&module_path).is_some());
        assert!(registry.get_metadata(&module_path).is_some());
    }

    #[test]
    fn test_registry_find_modules() {
        let config = RegistryConfig::default();
        let mut registry = ModuleRegistry::new(config);

        registry
            .register_module(create_test_module("test.foo"))
            .unwrap();
        registry
            .register_module(create_test_module("test.bar"))
            .unwrap();
        registry
            .register_module(create_test_module("other.baz"))
            .unwrap();

        let test_modules = registry.find_modules("test");
        assert_eq!(test_modules.len(), 2);

        let foo_modules = registry.find_modules("foo");
        assert_eq!(foo_modules.len(), 1);
    }

    #[test]
    fn test_export_info() {
        let export = ExportInfo {
            name: "test_function".to_string(),
            kind: ExportKind::Function,
            visibility: Visibility::Public,
            documentation: Some("A test function".to_string()),
        };

        assert_eq!(export.name, "test_function");
        assert_eq!(export.kind, ExportKind::Function);
        assert_eq!(export.visibility, Visibility::Public);
    }

    #[test]
    fn test_import_info() {
        let module_path = ModulePath::from_string("test.module").unwrap();
        let import = ImportInfo {
            module_path,
            imported_name: "foo".to_string(),
            local_name: "bar".to_string(),
            kind: ImportKind::Aliased,
        };

        assert_eq!(import.imported_name, "foo");
        assert_eq!(import.local_name, "bar");
        assert_eq!(import.kind, ImportKind::Aliased);
    }

    #[test]
    fn test_registry_stats() {
        let config = RegistryConfig::default();
        let mut registry = ModuleRegistry::new(config);

        registry
            .register_module(create_test_module("test1"))
            .unwrap();
        registry
            .register_module(create_test_module("test2"))
            .unwrap();

        let stats = registry.stats();
        assert_eq!(stats.total_modules, 2);
        assert!(stats.cache_stats.is_some());
    }
}
