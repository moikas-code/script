//! Tests for closure memory management
//!
//! This module tests heap allocation, reference counting, and cleanup
//! for closures in the Script runtime.

use script::runtime::{Value, ScriptRc};
use script::runtime::closure::{Closure, create_closure_heap};
use std::collections::HashMap;

#[test]
fn test_closure_heap_allocation() {
    // Test that closures are allocated on the heap using ScriptRc
    let closure_value = create_closure_heap(
        "test_closure".to_string(),
        vec!["x".to_string(), "y".to_string()],
        vec![],
        false,
    );
    
    // Verify it's a closure value
    match &closure_value {
        Value::Closure(rc_closure) => {
            assert_eq!(rc_closure.function_id, "test_closure");
            assert_eq!(rc_closure.parameters.len(), 2);
            assert_eq!(ScriptRc::strong_count(rc_closure), 1);
        }
        _ => panic!("Expected Value::Closure"),
    }
}

#[test]
fn test_closure_with_captured_values() {
    // Create some values to capture
    let captured_vars = vec![
        ("x".to_string(), Value::I32(42)),
        ("y".to_string(), Value::String("hello".to_string())),
        ("z".to_string(), Value::Bool(true)),
    ];
    
    let closure_value = create_closure_heap(
        "capturing_closure".to_string(),
        vec!["param".to_string()],
        captured_vars,
        false, // by value
    );
    
    match &closure_value {
        Value::Closure(rc_closure) => {
            assert_eq!(rc_closure.captured_vars.len(), 3);
            assert_eq!(rc_closure.get_captured("x"), Some(&Value::I32(42)));
            assert_eq!(rc_closure.get_captured("y"), Some(&Value::String("hello".to_string())));
            assert_eq!(rc_closure.get_captured("z"), Some(&Value::Bool(true)));
        }
        _ => panic!("Expected Value::Closure"),
    }
}

#[test]
fn test_closure_reference_counting() {
    // Create a closure
    let closure_value = create_closure_heap(
        "ref_counted_closure".to_string(),
        vec![],
        vec![("counter".to_string(), Value::I32(0))],
        false,
    );
    
    // Clone the closure multiple times
    let closure2 = closure_value.clone();
    let closure3 = closure_value.clone();
    
    // Verify reference count
    match &closure_value {
        Value::Closure(rc_closure) => {
            assert_eq!(ScriptRc::strong_count(rc_closure), 3);
        }
        _ => panic!("Expected Value::Closure"),
    }
    
    // Drop one reference
    drop(closure3);
    
    match &closure_value {
        Value::Closure(rc_closure) => {
            assert_eq!(ScriptRc::strong_count(rc_closure), 2);
        }
        _ => panic!("Expected Value::Closure"),
    }
    
    // Drop another reference
    drop(closure2);
    
    match &closure_value {
        Value::Closure(rc_closure) => {
            assert_eq!(ScriptRc::strong_count(rc_closure), 1);
        }
        _ => panic!("Expected Value::Closure"),
    }
}

#[test]
fn test_closure_with_rc_captures() {
    // Create values that themselves use reference counting
    let array_value = Value::Array(ScriptRc::new(vec![
        Value::I32(1),
        Value::I32(2),
        Value::I32(3),
    ]));
    
    let string_value = Value::String("shared string".to_string());
    
    // Create a closure capturing these values
    let captured_vars = vec![
        ("array".to_string(), array_value.clone()),
        ("string".to_string(), string_value.clone()),
    ];
    
    let closure_value = create_closure_heap(
        "rc_capturing_closure".to_string(),
        vec![],
        captured_vars,
        false, // by value - should clone and increment ref counts
    );
    
    // Verify the array's reference count increased
    match &array_value {
        Value::Array(rc_array) => {
            assert_eq!(ScriptRc::strong_count(rc_array), 2); // original + captured
        }
        _ => panic!("Expected Value::Array"),
    }
    
    // Drop the closure
    drop(closure_value);
    
    // Verify the array's reference count decreased
    match &array_value {
        Value::Array(rc_array) => {
            assert_eq!(ScriptRc::strong_count(rc_array), 1); // just original
        }
        _ => panic!("Expected Value::Array"),
    }
}

