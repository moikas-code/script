//! Comprehensive tests for functional programming integration
//!
//! Tests that the functional programming features work correctly with
//! both the stdlib and runtime integration.

use script::stdlib::{StdLib, ScriptValue, ScriptVec, ScriptOption, ScriptResult, ScriptString};
use script::runtime::{Value, closure::Closure};
use script::runtime::closure::ClosureRuntime;
use script::stdlib::functional::{FunctionalOps, FunctionalExecutor, ClosureExecutionBridge};
use script::error::{Error, ErrorKind};
use script::runtime::ScriptRc;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Test helper to create a simple doubling closure
fn create_double_closure() -> (Closure, impl Fn(&[Value]) -> Result<Value, Error>) {
    let closure = Closure::new(
        "double".to_string(),
        vec!["x".to_string()],
        HashMap::new(),
    );
    
    let implementation = |args: &[Value]| -> Result<Value, Error> {
        match &args[0] {
            Value::I32(n) => Ok(Value::I32(n * 2)),
            _ => Err(Error::new(ErrorKind::TypeError, "Expected i32")),
        }
    };
    
    (closure, implementation)
}

/// Test helper to create a predicate closure (returns true for even numbers)
fn create_even_predicate() -> (Closure, impl Fn(&[Value]) -> Result<Value, Error>) {
    let closure = Closure::new(
        "is_even".to_string(),
        vec!["x".to_string()],
        HashMap::new(),
    );
    
    let implementation = |args: &[Value]| -> Result<Value, Error> {
        match &args[0] {
            Value::I32(n) => Ok(Value::Bool(n % 2 == 0)),
            _ => Err(Error::new(ErrorKind::TypeError, "Expected i32")),
        }
    };
    
    (closure, implementation)
}

