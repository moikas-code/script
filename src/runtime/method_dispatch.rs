//! Method dispatch for runtime values
//!
//! This module implements method dispatch for enum types like Result and Option,
//! allowing methods to be called on these values at runtime.

use crate::runtime::value_conversion::{script_value_to_value, value_to_script_value};
use crate::runtime::{RuntimeError, Value};
use crate::stdlib::StdLib;
use std::collections::HashMap;

/// Method dispatcher for runtime values
pub struct MethodDispatcher {
    /// Registered methods for each type
    methods: HashMap<String, HashMap<String, MethodImpl>>,
    /// Reference to standard library for function implementations
    stdlib: StdLib,
}

/// A method implementation
type MethodImpl = fn(&Value, &[Value], &StdLib) -> Result<Value, RuntimeError>;

impl MethodDispatcher {
    /// Create a new method dispatcher
    pub fn new() -> Self {
        let mut dispatcher = MethodDispatcher {
            methods: HashMap::new(),
            stdlib: StdLib::new(),
        };

        dispatcher.register_option_methods();
        dispatcher.register_result_methods();

        dispatcher
    }

    /// Register a method for a type
    fn register_method(&mut self, type_name: &str, method_name: &str, implementation: MethodImpl) {
        self.methods
            .entry(type_name.to_string())
            .or_insert_with(HashMap::new)
            .insert(method_name.to_string(), implementation);
    }

    /// Register Option methods
    fn register_option_methods(&mut self) {
        self.register_method("Option", "is_some", option_is_some);
        self.register_method("Option", "is_none", option_is_none);
        self.register_method("Option", "unwrap", option_unwrap);
        self.register_method("Option", "unwrap_or", option_unwrap_or);
        self.register_method("Option", "map", option_map);
        self.register_method("Option", "and_then", option_and_then);
        self.register_method("Option", "or", option_or);
        self.register_method("Option", "filter", option_filter);
        self.register_method("Option", "ok_or", option_ok_or);
        self.register_method("Option", "expect", option_expect);
    }

    /// Register Result methods
    fn register_result_methods(&mut self) {
        self.register_method("Result", "is_ok", result_is_ok);
        self.register_method("Result", "is_err", result_is_err);
        self.register_method("Result", "unwrap", result_unwrap);
        self.register_method("Result", "unwrap_err", result_unwrap_err);
        self.register_method("Result", "unwrap_or", result_unwrap_or);
        self.register_method("Result", "map", result_map);
        self.register_method("Result", "map_err", result_map_err);
        self.register_method("Result", "and_then", result_and_then);
        self.register_method("Result", "or", result_or);
        self.register_method("Result", "expect", result_expect);
    }

    /// Dispatch a method call on a value
    pub fn dispatch_method(
        &self,
        receiver: &Value,
        method_name: &str,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        // Get the type name from the value
        let type_name = match receiver {
            Value::Enum { type_name, .. } => type_name.as_str(),
            _ => {
                return Err(RuntimeError::InvalidOperation(format!(
                    "Cannot call method {} on non-enum value",
                    method_name
                )));
            }
        };

        // Look up the method
        let methods = self.methods.get(type_name).ok_or_else(|| {
            RuntimeError::InvalidOperation(format!("No methods registered for type {type_name}"))
        })?;

        let method = methods.get(method_name).ok_or_else(|| {
            RuntimeError::InvalidOperation(format!(
                "Method {} not found for type {}",
                method_name, type_name
            ))
        })?;

        // Call the method
        method(receiver, args, &self.stdlib)
    }
}

// Option method implementations

fn option_is_some(
    receiver: &Value,
    _args: &[Value],
    stdlib: &StdLib,
) -> Result<Value, RuntimeError> {
    // Convert to ScriptValue and call stdlib function
    let script_val = value_to_script_value(receiver)
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    let result = (stdlib
        .get_function("is_some")
        .ok_or_else(|| RuntimeError::InvalidOperation("is_some not found in stdlib".to_string()))?
        .implementation)(&[script_val])
    .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    Ok(script_value_to_value(&result))
}

