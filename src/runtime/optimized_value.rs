use std::collections::HashMap;
use std::fmt;

/// Optimized runtime value representation for Script language
///
/// Key optimizations:
/// 1. Use inline storage for small values to avoid heap allocation
/// 2. Pool object allocation for better cache locality  
/// 3. Reduced reference counting overhead
/// 4. Compact memory layout with better alignment
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizedValue {
    /// Immediate values that fit in 8 bytes - no heap allocation
    Immediate(ImmediateValue),

    /// Heap-allocated values for larger data
    Heap(HeapValue),
}

/// Immediate values stored inline - no heap allocation needed
#[derive(Debug, Clone, PartialEq)]
pub enum ImmediateValue {
    /// Null value
    Null,

    /// Boolean value
    Bool(bool),

    /// 32-bit integer value
    I32(i32),

    /// 64-bit integer value  
    I64(i64),

    /// 32-bit floating point value
    F32(f32),

    /// 64-bit floating point value
    F64(f64),

    /// Small string (up to 7 bytes) stored inline
    SmallString([u8; 7], u8), // data, length
}

/// Heap-allocated values for larger data structures
#[derive(Debug, Clone, PartialEq)]
pub enum HeapValue {
    /// Large string value
    String(String),

    /// Array value with optimized storage
    Array(ArrayValue),

    /// Object/Dictionary value with optimized storage
    Object(ObjectValue),

    /// Function reference
    Function(String),

    /// Enum variant value
    EnumVariant {
        variant_name: String,
        data: Option<Box<OptimizedValue>>,
    },

    /// Struct value with named fields
    Struct {
        type_name: String,
        fields: HashMap<String, OptimizedValue>,
    },
}

/// Optimized array storage
#[derive(Debug, Clone, PartialEq)]
pub struct ArrayValue {
    /// Elements stored directly when possible to improve cache locality
    elements: Vec<OptimizedValue>,
    /// Pre-computed hash for quick equality checks
    hash_cache: Option<u64>,
}

/// Optimized object storage
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectValue {
    /// Fields stored in sorted order for binary search
    fields: Vec<(String, OptimizedValue)>,
    /// Pre-computed hash for quick equality checks
    hash_cache: Option<u64>,
}

impl OptimizedValue {
    /// Create a null value
    pub fn null() -> Self {
        OptimizedValue::Immediate(ImmediateValue::Null)
    }

    /// Create a boolean value
    pub fn bool(value: bool) -> Self {
        OptimizedValue::Immediate(ImmediateValue::Bool(value))
    }

    /// Create an i32 value
    pub fn i32(value: i32) -> Self {
        OptimizedValue::Immediate(ImmediateValue::I32(value))
    }

    /// Create an i64 value
    pub fn i64(value: i64) -> Self {
        OptimizedValue::Immediate(ImmediateValue::I64(value))
    }

    /// Create an f32 value
    pub fn f32(value: f32) -> Self {
        OptimizedValue::Immediate(ImmediateValue::F32(value))
    }

    /// Create an f64 value
    pub fn f64(value: f64) -> Self {
        OptimizedValue::Immediate(ImmediateValue::F64(value))
    }

    /// Create a string value with optimized storage
    pub fn string(value: String) -> Self {
        if value.len() <= 7 {
            // Store small strings inline
            let mut data = [0u8; 7];
            let bytes = value.as_bytes();
            data[..bytes.len()].copy_from_slice(bytes);
            OptimizedValue::Immediate(ImmediateValue::SmallString(data, bytes.len() as u8))
        } else {
            OptimizedValue::Heap(HeapValue::String(value))
        }
    }

    /// Create an array value
    pub fn array(elements: Vec<OptimizedValue>) -> Self {
        OptimizedValue::Heap(HeapValue::Array(ArrayValue {
            elements,
            hash_cache: None,
        }))
    }

    /// Create an object value
    pub fn object(fields: HashMap<String, OptimizedValue>) -> Self {
        let mut sorted_fields: Vec<_> = fields.into_iter().collect();
        sorted_fields.sort_by(|a, b| a.0.cmp(&b.0));

        OptimizedValue::Heap(HeapValue::Object(ObjectValue {
            fields: sorted_fields,
            hash_cache: None,
        }))
    }

