use crate::debugger::{
    BreakpointManager, ExecutionState, StackFrame, Variable, VariableValue, VariableScope,
    StepMode, ExecutionStatus, DebuggerError, DebuggerResult, BreakpointLocation,
};
use crate::parser::ast::Program;
use crate::source::SourceLocation;
use colored::*;
use std::collections::HashMap;
use std::io::{self, Write};

/// Result type for debug operations
pub type DebugResult<T> = DebuggerResult<T>;

/// Available debug commands
#[derive(Debug, Clone, PartialEq)]
pub enum DebugCommand {
    /// Start or continue execution
    Run,
    /// Step into the next line
    Step,
    /// Step over function calls
    Next,
    /// Step out of current function
    StepOut,
    /// Set a breakpoint
    Break(String),
    /// Remove a breakpoint
    Delete(u32),
    /// List all breakpoints
    Breakpoints,
    /// Print variable value
    Print(String),
    /// Show call stack backtrace
    Backtrace,
    /// Navigate to specific frame
    Frame(usize),
    /// List local variables
    Locals,
    /// Show current source context
    List(Option<usize>),
    /// Show help
    Help,
    /// Quit debugger
    Quit,
    /// Invalid command
    Invalid(String),
}

/// Interactive debugger CLI
pub struct Debugger {
    breakpoint_manager: BreakpointManager,
    execution_state: ExecutionState,
    source_content: String,
    file_name: Option<String>,
    quit_requested: bool,
}

impl Debugger {
    /// Create a new debugger instance
    pub fn new() -> Self {
        Self {
            breakpoint_manager: BreakpointManager::new(),
            execution_state: ExecutionState::new(),
            source_content: String::new(),
            file_name: None,
            quit_requested: false,
        }
    }

    /// Load a program for debugging
    pub fn load_program(&mut self, program: Program, source: String, file_name: Option<String>) {
        self.source_content = source.clone();
        self.file_name = file_name.clone();
        self.execution_state.load_program(program, &source, file_name);
    }

