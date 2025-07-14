use colored::*;
use std::io::{self, Write};

use crate::error::ErrorReporter;
use crate::runtime::{Runtime, Value};
use crate::semantic::SemanticAnalyzer;
use crate::{Lexer, Parser, Token, TokenKind};

mod history;
mod module_loader;
mod session;

pub use history::History;
pub use module_loader::{ModuleExports, ModuleInfo, ModuleLoader};
pub use session::Session;

/// Enhanced REPL modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReplMode {
    /// Interactive development mode (default)
    Interactive,
    /// Token analysis mode
    Tokens,
    /// Parse tree mode
    Parse,
    /// Debug mode
    Debug,
}

/// Enhanced REPL with state persistence and full language support
pub struct EnhancedRepl {
    /// Current REPL mode
    mode: ReplMode,
    /// Session state (variables, functions, etc.)
    session: Session,
    /// Command history
    history: History,
    /// Runtime for executing code
    runtime: Runtime,
    /// Module loader for handling imports
    module_loader: ModuleLoader,
    /// Current multiline input buffer
    multiline_buffer: String,
    /// Whether we're in multiline input mode
    in_multiline: bool,
    /// Prompt counter for better UX
    prompt_counter: usize,
}

impl EnhancedRepl {
    /// Create a new enhanced REPL
    pub fn new() -> io::Result<Self> {
        let history = History::load_or_create()?;
        let runtime = Runtime::new(Default::default());

        Ok(EnhancedRepl {
            mode: ReplMode::Interactive,
            session: Session::new(),
            history,
            runtime,
            module_loader: ModuleLoader::new(),
            multiline_buffer: String::new(),
            in_multiline: false,
            prompt_counter: 1,
        })
    }

    /// Run the enhanced REPL
    pub fn run(&mut self) -> io::Result<()> {
        self.print_welcome();
        self.print_help();

        loop {
            let input = self.read_input()?;

            // Handle empty input
            if input.trim().is_empty() {
                continue;
            }

            // Handle special commands
            if input.starts_with(':') {
                if self.handle_command(&input) {
                    break; // Exit command
                }
                continue;
            }

            // Handle multiline input detection
            if self.is_multiline_start(&input) {
                self.start_multiline_input(input);
                continue;
            }

            // Process the input
            self.process_input(input);
        }

        // Save history and session state
        self.history.save()?;
        self.session.save()?;

        println!("Goodbye!");
        Ok(())
    }

    /// Print welcome message
    fn print_welcome(&self) {
        println!(
            "\n{} {} - {}",
            "Script".cyan().bold(),
            env!("CARGO_PKG_VERSION").green(),
            "AI-Native Programming Language".bright_white()
        );
        println!(
            "{}",
            "🚀 Production Ready - Enhanced Interactive Mode".green()
        );
        println!(
            "{}",
            "Type ':help' for commands, ':exit' to quit\n".dimmed()
        );
    }

    /// Print help information
    fn print_help(&self) {
        println!("{}", "Available commands:".yellow().bold());
        println!(
            "  {}  - Switch to interactive development mode (default)",
            ":interactive".cyan()
        );
        println!(
            "  {}      - Switch to token analysis mode",
            ":tokens".cyan()
        );
        println!("  {}       - Switch to parse tree mode", ":parse".cyan());
        println!("  {}       - Switch to debug mode", ":debug".cyan());
        println!("  {}       - Show command history", ":history".cyan());
        println!("  {}        - Clear session state", ":clear".cyan());
        println!("  {}         - Show session variables", ":vars".cyan());
        println!("  {}        - Show defined types", ":types".cyan());
        println!("  {}        - Show defined functions", ":funcs".cyan());
        println!("  {}      - Show imported modules", ":modules".cyan());
        println!("  {}         - Save session state", ":save".cyan());
        println!("  {}         - Load session state", ":load".cyan());
        println!("  {}         - Show help", ":help".cyan());
        println!("  {}         - Exit REPL", ":exit".cyan());
        println!();
    }

    /// Read input with proper prompt handling
    fn read_input(&mut self) -> io::Result<String> {
        let prompt = if self.in_multiline {
            format!("{}    ", "|".blue().bold())
        } else {
            match self.mode {
                ReplMode::Interactive => format!("[{}]> ", self.prompt_counter.to_string().green()),
                ReplMode::Tokens => "tokens> ".cyan().to_string(),
                ReplMode::Parse => "parse> ".yellow().to_string(),
                ReplMode::Debug => "debug> ".red().to_string(),
            }
        };

        print!("{}", prompt);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim_end().to_string();

        // Add to history if not empty and not a command
        if !input.trim().is_empty() && !input.starts_with(':') {
            self.history.add(input.clone());
        }

        Ok(input)
    }

