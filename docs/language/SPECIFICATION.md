# Script Language Specification

## Table of Contents

1. [Introduction](#introduction)
2. [Lexical Structure](#lexical-structure)
3. [Syntax](#syntax)
4. [Types](#types)
5. [Expressions](#expressions)
6. [Statements](#statements)
7. [Functions](#functions)
8. [Control Flow](#control-flow)
9. [Pattern Matching](#pattern-matching)
10. [Memory Model](#memory-model)
11. [Error Handling](#error-handling)
12. [Standard Library](#standard-library)

## Introduction

Script is a modern, expression-oriented programming language designed for simplicity, safety, and performance. It combines the familiarity of JavaScript syntax with the safety of Rust and the elegance of functional programming.

### Design Philosophy

- **Expression-oriented**: Everything returns a value
- **Gradual typing**: Optional type annotations with inference
- **Memory safe**: Automatic reference counting with cycle detection
- **Beginner friendly**: Intuitive syntax and helpful error messages
- **Game-focused**: Built-in types and utilities for game development

### Language Goals

- Simple enough for beginners to learn intuitively
- Powerful enough for production applications and games
- Compiled to native code and WebAssembly
- Memory safe without garbage collection pauses

## Lexical Structure

### Character Set

Script source code is written in UTF-8 encoded text. All Unicode code points are valid in string literals and comments.

### Comments

```script
// Single-line comment

/*
  Multi-line comment
  Can span multiple lines
*/
```

### Identifiers

Identifiers start with a letter or underscore, followed by any number of letters, digits, or underscores.

```script
valid_identifier
_private
camelCase
PascalCase
with123numbers
```

### Keywords

Reserved keywords that cannot be used as identifiers:

```
fn      let     if      else    while   for     return
true    false   match   &&      ||      !
```

Note: `print` is a built-in function, not a keyword. The logical operators `and`, `or`, and `not` are represented as `&&`, `||`, and `!` respectively.

### Literals

#### Number Literals

```script
42          // Integer (i32 by default)
3.14        // Float (f32 by default)
0xFF        // Hexadecimal
0b1010      // Binary
0o777       // Octal
1_000_000   // Underscores for readability
```

#### String Literals

```script
"Hello, World!"                // Basic string
"String with \"quotes\""       // Escaped quotes
"Unicode: 🚀 ⭐ 💫"           // Unicode support
"Multi-line
string"                        // Multi-line strings
```

#### Boolean Literals

```script
true
false
```

### Operators

#### Arithmetic Operators

```
+   -   *   /   %
```

#### Comparison Operators

```
==  !=  <   >   <=  >=
```

#### Logical Operators

```
&&  ||  !
```

#### Assignment Operators

```
=
```

### Delimiters

```
(   )   [   ]   {   }
,   .   ;   :   ->  =>  ..  _
```

## Syntax

### Grammar Overview

Script uses a context-free grammar with the following structure:

```
program     → statement*
statement   → letStmt | fnStmt | returnStmt | whileStmt | forStmt | exprStmt
expression  → assignment | logicalOr
assignment  → IDENTIFIER "=" assignment | logicalOr
logicalOr   → logicalAnd ( "||" logicalAnd )*
logicalAnd  → equality ( "&&" equality )*
equality    → comparison ( ( "!=" | "==" ) comparison )*
comparison  → term ( ( ">" | ">=" | "<" | "<=" ) term )*
term        → factor ( ( "-" | "+" ) factor )*
factor      → unary ( ( "/" | "*" | "%" ) unary )*
unary       → ( "!" | "-" ) unary | call
call        → primary ( "(" arguments? ")" | "[" expression "]" | "." IDENTIFIER )*
primary     → NUMBER | STRING | "true" | "false" | IDENTIFIER
            | "(" expression ")" | "[" arguments? "]" | ifExpr | blockExpr | matchExpr
```

### Expression-Oriented Design

In Script, most constructs are expressions that return values:

```script
// if is an expression
let result = if x > 0 { "positive" } else { "non-positive" }

// blocks are expressions
let value = {
    let temp = calculate_something()
    temp * 2  // last expression is the return value
}

// match is an expression
let type_name = match value {
    42 => "the answer",
    x if x > 100 => "big number",
    _ => "something else"
}
```

## Types

### Primitive Types

#### Numeric Types

```script
let integer: i32 = 42           // 32-bit signed integer
let float: f32 = 3.14           // 32-bit floating point
```

#### Boolean Type

```script
let flag: bool = true
let check: bool = false
```

#### String Type

```script
let message: string = "Hello, World!"
```

### Composite Types

#### Array Type

Arrays are homogeneous collections with fixed element type:

```script
let numbers: [i32] = [1, 2, 3, 4, 5]
let names: [string] = ["Alice", "Bob", "Charlie"]
let empty: [f32] = []
```

#### Function Type

Functions are first-class values with specific signatures:

```script
// Function type: (i32, i32) -> i32
fn add(a: i32, b: i32) -> i32 {
    a + b
}

// Function as variable
let operation: (i32, i32) -> i32 = add
```

### Type Annotations

Type annotations are optional in most contexts:

```script
// Explicit type annotation
let x: i32 = 42

// Type inference
let y = 42  // inferred as i32

// Partial annotation in functions
fn process(data: [i32]) {  // return type inferred
    data[0] + data[1]
}
```

### Gradual Typing

Script supports gradual typing with the `unknown` type for seamless integration between typed and untyped code:

```script
let dynamic = some_external_function()  // type: unknown
let result = dynamic + 42               // runtime type check

// Type inference with gradual typing
let mixed_data = process_unknown_data() // inferred as unknown
let safe_result: i32 = mixed_data       // explicit cast with runtime check
```

The type system allows unknown types to be compatible with any other type, with runtime checks inserted automatically when necessary.

## Expressions

### Literal Expressions

```script
42          // number literal
"hello"     // string literal
true        // boolean literal
[1, 2, 3]   // array literal
```

### Variable Expressions

```script
let x = 42
let y = x   // variable reference
```

### Binary Expressions

Arithmetic, comparison, and logical operations:

```script
let sum = a + b
let product = x * y
let is_equal = left == right
let is_valid = check1 && check2
```

#### Operator Precedence

From highest to lowest precedence:

1. Unary: `!`, `-` (unary minus)
2. Multiplicative: `*`, `/`, `%`
3. Additive: `+`, `-`
4. Comparison: `<`, `<=`, `>`, `>=`
5. Equality: `==`, `!=`
6. Logical AND: `&&`
7. Logical OR: `||`
8. Assignment: `=`

### Function Call Expressions

```script
let result = add(10, 20)
let length = get_length(array)
print("Hello, World!")
```

### Array Access and Member Access

```script
let first = numbers[0]          // array indexing
let count = array.length        // member access (future)
```

### If Expressions

```script
let result = if condition {
    "true branch"
} else {
    "false branch"
}

// Without else, returns unit type ()
if should_print {
    print("Hello")
}
```

### Block Expressions

```script
let result = {
    let temp = expensive_calculation()
    let processed = temp * 2
    processed + 1  // final expression is returned
}
```

### Match Expressions

```script
let result = match value {
    0 => "zero",
    1 => "one",
    n if n > 10 => "big",
    _ => "other"
}
```

## Statements

### Let Statements

Variable declarations with optional initialization:

```script
let x: i32              // declaration without initialization
let y = 42              // declaration with initialization
let z: f32 = 3.14       // declaration with type and initialization
```

### Expression Statements

Any expression can be used as a statement:

```script
print("Hello")          // function call statement
x + y                   // expression statement (result discarded)
```

### Assignment Statements

```script
x = 42                  // simple assignment
array[0] = new_value    // indexed assignment (future)
```

## Functions

### Function Declaration

```script
fn function_name(param1: Type1, param2: Type2) -> ReturnType {
    // function body
    return_expression
}
```

### Parameters

All parameters must have type annotations:

```script
fn greet(name: string, age: i32) -> string {
    "Hello " + name + ", you are " + age + " years old"
}
```

### Return Values

Functions can return values explicitly or implicitly:

```script
fn add(a: i32, b: i32) -> i32 {
    return a + b    // explicit return
}

fn multiply(a: i32, b: i32) -> i32 {
    a * b           // implicit return (last expression)
}
```

### Higher-Order Functions

Functions can take other functions as parameters:

```script
fn apply_twice(f: (i32) -> i32, x: i32) -> i32 {
    f(f(x))
}

fn double(x: i32) -> i32 {
    x * 2
}

let result = apply_twice(double, 5)  // result is 20
```

## Control Flow

### If Expressions

```script
if condition {
    // then branch
} else if other_condition {
    // else if branch
} else {
    // else branch
}
```

### While Loops

```script
while condition {
    // loop body
}
```

### For Loops

```script
for item in iterable {
    // loop body with item
}

// Example with array
for number in [1, 2, 3, 4, 5] {
    print(number)
}
```

### Loop Control (Future)

```script
while true {
    if should_break {
        break       // exit loop
    }
    if should_skip {
        continue    // skip to next iteration
    }
}
```

## Pattern Matching

### Match Expressions

Pattern matching provides powerful control flow:

```script
match value {
    pattern1 => expression1,
    pattern2 if guard => expression2,
    _ => default_expression
}
```

### Pattern Types

#### Literal Patterns

```script
match x {
    0 => "zero",
    1 => "one",
    42 => "the answer",
    _ => "other"
}
```

#### Variable Patterns

```script
match x {
    n => "the value is " + n
}
```

#### Array Patterns

```script
match array {
    [] => "empty",
    [x] => "single element: " + x,
    [first, second] => "two elements",
    [head, ..tail] => "head and tail",  // future
    _ => "other"
}
```

#### Guards

```script
match number {
    x if x > 0 => "positive",
    x if x < 0 => "negative",
    _ => "zero"
}
```

#### Or Patterns

```script
match character {
    'a' | 'e' | 'i' | 'o' | 'u' => "vowel",
    _ => "consonant"
}
```

## Memory Model

### Reference Counting

Script uses automatic reference counting (ARC) for memory management:

- Objects are automatically freed when their reference count reaches zero
- Cycle detection prevents memory leaks from circular references
- No garbage collection pauses

### Value vs Reference Semantics

- Primitive types (i32, f32, bool) have value semantics
- Complex types (string, arrays, functions) have reference semantics
- Assignment creates new references, not copies

```script
let x = [1, 2, 3]
let y = x           // y and x reference the same array
x[0] = 99           // both x and y see the change
```

### Memory Safety

Script prevents common memory errors:

- No null pointer dereferences
- No buffer overflows
- No use-after-free
- No double-free

## Error Handling

### Result Type (Future)

Script will use Result types for recoverable errors:

```script
fn divide(a: f32, b: f32) -> Result<f32, string> {
    if b == 0.0 {
        Err("Division by zero")
    } else {
        Ok(a / b)
    }
}

// Usage
match divide(10.0, 2.0) {
    Ok(result) => print("Result: " + result),
    Err(error) => print("Error: " + error)
}
```

### Panic

For unrecoverable errors, Script uses panic:

```script
if index >= array.length {
    panic("Index out of bounds")
}
```

## Standard Library

### I/O Functions

```script
print(value)            // Print to stdout
println(value)          // Print with newline
eprintln(value)         // Print to stderr
```

### Array Operations

```script
let length = array.length       // Get array length
array.push(value)              // Add element (future)
array.pop()                    // Remove last element (future)
```

### String Operations (Future)

```script
let length = string.length
let upper = string.to_upper()
let contains = string.contains("substring")
```

### Math Functions (Future)

```script
let abs_value = abs(-42)
let sqrt_value = sqrt(16.0)
let max_value = max(a, b)
```

## Grammar Reference

### Complete EBNF Grammar

```ebnf
program         ::= statement*

statement       ::= let_stmt | fn_stmt | return_stmt | while_stmt | for_stmt | expr_stmt

let_stmt        ::= "let" IDENTIFIER (":" type_ann)? ("=" expression)? ";"?
fn_stmt         ::= "fn" IDENTIFIER "(" parameters? ")" ("->" type_ann)? block
return_stmt     ::= "return" expression? ";"?
while_stmt      ::= "while" expression block
for_stmt        ::= "for" IDENTIFIER "in" expression block
expr_stmt       ::= expression ";"?

expression      ::= assignment
assignment      ::= IDENTIFIER "=" assignment | logical_or
logical_or      ::= logical_and ("||" logical_and)*
logical_and     ::= equality ("&&" equality)*
equality        ::= comparison (("!=" | "==") comparison)*
comparison      ::= term ((">" | ">=" | "<" | "<=") term)*
term            ::= factor (("-" | "+") factor)*
factor          ::= unary (("/" | "*" | "%") unary)*
unary           ::= ("!" | "-") unary | call
call            ::= primary (("(" arguments? ")") | ("[" expression "]") | ("." IDENTIFIER))*
primary         ::= NUMBER | STRING | "true" | "false" | IDENTIFIER
                  | "(" expression ")" | "[" arguments? "]" | if_expr | block | match_expr

if_expr         ::= "if" expression block ("else" (if_expr | block))?
block           ::= "{" statement* (expression)? "}"
match_expr      ::= "match" expression "{" match_arm* "}"
match_arm       ::= pattern ("if" expression)? "=>" expression ","?

pattern         ::= literal_pattern | identifier_pattern | array_pattern | wildcard_pattern | or_pattern
literal_pattern ::= NUMBER | STRING | "true" | "false"
identifier_pattern ::= IDENTIFIER
array_pattern   ::= "[" (pattern ("," pattern)*)? "]"
wildcard_pattern ::= "_"
or_pattern      ::= pattern ("|" pattern)+

type_ann        ::= named_type | array_type | function_type
named_type      ::= IDENTIFIER
array_type      ::= "[" type_ann "]"
function_type   ::= "(" (type_ann ("," type_ann)*)? ")" "->" type_ann

parameters      ::= parameter ("," parameter)*
parameter       ::= IDENTIFIER ":" type_ann
arguments       ::= expression ("," expression)*

IDENTIFIER      ::= [a-zA-Z_][a-zA-Z0-9_]*
NUMBER          ::= [0-9]+ ("." [0-9]+)?
STRING          ::= '"' ([^"\\] | '\\' .)* '"'
```

## Language Evolution

Script is designed to evolve carefully while maintaining backward compatibility:

### Versioning

- Semantic versioning (MAJOR.MINOR.PATCH)
- Breaking changes only in major versions
- New features in minor versions
- Bug fixes in patch versions

### Future Features

- Async/await support
- Module system
- Struct types and methods
- Traits and interfaces
- Generic types and functions
- Metaprogramming with attributes

---

*This specification defines Script Language version 0.1.0*