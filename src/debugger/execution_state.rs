use crate::source::SourceLocation;
use crate::parser::ast::{Program, Stmt, Expr};
use crate::debugger::{DebuggerError, DebuggerResult, StackFrame};
use std::collections::HashMap;

/// Execution stepping modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StepMode {
    /// Step into function calls
    Into,
    /// Step over function calls
    Over,
    /// Step out of current function
    Out,
    /// Continue execution until next breakpoint
    Continue,
}

/// Current state of program execution
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionStatus {
    /// Execution has not started
    NotStarted,
    /// Execution is paused (at breakpoint or after step)
    Paused,
    /// Execution is running
    Running,
    /// Execution has finished successfully
    Finished,
    /// Execution stopped due to error
    Error(String),
}

/// Tracks the current execution state of the debugged program
#[derive(Debug, Clone)]
pub struct ExecutionState {
    /// The program being debugged
    program: Option<Program>,
    /// Current execution status
    status: ExecutionStatus,
    /// Current source location (if paused)
    current_location: Option<SourceLocation>,
    /// Call stack frames
    call_stack: Vec<StackFrame>,
    /// Current frame index (for navigation)
    current_frame_index: usize,
    /// Global variables
    global_variables: HashMap<String, crate::debugger::Variable>,
    /// Source code lines for context display
    source_lines: Vec<String>,
    /// Current file being debugged
    current_file: Option<String>,
}

impl ExecutionState {
    /// Create a new execution state
    pub fn new() -> Self {
        Self {
            program: None,
            status: ExecutionStatus::NotStarted,
            current_location: None,
            call_stack: Vec::new(),
            current_frame_index: 0,
            global_variables: HashMap::new(),
            source_lines: Vec::new(),
            current_file: None,
        }
    }

    /// Load a program for debugging
    pub fn load_program(&mut self, program: Program, source: &str, file_name: Option<String>) {
        self.program = Some(program);
        self.source_lines = source.lines().map(|s| s.to_string()).collect();
        self.current_file = file_name;
        self.status = ExecutionStatus::NotStarted;
        self.call_stack.clear();
        self.current_frame_index = 0;
        self.global_variables.clear();
    }

    /// Start execution (move to first statement)
    pub fn start_execution(&mut self) -> DebuggerResult<()> {
        if self.program.is_none() {
            return Err(DebuggerError::NoProgramLoaded);
        }

        self.status = ExecutionStatus::Paused;
        // Set current location to the first statement
        if let Some(program) = &self.program {
            if let Some(first_stmt) = program.statements.first() {
                self.current_location = Some(SourceLocation::new(
                    first_stmt.span.start.line,
                    first_stmt.span.start.column,
                ));
            }
        }

        // Initialize global frame
        let global_frame = StackFrame::new("global".to_string(), self.current_location);
        self.call_stack.push(global_frame);

        Ok(())
    }

    /// Step execution forward
    pub fn step(&mut self, mode: StepMode) -> DebuggerResult<()> {
        match self.status {
            ExecutionStatus::NotStarted => return Err(DebuggerError::ExecutionNotStarted),
            ExecutionStatus::Finished => return Err(DebuggerError::ExecutionFinished),
            ExecutionStatus::Error(_) => return Err(DebuggerError::ExecutionFinished),
            _ => {}
        }

        // TODO: Implement actual stepping logic based on AST traversal
        // For now, just simulate moving to the next line
        if let Some(current_loc) = &mut self.current_location {
            current_loc.line += 1;
            
            // Check if we've reached the end of the source
            if current_loc.line > self.source_lines.len() {
                self.status = ExecutionStatus::Finished;
                self.current_location = None;
            }
        }

        Ok(())
    }

    /// Continue execution until next breakpoint
    pub fn continue_execution(&mut self) -> DebuggerResult<()> {
        if matches!(self.status, ExecutionStatus::NotStarted) {
            return Err(DebuggerError::ExecutionNotStarted);
        }

        self.status = ExecutionStatus::Running;
        Ok(())
    }

    /// Pause execution
    pub fn pause(&mut self) -> DebuggerResult<()> {
        if matches!(self.status, ExecutionStatus::Running) {
            self.status = ExecutionStatus::Paused;
        }
        Ok(())
    }

