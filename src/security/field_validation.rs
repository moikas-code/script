//! Field access validation for Script language
//!
//! This module provides secure field access validation to prevent
//! type confusion attacks and unauthorized memory access through
//! dynamic field operations.

use super::{SecurityError, SecurityMetrics};
use crate::error::{Error, ErrorKind};
use crate::ir::{Instruction, ValueId};
use crate::types::Type;
use std::collections::HashMap;

/// Field validation configuration with performance optimizations
#[derive(Debug, Clone)]
pub struct FieldValidationConfig {
    /// Enable runtime field validation (default: true)
    pub enable_runtime_validation: bool,
    /// Enable compile-time field analysis (default: true)
    pub enable_static_analysis: bool,
    /// Emit field validation instructions in IR (default: true)
    pub emit_validation_instructions: bool,
    /// Cache field validation results (default: true)
    pub enable_validation_cache: bool,
    /// Strict mode - reject unknown fields (default: true)
    pub strict_mode: bool,
    /// Enable fast-path optimizations (default: true)
    pub enable_fast_path: bool,
    /// Maximum cache size (default: 1024)
    pub max_cache_size: usize,
    /// Cache eviction threshold (default: 0.8)
    pub cache_eviction_threshold: f64,
}

impl Default for FieldValidationConfig {
    fn default() -> Self {
        FieldValidationConfig {
            #[cfg(debug_assertions)]
            enable_runtime_validation: true,
            #[cfg(not(debug_assertions))]
            enable_runtime_validation: false, // Disabled in release for performance
            enable_static_analysis: true,
            #[cfg(debug_assertions)]
            emit_validation_instructions: true,
            #[cfg(not(debug_assertions))]
            emit_validation_instructions: false, // Disabled in release for performance
            enable_validation_cache: true,
            strict_mode: true,
            enable_fast_path: true,
            max_cache_size: 1024,
            cache_eviction_threshold: 0.8,
        }
    }
}

/// Field validation result
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValidationResult {
    /// Field access is valid
    Valid {
        field_offset: Option<u32>,
        field_type: Type,
    },
    /// Field does not exist on the type
    InvalidField {
        type_name: String,
        field_name: String,
    },
    /// Type information is insufficient for validation
    InsufficientTypeInfo,
    /// Field access is ambiguous
    Ambiguous { possible_types: Vec<String> },
}

/// Type information for field validation
#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub name: String,
    pub fields: HashMap<String, FieldInfo>,
    pub is_generic: bool,
    pub base_types: Vec<String>, // For inheritance/trait implementation
}

/// Field information
#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub name: String,
    pub field_type: Type,
    pub offset: u32,
    pub is_public: bool,
    pub is_mutable: bool,
}

/// Field validator with type registry and performance optimizations
pub struct FieldValidator {
    config: FieldValidationConfig,
    type_registry: HashMap<String, TypeInfo>,
    validation_cache: HashMap<String, FieldValidationResult>,
    metrics: Option<SecurityMetrics>,
    /// Fast lookup cache for commonly accessed field types
    field_type_cache: HashMap<String, Type>,
    /// Cache access counter for LRU eviction
    cache_access_counter: std::collections::HashMap<String, u64>,
    /// Global access counter for LRU tracking
    global_access_counter: u64,
}

impl FieldValidator {
    /// Create new field validator
    pub fn new() -> Self {
        FieldValidator {
            config: FieldValidationConfig::default(),
            type_registry: HashMap::new(),
            validation_cache: HashMap::new(),
            metrics: None,
            field_type_cache: HashMap::new(),
            cache_access_counter: HashMap::new(),
            global_access_counter: 0,
        }
    }

    /// Create field validator with custom configuration
    pub fn with_config(config: FieldValidationConfig) -> Self {
        let initial_capacity = config.max_cache_size / 2;
        FieldValidator {
            config,
            type_registry: HashMap::new(),
            validation_cache: HashMap::with_capacity(initial_capacity),
            metrics: None,
            field_type_cache: HashMap::with_capacity(initial_capacity),
            cache_access_counter: HashMap::new(),
            global_access_counter: 0,
        }
    }

