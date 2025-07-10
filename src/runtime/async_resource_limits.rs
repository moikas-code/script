//! Async-specific resource limits and monitoring
//!
//! This module extends the base resource limit system with async-specific controls
//! including task rate limiting, async memory tracking, and DoS protection.

use super::resource_limits::{ResourceLimits, ResourceMonitor, ResourceViolation};
use crate::security::{SecurityError, SecurityMetrics};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

/// Async-specific resource limits
#[derive(Debug, Clone)]
pub struct AsyncResourceLimits {
    /// Base resource limits
    pub base_limits: ResourceLimits,
    /// Maximum number of concurrent async tasks
    pub max_concurrent_tasks: usize,
    /// Maximum async task memory per task in bytes
    pub max_task_memory_bytes: usize,
    /// Maximum total async memory across all tasks
    pub max_total_async_memory_bytes: usize,
    /// Maximum task execution time
    pub max_task_execution_time: Duration,
    /// Maximum task spawn rate (tasks per second)
    pub max_task_spawn_rate: f64,
    /// Maximum FFI calls per second
    pub max_ffi_call_rate: f64,
    /// Maximum pointer registrations per second
    pub max_pointer_registration_rate: f64,
    /// Task cleanup interval
    pub task_cleanup_interval: Duration,
    /// Enable async-specific throttling
    pub enable_async_throttling: bool,
    /// Rate limiting window size
    pub rate_limit_window: Duration,
}

impl Default for AsyncResourceLimits {
    fn default() -> Self {
        Self {
            base_limits: ResourceLimits::default(),
            max_concurrent_tasks: 10_000,
            max_task_memory_bytes: 10 * 1024 * 1024, // 10MB per task
            max_total_async_memory_bytes: 100 * 1024 * 1024, // 100MB total
            max_task_execution_time: Duration::from_secs(300), // 5 minutes
            max_task_spawn_rate: 1000.0,             // 1000 tasks/second
            max_ffi_call_rate: 10_000.0,             // 10,000 FFI calls/second
            max_pointer_registration_rate: 50_000.0, // 50,000 pointers/second
            task_cleanup_interval: Duration::from_secs(60), // 1 minute
            enable_async_throttling: true,
            rate_limit_window: Duration::from_secs(1),
        }
    }
}

/// Async resource usage tracking
#[derive(Debug, Default)]
pub struct AsyncResourceUsage {
    /// Current number of active tasks
    active_tasks: AtomicUsize,
    /// Peak concurrent tasks
    peak_concurrent_tasks: AtomicUsize,
    /// Total tasks spawned
    total_tasks_spawned: AtomicU64,
    /// Total tasks completed
    total_tasks_completed: AtomicU64,
    /// Total tasks failed
    total_tasks_failed: AtomicU64,
    /// Current async memory usage
    current_async_memory: AtomicUsize,
    /// Peak async memory usage
    peak_async_memory: AtomicUsize,
    /// Total FFI calls made
    total_ffi_calls: AtomicU64,
    /// Total pointer registrations
    total_pointer_registrations: AtomicU64,
    /// Rate limiting counters
    rate_counters: RwLock<RateCounters>,
    /// Task execution times for monitoring
    execution_times: Mutex<VecDeque<Duration>>,
}

/// Rate limiting counters
#[derive(Debug)]
struct RateCounters {
    /// Task spawn events with timestamps
    task_spawns: VecDeque<Instant>,
    /// FFI call events with timestamps
    ffi_calls: VecDeque<Instant>,
    /// Pointer registration events with timestamps
    pointer_registrations: VecDeque<Instant>,
    /// Last cleanup time
    last_cleanup: Instant,
}

impl Default for RateCounters {
    fn default() -> Self {
        Self {
            task_spawns: VecDeque::new(),
            ffi_calls: VecDeque::new(),
            pointer_registrations: VecDeque::new(),
            last_cleanup: Instant::now(),
        }
    }
}

