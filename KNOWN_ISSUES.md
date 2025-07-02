# Known Issues and Limitations

This document tracks known issues, bugs, and limitations in the Script language implementation (v0.9.0-beta).

## Critical Issues (Blocking 1.0)

### 1. Pattern Matching Not Safe
**Severity**: High  
**Component**: Parser, Semantic Analysis  
**Description**: Pattern matching lacks exhaustiveness checking, making it possible to write code that crashes at runtime due to unhandled cases.

```script
// This compiles but will crash if x is not 1 or 2
match x {
    1 => "one",
    2 => "two"
    // Missing default case - runtime panic!
}
```

**Files Affected**:
- `src/semantic/analyzer.rs` - Missing exhaustiveness checking
- `src/lowering/expr.rs:172` - Or patterns not implemented
- `src/parser/parser.rs` - Guards parsed but not analyzed

### 2. Generics Don't Compile
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

Last Updated: 2025-07-02