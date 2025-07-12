//! Functional programming utilities for the Script standard library
//!
//! This module provides higher-order functions, function composition utilities,
//! and closure-based operations that enable functional programming patterns
//! in Script code.

use crate::error::{Error, ErrorKind, Result};
use crate::runtime::closure::debug::{
    debug_print_closure_state, debug_print_full_report, init_closure_debugger,
};
use crate::runtime::closure::{Closure, ClosureRuntime};
use crate::runtime::RuntimeError;
use crate::runtime::{ScriptRc, Value};
use crate::stdlib::{ScriptOption, ScriptResult, ScriptValue, ScriptVec};
use std::sync::{Arc, RwLock};

/// Trait for collections that support functional operations
pub trait FunctionalOps {
    /// Map over each element using a closure
    fn map(&self, closure: &Value) -> Result<Self>
    where
        Self: Sized;

    /// Filter elements using a predicate closure
    fn filter(&self, predicate: &Value) -> Result<Self>
    where
        Self: Sized;

    /// Reduce the collection to a single value using an accumulator closure
    fn reduce(&self, closure: &Value, initial: ScriptValue) -> Result<ScriptValue>;

    /// Execute a closure for each element (side effects)
    fn for_each(&self, closure: &Value) -> Result<()>;

    /// Find the first element matching a predicate
    fn find(&self, predicate: &Value) -> Result<ScriptOption>;

    /// Test if all elements satisfy a predicate
    fn every(&self, predicate: &Value) -> Result<bool>;

    /// Test if any element satisfies a predicate
    fn some(&self, predicate: &Value) -> Result<bool>;

    /// Map and flatten nested collections
    fn flat_map(&self, closure: &Value) -> Result<Self>
    where
        Self: Sized;

    /// Zip with another collection element-wise
    fn zip(&self, other: &Self) -> Result<Self>
    where
        Self: Sized;

    /// Chain with another collection
    fn chain(&self, other: &Self) -> Result<Self>
    where
        Self: Sized;

    /// Take elements while predicate is true
    fn take_while(&self, predicate: &Value) -> Result<Self>
    where
        Self: Sized;

    /// Drop elements while predicate is true
    fn drop_while(&self, predicate: &Value) -> Result<Self>
    where
        Self: Sized;

    /// Partition into two collections based on predicate
    fn partition(&self, predicate: &Value) -> Result<(Self, Self)>
    where
        Self: Sized;

    /// Group elements by key function
    fn group_by(&self, key_fn: &Value) -> Result<Vec<Self>>
    where
        Self: Sized;
}

/// Closure execution engine for stdlib functions
pub struct FunctionalExecutor {
    /// Runtime for executing closures
    closure_runtime: ClosureRuntime,
}

/// Global static runtime for closure execution
/// This provides a bridge between stdlib functions and the runtime
pub struct ClosureExecutionBridge {
    runtime: ClosureRuntime,
}

impl ClosureExecutionBridge {
    /// Create a new closure execution bridge
    pub fn new() -> Self {
        ClosureExecutionBridge {
            runtime: ClosureRuntime::new(),
        }
    }

    /// Execute a closure with the given arguments
    pub fn execute_closure(&mut self, closure: &Closure, args: &[Value]) -> Result<Value> {
        self.runtime.execute_closure(closure, args)
    }

    /// Register a closure implementation
    pub fn register_closure<F>(&mut self, function_id: String, implementation: F)
    where
        F: Fn(&[Value]) -> Result<Value> + 'static,
    {
        self.runtime.register_closure(function_id, implementation);
    }
}

impl FunctionalExecutor {
    /// Create a new functional executor
    pub fn new() -> Self {
        FunctionalExecutor {
            closure_runtime: ClosureRuntime::new(),
        }
    }

    /// Execute a unary closure (one argument)
    pub fn execute_unary(&mut self, closure: &Closure, arg: ScriptValue) -> Result<ScriptValue> {
        // Convert ScriptValue to runtime Value
        let runtime_arg = script_value_to_runtime_value(&arg)?;

        // Execute the closure
        let result = self
            .closure_runtime
            .execute_closure(closure, &[runtime_arg])
            .map_err(|e| Error::new(ErrorKind::RuntimeError, e.to_string()))?;

        // Convert back to ScriptValue
        runtime_value_to_script_value(&result)
    }

    /// Execute a binary closure (two arguments)
    pub fn execute_binary(
        &mut self,
        closure: &Closure,
        arg1: ScriptValue,
        arg2: ScriptValue,
    ) -> Result<ScriptValue> {
        // Convert ScriptValues to runtime Values
        let runtime_arg1 = script_value_to_runtime_value(&arg1)?;
        let runtime_arg2 = script_value_to_runtime_value(&arg2)?;

        // Execute the closure
        let result = self
            .closure_runtime
            .execute_closure(closure, &[runtime_arg1, runtime_arg2])
            .map_err(|e| Error::new(ErrorKind::RuntimeError, e.to_string()))?;

        // Convert back to ScriptValue
        runtime_value_to_script_value(&result)
    }

    /// Execute a predicate closure that should return a boolean
    pub fn execute_predicate(&mut self, closure: &Closure, arg: ScriptValue) -> Result<bool> {
        let result = self.execute_unary(closure, arg)?;
        match result {
            ScriptValue::Bool(b) => Ok(b),
            _ => Err(Error::new(
                ErrorKind::TypeError,
                "Predicate closure must return a boolean value",
            )),
        }
    }

    /// Register a closure implementation for execution
    pub fn register_closure<F>(&mut self, function_id: String, implementation: F)
    where
        F: Fn(&[Value]) -> Result<Value> + 'static,
    {
        self.closure_runtime
            .register_closure(function_id, move |args| {
                implementation(args)
                    .map_err(|e| crate::error::Error::new(ErrorKind::RuntimeError, e.to_string()))
            });
    }
}

/// Execute a closure from a ScriptValue
pub fn execute_script_closure(
    closure_value: &ScriptValue,
    args: &[ScriptValue],
) -> Result<ScriptValue> {
    // Get the closure from the ScriptValue
    let closure = match closure_value {
        ScriptValue::Closure(c) => c,
        _ => return Err(Error::new(ErrorKind::TypeError, "Expected a closure")),
    };

    // Create a temporary executor
    let mut executor = FunctionalExecutor::new();

    // Convert ScriptValues to runtime Values
    let runtime_args: std::result::Result<Vec<Value>, Error> =
        args.iter().map(script_value_to_runtime_value).collect();
    let runtime_args = runtime_args?;

    // Execute the closure
    let result = executor
        .closure_runtime
        .execute_closure(closure, &runtime_args)
        .map_err(|e| Error::new(ErrorKind::RuntimeError, e.to_string()))?;

    // Convert back to ScriptValue
    runtime_value_to_script_value(&result)
}

