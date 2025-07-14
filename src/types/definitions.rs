use super::{generics::GenericParams, Type};
use crate::parser::{EnumVariant, StructField, WhereClause};
use crate::source::Span;
use std::collections::HashMap;

/// Storage for type definitions in the Script language
/// This module handles the storage and retrieval of struct and enum definitions,
/// including their generic parameters and constraints.

/// A struct definition with generic parameters
#[derive(Debug, Clone)]
pub struct StructDefinition {
    /// Name of the struct
    pub name: String,
    /// Generic parameters (if any)
    pub generic_params: Option<GenericParams>,
    /// Fields of the struct
    pub fields: Vec<StructField>,
    /// Where clause for additional constraints
    pub where_clause: Option<WhereClause>,
    /// Source location
    pub span: Span,
    /// Whether this is a monomorphized instance
    pub is_monomorphized: bool,
    /// Original generic type (for monomorphized instances)
    pub original_type: Option<String>,
}

/// An enum definition with generic parameters
#[derive(Debug, Clone)]
pub struct EnumDefinition {
    /// Name of the enum
    pub name: String,
    /// Generic parameters (if any)
    pub generic_params: Option<GenericParams>,
    /// Variants of the enum
    pub variants: Vec<EnumVariant>,
    /// Where clause for additional constraints
    pub where_clause: Option<WhereClause>,
    /// Source location
    pub span: Span,
    /// Whether this is a monomorphized instance
    pub is_monomorphized: bool,
    /// Original generic type (for monomorphized instances)
    pub original_type: Option<String>,
}

/// Registry for all type definitions in a compilation unit
#[derive(Debug, Default)]
pub struct TypeDefinitionRegistry {
    /// All struct definitions, keyed by name
    structs: HashMap<String, StructDefinition>,
    /// All enum definitions, keyed by name
    enums: HashMap<String, EnumDefinition>,
    /// Monomorphized struct instances, keyed by mangled name
    monomorphized_structs: HashMap<String, StructDefinition>,
    /// Monomorphized enum instances, keyed by mangled name
    monomorphized_enums: HashMap<String, EnumDefinition>,
    /// Cache for type instantiations to avoid duplicates
    instantiation_cache: HashMap<(String, Vec<Type>), String>,
}

impl TypeDefinitionRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a struct definition
    pub fn register_struct(&mut self, def: StructDefinition) -> Result<(), String> {
        if self.structs.contains_key(&def.name) {
            return Err(format!("Struct '{}' is already defined", def.name));
        }
        self.structs.insert(def.name.clone(), def);
        Ok(())
    }

    /// Register an enum definition
    pub fn register_enum(&mut self, def: EnumDefinition) -> Result<(), String> {
        if self.enums.contains_key(&def.name) {
            return Err(format!("Enum '{}' is already defined", def.name));
        }
        self.enums.insert(def.name.clone(), def);
        Ok(())
    }

    /// Get a struct definition by name
    pub fn get_struct(&self, name: &str) -> Option<&StructDefinition> {
        self.structs
            .get(name)
            .or_else(|| self.monomorphized_structs.get(name))
    }

    /// Get an enum definition by name
    pub fn get_enum(&self, name: &str) -> Option<&EnumDefinition> {
        self.enums
            .get(name)
            .or_else(|| self.monomorphized_enums.get(name))
    }

    /// Check if a struct needs monomorphization
    pub fn struct_needs_monomorphization(&self, name: &str) -> bool {
        self.structs
            .get(name)
            .map(|def| def.generic_params.is_some() && !def.is_monomorphized)
            .unwrap_or(false)
    }

    /// Check if an enum needs monomorphization
    pub fn enum_needs_monomorphization(&self, name: &str) -> bool {
        self.enums
            .get(name)
            .map(|def| def.generic_params.is_some() && !def.is_monomorphized)
            .unwrap_or(false)
    }

    /// Generate a mangled name for a monomorphized type
    pub fn mangle_type_name(base_name: &str, type_args: &[Type]) -> String {
        let mut name = base_name.to_string();
        if !type_args.is_empty() {
            name.push('_');
            for (i, ty) in type_args.iter().enumerate() {
                if i > 0 {
                    name.push('_');
                }
                name.push_str(&mangle_type(ty));
            }
        }
        name
    }

    /// Check if a type instantiation has been cached
    pub fn get_cached_instantiation(&self, base_name: &str, type_args: &[Type]) -> Option<&str> {
        self.instantiation_cache
            .get(&(base_name.to_string(), type_args.to_vec()))
            .map(|s| s.as_str())
    }

    /// Cache a type instantiation
    pub fn cache_instantiation(
        &mut self,
        base_name: String,
        type_args: Vec<Type>,
        mangled_name: String,
    ) {
        self.instantiation_cache
            .insert((base_name, type_args), mangled_name);
    }

    /// Register a monomorphized struct
    pub fn register_monomorphized_struct(&mut self, mangled_name: String, def: StructDefinition) {
        self.monomorphized_structs.insert(mangled_name, def);
    }

    /// Register a monomorphized enum
    pub fn register_monomorphized_enum(&mut self, mangled_name: String, def: EnumDefinition) {
        self.monomorphized_enums.insert(mangled_name, def);
    }

    /// Get all struct definitions (generic and monomorphized)
    pub fn all_structs(&self) -> impl Iterator<Item = (&String, &StructDefinition)> {
        self.structs.iter().chain(self.monomorphized_structs.iter())
    }

    /// Get all enum definitions (generic and monomorphized)
    pub fn all_enums(&self) -> impl Iterator<Item = (&String, &EnumDefinition)> {
        self.enums.iter().chain(self.monomorphized_enums.iter())
    }
}