    /// Stop execution
    pub fn stop(&mut self) -> DebuggerResult<()> {
        self.status = ExecutionStatus::Finished;
        self.current_location = None;
        self.call_stack.clear();
        self.current_frame_index = 0;
        Ok(())
    }

    /// Set execution status
    pub fn set_status(&mut self, status: ExecutionStatus) {
        self.status = status;
    }

    /// Get current execution status
    pub fn status(&self) -> &ExecutionStatus {
        &self.status
    }

    /// Get current source location
    pub fn current_location(&self) -> Option<&SourceLocation> {
        self.current_location.as_ref()
    }

    /// Set current source location
    pub fn set_current_location(&mut self, location: SourceLocation) {
        self.current_location = Some(location);
    }

    /// Get call stack
    pub fn call_stack(&self) -> &[StackFrame] {
        &self.call_stack
    }

    /// Get current frame
    pub fn current_frame(&self) -> Option<&StackFrame> {
        self.call_stack.get(self.current_frame_index)
    }

    /// Get current frame (mutable)
    pub fn current_frame_mut(&mut self) -> Option<&mut StackFrame> {
        self.call_stack.get_mut(self.current_frame_index)
    }

    /// Navigate to a specific frame
    pub fn set_current_frame(&mut self, index: usize) -> DebuggerResult<()> {
        if index >= self.call_stack.len() {
            return Err(DebuggerError::InvalidCommand(format!(
                "Frame index {} out of range (0-{})",
                index,
                self.call_stack.len().saturating_sub(1)
            )));
        }
        self.current_frame_index = index;
        Ok(())
    }

    /// Push a new frame onto the call stack
    pub fn push_frame(&mut self, frame: StackFrame) {
        self.call_stack.push(frame);
        self.current_frame_index = self.call_stack.len() - 1;
    }

    /// Pop the current frame from the call stack
    pub fn pop_frame(&mut self) -> Option<StackFrame> {
        let frame = self.call_stack.pop();
        if self.current_frame_index >= self.call_stack.len() && !self.call_stack.is_empty() {
            self.current_frame_index = self.call_stack.len() - 1;
        }
        frame
    }

    /// Get global variables
    pub fn global_variables(&self) -> &HashMap<String, crate::debugger::Variable> {
        &self.global_variables
    }

    /// Set a global variable
    pub fn set_global_variable(&mut self, name: String, variable: crate::debugger::Variable) {
        self.global_variables.insert(name, variable);
    }

    /// Get source lines for context display
    pub fn source_lines(&self) -> &[String] {
        &self.source_lines
    }

    /// Get current file name
    pub fn current_file(&self) -> Option<&String> {
        self.current_file.as_ref()
    }

    /// Get source context around current location
    pub fn get_source_context(&self, context_lines: usize) -> Vec<(usize, String, bool)> {
        if let Some(location) = &self.current_location {
            let current_line = location.line;
            let start = current_line.saturating_sub(context_lines).max(1);
            let end = (current_line + context_lines).min(self.source_lines.len());

            let mut context = Vec::new();
            for line_num in start..=end {
                if line_num <= self.source_lines.len() {
                    let line_content = self.source_lines.get(line_num - 1)
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    let is_current = line_num == current_line;
                    context.push((line_num, line_content, is_current));
                }
            }
            context
        } else {
            Vec::new()
        }
    }

    /// Check if execution is paused
    pub fn is_paused(&self) -> bool {
        matches!(self.status, ExecutionStatus::Paused)
    }

    /// Check if execution is running
    pub fn is_running(&self) -> bool {
        matches!(self.status, ExecutionStatus::Running)
    }

    /// Check if execution has finished
    pub fn is_finished(&self) -> bool {
        matches!(self.status, ExecutionStatus::Finished | ExecutionStatus::Error(_))
    }

    /// Check if program is loaded
    pub fn has_program(&self) -> bool {
        self.program.is_some()
    }
}

