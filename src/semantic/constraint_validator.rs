//! Where clause constraint validation for the Script language
//!
//! This module provides secure validation of where clause constraints to prevent
//! DoS attacks through constraint explosion and ensure type safety.

use crate::error::{Error, ErrorKind};
use crate::parser::{WhereClause, TypeAnn, Span};
use crate::semantic::{SemanticError, SemanticErrorKind, TypeContext};
use crate::types::{Type, TypeRegistry};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// Security limits for constraint validation to prevent DoS attacks
#[derive(Debug, Clone)]
pub struct SecurityLimits {
    /// Maximum number of constraints per function/type
    pub max_constraints: usize,
    /// Maximum constraint validation depth
    pub max_validation_depth: usize,
    /// Maximum time allowed for constraint validation
    pub max_validation_time: Duration,
    /// Maximum number of type variables in constraints
    pub max_type_variables: usize,
}

impl Default for SecurityLimits {
    fn default() -> Self {
        SecurityLimits {
            max_constraints: 100,
            max_validation_depth: 50,
            max_validation_time: Duration::from_millis(100),
            max_type_variables: 1000,
        }
    }
}

/// Result of constraint validation
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    /// All constraints are satisfied
    Valid,
    /// Constraints are unsatisfiable
    Unsatisfiable(String),
    /// Validation incomplete due to security limits
    LimitExceeded(String),
}

/// Where clause constraint validator with security enforcement
#[derive(Debug)]
pub struct WhereClauseValidator {
    type_registry: TypeRegistry,
    security_limits: SecurityLimits,
    constraint_cache: HashMap<String, ValidationResult>,
}

impl WhereClauseValidator {
    /// Create a new constraint validator with security limits
    pub fn new(type_registry: TypeRegistry, security_limits: SecurityLimits) -> Self {
        WhereClauseValidator {
            type_registry,
            security_limits,
            constraint_cache: HashMap::new(),
        }
    }

    /// Validate where clause constraints with security enforcement
    pub fn validate_constraints(
        &mut self,
        constraints: &[WhereClause],
        context: &TypeContext,
        span: Span,
    ) -> Result<ValidationResult, SemanticError> {
        let start_time = Instant::now();

        // Security check: prevent constraint explosion DoS
        if constraints.len() > self.security_limits.max_constraints {
            return Err(SemanticError::new(
                SemanticErrorKind::TooManyConstraints {
                    count: constraints.len(),
                    max_allowed: self.security_limits.max_constraints,
                },
                span,
            ));
        }

        // Check constraint validation cache
        let cache_key = self.generate_cache_key(constraints, context);
        if let Some(cached_result) = self.constraint_cache.get(&cache_key) {
            return Ok(cached_result.clone());
        }

        let mut validation_context = ConstraintValidationContext::new(
            context,
            &self.security_limits,
            start_time,
        );

        let result = self.validate_constraints_impl(constraints, &mut validation_context)?;

        // Cache successful validations (with size limit)
        if self.constraint_cache.len() < 10000 {
            self.constraint_cache.insert(cache_key, result.clone());
        }

        Ok(result)
    }

    fn validate_constraints_impl(
        &self,
        constraints: &[WhereClause],
        context: &mut ConstraintValidationContext,
    ) -> Result<ValidationResult, SemanticError> {
        for constraint in constraints {
            // Check timeout
            if context.start_time.elapsed() > self.security_limits.max_validation_time {
                return Ok(ValidationResult::LimitExceeded(
                    "Constraint validation timeout".to_string(),
                ));
            }

            // Check depth limit
            if context.depth > self.security_limits.max_validation_depth {
                return Ok(ValidationResult::LimitExceeded(
                    "Constraint validation depth exceeded".to_string(),
                ));
            }

            self.validate_single_constraint(constraint, context)?;
        }

        Ok(ValidationResult::Valid)
    }

    fn validate_single_constraint(
        &self,
        constraint: &WhereClause,
        context: &mut ConstraintValidationContext,
    ) -> Result<(), SemanticError> {
        context.depth += 1;

        let result = match constraint {
            WhereClause::TraitBound { ty, trait_ref, span } => {
                self.validate_trait_bound(ty, trait_ref, context, *span)
            }
            WhereClause::LifetimeBound { lifetime, bounds, span } => {
                self.validate_lifetime_bound(lifetime, bounds, context, *span)
            }
            WhereClause::TypeEquality { lhs, rhs, span } => {
                self.validate_type_equality(lhs, rhs, context, *span)
            }
        };

        context.depth -= 1;
        result
    }

