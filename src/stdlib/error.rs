//! Standard Error trait and error types for the Script programming language
//!
//! This module provides a unified error handling system that allows custom error types
//! to implement the Script Error trait for consistent error reporting and handling.

use crate::runtime::{RuntimeError, ScriptRc};
use crate::stdlib::ScriptValue;
use std::fmt;

/// Standard Error trait for Script custom error types
pub trait ScriptError: fmt::Debug + fmt::Display + Send + Sync + 'static {
    /// Get the error message as a string
    fn message(&self) -> String {
        self.to_string()
    }

    /// Get the error code if available
    fn code(&self) -> Option<i32> {
        None
    }

    /// Get the error kind/category
    fn kind(&self) -> String {
        "GenericError".to_string()
    }

    /// Get the underlying cause of this error
    fn cause(&self) -> Option<&dyn ScriptError> {
        None
    }

    /// Get the stack trace if available
    fn stack_trace(&self) -> Option<String> {
        None
    }

    /// Check if this error is recoverable
    fn is_recoverable(&self) -> bool {
        true
    }

    /// Convert to ScriptValue for Script language consumption
    fn to_script_value(&self) -> ScriptValue {
        let error_map = std::collections::HashMap::from([
            (
                "message".to_string(),
                ScriptValue::String(ScriptRc::new(crate::stdlib::ScriptString::new(
                    self.message(),
                ))),
            ),
            (
                "kind".to_string(),
                ScriptValue::String(ScriptRc::new(crate::stdlib::ScriptString::new(self.kind()))),
            ),
            (
                "recoverable".to_string(),
                ScriptValue::Bool(self.is_recoverable()),
            ),
        ]);

        // For now, return a simple string representation
        // In a full implementation, this would return a proper error object
        ScriptValue::String(ScriptRc::new(crate::stdlib::ScriptString::new(
            self.message(),
        )))
    }
}

/// Standard error types provided by the Script standard library

/// I/O Error for file operations, network operations, etc.
#[derive(Debug, Clone)]
pub struct IoError {
    pub message: String,
    pub kind: IoErrorKind,
    pub code: Option<i32>,
}

#[derive(Debug, Clone)]
pub enum IoErrorKind {
    NotFound,
    PermissionDenied,
    ConnectionRefused,
    Interrupted,
    InvalidInput,
    InvalidData,
    TimedOut,
    WriteZero,
    UnexpectedEof,
    Other,
}

impl fmt::Display for IoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IO Error ({}): {}", self.kind, self.message)
    }
}

impl fmt::Display for IoErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind_str = match self {
            IoErrorKind::NotFound => "NotFound",
            IoErrorKind::PermissionDenied => "PermissionDenied",
            IoErrorKind::ConnectionRefused => "ConnectionRefused",
            IoErrorKind::Interrupted => "Interrupted",
            IoErrorKind::InvalidInput => "InvalidInput",
            IoErrorKind::InvalidData => "InvalidData",
            IoErrorKind::TimedOut => "TimedOut",
            IoErrorKind::WriteZero => "WriteZero",
            IoErrorKind::UnexpectedEof => "UnexpectedEof",
            IoErrorKind::Other => "Other",
        };
        write!(f, "{}", kind_str)
    }
}

impl ScriptError for IoError {
    fn message(&self) -> String {
        self.message.clone()
    }

    fn code(&self) -> Option<i32> {
        self.code
    }

    fn kind(&self) -> String {
        format!("IoError::{}", self.kind)
    }
}

impl IoError {
    pub fn new(kind: IoErrorKind, message: String) -> Self {
        IoError {
            message,
            kind,
            code: None,
        }
    }

    pub fn with_code(kind: IoErrorKind, message: String, code: i32) -> Self {
        IoError {
            message,
            kind,
            code: Some(code),
        }
    }

    pub fn not_found(message: String) -> Self {
        IoError::new(IoErrorKind::NotFound, message)
    }

    pub fn permission_denied(message: String) -> Self {
        IoError::new(IoErrorKind::PermissionDenied, message)
    }

    pub fn invalid_input(message: String) -> Self {
        IoError::new(IoErrorKind::InvalidInput, message)
    }
}

