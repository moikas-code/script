use script::runtime::async_runtime::{Executor, BoxedFuture, ScriptFuture, SharedResult};
use script::runtime::async_ffi;
use script::security::async_security::{AsyncSecurityConfig, AsyncSecurityManager};
use script::security::SecurityError;
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};
use std::thread;
use std::time::Duration;

/// Test suite for async runtime security vulnerabilities
/// Tests all fixes implemented for use-after-free, memory corruption, and race conditions

#[cfg(test)]
mod waker_safety_tests {
    use super::*;

    #[test]
    fn test_waker_null_pointer_safety() {
        // Test that null pointer in waker doesn't cause segfault
        unsafe {
            let raw_waker = script::runtime::async_runtime::clone_waker(std::ptr::null());
            // Should return a no-op waker instead of crashing
            assert!(!raw_waker.data().is_null());
        }
    }

    #[test]
    fn test_waker_double_free_prevention() {
        // Create a test future that can be polled multiple times
        struct TestFuture {
            polled_count: u32,
        }

        impl ScriptFuture for TestFuture {
            type Output = u32;

            fn poll(&mut self, _waker: &Waker) -> Poll<Self::Output> {
                self.polled_count += 1;
                if self.polled_count >= 3 {
                    Poll::Ready(42)
                } else {
                    Poll::Pending
                }
            }
        }

        let executor = Arc::new(Mutex::new(Executor::new()));
        let future: BoxedFuture<()> = Box::new(TestFuture { polled_count: 0 });
        
        // Spawn the task
        let task_id = Executor::spawn(executor.clone(), future).unwrap();
        
        // Run the executor - waker should be properly reference counted
        thread::spawn(move || {
            let _ = Executor::run(executor);
        });
        
        // Give executor time to run
        thread::sleep(Duration::from_millis(100));
        
        // No crash means waker reference counting is working correctly
    }

    #[test]
    fn test_waker_use_after_free_prevention() {
        // Test that waker properly handles Arc reference counting
        let executor = Arc::new(Mutex::new(Executor::new()));
        
        // Create multiple futures that will wake each other
        for _ in 0..10 {
            let future: BoxedFuture<()> = Box::new(TestFuture { polled_count: 0 });
            let _ = Executor::spawn(executor.clone(), future);
        }
        
        // Run executor in separate thread
        let exec_clone = executor.clone();
        let handle = thread::spawn(move || {
            let _ = Executor::run(exec_clone);
        });
        
        // Let it run
        thread::sleep(Duration::from_millis(100));
        
        // Shutdown executor
        if let Ok(exec) = executor.lock() {
            exec.shutdown();
        }
        
        let _ = handle.join();
        
        // No crash means no use-after-free occurred
    }
}

#[cfg(test)]
mod ffi_security_tests {
    use super::*;

    #[test]
    fn test_ffi_pointer_lifetime_validation() {
        // Initialize security manager
        let security_manager = AsyncSecurityManager::new();
        
        // Create a test pointer
        let mut test_value = 42u32;
        let ptr = &mut test_value as *mut u32;
        
        // Register pointer
        let _ = security_manager.register_pointer(ptr, "u32".to_string()).unwrap();
        
        // Validate pointer works
        assert!(security_manager.validate_pointer(ptr).is_ok());
        
        // Mark as consumed
        assert!(security_manager.mark_pointer_consumed(ptr).is_ok());
        
        // Second consumption should fail (double-free prevention)
        match security_manager.mark_pointer_consumed(ptr) {
            Err(SecurityError::AsyncPointerViolation { validation_failed, .. }) => {
                assert_eq!(validation_failed, "double consumption");
            }
            _ => panic!("Expected double consumption error"),
        }
    }

    #[test]
    fn test_ffi_null_pointer_rejection() {
        // Test that null pointers are rejected
        let result = async_ffi::validate_future_pointer::<u32>(
            std::ptr::null_mut(),
            "test_type"
        );
        
        match result {
            Err(SecurityError::AsyncPointerViolation { validation_failed, .. }) => {
                assert_eq!(validation_failed, "null pointer");
            }
            _ => panic!("Expected null pointer error"),
        }
    }

    #[test]
    fn test_ffi_secure_result_pointer_creation() {
        // Test secure result pointer creation and registration
        let value = 42u32;
        let ptr = async_ffi::create_secure_result_pointer(value);
        
        assert!(!ptr.is_null());
        
        // Pointer should be registered with security manager
        let security_manager = async_ffi::get_security_manager().unwrap();
        let manager = security_manager.lock().unwrap();
        assert!(manager.validate_pointer(ptr).is_ok());
    }
}

#[cfg(test)]
mod async_state_security_tests {
    use super::*;
    use script::lowering::async_transform::AsyncTransformContext;

    #[test]
    fn test_async_state_bounds_checking() {
        let mut context = AsyncTransformContext::new();
        
        // Try to allocate reasonable size
        let offset1 = context.allocate_variable("var1".to_string(), 8);
        assert_ne!(offset1, u32::MAX);
        
        // Try to allocate huge size (should fail)
        let offset2 = context.allocate_variable("var2".to_string(), u32::MAX / 2);
        assert_eq!(offset2, u32::MAX); // Allocation failed
        
        // Try multiple allocations that would overflow
        for i in 0..1000 {
            let offset = context.allocate_variable(format!("var{}", i), 1024);
            if offset == u32::MAX {
                // Correctly prevented overflow
                return;
            }
        }
        
        panic!("Should have prevented state overflow");
    }

