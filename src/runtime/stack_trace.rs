//! Runtime stack trace implementation for Script
//!
//! This module provides stack trace functionality for runtime errors,
//! including capturing call frames, formatting stack traces, and
//! integrating with the panic handling system.

use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};

/// Maximum number of stack frames to capture
const MAX_STACK_FRAMES: usize = 100;

/// A single frame in the call stack
#[derive(Debug, Clone, PartialEq)]
pub struct StackFrame {
    /// Function name
    pub function_name: String,
    /// File name (if available)
    pub file_name: Option<String>,
    /// Line number (if available)
    pub line_number: Option<u32>,
    /// Column number (if available)
    pub column_number: Option<u32>,
    /// Module name (if available)
    pub module_name: Option<String>,
}

/// Complete stack trace with multiple frames
#[derive(Debug, Clone)]
pub struct StackTrace {
    /// Stack frames (top to bottom)
    pub frames: Vec<StackFrame>,
    /// Timestamp when trace was captured
    pub timestamp: std::time::Instant,
    /// Total frame count (may be truncated)
    pub total_frames: usize,
}

impl PartialEq for StackTrace {
    fn eq(&self, other: &Self) -> bool {
        self.frames == other.frames && self.total_frames == other.total_frames
        // Note: We don't compare timestamps as they represent capture time, not content
    }
}

/// Stack trace builder for accumulating frames
#[derive(Debug)]
pub struct StackTraceBuilder {
    frames: Vec<StackFrame>,
    max_frames: usize,
}

/// Runtime stack tracker - maintains current call stack
pub struct RuntimeStackTracker {
    /// Current call stack
    stack: Arc<Mutex<VecDeque<StackFrame>>>,
    /// Stack trace configuration
    config: StackTraceConfig,
}

/// Configuration for stack trace capture
#[derive(Debug, Clone)]
pub struct StackTraceConfig {
    /// Maximum frames to capture
    pub max_frames: usize,
    /// Whether to capture file information
    pub capture_files: bool,
    /// Whether to capture line numbers
    pub capture_lines: bool,
    /// Whether to include system functions
    pub include_system: bool,
}

impl Default for StackTraceConfig {
    fn default() -> Self {
        Self {
            max_frames: MAX_STACK_FRAMES,
            capture_files: true,
            capture_lines: true,
            include_system: false,
        }
    }
}

impl StackFrame {
    /// Create a new stack frame with just a function name
    pub fn new(function_name: String) -> Self {
        Self {
            function_name,
            file_name: None,
            line_number: None,
            column_number: None,
            module_name: None,
        }
    }

    /// Create a stack frame with file and line information
    pub fn with_location(
        function_name: String,
        file_name: String,
        line_number: u32,
        column_number: Option<u32>,
    ) -> Self {
        Self {
            function_name,
            file_name: Some(file_name),
            line_number: Some(line_number),
            column_number,
            module_name: None,
        }
    }

    /// Add module information to the frame
    pub fn with_module(mut self, module_name: String) -> Self {
        self.module_name = Some(module_name);
        self
    }

    /// Format the frame for display
    pub fn format_location(&self) -> String {
        match (&self.file_name, &self.line_number) {
            (Some(file), Some(line)) => {
                if let Some(col) = self.column_number {
                    format!("{}:{}:{file, line, col}")
                } else {
                    format!("{}:{file, line}")
                }
            }
            (Some(file), None) => file.clone(),
            _ => "<unknown>".to_string(),
        }
    }
}

impl fmt::Display for StackFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(module) = &self.module_name {
            write!(f, "{}::{}", module, self.function_name)?;
        } else {
            write!(f, "{}", self.function_name)?;
        }

        let location = self.format_location();
        if location != "<unknown>" {
            write!(f, " at {}", location)?;
        }

        Ok(())
    }
}

impl StackTrace {
    /// Create a new empty stack trace
    pub fn new() -> Self {
        Self {
            frames: Vec::new(),
            timestamp: std::time::Instant::now(),
            total_frames: 0,
        }
    }

    /// Create a stack trace from captured frames
    pub fn from_frames(frames: Vec<StackFrame>) -> Self {
        let total_frames = frames.len();
        Self {
            frames,
            timestamp: std::time::Instant::now(),
            total_frames,
        }
    }

    /// Capture the current system stack trace (fallback for native traces)
    pub fn capture_system() -> Self {
        let mut frames = Vec::new();
        let backtrace = std::backtrace::Backtrace::capture();

        // Parse the backtrace string to extract frame information
        // This is a simplified implementation - in production you'd want
        // more sophisticated parsing or use a crate like backtrace-rs
        let trace_str = backtrace.to_string();
        for (i, line) in trace_str.lines().enumerate() {
            if i >= MAX_STACK_FRAMES {
                break;
            }

            if line.trim().is_empty() {
                continue;
            }

            // Extract function name from backtrace line
            // Format is typically: " at function_name (file:line:col)"
            let function_name = if let Some(start) = line.find(" at ") {
                let rest = &line[start + 4..];
                if let Some(end) = rest.find(" (") {
                    rest[..end].trim().to_string()
                } else {
                    rest.trim().to_string()
                }
            } else {
                format!("frame_{i}")
            };

            frames.push(StackFrame::new(function_name));
        }

        Self::from_frames(frames)
    }

