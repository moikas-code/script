# Script Module System

The Script programming language provides a powerful module system for organizing and sharing code.

## Table of Contents

1. [Overview](#overview)
2. [Basic Usage](#basic-usage)
3. [Import Syntax](#import-syntax)
4. [Export Syntax](#export-syntax)
5. [Module Resolution](#module-resolution)
6. [Best Practices](#best-practices)
7. [Common Patterns](#common-patterns)
8. [Error Handling](#error-handling)

## Overview

The module system in Script allows you to:
- Organize code into separate files
- Share functionality between modules
- Avoid naming conflicts through namespacing
- Build reusable libraries
- Manage dependencies clearly

Each `.script` file is treated as a module, with explicit imports and exports controlling visibility.

## Basic Usage

### Creating a Module

```script
// math_utils.script
export { add, subtract, PI }

let PI = 3.14159

fn add(a: float, b: float) -> float {
    a + b
}

fn subtract(a: float, b: float) -> float {
    a - b
}

// This function is private to the module
fn internal_helper() {
    // Not exported, only available within this module
}
```

### Using a Module

```script
// main.script
import math_utils.{ add, PI }

fn main() {
    let result = add(10.0, PI)
    println("Result: " + result)
}
```

## Import Syntax

Script provides several ways to import modules:

### 1. Selective Import

Import specific items from a module:

```script
import math_utils.{ add, subtract, PI }
```

### 2. Import with Alias

Import specific items with renamed bindings:

```script
import math_utils.{ add as sum, subtract as diff }

let result = sum(10, 20)  // Uses 'sum' instead of 'add'
```

### 3. Import All

Import all exported items from a module:

```script
import string_utils.*

// All exported functions are now available
let result = concat("Hello, ", "World!")
```

### 4. Module Alias

Import a module with an alias for namespaced access:

```script
import geometry as Geo

let circle = Geo.Circle(0, 0, 5)
let area = Geo.calculateArea(circle)
```

### 5. Nested Module Paths

Import from nested module hierarchies:

```script
import std.collections.map as Map
import std.io.file as File
```

### 6. Relative Imports

Import modules relative to the current file:

```script
import "./sibling_module.script"
import "../parent/other_module.script"
```

## Export Syntax

Control what gets exposed from your module:

### Basic Export

```script
export { functionA, functionB, ConstantC, TypeD }
```

### Export Everything (Not Recommended)

```script
export *  // Exports all top-level items
```

### Re-exporting

```script
// common.script
import "./types.script" as Types
import "./utils.script" as Utils

export { Types, Utils }
```

## Module Resolution

Script follows these rules for finding modules:

### 1. Relative Paths

Paths starting with `./` or `../` are resolved relative to the importing file:

```script
import "./utils.script"      // Same directory
import "../common/types.script"  // Parent directory
```

### 2. Absolute Paths

Non-relative paths are resolved from module search paths:

```script
import math_utils         // Searches in module paths
import company.project.utils  // Hierarchical modules
```

### 3. Standard Library

Standard library modules use the `std` prefix:

```script
import std.io
import std.collections.vec
import std.net.http
```

### 4. Search Path Priority

1. Current project's `src/` directory
2. Project dependencies
3. Standard library location
4. System module paths

## Best Practices

### 1. One Module Per File

Each `.script` file should be a single, cohesive module:

```script
// Good: math_utils.script contains math-related functions
// Bad: utils.script with unrelated functions mixed together
```

### 2. Explicit Exports

Only export what's necessary:

```script
// Good: Export only the public API
export { calculate, Config }

// Internal implementation details remain private
fn internal_calculation() { ... }
```

### 3. Avoid Circular Dependencies

Structure modules hierarchically to prevent circular imports:

```
// Bad: A imports B, B imports A

// Good: Both A and B import common C
common/
  types.script
features/
  feature_a.script  // imports common.types
  feature_b.script  // imports common.types
```

### 4. Use Clear Module Names

Module names should reflect their purpose:

```script
// Good
import user_authentication
import database_connection
import http_client

// Bad
import utils
import helpers
import stuff
```

### 5. Document Module Interfaces

Add documentation comments to exported items:

```script
/**
 * Calculates the area of various geometric shapes
 * @param shape - A shape object (Circle, Rectangle, etc.)
 * @returns The area as a float
 */
export fn calculateArea(shape: Shape) -> float {
    // Implementation
}
```

## Common Patterns

### Factory Pattern

```script
// factory.script
import "./impl_a.script" as ImplA
import "./impl_b.script" as ImplB

export { create }

fn create(type: string) -> Implementation {
    match type {
        "A" => ImplA.new(),
        "B" => ImplB.new(),
        _ => panic("Unknown type: " + type)
    }
}
```

### Configuration Module

```script
// config.script
export { Config, load, save }

struct Config {
    debug: bool,
    port: int,
    host: string
}

fn load(path: string) -> Result<Config, string> {
    // Load configuration from file
}

fn save(config: Config, path: string) -> Result<(), string> {
    // Save configuration to file
}
```

### Module as Namespace

```script
// math/vector.script
export { Vec2, Vec3, dot, cross, normalize }

// Usage
import math.vector as Vector

let v1 = Vector.Vec3(1, 0, 0)
let v2 = Vector.Vec3(0, 1, 0)
let result = Vector.cross(v1, v2)
```

### Plugin System

```script
// plugin.script
export { Plugin, register, execute }

trait Plugin {
    fn name(self) -> string
    fn execute(self, input: any) -> any
}

let mut plugins = Map.new()

fn register(plugin: Plugin) {
    plugins.set(plugin.name(), plugin)
}

fn execute(name: string, input: any) -> Result<any, string> {
    match plugins.get(name) {
        Some(plugin) => Ok(plugin.execute(input)),
        None => Err("Plugin not found: " + name)
    }
}
```

## Error Handling

### Module Not Found

```script
// Error: Cannot find module 'non_existent'
import non_existent

// Solution: Check module path and ensure file exists
```

### Circular Dependency

```script
// Error: Circular dependency detected: a -> b -> c -> a

// Solution: Refactor to remove circular dependencies
```

### Export Not Found

```script
// Error: Module 'math_utils' does not export 'divide'
import math_utils.{ divide }

// Solution: Check available exports or add to module
```

### Access to Private Items

```script
// Error: 'internal_helper' is private and not exported
import utils.{ internal_helper }

// Solution: Only import exported items
```

## Advanced Topics

### Dynamic Imports (Future)

```script
// Potential future feature
let module_name = "math_utils"
let math = import(module_name)
```

### Module Initialization

```script
// module_with_init.script

// This code runs when module is first imported
let initialized = init()

fn init() -> bool {
    // Perform one-time setup
    print("Module initializing...")
    true
}

export { someFunction }
```

### Conditional Exports

```script
// Potential pattern for platform-specific exports
export {
    common_function,
    #[cfg(target_os = "windows")]
    windows_specific,
    #[cfg(target_os = "linux")]
    linux_specific
}
```

## Module System Implementation Status

- ✅ Basic import/export parsing
- ✅ Module path resolution
- ✅ Circular dependency detection
- 🚧 Module compilation pipeline
- 🚧 Module caching
- 📋 Dynamic imports
- 📋 Module hot reloading
- 📋 Package management integration