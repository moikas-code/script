use crate::runtime::traceable::Traceable;
use crate::ScriptRc;
use std::any::Any;
use std::collections::HashMap;
/// Runtime value representation for Script language
///
/// This module provides the runtime representation of values
/// that are used during execution of Script programs.
use std::fmt;

/// Runtime value types
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
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

    /// String value
    String(String),

    /// Array value
    Array(Vec<ScriptRc<Value>>),

    /// Object/Dictionary value
    Object(HashMap<String, ScriptRc<Value>>),

    /// Function reference
    Function(String),

    /// Number value (f64)
    Number(f64),

    /// Boolean value (bool) - alias for Bool
    Boolean(bool),

    /// Enum variant value (for Result, Option, and user-defined enums)
    Enum {
        /// The enum type name (e.g., "Result", "Option")
        type_name: String,
        /// The variant name (e.g., "Ok", "Err", "Some", "None")
        variant: String,
        /// The associated data, if any
        data: Option<ScriptRc<Value>>,
    },

    /// Closure value with captured environment
    Closure(ScriptRc<crate::runtime::closure::Closure>),

    /// Optimized closure value with performance enhancements
    OptimizedClosure(ScriptRc<crate::runtime::closure::OptimizedClosure>),
}

impl Value {
    /// Create an Ok Result value
    pub fn ok(value: Value) -> Self {
        Value::Enum {
            type_name: "Result".to_string(),
            variant: "Ok".to_string(),
            data: Some(ScriptRc::new(value)),
        }
    }

    /// Create an Err Result value
    pub fn err(value: Value) -> Self {
        Value::Enum {
            type_name: "Result".to_string(),
            variant: "Err".to_string(),
            data: Some(ScriptRc::new(value)),
        }
    }

    /// Create a Some Option value
    pub fn some(value: Value) -> Self {
        Value::Enum {
            type_name: "Option".to_string(),
            variant: "Some".to_string(),
            data: Some(ScriptRc::new(value)),
        }
    }

    /// Create a None Option value
    pub fn none() -> Self {
        Value::Enum {
            type_name: "Option".to_string(),
            variant: "None".to_string(),
            data: None,
        }
    }

    /// Check if this is a Result type
    pub fn is_result(&self) -> bool {
        matches!(self, Value::Enum { type_name, .. } if type_name == "Result")
    }

    /// Check if this is an Option type
    pub fn is_option(&self) -> bool {
        matches!(self, Value::Enum { type_name, .. } if type_name == "Option")
    }

    /// Get the inner value of Ok/Some, or None if not applicable
    pub fn unwrap_ok_or_some(&self) -> Option<&Value> {
        match self {
            Value::Enum {
                type_name,
                variant,
                data,
            } => match (type_name.as_str(), variant.as_str()) {
                ("Result", "Ok") | ("Option", "Some") => data.as_ref().map(|v| &**v),
                _ => None,
            },
            _ => None,
        }
    }

    /// Get the inner value of Err, or None if not applicable
    pub fn unwrap_err(&self) -> Option<&Value> {
        match self {
            Value::Enum {
                type_name: t,
                variant: v,
                data,
            } if t == "Result" && v == "Err" => data.as_ref().map(|val| &**val),
            _ => None,
        }
    }

