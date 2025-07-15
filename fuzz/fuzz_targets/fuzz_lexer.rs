#![no_main]

use libfuzzer_sys::fuzz_target;
use script::lexer::Lexer;

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string, skip invalid UTF-8
    if let Ok(source) = std::str::from_utf8(data) {
        // Skip extremely long inputs to prevent timeout
        if source.len() > 100_000 {
            return;
        }
        
        // Test lexer with fuzzing input
        if let Ok(mut lexer) = Lexer::new(source) {
            // Lexer should never crash, even on malformed input
            let _ = lexer.scan_tokens();
        }
    }
});