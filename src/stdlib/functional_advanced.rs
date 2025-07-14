use crate::runtime::closure::Closure;
use crate::runtime::{RuntimeError, ScriptRc};
use crate::stdlib::string::ScriptString;
use crate::stdlib::{ScriptValue, ScriptVec};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Transducer: A composable transformation that can be applied to collections
#[derive(Debug, Clone)]
pub struct Transducer {
    /// The transformation function
    transform: ScriptRc<Closure>,
    /// Composed transformations
    composed: Vec<ScriptRc<Closure>>,
}

impl Transducer {
    /// Create a new transducer from a transformation function
    pub fn new(transform: ScriptRc<Closure>) -> Self {
        Self {
            transform,
            composed: Vec::new(),
        }
    }

    /// Compose this transducer with another
    pub fn compose(&mut self, other: Transducer) {
        self.composed.push(other.transform);
        self.composed.extend(other.composed);
    }

    /// Apply the transducer to a collection
    pub fn apply(&self, collection: &ScriptVec) -> Result<ScriptVec, RuntimeError> {
        let mut result = collection.clone();

        // Apply the main transformation
        result = self.apply_single_transform(&result, &self.transform)?;

        // Apply composed transformations
        for transform in &self.composed {
            result = self.apply_single_transform(&result, transform)?;
        }

        Ok(result)
    }

    /// Apply a single transformation
    fn apply_single_transform(
        &self,
        collection: &ScriptVec,
        _transform: &ScriptRc<Closure>,
    ) -> Result<ScriptVec, RuntimeError> {
        // In a real implementation, this would execute the closure on each element
        // For now, return a clone of the collection
        Ok(collection.clone())
    }
}

/// Lazy sequence for deferred computation
#[derive(Debug)]
pub struct LazySeq {
    /// Generator function that produces the next value
    generator: ScriptRc<Closure>,
    /// Cached values
    cache: Arc<RwLock<Vec<ScriptValue>>>,
    /// Current index
    current_index: Arc<RwLock<usize>>,
}

impl LazySeq {
    /// Create a new lazy sequence
    pub fn new(generator: ScriptRc<Closure>) -> Self {
        Self {
            generator,
            cache: Arc::new(RwLock::new(Vec::new())),
            current_index: Arc::new(RwLock::new(0)),
        }
    }

    /// Get the next value from the sequence
    pub fn next(&self) -> Option<ScriptValue> {
        let mut index = self.current_index.write().unwrap();
        let cache = self.cache.read().unwrap();

        if *index < cache.len() {
            let value = cache[*index].clone();
            *index += 1;
            Some(value)
        } else {
            // Generate new value
            // In a real implementation, this would call the generator closure
            None
        }
    }

    /// Take n elements from the sequence
    pub fn take(&self, n: usize) -> Vec<ScriptValue> {
        let mut result = Vec::with_capacity(n);
        for _ in 0..n {
            if let Some(value) = self.next() {
                result.push(value);
            } else {
                break;
            }
        }
        result
    }

    /// Force evaluation of the entire sequence (if finite)
    pub fn force(&self, limit: usize) -> Vec<ScriptValue> {
        self.take(limit)
    }
}

/// Advanced memoization with TTL (Time To Live)
#[derive(Debug)]
pub struct MemoizationCache {
    /// Cache entries with expiration times
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// Default TTL for cache entries
    default_ttl: Duration,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    value: ScriptValue,
    expiration: Instant,
}

impl MemoizationCache {
    /// Create a new memoization cache with default TTL
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            default_ttl,
        }
    }

    /// Get a value from the cache
    pub fn get(&self, key: &str) -> Option<ScriptValue> {
        let cache = self.cache.read().unwrap();
        if let Some(entry) = cache.get(key) {
            if entry.expiration > Instant::now() {
                Some(entry.value.clone())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Put a value in the cache with default TTL
    pub fn put(&self, key: String, value: ScriptValue) {
        self.put_with_ttl(key, value, self.default_ttl);
    }

    /// Put a value in the cache with custom TTL
    pub fn put_with_ttl(&self, key: String, value: ScriptValue, ttl: Duration) {
        let mut cache = self.cache.write().unwrap();
        cache.insert(
            key,
            CacheEntry {
                value,
                expiration: Instant::now() + ttl,
            },
        );
    }

    /// Clear expired entries
    pub fn cleanup(&self) {
        let mut cache = self.cache.write().unwrap();
        let now = Instant::now();
        cache.retain(|_, entry| entry.expiration > now);
    }

    /// Clear all entries
    pub fn clear(&self) {
        self.cache.write().unwrap().clear();
    }
}

/// Create a memoized version of a closure with TTL
pub fn memoize_with_ttl(closure: ScriptRc<Closure>, ttl: Duration) -> ScriptRc<Closure> {
    let _cache = MemoizationCache::new(ttl);

    // In a real implementation, this would create a new closure that:
    // 1. Checks the cache for the result
    // 2. If not found, calls the original closure
    // 3. Stores the result in the cache
    // 4. Returns the result

    // For now, return the original closure
    closure
}

/// Standard library function implementations

/// Implementation of transduce for stdlib registry
pub(crate) fn transduce_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(format!(
            "transduce expects 3 arguments (transducer, collection, initial), got {}",
            args.len()
        )));
    }

    // For now, return the collection
    Ok(args[1].clone())
}