fn option_is_none(
    receiver: &Value,
    _args: &[Value],
    stdlib: &StdLib,
) -> Result<Value, RuntimeError> {
    let script_val = value_to_script_value(receiver)
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    let result = (stdlib
        .get_function("is_none")
        .ok_or_else(|| RuntimeError::InvalidOperation("is_none not found in stdlib".to_string()))?
        .implementation)(&[script_val])
    .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    Ok(script_value_to_value(&result))
}

fn option_unwrap(
    receiver: &Value,
    _args: &[Value],
    stdlib: &StdLib,
) -> Result<Value, RuntimeError> {
    let script_val = value_to_script_value(receiver)
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    let result = (stdlib
        .get_function("option_unwrap")
        .ok_or_else(|| {
            RuntimeError::InvalidOperation("option_unwrap not found in stdlib".to_string())
        })?
        .implementation)(&[script_val])
    .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    Ok(script_value_to_value(&result))
}

fn option_unwrap_or(
    receiver: &Value,
    args: &[Value],
    stdlib: &StdLib,
) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "unwrap_or expects 1 argument, got {}",
            args.len()
        )));
    }

    let script_receiver = value_to_script_value(receiver)
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;
    let script_default = value_to_script_value(&args[0])
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    let result = (stdlib
        .get_function("option_unwrap_or")
        .ok_or_else(|| {
            RuntimeError::InvalidOperation("option_unwrap_or not found in stdlib".to_string())
        })?
        .implementation)(&[script_receiver, script_default])
    .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    Ok(script_value_to_value(&result))
}

fn option_map(receiver: &Value, args: &[Value], _stdlib: &StdLib) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "map expects 1 argument (function), got {}",
            args.len()
        )));
    }

    // For now, we'll implement a simple version that doesn't support function arguments
    // In a full implementation, this would need to handle function values
    match receiver {
        Value::Enum {
            type_name,
            variant,
            data: _,
        } if type_name == "Option" => {
            match variant.as_str() {
                "Some" => {
                    // In a real implementation, apply the function to the data
                    // For now, just return the argument as the mapped value
                    Ok(Value::some(args[0].clone()))
                }
                "None" => Ok(Value::none()),
                _ => Err(RuntimeError::InvalidOperation(
                    "Invalid Option variant".to_string(),
                )),
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "map called on non-Option value".to_string(),
        )),
    }
}

fn option_and_then(
    receiver: &Value,
    args: &[Value],
    stdlib: &StdLib,
) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "and_then expects 1 argument, got {}",
            args.len()
        )));
    }

    let script_receiver = value_to_script_value(receiver)
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;
    let script_fn = value_to_script_value(&args[0])
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    let result = (stdlib
        .get_function("option_and_then")
        .ok_or_else(|| {
            RuntimeError::InvalidOperation("option_and_then not found in stdlib".to_string())
        })?
        .implementation)(&[script_receiver, script_fn])
    .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    Ok(script_value_to_value(&result))
}

fn option_or(receiver: &Value, args: &[Value], stdlib: &StdLib) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "or expects 1 argument, got {}",
            args.len()
        )));
    }

    let script_receiver = value_to_script_value(receiver)
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;
    let script_other = value_to_script_value(&args[0])
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    let result = (stdlib
        .get_function("option_or")
        .ok_or_else(|| RuntimeError::InvalidOperation("option_or not found in stdlib".to_string()))?
        .implementation)(&[script_receiver, script_other])
    .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    Ok(script_value_to_value(&result))
}

fn option_filter(receiver: &Value, args: &[Value], stdlib: &StdLib) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "filter expects 1 argument, got {}",
            args.len()
        )));
    }

    let script_receiver = value_to_script_value(receiver)
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;
    let script_predicate = value_to_script_value(&args[0])
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    let result = (stdlib
        .get_function("option_filter")
        .ok_or_else(|| {
            RuntimeError::InvalidOperation("option_filter not found in stdlib".to_string())
        })?
        .implementation)(&[script_receiver, script_predicate])
    .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    Ok(script_value_to_value(&result))
}

fn option_ok_or(receiver: &Value, args: &[Value], stdlib: &StdLib) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "ok_or expects 1 argument, got {}",
            args.len()
        )));
    }

    let script_receiver = value_to_script_value(receiver)
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;
    let script_err = value_to_script_value(&args[0])
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    let result = (stdlib
        .get_function("option_ok_or")
        .ok_or_else(|| {
            RuntimeError::InvalidOperation("option_ok_or not found in stdlib".to_string())
        })?
        .implementation)(&[script_receiver, script_err])
    .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    Ok(script_value_to_value(&result))
}

