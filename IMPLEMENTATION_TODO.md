# Script Language Implementation TODO

This document tracks the implementation progress of the Script programming language, maintaining a comprehensive view of completed work and remaining tasks.

## Project Vision
Script aims to be a programming language that is:
- Simple enough for beginners to learn intuitively
- Powerful enough for production web applications and games
- Expression-oriented with gradual typing
- Memory safe with automatic reference counting
- Compiled to native code and WebAssembly

## Overall Progress

### ✅ Phase 1: Lexer Implementation (COMPLETED)
- [x] Project setup with Rust
- [x] Token definitions for all language features
- [x] Scanner implementation with Unicode support
- [x] Error reporting with source locations
- [x] Interactive REPL
- [x] File tokenization (.script files)
- [x] Comprehensive test suite (18 tests)
- [x] Performance benchmarks
- [x] Example Script files

### ✅ Phase 2: Parser & AST (COMPLETED)
- [x] AST node definitions
  - [x] Expression nodes (Literal, Binary, Unary, Variable, Call, If, Block, Array, Member, Index, Assign)
  - [x] Statement nodes (Let, Function, Return, Expression, While, For)
  - [x] Type annotation nodes (Named, Array, Function)
  - [ ] Pattern nodes for pattern matching (future)
- [x] Parser implementation
  - [x] Recursive descent parser structure
  - [x] Expression parsing with Pratt parsing
  - [x] Statement parsing
  - [x] Type annotation parsing
  - [x] Error recovery and synchronization
- [x] Parser tests
  - [x] Unit tests for each node type (17 tests)
  - [x] Integration tests with full programs
  - [x] Complex expression tests
- [x] REPL enhancement to show AST
- [x] Parser benchmarks

### ✅ Phase 3: Type System & Semantic Analysis (COMPLETED)
- [x] Type representation
  - [x] Basic types (i32, f32, bool, string)
  - [x] Function types with parameter and return types
  - [x] Array types with element type
  - [x] Result<T, E> type for error handling
  - [x] Type variable support for inference
  - [x] Unknown type for gradual typing
  - [ ] User-defined types (structs, enums) - future
  - [ ] Actor types for concurrency model - future
  - [ ] Generic types with constraints - future
- [x] Type inference engine
  - [x] Hindley-Milner type inference core
  - [x] Type variable generation and substitution
  - [x] Unification algorithm with occurs check
  - [x] Constraint generation from AST
  - [x] Gradual typing support (mix typed/untyped)
  - [x] Type annotations integration
  - [x] Structural type compatibility checking
- [x] Semantic analysis
  - [x] Symbol table with scope management
  - [x] Variable resolution with shadowing
  - [x] Function resolution with overloading support
  - [x] Basic semantic validation passes
  - [x] Symbol usage tracking
  - [ ] Type checking pass integration - next step
  - [ ] Const function validation - future
  - [ ] Actor message type checking - future
  - [ ] Memory safety analysis - future
- [x] Error reporting enhancements
  - [x] Semantic error types (undefined vars, duplicate defs)
  - [x] Type mismatch errors in inference
  - [x] Multiple error collection
  - [x] Source location tracking

### ✅ Phase 4: IR & Code Generation (COMPLETED)
- [x] Intermediate Representation (IR)
  - [x] Define Script IR format (SSA-based)
  - [x] AST to IR lowering
  - [x] IR builder and validation
  - [ ] IR optimization passes (future)
    - [ ] Constant folding
    - [ ] Dead code elimination
    - [ ] Common subexpression elimination
- [x] Cranelift backend (development)
  - [x] Basic code generation infrastructure
  - [x] Function compilation pipeline
  - [x] Runtime function registration
  - [ ] Full instruction set implementation (partial)
- [ ] LLVM backend (production) - future
  - [ ] LLVM bindings setup
  - [ ] Optimization pipeline
  - [ ] Debug information generation
- [ ] WebAssembly target - future
  - [ ] WASM code generation
  - [ ] JavaScript interop layer
  - [ ] Browser testing framework

### ✅ Phase 5: Runtime & Standard Library (COMPLETED)
- [x] Memory management
  - [x] Reference counting (RC) implementation
  - [x] RC smart pointer types (ScriptRc<T>, ScriptWeak<T>)
  - [x] Cycle detection algorithm
  - [x] Memory allocation tracking
  - [x] Memory profiler and leak detector
