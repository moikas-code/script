//! Resource limits and monitoring for DoS protection
//!
//! This module provides comprehensive resource monitoring and limits to protect
//! against denial-of-service attacks through resource exhaustion during compilation.
//!
//! # Security Features
//! - Timeout protection for all compilation phases
//! - Memory usage monitoring and limits
//! - Iteration count limits for recursive operations
//! - Graceful degradation when limits are reached
//!
//! # Usage
//! ```
//! use crate::compilation::resource_limits::{ResourceMonitor, ResourceLimits};
//!
//! let limits = ResourceLimits::production();
//! let mut monitor = ResourceMonitor::new(limits);
//!
//! // Check resource usage during compilation
//! monitor.check_iteration_limit("type_inference", 1000)?;
//! monitor.check_timeout("monomorphization")?;
//! ```

use crate::error::{Error, ErrorKind};
use std::collections::HashMap;
use std::time::{Duration, Instant};

// Platform-specific memory measurement
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(not(target_os = "linux"))]
use std::process;

/// Maximum safe limits for production environments
const MAX_SAFE_ITERATIONS: usize = 100_000;
const MAX_SAFE_TIMEOUT_SECS: u64 = 60;
const MAX_SAFE_MEMORY_MB: usize = 1024;
const MAX_SAFE_DEPTH: usize = 1000;

/// Configuration for resource limits during compilation
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum iterations for recursive operations (type inference, constraint solving)
    pub max_iterations: usize,
    /// Maximum time allowed for each compilation phase
    pub phase_timeout: Duration,
    /// Maximum total compilation time
    pub total_timeout: Duration,
    /// Maximum memory usage in bytes
    pub max_memory_bytes: usize,
    /// Maximum recursion depth for type operations
    pub max_recursion_depth: usize,
    /// Maximum number of type variables that can be created
    pub max_type_variables: usize,
    /// Maximum number of constraints in the constraint solver
    pub max_constraints: usize,
    /// Maximum number of generic specializations
    pub max_specializations: usize,
    /// Maximum work queue size for monomorphization
    pub max_work_queue_size: usize,
}

impl ResourceLimits {
    /// Create resource limits suitable for production environments
    pub fn production() -> Self {
        Self {
            max_iterations: MAX_SAFE_ITERATIONS,
            phase_timeout: Duration::from_secs(MAX_SAFE_TIMEOUT_SECS),
            total_timeout: Duration::from_secs(MAX_SAFE_TIMEOUT_SECS * 3), // 3x phase timeout
            max_memory_bytes: MAX_SAFE_MEMORY_MB * 1024 * 1024,
            max_recursion_depth: MAX_SAFE_DEPTH,
            max_type_variables: MAX_SAFE_ITERATIONS,
            max_constraints: MAX_SAFE_ITERATIONS * 2,
            max_specializations: 1_000,  // Same as monomorphization limit
            max_work_queue_size: 10_000, // Same as monomorphization limit
        }
    }

    /// Create resource limits suitable for development environments (more permissive)
    pub fn development() -> Self {
        Self {
            max_iterations: MAX_SAFE_ITERATIONS * 2,
            phase_timeout: Duration::from_secs(MAX_SAFE_TIMEOUT_SECS * 2),
            total_timeout: Duration::from_secs(MAX_SAFE_TIMEOUT_SECS * 6),
            max_memory_bytes: MAX_SAFE_MEMORY_MB * 2 * 1024 * 1024,
            max_recursion_depth: MAX_SAFE_DEPTH * 2,
            max_type_variables: MAX_SAFE_ITERATIONS * 2,
            max_constraints: MAX_SAFE_ITERATIONS * 4,
            max_specializations: 2_000,
            max_work_queue_size: 20_000,
        }
    }

    /// Create resource limits suitable for testing environments (very permissive)
    pub fn testing() -> Self {
        Self {
            max_iterations: usize::MAX,
            phase_timeout: Duration::from_secs(300), // 5 minutes for tests
            total_timeout: Duration::from_secs(600), // 10 minutes total
            max_memory_bytes: usize::MAX,
            max_recursion_depth: usize::MAX,
            max_type_variables: usize::MAX,
            max_constraints: usize::MAX,
            max_specializations: usize::MAX,
            max_work_queue_size: usize::MAX,
        }
    }

