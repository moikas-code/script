use std::collections::HashMap;
use std::time::Instant;
use crate::error::{Error, ErrorKind};
use crate::parser::{GenericParams, TypeAnn, TypeKind};
use crate::source::Span;
use crate::types::Type;
// Security imports removed - now handled internally
use super::{Constraint, ConstraintKind, Substitution, unify, apply_substitution, type_ann_to_type};

/// Type inference engine specifically for constructor expressions
#[derive(Debug)]
pub struct ConstructorInferenceEngine {
    /// Next available type variable ID
    next_type_var: u32,
    /// Collected constraints
    constraints: Vec<Constraint>,
    /// Current substitution
    substitution: Substitution,
    /// Mapping of generic parameter names to type variables
    type_param_map: HashMap<String, u32>,
    /// Resource limit tracking
    solving_iterations: usize,
    /// Inference start time for timeout detection
    inference_start: Option<Instant>,
}

/// Result of constructor type inference
#[derive(Debug, Clone)]
pub struct ConstructorInferenceResult {
    /// Inferred type arguments for generic parameters
    pub type_args: Vec<Type>,
    /// Final substitution after solving constraints
    pub substitution: Substitution,
}

impl ConstructorInferenceEngine {
    /// Security limits for DoS prevention
    const MAX_TYPE_VARS: u32 = 10_000;
    const MAX_CONSTRAINTS: usize = 50_000;
    const MAX_SOLVING_ITERATIONS: usize = 1_000;
    const MAX_INFERENCE_TIME_SECS: u64 = 30;

    /// Create a new inference engine
    pub fn new() -> Self {
        ConstructorInferenceEngine {
            next_type_var: 0,
            constraints: Vec::new(),
            substitution: Substitution::new(),
            type_param_map: HashMap::new(),
            solving_iterations: 0,
            inference_start: None,
        }
    }
    
    /// Generate a fresh type variable with security limits
    fn fresh_type_var(&mut self) -> Result<Type, Error> {
        // SECURITY CHECK: Prevent DoS through excessive type variable generation
        if self.next_type_var >= Self::MAX_TYPE_VARS {
            return Err(Error::security_violation(format!(
                "Type variable limit exceeded: {} >= {}. This prevents DoS attacks through exponential type variable generation.",
                self.next_type_var, Self::MAX_TYPE_VARS
            )));
        }

        let id = self.next_type_var;
        self.next_type_var += 1;
        Ok(Type::TypeVar(id))
    }
    
    /// Add a constraint to be solved with security limits
    fn add_constraint(&mut self, constraint: Constraint) -> Result<(), Error> {
        // SECURITY CHECK: Prevent DoS through excessive constraint generation
        if self.constraints.len() >= Self::MAX_CONSTRAINTS {
            return Err(Error::security_violation(format!(
                "Constraint limit exceeded: {} >= {}. This prevents DoS attacks through exponential constraint explosion.",
                self.constraints.len(), Self::MAX_CONSTRAINTS
            )));
        }

        self.constraints.push(constraint);
        Ok(())
    }
    
    /// Initialize type variables for generic parameters
    pub fn initialize_generic_params(&mut self, generic_params: &GenericParams) -> Result<Vec<Type>, Error> {
        let mut type_vars = Vec::new();
        
        for param in &generic_params.params {
            let type_var = self.fresh_type_var()?;
            if let Type::TypeVar(id) = &type_var {
                self.type_param_map.insert(param.name.clone(), *id);
            }
            
            // Add trait bound constraints if any
            for bound in &param.bounds {
                self.add_constraint(Constraint::trait_bound(
                    type_var.clone(),
                    bound.trait_name.clone(),
                    bound.span,
                ))?;
            }
            
            type_vars.push(type_var);
        }
        
        Ok(type_vars)
    }
    
