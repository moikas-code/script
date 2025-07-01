use std::collections::HashMap;
use crate::types::Type;

/// A substitution maps type variables to types
#[derive(Debug, Clone, PartialEq)]
pub struct Substitution {
    mapping: HashMap<u32, Type>,
}

impl Substitution {
    /// Create an empty substitution
    pub fn new() -> Self {
        Substitution {
            mapping: HashMap::new(),
        }
    }

    /// Create a substitution with a single mapping
    pub fn singleton(var_id: u32, ty: Type) -> Self {
        let mut mapping = HashMap::new();
        mapping.insert(var_id, ty);
        Substitution { mapping }
    }

    /// Add a mapping to the substitution
    pub fn insert(&mut self, var_id: u32, ty: Type) {
        self.mapping.insert(var_id, ty);
    }

    /// Look up a type variable in the substitution
    pub fn get(&self, var_id: u32) -> Option<&Type> {
        self.mapping.get(&var_id)
    }

    /// Compose this substitution with another
    /// The resulting substitution applies `other` first, then `self`
    pub fn compose(&mut self, other: Substitution) {
        // Apply self to all types in other
        let mut new_mapping = HashMap::new();
        for (var_id, ty) in other.mapping {
            new_mapping.insert(var_id, apply_substitution(self, &ty));
        }
        
        // Add mappings from self that aren't in other
        for (var_id, ty) in &self.mapping {
            if !new_mapping.contains_key(var_id) {
                new_mapping.insert(*var_id, ty.clone());
            }
        }
        
        self.mapping = new_mapping;
    }

    /// Check if the substitution is empty
    pub fn is_empty(&self) -> bool {
        self.mapping.is_empty()
    }

    /// Get the number of mappings
    pub fn len(&self) -> usize {
        self.mapping.len()
    }
}

impl Default for Substitution {
    fn default() -> Self {
        Self::new()
    }
}

/// Apply a substitution to a type
pub fn apply_substitution(subst: &Substitution, ty: &Type) -> Type {
    match ty {
        Type::TypeVar(id) => {
            // If there's a substitution for this variable, apply it recursively
            if let Some(replacement) = subst.get(*id) {
                apply_substitution(subst, replacement)
            } else {
                ty.clone()
            }
        }
        Type::Array(elem_ty) => {
            Type::Array(Box::new(apply_substitution(subst, elem_ty)))
        }
        Type::Function { params, ret } => {
            Type::Function {
                params: params.iter().map(|p| apply_substitution(subst, p)).collect(),
                ret: Box::new(apply_substitution(subst, ret)),
            }
        }
        Type::Result { ok, err } => {
            Type::Result {
                ok: Box::new(apply_substitution(subst, ok)),
                err: Box::new(apply_substitution(subst, err)),
            }
        }
        // Basic types and named types are not affected by substitution
        Type::I32 | Type::F32 | Type::Bool | Type::String | 
        Type::Unknown | Type::Named(_) => ty.clone(),
    }
}

/// Check if a type variable occurs in a type (occurs check for unification)
pub fn occurs_check(var_id: u32, ty: &Type) -> bool {
    match ty {
        Type::TypeVar(id) => *id == var_id,
        Type::Array(elem_ty) => occurs_check(var_id, elem_ty),
        Type::Function { params, ret } => {
            params.iter().any(|p| occurs_check(var_id, p)) || occurs_check(var_id, ret)
        }
        Type::Result { ok, err } => {
            occurs_check(var_id, ok) || occurs_check(var_id, err)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substitution_basic() {
        let mut subst = Substitution::new();
        assert!(subst.is_empty());
        
        subst.insert(0, Type::I32);
        subst.insert(1, Type::String);
        
        assert_eq!(subst.get(0), Some(&Type::I32));
        assert_eq!(subst.get(1), Some(&Type::String));
        assert_eq!(subst.get(2), None);
        assert_eq!(subst.len(), 2);
    }

    #[test]
    fn test_apply_substitution_basic() {
        let mut subst = Substitution::new();
        subst.insert(0, Type::I32);
        subst.insert(1, Type::Bool);
        
        // Apply to type variables
        assert_eq!(apply_substitution(&subst, &Type::TypeVar(0)), Type::I32);
        assert_eq!(apply_substitution(&subst, &Type::TypeVar(1)), Type::Bool);
        assert_eq!(apply_substitution(&subst, &Type::TypeVar(2)), Type::TypeVar(2));
        
        // Apply to basic types (no change)
        assert_eq!(apply_substitution(&subst, &Type::String), Type::String);
    }

    #[test]
    fn test_apply_substitution_complex() {
        let mut subst = Substitution::new();
        subst.insert(0, Type::I32);
        subst.insert(1, Type::String);
        
        // Apply to array type
        let array_ty = Type::Array(Box::new(Type::TypeVar(0)));
        assert_eq!(
            apply_substitution(&subst, &array_ty),
            Type::Array(Box::new(Type::I32))
        );
        
        // Apply to function type
        let fn_ty = Type::Function {
            params: vec![Type::TypeVar(0), Type::TypeVar(1)],
            ret: Box::new(Type::TypeVar(2)),
        };
        assert_eq!(
            apply_substitution(&subst, &fn_ty),
            Type::Function {
                params: vec![Type::I32, Type::String],
                ret: Box::new(Type::TypeVar(2)),
            }
        );
    }

    #[test]
    fn test_substitution_compose() {
        // s1: {0 -> I32, 1 -> Bool}
        let mut s1 = Substitution::new();
        s1.insert(0, Type::I32);
        s1.insert(1, Type::Bool);
        
        // s2: {2 -> T0, 3 -> String}
        let mut s2 = Substitution::new();
        s2.insert(2, Type::TypeVar(0));
        s2.insert(3, Type::String);
        
        // s1.compose(s2) should give {0 -> I32, 1 -> Bool, 2 -> I32, 3 -> String}
        s1.compose(s2);
        
        assert_eq!(s1.get(0), Some(&Type::I32));
        assert_eq!(s1.get(1), Some(&Type::Bool));
        assert_eq!(s1.get(2), Some(&Type::I32)); // T0 was substituted to I32
        assert_eq!(s1.get(3), Some(&Type::String));
    }

    #[test]
    fn test_occurs_check() {
        // Simple cases
        assert!(occurs_check(0, &Type::TypeVar(0)));
        assert!(!occurs_check(0, &Type::TypeVar(1)));
        assert!(!occurs_check(0, &Type::I32));
        
        // Array type
        let array_ty = Type::Array(Box::new(Type::TypeVar(0)));
        assert!(occurs_check(0, &array_ty));
        assert!(!occurs_check(1, &array_ty));
        
        // Function type
        let fn_ty = Type::Function {
            params: vec![Type::I32, Type::TypeVar(1)],
            ret: Box::new(Type::TypeVar(0)),
        };
        assert!(occurs_check(0, &fn_ty));
        assert!(occurs_check(1, &fn_ty));
        assert!(!occurs_check(2, &fn_ty));
    }
}