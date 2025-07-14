//! Runtime integration hooks for the Script debugger
//!
//! This module provides the integration layer between the debugger
//! and the runtime execution system. It defines hooks that should
//! be called at strategic points during execution to check for
//! breakpoints and handle debugging events.

use std::collections::HashMap;
use std::sync::Arc;

use crate::debugger::get_debugger;
use crate::runtime::value::Value;
use crate::source::SourceLocation;

/// Represents the current execution context for debugging
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Current source location
    pub location: SourceLocation,
    /// Current file being executed
    pub file: Option<String>,
    /// Current function name
    pub function_name: Option<String>,
    /// Local variables in current scope
    pub local_variables: HashMap<String, Value>,
    /// Call stack depth
    pub stack_depth: usize,
    /// Thread ID (for multi-threaded execution)
    pub thread_id: Option<usize>,
}

/// Debug events that can occur during execution
#[derive(Debug, Clone)]
pub enum DebugEvent {
    /// Execution started
    ExecutionStarted { file: String, entry_point: String },
    /// Execution stopped
    ExecutionStopped {
        reason: String,
        location: Option<SourceLocation>,
    },
    /// Function entered
    FunctionEntered {
        name: String,
        location: SourceLocation,
        parameters: HashMap<String, Value>,
    },
    /// Function exited
    FunctionExited {
        name: String,
        location: SourceLocation,
        return_value: Option<Value>,
    },
    /// Breakpoint hit
    BreakpointHit {
        breakpoint_id: crate::debugger::BreakpointId,
        location: SourceLocation,
        function_name: Option<String>,
    },
    /// Exception thrown
    ExceptionThrown {
        exception_type: String,
        message: String,
        location: SourceLocation,
    },
    /// Variable changed
    VariableChanged {
        name: String,
        old_value: Option<Value>,
        new_value: Value,
        location: SourceLocation,
    },
}

/// Current debugger state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebuggerState {
    /// Debugger is stopped (not debugging)
    Stopped,
    /// Execution is running normally
    Running,
    /// Execution is paused at a breakpoint
    Paused,
    /// Single-stepping to next line
    Stepping,
    /// Stepping into function calls
    SteppingInto,
    /// Stepping out of current function
    SteppingOut,
}

/// Debug hook trait for runtime integration
pub trait DebugHook: Send + Sync {
    /// Called before executing a statement or expression
    /// Returns true if execution should continue, false if it should pause
    fn before_execution(&self, context: &ExecutionContext) -> bool;

    /// Called after executing a statement or expression
    fn after_execution(&self, context: &ExecutionContext, result: Option<&Value>);

    /// Called when entering a function
    fn on_function_enter(&self, context: &ExecutionContext);

    /// Called when exiting a function
    fn on_function_exit(&self, context: &ExecutionContext, return_value: Option<&Value>);

    /// Called when an exception is thrown
    fn on_exception(&self, context: &ExecutionContext, exception_type: &str, message: &str);

    /// Called when a variable is assigned
    fn on_variable_assignment(
        &self,
        context: &ExecutionContext,
        variable_name: &str,
        old_value: Option<&Value>,
        new_value: &Value,
    );

    /// Called for debug events
    fn on_debug_event(&self, event: &DebugEvent);
}

/// Default debug hook implementation that integrates with the debugger
pub struct DefaultDebugHook;

impl DefaultDebugHook {
    /// Create a new default debug hook
    pub fn new() -> Self {
        DefaultDebugHook
    }
}

impl Default for DefaultDebugHook {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugHook for DefaultDebugHook {
    fn before_execution(&self, context: &ExecutionContext) -> bool {
        // Check if debugger is available and enabled
        let debugger = match get_debugger() {
            Ok(debugger) if debugger.is_enabled() => debugger,
            _ => return true, // Continue execution if debugger is not available or disabled
        };

        // Check debugger state
        match debugger.state() {
            DebuggerState::Stopped => return true,
            DebuggerState::Running => {
                // Check for breakpoints
                if let Some(file) = &context.file {
                    if debugger.breakpoint_manager().should_break_at_file_location(
                        file,
                        context.location,
                        context.function_name.as_deref(),
                    ) {
                        // Breakpoint hit
                        if let Err(e) = debugger
                            .handle_breakpoint(context.location, context.function_name.as_deref())
                        {
                            eprintln!("Error handling breakpoint: {e}");
                        }
                        return false; // Pause execution
                    }
                } else if debugger.should_break(context.location, context.function_name.as_deref())
                {
                    // Breakpoint hit (no file info)
                    if let Err(e) = debugger
                        .handle_breakpoint(context.location, context.function_name.as_deref())
                    {
                        eprintln!("Error handling breakpoint: {e}");
                    }
                    return false; // Pause execution
                }
                return true;
            }
            DebuggerState::Paused => return false, // Stay paused
            DebuggerState::Stepping => {
                // Step to next line, then pause
                debugger.set_state(DebuggerState::Paused);
                return false;
            }
            DebuggerState::SteppingInto => {
                // Step into function calls
                debugger.set_state(DebuggerState::Paused);
                return false;
            }
            DebuggerState::SteppingOut => {
                // Continue until function exit
                return true;
            }
        }
    }

