use colored::*;
use script::compilation::CompilationContext;
use script::debugger::{get_debugger, initialize_debugger, shutdown_debugger, Debugger};
use script::doc::{generator::DocGenerator, html::HtmlGenerator};
use script::testing::{ConsoleReporter, TestReporter, TestingFramework};
use script::{error::ErrorReporter, Lexer, Parser, SemanticAnalyzer, Token, TokenKind};
use script::{AstLowerer, CodeGenerator, SymbolTable};
use std::{
    collections::HashMap,
    env, fs,
    io::{self, Write},
    path::Path,
    process,
};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    Tokens,
    Parse,
    Run,
    Test,
    Doc,
    Debug,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Check for version flag
    if args.len() >= 2 && (args[1] == "--version" || args[1] == "-V") {
        println!("Script Language v{} (alpha - not production ready)", env!("CARGO_PKG_VERSION"));
        println!("⚠️  WARNING: Contains memory leaks, panic points, and incomplete features.");
        println!("Use for educational purposes and experimentation only.");
        return;
    }

    // Check for update command
    if args.len() >= 2 && args[1] == "update" {
        run_update_command(&args);
        return;
    }

    // Check for doc command
    if args.len() >= 2 && args[1] == "doc" {
        run_doc_command(&args);
        return;
    }

    // Check for debug command
    if args.len() >= 2 && args[1] == "debug" {
        run_debug_command(&args);
        return;
    }

    if args.len() > 3 {
        eprintln!(
            "Usage: {} [script file] [--tokens|--run|--test|--debug]",
            args[0]
        );
        eprintln!("   or: {} doc [source dir] [output dir]", args[0]);
        eprintln!("   or: {} debug [commands...]", args[0]);
        eprintln!(
            "   or: {} update [--check|--force|--version <version>]",
            args[0]
        );
        eprintln!("   or: {} --version", args[0]);
        process::exit(1);
    }

    if args.len() >= 2 {
        run_file(&args[1], &args);
    } else {
        run_repl();
    }
}

