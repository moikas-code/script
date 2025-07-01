//! Collections for the Script programming language
//! 
//! This module provides dynamic arrays (Vec) and hash maps (HashMap)
//! that can be used from Script code.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::runtime::{ScriptRc, RuntimeError};
use crate::stdlib::{ScriptValue, ScriptString, ScriptOption};

/// A dynamic array for Script
#[derive(Debug, Clone)]
pub struct ScriptVec {
    /// The underlying vector, wrapped in Arc<RwLock> for interior mutability
    data: Arc<RwLock<Vec<ScriptValue>>>,
}

impl ScriptVec {
    /// Create a new empty vector
    pub fn new() -> Self {
        ScriptVec {
            data: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Create a vector with initial capacity
    pub fn with_capacity(capacity: usize) -> Self {
        ScriptVec {
            data: Arc::new(RwLock::new(Vec::with_capacity(capacity))),
        }
    }
    
    /// Get the length of the vector
    pub fn len(&self) -> usize {
        self.data.read().unwrap().len()
    }
    
    /// Check if the vector is empty
    pub fn is_empty(&self) -> bool {
        self.data.read().unwrap().is_empty()
    }
    
    /// Push a value to the end of the vector
    pub fn push(&self, value: ScriptValue) {
        self.data.write().unwrap().push(value);
    }
    
    /// Pop a value from the end of the vector
    pub fn pop(&self) -> Option<ScriptValue> {
        self.data.write().unwrap().pop()
    }
    
    /// Get a value at an index
    pub fn get(&self, index: usize) -> Option<ScriptValue> {
        self.data.read().unwrap().get(index).cloned()
    }
    
    /// Set a value at an index
    /// Returns true if successful, false if index out of bounds
    pub fn set(&self, index: usize, value: ScriptValue) -> bool {
        let mut data = self.data.write().unwrap();
        if index < data.len() {
            data[index] = value;
            true
        } else {
            false
        }
    }
    
    /// Insert a value at an index
    /// Panics if index > len
    pub fn insert(&self, index: usize, value: ScriptValue) {
        self.data.write().unwrap().insert(index, value);
    }
    
    /// Remove a value at an index
    /// Returns None if index out of bounds
    pub fn remove(&self, index: usize) -> Option<ScriptValue> {
        let mut data = self.data.write().unwrap();
        if index < data.len() {
            Some(data.remove(index))
        } else {
            None
        }
    }
    
    /// Clear all elements from the vector
    pub fn clear(&self) {
        self.data.write().unwrap().clear();
    }
    
    /// Get the first element
    pub fn first(&self) -> Option<ScriptValue> {
        self.data.read().unwrap().first().cloned()
    }
    
    /// Get the last element
    pub fn last(&self) -> Option<ScriptValue> {
        self.data.read().unwrap().last().cloned()
    }
    
    /// Check if the vector contains a value
    pub fn contains(&self, value: &ScriptValue) -> bool {
        self.data.read().unwrap().contains(value)
    }
    
    /// Find the index of a value
    pub fn index_of(&self, value: &ScriptValue) -> Option<usize> {
        self.data.read().unwrap().iter().position(|v| v == value)
    }
    
    /// Reverse the vector in place
    pub fn reverse(&self) {
        self.data.write().unwrap().reverse();
    }
    
    /// Sort the vector (only works for comparable types)
    pub fn sort(&self) -> Result<(), String> {
        let mut data = self.data.write().unwrap();
        
        // Check if all elements are the same comparable type
        if data.is_empty() {
            return Ok(());
        }
        
        let first_type = data[0].get_type();
        if !first_type.is_comparable() {
            return Err(format!("Cannot sort vector of non-comparable type: {}", first_type));
        }
        
        for val in data.iter() {
            if !val.get_type().equals(&first_type) {
                return Err("Cannot sort vector with mixed types".to_string());
            }
        }
        
        // Sort based on type
        match &data[0] {
            ScriptValue::I32(_) => {
                data.sort_by(|a, b| {
                    a.as_i32().unwrap().cmp(&b.as_i32().unwrap())
                });
            }
            ScriptValue::F32(_) => {
                data.sort_by(|a, b| {
                    a.as_f32().unwrap().partial_cmp(&b.as_f32().unwrap()).unwrap()
                });
            }
            ScriptValue::Bool(_) => {
                data.sort_by(|a, b| {
                    a.as_bool().unwrap().cmp(&b.as_bool().unwrap())
                });
            }
            ScriptValue::String(_) => {
                data.sort_by(|a, b| {
                    a.as_string().unwrap().as_str().cmp(&b.as_string().unwrap().as_str())
                });
            }
            _ => return Err("Cannot sort this type".to_string()),
        }
        
        Ok(())
    }
    
    /// Convert to a Rust Vec (for internal use)
    pub fn to_vec(&self) -> Vec<ScriptValue> {
        self.data.read().unwrap().clone()
    }
}

impl Default for ScriptVec {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for ScriptVec {
    fn eq(&self, other: &Self) -> bool {
        let self_data = self.data.read().unwrap();
        let other_data = other.data.read().unwrap();
        *self_data == *other_data
    }
}

/// A hash map for Script
#[derive(Debug, Clone)]
pub struct ScriptHashMap {
    /// The underlying hash map, wrapped in Arc<RwLock> for interior mutability
    /// Keys are always strings in Script
    data: Arc<RwLock<HashMap<String, ScriptValue>>>,
}

impl ScriptHashMap {
    /// Create a new empty hash map
    pub fn new() -> Self {
        ScriptHashMap {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.data.read().unwrap().len()
    }
    
    /// Check if the map is empty
    pub fn is_empty(&self) -> bool {
        self.data.read().unwrap().is_empty()
    }
    
    /// Insert a key-value pair
    /// Returns the previous value if any
    pub fn insert(&self, key: String, value: ScriptValue) -> Option<ScriptValue> {
        self.data.write().unwrap().insert(key, value)
    }
    
    /// Get a value by key
    pub fn get(&self, key: &str) -> Option<ScriptValue> {
        self.data.read().unwrap().get(key).cloned()
    }
    
    /// Remove a key-value pair
    /// Returns the removed value if any
    pub fn remove(&self, key: &str) -> Option<ScriptValue> {
        self.data.write().unwrap().remove(key)
    }
    
    /// Check if a key exists
    pub fn contains_key(&self, key: &str) -> bool {
        self.data.read().unwrap().contains_key(key)
    }
    
    /// Clear all entries
    pub fn clear(&self) {
        self.data.write().unwrap().clear();
    }
    
    /// Get all keys as a vector
    pub fn keys(&self) -> ScriptVec {
        let vec = ScriptVec::new();
        for key in self.data.read().unwrap().keys() {
            vec.push(ScriptValue::String(ScriptRc::new(ScriptString::from_str(key))));
        }
        vec
    }
    
    /// Get all values as a vector
    pub fn values(&self) -> ScriptVec {
        let vec = ScriptVec::new();
        for value in self.data.read().unwrap().values() {
            vec.push(value.clone());
        }
        vec
    }
}

impl Default for ScriptHashMap {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for ScriptHashMap {
    fn eq(&self, other: &Self) -> bool {
        let self_data = self.data.read().unwrap();
        let other_data = other.data.read().unwrap();
        *self_data == *other_data
    }
}

// Implementation functions for stdlib registry

/// Create a new vector
pub(crate) fn vec_new_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if !args.is_empty() {
        return Err(RuntimeError::InvalidOperation(
            format!("Vec::new expects 0 arguments, got {}", args.len())
        ));
    }
    
    Ok(ScriptValue::Array(ScriptRc::new(ScriptVec::new())))
}

/// Get the length of a vector
pub(crate) fn vec_len_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("len expects 1 argument, got {}", args.len())
        ));
    }
    
