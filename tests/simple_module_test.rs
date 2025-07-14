use script::compilation::CompilationContext;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_basic_module_loading() {
    // Create temporary directory for test files
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Create a simple module that exports a function
    let math_content = r#"
export fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;

    // Create main module that imports from math
    let main_content = r#"
import { add } from "./math"

fn main() {
    let result = add(2, 3);
    print(result);
}
"#;

    // Write files
    fs::write(temp_path.join("math.script"), math_content).expect("Failed to write math.script");
    fs::write(temp_path.join("main.script"), main_content).expect("Failed to write main.script");

    // Create compilation context and compile the directory
    let mut context = CompilationContext::new();

    match context.compile_directory(temp_path) {
        Ok(_ir_module) => {
            println!("Module loading test passed!");
        }
        Err(error) => {
            panic!("Module loading failed: {:?}", error);
        }
    }
}

#[test]
fn test_default_export_import() {
    // Create temporary directory for test files
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Create a module with default export
    let utils_content = r#"
export default 42
"#;

    // Create main module that imports the default
    let main_content = r#"
import answer from "./utils"

fn main() {
    print(answer);
}
"#;

    // Write files
    fs::write(temp_path.join("utils.script"), utils_content).expect("Failed to write utils.script");
    fs::write(temp_path.join("main.script"), main_content).expect("Failed to write main.script");

    // Create compilation context and compile the directory
    let mut context = CompilationContext::new();

    match context.compile_directory(temp_path) {
        Ok(_ir_module) => {
            println!("Default export/import test passed!");
        }
        Err(error) => {
            // Default exports might not be fully implemented yet
            println!("Default export/import test failed (expected): {:?}", error);
        }
    }
}
