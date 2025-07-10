#[cfg(test)]
mod impl_block_tests {
    use crate::lexer::Lexer;
    use crate::parser::{Parser, StmtKind};

    #[test]
    fn test_simple_impl_block() {
        let code = r#"
            impl Point {
                fn new() -> Point {
                    Point { x: 0, y: 0 }
                }
            }
        "#;

        let mut lexer = Lexer::new(code).unwrap();
        let (tokens, errors) = lexer.scan_tokens();
        assert!(errors.is_empty(), "Lexer errors: {:?}", errors);
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().expect("Parse should succeed");

        assert_eq!(ast.statements.len(), 1);
        match &ast.statements[0].kind {
            StmtKind::Impl(impl_block) => {
                assert_eq!(impl_block.type_name, "Point");
                assert_eq!(impl_block.methods.len(), 1);
                assert_eq!(impl_block.methods[0].name, "new");
            }
            _ => panic!("Expected impl block"),
        }
    }

    #[test]
    fn test_generic_impl_block() {
        let code = r#"
            impl<T> Vec<T> {
                fn new() -> Vec<T> {
                    Vec { data: [] }
                }
            }
        "#;

        let mut lexer = Lexer::new(code).unwrap();
        let (tokens, errors) = lexer.scan_tokens();
        assert!(errors.is_empty(), "Lexer errors: {:?}", errors);
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().expect("Parse should succeed");

        assert_eq!(ast.statements.len(), 1);
        match &ast.statements[0].kind {
            StmtKind::Impl(impl_block) => {
                assert_eq!(impl_block.type_name, "Vec");
                assert!(impl_block.generic_params.is_some());
                let generics = impl_block.generic_params.as_ref().unwrap();
                assert_eq!(generics.params.len(), 1);
                assert_eq!(generics.params[0].name, "T");
            }
            _ => panic!("Expected impl block"),
        }
    }

    #[test]
    fn test_impl_with_where_clause() {
        let code = r#"
            impl<T> Vec<T> 
            where T: Clone {
                fn clone_all(self) -> Vec<T> {
                    self
                }
            }
        "#;

        let mut lexer = Lexer::new(code).unwrap();
        let (tokens, errors) = lexer.scan_tokens();
        assert!(errors.is_empty(), "Lexer errors: {:?}", errors);
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().expect("Parse should succeed");

        assert_eq!(ast.statements.len(), 1);
        match &ast.statements[0].kind {
            StmtKind::Impl(impl_block) => {
                assert_eq!(impl_block.type_name, "Vec");
                assert!(impl_block.where_clause.is_some());
                let where_clause = impl_block.where_clause.as_ref().unwrap();
                assert_eq!(where_clause.predicates.len(), 1);
            }
            _ => panic!("Expected impl block"),
        }
    }
}
