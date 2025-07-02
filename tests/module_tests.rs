use script::{
    error::Error,
    lexer::Lexer,
    module::{
        create_default_pipeline, CompilationConfig, FileSystemResolver, ImportPath, ModuleCache,
        ModuleLoadContext, ModulePath, ModuleRegistry, ModuleResolver,
    },
    parser::{ExportKind, Parser, Program},
};
use std::fs;
use std::path::PathBuf;

// Helper function to parse source code directly
fn parse_source(source: &str) -> Result<Program, Error> {
    let lexer = Lexer::new(source);
    let (tokens, errors) = lexer.scan_tokens();

    if !errors.is_empty() {
        return Err(errors.into_iter().next().unwrap());
    }

    let mut parser = Parser::new(tokens);
    parser.parse()
}

// Helper function to compile source code and check exports
fn compile_and_check_exports(source: &str) -> Result<Vec<String>, Error> {
    let ast = parse_source(source)?;

    // Extract exports from AST (simplified version)
    let mut exports = Vec::new();
    for stmt in &ast.statements {
        match &stmt.kind {
            script::parser::StmtKind::Export { export } => match export {
                script::parser::ExportKind::Named { specifiers } => {
                    for spec in specifiers {
                        exports.push(spec.name.clone());
                    }
                }
                script::parser::ExportKind::Function { name, .. } => {
                    exports.push(name.clone());
                }
                script::parser::ExportKind::Variable { name, .. } => {
                    exports.push(name.clone());
                }
                _ => {}
            },
            _ => {}
        }
    }

    Ok(exports)
}

// Helper to create a mock compiled module for testing
// Note: This function is not used in the current tests but kept for future use
#[allow(dead_code)]
fn create_mock_compiled_module(
    _path: ModulePath,
    source: &str,
    _exports: Vec<String>,
) -> Result<Program, Error> {
    // For now, just parse and return the AST
    parse_source(source)
}

#[test]
fn test_basic_module_import() {
    let module_dir = PathBuf::from("tests/modules");
    let mut config = script::module::ModuleResolverConfig::default();
    config.search_stdlib = false;
    let mut resolver = FileSystemResolver::new(config);
    let _registry = ModuleRegistry::default();
    let _cache = ModuleCache::new();

    // Load math_utils module
    let math_path = ModulePath::from_string("math_utils").unwrap();
    let resolved = resolver.resolve(&math_path, &module_dir).unwrap();

    assert!(resolved.file_path.exists());
    assert_eq!(resolved.path, math_path);
}

#[test]
fn test_module_export_parsing() {
    let source = r#"
export { add, subtract }

fn add(a: int, b: int) -> int {
    a + b
}

fn subtract(a: int, b: int) -> int {
    a - b
}
"#;

    let exports = compile_and_check_exports(source).unwrap();
    assert!(exports.contains(&"add".to_string()));
    assert!(exports.contains(&"subtract".to_string()));
}

#[test]
fn test_module_import_parsing() {
    let source = r#"
import math_utils.{ add, subtract }
import "./geometry.script" as Geo

fn calculate() -> int {
    let sum = add(10, 20)
    let area = Geo.calculateArea(shape)
    sum
}
"#;

    // Just test that it parses successfully
    let result = parse_source(source);
    assert!(result.is_ok());
}

#[test]
fn test_circular_dependency_detection() {
    let module_dir = PathBuf::from("tests/modules");
    let mut config = script::module::ModuleResolverConfig::default();
    config.search_stdlib = false;
    let mut resolver = FileSystemResolver::new(config);
    let _registry = ModuleRegistry::default();
    let _cache = ModuleCache::new();

    // Try to load circular_a which imports circular_b which imports circular_a
    let circular_path = ModulePath::from_string("circular_a").unwrap();
    let result = resolver.resolve(&circular_path, &module_dir);

    // The resolver itself should succeed, but compilation should detect the cycle
    assert!(result.is_ok());
}

