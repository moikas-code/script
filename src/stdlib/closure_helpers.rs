//! Helper functions for using Script-native closures with Result/Option types
//!
//! This module provides utilities to bridge Script closures with functional
//! operations on Result and Option types.

use crate::error::{Error, ErrorKind, Result};
use crate::ir::{FunctionId, Module};
use crate::runtime::closure::{Closure, ClosureRuntime};
use crate::runtime::Value;

/// Represents a closure executor that can run Script closures
pub struct ClosureExecutor<'a> {
    closure_runtime: &'a mut ClosureRuntime,
    module: &'a Module,
}

impl<'a> ClosureExecutor<'a> {
    /// Create a new closure executor
    pub fn new(closure_runtime: &'a mut ClosureRuntime, module: &'a Module) -> Self {
        ClosureExecutor {
            closure_runtime,
            module,
        }
    }

    /// Execute a closure with a single argument
    pub fn execute_unary(&mut self, closure: &Closure, arg: Value) -> Result<Value> {
        // Look up the closure's function in the module
        // FunctionId expects a u32, not a String - need to parse or look up the ID
        // For now, using a placeholder approach
        let func_id = FunctionId(0); // TODO: Proper function ID lookup
        let function = self.module.get_function(func_id).ok_or_else(|| {
            Error::new(
                ErrorKind::RuntimeError,
                format!("Closure function '{}' not found", closure.function_id),
            )
        })?;

        // Verify parameter count
        if closure.parameters.len() != 1 {
            return Err(Error::new(
                ErrorKind::TypeError,
                format!(
                    "Closure expects {} parameters, got 1",
                    closure.parameters.len()
                ),
            ));
        }

        // Set up the environment with captured variables
        let mut env = closure.captured_vars.clone();

        // Add the argument to the environment
        env.insert(closure.parameters[0].clone(), arg.clone());

        // Execute the closure using the runtime
        // The closure runtime expects a slice of Values as arguments
        self.closure_runtime.execute_closure(closure, &[arg])
    }

    /// Execute a closure with two arguments
    pub fn execute_binary(&mut self, closure: &Closure, arg1: Value, arg2: Value) -> Result<Value> {
        // Look up the closure's function in the module
        // FunctionId expects a u32, not a String - need to parse or look up the ID
        // For now, using a placeholder approach
        let func_id = FunctionId(0); // TODO: Proper function ID lookup
        let function = self.module.get_function(func_id).ok_or_else(|| {
            Error::new(
                ErrorKind::RuntimeError,
                format!("Closure function '{}' not found", closure.function_id),
            )
        })?;

        // Verify parameter count
        if closure.parameters.len() != 2 {
            return Err(Error::new(
                ErrorKind::TypeError,
                format!(
                    "Closure expects {} parameters, got 2",
                    closure.parameters.len()
                ),
            ));
        }

        // Set up the environment with captured variables
        let mut env = closure.captured_vars.clone();

        // Add the arguments to the environment
        env.insert(closure.parameters[0].clone(), arg1.clone());
        env.insert(closure.parameters[1].clone(), arg2.clone());

        // Execute the closure using the runtime
        // The closure runtime expects a slice of Values as arguments
        self.closure_runtime.execute_closure(closure, &[arg1, arg2])
    }

    /// Execute a predicate closure that returns a boolean
    pub fn execute_predicate(&mut self, closure: &Closure, arg: Value) -> Result<bool> {
        let result = self.execute_unary(closure, arg)?;
        match result {
            Value::Bool(b) => Ok(b),
            _ => Err(Error::new(
                ErrorKind::TypeError,
                "Predicate closure must return a boolean",
            )),
        }
    }
}

/// Helper trait to allow Result/Option types to work with Script closures
pub trait ScriptClosureAdapter {
    /// Apply a closure to transform a value
    fn apply_closure(&self, closure: &Value, executor: &mut ClosureExecutor) -> Result<Self>
    where
        Self: Sized;

    /// Apply a predicate closure for filtering
    fn test_predicate(&self, closure: &Value, executor: &mut ClosureExecutor) -> Result<bool>;
}

