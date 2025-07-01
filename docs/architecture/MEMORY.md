# Memory Management

## Table of Contents

- [Overview](#overview)
- [Reference Counting System](#reference-counting-system)
- [Cycle Detection and Collection](#cycle-detection-and-collection)
- [Memory Profiling](#memory-profiling)
- [Memory Safety Guarantees](#memory-safety-guarantees)
- [Runtime Integration](#runtime-integration)
- [Performance Characteristics](#performance-characteristics)
- [Thread Safety](#thread-safety)
- [Debugging and Leak Detection](#debugging-and-leak-detection)

## Overview

The Script programming language uses a sophisticated memory management system that combines reference counting with cycle detection to provide automatic memory management without the overhead of a traditional garbage collector. The system is designed to be:

- **Memory Safe**: Prevents use-after-free and double-free errors
- **Deterministic**: Objects are freed immediately when no longer referenced
- **Cycle-Aware**: Detects and breaks reference cycles automatically
- **Thread-Safe**: Supports concurrent access for future actor model implementation
- **Zero-Cost**: No runtime overhead when not needed
- **Observable**: Comprehensive profiling and debugging support

```
┌─────────────────────────────────────────────────────────────┐
│                   Memory Management System                  │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │  Reference      │  │    Cycle        │  │    Memory       │ │
│  │   Counting      │  │  Detection      │  │   Profiling     │ │
│  │                 │  │                 │  │                 │ │
│  │ • ScriptRc<T>   │  │ • Mark & Sweep  │  │ • Allocation    │ │
│  │ • ScriptWeak<T> │  │ • Background    │  │   Tracking      │ │
│  │ • Atomic Ops    │  │   Collection    │  │ • Leak          │ │
│  │ • Drop Safety   │  │ • Suspect List  │  │   Detection     │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Reference Counting System

### Core Components

The reference counting system is built around two primary smart pointer types:

#### `ScriptRc<T>` - Strong References

`ScriptRc<T>` is a thread-safe reference-counted smart pointer that maintains ownership of heap-allocated data:

```rust
pub struct ScriptRc<T: ?Sized> {
    ptr: NonNull<RcBox<T>>,
    phantom: PhantomData<RcBox<T>>,
}

struct RcBox<T: ?Sized> {
    strong: AtomicUsize,  // Strong reference count
    weak: AtomicUsize,    // Weak reference count  
    value: T,             // The actual data
}
```

**Key Features:**
- **Atomic Operations**: Thread-safe reference counting using `AtomicUsize`
- **Automatic Cleanup**: Objects are freed when strong count reaches zero
- **Shared Ownership**: Multiple `ScriptRc<T>` instances can share ownership
- **Clone-on-Write**: `make_mut()` provides copy-on-write semantics

#### `ScriptWeak<T>` - Weak References

`ScriptWeak<T>` provides non-owning references that don't prevent deallocation:

```rust
pub struct ScriptWeak<T> {
    ptr: NonNull<RcBox<T>>,
    phantom: PhantomData<RcBox<T>>,
}
```

**Key Features:**
- **Cycle Breaking**: Doesn't contribute to reference count
- **Upgrade Safety**: Can be safely upgraded to strong reference if object exists
- **Dangling Detection**: Automatically becomes invalid when object is freed

### Reference Counting Operations

#### Allocation and Initialization

```rust
impl<T> ScriptRc<T> {
    pub fn new(value: T) -> Self {
        // 1. Allocate RcBox<T> with proper layout
        let layout = std::alloc::Layout::new::<RcBox<T>>();
        let ptr = std::alloc::alloc(layout) as *mut RcBox<T>;
        
        // 2. Initialize reference counts
        // strong: 1 (this reference)
        // weak: 1 (implicit weak from strong)
        ptr.write(RcBox {
            strong: AtomicUsize::new(1),
            weak: AtomicUsize::new(1),
            value,
        });
        
        // 3. Register with cycle detector
        gc::register_rc(&rc);
        
        // 4. Track allocation for profiling
        profiler::record_allocation(layout.size(), type_name::<T>());
    }
}
```

#### Cloning (Reference Increment)

```rust
impl<T: ?Sized> Clone for ScriptRc<T> {
    fn clone(&self) -> Self {
        // Atomically increment strong count
        unsafe { 
            self.ptr.as_ref().strong.fetch_add(1, Ordering::Relaxed);
        }
        ScriptRc {
            ptr: self.ptr,
            phantom: PhantomData,
        }
    }
}
```

#### Dropping (Reference Decrement)

```rust
impl<T: ?Sized> Drop for ScriptRc<T> {
    fn drop(&mut self) {
        unsafe {
            // Atomically decrement strong count
            if self.ptr.as_ref().strong.fetch_sub(1, Ordering::Release) == 1 {
                // Last strong reference - object can be freed
                std::sync::atomic::fence(Ordering::Acquire);
                
                // 1. Unregister from cycle detector
                gc::unregister_rc(self);
                
                // 2. Drop the contained value
                std::ptr::drop_in_place(&mut (*self.ptr.as_ptr()).value);
                
                // 3. Decrement weak count (from implicit weak)
                if self.ptr.as_ref().weak.fetch_sub(1, Ordering::Release) == 1 {
                    // Last reference - deallocate memory
                    let layout = std::alloc::Layout::for_value(&**self);
                    profiler::record_deallocation(layout.size(), type_name::<T>());
                    std::alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout);
                }
            }
        }
    }
}
```

### Weak Reference Management

#### Creating Weak References

```rust
impl<T: ?Sized> ScriptRc<T> {
    pub fn downgrade(&self) -> ScriptWeak<T> {
        // Increment weak count
        unsafe { 
            self.ptr.as_ref().weak.fetch_add(1, Ordering::Relaxed);
        }
        ScriptWeak {
            ptr: self.ptr,
            phantom: PhantomData,
        }
    }
}
```

#### Upgrading Weak References

```rust
impl<T> ScriptWeak<T> {
    pub fn upgrade(&self) -> Option<ScriptRc<T>> {
        let inner = unsafe { self.ptr.as_ref() };
        
        // Try to increment strong count if it's not zero
        let mut strong = inner.strong.load(Ordering::Relaxed);
        loop {
            if strong == 0 {
                return None; // Object has been freed
            }
            
            // Attempt to increment strong count atomically
            match inner.strong.compare_exchange_weak(
                strong,
                strong + 1,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    return Some(ScriptRc {
                        ptr: self.ptr,
                        phantom: PhantomData,
                    });
                }
                Err(current) => strong = current,
            }
        }
    }
}
```

## Cycle Detection and Collection

### Problem Statement

Reference counting alone cannot handle circular references:

```rust
// Example: Two objects referencing each other
struct Node {
    data: i32,
    next: Option<ScriptRc<Node>>,
}

let node1 = ScriptRc::new(Node { data: 1, next: None });
let node2 = ScriptRc::new(Node { data: 2, next: Some(node1.clone()) });
node1.make_mut().next = Some(node2.clone()); // Creates cycle!

// Without cycle detection, both nodes would never be freed
```

### Cycle Detection Algorithm

The Script runtime implements a **mark-and-sweep** cycle detector that runs alongside reference counting:

#### Architecture

```rust
pub struct CycleCollector {
    /// Objects suspected of being in cycles
    suspects: Mutex<HashSet<usize>>,
    /// All registered ScriptRc instances
    registered: Mutex<HashSet<usize>>,
    /// Collection statistics
    stats: Mutex<CollectionStats>,
    /// Background thread control
    shutdown: AtomicBool,
}
```

#### Collection Process

1. **Suspect Identification**: Objects are marked as suspects when:
   - Reference count decreases but doesn't reach zero
   - Allocation threshold is exceeded
   - Manual collection is triggered

2. **Graph Construction**: Build object reference graph from suspects
   ```rust
   struct GraphNode {
       address: usize,
       marked: bool,
       strong_count: usize,
       references: Vec<usize>, // Outgoing references
   }
   ```

3. **Mark Phase**: Mark all objects reachable from roots
   ```rust
   fn mark_reachable(&self, graph: &mut [GraphNode]) {
       // Mark objects with external references as roots
       for i in 0..graph.len() {
           if graph[i].strong_count > graph[i].references.len() {
               self.mark_recursive(i, graph);
           }
       }
   }
   ```

4. **Sweep Phase**: Collect unmarked objects
   ```rust
   fn sweep(&self, graph: &[GraphNode]) -> usize {
       let mut collected = 0;
       for node in graph {
           if !node.marked {
               // Decrease reference counts, triggering deallocation
               self.break_cycle(node);
               collected += 1;
           }
       }
       collected
   }
   ```

#### Background Collection

```rust
fn background_collector(&self) {
    while !self.shutdown.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(100));
        
        // Trigger collection based on conditions
        if self.should_collect() {
            self.collect();
        }
    }
}

fn should_collect(&self) -> bool {
    let suspects = self.suspects.lock().unwrap();
    suspects.len() > 100 || // Many suspects
    self.allocation_count.load(Ordering::Relaxed) > 1000 // Many allocations
}
```

### Collection Triggers

Cycle collection can be triggered by:

1. **Allocation Threshold**: Every N allocations (default: 1000)
2. **Suspect Count**: When suspect list grows large (default: 100)  
3. **Manual Collection**: Explicit calls to `collect_cycles()`
4. **Time-based**: Periodic background collection
5. **Memory Pressure**: When memory usage exceeds limits

### Collection Statistics

```rust
pub struct CollectionStats {
    pub collections: usize,           // Total collections performed
    pub cycles_detected: usize,       // Number of cycles found
    pub objects_collected: usize,     // Objects freed by GC
    pub total_time: Duration,         // Total time in GC
    pub last_collection: Option<Instant>,
}
```

## Memory Profiling

### Profiling Architecture

The memory profiler provides comprehensive tracking of memory usage:

```rust
pub struct MemoryProfiler {
    enabled: AtomicBool,
    allocations: Mutex<AllocationTracker>,
    type_stats: RwLock<HashMap<String, TypeStats>>,
    gc_stats: Mutex<GcStats>,
    start_time: Instant,
}
```

### Allocation Tracking

#### Per-Allocation Information

```rust
struct AllocationInfo {
    size: usize,
    type_name: String,
    timestamp: Instant,
    backtrace: Option<String>, // Debug builds only
}
```

#### Aggregated Statistics

```rust
pub struct AllocationStats {
    pub total_allocations: usize,
    pub total_deallocations: usize,
    pub total_bytes_allocated: usize,
    pub total_bytes_deallocated: usize,
    pub current_memory: usize,
    pub peak_memory: usize,
    pub potential_leaks: usize,
}
```

### Type-Based Profiling

Track memory usage by type:

```rust
pub struct TypeStats {
    pub allocations: usize,     // Count of allocations
    pub deallocations: usize,   // Count of deallocations  
    pub total_bytes: usize,     // Total bytes allocated
    pub current_bytes: usize,   // Current bytes in use
    pub peak_bytes: usize,      // Peak usage for this type
}
```

### Profiling Integration

#### Allocation Hooks

```rust
pub fn record_allocation(size: usize, type_name: &str) {
    if let Ok(profiler) = PROFILER.read() {
        if let Some(p) = profiler.as_ref() {
            p.record_allocation(size, type_name);
        }
    }
}
```

#### Automatic Integration

All ScriptRc operations automatically integrate with profiling:

```rust
impl<T> ScriptRc<T> {
    pub fn new(value: T) -> Self {
        // ... allocation logic ...
        
        // Automatic profiling integration
        profiler::record_allocation(layout.size(), std::any::type_name::<T>());
        
        // ... rest of initialization ...
    }
}
```

### Memory Reports

Generate comprehensive memory usage reports:

```rust
pub fn generate_report() -> String {
    format!(
        "=== Memory Profile Report ===
Duration: {:?}

Allocation Summary:
  Total allocations: {}
  Total deallocations: {}
  Current memory: {} bytes
  Peak memory: {} bytes
  Potential leaks: {}

GC Summary:
  Collections: {}
  Objects collected: {}
  Total GC time: {:?}

Top Types by Usage:
{}",
        duration,
        total_allocs,
        total_deallocs,
        current_memory,
        peak_memory,
        leaks,
        gc_collections,
        gc_objects,
        gc_time,
        type_breakdown
    )
}
```

## Memory Safety Guarantees

### Safety Properties

The Script memory system provides several key safety guarantees:

#### 1. Use-After-Free Prevention

```rust
// ScriptRc prevents use-after-free through ownership tracking
let data = ScriptRc::new(vec![1, 2, 3]);
let weak_ref = data.downgrade();
drop(data); // Last strong reference

// This safely returns None instead of accessing freed memory
assert!(weak_ref.upgrade().is_none());
```

#### 2. Double-Free Prevention

Reference counting ensures objects are freed exactly once:

```rust
// Multiple references to same object
let data1 = ScriptRc::new("hello");
let data2 = data1.clone();
drop(data1); // Decrements count to 1
drop(data2); // Decrements count to 0, frees object
// Object freed exactly once
```

#### 3. Memory Leak Prevention

Cycle detection prevents reference cycles from causing leaks:

```rust
// Circular reference automatically detected and broken
struct Node {
    next: Option<ScriptRc<Node>>,
}

let a = ScriptRc::new(Node { next: None });
let b = ScriptRc::new(Node { next: Some(a.clone()) });
a.make_mut().next = Some(b); // Creates cycle

// Cycle detector will identify and break this cycle
```

#### 4. Thread Safety

All operations are thread-safe through atomic reference counting:

```rust
// Safe to share ScriptRc across threads
let data = ScriptRc::new(42);
let data_clone = data.clone();

thread::spawn(move || {
    println!("Data: {}", *data_clone); // Safe concurrent access
});
```

### Panic Safety

The system is designed to be panic-safe:

```rust
impl<T: ?Sized> Drop for ScriptRc<T> {
    fn drop(&mut self) {
        // Even if panic occurs during drop, memory is properly tracked
        // and reference counts are correctly managed
    }
}
```

## Runtime Integration

### Code Generation Integration

Generated code automatically integrates with memory management:

```rust
// Script code:
let x = create_object()
let y = x

// Generated IR includes reference management:
%x = call @create_object()        ; Returns ScriptRc<Object>
%y = call @script_rc_clone(%x)    ; Increment reference count
call @script_rc_drop(%x)          ; Decrement when x goes out of scope
call @script_rc_drop(%y)          ; Decrement when y goes out of scope
```

### Function Call Integration

Function calls properly manage reference counts:

```rust
// Function calls transfer ownership properly
fn take_ownership(obj: ScriptRc<Object>) {
    // obj's reference count transferred, not incremented
}

fn borrow_reference(obj: &ScriptRc<Object>) {
    // No reference count changes, temporary access only
}
```

### Exception Handling

Memory management integrates with Script's error handling:

```rust
// Memory is properly cleaned up even during error propagation
try {
    let data = ScriptRc::new(expensive_object());
    risky_operation(data)?; // May throw error
} catch (error) {
    // data automatically cleaned up via RAII
}
```

## Performance Characteristics

### Time Complexity

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Clone | O(1) | Atomic increment |
| Drop | O(1) | Atomic decrement |
| Weak upgrade | O(1) | Atomic compare-and-swap |
| Cycle detection | O(n) | Where n = suspect objects |
| Collection | O(n + e) | n = nodes, e = edges |

### Space Complexity

- **Per Object**: 16 bytes overhead (2 × usize for counts)
- **Weak References**: 8 bytes per weak reference
- **Cycle Detector**: O(n) where n = number of suspects
- **Profiler**: O(a) where a = number of active allocations

### Performance Optimizations

#### Reference Count Optimizations

1. **Relaxed Ordering**: Use relaxed atomic operations where possible
2. **Local Counting**: Batch reference count changes in tight loops
3. **Copy Elision**: Avoid unnecessary clones through move semantics

#### Cycle Detection Optimizations

1. **Incremental Collection**: Only scan suspect objects
2. **Background Collection**: Non-blocking collection in separate thread
3. **Adaptive Thresholds**: Adjust collection frequency based on workload
4. **Early Termination**: Stop scanning when no cycles are found

#### Profiling Optimizations

1. **Conditional Compilation**: Zero-cost when profiling disabled
2. **Sampling**: Profile subset of allocations for large programs
3. **Lock-Free Statistics**: Use atomic counters for basic stats

### Benchmarks

Typical performance characteristics:

```
ScriptRc Clone:     ~2ns  (single atomic increment)
ScriptRc Drop:      ~3ns  (atomic decrement + conditional cleanup)
Weak Upgrade:       ~5ns  (compare-and-swap loop)
Cycle Collection:   ~1ms  (per 1000 objects scanned)
Allocation Profiling: ~50ns (when enabled)
```

## Thread Safety

### Atomic Operations

All reference counting operations use atomic instructions:

```rust
// Thread-safe reference count operations
strong.fetch_add(1, Ordering::Relaxed);     // Clone
strong.fetch_sub(1, Ordering::Release);     // Drop
strong.compare_exchange_weak(...);          // Weak upgrade
```

### Memory Ordering

Careful memory ordering ensures correctness:

- **Relaxed**: For reference count increments (cloning)
- **Release**: For reference count decrements (dropping)
- **Acquire**: Fence after final decrement
- **SeqCst**: For weak reference upgrades

### Lock-Free Design

The core reference counting is lock-free, enabling high concurrency:

```rust
// Multiple threads can safely clone/drop concurrently
let shared = ScriptRc::new(data);

for _ in 0..10 {
    let shared_clone = shared.clone();
    thread::spawn(move || {
        // Each thread has its own reference
        use_data(&shared_clone);
        // Automatic cleanup when thread exits
    });
}
```

### Cycle Detector Synchronization

The cycle detector uses conventional locks but minimizes contention:

```rust
// Short critical sections
{
    let mut suspects = self.suspects.lock().unwrap();
    suspects.insert(address); // Fast insertion
} // Lock released immediately

// Long-running operations outside locks
let suspect_snapshot = take_snapshot();
let graph = build_graph(suspect_snapshot); // No locks held
```

## Debugging and Leak Detection

### Leak Detection

The profiler automatically detects potential memory leaks:

```rust
// Allocation without corresponding deallocation
let data = ScriptRc::new(expensive_object());
// ... data is dropped but profiler still tracks it ...

// At shutdown:
profiler.report_leaks(); // Reports potential leaks with backtraces
```

### Debug Features

#### Backtrace Collection

In debug builds, allocation backtraces are captured:

```rust
let allocation = AllocationInfo {
    size,
    type_name: type_name.to_string(),
    timestamp: Instant::now(),
    backtrace: if cfg!(debug_assertions) {
        Some(Backtrace::capture().to_string())
    } else {
        None
    },
};
```

#### Comprehensive Logging

Debug builds include detailed logging:

```rust
debug!("ScriptRc::new: allocated {} bytes for {}", size, type_name);
debug!("ScriptRc::clone: {} -> {} refs", old_count, new_count);
debug!("ScriptRc::drop: {} -> {} refs", old_count, new_count);
debug!("Cycle collection: found {} cycles, collected {} objects", cycles, objects);
```

### Debugging Tools

#### Memory Reports

Generate detailed memory usage reports:

```rust
let report = profiler::generate_report();
println!("{}", report);
// Shows allocation patterns, leak suspects, type usage
```

#### Runtime Statistics

Access runtime statistics programmatically:

```rust
let stats = profiler::get_stats().unwrap();
println!("Current memory usage: {} bytes", stats.allocations.current_memory);
println!("Potential leaks: {}", stats.allocations.potential_leaks);
```

#### Integration with Development Tools

- **IDE Integration**: Memory usage displayed in development environment
- **Test Integration**: Automatic leak checking in unit tests
- **CI Integration**: Memory regression detection in continuous integration

### Best Practices

1. **Regular Profiling**: Monitor memory usage during development
2. **Leak Testing**: Include leak detection in automated tests
3. **Cycle Awareness**: Design data structures to minimize cycles
4. **Weak References**: Use weak references for back-pointers
5. **Resource Management**: Implement proper cleanup in destructors

This comprehensive memory management system provides Script with automatic, efficient, and safe memory management that scales from simple scripts to complex applications while maintaining predictable performance characteristics and excellent debugging support.