    fn after_execution(&self, context: &ExecutionContext, result: Option<&Value>) {
        if let Ok(debugger) = get_debugger() {
            if debugger.is_enabled() {
                // Handle stepping out
                if debugger.state() == DebuggerState::SteppingOut && context.stack_depth == 0 {
                    debugger.set_state(DebuggerState::Paused);
                }

                // Log execution if needed
                if let Some(value) = result {
                    println!("Executed at {}: result = {:?}", context.location, value);
                }
            }
        }
    }

    fn on_function_enter(&self, context: &ExecutionContext) {
        if let Ok(debugger) = get_debugger() {
            if debugger.is_enabled() {
                // Check for function breakpoints
                if let Some(function_name) = &context.function_name {
                    let breakpoints = debugger
                        .breakpoint_manager()
                        .get_breakpoints_for_function(function_name);
                    for breakpoint in breakpoints {
                        if breakpoint.enabled {
                            if let Err(e) =
                                debugger.handle_breakpoint(context.location, Some(function_name))
                            {
                                eprintln!("Error handling function breakpoint: {e}");
                            }
                            break;
                        }
                    }
                }

                // Emit debug event
                let event = DebugEvent::FunctionEntered {
                    name: context.function_name.clone().unwrap_or_default(),
                    location: context.location,
                    parameters: context.local_variables.clone(),
                };
                self.on_debug_event(&event);
            }
        }
    }

    fn on_function_exit(&self, context: &ExecutionContext, return_value: Option<&Value>) {
        if let Ok(debugger) = get_debugger() {
            if debugger.is_enabled() {
                // Emit debug event
                let event = DebugEvent::FunctionExited {
                    name: context.function_name.clone().unwrap_or_default(),
                    location: context.location,
                    return_value: return_value.cloned(),
                };
                self.on_debug_event(&event);
            }
        }
    }

    fn on_exception(&self, context: &ExecutionContext, exception_type: &str, message: &str) {
        if let Ok(debugger) = get_debugger() {
            if debugger.is_enabled() {
                // Check for exception breakpoints
                let breakpoints = debugger.breakpoint_manager().get_all_breakpoints();
                for breakpoint in breakpoints {
                    if let crate::debugger::BreakpointType::Exception {
                        exception_type: bp_type,
                    } = &breakpoint.breakpoint_type
                    {
                        if breakpoint.enabled {
                            let should_break = bp_type
                                .as_ref()
                                .map(|t| t == exception_type)
                                .unwrap_or(true); // Break on all exceptions if no specific type

                            if should_break {
                                if let Err(e) = debugger.handle_breakpoint(
                                    context.location,
                                    context.function_name.as_deref(),
                                ) {
                                    eprintln!("Error handling exception breakpoint: {e}");
                                }
                                break;
                            }
                        }
                    }
                }

                // Emit debug event
                let event = DebugEvent::ExceptionThrown {
                    exception_type: exception_type.to_string(),
                    message: message.to_string(),
                    location: context.location,
                };
                self.on_debug_event(&event);
            }
        }
    }

    fn on_variable_assignment(
        &self,
        context: &ExecutionContext,
        variable_name: &str,
        old_value: Option<&Value>,
        new_value: &Value,
    ) {
        if let Ok(debugger) = get_debugger() {
            if debugger.is_enabled() {
                // Emit debug event
                let event = DebugEvent::VariableChanged {
                    name: variable_name.to_string(),
                    old_value: old_value.cloned(),
                    new_value: new_value.clone(),
                    location: context.location,
                };
                self.on_debug_event(&event);

                // TODO: Support data breakpoints (break when variable changes)
            }
        }
    }