    /// Check if value is truthy
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::I32(i) => *i != 0,
            Value::I64(i) => *i != 0,
            Value::F32(f) => *f != 0.0,
            Value::F64(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Object(o) => !o.is_empty(),
            Value::Function(_) => true,
            Value::Number(n) => *n != 0.0,
            Value::Boolean(b) => *b,
            Value::Enum { variant, .. } => {
                // Result::Err and Option::None are falsy, everything else is truthy
                variant != "Err" && variant != "None"
            }
            Value::Closure(_) => true,
            Value::OptimizedClosure(_) => true,
        }
    }

    /// Get type name for error messages
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::I32(_) => "i32",
            Value::I64(_) => "i64",
            Value::F32(_) => "f32",
            Value::F64(_) => "f64",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
            Value::Function(_) => "function",
            Value::Number(_) => "number",
            Value::Boolean(_) => "boolean",
            Value::Enum { type_name, .. } => {
                // Return a static str for known types, otherwise leak for safety
                match type_name.as_str() {
                    "Result" => "Result",
                    "Option" => "Option",
                    _ => Box::leak(type_name.clone().into_boxed_str()),
                }
            }
            Value::Closure(_) => "closure",
            Value::OptimizedClosure(_) => "closure",
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::I32(i) => write!(f, "{}", i),
            Value::I64(i) => write!(f, "{}", i),
            Value::F32(fl) => write!(f, "{}", fl),
            Value::F64(fl) => write!(f, "{}", fl),
            Value::String(s) => write!(f, "{}", s),
            Value::Array(arr) => {
                write!(f, "[")?;
                for (i, item) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Value::Object(obj) => {
                write!(f, "{{")?;
                for (i, (key, value)) in obj.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", key, value)?;
                }
                write!(f, "}}")
            }
            Value::Function(name) => write!(f, "<function {}>", name),
            Value::Number(n) => write!(f, "{}", n),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Enum {
                type_name,
                variant,
                data,
            } => {
                match (type_name.as_str(), variant.as_str(), data) {
                    ("Option", "Some", Some(val)) => write!(f, "Some({})", val),
                    ("Option", "None", None) => write!(f, "None"),
                    ("Result", "Ok", Some(val)) => write!(f, "Ok({})", val),
                    ("Result", "Err", Some(val)) => write!(f, "Err({})", val),
                    // Generic enum display
                    (_, _, Some(val)) => write!(f, "{}::{}({})", type_name, variant, val),
                    (_, _, None) => write!(f, "{}::{}", type_name, variant),
                }
            }
            Value::Closure(closure) => write!(f, "{}", closure),
            Value::OptimizedClosure(closure) => write!(f, "{}", closure),
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}

impl Traceable for Value {
    fn trace(&self, visitor: &mut dyn FnMut(&dyn Any)) {
        match self {
            // Primitive values have no references to trace
            Value::Null
            | Value::Bool(_)
            | Value::I32(_)
            | Value::I64(_)
            | Value::F32(_)
            | Value::F64(_)
            | Value::String(_)
            | Value::Function(_)
            | Value::Number(_)
            | Value::Boolean(_) => {
                // No references to trace
            }

            // Arrays contain ScriptRc references
            Value::Array(items) => {
                for item in items {
                    // Report the ScriptRc itself
                    visitor(item as &dyn Any);
                    // Also trace the contained value
                    item.trace(visitor);
                }
            }

            // Objects contain ScriptRc references in values
            Value::Object(map) => {
                for value in map.values() {
                    // Report the ScriptRc itself
                    visitor(value as &dyn Any);
                    // Also trace the contained value
                    value.trace(visitor);
                }
            }

            // Enum variants may contain ScriptRc references
            Value::Enum { data, .. } => {
                if let Some(val) = data {
                    // Report the ScriptRc itself
                    visitor(val as &dyn Any);
                    // Also trace the contained value
                    val.trace(visitor);
                }
            }

            // Closures contain ScriptRc references to captured variables
            Value::Closure(closure) => {
                // Report the ScriptRc itself
                visitor(closure as &dyn Any);
                // Also trace the closure's captured variables
                closure.trace(visitor);
            }

            // Optimized closures also contain ScriptRc references
            Value::OptimizedClosure(closure) => {
                // Report the ScriptRc itself
                visitor(closure as &dyn Any);
                // Also trace the closure's captured variables
                closure.trace(visitor);
            }
        }
    }

