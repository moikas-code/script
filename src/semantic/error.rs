use crate::error::{Error, ErrorKind};
use crate::source::Span;
use crate::types::Type;
use std::fmt;

/// Kinds of semantic errors
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticErrorKind {
    /// Variable not defined
    UndefinedVariable(String),
    /// Function not defined
    UndefinedFunction(String),
    /// Variable already defined in scope
    DuplicateVariable(String),
    /// Function with same signature already defined
    DuplicateFunction(String),
    /// Type mismatch
    TypeMismatch { expected: Type, found: Type },
    /// Wrong number of arguments
    ArgumentCountMismatch { expected: usize, found: usize },
    /// Cannot assign to immutable variable
    AssignmentToImmutable(String),
    /// Invalid left-hand side of assignment
    InvalidAssignmentTarget,
    /// Break outside of loop
    BreakOutsideLoop,
    /// Continue outside of loop
    ContinueOutsideLoop,
    /// Return outside of function
    ReturnOutsideFunction,
    /// Return type mismatch
    ReturnTypeMismatch { expected: Type, found: Type },
    /// Missing return statement
    MissingReturn { expected: Type },
    /// Invalid operation for type
    InvalidOperation { op: String, ty: Type },
    /// Invalid binary operation
    InvalidBinaryOperation { op: String, left: Type, right: Type },
    /// Function used as value (future: first-class functions)
    FunctionAsValue(String),
    /// Not a function
    NotCallable(Type),
    /// Not indexable
    NotIndexable(Type),
    /// Invalid index type
    InvalidIndexType(Type),
    /// Member access on non-struct type
    InvalidMemberAccess(Type),
    /// Unknown member
    UnknownMember { ty: Type, member: String },
    /// Method not found for type
    MethodNotFound {
        type_name: String,
        method_name: String,
    },
    /// Const function violation
    ConstFunctionViolation(String),
    /// Non-const function called from const function
    NonConstFunctionCall { function: String, caller: String },
    /// I/O operation in const function
    IoInConstFunction(String),
    /// Mutable operation in const function
    MutableOperationInConstFunction(String),
    /// Actor-related error (future)
    ActorError(String),
    /// Module-related errors
    ModuleError(String),
    /// Undefined import
    UndefinedImport { symbol: String, module: String },
    /// Conflicting import
    ConflictingImport(String),
    /// Undefined export
    UndefinedExport(String),
    /// Conflicting export
    ConflictingExport(String),
    /// Module not found
    ModuleNotFound(String),
    /// Circular import
    CircularImport { modules: Vec<String> },
    /// Private symbol access
    PrivateSymbolAccess { symbol: String, module: String },
    /// Memory safety violations
    MemorySafetyViolation(crate::semantic::memory_safety::MemorySafetyViolation),
    /// Non-exhaustive pattern matching
    NonExhaustivePatterns,
    /// Redundant pattern (unreachable)
    RedundantPattern,
    /// Undefined type parameter
    UndefinedTypeParameter(String),
    /// Duplicate field in struct/enum constructor
    DuplicateField(String),
    /// Missing required field
    MissingField(String),
    /// Unknown field
    UnknownField(String),
    /// Not a struct type
    NotAStruct(String),
    /// Not an enum type
    NotAnEnum(String),
    /// Unknown enum variant
    UnknownVariant {
        enum_name: String,
        variant_name: String,
    },
    /// Unqualified enum variant
    UnqualifiedEnumVariant(String),
    /// Variant form mismatch (unit/tuple/struct)
    VariantFormMismatch {
        variant: String,
        expected: String,
        found: String,
    },
    /// Undefined type
    UndefinedType(String),
    /// Error propagation (?) used in non-Result/Option function
    ErrorPropagationInNonResult,
    /// Invalid error propagation on non-Result/Option type
    InvalidErrorPropagation { actual_type: Type },
    /// Duplicate type definition
    DuplicateType(String),
}

/// Semantic error with location information
#[derive(Debug, Clone)]
pub struct SemanticError {
    pub kind: SemanticErrorKind,
    pub span: Span,
    pub notes: Vec<String>,
}

