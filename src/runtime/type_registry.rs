//! Type registry for runtime type information
//!
//! This module provides a global registry of type information used for
//! safe downcasting and cycle detection. Each type that can be stored
//! in a ScriptRc is automatically registered with metadata needed for
//! the garbage collector.

use std::any::{Any, TypeId as StdTypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicU64, Ordering};

/// Unique identifier for registered types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(u64);

impl TypeId {
    /// Get the raw ID value
    pub fn raw(&self) -> u64 {
        self.0
    }
}

/// Type information for registered types
#[derive(Clone)]
pub struct TypeInfo {
    /// Unique type identifier
    pub type_id: TypeId,
    /// Human-readable type name
    pub name: &'static str,
    /// Size of the type in bytes
    pub size: usize,
    /// Rust's standard TypeId
    pub std_type_id: StdTypeId,
    /// Function to trace references in this type
    pub trace_fn: fn(*const u8, &mut dyn FnMut(&dyn Any)),
    /// Function to drop the value
    pub drop_fn: fn(*mut u8),
}

/// Global type registry
static TYPE_REGISTRY: RwLock<Option<Arc<TypeRegistry>>> = RwLock::new(None);

/// Counter for generating unique type IDs
static NEXT_TYPE_ID: AtomicU64 = AtomicU64::new(1);

/// The type registry
pub struct TypeRegistry {
    /// Map from Rust TypeId to our TypeInfo
    by_std_id: HashMap<StdTypeId, TypeInfo>,
    /// Map from our TypeId to TypeInfo
    by_type_id: HashMap<TypeId, TypeInfo>,
    /// Map from type name to TypeInfo
    by_name: HashMap<&'static str, TypeInfo>,
}

impl TypeRegistry {
    /// Create a new type registry
    fn new() -> Self {
        TypeRegistry {
            by_std_id: HashMap::new(),
            by_type_id: HashMap::new(),
            by_name: HashMap::new(),
        }
    }

    /// Register a type with the registry
    fn register_type(&mut self, info: TypeInfo) {
        self.by_std_id.insert(info.std_type_id, info.clone());
        self.by_type_id.insert(info.type_id, info.clone());
        self.by_name.insert(info.name, info);
    }

    /// Look up type info by Rust TypeId
    pub fn get_by_std_id(&self, id: &StdTypeId) -> Option<&TypeInfo> {
        self.by_std_id.get(id)
    }

    /// Look up type info by our TypeId
    pub fn get_by_type_id(&self, id: TypeId) -> Option<&TypeInfo> {
        self.by_type_id.get(&id)
    }

    /// Look up type info by name
    pub fn get_by_name(&self, name: &str) -> Option<&TypeInfo> {
        self.by_name.get(&name)
    }
}

/// Initialize the type registry
pub fn initialize() {
    let mut registry = TYPE_REGISTRY.write().unwrap();
    *registry = Some(Arc::new(TypeRegistry::new()));
}

/// Shutdown the type registry
pub fn shutdown() {
    let mut registry = TYPE_REGISTRY.write().unwrap();
    *registry = None;
}

/// Register a type with the global registry
/// 
/// This is typically called automatically when creating a ScriptRc<T>.
/// Returns the assigned TypeId.
pub fn register_type<T: Any + 'static>(
    name: &'static str,
    trace_fn: fn(*const u8, &mut dyn FnMut(&dyn Any)),
    drop_fn: fn(*mut u8),
) -> TypeId {
    let std_type_id = StdTypeId::of::<T>();
    
    // Check if already registered
    {
        let registry = TYPE_REGISTRY.read().unwrap();
        if let Some(reg) = registry.as_ref() {
            if let Some(info) = reg.get_by_std_id(&std_type_id) {
                return info.type_id;
            }
        }
    }
    
    // Generate new type ID
    let type_id = TypeId(NEXT_TYPE_ID.fetch_add(1, Ordering::Relaxed));
    
    // Create type info
    let info = TypeInfo {
        type_id,
        name,
        size: std::mem::size_of::<T>(),
        std_type_id,
        trace_fn,
        drop_fn,
    };
    
    // Register the type
    let mut registry = TYPE_REGISTRY.write().unwrap();
    if let Some(reg) = registry.as_mut() {
        Arc::get_mut(reg).unwrap().register_type(info);
    }
    
    type_id
}

/// Get type info by TypeId
pub fn get_type_info(id: TypeId) -> Option<TypeInfo> {
    let registry = TYPE_REGISTRY.read().unwrap();
    registry.as_ref()?.get_by_type_id(id).cloned()
}

/// Get type info by Rust TypeId
pub fn get_type_info_by_std_id(id: &StdTypeId) -> Option<TypeInfo> {
    let registry = TYPE_REGISTRY.read().unwrap();
    registry.as_ref()?.get_by_std_id(id).cloned()
}

/// Helper trait for types that can be registered
pub trait RegisterableType: Any + 'static {
    /// Get the type name
    fn type_name() -> &'static str;
    
    /// Trace references in this type
    fn trace_refs(ptr: *const u8, visitor: &mut dyn FnMut(&dyn Any));
    
    /// Drop the value
    fn drop_value(ptr: *mut u8);
}

/// Register a type using the RegisterableType trait
pub fn register_with_trait<T: RegisterableType>() -> TypeId {
    register_type::<T>(
        T::type_name(),
        T::trace_refs,
        T::drop_value,
    )
}

/// Safe downcasting using the type registry
pub struct TypedPointer {
    /// Raw pointer to the data
    ptr: *const u8,
    /// Type ID of the data
    type_id: TypeId,
}

impl TypedPointer {
    /// Create a new typed pointer
    pub fn new(ptr: *const u8, type_id: TypeId) -> Self {
        TypedPointer { ptr, type_id }
    }
    
    /// Try to downcast to a specific type
    pub fn downcast<T: Any + 'static>(&self) -> Option<&T> {
        let target_type_id = StdTypeId::of::<T>();
        
        // Get type info
        let info = get_type_info(self.type_id)?;
        
        // Check if types match
        if info.std_type_id == target_type_id {
            unsafe { Some(&*(self.ptr as *const T)) }
        } else {
            None
        }
    }
    
    /// Get the raw pointer
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }
    
    /// Get the type ID
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::traceable::Traceable;
    
    struct TestType {
        value: i32,
    }
    
    impl RegisterableType for TestType {
        fn type_name() -> &'static str {
            "TestType"
        }
        
        fn trace_refs(_ptr: *const u8, _visitor: &mut dyn FnMut(&dyn Any)) {
            // No references to trace
        }
        
        fn drop_value(ptr: *mut u8) {
            unsafe {
                std::ptr::drop_in_place(ptr as *mut TestType);
            }
        }
    }
    
    #[test]
    fn test_type_registration() {
        initialize();
        
        let type_id = register_with_trait::<TestType>();
        assert!(type_id.0 > 0);
        
        let info = get_type_info(type_id).unwrap();
        assert_eq!(info.name, "TestType");
        assert_eq!(info.size, std::mem::size_of::<TestType>());
        
        shutdown();
    }
    
    #[test]
    fn test_typed_pointer() {
        initialize();
        
        let value = TestType { value: 42 };
        let type_id = register_with_trait::<TestType>();
        
        let typed_ptr = TypedPointer::new(
            &value as *const _ as *const u8,
            type_id,
        );
        
        let downcasted = typed_ptr.downcast::<TestType>().unwrap();
        assert_eq!(downcasted.value, 42);
        
        // Wrong type should fail
        assert!(typed_ptr.downcast::<String>().is_none());
        
        shutdown();
    }
}