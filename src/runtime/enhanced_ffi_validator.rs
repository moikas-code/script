//! Enhanced FFI validation with extended security patterns
//!
//! This module provides comprehensive security validation for FFI calls
//! with expanded dangerous pattern detection, platform-specific checks,
//! and audit logging capabilities.

use crate::runtime::value::Value;
use crate::security::SecurityError;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Enhanced FFI validator with comprehensive security checks
#[derive(Debug)]
pub struct EnhancedFFIValidator {
    security_manager: SecurityManager,
    call_auditor: FFICallAuditor,
    platform_validator: PlatformValidator,
    dangerous_patterns: HashSet<String>,
    allowed_functions: HashSet<String>,
    rate_limiter: RateLimiter,
}

/// Security manager for FFI operations
#[derive(Debug)]
pub struct SecurityManager {
    max_call_rate: u64,
    max_argument_size: usize,
    max_string_length: usize,
    timeout_duration: Duration,
}

/// FFI call auditor for security logging
#[derive(Debug)]
pub struct FFICallAuditor {
    audit_log: Arc<Mutex<Vec<AuditEntry>>>,
    max_log_entries: usize,
}

/// Platform-specific FFI security validator
#[derive(Debug)]
pub struct PlatformValidator {
    platform: Platform,
    restricted_symbols: HashSet<String>,
    abi_validators: HashMap<String, Box<dyn ABIValidator>>,
}

/// Rate limiter for FFI calls
#[derive(Debug)]
pub struct RateLimiter {
    call_counts: Arc<Mutex<HashMap<String, CallCounter>>>,
    global_limit: u64,
    per_function_limit: u64,
    time_window: Duration,
}

/// Call counter for rate limiting
#[derive(Debug, Clone)]
struct CallCounter {
    count: u64,
    window_start: Instant,
}

/// Audit entry for FFI call logging
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub timestamp: Instant,
    pub function_name: String,
    pub argument_count: usize,
    pub argument_sizes: Vec<usize>,
    pub allowed: bool,
    pub rejection_reason: Option<String>,
    pub execution_time: Option<Duration>,
}

/// Platform detection
#[derive(Debug, Clone, PartialEq)]
pub enum Platform {
    Linux,
    Windows,
    MacOS,
    Unknown,
}

/// ABI validator trait for platform-specific validation
pub trait ABIValidator: Send + Sync + std::fmt::Debug {
    fn validate_call(&self, function_name: &str, args: &[Value]) -> Result<(), SecurityError>;
    fn validate_return_value(&self, value: &Value) -> Result<Value, SecurityError>;
}

/// Result of FFI call validation
#[derive(Debug, Clone, PartialEq)]
pub enum FFICallPermission {
    Allowed,
    Denied(String),
    AllowedWithWarning(String),
}

impl EnhancedFFIValidator {
    /// Create a new enhanced FFI validator
    pub fn new() -> Self {
        let security_manager = SecurityManager::new();
        let call_auditor = FFICallAuditor::new();
        let platform_validator = PlatformValidator::new();
        let rate_limiter = RateLimiter::new();

        let mut dangerous_patterns = HashSet::new();
        Self::populate_dangerous_patterns(&mut dangerous_patterns);

        let mut allowed_functions = HashSet::new();
        Self::populate_allowed_functions(&mut allowed_functions);

        EnhancedFFIValidator {
            security_manager,
            call_auditor,
            platform_validator,
            dangerous_patterns,
            allowed_functions,
            rate_limiter,
        }
    }