    /// Create custom resource limits
    pub fn custom() -> ResourceLimitsBuilder {
        ResourceLimitsBuilder::new()
    }

    /// Validate that the limits are reasonable and secure
    pub fn validate(&self) -> Result<(), Error> {
        if self.max_iterations == 0 {
            return Err(Error::new(
                ErrorKind::Configuration,
                "max_iterations must be greater than 0",
            ));
        }

        if self.phase_timeout.as_secs() == 0 {
            return Err(Error::new(
                ErrorKind::Configuration,
                "phase_timeout must be greater than 0",
            ));
        }

        if self.total_timeout < self.phase_timeout {
            return Err(Error::new(
                ErrorKind::Configuration,
                "total_timeout must be greater than or equal to phase_timeout",
            ));
        }

        if self.max_memory_bytes == 0 {
            return Err(Error::new(
                ErrorKind::Configuration,
                "max_memory_bytes must be greater than 0",
            ));
        }

        Ok(())
    }
}

/// Builder for creating custom resource limits
#[derive(Debug)]
pub struct ResourceLimitsBuilder {
    limits: ResourceLimits,
}

impl ResourceLimitsBuilder {
    fn new() -> Self {
        Self {
            limits: ResourceLimits::production(),
        }
    }

    pub fn max_iterations(mut self, max: usize) -> Self {
        self.limits.max_iterations = max;
        self
    }

    pub fn phase_timeout(mut self, timeout: Duration) -> Self {
        self.limits.phase_timeout = timeout;
        self
    }

    pub fn total_timeout(mut self, timeout: Duration) -> Self {
        self.limits.total_timeout = timeout;
        self
    }

    pub fn max_memory_bytes(mut self, bytes: usize) -> Self {
        self.limits.max_memory_bytes = bytes;
        self
    }

    pub fn max_recursion_depth(mut self, depth: usize) -> Self {
        self.limits.max_recursion_depth = depth;
        self
    }

    pub fn build(self) -> Result<ResourceLimits, Error> {
        self.limits.validate()?;
        Ok(self.limits)
    }
}

/// Tracks resource usage during compilation phases
#[derive(Debug)]
pub struct ResourceMonitor {
    /// Configuration limits
    limits: ResourceLimits,
    /// Start time of compilation
    compilation_start: Instant,
    /// Start times of individual phases
    phase_starts: HashMap<String, Instant>,
    /// Iteration counts for different operations
    iteration_counts: HashMap<String, usize>,
    /// Recursion depth tracking
    recursion_depths: HashMap<String, usize>,
    /// Memory usage tracking (approximated)
    memory_usage: usize,
    /// Type variable counter
    type_variable_count: usize,
    /// Constraint counter
    constraint_count: usize,
    /// Specialization counter
    specialization_count: usize,
    /// Work queue size tracking
    work_queue_sizes: HashMap<String, usize>,
}

