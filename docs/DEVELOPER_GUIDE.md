# Script Language Developer Guide

This guide is for developers who want to contribute to the Script language compiler, runtime, or tooling. It covers the project architecture, development workflow, and guidelines for contributing.

## Table of Contents

1. [Project Overview](#project-overview)
2. [Development Setup](#development-setup)
3. [Architecture Overview](#architecture-overview)
4. [Compiler Pipeline](#compiler-pipeline)
5. [Code Organization](#code-organization)
6. [Testing](#testing)
7. [Debugging the Compiler](#debugging-the-compiler)
8. [Adding New Features](#adding-new-features)
9. [Performance Considerations](#performance-considerations)
10. [Contributing Guidelines](#contributing-guidelines)

## Project Overview

Script is a modern programming language implemented in Rust, designed to be:
- Simple for beginners yet powerful for production use
- Expression-oriented with gradual typing
- Memory-safe with automatic reference counting
- Compiled to native code via Cranelift/LLVM

### Key Components

1. **Lexer**: Tokenizes Script source code
2. **Parser**: Builds an Abstract Syntax Tree (AST)
3. **Type System**: Hindley-Milner inference with gradual typing
4. **Semantic Analyzer**: Symbol resolution and validation
5. **IR Generator**: Lowers AST to intermediate representation
6. **Code Generator**: Produces machine code via Cranelift
7. **Runtime**: Memory management, panic handling, standard library
8. **Tooling**: LSP server, package manager, documentation generator

## Development Setup

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Git
- C compiler (for linking)
- Optional: LLVM 15+ (for LLVM backend)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/moikapy/script
cd script

# Build in debug mode
cargo build

# Build in release mode
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench

# Generate documentation
cargo doc --open
```

### Development Tools

```bash
# Install development tools
cargo install cargo-watch cargo-flamegraph cargo-criterion

# Watch for changes and rebuild
cargo watch -x build -x test

# Profile with flamegraph
cargo flamegraph --bin script -- examples/complex.script

# Format code
cargo fmt

# Lint code
cargo clippy -- -D warnings
```

## Architecture Overview

### High-Level Architecture

```
Source Code (.script)
    ↓
[Lexer] → Tokens
    ↓
[Parser] → AST
    ↓
[Type Inference] → Typed AST
    ↓
[Semantic Analysis] → Validated AST
    ↓
[IR Lowering] → Script IR
    ↓
[Optimization] → Optimized IR
    ↓
[Code Generation] → Machine Code
    ↓
[Runtime] → Execution
```

### Module Structure

```
src/
├── lexer/          # Tokenization
│   ├── mod.rs      # Token types and exports
│   ├── scanner.rs  # Scanner implementation
│   └── token.rs    # Token definitions
├── parser/         # Parsing
│   ├── mod.rs      # AST definitions
│   ├── parser.rs   # Parser implementation
│   └── ast.rs      # AST node types
├── types/          # Type system
│   ├── mod.rs      # Type definitions
│   └── conversion.rs # Type conversions
├── inference/      # Type inference
│   ├── mod.rs      # Inference engine
│   ├── constraint.rs # Constraint generation
│   └── unification.rs # Unification algorithm
├── semantic/       # Semantic analysis
│   ├── mod.rs      # Analyzer exports
│   ├── analyzer.rs # Analysis passes
│   └── symbol_table.rs # Symbol management
├── ir/             # Intermediate representation
│   ├── mod.rs      # IR types
│   ├── builder.rs  # IR construction
│   └── optimizer/  # Optimization passes
├── codegen/        # Code generation
│   ├── mod.rs      # Codegen interface
│   └── cranelift/  # Cranelift backend
├── runtime/        # Runtime system
│   ├── mod.rs      # Runtime core
│   ├── gc.rs       # Memory management
│   └── panic.rs    # Error handling
├── stdlib/         # Standard library
│   ├── mod.rs      # Stdlib exports
│   ├── io.rs       # I/O operations
│   └── collections.rs # Data structures
└── main.rs         # CLI entry point
```

## Compiler Pipeline

### 1. Lexical Analysis (Lexer)

The lexer converts source text into tokens:

```rust
// In src/lexer/scanner.rs
impl Scanner {
    pub fn scan_token(&mut self) -> Result<Token, LexError> {
        self.skip_whitespace();
        self.start = self.current;
        
        match self.advance() {
            '+' => Ok(self.make_token(TokenKind::Plus)),
            '-' => Ok(self.make_token(TokenKind::Minus)),
            // ... more token patterns
        }
    }
}
```

Key files:
- `src/lexer/token.rs`: Token definitions
- `src/lexer/scanner.rs`: Scanner implementation
- `src/lexer/tests.rs`: Lexer tests

### 2. Parsing (Parser)

The parser builds an AST using recursive descent with Pratt parsing:

```rust
// In src/parser/parser.rs
impl Parser {
    fn parse_expression(&mut self, min_bp: u8) -> Result<Expr, ParseError> {
        let mut left = self.parse_primary()?;
        
        while let Some((left_bp, right_bp)) = self.current_binding_power() {
            if left_bp < min_bp {
                break;
            }
            
            left = self.parse_binary(left, right_bp)?;
        }
        
        Ok(left)
    }
}
```

Key concepts:
- Recursive descent for statements
- Pratt parsing for expressions
- Error recovery for better diagnostics

### 3. Type Inference

Hindley-Milner type inference with extensions:

```rust
// In src/inference/inference_engine.rs
impl InferenceEngine {
    pub fn infer(&mut self, expr: &Expr) -> Result<Type, TypeError> {
        match expr {
            Expr::Literal(lit) => self.infer_literal(lit),
            Expr::Variable(name) => self.lookup_type(name),
            Expr::Binary { left, op, right } => {
                let left_ty = self.infer(left)?;
                let right_ty = self.infer(right)?;
                self.infer_binary_op(op, left_ty, right_ty)
            }
            // ... more expression types
        }
    }
}
```

Key algorithms:
- Constraint generation
- Unification with occurs check
- Type variable substitution

### 4. Semantic Analysis

Validates the program and builds symbol tables:

```rust
// In src/semantic/analyzer.rs
impl SemanticAnalyzer {
    pub fn analyze(&mut self, program: &mut Program) -> Result<(), SemanticError> {
        for item in &mut program.items {
            match item {
                Item::Function(func) => self.analyze_function(func)?,
                Item::Const(const_def) => self.analyze_const(const_def)?,
                // ... more item types
            }
        }
        Ok(())
    }
}
```

Validation includes:
- Variable resolution
- Type checking
- Pattern exhaustiveness
- Const evaluation

### 5. IR Generation

Lowers the typed AST to SSA-based IR:

```rust
// In src/lowering/mod.rs
impl Lowerer {
    pub fn lower_expr(&mut self, expr: &TypedExpr) -> ValueId {
        match &expr.kind {
            ExprKind::Binary { left, op, right } => {
                let left_val = self.lower_expr(left);
                let right_val = self.lower_expr(right);
                self.builder.build_binary(*op, left_val, right_val)
            }
            // ... more expression types
        }
    }
}
```

IR features:
- Static Single Assignment (SSA)
- Control flow graphs
- Type preservation

### 6. Code Generation

Generates machine code using Cranelift:

```rust
// In src/codegen/cranelift/translator.rs
impl Translator {
    pub fn translate_function(&mut self, ir_func: &IrFunction) -> FuncId {
        let mut func = Function::new();
        
        // Translate parameters
        for param in &ir_func.params {
            let ty = self.translate_type(&param.ty);
            func.signature.params.push(AbiParam::new(ty));
        }
        
        // Translate body
        self.translate_blocks(&ir_func.blocks, &mut func);
        
        self.module.define_function(func)
    }
}
```

## Code Organization

### Adding a New Module

1. Create the module directory:
```bash
mkdir src/new_feature
touch src/new_feature/mod.rs
```

2. Define the module interface:
```rust
// src/new_feature/mod.rs
pub mod implementation;
pub mod types;

pub use implementation::NewFeature;
pub use types::{FeatureConfig, FeatureResult};
```

3. Add to lib.rs:
```rust
// src/lib.rs
pub mod new_feature;
```

### Code Style Guidelines

- Use `snake_case` for functions and variables
- Use `PascalCase` for types and traits
- Use `SCREAMING_SNAKE_CASE` for constants
- Document all public APIs
- Write tests for all new functionality
- Keep functions focused and small
- Prefer composition over inheritance

### Error Handling

Use custom error types with good messages:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum CompileError {
    LexError(LexError),
    ParseError(ParseError),
    TypeError(TypeError),
    SemanticError(SemanticError),
}

impl Display for CompileError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::LexError(e) => write!(f, "Lexical error: {}", e),
            // ... more error types
        }
    }
}
```

## Testing

### Unit Tests

Write unit tests alongside implementation:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_binary_expr() {
        let input = "1 + 2 * 3";
        let tokens = lex(input).unwrap();
        let expr = parse_expression(&tokens).unwrap();
        
        assert_matches!(expr, Expr::Binary { .. });
    }
}
```

### Integration Tests

Add integration tests in `tests/`:

```rust
// tests/compilation_tests.rs
#[test]
fn test_compile_hello_world() {
    let source = r#"
        fn main() {
            println("Hello, World!")
        }
    "#;
    
    let result = compile_string(source);
    assert!(result.is_ok());
}
```

### Test Organization

- Unit tests: Next to implementation
- Integration tests: In `tests/` directory
- Example programs: In `examples/` directory
- Benchmarks: In `benches/` directory

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_parse_binary_expr

# Run tests with output
cargo test -- --nocapture

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test '*'
```

## Debugging the Compiler

### Debug Builds

Enable debug output:

```rust
// Set SCRIPT_DEBUG=1 environment variable
if std::env::var("SCRIPT_DEBUG").is_ok() {
    eprintln!("Debug: Parsing expression: {:?}", expr);
}
```

### Compiler Flags

Add debug flags to the CLI:

```rust
#[derive(Parser)]
struct Cli {
    #[arg(long)]
    debug_lexer: bool,
    
    #[arg(long)]
    debug_parser: bool,
    
    #[arg(long)]
    debug_ir: bool,
}
```

### Using the Debugger

```bash
# Debug the compiler itself
rust-gdb target/debug/script

# Set breakpoints
(gdb) break script::parser::Parser::parse_expression
(gdb) run examples/test.script

# Debug a specific phase
SCRIPT_DEBUG_PARSER=1 cargo run -- examples/test.script
```

### Tracing

Use the `tracing` crate for structured logging:

```rust
use tracing::{debug, info, warn, error};

#[instrument]
fn parse_function(&mut self) -> Result<Function, ParseError> {
    debug!("Parsing function");
    // ... implementation
}
```

## Adding New Features

### 1. Language Feature Checklist

When adding a new language feature:

- [ ] Update lexer if new tokens needed
- [ ] Update parser for new syntax
- [ ] Add AST nodes if needed
- [ ] Update type system if needed
- [ ] Add semantic analysis
- [ ] Implement IR lowering
- [ ] Add codegen support
- [ ] Update standard library if needed
- [ ] Write comprehensive tests
- [ ] Update documentation
- [ ] Add examples

### 2. Example: Adding a New Operator

Let's add a `**` (power) operator:

#### Step 1: Update Lexer
```rust
// src/lexer/token.rs
pub enum TokenKind {
    // ... existing tokens
    StarStar,  // **
}

// src/lexer/scanner.rs
'*' => {
    if self.peek() == '*' {
        self.advance();
        Ok(self.make_token(TokenKind::StarStar))
    } else {
        Ok(self.make_token(TokenKind::Star))
    }
}
```

#### Step 2: Update Parser
```rust
// src/parser/parser.rs
fn binding_power(op: &TokenKind) -> Option<(u8, u8)> {
    match op {
        TokenKind::StarStar => Some((14, 15)), // Right associative
        // ... other operators
    }
}
```

#### Step 3: Update Type Inference
```rust
// src/inference/inference_engine.rs
BinaryOp::Power => {
    self.unify(left_ty, Type::F32)?;
    self.unify(right_ty, Type::F32)?;
    Ok(Type::F32)
}
```

#### Step 4: Update IR Generation
```rust
// src/ir/instruction.rs
pub enum BinaryOp {
    // ... existing ops
    Power,
}

// src/lowering/expr.rs
BinaryOp::Power => self.builder.build_call("pow", vec![left, right]),
```

#### Step 5: Add Runtime Support
```rust
// src/stdlib/math.rs
#[runtime_function]
pub fn pow(base: f32, exp: f32) -> f32 {
    base.powf(exp)
}
```

### 3. Feature Flags

Use feature flags for experimental features:

```toml
# Cargo.toml
[features]
experimental = []
pattern-matching = []
async-await = []
```

```rust
#[cfg(feature = "pattern-matching")]
mod pattern_matching;
```

## Performance Considerations

### Benchmarking

Use Criterion for benchmarks:

```rust
// benches/parser_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_parser(c: &mut Criterion) {
    let source = include_str!("../examples/complex.script");
    
    c.bench_function("parse complex program", |b| {
        b.iter(|| {
            let tokens = lex(black_box(source)).unwrap();
            parse(black_box(&tokens)).unwrap()
        })
    });
}

criterion_group!(benches, benchmark_parser);
criterion_main!(benches);
```

### Profiling

```bash
# CPU profiling
cargo flamegraph --bin script -- examples/large.script

# Memory profiling
valgrind --tool=massif target/release/script examples/large.script
ms_print massif.out.<pid>

# Cache profiling
perf stat -e cache-misses,cache-references cargo run --release
```

### Optimization Guidelines

1. **Avoid Allocations**: Use references where possible
2. **Use Iterators**: Prefer iterators over collecting
3. **Inline Hot Functions**: Use `#[inline]` judiciously
4. **Minimize Cloning**: Use `Cow` or `Arc` for shared data
5. **Profile First**: Don't guess, measure

### Memory Management

- Use arena allocation for AST nodes
- Pool common objects (tokens, small strings)
- Implement custom allocators for hot paths
- Use `SmallVec` for small collections

## Contributing Guidelines

### Getting Started

1. Fork the repository
2. Create a feature branch: `git checkout -b feature-name`
3. Make your changes
4. Add tests
5. Run tests: `cargo test`
6. Format code: `cargo fmt`
7. Check lints: `cargo clippy`
8. Commit with clear message
9. Push and create PR

### Pull Request Process

1. **Title**: Clear, concise description
2. **Description**: What, why, and how
3. **Tests**: All new code must have tests
4. **Documentation**: Update relevant docs
5. **Benchmarks**: Include if performance-critical

### Code Review

Reviewers will check:
- Code quality and style
- Test coverage
- Performance impact
- Documentation updates
- Breaking changes

### Commit Messages

Follow conventional commits:

```
feat: add power operator support
fix: resolve parser panic on invalid syntax
docs: update contributor guide
test: add integration tests for modules
perf: optimize lexer for large files
refactor: simplify type inference engine
```

### Release Process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Run full test suite
4. Create release PR
5. Tag release after merge
6. Publish to crates.io

## Resources

### Internal Documentation

- [Architecture Overview](./architecture/OVERVIEW.md)
- [Memory Management](./architecture/MEMORY.md)
- [Module System](./architecture/MODULES.md)
- [Compilation Pipeline](./architecture/PIPELINE.md)

### External Resources

- [Crafting Interpreters](https://craftinginterpreters.com/)
- [Engineering a Compiler](https://www.elsevier.com/books/engineering-a-compiler/cooper/978-0-12-815412-0)
- [Types and Programming Languages](https://www.cis.upenn.edu/~bcpierce/tapl/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

### Getting Help

- GitHub Issues: Bug reports and feature requests
- Discussions: General questions and ideas
- Discord: Real-time chat with contributors
- Email: security@script.org (security issues only)

---

Thank you for contributing to Script! Your efforts help make programming more accessible and enjoyable for everyone.