    /// Convert a TypeAnn to Type, substituting type parameters with type variables
    fn type_ann_to_type_with_params(&self, type_ann: &TypeAnn) -> Type {
        match &type_ann.kind {
            TypeKind::TypeParam(name) => {
                // Look up the type variable for this parameter
                if let Some(&var_id) = self.type_param_map.get(name) {
                    Type::TypeVar(var_id)
                } else {
                    // Not a known type parameter, treat as concrete type
                    Type::TypeParam(name.clone())
                }
            }
            TypeKind::Generic { name, args } => {
                // Recursively convert type arguments
                let converted_args: Vec<Type> = args.iter()
                    .map(|arg| self.type_ann_to_type_with_params(arg))
                    .collect();
                
                // Handle special cases like Option and Result
                match name.as_str() {
                    "Option" if converted_args.len() == 1 => {
                        Type::Option(Box::new(converted_args[0].clone()))
                    }
                    "Result" if converted_args.len() == 2 => {
                        Type::Result {
                            ok: Box::new(converted_args[0].clone()),
                            err: Box::new(converted_args[1].clone()),
                        }
                    }
                    _ => Type::Generic {
                        name: name.clone(),
                        args: converted_args,
                    }
                }
            }
            TypeKind::Tuple(types) => {
                Type::Tuple(types.iter()
                    .map(|t| self.type_ann_to_type_with_params(t))
                    .collect())
            }
            TypeKind::Reference { mutable, inner } => {
                Type::Reference {
                    mutable: *mutable,
                    inner: Box::new(self.type_ann_to_type_with_params(inner)),
                }
            }
            TypeKind::Array(elem) => {
                Type::Array(Box::new(self.type_ann_to_type_with_params(elem)))
            }
            TypeKind::Function { params, ret } => {
                Type::Function {
                    params: params.iter()
                        .map(|p| self.type_ann_to_type_with_params(p))
                        .collect(),
                    ret: Box::new(self.type_ann_to_type_with_params(ret)),
                }
            }
            // For other cases, use the standard conversion
            _ => type_ann_to_type(type_ann),
        }
    }
    
    /// Generate constraints for struct field assignments
    pub fn constrain_struct_fields(
        &mut self,
        expected_fields: &[(String, TypeAnn)],
        provided_values: &[(String, Type)],
        span: Span,
    ) -> Result<(), Error> {
        // Create a map of provided fields for easy lookup
        let provided_map: HashMap<&String, &Type> = provided_values.iter()
            .map(|(name, ty)| (name, ty))
            .collect();
        
        // Generate constraints for each expected field
        for (field_name, field_type_ann) in expected_fields {
            if let Some(&provided_type) = provided_map.get(field_name) {
                // Convert expected type with parameter substitution
                let expected_type = self.type_ann_to_type_with_params(field_type_ann);
                
                // Add equality constraint
                self.add_constraint(Constraint::equality(
                    expected_type,
                    provided_type.clone(),
                    span,
                ))?;
            }
        }
        
        Ok(())
    }
    
    /// Generate constraints for enum variant arguments
    pub fn constrain_enum_variant_args(
        &mut self,
        expected_types: &[Type],
        provided_types: &[Type],
        span: Span,
    ) -> Result<(), Error> {
        if expected_types.len() != provided_types.len() {
            return Err(Error::new(
                ErrorKind::TypeError,
                format!(
                    "Argument count mismatch: expected {}, found {}",
                    expected_types.len(),
                    provided_types.len()
                ),
            ).with_location(span.start));
        }
        
        // Generate constraints for each argument
        for (expected, provided) in expected_types.iter().zip(provided_types.iter()) {
            self.add_constraint(Constraint::equality(
                expected.clone(),
                provided.clone(),
                span,
            ))?;
        }
        
        Ok(())
    }
    
