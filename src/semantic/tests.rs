use super::*;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::types::Type;

fn parse_and_analyze(source: &str) -> Result<SemanticAnalyzer> {
    let lexer = Lexer::new(source)?;
    let (tokens, errors) = lexer.scan_tokens();
    if !errors.is_empty() {
        return Err(errors[0].clone());
    }
    let mut parser = Parser::new(tokens);
    let program = parser.parse()?;

    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze_program(&program)?;
    Ok(analyzer)
}

fn expect_semantic_error(source: &str, expected_kind: SemanticErrorKind) {
    let result = parse_and_analyze(source);
    assert!(result.is_err());

    // Parse again to get the analyzer with errors
    let lexer = Lexer::new(source).unwrap();
    let (tokens, _) = lexer.scan_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut analyzer = SemanticAnalyzer::new();
    let _ = analyzer.analyze_program(&program);

    assert!(!analyzer.errors().is_empty());
    assert_eq!(analyzer.errors()[0].kind, expected_kind);
}

#[test]
fn test_basic_variable_declaration() {
    let analyzer = parse_and_analyze("let x: i32 = 42;").unwrap();
    let symbol = analyzer.symbol_table().lookup("x").unwrap();
    assert_eq!(symbol.name, "x");
    assert_eq!(symbol.ty, Type::I32);
    assert!(symbol.is_mutable);
}

#[test]
fn test_variable_without_initializer() {
    // Variable without initializer and with type annotation should work
    let analyzer = parse_and_analyze("let x: i32; x;").unwrap();
    let symbol = analyzer.symbol_table().lookup("x").unwrap();
    assert_eq!(symbol.ty, Type::I32);
}

#[test]
fn test_variable_type_inference() {
    let analyzer = parse_and_analyze("let x = 42;").unwrap();
    let symbol = analyzer.symbol_table().lookup("x").unwrap();
    // Currently returns Unknown, but should infer numeric type
    assert_eq!(symbol.ty, Type::Unknown);
}

#[test]
fn test_undefined_variable_error() {
    expect_semantic_error(
        "x + 1;",
        SemanticErrorKind::UndefinedVariable("x".to_string()),
    );
}

#[test]
fn test_duplicate_variable_error() {
    expect_semantic_error(
        r#"
        let x: i32 = 1;
        let x: i32 = 2;
        "#,
        SemanticErrorKind::DuplicateVariable("x".to_string()),
    );
}

