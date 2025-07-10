use super::*;
use crate::{lexer::Lexer, Result};

fn parse(input: &str) -> Result<Program> {
    let lexer = Lexer::new(input)?;
    let (tokens, errors) = lexer.scan_tokens();

    if !errors.is_empty() {
        return Err(errors.into_iter().next().unwrap());
    }

    let mut parser = Parser::new(tokens);
    parser.parse()
}

fn parse_expr(input: &str) -> Result<Expr> {
    let lexer = Lexer::new(input)?;
    let (tokens, _) = lexer.scan_tokens();
    let mut parser = Parser::new(tokens);
    parser.parse_expression()
}

#[test]
fn test_parse_impl_blocks() {
    // Simple impl block
    let program = parse(
        r#"
        struct Point { x: i32, y: i32 }
        
        impl Point {
            fn new(x: i32, y: i32) -> Point {
                Point { x: x, y: y }
            }
        }
    "#,
    )
    .unwrap();

    assert_eq!(program.statements.len(), 2);

    // Check that the second statement is an impl block
    match &program.statements[1].kind {
        StmtKind::Impl(impl_block) => {
            assert_eq!(impl_block.type_name, "Point");
            assert!(impl_block.generic_params.is_none());
            assert!(impl_block.where_clause.is_none());
            assert_eq!(impl_block.methods.len(), 1);

            // Check the method
            let method = &impl_block.methods[0];
            assert_eq!(method.name, "new");
            assert!(!method.is_async);
            assert_eq!(method.params.len(), 2);
        }
        _ => panic!("Expected impl block"),
    }
}

#[test]
fn test_parse_generic_impl_block() {
    let program = parse(
        r#"
        struct Vec<T> { items: [T] }
        
        impl<T> Vec<T> {
            fn new() -> Vec<T> {
                Vec { items: [] }
            }
        }
    "#,
    )
    .unwrap();

    assert_eq!(program.statements.len(), 2);

    match &program.statements[1].kind {
        StmtKind::Impl(impl_block) => {
            assert_eq!(impl_block.type_name, "Vec");

            // Check generic params
            assert!(impl_block.generic_params.is_some());
            let generics = impl_block.generic_params.as_ref().unwrap();
            assert_eq!(generics.params.len(), 1);
            assert_eq!(generics.params[0].name, "T");
        }
        _ => panic!("Expected impl block"),
    }
}

#[test]
fn test_parse_impl_with_where_clause() {
    let program = parse(
        r#"
        struct Container<T> { value: T }
        
        impl<T> Container<T> where T: Clone {
            fn clone_value(self) -> T {
                self.value.clone()
            }
        }
    "#,
    )
    .unwrap();

    match &program.statements[1].kind {
        StmtKind::Impl(impl_block) => {
            assert!(impl_block.where_clause.is_some());
            let where_clause = impl_block.where_clause.as_ref().unwrap();
            assert_eq!(where_clause.predicates.len(), 1);

            let predicate = &where_clause.predicates[0];
            assert_eq!(predicate.bounds.len(), 1);
            assert_eq!(predicate.bounds[0].trait_name, "Clone");
        }
        _ => panic!("Expected impl block"),
    }
}

#[test]
fn test_parse_async_methods() {
    let program = parse(
        r#"
        struct Worker {}
        
        impl Worker {
            async fn do_work(self) -> Result<string> {
                await something()
            }
        }
    "#,
    )
    .unwrap();

    match &program.statements[1].kind {
        StmtKind::Impl(impl_block) => {
            let method = &impl_block.methods[0];
            assert!(method.is_async);
            assert_eq!(method.name, "do_work");
        }
        _ => panic!("Expected impl block"),
    }
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
    assert!(matches!(
        expr.kind,
        ExprKind::Literal(Literal::Boolean(true))
    ));

    let expr = parse_expr("false").unwrap();
    assert!(matches!(
        expr.kind,
        ExprKind::Literal(Literal::Boolean(false))
    ));
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
            assert!(matches!(
                right.kind,
                ExprKind::Literal(Literal::Number(2.0))
            ));
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
                    assert_eq!(*op, BinaryOp::Mul);
                    assert!(matches!(
                        right.kind,
                        ExprKind::Literal(Literal::Number(3.0))
                    ));
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
            assert_eq!(*op, UnaryOp::Minus);
            assert!(matches!(
                expr.kind,
                ExprKind::Literal(Literal::Number(42.0))
            ));
        }
        _ => panic!("Expected unary expression"),
    }

    let expr = parse_expr("!true").unwrap();
    match &expr.kind {
        ExprKind::Unary { op, expr } => {
            assert_eq!(*op, UnaryOp::Not);
            assert!(matches!(
                expr.kind,
                ExprKind::Literal(Literal::Boolean(true))
            ));
        }
        _ => panic!("Expected unary expression"),
    }
}

