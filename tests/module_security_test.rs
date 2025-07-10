//! Tests for module security, permissions, and sandboxing features

use script::module::{
    CompilationConfig, FileSystemPermission, HostPattern, ModuleCapability,
    ModuleCompilationPipeline, ModuleLoadContext, ModulePath, ModulePermissions, ModuleSandbox,
    ModuleSecurityContext, ModuleSecurityManager, NetworkPermission, PathPattern, Permission,
    PermissionManager, SandboxConfig, TrustLevel,
};
use script::runtime::Value;
use std::path::PathBuf;
use std::time::Duration;

#[test]
fn test_trust_level_capabilities() {
    // System level should allow everything
    assert!(TrustLevel::System.allows_capability(&ModuleCapability::SystemCall));
    assert!(TrustLevel::System.allows_capability(&ModuleCapability::UnsafeCode));
    assert!(TrustLevel::System.allows_capability(&ModuleCapability::FFICall));

    // Trusted level should allow most things except system operations
    assert!(TrustLevel::Trusted.allows_capability(&ModuleCapability::FileRead(PathBuf::new())));
    assert!(TrustLevel::Trusted
        .allows_capability(&ModuleCapability::NetworkConnect("example.com".to_string())));
    assert!(!TrustLevel::Trusted.allows_capability(&ModuleCapability::SystemCall));
    assert!(!TrustLevel::Trusted.allows_capability(&ModuleCapability::UnsafeCode));

    // Untrusted level should be restricted
    assert!(TrustLevel::Untrusted.allows_capability(&ModuleCapability::FileRead(PathBuf::new())));
    assert!(!TrustLevel::Untrusted.allows_capability(&ModuleCapability::FileWrite(PathBuf::new())));
    assert!(!TrustLevel::Untrusted
        .allows_capability(&ModuleCapability::NetworkConnect("example.com".to_string())));
    assert!(!TrustLevel::Untrusted.allows_capability(&ModuleCapability::ProcessSpawn));

    // Sandbox level should be highly restricted
    assert!(!TrustLevel::Sandbox.allows_capability(&ModuleCapability::FileRead(PathBuf::new())));
    assert!(!TrustLevel::Sandbox
        .allows_capability(&ModuleCapability::NetworkConnect("example.com".to_string())));
    assert!(
        TrustLevel::Sandbox.allows_capability(&ModuleCapability::ResourceAllocation {
            cpu_time: 500,
            memory: 5_000_000
        })
    );
    assert!(
        !TrustLevel::Sandbox.allows_capability(&ModuleCapability::ResourceAllocation {
            cpu_time: 2000,
            memory: 20_000_000
        })
    );
}

#[test]
fn test_module_security_context() {
    let module_path = ModulePath::from_string("test.security.module").unwrap();
    let mut ctx = ModuleSecurityContext::new(module_path.clone(), TrustLevel::Trusted);

    // Should be able to grant allowed capabilities
    assert!(ctx
        .grant_capability(ModuleCapability::FileRead(PathBuf::from("/tmp")))
        .is_ok());
    assert!(ctx
        .grant_capability(ModuleCapability::NetworkConnect("example.com".to_string()))
        .is_ok());

    // Should not be able to grant disallowed capabilities
    assert!(ctx.grant_capability(ModuleCapability::SystemCall).is_err());
    assert!(ctx.grant_capability(ModuleCapability::UnsafeCode).is_err());

    // Check capability should work for granted capabilities
    assert!(ctx
        .check_capability(&ModuleCapability::FileRead(PathBuf::from("/tmp")))
        .is_ok());

    // Check capability should fail for non-granted capabilities
    assert!(ctx
        .check_capability(&ModuleCapability::FileWrite(PathBuf::from("/tmp")))
        .is_err());
    assert!(ctx
        .check_capability(&ModuleCapability::ProcessSpawn)
        .is_err());
}

#[test]
fn test_module_security_manager() {
    let mut manager = ModuleSecurityManager::new();

    // System modules should get system trust level
    let std_path = ModulePath::from_string("std.io").unwrap();
    let std_ctx = manager.get_or_create_context(&std_path);
    assert_eq!(std_ctx.trust_level, TrustLevel::System);

    // Core modules should also get system trust level
    let core_path = ModulePath::from_string("core.mem").unwrap();
    let core_ctx = manager.get_or_create_context(&core_path);
    assert_eq!(core_ctx.trust_level, TrustLevel::System);

    // Regular modules should get default trust level (untrusted)
    let user_path = ModulePath::from_string("myapp.module").unwrap();
    let user_ctx = manager.get_or_create_context(&user_path);
    assert_eq!(user_ctx.trust_level, TrustLevel::Untrusted);

    // Add trusted pattern and verify it works
    manager.add_trusted_pattern("mycompany.*".to_string());
    let company_path = ModulePath::from_string("mycompany.utils").unwrap();
    let company_ctx = manager.get_or_create_context(&company_path);
    assert_eq!(company_ctx.trust_level, TrustLevel::Trusted);

    // Test import permission checking
    assert!(manager
        .check_import_permission(&company_path, &std_path)
        .is_ok());
    assert!(manager
        .check_import_permission(&user_path, &company_path)
        .is_ok());

    // Untrusted modules cannot import system modules
    assert!(manager
        .check_import_permission(&user_path, &std_path)
        .is_err());
}

