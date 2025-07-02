# Getting Started with Script

Welcome to Script! This tutorial will guide you from your first "Hello World" program to writing sophisticated applications. Script is designed to be beginner-friendly while providing the power and performance needed for real-world projects.

## Table of Contents

1. [Installation](#installation)
2. [Your First Program](#your-first-program)
3. [Basic Syntax](#basic-syntax)
4. [Variables and Types](#variables-and-types)
5. [Functions](#functions)
6. [Control Flow](#control-flow)
7. [Collections](#collections)
8. [Error Handling](#error-handling)
9. [Working with Files](#working-with-files)
10. [Next Steps](#next-steps)

## Installation

### From Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/moikapy/script.git
cd script

# Build Script
cargo build --release

# Install globally (optional)
sudo cp target/release/script /usr/local/bin/

# Verify installation
script --version
```

### Using Cargo

```bash
# Install directly with Cargo
cargo install script

# Verify installation
script --version
```

## Your First Program

Let's start with the classic "Hello World" program. Create a file called `hello.script`:

```script
// hello.script - Your first Script program!
fn main() {
    print("Hello, World!")
}
```

Run your program:

```bash
script hello.script
```

You should see:
```
Hello, World!
```

### Using the REPL

Script includes an interactive REPL (Read-Eval-Print Loop) perfect for experimentation:

```bash
# Start the REPL
script

# Try some expressions
> 2 + 3
5
> let name = "Alice"
> print("Hello, " + name)
Hello, Alice
```

The REPL is excellent for learning and testing ideas quickly!

## Basic Syntax

Script has clean, intuitive syntax that feels familiar to programmers from many backgrounds:

```script
// Comments start with double slashes
/* Multi-line comments
   work like this */

// Statements end with newlines (semicolons are optional)
let x = 42
let y = 3.14

// Variables can be reassigned
x = 100

// Expressions return values
let result = if x > 50 { "big" } else { "small" }
```

### Key Principles

1. **Everything is an expression** - Almost everything returns a value
2. **Optional semicolons** - Use them for clarity, but they're not required
3. **Type inference** - Types are inferred automatically but can be specified
4. **Expression-oriented** - if, while, and blocks all return values

## Variables and Types

### Basic Variables

```script
// Let creates variables with type inference
let name = "Alice"        // string
let age = 25             // i32 (32-bit integer)
let height = 5.6         // f32 (32-bit float)
let is_student = true    // bool

// Variables are mutable by default
age = 26
name = "Bob"
```

### Explicit Types

While Script infers types automatically, you can specify them for clarity:

```script
let score: i32 = 100
let temperature: f32 = 98.6
let message: string = "Hello"
let is_ready: bool = false
```

### Type System Basics

Script has several built-in types:

```script
// Numbers
let small_int: i32 = 42
let big_int: i64 = 1000000
let decimal: f32 = 3.14
let precise: f64 = 3.141592653589793

// Text
let greeting: string = "Hello, Script!"

// Boolean
let flag: bool = true

// Optional values (covered later)
let maybe_number: Option<i32> = Option::some(42)
let no_value: Option<i32> = Option::none()
```

### Constants

Use constants for values that never change:

```script
// Constants are immutable and must have explicit values
const PI: f32 = 3.14159
const APP_NAME: string = "My Script App"
const MAX_PLAYERS: i32 = 100
```

## Functions

Functions are the building blocks of Script programs:

### Basic Functions

```script
// Simple function
fn greet() {
    print("Hello from a function!")
}

// Function with parameters
fn greet_person(name: string) {
    print("Hello, " + name + "!")
}

// Function with return value
fn add(a: i32, b: i32) -> i32 {
    return a + b
}

// Using functions
greet()
greet_person("Alice")
let sum = add(5, 3)
print("5 + 3 = " + sum)
```

### Expression-Based Returns

In Script, the last expression in a function is automatically returned:

```script
// These are equivalent:
fn multiply1(a: i32, b: i32) -> i32 {
    return a * b
}

fn multiply2(a: i32, b: i32) -> i32 {
    a * b  // No return needed!
}

// Works with if expressions too
fn max(a: i32, b: i32) -> i32 {
    if a > b { a } else { b }
}
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

// Format a greeting
fn format_greeting(name: string, age: i32) -> string {
    "Hello, " + name + "! You are " + age + " years old."
}

// Using the functions
let fact5 = factorial(5)
let even_check = is_even(42)
let greeting = format_greeting("Bob", 30)

print("5! = " + fact5)
print("42 is even: " + even_check)
print(greeting)
```

## Control Flow

### If Expressions

In Script, `if` is an expression that returns a value:

```script
// Basic if
let weather = "sunny"
if weather == "sunny" {
    print("It's a beautiful day!")
}

// If-else expression
let temperature = 75
let clothing = if temperature > 70 {
    "shorts and t-shirt"
} else if temperature > 50 {
    "jeans and sweater"
} else {
    "winter coat"
}

print("Wear: " + clothing)

// Nested conditions
let age = 25
let has_license = true
let can_drive = if age >= 16 {
    if has_license {
        "yes"
    } else {
        "needs license"
    }
} else {
    "too young"
}
```

### While Loops

```script
// Basic while loop
let count = 0
while count < 5 {
    print("Count: " + count)
    count = count + 1
}

// While with break
let number = 1
while true {
    print(number)
    number = number * 2
    if number > 100 {
        break
    }
}
```

### For Loops

```script
// For loop with range
for i in 0..5 {
    print("Iteration: " + i)
}

// For loop with collection (covered in Collections section)
let numbers = [1, 2, 3, 4, 5]
for num in numbers {
    print("Number: " + num)
}
```

### Loop Examples

```script
// Find first even number greater than 10
fn find_even_above_ten() -> i32 {
    let num = 11
    while true {
        if is_even(num) {
            return num
        }
        num = num + 1
    }
}

// Count down
fn countdown(from: i32) {
    let current = from
    while current > 0 {
        print(current)
        current = current - 1
    }
    print("Blast off!")
}

countdown(5)
```

## Collections

Script provides powerful collection types for organizing data:

### Arrays (Lists)

```script
// Create arrays
let numbers = [1, 2, 3, 4, 5]
let names = ["Alice", "Bob", "Charlie"]
let mixed = [1, "hello", true]  // Mixed types allowed

// Access elements
let first = numbers[0]  // 1
let second = names[1]   // "Bob"

// Array operations using Vec functions
let vec = Vec::new()
vec_push(vec, 10)
vec_push(vec, 20)
vec_push(vec, 30)

let length = vec_len(vec)  // 3
let element = vec_get(vec, 1)  // Some(20)

// Remove elements
let popped = vec_pop(vec)  // Some(30)
```

### Dictionaries (HashMaps)

```script
// Create a dictionary
let person = HashMap::new()
hashmap_insert(person, "name", "Alice")
hashmap_insert(person, "age", 30)
hashmap_insert(person, "city", "New York")

// Access values
let name = hashmap_get(person, "name")  // Some("Alice")
let age = hashmap_get(person, "age")    // Some(30)

// Check if key exists
if hashmap_contains_key(person, "email") {
    print("Email found")
} else {
    print("No email address")
}
```

### Working with Collections

```script
// Process a list of scores
fn calculate_average(scores: Vec<i32>) -> f32 {
    let total = 0
    let count = vec_len(scores)
    
    for i in 0..count {
        let score = vec_get(scores, i)
        match score {
            Some(value) => total = total + value,
            None => continue
        }
    }
    
    total as f32 / count as f32
}

// Create a simple database
fn create_user_database() -> HashMap<string, any> {
    let db = HashMap::new()
    
    // Add users
    hashmap_insert(db, "user1", "Alice Johnson")
    hashmap_insert(db, "user2", "Bob Smith")
    hashmap_insert(db, "user3", "Charlie Brown")
    
    db
}

// Use the functions
let test_scores = Vec::new()
vec_push(test_scores, 85)
vec_push(test_scores, 92)
vec_push(test_scores, 78)
vec_push(test_scores, 96)

let average = calculate_average(test_scores)
print("Average score: " + average)

let users = create_user_database()
let user = hashmap_get(users, "user1")
match user {
    Some(name) => print("Found user: " + name),
    None => print("User not found")
}
```

## Error Handling

Script uses Result and Option types for safe error handling:

### Option Type

The Option type represents a value that might be present or absent:

```script
// Creating Options
let some_value = Option::some(42)
let no_value = Option::none()

// Checking Options
if is_some(some_value) {
    let value = option_unwrap(some_value)
    print("Value: " + value)
}

if is_none(no_value) {
    print("No value present")
}

// Safe function that might fail
fn divide(a: f32, b: f32) -> Option<f32> {
    if b == 0.0 {
        Option::none()
    } else {
        Option::some(a / b)
    }
}

let result = divide(10.0, 3.0)
match result {
    Some(value) => print("Result: " + value),
    None => print("Cannot divide by zero!")
}
```

### Result Type

The Result type represents either success or failure:

```script
// Creating Results
let success = Result::ok("File loaded successfully")
let failure = Result::err("File not found")

// Checking Results
if is_ok(success) {
    let message = result_unwrap(success)
    print(message)
}

if is_err(failure) {
    let error = unwrap_err(failure)
    print("Error: " + error)
}

// Safe parsing function
fn parse_number(text: string) -> Result<i32, string> {
    // This is a simplified example
    // In reality, you'd implement proper parsing
    if text == "42" {
        Result::ok(42)
    } else if text == "100" {
        Result::ok(100)
    } else {
        Result::err("Not a valid number")
    }
}

let num_result = parse_number("42")
match num_result {
    Ok(number) => print("Parsed: " + number),
    Err(error) => print("Parse error: " + error)
}
```

### Error Handling Best Practices

```script
// Always handle Results properly
fn safe_file_operation(filename: string) {
    let contents = read_file(filename)
    match contents {
        Ok(text) => {
            print("File contents: " + text)
            
            // Chain operations safely
            let write_result = write_file("backup.txt", text)
            match write_result {
                Ok(()) => print("Backup created"),
                Err(error) => print("Backup failed: " + error)
            }
        },
        Err(error) => {
            print("Could not read file: " + error)
        }
    }
}

// Helper function for safe operations
fn safe_divide_and_display(a: f32, b: f32) {
    let result = divide(a, b)
    match result {
        Some(value) => print(a + " / " + b + " = " + value),
        None => print("Cannot divide " + a + " by " + b + " (division by zero)")
    }
}

safe_divide_and_display(10.0, 2.0)  // Works
safe_divide_and_display(10.0, 0.0)  // Handled gracefully
```

## Working with Files

Script makes file operations safe and straightforward:

### Reading Files

```script
// Read entire file
fn read_and_display(filename: string) {
    let contents = read_file(filename)
    match contents {
        Ok(text) => {
            print("File '" + filename + "' contains:")
            print(text)
        },
        Err(error) => {
            print("Error reading '" + filename + "': " + error)
        }
    }
}

read_and_display("config.txt")
```

### Writing Files

```script
// Write to file
fn save_user_data(name: string, score: i32) {
    let data = "Player: " + name + "\nScore: " + score + "\n"
    let result = write_file("scores.txt", data)
    
    match result {
        Ok(()) => print("Score saved for " + name),
        Err(error) => print("Failed to save score: " + error)
    }
}

save_user_data("Alice", 1250)
```

### File Processing Examples

```script
// Process a configuration file
fn load_config() -> HashMap<string, string> {
    let config = HashMap::new()
    let contents = read_file("app.config")
    
    match contents {
        Ok(text) => {
            // Simple key=value parser
            let lines = split(text, "\n")
            for line in lines {
                if contains(line, "=") {
                    let parts = split(line, "=")
                    if vec_len(parts) >= 2 {
                        let key_opt = vec_get(parts, 0)
                        let value_opt = vec_get(parts, 1)
                        
                        match (key_opt, value_opt) {
                            (Some(key), Some(value)) => {
                                hashmap_insert(config, trim(key), trim(value))
                            },
                            _ => continue
                        }
                    }
                }
            }
        },
        Err(error) => {
            print("Warning: Could not load config: " + error)
            // Set default values
            hashmap_insert(config, "app_name", "My Script App")
            hashmap_insert(config, "version", "1.0")
        }
    }
    
    config
}

// Log system
fn log_message(level: string, message: string) {
    let timestamp = time_now()  // Get current time
    let log_line = "[" + timestamp + "] " + level + ": " + message + "\n"
    
    // Append to log file (simplified - would need append mode)
    let result = write_file("app.log", log_line)
    match result {
        Ok(()) => {},  // Success, do nothing
        Err(error) => eprintln("Failed to write log: " + error)
    }
}

// Using the file functions
let config = load_config()
let app_name = hashmap_get(config, "app_name")
match app_name {
    Some(name) => {
        print("Starting " + name)
        log_message("INFO", "Application started: " + name)
    },
    None => {
        print("Starting application")
        log_message("WARN", "No app name configured")
    }
}
```

## Putting It All Together

Let's create a complete example program that uses everything we've learned:

```script
// A simple task manager application
fn main() {
    print("=== Script Task Manager ===")
    
    let tasks = Vec::new()
    
    // Add some initial tasks
    add_task(tasks, "Learn Script programming")
    add_task(tasks, "Build a simple game")
    add_task(tasks, "Write documentation")
    
    // Display all tasks
    print("\nYour tasks:")
    display_tasks(tasks)
    
    // Mark a task as complete
    complete_task(tasks, 0)
    
    print("\nAfter completing first task:")
    display_tasks(tasks)
    
    // Save tasks to file
    save_tasks_to_file(tasks, "my_tasks.txt")
}

fn add_task(tasks: Vec<string>, description: string) {
    vec_push(tasks, description)
    print("Added task: " + description)
}

fn display_tasks(tasks: Vec<string>) {
    let count = vec_len(tasks)
    if count == 0 {
        print("No tasks found.")
        return
    }
    
    for i in 0..count {
        let task = vec_get(tasks, i)
        match task {
            Some(description) => {
                let task_num = i + 1
                print(task_num + ". " + description)
            },
            None => continue
        }
    }
}

fn complete_task(tasks: Vec<string>, index: i32) {
    let task = vec_get(tasks, index)
    match task {
        Some(description) => {
            print("Completed: " + description)
            // In a real app, we might mark it as done instead of removing
            // For simplicity, we'll just print a completion message
        },
        None => {
            print("Task not found at index " + index)
        }
    }
}

fn save_tasks_to_file(tasks: Vec<string>, filename: string) {
    let content = "=== My Tasks ===\n"
    let count = vec_len(tasks)
    
    for i in 0..count {
        let task = vec_get(tasks, i)
        match task {
            Some(description) => {
                let task_num = i + 1
                content = content + task_num + ". " + description + "\n"
            },
            None => continue
        }
    }
    
    let result = write_file(filename, content)
    match result {
        Ok(()) => print("Tasks saved to " + filename),
        Err(error) => print("Failed to save tasks: " + error)
    }
}

// Run the main function
main()
```

## Next Steps

Congratulations! You now have a solid foundation in Script programming. Here's what to explore next:

### Immediate Next Steps

1. **Practice**: Try modifying the examples in this tutorial
2. **Experiment**: Use the REPL to test ideas and learn interactively
3. **Build**: Create small programs to solve problems you encounter

### Advanced Topics

1. **[Advanced Features Tutorial](ADVANCED.md)** - Pattern matching, advanced types, metaprogramming
2. **[Game Development Guide](GAME_DEV.md)** - Using Script for game development
3. **Type System Deep Dive** - Understanding Script's gradual typing system
4. **Performance Optimization** - Writing efficient Script code

### Resources

- **Language Reference**: Complete syntax and feature documentation
- **Standard Library Guide**: Comprehensive function reference
- **Example Programs**: More complex examples to study
- **Community**: Join discussions and share your projects

### Common Patterns to Master

```script
// Error handling patterns
fn safe_operation() -> Result<string, string> {
    let step1 = risky_function1()
    match step1 {
        Ok(value1) => {
            let step2 = risky_function2(value1)
            match step2 {
                Ok(value2) => Result::ok(value2),
                Err(error) => Result::err("Step 2 failed: " + error)
            }
        },
        Err(error) => Result::err("Step 1 failed: " + error)
    }
}

// Collection processing patterns
fn process_numbers(numbers: Vec<i32>) -> Vec<i32> {
    let results = Vec::new()
    let count = vec_len(numbers)
    
    for i in 0..count {
        let num_opt = vec_get(numbers, i)
        match num_opt {
            Some(num) => {
                if num > 0 {
                    vec_push(results, num * 2)
                }
            },
            None => continue
        }
    }
    
    results
}

// Configuration and setup patterns
fn initialize_app() -> Result<AppConfig, string> {
    print("Initializing application...")
    
    let config = load_config()
    let data_dir = hashmap_get(config, "data_directory")
    
    match data_dir {
        Some(dir) => {
            // Setup would continue here
            print("Using data directory: " + dir)
            Result::ok(config)
        },
        None => {
            Result::err("No data directory configured")
        }
    }
}
```

### Tips for Continued Learning

1. **Read Error Messages**: Script provides helpful error messages - they're your friend!
2. **Use the REPL**: Perfect for experimenting with new concepts
3. **Start Small**: Build simple programs before attempting complex projects
4. **Ask Questions**: Join the community and don't hesitate to ask for help
5. **Read Code**: Study the examples and other Script programs

Happy coding with Script! 🚀

---

*This tutorial is part of the Script programming language documentation. For more information, visit the [Script GitHub repository](https://github.com/moikapy/script).*