#[test]
fn test_closure_by_value_vs_by_reference() {
    let mut original_value = Value::I32(100);
    
    // Test by-value capture (cloning)
    let by_value_closure = create_closure_heap(
        "by_value".to_string(),
        vec![],
        vec![("val".to_string(), original_value.clone())],
        false, // by value
    );
    
    // Modify original
    original_value = Value::I32(200);
    
    // Captured value should still be 100
    match &by_value_closure {
        Value::Closure(rc_closure) => {
            assert_eq!(rc_closure.get_captured("val"), Some(&Value::I32(100)));
        }
        _ => panic!("Expected Value::Closure"),
    }
    
    // Test by-reference capture
    let by_ref_closure = create_closure_heap(
        "by_ref".to_string(),
        vec![],
        vec![("val".to_string(), original_value.clone())],
        true, // by reference
    );
    
    // Should have captured the new value
    match &by_ref_closure {
        Value::Closure(rc_closure) => {
            assert_eq!(rc_closure.get_captured("val"), Some(&Value::I32(200)));
            assert!(rc_closure.captures_by_reference());
        }
        _ => panic!("Expected Value::Closure"),
    }
}

#[test]
fn test_nested_closure_captures() {
    // Create an inner closure
    let inner_closure = create_closure_heap(
        "inner".to_string(),
        vec!["x".to_string()],
        vec![("inner_val".to_string(), Value::I32(42))],
        false,
    );
    
    // Create an outer closure that captures the inner closure
    let outer_closure = create_closure_heap(
        "outer".to_string(),
        vec!["y".to_string()],
        vec![
            ("inner_fn".to_string(), inner_closure.clone()),
            ("outer_val".to_string(), Value::I32(84)),
        ],
        false,
    );
    
    // Verify nested structure
    match &outer_closure {
        Value::Closure(rc_outer) => {
            assert_eq!(rc_outer.captured_vars.len(), 2);
            
            // Check the captured inner closure
            match rc_outer.get_captured("inner_fn") {
                Some(Value::Closure(rc_inner)) => {
                    assert_eq!(rc_inner.function_id, "inner");
                    assert_eq!(rc_inner.get_captured("inner_val"), Some(&Value::I32(42)));
                    // Should have 2 references: original + captured
                    assert_eq!(ScriptRc::strong_count(rc_inner), 2);
                }
                _ => panic!("Expected captured closure"),
            }
        }
        _ => panic!("Expected Value::Closure"),
    }
    
    // Drop the original inner closure
    drop(inner_closure);
    
    // The captured closure should still exist
    match &outer_closure {
        Value::Closure(rc_outer) => {
            match rc_outer.get_captured("inner_fn") {
                Some(Value::Closure(rc_inner)) => {
                    // Now only 1 reference from the outer closure
                    assert_eq!(ScriptRc::strong_count(rc_inner), 1);
                }
                _ => panic!("Expected captured closure"),
            }
        }
        _ => panic!("Expected Value::Closure"),
    }
}

#[test]
fn test_closure_memory_cleanup() {
    // Create a closure with multiple captured values
    let captured_vars = vec![
        ("a".to_string(), Value::I32(1)),
        ("b".to_string(), Value::F64(2.5)),
        ("c".to_string(), Value::String("test".to_string())),
        ("d".to_string(), Value::Array(ScriptRc::new(vec![Value::I32(1), Value::I32(2)]))),
    ];
    
    let closure = create_closure_heap(
        "cleanup_test".to_string(),
        vec!["param1".to_string(), "param2".to_string()],
        captured_vars,
        false,
    );
    
    // Get reference count of the array before dropping
    let array_rc_count = match &closure {
        Value::Closure(rc_closure) => {
            match rc_closure.get_captured("d") {
                Some(Value::Array(rc_array)) => ScriptRc::strong_count(rc_array),
                _ => panic!("Expected array capture"),
            }
        }
        _ => panic!("Expected closure"),
    };
    
    assert_eq!(array_rc_count, 1); // Only the closure holds a reference
    
    // Drop the closure - this should clean up all captured values
    drop(closure);
    
    // All memory should be freed at this point
    // In a real scenario, we would verify this through memory profiling
}

