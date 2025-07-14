//! Secure FFI bindings for async runtime functions
//!
//! This module provides secure FFI functions that can be called from Script code
//! to interact with the async runtime. All security vulnerabilities from the original
//! implementation have been addressed through proper validation, error handling,
//! and memory safety measures.

use super::async_runtime_secure::{AsyncRuntimeError, BlockingExecutor, BoxedFuture, Executor};
use super::value::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Maximum allowed future count to prevent resource exhaustion
const MAX_FUTURE_COUNT: usize = 10000;

/// Maximum timeout in milliseconds to prevent infinite blocking
const MAX_TIMEOUT_MS: u64 = 300_000; // 5 minutes

/// Wrapper for raw pointers that implements Send and Sync
/// This is safe because we only use the pointer as an opaque identifier
/// and always validate it before dereferencing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PointerKey(usize);

unsafe impl Send for PointerKey {}
unsafe impl Sync for PointerKey {}

impl PointerKey {
    fn from_ptr<T>(ptr: *mut T) -> Self {
        PointerKey(ptr as usize)
    }
}

/// Secure pointer validation and tracking
#[derive(Debug)]
struct SecurePointerRegistry {
    /// Valid pointers mapped to their type information
    valid_pointers: HashMap<PointerKey, PointerInfo>,
    /// Next unique ID for tracking
    next_id: AtomicU64,
}

#[derive(Debug, Clone)]
struct PointerInfo {
    id: u64,
    type_name: &'static str,
    created_at: std::time::Instant,
}

impl SecurePointerRegistry {
    fn new() -> Self {
        Self {
            valid_pointers: HashMap::new(),
            next_id: AtomicU64::new(1),
        }
    }

    /// Register a pointer as valid with type information
    fn register_pointer<T>(&mut self, ptr: *mut T, type_name: &'static str) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let info = PointerInfo {
            id,
            type_name,
            created_at: std::time::Instant::now(),
        };
        self.valid_pointers.insert(PointerKey::from_ptr(ptr), info);
        id
    }

    /// Validate and remove a pointer from registry
    fn validate_and_remove<T>(
        &mut self,
        ptr: *mut T,
        expected_type: &str,
    ) -> Result<PointerInfo, SecurityError> {
        if ptr.is_null() {
            return Err(SecurityError::NullPointer);
        }

        let key = PointerKey::from_ptr(ptr);
        match self.valid_pointers.remove(&key) {
            Some(info) => {
                if info.type_name == expected_type {
                    Ok(info)
                } else {
                    Err(SecurityError::TypeMismatch {
                        expected: expected_type.to_string(),
                        actual: info.type_name.to_string(),
                    })
                }
            }
            None => Err(SecurityError::InvalidPointer),
        }
    }

    /// Check if a pointer is valid without removing it
    fn is_valid_pointer<T>(&self, ptr: *mut T, expected_type: &str) -> bool {
        if ptr.is_null() {
            return false;
        }

        let key = PointerKey::from_ptr(ptr);
        self.valid_pointers
            .get(&key)
            .map_or(false, |info| info.type_name == expected_type)
    }

    /// Clean up expired pointers (older than 1 hour)
    fn cleanup_expired(&mut self) {
        let now = std::time::Instant::now();
        let one_hour = Duration::from_secs(3600);

        self.valid_pointers
            .retain(|_, info| now.duration_since(info.created_at) < one_hour);
    }
}

/// Thread-safe global pointer registry
static POINTER_REGISTRY: std::sync::OnceLock<Arc<Mutex<SecurePointerRegistry>>> =
    std::sync::OnceLock::new();

fn get_pointer_registry() -> Arc<Mutex<SecurePointerRegistry>> {
    POINTER_REGISTRY
        .get_or_init(|| Arc::new(Mutex::new(SecurePointerRegistry::new())))
        .clone()
}

/// Security error types for FFI operations
#[derive(Debug, Clone)]
pub enum SecurityError {
    NullPointer,
    InvalidPointer,
    TypeMismatch { expected: String, actual: String },
    InvalidTimeout,
    ResourceLimitExceeded,
    PoisonedMutex,
    ExecutorNotInitialized,
}

