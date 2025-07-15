use super::{union_find::UnionFind, Constraint, ConstraintKind, TraitChecker, UnionFindStats};
use crate::error::{Error, ErrorKind};
use crate::source::Span;
use crate::types::{Type, TypeEnv};
use std::collections::HashMap;

/// Optimized inference context using union-find for constraint solving
/// Reduces unification complexity from O(n²) to O(n·α(n)) ≈ O(n)
#[derive(Debug, Clone)]
pub struct OptimizedInferenceContext {
    /// Union-find structure for efficient type unification
    union_find: UnionFind,
    /// Type environment for looking up variable types
    type_env: TypeEnv,
    /// Collected constraints to be solved
    constraints: Vec<Constraint>,
    /// Trait checker for validating trait implementations
    trait_checker: TraitChecker,
    /// Cache for resolved types to avoid repeated computation
    type_cache: HashMap<Type, Type>,
}

impl OptimizedInferenceContext {
    /// Create a new optimized inference context
    pub fn new() -> Self {
        OptimizedInferenceContext {
            union_find: UnionFind::new(),
            type_env: TypeEnv::new(),
            constraints: Vec::new(),
            trait_checker: TraitChecker::new(),
            type_cache: HashMap::new(),
        }
    }

    /// Generate a fresh type variable using union-find
    pub fn fresh_type_var(&mut self) -> Type {
        self.union_find.fresh_type_var()
    }

    /// Add a constraint to be solved
    pub fn add_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    /// Get the current type environment
    pub fn type_env(&self) -> &TypeEnv {
        &self.type_env
    }

    /// Get a mutable reference to the type environment
    pub fn type_env_mut(&mut self) -> &mut TypeEnv {
        &mut self.type_env
    }

    /// Apply union-find resolution to a type with caching
    pub fn apply_substitution(&mut self, ty: &Type) -> Type {
        // Check cache first
        if let Some(cached) = self.type_cache.get(ty) {
            return cached.clone();
        }

        let resolved = self.union_find.resolve_type(ty);

        // Cache the result for future lookups
        self.type_cache.insert(ty.clone(), resolved.clone());

        resolved
    }

