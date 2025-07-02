/// Basic test that only tests the generics module directly
/// This file allows testing the core generics functionality
/// without depending on other parts of the codebase

#[cfg(test)]
mod basic_tests {
    use crate::types::generics::*;
    use crate::types::Type;
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
    }

    #[test]
    fn test_builtin_traits() {
        assert_eq!(BuiltinTrait::Eq.name(), "Eq");
        assert_eq!(BuiltinTrait::Ord.name(), "Ord");
        assert_eq!(BuiltinTrait::Clone.name(), "Clone");
        
        assert_eq!(BuiltinTrait::from_name("Eq"), Some(BuiltinTrait::Eq));
        assert_eq!(BuiltinTrait::from_name("Unknown"), None);
    }

    #[test]
    fn test_generic_env() {
        let env = GenericEnv::new();
        
        // Test built-in trait implementations
        assert!(env.implements_trait(&Type::I32, &BuiltinTrait::Eq));
        assert!(env.implements_trait(&Type::I32, &BuiltinTrait::Ord));
        assert!(!env.implements_trait(&Type::String, &BuiltinTrait::Ord));
    }

    #[test]
    fn test_type_param_bounds() {
        let param = TypeParam::new("T".to_string(), test_span())
            .with_bound(TraitBound::builtin(BuiltinTrait::Eq, test_span()));
        
        assert!(param.has_bound("Eq"));
        assert!(!param.has_bound("Ord"));
    }

    #[test]
    fn test_constraint_checking() {
        let mut env = GenericEnv::new();
        env.add_substitution("T".to_string(), Type::I32);
        
        let constraints = vec![
            ("T".to_string(), vec![TraitBound::builtin(BuiltinTrait::Eq, test_span())]),
        ];
        
        let result = env.check_constraints(&constraints);
        assert!(result.satisfied);
    }
}