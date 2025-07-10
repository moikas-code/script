# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with the Script programming language repository.

- Format each individual memory as a bullet point and group related memories under descriptive markdown headings.

- Perform a web search if you need more information

- Create a Task in the KB when making a plan to keep track between sessions

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
```

## Architecture Overview

### Implementation Status (v0.5.0-alpha)
**Overall Completion**: ~90% - See [kb/status/OVERALL_STATUS.md](kb/status/OVERALL_STATUS.md) for detailed tracking

| Phase | Status | Completion | Notes |
|-------|--------|------------|-------|
| Lexer | ✅ | 100% | Unicode support, error reporting, REPL |
| Parser | ✅ | 99% | Generics, enum patterns, exhaustiveness, closures |
| Type System | ✅ | 98% | **O(n²) → O(n log n) optimizations complete**, union-find unification |
| Semantic | ✅ | 99% | Pattern safety, exhaustiveness, symbol resolution |
| CodeGen | 🔧 | 90% | Generic compilation, closures, patterns (+5% from functional) |
| Runtime | 🔧 | 75% | Bacon-Rajan cycle detection, closure support (+15%) |
| Stdlib | ✅ | 100% | **COMPLETE**: Collections, I/O, math, functional programming |
| Module System | ✅ | 100% | **COMPLETE**: Multi-file projects fully supported |
| MCP | 🔄 | 15% | Security framework designed, implementation starting |

### Key Architecture Components

**Core Modules:**
- `src/lexer/` - Tokenization with Unicode support ✅
- `src/parser/` - Recursive descent + Pratt parsing ✅  
- `src/semantic/` - Type checking, pattern exhaustiveness ✅
- `src/codegen/` - Cranelift IR generation 🔧
- `src/runtime/` - Memory management, cycle detection 🔧
- `src/mcp/` - AI integration security framework 🔄

### Core Design Principles

1. **Expression-Oriented**: Everything returns a value (if/while/blocks)
2. **Gradual Typing**: Optional type annotations with inference
3. **Memory Safety**: ARC with Bacon-Rajan cycle detection
4. **AI-Native**: Security-first MCP implementation for AI development
5. **Error Recovery**: Multiple error reporting with source context
6. **Performance-First**: O(n log n) type system with union-find unification

## Current Development Focus

### 🚨 Critical Issues (Immediate Priority)
*No critical issues remaining - all major systems complete!*

### ✅ Recently Completed (January 2025)
- **Module System** - Full multi-file project support with import/export
- **Standard Library** - Complete collections (HashMap/Set/Vec), I/O, math, networking
- **Functional Programming** - Closures, higher-order functions, iterators (57 operations)
- **Error Handling** - Result<T,E> and Option<T> with monadic operations
- **Async Security Vulnerabilities** - All use-after-free and memory corruption issues resolved
- **Generic Implementation Security** - Array bounds checking and field access validation
- **Resource Limits** - Comprehensive DoS protection for all compilation phases
- **Type System Complexity** - O(n²) algorithms optimized to O(n log n) with union-find

### 🔧 Active Development
- **Error Messages** - Improving quality and context
- **REPL Enhancements** - Better interactive development experience
- **MCP Security Framework** - AI integration implementation
- **Performance Optimizations** - Further runtime improvements

## Development Workflow

### Quick Start
1. **Check Issues**: Review `kb/KNOWN_ISSUES.md` before starting work
2. **Check Status**: Review `kb/STATUS.md` for current completion status  
3. **Security First**: All external inputs are untrusted, validate everything
4. **Update Docs**: Keep `kb/` documentation current as you work

### Key Development Rules
- **Git Rule**: Never author commits as Claude, always as the user
- **Security Rule**: All MCP tools must use sandboxed analysis
- **Memory Rule**: Runtime safety takes precedence over new features
- **Testing Rule**: Security validation tests are mandatory
- **Resource Rule**: All compilation operations must respect resource limits for DoS protection

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

### MCP Security Guidelines

**Core Principles:**
- All MCP tools use sandboxed analysis (non-negotiable)
- All external inputs require validation (untrusted by default)  
- Resource limits enforced at multiple layers (defense in depth)
- Security validation tests mandatory (no exceptions)

**Testing Pattern:**
```rust
// Validate dangerous input rejection
assert!(mcp_server.validate_input(malicious_code).is_err());
// Test resource limits  
assert!(mcp_server.analyze_code(&"x".repeat(1_000_000)).is_err());
```

## Implementation Roadmap

See `./kb/IMPLEMENTATION_TODO.md` for original planning. Current status:
- **[status](./kb/status)**: Detailed completion tracking for all phases
- **[KNOWN_ISSUES.md](./kb/active/KNOWN_ISSUES.md)**: All known bugs and limitations

### Recent Major Achievements ✅
- **Module System**: Complete multi-file project support with ModuleLoaderIntegration
- **Standard Library**: 100% complete - Vec, HashMap, HashSet, I/O, math, networking
- **Functional Programming**: Full closure system with 57 stdlib operations
- **Error Handling**: Complete Result/Option types with monadic operations
- **Pattern Matching**: Exhaustiveness checking, or-patterns, guard support
- **Generics System**: Full monomorphization pipeline with 43% deduplication efficiency  
- **Memory Management**: Production-grade Bacon-Rajan cycle detection
- **Type System**: Complete type inference with O(n log n) performance
- **Security Infrastructure**: Comprehensive DoS protection and resource limits
- **Async Runtime Security**: All memory corruption vulnerabilities resolved

### Resource Limits & DoS Protection ✅
- **Timeout Protection**: All compilation phases have configurable timeouts
- **Memory Monitoring**: System memory usage tracking and limits  
- **Iteration Limits**: Recursive operations bounded to prevent infinite loops
- **Recursion Depth**: Stack overflow protection through depth tracking
- **Specialization Limits**: Generic instantiation explosion prevention
- **Work Queue Limits**: Bounded compilation resource usage
- **Environment Configs**: Production, development, and testing limit profiles

## File Organization

### Directory Structure
- **Root**: Only config files, documentation, build files  
- **tests/**: All test files and fixtures organized by type
- **examples/**: Demo programs and tutorials
- **kb/**: Knowledge base (see KB Organization below)

### Prohibited Root Files
No `.script` files in root directory. Use:
- `tests/fixtures/valid_programs/` for test cases
- `examples/` for demonstrations  
- `tests/mcp/security/` for MCP validation

## KB (Knowledge Base) Organization

The `kb/` directory follows a specific structure for tracking project state:

### KB Directory Structure
```
kb/
├── active/          # Current work and open issues
├── completed/       # Resolved issues and finished work
├── status/          # Overall project status tracking
├── development/     # Development guides and standards
├── architecture/    # Design decisions and architecture docs
├── guides/          # How-to guides and tutorials
├── references/      # External references and resources
├── planning/        # Future plans and roadmaps
├── reports/         # Analysis and investigation reports
├── notes/           # Miscellaneous notes and observations
└── legacy/          # Deprecated or historical documents
```

### What Goes Where

#### `active/` - Current Work
- Open issues and bugs (e.g., KNOWN_ISSUES.md)
- In-progress implementations
- Current investigation reports
- Active todo lists and tasks
- **Move to `completed/` when resolved**

#### `completed/` - Resolved Work
- Closed issues with resolution details
- Completed feature implementations
- Finished investigation reports
- Resolved security audits
- **Include resolution date and summary**

#### `status/` - Project Status
- Overall implementation status (OVERALL_STATUS.md)
- Component-specific status reports
- Production readiness assessments
- Compliance status tracking
- **Keep continuously updated**

#### `development/` - Development Resources
- Coding standards and guidelines
- Testing standards (e.g., CLOSURE_TESTING_STANDARDS.md)
- Implementation guides
- Best practices documentation
- **Reference documents for developers**

#### Other Directories
- **architecture/**: Design decisions, system architecture
- **guides/**: Step-by-step tutorials, how-to guides
- **references/**: Links to external docs, resources
- **planning/**: Roadmaps, future feature plans
- **reports/**: Investigation results, analysis reports
- **notes/**: Quick notes, observations, ideas
- **legacy/**: Old/deprecated docs kept for reference

## Essential Documentation

### Critical References
- **[kb/status](kb/status)** - Implementation status tracking
- **[kb/active/KNOWN_ISSUES.md](kb/active/KNOWN_ISSUES.md)** - Bug tracker and limitations  
- **[README.md](README.md)** - Project overview and setup

### External References  
- [Rust Documentation](https://doc.rust-lang.org/book/)
- [MCP Specification](https://modelcontextprotocol.io/docs)


### Current Version: v0.5.0-alpha
- **Overall**: ~90% complete with all major systems implemented
- **Recent**: Module system, standard library, and functional programming completed
- **Focus**: Error message quality, REPL enhancements, MCP integration
- **Achievement**: Production-grade core language with only minor issues remaining

## CLI Commands

### Security and Optimization
- `/audit` - Perform a Security (SOC2 Compliant), and Optimization Audit of the provided file or files; Ensure they are ready for Production.
- `/implement` - Implement a production-ready implementation of the provided text; Refer to and update the @kb documentation for tracking and guidance
- `/scan` - Audit the entire codebase, create any issue that have been added to the KB, and report back on the completion status of the project;

### Available MCP Tools
- `kb_read` - Read any KB file (e.g., "active/KNOWN_ISSUES.md")
- `kb_list` - Browse KB directory structure
- `kb_update` - Create/update KB files 
- `kb_delete` - Delete KB files
- `kb_search` - Search KB content
- `kb_status` - Get implementation status overview
- `kb_issues` - Get current known issues

### Usage Examples
- "Show me the current implementation status" → Uses `kb_status`
- "What are the known issues?" → Uses `kb_issues`
- "Update the roadmap with this milestone" → Uses `kb_update`
- "Search for async implementation details" → Uses `kb_search`

---

*Script: The first AI-native programming language. Security-first development transforms AI integration challenges into competitive advantages.*
```