# Closure Testing Standards

**Status**: ACTIVE  
**Created**: 2025-01-10  
**Updated**: 2025-01-10  
**Category**: Testing Standards  

## Overview

This document defines comprehensive testing standards for the Script language closure system. Following the successful implementation of closures with 100% functionality coverage, these standards ensure consistent, thorough testing practices for maintaining and extending closure functionality.

## Testing Scope

### Core Closure Features
All closure tests must validate:
1. **Creation**: Closure creation with various parameter counts and capture scenarios
2. **Capture Semantics**: By-value and by-reference capture behaviors
3. **Execution**: Closure invocation with correct argument passing
4. **Memory Safety**: Reference counting and cycle detection
5. **Performance**: Optimization features and benchmarks

### Integration Points
Tests must cover closure interaction with:
- Type system (type inference and checking)
- Runtime system (memory management)
- Standard library (functional operations)
- Code generation (IR production)
- Serialization system (all three formats)

## Test Categories

### 1. Unit Tests (Per Module)
Located in `src/runtime/closure/tests.rs` and module-specific test files.

#### Required Coverage:
```rust
// Basic closure creation
#[test]
fn test_closure_creation_with_captures() {
    let closure = create_closure_heap(
        "test_func".to_string(),
        vec!["x".to_string(), "y".to_string()],
        vec![
            ("captured_int".to_string(), Value::I32(42)),
            ("captured_str".to_string(), Value::String("hello".to_string())),
        ],
        false, // captures by value
    );
    
    match closure {
        Value::Closure(c) => {
            assert_eq!(c.function_id, "test_func");
            assert_eq!(c.parameters.len(), 2);
            assert_eq!(c.captured_vars.len(), 2);
            assert!(!c.captures_by_ref);
        }
        _ => panic!("Expected closure value"),
    }
}

// Optimized closure features
#[test]
fn test_optimized_closure_storage() {
    let small_closure = create_optimized_closure_heap(
        "small".to_string(),
        vec![],
        vec![("x".to_string(), Value::I32(1))],
        false,
    );
    
    match small_closure {
        Value::OptimizedClosure(c) => {
            assert_eq!(c.storage_type(), "inline");
        }
        _ => panic!("Expected optimized closure"),
    }
}
```

### 2. Integration Tests
Located in `tests/runtime/` and `tests/integration/`.

#### Required Scenarios:
- Closure creation → execution pipeline
- Closure passing between functions
- Closure capture of other closures
- Async closure execution
- Cross-module closure usage

```rust
#[test]
fn test_closure_execution_pipeline() {
    let mut runtime = ClosureRuntime::new();
    
    // Register implementation
    runtime.register_closure("add", |args| {
        match (&args[0], &args[1]) {
            (Value::I32(a), Value::I32(b)) => Ok(Value::I32(a + b)),
            _ => Err(Error::new(ErrorKind::TypeError, "Expected integers")),
        }
    });
    
    // Create and execute
    let closure = create_closure_heap(
        "add".to_string(),
        vec!["a".to_string(), "b".to_string()],
        vec![],
        false,
    );
    
    let result = runtime.execute_closure(
        &extract_closure(&closure), 
        &[Value::I32(10), Value::I32(20)]
    );
    
    assert_eq!(result.unwrap(), Value::I32(30));
}
```

### 3. Performance Tests
Located in `benches/closure_bench.rs`.

#### Required Benchmarks:
```rust
fn benchmark_closure_creation(c: &mut Criterion) {
    c.bench_function("closure_creation_small", |b| {
        b.iter(|| {
            create_closure_heap(
                "test".to_string(),
                vec!["x".to_string()],
                vec![("y".to_string(), Value::I32(42))],
                false,
            )
        });
    });
    
    c.bench_function("optimized_vs_regular", |b| {
        b.iter_batched(
            || generate_large_capture_set(100),
            |captures| {
                let opt = create_optimized_closure_heap(
                    "test".to_string(), vec![], captures.clone(), false
                );
                let reg = create_closure_heap(
                    "test".to_string(), vec![], captures, false
                );
                (opt, reg)
            },
            BatchSize::SmallInput,
        );
    });
}
```

### 4. Security Tests
Located in `tests/security/closure_security_test.rs`.

#### Required Validations:
- Stack overflow protection (deep recursion)
- Memory exhaustion prevention (capture limits)
- Serialization size limits
- Malformed closure data handling