    fn trace_size(&self) -> usize {
        let base_size = std::mem::size_of::<Value>();
        match self {
            Value::String(s) => base_size + s.capacity(),
            Value::Array(arr) => {
                base_size + arr.capacity() * std::mem::size_of::<ScriptRc<Value>>()
            }
            Value::Object(map) => {
                base_size
                    + map.capacity()
                        * (std::mem::size_of::<String>() + std::mem::size_of::<ScriptRc<Value>>())
            }
            Value::Enum {
                type_name,
                variant,
                data,
            } => {
                base_size
                    + type_name.capacity()
                    + variant.capacity()
                    + data
                        .as_ref()
                        .map_or(0, |_| std::mem::size_of::<ScriptRc<Value>>())
            }
            Value::Closure(closure) => base_size + closure.trace_size(),
            Value::OptimizedClosure(closure) => base_size + closure.trace_size(),
            _ => base_size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_creation() {
        let ok_val = Value::ok(Value::I32(42));
        assert!(ok_val.is_result());
        assert!(!ok_val.is_option());

        match &ok_val {
            Value::Enum {
                type_name,
                variant,
                data,
            } => {
                assert_eq!(type_name, "Result");
                assert_eq!(variant, "Ok");
                assert!(data.is_some());
                let inner = data.as_ref().unwrap();
                assert_eq!(**inner, Value::I32(42));
            }
            _ => panic!("Expected Enum variant"),
        }

        let err_val = Value::err(Value::String("error".to_string()));
        assert!(err_val.is_result());
        assert_eq!(
            err_val.unwrap_err(),
            Some(&Value::String("error".to_string()))
        );
    }

    #[test]
    fn test_option_creation() {
        let some_val = Value::some(Value::Bool(true));
        assert!(some_val.is_option());
        assert!(!some_val.is_result());

        match &some_val {
            Value::Enum {
                type_name,
                variant,
                data,
            } => {
                assert_eq!(type_name, "Option");
                assert_eq!(variant, "Some");
                assert!(data.is_some());
            }
            _ => panic!("Expected Enum variant"),
        }

        let none_val = Value::none();
        assert!(none_val.is_option());
        match &none_val {
            Value::Enum {
                type_name,
                variant,
                data,
            } => {
                assert_eq!(type_name, "Option");
                assert_eq!(variant, "None");
                assert!(data.is_none());
            }
            _ => panic!("Expected Enum variant"),
        }
    }

    #[test]
    fn test_enum_truthiness() {
        let ok_val = Value::ok(Value::I32(42));
        assert!(ok_val.is_truthy());

        let err_val = Value::err(Value::String("error".to_string()));
        assert!(!err_val.is_truthy());

        let some_val = Value::some(Value::Bool(false));
        assert!(some_val.is_truthy());

        let none_val = Value::none();
        assert!(!none_val.is_truthy());
    }

    #[test]
    fn test_enum_display() {
        let ok_val = Value::ok(Value::I32(42));
        assert_eq!(ok_val.to_string(), "Ok(42)");

        let err_val = Value::err(Value::String("error".to_string()));
        assert_eq!(err_val.to_string(), "Err(error)");

        let some_val = Value::some(Value::Bool(true));
        assert_eq!(some_val.to_string(), "Some(true)");

        let none_val = Value::none();
        assert_eq!(none_val.to_string(), "None");
    }

    #[test]
    fn test_unwrap_helpers() {
        let ok_val = Value::ok(Value::I32(42));
        assert_eq!(ok_val.unwrap_ok_or_some(), Some(&Value::I32(42)));
        assert_eq!(ok_val.unwrap_err(), None);

        let err_val = Value::err(Value::String("error".to_string()));
        assert_eq!(err_val.unwrap_ok_or_some(), None);
        assert_eq!(
            err_val.unwrap_err(),
            Some(&Value::String("error".to_string()))
        );

        let some_val = Value::some(Value::F32(3.14));
        assert_eq!(some_val.unwrap_ok_or_some(), Some(&Value::F32(3.14)));

        let none_val = Value::none();
        assert_eq!(none_val.unwrap_ok_or_some(), None);
    }

    #[test]
    fn test_custom_enum() {
        let custom = Value::Enum {
            type_name: "Color".to_string(),
            variant: "Red".to_string(),
            data: None,
        };

        assert!(!custom.is_result());
        assert!(!custom.is_option());
        assert_eq!(custom.type_name(), "Color");
        assert_eq!(custom.to_string(), "Color::Red");

        let with_data = Value::Enum {
            type_name: "Message".to_string(),
            variant: "Text".to_string(),
            data: Some(ScriptRc::new(Value::String("Hello".to_string()))),
        };

        assert_eq!(with_data.to_string(), "Message::Text(Hello)");
    }
}