#[test]
fn test_parse_grouped_expressions() {
    let expr = parse_expr("(1 + 2) * 3").unwrap();
    match &expr.kind {
        ExprKind::Binary { left, op, right } => {
            assert_eq!(*op, BinaryOp::Mul);

            match &left.kind {
                ExprKind::Binary { left, op, right } => {
                    assert!(matches!(left.kind, ExprKind::Literal(Literal::Number(1.0))));
                    assert_eq!(*op, BinaryOp::Add);
                    assert!(matches!(
                        right.kind,
                        ExprKind::Literal(Literal::Number(2.0))
                    ));
                }
                _ => panic!("Expected binary expression"),
            }

            assert!(matches!(
                right.kind,
                ExprKind::Literal(Literal::Number(3.0))
            ));
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
            assert!(matches!(
                elements[0].kind,
                ExprKind::Literal(Literal::Number(1.0))
            ));
            assert!(matches!(
                elements[1].kind,
                ExprKind::Literal(Literal::Number(2.0))
            ));
            assert!(matches!(
                elements[2].kind,
                ExprKind::Literal(Literal::Number(3.0))
            ));
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
            assert!(matches!(
                index.kind,
                ExprKind::Literal(Literal::Number(0.0))
            ));
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
                    assert!(matches!(
                        index.kind,
                        ExprKind::Literal(Literal::Number(0.0))
                    ));

                    match &object.kind {
                        ExprKind::Member { object, property } => {
                            assert!(
                                matches!(object.kind, ExprKind::Identifier(ref s) if s == "foo")
                            );
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
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            assert!(matches!(
                condition.kind,
                ExprKind::Literal(Literal::Boolean(true))
            ));

            // Check then branch
            match &then_branch.kind {
                ExprKind::Block(block) => {
                    assert!(block.statements.is_empty());
                    assert!(matches!(
                        block.final_expr.as_ref().unwrap().kind,
                        ExprKind::Literal(Literal::Number(1.0))
                    ));
                }
                _ => panic!("Expected block expression in then branch"),
            }

            // Check else branch
            match &else_branch.as_ref().unwrap().kind {
                ExprKind::Block(block) => {
                    assert!(block.statements.is_empty());
                    assert!(matches!(
                        block.final_expr.as_ref().unwrap().kind,
                        ExprKind::Literal(Literal::Number(2.0))
                    ));
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
        StmtKind::Let {
            name,
            type_ann,
            init,
        } => {
            assert_eq!(name, "x");
            assert!(type_ann.is_none());
            assert!(matches!(
                init.as_ref().unwrap().kind,
                ExprKind::Literal(Literal::Number(42.0))
            ));
        }
        _ => panic!("Expected let statement"),
    }

    // With type annotation
    let program = parse("let x: i32 = 42").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let {
            name,
            type_ann,
            init,
        } => {
            assert_eq!(name, "x");
            assert!(
                matches!(type_ann.as_ref().unwrap().kind, TypeKind::Named(ref s) if s == "i32")
            );
            assert!(matches!(
                init.as_ref().unwrap().kind,
                ExprKind::Literal(Literal::Number(42.0))
            ));
        }
        _ => panic!("Expected let statement"),
    }
}

#[test]
fn test_parse_function_statement() {
    let program = parse("fn add(a: i32, b: i32) -> i32 { a + b }").unwrap();
    assert_eq!(program.statements.len(), 1);

    match &program.statements[0].kind {
        StmtKind::Function {
            name,
            params,
            ret_type,
            body,
            is_async: _,
            generic_params: _,
            where_clause: _,
        } => {
            assert_eq!(name, "add");
            assert_eq!(params.len(), 2);
            assert_eq!(params[0].name, "a");
            assert_eq!(params[1].name, "b");
            assert!(
                matches!(ret_type.as_ref().unwrap().kind, TypeKind::Named(ref s) if s == "i32")
            );
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
                    assert!(matches!(
                        right.kind,
                        ExprKind::Literal(Literal::Number(10.0))
                    ));
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
            assert!(matches!(
                expr.as_ref().unwrap().kind,
                ExprKind::Literal(Literal::Number(42.0))
            ));
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
            assert!(matches!(
                value.kind,
                ExprKind::Literal(Literal::Number(42.0))
            ));
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
                    assert!(matches!(
                        value.kind,
                        ExprKind::Literal(Literal::Number(42.0))
                    ));
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
            assert!(matches!(
                right.kind,
                ExprKind::Literal(Literal::Boolean(true))
            ));

            match &left.kind {
                ExprKind::Binary { left, op, right } => {
                    assert_eq!(*op, BinaryOp::And);

                    // Check the call expression
                    match &left.kind {
                        ExprKind::Call { callee, args } => {
                            assert!(
                                matches!(callee.kind, ExprKind::Identifier(ref s) if s == "foo")
                            );
                            assert_eq!(args.len(), 2);
                        }
                        _ => panic!("Expected call expression"),
                    }

                    // Check !false
                    match &right.kind {
                        ExprKind::Unary { op, expr } => {
                            assert_eq!(*op, UnaryOp::Not);
                            assert!(matches!(
                                expr.kind,
                                ExprKind::Literal(Literal::Boolean(false))
                            ));
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
            assert!(
                matches!(type_ann.as_ref().unwrap().kind, TypeKind::Named(ref s) if s == "i32")
            );
        }
        _ => panic!("Expected let statement"),
    }

    // Array type
    let program = parse("let arr: [i32]").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => match &type_ann.as_ref().unwrap().kind {
            TypeKind::Array(elem) => {
                assert!(matches!(elem.kind, TypeKind::Named(ref s) if s == "i32"));
            }
            _ => panic!("Expected array type"),
        },
        _ => panic!("Expected let statement"),
    }

    // Function type
    let program = parse("let f: (i32, i32) -> i32").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => match &type_ann.as_ref().unwrap().kind {
            TypeKind::Function { params, ret } => {
                assert_eq!(params.len(), 2);
                assert!(matches!(ret.kind, TypeKind::Named(ref s) if s == "i32"));
            }
            _ => panic!("Expected function type"),
        },
        _ => panic!("Expected let statement"),
    }
}

#[test]
fn test_parse_import_named() {
    let program = parse("import { foo, bar } from \"./module\"").unwrap();
    assert_eq!(program.statements.len(), 1);

    match &program.statements[0].kind {
        StmtKind::Import { imports, module } => {
            assert_eq!(module, "./module");
            assert_eq!(imports.len(), 2);

            match &imports[0] {
                ImportSpecifier::Named { name, alias } => {
                    assert_eq!(name, "foo");
                    assert_eq!(alias, &None);
                }
                _ => panic!("Expected named import"),
            }

            match &imports[1] {
                ImportSpecifier::Named { name, alias } => {
                    assert_eq!(name, "bar");
                    assert_eq!(alias, &None);
                }
                _ => panic!("Expected named import"),
            }
        }
        _ => panic!("Expected import statement"),
    }
}

#[test]
fn test_parse_import_aliased() {
    let program = parse("import { foo as f, bar as b } from \"./module\"").unwrap();
    assert_eq!(program.statements.len(), 1);

    match &program.statements[0].kind {
        StmtKind::Import { imports, module } => {
            assert_eq!(module, "./module");
            assert_eq!(imports.len(), 2);

            match &imports[0] {
                ImportSpecifier::Named { name, alias } => {
                    assert_eq!(name, "foo");
                    assert_eq!(alias, &Some("f".to_string()));
                }
                _ => panic!("Expected named import"),
            }

            match &imports[1] {
                ImportSpecifier::Named { name, alias } => {
                    assert_eq!(name, "bar");
                    assert_eq!(alias, &Some("b".to_string()));
                }
                _ => panic!("Expected named import"),
            }
        }
        _ => panic!("Expected import statement"),
    }
}

#[test]
fn test_parse_import_namespace() {
    let program = parse("import * as utils from \"./utils\"").unwrap();
    assert_eq!(program.statements.len(), 1);

    match &program.statements[0].kind {
        StmtKind::Import { imports, module } => {
            assert_eq!(module, "./utils");
            assert_eq!(imports.len(), 1);

            match &imports[0] {
                ImportSpecifier::Namespace { alias } => {
                    assert_eq!(alias, "utils");
                }
                _ => panic!("Expected namespace import"),
            }
        }
        _ => panic!("Expected import statement"),
    }
}

#[test]
fn test_parse_import_mixed_aliasing() {
    // Test mixing aliased and non-aliased imports in the same statement
    let program = parse("import { foo, bar as baz } from \"./module\"").unwrap();
    assert_eq!(program.statements.len(), 1);

    match &program.statements[0].kind {
        StmtKind::Import { imports, module } => {
            assert_eq!(module, "./module");
            assert_eq!(imports.len(), 2);

            match &imports[0] {
                ImportSpecifier::Named { name, alias } => {
                    assert_eq!(name, "foo");
                    assert!(alias.is_none());
                }
                _ => panic!("Expected named import"),
            }

            match &imports[1] {
                ImportSpecifier::Named { name, alias } => {
                    assert_eq!(name, "bar");
                    assert_eq!(alias.as_ref(), Some(&"baz".to_string()));
                }
                _ => panic!("Expected named import with alias"),
            }
        }
        _ => panic!("Expected import statement"),
    }
}

#[test]
fn test_parse_import_default() {
    let program = parse("import defaultExport from \"./module\"").unwrap();
    assert_eq!(program.statements.len(), 1);

    match &program.statements[0].kind {
        StmtKind::Import { imports, module } => {
            assert_eq!(module, "./module");
            assert_eq!(imports.len(), 1);

            match &imports[0] {
                ImportSpecifier::Default { name } => {
                    assert_eq!(name, "defaultExport");
                }
                _ => panic!("Expected default import"),
            }
        }
        _ => panic!("Expected import statement"),
    }
}

#[test]
fn test_parse_export_named() {
    let program = parse("export { foo, bar as baz }").unwrap();
    assert_eq!(program.statements.len(), 1);

    match &program.statements[0].kind {
        StmtKind::Export { export } => match export {
            ExportKind::Named { specifiers } => {
                assert_eq!(specifiers.len(), 2);

                assert_eq!(specifiers[0].name, "foo");
                assert_eq!(specifiers[0].alias, None);

                assert_eq!(specifiers[1].name, "bar");
                assert_eq!(specifiers[1].alias, Some("baz".to_string()));
            }
            _ => panic!("Expected named export"),
        },
        _ => panic!("Expected export statement"),
    }
}

#[test]
fn test_parse_export_function() {
    let program = parse("export fn add(a: number, b: number) -> number { a + b }").unwrap();
    assert_eq!(program.statements.len(), 1);

    match &program.statements[0].kind {
        StmtKind::Export { export } => match export {
            ExportKind::Function {
                name,
                params,
                ret_type,
                body: _,
                is_async: _,
            } => {
                assert_eq!(name, "add");
                assert_eq!(params.len(), 2);

                assert_eq!(params[0].name, "a");
                assert_eq!(params[1].name, "b");

                assert!(ret_type.is_some());
            }
            _ => panic!("Expected function export"),
        },
        _ => panic!("Expected export statement"),
    }
}

#[test]
fn test_parse_export_variable() {
    let program = parse("export let myValue: number = 42").unwrap();
    assert_eq!(program.statements.len(), 1);

    match &program.statements[0].kind {
        StmtKind::Export { export } => match export {
            ExportKind::Variable {
                name,
                type_ann,
                init,
            } => {
                assert_eq!(name, "myValue");
                assert!(type_ann.is_some());
                assert!(init.is_some());
            }
            _ => panic!("Expected variable export"),
        },
        _ => panic!("Expected export statement"),
    }
}

#[test]
fn test_parse_export_default() {
    let program = parse("export default 42").unwrap();
    assert_eq!(program.statements.len(), 1);

    match &program.statements[0].kind {
        StmtKind::Export { export } => match export {
            ExportKind::Default { expr } => match &expr.kind {
                ExprKind::Literal(Literal::Number(n)) => {
                    assert_eq!(*n, 42.0);
                }
                _ => panic!("Expected number literal"),
            },
            _ => panic!("Expected default export"),
        },
        _ => panic!("Expected export statement"),
    }
}

#[test]
fn test_parse_import_export_mixed() {
    let program = parse(
        r#"
        import { Component } from "./components"
        export fn render() { Component() }
        export default "1.0"
    "#,
    )
    .unwrap();

    assert_eq!(program.statements.len(), 3);

    // Check import
    match &program.statements[0].kind {
        StmtKind::Import { imports, module } => {
            assert_eq!(module, "./components");
            assert_eq!(imports.len(), 1);
        }
        _ => panic!("Expected import statement"),
    }

    // Check export function
    match &program.statements[1].kind {
        StmtKind::Export { export } => match export {
            ExportKind::Function { name, .. } => {
                assert_eq!(name, "render");
            }
            _ => panic!("Expected function export"),
        },
        _ => panic!("Expected export statement"),
    }

    // Check default export
    match &program.statements[2].kind {
        StmtKind::Export { export } => {
            match export {
                ExportKind::Default { .. } => {
                    // Successfully parsed default export
                }
                _ => panic!("Expected default export"),
            }
        }
        _ => panic!("Expected export statement"),
    }
}

#[test]
fn test_import_error_cases() {
    // Missing from
    assert!(parse("import { foo }").is_err());

    // Missing module path
    assert!(parse("import { foo } from").is_err());

    // Invalid namespace import
    assert!(parse("import * foo from \"./module\"").is_err());

    // Missing closing brace
    assert!(parse("import { foo from \"./module\"").is_err());
}

#[test]
fn test_export_error_cases() {
    // Invalid export syntax
    assert!(parse("export").is_err());

    // Missing function body
    assert!(parse("export fn test()").is_err());

    // Missing closing brace in named export
    assert!(parse("export { foo").is_err());
}

#[test]
fn test_module_ast_types() {
    use crate::source::{SourceLocation, Span};

    // Test creating import AST nodes directly
    let import_stmt = Stmt {
        kind: StmtKind::Import {
            imports: vec![
                ImportSpecifier::Default {
                    name: "React".to_string(),
                },
                ImportSpecifier::Named {
                    name: "useState".to_string(),
                    alias: None,
                },
                ImportSpecifier::Namespace {
                    alias: "utils".to_string(),
                },
            ],
            module: "react".to_string(),
        },
        span: Span::single(SourceLocation::initial()),
        attributes: vec![],
    };

    // Test creating export AST nodes directly
    let export_stmt = Stmt {
        kind: StmtKind::Export {
            export: ExportKind::Named {
                specifiers: vec![
                    ExportSpecifier {
                        name: "foo".to_string(),
                        alias: None,
                    },
                    ExportSpecifier {
                        name: "bar".to_string(),
                        alias: Some("baz".to_string()),
                    },
                ],
            },
        },
        span: Span::single(SourceLocation::initial()),
        attributes: vec![],
    };

    // Verify Display implementations work
    let import_str = format!("{}", import_stmt);
    assert!(import_str.contains("import"));
    assert!(import_str.contains("React"));
    assert!(import_str.contains("useState"));
    assert!(import_str.contains("* as utils"));
    assert!(import_str.contains("from \"react\""));

    let export_str = format!("{}", export_stmt);
    assert!(export_str.contains("export"));
    assert!(export_str.contains("foo"));
}

#[test]
fn test_parse_attributes() {
    // Test single attribute
    let program = parse("@derive(Debug)\nfn foo() {}").unwrap();
    assert_eq!(program.statements.len(), 1);
    let stmt = &program.statements[0];
    assert_eq!(stmt.attributes.len(), 1);
    assert_eq!(stmt.attributes[0].name, "derive");
    assert_eq!(stmt.attributes[0].args, vec!["Debug"]);

    // Test multiple attributes
    let program = parse("@derive(Debug, Serialize)\n@const\nfn bar() { 42 }").unwrap();
    assert_eq!(program.statements.len(), 1);
    let stmt = &program.statements[0];
    assert_eq!(stmt.attributes.len(), 2);
    assert_eq!(stmt.attributes[0].name, "derive");
    assert_eq!(stmt.attributes[0].args, vec!["Debug", "Serialize"]);
    assert_eq!(stmt.attributes[1].name, "const");
    assert_eq!(stmt.attributes[1].args.len(), 0);

    // Test attribute on let statement
    let program = parse("@const\nlet x = 42").unwrap();
    assert_eq!(program.statements.len(), 1);
    let stmt = &program.statements[0];
    assert_eq!(stmt.attributes.len(), 1);
    assert_eq!(stmt.attributes[0].name, "const");
}

#[test]
fn test_parse_list_comprehensions() {
    // Basic list comprehension
    let expr = parse_expr("[x * 2 for x in nums]").unwrap();
    if let ExprKind::ListComprehension {
        element,
        variable,
        iterable,
        condition,
    } = expr.kind
    {
        assert_eq!(variable, "x");
        assert!(matches!(element.kind, ExprKind::Binary { .. }));
        assert!(matches!(iterable.kind, ExprKind::Identifier(ref s) if s == "nums"));
        assert!(condition.is_none());
    } else {
        panic!("Expected list comprehension, got {:?}", expr.kind);
    }

    // List comprehension with condition
    let expr = parse_expr("[x * 2 for x in nums if x > 0]").unwrap();
    if let ExprKind::ListComprehension {
        element,
        variable,
        iterable,
        condition,
    } = expr.kind
    {
        assert_eq!(variable, "x");
        assert!(matches!(element.kind, ExprKind::Binary { .. }));
        assert!(matches!(iterable.kind, ExprKind::Identifier(ref s) if s == "nums"));
        assert!(condition.is_some());
        if let Some(cond) = condition {
            assert!(matches!(cond.kind, ExprKind::Binary { .. }));
        }
    } else {
        panic!("Expected list comprehension, got {:?}", expr.kind);
    }

    // Complex list comprehension
    let expr = parse_expr("[y + 1 for y in [1, 2, 3] if y != 2]").unwrap();
    if let ExprKind::ListComprehension {
        element,
        variable,
        iterable,
        condition,
    } = expr.kind
    {
        assert_eq!(variable, "y");
        assert!(matches!(element.kind, ExprKind::Binary { .. }));
        assert!(matches!(iterable.kind, ExprKind::Array(_)));
        assert!(condition.is_some());
    } else {
        panic!("Expected list comprehension, got {:?}", expr.kind);
    }
}

#[test]
fn test_parse_regular_arrays_vs_comprehensions() {
    // Regular array
    let expr = parse_expr("[1, 2, 3]").unwrap();
    assert!(matches!(expr.kind, ExprKind::Array(ref elements) if elements.len() == 3));

    // Empty array
    let expr = parse_expr("[]").unwrap();
    assert!(matches!(expr.kind, ExprKind::Array(ref elements) if elements.is_empty()));

    // Array with trailing comma
    let expr = parse_expr("[1, 2, 3,]").unwrap();
    assert!(matches!(expr.kind, ExprKind::Array(ref elements) if elements.len() == 3));
}

// ========== GENERIC TYPE PARSING TESTS ==========

#[test]
fn test_parse_generic_type_basic() {
    // Basic generic type with single argument
    let program = parse("let v: Vec<i32>").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => {
            let type_ann = type_ann.as_ref().unwrap();
            match &type_ann.kind {
                TypeKind::Generic { name, args } => {
                    assert_eq!(name, "Vec");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0].kind, TypeKind::Named(n) if n == "i32"));
                }
                _ => panic!("Expected generic type, got {:?}", type_ann.kind),
            }
        }
        _ => panic!("Expected let statement"),
    }
}

#[test]
fn test_parse_generic_type_multiple_args() {
    // Generic type with multiple type arguments
    let program = parse("let map: HashMap<String, i32>").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => {
            let type_ann = type_ann.as_ref().unwrap();
            match &type_ann.kind {
                TypeKind::Generic { name, args } => {
                    assert_eq!(name, "HashMap");
                    assert_eq!(args.len(), 2);
                    assert!(matches!(&args[0].kind, TypeKind::Named(n) if n == "String"));
                    assert!(matches!(&args[1].kind, TypeKind::Named(n) if n == "i32"));
                }
                _ => panic!("Expected generic type"),
            }
        }
        _ => panic!("Expected let statement"),
    }
}

#[test]
fn test_parse_generic_type_with_type_params() {
    // Generic type with type parameters
    let program = parse("let result: Result<T, E>").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => {
            let type_ann = type_ann.as_ref().unwrap();
            match &type_ann.kind {
                TypeKind::Generic { name, args } => {
                    assert_eq!(name, "Result");
                    assert_eq!(args.len(), 2);
                    assert!(matches!(&args[0].kind, TypeKind::TypeParam(n) if n == "T"));
                    assert!(matches!(&args[1].kind, TypeKind::TypeParam(n) if n == "E"));
                }
                _ => panic!("Expected generic type"),
            }
        }
        _ => panic!("Expected let statement"),
    }
}

