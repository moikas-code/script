//! Security tests for async function transformation
//!
//! This test suite validates that the async transformation process
//! properly enforces security constraints and generates safe code.

use script::error::{Error, ErrorKind};
use script::ir::{Function, FunctionId, Module, Parameter, Type, Visibility};
use script::lowering::async_transform::{find_await_expressions, transform_async_function};
use script::parser::{Expr, Location, Stmt};
use std::collections::HashMap;

fn create_test_module() -> Module {
    Module::new("test_module".to_string())
}

fn create_async_function(name: &str, body_size: usize) -> Function {
    let mut func = Function {
        id: FunctionId(0),
        name: name.to_string(),
        params: vec![Parameter {
            name: "x".to_string(),
            ty: Type::Named("i32".to_string()),
        }],
        return_type: Type::Named("i32".to_string()),
        is_async: true,
        is_generator: false,
        is_extern: false,
        visibility: Visibility::Private,
        blocks: HashMap::new(),
        entry_block: None,
        value_counter: 0,
        block_counter: 0,
    };

    // Create entry block with instructions
    let entry = func.create_block("entry".to_string());
    func.entry_block = Some(entry);

    // Add instructions to simulate function body
    for i in 0..body_size {
        func.get_block_mut(entry).unwrap().add_instruction(
            script::ir::Instruction::Const(script::ir::Constant::I32(i as i32)),
            Location { line: i, column: 0 },
        );
    }

    func
}

#[test]
fn test_async_transform_security_validation() {
    let mut module = create_test_module();

    // Test 1: Normal async function should pass
    let normal_func = create_async_function("normal_async", 100);
    let func_id = module.add_function(normal_func);

    let result = transform_async_function(&mut module, func_id);
    assert!(result.is_ok());

    // Test 2: Function with too many instructions should fail
    let large_func = create_async_function("large_async", 20_000);
    let large_func_id = module.add_function(large_func);

    let result = transform_async_function(&mut module, large_func_id);
    assert!(result.is_err());
    if let Err(e) = result {
        assert_eq!(e.kind(), &ErrorKind::SecurityViolation);
    }
}

#[test]
fn test_async_transform_await_limit() {
    let mut module = create_test_module();

    // Create function with many await points
    let mut func = create_async_function("many_awaits", 10);
    let entry = func.entry_block.unwrap();

    // Add many PollFuture instructions (await points)
    for i in 0..150 {
        func.get_block_mut(entry).unwrap().add_instruction(
            script::ir::Instruction::PollFuture {
                future: script::ir::ValueId(i),
                output_ty: Type::Named("i32".to_string()),
            },
            Location {
                line: i as usize,
                column: 0,
            },
        );
    }

    let func_id = module.add_function(func);
    let result = transform_async_function(&mut module, func_id);

    // Should fail due to too many await points
    assert!(result.is_err());
    if let Err(e) = result {
        assert_eq!(e.kind(), &ErrorKind::SecurityViolation);
        assert!(e.to_string().contains("await points"));
    }
}