/// Validation Error for input validation, type checking, etc.
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub message: String,
    pub field: Option<String>,
    pub value: Option<String>,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.field, &self.value) {
            (Some(field), Some(value)) => {
                write!(
                    f, "Validation Error for field '{}' with value '{}': {}", field, value, self.message)
            }
            (Some(field), None) => {
                write!(
                    f, "Validation Error for field '{}': {}", field, self.message)
            }
            _ => {
                write!(f, "Validation Error: {}", self.message)
            }
        }
    }
}

impl ScriptError for ValidationError {
    fn message(&self) -> String {
        self.message.clone()
    }

    fn kind(&self) -> String {
        "ValidationError".to_string()
    }

    fn is_recoverable(&self) -> bool {
        true // Validation errors are typically recoverable
    }
}

impl ValidationError {
    pub fn new(message: String) -> Self {
        ValidationError {
            message,
            field: None,
            value: None,
        }
    }

    pub fn with_field(message: String, field: String) -> Self {
        ValidationError {
            message,
            field: Some(field),
            value: None,
        }
    }

    pub fn with_field_value(message: String, field: String, value: String) -> Self {
        ValidationError {
            message,
            field: Some(field),
            value: Some(value),
        }
    }
}

/// Network Error for HTTP, TCP, UDP operations
#[derive(Debug, Clone)]
pub struct NetworkError {
    pub message: String,
    pub kind: NetworkErrorKind,
    pub status_code: Option<u16>,
}

#[derive(Debug, Clone)]
pub enum NetworkErrorKind {
    ConnectionTimeout,
    ConnectionRefused,
    HostUnreachable,
    DnsResolution,
    TlsHandshake,
    HttpStatus,
    InvalidUrl,
    Other,
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.status_code {
            Some(code) => write!(
                f, "Network Error ({}, status: {}): {}", self.kind, code, self.message),
            None => write!(f, "Network Error ({}): {}", self.kind, self.message),
        }
    }
}

impl fmt::Display for NetworkErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind_str = match self {
            NetworkErrorKind::ConnectionTimeout => "ConnectionTimeout",
            NetworkErrorKind::ConnectionRefused => "ConnectionRefused",
            NetworkErrorKind::HostUnreachable => "HostUnreachable",
            NetworkErrorKind::DnsResolution => "DnsResolution",
            NetworkErrorKind::TlsHandshake => "TlsHandshake",
            NetworkErrorKind::HttpStatus => "HttpStatus",
            NetworkErrorKind::InvalidUrl => "InvalidUrl",
            NetworkErrorKind::Other => "Other",
        };
        write!(f, "{}", kind_str)
    }
}

impl ScriptError for NetworkError {
    fn message(&self) -> String {
        self.message.clone()
    }

    fn code(&self) -> Option<i32> {
        self.status_code.map(|c| c as i32)
    }

    fn kind(&self) -> String {
        format!("NetworkError::{}", self.kind)
    }

    fn is_recoverable(&self) -> bool {
        match self.kind {
            NetworkErrorKind::ConnectionTimeout => true,
            NetworkErrorKind::ConnectionRefused => true,
            NetworkErrorKind::HostUnreachable => false,
            NetworkErrorKind::DnsResolution => false,
            NetworkErrorKind::TlsHandshake => false,
            NetworkErrorKind::HttpStatus => self.status_code.map_or(true, |c| c >= 500),
            NetworkErrorKind::InvalidUrl => false,
            NetworkErrorKind::Other => true,
        }
    }
}

/// Parse Error for JSON, YAML, XML, etc. parsing
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub format: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.line, self.column) {
            (Some(line), Some(col)) => {
                write!(
                    f, "Parse Error in {} at line {}, column {}: {}", self.format, line, col, self.message)
            }
            (Some(line), None) => {
                write!(
                    f, "Parse Error in {} at line {}: {}", self.format, line, self.message)
            }
            _ => {
                write!(f, "Parse Error in {}: {}", self.format, self.message)
            }
        }
    }
}

impl ScriptError for ParseError {
    fn message(&self) -> String {
        self.message.clone()
    }

    fn kind(&self) -> String {
        format!("ParseError::{}", self.format)
    }