#[test]
fn test_parse_tuple_type_basic() {
    // Basic tuple type
    let program = parse("let point: (i32, i32)").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => {
            let type_ann = type_ann.as_ref().unwrap();
            match &type_ann.kind {
                TypeKind::Tuple(types) => {
                    assert_eq!(types.len(), 2);
                    assert!(matches!(&types[0].kind, TypeKind::Named(n) if n == "i32"));
                    assert!(matches!(&types[1].kind, TypeKind::Named(n) if n == "i32"));
                }
                _ => panic!("Expected tuple type, got {:?}", type_ann.kind),
            }
        }
        _ => panic!("Expected let statement"),
    }
}

#[test]
fn test_parse_tuple_type_multiple() {
    // Tuple with multiple types
    let program = parse("let data: (string, bool, f32)").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => {
            let type_ann = type_ann.as_ref().unwrap();
            match &type_ann.kind {
                TypeKind::Tuple(types) => {
                    assert_eq!(types.len(), 3);
                    assert!(matches!(&types[0].kind, TypeKind::Named(n) if n == "string"));
                    assert!(matches!(&types[1].kind, TypeKind::Named(n) if n == "bool"));
                    assert!(matches!(&types[2].kind, TypeKind::Named(n) if n == "f32"));
                }
                _ => panic!("Expected tuple type"),
            }
        }
        _ => panic!("Expected let statement"),
    }
}

