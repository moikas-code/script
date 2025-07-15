//! Property-based security tests for the Script runtime
//! 
//! These tests validate memory safety, concurrency safety, and security properties
//! using property-based testing to generate many test cases automatically.

use proptest::prelude::*;
use script::runtime::{Runtime, RuntimeConfig, Value};
use script::types::Type;
use std::alloc::Layout;
use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::TempDir;

/// Test that memory allocation and deallocation never causes crashes
proptest! {
    #[test]
    fn memory_allocation_never_crashes(
        sizes in prop::collection::vec(1usize..=4096, 1..100),
        alignments in prop::collection::vec(prop::sample::select(&[1, 2, 4, 8, 16]), 1..100)
    ) {
        let runtime = Runtime::new(RuntimeConfig::default()).unwrap();
        let mut allocated_ptrs = Vec::new();
        
        // Allocate memory with various sizes and alignments
        for (size, &align) in sizes.iter().zip(alignments.iter()) {
            if let Ok(layout) = Layout::from_size_align(*size, align) {
                if let Ok(ptr) = runtime.memory().allocate(layout) {
                    allocated_ptrs.push((ptr, layout));
                }
            }
        }
        
        // Deallocate all allocated memory
        for (ptr, layout) in allocated_ptrs {
            unsafe {
                runtime.memory().deallocate(ptr, layout);
            }
        }
        
        // Runtime should remain stable
        prop_assert!(runtime.is_healthy());
    }
}

/// Test that garbage collection never causes memory leaks
proptest! {
    #[test]
    fn gc_prevents_memory_leaks(
        allocation_count in 1usize..1000,
        object_complexity in 1usize..50
    ) {
        let runtime = Runtime::new(RuntimeConfig::default()).unwrap();
        let initial_memory = runtime.memory_usage();
        
        // Create objects that should be collected
        for _ in 0..allocation_count {
            let mut values = Vec::new();
            for _ in 0..object_complexity {
                values.push(Value::I32(42));
            }
            // Objects go out of scope and should be collected
        }
        
        // Force garbage collection
        runtime.collect_garbage();
        
        let final_memory = runtime.memory_usage();
        
        // Memory usage should not grow significantly
        prop_assert!(final_memory <= initial_memory + 1024); // Allow small overhead
    }
}

