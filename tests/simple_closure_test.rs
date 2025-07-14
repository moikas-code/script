use std::collections::HashMap;

// Simple test to verify closure heap allocation works
#[test]
fn test_closure_heap_allocation() {
    use script::runtime::closure::{create_closure_heap, Closure};
    use script::runtime::{ScriptRc, Value};

    // Create a simple closure
    let closure_value = create_closure_heap(
        "test_closure".to_string(),
        vec!["x".to_string()],
        vec![("captured".to_string(), Value::I32(42))],
        false,
    );

    // Verify it's properly allocated
    match closure_value {
        Value::Closure(rc_closure) => {
            assert_eq!(rc_closure.function_id, "test_closure");
            assert_eq!(rc_closure.parameters.len(), 1);
            assert_eq!(rc_closure.get_captured("captured"), Some(&Value::I32(42)));
            assert_eq!(ScriptRc::strong_count(&rc_closure), 1);
        }
        _ => panic!("Expected Value::Closure"),
    }
}

#[test]
fn test_closure_reference_counting() {
    use script::runtime::closure::create_closure_heap;
    use script::runtime::{ScriptRc, Value};

    // Create a closure with captured value
    let closure_value = create_closure_heap(
        "ref_test".to_string(),
        vec![],
        vec![("val".to_string(), Value::I32(100))],
        false,
    );

    // Clone it
    let closure_clone = closure_value.clone();

    // Check reference count
    match &closure_value {
        Value::Closure(rc_closure) => {
            assert_eq!(ScriptRc::strong_count(rc_closure), 2);
        }
        _ => panic!("Expected Value::Closure"),
    }

    // Drop the clone
    drop(closure_clone);

    // Check reference count again
    match &closure_value {
        Value::Closure(rc_closure) => {
            assert_eq!(ScriptRc::strong_count(rc_closure), 1);
        }
        _ => panic!("Expected Value::Closure"),
    }
}

#[test]
fn test_closure_with_nested_rc_values() {
    use script::runtime::closure::create_closure_heap;
    use script::runtime::{ScriptRc, Value};

    // Create an array value with ScriptRc elements
    let array_value = Value::Array(vec![
        ScriptRc::new(Value::I32(1)),
        ScriptRc::new(Value::I32(2)),
        ScriptRc::new(Value::I32(3)),
    ]);

    // Create a closure capturing the array
    let closure_value = create_closure_heap(
        "array_capture".to_string(),
        vec![],
        vec![("array".to_string(), array_value.clone())],
        false, // by value - should clone the array
    );

    // Verify the closure captured the array
    match &closure_value {
        Value::Closure(rc_closure) => {
            match rc_closure.get_captured("array") {
                Some(Value::Array(captured_array)) => {
                    assert_eq!(captured_array.len(), 3);
                    // Verify the elements are there
                    match &*captured_array[0] {
                        Value::I32(1) => {}
                        _ => panic!("Expected first element to be I32(1)"),
                    }
                }
                _ => panic!("Expected captured array"),
            }
        }
        _ => panic!("Expected Value::Closure"),
    }

    // Drop the closure to verify memory cleanup
    drop(closure_value);
}

#[test]
fn test_closure_by_value_vs_by_reference() {
    use script::runtime::closure::create_closure_heap;
    use script::runtime::Value;

    let original_value = Value::I32(100);

    // Test by-value capture
    let by_value_closure = create_closure_heap(
        "by_value".to_string(),
        vec![],
        vec![("val".to_string(), original_value.clone())],
        false, // by value
    );

    // Test by-reference capture
    let by_ref_closure = create_closure_heap(
        "by_ref".to_string(),
        vec![],
        vec![("val".to_string(), original_value.clone())],
        true, // by reference
    );

    // Both should have captured the same value
    match (&by_value_closure, &by_ref_closure) {
        (Value::Closure(rc_by_val), Value::Closure(rc_by_ref)) => {
            assert_eq!(rc_by_val.get_captured("val"), Some(&Value::I32(100)));
            assert_eq!(rc_by_ref.get_captured("val"), Some(&Value::I32(100)));
            assert!(!rc_by_val.captures_by_reference());
            assert!(rc_by_ref.captures_by_reference());
        }
        _ => panic!("Expected Value::Closure"),
    }
}

#[test]
fn test_closure_memory_cleanup() {
    use script::runtime::closure::create_closure_heap;
    use script::runtime::{ScriptRc, Value};

    // Create a closure with multiple captured values
    let captured_vars = vec![
        ("a".to_string(), Value::I32(1)),
        ("b".to_string(), Value::F64(2.5)),
        ("c".to_string(), Value::String("test".to_string())),
        (
            "d".to_string(),
            Value::Array(vec![ScriptRc::new(Value::I32(1))]),
        ),
    ];

    let closure = create_closure_heap(
        "cleanup_test".to_string(),
        vec!["param".to_string()],
        captured_vars,
        false,
    );

    // Verify the closure captured all values
    match &closure {
        Value::Closure(rc_closure) => {
            assert_eq!(rc_closure.captured_vars.len(), 4);
            assert_eq!(rc_closure.get_captured("a"), Some(&Value::I32(1)));
            assert_eq!(rc_closure.get_captured("b"), Some(&Value::F64(2.5)));
            assert_eq!(
                rc_closure.get_captured("c"),
                Some(&Value::String("test".to_string()))
            );

            // Verify the array was captured
            match rc_closure.get_captured("d") {
                Some(Value::Array(captured_array)) => {
                    assert_eq!(captured_array.len(), 1);
                    match &*captured_array[0] {
                        Value::I32(1) => {}
                        _ => panic!("Expected array element to be I32(1)"),
                    }
                }
                _ => panic!("Expected array capture"),
            }
        }
        _ => panic!("Expected closure"),
    };

    // Drop the closure - this should clean up all captured values
    drop(closure);

    // Memory should be freed at this point
    // In a real scenario, we would verify this through memory profiling
}