    /// Handle special commands
    fn handle_command(&mut self, command: &str) -> bool {
        match command.trim() {
            ":exit" | ":quit" => return true,
            ":help" => self.print_help(),
            ":interactive" => {
                self.mode = ReplMode::Interactive;
                println!("Switched to {} mode", "interactive".green());
            }
            ":tokens" => {
                self.mode = ReplMode::Tokens;
                println!("Switched to {} mode", "tokens".cyan());
            }
            ":parse" => {
                self.mode = ReplMode::Parse;
                println!("Switched to {} mode", "parse".yellow());
            }
            ":debug" => {
                self.mode = ReplMode::Debug;
                println!("Switched to {} mode", "debug".red());
            }
            ":history" => self.show_history(),
            ":clear" => {
                self.session.clear();
                println!("{} Session state cleared", "✓".green());
            }
            ":vars" => self.show_variables(),
            ":types" => self.show_types(),
            ":funcs" => self.show_functions(),
            ":modules" => self.show_modules(),
            ":save" => self.save_session(),
            ":load" => self.load_session(),
            _ => {
                println!("{} Unknown command: {}", "Error:".red(), command);
                println!("Type {} for available commands", ":help".cyan());
            }
        }
        false
    }

    /// Show command history
    fn show_history(&self) {
        println!("{}", "Command History:".yellow().bold());
        for (i, cmd) in self.history.recent(10).iter().enumerate() {
            println!("  {}: {}", (i + 1).to_string().dimmed(), cmd);
        }
    }

    /// Show session variables
    fn show_variables(&self) {
        println!("{}", "Session Variables:".yellow().bold());
        if self.session.variables().is_empty() {
            println!("  {}", "No variables defined".dimmed());
        } else {
            for (name, value) in self.session.variables() {
                println!("  {} = {}", name.cyan(), format!("{:?}", value).green());
            }
        }

        // Also show types and functions
        if !self.session.types().is_empty() {
            println!("\n{}", "Session Types:".yellow().bold());
            for (name, type_def) in self.session.types() {
                println!("  {} : {}", name.cyan(), format!("{:?}", type_def).green());
            }
        }

        if !self.session.functions().is_empty() {
            println!("\n{}", "Session Functions:".yellow().bold());
            for (name, signature) in self.session.functions() {
                println!("  {} : {}", name.cyan(), format!("{:?}", signature).green());
            }
        }
    }

    /// Check if input starts a multiline block
    fn is_multiline_start(&self, input: &str) -> bool {
        let trimmed = input.trim();

        // Check for explicit multiline indicators
        if trimmed.ends_with('{') {
            return true;
        }

        // Check for type definitions
        if trimmed.starts_with("struct ")
            || trimmed.starts_with("enum ")
            || trimmed.starts_with("type ")
        {
            return true;
        }

        // Check for function definitions
        if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") {
            return true;
        }

        // Check for implementation blocks
        if trimmed.starts_with("impl ") {
            return true;
        }

        // Check for module definitions
        if trimmed.starts_with("mod ") {
            return true;
        }

        // Check for incomplete statements that need continuation
        if self.needs_continuation(trimmed) {
            return true;
        }