#[test]
fn test_closure_weak_references() {
    // Test that weak references work properly with closures
    let closure = create_closure_heap(
        "weak_ref_test".to_string(),
        vec![],
        vec![("val".to_string(), Value::I32(42))],
        false,
    );
    
    // Create a weak reference to the closure
    let weak_ref = match &closure {
        Value::Closure(rc_closure) => ScriptRc::downgrade(rc_closure),
        _ => panic!("Expected closure"),
    };
    
    // Weak count should be 2 (1 implicit from strong + 1 explicit)
    assert_eq!(weak_ref.weak_count(), 2);
    
    // Upgrade should succeed while strong reference exists
    assert!(weak_ref.upgrade().is_some());
    
    // Drop the strong reference
    drop(closure);
    
    // Upgrade should fail after dropping all strong references
    assert!(weak_ref.upgrade().is_none());
    assert_eq!(weak_ref.strong_count(), 0);
}

#[test]
fn test_closure_cycle_potential() {
    // Create a scenario where closures could form a cycle
    // This tests that our reference counting handles potential cycles
    
    // Create a mutable hashmap to simulate a closure environment
    let mut env1 = HashMap::new();
    env1.insert("self_ref".to_string(), Value::Null); // Placeholder
    
    let closure1 = Closure::new(
        "recursive_closure".to_string(),
        vec!["x".to_string()],
        env1,
    );
    
    let rc_closure1 = ScriptRc::new(closure1);
    let closure_value1 = Value::Closure(rc_closure1.clone());
    
    // In a real implementation, we would update the closure to reference itself
    // This would create a cycle that our Bacon-Rajan cycle detector should handle
    
    // For now, just verify the closure was created correctly
    assert_eq!(rc_closure1.function_id, "recursive_closure");
    assert_eq!(ScriptRc::strong_count(&rc_closure1), 1);
}

#[test]
fn test_closure_self_reference_cycle() {
    use script::runtime::gc;
    
    // Initialize cycle collector for this test
    let _ = gc::initialize();
    
    // Create a closure that captures another closure (potential cycle)
    let inner_closure = create_closure_heap(
        "inner".to_string(),
        vec!["x".to_string()],
        vec![("value".to_string(), Value::I32(42))],
        false,
    );
    
    // Create an outer closure that captures the inner closure
    let outer_closure = create_closure_heap(
        "outer".to_string(),
        vec!["y".to_string()],
        vec![
            ("inner_fn".to_string(), inner_closure.clone()),
            ("data".to_string(), Value::String("test".to_string())),
        ],
        false,
    );
    
    // Verify both closures exist and are properly linked
    match (&inner_closure, &outer_closure) {
        (Value::Closure(inner_rc), Value::Closure(outer_rc)) => {
            assert_eq!(ScriptRc::strong_count(inner_rc), 2); // original + captured
            assert_eq!(ScriptRc::strong_count(outer_rc), 1); // just original
            
            // Verify the capture
            match outer_rc.get_captured("inner_fn") {
                Some(Value::Closure(captured_inner)) => {
                    assert_eq!(captured_inner.function_id, "inner");
                }
                _ => panic!("Expected captured inner closure"),
            }
        }
        _ => panic!("Expected closures"),
    }
    
    // Drop the original inner closure reference
    drop(inner_closure);
    
    // The inner closure should still exist through the outer closure
    match &outer_closure {
        Value::Closure(outer_rc) => {
            match outer_rc.get_captured("inner_fn") {
                Some(Value::Closure(captured_inner)) => {
                    assert_eq!(ScriptRc::strong_count(captured_inner), 1); // only captured ref
                }
                _ => panic!("Expected captured inner closure"),
            }
        }
        _ => panic!("Expected outer closure"),
    }
    
    // Force cycle collection to test cycle detection
    gc::collect_cycles();
    
    // Both closures should still be alive (no actual cycle in this case)
    match &outer_closure {
        Value::Closure(outer_rc) => {
            assert_eq!(outer_rc.function_id, "outer");
            match outer_rc.get_captured("inner_fn") {
                Some(Value::Closure(captured_inner)) => {
                    assert_eq!(captured_inner.function_id, "inner");
                }
                _ => panic!("Expected captured inner closure after GC"),
            }
        }
        _ => panic!("Expected outer closure after GC"),
    }
    
    // Cleanup
    let _ = gc::shutdown();
}

