#!/usr/bin/env rust-script

//! Simple performance test for Script language lexer
//! Provides basic performance comparison with other languages

use std::time::{Duration, Instant};

// Include the lexer from the script crate
use script::lexer::{Lexer, TokenKind};

/// Test program for lexing
const TEST_PROGRAM: &str = r#"
fn fibonacci(n: i32) -> i32 {
    if n <= 1 {
        return n
    }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

fn factorial(n: i32) -> i32 {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

let result = fibonacci(10)
let fact_result = factorial(5)
print("Fibonacci(10): " + result)
print("Factorial(5): " + fact_result)

// Test more complex features
struct Point {
    x: f32,
    y: f32
}

enum Status {
    Ok(i32),
    Error(string)
}

let point = Point { x: 3.14, y: 2.71 }
let status = Status::Ok(42)

match status {
    Status::Ok(value) => print("Success: " + value),
    Status::Error(msg) => print("Error: " + msg)
}
"#;

/// Large test program
fn generate_large_program() -> String {
    let mut program = String::new();
    
    // Generate many functions
    for i in 0..100 {
        program.push_str(&format!(r#"
fn function_{i}(param1: i32, param2: f32, param3: string) -> i32 {{
    let local_var_{i} = param1 * {i}
    let another_var_{i} = param2 + {i}.0
    let string_var_{i} = param3 + "suffix_{i}"
    
    if local_var_{i} > {i} {{
        return local_var_{i} + {i}
    }} else {{
        return another_var_{i} as i32
    }}
}}
"#));

        // Generate some data structures
        program.push_str(&format!(r#"
struct Data_{i} {{
    field1: i32,
    field2: f32,
    field3: string,
    field4: bool
}}

let instance_{i} = Data_{i} {{
    field1: {i},
    field2: {i}.5,
    field3: "value_{i}",
    field4: {is_even}
}}
"#, is_even = i % 2 == 0));
    }
    
    program
}

/// Benchmark the lexer
fn benchmark_lexer(name: &str, source: &str, iterations: usize) -> Duration {
    let start = Instant::now();
    
    for _ in 0..iterations {
        match Lexer::new(source) {
            Ok(lexer) => {
                let (tokens, errors) = lexer.scan_tokens();
                
                // Count tokens to prevent optimization
                let token_count = tokens.len();
                let error_count = errors.len();
                
                // Simple check to ensure we're actually processing
                if token_count == 0 && !source.trim().is_empty() {
                    println!("Warning: No tokens found for non-empty source");
                }
                if error_count > 0 {
                    println!("Warning: {} lexer errors found", error_count);
                }
            }
            Err(e) => {
                println!("Lexer creation failed: {}", e);
                break;
            }
        }
    }
    
    let elapsed = start.elapsed();
    println!("{}: {} iterations in {:?} ({:.2} tokens/sec)", 
             name, 
             iterations, 
             elapsed,
             (iterations as f64) / elapsed.as_secs_f64());
    
    elapsed
}

/// Get rough performance metrics
fn analyze_complexity(source: &str) -> (usize, usize, usize, usize) {
    let lines = source.lines().count();
    let chars = source.len();
    let words = source.split_whitespace().count();
    let identifiers = source.split(|c: char| !c.is_alphanumeric() && c != '_')
                           .filter(|s| !s.is_empty() && s.chars().next().unwrap().is_alphabetic())
                           .count();
    
    (lines, chars, words, identifiers)
}

fn main() {
    println!("=== Script Language Lexer Performance Test ===\n");
    
    // Test with small program
    println!("Testing small program...");
    let (lines, chars, words, identifiers) = analyze_complexity(TEST_PROGRAM);
    println!("Program stats: {} lines, {} chars, {} words, {} identifiers", 
             lines, chars, words, identifiers);
    
    let small_time = benchmark_lexer("Small Program", TEST_PROGRAM, 1000);
    
    // Test with large program
    println!("\nTesting large program...");
    let large_program = generate_large_program();
    let (lines, chars, words, identifiers) = analyze_complexity(&large_program);
    println!("Program stats: {} lines, {} chars, {} words, {} identifiers", 
             lines, chars, words, identifiers);
    
    let large_time = benchmark_lexer("Large Program", &large_program, 100);
    
    // Calculate throughput
    println!("\n=== Performance Analysis ===");
    
    let small_chars_per_sec = (TEST_PROGRAM.len() * 1000) as f64 / small_time.as_secs_f64();
    let large_chars_per_sec = (large_program.len() * 100) as f64 / large_time.as_secs_f64();
    
    println!("Small program throughput: {:.0} chars/sec", small_chars_per_sec);
    println!("Large program throughput: {:.0} chars/sec", large_chars_per_sec);
    
    // Rough comparison with other language parsers
    println!("\n=== Rough Performance Comparison ===");
    println!("Script Lexer:     {:.0} chars/sec", large_chars_per_sec);
    println!("Typical ranges for comparison:");
    println!("  Python AST:     ~100,000 - 500,000 chars/sec");
    println!("  JavaScript:     ~500,000 - 2,000,000 chars/sec");
    println!("  Rust Parser:    ~1,000,000 - 5,000,000 chars/sec");
    println!("  C++ Clang:      ~2,000,000 - 10,000,000 chars/sec");
    println!("  Go Parser:      ~1,000,000 - 3,000,000 chars/sec");
    
    // Determine relative performance
    let performance_tier = match large_chars_per_sec as u64 {
        n if n >= 5_000_000 => "Excellent (comparable to production compilers)",
        n if n >= 1_000_000 => "Good (competitive with modern languages)", 
        n if n >= 500_000 => "Decent (usable for development)",
        n if n >= 100_000 => "Slow (needs optimization)",
        _ => "Very slow (requires significant optimization)"
    };
    
    println!("\nScript Language Performance: {}", performance_tier);
    
    // Memory usage estimation
    println!("\n=== Memory Usage (Estimates) ===");
    
    // Test actual memory usage by tokenizing and measuring
    if let Ok(lexer) = Lexer::new(&large_program) {
        let (tokens, _) = lexer.scan_tokens();
        let token_count = tokens.len();
        let estimated_memory = token_count * std::mem::size_of::<TokenKind>() * 3; // rough estimate
        
        println!("Large program tokens: {}", token_count);
        println!("Estimated token memory: {} bytes ({:.1} KB)", 
                 estimated_memory, estimated_memory as f64 / 1024.0);
        
        let memory_efficiency = large_program.len() as f64 / estimated_memory as f64;
        println!("Memory efficiency: {:.2}x (source bytes per token memory byte)", memory_efficiency);
    }
    
    println!("\n=== Recommendations ===");
    if large_chars_per_sec < 500_000.0 {
        println!("- Consider optimizing string handling in lexer");
        println!("- Profile token allocation patterns");
        println!("- Consider using string interning for identifiers");
    } else if large_chars_per_sec < 1_000_000.0 {
        println!("- Performance is adequate for development");
        println!("- Consider minor optimizations for production use");
    } else {
        println!("- Performance is competitive with modern languages");
        println!("- Focus on correctness and features rather than lexer optimization");
    }
}