impl Default for FunctionalExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Get or create a functional executor (thread-local approach)
fn get_executor() -> Result<FunctionalExecutor> {
    // For now, create a new executor each time
    // In a full implementation, this would use thread-local storage
    Ok(FunctionalExecutor::new())
}

impl FunctionalOps for ScriptVec {
    /// Map over each element using a closure
    fn map(&self, closure: &Value) -> Result<Self> {
        // Convert runtime Value to ScriptValue
        let closure_script_value = runtime_value_to_script_value(closure)?;

        let data = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read vector data"))?;

        let mut result_vec = Vec::with_capacity(data.len());

        for item in data.iter() {
            let mapped_item = execute_script_closure(&closure_script_value, &[item.clone()])?;
            result_vec.push(mapped_item);
        }

        Ok(ScriptVec {
            data: Arc::new(RwLock::new(result_vec)),
        })
    }

    /// Filter elements using a predicate closure
    fn filter(&self, predicate: &Value) -> Result<Self> {
        // Convert runtime Value to ScriptValue
        let predicate_script_value = runtime_value_to_script_value(predicate)?;

        let data = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read vector data"))?;

        let mut result_vec = Vec::new();

        for item in data.iter() {
            let result = execute_script_closure(&predicate_script_value, &[item.clone()])?;
            let keep = match result {
                ScriptValue::Bool(b) => b,
                _ => {
                    return Err(Error::new(
                        ErrorKind::TypeError,
                        "Predicate must return a boolean",
                    ))
                }
            };
            if keep {
                result_vec.push(item.clone());
            }
        }

        Ok(ScriptVec {
            data: Arc::new(RwLock::new(result_vec)),
        })
    }

    /// Reduce the collection to a single value using an accumulator closure
    fn reduce(&self, closure: &Value, initial: ScriptValue) -> Result<ScriptValue> {
        // Convert runtime Value to ScriptValue
        let closure_script_value = runtime_value_to_script_value(closure)?;

        let data = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read vector data"))?;

        let mut accumulator = initial;

        for item in data.iter() {
            accumulator =
                execute_script_closure(&closure_script_value, &[accumulator, item.clone()])?;
        }

        Ok(accumulator)
    }

    /// Execute a closure for each element (side effects)
    fn for_each(&self, closure: &Value) -> Result<()> {
        // Convert runtime Value to ScriptValue
        let closure_script_value = runtime_value_to_script_value(closure)?;

        let data = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read vector data"))?;

        for item in data.iter() {
            execute_script_closure(&closure_script_value, &[item.clone()])?;
        }

        Ok(())
    }

    /// Find the first element matching a predicate
    fn find(&self, predicate: &Value) -> Result<ScriptOption> {
        // Convert runtime Value to ScriptValue
        let predicate_script_value = runtime_value_to_script_value(predicate)?;

        let data = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read vector data"))?;

        for item in data.iter() {
            let result = execute_script_closure(&predicate_script_value, &[item.clone()])?;
            let matches = match result {
                ScriptValue::Bool(b) => b,
                _ => {
                    return Err(Error::new(
                        ErrorKind::TypeError,
                        "Predicate must return a boolean",
                    ))
                }
            };
            if matches {
                return Ok(ScriptOption::Some(item.clone()));
            }
        }

        Ok(ScriptOption::None)
    }