    /// Solve all collected constraints using union-find algorithm
    /// This is the key optimization - O(n·α(n)) instead of O(n²)
    pub fn solve_constraints(&mut self) -> Result<(), Error> {
        // Clear cache at start of solving
        self.type_cache.clear();

        // Take ownership of constraints to avoid borrowing issues
        let constraints = std::mem::take(&mut self.constraints);

        for constraint in constraints {
            match &constraint.kind {
                ConstraintKind::Equality(t1, t2) => {
                    // Apply current resolution before unifying
                    let t1_resolved = self.union_find.resolve_type(t1);
                    let t2_resolved = self.union_find.resolve_type(t2);

                    // Unify using union-find (nearly O(α(n)) per operation)
                    if let Err(err) = self.union_find.unify_types(&t1_resolved, &t2_resolved) {
                        return Err(Error::new(
                            ErrorKind::TypeError,
                            format!("Type unification failed: {err}"),
                        )
                        .with_location(constraint.span.start));
                    }
                }
                ConstraintKind::TraitBound { type_, trait_name } => {
                    // Apply current resolution to the type
                    let concrete_type = self.union_find.resolve_type(type_);

                    // Check if the type implements the trait
                    if !self
                        .trait_checker
                        .implements_trait(&concrete_type, trait_name)
                    {
                        return Err(Error::new(
                            ErrorKind::TypeError,
                            format!(
                                "Type {} does not implement trait {}",
                                concrete_type, trait_name
                            ),
                        )
                        .with_location(constraint.span.start));
                    }
                }
                ConstraintKind::GenericBounds { type_param, bounds } => {
                    // Look up the concrete type for the type parameter
                    if let Some(param_type) = self.type_env.lookup(type_param) {
                        let concrete_type = self.union_find.resolve_type(param_type);

                        // Check each bound
                        for bound in bounds {
                            if !self.trait_checker.implements_trait(&concrete_type, bound) {
                                return Err(Error::new(
                                    ErrorKind::TypeError,
                                    format!(
                                        "Type parameter {} (resolved to {}) does not implement trait {}",
                                        type_param, concrete_type, bound
                                    ),
                                )
                                .with_location(constraint.span.start));
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Enter a new scope in the type environment
    pub fn push_scope(&mut self) {
        self.type_env.push_scope();
    }

    /// Exit the current scope in the type environment
    pub fn pop_scope(&mut self) {
        self.type_env.pop_scope();
    }

    /// Define a type parameter in the current scope
    pub fn define_type_param(&mut self, name: &str) {
        self.type_env
            .define(name.to_string(), Type::TypeParam(name.to_string()));
    }

    /// Get a reference to the trait checker
    pub fn trait_checker(&self) -> &TraitChecker {
        &self.trait_checker
    }

    /// Get a mutable reference to the trait checker
    pub fn trait_checker_mut(&mut self) -> &mut TraitChecker {
        &mut self.trait_checker
    }

    /// Check if a type implements a specific trait
    pub fn check_trait_implementation(&mut self, type_: &Type, trait_name: &str) -> bool {
        let resolved_type = self.union_find.resolve_type(type_);
        self.trait_checker
            .implements_trait(&resolved_type, trait_name)
    }

    /// Add a trait bound constraint
    pub fn add_trait_bound(&mut self, type_: Type, trait_name: String, span: Span) {
        self.add_constraint(Constraint::trait_bound(type_, trait_name, span));
    }

    /// Add generic bounds constraints
    pub fn add_generic_bounds(&mut self, type_param: String, bounds: Vec<String>, span: Span) {
        self.add_constraint(Constraint::generic_bounds(type_param, bounds, span));
    }

    /// Validate trait bounds for a type
    pub fn validate_trait_bounds(
        &mut self,
        type_: &Type,
        bounds: &[crate::types::generics::TraitBound],
    ) -> Result<(), Error> {
        let resolved_type = self.union_find.resolve_type(type_);
        let missing = self
            .trait_checker
            .validate_trait_bounds(&resolved_type, bounds);
        if !missing.is_empty() {
            let trait_names: Vec<_> = missing.iter().map(|m| m.trait_.name()).collect();
            return Err(Error::new(
                ErrorKind::TypeError,
                format!(
                    "Type {} does not implement required traits: {}",
                    resolved_type,
                    trait_names.join(", ")
                ),
            )
            .with_location(bounds[0].span.start));
        }
        Ok(())
    }

    /// Get union-find statistics for performance monitoring
    pub fn union_find_stats(&self) -> UnionFindStats {
        self.union_find.stats()
    }

    /// Clear the type resolution cache
    pub fn clear_cache(&mut self) {
        self.type_cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.type_cache.len(), self.type_cache.capacity())
    }

    /// Batch resolve multiple types efficiently
    pub fn batch_resolve_types(&mut self, types: &[Type]) -> Vec<Type> {
        types.iter().map(|ty| self.apply_substitution(ty)).collect()
    }
}

impl Default for OptimizedInferenceContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance comparison utilities
pub struct PerformanceComparison {
    pub original_constraints: usize,
    pub union_find_stats: UnionFindStats,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

impl OptimizedInferenceContext {
    /// Get performance metrics for analysis
    pub fn performance_metrics(&self) -> PerformanceComparison {
        PerformanceComparison {
            original_constraints: self.constraints.len(),
            union_find_stats: self.union_find.stats(),
            cache_hits: 0,   // Would need instrumentation to track
            cache_misses: 0, // Would need instrumentation to track
        }
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
    fn test_optimized_fresh_type_vars() {
        let mut ctx = OptimizedInferenceContext::new();

        let t1 = ctx.fresh_type_var();
        let t2 = ctx.fresh_type_var();
        let t3 = ctx.fresh_type_var();

        // Types should be distinct
        assert_ne!(t1, t2);
        assert_ne!(t2, t3);
        assert_ne!(t1, t3);
    }

    #[test]
    fn test_optimized_unification() {
        let mut ctx = OptimizedInferenceContext::new();
        let span = test_span();

        let var1 = ctx.fresh_type_var();
        let var2 = ctx.fresh_type_var();

        // Add constraint: var1 = var2
        ctx.add_constraint(Constraint::equality(var1.clone(), var2.clone(), span));

        // Add constraint: var1 = i32
        ctx.add_constraint(Constraint::equality(var1.clone(), Type::I32, span));

        // Solve constraints
        ctx.solve_constraints().unwrap();

        // Both variables should resolve to i32
        let resolved1 = ctx.apply_substitution(&var1);
        let resolved2 = ctx.apply_substitution(&var2);

        assert_eq!(resolved1, Type::I32);
        assert_eq!(resolved2, Type::I32);
    }

    #[test]
    fn test_complex_type_unification() {
        let mut ctx = OptimizedInferenceContext::new();
        let span = test_span();

        let var1 = ctx.fresh_type_var();
        let var2 = ctx.fresh_type_var();

        // Create function types: (T1) -> T2 and (i32) -> string
        let fn1 = Type::Function {
            params: vec![var1.clone()],
            ret: Box::new(var2.clone()),
        };

        let fn2 = Type::Function {
            params: vec![Type::I32],
            ret: Box::new(Type::String),
        };

        // Unify the function types
        ctx.add_constraint(Constraint::equality(fn1, fn2, span));
        ctx.solve_constraints().unwrap();

        // Variables should resolve correctly
        assert_eq!(ctx.apply_substitution(&var1), Type::I32);
        assert_eq!(ctx.apply_substitution(&var2), Type::String);
    }

    #[test]
    fn test_occurs_check() {
        let mut ctx = OptimizedInferenceContext::new();
        let span = test_span();

        let var = ctx.fresh_type_var();

        // Try to create infinite type: T = Array<T>
        let recursive_type = Type::Array(Box::new(var.clone()));
        ctx.add_constraint(Constraint::equality(var, recursive_type, span));

        // Should fail due to occurs check
        assert!(ctx.solve_constraints().is_err());
    }

    #[test]
    fn test_cache_effectiveness() {
        let mut ctx = OptimizedInferenceContext::new();

        let var = ctx.fresh_type_var();

        // First resolution should populate cache
        let resolved1 = ctx.apply_substitution(&var);

        // Second resolution should use cache
        let resolved2 = ctx.apply_substitution(&var);

        assert_eq!(resolved1, resolved2);
    }

    #[test]
    fn test_batch_resolution() {
        let mut ctx = OptimizedInferenceContext::new();
        let span = test_span();

        let var1 = ctx.fresh_type_var();
        let var2 = ctx.fresh_type_var();
        let var3 = ctx.fresh_type_var();

        // Unify all variables with i32
        ctx.add_constraint(Constraint::equality(var1.clone(), Type::I32, span));
        ctx.add_constraint(Constraint::equality(var2.clone(), Type::I32, span));
        ctx.add_constraint(Constraint::equality(var3.clone(), Type::I32, span));

        ctx.solve_constraints().unwrap();

        // Batch resolve
        let types = vec![var1, var2, var3];
        let resolved = ctx.batch_resolve_types(&types);

        assert_eq!(resolved, vec![Type::I32, Type::I32, Type::I32]);
    }

    #[test]
    fn test_performance_metrics() {
        let mut ctx = OptimizedInferenceContext::new();
        let span = test_span();

        // Add several constraints
        let var1 = ctx.fresh_type_var();
        let var2 = ctx.fresh_type_var();
        ctx.add_constraint(Constraint::equality(var1, var2, span));

        let metrics = ctx.performance_metrics();
        assert_eq!(metrics.original_constraints, 1);
        assert!(metrics.union_find_stats.total_variables >= 2);
    }

    #[test]
    fn test_trait_bound_optimization() {
        let mut ctx = OptimizedInferenceContext::new();
        let span = test_span();

        // Add trait bound that should succeed
        ctx.add_trait_bound(Type::I32, "Eq".to_string(), span);
        assert!(ctx.solve_constraints().is_ok());

        // Add trait bound that should fail
        let mut ctx2 = OptimizedInferenceContext::new();
        ctx2.add_trait_bound(Type::String, "Ord".to_string(), span);
        assert!(ctx2.solve_constraints().is_err());
    }
}
