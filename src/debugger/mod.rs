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
pub use manager::BreakpointManager;
pub use runtime_hooks::{
    DebugEvent, DebugHook, DebuggerState, ExecutionContext, RuntimeDebugInterface,
};

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use crate::error::{Error, Result};
use crate::source::SourceLocation;

/// Debugger-specific error types
#[derive(Debug, Clone, PartialEq)]
pub enum DebuggerError {
    /// No program loaded for debugging
    NoProgramLoaded,
    /// Execution has not started yet
    ExecutionNotStarted,
    /// Execution has already finished
    ExecutionFinished,
    /// Breakpoint not found
    BreakpointNotFound(u32),
    /// Variable not found
    VariableNotFound(String),
    /// Invalid debugger command
    InvalidCommand(String),
    /// Invalid breakpoint location
    InvalidLocation(String),
    /// Debugger already initialized
    AlreadyInitialized,
    /// Debugger not initialized
    NotInitialized,
    /// IO operation failed
    IoError(String),
}

impl std::fmt::Display for DebuggerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DebuggerError::NoProgramLoaded => write!(f, "No program loaded for debugging"),
            DebuggerError::ExecutionNotStarted => write!(f, "Execution has not started yet"),
            DebuggerError::ExecutionFinished => write!(f, "Execution has already finished"),
            DebuggerError::BreakpointNotFound(id) => write!(f, "Breakpoint {} not found", id),
            DebuggerError::VariableNotFound(name) => write!(f, "Variable '{}' not found", name),
            DebuggerError::InvalidCommand(cmd) => write!(f, "Invalid command: {}", cmd),
            DebuggerError::InvalidLocation(loc) => write!(f, "Invalid location: {}", loc),
            DebuggerError::AlreadyInitialized => write!(f, "Debugger is already initialized"),
            DebuggerError::NotInitialized => write!(f, "Debugger is not initialized"),
            DebuggerError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for DebuggerError {}

/// Result type for debugger operations
pub type DebuggerResult<T> = std::result::Result<T, DebuggerError>;

/// Global debugger instance
static DEBUGGER: RwLock<Option<Arc<Debugger>>> = RwLock::new(None);

/// Initialize the global debugger
///
/// This must be called before any debugging operations can be performed.
/// Returns an error if the debugger is already initialized.
pub fn initialize_debugger() -> Result<()> {
    let mut debugger_lock = DEBUGGER
        .write()
        .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on global debugger"))?;
    if debugger_lock.is_some() {
        return Err(Error::invalid_conversion("Debugger is already initialized"));
    }

    let debugger = Arc::new(Debugger::new());
    *debugger_lock = Some(debugger);

    Ok(())
}

/// Get the global debugger instance
pub fn get_debugger() -> Result<Arc<Debugger>> {
    let debugger_lock = DEBUGGER
        .read()
        .map_err(|_| Error::lock_poisoned("Failed to acquire read lock on global debugger"))?;
    debugger_lock
        .as_ref()
        .cloned()
        .ok_or_else(|| Error::invalid_conversion("Debugger is not initialized"))
}

