# Generic Types Implementation Guide

## Overview

This document provides a detailed implementation guide for the generic type system in Script. It covers the integration with existing systems and the step-by-step implementation approach.

## Architecture Integration

### Type System Extensions

The generic type system integrates with the existing type system by:

1. **Extending the Type enum** with new variants:
   - `Generic { name: String, args: Vec<Type> }` - For instantiated generic types
   - `TypeParam(String)` - For type parameters in generic contexts

2. **Enhancing the constraint system** with new constraint types:
   - `TraitBound` - For type parameter bounds
   - `GenericBounds` - For multiple bounds on a type parameter

3. **Adding generic environment tracking** via `GenericEnv` for:
   - Type parameter substitutions
   - Trait implementation checking
   - Constraint satisfaction verification

### AST Extensions

New AST nodes support generic syntax:

```rust
// Generic parameters in function/struct definitions
pub struct GenericParams {
    pub params: Vec<GenericParam>,
    pub span: Span,
}

pub struct GenericParam {
    pub name: String,
    pub bounds: Vec<TraitBound>,
    pub span: Span,
}

// Function definitions now include generic parameters
StmtKind::Function {
    name: String,
    generic_params: Option<GenericParams>,  // NEW
    params: Vec<Param>,
    ret_type: Option<TypeAnn>,
    body: Block,
    is_async: bool,
}

// Type annotations support generic syntax
TypeKind::Generic {
    name: String,
    args: Vec<TypeAnn>,
}
TypeKind::TypeParam(String),
```

## Implementation Phases

### Phase 1: Basic Generic Types (Foundation)

#### 1.1 Type System Core
- [x] Extend `Type` enum with `Generic` and `TypeParam` variants
- [x] Update type equality and display methods
- [x] Create `types/generics.rs` module with core types

#### 1.2 AST Support
- [x] Add `GenericParams`, `GenericParam`, `TraitBound` to AST
- [x] Extend function definitions with generic parameters
- [x] Add generic type annotations (`TypeKind::Generic`, `TypeKind::TypeParam`)
- [x] Update Display implementations

#### 1.3 Constraint System
- [x] Extend constraint types with `TraitBound` and `GenericBounds`
- [x] Add constraint creation helpers
- [x] Update constraint display and testing

### Phase 2: Parsing Support

#### 2.1 Lexer Extensions
- [ ] Add tokens for generic syntax: `<`, `>`, `where`
- [ ] Handle angle bracket disambiguation (less-than vs generic)
- [ ] Support trait names in bound contexts

#### 2.2 Parser Implementation
- [ ] Parse generic parameter lists: `<T, U: Eq, V: Ord + Clone>`
- [ ] Parse where clauses: `where T: Clone, U: Default`
- [ ] Parse generic type annotations: `Vec<i32>`, `Map<string, bool>`
- [ ] Parse generic function calls with explicit types: `identity::<i32>(42)`
- [ ] Handle parser precedence for `<` and `>` tokens

```rust
// Example parsing methods to implement
impl Parser {
    fn parse_generic_params(&mut self) -> Result<Option<GenericParams>, Error>
    fn parse_type_param(&mut self) -> Result<GenericParam, Error>
    fn parse_trait_bounds(&mut self) -> Result<Vec<TraitBound>, Error>
    fn parse_where_clause(&mut self) -> Result<WhereClause, Error>
    fn parse_generic_type(&mut self, name: String) -> Result<TypeAnn, Error>
}
```

### Phase 3: Type Inference Integration

#### 3.1 Generic Context Tracking
- [ ] Extend `InferenceContext` to track generic environments
- [ ] Add type parameter scoping (enter/exit generic contexts)
- [ ] Handle type parameter substitutions during inference

#### 3.2 Constraint Generation
- [ ] Generate trait bound constraints for generic parameters
- [ ] Propagate constraints from function signatures
- [ ] Handle generic function calls and type instantiation

#### 3.3 Inference Engine Updates
```rust
impl InferenceEngine {
    // Track generic contexts
    fn enter_generic_context(&mut self, params: &GenericParams)
    fn exit_generic_context(&mut self)
    
    // Handle generic functions
    fn infer_generic_function(&mut self, ...) -> Result<Type, Error>
    fn instantiate_generic_type(&mut self, ...) -> Result<Type, Error>
    
    // Constraint checking
    fn check_trait_bounds(&mut self, ...) -> Result<(), Error>
    fn verify_generic_constraints(&mut self, ...) -> Result<(), Error>
}
```

### Phase 4: Built-in Trait System

#### 4.1 Trait Definitions
- [ ] Define built-in traits (`Eq`, `Ord`, `Clone`, `Display`, etc.)
- [ ] Implement automatic trait derivation for primitive types
- [ ] Add trait dependency tracking (e.g., `Ord` requires `Eq`)

