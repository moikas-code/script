use colored::*;
use script_lang::{error::ErrorReporter, Lexer, Parser, Token};
use script_lang::{SymbolTable, AstLowerer, CodeGenerator};
use std::{env, fs, io::{self, Write}, path::Path, process, collections::HashMap};

#[derive(Debug, Clone, Copy)]
enum Mode {
    Tokens,
    Parse,
    Run,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 3 {
        eprintln!("Usage: {} [script file] [--tokens|--run]", args[0]);
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
            }
        }
        Err(e) => {
            eprintln!("{}: Could not read file '{}': {}", 
                     "Error".red().bold(), 
                     path.display(), 
                     e);
            process::exit(1);
        }
    }
}

fn run_repl() {
    println!("{} {} - The Script Programming Language", 
             "Script".cyan().bold(), 
             "v0.1.0".green());
    println!("Type 'exit' to quit");
    println!("Type ':tokens' to switch to token mode");
    println!("Type ':parse' to switch to parse mode (default)\n");
    
    let mut mode = Mode::Parse;

    loop {
        let prompt = match mode {
            Mode::Tokens => "tokens>",
            Mode::Parse => "script>",
            Mode::Run => "script>",
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

        if !line.is_empty() {
            match mode {
                Mode::Tokens => tokenize_and_display(line, None),
                Mode::Parse => parse_and_display(line, None),
                Mode::Run => {
                    println!("{} Run mode is not supported in REPL", "Note:".yellow());
                    println!("Use {} or {} mode instead", ":tokens".cyan(), ":parse".cyan());
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
        if matches!(token.kind, script_lang::TokenKind::Newline) {
            continue;
        }
        
        println!("{:>4}:{:<4} {:20} {}", 
                 token.span.start.line,
                 token.span.start.column,
                 format!("{:?}", token.kind).yellow(),
                 token.lexeme.cyan());
                 
        if matches!(token.kind, script_lang::TokenKind::Eof) {
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

fn run_program(source: &str, _file_name: Option<&str>) {
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
    
    // For now, skip semantic analysis and use empty symbol table
    let symbol_table = SymbolTable::new();
    let type_info = HashMap::new();
    
    // Lower to IR
    let mut lowerer = AstLowerer::new(symbol_table, type_info);
    let ir_module = match lowerer.lower_program(&program) {
        Ok(module) => module,
        Err(error) => {
            let mut reporter = ErrorReporter::new();
            reporter.report(error);
            reporter.print_all();
            return;
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
