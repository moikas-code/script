//! Performance benchmarks for generic type monomorphization
//!
//! These benchmarks measure the performance characteristics of
//! monomorphizing generic types with various levels of complexity.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use script::{Lexer, Parser, SemanticAnalyzer};

/// Generate a program with generic functions and their instantiations
fn generate_generic_program(func_count: usize, instantiation_count: usize) -> String {
    let mut code = String::new();

    // Define generic functions
    for i in 0..func_count {
        code.push_str(format!("fn generic{}<T>(x: T) -> T {{ x }}\n", i));
    }

    code.push_str("\nfn main() {\n");

    // Create instantiations
    for i in 0..instantiation_count {
        let func_idx = i % func_count;
        let type_choice = i % 3;
        let value = match type_choice {
            0 => format!("{}", i),
            1 => format!(r#""str{}""#, i),
            _ => "true".to_string(),
        };
        code.push_str(format!(
            "    let result{} = generic{}({});\n",
            i, func_idx, value
        ));
    }

    code.push_str("}\n");
    code
}

/// Generate a program with deeply nested generic types
fn generate_nested_generics_program(depth: usize) -> String {
    let mut code = String::new();

    // Define generic types
    code.push_str("struct Box<T> { value: T }\n");
    code.push_str("enum Option<T> { Some(T), None }\n");
    code.push_str("enum Result<T, E> { Ok(T), Err(E) }\n\n");

    code.push_str("fn main() {\n");

    // Create nested type instantiations
    for d in 1..=depth {
        let mut type_expr = "42".to_string();
        for _ in 0..d {
            type_expr = format!("Box {} }}", { value: {type_expr);
        }
        code.push_str(format!("    let nested{} = {type_expr};\n", d));
    }

    code.push_str("}\n");
    code
}

/// Generate a program with multiple generic struct instantiations
fn generate_struct_instantiations(count: usize) -> String {
    let mut code = String::new();

    // Define generic structs
    code.push_str("struct Pair<A, B> { first: A, second: B }\n");
    code.push_str("struct Triple<A, B, C> { first: A, second: B, third: C }\n\n");

    code.push_str("fn main() {\n");

    // Create various instantiations
    for i in 0..count {
        match i % 4 {
            0 => code.push_str(format!(
                "    let p{} = Pair {{ first: {}, second: \"{}\" }};\n",
                i, i, i
            )),
            1 => code.push_str(format!(
                "    let p{} = Pair {{ first: true, second: {} }};\n",
                i, i as f32
            )),
            2 => code.push_str(format!(
                "    let t{} = Triple {{ first: {}, second: \"x\", third: false }};\n",
                i, i
            )),
            _ => code.push_str(format!(
                "    let t{} = Triple {{ first: \"a\", second: {}, third: {} }};\n",
                i, i, i as f32
            )),
        }
    }

    code.push_str("}\n");
    code
}

fn bench_simple_monomorphization(c: &mut Criterion) {
    let mut group = c.benchmark_group("monomorphization/simple");

    for count in [10, 100, 1000].iter() {
        let code = generate_generic_program(10, *count);

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

fn bench_nested_generics(c: &mut Criterion) {
    let mut group = c.benchmark_group("monomorphization/nested");

    for depth in [5, 10, 15].iter() {
        let code = generate_nested_generics_program(*depth);

        group.bench_with_input(BenchmarkId::new("depth", depth), &code, |b, code| {
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

fn bench_struct_instantiations(c: &mut Criterion) {
    let mut group = c.benchmark_group("monomorphization/structs");

    for count in [50, 100, 200].iter() {
        let code = generate_struct_instantiations(*count);

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

fn bench_mixed_generics(c: &mut Criterion) {
    let mut group = c.benchmark_group("monomorphization/mixed");

    let small_program = r#"
        struct Box<T> { value: T }
        enum Option<T> { Some(T), None }
        enum Result<T, E> { Ok(T), Err(E) }
        
        fn identity<T>(x: T) -> T { x }
        fn map<T, U>(opt: Option<T>, f: fn(T) -> U) -> Option<U> {
            match opt {
                Option::Some(val) => Option::Some(f(val)),
                Option::None => Option::None
            }
        }
        
        fn main() {
            let b1 = Box { value: 42 };
            let b2 = Box { value: "hello" };
            let b3 = Box { value: Box { value: true } };
            
            let opt1 = Option::Some(100);
            let opt2 = Option::Some("world");
            let opt3: Option<i32> = Option::None;
            
            let res1: Result<i32, string> = Result::Ok(200);
            let res2: Result<bool, string> = Result::Err("error");
            
            let id1 = identity(42);
            let id2 = identity("test");
            let id3 = identity(Box { value: 3.14 });
        }
    "#;

    group.bench_function("small", |b| {
        b.iter(|| {
            let lexer = Lexer::new(black_box(small_program));
            let (tokens, _) = lexer.scan_tokens();
            let mut parser = Parser::new(tokens);
            let program = parser.parse().unwrap();
            let mut analyzer = SemanticAnalyzer::new();
            let _ = analyzer.analyze_program(&program);
        })
    });

    // Generate a larger mixed program
    let mut large_program = String::new();
    large_program.push_str("struct Box<T> { value: T }\n");
    large_program.push_str("enum Option<T> { Some(T), None }\n");
    large_program.push_str("struct Pair<A, B> { first: A, second: B }\n\n");

    // Add generic functions
    for i in 0..20 {
        large_program.push_str(format!("fn process{}<T>(x: T) -> T {{ x }}\n", i));
    }

    large_program.push_str("\nfn main() {\n");

    // Add varied instantiations
    for i in 0..100 {
        match i % 5 {
            0 => large_program.push_str(format!("    let v{} = Box {{ value: {i} }};\n", i)),
            1 => large_program.push_str(format!("    let v{} = Option::Some({i});\n", i)),
            2 => large_program.push_str(format!(
                "    let v{} = Pair {{ first: {}, second: \"{}\" }};\n",
                i, i, i
            )),
            3 => large_program.push_str(format!("    let v{} = process{i % 20}({i});\n", i)),
            _ => large_program.push_str(format!(
                "    let v{} = Box {{ value: Option::Some({}) }};\n",
                i, i
            )),
        }
    }

    large_program.push_str("}\n");

    group.bench_function("large", |b| {
        b.iter(|| {
            let lexer = Lexer::new(black_box(&large_program));
            let (tokens, _) = lexer.scan_tokens();
            let mut parser = Parser::new(tokens);
            let program = parser.parse().unwrap();
            let mut analyzer = SemanticAnalyzer::new();
            let _ = analyzer.analyze_program(&program);
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_simple_monomorphization,
    bench_nested_generics,
    bench_struct_instantiations,
    bench_mixed_generics
);
criterion_main!(benches);
