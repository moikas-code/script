use super::*;
use crate::{lexer::Lexer, Result};

fn parse(input: &str) -> Result<Program> {
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.scan_tokens();
    
    if !errors.is_empty() {
        return Err(errors.into_iter().next().unwrap());
    }
    
    let mut parser = Parser::new(tokens);
    parser.parse()
}

fn parse_expr(input: &str) -> Result<Expr> {
    let lexer = Lexer::new(input);
    let (tokens, _) = lexer.scan_tokens();
    let mut parser = Parser::new(tokens);
    parser.parse_expression()
}

#[test]
fn test_parse_literals() {
    // Numbers
    let expr = parse_expr("42").unwrap();
    assert!(matches!(expr.kind, ExprKind::Literal(Literal::Number(n)) if n == 42.0));
    
    let expr = parse_expr("3.14").unwrap();
    assert!(matches!(expr.kind, ExprKind::Literal(Literal::Number(n)) if n == 3.14));
    
    // Strings
    let expr = parse_expr(r#""hello world""#).unwrap();
    assert!(matches!(expr.kind, ExprKind::Literal(Literal::String(ref s)) if s == "hello world"));
    
    // Booleans
    let expr = parse_expr("true").unwrap();
    assert!(matches!(expr.kind, ExprKind::Literal(Literal::Boolean(true))));
    
    let expr = parse_expr("false").unwrap();
    assert!(matches!(expr.kind, ExprKind::Literal(Literal::Boolean(false))));
}

#[test]
fn test_parse_identifiers() {
    let expr = parse_expr("foo").unwrap();
    assert!(matches!(expr.kind, ExprKind::Identifier(ref s) if s == "foo"));
    
    let expr = parse_expr("_bar123").unwrap();
    assert!(matches!(expr.kind, ExprKind::Identifier(ref s) if s == "_bar123"));
}

#[test]
fn test_parse_binary_expressions() {
    // Arithmetic
    let expr = parse_expr("1 + 2").unwrap();
    match &expr.kind {
        ExprKind::Binary { left, op, right } => {
            assert!(matches!(left.kind, ExprKind::Literal(Literal::Number(1.0))));
            assert_eq!(*op, BinaryOp::Add);
            assert!(matches!(right.kind, ExprKind::Literal(Literal::Number(2.0))));
        }
        _ => panic!("Expected binary expression"),
    }
    
    // Precedence
    let expr = parse_expr("1 + 2 * 3").unwrap();
    match &expr.kind {
        ExprKind::Binary { left, op, right } => {
            assert!(matches!(left.kind, ExprKind::Literal(Literal::Number(1.0))));
            assert_eq!(*op, BinaryOp::Add);
            
            match &right.kind {
                ExprKind::Binary { left, op, right } => {
                    assert!(matches!(left.kind, ExprKind::Literal(Literal::Number(2.0))));
                    assert_eq!(*op, BinaryOp::Multiply);
                    assert!(matches!(right.kind, ExprKind::Literal(Literal::Number(3.0))));
                }
                _ => panic!("Expected binary expression"),
            }
        }
        _ => panic!("Expected binary expression"),
    }
}

#[test]
fn test_parse_unary_expressions() {
    let expr = parse_expr("-42").unwrap();
    match &expr.kind {
        ExprKind::Unary { op, expr } => {
            assert_eq!(*op, UnaryOp::Negate);
            assert!(matches!(expr.kind, ExprKind::Literal(Literal::Number(42.0))));
        }
        _ => panic!("Expected unary expression"),
    }
    
    let expr = parse_expr("!true").unwrap();
    match &expr.kind {
        ExprKind::Unary { op, expr } => {
            assert_eq!(*op, UnaryOp::Not);
            assert!(matches!(expr.kind, ExprKind::Literal(Literal::Boolean(true))));
        }
        _ => panic!("Expected unary expression"),
    }
}

#[test]
fn test_parse_grouped_expressions() {
    let expr = parse_expr("(1 + 2) * 3").unwrap();
    match &expr.kind {
        ExprKind::Binary { left, op, right } => {
            assert_eq!(*op, BinaryOp::Multiply);
            
            match &left.kind {
                ExprKind::Binary { left, op, right } => {
                    assert!(matches!(left.kind, ExprKind::Literal(Literal::Number(1.0))));
                    assert_eq!(*op, BinaryOp::Add);
                    assert!(matches!(right.kind, ExprKind::Literal(Literal::Number(2.0))));
                }
                _ => panic!("Expected binary expression"),
            }
            
            assert!(matches!(right.kind, ExprKind::Literal(Literal::Number(3.0))));
        }
        _ => panic!("Expected binary expression"),
    }
}

#[test]
fn test_parse_call_expressions() {
    let expr = parse_expr("foo()").unwrap();
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            assert!(matches!(callee.kind, ExprKind::Identifier(ref s) if s == "foo"));
            assert!(args.is_empty());
        }
        _ => panic!("Expected call expression"),
    }
    
    let expr = parse_expr("add(1, 2, 3)").unwrap();
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            assert!(matches!(callee.kind, ExprKind::Identifier(ref s) if s == "add"));
            assert_eq!(args.len(), 3);
        }
        _ => panic!("Expected call expression"),
    }
}