#[test]
fn test_parse_tuple_type_generic() {
    // Tuple with generic types
    let program = parse("let pair: (T, U)").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => {
            let type_ann = type_ann.as_ref().unwrap();
            match &type_ann.kind {
                TypeKind::Tuple(types) => {
                    assert_eq!(types.len(), 2);
                    assert!(matches!(&types[0].kind, TypeKind::TypeParam(n) if n == "T"));
                    assert!(matches!(&types[1].kind, TypeKind::TypeParam(n) if n == "U"));
                }
                _ => panic!("Expected tuple type"),
            }
        }
        _ => panic!("Expected let statement"),
    }
}

#[test]
fn test_parse_reference_type() {
    // Immutable reference
    let program = parse("let x: &i32").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => {
            let type_ann = type_ann.as_ref().unwrap();
            match &type_ann.kind {
                TypeKind::Reference { mutable, inner } => {
                    assert!(!mutable);
                    assert!(matches!(&inner.kind, TypeKind::Named(n) if n == "i32"));
                }
                _ => panic!("Expected reference type, got {:?}", type_ann.kind),
            }
        }
        _ => panic!("Expected let statement"),
    }
}

#[test]
fn test_parse_mutable_reference_type() {
    // Mutable reference
    let program = parse("let x: &mut string").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => {
            let type_ann = type_ann.as_ref().unwrap();
            match &type_ann.kind {
                TypeKind::Reference { mutable, inner } => {
                    assert!(mutable);
                    assert!(matches!(&inner.kind, TypeKind::Named(n) if n == "string"));
                }
                _ => panic!("Expected reference type"),
            }
        }
        _ => panic!("Expected let statement"),
    }
}

