# Known Issues and Limitations

This document tracks known issues, bugs, and limitations in the Script language implementation (v0.3.0-alpha).

## Critical Issues (Blocking Educational Use)

### 1. Pattern Matching Not Safe ✅ FULLY FIXED
**Severity**: ~~High~~ ~~Medium~~ Low (Resolved)  
**Component**: Parser, Semantic Analysis  
**Description**: ~~Pattern matching lacks exhaustiveness checking~~ ✅ COMPLETE! Pattern matching is now safe with full exhaustiveness checking, or-patterns, and guard awareness.

**Update (2025-07-03)**: 
1. ✅ Basic exhaustiveness checking implemented in `src/semantic/pattern_exhaustiveness.rs`
2. ✅ Or-pattern parsing implemented with `Pipe` token support
3. ✅ Guard-aware exhaustiveness checking completed

```script
// All of these now work correctly:

// Exhaustiveness checking
match x {
    1 => "one",
    2 => "two"
    // Error: non-exhaustive patterns!
}

// Or-patterns
match x {
    1 | 2 | 3 => "small",
    _ => "other"
}

// Guards properly handled
match x {
    n if n > 0 => "positive"
    // Error with note: guards are not exhaustive
}
```

**Resolution**: Pattern matching is now fully safe. The compiler:
- Reports errors for non-exhaustive patterns
- Supports or-patterns with `|` syntax
- Correctly handles guards (with appropriate warnings about runtime behavior)
- Detects redundant patterns (considering guards don't make patterns redundant)


### 2. Generics Don't Compile ❌ CRITICAL
**Severity**: High  
**Component**: Parser, Type System  
**Description**: Generic functions and types are defined in AST but parser doesn't implement them, causing compilation failures.

```script
// This should work but doesn't parse
fn identity<T>(x: T) -> T { x }  // Parser error!
```

**Files Affected**:
- `src/parser/parser.rs:149` - TODO: parse_generic_parameters
- `src/parser/ast.rs` - Missing `generic_params` field implementation
- `src/types/mod.rs` - Generic type variants unused

### 3. Memory Cycles Can Leak
**Severity**: High  
**Component**: Runtime  
**Description**: Reference counting implementation lacks cycle detection, causing memory leaks with circular references.

```script
// This creates a memory leak
let a = Node { next: null }
let b = Node { next: a }
a.next = b  // Circular reference - memory leak!
```

**Files Affected**:
- `src/runtime/rc.rs` - No weak reference support
- `src/runtime/gc.rs` - Cycle detection not implemented

## Major Issues

### 4. Async/Await Not Implemented
**Severity**: Medium  
**Component**: Parser, Runtime  
**Description**: Keywords are recognized but no implementation exists.

```script
// Parses but doesn't work
async fn fetch_data() -> string {
    await http_get("url")  // Runtime error
}
```

### 5. Module Resolution Incomplete
**Severity**: Medium  
**Component**: Module System  
**Description**: Import/export syntax parses but resolution fails for multi-file projects.

```script
// In math.script
export fn add(a, b) { a + b }

// In main.script
import { add } from "./math"  // Resolution fails
```

### 6. Limited Error Handling
**Severity**: Medium  
**Component**: Type System, Runtime  
**Description**: No Result/Option types or try/catch mechanism.

```script
// No way to handle errors gracefully
let file = open("missing.txt")  // Panics if file doesn't exist
```

## Minor Issues

### 7. LSP Features Missing
- No goto definition
- No hover information  
- No rename refactoring
- Completion only works for local variables

### 8. Debugger Non-Functional
- Cannot set breakpoints
- Step commands don't work
- Variable inspection incomplete

### 9. Standard Library Gaps
- No HashMap/Set implementations
- File I/O incomplete
- No regular expressions
- Missing string manipulation functions
- No JSON parsing

### 10. Performance Issues
- Parser allocates excessively
- Type checker is O(n²) for some cases
- No optimization passes in codegen
- Runtime 3x slower than target

## Parser Specific Issues

### 11. Error Recovery Limitations
- Parser can't recover from missing semicolons in all contexts
- Nested function parsing can fail silently
- Some syntax errors produce misleading messages

### 12. Unicode Handling Inconsistent
- Identifiers support Unicode but operators don't
- String escaping doesn't handle all Unicode sequences
- Comments can break with certain emoji

## Type System Issues

### 13. Type Inference Limitations
- Cannot infer types across function boundaries
- Recursive types not supported
- No variance annotations
- Trait bounds not implemented

### 14. Missing Type Features
- No union types
- No intersection types  
- No higher-kinded types
- No associated types

## Runtime Issues

### 15. Limited Platform Support
- Only tested on Linux/macOS
- Windows support uncertain
- No WebAssembly target
- No embedded system support

### 16. Resource Management
- File handles not automatically closed
- No RAII pattern
- Network connections can leak
- No timeout mechanisms

## Tooling Issues

### 17. Build System Limitations
- No incremental compilation
- No build caching
- No parallel compilation
- No cross-compilation support

### 18. Testing Framework Missing
- No built-in test runner
- No assertion library
- No property-based testing
- No coverage tools

## Documentation Issues

### 19. Incomplete Documentation
- Many standard library functions undocumented
- No API stability guarantees
- Migration guides missing
- Performance guide incomplete

### 20. Example Gaps
- No real-world application examples
- Game development examples incomplete
- Web server examples don't compile
- FFI examples missing

## Workarounds

### Pattern Matching Safety
Always include a default case:
```script
match value {
    // ... specific cases ...
    _ => panic("Unhandled case")
}
```

### Memory Cycles
Manually break cycles:
```script
// Before dropping
node.next = null  // Break cycle manually
```

### Error Handling
Use manual checks:
```script
if file_exists(path) {
    let content = read_file(path)
} else {
    print("File not found")
}
```

## Reporting New Issues

Please report issues to: https://github.com/moikapy/script/issues

Include:
1. Script version
2. Minimal reproduction code
3. Expected vs actual behavior
4. Platform information

## Summary: Priorities for Production Use

### 🎓 Educational Use (6-12 months)
**Required for teaching programming safely:**
1. Fix generics parser implementation (TODO at line 149)
2. ~~Implement pattern matching exhaustiveness checking~~ ✅ COMPLETED
3. Add memory cycle detection to prevent leaks
4. Complete module system for multi-file projects
5. Add Result/Option types for error handling
6. Implement HashMap and basic collections
7. Fix debugger for student code inspection

### 🌐 Web App Production (2-3 years)
**Required for building production web applications:**
8. HTTP server framework with routing and middleware
9. JSON parsing/serialization library
10. Database connectivity (SQL drivers + ORM)
11. WebAssembly compilation target
12. JavaScript interop for web ecosystem
13. Security features (HTTPS, auth, sessions)
14. Template engine for dynamic pages
15. WebSocket support for real-time apps

### 🎮 Game Development Production (2-4 years)
**Required for building shippable games:**
16. Graphics/rendering (OpenGL/Vulkan bindings)
17. Audio system (playback/synthesis)
18. Input handling (keyboard/mouse/gamepad)
19. Physics engine integration
20. Asset loading (images/models/audio)
21. Platform builds (console/mobile targets)
22. Real-time performance (60+ FPS guarantees)
23. GPU compute/shader pipeline

### 🤖 AI/ML Production (3-5 years)
**Required for building ML/AI applications:**
24. Tensor operations (NumPy-like arrays)
25. GPU acceleration (CUDA/OpenCL)
26. Python interop (PyTorch/TensorFlow ecosystem)
27. Linear algebra libraries (BLAS/LAPACK)
28. Memory mapping for large datasets
29. Distributed computing primitives
30. JIT optimization for numerical code
31. Scientific libraries (statistics/signal processing)

Last Updated: 2025-01-20