/// Implementation of lazy_seq for stdlib registry
pub(crate) fn lazy_seq_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "lazy_seq expects 1 argument (generator closure), got {}",
            args.len()
        )));
    }

    let generator = match &args[0] {
        ScriptValue::Closure(c) => c.clone(),
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "Argument to lazy_seq must be a closure".to_string(),
            ))
        }
    };

    // Create lazy sequence
    let _lazy_seq = LazySeq::new(generator);

    // Return a placeholder
    Ok(ScriptValue::String(ScriptRc::new(ScriptString::from(
        "LazySeq",
    ))))
}

/// Implementation of memoize_with_ttl for stdlib registry
pub(crate) fn memoize_with_ttl_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "memoize_with_ttl expects 2 arguments (closure, ttl_ms), got {}",
            args.len()
        )));
    }

    let closure = match &args[0] {
        ScriptValue::Closure(c) => c.clone(),
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "First argument to memoize_with_ttl must be a closure".to_string(),
            ))
        }
    };

    let ttl_ms = match &args[1] {
        ScriptValue::I32(ms) => *ms,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "Second argument to memoize_with_ttl must be an integer (milliseconds)".to_string(),
            ))
        }
    };

    if ttl_ms < 0 {
        return Err(RuntimeError::InvalidOperation(
            "TTL cannot be negative".to_string(),
        ));
    }

    let ttl = Duration::from_millis(ttl_ms as u64);
    let memoized = memoize_with_ttl(closure, ttl);

    Ok(ScriptValue::Closure(memoized))
}

/// Implementation of lazy_take for stdlib registry
pub(crate) fn lazy_take_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "lazy_take expects 2 arguments (lazy_seq, count), got {}",
            args.len()
        )));
    }

    let count = match &args[1] {
        ScriptValue::I32(n) => *n,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "Second argument to lazy_take must be an integer".to_string(),
            ))
        }
    };

    if count < 0 {
        return Err(RuntimeError::InvalidOperation(
            "Count cannot be negative".to_string(),
        ));
    }

    // For now, return an empty array
    Ok(ScriptValue::Array(ScriptRc::new(ScriptVec::new())))
}

/// Implementation of lazy_force for stdlib registry
pub(crate) fn lazy_force_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "lazy_force expects 2 arguments (lazy_seq, limit), got {}",
            args.len()
        )));
    }

    let limit = match &args[1] {
        ScriptValue::I32(n) => *n,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "Second argument to lazy_force must be an integer".to_string(),
            ))
        }
    };

    if limit < 0 {
        return Err(RuntimeError::InvalidOperation(
            "Limit cannot be negative".to_string(),
        ));
    }

    // For now, return an empty array
    Ok(ScriptValue::Array(ScriptRc::new(ScriptVec::new())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memoization_cache() {
        let cache = MemoizationCache::new(Duration::from_secs(60));

        // Test put and get
        cache.put("test".to_string(), ScriptValue::I32(42));
        assert!(matches!(cache.get("test"), Some(ScriptValue::I32(42))));

        // Test missing key
        assert!(cache.get("missing").is_none());

        // Test clear
        cache.clear();
        assert!(cache.get("test").is_none());
    }

    #[test]
    fn test_lazy_seq_creation() {
        let closure = ScriptRc::new(Closure::new(
            "generator".to_string(),
            vec![],
            HashMap::new(),
        ));
        let lazy_seq = LazySeq::new(closure);

        // Test that we can create a lazy sequence
        let values = lazy_seq.take(0);
        assert_eq!(values.len(), 0);
    }

    #[test]
    fn test_transducer_creation() {
        let transform = ScriptRc::new(Closure::new(
            "transform".to_string(),
            vec![],
            HashMap::new(),
        ));
        let transducer = Transducer::new(transform);

        assert_eq!(transducer.composed.len(), 0);
    }
}
