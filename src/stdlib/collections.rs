//! Collections for the Script programming language
//!
//! This module provides dynamic arrays (Vec) and hash maps (HashMap)
//! that can be used from Script code, with functional programming capabilities.

use crate::error::{Error, ErrorKind, Result};
use crate::runtime::{RuntimeError, ScriptRc, Value};
use crate::stdlib::{ScriptOption, ScriptString, ScriptValue};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A dynamic array for Script
#[derive(Debug, Clone)]
pub struct ScriptVec {
    /// The underlying vector, wrapped in Arc<RwLock> for interior mutability
    pub data: Arc<RwLock<Vec<ScriptValue>>>,
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

    /// Create a vector from a Vec of ScriptValues
    pub fn from_vec(vec: Vec<ScriptValue>) -> Self {
        ScriptVec {
            data: Arc::new(RwLock::new(vec)),
        }
    }

    /// Get the length of the vector
    pub fn len(&self) -> usize {
        self.data.read().map(|guard| guard.len()).unwrap_or(0) // Return 0 on lock failure rather than panic
    }

    /// Check if the vector is empty
    pub fn is_empty(&self) -> bool {
        self.data
            .read()
            .map(|guard| guard.is_empty())
            .unwrap_or(true) // Return true on lock failure for safety
    }

    /// Push a value to the end of the vector
    pub fn push(&self, value: ScriptValue) -> Result<()> {
        self.data
            .write()
            .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on vector data"))?
            .push(value);
        Ok(())
    }

    /// Pop a value from the end of the vector
    pub fn pop(&self) -> Result<Option<ScriptValue>> {
        Ok(self
            .data
            .write()
            .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on vector data"))?
            .pop())
    }

    /// Get a value at an index
    pub fn get(&self, index: usize) -> Result<Option<ScriptValue>> {
        Ok(self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to acquire read lock on vector data"))?
            .get(index)
            .cloned())
    }

