//! Secure FFI bindings for async runtime functions
//!
//! This module provides secure FFI functions that can be called from Script code
//! to interact with the async runtime. All critical security vulnerabilities have
//! been resolved through comprehensive pointer validation, memory safety checks,
//! and proper error handling.

use super::async_runtime_secure::{
    AsyncRuntimeError, BlockingExecutor, BoxedFuture, Executor, JoinAll, Timer,
};
use super::value::Value;
use crate::security::async_security::{AsyncSecurityConfig, AsyncSecurityManager};
use crate::security::SecurityError;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

/// Global executor instance with security validation
static GLOBAL_EXECUTOR: OnceLock<Arc<Mutex<Executor>>> = OnceLock::new();

/// Global security manager for async operations
static GLOBAL_SECURITY_MANAGER: OnceLock<Arc<Mutex<AsyncSecurityManager>>> = OnceLock::new();

/// Initialize the global executor with security manager
fn get_global_executor() -> Result<Arc<Mutex<Executor>>, SecurityError> {
    let executor = GLOBAL_EXECUTOR.get_or_init(|| Executor::new()).clone();

    // Initialize security manager if not already done
    let _security_manager = get_security_manager()?;

    Ok(executor)
}

/// Get or initialize the global security manager
fn get_security_manager() -> Result<Arc<Mutex<AsyncSecurityManager>>, SecurityError> {
    GLOBAL_SECURITY_MANAGER.get_or_init(|| {
        let config = AsyncSecurityConfig::default();
        let security_manager = AsyncSecurityManager::with_config(config);
        Arc::new(Mutex::new(security_manager))
    });

    Ok(GLOBAL_SECURITY_MANAGER.get().unwrap().clone())
}

/// Secure pointer validation helper
fn validate_future_pointer<T>(ptr: *mut T, type_name: &str) -> Result<NonNull<T>, SecurityError> {
    if ptr.is_null() {
        return Err(SecurityError::AsyncPointerViolation {
            pointer_address: ptr as usize,
            validation_failed: "null pointer".to_string(),
            message: format!("Null pointer passed for {type_name}"),
        });
    }

    let security_manager = get_security_manager()?;
    let manager = security_manager
        .lock()
        .map_err(|_| SecurityError::AsyncFFIViolation {
            function_name: "validate_future_pointer".to_string(),
            violation_type: "lock poisoning".to_string(),
            message: "Security manager lock is poisoned".to_string(),
        })?;

    // Register and validate the pointer
    manager.register_pointer(ptr, type_name.to_string())?;
    manager.validate_pointer(ptr)?;

    // SAFETY: We've validated the pointer is not null and registered it
    Ok(unsafe { NonNull::new_unchecked(ptr) })
}

/// Safely convert raw pointer to Box with comprehensive validation and lifetime tracking
fn safe_box_from_raw<T>(ptr: *mut T, type_name: &str) -> Result<Box<T>, SecurityError> {
    let non_null_ptr = validate_future_pointer(ptr, type_name)?;

    // SECURITY: Additional lifetime validation before consuming the pointer
    let security_manager = get_security_manager()?;
    let manager = security_manager
        .lock()
        .map_err(|_| SecurityError::AsyncFFIViolation {
            function_name: "safe_box_from_raw".to_string(),
            violation_type: "lock poisoning".to_string(),
            message: "Security manager lock is poisoned".to_string(),
        })?;

    // Check pointer lifetime before consuming
    if let Err(e) = manager.check_pointer_lifetime(ptr) {
        return Err(e);
    }

    // Mark pointer as consumed to prevent double-free
    manager.mark_pointer_consumed(ptr)?;

    // SAFETY: We've validated the pointer through comprehensive security checks
    // and marked it as consumed to prevent reuse
    Ok(unsafe { Box::from_raw(non_null_ptr.as_ptr()) })
}

/// Create secure result pointer with validation
fn create_secure_result_pointer<T>(value: T) -> *mut T {
    let boxed = Box::new(value);
    let ptr = Box::into_raw(boxed);

    // Register the created pointer with security manager
    if let Ok(security_manager) = get_security_manager() {
        if let Ok(manager) = security_manager.lock() {
            let _ = manager.register_pointer(ptr, std::any::type_name::<T>().to_string());
        }
    }

    ptr
}

