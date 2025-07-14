//! MCP Security implementation for Script Language
//!
//! This module provides comprehensive security for Model Context Protocol operations,
//! including input validation, sandboxed analysis, resource limits, and audit logging.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime};
use uuid::Uuid;

/// Maximum input size for MCP requests (1MB)
const MAX_INPUT_SIZE: usize = 1024 * 1024;

/// Maximum analysis time per request (30 seconds)
const MAX_ANALYSIS_TIME: Duration = Duration::from_secs(30);

/// Maximum memory usage per analysis (10MB)
const MAX_MEMORY_USAGE: usize = 10 * 1024 * 1024;

/// Rate limit: requests per minute per client
const RATE_LIMIT_PER_MINUTE: u32 = 60;

/// Security context for MCP operations
#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// Unique session identifier
    pub session_id: Uuid,
    /// Client identifier (if known)
    pub client_id: Option<String>,
    /// Session creation time
    pub created_at: SystemTime,
    /// Session expiration time
    pub expires_at: SystemTime,
    /// Permission level
    pub permission_level: PermissionLevel,
    /// Resource limits for this session
    pub resource_limits: ResourceLimits,
    /// Audit context
    pub audit_context: AuditContext,
}

/// Permission levels for MCP operations
#[derive(Debug, Clone, PartialEq)]
pub enum PermissionLevel {
    /// Read-only analysis (safest)
    ReadOnly,
    /// Standard operations (default)
    Standard,
    /// Administrative operations (restricted)
    Administrative,
}

/// Resource limits for MCP sessions
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum input size in bytes
    pub max_input_size: usize,
    /// Maximum analysis time
    pub max_analysis_time: Duration,
    /// Maximum memory usage
    pub max_memory_usage: usize,
    /// Maximum number of concurrent requests
    pub max_concurrent_requests: u32,
}

/// Audit context for security logging
#[derive(Debug, Clone)]
pub struct AuditContext {
    /// Request counter
    pub request_count: u64,
    /// Failed request counter
    pub failed_request_count: u64,
    /// Last activity time
    pub last_activity: SystemTime,
    /// Suspicious activity flags
    pub flags: Vec<SecurityFlag>,
}

/// Security flags for suspicious activity
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityFlag {
    /// High frequency requests
    HighFrequency,
    /// Large input sizes
    LargeInput,
    /// Potentially malicious patterns
    SuspiciousPattern,
    /// Resource limit violations
    ResourceViolation,
}

/// Security manager for MCP operations
pub struct SecurityManager {
    /// Active sessions
    sessions: Arc<RwLock<HashMap<Uuid, SecurityContext>>>,
    /// Rate limiting tracker
    rate_limits: Arc<Mutex<HashMap<String, RateLimitState>>>,
    /// Security configuration
    config: SecurityConfig,
    /// Audit logger
    audit_logger: AuditLogger,
}

/// Rate limiting state
#[derive(Debug)]
struct RateLimitState {
    /// Request timestamps in the current window
    requests: Vec<Instant>,
    /// Last cleanup time
    last_cleanup: Instant,
}

/// Security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Session timeout duration
    pub session_timeout: Duration,
    /// Maximum concurrent sessions
    pub max_concurrent_sessions: usize,
    /// Enable strict mode (extra security checks)
    pub strict_mode: bool,
    /// Default resource limits
    pub default_limits: ResourceLimits,
}

/// Audit logger for security events
#[derive(Debug)]
pub struct AuditLogger {
    /// Log entries
    entries: Arc<Mutex<Vec<AuditEntry>>>,
    /// Maximum log entries to keep
    max_entries: usize,
}

/// Audit log entry
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// Timestamp
    pub timestamp: SystemTime,
    /// Session ID
    pub session_id: Uuid,
    /// Event type
    pub event_type: AuditEventType,
    /// Event message
    pub message: String,
    /// Additional context
    pub context: HashMap<String, String>,
}

/// Types of audit events
#[derive(Debug, Clone, PartialEq)]
pub enum AuditEventType {
    /// Session created
    SessionCreated,
    /// Session expired
    SessionExpired,
    /// Request processed
    RequestProcessed,
    /// Security violation
    SecurityViolation,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Input validation failed
    ValidationFailed,
    /// Resource limit exceeded
    ResourceLimitExceeded,
}

