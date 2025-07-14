use ahash::AHashMap;
use std::collections::VecDeque;
use std::hash::Hash;

/// A simple LRU (Least Recently Used) cache with size limit
#[derive(Debug)]
pub struct LruCache<K: Hash + Eq + Clone, V: Clone> {
    capacity: usize,
    map: AHashMap<K, V>,
    order: VecDeque<K>,
}

impl<K: Hash + Eq + Clone, V: Clone> LruCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            map: AHashMap::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
        }
    }

    pub fn get(&mut self, key: &K) -> Option<V> {
        if let Some(value) = self.map.get(key) {
            // Move to front
            self.order.retain(|k| k != key);
            self.order.push_front(key.clone());
            Some(value.clone())
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        if self.map.contains_key(&key) {
            // Update existing and move to front
            self.map.insert(key.clone(), value);
            self.order.retain(|k| k != &key);
            self.order.push_front(key);
        } else {
            // Check capacity
            if self.map.len() >= self.capacity {
                // Remove least recently used
                if let Some(old_key) = self.order.pop_back() {
                    self.map.remove(&old_key);
                }
            }
            self.map.insert(key.clone(), value);
            self.order.push_front(key);
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.map.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_basic() {
        let mut cache = LruCache::new(2);
        cache.insert("a", 1);
        cache.insert("b", 2);

        assert_eq!(cache.get(&"a"), Some(1));
        assert_eq!(cache.get(&"b"), Some(2));

        // This should evict "a" since "b" was accessed more recently
        cache.insert("c", 3);
        assert_eq!(cache.get(&"a"), None);
        assert_eq!(cache.get(&"b"), Some(2));
        assert_eq!(cache.get(&"c"), Some(3));
    }

    #[test]
    fn test_lru_update() {
        let mut cache = LruCache::new(2);
        cache.insert("a", 1);
        cache.insert("b", 2);

        // Update "a" - should not increase size
        cache.insert("a", 10);
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get(&"a"), Some(10));
    }
}
