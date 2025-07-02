/// Integration test to verify generic types work across the entire pipeline
use script::lexer::Lexer;
use script::parser::Parser;
use script::semantic::SemanticAnalyzer;

#[test]
fn test_generic_types_semantic_analysis() {
    let code = r#"
        let vec: Vec<i32> = Vec<i32>()
        let map: HashMap<String, i32> = HashMap<String, i32>()
        
        // Generic functions would go here, but they're not implemented yet
        // fn identity<T>(x: T) -> T { x }
    "#;

    // Lex
    let lexer = Lexer::new(code);
    let (tokens, lex_errors) = lexer.scan_tokens();
    assert!(lex_errors.is_empty(), "Lexer errors: {:?}", lex_errors);

    // Parse
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    // Semantic analysis
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze_program(&program);

    // The semantic analyzer should handle generic types without errors
    // Even though it currently treats them as named types
    assert!(result.is_ok(), "Semantic analysis failed: {:?}", result);
}

#[test]
fn test_generic_type_display() {
    use script::parser::{TypeAnn, TypeKind};
    use script::source::{SourceLocation, Span};

    let dummy_span = Span::new(SourceLocation::new(1, 1, 0), SourceLocation::new(1, 10, 9));

    // Test display of various generic types
    let test_cases = vec![
        (
            TypeAnn {
                kind: TypeKind::Generic {
                    name: "Vec".to_string(),
                    args: vec![TypeAnn {
                        kind: TypeKind::Named("i32".to_string()),
                        span: dummy_span,
                    }],
                },
                span: dummy_span,
            },
            "Vec<i32>",
        ),
        (
            TypeAnn {
                kind: TypeKind::Generic {
                    name: "HashMap".to_string(),
                    args: vec![
                        TypeAnn {
                            kind: TypeKind::Named("String".to_string()),
                            span: dummy_span,
                        },
                        TypeAnn {
                            kind: TypeKind::TypeParam("T".to_string()),
                            span: dummy_span,
                        },
                    ],
                },
                span: dummy_span,
            },
            "HashMap<String, T>",
        ),
        (
            TypeAnn {
                kind: TypeKind::TypeParam("T".to_string()),
                span: dummy_span,
            },
            "T",
        ),
    ];

    for (type_ann, expected) in test_cases {
        let displayed = format!("{}", type_ann);
        assert_eq!(displayed, expected, "Display mismatch for type");
    }
}

#[test]
fn test_nested_generic_parsing_and_display() {
    let code = "let x: Result<Vec<Option<T>>, Error<E>> = Ok(Vec<Option<T>>())";

    let lexer = Lexer::new(code);
    let (tokens, errors) = lexer.scan_tokens();
    assert!(errors.is_empty());

    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    // Check that the AST was built correctly
    match &program.statements[0].kind {
        script::parser::StmtKind::Let { type_ann, .. } => {
            let type_ann = type_ann.as_ref().expect("Missing type annotation");
            let displayed = format!("{}", type_ann);
            assert_eq!(displayed, "Result<Vec<Option<T>>, Error<E>>");
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn test_generic_constructor_in_expressions() {
    let code = r#"
        let x = Vec<i32>()
        let y = HashMap<String, i32>()
        let z = Some<T>(value)
    "#;

    let lexer = Lexer::new(code);
    let (tokens, errors) = lexer.scan_tokens();
    assert!(errors.is_empty());

    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    // All three statements should parse successfully
    assert_eq!(program.statements.len(), 3);

    // Each should be a Let with a Call expression containing a GenericConstructor
    for stmt in &program.statements {
        match &stmt.kind {
            script::parser::StmtKind::Let { init, .. } => {
                let init = init.as_ref().expect("Missing initializer");
                match &init.kind {
                    script::parser::ExprKind::Call { callee, .. } => match &callee.kind {
                        script::parser::ExprKind::GenericConstructor { name, type_args } => {
                            assert!(!name.is_empty());
                            assert!(!type_args.is_empty());
                        }
                        _ => panic!("Expected GenericConstructor as callee"),
                    },
                    _ => panic!("Expected Call expression"),
                }
            }
            _ => panic!("Expected Let statement"),
        }
    }
}

#[test]
fn test_generic_type_in_function_signatures() {
    // Test generic types in function parameters and return types
    // (without generic function syntax which isn't implemented yet)
    let code = r#"
        fn process_vec(items: Vec<i32>) -> Vec<String> {
            Vec<String>()
        }
        
        fn create_map() -> HashMap<String, i32> {
            HashMap<String, i32>()
        }
    "#;

    let lexer = Lexer::new(code);
    let (tokens, errors) = lexer.scan_tokens();
    assert!(errors.is_empty());

    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    // Verify both functions parsed correctly
    assert_eq!(program.statements.len(), 2);

    // Check first function
    match &program.statements[0].kind {
        script::parser::StmtKind::Function {
            params, ret_type, ..
        } => {
            // Check parameter type
            assert_eq!(params.len(), 1);
            let param_type = format!("{}", params[0].type_ann);
            assert_eq!(param_type, "Vec<i32>");

            // Check return type
            let ret = ret_type.as_ref().expect("Missing return type");
            assert_eq!(format!("{}", ret), "Vec<String>");
        }
        _ => panic!("Expected Function statement"),
    }
}