- [x] Runtime core
  - [x] Runtime initialization
  - [x] Panic handling mechanism
  - [x] Stack trace generation
  - [x] Error propagation support
  - [x] Dynamic dispatch infrastructure
- [x] Core standard library
  - [x] I/O operations (print, println, eprintln)
  - [x] File I/O (read_file, write_file)
  - [x] String manipulation functions
  - [x] Result<T, E> implementation
  - [x] Option<T> implementation
- [x] Collections
  - [x] Vec<T> dynamic array
  - [x] HashMap<K, V> hash table
  - [x] String type with UTF-8 support
  - [x] Iterator support
- [x] Game-oriented utilities
  - [x] Vector math (Vec2, Vec3, Vec4, Mat4)
  - [x] Matrix operations (transformations, projections)
  - [x] Random number generation (RNG)
  - [x] Time/Timer utilities
    - [x] High-precision timers
    - [x] Delta time calculation
    - [x] Frame rate helpers
  - [x] Math utilities (lerp, clamp, smoothstep, easing)
  - [x] Color types (RGBA, HSV, HSL conversions)

### 🚧 Phase 6: Advanced Features (IN PROGRESS)
- [x] Pattern matching **COMPLETED**
  - [x] Match expressions - AST definitions, parser implementation
  - [x] Destructuring - Array and object pattern parsing
  - [x] Guards - Optional if expressions in match arms
  - [x] Semantic analysis - Pattern variable binding and type checking
  - [x] Type inference - Pattern compatibility checking
  - [x] Lowering - IR generation for pattern tests and variable binding
  - [ ] Comprehensive testing - Missing dedicated pattern matching tests
  - [ ] Documentation - Pattern matching examples and guides
- [ ] Modules and packages **NEXT PRIORITY**
  - [ ] Module system design
  - [ ] Import/export syntax
  - [ ] Package manifest format
- [ ] Async/await support
  - [ ] Async runtime
  - [ ] Future types
  - [ ] Task scheduling
- [ ] Built-in metaprogramming
  - [ ] @derive attributes (Debug, Serialize, etc.)
  - [ ] @const function support
  - [ ] @generate for external code generation
  - [ ] List comprehensions

### 📋 Phase 7: Tooling & Ecosystem (PLANNED)
- [ ] Language Server Protocol (LSP)
  - [ ] Syntax highlighting
  - [ ] Auto-completion
  - [ ] Go-to definition
  - [ ] Refactoring support
  - [ ] Inline errors
- [ ] Package manager ("manuscript")
  - [ ] Dependency resolution
  - [ ] Package registry design
  - [ ] Build system integration
- [ ] Documentation generator
  - [ ] Doc comment syntax
  - [ ] HTML generation
  - [ ] Search functionality
- [ ] Testing framework
  - [ ] Built-in test runner
  - [ ] Assertion library
  - [ ] Coverage reporting
- [ ] Debugger support
  - [ ] Debug symbols
  - [ ] Breakpoint support
  - [ ] Stack traces

### 📋 Phase 8: Optimizations & Performance (PLANNED)
- [ ] Advanced optimizations
  - [ ] Inlining
  - [ ] Loop optimizations
  - [ ] Vectorization
  - [ ] Escape analysis
- [ ] Profile-guided optimization
- [ ] Link-time optimization
- [ ] Incremental compilation
- [ ] Parallel compilation

## Technical Decisions Made

1. **Implementation Language**: Rust (for memory safety and performance)
2. **Parsing Strategy**: Hand-written recursive descent with Pratt parsing
3. **Memory Model**: Automatic Reference Counting with cycle detection
4. **Type System**: Gradual typing with Hindley-Milner inference
5. **Compilation Strategy**: Dual backend (Cranelift for dev, LLVM for prod)
6. **Error Philosophy**: Multiple errors per compilation, helpful messages
7. **Syntax Style**: JavaScript/GDScript inspired for familiarity

## Open Design Questions

1. **Concurrency Model**: Actor model vs shared memory with safety?
-  Actor model with escape hatches
The actor model aligns perfectly with your philosophy:

