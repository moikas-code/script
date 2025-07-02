# Script Type System Guide

## Table of Contents

1. [Introduction](#introduction)
2. [Type Philosophy](#type-philosophy)
3. [Basic Types](#basic-types)
4. [Composite Types](#composite-types)
5. [Type Annotations](#type-annotations)
6. [Type Inference](#type-inference)
7. [Gradual Typing](#gradual-typing)
8. [Function Types](#function-types)
9. [Advanced Types](#advanced-types)
10. [Type Compatibility](#type-compatibility)
11. [Error Handling Types](#error-handling-types)
12. [Best Practices](#best-practices)

## Introduction

Script features a powerful type system that combines the safety of static typing with the flexibility of dynamic typing. The type system is designed to be:

- **Beginner-friendly**: Optional type annotations with helpful inference
- **Gradual**: Mix typed and untyped code seamlessly
- **Safe**: Catch errors at compile time when possible
- **Expressive**: Rich type system for complex applications

## Type Philosophy

Script's type system follows these core principles:

### 1. Optional Type Annotations
Types can be inferred in most contexts, but explicit annotations are supported for clarity and documentation:

```script
// Type inference - compiler figures out the types
let name = "Alice"          // inferred as string
let age = 25               // inferred as i32
let height = 5.8           // inferred as f32

// Explicit type annotations for clarity
let score: i32 = 100
let pi: f32 = 3.14159
let is_ready: bool = true
```

### 2. Gradual Typing
Code can be partially typed, allowing migration from dynamic to static typing:

```script
// Start with dynamic code
let user_data = get_external_data()  // type: unknown

// Add types gradually
let user_name: string = user_data.name
let user_age: i32 = user_data.age
```

### 3. Type Safety
The compiler prevents common type errors while allowing flexibility:

```script
let number = 42
let text = "hello"

// ✅ This works - same types
let sum = number + 10

// ❌ This fails at compile time - type mismatch
// let invalid = number + text
```

## Basic Types

### Integer Types

Script uses 32-bit signed integers as the default integer type:

```script
let count: i32 = 42
let negative: i32 = -100
let hex: i32 = 0xFF        // 255 in decimal
let binary: i32 = 0b1010   // 10 in decimal
let octal: i32 = 0o777     // 511 in decimal

// Underscores for readability
let large: i32 = 1_000_000
```

**Characteristics:**
- Range: -2,147,483,648 to 2,147,483,647
- Default integer type when no annotation is provided
- Overflow behavior: Debug builds panic, release builds wrap

### Floating Point Types

Script uses 32-bit floating point as the default float type:

```script
let pi: f32 = 3.14159
let scientific: f32 = 1.23e-4
let large: f32 = 1.5e10

// Type inference defaults to f32
let radius = 2.5           // inferred as f32
```

**Characteristics:**
- IEEE 754 single precision
- Range: approximately ±3.4 × 10^38
- Precision: about 7 decimal digits

### Boolean Type

Boolean values for logical operations:

```script
let is_valid: bool = true
let is_complete: bool = false

// Boolean expressions
let result = is_valid && !is_complete
let check = (age >= 18) || has_permission
```

### String Type

UTF-8 encoded strings with full Unicode support:

```script
let greeting: string = "Hello, World!"
let unicode: string = "Unicode: 🌟 ⚡ 🚀"
let multiline: string = "Line 1
Line 2
Line 3"

// Escape sequences
let escaped: string = "She said, \"Hello!\""
let special: string = "Tab:\t Newline:\n Backslash:\\"
```

**String Operations:**
```script
// String concatenation (current)
let full_name = first_name + " " + last_name

// Future string methods
// let length = text.length
// let upper = text.to_upper()
// let contains = text.contains("substring")
```

## Composite Types

### Array Types

Homogeneous collections with a specific element type:

```script
// Array literals with type inference
let numbers = [1, 2, 3, 4, 5]           // inferred as [i32]
let names = ["Alice", "Bob", "Charlie"]  // inferred as [string]
let scores = [95.5, 87.2, 92.1]        // inferred as [f32]

// Explicit type annotations
let empty_numbers: [i32] = []
let boolean_flags: [bool] = [true, false, true]

// Nested arrays
let matrix: [[i32]] = [[1, 2], [3, 4], [5, 6]]
```

**Array Operations:**
```script
// Element access
let first = numbers[0]      // Gets first element
let last = numbers[4]       // Gets fifth element

// Iteration
for number in numbers {
    print(number)
}

// Future operations
// let length = numbers.length
// numbers.push(6)
// let popped = numbers.pop()
```

### Function Types

Functions are first-class values with specific type signatures:

```script
// Function type syntax: (param_types...) -> return_type
fn add(a: i32, b: i32) -> i32 {
    a + b
}

// Function as variable
let operation: (i32, i32) -> i32 = add
let result = operation(10, 20)

// Higher-order functions
fn apply_twice(f: (i32) -> i32, x: i32) -> i32 {
    f(f(x))
}

fn double(n: i32) -> i32 {
    n * 2
}

let result = apply_twice(double, 5)  // 20
```

## Type Annotations

### Variable Annotations

```script
// Basic type annotations
let name: string = "Alice"
let age: i32 = 30
let height: f32 = 5.8
let is_student: bool = true

// Array type annotations
let scores: [f32] = [95.5, 87.2, 92.1]
let empty_list: [string] = []

// Function type annotations
let callback: (i32) -> bool = is_positive
```

### Function Parameter Annotations

All function parameters must have type annotations:

```script
// Required parameter types
fn calculate_area(width: f32, height: f32) -> f32 {
    width * height
}

// Return type can be inferred
fn process_data(items: [string]) {
    // return type inferred as [string]
    items
}

// Explicit return type for clarity
fn complex_calculation(input: f32) -> f32 {
    // complex logic here
    result
}
```

### Type Annotation Syntax

```script
// Named types
let count: i32 = 0
let message: string = "hello"

// Array types
let numbers: [i32] = [1, 2, 3]
let matrix: [[f32]] = [[1.0, 2.0], [3.0, 4.0]]

// Function types
let handler: (string, i32) -> bool = process_event
let mapper: (i32) -> string = number_to_string

// Future: User-defined types
// let player: Player = create_player()
// let result: Result<i32, string> = safe_operation()
```

## Type Inference

Script uses a sophisticated type inference engine based on the Hindley-Milner algorithm, allowing you to write code without explicit type annotations in many cases.

### Basic Inference

```script
// Types inferred from literals
let number = 42        // inferred as i32
let decimal = 3.14     // inferred as f32
let text = "hello"     // inferred as string
let flag = true        // inferred as bool

// Types inferred from expressions
let sum = 10 + 20      // inferred as i32
let average = sum / 2  // inferred as i32 (integer division)
let ratio = 3.0 / 2.0  // inferred as f32
```

### Context-Sensitive Inference

The inference engine uses context to determine types:

```script
// Function parameter types help infer variable types
fn process_numbers(numbers: [i32]) -> i32 {
    let total = 0      // inferred as i32 from return type
    for num in numbers {
        total = total + num  // num inferred as i32 from array type
    }
    total
}

// Return type constraints
fn get_name() -> string {
    let result = "Alice"  // inferred as string from return type
    result
}
```

### Inference with Arrays

```script
// Element types infer array type
let numbers = [1, 2, 3]           // [i32]
let mixed = [1, 2.0]              // Error: inconsistent types
let empty: [string] = []          // Explicit annotation needed for empty arrays

// Function calls help with inference
fn sum_array(arr: [i32]) -> i32 { /* ... */ }
let data = [10, 20, 30]           // inferred as [i32]
let total = sum_array(data)       // confirms data is [i32]
```

### Inference Limitations

Some cases require explicit type annotations:

```script
// Empty collections need annotations
let empty_numbers: [i32] = []
let empty_strings: [string] = []

// Ambiguous numeric literals might need clarification
let precise: f32 = 3.14159  // explicit for documentation
let integer: i32 = 42       // explicit when context is unclear
```

## Gradual Typing

Script's gradual typing system allows mixing typed and untyped code seamlessly.

### The Unknown Type

The `unknown` type represents values whose type is not known at compile time:

```script
// Functions returning unknown
let data = get_external_data()  // returns unknown

// Using unknown values
let processed = data + 42       // runtime type check
let name = data.name           // runtime property access
```

### Runtime Type Checks

When unknown types interact with typed values, runtime checks are inserted:

```script
let dynamic_value = get_unknown()  // type: unknown
let number: i32 = 42

// This generates a runtime type check
let result = dynamic_value + number
// Equivalent to:
// let result = (dynamic_value as i32) + number
```

### Gradual Migration

Start with dynamic code and add types incrementally:

```script
// Step 1: All dynamic
let user = get_user_data()
let name = user.name
let age = user.age

// Step 2: Add some types
let user = get_user_data()        // still unknown
let name: string = user.name      // explicit type
let age: i32 = user.age          // explicit type

// Step 3: Full typing (future)
// let user: User = get_user_data()
// let name = user.name             // inferred from User type
// let age = user.age              // inferred from User type
```

## Function Types

Functions in Script are first-class values with rich type signatures.

### Function Signatures

```script
// Basic function signature
fn greet(name: string) -> string {
    "Hello, " + name
}
// Type: (string) -> string

// Multiple parameters
fn calculate(x: i32, y: i32, z: f32) -> f32 {
    (x + y) as f32 * z
}
// Type: (i32, i32, f32) -> f32

// No parameters
fn get_random() -> i32 {
    42  // placeholder
}
// Type: () -> i32
```

### Function Variables

```script
// Store functions in variables
let greeter: (string) -> string = greet
let calculator: (i32, i32, f32) -> f32 = calculate

// Use function variables
let greeting = greeter("Alice")
let result = calculator(10, 20, 1.5)
```

### Higher-Order Functions

Functions that take or return other functions:

```script
// Function that takes a function parameter
fn apply_to_each(items: [i32], transform: (i32) -> i32) -> [i32] {
    let result: [i32] = []
    for item in items {
        result.push(transform(item))  // future syntax
    }
    result
}

// Function that returns a function
fn make_multiplier(factor: i32) -> (i32) -> i32 {
    fn multiply(x: i32) -> i32 {
        x * factor
    }
    multiply
}

// Usage
let double = make_multiplier(2)
let numbers = [1, 2, 3, 4, 5]
let doubled = apply_to_each(numbers, double)
```

### Function Type Compatibility

Function types are compatible based on their signatures:

```script
fn add(a: i32, b: i32) -> i32 { a + b }
fn subtract(x: i32, y: i32) -> i32 { x - y }

// Both functions have the same type signature
let operation: (i32, i32) -> i32 = add
operation = subtract  // This is valid

// Different signatures are not compatible
fn greet(name: string) -> string { "Hello " + name }
// operation = greet  // Error: incompatible types
```

## Advanced Types

### Type Variables (Future)

Type variables enable generic programming:

```script
// Future syntax for generic functions
fn identity<T>(value: T) -> T {
    value
}

// Usage with different types
let number = identity(42)        // T = i32
let text = identity("hello")     // T = string
let flag = identity(true)        // T = bool
```

### Result Types (Future)

For explicit error handling:

```script
// Future: Result type for error handling
fn divide(a: f32, b: f32) -> Result<f32, string> {
    if b == 0.0 {
        Err("Division by zero")
    } else {
        Ok(a / b)
    }
}

// Pattern matching on results
match divide(10.0, 2.0) {
    Ok(result) => print("Result: " + result),
    Err(error) => print("Error: " + error)
}
```

### Option Types (Future)

For nullable values:

```script
// Future: Option type for nullable values
fn find_user(id: i32) -> Option<string> {
    if id == 1 {
        Some("Alice")
    } else {
        None
    }
}

// Pattern matching on options
match find_user(1) {
    Some(name) => print("Found user: " + name),
    None => print("User not found")
}
```

## Type Compatibility

### Assignment Compatibility

```script
// Same types are always compatible
let x: i32 = 42
let y: i32 = x  // ✅ Compatible

// Different primitive types are not compatible
let a: i32 = 42
// let b: f32 = a  // ❌ Incompatible - would need explicit conversion

// Unknown type is compatible with everything
let unknown_value = get_dynamic_data()  // type: unknown
let typed_value: i32 = unknown_value    // ✅ Runtime check inserted
```

### Function Compatibility

```script
// Functions must have identical signatures for compatibility
fn add1(a: i32, b: i32) -> i32 { a + b }
fn add2(x: i32, y: i32) -> i32 { x + y }

let operation: (i32, i32) -> i32 = add1
operation = add2  // ✅ Same signature

// Different parameter names don't matter
fn multiply(first: i32, second: i32) -> i32 { first * second }
operation = multiply  // ✅ Same signature type
```

### Array Compatibility

```script
// Array types must have compatible element types
let numbers1: [i32] = [1, 2, 3]
let numbers2: [i32] = [4, 5, 6]
numbers1 = numbers2  // ✅ Same element type

// Different element types are incompatible
let floats: [f32] = [1.0, 2.0, 3.0]
// numbers1 = floats  // ❌ Incompatible element types
```

## Error Handling Types

### Current Error Handling

Script currently uses simple return values and print statements for error handling:

```script
fn safe_divide(a: f32, b: f32) -> f32 {
    if b == 0.0 {
        print("Error: Division by zero")
        return 0.0  // fallback value
    }
    a / b
}

// Caller must handle the case manually
let result = safe_divide(10.0, 0.0)
if result == 0.0 {
    print("Division failed")
}
```

### Future Result Types

Script will support Result types for explicit error handling:

```script
// Future: Explicit error handling with Result types
enum Result<T, E> {
    Ok(T),
    Err(E)
}

fn safe_divide(a: f32, b: f32) -> Result<f32, string> {
    if b == 0.0 {
        Err("Division by zero")
    } else {
        Ok(a / b)
    }
}

// Pattern matching for error handling
match safe_divide(10.0, 2.0) {
    Ok(result) => print("Result: " + result),
    Err(error) => print("Error: " + error)
}

// Question mark operator for error propagation
fn calculate_complex() -> Result<f32, string> {
    let x = safe_divide(10.0, 2.0)?  // Returns early if error
    let y = safe_divide(x, 3.0)?     // Returns early if error
    Ok(x + y)
}
```

## Best Practices

### When to Use Type Annotations

1. **Function Parameters**: Always annotate function parameters
```script
// ✅ Good - clear parameter types
fn process_data(items: [string], count: i32) -> [string] {
    // ...
}

// ❌ Avoid - unclear what types are expected
// fn process_data(items, count) {
//     // ...
// }
```

2. **Empty Collections**: Annotate empty arrays and collections
```script
// ✅ Good - explicit type for empty array
let names: [string] = []

// ❌ Avoid - ambiguous type
// let names = []
```

3. **Public APIs**: Use explicit types for public interfaces
```script
// ✅ Good - clear API contract
export fn calculate_tax(income: f32, rate: f32) -> f32 {
    income * rate
}

// ❌ Avoid - unclear API
// export fn calculate_tax(income, rate) {
//     income * rate
// }
```

4. **Complex Expressions**: Clarify complex or ambiguous expressions
```script
// ✅ Good - explicit type prevents confusion
let ratio: f32 = total_score / max_score

// Sometimes needed to clarify intent
let percentage: f32 = ratio * 100.0
```

### Type Organization

Group related types and functions together:

```script
// User-related types and functions
struct User {  // Future syntax
    name: string,
    age: i32,
    email: string
}

fn create_user(name: string, age: i32, email: string) -> User {
    User { name, age, email }
}

fn is_adult(user: User) -> bool {
    user.age >= 18
}
```

### Gradual Typing Strategy

1. **Start Simple**: Begin with inferred types
2. **Add Annotations**: Add types for clarity and documentation
3. **Refine Gradually**: Move from unknown to specific types over time

```script
// Phase 1: Start with inference
let data = load_config()
let port = data.port
let host = data.host

// Phase 2: Add strategic annotations
let data = load_config()        // still inferred
let port: i32 = data.port      // explicit for safety
let host: string = data.host   // explicit for clarity

// Phase 3: Full typing (future)
// let data: Config = load_config()
// let port = data.port          // inferred from Config
// let host = data.host         // inferred from Config
```

### Error Handling Best Practices

1. **Current Approach**: Use clear error messages and safe defaults
```script
fn safe_array_access(arr: [i32], index: i32) -> i32 {
    if index < 0 || index >= arr.length {
        print("Warning: Array index out of bounds")
        return 0  // safe default
    }
    arr[index]
}
```

2. **Future Approach**: Use Result types for explicit error handling
```script
fn safe_array_access(arr: [i32], index: i32) -> Result<i32, string> {
    if index < 0 || index >= arr.length {
        Err("Array index out of bounds")
    } else {
        Ok(arr[index])
    }
}
```

### Performance Considerations

1. **Type Annotations**: Explicit types can improve compilation performance
2. **Unknown Types**: Minimize usage of unknown types in performance-critical code
3. **Function Types**: Function pointers have minimal overhead in Script

---

The Script type system provides a powerful foundation for building reliable, maintainable applications while remaining approachable for beginners. As the language evolves, additional type features like generics, user-defined types, and advanced error handling will expand its capabilities further.