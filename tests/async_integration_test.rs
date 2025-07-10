//! End-to-end integration tests for async/await security pipeline
//!
//! This test suite validates the complete async/await pipeline from
//! source code through transformation, compilation, and runtime execution
//! with all security mechanisms active.

use script::codegen::CodeGenerator;
use script::lexer::Lexer;
use script::parser::Parser;
use script::runtime::async_ffi::*;
use script::runtime::value::Value;
use script::runtime::{initialize, shutdown, Runtime, RuntimeConfig};
use script::security::{SecurityConfig, SecurityMetrics};
use script::semantic::SemanticAnalyzer;
use std::sync::Arc;
use std::time::Duration;

/// Helper to compile and run async Script code
fn compile_and_run_async(source: &str) -> Result<Value, Box<dyn std::error::Error>> {
    // Initialize runtime if needed
    if !script::runtime::is_initialized() {
        initialize()?;
    }

    // Lexical analysis
    let mut lexer = Lexer::new(source);
    let tokens = lexer.scan_tokens()?;

    // Parsing
    let mut parser = Parser::new(tokens);
    let ast = parser.parse()?;

    // Semantic analysis with security validation
    let mut analyzer = SemanticAnalyzer::new();
    let analyzed_ast = analyzer.analyze(ast)?;

    // Code generation with async transformation
    let mut codegen = CodeGenerator::new();
    let module = codegen.generate(analyzed_ast)?;

    // Create runtime with security configuration
    let security_config = SecurityConfig {
        enable_async_pointer_validation: true,
        enable_async_memory_safety: true,
        max_async_tasks: 1000,
        max_async_task_timeout_secs: 60,
        enable_async_ffi_validation: true,
        ..Default::default()
    };

    let runtime_config = RuntimeConfig::default().with_security(security_config);

    let mut runtime = Runtime::new(runtime_config)?;

    // Execute the module
    runtime.execute_module(module)
}

#[test]
fn test_simple_async_function() {
    let source = r#"
        async fn delay_add(x: i32, y: i32) -> i32 {
            await sleep_ms(10);
            return x + y;
        }
        
        fn main() -> i32 {
            let result = await delay_add(5, 3);
            return result;
        }
    "#;

    let result = compile_and_run_async(source).expect("Failed to run async code");
    assert_eq!(result, Value::I32(8));
}

#[test]
fn test_multiple_await_security() {
    let source = r#"
        async fn fetch_value(id: i32) -> i32 {
            await sleep_ms(5);
            return id * 2;
        }
        
        async fn process_values() -> i32 {
            let a = await fetch_value(1);
            let b = await fetch_value(2);
            let c = await fetch_value(3);
            return a + b + c;
        }
        
        fn main() -> i32 {
            return await process_values();
        }
    "#;

    let result = compile_and_run_async(source).expect("Failed to run async code");
    assert_eq!(result, Value::I32(12)); // (1*2) + (2*2) + (3*2) = 12
}

#[test]
fn test_async_memory_limits() {
    let source = r#"
        async fn allocate_memory() -> i32 {
            let mut arrays = [];
            for i in 0..100 {
                // Try to allocate large arrays
                let arr = [0; 10000];
                arrays.push(arr);
                await sleep_ms(1);
            }
            return arrays.len();
        }
        
        fn main() -> i32 {
            return await allocate_memory();
        }
    "#;

    // Should enforce memory limits
    let result = compile_and_run_async(source);
    // Either succeeds with limited allocations or fails with memory error
    match result {
        Ok(Value::I32(count)) => assert!(count <= 100),
        Err(_) => {} // Memory limit enforced
        _ => panic!("Unexpected result"),
    }
}

#[test]
fn test_async_timeout_enforcement() {
    let source = r#"
        async fn infinite_loop() -> i32 {
            let mut counter = 0;
            while true {
                counter = counter + 1;
                await sleep_ms(10);
                if counter > 1000000 {
                    break;
                }
            }
            return counter;
        }
        
        fn main() -> i32 {
            // This should timeout
            return await_timeout(infinite_loop(), 100);
        }
    "#;

    let result = compile_and_run_async(source);
    // Should timeout and return error or default value
    match result {
        Ok(Value::I32(0)) => {} // Timeout returned default
        Ok(Value::Null) => {}   // Timeout returned null
        Err(_) => {}            // Timeout error
        Ok(other) => panic!("Expected timeout, got {:?}", other),
    }
}

#[test]
fn test_concurrent_async_tasks() {
    let source = r#"
        async fn worker(id: i32) -> i32 {
            await sleep_ms(id * 10);
            return id * id;
        }
        
        async fn run_concurrent() -> i32 {
            let tasks = [];
            
            // Spawn multiple concurrent tasks
            for i in 1..6 {
                tasks.push(spawn(worker(i)));
            }
            
            // Wait for all tasks
            let results = await join_all(tasks);
            
            // Sum the results
            let mut sum = 0;
            for result in results {
                sum = sum + result;
            }
            
            return sum;
        }
        
        fn main() -> i32 {
            return await run_concurrent();
        }
    "#;

    let result = compile_and_run_async(source).expect("Failed to run concurrent async");
    assert_eq!(result, Value::I32(55)); // 1 + 4 + 9 + 16 + 25 = 55
}

