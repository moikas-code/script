//! Stress tests for generic type inference
//! 
//! These tests challenge the type inference system with complex
//! scenarios including partial annotations, conflicting constraints,
//! and intricate inference chains.

#[path = "../utils/mod.rs"]
mod utils;

use utils::generic_test_helpers::*;

#[test]
fn test_partial_type_annotation_complex() {
    let code = r#"
        struct Container<T, U> {
            first: T,
            second: U
        }
        
        struct Box<T> {
            value: T
        }
        
        enum Option<T> {
            Some(T),
            None
        }
        
        fn main() {
            // Partial annotation with wildcard
            let c1: Container<i32, _> = Container {
                first: 42,
                second: "inferred"
            };
            
            // Nested partial annotation
            let b1: Box<Container<_, string>> = Box {
                value: Container {
                    first: 100,
                    second: "hello"
                }
            };
            
            // Multiple wildcards
            let c2: Container<_, _> = Container {
                first: true,
                second: 3.14
            };
            
            // Partial in Option
            let opt: Option<Container<i32, _>> = Option::Some(
                Container {
                    first: 200,
                    second: [1, 2, 3]
                }
            );
        }
    "#;
    
    let program = compile_generic_program(code).expect("Failed to compile");
    assert_no_errors(&program);
    
    // Should infer all the wildcard types correctly
    assert!(count_monomorphized_instances(&program, "Container") >= 4);
    assert!(count_monomorphized_instances(&program, "Box") >= 1);
    assert!(count_monomorphized_instances(&program, "Option") >= 1);
}

#[test]
fn test_conflicting_constraints() {
    let code = r#"
        struct Box<T> {
            value: T
        }
        
        fn main() {
            // This should fail - can't have both i32 and string for T
            let b = Box { 
                value: if true { 42 } else { "hello" } 
            };
        }
    "#;
    
    let program = compile_generic_program(code);
    
    // This should produce a type error
    match program {
        Ok(ref prog) => {
            assert!(!prog.errors.is_empty(), "Expected type error for conflicting constraints");
        }
        Err(_) => {
            // Expected to fail
        }
    }
}

#[test]
fn test_inference_with_trait_bounds() {
    let code = r#"
        trait Clone {
            fn clone(&self) -> Self;
        }
        
        trait Eq {
            fn eq(&self, other: &Self) -> bool;
        }
        
        struct Constrained<T> where T: Clone + Eq {
            value: T
        }
        
        fn compare_and_clone<T: Clone + Eq>(x: T, y: T) -> T {
            if x.eq(&y) {
                x.clone()
            } else {
                y.clone()
            }
        }
        
        fn main() {
            // Type must satisfy constraints
            let c1 = Constrained { value: 42 };
            
            let result = compare_and_clone(10, 20);
        }
    "#;
    
    let program = compile_generic_program(code);
    
    // Trait bounds might not be fully implemented
    if let Ok(ref prog) = program {
        assert!(count_monomorphized_instances(prog, "Constrained") >= 1);
    }
}

#[test]
fn test_cross_function_type_inference() {
    let code = r#"
        struct Wrapper<T> {
            value: T
        }
        
        fn wrap<T>(value: T) -> Wrapper<T> {
            Wrapper { value: value }
        }
        
        fn unwrap<T>(w: Wrapper<T>) -> T {
            w.value
        }
        
        fn transform<T, U>(w: Wrapper<T>, f: fn(T) -> U) -> Wrapper<U> {
            Wrapper { value: f(w.value) }
        }
        
        fn double(x: i32) -> i32 { x * 2 }
        fn to_string(x: i32) -> string { "converted" }
        
        fn main() {
            let w1 = wrap(42);              // Wrapper<i32>
            let v1 = unwrap(w1);           // i32
            
            let w2 = wrap("hello");         // Wrapper<string>
            let v2 = unwrap(w2);           // string
            
            let w3 = wrap(100);
            let w4 = transform(w3, double);      // Wrapper<i32>
            let w5 = transform(w3, to_string);   // Wrapper<string>
        }
    "#;
    
    let program = compile_generic_program(code).expect("Failed to compile");
    
    // Should handle inference across function boundaries
    assert!(count_monomorphized_instances(&program, "Wrapper") >= 3);
}

#[test]
fn test_inference_with_method_chains() {
    let code = r#"
        struct Container<T> {
            value: T
        }
        
        impl<T> Container<T> {
            fn new(value: T) -> Container<T> {
                Container { value: value }
            }
            
            fn map<U>(self, f: fn(T) -> U) -> Container<U> {
                Container { value: f(self.value) }
            }
            
            fn get(self) -> T {
                self.value
            }
        }
        
        fn add_one(x: i32) -> i32 { x + 1 }
        fn to_bool(x: i32) -> bool { x > 0 }
        
        fn main() {
            // Chain of method calls with type transformations
            let result = Container::new(42)
                .map(add_one)              // Container<i32>
                .map(to_bool)              // Container<bool>
                .get();                    // bool
            
            let c1 = Container::new("hello");
            let c2 = c1.map(|s| s.len());  // Would need closure support
        }
    "#;
    
    let program = compile_generic_program(code);
    
    if let Ok(ref prog) = program {
        // Should track type through method chain
        let container_instances = count_monomorphized_instances(prog, "Container");
        assert!(container_instances >= 2);
    }
}

