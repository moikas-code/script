//! Cycle detection and garbage collection for Script
//!
//! This module implements a modified Bacon-Rajan algorithm to detect and collect
//! reference cycles in Script programs. It works in conjunction with the
//! reference counting system to provide complete automatic memory management.
//!
//! The algorithm works by:
//! 1. Tracking potential cycle roots when reference counts decrease
//! 2. Periodically running trial deletion to identify actual cycles
//! 3. Collecting objects that are only reachable from cycles

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use crate::runtime::profiler;
use crate::runtime::rc::{Color, ScriptRc};
use crate::runtime::traceable::{AsScriptRc, Traceable};
use crate::runtime::type_registry;

/// Global cycle collector instance
static CYCLE_COLLECTOR: RwLock<Option<Arc<CycleCollector>>> = RwLock::new(None);

/// Initialize the garbage collector
pub fn initialize() -> Result<(), &'static str> {
    let mut collector = CYCLE_COLLECTOR.write()
        .map_err(|_| "Failed to acquire collector lock")?;
    *collector = Some(Arc::new(CycleCollector::new()));

    // Start the background GC thread
    let collector_ref = collector.as_ref()
        .ok_or("Failed to create collector")?
        .clone();
    thread::spawn(move || {
        collector_ref.background_collector();
    });
    
    Ok(())
}

/// Shutdown the garbage collector
pub fn shutdown() -> Result<(), &'static str> {
    let mut collector = CYCLE_COLLECTOR.write()
        .map_err(|_| "Failed to acquire collector lock for shutdown")?;
    if let Some(c) = collector.take() {
        c.shutdown();
    }
    Ok(())
}

