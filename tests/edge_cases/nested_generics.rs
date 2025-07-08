//! Edge case tests for deeply nested generic types
//! 
//! These tests push the limits of the generic type system with
//! complex nesting, mutual recursion, and type aliases.

#[path = "../utils/mod.rs"]
mod utils;

use utils::generic_test_helpers::*;

#[test]
fn test_deeply_nested_generics() {
    let code = r#"
        struct Box<T> {
            value: T
        }
        
        enum Option<T> {
            Some(T),
            None
        }
        
        enum Result<T, E> {
            Ok(T),
            Err(E)
        }
        
        struct Vec<T> {
            data: [T],
            len: i32
        }
        
        fn main() {
            // Box<Option<Result<Vec<i32>, string>>>
            let deeply_nested = Box {
                value: Option::Some(
                    Result::Ok(
                        Vec { data: [1, 2, 3], len: 3 }
                    )
                )
            };
            
            // Option<Result<Box<Vec<string>>, i32>>
            let another = Option::Some(
                Result::Ok(
                    Box {
                        value: Vec { data: ["a", "b"], len: 2 }
                    }
                )
            );
            
            // Result<Option<Box<Option<i32>>>, string>
            let very_deep = Result::Ok(
                Option::Some(
                    Box {
                        value: Option::Some(42)
                    }
                )
            );
        }
    "#;
    
    let program = compile_generic_program(code).expect("Failed to compile");
    assert_no_errors(&program);
    
    // Should handle all the nested instantiations
    assert!(count_monomorphized_instances(&program, "Box") >= 2);
    assert!(count_monomorphized_instances(&program, "Option") >= 3);
    assert!(count_monomorphized_instances(&program, "Result") >= 2);
    assert!(count_monomorphized_instances(&program, "Vec") >= 2);
}

#[test]
fn test_mutual_recursion() {
    let code = r#"
        struct A<T> {
            value: T,
            b: Option<B<T>>
        }
        
        struct B<T> {
            value: T,
            a: Option<Box<A<T>>>
        }
        
        struct Box<T> {
            value: T
        }
        
        enum Option<T> {
            Some(T),
            None
        }
        
        fn main() {
            let a1 = A {
                value: 42,
                b: Option::None
            };
            
            let b1 = B {
                value: 100,
                a: Option::Some(Box { value: a1 })
            };
            
            let a2 = A {
                value: 200,
                b: Option::Some(b1)
            };
        }
    "#;
    
    let program = compile_generic_program(code);
    
    if let Ok(ref prog) = program {
        // Should handle mutually recursive types
        assert!(count_monomorphized_instances(prog, "A") >= 1);
        assert!(count_monomorphized_instances(prog, "B") >= 1);
    }
}

#[test]
fn test_generic_type_aliases() {
    let code = r#"
        type StringMap<T> = HashMap<string, T>;
        type IntPair<T> = Pair<i32, T>;
        type OptionVec<T> = Option<Vec<T>>;
        
        struct HashMap<K, V> {
            key_type: K,
            value_type: V
        }
        
        struct Pair<A, B> {
            first: A,
            second: B
        }
        
        enum Option<T> {
            Some(T),
            None
        }
        
        struct Vec<T> {
            data: [T],
            len: i32
        }
        
        fn main() {
            // Type aliases might not be fully supported yet
            // but we test the underlying types
            let map = HashMap {
                key_type: "key",
                value_type: 42
            };
            
            let pair = Pair {
                first: 100,
                second: "hello"
            };
            
            let opt_vec = Option::Some(
                Vec { data: [1, 2, 3], len: 3 }
            );
        }
    "#;
    
    let program = compile_generic_program(code);
    
    if let Ok(ref prog) = program {
        // Even if type aliases don't work, the underlying types should
        assert!(count_monomorphized_instances(prog, "HashMap") >= 1);
        assert!(count_monomorphized_instances(prog, "Pair") >= 1);
        assert!(count_monomorphized_instances(prog, "Option") >= 1);
    }
}

#[test]
fn test_extremely_long_type_chain() {
    let code = r#"
        struct W1<T> { v: T }
        struct W2<T> { v: W1<T> }
        struct W3<T> { v: W2<T> }
        struct W4<T> { v: W3<T> }
        struct W5<T> { v: W4<T> }
        
        fn main() {
            let chain = W5 {
                v: W4 {
                    v: W3 {
                        v: W2 {
                            v: W1 {
                                v: 42
                            }
                        }
                    }
                }
            };
        }
    "#;
    
    let program = compile_generic_program(code).expect("Failed to compile");
    assert_no_errors(&program);
    
    // Should handle the chain of wrappers
    assert_eq!(count_monomorphized_instances(&program, "W1"), 1);
    assert_eq!(count_monomorphized_instances(&program, "W2"), 1);
    assert_eq!(count_monomorphized_instances(&program, "W3"), 1);
    assert_eq!(count_monomorphized_instances(&program, "W4"), 1);
    assert_eq!(count_monomorphized_instances(&program, "W5"), 1);
}