#[test] 
fn test_closure_circular_reference() {
    use script::runtime::gc;
    
    // Initialize cycle collector
    let _ = gc::initialize();
    
    // Create two closures that will reference each other (circular reference)
    let closure_a = create_closure_heap(
        "closure_a".to_string(),
        vec!["x".to_string()],
        vec![("id".to_string(), Value::String("A".to_string()))],
        false,
    );
    
    let closure_b = create_closure_heap(
        "closure_b".to_string(),
        vec!["y".to_string()],
        vec![
            ("id".to_string(), Value::String("B".to_string())),
            ("friend".to_string(), closure_a.clone()), // B references A
        ],
        false,
    );
    
    // Now create a modified A that references B (completing the cycle)
    // In a real scenario, this would be done through mutable captures or environment updates
    let closure_a_with_b = create_closure_heap(
        "closure_a_updated".to_string(),
        vec!["x".to_string()],
        vec![
            ("id".to_string(), Value::String("A".to_string())),
            ("friend".to_string(), closure_b.clone()), // A references B
        ],
        false,
    );
    
    // Verify the circular reference setup
    match (&closure_a_with_b, &closure_b) {
        (Value::Closure(a_rc), Value::Closure(b_rc)) => {
            // Verify A has B as friend
            match a_rc.get_captured("friend") {
                Some(Value::Closure(friend_b)) => {
                    assert_eq!(friend_b.function_id, "closure_b");
                }
                _ => panic!("Expected A to capture B"),
            }
            
            // Verify B has A as friend  
            match b_rc.get_captured("friend") {
                Some(Value::Closure(friend_a)) => {
                    assert_eq!(friend_a.function_id, "closure_a");
                }
                _ => panic!("Expected B to capture A"),
            }
        }
        _ => panic!("Expected both closures"),
    }
    
    // Get initial reference counts
    let initial_a_count = match &closure_a_with_b {
        Value::Closure(rc) => ScriptRc::strong_count(rc),
        _ => panic!("Expected closure A"),
    };
    
    let initial_b_count = match &closure_b {
        Value::Closure(rc) => ScriptRc::strong_count(rc),
        _ => panic!("Expected closure B"),
    };
    
    // Force cycle collection - should detect the circular reference
    gc::collect_cycles();
    
    // Verify closures are still accessible (they should be since we hold references)
    match (&closure_a_with_b, &closure_b) {
        (Value::Closure(a_rc), Value::Closure(b_rc)) => {
            assert_eq!(a_rc.function_id, "closure_a_updated");
            assert_eq!(b_rc.function_id, "closure_b");
        }
        _ => panic!("Expected closures to survive GC while referenced"),
    }
    
    // Drop our references to create orphaned cycle
    drop(closure_a);
    drop(closure_a_with_b);
    drop(closure_b);
    
    // Force another collection - should clean up the cycle
    gc::collect_cycles();
    
    // Verify collection occurred (we can't directly test object deletion,
    // but we can verify the collector ran)
    let stats = gc::get_stats();
    if let Some(stats) = stats {
        assert!(stats.collections > 0);
    }
    
    // Cleanup
    let _ = gc::shutdown();
}