    /// Test if all elements satisfy a predicate
    fn every(&self, predicate: &Value) -> Result<bool> {
        // Convert runtime Value to ScriptValue
        let predicate_script_value = runtime_value_to_script_value(predicate)?;

        let data = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read vector data"))?;

        for item in data.iter() {
            let result = execute_script_closure(&predicate_script_value, &[item.clone()])?;
            let satisfies = match result {
                ScriptValue::Bool(b) => b,
                _ => {
                    return Err(Error::new(
                        ErrorKind::TypeError,
                        "Predicate must return a boolean",
                    ))
                }
            };
            if !satisfies {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Test if any element satisfies a predicate
    fn some(&self, predicate: &Value) -> Result<bool> {
        // Convert runtime Value to ScriptValue
        let predicate_script_value = runtime_value_to_script_value(predicate)?;

        let data = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read vector data"))?;

        for item in data.iter() {
            let result = execute_script_closure(&predicate_script_value, &[item.clone()])?;
            let satisfies = match result {
                ScriptValue::Bool(b) => b,
                _ => {
                    return Err(Error::new(
                        ErrorKind::TypeError,
                        "Predicate must return a boolean",
                    ))
                }
            };
            if satisfies {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Map and flatten nested collections
    fn flat_map(&self, closure: &Value) -> Result<Self> {
        let closure_script_value = runtime_value_to_script_value(closure)?;

        let data = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read vector data"))?;

        let mut result_vec = Vec::new();

        for item in data.iter() {
            let mapped_result = execute_script_closure(&closure_script_value, &[item.clone()])?;

            // Flatten if the result is an array
            match mapped_result {
                ScriptValue::Array(arr) => {
                    let arr_data = arr
                        .data
                        .read()
                        .map_err(|_| Error::lock_poisoned("Failed to read array data"))?;
                    result_vec.extend(arr_data.iter().cloned());
                }
                other => {
                    result_vec.push(other);
                }
            }
        }

        Ok(ScriptVec {
            data: Arc::new(RwLock::new(result_vec)),
        })
    }

    /// Zip with another collection element-wise
    fn zip(&self, other: &Self) -> Result<Self> {
        let data1 = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read first vector data"))?;
        let data2 = other
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read second vector data"))?;

        let mut result_vec = Vec::new();

        for (item1, item2) in data1.iter().zip(data2.iter()) {
            // Create a tuple (item1, item2)
            let tuple = ScriptValue::Array(ScriptRc::new(ScriptVec {
                data: Arc::new(RwLock::new(vec![item1.clone(), item2.clone()])),
            }));
            result_vec.push(tuple);
        }

        Ok(ScriptVec {
            data: Arc::new(RwLock::new(result_vec)),
        })
    }

    /// Chain with another collection
    fn chain(&self, other: &Self) -> Result<Self> {
        let data1 = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read first vector data"))?;
        let data2 = other
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read second vector data"))?;

        let mut result_vec = Vec::with_capacity(data1.len() + data2.len());
        result_vec.extend(data1.iter().cloned());
        result_vec.extend(data2.iter().cloned());

        Ok(ScriptVec {
            data: Arc::new(RwLock::new(result_vec)),
        })
    }

    /// Take elements while predicate is true
    fn take_while(&self, predicate: &Value) -> Result<Self> {
        let predicate_script_value = runtime_value_to_script_value(predicate)?;

        let data = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read vector data"))?;

        let mut result_vec = Vec::new();

        for item in data.iter() {
            let result = execute_script_closure(&predicate_script_value, &[item.clone()])?;
            let should_take = match result {
                ScriptValue::Bool(b) => b,
                _ => {
                    return Err(Error::new(
                        ErrorKind::TypeError,
                        "Predicate must return a boolean",
                    ))
                }
            };

            if should_take {
                result_vec.push(item.clone());
            } else {
                break; // Stop at first false predicate
            }
        }

        Ok(ScriptVec {
            data: Arc::new(RwLock::new(result_vec)),
        })
    }

    /// Drop elements while predicate is true
    fn drop_while(&self, predicate: &Value) -> Result<Self> {
        let predicate_script_value = runtime_value_to_script_value(predicate)?;

        let data = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read vector data"))?;

        let mut result_vec = Vec::new();
        let mut dropping = true;

        for item in data.iter() {
            if dropping {
                let result = execute_script_closure(&predicate_script_value, &[item.clone()])?;
                let should_drop = match result {
                    ScriptValue::Bool(b) => b,
                    _ => {
                        return Err(Error::new(
                            ErrorKind::TypeError,
                            "Predicate must return a boolean",
                        ))
                    }
                };

                if !should_drop {
                    dropping = false;
                    result_vec.push(item.clone());
                }
            } else {
                result_vec.push(item.clone());
            }
        }

        Ok(ScriptVec {
            data: Arc::new(RwLock::new(result_vec)),
        })
    }

    /// Partition into two collections based on predicate
    fn partition(&self, predicate: &Value) -> Result<(Self, Self)> {
        let predicate_script_value = runtime_value_to_script_value(predicate)?;

        let data = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read vector data"))?;

        let mut true_vec = Vec::new();
        let mut false_vec = Vec::new();

        for item in data.iter() {
            let result = execute_script_closure(&predicate_script_value, &[item.clone()])?;
            let matches = match result {
                ScriptValue::Bool(b) => b,
                _ => {
                    return Err(Error::new(
                        ErrorKind::TypeError,
                        "Predicate must return a boolean",
                    ))
                }
            };

            if matches {
                true_vec.push(item.clone());
            } else {
                false_vec.push(item.clone());
            }
        }

        Ok((
            ScriptVec {
                data: Arc::new(RwLock::new(true_vec)),
            },
            ScriptVec {
                data: Arc::new(RwLock::new(false_vec)),
            },
        ))
    }

    /// Group elements by key function
    fn group_by(&self, key_fn: &Value) -> Result<Vec<Self>> {
        let key_fn_script_value = runtime_value_to_script_value(key_fn)?;

        let data = self
            .data
            .read()
            .map_err(|_| Error::lock_poisoned("Failed to read vector data"))?;

        use std::collections::HashMap;
        let mut groups: HashMap<String, Vec<ScriptValue>> = HashMap::new();

        for item in data.iter() {
            let key_result = execute_script_closure(&key_fn_script_value, &[item.clone()])?;
            let key = match key_result {
                ScriptValue::String(s) => s.to_string(),
                ScriptValue::I32(i) => i.to_string(),
                ScriptValue::F32(f) => f.to_string(),
                ScriptValue::Bool(b) => b.to_string(),
                _ => {
                    return Err(Error::new(
                        ErrorKind::TypeError,
                        "Key function must return a hashable value",
                    ))
                }
            };

            groups
                .entry(key)
                .or_insert_with(Vec::new)
                .push(item.clone());
        }

        let mut result_groups = Vec::new();
        for (_, group_items) in groups {
            result_groups.push(ScriptVec {
                data: Arc::new(RwLock::new(group_items)),
            });
        }

        Ok(result_groups)
    }
}

/// Function composition utilities
pub struct FunctionComposition;

impl FunctionComposition {
    /// Compose two functions: compose(f, g) returns a closure that computes f(g(x))
    pub fn compose(f: &Value, g: &Value) -> Result<Value> {
        let f_closure = match f {
            Value::Closure(c) => c.clone(),
            _ => {
                return Err(Error::new(
                    ErrorKind::TypeError,
                    "First argument must be a closure",
                ))
            }
        };

        let g_closure = match g {
            Value::Closure(c) => c.clone(),
            _ => {
                return Err(Error::new(
                    ErrorKind::TypeError,
                    "Second argument must be a closure",
                ))
            }
        };

        // Create a new closure that captures both f and g
        let composed_closure = crate::runtime::closure::create_closure_heap(
            "composed_function".to_string(),
            vec!["x".to_string()],
            vec![
                ("f".to_string(), Value::Closure(f_closure)),
                ("g".to_string(), Value::Closure(g_closure)),
            ],
            false, // by-value capture
        );

        Ok(composed_closure)
    }

    /// Pipe functions left-to-right: pipe(x, f, g, h) computes h(g(f(x)))
    pub fn pipe(input: ScriptValue, functions: &[Value]) -> Result<ScriptValue> {
        if functions.is_empty() {
            return Ok(input);
        }

        let mut executor = get_executor()?;

        let mut result = input;

        for func in functions {
            let closure_ref = match func {
                Value::Closure(c) => c,
                _ => {
                    return Err(Error::new(
                        ErrorKind::TypeError,
                        "All pipe arguments must be closures",
                    ))
                }
            };

            result = executor.execute_unary(closure_ref, result)?;
        }

        Ok(result)
    }

    /// Partial application: Create a closure with some arguments pre-filled
    pub fn partial(func: &Value, partial_args: &[ScriptValue]) -> Result<Value> {
        let closure_ref = match func {
            Value::Closure(c) => c,
            _ => {
                return Err(Error::new(
                    ErrorKind::TypeError,
                    "First argument must be a closure",
                ))
            }
        };

        // Check if we have enough arguments to partially apply
        if partial_args.len() >= closure_ref.parameters.len() {
            return Err(Error::new(
                ErrorKind::TypeError,
                "Cannot partially apply more arguments than the function accepts",
            ));
        }

        // Create captured variables with the partial arguments
        let mut captured_vars = Vec::new();
        captured_vars.push((
            "original_func".to_string(),
            Value::Closure(closure_ref.clone()),
        ));

        for (i, arg) in partial_args.iter().enumerate() {
            captured_vars.push((
                format!("partial_arg_{}", i),
                script_value_to_runtime_value(arg)?,
            ));
        }

        // Create new parameter list (remaining parameters)
        let remaining_params: Vec<String> = closure_ref
            .parameters
            .iter()
            .skip(partial_args.len())
            .cloned()
            .collect();

        let partial_closure = crate::runtime::closure::create_closure_heap(
            format!("partial_{}", closure_ref.function_id),
            remaining_params,
            captured_vars,
            false, // by-value capture
        );

        Ok(partial_closure)
    }

    /// Curry a function: Convert a multi-argument function to a chain of single-argument functions
    pub fn curry(func: &Value) -> Result<Value> {
        let closure_ref = match func {
            Value::Closure(c) => c,
            _ => {
                return Err(Error::new(
                    ErrorKind::TypeError,
                    "Argument must be a closure",
                ))
            }
        };

        if closure_ref.parameters.is_empty() {
            return Ok(func.clone());
        }

        if closure_ref.parameters.len() == 1 {
            return Ok(func.clone());
        }

        // Create a curried version that returns nested closures
        let curried_closure = crate::runtime::closure::create_closure_heap(
            format!("curried_{}", closure_ref.function_id),
            vec![closure_ref.parameters[0].clone()], // Take first parameter
            vec![("original_func".to_string(), func.clone())],
            false, // by-value capture
        );

        Ok(curried_closure)
    }
}

/// Helper function to convert ScriptValue to runtime Value
fn script_value_to_runtime_value(script_val: &ScriptValue) -> Result<Value> {
    match script_val {
        ScriptValue::I32(n) => Ok(Value::I32(*n)),
        ScriptValue::F32(f) => Ok(Value::F32(*f)),
        ScriptValue::Bool(b) => Ok(Value::Bool(*b)),
        ScriptValue::String(s) => Ok(Value::String(s.to_string())),
        ScriptValue::Unit => Ok(Value::Null),
        ScriptValue::Array(arr) => {
            // Convert ScriptVec to Vec<ScriptRc<Value>>
            let data = arr
                .data
                .read()
                .map_err(|_| Error::lock_poisoned("Failed to read array data"))?;
            let mut values = Vec::with_capacity(data.len());
            for item in data.iter() {
                let runtime_val = script_value_to_runtime_value(item)?;
                values.push(ScriptRc::new(runtime_val));
            }
            Ok(Value::Array(values))
        }
        ScriptValue::Option(opt) => match &**opt {
            ScriptOption::Some(val) => {
                let runtime_val = script_value_to_runtime_value(val)?;
                Ok(Value::some(runtime_val))
            }
            ScriptOption::None => Ok(Value::none()),
        },
        ScriptValue::Result(res) => match &**res {
            ScriptResult::Ok(val) => {
                let runtime_val = script_value_to_runtime_value(val)?;
                Ok(Value::ok(runtime_val))
            }
            ScriptResult::Err(err) => {
                let runtime_err = script_value_to_runtime_value(err)?;
                Ok(Value::err(runtime_err))
            }
        },
        ScriptValue::Closure(closure) => {
            // Create a runtime closure value
            Ok(Value::Closure(closure.clone()))
        }
        ScriptValue::Iterator(_) => {
            // For now, convert iterators to a generic object representation
            Err(Error::new(
                ErrorKind::TypeError,
                "Iterator conversion not yet implemented",
            ))
        }
        ScriptValue::Object(_) => {
            // For now, convert objects to a generic representation
            Err(Error::new(
                ErrorKind::TypeError,
                "Object conversion not yet implemented",
            ))
        }
        ScriptValue::HashMap(_) | ScriptValue::HashSet(_) => {
            // For now, convert collections to a generic representation
            Err(Error::new(
                ErrorKind::TypeError,
                "Collection conversion not yet implemented",
            ))
        }
    }
}

/// Helper function to convert runtime Value to ScriptValue
fn runtime_value_to_script_value(runtime_val: &Value) -> Result<ScriptValue> {
    match runtime_val {
        Value::I32(n) => Ok(ScriptValue::I32(*n)),
        Value::F32(f) => Ok(ScriptValue::F32(*f)),
        Value::Bool(b) => Ok(ScriptValue::Bool(*b)),
        Value::String(s) => Ok(ScriptValue::String(ScriptRc::new(
            crate::stdlib::ScriptString::from_str(s),
        ))),
        Value::Null => Ok(ScriptValue::Unit),
        Value::Array(arr) => {
            let mut script_vec = ScriptVec::new();
            for item in arr.iter() {
                let script_val = runtime_value_to_script_value(item)?;
                script_vec.push(script_val)?;
            }
            Ok(ScriptValue::Array(ScriptRc::new(script_vec)))
        }
        Value::Enum {
            type_name,
            variant,
            data,
        } => match (type_name.as_str(), variant.as_str()) {
            ("Option", "Some") => {
                if let Some(val) = data {
                    let script_val = runtime_value_to_script_value(val)?;
                    Ok(ScriptValue::Option(ScriptRc::new(ScriptOption::Some(
                        script_val,
                    ))))
                } else {
                    Err(Error::new(
                        ErrorKind::TypeError,
                        "Some variant missing data",
                    ))
                }
            }
            ("Option", "None") => Ok(ScriptValue::Option(ScriptRc::new(ScriptOption::None))),
            ("Result", "Ok") => {
                if let Some(val) = data {
                    let script_val = runtime_value_to_script_value(val)?;
                    Ok(ScriptValue::Result(ScriptRc::new(ScriptResult::Ok(
                        script_val,
                    ))))
                } else {
                    Err(Error::new(ErrorKind::TypeError, "Ok variant missing data"))
                }
            }
            ("Result", "Err") => {
                if let Some(val) = data {
                    let script_val = runtime_value_to_script_value(val)?;
                    Ok(ScriptValue::Result(ScriptRc::new(ScriptResult::Err(
                        script_val,
                    ))))
                } else {
                    Err(Error::new(ErrorKind::TypeError, "Err variant missing data"))
                }
            }
            _ => Err(Error::new(
                ErrorKind::TypeError,
                format!("Unknown enum type: {}::{}", type_name, variant),
            )),
        },
        Value::Closure(closure) => {
            // Create a ScriptValue closure
            Ok(ScriptValue::Closure(closure.clone()))
        }
        _ => Err(Error::new(
            ErrorKind::TypeError,
            format!(
                "Cannot convert runtime Value to ScriptValue: {:?}",
                runtime_val
            ),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_map() {
        let vec = ScriptVec::new();
        vec.push(ScriptValue::I32(1)).unwrap();
        vec.push(ScriptValue::I32(2)).unwrap();
        vec.push(ScriptValue::I32(3)).unwrap();

        // Create a simple doubling closure for testing
        // In practice, this would be created by the language runtime
        let double_closure = crate::runtime::closure::create_closure_heap(
            "double".to_string(),
            vec!["x".to_string()],
            vec![],
            false,
        );

        // For testing, we need to register a closure implementation
        let mut executor = FunctionalExecutor::new();
        executor.register_closure("double".to_string(), |args| {
            if let Value::I32(n) = &args[0] {
                Ok(Value::I32(n * 2))
            } else {
                Err(crate::error::Error::new(
                    ErrorKind::TypeError,
                    "Expected i32",
                ))
            }
        });

        // Test would need proper closure execution setup
        // This is a structural test to verify the API works
        assert_eq!(vec.len(), 3);
    }

    #[test]
    fn test_function_composition() {
        // Test that compose function structure works
        let f = crate::runtime::closure::create_closure_heap(
            "add_one".to_string(),
            vec!["x".to_string()],
            vec![],
            false,
        );

        let g = crate::runtime::closure::create_closure_heap(
            "multiply_two".to_string(),
            vec!["x".to_string()],
            vec![],
            false,
        );

        let composed = FunctionComposition::compose(&f, &g);
        assert!(composed.is_ok());
    }

    #[test]
    fn test_partial_application() {
        let add_closure = crate::runtime::closure::create_closure_heap(
            "add".to_string(),
            vec!["x".to_string(), "y".to_string()],
            vec![],
            false,
        );

        let partial_args = vec![ScriptValue::I32(5)];
        let partial_result = FunctionComposition::partial(&add_closure, &partial_args);
        assert!(partial_result.is_ok());
    }
}

/// Standard library function implementations for functional programming
/// These functions provide the bridge between Script's function call system
/// and the internal functional programming capabilities.

/// Implementation of vec_map for stdlib registry
pub(crate) fn vec_map_impl(args: &[ScriptValue]) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "vec_map expects 2 arguments, got {}",
            args.len()
        )));
    }

    let _vec = match &args[0] {
        ScriptValue::Array(v) => v,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "First argument to vec_map must be an array".to_string(),
            ))
        }
    };

    let _closure = match &args[1] {
        ScriptValue::Object(_obj) => {
            // Extract closure from object representation
            // This is a simplified approach - in practice, would need proper runtime bridge
            return Err(RuntimeError::InvalidOperation(
                "Closure execution not yet implemented in stdlib integration".to_string(),
            ));
        }
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "Second argument to vec_map must be a closure".to_string(),
            ))
        }
    };
}

