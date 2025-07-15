use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use script::{
    codegen::MonomorphizationContext,
    inference::{
        apply_optimized_substitution, apply_substitution, InferenceContext,
        OptimizedInferenceContext, OptimizedSubstitution, Substitution, UnionFind,
    },
    source::{SourceLocation, Span},
    types::Type,
};
use std::collections::HashMap;

/// Generate test types of varying complexity
fn generate_test_types(count: usize) -> Vec<Type> {
    let mut types = Vec::with_capacity(count);

    for i in 0..count {
        let type_var = Type::TypeVar(i as u32);
        let complex_type = match i % 5 {
            0 => Type::Array(Box::new(type_var)),
            1 => Type::Function {
                params: vec![type_var.clone(), Type::I32],
                ret: Box::new(type_var),
            },
            2 => Type::Generic {
                name: "Vec".to_string(),
                args: vec![type_var],
            },
            3 => Type::Tuple(vec![type_var.clone(), Type::String, type_var]),
            4 => Type::Option(Box::new(Type::Result {
                ok: Box::new(type_var),
                err: Box::new(Type::String),
            })),
            _ => unreachable!(),
        };
        types.push(complex_type);
    }

    types
}

/// Benchmark unification algorithms
fn bench_unification(c: &mut Criterion) {
    let mut group = c.benchmark_group("unification");

    for size in [10, 50, 100, 200, 500].iter() {
        let types = generate_test_types(*size);
        let span = Span::new(SourceLocation::new(1, 1, 0), SourceLocation::new(1, 10, 10));

        // Benchmark original unification
        group.bench_with_input(
            BenchmarkId::new("original_unification", size),
            size,
            |b, &_size| {
                b.iter(|| {
                    let mut ctx = InferenceContext::new();
                    for i in 0..types.len() - 1 {
                        let constraint = script::inference::Constraint::equality(
                            types[i].clone(),
                            types[i + 1].clone(),
                            span,
                        );
                        ctx.add_constraint(constraint);
                    }
                    black_box(ctx.solve_constraints())
                });
            },
        );

        // Benchmark union-find unification
        group.bench_with_input(
            BenchmarkId::new("union_find_unification", size),
            size,
            |b, &_size| {
                b.iter(|| {
                    let mut union_find = UnionFind::new();
                    for i in 0..types.len() - 1 {
                        black_box(union_find.unify_types(&types[i], &types[i + 1]));
                    }
                });
            },
        );

        // Benchmark optimized inference context
        group.bench_with_input(
            BenchmarkId::new("optimized_inference_context", size),
            size,
            |b, &_size| {
                b.iter(|| {
                    let mut ctx = OptimizedInferenceContext::new();
                    for i in 0..types.len() - 1 {
                        let constraint = script::inference::Constraint::equality(
                            types[i].clone(),
                            types[i + 1].clone(),
                            span,
                        );
                        ctx.add_constraint(constraint);
                    }
                    black_box(ctx.solve_constraints())
                });
            },
        );
    }

    group.finish();
}

