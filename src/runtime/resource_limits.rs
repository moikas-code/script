//! Resource limit enforcement for Script runtime
//!
//! This module provides comprehensive resource limiting to prevent denial of service
//! attacks and ensure stable operation under adversarial conditions.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Resource limits configuration
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum memory usage in bytes
    pub max_memory_bytes: usize,
    /// Maximum number of allocations
    pub max_allocations: usize,
    /// Maximum collection time per cycle
    pub max_collection_time: Duration,
    /// Maximum graph traversal depth
    pub max_graph_depth: usize,
    /// Maximum number of possible roots
    pub max_possible_roots: usize,
    /// Maximum incremental work per step
    pub max_incremental_work: usize,
    /// Memory pressure threshold (percentage)
    pub memory_pressure_threshold: f64,
    /// Enable automatic throttling
    pub enable_auto_throttling: bool,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 1024 * 1024 * 1024, // 1GB
            max_allocations: 10_000_000,
            max_collection_time: Duration::from_secs(1),
            max_graph_depth: 10_000,
            max_possible_roots: 100_000,
            max_incremental_work: 1000,
            memory_pressure_threshold: 0.8, // 80%
            enable_auto_throttling: true,
        }
    }
}

/// Resource usage tracking
#[derive(Debug, Default)]
pub struct ResourceUsage {
    /// Current memory usage in bytes
    current_memory: AtomicUsize,
    /// Current number of allocations
    current_allocations: AtomicUsize,
    /// Peak memory usage
    peak_memory: AtomicUsize,
    /// Peak allocations
    peak_allocations: AtomicUsize,
    /// Total collections performed
    total_collections: AtomicUsize,
    /// Total collection time
    total_collection_time: AtomicU64, // Nanoseconds
    /// Collection history for throttling
    collection_history: Mutex<VecDeque<CollectionMetrics>>,
}

/// Metrics for a single collection
#[derive(Debug, Clone)]
struct CollectionMetrics {
    /// When the collection started
    timestamp: Instant,
    /// How long it took
    duration: Duration,
    /// Objects processed
    objects_processed: usize,
    /// Memory freed
    memory_freed: usize,
}

/// Resource limit violations
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceViolation {
    /// Memory limit exceeded
    MemoryLimitExceeded { current: usize, limit: usize },
    /// Allocation count limit exceeded
    AllocationLimitExceeded { current: usize, limit: usize },
    /// Collection time limit exceeded
    CollectionTimeExceeded { current: Duration, limit: Duration },
    /// Graph depth limit exceeded
    GraphDepthExceeded { current: usize, limit: usize },
    /// Possible roots limit exceeded
    PossibleRootsExceeded { current: usize, limit: usize },
    /// Memory pressure threshold exceeded
    MemoryPressure { usage_percent: f64, threshold: f64 },
}

/// Resource monitor with enforcement capabilities
pub struct ResourceMonitor {
    /// Resource limits configuration
    limits: ResourceLimits,
    /// Current resource usage
    usage: ResourceUsage,
    /// Auto-throttling state
    throttling: Mutex<ThrottlingState>,
}

/// Auto-throttling state
#[derive(Debug)]
struct ThrottlingState {
    /// Current throttling level (0.0 = no throttling, 1.0 = maximum)
    level: f64,
    /// Number of consecutive high-pressure collections
    pressure_streak: usize,
    /// Last adjustment time
    last_adjustment: Instant,
    /// Minimum time between adjustments
    adjustment_cooldown: Duration,
}

impl Default for ThrottlingState {
    fn default() -> Self {
        Self {
            level: 0.0,
            pressure_streak: 0,
            last_adjustment: Instant::now(),
            adjustment_cooldown: Duration::from_millis(100),
        }
    }
}

