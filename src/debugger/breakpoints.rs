use crate::source::SourceLocation;
use crate::debugger::{DebuggerError, DebuggerResult};
use std::collections::HashMap;

/// Represents a single breakpoint
#[derive(Debug, Clone, PartialEq)]
pub struct Breakpoint {
    pub id: u32,
    pub location: BreakpointLocation,
    pub condition: Option<BreakpointCondition>,
    pub enabled: bool,
    pub hit_count: u32,
}

/// Different types of breakpoint locations
#[derive(Debug, Clone, PartialEq)]
pub enum BreakpointLocation {
    /// Line-based breakpoint (file:line)
    Line { file: String, line: usize },
    /// Function entry breakpoint
    Function { name: String },
    /// Source location breakpoint
    Source(SourceLocation),
}

/// Conditions for when a breakpoint should trigger
#[derive(Debug, Clone, PartialEq)]
pub enum BreakpointCondition {
    /// Always trigger (default)
    Always,
    /// Trigger when expression evaluates to true
    Expression(String),
    /// Trigger after N hits
    HitCount(u32),
}

/// Manages all breakpoints in the debugger
#[derive(Debug, Clone)]
pub struct BreakpointManager {
    breakpoints: HashMap<u32, Breakpoint>,
    next_id: u32,
}

impl BreakpointManager {
    /// Create a new breakpoint manager
    pub fn new() -> Self {
        Self {
            breakpoints: HashMap::new(),
            next_id: 1,
        }
    }

    /// Add a new breakpoint at the specified location
    pub fn add_breakpoint(&mut self, location: BreakpointLocation) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        let breakpoint = Breakpoint {
            id,
            location,
            condition: Some(BreakpointCondition::Always),
            enabled: true,
            hit_count: 0,
        };

        self.breakpoints.insert(id, breakpoint);
        id
    }

    /// Add a breakpoint with a condition
    pub fn add_conditional_breakpoint(
        &mut self,
        location: BreakpointLocation,
        condition: BreakpointCondition,
    ) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        let breakpoint = Breakpoint {
            id,
            location,
            condition: Some(condition),
            enabled: true,
            hit_count: 0,
        };

        self.breakpoints.insert(id, breakpoint);
        id
    }

    /// Remove a breakpoint by ID
    pub fn remove_breakpoint(&mut self, id: u32) -> DebuggerResult<Breakpoint> {
        self.breakpoints
            .remove(&id)
            .ok_or(DebuggerError::BreakpointNotFound(id))
    }

    /// Enable or disable a breakpoint
    pub fn set_breakpoint_enabled(&mut self, id: u32, enabled: bool) -> DebuggerResult<()> {
        match self.breakpoints.get_mut(&id) {
            Some(bp) => {
                bp.enabled = enabled;
                Ok(())
            }
            None => Err(DebuggerError::BreakpointNotFound(id)),
        }
    }

    /// Get a breakpoint by ID
    pub fn get_breakpoint(&self, id: u32) -> Option<&Breakpoint> {
        self.breakpoints.get(&id)
    }

    /// Get all breakpoints
    pub fn get_all_breakpoints(&self) -> impl Iterator<Item = &Breakpoint> {
        self.breakpoints.values()
    }

    /// Check if execution should stop at the given location
    pub fn should_break_at(&mut self, location: &SourceLocation) -> Option<u32> {
        for (id, breakpoint) in self.breakpoints.iter_mut() {
            if !breakpoint.enabled {
                continue;
            }

            let matches = match &breakpoint.location {
                BreakpointLocation::Source(bp_loc) => {
                    bp_loc.line == location.line && bp_loc.column == location.column
                }
                BreakpointLocation::Line { line, .. } => *line == location.line,
                BreakpointLocation::Function { .. } => false, // TODO: Implement function breakpoints
            };

            if matches {
                breakpoint.hit_count += 1;

                // Check condition
                let should_break = match &breakpoint.condition {
                    Some(BreakpointCondition::Always) => true,
                    Some(BreakpointCondition::HitCount(count)) => breakpoint.hit_count >= *count,
                    Some(BreakpointCondition::Expression(_)) => {
                        // TODO: Implement expression evaluation
                        true
                    }
                    None => true,
                };

                if should_break {
                    return Some(*id);
                }
            }
        }
        None
    }

    /// Parse a breakpoint location string (e.g., "file.script:10", "main", etc.)
    pub fn parse_location(&self, location_str: &str) -> DebuggerResult<BreakpointLocation> {
        // Try to parse as file:line
        if let Some(colon_pos) = location_str.rfind(':') {
            let file_part = &location_str[..colon_pos];
            let line_part = &location_str[colon_pos + 1..];

            if let Ok(line) = line_part.parse::<usize>() {
                return Ok(BreakpointLocation::Line {
                    file: file_part.to_string(),
                    line,
                });
            }
        }

        // Try to parse as function name
        if location_str.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Ok(BreakpointLocation::Function {
                name: location_str.to_string(),
            });
        }

        Err(DebuggerError::InvalidLocation(location_str.to_string())
    }

    /// Clear all breakpoints
    pub fn clear_all(&mut self) {
        self.breakpoints.clear();
    }

    /// Get the number of breakpoints
    pub fn count(&self) -> usize {
        self.breakpoints.len()
    }
}

