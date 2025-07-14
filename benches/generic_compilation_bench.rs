//! Benchmarks for generic type compilation performance
//!
//! These benchmarks measure parsing, type checking, and full compilation
//! performance for programs with varying amounts of generic code.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use script::{Lexer, Parser, SemanticAnalyzer};

/// Generate a program with N generic struct definitions
fn generate_generic_structs(count: usize) -> String {
    let mut code = String::new();

    for i in 0..count {
        code.push_str(format!(
            "struct Generic{}<T> {{\n    value: T,\n    id: i32\n}}\n\n",
            i
        ));
    }

    // Add usage in main
    code.push_str("fn main() {\n");
    for i in 0..count.min(10) {
        code.push_str(format!(
            "    let g{} = Generic{} {{ value: {}, id: {} }};\n",
            i, i, i, i
        ));
    }
    code.push_str("}\n");

    code
}

/// Generate a program with nested generic types
fn generate_nested_generics(depth: usize) -> String {
    let mut code = String::new();

    // Define basic generic types
    code.push_str("struct Box<T> { value: T }\n");
    code.push_str("enum Option<T> { Some(T), None }\n");
    code.push_str("enum Result<T, E> { Ok(T), Err(E) }\n\n");

    code.push_str("fn main() {\n");

    // Create nested types of increasing depth
    for d in 1..=depth {
        let mut type_str = "i32".to_string();
        for _ in 0..d {
            type_str = format!("Box<{}>", type_str);
        }
        code.push_str(&format!("    let nested{} = {};\n", d, type_str));
    }

    code.push_str("}\n");

    code
}

/// Generate a program with generic functions
fn generate_generic_functions(count: usize) -> String {
    let mut code = String::new();

    for i in 0..count {
        code.push_str(format!("fn generic{}<T>(x: T) -> T {{ x }}\n", i));
    }

    code.push_str("\nfn main() {\n");
    for i in 0..count.min(10) {
        code.push_str(format!("    let r{} = generic{i}({i * 10});\n", i));
    }
    code.push_str("}\n");

    code
}

fn bench_generic_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("generic_compilation/parsing");

    for size in [10, 50, 100, 200].iter() {
        let code = generate_generic_structs(*size);

        group.bench_with_input(BenchmarkId::new("structs", size), &code, |b, code| {
            b.iter(|| {
                let lexer = Lexer::new(black_box(code));
                let (tokens, _) = lexer.scan_tokens();
                let mut parser = Parser::new(tokens);
                let _ = parser.parse();
            })
        });
    }

    for depth in [5, 10, 15].iter() {
        let code = generate_nested_generics(*depth);

        group.bench_with_input(BenchmarkId::new("nested", depth), &code, |b, code| {
            b.iter(|| {
                let lexer = Lexer::new(black_box(code));
                let (tokens, _) = lexer.scan_tokens();
                let mut parser = Parser::new(tokens);
                let _ = parser.parse();
            })
        });
    }

    group.finish();
}

fn bench_type_checking_generics(c: &mut Criterion) {
    let mut group = c.benchmark_group("generic_compilation/type_checking");

    // Pre-parse programs for type checking benchmarks
    let small_program = {
        let code = generate_generic_structs(10);
        let lexer = Lexer::new(&code);
        let (tokens, _) = lexer.scan_tokens();
        let mut parser = Parser::new(tokens);
        parser.parse().unwrap()
    };

    let medium_program = {
        let code = generate_generic_structs(50);
        let lexer = Lexer::new(&code);
        let (tokens, _) = lexer.scan_tokens();
        let mut parser = Parser::new(tokens);
        parser.parse().unwrap()
    };

    let large_program = {
        let code = generate_generic_structs(100);
        let lexer = Lexer::new(&code);
        let (tokens, _) = lexer.scan_tokens();
        let mut parser = Parser::new(tokens);
        parser.parse().unwrap()
    };

    group.bench_function("small", |b| {
        b.iter(|| {
            let mut analyzer = SemanticAnalyzer::new();
            let _ = analyzer.analyze_program(black_box(&small_program));
        })
    });

    group.bench_function("medium", |b| {
        b.iter(|| {
            let mut analyzer = SemanticAnalyzer::new();
            let _ = analyzer.analyze_program(black_box(&medium_program));
        })
    });

    group.bench_function("large", |b| {
        b.iter(|| {
            let mut analyzer = SemanticAnalyzer::new();
            let _ = analyzer.analyze_program(black_box(&large_program));
        })
    });

    group.finish();
}