impl SemanticError {
    /// Create a new semantic error
    pub fn new(kind: SemanticErrorKind, span: Span) -> Self {
        SemanticError {
            kind,
            span,
            notes: Vec::new(),
        }
    }

    /// Add a note to this error
    pub fn with_note(mut self, note: String) -> Self {
        self.notes.push(note);
        self
    }

    /// Add a help message to this error (convenience method)
    pub fn with_help(self, help: String) -> Self {
        self.with_note(format!("help: {}", help))
    }

    /// Convert to a general Error
    pub fn into_error(self) -> Error {
        self.into_error_with_source(None)
    }

    /// Convert to a general Error with optional source line context
    pub fn into_error_with_source(self, source_line: Option<&str>) -> Error {
        let message = self.kind.to_string();
        let mut error =
            Error::new(ErrorKind::SemanticError, message).with_location(self.span.start);

        // Add source line context if provided
        if let Some(line) = source_line {
            error = error.with_source_line(line);
        }

        // Add notes as part of the message
        if !self.notes.is_empty() {
            let notes_str = self.notes.join("\n    note: ");
            error = Error::new(
                ErrorKind::SemanticError,
                format!("{}\n    note: {}", error.message, notes_str),
            )
            .with_location(self.span.start);

            // Re-add source line if we had it
            if let Some(line) = source_line {
                error = error.with_source_line(line);
            }
        }

        error
    }
}