    /// Validate FFI call with comprehensive security checks
    pub fn validate_ffi_call(
        &mut self,
        function_name: &str,
        args: &[Value],
        context: &FFIContext,
    ) -> Result<FFICallPermission, SecurityError> {
        let start_time = Instant::now();

        // Rate limiting check
        self.rate_limiter.check_rate_limit(function_name)?;

        // Basic security validation
        self.validate_function_security(function_name)?;

        // Argument validation
        self.validate_arguments(function_name, args)?;

        // Platform-specific validation
        self.platform_validator.validate_call(function_name, args)?;

        // Security manager validation
        self.security_manager.validate_call(function_name, args)?;

        let execution_time = start_time.elapsed();

        // Create audit entry
        let audit_entry = AuditEntry {
            timestamp: start_time,
            function_name: function_name.to_string(),
            argument_count: args.len(),
            argument_sizes: args.iter().map(|arg| self.estimate_value_size(arg)).collect(),
            allowed: true,
            rejection_reason: None,
            execution_time: Some(execution_time),
        };

        self.call_auditor.log_call(audit_entry);

        // Check for warnings
        if self.has_security_warnings(function_name, args) {
            Ok(FFICallPermission::AllowedWithWarning(
                format!("Function '{}' has potential security implications", function_name)
            ))
        } else {
            Ok(FFICallPermission::Allowed)
        }
    }

    /// Validate function name against security patterns
    fn validate_function_security(&self, function_name: &str) -> Result<(), SecurityError> {
        // Check against dangerous patterns
        for pattern in &self.dangerous_patterns {
            if function_name.contains(pattern) {
                return Err(SecurityError::DangerousFFIFunction(
                    format!("Function '{}' matches dangerous pattern '{}'", function_name, pattern)
                ));
            }
        }

        // Check function name length to prevent buffer overflow attacks
        if function_name.len() > 64 {
            return Err(SecurityError::InvalidFFIFunction(
                "Function name too long".to_string()
            ));
        }

        // Check for null bytes (C string injection)
        if function_name.contains('\0') {
            return Err(SecurityError::InvalidFFIFunction(
                "Function name contains null bytes".to_string()
            ));
        }

        // Check for control characters
        if function_name.chars().any(|c| c.is_control()) {
            return Err(SecurityError::InvalidFFIFunction(
                "Function name contains control characters".to_string()
            ));
        }

        Ok(())
    }

    /// Validate function arguments
    fn validate_arguments(&self, function_name: &str, args: &[Value]) -> Result<(), SecurityError> {
        // Argument count validation
        if args.len() > 16 {
            return Err(SecurityError::TooManyFFIArguments {
                count: args.len(),
                max_allowed: 16,
            });
        }

        // Validate each argument
        for (i, arg) in args.iter().enumerate() {
            self.validate_single_argument(function_name, i, arg)?;
        }

        Ok(())
    }

    /// Validate a single argument
    fn validate_single_argument(
        &self,
        function_name: &str,
        arg_index: usize,
        arg: &Value,
    ) -> Result<(), SecurityError> {
        match arg {
            Value::String(s) => {
                // Check string length to prevent buffer overflow
                if s.len() > self.security_manager.max_string_length {
                    return Err(SecurityError::FFIArgumentTooLarge {
                        function_name: function_name.to_string(),
                        arg_index,
                        size: s.len(),
                        max_size: self.security_manager.max_string_length,
                    });
                }

                // Check for format string vulnerabilities
                if s.contains('%') && self.is_format_function(function_name) {
                    return Err(SecurityError::FormatStringVulnerability {
                        function_name: function_name.to_string(),
                        format_string: s.clone(),
                    });
                }

                // Check for path traversal in file functions
                if self.is_file_function(function_name) && (s.contains("..") || s.contains('\0')) {
                    return Err(SecurityError::PathTraversalAttempt {
                        function_name: function_name.to_string(),
                        path: s.clone(),
                    });
                }
            }
            Value::Array(arr) => {
                // Check array size
                if arr.len() > 1000 {
                    return Err(SecurityError::FFIArgumentTooLarge {
                        function_name: function_name.to_string(),
                        arg_index,
                        size: arr.len(),
                        max_size: 1000,
                    });
                }

                // Validate array elements recursively
                for (elem_index, elem) in arr.iter().enumerate() {
                    self.validate_single_argument(
                        &format!("{}[{}]", function_name, elem_index),
                        arg_index,
                        elem,
                    )?;
                }
            }
            _ => {
                // Other value types are generally safe
            }
        }

        Ok(())
    }

    /// Check if function has security warnings
    fn has_security_warnings(&self, function_name: &str, _args: &[Value]) -> bool {
        // Functions that are allowed but potentially risky
        const RISKY_FUNCTIONS: &[&str] = &[
            "system", "exec", "eval", "open", "fopen", "socket", "connect",
            "malloc", "realloc", "memcpy", "strcpy", "strcat"
        ];

        RISKY_FUNCTIONS.iter().any(|&risky| function_name.contains(risky))
    }

