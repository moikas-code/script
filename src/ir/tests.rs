use super::*;
use crate::types::Type;

#[test]
fn test_ir_builder_function_creation() {
    let mut builder = IrBuilder::new();
    
    let params = vec![
        Parameter { name: "x".to_string(), ty: Type::I32 },
        Parameter { name: "y".to_string(), ty: Type::I32 },
    ];
    
    let func_id = builder.create_function("add".to_string(), params, Type::I32);
    assert_eq!(builder.current_function(), Some(func_id));
    
    // Function should have an entry block
    let module = builder.build();
    let func = module.get_function(func_id).unwrap();
    assert!(func.entry_block.is_some());
}

#[test]
fn test_ir_builder_constants() {
    let mut builder = IrBuilder::new();
    
    let _func = builder.create_function("test".to_string(), vec![], Type::I32);
    
    let int_const = builder.const_value(Constant::I32(42));
    let float_const = builder.const_value(Constant::F32(3.14));
    let bool_const = builder.const_value(Constant::Bool(true));
    let str_const = builder.const_value(Constant::String("hello".to_string()));
    
    assert_ne!(int_const, float_const);
    assert_ne!(bool_const, str_const);
}

#[test]
fn test_ir_builder_arithmetic() {
    let mut builder = IrBuilder::new();
    
    let _func = builder.create_function("arithmetic".to_string(), vec![], Type::I32);
    
    let x = builder.const_value(Constant::I32(10));
    let y = builder.const_value(Constant::I32(20));
    
    let sum = builder.build_binary(BinaryOp::Add, x, y, Type::I32);
    let diff = builder.build_binary(BinaryOp::Sub, x, y, Type::I32);
    let prod = builder.build_binary(BinaryOp::Mul, x, y, Type::I32);
    let quot = builder.build_binary(BinaryOp::Div, x, y, Type::I32);
    
    assert!(sum.is_some());
    assert!(diff.is_some());
    assert!(prod.is_some());
    assert!(quot.is_some());
}

#[test]
fn test_ir_builder_comparisons() {
    let mut builder = IrBuilder::new();
    
    let _func = builder.create_function("compare".to_string(), vec![], Type::Bool);
    
    let x = builder.const_value(Constant::I32(10));
    let y = builder.const_value(Constant::I32(20));
    
    let eq = builder.build_compare(ComparisonOp::Eq, x, y);
    let ne = builder.build_compare(ComparisonOp::Ne, x, y);
    let lt = builder.build_compare(ComparisonOp::Lt, x, y);
    let gt = builder.build_compare(ComparisonOp::Gt, x, y);
    
    assert!(eq.is_some());
    assert!(ne.is_some());
    assert!(lt.is_some());
    assert!(gt.is_some());
}

#[test]
fn test_ir_builder_control_flow() {
    let mut builder = IrBuilder::new();
    
    let _func = builder.create_function("control_flow".to_string(), vec![], Type::I32);
    
    // Create blocks
    let then_block = builder.create_block("then".to_string()).unwrap();
    let else_block = builder.create_block("else".to_string()).unwrap();
    let merge_block = builder.create_block("merge".to_string()).unwrap();
    
    // Build condition
    let cond = builder.const_value(Constant::Bool(true));
    
    // Build conditional branch
    builder.build_cond_branch(cond, then_block, else_block);
    
    // Build then block
    builder.set_current_block(then_block);
    let then_val = builder.const_value(Constant::I32(1));
    builder.build_branch(merge_block);
    
    // Build else block
    builder.set_current_block(else_block);
    let _else_val = builder.const_value(Constant::I32(2));
    builder.build_branch(merge_block);
    
    // Build merge block
    builder.set_current_block(merge_block);
    builder.build_return(Some(then_val));
    
    let module = builder.build();
    assert!(module.validate().is_ok());
}

#[test]
fn test_ir_builder_function_call() {
    let mut builder = IrBuilder::new();
    
    // Create a function to call
    let callee_params = vec![
        Parameter { name: "x".to_string(), ty: Type::I32 },
    ];
    let callee_id = builder.create_function("double".to_string(), callee_params, Type::I32);
    
    // Create caller function
    let _caller_id = builder.create_function("caller".to_string(), vec![], Type::I32);
    
    // Build call
    let arg = builder.const_value(Constant::I32(21));
    let result = builder.build_call(callee_id, vec![arg], Type::I32);
    
    assert!(result.is_some());
    builder.build_return(result);
}

#[test]
fn test_ir_builder_memory_operations() {
    let mut builder = IrBuilder::new();
    
    let _func = builder.create_function("memory".to_string(), vec![], Type::I32);
    
    // Allocate memory
    let ptr = builder.build_alloc(Type::I32).unwrap();
    
    // Store value
    let value = builder.const_value(Constant::I32(42));
    builder.build_store(ptr, value);
    
    // Load value
    let loaded = builder.build_load(ptr, Type::I32);
    assert!(loaded.is_some());
    
    builder.build_return(loaded);
}

