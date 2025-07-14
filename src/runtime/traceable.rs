//! Traceable trait for cycle detection
//!
//! This module provides the `Traceable` trait which allows objects to
//! participate in cycle detection by reporting their outgoing references.

use super::rc::ScriptRc;
use std::any::Any;

/// A trait for types that can contain references to other objects
///
/// Types implementing this trait can participate in cycle detection
/// by reporting their outgoing references to the garbage collector.
pub trait Traceable {
    /// Visit all ScriptRc references contained in this object
    ///
    /// The visitor function is called once for each ScriptRc field.
    /// This allows the GC to build the object graph for cycle detection.
    fn trace(&self, visitor: &mut dyn FnMut(&dyn Any));

    /// Get the size of this object for memory accounting
    fn trace_size(&self) -> usize {
        std::mem::size_of_val(self)
    }
}

/// Helper macro to implement Traceable for types with no references
#[macro_export]
macro_rules! impl_traceable_empty {
    ($($t:ty),*) => {
        $(
            impl Traceable for $t {
                fn trace(&self, _visitor: &mut dyn FnMut(&dyn Any)) {
                    // No references to trace
                }
            }
        )*
    };
}

// Implement Traceable for primitive types that contain no references
impl_traceable_empty!(
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    f32,
    f64,
    bool,
    char,
    (),
    String
);

// Implement Traceable for Option<T>
impl<T: Traceable + 'static> Traceable for Option<T> {
    fn trace(&self, visitor: &mut dyn FnMut(&dyn Any)) {
        if let Some(inner) = self {
            inner.trace(visitor);
        }
    }
}

// Implement Traceable for Vec<T>
impl<T: Traceable + 'static> Traceable for Vec<T> {
    fn trace(&self, visitor: &mut dyn FnMut(&dyn Any)) {
        for item in self {
            item.trace(visitor);
        }
    }
}

// Implement Traceable for ScriptRc<T>
impl<T: Traceable + ?Sized + 'static> Traceable for ScriptRc<T> {
    fn trace(&self, visitor: &mut dyn FnMut(&dyn Any)) {
        // Report this reference to the visitor
        visitor(self as &dyn Any);

        // Also trace the contents
        (**self).trace(visitor);
    }
}

// Implement Traceable for tuples
impl<A: Traceable + 'static, B: Traceable + 'static> Traceable for (A, B) {
    fn trace(&self, visitor: &mut dyn FnMut(&dyn Any)) {
        self.0.trace(visitor);
        self.1.trace(visitor);
    }
}

impl<A: Traceable + 'static, B: Traceable + 'static, C: Traceable + 'static> Traceable
    for (A, B, C)
{
    fn trace(&self, visitor: &mut dyn FnMut(&dyn Any)) {
        self.0.trace(visitor);
        self.1.trace(visitor);
        self.2.trace(visitor);
    }
}

/// A helper trait to extract ScriptRc from Any
pub trait AsScriptRc {
    /// Try to downcast to a ScriptRc
    fn as_script_rc(&self) -> Option<usize>;
}

impl<T: ?Sized + 'static> AsScriptRc for ScriptRc<T> {
    fn as_script_rc(&self) -> Option<usize> {
        Some(self.as_raw() as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestNode {
        value: i32,
        next: Option<ScriptRc<TestNode>>,
    }

    impl Traceable for TestNode {
        fn trace(&self, visitor: &mut dyn FnMut(&dyn Any)) {
            self.next.trace(visitor);
        }
    }

    #[test]
    fn test_trace_simple() {
        let node = TestNode {
            value: 42,
            next: None,
        };

        let mut count = 0;
        node.trace(&mut |_| count += 1);
        assert_eq!(count, 0); // No references
    }

    #[test]
    fn test_trace_with_reference() {
        let node1 = ScriptRc::new(TestNode {
            value: 1,
            next: None,
        });

        let node2 = TestNode {
            value: 2,
            next: Some(node1),
        };

        let mut count = 0;
        node2.trace(&mut |_| count += 1);
        assert_eq!(count, 1); // One reference
    }
}
