/// Tests for the generic type system
/// These tests verify the core functionality of generics before full parser integration

#[cfg(test)]
mod tests {
    use super::super::generics::*;
    use super::super::Type;
    use crate::source::{SourceLocation, Span};

    fn test_span() -> Span {
        Span::new(SourceLocation::new(1, 1, 0), SourceLocation::new(1, 10, 10))
    }

    #[test]
    fn test_generic_type_creation() {
        // Test Vec<i32>
        let vec_i32 = Type::Generic {
            name: "Vec".to_string(),
            args: vec![Type::I32],
        };
        
        assert_eq!(vec_i32.to_string(), "Vec<i32>");
        
        // Test Map<string, i32>
        let map_str_int = Type::Generic {
            name: "Map".to_string(),
            args: vec![Type::String, Type::I32],
        };
        
        assert_eq!(map_str_int.to_string(), "Map<string, i32>");
    }

    #[test]
    fn test_generic_type_equality() {
        let vec1 = Type::Generic {
            name: "Vec".to_string(),
            args: vec![Type::I32],
        };
        
        let vec2 = Type::Generic {
            name: "Vec".to_string(),
            args: vec![Type::I32],
        };
        
        let vec3 = Type::Generic {
            name: "Vec".to_string(),
            args: vec![Type::String],
        };
        
        assert!(vec1.equals(&vec2));
        assert!(!vec1.equals(&vec3));
    }

    #[test]
    fn test_type_param_creation() {
        let param = TypeParam::new("T".to_string(), test_span());
        assert_eq!(param.name, "T");
        assert!(param.bounds.is_empty());
    }

    #[test]
    fn test_type_param_with_bounds() {
        let eq_bound = TraitBound::builtin(BuiltinTrait::Eq, test_span());
        let ord_bound = TraitBound::builtin(BuiltinTrait::Ord, test_span());
        
        let param = TypeParam::new("T".to_string(), test_span())
            .with_bound(eq_bound)
            .with_bound(ord_bound);
        
        assert_eq!(param.bounds.len(), 2);
        assert!(param.has_bound("Eq"));
        assert!(param.has_bound("Ord"));
        assert!(!param.has_bound("Clone"));
    }

