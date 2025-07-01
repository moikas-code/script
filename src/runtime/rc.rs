//! Reference counting smart pointer implementation for Script
//! 
//! This module provides `ScriptRc<T>` and `ScriptWeak<T>`, which are similar to
//! Rust's `Rc<T>` and `Weak<T>` but adapted for Script's needs:
//! - Integration with cycle detection
//! - Memory profiling hooks
//! - Thread-safe for future actor model support
//! - Panic-safe with proper cleanup

use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::fmt;
use std::ops::Deref;
use std::hash::{Hash, Hasher};

use crate::runtime::profiler;
use crate::runtime::gc;

/// Internal reference count structure
struct RcBox<T: ?Sized> {
    /// Number of strong references
    strong: AtomicUsize,
    /// Number of weak references
    weak: AtomicUsize,
    /// The actual value
    value: T,
}

/// A reference-counted smart pointer for Script values
/// 
/// This is the primary way to share values in Script. It provides:
/// - Automatic memory management through reference counting
/// - Integration with cycle detection for breaking reference cycles
/// - Memory profiling support
pub struct ScriptRc<T: ?Sized> {
    ptr: NonNull<RcBox<T>>,
    phantom: PhantomData<RcBox<T>>,
}

/// A weak reference to a `ScriptRc<T>`
/// 
/// Weak references don't prevent the value from being deallocated.
/// They can be upgraded to strong references if the value still exists.
pub struct ScriptWeak<T> {
    ptr: NonNull<RcBox<T>>,
    phantom: PhantomData<RcBox<T>>,
}

impl<T> ScriptRc<T> {
    /// Create a new reference-counted value
    pub fn new(value: T) -> Self {
        let layout = std::alloc::Layout::new::<RcBox<T>>();
        
        // Notify profiler of allocation
        profiler::record_allocation(layout.size(), std::any::type_name::<T>());
        
        // Allocate memory
        unsafe {
            let ptr = std::alloc::alloc(layout) as *mut RcBox<T>;
            if ptr.is_null() {
                std::alloc::handle_alloc_error(layout);
            }
            
            // Initialize the RcBox
            ptr.write(RcBox {
                strong: AtomicUsize::new(1),
                weak: AtomicUsize::new(1), // +1 for the strong reference
                value,
            });
            
            let rc = ScriptRc {
                ptr: NonNull::new_unchecked(ptr),
                phantom: PhantomData,
            };
            
            // Register with GC for cycle detection
            gc::register_rc(&rc);
            
            rc
        }
    }
    
    /// Get the number of strong references
    pub fn strong_count(&self) -> usize {
        unsafe { self.ptr.as_ref().strong.load(Ordering::SeqCst) }
    }
    
    /// Get the number of weak references
    /// 
    /// Note: This includes the implicit weak reference held by strong references
    pub fn weak_count(&self) -> usize {
        unsafe { self.ptr.as_ref().weak.load(Ordering::SeqCst) }
    }
    
    /// Create a new weak reference
    pub fn downgrade(&self) -> ScriptWeak<T> {
        unsafe { self.ptr.as_ref().weak.fetch_add(1, Ordering::Relaxed); }
        ScriptWeak {
            ptr: self.ptr,
            phantom: PhantomData,
        }
    }
    
    /// Try to get a mutable reference if there's only one strong reference
    /// 
    /// Returns `None` if there are multiple strong references or any weak references
    /// (excluding the implicit weak reference from the single strong reference).
    pub fn get_mut(&mut self) -> Option<&mut T> {
        if self.strong_count() == 1 && self.weak_count() == 1 {
            unsafe { Some(&mut (*self.ptr.as_ptr()).value) }
        } else {
            None
        }
    }
    
