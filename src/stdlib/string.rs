//! String manipulation for the Script programming language
//!
//! This module provides a UTF-8 string type and common string operations
//! that can be used from Script code. All strings in Script are UTF-8 encoded.

use crate::runtime::{RuntimeError, ScriptRc};
use crate::stdlib::{ScriptValue, ScriptVec};
use std::fmt;

/// A UTF-8 encoded string for Script
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScriptString {
    /// The underlying UTF-8 string
    data: String,
}

impl ScriptString {
    /// Create a new Script string
    pub fn new(s: String) -> Self {
        ScriptString { data: s }
    }

    /// Create a Script string from a &str
    pub fn from_str(s: &str) -> Self {
        ScriptString {
            data: s.to_string(),
        }
    }

    /// Get the string as a &str
    pub fn as_str(&self) -> &str {
        &self.data
    }

    /// Get the length in bytes
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Get the length in Unicode characters
    pub fn char_count(&self) -> usize {
        self.data.chars().count()
    }

    /// Check if the string is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Convert to uppercase
    pub fn to_uppercase(&self) -> ScriptString {
        ScriptString::new(self.data.to_uppercase())
    }

    /// Convert to lowercase
    pub fn to_lowercase(&self) -> ScriptString {
        ScriptString::new(self.data.to_lowercase())
    }

    /// Trim whitespace from both ends
    pub fn trim(&self) -> ScriptString {
        ScriptString::new(self.data.trim().to_string())
    }

    /// Trim whitespace from the start
    pub fn trim_start(&self) -> ScriptString {
        ScriptString::new(self.data.trim_start().to_string())
    }

    /// Trim whitespace from the end
    pub fn trim_end(&self) -> ScriptString {
        ScriptString::new(self.data.trim_end().to_string())
    }

    /// Check if the string starts with a prefix
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.data.starts_with(prefix)
    }

    /// Check if the string ends with a suffix
    pub fn ends_with(&self, suffix: &str) -> bool {
        self.data.ends_with(suffix)
    }

    /// Check if the string contains a substring
    pub fn contains(&self, substring: &str) -> bool {
        self.data.contains(substring)
    }

    /// Replace all occurrences of a pattern with a replacement
    pub fn replace(&self, from: &str, to: &str) -> ScriptString {
        ScriptString::new(self.data.replace(from, to))
    }

    /// Replace the first occurrence of a pattern with a replacement
    pub fn replace_first(&self, from: &str, to: &str) -> ScriptString {
        if let Some(pos) = self.data.find(from) {
            let mut result = String::with_capacity(self.data.len());
            result.push_str(&self.data[..pos]);
            result.push_str(to);
            result.push_str(&self.data[pos + from.len()..]);
            ScriptString::new(result)
        } else {
            self.clone()
        }
    }

    /// Split the string by a delimiter
    pub fn split(&self, delimiter: &str) -> Vec<ScriptString> {
        self.data
            .split(delimiter)
            .map(|s| ScriptString::from_str(s))
            .collect()
    }

    /// Split the string by whitespace
    pub fn split_whitespace(&self) -> Vec<ScriptString> {
        self.data
            .split_whitespace()
            .map(|s| ScriptString::from_str(s))
            .collect()
    }

    /// Get a substring by byte range
    /// Returns None if the range is invalid or not on char boundaries
    pub fn substring(&self, start: usize, end: usize) -> Option<ScriptString> {
        if start > end || end > self.data.len() {
            return None;
        }

        // Check if we're on valid UTF-8 boundaries
        if !self.data.is_char_boundary(start) || !self.data.is_char_boundary(end) {
            return None;
        }

        Some(ScriptString::new(self.data[start..end].to_string()))
    }

    /// Get a character at a specific index
    /// Returns None if the index is out of bounds
    pub fn char_at(&self, index: usize) -> Option<char> {
        self.data.chars().nth(index)
    }

    /// Concatenate with another string
    pub fn concat(&self, other: &ScriptString) -> ScriptString {
        ScriptString::new(format!("{}{}", self.data, other.data))
    }

    /// Repeat the string n times
    pub fn repeat(&self, n: usize) -> ScriptString {
        ScriptString::new(self.data.repeat(n))
    }

    /// Find the first occurrence of a substring
    /// Returns the byte index if found
    pub fn find(&self, pattern: &str) -> Option<usize> {
        self.data.find(pattern)
    }

    /// Find the last occurrence of a substring
    /// Returns the byte index if found
    pub fn rfind(&self, pattern: &str) -> Option<usize> {
        self.data.rfind(pattern)
    }

    /// Parse the string as an integer
    pub fn parse_i32(&self) -> Result<i32, String> {
        self.data
            .trim()
            .parse::<i32>()
            .map_err(|e| format!("Failed to parse '{}' as i32: {}", self.data, e))
    }

    /// Parse the string as a float
    pub fn parse_f32(&self) -> Result<f32, String> {
        self.data
            .trim()
            .parse::<f32>()
            .map_err(|e| format!("Failed to parse '{}' as f32: {}", self.data, e))
    }
}

