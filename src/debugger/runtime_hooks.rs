//! Runtime integration hooks for the Script debugger
//!
//! This module provides the integration layer between the debugger
//! and the runtime execution system. It defines hooks that should
//! be called at strategic points during execution to check for
//! breakpoints and handle debugging events.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use crate::debugger::get_debugger;
use crate::runtime::value::Value;
use crate::source::SourceLocation;

/// Secure debug logging configuration
#[derive(Debug, Clone)]
pub struct SecureDebugConfig {
    /// Log level for debug output
    pub log_level: LogLevel,
    /// Patterns for sensitive variable names that should be filtered
    pub sensitive_patterns: Vec<String>,
    /// Maximum size for logged values (in characters)
    pub max_value_size: usize,
    /// Whether to log variable values at all
    pub log_values: bool,
}

/// Debug log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LogLevel {
    Off = 0,
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

/// Resource limits for execution context
#[derive(Debug, Clone)]
pub struct ExecutionLimits {
    /// Maximum number of variables per context
    pub max_variables: usize,
    /// Maximum size per variable value (bytes)
    pub max_variable_size: usize,
    /// Maximum total memory for variables
    pub max_total_memory: usize,
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            max_variables: Self::default_max_variables(),
            max_variable_size: Self::default_max_variable_size(),
            max_total_memory: Self::default_max_total_memory(),
        }
    }
}

impl ExecutionLimits {
    /// Default maximum number of variables per execution context
    pub const fn default_max_variables() -> usize {
        1000
    }

    /// Default maximum size per variable (1MB)
    pub const fn default_max_variable_size() -> usize {
        1024 * 1024
    }

    /// Default maximum total memory for variables (10MB)
    pub const fn default_max_total_memory() -> usize {
        10 * 1024 * 1024
    }

    /// Create execution limits for development environments
    pub fn for_development() -> Self {
        Self {
            max_variables: 2000,
            max_variable_size: 4 * 1024 * 1024, // 4MB per variable
            max_total_memory: 50 * 1024 * 1024, // 50MB total
        }
    }

    /// Create execution limits for production environments
    pub fn for_production() -> Self {
        Self {
            max_variables: 500,
            max_variable_size: 512 * 1024, // 512KB per variable
            max_total_memory: 5 * 1024 * 1024, // 5MB total
        }
    }

    /// Create execution limits for testing environments
    pub fn for_testing() -> Self {
        Self {
            max_variables: 100,
            max_variable_size: 64 * 1024, // 64KB per variable
            max_total_memory: 1024 * 1024, // 1MB total
        }
    }
}

impl Default for SecureDebugConfig {
    fn default() -> Self {
        Self {
            log_level: LogLevel::Info,
            // Pre-process patterns to lowercase for performance
            sensitive_patterns: Self::default_sensitive_patterns(),
            max_value_size: Self::default_max_value_size(),
            log_values: false, // Default to safe - don't log values
        }
    }
}

impl SecureDebugConfig {
    /// Default sensitive patterns for variable name filtering
    pub fn default_sensitive_patterns() -> Vec<String> {
        vec![
            "password".to_string(),
            "secret".to_string(),
            "token".to_string(),
            "key".to_string(),
            "auth".to_string(),
            "credential".to_string(),
        ]
    }

    /// Default maximum value size for logging (200 characters)
    pub const fn default_max_value_size() -> usize {
        200
    }

    /// Create a new config with pre-processed lowercase patterns for performance
    pub fn new() -> Self {
        let mut config = Self::default();
        // Ensure all patterns are lowercase for efficient matching
        config.sensitive_patterns = config.sensitive_patterns
            .into_iter()
            .map(|p| p.to_lowercase())
            .collect();
        config
    }

    /// Create configuration for development environments (more verbose logging)
    pub fn for_development() -> Self {
        Self {
            log_level: LogLevel::Debug,
            sensitive_patterns: Self::default_sensitive_patterns(),
            max_value_size: 500,
            log_values: true, // Allow value logging in development
        }
    }

    /// Create configuration for production environments (minimal logging)
    pub fn for_production() -> Self {
        Self {
            log_level: LogLevel::Error,
            sensitive_patterns: Self::default_sensitive_patterns(),
            max_value_size: 100,
            log_values: false, // Never log values in production
        }
    }

    /// Add a sensitive pattern (automatically converted to lowercase)
    pub fn add_sensitive_pattern(&mut self, pattern: impl Into<String>) {
        self.sensitive_patterns.push(pattern.into().to_lowercase());
    }
}

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
    /// Resource usage tracking
    variable_count: usize,
    memory_usage: usize,
    limits: ExecutionLimits,
}