    /// Generate constraints for partial type annotation
    pub fn constrain_partial_annotation(
        &mut self,
        partial_type: &Type,
        full_type: &Type,
        span: Span,
    ) -> Result<(), Error> {
        // Handle wildcard types (e.g., Box<_>)
        match (partial_type, full_type) {
            (Type::Generic { name: n1, args: a1 }, Type::Generic { name: n2, args: a2 }) 
                if n1 == n2 && a1.len() == a2.len() => {
                // For each type argument, if it's Unknown (_), create a fresh type variable
                for (partial_arg, full_arg) in a1.iter().zip(a2.iter()) {
                    if matches!(partial_arg, Type::Unknown) {
                        // Create fresh type variable for wildcard
                        let fresh_var = self.fresh_type_var()?;
                        self.add_constraint(Constraint::equality(
                            fresh_var,
                            full_arg.clone(),
                            span,
                        ))?;
                    } else {
                        // Recursively constrain non-wildcard arguments
                        self.constrain_partial_annotation(partial_arg, full_arg, span)?;
                    }
                }
                Ok(())
            }
            _ => {
                // For non-generic types, just add equality constraint
                self.add_constraint(Constraint::equality(
                    partial_type.clone(),
                    full_type.clone(),
                    span,
                ))?;
                Ok(())
            }
        }
    }
    
    /// Solve all collected constraints
    pub fn solve_constraints(&mut self) -> Result<(), Error> {
        // SECURITY CHECK: Initialize timing for timeout detection
        if self.inference_start.is_none() {
            self.inference_start = Some(Instant::now());
        }

        // Take ownership of constraints to avoid borrowing issues
        let constraints = std::mem::take(&mut self.constraints);
        
        for constraint in constraints {
            // SECURITY CHECK: Prevent DoS through excessive solving iterations
            self.solving_iterations += 1;
            if self.solving_iterations >= Self::MAX_SOLVING_ITERATIONS {
                return Err(Error::security_violation(format!(
                    "Solving iteration limit exceeded: {} >= {}. This prevents DoS attacks through infinite constraint solving loops.",
                    self.solving_iterations, Self::MAX_SOLVING_ITERATIONS
                )));
            }

            // SECURITY CHECK: Prevent DoS through compilation timeout
            if let Some(start_time) = self.inference_start {
                if start_time.elapsed().as_secs() >= Self::MAX_INFERENCE_TIME_SECS {
                    return Err(Error::security_violation(format!(
                        "Type inference timeout exceeded: {} seconds. This prevents DoS attacks through long-running type inference.",
                        Self::MAX_INFERENCE_TIME_SECS
                    )));
                }
            }
            match &constraint.kind {
                ConstraintKind::Equality(t1, t2) => {
                    // Apply current substitution before unifying
                    let t1_subst = apply_substitution(&self.substitution, t1);
                    let t2_subst = apply_substitution(&self.substitution, t2);
                    
                    // Unify and get new substitution
                    let new_subst = unify(&t1_subst, &t2_subst, constraint.span)?;
                    
                    // Compose with existing substitution
                    self.substitution.compose(new_subst);
                }
                ConstraintKind::TraitBound { .. } => {
                    // Trait bounds are checked separately after type inference
                    // For now, we just collect them
                }
                ConstraintKind::GenericBounds { .. } => {
                    // Generic bounds are also checked separately
                }
            }
        }
        
        Ok(())
    }
    
    /// Extract the final type arguments for generic parameters
    pub fn extract_type_args(&self, generic_params: &GenericParams) -> Vec<Type> {
        let mut type_args = Vec::new();
        
        for param in &generic_params.params {
            if let Some(&var_id) = self.type_param_map.get(&param.name) {
                // Apply substitution to get concrete type
                let type_var = Type::TypeVar(var_id);
                let concrete_type = apply_substitution(&self.substitution, &type_var);
                type_args.push(concrete_type);
            } else {
                // This shouldn't happen if initialize_generic_params was called
                type_args.push(Type::Unknown);
            }
        }
        
        type_args
    }
    
