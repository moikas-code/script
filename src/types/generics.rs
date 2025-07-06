use super::Type;
use crate::source::Span;
/// Generic type system for Script language
///
/// This module provides support for:
/// - Type parameters (T, U, K, V, etc.)
/// - Constraints/bounds on type parameters
/// - Built-in traits (Eq, Ord, Clone, Display, etc.)
/// - Generic type instantiation and monomorphization
use std::collections::{HashMap, HashSet};
use std::fmt;

/// A type parameter in a generic declaration
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeParam {
    /// Name of the type parameter (e.g., "T", "U", "Key", "Value")
    pub name: String,
    /// Constraints that this type parameter must satisfy
    pub bounds: Vec<TraitBound>,
    /// Source location for error reporting
    pub span: Span,
}

/// A constraint requiring a type to implement a specific trait
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitBound {
    /// Name of the trait (e.g., "Eq", "Ord", "Clone")
    pub trait_name: String,
    /// Source location for error reporting
    pub span: Span,
}

/// Collection of type parameters for a generic item
#[derive(Debug, Clone, PartialEq)]
pub struct GenericParams {
    /// List of type parameters
    pub params: Vec<TypeParam>,
    /// Optional where clause for complex constraints
    pub where_clause: Option<WhereClause>,
    /// Source location
    pub span: Span,
}

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
    pub type_: Type,
    /// The bounds that type must satisfy
    pub bounds: Vec<TraitBound>,
    /// Source location
    pub span: Span,
}

/// Built-in traits that types can implement
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BuiltinTrait {
    /// Equality comparison (==, !=)
    Eq,
    /// Ordering comparison (<, >, <=, >=)
    Ord,
    /// Cloning/duplication
    Clone,
    /// String representation for printing
    Display,
    /// Debug representation
    Debug,
    /// Default value construction
    Default,
    /// Copying (bitwise copy)
    Copy,
    /// Hash computation
    Hash,
}

/// A generic type that can be instantiated with concrete types
#[derive(Debug, Clone, PartialEq)]
pub struct GenericType {
    /// Base type constructor (e.g., "Vec", "Option", "Result")
    pub base: String,
    /// Type arguments for instantiation
    pub args: Vec<Type>,
    /// Original generic parameters (for constraint checking)
    pub params: Vec<TypeParam>,
}

/// Environment for tracking generic instantiations
#[derive(Debug, Clone)]
pub struct GenericEnv {
    /// Map from type parameter names to concrete types
    type_substitutions: HashMap<String, Type>,
    /// Available trait implementations
    trait_impls: HashMap<(Type, BuiltinTrait), bool>,
    /// Generic type definitions
    generic_types: HashMap<String, GenericTypeDefinition>,
}

/// Definition of a generic type (struct, enum, etc.)
#[derive(Debug, Clone)]
pub struct GenericTypeDefinition {
    /// Name of the type
    pub name: String,
    /// Generic parameters
    pub params: Vec<TypeParam>,
    /// Fields or variants (depending on type kind)
    pub body: GenericTypeBody,
    /// Source location
    pub span: Span,
}

/// Body of a generic type definition
#[derive(Debug, Clone)]
pub enum GenericTypeBody {
    /// Struct with named fields
    Struct { fields: Vec<FieldDef> },
    /// Enum with variants
    Enum { variants: Vec<VariantDef> },
    /// Type alias
    Alias { target: Type },
}

/// Field definition in a generic struct
#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub type_: Type,
    pub span: Span,
}

/// Variant definition in a generic enum
#[derive(Debug, Clone)]
pub struct VariantDef {
    pub name: String,
    pub fields: Vec<Type>,
    pub span: Span,
}

/// Result of generic constraint checking
#[derive(Debug, Clone)]
pub struct ConstraintCheckResult {
    /// Whether all constraints are satisfied
    pub satisfied: bool,
    /// Missing constraints that prevent satisfaction
    pub missing: Vec<MissingConstraint>,
}

/// A constraint that is not satisfied
#[derive(Debug, Clone)]
pub struct MissingConstraint {
    /// The type that doesn't satisfy the constraint
    pub type_: Type,
    /// The trait that is required but not implemented
    pub trait_: BuiltinTrait,
    /// Source location of the constraint
    pub span: Span,
}