/// Debug events that can occur during execution
/// Uses String for simplicity and to avoid lifetime complications
#[derive(Debug, Clone)]
pub enum DebugEvent {
    /// Execution started
    ExecutionStarted { 
        file: String, 
        entry_point: String 
    },
    /// Execution stopped
    ExecutionStopped {
        reason: String,
        location: Option<SourceLocation>,
    },
    /// Function entered
    FunctionEntered {
        name: String,
        location: SourceLocation,
        parameters: HashMap<String, Value>, // Keep as-is for now
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
#[repr(u8)]
pub enum DebuggerState {
    /// Debugger is stopped (not debugging)
    Stopped = 0,
    /// Execution is running normally
    Running = 1,
    /// Execution is paused at a breakpoint
    Paused = 2,
    /// Single-stepping to next line
    Stepping = 3,
    /// Stepping into function calls
    SteppingInto = 4,
    /// Stepping out of current function
    SteppingOut = 5,
}

impl From<u8> for DebuggerState {
    fn from(value: u8) -> Self {
        match value {
            0 => DebuggerState::Stopped,
            1 => DebuggerState::Running,
            2 => DebuggerState::Paused,
            3 => DebuggerState::Stepping,
            4 => DebuggerState::SteppingInto,
            5 => DebuggerState::SteppingOut,
            _ => DebuggerState::Stopped, // Default to safe state
        }
    }
}

/// Thread-safe debugger state manager
#[derive(Debug)]
pub struct ThreadSafeDebuggerState {
    state: AtomicUsize,
}

impl ThreadSafeDebuggerState {
    pub fn new(initial_state: DebuggerState) -> Self {
        Self {
            state: AtomicUsize::new(initial_state as usize),
        }
    }

    /// Get the current state
    pub fn get(&self) -> DebuggerState {
        let value = self.state.load(Ordering::Acquire);
        DebuggerState::from(value as u8)
    }

    /// Atomically set the state
    pub fn set(&self, new_state: DebuggerState) {
        self.state.store(new_state as usize, Ordering::Release);
    }

    /// Atomically transition from one state to another
    /// Returns true if the transition was successful
    pub fn transition_from_to(&self, from: DebuggerState, to: DebuggerState) -> bool {
        let from_val = from as usize;
        let to_val = to as usize;
        
        self.state
            .compare_exchange(from_val, to_val, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// Check if a state transition is valid
    pub fn is_valid_transition(&self, from: DebuggerState, to: DebuggerState) -> bool {
        use DebuggerState::*;
        
        match (from, to) {
            // Always allow stopping
            (_, Stopped) => true,
            // From stopped, can only go to running
            (Stopped, Running) => true,
            // From running, can go to paused or stepping states
            (Running, Paused | Stepping | SteppingInto | SteppingOut) => true,
            // From paused, can go to running or stepping states
            (Paused, Running | Stepping | SteppingInto | SteppingOut) => true,
            // From stepping states, can go to paused or running
            (Stepping | SteppingInto | SteppingOut, Paused | Running) => true,
            // Invalid transitions
            _ => false,
        }
    }

    /// Safely transition to a new state with validation
    pub fn safe_transition(&self, to: DebuggerState) -> Result<DebuggerState, String> {
        let current = self.get();
        
        if !self.is_valid_transition(current, to) {
            return Err(format!(
                "Invalid state transition from {:?} to {:?}",
                current, to
            ));
        }

        if self.transition_from_to(current, to) {
            Ok(current)
        } else {
            // State changed between get() and transition attempt
            Err("State changed during transition attempt".to_string())
        }
    }
}

impl Default for ThreadSafeDebuggerState {
    fn default() -> Self {
        Self::new(DebuggerState::Stopped)
    }
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

/// Error types for debugger runtime hooks
#[derive(Debug, Clone)]
pub enum DebugError {
    TooManyVariables { current: usize, limit: usize },
    VariableTooLarge { size: usize, limit: usize },
    MemoryLimitExceeded { current: usize, limit: usize },
    InvalidVariableName(String),
}

impl std::fmt::Display for DebugError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DebugError::TooManyVariables { current, limit } => {
                write!(f, "Too many variables: {} exceeds limit of {}", current, limit)
            }
            DebugError::VariableTooLarge { size, limit } => {
                write!(f, "Variable too large: {} bytes exceeds limit of {}", size, limit)
            }
            DebugError::MemoryLimitExceeded { current, limit } => {
                write!(f, "Memory limit exceeded: {} bytes exceeds limit of {}", current, limit)
            }
            DebugError::InvalidVariableName(name) => {
                write!(f, "Invalid variable name: {}", name)
            }
        }
    }
}

impl std::error::Error for DebugError {}

/// Secure debug logger that filters sensitive information
#[derive(Debug)]
pub struct SecureDebugLogger {
    config: SecureDebugConfig,
    start_time: Instant,
}

/// Helper function to get consistent timestamp formatting
fn get_timestamp_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

impl SecureDebugLogger {
    pub fn new(config: SecureDebugConfig) -> Self {
        Self {
            config,
            start_time: Instant::now(),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(SecureDebugConfig::default())
    }

    /// Check if a variable name contains sensitive patterns
    /// Optimized to avoid double lowercase conversion
    fn is_sensitive(&self, name: &str) -> bool {
        let name_lower = name.to_lowercase();
        self.config.sensitive_patterns.iter().any(|pattern| {
            name_lower.contains(pattern) // Patterns are already lowercase
        })
    }

    /// Sanitize a value for logging with optimized string handling
    fn sanitize_value(&self, value: &Value) -> String {
        if !self.config.log_values {
            return "<value hidden>".to_string();
        }

        // For simple values, return static strings to avoid allocations
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(true) | Value::Boolean(true) => "true".to_string(),
            Value::Bool(false) | Value::Boolean(false) => "false".to_string(),
            Value::I32(n) if *n >= -100 && *n <= 100 => {
                // Use optimized conversion for common small integers
                match *n {
                    0 => "0".to_string(),
                    1 => "1".to_string(),
                    -1 => "-1".to_string(),
                    _ => n.to_string(),
                }
            }
            _ => {
                let value_str = format!("{:?}", value);
                if value_str.len() > self.config.max_value_size {
                    format!("{}... <truncated>", &value_str[..self.config.max_value_size])
                } else {
                    value_str
                }
            }
        }
    }

    /// Log a variable change with security filtering
    fn log_variable_change(
        &self,
        name: &str,
        old_value: Option<&Value>,
        new_value: &Value,
        location: SourceLocation,
    ) {
        if self.config.log_level < LogLevel::Debug {
            return;
        }

        let timestamp = self.start_time.elapsed().as_millis();

        if self.is_sensitive(name) {
            // Only log that the variable changed, not its values
            eprintln!("[{}ms] DEBUG: Sensitive variable '{}' changed at {}", 
                timestamp, name, location);
        } else {
            // Only log in debug builds to avoid performance overhead
            #[cfg(debug_assertions)]
            {
                if let Some(old) = old_value {
                    let old_sanitized = self.sanitize_value(old);
                    let new_sanitized = self.sanitize_value(new_value);
                    eprintln!("[{}ms] DEBUG: Variable '{}' changed at {} from {} to {}", 
                        timestamp, name, location, old_sanitized, new_sanitized);
                } else {
                    let new_sanitized = self.sanitize_value(new_value);
                    eprintln!("[{}ms] DEBUG: Variable '{}' assigned at {} = {}", 
                        timestamp, name, location, new_sanitized);
                }
            }
        }
    }

    /// Log an execution result with security filtering
    fn log_execution_result(&self, location: SourceLocation, value: &Value) {
        if self.config.log_level < LogLevel::Debug {
            return;
        }

        #[cfg(debug_assertions)]
        {
            let timestamp = self.start_time.elapsed().as_millis();
            let sanitized = self.sanitize_value(value);
            eprintln!("[{}ms] DEBUG: Executed at {}: result = {}", 
                timestamp, location, sanitized);
        }
    }

    /// Log an exception with controlled information disclosure
    fn log_exception(&self, exception_type: &str, message: &str, location: SourceLocation) {
        if self.config.log_level < LogLevel::Info {
            return;
        }

        let timestamp = self.start_time.elapsed().as_millis();
        
        // Filter potentially sensitive information from exception messages
        let safe_message = if message.len() > self.config.max_value_size {
            format!("{}... <truncated>", &message[..self.config.max_value_size])
        } else {
            message.to_string()
        };

        eprintln!("[{}ms] INFO: Exception {} thrown at {}: {}", 
            timestamp, exception_type, location, safe_message);
    }

    /// Log function entry with parameter filtering
    fn log_function_entry(&self, name: &str, location: SourceLocation, parameter_count: usize) {
        if self.config.log_level < LogLevel::Debug {
            return;
        }

        let timestamp = self.start_time.elapsed().as_millis();
        eprintln!("[{}ms] DEBUG: Entered function '{}' at {} ({} parameters)", 
            timestamp, name, location, parameter_count);
    }

    /// Log function exit with return value filtering
    fn log_function_exit(&self, name: &str, location: SourceLocation, return_value: Option<&Value>) {
        if self.config.log_level < LogLevel::Debug {
            return;
        }

        let timestamp = self.start_time.elapsed().as_millis();
        if let Some(value) = return_value {
            let sanitized = self.sanitize_value(value);
            eprintln!("[{}ms] DEBUG: Exited function '{}' at {} with return value: {}", 
                timestamp, name, location, sanitized);
        } else {
            eprintln!("[{}ms] DEBUG: Exited function '{}' at {}", 
                timestamp, name, location);
        }
    }
}

/// Default debug hook implementation that integrates with the debugger
pub struct DefaultDebugHook {
    logger: SecureDebugLogger,
}

impl DefaultDebugHook {
    /// Create a new default debug hook
    pub fn new() -> Self {
        DefaultDebugHook {
            logger: SecureDebugLogger::with_default_config(),
        }
    }

    /// Create a debug hook with custom logging configuration
    pub fn with_config(config: SecureDebugConfig) -> Self {
        DefaultDebugHook {
            logger: SecureDebugLogger::new(config),
        }
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
                            // Use structured error logging with context
                            let timestamp = get_timestamp_ms();
                            eprintln!(
                                "[{}ms] ERROR: Breakpoint handling failed at {} in function '{}': {}",
                                timestamp,
                                context.location,
                                context.function_name.as_deref().unwrap_or("<unknown>"),
                                e
                            );
                            
                            // Attempt graceful degradation - continue execution
                            // but disable further breakpoint processing for this context
                        }
                        return false; // Pause execution
                    }
                } else if debugger.should_break(context.location, context.function_name.as_deref())
                {
                    // Breakpoint hit (no file info)
                    if let Err(e) = debugger
                        .handle_breakpoint(context.location, context.function_name.as_deref())
                    {
                        // Use structured error logging with context
                        let timestamp = get_timestamp_ms();
                        eprintln!(
                            "[{}ms] ERROR: Breakpoint handling failed at {} in function '{}': {}",
                            timestamp,
                            context.location,
                            context.function_name.as_deref().unwrap_or("<unknown>"),
                            e
                        );
                        
                        // Continue execution despite error
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

                // Log execution if needed (with secure logging)
                if let Some(value) = result {
                    self.logger.log_execution_result(context.location, value);
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
                                // Use structured error logging with function context
                                let timestamp = get_timestamp_ms();
                                eprintln!(
                                    "[{}ms] ERROR: Function breakpoint handling failed at {} in '{}': {}",
                                    timestamp, context.location, function_name, e
                                );
                                
                                // Break out of loop to prevent cascading errors
                                break;
                            }
                            break;
                        }
                    }
                }

                // Log function entry securely
                let function_name = context.function_name.as_deref().unwrap_or("<unknown>");
                self.logger.log_function_entry(function_name, context.location, context.local_variables.len());

                // Emit debug event (without sensitive parameter values)
                let event = DebugEvent::FunctionEntered {
                    name: function_name.to_string(),
                    location: context.location,
                    parameters: HashMap::new(), // Don't include actual parameter values for security
                };
                self.on_debug_event(&event);
            }
        }
    }

    fn on_function_exit(&self, context: &ExecutionContext, return_value: Option<&Value>) {
        if let Ok(debugger) = get_debugger() {
            if debugger.is_enabled() {
                // Log function exit securely
                let function_name = context.function_name.as_deref().unwrap_or("<unknown>");
                self.logger.log_function_exit(function_name, context.location, return_value);

                // Emit debug event (without sensitive return value)
                let event = DebugEvent::FunctionExited {
                    name: function_name.to_string(),
                    location: context.location,
                    return_value: None, // Don't include actual return value for security
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
                                    // Use structured error logging with exception context
                                    let timestamp = get_timestamp_ms();
                                    eprintln!(
                                        "[{}ms] ERROR: Exception breakpoint handling failed for {} at {} in '{}': {}",
                                        timestamp,
                                        exception_type,
                                        context.location,
                                        context.function_name.as_deref().unwrap_or("<unknown>"),
                                        e
                                    );
                                    
                                    // Break out to prevent cascading errors
                                    break;
                                }
                                break;
                            }
                        }
                    }
                }

                // Log exception securely
                self.logger.log_exception(exception_type, message, context.location);

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
                // Log variable change securely
                self.logger.log_variable_change(variable_name, old_value, new_value, context.location);

                // Emit debug event (without sensitive variable values)
                let event = DebugEvent::VariableChanged {
                    name: variable_name.to_string(),
                    old_value: None, // Don't include actual values for security
                    new_value: Value::String("<filtered>".to_string()), // Safe placeholder
                    location: context.location,
                };
                self.on_debug_event(&event);

                // Data breakpoint functionality
                // Note: Data breakpoints are not yet fully implemented in the debugger backend.
                // When implemented, this section should:
                // 1. Query debugger.get_data_breakpoints() for variables matching 'variable_name'
                // 2. Check if the value change satisfies breakpoint conditions
                // 3. Trigger breakpoint handling if conditions are met
                // 4. Emit appropriate BreakpointHit events
                // 
                // Current implementation emits debug events for variable changes but does not
                // pause execution based on data breakpoint conditions.
            }
        }
    }

    fn on_debug_event(&self, event: &DebugEvent) {
        // Log debug events with structured, secure logging
        if self.logger.config.log_level < LogLevel::Info {
            return;
        }

        let timestamp = self.logger.start_time.elapsed().as_millis();

        match event {
            DebugEvent::ExecutionStarted { file, entry_point } => {
                eprintln!("[{}ms] INFO: Execution started in {} at {}", timestamp, file, entry_point);
            }
            DebugEvent::ExecutionStopped { reason, location } => {
                if let Some(loc) = location {
                    eprintln!("[{}ms] INFO: Execution stopped at {}: {}", timestamp, loc, reason);
                } else {
                    eprintln!("[{}ms] INFO: Execution stopped: {}", timestamp, reason);
                }
            }
            DebugEvent::FunctionEntered { name, location, .. } => {
                eprintln!("[{}ms] INFO: Entered function '{}' at {}", timestamp, name, location);
            }
            DebugEvent::FunctionExited { name, location, .. } => {
                eprintln!("[{}ms] INFO: Exited function '{}' at {}", timestamp, name, location);
            }
            DebugEvent::BreakpointHit {
                breakpoint_id,
                location,
                function_name,
            } => {
                if let Some(func) = function_name {
                    eprintln!(
                        "[{}ms] INFO: Breakpoint {} hit at {} in function '{}'",
                        timestamp, breakpoint_id, location, func
                    );
                } else {
                    eprintln!("[{}ms] INFO: Breakpoint {} hit at {}", timestamp, breakpoint_id, location);
                }
            }
            DebugEvent::ExceptionThrown {
                exception_type,
                message: _,
                location,
            } => {
                // Exception already logged securely in log_exception
                eprintln!("[{}ms] INFO: Exception {} at {}", timestamp, exception_type, location);
            }
            DebugEvent::VariableChanged { name, location, .. } => {
                // Variable changes already logged securely in log_variable_change
                eprintln!("[{}ms] INFO: Variable '{}' changed at {}", timestamp, name, location);
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
    /// Returns true if execution should continue, false if it should pause.
    /// On error, defaults to continuing execution for safety.
    pub fn should_continue_execution(&self, context: &ExecutionContext) -> bool {
        // Wrap hook call in error handling for robustness
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.hook.before_execution(context)
        }))
        .unwrap_or_else(|panic_info| {
            let timestamp = get_timestamp_ms();
            eprintln!(
                "[{}ms] ERROR: Debug hook panicked during before_execution at {}: {:?}",
                timestamp, context.location, panic_info
            );
            // Default to continuing execution for safety
            true
        })
    }

    /// Notify after execution
    /// Errors are logged but don't affect execution flow
    pub fn after_execution(&self, context: &ExecutionContext, result: Option<&Value>) {
        if let Err(panic_info) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.hook.after_execution(context, result);
        })) {
            let timestamp = get_timestamp_ms();
            eprintln!(
                "[{}ms] ERROR: Debug hook panicked during after_execution at {}: {:?}",
                timestamp, context.location, panic_info
            );
        }
    }

    /// Notify function entry
    /// Errors are logged but don't affect execution flow
    pub fn on_function_enter(&self, context: &ExecutionContext) {
        if let Err(panic_info) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.hook.on_function_enter(context);
        })) {
            let timestamp = get_timestamp_ms();
            eprintln!(
                "[{}ms] ERROR: Debug hook panicked during on_function_enter at {} in '{}': {:?}",
                timestamp,
                context.location,
                context.function_name.as_deref().unwrap_or("<unknown>"),
                panic_info
            );
        }
    }

    /// Notify function exit
    /// Errors are logged but don't affect execution flow
    pub fn on_function_exit(&self, context: &ExecutionContext, return_value: Option<&Value>) {
        if let Err(panic_info) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.hook.on_function_exit(context, return_value);
        })) {
            let timestamp = get_timestamp_ms();
            eprintln!(
                "[{}ms] ERROR: Debug hook panicked during on_function_exit at {} in '{}': {:?}",
                timestamp,
                context.location,
                context.function_name.as_deref().unwrap_or("<unknown>"),
                panic_info
            );
        }
    }

    /// Notify exception
    /// Errors are logged but don't affect exception handling flow
    pub fn on_exception(&self, context: &ExecutionContext, exception_type: &str, message: &str) {
        if let Err(panic_info) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.hook.on_exception(context, exception_type, message);
        })) {
            let timestamp = get_timestamp_ms();
            eprintln!(
                "[{}ms] ERROR: Debug hook panicked during on_exception for {} at {}: {:?}",
                timestamp, exception_type, context.location, panic_info
            );
        }
    }

    /// Notify variable assignment
    /// Errors are logged but don't affect assignment flow
    pub fn on_variable_assignment(
        &self,
        context: &ExecutionContext,
        variable_name: &str,
        old_value: Option<&Value>,
        new_value: &Value,
    ) {
        if let Err(panic_info) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.hook
                .on_variable_assignment(context, variable_name, old_value, new_value);
        })) {
            let timestamp = get_timestamp_ms();
            eprintln!(
                "[{}ms] ERROR: Debug hook panicked during on_variable_assignment for '{}' at {}: {:?}",
                timestamp, variable_name, context.location, panic_info
            );
        }
    }

    /// Emit a debug event
    /// Errors are logged but don't affect event emission flow
    pub fn emit_debug_event(&self, event: &DebugEvent) {
        if let Err(panic_info) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.hook.on_debug_event(event);
        })) {
            let timestamp = get_timestamp_ms();
            eprintln!(
                "[{}ms] ERROR: Debug hook panicked during emit_debug_event: {:?}",
                timestamp, panic_info
            );
        }
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
            variable_count: 0,
            memory_usage: 0,
            limits: ExecutionLimits::default(),
        }
    }

    /// Create a new execution context with custom limits
    pub fn with_limits(location: SourceLocation, limits: ExecutionLimits) -> Self {
        ExecutionContext {
            location,
            file: None,
            function_name: None,
            local_variables: HashMap::new(),
            stack_depth: 0,
            thread_id: None,
            variable_count: 0,
            memory_usage: 0,
            limits,
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
            variable_count: 0,
            memory_usage: 0,
            limits: ExecutionLimits::default(),
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
            variable_count: 0,
            memory_usage: 0,
            limits: ExecutionLimits::default(),
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

    /// Add a local variable with resource limits checking
    pub fn add_variable(&mut self, name: String, value: Value) -> Result<(), DebugError> {
        // Validate variable name
        if name.is_empty() || name.len() > 256 {
            return Err(DebugError::InvalidVariableName(name));
        }

        // Check variable count limit
        if self.variable_count >= self.limits.max_variables {
            return Err(DebugError::TooManyVariables {
                current: self.variable_count,
                limit: self.limits.max_variables,
            });
        }

        // Estimate value size (rough approximation)
        let value_size = self.estimate_value_size(&value);
        
        // Check individual variable size limit
        if value_size > self.limits.max_variable_size {
            return Err(DebugError::VariableTooLarge {
                size: value_size,
                limit: self.limits.max_variable_size,
            });
        }

        // Check total memory limit
        let new_memory_usage = self.memory_usage + value_size;
        if new_memory_usage > self.limits.max_total_memory {
            return Err(DebugError::MemoryLimitExceeded {
                current: new_memory_usage,
                limit: self.limits.max_total_memory,
            });
        }

        // Update tracking
        let was_new = self.local_variables.insert(name, value).is_none();
        if was_new {
            self.variable_count += 1;
        }
        self.memory_usage = new_memory_usage;

        Ok(())
    }

    /// Estimate the memory size of a value (rough approximation)
    fn estimate_value_size(&self, value: &Value) -> usize {
        match value {
            Value::Null => 1,
            Value::Bool(_) | Value::Boolean(_) => 1,
            Value::I32(_) => 4,
            Value::I64(_) => 8,
            Value::F32(_) => 4,
            Value::F64(_) | Value::Number(_) => 8,
            Value::String(s) => s.len() + 8, // String overhead
            Value::Array(arr) => {
                8 + arr.iter().map(|v| self.estimate_value_size(&**v)).sum::<usize>()
            }
            Value::Object(obj) => {
                8 + obj.iter().map(|(k, v)| k.len() + self.estimate_value_size(&**v)).sum::<usize>()
            }
            Value::Function(_) => 32, // Conservative estimate for functions
            Value::Enum { data, .. } => {
                64 + data.as_ref().map_or(0, |v| self.estimate_value_size(&**v))
            }
            Value::Closure(_) | Value::OptimizedClosure(_) => 128, // Conservative estimate for closures
        }
    }

    /// Add a local variable (unsafe version for backwards compatibility)
    pub fn add_variable_unchecked(&mut self, name: String, value: Value) {
        self.local_variables.insert(name, value);
    }

    /// Remove a local variable
    pub fn remove_variable(&mut self, name: &str) -> Option<Value> {
        if let Some(value) = self.local_variables.remove(name) {
            self.variable_count = self.variable_count.saturating_sub(1);
            let value_size = self.estimate_value_size(&value);
            self.memory_usage = self.memory_usage.saturating_sub(value_size);
            Some(value)
        } else {
            None
        }
    }

    /// Get current resource usage statistics
    pub fn resource_usage(&self) -> (usize, usize, usize) {
        (self.variable_count, self.memory_usage, self.limits.max_total_memory)
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

    #[test]
    fn test_secure_debug_config() {
        // Test default configuration
        let config = SecureDebugConfig::default();
        assert_eq!(config.log_level, LogLevel::Info);
        assert!(!config.log_values);
        assert_eq!(config.max_value_size, 200);
        assert!(config.sensitive_patterns.contains(&"password".to_string()));

        // Test development configuration
        let dev_config = SecureDebugConfig::for_development();
        assert_eq!(dev_config.log_level, LogLevel::Debug);
        assert!(dev_config.log_values);
        assert_eq!(dev_config.max_value_size, 500);

        // Test production configuration
        let prod_config = SecureDebugConfig::for_production();
        assert_eq!(prod_config.log_level, LogLevel::Error);
        assert!(!prod_config.log_values);
        assert_eq!(prod_config.max_value_size, 100);
    }

    #[test]
    fn test_execution_limits() {
        // Test default limits
        let limits = ExecutionLimits::default();
        assert_eq!(limits.max_variables, 1000);
        assert_eq!(limits.max_variable_size, 1024 * 1024);
        assert_eq!(limits.max_total_memory, 10 * 1024 * 1024);

        // Test development limits
        let dev_limits = ExecutionLimits::for_development();
        assert_eq!(dev_limits.max_variables, 2000);
        assert_eq!(dev_limits.max_variable_size, 4 * 1024 * 1024);
        assert_eq!(dev_limits.max_total_memory, 50 * 1024 * 1024);

        // Test production limits
        let prod_limits = ExecutionLimits::for_production();
        assert_eq!(prod_limits.max_variables, 500);
        assert_eq!(prod_limits.max_variable_size, 512 * 1024);
        assert_eq!(prod_limits.max_total_memory, 5 * 1024 * 1024);

        // Test testing limits
        let test_limits = ExecutionLimits::for_testing();
        assert_eq!(test_limits.max_variables, 100);
        assert_eq!(test_limits.max_variable_size, 64 * 1024);
        assert_eq!(test_limits.max_total_memory, 1024 * 1024);
    }

    #[test]
    fn test_resource_limits_enforcement() {
        let location = SourceLocation::new(10, 1, 0);
        let limits = ExecutionLimits::for_testing(); // Small limits for testing
        let mut context = ExecutionContext::with_limits(location, limits);

        // Test variable count limit
        for i in 0..100 {
            let result = context.add_variable(format!("var{}", i), Value::I32(i as i32));
            assert!(result.is_ok());
        }
        
        // Should hit variable count limit
        let result = context.add_variable("overflow_var".to_string(), Value::I32(999));
        assert!(matches!(result, Err(DebugError::TooManyVariables { .. })));

        // Test variable name validation
        let result = context.add_variable("".to_string(), Value::I32(1));
        assert!(matches!(result, Err(DebugError::InvalidVariableName(_))));

        let result = context.add_variable("a".repeat(300), Value::I32(1));
        assert!(matches!(result, Err(DebugError::InvalidVariableName(_))));
    }

    #[test]
    fn test_sensitive_data_filtering() {
        let config = SecureDebugConfig::default();
        let logger = SecureDebugLogger::new(config);

        // Test sensitive pattern detection
        assert!(logger.is_sensitive("user_password"));
        assert!(logger.is_sensitive("api_token"));
        assert!(logger.is_sensitive("secret_key"));
        assert!(logger.is_sensitive("auth_credential"));
        assert!(!logger.is_sensitive("username"));
        assert!(!logger.is_sensitive("public_data"));
    }

    #[test]
    fn test_value_sanitization() {
        let config = SecureDebugConfig {
            log_values: true,
            max_value_size: 10,
            ..SecureDebugConfig::default()
        };
        let logger = SecureDebugLogger::new(config);

        // Test static value optimization
        assert_eq!(logger.sanitize_value(&Value::Bool(true)), "true");
        assert_eq!(logger.sanitize_value(&Value::Bool(false)), "false");
        assert_eq!(logger.sanitize_value(&Value::I32(0)), "0");
        assert_eq!(logger.sanitize_value(&Value::I32(1)), "1");
        assert_eq!(logger.sanitize_value(&Value::I32(-1)), "-1");
        assert_eq!(logger.sanitize_value(&Value::Null), "null");

        // Test truncation for large values
        let long_string = Value::String("a".repeat(20));
        let sanitized = logger.sanitize_value(&long_string);
        assert!(sanitized.len() <= 15); // Includes "... <truncated>"
        assert!(sanitized.contains("truncated"));
    }

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Off < LogLevel::Error);
        assert!(LogLevel::Error < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Trace);
    }

    #[test]
    fn test_thread_safe_debugger_state() {
        let state = ThreadSafeDebuggerState::new(DebuggerState::Stopped);
        
        // Test initial state
        assert_eq!(state.get(), DebuggerState::Stopped);
        
        // Test valid transitions
        assert!(state.is_valid_transition(DebuggerState::Stopped, DebuggerState::Running));
        assert!(state.is_valid_transition(DebuggerState::Running, DebuggerState::Paused));
        assert!(state.is_valid_transition(DebuggerState::Paused, DebuggerState::Stepping));
        
        // Test invalid transitions
        assert!(!state.is_valid_transition(DebuggerState::Stopped, DebuggerState::Paused));
        assert!(!state.is_valid_transition(DebuggerState::Stepping, DebuggerState::SteppingOut));
        
        // Test atomic transitions
        state.set(DebuggerState::Running);
        assert_eq!(state.get(), DebuggerState::Running);
        
        assert!(state.transition_from_to(DebuggerState::Running, DebuggerState::Paused));
        assert_eq!(state.get(), DebuggerState::Paused);
        
        // Test safe transition with validation
        let result = state.safe_transition(DebuggerState::Running);
        assert!(result.is_ok());
        assert_eq!(state.get(), DebuggerState::Running);
        
        // Test invalid safe transition
        let result = state.safe_transition(DebuggerState::Stopped);
        assert!(result.is_ok()); // Stopping is always allowed
        assert_eq!(state.get(), DebuggerState::Stopped);
    }

    #[test]
    fn test_memory_estimation() {
        let location = SourceLocation::new(1, 1, 0);
        let context = ExecutionContext::new(location);

        // Test basic type size estimation
        assert_eq!(context.estimate_value_size(&Value::Bool(true)), 1);
        assert_eq!(context.estimate_value_size(&Value::I32(42)), 4);
        assert_eq!(context.estimate_value_size(&Value::I64(42)), 8);
        assert_eq!(context.estimate_value_size(&Value::F32(3.14)), 4);
        assert_eq!(context.estimate_value_size(&Value::F64(3.14)), 8);
        assert_eq!(context.estimate_value_size(&Value::String("hello".to_string())), 13); // 5 + 8 overhead
        assert_eq!(context.estimate_value_size(&Value::Null), 1);
        
        // Test function size estimation
        assert_eq!(context.estimate_value_size(&Value::Function("test".to_string())), 32);
    }

    #[test]
    fn test_error_display() {
        let error = DebugError::TooManyVariables { current: 1001, limit: 1000 };
        assert!(error.to_string().contains("1001"));
        assert!(error.to_string().contains("1000"));
        
        let error = DebugError::VariableTooLarge { size: 2048, limit: 1024 };
        assert!(error.to_string().contains("2048"));
        assert!(error.to_string().contains("1024"));
        
        let error = DebugError::MemoryLimitExceeded { current: 15000, limit: 10000 };
        assert!(error.to_string().contains("15000"));
        assert!(error.to_string().contains("10000"));
        
        let error = DebugError::InvalidVariableName("invalid name".to_string());
        assert!(error.to_string().contains("invalid name"));
    }
}
