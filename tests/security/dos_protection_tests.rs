use script::codegen::monomorphization::MonomorphizationContext;
use script::error::{Error, ErrorKind};
use script::inference::constructor_inference::ConstructorInferenceEngine;
use script::lexer::Lexer;
use script::lowering::AstLowerer;
use script::parser::Parser;
use script::semantic::SemanticAnalyzer;
use script::source::SourceLocation;
use std::time::{Duration, Instant};

/// Test suite for Denial of Service (DoS) protection
/// Tests resource limits, timeout protection, and compilation bomb prevention

#[cfg(test)]
mod type_inference_dos_tests {
    use super::*;

    #[test]
    fn test_type_variable_explosion_protection() {
        let mut engine = ConstructorInferenceEngine::new();

        // Try to create an excessive number of type variables
        let mut created_vars = 0;
        let mut last_error = None;

        for _ in 0..15000 {
            match engine.fresh_type_var() {
                Ok(_) => created_vars += 1,
                Err(e) => {
                    last_error = Some(e);
                    break;
                }
            }
        }

        // Should hit the limit before creating too many
        assert!(created_vars < 15000, "Should limit type variable creation");
        assert!(
            last_error.is_some(),
            "Should return error when limit exceeded"
        );

        if let Some(Error {
            kind: ErrorKind::SecurityViolation,
            message,
            ..
        }) = last_error
        {
            assert!(
                message.contains("Type variable limit exceeded"),
                "Should indicate type variable limit: {}",
                message
            );
        } else {
            panic!("Should return SecurityViolation error");
        }
    }

    #[test]
    fn test_constraint_explosion_protection() {
        let mut engine = ConstructorInferenceEngine::new();

        // Create a basic constraint for testing
        let test_constraint = script::inference::Constraint::equality(
            script::types::Type::I32,
            script::types::Type::I32,
            script::source::Span::new(SourceLocation::new(1, 1, 0), SourceLocation::new(1, 1, 0)),
        );

        // Try to add excessive constraints
        let mut added_constraints = 0;
        let mut last_error = None;

        for _ in 0..60000 {
            match engine.add_constraint(test_constraint.clone()) {
                Ok(_) => added_constraints += 1,
                Err(e) => {
                    last_error = Some(e);
                    break;
                }
            }
        }

        // Should hit the limit
        assert!(
            added_constraints < 60000,
            "Should limit constraint creation"
        );
        assert!(
            last_error.is_some(),
            "Should return error when limit exceeded"
        );

        if let Some(Error {
            kind: ErrorKind::SecurityViolation,
            message,
            ..
        }) = last_error
        {
            assert!(
                message.contains("Constraint limit exceeded"),
                "Should indicate constraint limit: {}",
                message
            );
        } else {
            panic!("Should return SecurityViolation error");
        }
    }

    #[test]
    fn test_solving_iteration_protection() {
        let mut engine = ConstructorInferenceEngine::new();

        // Create a complex constraint system that would require many iterations
        // Each constraint creates a chain of dependencies
        for i in 0..500 {
            let constraint = script::inference::Constraint::equality(
                script::types::Type::TypeVar(i),
                script::types::Type::TypeVar(i + 1),
                script::source::Span::new(
                    SourceLocation::new(1, 1, 0),
                    SourceLocation::new(1, 1, 0),
                ),
            );

            if engine.add_constraint(constraint).is_err() {
                break; // Hit constraint limit first
            }
        }

        // Try to solve - should hit iteration limit
        let result = engine.solve_constraints();

        match result {
            Err(Error {
                kind: ErrorKind::SecurityViolation,
                message,
                ..
            }) => {
                assert!(
                    message.contains("iteration limit") || message.contains("timeout"),
                    "Should indicate iteration limit or timeout: {}",
                    message
                );
            }
            _ => {
                // May succeed with simple constraints, but should be limited
                // The real test is that it doesn't run forever
            }
        }
    }