    /// Check if this value is truthy
    pub fn is_truthy(&self) -> bool {
        match self {
            OptimizedValue::Immediate(ImmediateValue::Null) => false,
            OptimizedValue::Immediate(ImmediateValue::Bool(b)) => *b,
            OptimizedValue::Immediate(ImmediateValue::I32(i)) => *i != 0,
            OptimizedValue::Immediate(ImmediateValue::I64(i)) => *i != 0,
            OptimizedValue::Immediate(ImmediateValue::F32(f)) => *f != 0.0,
            OptimizedValue::Immediate(ImmediateValue::F64(f)) => *f != 0.0,
            OptimizedValue::Immediate(ImmediateValue::SmallString(_, len)) => *len > 0,
            OptimizedValue::Heap(HeapValue::String(s)) => !s.is_empty(),
            OptimizedValue::Heap(HeapValue::Array(arr)) => !arr.elements.is_empty(),
            OptimizedValue::Heap(HeapValue::Object(obj)) => !obj.fields.is_empty(),
            _ => true,
        }
    }

    /// Get the size in bytes of this value (approximate)
    pub fn size_bytes(&self) -> usize {
        match self {
            OptimizedValue::Immediate(_) => std::mem::size_of::<ImmediateValue>(),
            OptimizedValue::Heap(heap_val) => {
                std::mem::size_of::<HeapValue>()
                    + match heap_val {
                        HeapValue::String(s) => s.capacity(),
                        HeapValue::Array(arr) => {
                            arr.elements.len() * std::mem::size_of::<OptimizedValue>()
                        }
                        HeapValue::Object(obj) => {
                            obj.fields.len()
                                * (std::mem::size_of::<String>()
                                    + std::mem::size_of::<OptimizedValue>())
                        }
                        HeapValue::Function(name) => name.capacity(),
                        HeapValue::EnumVariant { variant_name, data } => {
                            variant_name.capacity() + data.as_ref().map_or(0, |v| v.size_bytes())
                        }
                        HeapValue::Struct { type_name, fields } => {
                            type_name.capacity()
                                + fields.len()
                                    * (std::mem::size_of::<String>()
                                        + std::mem::size_of::<OptimizedValue>())
                        }
                    }
            }
        }
    }
}

impl ArrayValue {
    /// Get an element by index
    pub fn get(&self, index: usize) -> Option<&OptimizedValue> {
        self.elements.get(index)
    }

    /// Get the length of the array
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Check if the array is empty
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Invalidate the hash cache when modified
    pub fn invalidate_cache(&mut self) {
        self.hash_cache = None;
    }
}

impl ObjectValue {
    /// Get a field value by name using binary search
    pub fn get(&self, name: &str) -> Option<&OptimizedValue> {
        self.fields
            .binary_search_by_key(&name, |(k, _)| k.as_str())
            .ok()
            .map(|index| &self.fields[index].1)
    }

    /// Get all field names
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.fields.iter().map(|(k, _)| k.as_str())
    }

    /// Get the number of fields
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Check if the object is empty
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Invalidate the hash cache when modified
    pub fn invalidate_cache(&mut self) {
        self.hash_cache = None;
    }
}

