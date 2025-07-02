//! Debugger module for the Script programming language
//!
//! This module provides comprehensive debugging capabilities including:
//! - Breakpoint management (line, function, conditional breakpoints)
//! - Runtime execution control and stepping
//! - Integration with the runtime execution system
//! - Thread-safe operations for concurrent debugging
//!
//! The debugger is designed to integrate seamlessly with the Script runtime
//! and provide hooks for IDE integration and command-line debugging.

pub mod breakpoint;
pub mod manager;
pub mod runtime_hooks;

pub use breakpoint::{Breakpoint, BreakpointCondition, BreakpointId, BreakpointType};
pub use manager::{BreakpointManager, BreakpointManagerError, BreakpointManagerResult};
pub use runtime_hooks::{
    DebugEvent, DebugHook, DebuggerState, ExecutionContext, RuntimeDebugInterface,
};

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use crate::source::SourceLocation;

/// Global debugger instance
static DEBUGGER: RwLock<Option<Arc<Debugger>>> = RwLock::new(None);

/// Initialize the global debugger
///
/// This must be called before any debugging operations can be performed.
/// Returns an error if the debugger is already initialized.
pub fn initialize_debugger() -> Result<(), DebuggerError> {
    let mut debugger_lock = DEBUGGER.write().unwrap();
    if debugger_lock.is_some() {
        return Err(DebuggerError::AlreadyInitialized);
    }

    let debugger = Arc::new(Debugger::new());
    *debugger_lock = Some(debugger);

    Ok(())
}

/// Get the global debugger instance
pub fn get_debugger() -> Result<Arc<Debugger>, DebuggerError> {
    DEBUGGER
        .read()
        .unwrap()
        .as_ref()
        .cloned()
        .ok_or(DebuggerError::NotInitialized)
}

/// Shutdown the debugger
pub fn shutdown_debugger() -> Result<(), DebuggerError> {
    let mut debugger_lock = DEBUGGER.write().unwrap();
    if debugger_lock.is_none() {
        return Err(DebuggerError::NotInitialized);
    }

    *debugger_lock = None;
    Ok(())
}

/// Main debugger structure
///
/// The debugger manages breakpoints, execution state, and provides
/// hooks for runtime integration. It's designed to be thread-safe
/// and can handle multiple debugging sessions.
pub struct Debugger {
    /// Breakpoint manager
    breakpoint_manager: Arc<BreakpointManager>,
    /// Current execution state
    state: RwLock<DebuggerState>,
    /// Whether debugging is enabled
    enabled: AtomicBool,
    /// Next session ID
    next_session_id: AtomicUsize,
    /// Active debugging sessions
    sessions: Mutex<HashMap<usize, DebugSession>>,
}

/// Represents a debugging session
#[derive(Debug, Clone)]
pub struct DebugSession {
    /// Session ID
    pub id: usize,
    /// Session name
    pub name: String,
    /// File being debugged
    pub file: Option<String>,
    /// Current execution location
    pub current_location: Option<SourceLocation>,
    /// Whether the session is active
    pub active: bool,
}

/// Debugger error types
#[derive(Debug, Clone, PartialEq)]
pub enum DebuggerError {
    /// Debugger is already initialized
    AlreadyInitialized,
    /// Debugger is not initialized
    NotInitialized,
    /// Breakpoint operation failed
    BreakpointError(String),
    /// Runtime integration error
    RuntimeError(String),
    /// Session not found
    SessionNotFound(usize),
    /// Invalid operation
    InvalidOperation(String),
}

impl std::fmt::Display for DebuggerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DebuggerError::AlreadyInitialized => write!(f, "Debugger is already initialized"),
            DebuggerError::NotInitialized => write!(f, "Debugger is not initialized"),
            DebuggerError::BreakpointError(msg) => write!(f, "Breakpoint error: {}", msg),
            DebuggerError::RuntimeError(msg) => write!(f, "Runtime error: {}", msg),
            DebuggerError::SessionNotFound(id) => write!(f, "Debug session {} not found", id),
            DebuggerError::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
        }
    }
}

impl std::error::Error for DebuggerError {}

/// Result type for debugger operations
pub type DebuggerResult<T> = std::result::Result<T, DebuggerError>;

impl Debugger {
    /// Create a new debugger instance
    pub fn new() -> Self {
        Debugger {
            breakpoint_manager: Arc::new(BreakpointManager::new()),
            state: RwLock::new(DebuggerState::Stopped),
            enabled: AtomicBool::new(false),
            next_session_id: AtomicUsize::new(1),
            sessions: Mutex::new(HashMap::new()),
        }
    }

    /// Enable or disable debugging
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    /// Check if debugging is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Get the breakpoint manager
    pub fn breakpoint_manager(&self) -> &Arc<BreakpointManager> {
        &self.breakpoint_manager
    }

    /// Get the current debugger state
    pub fn state(&self) -> DebuggerState {
        *self.state.read().unwrap()
    }

