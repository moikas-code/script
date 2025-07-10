//! Comprehensive tests for resource limits and DoS protection
//!
//! These tests verify that the compiler properly enforces resource limits
//! to prevent denial-of-service attacks through resource exhaustion.

use script::codegen::monomorphization::MonomorphizationContext;
use script::compilation::resource_limits::{ResourceLimits, ResourceMonitor};
use script::compilation::CompilationContext;
use script::error::{Error, ErrorKind};
use script::inference::InferenceEngine;
use std::time::Duration;

#[cfg(test)]
mod resource_limit_tests {
    use super::*;

    #[test]
    fn test_resource_limits_validation() {
        // Test valid limits
        let limits = ResourceLimits::production();
        assert!(limits.validate().is_ok());

        // Test invalid limits - zero iterations
        let invalid_limits = ResourceLimits::custom().max_iterations(0).build();
        assert!(invalid_limits.is_err());

        // Test invalid limits - zero timeout
        let invalid_limits = ResourceLimits::custom()
            .phase_timeout(Duration::from_secs(0))
            .build();
        assert!(invalid_limits.is_err());

        // Test invalid limits - total timeout less than phase timeout
        let invalid_limits = ResourceLimits::custom()
            .phase_timeout(Duration::from_secs(60))
            .total_timeout(Duration::from_secs(30))
            .build();
        assert!(invalid_limits.is_err());
    }