    /// Check if function is a format function
    fn is_format_function(&self, function_name: &str) -> bool {
        const FORMAT_FUNCTIONS: &[&str] = &[
            "printf", "sprintf", "snprintf", "fprintf", "scanf", "sscanf", "fscanf"
        ];

        FORMAT_FUNCTIONS.iter().any(|&fmt_func| function_name.contains(fmt_func))
    }

    /// Check if function is a file function
    fn is_file_function(&self, function_name: &str) -> bool {
        const FILE_FUNCTIONS: &[&str] = &[
            "open", "fopen", "read", "write", "fread", "fwrite", "access", "stat"
        ];

        FILE_FUNCTIONS.iter().any(|&file_func| function_name.contains(file_func))
    }

    /// Estimate the size of a value for security validation
    fn estimate_value_size(&self, value: &Value) -> usize {
        match value {
            Value::String(s) => s.len(),
            Value::Array(arr) => arr.len() * 8, // Rough estimate
            Value::I32(_) => 4,
            Value::I64(_) => 8,
            Value::F32(_) => 4,
            Value::F64(_) => 8,
            Value::Bool(_) => 1,
            _ => 8, // Default estimate
        }
    }

    /// Populate dangerous function patterns
    fn populate_dangerous_patterns(patterns: &mut HashSet<String>) {
        // System execution functions
        patterns.insert("system".to_string());
        patterns.insert("exec".to_string());
        patterns.insert("popen".to_string());
        patterns.insert("fork".to_string());
        patterns.insert("vfork".to_string());
        patterns.insert("clone".to_string());

        // Memory manipulation functions
        patterns.insert("malloc".to_string());
        patterns.insert("free".to_string());
        patterns.insert("realloc".to_string());
        patterns.insert("calloc".to_string());

        // Dangerous string functions
        patterns.insert("gets".to_string());
        patterns.insert("strcpy".to_string());
        patterns.insert("strcat".to_string());
        patterns.insert("sprintf".to_string());
        patterns.insert("scanf".to_string());

        // Dynamic loading functions
        patterns.insert("dlopen".to_string());
        patterns.insert("dlsym".to_string());
        patterns.insert("LoadLibrary".to_string());
        patterns.insert("GetProcAddress".to_string());

        // Process debugging/introspection
        patterns.insert("ptrace".to_string());
        patterns.insert("DebugActiveProcess".to_string());

        // Network functions
        patterns.insert("socket".to_string());
        patterns.insert("bind".to_string());
        patterns.insert("listen".to_string());
        patterns.insert("accept".to_string());

        // File system manipulation
        patterns.insert("unlink".to_string());
        patterns.insert("rmdir".to_string());
        patterns.insert("chmod".to_string());
        patterns.insert("chown".to_string());
    }

    /// Populate allowed safe functions
    fn populate_allowed_functions(functions: &mut HashSet<String>) {
        // Safe string functions
        functions.insert("strlen".to_string());
        functions.insert("strncmp".to_string());
        functions.insert("strncpy".to_string());
        functions.insert("strncat".to_string());

        // Safe math functions
        functions.insert("sin".to_string());
        functions.insert("cos".to_string());
        functions.insert("tan".to_string());
        functions.insert("sqrt".to_string());
        functions.insert("pow".to_string());
        functions.insert("log".to_string());

        // Safe memory functions
        functions.insert("memset".to_string());
        functions.insert("memcmp".to_string());

        // Safe I/O functions (with validation)
        functions.insert("puts".to_string());
        functions.insert("putchar".to_string());
    }
}

impl SecurityManager {
    fn new() -> Self {
        SecurityManager {
            max_call_rate: 10_000,
            max_argument_size: 1_000_000, // 1MB
            max_string_length: 10_000,
            timeout_duration: Duration::from_secs(5),
        }
    }

