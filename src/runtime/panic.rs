//! Panic handler for Script runtime
//!
//! This module provides panic handling with stack traces and error recovery
//! for Script programs. It integrates with Rust's panic system while providing
//! Script-specific context and debugging information.

use std::backtrace::{Backtrace, BacktraceStatus};
use std::collections::VecDeque;
use std::fmt;
use std::panic;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

/// Global panic handler instance
static PANIC_HANDLER: RwLock<Option<Arc<PanicHandler>>> = RwLock::new(None);

/// Maximum number of recent panics to keep
const MAX_PANIC_HISTORY: usize = 10;

/// Recovery policies for panic handling
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecoveryPolicy {
    /// Abort the program (default Rust behavior)
    Abort,
    /// Continue execution after recovery
    Continue,
    /// Restart the current operation
    Restart,
    /// Restart with degraded functionality
    DegradedRestart,
    /// Custom recovery via callback
    Custom,
}

/// Recovery context for panic recovery operations
#[derive(Debug, Clone)]
pub struct RecoveryContext {
    /// The panic that triggered recovery
    pub panic_info: PanicInfo,
    /// Current recovery attempt number
    pub recovery_attempt: u32,
    /// Time when recovery started
    pub recovery_start: Instant,
    /// Maximum recovery attempts allowed
    pub max_attempts: u32,
    /// Recovery timeout
    pub timeout: Duration,
    /// Additional context data
    pub context_data: std::collections::HashMap<String, String>,
}

/// Recovery result after attempting panic recovery
#[derive(Debug, Clone)]
pub enum RecoveryResult {
    /// Recovery successful, continue execution
    Success,
    /// Recovery failed, try again
    Retry,
    /// Recovery failed, abort
    Abort,
    /// Recovery successful with degraded functionality
    Degraded(String),
}

/// Recovery callback type
pub type RecoveryCallback = Box<dyn Fn(&RecoveryContext) -> RecoveryResult + Send + Sync>;

/// Panic boundary for isolating failures
#[derive(Debug)]
pub struct PanicBoundary {
    /// Boundary identifier
    pub id: String,
    /// Recovery policy for this boundary
    pub recovery_policy: RecoveryPolicy,
    /// Maximum recovery attempts
    pub max_recovery_attempts: u32,
    /// Recovery timeout
    pub recovery_timeout: Duration,
    /// Whether this boundary is active
    pub active: bool,
}

/// Initialize the panic handler
pub fn initialize() {
    let mut handler = PANIC_HANDLER.write().unwrap();
    *handler = Some(Arc::new(PanicHandler::new()));
}

/// Shutdown the panic handler
pub fn shutdown() {
    let mut handler = PANIC_HANDLER.write().unwrap();
    *handler = None;
}

/// Record a panic
pub fn record_panic(info: PanicInfo) {
    if let Ok(handler) = PANIC_HANDLER.read() {
        if let Some(h) = handler.as_ref() {
            h.record_panic(info);
        }
    }
}

/// Get the last panic info
pub fn last_panic() -> Option<PanicInfo> {
    if let Ok(handler) = PANIC_HANDLER.read() {
        if let Some(h) = handler.as_ref() {
            return h.last_panic();
        }
    }
    None
}

/// Get panic history
pub fn panic_history() -> Vec<PanicInfo> {
    if let Ok(handler) = PANIC_HANDLER.read() {
        if let Some(h) = handler.as_ref() {
            return h.history();
        }
    }
    Vec::new()
}

/// Information about a panic
#[derive(Debug, Clone)]
pub struct PanicInfo {
    /// Panic message
    pub message: String,
    /// Location where panic occurred
    pub location: Option<String>,
    /// Backtrace at panic point
    pub backtrace: String,
    /// Timestamp when panic occurred
    pub timestamp: Instant,
    /// Recovery attempts made
    pub recovery_attempts: u32,
    /// Whether recovery was successful
    pub recovered: bool,
    /// Recovery policy used
    pub recovery_policy: RecoveryPolicy,
}