    /// Set a value at an index
    /// Returns true if successful, false if index out of bounds
    pub fn set(&self, index: usize, value: ScriptValue) -> Result<bool> {
        let mut data = self
            .data
            .write()
            .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on vector data"))?;
        if index < data.len() {
            data[index] = value;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Insert a value at an index
    /// Returns error if index > len or lock fails
    pub fn insert(&self, index: usize, value: ScriptValue) -> Result<()> {
        let mut data = self
            .data
            .write()
            .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on vector data"))?;
        if index > data.len() {
            return Err(Error::index_out_of_bounds(index, data.len()));
        }
        data.insert(index, value);
        Ok(())
    }

    /// Remove a value at an index
    /// Returns None if index out of bounds
    pub fn remove(&self, index: usize) -> Result<Option<ScriptValue>> {
        let mut data = self
            .data
            .write()
            .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on vector data"))?;
        if index < data.len() {
            Ok(Some(data.remove(index)))
        } else {
            Ok(None)
        }
    }

    /// Clear all elements from the vector
    pub fn clear(&self) -> Result<()> {
        self.data
            .write()
            .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on vector data"))?
            .clear();
        Ok(())
    }

    /// Get the first element
    pub fn first(&self) -> Result<Option<ScriptValue>> {
        Ok(self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to acquire read lock on vector data"))?
            .first()
            .cloned())
    }

    /// Get the last element
    pub fn last(&self) -> Result<Option<ScriptValue>> {
        Ok(self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to acquire read lock on vector data"))?
            .last()
            .cloned())
    }

    /// Check if the vector contains a value
    pub fn contains(&self, value: &ScriptValue) -> bool {
        self.data
            .read()
            .map(|guard| guard.contains(value))
            .unwrap_or(false) // Return false on lock failure for safety
    }

    /// Find the index of a value
    pub fn index_of(&self, value: &ScriptValue) -> Result<Option<usize>> {
        Ok(self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to acquire read lock on vector data"))?
            .iter()
            .position(|v| v == value))
    }

    /// Reverse the vector in place
    pub fn reverse(&self) -> Result<()> {
        self.data
            .write()
            .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on vector data"))?
            .reverse();
        Ok(())
    }

    /// Sort the vector (only works for comparable types)
    pub fn sort(&self) -> Result<()> {
        let mut data = self
            .data
            .write()
            .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on vector data"))?;

        // Check if all elements are the same comparable type
        if data.is_empty() {
            return Ok(());
        }

        let first_type = data[0].get_type();
        if !first_type.is_comparable() {
            return Err(Error::type_error(format!(
                "Cannot sort vector of non-comparable type: {}",
                first_type
            )));
        }

        for val in data.iter() {
            if !val.get_type().equals(&first_type) {
                return Err(Error::type_error("Cannot sort vector with mixed types"));
            }
        }

        // Sort based on type
        match &data[0] {
            ScriptValue::I32(_) => {
                data.sort_by(|a, b| {
                    let a_val = a.as_i32().expect("Type checked to be I32");
                    let b_val = b.as_i32().expect("Type checked to be I32");
                    a_val.cmp(&b_val)
                });
            }
            ScriptValue::F32(_) => {
                data.sort_by(|a, b| {
                    let a_val = a.as_f32().expect("Type checked to be F32");
                    let b_val = b.as_f32().expect("Type checked to be F32");
                    a_val
                        .partial_cmp(&b_val)
                        .unwrap_or(std::cmp::Ordering::Equal) // Handle NaN gracefully
                });
            }
            ScriptValue::Bool(_) => {
                data.sort_by(|a, b| {
                    let a_val = a.as_bool().expect("Type checked to be Bool");
                    let b_val = b.as_bool().expect("Type checked to be Bool");
                    a_val.cmp(&b_val)
                });
            }
            ScriptValue::String(_) => {
                data.sort_by(|a, b| {
                    let a_str = a.as_string().expect("Type checked to be String");
                    let b_str = b.as_string().expect("Type checked to be String");
                    a_str.as_str().cmp(&b_str.as_str())
                });
            }
            _ => return Err(Error::type_error("Cannot sort this type")),
        }

        Ok(())
    }

    /// Convert to a Rust Vec (for internal use)
    pub fn to_vec(&self) -> Result<Vec<ScriptValue>> {
        Ok(self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to acquire read lock on vector data"))?
            .clone())
    }

    /// Create an iterator over the vector elements
    pub fn iter(&self) -> Result<Vec<ScriptValue>> {
        self.to_vec()
    }

    // Functional programming methods

    /// Map over each element using a closure
    pub fn map(&self, closure: &Value) -> Result<Self> {
        use crate::stdlib::functional::FunctionalOps;
        FunctionalOps::map(self, closure)
    }

    /// Filter elements using a predicate closure
    pub fn filter(&self, predicate: &Value) -> Result<Self> {
        use crate::stdlib::functional::FunctionalOps;
        FunctionalOps::filter(self, predicate)
    }

    /// Reduce the collection to a single value using an accumulator closure
    pub fn reduce(&self, closure: &Value, initial: ScriptValue) -> Result<ScriptValue> {
        use crate::stdlib::functional::FunctionalOps;
        FunctionalOps::reduce(self, closure, initial)
    }

    /// Execute a closure for each element (side effects)
    pub fn for_each(&self, closure: &Value) -> Result<()> {
        use crate::stdlib::functional::FunctionalOps;
        FunctionalOps::for_each(self, closure)
    }

    /// Find the first element matching a predicate
    pub fn find(&self, predicate: &Value) -> Result<ScriptOption> {
        use crate::stdlib::functional::FunctionalOps;
        FunctionalOps::find(self, predicate)
    }

    /// Test if all elements satisfy a predicate
    pub fn every(&self, predicate: &Value) -> Result<bool> {
        use crate::stdlib::functional::FunctionalOps;
        FunctionalOps::every(self, predicate)
    }

    /// Test if any element satisfies a predicate
    pub fn some(&self, predicate: &Value) -> Result<bool> {
        use crate::stdlib::functional::FunctionalOps;
        FunctionalOps::some(self, predicate)
    }

    /// Chain with another vector (concatenate)
    pub fn chain(&self, other: &Self) -> Result<Self> {
        let data1 = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read vector data"))?;
        let data2 = other
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read other vector data"))?;

        let mut result_data = data1.clone();
        result_data.extend(data2.iter().cloned());

        Ok(ScriptVec {
            data: Arc::new(RwLock::new(result_data)),
        })
    }

    /// Take the first n elements
    pub fn take(&self, n: usize) -> Result<Self> {
        let data = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read vector data"))?;

        let taken_elements: Vec<ScriptValue> = data.iter().take(n).cloned().collect();

        Ok(ScriptVec {
            data: Arc::new(RwLock::new(taken_elements)),
        })
    }

    /// Skip the first n elements
    pub fn skip(&self, n: usize) -> Result<Self> {
        let data = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read vector data"))?;

        let skipped_elements: Vec<ScriptValue> = data.iter().skip(n).cloned().collect();

        Ok(ScriptVec {
            data: Arc::new(RwLock::new(skipped_elements)),
        })
    }

    /// Zip this vector with another vector
    pub fn zip(&self, other: &Self) -> Result<ScriptVec> {
        let self_data = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read vector data"))?;
        let other_data = other
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read other vector data"))?;

        let mut zipped = Vec::new();
        for (a, b) in self_data.iter().zip(other_data.iter()) {
            // Create a tuple as an array with two elements
            let tuple = ScriptVec::new();
            tuple.push(a.clone())?;
            tuple.push(b.clone())?;
            zipped.push(ScriptValue::Array(ScriptRc::new(tuple)));
        }

        Ok(ScriptVec {
            data: Arc::new(RwLock::new(zipped)),
        })
    }

    /// Enumerate elements with their indices
    pub fn enumerate(&self) -> Result<ScriptVec> {
        let data = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read vector data"))?;

        let mut enumerated = Vec::new();
        for (i, value) in data.iter().enumerate() {
            // Create a tuple as an array with index and value
            let tuple = ScriptVec::new();
            tuple.push(ScriptValue::I32(i as i32))?;
            tuple.push(value.clone())?;
            enumerated.push(ScriptValue::Array(ScriptRc::new(tuple)));
        }

        Ok(ScriptVec {
            data: Arc::new(RwLock::new(enumerated)),
        })
    }

    /// Flatten a vector of vectors
    pub fn flatten(&self) -> Result<ScriptVec> {
        let data = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read vector data"))?;

        let mut flattened = Vec::new();
        for item in data.iter() {
            match item {
                ScriptValue::Array(inner_vec) => {
                    let inner_data = inner_vec
                        .data
                        .read()
                        .map_err(|_| Error::lock_poisoned("Failed to read inner vector data"))?;
                    flattened.extend(inner_data.iter().cloned());
                }
                _ => flattened.push(item.clone()),
            }
        }

        Ok(ScriptVec {
            data: Arc::new(RwLock::new(flattened)),
        })
    }

    /// Collect unique elements (removes duplicates)
    pub fn unique(&self) -> Result<ScriptVec> {
        let data = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read vector data"))?;

        let mut unique_elements = Vec::new();
        for item in data.iter() {
            if !unique_elements.contains(item) {
                unique_elements.push(item.clone());
            }
        }

        Ok(ScriptVec {
            data: Arc::new(RwLock::new(unique_elements)),
        })
    }

    /// Partition elements into two vectors based on a predicate
    pub fn partition(&self, predicate: &Value) -> Result<(ScriptVec, ScriptVec)> {
        use crate::stdlib::functional::FunctionalExecutor;

        let closure_ref = match predicate {
            Value::Closure(c) => c,
            _ => {
                return Err(Error::new(
                    ErrorKind::TypeError,
                    "Expected a closure for partition operation",
                ))
            }
        };

        let mut executor = FunctionalExecutor::new();
        let data = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read vector data"))?;

        let mut true_elements = Vec::new();
        let mut false_elements = Vec::new();

        for item in data.iter() {
            let matches = executor.execute_predicate(closure_ref, item.clone())?;
            if matches {
                true_elements.push(item.clone());
            } else {
                false_elements.push(item.clone());
            }
        }

        Ok((
            ScriptVec {
                data: Arc::new(RwLock::new(true_elements)),
            },
            ScriptVec {
                data: Arc::new(RwLock::new(false_elements)),
            },
        ))
    }

    /// Shuffle the elements in place using the provided RNG
    pub fn shuffle<R: rand::Rng>(&self, rng: &mut R) -> Result<()> {
        use rand::seq::SliceRandom;
        let mut data = self
            .data
            .write()
            .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on vector data"))?;
        data.shuffle(rng);
        Ok(())
    }
}

impl Default for ScriptVec {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for ScriptVec {
    fn eq(&self, other: &Self) -> bool {
        match (self.data.read(), other.data.read()) {
            (Ok(self_data), Ok(other_data)) => *self_data == *other_data,
            _ => false, // If we can't compare due to lock failure, assume not equal
        }
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
        self.data.read().map(|guard| guard.len()).unwrap_or(0) // Return 0 on lock failure
    }

    /// Check if the map is empty
    pub fn is_empty(&self) -> bool {
        self.data
            .read()
            .map(|guard| guard.is_empty())
            .unwrap_or(true) // Return true on lock failure for safety
    }

    /// Insert a key-value pair
    /// Returns the previous value if any
    pub fn insert(&self, key: String, value: ScriptValue) -> Result<Option<ScriptValue>> {
        Ok(self
            .data
            .write()
            .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on hash map data"))?
            .insert(key, value))
    }

    /// Get a value by key
    pub fn get(&self, key: &str) -> Result<Option<ScriptValue>> {
        Ok(self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to acquire read lock on hash map data"))?
            .get(key)
            .cloned())
    }

    /// Remove a key-value pair
    /// Returns the removed value if any
    pub fn remove(&self, key: &str) -> Result<Option<ScriptValue>> {
        Ok(self
            .data
            .write()
            .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on hash map data"))?
            .remove(key))
    }

    /// Check if a key exists
    pub fn contains_key(&self, key: &str) -> bool {
        self.data
            .read()
            .map(|guard| guard.contains_key(key))
            .unwrap_or(false) // Return false on lock failure for safety
    }

    /// Clear all entries
    pub fn clear(&self) -> Result<()> {
        self.data
            .write()
            .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on hash map data"))?
            .clear();
        Ok(())
    }

    /// Get all keys as a vector
    pub fn keys(&self) -> Result<ScriptVec> {
        let vec = ScriptVec::new();
        let data = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to acquire read lock on hash map data"))?;
        for key in data.keys() {
            vec.push(ScriptValue::String(ScriptRc::new(ScriptString::from_str(
                key,
            ))))?;
        }
        Ok(vec)
    }

    /// Get all values as a vector
    pub fn values(&self) -> Result<ScriptVec> {
        let vec = ScriptVec::new();
        let data = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to acquire read lock on hash map data"))?;
        for value in data.values() {
            vec.push(value.clone())?;
        }
        Ok(vec)
    }

    /// Create an iterator over the hash map entries
    pub fn iter(&self) -> Result<Vec<(String, ScriptValue)>> {
        let data = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to acquire read lock on hash map data"))?;
        Ok(data.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
    }
}

impl Default for ScriptHashMap {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for ScriptHashMap {
    fn eq(&self, other: &Self) -> bool {
        match (self.data.read(), other.data.read()) {
            (Ok(self_data), Ok(other_data)) => *self_data == *other_data,
            _ => false, // If we can't compare due to lock failure, assume not equal
        }
    }
}

// Implementation functions for stdlib registry

/// Create a new vector
pub(crate) fn vec_new_impl(args: &[ScriptValue]) -> std::result::Result<ScriptValue, RuntimeError> {
    if !args.is_empty() {
        return Err(RuntimeError::InvalidOperation(format!(
            "Vec::new expects 0 arguments, got {}",
            args.len()
        )));
    }

    Ok(ScriptValue::Array(ScriptRc::new(ScriptVec::new())))
}

/// Get the length of a vector
pub(crate) fn vec_len_impl(args: &[ScriptValue]) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "len expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::Array(vec) => Ok(ScriptValue::I32(vec.len() as i32)),
        _ => Err(RuntimeError::InvalidOperation(
            "len expects a vector argument".to_string(),
        )),
    }
}

/// Push a value to a vector
pub(crate) fn vec_push_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "push expects 2 arguments, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::Array(vec) => {
            vec.push(args[1].clone())
                .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;
            Ok(ScriptValue::Unit)
        }
        _ => Err(RuntimeError::InvalidOperation(
            "push expects a vector as first argument".to_string(),
        )),
    }
}