    fn on_debug_event(&self, event: &DebugEvent) {
        // Log debug events
        match event {
            DebugEvent::ExecutionStarted { file, entry_point } => {
                println!("Debug: Execution started in {} at {}", file, entry_point);
            }
            DebugEvent::ExecutionStopped { reason, location } => {
                if let Some(loc) = location {
                    println!("Debug: Execution stopped at {}: {}", loc, reason);
                } else {
                    println!("Debug: Execution stopped: {}", reason);
                }
            }
            DebugEvent::FunctionEntered { name, location, .. } => {
                println!("Debug: Entered function '{}' at {}", name, location);
            }
            DebugEvent::FunctionExited {
                name,
                location,
                return_value,
            } => {
                if let Some(value) = return_value {
                    println!(
                        "Debug: Exited function '{}' at {} with return value: {:?}",
                        name, location, value
                    );
                } else {
                    println!("Debug: Exited function '{}' at {}", name, location);
                }
            }
            DebugEvent::BreakpointHit {
                breakpoint_id,
                location,
                function_name,
            } => {
                if let Some(func) = function_name {
                    println!(
                        "Debug: Breakpoint {} hit at {} in function '{}'",
                        breakpoint_id, location, func
                    );
                } else {
                    println!("Debug: Breakpoint {} hit at {}", breakpoint_id, location);
                }
            }
            DebugEvent::ExceptionThrown {
                exception_type,
                message,
                location,
            } => {
                println!(
                    "Debug: Exception {} thrown at {}: {}",
                    exception_type, location, message
                );
            }
            DebugEvent::VariableChanged {
                name,
                old_value,
                new_value,
                location,
            } => {
                if let Some(old) = old_value {
                    println!(
                        "Debug: Variable '{}' changed at {} from {:?} to {:?}",
                        name, location, old, new_value
                    );
                } else {
                    println!(
                        "Debug: Variable '{}' assigned at {} = {:?}",
                        name, location, new_value
                    );
                }
            }
        }
    }
}

/// Runtime debug interface
///
/// This provides the main interface for the runtime to interact
/// with the debugger system.
pub struct RuntimeDebugInterface {
    hook: Arc<dyn DebugHook>,
}

impl RuntimeDebugInterface {
    /// Create a new runtime debug interface with the default hook
    pub fn new() -> Self {
        RuntimeDebugInterface {
            hook: Arc::new(DefaultDebugHook::new()),
        }
    }

    /// Create a new runtime debug interface with a custom hook
    pub fn with_hook(hook: Arc<dyn DebugHook>) -> Self {
        RuntimeDebugInterface { hook }
    }

    /// Check if execution should continue at the given location
    ///
    /// This is the main entry point for runtime integration.
    /// It should be called before executing each statement or expression.
    pub fn should_continue_execution(&self, context: &ExecutionContext) -> bool {
        self.hook.before_execution(context)
    }

    /// Notify after execution
    pub fn after_execution(&self, context: &ExecutionContext, result: Option<&Value>) {
        self.hook.after_execution(context, result);
    }

    /// Notify function entry
    pub fn on_function_enter(&self, context: &ExecutionContext) {
        self.hook.on_function_enter(context);
    }

    /// Notify function exit
    pub fn on_function_exit(&self, context: &ExecutionContext, return_value: Option<&Value>) {
        self.hook.on_function_exit(context, return_value);
    }

    /// Notify exception
    pub fn on_exception(&self, context: &ExecutionContext, exception_type: &str, message: &str) {
        self.hook.on_exception(context, exception_type, message);
    }

    /// Notify variable assignment
    pub fn on_variable_assignment(
        &self,
        context: &ExecutionContext,
        variable_name: &str,
        old_value: Option<&Value>,
        new_value: &Value,
    ) {
        self.hook
            .on_variable_assignment(context, variable_name, old_value, new_value);
    }

    /// Emit a debug event
    pub fn emit_debug_event(&self, event: &DebugEvent) {
        self.hook.on_debug_event(event);
    }
}

impl Default for RuntimeDebugInterface {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience functions for runtime integration
impl ExecutionContext {
    /// Create a new execution context
    pub fn new(location: SourceLocation) -> Self {
        ExecutionContext {
            location,
            file: None,
            function_name: None,
            local_variables: HashMap::new(),
            stack_depth: 0,
            thread_id: None,
        }
    }

    /// Create an execution context with file information
    pub fn with_file(location: SourceLocation, file: String) -> Self {
        ExecutionContext {
            location,
            file: Some(file),
            function_name: None,
            local_variables: HashMap::new(),
            stack_depth: 0,
            thread_id: None,
        }
    }

    /// Create an execution context with function information
    pub fn with_function(
        location: SourceLocation,
        file: Option<String>,
        function_name: String,
    ) -> Self {
        ExecutionContext {
            location,
            file,
            function_name: Some(function_name),
            local_variables: HashMap::new(),
            stack_depth: 0,
            thread_id: None,
        }
    }

