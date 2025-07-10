# Generic Types Implementation Plan

## Current State Analysis

### ✅ Already Implemented:
1. **Type System Foundation** (`src/types/generics.rs`)
   - Generic type definitions (`GenericType`, `TypeParam`, `TraitBound`)
   - Built-in traits (`Eq`, `Ord`, `Clone`, etc.)
   - Generic environment tracking (`GenericEnv`)
   - Constraint checking infrastructure

2. **AST Support** (`src/parser/ast.rs`)
   - Generic parameter definitions (`GenericParam`, `GenericParams`)
   - Generic type annotations (`TypeKind::Generic`, `TypeKind::TypeParam`)
   - Generic constructor expressions
   - Function definitions with generic parameter placeholders

### ❌ Missing Components:
1. **Parser Implementation**
   - No parsing of generic syntax (`<T>`, `<T: Eq>`, etc.)
   - Function generic params always set to `None` with TODO comment
   - No where clause parsing

2. **Semantic Analysis**
   - No handling of generic types in semantic analyzer
   - No generic function validation
   - No trait bound checking

3. **Type Inference**
   - No generic type instantiation
   - No constraint propagation for generics
   - No monomorphization support

## Implementation Plan

### Phase 1: Parser Support (High Priority)
**Goal**: Enable parsing of generic syntax in function declarations and type annotations

#### 1.1 Lexer Updates
```rust
// File: src/lexer/scanner.rs
// Add angle bracket tokenization with disambiguation
```

**Tasks**:
- [ ] Handle `<` and `>` as both comparison and generic delimiters
- [ ] Add lookahead for disambiguation (e.g., `<` followed by identifier)
- [ ] Support nested angle brackets tracking

#### 1.2 Parser Generic Functions
```rust
// File: src/parser/parser.rs
// In parse_function_common() around line 149
```

**Tasks**:
- [ ] Implement `parse_generic_params()` method
- [ ] Parse syntax: `fn identity<T>(x: T) -> T`
- [ ] Parse bounds: `fn sort<T: Ord>(items: Vec<T>)`
- [ ] Update function parsing to call `parse_generic_params()`

#### 1.3 Parser Type Annotations
```rust
// File: src/parser/parser.rs
// In parse_type_annotation()
```

**Tasks**:
- [ ] Parse generic type instantiations: `Vec<i32>`
- [ ] Handle nested generics: `Result<Vec<T>, Error<E>>`
- [ ] Parse type parameters in signatures

### Phase 2: Semantic Analysis Integration (High Priority)
**Goal**: Validate generic usage and track generic contexts

#### 2.1 Generic Context Management
```rust
// File: src/semantic/analyzer.rs
```

**Tasks**:
- [ ] Add generic parameter tracking to `AnalysisContext`
- [ ] Implement scope management for type parameters
- [ ] Validate type parameter usage in function bodies

#### 2.2 Generic Type Resolution
```rust
// File: src/semantic/analyzer.rs
// In analyze_type_annotation()
```

**Tasks**:
- [ ] Resolve type parameters to their definitions
- [ ] Validate generic type argument counts
- [ ] Check trait bound satisfaction

### Phase 3: Type Inference Support (Medium Priority)
**Goal**: Enable type inference for generic functions

#### 3.1 Generic Function Inference
```rust
// File: src/inference/inference_engine.rs
```

**Tasks**:
- [ ] Track generic instantiations during inference
- [ ] Implement type parameter substitution
- [ ] Generate constraints from trait bounds

#### 3.2 Constraint Solving
```rust
// File: src/inference/unification.rs
```

**Tasks**:
- [ ] Extend unification to handle type parameters
- [ ] Implement trait bound constraint solving
- [ ] Add generic type compatibility checks

### Phase 4: Monomorphization (Medium Priority)
**Goal**: Generate specialized versions of generic functions

#### 4.1 Instantiation Collection
```rust
// File: src/lowering/mod.rs (new module)
```

