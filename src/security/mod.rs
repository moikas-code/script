//! Security module for Script language compiler
//!
//! This module provides comprehensive security mechanisms including:
//! - Memory safety validation and bounds checking
//! - Resource limits to prevent DoS attacks
//! - Type safety validation for dynamic operations
//! - Security configuration and monitoring

pub mod async_security;
pub mod bounds_checking;
pub mod field_validation;
pub mod module_security;
pub mod resource_limits;

pub use self::module_security::{
    ModuleIsolationBoundary, ModuleResourceUsage, ModuleSecurityEnforcer, ModuleSecurityPolicy,
};

use crate::error::{Error, ErrorKind};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Security violation types for the module system
#[derive(Debug, Clone)]
pub enum SecurityViolation {
    /// Unauthorized resource access
    UnauthorizedAccess { resource: String, operation: String },
    /// Resource limit exceeded
    ResourceLimitExceeded {
        resource: String,
        limit: usize,
        used: usize,
    },
    /// Cross-module security violation
    CrossModuleViolation {
        caller: String,
        callee: String,
        reason: String,
    },
    /// Permission denied
    PermissionDenied { permission: String, module: String },
    /// Internal security system error
    InternalError { message: String },
}

impl std::fmt::Display for SecurityViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityViolation::UnauthorizedAccess {
                resource,
                operation,
            } => {
                write!(
                    f, "Unauthorized access: cannot {} on {}", operation, resource)
            }
            SecurityViolation::ResourceLimitExceeded {
                resource,
                limit,
                used,
            } => {
                write!(
                    f, "Resource limit exceeded: {} used {} but limit is {}", resource, used, limit)
            }
            SecurityViolation::CrossModuleViolation {
                caller,
                callee,
                reason,
            } => {
                write!(
                    f, "Cross-module violation: {} cannot call {} - {}", caller, callee, reason)
            }
            SecurityViolation::PermissionDenied { permission, module } => {
                write!(
                    f, "Permission denied: module {} lacks permission {}", module, permission)
            }
            SecurityViolation::InternalError { message } => {
                write!(f, "Internal security error: {}", message)
            }
        }
    }
}

/// Security policy for module operations
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// Allow file system access
    pub allow_file_system: bool,
    /// Allow network access
    pub allow_network: bool,
    /// Allow process spawning
    pub allow_process_spawn: bool,
    /// Allow FFI calls
    pub allow_ffi: bool,
    /// Maximum memory allocation
    pub max_memory: usize,
    /// Maximum CPU time (milliseconds)
    pub max_cpu_time: u64,
}

impl SecurityPolicy {
    /// Create a permissive policy (for system modules)
    pub fn permissive() -> Self {
        SecurityPolicy {
            allow_file_system: true,
            allow_network: true,
            allow_process_spawn: true,
            allow_ffi: true,
            max_memory: usize::MAX,
            max_cpu_time: u64::MAX,
        }
    }

    /// Create a restrictive policy (for untrusted modules)
    pub fn restrictive() -> Self {
        SecurityPolicy {
            allow_file_system: false,
            allow_network: false,
            allow_process_spawn: false,
            allow_ffi: false,
            max_memory: 10_000_000, // 10MB
            max_cpu_time: 5_000,    // 5 seconds
        }
    }

    /// Create a strict policy (for sandbox modules)
    pub fn strict() -> Self {
        SecurityPolicy {
            allow_file_system: false,
            allow_network: false,
            allow_process_spawn: false,
            allow_ffi: false,
            max_memory: 1_000_000, // 1MB
            max_cpu_time: 1_000,   // 1 second
        }
    }
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        SecurityPolicy {
            allow_file_system: true,
            allow_network: true,
            allow_process_spawn: false,
            allow_ffi: false,
            max_memory: 100_000_000, // 100MB
            max_cpu_time: 30_000,    // 30 seconds
        }
    }
}