/// Stack frame information
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// Function name
    pub function: String,
    /// File path
    pub file: Option<String>,
    /// Line number
    pub line: Option<u32>,
    /// Column number
    pub column: Option<u32>,
}

/// A stack trace for Script execution
#[derive(Debug, Clone)]
pub struct StackTrace {
    /// Stack frames
    pub frames: Vec<StackFrame>,
}

/// The panic handler
pub struct PanicHandler {
    /// Recent panic history
    history: Mutex<VecDeque<PanicInfo>>,
    /// Custom panic hook
    custom_hook: RwLock<Option<Box<dyn Fn(&PanicInfo) + Send + Sync>>>,
    /// Recovery callbacks
    recovery_callbacks: RwLock<Vec<RecoveryCallback>>,
    /// Active panic boundaries
    boundaries: RwLock<Vec<PanicBoundary>>,
    /// Default recovery policy
    default_recovery_policy: RwLock<RecoveryPolicy>,
    /// Recovery metrics
    recovery_metrics: Mutex<RecoveryMetrics>,
}

/// Metrics for panic recovery operations
#[derive(Debug, Clone, Default)]
pub struct RecoveryMetrics {
    /// Total panic count
    pub total_panics: u64,
    /// Successful recoveries
    pub successful_recoveries: u64,
    /// Failed recoveries
    pub failed_recoveries: u64,
    /// Total recovery attempts
    pub total_recovery_attempts: u64,
    /// Average recovery time
    pub average_recovery_time: Duration,
    /// Recovery success rate
    pub success_rate: f64,
}

impl PanicHandler {
    /// Create a new panic handler
    fn new() -> Self {
        PanicHandler {
            history: Mutex::new(VecDeque::with_capacity(MAX_PANIC_HISTORY)),
            custom_hook: RwLock::new(None),
            recovery_callbacks: RwLock::new(Vec::new()),
            boundaries: RwLock::new(Vec::new()),
            default_recovery_policy: RwLock::new(RecoveryPolicy::Abort),
            recovery_metrics: Mutex::new(RecoveryMetrics::default()),
        }
    }

    /// Record a panic and attempt recovery
    fn record_panic(&self, mut info: PanicInfo) {
        // Update metrics
        {
            let mut metrics = self.recovery_metrics.lock().unwrap();
            metrics.total_panics += 1;
        }

        // Attempt recovery if policy allows
        if info.recovery_policy != RecoveryPolicy::Abort {
            match self.attempt_recovery(&mut info) {
                Ok(RecoveryResult::Success) => {
                    info.recovered = true;
                    self.update_recovery_metrics(true, info.recovery_attempts);
                },
                Ok(RecoveryResult::Degraded(msg)) => {
                    info.recovered = true;
                    eprintln!("Panic recovery succeeded with degraded functionality: {}", msg);
                    self.update_recovery_metrics(true, info.recovery_attempts);
                },
                _ => {
                    info.recovered = false;
                    self.update_recovery_metrics(false, info.recovery_attempts);
                }
            }
        }

        // Add to history
        let mut history = self.history.lock().unwrap();
        if history.len() >= MAX_PANIC_HISTORY {
            history.pop_front();
        }
        history.push_back(info.clone());

        // Call custom hook if set
        if let Ok(hook) = self.custom_hook.read() {
            if let Some(h) = hook.as_ref() {
                h(&info);
            }
        }
    }

    /// Get the last panic
    fn last_panic(&self) -> Option<PanicInfo> {
        let history = self.history.lock().unwrap();
        history.back().cloned()
    }

    /// Get panic history
    fn history(&self) -> Vec<PanicInfo> {
        let history = self.history.lock().unwrap();
        history.iter().cloned().collect()
    }

    /// Set a custom panic hook
    pub fn set_hook<F>(&self, hook: F)
    where
        F: Fn(&PanicInfo) + Send + Sync + 'static,
    {
        let mut custom_hook = self.custom_hook.write().unwrap();
        *custom_hook = Some(Box::new(hook));
    }

