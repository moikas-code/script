//! Module-aware error context for enhanced error reporting
//!
//! This module provides rich error context that includes module import chains,
//! source locations across modules, and detailed diagnostics for cross-module issues.

use crate::error::{Error, ErrorKind};
use crate::module::{ImportPath, ModulePath};
use crate::source::Span;
use crate::types::Type;
use std::path::PathBuf;

/// Enhanced error with module context
#[derive(Debug, Clone)]
pub struct ModuleAwareError {
    /// The base error
    pub error: Error,
    /// Module where the error occurred
    pub module: ModulePath,
    /// Import chain leading to this error
    pub import_chain: Vec<ImportStep>,
    /// Related errors in other modules
    pub related_errors: Vec<RelatedError>,
    /// Suggestions for fixing the error
    pub suggestions: Vec<String>,
}

/// A step in the import chain
#[derive(Debug, Clone)]
pub struct ImportStep {
    /// Module that contains the import
    pub from_module: ModulePath,
    /// Module being imported
    pub to_module: ModulePath,
    /// Import statement
    pub import: ImportPath,
    /// Source location of the import
    pub location: Span,
    /// Source file path
    pub source_file: Option<PathBuf>,
}

/// Related error in another module
#[derive(Debug, Clone)]
pub struct RelatedError {
    /// Module with the related error
    pub module: ModulePath,
    /// Error message
    pub message: String,
    /// Source location
    pub location: Option<Span>,
}