/// Pop a value from a vector
pub(crate) fn vec_pop_impl(args: &[ScriptValue]) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "pop expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::Array(vec) => match vec
            .pop()
            .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?
        {
            Some(val) => Ok(ScriptValue::Option(ScriptRc::new(ScriptOption::some(val)))),
            None => Ok(ScriptValue::Option(ScriptRc::new(ScriptOption::none()))),
        },
        _ => Err(RuntimeError::InvalidOperation(
            "pop expects a vector argument".to_string(),
        )),
    }
}

/// Get a value from a vector at an index
pub(crate) fn vec_get_impl(args: &[ScriptValue]) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "get expects 2 arguments, got {}",
            args.len()
        )));
    }

    match (&args[0], &args[1]) {
        (ScriptValue::Array(vec), ScriptValue::I32(index)) => {
            if *index < 0 {
                return Err(RuntimeError::InvalidOperation(format!(
                    "Index cannot be negative: {}",
                    index
                )));
            }

            match vec
                .get(*index as usize)
                .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?
            {
                Some(val) => Ok(ScriptValue::Option(ScriptRc::new(ScriptOption::some(val)))),
                None => Ok(ScriptValue::Option(ScriptRc::new(ScriptOption::none()))),
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "get expects a vector and an integer index".to_string(),
        )),
    }
}

