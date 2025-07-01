# Script Programming Language - Initial Project Prompt for Claude Code

## Project Overview

I seek to create Script, a new programming language that embodies the principle of accessible power - simple enough for beginners to grasp intuitively, yet performant enough to build production web applications and games. This is not merely a technical exercise, but a philosophical pursuit: can we reconcile ease of learning with computational efficiency?

Like a well-written script guides actors through a performance, Script will guide programmers from their first "Hello World" to complex applications with clarity and purpose.

## Core Philosophy & Requirements

**Language Vision:**
- **Name**: Script - guiding programmers through their coding journey
- **Primary Purpose**: A compiled language for web applications and game development
- **Design Philosophy**: "Simple by default, powerful when needed"
- **Key Principle**: Every complexity must justify its existence through clear user benefit

**Technical Requirements:**
1. **Syntax**: JavaScript/GDScript-inspired for familiarity
2. **Type System**: Gradual typing with Hindley-Milner inference
3. **Memory Management**: Automatic Reference Counting with cycle detection
4. **Compilation**: Dual backend - Cranelift for development, LLVM for production
5. **Targets**: Native executables and WebAssembly
6. **File Extension**: .script

**Design Decisions Already Made:**
- Expression-oriented syntax (everything returns a value)
- Hand-written recursive descent parser
- Arena allocation for AST nodes
- Rust as implementation language

## Implementation Todo List

### Phase 1: Foundation (Weeks 1-2)
- [ ] **Project Setup**
  - Initialize Rust workspace with "script-lang" as the project name
  - Set up testing framework and CI pipeline
  - Create basic error reporting infrastructure
  - Design Script logo and branding elements

- [ ] **Lexer Implementation**
  - Token types for numbers, identifiers, operators, keywords
  - Source location tracking for each token
  - Basic error recovery mechanisms
  - File handling for .script source files

- [ ] **Expression Parser**
  - Arithmetic expressions with proper precedence
  - Variable declarations and references
  - Basic type annotations (optional)

- [ ] **Tree-Walking Evaluator**
  - Evaluate arithmetic expressions
  - Variable storage and lookup
  - Type checking for annotated expressions

### Phase 2: Control Flow & REPL (Weeks 3-4)
- [ ] **Control Structures**
  - If expressions (not statements)
  - While/for loops as expressions
  - Pattern matching basics

- [ ] **Script REPL Development**
  - Interactive evaluation loop
  - History and tab completion
  - Pretty-printing of values
  - "script>" prompt design

- [ ] **Error Handling**
  - Result type for recoverable errors
  - Panic mechanism for unrecoverable errors
  - Clear, helpful error messages with Script branding

### Phase 3: Functions & Type System (Weeks 5-6)
- [ ] **Function Implementation**
  - Function declarations and calls
  - Closures with captured variables
  - Higher-order functions

- [ ] **Type Inference**
  - Local type inference for variables
  - Function parameter/return type inference
  - Gradual typing integration

### Phase 4: Compilation Pipeline (Month 2)
- [ ] **Script IR Design**
  - Define intermediate representation
  - AST to IR lowering
  - Basic optimizations (constant folding)

- [ ] **Cranelift Backend**
  - Code generation for basic operations
  - Function compilation
  - Native executable output

- [ ] **WebAssembly Target**
  - WASM code generation
  - JavaScript interop layer
  - Browser testing setup

### Phase 5: Advanced Features (Months 3-4)
- [ ] **Data Structures**
  - Arrays/vectors with bounds checking
  - Hash maps/dictionaries
  - User-defined structures

- [ ] **Script Standard Library**
  - I/O operations
  - String manipulation
  - Basic collections
  - Game-oriented math utilities

- [ ] **Memory Management**
  - Implement ARC system
  - Cycle detection algorithm
  - Memory profiling tools

### Phase 6: Tooling & Polish (Month 5+)
- [ ] **Script Language Server Protocol**
  - Syntax highlighting
  - Auto-completion
  - Go-to definition
  - VS Code extension

- [ ] **Documentation**
  - Script language specification
  - "Learn Script in Y Minutes"
  - Tutorial: "Your First Game in Script"
  - API documentation

- [ ] **Performance Optimization**
  - LLVM backend integration
  - Optimization passes
  - Benchmarking suite

- [ ] **Package Manager (manuscript)**
  - Basic dependency management
  - Local package support
  - Package manifest format

## Future Considerations (Post-MVP)
- [ ] **ML Interop Foundation**
  - FFI design for Python/C libraries
  - Basic tensor type prototype
  - GPU memory management research

- [ ] **Community Building**
  - Script playground (online REPL)
  - Example games and web apps
  - Discord/Forum setup

## Initial Code Structure

```
script-lang/
├── src/
│   ├── lexer/       # Tokenization
│   ├── parser/      # AST construction
│   ├── analyzer/    # Type checking & inference
│   ├── ir/          # Intermediate representation
│   ├── codegen/     # Code generation backends
│   ├── runtime/     # Runtime library
│   └── repl/        # Interactive shell
├── std/             # Standard library
├── tests/           # Test suite
├── examples/        # Example programs
│   ├── hello.script
│   ├── game.script
│   └── webapp.script
└── docs/            # Documentation

## Example Script Syntax (proposed)

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

// Game-oriented example
fn update_player(player: Player, dt: f32) -> Player {
    Player {
        x: player.x + player.vx * dt,
        y: player.y + player.vy * dt,
        ..player  // spread syntax
    }
}
```

## Guiding Principles for Implementation

1. **Incremental Progress**: Each phase builds upon the previous, maintaining a working system at all times
2. **Test-Driven Development**: Every feature accompanied by comprehensive tests
3. **User-Centric Design**: Regular evaluation against the "beginner-friendly" criterion
4. **Performance Awareness**: Profile early, optimize deliberately
5. **Code Clarity**: The implementation itself should embody Script's philosophy of simplicity
6. **Community First**: Design decisions should consider the eventual Script community

## First Session Goals

Begin with the lexer - the foundation upon which all else rests. Focus on:
1. Setting up the Rust project structure for "script-lang"
2. Implementing basic token types
3. Creating a simple scanner that can tokenize arithmetic expressions
4. Writing tests for edge cases
5. Creating a simple main.rs that can tokenize a .script file

Remember: The journey of creating Script begins with a single token. Each small step forward is progress toward the larger vision. Like a carefully written script guides a performance, our language will guide programmers through their coding journey with clarity and purpose.

## Project Metadata

- **Creator**: Warren Gates (moikapy)
- **Language Name**: Script
- **Target Audience**: Beginners learning programming, web developers, game developers
- **Core Values**: Simplicity, performance, approachability, community

---

*"The impediment to action advances action. What stands in the way becomes the way."* - Marcus Aurelius

This project is not merely about creating a programming language; it is about discovering whether we can challenge the assumed trade-offs between simplicity and performance. Proceed with patience, clarity of purpose, and acceptance that the path will reveal itself through the walking of it.

May Script guide many programmers on their journey.