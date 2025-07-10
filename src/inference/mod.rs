use crate::error::Error;
use crate::parser::{TypeAnn, TypeKind};
use crate::types::{Type, TypeEnv};

mod constraint;
mod constructor_inference;
mod inference_engine;
mod optimized_inference_context;
mod optimized_substitution;
mod substitution;
mod trait_checker;
mod unification;
mod union_find;

pub use constraint::{Constraint, ConstraintKind};
pub use constructor_inference::{ConstructorInferenceEngine, ConstructorInferenceResult};
pub use inference_engine::{InferenceEngine, InferenceResult};
pub use optimized_inference_context::OptimizedInferenceContext;
pub use optimized_substitution::{
    apply_optimized_substitution, optimized_occurs_check, OptimizedSubstitution,
};
pub use substitution::{apply_substitution, Substitution};
pub use trait_checker::TraitChecker;
pub use unification::{unify, unify_optimized};
pub use union_find::{UnionFind, UnionFindStats};

/// Type inference context that manages type variables and constraints
#[derive(Debug, Clone)]
pub struct InferenceContext {
    /// Next available type variable ID
    next_type_var: u32,
    /// Current substitution mapping type variables to types
    substitution: OptimizedSubstitution,
    /// Type environment for looking up variable types
    type_env: TypeEnv,
    /// Collected constraints to be solved
    constraints: Vec<Constraint>,
    /// Trait checker for validating trait implementations
    trait_checker: TraitChecker,
}

impl InferenceContext {
    /// Create a new inference context
    pub fn new() -> Self {
        InferenceContext {
            next_type_var: 0,
            substitution: OptimizedSubstitution::new(),
            type_env: TypeEnv::new(),
            constraints: Vec::new(),
            trait_checker: TraitChecker::new(),
        }
    }

