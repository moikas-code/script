//! Panic handler for Script runtime
//! 
//! This module provides panic handling with stack traces and error recovery
//! for Script programs. It integrates with Rust's panic system while providing
//! Script-specific context and debugging information.

use std::sync::{Arc, Mutex, RwLock};
use std::panic;
use std::backtrace::{Backtrace, BacktraceStatus};
use std::collections::VecDeque;
use std::fmt;

/// Global panic handler instance
static PANIC_HANDLER: RwLock<Option<Arc<PanicHandler>>> = RwLock::new(None);

/// Maximum number of recent panics to keep
const MAX_PANIC_HISTORY: usize = 10;

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
}

impl PanicHandler {
    /// Create a new panic handler
    fn new() -> Self {
        PanicHandler {
            history: Mutex::new(VecDeque::with_capacity(MAX_PANIC_HISTORY)),
            custom_hook: RwLock::new(None),
        }
    }
    
    /// Record a panic
    fn record_panic(&self, info: PanicInfo) {
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
                if !frame.function.contains("script_lang::runtime") &&
                   !frame.function.contains("std::panic") &&
                   !frame.function.contains("rust_begin_unwind") {
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
                    output.push_str(&format!(":{}",

 line));
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
    let info = PanicInfo {
        message: message.into(),
        location: None,
        backtrace: StackTrace::capture().format(),
    };
    
    record_panic(info.clone());
    panic!("{}", info.message);
}

/// Create a Script panic with location
pub fn script_panic_at(message: impl Into<String>, file: &str, line: u32, column: u32) -> ! {
    let info = PanicInfo {
        message: message.into(),
        location: Some(format!("{}:{}:{}", file, line, column)),
        backtrace: StackTrace::capture().format(),
    };
    
    record_panic(info.clone());
    panic!("{}", info.message);
}

/// Assert with Script panic
#[macro_export]
macro_rules! script_assert {
    ($cond:expr) => {
        if !$cond {
            $crate::runtime::panic::script_panic(
                concat!("assertion failed: ", stringify!($cond))
            );
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
            $crate::runtime::panic::script_panic(
                format!("assertion failed: {} != {}", 
                    stringify!($left), 
                    stringify!($right)
                )
            );
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
}