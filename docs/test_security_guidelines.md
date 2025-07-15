# Test Security Guidelines

## Overview

This document establishes security standards for test development in the Script programming language project. These guidelines ensure that tests maintain security while providing comprehensive coverage of defensive mechanisms.

## Core Principles

### 1. Defensive Testing Only
**Always test security measures, never implement exploits.**

✅ **Good Example**:
```rust
#[test]
fn test_memory_limit_enforcement() {
    let test_future = MemoryLimitValidationTest::new(); // Uses 1KB allocations
    let result = runtime.execute_with_limits(test_future);
    assert!(result.memory_usage <= SAFE_LIMIT);
}
```

❌ **Bad Example**:
```rust
#[test] 
fn test_memory_exhaustion() {
    let attack_future = MemoryExhaustionFuture::new(); // Tries to allocate 10GB
    // This creates actual DoS conditions!
}
```

### 2. Resource Awareness
**Limit memory and CPU usage to protect development and CI environments.**

✅ **Good Example**:
```rust
let limits = TestLimits::current();
let result = SafeTestOps::safe_iterate(
    limits.max_iterations,
    &mut monitor,
    |_| test_operation()
);
```

❌ **Bad Example**:
```rust
for i in 0..1_000_000 { // Hard-coded large number
    test_operation(); // No resource monitoring
}
```

### 3. Clear Intent and Documentation
**Mark security-related code with warnings and context.**

✅ **Good Example**:
```rust
//! SECURITY NOTE: This test validates double-poll detection
//! WITHOUT actually implementing double-poll exploits.

/// Test helper that simulates double-poll detection patterns
/// Uses safe, bounded operations to verify runtime protections work.
struct DoublePollDetectionTest { /* ... */ }
```

❌ **Bad Example**:
```rust
// No explanation of why this exists or what it tests
struct ExploitFuture { /* ... */ }
```

### 4. Test Isolation
**Ensure security tests don't affect other tests or environments.**

✅ **Good Example**:
```rust
#[test]
fn test_with_isolation() {
    let mut monitor = ResourceMonitor::new();
    // All operations are bounded and cleaned up
    let result = perform_safe_test(&mut monitor);
    // Automatic cleanup via Drop
}
```

❌ **Bad Example**:
```rust
static mut GLOBAL_STATE: Vec<u8> = Vec::new();
#[test]
fn test_with_side_effects() {
    unsafe { GLOBAL_STATE.push(42); } // Affects other tests
}
```

## Implementation Guidelines

### Security Test Patterns

#### 1. Memory Safety Testing
```rust
// ✅ Safe pattern - bounded allocations with monitoring
#[test]
fn test_memory_bounds_checking() {
    let limits = TestLimits::current();
    let mut monitor = ResourceMonitor::new();
    
    // Test with safe, small allocation
    let result = SafeTestOps::safe_alloc(1024, &mut monitor);
    assert!(result.is_ok());
    
    // Verify bounds are enforced
    let oversized = SafeTestOps::safe_alloc(limits.max_memory_per_test + 1, &mut monitor);
    assert!(oversized.is_err());
}

// ❌ Dangerous pattern - actual memory exhaustion
#[test]
fn test_memory_exhaustion() {
    let mut allocations = Vec::new();
    loop {
        allocations.push(vec![0u8; 10 * 1024 * 1024]); // 10MB each iteration
        // This will crash the test runner!
    }
}
```

#### 2. DoS Protection Testing
```rust
// ✅ Safe pattern - environment-aware resource usage
#[test]
fn test_dos_protection() {
    let limits = TestLimits::current();
    let mut monitor = ResourceMonitor::new();
    
    let result = SafeTestOps::safe_iterate(
        limits.max_type_variables + 100, // Slightly over limit
        &mut monitor,
        |_| engine.create_type_var() // Test limit enforcement
    );
    
    // Should hit limits appropriately for environment
    assert!(result.is_err() || monitor.check_timeout().is_err());
}

// ❌ Dangerous pattern - fixed large resource usage
#[test] 
fn test_type_variable_explosion() {
    for _ in 0..100_000 { // Always creates 100k regardless of environment
        engine.create_type_var(); // Will slow down CI significantly
    }
}
```

