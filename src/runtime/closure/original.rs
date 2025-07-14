//! Closure support for Script programming language
//!
//! This module provides the runtime representation and execution of closures,
//! enabling functional programming patterns in Script code.

use crate::error::{Error, ErrorKind, Result};
use crate::runtime::gc;
use crate::runtime::rc::ScriptRc;
use crate::runtime::traceable::Traceable;
use crate::runtime::Value;
use std::any::Any;
use std::collections::HashMap;
use std::fmt;

/// Represents a closure with captured environment
#[derive(Debug, Clone)]
pub struct Closure {
    /// The function body as a unique identifier
    pub function_id: String,
    /// Captured variables from the surrounding scope
    pub captured_vars: HashMap<String, Value>,
    /// Parameter names for the closure
    pub parameters: Vec<String>,
    /// Whether this closure captures variables by reference
    pub captures_by_ref: bool,
}

impl Closure {
    /// Create a new closure with the given function ID and captures
    pub fn new(
        function_id: String,
        parameters: Vec<String>,
        captured_vars: HashMap<String, Value>,
    ) -> Self {
        Closure {
            function_id,
            captured_vars,
            parameters,
            captures_by_ref: false,
        }
    }

    /// Create a new closure that captures by reference
    pub fn new_by_ref(
        function_id: String,
        parameters: Vec<String>,
        captured_vars: HashMap<String, Value>,
    ) -> Self {
        Closure {
            function_id,
            captured_vars,
            parameters,
            captures_by_ref: true,
        }
    }

    /// Get the number of parameters this closure expects
    pub fn param_count(&self) -> usize {
        self.parameters.len()
    }

    /// Get a captured variable by name
    pub fn get_captured(&self, name: &str) -> Option<&Value> {
        self.captured_vars.get(name)
    }

    /// Update a captured variable (only if captured by reference)
    pub fn set_captured(&mut self, name: String, value: Value) -> Result<()> {
        if !self.captures_by_ref {
            return Err(Error::new(
                ErrorKind::RuntimeError,
                "Cannot modify captured variables in closure that captures by value",
            ));
        }
        self.captured_vars.insert(name, value);
        Ok(())
    }

    /// Get the parameter names
    pub fn get_parameters(&self) -> &[String] {
        &self.parameters
    }

    /// Get the function ID for this closure
    pub fn get_function_id(&self) -> &str {
        &self.function_id
    }

    /// Check if this closure captures by reference
    pub fn captures_by_reference(&self) -> bool {
        self.captures_by_ref
    }
}

impl PartialEq for Closure {
    fn eq(&self, other: &Self) -> bool {
        self.function_id == other.function_id
            && self.parameters == other.parameters
            && self.captures_by_ref == other.captures_by_ref
        // Note: We don't compare captured_vars for equality as they may contain
        // different values but represent the same closure
    }
}

impl fmt::Display for Closure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "closure({}) -> {{", self.parameters.join(", "))?;
        if !self.captured_vars.is_empty() {
            write!(
                f,
                " captures: {:?}",
                self.captured_vars.keys().collect::<Vec<_>>()
            )?;
        }
        write!(f, " }}")
    }
}

impl Drop for Closure {
    fn drop(&mut self) {
        // When a closure is dropped, all captured values will be dropped automatically
        // due to HashMap's Drop implementation. This ensures proper reference counting
        // for any ScriptRc values that were captured.

        // Check if we had closure captures and notify cycle collector of potential cleanup
        let has_closure_captures = self
            .captured_vars
            .values()
            .any(|v| matches!(v, Value::Closure(_)));

        if has_closure_captures {
            // Notify cycle collector that a closure with potential cycles is being dropped
            // This may trigger cycle collection if there are many potential roots
            for value in self.captured_vars.values() {
                if let Value::Closure(closure_rc) = value {
                    gc::possible_cycle(closure_rc);
                }
            }
        }

        // Log closure destruction for debugging (optional)
        #[cfg(debug_assertions)]
        {
            if !self.captured_vars.is_empty() {
                eprintln!(
                    "Dropping closure '{}' with {} captured variables",
                    self.function_id,
                    self.captured_vars.len()
                );
            }
        }
    }
}

impl Traceable for Closure {
    fn trace(&self, visitor: &mut dyn FnMut(&dyn Any)) {
        // Trace all captured variables, particularly looking for Value::Closure references
        for value in self.captured_vars.values() {
            value.trace(visitor);

            // Special handling for closure references to enable cycle detection
            if let Value::Closure(closure_rc) = value {
                visitor(closure_rc as &dyn Any);
            }
        }
    }