    #[test]
    fn test_generic_params_display() {
        let param1 = TypeParam::new("T".to_string(), test_span())
            .with_bound(TraitBound::builtin(BuiltinTrait::Eq, test_span()));
        
        let param2 = TypeParam::new("U".to_string(), test_span())
            .with_bound(TraitBound::builtin(BuiltinTrait::Ord, test_span()))
            .with_bound(TraitBound::builtin(BuiltinTrait::Clone, test_span()));
        
        let params = GenericParams::new(vec![param1, param2], test_span());
        let display = format!("{params}"));
        assert_eq!(display, "<T: Eq, U: Ord + Clone>");
    }

    #[test]
    fn test_builtin_trait_dependencies() {
        assert!(BuiltinTrait::Ord.depends_on(&BuiltinTrait::Eq));
        assert!(BuiltinTrait::Copy.depends_on(&BuiltinTrait::Clone));
        assert!(!BuiltinTrait::Eq.depends_on(&BuiltinTrait::Ord));
        
        let ord_deps = BuiltinTrait::Ord.dependencies();
        assert_eq!(ord_deps, vec![BuiltinTrait::Eq]);
    }

    #[test]
    fn test_generic_env_basic() {
        let mut env = GenericEnv::new();
        
        // Test built-in implementations
        assert!(env.implements_trait(&Type::I32, &BuiltinTrait::Eq));
        assert!(env.implements_trait(&Type::I32, &BuiltinTrait::Ord));
        assert!(env.implements_trait(&Type::String, &BuiltinTrait::Eq));
        assert!(!env.implements_trait(&Type::String, &BuiltinTrait::Ord));
    }

    #[test]
    fn test_generic_env_substitution() {
        let mut env = GenericEnv::new();
        env.add_substitution("T".to_string(), Type::I32);
        
        // Test simple substitution
        let type_param = Type::TypeParam("T".to_string());
        let substituted = env.substitute_type(&type_param);
        assert_eq!(substituted, Type::I32);
        
        // Test complex substitution
        let generic_type = Type::Generic {
            name: "Vec".to_string(),
            args: vec![Type::TypeParam("T".to_string())],
        };
        
        let substituted_generic = env.substitute_type(&generic_type);
        assert_eq!(substituted_generic, Type::Generic {
            name: "Vec".to_string(),
            args: vec![Type::I32],
        });
    }

    #[test]
    fn test_trait_implementation_structural() {
        let env = GenericEnv::new();
        
        // Arrays implement traits if their elements do
        let int_array = Type::Array(Box::new(Type::I32));
        assert!(env.implements_trait(&int_array, &BuiltinTrait::Eq));
        assert!(env.implements_trait(&int_array, &BuiltinTrait::Ord));
        
        // Options implement traits if their inner type does
        let int_option = Type::Option(Box::new(Type::I32));
        assert!(env.implements_trait(&int_option, &BuiltinTrait::Eq));
        assert!(env.implements_trait(&int_option, &BuiltinTrait::Ord));
    }

    #[test]
    fn test_constraint_checking() {
        let mut env = GenericEnv::new();
        env.add_substitution("T".to_string(), Type::I32);
        env.add_substitution("U".to_string(), Type::String);
        
        let constraints = vec![
            ("T".to_string(), vec![TraitBound::builtin(BuiltinTrait::Eq, test_span())]),
            ("U".to_string(), vec![TraitBound::builtin(BuiltinTrait::Eq, test_span())]),
        ];
        
        let result = env.check_constraints(&constraints);
        assert!(result.satisfied);
        assert!(result.missing.is_empty());
        
        // Test unsatisfied constraint
        let ord_constraints = vec![
            ("U".to_string(), vec![TraitBound::builtin(BuiltinTrait::Ord, test_span())]),
        ];
        
        let result = env.check_constraints(&ord_constraints);
        assert!(!result.satisfied);
        assert_eq!(result.missing.len(), 1);
        assert_eq!(result.missing[0].trait_, BuiltinTrait::Ord);
    }

    #[test]
    fn test_generic_type_instantiation() {
        let mut env = GenericEnv::new();
        
        // Define a generic type Vec<T>
        let vec_def = GenericTypeDefinition {
            name: "Vec".to_string(),
            params: vec![TypeParam::new("T".to_string(), test_span())],
            body: GenericTypeBody::Struct {
                fields: vec![FieldDef {
                    name: "data".to_string(),
                    type_: Type::Array(Box::new(Type::TypeParam("T".to_string()))),
                    span: test_span(),
                }],
            },
            span: test_span(),
        };
        
        env.define_generic_type(vec_def);
        
        // Instantiate Vec<i32>
        let vec_i32 = env.instantiate_generic("Vec", vec![Type::I32]);
        assert!(vec_i32.is_some());
        assert_eq!(vec_i32.unwrap().to_string(), "Vec<i32>");
        
        // Try invalid instantiation (wrong number of args)
        let invalid = env.instantiate_generic("Vec", vec![Type::I32, Type::String]);
        assert!(invalid.is_none());
    }

    #[test]
    fn test_complex_generic_constraints() {
        let mut env = GenericEnv::new();
        env.add_substitution("T".to_string(), Type::I32);
        
        // Define a type parameter with multiple bounds
        let multi_bound_param = TypeParam::new("T".to_string(), test_span())
            .with_bound(TraitBound::builtin(BuiltinTrait::Eq, test_span()))
            .with_bound(TraitBound::builtin(BuiltinTrait::Ord, test_span()))
            .with_bound(TraitBound::builtin(BuiltinTrait::Clone, test_span()));
        
        let constraints = vec![
            ("T".to_string(), multi_bound_param.bounds),
        ];
        
        let result = env.check_constraints(&constraints);
        assert!(result.satisfied);
    }

    #[test]
    fn test_trait_name_parsing() {
        assert_eq!(BuiltinTrait::from_name("Eq"), Some(BuiltinTrait::Eq));
        assert_eq!(BuiltinTrait::from_name("Ord"), Some(BuiltinTrait::Ord));
        assert_eq!(BuiltinTrait::from_name("Clone"), Some(BuiltinTrait::Clone));
        assert_eq!(BuiltinTrait::from_name("Display"), Some(BuiltinTrait::Display));
        assert_eq!(BuiltinTrait::from_name("Unknown"), None);
    }

    #[test]
    fn test_type_param_display() {
        let type_ = Type::TypeParam("T".to_string());
        assert_eq!(type_.to_string(), "T");
        
        let generic = Type::Generic {
            name: "Container".to_string(),
            args: vec![Type::TypeParam("T".to_string())],
        };
        assert_eq!(generic.to_string(), "Container<T>");
    }

    #[test]
    fn test_nested_generic_types() {
        // Test Vec<Option<i32>>
        let nested = Type::Generic {
            name: "Vec".to_string(),
            args: vec![Type::Generic {
                name: "Option".to_string(),
                args: vec![Type::I32],
            }],
        };
        
        assert_eq!(nested.to_string(), "Vec<Option<i32>>");
        
        // Test equality of nested types
        let nested2 = Type::Generic {
            name: "Vec".to_string(),
            args: vec![Type::Generic {
                name: "Option".to_string(),
                args: vec![Type::I32],
            }],
        };
        
        assert!(nested.equals(&nested2));
    }

    #[test]
    fn test_generic_function_signature() {
        // Simulate a generic function signature: fn map<T, U>(arr: [T], f: (T) -> U) -> [U]
        let t_param = Type::TypeParam("T".to_string());
        let u_param = Type::TypeParam("U".to_string());
        
        let array_t = Type::Array(Box::new(t_param.clone());
        let func_t_to_u = Type::Function {
            params: vec![t_param],
            ret: Box::new(u_param.clone()),
        };
        let array_u = Type::Array(Box::new(u_param));
        
        let map_signature = Type::Function {
            params: vec![array_t, func_t_to_u],
            ret: Box::new(array_u),
        };
        
        assert_eq!(map_signature.to_string(), "([T], (T) -> U) -> [U]");
    }
}