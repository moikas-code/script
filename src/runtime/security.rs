//! Security monitoring and validation for Script runtime
//!
//! This module provides comprehensive security monitoring, attack detection,
//! and validation mechanisms to protect against malicious input and ensure
//! safe operation under adversarial conditions.

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Security monitoring configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Enable attack detection heuristics
    pub enable_attack_detection: bool,
    /// Enable memory corruption detection
    pub enable_memory_validation: bool,
    /// Enable type safety validation
    pub enable_type_validation: bool,
    /// Enable resource monitoring
    pub enable_resource_monitoring: bool,
    /// Alert threshold for suspicious activities
    pub alert_threshold: f64,
    /// History window for pattern detection
    pub history_window: Duration,
    /// Maximum alerts per minute to prevent spam
    pub max_alerts_per_minute: usize,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_attack_detection: true,
            enable_memory_validation: true,
            enable_type_validation: true,
            enable_resource_monitoring: true,
            alert_threshold: 0.7,
            history_window: Duration::from_secs(300), // 5 minutes
            max_alerts_per_minute: 10,
        }
    }
}

/// Security event types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SecurityEventType {
    /// Memory corruption detected
    MemoryCorruption,
    /// Type confusion attack detected
    TypeConfusion,
    /// Resource exhaustion attack
    ResourceExhaustion,
    /// Suspicious allocation patterns
    SuspiciousAllocation,
    /// Potential use-after-free
    UseAfterFree,
    /// Race condition detected
    RaceCondition,
    /// Unauthorized access attempt
    UnauthorizedAccess,
    /// Pattern indicates automated attack
    AutomatedAttack,
    /// Denial of service attempt
    DenialOfService,
    /// Information disclosure attempt
    InformationDisclosure,
}

/// Security event with detailed context
#[derive(Debug, Clone)]
pub struct SecurityEvent {
    /// Event type
    pub event_type: SecurityEventType,
    /// Event severity (0.0 - 1.0)
    pub severity: f64,
    /// Human-readable description
    pub description: String,
    /// Event timestamp
    pub timestamp: SystemTime,
    /// Additional context data
    pub context: HashMap<String, String>,
    /// Source component that detected the event
    pub source: String,
    /// Event ID for correlation
    pub event_id: u64,
}

/// Security metrics for monitoring
#[derive(Debug, Default, Clone)]
pub struct SecurityMetrics {
    /// Total security events detected
    pub total_events: usize,
    /// Events by type
    pub events_by_type: HashMap<SecurityEventType, usize>,
    /// High severity events (> 0.8)
    pub high_severity_events: usize,
    /// Events in last hour
    pub recent_events: usize,
    /// Average event severity
    pub average_severity: f64,
    /// Potential attack indicators
    pub attack_indicators: usize,
    /// Memory validation failures
    pub memory_validation_failures: usize,
    /// Type validation failures
    pub type_validation_failures: usize,
}

/// Pattern detection for automated attacks
#[derive(Debug)]
struct AttackPattern {
    /// Pattern name
    name: String,
    /// Minimum events to trigger
    min_events: usize,
    /// Time window for pattern detection
    window: Duration,
    /// Event types that contribute to this pattern
    event_types: Vec<SecurityEventType>,
    /// Confidence threshold
    confidence_threshold: f64,
}

/// Security monitor with comprehensive threat detection
pub struct SecurityMonitor {
    /// Configuration
    config: SecurityConfig,
    /// Event history for pattern detection
    event_history: RwLock<VecDeque<SecurityEvent>>,
    /// Global event counter
    event_counter: AtomicU64,
    /// Metrics tracking
    metrics: RwLock<SecurityMetrics>,
    /// Rate limiting for alerts
    alert_limiter: Mutex<AlertLimiter>,
    /// Known attack patterns
    attack_patterns: Vec<AttackPattern>,
    /// Memory validation state
    memory_validator: MemoryValidator,
    /// Type validation state  
    type_validator: TypeValidator,
}

/// Rate limiter for security alerts
#[derive(Debug)]
struct AlertLimiter {
    /// Alert timestamps in current minute
    current_minute_alerts: VecDeque<Instant>,
    /// Current minute start
    current_minute_start: Instant,
}

impl AlertLimiter {
    fn new() -> Self {
        Self {
            current_minute_alerts: VecDeque::new(),
            current_minute_start: Instant::now(),
        }
    }

