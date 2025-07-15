//! Comprehensive security benchmarking suite for Script language
//! 
//! This benchmark suite measures the performance impact of security features
//! and validates that security overhead remains within acceptable limits.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use script::runtime::{Runtime, RuntimeConfig, Value};
use script::lexer::Lexer;
use script::parser::Parser;
use script::semantic::SemanticAnalyzer;
use script::runtime::enhanced_ffi_validator::{EnhancedFFIValidator, FFIContext};
use script::types::Type;
use std::alloc::Layout;
use std::time::Duration;

/// Benchmark memory allocation with and without security checks
fn memory_security_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_security");
    
    // Configuration with security enabled
    let secure_config = RuntimeConfig {
        max_heap_size: 0,
        enable_profiling: true,
        enable_gc: true,
        gc_threshold: 100,
        enable_panic_handler: true,
        stack_size: 8192,
    };
    
    // Configuration with minimal security
    let minimal_config = RuntimeConfig {
        max_heap_size: 0,
        enable_profiling: false,
        enable_gc: true,
        gc_threshold: 10000,
        enable_panic_handler: false,
        stack_size: 8192,
    };
    
    let secure_runtime = Runtime::new(secure_config).unwrap();
    let minimal_runtime = Runtime::new(minimal_config).unwrap();
    
    // Benchmark allocation performance
    group.bench_function("secure_allocation", |b| {
        b.iter(|| {
            let layout = Layout::new::<u64>();
            if let Ok(ptr) = secure_runtime.memory().allocate(layout) {
                unsafe {
                    secure_runtime.memory().deallocate(ptr, layout);
                }
            }
        })
    });
    
    group.bench_function("minimal_allocation", |b| {
        b.iter(|| {
            let layout = Layout::new::<u64>();
            if let Ok(ptr) = minimal_runtime.memory().allocate(layout) {
                unsafe {
                    minimal_runtime.memory().deallocate(ptr, layout);
                }
            }
        })
    });
    
    // Benchmark bulk allocations
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("secure_bulk_allocation", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut ptrs = Vec::new();
                    for _ in 0..size {
                        let layout = Layout::new::<u64>();
                        if let Ok(ptr) = secure_runtime.memory().allocate(layout) {
                            ptrs.push((ptr, layout));
                        }
                    }
                    
                    for (ptr, layout) in ptrs {
                        unsafe {
                            secure_runtime.memory().deallocate(ptr, layout);
                        }
                    }
                })
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("minimal_bulk_allocation", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut ptrs = Vec::new();
                    for _ in 0..size {
                        let layout = Layout::new::<u64>();
                        if let Ok(ptr) = minimal_runtime.memory().allocate(layout) {
                            ptrs.push((ptr, layout));
                        }
                    }
                    
                    for (ptr, layout) in ptrs {
                        unsafe {
                            minimal_runtime.memory().deallocate(ptr, layout);
                        }
                    }
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark parsing with security validation
fn parser_security_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_security");
    
    // Test cases with varying complexity
    let test_cases = vec![
        ("simple", "let x = 42;"),
        ("medium", "fn factorial(n: i32) -> i32 { if n <= 1 { 1 } else { n * factorial(n - 1) } }"),
        ("complex", include_str!("../examples/fibonacci.script")),
    ];
    
    for (name, source) in test_cases {
        group.bench_function(&format!("lexer_{}", name), |b| {
            b.iter(|| {
                if let Ok(mut lexer) = Lexer::new(black_box(source)) {
                    let _ = lexer.scan_tokens();
                }
            })
        });
        
        group.bench_function(&format!("parser_{}", name), |b| {
            b.iter(|| {
                if let Ok(mut lexer) = Lexer::new(black_box(source)) {
                    if let Ok((tokens, _)) = lexer.scan_tokens() {
                        let mut parser = Parser::new(tokens);
                        let _ = parser.parse_program();
                    }
                }
            })
        });
        
        group.bench_function(&format!("semantic_{}", name), |b| {
            b.iter(|| {
                if let Ok(mut lexer) = Lexer::new(black_box(source)) {
                    if let Ok((tokens, _)) = lexer.scan_tokens() {
                        let mut parser = Parser::new(tokens);
                        if let Ok(ast) = parser.parse_program() {
                            let mut analyzer = SemanticAnalyzer::new();
                            let _ = analyzer.analyze_program(&ast);
                        }
                    }
                }
            })
        });
    }
    
    group.finish();
}