#[test]
fn test_parse_complex_tuple_reference() {
    // Complex type: reference to tuple containing generics
    let program = parse("let x: &(Vec<T>, Option<U>)").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => {
            let type_ann = type_ann.as_ref().unwrap();
            match &type_ann.kind {
                TypeKind::Reference { mutable, inner } => {
                    assert!(!mutable);
                    match &inner.kind {
                        TypeKind::Tuple(types) => {
                            assert_eq!(types.len(), 2);

                            // Check Vec<T>
                            match &types[0].kind {
                                TypeKind::Generic { name, args } => {
                                    assert_eq!(name, "Vec");
                                    assert_eq!(args.len(), 1);
                                    assert!(
                                        matches!(&args[0].kind, TypeKind::TypeParam(n) if n == "T")
                                    );
                                }
                                _ => panic!("Expected generic Vec<T>"),
                            }

                            // Check Option<U>
                            match &types[1].kind {
                                TypeKind::Generic { name, args } => {
                                    assert_eq!(name, "Option");
                                    assert_eq!(args.len(), 1);
                                    assert!(
                                        matches!(&args[0].kind, TypeKind::TypeParam(n) if n == "U")
                                    );
                                }
                                _ => panic!("Expected generic Option<U>"),
                            }
                        }
                        _ => panic!("Expected tuple type inside reference"),
                    }
                }
                _ => panic!("Expected reference type"),
            }
        }
        _ => panic!("Expected let statement"),
    }
}

#[test]
fn test_parse_function_vs_tuple_types() {
    // Function type (has arrow)
    let program = parse("let f: (i32, i32) -> i32").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => {
            let type_ann = type_ann.as_ref().unwrap();
            match &type_ann.kind {
                TypeKind::Function { params, ret } => {
                    assert_eq!(params.len(), 2);
                    assert!(matches!(&params[0].kind, TypeKind::Named(n) if n == "i32"));
                    assert!(matches!(&params[1].kind, TypeKind::Named(n) if n == "i32"));
                    assert!(matches!(&ret.kind, TypeKind::Named(n) if n == "i32"));
                }
                _ => panic!("Expected function type, got {:?}", type_ann.kind),
            }
        }
        _ => panic!("Expected let statement"),
    }

    // Tuple type (no arrow)
    let program = parse("let t: (i32, i32)").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => {
            let type_ann = type_ann.as_ref().unwrap();
            match &type_ann.kind {
                TypeKind::Tuple(types) => {
                    assert_eq!(types.len(), 2);
                    assert!(matches!(&types[0].kind, TypeKind::Named(n) if n == "i32"));
                    assert!(matches!(&types[1].kind, TypeKind::Named(n) if n == "i32"));
                }
                _ => panic!("Expected tuple type, got {:?}", type_ann.kind),
            }
        }
        _ => panic!("Expected let statement"),
    }
}

#[test]
fn test_parse_nested_generic_types() {
    // Nested generic types
    let program = parse("let opt: Option<Vec<i32>>").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => {
            let type_ann = type_ann.as_ref().unwrap();
            match &type_ann.kind {
                TypeKind::Generic { name, args } => {
                    assert_eq!(name, "Option");
                    assert_eq!(args.len(), 1);

                    // Check nested Vec<i32>
                    match &args[0].kind {
                        TypeKind::Generic { name, args } => {
                            assert_eq!(name, "Vec");
                            assert_eq!(args.len(), 1);
                            assert!(matches!(&args[0].kind, TypeKind::Named(n) if n == "i32"));
                        }
                        _ => panic!("Expected nested generic type"),
                    }
                }
                _ => panic!("Expected generic type"),
            }
        }
        _ => panic!("Expected let statement"),
    }
}

#[test]
fn test_parse_deeply_nested_generics() {
    // Deeply nested generic types
    let program = parse("let complex: Result<HashMap<String, Vec<Option<T>>>, Error>").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => {
            let type_ann = type_ann.as_ref().unwrap();
            match &type_ann.kind {
                TypeKind::Generic { name, args } => {
                    assert_eq!(name, "Result");
                    assert_eq!(args.len(), 2);

                    // Check first arg: HashMap<String, Vec<Option<T>>>
                    match &args[0].kind {
                        TypeKind::Generic { name, args } => {
                            assert_eq!(name, "HashMap");
                            assert_eq!(args.len(), 2);
                            assert!(matches!(&args[0].kind, TypeKind::Named(n) if n == "String"));

                            // Check Vec<Option<T>>
                            match &args[1].kind {
                                TypeKind::Generic { name, args } => {
                                    assert_eq!(name, "Vec");
                                    assert_eq!(args.len(), 1);

                                    // Check Option<T>
                                    match &args[0].kind {
                                        TypeKind::Generic { name, args } => {
                                            assert_eq!(name, "Option");
                                            assert_eq!(args.len(), 1);
                                            assert!(
                                                matches!(&args[0].kind, TypeKind::TypeParam(n) if n == "T")
                                            );
                                        }
                                        _ => panic!("Expected Option<T>"),
                                    }
                                }
                                _ => panic!("Expected Vec type"),
                            }
                        }
                        _ => panic!("Expected HashMap type"),
                    }

                    // Check second arg: Error
                    assert!(matches!(&args[1].kind, TypeKind::Named(n) if n == "Error"));
                }
                _ => panic!("Expected generic type"),
            }
        }
        _ => panic!("Expected let statement"),
    }
}

#[test]
fn test_parse_generic_empty_args() {
    // Empty generic arguments
    let program = parse("let v: Vec<>").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => {
            let type_ann = type_ann.as_ref().unwrap();
            match &type_ann.kind {
                TypeKind::Generic { name, args } => {
                    assert_eq!(name, "Vec");
                    assert_eq!(args.len(), 0);
                }
                _ => panic!("Expected generic type"),
            }
        }
        _ => panic!("Expected let statement"),
    }
}

#[test]
fn test_parse_generic_trailing_comma() {
    // Trailing comma in generic arguments
    let program = parse("let map: HashMap<K, V,>").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => {
            let type_ann = type_ann.as_ref().unwrap();
            match &type_ann.kind {
                TypeKind::Generic { name, args } => {
                    assert_eq!(name, "HashMap");
                    assert_eq!(args.len(), 2);
                    assert!(matches!(&args[0].kind, TypeKind::TypeParam(n) if n == "K"));
                    assert!(matches!(&args[1].kind, TypeKind::TypeParam(n) if n == "V"));
                }
                _ => panic!("Expected generic type"),
            }
        }
        _ => panic!("Expected let statement"),
    }
}

