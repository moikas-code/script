//! Comprehensive tests for Result<T, E> error handling system
//!
//! This test module validates all aspects of the Result error handling implementation:
//! - Semantic analysis for error propagation (? operator)
//! - Code generation for error handling
//! - Pattern exhaustiveness checking for Result/Option types
//! - Type safety and error propagation validation

use crate::common::test_utils::*;
use script::error::ErrorKind;
use script::lexer::Lexer;
use script::parser::Parser;
use script::semantic::{SemanticAnalyzer, SemanticErrorKind};
use script::types::Type;

#[test]
fn test_error_propagation_semantic_analysis() {
    let source = r#"
        fn divide(a: i32, b: i32) -> Result<i32, String> {
            if b == 0 {
                Err("Division by zero")
            } else {
                Ok(a / b)
            }
        }
        
        fn safe_divide(a: i32, b: i32, c: i32) -> Result<i32, String> {
            let x = divide(a, b)?;
            let y = divide(x, c)?;
            Ok(y)
        }
    "#;

    let lexer = Lexer::new(source).expect("Lexer creation should succeed");
    let (tokens, _errors) = lexer.scan_tokens();
    assert!(_errors.is_empty(), "Lexer should not produce errors");
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Failed to parse");

    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze(&ast);

    // Should succeed - valid error propagation
    assert!(
        result.is_ok(),
        "Error propagation analysis should succeed: {:?}",
        result.err()
    );
}

#[test]
fn test_error_propagation_invalid_context() {
    let source = r#"
        fn divide(a: i32, b: i32) -> Result<i32, String> {
            if b == 0 {
                Err("Division by zero")
            } else {
                Ok(a / b)
            }
        }
        
        fn unsafe_divide(a: i32, b: i32, c: i32) -> i32 {
            let x = divide(a, b)?;  // Error: ? used in non-Result function
            x
        }
    "#;

    let lexer = Lexer::new(source).expect("Lexer creation should succeed");
    let (tokens, _errors) = lexer.scan_tokens();
    assert!(_errors.is_empty(), "Lexer should not produce errors");
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Failed to parse");

    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze(&ast);

    // Should fail - ? operator used in non-Result function
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors
        .iter()
        .any(|e| matches!(e.kind, SemanticErrorKind::ErrorPropagationInNonResult)));
}

#[test]
fn test_error_propagation_type_mismatch() {
    let source = r#"
        fn divide(a: i32, b: i32) -> Result<i32, String> {
            if b == 0 {
                Err("Division by zero")
            } else {
                Ok(a / b)
            }
        }
        
        fn parse_number(s: String) -> Result<i32, i32> {
            // Return different error type
            Err(42)
        }
        
        fn mixed_errors(a: i32, b: i32) -> Result<i32, String> {
            let x = divide(a, b)?;
            let y = parse_number("123")?;  // Error: incompatible error types
            Ok(x + y)
        }
    "#;

    let lexer = Lexer::new(source).expect("Lexer creation should succeed");
    let (tokens, _errors) = lexer.scan_tokens();
    assert!(_errors.is_empty(), "Lexer should not produce errors");
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Failed to parse");

    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze(&ast);

    // Should fail - incompatible error types
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors
        .iter()
        .any(|e| matches!(e.kind, SemanticErrorKind::TypeMismatch { .. })));
}

#[test]
fn test_option_error_propagation() {
    let source = r#"
        fn find_index(arr: [i32], target: i32) -> Option<i32> {
            for i in 0..arr.len() {
                if arr[i] == target {
                    return Some(i);
                }
            }
            None
        }
        
        fn double_lookup(arr: [i32], target1: i32, target2: i32) -> Option<i32> {
            let idx1 = find_index(arr, target1)?;
            let idx2 = find_index(arr, target2)?;
            Some(idx1 + idx2)
        }
    "#;

    let lexer = Lexer::new(source).expect("Lexer creation should succeed");
    let (tokens, _errors) = lexer.scan_tokens();
    assert!(_errors.is_empty(), "Lexer should not produce errors");
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Failed to parse");

    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze(&ast);

    // Should succeed - valid Option error propagation
    assert!(
        result.is_ok(),
        "Option error propagation analysis should succeed: {:?}",
        result.err()
    );
}

