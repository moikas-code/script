//! Integration tests for the Script runtime system

use script::runtime::{self, Runtime, RuntimeConfig, ScriptRc};
use script::script_assert;
use std::sync::Arc;
use std::thread;

/// Helper to ensure clean runtime state for tests
fn with_runtime<F>(config: RuntimeConfig, test: F)
where
    F: FnOnce(),
{
    // Ensure runtime is not initialized - ignore errors
    let _ = runtime::shutdown();

    // Initialize runtime
    if let Err(e) = runtime::initialize() {
        if !matches!(e, script::runtime::RuntimeError::AlreadyInitialized) {
            panic!("Failed to initialize runtime: {}", e);
        }
    }

    if let Err(e) = Runtime::initialize_with_config(config) {
        if !matches!(e, script::runtime::RuntimeError::AlreadyInitialized) {
            panic!("Failed to initialize runtime with config: {}", e);
        }
    }

    // Run test
    test();

    // Clean up - ignore errors
    let _ = runtime::shutdown();
}

#[test]
fn test_runtime_basic_lifecycle() {
    with_runtime(RuntimeConfig::default(), || {
        assert!(runtime::is_initialized());

        // Get runtime instance
        let rt = runtime::core::runtime().unwrap();
        assert!(rt.config().enable_gc);
        assert!(rt.config().enable_panic_handler);
    });

    assert!(!runtime::is_initialized());
}

#[test]
fn test_reference_counting() {
    with_runtime(RuntimeConfig::default(), || {
        // Test basic RC operations
        let rc1 = ScriptRc::new(42);
        assert_eq!(*rc1, 42);
        assert_eq!(rc1.strong_count(), 1);

        let rc2 = rc1.clone();
        assert_eq!(rc1.strong_count(), 2);
        assert_eq!(rc2.strong_count(), 2);

        drop(rc1);
        assert_eq!(rc2.strong_count(), 1);
    });
}

#[test]
fn test_weak_references() {
    with_runtime(RuntimeConfig::default(), || {
        let rc = ScriptRc::new("test string");
        let weak = rc.downgrade();

        assert_eq!(weak.strong_count(), 1);
        assert!(weak.upgrade().is_some());

        drop(rc);
        assert_eq!(weak.strong_count(), 0);
        assert!(weak.upgrade().is_none());
    });
}

#[test]
fn test_complex_object_graph() {
    with_runtime(RuntimeConfig::default(), || {
        // Create a complex object graph
        #[derive(Debug)]
        struct Node {
            value: i32,
            next: Option<ScriptRc<Node>>,
        }

        let node3 = ScriptRc::new(Node {
            value: 3,
            next: None,
        });
        let node2 = ScriptRc::new(Node {
            value: 2,
            next: Some(node3.clone()),
        });
        let node1 = ScriptRc::new(Node {
            value: 1,
            next: Some(node2.clone()),
        });

        assert_eq!(node1.value, 1);
        assert_eq!(node1.next.as_ref().unwrap().value, 2);
        assert_eq!(node1.next.as_ref().unwrap().next.as_ref().unwrap().value, 3);
    });
}

#[test]
fn test_memory_limits() {
    let mut config = RuntimeConfig::default();
    config.max_heap_size = 1024; // 1KB limit

    with_runtime(config, || {
        let rt = runtime::core::runtime().unwrap();

        // Should succeed - small allocation
        let small = rt
            .memory()
            .allocate(std::alloc::Layout::from_size_align(100, 8).unwrap());
        assert!(small.is_ok());

        // Should fail - exceeds limit
        let large = rt
            .memory()
            .allocate(std::alloc::Layout::from_size_align(2048, 8).unwrap());
        assert!(large.is_err());

        // Clean up
        if let Ok(ptr) = small {
            rt.memory()
                .deallocate(ptr, std::alloc::Layout::from_size_align(100, 8).unwrap());
        }
    });
}

#[test]
fn test_memory_statistics() {
    with_runtime(RuntimeConfig::default(), || {
        let rt = runtime::core::runtime().unwrap();

        // Initial stats
        let stats = rt.stats();
        let initial_used = stats.memory.heap_used;

        // Allocate some memory through ScriptRc
        let _rc1 = ScriptRc::new(vec![1, 2, 3, 4, 5]);
        let _rc2 = ScriptRc::new("a string with some content".to_string());

        // Check stats increased
        let stats = rt.stats();
        assert!(stats.memory.heap_used > initial_used);
        assert!(stats.memory.total_allocations > 0);

        // Check uptime
        assert!(stats.uptime.as_millis() > 0);
    });
}

#[test]
fn test_profiler_integration() {
    let mut config = RuntimeConfig::default();
    config.enable_profiling = true;

    with_runtime(config, || {
        // Create some allocations
        for i in 0..10 {
            let _rc = ScriptRc::new(vec![i; 100]);
        }

        // Get profiling stats
        if let Some(stats) = runtime::profiler::get_stats() {
            assert!(stats.allocations.total_allocations >= 10);
            assert!(stats.allocations.total_bytes_allocated > 0);
            assert!(!stats.type_stats.is_empty());
        }
    });
}