    /// Set security metrics for recording events
    pub fn with_metrics(mut self, metrics: SecurityMetrics) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Register a type in the validator's registry
    pub fn register_type(&mut self, type_info: TypeInfo) {
        // Pre-populate fast lookup cache for common field accesses
        if self.config.enable_fast_path {
            for (field_name, field_info) in &type_info.fields {
                let cache_key = format!("{}::{}", type_info.name, field_name);
                self.field_type_cache
                    .insert(cache_key, field_info.field_type.clone());
            }
        }

        // Save the type name before moving type_info
        let type_name = type_info.name.clone();
        self.type_registry.insert(type_name.clone(), type_info);

        // Selectively clear cache when types change
        if self.config.enable_validation_cache {
            // Only clear entries related to this type instead of full clear
            let keys_to_remove: Vec<_> = self
                .validation_cache
                .keys()
                .filter(|key| key.contains(&format!("{}::", &type_name)))
                .cloned()
                .collect();
            for key in keys_to_remove {
                self.validation_cache.remove(&key);
                self.cache_access_counter.remove(&key);
            }
        }
    }

    /// Register multiple types
    pub fn register_types(&mut self, types: Vec<TypeInfo>) {
        for type_info in types {
            self.register_type(type_info);
        }
    }

    /// Generate field validation instruction
    pub fn generate_field_validation(
        &self,
        object: ValueId,
        field_name: &str,
        object_type: &Type,
        _error_context: &str,
    ) -> Result<Instruction, Error> {
        if !self.config.enable_runtime_validation {
            return Err(Error::new(
                ErrorKind::SecurityViolation,
                "Field validation is disabled but validation was requested".to_string(),
            ));
        }

        Ok(Instruction::ValidateFieldAccess {
            object,
            field_name: field_name.to_string(),
            object_type: object_type.clone(),
        })
    }

    /// Validate field access statically with performance optimizations
    pub fn validate_field_access(
        &mut self,
        type_name: &str,
        field_name: &str,
    ) -> FieldValidationResult {
        let cache_key = format!("{}::{}", type_name, field_name);

        // Fast path: check field type cache first
        if self.config.enable_fast_path {
            if let Some(field_type) = self.field_type_cache.get(&cache_key) {
                // Update access tracking for LRU
                self.global_access_counter += 1;
                self.cache_access_counter
                    .insert(cache_key.clone(), self.global_access_counter);

                return FieldValidationResult::Valid {
                    field_offset: None, // Fast path doesn't compute offset
                    field_type: field_type.clone(),
                };
            }
        }

        // Check validation cache
        if self.config.enable_validation_cache {
            if let Some(cached_result) = self.validation_cache.get(&cache_key) {
                // Update access tracking
                self.global_access_counter += 1;
                self.cache_access_counter
                    .insert(cache_key.clone(), self.global_access_counter);
                return cached_result.clone();
            }
        }

        let result = self.validate_field_access_impl(type_name, field_name);

        // Record metrics
        if let Some(ref metrics) = self.metrics {
            let invalid_access = matches!(result, FieldValidationResult::InvalidField { .. });
            metrics.record_field_validation(invalid_access);
        }

        // Cache result with LRU eviction
        if self.config.enable_validation_cache {
            self.maybe_evict_cache();
            self.global_access_counter += 1;
            self.validation_cache
                .insert(cache_key.clone(), result.clone());
            self.cache_access_counter
                .insert(cache_key, self.global_access_counter);
        }

        result
    }

