# Script Language Testing Framework

## Quick Start

The Script language includes a built-in testing framework that makes it easy to write and run tests.

### Writing Your First Test

Create a file `my_test.script`:

```script
@test
fn test_addition() {
    assert_eq(2 + 2, 4)
}

@test
fn test_string() {
    let greeting = "Hello, World!"
    assert_contains(greeting, "World")
}
```

### Running Tests

```bash
# Run tests in a single file
script my_test.script --test

# Run all tests in a directory
script tests/ --test

# Run with different output formats
script my_test.script --test --format json
script my_test.script --test --format junit
```

## Key Features

### 🧪 Simple Test Definition
- Use `@test` attribute to mark test functions
- No boilerplate or complex setup required

### ✅ Rich Assertion Library
- `assert_eq`, `assert_ne` - Equality checks
- `assert_true`, `assert_false` - Boolean assertions
- `assert_gt`, `assert_lt` - Comparison assertions
- `assert_contains`, `assert_empty` - Collection assertions
- `assert_approx_eq` - Floating-point comparisons

### 🏷️ Test Attributes
- `@test(skip = "reason")` - Skip tests
- `@test(should_panic)` - Expect panics
- `@test(timeout = 1000)` - Set timeouts
- `@test(tag = "integration")` - Tag tests

### 📊 Multiple Output Formats
- **Console**: Human-readable colored output
- **JSON**: Machine-readable format
- **JUnit XML**: CI/CD integration

### ⚡ Parallel Execution
- Tests run in parallel by default
- Configurable thread pool
- Isolated test environments

### 🔧 Setup and Teardown
- `@setup` - Run before each test
- `@teardown` - Run after each test

## Examples

### Basic Assertions

```script
@test
fn test_basic_assertions() {
    // Equality
    assert_eq(1 + 1, 2)
    assert_ne(1, 2)
    
    // Comparisons
    assert_gt(5, 3)
    assert_le(3, 3)
    
    // Booleans
    assert_true(5 > 3)
    assert_false(2 > 10)
}
```

### Testing Collections

```script
@test
fn test_arrays() {
    let numbers = [1, 2, 3, 4, 5]
    
    assert_len(numbers, 5)
    assert_eq(numbers[0], 1)
    assert_contains([1, 2, 3], 2)
    assert_empty([])
}
```

### Async Tests

```script
@test
async fn test_async_operation() {
    let result = await fetch_data()
    assert_eq(result.status, "success")
}
```

### Expected Failures

```script
@test(should_panic = "division by zero")
fn test_division_by_zero() {
    let x = 10 / 0  // This should panic
}
```

### Skipping Tests

```script
@test(skip = "Feature not implemented")
fn test_future_feature() {
    // This test won't run
}
```

## Best Practices

1. **Clear Test Names**: Use descriptive names that explain what is being tested
2. **Independent Tests**: Each test should be self-contained
3. **Test Edge Cases**: Don't just test the happy path
4. **One Concept Per Test**: Keep tests focused
5. **Use Setup/Teardown**: For common initialization

## Integration

### VS Code Extension

The Script VS Code extension provides:
- Test discovery and running
- Inline test results
- Code coverage visualization

### CI/CD

Example GitHub Actions:

```yaml
- name: Run Script Tests
  run: script tests/ --test --format junit > results.xml
  
- name: Publish Test Results
  uses: actions/upload-artifact@v4
  with:
    name: test-results
    path: results.xml
```

## Troubleshooting

### Tests Not Found
- Ensure functions have `@test` attribute
- Check file has `.script` extension
- Verify syntax is correct

### Assertion Errors
- Import assertions: `import * as assert from "std::testing::assertions"`
- Check assertion function names
- Verify expected vs actual order

### Timeout Issues
- Increase timeout: `@test(timeout = 5000)`
- Check for infinite loops
- Consider async tests for I/O operations

## Learn More

- [Full Testing Documentation](development/TESTING_FRAMEWORK.md)
- [Example Tests](../examples/test_basics.script)
- [Advanced Examples](../examples/test_stdlib.script)

## Contributing

The testing framework is part of the Script language core. To contribute:

1. Fork the repository
2. Add tests for your changes
3. Ensure all tests pass
4. Submit a pull request

Happy testing! 🚀