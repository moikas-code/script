//! Iterator support for the Script standard library
//!
//! This module provides iterator traits and implementations for lazy evaluation
//! and functional programming patterns in Script.

use crate::runtime::RuntimeError;
use crate::stdlib::ScriptValue;
use std::fmt::Debug;

/// Trait for Script iterators
pub trait ScriptIterator: Debug + Send + Sync {
    /// Get the next value from the iterator
    fn next(&mut self) -> Option<ScriptValue>;

    /// Check if the iterator has more elements
    fn has_next(&self) -> bool;

    /// Get the size hint for the iterator
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }

    /// Clone the iterator (if possible)
    fn clone_box(&self) -> Box<dyn ScriptIterator>;
}

/// Range iterator implementation
#[derive(Debug, Clone)]
pub struct RangeIterator {
    start: i32,
    end: i32,
    step: i32,
    current: i32,
}

impl RangeIterator {
    /// Create a new range iterator
    pub fn new(start: i32, end: i32, step: i32) -> Result<Self, RuntimeError> {
        if step == 0 {
            return Err(RuntimeError::InvalidOperation(
                "Range step cannot be zero".to_string(),
            ));
        }

        // Validate that the range makes sense
        if step > 0 && start > end {
            return Err(RuntimeError::InvalidOperation(
                "Invalid range: start > end with positive step".to_string(),
            ));
        }
        if step < 0 && start < end {
            return Err(RuntimeError::InvalidOperation(
                "Invalid range: start < end with negative step".to_string(),
            ));
        }

        Ok(RangeIterator {
            start,
            end,
            step,
            current: start,
        })
    }
}

impl ScriptIterator for RangeIterator {
    fn next(&mut self) -> Option<ScriptValue> {
        if self.step > 0 {
            if self.current >= self.end {
                return None;
            }
        } else {
            if self.current <= self.end {
                return None;
            }
        }

        let value = self.current;
        self.current += self.step;
        Some(ScriptValue::I32(value))
    }

    fn has_next(&self) -> bool {
        if self.step > 0 {
            self.current < self.end
        } else {
            self.current > self.end
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.step == 0 {
            return (0, None);
        }

        let diff = if self.step > 0 {
            self.end - self.current
        } else {
            self.current - self.end
        };

        if diff <= 0 {
            return (0, Some(0));
        }

        let count = (diff.abs() as usize) / (self.step.abs() as usize);
        (count, Some(count))
    }

    fn clone_box(&self) -> Box<dyn ScriptIterator> {
        Box::new(self.clone())
    }
}

/// Vector iterator implementation
#[derive(Debug)]
pub struct VecIterator {
    data: Vec<ScriptValue>,
    index: usize,
}

impl VecIterator {
    /// Create a new vector iterator
    pub fn new(data: Vec<ScriptValue>) -> Self {
        VecIterator { data, index: 0 }
    }
}

impl ScriptIterator for VecIterator {
    fn next(&mut self) -> Option<ScriptValue> {
        if self.index < self.data.len() {
            let value = self.data[self.index].clone();
            self.index += 1;
            Some(value)
        } else {
            None
        }
    }

    fn has_next(&self) -> bool {
        self.index < self.data.len()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.data.len() - self.index;
        (remaining, Some(remaining))
    }

    fn clone_box(&self) -> Box<dyn ScriptIterator> {
        Box::new(VecIterator {
            data: self.data.clone(),
            index: self.index,
        })
    }
}

/// Map iterator that applies a function to each element
#[derive(Debug)]
pub struct MapIterator {
    inner: Box<dyn ScriptIterator>,
    // In a full implementation, this would hold a closure
    // For now, we'll use a placeholder
}

impl MapIterator {
    /// Create a new map iterator
    pub fn new(inner: Box<dyn ScriptIterator>) -> Self {
        MapIterator { inner }
    }
}

impl ScriptIterator for MapIterator {
    fn next(&mut self) -> Option<ScriptValue> {
        // In a full implementation, this would apply the mapping function
        self.inner.next()
    }

    fn has_next(&self) -> bool {
        self.inner.has_next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    fn clone_box(&self) -> Box<dyn ScriptIterator> {
        Box::new(MapIterator {
            inner: self.inner.clone_box(),
        })
    }
}

/// Filter iterator that filters elements based on a predicate
#[derive(Debug)]
pub struct FilterIterator {
    inner: Box<dyn ScriptIterator>,
    // In a full implementation, this would hold a predicate closure
}

impl FilterIterator {
    /// Create a new filter iterator
    pub fn new(inner: Box<dyn ScriptIterator>) -> Self {
        FilterIterator { inner }
    }
}

impl ScriptIterator for FilterIterator {
    fn next(&mut self) -> Option<ScriptValue> {
        // In a full implementation, this would apply the filter predicate
        self.inner.next()
    }

    fn has_next(&self) -> bool {
        self.inner.has_next()
    }

