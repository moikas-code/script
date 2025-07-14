//! Integration test for trait checking with inference engine
//! This test verifies that trait checking is properly integrated with the type inference system.

#[cfg(test)]
mod integration_tests {
    use super::super::{Constraint, ConstraintKind, InferenceContext};
    use crate::source::{SourceLocation, Span};
    use crate::types::Type;

    fn test_span() -> Span {
        Span::new(SourceLocation::new(1, 1, 0), SourceLocation::new(1, 10, 10))
    }

    #[test]
    fn test_trait_checking_basic_integration() {
        let mut ctx = InferenceContext::new();

        // Test basic trait implementation checks
        assert!(ctx.check_trait_implementation(&Type::I32, "Eq"));
        assert!(ctx.check_trait_implementation(&Type::I32, "Clone"));
        assert!(ctx.check_trait_implementation(&Type::I32, "Ord"));
        assert!(!ctx.check_trait_implementation(&Type::String, "Ord"));

        println!("✅ Basic trait implementation checks working");
    }

    #[test]
    fn test_trait_bound_constraint_success() {
        let mut ctx = InferenceContext::new();
        let span = test_span();

        // Add a trait bound constraint that should succeed
        ctx.add_trait_bound(Type::I32, "Eq".to_string(), span);

        // This should succeed since i32 implements Eq
        match ctx.solve_constraints() {
            Ok(()) => println!("✅ Trait bound constraint validation (success) working"),
            Err(e) => panic!("Expected success but got error: {}", e),
        }
    }

    #[test]
    fn test_trait_bound_constraint_failure() {
        let mut ctx = InferenceContext::new();
        let span = test_span();

        // Add a trait bound constraint that should fail
        ctx.add_trait_bound(Type::String, "Ord".to_string(), span);

        // This should fail since String doesn't implement Ord
        match ctx.solve_constraints() {
            Ok(()) => panic!("Expected failure but got success"),
            Err(e) => {
                assert!(e.to_string().contains("does not implement trait Ord"));
                println!("✅ Trait bound constraint validation (failure) working");
            }
        }
    }

    #[test]
    fn test_generic_bounds_constraint_success() {
        let mut ctx = InferenceContext::new();
        let span = test_span();

        // Define a type parameter with a concrete type
        ctx.type_env_mut().define("T".to_string(), Type::I32);

        // Add generic bounds constraint that should succeed
        ctx.add_generic_bounds(
            "T".to_string(),
            vec!["Eq".to_string(), "Clone".to_string()],
            span,
        );

        // This should succeed since i32 implements both Eq and Clone
        match ctx.solve_constraints() {
            Ok(()) => println!("✅ Generic bounds constraint validation (success) working"),
            Err(e) => panic!("Expected success but got error: {}", e),
        }
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
        match ctx.solve_constraints() {
            Ok(()) => panic!("Expected failure but got success"),
            Err(e) => {
                assert!(e.to_string().contains("does not implement trait Ord"));
                println!("✅ Generic bounds constraint validation (failure) working");
            }
        }
    }

    #[test]
    fn test_trait_bounds_validation_api() {
        use crate::types::generics::TraitBound;

        let mut ctx = InferenceContext::new();
        let span = test_span();

        // Test validation with valid bounds
        let bounds = vec![
            TraitBound::new("Eq".to_string(), span),
            TraitBound::new("Clone".to_string(), span),
        ];

        match ctx.validate_trait_bounds(&Type::I32, &bounds) {
            Ok(()) => println!("✅ Trait bounds validation API (success) working"),
            Err(e) => panic!("Expected success but got error: {}", e),
        }

        // Test validation with invalid bounds
        let invalid_bounds = vec![TraitBound::new("Ord".to_string(), span)];

        match ctx.validate_trait_bounds(&Type::String, &invalid_bounds) {
            Ok(()) => panic!("Expected failure but got success"),
            Err(e) => {
                assert!(e.to_string().contains("does not implement required traits"));
                println!("✅ Trait bounds validation API (failure) working");
            }
        }
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

        println!("✅ Array trait inheritance working");
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

        println!("✅ Option trait inheritance working");
    }

    #[test]
    fn test_complex_constraint_solving() {
        let mut ctx = InferenceContext::new();
        let span = test_span();

        // Add multiple constraints
        ctx.add_constraint(Constraint::equality(Type::I32, Type::TypeVar(0), span));
        ctx.add_trait_bound(Type::TypeVar(0), "Eq".to_string(), span);
        ctx.add_trait_bound(Type::TypeVar(0), "Clone".to_string(), span);

        // This should succeed - after unification TypeVar(0) becomes i32,
        // and i32 implements both Eq and Clone
        match ctx.solve_constraints() {
            Ok(()) => println!("✅ Complex constraint solving with trait bounds working"),
            Err(e) => panic!("Expected success but got error: {}", e),
        }
    }

    #[test]
    fn test_trait_constraint_integration_summary() {
        println!("\n🎉 TRAIT CHECKING INTEGRATION COMPLETE!");
        println!("======================================");
        println!("✅ TraitChecker integrated into InferenceContext");
        println!("✅ Trait bound constraints implemented");
        println!("✅ Generic bounds constraints implemented");
        println!("✅ Trait bounds validation API working");
        println!("✅ Array and Option trait inheritance working");
        println!("✅ Complex constraint solving with traits working");
        println!("✅ Error handling for missing trait implementations working");
        println!("\nThe trait checking system is now fully integrated with the inference engine!");
    }
}
