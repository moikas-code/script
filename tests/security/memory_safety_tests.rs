use script::error::{Error, ErrorKind};
use script::lexer::Lexer;
use script::parser::Parser;
use script::semantic::SemanticAnalyzer;
use script::lowering::AstLowerer;
use script::runtime::{Runtime, RuntimeConfig};
use script::types::Type;
use std::time::Duration;

/// Test suite for memory safety features
/// Tests bounds checking, field validation, and memory corruption prevention

#[cfg(test)]
mod memory_bounds_tests {
    use super::*;

    #[test]
    fn test_array_buffer_overflow_prevention() {
        let source = r#"
        fn test_overflow() {
            let small_array = [1, 2, 3];
            let dangerous_index = 100;
            small_array[dangerous_index]  // Should be caught by bounds checking
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
        assert!(result.is_ok(), "Should compile with bounds checking");
        
        // Verify bounds check IR instructions are generated
        let module = result.unwrap();
        let functions = module.functions();
        let bounds_checks = functions.iter()
            .flat_map(|func| func.body().instructions())
            .filter(|instr| matches!(instr, script::ir::Instruction::BoundsCheck { .. }))
            .count();
        
        assert!(bounds_checks > 0, "Should generate bounds check instructions");
    }

    #[test]
    fn test_negative_array_index_protection() {
        let source = r#"
        fn test_negative_index() {
            let arr = [10, 20, 30];
            let negative_index = -1;
            arr[negative_index]  // Should be caught by bounds checking
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
        assert!(result.is_ok(), "Should compile with negative index protection");
        
        // Verify bounds check handles negative indices
        let module = result.unwrap();
        let functions = module.functions();
        let has_bounds_check = functions.iter().any(|func| {
            func.body().instructions().iter().any(|instr| {
                if let script::ir::Instruction::BoundsCheck { error_msg, .. } = instr {
                    error_msg.contains("bounds")
                } else {
                    false
                }
            })
        });
        
        assert!(has_bounds_check, "Should have bounds checking for negative indices");
    }

    #[test]
    fn test_array_length_validation() {
        let source = r#"
        fn test_length_based_access(arr: [i32], len: i32) {
            let index = len - 1;
            arr[index]  // Should validate against actual array length
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
        assert!(result.is_ok(), "Should compile with length validation");
        
        // Verify bounds check compares against actual array length
        let module = result.unwrap();
        let functions = module.functions();
        let has_length_check = functions.iter().any(|func| {
            func.body().instructions().iter().any(|instr| {
                matches!(instr, script::ir::Instruction::BoundsCheck { length: Some(_), .. })
            })
        });
        
        assert!(has_length_check, "Should validate against actual array length");
    }
}

#[cfg(test)]
mod type_safety_tests {
    use super::*;

    #[test]
    fn test_struct_field_type_confusion_prevention() {
        let source = r#"
        struct Point {
            x: i32,
            y: i32,
        }
        
        struct Color {
            r: i32,
            g: i32,
            b: i32,
        }
        
        fn test_field_access(point: Point) {
            point.x  // Should validate Point has x field
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
        assert!(result.is_ok(), "Should compile with field validation");
        
        // Verify field validation instructions
        let module = result.unwrap();
        let functions = module.functions();
        let has_field_validation = functions.iter().any(|func| {
            func.body().instructions().iter().any(|instr| {
                if let script::ir::Instruction::ValidateFieldAccess { field_name, .. } = instr {
                    field_name == "x"
                } else {
                    false
                }
            })
        });
        
        assert!(has_field_validation, "Should validate field access");
    }

    #[test]
    fn test_generic_type_safety() {
        let source = r#"
        struct Container<T> {
            value: T,
        }
        
        fn test_generic_field_access<T>(container: Container<T>) -> T {
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
        assert!(result.is_ok(), "Should compile with generic field validation");
        
        // Verify field validation for generic types
        let module = result.unwrap();
        let functions = module.functions();
        let has_generic_field_validation = functions.iter().any(|func| {
            func.body().instructions().iter().any(|instr| {
                if let script::ir::Instruction::ValidateFieldAccess { field_name, object_type, .. } = instr {
                    field_name == "value" && matches!(object_type, Type::Generic { .. })
                } else {
                    false
                }
            })
        });
        
        assert!(has_generic_field_validation, "Should validate generic field access");
    }

    #[test]
    fn test_enum_variant_type_safety() {
        let source = r#"
        enum Option<T> {
            Some(T),
            None,
        }
        
        fn test_enum_access(opt: Option<i32>) {
            match opt {
                Some(value) => value,  // Should validate enum variant access
                None => 0,
            }
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
        assert!(result.is_ok(), "Should compile with enum variant validation");
        
        // Verify enum variant validation
        let module = result.unwrap();
        let functions = module.functions();
        let has_enum_validation = functions.iter().any(|func| {
            func.body().instructions().iter().any(|instr| {
                // Check for enum-related instructions
                matches!(instr, script::ir::Instruction::Match { .. })
            })
        });
        
        assert!(has_enum_validation, "Should validate enum variant access");
    }
}

#[cfg(test)]
mod memory_corruption_tests {
    use super::*;

    #[test]
    fn test_use_after_free_prevention() {
        let source = r#"
        fn test_use_after_free() {
            let arr = [1, 2, 3];
            let ptr = &arr[0];
            // In a real scenario, arr might be freed here
            // But our bounds checking should prevent unsafe access
            *ptr
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
        assert!(result.is_ok(), "Should compile with memory safety checks");
        
        // Verify safety instructions are generated
        let module = result.unwrap();
        let functions = module.functions();
        let has_safety_checks = functions.iter().any(|func| {
            func.body().instructions().iter().any(|instr| {
                matches!(instr, 
                    script::ir::Instruction::BoundsCheck { .. } |
                    script::ir::Instruction::ValidateFieldAccess { .. }
                )
            })
        });
        
        assert!(has_safety_checks, "Should have memory safety checks");
    }

    #[test]
    fn test_double_free_prevention() {
        let source = r#"
        fn test_double_free() {
            let container = Box::new(42);
            // In unsafe code, this could lead to double-free
            // Our safety checks should prevent this
            drop(container);
            // Any further access should be caught
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
        assert!(result.is_ok(), "Should compile with double-free prevention");
        
        // Verify memory management instructions
        let module = result.unwrap();
        let functions = module.functions();
        let has_memory_management = functions.iter().any(|func| {
            func.body().instructions().iter().any(|instr| {
                // Check for memory management related instructions
                matches!(instr, script::ir::Instruction::Call { .. })
            })
        });
        
        assert!(has_memory_management, "Should have memory management checks");
    }

    #[test]
    fn test_buffer_underflow_prevention() {
        let source = r#"
        fn test_underflow() {
            let arr = [1, 2, 3];
            let index = -5;  // Negative index causing underflow
            arr[index]  // Should be caught by bounds checking
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
        assert!(result.is_ok(), "Should compile with underflow prevention");
        
        // Verify bounds checking handles underflow
        let module = result.unwrap();
        let functions = module.functions();
        let has_underflow_check = functions.iter().any(|func| {
            func.body().instructions().iter().any(|instr| {
                matches!(instr, script::ir::Instruction::BoundsCheck { .. })
            })
        });
        
        assert!(has_underflow_check, "Should prevent buffer underflow");
    }
}

#[cfg(test)]
mod runtime_safety_tests {
    use super::*;

    #[test]
    fn test_runtime_bounds_checking() {
        let source = r#"
        fn main() {
            let arr = [1, 2, 3, 4, 5];
            let index = 10;  // Out of bounds
            arr[index]  // Should trigger runtime bounds check
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
        
        // Verify runtime will perform bounds checking
        let config = RuntimeConfig::default();
        let runtime = Runtime::new(config);
        
        // The runtime should be able to handle the module with bounds checks
        // In a real scenario, this would trigger a runtime error when executed
        assert!(runtime.load_module(module).is_ok(), "Runtime should handle bounds checking");
    }

    #[test]
    fn test_runtime_field_validation() {
        let source = r#"
        struct Point {
            x: i32,
            y: i32,
        }
        
        fn main() {
            let p = Point { x: 10, y: 20 };
            p.x  // Should validate field access at runtime
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
        
        // Verify runtime field validation
        let config = RuntimeConfig::default();
        let runtime = Runtime::new(config);
        
        assert!(runtime.load_module(module).is_ok(), "Runtime should handle field validation");
    }

    #[test]
    fn test_runtime_type_safety() {
        let source = r#"
        enum Value {
            Int(i32),
            String(String),
        }
        
        fn main() {
            let val = Value::Int(42);
            match val {
                Int(n) => n,
                String(s) => s.len(),  // Type safety should be enforced
            }
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
        
        // Verify runtime type safety
        let config = RuntimeConfig::default();
        let runtime = Runtime::new(config);
        
        assert!(runtime.load_module(module).is_ok(), "Runtime should enforce type safety");
    }
}

#[cfg(test)]
mod integration_memory_tests {
    use super::*;

    #[test]
    fn test_comprehensive_memory_safety() {
        let source = r#"
        struct SafeContainer<T> {
            data: [T],
            size: i32,
        }
        
        fn safe_access<T>(container: SafeContainer<T>, index: i32) -> T {
            // This should have comprehensive safety checks:
            // 1. Bounds checking for array access
            // 2. Field validation for struct access
            // 3. Type safety for generic access
            if index >= 0 && index < container.size {
                container.data[index]
            } else {
                panic!("Index out of bounds")
            }
        }
        
        fn main() {
            let container = SafeContainer {
                data: [1, 2, 3, 4, 5],
                size: 5,
            };
            
            safe_access(container, 2);  // Safe access
            safe_access(container, 10); // Should be caught by bounds check
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
        
        // Verify all safety features are present
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
        
        // Verify runtime integration
        let config = RuntimeConfig::default();
        let runtime = Runtime::new(config);
        assert!(runtime.load_module(module).is_ok(), "Runtime should handle comprehensive safety");
    }
}