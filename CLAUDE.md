# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Development Commands

```bash
# Build
cargo build              # Debug build
cargo build --release    # Release build

# Run REPL
cargo run               # Interactive REPL (parse mode by default)
cargo run -- --tokens   # Run REPL in token mode

# Parse Script files
cargo run examples/hello.script           # Parse and display AST
cargo run examples/hello.script --tokens  # Tokenize only

# Testing
cargo test                              # Run all tests
cargo test lexer                        # Test lexer module only
cargo test parser                       # Test parser module only
cargo test -- --nocapture              # Show print output during tests
cargo test test_name                   # Run specific test

# Benchmarking
cargo bench                             # Run all benchmarks
cargo bench lexer                       # Run lexer benchmarks only

# Documentation
cargo doc --open                        # Generate and open docs
```

## Architecture Overview

### Current Status
- **Phase 1 (Lexer)**: ✅ COMPLETED - Full tokenization with Unicode support, error reporting, REPL
- **Phase 2 (Parser)**: 🚧 IN PROGRESS - AST definitions complete, parser implementation underway

### Module Structure

```
src/
├── lexer/          # Tokenization (Phase 1 - Complete)
│   ├── mod.rs      # Token types and lexer module exports
│   ├── scanner.rs  # Scanner implementation with Unicode support
│   └── tests.rs    # Comprehensive lexer tests
├── parser/         # AST & Parsing (Phase 2 - In Progress)
│   ├── mod.rs      # AST node definitions and parser exports
│   ├── parser.rs   # Recursive descent parser with Pratt parsing
│   └── tests.rs    # Parser tests
├── error/          # Error infrastructure
│   └── mod.rs      # ScriptError type with source locations
├── source/         # Source tracking
│   └── mod.rs      # SourceLocation and Span types
└── main.rs         # CLI entry point with REPL modes
```

### Parser Architecture

The parser uses **recursive descent** with **Pratt parsing** for expressions:
- `parse_expression()` handles operator precedence via binding power
- `parse_primary()` handles literals, identifiers, and grouped expressions
- Statements include let bindings, functions, returns, while/for loops
- Everything is expression-oriented (if/while/blocks return values)

### Error Handling

Script uses a custom `ScriptError` type that:
- Tracks source location (line, column, file)
- Provides contextual error messages with source line display
- Supports error recovery in the lexer (continues after errors)
- Reports multiple errors before failing

### AST Design

Key expression types:
```rust
Expr::Literal(value)                    // Numbers, strings, booleans
Expr::Binary { left, op, right }        // Arithmetic, comparison, logical
Expr::Call { callee, args }             // Function calls
Expr::If { condition, then, else_ }     // If expressions (not statements!)
Expr::Block(statements)                 // Block expressions return last value
```

### REPL Modes

The REPL supports two modes:
1. **Token mode** (`--tokens`): Shows tokenization output with spans
2. **Parse mode** (default): Parses input and displays AST

## Key Design Decisions

1. **Expression-Oriented**: Everything returns a value (if, while, blocks)
2. **Gradual Typing**: Type annotations are optional; inference fills gaps
3. **Memory Strategy**: Will use ARC with cycle detection (not yet implemented)
4. **Compilation Targets**: Planning Cranelift (dev) and LLVM (prod) backends
5. **Error Philosophy**: Show multiple errors, provide helpful context

## Working with the Codebase

### Adding New Token Types
1. Add variant to `TokenKind` enum in `src/lexer/mod.rs`
2. Update `Scanner::scan_token()` in `src/lexer/scanner.rs`
3. Add keyword to `Scanner::get_keyword()` if applicable
4. Add tests in `src/lexer/tests.rs`

### Adding New AST Nodes
1. Add variant to `Expr` or `Stmt` enum in `src/parser/mod.rs`
2. Implement parsing logic in `src/parser/parser.rs`
3. Update `Display` implementations for pretty-printing
4. Add parser tests in `src/parser/tests.rs`

### Testing Error Cases
Use the error reporter pattern:
```rust
let mut reporter = ErrorReporter::new();
reporter.report(error.with_file_name("test.script").with_source_line(line));
reporter.print_all();
```

## Implementation Roadmap

See `IMPLEMENTATION_TODO.md` for detailed phase planning. Next major milestones:
- Complete Phase 2: Parser implementation and testing
- Phase 3: Type system with inference
- Phase 4: Code generation (Cranelift/LLVM)
- Phase 5: Runtime and standard library


### Git Rules
- Never Author as Claude