fn run_file(path: &str, args: &[String]) {
    let path = Path::new(path);

    // Check if it's a directory - if so, compile as a project
    if path.is_dir() {
        let mode = if args.len() > 2 && args[2] == "--run" {
            Mode::Run
        } else {
            Mode::Parse
        };

        if mode == Mode::Run {
            println!(
                "{} Compiling project in {}",
                "Script:".cyan().bold(),
                path.display()
            );
            compile_and_run_project(path);
        } else {
            println!(
                "{} Project compilation without --run is not yet supported",
                "Error".red().bold()
            );
            println!("Use: {} {} --run", args[0], path.display());
            process::exit(1);
        }
        return;
    }

    // Single file compilation
    if path.extension().and_then(|s| s.to_str()) != Some("script") {
        eprintln!("{}: File must have .script extension", "Error".red().bold());
        process::exit(1);
    }

    match fs::read_to_string(path) {
        Ok(source) => {
            let mode = if args.len() > 2 {
                match args[2].as_str() {
                    "--tokens" => Mode::Tokens,
                    "--run" => Mode::Run,
                    "--test" => Mode::Test,
                    "--debug" => Mode::Debug,
                    _ => Mode::Parse,
                }
            } else {
                Mode::Parse
            };

            match mode {
                Mode::Tokens => {
                    println!("{} Tokenizing {}", "Script:".cyan().bold(), path.display());
                    tokenize_and_display(&source, Some(path.to_string_lossy().as_ref()));
                }
                Mode::Parse => {
                    println!("{} Parsing {}", "Script:".cyan().bold(), path.display());
                    parse_and_display(&source, Some(path.to_string_lossy().as_ref()));
                }
                Mode::Run => {
                    println!("{} Running {}", "Script:".cyan().bold(), path.display());
                    run_program(&source, Some(path.to_string_lossy().as_ref()));
                }
                Mode::Test => {
                    println!("{} Testing {}", "Script:".cyan().bold(), path.display());
                    run_tests(&source, Some(path.to_string_lossy().as_ref()));
                }
                Mode::Debug => {
                    println!("{} Debugging {}", "Script:".cyan().bold(), path.display());
                    run_debug_session(&source, Some(path.to_string_lossy().as_ref()));
                }
                Mode::Doc => {
                    println!(
                        "{} Mode::Doc is not supported for single files",
                        "Error".red().bold()
                    );
                    println!("Use: {} doc <source dir> [output dir]", args[0]);
                    process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!(
                "{}: Could not read file '{}': {}",
                "Error".red().bold(),
                path.display(),
                e
            );
            process::exit(1);
        }
    }
}

fn run_repl() {
    println!(
        "{} {} - The Script Programming Language",
        "Script".cyan().bold(),
        "v0.1.0".green()
    );
    println!("{}", "⚠️  ALPHA VERSION - Not production ready".yellow());
    println!("Type 'exit' to quit");
    println!("Type ':tokens' to switch to token mode");
    println!("Type ':parse' to switch to parse mode (default)");
    println!("Type ':debug' to switch to debug mode\n");

    let mut mode = Mode::Parse;

    loop {
        let prompt = match mode {
            Mode::Tokens => "tokens>",
            Mode::Parse => "script>",
            Mode::Run => "script>",
            Mode::Test => "test>",
            Mode::Debug => "debug>",
            Mode::Doc => "doc>",
        };
        print!("{} ", prompt.cyan().bold());
        io::stdout().flush().unwrap();

        let mut line = String::new();
        io::stdin().read_line(&mut line).unwrap();

        let line = line.trim();
        if line == "exit" {
            println!("Goodbye!");
            break;
        }

        if line == ":tokens" {
            mode = Mode::Tokens;
            println!("Switched to token mode");
            continue;
        }

        if line == ":parse" {
            mode = Mode::Parse;
            println!("Switched to parse mode");
            continue;
        }

        if line == ":debug" {
            mode = Mode::Debug;
            println!("Switched to debug mode");
            continue;
        }

        if !line.is_empty() {
            match mode {
                Mode::Tokens => tokenize_and_display(line, None),
                Mode::Parse => parse_and_display(line, None),
                Mode::Run => {
                    println!("{} Run mode is not supported in REPL", "Note:".yellow());
                    println!(
                        "Use {} or {} mode instead",
                        ":tokens".cyan(),
                        ":parse".cyan()
                    );
                }
                Mode::Test => {
                    println!("{} Test mode is not supported in REPL", "Note:".yellow());
                    println!(
                        "Use {} or {} mode instead",
                        ":tokens".cyan(),
                        ":parse".cyan()
                    );
                }
                Mode::Debug => {
                    handle_debug_command(line);
                }
                Mode::Doc => {
                    println!("{} Doc mode is not supported in REPL", "Note:".yellow());
                    println!(
                        "Use {} or {} mode instead",
                        ":tokens".cyan(),
                        ":parse".cyan()
                    );
                }
            }
        }
    }
}

fn tokenize_and_display(source: &str, file_name: Option<&str>) {
    let lexer = Lexer::new(source);
    let (tokens, errors) = lexer.scan_tokens();

    if !errors.is_empty() {
        let mut reporter = ErrorReporter::new();
        for mut error in errors {
            if let Some(name) = file_name {
                error = error.with_file_name(name);
            }

            // Add source line context
            if let Some(loc) = error.location {
                if loc.line > 0 {
                    let lines: Vec<&str> = source.lines().collect();
                    if loc.line <= lines.len() {
                        error = error.with_source_line(lines[loc.line - 1]);
                    }
                }
            }

            reporter.report(error);
        }
        reporter.print_all();
    } else {
        print_tokens(&tokens);
    }
}

fn print_tokens(tokens: &[Token]) {
    println!("\n{}", "Tokens:".green().bold());
    println!("{}", "-".repeat(60));

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

    println!("{}\n", "-".repeat(60));
}

fn parse_and_display(source: &str, file_name: Option<&str>) {
    let lexer = Lexer::new(source);
    let (tokens, lex_errors) = lexer.scan_tokens();

    if !lex_errors.is_empty() {
        let mut reporter = ErrorReporter::new();
        for mut error in lex_errors {
            if let Some(name) = file_name {
                error = error.with_file_name(name);
            }

            // Add source line context
            if let Some(loc) = error.location {
                if loc.line > 0 {
                    let lines: Vec<&str> = source.lines().collect();
                    if loc.line <= lines.len() {
                        error = error.with_source_line(lines[loc.line - 1]);
                    }
                }
            }

            reporter.report(error);
        }
        reporter.print_all();
        return;
    }

    let mut parser = Parser::new(tokens);
    match parser.parse() {
        Ok(program) => {
            println!("\n{}", "AST:".green().bold());
            println!("{}", "-".repeat(60));
            println!("{}", program);
            println!("{}\n", "-".repeat(60));
        }
        Err(mut error) => {
            if let Some(name) = file_name {
                error = error.with_file_name(name);
            }

            // Add source line context
            if let Some(loc) = error.location {
                if loc.line > 0 {
                    let lines: Vec<&str> = source.lines().collect();
                    if loc.line <= lines.len() {
                        error = error.with_source_line(lines[loc.line - 1]);
                    }
                }
            }

            let mut reporter = ErrorReporter::new();
            reporter.report(error);
            reporter.print_all();
        }
    }
}

fn run_program(source: &str, file_name: Option<&str>) {
    // Lexing
    let lexer = Lexer::new(source);
    let (tokens, lex_errors) = lexer.scan_tokens();

    if !lex_errors.is_empty() {
        let mut reporter = ErrorReporter::new();
        for error in lex_errors {
            reporter.report(error);
        }
        reporter.print_all();
        return;
    }

    // Parsing
    let mut parser = Parser::new(tokens);
    let program = match parser.parse() {
        Ok(prog) => prog,
        Err(error) => {
            let mut reporter = ErrorReporter::new();
            reporter.report(error);
            reporter.print_all();
            return;
        }
    };

    // Perform semantic analysis
    let mut analyzer = SemanticAnalyzer::new();
    if let Err(error) = analyzer.analyze_program(&program) {
        let mut reporter = ErrorReporter::new();
        reporter.report(error);
        reporter.print_all();
        return;
    }

    // Check for semantic errors
    let errors = analyzer.errors();
    if !errors.is_empty() {
        let mut reporter = ErrorReporter::new();
        for error in errors {
            let mut err = error.clone().into_error();

            // Add file context
            if let Some(fname) = file_name {
                err = err.with_file_name(fname);
            }

            // Add source line context
            if let Some(loc) = err.location {
                if loc.line > 0 {
                    let lines: Vec<&str> = source.lines().collect();
                    if loc.line <= lines.len() {
                        err = err.with_source_line(lines[loc.line - 1]);
                    }
                }
            }

            reporter.report(err);
        }
        reporter.print_all();
        return;
    }

    // Extract type information, generic instantiations, and symbol table
    let type_info = analyzer.extract_type_info();
    let generic_instantiations = analyzer.generic_instantiations().to_vec();
    let symbol_table = analyzer.into_symbol_table();

    // Lower to IR
    let mut lowerer = AstLowerer::new(symbol_table, type_info.clone(), generic_instantiations.clone());
    let mut ir_module = match lowerer.lower_program(&program) {
        Ok(module) => module,
        Err(error) => {
            let mut reporter = ErrorReporter::new();
            reporter.report(error);
            reporter.print_all();
            return;
        }
    };

    // Monomorphize generic functions if any exist
    if !generic_instantiations.is_empty() {
        use script::codegen::monomorphization::MonomorphizationContext;
        
        let mut mono_context = MonomorphizationContext::new();
        mono_context.initialize_from_semantic_analysis(&generic_instantiations, &type_info);
        
        if let Err(error) = mono_context.monomorphize(&mut ir_module) {
            let mut reporter = ErrorReporter::new();
            reporter.report(error);
            reporter.print_all();
            return;
        }
        
        // Print monomorphization statistics if there were any generic functions
        let stats = mono_context.stats();
        if stats.functions_monomorphized > 0 {
            println!(
                "{} Monomorphized {} generic functions ({} instantiations, {} duplicates avoided)",
                "Info:".blue().bold(),
                stats.functions_monomorphized,
                stats.type_instantiations,
                stats.duplicates_avoided
            );
        }
    }

    // Generate code
    let mut codegen = CodeGenerator::new();
    let executable = match codegen.generate(&ir_module) {
        Ok(exec) => exec,
        Err(error) => {
            let mut reporter = ErrorReporter::new();
            reporter.report(error);
            reporter.print_all();
            return;
        }
    };

    // Execute
    match executable.execute() {
        Ok(exit_code) => {
            if exit_code != 0 {
                process::exit(exit_code);
            }
        }
        Err(error) => {
            let mut reporter = ErrorReporter::new();
            reporter.report(error);
            reporter.print_all();
            process::exit(1);
        }
    }
}

fn compile_and_run_project(dir: &Path) {
    let mut context = CompilationContext::new();

    let ir_module = match context.compile_directory(dir) {
        Ok(module) => module,
        Err(error) => {
            let mut reporter = ErrorReporter::new();
            reporter.report(error);
            reporter.print_all();
            process::exit(1);
        }
    };

    // Generate code
    let mut codegen = CodeGenerator::new();
    let executable = match codegen.generate(&ir_module) {
        Ok(exec) => exec,
        Err(error) => {
            let mut reporter = ErrorReporter::new();
            reporter.report(error);
            reporter.print_all();
            process::exit(1);
        }
    };

    // Execute
    match executable.execute() {
        Ok(exit_code) => {
            if exit_code != 0 {
                process::exit(exit_code);
            }
        }
        Err(error) => {
            let mut reporter = ErrorReporter::new();
            reporter.report(error);
            reporter.print_all();
            process::exit(1);
        }
    }
}

fn run_tests(source: &str, file_name: Option<&str>) {
    // Lexing
    let lexer = Lexer::new(source);
    let (tokens, lex_errors) = lexer.scan_tokens();

    if !lex_errors.is_empty() {
        let mut reporter = ErrorReporter::new();
        for mut error in lex_errors {
            if let Some(name) = file_name {
                error = error.with_file_name(name);
            }

            // Add source line context
            if let Some(loc) = error.location {
                if loc.line > 0 {
                    let lines: Vec<&str> = source.lines().collect();
                    if loc.line <= lines.len() {
                        error = error.with_source_line(lines[loc.line - 1]);
                    }
                }
            }

            reporter.report(error);
        }
        reporter.print_all();
        return;
    }

    // Parsing
    let mut parser = Parser::new(tokens);
    let program = match parser.parse() {
        Ok(prog) => prog,
        Err(mut error) => {
            if let Some(name) = file_name {
                error = error.with_file_name(name);
            }

            // Add source line context
            if let Some(loc) = error.location {
                if loc.line > 0 {
                    let lines: Vec<&str> = source.lines().collect();
                    if loc.line <= lines.len() {
                        error = error.with_source_line(lines[loc.line - 1]);
                    }
                }
            }

            let mut reporter = ErrorReporter::new();
            reporter.report(error);
            reporter.print_all();
            return;
        }
    };

    // Run tests using the testing framework
    let mut framework = TestingFramework::new();

    match framework.run_tests(&program) {
        Ok(summary) => {
            println!("\n{}", summary);

            if !summary.all_passed() {
                process::exit(1);
            }
        }
        Err(error) => {
            let mut reporter = ErrorReporter::new();
            reporter.report(error);
            reporter.print_all();
            process::exit(1);
        }
    }
}

fn run_debug_session(source: &str, file_name: Option<&str>) {
    // Lexing
    let lexer = Lexer::new(source);
    let (tokens, lex_errors) = lexer.scan_tokens();

    if !lex_errors.is_empty() {
        let mut reporter = ErrorReporter::new();
        for mut error in lex_errors {
            if let Some(name) = file_name {
                error = error.with_file_name(name);
            }

            // Add source line context
            if let Some(loc) = error.location {
                if loc.line > 0 {
                    let lines: Vec<&str> = source.lines().collect();
                    if loc.line <= lines.len() {
                        error = error.with_source_line(lines[loc.line - 1]);
                    }
                }
            }

            reporter.report(error);
        }
        reporter.print_all();
        return;
    }

    // Parsing
    let mut parser = Parser::new(tokens);
    let program = match parser.parse() {
        Ok(prog) => prog,
        Err(mut error) => {
            if let Some(name) = file_name {
                error = error.with_file_name(name);
            }

            // Add source line context
            if let Some(loc) = error.location {
                if loc.line > 0 {
                    let lines: Vec<&str> = source.lines().collect();
                    if loc.line <= lines.len() {
                        error = error.with_source_line(lines[loc.line - 1]);
                    }
                }
            }

            let mut reporter = ErrorReporter::new();
            reporter.report(error);
            reporter.print_all();
            return;
        }
    };

    // Start debugger session
    let mut debugger = Debugger::new();
    debugger.load_program(
        program,
        source.to_string(),
        file_name.map(|s| s.to_string()),
    );

    match debugger.start_session() {
        Ok(()) => {
            println!("{}", "Debug session ended.".green());
        }
        Err(error) => {
            eprintln!("{}: {}", "Debug Error".red().bold(), error);
            process::exit(1);
        }
    }
}

fn run_doc_command(args: &[String]) {
    if args.len() < 3 {
        eprintln!(
            "{}: doc command requires source directory",
            "Error".red().bold()
        );
        eprintln!("Usage: {} doc <source dir> [output dir]", args[0]);
        process::exit(1);
    }

    let source_dir = Path::new(&args[2]);
    let output_dir = if args.len() >= 4 {
        Path::new(&args[3])
    } else {
        Path::new("./docs")
    };

    if !source_dir.exists() {
        eprintln!(
            "{}: Source directory '{}' does not exist",
            "Error".red().bold(),
            source_dir.display()
        );
        process::exit(1);
    }

    println!("{} Generating documentation", "Script:".cyan().bold());
    println!("  Source: {}", source_dir.display());
    println!("  Output: {}", output_dir.display());

    // Create documentation generator
    let mut doc_generator = DocGenerator::new();

    // Process all .script files in the directory
    if let Err(e) = process_directory(&mut doc_generator, source_dir, "") {
        eprintln!("{}: {}", "Error".red().bold(), e);
        process::exit(1);
    }

    // Generate HTML documentation
    let html_generator = HtmlGenerator::new(output_dir);
    match html_generator.generate(doc_generator.database()) {
        Ok(_) => {
            println!(
                "\n{} Documentation generated successfully!",
                "Success:".green().bold()
            );
            println!(
                "Open {}/index.html to view the documentation",
                output_dir.display()
            );
        }
        Err(e) => {
            eprintln!("{}: Failed to generate HTML: {}", "Error".red().bold(), e);
            process::exit(1);
        }
    }
}

fn process_directory(
    doc_generator: &mut DocGenerator,
    dir: &Path,
    module_prefix: &str,
) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Recursively process subdirectories
            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            let new_prefix = if module_prefix.is_empty() {
                dir_name.to_string()
            } else {
                format!("{}::{}", module_prefix, dir_name)
            };

            process_directory(doc_generator, &path, &new_prefix)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("script") {
            // Process .script file
            let file_name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("");

            let module_name = if module_prefix.is_empty() {
                file_name.to_string()
            } else {
                format!("{}::{}", module_prefix, file_name)
            };

            println!("  Processing: {}", module_name);

            match fs::read_to_string(&path) {
                Ok(source) => {
                    if let Err(e) = doc_generator.generate_from_source(&source, &module_name) {
                        eprintln!("    {}: {}", "Warning".yellow(), e);
                    }
                }
                Err(e) => {
                    eprintln!(
                        "    {}: Could not read {}: {}",
                        "Warning".yellow(),
                        path.display(),
                        e
                    );
                }
            }
        }
    }

    Ok(())
}