#[test]
fn test_path_pattern_matching() {
    // Exact pattern
    let exact = PathPattern::Exact(PathBuf::from("/tmp/test.txt"));
    assert!(exact.matches(&PathBuf::from("/tmp/test.txt")));
    assert!(!exact.matches(&PathBuf::from("/tmp/other.txt")));
    assert!(!exact.matches(&PathBuf::from("/tmp/test.txt/subdir")));

    // Prefix pattern
    let prefix = PathPattern::Prefix(PathBuf::from("/tmp"));
    assert!(prefix.matches(&PathBuf::from("/tmp")));
    assert!(prefix.matches(&PathBuf::from("/tmp/test.txt")));
    assert!(prefix.matches(&PathBuf::from("/tmp/subdir/file.txt")));
    assert!(!prefix.matches(&PathBuf::from("/var/test.txt")));

    // Glob pattern
    let glob_all = PathPattern::Glob("*".to_string());
    assert!(glob_all.matches(&PathBuf::from("/anything")));
    assert!(glob_all.matches(&PathBuf::from("/tmp/test.txt")));

    let glob_dir = PathPattern::Glob("/tmp/*".to_string());
    assert!(glob_dir.matches(&PathBuf::from("/tmp/test.txt")));
    assert!(glob_dir.matches(&PathBuf::from("/tmp/subdir")));
    assert!(!glob_dir.matches(&PathBuf::from("/var/test.txt")));

    // Any pattern
    let any = PathPattern::Any;
    assert!(any.matches(&PathBuf::from("/anything")));
    assert!(any.matches(&PathBuf::from("/tmp/test.txt")));
}

#[test]
fn test_permission_manager() {
    let manager = PermissionManager::new();
    let module = ModulePath::from_string("test.permissions").unwrap();

    // Should fail without permissions
    let perm = Permission::FileSystem(FileSystemPermission::Read(PathPattern::Any));
    assert!(manager.check_permission(&module, &perm).is_err());

    // Grant permission
    manager.grant_permission(&module, perm.clone()).unwrap();

    // Should succeed now
    assert!(manager.check_permission(&module, &perm).is_ok());

    // Different permission should still fail
    let write_perm = Permission::FileSystem(FileSystemPermission::Write(PathPattern::Any));
    assert!(manager.check_permission(&module, &write_perm).is_err());

    // Revoke permission
    manager.revoke_permission(&module, &perm).unwrap();
    assert!(manager.check_permission(&module, &perm).is_err());

    // Check audit log
    let log = manager.get_audit_log();
    assert!(!log.is_empty());
    // Should have at least 3 entries (2 failed checks, 1 successful check)
    assert!(log.len() >= 3);
}

#[test]
fn test_module_permissions_sets() {
    let trusted = PermissionManager::trusted_permissions();
    let system = PermissionManager::system_permissions();

    // Trusted should have file read, network, but not write everywhere
    assert!(
        trusted.contains(&Permission::FileSystem(FileSystemPermission::Read(
            PathPattern::Any
        )))
    );
    assert!(
        trusted.contains(&Permission::Network(NetworkPermission::Connect(
            HostPattern::Any
        )))
    );
    assert!(
        !trusted.contains(&Permission::FileSystem(FileSystemPermission::Write(
            PathPattern::Any
        )))
    );

    // System should have everything trusted has plus more
    assert!(
        system.contains(&Permission::FileSystem(FileSystemPermission::Read(
            PathPattern::Any
        )))
    );
    assert!(
        system.contains(&Permission::FileSystem(FileSystemPermission::Write(
            PathPattern::Any
        )))
    );
    assert!(
        system.contains(&Permission::FileSystem(FileSystemPermission::Execute(
            PathPattern::Any
        )))
    );
    assert!(system.contains(&Permission::Process(
        script::module::ProcessPermission::Spawn(script::module::ProcessPattern::Any)
    )));
}

#[test]
fn test_sandbox_config() {
    let config = SandboxConfig::default();
    assert_eq!(config.max_execution_time, Duration::from_secs(5));
    assert_eq!(config.max_memory, 50_000_000);
    assert_eq!(config.max_stack_depth, 1000);
    assert!(config.allowed_read_paths.is_empty());
    assert!(config.allowed_write_paths.is_empty());
    assert!(config.allowed_hosts.is_empty());
    assert!(!config.deterministic);
    assert!(!config.trace_execution);

    // Create custom config
    let mut custom_config = SandboxConfig {
        max_execution_time: Duration::from_secs(1),
        max_memory: 1_000_000,
        max_stack_depth: 100,
        allowed_read_paths: vec![PathBuf::from("/tmp")],
        allowed_write_paths: vec![],
        allowed_hosts: vec!["example.com".to_string()],
        deterministic: true,
        trace_execution: true,
    };

    assert_eq!(custom_config.max_execution_time, Duration::from_secs(1));
    assert_eq!(custom_config.max_memory, 1_000_000);
    assert!(custom_config.deterministic);
    assert!(custom_config.trace_execution);
}