    /// Start the interactive debugging session
    pub fn start_session(&mut self) -> DebugResult<()> {
        if !self.execution_state.has_program() {
            return Err(DebuggerError::NoProgramLoaded);
        }

        println!("{"Script Debugger v0.1.0".cyan(}")).bold());
        println!("Type 'help' for available commands\n");

        // Start execution in paused state
        self.execution_state.start_execution()?;
        self.show_current_location();

        // Main command loop
        while !self.quit_requested && !self.execution_state.is_finished() {
            self.command_loop()?;
        }

        if self.execution_state.is_finished() {
            println!("{"Program execution finished.".green(}");
        }

        Ok(())
    }

    /// Main command processing loop
    fn command_loop(&mut self) -> DebugResult<()> {
        print!("{} ", "(debug)".cyan().bold());
        io::stdout().flush().map_err(|e| DebuggerError::IoError(e.to_string())?;

        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(|e| DebuggerError::IoError(e.to_string())?;
        let input = input.trim();

        if input.is_empty() {
            return Ok(());
        }

        let command = self.parse_command(input);
        self.execute_command(command)?;

        Ok(())
    }

    /// Parse user input into a debug command
    fn parse_command(&self, input: &str) -> DebugCommand {
        let parts: Vec<&str> = input.split_whitespace().collect();
        
        if parts.is_empty() {
            return DebugCommand::Invalid("Empty command".to_string());
        }

        match parts[0] {
            "run" | "r" | "continue" | "c" => DebugCommand::Run,
            "step" | "s" => DebugCommand::Step,
            "next" | "n" => DebugCommand::Next,
            "stepout" | "so" | "finish" => DebugCommand::StepOut,
            "break" | "b" => {
                if parts.len() > 1 {
                    DebugCommand::Break(parts[1..].join(" "))
                } else {
                    DebugCommand::Invalid("Break command requires a location".to_string())
                }
            }
            "delete" | "d" => {
                if parts.len() > 1 {
                    match parts[1].parse::<u32>() {
                        Ok(id) => DebugCommand::Delete(id),
                        Err(_) => DebugCommand::Invalid("Invalid breakpoint ID".to_string()),
                    }
                } else {
                    DebugCommand::Invalid("Delete command requires breakpoint ID".to_string())
                }
            }
            "breakpoints" | "info" | "i" => DebugCommand::Breakpoints,
            "print" | "p" => {
                if parts.len() > 1 {
                    DebugCommand::Print(parts[1..].join(" "))
                } else {
                    DebugCommand::Invalid("Print command requires a variable name".to_string())
                }
            }
            "backtrace" | "bt" | "stack" => DebugCommand::Backtrace,
            "frame" | "f" => {
                if parts.len() > 1 {
                    match parts[1].parse::<usize>() {
                        Ok(index) => DebugCommand::Frame(index),
                        Err(_) => DebugCommand::Invalid("Invalid frame index".to_string()),
                    }
                } else {
                    DebugCommand::Backtrace // Show backtrace if no frame specified
                }
            }
            "locals" | "l" => DebugCommand::Locals,
            "list" | "ll" => {
                let lines = if parts.len() > 1 {
                    parts[1].parse::<usize>().ok()
                } else {
                    None
                };
                DebugCommand::List(lines)
            }
            "help" | "h" | "?" => DebugCommand::Help,
            "quit" | "q" | "exit" => DebugCommand::Quit,
            _ => DebugCommand::Invalid(format!("Unknown command: {parts[0]}")),
        }
    }

    /// Execute a parsed debug command
    fn execute_command(&mut self, command: DebugCommand) -> DebugResult<()> {
        match command {
            DebugCommand::Run => self.handle_run(),
            DebugCommand::Step => self.handle_step(StepMode::Into),
            DebugCommand::Next => self.handle_step(StepMode::Over),
            DebugCommand::StepOut => self.handle_step(StepMode::Out),
            DebugCommand::Break(location) => self.handle_break(location),
            DebugCommand::Delete(id) => self.handle_delete(id),
            DebugCommand::Breakpoints => self.handle_breakpoints(),
            DebugCommand::Print(var_name) => self.handle_print(var_name),
            DebugCommand::Backtrace => self.handle_backtrace(),
            DebugCommand::Frame(index) => self.handle_frame(index),
            DebugCommand::Locals => self.handle_locals(),
            DebugCommand::List(lines) => self.handle_list(lines.unwrap_or(5)),
            DebugCommand::Help => self.handle_help(),
            DebugCommand::Quit => {
                self.quit_requested = true;
                Ok(())
            }
            DebugCommand::Invalid(msg) => {
                println!("{}: {"Error".red(}")).bold(), msg);
                Ok(())
            }
        }
    }

    /// Handle run/continue command
    fn handle_run(&mut self) -> DebugResult<()> {
        if self.execution_state.is_finished() {
            println!("{"Program has already finished".yellow(}");
            return Ok(());
        }

        self.execution_state.continue_execution()?;
        println!("{"Continuing execution...".green(}");

        // Simulate execution until breakpoint or end
        self.simulate_execution_until_breakpoint()?;
        self.show_current_location();

        Ok(())
    }

    /// Handle step commands
    fn handle_step(&mut self, mode: StepMode) -> DebugResult<()> {
        if self.execution_state.is_finished() {
            println!("{"Program has already finished".yellow(}");
            return Ok(());
        }

        self.execution_state.step(mode)?;

        // Check if we hit a breakpoint
        if let Some(location) = self.execution_state.current_location() {
            if let Some(bp_id) = self.breakpoint_manager.should_break_at(location) {
                if let Some(bp) = self.breakpoint_manager.get_breakpoint(bp_id) {
                    println!("{} {"Breakpoint hit:".yellow(}")).bold(), bp);
                } else {
                    println!("{"Warning: Breakpoint reference is invalid".yellow(}");
                }
            }
        }

        self.show_current_location();
        Ok(())
    }

    /// Handle break command
    fn handle_break(&mut self, location_str: String) -> DebugResult<()> {
        let location = self.breakpoint_manager.parse_location(&location_str)?;
        let bp_id = self.breakpoint_manager.add_breakpoint(location);
        if let Some(bp) = self.breakpoint_manager.get_breakpoint(bp_id) {
            println!("{} {"Breakpoint set:".green(}")).bold(), bp);
        } else {
            return Err(DebuggerError::BreakpointNotFound(bp_id));
        }
        Ok(())
    }

    /// Handle delete breakpoint command
    fn handle_delete(&mut self, id: u32) -> DebugResult<()> {
        let bp = self.breakpoint_manager.remove_breakpoint(id)?;
        println!("{} {"Breakpoint deleted:".green(}")).bold(), bp);
        Ok(())
    }

    /// Handle breakpoints list command
    fn handle_breakpoints(&self) -> DebugResult<()> {
        let breakpoints: Vec<_> = self.breakpoint_manager.get_all_breakpoints().collect();
        
        if breakpoints.is_empty() {
            println!("No breakpoints set");
        } else {
            println!("{"Breakpoints:".cyan(}")).bold());
            for bp in breakpoints {
                println!("  {bp}");
            }
        }
        Ok(())
    }

    /// Handle print variable command
    fn handle_print(&self, var_name: String) -> DebugResult<()> {
        // First check current frame
        if let Some(frame) = self.execution_state.current_frame() {
            if let Some(var) = frame.get_variable(&var_name) {
                println!("{} = {var_name.cyan(}")), var.value().debug_string());
                return Ok(());
            }
        }

        // Then check global variables
        if let Some(var) = self.execution_state.global_variables().get(&var_name) {
            println!("{} = {var_name.cyan(}")), var.value().debug_string());
            return Ok(());
        }

        Err(DebuggerError::VariableNotFound(var_name))
    }

    /// Handle backtrace command
    fn handle_backtrace(&self) -> DebugResult<()> {
        let stack = self.execution_state.call_stack();
        
        if stack.is_empty() {
            println!("No stack frames");
            return Ok(());
        }

        println!("{"Call stack:".cyan(}")).bold());
        for (i, frame) in stack.iter().enumerate().rev() {
            let marker = if i == self.execution_state.current_frame_index { ">" } else { " " };
            println!("{}#{} {marker.yellow(}")), i, frame);
        }
        Ok(())
    }

    /// Handle frame navigation command
    fn handle_frame(&mut self, index: usize) -> DebugResult<()> {
        self.execution_state.set_current_frame(index)?;
        println!("Switched to frame #{index}");
        self.show_current_location();
        Ok(())
    }

    /// Handle locals command
    fn handle_locals(&self) -> DebugResult<()> {
        if let Some(frame) = self.execution_state.current_frame() {
            let variables = frame.all_variables();
            
            if variables.is_empty() {
                println!("No local variables");
            } else {
                println!("{"Local variables:".cyan(}")).bold());
                for var in variables {
                    println!("  {} {} = {
                        var.scope(}")).to_string().yellow(),
                        var.name().cyan(),
                        var.value().debug_string()
                    );
                }
            }
        } else {
            println!("No current frame");
        }
        Ok(())
    }

    /// Handle list source command
    fn handle_list(&self, context_lines: usize) -> DebugResult<()> {
        let context = self.execution_state.get_source_context(context_lines);
        
        if context.is_empty() {
            println!("No source context available");
            return Ok(());
        }

        println!("{"Source:".cyan(}")).bold());
        for (line_num, line_content, is_current) in context {
            let marker = if is_current { ">" } else { " " };
            let line_num_str = format!("{:4}", line_num));
            if is_current {
                println!("{}{} {marker.yellow(}")).bold(), line_num_str.yellow(), line_content);
            } else {
                println!("{}{} {marker, line_num_str.dimmed(}")), line_content);
            }
        }
        Ok(())
    }

    /// Handle help command
    fn handle_help(&self) -> DebugResult<()> {
        println!("{"Available commands:".cyan(}")).bold());
        println!();
        println!("  {}         - Start/continue execution", "run, r, continue, c".green());
        println!("  {}              - Step into next line", "step, s".green());
        println!("  {}              - Step over function calls", "next, n".green());
        println!("  {}     - Step out of current function", "stepout, so, finish".green());
        println!("  {}        - Set breakpoint at location", "break <loc>, b <loc>".green());
        println!("  {}         - Delete breakpoint by ID", "delete <id>, d <id>".green());
        println!("  {}      - List all breakpoints", "breakpoints, info, i".green());
        println!("  {}       - Print variable value", "print <var>, p <var>".green());
        println!("  {}      - Show call stack", "backtrace, bt, stack".green());
        println!("  {}        - Switch to stack frame", "frame <num>, f <num>".green());
        println!("  {}           - List local variables", "locals, l".green());
        println!("  {}       - Show source context", "list [lines], ll [lines]".green());
        println!("  {}             - Show this help", "help, h, ?".green());
        println!("  {}            - Quit debugger", "quit, q, exit".green());
        println!();
        println!("{}:", "Examples".yellow().bold());
        println!("  break main.script:10    - Set breakpoint at line 10");
        println!("  break main              - Set breakpoint at function main");
        println!("  print x                 - Print value of variable x");
        println!("  list 3                  - Show 3 lines of context");
        Ok(())
    }

    /// Show current execution location
    fn show_current_location(&self) {
        if let Some(location) = self.execution_state.current_location() {
            let file_name = self.file_name.as_deref().unwrap_or("<unknown>");
            println!("\n{} {}:{}:{
                "Stopped at".yellow(}")).bold(),
                file_name.cyan(),
                location.line,
                location.column
            );
            
            // Show a few lines of context
            self.handle_list(2).unwrap_or(());
        } else if self.execution_state.is_finished() {
            println!("\n{"Program execution finished".green(}")).bold());
        }
    }

    /// Simulate execution until breakpoint or completion
    fn simulate_execution_until_breakpoint(&mut self) -> DebugResult<()> {
        // This is a simplified simulation
        // In a real implementation, this would actually execute the program
        // and check for breakpoints at each step
        
        loop {
            if let Some(location) = self.execution_state.current_location() {
                // Check if we should break at this location
                if let Some(bp_id) = self.breakpoint_manager.should_break_at(&location) {
                    if let Some(bp) = self.breakpoint_manager.get_breakpoint(bp_id) {
                        println!("{} {"Breakpoint hit:".yellow(}")).bold(), bp);
                        self.execution_state.pause()?;
                        break;
                    } else {
                        println!("{"Warning: Breakpoint reference is invalid".yellow(}");
                    }
                }
            }

            // Simulate stepping forward
            if self.execution_state.step(StepMode::Into).is_err() {
                break;
            }

            // If we've reached the end, stop
            if self.execution_state.is_finished() {
                break;
            }
        }

        Ok(())
    }

    /// Add some sample variables for testing
    #[allow(dead_code)]
    fn add_sample_variables(&mut self) {
        if let Some(frame) = self.execution_state.current_frame_mut() {
            frame.add_local_variable(Variable::number("x".to_string(), 42.0, VariableScope::Local));
            frame.add_local_variable(Variable::string("name".to_string(), "Script".to_string(), VariableScope::Local));
            frame.add_local_variable(Variable::boolean("debug".to_string(), true, VariableScope::Local));
            frame.add_parameter(Variable::number("param1".to_string(), 10.0, VariableScope::Parameter));
        }
    }
}