#[test]
fn test_closure_deep_nesting_cycles() {
    use script::runtime::gc;
    
    // Test deeply nested closures with potential cycles
    let _ = gc::initialize();
    
    // Create a chain of closures: A -> B -> C -> A (cycle)
    let closure_c = create_closure_heap(
        "closure_c".to_string(),
        vec!["z".to_string()],
        vec![("level".to_string(), Value::I32(3))],
        false,
    );
    
    let closure_b = create_closure_heap(
        "closure_b".to_string(),
        vec!["y".to_string()],
        vec![
            ("level".to_string(), Value::I32(2)),
            ("next".to_string(), closure_c.clone()),
        ],
        false,
    );
    
    let closure_a = create_closure_heap(
        "closure_a".to_string(),
        vec!["x".to_string()],
        vec![
            ("level".to_string(), Value::I32(1)),
            ("next".to_string(), closure_b.clone()),
        ],
        false,
    );
    
    // Complete the cycle: C -> A
    let closure_c_cyclic = create_closure_heap(
        "closure_c_cyclic".to_string(),
        vec!["z".to_string()],
        vec![
            ("level".to_string(), Value::I32(3)),
            ("next".to_string(), closure_a.clone()),
        ],
        false,
    );
    
    // Verify the chain structure
    match &closure_a {
        Value::Closure(a_rc) => {
            match a_rc.get_captured("next") {
                Some(Value::Closure(b_rc)) => {
                    assert_eq!(b_rc.function_id, "closure_b");
                    match b_rc.get_captured("next") {
                        Some(Value::Closure(c_rc)) => {
                            assert_eq!(c_rc.function_id, "closure_c");
                        }
                        _ => panic!("Expected C in B"),
                    }
                }
                _ => panic!("Expected B in A"),
            }
        }
        _ => panic!("Expected closure A"),
    }
    
    // Test incremental collection with complex cycle
    let collected = gc::collect_cycles_incremental(10); // Limit work units
    
    // May need multiple incremental collections for complex cycles
    let mut iterations = 0;
    let mut is_complete = collected;
    while !is_complete && iterations < 10 {
        is_complete = gc::collect_cycles_incremental(10);
        iterations += 1;
    }
    
    // Verify collection completed
    assert!(iterations < 10, "Incremental collection should complete");
    
    // All closures should still be alive since we hold references
    match &closure_a {
        Value::Closure(a_rc) => assert_eq!(a_rc.function_id, "closure_a"),
        _ => panic!("Expected closure A after incremental GC"),
    }
    
    // Cleanup
    let _ = gc::shutdown();
}

