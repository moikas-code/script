//! Type information builder for DWARF debug information (simplified implementation)
//!
//! This is a placeholder implementation for type debugging information.
//! A full implementation would create proper DWARF type entries.

use crate::types::Type as ScriptType;
use std::collections::HashMap;

/// Builder for DWARF type information (simplified)
pub struct TypeInfoBuilder {
    /// Map from Script types to type IDs
    type_map: HashMap<ScriptType, u32>,
    /// Next type ID to assign
    next_id: u32,
}

impl TypeInfoBuilder {
    /// Create a new type info builder
    pub fn new() -> Self {
        Self {
            type_map: HashMap::new(),
            next_id: 1,
        }
    }

    /// Add a base type and return its ID
    pub fn add_base_type(&mut self, script_type: &ScriptType) -> u32 {
        if let Some(&existing_id) = self.type_map.get(script_type) {
            return existing_id;
        }

        let type_id = self.get_or_create_type_id(script_type);
        type_id
    }

    /// Get or create a type ID for the given Script type
    fn get_or_create_type_id(&mut self, script_type: &ScriptType) -> u32 {
        match script_type {
            ScriptType::I32 => self.get_or_insert_primitive("i32", script_type),
            ScriptType::F32 => self.get_or_insert_primitive("f32", script_type),
            ScriptType::Bool => self.get_or_insert_primitive("bool", script_type),
            ScriptType::String => self.get_or_insert_primitive("string", script_type),
            ScriptType::Unknown => self.get_or_insert_primitive("unknown", script_type),
            ScriptType::Never => self.get_or_insert_primitive("never", script_type),
            ScriptType::Array(element_type) => {
                // For arrays, create a type based on the element type
                let _element_id = self.add_base_type(element_type);
                self.get_or_insert_composite("array", script_type)
            }
            ScriptType::Function { params, ret } => {
                // For functions, recursively add parameter and return types
                for param_type in params {
                    let _param_id = self.add_base_type(param_type);
                }
                let _return_id = self.add_base_type(ret);
                self.get_or_insert_composite("function", script_type)
            }
            ScriptType::Result { ok, err } => {
                let _ok_id = self.add_base_type(ok);
                let _err_id = self.add_base_type(err);
                self.get_or_insert_composite("result", script_type)
            }
            ScriptType::Future(inner) => {
                let _inner_id = self.add_base_type(inner);
                self.get_or_insert_composite("future", script_type)
            }
            ScriptType::Option(inner) => {
                let _inner_id = self.add_base_type(inner);
                self.get_or_insert_composite("option", script_type)
            }
            ScriptType::Named(name) => self.get_or_insert_named(name, script_type),
            ScriptType::TypeVar(_) => {
                // Type variables should be resolved by now, treat as unknown
                self.get_or_insert_primitive("typevar", script_type)
            }
            ScriptType::Generic { name, args: _ } => {
                // Generic types should be resolved by now, treat as named type
                self.get_or_insert_named(name, script_type)
            }
            ScriptType::TypeParam(name) => {
                // Type parameters should be resolved by now, treat as named type
                self.get_or_insert_named(name, script_type)
            }
            ScriptType::Tuple(types) => {
                // For tuples, recursively add element types
                for element_type in types {
                    let _element_id = self.add_base_type(element_type);
                }
                self.get_or_insert_composite("tuple", script_type)
            }
            ScriptType::Reference { inner, .. } => {
                // For references, add the inner type
                let _inner_id = self.add_base_type(inner);
                self.get_or_insert_composite("reference", script_type)
            }
        }
    }

    /// Get or insert a primitive type
    fn get_or_insert_primitive(&mut self, _name: &str, script_type: &ScriptType) -> u32 {
        if let Some(&existing_id) = self.type_map.get(script_type) {
            existing_id
        } else {
            let type_id = self.next_id;
            self.next_id += 1;
            self.type_map.insert(script_type.clone(), type_id);
            type_id
        }
    }

