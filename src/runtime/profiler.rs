//! Memory profiler for Script runtime
//!
//! This module provides memory profiling capabilities including:
//! - Tracking allocations and deallocations
//! - Memory usage statistics
//! - Leak detection
//! - Allocation hotspot analysis

use std::backtrace::Backtrace;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

/// Global profiler instance
static PROFILER: RwLock<Option<Arc<MemoryProfiler>>> = RwLock::new(None);

/// Initialize the memory profiler
pub fn initialize() {
    let mut profiler = PROFILER.write().unwrap();
    *profiler = Some(Arc::new(MemoryProfiler::new()));
}

/// Shutdown the memory profiler
pub fn shutdown() {
    let mut profiler = PROFILER.write().unwrap();
    if let Some(p) = profiler.take() {
        p.report_leaks();
    }
}

/// Record an allocation
pub fn record_allocation(size: usize, type_name: &str) {
    if let Ok(profiler) = PROFILER.read() {
        if let Some(p) = profiler.as_ref() {
            p.record_allocation(size, type_name);
        }
    }
}

/// Record a deallocation
pub fn record_deallocation(size: usize, type_name: &str) {
    if let Ok(profiler) = PROFILER.read() {
        if let Some(p) = profiler.as_ref() {
            p.record_deallocation(size, type_name);
        }
    }
}

/// Record a GC collection
pub fn record_gc_collection(objects_collected: usize, duration: Duration) {
    if let Ok(profiler) = PROFILER.read() {
        if let Some(p) = profiler.as_ref() {
            p.record_gc_collection(objects_collected, duration);
        }
    }
}

/// Get current profiling statistics
pub fn get_stats() -> Option<ProfilingStats> {
    if let Ok(profiler) = PROFILER.read() {
        if let Some(p) = profiler.as_ref() {
            return Some(p.get_stats());
        }
    }
    None
}

/// Memory profiler
pub struct MemoryProfiler {
    /// Whether profiling is enabled
    enabled: AtomicBool,
    /// Allocation tracking
    allocations: Mutex<AllocationTracker>,
    /// Type statistics
    type_stats: RwLock<HashMap<String, TypeStats>>,
    /// GC statistics
    gc_stats: Mutex<GcStats>,
    /// Profiling start time
    start_time: Instant,
}

/// Tracks individual allocations
struct AllocationTracker {
    /// Active allocations (address -> allocation info)
    active: HashMap<usize, AllocationInfo>,
    /// Total allocations
    total_allocations: usize,
    /// Total deallocations
    total_deallocations: usize,
    /// Total bytes allocated
    total_bytes_allocated: usize,
    /// Total bytes deallocated
    total_bytes_deallocated: usize,
    /// Peak memory usage
    peak_memory: usize,
    /// Current memory usage
    current_memory: usize,
}

/// Information about a single allocation
#[derive(Debug, Clone)]
struct AllocationInfo {
    /// Size of allocation
    size: usize,
    /// Type name
    type_name: String,
    /// Allocation time
    #[allow(dead_code)]
    timestamp: Instant,
    /// Allocation backtrace (if enabled)
    backtrace: Option<String>,
}

/// Statistics for a specific type
#[derive(Debug, Clone, Default)]
pub struct TypeStats {
    /// Number of allocations
    pub allocations: usize,
    /// Number of deallocations
    pub deallocations: usize,
    /// Total bytes allocated
    pub total_bytes: usize,
    /// Current bytes in use
    pub current_bytes: usize,
    /// Peak bytes used
    pub peak_bytes: usize,
}

/// GC statistics
#[derive(Debug, Clone, Default)]
struct GcStats {
    /// Total collections
    collections: usize,
    /// Total objects collected
    objects_collected: usize,
    /// Total time spent in GC
    total_time: Duration,
    /// Last collection time
    last_collection: Option<Instant>,
}

/// Allocation statistics
#[derive(Debug, Clone)]
pub struct AllocationStats {
    /// Total number of allocations
    pub total_allocations: usize,
    /// Total number of deallocations
    pub total_deallocations: usize,
    /// Total bytes allocated
    pub total_bytes_allocated: usize,
    /// Total bytes deallocated
    pub total_bytes_deallocated: usize,
    /// Current memory usage
    pub current_memory: usize,
    /// Peak memory usage
    pub peak_memory: usize,
    /// Number of potential leaks
    pub potential_leaks: usize,
}

/// Complete profiling statistics
#[derive(Debug, Clone)]
pub struct ProfilingStats {
    /// Allocation statistics
    pub allocations: AllocationStats,
    /// Per-type statistics
    pub type_stats: HashMap<String, TypeStats>,
    /// GC statistics
    pub gc_collections: usize,
    pub gc_objects_collected: usize,
    pub gc_total_time: Duration,
    /// Profiling duration
    pub duration: Duration,
}

impl MemoryProfiler {
    /// Create a new memory profiler
    fn new() -> Self {
        MemoryProfiler {
            enabled: AtomicBool::new(true),
            allocations: Mutex::new(AllocationTracker::new()),
            type_stats: RwLock::new(HashMap::new()),
            gc_stats: Mutex::new(GcStats::default()),
            start_time: Instant::now(),
        }
    }