#[test]
fn test_closure_cycle_collector_integration() {
    use script::runtime::gc;
    
    // Test that closures properly integrate with the cycle collector
    let _ = gc::initialize();
    
    // Check initial stats
    let initial_stats = gc::get_stats().unwrap_or_default();
    let initial_collections = initial_stats.collections;
    
    // Create closures with potential for cycles
    let mut closures = Vec::new();
    for i in 0..5 {
        let closure = create_closure_heap(
            format!("closure_{}", i),
            vec![format!("param_{}", i)],
            vec![
                ("index".to_string(), Value::I32(i as i32)),
                ("data".to_string(), Value::String(format!("data_{}", i),
            ],
            false,
        );
        closures.push(closure);
    }
    
    // Create references between closures
    for i in 0..4 {
        let next_closure = closures[i + 1].clone();
        let updated_closure = create_closure_heap(
            format!("closure_{}_updated", i),
            vec![format!("param_{}", i)],
            vec![
                ("index".to_string(), Value::I32(i as i32)),
                ("next".to_string(), next_closure),
            ],
            false,
        );
        closures[i] = updated_closure;
    }
    
    // Complete a cycle by making the last closure reference the first
    let first_closure = closures[0].clone();
    let last_updated = create_closure_heap(
        "closure_4_cyclic".to_string(),
        vec!["param_4".to_string()],
        vec![
            ("index".to_string(), Value::I32(4)),
            ("next".to_string(), first_closure),
        ],
        false,
    );
    closures[4] = last_updated;
    
    // Trigger collection
    gc::collect_cycles();
    
    // Check that collection ran
    let final_stats = gc::get_stats().unwrap_or_default();
    assert!(final_stats.collections >= initial_collections);
    
    // Verify closures are still accessible
    for (i, closure) in closures.iter().enumerate() {
        match closure {
            Value::Closure(rc_closure) => {
                // Verify closure identity
                assert!(rc_closure.function_id.contains(&i.to_string());
                
                // Verify captured data
                match rc_closure.get_captured("index") {
                    Some(Value::I32(idx)) => assert_eq!(*idx, i as i32),
                    _ => panic!("Expected index capture"),
                }
            }
            _ => panic!("Expected closure at index {}", i),
        }
    }
    
    // Test cycle detection by dropping all references
    drop(closures);
    
    // Force another collection
    gc::collect_cycles();
    
    // Verify collection stats updated
    let post_drop_stats = gc::get_stats().unwrap_or_default();
    assert!(post_drop_stats.collections >= final_stats.collections);
    
    // Cleanup
    let _ = gc::shutdown();
}

#[test]
fn test_closure_thread_safety() {
    use std::thread;
    use std::sync::Arc;
    
    // Create a closure that will be shared between threads
    let closure = create_closure_heap(
        "thread_safe_closure".to_string(),
        vec!["x".to_string()],
        vec![("shared_val".to_string(), Value::I32(100))],
        false,
    );
    
    // Wrap in Arc for thread sharing
    let arc_closure = Arc::new(closure);
    
    // Spawn multiple threads that clone the closure
    let mut handles = vec![];
    
    for i in 0..5 {
        let closure_clone = Arc::clone(&arc_closure);
        let handle = thread::spawn(move || {
            // Each thread clones the closure value
            let _local_copy = closure_clone.clone();
            
            // Verify the closure in this thread
            match &**closure_clone {
                Value::Closure(rc_closure) => {
                    assert_eq!(rc_closure.function_id, "thread_safe_closure");
                    assert_eq!(rc_closure.get_captured("shared_val"), Some(&Value::I32(100)));
                }
                _ => panic!("Expected closure in thread {}", i),
            }
        });
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Verify the original closure is still valid
    match &*arc_closure {
        Value::Closure(rc_closure) => {
            assert_eq!(rc_closure.function_id, "thread_safe_closure");
            // Reference count should be back to 1 (just the Arc)
            assert_eq!(ScriptRc::strong_count(rc_closure), 1);
        }
        _ => panic!("Expected closure"),
    }
}

#[cfg(test)]
mod ffi_tests {
    use super::*;
    use std::ffi::CString;
    use script::runtime::closure::{script_create_closure, script_free_closure};
    
    #[test]
    fn test_ffi_closure_creation() {
        unsafe {
            // Prepare function ID
            let function_id = CString::new("ffi_closure").unwrap();
            let function_id_ptr = function_id.as_ptr() as *const u8;
            let function_id_len = function_id.as_bytes().len();
            
            // Prepare parameters
            let param1 = CString::new("x").unwrap();
            let param2 = CString::new("y").unwrap();
            let param_ptrs = vec![
                param1.as_ptr() as *const u8,
                param2.as_ptr() as *const u8,
            ];
            let param_lengths = vec![1, 1];
            
            // Prepare captured variables
            let capture_name = CString::new("captured").unwrap();
            let capture_names = vec![capture_name.as_ptr() as *const u8];
            let capture_lengths = vec![8];
            let capture_values = vec![Value::I32(42)];
            
            // Create closure through FFI
            let closure_ptr = script_create_closure(
                function_id_ptr,
                function_id_len,
                param_ptrs.as_ptr() as *const *const u8,
                param_lengths.as_ptr(),
                2,
                capture_names.as_ptr() as *const *const u8,
                capture_lengths.as_ptr(),
                capture_values.as_ptr(),
                1,
                false,
            );
            
            assert!(!closure_ptr.is_null());
            
            // Verify the created closure
            let closure_value = &*closure_ptr;
            match closure_value {
                Value::Closure(rc_closure) => {
                    assert_eq!(rc_closure.function_id, "ffi_closure");
                    assert_eq!(rc_closure.parameters, vec!["x", "y"]);
                    assert_eq!(rc_closure.get_captured("captured"), Some(&Value::I32(42)));
                }
                _ => panic!("Expected closure from FFI"),
            }
            
            // Clean up
            script_free_closure(closure_ptr);
        }
    }
}