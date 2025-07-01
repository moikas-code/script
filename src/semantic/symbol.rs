use crate::source::Span;
use crate::types::Type;
use std::fmt;

/// A unique identifier for a symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub usize);

/// Represents a symbol in the symbol table
#[derive(Debug, Clone)]
pub struct Symbol {
    /// Unique identifier for this symbol
    pub id: SymbolId,
    /// Name of the symbol
    pub name: String,
    /// Kind of symbol (variable, function, etc.)
    pub kind: SymbolKind,
    /// Type of the symbol
    pub ty: Type,
    /// Location where the symbol was defined
    pub def_span: Span,
    /// Whether this symbol is mutable (for variables)
    pub is_mutable: bool,
    /// Whether this symbol has been used
    pub is_used: bool,
    /// Scope where this symbol was defined
    pub scope_id: super::ScopeId,
}

/// Different kinds of symbols
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    /// Local variable
    Variable,
    /// Function
    Function(FunctionSignature),
    /// Parameter
    Parameter,
    /// Built-in function
    BuiltIn,
    /// Actor (for future use)
    Actor,
    /// Constant (for future use)
    Constant,
}

/// Function signature information
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSignature {
    /// Parameter names and types
    pub params: Vec<(String, Type)>,
    /// Return type
    pub return_type: Type,
    /// Whether this function is marked as @const
    pub is_const: bool,
    /// Whether this function is async (for actors)
    pub is_async: bool,
}

impl Symbol {
    /// Create a new variable symbol
    pub fn variable(
        id: SymbolId,
        name: String,
        ty: Type,
        def_span: Span,
        is_mutable: bool,
        scope_id: super::ScopeId,
    ) -> Self {
        Symbol {
            id,
            name,
            kind: SymbolKind::Variable,
            ty,
            def_span,
            is_mutable,
            is_used: false,
            scope_id,
        }
    }

    /// Create a new function symbol
    pub fn function(
        id: SymbolId,
        name: String,
        signature: FunctionSignature,
        def_span: Span,
        scope_id: super::ScopeId,
    ) -> Self {
        let ty = Type::Function {
            params: signature.params.iter().map(|(_, ty)| ty.clone()).collect(),
            ret: Box::new(signature.return_type.clone()),
        };

        Symbol {
            id,
            name,
            kind: SymbolKind::Function(signature),
            ty,
            def_span,
            is_mutable: false,
            is_used: false,
            scope_id,
        }
    }

    /// Create a new parameter symbol
    pub fn parameter(
        id: SymbolId,
        name: String,
        ty: Type,
        def_span: Span,
        scope_id: super::ScopeId,
    ) -> Self {
        Symbol {
            id,
            name,
            kind: SymbolKind::Parameter,
            ty,
            def_span,
            is_mutable: false,
            is_used: false,
            scope_id,
        }
    }

    /// Mark this symbol as used
    pub fn mark_used(&mut self) {
        self.is_used = true;
    }

    /// Check if this symbol is a function
    pub fn is_function(&self) -> bool {
        matches!(self.kind, SymbolKind::Function(_))
    }

    /// Get the function signature if this is a function symbol
    pub fn function_signature(&self) -> Option<&FunctionSignature> {
        match &self.kind {
            SymbolKind::Function(sig) => Some(sig),
            _ => None,
        }
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} : {}", self.kind, self.name, self.ty)
    }
}

impl fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SymbolKind::Variable => write!(f, "variable"),
            SymbolKind::Function(_) => write!(f, "function"),
            SymbolKind::Parameter => write!(f, "parameter"),
            SymbolKind::BuiltIn => write!(f, "builtin"),
            SymbolKind::Actor => write!(f, "actor"),
            SymbolKind::Constant => write!(f, "const"),
        }
    }
}

impl FunctionSignature {
    /// Check if two function signatures are compatible for overloading
    /// For now, we only check parameter count and types
    pub fn is_compatible_for_overload(&self, other: &FunctionSignature) -> bool {
        // Different parameter counts allow overloading
        if self.params.len() != other.params.len() {
            return true;
        }

        // Same parameter count - check if types differ
        for (i, (_, ty1)) in self.params.iter().enumerate() {
            if let Some((_, ty2)) = other.params.get(i) {
                if !ty1.equals(ty2) {
                    return true;
                }
            }
        }

        // Same signature - not allowed
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{SourceLocation, Span};

    fn make_span() -> Span {
        Span::new(
            SourceLocation::new(1, 1, 0),
            SourceLocation::new(1, 5, 5),
        )
    }

    #[test]
    fn test_symbol_creation() {
        let var_symbol = Symbol::variable(
            SymbolId(1),
            "x".to_string(),
            Type::I32,
            make_span(),
            true,
            super::super::ScopeId(0),
        );

        assert_eq!(var_symbol.name, "x");
        assert_eq!(var_symbol.ty, Type::I32);
        assert!(var_symbol.is_mutable);
        assert!(!var_symbol.is_used);
        assert_eq!(var_symbol.kind, SymbolKind::Variable);
    }

    #[test]
    fn test_function_symbol() {
        let sig = FunctionSignature {
            params: vec![("x".to_string(), Type::I32), ("y".to_string(), Type::I32)],
            return_type: Type::I32,
            is_const: false,
            is_async: false,
        };

        let func_symbol = Symbol::function(
            SymbolId(2),
            "add".to_string(),
            sig.clone(),
            make_span(),
            super::super::ScopeId(0),
        );

        assert!(func_symbol.is_function());
        assert_eq!(func_symbol.function_signature(), Some(&sig));
        
        let expected_type = Type::Function {
            params: vec![Type::I32, Type::I32],
            ret: Box::new(Type::I32),
        };
        assert_eq!(func_symbol.ty, expected_type);
    }

    #[test]
    fn test_function_overload_compatibility() {
        // Different parameter count - compatible
        let sig1 = FunctionSignature {
            params: vec![("x".to_string(), Type::I32)],
            return_type: Type::I32,
            is_const: false,
            is_async: false,
        };

        let sig2 = FunctionSignature {
            params: vec![("x".to_string(), Type::I32), ("y".to_string(), Type::I32)],
            return_type: Type::I32,
            is_const: false,
            is_async: false,
        };

        assert!(sig1.is_compatible_for_overload(&sig2));
        assert!(sig2.is_compatible_for_overload(&sig1));

        // Different parameter types - compatible
        let sig3 = FunctionSignature {
            params: vec![("x".to_string(), Type::F32)],
            return_type: Type::I32,
            is_const: false,
            is_async: false,
        };

        assert!(sig1.is_compatible_for_overload(&sig3));

        // Same signature - not compatible
        let sig4 = FunctionSignature {
            params: vec![("y".to_string(), Type::I32)],
            return_type: Type::F32, // Different return type doesn't matter
            is_const: true, // Different const doesn't matter
            is_async: false,
        };

        assert!(!sig1.is_compatible_for_overload(&sig4));
    }
}