#[test]
fn test_parse_generic_whitespace_handling() {
    // Extra whitespace in generic types
    let program = parse("let v: Vec< i32 >").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => {
            let type_ann = type_ann.as_ref().unwrap();
            match &type_ann.kind {
                TypeKind::Generic { name, args } => {
                    assert_eq!(name, "Vec");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0].kind, TypeKind::Named(n) if n == "i32"));
                }
                _ => panic!("Expected generic type"),
            }
        }
        _ => panic!("Expected let statement"),
    }

    // Whitespace between type arguments
    let program = parse("let map: HashMap<String,  i32>").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => {
            let type_ann = type_ann.as_ref().unwrap();
            match &type_ann.kind {
                TypeKind::Generic { name, args } => {
                    assert_eq!(name, "HashMap");
                    assert_eq!(args.len(), 2);
                }
                _ => panic!("Expected generic type"),
            }
        }
        _ => panic!("Expected let statement"),
    }
}

#[test]
fn test_parse_generic_in_function_params() {
    // Generic types in function parameters
    let program = parse("fn process(data: Vec<T>, map: HashMap<K, V>) {}").unwrap();
    match &program.statements[0].kind {
        StmtKind::Function { params, .. } => {
            assert_eq!(params.len(), 2);

            // Check first param: Vec<T>
            match &params[0].type_ann.kind {
                TypeKind::Generic { name, args } => {
                    assert_eq!(name, "Vec");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0].kind, TypeKind::TypeParam(n) if n == "T"));
                }
                _ => panic!("Expected generic type"),
            }

            // Check second param: HashMap<K, V>
            match &params[1].type_ann.kind {
                TypeKind::Generic { name, args } => {
                    assert_eq!(name, "HashMap");
                    assert_eq!(args.len(), 2);
                    assert!(matches!(&args[0].kind, TypeKind::TypeParam(n) if n == "K"));
                    assert!(matches!(&args[1].kind, TypeKind::TypeParam(n) if n == "V"));
                }
                _ => panic!("Expected generic type"),
            }
        }
        _ => panic!("Expected function statement"),
    }
}

#[test]
fn test_parse_generic_in_function_return() {
    // Generic type in function return
    let program = parse("fn get_items() -> Vec<String> { [] }").unwrap();
    match &program.statements[0].kind {
        StmtKind::Function { ret_type, .. } => {
            let ret_type = ret_type.as_ref().unwrap();
            match &ret_type.kind {
                TypeKind::Generic { name, args } => {
                    assert_eq!(name, "Vec");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0].kind, TypeKind::Named(n) if n == "String"));
                }
                _ => panic!("Expected generic return type"),
            }
        }
        _ => panic!("Expected function statement"),
    }
}

#[test]
fn test_parse_generic_constructors() {
    // Generic constructor calls
    let expr = parse_expr("Vec<i32>()").unwrap();
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            assert!(args.is_empty());
            match &callee.kind {
                ExprKind::GenericConstructor { name, type_args } => {
                    assert_eq!(name, "Vec");
                    assert_eq!(type_args.len(), 1);
                    assert!(matches!(&type_args[0].kind, TypeKind::Named(n) if n == "i32"));
                }
                _ => panic!("Expected generic constructor, got {:?}", callee.kind),
            }
        }
        _ => panic!("Expected call expression"),
    }

    // Constructor with arguments
    let expr = parse_expr("HashMap<String, i32>(capacity)").unwrap();
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            assert_eq!(args.len(), 1);
            match &callee.kind {
                ExprKind::GenericConstructor { name, type_args } => {
                    assert_eq!(name, "HashMap");
                    assert_eq!(type_args.len(), 2);
                }
                _ => panic!("Expected generic constructor"),
            }
        }
        _ => panic!("Expected call expression"),
    }
}

#[test]
fn test_parse_type_parameter_patterns() {
    // Test various type parameter patterns
    let patterns = vec![
        ("T", true),
        ("U", true),
        ("K", true),
        ("V", true),
        ("E", true),
        ("R", true),
        ("TKey", true),
        ("TValue", true),
        ("TItem", true),
        ("String", false),
        ("i32", false),
        ("MyType", false),
    ];

    for (type_name, should_be_param) in patterns {
        let program = parse(&format!("let x: {}", type_name)).unwrap();
        match &program.statements[0].kind {
            StmtKind::Let { type_ann, .. } => {
                let type_ann = type_ann.as_ref().unwrap();
                if should_be_param {
                    assert!(
                        matches!(&type_ann.kind, TypeKind::TypeParam(n) if n == type_name),
                        "{} should be TypeParam",
                        type_name
                    );
                } else {
                    assert!(
                        matches!(&type_ann.kind, TypeKind::Named(n) if n == type_name),
                        "{} should be Named",
                        type_name
                    );
                }
            }
            _ => panic!("Expected let statement"),
        }
    }
}

#[test]
fn test_parse_generic_in_array_type() {
    // Generic types within array types
    let program = parse("let arr: [Option<T>]").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => {
            let type_ann = type_ann.as_ref().unwrap();
            match &type_ann.kind {
                TypeKind::Array(elem_type) => match &elem_type.kind {
                    TypeKind::Generic { name, args } => {
                        assert_eq!(name, "Option");
                        assert_eq!(args.len(), 1);
                        assert!(matches!(&args[0].kind, TypeKind::TypeParam(n) if n == "T"));
                    }
                    _ => panic!("Expected generic element type"),
                },
                _ => panic!("Expected array type"),
            }
        }
        _ => panic!("Expected let statement"),
    }
}

#[test]
fn test_parse_generic_in_function_type() {
    // Generic types in function type annotations
    let program = parse("let callback: (Vec<T>) -> Option<U>").unwrap();
    match &program.statements[0].kind {
        StmtKind::Let { type_ann, .. } => {
            let type_ann = type_ann.as_ref().unwrap();
            match &type_ann.kind {
                TypeKind::Function { params, ret } => {
                    assert_eq!(params.len(), 1);

                    // Check param type
                    match &params[0].kind {
                        TypeKind::Generic { name, args } => {
                            assert_eq!(name, "Vec");
                            assert_eq!(args.len(), 1);
                            assert!(matches!(&args[0].kind, TypeKind::TypeParam(n) if n == "T"));
                        }
                        _ => panic!("Expected generic param type"),
                    }

                    // Check return type
                    match &ret.kind {
                        TypeKind::Generic { name, args } => {
                            assert_eq!(name, "Option");
                            assert_eq!(args.len(), 1);
                            assert!(matches!(&args[0].kind, TypeKind::TypeParam(n) if n == "U"));
                        }
                        _ => panic!("Expected generic return type"),
                    }
                }
                _ => panic!("Expected function type"),
            }
        }
        _ => panic!("Expected let statement"),
    }
}

#[test]
fn test_parse_generic_error_missing_closing_bracket() {
    // Missing closing bracket
    assert!(parse("let v: Vec<i32").is_err());
    assert!(parse("let v: Vec<").is_err());
    assert!(parse("let v: HashMap<String, i32").is_err());
}

#[test]
fn test_parse_generic_error_invalid_syntax() {
    // Invalid generic syntax
    assert!(parse("let v: Vec<,>").is_err());
    assert!(parse("let v: Vec<i32,>").is_ok()); // Trailing comma is allowed
    assert!(parse("let v: Vec i32>").is_err());
    assert!(parse("let v: Vec<<i32>").is_err());
}

#[test]
fn test_parse_generic_error_invalid_type_args() {
    // Invalid type arguments (numbers, special chars)
    assert!(parse("let v: Vec<123>").is_err());
    assert!(parse("let v: Vec<!@#>").is_err());
    assert!(parse("let v: Vec<+>").is_err());
}

