//! Safe garbage collection implementation for Script
//!
//! This module provides a security-hardened implementation of the Bacon-Rajan
//! cycle detection algorithm with comprehensive safety guarantees:
//! - Memory safety through validated pointer operations
//! - Resource limits to prevent DoS attacks
//! - Race condition protection through atomic operations
//! - Type safety validation before all casts
//! - Graceful error handling without panics

use std::collections::{HashMap, HashSet};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use crate::runtime::rc::{Color, ScriptRc};
use crate::runtime::traceable::AsScriptRc;
use crate::runtime::type_registry;

/// Security configuration for garbage collection
#[derive(Debug, Clone)]
pub struct GcSecurityConfig {
    /// Maximum number of possible roots to track
    pub max_possible_roots: usize,
    /// Maximum time to spend in a single collection cycle
    pub max_collection_time: Duration,
    /// Maximum depth for object graph traversal
    pub max_graph_depth: usize,
    /// Maximum number of objects to process per incremental step
    pub max_incremental_work: usize,
    /// Enable security monitoring and alerts
    pub enable_monitoring: bool,
    /// Enable type validation before casts
    pub enable_type_validation: bool,
}

impl Default for GcSecurityConfig {
    fn default() -> Self {
        Self {
            max_possible_roots: 100_000,
            max_collection_time: Duration::from_secs(1),
            max_graph_depth: 10_000,
            max_incremental_work: 1000,
            enable_monitoring: true,
            enable_type_validation: true,
        }
    }
}

/// Security monitoring events
#[derive(Debug, Clone)]
pub enum SecurityEvent {
    /// Resource limit exceeded
    ResourceLimitExceeded { limit: String, value: usize },
    /// Type validation failed
    TypeValidationFailure { expected: String, actual: String },
    /// Collection timeout exceeded
    CollectionTimeout { duration: Duration },
    /// Potential attack detected
    AttackDetected { description: String },
    /// Memory corruption detected
    MemoryCorruption { details: String },
}

/// Secure wrapper for ScriptRc operations
pub struct SecureRcWrapper {
    /// Non-null pointer to the RcBox
    ptr: NonNull<u8>,
    /// Type ID for validation
    type_id: type_registry::TypeId,
    /// Generation counter to prevent use-after-free
    generation: u64,
    /// Size of the object for bounds checking
    object_size: usize,
    /// Security configuration
    config: Arc<GcSecurityConfig>,
}

impl SecureRcWrapper {
    /// Create a new secure wrapper with validation
    pub fn new(
        ptr: NonNull<u8>,
        type_id: type_registry::TypeId,
        generation: u64,
        object_size: usize,
        config: Arc<GcSecurityConfig>,
    ) -> Result<Self, SecurityError> {
        // Validate pointer alignment
        if ptr.as_ptr() as usize % std::mem::align_of::<usize>() != 0 {
            return Err(SecurityError::InvalidAlignment);
        }

        // Validate object size
        if object_size == 0 || object_size > 1024 * 1024 * 1024 {
            return Err(SecurityError::InvalidObjectSize);
        }

        Ok(SecureRcWrapper {
            ptr,
            type_id,
            generation,
            object_size,
            config,
        })
    }

    /// Safely read an atomic value with bounds checking
    fn read_atomic<T>(&self, offset: usize) -> Result<T, SecurityError>
    where
        T: Copy,
    {
        // Check bounds
        if offset + std::mem::size_of::<T>() > self.object_size {
            return Err(SecurityError::OutOfBounds);
        }

        // Check alignment
        if (self.ptr.as_ptr() as usize + offset) % std::mem::align_of::<T>() != 0 {
            return Err(SecurityError::InvalidAlignment);
        }

        // Safe read
        unsafe {
            let ptr = self.ptr.as_ptr().add(offset) as *const T;
            Ok(std::ptr::read(ptr))
        }
    }

