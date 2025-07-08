//! Fuzzing support for the Script lexer
//! 
//! This module provides fuzzing targets for security testing.
//! Use with cargo-fuzz or similar fuzzing frameworks.

use super::{Lexer, UnicodeSecurityConfig, UnicodeSecurityLevel};

/// Main fuzzing entry point for the lexer
#[cfg(feature = "fuzzing")]
pub fn fuzz_lexer(data: &[u8]) {
    // Try to interpret data as UTF-8
    if let Ok(input) = std::str::from_utf8(data) {
        // Test with default configuration
        fuzz_lexer_with_config(input, UnicodeSecurityConfig::default());
        
        // Test with strict Unicode security
        let strict_config = UnicodeSecurityConfig {
            level: UnicodeSecurityLevel::Strict,
            normalize_identifiers: true,
            detect_confusables: true,
        };
        fuzz_lexer_with_config(input, strict_config);
        
        // Test with permissive configuration
        let permissive_config = UnicodeSecurityConfig {
            level: UnicodeSecurityLevel::Permissive,
            normalize_identifiers: false,
            detect_confusables: false,
        };
        fuzz_lexer_with_config(input, permissive_config);
    }
}

/// Fuzz the lexer with a specific configuration
fn fuzz_lexer_with_config(input: &str, config: UnicodeSecurityConfig) {
    // Try to create lexer - this tests input validation
    let lexer = match Lexer::with_unicode_config(input, config) {
        Ok(lexer) => lexer,
        Err(_) => return, // Expected for invalid inputs
    };
    
    // Scan all tokens - this tests the scanning logic
    let (tokens, errors) = lexer.scan_tokens();
    
    // Basic validation that we didn't panic
    assert!(tokens.len() + errors.len() > 0 || input.is_empty());
}

/// Fuzzing target for string literal parsing
#[cfg(feature = "fuzzing")]
pub fn fuzz_string_literals(data: &[u8]) {
    if let Ok(input) = std::str::from_utf8(data) {
        // Wrap input in quotes to make it a string literal
        let quoted = format!("\"{}\"", input);
        
        if let Ok(lexer) = Lexer::new(&quoted) {
            let _ = lexer.scan_tokens();
        }
    }
}

/// Fuzzing target for numeric literal parsing
#[cfg(feature = "fuzzing")]
pub fn fuzz_numeric_literals(data: &[u8]) {
    if let Ok(input) = std::str::from_utf8(data) {
        if let Ok(lexer) = Lexer::new(input) {
            let _ = lexer.scan_tokens();
        }
    }
}

/// Fuzzing target for comment parsing
#[cfg(feature = "fuzzing")]
pub fn fuzz_comments(data: &[u8]) {
    if let Ok(input) = std::str::from_utf8(data) {
        // Test single-line comments
        let single_line = format!("// {}", input);
        if let Ok(lexer) = Lexer::new(&single_line) {
            let _ = lexer.scan_tokens();
        }
        
        // Test multi-line comments
        let multi_line = format!("/* {} */", input);
        if let Ok(lexer) = Lexer::new(&multi_line) {
            let _ = lexer.scan_tokens();
        }
        
        // Test doc comments
        let doc_comment = format!("/// {}", input);
        if let Ok(lexer) = Lexer::new(&doc_comment) {
            let _ = lexer.scan_tokens();
        }
    }
}

/// Fuzzing target for Unicode edge cases
#[cfg(feature = "fuzzing")]
pub fn fuzz_unicode_edge_cases(data: &[u8]) {
    if let Ok(input) = std::str::from_utf8(data) {
        // Test with various Unicode normalization forms
        let configs = vec![
            UnicodeSecurityConfig {
                level: UnicodeSecurityLevel::Strict,
                normalize_identifiers: true,
                detect_confusables: true,
            },
            UnicodeSecurityConfig {
                level: UnicodeSecurityLevel::Warning,
                normalize_identifiers: true,
                detect_confusables: false,
            },
            UnicodeSecurityConfig {
                level: UnicodeSecurityLevel::Permissive,
                normalize_identifiers: false,
                detect_confusables: false,
            },
        ];
        
        for config in configs {
            if let Ok(lexer) = Lexer::with_unicode_config(input, config) {
                let _ = lexer.scan_tokens();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fuzz_empty_input() {
        fuzz_lexer(b"");
    }
    
    #[test]
    fn test_fuzz_ascii_input() {
        fuzz_lexer(b"let x = 42;");
    }
    
    #[test]
    fn test_fuzz_unicode_input() {
        fuzz_lexer("let 世界 = 42;".as_bytes());
    }
    
    #[test]
    fn test_fuzz_malformed_utf8() {
        // Invalid UTF-8 sequence
        fuzz_lexer(&[0xFF, 0xFE, 0xFD]);
    }
    
    #[test]
    fn test_fuzz_nested_comments() {
        fuzz_comments(b"/* /* /* nested */ */ */");
    }
    
    #[test]
    fn test_fuzz_large_string() {
        let large = "a".repeat(10000);
        fuzz_string_literals(large.as_bytes());
    }
}