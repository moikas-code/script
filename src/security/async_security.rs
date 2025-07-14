//! Async/Await Security Module for Script Language
//!
//! This module provides comprehensive security mechanisms for async/await operations:
//! - Secure pointer validation and lifetime tracking
//! - Memory safety validation for async tasks
//! - Resource limits and DoS protection
//! - FFI validation and sanitization
//! - Race condition detection and prevention

use super::{SecurityError, SecurityMetrics};
use crate::runtime::async_resource_limits::{AsyncResourceLimits, AsyncResourceMonitor};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Async security configuration
#[derive(Debug, Clone)]
pub struct AsyncSecurityConfig {
    /// Enable comprehensive pointer validation (default: true)
    pub enable_pointer_validation: bool,
    /// Enable memory safety checks (default: true for debug, false for release)
    pub enable_memory_safety: bool,
    /// Enable FFI validation (default: true)
    pub enable_ffi_validation: bool,
    /// Enable race condition detection (default: true for debug, false for release)
    pub enable_race_detection: bool,
    /// Maximum concurrent async tasks (default: 10,000)
    pub max_tasks: usize,
    /// Maximum task timeout in seconds (default: 300)
    pub max_task_timeout_secs: u64,
    /// Maximum memory per task in bytes (default: 10MB)
    pub max_task_memory_bytes: usize,
    /// Maximum FFI pointer lifetime in seconds (default: 3600)
    pub max_ffi_pointer_lifetime_secs: u64,
    /// Enable security logging (default: true)
    pub enable_logging: bool,
}

impl Default for AsyncSecurityConfig {
    fn default() -> Self {
        AsyncSecurityConfig {
            enable_pointer_validation: true,
            #[cfg(debug_assertions)]
            enable_memory_safety: true,
            #[cfg(not(debug_assertions))]
            enable_memory_safety: false,
            enable_ffi_validation: true,
            #[cfg(debug_assertions)]
            enable_race_detection: true,
            #[cfg(not(debug_assertions))]
            enable_race_detection: false,
            max_tasks: 10_000,
            max_task_timeout_secs: 300,              // 5 minutes
            max_task_memory_bytes: 10 * 1024 * 1024, // 10MB
            max_ffi_pointer_lifetime_secs: 3600,     // 1 hour
            enable_logging: true,
        }
    }
}

/// Pointer metadata for validation and tracking
#[derive(Debug, Clone)]
pub struct PointerMetadata {
    /// Unique pointer ID
    pub id: u64,
    /// Type name for type checking
    pub type_name: String,
    /// Creation timestamp
    pub created_at: Instant,
    /// Last validation timestamp
    pub last_validated: Instant,
    /// Validation count
    pub validation_count: u64,
    /// Memory size in bytes
    pub memory_size: usize,
    /// Whether pointer is currently valid
    pub is_valid: bool,
}

impl PointerMetadata {
    pub fn new(id: u64, type_name: String, memory_size: usize) -> Self {
        let now = Instant::now();
        PointerMetadata {
            id,
            type_name,
            created_at: now,
            last_validated: now,
            validation_count: 0,
            memory_size,
            is_valid: true,
        }
    }

    /// Check if pointer has expired based on lifetime limit
    pub fn is_expired(&self, max_lifetime_secs: u64) -> bool {
        self.created_at.elapsed().as_secs() > max_lifetime_secs
    }

    /// Update validation timestamp and count
    pub fn update_validation(&mut self) {
        self.last_validated = Instant::now();
        self.validation_count += 1;
    }

    /// Mark pointer as invalid
    pub fn invalidate(&mut self) {
        self.is_valid = false;
    }
}

/// Secure pointer registry for tracking and validation
pub struct SecurePointerRegistry {
    /// Map of pointer addresses to metadata
    pointers: RwLock<HashMap<usize, PointerMetadata>>,
    /// Next unique ID for pointer tracking
    next_id: AtomicU64,
    /// Total memory tracked
    total_memory: AtomicUsize,
    /// Configuration
    config: AsyncSecurityConfig,
}