impl std::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityError::NullPointer => write!(f, "Null pointer passed to FFI function"),
            SecurityError::InvalidPointer => write!(f, "Invalid or untracked pointer"),
            SecurityError::TypeMismatch { expected, actual } => {
                write!(f, "Type mismatch: expected {}, got {}", expected, actual)
            }
            SecurityError::InvalidTimeout => write!(f, "Invalid timeout value"),
            SecurityError::ResourceLimitExceeded => write!(f, "Resource limit exceeded"),
            SecurityError::PoisonedMutex => write!(f, "Mutex was poisoned"),
            SecurityError::ExecutorNotInitialized => write!(f, "Executor not properly initialized"),
        }
    }
}

impl std::error::Error for SecurityError {}

impl From<AsyncRuntimeError> for SecurityError {
    fn from(err: AsyncRuntimeError) -> Self {
        match err {
            AsyncRuntimeError::TaskLimitExceeded { .. } => SecurityError::ResourceLimitExceeded,
            AsyncRuntimeError::PoisonedMutex(_) => SecurityError::PoisonedMutex,
            _ => SecurityError::InvalidPointer, // Generic mapping for other errors
        }
    }
}

/// Secure result type for FFI operations
type SecureResult<T> = Result<T, SecurityError>;

/// Global executor instance with security tracking
static GLOBAL_EXECUTOR: std::sync::OnceLock<Arc<Mutex<Executor>>> = std::sync::OnceLock::new();

/// Initialize the global executor with security validation
fn get_global_executor() -> SecureResult<Arc<Mutex<Executor>>> {
    match GLOBAL_EXECUTOR.get_or_init(|| Executor::new()) {
        executor => Ok(executor.clone()),
    }
}

/// Secure spawn function with comprehensive validation
///
/// This function is exposed to Script code as `spawn`
#[no_mangle]
pub extern "C" fn script_spawn_secure(future_ptr: *mut BoxedFuture<()>) -> u64 {
    match script_spawn_internal(future_ptr) {
        Ok(task_id) => task_id,
        Err(err) => {
            eprintln!("Security error in script_spawn: {}", err);
            0 // Return 0 to indicate failure
        }
    }
}

fn script_spawn_internal(future_ptr: *mut BoxedFuture<()>) -> SecureResult<u64> {
    // Validate pointer
    let registry = get_pointer_registry();
    let mut registry_guard = registry.lock().map_err(|_| SecurityError::PoisonedMutex)?;

    let _info = registry_guard.validate_and_remove(future_ptr, "BoxedFuture<()>")?;
    drop(registry_guard);

    // Safely convert pointer to owned value
    let future = unsafe { Box::from_raw(future_ptr) };

    // Get executor with validation
    let executor = get_global_executor()?;

    // Spawn task
    let task_id = Executor::spawn(executor, *future)?;

    Ok(task_id.0 as u64)
}

/// Secure block_on function with timeout protection
///
/// This function is exposed to Script code as `block_on`
#[no_mangle]
pub extern "C" fn script_block_on_secure(future_ptr: *mut BoxedFuture<Value>) -> *mut Value {
    match script_block_on_internal(future_ptr) {
        Ok(value_ptr) => value_ptr,
        Err(err) => {
            eprintln!("Security error in script_block_on: {}", err);
            std::ptr::null_mut()
        }
    }
}

fn script_block_on_internal(future_ptr: *mut BoxedFuture<Value>) -> SecureResult<*mut Value> {
    // Validate pointer
    let registry = get_pointer_registry();
    let mut registry_guard = registry.lock().map_err(|_| SecurityError::PoisonedMutex)?;

    let _info = registry_guard.validate_and_remove(future_ptr, "BoxedFuture<Value>")?;
    drop(registry_guard);

    // Safely convert pointer to owned value
    let future = unsafe { Box::from_raw(future_ptr) };

    // Use the BlockingExecutor with built-in timeout protection
    let result = BlockingExecutor::block_on_with_timeout(*future, Duration::from_secs(300))?;

    // Create new pointer with tracking
    let result_ptr = Box::into_raw(Box::new(result));

    // Register the new pointer
    let registry = get_pointer_registry();
    let mut registry_guard = registry.lock().map_err(|_| SecurityError::PoisonedMutex)?;
    registry_guard.register_pointer(result_ptr, "Value");

    Ok(result_ptr)
}

