//! Line table builder for DWARF debug information (simplified implementation)
//!
//! This is a placeholder implementation for line number debugging information.
//! A full implementation would integrate with gimli's line table generation.

use crate::source::{SourceLocation, Span};
use crate::error::Error;
use std::collections::HashMap;
use super::safe_conversions::{usize_to_u64, validate_file_count, validate_line_number, validate_column_number};

/// Builder for DWARF line number information (simplified)
pub struct LineTableBuilder {
    /// Map from file paths to file IDs
    file_map: HashMap<String, u64>,
    /// Current address being processed
    current_address: u64,
    /// Current file being processed
    current_file: Option<u64>,
    /// Current line being processed
    current_line: u64,
    /// Current column being processed
    current_column: u64,
    /// Line entries for debugging
    line_entries: Vec<LineEntry>,
}

#[derive(Debug, Clone)]
struct LineEntry {
    address: u64,
    file_id: u64,
    line: u64,
    column: u64,
}

impl LineTableBuilder {
    /// Create a new line table builder
    pub fn new() -> Self {
        Self {
            file_map: HashMap::new(),
            current_address: 0,
            current_file: None,
            current_line: 1,
            current_column: 1,
            line_entries: Vec::new(),
        }
    }

    /// Add a source file to the line table
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

    /// Begin a new sequence (typically for a new function)
    pub fn begin_sequence(&mut self, start_address: u64) {
        self.current_address = start_address;
    }

    /// End the current sequence
    pub fn end_sequence(&mut self, _end_address: u64) {
        // Placeholder - in full implementation would finalize the sequence
    }

    /// Set the current file
    pub fn set_file(&mut self, file_path: &str) -> Result<(), Error> {
        let file_id = self.add_file(file_path)?;
        self.current_file = Some(file_id);
        Ok(())
    }

    /// Add a line entry for the given address and source location
    pub fn add_line(&mut self, address: u64, location: &SourceLocation) -> Result<(), Error> {
        // Validate and convert line and column numbers
        let line_u64 = validate_line_number(location.line)? as u64;
        let column_u64 = validate_column_number(location.column)? as u64;
        
        // Update current state
        self.current_address = address;
        self.current_line = line_u64;
        self.current_column = column_u64;

        // Add line entry if we have a current file
        if let Some(file_id) = self.current_file {
            self.line_entries.push(LineEntry {
                address,
                file_id,
                line: line_u64,
                column: column_u64,
            });
        }
        Ok(())
    }

    /// Add line entries for multiple addresses with the same source location
    pub fn add_line_range(
        &mut self,
        start_address: u64,
        _end_address: u64,
        location: &SourceLocation,
    ) -> Result<(), Error> {
        self.add_line(start_address, location)
    }

    /// Add line entries for a span covering multiple addresses
    pub fn add_span(&mut self, start_address: u64, end_address: u64, span: &Span) -> Result<(), Error> {
        self.add_line_range(start_address, end_address, &span.start)
    }

    /// Mark the beginning of a basic block
    pub fn set_basic_block(&mut self) {
        // Placeholder - in full implementation would set basic block flag
    }

    /// Mark the beginning of a statement
    pub fn set_statement(&mut self) {
        // Placeholder - in full implementation would set statement flag
    }

    /// Mark the end of a prologue
    pub fn set_prologue_end(&mut self) {
        // Placeholder - in full implementation would set prologue end flag
    }

    /// Mark the beginning of an epilogue
    pub fn set_epilogue_begin(&mut self) {
        // Placeholder - in full implementation would set epilogue begin flag
    }

    /// Get the number of line entries
    pub fn line_count(&self) -> usize {
        self.line_entries.len()
    }

    /// Get the current address
    pub fn current_address(&self) -> u64 {
        self.current_address
    }

    /// Get the current file ID
    pub fn current_file(&self) -> Option<u64> {
        self.current_file
    }

    /// Get the file count
    pub fn file_count(&self) -> usize {
        self.file_map.len()
    }
}

