use script::lexer::Lexer;
use script::parser::{ExprKind, Parser, StmtKind, TypeKind};

#[test]
fn test_generic_type_parsing() {
    let test_cases = vec![
        ("let x: Vec<i32> = Vec<i32>()", "Vec<i32>"),
        (
            "let map: HashMap<String, i32> = HashMap<String, i32>()",
            "HashMap<String, i32>",
        ),
        ("let opt: Option<T> = None", "Option<T>"),
        (
            "let nested: Vec<Option<i32>> = Vec<Option<i32>>()",
            "Vec<Option<i32>>",
        ),
    ];

    for (input, expected_type) in test_cases {
        let lexer = Lexer::new(input).expect("Failed to create lexer");
        let (tokens, errors) = lexer.scan_tokens();
        assert!(
            errors.is_empty(),
            "Lexer errors for '{}': {:?}",
            input,
            errors
        );

        let mut parser = Parser::new(tokens);
        let program = parser
            .parse()
            .expect(&format!("Failed to parse: {}", input));

        // Verify the AST contains the expected generic type
        let stmt = &program.statements[0];
        match &stmt.kind {
            StmtKind::Let { type_ann, .. } => {
                let type_ann = type_ann.as_ref().expect("Missing type annotation");
                let actual = format!("{}", type_ann);
                assert_eq!(actual, expected_type, "Type mismatch for: {}", input);
            }
            _ => panic!("Expected Let statement"),
        }
    }
}

#[test]
fn test_generic_constructor_expression() {
    let test_cases = vec![
        "Vec<i32>()",
        "HashMap<String, T>()",
        "Option<Result<T, E>>()",
    ];

    for input in test_cases {
        let lexer = Lexer::new(input).expect("Failed to create lexer");
        let (tokens, errors) = lexer.scan_tokens();
        assert!(
            errors.is_empty(),
            "Lexer errors for '{}': {:?}",
            input,
            errors
        );

        let mut parser = Parser::new(tokens);
        let program = parser
            .parse()
            .expect(&format!("Failed to parse: {}", input));

        // Verify the expression is a Call with GenericConstructor as callee
        let stmt = &program.statements[0];
        match &stmt.kind {
            StmtKind::Expression(expr) => {
                match &expr.kind {
                    ExprKind::Call { callee, args } => {
                        // The callee should be a GenericConstructor
                        match &callee.kind {
                            ExprKind::GenericConstructor { name, type_args } => {
                                assert!(!name.is_empty());
                                assert!(!type_args.is_empty());
                                assert!(args.is_empty()); // Constructor calls have no args
                                println!("Successfully parsed generic constructor call: {}", expr);
                            }
                            _ => panic!(
                                "Expected GenericConstructor as callee, got: {:?}",
                                callee.kind
                            ),
                        }
                    }
                    _ => panic!("Expected Call expression, got: {:?}", expr.kind),
                }
            }
            _ => panic!("Expected Expression statement"),
        }
    }
}

#[test]
fn test_type_parameter_recognition() {
    let inputs = vec![
        ("let x: T = value", "T"),
        ("let y: U = value", "U"),
        ("let z: TKey = value", "TKey"),
        ("fn foo(x: T) -> U { x }", "T"),
    ];

    for (input, expected_param) in inputs {
        let lexer = Lexer::new(input).expect("Failed to create lexer");
        let (tokens, errors) = lexer.scan_tokens();
        assert!(errors.is_empty());

        let mut parser = Parser::new(tokens);
        let program = parser
            .parse()
            .expect(&format!("Failed to parse: {}", input));

        // Check that type parameters are recognized
        let has_type_param = match &program.statements[0].kind {
            StmtKind::Let { type_ann, .. } => type_ann.as_ref().map_or(
                false,
                |ann| matches!(&ann.kind, TypeKind::TypeParam(name) if name == expected_param),
            ),
            StmtKind::Function { params, .. } => params.iter().any(
                |p| matches!(&p.type_ann.kind, TypeKind::TypeParam(name) if name == expected_param),
            ),
            _ => false,
        };

        assert!(
            has_type_param,
            "Type parameter '{}' not recognized in: {}",
            expected_param, input
        );
    }
}

#[test]
fn test_nested_generic_types() {
    let test_cases = vec![
        "let x: Vec<Vec<i32>> = Vec<Vec<i32>>()",
        "let y: Map<String, Vec<Option<T>>> = Map<String, Vec<Option<T>>>()",
        "let z: Result<Vec<T>, Error<E>> = Ok(Vec<T>())",
    ];

    for input in test_cases {
        let lexer = Lexer::new(input).expect("Failed to create lexer");
        let (tokens, errors) = lexer.scan_tokens();
        assert!(
            errors.is_empty(),
            "Lexer errors for '{}': {:?}",
            input,
            errors
        );

        let mut parser = Parser::new(tokens);
        let result = parser.parse();
        assert!(result.is_ok(), "Failed to parse nested generics: {}", input);
    }
}