#[test]
fn test_parse_array_expressions() {
    let expr = parse_expr("[]").unwrap();
    match &expr.kind {
        ExprKind::Array(elements) => {
            assert!(elements.is_empty());
        }
        _ => panic!("Expected array expression"),
    }
    
    let expr = parse_expr("[1, 2, 3]").unwrap();
    match &expr.kind {
        ExprKind::Array(elements) => {
            assert_eq!(elements.len(), 3);
            assert!(matches!(elements[0].kind, ExprKind::Literal(Literal::Number(1.0))));
            assert!(matches!(elements[1].kind, ExprKind::Literal(Literal::Number(2.0))));
            assert!(matches!(elements[2].kind, ExprKind::Literal(Literal::Number(3.0))));
        }
        _ => panic!("Expected array expression"),
    }
}

#[test]
fn test_parse_member_and_index_expressions() {
    let expr = parse_expr("foo.bar").unwrap();
    match &expr.kind {
        ExprKind::Member { object, property } => {
            assert!(matches!(object.kind, ExprKind::Identifier(ref s) if s == "foo"));
            assert_eq!(property, "bar");
        }
        _ => panic!("Expected member expression"),
    }
    
    let expr = parse_expr("arr[0]").unwrap();
    match &expr.kind {
        ExprKind::Index { object, index } => {
            assert!(matches!(object.kind, ExprKind::Identifier(ref s) if s == "arr"));
            assert!(matches!(index.kind, ExprKind::Literal(Literal::Number(0.0))));
        }
        _ => panic!("Expected index expression"),
    }
    
    // Chained
    let expr = parse_expr("foo.bar[0].baz").unwrap();
    match &expr.kind {
        ExprKind::Member { object, property } => {
            assert_eq!(property, "baz");
            
            match &object.kind {
                ExprKind::Index { object, index } => {
                    assert!(matches!(index.kind, ExprKind::Literal(Literal::Number(0.0))));
                    
                    match &object.kind {
                        ExprKind::Member { object, property } => {
                            assert!(matches!(object.kind, ExprKind::Identifier(ref s) if s == "foo"));
                            assert_eq!(property, "bar");
                        }
                        _ => panic!("Expected member expression"),
                    }
                }
                _ => panic!("Expected index expression"),
            }
        }
        _ => panic!("Expected member expression"),
    }
}

