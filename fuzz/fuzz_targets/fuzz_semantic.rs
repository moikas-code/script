#![no_main]

use libfuzzer_sys::fuzz_target;
use script::lexer::Lexer;
use script::parser::Parser;
use script::semantic::SemanticAnalyzer;

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string, skip invalid UTF-8
    if let Ok(source) = std::str::from_utf8(data) {
        // Skip extremely long inputs to prevent timeout
        if source.len() > 25_000 {
            return;
        }
        
        // Test full lexer -> parser -> semantic analysis pipeline
        if let Ok(mut lexer) = Lexer::new(source) {
            if let Ok((tokens, _errors)) = lexer.scan_tokens() {
                // Skip if too many tokens (DoS prevention)
                if tokens.len() > 5_000 {
                    return;
                }
                
                let mut parser = Parser::new(tokens);
                if let Ok(ast) = parser.parse_program() {
                    // Skip extremely complex ASTs
                    if ast.statements.len() > 1_000 {
                        return;
                    }
                    
                    let mut analyzer = SemanticAnalyzer::new();
                    // Semantic analyzer should never crash, even on invalid ASTs
                    let _ = analyzer.analyze_program(&ast);
                }
            }
        }
    }
});