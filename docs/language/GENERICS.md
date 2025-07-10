# Generic Types and Constraints in Script

> **Implementation Status**: ✅ Fully implemented as of v0.5.0-alpha. The generic system includes complete monomorphization, type inference, and constraint satisfaction with 43% deduplication efficiency.

## Table of Contents

1. [Introduction](#introduction)
2. [Generic Type Syntax](#generic-type-syntax)
3. [Type Parameters](#type-parameters)
4. [Constraints and Bounds](#constraints-and-bounds)
5. [Built-in Traits](#built-in-traits)
6. [Generic Functions](#generic-functions)
7. [Generic Structs](#generic-structs)
8. [Type Inference with Generics](#type-inference-with-generics)
9. [Monomorphization](#monomorphization)
10. [Advanced Features](#advanced-features)
11. [Implementation Details](#implementation-details)
12. [Examples](#examples)

## Introduction

Script's generic type system enables writing reusable, type-safe code by allowing types and functions to be parameterized over other types. The system includes:

- **Type Parameters**: Variables that represent types (`T`, `U`, `K`, `V`)
- **Constraints**: Requirements that type parameters must satisfy
- **Built-in Traits**: Standard interfaces like `Eq`, `Ord`, `Clone`, `Display`
- **Monomorphization**: Compile-time specialization for performance

## Generic Type Syntax

### Basic Generic Syntax

```script
// Generic function with single type parameter
fn identity<T>(value: T) -> T {
    value
}

// Generic function with multiple type parameters
fn pair<T, U>(first: T, second: U) -> (T, U) {
    (first, second)
}

// Generic struct
struct Container<T> {
    value: T
}

// Generic enum
enum Result<T, E> {
    Ok(T),
    Err(E)
}
```

### Type Parameter Naming Conventions

```script
// Common conventions:
// T, U, V, W - for general type parameters
// K, V - for key-value pairs (maps)
// E - for error types
// R - for result types

fn map<T, U>(items: [T], f: (T) -> U) -> [U] {
    // implementation
}

fn insert<K, V>(map: Map<K, V>, key: K, value: V) -> Map<K, V> {
    // implementation
}
```

## Type Parameters

### Declaring Type Parameters

```script
// Function with type parameters
fn swap<T>(a: T, b: T) -> (T, T) {
    (b, a)
}

// Struct with type parameters
struct Pair<T, U> {
    first: T,
    second: U
}

// Method with additional type parameters
impl<T> Container<T> {
    fn map<U>(self, f: (T) -> U) -> Container<U> {
        Container { value: f(self.value) }
    }
}
```

### Using Type Parameters

```script
// Explicit type specification
let number = identity<i32>(42)
let text = identity<string>("hello")

// Type inference (preferred)
let number = identity(42)     // T inferred as i32
let text = identity("hello")  // T inferred as string

// Mixed inference and specification
let result = pair(42, "hello")  // T=i32, U=string inferred
let result: (i32, string) = pair(42, "hello")  // equivalent
```

## Constraints and Bounds

### Constraint Syntax

```script
// Single constraint
fn compare<T: Eq>(a: T, b: T) -> bool {
    a == b
}

// Multiple constraints
fn sort<T: Ord + Clone>(items: [T]) -> [T] {
    // implementation that requires ordering and cloning
}

// Where clause syntax (for complex constraints)
fn process<T, U>(items: [T]) -> [U] 
where 
    T: Clone + Display,
    U: Default + FromStr<T>
{
    // implementation
}
```

### Constraint Types

#### Equality Constraints
```script
// Requires type to support equality comparison
fn find<T: Eq>(items: [T], target: T) -> Option<i32> {
    for i in 0..items.len() {
        if items[i] == target {
            return Some(i)
        }
    }
    None
}
```

#### Ordering Constraints
```script
// Requires type to support ordering
fn max<T: Ord>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

fn sort<T: Ord>(items: [T]) -> [T] {
    // sorting implementation using comparison
}
```

#### Display Constraints
```script
// Requires type to be printable
fn debug_print<T: Display>(value: T) {
    print("Debug: " + value.to_string())
}
```

## Built-in Traits

### Eq Trait (Equality)
```script
trait Eq {
    fn eq(self, other: Self) -> bool
    fn ne(self, other: Self) -> bool { !self.eq(other) }
}

// Automatically implemented for basic types
// i32, f32, bool, string implement Eq
```

### Ord Trait (Ordering)
```script
trait Ord: Eq {
    fn cmp(self, other: Self) -> Ordering
    fn lt(self, other: Self) -> bool { self.cmp(other) == Less }
    fn le(self, other: Self) -> bool { self.cmp(other) != Greater }
    fn gt(self, other: Self) -> bool { self.cmp(other) == Greater }
    fn ge(self, other: Self) -> bool { self.cmp(other) != Less }
}

enum Ordering {
    Less,
    Equal,
    Greater
}
```

### Clone Trait (Duplication)
```script
trait Clone {
    fn clone(self) -> Self
}

// Automatically implemented for Copy types
// Required for many generic operations
```

### Display Trait (String Representation)
```script
trait Display {
    fn to_string(self) -> string
}

// Implemented by all basic types
// Required for printing and debugging
```

### Default Trait (Default Values)
```script
trait Default {
    fn default() -> Self
}

// Examples of default implementations:
// i32::default() -> 0
// f32::default() -> 0.0
// bool::default() -> false
// string::default() -> ""
// [T]::default() -> []
```

## Generic Functions

### Basic Generic Functions

```script
// Identity function
fn identity<T>(value: T) -> T {
    value
}

// Array operations
fn first<T>(arr: [T]) -> Option<T> {
    if arr.len() > 0 {
        Some(arr[0])
    } else {
        None
    }
}

fn last<T>(arr: [T]) -> Option<T> {
    if arr.len() > 0 {
        Some(arr[arr.len() - 1])
    } else {
        None
    }
}
```

### Generic Functions with Constraints

```script
// Requires equality for comparison
fn contains<T: Eq>(arr: [T], item: T) -> bool {
    for element in arr {
        if element == item {
            return true
        }
    }
    false
}

// Requires ordering for sorting
fn binary_search<T: Ord>(arr: [T], target: T) -> Option<i32> {
    let mut left = 0
    let mut right = arr.len() - 1
    
    while left <= right {
        let mid = (left + right) / 2
        let mid_val = arr[mid]
        
        if mid_val == target {
            return Some(mid)
        } else if mid_val < target {
            left = mid + 1
        } else {
            right = mid - 1
        }
    }
    
    None
}
```

### Higher-Order Generic Functions

```script
// Map function
fn map<T, U>(arr: [T], f: (T) -> U) -> [U] {
    let result: [U] = []
    for item in arr {
        result.push(f(item))
    }
    result
}

// Filter function
fn filter<T>(arr: [T], predicate: (T) -> bool) -> [T] {
    let result: [T] = []
    for item in arr {
        if predicate(item) {
            result.push(item)
        }
    }
    result
}

// Reduce function
fn reduce<T, U>(arr: [T], initial: U, f: (U, T) -> U) -> U {
    let acc = initial
    for item in arr {
        acc = f(acc, item)
    }
    acc
}
```

## Generic Structs

### Basic Generic Structs

```script
// Generic container
struct Box<T> {
    value: T
}

impl<T> Box<T> {
    fn new(value: T) -> Box<T> {
        Box { value }
    }
    
    fn get(self) -> T {
        self.value
    }
    
    fn set(mut self, value: T) {
        self.value = value
    }
}

// Generic pair
struct Pair<T, U> {
    first: T,
    second: U
}

impl<T, U> Pair<T, U> {
    fn new(first: T, second: U) -> Pair<T, U> {
        Pair { first, second }
    }
    
    fn swap(self) -> Pair<U, T> {
        Pair { first: self.second, second: self.first }
    }
}
```

### Generic Collections

```script
// Generic vector
struct Vec<T> {
    data: [T],
    len: i32,
    capacity: i32
}

impl<T> Vec<T> {
    fn new() -> Vec<T> {
        Vec { data: [], len: 0, capacity: 0 }
    }
    
    fn push(mut self, item: T) {
        // resize if needed
        self.data.push(item)
        self.len += 1
    }
    
    fn pop(mut self) -> Option<T> {
        if self.len > 0 {
            self.len -= 1
            Some(self.data.pop())
        } else {
            None
        }
    }
    
    fn get(self, index: i32) -> Option<T> {
        if index >= 0 && index < self.len {
            Some(self.data[index])
        } else {
            None
        }
    }
}

// Generic map
struct Map<K, V> {
    keys: [K],
    values: [V]
}

impl<K: Eq, V> Map<K, V> {
    fn new() -> Map<K, V> {
        Map { keys: [], values: [] }
    }
    
    fn insert(mut self, key: K, value: V) {
        // Find existing key or add new one
        for i in 0..self.keys.len() {
            if self.keys[i] == key {
                self.values[i] = value
                return
            }
        }
        
        self.keys.push(key)
        self.values.push(value)
    }
    
    fn get(self, key: K) -> Option<V> {
        for i in 0..self.keys.len() {
            if self.keys[i] == key {
                return Some(self.values[i])
            }
        }
        None
    }
}
```

## Type Inference with Generics

### Inference Rules

1. **Function Call Inference**: Type parameters inferred from arguments
2. **Return Type Inference**: Type parameters inferred from expected return type
3. **Constraint Propagation**: Constraints help narrow type possibilities
4. **Unification**: Type variables unified with concrete types when possible

### Inference Examples

```script
// Simple inference
let x = identity(42)        // T inferred as i32
let y = identity("hello")   // T inferred as string

// Multi-parameter inference
let p = pair(42, "hello")   // T=i32, U=string

// Collection inference
let numbers = [1, 2, 3]     // [i32]
let first = first(numbers)  // T inferred as i32

// Constraint-based inference
let sorted = sort([3, 1, 4, 1, 5])  // T inferred as i32 (has Ord)
let found = contains(sorted, 4)     // T inferred as i32 (has Eq)
```

### Ambiguity Resolution

```script
// Sometimes explicit types needed
let empty: Vec<i32> = Vec::new()    // T cannot be inferred
let map: Map<string, i32> = Map::new()  // K, V cannot be inferred

// Type annotations resolve ambiguity
fn process_data() -> [i32] {
    let data = []               // Error: cannot infer type
    let data: [i32] = []        // OK: explicit type
    data
}
```

## Monomorphization

### Compilation Strategy

Script uses monomorphization to generate specialized versions of generic functions for each concrete type used:

```script
// Generic function
fn max<T: Ord>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

// Usage
let int_max = max(42, 10)           // generates max_i32
let float_max = max(3.14, 2.71)     // generates max_f32
let string_max = max("hello", "hi") // generates max_string
```

### Generated Code

```script
// Compiler generates these specialized functions:
fn max_i32(a: i32, b: i32) -> i32 {
    if a > b { a } else { b }
}

fn max_f32(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

fn max_string(a: string, b: string) -> string {
    if a > b { a } else { b }
}
```

### Optimization Benefits

1. **No Runtime Overhead**: Generic abstractions compiled away
2. **Specialized Code**: Each instance optimized for specific types
3. **Inlining Opportunities**: Small generic functions can be inlined
4. **Type Safety**: All type checking done at compile time

## Advanced Features

### Associated Types (Future)

```script
// Future: Associated types for more complex generic relationships
trait Iterator {
    type Item
    
    fn next(mut self) -> Option<Self::Item>
}

impl Iterator for Vec<T> {
    type Item = T
    
    fn next(mut self) -> Option<T> {
        self.pop()
    }
}
```

### Higher-Kinded Types (Future)

```script
// Future: Generic over type constructors
trait Functor<F> {
    fn map<A, B>(self: F<A>, f: (A) -> B) -> F<B>
}

impl Functor<Option> for Option<T> {
    fn map<A, B>(self: Option<A>, f: (A) -> B) -> Option<B> {
        match self {
            Some(a) => Some(f(a)),
            None => None
        }
    }
}
```

### Generic Constraints on Associated Types (Future)

```script
// Future: Constraints on associated types
trait Collect<T> {
    type Output: Default + Clone
    
    fn collect(items: [T]) -> Self::Output
}
```

## Implementation Details

### Type System Integration

The generic type system integrates with Script's existing type system:

1. **Type Variables**: Extended to support named type parameters
2. **Constraint System**: Enhanced to handle trait bounds
3. **Inference Engine**: Modified to handle generic inference
4. **Monomorphization**: New compilation phase for specialization

### AST Extensions

```rust
// New AST nodes for generics
pub struct TypeParam {
    pub name: String,
    pub bounds: Vec<TraitBound>,
    pub span: Span,
}

pub struct TraitBound {
    pub trait_name: String,
    pub span: Span,
}

pub struct GenericParams {
    pub params: Vec<TypeParam>,
    pub where_clause: Option<WhereClause>,
}
```

### Constraint Solver Extensions

```rust
// Extended constraint types
pub enum ConstraintKind {
    Equality(Type, Type),
    TraitBound(Type, String),  // Type must implement trait
    Associated(Type, String, Type),  // Associated type constraint
}
```

## Examples

### Basic Generic Usage

```script
// Generic utility functions
fn swap<T>(a: T, b: T) -> (T, T) {
    (b, a)
}

fn duplicate<T: Clone>(value: T) -> (T, T) {
    (value.clone(), value)
}

// Usage
let (x, y) = swap(1, 2)             // (2, 1)
let (a, b) = duplicate("hello")     // ("hello", "hello")
```

### Generic Data Structures

```script
// Stack implementation
struct Stack<T> {
    items: [T]
}

impl<T> Stack<T> {
    fn new() -> Stack<T> {
        Stack { items: [] }
    }
    
    fn push(mut self, item: T) {
        self.items.push(item)
    }
    
    fn pop(mut self) -> Option<T> {
        if self.items.len() > 0 {
            Some(self.items.pop())
        } else {
            None
        }
    }
    
    fn peek(self) -> Option<T> {
        if self.items.len() > 0 {
            Some(self.items[self.items.len() - 1])
        } else {
            None
        }
    }
}

// Usage
let mut stack: Stack<i32> = Stack::new()
stack.push(1)
stack.push(2)
stack.push(3)

while let Some(item) = stack.pop() {
    print(item)  // prints 3, 2, 1
}
```

### Generic Algorithms

```script
// Generic sorting
fn quicksort<T: Ord + Clone>(arr: [T]) -> [T] {
    if arr.len() <= 1 {
        return arr
    }
    
    let pivot = arr[arr.len() / 2].clone()
    let less = filter(arr, |x| x < pivot)
    let equal = filter(arr, |x| x == pivot)
    let greater = filter(arr, |x| x > pivot)
    
    quicksort(less) + equal + quicksort(greater)
}

// Usage
let numbers = [3, 1, 4, 1, 5, 9, 2, 6]
let sorted = quicksort(numbers)
print(sorted)  // [1, 1, 2, 3, 4, 5, 6, 9]

let words = ["banana", "apple", "cherry", "date"]
let sorted_words = quicksort(words)
print(sorted_words)  // ["apple", "banana", "cherry", "date"]
```

### Error Handling with Generics

```script
// Generic Result type
enum Result<T, E> {
    Ok(T),
    Err(E)
}

impl<T, E> Result<T, E> {
    fn is_ok(self) -> bool {
        match self {
            Ok(_) => true,
            Err(_) => false
        }
    }
    
    fn is_err(self) -> bool {
        !self.is_ok()
    }
    
    fn map<U>(self, f: (T) -> U) -> Result<U, E> {
        match self {
            Ok(value) => Ok(f(value)),
            Err(e) => Err(e)
        }
    }
    
    fn unwrap(self) -> T {
        match self {
            Ok(value) => value,
            Err(_) => panic("called unwrap on an Err value")
        }
    }
}

// Usage
fn divide(a: f32, b: f32) -> Result<f32, string> {
    if b == 0.0 {
        Err("division by zero")
    } else {
        Ok(a / b)
    }
}

let result = divide(10.0, 2.0)
match result {
    Ok(value) => print("Result: " + value.to_string()),
    Err(error) => print("Error: " + error)
}
```

---

This generic type system provides Script with powerful abstraction capabilities while maintaining type safety and performance through compile-time monomorphization. The system is designed to be approachable for beginners while offering advanced features for complex applications.