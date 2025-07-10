//! Closure performance benchmarks for Script language
//!
//! This module contains comprehensive benchmarks for measuring closure creation
//! and execution performance, memory usage, and optimization effectiveness.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use script::runtime::closure::{create_closure_heap, Closure, ClosureRuntime};
use script::runtime::gc;
use script::runtime::Value;
use std::collections::HashMap;
use std::time::Duration;

/// Benchmark closure creation with varying numbers of captured variables
fn bench_closure_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("closure_creation");

    // Initialize GC for consistent measurements
    let _ = gc::initialize();

    // Test different numbers of captured variables
    for capture_count in [0, 1, 5, 10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("heap_allocation", capture_count),
            capture_count,
            |b, &capture_count| {
                b.iter(|| {
                    let captures = create_test_captures(capture_count);
                    let closure = create_closure_heap(
                        format!("test_closure_{}", capture_count),
                        vec!["x".to_string(), "y".to_string()],
                        captures,
                        false,
                    );
                    black_box(closure);
                });
            },
        );
    }

    // Test closure creation with different parameter counts
    for param_count in [0, 1, 5, 10].iter() {
        group.bench_with_input(
            BenchmarkId::new("param_count", param_count),
            param_count,
            |b, &param_count| {
                b.iter(|| {
                    let params = (0..*param_count).map(|i| format!("param_{}", i)).collect();
                    let closure =
                        create_closure_heap("test_closure".to_string(), params, vec![], false);
                    black_box(closure);
                });
            },
        );
    }

    // Test closure creation with by-value vs by-reference captures
    group.bench_function("by_value_captures", |b| {
        b.iter(|| {
            let captures = create_test_captures(10);
            let closure = create_closure_heap(
                "by_value_test".to_string(),
                vec!["x".to_string()],
                captures,
                false, // by value
            );
            black_box(closure);
        });
    });

    group.bench_function("by_reference_captures", |b| {
        b.iter(|| {
            let captures = create_test_captures(10);
            let closure = create_closure_heap(
                "by_ref_test".to_string(),
                vec!["x".to_string()],
                captures,
                true, // by reference
            );
            black_box(closure);
        });
    });

    group.finish();
    let _ = gc::shutdown();
}

/// Benchmark closure execution performance
fn bench_closure_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("closure_execution");

    let mut runtime = ClosureRuntime::new();

    // Register a simple test closure
    runtime.register_closure("add_numbers".to_string(), |args: &[Value]| {
        match (&args[0], &args[1]) {
            (Value::I32(a), Value::I32(b)) => Ok(Value::I32(a + b)),
            _ => Ok(Value::I32(0)), // fallback
        }
    });

    // Register a closure with captures
    runtime.register_closure("multiply_by_captured".to_string(), |args: &[Value]| {
        match &args[0] {
            Value::I32(n) => Ok(Value::I32(n * 42)), // simulate captured value
            _ => Ok(Value::I32(0)),
        }
    });

    // Benchmark simple closure execution
    group.bench_function("simple_execution", |b| {
        let closure = Closure::new(
            "add_numbers".to_string(),
            vec!["a".to_string(), "b".to_string()],
            HashMap::new(),
        );
        let args = vec![Value::I32(10), Value::I32(20)];

        b.iter(|| {
            let result = runtime.execute_closure(&closure, &args);
            black_box(result);
        });
    });

    // Benchmark closure with captures
    group.bench_function("captured_execution", |b| {
        let mut captures = HashMap::new();
        captures.insert("multiplier".to_string(), Value::I32(42));

        let closure = Closure::new(
            "multiply_by_captured".to_string(),
            vec!["x".to_string()],
            captures,
        );
        let args = vec![Value::I32(5)];

        b.iter(|| {
            let result = runtime.execute_closure(&closure, &args);
            black_box(result);
        });
    });

    // Benchmark argument validation overhead
    group.bench_function("argument_validation", |b| {
        let closure = Closure::new(
            "add_numbers".to_string(),
            vec!["a".to_string(), "b".to_string()],
            HashMap::new(),
        );
        let args = vec![Value::I32(1), Value::I32(2)];

        b.iter(|| {
            // This will include validation overhead
            let param_count = closure.param_count();
            let valid = args.len() == param_count;
            black_box(valid);
        });
    });

    group.finish();
}