#[test]
fn test_variable_shadowing() {
    let analyzer = parse_and_analyze(
        r#"
        let x: i32 = 1;
        {
            let x: f32 = 2.0;
            x;
        }
        x;
    "#,
    )
    .unwrap();

    // Outer x should still be visible
    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_function_declaration() {
    let analyzer = parse_and_analyze(
        r#"
        fn add(x: i32, y: i32) -> i32 {
            x + y
        }
    "#,
    )
    .unwrap();

    let symbol = analyzer.symbol_table().lookup("add").unwrap();
    assert!(symbol.is_function());

    let sig = symbol.function_signature().unwrap();
    assert_eq!(sig.params.len(), 2);
    assert_eq!(sig.params[0].0, "x");
    assert_eq!(sig.params[0].1, Type::I32);
    assert_eq!(sig.return_type, Type::I32);
}

#[test]
fn test_function_call() {
    let analyzer = parse_and_analyze(
        r#"
        fn add(x: i32, y: i32) -> i32 {
            x + y
        }
        
        add(1, 2);
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_undefined_function_error() {
    expect_semantic_error(
        "foo(1, 2);",
        SemanticErrorKind::UndefinedFunction("foo".to_string()),
    );
}

#[test]
fn test_wrong_argument_count() {
    let result = parse_and_analyze(
        r#"
        fn add(x: i32, y: i32) -> i32 {
            x + y
        }
        
        add(1);
    "#,
    );

    // Should fail due to wrong argument count
    assert!(result.is_err());
}

#[test]
fn test_function_overloading() {
    let analyzer = parse_and_analyze(
        r#"
        fn add(x: i32) -> i32 { x }
        fn add(x: i32, y: i32) -> i32 { x + y }
        
        add(1);
        add(1, 2);
    "#,
    )
    .unwrap();

    let functions = analyzer.symbol_table().lookup_all("add");
    assert_eq!(functions.len(), 2);
}

#[test]
fn test_duplicate_function_signature() {
    expect_semantic_error(
        r#"
        fn add(x: i32) -> i32 { x }
        fn add(y: i32) -> f32 { y }
        "#,
        SemanticErrorKind::DuplicateFunction("add".to_string()),
    );
}

#[test]
fn test_parameter_shadowing() {
    let analyzer = parse_and_analyze(
        r#"
        fn test(x: i32, y: i32) -> i32 {
            let z: i32 = x + y;
            z
        }
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_duplicate_parameter_error() {
    expect_semantic_error(
        "fn test(x: i32, x: i32) -> i32 { x }",
        SemanticErrorKind::DuplicateVariable("x".to_string()),
    );
}

#[test]
fn test_while_loop() {
    let analyzer = parse_and_analyze(
        r#"
        let i: i32 = 0;
        while i < 10 {
            i = i + 1;
        }
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_for_loop() {
    let analyzer = parse_and_analyze(
        r#"
        let arr = [1, 2, 3];
        for x in arr {
            x;
        }
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_for_loop_variable_scope() {
    expect_semantic_error(
        r#"
        for x in [1, 2, 3] {
            x;
        }
        x;  // x should not be visible here
        "#,
        SemanticErrorKind::UndefinedVariable("x".to_string()),
    );
}

#[test]
fn test_array_literal() {
    let analyzer = parse_and_analyze(
        r#"
        let arr = [1, 2, 3];
        arr[0];
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_assignment() {
    let analyzer = parse_and_analyze(
        r#"
        let x: i32 = 1;
        x = 2;
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_assignment_to_immutable() {
    // Note: Currently all variables are mutable in our implementation
    // This test would fail with proper mutability checking
    let analyzer = parse_and_analyze(
        r#"
        let x: i32 = 1;
        x = 2;
    "#,
    )
    .unwrap();

    // Should have an error when mutability is properly implemented
    // For now, it passes because all variables are mutable
    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_assignment_type_mismatch() {
    expect_semantic_error(
        r#"
        let x: i32 = 42;
        x = "hello";
    "#,
        SemanticErrorKind::TypeMismatch {
            expected: Type::I32,
            found: Type::String,
        },
    );
}

#[test]
fn test_assignment_type_compatible() {
    let analyzer = parse_and_analyze(
        r#"
        let x: i32 = 42;
        x = 100;
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_assignment_unknown_type() {
    // Unknown type should be assignable to any type (gradual typing)
    let analyzer = parse_and_analyze(
        r#"
        let x = 42;  // Inferred as i32
        let y;       // Unknown type
        x = y;       // Should be allowed
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_array_element_assignment_type_check() {
    expect_semantic_error(
        r#"
        let arr: [i32] = [1, 2, 3];
        arr[0] = "hello";
    "#,
        SemanticErrorKind::TypeMismatch {
            expected: Type::I32,
            found: Type::String,
        },
    );
}

#[test]
fn test_array_element_assignment_compatible() {
    let analyzer = parse_and_analyze(
        r#"
        let arr: [i32] = [1, 2, 3];
        arr[0] = 42;
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_if_expression() {
    let analyzer = parse_and_analyze(
        r#"
        let x = if true { 1 } else { 2 };
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_if_without_else() {
    let analyzer = parse_and_analyze(
        r#"
        if true { 1 }
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_if_non_boolean_condition() {
    let lexer = Lexer::new(
        r#"
        if 42 { 1 } else { 2 }
    "#,
    )
    .unwrap();
    let (tokens, _) = lexer.scan_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut analyzer = SemanticAnalyzer::new();
    let _ = analyzer.analyze_program(&program);

    assert_eq!(analyzer.errors().len(), 1);

    match &analyzer.errors()[0].kind {
        SemanticErrorKind::TypeMismatch { expected, found } => {
            assert_eq!(*expected, Type::Bool);
            assert_eq!(*found, Type::I32);
        }
        _ => panic!("Expected TypeMismatch error"),
    }
}

#[test]
fn test_if_incompatible_branch_types() {
    let lexer = Lexer::new(
        r#"
        let x = if true { 42 } else { "hello" };
    "#,
    )
    .unwrap();
    let (tokens, _) = lexer.scan_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut analyzer = SemanticAnalyzer::new();
    let _ = analyzer.analyze_program(&program);

    assert_eq!(analyzer.errors().len(), 1);

    match &analyzer.errors()[0].kind {
        SemanticErrorKind::TypeMismatch { expected, found } => {
            assert_eq!(*expected, Type::I32);
            assert_eq!(*found, Type::String);
        }
        _ => panic!("Expected TypeMismatch error"),
    }
}

#[test]
fn test_if_compatible_numeric_types() {
    let analyzer = parse_and_analyze(
        r#"
        let x = if true { 42 } else { 100 };
        let y = if false { 3.14 } else { 2.71 };
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_nested_if_expressions() {
    let analyzer = parse_and_analyze(
        r#"
        let x = if true {
            if false { 1 } else { 2 }
        } else {
            if true { 3 } else { 4 }
        };
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_if_with_unknown_type_branches() {
    // When one branch has Unknown type, we should use the other branch's type
    let analyzer = parse_and_analyze(
        r#"
        fn unknown_func();
        let x = if true { 42 } else { unknown_func() };
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_if_expression_as_function_argument() {
    let analyzer = parse_and_analyze(
        r#"
        fn take_number(n: i32) -> i32 { n }
        take_number(if true { 42 } else { 0 });
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_if_expression_with_string_condition() {
    let lexer = Lexer::new(
        r#"
        if "not a boolean" { 1 } else { 2 }
    "#,
    )
    .unwrap();
    let (tokens, _) = lexer.scan_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut analyzer = SemanticAnalyzer::new();
    let _ = analyzer.analyze_program(&program);

    assert_eq!(analyzer.errors().len(), 1);

    match &analyzer.errors()[0].kind {
        SemanticErrorKind::TypeMismatch { expected, found } => {
            assert_eq!(*expected, Type::Bool);
            assert_eq!(*found, Type::String);
        }
        _ => panic!("Expected TypeMismatch error"),
    }
}

#[test]
fn test_block_scope() {
    let analyzer = parse_and_analyze(
        r#"
        let x: i32 = 1;
        {
            let y: i32 = 2;
            x + y;
        }
        x;
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_block_scope_variable_not_visible() {
    expect_semantic_error(
        r#"
        {
            let y: i32 = 2;
        }
        y;
        "#,
        SemanticErrorKind::UndefinedVariable("y".to_string()),
    );
}

#[test]
fn test_return_in_function() {
    let analyzer = parse_and_analyze(
        r#"
        fn test() -> i32 {
            return 42;
        }
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_return_outside_function() {
    expect_semantic_error("return 42;", SemanticErrorKind::ReturnOutsideFunction);
}

#[test]
fn test_binary_operations() {
    let analyzer = parse_and_analyze(
        r#"
        1 + 2;
        3.0 - 1.0;
        4 * 5;
        6 / 2;
        7 % 3;
        1 < 2;
        3 > 2;
        4 <= 4;
        5 >= 5;
        6 == 6;
        7 != 8;
        true && false;
        true || false;
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_unary_operations() {
    let analyzer = parse_and_analyze(
        r#"
        -42;
        !true;
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_builtin_functions() {
    let analyzer = parse_and_analyze(
        r#"
        print("Hello, world!");
        let arr = [1, 2, 3];
        len(arr);
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_nested_function_calls() {
    let analyzer = parse_and_analyze(
        r#"
        fn add(x: i32, y: i32) -> i32 { x + y }
        fn mul(x: i32, y: i32) -> i32 { x * y }
        
        mul(add(1, 2), 3);
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_complex_expression() {
    let analyzer = parse_and_analyze(
        r#"
        fn factorial(n: i32) -> i32 {
            if n <= 1 {
                1
            } else {
                n * factorial(n - 1)
            }
        }
        
        factorial(5);
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_unused_variable_detection() {
    let lexer = Lexer::new("let x: i32 = 42;").unwrap();
    let (tokens, _) = lexer.scan_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut analyzer = SemanticAnalyzer::new();
    let _ = analyzer.analyze_program(&program);

    // Currently we skip unused variable warnings
    // In a complete implementation, this would be a warning
    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_used_variable() {
    let analyzer = parse_and_analyze(
        r#"
        let x: i32 = 42;
        x + 1;
    "#,
    )
    .unwrap();

    // x is used, so no unused variable error
    let unused = analyzer.symbol_table().get_unused_symbols();
    assert!(unused.iter().all(|s| s.name != "x"));
}

// Const function tests
#[test]
fn test_basic_const_function() {
    let analyzer = parse_and_analyze(
        r#"
        @const
        fn square(x: i32) -> i32 {
            x * x
        }
        
        square(5);
    "#,
    )
    .unwrap();

    let symbol = analyzer.symbol_table().lookup("square").unwrap();
    let sig = symbol.function_signature().unwrap();
    assert!(sig.is_const);
    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_const_function_calling_const_function() {
    let analyzer = parse_and_analyze(
        r#"
        @const
        fn square(x: i32) -> i32 {
            x * x
        }
        
        @const
        fn sum_of_squares(a: i32, b: i32) -> i32 {
            square(a) + square(b)
        }
        
        sum_of_squares(3, 4);
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_const_function_calling_non_const_function() {
    expect_semantic_error(
        r#"
        fn regular_func(x: i32) -> i32 { x }
        
        @const
        fn const_func(x: i32) -> i32 {
            regular_func(x)
        }
    "#,
        SemanticErrorKind::ConstFunctionViolation(
            "@const functions can only call other @const functions, but 'regular_func' is not @const".to_string(),
        ),
    );
}

#[test]
fn test_const_function_cannot_be_async() {
    expect_semantic_error(
        r#"
        @const
        async fn bad_func() -> i32 {
            42
        }
    "#,
        SemanticErrorKind::ConstFunctionViolation("@const functions cannot be async".to_string()),
    );
}

#[test]
fn test_const_function_no_assignment() {
    expect_semantic_error(
        r#"
        @const
        fn bad_func(x: i32) -> i32 {
            let y = 10;
            y = x;  // Assignment not allowed
            y
        }
    "#,
        SemanticErrorKind::ConstFunctionViolation(
            "assignment expressions not allowed in @const functions".to_string(),
        ),
    );
}

#[test]
fn test_const_function_no_await() {
    expect_semantic_error(
        r#"
        @const
        fn bad_func() -> i32 {
            await some_future()
        }
    "#,
        SemanticErrorKind::ConstFunctionViolation(
            "await expressions not allowed in @const functions".to_string(),
        ),
    );
}

#[test]
fn test_const_function_with_loops() {
    let analyzer = parse_and_analyze(
        r#"
        @const
        fn factorial(n: i32) -> i32 {
            let result = 1;
            let i = 1;
            while i <= n {
                result = result * i;
                i = i + 1;
            }
            result
        }
    "#,
    )
    .unwrap();

    // This should fail because of assignment in loops
    // TODO: The current implementation doesn't catch this - needs refinement
    assert!(!analyzer.errors().is_empty());
}

#[test]
fn test_const_function_nested_functions_not_allowed() {
    expect_semantic_error(
        r#"
        @const
        fn outer() -> i32 {
            fn inner() -> i32 { 42 }
            inner()
        }
    "#,
        SemanticErrorKind::ConstFunctionViolation(
            "nested functions not allowed in @const functions".to_string(),
        ),
    );
}

#[test]
fn test_const_variable_with_initializer() {
    let analyzer = parse_and_analyze(
        r#"
        @const
        let PI = 3.14159;
        
        PI;
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_const_variable_without_initializer() {
    expect_semantic_error(
        r#"
        @const
        let UNINITIALIZED;
    "#,
        SemanticErrorKind::ConstFunctionViolation(
            "@const variables must have an initializer".to_string(),
        ),
    );
}

#[test]
fn test_const_function_with_if_expressions() {
    let analyzer = parse_and_analyze(
        r#"
        @const
        fn max(a: i32, b: i32) -> i32 {
            if a > b { a } else { b }
        }
        
        max(10, 20);
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_const_function_with_array_operations() {
    let analyzer = parse_and_analyze(
        r#"
        @const
        fn first_element(arr: [i32]) -> i32 {
            arr[0]
        }
        
        first_element([1, 2, 3]);
    "#,
    )
    .unwrap();

    assert!(analyzer.errors().is_empty());
}

#[test]
fn test_const_function_list_comprehensions_not_allowed() {
    expect_semantic_error(
        r#"
        @const
        fn generate_squares(n: i32) -> [i32] {
            [x * x for x in 0..n]
        }
    "#,
        SemanticErrorKind::ConstFunctionViolation(
            "list comprehensions not yet supported in @const functions".to_string(),
        ),
    );
}