/// Create a new hash map
pub(crate) fn hashmap_new_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if !args.is_empty() {
        return Err(RuntimeError::InvalidOperation(format!(
            "HashMap::new expects 0 arguments, got {}",
            args.len()
        )));
    }

    Ok(ScriptValue::HashMap(ScriptRc::new(ScriptHashMap::new())))
}

/// Insert a key-value pair into a hash map
pub(crate) fn hashmap_insert_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(format!(
            "insert expects 3 arguments, got {}",
            args.len()
        )));
    }

    match (&args[0], &args[1]) {
        (ScriptValue::HashMap(map), ScriptValue::String(key)) => {
            match map
                .insert(key.as_str().to_string(), args[2].clone())
                .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?
            {
                Some(old_val) => Ok(ScriptValue::Option(ScriptRc::new(ScriptOption::some(
                    old_val,
                )))),
                None => Ok(ScriptValue::Option(ScriptRc::new(ScriptOption::none()))),
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "insert expects a HashMap and a string key".to_string(),
        )),
    }
}

/// Get a value from a hash map by key
pub(crate) fn hashmap_get_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "get expects 2 arguments, got {}",
            args.len()
        )));
    }

    match (&args[0], &args[1]) {
        (ScriptValue::HashMap(map), ScriptValue::String(key)) => match map
            .get(&key.as_str())
            .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?
        {
            Some(val) => Ok(ScriptValue::Option(ScriptRc::new(ScriptOption::some(val)))),
            None => Ok(ScriptValue::Option(ScriptRc::new(ScriptOption::none()))),
        },
        _ => Err(RuntimeError::InvalidOperation(
            "get expects a HashMap and a string key".to_string(),
        )),
    }
}

/// Check if a key exists in a hash map
pub(crate) fn hashmap_contains_key_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "contains_key expects 2 arguments, got {}",
            args.len()
        )));
    }

    match (&args[0], &args[1]) {
        (ScriptValue::HashMap(map), ScriptValue::String(key)) => {
            Ok(ScriptValue::Bool(map.contains_key(&key.as_str())))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "contains_key expects a HashMap and a string key".to_string(),
        )),
    }
}

/// A hash set for Script (implemented using HashMap)
#[derive(Debug, Clone)]
pub struct ScriptHashSet {
    /// The underlying hash map, using unit values
    data: ScriptHashMap,
}

impl ScriptHashSet {
    /// Create a new empty hash set
    pub fn new() -> Self {
        ScriptHashSet {
            data: ScriptHashMap::new(),
        }
    }

    /// Get the number of elements
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the set is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Insert a value into the set
    /// Returns true if the value was newly inserted, false if it already existed
    pub fn insert(&self, value: ScriptValue) -> crate::error::Result<bool> {
        let key = self.value_to_key(&value)?;
        let previous = self.data.insert(key, ScriptValue::Unit)?;
        Ok(previous.is_none())
    }

    /// Check if a value exists in the set
    pub fn contains(&self, value: &ScriptValue) -> crate::error::Result<bool> {
        let key = self.value_to_key(value)?;
        Ok(self.data.contains_key(&key))
    }

    /// Remove a value from the set
    /// Returns true if the value was removed, false if it didn't exist
    pub fn remove(&self, value: &ScriptValue) -> crate::error::Result<bool> {
        let key = self.value_to_key(value)?;
        let removed = self.data.remove(&key)?;
        Ok(removed.is_some())
    }