    /// Get or insert a composite type
    fn get_or_insert_composite(&mut self, _name: &str, script_type: &ScriptType) -> u32 {
        if let Some(&existing_id) = self.type_map.get(script_type) {
            existing_id
        } else {
            let type_id = self.next_id;
            self.next_id += 1;
            self.type_map.insert(script_type.clone(), type_id);
            type_id
        }
    }

    /// Get or insert a named type
    fn get_or_insert_named(&mut self, _name: &str, script_type: &ScriptType) -> u32 {
        if let Some(&existing_id) = self.type_map.get(script_type) {
            existing_id
        } else {
            let type_id = self.next_id;
            self.next_id += 1;
            self.type_map.insert(script_type.clone(), type_id);
            type_id
        }
    }

    /// Get a type ID by Script type
    pub fn get_type_id(&self, script_type: &ScriptType) -> Option<u32> {
        self.type_map.get(script_type).copied()
    }

    /// Get the number of types registered
    pub fn type_count(&self) -> usize {
        self.type_map.len()
    }
}

impl Default for TypeInfoBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_info_builder_creation() {
        let builder = TypeInfoBuilder::new();
        assert_eq!(builder.type_count(), 0);
        assert_eq!(builder.next_id, 1);
    }

    #[test]
    fn test_add_base_types() {
        let mut builder = TypeInfoBuilder::new();

        let i32_id = builder.add_base_type(&ScriptType::I32);
        let f32_id = builder.add_base_type(&ScriptType::F32);
        let bool_id = builder.add_base_type(&ScriptType::Bool);
        let string_id = builder.add_base_type(&ScriptType::String);

        // Each type should get a unique ID
        assert_ne!(i32_id, f32_id);
        assert_ne!(f32_id, bool_id);
        assert_ne!(bool_id, string_id);

        // Adding the same type again should return the same ID
        let i32_id2 = builder.add_base_type(&ScriptType::I32);
        assert_eq!(i32_id, i32_id2);

        assert_eq!(builder.type_count(), 4);
    }

    #[test]
    fn test_add_array_type() {
        let mut builder = TypeInfoBuilder::new();

        let array_type = ScriptType::Array(Box::new(ScriptType::I32));
        let array_id = builder.add_base_type(&array_type);

        assert!(builder.type_map.contains_key(&array_type));
        assert_eq!(builder.get_type_id(&array_type), Some(array_id));
    }

    #[test]
    fn test_add_function_type() {
        let mut builder = TypeInfoBuilder::new();

        let func_type = ScriptType::Function {
            params: vec![ScriptType::I32, ScriptType::F32],
            ret: Box::new(ScriptType::Bool),
        };

        let func_id = builder.add_base_type(&func_type);

        assert!(builder.type_map.contains_key(&func_type));
        assert_eq!(builder.get_type_id(&func_type), Some(func_id));
    }

    #[test]
    fn test_get_type_id() {
        let mut builder = TypeInfoBuilder::new();

        let i32_type = ScriptType::I32;
        let i32_id = builder.add_base_type(&i32_type);

        assert_eq!(builder.get_type_id(&i32_type), Some(i32_id));
        assert_eq!(builder.get_type_id(&ScriptType::F32), None);
    }

    #[test]
    fn test_complex_types() {
        let mut builder = TypeInfoBuilder::new();

        // Test nested types
        let result_type = ScriptType::Result {
            ok: Box::new(ScriptType::I32),
            err: Box::new(ScriptType::String),
        };

        let option_type = ScriptType::Option(Box::new(ScriptType::F32));
        let future_type = ScriptType::Future(Box::new(ScriptType::Bool));

        let result_id = builder.add_base_type(&result_type);
        let option_id = builder.add_base_type(&option_type);
        let future_id = builder.add_base_type(&future_type);

        // All should have unique IDs
        assert_ne!(result_id, option_id);
        assert_ne!(option_id, future_id);
        assert_ne!(result_id, future_id);

        // Check that we can retrieve them
        assert_eq!(builder.get_type_id(&result_type), Some(result_id));
        assert_eq!(builder.get_type_id(&option_type), Some(option_id));
        assert_eq!(builder.get_type_id(&future_type), Some(future_id));
    }
}