fn option_expect(receiver: &Value, args: &[Value], stdlib: &StdLib) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "expect expects 1 argument, got {}",
            args.len()
        )));
    }

    let script_receiver = value_to_script_value(receiver)
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;
    let script_msg = value_to_script_value(&args[0])
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    let result = (stdlib
        .get_function("option_expect")
        .ok_or_else(|| {
            RuntimeError::InvalidOperation("option_expect not found in stdlib".to_string())
        })?
        .implementation)(&[script_receiver, script_msg])
    .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    Ok(script_value_to_value(&result))
}

// Result method implementations

fn result_is_ok(receiver: &Value, _args: &[Value], stdlib: &StdLib) -> Result<Value, RuntimeError> {
    let script_val = value_to_script_value(receiver)
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    let result = (stdlib
        .get_function("is_ok")
        .ok_or_else(|| RuntimeError::InvalidOperation("is_ok not found in stdlib".to_string()))?
        .implementation)(&[script_val])
    .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    Ok(script_value_to_value(&result))
}

fn result_is_err(
    receiver: &Value,
    _args: &[Value],
    stdlib: &StdLib,
) -> Result<Value, RuntimeError> {
    let script_val = value_to_script_value(receiver)
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    let result = (stdlib
        .get_function("is_err")
        .ok_or_else(|| RuntimeError::InvalidOperation("is_err not found in stdlib".to_string()))?
        .implementation)(&[script_val])
    .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    Ok(script_value_to_value(&result))
}

fn result_unwrap(
    receiver: &Value,
    _args: &[Value],
    stdlib: &StdLib,
) -> Result<Value, RuntimeError> {
    let script_val = value_to_script_value(receiver)
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    let result = (stdlib
        .get_function("result_unwrap")
        .ok_or_else(|| {
            RuntimeError::InvalidOperation("result_unwrap not found in stdlib".to_string())
        })?
        .implementation)(&[script_val])
    .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    Ok(script_value_to_value(&result))
}

fn result_unwrap_err(
    receiver: &Value,
    _args: &[Value],
    stdlib: &StdLib,
) -> Result<Value, RuntimeError> {
    let script_val = value_to_script_value(receiver)
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    let result = (stdlib
        .get_function("unwrap_err")
        .ok_or_else(|| {
            RuntimeError::InvalidOperation("unwrap_err not found in stdlib".to_string())
        })?
        .implementation)(&[script_val])
    .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    Ok(script_value_to_value(&result))
}

fn result_unwrap_or(
    receiver: &Value,
    args: &[Value],
    stdlib: &StdLib,
) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "unwrap_or expects 1 argument, got {}",
            args.len()
        )));
    }

    let script_receiver = value_to_script_value(receiver)
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;
    let script_default = value_to_script_value(&args[0])
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    let result = (stdlib
        .get_function("result_unwrap_or")
        .ok_or_else(|| {
            RuntimeError::InvalidOperation("result_unwrap_or not found in stdlib".to_string())
        })?
        .implementation)(&[script_receiver, script_default])
    .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    Ok(script_value_to_value(&result))
}

fn result_map(receiver: &Value, args: &[Value], stdlib: &StdLib) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "map expects 1 argument, got {}",
            args.len()
        )));
    }

    let script_receiver = value_to_script_value(receiver)
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;
    let script_fn = value_to_script_value(&args[0])
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    let result = (stdlib
        .get_function("result_map")
        .ok_or_else(|| {
            RuntimeError::InvalidOperation("result_map not found in stdlib".to_string())
        })?
        .implementation)(&[script_receiver, script_fn])
    .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    Ok(script_value_to_value(&result))
}

fn result_map_err(
    receiver: &Value,
    args: &[Value],
    stdlib: &StdLib,
) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "map_err expects 1 argument, got {}",
            args.len()
        )));
    }

    let script_receiver = value_to_script_value(receiver)
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;
    let script_fn = value_to_script_value(&args[0])
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    let result = (stdlib
        .get_function("result_map_err")
        .ok_or_else(|| {
            RuntimeError::InvalidOperation("result_map_err not found in stdlib".to_string())
        })?
        .implementation)(&[script_receiver, script_fn])
    .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    Ok(script_value_to_value(&result))
}

