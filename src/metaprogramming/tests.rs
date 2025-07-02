use super::*;
use crate::metaprogramming::const_eval::ConstValue;
use crate::parser::*;
use crate::source::Span;

#[test]
fn test_derive_debug() {
    let mut processor = MetaprogrammingProcessor::new();

    let mut func_stmt = Stmt {
        kind: StmtKind::Function {
            name: "Point".to_string(),
            generic_params: None,
            params: vec![],
            ret_type: None,
            body: Block {
                statements: vec![],
                final_expr: None,
            },
            is_async: false,
        },
        span: Span::dummy(),
        attributes: vec![Attribute {
            name: "derive".to_string(),
            args: vec!["Debug".to_string()],
            span: Span::dummy(),
        }],
    };

    let generated = processor.process_statement(&mut func_stmt).unwrap();
    assert_eq!(generated.len(), 1);

    // Check that a debug function was generated
    if let StmtKind::Function { name, .. } = &generated[0].kind {
        assert_eq!(name, "Point_debug");
    } else {
        panic!("Expected function statement");
    }
}

#[test]
fn test_derive_multiple_traits() {
    let mut processor = MetaprogrammingProcessor::new();

    let mut func_stmt = Stmt {
        kind: StmtKind::Function {
            name: "User".to_string(),
            generic_params: None,
            params: vec![],
            ret_type: None,
            body: Block {
                statements: vec![],
                final_expr: None,
            },
            is_async: false,
        },
        span: Span::dummy(),
        attributes: vec![Attribute {
            name: "derive".to_string(),
            args: vec!["Debug".to_string(), "Serialize".to_string()],
            span: Span::dummy(),
        }],
    };

    let generated = processor.process_statement(&mut func_stmt).unwrap();
    assert_eq!(generated.len(), 2);

    // Check that both functions were generated
    let func_names: Vec<String> = generated
        .iter()
        .filter_map(|stmt| {
            if let StmtKind::Function { name, .. } = &stmt.kind {
                Some(name.clone())
            } else {
                None
            }
        })
        .collect();

    assert!(func_names.contains(&"User_debug".to_string()));
    assert!(func_names.contains(&"User_serialize".to_string()));
}

#[test]
fn test_const_function_registration() {
    let mut processor = MetaprogrammingProcessor::new();

    let mut const_func = Stmt {
        kind: StmtKind::Function {
            name: "factorial".to_string(),
            generic_params: None,
            params: vec![Param {
                name: "n".to_string(),
                type_ann: TypeAnn {
                    kind: TypeKind::Named("Number".to_string()),
                    span: Span::dummy(),
                },
            }],
            ret_type: Some(TypeAnn {
                kind: TypeKind::Named("Number".to_string()),
                span: Span::dummy(),
            }),
            body: Block {
                statements: vec![],
                final_expr: Some(Box::new(Expr {
                    kind: ExprKind::Literal(Literal::Number(1.0)),
                    span: Span::dummy(),
                })),
            },
            is_async: false,
        },
        span: Span::dummy(),
        attributes: vec![Attribute {
            name: "const".to_string(),
            args: vec![],
            span: Span::dummy(),
        }],
    };

    // Should not generate any new statements, just register the function
    let generated = processor.process_statement(&mut const_func).unwrap();
    assert_eq!(generated.len(), 0);
}

#[test]
fn test_const_evaluator() {
    let mut evaluator = ConstEvaluator::new();

    // Test literal evaluation
    let expr = Expr {
        kind: ExprKind::Literal(Literal::Number(42.0)),
        span: Span::dummy(),
    };
    let result = evaluator.evaluate_expr(&expr).unwrap();
    assert_eq!(result, ConstValue::Number(42.0));

    // Test binary operations
    let expr = Expr {
        kind: ExprKind::Binary {
            left: Box::new(Expr {
                kind: ExprKind::Literal(Literal::Number(10.0)),
                span: Span::dummy(),
            }),
            op: BinaryOp::Add,
            right: Box::new(Expr {
                kind: ExprKind::Literal(Literal::Number(5.0)),
                span: Span::dummy(),
            }),
        },
        span: Span::dummy(),
    };
    let result = evaluator.evaluate_expr(&expr).unwrap();
    assert_eq!(result, ConstValue::Number(15.0));

    // Test array evaluation
    let expr = Expr {
        kind: ExprKind::Array(vec![
            Expr {
                kind: ExprKind::Literal(Literal::Number(1.0)),
                span: Span::dummy(),
            },
            Expr {
                kind: ExprKind::Literal(Literal::Number(2.0)),
                span: Span::dummy(),
            },
        ]),
        span: Span::dummy(),
    };
    let result = evaluator.evaluate_expr(&expr).unwrap();
    if let ConstValue::Array(values) = result {
        assert_eq!(values.len(), 2);
        assert_eq!(values[0], ConstValue::Number(1.0));
        assert_eq!(values[1], ConstValue::Number(2.0));
    } else {
        panic!("Expected array value");
    }
}

#[test]
fn test_invalid_attribute_target() {
    let mut processor = MetaprogrammingProcessor::new();

    // @derive on a let statement should fail
    let mut let_stmt = Stmt {
        kind: StmtKind::Let {
            name: "x".to_string(),
            type_ann: None,
            init: Some(Expr {
                kind: ExprKind::Literal(Literal::Number(42.0)),
                span: Span::dummy(),
            }),
        },
        span: Span::dummy(),
        attributes: vec![Attribute {
            name: "derive".to_string(),
            args: vec!["Debug".to_string()],
            span: Span::dummy(),
        }],
    };

    let result = processor.process_statement(&mut let_stmt);
    assert!(result.is_err());
}
