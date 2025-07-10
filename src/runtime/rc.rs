//! Reference counting smart pointer implementation for Script
//!
//! This module provides `ScriptRc<T>` and `ScriptWeak<T>`, which are similar to
//! Rust's `Rc<T>` and `Weak<T>` but adapted for Script's needs:
//! - Integration with cycle detection
//! - Memory profiling hooks
//! - Thread-safe for future actor model support
//! - Panic-safe with proper cleanup

use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};

use crate::runtime::gc;
use crate::runtime::profiler;
use crate::runtime::type_registry::{self, TypeId};

/// Color states for tri-color marking algorithm
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Color {
    /// Object not yet visited by GC
    White = 0,
    /// Object visited but children not yet processed
    Gray = 1,
    /// Object and all children processed
    Black = 2,
}

/// Internal reference count structure
struct RcBox<T: ?Sized> {
    /// Number of strong references
    strong: AtomicUsize,
    /// Number of weak references
    weak: AtomicUsize,
    /// Color for tri-color marking during GC
    color: AtomicU8,
    /// Whether this object is buffered as a potential cycle root
    buffered: AtomicBool,
    /// Whether this object has been traced in current GC cycle
    traced: AtomicBool,
    /// Type ID for safe downcasting
    type_id: TypeId,
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
    pub fn new(value: T) -> Self
    where
        T: 'static,
    {
        let layout = std::alloc::Layout::new::<RcBox<T>>();

        // Register the type and get its ID
        let type_id = type_registry::register_type::<T>(
            std::any::type_name::<T>(),
            |ptr, visitor| {
                // Default: assume type has no references
                // Types that contain ScriptRc should implement Traceable
                let _ = (ptr, visitor);
            },
            |ptr| unsafe {
                std::ptr::drop_in_place(ptr as *mut T);
            },
        );

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
                color: AtomicU8::new(Color::Black as u8),
                buffered: AtomicBool::new(false),
                traced: AtomicBool::new(false),
                type_id,
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
        unsafe {
            self.ptr.as_ref().weak.fetch_add(1, Ordering::Relaxed);
        }
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
        T: Clone + 'static,
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

    /// Create a ScriptRc from a raw pointer
    ///
    /// # Safety
    /// The pointer must have been created by ScriptRc::as_raw and must still be valid.
    /// This increments the reference count.
    pub unsafe fn from_raw(ptr: *const ()) -> Option<Self> {
        if ptr.is_null() {
            return None;
        }

        let rc_box = ptr as *mut RcBox<T>;
        let non_null = NonNull::new(rc_box)?;

        // Increment reference count
        (*rc_box).strong.fetch_add(1, Ordering::Relaxed);

        Some(ScriptRc {
            ptr: non_null,
            phantom: PhantomData,
        })
    }

    /// Get the color for GC marking
    pub(crate) fn color(&self) -> Color {
        let color_val = unsafe { self.ptr.as_ref().color.load(Ordering::Relaxed) };
        match color_val {
            0 => Color::White,
            1 => Color::Gray,
            2 => Color::Black,
            _ => Color::Black, // Default to black for safety
        }
    }

    /// Set the color for GC marking
    pub(crate) fn set_color(&self, color: Color) {
        unsafe {
            self.ptr
                .as_ref()
                .color
                .store(color as u8, Ordering::Relaxed);
        }
    }

    /// Check if this object is buffered as a potential cycle root
    pub(crate) fn is_buffered(&self) -> bool {
        unsafe { self.ptr.as_ref().buffered.load(Ordering::Relaxed) }
    }

    /// Mark this object as buffered for cycle detection
    pub(crate) fn set_buffered(&self, buffered: bool) {
        unsafe {
            self.ptr
                .as_ref()
                .buffered
                .store(buffered, Ordering::Relaxed);
        }
    }

    /// Check if this object has been traced in current GC cycle
    pub(crate) fn is_traced(&self) -> bool {
        unsafe { self.ptr.as_ref().traced.load(Ordering::Relaxed) }
    }

    /// Mark this object as traced in current GC cycle
    pub(crate) fn set_traced(&self, traced: bool) {
        unsafe {
            self.ptr.as_ref().traced.store(traced, Ordering::Relaxed);
        }
    }
}

impl<T: ?Sized> ScriptRc<T> {
    /// Get the type ID of this value
    pub fn type_id(&self) -> TypeId {
        unsafe { self.ptr.as_ref().type_id }
    }

