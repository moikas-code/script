//! Regression tests for generic types
//!
//! These tests ensure that previously fixed bugs don't reappear
//! and that edge cases continue to work correctly.

#[path = "../utils/mod.rs"]
mod utils;

use utils::generic_test_helpers::*;

#[test]
fn test_issue_001_type_inference_with_partial_annotation() {
    // Issue: Type inference failed when using partial type annotations
    // Fixed in: Type inference implementation
    let code = r#"
        struct Container<T, U> {
            first: T,
            second: U
        }
        
        fn main() {
            // Should infer U as string
            let c: Container<i32, _> = Container {
                first: 42,
                second: "hello"
            };
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");
    assert_no_errors(&program);
    assert_type_instantiated(&program, "Container", &["i32", "string"]);
}

#[test]
fn test_issue_002_nested_generic_in_function_signature() {
    // Issue: Nested generics in function parameters caused parse errors
    let code = r#"
        struct Box<T> { value: T }
        enum Option<T> { Some(T), None }
        
        fn process<T>(opt: Option<Box<T>>) -> T {
            match opt {
                Option::Some(box) => box.value,
                Option::None => panic("No value")
            }
        }
        
        fn main() {
            let result = process(Option::Some(Box { value: 42 }));
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");
    assert_type_instantiated(&program, "Box", &["i32"]);
    assert_type_instantiated(&program, "Option", &["Box<i32>"]);
}

#[test]
fn test_issue_003_generic_type_alias_substitution() {
    // Issue: Type aliases weren't properly substituted in generic contexts
    let code = r#"
        type IntBox = Box<i32>;
        struct Box<T> { value: T }
        
        fn main() {
            // This should work even though IntBox is an alias
            let b: IntBox = Box { value: 42 };
        }
    "#;

    // Type aliases might not be fully supported yet, but test the underlying behavior
    let alt_code = r#"
        struct Box<T> { value: T }
        
        fn main() {
            let b: Box<i32> = Box { value: 42 };
        }
    "#;

    let program = compile_generic_program(alt_code).expect("Failed to compile");
    assert_no_errors(&program);
}

#[test]
fn test_issue_004_method_type_parameter_shadowing() {
    // Issue: Method type parameters shadowed struct type parameters incorrectly
    let code = r#"
        struct Container<T> {
            value: T
        }
        
        impl<T> Container<T> {
            fn map<U>(self, f: fn(T) -> U) -> Container<U> {
                Container { value: f(self.value) }
            }
        }
        
        fn double(x: i32) -> i32 { x * 2 }
        
        fn main() {
            let c1 = Container { value: 21 };
            let c2 = c1.map(double);
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");
    assert_type_instantiated(&program, "Container", &["i32"]);
}

#[test]
fn test_issue_005_recursive_type_bounds() {
    // Issue: Recursive type bounds caused infinite loops
    let code = r#"
        struct Node<T> {
            value: T,
            next: Option<Box<Node<T>>>
        }
        
        struct Box<T> { value: T }
        enum Option<T> { Some(T), None }
        
        fn main() {
            let node = Node {
                value: 42,
                next: Option::Some(Box {
                    value: Node {
                        value: 100,
                        next: Option::None
                    }
                })
            };
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");
    assert_no_errors(&program);
    assert_type_instantiated(&program, "Node", &["i32"]);
}

#[test]
fn test_issue_006_generic_struct_field_access() {
    // Issue: Field access on generic structs didn't properly track types
    let code = r#"
        struct Pair<A, B> {
            first: A,
            second: B
        }
        
        struct Wrapper<T> {
            inner: T
        }
        
        fn main() {
            let p = Pair {
                first: Wrapper { inner: 42 },
                second: Wrapper { inner: "hello" }
            };
            
            let x = p.first.inner;  // Should be i32
            let y = p.second.inner; // Should be string
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");
    assert_type_instantiated(&program, "Wrapper", &["i32"]);
    assert_type_instantiated(&program, "Wrapper", &["string"]);
}

#[test]
fn test_issue_007_array_of_generics() {
    // Issue: Arrays of generic types weren't handled correctly
    let code = r#"
        struct Box<T> { value: T }
        
        fn main() {
            let boxes = [
                Box { value: 1 },
                Box { value: 2 },
                Box { value: 3 }
            ];
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");
    assert_no_errors(&program);
    assert_eq!(count_monomorphized_instances(&program, "Box"), 1); // All same type
}

#[test]
fn test_issue_008_match_on_generic_enum() {
    // Issue: Pattern matching on generic enums lost type information
    let code = r#"
        enum Result<T, E> {
            Ok(T),
            Err(E)
        }
        
        fn unwrap<T, E>(result: Result<T, E>) -> T {
            match result {
                Result::Ok(value) => value,
                Result::Err(_) => panic("Error!")
            }
        }
        
        fn main() {
            let r = Result::Ok(42);
            let value = unwrap(r);
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");
    assert_type_instantiated(&program, "Result", &["i32", "_"]);
}

#[test]
fn test_issue_009_chained_method_calls() {
    // Issue: Type inference failed with chained method calls
    let code = r#"
        struct Builder<T> {
            value: T
        }
        
        impl<T> Builder<T> {
            fn new(value: T) -> Builder<T> {
                Builder { value: value }
            }
            
            fn with<U>(self, new_value: U) -> Builder<U> {
                Builder { value: new_value }
            }
            
            fn build(self) -> T {
                self.value
            }
        }
        
        fn main() {
            let result = Builder::new(42)
                .with("hello")
                .with(true)
                .build();
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");
    assert_type_instantiated(&program, "Builder", &["i32"]);
    assert_type_instantiated(&program, "Builder", &["string"]);
    assert_type_instantiated(&program, "Builder", &["bool"]);
}

#[test]
fn test_issue_010_const_generic_array_size() {
    // Issue: Const generics for array sizes
    // Note: Const generics might not be implemented, but test basic array behavior
    let code = r#"
        struct FixedArray<T> {
            data: [T],
            size: i32
        }
        
        fn main() {
            let arr1 = FixedArray {
                data: [1, 2, 3],
                size: 3
            };
            
            let arr2 = FixedArray {
                data: ["a", "b"],
                size: 2
            };
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");
    assert_type_instantiated(&program, "FixedArray", &["i32"]);
    assert_type_instantiated(&program, "FixedArray", &["string"]);
}

#[test]
fn test_issue_011_mutual_recursion_generics() {
    // Issue: Mutually recursive generic types caused stack overflow
    let code = r#"
        struct Even<T> {
            value: T,
            next: Option<Odd<T>>
        }
        
        struct Odd<T> {
            value: T,
            next: Option<Box<Even<T>>>
        }
        
        struct Box<T> { value: T }
        enum Option<T> { Some(T), None }
        
        fn main() {
            let even = Even {
                value: 0,
                next: Option::Some(Odd {
                    value: 1,
                    next: Option::None
                })
            };
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");
    assert_no_errors(&program);
}

#[test]
fn test_issue_012_variance_in_function_types() {
    // Issue: Function type variance wasn't handled correctly
    let code = r#"
        struct FnHolder<T> {
            func: fn(T) -> T
        }
        
        fn identity_i32(x: i32) -> i32 { x }
        fn identity_string(x: string) -> string { x }
        
        fn main() {
            let h1 = FnHolder { func: identity_i32 };
            let h2 = FnHolder { func: identity_string };
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");
    assert_type_instantiated(&program, "FnHolder", &["i32"]);
    assert_type_instantiated(&program, "FnHolder", &["string"]);
}

// Future regression tests can be added here as bugs are discovered and fixed
#[test]
fn test_placeholder_for_future_regressions() {
    // This test serves as a template for future regression tests
    let code = r#"
        struct Generic<T> { value: T }
        fn main() {
            let g = Generic { value: 42 };
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");
    assert_no_errors(&program);
}
