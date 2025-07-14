use script::module::{create_default_pipeline, ModuleLoadContext, ModulePath};
use std::env;

#[test]
fn test_module_type_integration() {
    // Create a test pipeline
    let mut pipeline = create_default_pipeline();

    // Create a test context
    let current_module = ModulePath::from_string("test.main").unwrap();
    let package_root = env::temp_dir().join("script_test");
    let context = ModuleLoadContext::new(current_module, package_root);

    // Test that the pipeline can be created without panicking
    // This is a basic smoke test - in a real test we'd create actual module files
    assert_eq!(pipeline.get_all_compiled_modules().len(), 0);

    // Test compilation configuration
    let config = script::module::CompilationConfig::default();
    assert!(config.enable_caching);
    assert!(config.dependency_validation);
}

#[test]
fn test_module_exports_structure() {
    use script::module::{ExportVisibility, FunctionExportInfo, ModuleExports};
    use script::semantic::{FunctionSignature, SymbolTable};
    use std::collections::HashMap;

    // Test that we can create the new ModuleExports structure
    let symbol_table = SymbolTable::new();
    let exports = ModuleExports {
        symbols: symbol_table,
        type_definitions: HashMap::new(),
        functions: HashMap::new(),
        variables: HashMap::new(),
        re_exports: HashMap::new(),
    };

    // Verify the structure is properly initialized
    assert_eq!(exports.functions.len(), 0);
    assert_eq!(exports.variables.len(), 0);
    assert_eq!(exports.type_definitions.len(), 0);
    assert_eq!(exports.re_exports.len(), 0);
}
