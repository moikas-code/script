//! Breakpoint manager for the Script debugger
//!
//! This module provides the BreakpointManager which handles storage,
//! management, and querying of breakpoints. It's designed to be
//! thread-safe and efficient for runtime breakpoint checking.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, RwLock};

use crate::debugger::breakpoint::{
    Breakpoint, BreakpointCondition, BreakpointEvaluationContext, BreakpointHit, BreakpointId,
    BreakpointType,
};
use crate::error::{Error, Result};
use crate::source::SourceLocation;

/// Manages all breakpoints in the debugger
pub struct BreakpointManager {
    /// Storage for all breakpoints indexed by ID
    breakpoints: RwLock<HashMap<BreakpointId, Breakpoint>>,
    /// Index of line breakpoints by file path
    line_breakpoints_by_file: RwLock<HashMap<String, HashSet<BreakpointId>>>,
    /// Index of function breakpoints by function name
    function_breakpoints_by_name: RwLock<HashMap<String, HashSet<BreakpointId>>>,
    /// Next available breakpoint ID
    next_id: Mutex<BreakpointId>,
    /// History of breakpoint hits
    hit_history: Mutex<Vec<BreakpointHit>>,
    /// Maximum number of hits to keep in history
    max_history_size: usize,
}

impl BreakpointManager {
    /// Create a new breakpoint manager
    pub fn new() -> Self {
        BreakpointManager {
            breakpoints: RwLock::new(HashMap::new()),
            line_breakpoints_by_file: RwLock::new(HashMap::new()),
            function_breakpoints_by_name: RwLock::new(HashMap::new()),
            next_id: Mutex::new(1),
            hit_history: Mutex::new(Vec::new()),
            max_history_size: 1000, // Keep last 1000 hits
        }
    }

    /// Add a line breakpoint
    pub fn add_line_breakpoint(&self, file: String, line: usize) -> Result<BreakpointId> {
        // Validate the file path
        if file.is_empty() {
            return Err(Error::invalid_conversion("File path cannot be empty"));
        }

        if line == 0 {
            return Err(Error::invalid_conversion(
                "Line number must be greater than 0",
            ));
        }

        let id = self.get_next_id()?;
        let breakpoint = Breakpoint::line(id, file.clone(), line);

        // Store the breakpoint
        {
            let mut breakpoints = self
                .breakpoints
                .write()
                .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on breakpoints"))?;
            breakpoints.insert(id, breakpoint);
        }

        // Update line breakpoint index
        {
            let mut line_index = self.line_breakpoints_by_file.write().map_err(|_| {
                Error::lock_poisoned("Failed to acquire write lock on line breakpoints index")
            })?;
            line_index
                .entry(file)
                .or_insert_with(HashSet::new)
                .insert(id);
        }

        Ok(id)
    }

    /// Add a function breakpoint
    pub fn add_function_breakpoint(
        &self,
        name: String,
        file: Option<String>,
    ) -> Result<BreakpointId> {
        if name.is_empty() {
            return Err(Error::invalid_conversion("Function name cannot be empty"));
        }

        let id = self.get_next_id()?;
        let breakpoint = Breakpoint::function(id, name.clone(), file);

        // Store the breakpoint
        {
            let mut breakpoints = self
                .breakpoints
                .write()
                .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on breakpoints"))?;
            breakpoints.insert(id, breakpoint);
        }

        // Update function breakpoint index
        {
            let mut function_index = self.function_breakpoints_by_name.write().map_err(|_| {
                Error::lock_poisoned("Failed to acquire write lock on function breakpoints index")
            })?;
            function_index
                .entry(name)
                .or_insert_with(HashSet::new)
                .insert(id);
        }

        Ok(id)
    }

    /// Add an address breakpoint
    pub fn add_address_breakpoint(&self, address: usize) -> Result<BreakpointId> {
        let id = self.get_next_id()?;
        let breakpoint = Breakpoint::address(id, address);

        let mut breakpoints = self
            .breakpoints
            .write()
            .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on breakpoints"))?;
        breakpoints.insert(id, breakpoint);

        Ok(id)
    }