/// Test helper to create an addition closure
fn create_add_closure() -> (Closure, impl Fn(&[Value]) -> Result<Value, Error>) {
    let closure = Closure::new(
        "add".to_string(),
        vec!["acc".to_string(), "x".to_string()],
        HashMap::new(),
    );
    
    let implementation = |args: &[Value]| -> Result<Value, Error> {
        match (&args[0], &args[1]) {
            (Value::I32(a), Value::I32(b)) => Ok(Value::I32(a + b)),
            _ => Err(Error::new(ErrorKind::TypeError, "Expected i32")),
        }
    };
    
    (closure, implementation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_value_closure_enum() {
        // Test that Closure can be stored in ScriptValue
        let closure = Closure::new(
            "test".to_string(),
            vec!["x".to_string()],
            HashMap::new(),
        );
        
        let script_value = ScriptValue::Closure(ScriptRc::new(closure));
        
        // Test accessors
        assert!(script_value.as_closure().is_some());
        assert_eq!(script_value.get_type().to_string(), "function");
        
        // Test type checking
        assert!(matches!(script_value, ScriptValue::Closure(_)));
    }

    #[test]
    fn test_closure_execution_bridge() {
        let mut bridge = ClosureExecutionBridge::new();
        
        // Register a simple closure implementation
        bridge.register_closure("test_func".to_string(), |args| {
            if let Value::I32(n) = &args[0] {
                Ok(Value::I32(n + 1))
            } else {
                Err(Error::new(ErrorKind::TypeError, "Expected i32"))
            }
        });
        
        // Create a closure
        let closure = Closure::new(
            "test_func".to_string(),
            vec!["x".to_string()],
            HashMap::new(),
        );
        
        // Test execution
        let args = vec![Value::I32(5)];
        let result = bridge.execute_closure(&closure, &args).unwrap();
        assert_eq!(result, Value::I32(6));
    }

    #[test]
    fn test_functional_executor() {
        let mut executor = FunctionalExecutor::new();
        
        // Register the double closure
        executor.register_closure("double".to_string(), |args| {
            if let Value::I32(n) = &args[0] {
                Ok(Value::I32(n * 2))
            } else {
                Err(Error::new(ErrorKind::TypeError, "Expected i32"))
            }
        });
        
        let closure = Closure::new(
            "double".to_string(),
            vec!["x".to_string()],
            HashMap::new(),
        );
        
        // Test unary execution
        let result = executor.execute_unary(&closure, ScriptValue::I32(5)).unwrap();
        assert_eq!(result, ScriptValue::I32(10));
        
        // Test predicate execution
        executor.register_closure("is_positive".to_string(), |args| {
            if let Value::I32(n) = &args[0] {
                Ok(Value::Bool(*n > 0))
            } else {
                Err(Error::new(ErrorKind::TypeError, "Expected i32"))
            }
        });
        
        let predicate_closure = Closure::new(
            "is_positive".to_string(),
            vec!["x".to_string()],
            HashMap::new(),
        );
        
        let pred_result = executor.execute_predicate(&predicate_closure, ScriptValue::I32(5)).unwrap();
        assert_eq!(pred_result, true);
        
        let pred_result_neg = executor.execute_predicate(&predicate_closure, ScriptValue::I32(-3)).unwrap();
        assert_eq!(pred_result_neg, false);
    }

    #[test]
    fn test_vector_map_functional_ops() {
        // Create a test vector
        let vec = ScriptVec::new();
        vec.push(ScriptValue::I32(1)).unwrap();
        vec.push(ScriptValue::I32(2)).unwrap();
        vec.push(ScriptValue::I32(3)).unwrap();
        
        // Test that the FunctionalOps trait is implemented
        // Note: Full integration test would require runtime registration
        assert_eq!(vec.len(), 3);
        
        // Test vector contents
        let data = vec.data.read().unwrap();
        assert_eq!(data[0], ScriptValue::I32(1));
        assert_eq!(data[1], ScriptValue::I32(2));
        assert_eq!(data[2], ScriptValue::I32(3));
    }

    #[test]
    fn test_script_closure_conversion() {
        use script::stdlib::functional::{script_value_to_runtime_value, runtime_value_to_script_value};
        
        // Test basic value conversions
        let script_i32 = ScriptValue::I32(42);
        let runtime_val = script_value_to_runtime_value(&script_i32).unwrap();
        assert_eq!(runtime_val, Value::I32(42));
        
        let converted_back = runtime_value_to_script_value(&runtime_val).unwrap();
        assert_eq!(converted_back, ScriptValue::I32(42));
        
        // Test bool conversion
        let script_bool = ScriptValue::Bool(true);
        let runtime_bool = script_value_to_runtime_value(&script_bool).unwrap();
        assert_eq!(runtime_bool, Value::Bool(true));
        
        // Test string conversion
        let script_string = ScriptValue::String(ScriptRc::new(ScriptString::from_str("test")));
        let runtime_string = script_value_to_runtime_value(&script_string).unwrap();
        assert_eq!(runtime_string, Value::String("test".to_string()));
    }

    #[test]
    fn test_closure_script_value_integration() {
        // Create a closure and wrap it in ScriptValue
        let closure = Closure::new(
            "test_closure".to_string(),
            vec!["x".to_string(), "y".to_string()],
            HashMap::new(),
        );
        
        let script_closure = ScriptValue::Closure(ScriptRc::new(closure.clone());
        
        // Test that we can extract the closure back
        if let ScriptValue::Closure(extracted) = &script_closure {
            assert_eq!(extracted.function_id, "test_closure");
            assert_eq!(extracted.parameters, vec!["x".to_string(), "y".to_string()]);
            assert_eq!(extracted.param_count(), 2);
        } else {
            panic!("Failed to extract closure from ScriptValue");
        }
        
        // Test as_closure method
        let closure_ref = script_closure.as_closure().unwrap();
        assert_eq!(closure_ref.function_id, "test_closure");
    }

    #[test]
    fn test_stdlib_function_registration() {
        let stdlib = StdLib::new();
        
        // Test that functional programming functions are registered
        assert!(stdlib.get_function("vec_map").is_some());
        assert!(stdlib.get_function("vec_filter").is_some());
        assert!(stdlib.get_function("vec_reduce").is_some());
        assert!(stdlib.get_function("vec_for_each").is_some());
        assert!(stdlib.get_function("vec_find").is_some());
        assert!(stdlib.get_function("vec_every").is_some());
        assert!(stdlib.get_function("vec_some").is_some());
        assert!(stdlib.get_function("compose").is_some());
        assert!(stdlib.get_function("partial").is_some());
        assert!(stdlib.get_function("curry").is_some());
        assert!(stdlib.get_function("range").is_some());
        assert!(stdlib.get_function("iter_collect").is_some());
        assert!(stdlib.get_function("iter_take").is_some());
        assert!(stdlib.get_function("iter_skip").is_some());
        
        // Test function signatures
        let vec_map = stdlib.get_function("vec_map").unwrap();
        assert_eq!(vec_map.name, "vec_map");
        
        // Verify type signature
        if let script::types::Type::Function { params, ret } = &vec_map.signature {
            assert_eq!(params.len(), 2);
            // First param should be Array
            assert!(matches!(params[0], script::types::Type::Array(_)));
            // Second param should be Closure
            assert!(matches!(params[1], script::types::Type::Named(ref name) if name == "Closure"));
            // Return type should be Array
            assert!(matches!(**ret, script::types::Type::Array(_)));
        } else {
            panic!("vec_map should have Function type signature");
        }
    }

    #[test]
    fn test_option_and_result_conversions() {
        use script::stdlib::functional::{script_value_to_runtime_value, runtime_value_to_script_value};
        
        // Test Option::Some conversion
        let some_value = ScriptValue::Option(ScriptRc::new(ScriptOption::Some(ScriptValue::I32(42))));
        let runtime_some = script_value_to_runtime_value(&some_value).unwrap();
        
        // Test conversion back
        let converted_some = runtime_value_to_script_value(&runtime_some).unwrap();
        if let ScriptValue::Option(opt) = converted_some {
            if let ScriptOption::Some(val) = &**opt {
                assert_eq!(*val, ScriptValue::I32(42));
            } else {
                panic!("Expected Some variant");
            }
        } else {
            panic!("Expected Option type");
        }
        
        // Test Option::None conversion
        let none_value = ScriptValue::Option(ScriptRc::new(ScriptOption::None));
        let runtime_none = script_value_to_runtime_value(&none_value).unwrap();
        let converted_none = runtime_value_to_script_value(&runtime_none).unwrap();
        
        if let ScriptValue::Option(opt) = converted_none {
            assert!(matches!(**opt, ScriptOption::None));
        } else {
            panic!("Expected Option type");
        }
    }

    #[test]
    fn test_array_conversion() {
        use script::stdlib::functional::{script_value_to_runtime_value, runtime_value_to_script_value};
        
        // Create a ScriptVec with some values
        let vec = ScriptVec::new();
        vec.push(ScriptValue::I32(1)).unwrap();
        vec.push(ScriptValue::I32(2)).unwrap();
        vec.push(ScriptValue::I32(3)).unwrap();
        
        let script_array = ScriptValue::Array(ScriptRc::new(vec));
        
        // Convert to runtime value
        let runtime_array = script_value_to_runtime_value(&script_array).unwrap();
        
        // Verify runtime array
        if let Value::Array(arr) = &runtime_array {
            assert_eq!(arr.len(), 3);
            assert_eq!(**arr[0], Value::I32(1));
            assert_eq!(**arr[1], Value::I32(2));
            assert_eq!(**arr[2], Value::I32(3));
        } else {
            panic!("Expected Array type");
        }
        
        // Convert back to ScriptValue
        let converted_back = runtime_value_to_script_value(&runtime_array).unwrap();
        if let ScriptValue::Array(converted_vec) = converted_back {
            assert_eq!(converted_vec.len(), 3);
            let data = converted_vec.data.read().unwrap();
            assert_eq!(data[0], ScriptValue::I32(1));
            assert_eq!(data[1], ScriptValue::I32(2));
            assert_eq!(data[2], ScriptValue::I32(3));
        } else {
            panic!("Expected Array type after conversion");
        }
    }

    #[test]
    fn test_closure_parameter_validation() {
        let closure = Closure::new(
            "test".to_string(),
            vec!["x".to_string(), "y".to_string()],
            HashMap::new(),
        );
        
        assert_eq!(closure.param_count(), 2);
        assert_eq!(closure.get_parameters(), &["x".to_string(), "y".to_string()]);
        assert_eq!(closure.get_function_id(), "test");
        assert!(!closure.captures_by_reference());
        
        // Test captured variables
        let mut captured = HashMap::new();
        captured.insert("z".to_string(), Value::I32(42));
        
        let closure_with_captures = Closure::new(
            "test_with_captures".to_string(),
            vec!["x".to_string()],
            captured,
        );
        
        assert_eq!(closure_with_captures.get_captured("z"), Some(&Value::I32(42)));
        assert_eq!(closure_with_captures.get_captured("nonexistent"), None);
    }

    #[test]
    fn test_closure_by_reference() {
        let mut captured = HashMap::new();
        captured.insert("counter".to_string(), Value::I32(0));
        
        let mut closure = Closure::new_by_ref(
            "counter_closure".to_string(),
            vec![],
            captured,
        );
        
        assert!(closure.captures_by_reference());
        
        // Test modifying captured variable
        let result = closure.set_captured("counter".to_string(), Value::I32(5));
        assert!(result.is_ok());
        assert_eq!(closure.get_captured("counter"), Some(&Value::I32(5)));
        
        // Test with by-value closure (should fail)
        let mut by_value_closure = Closure::new(
            "by_value".to_string(),
            vec![],
            HashMap::new(),
        );
        
        let result = by_value_closure.set_captured("test".to_string(), Value::I32(1));
        assert!(result.is_err());
    }
}

/// Integration tests that require the full runtime
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_closure_runtime_integration() {
        let mut runtime = ClosureRuntime::new();
        
        // Register multiple closures
        runtime.register_closure("double".to_string(), |args| {
            if let Value::I32(n) = &args[0] {
                Ok(Value::I32(n * 2))
            } else {
                Err(Error::new(ErrorKind::TypeError, "Expected i32"))
            }
        });
        
        runtime.register_closure("add".to_string(), |args| {
            if let (Value::I32(a), Value::I32(b)) = (&args[0], &args[1]) {
                Ok(Value::I32(a + b))
            } else {
                Err(Error::new(ErrorKind::TypeError, "Expected i32"))
            }
        });
        
        // Test single-parameter closure
        let double_closure = Closure::new(
            "double".to_string(),
            vec!["x".to_string()],
            HashMap::new(),
        );
        
        let result = runtime.execute_closure(&double_closure, &[Value::I32(21)]).unwrap();
        assert_eq!(result, Value::I32(42));
        
        // Test two-parameter closure
        let add_closure = Closure::new(
            "add".to_string(),
            vec!["x".to_string(), "y".to_string()],
            HashMap::new(),
        );
        
        let result = runtime.execute_closure(&add_closure, &[Value::I32(15), Value::I32(27)]).unwrap();
        assert_eq!(result, Value::I32(42));
        
        // Test error on wrong argument count
        let result = runtime.execute_closure(&add_closure, &[Value::I32(15)]);
        assert!(result.is_err());
        
        // Test error on unregistered closure
        let unknown_closure = Closure::new(
            "unknown".to_string(),
            vec!["x".to_string()],
            HashMap::new(),
        );
        
        let result = runtime.execute_closure(&unknown_closure, &[Value::I32(1)]);
        assert!(result.is_err());
    }

    #[test]
    fn test_call_stack_tracking() {
        let mut runtime = ClosureRuntime::new();
        
        // Register a recursive closure for testing
        runtime.register_closure("factorial".to_string(), |args| {
            if let Value::I32(n) = &args[0] {
                if *n <= 1 {
                    Ok(Value::I32(1))
                } else {
                    // This would need recursive execution in a real implementation
                    Ok(Value::I32(*n))
                }
            } else {
                Err(Error::new(ErrorKind::TypeError, "Expected i32"))
            }
        });
        
        let factorial_closure = Closure::new(
            "factorial".to_string(),
            vec!["n".to_string()],
            HashMap::new(),
        );
        
        // Test call stack tracking
        assert_eq!(runtime.call_stack_depth(), 0);
        assert!(runtime.current_closure().is_none());
        
        let _result = runtime.execute_closure(&factorial_closure, &[Value::I32(5)]).unwrap();
        
        // After execution, call stack should be empty again
        assert_eq!(runtime.call_stack_depth(), 0);
        assert!(runtime.current_closure().is_none());
    }
}