/// Mangle a type into a string suitable for use in identifiers
fn mangle_type(ty: &Type) -> String {
    match ty {
        Type::I32 => "i32".to_string(),
        Type::F32 => "f32".to_string(),
        Type::Bool => "bool".to_string(),
        Type::String => "string".to_string(),
        Type::Unknown => "unknown".to_string(),
        Type::Never => "never".to_string(),
        Type::Array(elem) => format!("array_{}", mangle_type(elem)),
        Type::Option(inner) => format!("option_{}", mangle_type(inner)),
        Type::Result { ok, err } => format!("result_{}_{}", mangle_type(ok), mangle_type(err)),
        Type::Future(inner) => format!("future_{}", mangle_type(inner)),
        Type::Named(name) => name.replace("::", "_"),
        Type::TypeVar(id) => format!("var{id}"),
        Type::TypeParam(name) => format!("param_{name}"),
        Type::Generic { name, args } => {
            let mut result = name.clone();
            if !args.is_empty() {
                result.push('_');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        result.push('_');
                    }
                    result.push_str(&mangle_type(arg));
                }
            }
            result
        }
        Type::Function { params, ret } => {
            let mut result = "fn".to_string();
            for param in params {
                result.push('_');
                result.push_str(&mangle_type(param));
            }
            result.push_str("_ret_");
            result.push_str(&mangle_type(ret));
            result
        }
        Type::Tuple(types) => {
            let mut result = "tuple".to_string();
            for ty in types {
                result.push('_');
                result.push_str(&mangle_type(ty));
            }
            result
        }
        Type::Reference { mutable, inner } => {
            if *mutable {
                format!("refmut_{}", mangle_type(inner))
            } else {
                format!("ref_{}", mangle_type(inner))
            }
        }
        Type::Struct { name, .. } => format!("struct_{}", name.replace("::", "_")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceLocation;

    fn dummy_span() -> Span {
        Span::new(SourceLocation::new(1, 1, 0), SourceLocation::new(1, 1, 0))
    }

    #[test]
    fn test_mangle_type_name() {
        let mangled = TypeDefinitionRegistry::mangle_type_name("Vec", &[Type::I32]);
        assert_eq!(mangled, "Vec_i32");

        let mangled =
            TypeDefinitionRegistry::mangle_type_name("HashMap", &[Type::String, Type::I32]);
        assert_eq!(mangled, "HashMap_string_i32");

        let mangled = TypeDefinitionRegistry::mangle_type_name(
            "Option",
            &[Type::Generic {
                name: "Vec".to_string(),
                args: vec![Type::I32],
            }],
        );
        assert_eq!(mangled, "Option_Vec_i32");
    }

    #[test]
    fn test_registry_operations() {
        let mut registry = TypeDefinitionRegistry::new();

        let struct_def = StructDefinition {
            name: "Vec".to_string(),
            generic_params: None, // Would be Some for real generic struct
            fields: vec![],
            where_clause: None,
            span: dummy_span(),
            is_monomorphized: false,
            original_type: None,
        };

        registry.register_struct(struct_def).unwrap();
        assert!(registry.get_struct("Vec").is_some());
        assert!(registry.get_struct("NonExistent").is_none());

        // Test duplicate registration
        let duplicate = StructDefinition {
            name: "Vec".to_string(),
            generic_params: None,
            fields: vec![],
            where_clause: None,
            span: dummy_span(),
            is_monomorphized: false,
            original_type: None,
        };

        assert!(registry.register_struct(duplicate).is_err());
    }
}