**Tasks**:
- [ ] Collect all generic function calls
- [ ] Track concrete type arguments used
- [ ] Build instantiation dependency graph

#### 4.2 Code Generation
```rust
// File: src/codegen/cranelift/translator.rs
```

**Tasks**:
- [ ] Generate specialized function versions
- [ ] Update call sites to use specialized versions
- [ ] Handle generic type memory layouts

## Minimal Implementation for Basic Generics

To get a working `identity<T>` function, the minimal changes needed are:

### 1. Parser Changes (src/parser/parser.rs)
```rust
// Add after line 114 (before parse_function_common)
fn parse_generic_params(&mut self) -> Result<Option<GenericParams>> {
    if !self.check(&TokenKind::Less) {
        return Ok(None);
    }
    
    self.advance(); // consume '<'
    let mut params = Vec::new();
    
    loop {
        let name = self.consume_identifier("Expected type parameter name")?;
        let mut bounds = Vec::new();
        
        if self.match_token(&TokenKind::Colon) {
            // Parse trait bounds
            loop {
                let trait_name = self.consume_identifier("Expected trait name")?;
                bounds.push(TraitBound {
                    trait_name,
                    span: self.previous_span(),
                });
                
                if !self.match_token(&TokenKind::Plus) {
                    break;
                }
            }
        }
        
        params.push(GenericParam {
            name,
            bounds,
            span: self.previous_span(),
        });
        
        if !self.match_token(&TokenKind::Comma) {
            break;
        }
    }
    
    self.consume(&TokenKind::Greater, "Expected '>' after generic parameters")?;
    
    Ok(Some(GenericParams {
        params,
        span: self.span_from(start),
    }))
}

// Update line 149 in parse_function_common:
generic_params: self.parse_generic_params()?,
```

### 2. Semantic Analysis (src/semantic/analyzer.rs)
```rust
// Add to AnalysisContext:
generic_params: HashMap<String, Vec<TraitBound>>,

// Add method to handle generic contexts:
fn enter_generic_scope(&mut self, params: &Option<GenericParams>) {
    if let Some(params) = params {
        let mut generic_map = HashMap::new();
        for param in &params.params {
            generic_map.insert(param.name.clone(), param.bounds.clone());
        }
        self.current_context_mut().generic_params = generic_map;
    }
}

// Update analyze_function to call enter_generic_scope
```

### 3. Type Resolution (src/inference/type_conversion.rs)
```rust
// Update type_ann_to_type to handle TypeParam:
TypeKind::TypeParam(name) => {
    // Check if this is a valid type parameter in scope
    Type::TypeVar(self.fresh_type_var())
}
```

## Testing Strategy

### Unit Tests
1. Parser tests for generic syntax
2. Type inference tests for generic functions
3. Constraint checking tests

### Integration Tests
```script
// Test basic identity function
fn identity<T>(x: T) -> T {
    x
}

let num = identity(42)        // Should infer T = i32
let str = identity("hello")   // Should infer T = string

// Test with constraints
fn min<T: Ord>(a: T, b: T) -> T {
    if a < b { a } else { b }
}

let x = min(3, 5)            // Should work
let y = min("a", "b")        // Should fail - string doesn't implement Ord
```

## Priority Order

1. **Week 1**: Parser support for generic functions
2. **Week 2**: Basic semantic analysis and type parameter tracking  
3. **Week 3**: Type inference for simple generic functions
4. **Week 4**: Trait bound checking and error reporting
5. **Week 5**: Monomorphization for code generation
6. **Week 6**: Testing and documentation

## Success Criteria

The implementation is complete when:
1. ✅ Can parse generic function declarations
2. ✅ Can call generic functions with type inference
3. ✅ Trait bounds are enforced
4. ✅ Error messages clearly explain constraint violations
5. ✅ Generated code is efficient (via monomorphization)
6. ✅ All tests pass

## Next Steps

1. Start with parser implementation (Phase 1.2)
2. Add minimal semantic analysis support
3. Implement basic type inference
4. Create comprehensive test suite
5. Document usage patterns