/// Run the debug command interface
fn run_debug_command(args: &[String]) {
    if args.len() < 3 {
        print_debug_help();
        return;
    }

    // Initialize debugger
    if let Err(e) = initialize_debugger() {
        eprintln!(
            "{}: Failed to initialize debugger: {}",
            "Error".red().bold(),
            e
        );
        process::exit(1);
    }

    let command = &args[2];
    match command.as_str() {
        "help" | "-h" | "--help" => print_debug_help(),
        "break" | "b" => handle_breakpoint_command(&args[3..]),
        "list" | "l" => list_breakpoints(),
        "remove" | "rm" => remove_breakpoint_command(&args[3..]),
        "clear" => clear_all_breakpoints(),
        "enable" => enable_breakpoint_command(&args[3..]),
        "disable" => disable_breakpoint_command(&args[3..]),
        "stats" => show_breakpoint_stats(),
        _ => {
            eprintln!(
                "{}: Unknown debug command '{}'",
                "Error".red().bold(),
                command
            );
            print_debug_help();
            process::exit(1);
        }
    }

    // Shutdown debugger
    if let Err(e) = shutdown_debugger() {
        eprintln!("{}: Failed to shutdown debugger: {}", "Warning".yellow(), e);
    }
}

/// Print debug command help
fn print_debug_help() {
    println!("{} Debug Commands", "Script".cyan().bold());
    println!("{}", "-".repeat(50));
    println!(
        "  {} [line]                    Add line breakpoint",
        "break".green()
    );
    println!(
        "  {} [function]                Add function breakpoint",
        "break".green()
    );
    println!(
        "  {} [file] [line]             Add file-specific breakpoint",
        "break".green()
    );
    println!(
        "  {}                            List all breakpoints",
        "list".green()
    );
    println!(
        "  {} [id]                      Remove breakpoint by ID",
        "remove".green()
    );
    println!(
        "  {}                           Clear all breakpoints",
        "clear".green()
    );
    println!(
        "  {} [id]                     Enable breakpoint",
        "enable".green()
    );
    println!(
        "  {} [id]                    Disable breakpoint",
        "disable".green()
    );
    println!(
        "  {}                          Show breakpoint statistics",
        "stats".green()
    );
    println!(
        "  {}                           Show this help",
        "help".green()
    );
    println!();
    println!("Examples:");
    println!("  script debug break 10                # Break at line 10");
    println!("  script debug break main              # Break at function 'main'");
    println!("  script debug break test.script 15    # Break at line 15 in test.script");
    println!("  script debug list                    # List all breakpoints");
    println!("  script debug remove 1                # Remove breakpoint 1");
}