impl ResourceMonitor {
    /// Create a new resource monitor with the given limits
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            limits,
            compilation_start: Instant::now(),
            phase_starts: HashMap::new(),
            iteration_counts: HashMap::new(),
            recursion_depths: HashMap::new(),
            memory_usage: 0,
            type_variable_count: 0,
            constraint_count: 0,
            specialization_count: 0,
            work_queue_sizes: HashMap::new(),
        }
    }

    /// Start monitoring a compilation phase
    pub fn start_phase(&mut self, phase_name: &str) -> Result<(), Error> {
        self.phase_starts
            .insert(phase_name.to_string(), Instant::now());
        Ok(())
    }

    /// End monitoring a compilation phase
    pub fn end_phase(&mut self, phase_name: &str) {
        self.phase_starts.remove(phase_name);
    }

    /// Check if total compilation time limit has been exceeded
    pub fn check_total_timeout(&self) -> Result<(), Error> {
        if self.compilation_start.elapsed() > self.limits.total_timeout {
            return Err(Error::security_violation(format!(
                "Total compilation timeout exceeded: {} seconds. This prevents DoS attacks through long-running compilation.",
                self.limits.total_timeout.as_secs()
            )));
        }
        Ok(())
    }

    /// Check if phase timeout has been exceeded
    pub fn check_phase_timeout(&self, phase_name: &str) -> Result<(), Error> {
        if let Some(start_time) = self.phase_starts.get(phase_name) {
            if start_time.elapsed() > self.limits.phase_timeout {
                return Err(Error::security_violation(format!(
                    "Phase '{}' timeout exceeded: {} seconds. This prevents DoS attacks through long-running compilation phases.",
                    phase_name,
                    self.limits.phase_timeout.as_secs()
                )));
            }
        }
        Ok(())
    }

    /// Check and increment iteration count for an operation
    pub fn check_iteration_limit(
        &mut self,
        operation: &str,
        increment: usize,
    ) -> Result<(), Error> {
        let current = self
            .iteration_counts
            .entry(operation.to_string())
            .or_insert(0);
        *current += increment;

        if *current > self.limits.max_iterations {
            return Err(Error::security_violation(format!(
                "Iteration limit exceeded for operation '{}': {} > {}. This prevents DoS attacks through infinite loops.",
                operation,
                *current,
                self.limits.max_iterations
            )));
        }
        Ok(())
    }

    /// Check and track recursion depth
    pub fn check_recursion_depth(&mut self, operation: &str, depth: usize) -> Result<(), Error> {
        self.recursion_depths.insert(operation.to_string(), depth);

        if depth > self.limits.max_recursion_depth {
            return Err(Error::security_violation(format!(
                "Recursion depth limit exceeded for operation '{}': {} > {}. This prevents DoS attacks through deep recursion.",
                operation,
                depth,
                self.limits.max_recursion_depth
            )));
        }
        Ok(())
    }

    /// Track memory usage (approximated)
    pub fn add_memory_usage(&mut self, bytes: usize) -> Result<(), Error> {
        self.memory_usage += bytes;

        if self.memory_usage > self.limits.max_memory_bytes {
            return Err(Error::security_violation(format!(
                "Memory usage limit exceeded: {} bytes > {} bytes. This prevents DoS attacks through memory exhaustion.",
                self.memory_usage,
                self.limits.max_memory_bytes
            )));
        }
        Ok(())
    }

    /// Check current system memory usage
    pub fn check_system_memory(&self) -> Result<(), Error> {
        let current_memory = Self::get_current_memory_usage();

        if current_memory > self.limits.max_memory_bytes {
            return Err(Error::security_violation(format!(
                "System memory usage limit exceeded: {} MB > {} MB. This prevents DoS attacks through memory exhaustion.",
                current_memory / (1024 * 1024),
                self.limits.max_memory_bytes / (1024 * 1024)
            )));
        }
        Ok(())
    }

    /// Get current process memory usage in bytes
    #[cfg(target_os = "linux")]
    fn get_current_memory_usage() -> usize {
        // Try to read from /proc/self/status for Linux
        if let Ok(status) = fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<usize>() {
                            return kb * 1024; // Convert KB to bytes
                        }
                    }
                }
            }
        }
        0 // Return 0 if we can't determine memory usage
    }

    /// Get current process memory usage in bytes (fallback for non-Linux)
    #[cfg(not(target_os = "linux"))]
    fn get_current_memory_usage() -> usize {
        // On non-Linux systems, we can't easily get memory usage
        // Return 0 to effectively disable memory checking
        0
    }

    /// Track type variable creation
    pub fn add_type_variable(&mut self) -> Result<(), Error> {
        self.type_variable_count += 1;

        if self.type_variable_count > self.limits.max_type_variables {
            return Err(Error::security_violation(format!(
                "Type variable limit exceeded: {} > {}. This prevents DoS attacks through type variable explosion.",
                self.type_variable_count,
                self.limits.max_type_variables
            )));
        }
        Ok(())
    }

    /// Track constraint creation
    pub fn add_constraint(&mut self) -> Result<(), Error> {
        self.constraint_count += 1;

        if self.constraint_count > self.limits.max_constraints {
            return Err(Error::security_violation(format!(
                "Constraint limit exceeded: {} > {}. This prevents DoS attacks through constraint explosion.",
                self.constraint_count,
                self.limits.max_constraints
            )));
        }
        Ok(())
    }

    /// Track specialization creation
    pub fn add_specialization(&mut self) -> Result<(), Error> {
        self.specialization_count += 1;

        if self.specialization_count > self.limits.max_specializations {
            return Err(Error::security_violation(format!(
                "Specialization limit exceeded: {} > {}. This prevents DoS attacks through specialization explosion.",
                self.specialization_count,
                self.limits.max_specializations
            )));
        }
        Ok(())
    }

    /// Check specialization limit (used in monomorphization)
    pub fn check_specialization_limit(&self, current_count: usize) -> Result<(), Error> {
        if current_count > self.limits.max_specializations {
            return Err(Error::security_violation(format!(
                "Specialization limit exceeded: {} > {}. This prevents DoS attacks through specialization explosion.",
                current_count,
                self.limits.max_specializations
            )));
        }
        Ok(())
    }

    /// Track work queue size
    pub fn check_work_queue_size(&mut self, queue_name: &str, size: usize) -> Result<(), Error> {
        self.work_queue_sizes.insert(queue_name.to_string(), size);

        if size > self.limits.max_work_queue_size {
            return Err(Error::security_violation(format!(
                "Work queue '{}' size limit exceeded: {} > {}. This prevents DoS attacks through unbounded queue growth.",
                queue_name,
                size,
                self.limits.max_work_queue_size
            )));
        }
        Ok(())
    }

    /// Get current resource usage statistics
    pub fn get_stats(&self) -> ResourceStats {
        ResourceStats {
            compilation_time: self.compilation_start.elapsed(),
            iteration_counts: self.iteration_counts.clone(),
            recursion_depths: self.recursion_depths.clone(),
            memory_usage: self.memory_usage,
            type_variable_count: self.type_variable_count,
            constraint_count: self.constraint_count,
            specialization_count: self.specialization_count,
            work_queue_sizes: self.work_queue_sizes.clone(),
        }
    }

    /// Reset all counters (useful for testing)
    pub fn reset(&mut self) {
        self.compilation_start = Instant::now();
        self.phase_starts.clear();
        self.iteration_counts.clear();
        self.recursion_depths.clear();
        self.memory_usage = 0;
        self.type_variable_count = 0;
        self.constraint_count = 0;
        self.specialization_count = 0;
        self.work_queue_sizes.clear();
    }
}

