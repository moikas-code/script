/// Security test suite for the Script language
/// 
/// This module contains comprehensive security tests for all implemented
/// security features, including:
/// - Generic implementation vulnerabilities (bounds checking, field validation)
/// - Memory safety protections (use-after-free, double-free, buffer overflows)
/// - DoS protection (resource limits, timeouts, compilation bombs)
/// - Integration tests for end-to-end security

pub mod generic_security_tests;
pub mod memory_safety_tests;
pub mod dos_protection_tests;

#[cfg(test)]
mod integration_tests {
    use script::error::{Error, ErrorKind};
    use script::lexer::Lexer;
    use script::parser::Parser;
    use script::semantic::SemanticAnalyzer;
    use script::lowering::AstLowerer;
    use script::codegen::monomorphization::MonomorphizationContext;
    use script::runtime::{Runtime, RuntimeConfig};
    use std::time::Instant;

    /// Test that all security features work together in a complete compilation pipeline
    #[test]
    fn test_complete_security_pipeline() {
        let source = r#"
        struct SecureContainer<T> {
            data: [T],
            capacity: i32,
            count: i32,
        }
        
        impl<T> SecureContainer<T> {
            fn new(capacity: i32) -> Self {
                SecureContainer {
                    data: [],
                    capacity,
                    count: 0,
                }
            }
            
            fn get(&self, index: i32) -> Option<&T> {
                // This should have comprehensive security checks:
                // 1. Bounds checking for array access
                // 2. Field validation for struct access
                // 3. Type safety for generic operations
                if index >= 0 && index < self.count {
                    Some(&self.data[index])  // Bounds check here
                } else {
                    None
                }
            }
            
            fn set(&mut self, index: i32, value: T) -> Result<(), String> {
                if index >= 0 && index < self.capacity {
                    self.data[index] = value;  // Bounds check here
                    if index >= self.count {
                        self.count = index + 1;
                    }
                    Ok(())
                } else {
                    Err("Index out of bounds".to_string())
                }
            }
        }
        
        fn test_security_features() {
            let mut container = SecureContainer::<i32>::new(10);
            
            // Safe operations
            container.set(0, 42).unwrap();
            container.set(1, 84).unwrap();
            
            // Safe access
            let value = container.get(0).unwrap();
            assert_eq!(*value, 42);
            
            // These should trigger security checks:
            
            // Bounds checking
            let result = container.get(100);  // Out of bounds
            assert!(result.is_none());
            
            // Negative index
            let result = container.get(-1);   // Negative index
            assert!(result.is_none());
            
            // Field access validation
            let count = container.count;      // Field access
            assert_eq!(count, 2);
        }
        
        fn main() {
            test_security_features();
        }
        "#;

        // Test complete compilation pipeline
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Tokenization should succeed");
        
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing should succeed");
        
        let mut analyzer = SemanticAnalyzer::new();
        let analyzed = analyzer.analyze(&program).expect("Analysis should succeed");
        
        let mut lowerer = AstLowerer::new();
        let module = lowerer.lower_program(&analyzed).expect("Lowering should succeed");
        
        // Test monomorphization with security
        let mut context = MonomorphizationContext::new();
        let monomorphized = context.monomorphize(&module).expect("Monomorphization should succeed");
        
        // Verify all security features are present
        let functions = monomorphized.functions();
        
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
        
        assert!(has_bounds_check, "Should have bounds checking in complete pipeline");
        assert!(has_field_validation, "Should have field validation in complete pipeline");
        
        // Test runtime integration
        let config = RuntimeConfig::default();
        let runtime = Runtime::new(config);
        assert!(runtime.load_module(monomorphized).is_ok(), 
            "Runtime should handle secure module");
    }

    /// Test that security features don't significantly impact performance
    #[test]
    fn test_security_performance_impact() {
        let source = r#"
        fn simple_array_access(arr: [i32], index: i32) -> i32 {
            arr[index]  // Should have bounds checking
        }
        
        fn simple_field_access(point: Point) -> i32 {
            point.x     // Should have field validation
        }
        
        struct Point {
            x: i32,
            y: i32,
        }
        
        fn main() {
            let arr = [1, 2, 3, 4, 5];
            let point = Point { x: 10, y: 20 };
            
            // These should complete quickly even with security checks
            for i in 0..arr.len() {
                simple_array_access(arr, i);
            }
            
            for _ in 0..1000 {
                simple_field_access(point);
            }
        }
        "#;

        // Time the compilation with security features
        let start_time = Instant::now();
        
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Tokenization should succeed");
        
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing should succeed");
        
        let mut analyzer = SemanticAnalyzer::new();
        let analyzed = analyzer.analyze(&program).expect("Analysis should succeed");
        
        let mut lowerer = AstLowerer::new();
        let module = lowerer.lower_program(&analyzed).expect("Lowering should succeed");
        
        let mut context = MonomorphizationContext::new();
        let _monomorphized = context.monomorphize(&module).expect("Monomorphization should succeed");
        
        let elapsed = start_time.elapsed();
        
        // Security features should not cause excessive compilation time
        assert!(elapsed.as_secs() < 10, 
            "Security features should not significantly impact compilation time: {:?}", elapsed);
    }