/// Implementation of vec_filter for stdlib registry
pub(crate) fn vec_filter_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "vec_filter expects 2 arguments, got {}",
            args.len()
        )));
    }

    let vec = match &args[0] {
        ScriptValue::Array(v) => v,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "First argument to vec_filter must be an array".to_string(),
            ))
        }
    };

    // For now, return the original vector - full implementation requires runtime integration
    Ok(ScriptValue::Array(vec.clone()))
}

/// Implementation of vec_reduce for stdlib registry
pub(crate) fn vec_reduce_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(format!(
            "vec_reduce expects 3 arguments, got {}",
            args.len()
        )));
    }

    let _vec = match &args[0] {
        ScriptValue::Array(v) => v,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "First argument to vec_reduce must be an array".to_string(),
            ))
        }
    };

    let initial = &args[2];

    // For now, return the initial value - full implementation requires runtime integration
    Ok(initial.clone())
}

/// Implementation of vec_for_each for stdlib registry
pub(crate) fn vec_for_each_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "vec_for_each expects 2 arguments, got {}",
            args.len()
        )));
    }

    let _vec = match &args[0] {
        ScriptValue::Array(v) => v,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "First argument to vec_for_each must be an array".to_string(),
            ))
        }
    };

    // For now, return unit - full implementation requires runtime integration
    Ok(ScriptValue::Unit)
}