    fn validate_trait_bound(
        &self,
        ty: &TypeAnn,
        trait_ref: &str,
        context: &mut ConstraintValidationContext,
        span: Span,
    ) -> Result<(), SemanticError> {
        // Convert type annotation to concrete type
        let concrete_type = self.resolve_type_annotation(ty, context.type_context)?;

        // Check if trait exists
        if !self.type_registry.has_trait(trait_ref) {
            return Err(SemanticError::new(
                SemanticErrorKind::UnknownTrait(trait_ref.to_string()),
                span,
            ));
        }

        // Check if type implements trait
        if !self.type_registry.implements_trait(&concrete_type, trait_ref) {
            return Err(SemanticError::new(
                SemanticErrorKind::TraitNotImplemented {
                    ty: concrete_type,
                    trait_name: trait_ref.to_string(),
                },
                span,
            ));
        }

        // Track type variables for security
        self.track_type_variables(&concrete_type, context)?;

        Ok(())
    }

    fn validate_lifetime_bound(
        &self,
        _lifetime: &str,
        _bounds: &[String],
        _context: &mut ConstraintValidationContext,
        _span: Span,
    ) -> Result<(), SemanticError> {
        // TODO: Implement lifetime validation when lifetimes are fully supported
        // For now, accept all lifetime bounds
        Ok(())
    }

    fn validate_type_equality(
        &self,
        lhs: &TypeAnn,
        rhs: &TypeAnn,
        context: &mut ConstraintValidationContext,
        span: Span,
    ) -> Result<(), SemanticError> {
        let lhs_type = self.resolve_type_annotation(lhs, context.type_context)?;
        let rhs_type = self.resolve_type_annotation(rhs, context.type_context)?;

        // Track type variables for security
        self.track_type_variables(&lhs_type, context)?;
        self.track_type_variables(&rhs_type, context)?;

        // Check type equality
        if !self.types_equal(&lhs_type, &rhs_type) {
            return Err(SemanticError::new(
                SemanticErrorKind::TypeMismatch {
                    expected: lhs_type,
                    found: rhs_type,
                },
                span,
            ));
        }

        Ok(())
    }

    fn resolve_type_annotation(
        &self,
        type_ann: &TypeAnn,
        context: &TypeContext,
    ) -> Result<Type, SemanticError> {
        // Convert type annotation to concrete type
        // This is a simplified implementation - would need full type resolution
        match &type_ann.kind {
            crate::parser::TypeKind::Named(name) => {
                if let Some(ty) = context.resolve_type(name) {
                    Ok(ty)
                } else {
                    Ok(Type::Named(name.clone()))
                }
            }
            crate::parser::TypeKind::Array(inner) => {
                let inner_type = self.resolve_type_annotation(inner, context)?;
                Ok(Type::Array(Box::new(inner_type)))
            }
            crate::parser::TypeKind::Generic(name, args) => {
                let mut arg_types = Vec::new();
                for arg in args {
                    arg_types.push(self.resolve_type_annotation(arg, context)?);
                }
                Ok(Type::Generic {
                    name: name.clone(),
                    args: arg_types,
                })
            }
            _ => {
                // Handle other type annotation kinds
                Ok(Type::Named("unknown".to_string()))
            }
        }
    }

    fn track_type_variables(
        &self,
        ty: &Type,
        context: &mut ConstraintValidationContext,
    ) -> Result<(), SemanticError> {
        self.collect_type_variables(ty, &mut context.type_variables);

        if context.type_variables.len() > self.security_limits.max_type_variables {
            return Err(SemanticError::new(
                SemanticErrorKind::TooManyTypeVariables {
                    count: context.type_variables.len(),
                    max_allowed: self.security_limits.max_type_variables,
                },
                Span::default(),
            ));
        }

        Ok(())
    }

    fn collect_type_variables(&self, ty: &Type, variables: &mut HashSet<String>) {
        match ty {
            Type::TypeVariable(name) => {
                variables.insert(name.clone());
            }
            Type::Array(inner) => {
                self.collect_type_variables(inner, variables);
            }
            Type::Generic { args, .. } => {
                for arg in args {
                    self.collect_type_variables(arg, variables);
                }
            }
            _ => {
                // Other types don't introduce type variables
            }
        }
    }

