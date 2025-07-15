# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with the Script programming language repository.

- Format each individual memory as a bullet point and group related memories under descriptive markdown headings.

- Perform a web search if you need more information

- Create a Task in the ./kb when making a plan to keep track between sessions

- Never use placeholder logic when writing logic; provide a full solution

## Project Overview

**Script Language v0.5.0-alpha** - AI-native programming language with production-grade generics, pattern matching, memory cycle detection, and **complete functional programming support**. Major systems now complete (~90% overall), with remaining work on error messages, REPL enhancements, and MCP integration.

## Common Development Commands

```bash
# Build
cargo build              # Debug build
cargo build --release    # Release build

# Build with MCP support
cargo build --features mcp              # MCP development build
cargo build --release --features mcp    # MCP production build

# Run REPL
cargo run               # Interactive REPL (parse mode by default)
cargo run -- --tokens   # Run REPL in token mode

# Run MCP Server
cargo run --bin script-mcp --features mcp              # Development MCP server
cargo run --bin script-mcp --features mcp -- --strict-mode  # Strict security mode

# Parse Script files
cargo run examples/hello.script           # Parse and display AST
cargo run examples/hello.script --tokens  # Tokenize only

# Testing
cargo test                              # Run all tests
cargo test lexer                        # Test lexer module only
cargo test parser                       # Test parser module only
cargo test mcp --features mcp           # Test MCP functionality
cargo test -- --nocapture              # Show print output during tests
cargo test test_name                   # Run specific test

# Benchmarking
cargo bench                             # Run all benchmarks
cargo bench lexer                       # Run lexer benchmarks only
cargo bench type_system_benchmark       # Run type system optimization benchmarks

# Documentation
cargo doc --open                        # Generate and open docs
cargo doc --features mcp --open         # Include MCP documentation

# Development Tools
python tools/fix_rust_format.py analyze # Analyze Rust format issues
python tools/fix_rust_format.py fix     # Fix all format issues
python tools/fix_rust_format.py fix -f path/to/file.rs  # Fix specific file
```

## Code Quality and Best Practices

### Code Maintenance
- Always remove unused imports