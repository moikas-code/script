use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

mod common;
use common::BenchmarkAdapter;

/// Benchmark compilation performance (tooling related)
fn benchmark_compilation_tooling(c: &mut Criterion) {
    let mut group = c.benchmark_group("compilation_tooling");

    let sources = vec![
        (
            "small",
            r#"
            let x = 42
            let y = x + 10
            let result = x * y
        "#,
        ),
        (
            "medium",
            r#"
            fn add(a, b) {
                return a + b
            }
            
            fn multiply(a, b) {
                return a * b
            }
            
            fn calculate(x, y) {
                let sum = add(x, y)
                let product = multiply(x, y)
                return sum + product
            }
            
            let result = calculate(10, 20)
        "#,
        ),
        (
            "large",
            r#"
            fn fibonacci(n) {
                if n <= 1 {
                    return n
                }
                return fibonacci(n - 1) + fibonacci(n - 2)
            }
            
            fn factorial(n) {
                if n <= 1 {
                    return 1
                }
                return n * factorial(n - 1)
            }
            
            fn is_prime(n) {
                if n <= 1 {
                    return 0
                }
                if n <= 3 {
                    return 1
                }
                if n % 2 == 0 || n % 3 == 0 {
                    return 0
                }
                
                let i = 5
                while i * i <= n {
                    if n % i == 0 || n % (i + 2) == 0 {
                        return 0
                    }
                    i = i + 6
                }
                return 1
            }
            
            fn process_numbers(start, end) {
                let fib_sum = 0
                let fact_sum = 0
                let prime_count = 0
                
                let i = start
                while i <= end {
                    if i <= 20 {
                        fib_sum = fib_sum + fibonacci(i)
                    }
                    if i <= 10 {
                        fact_sum = fact_sum + factorial(i)
                    }
                    if is_prime(i) {
                        prime_count = prime_count + 1
                    }
                    i = i + 1
                }
                
                return fib_sum + fact_sum + prime_count
            }
            
            let result = process_numbers(1, 25)
        "#,
        ),
    ];

    for (name, source) in sources {
        group.bench_with_input(
            BenchmarkId::new("parse_time", name),
            &source,
            |b, source| b.iter(|| BenchmarkAdapter::prepare_program(black_box(source))),
        );
    }

    group.finish();
}

/// Benchmark repeated compilation (simulating IDE-like usage)
fn benchmark_incremental_compilation(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_compilation");

    let modifications = vec![
        (
            "add_variable",
            r#"
        fn base_function(x) {
            return x * 2
        }
        
        let base_value = 42
        let new_value = base_value + 10
    "#,
        ),
        (
            "add_function",
            r#"
        fn base_function(x) {
            return x * 2
        }
        
        fn new_function(y) {
            return y + base_function(y)
        }
        
        let base_value = 42
    "#,
        ),
        (
            "modify_function",
            r#"
        fn base_function(x) {
            return x * 3 + 1
        }
        
        let base_value = 42
        let result = base_function(base_value)
    "#,
        ),
    ];

    for (name, source) in modifications {
        group.bench_with_input(BenchmarkId::new("recompile", name), &source, |b, source| {
            b.iter(|| BenchmarkAdapter::prepare_program(black_box(source)))
        });
    }

    group.finish();
}

/// Benchmark error handling in compilation
fn benchmark_error_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_handling");

    let error_sources = vec![
        (
            "syntax_error",
            r#"
            let x = 42
            let y = x +
        "#,
        ),
        (
            "undefined_variable",
            r#"
            let x = undefined_var + 42
        "#,
        ),
        (
            "function_error",
            r#"
            fn broken_function(
                return x
            }
        "#,
        ),
    ];

    for (name, source) in error_sources {
        group.bench_with_input(
            BenchmarkId::new("handle_error", name),
            &source,
            |b, source| {
                b.iter(|| {
                    // This should fail but we want to benchmark the error handling
                    let _ = BenchmarkAdapter::prepare_program(black_box(source));
                })
            },
        );
    }

    group.finish();
}

/// Benchmark simple code analysis (without full LSP)
fn benchmark_code_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("code_analysis");

    let analysis_source = r#"
        fn fibonacci(n) {
            if n <= 1 {
                return n
            }
            return fibonacci(n - 1) + fibonacci(n - 2)
        }
        
        fn factorial(n) {
            if n <= 1 {
                return 1
            }
            return n * factorial(n - 1)
        }
        
        fn main() {
            let fib_result = fibonacci(10)
            let fact_result = factorial(5)
            return fib_result + fact_result
        }
        
        main()
    "#;

    group.bench_function("token_analysis", |b| {
        b.iter(|| {
            // Analyze at token level
            let lexer = script::Lexer::new(black_box(analysis_source));
            let (tokens, _) = lexer.scan_tokens();
            tokens.len() // Simple analysis: count tokens
        })
    });

    group.bench_function("ast_analysis", |b| {
        b.iter(|| {
            // Analyze at AST level
            let program = BenchmarkAdapter::parse_only(black_box(analysis_source))
                .expect("Failed to prepare for AST analysis");

            // Simple analysis: count statements
            program.statements.len()
        })
    });

    group.finish();
}

/// Benchmark batch processing (simulating build tools)
fn benchmark_batch_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_processing");

    let files = vec![
        r#"
            fn math_add(a, b) {
                return a + b
            }
        "#,
        r#"
            fn math_subtract(a, b) {
                return a - b
            }
        "#,
        r#"
            fn math_multiply(a, b) {
                return a * b
            }
        "#,
        r#"
            fn math_divide(a, b) {
                if b == 0 {
                    return 0
                }
                return a / b
            }
        "#,
        r#"
            fn main() {
                let result = math_add(10, math_multiply(5, 3))
                return result
            }
        "#,
    ];

    group.bench_function("compile_multiple_files", |b| {
        b.iter(|| {
            let mut compiled_count = 0;
            for source in &files {
                if BenchmarkAdapter::prepare_program(black_box(source)).is_ok() {
                    compiled_count += 1;
                }
            }
            compiled_count
        })
    });

    group.finish();
}

/// Benchmark parse-only vs full compilation
fn benchmark_parse_vs_compile(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_vs_compile");

    let test_source = r#"
        fn fibonacci(n) {
            if n <= 1 {
                return n
            }
            return fibonacci(n - 1) + fibonacci(n - 2)
        }
        
        fn factorial(n) {
            if n <= 1 {
                return 1
            }
            return n * factorial(n - 1)
        }
        
        let fib_result = fibonacci(15)
        let fact_result = factorial(8)
        let final_result = fib_result + fact_result
    "#;

    group.bench_function("parse_only", |b| {
        b.iter(|| BenchmarkAdapter::parse_only(black_box(test_source)))
    });

    group.bench_function("full_compilation", |b| {
        b.iter(|| BenchmarkAdapter::prepare_program(black_box(test_source)))
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_compilation_tooling,
    benchmark_incremental_compilation,
    benchmark_error_handling,
    benchmark_code_analysis,
    benchmark_batch_processing,
    benchmark_parse_vs_compile
);
criterion_main!(benches);
