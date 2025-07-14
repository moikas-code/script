use crate::types::{
    generics::{BuiltinTrait, MissingConstraint, TraitBound},
    Type,
};
use std::collections::HashMap;

/// Trait checker for validating trait implementations and constraints
#[derive(Debug, Clone)]
pub struct TraitChecker {
    /// Built-in trait implementations for primitive types
    builtin_impls: HashMap<(Type, BuiltinTrait), bool>,
    /// Cache for trait satisfaction checks
    trait_cache: HashMap<(Type, String), bool>,
    /// Trait dependency graph
    trait_dependencies: HashMap<String, Vec<String>>,
}

impl TraitChecker {
    /// Create a new trait checker
    pub fn new() -> Self {
        let mut checker = TraitChecker {
            builtin_impls: HashMap::new(),
            trait_cache: HashMap::new(),
            trait_dependencies: HashMap::new(),
        };

        checker.init_builtin_impls();
        checker.init_trait_dependencies();
        checker
    }

    /// Initialize built-in trait implementations for primitive types
    fn init_builtin_impls(&mut self) {
        let primitive_types = vec![Type::I32, Type::F32, Type::Bool, Type::String];

        for type_ in primitive_types {
            // All primitives implement basic traits
            self.builtin_impls
                .insert((type_.clone(), BuiltinTrait::Eq), true);
            self.builtin_impls
                .insert((type_.clone(), BuiltinTrait::Clone), true);
            self.builtin_impls
                .insert((type_.clone(), BuiltinTrait::Display), true);
            self.builtin_impls
                .insert((type_.clone(), BuiltinTrait::Debug), true);
            self.builtin_impls
                .insert((type_.clone(), BuiltinTrait::Default), true);
            self.builtin_impls
                .insert((type_.clone(), BuiltinTrait::Hash), true);

            // Numeric types implement Ord
            if matches!(type_, Type::I32 | Type::F32) {
                self.builtin_impls
                    .insert((type_.clone(), BuiltinTrait::Ord), true);
            }

            // Simple types implement Copy
            if matches!(type_, Type::I32 | Type::F32 | Type::Bool) {
                self.builtin_impls
                    .insert((type_.clone(), BuiltinTrait::Copy), true);
            }
        }
    }

    /// Initialize trait dependency relationships
    fn init_trait_dependencies(&mut self) {
        // Ord depends on Eq
        self.trait_dependencies
            .insert("Ord".to_string(), vec!["Eq".to_string()]);
        // Copy depends on Clone
        self.trait_dependencies
            .insert("Copy".to_string(), vec!["Clone".to_string()]);
    }

    /// Check if a type implements a trait
    pub fn implements_trait(&mut self, type_: &Type, trait_name: &str) -> bool {
        // Check cache first
        let cache_key = (type_.clone(), trait_name.to_string());
        if let Some(&cached) = self.trait_cache.get(&cache_key) {
            return cached;
        }

        let result = self.check_trait_implementation(type_, trait_name);

        // Cache the result
        self.trait_cache.insert(cache_key, result);
        result
    }

    /// Internal trait implementation checking
    fn check_trait_implementation(&mut self, type_: &Type, trait_name: &str) -> bool {
        // Check built-in trait
        if let Some(builtin_trait) = BuiltinTrait::from_name(trait_name) {
            return self.check_builtin_trait(type_, &builtin_trait);
        }

        // For non-builtin traits, check structural implementations
        self.check_structural_trait(type_, trait_name)
    }