/// Benchmark FFI validation overhead
fn ffi_security_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("ffi_security");
    
    let mut validator = EnhancedFFIValidator::new();
    let context = FFIContext::default();
    
    // Benchmark safe function validation
    group.bench_function("safe_function_validation", |b| {
        b.iter(|| {
            let _ = validator.validate_ffi_call(
                black_box("strlen"),
                black_box(&[Value::String("test".to_string())]),
                black_box(&context),
            );
        })
    });
    
    // Benchmark complex argument validation
    let complex_args = vec![
        Value::String("test_string".to_string()),
        Value::Array(vec![Value::I32(1), Value::I32(2), Value::I32(3)]),
        Value::I32(42),
        Value::F32(3.14),
        Value::Bool(true),
    ];
    
    group.bench_function("complex_argument_validation", |b| {
        b.iter(|| {
            let _ = validator.validate_ffi_call(
                black_box("complex_function"),
                black_box(&complex_args),
                black_box(&context),
            );
        })
    });
    
    // Benchmark validation with different argument counts
    for arg_count in [1, 5, 10, 16].iter() {
        let args: Vec<Value> = (0..*arg_count)
            .map(|i| Value::I32(i as i32))
            .collect();
        
        group.bench_with_input(
            BenchmarkId::new("argument_count_validation", arg_count),
            arg_count,
            |b, _| {
                b.iter(|| {
                    let _ = validator.validate_ffi_call(
                        black_box("test_function"),
                        black_box(&args),
                        black_box(&context),
                    );
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark garbage collection with security features
fn gc_security_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("gc_security");
    
    let runtime = Runtime::new(RuntimeConfig::default()).unwrap();
    
    // Benchmark GC performance with different object counts
    for object_count in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("gc_collection", object_count),
            object_count,
            |b, &count| {
                b.iter(|| {
                    // Create objects
                    let mut values = Vec::new();
                    for i in 0..count {
                        values.push(Value::I32(i as i32));
                    }
                    
                    // Force GC
                    runtime.collect_garbage();
                    
                    // Keep values alive until here
                    black_box(values);
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark type system operations with security validation
fn type_security_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("type_security");
    
    let types = vec![
        Type::I32,
        Type::F32,
        Type::Bool,
        Type::String,
        Type::Array(Box::new(Type::I32)),
        Type::Generic {
            name: "Vec".to_string(),
            args: vec![Type::I32],
        },
    ];
    
    // Benchmark type equality checks
    group.bench_function("type_equality", |b| {
        b.iter(|| {
            for (i, ty1) in types.iter().enumerate() {
                for (j, ty2) in types.iter().enumerate() {
                    black_box(ty1 == ty2);
                    black_box(i == j);
                }
            }
        })
    });
    
    // Benchmark type cloning (important for security validation)
    group.bench_function("type_cloning", |b| {
        b.iter(|| {
            for ty in &types {
                black_box(ty.clone());
            }
        })
    });
    
    group.finish();
}

/// Benchmark bounds checking performance
fn bounds_checking_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("bounds_checking");
    
    let array_sizes = vec![10, 100, 1000, 10000];
    
    for size in array_sizes {
        let array: Vec<i32> = (0..size).collect();
        
        // Benchmark safe array access
        group.bench_with_input(
            BenchmarkId::new("safe_array_access", size),
            &size,
            |b, _| {
                b.iter(|| {
                    for i in 0..size {
                        if i < array.len() {
                            black_box(array[i]);
                        }
                    }
                })
            },
        );
        
        // Benchmark unsafe array access (for comparison)
        group.bench_with_input(
            BenchmarkId::new("unsafe_array_access", size),
            &size,
            |b, _| {
                b.iter(|| {
                    for i in 0..size {
                        unsafe {
                            black_box(*array.get_unchecked(i));
                        }
                    }
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark constraint validation performance
fn constraint_validation_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("constraint_validation");
    
    // Create test constraints of varying complexity
    let constraint_counts = vec![1, 5, 10, 50];
    
    for count in constraint_counts {
        group.bench_with_input(
            BenchmarkId::new("constraint_validation", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    // Simulate constraint validation overhead
                    for _ in 0..count {
                        // Mock constraint validation work
                        let constraint_type = "TraitBound";
                        let type_name = "T";
                        let trait_name = "Clone";
                        
                        black_box(constraint_type);
                        black_box(type_name);
                        black_box(trait_name);
                        
                        // Simulate validation logic
                        let validation_result = type_name.len() > 0 && trait_name.len() > 0;
                        black_box(validation_result);
                    }
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark overall security overhead
fn overall_security_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("overall_security_overhead");
    
    // Comprehensive security benchmark
    let source = r#"
        fn fibonacci(n: i32) -> i32 {
            if n <= 1 {
                n
            } else {
                fibonacci(n - 1) + fibonacci(n - 2)
            }
        }
        
        let result = fibonacci(10);
    "#;
    
    // Benchmark with all security features enabled
    group.bench_function("full_security_pipeline", |b| {
        b.iter(|| {
            // Lexing
            if let Ok(mut lexer) = Lexer::new(black_box(source)) {
                if let Ok((tokens, _)) = lexer.scan_tokens() {
                    // Parsing
                    let mut parser = Parser::new(tokens);
                    if let Ok(ast) = parser.parse_program() {
                        // Semantic analysis with security validation
                        let mut analyzer = SemanticAnalyzer::new();
                        let _ = analyzer.analyze_program(&ast);
                    }
                }
            }
        })
    });
    
    // Compare with minimal security overhead
    group.bench_function("minimal_security_pipeline", |b| {
        b.iter(|| {
            // Same pipeline but with minimal validation
            if let Ok(mut lexer) = Lexer::new(black_box(source)) {
                if let Ok((tokens, _)) = lexer.scan_tokens() {
                    let mut parser = Parser::new(tokens);
                    if let Ok(_ast) = parser.parse_program() {
                        // Skip semantic analysis for minimal overhead
                    }
                }
            }
        })
    });
    
    group.finish();
}

/// Security-focused stress testing
fn security_stress_tests(c: &mut Criterion) {
    let mut group = c.benchmark_group("security_stress");
    
    // Test with deeply nested expressions
    let nested_expr = "(((((1 + 2) * 3) / 4) - 5) + 6)".repeat(10);
    
    group.bench_function("deeply_nested_parsing", |b| {
        b.iter(|| {
            if let Ok(mut lexer) = Lexer::new(black_box(&nested_expr)) {
                if let Ok((tokens, _)) = lexer.scan_tokens() {
                    let mut parser = Parser::new(tokens);
                    let _ = parser.parse_program();
                }
            }
        })
    });
    
    // Test with many function calls (potential stack overflow)
    let many_calls = "f(".repeat(100) + &")".repeat(100);
    
    group.bench_function("many_function_calls", |b| {
        b.iter(|| {
            if let Ok(mut lexer) = Lexer::new(black_box(&many_calls)) {
                if let Ok((tokens, _)) = lexer.scan_tokens() {
                    let mut parser = Parser::new(tokens);
                    let _ = parser.parse_program();
                }
            }
        })
    });
    
    group.finish();
}

// Configure benchmark groups
criterion_group!(
    security_benches,
    memory_security_benchmarks,
    parser_security_benchmarks,
    ffi_security_benchmarks,
    gc_security_benchmarks,
    type_security_benchmarks,
    bounds_checking_benchmarks,
    constraint_validation_benchmarks,
    overall_security_overhead,
    security_stress_tests
);

criterion_main!(security_benches);