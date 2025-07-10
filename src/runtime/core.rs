//! Core runtime system for Script
//!
//! This module provides the main runtime structure that manages:
//! - Memory allocation and deallocation
//! - Runtime initialization and configuration
//! - Integration with the panic handler
//! - Coordination between subsystems

use std::alloc::{GlobalAlloc, Layout};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};

use crate::error::Error;
use crate::runtime::gc::CollectionStats;
use crate::runtime::panic::PanicInfo;
use crate::runtime::{Result, RuntimeError};

/// Global runtime instance
static RUNTIME: RwLock<Option<Arc<Runtime>>> = RwLock::new(None);

/// Get the global runtime instance
pub fn runtime() -> Result<Arc<Runtime>> {
    RUNTIME
        .read()
        .map_err(|_| {
            RuntimeError::InvalidOperation("Failed to acquire read lock on runtime".to_string())
        })?
        .as_ref()
        .cloned()
        .ok_or_else(|| RuntimeError::NotInitialized)
}

/// Configuration for the Script runtime
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum heap size in bytes (0 = unlimited)
    pub max_heap_size: usize,
    /// Enable memory profiling
    pub enable_profiling: bool,
    /// Enable cycle detection
    pub enable_gc: bool,
    /// GC collection threshold (allocations between collections)
    pub gc_threshold: usize,
    /// Enable panic handler
    pub enable_panic_handler: bool,
    /// Stack size for Script threads
    pub stack_size: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        RuntimeConfig {
            max_heap_size: 0, // Unlimited
            enable_profiling: cfg!(debug_assertions),
            enable_gc: true,
            gc_threshold: 1000,
            enable_panic_handler: true,
            stack_size: 2 * 1024 * 1024, // 2MB
        }
    }
}

/// The main runtime structure
pub struct Runtime {
    /// Runtime configuration
    config: RuntimeConfig,
    /// Memory manager
    memory: Arc<MemoryManager>,
    /// Type registry for dynamic dispatch
    type_registry: RwLock<TypeRegistry>,
    /// Runtime metadata
    metadata: RwLock<RuntimeMetadata>,
}

/// Memory manager for Script
pub struct MemoryManager {
    /// Current heap usage
    heap_used: AtomicUsize,
    /// Peak heap usage
    heap_peak: AtomicUsize,
    /// Total allocations
    total_allocations: AtomicUsize,
    /// Total deallocations
    total_deallocations: AtomicUsize,
    /// Configuration
    config: RuntimeConfig,
}

/// Type registry for dynamic dispatch and reflection
struct TypeRegistry {
    /// Map from TypeId to type information
    types: HashMap<TypeId, TypeInfo>,
}

/// Information about a registered type
#[allow(dead_code)]
struct TypeInfo {
    /// Type name
    name: String,
    /// Size of the type
    size: usize,
    /// Alignment of the type
    align: usize,
}

/// Runtime metadata
struct RuntimeMetadata {
    /// Script version
    #[allow(dead_code)]
    version: String,
    /// Start time
    start_time: std::time::Instant,
    /// Custom metadata
    custom: HashMap<String, String>,
}

impl Default for RuntimeMetadata {
    fn default() -> Self {
        RuntimeMetadata {
            version: env!("CARGO_PKG_VERSION").to_string(),
            start_time: std::time::Instant::now(),
            custom: HashMap::new(),
        }
    }
}

impl Runtime {
    /// Create a new runtime with the given configuration
    pub fn new(config: RuntimeConfig) -> Self {
        Runtime {
            memory: Arc::new(MemoryManager::new(config.clone())),
            config,
            type_registry: RwLock::new(TypeRegistry::new()),
            metadata: RwLock::new(RuntimeMetadata {
                version: env!("CARGO_PKG_VERSION").to_string(),
                start_time: std::time::Instant::now(),
                custom: HashMap::new(),
            }),
        }
    }

    /// Initialize the runtime with the given configuration
    pub fn initialize_with_config(config: RuntimeConfig) -> Result<()> {
        let mut runtime_lock = RUNTIME.write().map_err(|_| {
            RuntimeError::InvalidOperation("Failed to acquire write lock on runtime".to_string())
        })?;
        if runtime_lock.is_some() {
            return Err(RuntimeError::AlreadyInitialized);
        }

        let runtime = Arc::new(Runtime::new(config.clone()));
        *runtime_lock = Some(runtime.clone());

        // Initialize subsystems based on configuration
        if config.enable_profiling {
            crate::runtime::profiler::initialize();
        }

        if config.enable_gc {
            crate::runtime::gc::initialize();
        }

        if config.enable_panic_handler {
            runtime.install_panic_handler();
        }

        Ok(())
    }

    /// Get the memory manager
    pub fn memory(&self) -> &MemoryManager {
        &self.memory
    }

    /// Get runtime configuration
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    /// Register a type with the runtime
    pub fn register_type<T: Any>(&self) -> Result<()> {
        let mut registry = self.type_registry.write().map_err(|_| {
            RuntimeError::InvalidOperation(
                "Failed to acquire write lock on type registry".to_string(),
            )
        })?;
        registry.register::<T>();
        Ok(())
    }