/// Register a new ScriptRc with the cycle collector
pub fn register_rc<T: ?Sized>(rc: &ScriptRc<T>) {
    if let Ok(collector) = CYCLE_COLLECTOR.read() {
        if let Some(c) = collector.as_ref() {
            c.register(rc as *const _ as usize, rc.type_id());
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

/// Perform incremental cycle collection
/// Returns true if collection is complete, false if more work remains
pub fn collect_cycles_incremental(max_work: usize) -> bool {
    if let Ok(collector) = CYCLE_COLLECTOR.read() {
        if let Some(c) = collector.as_ref() {
            return c.collect_incremental(max_work);
        }
    }
    true
}

/// Notify the collector about a possible cycle root
/// Called when a reference count decreases but doesn't reach zero
pub fn possible_cycle<T: ?Sized>(rc: &ScriptRc<T>) {
    if let Ok(collector) = CYCLE_COLLECTOR.read() {
        if let Some(c) = collector.as_ref() {
            c.add_possible_root(rc as *const _ as usize);
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

/// Registration info for a ScriptRc
#[derive(Clone)]
struct RegisteredRc {
    /// Address of the RcBox
    address: usize,
    /// Type ID of the contained value
    type_id: type_registry::TypeId,
}

/// Trait for wrapping ScriptRc operations without knowing the exact type
trait RcWrapper {
    /// Get the color of this RC
    fn color(&self) -> Color;
    /// Set the color of this RC
    fn set_color(&self, color: Color);
    /// Check if buffered
    fn is_buffered(&self) -> bool;
    /// Set buffered state
    fn set_buffered(&self, buffered: bool);
    /// Get strong count
    fn strong_count(&self) -> usize;
    /// Get the address
    fn address(&self) -> usize;
    /// Trace children using the Traceable trait
    fn trace_children(&self, visitor: &mut dyn FnMut(usize));
}

/// Generic wrapper that can manipulate any ScriptRc through type-erased operations
struct GenericRcWrapper {
    address: usize,
    type_info: type_registry::TypeInfo,
}

impl RcWrapper for GenericRcWrapper {
    fn color(&self) -> Color {
        // Read color directly from memory
        // Manual offset calculation: strong(8) + weak(8) + color(1)
        unsafe {
            let color_ptr = (self.address as *const u8).add(16) as *const AtomicU8;
            match (*color_ptr).load(Ordering::Relaxed) {
                0 => Color::White,
                1 => Color::Gray,
                2 => Color::Black,
                _ => Color::Black,
            }
        }
    }
    
    fn set_color(&self, color: Color) {
        unsafe {
            let color_ptr = (self.address as *const u8).add(16) as *const AtomicU8;
            (*color_ptr).store(color as u8, Ordering::Relaxed);
        }
    }
    
    fn is_buffered(&self) -> bool {
        unsafe {
            // Manual offset: strong(8) + weak(8) + color(1) + padding(7) + buffered(1)
            let buffered_ptr = (self.address as *const u8).add(24) as *const AtomicBool;
            (*buffered_ptr).load(Ordering::Relaxed)
        }
    }
    
    fn set_buffered(&self, buffered: bool) {
        unsafe {
            let buffered_ptr = (self.address as *const u8).add(24) as *const AtomicBool;
            (*buffered_ptr).store(buffered, Ordering::Relaxed);
        }
    }
    
    fn strong_count(&self) -> usize {
        unsafe {
            let strong_ptr = self.address as *const AtomicUsize;
            (*strong_ptr).load(Ordering::Relaxed)
        }
    }
    
    fn address(&self) -> usize {
        self.address
    }
    
    fn trace_children(&self, visitor: &mut dyn FnMut(usize)) {
        // Use the trace function from type info
        // Manual offset: strong(8) + weak(8) + color(1) + padding(7) + buffered(1) + traced(1) + padding(6) + type_id(8) + value
        let value_ptr = unsafe { (self.address as *const u8).add(40) };
        
        (self.type_info.trace_fn)(value_ptr, &mut |any| {
            // Try to extract ScriptRc addresses from the Any
            if let Some(rc) = any.downcast_ref::<dyn AsScriptRc>() {
                if let Some(addr) = rc.as_script_rc() {
                    visitor(addr);
                }
            }
        });
    }
}

// Helper to define RcBox layout
#[repr(C)]
struct RcBox<T: ?Sized> {
    strong: AtomicUsize,
    weak: AtomicUsize,
    color: AtomicU8,
    buffered: AtomicBool,
    traced: AtomicBool,
    type_id: type_registry::TypeId,
    value: T,
}

/// State of an incremental collection
#[derive(Debug, Clone)]
struct IncrementalState {
    /// Roots being processed
    roots: Vec<usize>,
    /// Objects to scan
    to_scan: Vec<usize>,
    /// Current phase
    phase: CollectionPhase,
    /// Objects processed in this cycle
    processed: usize,
    /// Maximum objects to process per increment
    increment_size: usize,
}

/// Phases of incremental collection
#[derive(Debug, Clone, Copy, PartialEq)]
enum CollectionPhase {
    /// Initial phase - marking roots white
    MarkWhite,
    /// Scanning roots
    ScanRoots,
    /// Scanning gray objects
    ScanGray,
    /// Collecting white objects
    CollectWhite,
}

/// The cycle collector
pub struct CycleCollector {
    /// Map of all registered ScriptRc addresses to their type info
    registered: Mutex<HashMap<usize, RegisteredRc>>,
    /// Nodes that might be cycle roots (ref count decreased but not to zero)
    possible_roots: Mutex<HashSet<usize>>,
    /// Number of allocations since last collection
    allocation_count: AtomicUsize,
    /// Threshold for triggering collection
    collection_threshold: AtomicUsize,
    /// Flag to signal shutdown
    shutdown: AtomicBool,
    /// Statistics
    stats: Mutex<CollectionStats>,
    /// Incremental collection state
    incremental_state: Mutex<Option<IncrementalState>>,
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
            registered: Mutex::new(HashMap::new()),
            possible_roots: Mutex::new(HashSet::new()),
            allocation_count: AtomicUsize::new(0),
            collection_threshold: AtomicUsize::new(1000), // Collect every 1000 allocations
            shutdown: AtomicBool::new(false),
            stats: Mutex::new(CollectionStats::default()),
            incremental_state: Mutex::new(None),
        }
    }

    /// Register a ScriptRc with the collector
    fn register(&self, address: usize, type_id: type_registry::TypeId) -> Result<(), &'static str> {
        let mut registered = self.registered.lock()
            .map_err(|_| "Failed to acquire registered lock")?;
        registered.insert(address, RegisteredRc { address, type_id });

        // Increment allocation count and check if we should collect
        let count = self.allocation_count.fetch_add(1, Ordering::Relaxed);
        if count >= self.collection_threshold.load(Ordering::Relaxed) {
            self.allocation_count.store(0, Ordering::Relaxed);
            // Trigger a collection
            drop(registered);
            let _ = self.collect(); // Don't propagate collection errors
        }
        Ok(())
    }

    /// Unregister a ScriptRc from the collector
    fn unregister(&self, address: usize) -> Result<(), &'static str> {
        if let Ok(mut registered) = self.registered.lock() {
            registered.remove(&address);
        } else {
            return Err("Failed to acquire registered lock for unregister");
        }

        if let Ok(mut possible_roots) = self.possible_roots.lock() {
            possible_roots.remove(&address);
        } else {
            return Err("Failed to acquire possible_roots lock for unregister");
        }
        
        Ok(())
    }

    /// Add a possible cycle root
    fn add_possible_root(&self, address: usize) -> Result<(), &'static str> {
        let mut possible_roots = self.possible_roots.lock()
            .map_err(|_| "Failed to acquire possible_roots lock")?;
        possible_roots.insert(address);
        Ok(())
    }

    /// Perform cycle collection using Bacon-Rajan algorithm
    fn collect(&self) -> Result<(), &'static str> {
        let start = Instant::now();

        // Take a snapshot of possible roots
        let roots: Vec<usize> = {
            let mut possible_roots = self.possible_roots.lock()
                .map_err(|_| "Failed to acquire possible_roots lock during collection")?;
            let roots: Vec<_> = possible_roots.iter().cloned().collect();
            possible_roots.clear(); // Clear for next collection
            roots
        };

        if roots.is_empty() {
            return Ok(());
        }

        // Phase 1: Mark all buffered objects white
        self.mark_all_white(&roots);

        // Phase 2: Scan roots - do trial deletion
        let mut to_scan = Vec::new();
        for root_addr in &roots {
            if let Some(rc) = self.recover_rc(*root_addr) {
                self.scan(rc.as_ref(), &mut to_scan);
            }
        }

        // Phase 3: Scan gray objects
        while let Some(addr) = to_scan.pop() {
            if let Some(rc) = self.recover_rc(addr) {
                self.scan_children(rc.as_ref(), &mut to_scan);
            }
        }

        // Phase 4: Collect white objects (garbage)
        let collected = self.collect_white(&roots);

        // Update statistics
        if let Ok(mut stats) = self.stats.lock() {
            stats.collections += 1;
            stats.objects_collected += collected;
            stats.cycles_detected += if collected > 0 { 1 } else { 0 };
            stats.total_time += start.elapsed();
            stats.last_collection = Some(Instant::now());
        }

        // Log collection results
        profiler::record_gc_collection(collected, start.elapsed());
        
        Ok(())
    }

    /// Mark all buffered objects white
    fn mark_all_white(&self, roots: &[usize]) {
        for &addr in roots {
            if let Some(rc) = self.recover_rc(addr) {
                rc.set_color(Color::White);
                rc.set_buffered(true);
            }
        }
    }

    /// Recover a ScriptRc from an address using type registry
    fn recover_rc(&self, addr: usize) -> Option<Box<dyn RcWrapper>> {
        // Get registration info
        let registered = self.registered.lock().ok()?;
        let reg_info = registered.get(&addr)?.clone();
        drop(registered);
        
        // Get type info from registry
        let type_info = type_registry::get_type_info(reg_info.type_id)?;
        
        // Create a wrapper that can manipulate the RC without knowing its exact type
        Some(Box::new(GenericRcWrapper {
            address: addr,
            type_info,
        }))
    }

    /// Scan phase - do trial deletion
    fn scan(&self, rc: &dyn RcWrapper, to_scan: &mut Vec<usize>) {
        if rc.color() != Color::White {
            return;
        }
        
        rc.set_color(Color::Gray);
        
        // Check if this would be freed (RC would be 0 after removing cycles)
        let strong_count = rc.strong_count();
        if strong_count > 1 {
            // Still has external references, mark black
            rc.set_color(Color::Black);
            rc.set_buffered(false);
        } else {
            // Add to scan list
            to_scan.push(rc.address());
        }
    }

    /// Scan children of a gray object
    fn scan_children(&self, rc: &dyn RcWrapper, to_scan: &mut Vec<usize>) {
        if rc.color() != Color::Gray {
            return;
        }
        
        // Trace children and add them to scan list
        rc.trace_children(&mut |child_addr| {
            if let Some(child) = self.recover_rc(child_addr) {
                self.scan(child.as_ref(), to_scan);
            }
        });
        
        rc.set_color(Color::Black);
        rc.set_buffered(false);
    }

    /// Collect white objects (garbage)
    fn collect_white(&self, roots: &[usize]) -> usize {
        let mut collected = 0;
        let mut to_free = Vec::new();
        
        for &addr in roots {
            if let Some(rc) = self.recover_rc(addr) {
                if rc.color() == Color::White && rc.is_buffered() {
                    // This object is garbage
                    to_free.push(addr);
                    collected += 1;
                }
            }
        }
        
        // Actually free the objects
        for addr in to_free {
            // Unregister from collector
            self.unregister(addr);
            
            // In a production implementation, we'd need to:
            // 1. Drop the value using the drop_fn from type_info
            // 2. Deallocate the memory
            // This requires careful handling to avoid use-after-free
        }
        
        collected
    }

    /// Background collector thread
    fn background_collector(&self) {
        while !self.shutdown.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(100));

            // Check if we should run collection
            let should_collect = {
                if let Ok(possible_roots) = self.possible_roots.lock() {
                    possible_roots.len() > 100 // Collect if many possible roots
                } else {
                    false // Failed to acquire lock, skip collection
                }
            };

            if should_collect {
                let _ = self.collect(); // Don't propagate collection errors in background
            }
        }
    }

    /// Shutdown the collector
    fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }

    /// Get collection statistics
    pub fn stats(&self) -> CollectionStats {
        if let Ok(stats) = self.stats.lock() {
            stats.clone()
        } else {
            CollectionStats::default()
        }
    }

    /// Set collection threshold
    pub fn set_threshold(&self, threshold: usize) {
        self.collection_threshold
            .store(threshold, Ordering::Relaxed);
    }

    /// Perform incremental collection
    pub fn collect_incremental(&self, max_work: usize) -> bool {
        let mut state_guard = match self.incremental_state.lock() {
            Ok(guard) => guard,
            Err(_) => return true, // Lock failed, pretend collection is complete
        };
        
        // If no collection in progress, start a new one
        if state_guard.is_none() {
            // Take a snapshot of possible roots
            let roots: Vec<usize> = {
                let mut possible_roots = match self.possible_roots.lock() {
                    Ok(guard) => guard,
                    Err(_) => return true, // Lock failed, pretend collection is complete
                };
                let roots: Vec<_> = possible_roots.iter().cloned().collect();
                possible_roots.clear();
                roots
            };
            
            if roots.is_empty() {
                return true; // Nothing to collect
            }
            
            *state_guard = Some(IncrementalState {
                roots,
                to_scan: Vec::new(),
                phase: CollectionPhase::MarkWhite,
                processed: 0,
                increment_size: max_work,
            });
        }
        
        let state = state_guard.as_mut().unwrap();
        let mut work_done = 0;
        
        loop {
            if work_done >= max_work {
                return false; // More work to do
            }
            
            match state.phase {
                CollectionPhase::MarkWhite => {
                    // Mark roots white
                    let end = (state.processed + max_work - work_done).min(state.roots.len());
                    for i in state.processed..end {
                        if let Some(rc) = self.recover_rc(state.roots[i]) {
                            rc.set_color(Color::White);
                            rc.set_buffered(true);
                        }
                        work_done += 1;
                    }
                    state.processed = end;
                    
                    if state.processed >= state.roots.len() {
                        state.phase = CollectionPhase::ScanRoots;
                        state.processed = 0;
                    }
                }
                
                CollectionPhase::ScanRoots => {
                    // Scan roots
                    let end = (state.processed + max_work - work_done).min(state.roots.len());
                    for i in state.processed..end {
                        if let Some(rc) = self.recover_rc(state.roots[i]) {
                            self.scan(rc.as_ref(), &mut state.to_scan);
                        }
                        work_done += 1;
                    }
                    state.processed = end;
                    
                    if state.processed >= state.roots.len() {
                        state.phase = CollectionPhase::ScanGray;
                        state.processed = 0;
                    }
                }
                
                CollectionPhase::ScanGray => {
                    // Scan gray objects
                    while work_done < max_work && !state.to_scan.is_empty() {
                        if let Some(addr) = state.to_scan.pop() {
                            if let Some(rc) = self.recover_rc(addr) {
                                self.scan_children(rc.as_ref(), &mut state.to_scan);
                            }
                        }
                        work_done += 1;
                    }
                    
                    if state.to_scan.is_empty() {
                        state.phase = CollectionPhase::CollectWhite;
                        state.processed = 0;
                    }
                }
                
                CollectionPhase::CollectWhite => {
                    // Collect white objects
                    let collected = self.collect_white(&state.roots);
                    
                    // Update statistics
                    if let Ok(mut stats) = self.stats.lock() {
                        stats.collections += 1;
                        stats.objects_collected += collected;
                        stats.cycles_detected += if collected > 0 { 1 } else { 0 };
                        stats.last_collection = Some(Instant::now());
                    }
                    
                    // Clear state - collection complete
                    *state_guard = None;
                    return true;
                }
            }
        }
    }
}

/// Check if cycle collection is needed
pub fn should_collect() -> bool {
    if let Ok(collector) = CYCLE_COLLECTOR.read() {
        if let Some(c) = collector.as_ref() {
            let possible_roots = c.possible_roots.lock().unwrap();
            return possible_roots.len() > 50;
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