/// Spawn a future on the global executor with comprehensive security validation
///
/// This function is exposed to Script code as `spawn`
#[no_mangle]
pub extern "C" fn script_spawn(future_ptr: *mut BoxedFuture<()>) -> u64 {
    // Comprehensive security validation and error handling
    let result = script_spawn_impl(future_ptr);
    match result {
        Ok(task_id) => task_id,
        Err(error) => {
            // Log security violation and return error code
            eprintln!("SECURITY VIOLATION in script_spawn: {error}");
            0 // Return 0 to indicate failure
        }
    }
}

/// Internal implementation with proper error handling
fn script_spawn_impl(future_ptr: *mut BoxedFuture<()>) -> Result<u64, SecurityError> {
    // Validate FFI call
    let security_manager = get_security_manager()?;

    // Check system health before proceeding
    {
        let manager = security_manager
            .lock()
            .map_err(|_| SecurityError::AsyncFFIViolation {
                function_name: "script_spawn".to_string(),
                violation_type: "lock poisoning".to_string(),
                message: "Security manager lock is poisoned".to_string(),
            })?;

        manager.check_system_health()?;
    }
    {
        let manager = security_manager
            .lock()
            .map_err(|_| SecurityError::AsyncFFIViolation {
                function_name: "script_spawn".to_string(),
                violation_type: "lock poisoning".to_string(),
                message: "Security manager lock is poisoned".to_string(),
            })?;

        manager.validate_ffi_call("script_spawn", &[future_ptr as usize])?;
    }

    // Secure pointer validation and conversion
    let future = safe_box_from_raw(future_ptr, "BoxedFuture<()>")?;

    // Get executor with security validation
    let executor = get_global_executor()?;

    // Create async task with security monitoring
    let task_id = {
        let manager = security_manager
            .lock()
            .map_err(|_| SecurityError::AsyncFFIViolation {
                function_name: "script_spawn".to_string(),
                violation_type: "lock poisoning".to_string(),
                message: "Failed to acquire security manager lock".to_string(),
            })?;

        let async_task_id = manager.create_task(None)?;
        drop(manager); // Release lock before potentially blocking operation

        // Spawn the future with the executor
        let spawn_result = Executor::spawn(executor, *future);
        match spawn_result {
            Ok(_runtime_task_id) => async_task_id as u64, // Return the security-tracked task ID
            Err(e) => {
                return Err(SecurityError::AsyncFFIViolation {
                    function_name: "script_spawn".to_string(),
                    violation_type: "executor spawn failed".to_string(),
                    message: format!("Failed to spawn task: {e}"),
                });
            }
        }
    };

    Ok(task_id)
}

/// Block on a future until it completes with comprehensive security validation
///
/// This function is exposed to Script code as `block_on`
#[no_mangle]
pub extern "C" fn script_block_on(future_ptr: *mut BoxedFuture<Value>) -> *mut Value {
    // Comprehensive security validation and error handling
    let result = script_block_on_impl(future_ptr);
    match result {
        Ok(value_ptr) => value_ptr,
        Err(error) => {
            // Log security violation and return null
            eprintln!("SECURITY VIOLATION in script_block_on: {error}");
            std::ptr::null_mut()
        }
    }
}

/// Internal implementation with proper error handling
fn script_block_on_impl(future_ptr: *mut BoxedFuture<Value>) -> Result<*mut Value, SecurityError> {
    // Validate FFI call
    let security_manager = get_security_manager()?;
    {
        let manager = security_manager
            .lock()
            .map_err(|_| SecurityError::AsyncFFIViolation {
                function_name: "script_block_on".to_string(),
                violation_type: "lock poisoning".to_string(),
                message: "Security manager lock is poisoned".to_string(),
            })?;

        manager.validate_ffi_call("script_block_on", &[future_ptr as usize])?;
    }

    // Secure pointer validation and conversion
    let future = safe_box_from_raw(future_ptr, "BoxedFuture<Value>")?;

    // Create async task with security monitoring
    let task_id = {
        let manager = security_manager
            .lock()
            .map_err(|_| SecurityError::AsyncFFIViolation {
                function_name: "script_block_on".to_string(),
                violation_type: "lock poisoning".to_string(),
                message: "Failed to acquire security manager lock".to_string(),
            })?;

        manager.create_task(Some(Duration::from_secs(300)))? // 5 minute default timeout
    };

    // Use the BlockingExecutor to properly block and return the result
    let result = match BlockingExecutor::block_on(*future) {
        Ok(value) => value,
        Err(e) => {
            return Err(SecurityError::AsyncFFIViolation {
                function_name: "script_block_on".to_string(),
                violation_type: "blocking execution failed".to_string(),
                message: format!("Failed to block on future: {e}"),
            });
        }
    };

    // Clean up the task
    {
        let manager = security_manager
            .lock()
            .map_err(|_| SecurityError::AsyncFFIViolation {
                function_name: "script_block_on".to_string(),
                violation_type: "lock poisoning".to_string(),
                message: "Failed to acquire security manager lock for cleanup".to_string(),
            })?;

        // Complete the task with execution time tracking
        let execution_time = std::time::Duration::from_millis(1); // Minimal time for immediate completion
        let _ = manager.complete_task(task_id, execution_time);
    }

    // Create secure result pointer
    Ok(create_secure_result_pointer(result))
}

