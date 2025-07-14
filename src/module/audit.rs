//! Security audit logging for module operations
//!
//! This module provides comprehensive audit logging for security-relevant
//! module operations to enable monitoring and forensic analysis.

use crate::module::{ModuleError, ModulePath, ModuleResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// Security audit logger for module operations
#[derive(Debug)]
pub struct SecurityAuditLogger {
    /// Log file writer
    writer: Arc<Mutex<Option<BufWriter<File>>>>,
    /// Configuration
    config: AuditConfig,
    /// In-memory buffer for recent events
    event_buffer: Arc<Mutex<Vec<SecurityAuditEvent>>>,
    /// Event statistics
    stats: Arc<Mutex<AuditStatistics>>,
}

/// Audit logger configuration
#[derive(Debug, Clone)]
pub struct AuditConfig {
    /// Log file path
    pub log_file: PathBuf,
    /// Minimum severity to log
    pub severity_filter: SecuritySeverity,
    /// Enable real-time alerts for critical events
    pub real_time_alerts: bool,
    /// Maximum events in memory buffer
    pub buffer_size: usize,
    /// Rotate log when it reaches this size
    pub max_file_size: u64,
    /// Include full stack traces
    pub include_stack_traces: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        AuditConfig {
            log_file: PathBuf::from("script_security_audit.log"),
            severity_filter: SecuritySeverity::Warning,
            real_time_alerts: true,
            buffer_size: 1000,
            max_file_size: 100_000_000, // 100MB
            include_stack_traces: false,
        }
    }
}

/// Security event severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SecuritySeverity {
    /// Informational events
    Info,
    /// Warning conditions
    Warning,
    /// Errors that don't compromise security
    Error,
    /// Critical security violations
    Critical,
    /// Emergency - system compromise detected
    Emergency,
}

impl std::fmt::Display for SecuritySeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecuritySeverity::Info => write!(f, "INFO"),
            SecuritySeverity::Warning => write!(f, "WARNING"),
            SecuritySeverity::Error => write!(f, "ERROR"),
            SecuritySeverity::Critical => write!(f, "CRITICAL"),
            SecuritySeverity::Emergency => write!(f, "EMERGENCY"),
        }
    }
}

/// Security audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditEvent {
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event severity
    pub severity: SecuritySeverity,
    /// Event category
    pub category: SecurityEventCategory,
    /// Module involved
    pub module: Option<ModulePath>,
    /// Event description
    pub description: String,
    /// Additional context
    pub context: SecurityEventContext,
    /// Stack trace if available
    pub stack_trace: Option<String>,
}

/// Categories of security events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEventCategory {
    /// Path traversal attempts
    PathTraversal,
    /// Module integrity violations
    IntegrityViolation,
    /// Resource exhaustion attempts
    ResourceExhaustion,
    /// Permission violations
    PermissionDenied,
    /// Suspicious patterns detected
    SuspiciousActivity,
    /// Module loading events
    ModuleLoad,
    /// Configuration changes
    ConfigurationChange,
    /// Authentication/authorization
    Authentication,
}

/// Additional context for security events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEventContext {
    /// User or process ID
    pub user_id: Option<String>,
    /// Source IP if applicable
    pub source_ip: Option<String>,
    /// File path involved
    pub file_path: Option<PathBuf>,
    /// Operation that triggered the event
    pub operation: Option<String>,
    /// Error details
    pub error: Option<String>,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

/// Audit statistics
#[derive(Debug, Default)]
pub struct AuditStatistics {
    /// Total events by severity
    pub events_by_severity: std::collections::HashMap<SecuritySeverity, u64>,
    /// Total events by category
    pub events_by_category: std::collections::HashMap<String, u64>,
    /// Last critical event time
    pub last_critical_event: Option<SystemTime>,
    /// Total events logged
    pub total_events: u64,
}

impl SecurityAuditLogger {
    /// Create a new security audit logger
    pub fn new(config: AuditConfig) -> ModuleResult<Self> {
        // Open or create log file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config.log_file)
            .map_err(|e| ModuleError::io_error(format!("Failed to open audit log: {e}")))?;

        let writer = BufWriter::new(file);

        Ok(SecurityAuditLogger {
            writer: Arc::new(Mutex::new(Some(writer))),
            config,
            event_buffer: Arc::new(Mutex::new(Vec::new())),
            stats: Arc::new(Mutex::new(AuditStatistics::default())),
        })
    }