/// Extension methods for ScriptResult to work with closures
impl crate::stdlib::ScriptResult {
    /// Map over the Ok value using a Script closure
    pub fn map_closure(
        &self,
        closure: &Value,
        executor: &mut ClosureExecutor,
    ) -> Result<crate::stdlib::ScriptResult> {
        match closure {
            Value::Closure(closure_ref) => {
                match self {
                    crate::stdlib::ScriptResult::Ok(val) => {
                        // Convert ScriptValue to Value for the closure
                        let runtime_val =
                            crate::runtime::value_conversion::script_value_to_value(val);
                        let mapped = executor.execute_unary(closure_ref, runtime_val)?;
                        // Convert back to ScriptValue
                        let script_val =
                            crate::runtime::value_conversion::value_to_script_value(&mapped)?;
                        Ok(crate::stdlib::ScriptResult::Ok(script_val))
                    }
                    crate::stdlib::ScriptResult::Err(err) => {
                        Ok(crate::stdlib::ScriptResult::Err(err.clone()))
                    }
                }
            }
            _ => Err(Error::new(
                ErrorKind::TypeError,
                "Expected a closure for map operation",
            )),
        }
    }

    /// Map over the Err value using a Script closure
    pub fn map_err_closure(
        &self,
        closure: &Value,
        executor: &mut ClosureExecutor,
    ) -> Result<crate::stdlib::ScriptResult> {
        match closure {
            Value::Closure(closure_ref) => {
                match self {
                    crate::stdlib::ScriptResult::Ok(val) => {
                        Ok(crate::stdlib::ScriptResult::Ok(val.clone()))
                    }
                    crate::stdlib::ScriptResult::Err(err) => {
                        // Convert ScriptValue to Value for the closure
                        let runtime_val =
                            crate::runtime::value_conversion::script_value_to_value(err);
                        let mapped = executor.execute_unary(closure_ref, runtime_val)?;
                        // Convert back to ScriptValue
                        let script_val =
                            crate::runtime::value_conversion::value_to_script_value(&mapped)?;
                        Ok(crate::stdlib::ScriptResult::Err(script_val))
                    }
                }
            }
            _ => Err(Error::new(
                ErrorKind::TypeError,
                "Expected a closure for map_err operation",
            )),
        }
    }

    /// Chain Result operations using a Script closure that returns a Result
    pub fn and_then_closure(
        &self,
        closure: &Value,
        executor: &mut ClosureExecutor,
    ) -> Result<crate::stdlib::ScriptResult> {
        match closure {
            Value::Closure(closure_ref) => {
                match self {
                    crate::stdlib::ScriptResult::Ok(val) => {
                        // Convert ScriptValue to Value for the closure
                        let runtime_val =
                            crate::runtime::value_conversion::script_value_to_value(val);
                        let result = executor.execute_unary(closure_ref, runtime_val)?;

                        // The result should be a Result type
                        // For now, we'll convert and validate
                        let script_val =
                            crate::runtime::value_conversion::value_to_script_value(&result)?;
                        match script_val {
                            crate::stdlib::ScriptValue::Result(result_ref) => {
                                Ok((*result_ref).clone())
                            }
                            _ => Err(Error::new(
                                ErrorKind::TypeError,
                                "and_then closure must return a Result",
                            )),
                        }
                    }
                    crate::stdlib::ScriptResult::Err(err) => {
                        Ok(crate::stdlib::ScriptResult::Err(err.clone()))
                    }
                }
            }
            _ => Err(Error::new(
                ErrorKind::TypeError,
                "Expected a closure for and_then operation",
            )),
        }
    }

    /// Inspect the Ok value using a Script closure without consuming it
    pub fn inspect_closure(
        &self,
        closure: &Value,
        executor: &mut ClosureExecutor,
    ) -> Result<crate::stdlib::ScriptResult> {
        match closure {
            Value::Closure(closure_ref) => {
                match self {
                    crate::stdlib::ScriptResult::Ok(val) => {
                        // Execute closure for side effects only
                        let runtime_val =
                            crate::runtime::value_conversion::script_value_to_value(val);
                        executor.execute_unary(closure_ref, runtime_val)?;
                        Ok(self.clone())
                    }
                    crate::stdlib::ScriptResult::Err(_) => Ok(self.clone()),
                }
            }
            _ => Err(Error::new(
                ErrorKind::TypeError,
                "Expected a closure for inspect operation",
            )),
        }
    }

    /// Inspect the Err value using a Script closure without consuming it  
    pub fn inspect_err_closure(
        &self,
        closure: &Value,
        executor: &mut ClosureExecutor,
    ) -> Result<crate::stdlib::ScriptResult> {
        match closure {
            Value::Closure(closure_ref) => {
                match self {
                    crate::stdlib::ScriptResult::Ok(_) => Ok(self.clone()),
                    crate::stdlib::ScriptResult::Err(err) => {
                        // Execute closure for side effects only
                        let runtime_val =
                            crate::runtime::value_conversion::script_value_to_value(err);
                        executor.execute_unary(closure_ref, runtime_val)?;
                        Ok(self.clone())
                    }
                }
            }
            _ => Err(Error::new(
                ErrorKind::TypeError,
                "Expected a closure for inspect_err operation",
            )),
        }
    }
}