    fn validate_call(&self, function_name: &str, args: &[Value]) -> Result<(), SecurityError> {
        // Validate total argument size
        let total_size: usize = args.iter()
            .map(|arg| match arg {
                Value::String(s) => s.len(),
                Value::Array(arr) => arr.len() * 8,
                _ => 8,
            })
            .sum();

        if total_size > self.max_argument_size {
            return Err(SecurityError::FFIArgumentsTooLarge {
                function_name: function_name.to_string(),
                total_size,
                max_size: self.max_argument_size,
            });
        }

        Ok(())
    }
}

impl FFICallAuditor {
    fn new() -> Self {
        FFICallAuditor {
            audit_log: Arc::new(Mutex::new(Vec::new())),
            max_log_entries: 10_000,
        }
    }

    fn log_call(&self, entry: AuditEntry) {
        if let Ok(mut log) = self.audit_log.lock() {
            log.push(entry);

            // Rotate log if it gets too large
            if log.len() > self.max_log_entries {
                log.remove(0);
            }
        }
    }

    pub fn get_audit_log(&self) -> Vec<AuditEntry> {
        self.audit_log.lock().unwrap().clone()
    }
}

impl PlatformValidator {
    fn new() -> Self {
        let platform = Self::detect_platform();
        let restricted_symbols = Self::get_platform_restricted_symbols(&platform);
        let abi_validators = HashMap::new();

        PlatformValidator {
            platform,
            restricted_symbols,
            abi_validators,
        }
    }

    fn detect_platform() -> Platform {
        #[cfg(target_os = "linux")]
        return Platform::Linux;
        #[cfg(target_os = "windows")]
        return Platform::Windows;
        #[cfg(target_os = "macos")]
        return Platform::MacOS;
        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        return Platform::Unknown;
    }

    fn get_platform_restricted_symbols(platform: &Platform) -> HashSet<String> {
        let mut symbols = HashSet::new();

        match platform {
            Platform::Linux => {
                symbols.insert("__libc_start_main".to_string());
                symbols.insert("syscall".to_string());
            }
            Platform::Windows => {
                symbols.insert("NtCreateProcess".to_string());
                symbols.insert("ZwCreateProcess".to_string());
            }
            Platform::MacOS => {
                symbols.insert("_dyld_get_image_name".to_string());
            }
            Platform::Unknown => {
                // No platform-specific restrictions
            }
        }

        symbols
    }

    fn validate_call(&self, function_name: &str, _args: &[Value]) -> Result<(), SecurityError> {
        if self.restricted_symbols.contains(function_name) {
            return Err(SecurityError::PlatformRestrictedFunction {
                function_name: function_name.to_string(),
                platform: format!("{:?}", self.platform),
            });
        }

        Ok(())
    }
}

impl RateLimiter {
    fn new() -> Self {
        RateLimiter {
            call_counts: Arc::new(Mutex::new(HashMap::new())),
            global_limit: 10_000,
            per_function_limit: 1_000,
            time_window: Duration::from_secs(60),
        }
    }

    fn check_rate_limit(&self, function_name: &str) -> Result<(), SecurityError> {
        let now = Instant::now();

        if let Ok(mut counts) = self.call_counts.lock() {
            let counter = counts.entry(function_name.to_string())
                .or_insert_with(|| CallCounter {
                    count: 0,
                    window_start: now,
                });

            // Reset counter if time window has passed
            if now.duration_since(counter.window_start) > self.time_window {
                counter.count = 0;
                counter.window_start = now;
            }

            counter.count += 1;

            // Check per-function limit
            if counter.count > self.per_function_limit {
                return Err(SecurityError::FFIRateLimitExceeded {
                    function_name: function_name.to_string(),
                    current_rate: counter.count,
                    limit: self.per_function_limit,
                });
            }

            // Check global limit
            let total_calls: u64 = counts.values().map(|c| c.count).sum();
            if total_calls > self.global_limit {
                return Err(SecurityError::FFIGlobalRateLimitExceeded {
                    current_rate: total_calls,
                    limit: self.global_limit,
                });
            }
        }

        Ok(())
    }
}

/// FFI context for validation
#[derive(Debug)]
pub struct FFIContext {
    pub caller_location: String,
    pub security_level: SecurityLevel,
    pub allowed_operations: HashSet<String>,
}

