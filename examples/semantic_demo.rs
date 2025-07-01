use script_lang::{Lexer, Parser, SemanticAnalyzer};

fn main() {
    let source = r#"
        // Define a function that adds two numbers
        fn add(x: i32, y: i32) -> i32 {
            x + y
        }
        
        // Define a factorial function
        fn factorial(n: i32) -> i32 {
            if n <= 1 {
                1
            } else {
                n * factorial(n - 1)
            }
        }
        
        // Use the functions
        let result = add(5, 3);
        let fact5 = factorial(5);
        
        // Variable shadowing in nested scope
        let x: i32 = 10;
        {
            let x: f32 = 3.14;
            print(x);
        }
        print(x); // Original x is still available
        
        // Arrays and loops
        let numbers = [1, 2, 3, 4, 5];
        for n in numbers {
            print(n);
        }
    "#;

    println!("=== Semantic Analysis Demo ===\n");
    println!("Source code:");
    println!("{}", source);
    println!("\n=== Analysis ===\n");

    // Tokenize
    let lexer = Lexer::new(source);
    let (tokens, lex_errors) = lexer.scan_tokens();
    
    if !lex_errors.is_empty() {
        println!("Lexer errors:");
        for err in lex_errors {
            println!("  {}", err);
        }
        return;
    }

    // Parse
    let mut parser = Parser::new(tokens);
    let program = match parser.parse() {
        Ok(program) => program,
        Err(err) => {
            println!("Parse error: {}", err);
            return;
        }
    };

    // Semantic analysis
    let mut analyzer = SemanticAnalyzer::new();
    match analyzer.analyze_program(&program) {
        Ok(()) => {
            println!("✓ Semantic analysis passed!");
            
            // Show symbol table information
            println!("\n=== Symbol Table ===");
            
            // Get all symbols in global scope
            let symbols = analyzer.symbol_table().get_current_scope_symbols();
            for symbol in symbols {
                println!("  {} - {}", symbol, if symbol.is_used { "used" } else { "unused" });
            }
        }
        Err(err) => {
            println!("Semantic error: {}", err);
        }
    }
}