    fn clone_box(&self) -> Box<dyn ScriptIterator> {
        Box::new(FilterIterator {
            inner: self.inner.clone_box(),
        })
    }
}

/// Take iterator that limits the number of elements
#[derive(Debug)]
pub struct TakeIterator {
    inner: Box<dyn ScriptIterator>,
    remaining: usize,
}

impl TakeIterator {
    /// Create a new take iterator
    pub fn new(inner: Box<dyn ScriptIterator>, count: usize) -> Self {
        TakeIterator {
            inner,
            remaining: count,
        }
    }
}

impl ScriptIterator for TakeIterator {
    fn next(&mut self) -> Option<ScriptValue> {
        if self.remaining > 0 {
            self.remaining -= 1;
            self.inner.next()
        } else {
            None
        }
    }

    fn has_next(&self) -> bool {
        self.remaining > 0 && self.inner.has_next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (inner_lower, inner_upper) = self.inner.size_hint();
        let lower = inner_lower.min(self.remaining);
        let upper = inner_upper
            .map(|u| u.min(self.remaining))
            .or(Some(self.remaining));
        (lower, upper)
    }

    fn clone_box(&self) -> Box<dyn ScriptIterator> {
        Box::new(TakeIterator {
            inner: self.inner.clone_box(),
            remaining: self.remaining,
        })
    }
}

/// Skip iterator that skips a number of elements
#[derive(Debug)]
pub struct SkipIterator {
    inner: Box<dyn ScriptIterator>,
    to_skip: usize,
}

impl SkipIterator {
    /// Create a new skip iterator
    pub fn new(inner: Box<dyn ScriptIterator>, count: usize) -> Self {
        SkipIterator {
            inner,
            to_skip: count,
        }
    }
}

impl ScriptIterator for SkipIterator {
    fn next(&mut self) -> Option<ScriptValue> {
        // Skip the required number of elements
        while self.to_skip > 0 {
            self.inner.next()?;
            self.to_skip -= 1;
        }
        self.inner.next()
    }

    fn has_next(&self) -> bool {
        self.inner.has_next()
    }

    fn clone_box(&self) -> Box<dyn ScriptIterator> {
        Box::new(SkipIterator {
            inner: self.inner.clone_box(),
            to_skip: self.to_skip,
        })
    }
}

/// Generator support for creating custom iterators
pub struct Generators;

impl Generators {
    /// Create a range iterator
    pub fn range(start: i32, end: i32, step: i32) -> Result<Box<dyn ScriptIterator>, RuntimeError> {
        Ok(Box::new(RangeIterator::new(start, end, step)?))
    }

    /// Create an iterator from a vector
    pub fn from_vec(data: Vec<ScriptValue>) -> Box<dyn ScriptIterator> {
        Box::new(VecIterator::new(data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_iterator() {
        let mut iter = RangeIterator::new(0, 5, 1).unwrap();
        assert_eq!(iter.next(), Some(ScriptValue::I32(0)));
        assert_eq!(iter.next(), Some(ScriptValue::I32(1)));
        assert_eq!(iter.next(), Some(ScriptValue::I32(2)));
        assert_eq!(iter.next(), Some(ScriptValue::I32(3)));
        assert_eq!(iter.next(), Some(ScriptValue::I32(4)));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_range_iterator_negative_step() {
        let mut iter = RangeIterator::new(5, 0, -1).unwrap();
        assert_eq!(iter.next(), Some(ScriptValue::I32(5)));
        assert_eq!(iter.next(), Some(ScriptValue::I32(4)));
        assert_eq!(iter.next(), Some(ScriptValue::I32(3)));
        assert_eq!(iter.next(), Some(ScriptValue::I32(2)));
        assert_eq!(iter.next(), Some(ScriptValue::I32(1)));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_vec_iterator() {
        let data = vec![
            ScriptValue::I32(1),
            ScriptValue::I32(2),
            ScriptValue::I32(3),
        ];
        let mut iter = VecIterator::new(data);
        assert_eq!(iter.next(), Some(ScriptValue::I32(1)));
        assert_eq!(iter.next(), Some(ScriptValue::I32(2)));
        assert_eq!(iter.next(), Some(ScriptValue::I32(3)));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_take_iterator() {
        let data = vec![
            ScriptValue::I32(1),
            ScriptValue::I32(2),
            ScriptValue::I32(3),
            ScriptValue::I32(4),
            ScriptValue::I32(5),
        ];
        let inner = Box::new(VecIterator::new(data));
        let mut iter = TakeIterator::new(inner, 3);
        assert_eq!(iter.next(), Some(ScriptValue::I32(1)));
        assert_eq!(iter.next(), Some(ScriptValue::I32(2)));
        assert_eq!(iter.next(), Some(ScriptValue::I32(3)));
        assert_eq!(iter.next(), None);
    }
}