    /// Clear panic history
    pub fn clear_history(&self) {
        let mut history = self.history.lock().unwrap();
        history.clear();
    }

    /// Set default recovery policy
    pub fn set_default_recovery_policy(&self, policy: RecoveryPolicy) {
        let mut default_policy = self.default_recovery_policy.write().unwrap();
        *default_policy = policy;
    }

    /// Add a recovery callback
    pub fn add_recovery_callback(&self, callback: RecoveryCallback) {
        let mut callbacks = self.recovery_callbacks.write().unwrap();
        callbacks.push(callback);
    }

    /// Create a panic boundary
    pub fn create_boundary(&self, id: String, policy: RecoveryPolicy) -> PanicBoundary {
        let boundary = PanicBoundary {
            id: id.clone(),
            recovery_policy: policy,
            max_recovery_attempts: 3,
            recovery_timeout: Duration::from_secs(30),
            active: true,
        };

        let mut boundaries = self.boundaries.write().unwrap();
        boundaries.push(boundary);
        
        // Return a new boundary with the same properties
        PanicBoundary {
            id,
            recovery_policy: policy,
            max_recovery_attempts: 3,
            recovery_timeout: Duration::from_secs(30),
            active: true,
        }
    }

    /// Attempt panic recovery
    fn attempt_recovery(&self, info: &mut PanicInfo) -> Result<RecoveryResult, String> {
        let recovery_start = Instant::now();
        let max_attempts = 3;
        let timeout = Duration::from_secs(30);

        // Create recovery context
        let mut context = RecoveryContext {
            panic_info: info.clone(),
            recovery_attempt: 0,
            recovery_start,
            max_attempts,
            timeout,
            context_data: std::collections::HashMap::new(),
        };

        // Try recovery based on policy
        match info.recovery_policy {
            RecoveryPolicy::Abort => Ok(RecoveryResult::Abort),
            RecoveryPolicy::Continue => {
                // Simple continue - just mark as recovered
                info.recovery_attempts = 1;
                Ok(RecoveryResult::Success)
            },
            RecoveryPolicy::Restart => {
                // Attempt restart recovery
                self.attempt_restart_recovery(&mut context)
            },
            RecoveryPolicy::DegradedRestart => {
                // Attempt degraded restart
                self.attempt_degraded_recovery(&mut context)
            },
            RecoveryPolicy::Custom => {
                // Try custom recovery callbacks
                self.attempt_custom_recovery(&mut context)
            },
        }
    }

    /// Attempt restart recovery
    fn attempt_restart_recovery(&self, context: &mut RecoveryContext) -> Result<RecoveryResult, String> {
        for attempt in 1..=context.max_attempts {
            context.recovery_attempt = attempt;
            
            if context.recovery_start.elapsed() > context.timeout {
                return Ok(RecoveryResult::Abort);
            }

            // Basic restart recovery - clear state and try again
            if self.clear_corrupt_state() {
                return Ok(RecoveryResult::Success);
            }

            // Wait between attempts
            std::thread::sleep(Duration::from_millis(100 * attempt as u64));
        }

        Ok(RecoveryResult::Abort)
    }

    /// Attempt degraded recovery
    fn attempt_degraded_recovery(&self, context: &mut RecoveryContext) -> Result<RecoveryResult, String> {
        context.recovery_attempt = 1;
        
        // Enable degraded mode - continue with reduced functionality
        if self.enable_degraded_mode() {
            Ok(RecoveryResult::Degraded("Running in degraded mode".to_string()))
        } else {
            Ok(RecoveryResult::Abort)
        }
    }

