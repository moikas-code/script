//! Runtime state recovery and validation mechanisms
//!
//! This module provides state recovery capabilities for the Script runtime,
//! including state validation, checkpoint creation, and rollback mechanisms.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use crate::runtime::{Result, RuntimeError, Value};

/// State recovery manager for runtime state
pub struct StateRecoveryManager {
    /// Checkpoints for rollback
    checkpoints: RwLock<HashMap<String, StateCheckpoint>>,
    /// Validation rules
    validation_rules: RwLock<Vec<Box<dyn Fn(&RuntimeState) -> ValidationResult + Send + Sync>>>,
    /// Recovery callbacks
    recovery_callbacks: RwLock<Vec<Box<dyn Fn(&mut RuntimeState) -> RecoveryResult + Send + Sync>>>,
    /// Recovery metrics
    metrics: Mutex<RecoveryMetrics>,
}

/// State checkpoint for recovery
#[derive(Debug, Clone)]
pub struct StateCheckpoint {
    /// Checkpoint identifier
    pub id: String,
    /// Timestamp when checkpoint was created
    pub timestamp: Instant,
    /// Serialized state data
    pub state_data: Vec<u8>,
    /// Metadata about the checkpoint
    pub metadata: HashMap<String, String>,
}

/// Runtime state representation
#[derive(Debug, Clone)]
pub struct RuntimeState {
    /// Variable bindings
    pub variables: HashMap<String, Value>,
    /// Memory usage statistics
    pub memory_usage: MemoryUsage,
    /// Active operations
    pub active_operations: Vec<Operation>,
    /// Error state
    pub error_state: Option<String>,
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryUsage {
    /// Total allocated bytes
    pub total_allocated: usize,
    /// Peak memory usage
    pub peak_usage: usize,
    /// Current active allocations
    pub active_allocations: usize,
    /// Garbage collection statistics
    pub gc_stats: GcStats,
}

/// Garbage collection statistics
#[derive(Debug, Clone)]
pub struct GcStats {
    /// Number of GC cycles
    pub cycles: u64,
    /// Total time spent in GC
    pub total_time: Duration,
    /// Objects collected
    pub objects_collected: u64,
    /// Bytes freed
    pub bytes_freed: usize,
}

/// Active operation tracking
#[derive(Debug, Clone)]
pub struct Operation {
    /// Operation identifier
    pub id: String,
    /// Operation type
    pub operation_type: OperationType,
    /// Start time
    pub start_time: Instant,
    /// Expected duration
    pub expected_duration: Duration,
    /// Current status
    pub status: OperationStatus,
}

/// Types of operations
#[derive(Debug, Clone)]
pub enum OperationType {
    /// Compilation operation
    Compilation,
    /// Execution operation
    Execution,
    /// Memory management operation
    MemoryManagement,
    /// I/O operation
    IO,
    /// Custom operation
    Custom(String),
}

/// Operation status
#[derive(Debug, Clone)]
pub enum OperationStatus {
    /// Operation is running
    Running,
    /// Operation is waiting
    Waiting,
    /// Operation is blocked
    Blocked,
    /// Operation completed successfully
    Completed,
    /// Operation failed
    Failed(String),
}

/// Validation result
#[derive(Debug, Clone)]
pub enum ValidationResult {
    /// State is valid
    Valid,
    /// State is invalid with reason
    Invalid(String),
    /// State is corrupted and needs recovery
    Corrupted(String),
}

/// Recovery result
#[derive(Debug, Clone)]
pub enum RecoveryResult {
    /// Recovery successful
    Success,
    /// Recovery failed
    Failed(String),
    /// Recovery partially successful
    Partial(String),
}

/// Recovery metrics
#[derive(Debug, Clone, Default)]
pub struct RecoveryMetrics {
    /// Total validation attempts
    pub total_validations: u64,
    /// Failed validations
    pub failed_validations: u64,
    /// Total recovery attempts
    pub total_recoveries: u64,
    /// Successful recoveries
    pub successful_recoveries: u64,
    /// Average recovery time
    pub average_recovery_time: Duration,
}

impl StateRecoveryManager {
    /// Create a new state recovery manager
    pub fn new() -> Self {
        StateRecoveryManager {
            checkpoints: RwLock::new(HashMap::new()),
            validation_rules: RwLock::new(Vec::new()),
            recovery_callbacks: RwLock::new(Vec::new()),
            metrics: Mutex::new(RecoveryMetrics::default()),
        }
    }

    /// Create a checkpoint of the current state
    pub fn create_checkpoint(&self, id: String, state: &RuntimeState) -> Result<()> {
        let checkpoint = StateCheckpoint {
            id: id.clone(),
            timestamp: Instant::now(),
            state_data: self.serialize_state(state)?,
            metadata: HashMap::new(),
        };

        let mut checkpoints = self.checkpoints.write().unwrap();
        checkpoints.insert(id, checkpoint);

        Ok(())
    }