    /// Clear all elements from the set
    pub fn clear(&self) -> crate::error::Result<()> {
        self.data.clear()
    }

    /// Get all values as a vector
    pub fn values(&self) -> crate::error::Result<ScriptVec> {
        let vec = ScriptVec::new();
        let entries = self.data.iter()?;
        for (key, _) in entries {
            let value = self.key_to_value(&key)?;
            vec.push(value)?;
        }
        Ok(vec)
    }

    /// Check if this set is a subset of another set
    pub fn is_subset(&self, other: &ScriptHashSet) -> crate::error::Result<bool> {
        let my_values = self.values()?;
        let my_items = my_values.iter()?;

        for item in my_items {
            if !other.contains(&item)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Check if this set is a superset of another set
    pub fn is_superset(&self, other: &ScriptHashSet) -> crate::error::Result<bool> {
        other.is_subset(self)
    }

    /// Get the union of two sets
    pub fn union(&self, other: &ScriptHashSet) -> crate::error::Result<ScriptHashSet> {
        let result = ScriptHashSet::new();

        // Add all elements from self
        let my_values = self.values()?;
        let my_items = my_values.iter()?;
        for item in my_items {
            result.insert(item)?;
        }

        // Add all elements from other
        let other_values = other.values()?;
        let other_items = other_values.iter()?;
        for item in other_items {
            result.insert(item)?;
        }

        Ok(result)
    }

    /// Get the intersection of two sets
    pub fn intersection(&self, other: &ScriptHashSet) -> crate::error::Result<ScriptHashSet> {
        let result = ScriptHashSet::new();

        let my_values = self.values()?;
        let my_items = my_values.iter()?;
        for item in my_items {
            if other.contains(&item)? {
                result.insert(item)?;
            }
        }

        Ok(result)
    }

    /// Get the difference of two sets (elements in self but not in other)
    pub fn difference(&self, other: &ScriptHashSet) -> crate::error::Result<ScriptHashSet> {
        let result = ScriptHashSet::new();

        let my_values = self.values()?;
        let my_items = my_values.iter()?;
        for item in my_items {
            if !other.contains(&item)? {
                result.insert(item)?;
            }
        }

        Ok(result)
    }

    /// Get the symmetric difference of two sets (elements in either set but not both)
    pub fn symmetric_difference(
        &self,
        other: &ScriptHashSet,
    ) -> crate::error::Result<ScriptHashSet> {
        let my_diff = self.difference(other)?;
        let other_diff = other.difference(self)?;
        my_diff.union(&other_diff)
    }

    /// Convert a ScriptValue to a string key for hashing
    fn value_to_key(&self, value: &ScriptValue) -> crate::error::Result<String> {
        match value {
            ScriptValue::I32(i) => Ok(format!("i32:{}", i)),
            ScriptValue::F32(f) => Ok(format!("f32:{}", f)),
            ScriptValue::Bool(b) => Ok(format!("bool:{}", b)),
            ScriptValue::String(s) => Ok(format!("string:{}", s.as_str())),
            ScriptValue::Unit => Ok("unit".to_string()),
            _ => Err(crate::error::Error::type_error(format!(
                "HashSet can only contain hashable types (i32, f32, bool, string, unit), got {:?}",
                value.get_type()
            ))),
        }
    }

    /// Convert a string key back to a ScriptValue
    fn key_to_value(&self, key: &str) -> crate::error::Result<ScriptValue> {
        if key == "unit" {
            return Ok(ScriptValue::Unit);
        }

        if let Some(stripped) = key.strip_prefix("i32:") {
            let i = stripped
                .parse::<i32>()
                .map_err(|_| crate::error::Error::type_error("Invalid i32 in HashSet key"))?;
            return Ok(ScriptValue::I32(i));
        }

        if let Some(stripped) = key.strip_prefix("f32:") {
            let f = stripped
                .parse::<f32>()
                .map_err(|_| crate::error::Error::type_error("Invalid f32 in HashSet key"))?;
            return Ok(ScriptValue::F32(f));
        }

        if let Some(stripped) = key.strip_prefix("bool:") {
            let b = stripped
                .parse::<bool>()
                .map_err(|_| crate::error::Error::type_error("Invalid bool in HashSet key"))?;
            return Ok(ScriptValue::Bool(b));
        }

        if let Some(stripped) = key.strip_prefix("string:") {
            return Ok(ScriptValue::String(ScriptRc::new(ScriptString::from_str(stripped))));
        }

        Err(crate::error::Error::type_error(
            "Invalid HashSet key format",
        ))
    }

    /// Create an iterator over the set elements
    pub fn iter(&self) -> crate::error::Result<Vec<ScriptValue>> {
        self.values()?.iter()
    }
}

impl Default for ScriptHashSet {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for ScriptHashSet {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }

        // Check if all elements in self are in other
        match self.values() {
            Ok(values) => match values.iter() {
                Ok(items) => {
                    for item in items {
                        if !other.contains(&item).unwrap_or(false) {
                            return false;
                        }
                    }
                    true
                }
                Err(_) => false,
            },
            Err(_) => false,
        }
    }
}

// Implementation functions for stdlib registry

/// Create a new hash set
pub(crate) fn hashset_new_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if !args.is_empty() {
        return Err(RuntimeError::InvalidOperation(format!(
            "HashSet::new expects 0 arguments, got {}",
            args.len()
        )));
    }

