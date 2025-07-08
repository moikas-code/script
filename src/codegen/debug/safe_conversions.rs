//! Safe integer conversion utilities for debug information generation
//!
//! This module provides safe conversion functions to prevent integer overflow
//! vulnerabilities when converting between different integer types in debug
//! information generation.

use crate::error::{Error, ErrorKind};
use std::fmt;

/// Error type for integer conversion failures
#[derive(Debug, Clone, PartialEq)]
pub enum ConversionError {
    /// Value too large for target type
    Overflow { value: String, target_type: &'static str },
    /// Value would cause overflow when adding
    AdditionOverflow { base: String, addend: String, target_type: &'static str },
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConversionError::Overflow { value, target_type } => {
                write!(f, "Integer overflow: value {} cannot fit in {}", value, target_type)
            }
            ConversionError::AdditionOverflow { base, addend, target_type } => {
                write!(f, "Integer overflow: {} + {} exceeds {} maximum", base, addend, target_type)
            }
        }
    }
}

impl std::error::Error for ConversionError {}

/// Safely convert usize to u64
pub fn usize_to_u64(value: usize) -> Result<u64, Error> {
    u64::try_from(value).map_err(|_| {
        Error::new(
            ErrorKind::Other,
            format!("Integer overflow: usize value {} cannot fit in u64", value),
        )
    })
}

/// Safely convert usize to u32
pub fn usize_to_u32(value: usize) -> Result<u32, Error> {
    u32::try_from(value).map_err(|_| {
        Error::new(
            ErrorKind::Other,
            format!("Integer overflow: usize value {} cannot fit in u32", value),
        )
    })
}

/// Safely convert usize to u32 and add a value
pub fn usize_to_u32_add(base: usize, addend: u32) -> Result<u32, Error> {
    let base_u32 = usize_to_u32(base)?;
    base_u32.checked_add(addend).ok_or_else(|| {
        Error::new(
            ErrorKind::Other,
            format!("Integer overflow: {} + {} exceeds u32 maximum", base_u32, addend),
        )
    })
}

/// Safely convert i32 to u32 (for non-negative values only)
pub fn i32_to_u32(value: i32) -> Result<u32, Error> {
    if value < 0 {
        return Err(Error::new(
            ErrorKind::Other,
            format!("Cannot convert negative i32 value {} to u32", value),
        ));
    }
    Ok(value as u32)
}

/// Resource limits for debug information
pub mod limits {
    /// Maximum number of source files in a compilation unit
    pub const MAX_SOURCE_FILES: usize = 100_000;
    
    /// Maximum line number supported
    pub const MAX_LINE_NUMBER: u32 = 10_000_000;
    
    /// Maximum column number supported
    pub const MAX_COLUMN_NUMBER: u32 = 100_000;
    
    /// Maximum number of debug entries
    pub const MAX_DEBUG_ENTRIES: usize = 1_000_000;
}

/// Validate that a file count is within acceptable limits
pub fn validate_file_count(count: usize) -> Result<(), Error> {
    if count > limits::MAX_SOURCE_FILES {
        return Err(Error::new(
            ErrorKind::Other,
            format!("Too many source files: {} exceeds limit of {}", count, limits::MAX_SOURCE_FILES),
        ));
    }
    Ok(())
}

/// Validate that a line number is within acceptable limits
pub fn validate_line_number(line: usize) -> Result<u32, Error> {
    let line_u32 = usize_to_u32(line)?;
    if line_u32 > limits::MAX_LINE_NUMBER {
        return Err(Error::new(
            ErrorKind::Other,
            format!("Line number {} exceeds maximum of {}", line_u32, limits::MAX_LINE_NUMBER),
        ));
    }
    Ok(line_u32)
}