/// Security level for FFI operations
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityLevel {
    Strict,
    Normal,
    Permissive,
}

impl Default for FFIContext {
    fn default() -> Self {
        FFIContext {
            caller_location: "unknown".to_string(),
            security_level: SecurityLevel::Normal,
            allowed_operations: HashSet::new(),
        }
    }
}

// Extend SecurityError with FFI-specific errors
impl SecurityError {
    pub fn DangerousFFIFunction(msg: String) -> Self {
        SecurityError::ResourceLimitExceeded(msg)
    }

    pub fn InvalidFFIFunction(msg: String) -> Self {
        SecurityError::ResourceLimitExceeded(msg)
    }

    pub fn TooManyFFIArguments { count: usize, max_allowed: usize } -> Self {
        SecurityError::ResourceLimitExceeded(format!(
            "Too many FFI arguments: {} (max: {})", count, max_allowed
        ))
    }

    pub fn FFIArgumentTooLarge { 
        function_name: String, 
        arg_index: usize, 
        size: usize, 
        max_size: usize 
    } -> Self {
        SecurityError::ResourceLimitExceeded(format!(
            "FFI argument {} for function '{}' too large: {} bytes (max: {})",
            arg_index, function_name, size, max_size
        ))
    }

    pub fn FFIArgumentsTooLarge { 
        function_name: String, 
        total_size: usize, 
        max_size: usize 
    } -> Self {
        SecurityError::ResourceLimitExceeded(format!(
            "Total FFI arguments for function '{}' too large: {} bytes (max: {})",
            function_name, total_size, max_size
        ))
    }

    pub fn FormatStringVulnerability { 
        function_name: String, 
        format_string: String 
    } -> Self {
        SecurityError::SecurityViolation(format!(
            "Format string vulnerability in function '{}': '{}'",
            function_name, format_string
        ))
    }

    pub fn PathTraversalAttempt { 
        function_name: String, 
        path: String 
    } -> Self {
        SecurityError::SecurityViolation(format!(
            "Path traversal attempt in function '{}': '{}'",
            function_name, path
        ))
    }

    pub fn PlatformRestrictedFunction { 
        function_name: String, 
        platform: String 
    } -> Self {
        SecurityError::SecurityViolation(format!(
            "Function '{}' is restricted on platform '{}'",
            function_name, platform
        ))
    }

    pub fn FFIRateLimitExceeded { 
        function_name: String, 
        current_rate: u64, 
        limit: u64 
    } -> Self {
        SecurityError::ResourceLimitExceeded(format!(
            "Rate limit exceeded for function '{}': {} calls (limit: {})",
            function_name, current_rate, limit
        ))
    }

    pub fn FFIGlobalRateLimitExceeded { 
        current_rate: u64, 
        limit: u64 
    } -> Self {
        SecurityError::ResourceLimitExceeded(format!(
            "Global FFI rate limit exceeded: {} calls (limit: {})",
            current_rate, limit
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dangerous_function_detection() {
        let mut validator = EnhancedFFIValidator::new();
        let context = FFIContext::default();

        // Test dangerous function rejection
        let result = validator.validate_ffi_call("system", &[], &context);
        assert!(result.is_err());

        // Test safe function approval  
        let result = validator.validate_ffi_call("strlen", &[], &context);
        assert!(result.is_ok());
    }

    #[test]
    fn test_argument_validation() {
        let mut validator = EnhancedFFIValidator::new();
        let context = FFIContext::default();

        // Test string length validation
        let long_string = "x".repeat(20_000);
        let args = vec![Value::String(long_string)];
        let result = validator.validate_ffi_call("strlen", &args, &context);
        assert!(result.is_err());

        // Test valid arguments
        let args = vec![Value::String("hello".to_string())];
        let result = validator.validate_ffi_call("strlen", &args, &context);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rate_limiting() {
        let mut validator = EnhancedFFIValidator::new();
        let context = FFIContext::default();

        // Simulate many calls to trigger rate limit
        for _ in 0..1500 {
            let _ = validator.validate_ffi_call("strlen", &[], &context);
        }

        // This call should fail due to rate limiting
        let result = validator.validate_ffi_call("strlen", &[], &context);
        assert!(result.is_err());
    }
}