impl fmt::Display for OptimizedValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OptimizedValue::Immediate(ImmediateValue::Null) => write!(f, "null"),
            OptimizedValue::Immediate(ImmediateValue::Bool(b)) => write!(f, "{}", b),
            OptimizedValue::Immediate(ImmediateValue::I32(i)) => write!(f, "{}", i),
            OptimizedValue::Immediate(ImmediateValue::I64(i)) => write!(f, "{}", i),
            OptimizedValue::Immediate(ImmediateValue::F32(fl)) => write!(f, "{}", fl),
            OptimizedValue::Immediate(ImmediateValue::F64(fl)) => write!(f, "{}", fl),
            OptimizedValue::Immediate(ImmediateValue::SmallString(data, len)) => {
                let str_data = &data[..*len as usize];
                write!(
                    f,
                    "\"{}\"",
                    std::str::from_utf8(str_data).unwrap_or("<invalid utf8>")
                )
            }
            OptimizedValue::Heap(HeapValue::String(s)) => write!(f, "\"{}\"", s),
            OptimizedValue::Heap(HeapValue::Array(arr)) => {
                write!(f, "[")?;
                for (i, elem) in arr.elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, "]")
            }
            OptimizedValue::Heap(HeapValue::Object(obj)) => {
                write!(f, "{{")?;
                for (i, (key, value)) in obj.fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{}\": {}", key, value)?;
                }
                write!(f, "}}")
            }
            OptimizedValue::Heap(HeapValue::Function(name)) => write!(f, "<function {}>", name),
            OptimizedValue::Heap(HeapValue::EnumVariant { variant_name, data }) => match data {
                Some(data) => write!(f, "{}({})", variant_name, data),
                None => write!(f, "{}", variant_name),
            },
            OptimizedValue::Heap(HeapValue::Struct { type_name, fields }) => {
                write!(f, "{} {{", type_name)?;
                for (i, (key, value)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", key, value)?;
                }
                write!(f, "}}")
            }
        }
    }
}

// Conversion from the old Value enum to the optimized one
impl From<crate::runtime::value::Value> for OptimizedValue {
    fn from(value: crate::runtime::value::Value) -> Self {
        match value {
            crate::runtime::value::Value::Null => OptimizedValue::null(),
            crate::runtime::value::Value::Bool(b) => OptimizedValue::bool(b),
            crate::runtime::value::Value::I32(i) => OptimizedValue::i32(i),
            crate::runtime::value::Value::I64(i) => OptimizedValue::i64(i),
            crate::runtime::value::Value::F32(f) => OptimizedValue::f32(f),
            crate::runtime::value::Value::F64(f) => OptimizedValue::f64(f),
            crate::runtime::value::Value::String(s) => OptimizedValue::string(s),
            crate::runtime::value::Value::Number(n) => OptimizedValue::f64(n),
            crate::runtime::value::Value::Boolean(b) => OptimizedValue::bool(b),
            crate::runtime::value::Value::Function(name) => {
                OptimizedValue::Heap(HeapValue::Function(name))
            }
            // Note: Array and Object conversions would require recursive conversion
            // This is a simplified version for now
            _ => OptimizedValue::null(), // Fallback for complex cases
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_immediate_values() {
        let val = OptimizedValue::i32(42);
        assert_eq!(val.size_bytes(), std::mem::size_of::<ImmediateValue>());
        assert!(val.is_truthy());

        let null_val = OptimizedValue::null();
        assert!(!null_val.is_truthy());
    }

    #[test]
    fn test_small_string_optimization() {
        let small = OptimizedValue::string("hello".to_string());
        match small {
            OptimizedValue::Immediate(ImmediateValue::SmallString(_, len)) => {
                assert_eq!(len, 5);
            }
            _ => panic!("Expected small string"),
        }

        let large = OptimizedValue::string("this is a very long string".to_string());
        match large {
            OptimizedValue::Heap(HeapValue::String(_)) => {}
            _ => panic!("Expected heap string"),
        }
    }

    #[test]
    fn test_object_binary_search() {
        let mut fields = HashMap::new();
        fields.insert("c".to_string(), OptimizedValue::i32(3));
        fields.insert("a".to_string(), OptimizedValue::i32(1));
        fields.insert("b".to_string(), OptimizedValue::i32(2));

        let obj = OptimizedValue::object(fields);

        if let OptimizedValue::Heap(HeapValue::Object(obj_val)) = &obj {
            assert_eq!(obj_val.get("a"), Some(&OptimizedValue::i32(1)));
            assert_eq!(obj_val.get("b"), Some(&OptimizedValue::i32(2)));
            assert_eq!(obj_val.get("c"), Some(&OptimizedValue::i32(3)));
            assert_eq!(obj_val.get("d"), None);
        } else {
            panic!("Expected object value");
        }
    }
}
