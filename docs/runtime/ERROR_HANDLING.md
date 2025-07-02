# Error Handling Best Practices

Script provides a comprehensive error handling system based on Result and Option types, along with panic recovery mechanisms. This guide covers best practices for writing robust, error-safe Script code and handling exceptional conditions gracefully.

## Table of Contents

1. [Error Handling Philosophy](#error-handling-philosophy)
2. [Result Type Usage](#result-type-usage)
3. [Option Type Usage](#option-type-usage)
4. [Panic Handling](#panic-handling)
5. [Error Propagation](#error-propagation)
6. [Custom Error Types](#custom-error-types)
7. [Error Recovery Strategies](#error-recovery-strategies)
8. [Debugging Error Conditions](#debugging-error-conditions)
9. [Performance Considerations](#performance-considerations)
10. [Best Practices by Use Case](#best-practices-by-use-case)

## Error Handling Philosophy

Script follows a "fail-safe" approach to error handling:

### Core Principles

1. **Explicit Error Handling**: Errors are part of the type system and must be handled explicitly
2. **Recoverable vs Unrecoverable**: Distinguish between errors you can recover from and those that indicate programmer bugs
3. **Fail Fast**: Detect errors early and handle them close to where they occur
4. **Graceful Degradation**: Provide fallback behavior when possible
5. **User-Friendly Messages**: Present errors in a way users can understand and act upon

### Error Categories

**Recoverable Errors (use Result):**
- File not found
- Network timeouts
- Invalid user input
- Parse failures
- Resource exhaustion

**Logic Errors (use assertions/panics):**
- Array bounds violations
- Null pointer dereferences
- Contract violations
- Programmer mistakes

**Missing Values (use Option):**
- Optional configuration
- Search results
- Nullable fields

## Result Type Usage

The `Result<T, E>` type represents operations that can succeed with value `T` or fail with error `E`.

### Basic Result Handling

```script
// Function that can fail
fn divide(a: f32, b: f32) -> Result<f32, string> {
    if b == 0.0 {
        Result::err("Division by zero")
    } else {
        Result::ok(a / b)
    }
}

// Handling the result
fn calculate() {
    let result = divide(10.0, 2.0);
    match result {
        Ok(value) => println("Result: " + value),
        Err(error) => eprintln("Error: " + error)
    }
}
```

### The ? Operator Pattern

While Script doesn't currently have the `?` operator, you can implement similar patterns:

```script
// Manual error propagation
fn complex_calculation() -> Result<f32, string> {
    let step1 = divide(10.0, 2.0);
    if is_err(step1) {
        return step1;
    }
    
    let step2 = divide(result_unwrap(step1), 3.0);
    if is_err(step2) {
        return step2;
    }
    
    let final_result = result_unwrap(step2) * 2.0;
    Result::ok(final_result)
}

// Helper function for cleaner error propagation
fn try_unwrap<T>(result: Result<T, string>) -> T {
    if is_ok(result) {
        result_unwrap(result)
    } else {
        // Re-throw the error (in a real implementation)
        panic!("Error: " + unwrap_err(result))
    }
}
```

### Result Combinators

Implement common patterns for working with Results:

```script
// Map pattern: transform success value
fn map_result<T, U, E>(result: Result<T, E>, transform: fn(T) -> U) -> Result<U, E> {
    if is_ok(result) {
        let value = result_unwrap(result);
        Result::ok(transform(value))
    } else {
        Result::err(unwrap_err(result))
    }
}

// Usage
let doubled = map_result(divide(10.0, 2.0), |x| x * 2.0);

// AndThen pattern: chain operations that can fail
fn and_then_result<T, U, E>(result: Result<T, E>, next: fn(T) -> Result<U, E>) -> Result<U, E> {
    if is_ok(result) {
        let value = result_unwrap(result);
        next(value)
    } else {
        Result::err(unwrap_err(result))
    }
}

// Chain operations
fn process_file(filename: string) -> Result<string, string> {
    and_then_result(
        read_file(filename),
        |contents| parse_data(contents)
    )
}
```

### Result Best Practices

**DO:**
```script
// Return specific error messages
fn validate_age(age: i32) -> Result<i32, string> {
    if age < 0 {
        Result::err("Age cannot be negative")
    } else if age > 150 {
        Result::err("Age seems unrealistic")
    } else {
        Result::ok(age)
    }
}

// Handle all error cases
fn process_user_input(input: string) -> string {
    match parse_number(input) {
        Ok(number) => "Valid number: " + number,
        Err(error) => {
            eprintln("Parse error: " + error);
            "Please enter a valid number"
        }
    }
}

// Provide meaningful context
fn load_config() -> Result<Config, string> {
    match read_file("config.json") {
        Ok(contents) => parse_config(contents),
        Err(error) => Result::err("Failed to load config file: " + error)
    }
}
```

**DON'T:**
```script
// Don't ignore errors
let result = risky_operation();
let value = result_unwrap(result);  // May panic!

// Don't use generic error messages
fn bad_function() -> Result<string, string> {
    Result::err("Something went wrong")  // Not helpful
}

// Don't catch and ignore
match risky_operation() {
    Ok(value) => process(value),
    Err(_) => {}  // Silently ignoring errors
}
```

## Option Type Usage

The `Option<T>` type represents values that may or may not be present.

### Basic Option Handling

```script
// Function that may not find a value
fn find_user(id: i32) -> Option<User> {
    if user_exists(id) {
        Option::some(get_user(id))
    } else {
        Option::none()
    }
}

// Safe handling
fn display_user(id: i32) {
    let user_opt = find_user(id);
    if is_some(user_opt) {
        let user = option_unwrap(user_opt);
        println("User: " + user.name);
    } else {
        println("User not found");
    }
}
```

### Option Patterns

```script
// Default values
fn get_config_value(key: string) -> string {
    let value_opt = config_get(key);
    if is_some(value_opt) {
        option_unwrap(value_opt)
    } else {
        "default_value"  // Provide sensible default
    }
}

// Chaining operations
fn get_user_email(user_id: i32) -> Option<string> {
    let user_opt = find_user(user_id);
    if is_some(user_opt) {
        let user = option_unwrap(user_opt);
        user.email  // Assume this might be None
    } else {
        Option::none()
    }
}

// Filter pattern
fn find_adult_user(users: Vec<User>) -> Option<User> {
    for user in users {
        if user.age >= 18 {
            return Option::some(user);
        }
    }
    Option::none()
}
```

### Option Best Practices

**DO:**
```script
// Check before unwrapping
if is_some(optional_value) {
    let value = option_unwrap(optional_value);
    use_value(value);
}

// Provide alternatives
fn get_display_name(user: User) -> string {
    if is_some(user.display_name) {
        option_unwrap(user.display_name)
    } else {
        user.username  // Fallback to username
    }
}

// Document when functions return None
fn find_first_match(items: Vec<string>, pattern: string) -> Option<string> {
    // Returns None if no items match the pattern
    for item in items {
        if contains(item, pattern) {
            return Option::some(item);
        }
    }
    Option::none()
}
```

**DON'T:**
```script
// Don't unwrap without checking
let value = option_unwrap(maybe_value);  // May panic!

// Don't return meaningless Options
fn get_username() -> Option<string> {
    Option::some("")  // Empty string is not "None"
}

// Don't use Option for errors
fn divide(a: f32, b: f32) -> Option<f32> {
    if b == 0.0 {
        Option::none()  // Should be Result::err with error message
    } else {
        Option::some(a / b)
    }
}
```

## Panic Handling

Panics are for unrecoverable errors that indicate programming bugs.

### When to Use Panics

**Appropriate for panics:**
- Array bounds violations
- Invalid function arguments that indicate programmer error
- Broken invariants
- Failed assertions
- Resource corruption

**Use Result/Option instead:**
- User input validation
- File system operations
- Network operations
- Parsing external data

### Assertion Patterns

```script
// Validate function preconditions
fn calculate_square_root(x: f32) -> f32 {
    if x < 0.0 {
        panic!("Cannot calculate square root of negative number: " + x);
    }
    sqrt(x)
}

// Validate array access
fn get_item(array: Vec<string>, index: i32) -> string {
    let len = vec_len(array);
    if index < 0 || index >= len {
        panic!("Index out of bounds: " + index + " for array of length " + len);
    }
    vec_get(array, index).unwrap()  // Safe after bounds check
}

// Validate object state
fn withdraw(account: Account, amount: f32) -> Result<Account, string> {
    if amount < 0.0 {
        panic!("Withdraw amount cannot be negative");  // Programming error
    }
    
    if account.balance < amount {
        Result::err("Insufficient funds")  // Business logic error
    } else {
        account.balance = account.balance - amount;
        Result::ok(account)
    }
}
```

### Panic Recovery

```script
// Use runtime's protected execution for panic recovery
fn safe_operation() -> Result<string, string> {
    let runtime = script::runtime::runtime()?;
    
    runtime.execute_protected(|| {
        // Code that might panic
        risky_calculation()
    })
}

// Handle panics in critical sections
fn process_user_request(request: string) -> string {
    match safe_operation() {
        Ok(result) => result,
        Err(panic_msg) => {
            eprintln("Operation failed with panic: " + panic_msg);
            "Request could not be processed"
        }
    }
}
```

## Error Propagation

### Manual Propagation

```script
// Propagate errors up the call stack
fn load_and_process_file(filename: string) -> Result<ProcessedData, string> {
    // Load file
    let contents = read_file(filename);
    if is_err(contents) {
        return Result::err("Failed to read file: " + unwrap_err(contents));
    }
    
    // Parse data
    let data = parse_data(result_unwrap(contents));
    if is_err(data) {
        return Result::err("Failed to parse data: " + unwrap_err(data));
    }
    
    // Process data
    let processed = process_data(result_unwrap(data));
    if is_err(processed) {
        return Result::err("Failed to process data: " + unwrap_err(processed));
    }
    
    Result::ok(result_unwrap(processed))
}
```

### Error Context

```script
// Add context to errors as they propagate
fn high_level_operation() -> Result<string, string> {
    match low_level_operation() {
        Ok(value) => Result::ok(value),
        Err(error) => Result::err("High-level operation failed: " + error)
    }
}

// Chain context information
fn process_file_with_context(filename: string) -> Result<Data, string> {
    match read_file(filename) {
        Ok(contents) => {
            match parse_json(contents) {
                Ok(data) => Result::ok(data),
                Err(parse_error) => Result::err(
                    "Failed to parse " + filename + ": " + parse_error
                )
            }
        },
        Err(io_error) => Result::err(
            "Failed to read " + filename + ": " + io_error
        )
    }
}
```

## Custom Error Types

### Structured Error Information

```script
// Define error types using objects
fn create_error(error_type: string, message: string, code: i32) -> Object {
    ErrorInfo {
        error_type: error_type,
        message: message,
        code: code,
        timestamp: time_now()
    }
}

// Use structured errors
fn validate_input(input: string) -> Result<ParsedInput, Object> {
    if string_len(input) == 0 {
        return Result::err(create_error(
            "ValidationError",
            "Input cannot be empty",
            400
        ));
    }
    
    if string_len(input) > 100 {
        return Result::err(create_error(
            "ValidationError", 
            "Input too long (max 100 characters)",
            400
        ));
    }
    
    // Parse input...
    Result::ok(parsed_input)
}
```

### Error Classification

```script
// Classify errors by type
fn is_retryable_error(error: Object) -> bool {
    let error_type = error.error_type;
    error_type == "NetworkError" || 
    error_type == "TimeoutError" ||
    error_type == "RateLimitError"
}

fn is_user_error(error: Object) -> bool {
    let error_type = error.error_type;
    error_type == "ValidationError" ||
    error_type == "AuthenticationError" ||
    error_type == "PermissionError"
}

// Handle errors based on classification
fn handle_operation_error(error: Object) {
    if is_retryable_error(error) {
        eprintln("Retryable error: " + error.message);
        schedule_retry();
    } else if is_user_error(error) {
        eprintln("User error: " + error.message);
        show_user_message(error.message);
    } else {
        eprintln("System error: " + error.message);
        log_error(error);
    }
}
```

## Error Recovery Strategies

### Fallback Mechanisms

```script
// Fallback to default values
fn load_config_with_fallback() -> Config {
    match read_file("config.json") {
        Ok(contents) => {
            match parse_config(contents) {
                Ok(config) => config,
                Err(parse_error) => {
                    eprintln("Config parse error: " + parse_error);
                    get_default_config()
                }
            }
        },
        Err(io_error) => {
            eprintln("Config read error: " + io_error);
            get_default_config()
        }
    }
}

// Multiple fallback sources
fn get_user_avatar(user_id: i32) -> string {
    // Try custom avatar
    match load_custom_avatar(user_id) {
        Ok(avatar_url) => avatar_url,
        Err(_) => {
            // Try gravatar
            match load_gravatar(user_id) {
                Ok(gravatar_url) => gravatar_url,
                Err(_) => {
                    // Use default avatar
                    "default_avatar.png"
                }
            }
        }
    }
}
```

### Retry Logic

```script
// Simple retry with exponential backoff
fn retry_operation<T>(operation: fn() -> Result<T, string>, max_attempts: i32) -> Result<T, string> {
    let mut attempts = 0;
    let mut delay = 1.0;  // Start with 1 second
    
    while attempts < max_attempts {
        match operation() {
            Ok(result) => return Result::ok(result),
            Err(error) => {
                attempts = attempts + 1;
                if attempts >= max_attempts {
                    return Result::err("Max retries exceeded. Last error: " + error);
                }
                
                eprintln("Attempt " + attempts + " failed: " + error + ". Retrying in " + delay + "s");
                sleep(delay);
                delay = delay * 2.0;  // Exponential backoff
            }
        }
    }
    
    Result::err("Retry loop exited unexpectedly")
}

// Usage
fn reliable_network_call() -> Result<string, string> {
    retry_operation(|| make_network_request(), 3)
}
```

### Circuit Breaker Pattern

```script
// Circuit breaker for failing services
struct CircuitBreaker {
    failure_count: i32,
    last_failure_time: f32,
    failure_threshold: i32,
    recovery_timeout: f32,
    state: string  // "closed", "open", "half-open"
}

fn create_circuit_breaker(failure_threshold: i32, recovery_timeout: f32) -> CircuitBreaker {
    CircuitBreaker {
        failure_count: 0,
        last_failure_time: 0.0,
        failure_threshold: failure_threshold,
        recovery_timeout: recovery_timeout,
        state: "closed"
    }
}

fn call_with_breaker(breaker: CircuitBreaker, operation: fn() -> Result<string, string>) -> Result<string, string> {
    let current_time = time_now();
    
    // Check if circuit should transition from open to half-open
    if breaker.state == "open" && 
       current_time - breaker.last_failure_time > breaker.recovery_timeout {
        breaker.state = "half-open";
        breaker.failure_count = 0;
    }
    
    // Reject calls when circuit is open
    if breaker.state == "open" {
        return Result::err("Circuit breaker is open");
    }
    
    // Attempt the operation
    match operation() {
        Ok(result) => {
            // Success: reset circuit breaker
            breaker.failure_count = 0;
            breaker.state = "closed";
            Result::ok(result)
        },
        Err(error) => {
            // Failure: increment counter and possibly open circuit
            breaker.failure_count = breaker.failure_count + 1;
            breaker.last_failure_time = current_time;
            
            if breaker.failure_count >= breaker.failure_threshold {
                breaker.state = "open";
                eprintln("Circuit breaker opened due to repeated failures");
            }
            
            Result::err(error)
        }
    }
}
```

## Debugging Error Conditions

### Error Logging

```script
// Structured error logging
fn log_error(error: Object, context: string) {
    let timestamp = time_now();
    let log_entry = "ERROR [" + timestamp + "] " + context + ": " + error.message;
    
    // Log to stderr
    eprintln(log_entry);
    
    // Also log to file if possible
    let log_result = write_file("error.log", log_entry + "\n");
    if is_err(log_result) {
        eprintln("Failed to write to error log: " + unwrap_err(log_result));
    }
}

// Error with stack trace context
fn log_error_with_trace(error: string, operation: string) {
    eprintln("Error in " + operation + ": " + error);
    
    // Get current stack trace if available
    if let Some(trace) = get_stack_trace() {
        eprintln("Stack trace:");
        eprintln(trace);
    }
}
```

### Debug Helpers

```script
// Debug wrapper for operations
fn debug_operation<T>(operation: fn() -> Result<T, string>, operation_name: string) -> Result<T, string> {
    eprintln("Starting: " + operation_name);
    let start_time = time_now();
    
    let result = operation();
    let end_time = time_now();
    let duration = end_time - start_time;
    
    match result {
        Ok(value) => {
            eprintln("Completed: " + operation_name + " in " + duration + "s");
            Result::ok(value)
        },
        Err(error) => {
            eprintln("Failed: " + operation_name + " after " + duration + "s - " + error);
            Result::err(error)
        }
    }
}

// Usage
fn traced_file_operation() -> Result<string, string> {
    debug_operation(
        || read_file("important.txt"),
        "read_important_file"
    )
}
```

### Error Aggregation

```script
// Collect multiple errors
struct ErrorCollector {
    errors: Vec<string>,
    warnings: Vec<string>
}

fn create_error_collector() -> ErrorCollector {
    ErrorCollector {
        errors: Vec::new(),
        warnings: Vec::new()
    }
}

fn add_error(collector: ErrorCollector, error: string) {
    vec_push(collector.errors, error);
}

fn add_warning(collector: ErrorCollector, warning: string) {
    vec_push(collector.warnings, warning);
}

fn validate_data_thoroughly(data: Data) -> Result<Data, ErrorCollector> {
    let collector = create_error_collector();
    
    // Perform multiple validations
    if data.name == "" {
        add_error(collector, "Name is required");
    }
    
    if data.email == "" {
        add_error(collector, "Email is required");
    } else if !is_valid_email(data.email) {
        add_error(collector, "Email format is invalid");
    }
    
    if data.age < 0 {
        add_error(collector, "Age cannot be negative");
    } else if data.age > 120 {
        add_warning(collector, "Age seems unusually high");
    }
    
    // Return errors if any found
    if vec_len(collector.errors) > 0 {
        Result::err(collector)
    } else {
        // Log warnings but continue
        for warning in collector.warnings {
            eprintln("Warning: " + warning);
        }
        Result::ok(data)
    }
}
```

## Performance Considerations

### Error Handling Performance

**Fast Paths:**
- Option checking (`is_some`, `is_none`) is O(1)
- Result checking (`is_ok`, `is_err`) is O(1)
- Error propagation has minimal overhead

**Expensive Operations:**
- String concatenation for error messages
- Stack trace capture during panics
- Complex error object creation

### Optimization Strategies

```script
// Lazy error message construction
fn expensive_validation(data: Data) -> Result<Data, string> {
    if !is_valid_data(data) {
        // Only construct expensive error message if needed
        let detailed_error = build_detailed_error_message(data);
        Result::err(detailed_error)
    } else {
        Result::ok(data)
    }
}

// Pre-allocate error messages for common cases
let COMMON_ERRORS = {
    "invalid_input": "Input validation failed",
    "network_timeout": "Network operation timed out",
    "permission_denied": "Access denied"
};

fn get_error_message(error_code: string) -> string {
    match hashmap_get(COMMON_ERRORS, error_code) {
        Some(message) => option_unwrap(message),
        None => "Unknown error: " + error_code
    }
}

// Avoid error allocation in hot paths
fn fast_parsing(input: string) -> Option<i32> {
    // Return None instead of constructing error messages
    // in performance-critical code
    if string_len(input) == 0 {
        return Option::none();
    }
    
    // Parse and return Some(value) or None
    parse_number_simple(input)
}
```

## Best Practices by Use Case

### File Operations

```script
fn robust_file_operations() {
    // Always handle file errors
    match read_file("config.json") {
        Ok(contents) => {
            match parse_json(contents) {
                Ok(config) => use_config(config),
                Err(parse_error) => {
                    eprintln("Invalid config file: " + parse_error);
                    use_default_config();
                }
            }
        },
        Err(io_error) => {
            eprintln("Cannot read config file: " + io_error);
            use_default_config();
        }
    }
    
    // Provide informative error messages
    fn save_user_data(user: User) -> Result<unit, string> {
        let filename = "user_" + user.id + ".json";
        let json_data = serialize_user(user);
        
        match write_file(filename, json_data) {
            Ok(()) => Result::ok(()),
            Err(error) => Result::err(
                "Failed to save user " + user.id + " to " + filename + ": " + error
            )
        }
    }
}
```

### Network Operations

```script
fn handle_network_errors() {
    // Classify network errors
    match make_http_request("https://api.example.com/data") {
        Ok(response) => process_response(response),
        Err(error) => {
            if contains(error, "timeout") {
                eprintln("Request timed out - trying again later");
                schedule_retry();
            } else if contains(error, "404") {
                eprintln("Resource not found - using cached data");
                use_cached_data();
            } else if contains(error, "500") {
                eprintln("Server error - will retry");
                schedule_retry();
            } else {
                eprintln("Network error: " + error);
                use_fallback_data();
            }
        }
    }
}
```

### User Input Validation

```script
fn validate_user_input() {
    // Validate incrementally and provide specific feedback
    fn validate_email(email: string) -> Result<string, string> {
        if string_len(email) == 0 {
            return Result::err("Email address is required");
        }
        
        if !contains(email, "@") {
            return Result::err("Email must contain @ symbol");
        }
        
        if !contains(email, ".") {
            return Result::err("Email must contain a domain");
        }
        
        if string_len(email) > 254 {
            return Result::err("Email address is too long");
        }
        
        Result::ok(email)
    }
    
    // Sanitize input on error
    fn safe_get_user_input(prompt: string) -> string {
        print(prompt);
        match read_line() {
            Ok(input) => trim(input),
            Err(_) => {
                eprintln("Failed to read input - using default");
                ""
            }
        }
    }
}
```

### Game Development

```script
fn game_error_handling() {
    // Don't let errors crash the game
    fn safe_game_loop() {
        while game_running() {
            match update_game_state() {
                Ok(()) => {},
                Err(error) => {
                    eprintln("Game state error: " + error);
                    // Continue running but log the error
                }
            }
            
            match render_frame() {
                Ok(()) => {},
                Err(error) => {
                    eprintln("Render error: " + error);
                    // Skip this frame but continue
                }
            }
        }
    }
    
    // Handle resource loading failures gracefully
    fn load_game_assets() -> Result<Assets, string> {
        let mut assets = Assets::new();
        
        // Load critical assets (fail if missing)
        match load_texture("player.png") {
            Ok(texture) => assets.player_texture = texture,
            Err(error) => return Result::err("Failed to load player texture: " + error)
        }
        
        // Load optional assets (warn if missing)
        match load_texture("background.png") {
            Ok(texture) => assets.background_texture = texture,
            Err(error) => {
                eprintln("Warning: Failed to load background texture: " + error);
                assets.background_texture = create_default_background();
            }
        }
        
        Result::ok(assets)
    }
}
```

By following these error handling best practices, you can write Script programs that are robust, maintainable, and provide excellent user experiences even when things go wrong. Remember that good error handling is not just about preventing crashes—it's about creating software that degrades gracefully and helps users understand and resolve problems when they occur.