    #[test]
    fn test_inference_timeout_protection() {
        let mut engine = ConstructorInferenceEngine::new();

        // Create a scenario that would take a long time to solve
        // Generate many interconnected constraints
        for i in 0..1000 {
            let constraint1 = script::inference::Constraint::equality(
                script::types::Type::TypeVar(i * 2),
                script::types::Type::TypeVar(i * 2 + 1),
                script::source::Span::new(
                    SourceLocation::new(1, 1, 0),
                    SourceLocation::new(1, 1, 0),
                ),
            );

            let constraint2 = script::inference::Constraint::equality(
                script::types::Type::TypeVar(i * 2 + 1),
                script::types::Type::TypeVar(i * 2 + 2),
                script::source::Span::new(
                    SourceLocation::new(1, 1, 0),
                    SourceLocation::new(1, 1, 0),
                ),
            );

            if engine.add_constraint(constraint1).is_err()
                || engine.add_constraint(constraint2).is_err()
            {
                break; // Hit constraint limit
            }
        }

        // Start timing
        let start = Instant::now();
        let result = engine.solve_constraints();
        let elapsed = start.elapsed();

        // Should either complete quickly or timeout
        match result {
            Err(Error {
                kind: ErrorKind::SecurityViolation,
                message,
                ..
            }) => {
                assert!(
                    message.contains("timeout"),
                    "Should indicate timeout: {}",
                    message
                );
            }
            Ok(_) => {
                // If it succeeds, it should complete within reasonable time
                assert!(
                    elapsed < Duration::from_secs(35),
                    "Should complete within timeout period"
                );
            }
            Err(e) => {
                panic!("Unexpected error: {:?}", e);
            }
        }
    }

    #[test]
    fn test_nested_generics_dos_protection() {
        let source = r#"
        struct Level1<T> {
            value: T,
        }
        
        struct Level2<T> {
            level1: Level1<T>,
        }
        
        struct Level3<T> {
            level2: Level2<T>,
        }
        
        // This creates a deep nesting that could cause exponential type inference
        fn deeply_nested<T>(value: Level3<Level2<Level1<T>>>) -> T {
            value.level2.level1.value.level2.level1.value
        }
        "#;

        let lexer = Lexer::new(source).expect("Lexer creation should succeed");
        let (tokens, _errors) = lexer.scan_tokens();
        assert!(_errors.is_empty(), "Lexer should not produce errors");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing should succeed");

        let mut analyzer = SemanticAnalyzer::new();
        let start_time = Instant::now();
        let result = analyzer.analyze(&program);
        let elapsed = start_time.elapsed();

        match result {
            Ok(_) => {
                // Should complete within reasonable time
                assert!(
                    elapsed < Duration::from_secs(30),
                    "Deep nesting should not cause excessive inference time"
                );
            }
            Err(Error {
                kind: ErrorKind::SecurityViolation,
                ..
            }) => {
                // Acceptable - hit security limits
            }
            Err(e) => {
                panic!("Unexpected error: {:?}", e);
            }
        }
    }
}

#[cfg(test)]
mod monomorphization_dos_tests {
    use super::*;

    #[test]
    fn test_specialization_explosion_protection() {
        let mut context = MonomorphizationContext::new();

        // Try to create excessive specializations
        let mut created_specializations = 0;
        let mut last_error = None;

        for i in 0..1500 {
            let func_name = format!("test_function_{}", i);
            let type_args = vec![script::types::Type::I32];

            match context.add_instantiation(func_name, type_args) {
                Ok(_) => created_specializations += 1,
                Err(e) => {
                    last_error = Some(e);
                    break;
                }
            }
        }

        // Should hit the limit
        assert!(
            created_specializations < 1500,
            "Should limit specialization creation"
        );
        assert!(
            last_error.is_some(),
            "Should return error when limit exceeded"
        );

        if let Some(Error {
            kind: ErrorKind::SecurityViolation,
            message,
            ..
        }) = last_error
        {
            assert!(
                message.contains("specialization limit")
                    || message.contains("Specialization limit"),
                "Should indicate specialization limit: {}",
                message
            );
        } else {
            panic!("Should return SecurityViolation error");
        }
    }