#[test]
fn test_find_await_expressions() {
    use script::parser::{BinaryOp, UnaryOp};

    // Test 1: No await expressions
    let stmts = vec![Stmt::Expression {
        expr: Expr::Literal {
            value: script::parser::Literal::I32(42),
            location: Location { line: 1, column: 0 },
        },
        location: Location { line: 1, column: 0 },
    }];

    let awaits = find_await_expressions(&stmts);
    assert_eq!(awaits.len(), 0);

    // Test 2: Single await expression
    let stmts = vec![Stmt::Expression {
        expr: Expr::Await {
            expr: Box::new(Expr::Call {
                callee: Box::new(Expr::Identifier {
                    name: "async_func".to_string(),
                    location: Location { line: 2, column: 0 },
                }),
                args: vec![],
                location: Location { line: 2, column: 0 },
            }),
            location: Location { line: 2, column: 0 },
        },
        location: Location { line: 2, column: 0 },
    }];

    let awaits = find_await_expressions(&stmts);
    assert_eq!(awaits.len(), 1);
    assert_eq!(awaits[0], 0);

    // Test 3: Nested await expressions
    let stmts = vec![Stmt::If {
        condition: Expr::Bool {
            value: true,
            location: Location { line: 3, column: 0 },
        },
        then_branch: vec![Stmt::Expression {
            expr: Expr::Await {
                expr: Box::new(Expr::Identifier {
                    name: "future1".to_string(),
                    location: Location { line: 4, column: 0 },
                }),
                location: Location { line: 4, column: 0 },
            },
            location: Location { line: 4, column: 0 },
        }],
        else_branch: Some(vec![Stmt::Expression {
            expr: Expr::Await {
                expr: Box::new(Expr::Identifier {
                    name: "future2".to_string(),
                    location: Location { line: 5, column: 0 },
                }),
                location: Location { line: 5, column: 0 },
            },
            location: Location { line: 5, column: 0 },
        }]),
        location: Location { line: 3, column: 0 },
    }];

    let awaits = find_await_expressions(&stmts);
    assert_eq!(awaits.len(), 2);
}

#[test]
fn test_async_state_size_calculation() {
    let mut module = create_test_module();

    // Create function with various types of locals
    let mut func = create_async_function("state_size_test", 5);
    let entry = func.entry_block.unwrap();
    let block = func.get_block_mut(entry).unwrap();

    // Add different types of locals
    block.add_instruction(
        script::ir::Instruction::Alloc {
            ty: Type::Named("i32".to_string()),
        },
        Location { line: 1, column: 0 },
    );

    block.add_instruction(
        script::ir::Instruction::Alloc {
            ty: Type::Named("i64".to_string()),
        },
        Location { line: 2, column: 0 },
    );

    block.add_instruction(
        script::ir::Instruction::Alloc {
            ty: Type::Named("bool".to_string()),
        },
        Location { line: 3, column: 0 },
    );

    // Add await points
    block.add_instruction(
        script::ir::Instruction::PollFuture {
            future: script::ir::ValueId(4),
            output_ty: Type::Named("i32".to_string()),
        },
        Location { line: 4, column: 0 },
    );

    let func_id = module.add_function(func);
    let result = transform_async_function(&mut module, func_id);

    assert!(result.is_ok());
    let transform_info = result.unwrap();

    // Check state size calculation
    // Should include: state enum (8) + result (8) + waker (8) + params + locals + futures
    assert!(transform_info.state_size >= 24); // Minimum for control vars
    assert!(transform_info.state_size < 1000); // Reasonable upper bound

    // Check that variables are properly mapped
    assert!(transform_info.state_offsets.contains_key("int_var"));
    assert!(transform_info.state_offsets.contains_key("long_var"));
    assert!(transform_info.state_offsets.contains_key("bool_var"));
}

#[test]
fn test_async_transform_memory_safety() {
    let mut module = create_test_module();

    // Create function that allocates arrays
    let mut func = create_async_function("array_test", 5);
    let entry = func.entry_block.unwrap();
    let block = func.get_block_mut(entry).unwrap();

    // Add array creation using alloc for array pointer
    block.add_instruction(
        script::ir::Instruction::Alloc {
            ty: Type::Array(Box::new(Type::Named("i32".to_string()))),
        },
        Location { line: 1, column: 0 },
    );

    // Add struct creation using proper construct instruction
    block.add_instruction(
        script::ir::Instruction::ConstructStruct {
            struct_name: "TestStruct".to_string(),
            fields: vec![
                ("field1".to_string(), script::ir::ValueId(4)),
                ("field2".to_string(), script::ir::ValueId(5)),
            ],
            ty: Type::Named("TestStruct".to_string()),
        },
        Location { line: 2, column: 0 },
    );

    let func_id = module.add_function(func);
    let result = transform_async_function(&mut module, func_id);

    assert!(result.is_ok());
    let transform_info = result.unwrap();

    // Should have allocated space for temporaries
    assert!(transform_info.state_size > 50); // Should include space for array/struct temps
}

