//! Closure system for Script with performance optimizations
//!
//! This module provides both the original closure implementation and new optimized
//! versions that use string interning, efficient storage, and reduced allocations.

pub mod capture_storage;
pub mod debug;
pub mod id_cache;
pub mod optimized;
pub mod serialize;

pub use capture_storage::{CaptureStorage, CaptureStorageStats};
pub use debug::{
    debug_print_closure_state, debug_print_full_report, get_closure_debugger,
    init_closure_debugger, ClosureDebugInfo, ClosureDebugReport, ClosureDebugger, DebugConfig,
};
pub use id_cache::{global_function_id_cache, intern_function_id, FunctionId, OptimizedFunctionId};
pub use optimized::{create_optimized_closure_heap, OptimizedClosure, OptimizedClosureRuntime};
pub use serialize::{
    deserialize_closure, deserialize_optimized_closure, serialize_closure_binary,
    serialize_closure_compact, serialize_closure_json, serialize_optimized_closure_binary,
    ClosureMetadata, ClosureSerializer, SerializationConfig, SerializationFormat,
    SerializedClosure,
};

// Include the original implementation
mod original;
pub use original::{
    create_closure_heap, create_simple_closure, script_free_closure, Closure, ClosureRuntime,
};

/// Performance configuration for closure system
#[derive(Debug, Clone)]
pub struct ClosurePerformanceConfig {
    /// Enable function ID interning
    pub enable_id_interning: bool,
    /// Enable optimized capture storage
    pub enable_optimized_storage: bool,
    /// Enable parameter count caching
    pub enable_param_caching: bool,
    /// Enable cycle detection optimization
    pub enable_lazy_cycle_detection: bool,
}

impl Default for ClosurePerformanceConfig {
    fn default() -> Self {
        ClosurePerformanceConfig {
            enable_id_interning: true,
            enable_optimized_storage: true,
            enable_param_caching: true,
            enable_lazy_cycle_detection: true,
        }
    }
}

/// Global performance configuration
static mut GLOBAL_PERFORMANCE_CONFIG: ClosurePerformanceConfig = ClosurePerformanceConfig {
    enable_id_interning: true,
    enable_optimized_storage: true,
    enable_param_caching: true,
    enable_lazy_cycle_detection: true,
};

/// Get the global performance configuration
pub fn get_performance_config() -> ClosurePerformanceConfig {
    unsafe { GLOBAL_PERFORMANCE_CONFIG.clone() }
}

/// Set the global performance configuration
pub fn set_performance_config(config: ClosurePerformanceConfig) {
    unsafe {
        GLOBAL_PERFORMANCE_CONFIG = config;
    }
}

/// Performance statistics for the closure system
#[derive(Debug, Clone, Default)]
pub struct ClosurePerformanceStats {
    /// Number of closures created
    pub closures_created: usize,
    /// Number of closures executed
    pub closures_executed: usize,
    /// Storage usage statistics
    pub storage_stats: CaptureStorageStats,
    /// Function ID cache hit rate
    pub id_cache_hits: usize,
    /// Function ID cache misses
    pub id_cache_misses: usize,
    /// Parameter validation cache hits
    pub param_cache_hits: usize,
    /// Parameter validation cache misses
    pub param_cache_misses: usize,
}

impl ClosurePerformanceStats {
    /// Get the cache hit rate for function IDs
    pub fn id_cache_hit_rate(&self) -> f64 {
        let total = self.id_cache_hits + self.id_cache_misses;
        if total == 0 {
            0.0
        } else {
            (self.id_cache_hits as f64 / total as f64) * 100.0
        }
    }

    /// Get the cache hit rate for parameter validation
    pub fn param_cache_hit_rate(&self) -> f64 {
        let total = self.param_cache_hits + self.param_cache_misses;
        if total == 0 {
            0.0
        } else {
            (self.param_cache_hits as f64 / total as f64) * 100.0
        }
    }

    /// Get the total number of cache operations
    pub fn total_cache_operations(&self) -> usize {
        self.id_cache_hits + self.id_cache_misses + self.param_cache_hits + self.param_cache_misses
    }
}