    /// Check if a type implements a built-in trait
    fn check_builtin_trait(&mut self, type_: &Type, trait_: &BuiltinTrait) -> bool {
        // Check direct implementation
        if let Some(&implemented) = self.builtin_impls.get(&(type_.clone(), trait_.clone())) {
            return implemented;
        }

        // Check structural implementations
        match (type_, trait_) {
            // Arrays implement traits if their elements do
            (Type::Array(elem), _) => self.check_builtin_trait(elem, trait_),

            // Options implement traits if their inner type does
            (Type::Option(inner), _) => self.check_builtin_trait(inner, trait_),

            // Results implement traits if both Ok and Err types do
            (Type::Result { ok, err }, _) => {
                self.check_builtin_trait(ok, trait_) && self.check_builtin_trait(err, trait_)
            }

            // Functions don't implement most traits
            (Type::Function { .. }, BuiltinTrait::Eq) => false,
            (Type::Function { .. }, _) => false,

            // Type parameters don't implement traits directly
            (Type::TypeParam(_), _) => false,

            // Generic types need special handling
            (Type::Generic { .. }, _) => false,

            _ => false,
        }
    }

    /// Check structural trait implementations for non-builtin traits
    fn check_structural_trait(&mut self, type_: &Type, trait_name: &str) -> bool {
        match type_ {
            Type::Array(elem) => self.implements_trait(elem, trait_name),
            Type::Option(inner) => self.implements_trait(inner, trait_name),
            Type::Result { ok, err } => {
                self.implements_trait(ok, trait_name) && self.implements_trait(err, trait_name)
            }
            _ => false,
        }
    }

    /// Check if all trait dependencies are satisfied
    pub fn check_trait_dependencies(&mut self, type_: &Type, trait_name: &str) -> Vec<String> {
        let mut missing_deps = Vec::new();

        if let Some(deps) = self.trait_dependencies.get(trait_name) {
            let deps = deps.clone(); // Clone to avoid borrow checker issues
            for dep in &deps {
                if !self.implements_trait(type_, dep) {
                    missing_deps.push(dep.clone());
                }
            }
        }

        missing_deps
    }

    /// Validate multiple trait bounds for a type
    pub fn validate_trait_bounds(
        &mut self,
        type_: &Type,
        bounds: &[TraitBound],
    ) -> Vec<MissingConstraint> {
        let mut missing = Vec::new();

        for bound in bounds {
            if !self.implements_trait(type_, &bound.trait_name) {
                if let Some(builtin_trait) = BuiltinTrait::from_name(&bound.trait_name) {
                    missing.push(MissingConstraint {
                        type_: type_.clone(),
                        trait_: builtin_trait,
                        span: bound.span,
                    });
                }
            }
        }

        missing
    }

    /// Check trait conjunction (T: Trait1 + Trait2)
    pub fn check_trait_conjunction(&mut self, type_: &Type, trait_names: &[String]) -> Vec<String> {
        let mut missing = Vec::new();

        for trait_name in trait_names {
            if !self.implements_trait(type_, trait_name) {
                missing.push(trait_name.clone());
            }
        }

        missing
    }

    /// Get all traits implemented by a type
    pub fn get_implemented_traits(&mut self, type_: &Type) -> Vec<String> {
        let mut traits = Vec::new();

        // Check all built-in traits
        for builtin_trait in &[
            BuiltinTrait::Eq,
            BuiltinTrait::Ord,
            BuiltinTrait::Clone,
            BuiltinTrait::Display,
            BuiltinTrait::Debug,
            BuiltinTrait::Default,
            BuiltinTrait::Copy,
            BuiltinTrait::Hash,
        ] {
            if self.check_builtin_trait(type_, builtin_trait) {
                traits.push(builtin_trait.name().to_string());
            }
        }

        traits
    }

    /// Clear the trait cache (useful for testing)
    pub fn clear_cache(&mut self) {
        self.trait_cache.clear();
    }
}

