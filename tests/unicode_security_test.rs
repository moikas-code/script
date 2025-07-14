use script::lexer::{Lexer, UnicodeSecurityConfig, UnicodeSecurityLevel};

#[test]
fn test_ascii_identifiers_pass_through() {
    let input = "let ascii_identifier = 42";
    let lexer = Lexer::new(input).unwrap();
    let (tokens, errors) = lexer.scan_tokens();

    assert_eq!(errors.len(), 0);
    assert!(tokens.len() > 0);
}

#[test]
fn test_unicode_normalization_enabled() {
    let config = UnicodeSecurityConfig {
        level: UnicodeSecurityLevel::Permissive,
        normalize_identifiers: true,
        detect_confusables: false,
    };

    // Test with Unicode identifier that should be normalized
    let input = "let café = 42"; // Contains Unicode 'é'
    let lexer = Lexer::with_unicode_config(input, config).unwrap();
    let (tokens, errors) = lexer.scan_tokens();

    assert_eq!(errors.len(), 0);
    assert!(tokens.len() > 0);
}

#[test]
fn test_confusable_detection_warning_level() {
    let config = UnicodeSecurityConfig {
        level: UnicodeSecurityLevel::Warning,
        normalize_identifiers: true,
        detect_confusables: true,
    };

    // Use Cyrillic 'а' which looks like Latin 'a'
    let input = "let а = 42; let a = 43"; // First 'a' is Cyrillic, second is Latin
    let lexer = Lexer::with_unicode_config(input, config).unwrap();
    let (tokens, errors) = lexer.scan_tokens();

    // Should have warning about confusable identifiers
    // For now we expect an error since we use error() instead of warn()
    assert!(errors.len() > 0);
    assert!(errors[0].to_string().contains("confusable"));
}

#[test]
fn test_confusable_detection_strict_level() {
    let config = UnicodeSecurityConfig {
        level: UnicodeSecurityLevel::Strict,
        normalize_identifiers: true,
        detect_confusables: true,
    };

    // Use Cyrillic 'а' which looks like Latin 'a'
    let input = "let а = 42; let a = 43"; // First 'a' is Cyrillic, second is Latin
    let lexer = Lexer::with_unicode_config(input, config).unwrap();
    let (tokens, errors) = lexer.scan_tokens();

    // Should reject confusable identifiers in strict mode
    assert!(errors.len() > 0);
    assert!(errors[0].to_string().contains("Confusable identifier"));
}

#[test]
fn test_confusable_detection_permissive_level() {
    let config = UnicodeSecurityConfig {
        level: UnicodeSecurityLevel::Permissive,
        normalize_identifiers: true,
        detect_confusables: true,
    };

    // Use Cyrillic 'а' which looks like Latin 'a'
    let input = "let а = 42; let a = 43"; // First 'a' is Cyrillic, second is Latin
    let lexer = Lexer::with_unicode_config(input, config).unwrap();
    let (tokens, errors) = lexer.scan_tokens();

    // Should allow confusable identifiers in permissive mode
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_confusable_detection_disabled() {
    let config = UnicodeSecurityConfig {
        level: UnicodeSecurityLevel::Warning,
        normalize_identifiers: true,
        detect_confusables: false, // Disabled
    };

    // Use Cyrillic 'а' which looks like Latin 'a'
    let input = "let а = 42; let a = 43"; // First 'a' is Cyrillic, second is Latin
    let lexer = Lexer::with_unicode_config(input, config).unwrap();
    let (tokens, errors) = lexer.scan_tokens();

    // Should not detect confusables when disabled
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_normalization_disabled() {
    let config = UnicodeSecurityConfig {
        level: UnicodeSecurityLevel::Warning,
        normalize_identifiers: false, // Disabled
        detect_confusables: false,
    };

    // Test with Unicode identifier
    let input = "let café = 42"; // Contains Unicode 'é'
    let lexer = Lexer::with_unicode_config(input, config).unwrap();
    let (tokens, errors) = lexer.scan_tokens();

    // Should still work even without normalization
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_greek_confusables() {
    let config = UnicodeSecurityConfig {
        level: UnicodeSecurityLevel::Warning,
        normalize_identifiers: true,
        detect_confusables: true,
    };

    // Use Greek 'α' which looks like Latin 'a'
    let input = "let α = 42; let a = 43"; // First is Greek alpha, second is Latin a
    let lexer = Lexer::with_unicode_config(input, config).unwrap();
    let (tokens, errors) = lexer.scan_tokens();

    // Should detect Greek confusables
    assert!(errors.len() > 0);
    assert!(errors[0].to_string().contains("confusable"));
}
