//! Breakpoint data structures and types
//!
//! This module defines the core breakpoint types and structures used
//! throughout the debugger system. It supports different types of
//! breakpoints including line breakpoints, function breakpoints,
//! and conditional breakpoints.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::source::SourceLocation;

/// Unique identifier for breakpoints
pub type BreakpointId = usize;

/// Represents a breakpoint in the debugger
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Breakpoint {
    /// Unique identifier for this breakpoint
    pub id: BreakpointId,
    /// Type of breakpoint
    pub breakpoint_type: BreakpointType,
    /// Whether this breakpoint is enabled
    pub enabled: bool,
    /// Optional condition that must be true for the breakpoint to trigger
    pub condition: Option<BreakpointCondition>,
    /// Hit count - how many times this breakpoint has been hit
    pub hit_count: usize,
    /// Optional message to display when breakpoint is hit
    pub message: Option<String>,
    /// Whether to log the message without stopping execution
    pub log_message: bool,
}

/// Different types of breakpoints supported by the debugger
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BreakpointType {
    /// Line breakpoint - breaks at a specific line in a file
    Line {
        /// File path
        file: String,
        /// Line number (1-based)
        line: usize,
    },
    /// Function entry breakpoint - breaks when entering a function
    Function {
        /// Function name
        name: String,
        /// Optional file to restrict the breakpoint to
        file: Option<String>,
    },
    /// Address breakpoint - breaks at a specific memory address (for low-level debugging)
    Address {
        /// Memory address
        address: usize,
    },
    /// Exception breakpoint - breaks when an exception is thrown
    Exception {
        /// Exception type to break on (None for all exceptions)
        exception_type: Option<String>,
    },
}

/// Condition for conditional breakpoints
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BreakpointCondition {
    /// Script expression that must evaluate to true
    pub expression: String,
    /// Whether the condition should be evaluated in the current scope
    pub use_current_scope: bool,
}

/// Information about a breakpoint hit
#[derive(Debug, Clone)]
pub struct BreakpointHit {
    /// The breakpoint that was hit
    pub breakpoint: Breakpoint,
    /// Location where the breakpoint was hit
    pub location: SourceLocation,
    /// Function name where the breakpoint was hit (if available)
    pub function_name: Option<String>,
    /// Thread ID (for multi-threaded debugging)
    pub thread_id: Option<usize>,
    /// Timestamp when the breakpoint was hit
    pub timestamp: std::time::Instant,
}

impl Breakpoint {
    /// Create a new line breakpoint
    pub fn line(id: BreakpointId, file: String, line: usize) -> Self {
        Breakpoint {
            id,
            breakpoint_type: BreakpointType::Line { file, line },
            enabled: true,
            condition: None,
            hit_count: 0,
            message: None,
            log_message: false,
        }
    }

    /// Create a new function breakpoint
    pub fn function(id: BreakpointId, name: String, file: Option<String>) -> Self {
        Breakpoint {
            id,
            breakpoint_type: BreakpointType::Function { name, file },
            enabled: true,
            condition: None,
            hit_count: 0,
            message: None,
            log_message: false,
        }
    }

    /// Create a new address breakpoint
    pub fn address(id: BreakpointId, address: usize) -> Self {
        Breakpoint {
            id,
            breakpoint_type: BreakpointType::Address { address },
            enabled: true,
            condition: None,
            hit_count: 0,
            message: None,
            log_message: false,
        }
    }

    /// Create a new exception breakpoint
    pub fn exception(id: BreakpointId, exception_type: Option<String>) -> Self {
        Breakpoint {
            id,
            breakpoint_type: BreakpointType::Exception { exception_type },
            enabled: true,
            condition: None,
            hit_count: 0,
            message: None,
            log_message: false,
        }
    }

    /// Enable this breakpoint
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable this breakpoint
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Toggle the enabled state of this breakpoint
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    /// Add a condition to this breakpoint
    pub fn set_condition(&mut self, condition: BreakpointCondition) {
        self.condition = Some(condition);
    }

    /// Remove the condition from this breakpoint
    pub fn clear_condition(&mut self) {
        self.condition = None;
    }

    /// Set a message to display when this breakpoint is hit
    pub fn set_message(&mut self, message: String, log_only: bool) {
        self.message = Some(message);
        self.log_message = log_only;
    }

    /// Clear the message for this breakpoint
    pub fn clear_message(&mut self) {
        self.message = None;
        self.log_message = false;
    }

    /// Increment the hit count
    pub fn hit(&mut self) {
        self.hit_count += 1;
    }

    /// Reset the hit count
    pub fn reset_hit_count(&mut self) {
        self.hit_count = 0;
    }

