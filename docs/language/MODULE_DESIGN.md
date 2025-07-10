# Script Module System Design

This document outlines the design principles and patterns for the Script module system.

## Design Goals

1. **Simplicity**: Easy to understand and use
2. **Explicitness**: Clear dependencies and exports
3. **Safety**: Prevent circular dependencies and namespace conflicts
4. **Performance**: Efficient module loading and caching
5. **Flexibility**: Support various project structures

## Module Concepts

### What is a Module?

- Each `.script` file is a module
- Modules have their own namespace
- Modules explicitly export what they want to share
- Modules explicitly import what they need

### Module Resolution

Modules can be referenced in three ways:

1. **Relative paths**: Start with `./` or `../`
   ```script
   import "./utils.script"
   import "../common/types.script"
   ```

2. **Package paths**: Resolved from project root
   ```script
   import math_utils
   import company.project.module
   ```

3. **Standard library**: Prefixed with `std`
   ```script
   import std.io
   import std.collections.vec
   ```

## Import Patterns

### Selective Import
Import specific items from a module:
```script
import math.{ sin, cos, PI }
```

### Import with Alias
Rename imports to avoid conflicts:
```script
import math.{ sqrt as square_root }
import graphics.Point as GPoint
```

### Namespace Import
Import all exports under a namespace:
```script
import math as M
let result = M.sin(M.PI / 2)
```

### Wildcard Import
Import all exports directly (use sparingly):
```script
import math.*
let result = sin(PI / 2)
```

## Export Patterns

### Named Exports
Export specific items:
```script
export { add, subtract, multiply, divide }

fn add(a: float, b: float) -> float { a + b }
fn subtract(a: float, b: float) -> float { a - b }
fn multiply(a: float, b: float) -> float { a * b }
fn divide(a: float, b: float) -> float { a / b }
```

### Export Declarations
Export as you declare:
```script
export fn calculate(x: float) -> float {
    x * x + 2 * x + 1
}

export let VERSION = "1.0.0"

export struct Config {
    debug: bool,
    timeout: int
}
```

### Re-exports
Export items from other modules:
```script
import "./types.script" as types
import "./utils.script" as utils

export { types.Point, utils.distance }
```

## Module Organization Patterns

### Flat Structure
Good for small projects:
```
project/
├── main.script
├── utils.script
├── types.script
└── config.script
```

### Hierarchical Structure
Better for larger projects:
```
project/
├── main.script
├── core/
│   ├── engine.script
│   └── types.script
├── utils/
│   ├── math.script
│   └── string.script
└── features/
    ├── auth.script
    └── database.script
```

### Index Pattern
Use `index.script` for cleaner imports:
```
math/
├── index.script    // Re-exports from other files
├── basic.script
├── advanced.script
└── constants.script
```

## Best Practices

### 1. One Module, One Purpose
Each module should have a single, clear responsibility.

### 2. Minimal Public API
Only export what consumers actually need.

### 3. Avoid Circular Dependencies
Structure modules hierarchically:
- Core modules at the bottom (no dependencies)
- Feature modules in the middle
- Application modules at the top

### 4. Consistent Naming
- Use lowercase with underscores for module files
- Use PascalCase for types and constructors
- Use camelCase for functions and variables

### 5. Documentation
Document module purpose and exports:
```script
/**
 * Math utilities module
 * Provides basic mathematical operations and constants
 */

export { add, multiply, PI, E }
```

## Common Patterns

### Factory Module
```script
// factory.script
import "./car.script" as Car
import "./truck.script" as Truck

export fn createVehicle(type: string) -> Vehicle {
    match type {
        "car" => Car.new(),
        "truck" => Truck.new(),
        _ => panic("Unknown vehicle type")
    }
}
```

### Configuration Module
```script
// config.script
export struct Config {
    host: string,
    port: int,
    debug: bool
}

export fn load() -> Config {
    // Load from environment or file
}

export fn validate(config: Config) -> Result<(), string> {
    // Validate configuration
}
```

### Service Module
```script
// auth_service.script
import "./user.script" as User
import "./token.script" as Token

let mut users = Map.new()

export fn register(username: string, password: string) -> Result<User, string> {
    // Implementation
}

export fn login(username: string, password: string) -> Result<Token, string> {
    // Implementation
}

export fn logout(token: Token) {
    // Implementation
}
```

### Plugin System
```script
// plugin.script
export trait Plugin {
    fn name(self) -> string
    fn version(self) -> string
    fn execute(self, context: Context) -> Result<(), Error>
}

let mut registry = Map.new()

export fn register(plugin: Plugin) {
    registry.set(plugin.name(), plugin)
}

export fn run(name: string, context: Context) -> Result<(), Error> {
    match registry.get(name) {
        Some(plugin) => plugin.execute(context),
        None => Err(Error.new("Plugin not found"))
    }
}
```

## Module Initialization

Modules can have initialization code:
```script
// database.script
import std.env

// This runs when the module is first imported
let connection = connect(env.get("DATABASE_URL"))

export fn query(sql: string) -> Result<Rows, Error> {
    connection.execute(sql)
}
```

## Testing Modules

Modules can include tests:
```script
// math_utils.script
export fn add(a: float, b: float) -> float {
    a + b
}

#[test]
fn test_add() {
    assert_eq(add(2, 3), 5)
    assert_eq(add(-1, 1), 0)
}
```

## Future Considerations

### Dynamic Imports
```script
let module_name = "plugin_" + version
let plugin = await import(module_name)
```

### Conditional Exports
```script
#[cfg(debug)]
export fn debug_info() -> string {
    // Only available in debug builds
}
```

### Module Metadata
```script
#[module(
    name = "my_module",
    version = "1.0.0",
    author = "Script Team"
)]
```

## Implementation Phases

1. **Phase 1**: Basic import/export parsing ✅
2. **Phase 2**: Module resolution and loading
3. **Phase 3**: Circular dependency detection
4. **Phase 4**: Module caching
5. **Phase 5**: Standard library modules
6. **Phase 6**: Package management integration
7. **Phase 7**: Advanced features (dynamic imports, etc.)