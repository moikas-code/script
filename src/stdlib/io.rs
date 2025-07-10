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
use crate::stdlib::error::{io_error_from_std, IoError, ScriptError};
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
/// Returns a Result<string, IoError> in Script
pub fn read_line() -> Result<String, IoError> {
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
        Err(e) => Err(io_error_from_std(e)),
    }
}

/// Read the entire contents of a file
/// Returns a Result<string, IoError> in Script
pub fn read_file(path: &str) -> Result<String, IoError> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(contents),
        Err(e) => {
            let mut io_err = io_error_from_std(e);
            io_err.message = format!("Failed to read file '{}': {}", path, io_err.message);
            Err(io_err)
        }
    }
}

/// Write a string to a file
/// Returns a Result<unit, IoError> in Script
pub fn write_file(path: &str, contents: &str) -> Result<(), IoError> {
    // Ensure parent directory exists
    if let Some(parent) = Path::new(path).parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!(
                    "Failed to create directory '{}': {}",
                    parent.display(),
                    io_err.message
                );
                return Err(io_err);
            }
        }
    }

    match fs::write(path, contents) {
        Ok(()) => Ok(()),
        Err(e) => {
            let mut io_err = io_error_from_std(e);
            io_err.message = format!("Failed to write file '{}': {}", path, io_err.message);
            Err(io_err)
        }
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
        Err(io_err) => {
            // Convert IoError to ScriptValue
            let error_val = io_err.to_script_value();
            let result = ScriptResult::err(error_val);
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
            Err(io_err) => {
                // Convert IoError to ScriptValue
                let error_val = io_err.to_script_value();
                let result = ScriptResult::err(error_val);
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
                Err(io_err) => {
                    // Convert IoError to ScriptValue
                    let error_val = io_err.to_script_value();
                    let result = ScriptResult::err(error_val);
                    Ok(ScriptValue::Result(ScriptRc::new(result)))
                }
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "write_file expects two string arguments (path, contents)".to_string(),
        )),
    }
}

/// Check if a file exists
/// Returns a Result<bool, IoError> in Script
pub fn file_exists(path: &str) -> Result<bool, IoError> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.is_file()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(io_error_from_std(e)),
    }
}

/// Check if a directory exists
/// Returns a Result<bool, IoError> in Script
pub fn dir_exists(path: &str) -> Result<bool, IoError> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.is_dir()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(io_error_from_std(e)),
    }
}

/// Create a directory (including parent directories)
/// Returns a Result<unit, IoError> in Script
pub fn create_dir(path: &str) -> Result<(), IoError> {
    match fs::create_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let mut io_err = io_error_from_std(e);
            io_err.message = format!("Failed to create directory '{}': {}", path, io_err.message);
            Err(io_err)
        }
    }
}

/// Delete a file
/// Returns a Result<unit, IoError> in Script
pub fn delete_file(path: &str) -> Result<(), IoError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let mut io_err = io_error_from_std(e);
            io_err.message = format!("Failed to delete file '{}': {}", path, io_err.message);
            Err(io_err)
        }
    }
}

/// Copy a file
/// Returns a Result<unit, IoError> in Script
pub fn copy_file(from: &str, to: &str) -> Result<(), IoError> {
    match fs::copy(from, to) {
        Ok(_) => Ok(()),
        Err(e) => {
            let mut io_err = io_error_from_std(e);
            io_err.message = format!("Failed to copy '{}' to '{}': {}", from, to, io_err.message);
            Err(io_err)
        }
    }
}

/// Append content to a file
/// Returns a Result<unit, IoError> in Script
pub fn append_file(path: &str, contents: &str) -> Result<(), IoError> {
    use std::fs::OpenOptions;
    use std::io::Write;

    // Ensure parent directory exists
    if let Some(parent) = Path::new(path).parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!(
                    "Failed to create directory '{}': {}",
                    parent.display(),
                    io_err.message
                );
                return Err(io_err);
            }
        }
    }

    match OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut file) => match file.write_all(contents.as_bytes()) {
            Ok(()) => Ok(()),
            Err(e) => {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!("Failed to append to file '{}': {}", path, io_err.message);
                Err(io_err)
            }
        },
        Err(e) => {
            let mut io_err = io_error_from_std(e);
            io_err.message = format!(
                "Failed to open file '{}' for appending: {}",
                path, io_err.message
            );
            Err(io_err)
        }
    }
}