    /// Check if alert should be rate limited
    fn should_limit(&mut self, config: &SecurityConfig) -> bool {
        let now = Instant::now();

        // Check if we've moved to a new minute
        if now.duration_since(self.current_minute_start) >= Duration::from_secs(60) {
            self.current_minute_alerts.clear();
            self.current_minute_start = now;
        }

        // Remove old alerts from current minute
        while let Some(&front_time) = self.current_minute_alerts.front() {
            if now.duration_since(front_time) >= Duration::from_secs(60) {
                self.current_minute_alerts.pop_front();
            } else {
                break;
            }
        }

        // Check rate limit
        if self.current_minute_alerts.len() >= config.max_alerts_per_minute {
            return true; // Rate limited
        }

        self.current_minute_alerts.push_back(now);
        false
    }
}

/// Memory validation for detecting corruption
#[derive(Debug)]
struct MemoryValidator {
    /// Known memory regions and their checksums
    regions: RwLock<HashMap<usize, MemoryRegion>>,
    /// Validation failure count
    failures: AtomicUsize,
}

#[derive(Debug, Clone)]
struct MemoryRegion {
    /// Start address
    start: usize,
    /// Size in bytes
    size: usize,
    /// Expected checksum
    checksum: u64,
    /// Last validation time
    last_validated: Instant,
}

impl MemoryValidator {
    fn new() -> Self {
        Self {
            regions: RwLock::new(HashMap::new()),
            failures: AtomicUsize::new(0),
        }
    }

    /// Register a memory region for validation
    fn register_region(&self, address: usize, size: usize) -> Result<(), SecurityError> {
        if size == 0 || size > 1024 * 1024 * 1024 {
            return Err(SecurityError::InvalidRegionSize);
        }

        let checksum = self.calculate_checksum(address, size)?;
        let region = MemoryRegion {
            start: address,
            size,
            checksum,
            last_validated: Instant::now(),
        };

        let mut regions = self
            .regions
            .write()
            .map_err(|_| SecurityError::LockPoisoned)?;
        regions.insert(address, region);
        Ok(())
    }

