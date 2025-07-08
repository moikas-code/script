//! Comprehensive security tests for the module system

use script::module::{
    PathSecurityValidator, ModulePathSanitizer, ModuleIntegrityVerifier,
    ModuleChecksum, TrustLevel as IntegrityTrustLevel, VerificationRequirements,
    ResourceMonitor, ResourceMonitorLimits, SecurityAuditLogger, AuditConfig,
    SecuritySeverity, SecurityEventCategory, ModulePath, ModuleError,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_path_traversal_attacks() {
    let temp_dir = TempDir::new().unwrap();
    let validator = PathSecurityValidator::new(temp_dir.path().to_path_buf()).unwrap();
    
    // Various path traversal attempts
    let attack_vectors = vec![
        "../../../etc/passwd",
        "../../root/.ssh/id_rsa",
        "../..",
        "foo/../../bar",
        "foo/../../../etc/hosts",
        "./../../sensitive",
        "~/.bashrc",
        "$HOME/secrets",
        "%APPDATA%\\config",
        "..\\..\\windows\\system32",
        "/etc/passwd",
        "\\\\server\\share",
        "C:\\Windows\\System32",
        "file:///etc/passwd",
        "foo/./../../bar",
        "foo//../../bar",
        "foo/./../bar",
        "foo\0bar",
        "foo\rbar",
        "foo\nbar",
    ];
    
    for attack in attack_vectors {
        let result = validator.validate_module_path(attack);
        assert!(result.is_err(), "Path traversal not blocked: {}", attack);
        
        if let Err(e) = result {
            assert!(e.to_string().contains("security violation") ||
                    e.to_string().contains("Invalid character") ||
                    e.to_string().contains("Forbidden pattern"));
        }
    }
}

#[test]
fn test_symlink_protection() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();
    
    // Create a file outside project
    let outside_dir = temp_dir.path().join("outside");
    fs::create_dir(&outside_dir).unwrap();
    let sensitive_file = outside_dir.join("sensitive.script");
    fs::write(&sensitive_file, "sensitive data").unwrap();
    
    // Create symlink inside project pointing outside
    let link_path = project_root.join("evil_link.script");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&sensitive_file, &link_path).unwrap();
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(&sensitive_file, &link_path).unwrap();
    }
    
    let validator = PathSecurityValidator::new(project_root).unwrap();
    
    // Should detect symlink if symlinks are disabled
    let result = validator.validate_module_path("evil_link");
    
    // The validator should either reject the symlink or the path that escapes bounds
    if result.is_ok() {
        let validated_path = result.unwrap();
        // If it got through initial validation, the bounds check should catch it
        assert!(!validated_path.starts_with(&outside_dir));
    }
}

#[test]
fn test_module_name_sanitization() {
    // Valid module names
    let valid_names = vec![
        "module",
        "my_module",
        "MyModule",
        "module123",
        "module_123",
        "a",
        "A1",
    ];
    
    for name in valid_names {
        let result = ModulePathSanitizer::sanitize_module_name(name);
        assert!(result.is_ok(), "Valid name rejected: {}", name);
    }
    
    // Invalid module names
    let invalid_names = vec![
        "",
        "module-name",
        "module.name",
        "module name",
        "module/name",
        "module\\name",
        "../module",
        "module$",
        "module@",
        "module!",
        "module#",
        "module%",
        "module&",
        "module*",
        "module(",
        "module)",
        "module[",
        "module]",
        "module{",
        "module}",
        "module<",
        "module>",
        "module?",
        "module|",
        "module;",
        "module:",
        "module'",
        "module\"",
        "module`",
        "module~",
        &"x".repeat(100), // Too long
    ];
    
    for name in invalid_names {
        let result = ModulePathSanitizer::sanitize_module_name(name);
        assert!(result.is_err(), "Invalid name accepted: {}", name);
    }
    
    // Reserved names
    let reserved_names = vec!["script", "mod", "lib", "bin", "test", "bench"];
    
    for name in reserved_names {
        let result = ModulePathSanitizer::sanitize_module_name(name);
        assert!(result.is_err(), "Reserved name accepted: {}", name);
    }
}