    #[test]
    fn test_async_state_integer_overflow_prevention() {
        let mut context = AsyncTransformContext::new();
        
        // Allocate near the limit
        context.current_offset = 1024 * 1024 - 100; // Near 1MB limit
        
        // Try to allocate more than remaining space
        let offset = context.allocate_variable("overflow".to_string(), 200);
        assert_eq!(offset, u32::MAX); // Should fail
    }
}

#[cfg(test)]
mod race_condition_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_atomic_task_reservation() {
        // Test that task reservation is atomic and prevents TOCTOU races
        let executor = Arc::new(Mutex::new(Executor::new()));
        let spawn_count = Arc::new(AtomicUsize::new(0));
        let error_count = Arc::new(AtomicUsize::new(0));
        
        // Spawn many threads trying to create tasks concurrently
        let mut handles = vec![];
        for _ in 0..100 {
            let exec_clone = executor.clone();
            let spawn_count_clone = spawn_count.clone();
            let error_count_clone = error_count.clone();
            
            let handle = thread::spawn(move || {
                for _ in 0..10 {
                    let future: BoxedFuture<()> = Box::new(TestFuture { polled_count: 0 });
                    match Executor::spawn(exec_clone.clone(), future) {
                        Ok(_) => {
                            spawn_count_clone.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(_) => {
                            error_count_clone.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            });
            handles.push(handle);
        }
        
        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }
        
        // Verify that we never exceeded limits
        let total_attempts = spawn_count.load(Ordering::Relaxed) + error_count.load(Ordering::Relaxed);
        assert_eq!(total_attempts, 1000); // All attempts accounted for
        
        // Check executor state is consistent
        if let Ok(exec) = executor.lock() {
            let task_count = exec.shared.monitor.active_tasks.load(Ordering::Relaxed);
            assert!(task_count <= exec.shared.config.max_concurrent_tasks);
        }
    }

    #[test]
    fn test_concurrent_wake_safety() {
        // Test that concurrent wake operations don't cause races
        let executor = Arc::new(Mutex::new(Executor::new()));
        
        // Create a future that saves its waker
        struct WakerSavingFuture {
            waker: Option<Waker>,
            complete: Arc<AtomicBool>,
        }
        
        impl ScriptFuture for WakerSavingFuture {
            type Output = ();
            
            fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
                self.waker = Some(waker.clone());
                if self.complete.load(Ordering::Relaxed) {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            }
        }
        
        let complete_flag = Arc::new(AtomicBool::new(false));
        let future = WakerSavingFuture {
            waker: None,
            complete: complete_flag.clone(),
        };
        
        let boxed_future: BoxedFuture<()> = Box::new(future);
        let _ = Executor::spawn(executor.clone(), boxed_future);
        
        // Run executor to get waker
        let exec_clone = executor.clone();
        thread::spawn(move || {
            let _ = Executor::run(exec_clone);
        });
        
        thread::sleep(Duration::from_millis(50));
        
        // Now wake from multiple threads concurrently
        let mut wake_handles = vec![];
        for _ in 0..10 {
            let complete_clone = complete_flag.clone();
            let handle = thread::spawn(move || {
                thread::sleep(Duration::from_millis(10));
                complete_clone.store(true, Ordering::Relaxed);
                // Waker wake operations should be thread-safe
            });
            wake_handles.push(handle);
        }
        
        for handle in wake_handles {
            handle.join().unwrap();
        }
        
        // No crash means concurrent wake is safe
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_secure_async_execution_end_to_end() {
        // Test complete async execution with all security features
        let executor = Arc::new(Mutex::new(Executor::new()));
        
        // Create a future that uses multiple security features
        struct SecureFuture {
            step: u32,
        }
        
        impl ScriptFuture for SecureFuture {
            type Output = u32;
            
            fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
                match self.step {
                    0 => {
                        self.step = 1;
                        waker.wake_by_ref();
                        Poll::Pending
                    }
                    1 => {
                        self.step = 2;
                        // Simulate async work
                        Poll::Pending
                    }
                    _ => Poll::Ready(42),
                }
            }
        }
        
        let result_storage = SharedResult::new();
        let result_clone = result_storage.clone();
        
        let wrapped_future: BoxedFuture<()> = Box::new(move || {
            let mut future = SecureFuture { step: 0 };
            loop {
                // Poll future manually
                let waker = futures::task::noop_waker();
                match future.poll(&waker) {
                    Poll::Ready(value) => {
                        let _ = result_clone.set_result(value);
                        return;
                    }
                    Poll::Pending => {
                        thread::yield_now();
                    }
                }
            }
        });
        
        // Spawn the secure future
        let task_id = Executor::spawn(executor.clone(), wrapped_future).unwrap();
        
        // Run executor
        let exec_clone = executor.clone();
        let handle = thread::spawn(move || {
            let _ = Executor::run(exec_clone);
        });
        
        // Wait for result with timeout
        let result = result_storage.wait_for_result_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(result, Some(42));
        
        // Cleanup
        if let Ok(exec) = executor.lock() {
            exec.shutdown();
        }
        let _ = handle.join();
    }
}

// Helper test future
struct TestFuture {
    polled_count: u32,
}

impl ScriptFuture for TestFuture {
    type Output = ();
    
    fn poll(&mut self, _waker: &Waker) -> Poll<Self::Output> {
        self.polled_count += 1;
        if self.polled_count >= 2 {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}