/// Delete a directory and all its contents
/// Returns a Result<unit, IoError> in Script
pub fn delete_dir(path: &str) -> Result<(), IoError> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let mut io_err = io_error_from_std(e);
            io_err.message = format!("Failed to delete directory '{}': {}", path, io_err.message);
            Err(io_err)
        }
    }
}

/// List files and directories in a directory
/// Returns a Result<Vec<string>, IoError> in Script
pub fn list_dir(path: &str) -> Result<Vec<String>, IoError> {
    match fs::read_dir(path) {
        Ok(entries) => {
            let mut files = Vec::new();
            for entry in entries {
                match entry {
                    Ok(dir_entry) => {
                        if let Some(name) = dir_entry.file_name().to_str() {
                            files.push(name.to_string());
                        }
                    }
                    Err(e) => {
                        let mut io_err = io_error_from_std(e);
                        io_err.message = format!(
                            "Failed to read directory entry in '{}': {}",
                            path, io_err.message
                        );
                        return Err(io_err);
                    }
                }
            }
            Ok(files)
        }
        Err(e) => {
            let mut io_err = io_error_from_std(e);
            io_err.message = format!("Failed to read directory '{}': {}", path, io_err.message);
            Err(io_err)
        }
    }
}

/// Get file metadata (size, modification time, etc.)
/// Returns a Result<Object, IoError> in Script
pub fn file_metadata(
    path: &str,
) -> Result<std::collections::HashMap<String, ScriptValue>, IoError> {
    match fs::metadata(path) {
        Ok(metadata) => {
            let mut info = std::collections::HashMap::new();

            info.insert("size".to_string(), ScriptValue::I32(metadata.len() as i32));
            info.insert("is_file".to_string(), ScriptValue::Bool(metadata.is_file()));
            info.insert("is_dir".to_string(), ScriptValue::Bool(metadata.is_dir()));
            info.insert(
                "read_only".to_string(),
                ScriptValue::Bool(metadata.permissions().readonly()),
            );

            if let Ok(modified) = metadata.modified() {
                if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                    info.insert(
                        "modified_time".to_string(),
                        ScriptValue::I32(duration.as_secs() as i32),
                    );
                }
            }

            Ok(info)
        }
        Err(e) => {
            let mut io_err = io_error_from_std(e);
            io_err.message = format!("Failed to get metadata for '{}': {}", path, io_err.message);
            Err(io_err)
        }
    }
}

// Implementation functions for new I/O operations

/// Implementation of file_exists for Script
pub(crate) fn file_exists_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "file_exists expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(path) => match file_exists(&path.as_str()) {
            Ok(exists) => {
                let result = ScriptResult::ok(ScriptValue::Bool(exists));
                Ok(ScriptValue::Result(ScriptRc::new(result)))
            }
            Err(io_err) => {
                let error_val = io_err.to_script_value();
                let result = ScriptResult::err(error_val);
                Ok(ScriptValue::Result(ScriptRc::new(result)))
            }
        },
        _ => Err(RuntimeError::InvalidOperation(
            "file_exists expects a string argument".to_string(),
        )),
    }
}

/// Implementation of dir_exists for Script
pub(crate) fn dir_exists_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "dir_exists expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(path) => match dir_exists(&path.as_str()) {
            Ok(exists) => {
                let result = ScriptResult::ok(ScriptValue::Bool(exists));
                Ok(ScriptValue::Result(ScriptRc::new(result)))
            }
            Err(io_err) => {
                let error_val = io_err.to_script_value();
                let result = ScriptResult::err(error_val);
                Ok(ScriptValue::Result(ScriptRc::new(result)))
            }
        },
        _ => Err(RuntimeError::InvalidOperation(
            "dir_exists expects a string argument".to_string(),
        )),
    }
}