    /// Log a security event
    pub fn log_event(&self, event: SecurityAuditEvent) -> ModuleResult<()> {
        // Check severity filter
        if event.severity < self.config.severity_filter {
            return Ok(());
        }

        // Update statistics
        self.update_statistics(&event);

        // Real-time alert for critical events
        if self.config.real_time_alerts && event.severity >= SecuritySeverity::Critical {
            self.send_alert(&event)?;
        }

        // Add to memory buffer
        self.buffer_event(event.clone())?;

        // Write to log file
        self.write_event(&event)?;

        Ok(())
    }

    /// Log a path traversal attempt
    pub fn log_path_traversal(
        &self,
        module: Option<ModulePath>,
        attempted_path: &str,
        error: &ModuleError,
    ) -> ModuleResult<()> {
        let event = SecurityAuditEvent {
            timestamp: Utc::now(),
            severity: SecuritySeverity::Critical,
            category: SecurityEventCategory::PathTraversal,
            module,
            description: format!("Path traversal attempt detected: {attempted_path}"),
            context: SecurityEventContext {
                user_id: None,
                source_ip: None,
                file_path: Some(PathBuf::from(attempted_path)),
                operation: Some("module_load".to_string()),
                error: Some(error.to_string()),
                metadata: std::collections::HashMap::new(),
            },
            stack_trace: if self.config.include_stack_traces {
                Some(Self::capture_stack_trace())
            } else {
                None
            },
        };

        self.log_event(event)
    }

    /// Log an integrity violation
    pub fn log_integrity_violation(
        &self,
        module: ModulePath,
        expected_hash: &str,
        actual_hash: &str,
    ) -> ModuleResult<()> {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("expected_hash".to_string(), expected_hash.to_string());
        metadata.insert("actual_hash".to_string(), actual_hash.to_string());

        let event = SecurityAuditEvent {
            timestamp: Utc::now(),
            severity: SecuritySeverity::Critical,
            category: SecurityEventCategory::IntegrityViolation,
            module: Some(module.clone()),
            description: format!("Module {} failed integrity check", module),
            context: SecurityEventContext {
                user_id: None,
                source_ip: None,
                file_path: None,
                operation: Some("integrity_check".to_string()),
                error: None,
                metadata,
            },
            stack_trace: None,
        };

        self.log_event(event)
    }

    /// Log a resource exhaustion attempt
    pub fn log_resource_exhaustion(
        &self,
        module: Option<ModulePath>,
        resource_type: &str,
        limit: usize,
        attempted: usize,
    ) -> ModuleResult<()> {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("resource_type".to_string(), resource_type.to_string());
        metadata.insert("limit".to_string(), limit.to_string());
        metadata.insert("attempted".to_string(), attempted.to_string());

        let event = SecurityAuditEvent {
            timestamp: Utc::now(),
            severity: SecuritySeverity::Warning,
            category: SecurityEventCategory::ResourceExhaustion,
            module,
            description: format!(
                "Resource limit exceeded: {} (limit: {}, attempted: {})",
                resource_type, limit, attempted
            ),
            context: SecurityEventContext {
                user_id: None,
                source_ip: None,
                file_path: None,
                operation: Some("resource_allocation".to_string()),
                error: None,
                metadata,
            },
            stack_trace: None,
        };

        self.log_event(event)
    }

    /// Log a successful module load
    pub fn log_module_load(
        &self,
        module: ModulePath,
        file_path: &Path,
        checksum: &str,
    ) -> ModuleResult<()> {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("checksum".to_string(), checksum.to_string());

        let event = SecurityAuditEvent {
            timestamp: Utc::now(),
            severity: SecuritySeverity::Info,
            category: SecurityEventCategory::ModuleLoad,
            module: Some(module.clone()),
            description: format!("Module {} loaded successfully", module),
            context: SecurityEventContext {
                user_id: None,
                source_ip: None,
                file_path: Some(file_path.to_path_buf()),
                operation: Some("module_load".to_string()),
                error: None,
                metadata,
            },
            stack_trace: None,
        };

        self.log_event(event)
    }

