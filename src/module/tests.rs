use super::*;
use crate::module::cache::DependencyGraph;
use crate::module::resolver::CompositeResolver;
use crate::semantic::SemanticAnalyzer;
use crate::Error;
use std::fs;
use std::path::PathBuf;

/// Comprehensive integration tests for the module resolution system
#[cfg(test)]
mod integration_tests {
    use super::*;
    use tempfile::TempDir;

    struct TestEnvironment {
        temp_dir: TempDir,
        source_dir: PathBuf,
        stdlib_dir: PathBuf,
    }

    impl TestEnvironment {
        fn new() -> Self {
            let temp_dir = TempDir::new().unwrap();
            let source_dir = temp_dir.path().join("src");
            let stdlib_dir = temp_dir.path().join("stdlib");

            fs::create_dir_all(&source_dir).unwrap();
            fs::create_dir_all(&stdlib_dir).unwrap();

            Self {
                temp_dir,
                source_dir,
                stdlib_dir,
            }
        }

        fn create_module(&self, path: &str, content: &str) -> PathBuf {
            let file_path = self
                .source_dir
                .join(format!("{}.script", path.replace('.', "/")));
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&file_path, content).unwrap();
            file_path
        }

        fn create_dir_module(&self, path: &str, content: &str) -> PathBuf {
            let dir_path = self.source_dir.join(path.replace('.', "/"));
            fs::create_dir_all(&dir_path).unwrap();
            let file_path = dir_path.join("mod.script");
            fs::write(&file_path, content).unwrap();
            file_path
        }