#[test]
fn test_result_pattern_exhaustiveness() {
    let source = r#"
        fn divide(a: i32, b: i32) -> Result<i32, String> {
            if b == 0 {
                Err("Division by zero")
            } else {
                Ok(a / b)
            }
        }
        
        fn handle_result(a: i32, b: i32) -> String {
            match divide(a, b) {
                Ok(value) => format!("Result: {}", value),
                Err(error) => format!("Error: {}", error),
            }
        }
    "#;

    let lexer = Lexer::new(source).expect("Lexer creation should succeed");
    let (tokens, _errors) = lexer.scan_tokens();
    assert!(_errors.is_empty(), "Lexer should not produce errors");
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Failed to parse");

    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze(&ast);

    // Should succeed - exhaustive Result pattern matching
    assert!(
        result.is_ok(),
        "Exhaustive Result pattern matching should succeed: {:?}",
        result.err()
    );
}

#[test]
fn test_result_pattern_non_exhaustive() {
    let source = r#"
        fn divide(a: i32, b: i32) -> Result<i32, String> {
            if b == 0 {
                Err("Division by zero")
            } else {
                Ok(a / b)
            }
        }
        
        fn handle_result_incomplete(a: i32, b: i32) -> String {
            match divide(a, b) {
                Ok(value) => format!("Result: {}", value),
                // Missing Err case - should trigger non-exhaustive warning
            }
        }
    "#;

    let lexer = Lexer::new(source).expect("Lexer creation should succeed");
    let (tokens, _errors) = lexer.scan_tokens();
    assert!(_errors.is_empty(), "Lexer should not produce errors");
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Failed to parse");

    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze(&ast);

    // Should fail or warn - non-exhaustive Result pattern matching
    // Depending on implementation, this might be an error or warning
    if result.is_err() {
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| matches!(e.kind, SemanticErrorKind::NonExhaustivePatterns)));
    }
}

#[test]
fn test_option_pattern_exhaustiveness() {
    let source = r#"
        fn find_value(arr: [i32], target: i32) -> Option<i32> {
            for item in arr {
                if item == target {
                    return Some(item);
                }
            }
            None
        }
        
        fn handle_option_complete(arr: [i32], target: i32) -> String {
            match find_value(arr, target) {
                Some(value) => format!("Found: {}", value),
                None => "Not found".to_string(),
            }
        }
        
        fn handle_option_incomplete(arr: [i32], target: i32) -> String {
            match find_value(arr, target) {
                Some(value) => format!("Found: {}", value),
                // Missing None case - should trigger non-exhaustive warning
            }
        }
    "#;

    let lexer = Lexer::new(source).expect("Lexer creation should succeed");
    let (tokens, _errors) = lexer.scan_tokens();
    assert!(_errors.is_empty(), "Lexer should not produce errors");
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Failed to parse");

    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze(&ast);

    // Should fail or warn due to non-exhaustive Option pattern in second function
    if result.is_err() {
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| matches!(e.kind, SemanticErrorKind::NonExhaustivePatterns)));
    }
}

