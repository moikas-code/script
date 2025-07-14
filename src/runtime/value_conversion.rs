//! Conversion between runtime Value and stdlib ScriptValue
//!
//! This module provides the bridge between the runtime representation of values
//! and the standard library representation, enabling seamless interoperability.

use crate::error::{Error, ErrorKind};
use crate::runtime::{ScriptRc, Value};
use crate::stdlib::{
    ScriptHashMap, ScriptOption, ScriptResult, ScriptString, ScriptValue, ScriptVec,
};

/// Convert a runtime Value to a stdlib ScriptValue
pub fn value_to_script_value(value: &Value) -> Result<ScriptValue, Error> {
    match value {
        Value::Null => Ok(ScriptValue::Unit),

        Value::Bool(b) | Value::Boolean(b) => Ok(ScriptValue::Bool(*b)),

        Value::I32(i) => Ok(ScriptValue::I32(*i)),

        Value::I64(_) => {
            // ScriptValue doesn't have I64, convert to I32 with bounds check
            return Err(Error::new(
                ErrorKind::TypeError,
                "Cannot convert i64 to ScriptValue (no i64 support)",
            ));
        }

        Value::F32(f) => Ok(ScriptValue::F32(*f)),

        Value::F64(f) | Value::Number(f) => {
            // Convert to F32 with potential precision loss
            Ok(ScriptValue::F32(*f as f32))
        }

        Value::String(s) => Ok(ScriptValue::String(ScriptRc::new(ScriptString::from_str(
            s,
        )))),

        Value::Array(items) => {
            let mut vec = ScriptVec::new();
            for item in items {
                vec.push(value_to_script_value(item)?);
            }
            Ok(ScriptValue::Array(ScriptRc::new(vec)))
        }

        Value::Object(map) => {
            let mut hashmap = ScriptHashMap::new();
            for (key, value) in map {
                hashmap.insert(key.clone(), value_to_script_value(value)?);
            }
            Ok(ScriptValue::HashMap(ScriptRc::new(hashmap)))
        }

        Value::Function(_) => {
            // Functions aren't directly representable in ScriptValue
            Err(Error::new(
                ErrorKind::TypeError,
                "Cannot convert function to ScriptValue",
            ))
        }

        Value::Enum {
            type_name,
            variant,
            data,
        } => {
            match type_name.as_str() {
                "Option" => {
                    let script_option = match variant.as_str() {
                        "Some" => {
                            if let Some(inner) = data {
                                ScriptOption::Some(value_to_script_value(inner)?)
                            } else {
                                return Err(Error::new(
                                    ErrorKind::TypeError,
                                    "Option::Some missing data",
                                ));
                            }
                        }
                        "None" => ScriptOption::None,
                        _ => {
                            return Err(Error::new(
                                ErrorKind::TypeError,
                                format!("Unknown Option variant: {variant}"),
                            ));
                        }
                    };
                    Ok(ScriptValue::Option(ScriptRc::new(script_option)))
                }

                "Result" => {
                    let script_result = match variant.as_str() {
                        "Ok" => {
                            if let Some(inner) = data {
                                ScriptResult::Ok(value_to_script_value(inner)?)
                            } else {
                                return Err(Error::new(
                                    ErrorKind::TypeError,
                                    "Result::Ok missing data",
                                ));
                            }
                        }
                        "Err" => {
                            if let Some(inner) = data {
                                ScriptResult::Err(value_to_script_value(inner)?)
                            } else {
                                return Err(Error::new(
                                    ErrorKind::TypeError,
                                    "Result::Err missing data",
                                ));
                            }
                        }
                        _ => {
                            return Err(Error::new(
                                ErrorKind::TypeError,
                                format!("Unknown Result variant: {variant}"),
                            ));
                        }
                    };
                    Ok(ScriptValue::Result(ScriptRc::new(script_result)))
                }

                _ => {
                    // Other enum types not supported in ScriptValue
                    Err(Error::new(
                        ErrorKind::TypeError,
                        format!("Cannot convert enum {} to ScriptValue", type_name),
                    ))
                }
            }
        }

        Value::Closure(_) => {
            // Closures aren't directly representable in ScriptValue
            Err(Error::new(
                ErrorKind::TypeError,
                "Cannot convert closure to ScriptValue",
            ))
        }

        Value::OptimizedClosure(_) => {
            // Optimized closures aren't directly representable in ScriptValue
            Err(Error::new(
                ErrorKind::TypeError,
                "Cannot convert optimized closure to ScriptValue",
            ))
        }
    }
}

