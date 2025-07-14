# Script Language Testing Framework

The Script language includes a built-in testing framework that makes it easy to write and run tests for your code. This guide covers how to use the testing framework effectively.

## Table of Contents
- [Writing Tests](#writing-tests)
- [Running Tests](#running-tests)
- [Assertions](#assertions)
- [Test Attributes](#test-attributes)
- [Test Organization](#test-organization)
- [Best Practices](#best-practices)

## Writing Tests

Tests in Script are regular functions decorated with the `@test` attribute:

```script
@test
fn test_addition() {
    let result = 2 + 2
    assert_eq(result, 4)
}
```

### Basic Test Structure

Every test function should:
1. Set up test data
2. Perform the operation being tested
3. Assert the expected outcome

```script
@test
fn test_user_creation() {
    // Setup
    let name = "Alice"
    let age = 30
    
    // Act
    let user = create_user(name, age)
    
    // Assert
    assert_eq(user.name, "Alice")
    assert_eq(user.age, 30)
}
```

## Running Tests

To run tests in a Script file:

```bash
# Run all tests in a file
script test_file.script --test

# Run tests in a directory
script tests/ --test

# Run with verbose output
script test_file.script --test --verbose

# Run with JSON output
script test_file.script --test --format json
```

## Assertions

The testing framework provides a comprehensive set of assertion functions:

### Equality Assertions

```script
assert_eq(actual, expected)      // Assert values are equal
assert_ne(actual, expected)      // Assert values are not equal
```

### Boolean Assertions

```script
assert_true(condition)           // Assert condition is true
assert_false(condition)          // Assert condition is false
assert(condition, message)       // Assert with custom message
```

### Comparison Assertions

```script
assert_gt(left, right)          // Assert left > right
assert_ge(left, right)          // Assert left >= right
assert_lt(left, right)          // Assert left < right
assert_le(left, right)          // Assert left <= right
```

### Collection Assertions

```script
assert_empty(collection)         // Assert collection is empty
assert_len(collection, length)   // Assert collection has specific length
assert_contains(string, substr)  // Assert string contains substring
```

### Floating Point Assertions

```script
assert_approx_eq(actual, expected, tolerance)  // Assert floats are approximately equal
```

### Panic Assertions

```script
@test(should_panic)
fn test_invalid_operation() {
    divide_by_zero()  // This should panic
}

@test(should_panic = "division by zero")
fn test_specific_panic() {
    let x = 10 / 0  // Should panic with specific message
}
```

## Test Attributes

Tests can be configured using attributes:

### Skip Tests

```script
@test(skip = "Not implemented yet")
fn test_future_feature() {
    // This test will be skipped
}
```

### Test Timeouts

```script
@test(timeout = 5000)  // 5 second timeout
fn test_long_operation() {
    // Test fails if it takes longer than 5 seconds
}
```

### Test Tags

```script
@test(tag = "integration")
fn test_database_connection() {
    // Tagged for filtering
}

@test(tag = "unit", tag = "fast")
fn test_string_utils() {
    // Multiple tags
}
```

### Expected Failures

```script
@test(should_panic)
fn test_invalid_input() {
    parse_number("not a number")  // Should panic
}
```

## Test Organization

### Setup and Teardown

Use `@setup` and `@teardown` attributes for test fixtures:

```script
@setup
fn before_each_test() {
    // Runs before each test
    initialize_test_database()
}

@teardown
fn after_each_test() {
    // Runs after each test
    cleanup_test_database()
}
```

### Test Modules

Organize related tests in modules:

```script
// tests/user_tests.script
@test
fn test_create_user() {
    // ...
}

@test
fn test_update_user() {
    // ...
}

// tests/auth_tests.script
@test
fn test_login() {
    // ...
}

@test
fn test_logout() {
    // ...
}
```

## Best Practices

### 1. Descriptive Test Names

Use clear, descriptive names that explain what is being tested:

```script
// Good
@test
fn test_user_creation_with_valid_data() { }

// Bad
@test
fn test1() { }
```

### 2. One Assertion Per Test

Keep tests focused on a single behavior:

```script
// Good - separate tests
@test
fn test_user_has_correct_name() {
    let user = create_user("Alice", 30)
    assert_eq(user.name, "Alice")
}

@test
fn test_user_has_correct_age() {
    let user = create_user("Alice", 30)
    assert_eq(user.age, 30)
}

// Less ideal - multiple assertions
@test
fn test_user_creation() {
    let user = create_user("Alice", 30)
    assert_eq(user.name, "Alice")
    assert_eq(user.age, 30)
}
```

### 3. Test Edge Cases

Don't just test the happy path:

```script
@test
fn test_empty_string_handling() {
    let result = process_text("")
    assert_eq(result, "")
}

@test
fn test_null_handling() {
    let result = process_optional(null)
    assert_eq(result, null)
}

@test(should_panic)
fn test_negative_array_index() {
    let arr = [1, 2, 3]
    let value = arr[-1]  // Should panic
}
```

### 4. Use Test Data Builders

Create helper functions for complex test data:

```script
fn create_test_user(name: String = "Test User", age: Int = 25) -> User {
    return User {
        name: name,
        age: age,
        email: name.lower() + "@example.com"
    }
}

@test
fn test_user_email_generation() {
    let user = create_test_user("Alice Smith")
    assert_eq(user.email, "alice smith@example.com")
}
```

### 5. Keep Tests Independent

Tests should not depend on each other:

```script
// Bad - tests depend on shared state
let shared_counter = 0

@test
fn test_increment() {
    shared_counter = shared_counter + 1
    assert_eq(shared_counter, 1)
}

@test
fn test_decrement() {
    shared_counter = shared_counter - 1
    assert_eq(shared_counter, 0)  // Fails if run after test_increment
}

// Good - tests are independent
@test
fn test_increment() {
    let counter = 0
    counter = counter + 1
    assert_eq(counter, 1)
}
```

## Test Output

The testing framework provides several output formats:

### Console Output (Default)

```
Script: Testing examples/test_basics.script

test test_addition ... ok (0.001s)
test test_string_concatenation ... ok (0.002s)
test test_comparisons ... ok (0.001s)
test test_advanced_feature ... skipped - Not implemented yet
test test_division_by_zero ... ok (0.001s)
test test_with_timeout ... ok (0.050s)

Test Summary: 6 total, 5 passed, 0 failed, 1 skipped (0.056s)
```

### JSON Output

```json
{
  "test_results": [
    {
      "name": "test_addition",
      "status": "passed",
      "duration_ms": 1,
      "error": null
    },
    {
      "name": "test_division_by_zero",
      "status": "passed",
      "duration_ms": 1,
      "error": null
    }
  ],
  "summary": {
    "total": 6,
    "passed": 5,
    "failed": 0,
    "skipped": 1
  }
}
```

### JUnit XML Output

For CI/CD integration:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<testsuites>
  <testsuite name="script-tests" tests="6" failures="0" skipped="1" time="0.056">
    <testcase name="test_addition" time="0.001"/>
    <testcase name="test_string_concatenation" time="0.002"/>
    <testcase name="test_advanced_feature" time="0">
      <skipped message="Not implemented yet"/>
    </testcase>
  </testsuite>
</testsuites>
```

## Advanced Features

### Parameterized Tests (Future)

```script
@test(params = [
    (2, 2, 4),
    (3, 3, 6),
    (5, 5, 10)
])
fn test_addition_parameterized(a: Int, b: Int, expected: Int) {
    assert_eq(a + b, expected)
}
```

### Property-Based Testing (Future)

```script
@test(property)
fn test_addition_commutative(a: Int, b: Int) {
    assert_eq(a + b, b + a)
}
```

## Integration with Development Workflow

### Pre-commit Hooks

Add to `.git/hooks/pre-commit`:

```bash
#!/bin/bash
script tests/ --test
```

### Continuous Integration

Example GitHub Actions workflow:

```yaml
name: Tests
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Run tests
        run: script tests/ --test --format junit > test-results.xml
      - name: Publish test results
        uses: actions/upload-artifact@v4
        with:
          name: test-results
          path: test-results.xml
```

## Troubleshooting

### Common Issues

1. **Tests not discovered**: Ensure functions have the `@test` attribute
2. **Assertion not found**: Import assertions with `import * as assert from "std::testing::assertions"`
3. **Timeout errors**: Increase timeout with `@test(timeout = 10000)`

### Debugging Tests

Use print statements or a debugger:

```script
@test
fn test_complex_logic() {
    let input = prepare_data()
    print("Input data:", input)  // Debug output
    
    let result = process(input)
    print("Result:", result)      // Debug output
    
    assert_eq(result.status, "success")
}
```

## Summary

The Script testing framework provides:
- Simple, attribute-based test definition
- Comprehensive assertion library
- Flexible test organization
- Multiple output formats
- Integration with CI/CD pipelines

Start writing tests today to ensure your Script code is reliable and maintainable!