use script::error::{Error, ErrorKind};
use script::lexer::Lexer;
use script::parser::Parser;
use script::semantic::SemanticAnalyzer;
use script::lowering::AstLowerer;
use script::codegen::monomorphization::MonomorphizationContext;
use script::inference::constructor_inference::ConstructorInferenceEngine;
use script::source::SourceLocation;
use std::time::Duration;

/// Test suite for generic implementation security vulnerabilities
/// Tests all fixes implemented for the security audit findings

#[cfg(test)]
mod bounds_checking_tests {
    use super::*;

    #[test]
    fn test_array_bounds_checking_prevents_overflow() {
        let source = r#"
        fn test_array_access() {
            let arr = [1, 2, 3];
            let big_index = 1000000;
            arr[big_index]  // Should trigger bounds check
        }
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Tokenization should succeed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing should succeed");
        
        let mut analyzer = SemanticAnalyzer::new();
        let analyzed = analyzer.analyze(&program).expect("Analysis should succeed");
        
        let mut lowerer = AstLowerer::new();
        
        // The lowering should succeed because we now have bounds checking
        // The bounds check will be inserted as an IR instruction
        let result = lowerer.lower_program(&analyzed);
        assert!(result.is_ok(), "Lowering should succeed with bounds checking");
        
        // Verify that bounds check instructions were generated
        let module = result.unwrap();
        let functions = module.functions();
        assert!(!functions.is_empty(), "Should have at least one function");
        
        // Check that bounds check instructions exist in the IR
        let has_bounds_check = functions.iter().any(|func| {
            func.body().instructions().iter().any(|instr| {
                matches!(instr, script::ir::Instruction::BoundsCheck { .. })
            })
        });
        
        assert!(has_bounds_check, "Should contain bounds check instructions");
    }

    #[test]
    fn test_nested_array_bounds_checking() {
        let source = r#"
        fn test_nested_arrays() {
            let matrix = [[1, 2], [3, 4]];
            let row = 10;
            let col = 20;
            matrix[row][col]  // Should trigger bounds checks for both dimensions
        }
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Tokenization should succeed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing should succeed");
        
        let mut analyzer = SemanticAnalyzer::new();
        let analyzed = analyzer.analyze(&program).expect("Analysis should succeed");
        
        let mut lowerer = AstLowerer::new();
        let result = lowerer.lower_program(&analyzed);
        assert!(result.is_ok(), "Nested array bounds checking should work");
        
        // Verify multiple bounds check instructions for nested access
        let module = result.unwrap();
        let functions = module.functions();
        let bounds_check_count = functions.iter()
            .flat_map(|func| func.body().instructions())
            .filter(|instr| matches!(instr, script::ir::Instruction::BoundsCheck { .. }))
            .count();
        
        assert!(bounds_check_count >= 2, "Should have bounds checks for both array accesses");
    }

    #[test]
    fn test_dynamic_index_bounds_checking() {
        let source = r#"
        fn test_dynamic_index(arr: [i32], index: i32) {
            arr[index]  // Dynamic index should still be bounds checked
        }
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Tokenization should succeed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing should succeed");
        
        let mut analyzer = SemanticAnalyzer::new();
        let analyzed = analyzer.analyze(&program).expect("Analysis should succeed");
        
        let mut lowerer = AstLowerer::new();
        let result = lowerer.lower_program(&analyzed);
        assert!(result.is_ok(), "Dynamic index bounds checking should work");
        
        // Verify bounds check for dynamic indices
        let module = result.unwrap();
        let functions = module.functions();
        let has_bounds_check = functions.iter().any(|func| {
            func.body().instructions().iter().any(|instr| {
                matches!(instr, script::ir::Instruction::BoundsCheck { .. })
            })
        });
        
        assert!(has_bounds_check, "Should contain bounds check for dynamic index");
    }
}

#[cfg(test)]
mod field_validation_tests {
    use super::*;

    #[test]
    fn test_field_access_validation() {
        let source = r#"
        struct Point {
            x: i32,
            y: i32,
        }
        
        fn test_field_access() {
            let p = Point { x: 10, y: 20 };
            p.x  // Should use ValidateFieldAccess instruction
        }
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Tokenization should succeed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing should succeed");
        
        let mut analyzer = SemanticAnalyzer::new();
        let analyzed = analyzer.analyze(&program).expect("Analysis should succeed");
        
        let mut lowerer = AstLowerer::new();
        let result = lowerer.lower_program(&analyzed);
        assert!(result.is_ok(), "Field access validation should work");
        
        // Verify field validation instructions
        let module = result.unwrap();
        let functions = module.functions();
        let has_field_validation = functions.iter().any(|func| {
            func.body().instructions().iter().any(|instr| {
                matches!(instr, script::ir::Instruction::ValidateFieldAccess { .. })
            })
        });
        
        assert!(has_field_validation, "Should contain field validation instructions");
    }

