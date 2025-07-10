# Unicode Security Guide for Script Language

## Overview

The Script programming language implements comprehensive Unicode security features to prevent identifier spoofing attacks while providing excellent performance for international character support. This guide covers the security features, configuration options, and best practices for Unicode handling in Script.

## Security Features

### 1. Unicode Normalization (NFKC)

Script automatically normalizes all Unicode identifiers using NFKC (Normalization Form Compatibility Composition) to eliminate visual ambiguity and prevent homograph attacks.

#### Why NFKC?

- **Compatibility**: Converts compatibility characters to their canonical equivalents
- **Composition**: Combines base characters with combining marks
- **Security**: Prevents different Unicode sequences from appearing identical
- **Standards Compliance**: Follows Unicode TR31 and TR36 recommendations

#### Example

```script
// These are normalized to the same identifier
let café = 42;        // é as single character (U+00E9)
let café = 43;        // e + combining acute (U+0065 + U+0301)
// Both become the same normalized form
```

### 2. Confusable Character Detection

Script detects visually similar characters from different Unicode blocks that could be used for spoofing attacks.

#### Supported Confusable Sets

**Latin/Cyrillic Confusables:**
- `а` (U+0430) vs `a` (U+0061) - Cyrillic vs Latin 'a'
- `е` (U+0435) vs `e` (U+0065) - Cyrillic vs Latin 'e'  
- `о` (U+043E) vs `o` (U+006F) - Cyrillic vs Latin 'o'
- `р` (U+0440) vs `p` (U+0070) - Cyrillic vs Latin 'p'
- `с` (U+0441) vs `c` (U+0063) - Cyrillic vs Latin 'c'
- `х` (U+0445) vs `x` (U+0078) - Cyrillic vs Latin 'x'

**Greek/Latin Confusables:**
- `α` (U+03B1) vs `a` (U+0061) - Greek vs Latin 'a'
- `ε` (U+03B5) vs `e` (U+0065) - Greek vs Latin 'e'
- `ο` (U+03BF) vs `o` (U+006F) - Greek vs Latin 'o'
- `ρ` (U+03C1) vs `p` (U+0070) - Greek vs Latin 'p'

#### Example

```script
// These identifiers are confusable and will trigger warnings
let а = 42;  // Cyrillic 'a' (U+0430)
let a = 43;  // Latin 'a' (U+0061)
// Warning: Identifier 'а' may be confusable with other identifiers
```

## Security Configuration

### Security Levels

Script provides three security levels for Unicode handling:

#### 1. Strict Mode (`UnicodeSecurityLevel::Strict`)

- **Behavior**: Rejects confusable identifiers completely
- **Use Case**: High-security environments, financial applications
- **Impact**: Compilation fails on confusable identifier detection

```rust
use script::lexer::{Lexer, UnicodeSecurityConfig, UnicodeSecurityLevel};

let config = UnicodeSecurityConfig {
    level: UnicodeSecurityLevel::Strict,
    normalize_identifiers: true,
    detect_confusables: true,
};

let lexer = Lexer::with_unicode_config(input, config)?;
```

#### 2. Warning Mode (`UnicodeSecurityLevel::Warning`) - Default

- **Behavior**: Issues warnings for confusable identifiers but allows compilation
- **Use Case**: Development environments, general applications
- **Impact**: Warnings help developers identify potential issues

```rust
let config = UnicodeSecurityConfig {
    level: UnicodeSecurityLevel::Warning,
    normalize_identifiers: true,
    detect_confusables: true,
};
```

#### 3. Permissive Mode (`UnicodeSecurityLevel::Permissive`)

- **Behavior**: Allows all confusable identifiers without warnings
- **Use Case**: International applications with heavy Unicode usage
- **Impact**: Maximum flexibility, minimal security enforcement

```rust
let config = UnicodeSecurityConfig {
    level: UnicodeSecurityLevel::Permissive,
    normalize_identifiers: true,
    detect_confusables: true,
};
```

### Feature Toggles

You can independently control normalization and confusable detection:

```rust
// Only normalization, no confusable detection
let config = UnicodeSecurityConfig {
    level: UnicodeSecurityLevel::Warning,
    normalize_identifiers: true,
    detect_confusables: false,
};

// No Unicode processing at all
let config = UnicodeSecurityConfig {
    level: UnicodeSecurityLevel::Permissive,
    normalize_identifiers: false,
    detect_confusables: false,
};
```

## Performance Characteristics

### ASCII Fast Path

Script optimizes for ASCII-only identifiers, which make up the majority of code in most applications:

- **ASCII identifiers**: Zero Unicode processing overhead
- **Unicode identifiers**: 5-10% normalization overhead
- **Confusable detection**: 10-15% additional overhead with caching benefits

### Caching System

Script implements aggressive caching to minimize repeated Unicode operations:

```rust
// First occurrence: Full Unicode processing
let café = 42;

// Subsequent occurrences: Cache hit, no processing
let café = 43;
let café = 44;
```

### Memory Efficiency

- **String interning**: Eliminates duplicate Unicode strings
- **Compact caching**: Minimal memory overhead for Unicode metadata
- **Lazy loading**: Unicode data loaded only when needed

## Best Practices

### 1. Choose Appropriate Security Level

- **Use Strict**: For security-critical applications (finance, healthcare)
- **Use Warning**: For general development (recommended default)
- **Use Permissive**: For international applications with heavy Unicode

### 2. Handle Warnings Appropriately

```script
// Good: Use consistent character sets
let userName = "alice";     // All Latin
let файлИмя = "test.txt";   // All Cyrillic

// Avoid: Mixing similar characters from different scripts
let usеrName = "alice";     // Mixed Latin/Cyrillic - confusing!
```