    /// Rollback to a checkpoint
    pub fn rollback_to_checkpoint(&self, id: &str) -> Result<RuntimeState> {
        let checkpoints = self.checkpoints.read().unwrap();
        if let Some(checkpoint) = checkpoints.get(id) {
            self.deserialize_state(&checkpoint.state_data)
        } else {
            Err(RuntimeError::InvalidOperation(format!(
                "Checkpoint {} not found",
                id
            )))
        }
    }

    /// Validate runtime state
    pub fn validate_state(&self, state: &RuntimeState) -> ValidationResult {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.total_validations += 1;

        // Run validation rules
        let rules = self.validation_rules.read().unwrap();
        for rule in rules.iter() {
            match rule(state) {
                ValidationResult::Valid => continue,
                other => {
                    metrics.failed_validations += 1;
                    return other;
                }
            }
        }

        // Built-in validation checks
        if let Some(error) = &state.error_state {
            metrics.failed_validations += 1;
            return ValidationResult::Invalid(format!("Error state: {}", error));
        }

        // Check for memory corruption
        if state.memory_usage.total_allocated > state.memory_usage.peak_usage * 2 {
            metrics.failed_validations += 1;
            return ValidationResult::Corrupted("Memory usage anomaly detected".to_string());
        }

        // Check for stuck operations
        let now = Instant::now();
        for op in &state.active_operations {
            if now.duration_since(op.start_time) > op.expected_duration * 3 {
                metrics.failed_validations += 1;
                return ValidationResult::Corrupted(format!("Operation {} appears stuck", op.id));
            }
        }

        ValidationResult::Valid
    }

    /// Attempt to recover corrupted state
    pub fn recover_state(&self, state: &mut RuntimeState) -> RecoveryResult {
        let recovery_start = Instant::now();
        let mut metrics = self.metrics.lock().unwrap();
        metrics.total_recoveries += 1;

        // Try recovery callbacks
        let callbacks = self.recovery_callbacks.read().unwrap();
        for callback in callbacks.iter() {
            match callback(state) {
                RecoveryResult::Success => {
                    metrics.successful_recoveries += 1;
                    metrics.average_recovery_time = recovery_start.elapsed();
                    return RecoveryResult::Success;
                }
                RecoveryResult::Partial(msg) => {
                    metrics.successful_recoveries += 1;
                    metrics.average_recovery_time = recovery_start.elapsed();
                    return RecoveryResult::Partial(msg);
                }
                RecoveryResult::Failed(_) => continue,
            }
        }

        // Built-in recovery strategies
        if let Some(result) = self.attempt_memory_recovery(state) {
            match result {
                RecoveryResult::Success => {
                    metrics.successful_recoveries += 1;
                    metrics.average_recovery_time = recovery_start.elapsed();
                    return RecoveryResult::Success;
                }
                other => return other,
            }
        }

        if let Some(result) = self.attempt_operation_recovery(state) {
            match result {
                RecoveryResult::Success => {
                    metrics.successful_recoveries += 1;
                    metrics.average_recovery_time = recovery_start.elapsed();
                    return RecoveryResult::Success;
                }
                other => return other,
            }
        }

        RecoveryResult::Failed("All recovery attempts failed".to_string())
    }

    /// Add a validation rule
    pub fn add_validation_rule<F>(&self, rule: F)
    where
        F: Fn(&RuntimeState) -> ValidationResult + Send + Sync + 'static,
    {
        let mut rules = self.validation_rules.write().unwrap();
        rules.push(Box::new(rule));
    }

    /// Add a recovery callback
    pub fn add_recovery_callback<F>(&self, callback: F)
    where
        F: Fn(&mut RuntimeState) -> RecoveryResult + Send + Sync + 'static,
    {
        let mut callbacks = self.recovery_callbacks.write().unwrap();
        callbacks.push(Box::new(callback));
    }

    /// Get recovery metrics
    pub fn get_metrics(&self) -> RecoveryMetrics {
        let metrics = self.metrics.lock().unwrap();
        metrics.clone()
    }

    /// Serialize state to bytes
    fn serialize_state(&self, state: &RuntimeState) -> Result<Vec<u8>> {
        // In a real implementation, this would use a proper serialization format
        // For now, we'll just return a placeholder
        Ok(format!("{:?}", state).into_bytes())
    }