    /// Attempt custom recovery using callbacks
    fn attempt_custom_recovery(&self, context: &mut RecoveryContext) -> Result<RecoveryResult, String> {
        let callbacks = self.recovery_callbacks.read().unwrap();
        
        for callback in callbacks.iter() {
            context.recovery_attempt += 1;
            
            if context.recovery_start.elapsed() > context.timeout {
                return Ok(RecoveryResult::Abort);
            }

            match callback(context) {
                RecoveryResult::Success => return Ok(RecoveryResult::Success),
                RecoveryResult::Degraded(msg) => return Ok(RecoveryResult::Degraded(msg)),
                RecoveryResult::Abort => return Ok(RecoveryResult::Abort),
                RecoveryResult::Retry => continue,
            }
        }

        Ok(RecoveryResult::Abort)
    }

    /// Clear corrupt state for restart recovery
    fn clear_corrupt_state(&self) -> bool {
        // In a real implementation, this would clear runtime state
        // For now, we'll just return true to indicate success
        true
    }

    /// Enable degraded mode
    fn enable_degraded_mode(&self) -> bool {
        // In a real implementation, this would disable non-essential features
        // For now, we'll just return true to indicate success
        true
    }

    /// Update recovery metrics
    fn update_recovery_metrics(&self, success: bool, attempts: u32) {
        let mut metrics = self.recovery_metrics.lock().unwrap();
        metrics.total_recovery_attempts += attempts as u64;
        
        if success {
            metrics.successful_recoveries += 1;
        } else {
            metrics.failed_recoveries += 1;
        }

        // Update success rate
        let total_recoveries = metrics.successful_recoveries + metrics.failed_recoveries;
        if total_recoveries > 0 {
            metrics.success_rate = metrics.successful_recoveries as f64 / total_recoveries as f64;
        }
    }

    /// Get recovery metrics
    pub fn get_recovery_metrics(&self) -> RecoveryMetrics {
        let metrics = self.recovery_metrics.lock().unwrap();
        metrics.clone()
    }
}

impl StackTrace {
    /// Capture current stack trace
    pub fn capture() -> Self {
        let backtrace = Backtrace::capture();
        let frames = Self::parse_backtrace(&backtrace);
        StackTrace { frames }
    }

    /// Parse a Rust backtrace into Script stack frames
    fn parse_backtrace(backtrace: &Backtrace) -> Vec<StackFrame> {
        let mut frames = Vec::new();

        if backtrace.status() != BacktraceStatus::Captured {
            return frames;
        }

        // Parse backtrace string
        let bt_string = format!("{:?}", backtrace);
        for line in bt_string.lines() {
            // Skip non-frame lines
            if !line.contains(':') || line.contains("__rust") {
                continue;
            }

            // Try to parse frame information
            if let Some(frame) = Self::parse_frame_line(line) {
                // Filter out runtime internals
                if !frame.function.contains("script::runtime")
                    && !frame.function.contains("std::panic")
                    && !frame.function.contains("rust_begin_unwind")
                {
                    frames.push(frame);
                }
            }
        }

        frames
    }

    /// Parse a single frame line from backtrace
    fn parse_frame_line(line: &str) -> Option<StackFrame> {
        // This is a simplified parser - in production would need more robust parsing
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() < 2 {
            return None;
        }

        let function = parts[1].to_string();

        // Try to find file:line:column
        let mut file = None;
        let mut line_num = None;
        let mut column = None;

        for part in &parts[2..] {
            if part.contains(':') {
                let location_parts: Vec<&str> = part.split(':').collect();
                if location_parts.len() >= 2 {
                    file = Some(location_parts[0].to_string());
                    line_num = location_parts[1].parse().ok();
                    if location_parts.len() >= 3 {
                        column = location_parts[2].parse().ok();
                    }
                    break;
                }
            }
        }

        Some(StackFrame {
            function,
            file,
            line: line_num,
            column,
        })
    }

    /// Format the stack trace for display
    pub fn format(&self) -> String {
        let mut output = String::new();

        for (i, frame) in self.frames.iter().enumerate() {
            output.push_str(&format!("  {} at {}", i, frame.function));

            if let Some(file) = &frame.file {
                output.push_str(&format!("\n      {}", file));
                if let Some(line) = frame.line {
                    output.push_str(&format!(":{}", line));
                    if let Some(col) = frame.column {
                        output.push_str(&format!(":{}", col));
                    }
                }
            }
            output.push('\n');
        }

        output
    }
}