Beginners: "Send messages between actors" is intuitive - like passing notes
Games: Natural fit for game entities communicating
Web: Maps well to web workers and async operations
Safety: Eliminates data races by design
```
// Simple actor example
actor Player {
    let x: f32 = 0
    let y: f32 = 0
    
    receive move(dx, dy) {
        x += dx
        y += dy
    }
}

// But allow shared memory for performance-critical paths
unsafe shared {
    let physics_cache = SharedArray<Vec3>()
}
```
2. **Error Handling**: Result types vs exceptions vs panic?
- Result types with syntactic sugar
Results are more explicit and beginner-friendly than exceptions:

No invisible control flow
Errors are values, making them less scary
Can add sugar to reduce boilerplate
```
// Result type approach
fn divide(a: f32, b: f32) -> Result<f32> {
    if b == 0 {
        Err("Division by zero")
    } else {
        Ok(a / b)
    }
}

// With syntactic sugar using ? operator
fn calculate() -> Result<f32> {
    let x = divide(10, 2)?  // Returns early if error
    let y = divide(x, 3)?
    Ok(x + y)
}

// Panic only for unrecoverable errors
assert(index < array.len, "Index out of bounds")
```
3. **Trait/Interface System**: Structural vs nominal typing?
- Structural typing with optional nominal
Structural typing is more beginner-friendly and flexible:

"If it walks like a duck..." is intuitive
No need to explicitly implement interfaces
Natural for JavaScript developers
```
// Structural by default
type Drawable = {
    draw(canvas: Canvas) -> ()
}

// Any type with a draw method works
struct Circle {
    radius: f32
    
    fn draw(canvas: Canvas) {
        // implementation
    }
}

// Optional nominal for when you need it
trait GameEntity {
    fn update(dt: f32)
    fn render()
}

impl GameEntity for Player {
    // explicit implementation
}
```
4. **Macro System**: Include macros or keep language simple?
- No user-defined macros, but built-in metaprogramming
Macros add significant complexity for beginners. Instead:

Provide powerful built-in constructs
Code generation through external tools
Template-like features for common patterns
```
// Built-in derives instead of macros
@derive(Debug, Serialize)
struct Player {
    name: String
    score: i32
}

// Built-in list comprehensions
let doubled = [x * 2 for x in numbers if x > 0]

// External codegen for complex cases
@generate("protobuf", "schema.proto")
```
5. **Package Distribution**: Centralized registry vs decentralized?
- Hybrid approach - start centralized, allow mirrors
Begin with simplicity:

Central registry for discoverability (like npm, crates.io)
Git URLs as escape hatch
Mirror support from day one
```
// manuscript.toml
[dependencies]
web-framework = "1.2.0"  // from registry
game-engine = { git = "https://github.com/user/engine" }
internal-lib = { path = "../libs/internal" }

[registries]
default = "https://manuscript.script-lang.org"
corporate = "https://internal.company.com/manuscript"
```
6. **Compile-time Execution**: Const functions vs compile-time interpreter?
- Const functions with clear boundaries
Simpler than full compile-time interpretation:

Mark functions as @const for compile-time evaluation
Clear rules about what can be const
Useful for configuration and optimization
```
@const
fn calculate_pi(iterations: i32) -> f32 {
    // Can only call other @const functions
    // No I/O, no randomness
    let pi = // ... calculation
    pi
}

// Evaluated at compile time
const PI = calculate_pi(1000000)

// Conditional compilation
@const
fn target_os() -> String {
    // Returns "windows", "macos", "linux", etc.
}

if @const(target_os() == "windows") {
    // Windows-specific code
}
```
## Current Focus (Phase 6: Advanced Features)

With Phase 5 (Runtime & Standard Library) now complete, the focus shifts to advanced language features:

### Next Major Features:
1. **Pattern Matching** 
   - Match expressions with guards
   - Destructuring for arrays and objects
   - Or patterns and wildcards

2. **Async/Await Support**
   - Async runtime implementation
   - Future types and task scheduling
   - Integration with actor model

3. **Module System**
   - Import/export syntax
   - Package manifest format
   - Dependency resolution