impl ResourceMonitor {
    /// Create a new resource monitor
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            limits,
            usage: ResourceUsage::default(),
            throttling: Mutex::new(ThrottlingState::default()),
        }
    }

    /// Record memory allocation
    pub fn record_allocation(&self, size: usize) -> Result<(), ResourceViolation> {
        let new_memory = self.usage.current_memory.fetch_add(size, Ordering::Relaxed) + size;
        let new_allocs = self
            .usage
            .current_allocations
            .fetch_add(1, Ordering::Relaxed)
            + 1;

        // Update peaks
        self.usage
            .peak_memory
            .fetch_max(new_memory, Ordering::Relaxed);
        self.usage
            .peak_allocations
            .fetch_max(new_allocs, Ordering::Relaxed);

        // Check limits
        if new_memory > self.limits.max_memory_bytes {
            return Err(ResourceViolation::MemoryLimitExceeded {
                current: new_memory,
                limit: self.limits.max_memory_bytes,
            });
        }

        if new_allocs > self.limits.max_allocations {
            return Err(ResourceViolation::AllocationLimitExceeded {
                current: new_allocs,
                limit: self.limits.max_allocations,
            });
        }

        // Check memory pressure
        let usage_percent = new_memory as f64 / self.limits.max_memory_bytes as f64;
        if usage_percent > self.limits.memory_pressure_threshold {
            return Err(ResourceViolation::MemoryPressure {
                usage_percent,
                threshold: self.limits.memory_pressure_threshold,
            });
        }

        Ok(())
    }

    /// Record memory deallocation
    pub fn record_deallocation(&self, size: usize) {
        self.usage.current_memory.fetch_sub(
            size.min(self.usage.current_memory.load(Ordering::Relaxed)),
            Ordering::Relaxed,
        );

        self.usage.current_allocations.fetch_sub(
            1.min(self.usage.current_allocations.load(Ordering::Relaxed)),
            Ordering::Relaxed,
        );
    }

    /// Start a collection with time tracking
    pub fn start_collection(&self) -> CollectionTracker {
        CollectionTracker {
            monitor: self,
            start_time: Instant::now(),
            objects_processed: 0,
            initial_memory: self.usage.current_memory.load(Ordering::Relaxed),
        }
    }

    /// Check if collection time limit would be exceeded
    pub fn check_collection_time(&self, elapsed: Duration) -> Result<(), ResourceViolation> {
        if elapsed > self.limits.max_collection_time {
            return Err(ResourceViolation::CollectionTimeExceeded {
                current: elapsed,
                limit: self.limits.max_collection_time,
            });
        }
        Ok(())
    }

    /// Check if graph depth limit would be exceeded
    pub fn check_graph_depth(&self, depth: usize) -> Result<(), ResourceViolation> {
        if depth > self.limits.max_graph_depth {
            return Err(ResourceViolation::GraphDepthExceeded {
                current: depth,
                limit: self.limits.max_graph_depth,
            });
        }
        Ok(())
    }

    /// Check if possible roots limit would be exceeded
    pub fn check_possible_roots(&self, count: usize) -> Result<(), ResourceViolation> {
        if count > self.limits.max_possible_roots {
            return Err(ResourceViolation::PossibleRootsExceeded {
                current: count,
                limit: self.limits.max_possible_roots,
            });
        }
        Ok(())
    }

    /// Get current memory usage
    pub fn current_memory_usage(&self) -> usize {
        self.usage.current_memory.load(Ordering::Relaxed)
    }

    /// Get current allocation count
    pub fn current_allocations(&self) -> usize {
        self.usage.current_allocations.load(Ordering::Relaxed)
    }

    /// Get memory usage percentage
    pub fn memory_usage_percent(&self) -> f64 {
        let current = self.current_memory_usage() as f64;
        let max = self.limits.max_memory_bytes as f64;
        current / max
    }

    /// Check if under memory pressure
    pub fn is_under_memory_pressure(&self) -> bool {
        self.memory_usage_percent() > self.limits.memory_pressure_threshold
    }

    /// Get current throttling level
    pub fn get_throttling_level(&self) -> f64 {
        if let Ok(throttling) = self.throttling.lock() {
            throttling.level
        } else {
            0.0
        }
    }

    /// Adjust throttling based on recent collection performance
    fn adjust_throttling(&self, metrics: &CollectionMetrics) {
        if !self.limits.enable_auto_throttling {
            return;
        }

        if let Ok(mut throttling) = self.throttling.lock() {
            let now = Instant::now();

            // Check cooldown period
            if now.duration_since(throttling.last_adjustment) < throttling.adjustment_cooldown {
                return;
            }

            // Calculate pressure indicators
            let memory_pressure = self.memory_usage_percent();
            let time_pressure =
                metrics.duration.as_secs_f64() / self.limits.max_collection_time.as_secs_f64();

            let is_high_pressure =
                memory_pressure > self.limits.memory_pressure_threshold || time_pressure > 0.8;

            if is_high_pressure {
                throttling.pressure_streak += 1;
                // Increase throttling exponentially with pressure streak
                let adjustment = 0.1 * (1.0 + throttling.pressure_streak as f64 * 0.1);
                throttling.level = (throttling.level + adjustment).min(1.0);
            } else {
                throttling.pressure_streak = 0;
                // Gradually reduce throttling
                throttling.level = (throttling.level - 0.05).max(0.0);
            }

            throttling.last_adjustment = now;
        }
    }

    /// Get resource usage statistics
    pub fn get_stats(&self) -> ResourceStats {
        ResourceStats {
            current_memory: self.usage.current_memory.load(Ordering::Relaxed),
            peak_memory: self.usage.peak_memory.load(Ordering::Relaxed),
            current_allocations: self.usage.current_allocations.load(Ordering::Relaxed),
            peak_allocations: self.usage.peak_allocations.load(Ordering::Relaxed),
            total_collections: self.usage.total_collections.load(Ordering::Relaxed),
            total_collection_time: Duration::from_nanos(
                self.usage.total_collection_time.load(Ordering::Relaxed),
            ),
            memory_usage_percent: self.memory_usage_percent(),
            throttling_level: self.get_throttling_level(),
            is_under_pressure: self.is_under_memory_pressure(),
        }
    }
}

