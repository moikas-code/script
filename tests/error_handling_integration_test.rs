//! Comprehensive integration tests for Script language error handling
//!
//! These tests verify that all error handling components work together correctly:
//! - Result/Option types and methods
//! - Error propagation operator (?)
//! - Custom error types
//! - File I/O error handling
//! - Pattern matching on error types

use script::compilation::CompilationContext;
use script::lexer::Lexer;
use script::parser::Parser;
use script::runtime::{Runtime, RuntimeError};
use script::semantic::SemanticAnalyzer;
use script::stdlib::{ScriptOption, ScriptResult, StdLib};
use script::types::Type;

/// Test basic Result and Option construction
#[test]
fn test_result_option_construction() {
    let code = r#"
        // Test Result construction
        let ok_result = Result::ok(42);
        let err_result = Result::err("Something went wrong");
        
        // Test Option construction  
        let some_value = Option::some(100);
        let none_value = Option::none();
        
        // Test type checking
        assert(is_ok(ok_result));
        assert(is_err(err_result));
        assert(is_some(some_value));
        assert(is_none(none_value));
    "#;

    let mut runtime = create_test_runtime();
    let result = compile_and_run(&mut runtime, code);
    assert!(
        result.is_ok(),
        "Basic Result/Option construction should succeed"
    );
}

/// Test comprehensive Result methods
#[test]
fn test_result_methods() {
    let code = r#"
        let ok_result = Result::ok(42);
        let err_result = Result::err("error");
        
        // Test unwrap_or
        let value1 = result_unwrap_or(ok_result, 0);
        let value2 = result_unwrap_or(err_result, 0);
        assert(value1 == 42);
        assert(value2 == 0);
        
        // Test or operation
        let backup = Result::ok(99);
        let combined = result_or(err_result, backup);
        assert(is_ok(combined));
        
        // Test expect with Ok
        let expected = result_expect(ok_result, "Should not fail");
        assert(expected == 42);
    "#;

    let mut runtime = create_test_runtime();
    let result = compile_and_run(&mut runtime, code);
    assert!(result.is_ok(), "Result methods should work correctly");
}

/// Test comprehensive Option methods
#[test]
fn test_option_methods() {
    let code = r#"
        let some_val = Option::some(42);
        let none_val = Option::none();
        
        // Test unwrap_or
        let value1 = option_unwrap_or(some_val, 0);
        let value2 = option_unwrap_or(none_val, 0);
        assert(value1 == 42);
        assert(value2 == 0);
        
        // Test or operation
        let backup = Option::some(99);
        let combined = option_or(none_val, backup);
        assert(is_some(combined));
        
        // Test ok_or conversion
        let result1 = option_ok_or(some_val, "No value");
        let result2 = option_ok_or(none_val, "No value");
        assert(is_ok(result1));
        assert(is_err(result2));
    "#;

    let mut runtime = create_test_runtime();
    let result = compile_and_run(&mut runtime, code);
    assert!(result.is_ok(), "Option methods should work correctly");
}

/// Test error propagation operator (?)
#[test]
fn test_error_propagation() {
    let code = r#"
        fn divide(a: i32, b: i32) -> Result<i32, string> {
            if b == 0 {
                return Result::err("Division by zero");
            }
            return Result::ok(a / b);
        }
        
        fn calculate() -> Result<i32, string> {
            let x = divide(10, 2)?; // Should return 5
            let y = divide(x, 1)?;  // Should return 5
            return Result::ok(y * 2); // Should return 10
        }
        
        fn calculate_with_error() -> Result<i32, string> {
            let x = divide(10, 0)?; // Should early return with error
            return Result::ok(x); // This should never execute
        }
        
        // Test successful propagation
        let result1 = calculate();
        assert(is_ok(result1));
        assert(result_unwrap_or(result1, 0) == 10);
        
        // Test error propagation
        let result2 = calculate_with_error();
        assert(is_err(result2));
    "#;

    let mut runtime = create_test_runtime();
    let result = compile_and_run(&mut runtime, code);
    assert!(result.is_ok(), "Error propagation should work correctly");
}

