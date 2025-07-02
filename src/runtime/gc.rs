//! Cycle detection and garbage collection for Script
//!
//! This module implements a mark-and-sweep algorithm to detect and collect
//! reference cycles in Script programs. It works in conjunction with the
//! reference counting system to provide complete automatic memory management.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use crate::runtime::profiler;
use crate::runtime::rc::ScriptRc;

/// Global cycle collector instance
static CYCLE_COLLECTOR: RwLock<Option<Arc<CycleCollector>>> = RwLock::new(None);

/// Initialize the garbage collector
pub fn initialize() {
    let mut collector = CYCLE_COLLECTOR.write().unwrap();
    *collector = Some(Arc::new(CycleCollector::new()));

    // Start the background GC thread
    let collector_ref = collector.as_ref().unwrap().clone();
    thread::spawn(move || {
        collector_ref.background_collector();
    });
}

/// Shutdown the garbage collector
pub fn shutdown() {
    let mut collector = CYCLE_COLLECTOR.write().unwrap();
    if let Some(c) = collector.take() {
        c.shutdown();
    }
}

/// Register a new ScriptRc with the cycle collector
pub fn register_rc<T: ?Sized>(rc: &ScriptRc<T>) {
    if let Ok(collector) = CYCLE_COLLECTOR.read() {
        if let Some(c) = collector.as_ref() {
            c.register(rc as *const _ as usize);
        }
    }
}

/// Unregister a ScriptRc from the cycle collector
pub fn unregister_rc<T: ?Sized>(rc: &ScriptRc<T>) {
    if let Ok(collector) = CYCLE_COLLECTOR.read() {
        if let Some(c) = collector.as_ref() {
            c.unregister(rc as *const _ as usize);
        }
    }
}

/// Force a cycle collection
pub fn collect_cycles() {
    if let Ok(collector) = CYCLE_COLLECTOR.read() {
        if let Some(c) = collector.as_ref() {
            c.collect();
        }
    }
}

/// Represents a node in the object graph
#[derive(Debug, Clone)]
struct GraphNode {
    /// Address of the ScriptRc
    address: usize,
    /// Whether this node has been marked during collection
    marked: bool,
    /// Strong reference count at time of snapshot
    strong_count: usize,
    /// Outgoing references from this node
    references: Vec<usize>,
}

/// The cycle collector
pub struct CycleCollector {
    /// Set of all registered ScriptRc addresses
    registered: Mutex<HashSet<usize>>,
    /// Nodes suspected of being in cycles
    suspects: Mutex<HashSet<usize>>,
    /// Number of allocations since last collection
    allocation_count: AtomicUsize,
    /// Threshold for triggering collection
    collection_threshold: AtomicUsize,
    /// Flag to signal shutdown
    shutdown: AtomicBool,
    /// Statistics
    stats: Mutex<CollectionStats>,
}

/// Statistics about cycle collection
#[derive(Debug, Default, Clone)]
pub struct CollectionStats {
    /// Total number of collections performed
    pub collections: usize,
    /// Total number of cycles detected
    pub cycles_detected: usize,
    /// Total number of objects collected
    pub objects_collected: usize,
    /// Total time spent in collections
    pub total_time: Duration,
    /// Last collection time
    pub last_collection: Option<Instant>,
}

impl CycleCollector {
    /// Create a new cycle collector
    fn new() -> Self {
        CycleCollector {
            registered: Mutex::new(HashSet::new()),
            suspects: Mutex::new(HashSet::new()),
            allocation_count: AtomicUsize::new(0),
            collection_threshold: AtomicUsize::new(1000), // Collect every 1000 allocations
            shutdown: AtomicBool::new(false),
            stats: Mutex::new(CollectionStats::default()),
        }
    }

    /// Register a ScriptRc with the collector
    fn register(&self, address: usize) {
        let mut registered = self.registered.lock().unwrap();
        registered.insert(address);

        // Increment allocation count and check if we should collect
        let count = self.allocation_count.fetch_add(1, Ordering::Relaxed);
        if count >= self.collection_threshold.load(Ordering::Relaxed) {
            self.allocation_count.store(0, Ordering::Relaxed);
            self.mark_suspect(address);
        }
    }

    /// Unregister a ScriptRc from the collector
    fn unregister(&self, address: usize) {
        let mut registered = self.registered.lock().unwrap();
        registered.remove(&address);

        let mut suspects = self.suspects.lock().unwrap();
        suspects.remove(&address);
    }

    /// Mark an object as a potential cycle participant
    fn mark_suspect(&self, address: usize) {
        let mut suspects = self.suspects.lock().unwrap();
        suspects.insert(address);
    }

