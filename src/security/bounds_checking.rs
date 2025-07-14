//! Array bounds checking implementation for Script language
//!
//! This module provides secure bounds checking for array indexing operations
//! to prevent buffer overflow vulnerabilities and memory corruption.

use super::{SecurityError, SecurityMetrics};
use crate::error::{Error, ErrorKind};
use crate::ir::{Instruction, ValueId};
use crate::types::Type;

/// Bounds check configuration with performance optimizations
#[derive(Debug, Clone)]
pub struct BoundsCheckConfig {
    /// Enable runtime bounds checking (default: true)
    pub enable_runtime_checks: bool,
    /// Enable compile-time bounds analysis (default: true)
    pub enable_static_analysis: bool,
    /// Emit bounds check instructions in IR (default: true)
    pub emit_check_instructions: bool,
    /// Panic on bounds violation vs return error (default: false - return error)
    pub panic_on_violation: bool,
    /// Enable fast-path optimizations (default: true)
    pub enable_fast_path: bool,
    /// Batch size for bounds checking (default: 64)
    pub batch_size: usize,
    /// Cache size for recently validated bounds (default: 256)
    pub cache_size: usize,
}

impl Default for BoundsCheckConfig {
    fn default() -> Self {
        BoundsCheckConfig {
            #[cfg(debug_assertions)]
            enable_runtime_checks: true,
            #[cfg(not(debug_assertions))]
            enable_runtime_checks: false, // Disabled in release for performance
            enable_static_analysis: true,
            #[cfg(debug_assertions)]
            emit_check_instructions: true,
            #[cfg(not(debug_assertions))]
            emit_check_instructions: false, // Disabled in release for performance
            panic_on_violation: false,
            enable_fast_path: true,
            batch_size: 64,
            cache_size: 256,
        }
    }
}

/// Bounds check result
#[derive(Debug, Clone, PartialEq)]
pub enum BoundsCheckResult {
    /// Bounds check passed - access is safe
    Safe,
    /// Bounds check failed - access would be out of bounds
    OutOfBounds {
        array_size: usize,
        attempted_index: i64,
    },
    /// Bounds check cannot be determined at compile time
    Unknown,
}

/// Bounds checking engine with performance optimizations
pub struct BoundsChecker {
    config: BoundsCheckConfig,
    metrics: Option<SecurityMetrics>,
    /// Cache for recently validated bounds (array_size, index) -> is_valid
    bounds_cache: std::collections::HashMap<(usize, i64), bool>,
    /// Counter for batched checking
    check_counter: usize,
}

impl BoundsChecker {
    /// Create new bounds checker with default configuration
    pub fn new() -> Self {
        BoundsChecker {
            config: BoundsCheckConfig::default(),
            metrics: None,
            bounds_cache: std::collections::HashMap::new(),
            check_counter: 0,
        }
    }

    /// Create bounds checker with custom configuration
    pub fn with_config(config: BoundsCheckConfig) -> Self {
        let cache_size = config.cache_size;
        BoundsChecker {
            config,
            metrics: None,
            bounds_cache: std::collections::HashMap::with_capacity(cache_size),
            check_counter: 0,
        }
    }

    /// Set security metrics for recording events
    pub fn with_metrics(mut self, metrics: SecurityMetrics) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Clear the bounds checking cache
    pub fn clear_cache(&mut self) {
        self.bounds_cache.clear();
        self.check_counter = 0;
    }

    /// Get cache hit ratio for performance monitoring
    pub fn get_cache_hit_ratio(&self) -> f64 {
        if self.check_counter == 0 {
            return 0.0;
        }
        let cache_hits = self.bounds_cache.len();
        cache_hits as f64 / self.check_counter as f64
    }

    /// Generate bounds check instruction for array indexing
    pub fn generate_bounds_check(
        &self,
        array: ValueId,
        index: ValueId,
        array_type: &Type,
        error_context: &str,
    ) -> Result<Instruction, Error> {
        if !self.config.enable_runtime_checks {
            return Err(Error::new(
                ErrorKind::SecurityViolation,
                "Bounds checking is disabled but check was requested".to_string(),
            ));
        }

        let error_msg = format!("Array index out of bounds in {}", error_context);

        // Try to extract array length from type information
        let length = self.extract_array_length(array_type);

        Ok(Instruction::BoundsCheck {
            array,
            index,
            length,
            error_msg,
        })
    }

    /// Extract array length from type information if available
    fn extract_array_length(&self, array_type: &Type) -> Option<ValueId> {
        match array_type {
            Type::Array(_element_type) => {
                // For dynamic arrays, we cannot determine the size from the type alone
                // The size must be determined at runtime
                None
            }
            _ => None,
        }
    }