#[test]
fn test_ir_module_display() {
    let mut builder = IrBuilder::new();
    
    // Create a simple add function
    let params = vec![
        Parameter { name: "x".to_string(), ty: Type::I32 },
        Parameter { name: "y".to_string(), ty: Type::I32 },
    ];
    
    let _func = builder.create_function("add".to_string(), params, Type::I32);
    
    // Simulate loading parameters (in reality these would be special parameter values)
    let x = builder.const_value(Constant::I32(0)); // Placeholder for parameter
    let y = builder.const_value(Constant::I32(0)); // Placeholder for parameter
    
    let sum = builder.build_binary(BinaryOp::Add, x, y, Type::I32).unwrap();
    builder.build_return(Some(sum));
    
    let module = builder.build();
    let output = module.to_string();
    
    // Check that the output contains expected elements
    assert!(output.contains("Module: main"));
    assert!(output.contains("fn @0 add"));
    assert!(output.contains("add i32"));
    assert!(output.contains("return"));
}

#[test]
fn test_complex_ir_generation() {
    let mut builder = IrBuilder::new();
    
    // Create a factorial-like function
    let params = vec![
        Parameter { name: "n".to_string(), ty: Type::I32 },
    ];
    
    let _func = builder.create_function("factorial".to_string(), params, Type::I32);
    
    // Create blocks
    let check_block = builder.create_block("check".to_string()).unwrap();
    let recurse_block = builder.create_block("recurse".to_string()).unwrap();
    let base_block = builder.create_block("base".to_string()).unwrap();
    
    // Entry: branch to check
    builder.build_branch(check_block);
    
    // Check block: n <= 1?
    builder.set_current_block(check_block);
    let n = builder.const_value(Constant::I32(5)); // Placeholder for parameter
    let one = builder.const_value(Constant::I32(1));
    let cond = builder.build_compare(ComparisonOp::Le, n, one).unwrap();
    builder.build_cond_branch(cond, base_block, recurse_block);
    
    // Base case: return 1
    builder.set_current_block(base_block);
    builder.build_return(Some(one));
    
    // Recursive case: return n * factorial(n-1)
    builder.set_current_block(recurse_block);
    let _n_minus_1 = builder.build_binary(BinaryOp::Sub, n, one, Type::I32).unwrap();
    // In real implementation, we'd call factorial recursively here
    let recursive_result = builder.const_value(Constant::I32(1)); // Placeholder
    let result = builder.build_binary(BinaryOp::Mul, n, recursive_result, Type::I32).unwrap();
    builder.build_return(Some(result));
    
    let module = builder.build();
    assert!(module.validate().is_ok());
    
    // Check the generated IR structure
    let func = module.get_function_by_name("factorial").unwrap();
    assert_eq!(func.blocks().len(), 4); // entry + 3 created blocks
}

#[test]
fn test_phi_node_creation() {
    let mut builder = IrBuilder::new();
    
    let _func = builder.create_function("phi_test".to_string(), vec![], Type::I32);
    
    // Create blocks
    let _entry = BlockId(0); // Current entry block
    let left = builder.create_block("left".to_string()).unwrap();
    let right = builder.create_block("right".to_string()).unwrap();
    let merge = builder.create_block("merge".to_string()).unwrap();
    
    // Branch from entry
    let cond = builder.const_value(Constant::Bool(true));
    builder.build_cond_branch(cond, left, right);
    
    // Left block
    builder.set_current_block(left);
    let left_val = builder.const_value(Constant::I32(10));
    builder.build_branch(merge);
    
    // Right block
    builder.set_current_block(right);
    let right_val = builder.const_value(Constant::I32(20));
    builder.build_branch(merge);
    
    // Merge block with phi
    builder.set_current_block(merge);
    let phi_inst = Instruction::Phi {
        incoming: vec![(left_val, left), (right_val, right)],
        ty: Type::I32,
    };
    let phi_result = builder.add_instruction(phi_inst).unwrap();
    builder.build_return(Some(phi_result));
    
    let module = builder.build();
    let output = module.to_string();
    assert!(output.contains("phi"));
}

#[test]
fn test_type_cast_operations() {
    let mut builder = IrBuilder::new();
    
    let _func = builder.create_function("cast_test".to_string(), vec![], Type::F32);
    
    // Cast i32 to f32
    let int_val = builder.const_value(Constant::I32(42));
    let cast_inst = Instruction::Cast {
        value: int_val,
        from_ty: Type::I32,
        to_ty: Type::F32,
    };
    let float_val = builder.add_instruction(cast_inst).unwrap();
    
    builder.build_return(Some(float_val));
    
    let module = builder.build();
    let output = module.to_string();
    assert!(output.contains("cast"));
    assert!(output.contains("i32 to f32"));
}