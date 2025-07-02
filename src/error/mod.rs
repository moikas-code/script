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
            ErrorKind::LexerError => "Lexer Error",
            ErrorKind::ParseError => "Parse Error",
            ErrorKind::TypeError => "Type Error",
            ErrorKind::RuntimeError => "Runtime Error",
            ErrorKind::IoError => "IO Error",
            ErrorKind::PackageError => "Package Error",
            ErrorKind::ModuleError => "Module Error",
            ErrorKind::CompilationError => "Compilation Error",
            ErrorKind::FileError => "File Error",
            ErrorKind::SemanticError => "Semantic Error",
        };

        write!(f, "{}: {}", error_type.red().bold(), self.message)?;

        if let Some(loc) = &self.location {
            if let Some(file) = &self.file_name {
                write!(f, "\n    {} {}:{}", "-->".cyan(), file, loc)?;
            } else {
                write!(f, "\n    {} {}", "-->".cyan(), loc)?;
            }
        }

        if let (Some(line), Some(loc)) = (&self.source_line, &self.location) {
            write!(f, "\n{:4} | {}", loc.line, line)?;
            write!(f, "\n     | {}{}", " ".repeat(loc.column - 1), "^".red())?;
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