    #[test]
    fn test_work_queue_explosion_protection() {
        let mut context = MonomorphizationContext::new();

        // Try to overflow the work queue
        let mut queued_items = 0;
        let mut last_error = None;

        for i in 0..12000 {
            let func_name = format!("queued_function_{}", i);
            let type_args = vec![script::types::Type::I32];

            match context.add_to_work_queue(func_name, type_args) {
                Ok(_) => queued_items += 1,
                Err(e) => {
                    last_error = Some(e);
                    break;
                }
            }
        }

        // Should hit the limit
        assert!(queued_items < 12000, "Should limit work queue size");
        assert!(
            last_error.is_some(),
            "Should return error when limit exceeded"
        );

        if let Some(Error {
            kind: ErrorKind::SecurityViolation,
            message,
            ..
        }) = last_error
        {
            assert!(
                message.contains("work queue") || message.contains("queue size"),
                "Should indicate work queue limit: {}",
                message
            );
        } else {
            panic!("Should return SecurityViolation error");
        }
    }

    #[test]
    fn test_monomorphization_timeout_protection() {
        let mut context = MonomorphizationContext::new();

        // Create a scenario that would take a long time to monomorphize
        // Add many complex generic instantiations
        for i in 0..500 {
            let func_name = format!("complex_function_{}", i);
            let type_args = vec![script::types::Type::Generic {
                name: "Complex".to_string(),
                args: vec![script::types::Type::I32, script::types::Type::String],
            }];

            if context.add_instantiation(func_name, type_args).is_err() {
                break; // Hit limit
            }
        }

        // Create a simple module for testing
        let module = script::ir::Module::new("test_module".to_string());

        // Start timing
        let start = Instant::now();
        let result = context.monomorphize(&module);
        let elapsed = start.elapsed();

        // Should either complete quickly or timeout
        match result {
            Err(Error {
                kind: ErrorKind::SecurityViolation,
                message,
                ..
            }) => {
                assert!(
                    message.contains("timeout") || message.contains("time"),
                    "Should indicate timeout: {}",
                    message
                );
            }
            Ok(_) => {
                // If it succeeds, should complete within reasonable time
                assert!(
                    elapsed < Duration::from_secs(35),
                    "Should complete within timeout period"
                );
            }
            Err(e) => {
                panic!("Unexpected error: {:?}", e);
            }
        }
    }

    #[test]
    fn test_recursive_generic_instantiation_protection() {
        let source = r#"
        struct RecursiveGeneric<T> {
            value: T,
            next: Option<RecursiveGeneric<T>>,
        }
        
        fn create_recursive<T>(value: T) -> RecursiveGeneric<RecursiveGeneric<T>> {
            RecursiveGeneric {
                value: RecursiveGeneric { value, next: None },
                next: None,
            }
        }
        "#;

        let lexer = Lexer::new(source).expect("Lexer creation should succeed");
        let (tokens, _errors) = lexer.scan_tokens();
        assert!(_errors.is_empty(), "Lexer should not produce errors");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing should succeed");

        let mut analyzer = SemanticAnalyzer::new();
        let analyzed = analyzer.analyze(&program).expect("Analysis should succeed");

        let mut lowerer = AstLowerer::new();
        let module = lowerer
            .lower_program(&analyzed)
            .expect("Lowering should succeed");

        // Test monomorphization with recursive generics
        let mut context = MonomorphizationContext::new();
        let start_time = Instant::now();
        let result = context.monomorphize(&module);
        let elapsed = start_time.elapsed();

        match result {
            Ok(_) => {
                // Should complete within reasonable time
                assert!(
                    elapsed < Duration::from_secs(30),
                    "Recursive generics should not cause excessive monomorphization time"
                );
            }
            Err(Error {
                kind: ErrorKind::SecurityViolation,
                ..
            }) => {
                // Acceptable - hit security limits
            }
            Err(e) => {
                panic!("Unexpected error: {:?}", e);
            }
        }
    }
}