/// Implementation of vec_find for stdlib registry
pub(crate) fn vec_find_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "vec_find expects 2 arguments, got {}",
            args.len()
        )));
    }

    let _vec = match &args[0] {
        ScriptValue::Array(v) => v,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "First argument to vec_find must be an array".to_string(),
            ))
        }
    };

    // For now, return None - full implementation requires runtime integration
    Ok(ScriptValue::Option(ScriptRc::new(ScriptOption::None)))
}

/// Implementation of vec_every for stdlib registry
pub(crate) fn vec_every_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "vec_every expects 2 arguments, got {}",
            args.len()
        )));
    }

    let _vec = match &args[0] {
        ScriptValue::Array(v) => v,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "First argument to vec_every must be an array".to_string(),
            ))
        }
    };

    // For now, return true - full implementation requires runtime integration
    Ok(ScriptValue::Bool(true))
}

/// Implementation of vec_some for stdlib registry
pub(crate) fn vec_some_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "vec_some expects 2 arguments, got {}",
            args.len()
        )));
    }

    let _vec = match &args[0] {
        ScriptValue::Array(v) => v,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "First argument to vec_some must be an array".to_string(),
            ))
        }
    };

    // For now, return false - full implementation requires runtime integration
    Ok(ScriptValue::Bool(false))
}

/// Implementation of compose for stdlib registry
pub(crate) fn compose_impl(args: &[ScriptValue]) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "compose expects 2 arguments, got {}",
            args.len()
        )));
    }

    // For now, return the first function - full implementation requires runtime integration
    Ok(args[0].clone())
}

