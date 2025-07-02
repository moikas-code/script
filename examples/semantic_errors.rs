use script::{Lexer, Parser, SemanticAnalyzer};

fn main() {
    println!("=== Semantic Error Detection Demo ===\n");

    // Example 1: Undefined variable
    test_semantic_analysis(
        "Example 1: Undefined variable",
        r#"
        let x = 10;
        y = 20;  // Error: y is not defined
    "#,
    );

    // Example 2: Duplicate variable
    test_semantic_analysis(
        "Example 2: Duplicate variable",
        r#"
        let x = 10;
        let x = 20;  // Error: x already defined
    "#,
    );

    // Example 3: Wrong function arguments
    test_semantic_analysis(
        "Example 3: Wrong function arguments",
        r#"
        fn add(x: i32, y: i32) -> i32 {
            x + y
        }
        add(1);  // Error: wrong number of arguments
    "#,
    );

    // Example 4: Undefined function
    test_semantic_analysis(
        "Example 4: Undefined function",
        r#"
        let result = multiply(5, 3);  // Error: multiply not defined
    "#,
    );

    // Example 5: Variable shadowing (should work)
    test_semantic_analysis(
        "Example 5: Variable shadowing (valid)",
        r#"
        let x = 10;
        {
            let x = 20;  // This is allowed - different scope
            print(x);
        }
        print(x);
    "#,
    );

    // Example 6: Return outside function
    test_semantic_analysis(
        "Example 6: Return outside function",
        r#"
        let x = 10;
        return x;  // Error: return outside function
    "#,
    );
}

fn test_semantic_analysis(title: &str, source: &str) {
    println!("\n{}", title);
    println!("{}", "-".repeat(title.len()));
    println!("Code:\n{}", source);

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
        }
        Err(err) => {
            println!("✗ Semantic error: {}", err);
        }
    }
}