impl SecurePointerRegistry {
    pub fn new() -> Self {
        SecurePointerRegistry {
            pointers: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            total_memory: AtomicUsize::new(0),
            config: AsyncSecurityConfig::default(),
        }
    }

    /// Register a new pointer with metadata
    pub fn register_pointer<T>(
        &self,
        ptr: *mut T,
        type_name: String,
    ) -> Result<u64, SecurityError> {
        if ptr.is_null() {
            return Err(SecurityError::AsyncPointerViolation {
                pointer_address: ptr as usize,
                validation_failed: "null pointer".to_string(),
                message: "Cannot register null pointer".to_string(),
            });
        }

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let memory_size = std::mem::size_of::<T>();
        let metadata = PointerMetadata::new(id, type_name, memory_size);

        let address = ptr as usize;
        let mut pointers = self
            .pointers
            .write()
            .map_err(|_| SecurityError::LockError {
                resource_name: "pointer registry".to_string(),
                message: "Failed to acquire write lock".to_string(),
            })?;
        pointers.insert(address, metadata);
        self.total_memory.fetch_add(memory_size, Ordering::Relaxed);

        Ok(id)
    }

    /// Validate a pointer and update its metadata
    pub fn validate_pointer<T>(
        &self,
        ptr: *mut T,
        max_lifetime_secs: u64,
    ) -> Result<(), SecurityError> {
        if ptr.is_null() {
            return Err(SecurityError::AsyncPointerViolation {
                pointer_address: ptr as usize,
                validation_failed: "null pointer".to_string(),
                message: "Null pointer validation failed".to_string(),
            });
        }

        let address = ptr as usize;
        let mut pointers = self
            .pointers
            .write()
            .map_err(|_| SecurityError::LockError {
                resource_name: "pointer registry".to_string(),
                message: "Failed to acquire write lock for validation".to_string(),
            })?;

        if let Some(metadata) = pointers.get_mut(&address) {
            if !metadata.is_valid {
                return Err(SecurityError::AsyncPointerViolation {
                    pointer_address: address,
                    validation_failed: "invalidated pointer".to_string(),
                    message: "Pointer has been invalidated".to_string(),
                });
            }

            if metadata.is_expired(max_lifetime_secs) {
                metadata.invalidate();
                return Err(SecurityError::AsyncPointerViolation {
                    pointer_address: address,
                    validation_failed: "expired pointer".to_string(),
                    message: format!("Pointer lifetime exceeded {} seconds", max_lifetime_secs),
                });
            }

            metadata.update_validation();
            Ok(())
        } else {
            Err(SecurityError::AsyncPointerViolation {
                pointer_address: address,
                validation_failed: "unregistered pointer".to_string(),
                message: "Pointer not found in registry".to_string(),
            })
        }
    }

    /// Unregister a pointer and clean up metadata
    pub fn unregister_pointer<T>(&self, ptr: *mut T) -> Result<(), SecurityError> {
        let address = ptr as usize;
        let mut pointers = self
            .pointers
            .write()
            .map_err(|_| SecurityError::LockError {
                resource_name: "pointer registry".to_string(),
                message: "Failed to acquire write lock for unregistration".to_string(),
            })?;

        if let Some(metadata) = pointers.remove(&address) {
            self.total_memory
                .fetch_sub(metadata.memory_size, Ordering::Relaxed);
            Ok(())
        } else {
            Err(SecurityError::AsyncPointerViolation {
                pointer_address: address,
                validation_failed: "unregistered pointer".to_string(),
                message: "Cannot unregister unknown pointer".to_string(),
            })
        }
    }