/// Secure block_on with timeout validation
///
/// This function is exposed to Script code as `block_on_timeout`
#[no_mangle]
pub extern "C" fn script_block_on_timeout_secure(
    future_ptr: *mut BoxedFuture<Value>,
    timeout_ms: u64,
) -> *mut Value {
    match script_block_on_timeout_internal(future_ptr, timeout_ms) {
        Ok(value_ptr) => value_ptr,
        Err(err) => {
            eprintln!("Security error in script_block_on_timeout: {}", err);
            std::ptr::null_mut()
        }
    }
}

fn script_block_on_timeout_internal(
    future_ptr: *mut BoxedFuture<Value>,
    timeout_ms: u64,
) -> SecureResult<*mut Value> {
    // Validate timeout
    if timeout_ms > MAX_TIMEOUT_MS {
        return Err(SecurityError::InvalidTimeout);
    }

    // Validate pointer
    let registry = get_pointer_registry();
    let mut registry_guard = registry.lock().map_err(|_| SecurityError::PoisonedMutex)?;

    let _info = registry_guard.validate_and_remove(future_ptr, "BoxedFuture<Value>")?;
    drop(registry_guard);

    // Safely convert pointer to owned value
    let future = unsafe { Box::from_raw(future_ptr) };

    let timeout = Duration::from_millis(timeout_ms);

    // Use the BlockingExecutor with timeout
    let result = BlockingExecutor::block_on_with_timeout(*future, timeout)?;

    // Create new pointer with tracking
    let result_ptr = Box::into_raw(Box::new(result));

    // Register the new pointer
    let registry = get_pointer_registry();
    let mut registry_guard = registry.lock().map_err(|_| SecurityError::PoisonedMutex)?;
    registry_guard.register_pointer(result_ptr, "Value");

    Ok(result_ptr)
}

/// Secure sleep function with validation
///
/// This function is exposed to Script code as `sleep`
#[no_mangle]
pub extern "C" fn script_sleep_secure(millis: u64) -> *mut BoxedFuture<()> {
    match script_sleep_internal(millis) {
        Ok(future_ptr) => future_ptr,
        Err(err) => {
            eprintln!("Security error in script_sleep: {}", err);
            std::ptr::null_mut()
        }
    }
}

fn script_sleep_internal(millis: u64) -> SecureResult<*mut BoxedFuture<()>> {
    // Validate timeout
    if millis > MAX_TIMEOUT_MS {
        return Err(SecurityError::InvalidTimeout);
    }

    use super::async_runtime_secure::Timer;

    let timer = Timer::new(Duration::from_millis(millis))?;
    let boxed: BoxedFuture<()> = Box::new(timer);
    let future_ptr = Box::into_raw(Box::new(boxed));

    // Register the new pointer
    let registry = get_pointer_registry();
    let mut registry_guard = registry.lock().map_err(|_| SecurityError::PoisonedMutex)?;
    registry_guard.register_pointer(future_ptr, "BoxedFuture<()>");

    Ok(future_ptr)
}

/// Secure executor run function
///
/// This is typically called by the main function when async main is used
#[no_mangle]
pub extern "C" fn script_run_executor_secure() -> i32 {
    match script_run_executor_internal() {
        Ok(()) => 0, // Success
        Err(err) => {
            eprintln!("Security error in script_run_executor: {}", err);
            1 // Error code
        }
    }
}

fn script_run_executor_internal() -> SecureResult<()> {
    let executor = get_global_executor()?;
    Executor::run(executor);
    Ok(())
}

/// Secure executor shutdown function
#[no_mangle]
pub extern "C" fn script_shutdown_executor_secure() -> i32 {
    match script_shutdown_executor_internal() {
        Ok(()) => 0, // Success
        Err(err) => {
            eprintln!("Security error in script_shutdown_executor: {}", err);
            1 // Error code
        }
    }
}

fn script_shutdown_executor_internal() -> SecureResult<()> {
    let executor = get_global_executor()?;
    Executor::shutdown(executor);

    // Clean up pointer registry
    let registry = get_pointer_registry();
    let mut registry_guard = registry.lock().map_err(|_| SecurityError::PoisonedMutex)?;
    registry_guard.cleanup_expired();

    Ok(())
}

