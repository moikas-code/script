//! Type-safe field layout management for object field access
//! 
//! This module provides secure field offset calculation to prevent
//! memory corruption from hash-based field access.

use std::collections::HashMap;
use crate::types::Type;
use crate::error::{Error, ErrorKind};

/// Field layout information for a struct or object type
#[derive(Debug, Clone)]
pub struct FieldLayout {
    /// Field names to offsets mapping
    field_offsets: HashMap<String, usize>,
    /// Field names to types mapping
    field_types: HashMap<String, Type>,
    /// Total size of the struct
    total_size: usize,
    /// Alignment requirement
    alignment: usize,
}

impl FieldLayout {
    /// Create a new field layout from field definitions
    pub fn new(fields: &[(String, Type)]) -> Self {
        let mut field_offsets = HashMap::new();
        let mut field_types = HashMap::new();
        let mut current_offset = 0;
        let mut max_alignment = 1;

        // Calculate offsets with proper alignment
        for (name, ty) in fields {
            let (size, align) = Self::get_type_size_and_align(ty);
            
            // Align current offset
            current_offset = Self::align_offset(current_offset, align);
            
            field_offsets.insert(name.clone(), current_offset);
            field_types.insert(name.clone(), ty.clone());
            
            current_offset += size;
            max_alignment = max_alignment.max(align);
        }

        // Align total size to struct alignment
        let total_size = Self::align_offset(current_offset, max_alignment);

        FieldLayout {
            field_offsets,
            field_types,
            total_size,
            alignment: max_alignment,
        }
    }

    /// Get the offset of a field by name
    pub fn get_field_offset(&self, field_name: &str) -> Result<usize, Error> {
        self.field_offsets
            .get(field_name)
            .copied()
            .ok_or_else(|| Error::new(
                ErrorKind::CodegenError,
                format!("Field '{}' not found in struct layout", field_name)
            ))
    }

    /// Get the type of a field by name
    pub fn get_field_type(&self, field_name: &str) -> Result<&Type, Error> {
        self.field_types
            .get(field_name)
            .ok_or_else(|| Error::new(
                ErrorKind::CodegenError,
                format!("Field '{}' not found in struct layout", field_name)
            ))
    }

    /// Get total size of the struct
    pub fn total_size(&self) -> usize {
        self.total_size
    }

    /// Get alignment requirement
    pub fn alignment(&self) -> usize {
        self.alignment
    }

    /// Calculate size and alignment for a type
    fn get_type_size_and_align(ty: &Type) -> (usize, usize) {
        match ty {
            Type::I32 => (4, 4),
            Type::F32 => (4, 4),
            Type::Bool => (1, 1),
            Type::String => (16, 8), // String struct: ptr + len
            Type::Reference { .. } => (8, 8), // Pointer
            Type::Array(_) => (16, 8), // Array struct: ptr + len
            Type::Tuple(types) => {
                let mut size = 0;
                let mut align = 1;
                for ty in types {
                    let (ty_size, ty_align) = Self::get_type_size_and_align(ty);
                    size = Self::align_offset(size, ty_align) + ty_size;
                    align = align.max(ty_align);
                }
                (Self::align_offset(size, align), align)
            }
            Type::Generic { .. } => (8, 8), // Assume pointer-sized for now
            Type::Function { .. } => (8, 8), // Function pointer
            Type::Never => (0, 1),
            Type::Unknown => (8, 8), // Conservative estimate
            Type::Named(_) => (8, 8), // Will be resolved later
            Type::Option(_) => (12, 4), // Tag + payload
            Type::Result { .. } => (12, 4), // Tag + payload
            Type::Future(_) => (8, 8), // Future pointer
            Type::TypeVar(_) | Type::TypeParam(_) => (8, 8), // Should be resolved
        }
    }

    /// Align an offset to the given alignment
    fn align_offset(offset: usize, align: usize) -> usize {
        (offset + align - 1) & !(align - 1)
    }
}

/// Global field layout registry for user-defined types
#[derive(Debug, Default)]
pub struct FieldLayoutRegistry {
    layouts: HashMap<String, FieldLayout>,
}

impl FieldLayoutRegistry {
    /// Create a new field layout registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a field layout for a type
    pub fn register(&mut self, type_name: String, layout: FieldLayout) {
        self.layouts.insert(type_name, layout);
    }

    /// Get the field layout for a type
    pub fn get(&self, type_name: &str) -> Option<&FieldLayout> {
        self.layouts.get(type_name)
    }

    /// Get mutable field layout for a type
    pub fn get_mut(&mut self, type_name: &str) -> Option<&mut FieldLayout> {
        self.layouts.get_mut(type_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_struct_layout() {
        let fields = vec![
            ("x".to_string(), Type::Int),
            ("y".to_string(), Type::Float),
            ("active".to_string(), Type::Bool),
        ];

        let layout = FieldLayout::new(&fields);

        assert_eq!(layout.get_field_offset("x").unwrap(), 0);
        assert_eq!(layout.get_field_offset("y").unwrap(), 8);
        assert_eq!(layout.get_field_offset("active").unwrap(), 16);
        assert_eq!(layout.total_size(), 24); // Aligned to 8 bytes
    }

    #[test]
    fn test_mixed_alignment() {
        let fields = vec![
            ("flag".to_string(), Type::Bool),
            ("value".to_string(), Type::Int),
            ("flag2".to_string(), Type::Bool),
        ];

        let layout = FieldLayout::new(&fields);

        assert_eq!(layout.get_field_offset("flag").unwrap(), 0);
        assert_eq!(layout.get_field_offset("value").unwrap(), 8); // Aligned to 8
        assert_eq!(layout.get_field_offset("flag2").unwrap(), 16);
    }

    #[test]
    fn test_field_not_found() {
        let fields = vec![("x".to_string(), Type::Int)];
        let layout = FieldLayout::new(&fields);

        assert!(layout.get_field_offset("y").is_err());
        assert!(layout.get_field_type("y").is_err());
    }
}