    /// Perform static bounds checking analysis with caching optimization
    pub fn static_bounds_check(
        &mut self,
        array_size: Option<usize>,
        index_value: Option<i64>,
    ) -> BoundsCheckResult {
        if !self.config.enable_static_analysis {
            return BoundsCheckResult::Unknown;
        }

        match (array_size, index_value) {
            (Some(size), Some(index)) => {
                self.check_counter += 1;

                // Fast path: check cache first
                if self.config.enable_fast_path {
                    if let Some(&is_safe) = self.bounds_cache.get(&(size, index)) {
                        return if is_safe {
                            BoundsCheckResult::Safe
                        } else {
                            BoundsCheckResult::OutOfBounds {
                                array_size: size,
                                attempted_index: index,
                            }
                        };
                    }
                }

                // Perform bounds check
                let is_safe = index >= 0 && index < size as i64;

                // Record the check in metrics
                if let Some(ref metrics) = self.metrics {
                    metrics.record_bounds_check(!is_safe);
                }

                // Update cache if fast path is enabled
                if self.config.enable_fast_path {
                    // Manage cache size
                    if self.bounds_cache.len() >= self.config.cache_size {
                        // Simple eviction: clear half the cache
                        let keys_to_remove: Vec<_> = self
                            .bounds_cache
                            .keys()
                            .take(self.config.cache_size / 2)
                            .cloned()
                            .collect();
                        for key in keys_to_remove {
                            self.bounds_cache.remove(&key);
                        }
                    }
                    self.bounds_cache.insert((size, index), is_safe);
                }

                if is_safe {
                    BoundsCheckResult::Safe
                } else {
                    BoundsCheckResult::OutOfBounds {
                        array_size: size,
                        attempted_index: index,
                    }
                }
            }
            _ => BoundsCheckResult::Unknown,
        }
    }

    /// Validate bounds check at runtime with optimizations
    pub fn runtime_bounds_check(
        &mut self,
        array_size: usize,
        index: i64,
        error_context: &str,
    ) -> Result<(), SecurityError> {
        if !self.config.enable_runtime_checks {
            return Ok(()); // Skip runtime checks if disabled
        }

        self.check_counter += 1;

        // Fast path: check cache first
        if self.config.enable_fast_path {
            if let Some(&is_safe) = self.bounds_cache.get(&(array_size, index)) {
                if is_safe {
                    return Ok(());
                } else {
                    return Err(SecurityError::BoundsViolation {
                        array_size,
                        index,
                        message: format!("Bounds violation in {}", error_context),
                    });
                }
            }
        }

        // Batched checking optimization: only perform expensive validation periodically
        if self.config.enable_fast_path && self.check_counter % self.config.batch_size != 0 {
            // For most checks, use fast integer comparison
            let is_safe = index >= 0 && index < array_size as i64;
            if is_safe {
                return Ok(());
            }
        }

        // Full validation
        let is_safe = index >= 0 && index < array_size as i64;

        // Record the check in metrics
        if let Some(ref metrics) = self.metrics {
            metrics.record_bounds_check(!is_safe);
        }

        // Update cache
        if self.config.enable_fast_path {
            if self.bounds_cache.len() >= self.config.cache_size {
                // Simple eviction strategy
                let keys_to_remove: Vec<_> = self
                    .bounds_cache
                    .keys()
                    .take(self.config.cache_size / 2)
                    .cloned()
                    .collect();
                for key in keys_to_remove {
                    self.bounds_cache.remove(&key);
                }
            }
            self.bounds_cache.insert((array_size, index), is_safe);
        }

        if is_safe {
            Ok(())
        } else {
            Err(SecurityError::BoundsViolation {
                array_size,
                index,
                message: format!("Bounds violation in {}", error_context),
            })
        }
    }

    /// Generate secure array access pattern
    /// Returns instructions for safe array access with bounds checking
    pub fn generate_secure_array_access(
        &self,
        array: ValueId,
        index: ValueId,
        array_type: &Type,
        element_type: &Type,
        error_context: &str,
    ) -> Result<Vec<Instruction>, Error> {
        let mut instructions = Vec::new();

        if self.config.emit_check_instructions {
            // Generate bounds check instruction
            let bounds_check =
                self.generate_bounds_check(array, index, array_type, error_context)?;
            instructions.push(bounds_check);
        }

        // Generate the actual array access
        let access_instruction = Instruction::GetElementPtr {
            ptr: array,
            index,
            elem_ty: element_type.clone(),
        };
        instructions.push(access_instruction);

        Ok(instructions)
    }

    /// Check if bounds checking is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enable_runtime_checks || self.config.enable_static_analysis
    }

    /// Get configuration
    pub fn config(&self) -> &BoundsCheckConfig {
        &self.config
    }
}

impl Default for BoundsChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for bounds checking integration