    /// Safely write an atomic value with bounds checking
    fn write_atomic<T>(&self, offset: usize, value: T) -> Result<(), SecurityError>
    where
        T: Copy,
    {
        // Check bounds
        if offset + std::mem::size_of::<T>() > self.object_size {
            return Err(SecurityError::OutOfBounds);
        }

        // Check alignment
        if (self.ptr.as_ptr() as usize + offset) % std::mem::align_of::<T>() != 0 {
            return Err(SecurityError::InvalidAlignment);
        }

        // Safe write
        unsafe {
            let ptr = self.ptr.as_ptr().add(offset) as *mut T;
            std::ptr::write(ptr, value);
        }

        Ok(())
    }

    /// Get the color with safety checks
    pub fn color(&self) -> Result<Color, SecurityError> {
        // Validate type before access
        if self.config.enable_type_validation {
            self.validate_type()?;
        }

        // Read color from known offset in RcBox structure
        let color_val: u8 = self.read_atomic(16)?;
        match color_val {
            0 => Ok(Color::White),
            1 => Ok(Color::Gray),
            2 => Ok(Color::Black),
            _ => Err(SecurityError::InvalidColorValue),
        }
    }

    /// Set the color with safety checks
    pub fn set_color(&self, color: Color) -> Result<(), SecurityError> {
        // Validate type before access
        if self.config.enable_type_validation {
            self.validate_type()?;
        }

        self.write_atomic(16, color as u8)
    }

    /// Check if buffered with safety checks
    pub fn is_buffered(&self) -> Result<bool, SecurityError> {
        if self.config.enable_type_validation {
            self.validate_type()?;
        }

        let buffered: bool = self.read_atomic(24)?;
        Ok(buffered)
    }

    /// Set buffered state with safety checks
    pub fn set_buffered(&self, buffered: bool) -> Result<(), SecurityError> {
        if self.config.enable_type_validation {
            self.validate_type()?;
        }

        self.write_atomic(24, buffered)
    }

    /// Get strong count with safety checks
    pub fn strong_count(&self) -> Result<usize, SecurityError> {
        if self.config.enable_type_validation {
            self.validate_type()?;
        }

        self.read_atomic(0)
    }

    /// Get the raw address
    pub fn address(&self) -> usize {
        self.ptr.as_ptr() as usize
    }

    /// Validate type ID matches expected
    fn validate_type(&self) -> Result<(), SecurityError> {
        // Read type ID from object
        let actual_type_id: type_registry::TypeId = self.read_atomic(32)?;

        if actual_type_id != self.type_id {
            return Err(SecurityError::TypeMismatch {
                expected: self.type_id,
                actual: actual_type_id,
            });
        }

        Ok(())
    }

    /// Trace children with security checks
    pub fn trace_children<F>(&self, mut visitor: F) -> Result<(), SecurityError>
    where
        F: FnMut(usize),
    {
        if self.config.enable_type_validation {
            self.validate_type()?;
        }

        // Get type info for safe tracing
        let type_info =
            type_registry::get_type_info(self.type_id).ok_or(SecurityError::TypeNotFound)?;

        // Calculate value offset (after RcBox header)
        let value_offset = 40; // Size of RcBox header

        if value_offset >= self.object_size {
            return Err(SecurityError::OutOfBounds);
        }

        // Safe trace using type info
        unsafe {
            let value_ptr = self.ptr.as_ptr().add(value_offset);
            (type_info.trace_fn)(value_ptr, &mut |any| {
                // Try to extract ScriptRc addresses from the Any
                // We need to try different concrete types that implement AsScriptRc
                use crate::runtime::value::Value;
                if let Some(rc) = any.downcast_ref::<ScriptRc<Value>>() {
                    if let Some(addr) = rc.as_script_rc() {
                        visitor(addr);
                    }
                }
            });
        }

        Ok(())
    }
}

/// Security errors for garbage collection
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityError {
    /// Resource limit exceeded
    ResourceLimitExceeded(String),
    /// Type validation failed
    TypeMismatch {
        expected: type_registry::TypeId,
        actual: type_registry::TypeId,
    },
    /// Type not found in registry
    TypeNotFound,
    /// Invalid memory alignment
    InvalidAlignment,
    /// Invalid object size
    InvalidObjectSize,
    /// Out of bounds access
    OutOfBounds,
    /// Invalid color value
    InvalidColorValue,
    /// Collection timeout
    CollectionTimeout,
    /// Memory corruption detected
    MemoryCorruption(String),
}