    /// Internal field validation implementation
    fn validate_field_access_impl(
        &self,
        type_name: &str,
        field_name: &str,
    ) -> FieldValidationResult {
        if !self.config.enable_static_analysis {
            return FieldValidationResult::InsufficientTypeInfo;
        }

        // Look up type information
        if let Some(type_info) = self.type_registry.get(type_name) {
            // Direct field lookup
            if let Some(field_info) = type_info.fields.get(field_name) {
                return FieldValidationResult::Valid {
                    field_offset: Some(field_info.offset),
                    field_type: field_info.field_type.clone(),
                };
            }

            // Check base types/traits
            for base_type in &type_info.base_types {
                if let Some(base_info) = self.type_registry.get(base_type) {
                    if let Some(field_info) = base_info.fields.get(field_name) {
                        return FieldValidationResult::Valid {
                            field_offset: Some(field_info.offset),
                            field_type: field_info.field_type.clone(),
                        };
                    }
                }
            }

            // Field not found
            if self.config.strict_mode {
                FieldValidationResult::InvalidField {
                    type_name: type_name.to_string(),
                    field_name: field_name.to_string(),
                }
            } else {
                // In non-strict mode, allow unknown fields for gradual typing
                FieldValidationResult::Valid {
                    field_offset: None,
                    field_type: Type::Unknown,
                }
            }
        } else {
            // Type not registered
            if self.config.strict_mode {
                FieldValidationResult::InvalidField {
                    type_name: type_name.to_string(),
                    field_name: field_name.to_string(),
                }
            } else {
                FieldValidationResult::InsufficientTypeInfo
            }
        }
    }

    /// Validate field access at runtime
    pub fn runtime_field_validation(
        &mut self,
        type_name: &str,
        field_name: &str,
        error_context: &str,
    ) -> Result<FieldInfo, SecurityError> {
        let result = self.validate_field_access(type_name, field_name);

        match result {
            FieldValidationResult::Valid {
                field_type,
                field_offset,
            } => {
                Ok(FieldInfo {
                    name: field_name.to_string(),
                    field_type,
                    offset: field_offset.unwrap_or(0),
                    is_public: true,  // Default for validated fields
                    is_mutable: true, // Default for validated fields
                })
            }
            FieldValidationResult::InvalidField {
                type_name,
                field_name,
            } => Err(SecurityError::InvalidFieldAccess {
                type_name,
                field_name,
                message: format!("Invalid field access in {}", error_context),
            }),
            _ => Err(SecurityError::InvalidFieldAccess {
                type_name: type_name.to_string(),
                field_name: field_name.to_string(),
                message: format!("Cannot validate field access in {}", error_context),
            }),
        }
    }

    /// Generate secure field access pattern
    pub fn generate_secure_field_access(
        &self,
        object: ValueId,
        field_name: &str,
        object_type: &Type,
        error_context: &str,
    ) -> Result<Vec<Instruction>, Error> {
        let mut instructions = Vec::new();

        if self.config.emit_validation_instructions {
            // Generate field validation instruction
            let validation =
                self.generate_field_validation(object, field_name, object_type, error_context)?;
            instructions.push(validation);
        }

        // For known types, use type-safe field access
        if let Some(field_type) = self.extract_field_type(object_type, field_name) {
            let field_access = Instruction::LoadField {
                object,
                field_name: field_name.to_string(),
                field_ty: field_type,
            };
            instructions.push(field_access);
        } else {
            // For unknown types, use dynamic access with validation
            let field_access = Instruction::GetFieldPtr {
                object,
                field_name: field_name.to_string(),
                field_ty: Type::Unknown,
            };
            instructions.push(field_access);
        }

        Ok(instructions)
    }