impl Default for TraitChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{SourceLocation, Span};

    fn test_span() -> Span {
        Span::new(SourceLocation::new(1, 1, 0), SourceLocation::new(1, 10, 10))
    }

    #[test]
    fn test_primitive_trait_implementations() {
        let mut checker = TraitChecker::new();

        // Test basic trait implementations
        assert!(checker.implements_trait(&Type::I32, "Eq"));
        assert!(checker.implements_trait(&Type::I32, "Clone"));
        assert!(checker.implements_trait(&Type::I32, "Ord"));
        assert!(checker.implements_trait(&Type::I32, "Copy"));

        // Test string doesn't implement Ord
        assert!(!checker.implements_trait(&Type::String, "Ord"));

        // Test bool doesn't implement Ord
        assert!(!checker.implements_trait(&Type::Bool, "Ord"));
    }

    #[test]
    fn test_array_trait_implementations() {
        let mut checker = TraitChecker::new();

        let int_array = Type::Array(Box::new(Type::I32));
        assert!(checker.implements_trait(&int_array, "Eq"));
        assert!(checker.implements_trait(&int_array, "Clone"));
        assert!(checker.implements_trait(&int_array, "Ord"));

        let string_array = Type::Array(Box::new(Type::String));
        assert!(checker.implements_trait(&string_array, "Eq"));
        assert!(checker.implements_trait(&string_array, "Clone"));
        assert!(!checker.implements_trait(&string_array, "Ord"));
    }

    #[test]
    fn test_option_trait_implementations() {
        let mut checker = TraitChecker::new();

        let int_option = Type::Option(Box::new(Type::I32));
        assert!(checker.implements_trait(&int_option, "Eq"));
        assert!(checker.implements_trait(&int_option, "Clone"));
        assert!(checker.implements_trait(&int_option, "Ord"));
    }

    #[test]
    fn test_result_trait_implementations() {
        let mut checker = TraitChecker::new();

        let int_result = Type::Result {
            ok: Box::new(Type::I32),
            err: Box::new(Type::String),
        };
        assert!(checker.implements_trait(&int_result, "Eq"));
        assert!(checker.implements_trait(&int_result, "Clone"));
        assert!(!checker.implements_trait(&int_result, "Ord")); // String doesn't implement Ord
    }

    #[test]
    fn test_trait_dependencies() {
        let mut checker = TraitChecker::new();

        // Ord depends on Eq
        let deps = checker.check_trait_dependencies(&Type::I32, "Ord");
        assert!(deps.is_empty()); // I32 implements Eq, so no missing deps

        // Copy depends on Clone
        let deps = checker.check_trait_dependencies(&Type::I32, "Copy");
        assert!(deps.is_empty()); // I32 implements Clone, so no missing deps
    }

    #[test]
    fn test_trait_conjunction() {
        let mut checker = TraitChecker::new();

        let traits = vec!["Eq".to_string(), "Clone".to_string()];
        let missing = checker.check_trait_conjunction(&Type::I32, &traits);
        assert!(missing.is_empty());

        let traits = vec!["Eq".to_string(), "Ord".to_string()];
        let missing = checker.check_trait_conjunction(&Type::String, &traits);
        assert_eq!(missing, vec!["Ord"]);
    }

    #[test]
    fn test_trait_bounds_validation() {
        let mut checker = TraitChecker::new();

        let bounds = vec![
            TraitBound::new("Eq".to_string(), test_span()),
            TraitBound::new("Clone".to_string(), test_span()),
        ];

        let missing = checker.validate_trait_bounds(&Type::I32, &bounds);
        assert!(missing.is_empty());

        let bounds = vec![
            TraitBound::new("Eq".to_string(), test_span()),
            TraitBound::new("Ord".to_string(), test_span()),
        ];

        let missing = checker.validate_trait_bounds(&Type::String, &bounds);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].trait_, BuiltinTrait::Ord);
    }

    #[test]
    fn test_cache_functionality() {
        let mut checker = TraitChecker::new();

        // First check should compute result
        assert!(checker.implements_trait(&Type::I32, "Eq"));

        // Clear cache and check again
        checker.clear_cache();
        assert!(checker.implements_trait(&Type::I32, "Eq"));
    }
}