/// Implementation of create_dir for Script
pub(crate) fn create_dir_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "create_dir expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(path) => match create_dir(&path.as_str()) {
            Ok(()) => {
                let result = ScriptResult::ok(ScriptValue::Unit);
                Ok(ScriptValue::Result(ScriptRc::new(result)))
            }
            Err(io_err) => {
                let error_val = io_err.to_script_value();
                let result = ScriptResult::err(error_val);
                Ok(ScriptValue::Result(ScriptRc::new(result)))
            }
        },
        _ => Err(RuntimeError::InvalidOperation(
            "create_dir expects a string argument".to_string(),
        )),
    }
}

/// Implementation of delete_file for Script
pub(crate) fn delete_file_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "delete_file expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(path) => match delete_file(&path.as_str()) {
            Ok(()) => {
                let result = ScriptResult::ok(ScriptValue::Unit);
                Ok(ScriptValue::Result(ScriptRc::new(result)))
            }
            Err(io_err) => {
                let error_val = io_err.to_script_value();
                let result = ScriptResult::err(error_val);
                Ok(ScriptValue::Result(ScriptRc::new(result)))
            }
        },
        _ => Err(RuntimeError::InvalidOperation(
            "delete_file expects a string argument".to_string(),
        )),
    }
}

/// Implementation of copy_file for Script
pub(crate) fn copy_file_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "copy_file expects 2 arguments, got {}",
            args.len()
        )));
    }

    match (&args[0], &args[1]) {
        (ScriptValue::String(from), ScriptValue::String(to)) => {
            match copy_file(&from.as_str(), &to.as_str()) {
                Ok(()) => {
                    let result = ScriptResult::ok(ScriptValue::Unit);
                    Ok(ScriptValue::Result(ScriptRc::new(result)))
                }
                Err(io_err) => {
                    let error_val = io_err.to_script_value();
                    let result = ScriptResult::err(error_val);
                    Ok(ScriptValue::Result(ScriptRc::new(result)))
                }
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "copy_file expects two string arguments (from, to)".to_string(),
        )),
    }
}

/// Implementation of append_file for Script
pub(crate) fn append_file_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "append_file expects 2 arguments, got {}",
            args.len()
        )));
    }

    match (&args[0], &args[1]) {
        (ScriptValue::String(path), ScriptValue::String(contents)) => {
            match append_file(&path.as_str(), &contents.as_str()) {
                Ok(()) => {
                    let result = ScriptResult::ok(ScriptValue::Unit);
                    Ok(ScriptValue::Result(ScriptRc::new(result)))
                }
                Err(io_err) => {
                    let error_val = io_err.to_script_value();
                    let result = ScriptResult::err(error_val);
                    Ok(ScriptValue::Result(ScriptRc::new(result)))
                }
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "append_file expects two string arguments (path, contents)".to_string(),
        )),
    }
}

/// Implementation of delete_dir for Script
pub(crate) fn delete_dir_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "delete_dir expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(path) => match delete_dir(&path.as_str()) {
            Ok(()) => {
                let result = ScriptResult::ok(ScriptValue::Unit);
                Ok(ScriptValue::Result(ScriptRc::new(result)))
            }
            Err(io_err) => {
                let error_val = io_err.to_script_value();
                let result = ScriptResult::err(error_val);
                Ok(ScriptValue::Result(ScriptRc::new(result)))
            }
        },
        _ => Err(RuntimeError::InvalidOperation(
            "delete_dir expects a string argument".to_string(),
        )),
    }
}

