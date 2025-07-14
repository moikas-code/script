//! String manipulation for the Script programming language
//!
//! This module provides a UTF-8 string type and common string operations
//! that can be used from Script code. All strings in Script are UTF-8 encoded.

use crate::runtime::{RuntimeError, ScriptRc};
use crate::stdlib::collections::ScriptVec;
use crate::stdlib::ScriptValue;
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

    /// Join a vector of strings with this string as the delimiter
    pub fn join(&self, parts: &[ScriptString]) -> ScriptString {
        let strings: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();
        ScriptString::new(strings.join(&self.data))
    }

    /// Pad the string to a certain length with a fill string on the left
    pub fn pad_left(&self, width: usize, fill: &str) -> ScriptString {
        let current_len = self.char_count();
        if current_len >= width {
            self.clone()
        } else {
            let padding_needed = width - current_len;
            let fill_chars: Vec<char> = fill.chars().collect();
            if fill_chars.is_empty() {
                return self.clone();
            }

            let mut result = String::new();
            for i in 0..padding_needed {
                result.push(fill_chars[i % fill_chars.len()]);
            }
            result.push_str(&self.data);
            ScriptString::new(result)
        }
    }

    /// Pad the string to a certain length with a fill string on the right
    pub fn pad_right(&self, width: usize, fill: &str) -> ScriptString {
        let current_len = self.char_count();
        if current_len >= width {
            self.clone()
        } else {
            let padding_needed = width - current_len;
            let fill_chars: Vec<char> = fill.chars().collect();
            if fill_chars.is_empty() {
                return self.clone();
            }

            let mut result = self.data.clone();
            for i in 0..padding_needed {
                result.push(fill_chars[i % fill_chars.len()]);
            }
            ScriptString::new(result)
        }
    }

    /// Center the string within a certain width with padding
    pub fn center(&self, width: usize, fill: &str) -> ScriptString {
        let current_len = self.char_count();
        if current_len >= width {
            self.clone()
        } else {
            let total_padding = width - current_len;
            let left_padding = total_padding / 2;
            let right_padding = total_padding - left_padding;

            let fill_chars: Vec<char> = fill.chars().collect();
            if fill_chars.is_empty() {
                return self.clone();
            }

            let mut result = String::new();
            for i in 0..left_padding {
                result.push(fill_chars[i % fill_chars.len()]);
            }
            result.push_str(&self.data);
            for i in 0..right_padding {
                result.push(fill_chars[i % fill_chars.len()]);
            }
            ScriptString::new(result)
        }
    }

    /// Remove a prefix if it exists
    pub fn strip_prefix(&self, prefix: &str) -> ScriptString {
        if let Some(stripped) = self.data.strip_prefix(prefix) {
            ScriptString::new(stripped.to_string())
        } else {
            self.clone()
        }
    }

    /// Remove a suffix if it exists
    pub fn strip_suffix(&self, suffix: &str) -> ScriptString {
        if let Some(stripped) = self.data.strip_suffix(suffix) {
            ScriptString::new(stripped.to_string())
        } else {
            self.clone()
        }
    }

    /// Capitalize the first character and lowercase the rest
    pub fn capitalize(&self) -> ScriptString {
        let mut chars = self.data.chars();
        match chars.next() {
            None => ScriptString::new(String::new()),
            Some(first) => {
                let capitalized =
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase();
                ScriptString::new(capitalized)
            }
        }
    }

    /// Convert to title case (capitalize first letter of each word)
    pub fn title_case(&self) -> ScriptString {
        let mut result = String::new();
        let mut after_space = true;

        for ch in self.data.chars() {
            if ch.is_whitespace() {
                result.push(ch);
                after_space = true;
            } else if after_space {
                result.extend(ch.to_uppercase());
                after_space = false;
            } else {
                result.extend(ch.to_lowercase());
            }
        }

        ScriptString::new(result)
    }

    /// Count occurrences of a substring
    pub fn count_matches(&self, pattern: &str) -> usize {
        if pattern.is_empty() {
            return 0;
        }
        self.data.matches(pattern).count()
    }

    /// Split the string into lines
    pub fn lines(&self) -> Vec<ScriptString> {
        self.data
            .lines()
            .map(|line| ScriptString::from_str(line))
            .collect()
    }

    /// Check if the string is ASCII only
    pub fn is_ascii(&self) -> bool {
        self.data.is_ascii()
    }

    /// Check if all characters are alphabetic
    pub fn is_alphabetic(&self) -> bool {
        !self.data.is_empty() && self.data.chars().all(|c| c.is_alphabetic())
    }

    /// Check if all characters are numeric
    pub fn is_numeric(&self) -> bool {
        !self.data.is_empty() && self.data.chars().all(|c| c.is_numeric())
    }

    /// Check if all characters are alphanumeric
    pub fn is_alphanumeric(&self) -> bool {
        !self.data.is_empty() && self.data.chars().all(|c| c.is_alphanumeric())
    }

    /// Check if all characters are whitespace
    pub fn is_whitespace(&self) -> bool {
        !self.data.is_empty() && self.data.chars().all(|c| c.is_whitespace())
    }

    /// Reverse the string
    pub fn reverse(&self) -> ScriptString {
        ScriptString::new(self.data.chars().rev().collect())
    }

    /// Get the nth word (whitespace separated)
    pub fn word(&self, n: usize) -> Option<ScriptString> {
        self.data
            .split_whitespace()
            .nth(n)
            .map(ScriptString::from_str)
    }

    /// Format the string with arguments (simple placeholder replacement)
    /// Replaces {} with the provided arguments in order
    pub fn format(&self, args: &[ScriptString]) -> ScriptString {
        let mut result = self.data.clone();
        for arg in args {
            if let Some(pos) = result.find("{}") {
                result.replace_range(pos..pos + 2, arg.as_str());
            } else {
                break;
            }
        }
        ScriptString::new(result)
    }

    /// Truncate the string to a maximum length, adding ellipsis if truncated
    pub fn truncate(&self, max_len: usize, ellipsis: &str) -> ScriptString {
        let char_count = self.char_count();
        if char_count <= max_len {
            self.clone()
        } else {
            let ellipsis_len = ellipsis.chars().count();
            if max_len <= ellipsis_len {
                ScriptString::new(ellipsis.chars().take(max_len).collect())
            } else {
                let keep_chars = max_len - ellipsis_len;
                let truncated: String = self.data.chars().take(keep_chars).collect();
                ScriptString::new(truncated + ellipsis)
            }
        }
    }

    /// Escape special characters for use in HTML
    pub fn escape_html(&self) -> ScriptString {
        let escaped = self
            .data
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;");
        ScriptString::new(escaped)
    }

    /// Remove consecutive duplicate characters
    pub fn squeeze(&self, ch: char) -> ScriptString {
        let mut result = String::new();
        let mut last_char: Option<char> = None;

        for current in self.data.chars() {
            if current != ch || last_char != Some(ch) {
                result.push(current);
            }
            last_char = Some(current);
        }

        ScriptString::new(result)
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

/// Join strings with a delimiter
pub(crate) fn string_join_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "join expects 2 arguments, got {}",
            args.len()
        )));
    }

    match (&args[0], &args[1]) {
        (ScriptValue::String(delimiter), ScriptValue::Array(parts)) => {
            let parts_vec = parts
                .to_vec()
                .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;
            let string_parts: Result<Vec<ScriptString>, RuntimeError> = parts_vec
                .iter()
                .map(|v| match v {
                    ScriptValue::String(s) => Ok((**s).clone()),
                    _ => Err(RuntimeError::InvalidOperation(
                        "join expects array of strings".to_string(),
                    )),
                })
                .collect();

            match string_parts {
                Ok(strings) => {
                    let joined = delimiter.join(&strings);
                    Ok(ScriptValue::String(ScriptRc::new(joined)))
                }
                Err(e) => Err(e),
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "join expects a string delimiter and an array of strings".to_string(),
        )),
    }
}