/// Security configuration for the Script compiler
/// Optimized for production performance with conditional compilation
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Enable array bounds checking (default: true for debug, false for release)
    pub enable_bounds_checking: bool,
    /// Enable field access validation (default: true for debug, false for release)
    pub enable_field_validation: bool,
    /// Enable fast-path optimizations (default: true)
    pub enable_fast_path_optimizations: bool,
    /// Batch size for resource checking (default: 100)
    pub resource_check_batch_size: usize,
    /// Maximum number of type variables per function (default: 10,000)
    pub max_type_vars: u32,
    /// Maximum number of constraints in type inference (default: 50,000)
    pub max_constraints: usize,
    /// Maximum number of function specializations (default: 1,000)
    pub max_specializations: usize,
    /// Maximum type solving iterations (default: 1,000)
    pub max_solving_iterations: usize,
    /// Maximum work queue size for monomorphization (default: 10,000)
    pub max_work_queue_size: usize,
    /// Compilation timeout in seconds (default: 30)
    pub compilation_timeout_secs: u64,
    /// Enable comprehensive security logging (default: true)
    pub enable_security_logging: bool,

    // Async/Await Security Configuration
    /// Enable async pointer validation (default: true)
    pub enable_async_pointer_validation: bool,
    /// Enable async memory safety checks (default: true for debug, false for release)
    pub enable_async_memory_safety: bool,
    /// Maximum number of concurrent async tasks (default: 10,000)
    pub max_async_tasks: usize,
    /// Maximum async task timeout in seconds (default: 300)
    pub max_async_task_timeout_secs: u64,
    /// Maximum memory per async task in bytes (default: 10MB)
    pub max_async_task_memory_bytes: usize,
    /// Enable async FFI validation (default: true)
    pub enable_async_ffi_validation: bool,
    /// Maximum FFI pointer lifetime in seconds (default: 3600)
    pub max_ffi_pointer_lifetime_secs: u64,
    /// Enable async race condition detection (default: true for debug, false for release)
    pub enable_async_race_detection: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        SecurityConfig {
            #[cfg(debug_assertions)]
            enable_bounds_checking: true,
            #[cfg(not(debug_assertions))]
            enable_bounds_checking: false,
            #[cfg(debug_assertions)]
            enable_field_validation: true,
            #[cfg(not(debug_assertions))]
            enable_field_validation: false,
            enable_fast_path_optimizations: true,
            resource_check_batch_size: 100,
            max_type_vars: 10_000,
            max_constraints: 50_000,
            max_specializations: 1_000,
            max_solving_iterations: 1_000,
            max_work_queue_size: 10_000,
            compilation_timeout_secs: 30,
            #[cfg(debug_assertions)]
            enable_security_logging: true,
            #[cfg(not(debug_assertions))]
            enable_security_logging: false,

            // Async/Await Security Defaults
            enable_async_pointer_validation: true,
            #[cfg(debug_assertions)]
            enable_async_memory_safety: true,
            #[cfg(not(debug_assertions))]
            enable_async_memory_safety: false,
            max_async_tasks: 10_000,
            max_async_task_timeout_secs: 300, // 5 minutes
            max_async_task_memory_bytes: 10 * 1024 * 1024, // 10MB
            enable_async_ffi_validation: true,
            max_ffi_pointer_lifetime_secs: 3600, // 1 hour
            #[cfg(debug_assertions)]
            enable_async_race_detection: true,
            #[cfg(not(debug_assertions))]
            enable_async_race_detection: false,
        }
    }
}

/// Security metrics for monitoring and reporting
#[derive(Debug, Default)]
pub struct SecurityMetrics {
    /// Number of bounds checks performed
    pub bounds_checks_performed: AtomicUsize,
    /// Number of bounds violations prevented
    pub bounds_violations_prevented: AtomicUsize,
    /// Number of field validations performed
    pub field_validations_performed: AtomicUsize,
    /// Number of invalid field accesses prevented
    pub invalid_field_accesses_prevented: AtomicUsize,
    /// Number of resource limit violations
    pub resource_limit_violations: AtomicUsize,
    /// Number of compilation timeouts
    pub compilation_timeouts: AtomicUsize,
    /// Total security events logged
    pub security_events_logged: AtomicUsize,