    /// Check if this breakpoint matches the given location
    pub fn matches_location(
        &self,
        location: SourceLocation,
        file: Option<&str>,
        function_name: Option<&str>,
    ) -> bool {
        if !self.enabled {
            return false;
        }

        match &self.breakpoint_type {
            BreakpointType::Line {
                file: bp_file,
                line,
            } => {
                if let Some(file) = file {
                    file == bp_file && location.line == *line
                } else {
                    false
                }
            }
            BreakpointType::Function {
                name,
                file: bp_file,
            } => {
                if let Some(func_name) = function_name {
                    let name_matches = func_name == name;
                    let file_matches = bp_file
                        .as_ref()
                        .map(|f| file.map(|current_file| current_file == f).unwrap_or(false))
                        .unwrap_or(true);
                    name_matches && file_matches
                } else {
                    false
                }
            }
            BreakpointType::Address { .. } => {
                // Address breakpoints require special handling at the runtime level
                false
            }
            BreakpointType::Exception { .. } => {
                // Exception breakpoints are handled differently
                false
            }
        }
    }

    /// Get a human-readable description of this breakpoint
    pub fn description(&self) -> String {
        match &self.breakpoint_type {
            BreakpointType::Line { file, line } => {
                format!("Line breakpoint at {}:{}", file, line)
            }
            BreakpointType::Function { name, file } => {
                if let Some(file) = file {
                    format!("Function breakpoint at '{}' in {}", name, file)
                } else {
                    format!("Function breakpoint at '{}'", name)
                }
            }
            BreakpointType::Address { address } => {
                format!("Address breakpoint at 0x{}", address, :x)
            }
            BreakpointType::Exception { exception_type } => {
                if let Some(ex_type) = exception_type {
                    format!("Exception breakpoint for {}", ex_type)
                } else {
                    "Exception breakpoint for all exceptions".to_string()
                }
            }
        }
    }

    /// Get the file path associated with this breakpoint (if any)
    pub fn file_path(&self) -> Option<&str> {
        match &self.breakpoint_type {
            BreakpointType::Line { file, .. } => Some(file),
            BreakpointType::Function { file, .. } => file.as_deref(),
            _ => None,
        }
    }

    /// Get the line number associated with this breakpoint (if any)
    pub fn line_number(&self) -> Option<usize> {
        match &self.breakpoint_type {
            BreakpointType::Line { line, .. } => Some(*line),
            _ => None,
        }
    }
}

impl BreakpointCondition {
    /// Create a new breakpoint condition
    pub fn new(expression: String, use_current_scope: bool) -> Self {
        BreakpointCondition {
            expression,
            use_current_scope,
        }
    }

    /// Evaluate the condition (placeholder for now)
    ///
    /// In a full implementation, this would compile and evaluate
    /// the condition expression against the current runtime state.
    pub fn evaluate(&self, _context: &BreakpointEvaluationContext) -> Result<bool, String> {
        // TODO: Implement condition evaluation
        // For now, always return true
        println!("Evaluating condition: {self.expression}");
        Ok(true)
    }
}

/// Context for evaluating breakpoint conditions
#[derive(Debug, Clone)]
pub struct BreakpointEvaluationContext {
    /// Current variables in scope
    pub variables: std::collections::HashMap<String, String>, // Simplified for now
    /// Current location
    pub location: SourceLocation,
    /// Current function name
    pub function_name: Option<String>,
}

impl BreakpointHit {
    /// Create a new breakpoint hit record
    pub fn new(
        breakpoint: Breakpoint,
        location: SourceLocation,
        function_name: Option<String>,
        thread_id: Option<usize>,
    ) -> Self {
        BreakpointHit {
            breakpoint,
            location,
            function_name,
            thread_id,
            timestamp: std::time::Instant::now(),
        }
    }

    /// Get a human-readable description of this breakpoint hit
    pub fn description(&self) -> String {
        let base = format!("Breakpoint {} hit at {}", self.breakpoint.id, self.location);
        if let Some(function) = &self.function_name {
            format!("{} in function '{}'", base, function)
        } else {
            base
        }
    }
}

impl fmt::Display for Breakpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Breakpoint #{}: {}", self.id, self.description())?;
        if !self.enabled {
            write!(f, " (disabled)")?;
        }
        if self.hit_count > 0 {
            write!(f, " [hit {} times]", self.hit_count)?;
        }
        if let Some(condition) = &self.condition {
            write!(f, " when {}", condition.expression)?;
        }
        Ok(())
    }
}

