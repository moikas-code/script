# Foreign Function Interface (FFI) Guide

This guide covers Script's Foreign Function Interface (FFI) system, which allows Script code to call functions written in other languages (primarily C/C++, Rust, and system libraries) and vice versa.

## Table of Contents

- [Overview](#overview)
- [Calling C Functions from Script](#calling-c-functions-from-script)
- [Calling Script Functions from C](#calling-script-functions-from-c)
- [Rust Integration](#rust-integration)
- [Type Mapping](#type-mapping)
- [Memory Management](#memory-management)
- [Advanced FFI Patterns](#advanced-ffi-patterns)
- [Platform-Specific Considerations](#platform-specific-considerations)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Overview

Script's FFI system provides:
- **Automatic type conversion** between Script and native types
- **Memory-safe** bindings with automatic cleanup
- **Cross-platform** support for different architectures
- **Dynamic loading** of shared libraries
- **Callback support** for native code calling Script
- **Struct/object marshaling** for complex data types

## Calling C Functions from Script

### Basic C Function Binding

First, create a C library or use an existing one:

```c
// math_lib.c
#include <math.h>

double add_numbers(double a, double b) {
    return a + b;
}

double calculate_distance(double x1, double y1, double x2, double y2) {
    return sqrt((x2 - x1) * (x2 - x1) + (y2 - y1) * (y2 - y1));
}

int fibonacci(int n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

// String manipulation
char* to_uppercase(const char* str) {
    size_t len = strlen(str);
    char* result = malloc(len + 1);
    for (size_t i = 0; i < len; i++) {
        result[i] = toupper(str[i]);
    }
    result[len] = '\0';
    return result;
}
```

Compile to shared library:
```bash
# Linux/macOS
gcc -shared -fPIC -o libmath.so math_lib.c -lm

# Windows
gcc -shared -o math.dll math_lib.c -lm
```

### Load and Use in Script

```script
// Load the shared library
let lib = ffi.load("./libmath.so")  // Linux/macOS
// let lib = ffi.load("./math.dll")  // Windows

// Declare function signatures
lib.declare("add_numbers", ffi.double, [ffi.double, ffi.double])
lib.declare("calculate_distance", ffi.double, [ffi.double, ffi.double, ffi.double, ffi.double])
lib.declare("fibonacci", ffi.int, [ffi.int])
lib.declare("to_uppercase", ffi.string, [ffi.string])

// Call the functions
let sum = lib.add_numbers(5.5, 3.2)
print("Sum: " + sum)  // Sum: 8.7

let distance = lib.calculate_distance(0.0, 0.0, 3.0, 4.0)
print("Distance: " + distance)  // Distance: 5.0

let fib = lib.fibonacci(10)
print("Fibonacci(10): " + fib)  // Fibonacci(10): 55

let upper = lib.to_uppercase("hello world")
print("Uppercase: " + upper)  // Uppercase: HELLO WORLD
```

### Advanced Function Signatures

```script
// Structs and pointers
lib.declare("create_point", ffi.pointer, [ffi.double, ffi.double])
lib.declare("point_distance", ffi.double, [ffi.pointer, ffi.pointer])
lib.declare("free_point", ffi.void, [ffi.pointer])

// Arrays
lib.declare("sum_array", ffi.double, [ffi.pointer, ffi.int])
lib.declare("sort_array", ffi.void, [ffi.pointer, ffi.int])

// Callbacks
lib.declare("for_each_element", ffi.void, [ffi.pointer, ffi.int, ffi.callback])

// Using the advanced functions
let point1 = lib.create_point(1.0, 2.0)
let point2 = lib.create_point(4.0, 6.0)
let dist = lib.point_distance(point1, point2)
print("Point distance: " + dist)

// Clean up
lib.free_point(point1)
lib.free_point(point2)
```

## Calling Script Functions from C

### Embedding Script in C Applications

```c
// main.c
#include <stdio.h>
#include <script/script.h>

// Callback function that C will call
void log_message(const char* message) {
    printf("C Log: %s\n", message);
}

int main() {
    // Initialize Script runtime
    script_config_t config = script_config_default();
    script_runtime_t* runtime = script_runtime_new(&config);
    
    if (!runtime) {
        fprintf(stderr, "Failed to create Script runtime\n");
        return 1;
    }
    
    // Register C function with Script
    script_register_function(runtime, "log_message", log_message);
    
    // Execute Script code
    const char* script_code = 
        "fn greet(name: string) -> string {\n"
        "    log_message(\"Greeting: \" + name)\n"
        "    return \"Hello, \" + name + \"!\"\n"
        "}\n"
        "\n"
        "fn calculate(x: f64, y: f64) -> f64 {\n"
        "    log_message(\"Calculating: \" + x + \" + \" + y)\n"
        "    return x + y\n"
        "}\n";
    
    script_result_t result = script_execute_string(runtime, script_code);
    if (result.error) {
        fprintf(stderr, "Script error: %s\n", result.error);
        script_runtime_free(runtime);
        return 1;
    }
    
    // Call Script functions from C
    script_value_t greeting = script_call_function(runtime, "greet", 
        script_value_string("World"));
    printf("Greeting result: %s\n", script_value_as_string(greeting));
    
    script_value_t calc_result = script_call_function(runtime, "calculate",
        script_value_double(10.5), script_value_double(5.3));
    printf("Calculation result: %f\n", script_value_as_double(calc_result));
    
    // Clean up
    script_value_free(greeting);
    script_value_free(calc_result);
    script_runtime_free(runtime);
    
    return 0;
}
```

### Compilation and Linking

```bash
# Compile Script as a shared library
cd script
cargo build --release --features c-api

# Compile and link the C application
gcc -o main main.c -L./target/release -lscript -ldl -lpthread -lm
```

## Rust Integration

### Calling Rust from Script

```rust
// math_functions.rs
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_int};

#[no_mangle]
pub extern "C" fn rust_add(a: c_double, b: c_double) -> c_double {
    a + b
}

#[no_mangle]
pub extern "C" fn rust_multiply(a: c_double, b: c_double) -> c_double {
    a * b
}

#[no_mangle]
pub extern "C" fn rust_process_string(input: *const c_char) -> *mut c_char {
    let c_str = unsafe { CStr::from_ptr(input) };
    let rust_str = c_str.to_str().unwrap();
    let processed = format!("Processed: {}", rust_str.to_uppercase());
    CString::new(processed).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn rust_free_string(ptr: *mut c_char) {
    unsafe {
        if !ptr.is_null() {
            let _ = CString::from_raw(ptr);
        }
    }
}

// Complex data structures
#[repr(C)]
pub struct Point {
    pub x: c_double,
    pub y: c_double,
}

#[no_mangle]
pub extern "C" fn rust_create_point(x: c_double, y: c_double) -> *mut Point {
    Box::into_raw(Box::new(Point { x, y }))
}

#[no_mangle]
pub extern "C" fn rust_point_distance(p1: *const Point, p2: *const Point) -> c_double {
    let p1 = unsafe { &*p1 };
    let p2 = unsafe { &*p2 };
    
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    (dx * dx + dy * dy).sqrt()
}

#[no_mangle]
pub extern "C" fn rust_free_point(ptr: *mut Point) {
    unsafe {
        if !ptr.is_null() {
            let _ = Box::from_raw(ptr);
        }
    }
}
```

Build as shared library:
```bash
cargo build --release --crate-type cdylib
```

Use in Script:
```script
// Load Rust library
let rust_lib = ffi.load("./target/release/libmath_functions.so")

// Declare functions
rust_lib.declare("rust_add", ffi.double, [ffi.double, ffi.double])
rust_lib.declare("rust_multiply", ffi.double, [ffi.double, ffi.double])
rust_lib.declare("rust_process_string", ffi.string, [ffi.string])
rust_lib.declare("rust_free_string", ffi.void, [ffi.string])

// Declare struct functions
rust_lib.declare("rust_create_point", ffi.pointer, [ffi.double, ffi.double])
rust_lib.declare("rust_point_distance", ffi.double, [ffi.pointer, ffi.pointer])
rust_lib.declare("rust_free_point", ffi.void, [ffi.pointer])

// Use the functions
let sum = rust_lib.rust_add(10.5, 5.3)
let product = rust_lib.rust_multiply(4.0, 7.0)
let processed = rust_lib.rust_process_string("hello world")

print("Sum: " + sum)
print("Product: " + product)
print("Processed: " + processed)

// Work with structs
let point1 = rust_lib.rust_create_point(1.0, 2.0)
let point2 = rust_lib.rust_create_point(4.0, 6.0)
let distance = rust_lib.rust_point_distance(point1, point2)

print("Distance: " + distance)

// Clean up
rust_lib.rust_free_point(point1)
rust_lib.rust_free_point(point2)
```

### Direct Rust Integration

For tighter integration, you can use Script's native Rust API:

```rust
use script::{Runtime, RuntimeConfig, Value, NativeFunction};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = Runtime::new(RuntimeConfig::default())?;
    
    // Register Rust functions directly
    runtime.register_function("rust_fibonacci", |args| {
        let n = args[0].as_integer()? as u64;
        let result = fibonacci(n);
        Ok(Value::from(result))
    })?;
    
    runtime.register_function("rust_factorial", |args| {
        let n = args[0].as_integer()? as u64;
        let result = factorial(n);
        Ok(Value::from(result))
    })?;
    
    // Execute Script code that uses Rust functions
    let result = runtime.execute_string(r#"
        let fib10 = rust_fibonacci(10)
        let fact5 = rust_factorial(5)
        
        print("Fibonacci(10): " + fib10)
        print("Factorial(5): " + fact5)
        
        fib10 + fact5
    "#)?;
    
    println!("Result: {}", result);
    Ok(())
}

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn factorial(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        _ => n * factorial(n - 1),
    }
}
```

## Type Mapping

### Basic Type Mapping

| Script Type | C Type | Rust Type | Notes |
|-------------|--------|-----------|-------|
| `i32` | `int32_t` | `i32` | 32-bit signed integer |
| `i64` | `int64_t` | `i64` | 64-bit signed integer |
| `f32` | `float` | `f32` | 32-bit float |
| `f64` | `double` | `f64` | 64-bit float |
| `bool` | `bool` | `bool` | Boolean |
| `string` | `char*` | `*const c_char` | Null-terminated string |
| `pointer` | `void*` | `*mut c_void` | Raw pointer |
| `array` | `void*` | `*mut c_void` | Array/slice |

### Complex Type Mapping

```script
// Structs
let point_type = ffi.struct({
    x: ffi.double,
    y: ffi.double
})

// Arrays
let int_array_type = ffi.array(ffi.int, 10)

// Function pointers
let callback_type = ffi.callback(ffi.void, [ffi.int, ffi.string])

// Unions
let value_union = ffi.union({
    int_val: ffi.int,
    float_val: ffi.double,
    string_val: ffi.string
})
```

### Custom Type Conversion

```rust
// Custom type converter
use script::{Value, TypeConverter, ConversionError};

struct ColorConverter;

impl TypeConverter for ColorConverter {
    type NativeType = Color;
    
    fn to_script_value(&self, color: &Color) -> Value {
        Value::from_object([
            ("r", Value::from(color.r)),
            ("g", Value::from(color.g)),
            ("b", Value::from(color.b)),
            ("a", Value::from(color.a)),
        ])
    }
    
    fn from_script_value(&self, value: &Value) -> Result<Color, ConversionError> {
        let obj = value.as_object()?;
        Ok(Color {
            r: obj.get("r")?.as_number()? as f32,
            g: obj.get("g")?.as_number()? as f32,
            b: obj.get("b")?.as_number()? as f32,
            a: obj.get("a")?.as_number()? as f32,
        })
    }
}

// Register the converter
runtime.register_type_converter(ColorConverter)?;
```

## Memory Management

### Automatic Cleanup

Script's FFI system provides automatic cleanup for common patterns:

```script
// RAII pattern - automatically cleaned up
let file = ffi.raii_wrapper(
    lib.fopen("data.txt", "r"),  // Constructor
    lib.fclose                   // Destructor
)

// Use the file
let content = lib.fread(file.handle, 1024)
// file is automatically closed when it goes out of scope
```

### Manual Memory Management

```script
// For cases where you need manual control
let buffer = lib.malloc(1024)
defer lib.free(buffer)  // Ensure cleanup

// Use buffer
lib.memset(buffer, 0, 1024)
let data = lib.read_data(buffer, 1024)
```

### Shared Memory

```script
// Shared memory between Script and native code
let shared_buffer = ffi.shared_memory(1024)

// Pass to native function
lib.process_shared_data(shared_buffer.ptr, shared_buffer.size)

// Access from Script
shared_buffer.write_int32(0, 42)
let value = shared_buffer.read_int32(0)
```

## Advanced FFI Patterns

### Callbacks from Native Code

```c
// C function that takes a callback
typedef void (*ProcessCallback)(int index, const char* data);

void process_items(const char** items, int count, ProcessCallback callback) {
    for (int i = 0; i < count; i++) {
        callback(i, items[i]);
    }
}
```

```script
// Script callback
fn process_item(index: i32, data: string) {
    print("Item " + index + ": " + data)
}

// Register callback
let callback = ffi.callback(process_item)

// Use with native function
lib.declare("process_items", ffi.void, [ffi.pointer, ffi.int, ffi.callback])
let items = ["apple", "banana", "cherry"]
lib.process_items(items, items.length, callback)
```

### Async FFI

```script
// Async FFI calls
async fn fetch_data_async(url: string) -> string {
    let future = lib.http_get_async(url)
    return await future
}

// Use async function
let data = await fetch_data_async("https://api.example.com/data")
print("Received: " + data)
```

### Error Handling

```script
// Native functions can return error codes
let result = lib.risky_operation(42)
if result.is_error() {
    print("Error: " + result.error_message())
} else {
    print("Success: " + result.value())
}

// Exception-based error handling
try {
    let result = lib.throwing_operation(42)
    print("Result: " + result)
} catch (e) {
    print("Caught error: " + e.message)
}
```

### Thread Safety

```script
// Thread-safe FFI calls
let thread_safe_lib = ffi.load_thread_safe("./libthread_safe.so")

// Spawn multiple threads
let threads = []
for i in 0..4 {
    let thread = spawn_thread(fn() {
        let result = thread_safe_lib.thread_safe_function(i)
        print("Thread " + i + " result: " + result)
    })
    threads.push(thread)
}

// Wait for all threads
for thread in threads {
    thread.join()
}
```

## Platform-Specific Considerations

### Windows

```script
// Windows-specific library loading
let win_lib = ffi.load("kernel32.dll")
win_lib.declare("GetCurrentProcessId", ffi.uint32, [])
win_lib.declare("GetTickCount", ffi.uint32, [])

let pid = win_lib.GetCurrentProcessId()
let uptime = win_lib.GetTickCount()
```

### macOS

```script
// macOS framework loading
let cocoa = ffi.load_framework("Cocoa")
cocoa.declare("NSLog", ffi.void, [ffi.string])

cocoa.NSLog("Hello from Script!")
```

### Linux

```script
// Linux system calls
let libc = ffi.load("libc.so.6")
libc.declare("getpid", ffi.int, [])
libc.declare("gethostname", ffi.int, [ffi.pointer, ffi.size_t])

let pid = libc.getpid()
let hostname_buffer = ffi.allocate(256)
libc.gethostname(hostname_buffer, 256)
let hostname = ffi.string_from_buffer(hostname_buffer)
```

## Best Practices

### 1. Always Clean Up Resources

```script
// Good: Use RAII or defer
let file = ffi.raii_wrapper(lib.fopen("file.txt", "r"), lib.fclose)

// Or use defer
let buffer = lib.malloc(1024)
defer lib.free(buffer)
```

### 2. Validate Function Signatures

```script
// Good: Validate library functions exist
if !lib.has_function("important_function") {
    throw "Library missing required function: important_function"
}

lib.declare("important_function", ffi.int, [ffi.string])
```

### 3. Handle Errors Gracefully

```script
// Good: Check for errors
try {
    let result = lib.risky_operation(data)
    return result
} catch (e) {
    print("Failed to perform operation: " + e.message)
    return null
}
```

### 4. Use Type-Safe Wrappers

```script
// Good: Create type-safe wrappers
struct Point {
    x: f64,
    y: f64,
    
    fn new(x: f64, y: f64) -> Point {
        let ptr = lib.create_point(x, y)
        return Point { x, y, _ptr: ptr }
    }
    
    fn distance_to(self, other: Point) -> f64 {
        return lib.point_distance(self._ptr, other._ptr)
    }
    
    fn drop(self) {
        lib.free_point(self._ptr)
    }
}
```

### 5. Test FFI Boundaries

```script
// Good: Test edge cases
fn test_ffi_edge_cases() {
    // Test null pointers
    assert(lib.handle_null_pointer(null) == -1)
    
    // Test large numbers
    assert(lib.handle_large_number(2^63 - 1) != null)
    
    // Test empty strings
    assert(lib.handle_empty_string("") == "")
    
    // Test invalid parameters
    try {
        lib.invalid_operation(-1)
        assert(false, "Should have thrown")
    } catch (e) {
        // Expected
    }
}
```

## Troubleshooting

### Common Issues

1. **Library Not Found**
   ```script
   // Check library paths
   print("Library paths: " + ffi.get_library_paths())
   
   // Add custom path
   ffi.add_library_path("./custom/lib")
   ```

2. **Function Signature Mismatch**
   ```script
   // Enable debug mode to see actual vs expected signatures
   ffi.set_debug_mode(true)
   
   // This will show detailed error information
   lib.declare("problem_function", ffi.int, [ffi.string])
   ```

3. **Memory Corruption**
   ```script
   // Enable memory debugging
   ffi.enable_memory_debugging(true)
   
   // This will detect buffer overruns, double frees, etc.
   let buffer = lib.malloc(100)
   // ... use buffer ...
   lib.free(buffer)
   ```

4. **Platform Differences**
   ```script
   // Handle platform-specific differences
   let lib_name = match ffi.platform() {
       "windows" => "mylib.dll",
       "macos" => "libmylib.dylib",
       _ => "libmylib.so"
   }
   
   let lib = ffi.load(lib_name)
   ```

### Debug Tools

```script
// FFI debugging utilities
ffi.set_log_level("debug")  // Enable verbose logging
ffi.dump_loaded_libraries()  // Show all loaded libraries
ffi.dump_declared_functions()  // Show all declared functions
ffi.trace_calls(true)  // Trace all FFI calls
```

This comprehensive FFI guide provides everything needed to integrate Script with native code. The FFI system is designed to be both powerful and safe, allowing you to leverage existing libraries while maintaining Script's memory safety guarantees.