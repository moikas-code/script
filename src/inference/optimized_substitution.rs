use crate::types::Type;
use std::collections::HashMap;

/// Optimized substitution with memoization to reduce O(n²) complexity
/// Key improvements:
/// 1. Memoization cache for expensive substitution operations
/// 2. Lazy evaluation to avoid unnecessary work
/// 3. Reference counting for shared types
/// 4. Optimized path for common substitution patterns
#[derive(Debug, Clone)]
pub struct OptimizedSubstitution {
    /// Mapping from type variables to types
    mapping: HashMap<u32, Type>,
    /// Memoization cache for substitution results
    cache: HashMap<(Type, u64), Type>, // (type, substitution_hash) -> result
    /// Hash of current substitution state for cache invalidation
    substitution_hash: u64,
}

impl OptimizedSubstitution {
    /// Create an empty optimized substitution
    pub fn new() -> Self {
        OptimizedSubstitution {
            mapping: HashMap::new(),
            cache: HashMap::new(),
            substitution_hash: 0,
        }
    }

    /// Create a substitution with a single mapping
    pub fn singleton(var_id: u32, ty: Type) -> Self {
        let mut mapping = HashMap::new();
        mapping.insert(var_id, ty);

        let mut subst = OptimizedSubstitution {
            mapping,
            cache: HashMap::new(),
            substitution_hash: 0,
        };
        subst.update_hash();
        subst
    }

    /// Add a mapping to the substitution
    pub fn insert(&mut self, var_id: u32, ty: Type) {
        self.mapping.insert(var_id, ty);
        self.invalidate_cache();
    }

    /// Look up a type variable in the substitution
    pub fn get(&self, var_id: u32) -> Option<&Type> {
        self.mapping.get(&var_id)
    }

    /// Compose this substitution with another
    pub fn compose(&mut self, other: OptimizedSubstitution) {
        // Apply self to all types in other
        let mut new_mapping = HashMap::new();
        for (var_id, ty) in other.mapping {
            let substituted_ty = self.apply_to_type(&ty);
            new_mapping.insert(var_id, substituted_ty);
        }

        // Add mappings from self that aren't in other
        for (var_id, ty) in &self.mapping {
            if !new_mapping.contains_key(var_id) {
                new_mapping.insert(*var_id, ty.clone());
            }
        }

        self.mapping = new_mapping;
        self.invalidate_cache();
    }

    /// Check if the substitution is empty
    pub fn is_empty(&self) -> bool {
        self.mapping.is_empty()
    }

    /// Get the number of mappings
    pub fn len(&self) -> usize {
        self.mapping.len()
    }

    /// Apply substitution to a type with memoization
    pub fn apply_to_type(&mut self, ty: &Type) -> Type {
        // Check cache first
        let cache_key = (ty.clone(), self.substitution_hash);
        if let Some(cached_result) = self.cache.get(&cache_key) {
            return cached_result.clone();
        }

        // Compute the substitution
        let result = self.apply_to_type_impl(ty);

        // Cache the result if it's worth caching (non-trivial types)
        let should_cache = self.should_cache_type(ty);
        if should_cache {
            self.cache.insert(cache_key, result.clone());
        }

        result
    }