impl Default for LineTableBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::debug::safe_conversions;

    #[test]
    fn test_line_table_builder_creation() {
        let builder = LineTableBuilder::new();
        assert_eq!(builder.file_count(), 0);
        assert_eq!(builder.current_address(), 0);
        assert_eq!(builder.current_line, 1);
        assert_eq!(builder.current_column, 1);
    }

    #[test]
    fn test_add_file() -> Result<(), Error> {
        let mut builder = LineTableBuilder::new();

        let file_id1 = builder.add_file("/test/main.script")?;
        let file_id2 = builder.add_file("/test/lib.script")?;
        let file_id3 = builder.add_file("/test/main.script")?; // Should return same ID

        assert_eq!(file_id1, 0);
        assert_eq!(file_id2, 1);
        assert_eq!(file_id3, 0); // Same file, same ID
        assert_eq!(builder.file_count(), 2);
        Ok(())
    }

    #[test]
    fn test_begin_end_sequence() {
        let mut builder = LineTableBuilder::new();

        builder.begin_sequence(0x1000);
        assert_eq!(builder.current_address(), 0x1000);

        builder.end_sequence(0x2000);
        // No change to current address for this simplified implementation
        assert_eq!(builder.current_address(), 0x1000);
    }

    #[test]
    fn test_set_file() -> Result<(), Error> {
        let mut builder = LineTableBuilder::new();

        builder.set_file("/test/main.script")?;
        assert_eq!(builder.current_file(), Some(0));
        assert_eq!(builder.file_count(), 1);

        builder.set_file("/test/lib.script")?;
        assert_eq!(builder.current_file(), Some(1));
        assert_eq!(builder.file_count(), 2);
        Ok(())
    }

    #[test]
    fn test_add_line() -> Result<(), Error> {
        let mut builder = LineTableBuilder::new();
        builder.begin_sequence(0x1000);
        builder.set_file("/test/main.script")?;

        let location = SourceLocation::new(10, 5, 100);
        builder.add_line(0x1000, &location)?;

        assert_eq!(builder.current_address(), 0x1000);
        assert_eq!(builder.current_line, 10);
        assert_eq!(builder.current_column, 5);
        assert_eq!(builder.line_count(), 1);
        Ok(())
    }

    #[test]
    fn test_add_span() -> Result<(), Error> {
        let mut builder = LineTableBuilder::new();
        builder.begin_sequence(0x1000);
        builder.set_file("/test/main.script")?;

        let span = Span::new(
            SourceLocation::new(10, 5, 100),
            SourceLocation::new(12, 10, 150),
        );

        builder.add_span(0x1000, 0x1100, &span)?;

        assert_eq!(builder.current_line, 10);
        assert_eq!(builder.current_column, 5);
        assert_eq!(builder.line_count(), 1);
        Ok(())
    }

    #[test]
    fn test_file_count_limit() {
        let mut builder = LineTableBuilder::new();

        // Add files up to near the limit
        for i in 0..safe_conversions::limits::MAX_SOURCE_FILES {
            let file_path = format!("/test/file_{}.script", i);
            builder.file_map.insert(file_path, i as u64);
        }

        // This should fail as we're at the limit
        let result = builder.add_file("/test/one_more.script");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Too many source files"));
    }

    #[test]
    fn test_line_number_overflow() -> Result<(), Error> {
        let mut builder = LineTableBuilder::new();
        builder.begin_sequence(0x1000);
        builder.set_file("/test/main.script")?;

        // Valid line number at limit
        let valid_location = SourceLocation::new(
            safe_conversions::limits::MAX_LINE_NUMBER as usize,
            5,
            100
        );
        assert!(builder.add_line(0x1000, &valid_location).is_ok());

        // Invalid line number beyond limit
        let invalid_location = SourceLocation::new(
            (safe_conversions::limits::MAX_LINE_NUMBER + 1) as usize,
            5,
            100
        );
        let result = builder.add_line(0x2000, &invalid_location);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Line number"));
        
        Ok(())
    }

    #[test]
    fn test_column_number_overflow() -> Result<(), Error> {
        let mut builder = LineTableBuilder::new();
        builder.begin_sequence(0x1000);
        builder.set_file("/test/main.script")?;

        // Valid column number at limit
        let valid_location = SourceLocation::new(
            10,
            safe_conversions::limits::MAX_COLUMN_NUMBER as usize,
            100
        );
        assert!(builder.add_line(0x1000, &valid_location).is_ok());

        // Invalid column number beyond limit
        let invalid_location = SourceLocation::new(
            10,
            (safe_conversions::limits::MAX_COLUMN_NUMBER + 1) as usize,
            100
        );
        let result = builder.add_line(0x2000, &invalid_location);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Column number"));
        
        Ok(())
    }
}