/// Convert a stdlib ScriptValue to a runtime Value
pub fn script_value_to_value(script_value: &ScriptValue) -> Value {
    match script_value {
        ScriptValue::Unit => Value::Null,

        ScriptValue::Bool(b) => Value::Bool(*b),

        ScriptValue::I32(i) => Value::I32(*i),

        ScriptValue::F32(f) => Value::F32(*f),

        ScriptValue::String(s) => Value::String(s.as_str().to_string()),

        ScriptValue::Array(vec) => {
            let items: Vec<ScriptRc<Value>> = vec
                .iter()
                .unwrap_or_else(|_| Vec::new())
                .iter()
                .map(|v| ScriptRc::new(script_value_to_value(v)))
                .collect();
            Value::Array(items)
        }

        ScriptValue::HashMap(map) => {
            let mut object = std::collections::HashMap::new();
            for (key, value) in map.iter().unwrap_or_else(|_| Vec::new()) {
                object.insert(key.clone(), ScriptRc::new(script_value_to_value(&value)));
            }
            Value::Object(object)
        }

        ScriptValue::Option(opt) => match &**opt {
            ScriptOption::Some(val) => Value::Enum {
                type_name: "Option".to_string(),
                variant: "Some".to_string(),
                data: Some(ScriptRc::new(script_value_to_value(val))),
            },
            ScriptOption::None => Value::Enum {
                type_name: "Option".to_string(),
                variant: "None".to_string(),
                data: None,
            },
        },

        ScriptValue::Result(res) => match &**res {
            ScriptResult::Ok(val) => Value::Enum {
                type_name: "Result".to_string(),
                variant: "Ok".to_string(),
                data: Some(ScriptRc::new(script_value_to_value(val))),
            },
            ScriptResult::Err(val) => Value::Enum {
                type_name: "Result".to_string(),
                variant: "Err".to_string(),
                data: Some(ScriptRc::new(script_value_to_value(val))),
            },
        },

        ScriptValue::Object(obj) => {
            // Convert object fields to Value::Object
            let mut map = std::collections::HashMap::new();
            for (key, value) in (**obj).iter() {
                map.insert(key.clone(), ScriptRc::new(script_value_to_value(value)));
            }
            Value::Object(map)
        }

        ScriptValue::HashSet(_) => {
            // HashSets aren't directly convertible to runtime Value
            // Convert to empty object for now
            Value::Object(std::collections::HashMap::new())
        }

        ScriptValue::Iterator(_) => {
            // Iterators aren't directly convertible to runtime Value
            // Convert to null for now
            Value::Null
        }

        ScriptValue::Closure(_) => {
            // Closures aren't directly convertible to runtime Value
            // Convert to null for now
            Value::Null
        }
    }
}

/// Helper trait for converting between Value and ScriptValue
pub trait ValueConversion {
    /// Convert to ScriptValue
    fn to_script_value(&self) -> Result<ScriptValue, Error>;

    /// Convert from ScriptValue
    fn from_script_value(script_value: &ScriptValue) -> Result<Self, Error>
    where
        Self: Sized;
}

impl ValueConversion for Value {
    fn to_script_value(&self) -> Result<ScriptValue, Error> {
        value_to_script_value(self)
    }

    fn from_script_value(script_value: &ScriptValue) -> Result<Self, Error> {
        Ok(script_value_to_value(script_value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_conversions() {
        // Test null/unit
        let null_val = Value::Null;
        let script_val = value_to_script_value(&null_val).unwrap();
        assert!(matches!(script_val, ScriptValue::Unit));

        // Test bool
        let bool_val = Value::Bool(true);
        let script_val = value_to_script_value(&bool_val).unwrap();
        assert!(matches!(script_val, ScriptValue::Bool(true)));

        // Test i32
        let i32_val = Value::I32(42);
        let script_val = value_to_script_value(&i32_val).unwrap();
        assert!(matches!(script_val, ScriptValue::I32(42)));

        // Test f32
        let f32_val = Value::F32(3.14);
        let script_val = value_to_script_value(&f32_val).unwrap();
        assert!(matches!(script_val, ScriptValue::F32(f) if (f - 3.14).abs() < 0.001));

        // Test string
        let string_val = Value::String("hello".to_string());
        let script_val = value_to_script_value(&string_val).unwrap();
        match script_val {
            ScriptValue::String(s) => assert_eq!(s.as_str(), "hello"),
            _ => panic!("Expected String"),
        }
    }

    #[test]
    fn test_option_conversion() {
        // Test Some
        let some_val = Value::some(Value::I32(42));
        let script_val = value_to_script_value(&some_val).unwrap();
        match script_val {
            ScriptValue::Option(opt) => {
                assert!(opt.is_some());
                match opt.as_ref() {
                    ScriptOption::Some(ScriptValue::I32(42)) => {}
                    _ => panic!("Expected Some(42)"),
                }
            }
            _ => panic!("Expected Option"),
        }

        // Test None
        let none_val = Value::none();
        let script_val = value_to_script_value(&none_val).unwrap();
        match script_val {
            ScriptValue::Option(opt) => assert!(opt.is_none()),
            _ => panic!("Expected Option"),
        }
    }

    #[test]
    fn test_result_conversion() {
        // Test Ok
        let ok_val = Value::ok(Value::String("success".to_string()));
        let script_val = value_to_script_value(&ok_val).unwrap();
        match script_val {
            ScriptValue::Result(res) => {
                assert!(res.is_ok());
            }
            _ => panic!("Expected Result"),
        }

        // Test Err
        let err_val = Value::err(Value::String("error".to_string()));
        let script_val = value_to_script_value(&err_val).unwrap();
        match script_val {
            ScriptValue::Result(res) => {
                assert!(res.is_err());
            }
            _ => panic!("Expected Result"),
        }
    }

    #[test]
    fn test_roundtrip_conversion() {
        // Test that converting back and forth preserves values
        let original = Value::some(Value::I32(100));
        let script_val = value_to_script_value(&original).unwrap();
        let converted_back = script_value_to_value(&script_val);

        // Compare the values
        match (&original, &converted_back) {
            (
                Value::Enum {
                    type_name: t1,
                    variant: v1,
                    data: d1,
                },
                Value::Enum {
                    type_name: t2,
                    variant: v2,
                    data: d2,
                },
            ) => {
                assert_eq!(t1, t2);
                assert_eq!(v1, v2);
                match (d1, d2) {
                    (Some(val1), Some(val2)) => match (&**val1, &**val2) {
                        (Value::I32(100), Value::I32(100)) => {}
                        _ => panic!("Data mismatch"),
                    },
                    _ => panic!("Data mismatch"),
                }
            }
            _ => panic!("Type mismatch"),
        }
    }
}