impl TypeParam {
    /// Create a new type parameter
    pub fn new(name: String, span: Span) -> Self {
        TypeParam {
            name,
            bounds: Vec::new(),
            span,
        }
    }

    /// Add a trait bound to this type parameter
    pub fn with_bound(mut self, bound: TraitBound) -> Self {
        self.bounds.push(bound);
        self
    }

    /// Check if this type parameter has a specific bound
    pub fn has_bound(&self, trait_name: &str) -> bool {
        self.bounds.iter().any(|b| b.trait_name == trait_name)
    }
}

impl TraitBound {
    /// Create a new trait bound
    pub fn new(trait_name: String, span: Span) -> Self {
        TraitBound { trait_name, span }
    }

    /// Create a trait bound for a built-in trait
    pub fn builtin(trait_: BuiltinTrait, span: Span) -> Self {
        TraitBound {
            trait_name: trait_.name().to_string(),
            span,
        }
    }
}

impl GenericParams {
    /// Create an empty generic parameter list
    pub fn empty(span: Span) -> Self {
        GenericParams {
            params: Vec::new(),
            where_clause: None,
            span,
        }
    }

    /// Create generic parameters with a list of type parameters
    pub fn new(params: Vec<TypeParam>, span: Span) -> Self {
        GenericParams {
            params,
            where_clause: None,
            span,
        }
    }

    /// Add a where clause to these generic parameters
    pub fn with_where_clause(mut self, where_clause: WhereClause) -> Self {
        self.where_clause = Some(where_clause);
        self
    }

    /// Check if these generic parameters are empty
    pub fn is_empty(&self) -> bool {
        self.params.is_empty() && self.where_clause.is_none()
    }

    /// Get all constraints from both bounds and where clause
    pub fn all_constraints(&self) -> Vec<(String, Vec<TraitBound>)> {
        let mut constraints = Vec::new();

        // Add constraints from type parameter bounds
        for param in &self.params {
            if !param.bounds.is_empty() {
                constraints.push((param.name.clone(), param.bounds.clone()));
            }
        }

        // Add constraints from where clause
        if let Some(where_clause) = &self.where_clause {
            for constraint in &where_clause.constraints {
                if let Type::Named(name) = &constraint.type_ {
                    constraints.push((name.clone(), constraint.bounds.clone()));
                }
            }
        }

        constraints
    }
}

impl BuiltinTrait {
    /// Get the name of this trait
    pub fn name(&self) -> &'static str {
        match self {
            BuiltinTrait::Eq => "Eq",
            BuiltinTrait::Ord => "Ord",
            BuiltinTrait::Clone => "Clone",
            BuiltinTrait::Display => "Display",
            BuiltinTrait::Debug => "Debug",
            BuiltinTrait::Default => "Default",
            BuiltinTrait::Copy => "Copy",
            BuiltinTrait::Hash => "Hash",
        }
    }

    /// Parse a trait name into a builtin trait
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Eq" => Some(BuiltinTrait::Eq),
            "Ord" => Some(BuiltinTrait::Ord),
            "Clone" => Some(BuiltinTrait::Clone),
            "Display" => Some(BuiltinTrait::Display),
            "Debug" => Some(BuiltinTrait::Debug),
            "Default" => Some(BuiltinTrait::Default),
            "Copy" => Some(BuiltinTrait::Copy),
            "Hash" => Some(BuiltinTrait::Hash),
            _ => None,
        }
    }

    /// Check if this trait depends on another trait
    pub fn depends_on(&self, other: &BuiltinTrait) -> bool {
        match (self, other) {
            // Ord depends on Eq
            (BuiltinTrait::Ord, BuiltinTrait::Eq) => true,
            // Copy depends on Clone
            (BuiltinTrait::Copy, BuiltinTrait::Clone) => true,
            _ => false,
        }
    }

    /// Get all traits that this trait depends on
    pub fn dependencies(&self) -> Vec<BuiltinTrait> {
        match self {
            BuiltinTrait::Ord => vec![BuiltinTrait::Eq],
            BuiltinTrait::Copy => vec![BuiltinTrait::Clone],
            _ => Vec::new(),
        }
    }
}