    /// Get the top frame (most recent call)
    pub fn top_frame(&self) -> Option<&StackFrame> {
        self.frames.first()
    }

    /// Get frames starting from the top (most recent)
    pub fn frames(&self) -> &[StackFrame] {
        &self.frames
    }

    /// Check if the stack trace was truncated
    pub fn is_truncated(&self) -> bool {
        self.frames.len() < self.total_frames
    }

    /// Format the stack trace for display
    pub fn format_trace(&self) -> String {
        if self.frames.is_empty() {
            return "Stack trace unavailable".to_string();
        }

        let mut result = String::new();
        result.push_str("Stack trace:\n");

        for (i, frame) in self.frames.iter().enumerate() {
            result.push_str(&format!("  {} - {}\n", i, frame));
        }

        if self.is_truncated() {
            result.push_str(&format!(
                "  ... ({} more frames)\n",
                self.total_frames - self.frames.len()
            ));
        }

        result
    }
}

impl fmt::Display for StackTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_trace())
    }
}

impl StackTraceBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::with_max_frames(MAX_STACK_FRAMES)
    }

    /// Create a builder with custom max frames
    pub fn with_max_frames(max_frames: usize) -> Self {
        Self {
            frames: Vec::with_capacity(max_frames),
            max_frames,
        }
    }

    /// Add a frame to the trace
    pub fn push_frame(&mut self, frame: StackFrame) {
        if self.frames.len() < self.max_frames {
            self.frames.push(frame);
        }
    }

    /// Add a simple function frame
    pub fn push_function(&mut self, function_name: String) {
        self.push_frame(StackFrame::new(function_name));
    }

    /// Add a frame with location information
    pub fn push_location(
        &mut self,
        function_name: String,
        file_name: String,
        line_number: u32,
        column_number: Option<u32>,
    ) {
        self.push_frame(StackFrame::with_location(
            function_name,
            file_name,
            line_number,
            column_number,
        ));
    }

    /// Build the final stack trace
    pub fn build(self) -> StackTrace {
        StackTrace::from_frames(self.frames)
    }
}

impl RuntimeStackTracker {
    /// Create a new stack tracker
    pub fn new() -> Self {
        Self::with_config(StackTraceConfig::default())
    }

    /// Create a stack tracker with custom configuration
    pub fn with_config(config: StackTraceConfig) -> Self {
        Self {
            stack: Arc::new(Mutex::new(VecDeque::new())),
            config,
        }
    }

    /// Push a frame onto the current stack
    pub fn push_frame(&self, frame: StackFrame) {
        if let Ok(mut stack) = self.stack.lock() {
            // Keep stack size bounded
            if stack.len() >= self.config.max_frames {
                stack.pop_back();
            }
            stack.push_front(frame);
        }
    }

    /// Pop the top frame from the stack
    pub fn pop_frame(&self) -> Option<StackFrame> {
        if let Ok(mut stack) = self.stack.lock() {
            stack.pop_front()
        } else {
            None
        }
    }

    /// Capture the current stack trace
    pub fn capture_trace(&self) -> StackTrace {
        if let Ok(stack) = self.stack.lock() {
            let frames: Vec<StackFrame> = stack.iter().cloned().collect();
            StackTrace::from_frames(frames)
        } else {
            StackTrace::new()
        }
    }

    /// Get the current stack depth
    pub fn depth(&self) -> usize {
        if let Ok(stack) = self.stack.lock() {
            stack.len()
        } else {
            0
        }
    }

