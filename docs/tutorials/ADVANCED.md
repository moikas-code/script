# Advanced Features in Script

Welcome to the advanced Script tutorial! This guide covers sophisticated features that make Script a powerful language for complex applications. You'll learn about pattern matching, advanced type system features, metaprogramming, and performance optimization techniques.

## Table of Contents

1. [Pattern Matching](#pattern-matching)
2. [Advanced Type System](#advanced-type-system)
3. [Metaprogramming](#metaprogramming)
4. [Memory Management](#memory-management)
5. [Performance Optimization](#performance-optimization)
6. [Concurrency and Async](#concurrency-and-async)
7. [Foreign Function Interface (FFI)](#foreign-function-interface-ffi)
8. [Advanced Collections](#advanced-collections)
9. [Error Handling Patterns](#error-handling-patterns)
10. [Building Libraries](#building-libraries)

## Pattern Matching

Pattern matching is one of Script's most powerful features, allowing you to destructure data and handle different cases elegantly.

### Basic Pattern Matching

```script
// Match on simple values
fn describe_number(n: i32) -> string {
    match n {
        0 => "zero",
        1 => "one",
        2 => "two",
        n if n > 10 => "big number",
        n if n < 0 => "negative",
        _ => "small positive number"
    }
}

// Match on Option types
fn handle_optional(opt: Option<i32>) -> string {
    match opt {
        Some(value) => "Got value: " + value,
        None => "No value"
    }
}

// Match on Result types
fn handle_result(result: Result<string, string>) -> string {
    match result {
        Ok(success) => "Success: " + success,
        Err(error) => "Error: " + error
    }
}
```

### Array Pattern Matching

```script
// Match on array structure
fn process_coordinates(coords: [f32]) -> string {
    match coords {
        [] => "No coordinates",
        [x] => "1D point at " + x,
        [x, y] => "2D point at (" + x + ", " + y + ")",
        [x, y, z] => "3D point at (" + x + ", " + y + ", " + z + ")",
        [x, y, z, w] => "4D point at (" + x + ", " + y + ", " + z + ", " + w + ")",
        _ => "High-dimensional point"
    }
}

// Match with head/tail patterns
fn analyze_list(items: [i32]) -> string {
    match items {
        [] => "Empty list",
        [first] => "Single item: " + first,
        [first, second] => "Two items: " + first + " and " + second,
        [first, rest...] => "First item: " + first + ", plus " + vec_len(rest) + " more"
    }
}
```

### Object Pattern Matching

```script
// Define a struct-like object
struct Person {
    name: string,
    age: i32,
    email: Option<string>
}

// Match on object structure
fn greet_person(person: Person) -> string {
    match person {
        Person { name, age: 0..=17, email: None } => 
            "Hello young " + name + "! Please add an email.",
        Person { name, age: 18..=64, email: Some(email) } => 
            "Hello " + name + "! Your email is " + email,
        Person { name, age: 65.., email: _ } => 
            "Hello senior " + name + "!",
        Person { name, age, email: _ } => 
            "Hello " + name + ", age " + age
    }
}

// Nested pattern matching
fn analyze_contact(contact: { name: string, info: { phone: Option<string>, email: Option<string> }}) -> string {
    match contact {
        { name, info: { phone: Some(phone), email: Some(email) }} => 
            name + " has both phone (" + phone + ") and email (" + email + ")",
        { name, info: { phone: Some(phone), email: None }} => 
            name + " has phone only: " + phone,
        { name, info: { phone: None, email: Some(email) }} => 
            name + " has email only: " + email,
        { name, info: { phone: None, email: None }} => 
            name + " has no contact info"
    }
}
```

### Guard Expressions

```script
// Use guards for complex conditions
fn categorize_score(score: i32, subject: string) -> string {
    match score {
        s if s >= 90 && subject == "math" => "Math genius!",
        s if s >= 90 => "Excellent work!",
        s if s >= 80 => "Good job!",
        s if s >= 70 => "Not bad",
        s if s >= 60 => "Needs improvement",
        _ => "Must retake"
    }
}

// Multiple guards
fn weather_advice(temp: f32, humidity: f32, wind: f32) -> string {
    match (temp, humidity, wind) {
        (t, h, w) if t > 80.0 && h > 0.7 => "Hot and humid - stay indoors",
        (t, h, w) if t > 80.0 && w > 20.0 => "Hot and windy - find shade",
        (t, h, w) if t < 32.0 && w > 15.0 => "Cold and windy - bundle up",
        (t, h, w) if t > 70.0 && h < 0.3 => "Perfect weather!",
        _ => "Weather is okay"
    }
}
```

### Pattern Matching in Functions

```script
// Pattern match function parameters
fn distance(point1: [f32], point2: [f32]) -> f32 {
    match (point1, point2) {
        ([x1, y1], [x2, y2]) => {
            let dx = x2 - x1
            let dy = y2 - y1
            sqrt(dx * dx + dy * dy)
        },
        ([x1, y1, z1], [x2, y2, z2]) => {
            let dx = x2 - x1
            let dy = y2 - y1
            let dz = z2 - z1
            sqrt(dx * dx + dy * dy + dz * dz)
        },
        _ => 0.0  // Invalid dimensions
    }
}

// Complex pattern matching for state machines
enum GameState {
    Menu { selected: i32 },
    Playing { level: i32, score: i32, lives: i32 },
    Paused { level: i32, score: i32, lives: i32 },
    GameOver { final_score: i32 }
}

fn handle_input(state: GameState, input: string) -> GameState {
    match (state, input) {
        (GameState::Menu { selected }, "enter") => 
            GameState::Playing { level: 1, score: 0, lives: 3 },
        (GameState::Menu { selected }, "up") => 
            GameState::Menu { selected: max(0, selected - 1) },
        (GameState::Menu { selected }, "down") => 
            GameState::Menu { selected: min(2, selected + 1) },
        
        (GameState::Playing { level, score, lives }, "pause") => 
            GameState::Paused { level, score, lives },
        (GameState::Playing { level, score, lives: 0 }, _) => 
            GameState::GameOver { final_score: score },
        
        (GameState::Paused { level, score, lives }, "resume") => 
            GameState::Playing { level, score, lives },
        (GameState::Paused { .. }, "menu") => 
            GameState::Menu { selected: 0 },
        
        (GameState::GameOver { .. }, "restart") => 
            GameState::Menu { selected: 0 },
        
        (state, _) => state  // No change for unhandled input
    }
}
```

## Advanced Type System

Script's type system combines the flexibility of dynamic typing with the safety of static typing through gradual typing and advanced features.

### Generic Types

```script
// Generic functions
fn identity<T>(value: T) -> T {
    value
}

fn map_option<T, U>(opt: Option<T>, f: fn(T) -> U) -> Option<U> {
    match opt {
        Some(value) => Some(f(value)),
        None => None
    }
}

// Generic structs
struct Container<T> {
    value: T,
    metadata: string
}

impl<T> Container<T> {
    fn new(value: T, metadata: string) -> Container<T> {
        Container { value, metadata }
    }
    
    fn get_value(self) -> T {
        self.value
    }
    
    fn map<U>(self, f: fn(T) -> U) -> Container<U> {
        Container {
            value: f(self.value),
            metadata: self.metadata
        }
    }
}

// Usage
let int_container = Container::new(42, "integer")
let string_container = int_container.map(|x| "Value: " + x)
```

### Traits and Implementations

```script
// Define traits (interfaces)
trait Drawable {
    fn draw(self) -> string
    fn area(self) -> f32
}

trait Comparable<T> {
    fn compare(self, other: T) -> i32  // -1, 0, 1
}

// Implement traits for types
struct Circle {
    radius: f32,
    center: [f32; 2]
}

impl Drawable for Circle {
    fn draw(self) -> string {
        "Circle at (" + self.center[0] + ", " + self.center[1] + ") with radius " + self.radius
    }
    
    fn area(self) -> f32 {
        3.14159 * self.radius * self.radius
    }
}

impl Comparable<Circle> for Circle {
    fn compare(self, other: Circle) -> i32 {
        let self_area = self.area()
        let other_area = other.area()
        if self_area < other_area { -1 }
        else if self_area > other_area { 1 }
        else { 0 }
    }
}

struct Rectangle {
    width: f32,
    height: f32,
    position: [f32; 2]
}

impl Drawable for Rectangle {
    fn draw(self) -> string {
        "Rectangle at (" + self.position[0] + ", " + self.position[1] + 
        ") with size " + self.width + "x" + self.height
    }
    
    fn area(self) -> f32 {
        self.width * self.height
    }
}

// Generic functions with trait bounds
fn display_drawable<T: Drawable>(item: T) {
    print(item.draw())
    print("Area: " + item.area())
}

fn compare_and_display<T: Drawable + Comparable<T>>(a: T, b: T) {
    display_drawable(a)
    display_drawable(b)
    
    match a.compare(b) {
        -1 => print("First item is smaller"),
        0 => print("Items are equal"),
        1 => print("First item is larger"),
        _ => print("Comparison error")
    }
}
```

### Type Aliases and Newtype Pattern

```script
// Type aliases for clarity
type UserId = i32
type Score = i32
type Timestamp = f64
type Position = [f32; 2]
type Color = [f32; 4]  // RGBA

// Newtype pattern for type safety
struct Password(string)
struct Email(string)
struct Username(string)

impl Password {
    fn new(raw: string) -> Result<Password, string> {
        if string_len(raw) < 8 {
            Result::err("Password must be at least 8 characters")
        } else if !contains(raw, "0123456789") {
            Result::err("Password must contain at least one digit")
        } else {
            Result::ok(Password(raw))
        }
    }
    
    fn strength(self) -> i32 {
        let mut score = 0
        if string_len(self.0) >= 12 { score += 2 }
        if contains(self.0, "!@#$%^&*") { score += 2 }
        if contains(self.0, "ABCDEFGHIJKLMNOPQRSTUVWXYZ") { score += 1 }
        score
    }
}

impl Email {
    fn new(raw: string) -> Result<Email, string> {
        if contains(raw, "@") && contains(raw, ".") {
            Result::ok(Email(raw))
        } else {
            Result::err("Invalid email format")
        }
    }
    
    fn domain(self) -> string {
        let parts = split(self.0, "@")
        match vec_get(parts, 1) {
            Some(domain) => domain,
            None => ""
        }
    }
}

// Using newtypes
fn create_user(username: Username, email: Email, password: Password) -> Result<User, string> {
    let strength = password.strength()
    if strength < 3 {
        Result::err("Password too weak")
    } else {
        Result::ok(User {
            id: generate_user_id(),
            username: username.0,
            email: email.0,
            password_hash: hash_password(password.0)
        })
    }
}
```

### Advanced Pattern Types

```script
// Union types (sum types)
enum Value {
    Number(f64),
    Text(string),
    Boolean(bool),
    List(Vec<Value>),
    Object(HashMap<string, Value>)
}

fn value_to_string(value: Value) -> string {
    match value {
        Value::Number(n) => n.to_string(),
        Value::Text(s) => s,
        Value::Boolean(b) => if b { "true" } else { "false" },
        Value::List(items) => {
            let mut result = "["
            let count = vec_len(items)
            for i in 0..count {
                if i > 0 { result += ", " }
                match vec_get(items, i) {
                    Some(item) => result += value_to_string(item),
                    None => continue
                }
            }
            result + "]"
        },
        Value::Object(map) => {
            // Simplified object representation
            "{object}"
        }
    }
}

// Result chaining and error accumulation
enum ValidationError {
    TooShort(string),
    TooLong(string),
    InvalidFormat(string),
    Missing(string)
}

fn validate_user_input(input: HashMap<string, string>) -> Result<ValidatedUser, Vec<ValidationError>> {
    let mut errors = Vec::new()
    
    // Validate username
    let username_result = match hashmap_get(input, "username") {
        Some(name) => validate_username(name),
        None => {
            vec_push(errors, ValidationError::Missing("username"))
            Result::err("missing")
        }
    }
    
    // Validate email
    let email_result = match hashmap_get(input, "email") {
        Some(email) => validate_email(email),
        None => {
            vec_push(errors, ValidationError::Missing("email"))
            Result::err("missing")
        }
    }
    
    // Validate password
    let password_result = match hashmap_get(input, "password") {
        Some(pass) => validate_password(pass),
        None => {
            vec_push(errors, ValidationError::Missing("password"))
            Result::err("missing")
        }
    }
    
    // Combine results
    if vec_len(errors) > 0 {
        Result::err(errors)
    } else {
        match (username_result, email_result, password_result) {
            (Ok(user), Ok(email), Ok(pass)) => {
                Result::ok(ValidatedUser { username: user, email, password: pass })
            },
            _ => Result::err(errors)  // Should not happen if individual validations passed
        }
    }
}
```

## Metaprogramming

Script provides several metaprogramming features for code generation and compile-time computation.

### Attributes and Decorators

```script
// Derive common traits automatically
@derive(Debug, Clone, PartialEq)
struct Point {
    x: f32,
    y: f32
}

@derive(Serialize, Deserialize)
struct GameState {
    level: i32,
    score: i32,
    player_position: Point
}

// Custom attributes for code generation
@generate_builder
struct Configuration {
    database_url: string,
    api_key: string,
    debug_mode: bool,
    max_connections: i32
}

// Generated builder pattern:
// let config = Configuration::builder()
//     .database_url("localhost:5432")
//     .api_key("secret")
//     .debug_mode(true)
//     .max_connections(10)
//     .build()

// Compile-time assertions
@static_assert(size_of(Point) == 8)
@static_assert(align_of(Point) == 4)
struct Point {
    x: f32,
    y: f32
}
```

### Compile-Time Functions

```script
// Compile-time computation
@const
fn fibonacci_const(n: i32) -> i32 {
    if n <= 1 { n } else { fibonacci_const(n-1) + fibonacci_const(n-2) }
}

// This is computed at compile time
const FIB_10: i32 = fibonacci_const(10)

@const
fn generate_lookup_table() -> [i32; 256] {
    let mut table = [0; 256]
    for i in 0..256 {
        table[i] = i * i
    }
    table
}

const SQUARES: [i32; 256] = generate_lookup_table()

// Conditional compilation
@cfg(debug)
fn debug_print(message: string) {
    print("[DEBUG] " + message)
}

@cfg(not(debug))
fn debug_print(message: string) {
    // No-op in release builds
}

@cfg(feature = "networking")
fn send_telemetry(data: TelemetryData) {
    // Only compiled if networking feature is enabled
    http_post("https://api.example.com/telemetry", data)
}
```

### Macros and Code Generation

```script
// Simple macro for creating getters/setters
macro_rules! property {
    ($name:ident, $type:ty) => {
        fn get_$name(self) -> $type {
            self.$name
        }
        
        fn set_$name(mut self, value: $type) {
            self.$name = value
        }
    }
}

struct Player {
    name: string,
    health: i32,
    score: i32
}

impl Player {
    property!(name, string)
    property!(health, i32)
    property!(score, i32)
}

// Advanced macro for creating enums with string conversion
macro_rules! string_enum {
    ($name:ident { $($variant:ident => $str:literal),* }) => {
        enum $name {
            $($variant),*
        }
        
        impl $name {
            fn to_string(self) -> string {
                match self {
                    $($name::$variant => $str),*
                }
            }
            
            fn from_string(s: string) -> Option<$name> {
                match s {
                    $($str => Some($name::$variant)),*,
                    _ => None
                }
            }
        }
    }
}

string_enum! {
    GameMode {
        SinglePlayer => "single_player",
        MultiPlayer => "multi_player",
        Tournament => "tournament"
    }
}

// Usage
let mode = GameMode::SinglePlayer
print(mode.to_string())  // "single_player"

let parsed = GameMode::from_string("tournament")
match parsed {
    Some(GameMode::Tournament) => print("Tournament mode selected"),
    _ => print("Invalid mode")
}
```

## Memory Management

Script uses automatic reference counting with cycle detection for memory safety without garbage collection pauses.

### Reference Counting Basics

```script
// References are automatically managed
fn create_shared_data() -> Rc<Vec<i32>> {
    let data = Vec::new()
    vec_push(data, 1)
    vec_push(data, 2)
    vec_push(data, 3)
    
    Rc::new(data)  // Create shared reference
}

fn use_shared_data() {
    let data1 = create_shared_data()
    let data2 = Rc::clone(data1)  // Increment reference count
    
    // Both data1 and data2 point to the same memory
    print("Data1 length: " + vec_len(Rc::deref(data1)))
    print("Data2 length: " + vec_len(Rc::deref(data2)))
    
    // Memory is freed when both references go out of scope
}
```

### Weak References for Cycle Prevention

```script
// Tree structure that could create cycles
struct TreeNode {
    value: i32,
    children: Vec<Rc<TreeNode>>,
    parent: Option<Weak<TreeNode>>  // Weak reference prevents cycles
}

impl TreeNode {
    fn new(value: i32) -> Rc<TreeNode> {
        Rc::new(TreeNode {
            value,
            children: Vec::new(),
            parent: None
        })
    }
    
    fn add_child(parent: Rc<TreeNode>, child_value: i32) -> Rc<TreeNode> {
        let child = TreeNode::new(child_value)
        
        // Set parent as weak reference
        child.parent = Some(Rc::downgrade(parent))
        
        // Add child to parent's children
        vec_push(parent.children, Rc::clone(child))
        
        child
    }
    
    fn get_path_to_root(self: Rc<TreeNode>) -> Vec<i32> {
        let mut path = Vec::new()
        let mut current = Some(self)
        
        while let Some(node) = current {
            vec_push(path, node.value)
            current = match node.parent {
                Some(weak_parent) => Weak::upgrade(weak_parent),
                None => None
            }
        }
        
        path
    }
}

// Usage - no memory leaks despite parent-child relationships
fn build_tree() {
    let root = TreeNode::new(1)
    let child1 = TreeNode::add_child(root, 2)
    let child2 = TreeNode::add_child(root, 3)
    let grandchild = TreeNode::add_child(child1, 4)
    
    let path = grandchild.get_path_to_root()
    // path contains [4, 2, 1]
}
```

### Memory-Efficient Data Structures

```script
// String interning for memory efficiency
struct StringInterner {
    strings: HashMap<string, Rc<string>>,
    reverse: HashMap<Rc<string>, i32>
}

impl StringInterner {
    fn new() -> StringInterner {
        StringInterner {
            strings: HashMap::new(),
            reverse: HashMap::new()
        }
    }
    
    fn intern(mut self, s: string) -> Rc<string> {
        match hashmap_get(self.strings, s) {
            Some(existing) => Rc::clone(existing),
            None => {
                let interned = Rc::new(s)
                hashmap_insert(self.strings, s, Rc::clone(interned))
                hashmap_insert(self.reverse, Rc::clone(interned), vec_len(self.strings))
                interned
            }
        }
    }
}

// Copy-on-write for efficient cloning
struct CowString {
    data: Rc<string>,
    is_owned: bool
}

impl CowString {
    fn new(s: string) -> CowString {
        CowString {
            data: Rc::new(s),
            is_owned: true
        }
    }
    
    fn clone(self) -> CowString {
        CowString {
            data: Rc::clone(self.data),
            is_owned: false
        }
    }
    
    fn to_mut(mut self) -> string {
        if self.is_owned && Rc::strong_count(self.data) == 1 {
            // We own the data exclusively, can modify in place
            Rc::try_unwrap(self.data).unwrap()
        } else {
            // Need to clone for modification
            let cloned = (*self.data).clone()
            self.data = Rc::new(cloned)
            self.is_owned = true
            (*self.data).clone()
        }
    }
}
```

## Performance Optimization

### Profiling and Benchmarking

```script
// Built-in profiling support
@profile
fn expensive_calculation(n: i32) -> i32 {
    let mut result = 0
    for i in 0..n {
        result += i * i
    }
    result
}

// Micro-benchmarks
@benchmark
fn bench_vector_operations() {
    let vec = Vec::new()
    for i in 0..1000 {
        vec_push(vec, i)
    }
    
    let mut sum = 0
    for i in 0..vec_len(vec) {
        match vec_get(vec, i) {
            Some(value) => sum += value,
            None => continue
        }
    }
}

@benchmark
fn bench_hashmap_operations() {
    let map = HashMap::new()
    for i in 0..1000 {
        hashmap_insert(map, i.to_string(), i)
    }
    
    let mut sum = 0
    for i in 0..1000 {
        match hashmap_get(map, i.to_string()) {
            Some(value) => sum += value,
            None => continue
        }
    }
}
```

### Zero-Cost Abstractions

```script
// Iterator patterns that compile to efficient loops
fn process_numbers_efficiently(numbers: Vec<i32>) -> Vec<i32> {
    numbers
        .iter()
        .filter(|x| x % 2 == 0)
        .map(|x| x * x)
        .take(100)
        .collect()
}

// Compiles to approximately:
fn process_numbers_optimized(numbers: Vec<i32>) -> Vec<i32> {
    let result = Vec::new()
    let count = 0
    let len = vec_len(numbers)
    
    for i in 0..len {
        if count >= 100 { break }
        
        match vec_get(numbers, i) {
            Some(num) => {
                if num % 2 == 0 {
                    vec_push(result, num * num)
                    count += 1
                }
            },
            None => continue
        }
    }
    
    result
}

// Inline functions for zero-cost abstractions
@inline
fn square(x: f32) -> f32 {
    x * x
}

@inline
fn distance_squared(p1: [f32; 2], p2: [f32; 2]) -> f32 {
    square(p2[0] - p1[0]) + square(p2[1] - p1[1])
}

// Hot path optimization
@hot
fn game_update_loop(entities: Vec<Entity>, dt: f32) {
    let count = vec_len(entities)
    for i in 0..count {
        match vec_get(entities, i) {
            Some(entity) => update_entity(entity, dt),
            None => continue
        }
    }
}

@cold
fn error_handler(error: GameError) {
    // This code is rarely executed, optimize for size
    log_error(error)
    show_error_dialog(error.message)
}
```

### Memory Layout Optimization

```script
// Struct field ordering for optimal memory layout
@repr(packed)
struct OptimizedStruct {
    // Group fields by size for better alignment
    large_field: i64,      // 8 bytes
    medium_field1: i32,    // 4 bytes
    medium_field2: i32,    // 4 bytes
    small_field1: i16,     // 2 bytes
    small_field2: i16,     // 2 bytes
    tiny_field1: i8,       // 1 byte
    tiny_field2: i8,       // 1 byte
    tiny_field3: i8,       // 1 byte
    tiny_field4: i8,       // 1 byte
    flag: bool             // 1 byte
}

// Pool allocation for frequent allocations
struct ObjectPool<T> {
    available: Vec<T>,
    in_use: Vec<T>
}

impl<T> ObjectPool<T> {
    fn new() -> ObjectPool<T> {
        ObjectPool {
            available: Vec::new(),
            in_use: Vec::new()
        }
    }
    
    fn acquire(mut self) -> Option<T> {
        match vec_pop(self.available) {
            Some(obj) => {
                vec_push(self.in_use, obj)
                Some(obj)
            },
            None => None
        }
    }
    
    fn release(mut self, obj: T) {
        // Remove from in_use and add to available
        vec_push(self.available, obj)
    }
    
    fn prealloc(mut self, count: i32, factory: fn() -> T) {
        for _ in 0..count {
            vec_push(self.available, factory())
        }
    }
}

// Usage for game objects
let bullet_pool = ObjectPool::new()
bullet_pool.prealloc(100, || Bullet::new())

fn fire_bullet(pool: ObjectPool<Bullet>, position: [f32; 2], velocity: [f32; 2]) {
    match pool.acquire() {
        Some(bullet) => {
            bullet.reset(position, velocity)
            // Use bullet...
        },
        None => {
            // Pool exhausted, could create new bullet or ignore
        }
    }
}
```

## Concurrency and Async

Script provides async/await support for non-blocking I/O operations.

### Basic Async/Await

```script
// Async functions
async fn fetch_data(url: string) -> Result<string, string> {
    let response = await http_get(url)
    match response {
        Ok(data) => Result::ok(data),
        Err(error) => Result::err("Failed to fetch: " + error)
    }
}

async fn process_urls(urls: Vec<string>) -> Vec<Result<string, string>> {
    let results = Vec::new()
    
    for url in urls {
        let result = await fetch_data(url)
        vec_push(results, result)
    }
    
    results
}

// Parallel execution
async fn fetch_all_parallel(urls: Vec<string>) -> Vec<Result<string, string>> {
    let futures = Vec::new()
    
    // Start all requests concurrently
    for url in urls {
        vec_push(futures, fetch_data(url))
    }
    
    // Wait for all to complete
    let results = await Future::join_all(futures)
    results
}
```

### Task Spawning and Management

```script
// Spawn background tasks
async fn background_processor() {
    loop {
        let work = await get_work_item()
        match work {
            Some(item) => {
                process_work_item(item)
                await sleep(100)  // Small delay
            },
            None => {
                await sleep(1000)  // Wait longer when no work
            }
        }
    }
}

async fn main() {
    // Spawn background task
    let processor_task = spawn(background_processor())
    
    // Main application logic
    while app_running() {
        await handle_user_input()
        await update_ui()
        await sleep(16)  // 60 FPS
    }
    
    // Clean shutdown
    processor_task.cancel()
    await processor_task
}
```

### Channels for Communication

```script
// Channel-based communication
async fn producer(sender: Sender<i32>) {
    for i in 0..100 {
        await sender.send(i)
        await sleep(10)
    }
    sender.close()
}

async fn consumer(receiver: Receiver<i32>) {
    while true {
        match await receiver.recv() {
            Some(value) => {
                print("Received: " + value)
                process_value(value)
            },
            None => {
                print("Channel closed")
                break
            }
        }
    }
}

async fn channel_example() {
    let (sender, receiver) = channel()
    
    let producer_task = spawn(producer(sender))
    let consumer_task = spawn(consumer(receiver))
    
    await producer_task
    await consumer_task
}
```

## Foreign Function Interface (FFI)

Script can interface with C libraries and other languages through its FFI system.

### Basic FFI Usage

```script
// Load a C library
let math_lib = ffi.load("libm.so")

// Declare functions
math_lib.declare("sin", ffi.double, [ffi.double])
math_lib.declare("cos", ffi.double, [ffi.double])
math_lib.declare("sqrt", ffi.double, [ffi.double])

// Use C functions
fn calculate_hypotenuse(a: f64, b: f64) -> f64 {
    let a_squared = a * a
    let b_squared = b * b
    math_lib.sqrt(a_squared + b_squared)
}

// Complex type mappings
let graphics_lib = ffi.load("libgraphics.so")
graphics_lib.declare("create_window", ffi.pointer, [ffi.i32, ffi.i32, ffi.pointer])
graphics_lib.declare("destroy_window", ffi.void, [ffi.pointer])
graphics_lib.declare("draw_pixel", ffi.void, [ffi.pointer, ffi.i32, ffi.i32, ffi.u32])

struct Window {
    handle: ffi.pointer
}

impl Window {
    fn new(width: i32, height: i32, title: string) -> Window {
        let title_cstr = ffi.to_cstring(title)
        let handle = graphics_lib.create_window(width, height, title_cstr)
        Window { handle }
    }
    
    fn draw_pixel(self, x: i32, y: i32, color: u32) {
        graphics_lib.draw_pixel(self.handle, x, y, color)
    }
    
    fn close(self) {
        graphics_lib.destroy_window(self.handle)
    }
}
```

### Safe FFI Wrappers

```script
// Safe wrapper for potentially unsafe C functions
struct SafeBuffer {
    data: ffi.pointer,
    size: usize,
    capacity: usize
}

impl SafeBuffer {
    fn new(capacity: usize) -> SafeBuffer {
        let data = ffi.malloc(capacity)
        if ffi.is_null(data) {
            panic("Failed to allocate memory")
        }
        
        SafeBuffer {
            data,
            size: 0,
            capacity
        }
    }
    
    fn write(mut self, offset: usize, value: u8) -> Result<(), string> {
        if offset >= self.capacity {
            Result::err("Write out of bounds")
        } else {
            ffi.write_u8(self.data, offset, value)
            self.size = max(self.size, offset + 1)
            Result::ok(())
        }
    }
    
    fn read(self, offset: usize) -> Result<u8, string> {
        if offset >= self.size {
            Result::err("Read out of bounds")
        } else {
            Result::ok(ffi.read_u8(self.data, offset))
        }
    }
}

impl Drop for SafeBuffer {
    fn drop(self) {
        ffi.free(self.data)
    }
}
```

## Advanced Collections

Script provides sophisticated collection types beyond basic arrays and hashmaps.

### Custom Collections

```script
// Doubly-linked list implementation
struct ListNode<T> {
    value: T,
    next: Option<Rc<RefCell<ListNode<T>>>>,
    prev: Option<Weak<RefCell<ListNode<T>>>>
}

struct LinkedList<T> {
    head: Option<Rc<RefCell<ListNode<T>>>>,
    tail: Option<Weak<RefCell<ListNode<T>>>>,
    length: usize
}

impl<T> LinkedList<T> {
    fn new() -> LinkedList<T> {
        LinkedList {
            head: None,
            tail: None,
            length: 0
        }
    }
    
    fn push_front(mut self, value: T) {
        let new_node = Rc::new(RefCell::new(ListNode {
            value,
            next: self.head.take(),
            prev: None
        }))
        
        match self.head {
            Some(old_head) => {
                old_head.borrow_mut().prev = Some(Rc::downgrade(new_node))
            },
            None => {
                self.tail = Some(Rc::downgrade(new_node))
            }
        }
        
        self.head = Some(new_node)
        self.length += 1
    }
    
    fn pop_front(mut self) -> Option<T> {
        self.head.take().map(|old_head| {
            match old_head.borrow_mut().next.take() {
                Some(new_head) => {
                    new_head.borrow_mut().prev = None
                    self.head = Some(new_head)
                },
                None => {
                    self.tail = None
                }
            }
            
            self.length -= 1
            Rc::try_unwrap(old_head).ok().unwrap().into_inner().value
        })
    }
    
    fn len(self) -> usize {
        self.length
    }
}

// Tree-based map for ordered data
struct TreeMap<K: Ord, V> {
    root: Option<Rc<RefCell<TreeNode<K, V>>>>
}

struct TreeNode<K: Ord, V> {
    key: K,
    value: V,
    left: Option<Rc<RefCell<TreeNode<K, V>>>>,
    right: Option<Rc<RefCell<TreeNode<K, V>>>>
}

impl<K: Ord, V> TreeMap<K, V> {
    fn new() -> TreeMap<K, V> {
        TreeMap { root: None }
    }
    
    fn insert(mut self, key: K, value: V) {
        match self.root {
            None => {
                self.root = Some(Rc::new(RefCell::new(TreeNode {
                    key, value,
                    left: None,
                    right: None
                })))
            },
            Some(root) => {
                self.insert_recursive(root, key, value)
            }
        }
    }
    
    fn insert_recursive(self, node: Rc<RefCell<TreeNode<K, V>>>, key: K, value: V) {
        let mut borrowed = node.borrow_mut()
        
        match key.cmp(&borrowed.key) {
            Ordering::Less => {
                match borrowed.left {
                    None => {
                        borrowed.left = Some(Rc::new(RefCell::new(TreeNode {
                            key, value,
                            left: None,
                            right: None
                        })))
                    },
                    Some(left) => {
                        drop(borrowed)  // Release borrow before recursion
                        self.insert_recursive(left, key, value)
                    }
                }
            },
            Ordering::Greater => {
                match borrowed.right {
                    None => {
                        borrowed.right = Some(Rc::new(RefCell::new(TreeNode {
                            key, value,
                            left: None,
                            right: None
                        })))
                    },
                    Some(right) => {
                        drop(borrowed)  // Release borrow before recursion
                        self.insert_recursive(right, key, value)
                    }
                }
            },
            Ordering::Equal => {
                borrowed.value = value  // Update existing key
            }
        }
    }
    
    fn get(self, key: &K) -> Option<V> {
        self.root.and_then(|root| self.get_recursive(root, key))
    }
    
    fn get_recursive(self, node: Rc<RefCell<TreeNode<K, V>>>, key: &K) -> Option<V> {
        let borrowed = node.borrow()
        
        match key.cmp(&borrowed.key) {
            Ordering::Less => borrowed.left.and_then(|left| self.get_recursive(left, key)),
            Ordering::Greater => borrowed.right.and_then(|right| self.get_recursive(right, key)),
            Ordering::Equal => Some(borrowed.value.clone())
        }
    }
}
```

## Error Handling Patterns

### Custom Error Types

```script
// Define application-specific error types
enum AppError {
    NetworkError(string),
    DatabaseError(string),
    ValidationError(Vec<string>),
    AuthenticationError(string),
    NotFound(string),
    InternalError(string)
}

impl AppError {
    fn message(self) -> string {
        match self {
            AppError::NetworkError(msg) => "Network error: " + msg,
            AppError::DatabaseError(msg) => "Database error: " + msg,
            AppError::ValidationError(errors) => {
                let mut msg = "Validation errors: "
                for error in errors {
                    msg += error + "; "
                }
                msg
            },
            AppError::AuthenticationError(msg) => "Authentication error: " + msg,
            AppError::NotFound(resource) => resource + " not found",
            AppError::InternalError(msg) => "Internal error: " + msg
        }
    }
    
    fn is_recoverable(self) -> bool {
        match self {
            AppError::NetworkError(_) => true,
            AppError::ValidationError(_) => true,
            AppError::NotFound(_) => true,
            _ => false
        }
    }
}

// Result type with custom error
type AppResult<T> = Result<T, AppError>

// Error conversion and chaining
fn load_user_profile(user_id: i32) -> AppResult<UserProfile> {
    // Chain multiple operations that can fail
    let user = load_user(user_id)
        .map_err(|e| AppError::DatabaseError(e))?
    
    let preferences = load_user_preferences(user_id)
        .map_err(|e| AppError::DatabaseError(e))?
    
    let avatar = load_user_avatar(user_id)
        .map_err(|e| AppError::NetworkError(e))
        .unwrap_or_else(|_| default_avatar())
    
    AppResult::ok(UserProfile {
        user,
        preferences,
        avatar
    })
}
```

### Error Recovery Strategies

```script
// Retry logic with exponential backoff
async fn retry_with_backoff<T, E>(
    operation: fn() -> Result<T, E>,
    max_attempts: i32,
    base_delay: i32
) -> Result<T, E> {
    let mut attempts = 0
    let mut delay = base_delay
    
    loop {
        match operation() {
            Ok(result) => return Ok(result),
            Err(error) => {
                attempts += 1
                if attempts >= max_attempts {
                    return Err(error)
                }
                
                await sleep(delay)
                delay *= 2  // Exponential backoff
            }
        }
    }
}

// Circuit breaker pattern
struct CircuitBreaker {
    failure_count: i32,
    last_failure_time: f64,
    state: CircuitState
}

enum CircuitState {
    Closed,
    Open,
    HalfOpen
}

impl CircuitBreaker {
    fn new() -> CircuitBreaker {
        CircuitBreaker {
            failure_count: 0,
            last_failure_time: 0.0,
            state: CircuitState::Closed
        }
    }
    
    fn call<T, E>(mut self, operation: fn() -> Result<T, E>) -> Result<T, E> {
        match self.state {
            CircuitState::Open => {
                let now = time_now()
                if now - self.last_failure_time > 60.0 {  // 1 minute timeout
                    self.state = CircuitState::HalfOpen
                } else {
                    return Err("Circuit breaker is open")
                }
            },
            _ => {}
        }
        
        match operation() {
            Ok(result) => {
                self.failure_count = 0
                self.state = CircuitState::Closed
                Ok(result)
            },
            Err(error) => {
                self.failure_count += 1
                self.last_failure_time = time_now()
                
                if self.failure_count >= 5 {
                    self.state = CircuitState::Open
                }
                
                Err(error)
            }
        }
    }
}

// Usage
let circuit_breaker = CircuitBreaker::new()

async fn safe_api_call() -> Result<ApiResponse, string> {
    retry_with_backoff(
        || circuit_breaker.call(|| make_api_request()),
        3,
        1000
    )
}
```

## Building Libraries

### Library Structure

```script
// lib.rs - Main library file
pub mod utils {
    pub mod string_utils;
    pub mod math_utils;
}

pub mod collections {
    pub mod tree;
    pub mod graph;
}

pub mod async_utils {
    pub mod retry;
    pub mod circuit_breaker;
}

// Re-export commonly used items
pub use utils::string_utils::*;
pub use collections::tree::TreeMap;
pub use async_utils::retry::retry_with_backoff;

// Library configuration
pub struct LibraryConfig {
    pub debug_mode: bool,
    pub max_retries: i32,
    pub timeout_ms: i32
}

impl Default for LibraryConfig {
    fn default() -> LibraryConfig {
        LibraryConfig {
            debug_mode: false,
            max_retries: 3,
            timeout_ms: 5000
        }
    }
}

// Global library initialization
static mut LIBRARY_CONFIG: Option<LibraryConfig> = None

pub fn initialize(config: LibraryConfig) {
    unsafe {
        LIBRARY_CONFIG = Some(config)
    }
}

pub fn get_config() -> &'static LibraryConfig {
    unsafe {
        LIBRARY_CONFIG.as_ref().unwrap_or(&LibraryConfig::default())
    }
}
```

### API Design Best Practices

```script
// Builder pattern for complex configuration
pub struct HttpClientBuilder {
    timeout: Option<i32>,
    retries: Option<i32>,
    headers: HashMap<string, string>,
    base_url: Option<string>
}

impl HttpClientBuilder {
    pub fn new() -> HttpClientBuilder {
        HttpClientBuilder {
            timeout: None,
            retries: None,
            headers: HashMap::new(),
            base_url: None
        }
    }
    
    pub fn timeout(mut self, timeout: i32) -> HttpClientBuilder {
        self.timeout = Some(timeout)
        self
    }
    
    pub fn retries(mut self, retries: i32) -> HttpClientBuilder {
        self.retries = Some(retries)
        self
    }
    
    pub fn header(mut self, key: string, value: string) -> HttpClientBuilder {
        hashmap_insert(self.headers, key, value)
        self
    }
    
    pub fn base_url(mut self, url: string) -> HttpClientBuilder {
        self.base_url = Some(url)
        self
    }
    
    pub fn build(self) -> Result<HttpClient, string> {
        let client = HttpClient {
            timeout: self.timeout.unwrap_or(5000),
            retries: self.retries.unwrap_or(3),
            headers: self.headers,
            base_url: self.base_url.unwrap_or("".to_string())
        }
        
        Result::ok(client)
    }
}

// Usage
let client = HttpClient::builder()
    .timeout(10000)
    .retries(5)
    .header("User-Agent", "MyApp/1.0")
    .base_url("https://api.example.com")
    .build()?

// Fluent interface for method chaining
pub struct QueryBuilder {
    query: string,
    params: Vec<string>,
    limit: Option<i32>,
    offset: Option<i32>
}

impl QueryBuilder {
    pub fn new() -> QueryBuilder {
        QueryBuilder {
            query: "".to_string(),
            params: Vec::new(),
            limit: None,
            offset: None
        }
    }
    
    pub fn select(mut self, fields: string) -> QueryBuilder {
        self.query = "SELECT " + fields
        self
    }
    
    pub fn from(mut self, table: string) -> QueryBuilder {
        self.query += " FROM " + table
        self
    }
    
    pub fn where_clause(mut self, condition: string) -> QueryBuilder {
        self.query += " WHERE " + condition
        self
    }
    
    pub fn limit(mut self, limit: i32) -> QueryBuilder {
        self.limit = Some(limit)
        self
    }
    
    pub fn offset(mut self, offset: i32) -> QueryBuilder {
        self.offset = Some(offset)
        self
    }
    
    pub fn build(mut self) -> string {
        if let Some(limit) = self.limit {
            self.query += " LIMIT " + limit.to_string()
        }
        
        if let Some(offset) = self.offset {
            self.query += " OFFSET " + offset.to_string()
        }
        
        self.query
    }
}

// Usage
let sql = QueryBuilder::new()
    .select("name, email")
    .from("users")
    .where_clause("active = true")
    .limit(10)
    .offset(20)
    .build()
```

---

This concludes the Advanced Features tutorial for Script! These features enable you to build sophisticated, high-performance applications while maintaining code safety and clarity. The next step is to explore the [Game Development Guide](GAME_DEV.md) to see how these advanced features apply to game development specifically.

Remember that mastering these advanced features takes practice. Start with the concepts that are most relevant to your current projects, and gradually incorporate more advanced patterns as needed.

*This tutorial is part of the Script programming language documentation. For more information, visit the [Script GitHub repository](https://github.com/moikapy/script).*