/// Handle breakpoint command
fn handle_breakpoint_command(args: &[String]) {
    if args.is_empty() {
        eprintln!("{}: break command requires arguments", "Error".red().bold());
        println!("Usage: script debug break [line|function|file line]");
        return;
    }

    let debugger = match get_debugger() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            return;
        }
    };

    let manager = debugger.breakpoint_manager();

    if args.len() == 1 {
        // Single argument - could be line number or function name
        let arg = &args[0];

        if let Ok(line) = arg.parse::<usize>() {
            // It's a line number - add line breakpoint for current file
            match manager.add_line_breakpoint("current.script".to_string(), line) {
                Ok(id) => {
                    println!(
                        "{} Added line breakpoint {} at line {}",
                        "Success:".green().bold(),
                        id,
                        line
                    );
                }
                Err(e) => {
                    eprintln!("{}: {}", "Error".red().bold(), e);
                }
            }
        } else {
            // It's a function name
            match manager.add_function_breakpoint(arg.to_string(), None) {
                Ok(id) => {
                    println!(
                        "{} Added function breakpoint {} for '{}'",
                        "Success:".green().bold(),
                        id,
                        arg
                    );
                }
                Err(e) => {
                    eprintln!("{}: {}", "Error".red().bold(), e);
                }
            }
        }
    } else if args.len() == 2 {
        // Two arguments - file and line
        let file = &args[0];
        if let Ok(line) = args[1].parse::<usize>() {
            match manager.add_line_breakpoint(file.to_string(), line) {
                Ok(id) => {
                    println!(
                        "{} Added line breakpoint {} at {}:{}",
                        "Success:".green().bold(),
                        id,
                        file,
                        line
                    );
                }
                Err(e) => {
                    eprintln!("{}: {}", "Error".red().bold(), e);
                }
            }
        } else {
            eprintln!(
                "{}: Invalid line number '{}'",
                "Error".red().bold(),
                args[1]
            );
        }
    } else {
        eprintln!(
            "{}: Too many arguments for break command",
            "Error".red().bold()
        );
        println!("Usage: script debug break [line|function|file line]");
    }
}