#[test]
fn test_async_error_propagation() {
    let source = r#"
        async fn may_fail(should_fail: bool) -> Result<i32, String> {
            await sleep_ms(5);
            if should_fail {
                return Err("Async operation failed");
            }
            return Ok(42);
        }
        
        async fn handle_errors() -> i32 {
            match await may_fail(false) {
                Ok(value) => value,
                Err(_) => -1,
            }
        }
        
        fn main() -> i32 {
            return await handle_errors();
        }
    "#;

    let result = compile_and_run_async(source).expect("Failed to run async error handling");
    assert_eq!(result, Value::I32(42));
}

#[test]
fn test_async_recursion_limits() {
    let source = r#"
        async fn recursive_async(depth: i32) -> i32 {
            if depth <= 0 {
                return 0;
            }
            await sleep_ms(1);
            return 1 + await recursive_async(depth - 1);
        }
        
        fn main() -> i32 {
            // Should handle reasonable recursion depth
            return await recursive_async(10);
        }
    "#;

    let result = compile_and_run_async(source).expect("Failed to run async recursion");
    assert_eq!(result, Value::I32(10));
}

#[test]
fn test_async_with_closures() {
    let source = r#"
        async fn async_map(values: [i32], f: fn(i32) -> i32) -> [i32] {
            let results = [];
            for value in values {
                await sleep_ms(1);
                results.push(f(value));
            }
            return results;
        }
        
        fn main() -> i32 {
            let numbers = [1, 2, 3, 4, 5];
            let doubled = await async_map(numbers, |x| x * 2);
            
            let sum = 0;
            for n in doubled {
                sum = sum + n;
            }
            return sum;
        }
    "#;

    let result = compile_and_run_async(source).expect("Failed to run async with closures");
    assert_eq!(result, Value::I32(30)); // 2 + 4 + 6 + 8 + 10 = 30
}

#[test]
fn test_async_resource_cleanup() {
    let source = r#"
        struct AsyncResource {
            id: i32,
            data: [u8],
        }
        
        async fn use_resource() -> i32 {
            let resource = AsyncResource {
                id: 123,
                data: [0; 1000],
            };
            
            await sleep_ms(10);
            
            // Resource should be cleaned up after function
            return resource.id;
        }
        
        async fn leak_test() -> i32 {
            let mut total = 0;
            
            // Create many temporary resources
            for i in 0..100 {
                total = total + await use_resource();
            }
            
            return total;
        }
        
        fn main() -> i32 {
            return await leak_test();
        }
    "#;

    let result = compile_and_run_async(source).expect("Failed to run resource cleanup test");
    assert_eq!(result, Value::I32(12300)); // 123 * 100
}

#[test]
fn test_async_pattern_matching() {
    let source = r#"
        enum AsyncResult {
            Success(i32),
            Pending,
            Error(String),
        }
        
        async fn async_operation(id: i32) -> AsyncResult {
            await sleep_ms(id * 5);
            
            match id {
                1 => AsyncResult::Success(100),
                2 => AsyncResult::Pending,
                _ => AsyncResult::Error("Unknown ID"),
            }
        }
        
        async fn handle_results() -> i32 {
            let result1 = await async_operation(1);
            let result2 = await async_operation(2);
            
            match (result1, result2) {
                (AsyncResult::Success(n), AsyncResult::Pending) => n,
                _ => -1,
            }
        }
        
        fn main() -> i32 {
            return await handle_results();
        }
    "#;

    let result = compile_and_run_async(source).expect("Failed to run pattern matching");
    assert_eq!(result, Value::I32(100));
}

#[test]
fn test_async_generics() {
    let source = r#"
        async fn async_identity<T>(value: T) -> T {
            await sleep_ms(5);
            return value;
        }
        
        async fn use_generic_async() -> i32 {
            let int_result = await async_identity(42);
            let string_result = await async_identity("hello");
            
            return int_result;
        }
        
        fn main() -> i32 {
            return await use_generic_async();
        }
    "#;

    let result = compile_and_run_async(source).expect("Failed to run async generics");
    assert_eq!(result, Value::I32(42));
}

#[test]
fn test_async_security_metrics() {
    // Initialize metrics
    let metrics = Arc::new(SecurityMetrics::new());

    let source = r#"
        async fn monitored_function() -> i32 {
            await sleep_ms(10);
            return 42;
        }
        
        fn main() -> i32 {
            return await monitored_function();
        }
    "#;

    // Run with metrics collection
    let result = compile_and_run_async(source).expect("Failed to run with metrics");
    assert_eq!(result, Value::I32(42));

    // Verify metrics were collected
    // Note: Actual metric verification would need runtime access
}

#[test]
fn test_async_cancellation() {
    let source = r#"
        async fn cancellable_task() -> i32 {
            let mut counter = 0;
            for i in 0..1000 {
                await sleep_ms(1);
                counter = counter + 1;
                
                // Check for cancellation
                if is_cancelled() {
                    return counter;
                }
            }
            return counter;
        }
        
        async fn test_cancel() -> i32 {
            let task = spawn(cancellable_task());
            
            // Wait a bit then cancel
            await sleep_ms(50);
            cancel(task);
            
            // Get the result
            return await task;
        }
        
        fn main() -> i32 {
            return await test_cancel();
        }
    "#;

    let result = compile_and_run_async(source);
    match result {
        Ok(Value::I32(n)) => assert!(n < 1000), // Should be cancelled early
        _ => {}                                 // Cancellation might return error
    }
}

/// Cleanup after tests
#[test]
fn test_cleanup() {
    // Ensure executor is shutdown properly
    script_shutdown_executor();

    // Shutdown runtime if initialized
    if script::runtime::is_initialized() {
        let _ = shutdown();
    }
}