impl Default for Debugger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::*;
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
    fn test_command_parsing() {
        let debugger = Debugger::new();

        // Test basic commands
        assert_eq!(debugger.parse_command("run"), DebugCommand::Run);
        assert_eq!(debugger.parse_command("r"), DebugCommand::Run);
        assert_eq!(debugger.parse_command("step"), DebugCommand::Step);
        assert_eq!(debugger.parse_command("s"), DebugCommand::Step);
        assert_eq!(debugger.parse_command("next"), DebugCommand::Next);
        assert_eq!(debugger.parse_command("quit"), DebugCommand::Quit);

        // Test commands with arguments
        assert_eq!(debugger.parse_command("break main.script:10"), 
                   DebugCommand::Break("main.script:10".to_string()));
        assert_eq!(debugger.parse_command("print x"), 
                   DebugCommand::Print("x".to_string()));
        assert_eq!(debugger.parse_command("delete 1"), 
                   DebugCommand::Delete(1));
        assert_eq!(debugger.parse_command("frame 2"), 
                   DebugCommand::Frame(2));

        // Test invalid commands
        assert!(matches!(debugger.parse_command("invalid"), 
                        DebugCommand::Invalid(_)));
        assert!(matches!(debugger.parse_command("break"), 
                        DebugCommand::Invalid(_)));
    }

    #[test]
    fn test_debugger_lifecycle() {
        let mut debugger = Debugger::new();
        let program = create_test_program();
        let source = "let x = 42".to_string();

        // Load program
        debugger.load_program(program, source, Some("test.script".to_string()));
        assert!(debugger.execution_state.has_program());

        // Test breakpoint management
        let result = debugger.handle_break("test.script:5".to_string());
        assert!(result.is_ok());
        assert_eq!(debugger.breakpoint_manager.count(), 1);

        // Test breakpoint listing
        let result = debugger.handle_breakpoints();
        assert!(result.is_ok());

        // Test breakpoint deletion
        let result = debugger.handle_delete(1);
        assert!(result.is_ok());
        assert_eq!(debugger.breakpoint_manager.count(), 0);
    }

    #[test]
    fn test_variable_inspection() {
        let mut debugger = Debugger::new();
        let program = create_test_program();
        let source = "let x = 42".to_string();

        debugger.load_program(program, source, None);
        debugger.execution_state.start_execution().unwrap();

        // Add a test variable
        if let Some(frame) = debugger.execution_state.current_frame_mut() {
            frame.add_local_variable(Variable::number("test_var".to_string(), 123.0, VariableScope::Local));
        }

        // Test variable printing
        let result = debugger.handle_print("test_var".to_string());
        assert!(result.is_ok());

        // Test non-existent variable
        let result = debugger.handle_print("nonexistent".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_stack_frame_navigation() {
        let mut debugger = Debugger::new();
        let program = create_test_program();
        let source = "let x = 42".to_string();

        debugger.load_program(program, source, None);
        debugger.execution_state.start_execution().unwrap();

        // Add additional frames
        let frame1 = StackFrame::new("func1".to_string(), Some(SourceLocation::new(5, 1)));
        let frame2 = StackFrame::new("func2".to_string(), Some(SourceLocation::new(10, 1)));
        debugger.execution_state.push_frame(frame1);
        debugger.execution_state.push_frame(frame2);

        // Test backtrace
        let result = debugger.handle_backtrace();
        assert!(result.is_ok());

        // Test frame navigation
        let result = debugger.handle_frame(0);
        assert!(result.is_ok());

        let result = debugger.handle_frame(1);
        assert!(result.is_ok());

        // Test invalid frame
        let result = debugger.handle_frame(10);
        assert!(result.is_err());
    }
}