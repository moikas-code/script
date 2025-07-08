//! Tests for Result/Option error handling functionality

use script::{Lexer, Parser, SemanticAnalyzer};
use script::types::Type;

fn compile_and_analyze(source: &str) -> Result<Vec<Type>, Vec<script::Error>> {
    let lexer = Lexer::new(source).unwrap();
    let (tokens, lexer_errors) = lexer.scan_tokens();
    
    if !lexer_errors.is_empty() {
        return Err(lexer_errors);
    }
    
    let mut parser = Parser::new(tokens);
    let program = match parser.parse() {
        Ok(prog) => prog,
        Err(e) => return Err(vec![e]),
    };
    
    let mut analyzer = SemanticAnalyzer::new();
    if let Err(e) = analyzer.analyze(&program) {
        return Err(vec![e]);
    }
    
    let errors = analyzer.errors();
    if !errors.is_empty() {
        return Err(errors.into_iter().map(|e| e.into_error()).collect());
    }
    
    Ok(vec![])
}

#[test]
fn test_option_constructors() {
    let source = r#"
        let x = Some(42);
        let y = None;
        let z = Option::Some(3.14);
    "#;
    
    let result = compile_and_analyze(source);
    assert!(result.is_ok(), "Failed to compile Option constructors: {:?}", result);
}

#[test]
fn test_result_constructors() {
    let source = r#"
        let x = Ok(42);
        let y = Err("error");
        let z = Result::Ok("success");
    "#;
    
    let result = compile_and_analyze(source);
    assert!(result.is_ok(), "Failed to compile Result constructors: {:?}", result);
}

#[test]
fn test_question_operator_on_result() {
    let source = r#"
        fn may_fail() -> Result<i32, String> {
            Ok(42)
        }
        
        fn caller() -> Result<i32, String> {
            let x = may_fail()?;
            Ok(x + 1)
        }
    "#;
    
    let result = compile_and_analyze(source);
    assert!(result.is_ok(), "Failed to compile ? operator on Result: {:?}", result);
}

#[test]
fn test_question_operator_on_option() {
    let source = r#"
        fn may_be_none() -> Option<i32> {
            Some(42)
        }
        
        fn caller() -> Option<i32> {
            let x = may_be_none()?;
            Some(x + 1)
        }
    "#;
    
    let result = compile_and_analyze(source);
    assert!(result.is_ok(), "Failed to compile ? operator on Option: {:?}", result);
}

#[test]
fn test_question_operator_error_propagation() {
    let source = r#"
        fn may_fail() -> Result<i32, String> {
            Err("something went wrong")
        }
        
        fn caller() -> Result<i32, String> {
            let x = may_fail()?;  // This should propagate the error
            Ok(x + 1)
        }
    "#;
    
    let result = compile_and_analyze(source);
    assert!(result.is_ok(), "Failed to compile error propagation: {:?}", result);
}

#[test]
fn test_question_operator_in_non_result_function() {
    let source = r#"
        fn may_fail() -> Result<i32, String> {
            Ok(42)
        }
        
        fn caller() -> i32 {
            let x = may_fail()?;  // Error: ? in non-Result function
            x + 1
        }
    "#;
    
    let result = compile_and_analyze(source);
    assert!(result.is_err(), "Should fail when using ? in non-Result function");
    
    if let Err(errors) = result {
        assert!(errors.iter().any(|e| {
            e.to_string().contains("? operator can only be used in functions that return Result or Option")
        }));
    }
}

#[test]
fn test_question_operator_on_non_result_type() {
    let source = r#"
        fn caller() -> Result<i32, String> {
            let x = 42;
            let y = x?;  // Error: ? on non-Result/Option type
            Ok(y)
        }
    "#;
    
    let result = compile_and_analyze(source);
    assert!(result.is_err(), "Should fail when using ? on non-Result type");
    
    if let Err(errors) = result {
        assert!(errors.iter().any(|e| {
            e.to_string().contains("? operator can only be applied to Result or Option types")
        }));
    }
}

#[test]
fn test_pattern_matching_on_result() {
    let source = r#"
        fn handle_result(r: Result<i32, String>) -> i32 {
            match r {
                Ok(value) => value,
                Err(_) => -1
            }
        }
    "#;
    
    let result = compile_and_analyze(source);
    assert!(result.is_ok(), "Failed to compile pattern matching on Result: {:?}", result);
}

#[test]
fn test_pattern_matching_on_option() {
    let source = r#"
        fn handle_option(opt: Option<i32>) -> i32 {
            match opt {
                Some(value) => value,
                None => 0
            }
        }
    "#;
    
    let result = compile_and_analyze(source);
    assert!(result.is_ok(), "Failed to compile pattern matching on Option: {:?}", result);
}

#[test]
fn test_nested_error_propagation() {
    let source = r#"
        fn inner() -> Result<i32, String> {
            Err("inner error")
        }
        
        fn middle() -> Result<i32, String> {
            let x = inner()?;
            Ok(x * 2)
        }
        
        fn outer() -> Result<i32, String> {
            let y = middle()?;
            Ok(y + 10)
        }
    "#;
    
    let result = compile_and_analyze(source);
    assert!(result.is_ok(), "Failed to compile nested error propagation: {:?}", result);
}

#[test]
fn test_option_to_result_conversion() {
    let source = r#"
        fn get_option() -> Option<i32> {
            Some(42)
        }
        
        fn convert() -> Result<i32, String> {
            let x = get_option()?;  // Option ? in Result function (should work)
            Ok(x)
        }
    "#;
    
    let result = compile_and_analyze(source);
    assert!(result.is_ok(), "Failed to compile Option to Result conversion: {:?}", result);
}