/// Benchmark substitution algorithms
fn bench_substitution(c: &mut Criterion) {
    let mut group = c.benchmark_group("substitution");

    for size in [10, 50, 100, 200, 500].iter() {
        let types = generate_test_types(*size);

        // Create substitutions
        let mut original_subst = Substitution::new();
        let mut optimized_subst = OptimizedSubstitution::new();

        for i in 0..*size {
            let concrete_type = match i % 4 {
                0 => Type::I32,
                1 => Type::String,
                2 => Type::Bool,
                3 => Type::F32,
                _ => unreachable!(),
            };
            original_subst.insert(i as u32, concrete_type.clone());
            optimized_subst.insert(i as u32, concrete_type);
        }

        // Benchmark original substitution
        group.bench_with_input(
            BenchmarkId::new("original_substitution", size),
            size,
            |b, &_size| {
                b.iter(|| {
                    for ty in &types {
                        black_box(apply_substitution(&original_subst, ty));
                    }
                });
            },
        );

        // Benchmark optimized substitution
        group.bench_with_input(
            BenchmarkId::new("optimized_substitution", size),
            size,
            |b, &_size| {
                b.iter(|| {
                    let mut opt_subst = optimized_subst.clone();
                    for ty in &types {
                        black_box(apply_optimized_substitution(&mut opt_subst, ty));
                    }
                });
            },
        );

        // Benchmark batch substitution
        group.bench_with_input(
            BenchmarkId::new("batch_substitution", size),
            size,
            |b, &_size| {
                b.iter(|| {
                    let mut opt_subst = optimized_subst.clone();
                    black_box(opt_subst.apply_batch(&types));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark monomorphization algorithms
fn bench_monomorphization(c: &mut Criterion) {
    let mut group = c.benchmark_group("monomorphization");

    for size in [5, 10, 20, 50].iter() {
        // Create mock generic instantiations
        let mut instantiations = Vec::new();
        let span = Span::new(SourceLocation::new(1, 1, 0), SourceLocation::new(1, 10, 10));

        for i in 0..*size {
            let instantiation = script::semantic::analyzer::GenericInstantiation {
                function_name: format!("func_{}", i),
                type_args: vec![Type::I32, Type::String],
                span,
            };
            instantiations.push(instantiation);
        }

        // Benchmark original monomorphization
        group.bench_with_input(
            BenchmarkId::new("original_monomorphization", size),
            size,
            |b, &_size| {
                b.iter(|| {
                    let mut ctx = MonomorphizationContext::new();
                    ctx.initialize_from_semantic_analysis(&instantiations, &HashMap::new());
                    black_box(ctx);
                });
            },
        );

        // Benchmark optimized monomorphization
        group.bench_with_input(
            BenchmarkId::new("optimized_monomorphization", size),
            size,
            |b, &_size| {
                b.iter(|| {
                    let mut ctx = MonomorphizationContext::new();
                    let result =
                        ctx.initialize_from_semantic_analysis(&instantiations, &HashMap::new());
                    black_box(result);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark type variable creation and resolution
fn bench_type_variables(c: &mut Criterion) {
    let mut group = c.benchmark_group("type_variables");

    for size in [100, 500, 1000, 2000].iter() {
        // Benchmark original type variable creation
        group.bench_with_input(
            BenchmarkId::new("original_type_vars", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut ctx = InferenceContext::new();
                    for _ in 0..size {
                        black_box(ctx.fresh_type_var());
                    }
                });
            },
        );

        // Benchmark union-find type variable creation
        group.bench_with_input(
            BenchmarkId::new("union_find_type_vars", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut union_find = UnionFind::new();
                    for _ in 0..size {
                        black_box(union_find.fresh_type_var());
                    }
                });
            },
        );

        // Benchmark optimized inference context type variables
        group.bench_with_input(
            BenchmarkId::new("optimized_type_vars", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut ctx = OptimizedInferenceContext::new();
                    for _ in 0..size {
                        black_box(ctx.fresh_type_var());
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark cache effectiveness
fn bench_cache_effectiveness(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_effectiveness");

    let complex_type = Type::Function {
        params: vec![
            Type::Array(Box::new(Type::TypeVar(0))),
            Type::Generic {
                name: "Vec".to_string(),
                args: vec![Type::TypeVar(1)],
            },
            Type::Tuple(vec![Type::TypeVar(2), Type::TypeVar(3)]),
        ],
        ret: Box::new(Type::Result {
            ok: Box::new(Type::TypeVar(4)),
            err: Box::new(Type::String),
        }),
    };

    // Test repeated substitution of the same complex type
    for iterations in [10, 50, 100, 200].iter() {
        group.bench_with_input(
            BenchmarkId::new("without_cache", iterations),
            iterations,
            |b, &iterations| {
                b.iter(|| {
                    let mut subst = Substitution::new();
                    subst.insert(0, Type::I32);
                    subst.insert(1, Type::String);
                    subst.insert(2, Type::Bool);
                    subst.insert(3, Type::F32);
                    subst.insert(4, Type::I32);

                    for _ in 0..iterations {
                        black_box(apply_substitution(&subst, &complex_type));
                    }
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("with_cache", iterations),
            iterations,
            |b, &iterations| {
                b.iter(|| {
                    let mut subst = OptimizedSubstitution::new();
                    subst.insert(0, Type::I32);
                    subst.insert(1, Type::String);
                    subst.insert(2, Type::Bool);
                    subst.insert(3, Type::F32);
                    subst.insert(4, Type::I32);

                    for _ in 0..iterations {
                        black_box(subst.apply_to_type(&complex_type));
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark occurs check optimization
// Commented out as occurs_check functions are not publicly exposed
// fn bench_occurs_check(c: &mut Criterion) {
//     let mut group = c.benchmark_group("occurs_check");
//
//     // Create deeply nested type for occurs check
//     let mut nested_type = Type::TypeVar(999); // The variable we'll check for
//     for i in 0..10 {
//         nested_type = Type::Array(Box::new(Type::Function {
//             params: vec![nested_type, Type::TypeVar(i)],
//             ret: Box::new(Type::Option(Box::new(Type::TypeVar(i + 100)))),
//         }));
//     }
//
//     group.bench_function("original_occurs_check", |b| {
//         b.iter(|| {
//             // occurs_check is not publicly exposed
//             // black_box(script::inference::substitution::occurs_check(
//             //     999,
//             //     &nested_type,
//             // ));
//         });
//     });
//
//     // group.bench_function("optimized_occurs_check", |b| {
//     //     b.iter(|| {
//     //         black_box(script::inference::optimized_occurs_check(999, &nested_type));
//     //     });
//     // });
//
//     group.finish();
// }

/// Benchmark memory usage patterns
fn bench_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_patterns");

    for size in [100, 500, 1000].iter() {
        let types = generate_test_types(*size);

        // Benchmark memory allocation patterns
        group.bench_with_input(BenchmarkId::new("clone_heavy", size), size, |b, &_size| {
            b.iter(|| {
                let mut subst = Substitution::new();
                for (i, ty) in types.iter().enumerate() {
                    subst.insert(i as u32, ty.clone()); // Heavy cloning
                }
                black_box(subst);
            });
        });

        group.bench_with_input(
            BenchmarkId::new("optimized_allocation", size),
            size,
            |b, &_size| {
                b.iter(|| {
                    let mut subst = OptimizedSubstitution::new();
                    for (i, ty) in types.iter().enumerate() {
                        subst.insert(i as u32, ty.clone());
                    }
                    subst.optimize(); // Remove redundant mappings
                    black_box(subst);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_unification,
    bench_substitution,
    bench_monomorphization,
    bench_type_variables,
    bench_cache_effectiveness,
    // bench_occurs_check, // Commented out as occurs_check is not publicly exposed
    bench_memory_patterns
);

criterion_main!(benches);