/// Pad string on the left
pub(crate) fn string_pad_left_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(format!(
            "pad_left expects 3 arguments, got {}",
            args.len()
        )));
    }

    match (&args[0], &args[1], &args[2]) {
        (ScriptValue::String(s), ScriptValue::I32(width), ScriptValue::String(fill)) => {
            if *width < 0 {
                return Err(RuntimeError::InvalidOperation(
                    "pad_left width must be non-negative".to_string(),
                ));
            }
            let padded = s.pad_left(*width as usize, fill.as_str());
            Ok(ScriptValue::String(ScriptRc::new(padded)))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "pad_left expects a string, an integer width, and a fill string".to_string(),
        )),
    }
}

/// Pad string on the right
pub(crate) fn string_pad_right_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(format!(
            "pad_right expects 3 arguments, got {}",
            args.len()
        )));
    }

    match (&args[0], &args[1], &args[2]) {
        (ScriptValue::String(s), ScriptValue::I32(width), ScriptValue::String(fill)) => {
            if *width < 0 {
                return Err(RuntimeError::InvalidOperation(
                    "pad_right width must be non-negative".to_string(),
                ));
            }
            let padded = s.pad_right(*width as usize, fill.as_str());
            Ok(ScriptValue::String(ScriptRc::new(padded)))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "pad_right expects a string, an integer width, and a fill string".to_string(),
        )),
    }
}