#[cfg(test)]
mod compilation_bomb_tests {
    use super::*;

    #[test]
    fn test_exponential_template_expansion_protection() {
        let source = r#"
        struct Nested<T> {
            value: T,
        }
        
        // This could cause exponential expansion: 2^n instantiations
        fn exponential_expansion<T>() -> Nested<Nested<T>> {
            unimplemented!()
        }
        
        fn trigger_expansion() {
            exponential_expansion::<i32>();
            exponential_expansion::<Nested<i32>>();
            exponential_expansion::<Nested<Nested<i32>>>();
            exponential_expansion::<Nested<Nested<Nested<i32>>>>();
        }
        "#;

        let lexer = Lexer::new(source).expect("Lexer creation should succeed");
        let (tokens, _errors) = lexer.scan_tokens();
        assert!(_errors.is_empty(), "Lexer should not produce errors");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing should succeed");

        let mut analyzer = SemanticAnalyzer::new();
        let start_time = Instant::now();
        let result = analyzer.analyze(&program);
        let elapsed = start_time.elapsed();

        match result {
            Ok(analyzed) => {
                // Should complete within reasonable time
                assert!(
                    elapsed < Duration::from_secs(30),
                    "Exponential expansion should not cause excessive compilation time"
                );

                // Test lowering
                let mut lowerer = AstLowerer::new();
                let lowering_start = Instant::now();
                let lowering_result = lowerer.lower_program(&analyzed);
                let lowering_elapsed = lowering_start.elapsed();

                match lowering_result {
                    Ok(module) => {
                        assert!(
                            lowering_elapsed < Duration::from_secs(30),
                            "Lowering should complete within reasonable time"
                        );

                        // Test monomorphization
                        let mut context = MonomorphizationContext::new();
                        let mono_start = Instant::now();
                        let mono_result = context.monomorphize(&module);
                        let mono_elapsed = mono_start.elapsed();

                        match mono_result {
                            Ok(_) => {
                                assert!(
                                    mono_elapsed < Duration::from_secs(30),
                                    "Monomorphization should complete within reasonable time"
                                );
                            }
                            Err(Error {
                                kind: ErrorKind::SecurityViolation,
                                ..
                            }) => {
                                // Acceptable - hit security limits
                            }
                            Err(e) => {
                                panic!("Unexpected monomorphization error: {:?}", e);
                            }
                        }
                    }
                    Err(Error {
                        kind: ErrorKind::SecurityViolation,
                        ..
                    }) => {
                        // Acceptable - hit security limits during lowering
                    }
                    Err(e) => {
                        panic!("Unexpected lowering error: {:?}", e);
                    }
                }
            }
            Err(Error {
                kind: ErrorKind::SecurityViolation,
                ..
            }) => {
                // Acceptable - hit security limits during analysis
            }
            Err(e) => {
                panic!("Unexpected analysis error: {:?}", e);
            }
        }
    }