```rust
#[test]
fn test_closure_recursion_limit() {
    let mut runtime = ClosureRuntime::new();
    
    // Create recursive closure
    runtime.register_closure("recurse", |args| {
        // Recursive implementation
    });
    
    let closure = create_closure_heap("recurse".to_string(), vec![], vec![], false);
    
    // Should fail gracefully with recursion limit
    let result = runtime.execute_closure(&extract_closure(&closure), &[]);
    assert!(matches!(result, Err(Error { kind: ErrorKind::RecursionLimit, .. })));
}
```

### 5. Property-Based Tests
Using `proptest` for invariant validation.

```rust
proptest! {
    #[test]
    fn test_closure_serialization_roundtrip(
        function_id in "\\PC+",
        param_count in 0usize..10,
        capture_count in 0usize..20,
    ) {
        let params = (0..param_count)
            .map(|i| format!("param_{}", i))
            .collect();
        let captures = (0..capture_count)
            .map(|i| (format!("cap_{}", i), Value::I32(i as i32)))
            .collect();
            
        let closure = create_closure_heap(function_id, params, captures, false);
        
        // Test all serialization formats
        for format in [Binary, Json, Compact] {
            let serialized = serialize_closure(&closure, format)?;
            let deserialized = deserialize_closure(&serialized, format)?;
            
            prop_assert_eq!(
                extract_closure(&closure).function_id,
                extract_closure(&deserialized).function_id
            );
        }
    }
}
```

## Test Organization

### File Structure
```
tests/
├── runtime/
│   ├── closure_basic_tests.rs           # Core functionality
│   ├── closure_serialization_tests.rs   # Serialization/deserialization
│   ├── closure_memory_tests.rs          # Memory management
│   ├── closure_optimization_tests.rs    # Performance optimizations
│   └── closure_integration_tests.rs     # End-to-end scenarios
├── security/
│   └── closure_security_tests.rs        # Security validations
├── integration/
│   └── closure_stdlib_tests.rs          # Stdlib integration
└── fixtures/
    └── closures/                        # Test data files
```

### Test Naming Convention
```
test_[component]_[feature]_[scenario]_[expected_result]

Examples:
- test_closure_creation_with_captures_succeeds
- test_closure_execution_missing_params_fails
- test_optimized_closure_inline_storage_for_small_captures
- test_closure_serialization_binary_format_roundtrip
```

## Coverage Requirements

### Minimum Coverage Targets
- **Line Coverage**: 90% minimum
- **Branch Coverage**: 85% minimum
- **Function Coverage**: 100% for public APIs
- **Integration Coverage**: All documented use cases

### Critical Path Coverage
These areas require 100% test coverage:
1. Memory safety operations (reference counting, cycle detection)
2. Error handling paths (all error conditions)
3. Security validations (bounds checking, resource limits)
4. Public API surface (all stdlib functions)

## Test Quality Standards

### 1. Test Independence
Each test must be fully independent:
- No shared mutable state
- Clean runtime/environment per test
- No ordering dependencies

### 2. Test Clarity
Tests should be self-documenting:
```rust
#[test]
fn test_closure_captures_preserve_types() {
    // Arrange: Create closure with various typed captures
    let captures = vec![
        ("int_val".to_string(), Value::I32(42)),
        ("float_val".to_string(), Value::F32(3.14)),
        ("string_val".to_string(), Value::String("test".to_string())),
    ];
    
    // Act: Create closure and retrieve captures
    let closure = create_closure_heap("test".to_string(), vec![], captures, false);
    let retrieved = extract_closure(&closure);
    
    // Assert: All types preserved correctly
    assert_eq!(retrieved.captured_vars["int_val"], Value::I32(42));
    assert_eq!(retrieved.captured_vars["float_val"], Value::F32(3.14));
    assert_eq!(retrieved.captured_vars["string_val"], Value::String("test".to_string()));
}
```

### 3. Error Testing
All error paths must be tested:
```rust
#[test]
fn test_closure_execution_parameter_mismatch() {
    let mut runtime = ClosureRuntime::new();
    runtime.register_closure("needs_two", |_| Ok(Value::Null));
    
    let closure = create_closure_heap(
        "needs_two".to_string(),
        vec!["a".to_string(), "b".to_string()], // Expects 2 params
        vec![],
        false,
    );
    
    // Call with wrong number of arguments
    let result = runtime.execute_closure(
        &extract_closure(&closure),
        &[Value::I32(1)], // Only 1 argument
    );
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("expected 2 arguments"));
}
```