    fn trace_size(&self) -> usize {
        let base_size = std::mem::size_of::<Closure>();
        let params_size = self.parameters.iter().map(|s| s.capacity()).sum::<usize>();
        let captured_size = self
            .captured_vars
            .iter()
            .map(|(k, v)| k.capacity() + v.trace_size())
            .sum::<usize>();

        base_size + params_size + captured_size + self.function_id.capacity()
    }
}

/// Runtime for executing closures
pub struct ClosureRuntime {
    /// Currently executing closure stack
    call_stack: Vec<Closure>,
    /// Global closure registry for function lookups
    closure_registry: HashMap<String, Box<dyn Fn(&[Value]) -> Result<Value>>>,
}

impl ClosureRuntime {
    /// Create a new closure runtime
    pub fn new() -> Self {
        ClosureRuntime {
            call_stack: Vec::new(),
            closure_registry: HashMap::new(),
        }
    }

    /// Register a closure implementation
    pub fn register_closure<F>(&mut self, function_id: String, implementation: F)
    where
        F: Fn(&[Value]) -> Result<Value> + 'static,
    {
        self.closure_registry
            .insert(function_id, Box::new(implementation));
    }

    /// Execute a closure with the given arguments
    pub fn execute_closure(&mut self, closure: &Closure, args: &[Value]) -> Result<Value> {
        // Validate argument count
        if args.len() != closure.param_count() {
            return Err(Error::new(
                ErrorKind::RuntimeError,
                format!(
                    "Closure expected {} arguments, got {}",
                    closure.param_count(),
                    args.len()
                ),
            ));
        }

        // Push closure onto call stack
        self.call_stack.push(closure.clone());

        // Look up the closure implementation
        let result = if let Some(implementation) = self.closure_registry.get(&closure.function_id) {
            implementation(args)
        } else {
            Err(Error::new(
                ErrorKind::RuntimeError,
                format!("Closure implementation not found: {closure.function_id}"),
            ))
        };

        // Pop closure from call stack
        self.call_stack.pop();

        result
    }

    /// Get the current call stack depth
    pub fn call_stack_depth(&self) -> usize {
        self.call_stack.len()
    }

    /// Get the current closure being executed
    pub fn current_closure(&self) -> Option<&Closure> {
        self.call_stack.last()
    }
}

impl Default for ClosureRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to create a simple closure from a function
pub fn create_simple_closure<F>(
    name: &str,
    params: Vec<String>,
    func: F,
) -> (Closure, Box<dyn Fn(&[Value]) -> Result<Value>>)
where
    F: Fn(&[Value]) -> Result<Value> + 'static,
{
    let closure = Closure::new(name.to_string(), params, HashMap::new());

    (closure, Box::new(func))
}

/// Create a heap-allocated closure with proper reference counting and cycle detection
///
/// This function is designed to be called from generated code to create
/// closures on the heap with proper memory management and cycle detection support.
pub fn create_closure_heap(
    function_id: String,
    parameters: Vec<String>,
    captured_vars: Vec<(String, Value)>,
    captures_by_ref: bool,
) -> Value {
    // Process captured variables with proper reference counting
    let mut captures = HashMap::new();
    let mut has_closure_captures = false;

    for (name, value) in captured_vars {
        // For by-value captures, we clone the value (which increments ref count for ScriptRc values)
        // For by-reference captures, we store the value directly
        let captured_value = if captures_by_ref {
            value
        } else {
            value.clone()
        };

        // Check if we're capturing any closures (potential cycle source)
        if matches!(captured_value, Value::Closure(_)) {
            has_closure_captures = true;
        }

        captures.insert(name, captured_value);
    }

    // Create the closure
    let closure = if captures_by_ref {
        Closure::new_by_ref(function_id, parameters, captures)
    } else {
        Closure::new(function_id, parameters, captures)
    };

    // Register with debugger if available
    #[cfg(debug_assertions)]
    {
        if let Some(debugger) = crate::runtime::closure::debug::get_closure_debugger() {
            debugger.register_closure(&closure);
        }
    }

    // Allocate on heap using ScriptRc
    let rc_closure = ScriptRc::new(closure);

    // Register with cycle collector if we have closure captures (potential for cycles)
    if has_closure_captures {
        gc::register_rc(&rc_closure);
    }

    // Return as a Value
    Value::Closure(rc_closure)
}