    /// Generate a fresh type variable
    pub fn fresh_type_var(&mut self) -> Type {
        let id = self.next_type_var;
        self.next_type_var += 1;
        Type::TypeVar(id)
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

    /// Apply the current substitution to a type
    pub fn apply_substitution(&mut self, ty: &Type) -> Type {
        self.substitution.apply_to_type(ty)
    }

    /// Solve all collected constraints and update the substitution
    pub fn solve_constraints(&mut self) -> Result<(), Error> {
        // Take ownership of constraints to avoid borrowing issues
        let constraints = std::mem::take(&mut self.constraints);

        for constraint in constraints {
            match &constraint.kind {
                ConstraintKind::Equality(t1, t2) => {
                    // Apply current substitution before unifying
                    let t1_subst = self.apply_substitution(t1);
                    let t2_subst = self.apply_substitution(t2);

                    // Unify and get new substitution
                    let new_subst = unify_optimized(&t1_subst, &t2_subst, constraint.span)?;

                    // Compose with existing substitution
                    self.substitution.compose(new_subst);
                }
                ConstraintKind::TraitBound { type_, trait_name } => {
                    // Apply current substitution to the type
                    let concrete_type = self.apply_substitution(type_);

                    // Check if the type implements the trait
                    if !self
                        .trait_checker
                        .implements_trait(&concrete_type, trait_name)
                    {
                        return Err(Error::new(
                            crate::error::ErrorKind::TypeError,
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
                    if let Some(concrete_type) = self.type_env.lookup(type_param).cloned() {
                        let concrete_type = self.apply_substitution(&concrete_type);

                        // Check each bound
                        for bound in bounds {
                            if !self.trait_checker.implements_trait(&concrete_type, bound) {
                                return Err(Error::new(
                                    crate::error::ErrorKind::TypeError,
                                    format!("Type parameter {} (resolved to {}) does not implement trait {
                                           type_param, concrete_type, bound}"),
                                ).with_location(constraint.span.start));
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
        // For now, type parameters are represented as TypeParam types
        // In the future, we'll track bounds and constraints here
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
        self.trait_checker.implements_trait(type_, trait_name)
    }

    /// Add a trait bound constraint
    pub fn add_trait_bound(&mut self, type_: Type, trait_name: String, span: crate::source::Span) {
        self.add_constraint(Constraint::trait_bound(type_, trait_name, span));
    }

    /// Add generic bounds constraints
    pub fn add_generic_bounds(
        &mut self,
        type_param: String,
        bounds: Vec<String>,
        span: crate::source::Span,
    ) {
        self.add_constraint(Constraint::generic_bounds(type_param, bounds, span));
    }

    /// Validate trait bounds for a type
    pub fn validate_trait_bounds(
        &mut self,
        type_: &Type,
        bounds: &[crate::types::generics::TraitBound],
    ) -> Result<(), Error> {
        let missing = self.trait_checker.validate_trait_bounds(type_, bounds);
        if !missing.is_empty() {
            let trait_names: Vec<_> = missing.iter().map(|m| m.trait_.name()).collect();
            return Err(Error::new(
                crate::error::ErrorKind::TypeError,
                format!(
                    "Type {} does not implement required traits: {}",
                    type_,
                    trait_names.join(", ")
                ),
            )
            .with_location(bounds[0].span.start));
        }
        Ok(())
    }
}

impl Default for InferenceContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a type annotation from the AST to a Type
pub fn type_ann_to_type(type_ann: &TypeAnn) -> Type {
    match &type_ann.kind {
        TypeKind::Named(name) => {
            // Map common type names to built-in types
            match name.as_str() {
                "i32" => Type::I32,
                "f32" => Type::F32,
                "bool" => Type::Bool,
                "string" => Type::String,
                "unknown" => Type::Unknown,
                _ => Type::Named(name.clone()),
            }
        }
        TypeKind::Array(elem_type) => Type::Array(Box::new(type_ann_to_type(elem_type))),
        TypeKind::Function { params, ret } => Type::Function {
            params: params.iter().map(type_ann_to_type).collect(),
            ret: Box::new(type_ann_to_type(ret)),
        },
        TypeKind::Generic { name, args } => Type::Generic {
            name: name.clone(),
            args: args.iter().map(type_ann_to_type).collect(),
        },
        TypeKind::TypeParam(name) => Type::TypeParam(name.clone()),
        TypeKind::Tuple(types) => Type::Tuple(types.iter().map(type_ann_to_type).collect()),
        TypeKind::Reference { mutable, inner } => Type::Reference {
            mutable: *mutable,
            inner: Box::new(type_ann_to_type(inner)),
        },
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod integration_test;

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::source::{SourceLocation, Span};

    fn test_span() -> Span {
        Span::new(SourceLocation::new(1, 1, 0), SourceLocation::new(1, 10, 10))
    }

    #[test]
    fn test_fresh_type_vars() {
        let mut ctx = InferenceContext::new();

        let t1 = ctx.fresh_type_var();
        let t2 = ctx.fresh_type_var();
        let t3 = ctx.fresh_type_var();

        assert_eq!(t1, Type::TypeVar(0));
        assert_eq!(t2, Type::TypeVar(1));
        assert_eq!(t3, Type::TypeVar(2));
        assert_ne!(t1, t2);
    }

    #[test]
    fn test_type_ann_conversion() {
        use crate::source::{SourceLocation, Span};

        // Test basic types
        let i32_ann = TypeAnn {
            kind: TypeKind::Named("i32".to_string()),
            span: Span::new(SourceLocation::new(1, 1, 0), SourceLocation::new(1, 3, 3)),
        };
        assert_eq!(type_ann_to_type(&i32_ann), Type::I32);

        // Test array type
        let array_ann = TypeAnn {
            kind: TypeKind::Array(Box::new(TypeAnn {
                kind: TypeKind::Named("bool".to_string()),
                span: Span::new(SourceLocation::new(1, 2, 1), SourceLocation::new(1, 5, 5)),
            })),
            span: Span::new(SourceLocation::new(1, 1, 0), SourceLocation::new(1, 7, 7)),
        };
        assert_eq!(
            type_ann_to_type(&array_ann),
            Type::Array(Box::new(Type::Bool))
        );

        // Test function type
        let fn_ann = TypeAnn {
            kind: TypeKind::Function {
                params: vec![
                    TypeAnn {
                        kind: TypeKind::Named("i32".to_string()),
                        span: Span::new(SourceLocation::new(1, 2, 1), SourceLocation::new(1, 4, 4)),
                    },
                    TypeAnn {
                        kind: TypeKind::Named("f32".to_string()),
                        span: Span::new(SourceLocation::new(1, 7, 6), SourceLocation::new(1, 9, 9)),
                    },
                ],
                ret: Box::new(TypeAnn {
                    kind: TypeKind::Named("string".to_string()),
                    span: Span::new(
                        SourceLocation::new(1, 15, 14),
                        SourceLocation::new(1, 20, 20),
                    ),
                }),
            },
            span: Span::new(SourceLocation::new(1, 1, 0), SourceLocation::new(1, 20, 20)),
        };
        assert_eq!(
            type_ann_to_type(&fn_ann),
            Type::Function {
                params: vec![Type::I32, Type::F32],
                ret: Box::new(Type::String),
            }
        );
    }

    #[test]
    fn test_trait_checking_integration() {
        let mut ctx = InferenceContext::new();

        // Test basic trait implementation check
        assert!(ctx.check_trait_implementation(&Type::I32, "Eq"));
        assert!(ctx.check_trait_implementation(&Type::I32, "Clone"));
        assert!(ctx.check_trait_implementation(&Type::I32, "Ord"));
        assert!(!ctx.check_trait_implementation(&Type::String, "Ord"));
    }

    #[test]
    fn test_trait_bound_constraints() {
        let mut ctx = InferenceContext::new();
        let span = test_span();

        // Add a trait bound constraint
        ctx.add_trait_bound(Type::I32, "Eq".to_string(), span);

        // This should succeed since i32 implements Eq
        assert!(ctx.solve_constraints().is_ok());
    }

    #[test]
    fn test_trait_bound_constraint_failure() {
        let mut ctx = InferenceContext::new();
        let span = test_span();

        // Add a trait bound constraint that should fail
        ctx.add_trait_bound(Type::String, "Ord".to_string(), span);

        // This should fail since String doesn't implement Ord
        assert!(ctx.solve_constraints().is_err());
    }

    #[test]
    fn test_generic_bounds_constraints() {
        let mut ctx = InferenceContext::new();
        let span = test_span();

        // Define a type parameter with a concrete type
        ctx.type_env_mut().define("T".to_string(), Type::I32);

        // Add generic bounds constraint
        ctx.add_generic_bounds(
            "T".to_string(),
            vec!["Eq".to_string(), "Clone".to_string()],
            span,
        );

        // This should succeed since i32 implements both Eq and Clone
        assert!(ctx.solve_constraints().is_ok());
    }

    #[test]
    fn test_generic_bounds_constraint_failure() {
        let mut ctx = InferenceContext::new();
        let span = test_span();

        // Define a type parameter with a concrete type
        ctx.type_env_mut().define("T".to_string(), Type::String);

        // Add generic bounds constraint that should fail
        ctx.add_generic_bounds("T".to_string(), vec!["Ord".to_string()], span);

        // This should fail since String doesn't implement Ord
        assert!(ctx.solve_constraints().is_err());
    }

    #[test]
    fn test_trait_bounds_validation() {
        use crate::types::generics::TraitBound;

        let mut ctx = InferenceContext::new();
        let span = test_span();

        // Test validation with valid bounds
        let bounds = vec![
            TraitBound::new("Eq".to_string(), span),
            TraitBound::new("Clone".to_string(), span),
        ];

        assert!(ctx.validate_trait_bounds(&Type::I32, &bounds).is_ok());

        // Test validation with invalid bounds
        let invalid_bounds = vec![TraitBound::new("Ord".to_string(), span)];

        assert!(ctx
            .validate_trait_bounds(&Type::String, &invalid_bounds)
            .is_err());
    }

    #[test]
    fn test_array_trait_inheritance() {
        let mut ctx = InferenceContext::new();

        // Arrays should inherit traits from their element type
        let int_array = Type::Array(Box::new(Type::I32));
        assert!(ctx.check_trait_implementation(&int_array, "Eq"));
        assert!(ctx.check_trait_implementation(&int_array, "Clone"));
        assert!(ctx.check_trait_implementation(&int_array, "Ord"));

        let string_array = Type::Array(Box::new(Type::String));
        assert!(ctx.check_trait_implementation(&string_array, "Eq"));
        assert!(ctx.check_trait_implementation(&string_array, "Clone"));
        assert!(!ctx.check_trait_implementation(&string_array, "Ord"));
    }

    #[test]
    fn test_option_trait_inheritance() {
        let mut ctx = InferenceContext::new();

        // Options should inherit traits from their inner type
        let int_option = Type::Option(Box::new(Type::I32));
        assert!(ctx.check_trait_implementation(&int_option, "Eq"));
        assert!(ctx.check_trait_implementation(&int_option, "Clone"));
        assert!(ctx.check_trait_implementation(&int_option, "Ord"));

        let string_option = Type::Option(Box::new(Type::String));
        assert!(ctx.check_trait_implementation(&string_option, "Eq"));
        assert!(ctx.check_trait_implementation(&string_option, "Clone"));
        assert!(!ctx.check_trait_implementation(&string_option, "Ord"));
    }
}