/// Implementation of list_dir for Script
pub(crate) fn list_dir_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "list_dir expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(path) => match list_dir(&path.as_str()) {
            Ok(files) => {
                let script_vec = crate::stdlib::collections::ScriptVec::new();
                for file in files {
                    script_vec
                        .push(ScriptValue::String(ScriptRc::new(ScriptString::from_str(
                            &file,
                        ))))
                        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;
                }
                let result = ScriptResult::ok(ScriptValue::Array(ScriptRc::new(script_vec)));
                Ok(ScriptValue::Result(ScriptRc::new(result)))
            }
            Err(io_err) => {
                let error_val = io_err.to_script_value();
                let result = ScriptResult::err(error_val);
                Ok(ScriptValue::Result(ScriptRc::new(result)))
            }
        },
        _ => Err(RuntimeError::InvalidOperation(
            "list_dir expects a string argument".to_string(),
        )),
    }
}

/// Implementation of file_metadata for Script
pub(crate) fn file_metadata_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "file_metadata expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(path) => match file_metadata(&path.as_str()) {
            Ok(metadata) => {
                let result = ScriptResult::ok(ScriptValue::Object(ScriptRc::new(metadata)));
                Ok(ScriptValue::Result(ScriptRc::new(result)))
            }
            Err(io_err) => {
                let error_val = io_err.to_script_value();
                let result = ScriptResult::err(error_val);
                Ok(ScriptValue::Result(ScriptRc::new(result)))
            }
        },
        _ => Err(RuntimeError::InvalidOperation(
            "file_metadata expects a string argument".to_string(),
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

    #[test]
    fn test_append_file() {
        let test_dir = PathBuf::from("target/test_append");
        let test_file = test_dir.join("append_test.txt");

        // Ensure test directory exists
        fs::create_dir_all(&test_dir).unwrap();

        // Test append_file with new file
        let initial_content = "Hello, ";
        assert!(append_file(test_file.to_str().unwrap(), initial_content).is_ok());

        // Test append_file with existing file
        let additional_content = "World!";
        assert!(append_file(test_file.to_str().unwrap(), additional_content).is_ok());

        // Verify content was appended
        let result = read_file(test_file.to_str().unwrap()).unwrap();
        assert_eq!(result, "Hello, World!");

        // Cleanup
        let _ = fs::remove_file(&test_file);
        let _ = fs::remove_dir(&test_dir);
    }

    #[test]
    fn test_list_dir() {
        let test_dir = PathBuf::from("target/test_list");
        let test_file1 = test_dir.join("file1.txt");
        let test_file2 = test_dir.join("file2.txt");

        // Create test directory and files
        fs::create_dir_all(&test_dir).unwrap();
        fs::write(&test_file1, "content1").unwrap();
        fs::write(&test_file2, "content2").unwrap();

        // Test list_dir
        let result = list_dir(test_dir.to_str().unwrap()).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"file1.txt".to_string()));
        assert!(result.contains(&"file2.txt".to_string()));

        // Cleanup
        let _ = fs::remove_file(&test_file1);
        let _ = fs::remove_file(&test_file2);
        let _ = fs::remove_dir(&test_dir);
    }

    #[test]
    fn test_file_metadata() {
        let test_dir = PathBuf::from("target/test_metadata");
        let test_file = test_dir.join("metadata_test.txt");

        // Create test directory and file
        fs::create_dir_all(&test_dir).unwrap();
        let content = "test content";
        fs::write(&test_file, content).unwrap();

        // Test file_metadata
        let result = file_metadata(test_file.to_str().unwrap()).unwrap();
        assert_eq!(
            result.get("size"),
            Some(&ScriptValue::I32(content.len() as i32))
        );
        assert_eq!(result.get("is_file"), Some(&ScriptValue::Bool(true)));
        assert_eq!(result.get("is_dir"), Some(&ScriptValue::Bool(false)));
        assert!(result.contains_key("modified_time"));

        // Cleanup
        let _ = fs::remove_file(&test_file);
        let _ = fs::remove_dir(&test_dir);
    }

    #[test]
    fn test_delete_dir() {
        let test_dir = PathBuf::from("target/test_delete");
        let test_file = test_dir.join("file.txt");

        // Create test directory and file
        fs::create_dir_all(&test_dir).unwrap();
        fs::write(&test_file, "test").unwrap();

        // Test delete_dir
        assert!(delete_dir(test_dir.to_str().unwrap()).is_ok());
        assert!(!test_dir.exists());
    }
}
