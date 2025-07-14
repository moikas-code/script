//! Optimized closure implementation for better performance
//!
//! This module provides an optimized version of the closure system that uses
//! interned function IDs, efficient capture storage, and reduced allocations.

use super::capture_storage::CaptureStorage;
use super::id_cache::{FunctionId, OptimizedFunctionId};
use crate::error::{Error, ErrorKind, Result};
use crate::runtime::gc;
use crate::runtime::rc::ScriptRc;
use crate::runtime::traceable::Traceable;
use crate::runtime::Value;
use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// Optimized closure with interned function IDs and efficient capture storage
#[derive(Debug, Clone)]
pub struct OptimizedClosure {
    /// Interned function ID for fast comparison
    pub function_id: OptimizedFunctionId,
    /// Optimized storage for captured variables
    pub captured_vars: CaptureStorage,
    /// Parameter names (using Arc to avoid cloning)
    pub parameters: Arc<[String]>,
    /// Whether this closure captures variables by reference
    pub captures_by_ref: bool,
}

impl OptimizedClosure {
    /// Create a new optimized closure
    pub fn new(
        function_id: String,
        parameters: Vec<String>,
        captured_vars: Vec<(String, Value)>,
    ) -> Self {
        OptimizedClosure {
            function_id: OptimizedFunctionId::from_string(&function_id),
            captured_vars: CaptureStorage::from_captures(captured_vars),
            parameters: Arc::from(parameters.into_boxed_slice()),
            captures_by_ref: false,
        }
    }

    /// Create a new optimized closure that captures by reference
    pub fn new_by_ref(
        function_id: String,
        parameters: Vec<String>,
        captured_vars: Vec<(String, Value)>,
    ) -> Self {
        OptimizedClosure {
            function_id: OptimizedFunctionId::from_string(&function_id),
            captured_vars: CaptureStorage::from_captures(captured_vars),
            parameters: Arc::from(parameters.into_boxed_slice()),
            captures_by_ref: true,
        }
    }

    /// Get the function ID as a numeric ID
    pub fn function_id(&self) -> FunctionId {
        self.function_id.id()
    }

    /// Get the function ID as a string
    pub fn function_name(&self) -> Option<Arc<String>> {
        self.function_id.as_string()
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

    /// Check if this closure captures by reference
    pub fn captures_by_reference(&self) -> bool {
        self.captures_by_ref
    }

    /// Get the number of captured variables
    pub fn capture_count(&self) -> usize {
        self.captured_vars.len()
    }

    /// Check if this closure has any captured variables
    pub fn has_captures(&self) -> bool {
        !self.captured_vars.is_empty()
    }

    /// Check if this closure captures other closures (for cycle detection)
    pub fn captures_closures(&self) -> bool {
        self.captured_vars.contains_closures()
    }

    /// Get storage type for debugging
    pub fn storage_type(&self) -> &'static str {
        self.captured_vars.storage_type()
    }
}

impl PartialEq for OptimizedClosure {
    fn eq(&self, other: &Self) -> bool {
        self.function_id == other.function_id
            && self.parameters == other.parameters
            && self.captures_by_ref == other.captures_by_ref
        // Note: We don't compare captured_vars for equality as they may contain
        // different values but represent the same closure
    }
}

impl fmt::Display for OptimizedClosure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "closure({}) -> {{", self.parameters.join(", "))?;
        if !self.captured_vars.is_empty() {
            let capture_names: Vec<_> = self
                .captured_vars
                .iter()
                .map(|(name, _)| name.as_str())
                .collect();
            write!(f, " captures: {:?}", capture_names)?;
        }
        write!(f, " }}")
    }
}

impl Drop for OptimizedClosure {
    fn drop(&mut self) {
        // Check if we had closure captures and notify cycle collector of potential cleanup
        if self.captures_closures() {
            // Notify cycle collector that a closure with potential cycles is being dropped
            for (_, value) in self.captured_vars.iter() {
                match value {
                    Value::Closure(closure_rc) => {
                        gc::possible_cycle(closure_rc);
                    }
                    Value::OptimizedClosure(closure_rc) => {
                        gc::possible_cycle(closure_rc);
                    }
                    _ => {}
                }
            }
        }
    }
}

impl Traceable for OptimizedClosure {
    fn trace(&self, visitor: &mut dyn FnMut(&dyn Any)) {
        // Trace all captured variables, particularly looking for Value::Closure references
        for (_, value) in self.captured_vars.iter() {
            value.trace(visitor);

            // Special handling for closure references to enable cycle detection
            match value {
                Value::Closure(closure_rc) => {
                    visitor(closure_rc as &dyn Any);
                }
                Value::OptimizedClosure(closure_rc) => {
                    visitor(closure_rc as &dyn Any);
                }
                _ => {}
            }
        }
    }