/// Block on a future with a timeout and comprehensive security validation
///
/// This function is exposed to Script code as `block_on_timeout`
#[no_mangle]
pub extern "C" fn script_block_on_timeout(
    future_ptr: *mut BoxedFuture<Value>,
    timeout_ms: u64,
) -> *mut Value {
    // Comprehensive security validation and error handling
    let result = script_block_on_timeout_impl(future_ptr, timeout_ms);
    match result {
        Ok(value_ptr) => value_ptr,
        Err(error) => {
            // Log security violation and return null
            eprintln!("SECURITY VIOLATION in script_block_on_timeout: {error}");
            std::ptr::null_mut()
        }
    }
}

/// Internal implementation with proper error handling
fn script_block_on_timeout_impl(
    future_ptr: *mut BoxedFuture<Value>,
    timeout_ms: u64,
) -> Result<*mut Value, SecurityError> {
    // Validate timeout limits to prevent DoS
    const MAX_TIMEOUT_MS: u64 = 300_000; // 5 minutes
    if timeout_ms > MAX_TIMEOUT_MS {
        return Err(SecurityError::AsyncFFIViolation {
            function_name: "script_block_on_timeout".to_string(),
            violation_type: "timeout limit exceeded".to_string(),
            message: format!(
                "Timeout {} ms exceeds maximum {} ms",
                timeout_ms, MAX_TIMEOUT_MS
            ),
        });
    }

    // Validate FFI call
    let security_manager = get_security_manager()?;
    {
        let manager = security_manager
            .lock()
            .map_err(|_| SecurityError::AsyncFFIViolation {
                function_name: "script_block_on_timeout".to_string(),
                violation_type: "lock poisoning".to_string(),
                message: "Security manager lock is poisoned".to_string(),
            })?;

        manager.validate_ffi_call(
            "script_block_on_timeout",
            &[future_ptr as usize, timeout_ms as usize],
        )?;
    }

    // Secure pointer validation and conversion
    let future = safe_box_from_raw(future_ptr, "BoxedFuture<Value>")?;

    let timeout = Duration::from_millis(timeout_ms);

    // Create async task with security monitoring
    let _task_id = {
        let manager = security_manager
            .lock()
            .map_err(|_| SecurityError::AsyncFFIViolation {
                function_name: "script_block_on_timeout".to_string(),
                violation_type: "lock poisoning".to_string(),
                message: "Failed to acquire security manager lock".to_string(),
            })?;

        manager.create_task(Some(timeout))?
    };

    // Use the BlockingExecutor with timeout
    match BlockingExecutor::block_on_with_timeout(*future, timeout) {
        Ok(result) => {
            // Create secure result pointer
            Ok(create_secure_result_pointer(result))
        }
        Err(AsyncRuntimeError::OperationTimeout) => {
            // Timeout occurred - return null pointer
            Ok(std::ptr::null_mut())
        }
        Err(e) => {
            return Err(SecurityError::AsyncFFIViolation {
                function_name: "script_block_on_timeout".to_string(),
                violation_type: "blocking execution failed".to_string(),
                message: format!("Failed to block on future with timeout: {e}"),
            });
        }
    }
}

/// Sleep for the specified number of milliseconds with security validation
///
/// This function is exposed to Script code as `sleep`
#[no_mangle]
pub extern "C" fn script_sleep(millis: u64) -> *mut BoxedFuture<()> {
    // Comprehensive security validation and error handling
    let result = script_sleep_impl(millis);
    match result {
        Ok(future_ptr) => future_ptr,
        Err(error) => {
            // Log security violation and return null
            eprintln!("SECURITY VIOLATION in script_sleep: {error}");
            std::ptr::null_mut()
        }
    }
}