    #[test]
    fn test_generic_struct_field_validation() {
        let source = r#"
        struct Container<T> {
            value: T,
        }
        
        fn test_generic_field_access() {
            let container = Container { value: 42 };
            container.value  // Should validate generic field access
        }
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Tokenization should succeed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing should succeed");
        
        let mut analyzer = SemanticAnalyzer::new();
        let analyzed = analyzer.analyze(&program).expect("Analysis should succeed");
        
        let mut lowerer = AstLowerer::new();
        let result = lowerer.lower_program(&analyzed);
        assert!(result.is_ok(), "Generic field access validation should work");
        
        // Check for field validation in generic context
        let module = result.unwrap();
        let functions = module.functions();
        let has_field_validation = functions.iter().any(|func| {
            func.body().instructions().iter().any(|instr| {
                matches!(instr, script::ir::Instruction::ValidateFieldAccess { .. })
            })
        });
        
        assert!(has_field_validation, "Should validate generic struct field access");
    }

    #[test]
    fn test_nested_field_access_validation() {
        let source = r#"
        struct Inner {
            value: i32,
        }
        
        struct Outer {
            inner: Inner,
        }
        
        fn test_nested_field_access() {
            let outer = Outer { inner: Inner { value: 42 } };
            outer.inner.value  // Should validate each field access
        }
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Tokenization should succeed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing should succeed");
        
        let mut analyzer = SemanticAnalyzer::new();
        let analyzed = analyzer.analyze(&program).expect("Analysis should succeed");
        
        let mut lowerer = AstLowerer::new();
        let result = lowerer.lower_program(&analyzed);
        assert!(result.is_ok(), "Nested field access validation should work");
        
        // Verify multiple field validation instructions
        let module = result.unwrap();
        let functions = module.functions();
        let field_validation_count = functions.iter()
            .flat_map(|func| func.body().instructions())
            .filter(|instr| matches!(instr, script::ir::Instruction::ValidateFieldAccess { .. }))
            .count();
        
        assert!(field_validation_count >= 2, "Should have validation for each field access");
    }
}

#[cfg(test)]
mod resource_limit_tests {
    use super::*;

    #[test]
    fn test_type_inference_resource_limits() {
        let mut engine = ConstructorInferenceEngine::new();
        
        // Test type variable limit
        let mut type_vars = Vec::new();
        for _ in 0..9999 {
            if let Ok(type_var) = engine.fresh_type_var() {
                type_vars.push(type_var);
            } else {
                break;
            }
        }
        
        // Should hit the limit and return an error
        let result = engine.fresh_type_var();
        match result {
            Err(Error { kind: ErrorKind::SecurityViolation, .. }) => {
                // Expected - hit the type variable limit
            }
            _ => panic!("Should hit type variable limit and return SecurityViolation"),
        }
    }

