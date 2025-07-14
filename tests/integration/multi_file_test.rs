//! Integration test for multi-file module compilation
//!
//! This test verifies that the module system can properly compile
//! projects with multiple .script files and handle imports correctly.

use script::compilation::context::CompilationContext;
use script::error::ErrorKind;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_simple_two_file_compilation() {
    // Create temporary directory for test files
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Create main.script file
    let main_content = r#"
import { add } from "./math.script"

fn main() {
    let result = add(2, 3);
    print(result);
}
"#;

    // Create math.script file
    let math_content = r#"
export fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;

    // Write files
    fs::write(temp_path.join("main.script"), main_content).expect("Failed to write main.script");
    fs::write(temp_path.join("math.script"), math_content).expect("Failed to write math.script");

    // Create compilation context and compile the directory
    let mut context = CompilationContext::new();
    let result = context.compile_directory(temp_path);

    // Should succeed
    match result {
        Ok(_ir_module) => {
            // Compilation succeeded - this is what we want
            println!("Multi-file compilation succeeded!");
        }
        Err(error) => {
            panic!("Multi-file compilation failed: {:?}", error);
        }
    }
}

#[test]
fn test_circular_dependency_detection() {
    // Create temporary directory for test files
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Create a.script with circular dependency
    let a_content = r#"
import { func_b } from "./b.script"

export fn func_a() {
    func_b();
}
"#;

    // Create b.script with circular dependency
    let b_content = r#"
import { func_a } from "./a.script"

export fn func_b() {
    func_a();
}
"#;

    // Write files
    fs::write(temp_path.join("a.script"), a_content).expect("Failed to write a.script");
    fs::write(temp_path.join("b.script"), b_content).expect("Failed to write b.script");

    // Create compilation context and compile the directory
    let mut context = CompilationContext::new();
    let result = context.compile_directory(temp_path);

    // Should fail with circular dependency error
    match result {
        Ok(_) => {
            panic!("Expected circular dependency error, but compilation succeeded");
        }
        Err(error) => {
            let error_msg = format!("{:?}", error);
            assert!(
                error_msg.contains("Circular dependency"),
                "Expected circular dependency error, got: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_missing_import_error() {
    // Create temporary directory for test files
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Create main.script that imports from non-existent file
    let main_content = r#"
import { missing_func } from "./nonexistent.script"

fn main() {
    missing_func();
}
"#;

    // Write file
    fs::write(temp_path.join("main.script"), main_content).expect("Failed to write main.script");

    // Create compilation context and compile the directory
    let mut context = CompilationContext::new();
    let result = context.compile_directory(temp_path);

    // Should fail with file not found or import error
    match result {
        Ok(_) => {
            panic!("Expected import error, but compilation succeeded");
        }
        Err(error) => {
            // Should get some kind of import/file error
            println!("Got expected error: {:?}", error);
            // Accept various error types related to missing modules
            let error_msg = format!("{:?}", error);
            assert!(
                error_msg.contains("not found")
                    || error_msg.contains("missing")
                    || error_msg.contains("import")
                    || error_msg.contains("module"),
                "Expected import-related error, got: {}",
                error_msg
            );
        }
    }
}