/// Statistics about resource usage during compilation
#[derive(Debug, Clone)]
pub struct ResourceStats {
    /// Total compilation time
    pub compilation_time: Duration,
    /// Iteration counts for different operations
    pub iteration_counts: HashMap<String, usize>,
    /// Current recursion depths
    pub recursion_depths: HashMap<String, usize>,
    /// Approximate memory usage
    pub memory_usage: usize,
    /// Number of type variables created
    pub type_variable_count: usize,
    /// Number of constraints created
    pub constraint_count: usize,
    /// Number of specializations created
    pub specialization_count: usize,
    /// Work queue sizes
    pub work_queue_sizes: HashMap<String, usize>,
}

impl ResourceStats {
    /// Check if any resource usage is concerning (above 80% of limits)
    pub fn has_concerning_usage(&self, limits: &ResourceLimits) -> Vec<String> {
        let mut concerns = Vec::new();

        // Check iteration counts
        for (op, count) in &self.iteration_counts {
            if *count > (limits.max_iterations * 80) / 100 {
                concerns.push(format!(
                    "Operation '{}' iteration count high: {}",
                    op, count
                ));
            }
        }

        // Check recursion depths
        for (op, depth) in &self.recursion_depths {
            if *depth > (limits.max_recursion_depth * 80) / 100 {
                concerns.push(format!(
                    "Operation '{}' recursion depth high: {}",
                    op, depth
                ));
            }
        }

        // Check memory usage
        if self.memory_usage > (limits.max_memory_bytes * 80) / 100 {
            concerns.push(format!("Memory usage high: {} bytes", self.memory_usage));
        }

        // Check type variables
        if self.type_variable_count > (limits.max_type_variables * 80) / 100 {
            concerns.push(format!(
                "Type variable count high: {}",
                self.type_variable_count
            ));
        }

        // Check constraints
        if self.constraint_count > (limits.max_constraints * 80) / 100 {
            concerns.push(format!("Constraint count high: {self.constraint_count}"));
        }

        concerns
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_limits_creation() {
        let limits = ResourceLimits::production();
        assert!(limits.validate().is_ok());
        assert_eq!(limits.max_iterations, MAX_SAFE_ITERATIONS);
        assert_eq!(
            limits.phase_timeout,
            Duration::from_secs(MAX_SAFE_TIMEOUT_SECS)
        );
    }

    #[test]
    fn test_resource_limits_validation() {
        let mut limits = ResourceLimits::production();

        // Test invalid max_iterations
        limits.max_iterations = 0;
        assert!(limits.validate().is_err());

        // Test invalid timeout
        limits.max_iterations = 1000;
        limits.phase_timeout = Duration::from_secs(0);
        assert!(limits.validate().is_err());

        // Test invalid total vs phase timeout
        limits.phase_timeout = Duration::from_secs(60);
        limits.total_timeout = Duration::from_secs(30);
        assert!(limits.validate().is_err());
    }

    #[test]
    fn test_resource_monitor_iteration_limit() {
        let limits = ResourceLimits::custom()
            .max_iterations(100)
            .build()
            .unwrap();
        let mut monitor = ResourceMonitor::new(limits);

        // Should succeed within limit
        assert!(monitor.check_iteration_limit("test", 50).is_ok());
        assert!(monitor.check_iteration_limit("test", 49).is_ok());

        // Should fail when exceeding limit
        assert!(monitor.check_iteration_limit("test", 2).is_err());
    }

    #[test]
    fn test_resource_monitor_timeout() {
        let limits = ResourceLimits::custom()
            .phase_timeout(Duration::from_millis(100))
            .build()
            .unwrap();
        let mut monitor = ResourceMonitor::new(limits);

        monitor.start_phase("test");

        // Should succeed immediately
        assert!(monitor.check_phase_timeout("test").is_ok());

        // Sleep to exceed timeout
        std::thread::sleep(Duration::from_millis(150));

        // Should fail after timeout
        assert!(monitor.check_phase_timeout("test").is_err());
    }

    #[test]
    fn test_resource_monitor_memory_tracking() {
        let limits = ResourceLimits::custom()
            .max_memory_bytes(1000)
            .build()
            .unwrap();
        let mut monitor = ResourceMonitor::new(limits);

        // Should succeed within limit
        assert!(monitor.add_memory_usage(500).is_ok());
        assert!(monitor.add_memory_usage(400).is_ok());

        // Should fail when exceeding limit
        assert!(monitor.add_memory_usage(200).is_err());
    }

    #[test]
    fn test_resource_monitor_type_variable_tracking() {
        let limits = ResourceLimits::custom()
            .max_iterations(2) // Using max_iterations as proxy for type variables
            .build()
            .unwrap();
        let mut monitor = ResourceMonitor::new(limits);

        // Should succeed within limit
        assert!(monitor.add_type_variable().is_ok());
        assert!(monitor.add_type_variable().is_ok());

        // Should fail when exceeding limit
        assert!(monitor.add_type_variable().is_err());
    }

    #[test]
    fn test_resource_stats_concerning_usage() {
        let limits = ResourceLimits::production();
        let mut stats = ResourceStats {
            compilation_time: Duration::from_secs(10),
            iteration_counts: HashMap::new(),
            recursion_depths: HashMap::new(),
            memory_usage: (limits.max_memory_bytes * 85) / 100, // 85% of limit
            type_variable_count: 100,
            constraint_count: 200,
            specialization_count: 50,
            work_queue_sizes: HashMap::new(),
        };

        stats
            .iteration_counts
            .insert("test".to_string(), (limits.max_iterations * 90) / 100);

        let concerns = stats.has_concerning_usage(&limits);
        assert!(concerns.len() >= 2); // Should have memory and iteration concerns
    }
}