    /// Set the stack depth
    pub fn with_stack_depth(mut self, depth: usize) -> Self {
        self.stack_depth = depth;
        self
    }

    /// Set the thread ID
    pub fn with_thread_id(mut self, thread_id: usize) -> Self {
        self.thread_id = Some(thread_id);
        self
    }

    /// Add a local variable
    pub fn add_variable(&mut self, name: String, value: Value) {
        self.local_variables.insert(name, value);
    }

    /// Remove a local variable
    pub fn remove_variable(&mut self, name: &str) -> Option<Value> {
        self.local_variables.remove(name)
    }

    /// Get a local variable
    pub fn get_variable(&self, name: &str) -> Option<&Value> {
        self.local_variables.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debugger::{initialize_debugger, shutdown_debugger};
    use crate::runtime::value::Value;

    #[test]
    fn test_execution_context_creation() {
        let location = SourceLocation::new(10, 1, 0);
        let context = ExecutionContext::new(location);

        assert_eq!(context.location, location);
        assert!(context.file.is_none());
        assert!(context.function_name.is_none());
        assert_eq!(context.stack_depth, 0);
        assert!(context.thread_id.is_none());
    }

    #[test]
    fn test_execution_context_with_file() {
        let location = SourceLocation::new(10, 1, 0);
        let context = ExecutionContext::with_file(location, "test.script".to_string());

        assert_eq!(context.location, location);
        assert_eq!(context.file, Some("test.script".to_string()));
    }

    #[test]
    fn test_execution_context_with_function() {
        let location = SourceLocation::new(10, 1, 0);
        let context = ExecutionContext::with_function(
            location,
            Some("test.script".to_string()),
            "main".to_string(),
        );

        assert_eq!(context.location, location);
        assert_eq!(context.file, Some("test.script".to_string()));
        assert_eq!(context.function_name, Some("main".to_string()));
    }

    #[test]
    fn test_execution_context_variables() {
        let location = SourceLocation::new(10, 1, 0);
        let mut context = ExecutionContext::new(location);

        // Add a variable
        context.add_variable("x".to_string(), Value::I32(42));
        assert_eq!(context.get_variable("x"), Some(&Value::I32(42)));

        // Remove a variable
        let removed = context.remove_variable("x");
        assert_eq!(removed, Some(Value::I32(42)));
        assert!(context.get_variable("x").is_none());
    }

    #[test]
    fn test_runtime_debug_interface() {
        let interface = RuntimeDebugInterface::new();
        let location = SourceLocation::new(10, 1, 0);
        let context = ExecutionContext::new(location);

        // Should continue execution when debugger is not initialized
        assert!(interface.should_continue_execution(&context));

        // Test other methods don't panic
        interface.after_execution(&context, None);
        interface.on_function_enter(&context);
        interface.on_function_exit(&context, None);
        interface.on_exception(&context, "TestError", "Test message");
        interface.on_variable_assignment(&context, "x", None, &Value::I32(42));
    }

    #[test]
    fn test_debug_hook_with_debugger() {
        // Clean up any existing debugger
        let _ = shutdown_debugger();

        // Initialize debugger
        initialize_debugger().unwrap();

        let debugger = get_debugger().unwrap();
        debugger.set_enabled(true);

        let hook = DefaultDebugHook::new();
        let location = SourceLocation::new(10, 1, 0);
        let context = ExecutionContext::with_file(location, "test.script".to_string());

        // Should continue execution when no breakpoints are set
        assert!(hook.before_execution(&context));

        // Add a breakpoint
        let _bp_id = debugger
            .breakpoint_manager()
            .add_line_breakpoint("test.script".to_string(), 10)
            .unwrap();

        // Should not continue execution when breakpoint is hit
        assert!(!hook.before_execution(&context));

        // Cleanup
        shutdown_debugger().unwrap();
    }

    #[test]
    fn test_debug_events() {
        let hook = DefaultDebugHook::new();

        // Test various debug events
        let event = DebugEvent::ExecutionStarted {
            file: "test.script".to_string(),
            entry_point: "main".to_string(),
        };
        hook.on_debug_event(&event);

        let event = DebugEvent::BreakpointHit {
            breakpoint_id: 1,
            location: SourceLocation::new(10, 1, 0),
            function_name: Some("main".to_string()),
        };
        hook.on_debug_event(&event);

        let event = DebugEvent::ExceptionThrown {
            exception_type: "RuntimeError".to_string(),
            message: "Test error".to_string(),
            location: SourceLocation::new(20, 1, 0),
        };
        hook.on_debug_event(&event);
    }
}