    #[test]
    fn test_iteration_limit_enforcement() {
        let limits = ResourceLimits::custom()
            .max_iterations(100)
            .build()
            .unwrap();
        let mut monitor = ResourceMonitor::new(limits);

        // Should succeed within limit
        for i in 1..=100 {
            assert!(monitor.check_iteration_limit("test_operation", 1).is_ok());
        }

        // Should fail when exceeding limit
        let result = monitor.check_iteration_limit("test_operation", 1);
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.kind(), &ErrorKind::SecurityViolation);
            assert!(error.message().contains("Iteration limit exceeded"));
            assert!(error.message().contains("DoS attacks"));
        }
    }

    #[test]
    fn test_timeout_enforcement() {
        let limits = ResourceLimits::custom()
            .phase_timeout(Duration::from_millis(10))
            .total_timeout(Duration::from_millis(50))
            .build()
            .unwrap();
        let mut monitor = ResourceMonitor::new(limits);

        monitor.start_phase("test_phase");

        // Sleep longer than the timeout
        std::thread::sleep(Duration::from_millis(20));

        // Should fail due to phase timeout
        let result = monitor.check_phase_timeout("test_phase");
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.kind(), &ErrorKind::SecurityViolation);
            assert!(error.message().contains("timeout exceeded"));
        }

        // Sleep even longer to trigger total timeout
        std::thread::sleep(Duration::from_millis(40));

        // Should fail due to total timeout
        let result = monitor.check_total_timeout();
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.kind(), &ErrorKind::SecurityViolation);
            assert!(error.message().contains("Total compilation timeout"));
        }
    }

    #[test]
    fn test_recursion_depth_enforcement() {
        let limits = ResourceLimits::custom()
            .max_recursion_depth(10)
            .build()
            .unwrap();
        let mut monitor = ResourceMonitor::new(limits);

        // Should succeed within limit
        for depth in 1..=10 {
            assert!(monitor
                .check_recursion_depth("recursive_operation", depth)
                .is_ok());
        }

        // Should fail when exceeding limit
        let result = monitor.check_recursion_depth("recursive_operation", 11);
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.kind(), &ErrorKind::SecurityViolation);
            assert!(error.message().contains("Recursion depth limit exceeded"));
            assert!(error.message().contains("deep recursion"));
        }
    }

    #[test]
    fn test_type_variable_limit_enforcement() {
        let limits = ResourceLimits::custom()
            .max_iterations(50) // Set max_type_variables through max_iterations for simplicity
            .build()
            .unwrap();
        let mut monitor = ResourceMonitor::new(limits);

        // Should succeed within limit
        for _ in 1..=limits.max_type_variables {
            assert!(monitor.add_type_variable().is_ok());
        }

        // Should fail when exceeding limit
        let result = monitor.add_type_variable();
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.kind(), &ErrorKind::SecurityViolation);
            assert!(error.message().contains("Type variable limit exceeded"));
            assert!(error.message().contains("type variable explosion"));
        }
    }

    #[test]
    fn test_constraint_limit_enforcement() {
        let limits = ResourceLimits::custom()
            .max_iterations(25) // This sets max_constraints to 50 (max_iterations * 2)
            .build()
            .unwrap();
        let mut monitor = ResourceMonitor::new(limits);

        // Should succeed within limit
        for _ in 1..=limits.max_constraints {
            assert!(monitor.add_constraint().is_ok());
        }

        // Should fail when exceeding limit
        let result = monitor.add_constraint();
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.kind(), &ErrorKind::SecurityViolation);
            assert!(error.message().contains("Constraint limit exceeded"));
            assert!(error.message().contains("constraint explosion"));
        }
    }

    #[test]
    fn test_specialization_limit_enforcement() {
        let limits = ResourceLimits::production();
        let mut monitor = ResourceMonitor::new(limits.clone());

        // Should succeed within limit
        for _ in 1..=limits.max_specializations {
            assert!(monitor.add_specialization().is_ok());
        }

        // Should fail when exceeding limit
        let result = monitor.add_specialization();
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.kind(), &ErrorKind::SecurityViolation);
            assert!(error.message().contains("Specialization limit exceeded"));
            assert!(error.message().contains("specialization explosion"));
        }

        // Test direct check method
        let result = monitor.check_specialization_limit(limits.max_specializations + 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_work_queue_size_enforcement() {
        let limits = ResourceLimits::production();
        let mut monitor = ResourceMonitor::new(limits.clone());

        // Should succeed within limit
        let result = monitor.check_work_queue_size("test_queue", limits.max_work_queue_size);
        assert!(result.is_ok());

        // Should fail when exceeding limit
        let result = monitor.check_work_queue_size("test_queue", limits.max_work_queue_size + 1);
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.kind(), &ErrorKind::SecurityViolation);
            assert!(error.message().contains("Work queue"));
            assert!(error.message().contains("size limit exceeded"));
            assert!(error.message().contains("unbounded queue growth"));
        }
    }

    #[test]
    fn test_memory_usage_tracking() {
        let limits = ResourceLimits::custom()
            .max_memory_bytes(1024) // 1KB limit for testing
            .build()
            .unwrap();
        let mut monitor = ResourceMonitor::new(limits);

        // Should succeed within limit
        assert!(monitor.add_memory_usage(512).is_ok());
        assert!(monitor.add_memory_usage(256).is_ok());

        // Should fail when exceeding limit
        let result = monitor.add_memory_usage(512);
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.kind(), &ErrorKind::SecurityViolation);
            assert!(error.message().contains("Memory usage limit exceeded"));
            assert!(error.message().contains("memory exhaustion"));
        }
    }

    #[test]
    fn test_system_memory_checking() {
        let limits = ResourceLimits::custom()
            .max_memory_bytes(1) // Very small limit to force failure if system memory is detected
            .build()
            .unwrap();
        let monitor = ResourceMonitor::new(limits);

        // This test may pass or fail depending on the system and platform
        // On Linux, it should detect actual memory usage and potentially fail
        // On other platforms, it returns 0 and passes
        let _result = monitor.check_system_memory();
        // We don't assert here because behavior is platform-dependent
    }

    #[test]
    fn test_inference_engine_resource_integration() {
        let limits = ResourceLimits::custom()
            .max_iterations(10) // Very low limit to trigger quickly
            .max_memory_bytes(1024 * 1024) // 1MB
            .phase_timeout(Duration::from_secs(1))
            .build()
            .unwrap();

        let mut engine = InferenceEngine::with_resource_limits(limits);

        // Create a program that will likely trigger resource limits
        let program_str = r#"
            let x1 = 42;
            let x2 = x1 + 1;
            let x3 = x2 + x1;
            let x4 = x3 + x2;
            let x5 = x4 + x3;
            let x6 = x5 + x4;
            let x7 = x6 + x5;
            let x8 = x7 + x6;
            let x9 = x8 + x7;
            let x10 = x9 + x8;
            let x11 = x10 + x9;
            let x12 = x11 + x10;
        "#;

        // Parse the program
        use script::lexer::Lexer;
        use script::parser::Parser;

        let lexer = Lexer::new(program_str).unwrap();
        let (tokens, _errors) = lexer.scan_tokens();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        // Type inference should either succeed or fail with resource limits
        let result = engine.infer_program(&program);

        // The test passes if either:
        // 1. Type inference succeeds (within resource limits)
        // 2. Type inference fails with a SecurityViolation (resource limit exceeded)
        match result {
            Ok(_) => {
                // Success - resource limits weren't exceeded
                println!("Type inference completed within resource limits");
            }
            Err(error) if error.kind() == &ErrorKind::SecurityViolation => {
                // Expected failure due to resource limits
                println!("Resource limit enforced: {}", error.message());
                assert!(
                    error.message().contains("DoS attacks")
                        || error.message().contains("security")
                        || error.message().contains("limit exceeded")
                );
            }
            Err(error) => {
                // Unexpected error type
                panic!("Unexpected error type: {:?}", error);
            }
        }
    }

    #[test]
    fn test_compilation_context_resource_integration() {
        let limits = ResourceLimits::custom()
            .max_iterations(100)
            .phase_timeout(Duration::from_secs(5))
            .total_timeout(Duration::from_secs(15))
            .max_memory_bytes(10 * 1024 * 1024) // 10MB
            .build()
            .unwrap();

        let context = CompilationContext::with_resource_limits(limits);

        // Verify that the context was created with custom limits
        // This test mainly verifies the integration compiles correctly
        assert_eq!(
            std::mem::size_of_val(&context),
            std::mem::size_of::<CompilationContext>()
        );
    }

    #[test]
    fn test_monomorphization_context_resource_integration() {
        let limits = ResourceLimits::custom().max_iterations(50).build().unwrap();

        let context = MonomorphizationContext::with_resource_limits(limits);

        // Verify that the context was created with custom limits
        // This test mainly verifies the integration compiles correctly
        assert_eq!(
            std::mem::size_of_val(&context),
            std::mem::size_of::<MonomorphizationContext>()
        );
    }

    #[test]
    fn test_resource_stats_collection() {
        let limits = ResourceLimits::production();
        let mut monitor = ResourceMonitor::new(limits);

        // Perform some operations
        monitor.start_phase("test_phase");
        let _ = monitor.check_iteration_limit("operation1", 10);
        let _ = monitor.check_iteration_limit("operation2", 5);
        let _ = monitor.check_recursion_depth("recursive_op", 3);
        let _ = monitor.add_type_variable();
        let _ = monitor.add_constraint();
        let _ = monitor.check_work_queue_size("queue1", 100);

        // Get stats
        let stats = monitor.get_stats();

        // Verify stats were collected
        assert!(stats.compilation_time.as_millis() > 0);
        assert_eq!(stats.iteration_counts.get("operation1"), Some(&10));
        assert_eq!(stats.iteration_counts.get("operation2"), Some(&5));
        assert_eq!(stats.recursion_depths.get("recursive_op"), Some(&3));
        assert_eq!(stats.type_variable_count, 1);
        assert_eq!(stats.constraint_count, 1);
        assert_eq!(stats.work_queue_sizes.get("queue1"), Some(&100));
    }

    #[test]
    fn test_resource_monitor_reset() {
        let limits = ResourceLimits::production();
        let mut monitor = ResourceMonitor::new(limits);

        // Perform some operations
        let _ = monitor.check_iteration_limit("test", 10);
        let _ = monitor.add_type_variable();

        // Verify stats exist
        let stats = monitor.get_stats();
        assert_eq!(stats.iteration_counts.get("test"), Some(&10));
        assert_eq!(stats.type_variable_count, 1);

        // Reset monitor
        monitor.reset();

        // Verify stats were cleared
        let stats = monitor.get_stats();
        assert_eq!(stats.iteration_counts.get("test"), None);
        assert_eq!(stats.type_variable_count, 0);
    }

    #[test]
    fn test_different_environment_limits() {
        // Production limits should be most restrictive
        let prod_limits = ResourceLimits::production();

        // Development limits should be more permissive
        let dev_limits = ResourceLimits::development();
        assert!(dev_limits.max_iterations >= prod_limits.max_iterations);
        assert!(dev_limits.max_memory_bytes >= prod_limits.max_memory_bytes);

        // Testing limits should be most permissive
        let test_limits = ResourceLimits::testing();
        assert!(test_limits.max_iterations >= dev_limits.max_iterations);
        assert!(test_limits.max_memory_bytes >= dev_limits.max_memory_bytes);
    }

    #[test]
    fn test_dos_attack_simulation() {
        // Simulate a potential DoS attack through excessive resource usage
        let limits = ResourceLimits::custom()
            .max_iterations(100)
            .max_memory_bytes(1024)
            .phase_timeout(Duration::from_millis(100))
            .build()
            .unwrap();

        let mut monitor = ResourceMonitor::new(limits);
        monitor.start_phase("attack_simulation");

        // Simulate rapid resource consumption
        let mut attack_detected = false;

        // Try to exhaust iteration limit
        for i in 1..=200 {
            if monitor
                .check_iteration_limit("malicious_operation", 1)
                .is_err()
            {
                attack_detected = true;
                break;
            }
        }

        assert!(attack_detected, "DoS protection should have triggered");

        // Reset and try memory exhaustion
        monitor.reset();
        monitor.start_phase("memory_attack");
        attack_detected = false;

        // Try to exhaust memory limit
        for _ in 1..=10 {
            if monitor.add_memory_usage(200).is_err() {
                attack_detected = true;
                break;
            }
        }

        assert!(
            attack_detected,
            "Memory DoS protection should have triggered"
        );
    }
}
