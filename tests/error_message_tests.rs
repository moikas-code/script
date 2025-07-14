//! Comprehensive tests for enhanced error message system
//!
//! This module tests the improved error messages, suggestions,
//! and stack trace functionality to ensure they provide
//! helpful developer experience.

use script::error::{Error, ErrorKind};
use script::runtime::{StackFrame, StackTrace, StackTraceBuilder};
use script::semantic::error::{SemanticError, SemanticErrorKind};
use script::source::{SourceLocation, Span};
use script::types::Type;

fn create_test_span() -> Span {
    Span::new(SourceLocation::new(1, 1, 0), SourceLocation::new(1, 10, 10))
}

#[cfg(test)]
mod semantic_error_tests {
    use super::*;

    #[test]
    fn test_undefined_variable_suggestions() {
        let error =
            SemanticError::undefined_variable("usrName", create_test_span()).with_suggestions();

        let formatted = error.into_error().message;
        assert!(formatted.contains("❌ Variable 'usrName' is not defined"));
        assert!(formatted.contains("💡 Suggestions:"));
        assert!(formatted.contains("• Check for typos in the variable name"));
        assert!(formatted.contains("• Ensure the variable is declared before use"));
        assert!(formatted.contains("• Verify the variable is in the correct scope"));
    }

    #[test]
    fn test_undefined_function_suggestions() {
        let error =
            SemanticError::undefined_function("prrint", create_test_span()).with_suggestions();

        let formatted = error.into_error().message;
        assert!(formatted.contains("❌ Function 'prrint' is not defined"));
        assert!(formatted.contains("💡 Suggestions:"));
        assert!(formatted.contains("• Check for typos in the function name"));
        assert!(formatted.contains("• Ensure the function is imported"));
        assert!(formatted.contains("• Verify the function is declared before use"));
    }

    #[test]
    fn test_type_mismatch_enhanced_formatting() {
        let error = SemanticError::type_mismatch(Type::I32, Type::F32, create_test_span())
            .with_suggestions();

        let formatted = error.into_error().message;
        assert!(formatted.contains("╭─ Type Mismatch Details"));
        assert!(formatted.contains("│ Expected: i32"));
        assert!(formatted.contains("│    Found: f32"));
        assert!(formatted.contains("╰─"));
        assert!(formatted.contains("💡 cast to int using `as int`"));
    }

    #[test]
    fn test_type_mismatch_specific_suggestions() {
        // Test int to float conversion suggestion
        let error = SemanticError::type_mismatch(Type::F32, Type::I32, create_test_span())
            .with_suggestions();
        let formatted = error.into_error().message;
        assert!(formatted.contains("💡 cast to float using `as float`"));

        // Test string conversion suggestion
        let error = SemanticError::type_mismatch(Type::String, Type::I32, create_test_span())
            .with_suggestions();
        let formatted = error.into_error().message;
        assert!(formatted.contains("💡 convert to string using `toString()`"));

        // Test boolean conversion suggestion
        let error = SemanticError::type_mismatch(Type::Bool, Type::I32, create_test_span())
            .with_suggestions();
        let formatted = error.into_error().message;
        assert!(formatted.contains("💡 use comparison operator (==, !=, <, >)"));
    }

    #[test]
    fn test_argument_count_mismatch_suggestions() {
        let error =
            SemanticError::argument_count_mismatch(3, 2, create_test_span()).with_suggestions();

        let formatted = error.into_error().message;
        assert!(formatted.contains("❌ Wrong number of arguments"));
        assert!(formatted.contains("Expected: 3 arguments"));
        assert!(formatted.contains("Found:    2 arguments"));
        assert!(formatted.contains("💡 Add missing arguments"));

        let error =
            SemanticError::argument_count_mismatch(2, 4, create_test_span()).with_suggestions();

        let formatted = error.into_error().message;
        assert!(formatted.contains("💡 Remove extra arguments"));
    }