/// List all breakpoints
fn list_breakpoints() {
    let debugger = match get_debugger() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            return;
        }
    };

    let breakpoints = debugger.breakpoint_manager().get_all_breakpoints();

    if breakpoints.is_empty() {
        println!("{} No breakpoints set", "Info:".blue().bold());
        return;
    }

    println!("{} Breakpoints", "Script".cyan().bold());
    println!("{}", "-".repeat(60));

    for bp in breakpoints {
        let status = if bp.enabled { "enabled" } else { "disabled" };
        let status_color = if bp.enabled { "green" } else { "red" };

        println!(
            "  {:3}: {} [{}]",
            bp.id.to_string().cyan(),
            bp.description(),
            status.color(status_color)
        );

        if bp.hit_count > 0 {
            println!("       Hit {} times", bp.hit_count.to_string().yellow());
        }

        if let Some(condition) = &bp.condition {
            println!("       Condition: {}", condition.expression.cyan());
        }

        if let Some(message) = &bp.message {
            println!("       Message: {}", message.cyan());
        }
    }

    println!("{}", "-".repeat(60));
}

/// Remove a breakpoint by ID
fn remove_breakpoint_command(args: &[String]) {
    if args.is_empty() {
        eprintln!(
            "{}: remove command requires breakpoint ID",
            "Error".red().bold()
        );
        println!("Usage: script debug remove [id]");
        return;
    }

    let id_str = &args[0];
    let id = match id_str.parse::<usize>() {
        Ok(id) => id,
        Err(_) => {
            eprintln!(
                "{}: Invalid breakpoint ID '{}'",
                "Error".red().bold(),
                id_str
            );
            return;
        }
    };

    let debugger = match get_debugger() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            return;
        }
    };

    match debugger.breakpoint_manager().remove_breakpoint(id) {
        Ok(()) => {
            println!("{} Removed breakpoint {}", "Success:".green().bold(), id);
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
        }
    }
}