/// Secure join_all function with resource limits
///
/// This creates a future that waits for all provided futures to complete
#[no_mangle]
pub extern "C" fn script_join_all_secure(
    futures_ptr: *mut Vec<BoxedFuture<Value>>,
    count: usize,
) -> *mut BoxedFuture<Vec<Value>> {
    match script_join_all_internal(futures_ptr, count) {
        Ok(future_ptr) => future_ptr,
        Err(err) => {
            eprintln!("Security error in script_join_all: {}", err);
            std::ptr::null_mut()
        }
    }
}

fn script_join_all_internal(
    futures_ptr: *mut Vec<BoxedFuture<Value>>,
    count: usize,
) -> SecureResult<*mut BoxedFuture<Vec<Value>>> {
    use super::async_runtime_secure::JoinAll;

    // Validate count
    if count > MAX_FUTURE_COUNT {
        return Err(SecurityError::ResourceLimitExceeded);
    }

    // Validate pointer
    let registry = get_pointer_registry();
    let mut registry_guard = registry.lock().map_err(|_| SecurityError::PoisonedMutex)?;

    let _info = registry_guard.validate_and_remove(futures_ptr, "Vec<BoxedFuture<Value>>")?;
    drop(registry_guard);

    // Safely convert pointer to owned value
    let futures = unsafe { Box::from_raw(futures_ptr) };

    // Validate that the actual count matches expected count
    if futures.len() != count {
        return Err(SecurityError::ResourceLimitExceeded);
    }

    // Convert to the expected type
    let mut converted_futures = Vec::with_capacity(count);
    for future in futures.into_iter() {
        converted_futures.push(future);
    }

    let join_all = JoinAll::new(converted_futures)?;
    let boxed: BoxedFuture<Vec<Value>> = Box::new(join_all);
    let future_ptr = Box::into_raw(Box::new(boxed));

    // Register the new pointer
    let registry = get_pointer_registry();
    let mut registry_guard = registry.lock().map_err(|_| SecurityError::PoisonedMutex)?;
    registry_guard.register_pointer(future_ptr, "BoxedFuture<Vec<Value>>");

    Ok(future_ptr)
}

/// Initialize the secure FFI system
#[no_mangle]
pub extern "C" fn script_init_secure_ffi() -> i32 {
    // Initialize pointer registry
    let _registry = get_pointer_registry();

    // Initialize executor
    match get_global_executor() {
        Ok(_) => 0, // Success
        Err(err) => {
            eprintln!("Failed to initialize secure FFI: {}", err);
            1 // Error
        }
    }
}

/// Clean up the secure FFI system
#[no_mangle]
pub extern "C" fn script_cleanup_secure_ffi() -> i32 {
    match script_cleanup_internal() {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("Error during cleanup: {}", err);
            1
        }
    }
}

