//! Resource monitoring and limits for module operations
//! 
//! This module provides resource limiting and monitoring to prevent
//! denial of service attacks through resource exhaustion.

use crate::module::{ModulePath, ModuleError, ModuleResult};
use std::collections::HashMap;
use std::sync::{Arc, RwLock, Mutex};
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicUsize, AtomicU64, Ordering};

/// Resource monitor for module operations
#[derive(Debug)]
pub struct ResourceMonitor {
    /// Resource limits configuration
    limits: ResourceLimits,
    /// Current resource usage
    usage: Arc<ResourceUsage>,
    /// Per-module resource tracking
    module_usage: Arc<RwLock<HashMap<ModulePath, ModuleResourceUsage>>>,
    /// Compilation timeout enforcer
    timeout_enforcer: Arc<Mutex<TimeoutEnforcer>>,
}

/// Resource limits configuration
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum number of modules that can be loaded
    pub max_modules: usize,
    /// Maximum dependency graph depth
    pub max_dependency_depth: usize,
    /// Maximum size for a single module (bytes)
    pub max_module_size: usize,
    /// Maximum total memory for all modules (bytes)
    pub max_total_memory: usize,
    /// Compilation timeout for a single module
    pub module_timeout: Duration,
    /// Total compilation timeout
    pub total_timeout: Duration,
    /// Maximum number of concurrent operations
    pub max_concurrent_ops: usize,
    /// Maximum number of imports per module
    pub max_imports_per_module: usize,
    /// Maximum circular dependency chain check iterations
    pub max_cycle_check_iterations: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        ResourceLimits {
            max_modules: 1000,
            max_dependency_depth: 100,
            max_module_size: 10_000_000,      // 10 MB
            max_total_memory: 1_000_000_000,  // 1 GB
            module_timeout: Duration::from_secs(30),
            total_timeout: Duration::from_secs(300),
            max_concurrent_ops: 10,
            max_imports_per_module: 100,
            max_cycle_check_iterations: 10_000,
        }
    }
}

/// Current resource usage tracking
#[derive(Debug, Default)]
pub struct ResourceUsage {
    /// Number of loaded modules
    loaded_modules: AtomicUsize,
    /// Current dependency depth
    current_depth: AtomicUsize,
    /// Total memory used (bytes)
    memory_used: AtomicU64,
    /// Number of concurrent operations
    concurrent_ops: AtomicUsize,
    /// Cycle check iterations performed
    cycle_check_iterations: AtomicUsize,
}

/// Per-module resource usage
#[derive(Debug, Clone)]
pub struct ModuleResourceUsage {
    /// Module identifier
    pub module: ModulePath,
    /// Module size in bytes
    pub size: usize,
    /// Number of imports
    pub import_count: usize,
    /// Compilation time
    pub compilation_time: Duration,
    /// Memory allocated
    pub memory_allocated: usize,
    /// Load timestamp
    pub loaded_at: Instant,
}

/// Timeout enforcement for operations
#[derive(Debug)]
struct TimeoutEnforcer {
    /// Active operations with their deadlines
    operations: HashMap<String, Instant>,
    /// Global compilation start time
    compilation_start: Option<Instant>,
}

impl ResourceMonitor {
    /// Create a new resource monitor with default limits
    pub fn new() -> Self {
        Self::with_limits(ResourceLimits::default())
    }
    
    /// Create a new resource monitor with custom limits
    pub fn with_limits(limits: ResourceLimits) -> Self {
        ResourceMonitor {
            limits,
            usage: Arc::new(ResourceUsage::default()),
            module_usage: Arc::new(RwLock::new(HashMap::new())),
            timeout_enforcer: Arc::new(Mutex::new(TimeoutEnforcer {
                operations: HashMap::new(),
                compilation_start: None,
            })),
        }
    }
    
    /// Check if a module can be loaded given current resources
    pub fn check_module_load(&self, module_path: &ModulePath, size: usize) -> ModuleResult<()> {
        // Check module count limit
        let current_modules = self.usage.loaded_modules.load(Ordering::Relaxed);
        if current_modules >= self.limits.max_modules {
            return Err(ModuleError::resource_exhausted(
                format!("Module limit exceeded: {} >= {}", current_modules, self.limits.max_modules)
            ));
        }
        
        // Check module size limit
        if size > self.limits.max_module_size {
            return Err(ModuleError::resource_exhausted(
                format!("Module too large: {} bytes (max: {})", size, self.limits.max_module_size)
            ));
        }
        
        // Check total memory limit
        let current_memory = self.usage.memory_used.load(Ordering::Relaxed) as usize;
        if current_memory + size > self.limits.max_total_memory {
            return Err(ModuleError::resource_exhausted(
                format!("Memory limit would be exceeded: {} + {} > {}", 
                    current_memory, size, self.limits.max_total_memory)
            ));
        }
        
        Ok(())
    }
    
