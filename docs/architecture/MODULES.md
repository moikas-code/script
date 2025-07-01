# Module System

## Table of Contents

- [Overview](#overview)
- [Module Design Philosophy](#module-design-philosophy)
- [Module Declaration and Structure](#module-declaration-and-structure)
- [Import and Export System](#import-and-export-system)
- [Module Resolution](#module-resolution)
- [Package Management](#package-management)
- [Compilation and Linking](#compilation-and-linking)
- [Module Privacy and Visibility](#module-privacy-and-visibility)
- [Circular Dependencies](#circular-dependencies)
- [Standard Library Integration](#standard-library-integration)
- [Implementation Plan](#implementation-plan)

## Overview

The Script module system is designed to provide clean, explicit, and efficient organization of code into reusable components. It draws inspiration from modern module systems while being tailored for Script's specific needs including gradual typing, game development, and Actor model support.

**Status**: 🚧 **PLANNED** - Module system is part of Phase 6 development

```
┌─────────────────────────────────────────────────────────────┐
│                    Module System Architecture               │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │
│  │   Module    │  │   Import    │  │  Package    │  │   Module    │ │
│  │ Declaration │  │ Resolution  │  │ Management  │  │   Cache     │ │
│  │             │  │             │  │             │  │             │ │
│  │ • pub/priv  │  │ • Path      │  │ • manifest  │  │ • Compiled  │ │
│  │ • exports   │  │ • Scope     │  │ • versions  │  │   modules   │ │
│  │ • imports   │  │ • Aliases   │  │ • deps      │  │ • Symbol    │ │
│  │ • re-export │  │ • Wildcards │  │ • registry  │  │   tables    │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Module Design Philosophy

### Core Principles

1. **Explicit is Better**: All imports and exports must be explicitly declared
2. **Static Resolution**: Module dependencies are resolved at compile time
3. **Zero-Cost Abstraction**: No runtime overhead for module boundaries
4. **Gradual Adoption**: Modules are optional - single files work without module declarations
5. **Game Development Focus**: Optimized for asset loading and game module patterns
6. **Actor Model Ready**: Designed to support future distributed computing features

### Design Goals

- **Performance**: Fast compilation and resolution
- **Simplicity**: Easy to understand and use
- **Flexibility**: Support various code organization patterns
- **Tooling**: IDE support for auto-completion and refactoring
- **Compatibility**: Work with existing Script code

## Module Declaration and Structure

### Module Declaration Syntax

Modules are declared using the `module` keyword at the beginning of a file:

```rust
// File: math/vector.script
module math.vector

// Module-level documentation
/// 3D vector operations for game development
///
/// This module provides efficient vector operations
/// with SIMD optimizations where available.

import std.math
import std.assert

// Private helper function
fn dot_product_impl(a: Vec3, b: Vec3) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

// Public type definition
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

// Public function
pub fn dot(a: Vec3, b: Vec3) -> f32 {
    dot_product_impl(a, b)
}

// Public constant
pub const ZERO: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 }

// Re-export from another module
pub use math.matrix.Matrix4
```

### Module Naming Conventions

Modules follow hierarchical naming:

- **Package Name**: `game_engine`
- **Module Path**: `game_engine.graphics.renderer`
- **File Path**: `src/graphics/renderer.script`

```rust
// Module hierarchy example:
module game_engine.graphics.renderer

module game_engine.physics.collision

module game_engine.audio.mixer
```

### File Organization

```
project_root/
├── script.toml              # Package manifest
├── src/
│   ├── main.script          # Entry point (module main)
│   ├── lib.script           # Library root (module project_name)
│   ├── graphics/
│   │   ├── mod.script       # module graphics
│   │   ├── renderer.script  # module graphics.renderer
│   │   ├── mesh.script      # module graphics.mesh
│   │   └── texture.script   # module graphics.texture
│   ├── physics/
│   │   ├── mod.script       # module physics
│   │   └── collision.script # module physics.collision
│   └── utils/
│       └── math.script      # module utils.math
└── tests/
    ├── graphics_tests.script
    └── physics_tests.script
```

## Import and Export System

### Export Syntax

```rust
// Explicit exports
pub fn public_function() -> i32 { 42 }
pub struct PublicStruct { pub field: i32 }
pub const PUBLIC_CONSTANT: i32 = 100

// Private by default
fn private_function() -> i32 { 0 }
struct PrivateStruct { field: i32 }

// Conditional exports (for feature flags)
#[cfg(feature = "debug")]
pub fn debug_function() -> String { "debug" }

// Re-exports
pub use other_module.SomeType
pub use other_module.{ function1, function2 }

// Wildcard re-export (discouraged, but available)
pub use other_module.*
```

### Import Syntax

```rust
// Basic imports
import std.io
import std.collections.HashMap
import graphics.renderer

// Aliased imports
import graphics.renderer as render
import very.long.module.name as short

// Selective imports
import std.collections.{ HashMap, Vec, HashSet }
import graphics.{ Mesh, Texture, Material }

// Wildcard imports (local scope only)
import math.* // Brings all public items into scope

// Conditional imports
#[cfg(feature = "opengl")]
import graphics.opengl.Renderer

// Relative imports (within same package)
import super.sibling_module     // ../sibling_module
import self.child_module        // ./child_module
import crate.other_module       // From package root
```

### Advanced Import Features

#### Import Attributes

```rust
// Lazy loading for large modules
#[lazy]
import large_asset_loader

// Preload for critical modules
#[preload]
import core.engine

// Version constraints
#[version(">=1.0.0, <2.0.0")]
import external_crate
```

#### Scoped Imports

```rust
fn some_function() {
    // Local import scope
    import specialized.algorithm
    
    return algorithm.process(data)
} // algorithm goes out of scope
```

## Module Resolution

### Resolution Algorithm

1. **Absolute Imports**: Start from package root
2. **Relative Imports**: Resolve relative to current module
3. **Standard Library**: Check std library modules
4. **External Packages**: Search in package dependencies
5. **File System Mapping**: Map module names to file paths

### Resolution Process

```rust
// For import graphics.renderer:

1. Check if "graphics.renderer" is in current package
   → Look for src/graphics/renderer.script
   → Look for src/graphics/mod.script with renderer submodule

2. Check standard library
   → Look for std/graphics/renderer.script

3. Check external dependencies
   → Look in each dependency's exported modules

4. Check local paths
   → Look for ./graphics/renderer.script relative to current file

5. Error if not found
   → Report helpful error with suggestions
```

### Module Path Mapping

```rust
Module Name              → File Path
-----------                --------
graphics                 → src/graphics/mod.script
graphics.renderer        → src/graphics/renderer.script
std.io                   → <stdlib>/io/mod.script
std.collections.HashMap  → <stdlib>/collections/hash_map.script
external_crate.module    → <deps>/external_crate/src/module.script
```

### Caching and Performance

```rust
// Module resolution cache
struct ModuleCache {
    resolved_paths: HashMap<String, PathBuf>,
    compiled_modules: HashMap<String, CompiledModule>,
    symbol_tables: HashMap<String, SymbolTable>,
    dependency_graph: DependencyGraph,
}

// Fast lookup for repeated imports
impl ModuleResolver {
    fn resolve(&mut self, module_name: &str) -> Result<ModulePath> {
        if let Some(cached) = self.cache.resolved_paths.get(module_name) {
            return Ok(cached.clone());
        }
        
        let path = self.resolve_fresh(module_name)?;
        self.cache.resolved_paths.insert(module_name.to_string(), path.clone());
        Ok(path)
    }
}
```

## Package Management

### Package Manifest (script.toml)

```toml
[package]
name = "my_game"
version = "0.1.0"
authors = ["Your Name <email@example.com>"]
edition = "2024"
description = "An awesome game built with Script"
license = "MIT"
repository = "https://github.com/user/my_game"

[dependencies]
math = "1.2.0"
graphics = { version = "0.5.0", features = ["opengl", "vulkan"] }
physics = { git = "https://github.com/physics-engine/script-physics" }
audio = { path = "../audio-engine" }

[dev-dependencies]
test_framework = "0.3.0"

[features]
default = ["graphics", "audio"]
opengl = ["graphics/opengl"]
vulkan = ["graphics/vulkan"]
debug = []

[build]
target = "native"
optimization = "speed"

[lib]
name = "my_game"
path = "src/lib.script"

[[bin]]
name = "game"
path = "src/main.script"

[workspace]
members = ["graphics", "physics", "audio"]
```

### Dependency Resolution

```rust
// Semantic versioning support
struct Version {
    major: u32,
    minor: u32,
    patch: u32,
    pre: Option<String>,
    build: Option<String>,
}

// Dependency constraints
enum VersionConstraint {
    Exact(Version),           // =1.2.3
    GreaterEqual(Version),    // >=1.2.0
    Compatible(Version),      // ^1.2.0 (>=1.2.0, <2.0.0)
    Tilde(Version),          // ~1.2.0 (>=1.2.0, <1.3.0)
    Range(Version, Version), // >=1.2.0, <2.0.0
}

// Dependency resolution algorithm
impl PackageResolver {
    fn resolve_dependencies(&self, manifest: &Manifest) -> Result<DependencyGraph> {
        // 1. Build dependency tree
        // 2. Resolve version constraints
        // 3. Detect conflicts
        // 4. Generate lock file
        // 5. Download/build dependencies
    }
}
```

### Package Registry

```rust
// Package registry interface
trait PackageRegistry {
    fn search(&self, query: &str) -> Result<Vec<PackageInfo>>;
    fn download(&self, package: &PackageId) -> Result<PackageArchive>;
    fn publish(&self, package: &Package) -> Result<()>;
    fn info(&self, package: &str) -> Result<PackageMetadata>;
}

// Default registry configuration
struct RegistryConfig {
    default_registry: Url,      // https://packages.script-lang.org
    alternative_registries: Vec<Registry>,
    local_cache: PathBuf,       // ~/.script/registry
    auth_tokens: HashMap<String, String>,
}
```

## Compilation and Linking

### Module Compilation Pipeline

```rust
// Compilation phases for modules
struct ModuleCompiler {
    lexer: Lexer,
    parser: Parser,
    semantic_analyzer: SemanticAnalyzer,
    type_inferrer: TypeInferrer,
    ir_builder: IrBuilder,
    optimizer: Optimizer,
}

impl ModuleCompiler {
    fn compile_module(&mut self, source: &str, module_path: &str) -> Result<CompiledModule> {
        // 1. Parse module declaration
        let module_decl = self.parse_module_declaration(source)?;
        
        // 2. Resolve imports
        let imports = self.resolve_imports(&module_decl.imports)?;
        
        // 3. Create module scope
        let mut scope = ModuleScope::new(module_path);
        scope.add_imports(imports);
        
        // 4. Compile module body
        let ast = self.parser.parse(source)?;
        let typed_ast = self.analyze_semantics(ast, &scope)?;
        let inferred_ast = self.infer_types(typed_ast)?;
        let ir = self.generate_ir(inferred_ast)?;
        let optimized_ir = self.optimizer.optimize(ir)?;
        
        // 5. Extract exports
        let exports = self.extract_exports(&optimized_ir, &module_decl)?;
        
        Ok(CompiledModule {
            path: module_path.to_string(),
            ir: optimized_ir,
            exports,
            imports: module_decl.imports,
            metadata: module_decl.metadata,
        })
    }
}
```

### Incremental Compilation

```rust
// Dependency tracking for incremental builds
struct DependencyTracker {
    file_hashes: HashMap<PathBuf, u64>,
    module_dependencies: HashMap<String, Vec<String>>,
    last_build_time: HashMap<String, SystemTime>,
}

impl DependencyTracker {
    fn needs_rebuild(&self, module: &str) -> bool {
        // Check if module file changed
        if self.file_changed(module) {
            return true;
        }
        
        // Check if any dependency changed
        if let Some(deps) = self.module_dependencies.get(module) {
            for dep in deps {
                if self.needs_rebuild(dep) {
                    return true;
                }
            }
        }
        
        false
    }
}
```

### Linking Process

```rust
// Module linking
struct ModuleLinker {
    compiled_modules: HashMap<String, CompiledModule>,
    symbol_table: GlobalSymbolTable,
    type_registry: TypeRegistry,
}

impl ModuleLinker {
    fn link_modules(&mut self, entry_point: &str) -> Result<ExecutableModule> {
        // 1. Topological sort of dependency graph
        let build_order = self.topological_sort(entry_point)?;
        
        // 2. Link modules in dependency order
        for module_name in build_order {
            self.link_module(&module_name)?;
        }
        
        // 3. Resolve all symbols
        self.resolve_symbols()?;
        
        // 4. Generate final executable
        self.generate_executable(entry_point)
    }
    
    fn link_module(&mut self, module_name: &str) -> Result<()> {
        let module = self.compiled_modules.get(module_name).unwrap();
        
        // Register exported symbols
        for export in &module.exports {
            self.symbol_table.register(export.clone())?;
        }
        
        // Resolve imported symbols
        for import in &module.imports {
            self.resolve_import(import)?;
        }
        
        Ok(())
    }
}
```

## Module Privacy and Visibility

### Visibility Rules

```rust
// Visibility modifiers
enum Visibility {
    Private,           // Default - only within module
    Public,            // pub - exported from module
    Crate,            // pub(crate) - within package only
    Super,            // pub(super) - within parent module
    Module(String),   // pub(in path) - within specific module
}

// Visibility checking
impl VisibilityChecker {
    fn can_access(&self, item: &Item, from_module: &str) -> bool {
        match item.visibility {
            Visibility::Private => self.same_module(item.module, from_module),
            Visibility::Public => true,
            Visibility::Crate => self.same_crate(item.module, from_module),
            Visibility::Super => self.in_parent_module(item.module, from_module),
            Visibility::Module(ref path) => self.in_module(path, from_module),
        }
    }
}
```

### Access Control Examples

```rust
// Module: graphics.renderer
module graphics.renderer

// Private - only within this module
struct RenderCommand { /* ... */ }

// Public - exported from module
pub struct Renderer { /* ... */ }

// Package-only - visible within graphics package
pub(crate) struct InternalBuffer { /* ... */ }

// Parent module only - visible in graphics module
pub(super) struct DebugInfo { /* ... */ }

// Specific module access
pub(in graphics) struct SharedResource { /* ... */ }

// Friend module pattern
#[visible_to(graphics.debug)]
struct DiagnosticData { /* ... */ }
```

## Circular Dependencies

### Detection and Prevention

```rust
// Circular dependency detection
struct CircularDependencyDetector {
    dependency_graph: Graph<String>,
    visiting: HashSet<String>,
    visited: HashSet<String>,
}

impl CircularDependencyDetector {
    fn detect_cycles(&mut self, start: &str) -> Result<()> {
        if self.visiting.contains(start) {
            return Err(Error::CircularDependency(self.get_cycle_path(start)));
        }
        
        if self.visited.contains(start) {
            return Ok(());
        }
        
        self.visiting.insert(start.to_string());
        
        for dependency in self.dependency_graph.neighbors(start) {
            self.detect_cycles(dependency)?;
        }
        
        self.visiting.remove(start);
        self.visited.insert(start.to_string());
        Ok(())
    }
}
```

### Handling Strategies

1. **Forward Declarations**: Allow declaring types before definition
2. **Interface Modules**: Use trait/interface modules to break cycles
3. **Dependency Injection**: Use dependency injection patterns
4. **Event Systems**: Use event-driven communication

```rust
// Forward declaration example
module graphics.mesh

// Forward declare before use
forward struct Material

pub struct Mesh {
    material: Material,  // Forward declared
    vertices: Vec<Vertex>,
}

// Interface module pattern
module graphics.interfaces

pub trait Drawable {
    fn draw(&self, renderer: &Renderer);
}

module graphics.mesh
import graphics.interfaces.Drawable

impl Drawable for Mesh {
    fn draw(&self, renderer: &Renderer) { /* ... */ }
}
```

## Standard Library Integration

### Standard Library Structure

```
std/
├── mod.script                  # Standard library root
├── core/
│   ├── mod.script             # Core types and traits
│   ├── primitives.script      # i32, f32, bool, string
│   ├── option.script          # Option<T> type
│   ├── result.script          # Result<T, E> type
│   └── iterator.script        # Iterator trait
├── collections/
│   ├── mod.script
│   ├── array.script           # Dynamic arrays
│   ├── hash_map.script        # Hash maps
│   ├── hash_set.script        # Hash sets
│   └── linked_list.script     # Linked lists
├── io/
│   ├── mod.script
│   ├── file.script            # File I/O
│   ├── stream.script          # Stream abstractions
│   └── console.script         # Console I/O
├── math/
│   ├── mod.script
│   ├── basic.script           # Basic math functions
│   ├── vector.script          # Vector operations
│   ├── matrix.script          # Matrix operations
│   └── random.script          # Random number generation
└── game/
    ├── mod.script
    ├── graphics.script        # Graphics primitives
    ├── input.script           # Input handling
    ├── audio.script           # Audio system
    └── time.script            # Timing utilities
```

### Standard Library Imports

```rust
// Automatic prelude imports (available without explicit import)
// These are implicitly available in every module:
use std.core.{
    Option, Some, None,
    Result, Ok, Err,
    i32, f32, bool, string,
    print, println, assert,
}

// Explicit imports for other functionality
import std.collections.HashMap
import std.io.File
import std.math.Vector3
import std.game.Input
```

## Implementation Plan

### Phase 1: Basic Module System (Planned)

**Timeline**: Phase 6.1 of development roadmap

**Features**:
- Module declaration syntax
- Basic import/export system
- File-based module resolution
- Compilation integration

**Implementation Steps**:
1. Extend lexer for module keywords
2. Add module AST nodes to parser
3. Implement module resolver
4. Update semantic analyzer for module scopes
5. Modify compilation pipeline

### Phase 2: Package Management (Planned)

**Timeline**: Phase 6.2 of development roadmap

**Features**:
- Package manifests (script.toml)
- Dependency resolution
- Version management
- Basic package registry

**Implementation Steps**:
1. Design manifest format
2. Implement dependency resolver
3. Create package cache system
4. Build registry client
5. Integration with build system

### Phase 3: Advanced Features (Future)

**Timeline**: Phase 6.3+ of development roadmap

**Features**:
- Incremental compilation
- Module-level optimizations
- Package publishing
- IDE integration

### Compiler Integration

```rust
// Planned compiler changes for module support

// Lexer changes
enum TokenKind {
    // Existing tokens...
    
    // New module tokens
    Module,      // module
    Import,      // import
    Export,      // pub
    Use,         // use
    As,          // as
    Super,       // super
    Crate,       // crate
    Self_,       // self
}

// Parser changes
enum Stmt {
    // Existing statements...
    
    // New module statements
    ModuleDecl { name: ModulePath, span: Span },
    Import { path: ImportPath, items: ImportItems, span: Span },
    Use { path: UsePath, alias: Option<String>, span: Span },
}

// Semantic analyzer changes
struct ModuleScope {
    name: String,
    exports: HashMap<String, Symbol>,
    imports: HashMap<String, ImportedSymbol>,
    children: HashMap<String, ModuleScope>,
    parent: Option<String>,
}

// IR changes
struct IrModule {
    name: String,
    functions: Vec<Function>,
    types: Vec<TypeDef>,
    exports: Vec<Export>,
    imports: Vec<Import>,
    dependencies: Vec<String>,
}
```

### Integration Points

1. **Build System**: Module-aware build pipeline
2. **IDE Support**: Auto-completion and navigation
3. **Testing**: Module-specific test discovery
4. **Documentation**: Module-level documentation generation
5. **Debugging**: Module-aware debugging information

This module system design provides a solid foundation for organizing Script code into maintainable, reusable components while supporting the language's goals of simplicity, performance, and game development focus.