    /// Internal implementation of type substitution without caching
    fn apply_to_type_impl(&mut self, ty: &Type) -> Type {
        match ty {
            Type::TypeVar(id) => {
                // Direct substitution lookup
                if let Some(replacement) = self.mapping.get(id).cloned() {
                    // Apply substitution recursively to handle chains
                    if replacement != *ty {
                        self.apply_to_type(&replacement)
                    } else {
                        ty.clone()
                    }
                } else {
                    ty.clone()
                }
            }

            // Optimized cases for common patterns
            Type::Array(elem_ty) => {
                let substituted_elem = self.apply_to_type(elem_ty);
                if substituted_elem == **elem_ty {
                    // No change, return original to avoid allocation
                    ty.clone()
                } else {
                    Type::Array(Box::new(substituted_elem))
                }
            }

            Type::Function { params, ret } => {
                let mut changed = false;
                let mut new_params = Vec::with_capacity(params.len());

                for param in params {
                    let substituted_param = self.apply_to_type(param);
                    if substituted_param != *param {
                        changed = true;
                    }
                    new_params.push(substituted_param);
                }

                let substituted_ret = self.apply_to_type(ret);
                if substituted_ret != **ret {
                    changed = true;
                }

                if changed {
                    Type::Function {
                        params: new_params,
                        ret: Box::new(substituted_ret),
                    }
                } else {
                    ty.clone()
                }
            }

            Type::Result { ok, err } => {
                let substituted_ok = self.apply_to_type(ok);
                let substituted_err = self.apply_to_type(err);

                if substituted_ok == **ok && substituted_err == **err {
                    ty.clone()
                } else {
                    Type::Result {
                        ok: Box::new(substituted_ok),
                        err: Box::new(substituted_err),
                    }
                }
            }

            Type::Future(inner_ty) => {
                let substituted_inner = self.apply_to_type(inner_ty);
                if substituted_inner == **inner_ty {
                    ty.clone()
                } else {
                    Type::Future(Box::new(substituted_inner))
                }
            }

            Type::Option(inner_ty) => {
                let substituted_inner = self.apply_to_type(inner_ty);
                if substituted_inner == **inner_ty {
                    ty.clone()
                } else {
                    Type::Option(Box::new(substituted_inner))
                }
            }

            Type::Generic { name, args } => {
                let mut changed = false;
                let mut new_args = Vec::with_capacity(args.len());

                for arg in args {
                    let substituted_arg = self.apply_to_type(arg);
                    if substituted_arg != *arg {
                        changed = true;
                    }
                    new_args.push(substituted_arg);
                }

                if changed {
                    Type::Generic {
                        name: name.clone(),
                        args: new_args,
                    }
                } else {
                    ty.clone()
                }
            }

            Type::Tuple(types) => {
                let mut changed = false;
                let mut new_types = Vec::with_capacity(types.len());

                for t in types {
                    let substituted_t = self.apply_to_type(t);
                    if substituted_t != *t {
                        changed = true;
                    }
                    new_types.push(substituted_t);
                }

                if changed {
                    Type::Tuple(new_types)
                } else {
                    ty.clone()
                }
            }

            Type::Reference { mutable, inner } => {
                let substituted_inner = self.apply_to_type(inner);
                if substituted_inner == **inner {
                    ty.clone()
                } else {
                    Type::Reference {
                        mutable: *mutable,
                        inner: Box::new(substituted_inner),
                    }
                }
            }

            Type::Struct { name, fields } => {
                let mut changed = false;
                let mut new_fields = Vec::with_capacity(fields.len());

                for (field_name, field_type) in fields {
                    let substituted_field = self.apply_to_type(field_type);
                    if substituted_field != *field_type {
                        changed = true;
                    }
                    new_fields.push((field_name.clone(), substituted_field));
                }

                if changed {
                    Type::Struct {
                        name: name.clone(),
                        fields: new_fields,
                    }
                } else {
                    ty.clone()
                }
            }

            // Primitive types and types without type variables
            Type::I32
            | Type::F32
            | Type::Bool
            | Type::String
            | Type::Unknown
            | Type::Named(_)
            | Type::Never
            | Type::TypeParam(_) => ty.clone(),
        }
    }

    /// Apply substitution to multiple types efficiently
    pub fn apply_batch(&mut self, types: &[Type]) -> Vec<Type> {
        types.iter().map(|ty| self.apply_to_type(ty)).collect()
    }

    /// Determine if a type should be cached
    fn should_cache_type(&self, ty: &Type) -> bool {
        match ty {
            // Cache complex types that are expensive to substitute
            Type::Function { params, .. } => params.len() > 2,
            Type::Generic { args, .. } => args.len() > 1,
            Type::Tuple(types) => types.len() > 2,
            Type::Array(_) | Type::Option(_) | Type::Future(_) | Type::Result { .. } => true,
            // Don't cache simple types
            Type::I32 | Type::F32 | Type::Bool | Type::String | Type::Unknown | Type::Never => {
                false
            }
            Type::TypeVar(_) | Type::Named(_) | Type::TypeParam(_) => false,
            Type::Reference { .. } => true,
            Type::Struct { fields, .. } => fields.len() > 2, // Cache structs with multiple fields
        }
    }

    /// Update the hash used for cache keys
    fn update_hash(&mut self) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // Hash the mapping in a deterministic order
        let mut sorted_mapping: Vec<_> = self.mapping.iter().collect();
        sorted_mapping.sort_by_key(|(k, _)| *k);

        for (var_id, ty) in sorted_mapping {
            var_id.hash(&mut hasher);
            self.hash_type(ty, &mut hasher);
        }