impl RateCounters {
    /// Clean up old events outside the rate limiting window
    fn cleanup(&mut self, window: Duration) {
        let cutoff = Instant::now() - window;

        self.task_spawns.retain(|&timestamp| timestamp > cutoff);
        self.ffi_calls.retain(|&timestamp| timestamp > cutoff);
        self.pointer_registrations
            .retain(|&timestamp| timestamp > cutoff);

        self.last_cleanup = Instant::now();
    }

    /// Check if cleanup is needed
    fn needs_cleanup(&self) -> bool {
        self.last_cleanup.elapsed() > Duration::from_secs(10) // Cleanup every 10 seconds
    }
}

/// Async-specific resource violations
#[derive(Debug, Clone, PartialEq)]
pub enum AsyncResourceViolation {
    /// Base resource violation
    Base(ResourceViolation),
    /// Too many concurrent tasks
    TooManyConcurrentTasks { current: usize, limit: usize },
    /// Task memory limit exceeded
    TaskMemoryExceeded { task_memory: usize, limit: usize },
    /// Total async memory limit exceeded
    TotalAsyncMemoryExceeded { total_memory: usize, limit: usize },
    /// Task execution time exceeded
    TaskExecutionTimeExceeded {
        execution_time: Duration,
        limit: Duration,
    },
    /// Task spawn rate exceeded
    TaskSpawnRateExceeded { current_rate: f64, limit: f64 },
    /// FFI call rate exceeded
    FfiCallRateExceeded { current_rate: f64, limit: f64 },
    /// Pointer registration rate exceeded
    PointerRegistrationRateExceeded { current_rate: f64, limit: f64 },
    /// System overloaded
    SystemOverloaded { reason: String },
}

impl From<ResourceViolation> for AsyncResourceViolation {
    fn from(violation: ResourceViolation) -> Self {
        AsyncResourceViolation::Base(violation)
    }
}

impl From<AsyncResourceViolation> for SecurityError {
    fn from(violation: AsyncResourceViolation) -> Self {
        use crate::security::ResourceType;

        match violation {
            AsyncResourceViolation::Base(base) => match base {
                ResourceViolation::MemoryLimitExceeded { current, limit } => {
                    SecurityError::ResourceLimitExceeded {
                        resource_type: ResourceType::AsyncTaskMemory,
                        current_count: current,
                        limit,
                        message: "Memory limit exceeded".to_string(),
                    }
                }
                _ => SecurityError::AsyncTaskLimitExceeded {
                    current_tasks: 0,
                    task_limit: 0,
                    message: format!("Resource violation: {:?}", base),
                },
            },
            AsyncResourceViolation::TooManyConcurrentTasks { current, limit } => {
                SecurityError::AsyncTaskLimitExceeded {
                    current_tasks: current,
                    task_limit: limit,
                    message: "Too many concurrent async tasks".to_string(),
                }
            }
            AsyncResourceViolation::TaskMemoryExceeded { task_memory, limit } => {
                SecurityError::AsyncMemoryViolation {
                    task_id: 0, // Will be filled by caller
                    memory_used: task_memory,
                    memory_limit: limit,
                    message: "Task memory limit exceeded".to_string(),
                }
            }
            AsyncResourceViolation::TotalAsyncMemoryExceeded {
                total_memory,
                limit,
            } => SecurityError::ResourceLimitExceeded {
                resource_type: ResourceType::AsyncTaskMemory,
                current_count: total_memory,
                limit,
                message: "Total async memory limit exceeded".to_string(),
            },
            AsyncResourceViolation::TaskExecutionTimeExceeded {
                execution_time,
                limit,
            } => SecurityError::AsyncFFIViolation {
                function_name: "task_execution".to_string(),
                violation_type: "execution timeout".to_string(),
                message: format!(
                    "Task execution time {:?} exceeded limit {:?}",
                    execution_time, limit
                ),
            },
            AsyncResourceViolation::TaskSpawnRateExceeded {
                current_rate,
                limit,
            } => SecurityError::AsyncFFIViolation {
                function_name: "spawn_task".to_string(),
                violation_type: "rate limit exceeded".to_string(),
                message: format!(
                    "Task spawn rate {:.1}/s exceeded limit {:.1}/s",
                    current_rate, limit
                ),
            },
            AsyncResourceViolation::FfiCallRateExceeded {
                current_rate,
                limit,
            } => SecurityError::AsyncFFIViolation {
                function_name: "ffi_call".to_string(),
                violation_type: "rate limit exceeded".to_string(),
                message: format!(
                    "FFI call rate {:.1}/s exceeded limit {:.1}/s",
                    current_rate, limit
                ),
            },
            AsyncResourceViolation::PointerRegistrationRateExceeded {
                current_rate,
                limit,
            } => SecurityError::AsyncPointerViolation {
                pointer_address: 0,
                validation_failed: "rate limit exceeded".to_string(),
                message: format!(
                    "Pointer registration rate {:.1}/s exceeded limit {:.1}/s",
                    current_rate, limit
                ),
            },
            AsyncResourceViolation::SystemOverloaded { reason } => {
                SecurityError::AsyncTaskLimitExceeded {
                    current_tasks: 0,
                    task_limit: 0,
                    message: format!("System overloaded: {}", reason),
                }
            }
        }
    }
}