#[test]
fn test_parse_generic_mixed_with_regular_types() {
    // Mix of generic and non-generic types
    let program =
        parse("fn transform(input: String, options: Config<T>) -> Result<String, Error> {}")
            .unwrap();
    match &program.statements[0].kind {
        StmtKind::Function {
            params, ret_type, ..
        } => {
            // First param: regular type
            assert!(matches!(&params[0].type_ann.kind, TypeKind::Named(n) if n == "String"));

            // Second param: generic type
            match &params[1].type_ann.kind {
                TypeKind::Generic { name, args } => {
                    assert_eq!(name, "Config");
                    assert_eq!(args.len(), 1);
                }
                _ => panic!("Expected generic type"),
            }

            // Return type: generic with mixed args
            let ret_type = ret_type.as_ref().unwrap();
            match &ret_type.kind {
                TypeKind::Generic { name, args } => {
                    assert_eq!(name, "Result");
                    assert_eq!(args.len(), 2);
                    assert!(matches!(&args[0].kind, TypeKind::Named(n) if n == "String"));
                    assert!(matches!(&args[1].kind, TypeKind::Named(n) if n == "Error"));
                }
                _ => panic!("Expected generic return type"),
            }
        }
        _ => panic!("Expected function statement"),
    }
}

#[test]
fn test_parse_common_generic_patterns() {
    // Test common generic type patterns from real code
    let test_cases = vec![
        ("let opt: Option<String>", "Option", 1),
        ("let res: Result<i32, Error>", "Result", 2),
        ("let vec: Vec<User>", "Vec", 1),
        ("let map: BTreeMap<String, Value>", "BTreeMap", 2),
        ("let set: HashSet<i32>", "HashSet", 1),
        ("let arc: Arc<Mutex<T>>", "Arc", 1),
        ("let boxed: Box<dyn Trait>", "Box", 1),
    ];

    for (input, expected_name, expected_args) in test_cases {
        let program = parse(input).unwrap();
        match &program.statements[0].kind {
            StmtKind::Let { type_ann, .. } => {
                let type_ann = type_ann.as_ref().unwrap();
                match &type_ann.kind {
                    TypeKind::Generic { name, args } => {
                        assert_eq!(name, expected_name, "Failed for: {}", input);
                        assert_eq!(args.len(), expected_args, "Failed for: {}", input);
                    }
                    _ => panic!("Expected generic type for: {}", input),
                }
            }
            _ => panic!("Expected let statement"),
        }
    }
}

#[test]
fn test_parse_generic_display_implementation() {
    // Verify Display implementation for generic types
    let program = parse("let x: Vec<i32>").unwrap();
    let stmt_str = format!("{}", program.statements[0]);
    assert!(stmt_str.contains("Vec<i32>"));

    let program = parse("let map: HashMap<String, Vec<Option<T>>>").unwrap();
    let stmt_str = format!("{}", program.statements[0]);
    assert!(stmt_str.contains("HashMap<String, Vec<Option<T>>>"));
}

// ========== GENERIC FUNCTION PARAMETER TESTS ==========

#[test]
fn test_parse_generic_function_simple() {
    // Simple generic function
    let program = parse("fn identity<T>(x: T) -> T { x }").unwrap();
    assert_eq!(program.statements.len(), 1);

    match &program.statements[0].kind {
        StmtKind::Function {
            name,
            generic_params,
            params,
            ret_type,
            ..
        } => {
            assert_eq!(name, "identity");

            // Check generic parameters
            let generics = generic_params.as_ref().unwrap();
            assert_eq!(generics.params.len(), 1);
            assert_eq!(generics.params[0].name, "T");
            assert!(generics.params[0].bounds.is_empty());

            // Check function parameters
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "x");
            assert!(matches!(&params[0].type_ann.kind, TypeKind::TypeParam(n) if n == "T"));

            // Check return type
            assert!(matches!(&ret_type.as_ref().unwrap().kind, TypeKind::TypeParam(n) if n == "T"));
        }
        _ => panic!("Expected function statement"),
    }
}

#[test]
fn test_parse_generic_function_with_bounds() {
    // Generic function with trait bounds
    let program = parse("fn clone_it<T: Clone>(x: T) -> T { x }").unwrap();

    match &program.statements[0].kind {
        StmtKind::Function { generic_params, .. } => {
            let generics = generic_params.as_ref().unwrap();
            assert_eq!(generics.params.len(), 1);
            assert_eq!(generics.params[0].name, "T");
            assert_eq!(generics.params[0].bounds.len(), 1);
            assert_eq!(generics.params[0].bounds[0].trait_name, "Clone");
        }
        _ => panic!("Expected function statement"),
    }
}

#[test]
fn test_parse_generic_function_multiple_bounds() {
    // Generic function with multiple trait bounds
    let program = parse("fn process<T: Clone + Debug + Send>(x: T) {}").unwrap();

    match &program.statements[0].kind {
        StmtKind::Function { generic_params, .. } => {
            let generics = generic_params.as_ref().unwrap();
            assert_eq!(generics.params.len(), 1);
            assert_eq!(generics.params[0].name, "T");
            assert_eq!(generics.params[0].bounds.len(), 3);
            assert_eq!(generics.params[0].bounds[0].trait_name, "Clone");
            assert_eq!(generics.params[0].bounds[1].trait_name, "Debug");
            assert_eq!(generics.params[0].bounds[2].trait_name, "Send");
        }
        _ => panic!("Expected function statement"),
    }
}

#[test]
fn test_parse_generic_function_multiple_params() {
    // Generic function with multiple type parameters
    let program = parse("fn swap<T, U>(a: T, b: U) -> U { b }").unwrap();

    match &program.statements[0].kind {
        StmtKind::Function {
            name,
            generic_params,
            params,
            ..
        } => {
            assert_eq!(name, "swap");

            let generics = generic_params.as_ref().unwrap();
            assert_eq!(generics.params.len(), 2);
            assert_eq!(generics.params[0].name, "T");
            assert_eq!(generics.params[1].name, "U");

            assert_eq!(params.len(), 2);
            assert!(matches!(&params[0].type_ann.kind, TypeKind::TypeParam(n) if n == "T"));
            assert!(matches!(&params[1].type_ann.kind, TypeKind::TypeParam(n) if n == "U"));
        }
        _ => panic!("Expected function statement"),
    }
}

#[test]
fn test_parse_generic_function_mixed_bounds() {
    // Generic function with mixed bounds
    let program = parse("fn complex<T: Clone, U: Debug + Send, V>(a: T, b: U, c: V) {}").unwrap();

    match &program.statements[0].kind {
        StmtKind::Function { generic_params, .. } => {
            let generics = generic_params.as_ref().unwrap();
            assert_eq!(generics.params.len(), 3);

            // T: Clone
            assert_eq!(generics.params[0].name, "T");
            assert_eq!(generics.params[0].bounds.len(), 1);
            assert_eq!(generics.params[0].bounds[0].trait_name, "Clone");

            // U: Debug + Send
            assert_eq!(generics.params[1].name, "U");
            assert_eq!(generics.params[1].bounds.len(), 2);
            assert_eq!(generics.params[1].bounds[0].trait_name, "Debug");
            assert_eq!(generics.params[1].bounds[1].trait_name, "Send");

            // V (no bounds)
            assert_eq!(generics.params[2].name, "V");
            assert!(generics.params[2].bounds.is_empty());
        }
        _ => panic!("Expected function statement"),
    }
}

