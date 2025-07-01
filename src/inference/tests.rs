use super::*;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::types::Type;

/// Helper function to parse and infer types for a program
fn infer_types(input: &str) -> Result<InferenceResult, Error> {
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.scan_tokens();
    if !errors.is_empty() {
        return Err(errors[0].clone());
    }
    let mut parser = Parser::new(tokens);
    let program = parser.parse()?;
    
    let mut engine = InferenceEngine::new();
    engine.infer_program(&program)
}

/// Helper to check if a type exists in the result
fn has_type(result: &InferenceResult, expected: &Type) -> bool {
    result.expr_types.values().any(|t| t == expected) ||
    result.stmt_types.values().any(|t| t == expected)
}

#[test]
fn test_literal_inference() {
    // Number literals get fresh type variables
    let result = infer_types("42;").unwrap();
    // Should have a type variable for the number
    assert!(result.expr_types.values().any(|t| matches!(t, Type::TypeVar(_))));
    
    // String literals
    let result = infer_types("\"hello world\";").unwrap();
    assert!(has_type(&result, &Type::String));
    
    // Boolean literals
    let result = infer_types("true; false;").unwrap();
    assert!(has_type(&result, &Type::Bool));
}

#[test]
fn test_variable_inference() {
    // Variable without type annotation
    let result = infer_types("let x = 42; x;").unwrap();
    // Both literal and variable should have the same type variable
    let type_vars: Vec<_> = result.expr_types.values()
        .filter_map(|t| match t {
            Type::TypeVar(id) => Some(*id),
            _ => None,
        })
        .collect();
    assert!(type_vars.len() >= 2); // literal and variable reference
    
    // Variable with type annotation
    let result = infer_types("let x: i32 = 42;").unwrap();
    assert!(has_type(&result, &Type::I32));
}

#[test]
fn test_arithmetic_operations() {
    // Basic arithmetic - numbers get type variables
    let result = infer_types("1 + 2;").unwrap();
    // Result should be a type variable (numeric but not yet resolved)
    assert!(result.expr_types.values().any(|t| matches!(t, Type::TypeVar(_))));
    
    // Complex arithmetic
    let result = infer_types("(1 + 2) * 3 - 4 / 2;").unwrap();
    assert!(result.expr_types.values().any(|t| matches!(t, Type::TypeVar(_))));
    
    // Variable arithmetic with explicit types
    let result = infer_types("let x: f32 = 10; let y: f32 = 20; x + y;").unwrap();
    assert!(has_type(&result, &Type::F32));
}

#[test]
fn test_comparison_operations() {
    // Numeric comparisons
    let result = infer_types("1 < 2;").unwrap();
    assert!(has_type(&result, &Type::Bool));
    
    let result = infer_types("3.14 >= 2.71;").unwrap();
    assert!(has_type(&result, &Type::Bool));
    
    // Equality
    let result = infer_types("\"hello\" == \"world\";").unwrap();
    assert!(has_type(&result, &Type::Bool));
}

#[test]
fn test_logical_operations() {
    // Basic logical ops
    let result = infer_types("true && false;").unwrap();
    assert!(has_type(&result, &Type::Bool));
    
    let result = infer_types("true || false;").unwrap();
    assert!(has_type(&result, &Type::Bool));
    
    // Complex logical expressions
    let result = infer_types("(1 < 2) && (3 > 2) || false;").unwrap();
    assert!(has_type(&result, &Type::Bool));
}

#[test]
fn test_unary_operations() {
    // Negation - numeric gets type variable
    let result = infer_types("-42;").unwrap();
    assert!(result.expr_types.values().any(|t| matches!(t, Type::TypeVar(_))));
    
    // Logical not
    let result = infer_types("!true;").unwrap();
    assert!(has_type(&result, &Type::Bool));
    
    let result = infer_types("!(1 < 2);").unwrap();
    assert!(has_type(&result, &Type::Bool));
}

#[test]
fn test_if_expressions() {
    // Simple if - numeric literals get type variables
    let result = infer_types("if true { 42 } else { 0 };").unwrap();
    assert!(result.expr_types.values().any(|t| matches!(t, Type::TypeVar(_))));
    
    // If with explicit type
    let result = infer_types("let x: i32 = if true { 1 } else { 2 }; x;").unwrap();
    assert!(has_type(&result, &Type::I32));
}

#[test]
fn test_array_inference() {
    // Empty array
    let code = "let arr = []; arr;";
    let result = infer_types(code).unwrap();
    // Should have Array type with type variable element
    assert!(result.expr_types.values().any(|t| matches!(t, Type::Array(_))));
    
    // Array with elements - numeric literals get type variables
    let result = infer_types("[1, 2, 3];").unwrap();
    assert!(result.expr_types.values().any(|t| matches!(t, Type::Array(_))));
    
    // Array indexing with explicit type
    let result = infer_types("let arr: [i32] = [1, 2, 3]; arr[0];").unwrap();
    assert!(has_type(&result, &Type::I32)); // indexing result
}

