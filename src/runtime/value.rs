use crate::ScriptRc;
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
}

impl Value {
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
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}