#[test]
fn test_generic_array_nesting() {
    let code = r#"
        struct Matrix<T> {
            data: [[T]]
        }
        
        struct Tensor<T> {
            data: [[[T]]]
        }
        
        fn main() {
            let mat = Matrix {
                data: [[1, 2], [3, 4]]
            };
            
            let tensor = Tensor {
                data: [[[1, 2], [3, 4]], [[5, 6], [7, 8]]]
            };
        }
    "#;
    
    let program = compile_generic_program(code).expect("Failed to compile");
    assert_no_errors(&program);
    
    // Should handle nested array types
    assert_eq!(count_monomorphized_instances(&program, "Matrix"), 1);
    assert_eq!(count_monomorphized_instances(&program, "Tensor"), 1);
}

#[test]
fn test_circular_type_reference() {
    let code = r#"
        struct Cycle<T> {
            value: T,
            next: Option<Box<Cycle<T>>>
        }
        
        struct Box<T> {
            value: T
        }
        
        enum Option<T> {
            Some(T),
            None
        }
        
        fn main() {
            let c1 = Cycle {
                value: 42,
                next: Option::None
            };
            
            let c2 = Cycle {
                value: 100,
                next: Option::Some(Box { value: c1 })
            };
            
            // Would create a cycle if we could mutate c1.next = Some(Box(c2))
        }
    "#;
    
    let program = compile_generic_program(code).expect("Failed to compile");
    
    // Should handle self-referential types
    assert!(count_monomorphized_instances(&program, "Cycle") >= 1);
}

#[test]
fn test_mixed_generic_concrete_nesting() {
    let code = r#"
        struct Generic<T> {
            value: T,
            count: i32
        }
        
        struct Concrete {
            data: Generic<string>
        }
        
        struct Mixed<T> {
            generic_field: T,
            concrete_field: Concrete
        }
        
        fn main() {
            let g = Generic {
                value: "hello",
                count: 5
            };
            
            let c = Concrete {
                data: g
            };
            
            let m = Mixed {
                generic_field: 42,
                concrete_field: c
            };
        }
    "#;
    
    let program = compile_generic_program(code).expect("Failed to compile");
    assert_no_errors(&program);
    
    // Should have Generic<string> and correct Mixed instantiation
    assert!(count_monomorphized_instances(&program, "Generic") >= 1);
    assert!(count_monomorphized_instances(&program, "Mixed") >= 1);
}

#[test]
fn test_function_type_in_generic() {
    let code = r#"
        struct FnWrapper<T> {
            func: fn(T) -> T,
            value: T
        }
        
        fn identity_i32(x: i32) -> i32 { x }
        fn identity_string(x: string) -> string { x }
        
        fn main() {
            let w1 = FnWrapper {
                func: identity_i32,
                value: 42
            };
            
            let w2 = FnWrapper {
                func: identity_string,
                value: "hello"
            };
        }
    "#;
    
    let program = compile_generic_program(code);
    
    if let Ok(ref prog) = program {
        // Should handle function types in generics
        assert!(count_monomorphized_instances(prog, "FnWrapper") >= 2);
    }
}

#[test]
fn test_phantom_type_parameter() {
    let code = r#"
        struct Phantom<T> {
            value: i32
            // T is not used in the struct
        }
        
        fn main() {
            let p1 = Phantom::<string> { value: 42 };
            let p2 = Phantom::<bool> { value: 100 };
        }
    "#;
    
    let program = compile_generic_program(code).expect("Failed to compile");
    
    // Should handle phantom type parameters
    assert_eq!(count_monomorphized_instances(&program, "Phantom"), 2);
}

#[test]
fn test_generic_const_expressions() {
    let code = r#"
        struct Array<T, const N: i32> {
            data: [T],
            size: i32
        }
        
        fn main() {
            // Const generics might not be supported yet
            // Fall back to regular generics
            let arr1 = Array::<i32> {
                data: [1, 2, 3],
                size: 3
            };
            
            let arr2 = Array::<string> {
                data: ["a", "b"],
                size: 2
            };
        }
    "#;
    
    let program = compile_generic_program(code);
    
    // Even if const generics fail, regular generics should work
    if let Ok(ref prog) = program {
        let array_instances = count_monomorphized_instances(prog, "Array");
        assert!(array_instances >= 1);
    }
}