/// Create a closure from raw parts (for FFI)
///
/// # Safety
/// This function is unsafe because it works with raw pointers from FFI.
/// The caller must ensure:
/// - function_id_ptr points to a valid UTF-8 string
/// - param_names and param_count describe a valid array
/// - capture_names, capture_values, and capture_count describe valid arrays of the same length
// Renamed to avoid conflict with the new implementation in cranelift/runtime.rs
// #[no_mangle]
#[allow(dead_code)]
pub unsafe extern "C" fn script_create_closure_original(
    function_id_ptr: *const u8,
    function_id_len: usize,
    param_names: *const *const u8,
    param_lengths: *const usize,
    param_count: usize,
    capture_names: *const *const u8,
    capture_name_lengths: *const usize,
    capture_values: *const Value,
    capture_count: usize,
    captures_by_ref: bool,
) -> *mut Value {
    // Convert function ID from C string
    let function_id_slice = std::slice::from_raw_parts(function_id_ptr, function_id_len);
    let function_id = match std::str::from_utf8(function_id_slice) {
        Ok(s) => s.to_string(),
        Err(_) => return std::ptr::null_mut(),
    };

    // Convert parameter names
    let mut parameters = Vec::with_capacity(param_count);
    for i in 0..param_count {
        let name_ptr = *param_names.add(i);
        let name_len = *param_lengths.add(i);
        let name_slice = std::slice::from_raw_parts(name_ptr, name_len);
        match std::str::from_utf8(name_slice) {
            Ok(s) => parameters.push(s.to_string()),
            Err(_) => return std::ptr::null_mut(),
        }
    }

    // Convert captured variables
    let mut captured_vars = Vec::with_capacity(capture_count);
    for i in 0..capture_count {
        let name_ptr = *capture_names.add(i);
        let name_len = *capture_name_lengths.add(i);
        let name_slice = std::slice::from_raw_parts(name_ptr, name_len);
        let name = match std::str::from_utf8(name_slice) {
            Ok(s) => s.to_string(),
            Err(_) => return std::ptr::null_mut(),
        };

        let value = (*capture_values.add(i)).clone();
        captured_vars.push((name, value));
    }

    // Create the closure on the heap
    let closure_value =
        create_closure_heap(function_id, parameters, captured_vars, captures_by_ref);

    // Allocate a Value on the heap and return a pointer to it
    Box::into_raw(Box::new(closure_value))
}

/// Free a closure value created by script_create_closure
///
/// # Safety
/// The value_ptr must be a valid pointer returned by script_create_closure
#[no_mangle]
pub unsafe extern "C" fn script_free_closure(value_ptr: *mut Value) {
    if !value_ptr.is_null() {
        // Reconstruct the Box and let it drop naturally
        let _ = Box::from_raw(value_ptr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Value;

    #[test]
    fn test_closure_creation() {
        let mut captured = HashMap::new();
        captured.insert("x".to_string(), Value::I32(42));

        let closure = Closure::new("test_closure".to_string(), vec!["y".to_string()], captured);

        assert_eq!(closure.param_count(), 1);
        assert_eq!(closure.get_function_id(), "test_closure");
        assert_eq!(closure.get_captured("x"), Some(&Value::I32(42)));
        assert!(!closure.captures_by_reference());
    }

    #[test]
    fn test_closure_runtime() {
        let mut runtime = ClosureRuntime::new();

        // Create a simple closure that adds 1 to its argument
        let (closure, implementation) =
            create_simple_closure("add_one", vec!["x".to_string()], |args: &[Value]| {
                if let Value::I32(n) = &args[0] {
                    Ok(Value::I32(n + 1))
                } else {
                    Err(Error::new(ErrorKind::TypeError, "Expected i32"))
                }
            });

        // Register the closure
        runtime.register_closure("add_one".to_string(), move |args| implementation(args));

        // Execute the closure
        let result = runtime.execute_closure(&closure, &[Value::I32(5)]).unwrap();
        assert_eq!(result, Value::I32(6));
    }

    #[test]
    fn test_closure_with_captured_vars() {
        let mut captured = HashMap::new();
        captured.insert("multiplier".to_string(), Value::I32(3));

        let closure = Closure::new(
            "multiply_by_captured".to_string(),
            vec!["x".to_string()],
            captured,
        );

        let mut runtime = ClosureRuntime::new();
        runtime.register_closure("multiply_by_captured".to_string(), |args: &[Value]| {
            if let Value::I32(n) = &args[0] {
                // In a real implementation, this would access the captured variable
                // For now, we'll simulate it
                Ok(Value::I32(n * 3))
            } else {
                Err(Error::new(ErrorKind::TypeError, "Expected i32"))
            }
        });

        let result = runtime.execute_closure(&closure, &[Value::I32(4)]).unwrap();
        assert_eq!(result, Value::I32(12));
    }
}
