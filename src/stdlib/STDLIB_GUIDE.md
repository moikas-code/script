# Script Standard Library Guide

The Script programming language provides a comprehensive standard library designed to be beginner-friendly while providing powerful functionality for game development and general programming.

## Table of Contents

1. [I/O Operations](#io-operations)
2. [String Manipulation](#string-manipulation)
3. [Core Types](#core-types)
4. [Collections](#collections)
5. [Integration with Script Code](#integration-with-script-code)

## I/O Operations

The I/O module provides functions for console output, console input, and file operations.

### Console Output

```script
// Print without newline
print("Hello");
print(" World"); // Output: Hello World

// Print with newline
println("Hello World");

// Print to stderr
eprintln("Error: Something went wrong!");
```

### Console Input

```script
// Read a line from stdin
let input = read_line();
match input {
    Ok(line) => println("You entered: " + line),
    Err(error) => eprintln("Failed to read: " + error)
}
```

### File Operations

```script
// Read file contents
let contents = read_file("data.txt");
match contents {
    Ok(text) => println("File contents: " + text),
    Err(error) => eprintln("Failed to read file: " + error)
}

// Write to file
let result = write_file("output.txt", "Hello, Script!");
match result {
    Ok(()) => println("File written successfully"),
    Err(error) => eprintln("Failed to write file: " + error)
}
```

## String Manipulation

Script strings are UTF-8 encoded and provide rich functionality for text processing.

### Basic Operations

```script
let s = "Hello, World!";

// Length operations
let char_count = string_len(s);  // 13 characters
let is_empty = s == "";

// Case conversion
let upper = to_uppercase(s);  // "HELLO, WORLD!"
let lower = to_lowercase(s);  // "hello, world!"

// Trimming
let trimmed = trim("  hello  ");  // "hello"
```

### String Analysis

```script
let text = "Hello, World!";

// Check contents
let has_hello = contains(text, "Hello");  // true
let starts = text.starts_with("Hello");    // true (future)
let ends = text.ends_with("!");            // true (future)
```

### String Transformation

```script
// Split string
let parts = split("one,two,three", ",");  // ["one", "two", "three"]

// Replace occurrences
let replaced = replace("Hello World", "World", "Script");  // "Hello Script"

// String concatenation (future)
let greeting = concat("Hello, ", name);
```

## Core Types

### Option Type

The Option type represents an optional value that may or may not be present.

```script
// Creating Options
let some_value = Option::some(42);
let no_value = Option::none();

// Checking Options
if is_some(some_value) {
    let value = option_unwrap(some_value);
    println("Value: " + value);
}

if is_none(no_value) {
    println("No value present");
}
```

### Result Type

The Result type is used for error handling, representing either success (Ok) or failure (Err).

```script
// Creating Results
let success = Result::ok(42);
let failure = Result::err("Something went wrong");

// Checking Results
if is_ok(success) {
    let value = result_unwrap(success);
    println("Success: " + value);
}

if is_err(failure) {
    let error = unwrap_err(failure);
    println("Error: " + error);
}

// Pattern matching (future)
match read_file("data.txt") {
    Ok(contents) => println(contents),
    Err(error) => eprintln(error)
}
```

## Collections

### Vec (Dynamic Array)

Vec is a growable array that can hold elements of any type.

```script
// Create a new vector
let vec = Vec::new();

// Add elements
vec_push(vec, 10);
vec_push(vec, 20);
vec_push(vec, 30);

// Get length
let len = vec_len(vec);  // 3

// Access elements
let elem = vec_get(vec, 1);
match elem {
    Some(value) => println("Element at index 1: " + value),
    None => println("Index out of bounds")
}

// Remove elements
let popped = vec_pop(vec);
match popped {
    Some(value) => println("Popped: " + value),
    None => println("Vector is empty")
}
```

### HashMap

HashMap is a key-value store where keys are strings and values can be any type.

```script
// Create a new HashMap
let map = HashMap::new();

// Insert key-value pairs
hashmap_insert(map, "name", "Alice");
hashmap_insert(map, "age", 30);

// Get values
let name = hashmap_get(map, "name");
match name {
    Some(value) => println("Name: " + value),
    None => println("Key not found")
}

// Check if key exists
if hashmap_contains_key(map, "age") {
    println("Age is defined");
}
```

## Integration with Script Code

The standard library is automatically available in all Script programs. Functions are called directly without imports:

```script
// Example Script program using stdlib
fn main() {
    println("Welcome to Script!");
    
    // Read user input
    print("Enter your name: ");
    let input = read_line();
    
    match input {
        Ok(name) => {
            let trimmed_name = trim(name);
            let greeting = "Hello, " + trimmed_name + "!";
            println(greeting);
            
            // Save to file
            let result = write_file("greeting.txt", greeting);
            if is_ok(result) {
                println("Greeting saved to file");
            }
        },
        Err(error) => {
            eprintln("Failed to read input: " + error);
        }
    }
}
```

## Type Safety

All stdlib functions are type-checked by Script's type system:

```script
// This will cause a type error at compile time
let result = string_len(42);  // Error: string_len expects a string, got i32

// Correct usage
let result = string_len("Hello");  // OK: returns 5
```

## Error Handling Best Practices

1. Always handle Result types:
```script
// Bad: May panic
let contents = result_unwrap(read_file("data.txt"));

// Good: Handle errors gracefully
match read_file("data.txt") {
    Ok(contents) => process_data(contents),
    Err(error) => eprintln("Error: " + error)
}
```

2. Use Option for nullable values:
```script
fn find_user(id: i32) -> Option<User> {
    if user_exists(id) {
        Option::some(get_user(id))
    } else {
        Option::none()
    }
}
```

## Performance Considerations

- Strings are reference-counted for efficient memory usage
- Collections use interior mutability for safe concurrent access
- All stdlib functions are designed to integrate with Script's runtime

## Future Enhancements

The following features are planned for future releases:

1. **Additional String Methods**: `starts_with`, `ends_with`, `substring`, `char_at`
2. **Iterator Support**: `map`, `filter`, `reduce` for collections
3. **More Collection Types**: `Set`, `Deque`, `BinaryHeap`
4. **Async I/O**: Non-blocking file and network operations
5. **Regular Expressions**: Pattern matching and text processing
6. **JSON Support**: Parsing and serialization
7. **Date/Time**: Comprehensive time handling
8. **Math Library**: Extended mathematical functions

## Examples

### Word Counter
```script
fn count_words(text: string) -> i32 {
    let words = split(text, " ");
    vec_len(words)
}
```

### File Processor
```script
fn process_file(input_path: string, output_path: string) -> Result<(), string> {
    match read_file(input_path) {
        Ok(contents) => {
            let processed = to_uppercase(contents);
            write_file(output_path, processed)
        },
        Err(error) => Result::err(error)
    }
}
```

### Simple Database
```script
fn create_database() -> HashMap<string, any> {
    let db = HashMap::new();
    
    hashmap_insert(db, "users", Vec::new());
    hashmap_insert(db, "products", Vec::new());
    
    db
}

fn add_user(db: HashMap<string, any>, name: string) {
    match hashmap_get(db, "users") {
        Some(users) => vec_push(users, name),
        None => eprintln("Users collection not found")
    }
}
```

This standard library provides a solid foundation for Script programming while maintaining simplicity and safety. As the language evolves, the stdlib will grow to support more advanced use cases while preserving backward compatibility.