    /// Get runtime statistics
    pub fn stats(&self) -> RuntimeStats {
        RuntimeStats {
            memory: self.memory.stats(),
            gc: crate::runtime::gc::get_stats(),
            uptime: self.uptime(),
        }
    }

    /// Get runtime uptime
    pub fn uptime(&self) -> std::time::Duration {
        let metadata = self
            .metadata
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to acquire read lock on metadata"));
        match metadata {
            Ok(data) => data.start_time.elapsed(),
            Err(_) => std::time::Duration::new(0, 0), // Return zero duration on lock failure
        }
    }

    /// Set custom metadata
    pub fn set_metadata(&self, key: String, value: String) -> Result<()> {
        let mut metadata = self.metadata.write().map_err(|_| {
            RuntimeError::InvalidOperation("Failed to acquire write lock on metadata".to_string())
        })?;
        metadata.custom.insert(key, value);
        Ok(())
    }

    /// Get custom metadata
    pub fn get_metadata(&self, key: &str) -> Option<String> {
        let metadata = self
            .metadata
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to acquire read lock on metadata"));
        match metadata {
            Ok(data) => data.custom.get(key).cloned(),
            Err(_) => None, // Return None on lock failure
        }
    }

    /// Install the panic handler
    fn install_panic_handler(&self) {
        let memory = self.memory.clone();

        panic::set_hook(Box::new(move |panic_info| {
            // Create panic info
            let info = PanicInfo {
                message: panic_info.to_string(),
                location: panic_info
                    .location()
                    .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column())),
                backtrace: std::backtrace::Backtrace::capture().to_string(),
                timestamp: std::time::Instant::now(),
                recovery_attempts: 0,
                recovered: false,
                recovery_policy: crate::runtime::panic::RecoveryPolicy::default(),
            };

            // Log panic
            eprintln!("Script panic: {}", info.message);
            if let Some(loc) = &info.location {
                eprintln!("  at {}", loc);
            }

            // Log memory stats at panic
            let stats = memory.stats();
            eprintln!("Memory at panic: {} bytes used", stats.heap_used);

            // Save panic info
            crate::runtime::panic::record_panic(info);
        }));
    }

    /// Execute a closure with panic recovery
    pub fn execute_protected<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce() -> R + std::panic::UnwindSafe,
    {
        match panic::catch_unwind(AssertUnwindSafe(f)) {
            Ok(result) => Ok(result),
            Err(_) => {
                // Get the last panic info
                if let Some(info) = crate::runtime::panic::last_panic() {
                    Err(RuntimeError::Panic(info.message))
                } else {
                    Err(RuntimeError::Panic("Unknown panic".to_string()))
                }
            }
        }
    }
}

impl MemoryManager {
    /// Create a new memory manager
    fn new(config: RuntimeConfig) -> Self {
        MemoryManager {
            heap_used: AtomicUsize::new(0),
            heap_peak: AtomicUsize::new(0),
            total_allocations: AtomicUsize::new(0),
            total_deallocations: AtomicUsize::new(0),
            config,
        }
    }

    /// Allocate memory
    pub fn allocate(&self, layout: Layout) -> Result<*mut u8> {
        let size = layout.size();

        // Check heap limit
        if self.config.max_heap_size > 0 {
            let new_used = self.heap_used.fetch_add(size, Ordering::Relaxed) + size;
            if new_used > self.config.max_heap_size {
                self.heap_used.fetch_sub(size, Ordering::Relaxed);
                return Err(RuntimeError::AllocationFailed(format!(
                    "Heap limit exceeded: {} bytes",
                    self.config.max_heap_size
                )));
            }
        } else {
            self.heap_used.fetch_add(size, Ordering::Relaxed);
        }

        // Update peak usage
        let current = self.heap_used.load(Ordering::Relaxed);
        let mut peak = self.heap_peak.load(Ordering::Relaxed);
        while current > peak {
            match self.heap_peak.compare_exchange_weak(
                peak,
                current,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(p) => peak = p,
            }
        }

        // Increment allocation count
        self.total_allocations.fetch_add(1, Ordering::Relaxed);

        // Allocate memory
        unsafe {
            let ptr = std::alloc::alloc(layout);
            if ptr.is_null() {
                self.heap_used.fetch_sub(size, Ordering::Relaxed);
                Err(RuntimeError::AllocationFailed("Out of memory".to_string()))
            } else {
                Ok(ptr)
            }
        }
    }

    /// Deallocate memory
    pub fn deallocate(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            std::alloc::dealloc(ptr, layout);
        }

        self.heap_used.fetch_sub(layout.size(), Ordering::Relaxed);
        self.total_deallocations.fetch_add(1, Ordering::Relaxed);
    }

    /// Get memory statistics
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            heap_used: self.heap_used.load(Ordering::Relaxed),
            heap_peak: self.heap_peak.load(Ordering::Relaxed),
            total_allocations: self.total_allocations.load(Ordering::Relaxed),
            total_deallocations: self.total_deallocations.load(Ordering::Relaxed),
        }
    }
}