    /// Get recent security events
    pub fn get_recent_events(&self, count: usize) -> Vec<SecurityAuditEvent> {
        let buffer = self.event_buffer.lock().unwrap();
        let start = buffer.len().saturating_sub(count);
        buffer[start..].to_vec()
    }

    /// Get events by severity
    pub fn get_events_by_severity(&self, severity: SecuritySeverity) -> Vec<SecurityAuditEvent> {
        let buffer = self.event_buffer.lock().unwrap();
        buffer
            .iter()
            .filter(|e| e.severity >= severity)
            .cloned()
            .collect()
    }

    /// Get audit statistics
    pub fn get_statistics(&self) -> AuditStatistics {
        let stats = self.stats.lock().unwrap();
        AuditStatistics {
            events_by_severity: stats.events_by_severity.clone(),
            events_by_category: stats.events_by_category.clone(),
            last_critical_event: stats.last_critical_event,
            total_events: stats.total_events,
        }
    }

    /// Write event to log file
    fn write_event(&self, event: &SecurityAuditEvent) -> ModuleResult<()> {
        let mut writer_opt = self.writer.lock().unwrap();

        if let Some(writer) = writer_opt.as_mut() {
            // Serialize event to JSON
            let json = serde_json::to_string(event).map_err(|e| {
                ModuleError::io_error(format!("Failed to serialize audit event: {e}"))
            })?;

            // Write with newline
            writeln!(writer, "{}", json).map_err(|e| {
                ModuleError::io_error(format!("Failed to write audit event: {e}"))
            })?;

            // Flush for critical events
            if event.severity >= SecuritySeverity::Critical {
                writer.flush().map_err(|e| {
                    ModuleError::io_error(format!("Failed to flush audit log: {e}"))
                })?;
            }
        }

        Ok(())
    }

    /// Buffer event in memory
    fn buffer_event(&self, event: SecurityAuditEvent) -> ModuleResult<()> {
        let mut buffer = self.event_buffer.lock().unwrap();

        buffer.push(event);

        // Trim buffer if too large
        if buffer.len() > self.config.buffer_size {
            let drain_count = buffer.len() - self.config.buffer_size;
            buffer.drain(0..drain_count);
        }

        Ok(())
    }

    /// Update statistics
    fn update_statistics(&self, event: &SecurityAuditEvent) {
        let mut stats = self.stats.lock().unwrap();

        // Update severity counts
        *stats.events_by_severity.entry(event.severity).or_insert(0) += 1;

        // Update category counts
        let category_str = format!("{:?}", event.category);
        *stats.events_by_category.entry(category_str).or_insert(0) += 1;

        // Update last critical time
        if event.severity >= SecuritySeverity::Critical {
            stats.last_critical_event = Some(SystemTime::now());
        }

        // Update total
        stats.total_events += 1;
    }

    /// Send real-time alert for critical events
    fn send_alert(&self, event: &SecurityAuditEvent) -> ModuleResult<()> {
        // In a real implementation, this would send to monitoring system
        eprintln!(
            "🚨 SECURITY ALERT: {} - {}",
            event.severity, event.description
        );
        Ok(())
    }

    /// Capture current stack trace
    fn capture_stack_trace() -> String {
        // In a real implementation, would use backtrace crate
        "Stack trace capture not implemented".to_string()
    }

    /// Rotate log file if needed
    pub fn rotate_if_needed(&self) -> ModuleResult<()> {
        let metadata = std::fs::metadata(&self.config.log_file).map_err(|e| {
            ModuleError::io_error(format!("Failed to get log file metadata: {e}"))
        })?;

        if metadata.len() >= self.config.max_file_size {
            self.rotate_log()?;
        }

        Ok(())
    }

