//! Resource limit enforcement for Script language compiler
//!
//! This module provides resource limit enforcement to prevent denial-of-service
//! attacks through excessive resource consumption during compilation.

use super::{ResourceType, SecurityError, SecurityMetrics};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Resource limit configuration
#[derive(Debug, Clone)]
pub struct ResourceLimitConfig {
    /// Maximum number of type variables per function
    pub max_type_vars: u32,
    /// Maximum number of constraints in type inference
    pub max_constraints: usize,
    /// Maximum number of function specializations
    pub max_specializations: usize,
    /// Maximum type solving iterations
    pub max_solving_iterations: usize,
    /// Maximum work queue size for monomorphization
    pub max_work_queue_size: usize,
    /// Maximum compilation time in seconds
    pub max_compilation_time_secs: u64,
    /// Maximum memory usage in bytes (0 = no limit)
    pub max_memory_usage_bytes: usize,
    /// Enable resource limit enforcement
    pub enable_limits: bool,
    /// Enable resource monitoring
    pub enable_monitoring: bool,
    /// Maximum number of async tasks
    pub max_async_tasks: usize,
    /// Maximum async task memory usage in bytes
    pub max_async_task_memory_bytes: usize,
    /// Maximum FFI pointer lifetime in seconds
    pub max_ffi_pointer_lifetime_secs: u64,
}

impl Default for ResourceLimitConfig {
    fn default() -> Self {
        ResourceLimitConfig {
            max_type_vars: 10_000,
            max_constraints: 50_000,
            max_specializations: 1_000,
            max_solving_iterations: 1_000,
            max_work_queue_size: 10_000,
            max_compilation_time_secs: 30,
            max_memory_usage_bytes: 2 * 1024 * 1024 * 1024, // 2GB
            enable_limits: true,
            enable_monitoring: true,
            max_async_tasks: 1_000,
            max_async_task_memory_bytes: 100 * 1024 * 1024, // 100MB
            max_ffi_pointer_lifetime_secs: 300,             // 5 minutes
        }
    }
}

/// Resource usage tracking
#[derive(Debug, Default)]
pub struct ResourceUsage {
    /// Current number of type variables
    pub type_vars_used: AtomicU32,
    /// Current number of constraints
    pub constraints_used: AtomicUsize,
    /// Current number of specializations
    pub specializations_used: AtomicUsize,
    /// Current solving iterations
    pub solving_iterations_used: AtomicUsize,
    /// Current work queue size
    pub work_queue_size: AtomicUsize,
    /// Peak memory usage observed
    pub peak_memory_usage: AtomicUsize,
    /// Compilation start time
    pub compilation_start: Option<Instant>,
    /// Current number of async tasks
    pub async_tasks_used: AtomicUsize,
    /// Current async task memory usage
    pub async_task_memory_used: AtomicUsize,
    /// Current number of active FFI pointers
    pub ffi_pointers_active: AtomicUsize,
}

impl ResourceUsage {
    /// Create new resource usage tracker
    pub fn new() -> Self {
        ResourceUsage::default()
    }

    /// Start compilation timing
    pub fn start_compilation(&mut self) {
        self.compilation_start = Some(Instant::now());
    }

    /// Get compilation duration
    pub fn compilation_duration(&self) -> Option<Duration> {
        self.compilation_start.map(|start| start.elapsed())
    }

    /// Increment type variable count
    pub fn increment_type_vars(&self, count: u32) -> u32 {
        self.type_vars_used.fetch_add(count, Ordering::Relaxed) + count
    }

    /// Increment constraint count
    pub fn increment_constraints(&self, count: usize) -> usize {
        self.constraints_used.fetch_add(count, Ordering::Relaxed) + count
    }

    /// Increment specialization count
    pub fn increment_specializations(&self, count: usize) -> usize {
        self.specializations_used
            .fetch_add(count, Ordering::Relaxed)
            + count
    }