    /// Enable or disable profiling
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Check if profiling is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Record an allocation
    fn record_allocation(&self, size: usize, type_name: &str) {
        if !self.is_enabled() {
            return;
        }

        // Update allocation tracker
        let mut tracker = self.allocations.lock().unwrap();
        tracker.total_allocations += 1;
        tracker.total_bytes_allocated += size;
        tracker.current_memory += size;

        if tracker.current_memory > tracker.peak_memory {
            tracker.peak_memory = tracker.current_memory;
        }

        // Record allocation with backtrace if in debug mode
        let allocation = AllocationInfo {
            size,
            type_name: type_name.to_string(),
            timestamp: Instant::now(),
            backtrace: if cfg!(debug_assertions) {
                Some(Backtrace::capture().to_string())
            } else {
                None
            },
        };

        // Use a placeholder address for now
        let address = tracker.active.len();
        tracker.active.insert(address, allocation);

        // Update type statistics
        drop(tracker); // Release lock
        let mut type_stats = self.type_stats.write().unwrap();
        let stats = type_stats.entry(type_name.to_string()).or_default();
        stats.allocations += 1;
        stats.total_bytes += size;
        stats.current_bytes += size;
        if stats.current_bytes > stats.peak_bytes {
            stats.peak_bytes = stats.current_bytes;
        }
    }

    /// Record a deallocation
    fn record_deallocation(&self, size: usize, type_name: &str) {
        if !self.is_enabled() {
            return;
        }

        // Update allocation tracker
        let mut tracker = self.allocations.lock().unwrap();
        tracker.total_deallocations += 1;
        tracker.total_bytes_deallocated += size;
        tracker.current_memory = tracker.current_memory.saturating_sub(size);

        // Remove from active allocations (simplified - in reality would use actual address)
        if let Some((&addr, _)) = tracker
            .active
            .iter()
            .find(|(_, info)| info.type_name == type_name && info.size == size)
        {
            tracker.active.remove(&addr);
        }

        // Update type statistics
        drop(tracker); // Release lock
        let mut type_stats = self.type_stats.write().unwrap();
        if let Some(stats) = type_stats.get_mut(type_name) {
            stats.deallocations += 1;
            stats.current_bytes = stats.current_bytes.saturating_sub(size);
        }
    }

    /// Record a GC collection
    fn record_gc_collection(&self, objects_collected: usize, duration: Duration) {
        let mut gc_stats = self.gc_stats.lock().unwrap();
        gc_stats.collections += 1;
        gc_stats.objects_collected += objects_collected;
        gc_stats.total_time += duration;
        gc_stats.last_collection = Some(Instant::now());
    }

    /// Get current statistics
    pub fn get_stats(&self) -> ProfilingStats {
        let tracker = self.allocations.lock().unwrap();
        let type_stats = self.type_stats.read().unwrap();
        let gc_stats = self.gc_stats.lock().unwrap();

        ProfilingStats {
            allocations: AllocationStats {
                total_allocations: tracker.total_allocations,
                total_deallocations: tracker.total_deallocations,
                total_bytes_allocated: tracker.total_bytes_allocated,
                total_bytes_deallocated: tracker.total_bytes_deallocated,
                current_memory: tracker.current_memory,
                peak_memory: tracker.peak_memory,
                potential_leaks: tracker.active.len(),
            },
            type_stats: type_stats.clone(),
            gc_collections: gc_stats.collections,
            gc_objects_collected: gc_stats.objects_collected,
            gc_total_time: gc_stats.total_time,
            duration: self.start_time.elapsed(),
        }
    }