    /// Add an exception breakpoint
    pub fn add_exception_breakpoint(&self, exception_type: Option<String>) -> Result<BreakpointId> {
        let id = self.get_next_id()?;
        let breakpoint = Breakpoint::exception(id, exception_type);

        let mut breakpoints = self
            .breakpoints
            .write()
            .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on breakpoints"))?;
        breakpoints.insert(id, breakpoint);

        Ok(id)
    }

    /// Remove a breakpoint by ID
    pub fn remove_breakpoint(&self, id: BreakpointId) -> Result<()> {
        let breakpoint = {
            let mut breakpoints = self
                .breakpoints
                .write()
                .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on breakpoints"))?;
            breakpoints
                .remove(&id)
                .ok_or_else(|| Error::key_not_found(format!("Breakpoint {}", id)))?
        };

        // Remove from indexes
        match &breakpoint.breakpoint_type {
            BreakpointType::Line { file, .. } => {
                let mut line_index = self.line_breakpoints_by_file.write().map_err(|_| {
                    Error::lock_poisoned("Failed to acquire write lock on line breakpoints index")
                })?;
                if let Some(file_breakpoints) = line_index.get_mut(file) {
                    file_breakpoints.remove(&id);
                    if file_breakpoints.is_empty() {
                        line_index.remove(file);
                    }
                }
            }
            BreakpointType::Function { name, .. } => {
                let mut function_index =
                    self.function_breakpoints_by_name.write().map_err(|_| {
                        Error::lock_poisoned(
                            "Failed to acquire write lock on function breakpoints index",
                        )
                    })?;
                if let Some(function_breakpoints) = function_index.get_mut(name) {
                    function_breakpoints.remove(&id);
                    if function_breakpoints.is_empty() {
                        function_index.remove(name);
                    }
                }
            }
            _ => {} // Other types don't have special indexes
        }

        Ok(())
    }

    /// Get a breakpoint by ID
    pub fn get_breakpoint(&self, id: BreakpointId) -> Result<Breakpoint> {
        let breakpoints = self
            .breakpoints
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to acquire read lock on breakpoints"))?;
        breakpoints
            .get(&id)
            .cloned()
            .ok_or_else(|| Error::key_not_found(format!("Breakpoint {}", id)))
    }

    /// Get all breakpoints
    pub fn get_all_breakpoints(&self) -> Vec<Breakpoint> {
        let breakpoints = self
            .breakpoints
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to acquire read lock on breakpoints"));
        match breakpoints {
            Ok(bps) => bps.values().cloned().collect(),
            Err(_) => Vec::new(), // Return empty vec on lock failure
        }
    }