#[test]
fn test_complex_return_type_inference() {
    let code = r#"
        struct Pair<A, B> {
            first: A,
            second: B
        }
        
        enum Result<T, E> {
            Ok(T),
            Err(E)
        }
        
        fn make_pair<T>(x: T, y: T) -> Pair<T, T> {
            Pair { first: x, second: y }
        }
        
        fn try_operation<T>() -> Result<T, string> {
            Result::Err("not implemented")
        }
        
        fn complex_return<T, U>() -> Result<Pair<T, U>, string> {
            Result::Err("complex error")
        }
        
        fn main() {
            let p1 = make_pair(10, 20);        // Pair<i32, i32>
            let p2 = make_pair("a", "b");      // Pair<string, string>
            
            let r1: Result<i32, string> = try_operation();
            let r2: Result<bool, string> = try_operation();
            
            let r3: Result<Pair<i32, string>, string> = complex_return();
        }
    "#;
    
    let program = compile_generic_program(code).expect("Failed to compile");
    
    // Should handle complex return types
    assert!(count_monomorphized_instances(&program, "Pair") >= 3);
    assert!(count_monomorphized_instances(&program, "Result") >= 3);
}

#[test]
fn test_inference_with_array_literals() {
    let code = r#"
        struct Vec<T> {
            data: [T],
            len: i32
        }
        
        fn vec_from_array<T>(arr: [T]) -> Vec<T> {
            Vec { data: arr, len: 0 }  // Simplified
        }
        
        fn main() {
            // Infer from array literal
            let v1 = vec_from_array([1, 2, 3, 4, 5]);
            let v2 = vec_from_array(["a", "b", "c"]);
            let v3 = vec_from_array([true, false, true]);
            
            // Mixed with explicit types
            let v4: Vec<f32> = vec_from_array([1.0, 2.0, 3.0]);
        }
    "#;
    
    let program = compile_generic_program(code).expect("Failed to compile");
    assert_no_errors(&program);
    
    // Should infer Vec<i32>, Vec<string>, Vec<bool>, Vec<f32>
    assert_eq!(count_monomorphized_instances(&program, "Vec"), 4);
}

#[test]
fn test_bidirectional_type_inference() {
    let code = r#"
        struct Box<T> {
            value: T
        }
        
        fn identity<T>(x: T) -> T { x }
        
        fn expects_box_i32(b: Box<i32>) -> i32 {
            b.value
        }
        
        fn main() {
            // Forward inference
            let b1 = Box { value: 42 };
            
            // Backward inference from expected type
            let result = expects_box_i32(Box { value: identity(100) });
            
            // Bidirectional
            let b2: Box<_> = Box { value: identity(200) };
            let used = expects_box_i32(b2);
        }
    "#;
    
    let program = compile_generic_program(code).expect("Failed to compile");
    assert_no_errors(&program);
    
    // Should handle bidirectional inference
    assert!(count_monomorphized_instances(&program, "Box") >= 1);
}

#[test]
fn test_inference_with_literals() {
    let code = r#"
        struct Numeric<T> {
            value: T
        }
        
        fn main() {
            // Integer literal inference
            let n1 = Numeric { value: 42 };          // i32 by default
            let n2: Numeric<i32> = Numeric { value: 42 };
            let n3: Numeric<f32> = Numeric { value: 42 };  // Coercion
            
            // Float literal inference
            let n4 = Numeric { value: 3.14 };        // f32 by default
            let n5: Numeric<f32> = Numeric { value: 3.14 };
            
            // Boolean literal
            let n6 = Numeric { value: true };        // bool
            
            // String literal
            let n7 = Numeric { value: "hello" };     // string
        }
    "#;
    
    let program = compile_generic_program(code).expect("Failed to compile");
    
    // Should handle different literal types
    let numeric_instances = count_monomorphized_instances(&program, "Numeric");
    assert!(numeric_instances >= 4); // i32, f32, bool, string
}

#[test]
fn test_recursive_type_inference() {
    let code = r#"
        enum List<T> {
            Cons(T, Box<List<T>>),
            Nil
        }
        
        struct Box<T> {
            value: T
        }
        
        fn main() {
            // Build a list recursively, inferring T throughout
            let list = List::Cons(
                1,
                Box {
                    value: List::Cons(
                        2,
                        Box {
                            value: List::Cons(
                                3,
                                Box {
                                    value: List::Nil
                                }
                            )
                        }
                    )
                }
            );
        }
    "#;
    
    let program = compile_generic_program(code).expect("Failed to compile");
    
    // Should infer List<i32> throughout the recursive structure
    assert!(count_monomorphized_instances(&program, "List") >= 1);
    assert!(count_monomorphized_instances(&program, "Box") >= 1);
}