#[test]
fn test_gc_cycle_detection() {
    let mut config = RuntimeConfig::default();
    config.enable_gc = true;
    config.gc_threshold = 5; // Low threshold for testing

    with_runtime(config, || {
        // Create potential cycles
        for _ in 0..10 {
            let _rc = ScriptRc::new(vec![1, 2, 3]);
        }

        // Force collection
        runtime::gc::collect_cycles();

        // Check GC stats
        if let Some(stats) = runtime::gc::get_stats() {
            assert!(stats.collections > 0);
        }
    });
}

#[test]
fn test_panic_handling() {
    let mut config = RuntimeConfig::default();
    config.enable_panic_handler = true;

    with_runtime(config, || {
        let rt = runtime::core::runtime().unwrap();

        // Test protected execution
        let result = rt.execute_protected(|| 42 + 58);
        assert_eq!(result.unwrap(), 100);

        // Test panic recovery
        let result = rt.execute_protected(|| {
            panic!("Test panic!");
        });
        assert!(result.is_err());

        // Check panic was recorded
        let last_panic = runtime::panic::last_panic();
        assert!(last_panic.is_some());
        assert!(last_panic.unwrap().message.contains("Test panic"));
    });
}

#[test]
fn test_concurrent_access() {
    with_runtime(RuntimeConfig::default(), || {
        let shared = ScriptRc::new(Arc::new(std::sync::Mutex::new(0)));

        let mut handles = vec![];

        // Spawn multiple threads accessing shared data
        for i in 0..10 {
            let shared_clone = shared.clone();
            let handle = thread::spawn(move || {
                let mut value = shared_clone.lock().unwrap();
                *value += i;
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Check result
        let final_value = *shared.lock().unwrap();
        assert_eq!(final_value, 45); // 0+1+2+...+9
    });
}

#[test]
fn test_make_mut() {
    with_runtime(RuntimeConfig::default(), || {
        let mut rc1 = ScriptRc::new(vec![1, 2, 3]);
        let rc2 = rc1.clone();

        // make_mut should clone since there are multiple references
        let mutable = rc1.make_mut();
        mutable.push(4);

        assert_eq!(*rc1, vec![1, 2, 3, 4]);
        assert_eq!(*rc2, vec![1, 2, 3]); // Original unchanged
    });
}

#[test]
fn test_metadata() {
    with_runtime(RuntimeConfig::default(), || {
        let rt = runtime::core::runtime().unwrap();

        // Set and retrieve metadata
        rt.set_metadata("test_key".to_string(), "test_value".to_string());
        assert_eq!(rt.get_metadata("test_key"), Some("test_value".to_string()));

        // Non-existent key
        assert_eq!(rt.get_metadata("missing"), None);
    });
}

#[test]
fn test_type_registry() {
    with_runtime(RuntimeConfig::default(), || {
        let rt = runtime::core::runtime().unwrap();

        // Register some types
        rt.register_type::<i32>();
        rt.register_type::<String>();
        rt.register_type::<Vec<f64>>();

        // Type registry is used internally for dynamic dispatch
        // No public API to query it, but registration should not panic
    });
}

#[test]
fn test_memory_leak_detection() {
    let mut config = RuntimeConfig::default();
    config.enable_profiling = true;

    with_runtime(config, || {
        // Create an allocation that won't be freed before profiler shutdown
        let _leaked = ScriptRc::new(vec![1; 1024]);

        // Check for leaks
        assert!(runtime::profiler::check_leaks());
    });
}

#[test]
fn test_runtime_report() {
    let mut config = RuntimeConfig::default();
    config.enable_profiling = true;

    with_runtime(config, || {
        // Create some activity
        for i in 0..5 {
            let _rc = ScriptRc::new(format!("String {}", i));
        }

        // Generate report
        let report = runtime::profiler::generate_report();
        assert!(report.is_some());

        let report_text = report.unwrap();
        assert!(report_text.contains("Memory Profile Report"));
        assert!(report_text.contains("Total allocations"));
    });
}

#[test]
#[should_panic(expected = "assertion failed")]
fn test_script_assert_macro() {
    with_runtime(RuntimeConfig::default(), || {
        script_assert!(true); // Should pass
        script_assert!(false); // Should panic
    });
}

#[test]
fn test_stress_allocation() {
    with_runtime(RuntimeConfig::default(), || {
        let mut refs = Vec::new();

        // Create many small allocations
        for i in 0..1000 {
            refs.push(ScriptRc::new(i));
        }

        // Drop half of them
        refs.truncate(500);

        // Force GC
        runtime::gc::collect_cycles();

        // Create more
        for i in 1000..1500 {
            refs.push(ScriptRc::new(i));
        }

        // Verify all values are correct
        for (idx, rc) in refs.iter().enumerate() {
            if idx < 500 {
                assert_eq!(**rc, idx);
            } else {
                assert_eq!(**rc, idx + 500);
            }
        }
    });
}
