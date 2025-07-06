use script::codegen::monomorphization::MonomorphizationContext;
use script::ir::{Module, Function, FunctionId, Parameter, IrBuilder, Instruction, ValueId};
use script::ir::instruction::{Constant, BinaryOp};
use script::types::Type;
use script::semantic::analyzer::GenericInstantiation;
use std::collections::HashMap;

#[test]
fn test_complete_monomorphization_workflow() {
    // Create a module with a generic function
    let mut module = Module::new();
    
    // Create a generic identity function: fn identity<T>(x: T) -> T { x }
    let generic_func_id = module.create_function(
        "identity".to_string(),
        vec![Parameter {
            name: "x".to_string(),
            ty: Type::TypeParam("T".to_string()),
        }],
        Type::TypeParam("T".to_string()),
    );
    
    // Add a simple return instruction to the function
    if let Some(func) = module.get_function_mut(generic_func_id) {
        let entry_block = func.create_block("entry".to_string());
        if let Some(block) = func.get_block_mut(entry_block) {
            // In a real implementation, this would return the parameter
            block.add_instruction(ValueId(0), Instruction::Return(Some(ValueId(999))));
        }
    }
    
    // Create monomorphization context with generic instantiations
    let mut mono_ctx = MonomorphizationContext::new();
    
    // Simulate semantic analysis results
    let instantiations = vec![
        GenericInstantiation {
            function_name: "identity".to_string(),
            type_args: vec![Type::I32],
            span: script::source::Span::dummy(),
        },
        GenericInstantiation {
            function_name: "identity".to_string(),
            type_args: vec![Type::String],
            span: script::source::Span::dummy(),
        },
    ];
    
    let type_info = HashMap::new();
    mono_ctx.initialize_from_semantic_analysis(&instantiations, &type_info);
    
    // Perform monomorphization
    let result = mono_ctx.monomorphize(&mut module);
    assert!(result.is_ok(), "Monomorphization should succeed");
    
    // Verify results
    let stats = mono_ctx.stats();
    assert_eq!(stats.functions_monomorphized, 2);
    assert_eq!(stats.type_instantiations, 2);
    
    // Check that specialized functions were added
    assert!(module.has_function_by_name("identity_i32"));
    assert!(module.has_function_by_name("identity_string"));
    
    // Check that the generic function was removed (if it has specializations)
    // Note: In a real implementation, this might depend on whether the generic function is still needed
    
    println!("Monomorphization completed successfully!");
    println!("Functions monomorphized: {}", stats.functions_monomorphized);
    println!("Type instantiations: {}", stats.type_instantiations);
    println!("Duplicates avoided: {}", stats.duplicates_avoided);
}

#[test]
fn test_monomorphization_with_complex_types() {
    let mut module = Module::new();
    
    // Create a generic map function: fn map<T, U>(arr: Array<T>, f: fn(T) -> U) -> Array<U>
    let map_func_id = module.create_function(
        "map".to_string(),
        vec![
            Parameter {
                name: "arr".to_string(),
                ty: Type::Array(Box::new(Type::TypeParam("T".to_string()))),
            },
            Parameter {
                name: "f".to_string(),
                ty: Type::Function {
                    params: vec![Type::TypeParam("T".to_string())],
                    ret: Box::new(Type::TypeParam("U".to_string())),
                },
            },
        ],
        Type::Array(Box::new(Type::TypeParam("U".to_string()))),
    );
    
    // Add basic block structure
    if let Some(func) = module.get_function_mut(map_func_id) {
        let entry_block = func.create_block("entry".to_string());
        if let Some(block) = func.get_block_mut(entry_block) {
            block.add_instruction(ValueId(0), Instruction::Return(Some(ValueId(999))));
        }
    }
    
    let mut mono_ctx = MonomorphizationContext::new();
    
    // Instantiate map with i32 -> string
    let instantiations = vec![
        GenericInstantiation {
            function_name: "map".to_string(),
            type_args: vec![Type::I32, Type::String],
            span: script::source::Span::dummy(),
        },
    ];
    
    mono_ctx.initialize_from_semantic_analysis(&instantiations, &HashMap::new());
    
    let result = mono_ctx.monomorphize(&mut module);
    assert!(result.is_ok(), "Complex type monomorphization should succeed");
    
    // Verify the specialized function has correct types
    if let Some(specialized_func) = module.get_function_by_name("map_i32_string") {
        // Check parameter types were substituted correctly
        assert_eq!(specialized_func.params[0].ty, Type::Array(Box::new(Type::I32)));
        assert_eq!(specialized_func.return_type, Type::Array(Box::new(Type::String)));
        
        // Check function parameter type
        if let Type::Function { params, ret } = &specialized_func.params[1].ty {
            assert_eq!(params[0], Type::I32);
            assert_eq!(**ret, Type::String);
        } else {
            panic!("Expected Function type for second parameter");
        }
    } else {
        panic!("Specialized map function not found");
    }
}

#[test]
fn test_monomorphization_duplicate_handling() {
    let mut module = Module::new();
    
    // Create generic function
    let func_id = module.create_function(
        "duplicate_test".to_string(),
        vec![Parameter {
            name: "x".to_string(),
            ty: Type::TypeParam("T".to_string()),
        }],
        Type::TypeParam("T".to_string()),
    );
    
    if let Some(func) = module.get_function_mut(func_id) {
        let entry_block = func.create_block("entry".to_string());
        if let Some(block) = func.get_block_mut(entry_block) {
            block.add_instruction(ValueId(0), Instruction::Return(Some(ValueId(999))));
        }
    }
    
    let mut mono_ctx = MonomorphizationContext::new();
    
    // Add the same instantiation multiple times
    let instantiations = vec![
        GenericInstantiation {
            function_name: "duplicate_test".to_string(),
            type_args: vec![Type::I32],
            span: script::source::Span::dummy(),
        },
        GenericInstantiation {
            function_name: "duplicate_test".to_string(),
            type_args: vec![Type::I32], // Duplicate
        },
        GenericInstantiation {
            function_name: "duplicate_test".to_string(),
            type_args: vec![Type::I32], // Another duplicate
        },
    ];
    
    mono_ctx.initialize_from_semantic_analysis(&instantiations, &HashMap::new());
    
    let result = mono_ctx.monomorphize(&mut module);
    assert!(result.is_ok());
    
    let stats = mono_ctx.stats();
    // Should only create one specialized function despite multiple requests
    assert_eq!(stats.functions_monomorphized, 1);
    assert_eq!(stats.duplicates_avoided, 0); // Duplicates are avoided at the work queue level
}

#[test]
fn test_error_handling_in_monomorphization() {
    let mut module = Module::new();
    let mut mono_ctx = MonomorphizationContext::new();
    
    // Try to instantiate a non-existent function
    let instantiations = vec![
        GenericInstantiation {
            function_name: "non_existent".to_string(),
            type_args: vec![Type::I32],
            span: script::source::Span::dummy(),
        },
    ];
    
    mono_ctx.initialize_from_semantic_analysis(&instantiations, &HashMap::new());
    
    // Should handle missing functions gracefully
    let result = mono_ctx.monomorphize(&mut module);
    assert!(result.is_ok(), "Should handle missing functions gracefully");
    
    // No functions should be monomorphized
    assert_eq!(mono_ctx.stats().functions_monomorphized, 0);
}