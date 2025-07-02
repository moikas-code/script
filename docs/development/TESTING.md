# Testing Guide

This guide covers the testing philosophy, practices, and tools used in the Script programming language project.

## Table of Contents

- [Testing Philosophy](#testing-philosophy)
- [Test Categories](#test-categories)
- [Running Tests](#running-tests)
- [Writing Tests](#writing-tests)
- [Test Organization](#test-organization)
- [Property-Based Testing](#property-based-testing)
- [Performance Testing](#performance-testing)
- [Coverage Analysis](#coverage-analysis)
- [Debugging Tests](#debugging-tests)
- [Continuous Integration](#continuous-integration)

## Testing Philosophy

The Script language project follows a comprehensive testing strategy based on these principles:

1. **Test-Driven Development**: Write tests before implementing features when possible
2. **Comprehensive Coverage**: Aim for high code coverage, especially for critical paths
3. **Fast Feedback**: Tests should run quickly to enable rapid development
4. **Reliable Tests**: Tests should be deterministic and not flaky
5. **Clear Documentation**: Tests serve as living documentation of expected behavior

### Testing Pyramid

```
    /\      End-to-End Tests (Few)
   /  \     ├─ CLI integration tests
  /____\    ├─ REPL behavior tests
 /      \   └─ Cross-module integration
/________\  Integration Tests (Some)
           ├─ Parser + Lexer integration
           ├─ Semantic analysis flow
           └─ Code generation pipeline
           Unit Tests (Many)
           ├─ Individual function tests
           ├─ Module-level tests
           └─ Error condition tests
```

## Test Categories

### 1. Unit Tests
Test individual functions, methods, and small units of code in isolation.

**Location**: `src/**/tests.rs` files within each module

**Examples**:
- Lexer token recognition
- Parser expression parsing
- Individual AST node operations
- Error formatting functions

### 2. Integration Tests
Test interactions between multiple modules or components.

**Location**: `tests/` directory

**Examples**:
- Full parsing pipeline (lexer → parser → AST)
- Error reporting across modules
- REPL command processing

### 3. Property-Based Tests
Test properties and invariants that should hold across a wide range of inputs.

**Location**: Alongside unit tests using `proptest`

**Examples**:
- Lexer roundtrip properties (tokenize → reconstruct)
- Parser invariants (valid AST structure)
- Semantic analysis consistency

### 4. Performance Tests (Benchmarks)
Measure and track performance characteristics.

**Location**: `benches/` directory

**Examples**:
- Lexing performance across different input sizes
- Parsing complexity for nested expressions
- Memory usage patterns

### 5. End-to-End Tests
Test complete user workflows through the CLI interface.

**Location**: `tests/integration/` directory

**Examples**:
- Script file execution
- REPL session flows
- Error message formatting

## Running Tests

### Basic Test Commands

```bash
# Run all tests
cargo test

# Run tests with output visible (don't capture stdout/stderr)
cargo test -- --nocapture

# Run tests in a specific module
cargo test lexer
cargo test parser
cargo test semantic

# Run a specific test by name
cargo test test_parse_binary_expression

# Run tests with multiple threads (default)
cargo test

# Run tests single-threaded (useful for debugging)
cargo test -- --test-threads=1

# Run only failing tests
cargo test --no-fail-fast
```

### Test Filtering

```bash
# Run only tests matching a pattern
cargo test token

# Run tests and ignored tests
cargo test -- --ignored

# Run only ignored tests
cargo test -- --ignored --quiet

# Show test output even for passing tests
cargo test -- --show-output
```

### Integration Tests

```bash
# Run all integration tests
cargo test --test '*'

# Run specific integration test file
cargo test --test parser_integration

# Run integration tests with full output
cargo test --test '*' -- --nocapture
```

### Test Profiles

```bash
# Run tests in release mode (slower compilation, faster execution)
cargo test --release

# Run tests with specific features
cargo test --features "debug-mode"

# Run tests without default features
cargo test --no-default-features
```

## Writing Tests

### Unit Test Structure

#### Basic Test Template
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    /// Test function naming convention: test_[component]_[behavior]_[condition]
    #[test]
    fn test_lexer_scans_identifier_correctly() {
        // Arrange
        let source = "hello_world";
        let lexer = Lexer::new(source);
        
        // Act
        let (tokens, errors) = lexer.scan_tokens();
        
        // Assert
        assert_eq!(errors.len(), 0);
        assert_eq!(tokens.len(), 2); // identifier + EOF
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].lexeme, "hello_world");
    }
    
    #[test]
    fn test_parser_handles_invalid_syntax_gracefully() {
        // Test error conditions
        let source = "let x = ";  // Incomplete expression
        let mut parser = create_parser(source);
        
        let result = parser.parse_let_statement();
        
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::UnexpectedEof => {}, // Expected error type
            _ => panic!("Expected UnexpectedEof error"),
        }
    }
    
    /// Helper function for test setup
    fn create_parser(source: &str) -> Parser {
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.scan_tokens();
        Parser::new(tokens)
    }
}
```

#### Testing Error Conditions
```rust
#[test]
fn test_lexer_reports_unterminated_string() {
    let source = r#""unterminated string"#;
    let lexer = Lexer::new(source);
    let (tokens, errors) = lexer.scan_tokens();
    
    assert_eq!(errors.len(), 1);
    assert!(matches!(errors[0], LexError::UnterminatedString { .. }));
}

#[test]
#[should_panic(expected = "Stack overflow")]
fn test_parser_prevents_infinite_recursion() {
    // Create deeply nested expression that could cause stack overflow
    let source = "(".repeat(10000) + &")".repeat(10000);
    let mut parser = create_parser(&source);
    parser.parse_expression().unwrap(); // Should panic with controlled error
}
```

#### Testing with Fixtures
```rust
#[test]
fn test_parser_handles_complex_expressions() {
    let test_cases = vec![
        ("2 + 3 * 4", "Binary(Literal(2), Add, Binary(Literal(3), Mul, Literal(4)))"),
        ("(2 + 3) * 4", "Binary(Binary(Literal(2), Add, Literal(3)), Mul, Literal(4))"),
        ("a.b.c", "Member(Member(Identifier(a), b), c)"),
    ];
    
    for (input, expected_debug) in test_cases {
        let mut parser = create_parser(input);
        let expr = parser.parse_expression().unwrap();
        assert_eq!(format!("{:?}", expr), expected_debug, "Failed for input: {}", input);
    }
}
```

### Property-Based Testing

Use `proptest` for testing properties that should hold across many inputs:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_lexer_roundtrip_property(source in "\\PC*") {
        // Property: lexing then reconstructing should preserve meaningful content
        let lexer = Lexer::new(&source);
        let (tokens, errors) = lexer.scan_tokens();
        
        // Skip if lexing produces errors (invalid input)
        prop_assume!(errors.is_empty());
        
        let reconstructed = reconstruct_source_from_tokens(&tokens);
        let lexer2 = Lexer::new(&reconstructed);
        let (tokens2, errors2) = lexer2.scan_tokens();
        
        prop_assert_eq!(errors2.len(), 0);
        prop_assert_eq!(tokens.len(), tokens2.len());
        
        // Token kinds should match (lexemes might differ due to whitespace normalization)
        for (t1, t2) in tokens.iter().zip(tokens2.iter()) {
            prop_assert_eq!(t1.kind, t2.kind);
        }
    }
    
    #[test]
    fn test_parser_expression_depth_bounded(
        depth in 1u32..100,
        op in prop::sample::select(vec!["+", "-", "*", "/"])
    ) {
        // Property: Parser should handle expressions up to reasonable depth
        let expr = generate_nested_expression(depth, &op);
        let mut parser = create_parser(&expr);
        
        let result = parser.parse_expression();
        
        // Should either parse successfully or fail gracefully (no panics)
        match result {
            Ok(ast) => {
                prop_assert!(get_expression_depth(&ast) <= depth + 1);
            }
            Err(_) => {
                // Graceful failure is acceptable for very deep expressions
                prop_assert!(depth > 50); // Only expect failures for deep nesting
            }
        }
    }
}

fn generate_nested_expression(depth: u32, op: &str) -> String {
    if depth == 0 {
        "1".to_string()
    } else {
        format!("({} {} {})", 
                generate_nested_expression(depth - 1, op), 
                op, 
                generate_nested_expression(depth - 1, op))
    }
}

fn get_expression_depth(expr: &Expr) -> u32 {
    match expr {
        Expr::Literal(_) | Expr::Identifier(_) => 1,
        Expr::Binary { left, right, .. } => {
            1 + get_expression_depth(left).max(get_expression_depth(right))
        }
        Expr::Call { callee, args } => {
            1 + get_expression_depth(callee).max(
                args.iter().map(get_expression_depth).max().unwrap_or(0)
            )
        }
        // ... handle other expression types
    }
}
```

### Integration Tests

Create files in `tests/` directory:

```rust
// tests/parser_integration.rs
use script::{Lexer, Parser, Program};

#[test]
fn test_full_parsing_pipeline() {
    let source = r#"
        fn fibonacci(n: i32) -> i32 {
            if n <= 1 {
                return n
            }
            return fibonacci(n - 1) + fibonacci(n - 2)
        }
        
        let result = fibonacci(10)
    "#;
    
    // Test complete pipeline
    let lexer = Lexer::new(source);
    let (tokens, lex_errors) = lexer.scan_tokens();
    assert_eq!(lex_errors.len(), 0, "Lexing should succeed");
    
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    
    // Verify program structure
    assert_eq!(program.statements.len(), 2);
    
    // Verify function declaration
    match &program.statements[0] {
        Statement::Function { name, params, return_type, body } => {
            assert_eq!(name, "fibonacci");
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "n");
            // ... more assertions
        }
        _ => panic!("Expected function declaration"),
    }
}

#[test]
fn test_error_recovery_across_modules() {
    let source = "let x = + 5"; // Invalid: missing left operand
    
    let lexer = Lexer::new(source);
    let (tokens, _) = lexer.scan_tokens();
    
    let mut parser = Parser::new(tokens);
    let result = parser.parse();
    
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert_eq!(errors.len(), 1);
    
    // Verify error contains helpful information
    let error = &errors[0];
    assert!(error.message.contains("expression"));
    assert_eq!(error.location.line, 1);
    assert_eq!(error.location.column, 9); // Position of '+'
}
```

## Test Organization

### Module-Level Tests
Each module should have its own `tests.rs` file:

```
src/
├── lexer/
│   ├── mod.rs
│   ├── scanner.rs
│   ├── token.rs
│   └── tests.rs        # Lexer unit tests
├── parser/
│   ├── mod.rs
│   ├── parser.rs
│   ├── ast.rs
│   └── tests.rs        # Parser unit tests
└── semantic/
    ├── mod.rs
    ├── analyzer.rs
    └── tests.rs        # Semantic analysis tests
```

### Test Utilities
Create common test utilities in `src/test_utils.rs`:

```rust
// src/test_utils.rs
#![cfg(test)]

use crate::{Lexer, Parser, Token, TokenKind};

/// Helper to create a parser from source code
pub fn parse_source(source: &str) -> Result<Program, Vec<ParseError>> {
    let lexer = Lexer::new(source);
    let (tokens, errors) = lexer.scan_tokens();
    
    if !errors.is_empty() {
        panic!("Lexing errors: {:#?}", errors);
    }
    
    let mut parser = Parser::new(tokens);
    parser.parse()
}

/// Helper to create tokens for parser testing
pub fn create_tokens(kinds: &[TokenKind]) -> Vec<Token> {
    kinds.iter().enumerate().map(|(i, kind)| Token {
        kind: kind.clone(),
        lexeme: kind.default_lexeme(),
        location: SourceLocation::new(1, i + 1),
    }).collect()
}

/// Helper to assert AST structure matches expected pattern
pub fn assert_ast_matches(actual: &Expr, expected: &str) {
    let actual_str = format!("{:?}", actual);
    assert!(actual_str.contains(expected), 
            "Expected AST to contain '{}', got '{}'", expected, actual_str);
}

/// Snapshot testing helper
pub fn assert_snapshot(name: &str, content: &str) {
    let snapshot_path = format!("tests/snapshots/{}.snap", name);
    
    if std::env::var("UPDATE_SNAPSHOTS").is_ok() {
        std::fs::create_dir_all("tests/snapshots").unwrap();
        std::fs::write(&snapshot_path, content).unwrap();
    } else {
        let expected = std::fs::read_to_string(&snapshot_path)
            .unwrap_or_else(|_| panic!("Snapshot file not found: {}", snapshot_path));
        assert_eq!(content.trim(), expected.trim(), "Snapshot mismatch for {}", name);
    }
}
```

### Test Data Organization
```
tests/
├── fixtures/
│   ├── valid_programs/
│   │   ├── hello_world.script
│   │   ├── fibonacci.script
│   │   └── complex_types.script
│   ├── invalid_programs/
│   │   ├── syntax_errors.script
│   │   └── type_errors.script
│   └── performance/
│       ├── large_program.script
│       └── deep_nesting.script
├── snapshots/
│   ├── lexer_output.snap
│   └── parser_ast.snap
└── integration/
    ├── cli_tests.rs
    └── repl_tests.rs
```

## Performance Testing

### Benchmark Structure
```rust
// benches/lexer.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use script::Lexer;

fn benchmark_lexer_performance(c: &mut Criterion) {
    let inputs = vec![
        ("small", include_str!("../tests/fixtures/performance/small.script")),
        ("medium", include_str!("../tests/fixtures/performance/medium.script")),
        ("large", include_str!("../tests/fixtures/performance/large.script")),
    ];
    
    for (size, source) in inputs {
        c.bench_with_input(
            BenchmarkId::new("lexer_scan_tokens", size),
            source,
            |b, source| {
                b.iter(|| {
                    let lexer = Lexer::new(black_box(source));
                    let (tokens, _) = lexer.scan_tokens();
                    black_box(tokens);
                });
            },
        );
    }
}

fn benchmark_memory_usage(c: &mut Criterion) {
    let source = include_str!("../tests/fixtures/performance/large.script");
    
    c.bench_function("lexer_memory_usage", |b| {
        b.iter_custom(|iters| {
            let start = std::time::Instant::now();
            
            for _ in 0..iters {
                let lexer = Lexer::new(black_box(source));
                let (tokens, _) = lexer.scan_tokens();
                black_box(tokens);
                // Force memory cleanup
                std::mem::drop(lexer);
            }
            
            start.elapsed()
        });
    });
}

criterion_group!(benches, benchmark_lexer_performance, benchmark_memory_usage);
criterion_main!(benches);
```

### Running Benchmarks
```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench lexer

# Run benchmarks with detailed output
cargo bench -- --verbose

# Generate flamegraph (requires flamegraph tool)
cargo bench --bench lexer -- --profile-time=5

# Compare benchmark results
cargo bench -- --save-baseline main
# ... make changes ...
cargo bench -- --baseline main
```

## Coverage Analysis

### Installing Coverage Tools
```bash
# Install tarpaulin for coverage
cargo install cargo-tarpaulin

# Alternative: Install grcov (requires nightly)
rustup toolchain install nightly
cargo install grcov
```

### Generating Coverage Reports
```bash
# Generate HTML coverage report
cargo tarpaulin --out html

# Generate LCOV format for external tools
cargo tarpaulin --out lcov

# Exclude specific files from coverage
cargo tarpaulin --exclude-files "src/main.rs" --exclude-files "*/tests.rs"

# Run coverage for specific package
cargo tarpaulin --package script

# Generate coverage with test output
cargo tarpaulin --out html -- --nocapture
```

### Coverage Targets
- **Unit Tests**: Aim for >90% line coverage
- **Integration Tests**: Ensure all public APIs are tested
- **Error Paths**: All error conditions should be tested
- **Edge Cases**: Boundary conditions and corner cases

## Debugging Tests

### Debug Output
```rust
#[test]
fn test_with_debug_output() {
    // Use eprintln! for debug output (visible with --nocapture)
    eprintln!("Debug: Testing complex expression");
    
    let source = "2 + 3 * 4";
    let mut parser = create_parser(source);
    
    let expr = parser.parse_expression().unwrap();
    eprintln!("Parsed expression: {:#?}", expr);
    
    // Test assertions...
}
```

### Test-Specific Debugging
```bash
# Run single test with output
cargo test test_name -- --nocapture

# Run tests with backtrace on panic
RUST_BACKTRACE=1 cargo test

# Run tests with full backtrace
RUST_BACKTRACE=full cargo test

# Debug test with GDB/LLDB
cargo test --no-run  # Compile but don't run
gdb target/debug/deps/script-<hash>
# Set breakpoints and run specific test
```

### Test Isolation
```rust
use std::sync::Mutex;

// For tests that modify global state
lazy_static::lazy_static! {
    static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

#[test]
fn test_with_global_state() {
    let _guard = TEST_MUTEX.lock().unwrap();
    // Test that modifies global state
}
```

## Continuous Integration

### GitHub Actions Configuration
```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        rust: [stable, beta]
    
    steps:
    - uses: actions/checkout@v3
    
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: ${{ matrix.rust }}
        components: rustfmt, clippy
        override: true
    
    - name: Cache cargo
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Check formatting
      run: cargo fmt -- --check
    
    - name: Run clippy
      run: cargo clippy -- -D warnings
    
    - name: Run tests
      run: cargo test --verbose
    
    - name: Run benchmarks
      run: cargo bench --no-run
    
    - name: Generate coverage
      if: matrix.os == 'ubuntu-latest' && matrix.rust == 'stable'
      run: |
        cargo install cargo-tarpaulin
        cargo tarpaulin --out xml
    
    - name: Upload coverage
      if: matrix.os == 'ubuntu-latest' && matrix.rust == 'stable'
      uses: codecov/codecov-action@v3
```

### Test Quality Gates
Ensure these checks pass in CI:
- All tests pass on all platforms
- Code coverage meets minimum threshold (e.g., 85%)
- No clippy warnings
- Proper code formatting
- Benchmarks compile and run
- Documentation builds without warnings

## Best Practices Summary

1. **Write Tests First**: Use TDD when implementing new features
2. **Test Error Cases**: Don't just test the happy path
3. **Use Descriptive Names**: Test names should describe what they test
4. **Keep Tests Simple**: Each test should verify one specific behavior
5. **Use Test Utilities**: Create helpers to reduce boilerplate
6. **Mock External Dependencies**: Keep tests isolated and fast
7. **Test at the Right Level**: Unit tests for logic, integration tests for workflows
8. **Maintain Test Quality**: Refactor tests as you refactor code
9. **Use Property-Based Testing**: For testing invariants across many inputs
10. **Monitor Performance**: Use benchmarks to catch performance regressions

Following these testing practices ensures the Script language remains reliable, maintainable, and performant as it evolves.