    /// Report potential memory leaks
    fn report_leaks(&self) {
        let tracker = self.allocations.lock().unwrap();

        if !tracker.active.is_empty() {
            eprintln!("\n=== POTENTIAL MEMORY LEAKS DETECTED ===");
            eprintln!("{} allocations not freed:", tracker.active.len());

            // Group by type
            let mut leaks_by_type: HashMap<String, Vec<&AllocationInfo>> = HashMap::new();
            for info in tracker.active.values() {
                leaks_by_type
                    .entry(info.type_name.clone())
                    .or_default()
                    .push(info);
            }

            // Report leaks by type
            for (type_name, leaks) in leaks_by_type {
                let total_size: usize = leaks.iter().map(|l| l.size).sum();
                eprintln!("\n  Type: {type_name}");
                eprintln!("  Count: {leaks.len(}"));
                eprintln!("  Total size: {} bytes", total_size);

                // Show first few allocations with backtraces if available
                for (i, leak) in leaks.iter().take(3).enumerate() {
                    eprintln!("  Allocation #{}: {} bytes", i + 1, leak.size);
                    if let Some(bt) = &leak.backtrace {
                        eprintln!("    Backtrace:\n{bt}");
                    }
                }

                if leaks.len() > 3 {
                    eprintln!("  ... and {} more", leaks.len() - 3);
                }
            }

            eprintln!("\n=======================================\n");
        }
    }

    /// Generate a memory report
    pub fn generate_report(&self) -> String {
        let stats = self.get_stats();
        let mut report = String::new();

        report.push_str(&format!("=== Memory Profile Report ===\n"));
        report.push_str(&format!("Duration: {:?}\n\n", stats.duration));

        report.push_str(&format!("Allocation Summary:\n"));
        report.push_str(&format!(
            "  Total allocations: {}\n",
            stats.allocations.total_allocations
        ));
        report.push_str(&format!(
            "  Total deallocations: {}\n",
            stats.allocations.total_deallocations
        ));
        report.push_str(&format!(
            "  Total bytes allocated: {} bytes\n",
            stats.allocations.total_bytes_allocated
        ));
        report.push_str(&format!(
            "  Total bytes freed: {} bytes\n",
            stats.allocations.total_bytes_deallocated
        ));
        report.push_str(&format!(
            "  Current memory usage: {} bytes\n",
            stats.allocations.current_memory
        ));
        report.push_str(&format!(
            "  Peak memory usage: {} bytes\n",
            stats.allocations.peak_memory
        ));
        report.push_str(&format!(
            "  Potential leaks: {}\n\n",
            stats.allocations.potential_leaks
        ));

        report.push_str(&format!("GC Summary:\n"));
        report.push_str(&format!("  Collections: {}\n", stats.gc_collections));
        report.push_str(&format!(
            "  Objects collected: {}\n",
            stats.gc_objects_collected
        ));
        report.push_str(&format!("  Total GC time: {:?}\n\n", stats.gc_total_time));

        report.push_str(&format!("Type Statistics:\n"));
        let mut type_stats: Vec<_> = stats.type_stats.iter().collect();
        type_stats.sort_by_key(|(_, s)| s.current_bytes);
        type_stats.reverse();

        for (type_name, stats) in type_stats.iter().take(10) {
            report.push_str(&format!("  {}:\n", type_name));
            report.push_str(&format!("    Allocations: {}\n", stats.allocations));
            report.push_str(&format!("    Current bytes: {}\n", stats.current_bytes));
            report.push_str(&format!("    Peak bytes: {}\n", stats.peak_bytes));
        }

        report
    }
}

impl AllocationTracker {
    fn new() -> Self {
        AllocationTracker {
            active: HashMap::new(),
            total_allocations: 0,
            total_deallocations: 0,
            total_bytes_allocated: 0,
            total_bytes_deallocated: 0,
            peak_memory: 0,
            current_memory: 0,
        }
    }
}

/// Check for memory leaks
pub fn check_leaks() -> bool {
    if let Ok(profiler) = PROFILER.read() {
        if let Some(p) = profiler.as_ref() {
            let stats = p.get_stats();
            return stats.allocations.potential_leaks > 0;
        }
    }
    false
}

/// Generate memory report
pub fn generate_report() -> Option<String> {
    if let Ok(profiler) = PROFILER.read() {
        if let Some(p) = profiler.as_ref() {
            return Some(p.generate_report());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_lifecycle() {
        initialize();

        // Record some allocations
        record_allocation(100, "test::Type1");
        record_allocation(200, "test::Type2");
        record_deallocation(100, "test::Type1");

        // Get stats
        let stats = get_stats().unwrap();
        assert_eq!(stats.allocations.total_allocations, 2);
        assert_eq!(stats.allocations.total_deallocations, 1);
        assert_eq!(stats.allocations.current_memory, 200);
        assert_eq!(stats.allocations.peak_memory, 300);

        shutdown();
    }

    #[test]
    fn test_type_statistics() {
        initialize();

        // Multiple allocations of same type
        for _ in 0..5 {
            record_allocation(50, "test::Array");
        }

        for _ in 0..3 {
            record_deallocation(50, "test::Array");
        }

        let stats = get_stats().unwrap();
        let array_stats = &stats.type_stats["test::Array"];
        assert_eq!(array_stats.allocations, 5);
        assert_eq!(array_stats.deallocations, 3);
        assert_eq!(array_stats.current_bytes, 100); // 2 * 50
        assert_eq!(array_stats.peak_bytes, 250); // 5 * 50

        shutdown();
    }

    #[test]
    fn test_gc_stats() {
        initialize();

        record_gc_collection(10, Duration::from_millis(5));
        record_gc_collection(15, Duration::from_millis(7));

        let stats = get_stats().unwrap();
        assert_eq!(stats.gc_collections, 2);
        assert_eq!(stats.gc_objects_collected, 25);
        assert_eq!(stats.gc_total_time, Duration::from_millis(12));

        shutdown();
    }

    #[test]
    fn test_leak_detection() {
        initialize();

        // Allocate without deallocating
        record_allocation(1024, "test::LeakyType");

        assert!(check_leaks());

        let report = generate_report().unwrap();
        assert!(report.contains("Potential leaks: 1"));

        shutdown();
    }
}