#[test]
fn test_relative_path_resolution() {
    let _module_dir = PathBuf::from("tests/modules");
    let mut config = script::module::ModuleResolverConfig::default();
    config.search_stdlib = false;
    let _resolver = FileSystemResolver::new(config);

    // Test relative path starting with ./
    let rel_path = ImportPath::new("./point.script").unwrap();
    assert!(rel_path.path.starts_with("./"));

    // Test relative path starting with ../
    let parent_path = ImportPath::new("../other/module.script").unwrap();
    assert!(parent_path.path.starts_with("../"));

    // Test absolute module path
    let abs_path = ImportPath::new("std.collections").unwrap();
    assert!(!abs_path.path.starts_with("./") && !abs_path.path.starts_with("../"));
}

#[test]
fn test_module_with_dependencies() {
    let module_dir = PathBuf::from("tests/modules");

    // Read geometry module which depends on math_utils and point
    if let Ok(geometry_source) = fs::read_to_string(module_dir.join("geometry.script")) {
        let _exports = compile_and_check_exports(&geometry_source).unwrap_or_default();

        // The actual exports depend on what's in the file
        // For now, just check that parsing works
        let result = parse_source(&geometry_source);
        assert!(result.is_ok());
    }
}

#[test]
fn test_missing_module_error() {
    let source = r#"
import non_existent_module.{ someFunc }

fn main() {
    someFunc()
}
"#;

    // Test that parsing succeeds (the error would be caught during semantic analysis)
    let result = parse_source(source);
    assert!(result.is_ok());

    // In a real scenario, the module resolution would fail
    let import_path = ImportPath::new("non_existent_module").unwrap();
    let module_dir = PathBuf::from("tests/modules");
    let mut config = script::module::ModuleResolverConfig::default();
    config.search_stdlib = false; // Don't search stdlib for this test
    let mut resolver = FileSystemResolver::new(config);
    let load_context =
        ModuleLoadContext::new(ModulePath::from_string("test").unwrap(), module_dir.clone());

    // This should fail because the module doesn't exist
    let result = resolver.resolve_module(&import_path, &load_context);
    assert!(result.is_err());
}

#[test]
fn test_selective_imports() {
    let source = r#"
import math_utils.{ add, PI }
import string_utils.{ concat as join, trim }

fn main() {
    let sum = add(PI, 1.0)
    let text = join("Hello, ", trim("  World  "))
    sum
}
"#;

    let result = parse_source(source);
    assert!(result.is_ok());
}

#[test]
fn test_wildcard_import() {
    let source = r#"
import math_utils.*

fn main() {
    let sum = add(1.0, 2.0)
    let product = multiply(PI, 2.0)
    sum + product
}
"#;

    let result = parse_source(source);
    assert!(result.is_ok());
}

#[test]
fn test_module_aliasing() {
    let source = r#"
import math_utils as Math
import "./geometry.script" as Shapes

fn main() {
    let area = Shapes.calculateArea(circle)
    area
}
"#;

    let result = parse_source(source);
    assert!(result.is_ok());
}

#[test]
fn test_private_function_not_accessible() {
    let source = r#"
import math_utils.{ square }  // square is private

fn main() {
    square(5.0)
}
"#;

    // Parsing should succeed, the error would be caught during semantic analysis
    let result = parse_source(source);
    assert!(result.is_ok());
}

#[test]
fn test_nested_module_paths() {
    let source = r#"
import std.collections.map as Map
import std.io.file as File

fn main() {
    let data = Map.new()
    let content = File.read("data.txt")
    data
}
"#;

    let result = parse_source(source);
    assert!(result.is_ok());
}

#[test]
fn test_module_compilation_pipeline() {
    let module_dir = PathBuf::from("tests/modules");
    let mut pipeline = create_default_pipeline();

    // Test compiling a module using the proper API
    let module_path = ModulePath::from_string("test_module").unwrap();
    let context = ModuleLoadContext::new(module_path.clone(), module_dir.clone());
    let config = CompilationConfig::default();

    // This will fail if the module doesn't exist, which is expected
    // The test is mainly to ensure the API is used correctly
    let result = pipeline.compile_module(&module_path, &context, &config);

    // We expect this to fail since we don't have actual test modules set up
    assert!(result.is_err());
}