/// Clear all breakpoints
fn clear_all_breakpoints() {
    let debugger = match get_debugger() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            return;
        }
    };

    match debugger.breakpoint_manager().clear_all_breakpoints() {
        Ok(()) => {
            println!("{} Cleared all breakpoints", "Success:".green().bold());
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
        }
    }
}

/// Enable a breakpoint
fn enable_breakpoint_command(args: &[String]) {
    if args.is_empty() {
        eprintln!(
            "{}: enable command requires breakpoint ID",
            "Error".red().bold()
        );
        println!("Usage: script debug enable [id]");
        return;
    }

    let id_str = &args[0];
    let id = match id_str.parse::<usize>() {
        Ok(id) => id,
        Err(_) => {
            eprintln!(
                "{}: Invalid breakpoint ID '{}'",
                "Error".red().bold(),
                id_str
            );
            return;
        }
    };

    let debugger = match get_debugger() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            return;
        }
    };

    match debugger.breakpoint_manager().enable_breakpoint(id) {
        Ok(()) => {
            println!("{} Enabled breakpoint {}", "Success:".green().bold(), id);
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
        }
    }
}

/// Disable a breakpoint
fn disable_breakpoint_command(args: &[String]) {
    if args.is_empty() {
        eprintln!(
            "{}: disable command requires breakpoint ID",
            "Error".red().bold()
        );
        println!("Usage: script debug disable [id]");
        return;
    }

    let id_str = &args[0];
    let id = match id_str.parse::<usize>() {
        Ok(id) => id,
        Err(_) => {
            eprintln!(
                "{}: Invalid breakpoint ID '{}'",
                "Error".red().bold(),
                id_str
            );
            return;
        }
    };

    let debugger = match get_debugger() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            return;
        }
    };

    match debugger.breakpoint_manager().disable_breakpoint(id) {
        Ok(()) => {
            println!("{} Disabled breakpoint {}", "Success:".green().bold(), id);
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
        }
    }
}