/// Collection tracking helper
pub struct CollectionTracker<'a> {
    monitor: &'a ResourceMonitor,
    start_time: Instant,
    objects_processed: usize,
    initial_memory: usize,
}

impl<'a> CollectionTracker<'a> {
    /// Record processing an object
    pub fn record_object_processed(&mut self) {
        self.objects_processed += 1;
    }

    /// Check if we should continue collection based on limits
    pub fn should_continue(&self) -> Result<(), ResourceViolation> {
        let elapsed = self.start_time.elapsed();
        self.monitor.check_collection_time(elapsed)
    }

    /// Finish the collection and update metrics
    pub fn finish(self) {
        let duration = self.start_time.elapsed();
        let final_memory = self.monitor.usage.current_memory.load(Ordering::Relaxed);
        let memory_freed = self.initial_memory.saturating_sub(final_memory);

        // Update global stats
        self.monitor
            .usage
            .total_collections
            .fetch_add(1, Ordering::Relaxed);
        self.monitor
            .usage
            .total_collection_time
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);

        // Create metrics
        let metrics = CollectionMetrics {
            timestamp: self.start_time,
            duration,
            objects_processed: self.objects_processed,
            memory_freed,
        };

        // Update collection history
        if let Ok(mut history) = self.monitor.usage.collection_history.lock() {
            history.push_back(metrics.clone());

            // Keep only recent history (last 100 collections)
            while history.len() > 100 {
                history.pop_front();
            }
        }

        // Adjust throttling
        self.monitor.adjust_throttling(&metrics);
    }
}

