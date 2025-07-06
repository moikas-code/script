#[cfg(test)]
mod memory_safety_params_tests {
    use script::lexer::Lexer;
    use script::parser::Parser;
    use script::semantic::SemanticAnalyzer;
    use script::error::ErrorReporter;
    
    #[test]
    fn test_function_parameters_are_initialized() {
        let source = r#"
            fn identity<T>(x: T) -> T {
                return x
            }
            
            fn add(a: I32, b: I32) -> I32 {
                return a + b
            }
            
            fn main() {
                let result1 = identity(42)
                let result2 = add(10, 20)
            }
        "#;
        
        // Lex the source
        let mut lexer = Lexer::new(source);
        let result = lexer.scan_all();
        assert!(result.errors.is_empty(), "Lexer should not produce errors");
        
        // Parse the tokens
        let mut parser = Parser::new(result.tokens);
        let ast = parser.parse().expect("Parsing should succeed");
        
        // Run semantic analysis with memory safety enabled
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.set_memory_safety_enabled(true);
        let analysis_result = analyzer.analyze(&ast);
        
        // Check that there are no memory safety errors
        let memory_safety_errors: Vec<_> = analysis_result.errors.iter()
            .filter(|e| e.message().contains("uninitialized variable"))
            .collect();
            
        if !memory_safety_errors.is_empty() {
            let mut reporter = ErrorReporter::new();
            for error in &memory_safety_errors {
                reporter.report(error.clone());
            }
            reporter.print_all();
            panic!("Function parameters should not be flagged as uninitialized");
        }
        
        assert!(memory_safety_errors.is_empty(), 
                "Function parameters should be considered initialized");
    }
    
    #[test]
    fn test_method_self_parameter_is_initialized() {
        let source = r#"
            impl String {
                fn length(self) -> I32 {
                    return 0  // Using self should not error
                }
            }
            
            fn main() {
                let s = "test"
                let len = s.length()
            }
        "#;
        
        // Lex the source
        let mut lexer = Lexer::new(source);
        let result = lexer.scan_all();
        assert!(result.errors.is_empty(), "Lexer should not produce errors");
        
        // Parse the tokens
        let mut parser = Parser::new(result.tokens);
        let ast = parser.parse().expect("Parsing should succeed");
        
        // Run semantic analysis with memory safety enabled
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.set_memory_safety_enabled(true);
        let analysis_result = analyzer.analyze(&ast);
        
        // Check that there are no memory safety errors about self
        let self_errors: Vec<_> = analysis_result.errors.iter()
            .filter(|e| e.message().contains("uninitialized variable 'self'"))
            .collect();
            
        assert!(self_errors.is_empty(), 
                "Self parameter should be considered initialized");
    }
    
    #[test]
    fn test_uninitialized_local_variable_still_caught() {
        let source = r#"
            fn main() {
                let x: I32
                let y = x  // This should still error
            }
        "#;
        
        // Lex the source
        let mut lexer = Lexer::new(source);
        let result = lexer.scan_all();
        assert!(result.errors.is_empty(), "Lexer should not produce errors");
        
        // Parse the tokens
        let mut parser = Parser::new(result.tokens);
        let ast = parser.parse().expect("Parsing should succeed");
        
        // Run semantic analysis with memory safety enabled
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.set_memory_safety_enabled(true);
        let analysis_result = analyzer.analyze(&ast);
        
        // Check that uninitialized local variables are still caught
        let uninit_errors: Vec<_> = analysis_result.errors.iter()
            .filter(|e| e.message().contains("uninitialized variable 'x'"))
            .collect();
            
        assert!(!uninit_errors.is_empty(), 
                "Uninitialized local variables should still be caught");
    }
}