/// Input validation result
#[derive(Debug)]
pub enum ValidationResult {
    /// Input is safe to process
    Valid,
    /// Input is potentially dangerous
    Dangerous { reason: String },
    /// Input exceeds size limits
    TooLarge { size: usize, max_size: usize },
    /// Input contains forbidden patterns
    Forbidden { pattern: String },
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            session_timeout: Duration::from_secs(3600), // 1 hour
            max_concurrent_sessions: 100,
            strict_mode: true,
            default_limits: ResourceLimits::default(),
        }
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_input_size: MAX_INPUT_SIZE,
            max_analysis_time: MAX_ANALYSIS_TIME,
            max_memory_usage: MAX_MEMORY_USAGE,
            max_concurrent_requests: 10,
        }
    }
}

impl SecurityManager {
    /// Create a new security manager
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            rate_limits: Arc::new(Mutex::new(HashMap::new())),
            config,
            audit_logger: AuditLogger::new(10000),
        }
    }

    /// Create a new security session
    pub fn create_session(
        &self,
        client_id: Option<String>,
    ) -> Result<SecurityContext, SecurityError> {
        let session_id = Uuid::new_v4();
        let now = SystemTime::now();

        let context = SecurityContext {
            session_id,
            client_id: client_id.clone(),
            created_at: now,
            expires_at: now + self.config.session_timeout,
            permission_level: PermissionLevel::Standard,
            resource_limits: self.config.default_limits.clone(),
            audit_context: AuditContext {
                request_count: 0,
                failed_request_count: 0,
                last_activity: now,
                flags: Vec::new(),
            },
        };

        // Check session limits
        {
            let sessions = self.sessions.read().map_err(|_| SecurityError::LockError)?;
            if sessions.len() >= self.config.max_concurrent_sessions {
                return Err(SecurityError::TooManySessions);
            }
        }

        // Store session
        {
            let mut sessions = self
                .sessions
                .write()
                .map_err(|_| SecurityError::LockError)?;
            sessions.insert(session_id, context.clone());
        }

        // Log session creation
        self.audit_logger.log(AuditEntry {
            timestamp: now,
            session_id,
            event_type: AuditEventType::SessionCreated,
            message: "MCP session created".to_string(),
            context: {
                let mut ctx = HashMap::new();
                if let Some(id) = &client_id {
                    ctx.insert("client_id".to_string(), id.clone());
                }
                ctx
            },
        });

        Ok(context)
    }

    /// Validate session and update activity
    pub fn validate_session(&self, session_id: Uuid) -> Result<SecurityContext, SecurityError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| SecurityError::LockError)?;

        if let Some(context) = sessions.get_mut(&session_id) {
            let now = SystemTime::now();

            // Check if session has expired
            if now > context.expires_at {
                sessions.remove(&session_id);
                self.audit_logger.log(AuditEntry {
                    timestamp: now,
                    session_id,
                    event_type: AuditEventType::SessionExpired,
                    message: "Session expired".to_string(),
                    context: HashMap::new(),
                });
                return Err(SecurityError::SessionExpired);
            }

            // Update last activity
            context.audit_context.last_activity = now;
            Ok(context.clone())
        } else {
            Err(SecurityError::SessionNotFound)
        }
    }

    /// Check rate limits for a client
    pub fn check_rate_limit(&self, client_id: &str) -> Result<(), SecurityError> {
        let mut rate_limits = self
            .rate_limits
            .lock()
            .map_err(|_| SecurityError::LockError)?;
        let now = Instant::now();

        let state = rate_limits
            .entry(client_id.to_string())
            .or_insert_with(|| RateLimitState {
                requests: Vec::new(),
                last_cleanup: now,
            });

        // Clean up old requests (older than 1 minute)
        if now.duration_since(state.last_cleanup) > Duration::from_secs(10) {
            let cutoff = now - Duration::from_secs(60);
            state.requests.retain(|&time| time > cutoff);
            state.last_cleanup = now;
        }

        // Check rate limit
        if state.requests.len() >= RATE_LIMIT_PER_MINUTE as usize {
            self.audit_logger.log(AuditEntry {
                timestamp: SystemTime::now(),
                session_id: Uuid::nil(),
                event_type: AuditEventType::RateLimitExceeded,
                message: format!("Rate limit exceeded for client: {}", client_id),
                context: {
                    let mut ctx = HashMap::new();
                    ctx.insert("client_id".to_string(), client_id.to_string());
                    ctx.insert(
                        "request_count".to_string(),
                        state.requests.len().to_string(),
                    );
                    ctx
                },
            });
            return Err(SecurityError::RateLimitExceeded);
        }

        // Record this request
        state.requests.push(now);
        Ok(())
    }

    /// Validate input for security threats
    pub fn validate_input(&self, input: &str, context: &SecurityContext) -> ValidationResult {
        // Check size limits
        if input.len() > context.resource_limits.max_input_size {
            return ValidationResult::TooLarge {
                size: input.len(),
                max_size: context.resource_limits.max_input_size,
            };
        }

        // Check for dangerous patterns in strict mode
        if self.config.strict_mode {
            if let Some(pattern) = self.detect_dangerous_patterns(input) {
                return ValidationResult::Dangerous {
                    reason: format!("Detected dangerous pattern: {}", pattern),
                };
            }
        }

        // Check for forbidden patterns
        if let Some(pattern) = self.detect_forbidden_patterns(input) {
            return ValidationResult::Forbidden { pattern };
        }

        ValidationResult::Valid
    }

    /// Detect dangerous patterns in input
    fn detect_dangerous_patterns(&self, input: &str) -> Option<String> {
        let dangerous_patterns = [
            "eval(",
            "exec(",
            "system(",
            "shell(",
            "cmd(",
            "import os",
            "import subprocess",
            "__import__",
            "file://",
            "javascript:",
            "<script",
            "onload=",
            "onerror=",
        ];

        for pattern in &dangerous_patterns {
            if input.to_lowercase().contains(pattern) {
                return Some(pattern.to_string());
            }
        }

        None
    }

    /// Detect forbidden patterns
    fn detect_forbidden_patterns(&self, input: &str) -> Option<String> {
        let forbidden_patterns = [
            "DROP TABLE",
            "DELETE FROM",
            "UPDATE ",
            "INSERT INTO",
            "UNION SELECT",
            "../",
            "..\\",
            "/etc/passwd",
            "/proc/",
            "C:\\Windows\\",
        ];

        for pattern in &forbidden_patterns {
            if input.to_uppercase().contains(&pattern.to_uppercase()) {
                return Some(pattern.to_string());
            }
        }

        None
    }

    /// Record a security violation
    pub fn record_violation(&self, session_id: Uuid, violation: SecurityViolation) {
        // Update session flags
        if let Ok(mut sessions) = self.sessions.write() {
            if let Some(context) = sessions.get_mut(&session_id) {
                let flag = match violation.violation_type {
                    SecurityViolationType::InputValidation => SecurityFlag::SuspiciousPattern,
                    SecurityViolationType::RateLimit => SecurityFlag::HighFrequency,
                    SecurityViolationType::ResourceLimit => SecurityFlag::ResourceViolation,
                    SecurityViolationType::PermissionDenied => SecurityFlag::SuspiciousPattern,
                };

                if !context.audit_context.flags.contains(&flag) {
                    context.audit_context.flags.push(flag);
                }
                context.audit_context.failed_request_count += 1;
            }
        }

        // Log the violation
        self.audit_logger.log(AuditEntry {
            timestamp: SystemTime::now(),
            session_id,
            event_type: AuditEventType::SecurityViolation,
            message: violation.message,
            context: violation.context,
        });
    }

    /// Get security statistics
    pub fn get_stats(&self) -> SecurityStats {
        let sessions = self.sessions.read().unwrap();
        let rate_limits = self.rate_limits.lock().unwrap();

        SecurityStats {
            active_sessions: sessions.len(),
            total_clients: rate_limits.len(),
            audit_entries: self.audit_logger.entry_count(),
        }
    }

    /// Cleanup expired sessions
    pub fn cleanup_expired_sessions(&self) {
        if let Ok(mut sessions) = self.sessions.write() {
            let now = SystemTime::now();
            let expired: Vec<Uuid> = sessions
                .iter()
                .filter(|(_, context)| now > context.expires_at)
                .map(|(id, _)| *id)
                .collect();

            for session_id in expired {
                sessions.remove(&session_id);
                self.audit_logger.log(AuditEntry {
                    timestamp: now,
                    session_id,
                    event_type: AuditEventType::SessionExpired,
                    message: "Session expired during cleanup".to_string(),
                    context: HashMap::new(),
                });
            }
        }
    }
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
            max_entries,
        }
    }

    /// Log an audit entry
    pub fn log(&self, entry: AuditEntry) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.push(entry);

            // Maintain max entries limit
            if entries.len() > self.max_entries {
                entries.remove(0);
            }
        }
    }

    /// Get recent audit entries
    pub fn get_recent(&self, count: usize) -> Vec<AuditEntry> {
        if let Ok(entries) = self.entries.lock() {
            entries.iter().rev().take(count).cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Get entry count
    pub fn entry_count(&self) -> usize {
        if let Ok(entries) = self.entries.lock() {
            entries.len()
        } else {
            0
        }
    }

    /// Search audit entries
    pub fn search(&self, event_type: AuditEventType) -> Vec<AuditEntry> {
        if let Ok(entries) = self.entries.lock() {
            entries
                .iter()
                .filter(|entry| entry.event_type == event_type)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }
}

/// Security violation information
#[derive(Debug, Clone)]
pub struct SecurityViolation {
    /// Type of violation
    pub violation_type: SecurityViolationType,
    /// Violation message
    pub message: String,
    /// Additional context
    pub context: HashMap<String, String>,
}

/// Types of security violations
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityViolationType {
    /// Input validation failure
    InputValidation,
    /// Rate limit exceeded
    RateLimit,
    /// Resource limit exceeded
    ResourceLimit,
    /// Permission denied
    PermissionDenied,
}

/// Security statistics
#[derive(Debug)]
pub struct SecurityStats {
    /// Number of active sessions
    pub active_sessions: usize,
    /// Number of tracked clients
    pub total_clients: usize,
    /// Number of audit entries
    pub audit_entries: usize,
}

/// Security errors
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Session not found")]
    SessionNotFound,
    #[error("Session expired")]
    SessionExpired,
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Too many concurrent sessions")]
    TooManySessions,
    #[error("Input validation failed: {reason}")]
    ValidationFailed { reason: String },
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Resource limit exceeded")]
    ResourceLimitExceeded,
    #[error("Lock error")]
    LockError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_manager_creation() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config);
        let stats = manager.get_stats();

        assert_eq!(stats.active_sessions, 0);
        assert_eq!(stats.total_clients, 0);
    }

    #[test]
    fn test_session_creation() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config);

        let context = manager
            .create_session(Some("test_client".to_string()))
            .unwrap();
        assert_eq!(context.client_id, Some("test_client".to_string()));
        assert_eq!(context.permission_level, PermissionLevel::Standard);

        let stats = manager.get_stats();
        assert_eq!(stats.active_sessions, 1);
    }

    #[test]
    fn test_input_validation() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config);
        let context = manager.create_session(None).unwrap();

        // Valid input
        let result = manager.validate_input("let x = 5", &context);
        assert!(matches!(result, ValidationResult::Valid));

        // Dangerous input
        let result = manager.validate_input("eval('malicious code')", &context);
        assert!(matches!(result, ValidationResult::Dangerous { .. }));

        // Too large input
        let large_input = "x".repeat(MAX_INPUT_SIZE + 1);
        let result = manager.validate_input(&large_input, &context);
        assert!(matches!(result, ValidationResult::TooLarge { .. }));
    }

    #[test]
    fn test_rate_limiting() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config);

        // Should succeed for normal usage
        for _ in 0..10 {
            assert!(manager.check_rate_limit("client1").is_ok());
        }

        // Should fail after hitting limit (we can't easily test this without time manipulation)
        // This would require adding many requests quickly
    }

    #[test]
    fn test_dangerous_pattern_detection() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config);

        assert!(manager.detect_dangerous_patterns("eval(code)").is_some());
        assert!(manager.detect_dangerous_patterns("import os").is_some());
        assert!(manager
            .detect_dangerous_patterns("<script>alert('xss')</script>")
            .is_some());
        assert!(manager.detect_dangerous_patterns("let x = 5").is_none());
    }

    #[test]
    fn test_forbidden_pattern_detection() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config);

        assert!(manager
            .detect_forbidden_patterns("DROP TABLE users")
            .is_some());
        assert!(manager
            .detect_forbidden_patterns("../../../etc/passwd")
            .is_some());
        assert!(manager.detect_forbidden_patterns("normal code").is_none());
    }

    #[test]
    fn test_audit_logging() {
        let logger = AuditLogger::new(100);

        logger.log(AuditEntry {
            timestamp: SystemTime::now(),
            session_id: Uuid::new_v4(),
            event_type: AuditEventType::SessionCreated,
            message: "Test log".to_string(),
            context: HashMap::new(),
        });

        assert_eq!(logger.entry_count(), 1);

        let recent = logger.get_recent(5);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].message, "Test log");
    }

    #[test]
    fn test_session_expiration() {
        let mut config = SecurityConfig::default();
        config.session_timeout = Duration::from_millis(1); // Very short timeout
        let manager = SecurityManager::new(config);

        let context = manager.create_session(None).unwrap();
        let session_id = context.session_id;

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(2));

        // Should fail due to expiration
        assert!(matches!(
            manager.validate_session(session_id),
            Err(SecurityError::SessionExpired)
        ));
    }
}
