# Architecture Overview

## Table of Contents

- [System Architecture](#system-architecture)
- [Component Relationships](#component-relationships)
- [Core Modules](#core-modules)
- [Data Flow](#data-flow)
- [Design Principles](#design-principles)
- [Module Organization](#module-organization)

## System Architecture

The Script programming language is implemented as a multi-stage compiler with a runtime system. The architecture follows a traditional compiler pipeline with modern features including gradual typing, reference counting with cycle detection, and JIT compilation.

```
┌─────────────┐    ┌──────────────┐    ┌─────────────────┐    ┌──────────────┐
│   Source    │───▶│    Lexer     │───▶│     Parser      │───▶│   Semantic   │
│    Code     │    │ (Tokenizer)  │    │   (AST Gen)     │    │   Analysis   │
└─────────────┘    └──────────────┘    └─────────────────┘    └──────────────┘
                                                                      │
                                                                      ▼
┌─────────────┐    ┌──────────────┐    ┌─────────────────┐    ┌──────────────┐
│   Runtime   │◀───│ Code Gen     │◀───│   IR Builder    │◀───│    Type      │
│   System    │    │ (Cranelift)  │    │   (Lowering)    │    │  Inference   │
└─────────────┘    └──────────────┘    └─────────────────┘    └──────────────┘
```

## Component Relationships

### Frontend Components

The frontend is responsible for parsing source code and building an intermediate representation:

- **Lexer**: Tokenizes source code with Unicode support and error recovery
- **Parser**: Builds an Abstract Syntax Tree (AST) using recursive descent with Pratt parsing for expressions
- **Semantic Analyzer**: Performs symbol resolution, scope analysis, and semantic validation
- **Type Inference**: Implements Hindley-Milner type inference with gradual typing support

### Backend Components

The backend transforms the analyzed code into executable form:

- **IR Builder**: Lowers AST to Static Single Assignment (SSA) intermediate representation
- **Code Generator**: Compiles IR to native code using the Cranelift JIT compiler
- **Runtime System**: Manages memory, provides standard library, and handles execution

### Support Systems

- **Error Handling**: Comprehensive error reporting with source location tracking
- **Memory Management**: Reference counting with cycle detection for automatic memory management
- **Standard Library**: Core functionality including I/O, collections, math, and game development features

## Core Modules

### `src/lexer/`
- **Purpose**: Tokenization of source code
- **Key Components**:
  - `Scanner`: Main tokenization logic with Unicode support
  - `Token`: Token types and data structures
  - Error recovery and reporting
- **Features**: Unicode support, keyword recognition, number literals, string literals, operators

### `src/parser/`
- **Purpose**: AST construction from tokens
- **Key Components**:
  - `Parser`: Recursive descent parser with Pratt parsing for expressions
  - `AST`: Node definitions for all language constructs
- **Features**: Expression-oriented design, error recovery, precedence handling

### `src/semantic/`
- **Purpose**: Semantic analysis and validation
- **Key Components**:
  - `SemanticAnalyzer`: Main analysis engine
  - `SymbolTable`: Symbol resolution and scope management
  - `Symbol`: Symbol representation with type information
- **Features**: Scope tracking, symbol resolution, semantic validation

### `src/inference/`
- **Purpose**: Type inference and constraint solving
- **Key Components**:
  - `InferenceEngine`: Hindley-Milner type inference
  - `Constraint`: Type constraints and unification
  - `Substitution`: Type variable substitution
- **Features**: Gradual typing, type variable generation, constraint solving

### `src/ir/`
- **Purpose**: Intermediate representation
- **Key Components**:
  - `IrBuilder`: IR construction utilities
  - `Module`: IR module representation
  - `Instruction`: SSA instruction set
  - `Value`: SSA values and constants
- **Features**: SSA form, type preservation, optimization-friendly

### `src/lowering/`
- **Purpose**: AST to IR transformation
- **Key Components**:
  - `AstLowerer`: Main lowering logic
  - `LoweringContext`: Context management during lowering
- **Features**: Expression lowering, statement lowering, control flow handling

### `src/codegen/`
- **Purpose**: Code generation
- **Key Components**:
  - `CodeGenerator`: High-level code generation interface
  - `cranelift/`: Cranelift backend implementation
- **Features**: JIT compilation, multiple backend support, executable module generation

### `src/runtime/`
- **Purpose**: Runtime system
- **Key Components**:
  - `ScriptRc`/`ScriptWeak`: Reference counting smart pointers
  - `CycleCollector`: Cycle detection and collection
  - `Runtime`: Runtime initialization and management
  - `MemoryProfiler`: Memory usage tracking
- **Features**: Thread-safe reference counting, cycle detection, panic handling

### `src/stdlib/`
- **Purpose**: Standard library implementation
- **Key Components**:
  - Core types and operations
  - Collections (arrays, maps)
  - I/O operations
  - Math functions
  - Game development utilities
- **Features**: Comprehensive standard library, game-focused features

## Data Flow

### Compilation Flow

1. **Source Input**: Raw source code text
2. **Tokenization**: Source → Token stream
3. **Parsing**: Tokens → Abstract Syntax Tree (AST)
4. **Semantic Analysis**: AST → Typed AST + Symbol Table
5. **Type Inference**: Typed AST → Fully typed AST with inferred types
6. **IR Generation**: Typed AST → SSA Intermediate Representation
7. **Code Generation**: IR → Native machine code (via Cranelift)

### Runtime Flow

1. **Initialization**: Runtime system startup, memory management initialization
2. **Execution**: JIT-compiled code execution with runtime support
3. **Memory Management**: Automatic reference counting with cycle detection
4. **Standard Library**: Built-in function and type support
5. **Cleanup**: Graceful shutdown with resource cleanup

### Error Handling Flow

Errors are handled at each stage with comprehensive reporting:

- **Lexical Errors**: Invalid characters, malformed literals
- **Syntax Errors**: Parse errors with recovery and multiple error reporting
- **Semantic Errors**: Type mismatches, undefined variables, scope violations
- **Runtime Errors**: Memory allocation failures, panic conditions

## Design Principles

### Expression-Oriented Design
- Everything is an expression that returns a value
- If expressions, while loops, and blocks all return values
- Simplifies the language model and enables functional-style programming

### Gradual Typing
- Optional type annotations with inference
- `unknown` type allows gradual migration from dynamic to static typing
- Type compatibility rules support both strict and flexible typing

### Memory Safety
- Reference counting prevents use-after-free
- Cycle detection prevents memory leaks
- Thread-safe design for future concurrency features

### Error Recovery
- Lexer continues after errors to find multiple issues
- Parser uses panic-mode recovery to continue parsing
- Comprehensive error reporting with source locations

### Modularity
- Clear separation of concerns between compiler phases
- Pluggable backends (currently Cranelift, future LLVM support)
- Extensible standard library design

### Performance
- SSA-based IR enables optimizations
- JIT compilation for fast startup and execution
- Efficient memory management with minimal overhead

## Module Organization

The codebase is organized into logical modules that follow the compiler pipeline:

```
src/
├── lexer/          # Tokenization (Phase 1)
├── parser/         # AST Generation (Phase 2)
├── semantic/       # Semantic Analysis (Phase 3)
├── inference/      # Type Inference (Phase 4)
├── lowering/       # AST to IR (Phase 5)
├── ir/             # Intermediate Representation
├── codegen/        # Code Generation (Phase 6)
├── runtime/        # Runtime System
├── stdlib/         # Standard Library
├── types/          # Type System
├── error/          # Error Handling
└── source/         # Source Location Tracking
```

### Cross-Module Dependencies

- **Error Handling**: Used by all modules for consistent error reporting
- **Source Tracking**: Provides location information throughout the pipeline
- **Types**: Shared type definitions used from semantic analysis onward
- **Runtime**: Integrated with generated code for memory management

### Public API

The main library exports key types and functions:

```rust
pub use error::{Error, Result};
pub use lexer::{Lexer, Token, TokenKind};
pub use parser::{Parser, Program, Stmt, Expr};
pub use semantic::{SemanticAnalyzer, Symbol, SymbolTable};
pub use inference::{InferenceEngine, InferenceResult};
pub use ir::{IrBuilder, Module as IrModule};
pub use codegen::{CodeGenerator, ExecutableModule};
pub use runtime::{Runtime, ScriptRc, ScriptWeak};
```

This architecture provides a solid foundation for the Script programming language, with clear separation of concerns, comprehensive error handling, and extensible design for future enhancements.