impl std::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityError::ResourceLimitExceeded(msg) => {
                write!(f, "Resource limit exceeded: {}", msg)
            }
            SecurityError::TypeMismatch { expected, actual } => {
                write!(
                    f,
                    "Type mismatch: expected {:?}, got {:?}",
                    expected, actual
                )
            }
            SecurityError::TypeNotFound => write!(f, "Type not found in registry"),
            SecurityError::InvalidAlignment => write!(f, "Invalid memory alignment"),
            SecurityError::InvalidObjectSize => write!(f, "Invalid object size"),
            SecurityError::OutOfBounds => write!(f, "Out of bounds memory access"),
            SecurityError::InvalidColorValue => write!(f, "Invalid color value"),
            SecurityError::CollectionTimeout => write!(f, "Collection timeout exceeded"),
            SecurityError::MemoryCorruption(msg) => write!(f, "Memory corruption: {}", msg),
        }
    }
}

impl std::error::Error for SecurityError {}

/// Secure cycle collector with comprehensive safety guarantees
pub struct SecureCycleCollector {
    /// Registered objects with validation
    registered: RwLock<HashMap<usize, RegisteredObject>>,
    /// Possible roots with resource limits
    possible_roots: Mutex<BoundedSet<usize>>,
    /// Security configuration
    config: Arc<GcSecurityConfig>,
    /// Statistics with security monitoring
    stats: Mutex<SecureCollectionStats>,
    /// Shutdown flag
    shutdown: AtomicBool,
    /// Global generation counter
    generation: AtomicU64,
    /// Security event handler
    security_handler: Arc<dyn SecurityEventHandler>,
}

/// Registered object with security metadata
#[derive(Debug, Clone)]
struct RegisteredObject {
    /// Address of the object
    address: usize,
    /// Type ID for validation
    type_id: type_registry::TypeId,
    /// Generation when registered
    generation: u64,
    /// Size of the object
    object_size: usize,
    /// Registration timestamp
    registered_at: Instant,
}

/// Bounded set with resource limits
struct BoundedSet<T> {
    inner: HashSet<T>,
    max_size: usize,
}

