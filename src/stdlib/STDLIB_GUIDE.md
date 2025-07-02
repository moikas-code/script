# Script Standard Library Reference

The Script programming language provides a comprehensive standard library designed to be beginner-friendly while providing powerful functionality for game development and general programming. This reference provides complete API documentation, performance characteristics, and practical usage examples.

## Table of Contents

1. [I/O Operations](#io-operations)
2. [String Manipulation](#string-manipulation)
3. [Core Types](#core-types)
4. [Collections](#collections)
5. [Mathematics](#mathematics)
6. [Game Development](#game-development)
7. [Random Number Generation](#random-number-generation)
8. [Time and Date](#time-and-date)
9. [Graphics and Colors](#graphics-and-colors)
10. [Integration with Script Code](#integration-with-script-code)
11. [Performance Characteristics](#performance-characteristics)
12. [Memory Safety](#memory-safety)

## I/O Operations

The I/O module provides functions for console output, console input, and file operations. All I/O operations are designed to be safe and handle errors gracefully.

### Console Output

#### `print(text: string) -> unit`
Prints text to stdout without a trailing newline.

**Parameters:**
- `text`: The string to print

**Performance:** O(n) where n is the length of the text
**Memory Safety:** Safe, no memory allocation for simple strings

```script
print("Hello");
print(" World"); // Output: Hello World
```

#### `println(text: string) -> unit`
Prints text to stdout with a trailing newline.

**Parameters:**
- `text`: The string to print

**Performance:** O(n) where n is the length of the text
**Memory Safety:** Safe, no memory allocation for simple strings

```script
println("Hello World"); // Output: Hello World\n
```

#### `eprintln(text: string) -> unit`
Prints text to stderr with a trailing newline. Use for error messages and diagnostics.

**Parameters:**
- `text`: The error message to print

**Performance:** O(n) where n is the length of the text
**Memory Safety:** Safe, no memory allocation for simple strings

```script
eprintln("Error: Something went wrong!");
```

### Console Input

#### `read_line() -> Result<string, string>`
Reads a line from stdin, including interactive input from the user.

**Returns:**
- `Ok(string)`: The line read from stdin (without trailing newline)
- `Err(string)`: Error message if reading failed

**Performance:** Blocking operation, depends on user input
**Memory Safety:** Safe, allocates memory for the input string

```script
// Read user input
let input = read_line();
match input {
    Ok(line) => println("You entered: " + line),
    Err(error) => eprintln("Failed to read: " + error)
}

// Example with error handling
fn get_user_name() -> Option<string> {
    print("Enter your name: ");
    match read_line() {
        Ok(name) => Some(trim(name)),
        Err(_) => None
    }
}
```

### File Operations

#### `read_file(path: string) -> Result<string, string>`
Reads the entire contents of a file as a UTF-8 string.

**Parameters:**
- `path`: Path to the file to read

**Returns:**
- `Ok(string)`: The file contents
- `Err(string)`: Error message if reading failed

**Performance:** O(n) where n is the file size
**Memory Safety:** Safe, allocates memory for file contents
**Error Conditions:** File not found, permission denied, invalid UTF-8

```script
// Read configuration file
let config = read_file("config.json");
match config {
    Ok(contents) => {
        println("Config loaded: " + string_len(contents) + " bytes");
        parse_config(contents);
    },
    Err(error) => {
        eprintln("Failed to read config: " + error);
        use_default_config();
    }
}
```

#### `write_file(path: string, contents: string) -> Result<unit, string>`
Writes a string to a file, creating the file if it doesn't exist or overwriting if it does.

**Parameters:**
- `path`: Path to the file to write
- `contents`: The string contents to write

**Returns:**
- `Ok(unit)`: Success
- `Err(string)`: Error message if writing failed

**Performance:** O(n) where n is the contents length
**Memory Safety:** Safe, no additional allocations
**Error Conditions:** Permission denied, disk full, invalid path

```script
// Save user data
let data = serialize_user_data(user);
let result = write_file("user_data.json", data);
match result {
    Ok(()) => println("Data saved successfully"),
    Err(error) => eprintln("Failed to save data: " + error)
}

// Atomic write pattern for critical data
fn save_critical_data(data: string) -> Result<unit, string> {
    // Write to temporary file first
    let temp_path = "data.tmp";
    match write_file(temp_path, data) {
        Ok(()) => {
            // Move to final location (atomic on most filesystems)
            rename_file(temp_path, "data.json")
        },
        Err(error) => Err(error)
    }
}
```

### Advanced I/O Patterns

#### Buffered Output
For high-performance output, consider buffering:

```script
fn write_large_dataset(items: Vec<string>) -> Result<unit, string> {
    let mut buffer = "";
    for item in items {
        buffer = buffer + item + "\n";
        
        // Flush buffer when it gets large
        if string_len(buffer) > 8192 {
            write_file("output.txt", buffer)?;
            buffer = "";
        }
    }
    
    // Write remaining buffer
    if string_len(buffer) > 0 {
        write_file("output.txt", buffer)?;
    }
    
    Ok(())
}
```

#### Error Recovery
Implement robust error handling for I/O operations:

```script
fn robust_file_read(path: string) -> string {
    // Try primary location
    match read_file(path) {
        Ok(contents) => contents,
        Err(_) => {
            // Try backup location
            let backup_path = path + ".backup";
            match read_file(backup_path) {
                Ok(contents) => {
                    eprintln("Warning: Using backup file");
                    contents
                },
                Err(_) => {
                    eprintln("Error: Could not read file or backup");
                    ""  // Return empty string as fallback
                }
            }
        }
    }
}
```

## String Manipulation

Script strings are UTF-8 encoded and provide rich functionality for text processing. All string operations are memory-safe and handle Unicode correctly.

### String Metrics

#### `string_len(text: string) -> i32`
Returns the number of Unicode code points in a string (not bytes).

**Parameters:**
- `text`: The string to measure

**Returns:** Number of Unicode characters

**Performance:** O(n) for Unicode strings, O(1) for ASCII-only strings
**Memory Safety:** Safe, no allocations

```script
let s = "Hello, 世界!";
let len = string_len(s);  // 9 characters (not 13 bytes)

// Empty string check
let is_empty = string_len(text) == 0;

// Length-based operations
fn truncate_if_long(text: string, max_len: i32) -> string {
    if string_len(text) > max_len {
        // Note: This is conceptual - actual substring function may vary
        substring(text, 0, max_len) + "..."
    } else {
        text
    }
}
```

### Case Conversion

#### `to_uppercase(text: string) -> string`
Converts all characters to uppercase using Unicode case mapping rules.

**Parameters:**
- `text`: The string to convert

**Returns:** New string with uppercase characters

**Performance:** O(n) where n is string length
**Memory Safety:** Safe, allocates new string

```script
let greeting = "Hello, World!";
let shouting = to_uppercase(greeting);  // "HELLO, WORLD!"

// Handles Unicode correctly
let mixed = "Café";
let upper_mixed = to_uppercase(mixed);  // "CAFÉ"
```

#### `to_lowercase(text: string) -> string`
Converts all characters to lowercase using Unicode case mapping rules.

**Parameters:**
- `text`: The string to convert

**Returns:** New string with lowercase characters

**Performance:** O(n) where n is string length
**Memory Safety:** Safe, allocates new string

```script
let title = "SCRIPT PROGRAMMING";
let normal = to_lowercase(title);  // "script programming"

// Case-insensitive comparison
fn strings_equal_ignore_case(a: string, b: string) -> bool {
    to_lowercase(a) == to_lowercase(b)
}
```

### String Cleaning

#### `trim(text: string) -> string`
Removes leading and trailing whitespace (spaces, tabs, newlines).

**Parameters:**
- `text`: The string to trim

**Returns:** New string with whitespace removed

**Performance:** O(n) where n is string length
**Memory Safety:** Safe, may allocate new string if trimming needed

```script
let padded = "  hello world  \n";
let clean = trim(padded);  // "hello world"

// Common pattern for user input
fn get_clean_input() -> Option<string> {
    match read_line() {
        Ok(input) => {
            let cleaned = trim(input);
            if string_len(cleaned) > 0 {
                Some(cleaned)
            } else {
                None
            }
        },
        Err(_) => None
    }
}
```

### String Searching

#### `contains(text: string, needle: string) -> bool`
Checks if a string contains a substring.

**Parameters:**
- `text`: The string to search in
- `needle`: The substring to search for

**Returns:** `true` if needle is found in text

**Performance:** O(nm) where n=text length, m=needle length
**Memory Safety:** Safe, no allocations

```script
let document = "The quick brown fox jumps over the lazy dog";
let has_fox = contains(document, "fox");     // true
let has_cat = contains(document, "cat");     // false

// Case-sensitive search
let case_sensitive = contains("Hello", "hello");  // false

// Multi-word search
fn contains_all_words(text: string, words: Vec<string>) -> bool {
    for word in words {
        if !contains(text, word) {
            return false;
        }
    }
    true
}
```

### String Transformation

#### `split(text: string, delimiter: string) -> Vec<string>`
Splits a string into parts using a delimiter.

**Parameters:**
- `text`: The string to split
- `delimiter`: The string to split on

**Returns:** Vector of string parts

**Performance:** O(n) where n is text length
**Memory Safety:** Safe, allocates vector and string parts

```script
// Basic splitting
let csv = "apple,banana,cherry";
let fruits = split(csv, ",");  // ["apple", "banana", "cherry"]

// Multi-character delimiter
let data = "item1::item2::item3";
let items = split(data, "::");  // ["item1", "item2", "item3"]

// Handle empty parts
let with_empties = "a,,b,";
let parts = split(with_empties, ",");  // ["a", "", "b", ""]

// Common parsing pattern
fn parse_key_value(line: string) -> Option<(string, string)> {
    let parts = split(line, "=");
    if vec_len(parts) == 2 {
        let key = trim(vec_get(parts, 0)?);
        let value = trim(vec_get(parts, 1)?);
        Some((key, value))
    } else {
        None
    }
}
```

#### `replace(text: string, from: string, to: string) -> string`
Replaces all occurrences of a substring with another string.

**Parameters:**
- `text`: The original string
- `from`: The substring to replace
- `to`: The replacement string

**Returns:** New string with replacements made

**Performance:** O(n*m) where n=text length, m=replacement count
**Memory Safety:** Safe, allocates new string

```script
// Basic replacement
let original = "Hello World";
let updated = replace(original, "World", "Script");  // "Hello Script"

// Multiple replacements
let template = "Hello {name}, welcome to {place}!";
let message = replace(replace(template, "{name}", "Alice"), "{place}", "Script");

// Text cleaning
fn sanitize_filename(name: string) -> string {
    let step1 = replace(name, "/", "_");
    let step2 = replace(step1, "\\", "_");
    let step3 = replace(step2, ":", "_");
    step3
}

// Remove unwanted characters
fn remove_punctuation(text: string) -> string {
    let step1 = replace(text, ".", "");
    let step2 = replace(step1, ",", "");
    let step3 = replace(step2, "!", "");
    let step4 = replace(step3, "?", "");
    step4
}
```

### Advanced String Operations

#### String Building Patterns

For efficient string construction, consider these patterns:

```script
// Efficient: Use array and join pattern (conceptual)
fn build_large_string(parts: Vec<string>) -> string {
    let mut result = "";
    for part in parts {
        result = result + part;
    }
    result
}

// Template processing
fn fill_template(template: string, values: HashMap<string, string>) -> string {
    let mut result = template;
    
    // Replace each placeholder
    for (key, value) in values {
        let placeholder = "{" + key + "}";
        result = replace(result, placeholder, value);
    }
    
    result
}
```

#### Text Processing

```script
// Word counting
fn count_words(text: string) -> i32 {
    let words = split(trim(text), " ");
    // Filter out empty strings
    let mut count = 0;
    for word in words {
        if string_len(trim(word)) > 0 {
            count = count + 1;
        }
    }
    count
}

// Line processing
fn process_lines(text: string) -> Vec<string> {
    let lines = split(text, "\n");
    let mut processed = Vec::new();
    
    for line in lines {
        let clean_line = trim(line);
        if string_len(clean_line) > 0 && !contains(clean_line, "#") {
            vec_push(processed, clean_line);
        }
    }
    
    processed
}

// CSV parsing (simplified)
fn parse_csv_line(line: string) -> Vec<string> {
    let parts = split(line, ",");
    let mut result = Vec::new();
    
    for part in parts {
        vec_push(result, trim(part));
    }
    
    result
}
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

## Mathematics

Script provides a comprehensive set of mathematical functions for numerical computation, graphics programming, and game development.

### Basic Operations

#### `abs(x: f32) -> f32`
Returns the absolute value of a number.

**Parameters:**
- `x`: The input number

**Returns:** Absolute value of x

**Performance:** O(1), very fast
**Memory Safety:** Safe, no allocations

```script
let negative = -5.5;
let positive = abs(negative);  // 5.5

// Distance calculation
fn distance_1d(a: f32, b: f32) -> f32 {
    abs(a - b)
}
```

#### `min(a: f32, b: f32) -> f32`
Returns the smaller of two numbers.

**Parameters:**
- `a`: First number
- `b`: Second number

**Returns:** The smaller value

```script
let smaller = min(10.0, 5.0);  // 5.0

// Clamp to maximum
fn clamp_max(value: f32, maximum: f32) -> f32 {
    min(value, maximum)
}
```

#### `max(a: f32, b: f32) -> f32`
Returns the larger of two numbers.

**Parameters:**
- `a`: First number  
- `b`: Second number

**Returns:** The larger value

```script
let larger = max(10.0, 5.0);  // 10.0

// Clamp to minimum
fn clamp_min(value: f32, minimum: f32) -> f32 {
    max(value, minimum)
}
```

#### `sign(x: f32) -> f32`
Returns the sign of a number (-1, 0, or 1).

**Parameters:**
- `x`: The input number

**Returns:** -1.0 if negative, 0.0 if zero, 1.0 if positive

```script
let direction = sign(-3.14);  // -1.0

// Normalize velocity direction
fn get_direction(velocity: f32) -> f32 {
    sign(velocity)
}
```

### Power and Root Functions

#### `pow(base: f32, exponent: f32) -> f32`
Raises a number to a power.

**Parameters:**
- `base`: The base number
- `exponent`: The power to raise to

**Returns:** base^exponent

```script
let squared = pow(4.0, 2.0);     // 16.0
let cubed = pow(2.0, 3.0);       // 8.0
let fractional = pow(9.0, 0.5);  // 3.0 (square root)
```

#### `sqrt(x: f32) -> f32`
Returns the square root of a number.

**Parameters:**
- `x`: The input number (must be non-negative)

**Returns:** Square root of x

```script
let root = sqrt(16.0);  // 4.0

// Pythagorean theorem
fn hypotenuse(a: f32, b: f32) -> f32 {
    sqrt(a * a + b * b)
}
```

#### `cbrt(x: f32) -> f32`
Returns the cube root of a number.

**Parameters:**
- `x`: The input number

**Returns:** Cube root of x

```script
let cube_root = cbrt(27.0);  // 3.0
```

### Exponential and Logarithmic Functions

#### `exp(x: f32) -> f32`
Returns e raised to the power of x.

**Parameters:**
- `x`: The exponent

**Returns:** e^x

```script
let e_squared = exp(2.0);  // ~7.389
```

#### `log(x: f32) -> f32`
Returns the natural logarithm of x.

**Parameters:**
- `x`: The input number (must be positive)

**Returns:** ln(x)

```script
let natural_log = log(2.718281828);  // ~1.0
```

#### `log10(x: f32) -> f32`
Returns the base-10 logarithm of x.

**Parameters:**
- `x`: The input number (must be positive)

**Returns:** log₁₀(x)

```script
let log_ten = log10(100.0);  // 2.0
```

#### `log2(x: f32) -> f32`
Returns the base-2 logarithm of x.

**Parameters:**
- `x`: The input number (must be positive)

**Returns:** log₂(x)

```script
let log_two = log2(8.0);  // 3.0
```

### Trigonometric Functions

#### `sin(x: f32) -> f32`
Returns the sine of x (in radians).

**Parameters:**
- `x`: Angle in radians

**Returns:** sin(x)

```script
let sine_wave = sin(3.14159 / 2.0);  // ~1.0
```

#### `cos(x: f32) -> f32`
Returns the cosine of x (in radians).

**Parameters:**
- `x`: Angle in radians

**Returns:** cos(x)

```script
let cosine_wave = cos(0.0);  // 1.0
```

#### `tan(x: f32) -> f32`
Returns the tangent of x (in radians).

**Parameters:**
- `x`: Angle in radians

**Returns:** tan(x)

```script
let tangent = tan(3.14159 / 4.0);  // ~1.0
```

#### `asin(x: f32) -> f32`
Returns the arcsine of x (result in radians).

**Parameters:**
- `x`: Input value (-1 ≤ x ≤ 1)

**Returns:** arcsin(x) in radians

```script
let angle = asin(1.0);  // π/2
```

#### `acos(x: f32) -> f32`
Returns the arccosine of x (result in radians).

**Parameters:**
- `x`: Input value (-1 ≤ x ≤ 1)

**Returns:** arccos(x) in radians

```script
let angle = acos(0.0);  // π/2
```

#### `atan(x: f32) -> f32`
Returns the arctangent of x (result in radians).

**Parameters:**
- `x`: Input value

**Returns:** arctan(x) in radians

```script
let angle = atan(1.0);  // π/4
```

#### `atan2(y: f32, x: f32) -> f32`
Returns the arctangent of y/x, handling all quadrants correctly.

**Parameters:**
- `y`: Y coordinate
- `x`: X coordinate

**Returns:** Angle in radians from -π to π

```script
let angle = atan2(1.0, 1.0);  // π/4

// Get angle from origin to point
fn angle_to_point(x: f32, y: f32) -> f32 {
    atan2(y, x)
}
```

### Hyperbolic Functions

#### `sinh(x: f32) -> f32`
Returns the hyperbolic sine of x.

#### `cosh(x: f32) -> f32`  
Returns the hyperbolic cosine of x.

#### `tanh(x: f32) -> f32`
Returns the hyperbolic tangent of x.

### Rounding Functions

#### `floor(x: f32) -> f32`
Returns the largest integer ≤ x.

**Parameters:**
- `x`: Input number

**Returns:** Floor of x

```script
let floored = floor(3.7);   // 3.0
let negative = floor(-3.7); // -4.0
```

#### `ceil(x: f32) -> f32`
Returns the smallest integer ≥ x.

**Parameters:**
- `x`: Input number

**Returns:** Ceiling of x

```script
let ceiling = ceil(3.2);   // 4.0
let negative = ceil(-3.2); // -3.0
```

#### `round(x: f32) -> f32`
Returns x rounded to the nearest integer.

**Parameters:**
- `x`: Input number

**Returns:** Rounded value

```script
let rounded = round(3.6);  // 4.0
let exact = round(3.5);    // 4.0 (rounds half up)
```

#### `trunc(x: f32) -> f32`
Returns the integer part of x (truncates towards zero).

**Parameters:**
- `x`: Input number

**Returns:** Truncated value

```script
let truncated = trunc(3.7);   // 3.0
let negative = trunc(-3.7);   // -3.0
```

## Game Development

Script provides specialized functions and types optimized for game development, including vector math, interpolation, and utility functions.

### Vector Types

#### `vec2(x: f32, y: f32) -> Object`
Creates a 2D vector.

**Parameters:**
- `x`: X component
- `y`: Y component

**Returns:** 2D vector object with x and y fields

```script
let position = vec2(10.0, 20.0);
let velocity = vec2(-5.0, 0.0);

// Access components
let x_pos = position.x;  // 10.0
let y_pos = position.y;  // 20.0
```

#### `vec3(x: f32, y: f32, z: f32) -> Object`
Creates a 3D vector.

**Parameters:**
- `x`: X component
- `y`: Y component  
- `z`: Z component

**Returns:** 3D vector object with x, y, and z fields

```script
let position_3d = vec3(1.0, 2.0, 3.0);
let color_rgb = vec3(0.8, 0.6, 0.2);
```

#### `vec4(x: f32, y: f32, z: f32, w: f32) -> Object`
Creates a 4D vector.

**Parameters:**
- `x`: X component
- `y`: Y component
- `z`: Z component
- `w`: W component

**Returns:** 4D vector object with x, y, z, and w fields

```script
let quaternion = vec4(0.0, 0.0, 0.0, 1.0);
let color_rgba = vec4(1.0, 0.5, 0.0, 0.8);
```

### Vector Operations

#### `vec2_add(a: Object, b: Object) -> Object`
Adds two 2D vectors.

**Parameters:**
- `a`: First vector
- `b`: Second vector

**Returns:** Sum of the vectors

```script
let a = vec2(1.0, 2.0);
let b = vec2(3.0, 4.0);
let sum = vec2_add(a, b);  // vec2(4.0, 6.0)

// Movement calculation
fn update_position(position: Object, velocity: Object, delta_time: f32) -> Object {
    let scaled_velocity = vec2_scale(velocity, delta_time);
    vec2_add(position, scaled_velocity)
}
```

#### `vec2_dot(a: Object, b: Object) -> f32`
Calculates the dot product of two 2D vectors.

**Parameters:**
- `a`: First vector
- `b`: Second vector

**Returns:** Dot product (scalar value)

```script
let a = vec2(1.0, 0.0);
let b = vec2(0.0, 1.0);
let dot = vec2_dot(a, b);  // 0.0 (perpendicular vectors)

// Check if vectors point in similar direction
fn vectors_similar_direction(a: Object, b: Object) -> bool {
    vec2_dot(a, b) > 0.0
}
```

#### `vec2_length(v: Object) -> f32`
Calculates the length (magnitude) of a 2D vector.

**Parameters:**
- `v`: The vector

**Returns:** Length of the vector

```script
let velocity = vec2(3.0, 4.0);
let speed = vec2_length(velocity);  // 5.0

// Normalize a vector
fn vec2_normalize(v: Object) -> Object {
    let len = vec2_length(v);
    if len > 0.0 {
        vec2(v.x / len, v.y / len)
    } else {
        vec2(0.0, 0.0)
    }
}
```

### Interpolation Functions

#### `lerp(a: f32, b: f32, t: f32) -> f32`
Linear interpolation between two values.

**Parameters:**
- `a`: Start value
- `b`: End value
- `t`: Interpolation factor (0.0 to 1.0)

**Returns:** Interpolated value

```script
let start = 0.0;
let end = 100.0;
let halfway = lerp(start, end, 0.5);  // 50.0

// Animate a value over time
fn animate_value(from: f32, to: f32, progress: f32) -> f32 {
    lerp(from, to, progress)
}
```

#### `clamp(value: f32, min_val: f32, max_val: f32) -> f32`
Constrains a value between minimum and maximum bounds.

**Parameters:**
- `value`: The value to clamp
- `min_val`: Minimum allowed value
- `max_val`: Maximum allowed value

**Returns:** Clamped value

```script
let health = clamp(player_damage, 0.0, 100.0);
let normalized = clamp(user_input, -1.0, 1.0);

// Keep object within screen bounds
fn keep_in_bounds(pos: Object, screen_width: f32, screen_height: f32) -> Object {
    vec2(
        clamp(pos.x, 0.0, screen_width),
        clamp(pos.y, 0.0, screen_height)
    )
}
```

#### `smoothstep(edge0: f32, edge1: f32, x: f32) -> f32`
Smooth interpolation function for easing animations.

**Parameters:**
- `edge0`: Lower edge of interpolation range
- `edge1`: Upper edge of interpolation range
- `x`: Input value

**Returns:** Smoothly interpolated value (0.0 to 1.0)

```script
// Smooth fade in/out
let fade = smoothstep(0.0, 1.0, time_progress);

// Smooth distance-based effects
fn distance_effect(distance: f32, max_distance: f32) -> f32 {
    1.0 - smoothstep(0.0, max_distance, distance)
}
```

## Random Number Generation

#### `random() -> f32`
Returns a random floating-point number between 0.0 and 1.0.

**Returns:** Random float in range [0.0, 1.0)

```script
let chance = random();
if chance < 0.1 {
    spawn_rare_item();
}
```

#### `random_range(min: f32, max: f32) -> f32`
Returns a random floating-point number in the specified range.

**Parameters:**
- `min`: Minimum value (inclusive)
- `max`: Maximum value (exclusive)

**Returns:** Random float in range [min, max)

```script
let spawn_x = random_range(0.0, screen_width);
let damage = random_range(10.0, 20.0);
```

#### `random_int(min: i32, max: i32) -> i32`
Returns a random integer in the specified range.

**Parameters:**
- `min`: Minimum value (inclusive)
- `max`: Maximum value (exclusive)

**Returns:** Random integer in range [min, max)

```script
let dice_roll = random_int(1, 7);  // 1-6
let enemy_type = random_int(0, 3); // 0-2
```

## Time and Date

#### `time_now() -> f32`
Returns the current time in seconds since program start.

**Returns:** Time in seconds as floating-point

```script
let current_time = time_now();
let elapsed = current_time - start_time;

// Simple timer implementation
fn create_timer(duration: f32) -> Object {
    Timer { start_time: time_now(), duration: duration }
}

fn timer_expired(timer: Object) -> bool {
    time_now() - timer.start_time >= timer.duration
}
```

## Graphics and Colors

Script provides basic color support for graphics programming.

### Color Creation

Colors are represented as vec3 (RGB) or vec4 (RGBA) objects:

```script
// RGB colors (values 0.0 to 1.0)
let red = vec3(1.0, 0.0, 0.0);
let green = vec3(0.0, 1.0, 0.0);
let blue = vec3(0.0, 0.0, 1.0);

// RGBA colors with alpha channel
let transparent_red = vec4(1.0, 0.0, 0.0, 0.5);
let opaque_white = vec4(1.0, 1.0, 1.0, 1.0);

// Common colors
let black = vec3(0.0, 0.0, 0.0);
let white = vec3(1.0, 1.0, 1.0);
let gray = vec3(0.5, 0.5, 0.5);
```

### Utility Functions

#### `deg_to_rad(degrees: f32) -> f32`
Converts degrees to radians.

**Parameters:**
- `degrees`: Angle in degrees

**Returns:** Angle in radians

```script
let radians = deg_to_rad(90.0);  // π/2

// Rotate an object
fn rotate_object(angle_degrees: f32) {
    let radians = deg_to_rad(angle_degrees);
    apply_rotation(radians);
}
```

#### `rad_to_deg(radians: f32) -> f32`
Converts radians to degrees.

**Parameters:**
- `radians`: Angle in radians

**Returns:** Angle in degrees

```script
let degrees = rad_to_deg(3.14159);  // ~180.0

// Display angle to user
fn show_rotation(radians: f32) {
    let degrees = rad_to_deg(radians);
    println("Rotation: " + degrees + " degrees");
}
```

## Performance Characteristics

### Memory Usage

| Function Category | Memory Allocation | Notes |
|------------------|-------------------|-------|
| I/O Operations | Allocates for results | File contents, input strings |
| String Operations | May allocate new strings | Transformations create new strings |
| Core Types | Reference-counted | Shared ownership, minimal overhead |
| Collections | Dynamic allocation | Grows as needed |
| Math Functions | No allocation | Pure computation |
| Game Functions | Minimal allocation | Vector objects are small |

### Performance Guidelines

**Fast Operations (O(1)):**
- All math functions
- Vector operations
- Core type checks
- Collection length queries

**Linear Operations (O(n)):**
- String operations (proportional to string length)
- Collection iteration
- File I/O (proportional to file size)

**Potentially Expensive Operations:**
- String splitting and replacement (creates multiple allocations)
- Large file operations
- Complex string manipulations

### Optimization Tips

1. **Reuse objects when possible:**
```script
// Good: Reuse vectors
let temp_vector = vec2(0.0, 0.0);
fn update_entity(entity: Object) {
    temp_vector.x = entity.x + entity.velocity_x;
    temp_vector.y = entity.y + entity.velocity_y;
    entity.position = temp_vector;
}

// Avoid: Creating new vectors each frame
fn update_entity_slow(entity: Object) {
    entity.position = vec2(
        entity.x + entity.velocity_x,
        entity.y + entity.velocity_y
    );  // New allocation every call
}
```

2. **Batch string operations:**
```script
// Good: Build string once
let mut message = "Player stats: ";
message = message + "Health: " + health;
message = message + ", Score: " + score;

// Avoid: Multiple small concatenations in loops
```

3. **Use appropriate data structures:**
```script
// Good: Use HashMap for lookups
let entity_lookup = HashMap::new();
// Fast O(1) access

// Avoid: Linear search in Vec for frequent lookups
```

## Memory Safety

All standard library functions are designed with memory safety in mind:

### Automatic Memory Management
- All objects use reference counting
- Automatic cleanup when references are dropped
- Cycle detection prevents memory leaks

### Safe Error Handling
- I/O operations return Result types
- Out-of-bounds access returns Option types
- Math functions handle edge cases gracefully

### Thread Safety (Future)
- Current implementation is single-threaded
- Designed for future thread-safe operation
- Reference counting will use atomic operations

This standard library provides a solid foundation for Script programming while maintaining simplicity and safety. As the language evolves, the stdlib will grow to support more advanced use cases while preserving backward compatibility.