### Runtime Architecture (Planned):
```rust
// Memory management
pub struct ScriptRc<T> {
    ptr: NonNull<RcBox<T>>,
    phantom: PhantomData<RcBox<T>>,
}

struct RcBox<T> {
    strong: Cell<usize>,
    weak: Cell<usize>,
    value: T,
}

// Runtime core
pub struct Runtime {
    memory: MemoryManager,
    panic_handler: PanicHandler,
    gc_threshold: usize,
}

// Standard library types
pub enum ScriptValue {
    I32(i32),
    F32(f32),
    Bool(bool),
    String(ScriptRc<String>),
    Array(ScriptRc<Vec<ScriptValue>>),
    Object(ScriptRc<HashMap<String, ScriptValue>>),
    Function(ScriptRc<Function>),
}
```

## Testing Strategy

1. **Unit Tests**: Each language component tested in isolation
2. **Integration Tests**: Full programs testing multiple features
3. **Fuzzing**: Grammar-based fuzzing for parser robustness
4. **Benchmarks**: Performance tracking for each component
5. **Example Programs**: Real-world usage examples

## Community & Documentation

- [ ] Language specification document
- [ ] "Learn Script in Y Minutes" tutorial
- [ ] "Script for Game Developers" guide
- [ ] "Script for Web Developers" guide
- [ ] API documentation for standard library
- [ ] Contribution guidelines
- [ ] Discord/Forum community setup

## Long-term Vision (Post-1.0)

1. **ML/AI Integration**: First-class tensor types and GPU compute
2. **Mobile Targets**: iOS and Android compilation
3. **Script Playground**: Online REPL with sharing
4. **Educational Platform**: Interactive tutorials and courses
5. **Game Engine Integration**: Unity/Godot/Custom engine bindings

---

*Last Updated: Phase 6 In Progress - Pattern Matching Complete*
*Next Phase: Modules & Packages System*

## Phase 6: Module System Implementation Plan

With pattern matching fully implemented across all language layers, the next major priority is implementing a comprehensive module and package system for Script. This system should be beginner-friendly while supporting modern development practices.

### Design Philosophy

The Script module system follows these principles:
1. **Beginner-Friendly**: Simple, intuitive syntax inspired by familiar languages
2. **Explicit Control**: Clear visibility rules and intentional exports
3. **Path-Based**: Module paths correspond to file system structure
4. **Package-Aware**: Built-in support for external dependencies
5. **Performance-Oriented**: Efficient compilation and resolution

### Syntax Design

Based on research of modern languages (Rust, TypeScript, Python, Go), Script will use:

**Module Declaration:**
```script
// In math/geometry.script
export fn area_circle(radius: f32) -> f32 {
    PI * radius * radius
}

export fn area_rectangle(width: f32, height: f32) -> f32 {
    width * height
}

fn internal_helper() -> i32 {  // Not exported - private to module
    42
}

export const PI = 3.14159
```

**Import Syntax:**
```script
// Single import
import { area_circle } from "./math/geometry"

// Multiple imports  
import { area_circle, area_rectangle, PI } from "./math/geometry"

// Import entire module
import * as geo from "./math/geometry"

// Import with alias
import { area_circle as circle_area } from "./math/geometry"

// External package import
import { Vec2, Vec3 } from "mathlib"
import { http_get } from "web"
```

**Re-export Syntax:**
```script
// In math/mod.script - re-export from submodules
export { area_circle, area_rectangle } from "./geometry"
export { sin, cos, tan } from "./trigonometry"
export * from "./linear_algebra"
```

### Implementation Phases

#### Phase 6.1: Lexer & Parser Extensions
- [ ] Add module-related tokens: `import`, `export`, `from`, `as`, `*`
- [ ] Extend AST with import/export statement nodes
- [ ] Implement import/export parsing in parser
- [ ] Add module resolution path handling

#### Phase 6.2: Module Resolution System
- [ ] Design ModuleResolver trait and implementation
- [ ] File-based module resolution (relative/absolute paths)
- [ ] Package-based module resolution (external dependencies)
- [ ] Module caching and dependency graph management
- [ ] Circular dependency detection

#### Phase 6.3: Semantic Analysis Updates
- [ ] Module-aware symbol table with scoping
- [ ] Import statement processing and symbol binding
- [ ] Export statement validation and visibility rules
- [ ] Cross-module type checking and inference

#### Phase 6.4: Package Manifest System
- [ ] Design `script.toml` manifest format
- [ ] Dependency declaration and version constraints
- [ ] Package metadata (name, version, author, etc.)
- [ ] Build configuration options

