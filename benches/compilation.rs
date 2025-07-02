use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use script::{
    AstLowerer, CodeGenerator, InferenceEngine, Lexer, Parser, Runtime, RuntimeConfig,
    SemanticAnalyzer, SymbolTable,
};
use std::collections::HashMap;
use std::fs;

/// Helper functions to create proper API calls with required parameters
mod helpers {
    use super::*;
    use script::{SymbolTable, Type};
    use std::collections::HashMap;

    /// Create a new AstLowerer with required parameters
    pub fn create_ast_lowerer() -> AstLowerer {
        let symbol_table = SymbolTable::new();
        let type_info: HashMap<usize, Type> = HashMap::new();
        AstLowerer::new(symbol_table, type_info)
    }

    /// Simplified compilation pipeline that handles API properly
    pub fn compile_program(
        source: &str,
    ) -> Result<script::ExecutableModule, Box<dyn std::error::Error>> {
        // Lexing
        let lexer = Lexer::new(source);
        let (tokens, lex_errors) = lexer.scan_tokens();
        if !lex_errors.is_empty() {
            return Err("Lexer errors".into());
        }

        // Parsing
        let mut parser = Parser::new(tokens);
        let program = parser
            .parse()
            .map_err(|e| format!("Parser error: {:?}", e))?;

        // Semantic analysis
        let mut analyzer = SemanticAnalyzer::new();
        analyzer
            .analyze_program(&program)
            .map_err(|e| format!("Semantic error: {:?}", e))?;

        // Type inference
        let mut inference = InferenceEngine::new();
        let _inference_result = inference
            .infer_program(&program)
            .map_err(|e| format!("Inference error: {:?}", e))?;

        // Lower to IR
        let mut lowerer = create_ast_lowerer();
        let ir_module = lowerer
            .lower_program(&program)
            .map_err(|e| format!("Lowering error: {:?}", e))?;

        // Code generation
        let mut codegen = CodeGenerator::new();
        let executable = codegen
            .generate(&ir_module)
            .map_err(|e| format!("Codegen error: {:?}", e))?;

        Ok(executable)
    }
}

/// Benchmark the full compilation pipeline from source to executable
fn benchmark_full_pipeline(c: &mut Criterion) {
    let fixtures = [
        ("small", include_str!("fixtures/fibonacci_recursive.script")),
        ("large", include_str!("fixtures/large_program.script")),
        ("async_heavy", include_str!("fixtures/async_heavy.script")),
        (
            "pattern_matching",
            include_str!("fixtures/pattern_matching.script"),
        ),
    ];

    let mut group = c.benchmark_group("compilation_pipeline");

    for (name, source) in &fixtures {
        group.bench_with_input(
            BenchmarkId::new("full_compilation", name),
            source,
            |b, source| b.iter(|| helpers::compile_program(black_box(source))),
        );
    }

    group.finish();
}

/// Benchmark individual compilation stages
fn benchmark_compilation_stages(c: &mut Criterion) {
    let source = include_str!("fixtures/large_program.script");

    // Pre-compute tokens for parser benchmark
    let lexer = Lexer::new(source);
    let (tokens, _) = lexer.scan_tokens();

    // Pre-compute AST for semantic analysis
    let mut parser = Parser::new(tokens.clone());
    let program = parser.parse().unwrap();

    // Pre-compute analyzed AST for type inference
    let mut analyzer = SemanticAnalyzer::new();
    let _analyzed = analyzer.analyze_program(&program).unwrap();

    // Pre-compute typed AST for IR lowering
    let mut inference = InferenceEngine::new();
    let _inference_result = inference.infer_program(&program).unwrap();

    // Pre-compute IR for code generation
    let mut lowerer = helpers::create_ast_lowerer();
    let ir_module = lowerer.lower_program(&program).unwrap();

    let mut group = c.benchmark_group("compilation_stages");

    // Lexing stage
    group.bench_function("lexing", |b| {
        b.iter(|| {
            let lexer = Lexer::new(black_box(source));
            lexer.scan_tokens()
        })
    });

    // Parsing stage
    group.bench_function("parsing", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(tokens.clone()));
            parser.parse()
        })
    });

    // Semantic analysis stage
    group.bench_function("semantic_analysis", |b| {
        b.iter(|| {
            let mut analyzer = SemanticAnalyzer::new();
            analyzer.analyze_program(black_box(&program))
        })
    });

    // Type inference stage
    group.bench_function("type_inference", |b| {
        b.iter(|| {
            let mut inference = InferenceEngine::new();
            inference.infer_program(black_box(&program))
        })
    });

    // IR lowering stage
    group.bench_function("ir_lowering", |b| {
        b.iter(|| {
            let mut lowerer = helpers::create_ast_lowerer();
            lowerer.lower_program(black_box(&program))
        })
    });

    // Code generation stage
    group.bench_function("code_generation", |b| {
        b.iter(|| {
            let mut codegen = CodeGenerator::new();
            codegen.generate(black_box(&ir_module))
        })
    });

    group.finish();
}

