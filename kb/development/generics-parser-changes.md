# Generic Parser Implementation - Specific Code Changes

## Overview
This document provides the exact code changes needed to implement generic function parsing in the Script language parser.

## 1. Parser Method Additions

### Location: `src/parser/parser.rs`

Add these methods after the `parse_attribute` method (around line 90):

```rust
/// Parse generic parameters: <T>, <T: Eq>, <T, U: Ord + Clone>
fn parse_generic_params(&mut self) -> Result<Option<GenericParams>> {
    // Check if we have a '<' token
    if !self.check(&TokenKind::Less) {
        return Ok(None);
    }
    
    let start = self.current_location();
    self.advance(); // consume '<'
    
    let mut params = Vec::new();
    
    // Parse type parameters
    loop {
        let param_start = self.current_location();
        let name = self.consume_identifier("Expected type parameter name")?;
        
        // Parse optional trait bounds after ':'
        let mut bounds = Vec::new();
        if self.match_token(&TokenKind::Colon) {
            loop {
                let bound_start = self.current_location();
                let trait_name = self.consume_identifier("Expected trait name")?;
                let bound_end = self.previous_location();
                
                bounds.push(TraitBound {
                    trait_name,
                    span: Span::new(bound_start, bound_end),
                });
                
                // Check for additional bounds with '+'
                if !self.match_token(&TokenKind::Plus) {
                    break;
                }
            }
        }
        
        let param_end = self.previous_location();
        params.push(GenericParam {
            name,
            bounds,
            span: Span::new(param_start, param_end),
        });
        
        // Check for more parameters
        if !self.match_token(&TokenKind::Comma) {
            break;
        }
    }
    
    // Consume closing '>'
    self.consume(&TokenKind::Greater, "Expected '>' after generic parameters")?;
    
    let end = self.previous_location();
    Ok(Some(GenericParams {
        params,
        span: Span::new(start, end),
    }))
}

/// Parse a where clause: where T: Clone, U: Default
fn parse_where_clause(&mut self) -> Result<Option<WhereClause>> {
    if !self.match_token(&TokenKind::Where) {
        return Ok(None);
    }
    
    let start = self.previous_location();
    let mut constraints = Vec::new();
    
    loop {
        let constraint_start = self.current_location();
        
        // Parse the type being constrained
        let type_ann = self.parse_type_annotation()?;
        
        // Expect ':' after type
        self.consume(&TokenKind::Colon, "Expected ':' in where clause")?;
        
        // Parse trait bounds
        let mut bounds = Vec::new();
        loop {
            let bound_start = self.current_location();
            let trait_name = self.consume_identifier("Expected trait name")?;
            let bound_end = self.previous_location();
            
            bounds.push(TraitBound {
                trait_name,
                span: Span::new(bound_start, bound_end),
            });
            
            if !self.match_token(&TokenKind::Plus) {
                break;
            }
        }
        
        let constraint_end = self.previous_location();
        constraints.push(WhereConstraint {
            type_: type_ann_to_type(&type_ann), // You'll need to import or implement this
            bounds,
            span: Span::new(constraint_start, constraint_end),
        });
        
        if !self.match_token(&TokenKind::Comma) {
            break;
        }
    }
    
    let end = self.previous_location();
    Ok(Some(WhereClause {
        constraints,
        span: Span::new(start, end),
    }))
}
```

## 2. Update Function Parsing

### Location: `src/parser/parser.rs` - `parse_function_common` method

Replace line 149:
```rust
// OLD:
generic_params: None,  // TODO: Parse generic parameters

// NEW:
generic_params: self.parse_generic_params()?,
```

Also update the method to parse where clauses after the return type:

```rust
fn parse_function_common(&mut self, is_async: bool) -> Result<StmtKind> {
    let name = self.consume_identifier("Expected function name")?;
    
    // Parse generic parameters
    let generic_params = self.parse_generic_params()?;
    
    self.consume(&TokenKind::LeftParen, "Expected '(' after function name")?;
    
    // ... existing parameter parsing code ...
    
    let ret_type = if self.match_token(&TokenKind::Arrow) {
        Some(self.parse_type_annotation()?)
    } else {
        None
    };
    
    // Parse optional where clause
    let where_clause = self.parse_where_clause()?;
    
    // Add where clause to generic params if present
    let generic_params = match (generic_params, where_clause) {
        (Some(mut params), Some(where_clause)) => {
            params.where_clause = Some(where_clause);
            Some(params)
        }
        (None, Some(where_clause)) => {
            Some(GenericParams {
                params: vec![],
                where_clause: Some(where_clause),
                span: where_clause.span,
            })
        }
        (params, None) => params,
    };
    
    self.consume(&TokenKind::LeftBrace, "Expected '{' before function body")?;
    let body = self.parse_block()?;
    
    Ok(StmtKind::Function {
        name,
        generic_params,
        params,
        ret_type,
        body,
        is_async,
    })
}
```

## 3. Update Type Parsing for Generic Types

### Location: `src/parser/parser.rs` - `parse_type_annotation` method

Update to handle generic type instantiations:

```rust
fn parse_type_annotation(&mut self) -> Result<TypeAnn> {
    let start = self.current_location();
    
    // Parse base type
    let base_type = if self.check(&TokenKind::LeftBracket) {
        // Array type
        self.advance();
        let elem_type = self.parse_type_annotation()?;
        self.consume(&TokenKind::RightBracket, "Expected ']' after array element type")?;
        
        let end = self.previous_location();
        return Ok(TypeAnn {
            kind: TypeKind::Array(Box::new(elem_type)),
            span: Span::new(start, end),
        });
    } else if self.check(&TokenKind::LeftParen) {
        // Function type
        return self.parse_function_type();
    } else {
        // Named type or type parameter
        self.consume_identifier("Expected type name")?
    };
    
    // Check for generic arguments
    if self.check(&TokenKind::Less) {
        // This is a generic type instantiation
        self.advance(); // consume '<'
        
        let mut args = Vec::new();
        loop {
            args.push(self.parse_type_annotation()?);
            
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        
        self.consume(&TokenKind::Greater, "Expected '>' after type arguments")?;
        
        let end = self.previous_location();
        Ok(TypeAnn {
            kind: TypeKind::Generic {
                name: base_type,
                args,
            },
            span: Span::new(start, end),
        })
    } else {
        // Simple named type or type parameter
        let end = self.previous_location();
        
        // Check if this looks like a type parameter (single uppercase letter or starts with uppercase)
        let is_type_param = base_type.len() == 1 && base_type.chars().next().unwrap().is_uppercase()
            || (base_type.len() > 1 && base_type.chars().next().unwrap().is_uppercase() 
                && !matches!(base_type.as_str(), "String" | "Vec" | "HashMap" | "Result" | "Option"));
        
        Ok(TypeAnn {
            kind: if is_type_param {
                TypeKind::TypeParam(base_type)
            } else {
                TypeKind::Named(base_type)
            },
            span: Span::new(start, end),
        })
    }
}
```

## 4. Add Missing AST Types

### Location: `src/parser/ast.rs`

Add the WhereClause and WhereConstraint types if not already present:

```rust
/// Where clause for expressing complex generic constraints
#[derive(Debug, Clone, PartialEq)]
pub struct WhereClause {
    /// List of constraints in the where clause
    pub constraints: Vec<WhereConstraint>,
    /// Source location
    pub span: Span,
}

/// A constraint in a where clause
#[derive(Debug, Clone, PartialEq)]
pub struct WhereConstraint {
    /// The type being constrained
    pub type_: Type,  // Note: You'll need to import Type from the types module
    /// The bounds that type must satisfy
    pub bounds: Vec<TraitBound>,
    /// Source location
    pub span: Span,
}
```

Also update GenericParams to include where clause:

```rust
/// Collection of generic parameters
#[derive(Debug, Clone, PartialEq)]
pub struct GenericParams {
    pub params: Vec<GenericParam>,
    pub where_clause: Option<WhereClause>,  // Add this field
    pub span: Span,
}
```

## 5. Required Token Updates

### Location: `src/lexer/token.rs`

Ensure these tokens exist:

```rust
pub enum TokenKind {
    // ... existing tokens ...
    Where,      // 'where' keyword
    Plus,       // '+' for multiple trait bounds
    // ... rest of tokens ...
}
```

### Location: `src/lexer/scanner.rs`

In the `get_keyword` method, add:

```rust
"where" => TokenKind::Where,
```

## 6. Example Usage

After these changes, the parser should be able to handle:

```script
// Simple generic function
fn identity<T>(x: T) -> T {
    x
}

// Generic with bounds
fn min<T: Ord>(a: T, b: T) -> T {
    if a < b { a } else { b }
}

// Multiple type parameters with bounds
fn map<T, U>(items: Vec<T>, f: (T) -> U) -> Vec<U> {
    // implementation
}

// Complex bounds
fn complex<T: Eq + Clone, U: Ord>(x: T, y: U) -> bool {
    // implementation
}

// Where clause
fn process<T, U>(x: T, y: U) -> Result<T, U>
where T: Clone, U: Display {
    // implementation
}
```

## Testing

Add these test cases to `src/parser/tests.rs`:

```rust
#[test]
fn test_parse_generic_function() {
    let input = "fn identity<T>(x: T) -> T { x }";
    let tokens = tokenize(input);
    let mut parser = Parser::new(tokens);
    let result = parser.parse().unwrap();
    
    // Verify the generic params were parsed
    match &result.statements[0].kind {
        StmtKind::Function { generic_params, .. } => {
            assert!(generic_params.is_some());
            let params = generic_params.as_ref().unwrap();
            assert_eq!(params.params.len(), 1);
            assert_eq!(params.params[0].name, "T");
        }
        _ => panic!("Expected function statement"),
    }
}

#[test]
fn test_parse_generic_with_bounds() {
    let input = "fn sort<T: Ord + Clone>(items: Vec<T>) -> Vec<T> { items }";
    let tokens = tokenize(input);
    let mut parser = Parser::new(tokens);
    let result = parser.parse().unwrap();
    
    // Verify bounds were parsed
    match &result.statements[0].kind {
        StmtKind::Function { generic_params, .. } => {
            let params = generic_params.as_ref().unwrap();
            assert_eq!(params.params[0].bounds.len(), 2);
            assert_eq!(params.params[0].bounds[0].trait_name, "Ord");
            assert_eq!(params.params[0].bounds[1].trait_name, "Clone");
        }
        _ => panic!("Expected function statement"),
    }
}
```

## Next Steps

After implementing these parser changes:

1. Update the semantic analyzer to track generic contexts
2. Modify type inference to handle type parameters
3. Implement constraint checking for trait bounds
4. Add monomorphization in the code generation phase