# Script Runtime System Guide

The Script programming language runtime provides a comprehensive execution environment with automatic memory management, panic handling, and memory profiling. This guide covers the architecture, configuration, and integration of the runtime system.

## Table of Contents

1. [Runtime Architecture](#runtime-architecture)
2. [Runtime Initialization](#runtime-initialization)
3. [Execution Model](#execution-model)
4. [Configuration Options](#configuration-options)
5. [Memory Management Integration](#memory-management-integration)
6. [Panic Handling](#panic-handling)
7. [Performance Monitoring](#performance-monitoring)
8. [Embedding the Runtime](#embedding-the-runtime)
9. [Thread Safety](#thread-safety)
10. [Best Practices](#best-practices)

## Runtime Architecture

The Script runtime is built on a modular architecture with several key subsystems:

```
┌─────────────────────────────────────────────────────────┐
│                    Script Runtime                       │
├─────────────────┬─────────────────┬─────────────────────┤
│   Core Runtime  │ Memory Manager  │   Panic Handler     │
│   - Lifecycle   │ - Allocations   │   - Stack Traces    │
│   - Config      │ - Heap Limits   │   - Error Recovery  │
│   - Metadata    │ - Statistics    │   - Custom Hooks    │
├─────────────────┼─────────────────┼─────────────────────┤
│  Memory Profiler│ Cycle Collector │   Type Registry     │
│  - Allocation   │ - Leak Detection│   - Dynamic Types   │
│    Tracking     │ - Reference     │   - Reflection      │
│  - Leak Reports │   Cycles        │   - Metadata        │
└─────────────────┴─────────────────┴─────────────────────┘
```

### Core Components

- **Runtime Core**: Manages initialization, configuration, and coordination between subsystems
- **Memory Manager**: Handles memory allocation with heap limits and statistics tracking
- **Reference Counting**: Automatic memory management with `ScriptRc<T>` smart pointers
- **Cycle Collector**: Detects and breaks reference cycles to prevent memory leaks
- **Memory Profiler**: Tracks allocations and provides detailed profiling information
- **Panic Handler**: Provides panic recovery with stack traces and debugging context

## Runtime Initialization

### Basic Initialization

```rust
use script::runtime;

// Initialize with default configuration
runtime::initialize()?;

// Your Script code execution here...

// Shutdown the runtime
runtime::shutdown()?;
```

### Custom Configuration

```rust
use script::runtime::{Runtime, RuntimeConfig};

let config = RuntimeConfig {
    max_heap_size: 100 * 1024 * 1024,  // 100MB limit
    enable_profiling: true,
    enable_gc: true,
    gc_threshold: 500,                   // Collect every 500 allocations
    enable_panic_handler: true,
    stack_size: 4 * 1024 * 1024,        // 4MB stack
};

Runtime::initialize_with_config(config)?;
```

### Configuration Options

```rust
pub struct RuntimeConfig {
    /// Maximum heap size in bytes (0 = unlimited)
    pub max_heap_size: usize,
    
    /// Enable memory profiling (default: debug builds only)
    pub enable_profiling: bool,
    
    /// Enable cycle detection and collection
    pub enable_gc: bool,
    
    /// Number of allocations between GC cycles
    pub gc_threshold: usize,
    
    /// Enable panic handler with stack traces
    pub enable_panic_handler: bool,
    
    /// Stack size for Script execution threads
    pub stack_size: usize,
}
```

## Execution Model

The Script runtime uses a single-threaded execution model with provisions for future multi-threading support.

### Execution Flow

1. **Runtime Initialization**: Set up memory management, GC, and panic handling
2. **Code Loading**: Parse and compile Script source code to IR
3. **Type Checking**: Verify types and generate type constraints
4. **Code Generation**: Compile IR to native code (Cranelift or LLVM)
5. **Execution**: Run the compiled code within the runtime environment
6. **Memory Management**: Automatic reference counting with cycle detection
7. **Cleanup**: Runtime shutdown and resource cleanup

### Protected Execution

The runtime provides panic-safe execution with automatic recovery:

```rust
use script::runtime;

let runtime = runtime::runtime()?;

// Execute Script code with panic protection
let result = runtime.execute_protected(|| {
    // Your Script code execution
    run_script_function()
});

match result {
    Ok(value) => println!("Script executed successfully: {:?}", value),
    Err(runtime_error) => {
        eprintln!("Script execution failed: {}", runtime_error);
        // Runtime remains in valid state
    }
}
```

## Memory Management Integration

### Reference Counting

Script uses `ScriptRc<T>` for automatic memory management:

```rust
use script::runtime::ScriptRc;

// Create reference-counted value
let data = ScriptRc::new("Hello, Script!");
let data_clone = data.clone();  // Increment reference count

// Reference count automatically decrements when values go out of scope
```

### Cycle Detection

The garbage collector automatically detects and breaks reference cycles:

```rust
// The GC runs automatically, but can be triggered manually
script::runtime::gc::collect_cycles();

// Check if collection is needed
if script::runtime::gc::should_collect() {
    script::runtime::gc::collect_cycles();
}
```

### Memory Limits

Configure heap limits to prevent excessive memory usage:

```rust
let config = RuntimeConfig {
    max_heap_size: 50 * 1024 * 1024,  // 50MB limit
    ..Default::default()
};

// Allocations will fail if heap limit is exceeded
```

## Panic Handling

### Automatic Stack Traces

The runtime automatically captures stack traces on panics:

```rust
// Panics are automatically captured with context
script::runtime::panic::script_panic("Something went wrong!");

// Get the last panic information
if let Some(panic_info) = script::runtime::panic::last_panic() {
    println!("Last panic: {}", panic_info.message);
    println!("Location: {:?}", panic_info.location);
    println!("Stack trace:\n{}", panic_info.backtrace);
}
```

### Custom Panic Hooks

Register custom panic handlers for application-specific behavior:

```rust
use script::runtime::panic;

panic::set_panic_hook(|panic_info| {
    // Log to your application's logging system
    log::error!("Script panic: {}", panic_info.message);
    
    // Send telemetry
    send_crash_report(panic_info);
    
    // Attempt recovery
    attempt_graceful_recovery();
});
```

### Panic History

The runtime maintains a history of recent panics for debugging:

```rust
// Get all recent panics
let history = script::runtime::panic::panic_history();
for panic_info in history {
    println!("Panic: {} at {:?}", panic_info.message, panic_info.location);
}

// Clear panic history
script::runtime::panic::clear_panic_history();
```

## Performance Monitoring

### Memory Profiling

Get detailed memory usage statistics:

```rust
use script::runtime::profiler;

// Get current profiling stats
if let Some(stats) = profiler::get_stats() {
    println!("Total allocations: {}", stats.allocations.total_allocations);
    println!("Current memory: {} bytes", stats.allocations.current_memory);
    println!("Peak memory: {} bytes", stats.allocations.peak_memory);
    println!("Potential leaks: {}", stats.allocations.potential_leaks);
    
    // Per-type statistics
    for (type_name, type_stats) in &stats.type_stats {
        println!("{}: {} allocations, {} bytes current", 
                type_name, type_stats.allocations, type_stats.current_bytes);
    }
}
```

### Runtime Statistics

Monitor overall runtime performance:

```rust
let runtime = script::runtime::runtime()?;
let stats = runtime.stats();

println!("Runtime uptime: {:?}", stats.uptime);
println!("Memory usage: {} bytes", stats.memory.heap_used);

if let Some(gc_stats) = stats.gc {
    println!("GC collections: {}", gc_stats.collections);
    println!("Objects collected: {}", gc_stats.objects_collected);
}
```

### Leak Detection

Check for potential memory leaks:

```rust
use script::runtime::profiler;

// Check if there are potential leaks
if profiler::check_leaks() {
    // Generate detailed leak report
    if let Some(report) = profiler::generate_report() {
        eprintln!("Memory leak detected!\n{}", report);
    }
}
```

## Embedding the Runtime

### Host Application Integration

Embed the Script runtime in your application:

```rust
use script::runtime::{Runtime, RuntimeConfig};

pub struct MyApplication {
    script_runtime: Runtime,
}

impl MyApplication {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Configure runtime for your application
        let config = RuntimeConfig {
            max_heap_size: 100 * 1024 * 1024,  // 100MB
            enable_profiling: cfg!(debug_assertions),
            enable_gc: true,
            gc_threshold: 1000,
            enable_panic_handler: true,
            stack_size: 2 * 1024 * 1024,
        };
        
        Runtime::initialize_with_config(config)?;
        let runtime = script::runtime::runtime()?;
        
        Ok(MyApplication {
            script_runtime: runtime,
        })
    }
    
    pub fn execute_script(&self, script: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.script_runtime.execute_protected(|| {
            // Parse and execute the script
            execute_script_code(script)
        })?;
        
        Ok(())
    }
}

impl Drop for MyApplication {
    fn drop(&mut self) {
        // Clean shutdown
        let _ = script::runtime::shutdown();
    }
}
```

### Custom Memory Allocator

Use the Script allocator in your application:

```rust
use script::runtime::ScriptAllocator;

// Set as global allocator (optional)
#[global_allocator]
static GLOBAL: ScriptAllocator = ScriptAllocator;
```

## Thread Safety

### Current Limitations

The current runtime is designed for single-threaded execution:
- `ScriptRc<T>` is not `Send` or `Sync` by default
- Runtime state is managed with thread-local storage concepts
- Memory management is optimized for single-threaded access

### Future Multi-threading Support

The runtime is designed with future actor-model support in mind:
- Internal structures use atomic operations where possible
- Architecture supports multiple isolated runtime instances
- Message-passing between Script actors will be supported

### Safe Multi-threading Patterns

For current multi-threaded applications:

```rust
use std::sync::mpsc;
use std::thread;

// Use message passing to communicate with Script runtime
let (tx, rx) = mpsc::channel();

// Script runtime runs in dedicated thread
let script_thread = thread::spawn(move || {
    script::runtime::initialize().unwrap();
    
    for message in rx {
        // Process messages in Script runtime
        handle_message(message);
    }
    
    script::runtime::shutdown().unwrap();
});

// Send messages from other threads
tx.send(ScriptMessage::Execute("some_script.script".to_string()))?;
```

## Best Practices

### Runtime Lifecycle

1. **Initialize Early**: Set up the runtime before any Script code execution
2. **Single Instance**: Use one runtime instance per process/thread
3. **Clean Shutdown**: Always call `shutdown()` before process termination
4. **Error Handling**: Handle initialization errors gracefully

### Memory Management

1. **Configure Limits**: Set appropriate heap limits for your application
2. **Monitor Usage**: Regularly check memory statistics in production
3. **Profile Allocations**: Use profiling to identify memory hotspots
4. **Avoid Cycles**: Design data structures to minimize reference cycles

### Performance Optimization

1. **GC Tuning**: Adjust GC threshold based on allocation patterns
2. **Batch Operations**: Group related operations to minimize overhead
3. **Pool Resources**: Reuse expensive objects when possible
4. **Monitor Metrics**: Track runtime statistics for performance insights

### Error Handling

1. **Use Protected Execution**: Wrap Script execution in panic protection
2. **Custom Panic Hooks**: Implement application-specific error handling
3. **Log Panics**: Always log panic information for debugging
4. **Graceful Degradation**: Design fallback behavior for Script failures

### Production Deployment

1. **Disable Debug Features**: Turn off profiling in production builds
2. **Set Resource Limits**: Configure appropriate memory and time limits
3. **Monitor Health**: Track runtime metrics and error rates
4. **Update Handling**: Design for runtime version upgrades

## Integration Examples

### Web Server Integration

```rust
use script::runtime::Runtime;
use actix_web::{web, App, HttpServer, Result};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize Script runtime
    Runtime::initialize_with_config(RuntimeConfig {
        max_heap_size: 512 * 1024 * 1024,  // 512MB for web server
        enable_profiling: false,            // Disabled in production
        ..Default::default()
    }).expect("Failed to initialize Script runtime");
    
    HttpServer::new(|| {
        App::new()
            .route("/execute", web::post().to(execute_script))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

async fn execute_script(script: String) -> Result<String> {
    let runtime = script::runtime::runtime()
        .map_err(|e| format!("Runtime error: {}", e))?;
    
    runtime.execute_protected(|| {
        // Execute the Script code
        run_script(&script)
    })
    .map_err(|e| format!("Execution error: {}", e))
}
```

### Game Engine Integration

```rust
use script::runtime::{Runtime, RuntimeConfig};

pub struct GameEngine {
    script_runtime: Runtime,
}

impl GameEngine {
    pub fn new() -> Self {
        let config = RuntimeConfig {
            max_heap_size: 1024 * 1024 * 1024,  // 1GB for games
            enable_profiling: cfg!(debug_assertions),
            gc_threshold: 100,                   // Frequent GC for games
            ..Default::default()
        };
        
        Runtime::initialize_with_config(config)
            .expect("Failed to initialize Script runtime");
        
        let runtime = script::runtime::runtime()
            .expect("Failed to get runtime instance");
        
        GameEngine {
            script_runtime: runtime,
        }
    }
    
    pub fn update(&mut self, delta_time: f32) {
        // Execute game scripts with panic protection
        let _ = self.script_runtime.execute_protected(|| {
            self.run_game_scripts(delta_time)
        });
    }
    
    fn run_game_scripts(&self, delta_time: f32) -> Result<(), Box<dyn std::error::Error>> {
        // Execute Script-based game logic
        Ok(())
    }
}
```

The Script runtime provides a robust foundation for executing Script code with automatic memory management, comprehensive error handling, and detailed performance monitoring. By following these guidelines and best practices, you can effectively integrate the Script runtime into your applications while maintaining reliability and performance.