#[test]
fn test_parse_if_expressions() {
    let expr = parse_expr("if true { 1 } else { 2 }").unwrap();
    match &expr.kind {
        ExprKind::If { condition, then_branch, else_branch } => {
            assert!(matches!(condition.kind, ExprKind::Literal(Literal::Boolean(true))));
            
            // Check then branch
            match &then_branch.kind {
                ExprKind::Block(block) => {
                    assert!(block.statements.is_empty());
                    assert!(matches!(block.final_expr.as_ref().unwrap().kind, ExprKind::Literal(Literal::Number(1.0))));
                }
                _ => panic!("Expected block expression in then branch"),
            }
            
            // Check else branch
            match &else_branch.as_ref().unwrap().kind {
                ExprKind::Block(block) => {
                    assert!(block.statements.is_empty());
                    assert!(matches!(block.final_expr.as_ref().unwrap().kind, ExprKind::Literal(Literal::Number(2.0))));
                }
                _ => panic!("Expected block expression in else branch"),
            }
        }
        _ => panic!("Expected if expression"),
    }
}

#[test]
fn test_parse_let_statement() {
    let program = parse("let x = 42").unwrap();
    assert_eq!(program.statements.len(), 1);
    
    match &program.statements[0].kind {
        StmtKind::Let { name, type_ann, init } => {
            assert_eq!(name, "x");
            assert!(type_ann.is_none());
            assert!(matches!(init.as_ref().unwrap().kind, ExprKind::Literal(Literal::Number(42.0))));
        }
        _ => panic!("Expected let statement"),
    }
    
    // With type annotation
    let program = parse("let x: i32 = 42").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { name, type_ann, init } => {
            assert_eq!(name, "x");
            assert!(matches!(type_ann.as_ref().unwrap().kind, TypeKind::Named(ref s) if s == "i32"));
            assert!(matches!(init.as_ref().unwrap().kind, ExprKind::Literal(Literal::Number(42.0))));
        }
        _ => panic!("Expected let statement"),
    }
}

#[test]
fn test_parse_function_statement() {
    let program = parse("fn add(a: i32, b: i32) -> i32 { a + b }").unwrap();
    assert_eq!(program.statements.len(), 1);
    
    match &program.statements[0].kind {
        StmtKind::Function { name, params, ret_type, body } => {
            assert_eq!(name, "add");
            assert_eq!(params.len(), 2);
            assert_eq!(params[0].name, "a");
            assert_eq!(params[1].name, "b");
            assert!(matches!(ret_type.as_ref().unwrap().kind, TypeKind::Named(ref s) if s == "i32"));
            assert!(body.statements.is_empty());
            assert!(body.final_expr.is_some());
        }
        _ => panic!("Expected function statement"),
    }
}

#[test]
fn test_parse_while_statement() {
    let program = parse("while x < 10 { x = x + 1 }").unwrap();
    assert_eq!(program.statements.len(), 1);
    
    match &program.statements[0].kind {
        StmtKind::While { condition, body } => {
            match &condition.kind {
                ExprKind::Binary { left, op, right } => {
                    assert!(matches!(left.kind, ExprKind::Identifier(ref s) if s == "x"));
                    assert_eq!(*op, BinaryOp::Less);
                    assert!(matches!(right.kind, ExprKind::Literal(Literal::Number(10.0))));
                }
                _ => panic!("Expected binary expression"),
            }
            // The body should have the assignment as a final expression, not a statement
            assert_eq!(body.statements.len(), 0);
            assert!(body.final_expr.is_some());
        }
        _ => panic!("Expected while statement"),
    }
}

#[test]
fn test_parse_return_statement() {
    let program = parse("return 42").unwrap();
    assert_eq!(program.statements.len(), 1);
    
    match &program.statements[0].kind {
        StmtKind::Return(expr) => {
            assert!(matches!(expr.as_ref().unwrap().kind, ExprKind::Literal(Literal::Number(42.0))));
        }
        _ => panic!("Expected return statement"),
    }
    
    // Return without value
    let program = parse("return").unwrap();
    match &program.statements[0].kind {
        StmtKind::Return(expr) => {
            assert!(expr.is_none());
        }
        _ => panic!("Expected return statement"),
    }
}