#[test]
fn test_sandbox_execution() {
    let module_path = ModulePath::from_string("test.sandbox").unwrap();
    let security_context = ModuleSecurityContext::new(module_path.clone(), TrustLevel::Sandbox);
    let config = SandboxConfig::default();

    let mut sandbox = ModuleSandbox::new(module_path, security_context, config);

    // Attempting to execute a function should fail with "not yet implemented"
    let result = sandbox.execute_function("main", vec![]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("not yet implemented"));

    // Get resource usage (should be minimal)
    let usage = sandbox.get_resource_usage();
    assert_eq!(usage.memory_allocated, 0);
    assert_eq!(usage.syscall_count, 0);

    // Trace should be None if not enabled
    assert!(sandbox.get_trace().is_none());
}

#[test]
fn test_module_compilation_pipeline_security() {
    use tempfile::TempDir;

    // Create temporary directory for test modules
    let temp_dir = TempDir::new().unwrap();
    let package_root = temp_dir.path().to_path_buf();

    // Create a test module file
    let src_dir = package_root.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();

    let module_content = r#"
        export fn add(a: i32, b: i32) -> i32 {
            a + b
        }
    "#;
    std::fs::write(src_dir.join("math.script"), module_content).unwrap();

    // Create pipeline
    let mut pipeline = script::module::create_default_pipeline();

    // Grant capabilities to a module
    let module_path = ModulePath::from_string("math").unwrap();
    assert!(pipeline
        .grant_module_capability(
            &module_path,
            ModuleCapability::FileRead(PathBuf::from("/tmp"))
        )
        .is_ok());

    // Check security info
    let security_info = pipeline.get_module_security_info(&module_path);
    assert!(security_info.is_some());

    // Set module permissions
    let mut permissions = ModulePermissions {
        module: module_path.clone(),
        permissions: std::collections::HashSet::new(),
        inherit_from: None,
        custom_rules: vec![],
    };
    permissions
        .permissions
        .insert(Permission::FileSystem(FileSystemPermission::Read(
            PathPattern::Prefix(PathBuf::from("/tmp")),
        )));

    assert!(pipeline
        .set_module_permissions(&module_path, permissions)
        .is_ok());
}

#[test]
fn test_cross_module_security_checks() {
    let mut security_manager = ModuleSecurityManager::new();

    // Create modules with different trust levels
    let system_module = ModulePath::from_string("std.crypto").unwrap();
    let trusted_module = ModulePath::from_string("mycompany.utils").unwrap();
    let untrusted_module = ModulePath::from_string("external.plugin").unwrap();
    let sandbox_module = ModulePath::from_string("user.script").unwrap();

    // Set up trust patterns
    security_manager.add_trusted_pattern("mycompany.*".to_string());
    security_manager.set_default_trust_level(TrustLevel::Untrusted);

    // Create contexts
    let system_ctx = security_manager.get_or_create_context(&system_module);
    assert_eq!(system_ctx.trust_level, TrustLevel::System);

    let trusted_ctx = security_manager.get_or_create_context(&trusted_module);
    assert_eq!(trusted_ctx.trust_level, TrustLevel::Trusted);

    let untrusted_ctx = security_manager.get_or_create_context(&untrusted_module);
    assert_eq!(untrusted_ctx.trust_level, TrustLevel::Untrusted);

    // Manually set sandbox module to sandbox trust level
    let sandbox_ctx = security_manager.get_or_create_context(&sandbox_module);
    sandbox_ctx.trust_level = TrustLevel::Sandbox;

    // Test import permissions

    // System can import from anywhere
    assert!(security_manager
        .check_import_permission(&system_module, &trusted_module)
        .is_ok());
    assert!(security_manager
        .check_import_permission(&system_module, &untrusted_module)
        .is_ok());

    // Trusted can import from trusted and untrusted, but not sandbox-to-non-sandbox rule doesn't apply
    assert!(security_manager
        .check_import_permission(&trusted_module, &system_module)
        .is_ok());
    assert!(security_manager
        .check_import_permission(&trusted_module, &untrusted_module)
        .is_ok());

    // Untrusted cannot import system modules
    assert!(security_manager
        .check_import_permission(&untrusted_module, &system_module)
        .is_err());
    assert!(security_manager
        .check_import_permission(&untrusted_module, &trusted_module)
        .is_ok());

    // Sandbox cannot import from non-sandbox modules
    assert!(security_manager
        .check_import_permission(&sandbox_module, &system_module)
        .is_err());
    assert!(security_manager
        .check_import_permission(&sandbox_module, &trusted_module)
        .is_err());
}