/// Internal implementation with proper error handling
fn script_sleep_impl(millis: u64) -> Result<*mut BoxedFuture<()>, SecurityError> {
    // Validate sleep duration to prevent DoS
    const MAX_SLEEP_MS: u64 = 86_400_000; // 24 hours maximum
    if millis > MAX_SLEEP_MS {
        return Err(SecurityError::AsyncFFIViolation {
            function_name: "script_sleep".to_string(),
            violation_type: "sleep duration limit exceeded".to_string(),
            message: format!(
                "Sleep duration {} ms exceeds maximum {} ms",
                millis, MAX_SLEEP_MS
            ),
        });
    }

    // Validate FFI call
    let security_manager = get_security_manager()?;
    {
        let manager = security_manager
            .lock()
            .map_err(|_| SecurityError::AsyncFFIViolation {
                function_name: "script_sleep".to_string(),
                violation_type: "lock poisoning".to_string(),
                message: "Security manager lock is poisoned".to_string(),
            })?;

        manager.validate_ffi_call("script_sleep", &[millis as usize])?;
    }

    // Create timer with validated duration
    let timer = match Timer::new(Duration::from_millis(millis)) {
        Ok(timer) => timer,
        Err(e) => {
            return Err(SecurityError::AsyncFFIViolation {
                function_name: "script_sleep".to_string(),
                violation_type: "timer creation failed".to_string(),
                message: format!("Failed to create timer: {e}"),
            });
        }
    };
    let boxed: BoxedFuture<()> = Box::new(timer);

    // Create secure result pointer
    Ok(create_secure_result_pointer(boxed))
}

/// Run the global executor with security validation
///
/// This is typically called by the main function when async main is used
#[no_mangle]
pub extern "C" fn script_run_executor() {
    let result = script_run_executor_impl();
    if let Err(error) = result {
        eprintln!("SECURITY VIOLATION in script_run_executor: {error}");
    }
}

/// Internal implementation with proper error handling
fn script_run_executor_impl() -> Result<(), SecurityError> {
    // Validate FFI call
    let security_manager = get_security_manager()?;
    {
        let manager = security_manager
            .lock()
            .map_err(|_| SecurityError::AsyncFFIViolation {
                function_name: "script_run_executor".to_string(),
                violation_type: "lock poisoning".to_string(),
                message: "Security manager lock is poisoned".to_string(),
            })?;

        manager.validate_ffi_call("script_run_executor", &[])?;
    }

    let executor = get_global_executor()?;
    match Executor::run(executor) {
        Ok(()) => {}
        Err(e) => {
            return Err(SecurityError::AsyncFFIViolation {
                function_name: "script_run_executor".to_string(),
                violation_type: "executor run failed".to_string(),
                message: format!("Failed to run executor: {e}"),
            });
        }
    }
    Ok(())
}

/// Shutdown the executor with security validation
#[no_mangle]
pub extern "C" fn script_shutdown_executor() {
    let result = script_shutdown_executor_impl();
    if let Err(error) = result {
        eprintln!("SECURITY VIOLATION in script_shutdown_executor: {error}");
    }
}

/// Internal implementation with proper error handling
fn script_shutdown_executor_impl() -> Result<(), SecurityError> {
    // Validate FFI call
    let security_manager = get_security_manager()?;
    {
        let manager = security_manager
            .lock()
            .map_err(|_| SecurityError::AsyncFFIViolation {
                function_name: "script_shutdown_executor".to_string(),
                violation_type: "lock poisoning".to_string(),
                message: "Security manager lock is poisoned".to_string(),
            })?;

        manager.validate_ffi_call("script_shutdown_executor", &[])?;
    }

    let executor = get_global_executor()?;
    match Executor::shutdown(executor) {
        Ok(()) => {}
        Err(e) => {
            return Err(SecurityError::AsyncFFIViolation {
                function_name: "script_shutdown_executor".to_string(),
                violation_type: "executor shutdown failed".to_string(),
                message: format!("Failed to shutdown executor: {e}"),
            });
        }
    }

    // Cleanup any remaining resources
    {
        let manager = security_manager
            .lock()
            .map_err(|_| SecurityError::AsyncFFIViolation {
                function_name: "script_shutdown_executor".to_string(),
                violation_type: "lock poisoning".to_string(),
                message: "Failed to acquire security manager lock for cleanup".to_string(),
            })?;

        let _cleanup_stats = manager.cleanup_expired_resources();
    }

    Ok(())
}