impl GenericEnv {
    /// Create a new generic environment
    pub fn new() -> Self {
        let mut env = GenericEnv {
            type_substitutions: HashMap::new(),
            trait_impls: HashMap::new(),
            generic_types: HashMap::new(),
        };

        // Initialize built-in trait implementations
        env.init_builtin_impls();
        env
    }

    /// Initialize implementations of built-in traits for primitive types
    fn init_builtin_impls(&mut self) {
        let primitive_types = vec![Type::I32, Type::F32, Type::Bool, Type::String];

        for type_ in primitive_types {
            // All primitives implement basic traits
            self.trait_impls
                .insert((type_.clone(), BuiltinTrait::Eq), true);
            self.trait_impls
                .insert((type_.clone(), BuiltinTrait::Clone), true);
            self.trait_impls
                .insert((type_.clone(), BuiltinTrait::Display), true);
            self.trait_impls
                .insert((type_.clone(), BuiltinTrait::Debug), true);
            self.trait_impls
                .insert((type_.clone(), BuiltinTrait::Default), true);
            self.trait_impls
                .insert((type_.clone(), BuiltinTrait::Hash), true);

            // Numeric types implement Ord
            if matches!(type_, Type::I32 | Type::F32) {
                self.trait_impls
                    .insert((type_.clone(), BuiltinTrait::Ord), true);
            }

            // Simple types implement Copy
            if matches!(type_, Type::I32 | Type::F32 | Type::Bool) {
                self.trait_impls
                    .insert((type_.clone(), BuiltinTrait::Copy), true);
            }
        }
    }

    /// Add a type substitution
    pub fn add_substitution(&mut self, param: String, concrete: Type) {
        self.type_substitutions.insert(param, concrete);
    }

    /// Get a type substitution
    pub fn get_substitution(&self, param: &str) -> Option<&Type> {
        self.type_substitutions.get(param)
    }

    /// Apply type substitutions to a type with cycle detection
    pub fn substitute_type(&self, type_: &Type) -> Type {
        self.substitute_type_with_visited(type_, &mut HashSet::new())
    }

    /// Internal method for type substitution with cycle detection
    fn substitute_type_with_visited(&self, type_: &Type, visited: &mut HashSet<String>) -> Type {
        match type_ {
            Type::Named(name) => {
                if let Some(concrete) = self.get_substitution(name) {
                    self.substitute_type_with_visited(concrete, visited)
                } else {
                    type_.clone()
                }
            }
            Type::TypeParam(name) => {
                // Check for cycles
                if visited.contains(name) {
                    return Type::TypeParam(name.clone()); // Break cycle
                }
                
                if let Some(concrete) = self.get_substitution(name) {
                    visited.insert(name.clone());
                    let result = self.substitute_type_with_visited(concrete, visited);
                    visited.remove(name);
                    result
                } else {
                    type_.clone()
                }
            }
            Type::Generic { name, args } => {
                Type::Generic {
                    name: name.clone(),
                    args: args.iter().map(|arg| self.substitute_type_with_visited(arg, visited)).collect(),
                }
            }
            Type::Array(elem) => Type::Array(Box::new(self.substitute_type_with_visited(elem, visited))),
            Type::Function { params, ret } => Type::Function {
                params: params.iter().map(|p| self.substitute_type_with_visited(p, visited)).collect(),
                ret: Box::new(self.substitute_type_with_visited(ret, visited)),
            },
            Type::Result { ok, err } => Type::Result {
                ok: Box::new(self.substitute_type_with_visited(ok, visited)),
                err: Box::new(self.substitute_type_with_visited(err, visited)),
            },
            Type::Future(inner) => Type::Future(Box::new(self.substitute_type_with_visited(inner, visited))),
            Type::Option(inner) => Type::Option(Box::new(self.substitute_type_with_visited(inner, visited))),
            Type::TypeVar(id) => {
                // Type variables are handled by the inference engine, not generic substitution
                Type::TypeVar(*id)
            }
            _ => type_.clone(),
        }
    }