    /// Set the debugger state
    pub fn set_state(&self, state: DebuggerState) {
        *self.state.write().unwrap() = state;
    }

    /// Create a new debugging session
    pub fn create_session(&self, name: String, file: Option<String>) -> DebuggerResult<usize> {
        let session_id = self.next_session_id.fetch_add(1, Ordering::SeqCst);
        let session = DebugSession {
            id: session_id,
            name,
            file,
            current_location: None,
            active: true,
        };

        let mut sessions = self.sessions.lock().unwrap();
        sessions.insert(session_id, session);

        Ok(session_id)
    }

    /// Get a debugging session
    pub fn get_session(&self, session_id: usize) -> DebuggerResult<DebugSession> {
        let sessions = self.sessions.lock().unwrap();
        sessions
            .get(&session_id)
            .cloned()
            .ok_or(DebuggerError::SessionNotFound(session_id))
    }

    /// Update a debugging session
    pub fn update_session(&self, session_id: usize, session: DebugSession) -> DebuggerResult<()> {
        let mut sessions = self.sessions.lock().unwrap();
        if sessions.contains_key(&session_id) {
            sessions.insert(session_id, session);
            Ok(())
        } else {
            Err(DebuggerError::SessionNotFound(session_id))
        }
    }

    /// Remove a debugging session
    pub fn remove_session(&self, session_id: usize) -> DebuggerResult<()> {
        let mut sessions = self.sessions.lock().unwrap();
        sessions
            .remove(&session_id)
            .map(|_| ())
            .ok_or(DebuggerError::SessionNotFound(session_id))
    }

    /// List all active sessions
    pub fn list_sessions(&self) -> Vec<DebugSession> {
        let sessions = self.sessions.lock().unwrap();
        sessions.values().cloned().collect()
    }

    /// Check if execution should break at the given location
    ///
    /// This is the main integration point with the runtime.
    /// It should be called at strategic points during execution.
    pub fn should_break(&self, location: SourceLocation, function_name: Option<&str>) -> bool {
        if !self.is_enabled() {
            return false;
        }

        // Check if any breakpoint matches the current location
        self.breakpoint_manager
            .should_break_at_location(location, function_name)
    }

    /// Handle a breakpoint hit
    ///
    /// This method is called when execution hits a breakpoint.
    /// It updates the debugger state and can trigger debugging events.
    pub fn handle_breakpoint(
        &self,
        location: SourceLocation,
        function_name: Option<&str>,
    ) -> DebuggerResult<()> {
        // Set state to paused
        self.set_state(DebuggerState::Paused);

        // Update current location in active sessions
        let mut sessions = self.sessions.lock().unwrap();
        for session in sessions.values_mut() {
            if session.active {
                session.current_location = Some(location);
            }
        }

        // Log breakpoint hit
        let function_info = function_name
            .map(|name| format!(" in function '{}'", name))
            .unwrap_or_default();
        println!("Breakpoint hit at {}{}", location, function_info);

        Ok(())
    }

    /// Continue execution from a breakpoint
    pub fn continue_execution(&self) -> DebuggerResult<()> {
        self.set_state(DebuggerState::Running);
        println!("Continuing execution...");
        Ok(())
    }

    /// Step to the next line
    pub fn step_next(&self) -> DebuggerResult<()> {
        self.set_state(DebuggerState::Stepping);
        println!("Stepping to next line...");
        Ok(())
    }

    /// Step into function calls
    pub fn step_into(&self) -> DebuggerResult<()> {
        self.set_state(DebuggerState::SteppingInto);
        println!("Stepping into function...");
        Ok(())
    }

    /// Step out of current function
    pub fn step_out(&self) -> DebuggerResult<()> {
        self.set_state(DebuggerState::SteppingOut);
        println!("Stepping out of function...");
        Ok(())
    }

    /// Stop debugging
    pub fn stop(&self) -> DebuggerResult<()> {
        self.set_state(DebuggerState::Stopped);

        // Clear current location in all sessions
        let mut sessions = self.sessions.lock().unwrap();
        for session in sessions.values_mut() {
            session.current_location = None;
            session.active = false;
        }

        println!("Debugging stopped");
        Ok(())
    }

    /// Load a program for debugging
    ///
    /// This method prepares the debugger for debugging a specific program.
    /// It stores the source code and file information for later use.
    pub fn load_program(
        &mut self,
        _program: crate::parser::Program,
        source: String,
        file_name: Option<String>,
    ) {
        // Create a debugging session for this program
        let session_name = file_name.clone().unwrap_or_else(|| "unnamed".to_string());
        let _ = self.create_session(session_name, file_name);

        // Store the source code (in a real implementation, this would be
        // stored in a more sophisticated way)
        println!("Program loaded: {} lines", source.lines().count());
    }