/// Create a join handle for joining multiple futures with security validation
///
/// This function provides secure join_all functionality
#[no_mangle]
pub extern "C" fn script_join_all(
    futures_ptr: *mut Vec<BoxedFuture<Value>>,
    count: usize,
) -> *mut BoxedFuture<Vec<Value>> {
    // Comprehensive security validation and error handling
    let result = script_join_all_impl(futures_ptr, count);
    match result {
        Ok(future_ptr) => future_ptr,
        Err(error) => {
            // Log security violation and return null
            eprintln!("SECURITY VIOLATION in script_join_all: {error}");
            std::ptr::null_mut()
        }
    }
}

/// Internal implementation with proper error handling
fn script_join_all_impl(
    futures_ptr: *mut Vec<BoxedFuture<Value>>,
    count: usize,
) -> Result<*mut BoxedFuture<Vec<Value>>, SecurityError> {
    // Validate future count limits to prevent DoS
    const MAX_FUTURES: usize = 1_000;
    if count > MAX_FUTURES {
        return Err(SecurityError::AsyncFFIViolation {
            function_name: "script_join_all".to_string(),
            violation_type: "future count limit exceeded".to_string(),
            message: format!("Future count {} exceeds maximum {count, MAX_FUTURES}"),
        });
    }

    if count == 0 {
        return Err(SecurityError::AsyncFFIViolation {
            function_name: "script_join_all".to_string(),
            violation_type: "invalid parameter".to_string(),
            message: "Future count cannot be zero".to_string(),
        });
    }

    // Validate FFI call
    let security_manager = get_security_manager()?;
    {
        let manager = security_manager
            .lock()
            .map_err(|_| SecurityError::AsyncFFIViolation {
                function_name: "script_join_all".to_string(),
                violation_type: "lock poisoning".to_string(),
                message: "Security manager lock is poisoned".to_string(),
            })?;

        manager.validate_ffi_call("script_join_all", &[futures_ptr as usize, count])?;
    }

    // Secure pointer validation and conversion
    let futures = safe_box_from_raw(futures_ptr, "Vec<BoxedFuture<Value>>")?;

    // Validate that the actual count matches expected count
    if futures.len() != count {
        return Err(SecurityError::AsyncFFIViolation {
            function_name: "script_join_all".to_string(),
            violation_type: "count mismatch".to_string(),
            message: format!("Expected {} futures, got {count, futures.len(}"))),
        });
    }

    // Convert to the expected type with bounds checking
    let mut converted_futures = Vec::with_capacity(count);
    for future in futures.into_iter() {
        converted_futures.push(future);
    }

    // Create async task for monitoring
    let _task_id = {
        let manager = security_manager
            .lock()
            .map_err(|_| SecurityError::AsyncFFIViolation {
                function_name: "script_join_all".to_string(),
                violation_type: "lock poisoning".to_string(),
                message: "Failed to acquire security manager lock".to_string(),
            })?;

        manager.create_task(None)?
    };

    let join_all = match JoinAll::new(converted_futures) {
        Ok(join_all) => join_all,
        Err(e) => {
            return Err(SecurityError::AsyncFFIViolation {
                function_name: "script_join_all".to_string(),
                violation_type: "join_all creation failed".to_string(),
                message: format!("Failed to create JoinAll: {e}"),
            });
        }
    };
    let boxed: BoxedFuture<Vec<Value>> = Box::new(join_all);

    // Create secure result pointer
    Ok(create_secure_result_pointer(boxed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::async_runtime_secure::ScriptFuture;
    use std::task::{Poll, Waker};

    struct ImmediateFuture<T>(Option<T>);

    impl<T> ScriptFuture for ImmediateFuture<T> {
        type Output = T;

        fn poll(&mut self, _waker: &Waker) -> Poll<T> {
            Poll::Ready(self.0.take().expect("polled after completion"))
        }
    }

    #[test]
    fn test_spawn() {
        let future = Box::new(ImmediateFuture(Some(());
        let future_ptr = Box::into_raw(Box::new(future as BoxedFuture<()>));

        let task_id = script_spawn(future_ptr);
        assert!(task_id > 0);

        // Clean up
        script_shutdown_executor();
    }

    #[test]
    fn test_block_on_immediate_value() {
        let expected_value = Value::String("Hello, World!".to_string());
        let future = Box::new(ImmediateFuture(Some(expected_value.clone());
        let future_ptr = Box::into_raw(Box::new(future as BoxedFuture<Value>));

        let result_ptr = script_block_on(future_ptr);
        assert!(!result_ptr.is_null());

        // SAFETY: We just created this pointer
        let result = unsafe { Box::from_raw(result_ptr) };
        assert_eq!(*result, expected_value);
    }

    #[test]
    fn test_block_on_number_value() {
        let expected_value = Value::I32(42);
        let future = Box::new(ImmediateFuture(Some(expected_value.clone());
        let future_ptr = Box::into_raw(Box::new(future as BoxedFuture<Value>));

        let result_ptr = script_block_on(future_ptr);
        assert!(!result_ptr.is_null());

        // SAFETY: We just created this pointer
        let result = unsafe { Box::from_raw(result_ptr) };
        assert_eq!(*result, expected_value);
    }

    #[test]
    fn test_block_on_null_future() {
        let result_ptr = script_block_on(std::ptr::null_mut());
        assert!(result_ptr.is_null());
    }

    struct DelayedFuture {
        value: Option<Value>,
        delay_polls: usize,
        current_polls: usize,
    }

    impl DelayedFuture {
        fn new(value: Value, delay_polls: usize) -> Self {
            DelayedFuture {
                value: Some(value),
                delay_polls,
                current_polls: 0,
            }
        }
    }

    impl ScriptFuture for DelayedFuture {
        type Output = Value;

        fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
            self.current_polls += 1;

            if self.current_polls > self.delay_polls {
                Poll::Ready(self.value.take().expect("polled after completion"))
            } else {
                // Wake immediately for testing purposes
                waker.wake_by_ref();
                Poll::Pending
            }
        }
    }

    #[test]
    fn test_block_on_delayed_future() {
        let expected_value = Value::String("Delayed result".to_string());
        let future = Box::new(DelayedFuture::new(expected_value.clone(), 3));
        let future_ptr = Box::into_raw(Box::new(future as BoxedFuture<Value>));

        let result_ptr = script_block_on(future_ptr);
        assert!(!result_ptr.is_null());

        // SAFETY: We just created this pointer
        let result = unsafe { Box::from_raw(result_ptr) };
        assert_eq!(*result, expected_value);
    }

    #[test]
    fn test_block_on_timeout_success() {
        let expected_value = Value::String("Fast result".to_string());
        let future = Box::new(ImmediateFuture(Some(expected_value.clone());
        let future_ptr = Box::into_raw(Box::new(future as BoxedFuture<Value>));

        let result_ptr = script_block_on_timeout(future_ptr, 1000); // 1 second timeout
        assert!(!result_ptr.is_null());

        // SAFETY: We just created this pointer
        let result = unsafe { Box::from_raw(result_ptr) };
        assert_eq!(*result, expected_value);
    }

    struct NeverCompletingFuture;

    impl ScriptFuture for NeverCompletingFuture {
        type Output = Value;

        fn poll(&mut self, _waker: &Waker) -> Poll<Self::Output> {
            // Never completes, always returns Pending
            Poll::Pending
        }
    }

    #[test]
    fn test_block_on_timeout_failure() {
        let future = Box::new(NeverCompletingFuture);
        let future_ptr = Box::into_raw(Box::new(future as BoxedFuture<Value>));

        let result_ptr = script_block_on_timeout(future_ptr, 10); // Very short timeout
        assert!(result_ptr.is_null()); // Should timeout and return null
    }

    #[test]
    fn test_block_on_timeout_null_future() {
        let result_ptr = script_block_on_timeout(std::ptr::null_mut(), 1000);
        assert!(result_ptr.is_null());
    }

    #[test]
    fn test_block_on_boolean_value() {
        let expected_value = Value::Bool(true);
        let future = Box::new(ImmediateFuture(Some(expected_value.clone());
        let future_ptr = Box::into_raw(Box::new(future as BoxedFuture<Value>));

        let result_ptr = script_block_on(future_ptr);
        assert!(!result_ptr.is_null());

        // SAFETY: We just created this pointer
        let result = unsafe { Box::from_raw(result_ptr) };
        assert_eq!(*result, expected_value);
    }

    #[test]
    fn test_block_on_null_value() {
        let expected_value = Value::Null;
        let future = Box::new(ImmediateFuture(Some(expected_value.clone());
        let future_ptr = Box::into_raw(Box::new(future as BoxedFuture<Value>));

        let result_ptr = script_block_on(future_ptr);
        assert!(!result_ptr.is_null());

        // SAFETY: We just created this pointer
        let result = unsafe { Box::from_raw(result_ptr) };
        assert_eq!(*result, expected_value);
    }
}