/// Test file I/O error handling
#[test]
fn test_file_io_error_handling() {
    let code = r#"
        // Test reading non-existent file
        let read_result = read_file("nonexistent_file.txt");
        assert(is_err(read_result));
        
        // Test writing and reading a file
        let write_result = write_file("test_output.txt", "Hello, Script!");
        assert(is_ok(write_result));
        
        let read_result2 = read_file("test_output.txt");
        assert(is_ok(read_result2));
        
        let content = result_unwrap_or(read_result2, "");
        assert(content == "Hello, Script!");
    "#;

    let mut runtime = create_test_runtime();
    let result = compile_and_run(&mut runtime, code);
    assert!(result.is_ok(), "File I/O error handling should work");

    // Cleanup
    let _ = std::fs::remove_file("test_output.txt");
}

/// Test pattern matching on Result and Option
#[test]
fn test_pattern_matching() {
    let code = r#"
        let ok_result = Result::ok(42);
        let err_result = Result::err("error");
        
        // Pattern match on Result
        let msg1 = match ok_result {
            Ok(value) => "Got value: " + value.to_string(),
            Err(error) => "Got error: " + error,
        };
        
        let msg2 = match err_result {
            Ok(value) => "Got value: " + value.to_string(),
            Err(error) => "Got error: " + error,
        };
        
        assert(msg1 == "Got value: 42");
        assert(msg2 == "Got error: error");
        
        // Pattern match on Option
        let some_opt = Option::some(100);
        let none_opt = Option::none();
        
        let val1 = match some_opt {
            Some(x) => x,
            None => 0,
        };
        
        let val2 = match none_opt {
            Some(x) => x,
            None => 0,
        };
        
        assert(val1 == 100);
        assert(val2 == 0);
    "#;

    let mut runtime = create_test_runtime();
    let result = compile_and_run(&mut runtime, code);
    assert!(result.is_ok(), "Pattern matching should work correctly");
}

/// Test chaining error handling operations
#[test]
fn test_error_handling_chains() {
    let code = r#"
        fn parse_number(s: string) -> Result<i32, string> {
            // Simplified parsing - just check if it's "42"
            if s == "42" {
                return Result::ok(42);
            } else {
                return Result::err("Not a valid number");
            }
        }
        
        fn process_numbers(inputs: [string]) -> Result<i32, string> {
            let mut sum = 0;
            for input in inputs {
                let num = parse_number(input)?;
                sum = sum + num;
            }
            return Result::ok(sum);
        }
        
        // Test successful chain
        let good_inputs = ["42", "42", "42"];
        let result1 = process_numbers(good_inputs);
        assert(is_ok(result1));
        assert(result_unwrap_or(result1, 0) == 126);
        
        // Test chain with error
        let bad_inputs = ["42", "not_a_number", "42"];
        let result2 = process_numbers(bad_inputs);
        assert(is_err(result2));
    "#;

    let mut runtime = create_test_runtime();
    let result = compile_and_run(&mut runtime, code);
    assert!(result.is_ok(), "Error handling chains should work");
}

/// Test Result and Option method chaining
#[test]
fn test_method_chaining() {
    let code = r#"
        let opt1 = Option::some(10);
        let opt2 = Option::none();
        
        // Chain Option operations
        let result1 = option_unwrap_or(
            option_or(opt2, opt1),
            0
        );
        assert(result1 == 10);
        
        // Chain Result operations
        let res1 = Result::ok(42);
        let res2 = Result::err("error");
        let backup = Result::ok(99);
        
        let final_result = result_unwrap_or(
            result_or(res2, backup),
            0
        );
        assert(final_result == 99);
    "#;

    let mut runtime = create_test_runtime();
    let result = compile_and_run(&mut runtime, code);
    assert!(result.is_ok(), "Method chaining should work correctly");
}