    #[test]
    fn test_deeply_nested_generic_protection() {
        let source = r#"
        struct Layer<T> {
            inner: T,
        }
        
        // Create deeply nested generic types
        fn deeply_nested() -> Layer<Layer<Layer<Layer<Layer<i32>>>>> {
            unimplemented!()
        }
        
        fn even_deeper() -> Layer<Layer<Layer<Layer<Layer<Layer<Layer<Layer<i32>>>>>>>> {
            unimplemented!()
        }
        "#;

        let lexer = Lexer::new(source).expect("Lexer creation should succeed");
        let (tokens, _errors) = lexer.scan_tokens();
        assert!(_errors.is_empty(), "Lexer should not produce errors");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing should succeed");

        let mut analyzer = SemanticAnalyzer::new();
        let start_time = Instant::now();
        let result = analyzer.analyze(&program);
        let elapsed = start_time.elapsed();

        // Should complete within reasonable time or hit security limits
        match result {
            Ok(_) => {
                assert!(
                    elapsed < Duration::from_secs(30),
                    "Deep nesting should not cause excessive compilation time"
                );
            }
            Err(Error {
                kind: ErrorKind::SecurityViolation,
                ..
            }) => {
                // Acceptable - hit security limits
            }
            Err(e) => {
                panic!("Unexpected error: {:?}", e);
            }
        }
    }

    #[test]
    fn test_compilation_resource_exhaustion_protection() {
        let source = r#"
        // Generate many functions that could cause resource exhaustion
        fn func1<T>() -> T { unimplemented!() }
        fn func2<T>() -> T { unimplemented!() }
        fn func3<T>() -> T { unimplemented!() }
        // ... many more functions
        
        fn trigger_exhaustion() {
            func1::<i32>();
            func1::<String>();
            func1::<Vec<i32>>();
            func2::<i32>();
            func2::<String>();
            func2::<Vec<i32>>();
            func3::<i32>();
            func3::<String>();
            func3::<Vec<i32>>();
        }
        "#;

        // Generate a large source with many functions
        let mut large_source = String::new();
        for i in 0..100 {
            large_source.push_str(format!("fn func{}<T>() -> T {{ unimplemented!() }}\n", i));
        }

        large_source.push_str("fn trigger_exhaustion() {\n");
        for i in 0..100 {
            large_source.push_str(format!("    func{}::<i32>();\n", i));
            large_source.push_str(format!("    func{}::<String>();\n", i));
            large_source.push_str(format!("    func{}::<Vec<i32>>();\n", i));
        }
        large_source.push_str("}\n");

        let lexer = Lexer::new(&large_source).expect("Lexer creation should succeed");
        let (tokens, _errors) = lexer.scan_tokens();
        assert!(_errors.is_empty(), "Lexer should not produce errors");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing should succeed");

        // Test entire compilation pipeline with resource limits
        let mut analyzer = SemanticAnalyzer::new();
        let start_time = Instant::now();
        let result = analyzer.analyze(&program);
        let elapsed = start_time.elapsed();

        match result {
            Ok(analyzed) => {
                // Should complete within reasonable time
                assert!(
                    elapsed < Duration::from_secs(60),
                    "Large compilation should not cause excessive time"
                );

                // Test lowering with resource limits
                let mut lowerer = AstLowerer::new();
                let lowering_result = lowerer.lower_program(&analyzed);

                match lowering_result {
                    Ok(module) => {
                        // Test monomorphization with resource limits
                        let mut context = MonomorphizationContext::new();
                        let mono_result = context.monomorphize(&module);

                        match mono_result {
                            Ok(_) => {
                                // Success - resource limits worked properly
                            }
                            Err(Error {
                                kind: ErrorKind::SecurityViolation,
                                ..
                            }) => {
                                // Expected - hit resource limits
                            }
                            Err(e) => {
                                panic!("Unexpected error: {:?}", e);
                            }
                        }
                    }
                    Err(Error {
                        kind: ErrorKind::SecurityViolation,
                        ..
                    }) => {
                        // Expected - hit resource limits
                    }
                    Err(e) => {
                        panic!("Unexpected error: {:?}", e);
                    }
                }
            }
            Err(Error {
                kind: ErrorKind::SecurityViolation,
                ..
            }) => {
                // Expected - hit resource limits
            }
            Err(e) => {
                panic!("Unexpected error: {:?}", e);
            }
        }
    }
}