#[test]
fn test_integrity_verification() {
    let temp_dir = TempDir::new().unwrap();
    let verifier = ModuleIntegrityVerifier::new(true);
    
    // Create test module
    let module_path = ModulePath::from_string("test.module").unwrap();
    let file_path = temp_dir.path().join("test.script");
    let content = b"fn main() { println(\"Hello\"); }";
    fs::write(&file_path, content).unwrap();
    
    // First verification should succeed (unknown module)
    let result1 = verifier.verify_module(&module_path, &file_path).unwrap();
    assert_eq!(result1.trust_level, IntegrityTrustLevel::Unknown);
    assert!(!result1.checksum.sha256.is_empty());
    
    // Register as trusted
    verifier.register_trusted_module(
        module_path.clone(),
        result1.checksum.clone(),
        IntegrityTrustLevel::Trusted,
        VerificationRequirements::default(),
    ).unwrap();
    
    // Second verification should show as trusted
    let result2 = verifier.verify_module(&module_path, &file_path).unwrap();
    assert_eq!(result2.trust_level, IntegrityTrustLevel::Trusted);
    assert_eq!(result2.checksum.sha256, result1.checksum.sha256);
    
    // Modify file
    fs::write(&file_path, b"fn main() { println(\"Modified\"); }").unwrap();
    
    // Should fail verification now
    let result3 = verifier.verify_module(&module_path, &file_path).unwrap();
    assert_eq!(result3.trust_level, IntegrityTrustLevel::Unknown);
    assert_ne!(result3.checksum.sha256, result1.checksum.sha256);
}

#[test]
fn test_resource_limits() {
    let limits = ResourceMonitorLimits {
        max_modules: 3,
        max_dependency_depth: 5,
        max_module_size: 1000,
        max_total_memory: 2000,
        ..Default::default()
    };
    
    let monitor = ResourceMonitor::with_limits(limits);
    
    // Test module count limit
    let module1 = ModulePath::from_string("mod1").unwrap();
    let module2 = ModulePath::from_string("mod2").unwrap();
    let module3 = ModulePath::from_string("mod3").unwrap();
    let module4 = ModulePath::from_string("mod4").unwrap();
    
    monitor.check_module_load(&module1, 500).unwrap();
    monitor.record_module_load(module1, 500, 5).unwrap();
    
    monitor.check_module_load(&module2, 500).unwrap();
    monitor.record_module_load(module2, 500, 5).unwrap();
    
    monitor.check_module_load(&module3, 500).unwrap();
    monitor.record_module_load(module3, 500, 5).unwrap();
    
    // Fourth module should fail
    let result = monitor.check_module_load(&module4, 500);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Module limit exceeded"));
    
    // Test memory limit
    let monitor2 = ResourceMonitor::with_limits(limits);
    let big_module = ModulePath::from_string("big").unwrap();
    
    // Module too large
    let result = monitor2.check_module_load(&big_module, 1500);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Module too large"));
    
    // Would exceed total memory
    monitor2.check_module_load(&module1, 800).unwrap();
    monitor2.record_module_load(module1, 800, 5).unwrap();
    
    let result = monitor2.check_module_load(&module2, 1500);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Memory limit would be exceeded"));
    
    // Test dependency depth
    assert!(monitor2.check_dependency_depth(4).is_ok());
    assert!(monitor2.check_dependency_depth(5).is_ok());
    assert!(monitor2.check_dependency_depth(6).is_err());
}

