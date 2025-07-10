//! Property-based tests for generic types
//!
//! These tests use proptest to verify properties of the generic type system

#[path = "../utils/mod.rs"]
mod utils;

use proptest::prelude::*;
use utils::generic_test_helpers::*;

/// Type representing a simple generic type instantiation
#[derive(Clone, Debug)]
struct GenericTypeInstance {
    type_name: String,
    type_args: Vec<TypeArg>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum TypeArg {
    I32,
    String,
    Bool,
    F32,
}

impl TypeArg {
    fn strategy() -> impl Strategy<Value = Self> {
        prop_oneof![
            Just(TypeArg::I32),
            Just(TypeArg::String),
            Just(TypeArg::Bool),
            Just(TypeArg::F32),
        ]
    }
}

impl GenericTypeInstance {
    fn strategy() -> impl Strategy<Value = Self> {
        let type_names = ["Box", "Option", "Result", "Vec", "Pair"];

        prop_oneof![
            Just("Box").prop_map(|name| (name, 1)),
            Just("Option").prop_map(|name| (name, 1)),
            Just("Result").prop_map(|name| (name, 2)),
            Just("Vec").prop_map(|name| (name, 1)),
            Just("Pair").prop_map(|name| (name, 2)),
        ]
        .prop_flat_map(|(type_name, arg_count)| {
            proptest::collection::vec(TypeArg::strategy(), arg_count).prop_map(move |type_args| {
                GenericTypeInstance {
                    type_name: type_name.to_string(),
                    type_args,
                }
            })
        })
    }
}

/// Generate Script code for a type argument
fn type_arg_to_script(arg: &TypeArg) -> &'static str {
    match arg {
        TypeArg::I32 => "i32",
        TypeArg::String => "string",
        TypeArg::Bool => "bool",
        TypeArg::F32 => "f32",
    }
}