/// Test error handling with async operations (if implemented)
#[test]
#[ignore] // Enable when async error handling is implemented
fn test_async_error_handling() {
    let code = r#"
        async fn async_divide(a: i32, b: i32) -> Result<i32, string> {
            if b == 0 {
                return Result::err("Async division by zero");
            }
            return Result::ok(a / b);
        }
        
        async fn async_calculate() -> Result<i32, string> {
            let x = async_divide(10, 2).await?;
            let y = async_divide(x, 1).await?;
            return Result::ok(y * 2);
        }
        
        let result = async_calculate().await;
        assert(is_ok(result));
        assert(result_unwrap_or(result, 0) == 10);
    "#;

    let mut runtime = create_test_runtime();
    let result = compile_and_run(&mut runtime, code);
    assert!(result.is_ok(), "Async error handling should work");
}

/// Test custom error types
#[test]
fn test_custom_error_types() {
    let code = r#"
        // Test ValidationError
        let validation_err = ValidationError::new("Invalid input");
        assert(validation_err != null);
        
        // Test IoError  
        let io_err = IoError::new("NotFound", "File not found");
        assert(io_err != null);
    "#;

    let mut runtime = create_test_runtime();
    let result = compile_and_run(&mut runtime, code);
    assert!(result.is_ok(), "Custom error types should work");
}

/// Test exhaustive error handling patterns
#[test]
fn test_exhaustive_error_handling() {
    let code = r#"
        fn comprehensive_error_test(input: string) -> Result<string, string> {
            // Validate input
            if input == "" {
                return Result::err("Empty input not allowed");
            }
            
            // Try to process
            if input == "fail" {
                return Result::err("Processing failed");
            }
            
            // Success case
            return Result::ok("Processed: " + input);
        }
        
        // Test all cases
        let result1 = comprehensive_error_test("valid");
        let result2 = comprehensive_error_test("");
        let result3 = comprehensive_error_test("fail");
        
        assert(is_ok(result1));
        assert(is_err(result2));
        assert(is_err(result3));
        
        // Verify error messages
        let success_msg = result_unwrap_or(result1, "");
        assert(success_msg == "Processed: valid");
    "#;

    let mut runtime = create_test_runtime();
    let result = compile_and_run(&mut runtime, code);
    assert!(result.is_ok(), "Exhaustive error handling should work");
}

// Helper functions

fn create_test_runtime() -> Runtime {
    let mut runtime = Runtime::new();

    // Register standard library functions
    let stdlib = StdLib::new();
    for name in stdlib.function_names() {
        if let Some(func) = stdlib.get_function(name) {
            runtime.register_function(
                name.to_string(),
                func.signature.clone(),
                func.implementation,
            );
        }
    }

    runtime
}