    /// Start an interactive debugging session
    ///
    /// This method starts an interactive debugging session where the user
    /// can set breakpoints, step through code, inspect variables, etc.
    pub fn start_session(&mut self) -> DebuggerResult<()> {
        self.set_enabled(true);
        self.set_state(DebuggerState::Running);

        println!("Starting debug session...");
        println!("Type 'help' for available commands");
        println!();

        // In a real implementation, this would start an interactive
        // debugging loop with commands like:
        // - break <line> - Set breakpoint
        // - continue - Continue execution
        // - step - Step to next line
        // - print <var> - Print variable value
        // - list - Show source code
        // - quit - Exit debugger

        use std::io::{self, Write};

        loop {
            print!("(debug) ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            let input = input.trim();

            match input {
                "help" => {
                    println!("Available commands:");
                    println!("  break <line>  - Set breakpoint at line");
                    println!("  continue      - Continue execution");
                    println!("  step          - Step to next line");
                    println!("  list          - Show source code");
                    println!("  quit          - Exit debugger");
                }
                "continue" => {
                    self.continue_execution()?;
                }
                "step" => {
                    self.step_next()?;
                }
                "list" => {
                    println!("(Source code listing would appear here)");
                }
                "quit" => {
                    self.stop()?;
                    break;
                }
                cmd if cmd.starts_with("break ") => {
                    let line_str = cmd.trim_start_matches("break ");
                    if let Ok(line) = line_str.parse::<usize>() {
                        // Get the current file name from the session
                        let file_name = {
                            let sessions = self.sessions.lock().unwrap();
                            sessions
                                .values()
                                .find(|s| s.active)
                                .and_then(|s| s.file.clone())
                                .unwrap_or_else(|| "unnamed.script".to_string())
                        };

                        match self
                            .breakpoint_manager
                            .add_line_breakpoint(file_name.clone(), line)
                        {
                            Ok(id) => {
                                println!("Breakpoint {} set at line {} in {}", id, line, file_name)
                            }
                            Err(e) => println!("Error setting breakpoint: {}", e),
                        }
                    } else {
                        println!("Invalid line number: {}", line_str);
                    }
                }
                "" => continue,
                _ => println!(
                    "Unknown command: {}. Type 'help' for available commands.",
                    input
                ),
            }
        }

        Ok(())
    }
}

/// Check if the debugger is initialized
pub fn is_debugger_initialized() -> bool {
    DEBUGGER.read().unwrap().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debugger_initialization() {
        // Clean up any existing debugger
        let _ = shutdown_debugger();

        // Test initialization
        assert!(!is_debugger_initialized());
        assert!(initialize_debugger().is_ok());
        assert!(is_debugger_initialized());

        // Test double initialization
        assert_eq!(
            initialize_debugger().unwrap_err(),
            DebuggerError::AlreadyInitialized
        );

        // Test getting debugger
        let debugger = get_debugger().unwrap();
        assert!(!debugger.is_enabled());

        // Test shutdown
        assert!(shutdown_debugger().is_ok());
        assert!(!is_debugger_initialized());

        // Test double shutdown
        assert_eq!(
            shutdown_debugger().unwrap_err(),
            DebuggerError::NotInitialized
        );
    }

    #[test]
    fn test_debugger_sessions() {
        let _ = shutdown_debugger();
        initialize_debugger().unwrap();

        let debugger = get_debugger().unwrap();

        // Create a session
        let session_id = debugger
            .create_session("test_session".to_string(), Some("test.script".to_string()))
            .unwrap();
        assert_eq!(session_id, 1);

        // Get the session
        let session = debugger.get_session(session_id).unwrap();
        assert_eq!(session.name, "test_session");
        assert_eq!(session.file, Some("test.script".to_string()));
        assert!(session.active);

        // List sessions
        let sessions = debugger.list_sessions();
        assert_eq!(sessions.len(), 1);

        // Remove session
        assert!(debugger.remove_session(session_id).is_ok());
        assert_eq!(debugger.list_sessions().len(), 0);

        shutdown_debugger().unwrap();
    }

    #[test]
    fn test_debugger_state_management() {
        let _ = shutdown_debugger();
        initialize_debugger().unwrap();

        let debugger = get_debugger().unwrap();

        // Test initial state
        assert_eq!(debugger.state(), DebuggerState::Stopped);

        // Test state transitions
        debugger.set_state(DebuggerState::Running);
        assert_eq!(debugger.state(), DebuggerState::Running);

        debugger.set_state(DebuggerState::Paused);
        assert_eq!(debugger.state(), DebuggerState::Paused);

        // Test control methods
        assert!(debugger.continue_execution().is_ok());
        assert_eq!(debugger.state(), DebuggerState::Running);

        assert!(debugger.step_next().is_ok());
        assert_eq!(debugger.state(), DebuggerState::Stepping);

        assert!(debugger.stop().is_ok());
        assert_eq!(debugger.state(), DebuggerState::Stopped);

        shutdown_debugger().unwrap();
    }
}