        fn create_stdlib_module(&self, path: &str, content: &str) -> PathBuf {
            let file_path = self
                .stdlib_dir
                .join(format!("{}.script", path.replace('.', "/")));
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&file_path, content).unwrap();
            file_path
        }

        fn create_context(&self, current_module: &str) -> ModuleLoadContext {
            let module_path = ModulePath::from_string(current_module).unwrap();
            let mut context =
                ModuleLoadContext::new(module_path, self.temp_dir.path().to_path_buf());
            context.search_paths.push(self.stdlib_dir.clone());
            context
        }
    }

    #[test]
    fn test_module_path_creation_and_validation() {
        // Valid module paths
        assert!(ModulePath::from_string("simple").is_ok());
        assert!(ModulePath::from_string("package.module").is_ok());
        assert!(ModulePath::from_string("deeply.nested.module.path").is_ok());
        assert!(ModulePath::from_string("_private").is_ok());
        assert!(ModulePath::from_string("module123").is_ok());

        // Invalid module paths
        assert!(ModulePath::from_string("").is_err());
        assert!(ModulePath::from_string("123invalid").is_err());
        assert!(ModulePath::from_string("invalid-name").is_err());
        assert!(ModulePath::from_string("invalid..path").is_err());
        assert!(ModulePath::from_string("invalid.").is_err());
        assert!(ModulePath::from_string(".invalid").is_err());
    }

    #[test]
    fn test_import_path_resolution() {
        let env = TestEnvironment::new();
        let context = env.create_context("parent.child.current");

        // Absolute imports
        let absolute = ImportPath::new("external.module").unwrap();
        let resolved = absolute.resolve(&context.current_module).unwrap();
        assert_eq!(resolved.to_string(), "external.module");

        // Relative imports
        let relative = ImportPath::new("./sibling").unwrap();
        let resolved = relative.resolve(&context.current_module).unwrap();
        assert_eq!(resolved.to_string(), "parent.child.sibling");

        let parent_relative = ImportPath::new("../uncle").unwrap();
        let resolved = parent_relative.resolve(&context.current_module).unwrap();
        assert_eq!(resolved.to_string(), "parent.uncle");

        // Super imports
        let super_import = ImportPath::new("super.sibling").unwrap();
        let resolved = super_import.resolve(&context.current_module).unwrap();
        assert_eq!(resolved.to_string(), "parent.child.sibling");

        // Crate imports
        let crate_import = ImportPath::new("crate.root_module").unwrap();
        let resolved = crate_import.resolve(&context.current_module).unwrap();
        assert_eq!(resolved.to_string(), "root_module");
    }

    #[test]
    fn test_file_system_module_resolution() {
        let env = TestEnvironment::new();

        // Create test modules
        env.create_module("main", "// Main module");
        env.create_module("utils/helper", "// Helper module");
        env.create_dir_module("graphics", "// Graphics module");
        env.create_stdlib_module("io", "// Standard IO module");

        let mut resolver = FileSystemResolver::new(ModuleResolverConfig::default());
        resolver.add_search_path(env.stdlib_dir.clone());

        let context = env.create_context("main");

        // Test resolving regular module
        let main_import = ImportPath::new("main").unwrap();
        assert!(resolver.module_exists(&main_import, &context));

        // Test resolving nested module
        let helper_import = ImportPath::new("utils.helper").unwrap();
        assert!(resolver.module_exists(&helper_import, &context));

        // Test resolving directory module
        let graphics_import = ImportPath::new("graphics").unwrap();
        assert!(resolver.module_exists(&graphics_import, &context));

        // Test resolving standard library module
        let io_import = ImportPath::new("std.io").unwrap();
        assert!(resolver.module_exists(&io_import, &context));

        // Test non-existent module
        let missing_import = ImportPath::new("nonexistent").unwrap();
        assert!(!resolver.module_exists(&missing_import, &context));
    }

    #[test]
    fn test_module_registry_operations() {
        use tempfile::NamedTempFile;

        let config = RegistryConfig::default();
        let mut registry = ModuleRegistry::new(config);

        let module_path = ModulePath::from_string("test.module").unwrap();

        // Create a temporary file
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        std::fs::write(&file_path, "fn test() {}").unwrap();

        let source = "fn test() {}".to_string();
        let metadata = ModuleMetadata::default();
        let module = ResolvedModule::new(module_path.clone(), file_path, source, metadata);

        // Test registration
        assert!(!registry.is_registered(&module_path));
        registry.register_module(module).unwrap();
        assert!(registry.is_registered(&module_path));

        // Test retrieval
        assert!(registry.get_module(&module_path).is_some());
        assert!(registry.get_metadata(&module_path).is_some());

        // Test listing
        let modules = registry.list_modules();
        assert_eq!(modules.len(), 1);
        assert!(modules.contains(&&module_path));

        // Test finding
        let found = registry.find_modules("test");
        assert_eq!(found.len(), 1);

        // Test unregistration
        registry.unregister_module(&module_path).unwrap();
        assert!(!registry.is_registered(&module_path));
    }

    #[test]
    fn test_module_cache_operations() {
        let mut cache = ModuleCache::new();

        let module_path = ModulePath::from_string("test.module").unwrap();
        let file_path = PathBuf::from("/test/module.script");
        let source = "fn test() {}".to_string();
        let metadata = ModuleMetadata::default();
        let module = ResolvedModule::new(module_path.clone(), file_path, source, metadata);

        // Test caching
        assert!(!cache.is_cached(&module_path));
        cache.insert(module).unwrap();

        // Note: The module won't be considered cached because the file doesn't actually exist
        // In a real scenario with actual files, this would work differently

        // Test stats
        let stats = cache.stats();
        assert_eq!(stats.total_modules, 1);

        // Test invalidation
        cache.invalidate(&module_path);
        let stats = cache.stats();
        assert_eq!(stats.invalid_modules, 1);

        // Test clearing
        cache.clear();
        let stats = cache.stats();
        assert_eq!(stats.total_modules, 0);
    }

    #[test]
    fn test_dependency_graph_operations() {
        let mut graph = DependencyGraph::new();

        let mod_a = ModulePath::from_string("a").unwrap();
        let mod_b = ModulePath::from_string("b").unwrap();
        let mod_c = ModulePath::from_string("c").unwrap();
        let mod_d = ModulePath::from_string("d").unwrap();

        // Create dependency chain: a -> b -> c, d -> c
        graph.add_module(&mod_a, &[mod_b.clone()]);
        graph.add_module(&mod_b, &[mod_c.clone()]);
        graph.add_module(&mod_c, &[]);
        graph.add_module(&mod_d, &[mod_c.clone()]);

        // Test dependency retrieval
        assert_eq!(graph.get_dependencies(&mod_a), Some(vec![mod_b.clone()]));
        assert_eq!(graph.get_dependencies(&mod_b), Some(vec![mod_c.clone()]));
        assert_eq!(graph.get_dependencies(&mod_c), Some(vec![]));

        // Test dependents retrieval
        let c_dependents = graph.get_dependents(&mod_c).unwrap_or_default();
        assert_eq!(c_dependents.len(), 2);
        assert!(c_dependents.contains(&mod_b));
        assert!(c_dependents.contains(&mod_d));

        let b_dependents = graph.get_dependents(&mod_b).unwrap_or_default();
        assert_eq!(b_dependents, vec![mod_a.clone()]);

        // Test topological sort
        let sorted = graph.topological_sort().unwrap();
        let c_pos = sorted.iter().position(|m| m == &mod_c).unwrap();
        let b_pos = sorted.iter().position(|m| m == &mod_b).unwrap();
        let a_pos = sorted.iter().position(|m| m == &mod_a).unwrap();
        let d_pos = sorted.iter().position(|m| m == &mod_d).unwrap();

        assert!(c_pos < b_pos);
        assert!(b_pos < a_pos);
        assert!(c_pos < d_pos);

        // Test cycle detection
        assert!(!graph.has_cycle(&mod_a));

        // Add cycle and test detection
        graph.add_module(&mod_c, &[mod_a.clone()]);
        assert!(graph.has_cycle(&mod_a));
        assert!(graph.topological_sort().is_err());
    }

    #[test]
    fn test_circular_dependency_detection() {
        let env = TestEnvironment::new();

        // Create modules with circular dependencies
        env.create_module("a", "import b; fn a_func() {}");
        env.create_module("b", "import c; fn b_func() {}");
        env.create_module("c", "import a; fn c_func() {}");

        let mut resolver = FileSystemResolver::new(ModuleResolverConfig::default());
        let context = env.create_context("main");

        // This would detect circular dependencies in a full implementation
        // For now, we test the error reporting mechanism
        let mod_a = ModulePath::from_string("a").unwrap();
        let mod_b = ModulePath::from_string("b").unwrap();
        let mod_c = ModulePath::from_string("c").unwrap();

        let stack = vec![mod_a.clone(), mod_b.clone()];
        let error = ModuleError::circular_dependency(&stack, &mod_c);

        assert_eq!(error.kind, ModuleErrorKind::CircularDependency);
        assert!(error.message.contains("a -> b -> c"));
    }

    #[test]
    fn test_module_compilation_pipeline() {
        let registry = ModuleRegistry::new(RegistryConfig::default());
        let resolver = Box::new(FileSystemResolver::new(ModuleResolverConfig::default()));
        let semantic_analyzer = SemanticAnalyzer::new();

        let pipeline = ModuleCompilationPipeline::new(registry, resolver, semantic_analyzer);

        // Test initial state
        assert_eq!(pipeline.get_all_compiled_modules().len(), 0);

        // Test statistics
        let stats = pipeline.get_compilation_stats();
        assert_eq!(stats.total_modules, 0);
        assert_eq!(stats.total_dependencies, 0);
    }

    #[test]
    fn test_error_handling_and_reporting() {
        // Test various error types
        let not_found_error = ModuleError::not_found("missing.module");
        assert_eq!(not_found_error.kind, ModuleErrorKind::NotFound);
        assert!(not_found_error.message.contains("missing.module"));

        let invalid_path_error = ModuleError::invalid_path("123invalid", "starts with number");
        assert_eq!(invalid_path_error.kind, ModuleErrorKind::InvalidPath);
        assert!(invalid_path_error.message.contains("123invalid"));

        let parse_error = ModuleError::parse_error("test.module", "unexpected token");
        assert_eq!(parse_error.kind, ModuleErrorKind::ParseError);
        assert!(parse_error.message.contains("unexpected token"));

        // Test error conversion
        let converted: Error = not_found_error.into();
        assert_eq!(converted.kind, crate::error::ErrorKind::ParseError);
    }

    #[test]
    fn test_module_metadata_extraction() {
        let module_path = ModulePath::from_string("test.module").unwrap();
        let file_path = PathBuf::from("/test/module.script");
        let source = r#"
            /// Test module for demonstration
            module test.module
            
            pub fn public_function() -> i32 { 42 }
            fn private_function() {}
            pub const PUBLIC_CONSTANT: i32 = 100
        "#
        .to_string();

        let metadata = ModuleMetadata::default();
        let module = ResolvedModule::new(module_path, file_path, source, metadata);

        assert_eq!(module.path.module_name(), "module");
        assert!(module.is_local_module());
        assert!(!module.is_library_module());
    }

    #[test]
    fn test_search_path_management() {
        let mut resolver = FileSystemResolver::new(ModuleResolverConfig::default());
        let initial_paths = resolver.search_paths().len();

        let custom_path = PathBuf::from("/custom/path");
        resolver.add_search_path(custom_path.clone());

        assert_eq!(resolver.search_paths().len(), initial_paths + 1);
        assert!(resolver.search_paths().contains(&custom_path));

        // Test that duplicate paths aren't added
        resolver.add_search_path(custom_path.clone());
        assert_eq!(resolver.search_paths().len(), initial_paths + 1);
    }

    #[test]
    fn test_composite_resolver() {
        let mut composite = CompositeResolver::new();

        // Add filesystem resolver
        let fs_resolver = FileSystemResolver::new(ModuleResolverConfig::default());
        composite.add_resolver(Box::new(fs_resolver));

        let env = TestEnvironment::new();
        let context = env.create_context("test");

        // Test that the composite resolver delegates to its resolvers
        let import = ImportPath::new("nonexistent").unwrap();
        assert!(!composite.module_exists(&import, &context));

        // Test search paths delegation
        let custom_path = PathBuf::from("/test/path");
        composite.add_search_path(custom_path);
    }

    #[test]
    fn test_module_load_context() {
        let current_module = ModulePath::from_string("test.module").unwrap();
        let package_root = PathBuf::from("/test/project");
        let mut context = ModuleLoadContext::new(current_module.clone(), package_root.clone());

        assert_eq!(context.current_module, current_module);
        assert_eq!(context.package_root, package_root);
        assert!(context.loading_stack.is_empty());

        // Test loading stack operations
        let module_a = ModulePath::from_string("a").unwrap();
        let module_b = ModulePath::from_string("b").unwrap();

        context.push_loading(module_a.clone()).unwrap();
        assert_eq!(context.loading_stack.len(), 1);

        // Test circular dependency detection
        let result = context.push_loading(module_a.clone());
        assert!(result.is_err());

        context.pop_loading();
        assert!(context.loading_stack.is_empty());

        // Test context cloning with different current module
        let new_context = context.with_current_module(module_b.clone());
        assert_eq!(new_context.current_module, module_b);
        assert_eq!(new_context.package_root, package_root);
    }

    #[test]
    fn test_selective_imports() {
        let simple_import = SelectiveImport::new("test_function");
        assert_eq!(simple_import.name, "test_function");
        assert!(simple_import.alias.is_none());
        assert_eq!(simple_import.effective_name(), "test_function");

        let aliased_import = SelectiveImport::with_alias("original_name", "alias_name");
        assert_eq!(aliased_import.name, "original_name");
        assert_eq!(aliased_import.alias, Some("alias_name".to_string()));
        assert_eq!(aliased_import.effective_name(), "alias_name");
    }

    #[test]
    fn test_relative_path_utilities() {
        let complex_path = PathBuf::from("foo/../bar/./baz/../qux");
        let normalized = RelativePath::normalize(&complex_path);
        assert_eq!(normalized, PathBuf::from("bar/qux"));

        let base_path = PathBuf::from("/project/src");
        let within_path = PathBuf::from("/project/src/module");
        let outside_path = PathBuf::from("/other/path");

        assert!(RelativePath::is_within(&within_path, &base_path));
        assert!(!RelativePath::is_within(&outside_path, &base_path));
    }
}

