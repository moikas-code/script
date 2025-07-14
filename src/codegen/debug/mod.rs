//! Debug information generation module
//!
//! This module handles the generation of DWARF debug information
//! for Script language programs compiled with Cranelift.

use self::safe_conversions::{usize_to_u64, validate_file_count};
use crate::error::Error;
use crate::ir::Function as IrFunction;
use crate::source::SourceLocation;
use std::collections::HashMap;

pub mod dwarf_builder;
pub mod line_table;
pub mod safe_conversions;
pub mod type_info;

pub use dwarf_builder::DwarfBuilder;
pub use line_table::LineTableBuilder;
pub use type_info::TypeInfoBuilder;

/// Debug information context for a compilation unit
#[derive(Debug)]
pub struct DebugContext {
    /// Current compilation directory
    comp_dir: String,
    /// Current producer string
    producer: String,
    /// Mapping from source file paths to file IDs
    file_map: HashMap<String, u64>,
    /// Current source file being processed
    current_file: Option<String>,
}

impl DebugContext {
    /// Create a new debug context
    pub fn new(comp_dir: String, producer: String) -> Self {
        DebugContext {
            comp_dir,
            producer,
            file_map: HashMap::new(),
            current_file: None,
        }
    }

    /// Add a source file to the debug context
    pub fn add_file(&mut self, file_path: &str) -> Result<u64, Error> {
        if let Some(&file_id) = self.file_map.get(file_path) {
            return Ok(file_id);
        }

        // Validate file count is within limits
        validate_file_count(self.file_map.len())?;

        // Safely convert to u64
        let file_id = usize_to_u64(self.file_map.len())?;
        self.file_map.insert(file_path.to_string(), file_id);

        Ok(file_id)
    }

    /// Set current source file
    pub fn set_current_file(&mut self, file_path: String) -> Result<(), Error> {
        let _file_id = self.add_file(&file_path)?;
        self.current_file = Some(file_path);
        Ok(())
    }

    /// Add line information for an instruction
    pub fn add_line_info(&mut self, _address: u64, _location: &SourceLocation) {
        // Placeholder implementation for line info
        // In a full implementation, this would build line number tables
    }

    /// Add function debug information
    pub fn add_function(
        &mut self,
        _function: &IrFunction,
        _start_address: u64,
        _end_address: u64,
    ) -> Result<(), Error> {
        // Placeholder implementation for function debug info
        // In a full implementation, this would create DWARF function entries
        Ok(())
    }

    /// Generate DWARF sections (simplified)
    pub fn generate_sections(&mut self) -> Result<Vec<u8>, Error> {
        // Placeholder implementation that returns empty debug info
        // In a full implementation, this would generate proper DWARF sections
        Ok(Vec::new())
    }
}

/// Debug compilation flags
#[derive(Debug, Clone)]
pub struct DebugFlags {
    /// Generate debug information
    pub debug_info: bool,
    /// Generate line number information
    pub line_info: bool,
    /// Generate variable location information
    pub variable_info: bool,
    /// Optimization level affects debug info quality
    pub optimize: bool,
}

impl Default for DebugFlags {
    fn default() -> Self {
        Self {
            debug_info: true,
            line_info: true,
            variable_info: true,
            optimize: false,
        }
    }
}

impl DebugFlags {
    /// Create debug flags for release builds
    pub fn release() -> Self {
        Self {
            debug_info: false,
            line_info: false,
            variable_info: false,
            optimize: true,
        }
    }

    /// Create debug flags for debug builds
    pub fn debug() -> Self {
        Self {
            debug_info: true,
            line_info: true,
            variable_info: true,
            optimize: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::debug::safe_conversions;

    #[test]
    fn test_debug_context_creation() {
        let debug_ctx = DebugContext::new(
            "/test/project".to_string(),
            "Script Language Compiler 0.1.0".to_string(),
        );

        assert_eq!(debug_ctx.comp_dir, "/test/project");
        assert!(debug_ctx.file_map.is_empty());
    }

    #[test]
    fn test_add_file() -> Result<(), Error> {
        let mut debug_ctx = DebugContext::new(
            "/test/project".to_string(),
            "Script Language Compiler 0.1.0".to_string(),
        );

        let file_id1 = debug_ctx.add_file("/test/project/main.script")?;
        let file_id2 = debug_ctx.add_file("/test/project/lib.script")?;
        let file_id3 = debug_ctx.add_file("/test/project/main.script")?; // Should return same ID

        assert_eq!(file_id1, 0);
        assert_eq!(file_id2, 1);
        assert_eq!(file_id3, 0); // Same file, same ID
        assert_eq!(debug_ctx.file_map.len(), 2);
        Ok(())
    }

    #[test]
    fn test_debug_flags() {
        let debug_flags = DebugFlags::debug();
        assert!(debug_flags.debug_info);
        assert!(debug_flags.line_info);
        assert!(debug_flags.variable_info);
        assert!(!debug_flags.optimize);

        let release_flags = DebugFlags::release();
        assert!(!release_flags.debug_info);
        assert!(!release_flags.line_info);
        assert!(!release_flags.variable_info);
        assert!(release_flags.optimize);
    }

    #[test]
    fn test_file_count_limit() {
        let mut debug_ctx = DebugContext::new(
            "/test/project".to_string(),
            "Script Language Compiler 0.1.0".to_string(),
        );

        // Add files up to near the limit
        for i in 0..safe_conversions::limits::MAX_SOURCE_FILES {
            let file_path = format!("/test/file_{}.script", i);
            debug_ctx.file_map.insert(file_path, i as u64);
        }

        // This should fail as we're at the limit
        let result = debug_ctx.add_file("/test/one_more.script");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Too many source files"));
    }
}
