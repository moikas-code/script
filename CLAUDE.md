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

### Current Status (v0.3.0-alpha) - Honest Assessment
- **Overall Completion**: ~45% - See [STATUS.md](STATUS.md) for detailed tracking
- **Phase 1 (Lexer)**: ✅ COMPLETED - Full tokenization with Unicode support, error reporting, REPL
- **Phase 2 (Parser)**: 🔧 75% - Basic parsing works, generics completely broken
- **Phase 3-8**: ❌ Many critical features non-functional or have major gaps

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

See `IMPLEMENTATION_TODO.md` for original planning. Current status:
- **[STATUS.md](STATUS.md)**: Detailed completion tracking for all phases
- **[KNOWN_ISSUES.md](KNOWN_ISSUES.md)**: All known bugs and limitations

Next major milestones for v1.0:
- Fix pattern matching exhaustiveness checking
- Complete generic parameter parsing
- Implement cycle detection for memory safety
- Complete async/await implementation


### Git Rules
- Never Author Git Commits or PRs as Claude

## Development Principles

- Always create subagents when planning or implementing tasks
- **Supervisor Role**: You are a Supervisor of a team of subagents; when planning or implementing tasks, you will give each team member a task to complete

## File Organization Rules

### Project Structure Requirements

**Root Directory Policy:**
- **FORBIDDEN**: No test files, temporary files, or .script files in root
- **ALLOWED**: Only configuration files, documentation, and build files

**Test File Organization:**
```
tests/
├── fixtures/          # Test data and example programs
│   ├── legacy_tests/   # Moved legacy test files
│   ├── valid_programs/ # Working .script examples
│   └── error_cases/    # Programs that should fail
├── integration/        # Cross-module integration tests
└── modules/           # Module-specific test files
```

**Common Violations and Solutions:**
| ❌ Wrong | ✅ Correct |
|----------|------------|
| `test_simple.script` | `tests/fixtures/valid_programs/simple.script` |
| `debug_test.script` | `examples/debug_example.script` |
| `temp_example.script` | `examples/example.script` (or delete) |

**Enforcement:**
- Pre-commit hooks automatically reject root test files
- .gitignore prevents accidental commits of temporary files
- CI checks enforce file organization compliance

### File Naming Conventions

**Prohibited Root Directory Patterns:**
- `test_*.script` (matches: test_simple.script)
- `debug_*.script` (matches: debug_types.script)  
- `*_test.script` (matches: simple_test.script)
- `*_test_*.script` (matches: type_test_all.script)
- `*_types.script` (matches: debug_types.script)
- `type_*.script` (matches: type_test_all.script)
- `*test*.script` (matches: anytest.script)
- `all_*.script` (matches: all_test.script)

**Pattern Testing:**
```bash
# Test if a filename would be ignored
git check-ignore -v filename.script

# Examples of comprehensive pattern coverage:
git check-ignore -v debug_types.script    # ✅ Ignored by /*_types.script
git check-ignore -v type_test_all.script  # ✅ Ignored by /*test*.script
git check-ignore -v simple_test.script    # ✅ Ignored by /*test*.script
```

### Quick Reference for Developers
```bash
# WRONG - don't do this
touch test_something.script      # Matches /test_*.script
touch debug_test.script          # Matches /debug_*.script
touch simple_test.script         # Matches /*_test.script
touch type_test_all.script       # Matches /*test*.script

# RIGHT - proper organization
touch tests/fixtures/valid_programs/something.script
touch examples/demonstration.script
```

### Lessons Learned
- **GitIgnore Gap**: Initial patterns missed hybrid naming conventions (`*_test_*.script`)
- **Pre-commit Safety Net**: Location-based hooks catch what name-based patterns miss
- **Pattern Testing**: Always test new ignore patterns with `git check-ignore -v`
- **Comprehensive Coverage**: Use multiple overlapping patterns for robust protection

## Critical Documentation

- **[STATUS.md](STATUS.md)** - Current implementation status (v0.9.0-beta)
- **[KNOWN_ISSUES.md](KNOWN_ISSUES.md)** - All known bugs and limitations
- **[README.md](README.md)** - Project overview and getting started

## Reference Links
- [Rust Official Documentation](https://doc.rust-lang.org/book/title-page.html)

## Development Workflow Checklist

### Before Starting Any Task:
1. Check [KNOWN_ISSUES.md](KNOWN_ISSUES.md) to see if the issue is already documented
2. Review [STATUS.md](STATUS.md) for current completion status
3. Update both files as you work on features

### When Implementing Features:
- If you discover a bug, add it to KNOWN_ISSUES.md
- When you complete a feature, update STATUS.md percentages
- Document workarounds in KNOWN_ISSUES.md if applicable

### Version Information:
- Current version: **0.9.0-beta** (not 1.0!)
- ~75% overall completion
- Critical features like generics, pattern safety, and async are missing