    fn trace_size(&self) -> usize {
        let base_size = std::mem::size_of::<OptimizedClosure>();
        let params_size = self.parameters.iter().map(|s| s.len()).sum::<usize>();
        let captured_size = self
            .captured_vars
            .iter()
            .map(|(k, v)| k.len() + v.trace_size())
            .sum::<usize>();

        base_size + params_size + captured_size
    }
}

/// Optimized closure runtime with function ID caching and call stack optimization
pub struct OptimizedClosureRuntime {
    /// Function registry using numeric IDs for fast lookup
    closure_registry: HashMap<FunctionId, Box<dyn Fn(&[Value]) -> Result<Value>>>,
    /// Lightweight call frames instead of full closure cloning
    call_stack: Vec<CallFrame>,
    /// Cache for parameter count validation
    param_count_cache: HashMap<FunctionId, usize>,
}

/// Lightweight call frame for the call stack
#[derive(Debug, Clone)]
struct CallFrame {
    /// Function ID being called
    function_id: FunctionId,
    /// Argument count (for debugging)
    arg_count: usize,
}

impl OptimizedClosureRuntime {
    /// Create a new optimized closure runtime
    pub fn new() -> Self {
        OptimizedClosureRuntime {
            closure_registry: HashMap::new(),
            call_stack: Vec::new(),
            param_count_cache: HashMap::new(),
        }
    }

    /// Register a closure implementation with string ID
    pub fn register_closure<F>(&mut self, function_id: String, implementation: F)
    where
        F: Fn(&[Value]) -> Result<Value> + 'static,
    {
        let optimized_id = OptimizedFunctionId::from_string(&function_id);
        self.closure_registry
            .insert(optimized_id.id(), Box::new(implementation));
    }

    /// Register a closure implementation with numeric ID
    pub fn register_closure_by_id<F>(&mut self, function_id: FunctionId, implementation: F)
    where
        F: Fn(&[Value]) -> Result<Value> + 'static,
    {
        self.closure_registry
            .insert(function_id, Box::new(implementation));
    }

    /// Execute a closure with optimized validation and call stack
    pub fn execute_closure(&mut self, closure: &OptimizedClosure, args: &[Value]) -> Result<Value> {
        let function_id = closure.function_id();

        // Fast parameter count validation with caching
        let expected_params = match self.param_count_cache.get(&function_id) {
            Some(&count) => count,
            None => {
                let count = closure.param_count();
                self.param_count_cache.insert(function_id, count);
                count
            }
        };

        if args.len() != expected_params {
            return Err(Error::new(
                ErrorKind::RuntimeError,
                format!(
                    "Closure expected {} arguments, got {}",
                    expected_params,
                    args.len()
                ),
            ));
        }

        // Push lightweight call frame
        self.call_stack.push(CallFrame {
            function_id,
            arg_count: args.len(),
        });

        // Fast function lookup by numeric ID
        let result = match self.closure_registry.get(&function_id) {
            Some(implementation) => implementation(args),
            None => {
                let name = closure
                    .function_name()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("#{function_id}"));
                Err(Error::new(
                    ErrorKind::RuntimeError,
                    format!("Closure implementation not found: {name}"),
                ))
            }
        };

        // Pop call frame
        self.call_stack.pop();

        result
    }

    /// Get the current call stack depth
    pub fn call_stack_depth(&self) -> usize {
        self.call_stack.len()
    }

    /// Get the current function ID being executed
    pub fn current_function_id(&self) -> Option<FunctionId> {
        self.call_stack.last().map(|frame| frame.function_id)
    }

    /// Clear the parameter count cache (for testing)
    pub fn clear_param_cache(&mut self) {
        self.param_count_cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.closure_registry.len(), self.param_count_cache.len())
    }
}

