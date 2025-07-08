//! Common benchmark utilities and adapter layer
//!
//! This module provides a compatibility layer for benchmarks to work with
//! the current implementation state of the Script language.

use script::{AstLowerer, CodeGenerator, Lexer, Parser, SymbolTable};
use std::collections::HashMap;

/// Adapter layer that provides a simplified interface for benchmarks
pub struct BenchmarkAdapter;

/// Compiled program representation for benchmarks
pub struct CompiledProgram {
    pub executable: script::ExecutableModule,
}

impl BenchmarkAdapter {
    /// Prepare a compiled program from source code (following the main.rs pattern)
    pub fn prepare_program(source: &str) -> Result<CompiledProgram, String> {
        // Lexing
        let lexer = Lexer::new(source).unwrap();
        let (tokens, lex_errors) = lexer.scan_tokens();

        if !lex_errors.is_empty() {
            return Err(format!("Lexer errors: {:?}", lex_errors));
        }

        // Parsing
        let mut parser = Parser::new(tokens);
        let program = parser
            .parse()
            .map_err(|e| format!("Parser error: {:?}", e))?;

        // For benchmarks, use empty symbol table and type info (like main.rs)
        let symbol_table = SymbolTable::new();
        let type_info = HashMap::new();

        // Lower to IR
        let mut lowerer = AstLowerer::new(symbol_table, type_info, Vec::new());
        let ir_module = lowerer
            .lower_program(&program)
            .map_err(|e| format!("IR lowering error: {:?}", e))?;

        // Generate code
        let mut codegen = CodeGenerator::new();
        let executable = codegen
            .generate(&ir_module)
            .map_err(|e| format!("Code generation error: {:?}", e))?;

        Ok(CompiledProgram { executable })
    }

    /// Execute a compiled program (if execution is supported)
    pub fn execute_program(_program: &CompiledProgram) -> Result<(), String> {
        // For now, since execution isn't fully implemented, we'll just validate
        // that the program compiled successfully
        Ok(())
    }

    /// Simple compilation benchmark that just measures parse time
    pub fn parse_only(source: &str) -> Result<script::Program, String> {
        let lexer = Lexer::new(source).unwrap();
        let (tokens, lex_errors) = lexer.scan_tokens();

        if !lex_errors.is_empty() {
            return Err(format!("Lexer errors: {:?}", lex_errors));
        }

        let mut parser = Parser::new(tokens);
        parser.parse().map_err(|e| format!("Parser error: {:?}", e))
    }
}

/// Simplified source code patterns that work with current implementation
pub mod simple_patterns {
    /// Basic arithmetic operations
    pub const ARITHMETIC: &str = r#"
        let a = 10
        let b = 20
        let sum = a + b
        let product = a * b
        let difference = b - a
        let quotient = b / a
    "#;

    /// Simple function calls
    pub const FUNCTION_CALLS: &str = r#"
        fn add(x, y) {
            return x + y
        }
        
        fn multiply(x, y) {
            return x * y
        }
        
        let result1 = add(5, 3)
        let result2 = multiply(4, 6)
        let result3 = add(result1, result2)
    "#;

    /// Basic control flow
    pub const CONTROL_FLOW: &str = r#"
        let x = 15
        
        if x > 10 {
            let msg = "x is greater than 10"
        } else {
            let msg = "x is not greater than 10"
        }
        
        let i = 0
        while i < 5 {
            i = i + 1
        }
    "#;

    /// Simple data structures
    pub const DATA_STRUCTURES: &str = r#"
        let arr = [1, 2, 3, 4, 5]
        let first = arr[0]
        let last = arr[4]
        
        let sum = 0
        let index = 0
        while index < 5 {
            sum = sum + arr[index]
            index = index + 1
        }
    "#;

    /// Recursive fibonacci (simple version)
    pub const FIBONACCI_SIMPLE: &str = r#"
        fn fibonacci(n) {
            if n <= 1 {
                return n
            }
            return fibonacci(n - 1) + fibonacci(n - 2)
        }
        
        let result = fibonacci(10)
    "#;

    /// Iterative fibonacci
    pub const FIBONACCI_ITERATIVE: &str = r#"
        fn fibonacci_iter(n) {
            if n <= 1 {
                return n
            }
            
            let a = 0
            let b = 1
            let i = 2
            
            while i <= n {
                let temp = a + b
                a = b
                b = temp
                i = i + 1
            }
            
            return b
        }
        
        let result = fibonacci_iter(20)
    "#;

    /// String operations
    pub const STRING_OPERATIONS: &str = r#"
        let greeting = "Hello"
        let name = "World"
        let message = greeting + " " + name + "!"
        
        let count = 0
        while count < 10 {
            let temp = message + " " + count
            count = count + 1
        }
    "#;

    /// Large computation
    pub const LARGE_COMPUTATION: &str = r#"
        fn compute_sum(limit) {
            let sum = 0
            let i = 1
            while i <= limit {
                sum = sum + i
                i = i + 1
            }
            return sum
        }
        
        fn compute_product(limit) {
            let product = 1
            let i = 1
            while i <= limit {
                product = product * i
                if product > 1000000 {
                    product = product / 1000
                }
                i = i + 1
            }
            return product
        }
        
        let sum_result = compute_sum(1000)
        let product_result = compute_product(20)
        let final_result = sum_result + product_result
    "#;
}