### 4. Performance Assertions
Performance tests must include assertions:
```rust
#[bench]
fn bench_closure_creation_performance(b: &mut Bencher) {
    let baseline_ns = 1000; // Expected max nanoseconds
    
    b.iter(|| {
        create_closure_heap("test".to_string(), vec![], vec![], false)
    });
    
    assert!(
        b.ns_per_iter() < baseline_ns,
        "Closure creation too slow: {} ns > {} ns",
        b.ns_per_iter(),
        baseline_ns
    );
}
```

## Continuous Integration

### Required CI Checks
1. **All tests pass** on Linux, macOS, Windows
2. **Coverage thresholds** met (90% line, 85% branch)
3. **No performance regressions** (benchmarks within 5% of baseline)
4. **Security tests pass** (all attack vectors tested)
5. **Memory leak detection** (valgrind/sanitizers clean)

### Test Execution
```bash
# Run all closure tests
cargo test closure

# Run with coverage
cargo tarpaulin --out html --include-tests closure

# Run benchmarks
cargo bench closure

# Run security tests
cargo test --test closure_security_tests

# Run with sanitizers
RUSTFLAGS="-Z sanitizer=address" cargo test closure
```

## Maintenance Guidelines

### Adding New Features
When adding closure features:
1. Write tests FIRST (TDD approach)
2. Cover all new code paths
3. Add integration tests for feature interactions
4. Update benchmarks if performance-relevant
5. Add security tests if external input involved

### Fixing Bugs
When fixing closure bugs:
1. Write failing test that reproduces bug
2. Fix the bug
3. Verify test now passes
4. Add regression test to prevent reoccurrence
5. Check for similar issues in related code

### Performance Optimization
When optimizing closures:
1. Benchmark current performance
2. Implement optimization
3. Verify benchmarks show improvement
4. Ensure no functionality regression
5. Document performance characteristics

## Test Data Management

### Fixture Files
Store reusable test data in `tests/fixtures/closures/`:
```
simple_closure.script      # Basic closure examples
nested_closures.script     # Closures capturing closures
async_closures.script      # Async closure patterns
performance_test.script    # Large closure scenarios
```

### Test Builders
Use builder patterns for complex test setups:
```rust
struct ClosureTestBuilder {
    function_id: String,
    parameters: Vec<String>,
    captures: Vec<(String, Value)>,
    captures_by_ref: bool,
}

impl ClosureTestBuilder {
    fn new(id: &str) -> Self { /* ... */ }
    fn with_param(mut self, name: &str) -> Self { /* ... */ }
    fn with_capture(mut self, name: &str, value: Value) -> Self { /* ... */ }
    fn capturing_by_ref(mut self) -> Self { /* ... */ }
    fn build(self) -> Value { /* ... */ }
}

// Usage
let closure = ClosureTestBuilder::new("test")
    .with_param("x")
    .with_param("y")
    .with_capture("z", Value::I32(42))
    .build();
```

## Debugging Test Failures

### Common Issues and Solutions

1. **Memory Leaks in Tests**
   - Ensure proper cleanup in test teardown
   - Use weak references for cycle testing
   - Clear global state between tests

2. **Flaky Tests**
   - Remove timing dependencies
   - Use deterministic test data
   - Mock external dependencies

3. **Platform-Specific Failures**
   - Use platform-agnostic paths
   - Account for endianness in serialization
   - Test on all target platforms in CI

### Debug Helpers
```rust
// Enable detailed logging for test debugging
#[test]
fn test_with_logging() {
    init_test_logger(); // Initialize logging for tests
    
    closure_debug!("Creating test closure");
    let closure = create_test_closure();
    
    closure_debug!("Closure created: {:?}", closure);
    // ... rest of test
}
```

## Conclusion

These testing standards ensure the Script closure system maintains its high quality and reliability. All contributors must follow these standards when:
- Adding new closure features
- Fixing closure-related bugs  
- Optimizing closure performance
- Refactoring closure code

Regular review and updates of these standards ensure they remain relevant as the closure system evolves.

**Remember**: A well-tested closure system is a reliable closure system!