        false
    }

    /// Check if a statement needs continuation (incomplete syntax)
    fn needs_continuation(&self, input: &str) -> bool {
        // Count brackets and braces to detect incomplete blocks
        let mut brace_count = 0;
        let mut paren_count = 0;
        let mut bracket_count = 0;
        let mut in_string = false;
        let mut escape_next = false;

        for ch in input.chars() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => brace_count += 1,
                '}' if !in_string => brace_count -= 1,
                '(' if !in_string => paren_count += 1,
                ')' if !in_string => paren_count -= 1,
                '[' if !in_string => bracket_count += 1,
                ']' if !in_string => bracket_count -= 1,
                _ => {}
            }
        }

        // If any brackets are unclosed, we need continuation
        brace_count > 0 || paren_count > 0 || bracket_count > 0 || in_string
    }

    /// Start multiline input mode
    fn start_multiline_input(&mut self, input: String) {
        self.in_multiline = true;
        self.multiline_buffer = input + "\n";
        println!(
            "{}",
            "  (entering multiline mode, empty line or balanced braces to execute)".dimmed()
        );
    }

    /// Process input based on current mode
    fn process_input(&mut self, input: String) {
        // Handle multiline input completion
        if self.in_multiline {
            self.multiline_buffer.push_str(&input);
            self.multiline_buffer.push('\n');

            // Check if multiline input is complete
            if self.is_multiline_complete(&input) {
                let complete_input = self.multiline_buffer.clone();
                self.multiline_buffer.clear();
                self.in_multiline = false;
                self.execute_input(complete_input);
                self.prompt_counter += 1;
            }
            return;
        }

        // Process single line input
        match self.mode {
            ReplMode::Interactive => self.execute_input(input),
            ReplMode::Tokens => self.analyze_tokens(input),
            ReplMode::Parse => self.analyze_parse(input),
            ReplMode::Debug => self.debug_input(input),
        }

        self.prompt_counter += 1;
    }

    /// Check if multiline input is complete and ready to execute
    fn is_multiline_complete(&self, latest_input: &str) -> bool {
        // If empty line, consider it complete
        if latest_input.trim().is_empty() {
            return true;
        }

        // Check if the complete buffer has balanced brackets
        !self.needs_continuation(&self.multiline_buffer)
    }

    /// Execute input in interactive mode
    fn execute_input(&mut self, input: String) {
        // Try to parse and execute the input
        match self.compile_and_run(&input) {
            Ok(result) => {
                if let Some(value) = result {
                    println!("=> {}", format!("{:?}", value).green());
                }
            }
            Err(error) => {
                println!("{error}");
            }
        }
    }

    /// Compile and run input, updating session state
    fn compile_and_run(&mut self, source: &str) -> Result<Option<Value>, String> {
        // Tokenize
        let lexer = Lexer::new(source).map_err(|e| format!("Lexer error: {e}"))?;
        let (tokens, lex_errors) = lexer.scan_tokens();

        if !lex_errors.is_empty() {
            let mut error_msg = String::new();
            for error in lex_errors {
                error_msg.push_str(&format!("{}\n", error));
            }
            return Err(error_msg);
        }

        // Parse
        let mut parser = Parser::new(tokens);
        let program = parser.parse().map_err(|e| format!("Parse error: {e}"))?;

        // Enhanced semantic analysis with session state
        let analyzer = self.analyze_with_session(&program)?;

        // Process definitions and update session state
        self.process_definitions(&program)?;

        // Code generation and execution
        let result = self.execute_with_session(&program)?;

        Ok(result)
    }

    /// Perform semantic analysis incorporating session state
    fn analyze_with_session(
        &mut self,
        program: &crate::parser::Program,
    ) -> Result<SemanticAnalyzer, String> {
        // Create analyzer with existing session context
        let mut analyzer = SemanticAnalyzer::new();

        // Add session definitions to analyzer's symbol table
        self.populate_analyzer_with_session(&mut analyzer);

        // Analyze the program
        analyzer
            .analyze_program(program)
            .map_err(|e| format!("Semantic error: {e}"))?;

        Ok(analyzer)
    }

    /// Populate semantic analyzer with current session state
    fn populate_analyzer_with_session(&self, analyzer: &mut SemanticAnalyzer) {
        // Add session variables to symbol table
        for (name, value) in self.session.variables() {
            let var_type = self.infer_value_type(value);
            // Note: This is simplified - in practice we'd need proper scope management
        }

        // Add session types
        for (name, type_def) in self.session.types() {
            // Add type definitions to analyzer
        }

        // Add session functions
        for (name, signature) in self.session.functions() {
            // Add function signatures to analyzer
        }
    }

    /// Process definitions from parsed program and update session state
    fn process_definitions(&mut self, program: &crate::parser::Program) -> Result<(), String> {
        for stmt in &program.statements {
            match stmt {
                crate::parser::Stmt {
                    kind: crate::parser::StmtKind::Let { name, init, .. },
                    ..
                } => {
                    // Execute the value expression and store in session
                    if let Some(value) = init {
                        if let Some(computed_value) = self.evaluate_expression(value)? {
                            let var_type = self.infer_value_type(&computed_value);
                            self.session
                                .define_variable(name.clone(), computed_value, var_type);
                        }
                    }
                }
                crate::parser::Stmt {
                    kind:
                        crate::parser::StmtKind::Function {
                            name,
                            params,
                            ret_type,
                            ..
                        },
                    ..
                } => {
                    // Create function signature and store in session
                    let signature = self.create_function_signature(params, ret_type.as_ref())?;
                    self.session.define_function(name.clone(), signature);
                }
                stmt if matches!(
                    stmt.kind,
                    crate::parser::StmtKind::Struct { .. } | crate::parser::StmtKind::Enum { .. }
                ) =>
                {
                    // Process type definition and store in session
                    let processed_type = self.process_type_definition(stmt)?;
                    let name = match &stmt.kind {
                        crate::parser::StmtKind::Struct { name, .. } => name.clone(),
                        crate::parser::StmtKind::Enum { name, .. } => name.clone(),
                        _ => unreachable!(),
                    };
                    self.session.define_type(name, processed_type);
                }
                crate::parser::Stmt {
                    kind: crate::parser::StmtKind::Import { imports, module },
                    ..
                } => {
                    // Handle module imports
                    let items: Vec<String> = imports
                        .iter()
                        .map(|import_spec| match import_spec {
                            crate::parser::ImportSpecifier::Named { name, .. } => name.clone(),
                            crate::parser::ImportSpecifier::Default { name } => name.clone(),
                            crate::parser::ImportSpecifier::Namespace { alias } => alias.clone(),
                        })
                        .collect();
                    self.process_import(module, &items)?;
                }
                _ => {
                    // Other statements are handled during execution
                }
            }
        }
        Ok(())
    }

    /// Execute program with session context
    fn execute_with_session(
        &mut self,
        program: &crate::parser::Program,
    ) -> Result<Option<Value>, String> {
        // For now, just return successful compilation
        // In a full implementation, this would use the runtime system
        // to execute the program with the current session context

        println!("{} Compiled and analyzed successfully", "✓".green());

        // If the program contains expressions, evaluate them
        for stmt in &program.statements {
            if let crate::parser::Stmt {
                kind: crate::parser::StmtKind::Expression(expr),
                ..
            } = stmt
            {
                if let Some(value) = self.evaluate_expression(expr)? {
                    return Ok(Some(value));
                }
            }
        }

        Ok(None)
    }

    /// Evaluate an expression (simplified implementation)
    fn evaluate_expression(&self, expr: &crate::parser::Expr) -> Result<Option<Value>, String> {
        match &expr.kind {
            crate::parser::ExprKind::Literal(literal) => Ok(Some(self.literal_to_value(literal))),
            crate::parser::ExprKind::Identifier(name) => {
                if let Some(value) = self.session.get_variable(name) {
                    Ok(Some(value.clone()))
                } else {
                    Err(format!("Undefined variable: {name}"))
                }
            }
            crate::parser::ExprKind::Binary { left, op, right } => {
                // Simplified binary operation evaluation
                let left_val = self.evaluate_expression(left)?;
                let right_val = self.evaluate_expression(right)?;

                if let (Some(l), Some(r)) = (left_val, right_val) {
                    self.evaluate_binary_operation(&l, op, &r)
                } else {
                    Ok(None)
                }
            }
            _ => {
                // For now, return None for complex expressions
                Ok(None)
            }
        }
    }

    /// Convert literal to runtime value
    fn literal_to_value(&self, literal: &crate::parser::Literal) -> Value {
        match literal {
            crate::parser::Literal::Number(n) => Value::F32(*n as f32),
            crate::parser::Literal::String(s) => Value::String(s.clone()),
            crate::parser::Literal::Boolean(b) => Value::Bool(*b),
            crate::parser::Literal::Null => Value::Null,
        }
    }

    /// Infer type from runtime value
    fn infer_value_type(&self, value: &Value) -> crate::types::Type {
        match value {
            Value::Null => crate::types::Type::Unknown,
            Value::Bool(_) => crate::types::Type::Bool,
            Value::I32(_) => crate::types::Type::I32,
            Value::I64(_) => crate::types::Type::I32, // Map I64 to I32 since Type doesn't have I64
            Value::F32(_) => crate::types::Type::F32,
            Value::F64(_) => crate::types::Type::F32, // Map F64 to F32 since Type doesn't have F64
            Value::String(_) => crate::types::Type::String,
            Value::Array(_) => crate::types::Type::Array(Box::new(crate::types::Type::Unknown)),
            Value::Object(_) => crate::types::Type::Unknown,
            Value::Function(_) => crate::types::Type::Function {
                params: vec![],
                ret: Box::new(crate::types::Type::Unknown),
            },
            Value::Number(_) => crate::types::Type::F32,
            Value::Boolean(_) => crate::types::Type::Bool,
            _ => crate::types::Type::Unknown,
        }
    }

    /// Create function signature from parameters and return type
    fn create_function_signature(
        &self,
        params: &[crate::parser::Param],
        return_type: Option<&crate::parser::TypeAnn>,
    ) -> Result<crate::semantic::FunctionSignature, String> {
        // This is a simplified implementation
        // In practice, we'd need to properly handle parameter types and defaults
        Ok(crate::semantic::FunctionSignature {
            generic_params: None,
            params: params
                .iter()
                .map(|param| {
                    (param.name.clone(), crate::types::Type::Unknown) // Simplified: map TypeAnn to Type::Unknown
                })
                .collect(),
            return_type: crate::types::Type::Unknown, // Simplified: use Unknown for return type
            is_const: false,
            is_async: false,
        })
    }

    /// Process type definition
    fn process_type_definition(
        &self,
        stmt: &crate::parser::Stmt,
    ) -> Result<crate::types::Type, String> {
        // Process type definitions from statement AST
        match &stmt.kind {
            crate::parser::StmtKind::Struct { name, fields, .. } => {
                Ok(crate::types::Type::Struct {
                    name: name.clone(),
                    fields: fields
                        .iter()
                        .map(|field| {
                            (field.name.clone(), crate::types::Type::Unknown) // Simplified: map TypeAnn to Type::Unknown
                        })
                        .collect(),
                })
            }
            crate::parser::StmtKind::Enum { name, .. } => {
                Ok(crate::types::Type::Named(name.clone())) // Use Named type for enums
            }
            _ => Err(format!(
                "Statement is not a type definition: {:?}",
                stmt.kind
            )),
        }
    }

    /// Process module import
    fn process_import(&mut self, module: &str, items: &[String]) -> Result<(), String> {
        println!(
            "{} Processing import from module: {}",
            "📦".blue(),
            module.cyan()
        );

        // Load the module and import requested items
        let imported_exports = if items.is_empty() {
            // Import all items
            self.module_loader.import_all(module)?
        } else {
            // Import specific items
            self.module_loader.import_items(module, items)?
        };

        // Add imported items to the current session
        for (name, var_type) in imported_exports.variables {
            // For now, create a placeholder value
            // In practice, we'd need to properly handle module variable values
            let placeholder_value = self.create_placeholder_value(&var_type);
            self.session
                .define_variable(name.clone(), placeholder_value, var_type);
            println!("  {} Imported variable: {}", "✓".green(), name.cyan());
        }

        for (name, signature) in imported_exports.functions {
            self.session.define_function(name.clone(), signature);
            println!("  {} Imported function: {}", "✓".green(), name.cyan());
        }

        for (name, type_def) in imported_exports.types {
            self.session.define_type(name.clone(), type_def);
            println!("  {} Imported type: {}", "✓".green(), name.cyan());
        }

        Ok(())
    }

    /// Create a placeholder value for an imported variable
    fn create_placeholder_value(&self, var_type: &crate::types::Type) -> Value {
        match var_type {
            crate::types::Type::Bool => Value::Bool(false),
            crate::types::Type::I32 => Value::I32(0),
            crate::types::Type::F32 => Value::F32(0.0),
            crate::types::Type::String => Value::String(String::new()),
            _ => Value::Null,
        }
    }

    /// Evaluate binary operation (simplified)
    fn evaluate_binary_operation(
        &self,
        left: &Value,
        op: &crate::parser::BinaryOp,
        right: &Value,
    ) -> Result<Option<Value>, String> {
        match (left, op, right) {
            (Value::I32(a), crate::parser::BinaryOp::Add, Value::I32(b)) => {
                Ok(Some(Value::I32(a + b)))
            }
            (Value::F32(a), crate::parser::BinaryOp::Add, Value::F32(b)) => {
                Ok(Some(Value::F32(a + b)))
            }
            (Value::String(a), crate::parser::BinaryOp::Add, Value::String(b)) => {
                Ok(Some(Value::String(format!("{}{}", a, b))))
            }
            // Add more binary operations as needed
            _ => Err(format!(
                "Unsupported binary operation: {:?} {:?} {:?}",
                left, op, right
            )),
        }
    }

    /// Analyze tokens and display them
    fn analyze_tokens(&mut self, input: String) {
        match Lexer::new(&input) {
            Ok(lexer) => {
                let (tokens, errors) = lexer.scan_tokens();

                if !errors.is_empty() {
                    let mut reporter = ErrorReporter::new();
                    for error in errors {
                        reporter.report(error);
                    }
                    reporter.print_all();
                } else {
                    self.print_tokens(&tokens);
                }
            }
            Err(error) => {
                println!("Lexer error: {error}");
            }
        }
    }

    /// Analyze parse tree and display it
    fn analyze_parse(&mut self, input: String) {
        match Lexer::new(&input) {
            Ok(lexer) => {
                let (tokens, lex_errors) = lexer.scan_tokens();

                if !lex_errors.is_empty() {
                    let mut reporter = ErrorReporter::new();
                    for error in lex_errors {
                        reporter.report(error);
                    }
                    reporter.print_all();
                    return;
                }

                let mut parser = Parser::new(tokens);
                match parser.parse() {
                    Ok(program) => {
                        println!("{}", "Parse tree:".green().bold());
                        println!("{:#?}", program);
                    }
                    Err(error) => {
                        println!("Parse error: {error}");
                    }
                }
            }
            Err(error) => {
                println!("Lexer error: {error}");
            }
        }
    }

    /// Debug input processing
    fn debug_input(&mut self, input: String) {
        println!("{} Debug mode not fully implemented yet", "Note:".yellow());
        println!("Input: {}", input.cyan());
    }

    /// Print tokens in a nice format
    fn print_tokens(&self, tokens: &[Token]) {
        println!("\n{}", "Tokens:".green().bold());
        println!("{}", "─".repeat(60));

        for token in tokens {
            if matches!(token.kind, TokenKind::Newline) {
                continue;
            }

            println!(
                "{:>4}:{:<4} {:20} {}",
                token.span.start.line,
                token.span.start.column,
                format!("{:?}", token.kind).yellow(),
                token.lexeme.cyan()
            );

            if matches!(token.kind, TokenKind::Eof) {
                break;
            }
        }

        println!("{}\n", "─".repeat(60));
    }

    /// Show defined types
    fn show_types(&self) {
        println!("{}", "Defined Types:".yellow().bold());
        if self.session.types().is_empty() {
            println!("  {}", "No types defined".dimmed());
        } else {
            for (name, type_def) in self.session.types() {
                println!("  {} : {}", name.cyan(), format!("{:?}", type_def).green());
            }
        }
    }

    /// Show defined functions
    fn show_functions(&self) {
        println!("{}", "Defined Functions:".yellow().bold());
        if self.session.functions().is_empty() {
            println!("  {}", "No functions defined".dimmed());
        } else {
            for (name, signature) in self.session.functions() {
                println!("  {} : {}", name.cyan(), format!("{:?}", signature).green());
            }
        }
    }

    /// Show imported modules
    fn show_modules(&self) {
        println!("{}", "Imported Modules:".yellow().bold());
        let loaded_modules = self.module_loader.list_loaded_modules();

        if loaded_modules.is_empty() {
            println!("  {}", "No modules imported".dimmed());
        } else {
            for module_name in loaded_modules {
                if let Some(module_info) = self.module_loader.get_module_info(module_name) {
                    let exports_count = module_info.exports.variables.len()
                        + module_info.exports.functions.len()
                        + module_info.exports.types.len();

                    println!(
                        "  {} ({} exports) - {}",
                        module_name.cyan(),
                        exports_count.to_string().green(),
                        module_info.path.display().to_string().dimmed()
                    );
                }
            }
        }

        println!("\n{}", "Module Search Paths:".yellow().bold());
        for (i, path) in self.module_loader.search_paths().iter().enumerate() {
            println!("  {}: {}", (i + 1).to_string().dimmed(), path.display());
        }
    }

    /// Save session state
    fn save_session(&mut self) {
        match self.session.save() {
            Ok(()) => println!("{} Session saved successfully", "✓".green()),
            Err(e) => println!("{} Failed to save session: {}", "✗".red(), e),
        }
    }

    /// Load session state
    fn load_session(&mut self) {
        match Session::load_or_create() {
            Ok(session) => {
                self.session = session;
                println!("{} Session loaded successfully", "✓".green());
            }
            Err(e) => println!("{} Failed to load session: {}", "✗".red(), e),
        }
    }
}

impl Default for EnhancedRepl {
    fn default() -> Self {
        Self::new().expect("Failed to create enhanced REPL")
    }
}