### 3. Use Meaningful Identifiers

```script
// Good: Clear, unambiguous identifiers
let user_count = 0;
let файл_размер = 1024;

// Avoid: Single character variables that might be confusable
let а = 1;  // Cyrillic 'a'
let a = 2;  // Latin 'a' - confusing!
```

### 4. Consider Your Audience

- **International teams**: Use Unicode identifiers in native languages
- **ASCII-only environments**: Stick to ASCII for maximum compatibility
- **Mixed environments**: Use Warning level for safety

## Integration Examples

### Basic Usage

```rust
use script::lexer::{Lexer, UnicodeSecurityConfig};

// Use default security settings
let lexer = Lexer::new(source_code)?;
let (tokens, errors) = lexer.scan_tokens();

// Check for Unicode security warnings
for error in &errors {
    if error.to_string().contains("confusable") {
        println!("Unicode security warning: {}", error);
    }
}
```

### Custom Configuration

```rust
use script::lexer::{Lexer, UnicodeSecurityConfig, UnicodeSecurityLevel};

// Configure for high-security environment
let config = UnicodeSecurityConfig {
    level: UnicodeSecurityLevel::Strict,
    normalize_identifiers: true,
    detect_confusables: true,
};

let lexer = Lexer::with_unicode_config(source_code, config)?;
let (tokens, errors) = lexer.scan_tokens();

// In strict mode, confusable identifiers cause compilation errors
if !errors.is_empty() {
    eprintln!("Unicode security violations detected!");
    for error in errors {
        eprintln!("  {}", error);
    }
    return Err("Compilation failed due to security violations".into());
}
```

### Performance Monitoring

```rust
use script::lexer::{Lexer, UnicodeSecurityConfig};
use std::time::Instant;

let start = Instant::now();
let lexer = Lexer::with_unicode_config(source_code, config)?;
let (tokens, errors) = lexer.scan_tokens();
let duration = start.elapsed();

println!("Lexed {} tokens in {:?}", tokens.len(), duration);
println!("Unicode security processing: {}ms", 
         duration.as_millis());
```

## Security Considerations

### 1. Homograph Attacks

Unicode homograph attacks use visually similar characters to create malicious identifiers that appear legitimate.

**Example Attack:**
```script
// Malicious code using Cyrillic 'a' to mimic legitimate function
fn аuthenticate_user(password: string) -> bool {
    // Malicious implementation that always returns true
    return true;
}

// Legitimate function using Latin 'a'
fn authenticate_user(password: string) -> bool {
    // Secure implementation
    return check_password(password);
}
```

**Script Protection:**
```
Warning: Identifier 'аuthenticate_user' may be confusable with other identifiers (skeleton: 'authenticate_user')
```

### 2. Mixed Script Confusion

Mixing characters from different scripts can create confusion even without direct spoofing.

**Example:**
```script
// Confusing mixed-script identifiers
let usеr_nаme = "alice";  // Mixed Latin/Cyrillic
let user_name = "bob";    // All Latin

// Script warns about potential confusion
```

### 3. Normalization Bypass Attempts

Attackers might try to use different normalization forms to bypass security.

**Example:**
```script
// These normalize to the same identifier
let café = 1;  // NFC form (single character)
let café = 2;  // NFD form (base + combining)

// Script normalizes both to NFKC, detecting the collision
```

## Migration Guide

### From ASCII-Only Code

If your codebase currently uses only ASCII identifiers, no changes are needed:

```script
// This code works unchanged
fn calculate_sum(a: int, b: int) -> int {
    return a + b;
}
```

### Adding Unicode Support

To add Unicode identifiers safely:

1. **Start with Warning level** to identify potential issues
2. **Review warnings** and resolve confusable identifier conflicts
3. **Choose consistent character sets** for related identifiers
4. **Document Unicode usage** in your team's coding standards

### Upgrading Security Level

To upgrade from Permissive to Warning or Strict:

1. **Test with Warning level** first to identify issues
2. **Resolve confusable identifier warnings**
3. **Update build scripts** to handle new error types
4. **Train developers** on Unicode security best practices

## Troubleshooting

### Common Issues

**Issue**: "Confusable identifier" warnings for legitimate code
**Solution**: Use consistent character sets or rename conflicting identifiers

**Issue**: Performance degradation with Unicode-heavy code
**Solution**: Monitor cache hit rates and consider pre-warming caches

**Issue**: Compilation failures in Strict mode
**Solution**: Review and resolve all confusable identifier conflicts

### Debug Information

Enable debug logging to see Unicode processing details:

```rust
// This would be implementation-specific debug output
// showing normalization and confusable detection results
```

## References

- [Unicode Technical Report #31: Unicode Identifier and Pattern Syntax](https://unicode.org/reports/tr31/)
- [Unicode Technical Report #36: Unicode Security Considerations](https://unicode.org/reports/tr36/)
- [Unicode Technical Report #39: Unicode Security Mechanisms](https://unicode.org/reports/tr39/)
- [NFKC Normalization Specification](https://unicode.org/reports/tr15/)

## API Reference

### Types

- `UnicodeSecurityLevel`: Security level enumeration
- `UnicodeSecurityConfig`: Configuration structure
- `Lexer`: Main lexer with Unicode support

### Functions

- `Lexer::new(input)`: Create lexer with default Unicode settings
- `Lexer::with_unicode_config(input, config)`: Create lexer with custom settings
- `lexer.unicode_config()`: Get current configuration
- `lexer.set_unicode_config(config)`: Update configuration

### Configuration Options

- `level`: Security enforcement level
- `normalize_identifiers`: Enable/disable NFKC normalization
- `detect_confusables`: Enable/disable confusable character detection