    #[test]
    fn test_assignment_to_immutable_suggestions() {
        let error =
            SemanticError::assignment_to_immutable("count", create_test_span()).with_suggestions();

        let formatted = error.into_error().message;
        assert!(formatted.contains("❌ Cannot assign to immutable variable 'count'"));
        assert!(formatted.contains("💡 Make the variable mutable with `let mut`"));
        assert!(formatted.contains("💡 Or create a new variable with `let` (shadowing)"));
    }

    #[test]
    fn test_control_flow_error_suggestions() {
        let error = SemanticError::new(SemanticErrorKind::BreakOutsideLoop, create_test_span())
            .with_suggestions();

        let formatted = error.into_error().message;
        assert!(formatted.contains("❌ 'break' can only be used inside loops"));
        assert!(formatted.contains("💡 Use 'break' inside 'while', 'for', or 'loop'"));
        assert!(formatted.contains("💡 Consider using 'return' to exit from a function"));

        let error = SemanticError::new(SemanticErrorKind::ContinueOutsideLoop, create_test_span())
            .with_suggestions();

        let formatted = error.into_error().message;
        assert!(formatted.contains("❌ 'continue' can only be used inside loops"));
    }

    #[test]
    fn test_missing_return_suggestions() {
        let error = SemanticError::new(
            SemanticErrorKind::MissingReturn {
                expected: Type::I32,
            },
            create_test_span(),
        )
        .with_suggestions();

        let formatted = error.into_error().message;
        assert!(formatted.contains("❌ Missing return statement for type i32"));
        assert!(formatted.contains("💡 Add a return statement at the end"));
        assert!(formatted.contains("💡 Or change the function return type to 'void'"));
    }

    #[test]
    fn test_callable_error_suggestions() {
        let error =
            SemanticError::not_callable(Type::String, create_test_span()).with_suggestions();

        let formatted = error.into_error().message;
        assert!(formatted.contains("❌ Type string cannot be called like a function"));
        assert!(formatted.contains("💡 Use method syntax: value.method()"));

        let error = SemanticError::not_callable(Type::I32, create_test_span()).with_suggestions();

        let formatted = error.into_error().message;
        assert!(formatted.contains("💡 Only functions and closures can be called"));
    }

    #[test]
    fn test_index_error_suggestions() {
        let error =
            SemanticError::invalid_index_type(Type::String, create_test_span()).with_suggestions();

        let formatted = error.into_error().message;
        assert!(formatted.contains("❌ Invalid index type: string"));
        assert!(formatted.contains("💡 Array and string indices must be integers"));
        assert!(formatted.contains("💡 Parse the string to integer first: str.parse()"));

        let error =
            SemanticError::invalid_index_type(Type::F32, create_test_span()).with_suggestions();

        let formatted = error.into_error().message;
        assert!(formatted.contains("💡 Array and string indices must be integers"));
        assert!(!formatted.contains("Parse the string"));
    }

    #[test]
    fn test_pattern_matching_error_suggestions() {
        let error =
            SemanticError::new(SemanticErrorKind::NonExhaustivePatterns, create_test_span())
                .with_suggestions();

        let formatted = error.into_error().message;
        assert!(formatted.contains("❌ Pattern matching is not exhaustive"));
        assert!(formatted.contains("💡 Add patterns to cover all possible cases"));
        assert!(formatted.contains("💡 Use wildcard pattern (_) for catch-all"));

        let error = SemanticError::new(SemanticErrorKind::RedundantPattern, create_test_span())
            .with_suggestions();

        let formatted = error.into_error().message;
        assert!(formatted.contains("❌ This pattern is unreachable"));
        assert!(formatted.contains("💡 Remove the redundant pattern"));
        assert!(formatted.contains("💡 Check pattern order"));
    }

