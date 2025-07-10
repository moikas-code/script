# Actor Type System Specification

> **Implementation Status**: 🔄 In development. Basic async/await implemented, actor system design complete but not yet implemented.

This document specifies the design of the Actor type system for the Script programming language, building upon Script's existing async/await infrastructure and beginner-friendly philosophy.

## Table of Contents

1. [Overview](#overview)
2. [Actor Types](#actor-types)
3. [Message Types](#message-types)
4. [Actor Interfaces](#actor-interfaces)
5. [Actor Lifecycle](#actor-lifecycle)
6. [Type Safety](#type-safety)
7. [Integration with Existing Features](#integration-with-existing-features)
8. [Syntax and Semantics](#syntax-and-semantics)
9. [Error Handling](#error-handling)
10. [Performance and Scaling](#performance-and-scaling)
11. [Implementation Roadmap](#implementation-roadmap)

## Overview

The Actor model provides a foundation for building concurrent, fault-tolerant systems by encapsulating state and computation within independent actors that communicate through message passing. Script's actor system is designed to be:

- **Type-safe**: Compile-time guarantees for message protocols
- **Beginner-friendly**: Simple syntax with gradual complexity
- **Async-first**: Built on Script's existing Future/async infrastructure
- **Memory-safe**: No shared mutable state between actors
- **Fault-tolerant**: Supervisor hierarchies for error handling

### Key Principles

1. **No Shared State**: Actors communicate only through messages
2. **Location Transparency**: Actors can be local or remote
3. **Supervision Trees**: Hierarchical error handling and recovery
4. **Type-Safe Protocols**: Message interfaces defined at compile time
5. **Gradual Typing**: Support both typed and dynamic message handling

## Actor Types

### Basic Actor Type

```rust
// In the type system (src/types/mod.rs)
Type::Actor {
    interface: Box<Type>,  // The message interface this actor implements
    state: Box<Type>,      // The internal state type (optional for gradual typing)
}
```

### Actor Declaration Syntax

```script
// Basic actor with explicit message interface
actor Counter implements CounterMessages {
    private count: i32 = 0
    
    // Message handlers
    handle increment() {
        count += 1
    }
    
    handle get_count() -> i32 {
        count
    }
    
    handle add(amount: i32) {
        count += amount
    }
}

// Actor with gradual typing (interface inferred)
actor Logger {
    handle log(message: string) {
        println("LOG: {}", message)
    }
    
    handle error(error: string) {
        println("ERROR: {}", error)
    }
}

// Generic actor
actor<T> Buffer {
    private items: [T] = []
    
    handle push(item: T) {
        items.push(item)
    }
    
    handle pop() -> Option<T> {
        items.pop()
    }
}
```

## Message Types

### Message Interface Definition

```script
// Message interface (similar to traits)
interface CounterMessages {
    increment() -> ()
    get_count() -> i32
    add(amount: i32) -> ()
    reset() -> ()
}

// Generic message interface
interface<T> BufferMessages {
    push(item: T) -> ()
    pop() -> Option<T>
    clear() -> ()
    len() -> i32
}

// Message with async response
interface AsyncFileReader {
    read_file(path: string) -> Future<Result<string, IOError>>
    write_file(path: string, content: string) -> Future<Result<(), IOError>>
}
```

### Message Types in the Type System

```rust
// Message type representation
Type::Message {
    name: String,
    params: Vec<Type>,
    return_type: Box<Type>,
    is_async: bool,
}

// Message interface type
Type::MessageInterface {
    name: String,
    messages: Vec<Type>, // Vec of Message types
    generic_params: Vec<String>,
}
```

## Actor Interfaces

### Interface Implementation

```script
// Explicit interface implementation
actor FileManager implements AsyncFileReader {
    handle read_file(path: string) -> Future<Result<string, IOError>> {
        async {
            // File reading logic
            match fs::read_to_string(path).await {
                Ok(content) => Ok(content),
                Err(e) => Err(IOError::from(e))
            }
        }
    }
    
    handle write_file(path: string, content: string) -> Future<Result<(), IOError>> {
        async {
            fs::write(path, content).await
        }
    }
}

// Multiple interface implementation
actor DatabaseActor implements ReaderInterface + WriterInterface + AdminInterface {
    // Handler implementations...
}
```

### Interface Inheritance

```script
// Base interface
interface BaseActor {
    ping() -> string
    shutdown() -> ()
}

// Extended interface
interface WorkerActor extends BaseActor {
    process(data: string) -> Result<string, ProcessingError>
    get_status() -> WorkerStatus
}
```

## Actor Lifecycle

### Actor Creation and Spawning

```script
// Spawn actor with explicit type
let counter: Actor<CounterMessages> = spawn Counter::new()

// Spawn with supervisor
let supervised_counter = supervisor.spawn(Counter::new())

// Spawn with configuration
let configured_actor = spawn_with_config(
    Logger::new(),
    ActorConfig {
        mailbox_size: 1000,
        supervisor: Some(supervisor_ref),
        restart_policy: RestartPolicy::Always
    }
)
```

### Actor References and Addressing

```script
// Actor reference type
type ActorRef<T> = {
    address: ActorAddress,
    interface: T
}

// Sending messages
let counter_ref: ActorRef<CounterMessages> = spawn Counter::new()
counter_ref.increment()  // Fire-and-forget
let count = counter_ref.get_count().await  // Request-response
```

### Actor Supervision

```script
// Supervisor actor
actor Supervisor {
    private children: [ActorRef<unknown>] = []
    
    handle spawn_child<T>(actor_factory: () -> Actor<T>) -> ActorRef<T> {
        let child = spawn actor_factory()
        children.push(child)
        child
    }
    
    handle child_failed(child: ActorRef<unknown>, error: ActorError) {
        match restart_policy {
            RestartPolicy::Always => restart_child(child),
            RestartPolicy::Never => remove_child(child),
            RestartPolicy::OnError(error_type) => {
                if error.is_type(error_type) {
                    restart_child(child)
                } else {
                    escalate_error(error)
                }
            }
        }
    }
}
```

## Type Safety

### Compile-Time Message Validation

```script
// Type-safe message sending
let counter: ActorRef<CounterMessages> = mkspan Counter::new()

// This compiles - increment is in CounterMessages interface
counter.increment()

// This would be a compile error - invalid_method not in interface
// counter.invalid_method()  // ERROR: Method not found in CounterMessages

// Type-safe responses
let count: Future<i32> = counter.get_count()  // Return type guaranteed
```

### Interface Compatibility

```script
// Interface subtyping
interface BasicCounter {
    increment() -> ()
    get_count() -> i32
}

interface AdvancedCounter extends BasicCounter {
    decrement() -> ()
    add(amount: i32) -> ()
}

// Actor implementing AdvancedCounter can be used as BasicCounter
let advanced: ActorRef<AdvancedCounter> = spawn AdvancedCounterActor::new()
let basic: ActorRef<BasicCounter> = advanced  // Valid upcast
```

### Gradual Typing Support

```script
// Gradually typed actor (no explicit interface)
actor FlexibleActor {
    handle process(data: unknown) -> unknown {
        // Dynamic message handling
        match data {
            string(s) => "Processed string: " + s,
            i32(n) => "Processed number: " + n.to_string(),
            _ => "Unknown data type"
        }
    }
}

// Can be used with any message type
let flexible: ActorRef<unknown> = spawn FlexibleActor::new()
flexible.process("hello")
flexible.process(42)
```

## Integration with Existing Features

### Async/Await Integration

```script
actor AsyncWorker {
    handle async_operation(data: string) -> Future<Result<string, ProcessingError>> {
        async {
            // Use existing async features
            let processed = await external_api_call(data)?
            let validated = await validate_result(processed)?
            Ok(validated)
        }
    }
    
    handle batch_process(items: [string]) -> Future<[Result<string, ProcessingError>]> {
        async {
            // Parallel processing with existing async infrastructure
            let futures = items.map(|item| self.async_operation(item))
            await join_all(futures)
        }
    }
}
```

### Pattern Matching for Messages

```script
actor MessageRouter {
    handle route(message: Message) -> () {
        match message {
            Message::User { id, action } => {
                user_service.handle_action(id, action)
            },
            Message::System { level, content } => {
                logger.log(level, content)
            },
            Message::Error { error, context } => {
                error_handler.handle_error(error, context)
            },
            _ => {
                log("Unknown message type")
            }
        }
    }
}
```

### Type Inference Integration

```script
// Type inference works with actor messages
actor InferredActor {
    handle process(data: unknown) {
        // Type is inferred from usage
        let result = data + 1  // Infers data should be numeric
        result
    }
}

// The interface is automatically inferred as:
// interface InferredActorMessages {
//     process(data: unknown) -> unknown
// }
```

## Syntax and Semantics

### Actor Definition Grammar

```bnf
actor_def ::= "actor" type_params? IDENTIFIER interface_impl? "{" actor_body "}"
interface_impl ::= "implements" interface_list
interface_list ::= interface_type ("+" interface_type)*
actor_body ::= (field_def | handler_def)*

field_def ::= visibility? IDENTIFIER ":" type_ann ("=" expr)?
handler_def ::= "handle" IDENTIFIER "(" params? ")" ("->" type_ann)? block
visibility ::= "private" | "public"

interface_def ::= "interface" type_params? IDENTIFIER extends_clause? "{" message_sig* "}"
extends_clause ::= "extends" interface_type
message_sig ::= IDENTIFIER "(" params? ")" ("->" type_ann)?
```

### Message Sending Semantics

```script
// Three types of message sending

// 1. Fire-and-forget (async, no response expected)
actor_ref.method_name(args)

// 2. Request-response (async, returns Future)
let result: Future<ReturnType> = actor_ref.method_name(args)
let value = await result

// 3. Synchronous call (blocks until response, discouraged)
let value = actor_ref.method_name(args).wait()
```

### Actor Addressing

```script
// Local actor reference
let local_actor: ActorRef<MyInterface> = spawn MyActor::new()

// Remote actor reference (future extension)
let remote_actor: ActorRef<MyInterface> = connect("actor://remote-host:8080/my-actor")

// Actor addressing is location-transparent
fn send_message(actor: ActorRef<MyInterface>) {
    actor.some_method()  // Works for both local and remote
}
```

## Error Handling

### Actor Error Types

```rust
#[derive(Debug, Clone)]
pub enum ActorError {
    /// Actor panicked during message processing
    Panic { message: String, stack_trace: String },
    /// Actor mailbox is full
    MailboxFull,
    /// Actor has been terminated
    ActorTerminated,
    /// Message timeout
    Timeout { duration: Duration },
    /// Network error for remote actors
    NetworkError { error: NetworkError },
    /// Supervision error
    SupervisionError { error: String },
    /// User-defined error from actor handler
    HandlerError { error: Box<dyn Error> },
}
```

### Error Handling Strategies

```script
// Error handling in actor handlers
actor RobustActor {
    handle risky_operation(data: string) -> Result<string, ProcessingError> {
        try {
            let result = dangerous_computation(data)
            Ok(result)
        } catch ProcessingError as e {
            // Handle specific error
            Err(e)
        } catch panic {
            // Handle panics gracefully
            Err(ProcessingError::InternalError("Operation failed"))
        }
    }
    
    // Error recovery
    handle recover_from_error(error: ActorError) {
        match error {
            ActorError::Timeout { .. } => {
                // Retry logic
                retry_last_operation()
            },
            ActorError::HandlerError { .. } => {
                // Reset state
                reset_to_safe_state()
            },
            _ => {
                // Escalate to supervisor
                escalate_error(error)
            }
        }
    }
}

// Supervisor error handling
actor Supervisor {
    handle child_error(child: ActorRef<unknown>, error: ActorError) {
        match error {
            ActorError::Panic { .. } => {
                log("Child panicked, restarting...")
                restart_child(child)
            },
            ActorError::HandlerError { error } if error.is_recoverable() => {
                log("Recoverable error, sending recovery message")
                child.recover_from_error(error)
            },
            _ => {
                log("Unrecoverable error, terminating child")
                terminate_child(child)
            }
        }
    }
}
```

### Timeout Handling

```script
actor TimeoutActor {
    handle slow_operation(data: string) -> Future<Result<string, TimeoutError>> {
        // Automatic timeout handling
        timeout(Duration::from_secs(5), async {
            let result = await very_slow_computation(data)
            Ok(result)
        })
    }
}

// Usage with timeout
let result = match await timeout_actor.slow_operation("data") {
    Ok(value) => value,
    Err(TimeoutError) => "Operation timed out"
}
```

## Performance and Scaling

### Mailbox Management

```script
// Configurable mailbox types
enum MailboxType {
    Unbounded,              // Unlimited capacity (default)
    Bounded { capacity: i32 },  // Fixed capacity with backpressure
    Priority,               // Priority-based ordering
    Custom { handler: MailboxHandler }  // User-defined mailbox
}

// Actor configuration
struct ActorConfig {
    mailbox_type: MailboxType,
    max_message_size: Option<i32>,
    message_timeout: Option<Duration>,
    restart_policy: RestartPolicy,
}
```

### Work Stealing Integration

```script
// Actors automatically distributed across worker threads
// using existing work-stealing scheduler

// CPU-intensive actor
actor CPUIntensiveActor {
    handle compute(data: LargeDataSet) -> ComputeResult {
        // This will be scheduled on available worker threads
        expensive_computation(data)
    }
}

// I/O actor (uses async runtime)
actor IOActor {
    handle async read_file(path: string) -> Future<Result<string, IOError>> {
        // Non-blocking I/O using existing async infrastructure
        fs::read_to_string(path).await
    }
}
```

### Actor Pool Pattern

```script
// Pool of identical actors for scaling
struct ActorPool<T> {
    actors: [ActorRef<T>],
    current_index: i32,
}

impl<T> ActorPool<T> {
    fn new(size: i32, factory: () -> Actor<T>) -> ActorPool<T> {
        let actors = (0..size).map(|_| spawn factory()).collect()
        ActorPool { actors, current_index: 0 }
    }
    
    fn get_next_actor() -> ActorRef<T> {
        let actor = actors[current_index]
        current_index = (current_index + 1) % actors.len()
        actor
    }
}

// Usage
let worker_pool = ActorPool::new(4, || WorkerActor::new())
worker_pool.get_next_actor().process_task(task)
```

### Memory Management

```script
// Actors use Script's existing memory management
// - No shared mutable state between actors
// - Reference counting for actor references
// - Garbage collection for actor-local state

actor MemoryEfficientActor {
    private large_data: Option<LargeStruct> = None
    
    handle load_data(source: string) {
        // Load large data structure
        large_data = Some(load_from_source(source))
    }
    
    handle clear_data() {
        // Explicit cleanup
        large_data = None
        // Data is automatically collected when no longer referenced
    }
}
```

## Implementation Roadmap

### Phase 1: Core Actor Infrastructure (Months 1-2)

1. **Type System Extensions**
   - Add `Type::Actor` and `Type::MessageInterface` to type system
   - Implement actor reference types
   - Add message type validation

2. **AST Extensions**
   - Add `ActorDef`, `InterfaceDef`, and `HandleDef` AST nodes
   - Extend parser to handle actor syntax
   - Add semantic analysis for actor definitions

3. **Basic Actor Runtime**
   - Actor spawning and lifecycle management
   - Message queue implementation using existing async infrastructure
   - Integration with existing work-stealing scheduler

### Phase 2: Message Passing and Interfaces (Months 2-3)

1. **Message Interface System**
   - Interface definition and implementation checking
   - Message type safety validation
   - Interface inheritance and composition

2. **Message Sending Infrastructure**
   - Type-safe message dispatch
   - Request-response patterns using Futures
   - Integration with existing async/await

3. **Actor Reference Management**
   - Local actor references
   - Reference counting and cleanup
   - Actor addressing system

### Phase 3: Supervision and Error Handling (Months 3-4)

1. **Supervision Trees**
   - Supervisor actor implementation
   - Child actor management
   - Restart policies and strategies

2. **Error Handling Integration**
   - Actor error types and propagation
   - Error recovery mechanisms
   - Integration with existing error handling

3. **Fault Tolerance**
   - Actor isolation and failure containment
   - Supervisor escalation chains
   - System-wide error monitoring

### Phase 4: Performance and Advanced Features (Months 4-5)

1. **Performance Optimizations**
   - Mailbox optimizations and configurations
   - Message batching and prioritization
   - Actor pool patterns

2. **Advanced Patterns**
   - Publish-subscribe messaging
   - Router and balancer actors
   - State machine actors

3. **Integration Improvements**
   - Better async/await integration
   - Enhanced pattern matching for messages
   - Type inference improvements

### Phase 5: Remote Actors and Distribution (Months 5-6)

1. **Remote Actor Foundation**
   - Network protocol for actor communication
   - Serialization for messages
   - Location transparency

2. **Distributed System Features**
   - Cluster membership and discovery
   - Load balancing across nodes
   - Fault tolerance in distributed settings

3. **Development Tools**
   - Actor debugging and introspection
   - Performance monitoring
   - Visual supervision tree tools

### Integration Points with Existing Code

#### Type System Integration (`src/types/mod.rs`)
- Add actor and message interface types
- Extend type checking for actor operations
- Add subtyping rules for interfaces

#### AST Integration (`src/parser/ast.rs`)
- Add actor definition nodes
- Extend statement and expression types
- Add message handler syntax

#### Runtime Integration (`src/runtime/`)
- Use existing async runtime and scheduler
- Extend with actor-specific message handling
- Integrate with existing memory management

#### Error System Integration (`src/error/mod.rs`)
- Add actor-specific error types
- Integrate with existing error reporting
- Add supervision-specific error handling

This specification provides a comprehensive foundation for implementing actors in Script while maintaining consistency with the language's existing design principles and infrastructure.