    Ok(ScriptValue::HashSet(ScriptRc::new(ScriptHashSet::new())))
}

/// Insert a value into a hash set
pub(crate) fn hashset_insert_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "insert expects 2 arguments, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::HashSet(set) => {
            let was_new = set
                .insert(args[1].clone())
                .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;
            Ok(ScriptValue::Bool(was_new))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "insert expects a HashSet as first argument".to_string(),
        )),
    }
}

/// Check if a value exists in a hash set
pub(crate) fn hashset_contains_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "contains expects 2 arguments, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::HashSet(set) => {
            let contains = set
                .contains(&args[1])
                .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;
            Ok(ScriptValue::Bool(contains))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "contains expects a HashSet as first argument".to_string(),
        )),
    }
}

/// Remove a value from a hash set
pub(crate) fn hashset_remove_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "remove expects 2 arguments, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::HashSet(set) => {
            let was_removed = set
                .remove(&args[1])
                .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;
            Ok(ScriptValue::Bool(was_removed))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "remove expects a HashSet as first argument".to_string(),
        )),
    }
}

/// Get the size of a hash set
pub(crate) fn hashset_len_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "len expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::HashSet(set) => Ok(ScriptValue::I32(set.len() as i32)),
        _ => Err(RuntimeError::InvalidOperation(
            "len expects a HashSet argument".to_string(),
        )),
    }
}

/// Check if a hash set is empty
pub(crate) fn hashset_is_empty_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "is_empty expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::HashSet(set) => Ok(ScriptValue::Bool(set.is_empty())),
        _ => Err(RuntimeError::InvalidOperation(
            "is_empty expects a HashSet argument".to_string(),
        )),
    }
}

/// Get the union of two hash sets
pub(crate) fn hashset_union_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "union expects 2 arguments, got {}",
            args.len()
        )));
    }

    match (&args[0], &args[1]) {
        (ScriptValue::HashSet(set1), ScriptValue::HashSet(set2)) => {
            let union = set1
                .union(set2)
                .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;
            Ok(ScriptValue::HashSet(ScriptRc::new(union)))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "union expects two HashSet arguments".to_string(),
        )),
    }
}