fn compile_and_run(runtime: &mut Runtime, code: &str) -> Result<(), RuntimeError> {
    // Lexical analysis
    let lexer = Lexer::new(code.to_string())
        .map_err(|e| RuntimeError::InvalidOperation(format!("Lexer creation error: {:?}", e)))?;
    let (tokens, errors) = lexer.scan_tokens();
    if !errors.is_empty() {
        return Err(RuntimeError::InvalidOperation(format!(
            "Lexer errors: {:?}",
            errors
        )));
    }

    // Parsing
    let mut parser = Parser::new(tokens);
    let ast = parser
        .parse()
        .map_err(|e| RuntimeError::InvalidOperation(format!("Parser error: {:?}", e)))?;

    // Semantic analysis
    let mut analyzer = SemanticAnalyzer::new();
    let analyzed_ast = analyzer
        .analyze(ast)
        .map_err(|e| RuntimeError::InvalidOperation(format!("Semantic error: {:?}", e)))?;

    // Compilation
    let mut ctx = CompilationContext::new();
    let ir = ctx
        .compile_program(analyzed_ast)
        .map_err(|e| RuntimeError::InvalidOperation(format!("Compilation error: {:?}", e)))?;

    // Execution
    runtime
        .execute(ir)
        .map_err(|e| RuntimeError::InvalidOperation(format!("Runtime error: {:?}", e)))
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use quickcheck::TestResult;
    use quickcheck_macros::quickcheck;

    /// Property test: Result::ok followed by unwrap_or should always return the Ok value
    #[quickcheck]
    fn prop_result_ok_unwrap_or(value: i32, default: i32) -> TestResult {
        let code = format!(
            r#"
            let result = Result::ok({});
            let unwrapped = result_unwrap_or(result, {});
            assert(unwrapped == {});
            "#,
            value, default, value
        );

        let mut runtime = create_test_runtime();
        match compile_and_run(&mut runtime, &code) {
            Ok(_) => TestResult::passed(),
            Err(_) => TestResult::failed(),
        }
    }

    /// Property test: Result::err followed by unwrap_or should always return the default
    #[quickcheck]
    fn prop_result_err_unwrap_or(default: i32) -> TestResult {
        let code = format!(
            r#"
            let result = Result::err("error");
            let unwrapped = result_unwrap_or(result, {});
            assert(unwrapped == {});
            "#,
            default, default
        );

        let mut runtime = create_test_runtime();
        match compile_and_run(&mut runtime, &code) {
            Ok(_) => TestResult::passed(),
            Err(_) => TestResult::failed(),
        }
    }

    /// Property test: Option::some followed by unwrap_or should always return the Some value
    #[quickcheck]
    fn prop_option_some_unwrap_or(value: i32, default: i32) -> TestResult {
        let code = format!(
            r#"
            let option = Option::some({});
            let unwrapped = option_unwrap_or(option, {});
            assert(unwrapped == {});
            "#,
            value, default, value
        );

        let mut runtime = create_test_runtime();
        match compile_and_run(&mut runtime, &code) {
            Ok(_) => TestResult::passed(),
            Err(_) => TestResult::failed(),
        }
    }

    /// Property test: Option::none followed by unwrap_or should always return the default
    #[quickcheck]
    fn prop_option_none_unwrap_or(default: i32) -> TestResult {
        let code = format!(
            r#"
            let option = Option::none();
            let unwrapped = option_unwrap_or(option, {});
            assert(unwrapped == {});
            "#,
            default, default
        );

        let mut runtime = create_test_runtime();
        match compile_and_run(&mut runtime, &code) {
            Ok(_) => TestResult::passed(),
            Err(_) => TestResult::failed(),
        }
    }
}

/// Benchmark tests for error handling performance
#[cfg(test)]
mod benchmarks {
    use super::*;
    use std::time::Instant;

    #[test]
    #[ignore] // Run manually for performance testing
    fn bench_result_operations() {
        let code = r#"
            let mut total = 0;
            for i in 0..1000 {
                let result = if i % 2 == 0 { Result::ok(i) } else { Result::err("odd") };
                total = total + result_unwrap_or(result, 0);
            }
            assert(total > 0);
        "#;

        let mut runtime = create_test_runtime();
        let start = Instant::now();
        let result = compile_and_run(&mut runtime, code);
        let duration = start.elapsed();

        assert!(result.is_ok());
        println!("Result operations benchmark: {:?}", duration);
    }

    #[test]
    #[ignore] // Run manually for performance testing
    fn bench_option_operations() {
        let code = r#"
            let mut total = 0;
            for i in 0..1000 {
                let option = if i % 3 == 0 { Option::some(i) } else { Option::none() };
                total = total + option_unwrap_or(option, 0);
            }
            assert(total > 0);
        "#;

        let mut runtime = create_test_runtime();
        let start = Instant::now();
        let result = compile_and_run(&mut runtime, code);
        let duration = start.elapsed();

        assert!(result.is_ok());
        println!("Option operations benchmark: {:?}", duration);
    }

    #[test]
    #[ignore] // Run manually for performance testing
    fn bench_error_propagation() {
        let code = r#"
            fn chain_operations(count: i32) -> Result<i32, string> {
                let mut sum = 0;
                for i in 0..count {
                    let result = if i == 999 { Result::err("error") } else { Result::ok(i) };
                    sum = sum + result?;
                }
                return Result::ok(sum);
            }
            
            let result = chain_operations(500);
            assert(is_ok(result));
        "#;

        let mut runtime = create_test_runtime();
        let start = Instant::now();
        let result = compile_and_run(&mut runtime, code);
        let duration = start.elapsed();

        assert!(result.is_ok());
        println!("Error propagation benchmark: {:?}", duration);
    }
}