    /// Validate a memory region
    fn validate_region(&self, address: usize) -> Result<bool, SecurityError> {
        let regions = self
            .regions
            .read()
            .map_err(|_| SecurityError::LockPoisoned)?;

        if let Some(region) = regions.get(&address) {
            let current_checksum = self.calculate_checksum(region.start, region.size)?;
            if current_checksum != region.checksum {
                self.failures.fetch_add(1, Ordering::Relaxed);
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Calculate checksum for memory region
    fn calculate_checksum(&self, address: usize, size: usize) -> Result<u64, SecurityError> {
        if address == 0 || size == 0 {
            return Err(SecurityError::InvalidAddress);
        }

        // Simple checksum - in production would use a cryptographic hash
        let mut checksum = 0u64;
        unsafe {
            let ptr = address as *const u8;
            for i in 0..size {
                if i % 8 == 0 {
                    // Sample every 8th byte for performance
                    checksum = checksum.wrapping_add(*ptr.add(i) as u64);
                }
            }
        }
        Ok(checksum)
    }

    fn get_failure_count(&self) -> usize {
        self.failures.load(Ordering::Relaxed)
    }
}

/// Type validation for preventing type confusion
#[derive(Debug)]
struct TypeValidator {
    /// Type safety violations
    violations: AtomicUsize,
    /// Validated type combinations
    validated_combinations: RwLock<HashSet<(u64, u64)>>,
}

use std::collections::HashSet;

impl TypeValidator {
    fn new() -> Self {
        Self {
            violations: AtomicUsize::new(0),
            validated_combinations: RwLock::new(HashSet::new()),
        }
    }

    /// Validate a type cast operation
    fn validate_cast(&self, from_type: u64, to_type: u64) -> Result<bool, SecurityError> {
        // Check if this combination has been validated before
        let combinations = self
            .validated_combinations
            .read()
            .map_err(|_| SecurityError::LockPoisoned)?;

        if combinations.contains(&(from_type, to_type)) {
            return Ok(true);
        }
        drop(combinations);

        // Perform validation logic
        let is_valid = self.is_valid_cast(from_type, to_type);

        if is_valid {
            // Cache valid combination
            let mut combinations = self
                .validated_combinations
                .write()
                .map_err(|_| SecurityError::LockPoisoned)?;
            combinations.insert((from_type, to_type));
        } else {
            self.violations.fetch_add(1, Ordering::Relaxed);
        }

        Ok(is_valid)
    }

    fn is_valid_cast(&self, from_type: u64, to_type: u64) -> bool {
        // Simplified type validation - in production would check type hierarchy
        from_type == to_type || (from_type > 0 && to_type > 0)
    }

    fn get_violation_count(&self) -> usize {
        self.violations.load(Ordering::Relaxed)
    }
}

/// Security errors
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityError {
    /// Invalid memory region size
    InvalidRegionSize,
    /// Invalid memory address
    InvalidAddress,
    /// Lock was poisoned
    LockPoisoned,
    /// Rate limited
    RateLimited,
    /// Configuration error
    ConfigurationError(String),
}

impl fmt::Display for SecurityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecurityError::InvalidRegionSize => write!(f, "Invalid memory region size"),
            SecurityError::InvalidAddress => write!(f, "Invalid memory address"),
            SecurityError::LockPoisoned => write!(f, "Lock was poisoned"),
            SecurityError::RateLimited => write!(f, "Rate limited"),
            SecurityError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl std::error::Error for SecurityError {}

impl SecurityMonitor {
    /// Create a new security monitor
    pub fn new(config: SecurityConfig) -> Self {
        let attack_patterns = vec![
            AttackPattern {
                name: "Resource Exhaustion".to_string(),
                min_events: 5,
                window: Duration::from_secs(60),
                event_types: vec![
                    SecurityEventType::ResourceExhaustion,
                    SecurityEventType::SuspiciousAllocation,
                ],
                confidence_threshold: 0.8,
            },
            AttackPattern {
                name: "Memory Corruption".to_string(),
                min_events: 3,
                window: Duration::from_secs(30),
                event_types: vec![
                    SecurityEventType::MemoryCorruption,
                    SecurityEventType::UseAfterFree,
                ],
                confidence_threshold: 0.9,
            },
            AttackPattern {
                name: "Type Confusion".to_string(),
                min_events: 2,
                window: Duration::from_secs(10),
                event_types: vec![SecurityEventType::TypeConfusion],
                confidence_threshold: 0.95,
            },
        ];

        Self {
            config,
            event_history: RwLock::new(VecDeque::new()),
            event_counter: AtomicU64::new(0),
            metrics: RwLock::new(SecurityMetrics::default()),
            alert_limiter: Mutex::new(AlertLimiter::new()),
            attack_patterns,
            memory_validator: MemoryValidator::new(),
            type_validator: TypeValidator::new(),
        }
    }

    /// Record a security event
    pub fn record_event(&self, mut event: SecurityEvent) -> Result<(), SecurityError> {
        // Assign event ID
        event.event_id = self.event_counter.fetch_add(1, Ordering::Relaxed);
        event.timestamp = SystemTime::now();

        // Check rate limiting
        if let Ok(mut limiter) = self.alert_limiter.lock() {
            if limiter.should_limit(&self.config) {
                return Err(SecurityError::RateLimited);
            }
        }

        // Update metrics
        self.update_metrics(&event)?;

        // Add to history
        let mut history = self
            .event_history
            .write()
            .map_err(|_| SecurityError::LockPoisoned)?;

        history.push_back(event.clone());

        // Trim old events
        let cutoff = SystemTime::now() - self.config.history_window;
        while let Some(front) = history.front() {
            if front.timestamp < cutoff {
                history.pop_front();
            } else {
                break;
            }
        }

        // Check for attack patterns
        self.detect_attack_patterns(&history)?;

        Ok(())
    }

    /// Update security metrics
    fn update_metrics(&self, event: &SecurityEvent) -> Result<(), SecurityError> {
        let mut metrics = self
            .metrics
            .write()
            .map_err(|_| SecurityError::LockPoisoned)?;

        metrics.total_events += 1;
        *metrics
            .events_by_type
            .entry(event.event_type.clone())
            .or_insert(0) += 1;

        if event.severity > 0.8 {
            metrics.high_severity_events += 1;
        }

        // Update average severity
        let total_severity =
            metrics.average_severity * (metrics.total_events - 1) as f64 + event.severity;
        metrics.average_severity = total_severity / metrics.total_events as f64;

        // Update component-specific metrics
        match event.event_type {
            SecurityEventType::MemoryCorruption | SecurityEventType::UseAfterFree => {
                metrics.memory_validation_failures += 1;
            }
            SecurityEventType::TypeConfusion => {
                metrics.type_validation_failures += 1;
            }
            _ => {}
        }

        Ok(())
    }

    /// Detect attack patterns in event history
    fn detect_attack_patterns(
        &self,
        history: &VecDeque<SecurityEvent>,
    ) -> Result<(), SecurityError> {
        if !self.config.enable_attack_detection {
            return Ok(());
        }

        for pattern in &self.attack_patterns {
            let matching_events: Vec<_> = history
                .iter()
                .filter(|event| {
                    pattern.event_types.contains(&event.event_type)
                        && SystemTime::now()
                            .duration_since(event.timestamp)
                            .unwrap_or(Duration::MAX)
                            <= pattern.window
                })
                .collect();

            if matching_events.len() >= pattern.min_events {
                let confidence = self.calculate_pattern_confidence(&matching_events, pattern);

                if confidence >= pattern.confidence_threshold {
                    // Attack pattern detected
                    let attack_event = SecurityEvent {
                        event_type: SecurityEventType::AutomatedAttack,
                        severity: confidence,
                        description: format!("Attack pattern detected: {}", pattern.name),
                        timestamp: SystemTime::now(),
                        context: [
                            ("pattern_name".to_string(), pattern.name.clone()),
                            (
                                "matching_events".to_string(),
                                matching_events.len().to_string(),
                            ),
                            ("confidence".to_string(), confidence.to_string()),
                        ]
                        .iter()
                        .cloned()
                        .collect(),
                        source: "SecurityMonitor".to_string(),
                        event_id: 0, // Will be assigned
                    };

                    // Record the attack detection (recursive, but with different type)
                    if let Ok(mut metrics) = self.metrics.write() {
                        metrics.attack_indicators += 1;
                    }
                }
            }
        }

        Ok(())
    }

    /// Calculate confidence score for an attack pattern
    fn calculate_pattern_confidence(
        &self,
        events: &[&SecurityEvent],
        pattern: &AttackPattern,
    ) -> f64 {
        if events.is_empty() {
            return 0.0;
        }

        // Base confidence from event count
        let count_factor = (events.len() as f64 / pattern.min_events as f64).min(1.0);

        // Severity factor
        let avg_severity = events.iter().map(|e| e.severity).sum::<f64>() / events.len() as f64;

        // Time clustering factor (events close in time are more suspicious)
        let time_factor = if events.len() > 1 {
            let time_span = events
                .iter()
                .map(|e| e.timestamp.duration_since(UNIX_EPOCH).unwrap_or_default())
                .max()
                .unwrap_or_default()
                .saturating_sub(
                    events
                        .iter()
                        .map(|e| e.timestamp.duration_since(UNIX_EPOCH).unwrap_or_default())
                        .min()
                        .unwrap_or_default(),
                );

            if time_span <= Duration::from_secs(10) {
                1.0 // Very clustered
            } else if time_span <= Duration::from_secs(60) {
                0.8 // Somewhat clustered
            } else {
                0.5 // Spread out
            }
        } else {
            1.0
        };

        // Combine factors
        count_factor * avg_severity * time_factor
    }

    /// Validate memory region
    pub fn validate_memory(&self, address: usize, size: usize) -> Result<bool, SecurityError> {
        if !self.config.enable_memory_validation {
            return Ok(true);
        }

        // Register region if not already registered
        if let Err(_) = self.memory_validator.validate_region(address) {
            self.memory_validator.register_region(address, size)?;
        }

        let is_valid = self.memory_validator.validate_region(address)?;

        if !is_valid {
            let event = SecurityEvent {
                event_type: SecurityEventType::MemoryCorruption,
                severity: 0.9,
                description: "Memory corruption detected during validation".to_string(),
                timestamp: SystemTime::now(),
                context: [
                    ("address".to_string(), format!("0x{:x}", address)),
                    ("size".to_string(), size.to_string()),
                ]
                .iter()
                .cloned()
                .collect(),
                source: "MemoryValidator".to_string(),
                event_id: 0,
            };

            let _ = self.record_event(event);
        }

        Ok(is_valid)
    }

    /// Validate type cast
    pub fn validate_type_cast(&self, from_type: u64, to_type: u64) -> Result<bool, SecurityError> {
        if !self.config.enable_type_validation {
            return Ok(true);
        }

        let is_valid = self.type_validator.validate_cast(from_type, to_type)?;

        if !is_valid {
            let event = SecurityEvent {
                event_type: SecurityEventType::TypeConfusion,
                severity: 0.85,
                description: "Invalid type cast detected".to_string(),
                timestamp: SystemTime::now(),
                context: [
                    ("from_type".to_string(), from_type.to_string()),
                    ("to_type".to_string(), to_type.to_string()),
                ]
                .iter()
                .cloned()
                .collect(),
                source: "TypeValidator".to_string(),
                event_id: 0,
            };

            let _ = self.record_event(event);
        }

        Ok(is_valid)
    }

    /// Get current security metrics
    pub fn get_metrics(&self) -> Result<SecurityMetrics, SecurityError> {
        let metrics = self
            .metrics
            .read()
            .map_err(|_| SecurityError::LockPoisoned)?;
        Ok(metrics.clone())
    }

    /// Get recent events
    pub fn get_recent_events(&self, limit: usize) -> Result<Vec<SecurityEvent>, SecurityError> {
        let history = self
            .event_history
            .read()
            .map_err(|_| SecurityError::LockPoisoned)?;

        Ok(history.iter().rev().take(limit).cloned().collect())
    }

    /// Check if system is under attack
    pub fn is_under_attack(&self) -> bool {
        if let Ok(metrics) = self.metrics.read() {
            metrics.attack_indicators > 0 || metrics.average_severity > self.config.alert_threshold
        } else {
            false
        }
    }
}

/// Global security monitor instance
static SECURITY_MONITOR: RwLock<Option<Arc<SecurityMonitor>>> = RwLock::new(None);

/// Initialize global security monitor
pub fn initialize_security_monitor(config: SecurityConfig) -> Result<(), SecurityError> {
    let monitor = Arc::new(SecurityMonitor::new(config));
    let mut global_monitor = SECURITY_MONITOR
        .write()
        .map_err(|_| SecurityError::LockPoisoned)?;
    *global_monitor = Some(monitor);
    Ok(())
}

/// Get global security monitor
pub fn get_security_monitor() -> Result<Arc<SecurityMonitor>, SecurityError> {
    let monitor = SECURITY_MONITOR
        .read()
        .map_err(|_| SecurityError::LockPoisoned)?;
    monitor
        .as_ref()
        .ok_or(SecurityError::ConfigurationError(
            "Security monitor not initialized".to_string(),
        ))
        .map(Arc::clone)
}

/// Report a security event to the global monitor
pub fn report_security_event(event: SecurityEvent) -> Result<(), SecurityError> {
    let monitor = get_security_monitor()?;
    monitor.record_event(event)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_monitor_creation() {
        let config = SecurityConfig::default();
        let monitor = SecurityMonitor::new(config);
        assert!(monitor.get_metrics().is_ok());
    }

    #[test]
    fn test_event_recording() {
        let config = SecurityConfig::default();
        let monitor = SecurityMonitor::new(config);

        let event = SecurityEvent {
            event_type: SecurityEventType::MemoryCorruption,
            severity: 0.8,
            description: "Test event".to_string(),
            timestamp: SystemTime::now(),
            context: HashMap::new(),
            source: "Test".to_string(),
            event_id: 0,
        };

        assert!(monitor.record_event(event).is_ok());

        let metrics = monitor.get_metrics().unwrap();
        assert_eq!(metrics.total_events, 1);
    }

    #[test]
    fn test_memory_validation() {
        let config = SecurityConfig::default();
        let monitor = SecurityMonitor::new(config);

        // Test with valid memory region
        let test_data = vec![1u8, 2, 3, 4, 5];
        let address = test_data.as_ptr() as usize;
        assert!(monitor.validate_memory(address, test_data.len()).is_ok());
    }

    #[test]
    fn test_type_validation() {
        let config = SecurityConfig::default();
        let monitor = SecurityMonitor::new(config);

        // Test valid type cast
        assert!(monitor.validate_type_cast(1, 1).unwrap());

        // Test invalid type cast
        assert!(!monitor.validate_type_cast(0, 1).unwrap());
    }
}
