//! Function ID interning and caching for closure performance optimization
//!
//! This module provides string interning for function IDs to reduce memory usage
//! and improve lookup performance by using numeric IDs instead of string comparisons.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};

/// Numeric ID type for interned function IDs
pub type FunctionId = u32;

/// Thread-safe string interning cache for function IDs
#[derive(Debug)]
pub struct FunctionIdCache {
    /// Maps string function IDs to numeric IDs
    string_to_id: RwLock<HashMap<String, FunctionId>>,
    /// Maps numeric IDs back to string IDs (for debugging)
    id_to_string: RwLock<HashMap<FunctionId, Arc<String>>>,
    /// Counter for generating unique IDs
    next_id: AtomicU32,
}

impl FunctionIdCache {
    /// Create a new function ID cache
    pub fn new() -> Self {
        FunctionIdCache {
            string_to_id: RwLock::new(HashMap::new()),
            id_to_string: RwLock::new(HashMap::new()),
            next_id: AtomicU32::new(1), // Start from 1, reserve 0 for invalid
        }
    }

    /// Get or create a numeric ID for a function string
    /// Returns the same ID for the same string across all calls
    pub fn get_or_intern(&self, function_name: &str) -> FunctionId {
        // Fast path: check if we already have this ID
        if let Ok(string_to_id) = self.string_to_id.read() {
            if let Some(&id) = string_to_id.get(function_name) {
                return id;
            }
        }

        // Slow path: need to intern the string
        let mut string_to_id = self.string_to_id.write().unwrap();
        let mut id_to_string = self.id_to_string.write().unwrap();

        // Double-check in case another thread added it
        if let Some(&id) = string_to_id.get(function_name) {
            return id;
        }

        // Generate new ID and intern the string
        let new_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let interned_string = Arc::new(function_name.to_string());

        string_to_id.insert(function_name.to_string(), new_id);
        id_to_string.insert(new_id, interned_string);

        new_id
    }

    /// Get the string representation of a function ID
    /// Returns None if the ID is not found
    pub fn get_string(&self, id: FunctionId) -> Option<Arc<String>> {
        self.id_to_string.read().unwrap().get(&id).cloned()
    }

    /// Check if a function ID exists in the cache
    pub fn contains_id(&self, id: FunctionId) -> bool {
        self.id_to_string.read().unwrap().contains_key(&id)
    }

    /// Get the number of interned function IDs
    pub fn len(&self) -> usize {
        self.string_to_id.read().unwrap().len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.string_to_id.read().unwrap().is_empty()
    }

    /// Clear all cached IDs (useful for testing)
    pub fn clear(&self) {
        self.string_to_id.write().unwrap().clear();
        self.id_to_string.write().unwrap().clear();
        self.next_id.store(1, Ordering::Relaxed);
    }
}

impl Default for FunctionIdCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Global function ID cache instance
static GLOBAL_FUNCTION_ID_CACHE: std::sync::OnceLock<FunctionIdCache> = std::sync::OnceLock::new();

/// Get the global function ID cache
pub fn global_function_id_cache() -> &'static FunctionIdCache {
    GLOBAL_FUNCTION_ID_CACHE.get_or_init(|| FunctionIdCache::new())
}

/// Convenience function to intern a function string globally
pub fn intern_function_id(function_name: &str) -> FunctionId {
    global_function_id_cache().get_or_intern(function_name)
}

/// Convenience function to get a function string from ID globally
pub fn get_function_string(id: FunctionId) -> Option<Arc<String>> {
    global_function_id_cache().get_string(id)
}

/// Optimized function ID that uses numeric comparison instead of string comparison
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OptimizedFunctionId {
    id: FunctionId,
}

impl OptimizedFunctionId {
    /// Create an optimized function ID from a string
    pub fn from_string(function_name: &str) -> Self {
        OptimizedFunctionId {
            id: intern_function_id(function_name),
        }
    }

    /// Get the numeric ID
    pub fn id(&self) -> FunctionId {
        self.id
    }

    /// Get the string representation
    pub fn as_string(&self) -> Option<Arc<String>> {
        get_function_string(self.id)
    }

    /// Create from a raw numeric ID (for testing)
    pub fn from_raw_id(id: FunctionId) -> Self {
        OptimizedFunctionId { id }
    }
}

impl From<&str> for OptimizedFunctionId {
    fn from(function_name: &str) -> Self {
        OptimizedFunctionId::from_string(function_name)
    }
}

impl From<String> for OptimizedFunctionId {
    fn from(function_name: String) -> Self {
        OptimizedFunctionId::from_string(&function_name)
    }
}

impl std::fmt::Display for OptimizedFunctionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.as_string() {
            Some(string) => write!(f, "{}", string),
            None => write!(f, "<unknown function #{}>", self.id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_function_id_cache_basic() {
        let cache = FunctionIdCache::new();

        // Test basic interning
        let id1 = cache.get_or_intern("test_function");
        let id2 = cache.get_or_intern("test_function");
        assert_eq!(id1, id2);

        // Test different strings get different IDs
        let id3 = cache.get_or_intern("other_function");
        assert_ne!(id1, id3);

        // Test string retrieval
        assert_eq!(cache.get_string(id1).unwrap().as_str(), "test_function");
        assert_eq!(cache.get_string(id3).unwrap().as_str(), "other_function");
    }

    #[test]
    fn test_function_id_cache_thread_safety() {
        let cache = Arc::new(FunctionIdCache::new());
        let mut handles = vec![];

        // Spawn multiple threads that intern the same string
        for _ in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let handle = thread::spawn(move || cache_clone.get_or_intern("concurrent_function"));
            handles.push(handle);
        }

        // All threads should get the same ID
        let mut results = vec![];
        for handle in handles {
            results.push(handle.join().unwrap());
        }

        assert!(results.iter().all(|&id| id == results[0]));
    }

    #[test]
    fn test_optimized_function_id() {
        let id1 = OptimizedFunctionId::from_string("test_func");
        let id2 = OptimizedFunctionId::from_string("test_func");
        let id3 = OptimizedFunctionId::from_string("other_func");

        // Same string should produce same ID
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);

        // Test display
        assert_eq!(format!("{id1}"), "test_func");
        assert_eq!(format!("{id3}"), "other_func");
    }

    #[test]
    fn test_global_cache() {
        let id1 = intern_function_id("global_test");
        let id2 = intern_function_id("global_test");
        assert_eq!(id1, id2);

        let string = get_function_string(id1).unwrap();
        assert_eq!(string.as_str(), "global_test");
    }

    #[test]
    fn test_cache_metrics() {
        let cache = FunctionIdCache::new();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());

        cache.get_or_intern("function1");
        cache.get_or_intern("function2");
        assert_eq!(cache.len(), 2);
        assert!(!cache.is_empty());

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }
}