#[test]
fn test_parse_generic_function_empty_params() {
    // Edge case: empty generic parameters
    let program = parse("fn weird<>() {}").unwrap();

    match &program.statements[0].kind {
        StmtKind::Function { generic_params, .. } => {
            let generics = generic_params.as_ref().unwrap();
            assert!(generics.params.is_empty());
        }
        _ => panic!("Expected function statement"),
    }
}

#[test]
fn test_parse_generic_function_trailing_comma() {
    // Edge case: trailing comma
    let program = parse("fn test<T,>(x: T) -> T { x }").unwrap();

    match &program.statements[0].kind {
        StmtKind::Function { generic_params, .. } => {
            let generics = generic_params.as_ref().unwrap();
            assert_eq!(generics.params.len(), 1);
            assert_eq!(generics.params[0].name, "T");
        }
        _ => panic!("Expected function statement"),
    }
}

#[test]
fn test_parse_generic_function_complex_usage() {
    // Generic function using generic types in body
    let program = parse("fn map<T, U>(items: Vec<T>) -> Vec<U> { Vec<U>() }").unwrap();

    match &program.statements[0].kind {
        StmtKind::Function {
            name,
            generic_params,
            params,
            ret_type,
            ..
        } => {
            assert_eq!(name, "map");

            // Check generics
            let generics = generic_params.as_ref().unwrap();
            assert_eq!(generics.params.len(), 2);

            // Check parameter types use generics
            assert_eq!(params.len(), 1);
            match &params[0].type_ann.kind {
                TypeKind::Generic { name, args } => {
                    assert_eq!(name, "Vec");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0].kind, TypeKind::TypeParam(n) if n == "T"));
                }
                _ => panic!("Expected generic type"),
            }

            // Check return type
            match &ret_type.as_ref().unwrap().kind {
                TypeKind::Generic { name, args } => {
                    assert_eq!(name, "Vec");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0].kind, TypeKind::TypeParam(n) if n == "U"));
                }
                _ => panic!("Expected generic return type"),
            }
        }
        _ => panic!("Expected function statement"),
    }
}

#[test]
fn test_parse_generic_display() {
    // Test Display implementation
    let program = parse("fn test<T: Clone + Debug>(x: T) -> T { x }").unwrap();
    let stmt_str = format!("{}", program.statements[0]);

    assert!(stmt_str.contains("fn test<T: Clone + Debug>"));
    assert!(stmt_str.contains("(x: T)"));
    assert!(stmt_str.contains("-> T"));
}

#[test]
fn test_parse_generic_error_missing_closing() {
    // Missing closing >
    assert!(parse("fn test<T(x: T) {}").is_err());
    assert!(parse("fn test<T, U(x: T) {}").is_err());
}

#[test]
fn test_parse_generic_function_error_invalid_syntax() {
    // Invalid generic syntax
    assert!(parse("fn test<>(x: T) {}").is_ok()); // Empty is ok
    assert!(parse("fn test<123>(x: T) {}").is_err()); // Numbers not allowed
    assert!(parse("fn test<T U>(x: T) {}").is_err()); // Missing comma
}

#[test]
fn test_parse_generic_async_function() {
    // Async generic function
    let program = parse("async fn fetch<T: Send>(url: string) -> T { await get(url) }").unwrap();

    match &program.statements[0].kind {
        StmtKind::Function {
            is_async,
            generic_params,
            ..
        } => {
            assert!(*is_async);
            let generics = generic_params.as_ref().unwrap();
            assert_eq!(generics.params.len(), 1);
            assert_eq!(generics.params[0].name, "T");
            assert_eq!(generics.params[0].bounds[0].trait_name, "Send");
        }
        _ => panic!("Expected function statement"),
    }
}

#[test]
fn test_parse_function_with_where_clause() {
    // Function with where clause
    let program =
        parse("fn test<T, U>(x: T, y: U) -> T where T: Clone + Debug, U: Debug { x }").unwrap();

    match &program.statements[0].kind {
        StmtKind::Function {
            name,
            generic_params,
            where_clause,
            ..
        } => {
            assert_eq!(name, "test");

            // Check generic parameters
            let generics = generic_params.as_ref().unwrap();
            assert_eq!(generics.params.len(), 2);
            assert_eq!(generics.params[0].name, "T");
            assert_eq!(generics.params[1].name, "U");

            // Check where clause
            let where_clause = where_clause.as_ref().unwrap();
            assert_eq!(where_clause.predicates.len(), 2);

            // First predicate: T: Clone + Debug
            assert_eq!(where_clause.predicates[0].bounds.len(), 2);
            assert_eq!(where_clause.predicates[0].bounds[0].trait_name, "Clone");
            assert_eq!(where_clause.predicates[0].bounds[1].trait_name, "Debug");

            // Second predicate: U: Debug
            assert_eq!(where_clause.predicates[1].bounds.len(), 1);
            assert_eq!(where_clause.predicates[1].bounds[0].trait_name, "Debug");
        }
        _ => panic!("Expected function statement"),
    }
}

#[test]
fn test_parse_struct_with_where_clause() {
    // Struct with where clause
    let program =
        parse("struct Container<T, U> where T: Clone, U: Debug { data: T, meta: U }").unwrap();

    match &program.statements[0].kind {
        StmtKind::Struct {
            name,
            generic_params,
            where_clause,
            fields,
            ..
        } => {
            assert_eq!(name, "Container");

            // Check generic parameters
            let generics = generic_params.as_ref().unwrap();
            assert_eq!(generics.params.len(), 2);
            assert_eq!(generics.params[0].name, "T");
            assert_eq!(generics.params[1].name, "U");

            // Check where clause
            let where_clause = where_clause.as_ref().unwrap();
            assert_eq!(where_clause.predicates.len(), 2);

            // Check fields
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "data");
            assert_eq!(fields[1].name, "meta");
        }
        _ => panic!("Expected struct statement"),
    }
}

#[test]
fn test_parse_enum_with_where_clause() {
    // Enum with where clause
    let program = parse("enum Result<T, E> where T: Clone, E: Debug { Ok(T), Err(E) }").unwrap();

    match &program.statements[0].kind {
        StmtKind::Enum {
            name,
            generic_params,
            where_clause,
            variants,
            ..
        } => {
            assert_eq!(name, "Result");

            // Check generic parameters
            let generics = generic_params.as_ref().unwrap();
            assert_eq!(generics.params.len(), 2);
            assert_eq!(generics.params[0].name, "T");
            assert_eq!(generics.params[1].name, "E");

            // Check where clause
            let where_clause = where_clause.as_ref().unwrap();
            assert_eq!(where_clause.predicates.len(), 2);

            // Check variants
            assert_eq!(variants.len(), 2);
            assert_eq!(variants[0].name, "Ok");
            assert_eq!(variants[1].name, "Err");
        }
        _ => panic!("Expected enum statement"),
    }
}
