//! Optimized storage for closure captured variables
//!
//! This module provides efficient storage for captured variables that avoids
//! HashMap overhead for small capture counts and uses specialized data structures
//! for better performance.

use crate::runtime::Value;
use std::collections::HashMap;

/// Threshold for switching from inline storage to HashMap
const INLINE_THRESHOLD: usize = 4;

/// Optimized storage for captured variables that uses inline storage for small counts
#[derive(Debug, Clone)]
pub enum CaptureStorage {
    /// Inline storage for small capture counts (0-4 variables)
    Inline(InlineCaptures),
    /// HashMap storage for larger capture counts (5+ variables)
    HashMap(HashMap<String, Value>),
}

/// Inline storage for small numbers of captured variables
#[derive(Debug, Clone)]
pub struct InlineCaptures {
    /// Array of captured variables stored inline
    captures: [Option<(String, Value)>; INLINE_THRESHOLD],
    /// Number of active captures
    len: usize,
}

impl InlineCaptures {
    /// Create new empty inline captures
    pub fn new() -> Self {
        InlineCaptures {
            captures: [None, None, None, None],
            len: 0,
        }
    }

    /// Get the number of captured variables
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if there are no captured variables
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Check if at capacity
    pub fn is_full(&self) -> bool {
        self.len >= INLINE_THRESHOLD
    }

    /// Insert a captured variable, returns true if successful
    pub fn insert(&mut self, name: String, value: Value) -> bool {
        // Check if we're updating an existing capture
        for i in 0..self.len {
            if let Some((ref existing_name, _)) = &self.captures[i] {
                if existing_name == &name {
                    self.captures[i] = Some((name, value));
                    return true;
                }
            }
        }

        // Add new capture if we have space
        if self.len < INLINE_THRESHOLD {
            self.captures[self.len] = Some((name, value));
            self.len += 1;
            true
        } else {
            false
        }
    }

    /// Get a captured variable by name
    pub fn get(&self, name: &str) -> Option<&Value> {
        for i in 0..self.len {
            if let Some((ref capture_name, ref value)) = &self.captures[i] {
                if capture_name == name {
                    return Some(value);
                }
            }
        }
        None
    }

    /// Remove a captured variable, returns the value if found
    pub fn remove(&mut self, name: &str) -> Option<Value> {
        for i in 0..self.len {
            if let Some((ref capture_name, _)) = &self.captures[i] {
                if capture_name == name {
                    let removed = self.captures[i].take();
                    // Shift remaining elements down
                    for j in i..(self.len - 1) {
                        self.captures[j] = self.captures[j + 1].take();
                    }
                    self.len -= 1;
                    return removed.map(|(_, value)| value);
                }
            }
        }
        None
    }

    /// Iterate over captured variables
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.captures[0..self.len]
            .iter()
            .filter_map(|opt| opt.as_ref())
            .map(|(name, value)| (name, value))
    }

    /// Convert to HashMap (for when we exceed capacity)
    pub fn into_hashmap(self) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        for i in 0..self.len {
            if let Some((name, value)) = self.captures[i].clone() {
                map.insert(name, value);
            }
        }
        map
    }
}

impl Default for InlineCaptures {
    fn default() -> Self {
        Self::new()
    }
}

impl CaptureStorage {
    /// Create new empty capture storage
    pub fn new() -> Self {
        CaptureStorage::Inline(InlineCaptures::new())
    }

    /// Create from a vector of captures
    pub fn from_captures(captures: Vec<(String, Value)>) -> Self {
        if captures.len() <= INLINE_THRESHOLD {
            let mut inline = InlineCaptures::new();
            for (name, value) in captures {
                inline.insert(name, value);
            }
            CaptureStorage::Inline(inline)
        } else {
            CaptureStorage::HashMap(captures.into_iter().collect())
        }
    }

    /// Get the number of captured variables
    pub fn len(&self) -> usize {
        match self {
            CaptureStorage::Inline(inline) => inline.len(),
            CaptureStorage::HashMap(map) => map.len(),
        }
    }

    /// Check if there are no captured variables
    pub fn is_empty(&self) -> bool {
        match self {
            CaptureStorage::Inline(inline) => inline.is_empty(),
            CaptureStorage::HashMap(map) => map.is_empty(),
        }
    }

    /// Insert a captured variable
    pub fn insert(&mut self, name: String, value: Value) {
        match self {
            CaptureStorage::Inline(inline) => {
                if !inline.insert(name.clone(), value.clone()) {
                    // Inline storage is full, convert to HashMap
                    let mut map = inline.clone().into_hashmap();
                    map.insert(name, value);
                    *self = CaptureStorage::HashMap(map);
                }
            }
            CaptureStorage::HashMap(map) => {
                map.insert(name, value);
            }
        }
    }

    /// Get a captured variable by name
    pub fn get(&self, name: &str) -> Option<&Value> {
        match self {
            CaptureStorage::Inline(inline) => inline.get(name),
            CaptureStorage::HashMap(map) => map.get(name),
        }
    }

    /// Remove a captured variable
    pub fn remove(&mut self, name: &str) -> Option<Value> {
        match self {
            CaptureStorage::Inline(inline) => inline.remove(name),
            CaptureStorage::HashMap(map) => map.remove(name),
        }
    }