/// Resource usage statistics
#[derive(Debug, Clone)]
pub struct ResourceStats {
    /// Current memory usage in bytes
    pub current_memory: usize,
    /// Peak memory usage in bytes
    pub peak_memory: usize,
    /// Current number of allocations
    pub current_allocations: usize,
    /// Peak number of allocations
    pub peak_allocations: usize,
    /// Total collections performed
    pub total_collections: usize,
    /// Total time spent in collections
    pub total_collection_time: Duration,
    /// Current memory usage as percentage of limit
    pub memory_usage_percent: f64,
    /// Current throttling level (0.0-1.0)
    pub throttling_level: f64,
    /// Whether system is under memory pressure
    pub is_under_pressure: bool,
}

/// Time-bounded operation helper
pub struct TimeBudget {
    start_time: Instant,
    max_duration: Duration,
}

impl TimeBudget {
    /// Create a new time budget
    pub fn new(max_duration: Duration) -> Self {
        Self {
            start_time: Instant::now(),
            max_duration,
        }
    }

    /// Check if time budget is exhausted
    pub fn is_exhausted(&self) -> bool {
        self.start_time.elapsed() > self.max_duration
    }

    /// Get remaining time
    pub fn remaining(&self) -> Duration {
        self.max_duration.saturating_sub(self.start_time.elapsed())
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Check and return error if exhausted
    pub fn check(&self) -> Result<(), ResourceViolation> {
        if self.is_exhausted() {
            Err(ResourceViolation::CollectionTimeExceeded {
                current: self.elapsed(),
                limit: self.max_duration,
            })
        } else {
            Ok(())
        }
    }
}

/// Work budget for incremental operations
pub struct WorkBudget {
    remaining_work: usize,
}

impl WorkBudget {
    /// Create a new work budget
    pub fn new(max_work: usize) -> Self {
        Self {
            remaining_work: max_work,
        }
    }

    /// Try to consume work units
    pub fn try_consume(&mut self, units: usize) -> bool {
        if self.remaining_work >= units {
            self.remaining_work -= units;
            true
        } else {
            false
        }
    }

    /// Check if budget is exhausted
    pub fn is_exhausted(&self) -> bool {
        self.remaining_work == 0
    }

    /// Get remaining work units
    pub fn remaining(&self) -> usize {
        self.remaining_work
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_monitor_allocation_limits() {
        let limits = ResourceLimits {
            max_memory_bytes: 1000,
            max_allocations: 5,
            ..Default::default()
        };
        let monitor = ResourceMonitor::new(limits);

        // Should succeed within limits
        assert!(monitor.record_allocation(100).is_ok());
        assert!(monitor.record_allocation(100).is_ok());

        // Should fail when exceeding memory limit
        assert!(monitor.record_allocation(900).is_err());
    }

    #[test]
    fn test_time_budget() {
        let budget = TimeBudget::new(Duration::from_millis(10));
        assert!(!budget.is_exhausted());

        std::thread::sleep(Duration::from_millis(15));
        assert!(budget.is_exhausted());
        assert!(budget.check().is_err());
    }

    #[test]
    fn test_work_budget() {
        let mut budget = WorkBudget::new(10);
        assert!(!budget.is_exhausted());

        assert!(budget.try_consume(5));
        assert_eq!(budget.remaining(), 5);

        assert!(budget.try_consume(5));
        assert!(budget.is_exhausted());

        assert!(!budget.try_consume(1));
    }

    #[test]
    fn test_collection_tracker() {
        let limits = ResourceLimits::default();
        let monitor = ResourceMonitor::new(limits);

        let tracker = monitor.start_collection();
        assert!(tracker.should_continue().is_ok());

        tracker.finish();
        let stats = monitor.get_stats();
        assert_eq!(stats.total_collections, 1);
    }

    #[test]
    fn test_memory_pressure_detection() {
        let limits = ResourceLimits {
            max_memory_bytes: 1000,
            memory_pressure_threshold: 0.8,
            ..Default::default()
        };
        let monitor = ResourceMonitor::new(limits);

        assert!(!monitor.is_under_memory_pressure());

        // Allocate to 90% capacity
        let _ = monitor.record_allocation(900);
        assert!(monitor.is_under_memory_pressure());
    }
}