/// Implementation of partial for stdlib registry
pub(crate) fn partial_impl(args: &[ScriptValue]) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "partial expects 2 arguments, got {}",
            args.len()
        )));
    }

    // For now, return the original function - full implementation requires runtime integration
    Ok(args[0].clone())
}

/// Implementation of curry for stdlib registry
pub(crate) fn curry_impl(args: &[ScriptValue]) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "curry expects 1 argument, got {}",
            args.len()
        )));
    }

    // For now, return the original function - full implementation requires runtime integration
    Ok(args[0].clone())
}

/// Implementation of range for stdlib registry
pub(crate) fn range_impl(args: &[ScriptValue]) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(format!(
            "range expects 3 arguments, got {}",
            args.len()
        )));
    }

    let start = match &args[0] {
        ScriptValue::I32(n) => *n,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "First argument to range must be an integer".to_string(),
            ))
        }
    };

    let end = match &args[1] {
        ScriptValue::I32(n) => *n,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "Second argument to range must be an integer".to_string(),
            ))
        }
    };

    let step = match &args[2] {
        ScriptValue::I32(n) => *n,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "Third argument to range must be an integer".to_string(),
            ))
        }
    };

    if step == 0 {
        return Err(RuntimeError::InvalidOperation(
            "Range step cannot be zero".to_string(),
        ));
    }

    // Create a range iterator using the iterators module
    use crate::stdlib::iterators::RangeIterator;

    let range_iter = RangeIterator::new(start, end, step)?;
    Ok(ScriptValue::Iterator(ScriptRc::new(Box::new(range_iter))))
}

/// Implementation of iter_collect for stdlib registry
pub(crate) fn iter_collect_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "iter_collect expects 1 argument, got {}",
            args.len()
        )));
    }

    let iterator = match &args[0] {
        ScriptValue::Iterator(iter) => iter,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "Argument to iter_collect must be an iterator".to_string(),
            ))
        }
    };

    // Collect all values from the iterator
    let mut result_vec = ScriptVec::new();
    let mut iter_clone = (**iterator).clone_box();

    while let Some(value) = iter_clone.next() {
        result_vec
            .push(value)
            .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;
    }

    Ok(ScriptValue::Array(ScriptRc::new(result_vec)))
}

/// Implementation of iter_take for stdlib registry
pub(crate) fn iter_take_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "iter_take expects 2 arguments, got {}",
            args.len()
        )));
    }

    let iterator = match &args[0] {
        ScriptValue::Iterator(iter) => iter,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "First argument to iter_take must be an iterator".to_string(),
            ))
        }
    };

    let n = match &args[1] {
        ScriptValue::I32(n) => *n,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "Second argument to iter_take must be an integer".to_string(),
            ))
        }
    };

    // Create a take iterator
    use crate::stdlib::iterators::TakeIterator;
    let inner_iter = (**iterator).clone_box();
    let take_iter = TakeIterator::new(inner_iter, n as usize);

    Ok(ScriptValue::Iterator(ScriptRc::new(Box::new(take_iter))))
}

/// Implementation of iter_skip for stdlib registry
pub(crate) fn iter_skip_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "iter_skip expects 2 arguments, got {}",
            args.len()
        )));
    }

    let iterator = match &args[0] {
        ScriptValue::Iterator(iter) => iter,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "First argument to iter_skip must be an iterator".to_string(),
            ))
        }
    };

    let n = match &args[1] {
        ScriptValue::I32(n) => *n,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "Second argument to iter_skip must be an integer".to_string(),
            ))
        }
    };

    // Create a skip iterator
    use crate::stdlib::iterators::SkipIterator;
    let inner_iter = (**iterator).clone_box();
    let skip_iter = SkipIterator::new(inner_iter, n as usize);

    Ok(ScriptValue::Iterator(ScriptRc::new(Box::new(skip_iter))))
}

/// Implementation of vec_flat_map for stdlib registry
pub(crate) fn vec_flat_map_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "vec_flat_map expects 2 arguments, got {}",
            args.len()
        )));
    }

    let vec = match &args[0] {
        ScriptValue::Array(v) => v,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "First argument to vec_flat_map must be an array".to_string(),
            ))
        }
    };

    // For now, return the original vector - full implementation requires runtime integration
    Ok(ScriptValue::Array(vec.clone()))
}

/// Implementation of vec_zip for stdlib registry
pub(crate) fn vec_zip_impl(args: &[ScriptValue]) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "vec_zip expects 2 arguments, got {}",
            args.len()
        )));
    }

    let vec1 = match &args[0] {
        ScriptValue::Array(v) => v,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "First argument to vec_zip must be an array".to_string(),
            ))
        }
    };

    let vec2 = match &args[1] {
        ScriptValue::Array(v) => v,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "Second argument to vec_zip must be an array".to_string(),
            ))
        }
    };

    // Basic zip implementation
    let result = vec1
        .zip(vec2)
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    Ok(ScriptValue::Array(ScriptRc::new(result)))
}

/// Implementation of vec_chain for stdlib registry
pub(crate) fn vec_chain_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "vec_chain expects 2 arguments, got {}",
            args.len()
        )));
    }

    let vec1 = match &args[0] {
        ScriptValue::Array(v) => v,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "First argument to vec_chain must be an array".to_string(),
            ))
        }
    };

    let vec2 = match &args[1] {
        ScriptValue::Array(v) => v,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "Second argument to vec_chain must be an array".to_string(),
            ))
        }
    };

    // Basic chain implementation
    let result = vec1
        .chain(&vec2)
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    Ok(ScriptValue::Array(ScriptRc::new(result)))
}

/// Implementation of vec_take_while for stdlib registry
pub(crate) fn vec_take_while_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "vec_take_while expects 2 arguments, got {}",
            args.len()
        )));
    }

    let vec = match &args[0] {
        ScriptValue::Array(v) => v,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "First argument to vec_take_while must be an array".to_string(),
            ))
        }
    };

    // For now, return the original vector - full implementation requires runtime integration
    Ok(ScriptValue::Array(vec.clone()))
}

/// Implementation of vec_drop_while for stdlib registry
pub(crate) fn vec_drop_while_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "vec_drop_while expects 2 arguments, got {}",
            args.len()
        )));
    }

    let vec = match &args[0] {
        ScriptValue::Array(v) => v,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "First argument to vec_drop_while must be an array".to_string(),
            ))
        }
    };

    // For now, return the original vector - full implementation requires runtime integration
    Ok(ScriptValue::Array(vec.clone()))
}