#### 3. Async Security Testing
```rust
// ✅ Safe pattern - defensive future testing
struct SecurityValidationFuture {
    iterations: usize,
    max_safe_iterations: usize,
}

impl ScriptFuture for SecurityValidationFuture {
    type Output = Value;
    
    fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
        if self.iterations < self.max_safe_iterations {
            self.iterations += 1;
            waker.wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(Value::Bool(true)) // Test completed safely
        }
    }
}

// ❌ Dangerous pattern - actual exploit implementation
struct DoublePollExploitFuture {
    exploited: bool,
}

impl ScriptFuture for DoublePollExploitFuture {
    type Output = Value;
    
    fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
        if !self.exploited {
            self.exploited = true;
            Poll::Ready(Value::String("exploit successful".to_string()))
        } else {
            // Actually attempting double-poll exploit!
            Poll::Ready(Value::String("double poll exploit".to_string()))
        }
    }
}
```

### Error Handling Best Practices

#### 1. Descriptive Error Messages
```rust
// ✅ Good - descriptive error context
let result = compile_source(source)
    .expect("Failed to compile test source for memory bounds validation");

// ❌ Bad - no context for debugging
let result = compile_source(source).unwrap();
```

#### 2. Test Utility Functions
```rust
// ✅ Good - centralized error handling
pub fn expect_compilation_success(source: &str, test_name: &str) -> SemanticAnalyzer {
    compile_test_source(source)
        .unwrap_or_else(|e| panic!("Test '{}' compilation failed: {}", test_name, e))
}

// Usage:
let analyzer = expect_compilation_success(source, "memory_bounds_test");

// ❌ Bad - repeated error handling patterns
let analyzer = compile_source(source).unwrap(); // No context
```

#### 3. Graceful Test Failures
```rust
// ✅ Good - graceful failure with cleanup
#[test]
fn test_with_cleanup() {
    let mut monitor = ResourceMonitor::new();
    
    let result = std::panic::catch_unwind(|| {
        perform_risky_test_operation(&mut monitor)
    });
    
    // Always cleanup regardless of test outcome
    cleanup_test_resources();
    
    result.unwrap();
}

// ❌ Bad - no cleanup on failure
#[test]
fn test_without_cleanup() {
    setup_global_state();
    perform_test(); // If this panics, global state is corrupted
    cleanup_global_state(); // Never reached if test panics
}
```

## Environment Configuration

### Test Intensity Levels

#### CI Environment (Low Intensity)
```bash
export SCRIPT_TEST_INTENSITY=low
```
- Type variables: 500 max
- Constraints: 2,000 max  
- Memory per test: 1MB max
- Timeout: 5 seconds max
- Iterations: 10 max

#### Development Environment (Medium Intensity)
```bash
export SCRIPT_TEST_INTENSITY=medium
```
- Type variables: 2,000 max
- Constraints: 8,000 max
- Memory per test: 4MB max
- Timeout: 15 seconds max
- Iterations: 50 max

#### Thorough Testing (High Intensity)
```bash
export SCRIPT_TEST_INTENSITY=high
```
- Type variables: 5,000 max
- Constraints: 20,000 max
- Memory per test: 10MB max
- Timeout: 30 seconds max
- Iterations: 200 max

### Usage in Tests
```rust
#[test]
fn test_with_environment_awareness() {
    let limits = TestLimits::current(); // Automatically detects environment
    let mut monitor = ResourceMonitor::new();
    
    // Test scales appropriately for CI vs development
    let iterations = limits.safe_iteration_count(desired_iterations);
    
    for i in 0..iterations {
        perform_test_operation();
        
        if i % 10 == 0 {
            monitor.check_timeout()?; // Respect environment timeouts
        }
    }
}
```

## Code Review Checklist

### Security Review ✓
- [ ] No actual exploit implementations
- [ ] All memory allocations are bounded
- [ ] Resource usage scales with environment
- [ ] Clear documentation about test purpose
- [ ] No unsafe code without justification
- [ ] Proper resource cleanup