    match &args[0] {
        ScriptValue::Array(vec) => Ok(ScriptValue::I32(vec.len() as i32)),
        _ => Err(RuntimeError::InvalidOperation(
            "len expects a vector argument".to_string()
        )),
    }
}

/// Push a value to a vector
pub(crate) fn vec_push_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            format!("push expects 2 arguments, got {}", args.len())
        ));
    }
    
    match &args[0] {
        ScriptValue::Array(vec) => {
            vec.push(args[1].clone());
            Ok(ScriptValue::Unit)
        }
        _ => Err(RuntimeError::InvalidOperation(
            "push expects a vector as first argument".to_string()
        )),
    }
}

/// Pop a value from a vector
pub(crate) fn vec_pop_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("pop expects 1 argument, got {}", args.len())
        ));
    }
    
    match &args[0] {
        ScriptValue::Array(vec) => {
            match vec.pop() {
                Some(val) => Ok(ScriptValue::Option(ScriptRc::new(ScriptOption::some(val)))),
                None => Ok(ScriptValue::Option(ScriptRc::new(ScriptOption::none()))),
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "pop expects a vector argument".to_string()
        )),
    }
}

/// Get a value from a vector at an index
pub(crate) fn vec_get_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            format!("get expects 2 arguments, got {}", args.len())
        ));
    }
    
    match (&args[0], &args[1]) {
        (ScriptValue::Array(vec), ScriptValue::I32(index)) => {
            if *index < 0 {
                return Err(RuntimeError::InvalidOperation(
                    format!("Index cannot be negative: {}", index)
                ));
            }
            
            match vec.get(*index as usize) {
                Some(val) => Ok(ScriptValue::Option(ScriptRc::new(ScriptOption::some(val)))),
                None => Ok(ScriptValue::Option(ScriptRc::new(ScriptOption::none()))),
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "get expects a vector and an integer index".to_string()
        )),
    }
}