    /// Check pointer lifetime and return error if expired
    pub fn check_pointer_lifetime<T>(&self, ptr: *mut T) -> Result<(), SecurityError> {
        let address = ptr as usize;
        let pointers = self.pointers.read().map_err(|_| SecurityError::LockError {
            resource_name: "pointer registry".to_string(),
            message: "Failed to acquire read lock for lifetime check".to_string(),
        })?;

        if let Some(metadata) = pointers.get(&address) {
            if metadata.is_expired(self.config.max_ffi_pointer_lifetime_secs) {
                return Err(SecurityError::AsyncPointerViolation {
                    pointer_address: address,
                    validation_failed: "pointer expired".to_string(),
                    message: format!(
                        "Pointer lifetime exceeded {} seconds",
                        self.config.max_ffi_pointer_lifetime_secs
                    ),
                });
            }
            Ok(())
        } else {
            Err(SecurityError::AsyncPointerViolation {
                pointer_address: address,
                validation_failed: "unregistered pointer".to_string(),
                message: "Pointer not found in registry".to_string(),
            })
        }
    }

    /// Mark pointer as consumed to prevent double-free
    pub fn mark_pointer_consumed<T>(&self, ptr: *mut T) -> Result<(), SecurityError> {
        let address = ptr as usize;
        let mut pointers = self
            .pointers
            .write()
            .map_err(|_| SecurityError::LockError {
                resource_name: "pointer registry".to_string(),
                message: "Failed to acquire write lock for marking consumed".to_string(),
            })?;

        if let Some(metadata) = pointers.get_mut(&address) {
            if !metadata.is_valid {
                return Err(SecurityError::AsyncPointerViolation {
                    pointer_address: address,
                    validation_failed: "double consumption".to_string(),
                    message: "Pointer already consumed - potential double-free attempt".to_string(),
                });
            }
            metadata.is_valid = false;
            Ok(())
        } else {
            Err(SecurityError::AsyncPointerViolation {
                pointer_address: address,
                validation_failed: "unregistered pointer".to_string(),
                message: "Cannot consume unregistered pointer".to_string(),
            })
        }
    }

    /// Get total memory being tracked
    pub fn total_memory_tracked(&self) -> usize {
        self.total_memory.load(Ordering::Relaxed)
    }

    /// Get pointer count
    pub fn pointer_count(&self) -> usize {
        self.pointers.read().map(|guard| guard.len()).unwrap_or(0)
    }

    /// Clean up expired pointers
    pub fn cleanup_expired_pointers(&self, max_lifetime_secs: u64) -> usize {
        let mut pointers = match self.pointers.write() {
            Ok(pointers) => pointers,
            Err(_) => return 0, // Return 0 cleaned on lock failure
        };
        let initial_count = pointers.len();

        pointers.retain(|_, metadata| {
            if metadata.is_expired(max_lifetime_secs) {
                self.total_memory
                    .fetch_sub(metadata.memory_size, Ordering::Relaxed);
                false
            } else {
                true
            }
        });

        initial_count - pointers.len()
    }
}

/// Async task metadata for memory safety and resource tracking
#[derive(Debug)]
pub struct AsyncTaskMetadata {
    /// Unique task ID
    pub task_id: usize,
    /// Task creation timestamp
    pub created_at: Instant,
    /// Memory usage in bytes
    pub memory_usage: AtomicUsize,
    /// Task timeout duration
    pub timeout: Duration,
    /// Whether task is currently running
    pub is_running: bool,
}

impl AsyncTaskMetadata {
    pub fn new(task_id: usize, timeout: Duration) -> Self {
        AsyncTaskMetadata {
            task_id,
            created_at: Instant::now(),
            memory_usage: AtomicUsize::new(0),
            timeout,
            is_running: true,
        }
    }

    /// Check if task has timed out
    pub fn is_timed_out(&self) -> bool {
        self.created_at.elapsed() > self.timeout
    }

    /// Update memory usage
    pub fn update_memory_usage(&self, new_usage: usize) {
        self.memory_usage.store(new_usage, Ordering::Relaxed);
    }