    /// Clear the current stack
    pub fn clear(&self) {
        if let Ok(mut stack) = self.stack.lock() {
            stack.clear();
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &StackTraceConfig {
        &self.config
    }
}

/// Global stack tracker instance
static GLOBAL_STACK_TRACKER: RwLock<Option<Arc<RuntimeStackTracker>>> = RwLock::new(None);

/// Initialize the global stack tracker
pub fn initialize_stack_tracker() -> Result<(), String> {
    let mut tracker = GLOBAL_STACK_TRACKER
        .write()
        .map_err(|_| "Failed to acquire write lock for stack tracker")?;

    if tracker.is_some() {
        return Err("Stack tracker already initialized".to_string());
    }

    *tracker = Some(Arc::new(RuntimeStackTracker::new()));
    Ok(())
}

/// Get the global stack tracker
pub fn get_stack_tracker() -> Option<Arc<RuntimeStackTracker>> {
    GLOBAL_STACK_TRACKER.read().ok()?.as_ref().cloned()
}

/// Capture the current stack trace (global)
pub fn capture_current_trace() -> StackTrace {
    if let Some(tracker) = get_stack_tracker() {
        tracker.capture_trace()
    } else {
        // Fallback to system backtrace
        StackTrace::capture_system()
    }
}

/// RAII guard for automatic stack frame management
pub struct StackGuard {
    tracker: Arc<RuntimeStackTracker>,
}

impl StackGuard {
    /// Create a new stack guard that automatically pushes/pops frames
    pub fn new(tracker: Arc<RuntimeStackTracker>, frame: StackFrame) -> Self {
        tracker.push_frame(frame);
        Self { tracker }
    }

    /// Create a guard for a function call
    pub fn for_function(function_name: String) -> Option<Self> {
        let tracker = get_stack_tracker()?;
        let frame = StackFrame::new(function_name);
        Some(Self::new(tracker, frame))
    }

    /// Create a guard with location information
    pub fn for_location(
        function_name: String,
        file_name: String,
        line_number: u32,
        column_number: Option<u32>,
    ) -> Option<Self> {
        let tracker = get_stack_tracker()?;
        let frame = StackFrame::with_location(function_name, file_name, line_number, column_number);
        Some(Self::new(tracker, frame))
    }
}

impl Drop for StackGuard {
    fn drop(&mut self) {
        self.tracker.pop_frame();
    }
}

/// Macro for easy stack frame creation
#[macro_export]
macro_rules! stack_frame {
    ($func:expr) => {
        $crate::runtime::stack_trace::StackGuard::for_function($func.to_string())
    };
    ($func:expr, $file:expr, $line:expr) => {
        $crate::runtime::stack_trace::StackGuard::for_location(
            $func.to_string(),
            $file.to_string(),
            $line,
            None,
        )
    };
    ($func:expr, $file:expr, $line:expr, $col:expr) => {
        $crate::runtime::stack_trace::StackGuard::for_location(
            $func.to_string(),
            $file.to_string(),
            $line,
            Some($col),
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_frame_creation() {
        let frame = StackFrame::new("test_function".to_string());
        assert_eq!(frame.function_name, "test_function");
        assert!(frame.file_name.is_none());
        assert!(frame.line_number.is_none());
    }

    #[test]
    fn test_stack_frame_with_location() {
        let frame = StackFrame::with_location(
            "test_function".to_string(),
            "test.script".to_string(),
            42,
            Some(10),
        );
        assert_eq!(frame.function_name, "test_function");
        assert_eq!(frame.file_name, Some("test.script".to_string()));
        assert_eq!(frame.line_number, Some(42));
        assert_eq!(frame.column_number, Some(10));
    }

    #[test]
    fn test_stack_frame_display() {
        let frame = StackFrame::with_location(
            "test_function".to_string(),
            "test.script".to_string(),
            42,
            Some(10),
        );
        let display = frame.to_string();
        assert!(display.contains("test_function"));
        assert!(display.contains("test.script:42:10"));
    }

    #[test]
    fn test_stack_trace_builder() {
        let mut builder = StackTraceBuilder::new();
        builder.push_function("main".to_string());
        builder.push_function("helper".to_string());

        let trace = builder.build();
        assert_eq!(trace.frames.len(), 2);
        assert_eq!(trace.frames[0].function_name, "main");
        assert_eq!(trace.frames[1].function_name, "helper");
    }

    #[test]
    fn test_runtime_stack_tracker() {
        let tracker = RuntimeStackTracker::new();
        assert_eq!(tracker.depth(), 0);

        let frame1 = StackFrame::new("function1".to_string());
        let frame2 = StackFrame::new("function2".to_string());

        tracker.push_frame(frame1.clone());
        assert_eq!(tracker.depth(), 1);

        tracker.push_frame(frame2.clone());
        assert_eq!(tracker.depth(), 2);

        let trace = tracker.capture_trace();
        assert_eq!(trace.frames.len(), 2);
        assert_eq!(trace.frames[0].function_name, "function2"); // Most recent first

        let popped = tracker.pop_frame();
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().function_name, "function2");
        assert_eq!(tracker.depth(), 1);
    }

    #[test]
    fn test_stack_guard() {
        // Clean up any existing tracker
        {
            let mut tracker = GLOBAL_STACK_TRACKER.write().unwrap();
            *tracker = None;
        }

        // Initialize fresh tracker
        initialize_stack_tracker().unwrap();

        let initial_depth = if let Some(tracker) = get_stack_tracker() {
            tracker.depth()
        } else {
            0
        };

        {
            let _guard = StackGuard::for_function("test_function".to_string());

            if let Some(tracker) = get_stack_tracker() {
                assert_eq!(tracker.depth(), initial_depth + 1);
                let trace = tracker.capture_trace();
                assert_eq!(trace.frames[0].function_name, "test_function");
            }
        } // guard drops here

        if let Some(tracker) = get_stack_tracker() {
            assert_eq!(tracker.depth(), initial_depth);
        }
    }

    #[test]
    fn test_stack_trace_format() {
        let mut builder = StackTraceBuilder::new();
        builder.push_location("main".to_string(), "main.script".to_string(), 10, Some(5));
        builder.push_function("helper".to_string());

        let trace = builder.build();
        let formatted = trace.format_trace();

        assert!(formatted.contains("Stack trace:"));
        assert!(formatted.contains("main"));
        assert!(formatted.contains("helper"));
        assert!(formatted.contains("main.script:10:5"));
    }
}