impl Default for ExecutionState {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionStatus::NotStarted => write!(f, "Not Started"),
            ExecutionStatus::Paused => write!(f, "Paused"),
            ExecutionStatus::Running => write!(f, "Running"),
            ExecutionStatus::Finished => write!(f, "Finished"),
            ExecutionStatus::Error(msg) => write!(f, "Error: {}", msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{Program, Stmt, StmtKind, Expr, ExprKind, Literal};
    use crate::source::Span;

    fn create_test_program() -> Program {
        let span = Span::new(
            SourceLocation::new(1, 1),
            SourceLocation::new(1, 10),
        );

        Program {
            statements: vec![
                Stmt {
                    kind: StmtKind::Expression(Expr {
                        kind: ExprKind::Literal(Literal::Number(42.0)),
                        span,
                    }),
                    span,
                    attributes: vec![],
                },
            ],
        }
    }

    #[test]
    fn test_execution_state_lifecycle() {
        let mut state = ExecutionState::new();
        assert_eq!(state.status(), &ExecutionStatus::NotStarted);
        assert!(!state.has_program());

        // Load program
        let program = create_test_program();
        state.load_program(program, "let x = 42", Some("test.script".to_string()));
        assert!(state.has_program());
        assert_eq!(state.current_file(), Some(&"test.script".to_string()));

        // Start execution
        state.start_execution().unwrap();
        assert_eq!(state.status(), &ExecutionStatus::Paused);
        assert!(state.current_location().is_some());

        // Step forward
        state.step(StepMode::Into).unwrap();
        assert_eq!(state.status(), &ExecutionStatus::Paused);

        // Continue execution
        state.continue_execution().unwrap();
        assert_eq!(state.status(), &ExecutionStatus::Running);

        // Stop execution
        state.stop().unwrap();
        assert_eq!(state.status(), &ExecutionStatus::Finished);
        assert!(state.current_location().is_none());
    }

    #[test]
    fn test_call_stack_management() {
        let mut state = ExecutionState::new();
        let program = create_test_program();
        state.load_program(program, "let x = 42", None);
        state.start_execution().unwrap();

        // Should have global frame
        assert_eq!(state.call_stack().len(), 1);
        assert_eq!(state.current_frame().unwrap().function_name(), "global");

        // Push a function frame
        let func_frame = StackFrame::new("test_func".to_string(), Some(SourceLocation::new(5, 1)));
        state.push_frame(func_frame);
        assert_eq!(state.call_stack().len(), 2);
        assert_eq!(state.current_frame().unwrap().function_name(), "test_func");

        // Pop frame
        let popped = state.pop_frame().unwrap();
        assert_eq!(popped.function_name(), "test_func");
        assert_eq!(state.call_stack().len(), 1);
        assert_eq!(state.current_frame().unwrap().function_name(), "global");
    }

    #[test]
    fn test_source_context() {
        let mut state = ExecutionState::new();
        let source = "line 1\nline 2\nline 3\nline 4\nline 5";
        let program = create_test_program();
        state.load_program(program, source, None);
        state.start_execution().unwrap();

        // Set current location to line 3
        state.set_current_location(SourceLocation::new(3, 1));

        // Get context with 1 line before/after
        let context = state.get_source_context(1);
        assert_eq!(context.len(), 3);
        assert_eq!(context[0], (2, "line 2".to_string(), false));
        assert_eq!(context[1], (3, "line 3".to_string(), true));
        assert_eq!(context[2], (4, "line 4".to_string(), false));
    }

    #[test]
    fn test_frame_navigation() {
        let mut state = ExecutionState::new();
        let program = create_test_program();
        state.load_program(program, "let x = 42", None);
        state.start_execution().unwrap();

        // Add multiple frames
        state.push_frame(StackFrame::new("func1".to_string(), None));
        state.push_frame(StackFrame::new("func2".to_string(), None));
        assert_eq!(state.call_stack().len(), 3);

        // Navigate to different frames
        state.set_current_frame(0).unwrap();
        assert_eq!(state.current_frame().unwrap().function_name(), "global");

        state.set_current_frame(1).unwrap();
        assert_eq!(state.current_frame().unwrap().function_name(), "func1");

        state.set_current_frame(2).unwrap();
        assert_eq!(state.current_frame().unwrap().function_name(), "func2");

        // Test out of range
        assert!(state.set_current_frame(10).is_err());
    }

    #[test]
    fn test_execution_without_program() {
        let mut state = ExecutionState::new();

        // Should fail to start without program
        assert!(state.start_execution().is_err());

        // Should fail to step without starting
        assert!(state.step(StepMode::Into).is_err());
    }
}