    /// Record module load
    pub fn record_module_load(
        &self,
        module_path: ModulePath,
        size: usize,
        import_count: usize,
    ) -> ModuleResult<()> {
        // Increment counters
        self.usage.loaded_modules.fetch_add(1, Ordering::Relaxed);
        self.usage.memory_used.fetch_add(size as u64, Ordering::Relaxed);
        
        // Record per-module usage
        let mut module_usage = self.module_usage.write().unwrap();
        module_usage.insert(module_path.clone(), ModuleResourceUsage {
            module: module_path,
            size,
            import_count,
            compilation_time: Duration::default(),
            memory_allocated: size,
            loaded_at: Instant::now(),
        });
        
        Ok(())
    }
    
    /// Record module unload
    pub fn record_module_unload(&self, module_path: &ModulePath) -> ModuleResult<()> {
        let mut module_usage = self.module_usage.write().unwrap();
        
        if let Some(usage) = module_usage.remove(module_path) {
            // Decrement counters
            self.usage.loaded_modules.fetch_sub(1, Ordering::Relaxed);
            self.usage.memory_used.fetch_sub(usage.size as u64, Ordering::Relaxed);
        }
        
        Ok(())
    }
    
    /// Check dependency depth limit
    pub fn check_dependency_depth(&self, current_depth: usize) -> ModuleResult<()> {
        if current_depth > self.limits.max_dependency_depth {
            return Err(ModuleError::resource_exhausted(
                format!("Dependency depth limit exceeded: {} > {}", 
                    current_depth, self.limits.max_dependency_depth)
            ));
        }
        
        // Update current depth if higher
        let mut current_max = self.usage.current_depth.load(Ordering::Relaxed);
        while current_depth > current_max {
            match self.usage.current_depth.compare_exchange_weak(
                current_max,
                current_depth,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }
        
        Ok(())
    }
    
    /// Check import count for a module
    pub fn check_import_count(&self, import_count: usize) -> ModuleResult<()> {
        if import_count > self.limits.max_imports_per_module {
            return Err(ModuleError::resource_exhausted(
                format!("Too many imports: {} > {}", 
                    import_count, self.limits.max_imports_per_module)
            ));
        }
        Ok(())
    }
    
    /// Begin a timed operation
    pub fn begin_operation(&self, operation_id: String) -> ModuleResult<OperationGuard> {
        // Check concurrent operations limit
        let current_ops = self.usage.concurrent_ops.fetch_add(1, Ordering::Relaxed);
        if current_ops >= self.limits.max_concurrent_ops {
            self.usage.concurrent_ops.fetch_sub(1, Ordering::Relaxed);
            return Err(ModuleError::resource_exhausted(
                format!("Concurrent operation limit exceeded: {} >= {}", 
                    current_ops, self.limits.max_concurrent_ops)
            ));
        }
        
        // Record operation start
        let mut enforcer = self.timeout_enforcer.lock().map_err(|_| {
            ModuleError::internal("Failed to acquire lock on timeout enforcer")
        })?;
        let deadline = Instant::now() + self.limits.module_timeout;
        enforcer.operations.insert(operation_id.clone(), deadline);
        
        // Set global compilation start if not set
        if enforcer.compilation_start.is_none() {
            enforcer.compilation_start = Some(Instant::now());
        }
        
        Ok(OperationGuard {
            monitor: self,
            operation_id,
        })
    }
    
    /// Check if operation has timed out
    pub fn check_timeout(&self, operation_id: &str) -> ModuleResult<()> {
        let enforcer = self.timeout_enforcer.lock().map_err(|_| {
            ModuleError::internal("Failed to acquire lock on timeout enforcer")
        })?;
        
        // Check operation timeout
        if let Some(deadline) = enforcer.operations.get(operation_id) {
            if Instant::now() > *deadline {
                return Err(ModuleError::timeout(
                    format!("Operation '{}' timed out", operation_id)
                ));
            }
        }
        
        // Check global timeout
        if let Some(start) = enforcer.compilation_start {
            if Instant::now() > start + self.limits.total_timeout {
                return Err(ModuleError::timeout(
                    "Total compilation time limit exceeded"
                ));
            }
        }
        
        Ok(())
    }
    
    /// Increment and check cycle detection iterations
    pub fn check_cycle_iterations(&self) -> ModuleResult<()> {
        let iterations = self.usage.cycle_check_iterations.fetch_add(1, Ordering::Relaxed);
        
        if iterations >= self.limits.max_cycle_check_iterations {
            return Err(ModuleError::resource_exhausted(
                format!("Cycle detection iteration limit exceeded: {} >= {}", 
                    iterations, self.limits.max_cycle_check_iterations)
            ));
        }
        
        Ok(())
    }
    
    /// Get current resource usage summary
    pub fn get_usage_summary(&self) -> ResourceUsageSummary {
        let module_usage = self.module_usage.read().unwrap_or_else(|poisoned| {
            // Use the poisoned guard's data - better than crashing
            poisoned.into_inner()
        });
        
        ResourceUsageSummary {
            loaded_modules: self.usage.loaded_modules.load(Ordering::Relaxed),
            memory_used: self.usage.memory_used.load(Ordering::Relaxed),
            concurrent_ops: self.usage.concurrent_ops.load(Ordering::Relaxed),
            max_dependency_depth: self.usage.current_depth.load(Ordering::Relaxed),
            largest_module: module_usage.values()
                .max_by_key(|u| u.size)
                .map(|u| (u.module.clone(), u.size)),
            total_imports: module_usage.values()
                .map(|u| u.import_count)
                .sum(),
        }
    }
    
    /// Reset all resource counters (for testing)
    #[cfg(test)]
    pub fn reset(&self) {
        self.usage.loaded_modules.store(0, Ordering::Relaxed);
        self.usage.current_depth.store(0, Ordering::Relaxed);
        self.usage.memory_used.store(0, Ordering::Relaxed);
        self.usage.concurrent_ops.store(0, Ordering::Relaxed);
        self.usage.cycle_check_iterations.store(0, Ordering::Relaxed);
        if let Ok(mut usage) = self.module_usage.write() {
            usage.clear();
        }
        if let Ok(mut enforcer) = self.timeout_enforcer.lock() {
            enforcer.operations.clear();
            enforcer.compilation_start = None;
        }
    }
}

/// Guard for automatic operation cleanup
pub struct OperationGuard<'a> {
    monitor: &'a ResourceMonitor,
    operation_id: String,
}

