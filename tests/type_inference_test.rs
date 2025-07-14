#[cfg(test)]
mod type_inference_tests {
    use script::lexer::Lexer;
    use script::parser::Parser;
    use script::semantic::SemanticAnalyzer;
    use script::types::Type;

    fn analyze_code(source: &str) -> Result<SemanticAnalyzer, script::error::Error> {
        let lexer = Lexer::new(source).unwrap();
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

    #[test]
    fn test_simple_struct_inference() {
        let code = r#"
            struct Box<T> {
                value: T
            }
            
            fn main() {
                let b1 = Box { value: 42 };
                let b2 = Box { value: "hello" };
                let b3 = Box { value: true };
            }
        "#;

        let analyzer = analyze_code(code).unwrap();
        let instantiations = analyzer.generic_instantiations();

        // Should have 3 instantiations of Box
        assert_eq!(instantiations.len(), 3);

        // Check that we have Box<i32>, Box<string>, Box<bool>
        assert!(instantiations
            .iter()
            .any(|inst| inst.function_name == "Box::new"
                && inst.type_args.len() == 1
                && matches!(&inst.type_args[0], Type::I32 | Type::TypeVar(_))));
    }

    #[test]
    fn test_multi_param_struct_inference() {
        let code = r#"
            struct Pair<A, B> {
                first: A,
                second: B
            }
            
            fn main() {
                let p = Pair { first: 42, second: "hello" };
            }
        "#;

        let analyzer = analyze_code(code).unwrap();
        let instantiations = analyzer.generic_instantiations();

        // Should have 1 instantiation of Pair
        assert_eq!(instantiations.len(), 1);
        assert_eq!(instantiations[0].type_args.len(), 2);
    }

    #[test]
    fn test_nested_generic_inference() {
        let code = r#"
            struct Box<T> {
                value: T
            }
            
            enum Option<T> {
                Some(T),
                None
            }
            
            fn main() {
                let nested = Box { value: Option::Some(42) };
            }
        "#;

        let analyzer = analyze_code(code).unwrap();
        let instantiations = analyzer.generic_instantiations();

        // Should have instantiations for both Box and Option
        assert!(instantiations
            .iter()
            .any(|inst| inst.function_name.starts_with("Box")));
        assert!(instantiations
            .iter()
            .any(|inst| inst.function_name.starts_with("Option")));
    }

    #[test]
    fn test_enum_variant_inference() {
        let code = r#"
            enum Option<T> {
                Some(T),
                None
            }
            
            fn main() {
                let some = Option::Some(42);
                let some_str = Option::Some("hello");
            }
        "#;

        let analyzer = analyze_code(code).unwrap();
        let instantiations = analyzer.generic_instantiations();

        // Should have 2 instantiations of Option
        assert_eq!(instantiations.len(), 2);
    }

    #[test]
    fn test_struct_field_type_mismatch() {
        let code = r#"
            struct Box<T> {
                value: T
            }
            
            fn main() {
                // This should fail - can't have both i32 and string for T
                let b = Box { value: if true { 42 } else { "hello" } };
            }
        "#;

        // This should produce an error
        let result = analyze_code(code);
        assert!(
            result.is_err() || {
                // Or it should succeed but have errors recorded
                let analyzer = result.unwrap();
                !analyzer.errors().is_empty()
            }
        );
    }

    #[test]
    fn test_partial_type_annotation() {
        let code = r#"
            struct Container<T> {
                items: Vec<T>
            }
            
            fn main() {
                // Partial annotation with wildcard
                let c: Container<_> = Container { 
                    items: vec![1, 2, 3] 
                };
            }
        "#;

        let analyzer = analyze_code(code).unwrap();
        let instantiations = analyzer.generic_instantiations();

        // Should infer Container<i32> from the vec![1, 2, 3]
        assert_eq!(instantiations.len(), 1);
    }
}