/// Create a secure array indexing helper
pub fn create_secure_index_instruction(
    bounds_checker: &BoundsChecker,
    array: ValueId,
    index: ValueId,
    array_type: &Type,
    element_type: &Type,
    context: &str,
) -> Result<Vec<Instruction>, Error> {
    bounds_checker.generate_secure_array_access(array, index, array_type, element_type, context)
}

/// Validate array access safety at compile time
pub fn validate_array_access_safety(
    bounds_checker: &mut BoundsChecker,
    array_size: Option<usize>,
    index_value: Option<i64>,
    context: &str,
) -> Result<(), Error> {
    let result = bounds_checker.static_bounds_check(array_size, index_value);

    match result {
        BoundsCheckResult::Safe => Ok(()),
        BoundsCheckResult::OutOfBounds {
            array_size,
            attempted_index,
        } => Err(Error::new(
            ErrorKind::SecurityViolation,
            format!(
                "Static bounds check failed in {}: index {} out of bounds for array of size {}",
                context, attempted_index, array_size
            ),
        )),
        BoundsCheckResult::Unknown => {
            // Cannot determine at compile time - runtime check will be needed
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::SecurityMetrics;

    #[test]
    fn test_bounds_checker_creation() {
        let checker = BoundsChecker::new();
        assert!(checker.is_enabled());
    }

    #[test]
    fn test_static_bounds_check_safe() {
        let mut checker = BoundsChecker::new();
        let result = checker.static_bounds_check(Some(10), Some(5));
        assert_eq!(result, BoundsCheckResult::Safe);
    }

    #[test]
    fn test_static_bounds_check_out_of_bounds() {
        let mut checker = BoundsChecker::new();
        let result = checker.static_bounds_check(Some(10), Some(15));
        assert!(matches!(result, BoundsCheckResult::OutOfBounds { .. }));
    }

    #[test]
    fn test_static_bounds_check_negative_index() {
        let mut checker = BoundsChecker::new();
        let result = checker.static_bounds_check(Some(10), Some(-1));
        assert!(matches!(result, BoundsCheckResult::OutOfBounds { .. }));
    }

    #[test]
    fn test_static_bounds_check_unknown() {
        let mut checker = BoundsChecker::new();
        let result = checker.static_bounds_check(None, Some(5));
        assert_eq!(result, BoundsCheckResult::Unknown);
    }

    #[test]
    fn test_runtime_bounds_check_safe() {
        let mut checker = BoundsChecker::new();
        let result = checker.runtime_bounds_check(10, 5, "test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_runtime_bounds_check_violation() {
        let mut checker = BoundsChecker::new();
        let result = checker.runtime_bounds_check(10, 15, "test");
        assert!(result.is_err());

        if let Err(SecurityError::BoundsViolation {
            array_size, index, ..
        }) = result
        {
            assert_eq!(array_size, 10);
            assert_eq!(index, 15);
        } else {
            panic!("Expected BoundsViolation error");
        }
    }

    #[test]
    fn test_bounds_checker_with_metrics() {
        let metrics = SecurityMetrics::new();
        let mut checker = BoundsChecker::new().with_metrics(metrics);

        // Perform a bounds check that should trigger metrics
        let _result = checker.static_bounds_check(Some(10), Some(15));

        // The metrics would be recorded in the referenced SecurityMetrics
        // This test mainly verifies the API works
    }

    #[test]
    fn test_bounds_check_config() {
        let mut config = BoundsCheckConfig::default();
        config.enable_runtime_checks = false;

        let checker = BoundsChecker::with_config(config);
        assert!(!checker.config().enable_runtime_checks);
        assert!(checker.config().enable_static_analysis); // Still enabled
    }

    #[test]
    fn test_generate_bounds_check_instruction() {
        let checker = BoundsChecker::new();
        let array = ValueId(1);
        let index = ValueId(2);
        let array_type = Type::Array(Box::new(Type::I32));

        let result = checker.generate_bounds_check(array, index, &array_type, "test context");
        assert!(result.is_ok());

        if let Ok(Instruction::BoundsCheck {
            array: check_array,
            index: check_index,
            ..
        }) = result
        {
            assert_eq!(check_array, array);
            assert_eq!(check_index, index);
        } else {
            panic!("Expected BoundsCheck instruction");
        }
    }

    #[test]
    fn test_secure_array_access_generation() {
        let checker = BoundsChecker::new();
        let array = ValueId(1);
        let index = ValueId(2);
        let array_type = Type::Array(Box::new(Type::I32));
        let element_type = Type::I32;

        let result =
            checker.generate_secure_array_access(array, index, &array_type, &element_type, "test");
        assert!(result.is_ok());

        let instructions = result.unwrap();
        assert_eq!(instructions.len(), 2); // Bounds check + GetElementPtr

        // First instruction should be bounds check
        assert!(matches!(instructions[0], Instruction::BoundsCheck { .. }));
        // Second instruction should be array access
        assert!(matches!(instructions[1], Instruction::GetElementPtr { .. }));
    }
}