    #[test]
    fn test_type_definition_error_suggestions() {
        let error =
            SemanticError::undefined_type("MyStruct", create_test_span()).with_suggestions();

        let formatted = error.into_error().message;
        assert!(formatted.contains("❌ Undefined type 'MyStruct'"));
        assert!(formatted.contains("💡 Check for typos in the type name"));
        assert!(formatted.contains("💡 Ensure the type is imported"));

        let error = SemanticError::new(
            SemanticErrorKind::DuplicateField("name".to_string()),
            create_test_span(),
        )
        .with_suggestions();

        let formatted = error.into_error().message;
        assert!(formatted.contains("❌ Duplicate field 'name'"));
        assert!(formatted.contains("💡 Remove the duplicate field definition"));

        let error = SemanticError::new(
            SemanticErrorKind::MissingField("id".to_string()),
            create_test_span(),
        )
        .with_suggestions();

        let formatted = error.into_error().message;
        assert!(formatted.contains("❌ Missing required field 'id'"));
        assert!(formatted.contains("💡 Add the missing field to the struct"));

        let error = SemanticError::new(
            SemanticErrorKind::UnknownField("age".to_string()),
            create_test_span(),
        )
        .with_suggestions();

        let formatted = error.into_error().message;
        assert!(formatted.contains("❌ Unknown field 'age'"));
        assert!(formatted.contains("💡 Check field name spelling"));
        assert!(formatted.contains("💡 Verify the field exists in the struct"));
    }

    #[test]
    fn test_member_access_error_suggestions() {
        let error = SemanticError::unknown_member(Type::I32, "length", create_test_span())
            .with_suggestions();

        let formatted = error.into_error().message;
        assert!(formatted.contains("❌ Type i32 has no member 'length'"));
        assert!(formatted.contains("💡 Check the spelling of the member name"));
        assert!(formatted.contains("💡 Verify the member exists for this type"));

        let error = SemanticError::method_not_found("Vector", "append", create_test_span())
            .with_suggestions();

        let formatted = error.into_error().message;
        assert!(formatted.contains("❌ No method 'append' found for type 'Vector'"));
        assert!(formatted.contains("💡 Check method spelling and availability"));
        assert!(formatted.contains("💡 Verify the type supports this method"));
    }

    #[test]
    fn test_generic_fallback_suggestion() {
        // Test that unknown error types get the generic fallback
        let error = SemanticError::new(
            SemanticErrorKind::ActorError("test".to_string()),
            create_test_span(),
        )
        .with_suggestions();

        let formatted = error.into_error().message;
        assert!(formatted.contains("💡 Check the Script language documentation"));
    }
}

#[cfg(test)]
mod stack_trace_tests {
    use super::*;

    #[test]
    fn test_stack_frame_creation() {
        let frame = StackFrame::new("test_function".to_string());
        assert_eq!(frame.function_name, "test_function");
        assert!(frame.file_name.is_none());
        assert!(frame.line_number.is_none());
    }

    #[test]
    fn test_stack_frame_with_location() {
        let frame =
            StackFrame::with_location("main".to_string(), "main.script".to_string(), 42, Some(15));

        assert_eq!(frame.function_name, "main");
        assert_eq!(frame.file_name, Some("main.script".to_string()));
        assert_eq!(frame.line_number, Some(42));
        assert_eq!(frame.column_number, Some(15));
    }

    #[test]
    fn test_stack_frame_with_module() {
        let frame = StackFrame::new("helper".to_string()).with_module("utils".to_string());

        assert_eq!(frame.function_name, "helper");
        assert_eq!(frame.module_name, Some("utils".to_string()));
    }

    #[test]
    fn test_stack_frame_display() {
        let frame = StackFrame::with_location(
            "calculate".to_string(),
            "math.script".to_string(),
            100,
            Some(25),
        )
        .with_module("math".to_string());

        let display = frame.to_string();
        assert!(display.contains("math::calculate"));
        assert!(display.contains("math.script:100:25"));
    }

