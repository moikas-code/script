# Embedding Script in Your Applications

This guide shows how to embed the Script programming language into your Rust applications, game engines, web frameworks, and other software.

## Table of Contents

- [Quick Start](#quick-start)
- [Basic Embedding](#basic-embedding)
- [Runtime Configuration](#runtime-configuration)
- [Error Handling](#error-handling)
- [Memory Management](#memory-management)
- [Advanced Integration](#advanced-integration)
- [Game Engine Integration](#game-engine-integration)
- [Web Framework Integration](#web-framework-integration)
- [Best Practices](#best-practices)

## Quick Start

Add Script to your `Cargo.toml`:

```toml
[dependencies]
script = "0.1.0"
```

Basic usage:

```rust
use script::{Runtime, RuntimeConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the Script runtime
    let config = RuntimeConfig::default();
    let mut runtime = Runtime::new(config)?;
    
    // Execute Script code
    let result = runtime.execute_string(r#"
        fn greet(name: string) -> string {
            return "Hello, " + name + "!"
        }
        
        greet("World")
    "#)?;
    
    println!("Result: {}", result);
    Ok(())
}
```

## Basic Embedding

### Creating a Runtime

The `Runtime` is the main interface for embedding Script:

```rust
use script::{Runtime, RuntimeConfig, Error};

// Default configuration
let runtime = Runtime::new(RuntimeConfig::default())?;

// Custom configuration
let config = RuntimeConfig {
    enable_gc: true,
    gc_threshold: 1024 * 1024, // 1MB
    enable_profiling: false,
    max_memory: Some(100 * 1024 * 1024), // 100MB limit
    stack_size: 8192,
    ..Default::default()
};
let runtime = Runtime::new(config)?;
```

### Executing Script Code

```rust
// Execute a string
let result = runtime.execute_string("2 + 3 * 4")?;
println!("Result: {}", result); // Result: 14

// Execute from file
let result = runtime.execute_file("script/math.script")?;

// Parse and compile without executing
let program = runtime.compile_string("fn add(a, b) { a + b }")?;
let result = runtime.execute_program(&program)?;
```

### Data Exchange

Convert between Rust and Script values:

```rust
use script::{Value, ValueType};

// Rust to Script
let rust_number = 42i32;
let script_value = Value::from(rust_number);

let rust_string = "Hello".to_string();
let script_value = Value::from(rust_string);

let rust_vec = vec![1, 2, 3];
let script_value = Value::from_array(rust_vec)?;

// Script to Rust
let script_result = runtime.execute_string("42")?;
let rust_number: i32 = script_result.try_into()?;

let script_result = runtime.execute_string("\"Hello\"")?;
let rust_string: String = script_result.try_into()?;
```

## Runtime Configuration

### Configuration Options

```rust
use script::RuntimeConfig;

let config = RuntimeConfig {
    // Memory management
    enable_gc: true,
    gc_threshold: 1024 * 1024,
    max_memory: Some(500 * 1024 * 1024), // 500MB
    
    // Performance
    enable_jit: true,
    optimization_level: 2,
    
    // Debugging
    enable_profiling: true,
    enable_debug_info: true,
    
    // Security
    sandbox_mode: false,
    allow_file_io: true,
    allow_network: true,
    
    // Threading
    stack_size: 8192,
    max_threads: 4,
    
    ..Default::default()
};
```

### Environment Setup

```rust
// Set up custom standard library paths
runtime.add_module_path("./my_scripts")?;
runtime.add_module_path("/usr/local/share/script")?;

// Configure global variables
runtime.set_global("APP_VERSION", Value::from("1.0.0"))?;
runtime.set_global("DEBUG", Value::from(cfg!(debug_assertions)))?;

// Register native functions
runtime.register_function("log", |args| {
    println!("Script log: {}", args[0]);
    Ok(Value::Null)
})?;
```

## Error Handling

### Error Types

```rust
use script::{Error, ErrorKind};

match runtime.execute_string("invalid syntax") {
    Ok(result) => println!("Success: {}", result),
    Err(error) => {
        match error.kind() {
            ErrorKind::ParseError => {
                eprintln!("Parse error: {}", error);
                if let Some(location) = error.location() {
                    eprintln!("At line {}, column {}", location.line, location.column);
                }
            }
            ErrorKind::RuntimeError => {
                eprintln!("Runtime error: {}", error);
                if let Some(trace) = error.stack_trace() {
                    eprintln!("Stack trace:\n{}", trace);
                }
            }
            ErrorKind::TypeError => {
                eprintln!("Type error: {}", error);
            }
            _ => {
                eprintln!("Other error: {}", error);
            }
        }
    }
}
```

### Error Recovery

```rust
// Enable error recovery mode
let config = RuntimeConfig {
    continue_on_error: true,
    max_errors: 10,
    ..Default::default()
};

let mut runtime = Runtime::new(config)?;

// Collect multiple errors
let result = runtime.execute_string_with_recovery(r#"
    let x = 1 + ; // Parse error
    let y = x / 0; // Runtime error
    print(z); // Undefined variable error
"#);

match result {
    Ok(value) => println!("Executed with recovery: {}", value),
    Err(errors) => {
        println!("Found {} errors:", errors.len());
        for error in errors {
            println!("- {}", error);
        }
    }
}
```

## Memory Management

### Reference Counting

Script uses automatic reference counting with cycle detection:

```rust
// Monitor memory usage
let stats = runtime.memory_stats()?;
println!("Allocated: {} bytes", stats.allocated);
println!("Peak usage: {} bytes", stats.peak);
println!("GC cycles: {}", stats.gc_cycles);

// Force garbage collection
runtime.collect_garbage()?;

// Set memory limits
runtime.set_memory_limit(50 * 1024 * 1024)?; // 50MB

// Enable memory profiling
if cfg!(debug_assertions) {
    runtime.enable_memory_profiling()?;
    
    // ... run script code ...
    
    let profile = runtime.memory_profile()?;
    println!("Memory profile:\n{}", profile);
}
```

### Custom Allocators

```rust
use script::{Allocator, TrackingAllocator};

// Use a tracking allocator for debugging
let allocator = TrackingAllocator::new();
let config = RuntimeConfig {
    allocator: Some(Box::new(allocator)),
    ..Default::default()
};

let runtime = Runtime::new(config)?;
```

## Advanced Integration

### Custom Types

Register Rust types with Script:

```rust
use script::{TypeRegistry, NativeType};

#[derive(Debug, Clone)]
struct Player {
    name: String,
    health: i32,
    position: (f32, f32),
}

impl NativeType for Player {
    fn type_name() -> &'static str { "Player" }
    
    fn register_methods(registry: &mut TypeRegistry) {
        registry.register_method("get_name", |player: &Player| {
            Ok(Value::from(player.name.clone()))
        });
        
        registry.register_method("set_health", |player: &mut Player, health: i32| {
            player.health = health;
            Ok(Value::Null)
        });
        
        registry.register_method("move_to", |player: &mut Player, x: f32, y: f32| {
            player.position = (x, y);
            Ok(Value::Null)
        });
    }
}

// Register the type
runtime.register_type::<Player>()?;

// Use in Script
let script = r#"
    let player = Player { name: "Alice", health: 100, position: (0.0, 0.0) }
    player.move_to(10.0, 5.0)
    player.set_health(90)
    print(player.get_name())
"#;

runtime.execute_string(script)?;
```

### Callbacks and Events

```rust
use std::sync::{Arc, Mutex};
use script::Callback;

// Event system
let events = Arc::new(Mutex::new(Vec::new()));
let events_clone = events.clone();

runtime.register_function("emit_event", move |args| {
    let event_name = args[0].as_string()?;
    let event_data = args[1].clone();
    
    events_clone.lock().unwrap().push((event_name, event_data));
    Ok(Value::Null)
})?;

// Execute script
runtime.execute_string(r#"
    emit_event("player_moved", { x: 10, y: 20 })
    emit_event("item_collected", { item: "sword", count: 1 })
"#)?;

// Process events
for (event, data) in events.lock().unwrap().iter() {
    println!("Event: {} with data: {}", event, data);
}
```

### Async Integration

```rust
use tokio;
use script::AsyncRuntime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = AsyncRuntime::new(RuntimeConfig::default()).await?;
    
    // Register async functions
    runtime.register_async_function("fetch_data", |url: String| async move {
        let response = reqwest::get(&url).await?;
        let data = response.text().await?;
        Ok(Value::from(data))
    }).await?;
    
    // Execute async script
    let result = runtime.execute_string(r#"
        let data = await fetch_data("https://api.example.com/data")
        print("Received: " + data.length + " characters")
    "#).await?;
    
    Ok(())
}
```

## Game Engine Integration

### Bevy Integration

```rust
use bevy::prelude::*;
use script::{Runtime, RuntimeConfig, Value};

#[derive(Resource)]
struct ScriptRuntime(Runtime);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_script_runtime)
        .add_systems(Update, run_game_scripts)
        .run();
}

fn setup_script_runtime(mut commands: Commands) {
    let mut config = RuntimeConfig::default();
    config.enable_jit = true;
    config.optimization_level = 2;
    
    let mut runtime = Runtime::new(config).unwrap();
    
    // Register Bevy-specific functions
    runtime.register_function("spawn_entity", |args| {
        // Implementation would interact with Bevy's World
        Ok(Value::Null)
    }).unwrap();
    
    runtime.register_function("get_component", |args| {
        // Implementation would query components
        Ok(Value::Null)
    }).unwrap();
    
    commands.insert_resource(ScriptRuntime(runtime));
}

fn run_game_scripts(mut script_runtime: ResMut<ScriptRuntime>) {
    // Execute frame update scripts
    let _ = script_runtime.0.execute_string(r#"
        // Game logic in Script
        let players = query_entities("Player")
        for player in players {
            let pos = get_component(player, "Transform")
            // Update player logic...
        }
    "#);
}
```

### Unity-like Component System

```rust
use script::{Runtime, Value, Component};

struct ScriptComponent {
    runtime: Runtime,
    script_path: String,
}

impl Component for ScriptComponent {
    fn new(script_path: &str) -> Self {
        let mut runtime = Runtime::new(RuntimeConfig::default()).unwrap();
        
        // Register component system functions
        runtime.register_function("get_transform", |_| {
            // Return transform data
            Ok(Value::from_object([
                ("x", Value::from(0.0f32)),
                ("y", Value::from(0.0f32)),
                ("z", Value::from(0.0f32)),
            ]))
        }).unwrap();
        
        ScriptComponent {
            runtime,
            script_path: script_path.to_string(),
        }
    }
    
    fn start(&mut self) -> Result<(), Error> {
        let script = std::fs::read_to_string(&self.script_path)?;
        self.runtime.execute_string(&script)?;
        Ok(())
    }
    
    fn update(&mut self, delta_time: f32) -> Result<(), Error> {
        self.runtime.set_global("delta_time", Value::from(delta_time))?;
        self.runtime.call_function("update", &[])?;
        Ok(())
    }
}
```

## Web Framework Integration

### Actix Web Integration

```rust
use actix_web::{web, App, HttpServer, HttpResponse, Result};
use script::{Runtime, RuntimeConfig, Value};
use std::sync::{Arc, Mutex};

type SharedRuntime = Arc<Mutex<Runtime>>;

async fn execute_script(
    runtime: web::Data<SharedRuntime>,
    script: String,
) -> Result<HttpResponse> {
    let mut rt = runtime.lock().unwrap();
    
    match rt.execute_string(&script) {
        Ok(result) => Ok(HttpResponse::Ok().json(result.to_json())),
        Err(err) => Ok(HttpResponse::BadRequest().json(err.to_string())),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let mut config = RuntimeConfig::default();
    config.sandbox_mode = true; // Enable sandboxing for web
    config.allow_file_io = false;
    config.max_memory = Some(10 * 1024 * 1024); // 10MB limit
    
    let mut runtime = Runtime::new(config).unwrap();
    
    // Register web-specific functions
    runtime.register_function("http_get", |args| {
        // HTTP client implementation
        Ok(Value::Null)
    }).unwrap();
    
    let shared_runtime = Arc::new(Mutex::new(runtime));
    
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(shared_runtime.clone()))
            .route("/execute", web::post().to(execute_script))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

### Warp Integration

```rust
use warp::Filter;
use script::{Runtime, RuntimeConfig};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    let runtime = Arc::new(Mutex::new(
        Runtime::new(RuntimeConfig::default()).unwrap()
    ));
    
    let script_route = warp::path("script")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_runtime(runtime))
        .and_then(handle_script);
    
    warp::serve(script_route)
        .run(([127, 0, 0, 1], 3030))
        .await;
}

fn with_runtime(
    runtime: Arc<Mutex<Runtime>>
) -> impl Filter<Extract = (Arc<Mutex<Runtime>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || runtime.clone())
}

async fn handle_script(
    body: serde_json::Value,
    runtime: Arc<Mutex<Runtime>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let script = body["script"].as_str().unwrap_or("");
    let mut rt = runtime.lock().await;
    
    match rt.execute_string(script) {
        Ok(result) => Ok(warp::reply::json(&result.to_json())),
        Err(err) => Ok(warp::reply::json(&serde_json::json!({
            "error": err.to_string()
        }))),
    }
}
```

## Best Practices

### Performance Optimization

1. **Compile Once, Execute Many**:
   ```rust
   // Good: Compile once
   let program = runtime.compile_string(script_source)?;
   for _ in 0..1000 {
       runtime.execute_program(&program)?;
   }
   
   // Bad: Compile every time
   for _ in 0..1000 {
       runtime.execute_string(script_source)?;
   }
   ```

2. **Reuse Runtime Instances**:
   ```rust
   // Good: One runtime per thread
   thread_local! {
       static RUNTIME: RefCell<Runtime> = RefCell::new(
           Runtime::new(RuntimeConfig::default()).unwrap()
       );
   }
   
   RUNTIME.with(|rt| {
       rt.borrow_mut().execute_string(script)
   })
   ```

3. **Enable JIT for Long-Running Scripts**:
   ```rust
   let config = RuntimeConfig {
       enable_jit: true,
       optimization_level: 2,
       ..Default::default()
   };
   ```

### Security Considerations

1. **Use Sandbox Mode for Untrusted Code**:
   ```rust
   let config = RuntimeConfig {
       sandbox_mode: true,
       allow_file_io: false,
       allow_network: false,
       max_memory: Some(10 * 1024 * 1024),
       execution_timeout: Some(Duration::from_secs(5)),
       ..Default::default()
   };
   ```

2. **Validate Inputs**:
   ```rust
   runtime.register_function("safe_divide", |args| {
       let a = args[0].as_number()?;
       let b = args[1].as_number()?;
       
       if b == 0.0 {
           return Err(Error::runtime_error("Division by zero"));
       }
       
       Ok(Value::from(a / b))
   })?;
   ```

3. **Limit Resource Usage**:
   ```rust
   // Set memory limits
   runtime.set_memory_limit(50 * 1024 * 1024)?;
   
   // Set execution timeout
   runtime.set_timeout(Duration::from_secs(10))?;
   
   // Monitor resource usage
   let stats = runtime.resource_stats()?;
   if stats.cpu_time > Duration::from_secs(5) {
       runtime.interrupt()?;
   }
   ```

### Error Handling Patterns

1. **Use Result Types Consistently**:
   ```rust
   fn execute_user_script(script: &str) -> Result<Value, ScriptError> {
       let runtime = create_sandboxed_runtime()?;
       let result = runtime.execute_string(script)?;
       Ok(result)
   }
   ```

2. **Provide Context in Errors**:
   ```rust
   runtime.register_function("load_config", |args| {
       let path = args[0].as_string()?;
       std::fs::read_to_string(&path)
           .map(Value::from)
           .map_err(|e| Error::runtime_error(&format!(
               "Failed to load config from '{}': {}", path, e
           )))
   })?;
   ```

3. **Handle Panics Gracefully**:
   ```rust
   use std::panic;
   
   let result = panic::catch_unwind(|| {
       runtime.execute_string(untrusted_script)
   });
   
   match result {
       Ok(Ok(value)) => println!("Success: {}", value),
       Ok(Err(error)) => eprintln!("Script error: {}", error),
       Err(_) => eprintln!("Script caused a panic"),
   }
   ```

### Thread Safety

Script runtime instances are not thread-safe by default, but you can share them safely:

```rust
use std::sync::{Arc, Mutex};
use std::thread;

// Option 1: Shared runtime with mutex
let runtime = Arc::new(Mutex::new(Runtime::new(config)?));

let handles: Vec<_> = (0..4).map(|i| {
    let runtime = runtime.clone();
    thread::spawn(move || {
        let script = format!("print(\"Thread {}\")", i);
        runtime.lock().unwrap().execute_string(&script).unwrap();
    })
}).collect();

for handle in handles {
    handle.join().unwrap();
}

// Option 2: One runtime per thread
fn worker_thread(script: String) {
    let runtime = Runtime::new(RuntimeConfig::default()).unwrap();
    runtime.execute_string(&script).unwrap();
}
```

This comprehensive embedding guide provides everything needed to integrate Script into your applications. For more specific use cases or advanced topics, consult the other integration guides or the API documentation.