    /// Increment solving iterations
    pub fn increment_solving_iterations(&self, count: usize) -> usize {
        self.solving_iterations_used
            .fetch_add(count, Ordering::Relaxed)
            + count
    }

    /// Set work queue size
    pub fn set_work_queue_size(&self, size: usize) {
        self.work_queue_size.store(size, Ordering::Relaxed);
    }

    /// Update peak memory usage
    pub fn update_memory_usage(&self, current_usage: usize) {
        let mut peak = self.peak_memory_usage.load(Ordering::Relaxed);
        while current_usage > peak {
            match self.peak_memory_usage.compare_exchange_weak(
                peak,
                current_usage,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new_peak) => peak = new_peak,
            }
        }
    }

    /// Get current resource usage snapshot
    pub fn snapshot(&self) -> ResourceSnapshot {
        ResourceSnapshot {
            type_vars_used: self.type_vars_used.load(Ordering::Relaxed),
            constraints_used: self.constraints_used.load(Ordering::Relaxed),
            specializations_used: self.specializations_used.load(Ordering::Relaxed),
            solving_iterations_used: self.solving_iterations_used.load(Ordering::Relaxed),
            work_queue_size: self.work_queue_size.load(Ordering::Relaxed),
            peak_memory_usage: self.peak_memory_usage.load(Ordering::Relaxed),
            compilation_duration: self.compilation_duration(),
            async_tasks_used: self.async_tasks_used.load(Ordering::Relaxed),
            async_task_memory_used: self.async_task_memory_used.load(Ordering::Relaxed),
            ffi_pointers_active: self.ffi_pointers_active.load(Ordering::Relaxed),
        }
    }

    /// Reset all counters
    pub fn reset(&mut self) {
        self.type_vars_used.store(0, Ordering::Relaxed);
        self.constraints_used.store(0, Ordering::Relaxed);
        self.specializations_used.store(0, Ordering::Relaxed);
        self.solving_iterations_used.store(0, Ordering::Relaxed);
        self.work_queue_size.store(0, Ordering::Relaxed);
        self.peak_memory_usage.store(0, Ordering::Relaxed);
        self.async_tasks_used.store(0, Ordering::Relaxed);
        self.async_task_memory_used.store(0, Ordering::Relaxed);
        self.ffi_pointers_active.store(0, Ordering::Relaxed);
        self.compilation_start = None;
    }

    /// Increment async task count
    pub fn increment_async_tasks(&self, count: usize) -> usize {
        self.async_tasks_used.fetch_add(count, Ordering::Relaxed) + count
    }

    /// Increment async task memory usage
    pub fn increment_async_task_memory(&self, bytes: usize) -> usize {
        self.async_task_memory_used
            .fetch_add(bytes, Ordering::Relaxed)
            + bytes
    }

    /// Increment FFI pointer count
    pub fn increment_ffi_pointers(&self, count: usize) -> usize {
        self.ffi_pointers_active.fetch_add(count, Ordering::Relaxed) + count
    }
}

/// Snapshot of resource usage at a point in time
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceSnapshot {
    pub type_vars_used: u32,
    pub constraints_used: usize,
    pub specializations_used: usize,
    pub solving_iterations_used: usize,
    pub work_queue_size: usize,
    pub peak_memory_usage: usize,
    pub compilation_duration: Option<Duration>,
    pub async_tasks_used: usize,
    pub async_task_memory_used: usize,
    pub ffi_pointers_active: usize,
}