impl Default for BreakpointManager {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for BreakpointLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BreakpointLocation::Line { file, line } => write!(f, "{}:{}", file, line),
            BreakpointLocation::Function { name } => write!(f, "fn {}", name),
            BreakpointLocation::Source(loc) => write!(f, "{}:{}", loc.line, loc.column),
        }
    }
}

impl std::fmt::Display for Breakpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.enabled { "enabled" } else { "disabled" };
        write!(f, "Breakpoint {} at {} [{}]", self.id, self.location, status)?;
        if self.hit_count > 0 {
            write!(f, " (hit {} times)", self.hit_count)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceLocation;

    #[test]
    fn test_breakpoint_manager_basic() {
        let mut manager = BreakpointManager::new();

        // Add a breakpoint
        let location = BreakpointLocation::Line {
            file: "test.script".to_string(),
            line: 10,
        };
        let id = manager.add_breakpoint(location);
        assert_eq!(id, 1);
        assert_eq!(manager.count(), 1);

        // Get the breakpoint
        let bp = manager.get_breakpoint(id).unwrap();
        assert_eq!(bp.id, 1);
        assert!(bp.enabled);
        assert_eq!(bp.hit_count, 0);

        // Remove the breakpoint
        let removed = manager.remove_breakpoint(id).unwrap();
        assert_eq!(removed.id, 1);
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_breakpoint_location_parsing() {
        let manager = BreakpointManager::new();

        // Test file:line parsing
        let location = manager.parse_location("test.script:42").unwrap();
        match location {
            BreakpointLocation::Line { file, line } => {
                assert_eq!(file, "test.script");
                assert_eq!(line, 42);
            }
            _ => panic!("Expected line breakpoint"),
        }

        // Test function name parsing
        let location = manager.parse_location("main").unwrap();
        match location {
            BreakpointLocation::Function { name } => {
                assert_eq!(name, "main");
            }
            _ => panic!("Expected function breakpoint"),
        }

        // Test invalid location
        assert!(manager.parse_location("invalid:location:format").is_err());
    }

    #[test]
    fn test_should_break_at() {
        let mut manager = BreakpointManager::new();

        // Add a breakpoint at line 10
        let location = BreakpointLocation::Line {
            file: "test.script".to_string(),
            line: 10,
        };
        let id = manager.add_breakpoint(location);

        // Check if we should break at line 10
        let source_loc = SourceLocation::new(10, 1);
        let break_id = manager.should_break_at(&source_loc);
        assert_eq!(break_id, Some(id));

        // Check hit count was incremented
        let bp = manager.get_breakpoint(id).unwrap();
        assert_eq!(bp.hit_count, 1);

        // Check we don't break at line 11
        let source_loc = SourceLocation::new(11, 1);
        let break_id = manager.should_break_at(&source_loc);
        assert_eq!(break_id, None);
    }

    #[test]
    fn test_breakpoint_enable_disable() {
        let mut manager = BreakpointManager::new();

        let location = BreakpointLocation::Line {
            file: "test.script".to_string(),
            line: 10,
        };
        let id = manager.add_breakpoint(location);

        // Disable the breakpoint
        manager.set_breakpoint_enabled(id, false).unwrap();
        let bp = manager.get_breakpoint(id).unwrap();
        assert!(!bp.enabled);

        // Should not break when disabled
        let source_loc = SourceLocation::new(10, 1);
        let break_id = manager.should_break_at(&source_loc);
        assert_eq!(break_id, None);

        // Re-enable and check it works
        manager.set_breakpoint_enabled(id, true).unwrap();
        let break_id = manager.should_break_at(&source_loc);
        assert_eq!(break_id, Some(id));
    }

    #[test]
    fn test_conditional_breakpoint() {
        let mut manager = BreakpointManager::new();

        let location = BreakpointLocation::Line {
            file: "test.script".to_string(),
            line: 10,
        };
        let condition = BreakpointCondition::HitCount(3);
        let id = manager.add_conditional_breakpoint(location, condition);

        let source_loc = SourceLocation::new(10, 1);

        // Should not break on first two hits
        assert_eq!(manager.should_break_at(&source_loc), None);
        assert_eq!(manager.should_break_at(&source_loc), None);

        // Should break on third hit
        assert_eq!(manager.should_break_at(&source_loc), Some(id));
    }
}