    /// Returns true if two `ScriptRc`s point to the same allocation
    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        this.ptr.as_ptr() == other.ptr.as_ptr()
    }
    
    /// Make a mutable reference if there's only one strong reference
    /// 
    /// If there are multiple strong references, this will clone the data.
    pub fn make_mut(&mut self) -> &mut T 
    where 
        T: Clone,
    {
        if self.strong_count() != 1 {
            // Clone the data
            let cloned = (**self).clone();
            *self = ScriptRc::new(cloned);
        }
        
        // Now we definitely have unique access
        unsafe { &mut (*self.ptr.as_ptr()).value }
    }
    
    /// Get the inner RcBox
    #[allow(dead_code)]
    fn inner(&self) -> &RcBox<T> {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: ?Sized> Clone for ScriptRc<T> {
    fn clone(&self) -> Self {
        unsafe { self.ptr.as_ref().strong.fetch_add(1, Ordering::Relaxed); }
        ScriptRc {
            ptr: self.ptr,
            phantom: PhantomData,
        }
    }
}

impl<T: ?Sized> Drop for ScriptRc<T> {
    fn drop(&mut self) {
        unsafe {
            if self.ptr.as_ref().strong.fetch_sub(1, Ordering::Release) == 1 {
                // This was the last strong reference
                std::sync::atomic::fence(Ordering::Acquire);
                
                // Unregister from GC
                gc::unregister_rc(self);
                
                // Drop the value
                std::ptr::drop_in_place(&mut (*self.ptr.as_ptr()).value);
                
                // Decrement the weak count
                if self.ptr.as_ref().weak.fetch_sub(1, Ordering::Release) == 1 {
                    // This was the last weak reference too, deallocate
                    std::sync::atomic::fence(Ordering::Acquire);
                    
                    let layout = std::alloc::Layout::for_value(&**self);
                    
                    // Notify profiler of deallocation
                    profiler::record_deallocation(layout.size(), std::any::type_name::<T>());
                    
                    std::alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout);
                }
            }
        }
    }
}

impl<T: ?Sized> Deref for ScriptRc<T> {
    type Target = T;
    
    fn deref(&self) -> &T {
        unsafe { &(*self.ptr.as_ptr()).value }
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for ScriptRc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for ScriptRc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: ?Sized> fmt::Pointer for ScriptRc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.ptr.as_ptr(), f)
    }
}

impl<T: ?Sized + PartialEq> PartialEq for ScriptRc<T> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl<T: ?Sized + Eq> Eq for ScriptRc<T> {}

impl<T: ?Sized + PartialOrd> PartialOrd for ScriptRc<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (**self).partial_cmp(&**other)
    }
}

impl<T: ?Sized + Ord> Ord for ScriptRc<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (**self).cmp(&**other)
    }
}

impl<T: ?Sized + Hash> Hash for ScriptRc<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}

// Safety: ScriptRc can be Send if T is Send + Sync because we use atomic operations
// for reference counting and the internal operations are thread-safe
unsafe impl<T: ?Sized + Send + Sync> Send for ScriptRc<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for ScriptRc<T> {}

// ScriptWeak implementation

impl<T> ScriptWeak<T> {
    /// Create a new weak reference that doesn't point to anything
    pub fn new() -> Self {
        ScriptWeak {
            ptr: NonNull::dangling(),
            phantom: PhantomData,
        }
    }
    
    /// Attempt to upgrade to a strong reference
    /// 
    /// Returns `None` if the value has been deallocated
    pub fn upgrade(&self) -> Option<ScriptRc<T>> {
        if self.ptr == NonNull::dangling() {
            return None;
        }
        
        let inner = unsafe { self.ptr.as_ref() };
        
        // Try to increment the strong count
        let mut strong = inner.strong.load(Ordering::Relaxed);
        loop {
            if strong == 0 {
                return None;
            }
            
            match inner.strong.compare_exchange_weak(
                strong,
                strong + 1,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    return Some(ScriptRc {
                        ptr: self.ptr,
                        phantom: PhantomData,
                    });
                }
                Err(current) => strong = current,
            }
        }
    }
    
    /// Get the number of strong references
    /// 
    /// Returns 0 if the value has been deallocated
    pub fn strong_count(&self) -> usize {
        if self.ptr == NonNull::dangling() {
            0
        } else {
            unsafe { self.ptr.as_ref().strong.load(Ordering::SeqCst) }
        }
    }
    
    /// Get the number of weak references
    pub fn weak_count(&self) -> usize {
        if self.ptr == NonNull::dangling() {
            0
        } else {
            unsafe { self.ptr.as_ref().weak.load(Ordering::SeqCst) }
        }
    }
}