    #[test]
    fn test_stack_frame_format_location() {
        let frame =
            StackFrame::with_location("test".to_string(), "test.script".to_string(), 10, Some(5));
        assert_eq!(frame.format_location(), "test.script:10:5");

        let frame =
            StackFrame::with_location("test".to_string(), "test.script".to_string(), 10, None);
        assert_eq!(frame.format_location(), "test.script:10");

        let frame = StackFrame::new("test".to_string());
        assert_eq!(frame.format_location(), "<unknown>");
    }

    #[test]
    fn test_stack_trace_builder() {
        let mut builder = StackTraceBuilder::new();

        builder.push_function("main".to_string());
        builder.push_location(
            "helper".to_string(),
            "helper.script".to_string(),
            50,
            Some(10),
        );
        builder.push_function("utility".to_string());

        let trace = builder.build();
        assert_eq!(trace.frames.len(), 3);
        assert_eq!(trace.frames[0].function_name, "main");
        assert_eq!(trace.frames[1].function_name, "helper");
        assert_eq!(trace.frames[2].function_name, "utility");

        // Check that location info is preserved
        assert_eq!(trace.frames[1].file_name, Some("helper.script".to_string()));
        assert_eq!(trace.frames[1].line_number, Some(50));
        assert_eq!(trace.frames[1].column_number, Some(10));
    }

    #[test]
    fn test_stack_trace_from_frames() {
        let frames = vec![
            StackFrame::new("first".to_string()),
            StackFrame::new("second".to_string()),
            StackFrame::new("third".to_string()),
        ];

        let trace = StackTrace::from_frames(frames.clone());
        assert_eq!(trace.frames.len(), 3);
        assert_eq!(trace.total_frames, 3);
        assert!(!trace.is_truncated());
        assert_eq!(trace.top_frame().unwrap().function_name, "first");
    }

    #[test]
    fn test_stack_trace_format() {
        let mut builder = StackTraceBuilder::new();
        builder.push_location("main".to_string(), "main.script".to_string(), 10, Some(5));
        builder.push_function("calculate".to_string());
        builder.push_location("helper".to_string(), "utils.script".to_string(), 25, None);

        let trace = builder.build();
        let formatted = trace.format_trace();

        assert!(formatted.contains("Stack trace:"));
        assert!(formatted.contains("0 - main at main.script:10:5"));
        assert!(formatted.contains("1 - calculate"));
        assert!(formatted.contains("2 - helper at utils.script:25"));
    }

    #[test]
    fn test_stack_trace_empty() {
        let trace = StackTrace::new();
        assert!(trace.frames.is_empty());
        assert_eq!(trace.total_frames, 0);
        assert!(trace.top_frame().is_none());

        let formatted = trace.format_trace();
        assert_eq!(formatted, "Stack trace unavailable");
    }

    #[test]
    fn test_stack_trace_truncation() {
        let frames = vec![StackFrame::new("frame".to_string()); 5];
        let mut trace = StackTrace::from_frames(frames);

        // Simulate truncation
        trace.frames.truncate(3);
        trace.total_frames = 5;

        assert!(trace.is_truncated());
        assert_eq!(trace.frames.len(), 3);
        assert_eq!(trace.total_frames, 5);

        let formatted = trace.format_trace();
        assert!(formatted.contains("... (2 more frames)"));
    }