    /// Iterate over captured variables
    pub fn iter(&self) -> Box<dyn Iterator<Item = (&String, &Value)> + '_> {
        match self {
            CaptureStorage::Inline(inline) => Box::new(inline.iter()),
            CaptureStorage::HashMap(map) => Box::new(map.iter()),
        }
    }

    /// Check if storage contains closure values (for cycle detection)
    pub fn contains_closures(&self) -> bool {
        self.iter()
            .any(|(_, value)| matches!(value, Value::Closure(_) | Value::OptimizedClosure(_)))
    }

    /// Convert to HashMap (for compatibility)
    pub fn into_hashmap(self) -> HashMap<String, Value> {
        match self {
            CaptureStorage::Inline(inline) => inline.into_hashmap(),
            CaptureStorage::HashMap(map) => map,
        }
    }

    /// Get storage type for debugging
    pub fn storage_type(&self) -> &'static str {
        match self {
            CaptureStorage::Inline(_) => "inline",
            CaptureStorage::HashMap(_) => "hashmap",
        }
    }
}

impl Default for CaptureStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about capture storage usage
#[derive(Debug, Clone, Default)]
pub struct CaptureStorageStats {
    /// Number of closures using inline storage
    pub inline_count: usize,
    /// Number of closures using HashMap storage
    pub hashmap_count: usize,
    /// Total number of captured variables across all closures
    pub total_captures: usize,
}

impl CaptureStorageStats {
    /// Record usage of inline storage
    pub fn record_inline(&mut self, capture_count: usize) {
        self.inline_count += 1;
        self.total_captures += capture_count;
    }

    /// Record usage of HashMap storage
    pub fn record_hashmap(&mut self, capture_count: usize) {
        self.hashmap_count += 1;
        self.total_captures += capture_count;
    }

    /// Get the total number of closures
    pub fn total_closures(&self) -> usize {
        self.inline_count + self.hashmap_count
    }

    /// Get the percentage of closures using inline storage
    pub fn inline_percentage(&self) -> f64 {
        if self.total_closures() == 0 {
            0.0
        } else {
            (self.inline_count as f64 / self.total_closures() as f64) * 100.0
        }
    }

    /// Get the average number of captures per closure
    pub fn average_captures(&self) -> f64 {
        if self.total_closures() == 0 {
            0.0
        } else {
            self.total_captures as f64 / self.total_closures() as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inline_captures_basic() {
        let mut inline = InlineCaptures::new();
        assert_eq!(inline.len(), 0);
        assert!(inline.is_empty());

        // Insert captures
        assert!(inline.insert("x".to_string(), Value::I32(42)));
        assert!(inline.insert("y".to_string(), Value::String("hello".to_string());
        assert_eq!(inline.len(), 2);
        assert!(!inline.is_empty());

        // Test retrieval
        assert_eq!(inline.get("x"), Some(&Value::I32(42)));
        assert_eq!(inline.get("y"), Some(&Value::String("hello".to_string());
        assert_eq!(inline.get("z"), None);
    }

    #[test]
    fn test_inline_captures_capacity() {
        let mut inline = InlineCaptures::new();

        // Fill to capacity
        for i in 0..INLINE_THRESHOLD {
            assert!(inline.insert(format!("var_{i}"), Value::I32(i as i32)));
        }
        assert!(inline.is_full());

        // Should fail to insert more
        assert!(!inline.insert("overflow".to_string(), Value::I32(999)));
    }

    #[test]
    fn test_capture_storage_automatic_conversion() {
        let mut storage = CaptureStorage::new();

        // Should start as inline
        assert_eq!(storage.storage_type(), "inline");

        // Add variables within inline threshold
        for i in 0..INLINE_THRESHOLD {
            storage.insert(format!("var_{i}"), Value::I32(i as i32));
        }
        assert_eq!(storage.storage_type(), "inline");

        // Adding one more should convert to HashMap
        storage.insert("overflow".to_string(), Value::I32(999));
        assert_eq!(storage.storage_type(), "hashmap");

        // Should still be able to access all variables
        for i in 0..INLINE_THRESHOLD {
            assert_eq!(
                storage.get(&format!("var_{i}")),
                Some(&Value::I32(i as i32))
            );
        }
        assert_eq!(storage.get("overflow"), Some(&Value::I32(999)));
    }

    #[test]
    fn test_capture_storage_from_captures() {
        // Small capture count -> inline
        let small_captures = vec![
            ("x".to_string(), Value::I32(1)),
            ("y".to_string(), Value::I32(2)),
        ];
        let small_storage = CaptureStorage::from_captures(small_captures);
        assert_eq!(small_storage.storage_type(), "inline");

        // Large capture count -> HashMap
        let large_captures: Vec<_> = (0..10)
            .map(|i| (format!("var_{i}"), Value::I32(i)))
            .collect();
        let large_storage = CaptureStorage::from_captures(large_captures);
        assert_eq!(large_storage.storage_type(), "hashmap");
    }

    #[test]
    fn test_capture_storage_iteration() {
        let mut storage = CaptureStorage::new();
        storage.insert("a".to_string(), Value::I32(1));
        storage.insert("b".to_string(), Value::I32(2));
        storage.insert("c".to_string(), Value::I32(3));

        let mut collected: Vec<_> = storage.iter().collect();
        collected.sort_by_key(|(name, _)| name.as_str());

        assert_eq!(collected.len(), 3);
        assert_eq!(collected[0], (&"a".to_string(), &Value::I32(1)));
        assert_eq!(collected[1], (&"b".to_string(), &Value::I32(2)));
        assert_eq!(collected[2], (&"c".to_string(), &Value::I32(3)));
    }

    #[test]
    fn test_capture_storage_stats() {
        let mut stats = CaptureStorageStats::default();

        stats.record_inline(2);
        stats.record_inline(1);
        stats.record_hashmap(10);

        assert_eq!(stats.total_closures(), 3);
        assert_eq!(stats.inline_percentage(), 66.66666666666666);
        assert_eq!(stats.average_captures(), 13.0 / 3.0);
    }
}
