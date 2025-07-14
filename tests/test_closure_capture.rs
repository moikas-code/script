use script::types::Type;
use script::{AstLowerer, Lexer, Parser, SemanticAnalyzer};
use std::collections::HashMap;

#[test]
fn test_simple_capture() {
    let source = r#"
        fn test() {
            let x = 42;
            let closure = |y| x + y;
            closure(10)
        }
    "#;

    let lexer = Lexer::new(source).unwrap();
    let (tokens, _) = lexer.scan_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze_program(&program).unwrap();

    // The test passes if semantic analysis succeeds without errors
    // This means the closure and its captures were analyzed correctly
    let errors = analyzer.errors();
    assert!(errors.is_empty(), "Should have no semantic errors");
}

#[test]
fn test_mutable_capture() {
    let source = r#"
        fn test() {
            let mut counter = 0;
            let increment = || {
                counter = counter + 1;
                counter
            };
            increment()
        }
    "#;

    let lexer = Lexer::new(source).unwrap();
    let (tokens, _) = lexer.scan_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze_program(&program).unwrap();

    // The test passes if semantic analysis succeeds without errors
    let errors = analyzer.errors();
    assert!(errors.is_empty(), "Should have no semantic errors");
}

#[test]
fn test_multiple_captures() {
    let source = r#"
        fn test() {
            let a = 10;
            let b = 20;
            let mut c = 30;
            
            let closure = |x| {
                c = c + 1;
                a + b + c + x
            };
            
            closure(5)
        }
    "#;

    let lexer = Lexer::new(source).unwrap();
    let (tokens, _) = lexer.scan_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze_program(&program).unwrap();

    // The test passes if semantic analysis succeeds without errors
    let errors = analyzer.errors();
    assert!(errors.is_empty(), "Should have no semantic errors");
}

#[test]
fn test_no_captures() {
    let source = r#"
        fn test() {
            let closure = |x, y| x + y;
            closure(5, 10)
        }
    "#;

    let lexer = Lexer::new(source).unwrap();
    let (tokens, _) = lexer.scan_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze_program(&program).unwrap();

    // The test passes if semantic analysis succeeds without errors
    let errors = analyzer.errors();
    assert!(errors.is_empty(), "Should have no semantic errors");
}

#[test]
fn test_nested_closure_captures() {
    let source = r#"
        fn test() {
            let outer = 100;
            let mut shared = 50;
            
            let outer_closure = |x| {
                let local = 10;
                let inner_closure = |y| {
                    shared = shared + 1;
                    outer + local + x + y + shared
                };
                inner_closure(5)
            };
            
            outer_closure(20)
        }
    "#;

    let lexer = Lexer::new(source).unwrap();
    let (tokens, _) = lexer.scan_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze_program(&program).unwrap();

    // The test passes if semantic analysis succeeds without errors
    let errors = analyzer.errors();
    assert!(errors.is_empty(), "Should have no semantic errors");
}

#[test]
fn test_capture_in_control_flow() {
    let source = r#"
        fn test() {
            let x = 10;
            let mut y = 20;
            
            if x > 5 {
                let closure = || {
                    y = y * 2;
                    x + y
                };
                closure()
            } else {
                0
            }
        }
    "#;

    let lexer = Lexer::new(source).unwrap();
    let (tokens, _) = lexer.scan_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze_program(&program).unwrap();

    // The test passes if semantic analysis succeeds without errors
    let errors = analyzer.errors();
    assert!(errors.is_empty(), "Should have no semantic errors");
}