/// Extension methods for ScriptOption to work with closures
impl crate::stdlib::ScriptOption {
    /// Map over the Some value using a Script closure
    pub fn map_closure(
        &self,
        closure: &Value,
        executor: &mut ClosureExecutor,
    ) -> Result<crate::stdlib::ScriptOption> {
        match closure {
            Value::Closure(closure_ref) => {
                match self {
                    crate::stdlib::ScriptOption::Some(val) => {
                        // Convert ScriptValue to Value for the closure
                        let runtime_val =
                            crate::runtime::value_conversion::script_value_to_value(val);
                        let mapped = executor.execute_unary(closure_ref, runtime_val)?;
                        // Convert back to ScriptValue
                        let script_val =
                            crate::runtime::value_conversion::value_to_script_value(&mapped)?;
                        Ok(crate::stdlib::ScriptOption::Some(script_val))
                    }
                    crate::stdlib::ScriptOption::None => Ok(crate::stdlib::ScriptOption::None),
                }
            }
            _ => Err(Error::new(
                ErrorKind::TypeError,
                "Expected a closure for map operation",
            )),
        }
    }

    /// Chain Option operations using a Script closure that returns an Option
    pub fn and_then_closure(
        &self,
        closure: &Value,
        executor: &mut ClosureExecutor,
    ) -> Result<crate::stdlib::ScriptOption> {
        match closure {
            Value::Closure(closure_ref) => {
                match self {
                    crate::stdlib::ScriptOption::Some(val) => {
                        // Convert ScriptValue to Value for the closure
                        let runtime_val =
                            crate::runtime::value_conversion::script_value_to_value(val);
                        let result = executor.execute_unary(closure_ref, runtime_val)?;

                        // The result should be an Option type
                        // For now, we'll convert and validate
                        let script_val =
                            crate::runtime::value_conversion::value_to_script_value(&result)?;
                        match script_val {
                            crate::stdlib::ScriptValue::Option(option_ref) => {
                                Ok((*option_ref).clone())
                            }
                            _ => Err(Error::new(
                                ErrorKind::TypeError,
                                "and_then closure must return an Option",
                            )),
                        }
                    }
                    crate::stdlib::ScriptOption::None => Ok(crate::stdlib::ScriptOption::None),
                }
            }
            _ => Err(Error::new(
                ErrorKind::TypeError,
                "Expected a closure for and_then operation",
            )),
        }
    }

    /// Filter the Option using a predicate closure
    pub fn filter_closure(
        &self,
        closure: &Value,
        executor: &mut ClosureExecutor,
    ) -> Result<crate::stdlib::ScriptOption> {
        match closure {
            Value::Closure(closure_ref) => match self {
                crate::stdlib::ScriptOption::Some(val) => {
                    let runtime_val = crate::runtime::value_conversion::script_value_to_value(val);
                    let keep = executor.execute_predicate(closure_ref, runtime_val)?;
                    if keep {
                        Ok(self.clone())
                    } else {
                        Ok(crate::stdlib::ScriptOption::None)
                    }
                }
                crate::stdlib::ScriptOption::None => Ok(crate::stdlib::ScriptOption::None),
            },
            _ => Err(Error::new(
                ErrorKind::TypeError,
                "Expected a closure for filter operation",
            )),
        }
    }

    /// Inspect the Some value using a Script closure without consuming it
    pub fn inspect_closure(
        &self,
        closure: &Value,
        executor: &mut ClosureExecutor,
    ) -> Result<crate::stdlib::ScriptOption> {
        match closure {
            Value::Closure(closure_ref) => {
                match self {
                    crate::stdlib::ScriptOption::Some(val) => {
                        // Execute closure for side effects only
                        let runtime_val =
                            crate::runtime::value_conversion::script_value_to_value(val);
                        executor.execute_unary(closure_ref, runtime_val)?;
                        Ok(self.clone())
                    }
                    crate::stdlib::ScriptOption::None => Ok(self.clone()),
                }
            }
            _ => Err(Error::new(
                ErrorKind::TypeError,
                "Expected a closure for inspect operation",
            )),
        }
    }
}

/// Helper function to create a ClosureExecutor for stdlib functions
pub fn create_executor<'a>(
    closure_runtime: &'a mut ClosureRuntime,
    module: &'a Module,
) -> ClosureExecutor<'a> {
    ClosureExecutor::new(closure_runtime, module)
}