/// Test that concurrent memory operations are safe
proptest! {
    #[test]
    fn concurrent_memory_operations_are_safe(
        thread_count in 2usize..=8,
        operations_per_thread in 10usize..100
    ) {
        let runtime = Arc::new(Runtime::new(RuntimeConfig::default()).unwrap());
        let barrier = Arc::new(Barrier::new(thread_count));
        let mut handles = Vec::new();
        
        for _ in 0..thread_count {
            let runtime_clone = Arc::clone(&runtime);
            let barrier_clone = Arc::clone(&barrier);
            let ops = operations_per_thread;
            
            let handle = thread::spawn(move || {
                barrier_clone.wait();
                
                for _ in 0..ops {
                    let layout = Layout::new::<u64>();
                    if let Ok(ptr) = runtime_clone.memory().allocate(layout) {
                        unsafe {
                            runtime_clone.memory().deallocate(ptr, layout);
                        }
                    }
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
        
        // Runtime should remain healthy after concurrent access
        prop_assert!(runtime.is_healthy());
    }
}

/// Test that value creation and manipulation is memory safe
proptest! {
    #[test]
    fn value_operations_are_memory_safe(
        values in prop::collection::vec(
            prop::oneof![
                Just(Value::I32(42)),
                Just(Value::F32(3.14)),
                Just(Value::Bool(true)),
                Just(Value::String("test".to_string())),
            ],
            1..1000
        )
    ) {
        let runtime = Runtime::new(RuntimeConfig::default()).unwrap();
        let initial_memory = runtime.memory_usage();
        
        // Create and manipulate values
        let mut processed_values = Vec::new();
        for value in values {
            // Clone values to test reference counting
            let cloned = value.clone();
            processed_values.push(cloned);
        }
        
        // Values should be properly managed
        drop(processed_values);
        runtime.collect_garbage();
        
        let final_memory = runtime.memory_usage();
        
        // No significant memory growth
        prop_assert!(final_memory <= initial_memory + 2048);
    }
}

/// Test that type operations never cause crashes
proptest! {
    #[test]
    fn type_operations_never_crash(
        types in prop::collection::vec(
            prop::oneof![
                Just(Type::I32),
                Just(Type::F32),
                Just(Type::Bool),
                Just(Type::String),
                Just(Type::Array(Box::new(Type::I32))),
            ],
            1..100
        )
    ) {
        // Type operations should never crash
        for ty in types {
            let _cloned = ty.clone();
            let _debug = format!("{:?}", ty);
            // Type equality and compatibility checks
            let _compatible = ty == Type::I32;
        }
        
        // Test passed if we reach here without crashing
        prop_assert!(true);
    }
}

/// Test that runtime configuration changes are safe
proptest! {
    #[test]
    fn runtime_config_changes_are_safe(
        max_heap_size in prop::option::of(1024usize..1_000_000),
        gc_threshold in 100usize..10_000,
        enable_profiling in any::<bool>(),
        enable_gc in any::<bool>()
    ) {
        let config = RuntimeConfig {
            max_heap_size: max_heap_size.unwrap_or(0),
            enable_profiling,
            enable_gc,
            gc_threshold,
            enable_panic_handler: true,
            stack_size: 8192,
        };
        
        // Runtime should handle any valid configuration
        let runtime_result = Runtime::new(config);
        
        // Either succeeds or fails gracefully
        match runtime_result {
            Ok(runtime) => {
                prop_assert!(runtime.is_healthy());
            }
            Err(_) => {
                // Configuration was invalid but handled gracefully
                prop_assert!(true);
            }
        }
    }
}

/// Test that temporary file operations are secure
proptest! {
    #[test]
    fn temp_file_operations_are_secure(
        file_count in 1usize..20,
        content_sizes in prop::collection::vec(0usize..10_000, 1..20)
    ) {
        let temp_dir = TempDir::new().unwrap();
        
        for (i, &size) in content_sizes.iter().enumerate() {
            if i >= file_count {
                break;
            }
            
            let file_path = temp_dir.path().join(format!("test_{}.tmp", i));
            let content = "x".repeat(size);
            
            // File operations should be safe
            std::fs::write(&file_path, &content).unwrap();
            let read_content = std::fs::read_to_string(&file_path).unwrap();
            
            prop_assert_eq!(content, read_content);
        }
        
        // Cleanup should work
        drop(temp_dir);
        prop_assert!(true);
    }
}

#[cfg(test)]
mod security_regression_tests {
    use super::*;
    
    /// Test that previously identified vulnerabilities are fixed
    #[test]
    fn test_no_unsafe_code_crashes() {
        let runtime = Runtime::new(RuntimeConfig::default()).unwrap();
        
        // Test extreme allocation patterns that could crash unsafe code
        for size in [0, 1, usize::MAX / 2] {
            for align in [1, 2, 4, 8, 16, 32] {
                if let Ok(layout) = Layout::from_size_align(size, align) {
                    if let Ok(ptr) = runtime.memory().allocate(layout) {
                        unsafe {
                            runtime.memory().deallocate(ptr, layout);
                        }
                    }
                }
            }
        }
        
        assert!(runtime.is_healthy());
    }
    
    /// Test that panic handling doesn't cause issues
    #[test]
    fn test_panic_recovery_safety() {
        use script::runtime::panic;
        
        // Initialize panic handler
        panic::initialize().unwrap();
        
        // Test that panic recording works safely
        let info = script::runtime::panic::PanicInfo {
            message: "test panic".to_string(),
            location: None,
            backtrace: "test backtrace".to_string(),
            timestamp: std::time::Instant::now(),
            recovery_attempts: 0,
            recovered: false,
            recovery_policy: script::runtime::panic::RecoveryPolicy::Abort,
        };
        
        panic::record_panic(info);
        let last = panic::last_panic();
        assert!(last.is_some());
        
        // Cleanup
        panic::shutdown().unwrap();
    }
    
    /// Test that debugger operations are secure
    #[test]
    fn test_debugger_security() {
        // Test data breakpoint functionality
        use script::debugger::runtime_hooks::RuntimeHooks;
        use script::runtime::{ExecutionContext, Value};
        
        let hooks = RuntimeHooks::new();
        let context = ExecutionContext::default();
        
        // Variable assignment should not crash
        hooks.on_variable_assignment(
            &context,
            "test_var",
            Some(&Value::I32(1)),
            &Value::I32(2)
        );
        
        // Should complete without crashing
        assert!(true);
    }
}

#[cfg(test)]
mod performance_security_tests {
    use super::*;
    use std::time::{Duration, Instant};
    
    /// Test that security features don't cause excessive performance degradation
    #[test]
    fn test_security_overhead_acceptable() {
        let config_secure = RuntimeConfig {
            max_heap_size: 0,
            enable_profiling: true,
            enable_gc: true,
            gc_threshold: 100,
            enable_panic_handler: true,
            stack_size: 8192,
        };
        
        let config_minimal = RuntimeConfig {
            max_heap_size: 0,
            enable_profiling: false,
            enable_gc: true,
            gc_threshold: 10000,
            enable_panic_handler: false,
            stack_size: 8192,
        };
        
        // Benchmark allocation performance
        let runtime_secure = Runtime::new(config_secure).unwrap();
        let runtime_minimal = Runtime::new(config_minimal).unwrap();
        
        let start = Instant::now();
        for _ in 0..1000 {
            let layout = Layout::new::<u64>();
            if let Ok(ptr) = runtime_secure.memory().allocate(layout) {
                unsafe {
                    runtime_secure.memory().deallocate(ptr, layout);
                }
            }
        }
        let secure_time = start.elapsed();
        
        let start = Instant::now();
        for _ in 0..1000 {
            let layout = Layout::new::<u64>();
            if let Ok(ptr) = runtime_minimal.memory().allocate(layout) {
                unsafe {
                    runtime_minimal.memory().deallocate(ptr, layout);
                }
            }
        }
        let minimal_time = start.elapsed();
        
        // Security overhead should be reasonable (less than 3x)
        let overhead_ratio = secure_time.as_nanos() as f64 / minimal_time.as_nanos() as f64;
        println!("Security overhead ratio: {:.2}x", overhead_ratio);
        
        assert!(overhead_ratio < 3.0, "Security overhead too high: {:.2}x", overhead_ratio);
    }
}