/// Create a new hash map
pub(crate) fn hashmap_new_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if !args.is_empty() {
        return Err(RuntimeError::InvalidOperation(
            format!("HashMap::new expects 0 arguments, got {}", args.len())
        ));
    }
    
    Ok(ScriptValue::HashMap(ScriptRc::new(ScriptHashMap::new())))
}

/// Insert a key-value pair into a hash map
pub(crate) fn hashmap_insert_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(
            format!("insert expects 3 arguments, got {}", args.len())
        ));
    }
    
    match (&args[0], &args[1]) {
        (ScriptValue::HashMap(map), ScriptValue::String(key)) => {
            match map.insert(key.as_str().to_string(), args[2].clone()) {
                Some(old_val) => Ok(ScriptValue::Option(ScriptRc::new(ScriptOption::some(old_val)))),
                None => Ok(ScriptValue::Option(ScriptRc::new(ScriptOption::none()))),
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "insert expects a HashMap and a string key".to_string()
        )),
    }
}

/// Get a value from a hash map by key
pub(crate) fn hashmap_get_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            format!("get expects 2 arguments, got {}", args.len())
        ));
    }
    
    match (&args[0], &args[1]) {
        (ScriptValue::HashMap(map), ScriptValue::String(key)) => {
            match map.get(&key.as_str()) {
                Some(val) => Ok(ScriptValue::Option(ScriptRc::new(ScriptOption::some(val)))),
                None => Ok(ScriptValue::Option(ScriptRc::new(ScriptOption::none()))),
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "get expects a HashMap and a string key".to_string()
        )),
    }
}