/// Global performance statistics
static mut GLOBAL_PERFORMANCE_STATS: ClosurePerformanceStats = ClosurePerformanceStats {
    closures_created: 0,
    closures_executed: 0,
    storage_stats: CaptureStorageStats {
        inline_count: 0,
        hashmap_count: 0,
        total_captures: 0,
    },
    id_cache_hits: 0,
    id_cache_misses: 0,
    param_cache_hits: 0,
    param_cache_misses: 0,
};

/// Record closure creation in performance statistics
pub fn record_closure_creation(capture_count: usize, storage_type: &str) {
    unsafe {
        GLOBAL_PERFORMANCE_STATS.closures_created += 1;
        match storage_type {
            "inline" => GLOBAL_PERFORMANCE_STATS
                .storage_stats
                .record_inline(capture_count),
            "hashmap" => GLOBAL_PERFORMANCE_STATS
                .storage_stats
                .record_hashmap(capture_count),
            _ => {}
        }
    }
}

/// Record closure execution in performance statistics
pub fn record_closure_execution() {
    unsafe {
        GLOBAL_PERFORMANCE_STATS.closures_executed += 1;
    }
}

/// Record cache hit/miss statistics
pub fn record_cache_stat(cache_type: &str, hit: bool) {
    unsafe {
        match (cache_type, hit) {
            ("id", true) => GLOBAL_PERFORMANCE_STATS.id_cache_hits += 1,
            ("id", false) => GLOBAL_PERFORMANCE_STATS.id_cache_misses += 1,
            ("param", true) => GLOBAL_PERFORMANCE_STATS.param_cache_hits += 1,
            ("param", false) => GLOBAL_PERFORMANCE_STATS.param_cache_misses += 1,
            _ => {}
        }
    }
}

/// Get current performance statistics
pub fn get_performance_stats() -> ClosurePerformanceStats {
    unsafe { GLOBAL_PERFORMANCE_STATS.clone() }
}

/// Reset performance statistics
pub fn reset_performance_stats() {
    unsafe {
        GLOBAL_PERFORMANCE_STATS = ClosurePerformanceStats::default();
    }
}

/// Choose the optimal closure creation function based on configuration
pub fn create_closure_optimal(
    function_id: String,
    parameters: Vec<String>,
    captured_vars: Vec<(String, crate::runtime::Value)>,
    captures_by_ref: bool,
) -> crate::runtime::Value {
    let config = get_performance_config();

    if config.enable_id_interning && config.enable_optimized_storage {
        // Use fully optimized version
        create_optimized_closure_heap(function_id, parameters, captured_vars, captures_by_ref)
    } else {
        // Use original version
        create_closure_heap(function_id, parameters, captured_vars, captures_by_ref)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Value;

    #[test]
    fn test_performance_config() {
        let config = ClosurePerformanceConfig::default();
        assert!(config.enable_id_interning);
        assert!(config.enable_optimized_storage);
        assert!(config.enable_param_caching);
        assert!(config.enable_lazy_cycle_detection);
    }

    #[test]
    fn test_performance_stats() {
        reset_performance_stats();

        record_closure_creation(2, "inline");
        record_closure_creation(10, "hashmap");
        record_closure_execution();
        record_cache_stat("id", true);
        record_cache_stat("param", false);

        let stats = get_performance_stats();
        assert_eq!(stats.closures_created, 2);
        assert_eq!(stats.closures_executed, 1);
        assert_eq!(stats.storage_stats.inline_count, 1);
        assert_eq!(stats.storage_stats.hashmap_count, 1);
        assert_eq!(stats.id_cache_hits, 1);
        assert_eq!(stats.param_cache_misses, 1);
    }

    #[test]
    fn test_optimal_closure_creation() {
        let closure_value = create_closure_optimal(
            "test_optimal".to_string(),
            vec!["x".to_string()],
            vec![("captured".to_string(), Value::I32(42))],
            false,
        );

        match closure_value {
            Value::OptimizedClosure(_) => {
                // Should create optimized closure when optimizations are enabled
                assert!(true);
            }
            Value::Closure(_) => {
                // Should create regular closure when optimizations are disabled
                assert!(true);
            }
            _ => panic!("Expected closure value"),
        }
    }
}
