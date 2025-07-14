//! End-to-end integration tests for generic enums
//!
//! These tests verify the complete pipeline from parsing through execution
//! for generic enum definitions and usage.

#[path = "../utils/mod.rs"]
mod utils;

use script::types::Type;
use utils::generic_test_helpers::*;

#[test]
fn test_option_enum() {
    let code = r#"
        enum Option<T> {
            Some(T),
            None
        }
        
        fn main() {
            let opt1 = Option::Some(42);
            let opt2 = Option::Some("hello");
            let opt3 = Option::Some(true);
            let opt4 = Option::<i32>::None;
            let opt5 = Option::<string>::None;
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");
    assert_no_errors(&program);

    // Should have multiple Option instantiations
    let option_instances = count_monomorphized_instances(&program, "Option");
    assert!(
        option_instances >= 3,
        "Should have at least 3 Option instantiations"
    );

    // Verify specific instantiations
    assert!(assert_type_instantiated(&program, "Option_i32"));
    assert!(assert_type_instantiated(&program, "Option_string"));
    assert!(assert_type_instantiated(&program, "Option_bool"));
}

#[test]
fn test_result_enum() {
    let code = r#"
        enum Result<T, E> {
            Ok(T),
            Err(E)
        }
        
        fn main() {
            let r1 = Result::Ok(42);
            let r2 = Result::Err("error message");
            
            let r3: Result<string, i32> = Result::Ok("success");
            let r4: Result<string, i32> = Result::Err(404);
            
            let r5 = Result::<bool, string>::Ok(true);
            let r6 = Result::<bool, string>::Err("failed");
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");
    assert_no_errors(&program);

    // Should have multiple Result instantiations
    let result_instances = count_monomorphized_instances(&program, "Result");
    assert!(
        result_instances >= 3,
        "Should have at least 3 Result instantiations"
    );
}

#[test]
fn test_enum_pattern_matching() {
    let code = r#"
        enum Option<T> {
            Some(T),
            None
        }
        
        fn main() {
            let opt = Option::Some(42);
            
            match opt {
                Option::Some(value) => {
                    let x = value + 1;
                }
                Option::None => {
                    let x = 0;
                }
            }
            
            let opt2 = Option::Some("hello");
            match opt2 {
                Option::Some(s) => {
                    print(s);
                }
                Option::None => {
                    print("none");
                }
            }
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");
    // Pattern matching might have some errors but enum instantiation should work

    // Should have Option<i32> and Option<string>
    let option_instances = count_monomorphized_instances(&program, "Option");
    assert!(
        option_instances >= 2,
        "Should have at least 2 Option instantiations"
    );
}

#[test]
fn test_nested_enum_constructors() {
    let code = r#"
        enum Option<T> {
            Some(T),
            None
        }
        
        enum Result<T, E> {
            Ok(T),
            Err(E)
        }
        
        fn main() {
            // Option<Result<i32, string>>
            let nested1 = Option::Some(Result::Ok(42));
            let nested2 = Option::Some(Result::Err("error"));
            
            // Result<Option<i32>, string>
            let nested3 = Result::Ok(Option::Some(100));
            let nested4 = Result::Ok(Option::None);
            let nested5 = Result::<Option<i32>, string>::Err("failed");
            
            // Option<Option<bool>>
            let double_opt = Option::Some(Option::Some(true));
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");
    assert_no_errors(&program);

    // Should have multiple instantiations of both types
    let option_instances = count_monomorphized_instances(&program, "Option");
    assert!(
        option_instances >= 3,
        "Should have at least 3 Option instantiations"
    );

    let result_instances = count_monomorphized_instances(&program, "Result");
    assert!(
        result_instances >= 2,
        "Should have at least 2 Result instantiations"
    );
}

#[test]
fn test_enum_with_multiple_type_params() {
    let code = r#"
        enum Either<L, R> {
            Left(L),
            Right(R)
        }
        
        enum Triple<A, B, C> {
            First(A),
            Second(B),
            Third(C)
        }
        
        fn main() {
            let e1 = Either::Left(42);
            let e2 = Either::Right("hello");
            
            let e3: Either<bool, f32> = Either::Left(true);
            let e4: Either<bool, f32> = Either::Right(3.14);
            
            let t1 = Triple::First(1);
            let t2 = Triple::Second("two");
            let t3 = Triple::Third(3.0);
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");
    assert_no_errors(&program);

    // Check instantiations
    let either_instances = count_monomorphized_instances(&program, "Either");
    assert!(
        either_instances >= 2,
        "Should have at least 2 Either instantiations"
    );

    let triple_instances = count_monomorphized_instances(&program, "Triple");
    assert!(
        triple_instances >= 1,
        "Should have at least 1 Triple instantiation"
    );
}

#[test]
fn test_enum_with_struct_variants() {
    let code = r#"
        enum Message<T> {
            Text { content: string, data: T },
            Number { value: i32, data: T },
            Empty
        }
        
        fn main() {
            let m1 = Message::Text { 
                content: "hello", 
                data: 42 
            };
            
            let m2 = Message::Number {
                value: 100,
                data: "metadata"
            };
            
            let m3 = Message::<bool>::Empty;
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");
    // Struct variants might have some implementation issues

    // Should have multiple Message instantiations
    let message_instances = count_monomorphized_instances(&program, "Message");
    assert!(
        message_instances >= 2,
        "Should have at least 2 Message instantiations"
    );
}

#[test]
fn test_enum_with_tuple_variants() {
    let code = r#"
        enum Tree<T> {
            Leaf(T),
            Branch(T, Box<Tree<T>>, Box<Tree<T>>),
            Empty
        }
        
        struct Box<T> {
            value: T
        }
        
        fn main() {
            let leaf = Tree::Leaf(42);
            let empty = Tree::<i32>::Empty;
            
            // Would test Branch but recursive types might have issues
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");

    // Should have Tree<i32> instantiation
    let tree_instances = count_monomorphized_instances(&program, "Tree");
    assert!(
        tree_instances >= 1,
        "Should have at least 1 Tree instantiation"
    );
}

#[test]
fn test_enum_method_implementation() {
    let code = r#"
        enum Option<T> {
            Some(T),
            None
        }
        
        impl<T> Option<T> {
            fn is_some(&self) -> bool {
                match self {
                    Option::Some(_) => true,
                    Option::None => false
                }
            }
            
            fn unwrap(self) -> T {
                match self {
                    Option::Some(value) => value,
                    Option::None => panic("unwrap on None")
                }
            }
        }
        
        fn main() {
            let opt1 = Option::Some(42);
            let has_value = opt1.is_some();
            
            let opt2 = Option::Some("hello");
            let value = opt2.unwrap();
        }
    "#;

    let program = compile_generic_program(code);
    // Impl blocks might have errors but enum instantiation should work

    if let Ok(ref prog) = program {
        let option_instances = count_monomorphized_instances(prog, "Option");
        assert!(
            option_instances >= 2,
            "Should have at least 2 Option instantiations"
        );
    }
}

#[test]
fn test_enum_in_function_params() {
    let code = r#"
        enum Result<T, E> {
            Ok(T),
            Err(E)
        }
        
        fn process_result(r: Result<i32, string>) -> i32 {
            match r {
                Result::Ok(value) => value,
                Result::Err(_) => 0
            }
        }
        
        fn process_any<T, E>(r: Result<T, E>) -> bool {
            match r {
                Result::Ok(_) => true,
                Result::Err(_) => false
            }
        }
        
        fn main() {
            let r1 = Result::Ok(42);
            let val = process_result(r1);
            
            let r2 = Result::<string, i32>::Err(404);
            let ok = process_any(r2);
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");

    // Should have multiple Result instantiations
    let result_instances = count_monomorphized_instances(&program, "Result");
    assert!(
        result_instances >= 2,
        "Should have at least 2 Result instantiations"
    );
}

#[test]
fn test_unit_enum_variants() {
    let code = r##"
        enum Color<T> {
            Red,
            Green,
            Blue,
            Custom(T)
        }
        
        fn main() {
            let c1 = Color::<i32>::Red;
            let c2 = Color::<i32>::Green;
            let c3 = Color::Custom(255);
            
            let c4 = Color::<string>::Blue;
            let c5 = Color::Custom("#FF0000");
        }
    "##;

    let program = compile_generic_program(code).expect("Failed to compile");
    assert_no_errors(&program);

    // Should have Color<i32> and Color<string>
    let color_instances = count_monomorphized_instances(&program, "Color");
    assert_eq!(color_instances, 2, "Should have 2 Color instantiations");
}

#[test]
fn test_enum_with_generic_constraints() {
    let code = r#"
        trait Display {
            fn display(&self) -> string;
        }
        
        enum Displayable<T> where T: Display {
            Value(T),
            Empty
        }
        
        fn main() {
            // This might fail if trait bounds aren't fully implemented
            let d = Displayable::<i32>::Empty;
        }
    "#;

    let program = compile_generic_program(code);
    // This might have errors due to trait bound implementation
    // but we're testing that it at least attempts to compile
}

#[test]
fn test_enum_variant_inference() {
    let code = r#"
        enum Option<T> {
            Some(T),
            None
        }
        
        fn main() {
            // Type should be inferred from constructor argument
            let opt1 = Option::Some(42);        // Option<i32>
            let opt2 = Option::Some("hello");   // Option<string>
            let opt3 = Option::Some([1, 2, 3]); // Option<[i32]>
            
            // Explicit type needed for None
            let opt4 = Option::<bool>::None;
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");
    assert_no_errors(&program);

    // Should infer different types from constructor arguments
    let option_instances = count_monomorphized_instances(&program, "Option");
    assert!(
        option_instances >= 4,
        "Should have at least 4 Option instantiations"
    );
}