/// Check if a key exists in a hash map
pub(crate) fn hashmap_contains_key_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            format!("contains_key expects 2 arguments, got {}", args.len())
        ));
    }
    
    match (&args[0], &args[1]) {
        (ScriptValue::HashMap(map), ScriptValue::String(key)) => {
            Ok(ScriptValue::Bool(map.contains_key(&key.as_str())))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "contains_key expects a HashMap and a string key".to_string()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_vec_creation() {
        let vec = ScriptVec::new();
        assert_eq!(vec.len(), 0);
        assert!(vec.is_empty());
        
        let vec_cap = ScriptVec::with_capacity(10);
        assert_eq!(vec_cap.len(), 0);
    }
    
    #[test]
    fn test_vec_push_pop() {
        let vec = ScriptVec::new();
        
        vec.push(ScriptValue::I32(10));
        vec.push(ScriptValue::I32(20));
        vec.push(ScriptValue::I32(30));
        
        assert_eq!(vec.len(), 3);
        assert!(!vec.is_empty());
        
        assert_eq!(vec.pop(), Some(ScriptValue::I32(30)));
        assert_eq!(vec.pop(), Some(ScriptValue::I32(20)));
        assert_eq!(vec.len(), 1);
        
        assert_eq!(vec.pop(), Some(ScriptValue::I32(10)));
        assert_eq!(vec.pop(), None);
        assert!(vec.is_empty());
    }
    
    #[test]
    fn test_vec_get_set() {
        let vec = ScriptVec::new();
        vec.push(ScriptValue::I32(10));
        vec.push(ScriptValue::I32(20));
        vec.push(ScriptValue::I32(30));
        
        assert_eq!(vec.get(0), Some(ScriptValue::I32(10)));
        assert_eq!(vec.get(1), Some(ScriptValue::I32(20)));
        assert_eq!(vec.get(2), Some(ScriptValue::I32(30)));
        assert_eq!(vec.get(3), None);
        
        assert!(vec.set(1, ScriptValue::I32(25)));
        assert_eq!(vec.get(1), Some(ScriptValue::I32(25)));
        
        assert!(!vec.set(10, ScriptValue::I32(100))); // Out of bounds
    }
    
    #[test]
    fn test_vec_insert_remove() {
        let vec = ScriptVec::new();
        vec.push(ScriptValue::I32(10));
        vec.push(ScriptValue::I32(30));
        
        vec.insert(1, ScriptValue::I32(20));
        assert_eq!(vec.len(), 3);
        assert_eq!(vec.get(1), Some(ScriptValue::I32(20)));
        
        assert_eq!(vec.remove(1), Some(ScriptValue::I32(20)));
        assert_eq!(vec.len(), 2);
        assert_eq!(vec.get(1), Some(ScriptValue::I32(30)));
        
        assert_eq!(vec.remove(10), None); // Out of bounds
    }
    
    #[test]
    fn test_vec_methods() {
        let vec = ScriptVec::new();
        vec.push(ScriptValue::I32(10));
        vec.push(ScriptValue::I32(20));
        vec.push(ScriptValue::I32(30));
        
        assert_eq!(vec.first(), Some(ScriptValue::I32(10)));
        assert_eq!(vec.last(), Some(ScriptValue::I32(30)));
        
        assert!(vec.contains(&ScriptValue::I32(20)));
        assert!(!vec.contains(&ScriptValue::I32(40)));
        
        assert_eq!(vec.index_of(&ScriptValue::I32(20)), Some(1));
        assert_eq!(vec.index_of(&ScriptValue::I32(40)), None);
        
        vec.reverse();
        assert_eq!(vec.get(0), Some(ScriptValue::I32(30)));
        assert_eq!(vec.get(2), Some(ScriptValue::I32(10)));
    }
    
    #[test]
    fn test_vec_sort() {
        let vec = ScriptVec::new();
        vec.push(ScriptValue::I32(30));
        vec.push(ScriptValue::I32(10));
        vec.push(ScriptValue::I32(20));
        
        assert!(vec.sort().is_ok());
        assert_eq!(vec.get(0), Some(ScriptValue::I32(10)));
        assert_eq!(vec.get(1), Some(ScriptValue::I32(20)));
        assert_eq!(vec.get(2), Some(ScriptValue::I32(30)));
        
        // Test sorting strings
        let str_vec = ScriptVec::new();
        str_vec.push(ScriptValue::String(ScriptRc::new(ScriptString::from_str("charlie"))));
        str_vec.push(ScriptValue::String(ScriptRc::new(ScriptString::from_str("alice"))));
        str_vec.push(ScriptValue::String(ScriptRc::new(ScriptString::from_str("bob"))));
        
        assert!(str_vec.sort().is_ok());
        assert_eq!(
            str_vec.get(0).unwrap().as_string().unwrap().as_str(),
            "alice"
        );
    }
    
    #[test]
    fn test_hashmap_creation() {
        let map = ScriptHashMap::new();
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
    }
    
    #[test]
    fn test_hashmap_insert_get() {
        let map = ScriptHashMap::new();
        
        assert_eq!(map.insert("key1".to_string(), ScriptValue::I32(10)), None);
        assert_eq!(map.insert("key2".to_string(), ScriptValue::I32(20)), None);
        
        assert_eq!(map.len(), 2);
        assert!(!map.is_empty());
        
        assert_eq!(map.get("key1"), Some(ScriptValue::I32(10)));
        assert_eq!(map.get("key2"), Some(ScriptValue::I32(20)));
        assert_eq!(map.get("key3"), None);
        
        // Test overwrite
        assert_eq!(
            map.insert("key1".to_string(), ScriptValue::I32(15)),
            Some(ScriptValue::I32(10))
        );
        assert_eq!(map.get("key1"), Some(ScriptValue::I32(15)));
    }
    
    #[test]
    fn test_hashmap_remove() {
        let map = ScriptHashMap::new();
        map.insert("key1".to_string(), ScriptValue::I32(10));
        map.insert("key2".to_string(), ScriptValue::I32(20));
        
        assert_eq!(map.remove("key1"), Some(ScriptValue::I32(10)));
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("key1"), None);
        
        assert_eq!(map.remove("key3"), None); // Non-existent key
    }
    
    #[test]
    fn test_hashmap_contains_key() {
        let map = ScriptHashMap::new();
        map.insert("key1".to_string(), ScriptValue::I32(10));
        
        assert!(map.contains_key("key1"));
        assert!(!map.contains_key("key2"));
    }
    
    #[test]
    fn test_hashmap_keys_values() {
        let map = ScriptHashMap::new();
        map.insert("a".to_string(), ScriptValue::I32(1));
        map.insert("b".to_string(), ScriptValue::I32(2));
        map.insert("c".to_string(), ScriptValue::I32(3));
        
        let keys = map.keys();
        assert_eq!(keys.len(), 3);
        // Note: HashMap iteration order is not guaranteed
        
        let values = map.values();
        assert_eq!(values.len(), 3);
    }
    
    #[test]
    fn test_hashmap_clear() {
        let map = ScriptHashMap::new();
        map.insert("key1".to_string(), ScriptValue::I32(10));
        map.insert("key2".to_string(), ScriptValue::I32(20));
        
        map.clear();
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
        assert_eq!(map.get("key1"), None);
    }
}