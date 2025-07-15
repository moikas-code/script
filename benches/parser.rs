use criterion::{black_box, criterion_group, criterion_main, Criterion};
use script::{Lexer, Parser};

fn benchmark_parse_expression(c: &mut Criterion) {
    let source = "1 + 2 * 3 - 4 / 5 + (6 * 7) - 8";

    c.bench_function("parser_expression", |b| {
        b.iter(|| {
            let lexer = Lexer::new(black_box(source)).expect("Failed to create lexer");
            let (tokens, _) = lexer.scan_tokens();
            let mut parser = Parser::new(tokens);
            parser.parse_expression()
        })
    });
}

fn benchmark_parse_program(c: &mut Criterion) {
    let source = r#"
        fn fibonacci(n: i32) -> i32 {
            if n <= 1 {
                return n
            }
            return fibonacci(n - 1) + fibonacci(n - 2)
        }
        
        let result = fibonacci(10)
        print("Result: " + result)
        
        fn calculate(x: f32, y: f32) -> f32 {
            let temp = x * y
            let result = temp / 2.0
            return result + 42.0
        }
        
        let values = [1, 2, 3, 4, 5]
        let sum = 0
        
        for val in values {
            sum = sum + val
        }
    "#;

    c.bench_function("parser_program", |b| {
        b.iter(|| {
            let lexer = Lexer::new(black_box(source)).expect("Failed to create lexer");
            let (tokens, _) = lexer.scan_tokens();
            let mut parser = Parser::new(tokens);
            parser.parse()
        })
    });
}

fn benchmark_parse_deeply_nested(c: &mut Criterion) {
    // Generate deeply nested expressions
    let mut source = String::new();
    for i in 0..20 {
        source.push_str("if true { ");
    }
    source.push_str("42");
    for _ in 0..20 {
        source.push_str(" } else { 0 }");
    }

    c.bench_function("parser_deeply_nested", |b| {
        b.iter(|| {
            let lexer = Lexer::new(black_box(&source)).expect("Failed to create lexer");
            let (tokens, _) = lexer.scan_tokens();
            let mut parser = Parser::new(tokens);
            parser.parse_expression()
        })
    });
}

fn benchmark_parse_many_statements(c: &mut Criterion) {
    let mut source = String::new();
    for i in 0..100 {
        source.push_str(&format!("let var{} = {} + {} * {}\n", i, i, i + 1, i + 2));
    }

    c.bench_function("parser_many_statements", |b| {
        b.iter(|| {
            let lexer = Lexer::new(black_box(&source)).expect("Failed to create lexer");
            let (tokens, _) = lexer.scan_tokens();
            let mut parser = Parser::new(tokens);
            parser.parse()
        })
    });
}

criterion_group!(
    benches,
    benchmark_parse_expression,
    benchmark_parse_program,
    benchmark_parse_deeply_nested,
    benchmark_parse_many_statements
);
criterion_main!(benches);
