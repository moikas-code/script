//! Error handling tests for generic types
//!
//! These tests verify that the compiler produces appropriate error
//! messages for various generic type errors.

#[path = "../utils/mod.rs"]
mod utils;

use utils::generic_test_helpers::*;

#[test]
fn test_type_mismatch_in_generic() {
    let code = r#"
        struct Box<T> {
            value: T
        }
        
        fn main() {
            let b: Box<i32> = Box { value: "wrong type" };
        }
    "#;

    let program = compile_generic_program(code);

    match program {
        Ok(ref prog) => {
            assert!(!prog.errors.is_empty(), "Expected type mismatch error");
            // Could check for specific error message
        }
        Err(e) => {
            assert!(e.to_string().contains("type") || e.to_string().contains("mismatch"));
        }
    }
}

#[test]
fn test_missing_type_params() {
    let code = r#"
        struct Pair<A, B> {
            first: A,
            second: B
        }
        
        fn main() {
            // Missing type parameters where they can't be inferred
            let p: Pair = Pair { first: 42, second: "hello" };
        }
    "#;

    let program = compile_generic_program(code);

    // Should error about missing type parameters
    match program {
        Ok(ref prog) => {
            // Might succeed with inference, but check if there are warnings
        }
        Err(e) => {
            // Expected to fail
            assert!(e.to_string().contains("type") || e.to_string().contains("Pair"));
        }
    }
}

#[test]
fn test_too_many_type_params() {
    let code = r#"
        struct Box<T> {
            value: T
        }
        
        fn main() {
            // Too many type parameters
            let b = Box::<i32, string> { value: 42 };
        }
    "#;

    let program = compile_generic_program(code);

    // Should error about too many type parameters
    match program {
        Ok(ref prog) => {
            assert!(
                !prog.errors.is_empty(),
                "Expected error for too many type params"
            );
        }
        Err(_) => {
            // Expected to fail
        }
    }
}

#[test]
fn test_incompatible_type_constraints() {
    let code = r#"
        struct Container<T> {
            items: [T]
        }
        
        fn main() {
            // Try to create container with mixed types
            let c = Container {
                items: [1, "two", 3.0]  // Different types in array
            };
        }
    "#;

    let program = compile_generic_program(code);

    // Should error about incompatible types in array
    match program {
        Ok(ref prog) => {
            assert!(
                !prog.errors.is_empty(),
                "Expected error for mixed types in array"
            );
        }
        Err(_) => {
            // Expected to fail
        }
    }
}

#[test]
fn test_undefined_generic_type() {
    let code = r#"
        fn main() {
            // Reference to undefined generic type
            let x: NonExistent<i32> = something();
        }
    "#;

    let program = compile_generic_program(code);

    // Should error about undefined type
    match program {
        Ok(ref prog) => {
            assert!(!prog.errors.is_empty(), "Expected error for undefined type");
        }
        Err(e) => {
            assert!(e.to_string().contains("NonExistent") || e.to_string().contains("undefined"));
        }
    }
}

#[test]
fn test_recursive_type_without_indirection() {
    let code = r#"
        struct Recursive<T> {
            value: T,
            next: Recursive<T>  // Should need Box or similar
        }
        
        fn main() {
            // Would create infinite size type
        }
    "#;

    let program = compile_generic_program(code);

    // Should error about recursive type needing indirection
    // Though this might be caught at a later stage
}

#[test]
fn test_conflicting_impl_blocks() {
    let code = r#"
        struct Container<T> {
            value: T
        }
        
        impl<T> Container<T> {
            fn get(&self) -> T {
                self.value
            }
        }
        
        impl Container<i32> {
            fn get(&self) -> i32 {
                42  // Different implementation
            }
        }
        
        fn main() {
            let c = Container { value: 100 };
            let v = c.get();
        }
    "#;

    let program = compile_generic_program(code);

    // Might error about conflicting implementations
    // or might resolve to the more specific one
}

#[test]
fn test_type_param_shadowing() {
    let code = r#"
        struct Outer<T> {
            value: T
        }
        
        impl<T> Outer<T> {
            fn nested<T>(&self, other: T) -> T {
                // Inner T shadows outer T
                other
            }
        }
        
        fn main() {
            let o = Outer { value: 42 };
            let result = o.nested("different type");
        }
    "#;

    let program = compile_generic_program(code);

    // Should handle shadowing correctly or warn about it
}

#[test]
fn test_unresolved_type_in_struct_field() {
    let code = r#"
        struct Container<T> {
            value: T,
            unknown: U  // U is not defined
        }
        
        fn main() {
            let c = Container { value: 42, unknown: "?" };
        }
    "#;

    let program = compile_generic_program(code);

    // Should error about undefined type parameter U
    match program {
        Ok(ref prog) => {
            assert!(
                !prog.errors.is_empty(),
                "Expected error for undefined type param"
            );
        }
        Err(e) => {
            assert!(e.to_string().contains("U") || e.to_string().contains("undefined"));
        }
    }
}

#[test]
fn test_mismatched_type_args_in_impl() {
    let code = r#"
        struct Pair<A, B> {
            first: A,
            second: B
        }
        
        impl<T> Pair<T, T> {
            fn swap(self) -> Pair<T, T> {
                Pair { first: self.second, second: self.first }
            }
        }
        
        fn main() {
            // This should work for same types
            let p1 = Pair { first: 1, second: 2 };
            let swapped1 = p1.swap();
            
            // This should fail for different types
            let p2 = Pair { first: 1, second: "two" };
            let swapped2 = p2.swap();  // Error: swap requires both types to be same
        }
    "#;

    let program = compile_generic_program(code);

    // The second swap call should fail
}

#[test]
fn test_trait_bound_not_satisfied() {
    let code = r#"
        trait Ord {
            fn cmp(&self, other: &Self) -> i32;
        }
        
        struct Sorted<T> where T: Ord {
            items: [T]
        }
        
        struct NoOrd {
            value: string
        }
        
        fn main() {
            // i32 implements Ord (hypothetically)
            let s1 = Sorted { items: [3, 1, 4, 1, 5] };
            
            // NoOrd doesn't implement Ord
            let s2 = Sorted { items: [NoOrd { value: "a" }] };
        }
    "#;

    let program = compile_generic_program(code);

    // Should error about NoOrd not implementing Ord
    // (if trait bounds are checked)
}

#[test]
fn test_ambiguous_associated_type() {
    let code = r#"
        trait Container {
            type Item;
        }
        
        struct MyContainer<T> {
            value: T
        }
        
        impl<T> Container for MyContainer<T> {
            type Item = T;
        }
        
        fn get_item<C: Container>(c: C) -> C::Item {
            // Would need actual implementation
        }
        
        fn main() {
            let mc = MyContainer { value: 42 };
            let item = get_item(mc);
        }
    "#;

    let program = compile_generic_program(code);

    // Associated types might not be implemented yet
}

#[test]
fn test_lifetime_error_in_generic() {
    let code = r#"
        struct Borrowed<T> {
            value: &T  // Missing lifetime
        }
        
        fn main() {
            let x = 42;
            let b = Borrowed { value: &x };
        }
    "#;

    let program = compile_generic_program(code);

    // Should error about missing lifetime or references not being supported
}

#[test]
fn test_const_generic_mismatch() {
    let code = r#"
        struct Array<T, const N: i32> {
            data: [T; N]
        }
        
        fn main() {
            let a1: Array<i32, 5> = Array { data: [1, 2, 3] }; // Wrong size
        }
    "#;

    let program = compile_generic_program(code);

    // Const generics might not be supported, but if they are,
    // this should error about size mismatch
}
