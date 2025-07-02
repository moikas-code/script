# Contributing to Script Programming Language

Thank you for your interest in contributing to the Script programming language! This guide will help you get started with contributing to our project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Pull Request Process](#pull-request-process)
- [Issue Guidelines](#issue-guidelines)
- [Testing Requirements](#testing-requirements)
- [Documentation Standards](#documentation-standards)
- [Performance Considerations](#performance-considerations)

## Code of Conduct

We are committed to providing a welcoming and inclusive environment for all contributors. Please be respectful, constructive, and professional in all interactions.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/your-username/script.git
   cd script
   ```
3. **Set up your development environment** (see [SETUP.md](SETUP.md))
4. **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

## Development Workflow

### Branch Naming Convention

Use descriptive branch names with prefixes:
- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation updates
- `refactor/` - Code refactoring
- `test/` - Test improvements
- `perf/` - Performance improvements

Examples:
- `feature/add-pattern-matching`
- `fix/lexer-unicode-handling`
- `docs/update-api-examples`

### Commit Message Guidelines

Follow conventional commit format:
```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, missing semicolons, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `perf`: Performance improvements
- `chore`: Build process or auxiliary tool changes

**Examples:**
```
feat(parser): add pattern matching support

Implement pattern matching for match expressions including:
- Basic literal patterns
- Variable binding patterns
- Wildcard patterns
- Guard clauses

Closes #123
```

```
fix(lexer): handle Unicode identifiers correctly

Fix issue where Unicode characters in identifiers were not
properly categorized, causing parsing errors.

Fixes #456
```

## Coding Standards

### Rust Code Style

We follow the standard Rust formatting and naming conventions:

#### Naming Conventions
- **Functions and variables**: `snake_case`
  ```rust
  fn parse_expression() -> Result<Expr, ParseError> { ... }
  let token_kind = TokenKind::Identifier;
  ```

- **Types and traits**: `PascalCase`
  ```rust
  struct SyntaxNode { ... }
  enum TokenKind { ... }
  trait Visitor { ... }
  ```

- **Constants**: `SCREAMING_SNAKE_CASE`
  ```rust
  const MAX_RECURSION_DEPTH: usize = 1000;
  ```

- **Modules**: `snake_case`
  ```rust
  mod error_handling;
  ```

#### Code Organization

1. **Use descriptive names**:
   ```rust
   // Good
   fn parse_binary_expression(left: Expr, precedence: u8) -> Result<Expr, ParseError>
   
   // Avoid
   fn parse_bin_expr(l: Expr, p: u8) -> Result<Expr, ParseError>
   ```

2. **Keep functions focused and small** (preferably under 50 lines)

3. **Use meaningful comments for complex logic**:
   ```rust
   // Parse expressions using Pratt parsing with operator precedence
   fn parse_expression_with_precedence(&mut self, min_precedence: u8) -> Result<Expr, ParseError> {
       // ... implementation
   }
   ```

4. **Prefer explicit error handling**:
   ```rust
   // Good
   match self.advance() {
       Ok(token) => process_token(token),
       Err(e) => return Err(ParseError::UnexpectedToken(e)),
   }
   
   // Avoid unwrap() in production code
   let token = self.advance().unwrap(); // Don't do this
   ```

### Documentation Standards

#### Public API Documentation
All public functions, structs, and modules must have documentation:

```rust
/// Parses a Script source file into an Abstract Syntax Tree.
///
/// # Arguments
/// 
/// * `source` - The source code to parse
/// * `file_name` - Optional file name for error reporting
///
/// # Returns
///
/// Returns `Ok(Program)` on successful parsing, or `Err(Vec<ParseError>)`
/// containing all parsing errors encountered.
///
/// # Examples
///
/// ```rust
/// use script::{Parser, Lexer};
/// 
/// let source = "let x = 42";
/// let lexer = Lexer::new(source);
/// let (tokens, _) = lexer.scan_tokens();
/// let mut parser = Parser::new(tokens);
/// let program = parser.parse().unwrap();
/// ```
pub fn parse(&mut self) -> Result<Program, Vec<ParseError>> {
    // Implementation...
}
```

#### Internal Documentation
Use comments to explain complex algorithms or business logic:

```rust
impl Parser {
    /// Implements Pratt parsing for expressions with operator precedence.
    /// 
    /// This uses the "precedence climbing" algorithm where each recursive
    /// call handles operators of a minimum precedence level.
    fn parse_expression_pratt(&mut self, min_precedence: u8) -> Result<Expr, ParseError> {
        // Start with a primary expression (literal, identifier, grouped)
        let mut left = self.parse_primary()?;
        
        // Continue parsing binary operators while their precedence
        // is greater than or equal to our minimum
        while let Some(precedence) = self.current_operator_precedence() {
            if precedence < min_precedence {
                break;
            }
            // ... rest of implementation
        }
        
        Ok(left)
    }
}
```

### Error Handling Standards

1. **Use appropriate error types**:
   ```rust
   #[derive(Debug, Clone)]
   pub enum ParseError {
       UnexpectedToken { expected: String, found: Token },
       UnexpectedEof,
       InvalidSyntax { message: String },
       // ...
   }
   ```

2. **Provide helpful error messages**:
   ```rust
   return Err(ParseError::UnexpectedToken {
       expected: "expression".to_string(),
       found: self.current_token.clone(),
   });
   ```

3. **Use `Result` for recoverable errors**, `panic!` only for programmer errors

### Performance Guidelines

1. **Avoid unnecessary allocations** in hot paths
2. **Use `&str` instead of `String` when possible**
3. **Consider using `Cow<str>` for flexible string handling**
4. **Profile before optimizing** - use `cargo bench` to measure
5. **Use appropriate data structures**:
   - `Vec` for sequential access
   - `HashMap` for key-value lookups
   - `BTreeMap` for ordered key-value pairs

## Testing Requirements

### Unit Tests
- **Every module must have comprehensive unit tests**
- **Test files should be named `tests.rs` within each module**
- **Cover both happy path and error cases**

Example test structure:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple_expression() {
        let source = "2 + 3";
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.scan_tokens();
        let mut parser = Parser::new(tokens);
        
        let expr = parser.parse_expression().unwrap();
        
        match expr {
            Expr::Binary { left, op, right } => {
                assert_eq!(*left, Expr::Literal(Value::Number(2.0)));
                assert_eq!(op, BinaryOp::Add);
                assert_eq!(*right, Expr::Literal(Value::Number(3.0)));
            }
            _ => panic!("Expected binary expression"),
        }
    }
    
    #[test]
    fn test_parse_invalid_syntax_returns_error() {
        let source = "2 +";  // Missing right operand
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.scan_tokens();
        let mut parser = Parser::new(tokens);
        
        let result = parser.parse_expression();
        assert!(result.is_err());
    }
}
```

### Integration Tests
Place integration tests in `tests/` directory to test end-to-end functionality.

### Property-Based Testing
Use `proptest` for testing invariants:
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_lexer_roundtrip(s in "\\PC*") {
        // Test that lexing and then reconstructing gives original source
        // (within whitespace normalization)
    }
}
```

## Pull Request Process

### Before Submitting

1. **Ensure all tests pass**:
   ```bash
   cargo test
   ```

2. **Run benchmarks** to check for performance regressions:
   ```bash
   cargo bench
   ```

3. **Check code formatting**:
   ```bash
   cargo fmt --check
   ```

4. **Run Clippy** for additional linting:
   ```bash
   cargo clippy -- -D warnings
   ```

5. **Generate and review documentation**:
   ```bash
   cargo doc --open
   ```

### PR Template

When submitting a PR, please include:

```markdown
## Description
Brief description of changes and motivation.

## Type of Change
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Benchmarks run (include results if performance-related)

## Checklist
- [ ] Code follows the style guidelines
- [ ] Self-review completed
- [ ] Code is commented, particularly in hard-to-understand areas
- [ ] Documentation updated
- [ ] No new warnings introduced
```

### Review Process

1. **All PRs require at least one review**
2. **Address all feedback** before merge
3. **Squash commits** if requested to keep history clean
4. **Update documentation** if API changes are made

## Issue Guidelines

### Bug Reports

Include:
- **Script version**
- **Operating system and version**
- **Minimal reproduction case**
- **Expected vs actual behavior**
- **Error messages or stack traces**

### Feature Requests

Include:
- **Use case description**
- **Proposed API or syntax**
- **Implementation considerations**
- **Alternative solutions considered**

### Security Issues

**Do not create public issues for security vulnerabilities.** Instead, email the maintainers directly with details.

## Recognition

Contributors will be recognized in:
- `AUTHORS.md` file
- Release notes for significant contributions
- GitHub contributors page

Thank you for contributing to Script! Your efforts help make the language better for everyone.