impl fmt::Display for BreakpointType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BreakpointType::Line { file, line } => write!(f, "{}:{}", file, line),
            BreakpointType::Function { name, file } => {
                if let Some(file) = file {
                    write!(f, "{}() in {}", name, file)
                } else {
                    write!(f, "{}()", name)
                }
            }
            BreakpointType::Address { address } => write!(f, "0x{:x}", address),
            BreakpointType::Exception { exception_type } => {
                if let Some(ex_type) = exception_type {
                    write!(f, "exception {}", ex_type)
                } else {
                    write!(f, "all exceptions")
                }
            }
        }
    }
}

impl fmt::Display for BreakpointHit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_breakpoint_creation() {
        let bp = Breakpoint::line(1, "test.script".to_string(), 42);
        assert_eq!(bp.id, 1);
        assert!(bp.enabled);
        assert_eq!(bp.hit_count, 0);
        assert!(bp.condition.is_none());

        match bp.breakpoint_type {
            BreakpointType::Line { file, line } => {
                assert_eq!(file, "test.script");
                assert_eq!(line, 42);
            }
            _ => panic!("Expected line breakpoint"),
        }
    }

    #[test]
    fn test_function_breakpoint_creation() {
        let bp = Breakpoint::function(2, "main".to_string(), Some("test.script".to_string()));
        assert_eq!(bp.id, 2);

        match bp.breakpoint_type {
            BreakpointType::Function { name, file } => {
                assert_eq!(name, "main");
                assert_eq!(file, Some("test.script".to_string()));
            }
            _ => panic!("Expected function breakpoint"),
        }
    }

    #[test]
    fn test_breakpoint_state_management() {
        let mut bp = Breakpoint::line(1, "test.script".to_string(), 10);

        // Test enable/disable
        assert!(bp.enabled);
        bp.disable();
        assert!(!bp.enabled);
        bp.enable();
        assert!(bp.enabled);
        bp.toggle();
        assert!(!bp.enabled);

        // Test hit counting
        assert_eq!(bp.hit_count, 0);
        bp.hit();
        assert_eq!(bp.hit_count, 1);
        bp.hit();
        assert_eq!(bp.hit_count, 2);
        bp.reset_hit_count();
        assert_eq!(bp.hit_count, 0);
    }

    #[test]
    fn test_breakpoint_conditions() {
        let mut bp = Breakpoint::line(1, "test.script".to_string(), 10);

        // Test condition management
        assert!(bp.condition.is_none());
        let condition = BreakpointCondition::new("x > 10".to_string(), true);
        bp.set_condition(condition);
        assert!(bp.condition.is_some());
        bp.clear_condition();
        assert!(bp.condition.is_none());
    }

    #[test]
    fn test_breakpoint_location_matching() {
        let line_bp = Breakpoint::line(1, "test.script".to_string(), 42);
        let func_bp = Breakpoint::function(2, "main".to_string(), None);

        let location = SourceLocation::new(42, 1, 0);

        // Test line breakpoint matching
        assert!(line_bp.matches_location(location, Some("test.script"), None));
        assert!(!line_bp.matches_location(location, Some("other.script"), None));
        assert!(!line_bp.matches_location(
            SourceLocation::new(43, 1, 0),
            Some("test.script"),
            None
        ));

        // Test function breakpoint matching
        assert!(func_bp.matches_location(location, Some("test.script"), Some("main")));
        assert!(!func_bp.matches_location(location, Some("test.script"), Some("other")));
        assert!(!func_bp.matches_location(location, Some("test.script"), None));
    }

    #[test]
    fn test_breakpoint_description() {
        let line_bp = Breakpoint::line(1, "test.script".to_string(), 42);
        let func_bp = Breakpoint::function(2, "main".to_string(), Some("test.script".to_string()));
        let addr_bp = Breakpoint::address(3, 0x1000);
        let exc_bp = Breakpoint::exception(4, Some("RuntimeError".to_string()));

        assert_eq!(line_bp.description(), "Line breakpoint at test.script:42");
        assert_eq!(
            func_bp.description(),
            "Function breakpoint at 'main' in test.script"
        );
        assert_eq!(addr_bp.description(), "Address breakpoint at 0x1000");
        assert_eq!(
            exc_bp.description(),
            "Exception breakpoint for RuntimeError"
        );
    }

    #[test]
    fn test_breakpoint_hit_creation() {
        let bp = Breakpoint::line(1, "test.script".to_string(), 42);
        let location = SourceLocation::new(42, 1, 0);
        let hit = BreakpointHit::new(bp, location, Some("main".to_string()), Some(1));

        assert_eq!(hit.breakpoint.id, 1);
        assert_eq!(hit.location, location);
        assert_eq!(hit.function_name, Some("main".to_string()));
        assert_eq!(hit.thread_id, Some(1));

        let description = hit.description();
        assert!(description.contains("Breakpoint 1 hit"));
        assert!(description.contains("42:1"));
        assert!(description.contains("function 'main'"));
    }
}