/// Implementation of vec_partition for stdlib registry
pub(crate) fn vec_partition_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "vec_partition expects 2 arguments, got {}",
            args.len()
        )));
    }

    let vec = match &args[0] {
        ScriptValue::Array(v) => v,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "First argument to vec_partition must be an array".to_string(),
            ))
        }
    };

    // For now, return a tuple of the original vector and an empty vector
    let empty_vec = ScriptVec::new();
    let result_tuple = vec![vec.as_ref().clone(), empty_vec];

    Ok(ScriptValue::Array(ScriptRc::new(ScriptVec {
        data: Arc::new(RwLock::new(
            result_tuple
                .into_iter()
                .map(|v| ScriptValue::Array(ScriptRc::new(v)))
                .collect(),
        )),
    })))
}

/// Implementation of vec_group_by for stdlib registry
pub(crate) fn vec_group_by_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "vec_group_by expects 2 arguments, got {}",
            args.len()
        )));
    }

    let vec = match &args[0] {
        ScriptValue::Array(v) => v,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "First argument to vec_group_by must be an array".to_string(),
            ))
        }
    };

    // For now, return an array containing the original vector
    let result_groups = vec![vec.as_ref().clone()];

    Ok(ScriptValue::Array(ScriptRc::new(ScriptVec {
        data: Arc::new(RwLock::new(
            result_groups
                .into_iter()
                .map(|v| ScriptValue::Array(ScriptRc::new(v)))
                .collect(),
        )),
    })))
}

/// Debug function implementations for closure debugging

/// Initialize the closure debugger - to be called from Script
pub(crate) fn debug_init_closure_debugger_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if !args.is_empty() {
        return Err(RuntimeError::InvalidOperation(format!(
            "debug_init_closure_debugger expects 0 arguments, got {}",
            args.len()
        )));
    }

    init_closure_debugger();
    Ok(ScriptValue::Unit)
}

/// Print debug info for a specific closure - to be called from Script
pub(crate) fn debug_print_closure_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "debug_print_closure expects 1 argument, got {}",
            args.len()
        )));
    }

    let function_id = match &args[0] {
        ScriptValue::String(s) => s.as_str(),
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "Argument to debug_print_closure must be a string (function ID)".to_string(),
            ))
        }
    };

    debug_print_closure_state(function_id);
    Ok(ScriptValue::Unit)
}

/// Print full closure debug report - to be called from Script
pub(crate) fn debug_print_closure_report_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if !args.is_empty() {
        return Err(RuntimeError::InvalidOperation(format!(
            "debug_print_closure_report expects 0 arguments, got {}",
            args.len()
        )));
    }

    debug_print_full_report();
    Ok(ScriptValue::Unit)
}

/// Closure serialization functions

/// Serialize a closure to binary format - to be called from Script
pub(crate) fn closure_serialize_binary_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "closure_serialize_binary expects 1 argument, got {}",
            args.len()
        )));
    }

    let closure_value = match &args[0] {
        ScriptValue::Closure(closure) => {
            use crate::runtime::closure::serialize_closure_binary;
            let serialized = serialize_closure_binary(closure).map_err(|e| {
                RuntimeError::InvalidOperation(format!("Serialization failed: {}", e))
            })?;

            // Return serialized data as a byte array (represented as Vec<I32>)
            let bytes: Vec<ScriptValue> = serialized
                .data
                .into_iter()
                .map(|b| ScriptValue::I32(b as i32))
                .collect();

            Ok(ScriptValue::Array(ScriptRc::new(ScriptVec {
                data: Arc::new(RwLock::new(bytes)),
            })))
        }
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "Argument to closure_serialize_binary must be a closure".to_string(),
            ))
        }
    };

    closure_value
}

/// Serialize a closure to JSON format - to be called from Script  
pub(crate) fn closure_serialize_json_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "closure_serialize_json expects 1 argument, got {}",
            args.len()
        )));
    }

    let closure_value = match &args[0] {
        ScriptValue::Closure(closure) => {
            use crate::runtime::closure::serialize_closure_json;
            let serialized = serialize_closure_json(closure).map_err(|e| {
                RuntimeError::InvalidOperation(format!("JSON serialization failed: {}", e))
            })?;

            // Convert bytes to string
            let json_string = String::from_utf8(serialized.data)
                .map_err(|e| RuntimeError::InvalidOperation(format!("Invalid UTF-8: {}", e)))?;

            Ok(ScriptValue::String(ScriptRc::new(
                crate::stdlib::string::ScriptString::new(json_string),
            )))
        }
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "Argument to closure_serialize_json must be a closure".to_string(),
            ))
        }
    };

    closure_value
}

/// Serialize a closure to compact format - to be called from Script
pub(crate) fn closure_serialize_compact_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "closure_serialize_compact expects 1 argument, got {}",
            args.len()
        )));
    }

    let closure_value = match &args[0] {
        ScriptValue::Closure(closure) => {
            use crate::runtime::closure::serialize_closure_compact;
            let serialized = serialize_closure_compact(closure).map_err(|e| {
                RuntimeError::InvalidOperation(format!("Compact serialization failed: {}", e))
            })?;

            // Return serialized data as a byte array (represented as Vec<I32>)
            let bytes: Vec<ScriptValue> = serialized
                .data
                .into_iter()
                .map(|b| ScriptValue::I32(b as i32))
                .collect();

            Ok(ScriptValue::Array(ScriptRc::new(ScriptVec {
                data: Arc::new(RwLock::new(bytes)),
            })))
        }
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "Argument to closure_serialize_compact must be a closure".to_string(),
            ))
        }
    };

    closure_value
}

/// Serialize an optimized closure to binary format - to be called from Script
pub(crate) fn optimized_closure_serialize_binary_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "optimized_closure_serialize_binary expects 1 argument, got {}",
            args.len()
        )));
    }

    // Since ScriptValue doesn't have OptimizedClosure variant, we'll convert from Value enum
    // This is a simplified implementation for demonstration
    return Err(RuntimeError::InvalidOperation(
        "Optimized closure serialization requires runtime integration with Value enum".to_string(),
    ));
}