impl<T> Default for ScriptWeak<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for ScriptWeak<T> {
    fn clone(&self) -> Self {
        if self.ptr != NonNull::dangling() {
            unsafe { self.ptr.as_ref().weak.fetch_add(1, Ordering::Relaxed); }
        }
        ScriptWeak {
            ptr: self.ptr,
            phantom: PhantomData,
        }
    }
}

impl<T> Drop for ScriptWeak<T> {
    fn drop(&mut self) {
        if self.ptr != NonNull::dangling() {
            let inner = unsafe { self.ptr.as_ref() };
            
            if inner.weak.fetch_sub(1, Ordering::Release) == 1 {
                // This was the last weak reference, deallocate if no strong refs
                std::sync::atomic::fence(Ordering::Acquire);
                
                if inner.strong.load(Ordering::SeqCst) == 0 {
                    unsafe {
                        let layout = std::alloc::Layout::for_value(&*self.ptr.as_ptr());
                        
                        // Notify profiler of deallocation
                        profiler::record_deallocation(layout.size(), std::any::type_name::<T>());
                        
                        std::alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout);
                    }
                }
            }
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for ScriptWeak<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(Weak)")
    }
}

// Safety: ScriptWeak can be Send if T is Send + Sync for the same reasons as ScriptRc
unsafe impl<T: Send + Sync> Send for ScriptWeak<T> {}
unsafe impl<T: Send + Sync> Sync for ScriptWeak<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rc_basic() {
        let rc = ScriptRc::new(42);
        assert_eq!(*rc, 42);
        assert_eq!(rc.strong_count(), 1);
        assert_eq!(rc.weak_count(), 1);
    }
    
    #[test]
    fn test_rc_clone() {
        let rc1 = ScriptRc::new("hello");
        let rc2 = rc1.clone();
        
        assert_eq!(rc1.strong_count(), 2);
        assert_eq!(rc2.strong_count(), 2);
        assert!(ScriptRc::ptr_eq(&rc1, &rc2));
    }
    
    #[test]
    fn test_rc_drop() {
        let rc1 = ScriptRc::new(vec![1, 2, 3]);
        let rc2 = rc1.clone();
        assert_eq!(rc1.strong_count(), 2);
        
        drop(rc2);
        assert_eq!(rc1.strong_count(), 1);
    }
    
    #[test]
    fn test_weak_upgrade() {
        let rc = ScriptRc::new(100);
        let weak = rc.downgrade();
        
        assert_eq!(weak.strong_count(), 1);
        assert_eq!(weak.weak_count(), 2); // 1 from rc, 1 from weak
        
        let upgraded = weak.upgrade().unwrap();
        assert_eq!(*upgraded, 100);
        assert_eq!(rc.strong_count(), 2);
    }
    
    #[test]
    fn test_weak_upgrade_after_drop() {
        let rc = ScriptRc::new("test");
        let weak = rc.downgrade();
        
        drop(rc);
        assert_eq!(weak.strong_count(), 0);
        assert!(weak.upgrade().is_none());
    }
    
    #[test]
    fn test_make_mut() {
        let mut rc1 = ScriptRc::new(10);
        let rc2 = rc1.clone();
        
        // Can't get mutable reference when there are multiple strong refs
        assert!(rc1.get_mut().is_none());
        
        // make_mut will clone the data
        let mutable = rc1.make_mut();
        *mutable = 20;
        
        assert_eq!(*rc1, 20);
        assert_eq!(*rc2, 10); // rc2 still points to the old value
    }
    
    #[test]
    fn test_get_mut_unique() {
        let mut rc = ScriptRc::new(vec![1, 2, 3]);
        
        // Should be able to get mutable reference when unique
        let mutable = rc.get_mut().unwrap();
        mutable.push(4);
        
        assert_eq!(*rc, vec![1, 2, 3, 4]);
    }
}