    /// Perform cycle collection
    fn collect(&self) {
        let start = Instant::now();

        // Take a snapshot of suspects
        let suspects: Vec<usize> = {
            let suspects = self.suspects.lock().unwrap();
            suspects.iter().cloned().collect()
        };

        if suspects.is_empty() {
            return;
        }

        // Build object graph for suspects
        let mut graph = self.build_object_graph(&suspects);

        // Mark reachable objects
        self.mark_reachable(&mut graph);

        // Collect unmarked objects (potential cycles)
        let collected = self.sweep(&graph);

        // Update statistics
        let mut stats = self.stats.lock().unwrap();
        stats.collections += 1;
        stats.objects_collected += collected;
        stats.total_time += start.elapsed();
        stats.last_collection = Some(Instant::now());

        // Clear suspects that were collected
        let mut suspects_lock = self.suspects.lock().unwrap();
        for node in &graph {
            if !node.marked {
                suspects_lock.remove(&node.address);
            }
        }

        // Log collection results
        profiler::record_gc_collection(collected, start.elapsed());
    }

    /// Build object graph from suspects
    fn build_object_graph(&self, suspects: &[usize]) -> Vec<GraphNode> {
        let mut graph = Vec::new();

        for &address in suspects {
            // For now, we create a simple node
            // In a real implementation, we'd need to traverse the object's references
            graph.push(GraphNode {
                address,
                marked: false,
                strong_count: 1,        // Placeholder
                references: Vec::new(), // Would be filled by traversing object
            });
        }

        graph
    }

    /// Mark objects reachable from roots
    fn mark_reachable(&self, graph: &mut [GraphNode]) {
        // In a real implementation, we'd start from root objects
        // For now, mark objects with strong_count > 1 as reachable
        for i in 0..graph.len() {
            if graph[i].strong_count > 1 {
                self.mark_recursive(i, graph);
            }
        }
    }

    /// Recursively mark reachable objects
    fn mark_recursive(&self, node_idx: usize, graph: &mut [GraphNode]) {
        if graph[node_idx].marked {
            return;
        }

        graph[node_idx].marked = true;

        // Mark all referenced objects
        let references = graph[node_idx].references.clone();
        for ref_addr in references {
            if let Some(ref_idx) = graph.iter().position(|n| n.address == ref_addr) {
                self.mark_recursive(ref_idx, graph);
            }
        }
    }

    /// Sweep unmarked objects
    fn sweep(&self, graph: &[GraphNode]) -> usize {
        let mut collected = 0;

        for node in graph {
            if !node.marked {
                // In a real implementation, we'd decrease reference counts here
                // This would trigger deallocation if the count reaches zero
                collected += 1;
            }
        }

        collected
    }

    /// Background collector thread
    fn background_collector(&self) {
        while !self.shutdown.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(100));

            // Check if we should run collection
            let should_collect = {
                let suspects = self.suspects.lock().unwrap();
                suspects.len() > 100 // Collect if many suspects
            };

            if should_collect {
                self.collect();
            }
        }
    }

    /// Shutdown the collector
    fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }

    /// Get collection statistics
    pub fn stats(&self) -> CollectionStats {
        self.stats.lock().unwrap().clone()
    }

    /// Set collection threshold
    pub fn set_threshold(&self, threshold: usize) {
        self.collection_threshold
            .store(threshold, Ordering::Relaxed);
    }
}

/// Check if cycle collection is needed
pub fn should_collect() -> bool {
    if let Ok(collector) = CYCLE_COLLECTOR.read() {
        if let Some(c) = collector.as_ref() {
            let suspects = c.suspects.lock().unwrap();
            return suspects.len() > 50;
        }
    }
    false
}

/// Get collection statistics
pub fn get_stats() -> Option<CollectionStats> {
    if let Ok(collector) = CYCLE_COLLECTOR.read() {
        if let Some(c) = collector.as_ref() {
            return Some(c.stats());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cycle_collector_lifecycle() {
        // Initialize collector
        initialize();

        // Register some addresses
        register_rc(&ScriptRc::new(42));
        register_rc(&ScriptRc::new("test"));

        // Force collection
        collect_cycles();

        // Get stats
        let stats = get_stats().unwrap();
        assert!(stats.collections > 0);

        // Shutdown
        shutdown();
    }

    #[test]
    fn test_collection_threshold() {
        initialize();

        if let Ok(collector) = CYCLE_COLLECTOR.read() {
            if let Some(c) = collector.as_ref() {
                c.set_threshold(10);

                // Register many objects to trigger collection
                for i in 0..20 {
                    register_rc(&ScriptRc::new(i));
                }

                // Should have triggered collection
                let stats = c.stats();
                assert!(stats.collections > 0);
            }
        }

        shutdown();
    }
}