    /// Get a raw pointer to this ScriptRc
    pub fn as_raw(&self) -> *const () {
        self.ptr.as_ptr() as *const ()
    }
}

impl<T: ?Sized> Clone for ScriptRc<T> {
    fn clone(&self) -> Self {
        unsafe {
            self.ptr.as_ref().strong.fetch_add(1, Ordering::Relaxed);
        }
        ScriptRc {
            ptr: self.ptr,
            phantom: PhantomData,
        }
    }
}

impl<T: ?Sized> Drop for ScriptRc<T> {
    fn drop(&mut self) {
        unsafe {
            // Use atomic operations to prevent race conditions
            let old_strong = self.ptr.as_ref().strong.fetch_sub(1, Ordering::Release);

            if old_strong == 1 {
                // This was the last strong reference
                std::sync::atomic::fence(Ordering::Acquire);

                // Use secure unregistration if available
                if let Err(_) = crate::runtime::safe_gc::secure_unregister_rc(self) {
                    // Fallback to original GC
                    gc::unregister_rc(self);
                }

                // Drop the value
                std::ptr::drop_in_place(&mut (*self.ptr.as_ptr()).value);

                // Atomically decrement weak count with compare-and-swap to prevent races
                loop {
                    let current_weak = self.ptr.as_ref().weak.load(Ordering::Acquire);
                    if current_weak == 0 {
                        break; // Another thread already handled deallocation
                    }

                    if self
                        .ptr
                        .as_ref()
                        .weak
                        .compare_exchange_weak(
                            current_weak,
                            current_weak - 1,
                            Ordering::Release,
                            Ordering::Relaxed,
                        )
                        .is_ok()
                    {
                        if current_weak == 1 {
                            // This was the last weak reference too, deallocate
                            std::sync::atomic::fence(Ordering::Acquire);

                            let layout = std::alloc::Layout::for_value(&**self);

                            // Notify profiler of deallocation
                            profiler::record_deallocation(
                                layout.size(),
                                std::any::type_name::<T>(),
                            );

                            std::alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout);
                        }
                        break;
                    }
                }
            } else if old_strong > 1 {
                // Reference count decreased but object still alive
                // Use secure notification if available
                if let Err(_) = crate::runtime::safe_gc::secure_possible_cycle(self) {
                    // Fallback to original GC
                    gc::possible_cycle(self);
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

impl<T: ?Sized> AsRef<T> for ScriptRc<T> {
    fn as_ref(&self) -> &T {
        &**self
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

        // Use a retry loop with exponential backoff to handle race conditions
        let mut retries = 0;
        const MAX_RETRIES: usize = 10;

        loop {
            let strong = inner.strong.load(Ordering::Acquire);

            if strong == 0 {
                return None; // Object has been deallocated
            }

            // Use acquire ordering to synchronize with release in drop
            match inner.strong.compare_exchange_weak(
                strong,
                strong.checked_add(1)?, // Prevent overflow
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    // Successfully incremented strong count
                    return Some(ScriptRc {
                        ptr: self.ptr,
                        phantom: PhantomData,
                    });
                }
                Err(_) => {
                    retries += 1;
                    if retries >= MAX_RETRIES {
                        return None; // Give up after too many retries
                    }

                    // Exponential backoff to reduce contention
                    if retries > 3 {
                        std::thread::yield_now();
                    }
                }
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
            unsafe {
                self.ptr.as_ref().weak.fetch_add(1, Ordering::Relaxed);
            }
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

            // Use compare-and-swap loop to prevent race conditions
            loop {
                let current_weak = inner.weak.load(Ordering::Acquire);
                if current_weak == 0 {
                    break; // Already deallocated by another thread
                }

                if inner
                    .weak
                    .compare_exchange_weak(
                        current_weak,
                        current_weak - 1,
                        Ordering::Release,
                        Ordering::Relaxed,
                    )
                    .is_ok()
                {
                    if current_weak == 1 {
                        // This was the last weak reference, check for deallocation
                        std::sync::atomic::fence(Ordering::Acquire);

                        // Double-check strong count to avoid race with drop
                        if inner.strong.load(Ordering::SeqCst) == 0 {
                            unsafe {
                                let layout = std::alloc::Layout::for_value(&*self.ptr.as_ptr());

                                // Notify profiler of deallocation
                                profiler::record_deallocation(
                                    layout.size(),
                                    std::any::type_name::<T>(),
                                );

                                std::alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout);
                            }
                        }
                    }
                    break;
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
