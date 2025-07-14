//! Debugging support for closure state visibility
//!
//! This module provides debugging capabilities for inspecting closure internals,
//! performance metrics, and runtime state during development and debugging.

use super::{Closure, ClosurePerformanceStats, OptimizedClosure};
use crate::runtime::Value;
use std::collections::HashMap;
use std::fmt;

/// Debug representation of a closure's state
#[derive(Debug, Clone)]
pub struct ClosureDebugInfo {
    /// Function identifier
    pub function_id: String,
    /// Parameters
    pub parameters: Vec<String>,
    /// Captured variables and their values
    pub captured_vars: HashMap<String, DebugValue>,
    /// Whether captures are by reference
    pub captures_by_ref: bool,
    /// Memory usage in bytes
    pub memory_usage: usize,
    /// Performance metrics
    pub performance_info: DebugPerformanceInfo,
}

/// Debug representation of a value (without circular references)
#[derive(Debug, Clone)]
pub enum DebugValue {
    /// Integer value
    I32(i32),
    /// Float value
    F32(f32),
    /// Boolean value
    Bool(bool),
    /// String value
    String(String),
    /// Reference to another closure (by ID to avoid cycles)
    ClosureRef(String),
    /// Unit value
    Unit,
    /// Complex value (truncated for display)
    Complex(String),
}

/// Performance debugging information
#[derive(Debug, Clone, Default)]
pub struct DebugPerformanceInfo {
    /// Number of times this closure was called
    pub call_count: u64,
    /// Total execution time in microseconds
    pub total_execution_time_us: u64,
    /// Average execution time in microseconds
    pub avg_execution_time_us: f64,
    /// Memory allocations count
    pub allocations: u64,
    /// Whether this closure is optimized
    pub is_optimized: bool,
    /// Optimization level applied
    pub optimization_level: String,
}

/// Global debugging state for closures
pub struct ClosureDebugger {
    /// All registered closures for debugging
    closures: HashMap<String, ClosureDebugInfo>,
    /// Global performance statistics
    global_stats: ClosurePerformanceStats,
    /// Debug configuration
    config: DebugConfig,
}

/// Configuration for closure debugging
#[derive(Debug, Clone)]
pub struct DebugConfig {
    /// Enable call tracing
    pub enable_call_tracing: bool,
    /// Enable memory tracking
    pub enable_memory_tracking: bool,
    /// Enable performance profiling
    pub enable_performance_profiling: bool,
    /// Maximum call stack depth to trace
    pub max_trace_depth: usize,
    /// Whether to capture variable values
    pub capture_variable_values: bool,
}

impl Default for DebugConfig {
    fn default() -> Self {
        DebugConfig {
            enable_call_tracing: true,
            enable_memory_tracking: true,
            enable_performance_profiling: true,
            max_trace_depth: 100,
            capture_variable_values: true,
        }
    }
}

impl ClosureDebugger {
    /// Create a new closure debugger
    pub fn new() -> Self {
        ClosureDebugger {
            closures: HashMap::new(),
            global_stats: ClosurePerformanceStats::default(),
            config: DebugConfig::default(),
        }
    }

    /// Register a closure for debugging
    pub fn register_closure(&mut self, closure: &Closure) {
        let debug_info = self.extract_debug_info(closure);
        self.closures
            .insert(closure.function_id.clone(), debug_info);
    }

    /// Register an optimized closure for debugging
    pub fn register_optimized_closure(&mut self, closure: &OptimizedClosure) {
        let debug_info = self.extract_optimized_debug_info(closure);
        let function_id = format!("{}", closure.function_id); // Convert to string
        self.closures.insert(function_id, debug_info);
    }

    /// Get debug information for a specific closure
    pub fn get_closure_info(&self, function_id: &str) -> Option<&ClosureDebugInfo> {
        self.closures.get(function_id)
    }

    /// Get all registered closures
    pub fn list_closures(&self) -> Vec<&ClosureDebugInfo> {
        self.closures.values().collect()
    }

    /// Update performance metrics for a closure
    pub fn record_call(&mut self, function_id: &str, execution_time_us: u64) {
        if let Some(info) = self.closures.get_mut(function_id) {
            info.performance_info.call_count += 1;
            info.performance_info.total_execution_time_us += execution_time_us;
            info.performance_info.avg_execution_time_us =
                info.performance_info.total_execution_time_us as f64
                    / info.performance_info.call_count as f64;
        }
    }