/// Shutdown the debugger
pub fn shutdown_debugger() -> Result<()> {
    let mut debugger_lock = DEBUGGER
        .write()
        .map_err(|_| Error::lock_poisoned("Failed to acquire write lock on global debugger"))?;
    if debugger_lock.is_none() {
        return Err(Error::invalid_conversion("Debugger is not initialized"));
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
        self.state
            .read()
            .map(|state| *state)
            .unwrap_or(DebuggerState::Stopped)
    }

    /// Set the debugger state
    pub fn set_state(&self, state: DebuggerState) {
        if let Ok(mut state_lock) = self.state.write() {
            *state_lock = state;
        }
    }

    /// Create a new debugging session
    pub fn create_session(&self, name: String, file: Option<String>) -> Result<usize> {
        let session_id = self.next_session_id.fetch_add(1, Ordering::SeqCst);
        let session = DebugSession {
            id: session_id,
            name,
            file,
            current_location: None,
            active: true,
        };

        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| Error::lock_poisoned("Failed to acquire lock on debugging sessions"))?;
        sessions.insert(session_id, session);

        Ok(session_id)
    }

    /// Get a debugging session
    pub fn get_session(&self, session_id: usize) -> Result<DebugSession> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| Error::lock_poisoned("Failed to acquire lock on debugging sessions"))?;
        sessions
            .get(&session_id)
            .cloned()
            .ok_or_else(|| Error::key_not_found(format!("Debug session {session_id}")))
    }

    /// Update a debugging session
    pub fn update_session(&self, session_id: usize, session: DebugSession) -> Result<()> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| Error::lock_poisoned("Failed to acquire lock on debugging sessions"))?;
        if sessions.contains_key(&session_id) {
            sessions.insert(session_id, session);
            Ok(())
        } else {
            Err(Error::key_not_found(format!(
                "Debug session {}",
                session_id
            )))
        }
    }

    /// Remove a debugging session
    pub fn remove_session(&self, session_id: usize) -> Result<()> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| Error::lock_poisoned("Failed to acquire lock on debugging sessions"))?;
        sessions
            .remove(&session_id)
            .map(|_| ())
            .ok_or_else(|| Error::key_not_found(format!("Debug session {session_id}")))
    }

    /// List all active sessions
    pub fn list_sessions(&self) -> Vec<DebugSession> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| Error::lock_poisoned("Failed to acquire lock on debugging sessions"));
        match sessions {
            Ok(sessions) => sessions.values().cloned().collect(),
            Err(_) => Vec::new(), // Return empty vec on lock failure
        }
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
    ) -> Result<()> {
        // Set state to paused
        self.set_state(DebuggerState::Paused);

        // Update current location in active sessions
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| Error::lock_poisoned("Failed to acquire lock on debugging sessions"))?;
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
    pub fn continue_execution(&self) -> Result<()> {
        self.set_state(DebuggerState::Running);
        println!("Continuing execution...");
        Ok(())
    }

    /// Step to the next line
    pub fn step_next(&self) -> Result<()> {
        self.set_state(DebuggerState::Stepping);
        println!("Stepping to next line...");
        Ok(())
    }

    /// Step into function calls
    pub fn step_into(&self) -> Result<()> {
        self.set_state(DebuggerState::SteppingInto);
        println!("Stepping into function...");
        Ok(())
    }

    /// Step out of current function
    pub fn step_out(&self) -> Result<()> {
        self.set_state(DebuggerState::SteppingOut);
        println!("Stepping out of function...");
        Ok(())
    }

    /// Stop debugging
    pub fn stop(&self) -> Result<()> {
        self.set_state(DebuggerState::Stopped);

        // Clear current location in all sessions
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| Error::lock_poisoned("Failed to acquire lock on debugging sessions"))?;
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
    pub fn start_session(&mut self) -> Result<()> {
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
            if let Err(e) = io::stdout().flush() {
                eprintln!("Error flushing stdout: {e}");
                break;
            }

            let mut input = String::new();
            if let Err(e) = io::stdin().read_line(&mut input) {
                eprintln!("Error reading input: {e}");
                break;
            }
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
                            let sessions = self.sessions.lock().map_err(|_| {
                                Error::lock_poisoned("Failed to acquire lock on debugging sessions")
                            });
                            match sessions {
                                Ok(sessions) => sessions
                                    .values()
                                    .find(|s| s.active)
                                    .and_then(|s| s.file.clone())
                                    .unwrap_or_else(|| "unnamed.script".to_string()),
                                Err(_) => "unnamed.script".to_string(),
                            }
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
    DEBUGGER.read().map(|lock| lock.is_some()).unwrap_or(false)
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
        let err = initialize_debugger().unwrap_err();
        assert!(err.to_string().contains("already initialized"));

        // Test getting debugger
        let debugger = get_debugger().unwrap();
        assert!(!debugger.is_enabled());

        // Test shutdown
        assert!(shutdown_debugger().is_ok());
        assert!(!is_debugger_initialized());

        // Test double shutdown
        let err = shutdown_debugger().unwrap_err();
        assert!(err.to_string().contains("not initialized"));
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