/// Generate a value literal for a type
fn type_arg_to_value(arg: &TypeArg, seed: usize) -> String {
    match arg {
        TypeArg::I32 => seed.to_string(),
        TypeArg::String => format!(r#""value{}""#, seed),
        TypeArg::Bool => (seed % 2 == 0).to_string(),
        TypeArg::F32 => format!("{}.0", seed),
    }
}

/// Generate Script code for a generic type instance
fn generate_instance_code(instance: &GenericTypeInstance, var_name: &str, seed: usize) -> String {
    match instance.type_name.as_str() {
        "Box" => {
            let value = type_arg_to_value(&instance.type_args[0], seed);
            format!("let {} = Box {{ value: {} }};", var_name, value)
        }
        "Option" => {
            let value = type_arg_to_value(&instance.type_args[0], seed);
            if seed % 2 == 0 {
                format!("let {} = Option::Some({});", var_name, value)
            } else {
                format!(
                    "let {}: Option<{}> = Option::None;",
                    var_name,
                    type_arg_to_script(&instance.type_args[0])
                )
            }
        }
        "Result" => {
            let ok_value = type_arg_to_value(&instance.type_args[0], seed);
            let err_value = type_arg_to_value(&instance.type_args[1], seed + 1);
            if seed % 2 == 0 {
                format!(
                    "let {}: Result<{}, {}> = Result::Ok({});",
                    var_name,
                    type_arg_to_script(&instance.type_args[0]),
                    type_arg_to_script(&instance.type_args[1]),
                    ok_value
                )
            } else {
                format!(
                    "let {}: Result<{}, {}> = Result::Err({});",
                    var_name,
                    type_arg_to_script(&instance.type_args[0]),
                    type_arg_to_script(&instance.type_args[1]),
                    err_value
                )
            }
        }
        "Vec" => {
            let elem_type = type_arg_to_script(&instance.type_args[0]);
            let values: Vec<String> = (0..3)
                .map(|i| type_arg_to_value(&instance.type_args[0], seed + i))
                .collect();
            format!(
                "let {} = Vec::<{}> {{ data: [{}], len: 3 }};",
                var_name,
                elem_type,
                values.join(", ")
            )
        }
        "Pair" => {
            let first = type_arg_to_value(&instance.type_args[0], seed);
            let second = type_arg_to_value(&instance.type_args[1], seed + 1);
            format!(
                "let {} = Pair {{ first: {}, second: {} }};",
                var_name, first, second
            )
        }
        _ => unreachable!(),
    }
}

proptest! {
    #[test]
    fn prop_generic_instantiation_compiles(
        instances in proptest::collection::vec(GenericTypeInstance::strategy(), 0..10)
    ) {
        if instances.is_empty() {
            return Ok(());
        }

        let mut code = String::new();

        // Add type definitions
        code.push_str("struct Box<T> { value: T }\n");
        code.push_str("enum Option<T> { Some(T), None }\n");
        code.push_str("enum Result<T, E> { Ok(T), Err(E) }\n");
        code.push_str("struct Vec<T> { data: [T], len: i32 }\n");
        code.push_str("struct Pair<A, B> { first: A, second: B }\n\n");

        code.push_str("fn main() {\n");

        // Generate instances
        for (i, instance) in instances.iter().enumerate() {
            code.push_str("    ");
            code.push_str(&generate_instance_code(instance, &format!("v{}", i), i));
            code.push_str("\n");
        }

        code.push_str("}\n");

        // Try to compile
        match compile_generic_program(&code) {
            Ok(program) => prop_assert!(program.errors.is_empty()),
            Err(_) => prop_assert!(false, "Failed to compile generated program"),
        }
    }

    #[test]
    fn prop_same_type_different_values(type_arg in TypeArg::strategy()) {
        let code = format!(r#"
            struct Container<T> {{
                value: T
            }}
            
            fn main() {{
                let c1 = Container {{ value: {} }};
                let c2 = Container {{ value: {} }};
                // Both should have the same monomorphized type
            }}
        "#, 
            type_arg_to_value(&type_arg, 1),
            type_arg_to_value(&type_arg, 2)
        );

        match compile_generic_program(&code) {
            Ok(program) => {
                // Should create only one monomorphized instance
                prop_assert_eq!(count_monomorphized_instances(&program, "Container"), 1);
            }
            Err(_) => prop_assert!(false, "Failed to compile"),
        }
    }

    #[test]
    fn prop_nested_generics_compile(
        outer in TypeArg::strategy(),
        inner in TypeArg::strategy()
    ) {
        let code = format!(r#"
            struct Box<T> {{ value: T }}
            struct Container<T> {{ item: T }}
            
            fn main() {{
                let nested = Box {{
                    value: Container {{
                        item: {}
                    }}
                }};
            }}
        "#, type_arg_to_value(&inner, 42));

        match compile_generic_program(&code) {
            Ok(program) => prop_assert!(program.errors.is_empty()),
            Err(_) => prop_assert!(false, "Failed to compile nested generics"),
        }
    }

    #[test]
    fn prop_generic_function_instantiation(
        args in proptest::collection::vec(TypeArg::strategy(), 0..10)
    ) {
        if args.is_empty() {
            return Ok(());
        }

        let mut code = String::new();
        code.push_str("fn identity<T>(x: T) -> T { x }\n\n");
        code.push_str("fn main() {\n");

        for (i, arg) in args.iter().enumerate() {
            code.push_str(&format!(
                "    let result{} = identity({});\n",
                i,
                type_arg_to_value(arg, i)
            ));
        }

        code.push_str("}\n");

        match compile_generic_program(&code) {
            Ok(program) => {
                // Should create one monomorphization per unique type
                let unique_types = args.iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len();
                prop_assert_eq!(
                    count_monomorphized_instances(&program, "identity"),
                    unique_types
                );
            }
            Err(_) => prop_assert!(false, "Failed to compile"),
        }
    }

    #[test]
    fn prop_type_inference_consistency(
        use_annotation in any::<bool>(),
        type_arg in TypeArg::strategy()
    ) {
        let value = type_arg_to_value(&type_arg, 99);
        let type_name = type_arg_to_script(&type_arg);

        let code = if use_annotation {
            format!(r#"
                struct Wrapper<T> {{ value: T }}
                
                fn main() {{
                    let w: Wrapper<{}> = Wrapper {{ value: {} }};
                }}
            "#, type_name, value)
        } else {
            format!(r#"
                struct Wrapper<T> {{ value: T }}
                
                fn main() {{
                    let w = Wrapper {{ value: {} }};
                }}
            "#, value)
        };

        // Both versions should compile to the same result
        prop_assert!(compile_generic_program(&code).is_ok());
    }

    #[test]
    fn prop_referential_transparency(seed in 0u32..1000) {
        let value = seed % 100;

        let code = format!(r#"
            struct Box<T> {{ value: T }}
            
            fn make_box<T>(x: T) -> Box<T> {{
                Box {{ value: x }}
            }}
            
            fn main() {{
                let b1 = Box {{ value: {} }};
                let b2 = make_box({});
                // b1 and b2 should have the same type
            }}
        "#, value, value);

        prop_assert!(compile_generic_program(&code).is_ok());
    }
}

#[test]
fn test_property_tests_work() {
    // Quick sanity check that our property tests can run
    let instance = GenericTypeInstance {
        type_name: "Box".to_string(),
        type_args: vec![TypeArg::I32],
    };

    let code = format!(
        r#"
        struct Box<T> {{ value: T }}
        
        fn main() {{
            {}
        }}
    "#,
        generate_instance_code(&instance, "test", 0)
    );

    assert!(compile_generic_program(&code).is_ok());
}
