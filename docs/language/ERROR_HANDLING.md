# Error Handling in Script

This document provides a comprehensive guide to error handling in the Script programming language, covering the complete error handling system including Result and Option types, the error propagation operator (?), and advanced functional programming patterns.

## Table of Contents

1. [Philosophy and Design Principles](#philosophy-and-design-principles)
2. [Core Types: Result and Option](#core-types-result-and-option)
3. [Error Propagation Operator (?)](#error-propagation-operator)
4. [Basic Error Handling Patterns](#basic-error-handling-patterns)
5. [Advanced Methods](#advanced-methods)
6. [Functional Programming Patterns](#functional-programming-patterns)
7. [Custom Error Types](#custom-error-types)
8. [Performance Considerations](#performance-considerations)
9. [Best Practices](#best-practices)
10. [Migration Guide](#migration-guide)
11. [Integration with Async/Await](#integration-with-asyncawait)
12. [API Reference](#api-reference)

## Philosophy and Design Principles

Script's error handling system is built around the principle of **explicit error handling** without runtime panics. The design emphasizes:

- **Explicit over implicit**: Errors are part of the type system
- **Safety by default**: No hidden exceptions or runtime crashes
- **Composability**: Error handling operations can be chained and combined
- **Performance**: Zero-cost abstractions with compile-time optimization
- **Ergonomics**: Concise syntax for common error handling patterns

### Key Design Decisions

1. **No exceptions**: Script doesn't have exception handling with try/catch
2. **Explicit Result types**: Functions that can fail return `Result<T, E>`
3. **Null safety**: Use `Option<T>` instead of null pointers
4. **Composable operations**: Monadic patterns for chaining operations
5. **Early returns**: The `?` operator for convenient error propagation

## Core Types: Result and Option

### Result&lt;T, E&gt;

The `Result<T, E>` type represents either success (`Ok(T)`) or failure (`Err(E)`).

```script
// Function that can fail
fn divide(a: i32, b: i32) -> Result<i32, string> {
    if b == 0 {
        Err("Division by zero")
    } else {
        Ok(a / b)
    }
}

// Usage
match divide(10, 2) {
    Ok(result) => println("Result: {}", result),
    Err(error) => println("Error: {}", error),
}
```

### Option&lt;T&gt;

The `Option<T>` type represents either some value (`Some(T)`) or no value (`None`).

```script
// Function that might not find a value
fn find_user(id: i32) -> Option<User> {
    if id == 1 {
        Some(User { name: "Alice", id: 1 })
    } else {
        None
    }
}

// Usage
match find_user(1) {
    Some(user) => println("Found user: {}", user.name),
    None => println("User not found"),
}
```

## Error Propagation Operator (?)

The `?` operator provides a concise way to propagate errors up the call stack.

### Basic Usage

```script
fn process_data(input: string) -> Result<ProcessedData, string> {
    let parsed = parse_input(input)?;  // Returns early if parse_input fails
    let validated = validate_data(parsed)?;  // Returns early if validation fails
    let processed = transform_data(validated)?;  // Returns early if transformation fails
    Ok(processed)
}
```

### How It Works

The `?` operator:
1. Evaluates the expression
2. If it's `Ok(value)` or `Some(value)`, unwraps the value
3. If it's `Err(error)` or `None`, returns early with the error/None
4. Automatically converts error types if needed

### Type Requirements

The `?` operator can only be used in functions that return:
- `Result<T, E>` for Result propagation
- `Option<T>` for Option propagation

## Basic Error Handling Patterns

### Pattern Matching

```script
// Exhaustive pattern matching
match parse_number("42") {
    Ok(num) => println("Parsed: {}", num),
    Err(e) => println("Parse error: {}", e),
}

// Nested pattern matching
match get_user_data(user_id) {
    Ok(Some(data)) => process_data(data),
    Ok(None) => println("User has no data"),
    Err(e) => println("Error fetching user: {}", e),
}
```

### Unwrapping with Defaults

```script
// Provide default values
let value = parse_number("invalid").unwrap_or(0);
let name = get_username().unwrap_or("Anonymous");

// Compute default values
let expensive_default = expensive_computation().unwrap_or_else(|| {
    println("Computing fallback...");
    compute_fallback()
});
```

### Combining Multiple Operations

```script
// Sequential operations with error propagation
fn process_user_request(request: string) -> Result<Response, AppError> {
    let parsed = parse_request(request)?;
    let validated = validate_request(parsed)?;
    let user = authenticate_user(validated.user_id)?;
    let result = process_for_user(user, validated.action)?;
    Ok(format_response(result))
}
```

## Advanced Methods

### Flattening Nested Results

```script
// Flatten Result<Result<T, E>, E> to Result<T, E>
let nested: Result<Result<i32, string>, string> = Ok(Ok(42));
let flattened: Result<i32, string> = nested.flatten();

// Practical example
fn parse_and_validate(input: string) -> Result<i32, string> {
    let parse_result = Ok(parse_number(input));
    parse_result.flatten()
}
```

### Transposing Result and Option

```script
// Transpose Result<Option<T>, E> to Option<Result<T, E>>
let result_option: Result<Option<i32>, string> = Ok(Some(42));
let option_result: Option<Result<i32, string>> = result_option.transpose();

// Practical example
fn find_and_parse(data: [string], target: string) -> Result<Option<i32>, string> {
    let found = find_in_array(data, target);
    found.transpose()
}
```

### Inspecting Values

```script
// Debug values without consuming them
let result = parse_number("42")
    .inspect(|val| println("Parsed: {}", val))
    .inspect_err(|err| println("Error: {}", err));

// Chain multiple inspections
let processed = compute_something()
    .inspect(|val| log_debug("Computed: {}", val))
    .map(|val| val * 2)
    .inspect(|val| log_debug("Doubled: {}", val));
```

### Logical Operations

```script
// AND operation - returns first Err or second Result
let a = Ok(1);
let b = Ok(2);
let result = a.and(b);  // Returns Ok(2)

// OR operation - returns first Ok or second Result
let a = Err("error");
let b = Ok(42);
let result = a.or(b);  // Returns Ok(42)
```

## Functional Programming Patterns

### Chaining Operations

```script
// Functional pipeline
fn process_pipeline(input: string) -> Result<string, string> {
    parse_number(input)
        .and_then(|num| validate_positive(num))
        .and_then(|num| double_value(num))
        .map(|num| format!("Result: {}", num))
        .map_err(|e| format!("Pipeline error: {}", e))
}
```

### Transforming Collections

```script
// Map over collections with error handling
let numbers = ["1", "2", "invalid", "4"];
let results: [Result<i32, string>] = numbers.map(|s| parse_number(s));

// Filter successful results
let valid_numbers: [i32] = results
    .filter(|r| r.is_ok())
    .map(|r| r.unwrap());
```

### Collecting Results

```script
// Collect all results or fail on first error
fn parse_all(inputs: [string]) -> Result<[i32], string> {
    let mut results = [];
    for input in inputs {
        results.push(parse_number(input)?);
    }
    Ok(results)
}

// Collect with error accumulation
fn parse_with_errors(inputs: [string]) -> (Vec<i32>, Vec<string>) {
    let mut successes = [];
    let mut errors = [];
    
    for input in inputs {
        match parse_number(input) {
            Ok(val) => successes.push(val),
            Err(e) => errors.push(e),
        }
    }
    
    (successes, errors)
}
```

### Monadic Patterns

```script
// Option chaining
let result = get_user(user_id)
    .and_then(|user| get_profile(user.id))
    .and_then(|profile| get_settings(profile.id))
    .map(|settings| format!("Theme: {}", settings.theme));

// Result chaining with error transformation
let processed = load_config()
    .map_err(|e| format!("Config error: {}", e))
    .and_then(|config| validate_config(config))
    .map_err(|e| format!("Validation error: {}", e))
    .and_then(|config| apply_config(config))
    .map_err(|e| format!("Application error: {}", e));
```

## Custom Error Types

### Defining Error Enums

```script
enum ValidationError {
    EmptyInput,
    TooShort(i32),
    TooLong(i32),
    InvalidCharacters(string),
    OutOfRange(i32, i32, i32),  // min, max, actual
}

// Implementing display for errors
fn format_validation_error(error: ValidationError) -> string {
    match error {
        ValidationError::EmptyInput => "Input cannot be empty",
        ValidationError::TooShort(len) => format!("Input too short: {} characters", len),
        ValidationError::TooLong(len) => format!("Input too long: {} characters", len),
        ValidationError::InvalidCharacters(chars) => {
            format!("Invalid characters: {}", chars)
        },
        ValidationError::OutOfRange(min, max, actual) => {
            format!("Value {} not in range {}-{}", actual, min, max)
        },
    }
}
```

### Error Hierarchies

```script
// Top-level application error
enum AppError {
    Database(DatabaseError),
    Network(NetworkError),
    Validation(ValidationError),
    Authentication(AuthError),
}

// Convert between error types
fn convert_db_error(db_error: DatabaseError) -> AppError {
    AppError::Database(db_error)
}

// Using error conversion in functions
fn process_request(request: Request) -> Result<Response, AppError> {
    let user = authenticate(request.token)
        .map_err(|e| AppError::Authentication(e))?;
    
    let data = fetch_data(user.id)
        .map_err(|e| AppError::Database(e))?;
    
    let validated = validate_data(data)
        .map_err(|e| AppError::Validation(e))?;
    
    Ok(create_response(validated))
}
```

## Performance Considerations

### Zero-Cost Abstractions

Script's error handling is designed to have zero runtime cost:

```script
// This code...
let result = parse_number("42")?;

// Compiles to the same machine code as:
let result = match parse_number("42") {
    Ok(val) => val,
    Err(e) => return Err(e),
};
```

### Memory Layout

- `Result<T, E>` is stored as a tagged union with no extra indirection
- `Option<T>` uses null pointer optimization where possible
- Error propagation doesn't allocate memory

### Optimization Tips

1. **Avoid excessive unwrapping**: Use `?` operator instead of `unwrap()`
2. **Prefer early returns**: Use `?` to fail fast
3. **Batch error handling**: Collect multiple errors when appropriate
4. **Use `expect()` for development**: Provides better error messages than `unwrap()`

```script
// Efficient error handling
fn process_batch(items: [string]) -> Result<[ProcessedItem], string> {
    let mut results = Vec::with_capacity(items.len());
    
    for item in items {
        results.push(process_item(item)?);
    }
    
    Ok(results)
}
```

## Best Practices

### 1. Use Descriptive Error Messages

```script
// Good
fn validate_email(email: string) -> Result<Email, string> {
    if email.is_empty() {
        return Err("Email address cannot be empty");
    }
    
    if !email.contains("@") {
        return Err("Email address must contain @ symbol");
    }
    
    Ok(Email::new(email))
}

// Bad
fn validate_email(email: string) -> Result<Email, string> {
    if email.is_empty() || !email.contains("@") {
        return Err("Invalid email");
    }
    
    Ok(Email::new(email))
}
```

### 2. Fail Fast, Recover Gracefully

```script
// Fail fast in validation
fn validate_user_input(input: UserInput) -> Result<ValidatedInput, ValidationError> {
    validate_username(input.username)?;
    validate_email(input.email)?;
    validate_password(input.password)?;
    Ok(ValidatedInput::new(input))
}

// Recover gracefully in application logic
fn handle_user_request(request: Request) -> Response {
    match process_request(request) {
        Ok(response) => response,
        Err(AppError::Validation(e)) => {
            Response::bad_request(format!("Validation error: {}", e))
        },
        Err(AppError::Authentication(e)) => {
            Response::unauthorized(format!("Auth error: {}", e))
        },
        Err(e) => {
            log_error("Internal error: {}", e);
            Response::internal_error("Something went wrong")
        },
    }
}
```

### 3. Use Appropriate Error Types

```script
// Use Option for "not found" scenarios
fn find_user_by_id(id: i32) -> Option<User> { /* ... */ }

// Use Result for operations that can fail
fn save_user(user: User) -> Result<(), DatabaseError> { /* ... */ }

// Use custom error types for domain-specific errors
fn process_payment(payment: Payment) -> Result<Receipt, PaymentError> { /* ... */ }
```

### 4. Document Error Conditions

```script
/// Parse a configuration file
/// 
/// # Errors
/// 
/// Returns `ConfigError::FileNotFound` if the file doesn't exist
/// Returns `ConfigError::InvalidFormat` if the file is malformed
/// Returns `ConfigError::ValidationFailed` if the config is invalid
fn parse_config(path: string) -> Result<Config, ConfigError> {
    // Implementation
}
```

## Migration Guide

### From Panic-Based Code

```script
// Old panic-based code
fn old_parse_number(s: string) -> i32 {
    if s.is_empty() {
        panic!("Cannot parse empty string");
    }
    // ... parsing logic
    42  // placeholder
}

// New Result-based code
fn parse_number(s: string) -> Result<i32, string> {
    if s.is_empty() {
        return Err("Cannot parse empty string");
    }
    // ... parsing logic
    Ok(42)  // placeholder
}
```

### From Null-Based Code

```script
// Old null-based code
fn old_find_user(id: i32) -> User? {
    if id == 1 {
        User { name: "Alice", id: 1 }
    } else {
        null
    }
}

// New Option-based code
fn find_user(id: i32) -> Option<User> {
    if id == 1 {
        Some(User { name: "Alice", id: 1 })
    } else {
        None
    }
}
```

### API Migration Strategy

1. **Phase 1**: Add Result-based versions alongside existing functions
2. **Phase 2**: Deprecate old functions with warnings
3. **Phase 3**: Remove old functions in next major version

```script
// Phase 1: Dual APIs
fn parse_number_unsafe(s: string) -> i32 { /* old implementation */ }
fn parse_number(s: string) -> Result<i32, string> { /* new implementation */ }

// Phase 2: Deprecation
#[deprecated("Use parse_number() instead")]
fn parse_number_unsafe(s: string) -> i32 { /* old implementation */ }

// Phase 3: Remove old function
// fn parse_number_unsafe is removed
```

## Integration with Async/Await

Error handling works seamlessly with async functions:

```script
// Async function returning Result
async fn fetch_user_data(user_id: i32) -> Result<UserData, ApiError> {
    let user = fetch_user(user_id).await?;
    let profile = fetch_profile(user.id).await?;
    let settings = fetch_settings(profile.id).await?;
    
    Ok(UserData { user, profile, settings })
}

// Using async error handling
async fn handle_request(request: Request) -> Result<Response, AppError> {
    let user_data = fetch_user_data(request.user_id).await
        .map_err(|e| AppError::Api(e))?;
    
    let processed = process_user_data(user_data).await?;
    Ok(create_response(processed))
}
```

## API Reference

### Result&lt;T, E&gt; Methods

#### Construction
- `Ok(value)` - Create a success value
- `Err(error)` - Create an error value

#### Querying
- `is_ok()` - Returns true if Ok
- `is_err()` - Returns true if Err
- `get_ok()` - Returns Some(value) if Ok, None if Err
- `get_err()` - Returns Some(error) if Err, None if Ok

#### Transformation
- `map(f)` - Transform Ok value with function f
- `map_err(f)` - Transform Err value with function f
- `and_then(f)` - Chain another Result operation
- `or_else(f)` - Provide alternative Result if Err

#### Extraction
- `unwrap()` - Extract Ok value or panic
- `unwrap_or(default)` - Extract Ok value or return default
- `unwrap_or_else(f)` - Extract Ok value or compute default
- `expect(message)` - Extract Ok value or panic with message

#### Advanced
- `flatten()` - Flatten nested Results
- `transpose()` - Convert Result&lt;Option&lt;T&gt;, E&gt; to Option&lt;Result&lt;T, E&gt;&gt;
- `inspect(f)` - Inspect Ok value without consuming
- `inspect_err(f)` - Inspect Err value without consuming
- `and(other)` - Logical AND operation
- `or(other)` - Logical OR operation
- `collect()` - Collect into Result&lt;Vec&lt;T&gt;, E&gt;
- `fold(init, f)` - Fold with early termination
- `satisfies(predicate)` - Test if Ok value satisfies predicate

### Option&lt;T&gt; Methods

#### Construction
- `Some(value)` - Create a some value
- `None` - Create a none value

#### Querying
- `is_some()` - Returns true if Some
- `is_none()` - Returns true if None
- `unwrap()` - Extract Some value if present

#### Transformation
- `map(f)` - Transform Some value with function f
- `and_then(f)` - Chain another Option operation
- `or_else(f)` - Provide alternative Option if None

#### Extraction
- `unwrap()` - Extract Some value or panic
- `unwrap_or(default)` - Extract Some value or return default
- `unwrap_or_else(f)` - Extract Some value or compute default
- `expect(message)` - Extract Some value or panic with message

#### Advanced
- `flatten()` - Flatten nested Options
- `transpose()` - Convert Option&lt;Result&lt;T, E&gt;&gt; to Result&lt;Option&lt;T&gt;, E&gt;
- `inspect(f)` - Inspect Some value without consuming
- `zip(other)` - Combine two Options into tuple
- `copied()` - Copy the value if copyable
- `cloned()` - Clone the value
- `collect()` - Collect into Option&lt;Vec&lt;T&gt;&gt;
- `fold(init, f)` - Fold with early termination
- `satisfies(predicate)` - Test if Some value satisfies predicate

### Error Propagation Operator (?)

The `?` operator can be used with:
- `Result<T, E>` types - propagates errors
- `Option<T>` types - propagates None values

```script
// Usage examples
let value = some_result?;          // Propagates Err or unwraps Ok
let option_value = some_option?;   // Propagates None or unwraps Some
```

## Conclusion

Script's error handling system provides a robust, type-safe, and performant way to handle errors without runtime panics. By using Result and Option types with the error propagation operator, developers can write reliable code that explicitly handles all error conditions.

The system's functional programming patterns enable elegant composition of error-handling operations, while the zero-cost abstractions ensure that safety doesn't come at the expense of performance.

For more examples and patterns, see:
- [Basic Error Handling Examples](../examples/error_handling_demo.script)
- [Comprehensive Error Handling](../examples/error_handling_comprehensive.script)
- [Advanced Error Handling](../examples/error_handling_advanced.script)
- [Functional Error Handling](../examples/functional_error_handling.script)