impl fmt::Display for SemanticErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SemanticErrorKind::UndefinedVariable(name) => {
                write!(f, "undefined variable '{}'", name)
            }
            SemanticErrorKind::UndefinedFunction(name) => {
                write!(f, "undefined function '{}'", name)
            }
            SemanticErrorKind::DuplicateVariable(name) => {
                write!(f, "variable '{}' is already defined in this scope", name)
            }
            SemanticErrorKind::DuplicateFunction(name) => {
                write!(
                    f,
                    "function '{}' with the same signature is already defined",
                    name
                )
            }
            SemanticErrorKind::TypeMismatch { expected, found } => {
                write!(f, "type mismatch: expected {}, found {}", expected, found)
            }
            SemanticErrorKind::ArgumentCountMismatch { expected, found } => {
                write!(
                    f,
                    "wrong number of arguments: expected {}, found {}",
                    expected, found
                )
            }
            SemanticErrorKind::AssignmentToImmutable(name) => {
                write!(f, "cannot assign to immutable variable '{}'", name)
            }
            SemanticErrorKind::InvalidAssignmentTarget => {
                write!(f, "invalid assignment target")
            }
            SemanticErrorKind::BreakOutsideLoop => {
                write!(f, "'break' can only be used inside a loop")
            }
            SemanticErrorKind::ContinueOutsideLoop => {
                write!(f, "'continue' can only be used inside a loop")
            }
            SemanticErrorKind::ReturnOutsideFunction => {
                write!(f, "'return' can only be used inside a function")
            }
            SemanticErrorKind::ReturnTypeMismatch { expected, found } => {
                write!(
                    f,
                    "return type mismatch: expected {}, found {}",
                    expected, found
                )
            }
            SemanticErrorKind::MissingReturn { expected } => {
                write!(
                    f,
                    "missing return statement in function that returns {}",
                    expected
                )
            }
            SemanticErrorKind::InvalidOperation { op, ty } => {
                write!(f, "invalid operation '{}' for type {}", op, ty)
            }
            SemanticErrorKind::InvalidBinaryOperation { op, left, right } => {
                write!(
                    f,
                    "invalid binary operation '{}' between {} and {}",
                    op, left, right
                )
            }
            SemanticErrorKind::FunctionAsValue(name) => {
                write!(f, "cannot use function '{}' as a value", name)
            }
            SemanticErrorKind::NotCallable(ty) => {
                write!(f, "type {} is not callable", ty)
            }
            SemanticErrorKind::NotIndexable(ty) => {
                write!(f, "type {} cannot be indexed", ty)
            }
            SemanticErrorKind::InvalidIndexType(ty) => {
                write!(f, "invalid index type {}, expected integer", ty)
            }
            SemanticErrorKind::InvalidMemberAccess(ty) => {
                write!(f, "cannot access members on type {}", ty)
            }
            SemanticErrorKind::UnknownMember { ty, member } => {
                write!(f, "type {} has no member '{}'", ty, member)
            }
            SemanticErrorKind::MethodNotFound {
                type_name,
                method_name,
            } => {
                write!(
                    f,
                    "no method '{}' found for type '{}'",
                    method_name, type_name
                )
            }
            SemanticErrorKind::ConstFunctionViolation(msg) => {
                write!(f, "const function violation: {}", msg)
            }
            SemanticErrorKind::NonConstFunctionCall { function, caller } => {
                write!(
                    f,
                    "@const function '{}' cannot call non-const function '{}'",
                    caller, function
                )
            }
            SemanticErrorKind::IoInConstFunction(op) => {
                write!(f, "I/O operation '{}' not allowed in @const functions", op)
            }
            SemanticErrorKind::MutableOperationInConstFunction(op) => {
                write!(
                    f,
                    "mutable operation '{}' not allowed in @const functions",
                    op
                )
            }
            SemanticErrorKind::ActorError(msg) => {
                write!(f, "actor error: {}", msg)
            }
            SemanticErrorKind::ModuleError(msg) => {
                write!(f, "module error: {}", msg)
            }
            SemanticErrorKind::UndefinedImport { symbol, module } => {
                write!(f, "undefined import '{}' from module '{}'", symbol, module)
            }
            SemanticErrorKind::ConflictingImport(name) => {
                write!(f, "conflicting import: '{}' is already imported", name)
            }
            SemanticErrorKind::UndefinedExport(name) => {
                write!(
                    f,
                    "undefined export: '{}' is not defined in this module",
                    name
                )
            }
            SemanticErrorKind::ConflictingExport(name) => {
                write!(f, "conflicting export: '{}' is already exported", name)
            }
            SemanticErrorKind::ModuleNotFound(module) => {
                write!(f, "module not found: '{}'", module)
            }
            SemanticErrorKind::CircularImport { modules } => {
                write!(f, "circular import detected: {}", modules.join(" -> "))
            }
            SemanticErrorKind::PrivateSymbolAccess { symbol, module } => {
                write!(
                    f,
                    "cannot access private symbol '{}' from module '{}'",
                    symbol, module
                )
            }
            SemanticErrorKind::MemorySafetyViolation(violation) => {
                write!(f, "memory safety violation: {}", violation)
            }
            SemanticErrorKind::NonExhaustivePatterns => {
                write!(
                    f,
                    "non-exhaustive patterns: pattern matching does not cover all possible cases"
                )
            }
            SemanticErrorKind::RedundantPattern => {
                write!(f, "redundant pattern: this pattern is unreachable")
            }
            SemanticErrorKind::UndefinedTypeParameter(name) => {
                write!(f, "undefined type parameter '{}'", name)
            }
            SemanticErrorKind::DuplicateField(name) => {
                write!(f, "duplicate field '{}'", name)
            }
            SemanticErrorKind::MissingField(name) => {
                write!(f, "missing required field '{}'", name)
            }
            SemanticErrorKind::UnknownField(name) => {
                write!(f, "unknown field '{}'", name)
            }
            SemanticErrorKind::NotAStruct(name) => {
                write!(f, "'{}' is not a struct type", name)
            }
            SemanticErrorKind::NotAnEnum(name) => {
                write!(f, "'{}' is not an enum type", name)
            }
            SemanticErrorKind::UnknownVariant {
                enum_name,
                variant_name,
            } => {
                write!(
                    f,
                    "no variant '{}' found for enum '{}'",
                    variant_name, enum_name
                )
            }
            SemanticErrorKind::UnqualifiedEnumVariant(variant) => {
                write!(f, "unqualified enum variant '{}'", variant)
            }
            SemanticErrorKind::VariantFormMismatch {
                variant,
                expected,
                found,
            } => {
                write!(
                    f,
                    "variant '{}' expects {}, but {} were provided",
                    variant, expected, found
                )
            }
            SemanticErrorKind::UndefinedType(name) => {
                write!(f, "undefined type '{}'", name)
            }
            SemanticErrorKind::ErrorPropagationInNonResult => {
                write!(
                    f,
                    "the ? operator can only be used in functions that return Result or Option"
                )
            }
            SemanticErrorKind::InvalidErrorPropagation { actual_type } => {
                write!(
                    f,
                    "the ? operator can only be applied to Result or Option types, not {}",
                    actual_type
                )
            }
            SemanticErrorKind::DuplicateType(name) => {
                write!(f, "type '{}' is already defined", name)
            }
        }
    }
}

