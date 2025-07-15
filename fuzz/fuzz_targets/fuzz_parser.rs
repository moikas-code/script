#![no_main]

use libfuzzer_sys::fuzz_target;
use script::lexer::Lexer;
use script::parser::Parser;

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string, skip invalid UTF-8
    if let Ok(source) = std::str::from_utf8(data) {
        // Skip extremely long inputs to prevent timeout
        if source.len() > 50_000 {
            return;
        }
        
        // Test full lexer -> parser pipeline
        if let Ok(mut lexer) = Lexer::new(source) {
            if let Ok((tokens, _errors)) = lexer.scan_tokens() {
                // Skip if too many tokens (DoS prevention)
                if tokens.len() > 10_000 {
                    return;
                }
                
                let mut parser = Parser::new(tokens);
                // Parser should never crash, even on malformed token streams
                let _ = parser.parse_program();
            }
        }
    }
});