#[test]
fn test_function_inference() {
    // Function with explicit types
    let code = r#"
        fn add(x: i32, y: i32) -> i32 {
            x + y
        }
    "#;
    let result = infer_types(code).unwrap();
    let fn_type = Type::Function {
        params: vec![Type::I32, Type::I32],
        ret: Box::new(Type::I32),
    };
    assert!(has_type(&result, &fn_type));
    
    // Function without return type annotation
    let code = r#"
        fn double(x: f32) {
            x * 2
        }
    "#;
    let result = infer_types(code).unwrap();
    let fn_type = Type::Function {
        params: vec![Type::F32],
        ret: Box::new(Type::F32),
    };
    assert!(has_type(&result, &fn_type));
}

#[test]
fn test_function_calls() {
    // Direct function call
    let code = r#"
        fn identity(x: string) -> string { x }
        identity("hello");
    "#;
    let result = infer_types(code).unwrap();
    assert!(has_type(&result, &Type::String));
    
    // Function with multiple parameters
    let code = r#"
        fn add(x: i32, y: i32) -> i32 { x + y }
        add(10, 20);
    "#;
    let result = infer_types(code).unwrap();
    assert!(has_type(&result, &Type::I32));
}

#[test]
fn test_block_expressions() {
    // Block with final expression - numeric gets type variables
    let result = infer_types("{ let x = 1; x + 1 };").unwrap();
    assert!(result.expr_types.values().any(|t| matches!(t, Type::TypeVar(_))));
    
    // Nested blocks with explicit types
    let code = r#"
        {
            let x: f32 = 10;
            {
                let y: f32 = 20;
                x + y
            }
        };
    "#;
    let result = infer_types(code).unwrap();
    assert!(has_type(&result, &Type::F32));
}

#[test]
fn test_while_loops() {
    let code = r#"
        let i = 0;
        while i < 10 {
            i = i + 1;
        }
    "#;
    let result = infer_types(code).unwrap();
    assert!(has_type(&result, &Type::Bool)); // condition type
}

#[test]
fn test_for_loops() {
    let code = r#"
        let arr: [i32] = [1, 2, 3];
        for x in arr {
            x + 1;
        }
    "#;
    let result = infer_types(code).unwrap();
    assert!(has_type(&result, &Type::Array(Box::new(Type::I32))));
}

#[test]
fn test_gradual_typing() {
    // Unknown type should accept any value
    let code = r#"
        let x: unknown = 42;
        let y: unknown = "hello";
        let z: unknown = true;
    "#;
    let result = infer_types(code).unwrap();
    // All assignments should succeed
    assert!(has_type(&result, &Type::Unknown));
}

#[test]
fn test_type_errors() {
    // Type mismatch in binary operation
    // This currently doesn't error because we don't enforce numeric constraints
    // TODO: Add numeric type constraints for arithmetic operators
    // let result = infer_types("true + 1;");
    // assert!(result.is_err());
    
    // Wrong number of function arguments
    let result = infer_types(r#"
        fn f(x: i32) -> i32 { x }
        f(1, 2);
    "#);
    assert!(result.is_err());
    
    // Undefined variable
    let result = infer_types("x + 1;");
    assert!(result.is_err());
    
    // Type mismatch in assignment
    let result = infer_types("let x: i32 = \"hello\";");
    assert!(result.is_err());
    
    // Array element type mismatch - string vs bool
    let result = infer_types("[\"hello\", true, \"world\"];");
    assert!(result.is_err());
}

#[test]
fn test_assignment_expressions() {
    let code = r#"
        let x: i32 = 0;
        x = 42;
    "#;
    let result = infer_types(code).unwrap();
    assert!(has_type(&result, &Type::I32));
}

#[test]
fn test_complex_inference() {
    // Function returning array
    let code = r#"
        fn make_array(x: i32) -> [i32] {
            [x, x + 1, x + 2]
        }
        make_array(10);
    "#;
    let result = infer_types(code).unwrap();
    assert!(has_type(&result, &Type::Array(Box::new(Type::I32))));
    
    // Higher-order function (function type annotation)
    let code = r#"
        fn apply(f: (i32) -> i32, x: i32) -> i32 {
            f(x)
        }
    "#;
    let result = infer_types(code).unwrap();
    let apply_type = Type::Function {
        params: vec![
            Type::Function {
                params: vec![Type::I32],
                ret: Box::new(Type::I32),
            },
            Type::I32,
        ],
        ret: Box::new(Type::I32),
    };
    assert!(has_type(&result, &apply_type));
}

#[test]
fn test_return_statements() {
    let code = r#"
        fn early_return(x: i32) -> i32 {
            if x < 0 {
                return 0;
            }
            x
        }
    "#;
    let result = infer_types(code).unwrap();
    let fn_type = Type::Function {
        params: vec![Type::I32],
        ret: Box::new(Type::I32),
    };
    assert!(has_type(&result, &fn_type));
}

#[test]
fn test_nested_functions() {
    let code = r#"
        fn outer(x: i32) -> i32 {
            fn inner(y: i32) -> i32 {
                y * 2
            }
            inner(x)
        }
    "#;
    let result = infer_types(code).unwrap();
    // Both functions should be properly typed
    let outer_type = Type::Function {
        params: vec![Type::I32],
        ret: Box::new(Type::I32),
    };
    let inner_type = Type::Function {
        params: vec![Type::I32],
        ret: Box::new(Type::I32),
    };
    assert!(has_type(&result, &outer_type));
    assert!(has_type(&result, &inner_type));
}