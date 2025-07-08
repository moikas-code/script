# Error Handling System Implementation

This document tracks the implementation of a production-ready error handling system for Script, transitioning from panic-based error handling to Result/Option types.

## Implementation Overview

**Goal**: Implement Result<T, E> and Option<T> as first-class language features with proper type system integration, runtime support, and ergonomic syntax (? operator).

## Current State Analysis

### Existing Infrastructure
1. **Error Types**: Well-defined error system in `src/error/mod.rs`
2. **Stdlib Types**: Basic Result/Option in `src/stdlib/core_types.rs` (but as library types, not language features)
3. **Generic Support**: Full generic enum support in AST and type system
4. **Pattern Matching**: Complete exhaustiveness checking ready for Result/Option patterns

### Key Gaps
1. **Runtime Representation**: No enum variant support in `runtime::Value`
2. **Type System Integration**: Result/Option not recognized as built-in types
3. **Code Generation**: No Cranelift codegen for enum variants
4. **Syntax**: No ? operator for error propagation
5. **API Migration**: Hundreds of panic/unwrap calls throughout codebase

## Implementation Plan

### Phase 1: Runtime Foundation ✅ COMPLETED
- [x] Add `Enum` variant to `runtime::Value` type
  - [x] Support variant name and associated data
  - [x] Memory management with ScriptRc
  - [x] Display/Debug implementations
- [x] Add enum value creation/access methods (ok, err, some, none)
- [x] Update value conversion utilities (unwrap_ok_or_some, unwrap_err)
- [x] Add truthiness logic (Err and None are falsy)
- [x] Add unit tests for all functionality

### Phase 2: Type System Integration ✅ COMPLETED
- [x] Add Result<T, E> and Option<T> as built-in generic types
  - [x] Recognize these in type inference (already present)
  - [x] Special-case pattern matching exhaustiveness (using existing system)
  - [x] Type checking for Ok/Err/Some/None constructors
- [x] Update type display to show Result/Option nicely (already present)
- [x] Add built-in enum definitions in semantic analyzer

### Phase 3: Parser & AST Extensions ✅ COMPLETED
- [x] Add ? operator to lexer (Question token)
- [x] Parse ? as postfix operator
- [x] AST node for error propagation expression (ErrorPropagation)
- [x] Update precedence rules (postfix operator)

### Phase 4: Semantic Analysis ✅ COMPLETED  
- [x] Type checking for ? operator
  - [x] Ensure expression is Result/Option type
  - [x] Check function returns compatible type
  - [x] Infer error type compatibility
- [x] Update control flow analysis for early returns
- [x] Add error kinds for invalid ? usage
- [x] Support unqualified Some/None/Ok/Err constructors

### Phase 5: Code Generation
- [ ] Implement enum variant codegen in Cranelift
  - [ ] Discriminant + data layout
  - [ ] Constructor generation
  - [ ] Pattern matching compilation
- [ ] ? operator lowering to match + early return
- [ ] Optimize common patterns (is_ok, unwrap_or)

### Phase 6: Standard Library Enhancement
- [ ] Move Result/Option from stdlib to core
- [ ] Comprehensive method set:
  - [ ] Combinators: map, and_then, or_else
  - [ ] Conversions: ok_or, ok_or_else
  - [ ] Utilities: transpose, flatten
- [ ] Error trait for custom error types
- [ ] From/Into for error conversion

### Phase 7: API Migration
- [ ] Systematic replacement of panics with Results
  - [ ] File I/O operations
  - [ ] Parsing operations
  - [ ] Runtime operations
- [ ] Add #[must_use] equivalent for Results
- [ ] Migration guide for existing code

### Phase 8: Testing & Documentation
- [ ] Comprehensive test suite
  - [ ] Type inference tests
  - [ ] Pattern matching tests
  - [ ] ? operator tests
  - [ ] Error propagation tests
- [ ] Update all documentation
- [ ] Example programs demonstrating error handling

## Technical Design Decisions

### 1. Runtime Representation
```rust
enum Value {
    // ... existing variants ...
    Enum {
        name: String,        // "Result" or "Option"
        variant: String,     // "Ok", "Err", "Some", "None"
        data: Option<Box<Value>>, // Associated data if any
    }
}
```

### 2. Type System Representation
```rust
enum Type {
    // ... existing variants ...
    Result(Box<Type>, Box<Type>), // Result<T, E>
    Option(Box<Type>),            // Option<T>
}
```

### 3. ? Operator Desugaring
```script
// This:
let x = foo()?;

// Desugars to:
let x = match foo() {
    Ok(val) => val,
    Err(e) => return Err(e)
};
```

### 4. Pattern Matching Integration
```script
match result {
    Ok(value) => process(value),
    Err(FileNotFound) => handle_not_found(),
    Err(PermissionDenied) => handle_permission(),
    Err(e) => handle_other(e)
}
```

## Migration Strategy

1. **Backward Compatibility**: Keep panic versions with deprecation warnings
2. **Gradual Migration**: Start with new APIs, migrate existing incrementally  
3. **Tool Support**: Provide automated migration tool for simple cases
4. **Documentation**: Clear migration guide with examples

## Success Criteria

- [ ] All file I/O operations return Result
- [ ] No unwrap() calls in production code paths
- [ ] ? operator works seamlessly
- [ ] Pattern matching on Result/Option is exhaustive
- [ ] Performance overhead < 5% vs panics
- [ ] Clear error messages for type mismatches

## Timeline Estimate

- Phase 1-2: 2-3 days (Runtime & Type System)
- Phase 3-4: 2 days (Parser & Semantic)
- Phase 5: 3-4 days (Code Generation - most complex)
- Phase 6: 2 days (Standard Library)
- Phase 7: 3-4 days (API Migration)
- Phase 8: 2 days (Testing & Docs)

**Total: ~3 weeks for complete implementation**

## Related Issues

- Fixes #6 in KNOWN_ISSUES.md (Error Handling System Evolution)
- Depends on generic enum support (already complete)
- Enables better async/await error handling later
- Improves overall language safety and reliability