/// Comprehensive async resource monitor
pub struct AsyncResourceMonitor {
    /// Async-specific limits
    limits: AsyncResourceLimits,
    /// Base resource monitor
    base_monitor: ResourceMonitor,
    /// Async usage tracking
    usage: AsyncResourceUsage,
    /// Per-task memory tracking
    task_memory: RwLock<HashMap<usize, usize>>,
    /// Throttling state
    throttling_level: AtomicU64, // f64 as u64 bits
    /// Security metrics
    metrics: Option<Arc<SecurityMetrics>>,
    /// Last cleanup time
    last_cleanup: Mutex<Instant>,
}

impl AsyncResourceMonitor {
    /// Create a new async resource monitor
    pub fn new(limits: AsyncResourceLimits) -> Self {
        let base_monitor = ResourceMonitor::new(limits.base_limits.clone());

        Self {
            limits,
            base_monitor,
            usage: AsyncResourceUsage::default(),
            task_memory: RwLock::new(HashMap::new()),
            throttling_level: AtomicU64::new(0), // 0.0 as bits
            metrics: None,
            last_cleanup: Mutex::new(Instant::now()),
        }
    }

    /// Set security metrics for monitoring
    pub fn with_metrics(mut self, metrics: Arc<SecurityMetrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Record task spawn with rate limiting
    pub fn record_task_spawn(&self) -> Result<(), AsyncResourceViolation> {
        // Check concurrent task limit
        let current_tasks = self.usage.active_tasks.fetch_add(1, Ordering::Relaxed) + 1;
        if current_tasks > self.limits.max_concurrent_tasks {
            self.usage.active_tasks.fetch_sub(1, Ordering::Relaxed);
            return Err(AsyncResourceViolation::TooManyConcurrentTasks {
                current: current_tasks,
                limit: self.limits.max_concurrent_tasks,
            });
        }

        // Update peaks
        self.usage
            .peak_concurrent_tasks
            .fetch_max(current_tasks, Ordering::Relaxed);

        // Check spawn rate limit
        {
            let mut counters = self.usage.rate_counters.write().unwrap();
            if counters.needs_cleanup() {
                counters.cleanup(self.limits.rate_limit_window);
            }

            let now = Instant::now();
            counters.task_spawns.push_back(now);

            let spawn_rate =
                counters.task_spawns.len() as f64 / self.limits.rate_limit_window.as_secs_f64();
            if spawn_rate > self.limits.max_task_spawn_rate {
                return Err(AsyncResourceViolation::TaskSpawnRateExceeded {
                    current_rate: spawn_rate,
                    limit: self.limits.max_task_spawn_rate,
                });
            }
        }

        // Update metrics
        self.usage
            .total_tasks_spawned
            .fetch_add(1, Ordering::Relaxed);

        // Record with security metrics
        if let Some(ref metrics) = self.metrics {
            metrics.record_async_task_limit_violation();
        }

        Ok(())
    }

    /// Record task completion
    pub fn record_task_completion(
        &self,
        task_id: usize,
        execution_time: Duration,
    ) -> Result<(), AsyncResourceViolation> {
        // Validate execution time
        if execution_time > self.limits.max_task_execution_time {
            return Err(AsyncResourceViolation::TaskExecutionTimeExceeded {
                execution_time,
                limit: self.limits.max_task_execution_time,
            });
        }

        // Update counters
        self.usage.active_tasks.fetch_sub(1, Ordering::Relaxed);
        self.usage
            .total_tasks_completed
            .fetch_add(1, Ordering::Relaxed);

        // Clean up task memory tracking
        if let Ok(mut task_memory) = self.task_memory.write() {
            if let Some(memory) = task_memory.remove(&task_id) {
                self.usage
                    .current_async_memory
                    .fetch_sub(memory, Ordering::Relaxed);
            }
        }

        // Track execution times for monitoring
        {
            let mut times = self.usage.execution_times.lock().unwrap();
            times.push_back(execution_time);
            // Keep only recent execution times (last 1000)
            while times.len() > 1000 {
                times.pop_front();
            }
        }

        Ok(())
    }

    /// Record task failure
    pub fn record_task_failure(&self, task_id: usize) {
        self.usage.active_tasks.fetch_sub(1, Ordering::Relaxed);
        self.usage
            .total_tasks_failed
            .fetch_add(1, Ordering::Relaxed);

        // Clean up task memory tracking
        if let Ok(mut task_memory) = self.task_memory.write() {
            if let Some(memory) = task_memory.remove(&task_id) {
                self.usage
                    .current_async_memory
                    .fetch_sub(memory, Ordering::Relaxed);
            }
        }
    }

    /// Record task memory allocation
    pub fn record_task_memory_allocation(
        &self,
        task_id: usize,
        size: usize,
    ) -> Result<(), AsyncResourceViolation> {
        // Check per-task memory limit
        if size > self.limits.max_task_memory_bytes {
            return Err(AsyncResourceViolation::TaskMemoryExceeded {
                task_memory: size,
                limit: self.limits.max_task_memory_bytes,
            });
        }

        // Update total async memory
        let new_total = self
            .usage
            .current_async_memory
            .fetch_add(size, Ordering::Relaxed)
            + size;
        if new_total > self.limits.max_total_async_memory_bytes {
            self.usage
                .current_async_memory
                .fetch_sub(size, Ordering::Relaxed);
            return Err(AsyncResourceViolation::TotalAsyncMemoryExceeded {
                total_memory: new_total,
                limit: self.limits.max_total_async_memory_bytes,
            });
        }

        // Update peaks
        self.usage
            .peak_async_memory
            .fetch_max(new_total, Ordering::Relaxed);

        // Track per-task memory
        if let Ok(mut task_memory) = self.task_memory.write() {
            let current_task_memory = task_memory.entry(task_id).or_insert(0);
            *current_task_memory += size;

            if *current_task_memory > self.limits.max_task_memory_bytes {
                return Err(AsyncResourceViolation::TaskMemoryExceeded {
                    task_memory: *current_task_memory,
                    limit: self.limits.max_task_memory_bytes,
                });
            }
        }

        // Record with base monitor
        self.base_monitor.record_allocation(size)?;

        Ok(())
    }

    /// Record FFI call with rate limiting
    pub fn record_ffi_call(&self) -> Result<(), AsyncResourceViolation> {
        {
            let mut counters = self.usage.rate_counters.write().unwrap();
            if counters.needs_cleanup() {
                counters.cleanup(self.limits.rate_limit_window);
            }

            let now = Instant::now();
            counters.ffi_calls.push_back(now);

            let call_rate =
                counters.ffi_calls.len() as f64 / self.limits.rate_limit_window.as_secs_f64();
            if call_rate > self.limits.max_ffi_call_rate {
                return Err(AsyncResourceViolation::FfiCallRateExceeded {
                    current_rate: call_rate,
                    limit: self.limits.max_ffi_call_rate,
                });
            }
        }

        self.usage.total_ffi_calls.fetch_add(1, Ordering::Relaxed);

        // Record with security metrics
        if let Some(ref metrics) = self.metrics {
            metrics.record_async_ffi_validation(false);
        }

        Ok(())
    }

    /// Record pointer registration with rate limiting
    pub fn record_pointer_registration(&self) -> Result<(), AsyncResourceViolation> {
        {
            let mut counters = self.usage.rate_counters.write().unwrap();
            if counters.needs_cleanup() {
                counters.cleanup(self.limits.rate_limit_window);
            }

            let now = Instant::now();
            counters.pointer_registrations.push_back(now);

            let registration_rate = counters.pointer_registrations.len() as f64
                / self.limits.rate_limit_window.as_secs_f64();
            if registration_rate > self.limits.max_pointer_registration_rate {
                return Err(AsyncResourceViolation::PointerRegistrationRateExceeded {
                    current_rate: registration_rate,
                    limit: self.limits.max_pointer_registration_rate,
                });
            }
        }

        self.usage
            .total_pointer_registrations
            .fetch_add(1, Ordering::Relaxed);

        // Record with security metrics
        if let Some(ref metrics) = self.metrics {
            metrics.record_async_pointer_validation(false);
        }

        Ok(())
    }

    /// Check if system is overloaded and should throttle
    pub fn check_system_health(&self) -> Result<(), AsyncResourceViolation> {
        // Check base system health
        let base_stats = self.base_monitor.get_stats();
        if base_stats.is_under_pressure {
            return Err(AsyncResourceViolation::SystemOverloaded {
                reason: "Base system under memory pressure".to_string(),
            });
        }

        // Check async-specific overload conditions
        let async_memory_usage = self.usage.current_async_memory.load(Ordering::Relaxed) as f64
            / self.limits.max_total_async_memory_bytes as f64;

        if async_memory_usage > 0.9 {
            return Err(AsyncResourceViolation::SystemOverloaded {
                reason: format!("Async memory usage at {:.1}%", async_memory_usage * 100.0),
            });
        }

        let task_usage = self.usage.active_tasks.load(Ordering::Relaxed) as f64
            / self.limits.max_concurrent_tasks as f64;

        if task_usage > 0.9 {
            return Err(AsyncResourceViolation::SystemOverloaded {
                reason: format!("Task usage at {:.1}%", task_usage * 100.0),
            });
        }

        Ok(())
    }

    /// Periodic cleanup of resources
    pub fn cleanup_resources(&self) {
        let mut last_cleanup = self.last_cleanup.lock().unwrap();
        let now = Instant::now();

        if now.duration_since(*last_cleanup) < self.limits.task_cleanup_interval {
            return;
        }

        // Clean up rate counters
        {
            let mut counters = self.usage.rate_counters.write().unwrap();
            counters.cleanup(self.limits.rate_limit_window);
        }

        // Clean up execution time history
        {
            let mut times = self.usage.execution_times.lock().unwrap();
            let cutoff = now - Duration::from_secs(300); // Keep 5 minutes of history
            times.retain(|_| true); // Duration doesn't have timestamps, keep all for now
        }

        *last_cleanup = now;
    }

    /// Get current throttling level (0.0 = no throttling, 1.0 = maximum throttling)
    pub fn get_throttling_level(&self) -> f64 {
        f64::from_bits(self.throttling_level.load(Ordering::Relaxed))
    }

    /// Set throttling level
    pub fn set_throttling_level(&self, level: f64) {
        let clamped_level = level.clamp(0.0, 1.0);
        self.throttling_level
            .store(clamped_level.to_bits(), Ordering::Relaxed);
    }

    /// Get async resource statistics
    pub fn get_async_stats(&self) -> AsyncResourceStats {
        let base_stats = self.base_monitor.get_stats();

        AsyncResourceStats {
            base_stats,
            active_tasks: self.usage.active_tasks.load(Ordering::Relaxed),
            peak_concurrent_tasks: self.usage.peak_concurrent_tasks.load(Ordering::Relaxed),
            total_tasks_spawned: self.usage.total_tasks_spawned.load(Ordering::Relaxed),
            total_tasks_completed: self.usage.total_tasks_completed.load(Ordering::Relaxed),
            total_tasks_failed: self.usage.total_tasks_failed.load(Ordering::Relaxed),
            current_async_memory: self.usage.current_async_memory.load(Ordering::Relaxed),
            peak_async_memory: self.usage.peak_async_memory.load(Ordering::Relaxed),
            total_ffi_calls: self.usage.total_ffi_calls.load(Ordering::Relaxed),
            total_pointer_registrations: self
                .usage
                .total_pointer_registrations
                .load(Ordering::Relaxed),
            async_memory_usage_percent: self.usage.current_async_memory.load(Ordering::Relaxed)
                as f64
                / self.limits.max_total_async_memory_bytes as f64,
            task_usage_percent: self.usage.active_tasks.load(Ordering::Relaxed) as f64
                / self.limits.max_concurrent_tasks as f64,
            throttling_level: self.get_throttling_level(),
        }
    }
}

/// Comprehensive async resource statistics
#[derive(Debug, Clone)]
pub struct AsyncResourceStats {
    /// Base resource statistics
    pub base_stats: super::resource_limits::ResourceStats,
    /// Current number of active tasks
    pub active_tasks: usize,
    /// Peak concurrent tasks
    pub peak_concurrent_tasks: usize,
    /// Total tasks spawned
    pub total_tasks_spawned: u64,
    /// Total tasks completed
    pub total_tasks_completed: u64,
    /// Total tasks failed
    pub total_tasks_failed: u64,
    /// Current async memory usage
    pub current_async_memory: usize,
    /// Peak async memory usage
    pub peak_async_memory: usize,
    /// Total FFI calls made
    pub total_ffi_calls: u64,
    /// Total pointer registrations
    pub total_pointer_registrations: u64,
    /// Async memory usage as percentage of limit
    pub async_memory_usage_percent: f64,
    /// Task usage as percentage of limit
    pub task_usage_percent: f64,
    /// Current throttling level
    pub throttling_level: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_resource_monitor_task_limits() {
        let limits = AsyncResourceLimits {
            max_concurrent_tasks: 5,
            ..Default::default()
        };
        let monitor = AsyncResourceMonitor::new(limits);

        // Should succeed within limits
        for _ in 0..5 {
            assert!(monitor.record_task_spawn().is_ok());
        }

        // Should fail when exceeding limit
        assert!(monitor.record_task_spawn().is_err());
    }

