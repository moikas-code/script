# Contributing to Script

Thank you for your interest in contributing to Script! This document provides guidelines and instructions for contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [How to Contribute](#how-to-contribute)
- [Pull Request Process](#pull-request-process)
- [Coding Standards](#coding-standards)
- [Testing Guidelines](#testing-guidelines)
- [Documentation](#documentation)
- [Issue Guidelines](#issue-guidelines)

## Code of Conduct

By participating in this project, you agree to abide by our Code of Conduct:

- Be respectful and inclusive
- Welcome newcomers and help them get started
- Focus on constructive criticism
- Accept feedback gracefully
- Prioritize the project's best interests

## Getting Started

1. **Check existing issues** - Look for issues labeled `good first issue` or `help wanted`
2. **Read the documentation** - Familiarize yourself with:
   - [README.md](README.md) - Project overview
   - [CLAUDE.md](CLAUDE.md) - Development guide
   - [kb/active/KNOWN_ISSUES.md](kb/active/KNOWN_ISSUES.md) - Current bugs and limitations

## Development Setup

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Git
- A code editor (VS Code with rust-analyzer recommended)

### Initial Setup

```bash
# Clone the repository
git clone https://github.com/moikapy/script.git
cd script

# Build the project
cargo build

# Run tests
cargo test

# Run the REPL
cargo run

# Build with all features
cargo build --all-features
```

### Running Benchmarks

```bash
cargo bench
```

## How to Contribute

### Finding Something to Work On

1. Check [Issues](https://github.com/moikapy/script/issues) for:
   - `good first issue` - Great for newcomers
   - `help wanted` - Community help needed
   - `bug` - Bug fixes
   - `enhancement` - New features

2. Review the [kb/active/](kb/active/) directory for ongoing work

3. Current priorities (as of v0.5.0-alpha):
   - Improving error messages
   - REPL enhancements
   - MCP integration
   - Performance optimizations

### Before Starting Work

1. **Comment on the issue** to claim it
2. **Ask questions** if anything is unclear
3. **Discuss the approach** for larger changes
4. **Check for related work** to avoid conflicts

## Pull Request Process

### 1. Fork and Branch

```bash
# Fork the repository on GitHub, then:
git clone https://github.com/YOUR_USERNAME/script.git
cd script
git remote add upstream https://github.com/moikapy/script.git

# Create a feature branch
git checkout -b feature/your-feature-name
```

### 2. Make Your Changes

- Write clean, idiomatic Rust code
- Add tests for new functionality
- Update documentation as needed
- Follow the coding standards below

### 3. Test Your Changes

```bash
# Format your code
cargo fmt

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test

# Run tests with MCP features
cargo test --features mcp

# Check documentation
cargo doc --no-deps --all-features
```

### 4. Commit Your Changes

Use conventional commit messages:

```
feat: add new array methods to stdlib
fix: resolve memory leak in closure capture
docs: update REPL usage guide
test: add tests for pattern matching edge cases
refactor: simplify type inference algorithm
perf: optimize lexer tokenization
```

### 5. Submit Pull Request

1. Push to your fork
2. Create a pull request to `main` branch
3. Fill out the PR template
4. Wait for review and address feedback

## Coding Standards

### Rust Style

- Follow standard Rust naming conventions
- Use `cargo fmt` for formatting
- Address all `cargo clippy` warnings
- Prefer clarity over cleverness

### Code Organization

```rust
// Good: Clear module organization
pub mod lexer {
    mod scanner;
    mod token;
    
    pub use scanner::Scanner;
    pub use token::{Token, TokenKind};
}

// Good: Clear error handling
fn parse_expression(&mut self) -> Result<Expr, ParseError> {
    // implementation
}
```

### Documentation

```rust
/// Parses a Script source file into an AST.
/// 
/// # Arguments
/// 
/// * `source` - The source code to parse
/// 
/// # Returns
/// 
/// Returns `Ok(Program)` on success, or `Err(ParseError)` on failure.
/// 
/// # Examples
/// 
/// ```
/// use script::parser::parse;
/// 
/// let ast = parse("let x = 42").unwrap();
/// ```
pub fn parse(source: &str) -> Result<Program, ParseError> {
    // implementation
}
```

## Testing Guidelines

### Test Organization

- Unit tests go in the same file as the code
- Integration tests go in `tests/`
- Use descriptive test names

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_handles_unicode() {
        let input = "let 世界 = \"hello\"";
        let tokens = tokenize(input);
        assert_eq!(tokens[1].lexeme, "世界");
    }
    
    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn test_array_bounds_checking() {
        let arr = vec![1, 2, 3];
        let _ = arr[10]; // Should panic
    }
}
```

### Test Coverage

- Aim for >80% coverage for new code
- Test edge cases and error conditions
- Include both positive and negative tests

## Documentation

### Code Documentation

- Document all public APIs
- Include examples in doc comments
- Explain complex algorithms
- Update relevant KB files

### KB (Knowledge Base) Updates

When making significant changes:

1. Update relevant files in `kb/`
2. Move completed issues from `kb/active/` to `kb/completed/`
3. Update `kb/status/OVERALL_STATUS.md` if needed

## Issue Guidelines

### Reporting Bugs

Include:
- Script version
- Operating system
- Minimal reproduction code
- Expected vs actual behavior
- Error messages/stack traces

### Feature Requests

Include:
- Use case description
- Proposed syntax/API
- Examples of usage
- Impact on existing features

### Good Issue Example

```markdown
## Bug: Closure capture of mutable variables fails

### Version
Script v0.5.0-alpha

### Description
When capturing mutable variables in closures, the compiler panics.

### Reproduction
```script
let mut x = 10
let inc = || { x += 1 }  // Compiler panic here
inc()
```

### Expected
Should capture `x` by reference and allow mutation.

### Actual
Compiler panic: "cannot capture mutable variable"

### Environment
- OS: Ubuntu 22.04
- Rust: 1.75.0
```

## Need Help?

- Check the [documentation](docs/)
- Ask in [GitHub Discussions](https://github.com/moikapy/script/discussions)
- Review similar PRs/issues
- Tag `@moikapy` for guidance

## Recognition

Contributors are recognized in:
- The project README
- Release notes
- Annual contributor spotlight

Thank you for helping make Script better! 🚀