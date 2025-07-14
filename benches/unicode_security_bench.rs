use criterion::{black_box, criterion_group, criterion_main, Criterion};
use script::lexer::{Lexer, UnicodeSecurityConfig, UnicodeSecurityLevel};

fn lexer_ascii_benchmark(c: &mut Criterion) {
    let input = "let ascii_identifier = 42; fn another_function() { return true; }";

    c.bench_function("lexer_ascii_only", |b| {
        b.iter(|| {
            let lexer = Lexer::new(black_box(input)).unwrap();
            let (tokens, _errors) = lexer.scan_tokens();
            black_box(tokens)
        })
    });
}

fn lexer_unicode_benchmark(c: &mut Criterion) {
    let input = "let café = 42; fn naïve_function() { return true; }";

    c.bench_function("lexer_unicode_normalization", |b| {
        b.iter(|| {
            let config = UnicodeSecurityConfig {
                level: UnicodeSecurityLevel::Permissive,
                normalize_identifiers: true,
                detect_confusables: false,
            };
            let lexer = Lexer::with_unicode_config(black_box(input), config).unwrap();
            let (tokens, _errors) = lexer.scan_tokens();
            black_box(tokens)
        })
    });
}

fn lexer_confusable_detection_benchmark(c: &mut Criterion) {
    let input = "let α = 42; let а = 43; let a = 44;"; // Greek, Cyrillic, Latin 'a'

    c.bench_function("lexer_confusable_detection", |b| {
        b.iter(|| {
            let config = UnicodeSecurityConfig {
                level: UnicodeSecurityLevel::Warning,
                normalize_identifiers: true,
                detect_confusables: true,
            };
            let lexer = Lexer::with_unicode_config(black_box(input), config).unwrap();
            let (tokens, _errors) = lexer.scan_tokens();
            black_box(tokens)
        })
    });
}

fn lexer_mixed_content_benchmark(c: &mut Criterion) {
    let input = r#"
        // ASCII function
        fn calculate_sum(a, b) {
            return a + b;
        }
        
        // Unicode identifiers
        let π = 3.14159;
        let café = "coffee";
        let résumé = "CV";
        
        // Mixed content with confusables
        let α = 1;  // Greek alpha
        let а = 2;  // Cyrillic a
        let a = 3;  // Latin a
    "#;

    c.bench_function("lexer_mixed_content", |b| {
        b.iter(|| {
            let config = UnicodeSecurityConfig {
                level: UnicodeSecurityLevel::Warning,
                normalize_identifiers: true,
                detect_confusables: true,
            };
            let lexer = Lexer::with_unicode_config(black_box(input), config).unwrap();
            let (tokens, _errors) = lexer.scan_tokens();
            black_box(tokens)
        })
    });
}

fn lexer_security_levels_benchmark(c: &mut Criterion) {
    let input = "let α = 42; let а = 43; let a = 44;"; // Greek, Cyrillic, Latin 'a'

    let mut group = c.benchmark_group("lexer_security_levels");

    group.bench_function("strict", |b| {
        b.iter(|| {
            let config = UnicodeSecurityConfig {
                level: UnicodeSecurityLevel::Strict,
                normalize_identifiers: true,
                detect_confusables: true,
            };
            let lexer = Lexer::with_unicode_config(black_box(input), config).unwrap();
            let (tokens, _errors) = lexer.scan_tokens();
            black_box(tokens)
        })
    });

    group.bench_function("warning", |b| {
        b.iter(|| {
            let config = UnicodeSecurityConfig {
                level: UnicodeSecurityLevel::Warning,
                normalize_identifiers: true,
                detect_confusables: true,
            };
            let lexer = Lexer::with_unicode_config(black_box(input), config).unwrap();
            let (tokens, _errors) = lexer.scan_tokens();
            black_box(tokens)
        })
    });

    group.bench_function("permissive", |b| {
        b.iter(|| {
            let config = UnicodeSecurityConfig {
                level: UnicodeSecurityLevel::Permissive,
                normalize_identifiers: true,
                detect_confusables: true,
            };
            let lexer = Lexer::with_unicode_config(black_box(input), config).unwrap();
            let (tokens, _errors) = lexer.scan_tokens();
            black_box(tokens)
        })
    });

    group.finish();
}

fn lexer_caching_benchmark(c: &mut Criterion) {
    // Test with repeated identifiers to verify caching effectiveness
    let input = "let café = 1; let café = 2; let café = 3; let café = 4; let café = 5;";

    c.bench_function("lexer_unicode_caching", |b| {
        b.iter(|| {
            let config = UnicodeSecurityConfig {
                level: UnicodeSecurityLevel::Permissive,
                normalize_identifiers: true,
                detect_confusables: true,
            };
            let lexer = Lexer::with_unicode_config(black_box(input), config).unwrap();
            let (tokens, _errors) = lexer.scan_tokens();
            black_box(tokens)
        })
    });
}

fn lexer_large_file_benchmark(c: &mut Criterion) {
    // Simulate a larger file with mixed ASCII and Unicode content
    let mut large_input = String::new();
    for i in 0..100 {
        large_input.push_str(&format!(
            "let variable_{} = {}; let café_{} = {}; let π_{} = 3.14; ",
            i, i, i, i, i
        ));
    }

    c.bench_function("lexer_large_unicode_file", |b| {
        b.iter(|| {
            let config = UnicodeSecurityConfig {
                level: UnicodeSecurityLevel::Warning,
                normalize_identifiers: true,
                detect_confusables: true,
            };
            let lexer = Lexer::with_unicode_config(black_box(&large_input), config).unwrap();
            let (tokens, _errors) = lexer.scan_tokens();
            black_box(tokens)
        })
    });
}

criterion_group!(
    unicode_benches,
    lexer_ascii_benchmark,
    lexer_unicode_benchmark,
    lexer_confusable_detection_benchmark,
    lexer_mixed_content_benchmark,
    lexer_security_levels_benchmark,
    lexer_caching_benchmark,
    lexer_large_file_benchmark
);

criterion_main!(unicode_benches);