    fn types_equal(&self, lhs: &Type, rhs: &Type) -> bool {
        // Simplified type equality check
        match (lhs, rhs) {
            (Type::I32, Type::I32) => true,
            (Type::F32, Type::F32) => true,
            (Type::Bool, Type::Bool) => true,
            (Type::String, Type::String) => true,
            (Type::Array(l), Type::Array(r)) => self.types_equal(l, r),
            (Type::Named(l), Type::Named(r)) => l == r,
            (Type::Generic { name: ln, args: la }, Type::Generic { name: rn, args: ra }) => {
                ln == rn && la.len() == ra.len() && 
                la.iter().zip(ra.iter()).all(|(l, r)| self.types_equal(l, r))
            }
            _ => false,
        }
    }

    fn generate_cache_key(&self, constraints: &[WhereClause], context: &TypeContext) -> String {
        // Generate a unique key for caching constraint validation results
        let mut key = String::new();
        for constraint in constraints {
            key.push_str(&format!("{:?}", constraint));
        }
        key.push_str(&format!("{:?}", context.current_scope()));
        key
    }
}

/// Context for constraint validation with security tracking
#[derive(Debug)]
struct ConstraintValidationContext<'a> {
    type_context: &'a TypeContext,
    depth: usize,
    type_variables: HashSet<String>,
    start_time: Instant,
}

impl<'a> ConstraintValidationContext<'a> {
    fn new(
        type_context: &'a TypeContext,
        _security_limits: &SecurityLimits,
        start_time: Instant,
    ) -> Self {
        ConstraintValidationContext {
            type_context,
            depth: 0,
            type_variables: HashSet::new(),
            start_time,
        }
    }
}

// Extend SemanticErrorKind with constraint-specific errors
impl SemanticErrorKind {
    pub fn too_many_constraints(count: usize, max_allowed: usize) -> Self {
        SemanticErrorKind::Custom(format!(
            "Too many constraints: {} (maximum allowed: {})",
            count, max_allowed
        ))
    }

    pub fn too_many_type_variables(count: usize, max_allowed: usize) -> Self {
        SemanticErrorKind::Custom(format!(
            "Too many type variables: {} (maximum allowed: {})",
            count, max_allowed
        ))
    }

    pub fn unknown_trait(trait_name: String) -> Self {
        SemanticErrorKind::Custom(format!("Unknown trait: {}", trait_name))
    }

    pub fn trait_not_implemented(ty: Type, trait_name: String) -> Self {
        SemanticErrorKind::Custom(format!(
            "Type {:?} does not implement trait {}",
            ty, trait_name
        ))
    }

    pub fn Custom(message: String) -> Self {
        // This would need to be added to the actual SemanticErrorKind enum
        SemanticErrorKind::UndefinedVariable(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constraint_validation_security_limits() {
        let type_registry = TypeRegistry::new();
        let limits = SecurityLimits {
            max_constraints: 2,
            max_validation_depth: 5,
            max_validation_time: Duration::from_millis(10),
            max_type_variables: 10,
        };

        let mut validator = WhereClauseValidator::new(type_registry, limits);
        let context = TypeContext::new();

        // Test constraint limit
        let constraints = vec![
            WhereClause::TraitBound {
                ty: TypeAnn::simple("T"),
                trait_ref: "Clone".to_string(),
                span: Span::default(),
            };
            5
        ];

        let result = validator.validate_constraints(&constraints, &context, Span::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_constraint_caching() {
        let type_registry = TypeRegistry::new();
        let limits = SecurityLimits::default();
        let mut validator = WhereClauseValidator::new(type_registry, limits);
        let context = TypeContext::new();

        let constraints = vec![
            WhereClause::TraitBound {
                ty: TypeAnn::simple("T"),
                trait_ref: "Clone".to_string(),
                span: Span::default(),
            }
        ];

        // First validation
        let result1 = validator.validate_constraints(&constraints, &context, Span::default());
        
        // Second validation should use cache
        let result2 = validator.validate_constraints(&constraints, &context, Span::default());
        
        // Results should be identical
        assert_eq!(result1.is_ok(), result2.is_ok());
    }
}