/// Get the intersection of two hash sets
pub(crate) fn hashset_intersection_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "intersection expects 2 arguments, got {}",
            args.len()
        )));
    }

    match (&args[0], &args[1]) {
        (ScriptValue::HashSet(set1), ScriptValue::HashSet(set2)) => {
            let intersection = set1
                .intersection(set2)
                .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;
            Ok(ScriptValue::HashSet(ScriptRc::new(intersection)))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "intersection expects two HashSet arguments".to_string(),
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

        vec.push(ScriptValue::I32(10)).unwrap();
        vec.push(ScriptValue::I32(20)).unwrap();
        vec.push(ScriptValue::I32(30)).unwrap();

        assert_eq!(vec.len(), 3);
        assert!(!vec.is_empty());

        assert_eq!(vec.pop().unwrap(), Some(ScriptValue::I32(30)));
        assert_eq!(vec.pop().unwrap(), Some(ScriptValue::I32(20)));
        assert_eq!(vec.len(), 1);

        assert_eq!(vec.pop().unwrap(), Some(ScriptValue::I32(10)));
        assert_eq!(vec.pop().unwrap(), None);
        assert!(vec.is_empty());
    }

    #[test]
    fn test_vec_get_set() {
        let vec = ScriptVec::new();
        vec.push(ScriptValue::I32(10)).unwrap();
        vec.push(ScriptValue::I32(20)).unwrap();
        vec.push(ScriptValue::I32(30)).unwrap();

        assert_eq!(vec.get(0).unwrap(), Some(ScriptValue::I32(10)));
        assert_eq!(vec.get(1).unwrap(), Some(ScriptValue::I32(20)));
        assert_eq!(vec.get(2).unwrap(), Some(ScriptValue::I32(30)));
        assert_eq!(vec.get(3).unwrap(), None);

        assert!(vec.set(1, ScriptValue::I32(25)).unwrap());
        assert_eq!(vec.get(1).unwrap(), Some(ScriptValue::I32(25)));

        assert!(!vec.set(10, ScriptValue::I32(100)).unwrap()); // Out of bounds
    }

    #[test]
    fn test_vec_insert_remove() {
        let vec = ScriptVec::new();
        vec.push(ScriptValue::I32(10)).unwrap();
        vec.push(ScriptValue::I32(30)).unwrap();

        vec.insert(1, ScriptValue::I32(20)).unwrap();
        assert_eq!(vec.len(), 3);
        assert_eq!(vec.get(1).unwrap(), Some(ScriptValue::I32(20)));

        assert_eq!(vec.remove(1).unwrap(), Some(ScriptValue::I32(20)));
        assert_eq!(vec.len(), 2);
        assert_eq!(vec.get(1).unwrap(), Some(ScriptValue::I32(30)));

        assert_eq!(vec.remove(10).unwrap(), None); // Out of bounds
    }

    #[test]
    fn test_vec_methods() {
        let vec = ScriptVec::new();
        vec.push(ScriptValue::I32(10)).unwrap();
        vec.push(ScriptValue::I32(20)).unwrap();
        vec.push(ScriptValue::I32(30)).unwrap();

        assert_eq!(vec.first().unwrap(), Some(ScriptValue::I32(10)));
        assert_eq!(vec.last().unwrap(), Some(ScriptValue::I32(30)));

        assert!(vec.contains(&ScriptValue::I32(20)));
        assert!(!vec.contains(&ScriptValue::I32(40)));

        assert_eq!(vec.index_of(&ScriptValue::I32(20)).unwrap(), Some(1));
        assert_eq!(vec.index_of(&ScriptValue::I32(40)).unwrap(), None);

        vec.reverse().unwrap();
        assert_eq!(vec.get(0).unwrap(), Some(ScriptValue::I32(30)));
        assert_eq!(vec.get(2).unwrap(), Some(ScriptValue::I32(10)));
    }

    #[test]
    fn test_vec_sort() {
        let vec = ScriptVec::new();
        vec.push(ScriptValue::I32(30)).unwrap();
        vec.push(ScriptValue::I32(10)).unwrap();
        vec.push(ScriptValue::I32(20)).unwrap();

        assert!(vec.sort().is_ok());
        assert_eq!(vec.get(0).unwrap(), Some(ScriptValue::I32(10)));
        assert_eq!(vec.get(1).unwrap(), Some(ScriptValue::I32(20)));
        assert_eq!(vec.get(2).unwrap(), Some(ScriptValue::I32(30)));

        // Test sorting strings
        let str_vec = ScriptVec::new();
        str_vec
            .push(ScriptValue::String(ScriptRc::new(ScriptString::from_str(
                "charlie",
            ))))
            .unwrap();
        str_vec
            .push(ScriptValue::String(ScriptRc::new(ScriptString::from_str(
                "alice",
            ))))
            .unwrap();
        str_vec
            .push(ScriptValue::String(ScriptRc::new(ScriptString::from_str(
                "bob",
            ))))
            .unwrap();

        assert!(str_vec.sort().is_ok());
        assert_eq!(
            str_vec
                .get(0)
                .unwrap()
                .unwrap()
                .as_string()
                .unwrap()
                .as_str(),
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

        assert_eq!(
            map.insert("key1".to_string(), ScriptValue::I32(10))
                .unwrap(),
            None
        );
        assert_eq!(
            map.insert("key2".to_string(), ScriptValue::I32(20))
                .unwrap(),
            None
        );

        assert_eq!(map.len(), 2);
        assert!(!map.is_empty());

        assert_eq!(map.get("key1").unwrap(), Some(ScriptValue::I32(10)));
        assert_eq!(map.get("key2").unwrap(), Some(ScriptValue::I32(20)));
        assert_eq!(map.get("key3").unwrap(), None);

        // Test overwrite
        assert_eq!(
            map.insert("key1".to_string(), ScriptValue::I32(15))
                .unwrap(),
            Some(ScriptValue::I32(10))
        );
        assert_eq!(map.get("key1").unwrap(), Some(ScriptValue::I32(15)));
    }

    #[test]
    fn test_hashmap_remove() {
        let map = ScriptHashMap::new();
        map.insert("key1".to_string(), ScriptValue::I32(10))
            .unwrap();
        map.insert("key2".to_string(), ScriptValue::I32(20))
            .unwrap();

        assert_eq!(map.remove("key1").unwrap(), Some(ScriptValue::I32(10)));
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("key1").unwrap(), None);

        assert_eq!(map.remove("key3").unwrap(), None); // Non-existent key
    }

    #[test]
    fn test_hashmap_contains_key() {
        let map = ScriptHashMap::new();
        map.insert("key1".to_string(), ScriptValue::I32(10))
            .unwrap();

        assert!(map.contains_key("key1"));
        assert!(!map.contains_key("key2"));
    }

    #[test]
    fn test_hashmap_keys_values() {
        let map = ScriptHashMap::new();
        map.insert("a".to_string(), ScriptValue::I32(1)).unwrap();
        map.insert("b".to_string(), ScriptValue::I32(2)).unwrap();
        map.insert("c".to_string(), ScriptValue::I32(3)).unwrap();

        let keys = map.keys().unwrap();
        assert_eq!(keys.len(), 3);
        // Note: HashMap iteration order is not guaranteed

        let values = map.values().unwrap();
        assert_eq!(values.len(), 3);
    }

    #[test]
    fn test_hashmap_clear() {
        let map = ScriptHashMap::new();
        map.insert("key1".to_string(), ScriptValue::I32(10))
            .unwrap();
        map.insert("key2".to_string(), ScriptValue::I32(20))
            .unwrap();

        map.clear().unwrap();
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
        assert_eq!(map.get("key1").unwrap(), None);
    }

    #[test]
    fn test_hashset_creation() {
        let set = ScriptHashSet::new();
        assert_eq!(set.len(), 0);
        assert!(set.is_empty());
    }

    #[test]
    fn test_hashset_insert_contains() {
        let set = ScriptHashSet::new();

        // Test insertion
        assert!(set.insert(ScriptValue::I32(10)).unwrap());
        assert!(set.insert(ScriptValue::I32(20)).unwrap());
        assert!(!set.insert(ScriptValue::I32(10)).unwrap()); // Duplicate

        assert_eq!(set.len(), 2);
        assert!(!set.is_empty());

        // Test contains
        assert!(set.contains(&ScriptValue::I32(10)).unwrap());
        assert!(set.contains(&ScriptValue::I32(20)).unwrap());
        assert!(!set.contains(&ScriptValue::I32(30)).unwrap());
    }

    #[test]
    fn test_hashset_remove() {
        let set = ScriptHashSet::new();
        set.insert(ScriptValue::I32(10)).unwrap();
        set.insert(ScriptValue::I32(20)).unwrap();

        assert!(set.remove(&ScriptValue::I32(10)).unwrap());
        assert_eq!(set.len(), 1);
        assert!(!set.contains(&ScriptValue::I32(10)).unwrap());

        assert!(!set.remove(&ScriptValue::I32(30)).unwrap()); // Non-existent
    }

    #[test]
    fn test_hashset_clear() {
        let set = ScriptHashSet::new();
        set.insert(ScriptValue::I32(10)).unwrap();
        set.insert(ScriptValue::I32(20)).unwrap();

        set.clear().unwrap();
        assert_eq!(set.len(), 0);
        assert!(set.is_empty());
    }

    #[test]
    fn test_hashset_union() {
        let set1 = ScriptHashSet::new();
        set1.insert(ScriptValue::I32(1)).unwrap();
        set1.insert(ScriptValue::I32(2)).unwrap();

        let set2 = ScriptHashSet::new();
        set2.insert(ScriptValue::I32(2)).unwrap();
        set2.insert(ScriptValue::I32(3)).unwrap();

        let union = set1.union(&set2).unwrap();
        assert_eq!(union.len(), 3);
        assert!(union.contains(&ScriptValue::I32(1)).unwrap());
        assert!(union.contains(&ScriptValue::I32(2)).unwrap());
        assert!(union.contains(&ScriptValue::I32(3)).unwrap());
    }

    #[test]
    fn test_hashset_intersection() {
        let set1 = ScriptHashSet::new();
        set1.insert(ScriptValue::I32(1)).unwrap();
        set1.insert(ScriptValue::I32(2)).unwrap();

        let set2 = ScriptHashSet::new();
        set2.insert(ScriptValue::I32(2)).unwrap();
        set2.insert(ScriptValue::I32(3)).unwrap();

        let intersection = set1.intersection(&set2).unwrap();
        assert_eq!(intersection.len(), 1);
        assert!(intersection.contains(&ScriptValue::I32(2)).unwrap());
        assert!(!intersection.contains(&ScriptValue::I32(1)).unwrap());
        assert!(!intersection.contains(&ScriptValue::I32(3)).unwrap());
    }

    #[test]
    fn test_hashset_difference() {
        let set1 = ScriptHashSet::new();
        set1.insert(ScriptValue::I32(1)).unwrap();
        set1.insert(ScriptValue::I32(2)).unwrap();

        let set2 = ScriptHashSet::new();
        set2.insert(ScriptValue::I32(2)).unwrap();
        set2.insert(ScriptValue::I32(3)).unwrap();

        let difference = set1.difference(&set2).unwrap();
        assert_eq!(difference.len(), 1);
        assert!(difference.contains(&ScriptValue::I32(1)).unwrap());
        assert!(!difference.contains(&ScriptValue::I32(2)).unwrap());
        assert!(!difference.contains(&ScriptValue::I32(3)).unwrap());
    }

    #[test]
    fn test_hashset_subset_superset() {
        let set1 = ScriptHashSet::new();
        set1.insert(ScriptValue::I32(1)).unwrap();
        set1.insert(ScriptValue::I32(2)).unwrap();

        let set2 = ScriptHashSet::new();
        set2.insert(ScriptValue::I32(1)).unwrap();
        set2.insert(ScriptValue::I32(2)).unwrap();
        set2.insert(ScriptValue::I32(3)).unwrap();

        assert!(set1.is_subset(&set2).unwrap());
        assert!(!set2.is_subset(&set1).unwrap());
        assert!(set2.is_superset(&set1).unwrap());
        assert!(!set1.is_superset(&set2).unwrap());
    }

    #[test]
    fn test_hashset_equality() {
        let set1 = ScriptHashSet::new();
        set1.insert(ScriptValue::I32(1)).unwrap();
        set1.insert(ScriptValue::I32(2)).unwrap();

        let set2 = ScriptHashSet::new();
        set2.insert(ScriptValue::I32(2)).unwrap();
        set2.insert(ScriptValue::I32(1)).unwrap();

        assert_eq!(set1, set2);
    }

    #[test]
    fn test_hashset_different_types() {
        let set = ScriptHashSet::new();
        set.insert(ScriptValue::I32(42)).unwrap();
        set.insert(ScriptValue::F32(3.14)).unwrap();
        set.insert(ScriptValue::Bool(true)).unwrap();
        set.insert(ScriptValue::String(ScriptRc::new(ScriptString::from_str(
            "hello",
        ))))
        .unwrap();
        set.insert(ScriptValue::Unit).unwrap();

        assert_eq!(set.len(), 5);
        assert!(set.contains(&ScriptValue::I32(42)).unwrap());
        assert!(set.contains(&ScriptValue::F32(3.14)).unwrap());
        assert!(set.contains(&ScriptValue::Bool(true)).unwrap());
        assert!(set
            .contains(&ScriptValue::String(ScriptRc::new(ScriptString::from_str(
                "hello"
            ))))
            .unwrap());
        assert!(set.contains(&ScriptValue::Unit).unwrap());
    }
}