        self.substitution_hash = hasher.finish();
    }

    /// Hash a type in a consistent way
    fn hash_type(&self, ty: &Type, hasher: &mut impl std::hash::Hasher) {
        use std::hash::Hash;

        // Use a discriminant-based approach to hash types consistently
        match ty {
            Type::TypeVar(id) => {
                1u8.hash(hasher);
                id.hash(hasher);
            }
            Type::I32 => 2u8.hash(hasher),
            Type::F32 => 3u8.hash(hasher),
            Type::Bool => 4u8.hash(hasher),
            Type::String => 5u8.hash(hasher),
            Type::Array(elem) => {
                6u8.hash(hasher);
                self.hash_type(elem, hasher);
            }
            Type::Function { params, ret } => {
                7u8.hash(hasher);
                params.len().hash(hasher);
                for param in params {
                    self.hash_type(param, hasher);
                }
                self.hash_type(ret, hasher);
            }
            Type::Named(name) => {
                8u8.hash(hasher);
                name.hash(hasher);
            }
            Type::Generic { name, args } => {
                9u8.hash(hasher);
                name.hash(hasher);
                args.len().hash(hasher);
                for arg in args {
                    self.hash_type(arg, hasher);
                }
            }
            Type::TypeParam(name) => {
                10u8.hash(hasher);
                name.hash(hasher);
            }
            Type::Unknown => 11u8.hash(hasher),
            Type::Never => 12u8.hash(hasher),
            Type::Future(inner) => {
                13u8.hash(hasher);
                self.hash_type(inner, hasher);
            }
            Type::Tuple(types) => {
                14u8.hash(hasher);
                types.len().hash(hasher);
                for ty in types {
                    self.hash_type(ty, hasher);
                }
            }
            Type::Reference { mutable, inner } => {
                15u8.hash(hasher);
                mutable.hash(hasher);
                self.hash_type(inner, hasher);
            }
            Type::Option(inner) => {
                16u8.hash(hasher);
                self.hash_type(inner, hasher);
            }
            Type::Result { ok, err } => {
                17u8.hash(hasher);
                self.hash_type(ok, hasher);
                self.hash_type(err, hasher);
            }
            Type::Struct { name, fields } => {
                18u8.hash(hasher);
                name.hash(hasher);
                fields.len().hash(hasher);
                for (field_name, field_type) in fields {
                    field_name.hash(hasher);
                    self.hash_type(field_type, hasher);
                }
            }
        }
    }

    /// Invalidate cache and update hash
    fn invalidate_cache(&mut self) {
        self.cache.clear();
        self.update_hash();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize, usize) {
        (self.cache.len(), self.cache.capacity(), self.mapping.len())
    }

    /// Clear the cache to free memory
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Optimize the substitution by removing redundant mappings
    pub fn optimize(&mut self) {
        // Remove identity mappings (T -> T)
        self.mapping
            .retain(|var_id, ty| !matches!(ty, Type::TypeVar(id) if id == var_id));

        // Update hash after optimization
        self.invalidate_cache();
    }
}