impl fmt::Display for ScriptString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.data)
    }
}

impl From<String> for ScriptString {
    fn from(s: String) -> Self {
        ScriptString::new(s)
    }
}

impl From<&str> for ScriptString {
    fn from(s: &str) -> Self {
        ScriptString::from_str(s)
    }
}

/// String operations trait for implementing stdlib functions
pub trait StringOps {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn to_uppercase(&self) -> ScriptString;
    fn to_lowercase(&self) -> ScriptString;
    fn trim(&self) -> ScriptString;
    fn split(&self, delimiter: &str) -> Vec<ScriptString>;
    fn contains(&self, substring: &str) -> bool;
    fn replace(&self, from: &str, to: &str) -> ScriptString;
}

impl StringOps for ScriptString {
    fn len(&self) -> usize {
        self.len()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn to_uppercase(&self) -> ScriptString {
        self.to_uppercase()
    }

    fn to_lowercase(&self) -> ScriptString {
        self.to_lowercase()
    }

    fn trim(&self) -> ScriptString {
        self.trim()
    }

    fn split(&self, delimiter: &str) -> Vec<ScriptString> {
        self.split(delimiter)
    }

    fn contains(&self, substring: &str) -> bool {
        self.contains(substring)
    }

    fn replace(&self, from: &str, to: &str) -> ScriptString {
        self.replace(from, to)
    }
}

// Implementation functions for stdlib registry

/// Get the length of a string in characters
pub(crate) fn string_len_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "string_len expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(s) => Ok(ScriptValue::I32(s.char_count() as i32)),
        _ => Err(RuntimeError::InvalidOperation(
            "string_len expects a string argument".to_string(),
        )),
    }
}

/// Convert a string to uppercase
pub(crate) fn string_to_uppercase_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "to_uppercase expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(s) => {
            let upper = s.to_uppercase();
            Ok(ScriptValue::String(ScriptRc::new(upper)))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "to_uppercase expects a string argument".to_string(),
        )),
    }
}

/// Convert a string to lowercase
pub(crate) fn string_to_lowercase_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "to_lowercase expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(s) => {
            let lower = s.to_lowercase();
            Ok(ScriptValue::String(ScriptRc::new(lower)))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "to_lowercase expects a string argument".to_string(),
        )),
    }
}

/// Trim whitespace from a string
pub(crate) fn string_trim_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "trim expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(s) => {
            let trimmed = s.trim();
            Ok(ScriptValue::String(ScriptRc::new(trimmed)))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "trim expects a string argument".to_string(),
        )),
    }
}

/// Split a string by a delimiter
pub(crate) fn string_split_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "split expects 2 arguments, got {}",
            args.len()
        )));
    }

    match (&args[0], &args[1]) {
        (ScriptValue::String(s), ScriptValue::String(delimiter)) => {
            let parts = s.split(&delimiter.as_str());
            let vec = ScriptVec::new();
            for part in parts {
                vec.push(ScriptValue::String(ScriptRc::new(part)));
            }
            Ok(ScriptValue::Array(ScriptRc::new(vec)))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "split expects two string arguments".to_string(),
        )),
    }
}

/// Check if a string contains a substring
pub(crate) fn string_contains_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "contains expects 2 arguments, got {}",
            args.len()
        )));
    }

    match (&args[0], &args[1]) {
        (ScriptValue::String(s), ScriptValue::String(substring)) => {
            Ok(ScriptValue::Bool(s.contains(&substring.as_str())))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "contains expects two string arguments".to_string(),
        )),
    }
}