    /// Extract field type from object type if known
    fn extract_field_type(&self, object_type: &Type, field_name: &str) -> Option<Type> {
        match object_type {
            Type::Named(type_name) => {
                if let Some(type_info) = self.type_registry.get(type_name) {
                    type_info
                        .fields
                        .get(field_name)
                        .map(|field| field.field_type.clone())
                } else {
                    None
                }
            }
            Type::Struct { name, .. } => {
                if let Some(type_info) = self.type_registry.get(name) {
                    type_info
                        .fields
                        .get(field_name)
                        .map(|field| field.field_type.clone())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Get registered type information
    pub fn get_type_info(&self, type_name: &str) -> Option<&TypeInfo> {
        self.type_registry.get(type_name)
    }

    /// List all registered types
    pub fn list_types(&self) -> Vec<&str> {
        self.type_registry.keys().map(|s| s.as_str()).collect()
    }

    /// Clear validation cache
    pub fn clear_cache(&mut self) {
        self.validation_cache.clear();
        self.field_type_cache.clear();
        self.cache_access_counter.clear();
        self.global_access_counter = 0;
    }

    /// Maybe evict old cache entries using LRU strategy
    fn maybe_evict_cache(&mut self) {
        let current_size = self.validation_cache.len();
        let max_size = self.config.max_cache_size;
        let eviction_threshold = (max_size as f64 * self.config.cache_eviction_threshold) as usize;

        if current_size >= eviction_threshold {
            // Find LRU entries to evict
            let mut access_times: Vec<_> = self
                .cache_access_counter
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect();
            access_times.sort_by_key(|(_, access_time)| *access_time);

            // Remove oldest 25% of entries
            let num_to_remove = max_size / 4;
            for (cache_key, _) in access_times.into_iter().take(num_to_remove) {
                self.validation_cache.remove(&cache_key);
                self.field_type_cache.remove(&cache_key);
                self.cache_access_counter.remove(&cache_key);
            }
        }
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize, usize, f64) {
        let hit_ratio = if self.global_access_counter > 0 {
            (self.validation_cache.len() + self.field_type_cache.len()) as f64
                / self.global_access_counter as f64
        } else {
            0.0
        };

        (
            self.validation_cache.len(),
            self.field_type_cache.len(),
            self.type_registry.len(),
            hit_ratio,
        )
    }

    /// Check if field validation is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enable_runtime_validation || self.config.enable_static_analysis
    }

    /// Get configuration
    pub fn config(&self) -> &FieldValidationConfig {
        &self.config
    }
}

impl Default for FieldValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for field validation

/// Create type info for a struct
pub fn create_struct_type_info(
    name: String,
    fields: Vec<(String, Type, u32)>, // (name, type, offset)
) -> TypeInfo {
    let field_map = fields
        .into_iter()
        .map(|(field_name, field_type, offset)| {
            let field_info = FieldInfo {
                name: field_name.clone(),
                field_type,
                offset,
                is_public: true,
                is_mutable: true,
            };
            (field_name, field_info)
        })
        .collect();

    TypeInfo {
        name,
        fields: field_map,
        is_generic: false,
        base_types: Vec::new(),
    }
}

/// Create type info for an enum
pub fn create_enum_type_info(name: String, variants: Vec<String>) -> TypeInfo {
    let field_map = variants
        .into_iter()
        .enumerate()
        .map(|(index, variant_name)| {
            let field_info = FieldInfo {
                name: variant_name.clone(),
                field_type: Type::I32, // Enum discriminant
                offset: index as u32,
                is_public: true,
                is_mutable: false, // Enum variants are immutable
            };
            (variant_name, field_info)
        })
        .collect();

    TypeInfo {
        name,
        fields: field_map,
        is_generic: false,
        base_types: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_type() -> TypeInfo {
        create_struct_type_info(
            "TestStruct".to_string(),
            vec![
                ("x".to_string(), Type::I32, 0),
                ("y".to_string(), Type::F32, 4),
                ("name".to_string(), Type::String, 8),
            ],
        )
    }

    #[test]
    fn test_field_validator_creation() {
        let validator = FieldValidator::new();
        assert!(validator.is_enabled());
    }

    #[test]
    fn test_type_registration() {
        let mut validator = FieldValidator::new();
        let type_info = create_test_type();

        validator.register_type(type_info);
        assert!(validator.get_type_info("TestStruct").is_some());
    }

    #[test]
    fn test_valid_field_access() {
        let mut validator = FieldValidator::new();
        validator.register_type(create_test_type());

        let result = validator.validate_field_access("TestStruct", "x");
        assert!(matches!(result, FieldValidationResult::Valid { .. }));
    }

    #[test]
    fn test_invalid_field_access() {
        let mut validator = FieldValidator::new();
        validator.register_type(create_test_type());

        let result = validator.validate_field_access("TestStruct", "invalid_field");
        assert!(matches!(result, FieldValidationResult::InvalidField { .. }));
    }

    #[test]
    fn test_unknown_type_access() {
        let mut validator = FieldValidator::new();

        let result = validator.validate_field_access("UnknownType", "x");
        assert!(matches!(result, FieldValidationResult::InvalidField { .. }));
    }

    #[test]
    fn test_field_validation_caching() {
        let mut validator = FieldValidator::new();
        validator.register_type(create_test_type());

        // First call should populate cache
        let _result1 = validator.validate_field_access("TestStruct", "x");
        let (cache_size, _, _, _) = validator.cache_stats();
        assert_eq!(cache_size, 1);

        // Second call should use cache
        let _result2 = validator.validate_field_access("TestStruct", "x");
        let (cache_size, _, _, _) = validator.cache_stats();
        assert_eq!(cache_size, 1); // Should still be 1
    }

    #[test]
    fn test_runtime_field_validation_success() {
        let mut validator = FieldValidator::new();
        validator.register_type(create_test_type());

        let result = validator.runtime_field_validation("TestStruct", "x", "test");
        assert!(result.is_ok());

        let field_info = result.unwrap();
        assert_eq!(field_info.name, "x");
        assert_eq!(field_info.field_type, Type::I32);
    }

    #[test]
    fn test_runtime_field_validation_failure() {
        let mut validator = FieldValidator::new();
        validator.register_type(create_test_type());

        let result = validator.runtime_field_validation("TestStruct", "invalid", "test");
        assert!(result.is_err());

        if let Err(SecurityError::InvalidFieldAccess { field_name, .. }) = result {
            assert_eq!(field_name, "invalid");
        } else {
            panic!("Expected InvalidFieldAccess error");
        }
    }

    #[test]
    fn test_generate_field_validation_instruction() {
        let validator = FieldValidator::new();
        let object = ValueId(1);
        let object_type = Type::Named("TestStruct".to_string());

        let result = validator.generate_field_validation(object, "x", &object_type, "test");
        assert!(result.is_ok());

        if let Ok(Instruction::ValidateFieldAccess {
            object: val_object,
            field_name,
            ..
        }) = result
        {
            assert_eq!(val_object, object);
            assert_eq!(field_name, "x");
        } else {
            panic!("Expected ValidateFieldAccess instruction");
        }
    }

    #[test]
    fn test_secure_field_access_generation() {
        let mut validator = FieldValidator::new();
        validator.register_type(create_test_type());

        let object = ValueId(1);
        let object_type = Type::Named("TestStruct".to_string());

        let result = validator.generate_secure_field_access(object, "x", &object_type, "test");
        assert!(result.is_ok());

        let instructions = result.unwrap();
        assert_eq!(instructions.len(), 2); // Validation + LoadField

        // First instruction should be validation
        assert!(matches!(
            instructions[0],
            Instruction::ValidateFieldAccess { .. }
        ));
        // Second instruction should be field access
        assert!(matches!(instructions[1], Instruction::LoadField { .. }));
    }

    #[test]
    fn test_non_strict_mode() {
        let config = FieldValidationConfig {
            strict_mode: false,
            ..Default::default()
        };
        let mut validator = FieldValidator::with_config(config);
        validator.register_type(create_test_type());

        // Should allow unknown fields in non-strict mode
        let result = validator.validate_field_access("TestStruct", "unknown_field");
        assert!(matches!(result, FieldValidationResult::Valid { .. }));
    }

    #[test]
    fn test_create_struct_type_info() {
        let type_info = create_struct_type_info(
            "Point".to_string(),
            vec![
                ("x".to_string(), Type::I32, 0),
                ("y".to_string(), Type::I32, 4),
            ],
        );

        assert_eq!(type_info.name, "Point");
        assert_eq!(type_info.fields.len(), 2);
        assert!(type_info.fields.contains_key("x"));
        assert!(type_info.fields.contains_key("y"));
    }

    #[test]
    fn test_create_enum_type_info() {
        let type_info = create_enum_type_info(
            "Color".to_string(),
            vec!["Red".to_string(), "Green".to_string(), "Blue".to_string()],
        );

        assert_eq!(type_info.name, "Color");
        assert_eq!(type_info.fields.len(), 3);
        assert!(type_info.fields.contains_key("Red"));
        assert!(type_info.fields.contains_key("Green"));
        assert!(type_info.fields.contains_key("Blue"));
    }
}