fn script_cleanup_internal() -> SecureResult<()> {
    // Clean up pointer registry
    let registry = get_pointer_registry();
    let mut registry_guard = registry.lock().map_err(|_| SecurityError::PoisonedMutex)?;
    registry_guard.valid_pointers.clear();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::async_runtime_secure::ScriptFuture;
    use std::task::{Poll, Waker};

    struct ImmediateFuture<T>(Option<T>);

    impl<T> ScriptFuture for ImmediateFuture<T> {
        type Output = T;

        fn poll(&mut self, _waker: &Waker) -> Poll<Self::Output> {
            Poll::Ready(self.0.take().expect("polled after completion"))
        }
    }

    #[test]
    fn test_secure_spawn_with_validation() {
        script_init_secure_ffi();

        let future = Box::new(ImmediateFuture(Some(())));
        let future_ptr = Box::into_raw(Box::new(future as BoxedFuture<()>));

        // Register the pointer before use
        let registry = get_pointer_registry();
        let mut registry_guard = registry.lock().unwrap();
        registry_guard.register_pointer(future_ptr, "BoxedFuture<()>");
        drop(registry_guard);

        let task_id = script_spawn_secure(future_ptr);
        assert!(task_id > 0);

        // Clean up
        script_shutdown_executor_secure();
        script_cleanup_secure_ffi();
    }

    #[test]
    fn test_secure_spawn_null_pointer() {
        script_init_secure_ffi();

        let task_id = script_spawn_secure(std::ptr::null_mut());
        assert_eq!(task_id, 0); // Should fail with null pointer

        script_cleanup_secure_ffi();
    }

    #[test]
    fn test_secure_spawn_invalid_pointer() {
        script_init_secure_ffi();

        // Create a pointer but don't register it
        let future = Box::new(ImmediateFuture(Some(())));
        let future_ptr = Box::into_raw(Box::new(future as BoxedFuture<()>));

        let task_id = script_spawn_secure(future_ptr);
        assert_eq!(task_id, 0); // Should fail with unregistered pointer

        // Clean up the unregistered pointer manually
        let _cleanup = unsafe { Box::from_raw(future_ptr) };

        script_cleanup_secure_ffi();
    }

    #[test]
    fn test_secure_block_on_success() {
        script_init_secure_ffi();

        let expected_value = Value::String("Secure test".to_string());
        let future = Box::new(ImmediateFuture(Some(expected_value.clone())));
        let future_ptr = Box::into_raw(Box::new(future as BoxedFuture<Value>));

        // Register the pointer
        let registry = get_pointer_registry();
        let mut registry_guard = registry.lock().unwrap();
        registry_guard.register_pointer(future_ptr, "BoxedFuture<Value>");
        drop(registry_guard);

        let result_ptr = script_block_on_secure(future_ptr);
        assert!(!result_ptr.is_null());

        // The result pointer should be automatically registered
        let registry = get_pointer_registry();
        let registry_guard = registry.lock().unwrap();
        assert!(registry_guard.is_valid_pointer(result_ptr, "Value"));
        drop(registry_guard);

        // Clean up
        let _result = unsafe { Box::from_raw(result_ptr) };
        script_cleanup_secure_ffi();
    }

    #[test]
    fn test_secure_timeout_validation() {
        script_init_secure_ffi();

        let expected_value = Value::I32(42);
        let future = Box::new(ImmediateFuture(Some(expected_value)));
        let future_ptr = Box::into_raw(Box::new(future as BoxedFuture<Value>));

        // Register the pointer
        let registry = get_pointer_registry();
        let mut registry_guard = registry.lock().unwrap();
        registry_guard.register_pointer(future_ptr, "BoxedFuture<Value>");
        drop(registry_guard);

        // Test with timeout exceeding maximum
        let result_ptr = script_block_on_timeout_secure(future_ptr, MAX_TIMEOUT_MS + 1);
        assert!(result_ptr.is_null()); // Should fail due to timeout validation

        // Clean up the unused pointer
        let _cleanup = unsafe { Box::from_raw(future_ptr) };

        script_cleanup_secure_ffi();
    }

    #[test]
    fn test_secure_sleep_validation() {
        script_init_secure_ffi();

        // Test valid sleep duration
        let future_ptr = script_sleep_secure(1000); // 1 second
        assert!(!future_ptr.is_null());

        // Verify it's registered
        let registry = get_pointer_registry();
        let registry_guard = registry.lock().unwrap();
        assert!(registry_guard.is_valid_pointer(future_ptr, "BoxedFuture<()>"));
        drop(registry_guard);

        // Clean up
        let _cleanup = unsafe { Box::from_raw(future_ptr) };

        // Test invalid sleep duration
        let invalid_future_ptr = script_sleep_secure(MAX_TIMEOUT_MS + 1);
        assert!(invalid_future_ptr.is_null());

        script_cleanup_secure_ffi();
    }

    #[test]
    fn test_resource_limit_validation() {
        script_init_secure_ffi();

        // Create a vector that exceeds the limit
        let large_vec: Vec<BoxedFuture<Value>> = Vec::new();
        let vec_ptr = Box::into_raw(Box::new(large_vec));

        // Register the pointer
        let registry = get_pointer_registry();
        let mut registry_guard = registry.lock().unwrap();
        registry_guard.register_pointer(vec_ptr, "Vec<BoxedFuture<Value>>");
        drop(registry_guard);

        // This should fail due to count exceeding MAX_FUTURE_COUNT
        let result_ptr = script_join_all_secure(vec_ptr, MAX_FUTURE_COUNT + 1);
        assert!(result_ptr.is_null());

        // Clean up the unused pointer
        let _cleanup = unsafe { Box::from_raw(vec_ptr) };

        script_cleanup_secure_ffi();
    }
}
