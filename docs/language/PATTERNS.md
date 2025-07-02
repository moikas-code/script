# Script Pattern Matching Guide

## Table of Contents

1. [Introduction](#introduction)
2. [Basic Match Expressions](#basic-match-expressions)
3. [Pattern Types](#pattern-types)
4. [Advanced Patterns](#advanced-patterns)
5. [Guards](#guards)
6. [Destructuring](#destructuring)
7. [Pattern Matching in Practice](#pattern-matching-in-practice)
8. [Performance Considerations](#performance-considerations)
9. [Best Practices](#best-practices)
10. [Common Patterns](#common-patterns)

## Introduction

Pattern matching is a powerful feature in Script that allows you to test values against patterns and extract data from complex structures. It provides a concise and expressive way to handle different cases in your code, similar to switch statements but much more powerful.

### Why Pattern Matching?

Pattern matching offers several advantages:
- **Clarity**: Makes code more readable and intent clearer
- **Safety**: Compiler ensures all patterns are covered
- **Expressiveness**: Handle complex data structures elegantly
- **Performance**: Optimized by the compiler for efficiency

### Basic Syntax

```script
match expression {
    pattern1 => result1,
    pattern2 if guard => result2,
    pattern3 => result3,
    _ => default_result
}
```

## Basic Match Expressions

### Simple Value Matching

Match against literal values:

```script
let day = "Monday"

let schedule = match day {
    "Monday" => "Team meeting at 9am",
    "Tuesday" => "Code review at 2pm", 
    "Wednesday" => "Sprint planning",
    "Thursday" => "Development work",
    "Friday" => "Demo and retrospective",
    _ => "Weekend - no work!"
}

print(schedule)  // "Team meeting at 9am"
```

### Numeric Pattern Matching

```script
let score = 85

let grade = match score {
    100 => "Perfect!",
    90 => "Excellent",
    80 => "Good",
    70 => "Satisfactory", 
    60 => "Pass",
    _ => "Needs improvement"
}

// Match ranges with guards (see Guards section)
let category = match score {
    s if s >= 90 => "A",
    s if s >= 80 => "B", 
    s if s >= 70 => "C",
    s if s >= 60 => "D",
    _ => "F"
}
```

### Boolean Pattern Matching

```script
let is_logged_in = true
let is_admin = false

let access_level = match (is_logged_in, is_admin) {
    (true, true) => "Full admin access",
    (true, false) => "User access",
    (false, _) => "No access - please log in"
}
```

## Pattern Types

### Literal Patterns

Match exact values:

```script
let status_code = 404

let message = match status_code {
    200 => "OK",
    404 => "Not Found",
    500 => "Internal Server Error",
    503 => "Service Unavailable",
    _ => "Unknown status"
}
```

### Variable Patterns

Bind matched values to variables:

```script
let number = 42

let result = match number {
    0 => "Zero",
    n => "The number is " + n  // n binds to the value of number
}

print(result)  // "The number is 42"
```

### Wildcard Pattern

The wildcard pattern `_` matches anything but doesn't bind the value:

```script
let value = "anything"

let result = match value {
    "important" => "Handle specially",
    _ => "Default handling"  // Matches all other cases
}
```

### Array Patterns

Match and destructure arrays:

```script
let coordinates = [10, 20]

let description = match coordinates {
    [] => "No coordinates",
    [x] => "Single coordinate: " + x,
    [x, y] => "2D coordinate: (" + x + ", " + y + ")",
    [x, y, z] => "3D coordinate: (" + x + ", " + y + ", " + z + ")",
    _ => "Higher dimensional coordinate"
}

print(description)  // "2D coordinate: (10, 20)"
```

### Nested Array Patterns

```script
let matrix = [[1, 2], [3, 4]]

let result = match matrix {
    [] => "Empty matrix",
    [[]] => "Matrix with empty row",
    [[a]] => "1x1 matrix with value " + a,
    [[a, b], [c, d]] => "2x2 matrix",
    _ => "Complex matrix"
}
```

### Or Patterns

Match multiple patterns with the same result:

```script
let character = 'a'

let type = match character {
    'a' | 'e' | 'i' | 'o' | 'u' => "vowel",
    'y' => "sometimes vowel",
    _ => "consonant"
}

// Multiple numeric values
let day_number = 1

let weekend = match day_number {
    1 | 7 => "Weekend day",     // Sunday or Saturday
    2 | 3 | 4 | 5 | 6 => "Weekday",
    _ => "Invalid day"
}
```

## Advanced Patterns

### Object Patterns (Future)

When Script supports objects/structs, you'll be able to destructure them:

```script
// Future syntax - not yet implemented
struct Point {
    x: f32,
    y: f32
}

let point = Point { x: 3.0, y: 4.0 }

let quadrant = match point {
    Point { x: 0.0, y: 0.0 } => "Origin",
    Point { x, y } if x > 0.0 && y > 0.0 => "First quadrant",
    Point { x, y } if x < 0.0 && y > 0.0 => "Second quadrant", 
    Point { x, y } if x < 0.0 && y < 0.0 => "Third quadrant",
    Point { x, y } if x > 0.0 && y < 0.0 => "Fourth quadrant",
    Point { x: 0.0, y } => "On Y axis",
    Point { x, y: 0.0 } => "On X axis"
}
```

### Slice Patterns (Future)

```script
// Future syntax for matching array slices
let numbers = [1, 2, 3, 4, 5]

let pattern = match numbers {
    [first, ..rest] => "First: " + first + ", rest has " + rest.length + " elements",
    [..middle, last] => "Last: " + last + ", middle has " + middle.length + " elements",
    [first, ..middle, last] => "First: " + first + ", last: " + last,
    [] => "Empty array"
}
```

## Guards

Guards are additional conditions that can be added to patterns using the `if` keyword:

### Basic Guards

```script
let number = 15

let classification = match number {
    n if n < 0 => "Negative",
    n if n == 0 => "Zero", 
    n if n > 0 && n <= 10 => "Small positive",
    n if n > 10 && n <= 100 => "Medium positive",
    n => "Large positive"
}

print(classification)  // "Medium positive"
```

### Complex Guards

```script
let user_age = 25
let has_license = true

let can_drive = match (user_age, has_license) {
    (age, license) if age >= 18 && license => "Can drive",
    (age, license) if age >= 18 && !license => "Can get license",
    (age, _) if age >= 16 => "Can get learner's permit", 
    _ => "Too young to drive"
}
```

### Guards with Array Patterns

```script
let scores = [85, 92, 78]

let grade_summary = match scores {
    [] => "No scores",
    [score] if score >= 90 => "Single excellent score",
    [score] if score >= 70 => "Single good score", 
    [score] => "Single poor score",
    scores if scores.length > 5 => "Too many scores to analyze",
    scores => "Multiple scores: analyzing..."
}
```

### Function-Based Guards

```script
fn is_prime(n: i32) -> bool {
    if n <= 1 { return false }
    if n <= 3 { return true }
    if n % 2 == 0 || n % 3 == 0 { return false }
    
    let i = 5
    while i * i <= n {
        if n % i == 0 || n % (i + 2) == 0 {
            return false
        }
        i = i + 6
    }
    true
}

let number = 17

let classification = match number {
    n if n < 0 => "Negative",
    0 => "Zero",
    1 => "One", 
    n if is_prime(n) => "Prime number",
    n if n % 2 == 0 => "Even composite",
    n => "Odd composite"
}
```

## Destructuring

### Array Destructuring

Extract values from arrays directly in patterns:

```script
let point_3d = [1.0, 2.0, 3.0]

// Extract individual components
let description = match point_3d {
    [x, y, z] => {
        let distance = (x * x + y * y + z * z).sqrt()
        "Point at (" + x + ", " + y + ", " + z + ") with distance " + distance
    },
    [x, y] => "2D point at (" + x + ", " + y + ")",
    [x] => "1D point at " + x,
    [] => "No coordinates"
}
```

### Nested Array Destructuring

```script
let game_board = [
    ["X", "O", " "],
    ["O", "X", "O"], 
    [" ", " ", "X"]
]

let winner = match game_board {
    // Check rows
    [[a, b, c], _, _] if a == b && b == c && a != " " => a + " wins (top row)",
    [_, [a, b, c], _] if a == b && b == c && a != " " => a + " wins (middle row)",
    [_, _, [a, b, c]] if a == b && b == c && a != " " => a + " wins (bottom row)",
    
    // Check columns  
    [[a, _, _], [b, _, _], [c, _, _]] if a == b && b == c && a != " " => a + " wins (left column)",
    [[_, a, _], [_, b, _], [_, c, _]] if a == b && b == c && a != " " => a + " wins (middle column)",
    [[_, _, a], [_, _, b], [_, _, c]] if a == b && b == c && a != " " => a + " wins (right column)",
    
    // Check diagonals
    [[a, _, _], [_, b, _], [_, _, c]] if a == b && b == c && a != " " => a + " wins (diagonal)",
    [[_, _, a], [_, b, _], [c, _, _]] if a == b && b == c && a != " " => a + " wins (anti-diagonal)",
    
    _ => "Game continues"
}
```

### Partial Destructuring

You don't need to match all elements:

```script
let rgb_color = [255, 128, 64, 0.8]  // RGBA

let color_info = match rgb_color {
    [r, g, b] => "RGB color: (" + r + ", " + g + ", " + b + ")",
    [r, g, b, a] => "RGBA color: (" + r + ", " + g + ", " + b + ", " + a + ")",
    [r] => "Red component: " + r,
    [] => "No color data"
}
```

## Pattern Matching in Practice

### State Machine Implementation

```script
enum GameState {  // Future syntax
    Menu,
    Playing,
    Paused,
    GameOver
}

fn handle_input(state: GameState, input: string) -> GameState {
    match (state, input) {
        (GameState::Menu, "start") => GameState::Playing,
        (GameState::Menu, "quit") => GameState::GameOver,
        
        (GameState::Playing, "pause") => GameState::Paused,
        (GameState::Playing, "quit") => GameState::Menu,
        
        (GameState::Paused, "resume") => GameState::Playing,
        (GameState::Paused, "quit") => GameState::Menu,
        
        (GameState::GameOver, "restart") => GameState::Menu,
        (GameState::GameOver, "quit") => GameState::GameOver,
        
        (current_state, _) => current_state  // Invalid input, stay in current state
    }
}
```

### Command Parser

```script
fn parse_command(input: string) -> string {
    let parts = input.split(" ")  // Future method
    
    match parts {
        [] => "Empty command",
        ["help"] => "Available commands: move, look, take, quit",
        ["move", direction] => "Moving " + direction,
        ["look"] => "You look around...",
        ["look", target] => "You examine the " + target,
        ["take", item] => "You take the " + item,
        ["quit"] => "Goodbye!",
        [command] => "Unknown command: " + command,
        [command, ..args] => "Unknown command: " + command + " with args"
    }
}

// Usage
let response = parse_command("move north")  // "Moving north"
let help = parse_command("help")            // "Available commands: ..."
```

### Data Validation

```script
fn validate_user_input(data: [string]) -> string {
    match data {
        [] => "Error: No data provided",
        [name] if name.length == 0 => "Error: Name cannot be empty",
        [name] if name.length > 50 => "Error: Name too long",
        [name] => "Valid user: " + name,
        
        [name, email] if !email.contains("@") => "Error: Invalid email format",
        [name, email] => "Valid user: " + name + " <" + email + ">",
        
        [name, email, age] => {
            let age_num = parse_int(age)
            if age_num < 0 || age_num > 120 {
                "Error: Invalid age"
            } else {
                "Valid user: " + name + ", " + age + " years old"
            }
        },
        
        _ => "Error: Too many fields"
    }
}
```

### Configuration Processing

```script
fn process_config(config: [string]) -> string {
    match config {
        ["debug", "true"] => "Debug mode enabled",
        ["debug", "false"] => "Debug mode disabled",
        ["debug", value] => "Error: debug must be true or false, got " + value,
        
        ["port", port_str] => {
            let port = parse_int(port_str)
            if port >= 1024 && port <= 65535 {
                "Port set to " + port_str
            } else {
                "Error: Port must be between 1024 and 65535"
            }
        },
        
        ["server", "production"] => "Production server configuration",
        ["server", "development"] => "Development server configuration", 
        ["server", "test"] => "Test server configuration",
        ["server", env] => "Error: Unknown environment " + env,
        
        [key, value] => "Set " + key + " = " + value,
        [key] => "Error: No value provided for " + key,
        [] => "Error: No configuration key provided"
    }
}
```

## Performance Considerations

### Pattern Matching Optimization

The Script compiler optimizes pattern matching in several ways:

1. **Jump Tables**: For simple value matching, the compiler generates efficient jump tables
2. **Decision Trees**: Complex patterns are compiled into optimal decision trees
3. **Guard Optimization**: Guards are evaluated in the most efficient order

### Performance Tips

1. **Order Patterns by Frequency**: Put most common patterns first
```script
// ✅ Good - most common case first
match http_status {
    200 => "OK",                    // Most common
    404 => "Not Found",             // Common
    500 => "Internal Server Error", // Less common
    _ => "Other status"             // Rare
}
```

2. **Prefer Literal Patterns**: Literal patterns are fastest
```script
// ✅ Fast - literal pattern matching
match status {
    "active" => handle_active(),
    "inactive" => handle_inactive(),
    _ => handle_unknown()
}

// ❌ Slower - requires string operations
match status {
    s if s.starts_with("act") => handle_active(),
    s if s.starts_with("inact") => handle_inactive(),
    _ => handle_unknown()
}
```

3. **Minimize Guard Complexity**: Simple guards are more efficient
```script
// ✅ Simple guard
match value {
    n if n > 0 => "positive",
    n if n < 0 => "negative", 
    _ => "zero"
}

// ❌ Complex guard
match value {
    n if complex_calculation(n) > threshold => "complex case",
    _ => "simple case"
}
```

## Best Practices

### Exhaustive Pattern Matching

Always handle all possible cases:

```script
// ✅ Good - exhaustive matching
fn describe_day(day: i32) -> string {
    match day {
        1 => "Monday",
        2 => "Tuesday", 
        3 => "Wednesday",
        4 => "Thursday",
        5 => "Friday",
        6 => "Saturday",
        7 => "Sunday",
        _ => "Invalid day"  // Handle invalid input
    }
}

// ❌ Avoid - non-exhaustive matching
// fn describe_day(day: i32) -> string {
//     match day {
//         1 => "Monday",
//         2 => "Tuesday",
//         // Missing other cases!
//     }
// }
```

### Use Meaningful Variable Names in Patterns

```script
// ✅ Good - descriptive names
match user_input {
    [username, password] if username.length > 0 => authenticate(username, password),
    [username] => "Password required for " + username,
    [] => "Username and password required",
    _ => "Too many arguments"
}

// ❌ Avoid - unclear names
match user_input {
    [a, b] if a.length > 0 => authenticate(a, b),
    [a] => "Password required for " + a,
    [] => "Username and password required",
    _ => "Too many arguments"
}
```

### Prefer Pattern Matching Over Nested If-Else

```script
// ✅ Good - clear pattern matching
let message = match (status, priority) {
    ("error", "high") => "CRITICAL: " + details,
    ("error", "medium") => "ERROR: " + details,
    ("error", "low") => "Warning: " + details,
    ("info", _) => "Info: " + details,
    ("debug", _) => "Debug: " + details,
    _ => "Unknown: " + details
}

// ❌ Avoid - nested if-else
// let message = if status == "error" {
//     if priority == "high" {
//         "CRITICAL: " + details
//     } else if priority == "medium" {
//         "ERROR: " + details  
//     } else {
//         "Warning: " + details
//     }
// } else if status == "info" {
//     "Info: " + details
// } else if status == "debug" {
//     "Debug: " + details
// } else {
//     "Unknown: " + details
// }
```

### Use Guards Sparingly

```script
// ✅ Good - simple, clear guards
match score {
    s if s >= 90 => "A",
    s if s >= 80 => "B",
    s if s >= 70 => "C", 
    s if s >= 60 => "D",
    _ => "F"
}

// ❌ Avoid - overly complex guards
match data {
    d if validate_complex_business_rule(d) && check_permissions(user, d) && is_valid_date(d.date) => process_data(d),
    _ => reject_data(d)
}
```

## Common Patterns

### Option-Like Patterns (Current)

```script
// Simulate optional values with special values
fn find_index(array: [string], target: string) -> i32 {
    for i in 0..array.length {
        if array[i] == target {
            return i
        }
    }
    -1  // -1 indicates "not found"
}

// Pattern match on the result
let index = find_index(names, "Alice")
let message = match index {
    -1 => "Name not found",
    i => "Found at index " + i
}
```

### Result-Like Patterns (Current)

```script
// Simulate result types with status codes
fn divide_safe(a: f32, b: f32) -> [f32] {  // [result] for success, [] for error
    if b == 0.0 {
        []  // Empty array indicates error
    } else {
        [a / b]  // Single element array contains result
    }
}

// Pattern match on the result
let division_result = divide_safe(10.0, 2.0)
let message = match division_result {
    [] => "Error: Division by zero",
    [result] => "Result: " + result,
    _ => "Unexpected result format"
}
```

### State Pattern

```script
// Represent state as strings and match on them
let current_state = "playing"

let next_state = match (current_state, user_action) {
    ("menu", "start") => "playing",
    ("menu", "settings") => "settings",
    ("menu", "quit") => "exit",
    
    ("playing", "pause") => "paused",
    ("playing", "quit") => "menu",
    ("playing", "inventory") => "inventory",
    
    ("paused", "resume") => "playing",
    ("paused", "quit") => "menu",
    
    ("settings", "back") => "menu",
    ("settings", _) => "settings",  // Stay in settings for other actions
    
    ("inventory", "back") => "playing",
    ("inventory", _) => "inventory",
    
    (state, _) => state  // Invalid transition, stay in current state
}
```

### Command Pattern

```script
// Parse and execute commands
fn execute_command(input: string) -> string {
    let tokens = tokenize(input)  // Assume this function exists
    
    match tokens {
        // Movement commands
        ["go", direction] => move_player(direction),
        ["move", direction] => move_player(direction),
        ["walk", direction] => move_player(direction),
        
        // Interaction commands
        ["look"] => describe_current_room(),
        ["look", "at", target] => examine_object(target),
        ["examine", target] => examine_object(target),
        
        ["take", item] => take_item(item),
        ["pick", "up", item] => take_item(item),
        ["get", item] => take_item(item),
        
        ["use", item] => use_item(item),
        ["use", item, "on", target] => use_item_on(item, target),
        
        // Meta commands
        ["help"] => show_help(),
        ["inventory"] => show_inventory(),
        ["quit"] => "Goodbye!",
        ["exit"] => "Goodbye!",
        
        // Handle unknown commands
        [] => "Please enter a command.",
        [command] => "I don't understand '" + command + "'.",
        [command, ..args] => "I don't know how to '" + command + "' with those arguments.",
        _ => "That command is too complex for me to understand."
    }
}
```

---

Pattern matching in Script provides a powerful and expressive way to handle complex control flow and data extraction. As the language evolves, pattern matching capabilities will expand with new data types like enums, structs, and advanced destructuring patterns. The foundation provided here ensures that pattern matching remains both powerful for advanced users and approachable for beginners.