### Performance Review ✓
- [ ] Tests complete within environment timeout limits
- [ ] Memory usage stays within configured bounds
- [ ] CPU usage is reasonable for test environment
- [ ] No hard-coded large iteration counts
- [ ] Appropriate use of TestLimits configuration

### Quality Review ✓
- [ ] Descriptive error messages with context
- [ ] Proper use of test utility functions
- [ ] No panic-prone error handling patterns
- [ ] Test isolation maintained
- [ ] Clear intent and documentation

## Anti-Patterns to Avoid

### 1. Hard-Coded Resource Limits
```rust
// ❌ Bad - always uses same large limits
for i in 0..50_000 {
    create_type_variable();
}

// ✅ Good - environment-aware limits
let limits = TestLimits::current();
for i in 0..limits.max_type_variables {
    create_type_variable();
}
```

### 2. Actual Exploit Implementation
```rust
// ❌ Bad - implements real buffer overflow
fn test_buffer_overflow() {
    let mut buffer = [0u8; 10];
    for i in 0..100 { // Overflows buffer!
        buffer[i] = 42;
    }
}

// ✅ Good - tests overflow protection
fn test_buffer_overflow_protection() {
    let buffer = SafeBuffer::new(10);
    let result = buffer.try_write(100, 42); // Should fail safely
    assert!(result.is_err());
}
```

### 3. Resource Leaks in Tests
```rust
// ❌ Bad - leaks resources on failure
#[test]
fn test_with_leak() {
    let resource = allocate_expensive_resource();
    might_panic_operation(); // Resource never freed if this panics
    free_resource(resource);
}

// ✅ Good - automatic cleanup
#[test]
fn test_with_raii() {
    let _resource = ExpensiveResource::new(); // RAII cleanup
    might_panic_operation(); // Resource automatically freed
}
```

### 4. Side Effects Between Tests
```rust
// ❌ Bad - global state affects other tests
static mut GLOBAL_COUNTER: usize = 0;

#[test]
fn test_that_modifies_global() {
    unsafe { GLOBAL_COUNTER += 1; }
    assert_eq!(unsafe { GLOBAL_COUNTER }, 1); // Fails if run after other tests
}

// ✅ Good - isolated test state
#[test]
fn test_with_local_state() {
    let mut local_counter = 0;
    local_counter += 1;
    assert_eq!(local_counter, 1); // Always passes regardless of test order
}
```

## Security Validation Workflow

### 1. Pre-Implementation
- [ ] Review test requirements for security implications
- [ ] Choose appropriate defensive testing patterns
- [ ] Plan resource usage and limits
- [ ] Design test isolation strategy

### 2. Implementation
- [ ] Use TestLimits and ResourceMonitor consistently
- [ ] Implement proper error handling with context
- [ ] Add clear documentation about security aspects
- [ ] Follow established patterns from this guide

### 3. Review
- [ ] Security team review for exploit patterns
- [ ] Performance review for resource usage
- [ ] Code review using checklist above
- [ ] Validation that tests still catch intended issues

### 4. Integration
- [ ] Test in CI environment with low intensity
- [ ] Verify no side effects on other tests
- [ ] Confirm timeout and memory limits work
- [ ] Document any special requirements

## Migration from Unsafe Patterns

### Step 1: Identify Unsafe Tests
Look for these patterns in existing tests:
- Large hard-coded iteration counts (>1000)
- Direct memory allocation without limits
- Actual exploit implementations
- Missing resource cleanup
- Panic-prone error handling

### Step 2: Apply Safe Patterns
- Replace with TestLimits-based resource usage
- Add ResourceMonitor for tracking
- Convert exploits to defensive validation
- Add proper error handling with context
- Ensure test isolation

### Step 3: Validate Coverage
- Confirm tests still catch intended security issues
- Verify defensive patterns work correctly
- Test resource scaling across environments
- Validate no performance regressions in CI

## Conclusion

Following these guidelines ensures that security tests:
- Validate defensive mechanisms effectively
- Don't create actual security risks
- Scale appropriately for different environments
- Maintain high code quality standards
- Provide clear debugging information

Remember: **Test security measures, don't implement exploits.**