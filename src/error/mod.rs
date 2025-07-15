pub mod module_context;
mod reporter;

use crate::source::SourceLocation;
use colored::*;
use std::fmt;

pub use reporter::ErrorReporter;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
    pub location: Option<SourceLocation>,
    pub source_line: Option<String>,
    pub file_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    LexerError,
    ParseError,
    TypeError,
    RuntimeError,
    IoError,
    PackageError,
    ModuleError,
    CompilationError,
    FileError,
    SemanticError,
    SecurityViolation,
    // New error kinds for unwrap replacements
    LockPoisoned,
    KeyNotFound,
    IndexOutOfBounds,
    InvalidConversion,
    AsyncError,
    ResourceNotFound,
    InternalError,
    Configuration,
}

impl Error {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            location: None,
            source_line: None,
            file_name: None,
        }
    }

    pub fn lexer(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::LexerError, message)
    }

    pub fn parse(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::ParseError, message)
    }

    pub fn type_error(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::TypeError, message)
    }

    pub fn runtime(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::RuntimeError, message)
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::IoError, message)
    }

    pub fn package(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::PackageError, message)
    }

    pub fn module(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::ModuleError, message)
    }

    pub fn compilation(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::CompilationError, message)
    }

    pub fn file(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::FileError, message)
    }

    pub fn semantic(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::SemanticError, message)
    }

    pub fn security_violation(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::SecurityViolation, message)
    }

    pub fn security_error(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::SecurityViolation, message)
    }

    pub fn lock_poisoned(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::LockPoisoned, message)
    }

    pub fn key_not_found(key: impl Into<String>) -> Self {
        Self::new(
            ErrorKind::KeyNotFound,
            format!("Key not found: {}", key.into()),
        )
    }

    pub fn index_out_of_bounds(index: usize, len: usize) -> Self {
        Self::new(
            ErrorKind::IndexOutOfBounds,
            format!("Index {} out of bounds for length {}", index, len),
        )
    }

    pub fn invalid_conversion(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidConversion, message)
    }

    pub fn async_error(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::AsyncError, message)
    }

    pub fn resource_not_found(resource: impl Into<String>) -> Self {
        Self::new(
            ErrorKind::ResourceNotFound,
            format!("Resource not found: {}", resource.into()),
        )
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::InternalError, message)
    }

    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }

    pub fn with_source_line(mut self, line: impl Into<String>) -> Self {
        self.source_line = Some(line.into());
        self
    }

    pub fn with_file_name(mut self, name: impl Into<String>) -> Self {
        self.file_name = Some(name.into());
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let error_type = match self.kind {
            ErrorKind::LexerError => "lexer error",
            ErrorKind::ParseError => "parse error",
            ErrorKind::TypeError => "type error",
            ErrorKind::RuntimeError => "runtime error",
            ErrorKind::IoError => "io error",
            ErrorKind::PackageError => "package error",
            ErrorKind::ModuleError => "module error",
            ErrorKind::CompilationError => "compilation error",
            ErrorKind::FileError => "file error",
            ErrorKind::SemanticError => "error",
            ErrorKind::SecurityViolation => "security violation",
            ErrorKind::LockPoisoned => "lock poisoned",
            ErrorKind::KeyNotFound => "key not found",
            ErrorKind::IndexOutOfBounds => "index out of bounds",
            ErrorKind::InvalidConversion => "invalid conversion",
            ErrorKind::AsyncError => "async error",
            ErrorKind::ResourceNotFound => "resource not found",
            ErrorKind::InternalError => "internal error",
            ErrorKind::Configuration => "configuration error",
        };

        // Main error message
        write!(
            f,
            "{}{} {}",
            error_type.red().bold(),
            ":".white().bold(),
            self.message
        )?;

        // Location information with better formatting
        if let Some(loc) = &self.location {
            if let Some(file) = &self.file_name {
                write!(
                    f,
                    "\n  {} {}:{}:{}",
                    "-->".blue().bold(),
                    file.cyan(),
                    loc.line.to_string().yellow(),
                    loc.column.to_string().yellow()
                )?;
            } else {
                write!(
                    f,
                    "\n  {} line {}:{}",
                    "-->".blue().bold(),
                    loc.line.to_string().yellow(),
                    loc.column.to_string().yellow()
                )?;
            }
        }

        // Enhanced source line display with better highlighting
        if let (Some(line), Some(loc)) = (&self.source_line, &self.location) {
            let line_num_str = loc.line.to_string();
            let line_num_width = line_num_str.len().max(3);

            // Show line with number
            write!(
                f,
                "\n{:width$} {} {}",
                line_num_str.blue(),
                "|".blue().bold(),
                line,
                width = line_num_width
            )?;

            // Show error indicator with enhanced highlighting
            let spaces = " ".repeat(line_num_width);
            let pointer_padding = " ".repeat(loc.column.saturating_sub(1));
            write!(
                f,
                "\n{} {} {}{}",
                spaces,
                "|".blue().bold(),
                pointer_padding,
                "^".red().bold()
            )?;

            // Add additional context indicators for multi-character tokens
            if line.len() > loc.column {
                let remaining = &line[loc.column..];
                if let Some(first_char) = remaining.chars().next() {
                    if first_char.is_alphanumeric() || first_char == '_' {
                        // Highlight the full identifier/keyword
                        let token_len = remaining
                            .chars()
                            .take_while(|c| c.is_alphanumeric() || *c == '_')
                            .count()
                            .min(10); // Limit to reasonable length
                        if token_len > 1 {
                            write!(f, "{}", "~".repeat(token_len - 1).red())?;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::io(err.to_string())
    }
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(err: std::sync::PoisonError<T>) -> Self {
        Error::lock_poisoned(format!("Lock poisoned: {err}"))
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(err: std::num::ParseIntError) -> Self {
        Error::invalid_conversion(format!("Failed to parse integer: {err}"))
    }
}

impl From<std::num::ParseFloatError> for Error {
    fn from(err: std::num::ParseFloatError) -> Self {
        Error::invalid_conversion(format!("Failed to parse float: {err}"))
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(err: std::string::FromUtf8Error) -> Self {
        Error::invalid_conversion(format!("Invalid UTF-8: {err}"))
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Self {
        Error::invalid_conversion(format!("Invalid UTF-8: {err}"))
    }
}

impl From<String> for Error {
    fn from(err: String) -> Self {
        Error::semantic(err)
    }
}