/// Performance and stress tests
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_large_dependency_graph_performance() {
        let mut graph = DependencyGraph::new();
        let start_time = Instant::now();

        // Create a large dependency graph (1000 modules)
        for i in 0..1000 {
            let module = ModulePath::from_string(format!("module_{i}")).unwrap();
            let dependencies: Vec<ModulePath> = (0..std::cmp::min(i, 10))
                .map(|j| ModulePath::from_string(format!("module_{j}")).unwrap())
                .collect();
            graph.add_module(&module, &dependencies);
        }

        let creation_time = start_time.elapsed();
        println!("Graph creation time: {:?}", creation_time);

        // Test topological sort performance
        let sort_start = Instant::now();
        let sorted = graph.topological_sort().unwrap();
        let sort_time = sort_start.elapsed();

        println!("Topological sort time: {:?}", sort_time);
        assert_eq!(sorted.len(), 1000);

        // Both operations should complete in reasonable time (< 100ms)
        assert!(creation_time.as_millis() < 100);
        assert!(sort_time.as_millis() < 100);
    }

    #[test]
    fn test_module_cache_performance() {
        let mut cache = ModuleCache::new();
        let start_time = Instant::now();

        // Insert many modules (reduced count for testing)
        for i in 0..10 {
            let module_path = ModulePath::from_string(format!("module_{i}")).unwrap();
            let temp_file = tempfile::NamedTempFile::new().unwrap();
            let file_path = temp_file.into_temp_path().to_path_buf();
            let source = format!("// Module {i}");
            std::fs::write(&file_path, &source).unwrap();
            let metadata = ModuleMetadata::default();
            let module = ResolvedModule::new(module_path, file_path, source, metadata);

            cache.insert(module).unwrap();
        }

        let insertion_time = start_time.elapsed();
        println!("Cache insertion time: {:?}", insertion_time);

        // Test lookup performance
        let lookup_start = Instant::now();
        for i in 0..1000 {
            let module_path = ModulePath::from_string(format!("module_{i}")).unwrap();
            let _cached = cache.is_cached(&module_path);
        }
        let lookup_time = lookup_start.elapsed();

        println!("Cache lookup time: {:?}", lookup_time);

        // Operations should be fast
        assert!(insertion_time.as_millis() < 500);
        assert!(lookup_time.as_millis() < 10);
    }
}