impl ResourceSnapshot {
    /// Check if any resource limit is exceeded
    pub fn check_limits(&self, config: &ResourceLimitConfig) -> Vec<SecurityError> {
        let mut violations = Vec::new();

        if self.type_vars_used > config.max_type_vars {
            violations.push(SecurityError::ResourceLimitExceeded {
                resource_type: ResourceType::TypeVariables,
                current_count: self.type_vars_used as usize,
                limit: config.max_type_vars as usize,
                message: "Too many type variables generated".to_string(),
            });
        }

        if self.constraints_used > config.max_constraints {
            violations.push(SecurityError::ResourceLimitExceeded {
                resource_type: ResourceType::Constraints,
                current_count: self.constraints_used,
                limit: config.max_constraints,
                message: "Too many type constraints generated".to_string(),
            });
        }

        if self.specializations_used > config.max_specializations {
            violations.push(SecurityError::ResourceLimitExceeded {
                resource_type: ResourceType::Specializations,
                current_count: self.specializations_used,
                limit: config.max_specializations,
                message: "Too many function specializations generated".to_string(),
            });
        }

        if self.solving_iterations_used > config.max_solving_iterations {
            violations.push(SecurityError::ResourceLimitExceeded {
                resource_type: ResourceType::SolvingIterations,
                current_count: self.solving_iterations_used,
                limit: config.max_solving_iterations,
                message: "Too many type solving iterations".to_string(),
            });
        }

        if self.work_queue_size > config.max_work_queue_size {
            violations.push(SecurityError::ResourceLimitExceeded {
                resource_type: ResourceType::WorkQueueSize,
                current_count: self.work_queue_size,
                limit: config.max_work_queue_size,
                message: "Work queue size exceeded".to_string(),
            });
        }

        if let Some(duration) = self.compilation_duration {
            let limit_duration = Duration::from_secs(config.max_compilation_time_secs);
            if duration > limit_duration {
                violations.push(SecurityError::CompilationTimeout {
                    duration,
                    limit: limit_duration,
                    message: "Compilation time exceeded limit".to_string(),
                });
            }
        }

        violations
    }

    /// Calculate resource utilization percentage (0-100)
    pub fn utilization_percentage(&self, config: &ResourceLimitConfig) -> ResourceUtilization {
        ResourceUtilization {
            type_vars: (self.type_vars_used as f64 / config.max_type_vars as f64 * 100.0).min(100.0)
                as u8,
            constraints: (self.constraints_used as f64 / config.max_constraints as f64 * 100.0)
                .min(100.0) as u8,
            specializations: (self.specializations_used as f64 / config.max_specializations as f64
                * 100.0)
                .min(100.0) as u8,
            solving_iterations: (self.solving_iterations_used as f64
                / config.max_solving_iterations as f64
                * 100.0)
                .min(100.0) as u8,
            work_queue: (self.work_queue_size as f64 / config.max_work_queue_size as f64 * 100.0)
                .min(100.0) as u8,
            memory: if config.max_memory_usage_bytes > 0 {
                (self.peak_memory_usage as f64 / config.max_memory_usage_bytes as f64 * 100.0)
                    .min(100.0) as u8
            } else {
                0
            },
            compilation_time: if let Some(duration) = self.compilation_duration {
                let limit_secs = config.max_compilation_time_secs as f64;
                (duration.as_secs_f64() / limit_secs * 100.0).min(100.0) as u8
            } else {
                0
            },
        }
    }
}

/// Resource utilization percentages
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceUtilization {
    pub type_vars: u8,
    pub constraints: u8,
    pub specializations: u8,
    pub solving_iterations: u8,
    pub work_queue: u8,
    pub memory: u8,
    pub compilation_time: u8,
}

impl ResourceUtilization {
    /// Get maximum utilization across all resources
    pub fn max_utilization(&self) -> u8 {
        *[
            self.type_vars,
            self.constraints,
            self.specializations,
            self.solving_iterations,
            self.work_queue,
            self.memory,
            self.compilation_time,
        ]
        .iter()
        .max()
        .unwrap_or(&0)
    }

    /// Get average utilization
    pub fn average_utilization(&self) -> u8 {
        let total = self.type_vars as u16
            + self.constraints as u16
            + self.specializations as u16
            + self.solving_iterations as u16
            + self.work_queue as u16
            + self.memory as u16
            + self.compilation_time as u16;
        (total / 7) as u8
    }