impl Default for OptimizedClosureRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// Create an optimized heap-allocated closure with performance optimizations
pub fn create_optimized_closure_heap(
    function_id: String,
    parameters: Vec<String>,
    captured_vars: Vec<(String, Value)>,
    captures_by_ref: bool,
) -> Value {
    // Create the optimized closure
    let closure = if captures_by_ref {
        OptimizedClosure::new_by_ref(function_id, parameters, captured_vars)
    } else {
        OptimizedClosure::new(function_id, parameters, captured_vars)
    };

    // Register with debugger if available
    #[cfg(debug_assertions)]
    {
        if let Some(debugger) = crate::runtime::closure::debug::get_closure_debugger() {
            debugger.register_optimized_closure(&closure);
        }
    }

    // Allocate on heap using ScriptRc
    let rc_closure = ScriptRc::new(closure);

    // Register with cycle collector only if we have closure captures
    if rc_closure.captures_closures() {
        gc::register_rc(&rc_closure);
    }

    // Return as a Value
    Value::OptimizedClosure(rc_closure)
}

/// Create a simple optimized closure for testing
pub fn create_simple_optimized_closure<F>(
    name: &str,
    params: Vec<String>,
    func: F,
) -> (OptimizedClosure, Box<dyn Fn(&[Value]) -> Result<Value>>)
where
    F: Fn(&[Value]) -> Result<Value> + 'static,
{
    let closure = OptimizedClosure::new(name.to_string(), params, vec![]);

    (closure, Box::new(func))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimized_closure_creation() {
        let captured_vars = vec![
            ("x".to_string(), Value::I32(42)),
            ("y".to_string(), Value::String("hello".to_string())),
        ];

        let closure = OptimizedClosure::new(
            "test_closure".to_string(),
            vec!["param1".to_string(), "param2".to_string()],
            captured_vars,
        );

        assert_eq!(closure.param_count(), 2);
        assert_eq!(closure.capture_count(), 2);
        assert_eq!(closure.get_captured("x"), Some(&Value::I32(42)));
        assert_eq!(
            closure.get_captured("y"),
            Some(&Value::String("hello".to_string()))
        );
        assert!(!closure.captures_by_reference());
    }

    #[test]
    fn test_optimized_closure_function_id() {
        let closure1 = OptimizedClosure::new("test_function".to_string(), vec![], vec![]);

        let closure2 = OptimizedClosure::new("test_function".to_string(), vec![], vec![]);

        // Same function name should have same ID
        assert_eq!(closure1.function_id(), closure2.function_id());

        // Should be able to get string back
        assert_eq!(closure1.function_name().unwrap().as_str(), "test_function");
    }

    #[test]
    fn test_optimized_closure_runtime() {
        let mut runtime = OptimizedClosureRuntime::new();

        // Register a simple closure
        runtime.register_closure("add_numbers".to_string(), |args: &[Value]| {
            match (&args[0], &args[1]) {
                (Value::I32(a), Value::I32(b)) => Ok(Value::I32(a + b)),
                _ => Err(Error::new(ErrorKind::TypeError, "Expected i32")),
            }
        });

        // Create and execute closure
        let closure = OptimizedClosure::new(
            "add_numbers".to_string(),
            vec!["a".to_string(), "b".to_string()],
            vec![],
        );

        let result = runtime.execute_closure(&closure, &[Value::I32(10), Value::I32(20)]);
        assert_eq!(result.unwrap(), Value::I32(30));
    }

    #[test]
    fn test_parameter_count_caching() {
        let mut runtime = OptimizedClosureRuntime::new();

        runtime.register_closure("test_func".to_string(), |_args: &[Value]| {
            Ok(Value::I32(42))
        });

        let closure = OptimizedClosure::new(
            "test_func".to_string(),
            vec!["x".to_string(), "y".to_string()],
            vec![],
        );

        // First call should cache the parameter count
        let _ = runtime.execute_closure(&closure, &[Value::I32(1), Value::I32(2)]);

        // Second call should use cached count
        let result = runtime.execute_closure(&closure, &[Value::I32(3), Value::I32(4)]);
        assert_eq!(result.unwrap(), Value::I32(42));

        // Check cache statistics
        let (registry_size, cache_size) = runtime.cache_stats();
        assert_eq!(registry_size, 1);
        assert_eq!(cache_size, 1);
    }

    #[test]
    fn test_storage_type_optimization() {
        // Small capture count should use inline storage
        let small_closure = OptimizedClosure::new(
            "small".to_string(),
            vec![],
            vec![
                ("x".to_string(), Value::I32(1)),
                ("y".to_string(), Value::I32(2)),
            ],
        );
        assert_eq!(small_closure.storage_type(), "inline");

        // Large capture count should use HashMap
        let large_captures: Vec<_> = (0..10)
            .map(|i| (format!("var_{i}"), Value::I32(i)))
            .collect();
        let large_closure = OptimizedClosure::new("large".to_string(), vec![], large_captures);
        assert_eq!(large_closure.storage_type(), "hashmap");
    }
}