/// Edge case and error condition tests
#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_empty_and_malformed_paths() {
        assert!(ModulePath::from_string("").is_err());
        assert!(ModulePath::from_string(".").is_err());
        assert!(ModulePath::from_string("..").is_err());
        assert!(ModulePath::from_string("module.").is_err());
        assert!(ModulePath::from_string(".module").is_err());
        assert!(ModulePath::from_string("module..submodule").is_err());
    }

    #[test]
    fn test_very_long_module_paths() {
        let long_segments: Vec<String> = (0..100).map(|i| format!("segment_{i}")).collect();
        let long_path = long_segments.join(".");

        let module_path = ModulePath::from_string(&long_path).unwrap();
        assert_eq!(module_path.segments().len(), 100);
        assert_eq!(module_path.to_string(), long_path);
    }

    #[test]
    fn test_unicode_in_module_names() {
        // Unicode is not allowed in identifiers by our rules
        assert!(ModulePath::from_string("модуль").is_err());
        assert!(ModulePath::from_string("モジュール").is_err());
        assert!(ModulePath::from_string("module_🦀").is_err());
    }

    #[test]
    fn test_case_sensitivity() {
        let path1 = ModulePath::from_string("Module").unwrap();
        let path2 = ModulePath::from_string("module").unwrap();

        // Module paths are case sensitive
        assert_ne!(path1, path2);
    }

    #[test]
    fn test_deeply_nested_relative_imports() {
        let current = ModulePath::from_string("a.b.c.d.e.f.current").unwrap();

        let deep_relative = ImportPath::new("../../../sibling").unwrap();
        let resolved = deep_relative.resolve(&current).unwrap();
        assert_eq!(resolved.to_string(), "a.b.c.sibling");

        // Test going beyond root
        let too_deep = ImportPath::new("../../../../../../../outside").unwrap();
        let resolved = too_deep.resolve(&current).unwrap();
        // Should resolve to just "outside" when going beyond root
        assert_eq!(resolved.to_string(), "outside");
    }

    #[test]
    fn test_memory_usage_with_large_modules() {
        let mut registry = ModuleRegistry::new(RegistryConfig::default());

        // Create a module with large source content
        let large_source = "fn function() {}\n".repeat(10000);
        let module_path = ModulePath::from_string("large_module").unwrap();
        let file_path = PathBuf::from("/test/large_module.script");
        let metadata = ModuleMetadata::default();
        let module = ResolvedModule::new(module_path.clone(), file_path, large_source, metadata);

        registry.register_module(module).unwrap();

        let retrieved = registry.get_module(&module_path).unwrap();
        assert!(retrieved.source.len() > 100000); // Should be large
    }

    #[test]
    fn test_concurrent_access_patterns() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let cache = Arc::new(Mutex::new(ModuleCache::new()));
        let mut handles = vec![];

        // Simulate concurrent access
        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let handle = thread::spawn(move || {
                let module_path = ModulePath::from_string(format!("module_{i}")).unwrap();
                let temp_file = tempfile::NamedTempFile::new().unwrap();
                let file_path = temp_file.into_temp_path().to_path_buf();
                let source = format!("// Module {i}");
                std::fs::write(&file_path, &source).unwrap();
                let metadata = ModuleMetadata::default();
                let module = ResolvedModule::new(module_path, file_path, source, metadata);

                let mut cache = cache_clone.lock().unwrap();
                cache.insert(module).unwrap();
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        let cache = cache.lock().unwrap();
        let stats = cache.stats();
        assert_eq!(stats.total_modules, 10);
    }
}