    fn is_recoverable(&self) -> bool {
        false // Parse errors are typically not recoverable
    }
}

/// Runtime functions for error handling

/// Create an IoError from Rust std::io::Error
pub fn io_error_from_std(err: std::io::Error) -> IoError {
    let kind = match err.kind() {
        std::io::ErrorKind::NotFound => IoErrorKind::NotFound,
        std::io::ErrorKind::PermissionDenied => IoErrorKind::PermissionDenied,
        std::io::ErrorKind::ConnectionRefused => IoErrorKind::ConnectionRefused,
        std::io::ErrorKind::Interrupted => IoErrorKind::Interrupted,
        std::io::ErrorKind::InvalidInput => IoErrorKind::InvalidInput,
        std::io::ErrorKind::InvalidData => IoErrorKind::InvalidData,
        std::io::ErrorKind::TimedOut => IoErrorKind::TimedOut,
        std::io::ErrorKind::WriteZero => IoErrorKind::WriteZero,
        std::io::ErrorKind::UnexpectedEof => IoErrorKind::UnexpectedEof,
        _ => IoErrorKind::Other,
    };

    IoError::new(kind, err.to_string())
}

/// Implementation functions for stdlib registry

/// Create a new ValidationError
pub(crate) fn validation_error_new_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "ValidationError::new expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(message) => {
            let error = ValidationError::new(message.as_str().to_string());
            Ok(error.to_script_value())
        }
        _ => Err(RuntimeError::InvalidOperation(
            "ValidationError::new expects a string argument".to_string(),
        )),
    }
}

/// Create a new IoError
pub(crate) fn io_error_new_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "IoError::new expects 2 arguments, got {}",
            args.len()
        )));
    }

    match (&args[0], &args[1]) {
        (ScriptValue::String(kind_str), ScriptValue::String(message)) => {
            let kind = match kind_str.as_str() {
                "NotFound" => IoErrorKind::NotFound,
                "PermissionDenied" => IoErrorKind::PermissionDenied,
                "ConnectionRefused" => IoErrorKind::ConnectionRefused,
                "InvalidInput" => IoErrorKind::InvalidInput,
                "InvalidData" => IoErrorKind::InvalidData,
                "TimedOut" => IoErrorKind::TimedOut,
                _ => IoErrorKind::Other,
            };

            let error = IoError::new(kind, message.as_str().to_string());
            Ok(error.to_script_value())
        }
        _ => Err(RuntimeError::InvalidOperation(
            "IoError::new expects two string arguments (kind, message)".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_error() {
        let error = IoError::not_found("File not found".to_string());
        assert_eq!(error.kind(), "IoError::NotFound");
        assert_eq!(error.message(), "File not found");
        assert!(error.is_recoverable());
    }

    #[test]
    fn test_validation_error() {
        let error = ValidationError::with_field_value(
            "Invalid email format".to_string(),
            "email".to_string(),
            "invalid-email".to_string(),
        );
        assert_eq!(error.kind(), "ValidationError");
        assert!(error.is_recoverable());
        assert!(error.to_string().contains("field 'email'"));
    }

    #[test]
    fn test_network_error() {
        let error = NetworkError {
            message: "Server error".to_string(),
            kind: NetworkErrorKind::HttpStatus,
            status_code: Some(500),
        };
        assert_eq!(error.kind(), "NetworkError::HttpStatus");
        assert_eq!(error.code(), Some(500));
        assert!(error.is_recoverable()); // 500 status is recoverable
    }

    #[test]
    fn test_parse_error() {
        let error = ParseError {
            message: "Unexpected token".to_string(),
            line: Some(10),
            column: Some(15),
            format: "JSON".to_string(),
        };
        assert_eq!(error.kind(), "ParseError::JSON");
        assert!(!error.is_recoverable());
        assert!(error.to_string().contains("line 10, column 15"));
    }

    #[test]
    fn test_script_error_trait() {
        let error = ValidationError::new("Test error".to_string());
        let script_value = error.to_script_value();

        // Verify it returns a ScriptValue
        match script_value {
            ScriptValue::String(_) => {} // Expected
            _ => panic!("Expected string ScriptValue"),
        }
    }
}