/// Benchmark memory usage patterns
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");

    // Initialize GC for consistent measurements
    let _ = gc::initialize();

    // Benchmark memory allocation patterns
    group.bench_function("allocation_pattern", |b| {
        b.iter(|| {
            let closures: Vec<_> = (0..100)
                .map(|i| {
                    create_closure_heap(
                        format!("closure_{}", i),
                        vec!["x".to_string()],
                        vec![("value".to_string(), Value::I32(i))],
                        false,
                    )
                })
                .collect();
            black_box(closures);
        });
    });

    // Benchmark cycle detection overhead
    group.bench_function("cycle_detection_overhead", |b| {
        b.iter(|| {
            // Create closures that capture other closures (triggers cycle detection)
            let inner = create_closure_heap(
                "inner".to_string(),
                vec!["x".to_string()],
                vec![("data".to_string(), Value::I32(42))],
                false,
            );

            let outer = create_closure_heap(
                "outer".to_string(),
                vec!["y".to_string()],
                vec![("inner_closure".to_string(), inner)],
                false,
            );

            black_box(outer);
        });
    });

    // Benchmark tracing overhead
    group.bench_function("tracing_overhead", |b| {
        let closure = create_closure_heap(
            "trace_test".to_string(),
            vec!["x".to_string()],
            create_test_captures(20),
            false,
        );

        b.iter(|| {
            if let Value::Closure(rc_closure) = &closure {
                let size = rc_closure.trace_size();
                black_box(size);
            }
        });
    });

    group.finish();
    let _ = gc::shutdown();
}

/// Benchmark string operations and ID management
fn bench_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_operations");

    // Benchmark function ID lookups
    group.bench_function("function_id_lookup", |b| {
        let mut runtime = ClosureRuntime::new();

        // Register multiple closures
        for i in 0..100 {
            runtime.register_closure(format!("closure_{}", i), |_args| Ok(Value::I32(42)));
        }

        let closure = Closure::new(
            "closure_50".to_string(),
            vec!["x".to_string()],
            HashMap::new(),
        );

        b.iter(|| {
            // This simulates the HashMap lookup in execute_closure
            let result = runtime.execute_closure(&closure, &[Value::I32(1)]);
            black_box(result);
        });
    });

    // Benchmark string cloning overhead
    group.bench_function("string_cloning", |b| {
        let base_id = "very_long_function_identifier_that_gets_cloned_frequently";
        let params = vec![
            "parameter_one".to_string(),
            "parameter_two".to_string(),
            "parameter_three".to_string(),
        ];

        b.iter(|| {
            let closure = Closure::new(base_id.to_string(), params.clone(), HashMap::new());
            black_box(closure);
        });
    });

    // Benchmark parameter name operations
    group.bench_function("parameter_operations", |b| {
        let closure = Closure::new(
            "test_closure".to_string(),
            vec![
                "param1".to_string(),
                "param2".to_string(),
                "param3".to_string(),
                "param4".to_string(),
                "param5".to_string(),
            ],
            HashMap::new(),
        );

        b.iter(|| {
            let count = closure.param_count();
            let params = closure.get_parameters();
            black_box((count, params));
        });
    });

    group.finish();
}

/// Benchmark closure cloning and copying
fn bench_closure_cloning(c: &mut Criterion) {
    let mut group = c.benchmark_group("closure_cloning");

    // Create closures with different capture sizes
    let small_closure = create_closure_heap(
        "small".to_string(),
        vec!["x".to_string()],
        create_test_captures(5),
        false,
    );

    let large_closure = create_closure_heap(
        "large".to_string(),
        vec!["x".to_string()],
        create_test_captures(50),
        false,
    );

    group.bench_function("small_closure_clone", |b| {
        b.iter(|| {
            let cloned = small_closure.clone();
            black_box(cloned);
        });
    });

    group.bench_function("large_closure_clone", |b| {
        b.iter(|| {
            let cloned = large_closure.clone();
            black_box(cloned);
        });
    });

    // Benchmark call stack operations
    group.bench_function("call_stack_operations", |b| {
        let mut runtime = ClosureRuntime::new();
        let closure = Closure::new("test".to_string(), vec!["x".to_string()], HashMap::new());

        b.iter(|| {
            // Simulate the call stack push/pop from execute_closure
            let stack_depth_before = runtime.call_stack_depth();
            runtime.call_stack.push(closure.clone()); // This is what's expensive
            let stack_depth_after = runtime.call_stack_depth();
            runtime.call_stack.pop();
            black_box((stack_depth_before, stack_depth_after));
        });
    });

    group.finish();
}

/// Helper function to create test captures
fn create_test_captures(count: usize) -> Vec<(String, Value)> {
    (0..count)
        .map(|i| {
            let name = format!("capture_{}", i);
            let value = match i % 4 {
                0 => Value::I32(i as i32),
                1 => Value::String(format!("string_{}", i)),
                2 => Value::Bool(i % 2 == 0),
                3 => Value::F64(i as f64 * 1.5),
                _ => Value::Null,
            };
            (name, value)
        })
        .collect()
}

/// Configure benchmark groups
criterion_group!(
    name = closure_benches;
    config = Criterion::default()
        .sample_size(1000)
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(2));
    targets =
        bench_closure_creation,
        bench_closure_execution,
        bench_memory_usage,
        bench_string_operations,
        bench_closure_cloning
);

criterion_main!(closure_benches);
