# Script Syntax Guide

## Table of Contents

1. [Getting Started](#getting-started)
2. [Variables and Constants](#variables-and-constants)
3. [Basic Data Types](#basic-data-types)
4. [Operators](#operators)
5. [Control Flow](#control-flow)
6. [Functions](#functions)
7. [Arrays and Collections](#arrays-and-collections)
8. [Expression-Oriented Programming](#expression-oriented-programming)
9. [Error Handling](#error-handling)
10. [Best Practices](#best-practices)

## Getting Started

Script is an expression-oriented language where almost everything returns a value. This guide provides practical examples for writing Script code.

### Your First Script Program

```script
// hello.script
fn main() {
    print("Hello, Script! 🚀")
}
```

### Running Script Code

```bash
# Parse and display AST
cargo run examples/hello.script

# Interactive REPL
cargo run                    # Parse mode
cargo run -- --tokens       # Token mode
```

## Variables and Constants

### Variable Declaration

Variables are declared with the `let` keyword:

```script
// Basic variable declaration
let name = "Alice"
let age = 30
let height = 5.8

// With explicit type annotations
let count: i32 = 42
let pi: f32 = 3.14159
let is_ready: bool = true
```

### Type Inference

Script can infer types in most cases:

```script
let number = 42          // inferred as i32
let decimal = 3.14       // inferred as f32
let message = "hello"    // inferred as string
let flag = true          // inferred as bool
```

### Variable Scope

Variables have block scope:

```script
let outer = "I'm outside"

{
    let inner = "I'm inside"
    print(outer)     // ✅ Can access outer scope
    print(inner)     // ✅ Can access current scope
}

// print(inner)      // ❌ Error: inner not in scope
```

### Variable Shadowing

Variables can be shadowed in inner scopes:

```script
let x = 10
print(x)      // prints: 10

{
    let x = 20
    print(x)  // prints: 20
    
    {
        let x = 30
        print(x)  // prints: 30
    }
    
    print(x)  // prints: 20
}

print(x)      // prints: 10
```

### Variable Assignment

Variables can be reassigned after declaration:

```script
let counter = 0
counter = 1           // Reassign to new value
counter = counter + 1 // Use current value in assignment

// Array element assignment
let numbers = [1, 2, 3]
numbers[0] = 10      // Modify first element
```

## Basic Data Types

### Numbers

#### Integers

```script
let small = 42              // i32 (default integer type)
let big = 1_000_000        // underscores for readability
let hex = 0xFF             // hexadecimal (255 in decimal)
let binary = 0b1010        // binary (10 in decimal)
let octal = 0o777          // octal (511 in decimal)
```

#### Floating Point

```script
let pi = 3.14159           // f32 (default float type)  
let scientific = 1.23e-4   // scientific notation
let large = 1.5e10         // 15,000,000,000
```

### Strings

```script
let greeting = "Hello, World!"
let quote = "She said, \"Hello!\""  // escaped quotes
let unicode = "Unicode: 🌟 ⚡ 🚀"   // full Unicode support

// Multi-line strings
let poem = "Roses are red,
Violets are blue,
Script is awesome,
And so are you!"
```

### Booleans

```script
let is_valid = true
let is_complete = false
let result = !is_valid     // logical NOT
```

### Arrays

```script
// Array literals
let numbers = [1, 2, 3, 4, 5]
let names = ["Alice", "Bob", "Charlie"]
let mixed_types = [42, "hello", true]  // arrays can be mixed (future)

// Empty arrays (type annotation required)
let empty_numbers: [i32] = []
let empty_strings: [string] = []

// Accessing elements
let first = numbers[0]      // 1
let second = numbers[1]     // 2
```

## Operators

### Arithmetic Operators

```script
let a = 10
let b = 3

let sum = a + b         // 13
let diff = a - b        // 7
let product = a * b     // 30
let quotient = a / b    // 3 (integer division)
let remainder = a % b   // 1
```

### Comparison Operators

```script
let x = 10
let y = 20

let equal = x == y         // false
let not_equal = x != y     // true
let less = x < y           // true  
let greater = x > y        // false
let less_equal = x <= y    // true
let greater_equal = x >= y // false
```

### Logical Operators

```script
let a = true
let b = false

let and_result = a && b    // false
let or_result = a || b     // true
let not_result = !a        // false

// Short-circuit evaluation
let safe = x != 0 && (10 / x) > 1  // won't divide by zero
```

### Operator Precedence

From highest to lowest precedence:

```script
let result = 2 + 3 * 4      // 14 (not 20)
let complex = !true && false || true  // true

// Use parentheses for clarity
let clear = (2 + 3) * 4     // 20
let explicit = (!true && false) || true  // true
```

## Control Flow

### If Expressions

If expressions return values:

```script
// Basic if expression
let status = if age >= 18 { "adult" } else { "minor" }

// Multi-line if
let grade = if score >= 90 {
    "A"
} else if score >= 80 {
    "B"
} else if score >= 70 {
    "C"
} else {
    "F"
}

// If without else returns unit type ()
if should_print {
    print("Hello!")
}
```

### While Loops

```script
// Simple counting
let mut i = 0
while i < 5 {
    print(i)
    i = i + 1
}

// While with complex condition
let mut running = true
let mut counter = 0
while running && counter < 100 {
    counter = counter + 1
    if counter == 50 {
        running = false
    }
}
```

### For Loops

```script
// Iterate over array
let fruits = ["apple", "banana", "orange"]
for fruit in fruits {
    print("I like " + fruit)
}

// Iterate over numbers
for number in [1, 2, 3, 4, 5] {
    let squared = number * number
    print(number + " squared is " + squared)
}
```

## Functions

### Function Declaration

```script
// Basic function
fn greet(name: string) -> string {
    return "Hello, " + name
}

// Function with multiple parameters
fn add(a: i32, b: i32) -> i32 {
    return a + b
}

// Function with implicit return (last expression)
fn multiply(x: i32, y: i32) -> i32 {
    x * y  // no semicolon = return value
}
```

### Function Calls

```script
let greeting = greet("Alice")
let sum = add(10, 20)
let product = multiply(5, 6)

print(greeting)  // "Hello, Alice"
print(sum)       // 30
print(product)   // 30
```

### Functions as Values

Functions are first-class values:

```script
fn double(x: i32) -> i32 {
    x * 2
}

fn triple(x: i32) -> i32 {
    x * 3  
}

// Store function in variable
let operation = double
let result = operation(5)  // 10

// Pass function as parameter
fn apply_twice(f: (i32) -> i32, value: i32) -> i32 {
    f(f(value))
}

let doubled_twice = apply_twice(double, 3)  // 12
```

### Function Examples

```script
// Calculate factorial
fn factorial(n: i32) -> i32 {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}

// Check if number is even
fn is_even(n: i32) -> bool {
    n % 2 == 0
}

// Find maximum of two numbers
fn max(a: i32, b: i32) -> i32 {
    if a > b { a } else { b }
}
```

## Arrays and Collections

### Array Creation

```script
// Literal arrays
let numbers = [1, 2, 3, 4, 5]
let colors = ["red", "green", "blue"]

// Arrays with type annotations
let scores: [f32] = [85.5, 92.0, 78.5]
let flags: [bool] = [true, false, true]
```

### Array Access

```script
let fruits = ["apple", "banana", "cherry"]

// Index access
let first = fruits[0]    // "apple"
let last = fruits[2]     // "cherry"

// Array length (future feature)
// let count = fruits.length
```

### Array Processing

```script
// Process each element
let numbers = [1, 2, 3, 4, 5]
for num in numbers {
    let doubled = num * 2
    print("Double of " + num + " is " + doubled)
}

// Working with arrays in functions
fn sum_array(arr: [i32]) -> i32 {
    let mut total = 0
    for num in arr {
        total = total + num
    }
    total
}

let numbers = [10, 20, 30, 40]
let total = sum_array(numbers)  // 100
```

## Expression-Oriented Programming

Everything in Script is an expression that returns a value:

### Block Expressions

```script
let result = {
    let temp = expensive_calculation()
    let processed = temp * 2
    processed + 1  // final expression is returned
}
```

### Conditional Expressions

```script
let message = if user.is_admin {
    "Welcome, administrator!"
} else if user.is_member {
    "Welcome, member!"
} else {
    "Welcome, guest!"
}
```

### Nested Expressions

```script
let complex_result = {
    let base = if condition { 10 } else { 5 }
    let multiplier = {
        let temp = calculate_something()
        temp * 2
    }
    base * multiplier
}
```

### Expression Composition

```script
// Compose expressions naturally
let final_score = {
    let base_score = calculate_base_score()
    let bonus = if has_bonus { 10 } else { 0 }
    let penalty = if has_penalty { -5 } else { 0 }
    base_score + bonus + penalty
}
```

## Error Handling

### Current Error Handling

Script currently uses print statements for error reporting:

```script
fn divide(a: f32, b: f32) -> f32 {
    if b == 0.0 {
        print("Error: Division by zero")
        return 0.0  // fallback value
    }
    a / b
}
```

### Future: Result Types

Script will support Result types for better error handling:

```script
// Future syntax
fn safe_divide(a: f32, b: f32) -> Result<f32, string> {
    if b == 0.0 {
        Err("Division by zero")
    } else {
        Ok(a / b)
    }
}

// Usage
match safe_divide(10.0, 2.0) {
    Ok(result) => print("Result: " + result),
    Err(error) => print("Error: " + error)
}
```

## Best Practices

### Naming Conventions

```script
// Use snake_case for variables and functions
let user_name = "Alice"
let max_score = 100

fn calculate_total(items: [i32]) -> i32 {
    // ...
}

// Use PascalCase for types (future)
// struct UserAccount { ... }
```

### Type Annotations

```script
// Provide type annotations for function parameters
fn process_data(data: [string], threshold: i32) -> bool {
    // ...
}

// Use type annotations when type inference isn't clear
let empty_list: [string] = []
let result: f32 = complex_calculation()
```

### Expression Style

```script
// Prefer expressions over statements when possible
let status = if is_complete { "Done" } else { "In Progress" }

// Rather than:
let status = ""
if is_complete {
    status = "Done"
} else {
    status = "In Progress"
}
```

### Function Design

```script
// Keep functions small and focused
fn is_valid_email(email: string) -> bool {
    // Simple validation logic
    email.contains("@")  // simplified
}

// Compose small functions
fn process_user(email: string, age: i32) -> string {
    if !is_valid_email(email) {
        return "Invalid email"
    }
    
    if age < 13 {
        return "Too young"
    }
    
    "User accepted"
}
```

### Error Prevention

```script
// Check bounds before array access
fn get_safe(arr: [i32], index: i32) -> i32 {
    if index >= 0 && index < arr.length {
        arr[index]
    } else {
        0  // safe default
    }
}

// Validate inputs
fn calculate_percentage(part: f32, total: f32) -> f32 {
    if total <= 0.0 {
        return 0.0
    }
    (part / total) * 100.0
}
```

### Code Organization

```script
// Group related functionality
fn user_operations() {
    // User-related functions
    fn create_user(name: string, email: string) -> bool {
        // ...
    }
    
    fn validate_user(user: User) -> bool {
        // ...
    }
    
    fn update_user(user: User, new_data: UserData) -> User {
        // ...
    }
}
```

### Documentation

```script
// Use comments to explain complex logic
fn complex_algorithm(data: [f32]) -> f32 {
    // Apply weighted average with exponential decay
    let mut result = 0.0
    let mut weight = 1.0
    
    for value in data {
        result = result + (value * weight)
        weight = weight * 0.9  // decay factor
    }
    
    result
}
```

### Match Expressions

Pattern matching provides powerful control flow:

```script
let status_code = 404

let message = match status_code {
    200 => "OK",
    404 => "Not Found", 
    500 => "Internal Server Error",
    _ => "Unknown status"
}

// Match with guards
let score = 85
let grade = match score {
    s if s >= 90 => "A",
    s if s >= 80 => "B", 
    s if s >= 70 => "C",
    s if s >= 60 => "D",
    _ => "F"
}

// Array pattern matching
let coordinates = [10, 20]
let description = match coordinates {
    [] => "No coordinates",
    [x] => "1D point at " + x,
    [x, y] => "2D point at (" + x + ", " + y + ")",
    [x, y, z] => "3D point at (" + x + ", " + y + ", " + z + ")",
    _ => "Higher dimensional"
}
```

## Common Patterns

### Validation Pattern

```script
fn validate_input(input: string) -> bool {
    if input.length == 0 {
        return false
    }
    
    if input.length > 100 {
        return false
    }
    
    // Additional validation
    true
}

// Using pattern matching for validation
fn validate_age(age: i32) -> string {
    match age {
        a if a < 0 => "Age cannot be negative",
        a if a > 150 => "Age seems unrealistic",
        a if a < 18 => "Minor",
        a if a >= 65 => "Senior",
        _ => "Adult"
    }
}
```

### Builder Pattern (Future)

```script
// Future: Method chaining
let config = ConfigBuilder::new()
    .set_host("localhost")
    .set_port(8080)
    .set_timeout(30)
    .build()
```

### State Machine Pattern

```script
// Simple state machine using pattern matching
fn handle_game_input(state: string, input: string) -> string {
    match (state, input) {
        ("menu", "start") => "playing",
        ("menu", "quit") => "exit",
        ("playing", "pause") => "paused", 
        ("playing", "quit") => "menu",
        ("paused", "resume") => "playing",
        ("paused", "quit") => "menu",
        (current, _) => current  // Invalid input, stay in current state
    }
}

// Usage
let game_state = "menu"
game_state = handle_game_input(game_state, "start")  // "playing"
game_state = handle_game_input(game_state, "pause")  // "paused"
```

### Calculator Pattern

```script
fn calculate(operator: string, a: f32, b: f32) -> f32 {
    match operator {
        "+" => a + b,
        "-" => a - b,
        "*" => a * b,
        "/" => {
            if b == 0.0 {
                print("Error: Division by zero")
                0.0
            } else {
                a / b
            }
        },
        "%" => a % b,
        "^" => power(a, b),  // Assume power function exists
        _ => {
            print("Unknown operator: " + operator)
            0.0
        }
    }
}

// Usage
let result1 = calculate("+", 10.0, 5.0)   // 15.0
let result2 = calculate("/", 10.0, 3.0)   // 3.333...
let result3 = calculate("?", 1.0, 2.0)    // 0.0 (unknown operator)
```

### Array Processing Patterns

```script
// Sum all elements in array
fn sum_array(numbers: [i32]) -> i32 {
    let total = 0
    for num in numbers {
        total = total + num
    }
    total
}

// Find maximum element
fn find_max(numbers: [i32]) -> i32 {
    match numbers {
        [] => {
            print("Error: Empty array")
            0
        },
        [first] => first,
        [first, ..rest] => {  // Future syntax
            let max_rest = find_max(rest)
            if first > max_rest { first } else { max_rest }
        }
    }
}

// Current implementation without rest syntax
fn find_max_current(numbers: [i32]) -> i32 {
    if numbers.length == 0 {
        print("Error: Empty array")
        return 0
    }
    
    let max = numbers[0]
    for i in 1..numbers.length {
        if numbers[i] > max {
            max = numbers[i]
        }
    }
    max
}

// Filter even numbers (conceptual - returns count for now)
fn count_evens(numbers: [i32]) -> i32 {
    let count = 0
    for num in numbers {
        if num % 2 == 0 {
            count = count + 1
        }
    }
    count
}
```

### Configuration Parser Pattern

```script
fn parse_config_line(line: string) -> string {
    let parts = split_string(line, "=")  // Assume this function exists
    
    match parts {
        [] => "Error: Empty line",
        [key] => "Error: No value for key '" + key + "'",
        [key, value] => {
            let trimmed_key = trim(key)      // Assume trim function exists
            let trimmed_value = trim(value)
            
            match trimmed_key {
                "port" => validate_port(trimmed_value),
                "host" => validate_host(trimmed_value),
                "debug" => validate_boolean(trimmed_value),
                "timeout" => validate_timeout(trimmed_value),
                _ => "Unknown configuration key: " + trimmed_key
            }
        },
        _ => "Error: Invalid configuration format"
    }
}

fn validate_port(value: string) -> string {
    let port = parse_int(value)  // Assume this function exists
    match port {
        p if p >= 1 && p <= 65535 => "Port set to " + value,
        _ => "Error: Port must be between 1 and 65535"
    }
}

fn validate_boolean(value: string) -> string {
    match value {
        "true" | "yes" | "1" | "on" => "Boolean value: true",
        "false" | "no" | "0" | "off" => "Boolean value: false",
        _ => "Error: Boolean must be true/false, yes/no, 1/0, or on/off"
    }
}
```

### Error Handling Patterns

```script
// Simple error handling with return codes
fn safe_divide(a: f32, b: f32) -> [f32] {  // Empty array = error, [result] = success
    if b == 0.0 {
        print("Error: Division by zero")
        []
    } else {
        [a / b]
    }
}

// Using the safe divide function
fn calculate_average(total: f32, count: f32) -> string {
    let result = safe_divide(total, count)
    match result {
        [] => "Cannot calculate average: invalid input",
        [avg] => "Average: " + avg,
        _ => "Unexpected result format"
    }
}

// Chaining operations with error handling
fn complex_calculation(a: f32, b: f32, c: f32) -> string {
    let step1 = safe_divide(a, b)
    match step1 {
        [] => "Error in step 1",
        [result1] => {
            let step2 = safe_divide(result1, c)
            match step2 {
                [] => "Error in step 2",
                [final_result] => "Final result: " + final_result,
                _ => "Unexpected format in step 2"
            }
        },
        _ => "Unexpected format in step 1"
    }
}
```

### Command Line Parser Pattern

```script
fn parse_args(args: [string]) -> string {
    match args {
        [] => "Usage: program <command> [options]",
        
        ["help"] => show_help(),
        ["version"] => "Script Calculator v1.0",
        
        ["add", a, b] => {
            let num_a = parse_float(a)  // Assume this exists
            let num_b = parse_float(b)
            "Result: " + (num_a + num_b)
        },
        
        ["multiply", a, b] => {
            let num_a = parse_float(a)
            let num_b = parse_float(b)
            "Result: " + (num_a * num_b)
        },
        
        ["calc", expr] => evaluate_expression(expr),
        
        ["config", key, value] => set_config(key, value),
        ["config", key] => get_config(key),
        
        [command] => "Unknown command: " + command + ". Use 'help' for usage.",
        [command, ..rest] => "Command '" + command + "' called with " + rest.length + " arguments",
        
        _ => "Too many arguments. Use 'help' for usage."
    }
}

fn show_help() -> string {
    "Available commands:
  help                    - Show this help
  version                 - Show version
  add <a> <b>            - Add two numbers
  multiply <a> <b>       - Multiply two numbers  
  calc <expression>      - Evaluate expression
  config <key> [value]   - Get or set configuration"
}
```

### Game Development Patterns

```script
// Simple game entity state management
fn update_player(player_state: string, input: string, dt: f32) -> string {
    match (player_state, input) {
        ("idle", "move_left") => "walking_left",
        ("idle", "move_right") => "walking_right", 
        ("idle", "jump") => "jumping",
        ("idle", "attack") => "attacking",
        
        ("walking_left", "stop") => "idle",
        ("walking_left", "jump") => "jumping",
        ("walking_left", "attack") => "attacking",
        
        ("walking_right", "stop") => "idle", 
        ("walking_right", "jump") => "jumping",
        ("walking_right", "attack") => "attacking",
        
        ("jumping", "land") => "idle",
        ("attacking", "finish") => "idle",
        
        (current, _) => current  // Stay in current state for invalid transitions
    }
}

// Simple inventory management
fn handle_inventory_action(action: string, item: string, inventory: [string]) -> [string] {
    match action {
        "add" => add_to_inventory(inventory, item),
        "remove" => remove_from_inventory(inventory, item),
        "check" => {
            if contains_item(inventory, item) {
                print("You have: " + item)
            } else {
                print("You don't have: " + item)
            }
            inventory
        },
        _ => {
            print("Unknown inventory action: " + action)
            inventory
        }
    }
}

fn add_to_inventory(inventory: [string], item: string) -> [string] {
    // In a real implementation, this would add to the array
    print("Added " + item + " to inventory")
    inventory  // Return modified inventory
}
```

### Option Pattern (Future)

```script
// Future: Optional values
fn find_user(id: i32) -> Option<User> {
    // Search logic
    if found {
        Some(user)
    } else {
        None
    }
}

// Usage
match find_user(123) {
    Some(user) => print("Found: " + user.name),
    None => print("User not found")
}
```

---

This syntax guide covers the essential patterns for writing Script code. As the language evolves, new features like pattern matching, modules, and advanced error handling will be added to provide even more powerful programming constructs.