    /// Deserialize state from bytes
    fn deserialize_state(&self, _data: &[u8]) -> Result<RuntimeState> {
        // In a real implementation, this would deserialize from the format used above
        // For now, we'll return a default state
        Ok(RuntimeState {
            variables: HashMap::new(),
            memory_usage: MemoryUsage {
                total_allocated: 0,
                peak_usage: 0,
                active_allocations: 0,
                gc_stats: GcStats {
                    cycles: 0,
                    total_time: Duration::from_secs(0),
                    objects_collected: 0,
                    bytes_freed: 0,
                },
            },
            active_operations: Vec::new(),
            error_state: None,
        })
    }

    /// Attempt memory recovery
    fn attempt_memory_recovery(&self, state: &mut RuntimeState) -> Option<RecoveryResult> {
        // Check for memory leaks
        if state.memory_usage.total_allocated > state.memory_usage.peak_usage * 2 {
            // Reset memory statistics
            state.memory_usage.total_allocated = state.memory_usage.active_allocations;
            state.memory_usage.peak_usage = state.memory_usage.total_allocated;

            return Some(RecoveryResult::Partial(
                "Memory statistics reset".to_string(),
            ));
        }

        None
    }

    /// Attempt operation recovery
    fn attempt_operation_recovery(&self, state: &mut RuntimeState) -> Option<RecoveryResult> {
        let now = Instant::now();
        let mut recovered = false;

        // Cancel stuck operations
        state.active_operations.retain(|op| {
            if now.duration_since(op.start_time) > op.expected_duration * 3 {
                recovered = true;
                false // Remove stuck operation
            } else {
                true // Keep operation
            }
        });

        if recovered {
            Some(RecoveryResult::Partial(
                "Cancelled stuck operations".to_string(),
            ))
        } else {
            None
        }
    }
}

impl Default for StateRecoveryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global state recovery manager
static STATE_RECOVERY_MANAGER: RwLock<Option<Arc<StateRecoveryManager>>> = RwLock::new(None);

/// Initialize the state recovery manager
pub fn initialize_state_recovery() {
    let mut manager = STATE_RECOVERY_MANAGER.write().unwrap();
    *manager = Some(Arc::new(StateRecoveryManager::new()));
}

/// Shutdown the state recovery manager
pub fn shutdown_state_recovery() {
    let mut manager = STATE_RECOVERY_MANAGER.write().unwrap();
    *manager = None;
}

/// Create a state checkpoint
pub fn create_checkpoint(id: String, state: &RuntimeState) -> Result<()> {
    if let Ok(manager) = STATE_RECOVERY_MANAGER.read() {
        if let Some(m) = manager.as_ref() {
            return m.create_checkpoint(id, state);
        }
    }
    Err(RuntimeError::NotInitialized)
}

/// Rollback to a checkpoint
pub fn rollback_to_checkpoint(id: &str) -> Result<RuntimeState> {
    if let Ok(manager) = STATE_RECOVERY_MANAGER.read() {
        if let Some(m) = manager.as_ref() {
            return m.rollback_to_checkpoint(id);
        }
    }
    Err(RuntimeError::NotInitialized)
}

/// Validate runtime state
pub fn validate_state(state: &RuntimeState) -> ValidationResult {
    if let Ok(manager) = STATE_RECOVERY_MANAGER.read() {
        if let Some(m) = manager.as_ref() {
            return m.validate_state(state);
        }
    }
    ValidationResult::Invalid("State recovery manager not initialized".to_string())
}

/// Recover corrupted state
pub fn recover_state(state: &mut RuntimeState) -> RecoveryResult {
    if let Ok(manager) = STATE_RECOVERY_MANAGER.read() {
        if let Some(m) = manager.as_ref() {
            return m.recover_state(state);
        }
    }
    RecoveryResult::Failed("State recovery manager not initialized".to_string())
}

/// Add a validation rule
pub fn add_validation_rule<F>(rule: F)
where
    F: Fn(&RuntimeState) -> ValidationResult + Send + Sync + 'static,
{
    if let Ok(manager) = STATE_RECOVERY_MANAGER.read() {
        if let Some(m) = manager.as_ref() {
            m.add_validation_rule(rule);
        }
    }
}

/// Add a recovery callback
pub fn add_recovery_callback<F>(callback: F)
where
    F: Fn(&mut RuntimeState) -> RecoveryResult + Send + Sync + 'static,
{
    if let Ok(manager) = STATE_RECOVERY_MANAGER.read() {
        if let Some(m) = manager.as_ref() {
            m.add_recovery_callback(callback);
        }
    }
}

/// Get recovery metrics
pub fn get_recovery_metrics() -> Option<RecoveryMetrics> {
    if let Ok(manager) = STATE_RECOVERY_MANAGER.read() {
        if let Some(m) = manager.as_ref() {
            return Some(m.get_metrics());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_recovery_manager() {
        let manager = StateRecoveryManager::new();

        // Create a test state
        let state = RuntimeState {
            variables: HashMap::new(),
            memory_usage: MemoryUsage {
                total_allocated: 1000,
                peak_usage: 1000,
                active_allocations: 10,
                gc_stats: GcStats {
                    cycles: 5,
                    total_time: Duration::from_millis(100),
                    objects_collected: 50,
                    bytes_freed: 500,
                },
            },
            active_operations: Vec::new(),
            error_state: None,
        };

        // Test checkpoint creation
        assert!(manager
            .create_checkpoint("test_checkpoint".to_string(), &state)
            .is_ok());

        // Test rollback
        assert!(manager.rollback_to_checkpoint("test_checkpoint").is_ok());
    }

    #[test]
    fn test_state_validation() {
        let manager = StateRecoveryManager::new();

        // Test valid state
        let valid_state = RuntimeState {
            variables: HashMap::new(),
            memory_usage: MemoryUsage {
                total_allocated: 1000,
                peak_usage: 1000,
                active_allocations: 10,
                gc_stats: GcStats {
                    cycles: 5,
                    total_time: Duration::from_millis(100),
                    objects_collected: 50,
                    bytes_freed: 500,
                },
            },
            active_operations: Vec::new(),
            error_state: None,
        };

        match manager.validate_state(&valid_state) {
            ValidationResult::Valid => {}
            other => panic!("Expected valid state, got {:?}", other),
        }

        // Test invalid state
        let invalid_state = RuntimeState {
            variables: HashMap::new(),
            memory_usage: MemoryUsage {
                total_allocated: 1000,
                peak_usage: 1000,
                active_allocations: 10,
                gc_stats: GcStats {
                    cycles: 5,
                    total_time: Duration::from_millis(100),
                    objects_collected: 50,
                    bytes_freed: 500,
                },
            },
            active_operations: Vec::new(),
            error_state: Some("Test error".to_string()),
        };

        match manager.validate_state(&invalid_state) {
            ValidationResult::Invalid(_) => {}
            other => panic!("Expected invalid state, got {:?}", other),
        }
    }

    #[test]
    fn test_state_recovery() {
        let manager = StateRecoveryManager::new();

        // Test corrupted state with memory anomaly
        let mut corrupted_state = RuntimeState {
            variables: HashMap::new(),
            memory_usage: MemoryUsage {
                total_allocated: 10000,
                peak_usage: 1000,
                active_allocations: 10,
                gc_stats: GcStats {
                    cycles: 5,
                    total_time: Duration::from_millis(100),
                    objects_collected: 50,
                    bytes_freed: 500,
                },
            },
            active_operations: Vec::new(),
            error_state: None,
        };

        match manager.recover_state(&mut corrupted_state) {
            RecoveryResult::Partial(_) => {}
            other => panic!("Expected partial recovery, got {:?}", other),
        }
    }

    #[test]
    fn test_validation_rules() {
        let manager = StateRecoveryManager::new();

        // Add a custom validation rule
        manager.add_validation_rule(|state| {
            if state.memory_usage.total_allocated > 5000 {
                ValidationResult::Invalid("Memory usage too high".to_string())
            } else {
                ValidationResult::Valid
            }
        });

        // Test with high memory usage
        let high_memory_state = RuntimeState {
            variables: HashMap::new(),
            memory_usage: MemoryUsage {
                total_allocated: 6000,
                peak_usage: 6000,
                active_allocations: 10,
                gc_stats: GcStats {
                    cycles: 5,
                    total_time: Duration::from_millis(100),
                    objects_collected: 50,
                    bytes_freed: 500,
                },
            },
            active_operations: Vec::new(),
            error_state: None,
        };

        match manager.validate_state(&high_memory_state) {
            ValidationResult::Invalid(_) => {}
            other => panic!("Expected invalid state, got {:?}", other),
        }
    }

    #[test]
    fn test_recovery_callbacks() {
        let manager = StateRecoveryManager::new();

        // Add a recovery callback
        manager.add_recovery_callback(|state| {
            if state.error_state.is_some() {
                state.error_state = None;
                RecoveryResult::Success
            } else {
                RecoveryResult::Failed("No error to recover".to_string())
            }
        });

        // Test recovery
        let mut error_state = RuntimeState {
            variables: HashMap::new(),
            memory_usage: MemoryUsage {
                total_allocated: 1000,
                peak_usage: 1000,
                active_allocations: 10,
                gc_stats: GcStats {
                    cycles: 5,
                    total_time: Duration::from_millis(100),
                    objects_collected: 50,
                    bytes_freed: 500,
                },
            },
            active_operations: Vec::new(),
            error_state: Some("Test error".to_string()),
        };

        match manager.recover_state(&mut error_state) {
            RecoveryResult::Success => {}
            other => panic!("Expected successful recovery, got {:?}", other),
        }

        assert!(error_state.error_state.is_none());
    }
}
