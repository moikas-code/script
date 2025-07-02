# User-Defined Types Specification

## Table of Contents

1. [Introduction](#introduction)
2. [Structs](#structs)
3. [Enums](#enums)
4. [Syntax Design](#syntax-design)
5. [Type System Integration](#type-system-integration)
6. [Pattern Matching Integration](#pattern-matching-integration)
7. [Memory Management](#memory-management)
8. [Semantic Analysis](#semantic-analysis)
9. [Implementation Requirements](#implementation-requirements)
10. [Examples](#examples)
11. [Best Practices](#best-practices)

## Introduction

User-defined types in Script provide the foundation for building complex data structures and domain models. The design prioritizes:

- **Beginner-friendly syntax**: Clean, readable declarations
- **Type safety**: Full integration with the inference system
- **Pattern matching**: First-class support for destructuring
- **Memory efficiency**: Optimized layout with RC memory management
- **Extensibility**: Support for methods and traits (future)

This specification builds upon Script's existing type system, integrating seamlessly with:
- The gradual typing system (allowing `unknown` fields)
- Pattern matching infrastructure
- Memory management with RC and cycle detection
- Type inference engine

## Structs

### Basic Struct Declaration

Structs define custom types with named fields:

```script
struct Point {
    x: f32,
    y: f32
}

struct Person {
    name: string,
    age: i32,
    email: string
}
```

### Field Visibility

Fields are public by default, but can be marked private with `#`:

```script
struct Account {
    name: string,           // public
    #balance: f32,          // private
    #account_id: string     // private
}
```

### Optional Fields and Defaults

Fields can be optional or have default values:

```script
struct Config {
    host: string = "localhost",    // default value
    port: i32 = 8080,             // default value
    debug: bool?,                  // optional field (Option<bool>)
    timeout: f32?                  // optional field
}
```

### Generic Structs

Structs can be parameterized with type parameters:

```script
struct Container<T> {
    value: T,
    metadata: string
}

struct Pair<T, U> {
    first: T,
    second: U
}
```

### Struct Instantiation

Creating struct instances:

```script
// All fields specified
let point = Point { x: 10.0, y: 20.0 }

// Using defaults
let config = Config { host: "example.com" }

// Field punning (when variable name matches field name)
let name = "Alice"
let age = 30
let person = Person { name, age, email: "alice@example.com" }
```

### Struct Update Syntax

Creating new instances based on existing ones:

```script
let point1 = Point { x: 10.0, y: 20.0 }
let point2 = Point { y: 30.0, ..point1 }  // x: 10.0, y: 30.0
```

### Field Access

Accessing struct fields:

```script
let person = Person { name: "Bob", age: 25, email: "bob@example.com" }
let name = person.name
let adult = person.age >= 18
```

## Enums

### Basic Enum Declaration

Enums define types with multiple variants:

```script
enum Status {
    Pending,
    InProgress,
    Completed,
    Failed
}

enum Color {
    Red,
    Green,
    Blue,
    Custom(string)
}
```

### Enums with Data

Variants can carry associated data:

```script
enum Result<T, E> {
    Ok(T),
    Err(E)
}

enum Message {
    Text(string),
    Image { url: string, alt: string },
    Video { url: string, duration: f32 }
}
```

### Discriminated Unions

Enums serve as discriminated unions:

```script
enum Shape {
    Circle { radius: f32 },
    Rectangle { width: f32, height: f32 },
    Triangle { base: f32, height: f32 }
}
```

### Generic Enums

Enums can be parameterized:

```script
enum Option<T> {
    Some(T),
    None
}

enum Either<L, R> {
    Left(L),
    Right(R)
}
```

### Enum Instantiation

Creating enum instances:

```script
let status = Status::Pending
let color = Color::Custom("crimson")
let result = Result::Ok(42)
let message = Message::Image { 
    url: "photo.jpg", 
    alt: "A beautiful sunset" 
}
```

## Syntax Design

### Keywords

New keywords needed:
- `struct` - struct declaration
- `enum` - enum declaration
- `#` - private field marker (repurpose existing `At` token)

### Grammar Extensions

#### Struct Declaration
```ebnf
struct_decl ::= 'struct' IDENTIFIER generic_params? '{' struct_fields '}'
struct_fields ::= (struct_field (',' struct_field)* ','?)?
struct_field ::= visibility? IDENTIFIER ':' type_ann default_value?
visibility ::= '#'
default_value ::= '=' expression
generic_params ::= '<' (IDENTIFIER (',' IDENTIFIER)*) '>'
```

#### Enum Declaration
```ebnf
enum_decl ::= 'enum' IDENTIFIER generic_params? '{' enum_variants '}'
enum_variants ::= (enum_variant (',' enum_variant)* ','?)?
enum_variant ::= IDENTIFIER (enum_variant_data)?
enum_variant_data ::= '(' (type_ann (',' type_ann)*)? ')'
                   | '{' struct_fields '}'
```

#### Struct Expression
```ebnf
struct_expr ::= type_name '{' struct_field_inits '}'
struct_field_inits ::= (struct_field_init (',' struct_field_init)* ','?)?
struct_field_init ::= IDENTIFIER (':' expression)?
                   | '..' expression
```

#### Enum Constructor
```ebnf
enum_constructor ::= type_name '::' IDENTIFIER ('(' expression_list ')')?
                  | type_name '::' IDENTIFIER '{' struct_field_inits '}'
```

## Type System Integration

### Type Representation

Extend the `Type` enum:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    // ... existing types ...

    /// User-defined struct type
    Struct {
        name: String,
        fields: Vec<StructField>,
        type_params: Vec<String>,
    },

    /// User-defined enum type  
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
        type_params: Vec<String>,
    },

    /// Generic type instantiation
    Generic {
        base: Box<Type>,
        args: Vec<Type>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
    pub is_private: bool,
    pub default_value: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumVariant {
    pub name: String,
    pub data: EnumVariantData,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EnumVariantData {
    Unit,                           // Status::Pending
    Tuple(Vec<Type>),              // Result::Ok(T)
    Struct(Vec<StructField>),      // Message::Image { url, alt }
}
```

### Type Checking Rules

1. **Struct field access**: Verify field exists and is accessible
2. **Struct construction**: All required fields must be provided
3. **Enum construction**: Variant must exist with correct data
4. **Generic instantiation**: Type arguments must satisfy constraints
5. **Pattern matching**: Patterns must be exhaustive for enum types

### Type Inference Integration

The inference engine must handle:

```rust
// Field access type inference
let person = get_person()  // Type: unknown
let name = person.name     // Infer person: struct with name field

// Constructor inference
let point = Point { x: 1.0, y: 2.0 }  // Infer f32 for x, y

// Generic inference
let container = Container { value: 42, metadata: "data" }
// Infer Container<i32>
```

## Pattern Matching Integration

### Struct Patterns

Destructuring structs in patterns:

```script
let person = Person { name: "Alice", age: 30, email: "alice@example.com" }

match person {
    Person { name: "Alice", age, email } => print("Found Alice, age: " + age),
    Person { name, age } if age >= 18 => print("Adult: " + name),
    Person { name, .. } => print("Person: " + name)
}
```

### Enum Patterns

Matching enum variants:

```script
let result = divide(10, 2)

match result {
    Result::Ok(value) => print("Success: " + value),
    Result::Err(error) => print("Error: " + error)
}

let message = Message::Image { url: "pic.jpg", alt: "A picture" }

match message {
    Message::Text(content) => print("Text: " + content),
    Message::Image { url, alt } => print("Image: " + url + " (" + alt + ")"),
    Message::Video { url, duration } => print("Video: " + url + " (" + duration + "s)")
}
```

### Pattern Extensions

Extend `PatternKind`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum PatternKind {
    // ... existing patterns ...

    /// Struct pattern: Point { x, y }
    Struct {
        name: String,
        fields: Vec<(String, Option<Pattern>)>,
        rest: bool,  // for .. patterns
    },

    /// Enum pattern: Result::Ok(value)
    Enum {
        name: String,
        variant: String,
        data: Option<Box<Pattern>>,
    },
}
```

## Memory Management

### RC Integration

User-defined types integrate with Script's RC memory management:

```script
struct Node {
    value: i32,
    children: [Node]  // RC<[RC<Node>]>
}

// Cycle detection handles recursive structures
let node1 = Node { value: 1, children: [] }
let node2 = Node { value: 2, children: [node1] }
node1.children = [node2]  // Cycle detected and handled
```

### Memory Layout

Structs have predictable memory layout:
- Fields stored in declaration order
- Optional fields use `Option<T>` representation
- Private fields have same layout as public fields
- Generic structs monomorphized at compile time

Enums use discriminated union layout:
- Tag byte indicates active variant
- Union of all variant data
- Optimized representation for common cases (Option<T>, etc.)

## Semantic Analysis

### Symbol Table Extensions

The symbol table must track:

```rust
#[derive(Debug, Clone)]
pub enum Symbol {
    // ... existing symbols ...
    
    Struct {
        name: String,
        fields: Vec<StructField>,
        type_params: Vec<String>,
        methods: Vec<FunctionSignature>,  // Future: methods
    },
    
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
        type_params: Vec<String>,
        methods: Vec<FunctionSignature>,  // Future: methods
    },
}
```

### Semantic Checks

1. **Name uniqueness**: Struct/enum names must be unique in their scope
2. **Field uniqueness**: Field names must be unique within a struct
3. **Variant uniqueness**: Variant names must be unique within an enum
4. **Recursive type checking**: Detect infinite recursion
5. **Visibility checking**: Private fields only accessible within module
6. **Exhaustiveness**: Enum pattern matches must be exhaustive

### Error Reporting

Enhanced error messages for user-defined types:

```script
struct Point { x: f32, y: f32 }
let p = Point { x: 1.0 }  // Error: missing field `y`

enum Color { Red, Green, Blue }
let c = Color::Yellow  // Error: variant `Yellow` not found in enum `Color`

match Color::Red {
    Color::Red => "red",
    // Error: non-exhaustive pattern matching, missing `Green`, `Blue`
}
```

## Implementation Requirements

### Lexer Changes

Add new tokens:
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // ... existing tokens ...
    
    // New keywords
    Struct,
    Enum,
    
    // Reuse existing tokens
    // At,           // # for private fields (repurpose)
    // ColonColon,   // :: for enum constructors (new)
}
```

### Parser Extensions

New AST nodes:
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    // ... existing statements ...
    
    StructDecl {
        name: String,
        type_params: Vec<String>,
        fields: Vec<StructFieldDecl>,
    },
    
    EnumDecl {
        name: String,
        type_params: Vec<String>,
        variants: Vec<EnumVariantDecl>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    // ... existing expressions ...
    
    StructExpr {
        name: String,
        fields: Vec<(String, Option<Expr>)>,
        base: Option<Box<Expr>>,  // for update syntax
    },
    
    EnumConstructor {
        name: String,
        variant: String,
        data: Option<EnumConstructorData>,
    },
}

#[derive(Debug, Clone, PartialEq)]  
pub enum EnumConstructorData {
    Tuple(Vec<Expr>),
    Struct(Vec<(String, Expr)>),
}
```

### Type System Changes

1. Extend `Type` enum with struct and enum variants
2. Update type equality and assignability checking
3. Add generic type instantiation logic
4. Implement field access type checking

### Semantic Analysis Changes

1. Add struct and enum declarations to symbol table
2. Implement type checking for struct construction
3. Add enum variant checking
4. Implement pattern exhaustiveness checking
5. Add visibility checking for private fields

### Lowering Changes

1. Generate IR for struct construction and field access
2. Generate IR for enum construction and pattern matching
3. Handle generic type monomorphization
4. Optimize memory layout

## Examples

### Complete Example: Binary Tree

```script
enum Tree<T> {
    Empty,
    Node {
        value: T,
        left: Tree<T>,
        right: Tree<T>
    }
}

struct TreeStats {
    depth: i32,
    node_count: i32
}

fn insert<T>(tree: Tree<T>, value: T) -> Tree<T> {
    match tree {
        Tree::Empty => Tree::Node {
            value,
            left: Tree::Empty,
            right: Tree::Empty
        },
        Tree::Node { value: v, left, right } => {
            if value < v {
                Tree::Node { value: v, left: insert(left, value), right }
            } else {
                Tree::Node { value: v, left, right: insert(right, value) }
            }
        }
    }
}

fn analyze<T>(tree: Tree<T>) -> TreeStats {
    match tree {
        Tree::Empty => TreeStats { depth: 0, node_count: 0 },
        Tree::Node { left, right, .. } => {
            let left_stats = analyze(left)
            let right_stats = analyze(right)
            TreeStats {
                depth: 1 + max(left_stats.depth, right_stats.depth),
                node_count: 1 + left_stats.node_count + right_stats.node_count
            }
        }
    }
}
```

### Complete Example: HTTP Request/Response

```script
enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch
}

struct HttpRequest {
    method: HttpMethod,
    url: string,
    headers: [string],
    body: string?
}

struct HttpResponse {
    status: i32,
    headers: [string],
    body: string
}

enum HttpResult {
    Success(HttpResponse),
    NetworkError(string),
    TimeoutError,
    ParseError(string)
}

fn make_request(request: HttpRequest) -> HttpResult {
    // Implementation would use FFI or built-in HTTP client
    match request.method {
        HttpMethod::Get => {
            // Handle GET request
            HttpResult::Success(HttpResponse {
                status: 200,
                headers: ["Content-Type: application/json"],
                body: "{\"message\": \"Hello, World!\"}"
            })
        },
        HttpMethod::Post => {
            // Handle POST request
            if request.body? {
                HttpResult::Success(HttpResponse {
                    status: 201,
                    headers: ["Content-Type: application/json"],
                    body: "{\"status\": \"created\"}"
                })
            } else {
                HttpResult::ParseError("POST request missing body")
            }
        },
        _ => HttpResult::NetworkError("Method not implemented")
    }
}
```

## Best Practices

### Struct Design

1. **Use descriptive field names**: `user_id` instead of `id`
2. **Group related fields**: Consider multiple structs for complex data
3. **Prefer composition over inheritance**: Use struct fields instead of complex hierarchies
4. **Use private fields for invariants**: Mark fields private when they need validation

### Enum Design

1. **Prefer explicit variants**: `Status::InProgress` over magic numbers
2. **Use data-carrying variants**: Store relevant information with each variant
3. **Consider Result<T, E>**: For operations that can fail
4. **Group related states**: Use enums to model state machines

### Pattern Matching

1. **Handle all cases**: Ensure exhaustive matching
2. **Use guards for complex conditions**: `if age >= 18` in patterns
3. **Destructure only needed fields**: Use `..` for unused fields
4. **Order patterns by specificity**: Most specific patterns first

### Performance Considerations

1. **Small enums**: Keep enum variants small for efficient discriminated unions
2. **Field ordering**: Order struct fields by size for optimal packing
3. **Generic monomorphization**: Be aware of code bloat with many generic instantiations
4. **RC cycles**: Use weak references to break cycles in recursive structures

This specification provides a comprehensive foundation for implementing user-defined types in the Script language, maintaining consistency with existing language design while adding powerful new capabilities for data modeling and type safety.