fn bench_end_to_end_compilation(c: &mut Criterion) {
    let mut group = c.benchmark_group("generic_compilation/end_to_end");

    // Benchmark different types of generic programs
    let struct_heavy = generate_generic_structs(50);
    let function_heavy = generate_generic_functions(50);
    let nested_heavy = generate_nested_generics(10);

    group.bench_function("struct_heavy", |b| {
        b.iter(|| {
            let lexer = Lexer::new(black_box(&struct_heavy));
            let (tokens, _) = lexer.scan_tokens();
            let mut parser = Parser::new(tokens);
            let program = parser.parse().unwrap();
            let mut analyzer = SemanticAnalyzer::new();
            let _ = analyzer.analyze_program(&program);
        })
    });

    group.bench_function("function_heavy", |b| {
        b.iter(|| {
            let lexer = Lexer::new(black_box(&function_heavy));
            let (tokens, _) = lexer.scan_tokens();
            let mut parser = Parser::new(tokens);
            let program = parser.parse().unwrap();
            let mut analyzer = SemanticAnalyzer::new();
            let _ = analyzer.analyze_program(&program);
        })
    });

    group.bench_function("nested_heavy", |b| {
        b.iter(|| {
            let lexer = Lexer::new(black_box(&nested_heavy));
            let (tokens, _) = lexer.scan_tokens();
            let mut parser = Parser::new(tokens);
            let program = parser.parse().unwrap();
            let mut analyzer = SemanticAnalyzer::new();
            let _ = analyzer.analyze_program(&program);
        })
    });

    group.finish();
}

fn bench_incremental_generic_compilation(c: &mut Criterion) {
    let mut group = c.benchmark_group("generic_compilation/incremental");

    // Simulate adding one more generic type to an existing program
    let base_code = generate_generic_structs(50);
    let addition = "\nstruct NewGeneric<T> { value: T }\n";
    let modified_code = base_code.clone() + addition;

    group.bench_function("recompile_with_addition", |b| {
        b.iter(|| {
            let lexer = Lexer::new(black_box(&modified_code));
            let (tokens, _) = lexer.scan_tokens();
            let mut parser = Parser::new(tokens);
            let program = parser.parse().unwrap();
            let mut analyzer = SemanticAnalyzer::new();
            let _ = analyzer.analyze_program(&program);
        })
    });

    // Simulate changing a generic type
    let changed_code = base_code.replace("Generic0<T>", "Generic0<T, U>");

    group.bench_function("recompile_with_change", |b| {
        b.iter(|| {
            let lexer = Lexer::new(black_box(&changed_code));
            let (tokens, _) = lexer.scan_tokens();
            let mut parser = Parser::new(tokens);
            let _ = parser.parse(); // Might fail due to change
        })
    });

    group.finish();
}

fn bench_generic_instantiation_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("generic_compilation/instantiation_scaling");

    // Test how compilation scales with number of instantiations
    for count in [1, 5, 10, 20, 50].iter() {
        let mut code = String::new();
        code.push_str("struct Box<T> { value: T }\n");
        code.push_str("fn main() {\n");

        // Create N different instantiations
        for i in 0..*count {
            match i % 4 {
                0 => code.push_str(format!("    let b{} = Box {{ value: {i} }};\n", i)),
                1 => code.push_str(format!("    let b{} = Box {{ value: \"str{}\" }};\n", i, i)),
                2 => code.push_str(format!("    let b{} = Box {{ value: true }};\n", i)),
                _ => code.push_str(format!("    let b{} = Box {{ value: {i}.0 }};\n", i)),
            }
        }

        code.push_str("}\n");

        group.bench_with_input(BenchmarkId::from_parameter(count), &code, |b, code| {
            b.iter(|| {
                let lexer = Lexer::new(black_box(code));
                let (tokens, _) = lexer.scan_tokens();
                let mut parser = Parser::new(tokens);
                let program = parser.parse().unwrap();
                let mut analyzer = SemanticAnalyzer::new();
                let _ = analyzer.analyze_program(&program);
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_generic_parsing,
    bench_type_checking_generics,
    bench_end_to_end_compilation,
    bench_incremental_generic_compilation,
    bench_generic_instantiation_count
);
criterion_main!(benches);