/// Benchmark incremental compilation scenarios
fn benchmark_incremental_compilation(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_compilation");

    // Simulate changing a single function in a large program
    let base_source = include_str!("fixtures/large_program.script");
    let modified_source = base_source.replace(
        "fn add(a: f64, b: f64) -> f64 { return a + b }",
        "fn add(a: f64, b: f64) -> f64 { return a + b + 1.0 }",
    );

    group.bench_function("full_recompilation", |b| {
        b.iter(|| {
            let lexer = Lexer::new(black_box(&modified_source));
            let (tokens, _) = lexer.scan_tokens();
            let mut parser = Parser::new(tokens);
            let program = parser.parse().unwrap();
            let mut analyzer = SemanticAnalyzer::new();
            analyzer.analyze_program(&program)
        })
    });

    // TODO: Add actual incremental compilation when implemented
    // group.bench_function("incremental_update", |b| {
    //     b.iter(|| {
    //         // Incremental compilation logic
    //     })
    // });

    group.finish();
}

/// Benchmark parallel compilation of multiple modules
fn benchmark_parallel_compilation(c: &mut Criterion) {
    use crossbeam::thread;
    use std::sync::Arc;

    let sources = vec![
        include_str!("fixtures/fibonacci_recursive.script"),
        include_str!("fixtures/pattern_matching.script"),
        include_str!("fixtures/async_heavy.script"),
        include_str!("fixtures/game_simulation.script"),
    ];

    let mut group = c.benchmark_group("parallel_compilation");

    // Sequential compilation
    group.bench_function("sequential", |b| {
        b.iter(|| {
            for source in &sources {
                let lexer = Lexer::new(black_box(source));
                let (tokens, _) = lexer.scan_tokens();
                let mut parser = Parser::new(tokens);
                let _ = parser.parse();
            }
        })
    });

    // Parallel compilation
    group.bench_function("parallel", |b| {
        b.iter(|| {
            thread::scope(|s| {
                let handles: Vec<_> = sources
                    .iter()
                    .map(|source| {
                        s.spawn(move |_| {
                            let lexer = Lexer::new(black_box(source));
                            let (tokens, _) = lexer.scan_tokens();
                            let mut parser = Parser::new(tokens);
                            parser.parse()
                        })
                    })
                    .collect();

                for handle in handles {
                    let _ = handle.join();
                }
            })
            .unwrap();
        })
    });

    group.finish();
}

/// Benchmark runtime initialization
fn benchmark_runtime(c: &mut Criterion) {
    let mut group = c.benchmark_group("runtime");

    // Runtime initialization
    group.bench_function("initialization", |b| {
        b.iter(|| {
            let config = RuntimeConfig::default();
            Runtime::new(black_box(config))
        })
    });

    // Note: Runtime execution benchmarks are disabled as Runtime::execute method is not available
    // This would require integration with the actual execution engine once it's implemented

    group.finish();
}

criterion_group!(
    benches,
    benchmark_full_pipeline,
    benchmark_compilation_stages,
    benchmark_incremental_compilation,
    benchmark_parallel_compilation,
    benchmark_runtime
);
criterion_main!(benches);