    /// Get current memory usage
    pub fn get_memory_usage(&self) -> usize {
        self.memory_usage.load(Ordering::Relaxed)
    }
}

/// Async task manager with security monitoring
pub struct AsyncTaskManager {
    /// Map of task IDs to metadata
    tasks: RwLock<HashMap<usize, AsyncTaskMetadata>>,
    /// Next task ID
    next_task_id: AtomicUsize,
    /// Total memory usage across all tasks
    total_memory: AtomicUsize,
    /// Configuration
    config: AsyncSecurityConfig,
}

impl AsyncTaskManager {
    pub fn new(config: AsyncSecurityConfig) -> Self {
        AsyncTaskManager {
            tasks: RwLock::new(HashMap::new()),
            next_task_id: AtomicUsize::new(1),
            total_memory: AtomicUsize::new(0),
            config,
        }
    }

    /// Create a new async task with security validation
    pub fn create_task(&self, timeout_override: Option<Duration>) -> Result<usize, SecurityError> {
        let current_count = self.task_count();
        if current_count >= self.config.max_tasks {
            return Err(SecurityError::AsyncTaskLimitExceeded {
                current_tasks: current_count,
                task_limit: self.config.max_tasks,
                message: "Maximum async task limit exceeded".to_string(),
            });
        }

        let task_id = self.next_task_id.fetch_add(1, Ordering::SeqCst);
        let timeout = timeout_override
            .unwrap_or_else(|| Duration::from_secs(self.config.max_task_timeout_secs));

        let metadata = AsyncTaskMetadata::new(task_id, timeout);
        let mut tasks = self.tasks.write().map_err(|_| SecurityError::LockError {
            resource_name: "task registry".to_string(),
            message: "Failed to acquire write lock for task creation".to_string(),
        })?;
        tasks.insert(task_id, metadata);

        Ok(task_id)
    }