#[test]
fn test_audit_logging() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("security.log");
    
    let config = AuditConfig {
        log_file: log_file.clone(),
        severity_filter: SecuritySeverity::Info,
        real_time_alerts: false,
        ..Default::default()
    };
    
    let logger = SecurityAuditLogger::new(config).unwrap();
    
    // Log various security events
    let module = ModulePath::from_string("test.module").unwrap();
    
    // Path traversal attempt
    logger.log_path_traversal(
        Some(module.clone()),
        "../../../etc/passwd",
        &ModuleError::security_violation("Path traversal detected"),
    ).unwrap();
    
    // Integrity violation
    logger.log_integrity_violation(
        module.clone(),
        "expected_hash",
        "actual_hash",
    ).unwrap();
    
    // Resource exhaustion
    logger.log_resource_exhaustion(
        Some(module.clone()),
        "modules",
        100,
        150,
    ).unwrap();
    
    // Module load success
    logger.log_module_load(
        module.clone(),
        &temp_dir.path().join("test.script"),
        "abc123",
    ).unwrap();
    
    // Check statistics
    let stats = logger.get_statistics();
    assert_eq!(stats.total_events, 4);
    assert!(stats.last_critical_event.is_some());
    
    // Check log file exists and contains events
    assert!(log_file.exists());
    let log_content = fs::read_to_string(&log_file).unwrap();
    assert!(log_content.contains("PathTraversal"));
    assert!(log_content.contains("IntegrityViolation"));
    assert!(log_content.contains("ResourceExhaustion"));
    assert!(log_content.contains("ModuleLoad"));
}

#[test]
fn test_dependency_confusion_protection() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();
    
    // Create project structure
    let src_dir = project_root.join("src");
    fs::create_dir(&src_dir).unwrap();
    
    // Create legitimate module
    let legit_module = src_dir.join("auth.script");
    fs::write(&legit_module, "// Legitimate auth module").unwrap();
    
    // Try to create malicious module in parent directory
    let malicious = project_root.join("../auth.script");
    
    let validator = PathSecurityValidator::new(project_root.clone()).unwrap();
    
    // Should not be able to access parent directory
    let result = validator.validate_module_path("../auth");
    assert!(result.is_err());
}

#[test]
fn test_resource_exhaustion_via_imports() {
    let monitor = ResourceMonitor::new();
    
    // Test import count limit
    for i in 0..150 {
        let result = monitor.check_import_count(i);
        if i <= 100 {
            assert!(result.is_ok(), "Import count {} should be allowed", i);
        } else {
            assert!(result.is_err(), "Import count {} should be rejected", i);
        }
    }
    
    // Test cycle detection iteration limit
    for _ in 0..9999 {
        monitor.check_cycle_iterations().unwrap();
    }
    
    // Next iterations should fail
    for _ in 0..10 {
        let result = monitor.check_cycle_iterations();
        assert!(result.is_err());
    }
}

#[test]
fn test_concurrent_operation_limits() {
    let limits = ResourceMonitorLimits {
        max_concurrent_ops: 2,
        ..Default::default()
    };
    
    let monitor = ResourceMonitor::with_limits(limits);
    
    // Start operations up to limit
    let _op1 = monitor.begin_operation("op1".to_string()).unwrap();
    let _op2 = monitor.begin_operation("op2".to_string()).unwrap();
    
    // Third should fail
    let result = monitor.begin_operation("op3".to_string());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Concurrent operation limit"));
    
    // Drop one
    drop(_op1);
    
    // Now should succeed
    let _op3 = monitor.begin_operation("op3".to_string()).unwrap();
}

#[test]
fn test_safe_display_path() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();
    let validator = PathSecurityValidator::new(project_root.clone()).unwrap();
    
    // Path within project
    let internal_path = project_root.join("src/module.script");
    assert_eq!(validator.safe_display_path(&internal_path), "src/module.script");
    
    // Path outside project
    let external_path = PathBuf::from("/etc/passwd");
    assert_eq!(validator.safe_display_path(&external_path), "<external>");
}