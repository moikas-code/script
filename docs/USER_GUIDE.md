# Script Language User Guide

Welcome to Script - a modern programming language designed to be simple enough for beginners while powerful enough for production applications. This guide will help you get started with Script and explore its features.

## Table of Contents
1. [Getting Started](#getting-started)
2. [Language Basics](#language-basics)
3. [Functions and Control Flow](#functions-and-control-flow)
4. [Type System](#type-system)
5. [Pattern Matching](#pattern-matching)
6. [Modules and Packages](#modules-and-packages)
7. [Async Programming](#async-programming)
8. [Collections and Data Structures](#collections-and-data-structures)
9. [Error Handling](#error-handling)
10. [Testing Your Code](#testing-your-code)
11. [Using the Tooling](#using-the-tooling)
12. [Game Development](#game-development)
13. [Web Development](#web-development)
14. [Best Practices](#best-practices)

## Getting Started

### Installation

First, install the Script compiler and tools:

```bash
# Clone the repository
git clone https://github.com/moikapy/script
cd script

# Build and install
cargo install --path .

# Verify installation
script --version
```

### Your First Script Program

Create a file called `hello.script`:

```script
fn main() {
    println("Hello, Script!")
}
```

Run it:

```bash
script hello.script
```

### Interactive REPL

Script provides an interactive REPL for experimentation:

```bash
# Start REPL in parse mode (default)
script

# Start REPL in token mode (for debugging)
script --tokens
```

## Language Basics

### Variables and Constants

Script uses `let` for variables and `const` for constants:

```script
// Variables (mutable by default)
let name = "Script"
let count = 42
let price = 19.99
let is_awesome = true

// Type annotations (optional)
let age: i32 = 25
let height: f32 = 5.9
let message: String = "Hello"

// Constants (must be compile-time values)
const PI = 3.14159
const MAX_PLAYERS = 4
const VERSION = "1.0.0"
```

### Basic Types

Script has the following built-in types:

- `i32` - 32-bit integer
- `f32` - 32-bit floating point
- `bool` - Boolean (true/false)
- `String` - UTF-8 string
- `Array<T>` - Dynamic array
- `Result<T, E>` - Error handling type
- `Option<T>` - Optional values

### Operators

```script
// Arithmetic
let sum = 10 + 5        // 15
let diff = 10 - 5       // 5
let product = 10 * 5    // 50
let quotient = 10 / 5   // 2
let remainder = 10 % 3  // 1

// Comparison
let equal = 5 == 5      // true
let not_equal = 5 != 3  // true
let less = 5 < 10       // true
let greater = 10 > 5    // true
let less_eq = 5 <= 5    // true
let greater_eq = 10 >= 10  // true

// Logical
let and_result = true && false  // false
let or_result = true || false   // true
let not_result = !true          // false

// String concatenation
let greeting = "Hello, " + "World!"  // "Hello, World!"
```

### Comments

```script
// This is a single-line comment

/*
   This is a
   multi-line comment
*/
```

## Functions and Control Flow

### Function Definitions

```script
// Basic function
fn greet(name: String) {
    println("Hello, " + name + "!")
}

// Function with return value
fn add(a: i32, b: i32) -> i32 {
    a + b  // Implicit return (no semicolon)
}

// Function with explicit return
fn max(a: i32, b: i32) -> i32 {
    if a > b {
        return a
    }
    b
}

// Function with default parameters (future feature)
fn create_player(name: String, level: i32 = 1) -> Player {
    Player { name, level }
}
```

### If Expressions

In Script, `if` is an expression that returns a value:

```script
// Basic if statement
if temperature > 30 {
    println("It's hot!")
}

// If-else expression
let status = if health > 0 { "alive" } else { "dead" }

// Multi-branch if
let grade = if score >= 90 {
    "A"
} else if score >= 80 {
    "B"
} else if score >= 70 {
    "C"
} else {
    "F"
}
```

### Loops

```script
// While loop
let mut count = 0
while count < 10 {
    println("Count: " + count)
    count = count + 1
}

// For loop with range
for i in 0..10 {
    println("Index: " + i)
}

// For loop with array
let fruits = ["apple", "banana", "orange"]
for fruit in fruits {
    println("Fruit: " + fruit)
}

// Loop with break and continue
for i in 0..100 {
    if i % 2 == 0 {
        continue  // Skip even numbers
    }
    if i > 10 {
        break  // Stop at 10
    }
    println(i)
}
```

### Blocks

Blocks are expressions that evaluate to their last expression:

```script
let result = {
    let x = 10
    let y = 20
    x + y  // Block evaluates to 30
}
```

## Type System

Script features a gradual type system with type inference:

### Type Annotations

```script
// Optional type annotations
let name: String = "Script"
let age: i32 = 25
let scores: Array<i32> = [90, 85, 88]

// Function parameter and return types
fn calculate_area(width: f32, height: f32) -> f32 {
    width * height
}

// Type inference
let inferred = 42  // Inferred as i32
let message = "Hello"  // Inferred as String
```

### Type Aliases (Future Feature)

```script
type UserId = i32
type Point2D = { x: f32, y: f32 }
type GameState = "menu" | "playing" | "paused" | "game_over"
```

### Structs (Future Feature)

```script
struct Player {
    name: String,
    health: i32,
    position: Vec2,
    inventory: Array<Item>
}

// Methods
impl Player {
    fn new(name: String) -> Player {
        Player {
            name,
            health: 100,
            position: Vec2 { x: 0.0, y: 0.0 },
            inventory: []
        }
    }
    
    fn take_damage(mut self, amount: i32) {
        self.health = max(0, self.health - amount)
    }
}
```

## Pattern Matching

Pattern matching is a powerful feature for destructuring and control flow:

### Match Expressions

```script
// Basic pattern matching
let result = match value {
    0 -> "zero",
    1 -> "one",
    2 -> "two",
    _ -> "many"  // Wildcard pattern
}

// Pattern matching with guards
let description = match age {
    n if n < 13 -> "child",
    n if n < 20 -> "teenager",
    n if n < 60 -> "adult",
    _ -> "senior"
}

// Destructuring arrays
match point {
    [0, 0] -> "origin",
    [x, 0] -> "on x-axis at " + x,
    [0, y] -> "on y-axis at " + y,
    [x, y] -> "at (" + x + ", " + y + ")",
    _ -> "invalid point"
}

// Destructuring objects
match player {
    { health: 0, .. } -> "player is dead",
    { health: h, name: n } if h < 20 -> n + " is critical!",
    { name, .. } -> name + " is healthy"
}

// Or patterns
match key {
    "w" | "up" -> move_up(),
    "s" | "down" -> move_down(),
    "a" | "left" -> move_left(),
    "d" | "right" -> move_right(),
    _ -> {}  // Do nothing
}
```

### Destructuring in Let Bindings

```script
// Array destructuring
let [x, y, z] = [1, 2, 3]

// Object destructuring
let { name, age } = person

// With renaming
let { name: player_name, health: hp } = player
```

## Modules and Packages

### Creating Modules

Organize your code into modules for better structure:

```script
// math/geometry.script
export fn area_circle(radius: f32) -> f32 {
    PI * radius * radius
}

export fn area_rectangle(width: f32, height: f32) -> f32 {
    width * height
}

export const PI = 3.14159

// Private function (not exported)
fn helper() -> i32 {
    42
}
```

### Importing Modules

```script
// Import specific items
import { area_circle, PI } from "./math/geometry"

// Import with aliases
import { area_circle as circle_area } from "./math/geometry"

// Import all exports
import * as geo from "./math/geometry"

// Import from packages
import { Vec2, Vec3 } from "mathlib"
import { serve, Router } from "web"
```

### Package Management

Create a `script.toml` file for your project:

```toml
[package]
name = "my-game"
version = "0.1.0"
authors = ["Your Name <you@example.com>"]
description = "An awesome game in Script"

[dependencies]
mathlib = "1.2.0"
graphics = "2.0.0"
gamedev = { version = "3.0", features = ["physics"] }

[dev-dependencies]
test-utils = "0.5.0"
```

Install dependencies:

```bash
manuscript install
```

## Async Programming

Script provides built-in support for asynchronous programming:

### Async Functions

```script
// Define an async function
async fn fetch_data(url: String) -> Result<String> {
    let response = await http_get(url)?
    Ok(response.text())
}

// Using async functions
async fn main() {
    match await fetch_data("https://api.example.com/data") {
        Ok(data) -> println("Got data: " + data),
        Err(e) -> println("Error: " + e)
    }
}

// Concurrent operations
async fn load_game_assets() {
    // Run multiple async operations concurrently
    let [textures, sounds, models] = await all([
        load_textures(),
        load_sounds(),
        load_models()
    ])
}
```

### Working with Futures

```script
// Create a future
let future = async {
    await delay(1000)  // Wait 1 second
    "Done!"
}

// Execute a future
let result = await future

// Future combinators
let combined = future1.and_then(|result| {
    async { result + " and more" }
})
```

## Collections and Data Structures

### Arrays

```script
// Array creation
let numbers = [1, 2, 3, 4, 5]
let empty: Array<i32> = []

// Array operations
numbers.push(6)
let first = numbers[0]
let last = numbers[numbers.len() - 1]

// Array methods
let doubled = numbers.map(|n| n * 2)
let evens = numbers.filter(|n| n % 2 == 0)
let sum = numbers.reduce(0, |acc, n| acc + n)

// List comprehensions
let squares = [x * x for x in 1..10]
let even_squares = [x * x for x in 1..10 if x % 2 == 0]
```

### HashMaps (Future Feature)

```script
// Create a HashMap
let mut scores = HashMap::new()
scores.insert("Alice", 100)
scores.insert("Bob", 85)

// Access values
let alice_score = scores.get("Alice")  // Option<i32>

// Iterate over HashMap
for (name, score) in scores {
    println(name + ": " + score)
}
```

### Strings

```script
// String creation
let name = "Script"
let message = String::from("Hello, World!")

// String operations
let greeting = "Hello, " + name
let shouting = message.to_uppercase()
let length = message.len()

// String methods
let words = message.split(" ")
let trimmed = "  hello  ".trim()
let replaced = message.replace("World", "Script")

// String interpolation (future feature)
let formatted = `Hello, ${name}! You have ${count} messages.`
```

## Error Handling

Script uses Result types for explicit error handling:

### Result Type

```script
// Function that can fail
fn divide(a: f32, b: f32) -> Result<f32> {
    if b == 0.0 {
        Err("Division by zero")
    } else {
        Ok(a / b)
    }
}

// Handling errors explicitly
match divide(10.0, 2.0) {
    Ok(result) -> println("Result: " + result),
    Err(error) -> println("Error: " + error)
}

// Using the ? operator
fn calculate() -> Result<f32> {
    let x = divide(10.0, 2.0)?  // Returns early if error
    let y = divide(x, 3.0)?
    Ok(x + y)
}
```

### Option Type

```script
// Optional values
fn find_player(id: i32) -> Option<Player> {
    // Return Some(player) if found, None otherwise
}

// Using options
match find_player(123) {
    Some(player) -> println("Found: " + player.name),
    None -> println("Player not found")
}

// Option methods
let name = find_player(123)
    .map(|p| p.name)
    .unwrap_or("Unknown")
```

### Panic for Unrecoverable Errors

```script
// Assertions
assert(index < array.len(), "Index out of bounds")

// Panic with message
if critical_error {
    panic("Critical system failure!")
}
```

## Testing Your Code

Script has built-in testing support:

### Writing Tests

```script
// test_math.script
@test
fn test_addition() {
    assert_eq(2 + 2, 4)
}

@test
fn test_string_concat() {
    let result = "Hello, " + "World!"
    assert_eq(result, "Hello, World!")
}

@test
fn test_division_by_zero() {
    match divide(10.0, 0.0) {
        Ok(_) -> assert(false, "Should have failed"),
        Err(e) -> assert(e.contains("zero"))
    }
}

// Test with setup
@test
fn test_player_damage() {
    let player = Player::new("Test")
    player.take_damage(30)
    assert_eq(player.health, 70)
}
```

### Running Tests

```bash
# Run all tests
script test

# Run specific test file
script test test_math.script

# Run tests with pattern
script test --filter "player"

# Run tests in parallel
script test --parallel
```

### Test Assertions

```script
// Equality assertions
assert_eq(actual, expected)
assert_ne(actual, not_expected)

// Boolean assertions
assert(condition)
assert(condition, "Custom error message")

// Comparison assertions
assert_lt(a, b)  // less than
assert_gt(a, b)  // greater than
assert_le(a, b)  // less than or equal
assert_ge(a, b)  // greater than or equal

// String assertions
assert(text.contains("substring"))
assert(text.starts_with("prefix"))
assert(text.ends_with("suffix"))
```

## Using the Tooling

### Language Server (LSP)

Script provides a Language Server Protocol implementation for IDE support:

```bash
# Start the language server
script-lsp
```

Features:
- Syntax highlighting
- Auto-completion
- Go to definition
- Error diagnostics
- Hover information

### Package Manager (Manuscript)

```bash
# Initialize a new project
manuscript init

# Add a dependency
manuscript add mathlib

# Update dependencies
manuscript update

# Build the project
manuscript build

# Publish a package
manuscript publish
```

### Documentation Generator

Generate documentation from your code:

```script
/// Calculates the area of a circle
/// 
/// # Parameters
/// - `radius`: The radius of the circle
/// 
/// # Returns
/// The area of the circle
/// 
/// # Example
/// ```
/// let area = area_circle(5.0)
/// assert_eq(area, 78.53975)
/// ```
export fn area_circle(radius: f32) -> f32 {
    PI * radius * radius
}
```

Generate docs:

```bash
# Generate HTML documentation
script doc

# Generate and open in browser
script doc --open
```

### Debugger

Debug your Script programs:

```bash
# Run with debugger
script debug program.script

# Set breakpoints
(sdb) break main.script:10
(sdb) break calculate_score

# Control execution
(sdb) run
(sdb) continue
(sdb) step
(sdb) next
(sdb) finish

# Inspect values
(sdb) print player.health
(sdb) watch score
```

## Game Development

Script includes built-in support for game development:

### Vector Math

```script
import { Vec2, Vec3, Mat4 } from "std::math"

// 2D vectors
let position = Vec2 { x: 100.0, y: 200.0 }
let velocity = Vec2 { x: 5.0, y: -3.0 }

// Vector operations
let new_pos = position + velocity * delta_time
let distance = position.distance_to(target)
let normalized = velocity.normalize()

// 3D transformations
let transform = Mat4::identity()
    .translate(Vec3 { x: 10.0, y: 0.0, z: 5.0 })
    .rotate_y(angle)
    .scale(Vec3::uniform(2.0))
```

### Game Loop

```script
import { Timer, Input } from "std::game"

fn game_loop() {
    let mut timer = Timer::new()
    let mut running = true
    
    while running {
        let delta_time = timer.delta()
        
        // Handle input
        if Input::key_pressed(KeyCode::Escape) {
            running = false
        }
        
        // Update game state
        update_game(delta_time)
        
        // Render
        render_game()
        
        // Cap frame rate
        timer.sleep_until_fps(60)
    }
}
```

### Random Generation

```script
import { Random } from "std::random"

let mut rng = Random::new()

// Generate random values
let damage = rng.range(10, 20)
let crit_chance = rng.float()  // 0.0 to 1.0
let enemy_type = rng.choose(["goblin", "orc", "troll"])

// Seeded generation for reproducibility
let seeded_rng = Random::with_seed(12345)
```

## Web Development

Script can compile to WebAssembly for web applications:

### HTTP Server (Future Feature)

```script
import { serve, Router, Response } from "web"

async fn main() {
    let router = Router::new()
    
    router.get("/", |req| {
        Response::html("<h1>Hello from Script!</h1>")
    })
    
    router.get("/api/users/:id", |req| {
        let id = req.params.get("id")
        Response::json({
            id: id,
            name: "User " + id
        })
    })
    
    await serve(router, { port: 3000 })
}
```

### WebAssembly Integration

```script
// Export functions to JavaScript
@wasm_export
fn calculate_physics(positions: Array<Vec2>, delta: f32) -> Array<Vec2> {
    // Physics calculations
}

// Import from JavaScript
@wasm_import("console", "log")
fn js_log(message: String)
```

## Best Practices

### Code Organization

1. **Use modules** to organize related functionality
2. **Keep functions small** and focused on a single task
3. **Use descriptive names** for variables and functions
4. **Document public APIs** with doc comments

### Error Handling

1. **Use Result types** for operations that can fail
2. **Handle errors explicitly** rather than ignoring them
3. **Provide meaningful error messages**
4. **Use panic only for unrecoverable errors**

### Performance Tips

1. **Avoid unnecessary allocations** in hot loops
2. **Use `const` for compile-time constants**
3. **Prefer iteration over recursion** for better performance
4. **Profile before optimizing** - don't guess

### Type System Usage

1. **Add type annotations** to function signatures
2. **Let type inference work** for local variables
3. **Use type aliases** for complex types
4. **Design with types in mind** for better safety

### Testing

1. **Write tests as you code**, not after
2. **Test edge cases** and error conditions
3. **Keep tests focused** and independent
4. **Use descriptive test names**

## Conclusion

This guide covers the essential features of Script. For more detailed information, refer to:

- [Language Reference](./LANGUAGE_REFERENCE.md) - Complete syntax and semantics
- [Standard Library API](./stdlib/README.md) - All built-in functions and types
- [Developer Guide](./DEVELOPER_GUIDE.md) - Contributing to Script

Happy coding with Script! 🚀