/// Validate that a column number is within acceptable limits
pub fn validate_column_number(column: usize) -> Result<u32, Error> {
    let column_u32 = usize_to_u32(column)?;
    if column_u32 > limits::MAX_COLUMN_NUMBER {
        return Err(Error::new(
            ErrorKind::Other,
            format!("Column number {} exceeds maximum of {}", column_u32, limits::MAX_COLUMN_NUMBER),
        ));
    }
    Ok(column_u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usize_to_u64_success() {
        assert_eq!(usize_to_u64(0).unwrap(), 0u64);
        assert_eq!(usize_to_u64(42).unwrap(), 42u64);
        assert_eq!(usize_to_u64(usize::MAX).unwrap(), usize::MAX as u64);
    }

    #[test]
    fn test_usize_to_u32_success() {
        assert_eq!(usize_to_u32(0).unwrap(), 0u32);
        assert_eq!(usize_to_u32(42).unwrap(), 42u32);
        assert_eq!(usize_to_u32(u32::MAX as usize).unwrap(), u32::MAX);
    }

    #[test]
    fn test_usize_to_u32_overflow() {
        // On 64-bit systems, this should fail
        #[cfg(target_pointer_width = "64")]
        {
            let large_value = (u32::MAX as usize) + 1;
            assert!(usize_to_u32(large_value).is_err());
        }
    }

    #[test]
    fn test_usize_to_u32_add_success() {
        assert_eq!(usize_to_u32_add(10, 5).unwrap(), 15u32);
        assert_eq!(usize_to_u32_add(0, 0).unwrap(), 0u32);
        assert_eq!(usize_to_u32_add(100, 1).unwrap(), 101u32);
    }

    #[test]
    fn test_usize_to_u32_add_overflow() {
        assert!(usize_to_u32_add(u32::MAX as usize, 1).is_err());
        assert!(usize_to_u32_add(u32::MAX as usize - 1, 2).is_err());
    }

    #[test]
    fn test_i32_to_u32_success() {
        assert_eq!(i32_to_u32(0).unwrap(), 0u32);
        assert_eq!(i32_to_u32(42).unwrap(), 42u32);
        assert_eq!(i32_to_u32(i32::MAX).unwrap(), i32::MAX as u32);
    }

    #[test]
    fn test_i32_to_u32_negative() {
        assert!(i32_to_u32(-1).is_err());
        assert!(i32_to_u32(-100).is_err());
        assert!(i32_to_u32(i32::MIN).is_err());
    }

    #[test]
    fn test_validate_file_count() {
        assert!(validate_file_count(0).is_ok());
        assert!(validate_file_count(1000).is_ok());
        assert!(validate_file_count(limits::MAX_SOURCE_FILES).is_ok());
        assert!(validate_file_count(limits::MAX_SOURCE_FILES + 1).is_err());
    }

    #[test]
    fn test_validate_line_number() {
        assert_eq!(validate_line_number(0).unwrap(), 0);
        assert_eq!(validate_line_number(100).unwrap(), 100);
        assert_eq!(validate_line_number(limits::MAX_LINE_NUMBER as usize).unwrap(), limits::MAX_LINE_NUMBER);
        assert!(validate_line_number((limits::MAX_LINE_NUMBER + 1) as usize).is_err());
    }

    #[test]
    fn test_validate_column_number() {
        assert_eq!(validate_column_number(0).unwrap(), 0);
        assert_eq!(validate_column_number(80).unwrap(), 80);
        assert_eq!(validate_column_number(limits::MAX_COLUMN_NUMBER as usize).unwrap(), limits::MAX_COLUMN_NUMBER);
        assert!(validate_column_number((limits::MAX_COLUMN_NUMBER + 1) as usize).is_err());
    }

    #[test]
    fn test_edge_cases() {
        // Test maximum values
        #[cfg(target_pointer_width = "32")]
        {
            assert_eq!(usize_to_u32(usize::MAX).unwrap(), u32::MAX);
            assert_eq!(usize_to_u64(usize::MAX).unwrap(), u32::MAX as u64);
        }

        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(usize_to_u64(usize::MAX).unwrap(), u64::MAX);
        }
    }
}