    /// Get breakpoints for a specific file
    pub fn get_breakpoints_for_file(&self, file: &str) -> Vec<Breakpoint> {
        let line_index = self.line_breakpoints_by_file.read().map_err(|_| {
            Error::lock_poisoned("Failed to acquire read lock on line breakpoints index")
        });
        let breakpoints = self
            .breakpoints
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to acquire read lock on breakpoints"));

        match (line_index, breakpoints) {
            (Ok(line_idx), Ok(bps)) => {
                if let Some(breakpoint_ids) = line_idx.get(file) {
                    breakpoint_ids
                        .iter()
                        .filter_map(|id| bps.get(id))
                        .cloned()
                        .collect()
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(), // Return empty vec on lock failure
        }
    }

    /// Get breakpoints for a specific function
    pub fn get_breakpoints_for_function(&self, function_name: &str) -> Vec<Breakpoint> {
        let function_index = self.function_breakpoints_by_name.read().map_err(|_| {
            Error::lock_poisoned("Failed to acquire read lock on function breakpoints index")
        });
        let breakpoints = self
            .breakpoints
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to acquire read lock on breakpoints"));

        match (function_index, breakpoints) {
            (Ok(func_idx), Ok(bps)) => {
                if let Some(breakpoint_ids) = func_idx.get(function_name) {
                    breakpoint_ids
                        .iter()
                        .filter_map(|id| bps.get(id))
                        .cloned()
                        .collect()
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(), // Return empty vec on lock failure
        }
    }

    /// Enable a breakpoint
    pub fn enable_breakpoint(&self, id: BreakpointId) -> Result<()> {
        let mut breakpoints = self
            .breakpoints
            .write()
            .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on breakpoints"))?;
        if let Some(breakpoint) = breakpoints.get_mut(&id) {
            breakpoint.enable();
            Ok(())
        } else {
            Err(Error::key_not_found(format!("Breakpoint {}", id)))
        }
    }

    /// Disable a breakpoint
    pub fn disable_breakpoint(&self, id: BreakpointId) -> Result<()> {
        let mut breakpoints = self
            .breakpoints
            .write()
            .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on breakpoints"))?;
        if let Some(breakpoint) = breakpoints.get_mut(&id) {
            breakpoint.disable();
            Ok(())
        } else {
            Err(Error::key_not_found(format!("Breakpoint {}", id)))
        }
    }

    /// Toggle a breakpoint's enabled state
    pub fn toggle_breakpoint(&self, id: BreakpointId) -> Result<bool> {
        let mut breakpoints = self
            .breakpoints
            .write()
            .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on breakpoints"))?;
        if let Some(breakpoint) = breakpoints.get_mut(&id) {
            breakpoint.toggle();
            Ok(breakpoint.enabled)
        } else {
            Err(Error::key_not_found(format!("Breakpoint {}", id)))
        }
    }

    /// Set a condition on a breakpoint
    pub fn set_breakpoint_condition(
        &self,
        id: BreakpointId,
        condition: BreakpointCondition,
    ) -> Result<()> {
        let mut breakpoints = self
            .breakpoints
            .write()
            .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on breakpoints"))?;
        if let Some(breakpoint) = breakpoints.get_mut(&id) {
            breakpoint.set_condition(condition);
            Ok(())
        } else {
            Err(Error::key_not_found(format!("Breakpoint {}", id)))
        }
    }

    /// Clear a condition from a breakpoint
    pub fn clear_breakpoint_condition(&self, id: BreakpointId) -> Result<()> {
        let mut breakpoints = self
            .breakpoints
            .write()
            .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on breakpoints"))?;
        if let Some(breakpoint) = breakpoints.get_mut(&id) {
            breakpoint.clear_condition();
            Ok(())
        } else {
            Err(Error::key_not_found(format!("Breakpoint {}", id)))
        }
    }

    /// Check if execution should break at the given location
    ///
    /// This is the main runtime integration point. It's called frequently
    /// during execution, so it's optimized for performance.
    pub fn should_break_at_location(
        &self,
        location: SourceLocation,
        function_name: Option<&str>,
    ) -> bool {
        // Quick check: if no breakpoints are set, return false immediately
        let breakpoints = self
            .breakpoints
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to acquire read lock on breakpoints"));

        let breakpoints = match breakpoints {
            Ok(bps) => bps,
            Err(_) => return false, // Return false on lock failure
        };

        if breakpoints.is_empty() {
            return false;
        }

        // Check all breakpoints to see if any match
        for breakpoint in breakpoints.values() {
            if breakpoint.matches_location(location, None, function_name) {
                // If there's a condition, evaluate it
                if let Some(condition) = &breakpoint.condition {
                    let context = BreakpointEvaluationContext {
                        variables: std::collections::HashMap::new(), // TODO: Fill with actual variables
                        location,
                        function_name: function_name.map(String::from),
                    };

                    match condition.evaluate(&context) {
                        Ok(true) => return true,
                        Ok(false) => continue,
                        Err(_) => continue, // Skip breakpoint if condition evaluation fails
                    }
                } else {
                    return true;
                }
            }
        }

        false
    }

    /// Check if execution should break at the given location in a specific file
    pub fn should_break_at_file_location(
        &self,
        file: &str,
        location: SourceLocation,
        function_name: Option<&str>,
    ) -> bool {
        // Check line breakpoints for this file first (most common case)
        let line_index = self.line_breakpoints_by_file.read().map_err(|_| {
            Error::lock_poisoned("Failed to acquire read lock on line breakpoints index")
        });

        let line_index = match line_index {
            Ok(idx) => idx,
            Err(_) => return false, // Return false on lock failure
        };

        if let Some(breakpoint_ids) = line_index.get(file) {
            let breakpoints = self
                .breakpoints
                .read()
                .map_err(|_| Error::lock_poisoned("Failed to acquire read lock on breakpoints"));

            let breakpoints = match breakpoints {
                Ok(bps) => bps,
                Err(_) => return false, // Return false on lock failure
            };

            for id in breakpoint_ids {
                if let Some(breakpoint) = breakpoints.get(id) {
                    if breakpoint.matches_location(location, Some(file), function_name) {
                        // Handle conditions
                        if let Some(condition) = &breakpoint.condition {
                            let context = BreakpointEvaluationContext {
                                variables: std::collections::HashMap::new(),
                                location,
                                function_name: function_name.map(String::from),
                            };

                            match condition.evaluate(&context) {
                                Ok(true) => return true,
                                Ok(false) => continue,
                                Err(_) => continue,
                            }
                        } else {
                            return true;
                        }
                    }
                }
            }
        }

        // Check function breakpoints if we have a function name
        if let Some(func_name) = function_name {
            let function_index = self.function_breakpoints_by_name.read().map_err(|_| {
                Error::lock_poisoned("Failed to acquire read lock on function breakpoints index")
            });

            let function_index = match function_index {
                Ok(idx) => idx,
                Err(_) => return false, // Return false on lock failure
            };

            if let Some(breakpoint_ids) = function_index.get(func_name) {
                let breakpoints = self.breakpoints.read().map_err(|_| {
                    Error::lock_poisoned("Failed to acquire read lock on breakpoints")
                });

                let breakpoints = match breakpoints {
                    Ok(bps) => bps,
                    Err(_) => return false, // Return false on lock failure
                };

                for id in breakpoint_ids {
                    if let Some(breakpoint) = breakpoints.get(id) {
                        if breakpoint.matches_location(location, Some(file), Some(func_name)) {
                            // Handle conditions
                            if let Some(condition) = &breakpoint.condition {
                                let context = BreakpointEvaluationContext {
                                    variables: std::collections::HashMap::new(),
                                    location,
                                    function_name: Some(func_name.to_string()),
                                };

                                match condition.evaluate(&context) {
                                    Ok(true) => return true,
                                    Ok(false) => continue,
                                    Err(_) => continue,
                                }
                            } else {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        false
    }

    /// Record a breakpoint hit
    pub fn record_hit(
        &self,
        id: BreakpointId,
        location: SourceLocation,
        function_name: Option<String>,
        thread_id: Option<usize>,
    ) -> Result<()> {
        // Update hit count
        {
            let mut breakpoints = self
                .breakpoints
                .write()
                .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on breakpoints"))?;
            if let Some(breakpoint) = breakpoints.get_mut(&id) {
                breakpoint.hit();
            } else {
                return Err(Error::key_not_found(format!("Breakpoint {}", id)));
            }
        }

        // Record hit in history
        {
            let breakpoint = self.get_breakpoint(id)?;
            let hit = BreakpointHit::new(breakpoint, location, function_name, thread_id);

            let mut history = self
                .hit_history
                .lock()
                .map_err(|_| Error::lock_poisoned("Failed to acquire lock on hit history"))?;
            history.push(hit);

            // Limit history size
            if history.len() > self.max_history_size {
                history.remove(0);
            }
        }

        Ok(())
    }

    /// Get breakpoint hit history
    pub fn get_hit_history(&self) -> Vec<BreakpointHit> {
        let history = self
            .hit_history
            .lock()
            .map_err(|_| Error::lock_poisoned("Failed to acquire lock on hit history"));
        match history {
            Ok(hist) => hist.clone(),
            Err(_) => Vec::new(), // Return empty vec on lock failure
        }
    }

    /// Clear hit history
    pub fn clear_hit_history(&self) -> Result<()> {
        let mut history = self
            .hit_history
            .lock()
            .map_err(|_| Error::lock_poisoned("Failed to acquire lock on hit history"))?;
        history.clear();
        Ok(())
    }

    /// Clear all breakpoints
    pub fn clear_all_breakpoints(&self) -> Result<()> {
        {
            let mut breakpoints = self
                .breakpoints
                .write()
                .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on breakpoints"))?;
            breakpoints.clear();
        }

        {
            let mut line_index = self.line_breakpoints_by_file.write().map_err(|_| {
                Error::lock_poisoned("Failed to acquire write lock on line breakpoints index")
            })?;
            line_index.clear();
        }

        {
            let mut function_index = self.function_breakpoints_by_name.write().map_err(|_| {
                Error::lock_poisoned("Failed to acquire write lock on function breakpoints index")
            })?;
            function_index.clear();
        }

        Ok(())
    }

    /// Get statistics about breakpoints
    pub fn get_statistics(&self) -> BreakpointStatistics {
        let breakpoints = self
            .breakpoints
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to acquire read lock on breakpoints"));
        let history = self
            .hit_history
            .lock()
            .map_err(|_| Error::lock_poisoned("Failed to acquire lock on hit history"));

        let (breakpoints, history) = match (breakpoints, history) {
            (Ok(bps), Ok(hist)) => (bps, hist),
            _ => {
                // Return empty stats on lock failure
                return BreakpointStatistics {
                    total_breakpoints: 0,
                    enabled_breakpoints: 0,
                    disabled_breakpoints: 0,
                    line_breakpoints: 0,
                    function_breakpoints: 0,
                    address_breakpoints: 0,
                    exception_breakpoints: 0,
                    conditional_breakpoints: 0,
                    total_hits: 0,
                };
            }
        };

        let mut stats = BreakpointStatistics {
            total_breakpoints: breakpoints.len(),
            enabled_breakpoints: 0,
            disabled_breakpoints: 0,
            line_breakpoints: 0,
            function_breakpoints: 0,
            address_breakpoints: 0,
            exception_breakpoints: 0,
            conditional_breakpoints: 0,
            total_hits: history.len(),
        };

        for breakpoint in breakpoints.values() {
            if breakpoint.enabled {
                stats.enabled_breakpoints += 1;
            } else {
                stats.disabled_breakpoints += 1;
            }

            if breakpoint.condition.is_some() {
                stats.conditional_breakpoints += 1;
            }

            match &breakpoint.breakpoint_type {
                BreakpointType::Line { .. } => stats.line_breakpoints += 1,
                BreakpointType::Function { .. } => stats.function_breakpoints += 1,
                BreakpointType::Address { .. } => stats.address_breakpoints += 1,
                BreakpointType::Exception { .. } => stats.exception_breakpoints += 1,
            }
        }

        stats
    }

    /// Get the next available breakpoint ID
    fn get_next_id(&self) -> Result<BreakpointId> {
        let mut next_id = self
            .next_id
            .lock()
            .map_err(|_| Error::lock_poisoned("Failed to acquire lock on next_id"))?;
        let id = *next_id;
        *next_id += 1;
        Ok(id)
    }
}

impl Default for BreakpointManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about breakpoints
#[derive(Debug, Clone)]
pub struct BreakpointStatistics {
    /// Total number of breakpoints
    pub total_breakpoints: usize,
    /// Number of enabled breakpoints
    pub enabled_breakpoints: usize,
    /// Number of disabled breakpoints
    pub disabled_breakpoints: usize,
    /// Number of line breakpoints
    pub line_breakpoints: usize,
    /// Number of function breakpoints
    pub function_breakpoints: usize,
    /// Number of address breakpoints
    pub address_breakpoints: usize,
    /// Number of exception breakpoints
    pub exception_breakpoints: usize,
    /// Number of conditional breakpoints
    pub conditional_breakpoints: usize,
    /// Total number of hits recorded
    pub total_hits: usize,
}

impl std::fmt::Display for BreakpointStatistics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Breakpoint Statistics:")?;
        writeln!(f, "  Total breakpoints: {}", self.total_breakpoints)?;
        writeln!(f, "  Enabled: {}", self.enabled_breakpoints)?;
        writeln!(f, "  Disabled: {}", self.disabled_breakpoints)?;
        writeln!(f, "  Line breakpoints: {}", self.line_breakpoints)?;
        writeln!(f, "  Function breakpoints: {}", self.function_breakpoints)?;
        writeln!(f, "  Address breakpoints: {}", self.address_breakpoints)?;
        writeln!(f, "  Exception breakpoints: {}", self.exception_breakpoints)?;
        writeln!(
            f, "  Conditional breakpoints: {}", self.conditional_breakpoints)?;
        writeln!(f, "  Total hits: {}", self.total_hits)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debugger::breakpoint::BreakpointCondition;

    #[test]
    fn test_breakpoint_manager_creation() {
        let manager = BreakpointManager::new();
        assert_eq!(manager.get_all_breakpoints().len(), 0);
    }

    #[test]
    fn test_line_breakpoint_management() {
        let manager = BreakpointManager::new();

        // Add a line breakpoint
        let id = manager
            .add_line_breakpoint("test.script".to_string(), 42)
            .unwrap();
        assert_eq!(id, 1);

        // Get the breakpoint
        let bp = manager.get_breakpoint(id).unwrap();
        assert_eq!(bp.id, id);
        match bp.breakpoint_type {
            BreakpointType::Line { file, line } => {
                assert_eq!(file, "test.script");
                assert_eq!(line, 42);
            }
            _ => panic!("Expected line breakpoint"),
        }

        // Get breakpoints for file
        let file_bps = manager.get_breakpoints_for_file("test.script");
        assert_eq!(file_bps.len(), 1);

        // Remove the breakpoint
        assert!(manager.remove_breakpoint(id).is_ok());
        assert_eq!(manager.get_all_breakpoints().len(), 0);
    }

    #[test]
    fn test_function_breakpoint_management() {
        let manager = BreakpointManager::new();

        // Add a function breakpoint
        let id = manager
            .add_function_breakpoint("main".to_string(), Some("test.script".to_string()))
            .unwrap();

        // Get breakpoints for function
        let func_bps = manager.get_breakpoints_for_function("main");
        assert_eq!(func_bps.len(), 1);
        assert_eq!(func_bps[0].id, id);

        // Remove the breakpoint
        assert!(manager.remove_breakpoint(id).is_ok());
        assert_eq!(manager.get_breakpoints_for_function("main").len(), 0);
    }

    #[test]
    fn test_breakpoint_enable_disable() {
        let manager = BreakpointManager::new();
        let id = manager
            .add_line_breakpoint("test.script".to_string(), 10)
            .unwrap();

        // Breakpoint should be enabled by default
        let bp = manager.get_breakpoint(id).unwrap();
        assert!(bp.enabled);

        // Disable the breakpoint
        assert!(manager.disable_breakpoint(id).is_ok());
        let bp = manager.get_breakpoint(id).unwrap();
        assert!(!bp.enabled);

        // Enable the breakpoint
        assert!(manager.enable_breakpoint(id).is_ok());
        let bp = manager.get_breakpoint(id).unwrap();
        assert!(bp.enabled);

        // Toggle the breakpoint
        let enabled = manager.toggle_breakpoint(id).unwrap();
        assert!(!enabled);
        let bp = manager.get_breakpoint(id).unwrap();
        assert!(!bp.enabled);
    }

    #[test]
    fn test_breakpoint_conditions() {
        let manager = BreakpointManager::new();
        let id = manager
            .add_line_breakpoint("test.script".to_string(), 10)
            .unwrap();

        // Add a condition
        let condition = BreakpointCondition::new("x > 5".to_string(), true);
        assert!(manager.set_breakpoint_condition(id, condition).is_ok());

        let bp = manager.get_breakpoint(id).unwrap();
        assert!(bp.condition.is_some());

        // Clear the condition
        assert!(manager.clear_breakpoint_condition(id).is_ok());
        let bp = manager.get_breakpoint(id).unwrap();
        assert!(bp.condition.is_none());
    }

    #[test]
    fn test_should_break_at_location() {
        let manager = BreakpointManager::new();
        let location = SourceLocation::new(42, 1, 0);

        // No breakpoints - should not break
        assert!(!manager.should_break_at_location(location, None));

        // Add a line breakpoint
        let _id = manager
            .add_line_breakpoint("test.script".to_string(), 42)
            .unwrap();

        // Should break at file location
        assert!(manager.should_break_at_file_location("test.script", location, None));

        // Should not break at different file
        assert!(!manager.should_break_at_file_location("other.script", location, None));

        // Should not break at different line
        let other_location = SourceLocation::new(43, 1, 0);
        assert!(!manager.should_break_at_file_location("test.script", other_location, None));
    }

    #[test]
    fn test_hit_recording() {
        let manager = BreakpointManager::new();
        let id = manager
            .add_line_breakpoint("test.script".to_string(), 10)
            .unwrap();
        let location = SourceLocation::new(10, 1, 0);

        // Record a hit
        assert!(manager
            .record_hit(id, location, Some("main".to_string()), Some(1))
            .is_ok());

        // Check hit count
        let bp = manager.get_breakpoint(id).unwrap();
        assert_eq!(bp.hit_count, 1);

        // Check hit history
        let history = manager.get_hit_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].breakpoint.id, id);
        assert_eq!(history[0].location, location);
        assert_eq!(history[0].function_name, Some("main".to_string()));
    }

    #[test]
    fn test_statistics() {
        let manager = BreakpointManager::new();

        // Add various types of breakpoints
        let _line_id = manager
            .add_line_breakpoint("test.script".to_string(), 10)
            .unwrap();
        let func_id = manager
            .add_function_breakpoint("main".to_string(), None)
            .unwrap();
        let _addr_id = manager.add_address_breakpoint(0x1000).unwrap();

        // Disable one breakpoint
        manager.disable_breakpoint(func_id).unwrap();

        // Add a condition to one breakpoint
        let condition = BreakpointCondition::new("x > 0".to_string(), true);
        manager
            .set_breakpoint_condition(_line_id, condition)
            .unwrap();

        let stats = manager.get_statistics();
        assert_eq!(stats.total_breakpoints, 3);
        assert_eq!(stats.enabled_breakpoints, 2);
        assert_eq!(stats.disabled_breakpoints, 1);
        assert_eq!(stats.line_breakpoints, 1);
        assert_eq!(stats.function_breakpoints, 1);
        assert_eq!(stats.address_breakpoints, 1);
        assert_eq!(stats.conditional_breakpoints, 1);
    }

    #[test]
    fn test_clear_all_breakpoints() {
        let manager = BreakpointManager::new();

        // Add some breakpoints
        let _id1 = manager
            .add_line_breakpoint("test.script".to_string(), 10)
            .unwrap();
        let _id2 = manager
            .add_function_breakpoint("main".to_string(), None)
            .unwrap();

        assert_eq!(manager.get_all_breakpoints().len(), 2);

        // Clear all
        assert!(manager.clear_all_breakpoints().is_ok());
        assert_eq!(manager.get_all_breakpoints().len(), 0);
    }

    #[test]
    fn test_invalid_breakpoint_creation() {
        let manager = BreakpointManager::new();

        // Empty file path should fail
        assert!(manager.add_line_breakpoint("".to_string(), 10).is_err());

        // Line 0 should fail
        assert!(manager
            .add_line_breakpoint("test.script".to_string(), 0)
            .is_err());

        // Empty function name should fail
        assert!(manager
            .add_function_breakpoint("".to_string(), None)
            .is_err());
    }
}
