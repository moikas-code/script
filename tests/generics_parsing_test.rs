//! Simple parsing-only test for generics
//! This test only uses lexer and parser, avoiding the broken parts

#[cfg(test)]
mod tests {
    use script::lexer::{Lexer, TokenKind};
    use script::parser::{Expr, Parser, StmtKind};

    #[test]
    fn test_basic_generic_function_tokenization() {
        let source = "fn identity<T>(x: T) -> T { x }";

        let lexer = Lexer::new(source).expect("Lexer creation should succeed");
        let (tokens, _errors) = lexer.scan_tokens();

        // Check we have the expected tokens including angle brackets
        let token_kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();

        assert!(token_kinds.contains(&&TokenKind::Fn));
        assert!(token_kinds.contains(&&TokenKind::Identifier("identity".to_string())));
        assert!(token_kinds.contains(&&TokenKind::Less)); // <
        assert!(token_kinds.contains(&&TokenKind::Identifier("T".to_string())));
        assert!(token_kinds.contains(&&TokenKind::Greater)); // >
    }

    #[test]
    fn test_generic_function_parsing() {
        let source = r#"
            fn identity<T>(x: T) -> T {
                return x;
            }
        "#;

        let lexer = Lexer::new(source).expect("Lexer creation should succeed");
        let (tokens, _errors) = lexer.scan_tokens();

        let mut parser = Parser::new(tokens);
        let ast = parser.parse().expect("Parsing should succeed");

        // Check we parsed one function
        assert_eq!(ast.statements.len(), 1);

        // Check it's a generic function
        match &ast.statements[0].kind {
            StmtKind::Function {
                name,
                generic_params,
                params,
                ret_type,
                ..
            } => {
                assert_eq!(name, "identity");

                // Check generic params exist
                assert!(generic_params.is_some());
                let gen_params = generic_params.as_ref().unwrap();
                assert_eq!(gen_params.params.len(), 1);
                assert_eq!(gen_params.params[0].name, "T");

                // Check function params
                assert_eq!(params.len(), 1);
                assert_eq!(params[0].name, "x");

                // Check return type
                assert!(ret_type.is_some());
            }
            _ => panic!("Expected function statement"),
        }
    }

    #[test]
    fn test_generic_struct_parsing() {
        let source = r#"
            struct Pair<T, U> {
                first: T,
                second: U,
            }
        "#;

        let lexer = Lexer::new(source).expect("Lexer creation should succeed");
        let (tokens, _errors) = lexer.scan_tokens();

        let mut parser = Parser::new(tokens);
        let ast = parser.parse().expect("Parsing should succeed");

        assert_eq!(ast.statements.len(), 1);

        match &ast.statements[0].kind {
            StmtKind::Struct {
                name,
                generic_params,
                fields,
                ..
            } => {
                assert_eq!(name, "Pair");

                // Check generic params
                assert!(generic_params.is_some());
                let gen_params = generic_params.as_ref().unwrap();
                assert_eq!(gen_params.params.len(), 2);
                assert_eq!(gen_params.params[0].name, "T");
                assert_eq!(gen_params.params[1].name, "U");

                // Check fields
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].name, "first");
                assert_eq!(fields[1].name, "second");
            }
            _ => panic!("Expected struct statement"),
        }
    }

    #[test]
    fn test_trait_with_bounds_parsing() {
        let source = r#"
            fn compare<T: Eq + Clone>(a: T, b: T) -> T {
                if a == b { a } else { b }
            }
        "#;

        let lexer = Lexer::new(source).expect("Lexer creation should succeed");
        let (tokens, _errors) = lexer.scan_tokens();

        let mut parser = Parser::new(tokens);
        let ast = parser.parse().expect("Parsing should succeed");

        assert_eq!(ast.statements.len(), 1);

        match &ast.statements[0].kind {
            StmtKind::Function {
                name,
                generic_params,
                ..
            } => {
                assert_eq!(name, "compare");

                // Check generic params with bounds
                assert!(generic_params.is_some());
                let gen_params = generic_params.as_ref().unwrap();
                assert_eq!(gen_params.params.len(), 1);

                let param = &gen_params.params[0];
                assert_eq!(param.name, "T");
                assert_eq!(param.bounds.len(), 2);
                assert_eq!(param.bounds[0].trait_name, "Eq");
                assert_eq!(param.bounds[1].trait_name, "Clone");
            }
            _ => panic!("Expected function statement"),
        }
    }

    #[test]
    fn test_impl_block_with_generics() {
        let source = r#"
            impl<T> Container<T> {
                fn new(value: T) -> Container<T> {
                    Container { value: value }
                }
            }
        "#;

        let lexer = Lexer::new(source).expect("Lexer creation should succeed");
        let (tokens, _errors) = lexer.scan_tokens();

        let mut parser = Parser::new(tokens);
        let ast = parser.parse().expect("Parsing should succeed");

        assert_eq!(ast.statements.len(), 1);

        match &ast.statements[0].kind {
            StmtKind::Impl(impl_block) => {
                // Check generic params on impl block
                assert!(impl_block.generic_params.is_some());
                let gen_params = impl_block.generic_params.as_ref().unwrap();
                assert_eq!(gen_params.params.len(), 1);
                assert_eq!(gen_params.params[0].name, "T");

                // Check type name includes generics
                assert_eq!(impl_block.type_name, "Container");

                // Check method
                assert_eq!(impl_block.methods.len(), 1);
                assert_eq!(impl_block.methods[0].name, "new");
            }
            _ => panic!("Expected impl block"),
        }
    }

    #[test]
    fn test_multiple_bounds_parsing() {
        let source = "fn process<T: Debug + Clone + Send, U: Display>(x: T, y: U) {}";

        let lexer = Lexer::new(source).expect("Lexer creation should succeed");
        let (tokens, _errors) = lexer.scan_tokens();

        let mut parser = Parser::new(tokens);
        let ast = parser.parse().expect("Should parse");

        match &ast.statements[0].kind {
            StmtKind::Function { generic_params, .. } => {
                let gen_params = generic_params.as_ref().unwrap();
                assert_eq!(gen_params.params.len(), 2);

                // Check T bounds
                assert_eq!(gen_params.params[0].name, "T");
                assert_eq!(gen_params.params[0].bounds.len(), 3);
                assert_eq!(gen_params.params[0].bounds[0].trait_name, "Debug");
                assert_eq!(gen_params.params[0].bounds[1].trait_name, "Clone");
                assert_eq!(gen_params.params[0].bounds[2].trait_name, "Send");

                // Check U bounds
                assert_eq!(gen_params.params[1].name, "U");
                assert_eq!(gen_params.params[1].bounds.len(), 1);
                assert_eq!(gen_params.params[1].bounds[0].trait_name, "Display");
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_generic_type_in_expressions() {
        let source = r#"
            fn main() {
                let x = Container::<i32>::new(42);
                let y = identity::<String>("hello");
            }
        "#;

        let lexer = Lexer::new(source).expect("Lexer creation should succeed");
        let (tokens, _errors) = lexer.scan_tokens();

        let mut parser = Parser::new(tokens);
        let ast = parser.parse().expect("Should parse");

        // Just verify it parses without error
        assert_eq!(ast.statements.len(), 1);
    }
}