    /// Test that security features work correctly with complex generic scenarios
    #[test]
    fn test_complex_generic_security() {
        let source = r#"
        struct Matrix<T> {
            data: [[T]],
            rows: i32,
            cols: i32,
        }
        
        impl<T> Matrix<T> {
            fn get(&self, row: i32, col: i32) -> Option<&T> {
                // Multiple levels of bounds checking
                if row >= 0 && row < self.rows && col >= 0 && col < self.cols {
                    Some(&self.data[row][col])  // Two bounds checks here
                } else {
                    None
                }
            }
        }
        
        struct Container<T, U> {
            first: T,
            second: U,
            metadata: ContainerMetadata,
        }
        
        struct ContainerMetadata {
            size: i32,
            version: i32,
        }
        
        fn complex_generic_operations<T, U>(container: Container<T, U>) -> i32 {
            // Multiple field accesses with different types
            let size = container.metadata.size;      // Nested field access
            let version = container.metadata.version; // Nested field access
            size + version
        }
        
        fn main() {
            let matrix = Matrix {
                data: [[1, 2], [3, 4]],
                rows: 2,
                cols: 2,
            };
            
            let container = Container {
                first: 42,
                second: "hello",
                metadata: ContainerMetadata {
                    size: 10,
                    version: 1,
                },
            };
            
            // These should all have appropriate security checks
            matrix.get(0, 0);
            matrix.get(1, 1);
            matrix.get(2, 2);  // Out of bounds
            
            complex_generic_operations(container);
        }
        "#;

        // Test compilation with complex generics
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Tokenization should succeed");
        
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing should succeed");
        
        let mut analyzer = SemanticAnalyzer::new();
        let analyzed = analyzer.analyze(&program).expect("Analysis should succeed");
        
        let mut lowerer = AstLowerer::new();
        let module = lowerer.lower_program(&analyzed).expect("Lowering should succeed");
        
        let mut context = MonomorphizationContext::new();
        let monomorphized = context.monomorphize(&module).expect("Monomorphization should succeed");
        
        // Verify comprehensive security features
        let functions = monomorphized.functions();
        
        // Should have multiple bounds checks for nested array access
        let bounds_check_count = functions.iter()
            .flat_map(|func| func.body().instructions())
            .filter(|instr| matches!(instr, script::ir::Instruction::BoundsCheck { .. }))
            .count();
        
        // Should have multiple field validations for nested field access
        let field_validation_count = functions.iter()
            .flat_map(|func| func.body().instructions())
            .filter(|instr| matches!(instr, script::ir::Instruction::ValidateFieldAccess { .. }))
            .count();
        
        assert!(bounds_check_count > 0, "Should have bounds checking for matrix access");
        assert!(field_validation_count > 0, "Should have field validation for nested access");
        
        // Verify no excessive resource usage
        let instruction_count = functions.iter()
            .map(|func| func.body().instructions().len())
            .sum::<usize>();
        
        assert!(instruction_count < 10000, 
            "Should not generate excessive instructions: {}", instruction_count);
    }

    /// Test that security error messages are helpful and informative
    #[test]
    fn test_security_error_messages() {
        let mut engine = script::inference::constructor_inference::ConstructorInferenceEngine::new();
        
        // Test type variable limit error
        for _ in 0..15000 {
            if engine.fresh_type_var().is_err() {
                break;
            }
        }
        
        let result = engine.fresh_type_var();
        match result {
            Err(Error { kind: ErrorKind::SecurityViolation, message, .. }) => {
                assert!(message.contains("Type variable limit exceeded"), 
                    "Should have informative error message: {}", message);
                assert!(message.contains("DoS"), 
                    "Should mention DoS protection: {}", message);
            }
            _ => panic!("Should return SecurityViolation with informative message"),
        }
        
        // Test constraint limit error
        let mut engine2 = script::inference::constructor_inference::ConstructorInferenceEngine::new();
        let test_constraint = script::inference::Constraint::equality(
            script::types::Type::I32,
            script::types::Type::I32,
            script::source::Span::new(
                script::source::SourceLocation::new(1, 1, 0),
                script::source::SourceLocation::new(1, 1, 0)
            )
        );
        
        for _ in 0..60000 {
            if engine2.add_constraint(test_constraint.clone()).is_err() {
                break;
            }
        }
        
        let result = engine2.add_constraint(test_constraint);
        match result {
            Err(Error { kind: ErrorKind::SecurityViolation, message, .. }) => {
                assert!(message.contains("Constraint limit exceeded"), 
                    "Should have informative error message: {}", message);
                assert!(message.contains("exponential"), 
                    "Should mention exponential protection: {}", message);
            }
            _ => panic!("Should return SecurityViolation with informative message"),
        }
    }
}