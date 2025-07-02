# Memory Safety Guide

Script provides automatic memory management through reference counting with cycle detection, ensuring memory safety without requiring manual memory management. This guide covers the memory safety guarantees, best practices, and how to work effectively with Script's memory system.

## Table of Contents

1. [Memory Safety Guarantees](#memory-safety-guarantees)
2. [Reference Counting Overview](#reference-counting-overview)
3. [Cycle Detection and Collection](#cycle-detection-and-collection)
4. [Memory Safety Patterns](#memory-safety-patterns)
5. [Performance Characteristics](#performance-characteristics)
6. [Best Practices](#best-practices)
7. [Common Pitfalls](#common-pitfalls)
8. [Debugging Memory Issues](#debugging-memory-issues)
9. [Integration with Rust Code](#integration-with-rust-code)
10. [Advanced Topics](#advanced-topics)

## Memory Safety Guarantees

Script provides the following memory safety guarantees:

### No Memory Leaks
- Automatic reference counting ensures objects are deallocated when no longer referenced
- Cycle detection prevents reference cycles from causing memory leaks
- Memory profiler detects and reports potential leaks during development

### No Use-After-Free
- References are always valid - objects cannot be freed while references exist
- Weak references fail gracefully when attempting to access freed objects
- Type system prevents accessing uninitialized or freed memory

### No Double Free
- Objects are automatically freed exactly once when their reference count reaches zero
- Runtime prevents double-free scenarios through careful reference management

### No Buffer Overflows
- All collections (arrays, strings, hashmaps) perform bounds checking
- Runtime panics on out-of-bounds access rather than corrupting memory
- String operations are UTF-8 safe and prevent buffer overruns

## Reference Counting Overview

Script uses `ScriptRc<T>` (similar to Rust's `Rc<T>`) for automatic memory management.

### Basic Usage

```script
// Creating reference-counted values
let data = "Hello, Script!";  // Automatically wrapped in ScriptRc
let numbers = Vec::new();     // Collections are reference-counted

// Cloning creates new references to the same data
let data_copy = data;         // Reference count: 2
let another_copy = data;      // Reference count: 3

// References are automatically dropped when variables go out of scope
{
    let temp_ref = data;      // Reference count: 4
}                             // Reference count: 3 (temp_ref dropped)
```

### Reference Counting Internals

The `ScriptRc<T>` implementation provides:

```rust
pub struct ScriptRc<T: ?Sized> {
    ptr: NonNull<RcBox<T>>,
    phantom: PhantomData<RcBox<T>>,
}

struct RcBox<T: ?Sized> {
    strong: AtomicUsize,    // Strong reference count
    weak: AtomicUsize,      // Weak reference count  
    value: T,               // The actual data
}
```

### Strong vs Weak References

**Strong References (`ScriptRc<T>`)**:
- Keep the object alive
- Prevent deallocation while they exist
- Used for ownership and primary access

**Weak References (`ScriptWeak<T>`)**:
- Don't keep the object alive
- Can be upgraded to strong references if object still exists
- Used to break reference cycles

```script
// Example of weak references (conceptual - actual syntax may vary)
let strong_ref = create_object();
let weak_ref = strong_ref.downgrade();

// Later...
match weak_ref.upgrade() {
    Some(strong) => {
        // Object still exists, can use it
        use_object(strong);
    },
    None => {
        // Object was deallocated
        println("Object no longer exists");
    }
}
```

## Cycle Detection and Collection

Script's garbage collector detects and breaks reference cycles to prevent memory leaks.

### How Cycles Form

Reference cycles occur when objects reference each other, directly or indirectly:

```script
// Direct cycle
let node_a = Node::new("A");
let node_b = Node::new("B");
node_a.set_next(node_b);
node_b.set_prev(node_a);  // Cycle: A -> B -> A

// Indirect cycle through multiple objects
let parent = TreeNode::new("parent");
let child = TreeNode::new("child");
let grandchild = TreeNode::new("grandchild");

parent.add_child(child);
child.add_child(grandchild);
grandchild.set_parent(parent);  // Cycle: parent -> child -> grandchild -> parent
```

### Cycle Detection Algorithm

Script uses a mark-and-sweep algorithm integrated with reference counting:

1. **Mark Phase**: Identify objects that might be in cycles
2. **Trace Phase**: Follow references to find strongly connected components
3. **Sweep Phase**: Deallocate unreachable cycles

```rust
pub struct CycleCollector {
    registered: Mutex<HashSet<usize>>,     // All ScriptRc addresses
    suspects: Mutex<HashSet<usize>>,       // Objects that might be in cycles
    allocation_count: AtomicUsize,         // Trigger collection threshold
    collection_threshold: AtomicUsize,     // When to run collection
}
```

### Collection Triggers

The garbage collector runs automatically when:
- A configurable number of allocations have occurred (default: 1000)
- Memory pressure is detected
- Manually triggered via `collect_cycles()`

### Collection Statistics

Monitor garbage collection performance:

```rust
use script::runtime::gc;

let stats = gc::get_stats().unwrap();
println!("Collections: {}", stats.collections);
println!("Objects collected: {}", stats.objects_collected);
println!("Total GC time: {:?}", stats.total_time);
```

## Memory Safety Patterns

### Safe Data Sharing

**Pattern: Immutable Sharing**
```script
// Immutable data can be safely shared
let config = ImmutableConfig::new("settings.json");
let worker1 = Worker::new(config);  // Shares reference
let worker2 = Worker::new(config);  // Shares same reference
```

**Pattern: Interior Mutability**
```script
// Use controlled mutability for shared mutable state
let counter = RefCell::new(0);
let counter_ref1 = counter;
let counter_ref2 = counter;

// Safe mutation through runtime borrow checking
counter_ref1.borrow_mut() += 1;
// counter_ref2.borrow_mut() += 1;  // Would panic - already borrowed
```

### Breaking Cycles with Weak References

**Pattern: Parent-Child Relationships**
```script
// Use weak references for back-pointers
fn create_tree() -> TreeNode {
    let parent = TreeNode::new("parent");
    let child = TreeNode::new("child");
    
    // Strong reference: parent -> child
    parent.add_child(child);
    
    // Weak reference: child -> parent (breaks cycle)
    child.set_parent_weak(parent.downgrade());
    
    parent
}
```

**Pattern: Observer Pattern**
```script
// Observers hold weak references to avoid cycles
fn setup_observer_pattern() {
    let subject = Subject::new();
    let observer = Observer::new();
    
    // Subject holds weak references to observers
    subject.add_observer(observer.downgrade());
    
    // Observer can hold strong reference to subject
    observer.set_subject(subject);
}
```

### Safe Collection Patterns

**Pattern: RAII (Resource Acquisition Is Initialization)**
```script
fn process_file(filename: string) -> Result<(), string> {
    let file = File::open(filename)?;  // Acquired
    
    // File is automatically closed when 'file' goes out of scope
    // Even if an error occurs during processing
    
    let data = file.read_all()?;
    process_data(data)?;
    
    Ok(())  // File automatically closed here
}
```

**Pattern: Scoped Lifetimes**
```script
fn process_data() {
    let large_data = load_large_dataset();  // Memory allocated
    
    {
        let processed = transform_data(large_data);  // More memory allocated
        save_results(processed);
        // 'processed' freed here, reducing memory pressure
    }
    
    // Continue with just 'large_data'
    // 'large_data' freed when function returns
}
```

## Performance Characteristics

### Reference Counting Performance

**Advantages:**
- Immediate deallocation when reference count reaches zero
- Predictable performance - no stop-the-world pauses
- Cache-friendly - objects are freed near where they're used
- Good for real-time applications (games, audio processing)

**Costs:**
- Reference count updates on every assignment/drop
- Atomic operations for thread safety (future feature)
- Cannot handle cycles without additional collection

### Memory Overhead

Each `ScriptRc<T>` has a small overhead:
- 8 bytes for the pointer to RcBox
- 16 bytes for RcBox header (strong count + weak count)
- Negligible for most applications

### Cycle Collection Performance

- **Mark Phase**: O(n) where n is the number of suspected objects
- **Trace Phase**: O(m) where m is the number of references to trace
- **Sweep Phase**: O(k) where k is the number of objects to deallocate
- Typically runs in microseconds for small programs

## Best Practices

### Design for Reference Counting

1. **Prefer Tree Structures**: Hierarchical data structures work naturally with reference counting
2. **Use Weak References for Back-Pointers**: Break cycles at design time
3. **Minimize Shared Mutable State**: Reduces complexity and potential for cycles
4. **Design Clear Ownership**: Make it clear which objects own which resources

### Memory Efficient Patterns

```script
// Good: Use local variables for temporary data
fn process_items(items: Vec<Item>) {
    for item in items {
        let processed = expensive_transform(item);
        save_result(processed);
        // 'processed' freed immediately after each iteration
    }
}

// Avoid: Accumulating temporary objects
fn process_items_bad(items: Vec<Item>) -> Vec<ProcessedItem> {
    let mut results = Vec::new();
    for item in items {
        let processed = expensive_transform(item);
        results.push(processed);  // All objects kept in memory
    }
    results  // Memory usage peaks at the end
}
```

### Cycle Prevention

```script
// Good: Use weak references for non-owning relationships
struct Node {
    children: Vec<Node>,        // Strong references (ownership)
    parent: WeakRef<Node>,      // Weak reference (non-owning)
}

// Good: Use IDs instead of direct references
struct Entity {
    id: EntityId,
    components: Vec<ComponentId>,  // Reference by ID, not direct pointer
}

struct World {
    entities: HashMap<EntityId, Entity>,
    components: HashMap<ComponentId, Component>,
}
```

### Resource Management

```script
// Good: RAII pattern
fn download_file(url: string) -> Result<(), string> {
    let connection = HttpConnection::new()?;  // Resource acquired
    let response = connection.get(url)?;
    let file = File::create("download.tmp")?;
    
    file.write_all(response.body())?;
    
    Ok(())
    // All resources automatically cleaned up here
}

// Good: Explicit cleanup for critical resources
fn process_database() -> Result<(), string> {
    let db = Database::connect("localhost")?;
    
    let result = db.query("SELECT * FROM users")?;
    process_results(result);
    
    db.close();  // Explicit cleanup for database connections
    Ok(())
}
```

## Common Pitfalls

### Reference Cycles

**Problem**: Objects referencing each other prevent deallocation
```script
// Creates a reference cycle
let node_a = Node::new();
let node_b = Node::new();
node_a.set_next(node_b);
node_b.set_next(node_a);  // Cycle!
```

**Solution**: Use weak references or redesign data structure
```script
// Break the cycle with a weak reference
let node_a = Node::new();
let node_b = Node::new();
node_a.set_next(node_b);
node_b.set_prev_weak(node_a.downgrade());  // Weak reference
```

### Temporary Object Accumulation

**Problem**: Creating many temporary objects that aren't immediately freed
```script
// Inefficient: Creates many intermediate strings
fn build_message(parts: Vec<string>) -> string {
    let mut result = "";
    for part in parts {
        result = result + " " + part;  // Creates new string each time
    }
    result
}
```

**Solution**: Use efficient building patterns
```script
// Efficient: Use StringBuilder or similar
fn build_message(parts: Vec<string>) -> string {
    let builder = StringBuilder::new();
    for part in parts {
        builder.append(" ");
        builder.append(part);
    }
    builder.to_string()
}
```

### Holding References Too Long

**Problem**: Keeping references to large objects longer than necessary
```script
fn process_large_dataset() {
    let dataset = load_huge_dataset();  // 1GB of data
    
    let summary = create_summary(dataset);  // Only need summary
    
    // ... lots of other processing ...
    
    // Still holding reference to 1GB dataset!
    return summary;
}
```

**Solution**: Drop references explicitly when done
```script
fn process_large_dataset() {
    let dataset = load_huge_dataset();
    let summary = create_summary(dataset);
    
    // Explicitly drop large dataset
    drop(dataset);  // Or let it go out of scope
    
    // Continue with just the summary
    return summary;
}
```

## Debugging Memory Issues

### Memory Profiling

Enable memory profiling to track allocations:

```rust
use script::runtime::profiler;

// Check for memory leaks
if profiler::check_leaks() {
    if let Some(report) = profiler::generate_report() {
        eprintln!("Memory issues detected:\n{}", report);
    }
}

// Get detailed statistics
if let Some(stats) = profiler::get_stats() {
    println!("Current memory usage: {} bytes", stats.allocations.current_memory);
    println!("Peak memory usage: {} bytes", stats.allocations.peak_memory);
    println!("Potential leaks: {}", stats.allocations.potential_leaks);
    
    // Type-specific statistics
    for (type_name, type_stats) in &stats.type_stats {
        if type_stats.current_bytes > 1024 * 1024 {  // > 1MB
            println!("Large allocation - {}: {} bytes", 
                    type_name, type_stats.current_bytes);
        }
    }
}
```

### Garbage Collection Monitoring

Monitor GC behavior to identify performance issues:

```rust
use script::runtime::gc;

let stats = gc::get_stats().unwrap();
if stats.total_time.as_millis() > 100 {
    println!("Warning: GC taking too long ({:?})", stats.total_time);
    println!("Consider reducing allocation rate or adjusting GC threshold");
}

if stats.collections > 1000 {
    println!("Warning: Too many GC cycles ({})", stats.collections);
    println!("Consider optimizing allocation patterns");
}
```

### Runtime Configuration for Debugging

```rust
use script::runtime::{Runtime, RuntimeConfig};

let debug_config = RuntimeConfig {
    max_heap_size: 10 * 1024 * 1024,  // Small heap to trigger issues early
    enable_profiling: true,            // Always enable profiling
    gc_threshold: 10,                  // Aggressive GC for testing
    enable_panic_handler: true,        // Capture all panics
    ..Default::default()
};

Runtime::initialize_with_config(debug_config)?;
```

### Debugging Tools

**Memory Snapshots**: Take periodic snapshots to track memory growth
```rust
fn take_memory_snapshot(label: &str) {
    if let Some(stats) = profiler::get_stats() {
        println!("Snapshot '{}': {} bytes, {} objects", 
                label, 
                stats.allocations.current_memory,
                stats.allocations.total_allocations - stats.allocations.total_deallocations);
    }
}

// Usage
take_memory_snapshot("before_processing");
process_data();
take_memory_snapshot("after_processing");
```

**Leak Detection**: Run periodic leak checks
```rust
fn periodic_leak_check() {
    if profiler::check_leaks() {
        eprintln!("Memory leak detected!");
        if let Some(report) = profiler::generate_report() {
            eprintln!("{}", report);
        }
    }
}
```

## Integration with Rust Code

### Using ScriptRc in Rust

```rust
use script::runtime::ScriptRc;

// Create Script-managed data from Rust
let data = ScriptRc::new("Hello from Rust".to_string());

// Pass to Script code
call_script_function(data.clone());

// Check reference count
println!("Reference count: {}", data.strong_count());
```

### Converting Between Rust and Script Types

```rust
use script::stdlib::ScriptValue;

// Convert Rust values to Script values
let rust_string = "Hello".to_string();
let script_value = ScriptValue::String(ScriptRc::new(rust_string.into()));

// Convert Script values back to Rust
if let Some(script_string) = script_value.as_string() {
    let rust_string: String = script_string.to_string();
}
```

### Safe FFI Patterns

```rust
// Safe wrapper for exposing Rust functions to Script
pub fn safe_rust_function(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    // Validate arguments
    let arg1 = args.get(0)
        .and_then(|v| v.as_string())
        .ok_or_else(|| RuntimeError::InvalidOperation("Expected string".to_string()))?;
    
    // Call Rust function safely
    let result = my_rust_function(arg1.as_str())
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;
    
    // Convert result back to Script value
    Ok(ScriptValue::String(ScriptRc::new(result.into())))
}
```

## Advanced Topics

### Custom Allocators

Script supports custom allocators for specialized use cases:

```rust
use script::runtime::ScriptAllocator;

// Use Script's allocator globally
#[global_allocator]
static GLOBAL: ScriptAllocator = ScriptAllocator;

// Or use for specific allocations
use std::alloc::{GlobalAlloc, Layout};

unsafe {
    let layout = Layout::new::<MyStruct>();
    let ptr = GLOBAL.alloc(layout);
    // Use ptr...
    GLOBAL.dealloc(ptr, layout);
}
```

### Memory Pool Integration

For high-performance applications, integrate with memory pools:

```rust
pub struct PooledAllocator {
    small_pool: Pool<SmallObject>,
    large_pool: Pool<LargeObject>,
    fallback: ScriptAllocator,
}

impl PooledAllocator {
    pub fn allocate(&self, size: usize) -> *mut u8 {
        match size {
            0..=64 => self.small_pool.allocate(),
            65..=1024 => self.large_pool.allocate(),
            _ => unsafe { self.fallback.alloc(Layout::from_size_align(size, 8).unwrap()) }
        }
    }
}
```

### Weak Reference Patterns

Advanced weak reference patterns for complex data structures:

```rust
// Observer pattern with automatic cleanup
pub struct Subject<T> {
    value: T,
    observers: Vec<ScriptWeak<dyn Observer<T>>>,
}

impl<T> Subject<T> {
    pub fn notify(&mut self) {
        // Automatically remove dead observers
        self.observers.retain(|weak| {
            if let Some(observer) = weak.upgrade() {
                observer.notify(&self.value);
                true
            } else {
                false  // Observer was dropped, remove weak reference
            }
        });
    }
}
```

### Performance Optimization

**Batch Allocation Patterns**: Allocate related objects together
```rust
// Allocate a batch of related objects
let batch = ScriptRc::new(ObjectBatch::new(1000));
for i in 0..1000 {
    let obj = batch.get_object(i);  // No individual allocations
    process_object(obj);
}
// Single deallocation when batch is dropped
```

**Copy-on-Write Patterns**: Share data until modification
```rust
pub struct CowData<T: Clone> {
    data: ScriptRc<T>,
}

impl<T: Clone> CowData<T> {
    pub fn get(&self) -> &T {
        &*self.data
    }
    
    pub fn get_mut(&mut self) -> &mut T {
        // Clone data if there are other references
        if self.data.strong_count() > 1 {
            self.data = ScriptRc::new((*self.data).clone());
        }
        ScriptRc::get_mut(&mut self.data).unwrap()
    }
}
```

Script's memory safety system provides strong guarantees while maintaining performance and ease of use. By understanding the reference counting model, cycle detection system, and following the patterns and best practices outlined in this guide, you can write Script programs that are both memory-safe and efficient.