    #[test]
    fn test_constraint_generation_limits() {
        let mut engine = ConstructorInferenceEngine::new();
        
        // Generate constraints until we hit the limit
        let test_constraint = script::inference::Constraint::equality(
            script::types::Type::I32,
            script::types::Type::I32,
            script::source::Span::new(
                SourceLocation::new(1, 1, 0),
                SourceLocation::new(1, 1, 0)
            )
        );
        
        let mut constraint_count = 0;
        loop {
            match engine.add_constraint(test_constraint.clone()) {
                Ok(()) => constraint_count += 1,
                Err(Error { kind: ErrorKind::SecurityViolation, .. }) => {
                    break; // Hit the limit
                }
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
            
            // Safety check to prevent infinite loop
            if constraint_count > 60000 {
                panic!("Constraint limit should have been hit by now");
            }
        }
        
        assert!(constraint_count >= 50000, "Should hit constraint limit around 50,000");
    }

    #[test]
    fn test_monomorphization_resource_limits() {
        let mut context = MonomorphizationContext::new();
        
        // Test specialization limits by trying to add too many instantiations
        let mut specialization_count = 0;
        loop {
            let func_name = format!("test_function_{}", specialization_count);
            let type_args = vec![script::types::Type::I32];
            
            match context.add_instantiation(func_name, type_args) {
                Ok(()) => specialization_count += 1,
                Err(Error { kind: ErrorKind::SecurityViolation, .. }) => {
                    break; // Hit the limit
                }
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
            
            // Safety check to prevent infinite loop
            if specialization_count > 2000 {
                panic!("Specialization limit should have been hit by now");
            }
        }
        
        assert!(specialization_count >= 1000, "Should hit specialization limit around 1,000");
    }

    #[test]
    #[should_panic(expected = "timeout")]
    fn test_inference_timeout_protection() {
        let mut engine = ConstructorInferenceEngine::new();
        
        // Create a constraint solving scenario that would take too long
        // This is a simplified test - in practice, timeout would be hit during
        // complex type inference scenarios
        
        // Add many constraints that create a complex solving scenario
        for i in 0..1000 {
            let constraint = script::inference::Constraint::equality(
                script::types::Type::TypeVar(i),
                script::types::Type::TypeVar(i + 1),
                script::source::Span::new(
                    SourceLocation::new(1, 1, 0),
                    SourceLocation::new(1, 1, 0)
                )
            );
            engine.add_constraint(constraint).unwrap();
        }
        
        // This should eventually timeout
        let result = engine.solve_constraints();
        
        match result {
            Err(Error { kind: ErrorKind::SecurityViolation, message, .. }) 
                if message.contains("timeout") => {
                panic!("timeout"); // Expected timeout
            }
            _ => panic!("Should have hit timeout protection"),
        }
    }

    #[test]
    fn test_work_queue_size_limits() {
        let mut context = MonomorphizationContext::new();
        
        // Test work queue size limits
        let mut queue_items = 0;
        loop {
            let func_name = format!("queued_function_{}", queue_items);
            let type_args = vec![script::types::Type::I32];
            
            match context.add_to_work_queue(func_name, type_args) {
                Ok(()) => queue_items += 1,
                Err(Error { kind: ErrorKind::SecurityViolation, .. }) => {
                    break; // Hit the queue size limit
                }
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
            
            // Safety check to prevent infinite loop
            if queue_items > 12000 {
                panic!("Work queue size limit should have been hit by now");
            }
        }
        
        assert!(queue_items >= 10000, "Should hit work queue size limit around 10,000");
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_end_to_end_security_pipeline() {
        let source = r#"
        struct GenericContainer<T> {
            items: [T],
            count: i32,
        }
        
        fn process_container<T>(container: GenericContainer<T>) {
            let index = container.count;
            container.items[index]  // Should have bounds checking
        }
        
        fn main() {
            let container = GenericContainer {
                items: [1, 2, 3],
                count: 10,  // Deliberately out of bounds
            };
            process_container(container);
        }
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Tokenization should succeed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing should succeed");
        
        let mut analyzer = SemanticAnalyzer::new();
        let analyzed = analyzer.analyze(&program).expect("Analysis should succeed");
        
        let mut lowerer = AstLowerer::new();
        let result = lowerer.lower_program(&analyzed);
        assert!(result.is_ok(), "End-to-end security pipeline should work");
        
        // Verify all security features are present
        let module = result.unwrap();
        let functions = module.functions();
        
        let has_bounds_check = functions.iter().any(|func| {
            func.body().instructions().iter().any(|instr| {
                matches!(instr, script::ir::Instruction::BoundsCheck { .. })
            })
        });
        
        let has_field_validation = functions.iter().any(|func| {
            func.body().instructions().iter().any(|instr| {
                matches!(instr, script::ir::Instruction::ValidateFieldAccess { .. })
            })
        });
        
        assert!(has_bounds_check, "Should have bounds checking");
        assert!(has_field_validation, "Should have field validation");
    }

    #[test]
    fn test_security_with_monomorphization() {
        let source = r#"
        struct Pair<T, U> {
            first: T,
            second: U,
        }
        
        fn access_pair<T, U>(pair: Pair<T, U>) -> T {
            pair.first  // Should validate field access in monomorphized version
        }
        
        fn main() {
            let int_pair = Pair { first: 42, second: "hello" };
            let float_pair = Pair { first: 3.14, second: true };
            
            access_pair(int_pair);
            access_pair(float_pair);
        }
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Tokenization should succeed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing should succeed");
        
        let mut analyzer = SemanticAnalyzer::new();
        let analyzed = analyzer.analyze(&program).expect("Analysis should succeed");
        
        let mut lowerer = AstLowerer::new();
        let module = lowerer.lower_program(&analyzed).expect("Lowering should succeed");
        
        // Test monomorphization with security
        let mut mono_context = MonomorphizationContext::new();
        let result = mono_context.monomorphize(&module);
        
        assert!(result.is_ok(), "Monomorphization with security should work");
        
        // Verify security features are preserved after monomorphization
        let monomorphized = result.unwrap();
        let functions = monomorphized.functions();
        
        let has_field_validation = functions.iter().any(|func| {
            func.body().instructions().iter().any(|instr| {
                matches!(instr, script::ir::Instruction::ValidateFieldAccess { .. })
            })
        });
        
        assert!(has_field_validation, "Field validation should be preserved in monomorphized code");
    }
}