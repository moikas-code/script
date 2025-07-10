# User-Defined Types Implementation Plan

## Overview

This document outlines the implementation strategy for user-defined types (structs and enums) in the Script language. It breaks down the work into manageable phases and provides specific technical guidance.

## Implementation Phases

### Phase 1: Foundation (Lexer & Parser)

#### 1.1 Lexer Extensions
**Files to modify:**
- `src/lexer/token.rs`
- `src/lexer/scanner.rs`

**Changes needed:**
```rust
// Add to TokenKind enum
pub enum TokenKind {
    // ... existing tokens ...
    
    // New keywords
    Struct,
    Enum,
    ColonColon,  // :: for enum constructors
}

// Update from_keyword function
fn from_keyword(word: &str) -> Option<Self> {
    match word {
        // ... existing keywords ...
        "struct" => Some(TokenKind::Struct),
        "enum" => Some(TokenKind::Enum),
        _ => None,
    }
}
```

**Scanner updates:**
- Add `::` token recognition in `scan_token()`
- Update keyword table with `struct` and `enum`

#### 1.2 AST Extensions
**Files to modify:**
- `src/parser/ast.rs`

**New AST nodes:**
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    // ... existing statements ...
    
    StructDecl {
        name: String,
        type_params: Vec<String>,
        fields: Vec<StructFieldDecl>,
        span: Span,
    },
    
    EnumDecl {
        name: String,
        type_params: Vec<String>,
        variants: Vec<EnumVariantDecl>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructFieldDecl {
    pub name: String,
    pub ty: TypeAnn,
    pub is_private: bool,
    pub default_value: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariantDecl {
    pub name: String,
    pub data: EnumVariantData,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnumVariantData {
    Unit,                                    // Status::Pending
    Tuple(Vec<TypeAnn>),                    // Result::Ok(T)
    Struct(Vec<StructFieldDecl>),           // Message::Image { url, alt }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    // ... existing expressions ...
    
    StructExpr {
        name: String,
        fields: Vec<StructFieldInit>,
        base: Option<Box<Expr>>,  // for update syntax
    },
    
    EnumConstructor {
        enum_name: String,
        variant: String,
        data: Option<EnumConstructorData>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructFieldInit {
    pub name: String,
    pub value: Option<Expr>,  // None for field punning
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnumConstructorData {
    Tuple(Vec<Expr>),
    Struct(Vec<StructFieldInit>),
}
```

#### 1.3 Parser Implementation
**Files to modify:**
- `src/parser/parser.rs`

**New parsing functions:**
```rust
impl Parser {
    // Parse struct declaration
    fn parse_struct_decl(&mut self) -> Result<Stmt, Error> {
        self.consume(TokenKind::Struct, "Expected 'struct'")?;
        let name = self.consume_identifier("Expected struct name")?;
        
        let type_params = if self.check(&TokenKind::Less) {
            self.parse_generic_params()?
        } else {
            Vec::new()
        };
        
        self.consume(TokenKind::LeftBrace, "Expected '{' after struct name")?;
        let fields = self.parse_struct_fields()?;
        self.consume(TokenKind::RightBrace, "Expected '}' after struct fields")?;
        
        Ok(Stmt {
            kind: StmtKind::StructDecl { name, type_params, fields },
            span: /* calculate span */,
            attributes: Vec::new(),
        })
    }
    
    // Parse enum declaration  
    fn parse_enum_decl(&mut self) -> Result<Stmt, Error> {
        // Similar to parse_struct_decl but for enums
    }
    
    // Parse struct expression
    fn parse_struct_expr(&mut self, name: String) -> Result<Expr, Error> {
        // Parse { field: value, field2, ..base }
    }
    
    // Parse enum constructor
    fn parse_enum_constructor(&mut self, enum_name: String, variant: String) -> Result<Expr, Error> {
        // Parse Status::Ok(value) or Message::Text { content }
    }
}
```

### Phase 2: Type System Integration

#### 2.1 Type Representation
**Files to modify:**
- `src/types/mod.rs`

**Extend Type enum:**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    // ... existing types ...
    
    /// User-defined struct type
    Struct {
        name: String,
        fields: HashMap<String, StructField>,
        type_params: Vec<String>,
        module_path: Vec<String>,  // for scoping
    },
    
    /// User-defined enum type
    Enum {
        name: String,
        variants: HashMap<String, EnumVariant>,
        type_params: Vec<String>,
        module_path: Vec<String>,
    },
    
    /// Generic type instantiation
    Generic {
        base: Box<Type>,
        args: Vec<Type>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
    pub is_private: bool,
    pub default_value: Option<Box<Expr>>,  // Store default expressions
    pub field_index: usize,  // for efficient access
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumVariant {
    pub name: String,
    pub data: EnumVariantType,
    pub discriminant: u32,  // for efficient matching
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EnumVariantType {
    Unit,
    Tuple(Vec<Type>),
    Struct(HashMap<String, StructField>),
}
```

**Update type methods:**
```rust
impl Type {
    /// Get field type from struct
    pub fn get_field(&self, field_name: &str) -> Option<&Type> {
        match self {
            Type::Struct { fields, .. } => {
                fields.get(field_name).map(|f| &f.ty)
            }
            _ => None,
        }
    }
    
    /// Get enum variant
    pub fn get_variant(&self, variant_name: &str) -> Option<&EnumVariant> {
        match self {
            Type::Enum { variants, .. } => variants.get(variant_name),
            _ => None,
        }
    }
    
    /// Check if type is a specific struct
    pub fn is_struct(&self, name: &str) -> bool {
        matches!(self, Type::Struct { name: n, .. } if n == name)
    }
    
    /// Check if type is a specific enum
    pub fn is_enum(&self, name: &str) -> bool {
        matches!(self, Type::Enum { name: n, .. } if n == name)
    }
}
```

#### 2.2 Type Environment Updates
**Files to modify:**
- `src/types/mod.rs`

**Extend TypeEnv:**
```rust
impl TypeEnv {
    /// Define a struct type
    pub fn define_struct(&mut self, name: String, struct_type: Type) -> Result<(), Error> {
        if self.type_defs.contains_key(&name) {
            return Err(Error::new(
                ErrorKind::TypeError,
                format!("Type '{}' is already defined", name),
                Span::default(),
            ));
        }
        self.type_defs.insert(name, struct_type);
        Ok(())
    }
    
    /// Define an enum type
    pub fn define_enum(&mut self, name: String, enum_type: Type) -> Result<(), Error> {
        // Similar to define_struct
    }
    
    /// Get all variants of an enum type
    pub fn get_enum_variants(&self, enum_name: &str) -> Option<&HashMap<String, EnumVariant>> {
        match self.lookup_type(enum_name)? {
            Type::Enum { variants, .. } => Some(variants),
            _ => None,
        }
    }
}
```

### Phase 3: Pattern Matching Extensions

#### 3.1 Pattern AST Updates
**Files to modify:**
- `src/parser/ast.rs`

**Extend PatternKind:**
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum PatternKind {
    // ... existing patterns ...
    
    /// Struct pattern: Point { x, y }
    Struct {
        name: String,
        fields: Vec<(String, Option<Pattern>)>,
        rest: bool,  // for .. patterns
    },
    
    /// Enum pattern: Result::Ok(value)
    Enum {
        enum_name: String,
        variant: String,
        data: Option<Box<Pattern>>,
    },
}
```

#### 3.2 Pattern Parsing
**Files to modify:**
- `src/parser/parser.rs`

**Add pattern parsing methods:**
```rust
impl Parser {
    fn parse_struct_pattern(&mut self, name: String) -> Result<Pattern, Error> {
        // Parse Point { x, y: z, .. }
    }
    
    fn parse_enum_pattern(&mut self, enum_name: String, variant: String) -> Result<Pattern, Error> {
        // Parse Result::Ok(value) or Message::Text { content }
    }
}
```

### Phase 4: Semantic Analysis

#### 4.1 Symbol Table Extensions
**Files to modify:**
- `src/semantic/symbol_table.rs`

**Extend Symbol enum:**
```rust
#[derive(Debug, Clone)]
pub enum Symbol {
    // ... existing symbols ...
    
    Struct {
        name: String,
        fields: HashMap<String, StructField>,
        type_params: Vec<String>,
        methods: Vec<FunctionSignature>,  // for future method support
    },
    
    Enum {
        name: String,
        variants: HashMap<String, EnumVariant>,
        type_params: Vec<String>,
        methods: Vec<FunctionSignature>,
    },
    
    EnumVariant {
        enum_name: String,
        variant_name: String,
        variant_type: EnumVariant,
    },
}
```

#### 4.2 Semantic Analysis Updates
**Files to modify:**
- `src/semantic/analyzer.rs`

**Add analysis methods:**
```rust
impl SemanticAnalyzer {
    fn analyze_struct_decl(&mut self, stmt: &Stmt) -> Result<(), Error> {
        // 1. Check for name conflicts
        // 2. Validate field types
        // 3. Check for recursive definitions
        // 4. Add to symbol table
    }
    
    fn analyze_enum_decl(&mut self, stmt: &Stmt) -> Result<(), Error> {
        // Similar to analyze_struct_decl
    }
    
    fn analyze_struct_expr(&mut self, expr: &Expr) -> Result<Type, Error> {
        // 1. Look up struct type
        // 2. Validate all required fields present
        // 3. Check field types
        // 4. Handle field punning and update syntax
    }
    
    fn analyze_enum_constructor(&mut self, expr: &Expr) -> Result<Type, Error> {
        // 1. Look up enum and variant
        // 2. Validate constructor data matches variant
        // 3. Type check variant data
    }
    
    fn check_pattern_exhaustiveness(&mut self, match_expr: &Expr) -> Result<(), Error> {
        // For enum types, ensure all variants are covered
    }
}
```

### Phase 5: Type Inference Integration

#### 5.1 Inference Engine Updates
**Files to modify:**
- `src/inference/inference_engine.rs`

**Add inference methods:**
```rust
impl InferenceEngine {
    fn infer_struct_expr(&mut self, expr: &Expr) -> Result<Type, Error> {
        // Infer struct type from constructor
        // Handle field type inference
        // Generate constraints for field assignments
    }
    
    fn infer_field_access(&mut self, object_expr: &Expr, field_name: &str) -> Result<Type, Error> {
        // Infer that object_expr has a field of this name
        // Generate appropriate constraints
    }
    
    fn infer_enum_constructor(&mut self, expr: &Expr) -> Result<Type, Error> {
        // Infer enum type from constructor
        // Handle variant data type inference
    }
}
```

### Phase 6: IR Generation and Lowering

#### 6.1 IR Extensions
**Files to modify:**
- `src/ir/instruction.rs`

**Add new instructions:**
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    // ... existing instructions ...
    
    /// Create struct instance
    StructConstruct {
        struct_type: Type,
        field_values: Vec<Value>,
        result: Register,
    },
    
    /// Access struct field
    FieldAccess {
        object: Register,
        field_index: usize,
        result: Register,
    },
    
    /// Create enum instance
    EnumConstruct {
        enum_type: Type,
        variant_index: u32,
        data: Option<Vec<Value>>,
        result: Register,
    },
    
    /// Match enum variant
    EnumMatch {
        value: Register,
        variant_index: u32,
        success_block: BlockId,
        fail_block: BlockId,
    },
}
```

#### 6.2 Lowering Implementation
**Files to modify:**
- `src/lowering/expr.rs`
- `src/lowering/stmt.rs`

**Add lowering methods:**
```rust
impl ExprLowering {
    fn lower_struct_expr(&mut self, expr: &Expr) -> Result<Register, Error> {
        // Generate IR for struct construction
        // Handle field initialization order
        // Apply default values
    }
    
    fn lower_field_access(&mut self, object: &Expr, field: &str) -> Result<Register, Error> {
        // Generate IR for field access
        // Handle visibility checks
    }
    
    fn lower_enum_constructor(&mut self, expr: &Expr) -> Result<Register, Error> {
        // Generate IR for enum construction
        // Handle variant data
    }
}
```

### Phase 7: Memory Management Integration

#### 7.1 RC Integration
**Files to modify:**
- `src/runtime/value.rs`

**Extend Value enum:**
```rust
#[derive(Debug, Clone)]
pub enum Value {
    // ... existing values ...
    
    Struct {
        type_name: String,
        fields: HashMap<String, Rc<Value>>,
    },
    
    Enum {
        type_name: String,
        variant: String,
        data: Option<Rc<Value>>,
    },
}
```

### Phase 8: Testing and Examples

#### 8.1 Unit Tests
**New test files:**
- `tests/user_defined_types_tests.rs`
- `tests/struct_tests.rs`
- `tests/enum_tests.rs`
- `tests/pattern_matching_structs_tests.rs`
- `tests/generic_types_tests.rs`

#### 8.2 Integration Tests
**Test scenarios:**
- Basic struct definition and usage
- Enum definition with various variant types
- Pattern matching exhaustiveness
- Generic type instantiation
- Memory management with cycles
- Error handling and reporting

#### 8.3 Example Programs
**New example files:**
- `examples/structs_basic.script`
- `examples/enums_advanced.script`
- `examples/pattern_matching_comprehensive.script`
- `examples/generic_data_structures.script`

## Implementation Order

1. **Phase 1**: Start with basic lexer and parser changes
2. **Phase 2**: Implement type system foundations
3. **Phase 3**: Add pattern matching support
4. **Phase 4**: Implement semantic analysis
5. **Phase 5**: Integrate with type inference
6. **Phase 6**: Generate IR and implement lowering
7. **Phase 7**: Integrate with memory management
8. **Phase 8**: Comprehensive testing

## Testing Strategy

### Unit Testing
- Test each phase independently
- Mock dependencies where needed
- Focus on edge cases and error conditions

### Integration Testing
- Test cross-module interactions
- Verify type system consistency
- Test pattern matching exhaustiveness

### Performance Testing
- Memory usage with large structs/enums
- Pattern matching performance
- Generic type instantiation overhead

## Error Handling

### Common Error Cases
1. **Undefined types**: Reference to non-existent struct/enum
2. **Missing fields**: Struct construction without required fields
3. **Invalid variants**: Enum construction with wrong variant
4. **Visibility violations**: Access to private fields
5. **Type mismatches**: Assigning wrong types to fields
6. **Recursive definitions**: Infinite type recursion
7. **Non-exhaustive patterns**: Missing enum variants in match

### Error Recovery
- Continue parsing after struct/enum declaration errors
- Provide helpful suggestions for common mistakes
- Show available fields/variants in error messages

## Future Extensions

### Method Support
- Add methods to structs and enums
- Implement trait system
- Support for operator overloading

### Advanced Generics
- Trait bounds on generic parameters
- Associated types
- Higher-kinded types

### Memory Optimizations
- Struct field reordering for optimal packing
- Enum variant optimization
- Zero-cost abstractions for generic types

This implementation plan provides a structured approach to adding user-defined types to the Script language while maintaining consistency with existing architecture and design principles.