/// Center string within width
pub(crate) fn string_center_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(format!(
            "center expects 3 arguments, got {}",
            args.len()
        )));
    }

    match (&args[0], &args[1], &args[2]) {
        (ScriptValue::String(s), ScriptValue::I32(width), ScriptValue::String(fill)) => {
            if *width < 0 {
                return Err(RuntimeError::InvalidOperation(
                    "center width must be non-negative".to_string(),
                ));
            }
            let centered = s.center(*width as usize, fill.as_str());
            Ok(ScriptValue::String(ScriptRc::new(centered)))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "center expects a string, an integer width, and a fill string".to_string(),
        )),
    }
}

/// Strip prefix from string
pub(crate) fn string_strip_prefix_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "strip_prefix expects 2 arguments, got {}",
            args.len()
        )));
    }

    match (&args[0], &args[1]) {
        (ScriptValue::String(s), ScriptValue::String(prefix)) => {
            let stripped = s.strip_prefix(prefix.as_str());
            Ok(ScriptValue::String(ScriptRc::new(stripped)))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "strip_prefix expects two string arguments".to_string(),
        )),
    }
}

/// Strip suffix from string
pub(crate) fn string_strip_suffix_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "strip_suffix expects 2 arguments, got {}",
            args.len()
        )));
    }

    match (&args[0], &args[1]) {
        (ScriptValue::String(s), ScriptValue::String(suffix)) => {
            let stripped = s.strip_suffix(suffix.as_str());
            Ok(ScriptValue::String(ScriptRc::new(stripped)))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "strip_suffix expects two string arguments".to_string(),
        )),
    }
}

/// Capitalize string
pub(crate) fn string_capitalize_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "capitalize expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(s) => {
            let capitalized = s.capitalize();
            Ok(ScriptValue::String(ScriptRc::new(capitalized)))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "capitalize expects a string argument".to_string(),
        )),
    }
}

/// Convert to title case
pub(crate) fn string_title_case_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "title_case expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(s) => {
            let title_cased = s.title_case();
            Ok(ScriptValue::String(ScriptRc::new(title_cased)))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "title_case expects a string argument".to_string(),
        )),
    }
}

/// Count matches of a pattern
pub(crate) fn string_count_matches_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "count_matches expects 2 arguments, got {}",
            args.len()
        )));
    }

    match (&args[0], &args[1]) {
        (ScriptValue::String(s), ScriptValue::String(pattern)) => {
            let count = s.count_matches(pattern.as_str());
            Ok(ScriptValue::I32(count as i32))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "count_matches expects two string arguments".to_string(),
        )),
    }
}

/// Split string into lines
pub(crate) fn string_lines_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "lines expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(s) => {
            let lines = s.lines();
            let vec = ScriptVec::new();
            for line in lines {
                vec.push(ScriptValue::String(ScriptRc::new(line)))
                    .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;
            }
            Ok(ScriptValue::Array(ScriptRc::new(vec)))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "lines expects a string argument".to_string(),
        )),
    }
}

/// Reverse string
pub(crate) fn string_reverse_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "reverse expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(s) => {
            let reversed = s.reverse();
            Ok(ScriptValue::String(ScriptRc::new(reversed)))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "reverse expects a string argument".to_string(),
        )),
    }
}

/// Check if string is alphabetic
pub(crate) fn string_is_alphabetic_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "is_alphabetic expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(s) => Ok(ScriptValue::Bool(s.is_alphabetic())),
        _ => Err(RuntimeError::InvalidOperation(
            "is_alphabetic expects a string argument".to_string(),
        )),
    }
}

/// Check if string is numeric
pub(crate) fn string_is_numeric_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "is_numeric expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(s) => Ok(ScriptValue::Bool(s.is_numeric())),
        _ => Err(RuntimeError::InvalidOperation(
            "is_numeric expects a string argument".to_string(),
        )),
    }
}

/// Truncate string with ellipsis
pub(crate) fn string_truncate_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(format!(
            "truncate expects 3 arguments, got {}",
            args.len()
        )));
    }

    match (&args[0], &args[1], &args[2]) {
        (ScriptValue::String(s), ScriptValue::I32(max_len), ScriptValue::String(ellipsis)) => {
            if *max_len < 0 {
                return Err(RuntimeError::InvalidOperation(
                    "truncate max_len must be non-negative".to_string(),
                ));
            }
            let truncated = s.truncate(*max_len as usize, ellipsis.as_str());
            Ok(ScriptValue::String(ScriptRc::new(truncated)))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "truncate expects a string, an integer max length, and an ellipsis string".to_string(),
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