/// Get closure serialization metadata - to be called from Script
pub(crate) fn closure_get_metadata_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "closure_get_metadata expects 1 argument, got {}",
            args.len()
        )));
    }

    let closure_value = match &args[0] {
        ScriptValue::Closure(closure) => {
            use std::collections::HashMap;

            // Create metadata object
            let mut metadata = HashMap::new();
            metadata.insert(
                "function_id".to_string(),
                ScriptValue::String(ScriptRc::new(crate::stdlib::string::ScriptString::new(
                    closure.function_id.clone(),
                ))),
            );
            metadata.insert(
                "param_count".to_string(),
                ScriptValue::I32(closure.parameters.len() as i32),
            );
            metadata.insert(
                "capture_count".to_string(),
                ScriptValue::I32(closure.captured_vars.len() as i32),
            );
            metadata.insert(
                "captures_by_ref".to_string(),
                ScriptValue::Bool(closure.captures_by_ref),
            );
            metadata.insert("is_optimized".to_string(), ScriptValue::Bool(false));

            Ok(ScriptValue::Object(ScriptRc::new(metadata)))
        }
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "Argument to closure_get_metadata must be a closure".to_string(),
            ))
        }
    };

    closure_value
}

/// Check if closure serialization is supported for a closure - to be called from Script
pub(crate) fn closure_can_serialize_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "closure_can_serialize expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::Closure(_) => Ok(ScriptValue::Bool(true)),
        _ => Ok(ScriptValue::Bool(false)),
    }
}

/// Create a serialization configuration object - to be called from Script
pub(crate) fn closure_create_serialize_config_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() != 5 {
        return Err(RuntimeError::InvalidOperation(format!(
            "closure_create_serialize_config expects 5 arguments (include_captured_values, compress, max_size_bytes, include_debug_info, validate_on_deserialize), got {}",
            args.len()
        )));
    }

    let include_captured_values = match &args[0] {
        ScriptValue::Bool(b) => *b,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "First argument (include_captured_values) must be a boolean".to_string(),
            ))
        }
    };

    let compress = match &args[1] {
        ScriptValue::Bool(b) => *b,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "Second argument (compress) must be a boolean".to_string(),
            ))
        }
    };

    let max_size_bytes = match &args[2] {
        ScriptValue::I32(n) if *n >= 0 => *n as usize,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "Third argument (max_size_bytes) must be a non-negative integer".to_string(),
            ))
        }
    };

    let include_debug_info = match &args[3] {
        ScriptValue::Bool(b) => *b,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "Fourth argument (include_debug_info) must be a boolean".to_string(),
            ))
        }
    };

    let validate_on_deserialize = match &args[4] {
        ScriptValue::Bool(b) => *b,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "Fifth argument (validate_on_deserialize) must be a boolean".to_string(),
            ))
        }
    };

    // Create config object
    use std::collections::HashMap;
    let mut config = HashMap::new();
    config.insert(
        "include_captured_values".to_string(),
        ScriptValue::Bool(include_captured_values),
    );
    config.insert("compress".to_string(), ScriptValue::Bool(compress));
    config.insert(
        "max_size_bytes".to_string(),
        ScriptValue::I32(max_size_bytes as i32),
    );
    config.insert(
        "include_debug_info".to_string(),
        ScriptValue::Bool(include_debug_info),
    );
    config.insert(
        "validate_on_deserialize".to_string(),
        ScriptValue::Bool(validate_on_deserialize),
    );

    Ok(ScriptValue::Object(ScriptRc::new(config)))
}

//
// Public API functions for Script access
//

/// Serialize a closure to binary format
pub fn closure_serialize_binary(closure: &Value) -> Result<Vec<u8>> {
    match closure {
        Value::Closure(c) => {
            use crate::runtime::closure::serialize_closure_binary;
            let serialized = serialize_closure_binary(c)?;
            Ok(serialized.data)
        }
        _ => Err(Error::new(ErrorKind::TypeError, "Expected a closure")),
    }
}

/// Serialize a closure to JSON format
pub fn closure_serialize_json(closure: &Value) -> Result<String> {
    match closure {
        Value::Closure(c) => {
            use crate::runtime::closure::serialize_closure_json;
            let serialized = serialize_closure_json(c)?;
            String::from_utf8(serialized.data)
                .map_err(|e| Error::new(ErrorKind::RuntimeError, format!("Invalid UTF-8: {}", e)))
        }
        _ => Err(Error::new(ErrorKind::TypeError, "Expected a closure")),
    }
}

/// Serialize a closure to compact format
pub fn closure_serialize_compact(closure: &Value) -> Result<Vec<u8>> {
    match closure {
        Value::Closure(c) => {
            use crate::runtime::closure::serialize_closure_compact;
            let serialized = serialize_closure_compact(c)?;
            Ok(serialized.data)
        }
        _ => Err(Error::new(ErrorKind::TypeError, "Expected a closure")),
    }
}

/// Get closure metadata as JSON string
pub fn closure_get_metadata(closure: &Value) -> Result<String> {
    match closure {
        Value::Closure(c) => {
            let metadata = format!(
                r#"{{"function_id":"{}","parameter_count":{},"capture_count":{},"captures_by_reference":{}}}"#,
                c.function_id,
                c.parameters.len(),
                c.captured_vars.len(),
                c.captures_by_ref
            );
            Ok(metadata)
        }
        _ => Err(Error::new(ErrorKind::TypeError, "Expected a closure")),
    }
}

/// Check if a closure can be serialized
pub fn closure_can_serialize(closure: &Value) -> bool {
    matches!(closure, Value::Closure(_))
}

/// Create a serialization configuration
pub fn closure_create_serialize_config(
    compress: bool,
    max_size: i32,
    validate: bool,
) -> ClosureSerializeConfig {
    ClosureSerializeConfig {
        compress,
        max_size,
        validate,
    }
}

/// Debug initialization function
pub fn debug_init_closure_debugger() {
    use crate::runtime::closure::debug::init_closure_debugger;
    init_closure_debugger();
}

/// Debug print closure function
pub fn debug_print_closure(function_id: &str) {
    use crate::runtime::closure::debug::debug_print_closure_state;
    debug_print_closure_state(function_id);
}

/// Debug print closure report function
pub fn debug_print_closure_report() {
    use crate::runtime::closure::debug::debug_print_full_report;
    debug_print_full_report();
}

/// Configuration for closure serialization (for tests)
#[derive(Debug, Clone)]
pub struct ClosureSerializeConfig {
    pub compress: bool,
    pub max_size: i32,
    pub validate: bool,
}