    /// Check if a type implements a trait
    pub fn implements_trait(&self, type_: &Type, trait_: &BuiltinTrait) -> bool {
        // Apply substitutions first
        let concrete_type = self.substitute_type(type_);

        // Check direct implementation
        if let Some(&implemented) = self
            .trait_impls
            .get(&(concrete_type.clone(), trait_.clone()))
        {
            return implemented;
        }

        // Check structural implementations (e.g., arrays, functions)
        match (&concrete_type, trait_) {
            // Arrays implement traits if their elements do
            (Type::Array(elem), _) => self.implements_trait(elem, trait_),

            // Functions implement Eq if they're the same function
            // (simplified - in practice this is complex)
            (Type::Function { .. }, BuiltinTrait::Eq) => false,

            // Options implement traits if their inner type does
            (Type::Option(inner), _) => self.implements_trait(inner, trait_),

            // Results implement traits if both Ok and Err types do
            (Type::Result { ok, err }, _) => {
                self.implements_trait(ok, trait_) && self.implements_trait(err, trait_)
            }

            _ => false,
        }
    }

    /// Check if all constraints are satisfied for a set of type substitutions
    pub fn check_constraints(
        &self,
        constraints: &[(String, Vec<TraitBound>)],
    ) -> ConstraintCheckResult {
        let mut missing = Vec::new();

        for (type_param, bounds) in constraints {
            if let Some(concrete_type) = self.get_substitution(type_param) {
                for bound in bounds {
                    if let Some(trait_) = BuiltinTrait::from_name(&bound.trait_name) {
                        if !self.implements_trait(concrete_type, &trait_) {
                            missing.push(MissingConstraint {
                                type_: concrete_type.clone(),
                                trait_,
                                span: bound.span,
                            });
                        }
                    }
                }
            }
        }

        ConstraintCheckResult {
            satisfied: missing.is_empty(),
            missing,
        }
    }

    /// Register a generic type definition
    pub fn define_generic_type(&mut self, def: GenericTypeDefinition) {
        self.generic_types.insert(def.name.clone(), def);
    }

    /// Get a generic type definition
    pub fn get_generic_type(&self, name: &str) -> Option<&GenericTypeDefinition> {
        self.generic_types.get(name)
    }

    /// Instantiate a generic type with concrete type arguments
    pub fn instantiate_generic(&self, name: &str, args: Vec<Type>) -> Option<Type> {
        let def = self.get_generic_type(name)?;

        if args.len() != def.params.len() {
            return None; // Wrong number of type arguments
        }

        // Create substitution environment
        let mut inst_env = self.clone();
        for (param, arg) in def.params.iter().zip(args.iter()) {
            inst_env.add_substitution(param.name.clone(), arg.clone());
        }

        // Check constraints
        let constraints: Vec<_> = def
            .params
            .iter()
            .map(|p| (p.name.clone(), p.bounds.clone()))
            .collect();
        let check_result = inst_env.check_constraints(&constraints);

        if !check_result.satisfied {
            return None; // Constraints not satisfied
        }

        // Return instantiated type
        Some(Type::Named(format!(
            "{}<{}>",
            name,
            args.iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }
}

impl Default for GenericEnv {
    fn default() -> Self {
        Self::new()
    }
}

// Display implementations
impl fmt::Display for TypeParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if !self.bounds.is_empty() {
            write!(f, ": ")?;
            for (i, bound) in self.bounds.iter().enumerate() {
                if i > 0 {
                    write!(f, " + ")?;
                }
                write!(f, "{}", bound.trait_name)?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for GenericParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.params.is_empty() {
            return Ok(());
        }

        write!(f, "<")?;
        for (i, param) in self.params.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", param)?;
        }
        write!(f, ">")?;

        if let Some(where_clause) = &self.where_clause {
            write!(f, " {}", where_clause)?;
        }

        Ok(())
    }
}

impl fmt::Display for WhereClause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "where ")?;
        for (i, constraint) in self.constraints.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}: ", constraint.type_)?;
            for (j, bound) in constraint.bounds.iter().enumerate() {
                if j > 0 {
                    write!(f, " + ")?;
                }
                write!(f, "{}", bound.trait_name)?;
            }
        }
        Ok(())
    }
}