/// Show breakpoint statistics
fn show_breakpoint_stats() {
    let debugger = match get_debugger() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            return;
        }
    };

    let stats = debugger.breakpoint_manager().get_statistics();
    println!("{}", stats);
}

/// Handle debug command in REPL mode
fn handle_debug_command(input: &str) {
    // Initialize debugger if not already done
    if let Err(_) = get_debugger() {
        if let Err(e) = initialize_debugger() {
            eprintln!(
                "{}: Failed to initialize debugger: {}",
                "Error".red().bold(),
                e
            );
            return;
        }
    }

    let parts: Vec<&str> = input.trim().split_whitespace().collect();
    if parts.is_empty() {
        println!("{} Debug commands:", "Available".cyan().bold());
        println!("  break [line|function]  - Add breakpoint");
        println!("  list                   - List breakpoints");
        println!("  remove [id]            - Remove breakpoint");
        println!("  clear                  - Clear all breakpoints");
        println!("  enable [id]            - Enable breakpoint");
        println!("  disable [id]           - Disable breakpoint");
        println!("  stats                  - Show statistics");
        return;
    }

    let command = parts[0];
    let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

    match command {
        "break" | "b" => handle_breakpoint_command(&args),
        "list" | "l" => list_breakpoints(),
        "remove" | "rm" => remove_breakpoint_command(&args),
        "clear" => clear_all_breakpoints(),
        "enable" => enable_breakpoint_command(&args),
        "disable" => disable_breakpoint_command(&args),
        "stats" => show_breakpoint_stats(),
        "help" => {
            println!("{} Debug commands:", "Available".cyan().bold());
            println!("  break [line|function]  - Add breakpoint");
            println!("  list                   - List breakpoints");
            println!("  remove [id]            - Remove breakpoint");
            println!("  clear                  - Clear all breakpoints");
            println!("  enable [id]            - Enable breakpoint");
            println!("  disable [id]           - Disable breakpoint");
            println!("  stats                  - Show statistics");
        }
        _ => {
            println!(
                "{}: Unknown debug command '{}'",
                "Error".red().bold(),
                command
            );
            println!("Type 'help' for available commands");
        }
    }
}

