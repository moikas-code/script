/// Templates for package initialization

pub const GITIGNORE_TEMPLATE: &str = r#"# Build artifacts
/target/
/build/
*.script.out

# Dependencies
/script_modules/

# IDE
.vscode/
.idea/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db

# Logs
*.log

# Environment
.env
.env.local

# Cache
.manuscript-cache/
"#;

pub const LIBRARY_TEMPLATE: &str = r#"//! A new Script library
//!
//! This is the main library file for your Script package.

/// A simple greeting function
pub fn greet(name: String) -> String {
    return "Hello, " + name + "!"
}

/// Add two numbers together
pub fn add(a: i64, b: i64) -> i64 {
    return a + b
}

#[cfg(test)]
mod tests {
    use super::*
    
    #[test]
    fn test_greet() {
        assert_eq(greet("World"), "Hello, World!")
    }
    
    #[test]
    fn test_add() {
        assert_eq(add(2, 3), 5)
        assert_eq(add(-1, 1), 0)
    }
}
"#;

pub const LIBRARY_TEST_TEMPLATE: &str = r#"//! Integration tests for the library

use my_package::*

#[test]
fn test_integration() {
    // Test that exported functions work correctly
    let result = greet("Script")
    assert_eq(result, "Hello, Script!")
}
"#;

pub fn generate_main_file(package_name: &str) -> String {
    format!(
        r#"//! {} - A Script application
//!
//! This is the main entry point for your Script application.

use std::{{env, io}}

fn main() {{
    println("Welcome to {}")
    
    // Get command line arguments
    let args = env::args()
    
    if args.len() > 1 {{
        println("Arguments received:")
        for i in 1..args.len() {{
            println("  " + str(i) + ": " + args[i])
        }}
    }} else {{
        println("No arguments provided.")
        println("Try: manuscript run -- arg1 arg2")
    }}
}}
"#,
        package_name, package_name
    )
}

pub fn generate_readme(name: &str, description: Option<&str>) -> String {
    let desc = description.unwrap_or("A new Script package");

    format!(
        r#"# {}

{}

## Installation

```bash
manuscript install
```

## Usage

```script
use {}::*

// Your code here
```

## Development

```bash
# Run tests
manuscript test

# Build the package
manuscript build

# Run the application (if binary)
manuscript run
```

## License

This project is licensed under the MIT License.
"#,
        name, desc, name
    )
}

pub const EXAMPLE_MANIFEST: &str = r#"[package]
name = "example-package"
version = "0.1.0"
authors = ["Your Name <email@example.com>"]
description = "An example Script package"
license = "MIT"
edition = "2024"

[dependencies]
# Add your dependencies here
# example = "1.0"

[dev-dependencies]
# Add development dependencies here

[lib]
path = "src/lib.script"

[[bin]]
name = "example"
path = "src/main.script"

[scripts]
test = "manuscript test"
build = "manuscript build --release"
fmt = "manuscript fmt"
"#;