/// Helper functions for creating common semantic errors
impl SemanticError {
    pub fn undefined_variable(name: &str, span: Span) -> Self {
        SemanticError::new(SemanticErrorKind::UndefinedVariable(name.to_string()), span)
    }

    pub fn undefined_function(name: &str, span: Span) -> Self {
        SemanticError::new(SemanticErrorKind::UndefinedFunction(name.to_string()), span)
    }

    pub fn duplicate_variable(name: &str, span: Span) -> Self {
        SemanticError::new(SemanticErrorKind::DuplicateVariable(name.to_string()), span)
    }

    pub fn type_mismatch(expected: Type, found: Type, span: Span) -> Self {
        SemanticError::new(SemanticErrorKind::TypeMismatch { expected, found }, span)
    }

    pub fn argument_count_mismatch(expected: usize, found: usize, span: Span) -> Self {
        SemanticError::new(
            SemanticErrorKind::ArgumentCountMismatch { expected, found },
            span,
        )
    }

    pub fn assignment_to_immutable(name: &str, span: Span) -> Self {
        SemanticError::new(
            SemanticErrorKind::AssignmentToImmutable(name.to_string()),
            span,
        )
    }

    pub fn invalid_assignment_target(span: Span) -> Self {
        SemanticError::new(SemanticErrorKind::InvalidAssignmentTarget, span)
    }

    pub fn not_callable(ty: Type, span: Span) -> Self {
        SemanticError::new(SemanticErrorKind::NotCallable(ty), span)
    }

    pub fn not_indexable(ty: Type, span: Span) -> Self {
        SemanticError::new(SemanticErrorKind::NotIndexable(ty), span)
    }

    pub fn invalid_index_type(ty: Type, span: Span) -> Self {
        SemanticError::new(SemanticErrorKind::InvalidIndexType(ty), span)
    }

    pub fn invalid_member_access(ty: Type, span: Span) -> Self {
        SemanticError::new(SemanticErrorKind::InvalidMemberAccess(ty), span)
    }

    pub fn unknown_member(ty: Type, member: &str, span: Span) -> Self {
        SemanticError::new(
            SemanticErrorKind::UnknownMember {
                ty,
                member: member.to_string(),
            },
            span,
        )
    }

    pub fn method_not_found(type_name: &str, method_name: &str, span: Span) -> Self {
        SemanticError::new(
            SemanticErrorKind::MethodNotFound {
                type_name: type_name.to_string(),
                method_name: method_name.to_string(),
            },
            span,
        )
    }

    pub fn invalid_operation(op: &str, ty: Type, span: Span) -> Self {
        SemanticError::new(
            SemanticErrorKind::InvalidOperation {
                op: op.to_string(),
                ty,
            },
            span,
        )
    }

    pub fn invalid_binary_operation(op: &str, left: Type, right: Type, span: Span) -> Self {
        SemanticError::new(
            SemanticErrorKind::InvalidBinaryOperation {
                op: op.to_string(),
                left,
                right,
            },
            span,
        )
    }

    pub fn return_type_mismatch(expected: Type, found: Type, span: Span) -> Self {
        SemanticError::new(
            SemanticErrorKind::ReturnTypeMismatch { expected, found },
            span,
        )
    }

    // Module-related error constructors
    pub fn module_error(message: &str, span: Span) -> Self {
        SemanticError::new(SemanticErrorKind::ModuleError(message.to_string()), span)
    }

    pub fn undefined_import(symbol: &str, module: &str, span: Span) -> Self {
        SemanticError::new(
            SemanticErrorKind::UndefinedImport {
                symbol: symbol.to_string(),
                module: module.to_string(),
            },
            span,
        )
    }

    pub fn conflicting_import(name: &str, span: Span) -> Self {
        SemanticError::new(SemanticErrorKind::ConflictingImport(name.to_string()), span)
    }