#[test]
fn test_async_transform_recursion_detection() {
    let mut module = create_test_module();

    // Create self-referential function (simplified - real detection would be more complex)
    let mut func = create_async_function("recursive_async", 5);
    let entry = func.entry_block.unwrap();
    let block = func.get_block_mut(entry).unwrap();

    // Add a call instruction that could be recursive
    block.add_instruction(
        script::ir::Instruction::Call {
            func: script::ir::FunctionId(0), // Would need resolution in real impl
            args: vec![script::ir::ValueId(1)],
            ty: Type::Named("i32".to_string()),
        },
        Location { line: 1, column: 0 },
    );

    let func_id = module.add_function(func);
    let result = transform_async_function(&mut module, func_id);

    // Should complete successfully (basic recursion check passes)
    assert!(result.is_ok());
}

#[test]
fn test_async_transform_poll_function_generation() {
    let mut module = create_test_module();

    // Create simple async function
    let func = create_async_function("simple_async", 3);
    let func_id = module.add_function(func);

    let result = transform_async_function(&mut module, func_id);
    assert!(result.is_ok());

    let transform_info = result.unwrap();

    // Check that poll function was created
    let poll_func = module.get_function(transform_info.poll_fn);
    assert!(poll_func.is_some());

    let poll_func = poll_func.unwrap();
    assert!(poll_func.name.contains("_poll"));
    assert_eq!(poll_func.params.len(), 2); // self and waker

    // Check parameter types
    assert_eq!(poll_func.params[0].name, "self");
    assert!(matches!(poll_func.params[0].ty, Type::Named(ref name) if name.contains("AsyncState")));

    assert_eq!(poll_func.params[1].name, "waker");
    assert_eq!(poll_func.params[1].ty, Type::Named("Waker".to_string()));

    // Check return type is Poll<T>
    assert!(matches!(poll_func.return_type, Type::Generic { ref name, .. } if name == "Poll"));
}

#[test]
fn test_async_transform_error_handling() {
    let mut module = create_test_module();

    // Test 1: Non-async function should fail
    let mut non_async = create_async_function("not_async", 5);
    non_async.is_async = false;
    let func_id = module.add_function(non_async);

    let result = transform_async_function(&mut module, func_id);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), &ErrorKind::RuntimeError);

    // Test 2: Invalid function ID should fail
    let invalid_id = FunctionId(9999);
    let result = transform_async_function(&mut module, invalid_id);
    assert!(result.is_err());
}

#[test]
fn test_async_transform_suspend_points() {
    let mut module = create_test_module();

    // Create function with multiple await points
    let mut func = create_async_function("multi_await", 5);
    let entry = func.entry_block.unwrap();
    let block = func.get_block_mut(entry).unwrap();

    // Add multiple await points
    for i in 0..3 {
        block.add_instruction(
            script::ir::Instruction::PollFuture {
                future: script::ir::ValueId(i * 2),
                output_ty: Type::Named("i32".to_string()),
            },
            Location {
                line: i as usize,
                column: 0,
            },
        );
    }

    let func_id = module.add_function(func);
    let result = transform_async_function(&mut module, func_id);

    assert!(result.is_ok());
    let transform_info = result.unwrap();

    // Should have 3 suspend points
    assert_eq!(transform_info.suspend_points.len(), 3);

    // Each suspend point should have unique state ID
    let state_ids: Vec<u32> = transform_info
        .suspend_points
        .iter()
        .map(|sp| sp.state_id)
        .collect();

    // Check state IDs are unique and sequential
    for (i, &state_id) in state_ids.iter().enumerate() {
        assert_eq!(state_id, (i + 1) as u32);
    }
}
