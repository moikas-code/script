use criterion::{black_box, criterion_group, criterion_main, Criterion};

mod common;
use common::{simple_patterns, BenchmarkAdapter, CompiledProgram};

/// Benchmark basic arithmetic operations
fn benchmark_arithmetic_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("arithmetic_operations");

    group.bench_function("basic_arithmetic", |b| {
        b.iter(|| BenchmarkAdapter::prepare_program(black_box(simple_patterns::ARITHMETIC)))
    });

    group.finish();
}

/// Benchmark function call overhead
fn benchmark_function_calls(c: &mut Criterion) {
    let mut group = c.benchmark_group("function_calls");

    // Direct function calls
    group.bench_function("direct_calls", |b| {
        b.iter(|| BenchmarkAdapter::prepare_program(black_box(simple_patterns::FUNCTION_CALLS)))
    });

    // Recursive calls - simplified
    let recursive_calls = r#"
        fn factorial(n) {
            if n <= 1 {
                return 1
            }
            return n * factorial(n - 1)
        }
        
        let result1 = factorial(5)
        let result2 = factorial(6)
        let result3 = factorial(7)
    "#;

    group.bench_function("recursive_calls", |b| {
        b.iter(|| BenchmarkAdapter::prepare_program(black_box(recursive_calls)))
    });

    group.finish();
}

/// Benchmark control flow operations
fn benchmark_control_flow(c: &mut Criterion) {
    let mut group = c.benchmark_group("control_flow");

    group.bench_function("if_else_while", |b| {
        b.iter(|| BenchmarkAdapter::prepare_program(black_box(simple_patterns::CONTROL_FLOW)))
    });

    group.finish();
}

/// Benchmark array operations
fn benchmark_array_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("array_operations");

    // Basic array operations
    group.bench_function("basic_arrays", |b| {
        b.iter(|| BenchmarkAdapter::prepare_program(black_box(simple_patterns::DATA_STRUCTURES)))
    });

    // Array manipulation
    let array_manipulation = r#"
        let arr = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        let sum = 0
        let product = 1
        let i = 0
        
        while i < 10 {
            sum = sum + arr[i]
            if arr[i] <= 5 {
                product = product * arr[i]
            }
            i = i + 1
        }
        
        let avg = sum / 10
    "#;

    group.bench_function("array_manipulation", |b| {
        b.iter(|| BenchmarkAdapter::prepare_program(black_box(array_manipulation)))
    });

    group.finish();
}

/// Benchmark string operations
fn benchmark_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_operations");

    // String concatenation
    group.bench_function("concatenation", |b| {
        b.iter(|| BenchmarkAdapter::prepare_program(black_box(simple_patterns::STRING_OPERATIONS)))
    });

    group.finish();
}

/// Benchmark mathematical computations
fn benchmark_math_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("math_operations");

    // Basic arithmetic
    let arithmetic = r#"
        let sum = 0.0
        let product = 1.0
        let i = 1
        
        while i <= 100 {
            sum = sum + i
            product = product * (1.0 + 1.0 / i)
            if product > 1000.0 {
                product = product / 10.0
            }
            i = i + 1
        }
        
        let result = sum * product
    "#;

    group.bench_function("arithmetic", |b| {
        b.iter(|| BenchmarkAdapter::prepare_program(black_box(arithmetic)))
    });

    // Nested loops computation
    let nested_computation = r#"
        fn matrix_sum(size) {
            let total = 0
            let i = 0
            
            while i < size {
                let j = 0
                while j < size {
                    total = total + (i * size + j)
                    j = j + 1
                }
                i = i + 1
            }
            
            return total
        }
        
        let result = matrix_sum(20)
    "#;

    group.bench_function("nested_loops", |b| {
        b.iter(|| BenchmarkAdapter::prepare_program(black_box(nested_computation)))
    });

    group.finish();
}

/// Benchmark large computations
fn benchmark_large_computations(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_computations");

    group.bench_function("complex_computation", |b| {
        b.iter(|| BenchmarkAdapter::prepare_program(black_box(simple_patterns::LARGE_COMPUTATION)))
    });

    group.finish();
}

/// Benchmark parsing only (fast path)
fn benchmark_parsing_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing_only");

    group.bench_function("parse_arithmetic", |b| {
        b.iter(|| BenchmarkAdapter::parse_only(black_box(simple_patterns::ARITHMETIC)))
    });

    group.bench_function("parse_functions", |b| {
        b.iter(|| BenchmarkAdapter::parse_only(black_box(simple_patterns::FUNCTION_CALLS)))
    });

    group.bench_function("parse_large", |b| {
        b.iter(|| BenchmarkAdapter::parse_only(black_box(simple_patterns::LARGE_COMPUTATION)))
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_arithmetic_operations,
    benchmark_function_calls,
    benchmark_control_flow,
    benchmark_array_operations,
    benchmark_string_operations,
    benchmark_math_operations,
    benchmark_large_computations,
    benchmark_parsing_only
);
criterion_main!(benches);