#### Phase 6.5: Integration & Testing
- [ ] Update lowering to handle module boundaries
- [ ] Multi-file compilation pipeline
- [ ] Comprehensive module system tests
- [ ] Performance benchmarks for module resolution

### Package Manifest Format (`script.toml`)

```toml
[package]
name = "my-game"
version = "0.1.0"
authors = ["Warren Gates <warren@example.com>"]
edition = "2024"
description = "A simple game written in Script"

[dependencies]
mathlib = "1.2.0"
graphics = { version = "2.0", features = ["vulkan"] }
gamedev = { git = "https://github.com/gamedev/script-gamedev" }
internal-utils = { path = "./libs/utils" }

[dev-dependencies]
test-framework = "0.5.0"

[build]
target = "native"  # or "wasm", "cross-platform"
optimization = "release"  # or "debug"
```

### Module Resolution Algorithm

1. **Relative Imports** (`./path` or `../path`):
   - Resolve relative to current file location
   - Look for `.script` files or directories with `mod.script`

2. **Package Imports** (bare specifiers like `"mathlib"`):
   - Check local `script_modules/` directory
   - Fall back to global package cache
   - Download from registry if not found locally

3. **Standard Library** (built-in modules):
   - `std::io`, `std::math`, `std::collections`, etc.
   - Always available without explicit dependencies

### File System Structure

```
my-project/
├── script.toml                 # Package manifest
├── src/
│   ├── main.script            # Entry point
│   ├── game/
│   │   ├── mod.script         # Module re-exports
│   │   ├── player.script      # Player module
│   │   └── enemies.script     # Enemies module
│   └── utils/
│       └── math.script        # Utility functions
├── script_modules/            # Local dependencies
│   └── mathlib/
│       ├── script.toml
│       └── src/
└── tests/
    └── integration_tests.script
```

### Error Handling & Diagnostics

The module system will provide clear error messages for:
- Module not found errors with suggestions
- Circular dependency detection with cycle visualization  
- Import/export mismatch errors
- Version conflict resolution
- Duplicate export errors

### Integration with Existing Systems

- **Type System**: Cross-module type checking and inference
- **Semantic Analysis**: Module-aware symbol resolution
- **Code Generation**: Module boundary handling in IR
- **Standard Library**: Expose stdlib as importable modules
- **REPL**: Support for importing modules in interactive mode

This implementation plan provides a solid foundation for a modern, developer-friendly module system that scales from simple scripts to complex applications while maintaining Script's core philosophy of simplicity and power.

## Phase 3 Summary

Successfully implemented a complete type system and semantic analysis framework:
- **Type System**: Full type representation with basic types, composite types (arrays, functions, results), and gradual typing support
- **Type Inference**: Hindley-Milner algorithm with unification, supporting both explicit annotations and inference
- **Semantic Analysis**: Symbol table with scope management, variable/function resolution, and comprehensive error detection
- **108 passing tests** (35 from phases 1-2 + 73 new tests)
- **3 Major Components**:
  - Type system foundation (19 tests)
  - Type inference engine (39 tests) 
  - Semantic analyzer (50 tests)
- Full integration with existing parser and error reporting

The type system is ready for integration with code generation in Phase 4.

## Phase 4 Summary

Successfully implemented IR and code generation framework:
- **Intermediate Representation**: SSA-based IR with complete instruction set
- **AST to IR Lowering**: Full lowering infrastructure with type preservation
- **Cranelift Backend**: JIT compilation with runtime function registration
- **Compilation Pipeline**: Complete pipeline from source to executable
- Ready for runtime implementation in Phase 5

## Phase 5 Summary

Successfully implemented runtime and standard library:
- **Memory Management**: Reference counting with cycle detection
- **Runtime Core**: Memory allocation, panic handling, profiling
- **Standard Library**: Complete I/O, collections, string manipulation
- **Game Utilities**: Vector math, RNG, time utilities, color handling
- **Math Functions**: Full set of mathematical operations
- Note: Some runtime tests have deadlock issues that need resolution

## Phase 2 Summary

Successfully implemented a complete parser for Script with:
- 35 passing tests (18 lexer + 17 parser)
- Full expression parsing with proper precedence
- All planned statement types
- Type annotation support
- Dual-mode REPL (tokens/AST)
- Performance benchmarks
- Clean error reporting