impl TypeRegistry {
    /// Create a new type registry
    fn new() -> Self {
        TypeRegistry {
            types: HashMap::new(),
        }
    }

    /// Register a type
    fn register<T: Any>(&mut self) {
        let type_id = TypeId::of::<T>();
        let info = TypeInfo {
            name: std::any::type_name::<T>().to_string(),
            size: std::mem::size_of::<T>(),
            align: std::mem::align_of::<T>(),
        };
        self.types.insert(type_id, info);
    }

    /// Get type information
    #[allow(dead_code)]
    fn get_type_info(&self, type_id: TypeId) -> Option<&TypeInfo> {
        self.types.get(&type_id)
    }
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Current heap usage in bytes
    pub heap_used: usize,
    /// Peak heap usage in bytes
    pub heap_peak: usize,
    /// Total number of allocations
    pub total_allocations: usize,
    /// Total number of deallocations
    pub total_deallocations: usize,
}

/// Runtime statistics
#[derive(Debug, Clone)]
pub struct RuntimeStats {
    /// Memory statistics
    pub memory: MemoryStats,
    /// GC statistics
    pub gc: Option<CollectionStats>,
    /// Runtime uptime
    pub uptime: std::time::Duration,
}

/// Script memory allocator
///
/// This allocator integrates with the runtime's memory manager
pub struct ScriptAllocator;

unsafe impl GlobalAlloc for ScriptAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if let Ok(runtime) = runtime() {
            match runtime.memory().allocate(layout) {
                Ok(ptr) => ptr,
                Err(_) => std::ptr::null_mut(),
            }
        } else {
            // Fallback to system allocator if runtime not initialized
            std::alloc::alloc(layout)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if let Ok(runtime) = runtime() {
            runtime.memory().deallocate(ptr, layout);
        } else {
            // Fallback to system allocator
            std::alloc::dealloc(ptr, layout);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_initialization() {
        // Clean up any existing runtime
        let _ = crate::runtime::shutdown();

        // Initialize with default config
        let config = RuntimeConfig::default();
        assert!(Runtime::initialize_with_config(config).is_ok());

        // Should fail on second initialization
        assert!(Runtime::initialize_with_config(RuntimeConfig::default()).is_err());

        // Get runtime instance
        let runtime = runtime().unwrap();
        assert!(runtime.config().enable_gc);

        // Cleanup
        crate::runtime::shutdown().unwrap();
    }

    #[test]
    fn test_memory_allocation() {
        let _ = crate::runtime::shutdown();
        Runtime::initialize_with_config(RuntimeConfig::default()).unwrap();

        let runtime = runtime().unwrap();
        let layout = Layout::new::<i32>();

        // Allocate memory
        let ptr = runtime.memory().allocate(layout).unwrap();
        assert!(!ptr.is_null());

        // Check stats
        let stats = runtime.memory().stats();
        assert_eq!(stats.heap_used, 4);
        assert_eq!(stats.total_allocations, 1);

        // Deallocate
        runtime.memory().deallocate(ptr, layout);

        // Check stats again
        let stats = runtime.memory().stats();
        assert_eq!(stats.heap_used, 0);
        assert_eq!(stats.total_deallocations, 1);

        crate::runtime::shutdown().unwrap();
    }

    #[test]
    fn test_heap_limit() {
        let _ = crate::runtime::shutdown();

        let mut config = RuntimeConfig::default();
        config.max_heap_size = 1024; // 1KB limit
        Runtime::initialize_with_config(config).unwrap();

        let runtime = runtime().unwrap();

        // Try to allocate more than the limit
        let layout = Layout::from_size_align(2048, 8).unwrap();
        assert!(runtime.memory().allocate(layout).is_err());

        crate::runtime::shutdown().unwrap();
    }

    #[test]
    fn test_protected_execution() {
        let _ = crate::runtime::shutdown();
        Runtime::initialize_with_config(RuntimeConfig::default()).unwrap();

        let runtime = runtime().unwrap();

        // Successful execution
        let result = runtime.execute_protected(|| 42);
        assert_eq!(result.unwrap(), 42);

        // Panicking execution
        let result = runtime.execute_protected(|| {
            panic!("Test panic");
        });
        assert!(result.is_err());

        crate::runtime::shutdown().unwrap();
    }

    #[test]
    fn test_metadata() {
        let _ = crate::runtime::shutdown();
        Runtime::initialize_with_config(RuntimeConfig::default()).unwrap();

        let runtime = runtime().unwrap();

        // Set and get metadata
        runtime.set_metadata("test_key".to_string(), "test_value".to_string());
        assert_eq!(
            runtime.get_metadata("test_key"),
            Some("test_value".to_string())
        );
        assert_eq!(runtime.get_metadata("missing"), None);

        crate::runtime::shutdown().unwrap();
    }
}