    // Async/Await Security Metrics
    /// Number of async pointer validations performed
    pub async_pointer_validations: AtomicUsize,
    /// Number of invalid async pointers prevented
    pub invalid_async_pointers_prevented: AtomicUsize,
    /// Number of async memory safety checks performed
    pub async_memory_checks: AtomicUsize,
    /// Number of async memory violations prevented
    pub async_memory_violations_prevented: AtomicUsize,
    /// Number of async FFI calls validated
    pub async_ffi_validations: AtomicUsize,
    /// Number of malicious FFI calls prevented
    pub malicious_ffi_calls_prevented: AtomicUsize,
    /// Number of async race conditions detected
    pub async_race_conditions_detected: AtomicUsize,
    /// Number of async task limit violations
    pub async_task_limit_violations: AtomicUsize,
}

impl SecurityMetrics {
    /// Create new security metrics instance
    pub fn new() -> Self {
        SecurityMetrics::default()
    }

    /// Record a bounds check operation
    pub fn record_bounds_check(&self, violation_prevented: bool) {
        self.bounds_checks_performed.fetch_add(1, Ordering::Relaxed);
        if violation_prevented {
            self.bounds_violations_prevented
                .fetch_add(1, Ordering::Relaxed);
            self.security_events_logged.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record a field validation operation
    pub fn record_field_validation(&self, invalid_access_prevented: bool) {
        self.field_validations_performed
            .fetch_add(1, Ordering::Relaxed);
        if invalid_access_prevented {
            self.invalid_field_accesses_prevented
                .fetch_add(1, Ordering::Relaxed);
            self.security_events_logged.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record a resource limit violation
    pub fn record_resource_limit_violation(&self) {
        self.resource_limit_violations
            .fetch_add(1, Ordering::Relaxed);
        self.security_events_logged.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a compilation timeout
    pub fn record_compilation_timeout(&self) {
        self.compilation_timeouts.fetch_add(1, Ordering::Relaxed);
        self.security_events_logged.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an async pointer validation operation
    pub fn record_async_pointer_validation(&self, invalid_pointer_prevented: bool) {
        self.async_pointer_validations
            .fetch_add(1, Ordering::Relaxed);
        if invalid_pointer_prevented {
            self.invalid_async_pointers_prevented
                .fetch_add(1, Ordering::Relaxed);
            self.security_events_logged.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record an async memory safety check
    pub fn record_async_memory_check(&self, violation_prevented: bool) {
        self.async_memory_checks.fetch_add(1, Ordering::Relaxed);
        if violation_prevented {
            self.async_memory_violations_prevented
                .fetch_add(1, Ordering::Relaxed);
            self.security_events_logged.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record an async FFI validation operation
    pub fn record_async_ffi_validation(&self, malicious_call_prevented: bool) {
        self.async_ffi_validations.fetch_add(1, Ordering::Relaxed);
        if malicious_call_prevented {
            self.malicious_ffi_calls_prevented
                .fetch_add(1, Ordering::Relaxed);
            self.security_events_logged.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record an async race condition detection
    pub fn record_async_race_condition(&self) {
        self.async_race_conditions_detected
            .fetch_add(1, Ordering::Relaxed);
        self.security_events_logged.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an async task limit violation
    pub fn record_async_task_limit_violation(&self) {
        self.async_task_limit_violations
            .fetch_add(1, Ordering::Relaxed);
        self.security_events_logged.fetch_add(1, Ordering::Relaxed);
    }

    /// Get security summary report
    pub fn get_security_report(&self) -> SecurityReport {
        SecurityReport {
            bounds_checks_performed: self.bounds_checks_performed.load(Ordering::Relaxed),
            bounds_violations_prevented: self.bounds_violations_prevented.load(Ordering::Relaxed),
            field_validations_performed: self.field_validations_performed.load(Ordering::Relaxed),
            invalid_field_accesses_prevented: self
                .invalid_field_accesses_prevented
                .load(Ordering::Relaxed),
            resource_limit_violations: self.resource_limit_violations.load(Ordering::Relaxed),
            compilation_timeouts: self.compilation_timeouts.load(Ordering::Relaxed),
            total_security_events: self.security_events_logged.load(Ordering::Relaxed),
            // Async security metrics
            async_pointer_validations: self.async_pointer_validations.load(Ordering::Relaxed),
            invalid_async_pointers_prevented: self
                .invalid_async_pointers_prevented
                .load(Ordering::Relaxed),
            async_memory_checks: self.async_memory_checks.load(Ordering::Relaxed),
            async_memory_violations_prevented: self
                .async_memory_violations_prevented
                .load(Ordering::Relaxed),
            async_ffi_validations: self.async_ffi_validations.load(Ordering::Relaxed),
            malicious_ffi_calls_prevented: self
                .malicious_ffi_calls_prevented
                .load(Ordering::Relaxed),
            async_race_conditions_detected: self
                .async_race_conditions_detected
                .load(Ordering::Relaxed),
            async_task_limit_violations: self.async_task_limit_violations.load(Ordering::Relaxed),
        }
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.bounds_checks_performed.store(0, Ordering::Relaxed);
        self.bounds_violations_prevented.store(0, Ordering::Relaxed);
        self.field_validations_performed.store(0, Ordering::Relaxed);
        self.invalid_field_accesses_prevented
            .store(0, Ordering::Relaxed);
        self.resource_limit_violations.store(0, Ordering::Relaxed);
        self.compilation_timeouts.store(0, Ordering::Relaxed);
        self.security_events_logged.store(0, Ordering::Relaxed);
        // Reset async metrics
        self.async_pointer_validations.store(0, Ordering::Relaxed);
        self.invalid_async_pointers_prevented
            .store(0, Ordering::Relaxed);
        self.async_memory_checks.store(0, Ordering::Relaxed);
        self.async_memory_violations_prevented
            .store(0, Ordering::Relaxed);
        self.async_ffi_validations.store(0, Ordering::Relaxed);
        self.malicious_ffi_calls_prevented
            .store(0, Ordering::Relaxed);
        self.async_race_conditions_detected
            .store(0, Ordering::Relaxed);
        self.async_task_limit_violations.store(0, Ordering::Relaxed);
    }
}

/// Security report containing current metrics
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityReport {
    pub bounds_checks_performed: usize,
    pub bounds_violations_prevented: usize,
    pub field_validations_performed: usize,
    pub invalid_field_accesses_prevented: usize,
    pub resource_limit_violations: usize,
    pub compilation_timeouts: usize,
    pub total_security_events: usize,
    // Async security metrics
    pub async_pointer_validations: usize,
    pub invalid_async_pointers_prevented: usize,
    pub async_memory_checks: usize,
    pub async_memory_violations_prevented: usize,
    pub async_ffi_validations: usize,
    pub malicious_ffi_calls_prevented: usize,
    pub async_race_conditions_detected: usize,
    pub async_task_limit_violations: usize,
}

impl SecurityReport {
    /// Calculate security score (0-100)
    pub fn calculate_security_score(&self) -> u8 {
        if self.total_security_events == 0 {
            return 100; // No security events = perfect score
        }

        let total_checks = self.bounds_checks_performed + self.field_validations_performed;
        if total_checks == 0 {
            return 50; // No checks performed = neutral score
        }

        let violations_prevented =
            self.bounds_violations_prevented + self.invalid_field_accesses_prevented;
        let critical_events = self.resource_limit_violations + self.compilation_timeouts;

        // Deduct points for critical events
        let mut score = 100u8;
        score = score.saturating_sub(critical_events as u8 * 10u8);

        // Add points for successful prevention
        let prevention_ratio = (violations_prevented as f64 / total_checks as f64) * 20.0;
        score = (score as f64 + prevention_ratio).min(100.0) as u8;

        score
    }

    /// Get security grade (A-F)
    pub fn get_security_grade(&self) -> char {
        match self.calculate_security_score() {
            90..=100 => 'A',
            80..=89 => 'B',
            70..=79 => 'C',
            60..=69 => 'D',
            _ => 'F',
        }
    }

    /// Print detailed security report
    pub fn print_detailed_report(&self) {
        println!("\n🔒 SCRIPT SECURITY REPORT");
        println!("═══════════════════════════");
        println!("Memory Safety:");
        println!(
            "  Bounds Checks: {} performed, {} violations prevented",
            self.bounds_checks_performed, self.bounds_violations_prevented
        );
        println!(
            "  Field Validations: {} performed, {} invalid accesses prevented",
            self.field_validations_performed, self.invalid_field_accesses_prevented
        );

        println!("\nAsync/Await Security:");
        println!(
            "  Pointer Validations: {} performed, {} invalid prevented",
            self.async_pointer_validations, self.invalid_async_pointers_prevented
        );
        println!(
            "  Memory Checks: {} performed, {} violations prevented",
            self.async_memory_checks, self.async_memory_violations_prevented
        );
        println!(
            "  FFI Validations: {} performed, {} malicious calls prevented",
            self.async_ffi_validations, self.malicious_ffi_calls_prevented
        );
        println!(
            "  Race Conditions Detected: {}",
            self.async_race_conditions_detected
        );
        println!(
            "  Task Limit Violations: {}",
            self.async_task_limit_violations
        );

        println!("\nResource Protection:");
        println!(
            "  Resource Limit Violations: {}",
            self.resource_limit_violations
        );
        println!("  Compilation Timeouts: {self.compilation_timeouts}");

        println!("\nOverall Assessment:");
        println!("  Security Score: {}/100", self.calculate_security_score());
        println!("  Security Grade: {}", self.get_security_grade());
        println!("  Total Security Events: {self.total_security_events}");

        let status = match self.get_security_grade() {
            'A' | 'B' => "✅ PRODUCTION READY",
            'C' => "⚠️ NEEDS IMPROVEMENT",
            'D' | 'F' => "❌ NOT PRODUCTION READY",
            _ => "❓ UNKNOWN",
        };
        println!("  Status: {status}");
    }
}

/// Resource type for limit checking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    TypeVariables,
    Constraints,
    Specializations,
    SolvingIterations,
    WorkQueueSize,
    // Async resource types
    AsyncTasks,
    AsyncTaskMemory,
    FfiPointerLifetime,
}

/// Security error types
#[derive(Debug, Clone)]
pub enum SecurityError {
    BoundsViolation {
        array_size: usize,
        index: i64,
        message: String,
    },
    InvalidFieldAccess {
        type_name: String,
        field_name: String,
        message: String,
    },
    ResourceLimitExceeded {
        resource_type: ResourceType,
        current_count: usize,
        limit: usize,
        message: String,
    },
    CompilationTimeout {
        duration: Duration,
        limit: Duration,
        message: String,
    },
    // Async-specific security errors
    AsyncPointerViolation {
        pointer_address: usize,
        validation_failed: String,
        message: String,
    },
    AsyncMemoryViolation {
        task_id: usize,
        memory_used: usize,
        memory_limit: usize,
        message: String,
    },
    AsyncFFIViolation {
        function_name: String,
        violation_type: String,
        message: String,
    },
    AsyncRaceCondition {
        resource_name: String,
        thread_ids: Vec<u64>,
        message: String,
    },
    AsyncTaskLimitExceeded {
        current_tasks: usize,
        task_limit: usize,
        message: String,
    },
    /// Lock poisoning or acquisition failure
    LockError {
        resource_name: String,
        message: String,
    },
}

impl std::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityError::BoundsViolation {
                array_size,
                index,
                message,
            } => {
                write!(
                    f, "Array bounds violation: index {} out of bounds for array of size {}. {}", index, array_size, message)
            }
            SecurityError::InvalidFieldAccess {
                type_name,
                field_name,
                message,
            } => {
                write!(
                    f, "Invalid field access: field '{}' does not exist on type '{}'. {}", field_name, type_name, message)
            }
            SecurityError::ResourceLimitExceeded {
                resource_type,
                current_count,
                limit,
                message,
            } => {
                write!(
                    f, "Resource limit exceeded: {:?} count {} exceeds limit {}. {}", resource_type, current_count, limit, message)
            }
            SecurityError::CompilationTimeout {
                duration,
                limit,
                message,
            } => {
                write!(
                    f, "Compilation timeout: duration {:?} exceeds limit {:?}. {}", duration, limit, message)
            }
            SecurityError::AsyncPointerViolation {
                pointer_address,
                validation_failed,
                message,
            } => {
                write!(
                    f, "Async pointer violation: pointer 0x{:x} failed validation ({}). {}", pointer_address, validation_failed, message)
            }
            SecurityError::AsyncMemoryViolation {
                task_id,
                memory_used,
                memory_limit,
                message,
            } => {
                write!(
                    f, "Async memory violation: task {} used {} bytes, exceeds limit {} bytes. {}", task_id, memory_used, memory_limit, message)
            }
            SecurityError::AsyncFFIViolation {
                function_name,
                violation_type,
                message,
            } => {
                write!(
                    f, "Async FFI violation: function '{}' violated {} security policy. {}", function_name, violation_type, message)
            }
            SecurityError::AsyncRaceCondition {
                resource_name,
                thread_ids,
                message,
            } => {
                write!(
                    f, "Async race condition: resource '{}' accessed concurrently by threads {:?}. {}", resource_name, thread_ids, message)
            }
            SecurityError::AsyncTaskLimitExceeded {
                current_tasks,
                task_limit,
                message,
            } => {
                write!(
                    f, "Async task limit exceeded: {} tasks exceeds limit of {} tasks. {}", current_tasks, task_limit, message)
            }
            SecurityError::LockError {
                resource_name,
                message,
            } => {
                write!(f, "Lock error on {}: {}", resource_name, message)
            }
        }
    }
}

impl std::error::Error for SecurityError {}

impl From<SecurityError> for Error {
    fn from(security_error: SecurityError) -> Self {
        Error::new(ErrorKind::SecurityViolation, security_error.to_string())
    }
}

/// Global security manager for the compiler
/// Optimized for production performance
pub struct SecurityManager {
    config: SecurityConfig,
    metrics: SecurityMetrics,
    compilation_start: Option<Instant>,
    /// Cached check results for performance
    last_resource_check: std::sync::atomic::AtomicUsize,
    /// Batch counter for resource checking optimization
    resource_check_counter: std::sync::atomic::AtomicUsize,
}

impl SecurityManager {
    /// Create new security manager with default configuration
    pub fn new() -> Self {
        SecurityManager {
            config: SecurityConfig::default(),
            metrics: SecurityMetrics::new(),
            compilation_start: None,
            last_resource_check: std::sync::atomic::AtomicUsize::new(0),
            resource_check_counter: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Create security manager with custom configuration
    pub fn with_config(config: SecurityConfig) -> Self {
        SecurityManager {
            config,
            metrics: SecurityMetrics::new(),
            compilation_start: None,
            last_resource_check: std::sync::atomic::AtomicUsize::new(0),
            resource_check_counter: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Start compilation timing
    pub fn start_compilation(&mut self) {
        self.compilation_start = Some(Instant::now());
    }

    /// Check if compilation has timed out with fast-path optimization
    pub fn check_compilation_timeout(&self) -> Result<(), SecurityError> {
        // Fast path: skip timeout checks in release builds if disabled
        #[cfg(not(debug_assertions))]
        if !self.config.enable_fast_path_optimizations {
            return Ok(());
        }

        if let Some(start_time) = self.compilation_start {
            let elapsed = start_time.elapsed();
            let limit = Duration::from_secs(self.config.compilation_timeout_secs);

            if elapsed > limit {
                self.metrics.record_compilation_timeout();
                return Err(SecurityError::CompilationTimeout {
                    duration: elapsed,
                    limit,
                    message: "Compilation exceeded maximum allowed time".to_string(),
                });
            }
        }
        Ok(())
    }

    /// Check resource limits with batched optimization
    pub fn check_resource_limit(
        &self,
        resource_type: ResourceType,
        current_count: usize,
    ) -> Result<(), SecurityError> {
        // Fast path optimization: batch resource checks for performance
        if self.config.enable_fast_path_optimizations {
            let counter = self.resource_check_counter.fetch_add(1, Ordering::Relaxed);
            if counter % self.config.resource_check_batch_size != 0 {
                // Skip check for performance, but still update counter
                return Ok(());
            }
        }

        let limit = match resource_type {
            ResourceType::TypeVariables => self.config.max_type_vars as usize,
            ResourceType::Constraints => self.config.max_constraints,
            ResourceType::Specializations => self.config.max_specializations,
            ResourceType::SolvingIterations => self.config.max_solving_iterations,
            ResourceType::WorkQueueSize => self.config.max_work_queue_size,
            ResourceType::AsyncTasks => self.config.max_async_tasks,
            ResourceType::AsyncTaskMemory => self.config.max_async_task_memory_bytes,
            ResourceType::FfiPointerLifetime => self.config.max_ffi_pointer_lifetime_secs as usize,
        };

        if current_count >= limit {
            self.metrics.record_resource_limit_violation();
            return Err(SecurityError::ResourceLimitExceeded {
                resource_type,
                current_count,
                limit,
                message: format!("Resource limit exceeded for {:?}", resource_type),
            });
        }

        // Cache the last successful check
        self.last_resource_check
            .store(current_count, Ordering::Relaxed);
        Ok(())
    }

    /// Get security configuration
    pub fn config(&self) -> &SecurityConfig {
        &self.config
    }

    /// Get security metrics
    pub fn metrics(&self) -> &SecurityMetrics {
        &self.metrics
    }

    /// Get security report
    pub fn get_security_report(&self) -> SecurityReport {
        self.metrics.get_security_report()
    }

    /// Reset metrics
    pub fn reset_metrics(&self) {
        self.metrics.reset();
    }
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert!(config.enable_bounds_checking);
        assert!(config.enable_field_validation);
        assert_eq!(config.max_type_vars, 10_000);
        assert_eq!(config.max_constraints, 50_000);
    }

    #[test]
    fn test_security_metrics() {
        let metrics = SecurityMetrics::new();

        // Test bounds check recording
        metrics.record_bounds_check(true);
        metrics.record_bounds_check(false);

        let report = metrics.get_security_report();
        assert_eq!(report.bounds_checks_performed, 2);
        assert_eq!(report.bounds_violations_prevented, 1);
    }

    #[test]
    fn test_security_report_scoring() {
        let report = SecurityReport {
            bounds_checks_performed: 100,
            bounds_violations_prevented: 5,
            field_validations_performed: 50,
            invalid_field_accesses_prevented: 2,
            resource_limit_violations: 0,
            compilation_timeouts: 0,
            total_security_events: 7,
            async_pointer_validations: 0,
            invalid_async_pointers_prevented: 0,
            async_memory_checks: 0,
            async_memory_violations_prevented: 0,
            async_ffi_validations: 0,
            malicious_ffi_calls_prevented: 0,
            async_race_conditions_detected: 0,
            async_task_limit_violations: 0,
        };

        let score = report.calculate_security_score();
        assert!(score >= 80); // Should get a good score
        assert_eq!(report.get_security_grade(), 'A');
    }

    #[test]
    fn test_security_manager() {
        let mut manager = SecurityManager::new();
        manager.start_compilation();

        // Test resource limit checking
        let result = manager.check_resource_limit(ResourceType::TypeVariables, 5000);
        assert!(result.is_ok());

        let result = manager.check_resource_limit(ResourceType::TypeVariables, 15000);
        assert!(result.is_err());
    }

    #[test]
    fn test_security_error_display() {
        let error = SecurityError::BoundsViolation {
            array_size: 10,
            index: 15,
            message: "Test bounds violation".to_string(),
        };

        let display = format!("{}", error);
        assert!(display.contains("bounds violation"));
        assert!(display.contains("15"));
        assert!(display.contains("10"));
    }
}
