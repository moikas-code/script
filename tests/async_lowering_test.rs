#[cfg(test)]
mod async_lowering_tests {
    use script::lexer::Lexer;
    use script::lowering::AstLowerer;
    use script::parser::Parser;
    use script::semantic::analyzer::SemanticAnalyzer;
    use script::types::Type;

    #[test]
    fn test_async_function_lowering() {
        let input = r#"
            async fn simple_async() -> i32 {
                42
            }
        "#;

        // Lex
        let mut scanner = Lexer::new(input).unwrap();
        let tokens = scanner.scan_tokens().0;

        // Parse
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        // Semantic analysis
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze_program(&ast).unwrap();
        let symbol_table = analyzer.symbol_table().clone();
        let type_info = std::collections::HashMap::new(); // For test purposes
        let generic_instantiations = Vec::new(); // For test purposes

        // Lower to IR
        let closure_captures = std::collections::HashMap::new(); // For test purposes

        let mut lowerer = AstLowerer::new(symbol_table, type_info, generic_instantiations, closure_captures);
        let ir_module = lowerer.lower_program(&ast).unwrap();

        // Check that async function was created
        let func = ir_module.get_function_by_name("simple_async").unwrap();
        assert!(func.is_async);

        // Check that return type is wrapped in Future
        match &func.return_type {
            Type::Future(inner) => {
                assert_eq!(**inner, Type::I32);
            }
            _ => panic!("Expected Future return type for async function"),
        }
    }

    #[test]
    fn test_await_expression_lowering() {
        let input = r#"
            async fn delay(ms: i32) -> i32 {
                ms
            }

            async fn use_await() -> i32 {
                let result = await delay(100)
                result + 1
            }
        "#;

        // Lex
        let mut scanner = Lexer::new(input).unwrap();
        let tokens = scanner.scan_tokens().0;

        // Parse
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        // Semantic analysis
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze_program(&ast).unwrap();
        let symbol_table = analyzer.symbol_table().clone();
        let type_info = std::collections::HashMap::new(); // For test purposes
        let generic_instantiations = Vec::new(); // For test purposes

        // Lower to IR
        let closure_captures = std::collections::HashMap::new(); // For test purposes

        let mut lowerer = AstLowerer::new(symbol_table, type_info, generic_instantiations, closure_captures);
        let ir_module = lowerer.lower_program(&ast).unwrap();

        // Check that both async functions were created
        assert!(ir_module.get_function_by_name("delay").is_some());
        assert!(ir_module.get_function_by_name("use_await").is_some());

        // TODO: Once async transformation is complete, check for:
        // - PollFuture instructions
        // - Suspend instructions
        // - State machine structure
    }
}