    /// Check if any resource is critically utilized (>90%)
    pub fn has_critical_utilization(&self) -> bool {
        self.max_utilization() > 90
    }

    /// Check if any resource is highly utilized (>75%)
    pub fn has_high_utilization(&self) -> bool {
        self.max_utilization() > 75
    }
}

/// Resource limit enforcer
pub struct ResourceLimitEnforcer {
    config: ResourceLimitConfig,
    usage: ResourceUsage,
    metrics: Option<SecurityMetrics>,
}

impl ResourceLimitEnforcer {
    /// Create new resource limit enforcer
    pub fn new() -> Self {
        ResourceLimitEnforcer {
            config: ResourceLimitConfig::default(),
            usage: ResourceUsage::new(),
            metrics: None,
        }
    }

    /// Create enforcer with custom configuration
    pub fn with_config(config: ResourceLimitConfig) -> Self {
        ResourceLimitEnforcer {
            config,
            usage: ResourceUsage::new(),
            metrics: None,
        }
    }

    /// Set security metrics for recording events
    pub fn with_metrics(mut self, metrics: SecurityMetrics) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Start resource monitoring for compilation
    pub fn start_monitoring(&mut self) {
        self.usage.start_compilation();
    }

    /// Check if resource limit would be exceeded by increment
    pub fn check_resource_increment(
        &self,
        resource_type: ResourceType,
        increment: usize,
    ) -> Result<(), SecurityError> {
        if !self.config.enable_limits {
            return Ok(());
        }

        let current = match resource_type {
            ResourceType::TypeVariables => {
                self.usage.type_vars_used.load(Ordering::Relaxed) as usize
            }
            ResourceType::Constraints => self.usage.constraints_used.load(Ordering::Relaxed),
            ResourceType::Specializations => {
                self.usage.specializations_used.load(Ordering::Relaxed)
            }
            ResourceType::SolvingIterations => {
                self.usage.solving_iterations_used.load(Ordering::Relaxed)
            }
            ResourceType::WorkQueueSize => self.usage.work_queue_size.load(Ordering::Relaxed),
            ResourceType::AsyncTasks => self.usage.async_tasks_used.load(Ordering::Relaxed),
            ResourceType::AsyncTaskMemory => {
                self.usage.async_task_memory_used.load(Ordering::Relaxed)
            }
            ResourceType::FfiPointerLifetime => {
                self.usage.ffi_pointers_active.load(Ordering::Relaxed)
            }
        };

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

        if current + increment > limit {
            if let Some(ref metrics) = self.metrics {
                metrics.record_resource_limit_violation();
            }

            Err(SecurityError::ResourceLimitExceeded {
                resource_type,
                current_count: current + increment,
                limit,
                message: format!(
                    "Resource limit would be exceeded by increment of {}",
                    increment
                ),
            })
        } else {
            Ok(())
        }
    }

    /// Record resource usage increment
    pub fn record_resource_usage(
        &self,
        resource_type: ResourceType,
        increment: usize,
    ) -> Result<usize, SecurityError> {
        // Check limit before incrementing
        self.check_resource_increment(resource_type, increment)?;

        // Perform the increment
        let new_count = match resource_type {
            ResourceType::TypeVariables => {
                self.usage.increment_type_vars(increment as u32) as usize
            }
            ResourceType::Constraints => self.usage.increment_constraints(increment),
            ResourceType::Specializations => self.usage.increment_specializations(increment),
            ResourceType::SolvingIterations => self.usage.increment_solving_iterations(increment),
            ResourceType::WorkQueueSize => {
                self.usage.set_work_queue_size(increment);
                increment
            }
            ResourceType::AsyncTasks => {
                self.usage.increment_async_tasks(increment);
                increment
            }
            ResourceType::AsyncTaskMemory => {
                self.usage.increment_async_task_memory(increment);
                increment
            }
            ResourceType::FfiPointerLifetime => {
                self.usage.increment_ffi_pointers(increment);
                increment
            }
        };

        Ok(new_count)
    }

