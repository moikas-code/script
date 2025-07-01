# Script Programming Language 📜

Script is a new programming language that embodies the principle of accessible power - simple enough for beginners to grasp intuitively, yet performant enough to build production web applications and games.

## Philosophy

Like a well-written script guides actors through a performance, Script guides programmers from their first "Hello World" to complex applications with clarity and purpose.

- **Simple by default, powerful when needed**
- **Expression-oriented syntax**
- **Gradual typing with type inference**
- **Memory safe with automatic reference counting**

## Current Status

Phase 1: Lexer Implementation ✅
- Token scanning with full Unicode support
- Comprehensive error reporting with source locations
- Interactive REPL
- File tokenization for .script files

Phase 2: Parser & AST ✅
- Complete AST node definitions
- Recursive descent parser with Pratt parsing
- Expression and statement parsing
- Type annotation support
- REPL with parse/token modes

## Quick Start

```bash
# Build the project
cargo build --release

# Run the REPL
cargo run

# Parse a Script file (default)
cargo run examples/hello.script

# Show tokens only
cargo run examples/hello.script --tokens

# Run tests
cargo test

# Run benchmarks
cargo bench
```

## Example Script Syntax

```script
// Hello World in Script
fn main() {
    print("Hello from Script! 📜")
}

// Variables with optional types
let language = "Script"
let version: f32 = 0.1

// Everything is an expression
let result = if version > 1.0 {
    "stable"
} else {
    "experimental"
}

// Function example
fn add(a: i32, b: i32) -> i32 {
    return a + b
}
```

## Project Structure

```
script-lang/
├── src/
│   ├── lexer/       # Tokenization
│   ├── parser/      # AST construction (coming soon)
│   ├── analyzer/    # Type checking & inference (coming soon)
│   ├── error/       # Error handling infrastructure
│   └── source/      # Source location tracking
├── examples/        # Example Script programs
├── benches/         # Performance benchmarks
└── tests/           # Integration tests
```

## Roadmap

- [x] **Phase 1**: Lexer Implementation
- [x] **Phase 2**: Parser & AST
- [ ] **Phase 3**: Type System & Inference
- [ ] **Phase 4**: Code Generation (Cranelift/LLVM)
- [ ] **Phase 5**: Standard Library
- [ ] **Phase 6**: Tooling (LSP, Package Manager)

## Contributing

Script is in its early stages. Contributions, ideas, and feedback are welcome!

## License

MIT License

---

Created by Warren Gates (moikapy)