    /// Rotate the log file
    fn rotate_log(&self) -> ModuleResult<()> {
        let mut writer_opt = self.writer.lock().unwrap();

        // Close current file
        if let Some(writer) = writer_opt.take() {
            drop(writer);
        }

        // Rename current log
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let rotated_name = format!("{}.{self.config.log_file.display(}"), timestamp);
        std::fs::rename(&self.config.log_file, &rotated_name)
            .map_err(|e| ModuleError::io_error(format!("Failed to rotate log file: {e}")))?;

        // Open new file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.log_file)
            .map_err(|e| ModuleError::io_error(format!("Failed to create new log file: {e}")))?;

        *writer_opt = Some(BufWriter::new(file));

        Ok(())
    }
}

/// Security event builder for convenient event creation
pub struct SecurityEventBuilder {
    severity: SecuritySeverity,
    category: SecurityEventCategory,
    module: Option<ModulePath>,
    description: String,
    context: SecurityEventContext,
}

impl SecurityEventBuilder {
    pub fn new(category: SecurityEventCategory, description: String) -> Self {
        SecurityEventBuilder {
            severity: SecuritySeverity::Info,
            category,
            module: None,
            description,
            context: SecurityEventContext {
                user_id: None,
                source_ip: None,
                file_path: None,
                operation: None,
                error: None,
                metadata: std::collections::HashMap::new(),
            },
        }
    }

    pub fn severity(mut self, severity: SecuritySeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn module(mut self, module: ModulePath) -> Self {
        self.module = Some(module);
        self
    }

    pub fn file_path(mut self, path: PathBuf) -> Self {
        self.context.file_path = Some(path);
        self
    }

    pub fn operation(mut self, op: String) -> Self {
        self.context.operation = Some(op);
        self
    }

    pub fn error(mut self, error: String) -> Self {
        self.context.error = Some(error);
        self
    }

    pub fn metadata(mut self, key: String, value: String) -> Self {
        self.context.metadata.insert(key, value);
        self
    }

    pub fn build(self) -> SecurityAuditEvent {
        SecurityAuditEvent {
            timestamp: Utc::now(),
            severity: self.severity,
            category: self.category,
            module: self.module,
            description: self.description,
            context: self.context,
            stack_trace: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_audit_logger_creation() {
        let temp_dir = TempDir::new().unwrap();
        let log_file = temp_dir.path().join("test_audit.log");

        let config = AuditConfig {
            log_file: log_file.clone(),
            ..Default::default()
        };

        let logger = SecurityAuditLogger::new(config).unwrap();
        assert!(log_file.exists());
    }

    #[test]
    fn test_event_logging() {
        let temp_dir = TempDir::new().unwrap();
        let log_file = temp_dir.path().join("test_audit.log");

        let config = AuditConfig {
            log_file: log_file.clone(),
            severity_filter: SecuritySeverity::Info,
            ..Default::default()
        };

        let logger = SecurityAuditLogger::new(config).unwrap();

        // Log an event
        let module = ModulePath::from_string("test.module").unwrap();
        logger
            .log_path_traversal(
                Some(module),
                "../../../etc/passwd",
                &ModuleError::security_violation("test"),
            )
            .unwrap();

        // Check statistics
        let stats = logger.get_statistics();
        assert_eq!(stats.total_events, 1);
        assert!(stats.last_critical_event.is_some());
    }

    #[test]
    fn test_severity_filtering() {
        let temp_dir = TempDir::new().unwrap();
        let log_file = temp_dir.path().join("test_audit.log");

        let config = AuditConfig {
            log_file,
            severity_filter: SecuritySeverity::Error,
            ..Default::default()
        };

        let logger = SecurityAuditLogger::new(config).unwrap();

        // Log info event (should be filtered)
        let event = SecurityEventBuilder::new(
            SecurityEventCategory::ModuleLoad,
            "Test info event".to_string(),
        )
        .severity(SecuritySeverity::Info)
        .build();

        logger.log_event(event).unwrap();

        // Log critical event (should pass)
        let event = SecurityEventBuilder::new(
            SecurityEventCategory::PathTraversal,
            "Test critical event".to_string(),
        )
        .severity(SecuritySeverity::Critical)
        .build();

        logger.log_event(event).unwrap();

        // Check only critical event was logged
        let stats = logger.get_statistics();
        assert_eq!(stats.total_events, 1);
    }
}
