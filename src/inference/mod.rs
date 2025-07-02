use crate::error::Error;
use crate::parser::{TypeAnn, TypeKind};
use crate::types::{Type, TypeEnv};

mod constraint;
mod inference_engine;
mod substitution;
mod unification;

pub use constraint::{Constraint, ConstraintKind};
pub use inference_engine::{InferenceEngine, InferenceResult};
pub use substitution::{apply_substitution, Substitution};
pub use unification::unify;

/// Type inference context that manages type variables and constraints
#[derive(Debug, Clone)]
pub struct InferenceContext {
    /// Next available type variable ID
    next_type_var: u32,
    /// Current substitution mapping type variables to types
    substitution: Substitution,
    /// Type environment for looking up variable types
    type_env: TypeEnv,
    /// Collected constraints to be solved
    constraints: Vec<Constraint>,
}

impl InferenceContext {
    /// Create a new inference context
    pub fn new() -> Self {
        InferenceContext {
            next_type_var: 0,
            substitution: Substitution::new(),
            type_env: TypeEnv::new(),
            constraints: Vec::new(),
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
    pub fn apply_substitution(&self, ty: &Type) -> Type {
        apply_substitution(&self.substitution, ty)
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
                    let new_subst = unify(&t1_subst, &t2_subst, constraint.span)?;

                    // Compose with existing substitution
                    self.substitution.compose(new_subst);
                }
                ConstraintKind::TraitBound { .. } => {
                    // Trait bounds not yet implemented, skip for now
                }
                ConstraintKind::GenericBounds { .. } => {
                    // Generic bounds not yet implemented, skip for now
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
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod unit_tests {
    use super::*;

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
}