    #[test]
    fn test_async_memory_tracking() {
        let limits = AsyncResourceLimits {
            max_task_memory_bytes: 1000,
            max_total_async_memory_bytes: 5000,
            ..Default::default()
        };
        let monitor = AsyncResourceMonitor::new(limits);

        // Should succeed within per-task limit
        assert!(monitor.record_task_memory_allocation(1, 500).is_ok());
        assert!(monitor.record_task_memory_allocation(2, 500).is_ok());

        // Should fail when exceeding per-task limit
        assert!(monitor.record_task_memory_allocation(1, 600).is_err());
    }

    #[test]
    fn test_rate_limiting() {
        let limits = AsyncResourceLimits {
            max_task_spawn_rate: 2.0, // 2 tasks per second
            rate_limit_window: Duration::from_secs(1),
            ..Default::default()
        };
        let monitor = AsyncResourceMonitor::new(limits);

        // Should succeed initially
        assert!(monitor.record_task_spawn().is_ok());
        assert!(monitor.record_task_spawn().is_ok());

        // Should fail when exceeding rate limit
        assert!(monitor.record_task_spawn().is_err());
    }

    #[test]
    fn test_system_health_check() {
        let limits = AsyncResourceLimits {
            max_concurrent_tasks: 100,
            max_total_async_memory_bytes: 1000,
            ..Default::default()
        };
        let monitor = AsyncResourceMonitor::new(limits);

        // Should be healthy initially
        assert!(monitor.check_system_health().is_ok());

        // Allocate memory close to limit
        let _ = monitor.record_task_memory_allocation(1, 950);

        // Should detect overload
        assert!(monitor.check_system_health().is_err());
    }
}