    /// Validate task memory usage
    pub fn validate_task_memory(
        &self,
        task_id: usize,
        memory_usage: usize,
    ) -> Result<(), SecurityError> {
        if !self.config.enable_memory_safety {
            return Ok(());
        }

        if memory_usage > self.config.max_task_memory_bytes {
            return Err(SecurityError::AsyncMemoryViolation {
                task_id,
                memory_used: memory_usage,
                memory_limit: self.config.max_task_memory_bytes,
                message: "Task memory usage exceeds limit".to_string(),
            });
        }

        let tasks = self.tasks.read().map_err(|_| SecurityError::LockError {
            resource_name: "task registry".to_string(),
            message: "Failed to acquire read lock for memory validation".to_string(),
        })?;
        if let Some(metadata) = tasks.get(&task_id) {
            let old_usage = metadata.get_memory_usage();
            metadata.update_memory_usage(memory_usage);

            // Update total memory tracking
            self.total_memory.fetch_add(memory_usage, Ordering::Relaxed);
            self.total_memory.fetch_sub(old_usage, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Remove completed task and clean up resources
    pub fn remove_task(&self, task_id: usize) -> Result<(), SecurityError> {
        let mut tasks = self.tasks.write().map_err(|_| SecurityError::LockError {
            resource_name: "task registry".to_string(),
            message: "Failed to acquire write lock for task removal".to_string(),
        })?;
        if let Some(metadata) = tasks.remove(&task_id) {
            let memory_usage = metadata.get_memory_usage();
            self.total_memory.fetch_sub(memory_usage, Ordering::Relaxed);
            Ok(())
        } else {
            Err(SecurityError::AsyncTaskLimitExceeded {
                current_tasks: 0,
                task_limit: 0,
                message: format!("Task {} not found", task_id),
            })
        }
    }

    /// Get current task count
    pub fn task_count(&self) -> usize {
        self.tasks.read().map(|guard| guard.len()).unwrap_or(0)
    }

    /// Get total memory usage across all tasks
    pub fn total_memory_usage(&self) -> usize {
        self.total_memory.load(Ordering::Relaxed)
    }

    /// Clean up timed out tasks
    pub fn cleanup_timed_out_tasks(&self) -> usize {
        let mut tasks = match self.tasks.write() {
            Ok(tasks) => tasks,
            Err(_) => return 0, // Return 0 cleaned on lock failure
        };
        let initial_count = tasks.len();

        tasks.retain(|_, metadata| {
            if metadata.is_timed_out() {
                let memory_usage = metadata.get_memory_usage();
                self.total_memory.fetch_sub(memory_usage, Ordering::Relaxed);
                false
            } else {
                true
            }
        });

        initial_count - tasks.len()
    }
}

/// FFI call validation for security
pub struct AsyncFFIValidator {
    /// Configuration
    config: AsyncSecurityConfig,
    /// Blocked function patterns
    blocked_patterns: Vec<String>,
    /// Allowed function whitelist
    allowed_functions: HashMap<String, String>, // function_name -> security_policy
}

impl AsyncFFIValidator {
    pub fn new(config: AsyncSecurityConfig) -> Self {
        let mut validator = AsyncFFIValidator {
            config,
            blocked_patterns: vec![
                "system".to_string(),
                "exec".to_string(),
                "popen".to_string(),
                "malloc".to_string(),
                "free".to_string(),
                "memcpy".to_string(),
                "strcpy".to_string(),
            ],
            allowed_functions: HashMap::new(),
        };

        // Add safe functions to whitelist
        validator
            .allowed_functions
            .insert("strlen".to_string(), "read-only".to_string());
        validator
            .allowed_functions
            .insert("strncmp".to_string(), "read-only".to_string());

        validator
    }

    /// Validate FFI function call
    pub fn validate_ffi_call(
        &self,
        function_name: &str,
        args: &[usize],
    ) -> Result<(), SecurityError> {
        if !self.config.enable_ffi_validation {
            return Ok(());
        }

        // Check against blocked patterns
        for pattern in &self.blocked_patterns {
            if function_name.contains(pattern) {
                return Err(SecurityError::AsyncFFIViolation {
                    function_name: function_name.to_string(),
                    violation_type: "blocked function pattern".to_string(),
                    message: format!("Function matches blocked pattern: {pattern}"),
                });
            }
        }

        // Check whitelist
        if let Some(_policy) = self.allowed_functions.get(function_name) {
            // Function is explicitly allowed
            return Ok(());
        }

        // Unknown function - apply strict validation
        if function_name.len() > 64 {
            return Err(SecurityError::AsyncFFIViolation {
                function_name: function_name.to_string(),
                violation_type: "function name too long".to_string(),
                message: "Function name exceeds security limit".to_string(),
            });
        }

        // Validate argument count
        if args.len() > 16 {
            return Err(SecurityError::AsyncFFIViolation {
                function_name: function_name.to_string(),
                violation_type: "too many arguments".to_string(),
                message: "Function has too many arguments".to_string(),
            });
        }

        Ok(())
    }

    /// Add function to allowed list
    pub fn allow_function(&mut self, function_name: String, security_policy: String) {
        self.allowed_functions
            .insert(function_name, security_policy);
    }

    /// Block function pattern
    pub fn block_pattern(&mut self, pattern: String) {
        self.blocked_patterns.push(pattern);
    }
}

/// Race condition detector for async operations
pub struct AsyncRaceDetector {
    /// Resource access tracking
    resource_access: RwLock<HashMap<String, Vec<(u64, Instant)>>>, // resource -> (thread_id, timestamp)
    /// Configuration
    config: AsyncSecurityConfig,
}

impl AsyncRaceDetector {
    pub fn new(config: AsyncSecurityConfig) -> Self {
        AsyncRaceDetector {
            resource_access: RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Record resource access and detect potential race conditions
    pub fn record_access(&self, resource_name: &str, thread_id: u64) -> Result<(), SecurityError> {
        if !self.config.enable_race_detection {
            return Ok(());
        }

        let now = Instant::now();
        let mut access_map =
            self.resource_access
                .write()
                .map_err(|_| SecurityError::LockError {
                    resource_name: "race detector".to_string(),
                    message: "Failed to acquire write lock for race detection".to_string(),
                })?;

        let accesses = access_map
            .entry(resource_name.to_string())
            .or_insert_with(Vec::new);

        // Clean up old accesses (older than 1 second)
        accesses.retain(|(_, timestamp)| now.duration_since(*timestamp) < Duration::from_secs(1));

        // Check for concurrent access
        let concurrent_threads: Vec<u64> = accesses
            .iter()
            .filter(|(tid, timestamp)| {
                *tid != thread_id && now.duration_since(*timestamp) < Duration::from_millis(100)
            })
            .map(|(tid, _)| *tid)
            .collect();

        if !concurrent_threads.is_empty() {
            let mut all_threads = concurrent_threads;
            all_threads.push(thread_id);

            return Err(SecurityError::AsyncRaceCondition {
                resource_name: resource_name.to_string(),
                thread_ids: all_threads,
                message: "Potential race condition detected".to_string(),
            });
        }

        accesses.push((thread_id, now));
        Ok(())
    }
}

/// Comprehensive async security manager
pub struct AsyncSecurityManager {
    /// Configuration
    config: AsyncSecurityConfig,
    /// Secure pointer registry
    pointer_registry: SecurePointerRegistry,
    /// Task manager
    task_manager: AsyncTaskManager,
    /// FFI validator
    ffi_validator: AsyncFFIValidator,
    /// Race detector
    race_detector: AsyncRaceDetector,
    /// Resource monitor for comprehensive limits
    resource_monitor: AsyncResourceMonitor,
    /// Security metrics
    metrics: Option<Arc<SecurityMetrics>>,
}

impl AsyncSecurityManager {
    /// Create new async security manager
    pub fn new() -> Self {
        let config = AsyncSecurityConfig::default();
        AsyncSecurityManager::with_config(config)
    }

    /// Create async security manager with custom configuration
    pub fn with_config(config: AsyncSecurityConfig) -> Self {
        // Create async resource limits from config
        let async_limits = AsyncResourceLimits {
            max_concurrent_tasks: config.max_tasks,
            max_task_memory_bytes: config.max_task_memory_bytes,
            max_total_async_memory_bytes: config.max_task_memory_bytes * config.max_tasks,
            max_task_execution_time: Duration::from_secs(config.max_task_timeout_secs),
            max_ffi_call_rate: 10_000.0, // 10k FFI calls/second
            max_pointer_registration_rate: 50_000.0, // 50k registrations/second
            ..Default::default()
        };

        let resource_monitor = AsyncResourceMonitor::new(async_limits);

        AsyncSecurityManager {
            pointer_registry: SecurePointerRegistry::new(),
            task_manager: AsyncTaskManager::new(config.clone()),
            ffi_validator: AsyncFFIValidator::new(config.clone()),
            race_detector: AsyncRaceDetector::new(config.clone()),
            resource_monitor,
            config,
            metrics: None,
        }
    }

    /// Set security metrics for monitoring
    pub fn with_metrics(mut self, metrics: Arc<SecurityMetrics>) -> Self {
        self.resource_monitor = self.resource_monitor.with_metrics(metrics.clone());
        self.metrics = Some(metrics);
        self
    }

    /// Register pointer with security validation
    pub fn register_pointer<T>(
        &self,
        ptr: *mut T,
        type_name: String,
    ) -> Result<u64, SecurityError> {
        // Check rate limits first
        self.resource_monitor
            .record_pointer_registration()
            .map_err(|e| SecurityError::from(e))?;

        let result = self.pointer_registry.register_pointer(ptr, type_name);

        if let Some(ref metrics) = self.metrics {
            metrics.record_async_pointer_validation(result.is_err());
        }

        result
    }

    /// Validate pointer with comprehensive checks
    pub fn validate_pointer<T>(&self, ptr: *mut T) -> Result<(), SecurityError> {
        let result = self
            .pointer_registry
            .validate_pointer(ptr, self.config.max_ffi_pointer_lifetime_secs);

        if let Some(ref metrics) = self.metrics {
            metrics.record_async_pointer_validation(result.is_err());
        }

        result
    }

    /// Create secure async task
    pub fn create_task(&self, timeout_override: Option<Duration>) -> Result<usize, SecurityError> {
        // Check resource limits first
        self.resource_monitor
            .record_task_spawn()
            .map_err(|e| SecurityError::from(e))?;

        let result = self.task_manager.create_task(timeout_override);

        if let Some(ref metrics) = self.metrics {
            if result.is_err() {
                metrics.record_async_task_limit_violation();
            }
        }

        result
    }

    /// Validate task memory usage
    pub fn validate_task_memory(
        &self,
        task_id: usize,
        memory_usage: usize,
    ) -> Result<(), SecurityError> {
        // Check resource limits first
        self.resource_monitor
            .record_task_memory_allocation(task_id, memory_usage)
            .map_err(|e| SecurityError::from(e))?;

        let result = self
            .task_manager
            .validate_task_memory(task_id, memory_usage);

        if let Some(ref metrics) = self.metrics {
            metrics.record_async_memory_check(result.is_err());
        }

        result
    }

    /// Validate FFI call
    pub fn validate_ffi_call(
        &self,
        function_name: &str,
        args: &[usize],
    ) -> Result<(), SecurityError> {
        // Check rate limits first
        self.resource_monitor
            .record_ffi_call()
            .map_err(|e| SecurityError::from(e))?;

        let result = self.ffi_validator.validate_ffi_call(function_name, args);

        if let Some(ref metrics) = self.metrics {
            metrics.record_async_ffi_validation(result.is_err());
        }

        result
    }

    /// Record resource access for race detection
    pub fn record_resource_access(
        &self,
        resource_name: &str,
        thread_id: u64,
    ) -> Result<(), SecurityError> {
        let result = self.race_detector.record_access(resource_name, thread_id);

        if let Some(ref metrics) = self.metrics {
            if result.is_err() {
                metrics.record_async_race_condition();
            }
        }

        result
    }

    /// Get security statistics
    pub fn get_security_stats(&self) -> AsyncSecurityStats {
        AsyncSecurityStats {
            pointer_count: self.pointer_registry.pointer_count(),
            total_memory_tracked: self.pointer_registry.total_memory_tracked(),
            task_count: self.task_manager.task_count(),
            total_task_memory: self.task_manager.total_memory_usage(),
        }
    }

    /// Complete a task and update resource tracking
    pub fn complete_task(
        &self,
        task_id: usize,
        execution_time: Duration,
    ) -> Result<(), SecurityError> {
        // Update resource monitor
        self.resource_monitor
            .record_task_completion(task_id, execution_time)
            .map_err(|e| SecurityError::from(e))?;

        // Remove from task manager
        let _ = self.task_manager.remove_task(task_id);

        Ok(())
    }

    /// Mark a task as failed and update resource tracking
    pub fn fail_task(&self, task_id: usize) {
        self.resource_monitor.record_task_failure(task_id);
        let _ = self.task_manager.remove_task(task_id);
    }

    /// Check system health and throttling requirements
    pub fn check_system_health(&self) -> Result<(), SecurityError> {
        self.resource_monitor
            .check_system_health()
            .map_err(|e| SecurityError::from(e))
    }

    /// Get current system throttling level
    pub fn get_throttling_level(&self) -> f64 {
        self.resource_monitor.get_throttling_level()
    }

    /// Perform cleanup of expired resources
    pub fn cleanup_expired_resources(&self) -> AsyncCleanupStats {
        // Cleanup base resources
        let expired_pointers = self
            .pointer_registry
            .cleanup_expired_pointers(self.config.max_ffi_pointer_lifetime_secs);
        let timed_out_tasks = self.task_manager.cleanup_timed_out_tasks();

        // Cleanup resource monitor
        self.resource_monitor.cleanup_resources();

        AsyncCleanupStats {
            expired_pointers,
            timed_out_tasks,
        }
    }

    /// Check pointer lifetime through pointer registry
    pub fn check_pointer_lifetime<T>(&self, ptr: *mut T) -> Result<(), SecurityError> {
        self.pointer_registry.check_pointer_lifetime(ptr)
    }

    /// Mark pointer as consumed through pointer registry
    pub fn mark_pointer_consumed<T>(&self, ptr: *mut T) -> Result<(), SecurityError> {
        self.pointer_registry.mark_pointer_consumed(ptr)
    }
}

/// Security statistics for async operations
#[derive(Debug, Clone)]
pub struct AsyncSecurityStats {
    pub pointer_count: usize,
    pub total_memory_tracked: usize,
    pub task_count: usize,
    pub total_task_memory: usize,
}

/// Cleanup statistics
#[derive(Debug, Clone)]
pub struct AsyncCleanupStats {
    pub expired_pointers: usize,
    pub timed_out_tasks: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_security_config_default() {
        let config = AsyncSecurityConfig::default();
        assert!(config.enable_pointer_validation);
        assert!(config.enable_ffi_validation);
        assert_eq!(config.max_tasks, 10_000);
        assert_eq!(config.max_task_timeout_secs, 300);
    }

    #[test]
    fn test_pointer_registry() {
        let registry = SecurePointerRegistry::new();
        let mut test_value = 42i32;

        let id = registry
            .register_pointer(&mut test_value, "i32".to_string())
            .unwrap();
        assert!(id > 0);

        let result = registry.validate_pointer(&mut test_value, 3600);
        assert!(result.is_ok());

        registry.unregister_pointer(&mut test_value).unwrap();
    }

    #[test]
    fn test_task_manager() {
        let config = AsyncSecurityConfig::default();
        let manager = AsyncTaskManager::new(config);

        let task_id = manager.create_task(None).unwrap();
        assert!(task_id > 0);

        let result = manager.validate_task_memory(task_id, 1024);
        assert!(result.is_ok());

        manager.remove_task(task_id).unwrap();
    }

    #[test]
    fn test_ffi_validator() {
        let config = AsyncSecurityConfig::default();
        let validator = AsyncFFIValidator::new(config);

        // Should block dangerous functions
        let result = validator.validate_ffi_call("system", &[]);
        assert!(result.is_err());

        // Should allow safe functions
        let result = validator.validate_ffi_call("strlen", &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_race_detector() {
        let config = AsyncSecurityConfig::default();
        let detector = AsyncRaceDetector::new(config);

        // Single access should be fine
        let result = detector.record_access("resource1", 1);
        assert!(result.is_ok());

        // Concurrent access should trigger detection
        let result = detector.record_access("resource1", 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_async_security_manager() {
        let manager = AsyncSecurityManager::new();

        let mut test_value = 42i32;
        let id = manager
            .register_pointer(&mut test_value, "i32".to_string())
            .unwrap();
        assert!(id > 0);

        let result = manager.validate_pointer(&mut test_value);
        assert!(result.is_ok());

        let task_id = manager.create_task(None).unwrap();
        assert!(task_id > 0);

        let result = manager.validate_task_memory(task_id, 1024);
        assert!(result.is_ok());

        let result = manager.validate_ffi_call("strlen", &[]);
        assert!(result.is_ok());

        let stats = manager.get_security_stats();
        assert_eq!(stats.pointer_count, 1);
        assert_eq!(stats.task_count, 1);
    }
}