impl fmt::Display for StackTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}

impl fmt::Display for PanicInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Script panic: {}", self.message)?;
        if let Some(loc) = &self.location {
            writeln!(f, "Location: {}", loc)?;
        }
        write!(f, "Stack trace:\n{}", self.backtrace)
    }
}

/// Create a Script panic with current context
pub fn script_panic(message: impl Into<String>) -> ! {
    script_panic_with_policy(message, RecoveryPolicy::Abort);
}

/// Create a Script panic with recovery policy
pub fn script_panic_with_policy(message: impl Into<String>, policy: RecoveryPolicy) -> ! {
    let mut info = PanicInfo {
        message: message.into(),
        location: None,
        backtrace: StackTrace::capture().format(),
        timestamp: Instant::now(),
        recovery_attempts: 0,
        recovered: false,
        recovery_policy: policy,
    };

    record_panic(info.clone());
    
    // Check if panic was recovered after recording
    if let Some(last_panic) = last_panic() {
        if last_panic.recovered {
            // Recovery successful, but we still need to panic because of the ! return type
            // In a real implementation, this would need different handling
            panic!("Recovery successful: {}", info.message);
        }
    }
    
    panic!("{}", info.message);
}

/// Create a Script panic with location
pub fn script_panic_at(message: impl Into<String>, file: &str, line: u32, column: u32) -> ! {
    script_panic_at_with_policy(message, file, line, column, RecoveryPolicy::Abort);
}

/// Create a Script panic with location and recovery policy
pub fn script_panic_at_with_policy(message: impl Into<String>, file: &str, line: u32, column: u32, policy: RecoveryPolicy) -> ! {
    let mut info = PanicInfo {
        message: message.into(),
        location: Some(format!("{}:{}:{}", file, line, column)),
        backtrace: StackTrace::capture().format(),
        timestamp: Instant::now(),
        recovery_attempts: 0,
        recovered: false,
        recovery_policy: policy,
    };

    record_panic(info.clone());
    
    // Check if panic was recovered after recording
    if let Some(last_panic) = last_panic() {
        if last_panic.recovered {
            // Recovery successful, but we still need to panic because of the ! return type
            // In a real implementation, this would need different handling
            panic!("Recovery successful: {}", info.message);
        }
    }
    
    panic!("{}", info.message);
}

/// Assert with Script panic
#[macro_export]
macro_rules! script_assert {
    ($cond:expr) => {
        if !$cond {
            $crate::runtime::panic::script_panic(concat!("assertion failed: ", stringify!($cond)));
        }
    };
    ($cond:expr, $msg:expr) => {
        if !$cond {
            $crate::runtime::panic::script_panic($msg);
        }
    };
}

/// Assert equality with Script panic
#[macro_export]
macro_rules! script_assert_eq {
    ($left:expr, $right:expr) => {
        if $left != $right {
            $crate::runtime::panic::script_panic(format!(
                "assertion failed: {} != {}",
                stringify!($left),
                stringify!($right)
            ));
        }
    };
    ($left:expr, $right:expr, $msg:expr) => {
        if $left != $right {
            $crate::runtime::panic::script_panic($msg);
        }
    };
}

/// Set a custom panic hook
pub fn set_panic_hook<F>(hook: F)
where
    F: Fn(&PanicInfo) + Send + Sync + 'static,
{
    if let Ok(handler) = PANIC_HANDLER.read() {
        if let Some(h) = handler.as_ref() {
            h.set_hook(hook);
        }
    }
}

/// Clear panic history
pub fn clear_panic_history() {
    if let Ok(handler) = PANIC_HANDLER.read() {
        if let Some(h) = handler.as_ref() {
            h.clear_history();
        }
    }
}

