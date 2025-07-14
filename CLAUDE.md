# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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

# Documentation
cargo doc --open                        # Generate and open docs
cargo doc --features mcp --open         # Include MCP documentation
```

## Architecture Overview

### Current Status (v0.3.5-alpha) - Honest Assessment
- **Overall Completion**: ~60% - See [STATUS.md](STATUS.md) for detailed tracking
- **Phase 1 (Lexer)**: ✅ COMPLETED - Full tokenization with Unicode support, error reporting, REPL
- **Phase 2 (Parser)**: 🔧 85% - Basic parsing works, generics now functional
- **Phase 3-8**: ❌ Many critical features non-functional or have major gaps
- **Phase 9 (MCP)**: 🔄 IN DEVELOPMENT - AI integration framework

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
├── mcp/            # Model Context Protocol (Phase 9 - In Development)
│   ├── mod.rs      # MCP module exports and feature gates
│   ├── server/     # MCP server implementation
│   ├── security/   # Security framework and sandboxing
│   ├── tools/      # Script analysis tools for AI
│   └── resources/  # Secure resource access
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

### MCP Architecture

The MCP implementation follows security-first principles:
- **Security Manager**: Input validation, rate limiting, audit logging
- **Secure Sandbox**: Isolated analysis environment with resource limits
- **Tool Registry**: Script analyzer, formatter, documentation generator
- **Resource Management**: Controlled file access with path validation
- **Protocol Compliance**: Full MCP specification support

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
6. **AI Integration**: Security-first MCP implementation for AI-native development

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

### MCP Development Guidelines

When working on MCP functionality, maintain philosophical discipline:

#### Security-First Development
- All MCP tools must use sandboxed analysis - this is non-negotiable
- Input validation is mandatory for all AI interactions - external inputs are untrusted
- Audit logging required for security compliance - transparency builds trust
- Resource limits enforced at multiple layers - defense in depth

#### Architecture Consistency
- Follow existing patterns from LSP server implementation - consistency reduces complexity
- Integrate with existing lexer/parser/semantic components - reuse proven foundations
- Maintain consistency with error handling and reporting - unified experience
- Use existing configuration patterns - familiar patterns reduce friction

#### Testing Requirements
- Security validation tests mandatory - security without testing is faith
- Integration tests with existing components - isolation breeds fragility
- Performance benchmarks for analysis operations - measure to improve
- Protocol compliance verification - standards enable interoperability

### Testing Error Cases
Use the error reporter pattern:
```rust
let mut reporter = ErrorReporter::new();
reporter.report(error.with_file_name("test.script").with_source_line(line));
reporter.print_all();
```

### MCP Security Testing
```rust
// Test dangerous input rejection
let malicious_code = r#"
    import std.fs
    fs.delete_all("/")
"#;
assert!(mcp_server.validate_input(malicious_code).is_err());

// Test resource limits
let large_code = "x".repeat(1_000_000);
assert!(mcp_server.analyze_code(&large_code).is_err());
```

## Implementation Roadmap

See `IMPLEMENTATION_TODO.md` for original planning. Current status:
- **[STATUS.md](STATUS.md)**: Detailed completion tracking for all phases
- **[KNOWN_ISSUES.md](KNOWN_ISSUES.md)**: All known bugs and limitations

Next major milestones for v1.0:
- Complete MCP server implementation with security framework
- Integrate AI-powered development tools
- Fix pattern matching exhaustiveness checking (completed)
- Complete generic parameter parsing (completed)
- Implement cycle detection for memory safety
- Complete async/await implementation

### Git Rules
- Never Author Git Commits or PRs as Claude

## Development Principles

- Always create subagents when planning or implementing tasks
- **Supervisor Role**: You are a Supervisor of a team of subagents; when planning or implementing tasks, you will give each team member a task to complete
- **Security Mindset**: Every external input is untrusted until validated
- **Philosophical Approach**: The obstacle of AI integration becomes the way to language leadership

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
├── mcp/               # MCP-specific tests
│   ├── security/      # Security validation tests
│   ├── tools/         # Tool functionality tests
│   └── protocol/      # Protocol compliance tests
└── modules/           # Module-specific test files
```

**Common Violations and Solutions:**
| ❌ Wrong | ✅ Correct |
|----------|------------|
| `test_simple.script` | `tests/fixtures/valid_programs/simple.script` |
| `debug_test.script` | `examples/debug_example.script` |
| `temp_example.script` | `examples/example.script` (or delete) |
| `mcp_test.script` | `tests/mcp/security/validation_test.script` |

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
- `mcp_*.script` (matches: mcp_example.script)

**Pattern Testing:**
```bash
# Test if a filename would be ignored
git check-ignore -v filename.script

# Examples of comprehensive pattern coverage:
git check-ignore -v debug_types.script    # ✅ Ignored by /*_types.script
git check-ignore -v type_test_all.script  # ✅ Ignored by /*test*.script
git check-ignore -v simple_test.script    # ✅ Ignored by /*_test.script
git check-ignore -v mcp_test.script       # ✅ Ignored by /*test*.script
```

### Quick Reference for Developers
```bash
# WRONG - don't do this
touch test_something.script      # Matches /test_*.script
touch debug_test.script          # Matches /debug_*.script
touch simple_test.script         # Matches /*_test.script
touch mcp_example.script         # Matches /*test*.script

# RIGHT - proper organization
touch tests/fixtures/valid_programs/something.script
touch examples/demonstration.script
touch tests/mcp/security/validation_example.script
```

### Lessons Learned
- **GitIgnore Gap**: Initial patterns missed hybrid naming conventions (`*_test_*.script`)
- **Pre-commit Safety Net**: Location-based hooks catch what name-based patterns miss
- **Pattern Testing**: Always test new ignore patterns with `git check-ignore -v`
- **Comprehensive Coverage**: Use multiple overlapping patterns for robust protection

## Critical Documentation

- **[STATUS.md](STATUS.md)** - Current implementation status (v0.3.5-alpha)
- **[KNOWN_ISSUES.md](KNOWN_ISSUES.md)** - All known bugs and limitations
- **[README.md](README.md)** - Project overview and getting started

## Reference Links
- [Rust Official Documentation](https://doc.rust-lang.org/book/title-page.html)
- [MCP Specification](https://modelcontextprotocol.io/docs)

## Development Workflow Checklist

### Before Starting Any Task:
1. Check [KNOWN_ISSUES.md](KNOWN_ISSUES.md) to see if the issue is already documented
2. Review [STATUS.md](STATUS.md) for current completion status
3. Update both files as you work on features

### When Implementing Features:
- If you discover a bug, add it to KNOWN_ISSUES.md
- When you complete a feature, update STATUS.md percentages
- Document workarounds in KNOWN_ISSUES.md if applicable

### When Working on MCP:
- Security considerations must be documented first
- All external inputs require validation
- Resource limits must be enforced
- Audit logging is mandatory

### Version Information:
- Current version: **0.3.5-alpha** (including MCP development)
- ~60% overall completion
- Critical features like memory safety and async are missing
- MCP integration provides strategic differentiation opportunity

---

*"The impediment to action advances action. What stands in the way becomes the way."* - Marcus Aurelius

MCP integration is not merely a feature addition; it is the path to establishing Script as the first AI-native programming language. Through measured implementation and unwavering focus on security, we transform the challenge of AI integration into Script's greatest competitive advantage.
```

## Memory Entries

### Project Workflow
- Always Store Docs created to help the AI build the project in the `/kb` dir, and update and delete them when needed