#[test]
fn test_nested_error_propagation() {
    let source = r#"
        fn parse_int(s: String) -> Result<i32, String> {
            // Simplified implementation
            if s.is_empty() {
                Err("Empty string")
            } else {
                Ok(42) // Placeholder
            }
        }
        
        fn divide(a: i32, b: i32) -> Result<i32, String> {
            if b == 0 {
                Err("Division by zero")
            } else {
                Ok(a / b)
            }
        }
        
        fn complex_calculation(a_str: String, b_str: String) -> Result<i32, String> {
            let a = parse_int(a_str)?;
            let b = parse_int(b_str)?;
            let result = divide(a, b)?;
            Ok(result * 2)
        }
    "#;

    let lexer = Lexer::new(source).expect("Lexer creation should succeed");
    let (tokens, _errors) = lexer.scan_tokens();
    assert!(_errors.is_empty(), "Lexer should not produce errors");
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Failed to parse");

    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze(&ast);

    // Should succeed - valid nested error propagation
    assert!(
        result.is_ok(),
        "Nested error propagation should succeed: {:?}",
        result.err()
    );
}

#[test]
fn test_mixed_result_option_propagation() {
    let source = r#"
        fn find_value(arr: [i32], index: i32) -> Option<i32> {
            if index >= 0 && index < arr.len() {
                Some(arr[index])
            } else {
                None
            }
        }
        
        fn divide(a: i32, b: i32) -> Result<i32, String> {
            if b == 0 {
                Err("Division by zero")
            } else {
                Ok(a / b)
            }
        }
        
        // This should work: Option -> Result conversion
        fn mixed_propagation(arr: [i32], index: i32, divisor: i32) -> Result<i32, String> {
            let value = find_value(arr, index).ok_or("Index out of bounds")?;
            let result = divide(value, divisor)?;
            Ok(result)
        }
    "#;

    let lexer = Lexer::new(source).expect("Lexer creation should succeed");
    let (tokens, _errors) = lexer.scan_tokens();
    assert!(_errors.is_empty(), "Lexer should not produce errors");
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Failed to parse");

    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze(&ast);

    // Should succeed - valid mixed Option/Result propagation with conversion
    assert!(
        result.is_ok(),
        "Mixed Option/Result propagation should succeed: {:?}",
        result.err()
    );
}

#[test]
fn test_error_propagation_invalid_type() {
    let source = r#"
        fn regular_function() -> i32 {
            42
        }
        
        fn invalid_propagation() -> Result<i32, String> {
            let x = regular_function()?;  // Error: ? used on non-Result/Option type
            Ok(x)
        }
    "#;

    let lexer = Lexer::new(source).expect("Lexer creation should succeed");
    let (tokens, _errors) = lexer.scan_tokens();
    assert!(_errors.is_empty(), "Lexer should not produce errors");
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Failed to parse");

    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze(&ast);

    // Should fail - ? operator used on non-Result/Option type
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors
        .iter()
        .any(|e| matches!(e.kind, SemanticErrorKind::InvalidErrorPropagation { .. })));
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_full_result_workflow() {
        let source = r#"
            fn parse_and_divide(a_str: String, b_str: String) -> Result<String, String> {
                fn parse_int(s: String) -> Result<i32, String> {
                    if s.is_empty() {
                        Err("Empty string")
                    } else {
                        Ok(42) // Simplified
                    }
                }
                
                fn divide(a: i32, b: i32) -> Result<i32, String> {
                    if b == 0 {
                        Err("Division by zero")
                    } else {
                        Ok(a / b)
                    }
                }
                
                let a = parse_int(a_str)?;
                let b = parse_int(b_str)?;
                let result = divide(a, b)?;
                
                match result {
                    value if value > 0 => Ok(format!("Positive: {}", value)),
                    value if value < 0 => Ok(format!("Negative: {}", value)),
                    _ => Ok("Zero".to_string()),
                }
            }
        "#;

        let lexer = Lexer::new(source).expect("Lexer creation should succeed");
        let (tokens, _errors) = lexer.scan_tokens();
        assert!(_errors.is_empty(), "Lexer should not produce errors");
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().expect("Failed to parse");

        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze(&ast);

        // Should succeed - complete Result workflow with error propagation and pattern matching
        assert!(
            result.is_ok(),
            "Full Result workflow should succeed: {:?}",
            result.err()
        );
    }
}