// Additional comprehensive tests are in separate file when needed

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{SourceLocation, Span};

    fn test_span() -> Span {
        Span::new(SourceLocation::new(1, 1, 0), SourceLocation::new(1, 10, 10))
    }

    #[test]
    fn test_type_param_creation() {
        let param = TypeParam::new("T".to_string(), test_span());
        assert_eq!(param.name, "T");
        assert!(param.bounds.is_empty());
    }

    #[test]
    fn test_type_param_with_bounds() {
        let bound = TraitBound::builtin(BuiltinTrait::Eq, test_span());
        let param = TypeParam::new("T".to_string(), test_span()).with_bound(bound);

        assert_eq!(param.bounds.len(), 1);
        assert_eq!(param.bounds[0].trait_name, "Eq");
        assert!(param.has_bound("Eq"));
        assert!(!param.has_bound("Ord"));
    }

    #[test]
    fn test_builtin_trait_dependencies() {
        assert!(BuiltinTrait::Ord.depends_on(&BuiltinTrait::Eq));
        assert!(BuiltinTrait::Copy.depends_on(&BuiltinTrait::Clone));
        assert!(!BuiltinTrait::Eq.depends_on(&BuiltinTrait::Ord));
    }

    #[test]
    fn test_generic_env_substitution() {
        let mut env = GenericEnv::new();
        env.add_substitution("T".to_string(), Type::I32);

        let substituted = env.substitute_type(&Type::Named("T".to_string()));
        assert_eq!(substituted, Type::I32);

        let array_type = Type::Array(Box::new(Type::Named("T".to_string())));
        let substituted_array = env.substitute_type(&array_type);
        assert_eq!(substituted_array, Type::Array(Box::new(Type::I32)));
    }

    #[test]
    fn test_trait_implementation_checking() {
        let env = GenericEnv::new();

        // Basic types implement Eq
        assert!(env.implements_trait(&Type::I32, &BuiltinTrait::Eq));
        assert!(env.implements_trait(&Type::String, &BuiltinTrait::Eq));

        // Numeric types implement Ord
        assert!(env.implements_trait(&Type::I32, &BuiltinTrait::Ord));
        assert!(!env.implements_trait(&Type::String, &BuiltinTrait::Ord));

        // Arrays implement traits if elements do
        let int_array = Type::Array(Box::new(Type::I32));
        assert!(env.implements_trait(&int_array, &BuiltinTrait::Eq));
        assert!(env.implements_trait(&int_array, &BuiltinTrait::Ord));
    }

    #[test]
    fn test_constraint_checking() {
        let mut env = GenericEnv::new();
        env.add_substitution("T".to_string(), Type::I32);

        let bound = TraitBound::builtin(BuiltinTrait::Eq, test_span());
        let constraints = vec![("T".to_string(), vec![bound])];

        let result = env.check_constraints(&constraints);
        assert!(result.satisfied);
        assert!(result.missing.is_empty());

        // Test unsatisfied constraint
        let ord_bound = TraitBound::builtin(BuiltinTrait::Ord, test_span());
        env.add_substitution("U".to_string(), Type::String);
        let constraints = vec![("U".to_string(), vec![ord_bound])];

        let result = env.check_constraints(&constraints);
        assert!(!result.satisfied);
        assert_eq!(result.missing.len(), 1);
    }

    #[test]
    fn test_generic_params_display() {
        let param1 = TypeParam::new("T".to_string(), test_span())
            .with_bound(TraitBound::builtin(BuiltinTrait::Eq, test_span()));
        let param2 = TypeParam::new("U".to_string(), test_span())
            .with_bound(TraitBound::builtin(BuiltinTrait::Ord, test_span()))
            .with_bound(TraitBound::builtin(BuiltinTrait::Clone, test_span()));

        let params = GenericParams::new(vec![param1, param2], test_span());
        let display = format!("{}", params);
        assert_eq!(display, "<T: Eq, U: Ord + Clone>");
    }
}