    /// Check compilation timeout
    pub fn check_compilation_timeout(&self) -> Result<(), SecurityError> {
        if !self.config.enable_limits {
            return Ok(());
        }

        if let Some(duration) = self.usage.compilation_duration() {
            let limit = Duration::from_secs(self.config.max_compilation_time_secs);
            if duration > limit {
                if let Some(ref metrics) = self.metrics {
                    metrics.record_compilation_timeout();
                }

                return Err(SecurityError::CompilationTimeout {
                    duration,
                    limit,
                    message: "Compilation timeout exceeded".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Get current resource usage snapshot
    pub fn get_usage_snapshot(&self) -> ResourceSnapshot {
        self.usage.snapshot()
    }

    /// Get resource utilization
    pub fn get_utilization(&self) -> ResourceUtilization {
        self.usage.snapshot().utilization_percentage(&self.config)
    }

    /// Check all resource limits
    pub fn check_all_limits(&self) -> Result<(), Vec<SecurityError>> {
        let snapshot = self.get_usage_snapshot();
        let violations = snapshot.check_limits(&self.config);

        if violations.is_empty() {
            Ok(())
        } else {
            // Record violations in metrics
            if let Some(ref metrics) = self.metrics {
                for _ in &violations {
                    metrics.record_resource_limit_violation();
                }
            }
            Err(violations)
        }
    }

    /// Reset resource tracking
    pub fn reset(&mut self) {
        self.usage.reset();
    }

    /// Get configuration
    pub fn config(&self) -> &ResourceLimitConfig {
        &self.config
    }

    /// Get usage tracker (read-only)
    pub fn usage(&self) -> &ResourceUsage {
        &self.usage
    }

    /// Print resource usage report
    pub fn print_usage_report(&self) {
        let snapshot = self.get_usage_snapshot();
        let utilization = self.get_utilization();

        println!("\n📊 RESOURCE USAGE REPORT");
        println!("═══════════════════════════");

        println!(
            "Type Variables: {} / {} ({}%)",
            snapshot.type_vars_used, self.config.max_type_vars, utilization.type_vars
        );
        println!(
            "Constraints: {} / {} ({}%)",
            snapshot.constraints_used, self.config.max_constraints, utilization.constraints
        );
        println!(
            "Specializations: {} / {} ({}%)",
            snapshot.specializations_used,
            self.config.max_specializations,
            utilization.specializations
        );
        println!(
            "Solving Iterations: {} / {} ({}%)",
            snapshot.solving_iterations_used,
            self.config.max_solving_iterations,
            utilization.solving_iterations
        );
        println!(
            "Work Queue Size: {} / {} ({}%)",
            snapshot.work_queue_size, self.config.max_work_queue_size, utilization.work_queue
        );

        if let Some(duration) = snapshot.compilation_duration {
            println!(
                "Compilation Time: {:?} / {}s ({}%)",
                duration, self.config.max_compilation_time_secs, utilization.compilation_time
            );
        }

        if self.config.max_memory_usage_bytes > 0 {
            println!(
                "Peak Memory: {} / {} bytes ({}%)",
                snapshot.peak_memory_usage, self.config.max_memory_usage_bytes, utilization.memory
            );
        }

        println!("\nUtilization Summary:");
        println!("  Max: {}%", utilization.max_utilization());
        println!("  Average: {}%", utilization.average_utilization());

        if utilization.has_critical_utilization() {
            println!("  Status: ⚠️ CRITICAL UTILIZATION");
        } else if utilization.has_high_utilization() {
            println!("  Status: ⚠️ HIGH UTILIZATION");
        } else {
            println!("  Status: ✅ NORMAL");
        }
    }
}

impl Default for ResourceLimitEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_usage_tracking() {
        let usage = ResourceUsage::new();

        let count = usage.increment_type_vars(10);
        assert_eq!(count, 10);

        let count = usage.increment_constraints(5);
        assert_eq!(count, 5);

        let snapshot = usage.snapshot();
        assert_eq!(snapshot.type_vars_used, 10);
        assert_eq!(snapshot.constraints_used, 5);
    }

    #[test]
    fn test_resource_limit_checking() {
        let config = ResourceLimitConfig {
            max_type_vars: 100,
            max_constraints: 100,
            ..Default::default()
        };

        let mut enforcer = ResourceLimitEnforcer::with_config(config);

        // Should succeed within limits
        let result = enforcer.record_resource_usage(ResourceType::TypeVariables, 50);
        assert!(result.is_ok());

        // Should fail when exceeding limits
        let result = enforcer.record_resource_usage(ResourceType::TypeVariables, 60);
        assert!(result.is_err());
    }

    #[test]
    fn test_utilization_calculation() {
        let config = ResourceLimitConfig {
            max_type_vars: 100,
            max_constraints: 200,
            ..Default::default()
        };

        let snapshot = ResourceSnapshot {
            type_vars_used: 50,
            constraints_used: 100,
            specializations_used: 0,
            solving_iterations_used: 0,
            work_queue_size: 0,
            peak_memory_usage: 0,
            compilation_duration: None,
        };

        let utilization = snapshot.utilization_percentage(&config);
        assert_eq!(utilization.type_vars, 50);
        assert_eq!(utilization.constraints, 50);
        assert_eq!(utilization.max_utilization(), 50);
    }

    #[test]
    fn test_resource_snapshot_limit_checking() {
        let config = ResourceLimitConfig {
            max_type_vars: 10,
            max_constraints: 10,
            ..Default::default()
        };

        let snapshot = ResourceSnapshot {
            type_vars_used: 15,
            constraints_used: 5,
            specializations_used: 0,
            solving_iterations_used: 0,
            work_queue_size: 0,
            peak_memory_usage: 0,
            compilation_duration: None,
        };

        let violations = snapshot.check_limits(&config);
        assert_eq!(violations.len(), 1);

        if let SecurityError::ResourceLimitExceeded { resource_type, .. } = &violations[0] {
            assert_eq!(*resource_type, ResourceType::TypeVariables);
        } else {
            panic!("Expected ResourceLimitExceeded error");
        }
    }

    #[test]
    fn test_compilation_timeout() {
        let config = ResourceLimitConfig {
            max_compilation_time_secs: 1,
            ..Default::default()
        };

        let mut enforcer = ResourceLimitEnforcer::with_config(config);
        enforcer.start_monitoring();

        // Simulate passage of time
        std::thread::sleep(Duration::from_millis(1100));

        let result = enforcer.check_compilation_timeout();
        assert!(result.is_err());
    }

    #[test]
    fn test_memory_usage_tracking() {
        let usage = ResourceUsage::new();

        usage.update_memory_usage(1000);
        usage.update_memory_usage(500); // Should not update peak
        usage.update_memory_usage(1500); // Should update peak

        let snapshot = usage.snapshot();
        assert_eq!(snapshot.peak_memory_usage, 1500);
    }

    #[test]
    fn test_resource_utilization_analysis() {
        let utilization = ResourceUtilization {
            type_vars: 95,
            constraints: 50,
            specializations: 30,
            solving_iterations: 20,
            work_queue: 10,
            memory: 60,
            compilation_time: 40,
        };

        assert_eq!(utilization.max_utilization(), 95);
        assert_eq!(utilization.average_utilization(), 44);
        assert!(utilization.has_critical_utilization());
        assert!(utilization.has_high_utilization());
    }
}