#### 4.2 Trait Implementation Checking
- [ ] Create trait implementation registry
- [ ] Add structural trait implementations (arrays, tuples, etc.)
- [ ] Implement trait constraint verification

```rust
// Example trait implementation
impl TraitChecker {
    fn check_eq_impl(&self, type_: &Type) -> bool
    fn check_ord_impl(&self, type_: &Type) -> bool
    fn check_clone_impl(&self, type_: &Type) -> bool
    fn get_trait_dependencies(&self, trait_name: &str) -> Vec<String>
}
```

### Phase 5: Monomorphization

#### 5.1 Generic Instantiation Analysis
- [ ] Collect all generic function instantiations
- [ ] Track which generic types are actually used
- [ ] Build instantiation dependency graph

#### 5.2 Code Generation Integration
- [ ] Generate specialized function versions
- [ ] Update function names with type suffixes
- [ ] Handle generic type layout and sizing

```rust
// Example monomorphization
impl Monomorphizer {
    fn collect_instantiations(&mut self, program: &Program) -> Vec<Instantiation>
    fn generate_specialized_function(&mut self, ...) -> Result<Function, Error>
    fn mangle_generic_name(&self, name: &str, types: &[Type]) -> String
}
```

### Phase 6: Error Handling and Diagnostics

#### 6.1 Generic-Specific Errors
- [ ] Type parameter not found
- [ ] Constraint not satisfied
- [ ] Wrong number of type arguments
- [ ] Recursive type definitions

#### 6.2 Enhanced Error Messages
- [ ] Show which constraints are missing
- [ ] Suggest correct type parameter counts
- [ ] Display constraint requirements clearly

## Implementation Priority

### High Priority (Core Functionality)
1. **Parsing Support** - Users need to write generic code
2. **Basic Type Inference** - Generic functions must be usable
3. **Built-in Trait System** - Essential for useful constraints
4. **Error Handling** - Good diagnostics are crucial

### Medium Priority (Usability)
1. **Monomorphization** - Performance optimization
2. **Advanced Constraints** - Complex where clauses
3. **Generic Structs** - User-defined generic types

### Low Priority (Advanced Features)
1. **Associated Types** - Complex trait relationships
2. **Higher-Kinded Types** - Advanced type-level programming
3. **Generic Traits** - User-defined traits

## Testing Strategy

### Unit Tests
- Type equality with generic types
- Constraint generation and checking
- Generic environment operations
- AST construction and display

### Integration Tests
- Parse and type-check generic functions
- Generic function calls with inference
- Constraint satisfaction checking
- Error message quality

### Example Programs
- Generic containers (Vec, Map, Set)
- Generic algorithms (sort, search, map, filter)
- Generic error handling (Result, Option)
- Real-world use cases

## Migration Path

### Backward Compatibility
- Existing non-generic code continues to work
- Generic features are opt-in
- No breaking changes to current syntax

### Gradual Adoption
1. Start with simple generic functions
2. Add generic containers to standard library
3. Introduce generic traits and bounds
4. Advanced features for power users

## Performance Considerations

### Compile Time
- Generic instantiation can increase compile time
- Need efficient constraint solving algorithms
- Monomorphization can generate many functions

### Runtime
- Monomorphization eliminates runtime overhead
- Generic abstractions compile to efficient code
- No dynamic dispatch for generic calls

### Memory Usage
- Multiple instantiations increase binary size
- Need to balance generics vs code bloat
- Consider generic specialization strategies

## Standard Library Integration

### Generic Collections
```script
// Standard library generic types to implement
struct Vec<T> { ... }
struct Map<K: Eq, V> { ... }
struct Set<T: Eq> { ... }
enum Option<T> { Some(T), None }
enum Result<T, E> { Ok(T), Err(E) }
```

### Generic Functions
```script
// Standard library generic functions
fn map<T, U>(arr: [T], f: (T) -> U) -> [U]
fn filter<T>(arr: [T], pred: (T) -> bool) -> [T]
fn reduce<T, U>(arr: [T], init: U, f: (U, T) -> U) -> U
fn sort<T: Ord>(arr: [T]) -> [T]
```

### Built-in Trait Implementations
- All primitive types implement basic traits
- Arrays implement traits if elements do
- Function types implement limited traits
- Generic types implement traits structurally

## Development Tools

### Debugging Support
- Show generic instantiations in debug output
- Display constraint solving steps
- Trace type parameter substitutions

### IDE Integration
- Generic type information in hover
- Constraint satisfaction feedback
- Generic function signature help

### Documentation
- Generate docs for generic types and functions
- Show constraint requirements clearly
- Provide usage examples

---

This implementation guide provides a roadmap for adding comprehensive generic types to Script while maintaining the language's focus on simplicity and gradual typing.