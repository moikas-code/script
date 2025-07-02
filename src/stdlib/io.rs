//! I/O operations for the Script programming language
//!
//! This module provides functions for input/output operations including:
//! - Console output (print, println, eprintln)
//! - Console input (read_line)
//! - File I/O (read_file, write_file)
//!
//! All functions are designed to be called from Script code and handle
//! errors using the Script Result type.

use crate::runtime::{RuntimeError, ScriptRc};
use crate::stdlib::{ScriptResult, ScriptString, ScriptValue};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;

/// Print a string to stdout without a newline
pub fn print(s: &str) {
    print!("{}", s);
    // Flush to ensure output appears immediately
    let _ = io::stdout().flush();
}

/// Print a string to stdout with a newline
pub fn println(s: &str) {
    println!("{}", s);
}

/// Print a string to stderr with a newline
pub fn eprintln(s: &str) {
    eprintln!("{}", s);
}

/// Read a line from stdin
/// Returns a Result<string, string> in Script
pub fn read_line() -> Result<String, String> {
    let stdin = io::stdin();
    let mut line = String::new();

    match stdin.lock().read_line(&mut line) {
        Ok(_) => {
            // Remove trailing newline if present
            if line.ends_with('\n') {
                line.pop();
                if line.ends_with('\r') {
                    line.pop();
                }
            }
            Ok(line)
        }
        Err(e) => Err(format!("Failed to read line: {}", e)),
    }
}

/// Read the entire contents of a file
/// Returns a Result<string, string> in Script
pub fn read_file(path: &str) -> Result<String, String> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(contents),
        Err(e) => Err(format!("Failed to read file '{}': {}", path, e)),
    }
}

/// Write a string to a file
/// Returns a Result<unit, string> in Script
pub fn write_file(path: &str, contents: &str) -> Result<(), String> {
    // Ensure parent directory exists
    if let Some(parent) = Path::new(path).parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Err(format!(
                    "Failed to create directory '{}': {}",
                    parent.display(),
                    e
                ));
            }
        }
    }

    match fs::write(path, contents) {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("Failed to write file '{}': {}", path, e)),
    }
}

// Implementation functions for the stdlib registry

/// Implementation of print for Script
pub(crate) fn print_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "print expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(s) => {
            print(&s.as_str());
            Ok(ScriptValue::Unit)
        }
        _ => Err(RuntimeError::InvalidOperation(
            "print expects a string argument".to_string(),
        )),
    }
}

/// Implementation of println for Script
pub(crate) fn println_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "println expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(s) => {
            println(&s.as_str());
            Ok(ScriptValue::Unit)
        }
        _ => Err(RuntimeError::InvalidOperation(
            "println expects a string argument".to_string(),
        )),
    }
}

/// Implementation of eprintln for Script
pub(crate) fn eprintln_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "eprintln expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(s) => {
            eprintln(&s.as_str());
            Ok(ScriptValue::Unit)
        }
        _ => Err(RuntimeError::InvalidOperation(
            "eprintln expects a string argument".to_string(),
        )),
    }
}

/// Implementation of read_line for Script
pub(crate) fn read_line_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if !args.is_empty() {
        return Err(RuntimeError::InvalidOperation(format!(
            "read_line expects 0 arguments, got {}",
            args.len()
        )));
    }

    match read_line() {
        Ok(line) => {
            let script_str = ScriptString::new(line);
            let result = ScriptResult::ok(ScriptValue::String(ScriptRc::new(script_str)));
            Ok(ScriptValue::Result(ScriptRc::new(result)))
        }
        Err(err) => {
            let script_str = ScriptString::new(err);
            let result = ScriptResult::err(ScriptValue::String(ScriptRc::new(script_str)));
            Ok(ScriptValue::Result(ScriptRc::new(result)))
        }
    }
}

/// Implementation of read_file for Script
pub(crate) fn read_file_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "read_file expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(path) => match read_file(&path.as_str()) {
            Ok(contents) => {
                let script_str = ScriptString::new(contents);
                let result = ScriptResult::ok(ScriptValue::String(ScriptRc::new(script_str)));
                Ok(ScriptValue::Result(ScriptRc::new(result)))
            }
            Err(err) => {
                let script_str = ScriptString::new(err);
                let result = ScriptResult::err(ScriptValue::String(ScriptRc::new(script_str)));
                Ok(ScriptValue::Result(ScriptRc::new(result)))
            }
        },
        _ => Err(RuntimeError::InvalidOperation(
            "read_file expects a string argument".to_string(),
        )),
    }
}

/// Implementation of write_file for Script
pub(crate) fn write_file_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "write_file expects 2 arguments, got {}",
            args.len()
        )));
    }

    match (&args[0], &args[1]) {
        (ScriptValue::String(path), ScriptValue::String(contents)) => {
            match write_file(&path.as_str(), &contents.as_str()) {
                Ok(()) => {
                    let result = ScriptResult::ok(ScriptValue::Unit);
                    Ok(ScriptValue::Result(ScriptRc::new(result)))
                }
                Err(err) => {
                    let script_str = ScriptString::new(err);
                    let result = ScriptResult::err(ScriptValue::String(ScriptRc::new(script_str)));
                    Ok(ScriptValue::Result(ScriptRc::new(result)))
                }
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "write_file expects two string arguments (path, contents)".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_print_functions() {
        // These functions write to stdout/stderr, so we just test they don't panic
        print("Hello");
        println(" World");
        eprintln("Error message");
    }

    #[test]
    fn test_file_operations() {
        let test_dir = PathBuf::from("target/test_io");
        let test_file = test_dir.join("test.txt");

        // Ensure test directory exists
        fs::create_dir_all(&test_dir).unwrap();

        // Test write_file
        let content = "Hello, Script!";
        assert!(write_file(test_file.to_str().unwrap(), content).is_ok());

        // Test read_file
        let read_result = read_file(test_file.to_str().unwrap());
        assert!(read_result.is_ok());
        assert_eq!(read_result.unwrap(), content);

        // Test read non-existent file
        let bad_result = read_file("non_existent_file.txt");
        assert!(bad_result.is_err());

        // Cleanup
        let _ = fs::remove_file(&test_file);
        let _ = fs::remove_dir(&test_dir);
    }

    #[test]
    fn test_script_value_implementations() {
        // Test print_impl
        let str_val = ScriptValue::String(ScriptRc::new(ScriptString::new("test".to_string())));
        let result = print_impl(&[str_val.clone()]);
        assert!(result.is_ok());
        assert!(result.unwrap().is_unit());

        // Test println_impl
        let result = println_impl(&[str_val.clone()]);
        assert!(result.is_ok());
        assert!(result.unwrap().is_unit());

        // Test eprintln_impl
        let result = eprintln_impl(&[str_val]);
        assert!(result.is_ok());
        assert!(result.unwrap().is_unit());

        // Test wrong argument count
        let result = print_impl(&[]);
        assert!(result.is_err());

        // Test wrong argument type
        let int_val = ScriptValue::I32(42);
        let result = print_impl(&[int_val]);
        assert!(result.is_err());
    }
}