/// Set default recovery policy
pub fn set_default_recovery_policy(policy: RecoveryPolicy) {
    if let Ok(handler) = PANIC_HANDLER.read() {
        if let Some(h) = handler.as_ref() {
            h.set_default_recovery_policy(policy);
        }
    }
}

/// Add a recovery callback
pub fn add_recovery_callback<F>(callback: F) 
where 
    F: Fn(&RecoveryContext) -> RecoveryResult + Send + Sync + 'static,
{
    if let Ok(handler) = PANIC_HANDLER.read() {
        if let Some(h) = handler.as_ref() {
            h.add_recovery_callback(Box::new(callback));
        }
    }
}

/// Create a panic boundary
pub fn create_panic_boundary(id: String, policy: RecoveryPolicy) -> Option<PanicBoundary> {
    if let Ok(handler) = PANIC_HANDLER.read() {
        if let Some(h) = handler.as_ref() {
            return Some(h.create_boundary(id, policy));
        }
    }
    None
}

/// Get recovery metrics
pub fn get_recovery_metrics() -> Option<RecoveryMetrics> {
    if let Ok(handler) = PANIC_HANDLER.read() {
        if let Some(h) = handler.as_ref() {
            return Some(h.get_recovery_metrics());
        }
    }
    None
}

/// Execute code within a panic boundary
pub fn with_panic_boundary<F, R>(id: String, policy: RecoveryPolicy, f: F) -> Result<R, String>
where
    F: FnOnce() -> R + std::panic::UnwindSafe,
{
    // Create boundary
    let _boundary = create_panic_boundary(id, policy);
    
    // Execute with panic catching
    match std::panic::catch_unwind(f) {
        Ok(result) => Ok(result),
        Err(panic_info) => {
            // Convert panic info to string
            let panic_msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "Unknown panic".to_string()
            };
            
            Err(format!("Panic in boundary: {}", panic_msg))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic;

    #[test]
    fn test_panic_handler_lifecycle() {
        initialize();

        // Record a panic
        let info = PanicInfo {
            message: "Test panic".to_string(),
            location: Some("test.rs:42:10".to_string()),
            backtrace: "Test backtrace".to_string(),
            timestamp: Instant::now(),
            recovery_attempts: 0,
            recovered: false,
            recovery_policy: RecoveryPolicy::Abort,
        };
        record_panic(info.clone());

        // Check last panic
        let last = last_panic().unwrap();
        assert_eq!(last.message, "Test panic");

        // Check history
        let history = panic_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].message, "Test panic");

        shutdown();
    }

    #[test]
    fn test_panic_history_limit() {
        initialize();

        // Record more panics than the limit
        for i in 0..15 {
            let info = PanicInfo {
                message: format!("Panic {}", i),
                location: None,
                backtrace: "".to_string(),
                timestamp: Instant::now(),
                recovery_attempts: 0,
                recovered: false,
                recovery_policy: RecoveryPolicy::Abort,
            };
            record_panic(info);
        }

        // Should only keep the last MAX_PANIC_HISTORY panics
        let history = panic_history();
        assert_eq!(history.len(), MAX_PANIC_HISTORY);
        assert_eq!(history[0].message, "Panic 5"); // First kept panic
        assert_eq!(history[MAX_PANIC_HISTORY - 1].message, "Panic 14"); // Last panic

        shutdown();
    }

    #[test]
    fn test_stack_trace_capture() {
        let trace = StackTrace::capture();

        // Should have at least one frame
        assert!(!trace.frames.is_empty());

        // Should be able to format
        let formatted = trace.format();
        assert!(!formatted.is_empty());
    }

    #[test]
    fn test_custom_hook() {
        initialize();

        // Set up a custom hook that tracks calls
        let called = Arc::new(Mutex::new(false));
        let called_clone = called.clone();

        set_panic_hook(move |_info| {
            let mut c = called_clone.lock().unwrap();
            *c = true;
        });

        // Trigger a panic record
        let info = PanicInfo {
            message: "Hook test".to_string(),
            location: None,
            backtrace: "".to_string(),
            timestamp: Instant::now(),
            recovery_attempts: 0,
            recovered: false,
            recovery_policy: RecoveryPolicy::Abort,
        };
        record_panic(info);

        // Check that hook was called
        assert!(*called.lock().unwrap());

        shutdown();
    }

    #[test]
    #[should_panic(expected = "Test script panic")]
    fn test_script_panic() {
        initialize();
        script_panic("Test script panic");
    }

    #[test]
    fn test_panic_recovery() {
        initialize();

        // Use catch_unwind to test panic recovery
        let result = panic::catch_unwind(|| {
            script_panic("Recoverable panic");
        });

        assert!(result.is_err());

        // Should have recorded the panic
        let last = last_panic().unwrap();
        assert_eq!(last.message, "Recoverable panic");

        shutdown();
    }

    #[test]
    fn test_recovery_policies() {
        initialize();

        // Test continue policy
        let continue_info = PanicInfo {
            message: "Continue test".to_string(),
            location: None,
            backtrace: "".to_string(),
            timestamp: Instant::now(),
            recovery_attempts: 0,
            recovered: false,
            recovery_policy: RecoveryPolicy::Continue,
        };
        record_panic(continue_info);

        // Should have attempted recovery
        let last = last_panic().unwrap();
        assert_eq!(last.recovery_policy, RecoveryPolicy::Continue);

        shutdown();
    }

    #[test]
    fn test_recovery_metrics() {
        initialize();

        // Create a panic that will be recovered
        let info = PanicInfo {
            message: "Metrics test".to_string(),
            location: None,
            backtrace: "".to_string(),
            timestamp: Instant::now(),
            recovery_attempts: 0,
            recovered: false,
            recovery_policy: RecoveryPolicy::Continue,
        };
        record_panic(info);

        // Check metrics were updated
        let metrics = get_recovery_metrics().unwrap();
        assert!(metrics.total_panics > 0);

        shutdown();
    }

    #[test]
    fn test_panic_boundary() {
        initialize();

        // Test creating a panic boundary
        let boundary = create_panic_boundary("test_boundary".to_string(), RecoveryPolicy::Continue);
        assert!(boundary.is_some());

        let boundary = boundary.unwrap();
        assert_eq!(boundary.id, "test_boundary");
        assert_eq!(boundary.recovery_policy, RecoveryPolicy::Continue);

        shutdown();
    }

    #[test]
    fn test_with_panic_boundary() {
        initialize();

        // Test executing code within a panic boundary
        let result = with_panic_boundary(
            "test_execution".to_string(),
            RecoveryPolicy::Continue,
            || {
                // This should succeed
                42
            }
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);

        // Test panic within boundary
        let result = with_panic_boundary(
            "test_panic".to_string(),
            RecoveryPolicy::Continue,
            || {
                panic!("Test panic in boundary");
            }
        );

        assert!(result.is_err());

        shutdown();
    }

    #[test]
    fn test_recovery_callback() {
        initialize();

        // Add a recovery callback
        let callback_called = Arc::new(Mutex::new(false));
        let callback_called_clone = callback_called.clone();

        add_recovery_callback(move |_context| {
            let mut called = callback_called_clone.lock().unwrap();
            *called = true;
            RecoveryResult::Success
        });

        // Trigger a panic with custom recovery
        let info = PanicInfo {
            message: "Callback test".to_string(),
            location: None,
            backtrace: "".to_string(),
            timestamp: Instant::now(),
            recovery_attempts: 0,
            recovered: false,
            recovery_policy: RecoveryPolicy::Custom,
        };
        record_panic(info);

        // Check that callback was called
        assert!(*callback_called.lock().unwrap());

        shutdown();
    }

    #[test]
    fn test_default_recovery_policy() {
        initialize();

        // Test setting default recovery policy
        set_default_recovery_policy(RecoveryPolicy::Continue);

        // This test just ensures the API works - actual testing would require
        // integration with the panic system
        
        shutdown();
    }
}
