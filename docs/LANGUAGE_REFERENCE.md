# Script Language Reference

This document provides a complete specification of the Script programming language, including syntax, semantics, type system, and built-in features.

## Table of Contents

1. [Lexical Structure](#lexical-structure)
2. [Grammar](#grammar)
3. [Type System](#type-system)
4. [Expressions](#expressions)
5. [Statements](#statements)
6. [Functions](#functions)
7. [Pattern Matching](#pattern-matching)
8. [Modules and Imports](#modules-and-imports)
9. [Async/Await](#asyncawait)
10. [Metaprogramming](#metaprogramming)
11. [Memory Model](#memory-model)
12. [Standard Library](#standard-library)
13. [Error Handling](#error-handling)
14. [Attributes](#attributes)

## Lexical Structure

### Character Set

Script source files are UTF-8 encoded. The language supports:
- ASCII letters: `a-z`, `A-Z`
- Digits: `0-9`
- Unicode identifiers (following UAX #31)
- Whitespace: space, tab, newline, carriage return

### Comments

```ebnf
line_comment    = "//" ~newline* newline
block_comment   = "/*" ~"*/"* "*/"
```

### Identifiers

```ebnf
identifier = (letter | "_") (letter | digit | "_")*
letter     = "a".."z" | "A".."Z" | unicode_letter
digit      = "0".."9"
```

### Keywords

Reserved keywords that cannot be used as identifiers:

```
async     await     break     const     continue  else
export    false     fn        for       from      if
import    in        let       match     mod       mut
return    self      struct    true      type      while
```

### Literals

#### Integer Literals
```ebnf
integer_literal = decimal_literal | hex_literal | binary_literal
decimal_literal = digit+
hex_literal     = "0x" hex_digit+
binary_literal  = "0b" ("0" | "1")+
```

#### Floating-Point Literals
```ebnf
float_literal = digit+ "." digit+ ([eE] [+-]? digit+)?
```

#### String Literals
```ebnf
string_literal = '"' string_char* '"'
string_char    = ~["\\\n] | escape_sequence
escape_sequence = "\\" ("n" | "r" | "t" | "0" | "\\" | '"' | "x" hex_digit{2})
```

#### Boolean Literals
```ebnf
boolean_literal = "true" | "false"
```

### Operators and Punctuation

```
Arithmetic: + - * / %
Comparison: == != < > <= >=
Logical:    && || !
Bitwise:    & | ^ ~ << >>
Assignment: = += -= *= /= %= &= |= ^= <<= >>=
Other:      . .. -> => ? : :: ; , ( ) [ ] { }
```

## Grammar

### Program Structure

```ebnf
program = item*

item = function_def
     | const_def
     | type_def
     | import_stmt
     | export_stmt
```

### Statements

```ebnf
statement = let_stmt
          | expression_stmt
          | return_stmt
          | if_stmt
          | while_stmt
          | for_stmt
          | break_stmt
          | continue_stmt
          | block_stmt

let_stmt       = "let" pattern type_ann? "=" expression
expression_stmt = expression ";"
return_stmt    = "return" expression?
break_stmt     = "break"
continue_stmt  = "continue"
block_stmt     = "{" statement* expression? "}"
```

### Expressions

```ebnf
expression = assignment_expr

assignment_expr = logical_or_expr (assign_op logical_or_expr)?
assign_op      = "=" | "+=" | "-=" | "*=" | "/=" | "%="

logical_or_expr  = logical_and_expr ("||" logical_and_expr)*
logical_and_expr = equality_expr ("&&" equality_expr)*
equality_expr    = comparison_expr (("==" | "!=") comparison_expr)*
comparison_expr  = additive_expr (("<" | ">" | "<=" | ">=") additive_expr)*
additive_expr    = multiplicative_expr (("+" | "-") multiplicative_expr)*
multiplicative_expr = unary_expr (("*" | "/" | "%") unary_expr)*

unary_expr = ("!" | "-") unary_expr | postfix_expr

postfix_expr = primary_expr
             | postfix_expr "[" expression "]"       // Index
             | postfix_expr "." identifier           // Member access
             | postfix_expr "(" argument_list ")"    // Function call
             | "await" postfix_expr                  // Await

primary_expr = identifier
             | literal
             | "(" expression ")"
             | array_expr
             | if_expr
             | match_expr
             | block_expr
             | async_block
```

## Type System

### Type Syntax

```ebnf
type = named_type
     | array_type
     | function_type
     | result_type
     | option_type

named_type    = identifier type_args?
array_type    = "Array" "<" type ">"
function_type = "fn" "(" type_list? ")" "->" type
result_type   = "Result" "<" type "," type ">"
option_type   = "Option" "<" type ">"
type_args     = "<" type_list ">"
type_list     = type ("," type)*
```

### Built-in Types

#### Primitive Types
- `i32`: 32-bit signed integer (-2,147,483,648 to 2,147,483,647)
- `f32`: 32-bit IEEE 754 floating-point number
- `bool`: Boolean value (true or false)
- `String`: UTF-8 encoded string (heap-allocated)

#### Composite Types
- `Array<T>`: Dynamic array of elements of type T
- `Result<T, E>`: Success value of type T or error of type E
- `Option<T>`: Optional value of type T or None
- `Future<T>`: Asynchronous computation yielding type T

### Type Inference

Script uses Hindley-Milner type inference with extensions for gradual typing:

```script
// Type is inferred as i32
let x = 42

// Type is inferred as Array<String>
let names = ["Alice", "Bob"]

// Function type is inferred
let add = |a, b| a + b  // (i32, i32) -> i32

// Gradual typing - mix typed and untyped
fn process(data) -> i32 {  // 'data' has unknown type
    data.length()  // Works if data has 'length' method
}
```

### Type Compatibility

Script uses structural typing for compatibility:

```script
// These types are compatible if they have the same structure
type Point2D = { x: f32, y: f32 }
type Vector2D = { x: f32, y: f32 }

// Function accepting either
fn distance(p: Point2D) -> f32 { ... }
let v: Vector2D = { x: 3.0, y: 4.0 }
distance(v)  // OK - structurally compatible
```

## Expressions

### Literal Expressions

```script
42              // i32 literal
3.14            // f32 literal
true            // bool literal
"hello"         // String literal
[1, 2, 3]       // Array literal
```

### Arithmetic Expressions

```script
a + b   // Addition
a - b   // Subtraction
a * b   // Multiplication
a / b   // Division
a % b   // Remainder
-a      // Negation
```

### Comparison Expressions

```script
a == b  // Equal
a != b  // Not equal
a < b   // Less than
a > b   // Greater than
a <= b  // Less than or equal
a >= b  // Greater than or equal
```

### Logical Expressions

```script
a && b  // Logical AND (short-circuiting)
a || b  // Logical OR (short-circuiting)
!a      // Logical NOT
```

### Array Expressions

```script
// Array literal
let nums = [1, 2, 3, 4, 5]

// Array indexing
let first = nums[0]

// Array slice (future feature)
let slice = nums[1..3]  // [2, 3]

// List comprehension
let squares = [x * x for x in nums]
let evens = [x for x in nums if x % 2 == 0]
```

### If Expressions

```script
// If expression returns a value
let max = if a > b { a } else { b }

// Multi-branch if
let category = if age < 13 {
    "child"
} else if age < 20 {
    "teen"
} else {
    "adult"
}
```

### Block Expressions

```script
// Block evaluates to last expression
let result = {
    let x = compute_x()
    let y = compute_y()
    x + y  // Block's value
}
```

### Function Call Expressions

```script
// Regular function call
let result = add(5, 3)

// Method call (future feature)
let length = string.length()

// Chained calls
let processed = data
    .filter(|x| x > 0)
    .map(|x| x * 2)
    .sum()
```

## Statements

### Let Statements

```script
// Simple binding
let name = "Script"

// With type annotation
let count: i32 = 0

// Mutable binding (future feature)
let mut score = 100

// Pattern destructuring
let [x, y] = [10, 20]
let { name, age } = person
```

### Expression Statements

Any expression followed by a semicolon becomes a statement:

```script
println("Hello");
compute_value();
x + y;  // Result is discarded
```

### Return Statements

```script
// Return with value
return 42

// Early return
fn check(x: i32) -> bool {
    if x < 0 {
        return false
    }
    // More processing...
    true
}
```

### Loop Statements

#### While Loops
```script
while condition {
    // body
}

// With break and continue
while true {
    if should_stop() {
        break
    }
    if should_skip() {
        continue
    }
    process()
}
```

#### For Loops
```script
// Range iteration
for i in 0..10 {
    println(i)
}

// Collection iteration
for item in collection {
    process(item)
}

// With index (future feature)
for (index, value) in array.enumerate() {
    println(index + ": " + value)
}
```

## Functions

### Function Definitions

```script
// Basic function
fn greet(name: String) {
    println("Hello, " + name)
}

// With return type
fn add(a: i32, b: i32) -> i32 {
    a + b
}

// Generic function (future feature)
fn identity<T>(value: T) -> T {
    value
}

// Async function
async fn fetch_data(url: String) -> Result<String> {
    let response = await http_get(url)?
    Ok(response.body())
}
```

### Function Types

```script
// Function type annotation
let operation: fn(i32, i32) -> i32 = add

// Higher-order function
fn apply_twice(f: fn(i32) -> i32, x: i32) -> i32 {
    f(f(x))
}

// Closures (future feature)
let multiplier = |x| x * 2
let result = apply_twice(multiplier, 5)  // 20
```

### Function Parameters

```script
// Positional parameters
fn move_to(x: f32, y: f32) { ... }

// Default parameters (future feature)
fn create_window(title: String = "Untitled", width: i32 = 800) { ... }

// Variadic parameters (future feature)
fn sum(...numbers: i32) -> i32 { ... }
```

## Pattern Matching

### Match Expressions

```script
match value {
    pattern1 -> expression1,
    pattern2 -> expression2,
    _ -> default_expression
}
```

### Pattern Types

#### Literal Patterns
```script
match x {
    0 -> "zero",
    1 -> "one",
    _ -> "many"
}
```

#### Variable Patterns
```script
match result {
    Ok(value) -> "Success: " + value,
    Err(error) -> "Error: " + error
}
```

#### Array Patterns
```script
match coords {
    [] -> "empty",
    [x] -> "single: " + x,
    [x, y] -> "pair: " + x + ", " + y,
    [x, y, ..rest] -> "many, starting with " + x
}
```

#### Object Patterns
```script
match player {
    { health: 0, .. } -> "dead",
    { health, mana: 0 } -> "no mana, health: " + health,
    { health, mana, name } -> name + " is ready"
}
```

#### Or Patterns
```script
match input {
    "y" | "yes" | "Y" -> true,
    "n" | "no" | "N" -> false,
    _ -> panic("Invalid input")
}
```

#### Guard Clauses
```script
match value {
    x if x < 0 -> "negative",
    x if x == 0 -> "zero",
    x if x > 0 -> "positive"
}
```

### Pattern Matching Rules

1. Patterns are tested in order from top to bottom
2. The first matching pattern is selected
3. All patterns must be exhaustive (cover all cases)
4. Variable bindings in patterns are immutable
5. The `_` pattern matches anything and doesn't bind

## Modules and Imports

### Module System

Script uses a file-based module system where each file is a module:

```
project/
├── main.script       # Main module
├── math/
│   ├── mod.script   # math module index
│   ├── vector.script # math::vector submodule
│   └── matrix.script # math::matrix submodule
└── utils.script      # utils module
```

### Import Statements

```script
// Import specific items
import { Vec2, Vec3 } from "./math/vector"

// Import with renaming
import { calculate as calc } from "./utils"

// Import all items
import * as math from "./math/mod"

// Import from packages
import { HashMap } from "std::collections"

// Re-export
export { Vec2, Vec3 } from "./vector"
```

### Export Statements

```script
// Export a function
export fn public_function() { ... }

// Export a constant
export const VERSION = "1.0.0"

// Export a type (future feature)
export type Point = { x: f32, y: f32 }

// Private by default
fn private_function() { ... }
```

### Module Resolution

1. Relative imports (`./path` or `../path`) resolve relative to current file
2. Package imports (bare specifiers) resolve from `script_modules/` or global cache
3. Standard library imports use `std::` prefix

## Async/Await

### Async Functions

```script
// Async function declaration
async fn fetch_user(id: i32) -> Result<User> {
    let response = await http_get("/api/users/" + id)?
    let user = await response.json()?
    Ok(user)
}

// Async expression
let future = async {
    await delay(1000)
    "Done!"
}
```

### Await Expressions

```script
// Basic await
let result = await some_async_function()

// Await with error handling
let data = await fetch_data().unwrap_or_default()

// Concurrent execution
let [a, b, c] = await all([
    async_task_a(),
    async_task_b(),
    async_task_c()
])
```

### Future Combinators

```script
// Transform future result
let doubled = future.map(|x| x * 2)

// Chain futures
let chained = future
    .and_then(|x| another_async_func(x))
    .map_err(|e| "Error: " + e)

// Race multiple futures
let first = race([future1, future2, future3])
```

## Metaprogramming

### Attributes

```script
// Test attribute
@test
fn test_addition() {
    assert_eq(2 + 2, 4)
}

// Derive attribute
@derive(Debug, Serialize)
struct Point {
    x: f32,
    y: f32
}

// Const function
@const
fn compile_time_calculation() -> i32 {
    // Evaluated at compile time
    100 * 50
}

// Documentation
/// Calculates the area of a circle
/// @param radius The radius of the circle
/// @returns The area
fn area_circle(radius: f32) -> f32 {
    PI * radius * radius
}
```

### List Comprehensions

```script
// Basic comprehension
let squares = [x * x for x in 0..10]

// With condition
let evens = [x for x in numbers if x % 2 == 0]

// Nested comprehension
let pairs = [(x, y) for x in 0..3 for y in 0..3]

// With pattern matching
let names = [person.name for person in people if person.age >= 18]
```

## Memory Model

### Reference Counting

Script uses automatic reference counting (ARC) for memory management:

```script
// Values are reference counted
let a = [1, 2, 3]  // RC = 1
let b = a          // RC = 2 (shared reference)
// When a and b go out of scope, RC = 0 and memory is freed
```

### Ownership Rules

1. Each value has a single owner
2. Values can be borrowed immutably (shared references)
3. Values can be borrowed mutably (exclusive reference)
4. References cannot outlive their values

### Cycle Detection

Script includes automatic cycle detection to prevent memory leaks:

```script
// Circular references are detected and cleaned up
let node1 = Node { next: None }
let node2 = Node { next: Some(node1) }
node1.next = Some(node2)  // Cycle detected and handled
```

## Standard Library

### Core Module (`std::core`)

```script
// I/O functions
print(message: String)
println(message: String)
eprintln(message: String)
read_line() -> String

// Type conversions
to_string(value: any) -> String
parse_i32(s: String) -> Result<i32>
parse_f32(s: String) -> Result<f32>
```

### Collections Module (`std::collections`)

```script
// Array methods
array.len() -> i32
array.push(item: T)
array.pop() -> Option<T>
array.get(index: i32) -> Option<T>
array.map<U>(f: fn(T) -> U) -> Array<U>
array.filter(f: fn(T) -> bool) -> Array<T>
array.reduce<U>(init: U, f: fn(U, T) -> U) -> U

// HashMap (future feature)
HashMap<K, V>::new() -> HashMap<K, V>
map.insert(key: K, value: V)
map.get(key: K) -> Option<V>
map.remove(key: K) -> Option<V>
```

### Math Module (`std::math`)

```script
// Constants
const PI: f32 = 3.14159265359
const E: f32 = 2.71828182846

// Functions
abs(x: f32) -> f32
sqrt(x: f32) -> f32
pow(base: f32, exp: f32) -> f32
sin(x: f32) -> f32
cos(x: f32) -> f32
tan(x: f32) -> f32
min(a: f32, b: f32) -> f32
max(a: f32, b: f32) -> f32
clamp(value: f32, min: f32, max: f32) -> f32

// Vector types
Vec2 { x: f32, y: f32 }
Vec3 { x: f32, y: f32, z: f32 }
Vec4 { x: f32, y: f32, z: f32, w: f32 }
Mat4 { ... }  // 4x4 matrix
```

### Random Module (`std::random`)

```script
Random::new() -> Random
Random::with_seed(seed: i32) -> Random
rng.i32() -> i32
rng.f32() -> f32  // 0.0 to 1.0
rng.range(min: i32, max: i32) -> i32
rng.choose<T>(array: Array<T>) -> T
```

### Time Module (`std::time`)

```script
now() -> f32  // Seconds since program start
sleep(seconds: f32)
Timer::new() -> Timer
timer.elapsed() -> f32
timer.delta() -> f32
timer.reset()
```

## Error Handling

### Result Type

```script
enum Result<T, E> {
    Ok(T),
    Err(E)
}

// Result methods
result.is_ok() -> bool
result.is_err() -> bool
result.unwrap() -> T  // Panics if Err
result.unwrap_or(default: T) -> T
result.map<U>(f: fn(T) -> U) -> Result<U, E>
result.map_err<F>(f: fn(E) -> F) -> Result<T, F>
```

### Option Type

```script
enum Option<T> {
    Some(T),
    None
}

// Option methods
option.is_some() -> bool
option.is_none() -> bool
option.unwrap() -> T  // Panics if None
option.unwrap_or(default: T) -> T
option.map<U>(f: fn(T) -> U) -> Option<U>
option.and_then<U>(f: fn(T) -> Option<U>) -> Option<U>
```

### Error Propagation

```script
// Using ? operator
fn process() -> Result<String> {
    let data = read_file("data.txt")?  // Returns early if error
    let parsed = parse_data(data)?
    Ok(format_output(parsed))
}

// Manual error handling
fn safe_divide(a: f32, b: f32) -> Result<f32> {
    if b == 0.0 {
        Err("Division by zero")
    } else {
        Ok(a / b)
    }
}
```

### Panic

```script
// Assertions
assert(condition: bool)
assert(condition: bool, message: String)
assert_eq(a: T, b: T)
assert_ne(a: T, b: T)

// Explicit panic
panic(message: String)

// Unreachable code marker
unreachable()
```

## Attributes

### Built-in Attributes

#### `@test`
Marks a function as a test case:
```script
@test
fn test_example() {
    assert_eq(1 + 1, 2)
}
```

#### `@derive`
Automatically derives trait implementations:
```script
@derive(Debug, Clone, PartialEq)
struct Point { x: f32, y: f32 }
```

#### `@const`
Marks a function for compile-time evaluation:
```script
@const
fn factorial(n: i32) -> i32 {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}
```

#### `@deprecated`
Marks an item as deprecated:
```script
@deprecated("Use new_function instead")
fn old_function() { ... }
```

#### `@doc`
Documentation attribute (alternative to `///`):
```script
@doc("Calculates the square root")
@doc("Returns NaN for negative inputs")
fn sqrt(x: f32) -> f32 { ... }
```

### Custom Attributes (Future Feature)

```script
// Define custom attribute
@attribute
fn memoize(f: Function) -> Function {
    // Implementation
}

// Use custom attribute
@memoize
fn expensive_calculation(n: i32) -> i32 {
    // Will be automatically memoized
}
```

## Appendix: Operator Precedence

From highest to lowest precedence:

1. Postfix: `()` `[]` `.`
2. Unary: `!` `-` `await`
3. Multiplicative: `*` `/` `%`
4. Additive: `+` `-`
5. Comparison: `<` `>` `<=` `>=`
6. Equality: `==` `!=`
7. Logical AND: `&&`
8. Logical OR: `||`
9. Assignment: `=` `+=` `-=` etc.

## Appendix: Grammar Summary

```ebnf
program         = item*
item            = function | const | type | import | export

function        = "fn" identifier "(" params? ")" type? block
const           = "const" identifier type? "=" expression
import          = "import" import_spec "from" string
export          = "export" (function | const | "{" identifier+ "}")

statement       = let_stmt | expr_stmt | return_stmt | if_stmt | while_stmt | for_stmt | block
expression      = literal | identifier | binary | unary | call | if_expr | match_expr | block_expr

pattern         = "_" | identifier | literal | array_pattern | object_pattern | or_pattern
match_arm       = pattern guard? "->" expression

type            = identifier | array_type | function_type | result_type
```

---

This language reference serves as the authoritative specification for the Script programming language v1.0.