    /// Perform type inference and return the result
    pub fn infer(mut self, generic_params: &GenericParams) -> Result<ConstructorInferenceResult, Error> {
        // Solve all constraints
        self.solve_constraints()?;
        
        // Extract type arguments
        let type_args = self.extract_type_args(generic_params);
        
        Ok(ConstructorInferenceResult {
            type_args,
            substitution: self.substitution,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceLocation;
    use crate::parser::{GenericParam, TraitBound};
    
    fn test_span() -> Span {
        Span::new(SourceLocation::new(1, 1, 0), SourceLocation::new(1, 10, 10))
    }
    
    fn make_generic_params(names: Vec<&str>) -> GenericParams {
        GenericParams {
            params: names.into_iter().map(|name| GenericParam {
                name: name.to_string(),
                bounds: vec![],
                span: test_span(),
            }).collect(),
            span: test_span(),
        }
    }
    
    #[test]
    fn test_simple_struct_inference() {
        let mut engine = ConstructorInferenceEngine::new();
        let generic_params = make_generic_params(vec!["T"]);
        
        // Initialize type variables for generic parameters
        engine.initialize_generic_params(&generic_params);
        
        // Struct field: value: T
        let field_type = TypeAnn {
            kind: TypeKind::TypeParam("T".to_string()),
            span: test_span(),
        };
        
        // Provided value: 42 (i32)
        let provided_type = Type::I32;
        
        // Generate constraints
        engine.constrain_struct_fields(
            &vec![("value".to_string(), field_type)],
            &vec![("value".to_string(), provided_type)],
            test_span(),
        ).unwrap();
        
        // Infer types
        let result = engine.infer(&generic_params).unwrap();
        
        // Should infer T = i32
        assert_eq!(result.type_args.len(), 1);
        assert_eq!(result.type_args[0], Type::I32);
    }
    
    #[test]
    fn test_nested_generic_inference() {
        let mut engine = ConstructorInferenceEngine::new();
        let generic_params = make_generic_params(vec!["T"]);
        
        engine.initialize_generic_params(&generic_params);
        
        // Struct field: value: Option<T>
        let field_type = TypeAnn {
            kind: TypeKind::Generic {
                name: "Option".to_string(),
                args: vec![TypeAnn {
                    kind: TypeKind::TypeParam("T".to_string()),
                    span: test_span(),
                }],
            },
            span: test_span(),
        };
        
        // Provided value: Some(42) - Option<i32>
        let provided_type = Type::Option(Box::new(Type::I32));
        
        // Generate constraints
        engine.constrain_struct_fields(
            &vec![("value".to_string(), field_type)],
            &vec![("value".to_string(), provided_type)],
            test_span(),
        ).unwrap();
        
        // Infer types
        let result = engine.infer(&generic_params).unwrap();
        
        // Should infer T = i32
        assert_eq!(result.type_args.len(), 1);
        assert_eq!(result.type_args[0], Type::I32);
    }
    
    #[test]
    fn test_multiple_type_params() {
        let mut engine = ConstructorInferenceEngine::new();
        let generic_params = make_generic_params(vec!["T", "U"]);
        
        engine.initialize_generic_params(&generic_params);
        
        // Struct fields: first: T, second: U
        let field1 = TypeAnn {
            kind: TypeKind::TypeParam("T".to_string()),
            span: test_span(),
        };
        let field2 = TypeAnn {
            kind: TypeKind::TypeParam("U".to_string()),
            span: test_span(),
        };
        
        // Provided values
        let provided = vec![
            ("first".to_string(), Type::I32),
            ("second".to_string(), Type::String),
        ];
        
        // Generate constraints
        engine.constrain_struct_fields(
            &vec![
                ("first".to_string(), field1),
                ("second".to_string(), field2),
            ],
            &provided,
            test_span(),
        ).unwrap();
        
        // Infer types
        let result = engine.infer(&generic_params).unwrap();
        
        // Should infer T = i32, U = string
        assert_eq!(result.type_args.len(), 2);
        assert_eq!(result.type_args[0], Type::I32);
        assert_eq!(result.type_args[1], Type::String);
    }
}