/// Replace all occurrences in a string
pub(crate) fn string_replace_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(format!(
            "replace expects 3 arguments, got {}",
            args.len()
        )));
    }

    match (&args[0], &args[1], &args[2]) {
        (ScriptValue::String(s), ScriptValue::String(from), ScriptValue::String(to)) => {
            let replaced = s.replace(&from.as_str(), &to.as_str());
            Ok(ScriptValue::String(ScriptRc::new(replaced)))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "replace expects three string arguments".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_creation() {
        let s1 = ScriptString::new("Hello".to_string());
        let s2 = ScriptString::from_str("Hello");
        assert_eq!(s1, s2);
        assert_eq!(s1.as_str(), "Hello");
    }

    #[test]
    fn test_string_length() {
        let s = ScriptString::from_str("Hello, 世界!");
        assert_eq!(s.len(), 14); // byte length
        assert_eq!(s.char_count(), 10); // character count

        let empty = ScriptString::from_str("");
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        assert_eq!(empty.char_count(), 0);
    }

    #[test]
    fn test_string_case_conversion() {
        let s = ScriptString::from_str("Hello World!");
        assert_eq!(s.to_uppercase().as_str(), "HELLO WORLD!");
        assert_eq!(s.to_lowercase().as_str(), "hello world!");
    }

    #[test]
    fn test_string_trimming() {
        let s = ScriptString::from_str("  Hello World!  ");
        assert_eq!(s.trim().as_str(), "Hello World!");
        assert_eq!(s.trim_start().as_str(), "Hello World!  ");
        assert_eq!(s.trim_end().as_str(), "  Hello World!");
    }

    #[test]
    fn test_string_predicates() {
        let s = ScriptString::from_str("Hello World!");
        assert!(s.starts_with("Hello"));
        assert!(!s.starts_with("World"));
        assert!(s.ends_with("World!"));
        assert!(!s.ends_with("Hello"));
        assert!(s.contains("lo Wo"));
        assert!(!s.contains("xyz"));
    }

    #[test]
    fn test_string_replace() {
        let s = ScriptString::from_str("Hello World! Hello!");
        assert_eq!(s.replace("Hello", "Hi").as_str(), "Hi World! Hi!");
        assert_eq!(s.replace_first("Hello", "Hi").as_str(), "Hi World! Hello!");
        assert_eq!(s.replace("xyz", "abc").as_str(), "Hello World! Hello!");
    }

    #[test]
    fn test_string_split() {
        let s = ScriptString::from_str("one,two,three");
        let parts = s.split(",");
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].as_str(), "one");
        assert_eq!(parts[1].as_str(), "two");
        assert_eq!(parts[2].as_str(), "three");

        let s2 = ScriptString::from_str("one  two\tthree");
        let parts2 = s2.split_whitespace();
        assert_eq!(parts2.len(), 3);
        assert_eq!(parts2[0].as_str(), "one");
        assert_eq!(parts2[1].as_str(), "two");
        assert_eq!(parts2[2].as_str(), "three");
    }

    #[test]
    fn test_string_substring() {
        let s = ScriptString::from_str("Hello World!");
        assert_eq!(s.substring(0, 5).unwrap().as_str(), "Hello");
        assert_eq!(s.substring(6, 11).unwrap().as_str(), "World");
        assert!(s.substring(0, 100).is_none()); // out of bounds
        assert!(s.substring(5, 2).is_none()); // invalid range
    }

    #[test]
    fn test_string_char_at() {
        let s = ScriptString::from_str("Hello");
        assert_eq!(s.char_at(0), Some('H'));
        assert_eq!(s.char_at(4), Some('o'));
        assert_eq!(s.char_at(5), None);
    }

    #[test]
    fn test_string_concat_repeat() {
        let s1 = ScriptString::from_str("Hello");
        let s2 = ScriptString::from_str(" World");
        assert_eq!(s1.concat(&s2).as_str(), "Hello World");

        let s3 = ScriptString::from_str("Ha");
        assert_eq!(s3.repeat(3).as_str(), "HaHaHa");
        assert_eq!(s3.repeat(0).as_str(), "");
    }

    #[test]
    fn test_string_find() {
        let s = ScriptString::from_str("Hello World Hello");
        assert_eq!(s.find("Hello"), Some(0));
        assert_eq!(s.find("World"), Some(6));
        assert_eq!(s.find("xyz"), None);
        assert_eq!(s.rfind("Hello"), Some(12));
        assert_eq!(s.rfind("World"), Some(6));
    }

    #[test]
    fn test_string_parsing() {
        let s1 = ScriptString::from_str("42");
        assert_eq!(s1.parse_i32().unwrap(), 42);

        let s2 = ScriptString::from_str("  -123  ");
        assert_eq!(s2.parse_i32().unwrap(), -123);

        let s3 = ScriptString::from_str("3.14");
        assert_eq!(s3.parse_f32().unwrap(), 3.14);

        let s4 = ScriptString::from_str("not a number");
        assert!(s4.parse_i32().is_err());
        assert!(s4.parse_f32().is_err());
    }
}