impl ModuleAwareError {
    /// Create a new module-aware error
    pub fn new(error: Error, module: ModulePath) -> Self {
        ModuleAwareError {
            error,
            module,
            import_chain: Vec::new(),
            related_errors: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    /// Add an import step to the chain
    pub fn add_import_step(&mut self, step: ImportStep) {
        self.import_chain.push(step);
    }

    /// Add a related error
    pub fn add_related_error(&mut self, related: RelatedError) {
        self.related_errors.push(related);
    }

    /// Add a suggestion
    pub fn add_suggestion(&mut self, suggestion: impl Into<String>) {
        self.suggestions.push(suggestion.into());
    }

    /// Format the error with full context
    pub fn format_with_context(&self) -> String {
        let mut output = String::new();

        // Main error
        output.push_str(format!(
            "error[{}]: {}\n",
            self.error_code(),
            self.error.message
        ));
        output.push_str(format!(
            " --> {}:{}\n",
            self.module,
            self.error
                .location
                .map(|s| s.to_string())
                .unwrap_or_default()
        ));

        // Import chain
        if !self.import_chain.is_empty() {
            output.push_str("\nImport chain:\n");
            for (i, step) in self.import_chain.iter().enumerate() {
                let indent = "  ".repeat(i);
                output.push_str(format!(
                    "{}{} imports {}\n",
                    indent, step.from_module, step.to_module
                ));
                if let Some(file) = &step.source_file {
                    output.push_str(format!(
                        "{}  at {}:{}\n",
                        indent,
                        file.display(),
                        step.location
                    ));
                }
            }
        }

        // Related errors
        if !self.related_errors.is_empty() {
            output.push_str("\nRelated errors:\n");
            for related in &self.related_errors {
                output.push_str(format!("  - [{}] {}\n", related.module, related.message));
            }
        }

        // Suggestions
        if !self.suggestions.is_empty() {
            output.push_str("\nSuggestions:\n");
            for suggestion in &self.suggestions {
                output.push_str(format!("  - {}\n", suggestion));
            }
        }

        output
    }

    /// Get error code for categorization
    fn error_code(&self) -> &'static str {
        match &self.error.kind {
            ErrorKind::ModuleError => "E0001",
            ErrorKind::TypeError => "E0002",
            ErrorKind::SemanticError => "E0003",
            ErrorKind::SecurityViolation => "E0004",
            _ => "E0000",
        }
    }
}

/// Builder for creating module-aware errors with rich context
pub struct ModuleErrorBuilder {
    base_error: Option<Error>,
    module: Option<ModulePath>,
    import_chain: Vec<ImportStep>,
    related_errors: Vec<RelatedError>,
    suggestions: Vec<String>,
}

impl ModuleErrorBuilder {
    pub fn new() -> Self {
        ModuleErrorBuilder {
            base_error: None,
            module: None,
            import_chain: Vec::new(),
            related_errors: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    pub fn error(mut self, error: Error) -> Self {
        self.base_error = Some(error);
        self
    }

    pub fn module(mut self, module: ModulePath) -> Self {
        self.module = Some(module);
        self
    }

    pub fn import_chain(mut self, chain: Vec<ImportStep>) -> Self {
        self.import_chain = chain;
        self
    }

    pub fn add_import_step(mut self, step: ImportStep) -> Self {
        self.import_chain.push(step);
        self
    }

    pub fn related_error(
        mut self,
        module: ModulePath,
        message: String,
        location: Option<Span>,
    ) -> Self {
        self.related_errors.push(RelatedError {
            module,
            message,
            location,
        });
        self
    }

    pub fn suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }

    pub fn build(self) -> Result<ModuleAwareError, String> {
        let error = self.base_error.ok_or("Error is required")?;
        let module = self.module.ok_or("Module is required")?;

        Ok(ModuleAwareError {
            error,
            module,
            import_chain: self.import_chain,
            related_errors: self.related_errors,
            suggestions: self.suggestions,
        })
    }
}

/// Cross-module type mismatch error with detailed diagnostics
#[derive(Debug)]
pub struct CrossModuleTypeMismatch {
    /// Expected type
    pub expected: Type,
    /// Expected type's module
    pub expected_module: ModulePath,
    /// Actual type
    pub actual: Type,
    /// Actual type's module
    pub actual_module: ModulePath,
    /// Context of the mismatch
    pub context: TypeMismatchContext,
    /// Location where mismatch occurred
    pub location: Span,
}

#[derive(Debug)]
pub enum TypeMismatchContext {
    /// Function parameter type mismatch
    FunctionParameter {
        function: String,
        parameter: String,
        position: usize,
    },
    /// Function return type mismatch
    FunctionReturn { function: String },
    /// Variable assignment type mismatch
    VariableAssignment { variable: String },
    /// Import type mismatch
    ImportedSymbol { symbol: String },
}

impl CrossModuleTypeMismatch {
    /// Format as a detailed error message
    pub fn format_error(&self) -> String {
        let mut output = String::new();

        // Main error message
        output.push_str("error: type mismatch across module boundary\n");

        // Context-specific message
        match &self.context {
            TypeMismatchContext::FunctionParameter {
                function,
                parameter,
                position,
            } => {
                output.push_str(format!(
                    "  Function '{}' parameter '{}' (position {}) expects type '{}' from module '{}'\n",
                    function, parameter, position, self.expected, self.expected_module
                ));
                output.push_str(format!(
                    "  but received type '{}' from module '{}'\n",
                    self.actual, self.actual_module
                ));
            }
            TypeMismatchContext::FunctionReturn { function } => {
                output.push_str(format!(
                    "  Function '{}' is declared to return type '{}' from module '{}'\n",
                    function, self.expected, self.expected_module
                ));
                output.push_str(format!(
                    "  but returns type '{}' from module '{}'\n",
                    self.actual, self.actual_module
                ));
            }
            TypeMismatchContext::VariableAssignment { variable } => {
                output.push_str(format!(
                    "  Variable '{}' has type '{}' from module '{}'\n",
                    variable, self.expected, self.expected_module
                ));
                output.push_str(format!(
                    "  but is being assigned type '{}' from module '{}'\n",
                    self.actual, self.actual_module
                ));
            }
            TypeMismatchContext::ImportedSymbol { symbol } => {
                output.push_str(format!(
                    "  Imported symbol '{}' was expected to have type '{}'\n",
                    symbol, self.expected
                ));
                output.push_str(format!(
                    "  but has type '{}' in module '{}'\n",
                    self.actual, self.actual_module
                ));
            }
        }

        // Add location
        output.push_str(format!(" --> at {}\n", self.location));

        // Add help text
        output.push_str("\nhelp: ensure that types are compatible across module boundaries\n");
        if self.expected_module != self.actual_module {
            output.push_str(
                "note: types with the same name from different modules are considered distinct\n",
            );
        }

        output
    }
}

/// Import resolution failure with detailed diagnostics
#[derive(Debug)]
pub struct ImportResolutionFailure {
    /// The import that failed
    pub import: ImportPath,
    /// Module attempting the import
    pub importing_module: ModulePath,
    /// Candidates that were considered
    pub candidates: Vec<ModulePath>,
    /// Reason for failure
    pub reason: ImportFailureReason,
    /// Location of the import statement
    pub location: Span,
}

#[derive(Debug)]
pub enum ImportFailureReason {
    /// Module not found
    NotFound,
    /// Multiple matching modules (ambiguous)
    Ambiguous,
    /// Permission denied
    PermissionDenied { reason: String },
    /// Circular dependency
    CircularDependency { chain: Vec<ModulePath> },
    /// Version conflict
    VersionConflict { required: String, found: String },
}

impl ImportResolutionFailure {
    /// Format as a detailed error message
    pub fn format_error(&self) -> String {
        let mut output = String::new();

        // Main error message
        output.push_str(format!(
            "error: cannot resolve import '{}' in module '{}'\n",
            self.import, self.importing_module
        ));

        // Reason-specific details
        match &self.reason {
            ImportFailureReason::NotFound => {
                output.push_str("  Module not found in any search path\n");
                if !self.candidates.is_empty() {
                    output.push_str("\n  Did you mean one of these?\n");
                    for candidate in &self.candidates {
                        output.push_str(format!("    - {}\n", candidate));
                    }
                }
            }
            ImportFailureReason::Ambiguous => {
                output.push_str("  Multiple modules match this import:\n");
                for candidate in &self.candidates {
                    output.push_str(format!("    - {}\n", candidate));
                }
                output.push_str("\n  Use a more specific import path\n");
            }
            ImportFailureReason::PermissionDenied { reason } => {
                output.push_str(format!("  Permission denied: {}\n", reason));
            }
            ImportFailureReason::CircularDependency { chain } => {
                output.push_str("  Circular dependency detected:\n");
                for (i, module) in chain.iter().enumerate() {
                    if i > 0 {
                        output.push_str("    ↓\n");
                    }
                    output.push_str(format!("    {}\n", module));
                }
            }
            ImportFailureReason::VersionConflict { required, found } => {
                output.push_str(format!(
                    "  Version conflict: requires {} but found {}\n",
                    required, found
                ));
            }
        }

        // Add location
        output.push_str(format!("\n --> at {}\n", self.location));

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::ModuleError;
    use crate::source::SourceLocation;

    #[test]
    fn test_module_error_builder() {
        let base_error = Error::new(ErrorKind::ModuleError, "Module not found");

        let module = ModulePath::from_string("app.main").unwrap();

        let error = ModuleErrorBuilder::new()
            .error(base_error)
            .module(module.clone())
            .suggestion("Check if the module path is correct")
            .suggestion("Ensure the module file exists")
            .build()
            .unwrap();

        assert_eq!(error.module, module);
        assert_eq!(error.suggestions.len(), 2);
    }

    #[test]
    fn test_cross_module_type_mismatch() {
        let expected = Type::Named("User".to_string());
        let actual = Type::Named("User".to_string());
        let expected_module = ModulePath::from_string("auth.models").unwrap();
        let actual_module = ModulePath::from_string("api.models").unwrap();

        let mismatch = CrossModuleTypeMismatch {
            expected,
            expected_module,
            actual,
            actual_module,
            context: TypeMismatchContext::FunctionParameter {
                function: "createUser".to_string(),
                parameter: "user".to_string(),
                position: 0,
            },
            location: Span::new(
                SourceLocation::new(10, 5, 100),
                SourceLocation::new(10, 20, 115),
            ),
        };

        let error_msg = mismatch.format_error();
        assert!(error_msg.contains("type mismatch across module boundary"));
        assert!(error_msg.contains("auth.models"));
        assert!(error_msg.contains("api.models"));
    }
}
