//! Tests for closure execution in code generation

use script::codegen::{CodegenBackend, cranelift::CraneliftBackend};
use script::ir::{IrBuilder, Parameter, BinaryOp};
use script::types::Type;

#[test]
fn test_simple_closure_execution() {
    // Create a simple closure that adds 1 to its argument
    let mut builder = IrBuilder::new();
    
    // Create the closure function body
    let add_one = builder.create_function(
        "add_one_closure".to_string(),
        vec![Parameter { name: "x".to_string(), ty: Type::I32 }],
        Type::I32
    );
    builder.set_current_function(add_one);
    
    // Get parameter and add 1
    let param = script::ir::ValueId(1000); // First parameter
    let one = builder.const_value(script::ir::Constant::I32(1));
    let result = builder.build_binary(BinaryOp::Add, param, one, Type::I32).unwrap();
    builder.build_return(Some(result));
    
    // Create main function that creates and invokes the closure
    let main = builder.create_function(
        "main".to_string(),
        vec![],
        Type::I32
    );
    builder.set_current_function(main);
    
    // Create closure
    let closure = builder.build_create_closure(
        "add_one_closure".to_string(),
        vec!["x".to_string()],
        vec![],
        false
    ).unwrap();
    
    // Invoke closure with argument 41
    let arg = builder.const_value(script::ir::Constant::I32(41));
    let result = builder.build_invoke_closure(closure, vec![arg], Type::I32).unwrap();
    builder.build_return(Some(result));
    
    // Build the IR module
    let module = builder.build();
    
    // Generate code
    let mut backend = CraneliftBackend::new();
    let executable = backend.generate(&module).expect("Code generation failed");
    
    // Execute and verify result
    match executable.execute() {
        Ok(result) => {
            assert_eq!(result, 42, "Closure should return 42");
        }
        Err(e) => panic!("Execution failed: {:?}", e),
    }
}

#[test]
fn test_closure_with_captures() {
    // Create a closure that captures a value
    let mut builder = IrBuilder::new();
    
    // Create main function
    let main = builder.create_function(
        "main".to_string(),
        vec![],
        Type::I32
    );
    builder.set_current_function(main);
    
    // Create a value to capture
    let captured_value = builder.const_value(script::ir::Constant::I32(10));
    
    // Create closure that adds captured value to its argument
    let closure = builder.build_create_closure(
        "add_captured".to_string(),
        vec!["x".to_string()],
        vec![("captured".to_string(), captured_value)],
        false
    ).unwrap();
    
    // Invoke closure with argument 32
    let arg = builder.const_value(script::ir::Constant::I32(32));
    let result = builder.build_invoke_closure(closure, vec![arg], Type::I32).unwrap();
    builder.build_return(Some(result));
    
    // Build and execute
    let module = builder.build();
    let mut backend = CraneliftBackend::new();
    let executable = backend.generate(&module).expect("Code generation failed");
    
    match executable.execute() {
        Ok(result) => {
            assert_eq!(result, 42, "Closure should return 42 (32 + 10)");
        }
        Err(e) => panic!("Execution failed: {:?}", e),
    }
}

#[test]
fn test_closure_fast_path() {
    // Test that fast-path optimization works for small closures
    let mut builder = IrBuilder::new();
    
    // Create a closure function that multiplies two arguments
    let multiply = builder.create_function(
        "multiply".to_string(),
        vec![
            Parameter { name: "x".to_string(), ty: Type::I32 },
            Parameter { name: "y".to_string(), ty: Type::I32 }
        ],
        Type::I32
    );
    builder.set_current_function(multiply);
    
    let x = script::ir::ValueId(1000); // First parameter
    let y = script::ir::ValueId(1001); // Second parameter
    let result = builder.build_binary(BinaryOp::Mul, x, y, Type::I32).unwrap();
    builder.build_return(Some(result));
    
    // Create main function
    let main = builder.create_function(
        "main".to_string(),
        vec![],
        Type::I32
    );
    builder.set_current_function(main);
    
    // Create closure
    let closure = builder.build_create_closure(
        "multiply".to_string(),
        vec!["x".to_string(), "y".to_string()],
        vec![],
        false
    ).unwrap();
    
    // Invoke with 2 arguments (should use fast path)
    let arg1 = builder.const_value(script::ir::Constant::I32(6));
    let arg2 = builder.const_value(script::ir::Constant::I32(7));
    let result = builder.build_invoke_closure(closure, vec![arg1, arg2], Type::I32).unwrap();
    builder.build_return(Some(result));
    
    // Build and execute
    let module = builder.build();
    let mut backend = CraneliftBackend::new();
    let executable = backend.generate(&module).expect("Code generation failed");
    
    match executable.execute() {
        Ok(result) => {
            assert_eq!(result, 42, "Closure should return 42 (6 * 7)");
        }
        Err(e) => panic!("Execution failed: {:?}", e),
    }
}