fn result_and_then(
    receiver: &Value,
    args: &[Value],
    stdlib: &StdLib,
) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "and_then expects 1 argument, got {}",
            args.len()
        )));
    }

    let script_receiver = value_to_script_value(receiver)
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;
    let script_fn = value_to_script_value(&args[0])
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    let result = (stdlib
        .get_function("result_and_then")
        .ok_or_else(|| {
            RuntimeError::InvalidOperation("result_and_then not found in stdlib".to_string())
        })?
        .implementation)(&[script_receiver, script_fn])
    .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    Ok(script_value_to_value(&result))
}

fn result_or(receiver: &Value, args: &[Value], stdlib: &StdLib) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "or expects 1 argument, got {}",
            args.len()
        )));
    }

    let script_receiver = value_to_script_value(receiver)
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;
    let script_other = value_to_script_value(&args[0])
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    let result = (stdlib
        .get_function("result_or")
        .ok_or_else(|| RuntimeError::InvalidOperation("result_or not found in stdlib".to_string()))?
        .implementation)(&[script_receiver, script_other])
    .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    Ok(script_value_to_value(&result))
}

fn result_expect(receiver: &Value, args: &[Value], stdlib: &StdLib) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "expect expects 1 argument, got {}",
            args.len()
        )));
    }

    let script_receiver = value_to_script_value(receiver)
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;
    let script_msg = value_to_script_value(&args[0])
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    let result = (stdlib
        .get_function("result_expect")
        .ok_or_else(|| {
            RuntimeError::InvalidOperation("result_expect not found in stdlib".to_string())
        })?
        .implementation)(&[script_receiver, script_msg])
    .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    Ok(script_value_to_value(&result))
}

/// Global method dispatcher instance
static mut METHOD_DISPATCHER: Option<MethodDispatcher> = None;
static METHOD_DISPATCHER_INIT: std::sync::Once = std::sync::Once::new();

/// Get the global method dispatcher
pub fn get_method_dispatcher() -> &'static MethodDispatcher {
    unsafe {
        METHOD_DISPATCHER_INIT.call_once(|| {
            METHOD_DISPATCHER = Some(MethodDispatcher::new());
        });
        METHOD_DISPATCHER.as_ref().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_option_methods() {
        let dispatcher = MethodDispatcher::new();

        // Test is_some
        let some_val = Value::some(Value::I32(42));
        let result = dispatcher
            .dispatch_method(&some_val, "is_some", &[])
            .unwrap();
        assert_eq!(result, Value::Bool(true));

        // Test is_none
        let none_val = Value::none();
        let result = dispatcher
            .dispatch_method(&none_val, "is_none", &[])
            .unwrap();
        assert_eq!(result, Value::Bool(true));

        // Test unwrap_or
        let result = dispatcher
            .dispatch_method(&none_val, "unwrap_or", &[Value::I32(0)])
            .unwrap();
        assert_eq!(result, Value::I32(0));
    }

    #[test]
    fn test_result_methods() {
        let dispatcher = MethodDispatcher::new();

        // Test is_ok
        let ok_val = Value::ok(Value::String("success".to_string()));
        let result = dispatcher.dispatch_method(&ok_val, "is_ok", &[]).unwrap();
        assert_eq!(result, Value::Bool(true));

        // Test is_err
        let err_val = Value::err(Value::String("error".to_string()));
        let result = dispatcher.dispatch_method(&err_val, "is_err", &[]).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn test_invalid_method() {
        let dispatcher = MethodDispatcher::new();
        let some_val = Value::some(Value::I32(42));

        // Test calling non-existent method
        let result = dispatcher.dispatch_method(&some_val, "invalid_method", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_non_enum_value() {
        let dispatcher = MethodDispatcher::new();
        let int_val = Value::I32(42);

        // Test calling method on non-enum value
        let result = dispatcher.dispatch_method(&int_val, "is_some", &[]);
        assert!(result.is_err());
    }
}