    pub fn undefined_export(name: &str, span: Span) -> Self {
        SemanticError::new(SemanticErrorKind::UndefinedExport(name.to_string()), span)
    }

    pub fn conflicting_export(name: &str, span: Span) -> Self {
        SemanticError::new(SemanticErrorKind::ConflictingExport(name.to_string()), span)
    }

    pub fn module_not_found(module: &str, span: Span) -> Self {
        SemanticError::new(SemanticErrorKind::ModuleNotFound(module.to_string()), span)
    }

    pub fn circular_import(modules: Vec<String>, span: Span) -> Self {
        SemanticError::new(SemanticErrorKind::CircularImport { modules }, span)
    }

    pub fn private_symbol_access(symbol: &str, module: &str, span: Span) -> Self {
        SemanticError::new(
            SemanticErrorKind::PrivateSymbolAccess {
                symbol: symbol.to_string(),
                module: module.to_string(),
            },
            span,
        )
    }

    // Const function error constructors
    pub fn const_function_violation(message: &str, span: Span) -> Self {
        SemanticError::new(
            SemanticErrorKind::ConstFunctionViolation(message.to_string()),
            span,
        )
    }

    pub fn non_const_function_call(function: &str, caller: &str, span: Span) -> Self {
        SemanticError::new(
            SemanticErrorKind::NonConstFunctionCall {
                function: function.to_string(),
                caller: caller.to_string(),
            },
            span,
        )
    }

    pub fn io_in_const_function(operation: &str, span: Span) -> Self {
        SemanticError::new(
            SemanticErrorKind::IoInConstFunction(operation.to_string()),
            span,
        )
    }

    pub fn mutable_operation_in_const_function(operation: &str, span: Span) -> Self {
        SemanticError::new(
            SemanticErrorKind::MutableOperationInConstFunction(operation.to_string()),
            span,
        )
    }

    // Memory safety error constructors
    pub fn memory_safety_violation(
        violation: crate::semantic::memory_safety::MemorySafetyViolation,
        span: Span,
    ) -> Self {
        SemanticError::new(SemanticErrorKind::MemorySafetyViolation(violation), span)
    }

    // Type-related error constructors
    pub fn undefined_type(name: &str, span: Span) -> Self {
        SemanticError::new(SemanticErrorKind::UndefinedType(name.to_string()), span)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{SourceLocation, Span};

    fn make_span() -> Span {
        Span::new(SourceLocation::new(1, 1, 0), SourceLocation::new(1, 5, 5))
    }

    #[test]
    fn test_error_display() {
        let err = SemanticError::undefined_variable("x", make_span());
        assert_eq!(err.kind.to_string(), "undefined variable 'x'");

        let err = SemanticError::type_mismatch(Type::I32, Type::F32, make_span());
        assert_eq!(
            err.kind.to_string(),
            "type mismatch: expected i32, found f32"
        );

        let err = SemanticError::argument_count_mismatch(2, 3, make_span());
        assert_eq!(
            err.kind.to_string(),
            "wrong number of arguments: expected 2, found 3"
        );
    }

    #[test]
    fn test_error_with_notes() {
        let err = SemanticError::undefined_variable("x", make_span())
            .with_note("did you mean 'y'?".to_string())
            .with_note("variables must be declared before use".to_string());

        let general_err = err.into_error();
        assert!(general_err.message.contains("undefined variable 'x'"));
        assert!(general_err.message.contains("did you mean 'y'?"));
        assert!(general_err
            .message
            .contains("variables must be declared before use"));
    }

    #[test]
    fn test_return_type_mismatch_display() {
        let err = SemanticError::return_type_mismatch(Type::I32, Type::String, make_span());
        assert_eq!(
            err.kind.to_string(),
            "return type mismatch: expected i32, found string"
        );

        let err = SemanticError::return_type_mismatch(Type::Unknown, Type::I32, make_span())
            .with_note("function has no return type annotation, cannot return a value".to_string());
        let general_err = err.into_error();
        assert!(general_err
            .message
            .contains("return type mismatch: expected unknown, found i32"));
        assert!(general_err
            .message
            .contains("function has no return type annotation"));
    }
}
