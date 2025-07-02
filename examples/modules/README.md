# Script Module System Examples

This directory contains examples demonstrating the Script language module system.

## Examples

### Calculator
A multi-module calculator application that demonstrates:
- Module imports and exports
- Module aliasing
- Organizing code into logical modules
- Separation of concerns (lexer, parser, operations, display)

To run:
```bash
script examples/modules/calculator/main.script
```

### Game Engine
A simple game engine structure showing:
- Hierarchical module organization
- Relative imports
- Module dependencies
- Entity-Component-System architecture

To run:
```bash
script examples/modules/game_engine/main.script
```

## Module System Features

### Import Syntax

```script
// Import specific items
import math_utils.{ add, subtract, PI }

// Import all exported items
import math_utils.*

// Import with alias
import "./geometry.script" as Geo

// Import from nested modules
import std.collections.map as Map
```

### Export Syntax

```script
// Export specific items
export { add, subtract, multiply, divide }

// Items not exported are private to the module
fn privateHelper() {
    // Not accessible from outside
}
```

### Module Resolution

1. **Relative paths** start with `./` or `../`
   ```script
   import "./utils/math.script"
   import "../common/types.script"
   ```

2. **Absolute paths** resolve from module search paths
   ```script
   import math_utils
   import std.collections
   ```

### Best Practices

1. **One module per file** - Each `.script` file is a module
2. **Explicit exports** - Only export what's needed
3. **Avoid circular dependencies** - Structure modules hierarchically
4. **Use meaningful names** - Module names should reflect their purpose
5. **Group related functionality** - Keep related code together

### Module Organization Patterns

#### Flat Structure (Calculator)
```
calculator/
├── main.script
├── operations.script
├── parser.script
├── lexer.script
├── display.script
└── types.script
```

#### Hierarchical Structure (Game Engine)
```
game_engine/
├── main.script
├── core/
│   ├── engine.script
│   └── system.script
├── entities/
│   ├── entity.script
│   ├── player.script
│   └── enemy.script
├── systems/
│   ├── physics.script
│   └── renderer.script
└── utils/
    ├── vector.script
    └── math.script
```

### Common Patterns

#### Re-exporting
```script
// common.script
import "./types.script" as Types
import "./utils.script" as Utils

export { Types, Utils }
```

#### Factory Pattern
```script
// factory.script
import "./impl_a.script" as ImplA
import "./impl_b.script" as ImplB

export { create }

fn create(type: string) -> Implementation {
    match type {
        "A" => ImplA.new(),
        "B" => ImplB.new(),
        _ => panic("Unknown type")
    }
}
```

#### Module as Namespace
```script
// math.script
export { Vector, Matrix, Quaternion, lerp, clamp }

// Usage
import math

let v = math.Vector.new(1, 2, 3)
let clamped = math.clamp(value, 0, 1)
```