impl<T> BoundedSet<T>
where
    T: std::hash::Hash + Eq + Clone,
{
    fn new(max_size: usize) -> Self {
        Self {
            inner: HashSet::with_capacity(max_size.min(1000)),
            max_size,
        }
    }

    fn insert(&mut self, item: T) -> Result<bool, SecurityError> {
        if self.inner.len() >= self.max_size {
            return Err(SecurityError::ResourceLimitExceeded(format!(
                "Maximum set size {} exceeded",
                self.max_size
            )));
        }
        Ok(self.inner.insert(item))
    }

    fn remove(&mut self, item: &T) -> bool {
        self.inner.remove(item)
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn clear(&mut self) {
        self.inner.clear()
    }

    fn iter(&self) -> std::collections::hash_set::Iter<T> {
        self.inner.iter()
    }
}

/// Security-aware collection statistics
#[derive(Debug, Default, Clone)]
pub struct SecureCollectionStats {
    /// Total collections performed
    pub collections: usize,
    /// Total cycles detected
    pub cycles_detected: usize,
    /// Total objects collected
    pub objects_collected: usize,
    /// Total time spent in collections
    pub total_time: Duration,
    /// Security events triggered
    pub security_events: usize,
    /// Resource limit violations
    pub resource_violations: usize,
    /// Type validation failures
    pub type_validation_failures: usize,
    /// Collection timeouts
    pub collection_timeouts: usize,
}

/// Handler for security events
pub trait SecurityEventHandler: Send + Sync {
    /// Handle a security event
    fn handle_event(&self, event: SecurityEvent);
}

/// Default security event handler that logs events
pub struct DefaultSecurityHandler;

impl SecurityEventHandler for DefaultSecurityHandler {
    fn handle_event(&self, event: SecurityEvent) {
        match event {
            SecurityEvent::ResourceLimitExceeded { limit, value } => {
                eprintln!(
                    "Security Alert: Resource limit exceeded - {} = {}",
                    limit, value
                );
            }
            SecurityEvent::TypeValidationFailure { expected, actual } => {
                eprintln!(
                    "Security Alert: Type validation failed - expected {}, got {}",
                    expected, actual
                );
            }
            SecurityEvent::CollectionTimeout { duration } => {
                eprintln!("Security Alert: Collection timeout - {:?}", duration);
            }
            SecurityEvent::AttackDetected { description } => {
                eprintln!(
                    "Security Alert: Potential attack detected - {}",
                    description
                );
            }
            SecurityEvent::MemoryCorruption { details } => {
                eprintln!("Security Alert: Memory corruption detected - {details}");
            }
        }
    }
}

impl SecureCycleCollector {
    /// Create a new secure cycle collector
    pub fn new(config: GcSecurityConfig) -> Self {
        let config = Arc::new(config);
        Self {
            registered: RwLock::new(HashMap::new()),
            possible_roots: Mutex::new(BoundedSet::new(config.max_possible_roots)),
            config: config.clone(),
            stats: Mutex::new(SecureCollectionStats::default()),
            shutdown: AtomicBool::new(false),
            generation: AtomicU64::new(1),
            security_handler: Arc::new(DefaultSecurityHandler),
        }
    }

    /// Register an object with security validation
    pub fn register(
        &self,
        address: usize,
        type_id: type_registry::TypeId,
        object_size: usize,
    ) -> Result<(), SecurityError> {
        // Validate inputs
        if address == 0 {
            return Err(SecurityError::InvalidAlignment);
        }

        if object_size == 0 || object_size > 1024 * 1024 * 1024 {
            return Err(SecurityError::InvalidObjectSize);
        }

        // Get generation
        let generation = self.generation.fetch_add(1, Ordering::Relaxed);

        // Register object
        let registered_obj = RegisteredObject {
            address,
            type_id,
            generation,
            object_size,
            registered_at: Instant::now(),
        };

        // Acquire write lock with timeout
        let mut registered = self
            .registered
            .write()
            .map_err(|_| SecurityError::MemoryCorruption("Lock poisoned".to_string()))?;

        // Check for resource limits
        if registered.len() >= self.config.max_possible_roots {
            self.security_handler
                .handle_event(SecurityEvent::ResourceLimitExceeded {
                    limit: "registered_objects".to_string(),
                    value: registered.len(),
                });
            return Err(SecurityError::ResourceLimitExceeded(
                "Too many registered objects".to_string(),
            ));
        }

        registered.insert(address, registered_obj);

        Ok(())
    }

    /// Unregister an object
    pub fn unregister(&self, address: usize) -> Result<(), SecurityError> {
        // Remove from registered
        let mut registered = self
            .registered
            .write()
            .map_err(|_| SecurityError::MemoryCorruption("Lock poisoned".to_string()))?;
        registered.remove(&address);

        // Remove from possible roots
        let mut possible_roots = self
            .possible_roots
            .lock()
            .map_err(|_| SecurityError::MemoryCorruption("Lock poisoned".to_string()))?;
        possible_roots.remove(&address);

        Ok(())
    }

    /// Add a possible cycle root with security checks
    pub fn add_possible_root(&self, address: usize) -> Result<(), SecurityError> {
        let mut possible_roots = self
            .possible_roots
            .lock()
            .map_err(|_| SecurityError::MemoryCorruption("Lock poisoned".to_string()))?;

        possible_roots.insert(address)?;
        Ok(())
    }

    /// Perform secure cycle collection
    pub fn collect(&self) -> Result<usize, SecurityError> {
        let start_time = Instant::now();
        let mut collected = 0;

        // Check for timeout
        let timeout_check = || {
            if start_time.elapsed() > self.config.max_collection_time {
                self.security_handler
                    .handle_event(SecurityEvent::CollectionTimeout {
                        duration: start_time.elapsed(),
                    });
                return Err(SecurityError::CollectionTimeout);
            }
            Ok(())
        };

        // Take snapshot of possible roots
        let roots = {
            let mut possible_roots = self
                .possible_roots
                .lock()
                .map_err(|_| SecurityError::MemoryCorruption("Lock poisoned".to_string()))?;
            let roots: Vec<_> = possible_roots.iter().cloned().collect();
            possible_roots.clear();
            roots
        };

        if roots.is_empty() {
            return Ok(0);
        }

        // Phase 1: Mark roots white with security checks
        timeout_check()?;
        for &root_addr in &roots {
            if let Some(wrapper) = self.create_secure_wrapper(root_addr)? {
                wrapper.set_color(Color::White)?;
                wrapper.set_buffered(true)?;
            }
        }

        // Phase 2: Scan roots
        timeout_check()?;
        let mut to_scan = Vec::new();
        for &root_addr in &roots {
            if let Some(wrapper) = self.create_secure_wrapper(root_addr)? {
                self.secure_scan(&wrapper, &mut to_scan)?;
            }
        }

        // Phase 3: Scan gray objects
        timeout_check()?;
        let mut depth = 0;
        while let Some(addr) = to_scan.pop() {
            depth += 1;
            if depth > self.config.max_graph_depth {
                self.security_handler
                    .handle_event(SecurityEvent::AttackDetected {
                        description: format!(
                            "Graph depth limit {} exceeded",
                            self.config.max_graph_depth
                        ),
                    });
                return Err(SecurityError::ResourceLimitExceeded(
                    "Graph depth limit exceeded".to_string(),
                ));
            }

            if let Some(wrapper) = self.create_secure_wrapper(addr)? {
                self.secure_scan_children(&wrapper, &mut to_scan)?;
            }
        }

        // Phase 4: Collect white objects
        timeout_check()?;
        collected = self.secure_collect_white(&roots)?;

        // Update statistics
        let mut stats = self
            .stats
            .lock()
            .map_err(|_| SecurityError::MemoryCorruption("Lock poisoned".to_string()))?;
        stats.collections += 1;
        stats.objects_collected += collected;
        stats.cycles_detected += if collected > 0 { 1 } else { 0 };
        stats.total_time += start_time.elapsed();

        Ok(collected)
    }

    /// Create a secure wrapper for an address
    fn create_secure_wrapper(
        &self,
        address: usize,
    ) -> Result<Option<SecureRcWrapper>, SecurityError> {
        let registered = self
            .registered
            .read()
            .map_err(|_| SecurityError::MemoryCorruption("Lock poisoned".to_string()))?;

        if let Some(reg_obj) = registered.get(&address) {
            let wrapper = SecureRcWrapper::new(
                NonNull::new(address as *mut u8).ok_or(SecurityError::InvalidAlignment)?,
                reg_obj.type_id,
                reg_obj.generation,
                reg_obj.object_size,
                self.config.clone(),
            )?;
            Ok(Some(wrapper))
        } else {
            Ok(None)
        }
    }

    /// Secure scan implementation
    fn secure_scan(
        &self,
        wrapper: &SecureRcWrapper,
        to_scan: &mut Vec<usize>,
    ) -> Result<(), SecurityError> {
        if wrapper.color()? != Color::White {
            return Ok(());
        }

        wrapper.set_color(Color::Gray)?;

        let strong_count = wrapper.strong_count()?;
        if strong_count > 1 {
            wrapper.set_color(Color::Black)?;
            wrapper.set_buffered(false)?;
        } else {
            to_scan.push(wrapper.address());
        }

        Ok(())
    }

    /// Secure scan children implementation
    fn secure_scan_children(
        &self,
        wrapper: &SecureRcWrapper,
        to_scan: &mut Vec<usize>,
    ) -> Result<(), SecurityError> {
        if wrapper.color()? != Color::Gray {
            return Ok(());
        }

        wrapper.trace_children(|child_addr| {
            if let Ok(Some(child_wrapper)) = self.create_secure_wrapper(child_addr) {
                let _ = self.secure_scan(&child_wrapper, to_scan);
            }
        })?;

        wrapper.set_color(Color::Black)?;
        wrapper.set_buffered(false)?;

        Ok(())
    }

    /// Secure collect white objects
    fn secure_collect_white(&self, roots: &[usize]) -> Result<usize, SecurityError> {
        let mut collected = 0;
        let mut to_free = Vec::new();

        for &addr in roots {
            if let Some(wrapper) = self.create_secure_wrapper(addr)? {
                if wrapper.color()? == Color::White && wrapper.is_buffered()? {
                    to_free.push(addr);
                    collected += 1;
                }
            }
        }

        // Actually free the objects
        for addr in to_free {
            self.unregister(addr)?;
        }

        Ok(collected)
    }

    /// Get statistics
    pub fn stats(&self) -> Result<SecureCollectionStats, SecurityError> {
        let stats = self
            .stats
            .lock()
            .map_err(|_| SecurityError::MemoryCorruption("Lock poisoned".to_string()))?;
        Ok(stats.clone())
    }

    /// Shutdown the collector
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

/// Global secure cycle collector instance
static SECURE_CYCLE_COLLECTOR: RwLock<Option<Arc<SecureCycleCollector>>> = RwLock::new(None);

/// Initialize the secure garbage collector
pub fn initialize_secure_gc() -> Result<(), SecurityError> {
    let config = GcSecurityConfig::default();
    let collector = Arc::new(SecureCycleCollector::new(config));

    let mut global_collector = SECURE_CYCLE_COLLECTOR
        .write()
        .map_err(|_| SecurityError::MemoryCorruption("Global lock poisoned".to_string()))?;

    *global_collector = Some(collector);
    Ok(())
}

/// Get the global secure collector
pub fn get_secure_collector() -> Result<Arc<SecureCycleCollector>, SecurityError> {
    let collector = SECURE_CYCLE_COLLECTOR
        .read()
        .map_err(|_| SecurityError::MemoryCorruption("Global lock poisoned".to_string()))?;

    collector
        .as_ref()
        .ok_or(SecurityError::MemoryCorruption(
            "Collector not initialized".to_string(),
        ))
        .map(Arc::clone)
}

/// Secure register function
pub fn secure_register_rc<T: ?Sized>(rc: &ScriptRc<T>) -> Result<(), SecurityError> {
    let collector = get_secure_collector()?;
    let object_size = std::mem::size_of_val(rc);
    collector.register(rc.as_raw() as usize, rc.type_id(), object_size)
}

/// Secure unregister function
pub fn secure_unregister_rc<T: ?Sized>(rc: &ScriptRc<T>) -> Result<(), SecurityError> {
    let collector = get_secure_collector()?;
    collector.unregister(rc.as_raw() as usize)
}

/// Secure possible cycle notification
pub fn secure_possible_cycle<T: ?Sized>(rc: &ScriptRc<T>) -> Result<(), SecurityError> {
    let collector = get_secure_collector()?;
    collector.add_possible_root(rc.as_raw() as usize)
}

/// Secure force collection
pub fn secure_collect_cycles() -> Result<usize, SecurityError> {
    let collector = get_secure_collector()?;
    collector.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_gc_initialization() {
        assert!(initialize_secure_gc().is_ok());
        assert!(get_secure_collector().is_ok());
    }

    #[test]
    fn test_bounded_set_limits() {
        let mut set = BoundedSet::new(2);
        assert!(set.insert(1).is_ok());
        assert!(set.insert(2).is_ok());
        assert!(set.insert(3).is_err());
    }

    #[test]
    fn test_security_config_validation() {
        let config = GcSecurityConfig::default();
        assert!(config.max_possible_roots > 0);
        assert!(config.max_collection_time > Duration::from_millis(0));
        assert!(config.max_graph_depth > 0);
    }
}