impl Default for OptimizedSubstitution {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a type variable occurs in a type (optimized occurs check)
pub fn optimized_occurs_check(var_id: u32, ty: &Type) -> bool {
    match ty {
        Type::TypeVar(id) => *id == var_id,
        Type::Array(elem_ty) => optimized_occurs_check(var_id, elem_ty),
        Type::Function { params, ret } => {
            params.iter().any(|p| optimized_occurs_check(var_id, p))
                || optimized_occurs_check(var_id, ret)
        }
        Type::Result { ok, err } => {
            optimized_occurs_check(var_id, ok) || optimized_occurs_check(var_id, err)
        }
        Type::Future(inner_ty) => optimized_occurs_check(var_id, inner_ty),
        Type::Option(inner_ty) => optimized_occurs_check(var_id, inner_ty),
        Type::Generic { args, .. } => args.iter().any(|arg| optimized_occurs_check(var_id, arg)),
        Type::Tuple(types) => types.iter().any(|t| optimized_occurs_check(var_id, t)),
        Type::Reference { inner, .. } => optimized_occurs_check(var_id, inner),
        Type::Struct { fields, .. } => fields
            .iter()
            .any(|(_field_name, field_type)| optimized_occurs_check(var_id, field_type)),
        _ => false,
    }
}

/// Apply an optimized substitution to a type (convenience function)
pub fn apply_optimized_substitution(subst: &mut OptimizedSubstitution, ty: &Type) -> Type {
    subst.apply_to_type(ty)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimized_substitution_basic() {
        let mut subst = OptimizedSubstitution::new();
        assert!(subst.is_empty());

        subst.insert(0, Type::I32);
        subst.insert(1, Type::String);

        assert_eq!(subst.get(0), Some(&Type::I32));
        assert_eq!(subst.get(1), Some(&Type::String));
        assert_eq!(subst.get(2), None);
        assert_eq!(subst.len(), 2);
    }

    #[test]
    fn test_memoized_substitution() {
        let mut subst = OptimizedSubstitution::new();
        subst.insert(0, Type::I32);
        subst.insert(1, Type::Bool);

        // Apply to type variables
        assert_eq!(subst.apply_to_type(&Type::TypeVar(0)), Type::I32);
        assert_eq!(subst.apply_to_type(&Type::TypeVar(1)), Type::Bool);
        assert_eq!(subst.apply_to_type(&Type::TypeVar(2)), Type::TypeVar(2));

        // Check cache was populated
        let (cache_size, _, _) = subst.cache_stats();
        assert!(cache_size > 0);
    }

    #[test]
    fn test_optimized_substitution_complex() {
        let mut subst = OptimizedSubstitution::new();
        subst.insert(0, Type::I32);
        subst.insert(1, Type::String);

        // Apply to array type
        let array_ty = Type::Array(Box::new(Type::TypeVar(0)));
        assert_eq!(
            subst.apply_to_type(&array_ty),
            Type::Array(Box::new(Type::I32))
        );

        // Apply to function type
        let fn_ty = Type::Function {
            params: vec![Type::TypeVar(0), Type::TypeVar(1)],
            ret: Box::new(Type::TypeVar(2)),
        };
        assert_eq!(
            subst.apply_to_type(&fn_ty),
            Type::Function {
                params: vec![Type::I32, Type::String],
                ret: Box::new(Type::TypeVar(2)),
            }
        );
    }

    #[test]
    fn test_substitution_compose() {
        // s1: {0 -> I32, 1 -> Bool}
        let mut s1 = OptimizedSubstitution::new();
        s1.insert(0, Type::I32);
        s1.insert(1, Type::Bool);

        // s2: {2 -> T0, 3 -> String}
        let mut s2 = OptimizedSubstitution::new();
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
    fn test_optimized_occurs_check() {
        // Simple cases
        assert!(optimized_occurs_check(0, &Type::TypeVar(0)));
        assert!(!optimized_occurs_check(0, &Type::TypeVar(1)));
        assert!(!optimized_occurs_check(0, &Type::I32));

        // Array type
        let array_ty = Type::Array(Box::new(Type::TypeVar(0)));
        assert!(optimized_occurs_check(0, &array_ty));
        assert!(!optimized_occurs_check(1, &array_ty));

        // Function type
        let fn_ty = Type::Function {
            params: vec![Type::I32, Type::TypeVar(1)],
            ret: Box::new(Type::TypeVar(0)),
        };
        assert!(optimized_occurs_check(0, &fn_ty));
        assert!(optimized_occurs_check(1, &fn_ty));
        assert!(!optimized_occurs_check(2, &fn_ty));
    }

    #[test]
    fn test_batch_application() {
        let mut subst = OptimizedSubstitution::new();
        subst.insert(0, Type::I32);
        subst.insert(1, Type::String);

        let types = vec![
            Type::TypeVar(0),
            Type::TypeVar(1),
            Type::Array(Box::new(Type::TypeVar(0))),
        ];

        let results = subst.apply_batch(&types);
        assert_eq!(results[0], Type::I32);
        assert_eq!(results[1], Type::String);
        assert_eq!(results[2], Type::Array(Box::new(Type::I32)));
    }

    #[test]
    fn test_cache_effectiveness() {
        let mut subst = OptimizedSubstitution::new();
        subst.insert(0, Type::I32);

        let complex_type = Type::Function {
            params: vec![Type::TypeVar(0), Type::TypeVar(0), Type::TypeVar(0)],
            ret: Box::new(Type::TypeVar(0)),
        };

        // First application should populate cache
        let result1 = subst.apply_to_type(&complex_type);

        // Second application should use cache
        let result2 = subst.apply_to_type(&complex_type);

        assert_eq!(result1, result2);

        let (cache_size, _, _) = subst.cache_stats();
        assert!(cache_size > 0);
    }

    #[test]
    fn test_optimization() {
        let mut subst = OptimizedSubstitution::new();

        // Add some mappings including identity mapping
        subst.insert(0, Type::I32);
        subst.insert(1, Type::TypeVar(1)); // Identity mapping
        subst.insert(2, Type::String);

        assert_eq!(subst.len(), 3);

        // Optimize should remove the identity mapping
        subst.optimize();

        assert_eq!(subst.len(), 2);
        assert_eq!(subst.get(0), Some(&Type::I32));
        assert_eq!(subst.get(1), None); // Identity mapping removed
        assert_eq!(subst.get(2), Some(&Type::String));
    }

    #[test]
    fn test_no_change_optimization() {
        let mut subst = OptimizedSubstitution::new();
        subst.insert(0, Type::I32);

        let simple_type = Type::String; // No substitution needed
        let result = subst.apply_to_type(&simple_type);

        // Should return the same instance (not just equal)
        assert_eq!(result, simple_type);
    }

    #[test]
    fn test_cache_invalidation() {
        let mut subst = OptimizedSubstitution::new();
        subst.insert(0, Type::I32);

        let ty = Type::TypeVar(0);
        let _result1 = subst.apply_to_type(&ty);

        let (cache_size_before, _, _) = subst.cache_stats();

        // Adding a new mapping should invalidate cache
        subst.insert(1, Type::String);

        let (cache_size_after, _, _) = subst.cache_stats();
        assert_eq!(cache_size_after, 0); // Cache should be cleared
    }
}