    #[test]
    fn test_stack_trace_system_capture() {
        let trace = StackTrace::capture_system();
        // System trace should have some frames
        assert!(!trace.frames.is_empty());
        assert!(trace.total_frames > 0);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use script::runtime::RuntimeError;

    #[test]
    fn test_runtime_error_with_stack_trace() {
        let mut builder = StackTraceBuilder::new();
        builder.push_function("main".to_string());
        builder.push_function("problematic_function".to_string());
        let trace = builder.build();

        let error = RuntimeError::RuntimeErrorWithTrace {
            message: "Division by zero".to_string(),
            trace,
        };

        let formatted = error.to_string();
        assert!(formatted.contains("Runtime error: Division by zero"));
        assert!(formatted.contains("Stack trace:"));
        assert!(formatted.contains("main"));
        assert!(formatted.contains("problematic_function"));
    }

    #[test]
    fn test_semantic_error_with_source_context() {
        let source_line = "let x: int = \"hello\";";
        let error = SemanticError::type_mismatch(Type::I32, Type::String, create_test_span())
            .with_suggestions()
            .into_error_with_source(Some(source_line));

        let formatted = error.message;
        assert!(formatted.contains("╭─ Type Mismatch Details"));
        assert!(formatted.contains("│ Expected: i32"));
        assert!(formatted.contains("│    Found: string"));
        assert!(formatted.contains("💡 convert to string using `toString()`"));
    }

    #[test]
    fn test_comprehensive_error_pipeline() {
        // Simulate a complex error scenario with multiple suggestions
        let error = SemanticError::new(
            SemanticErrorKind::TypeMismatch {
                expected: Type::Bool,
                found: Type::I32,
            },
            create_test_span(),
        )
        .with_suggestions()
        .with_note("This error occurred in function validation".to_string())
        .with_help("Consider using explicit type conversion".to_string());

        let formatted = error.into_error().message;

        // Check that all parts are present
        assert!(formatted.contains("╭─ Type Mismatch Details"));
        assert!(formatted.contains("│ Expected: bool"));
        assert!(formatted.contains("│    Found: i32"));
        assert!(formatted.contains("💡 use comparison operator"));
        assert!(formatted.contains("This error occurred in function validation"));
        assert!(formatted.contains("Consider using explicit type conversion"));
    }

    #[test]
    fn test_error_message_consistency() {
        // Test that error messages are consistent and well-formatted
        let test_cases = vec![
            SemanticError::undefined_variable("test", create_test_span()),
            SemanticError::undefined_function("test", create_test_span()),
            SemanticError::type_mismatch(Type::I32, Type::String, create_test_span()),
            SemanticError::argument_count_mismatch(2, 3, create_test_span()),
            SemanticError::assignment_to_immutable("test", create_test_span()),
        ];

        for error in test_cases {
            let formatted = error.with_suggestions().into_error().message;

            // All enhanced errors should have these characteristics
            assert!(formatted.contains("❌") || formatted.contains("╭─"));
            assert!(formatted.contains("💡"));
            assert!(!formatted.is_empty());
            assert!(formatted.len() > 10); // Should be reasonably descriptive
        }
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_error_formatting_performance() {
        let start = Instant::now();

        // Create 1000 errors with suggestions
        for i in 0..1000 {
            let error = SemanticError::type_mismatch(Type::I32, Type::String, create_test_span())
                .with_suggestions()
                .with_note(format!("Test error {}", i));

            let _formatted = error.into_error().message;
        }

        let duration = start.elapsed();

        // Should complete in reasonable time (less than 100ms for 1000 errors)
        assert!(
            duration.as_millis() < 100,
            "Error formatting took too long: {:?}",
            duration
        );
    }

    #[test]
    fn test_stack_trace_building_performance() {
        let start = Instant::now();

        // Build 100 stack traces with 50 frames each
        for _ in 0..100 {
            let mut builder = StackTraceBuilder::new();
            for j in 0..50 {
                builder.push_location(
                    format!("function_{}", j),
                    format!("file_{}.script", j),
                    j as u32 + 1,
                    Some((j % 80) as u32 + 1),
                );
            }
            let _trace = builder.build();
        }

        let duration = start.elapsed();

        // Should complete in reasonable time (less than 50ms for 100 traces)
        assert!(
            duration.as_millis() < 50,
            "Stack trace building took too long: {:?}",
            duration
        );
    }
}