fn run_update_command(args: &[String]) {
    use script::update;

    if args.len() >= 3 {
        match args[2].as_str() {
            "--check" => match update::check_update() {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("{}: {}", "Error".red().bold(), e);
                    process::exit(1);
                }
            },
            "--list" => match update::list_versions() {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("{}: {}", "Error".red().bold(), e);
                    process::exit(1);
                }
            },
            "--force" => match update::update(true) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("{}: {}", "Error".red().bold(), e);
                    process::exit(1);
                }
            },
            "--version" => {
                if args.len() >= 4 {
                    match update::update_to_version(&args[3]) {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("{}: {}", "Error".red().bold(), e);
                            process::exit(1);
                        }
                    }
                } else {
                    eprintln!(
                        "{}: --version requires a version number",
                        "Error".red().bold()
                    );
                    eprintln!("Usage: {} update --version <version>", args[0]);
                    process::exit(1);
                }
            }
            "--rollback" => match update::rollback() {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("{}: {}", "Error".red().bold(), e);
                    process::exit(1);
                }
            },
            _ => {
                eprintln!(
                    "{}: Unknown update option '{}'",
                    "Error".red().bold(),
                    args[2]
                );
                eprintln!(
                    "Usage: {} update [--check|--force|--list|--version <version>|--rollback]",
                    args[0]
                );
                process::exit(1);
            }
        }
    } else {
        // Default: update with prompt
        match update::update(false) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("{}: {}", "Error".red().bold(), e);
                process::exit(1);
            }
        }
    }
}