impl<'a> Drop for OperationGuard<'a> {
    fn drop(&mut self) {
        // Decrement concurrent operations
        self.monitor.usage.concurrent_ops.fetch_sub(1, Ordering::Relaxed);
        
        // Remove operation from timeout tracking
        if let Ok(mut enforcer) = self.monitor.timeout_enforcer.lock() {
            enforcer.operations.remove(&self.operation_id);
        }
    }
}

/// Resource usage summary
#[derive(Debug)]
pub struct ResourceUsageSummary {
    pub loaded_modules: usize,
    pub memory_used: u64,
    pub concurrent_ops: usize,
    pub max_dependency_depth: usize,
    pub largest_module: Option<(ModulePath, usize)>,
    pub total_imports: usize,
}

/// Resource exhaustion error extension
impl ModuleError {
    pub fn resource_exhausted(message: impl Into<String>) -> Self {
        ModuleError::runtime_error(format!("Resource exhausted: {}", message.into()))
    }
    
    pub fn timeout(message: impl Into<String>) -> Self {
        ModuleError::runtime_error(format!("Operation timeout: {}", message.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_module_count_limit() {
        let limits = ResourceLimits {
            max_modules: 2,
            ..Default::default()
        };
        
        let monitor = ResourceMonitor::with_limits(limits);
        
        let module1 = ModulePath::from_string("test.module1").unwrap();
        let module2 = ModulePath::from_string("test.module2").unwrap();
        let module3 = ModulePath::from_string("test.module3").unwrap();
        
        // First two should succeed
        assert!(monitor.check_module_load(&module1, 1000).is_ok());
        monitor.record_module_load(module1.clone(), 1000, 5).unwrap();
        
        assert!(monitor.check_module_load(&module2, 1000).is_ok());
        monitor.record_module_load(module2.clone(), 1000, 5).unwrap();
        
        // Third should fail
        assert!(monitor.check_module_load(&module3, 1000).is_err());
    }
    
    #[test]
    fn test_memory_limit() {
        let limits = ResourceLimits {
            max_module_size: 1000,
            max_total_memory: 2000,
            ..Default::default()
        };
        
        let monitor = ResourceMonitor::with_limits(limits);
        
        let module1 = ModulePath::from_string("test.module1").unwrap();
        let module2 = ModulePath::from_string("test.module2").unwrap();
        
        // Module too large
        assert!(monitor.check_module_load(&module1, 1500).is_err());
        
        // Within size limit
        assert!(monitor.check_module_load(&module1, 800).is_ok());
        monitor.record_module_load(module1.clone(), 800, 5).unwrap();
        
        // Would exceed total memory
        assert!(monitor.check_module_load(&module2, 1500).is_err());
        
        // Within remaining memory
        assert!(monitor.check_module_load(&module2, 1000).is_ok());
    }
    
    #[test]
    fn test_dependency_depth_limit() {
        let limits = ResourceLimits {
            max_dependency_depth: 5,
            ..Default::default()
        };
        
        let monitor = ResourceMonitor::with_limits(limits);
        
        // Within limit
        assert!(monitor.check_dependency_depth(3).is_ok());
        assert!(monitor.check_dependency_depth(5).is_ok());
        
        // Exceeds limit
        assert!(monitor.check_dependency_depth(6).is_err());
    }
    
    #[test]
    fn test_concurrent_operations() {
        let limits = ResourceLimits {
            max_concurrent_ops: 2,
            ..Default::default()
        };
        
        let monitor = ResourceMonitor::with_limits(limits);
        
        // Start two operations
        let _op1 = monitor.begin_operation("op1".to_string()).unwrap();
        let _op2 = monitor.begin_operation("op2".to_string()).unwrap();
        
        // Third should fail
        assert!(monitor.begin_operation("op3".to_string()).is_err());
        
        // Drop one operation
        drop(_op1);
        
        // Now third should succeed
        let _op3 = monitor.begin_operation("op3".to_string()).unwrap();
    }
}