    /// Generate a debug report
    pub fn generate_report(&self) -> ClosureDebugReport {
        ClosureDebugReport {
            total_closures: self.closures.len(),
            active_closures: self
                .closures
                .values()
                .filter(|info| info.performance_info.call_count > 0)
                .count(),
            total_calls: self
                .closures
                .values()
                .map(|info| info.performance_info.call_count)
                .sum(),
            total_memory_usage: self.closures.values().map(|info| info.memory_usage).sum(),
            average_execution_time: {
                let total_time: u64 = self
                    .closures
                    .values()
                    .map(|info| info.performance_info.total_execution_time_us)
                    .sum();
                let total_calls: u64 = self
                    .closures
                    .values()
                    .map(|info| info.performance_info.call_count)
                    .sum();
                if total_calls > 0 {
                    total_time as f64 / total_calls as f64
                } else {
                    0.0
                }
            },
            optimization_stats: self.calculate_optimization_stats(),
        }
    }

    /// Extract debug information from a closure
    fn extract_debug_info(&self, closure: &Closure) -> ClosureDebugInfo {
        let captured_vars = closure
            .captured_vars
            .iter()
            .map(|(name, value)| (name.clone(), self.value_to_debug_value(value)))
            .collect();

        ClosureDebugInfo {
            function_id: closure.function_id.clone(),
            parameters: closure.parameters.clone(),
            captured_vars,
            captures_by_ref: closure.captures_by_ref,
            memory_usage: std::mem::size_of_val(closure) + closure.captured_vars.len() * 64, // Rough estimate
            performance_info: DebugPerformanceInfo {
                is_optimized: false,
                optimization_level: "None".to_string(),
                ..Default::default()
            },
        }
    }

    /// Extract debug information from an optimized closure
    fn extract_optimized_debug_info(&self, closure: &OptimizedClosure) -> ClosureDebugInfo {
        let captured_vars: HashMap<String, DebugValue> = match &closure.captured_vars {
            super::capture_storage::CaptureStorage::Inline(captures) => captures
                .iter()
                .enumerate()
                .map(|(i, (_name, value))| {
                    (format!("capture_{i}"), self.value_to_debug_value(value))
                })
                .collect(),
            super::capture_storage::CaptureStorage::HashMap(map) => map
                .iter()
                .map(|(name, value)| (name.clone(), self.value_to_debug_value(value)))
                .collect(),
        };

        ClosureDebugInfo {
            function_id: format!("{}", closure.function_id),
            parameters: closure.parameters.to_vec(),
            captured_vars: captured_vars.clone(),
            captures_by_ref: closure.captures_by_ref,
            memory_usage: std::mem::size_of_val(closure) + captured_vars.len() * 64, // Rough estimate since memory_usage method doesn't exist
            performance_info: DebugPerformanceInfo {
                is_optimized: true,
                optimization_level: "Optimized".to_string(),
                ..Default::default()
            },
        }
    }

    /// Convert a Value to a DebugValue (safe for circular references)
    fn value_to_debug_value(&self, value: &Value) -> DebugValue {
        match value {
            Value::I32(val) => DebugValue::I32(*val),
            Value::F32(val) => DebugValue::F32(*val),
            Value::Bool(val) => DebugValue::Bool(*val),
            Value::String(val) => DebugValue::String(val.clone()),
            Value::Null => DebugValue::Unit,
            Value::Closure(closure) => DebugValue::ClosureRef(closure.function_id.clone()),
            Value::OptimizedClosure(closure) => {
                DebugValue::ClosureRef(format!("{}", closure.function_id))
            }
            _ => DebugValue::Complex(format!("{:?}", value)),
        }
    }

    /// Calculate optimization statistics
    fn calculate_optimization_stats(&self) -> OptimizationStats {
        let total = self.closures.len();
        let optimized = self
            .closures
            .values()
            .filter(|info| info.performance_info.is_optimized)
            .count();

        OptimizationStats {
            total_closures: total,
            optimized_closures: optimized,
            optimization_rate: if total > 0 {
                (optimized as f64 / total as f64) * 100.0
            } else {
                0.0
            },
        }
    }
}