#[test]
fn test_parse_assignment() {
    let expr = parse_expr("x = 42").unwrap();
    match &expr.kind {
        ExprKind::Assign { target, value } => {
            assert!(matches!(target.kind, ExprKind::Identifier(ref s) if s == "x"));
            assert!(matches!(value.kind, ExprKind::Literal(Literal::Number(42.0))));
        }
        _ => panic!("Expected assignment expression"),
    }
    
    // Chained assignment
    let expr = parse_expr("x = y = 42").unwrap();
    match &expr.kind {
        ExprKind::Assign { target, value } => {
            assert!(matches!(target.kind, ExprKind::Identifier(ref s) if s == "x"));
            
            match &value.kind {
                ExprKind::Assign { target, value } => {
                    assert!(matches!(target.kind, ExprKind::Identifier(ref s) if s == "y"));
                    assert!(matches!(value.kind, ExprKind::Literal(Literal::Number(42.0))));
                }
                _ => panic!("Expected assignment expression"),
            }
        }
        _ => panic!("Expected assignment expression"),
    }
}

#[test]
fn test_parse_block_expression() {
    let expr = parse_expr("{ let x = 1; x + 1 }").unwrap();
    match &expr.kind {
        ExprKind::Block(block) => {
            assert_eq!(block.statements.len(), 1);
            assert!(block.final_expr.is_some());
        }
        _ => panic!("Expected block expression"),
    }
}

#[test]
fn test_parse_complex_expression() {
    let input = "foo(1 + 2 * 3, bar[0].baz) && !false || true";
    let expr = parse_expr(input).unwrap();
    
    // This should parse as: (foo(...) && !false) || true
    match &expr.kind {
        ExprKind::Binary { left, op, right } => {
            assert_eq!(*op, BinaryOp::Or);
            assert!(matches!(right.kind, ExprKind::Literal(Literal::Boolean(true))));
            
            match &left.kind {
                ExprKind::Binary { left, op, right } => {
                    assert_eq!(*op, BinaryOp::And);
                    
                    // Check the call expression
                    match &left.kind {
                        ExprKind::Call { callee, args } => {
                            assert!(matches!(callee.kind, ExprKind::Identifier(ref s) if s == "foo"));
                            assert_eq!(args.len(), 2);
                        }
                        _ => panic!("Expected call expression"),
                    }
                    
                    // Check !false
                    match &right.kind {
                        ExprKind::Unary { op, expr } => {
                            assert_eq!(*op, UnaryOp::Not);
                            assert!(matches!(expr.kind, ExprKind::Literal(Literal::Boolean(false))));
                        }
                        _ => panic!("Expected unary expression"),
                    }
                }
                _ => panic!("Expected binary expression"),
            }
        }
        _ => panic!("Expected binary expression"),
    }
}

#[test]
fn test_parse_type_annotations() {
    // Simple type
    let program = parse("let x: i32").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => {
            assert!(matches!(type_ann.as_ref().unwrap().kind, TypeKind::Named(ref s) if s == "i32"));
        }
        _ => panic!("Expected let statement"),
    }
    
    // Array type
    let program = parse("let arr: [i32]").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => {
            match &type_ann.as_ref().unwrap().kind {
                TypeKind::Array(elem) => {
                    assert!(matches!(elem.kind, TypeKind::Named(ref s) if s == "i32"));
                }
                _ => panic!("Expected array type"),
            }
        }
        _ => panic!("Expected let statement"),
    }
    
    // Function type
    let program = parse("let f: (i32, i32) -> i32").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => {
            match &type_ann.as_ref().unwrap().kind {
                TypeKind::Function { params, ret } => {
                    assert_eq!(params.len(), 2);
                    assert!(matches!(ret.kind, TypeKind::Named(ref s) if s == "i32"));
                }
                _ => panic!("Expected function type"),
            }
        }
        _ => panic!("Expected let statement"),
    }
}