use criterion::{black_box, criterion_group, criterion_main, Criterion};
use script::Lexer;

fn benchmark_small_program(c: &mut Criterion) {
    let source = r#"
        fn fibonacci(n: i32) -> i32 {
            if n <= 1 {
                return n
            }
            return fibonacci(n - 1) + fibonacci(n - 2)
        }
        
        let result = fibonacci(10)
        print("Result: " + result)
    "#;

    c.bench_function("lexer_small_program", |b| {
        b.iter(|| {
            let lexer = Lexer::new(black_box(source)).expect("Failed to create lexer");
            let (tokens, _) = lexer.scan_tokens();
            tokens
        })
    });
}

fn benchmark_large_program(c: &mut Criterion) {
    // Generate a large program with many tokens
    let mut source = String::new();
    for i in 0..100 {
        source.push_str(format!("let var{} = {i} + {i + 1} * {i + 2}\n", i));
        source.push_str(format!(
            "fn func{}(x: i32, y: i32) -> i32 {{ return x + y }}\n",
            i
        ));
    }

    c.bench_function("lexer_large_program", |b| {
        b.iter(|| {
            let lexer = Lexer::new(black_box(&source)).expect("Failed to create lexer");
            let (tokens, _) = lexer.scan_tokens();
            tokens
        })
    });
}

fn benchmark_string_heavy(c: &mut Criterion) {
    let mut source = String::new();
    for i in 0..50 {
        source.push_str(format!(
            r#"let str{} = "This is a string with some content and escape sequences \n\t\r""#,
            i
        ));
        source.push_str("\n");
    }

    c.bench_function("lexer_string_heavy", |b| {
        b.iter(|| {
            let lexer = Lexer::new(black_box(&source)).expect("Failed to create lexer");
            let (tokens, _) = lexer.scan_tokens();
            tokens
        })
    });
}

criterion_group!(
    benches,
    benchmark_small_program,
    benchmark_large_program,
    benchmark_string_heavy
);
criterion_main!(benches);