/// Report generated by the closure debugger
#[derive(Debug, Clone)]
pub struct ClosureDebugReport {
    /// Total number of closures
    pub total_closures: usize,
    /// Number of closures that have been called
    pub active_closures: usize,
    /// Total number of calls across all closures
    pub total_calls: u64,
    /// Total memory usage in bytes
    pub total_memory_usage: usize,
    /// Average execution time across all calls
    pub average_execution_time: f64,
    /// Optimization statistics
    pub optimization_stats: OptimizationStats,
}

/// Optimization statistics
#[derive(Debug, Clone)]
pub struct OptimizationStats {
    /// Total number of closures
    pub total_closures: usize,
    /// Number of optimized closures
    pub optimized_closures: usize,
    /// Optimization rate as a percentage
    pub optimization_rate: f64,
}

impl fmt::Display for ClosureDebugReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== Closure Debug Report ===")?;
        writeln!(f, "Total Closures: {}", self.total_closures)?;
        writeln!(f, "Active Closures: {}", self.active_closures)?;
        writeln!(f, "Total Calls: {}", self.total_calls)?;
        writeln!(f, "Total Memory Usage: {} bytes", self.total_memory_usage)?;
        writeln!(
            f,
            "Average Execution Time: {:.2} μs",
            self.average_execution_time
        )?;
        writeln!(f, "")?;
        writeln!(f, "=== Optimization Statistics ===")?;
        writeln!(
            f,
            "Optimized: {}/{} ({:.1}%)",
            self.optimization_stats.optimized_closures,
            self.optimization_stats.total_closures,
            self.optimization_stats.optimization_rate
        )?;
        Ok(())
    }
}

impl fmt::Display for ClosureDebugInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== Closure: {} ===", self.function_id)?;
        writeln!(f, "Parameters: {:?}", self.parameters)?;
        writeln!(f, "Captures by ref: {}", self.captures_by_ref)?;
        writeln!(f, "Memory usage: {} bytes", self.memory_usage)?;
        writeln!(f, "Optimized: {}", self.performance_info.is_optimized)?;
        writeln!(
            f,
            "Optimization level: {}",
            self.performance_info.optimization_level
        )?;
        writeln!(f, "Call count: {}", self.performance_info.call_count)?;
        writeln!(
            f,
            "Average execution time: {:.2} μs",
            self.performance_info.avg_execution_time_us
        )?;
        writeln!(f, "")?;
        writeln!(f, "Captured variables:")?;
        for (name, value) in &self.captured_vars {
            writeln!(f, "  {}: {:?}", name, value)?;
        }
        Ok(())
    }
}

/// Global closure debugger instance
static mut GLOBAL_CLOSURE_DEBUGGER: Option<ClosureDebugger> = None;

/// Initialize the global closure debugger
pub fn init_closure_debugger() {
    unsafe {
        GLOBAL_CLOSURE_DEBUGGER = Some(ClosureDebugger::new());
    }
}

/// Get a reference to the global closure debugger
pub fn get_closure_debugger() -> Option<&'static mut ClosureDebugger> {
    unsafe { GLOBAL_CLOSURE_DEBUGGER.as_mut() }
}

/// Debug print a closure's state
pub fn debug_print_closure_state(function_id: &str) {
    if let Some(debugger) = get_closure_debugger() {
        if let Some(info) = debugger.get_closure_info(function_id) {
            println!("{info}");
        } else {
            println!("Closure '{}' not found in debugger", function_id);
        }
    } else {
        println!("Closure debugger not initialized");
    }
}

/// Print a full debug report
pub fn debug_print_full_report() {
    if let Some(debugger) = get_closure_debugger() {
        println!("{}", debugger.generate_report());

        println!("\n=== Individual Closure Details ===");
        for info in debugger.list_closures() {
            println!("{info}");
        }
    } else {
        println!("Closure debugger not initialized");
    }
}

/// Macro for conditional debug printing
#[macro_export]
macro_rules! closure_debug {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            println!("[CLOSURE DEBUG] {}", format!($($arg)*));
        }
    };
}
