//! Module-specific security context and trust management
//! 
//! This module provides security contexts for Script modules, including trust levels,
//! permission management, and signature verification for safe module loading.

use crate::module::{ModulePath, ModuleError};
use std::collections::HashMap;
use std::path::PathBuf;

/// Trust level for a module
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrustLevel {
    /// Core system modules with full privileges
    System,
    /// Trusted modules with standard privileges
    Trusted,
    /// Untrusted modules with restricted privileges
    Untrusted,
    /// Sandboxed modules with minimal privileges
    Sandbox,
}

impl TrustLevel {
    /// Returns true if this trust level allows the given capability
    pub fn allows_capability(&self, capability: &ModuleCapability) -> bool {
        match (self, capability) {
            // System modules can do anything
            (TrustLevel::System, _) => true,
            
            // Trusted modules have most capabilities except system operations
            (TrustLevel::Trusted, ModuleCapability::SystemCall) => false,
            (TrustLevel::Trusted, ModuleCapability::UnsafeCode) => false,
            (TrustLevel::Trusted, _) => true,
            
            // Untrusted modules have limited capabilities
            (TrustLevel::Untrusted, ModuleCapability::FileRead(_)) => true,
            (TrustLevel::Untrusted, ModuleCapability::NetworkConnect(_)) => false,
            (TrustLevel::Untrusted, ModuleCapability::FileWrite(_)) => false,
            (TrustLevel::Untrusted, ModuleCapability::ProcessSpawn) => false,
            (TrustLevel::Untrusted, ModuleCapability::FFICall) => false,
            (TrustLevel::Untrusted, ModuleCapability::SystemCall) => false,
            (TrustLevel::Untrusted, ModuleCapability::UnsafeCode) => false,
            (TrustLevel::Untrusted, ModuleCapability::ResourceAllocation { .. }) => true,
            
            // Sandboxed modules have minimal capabilities
            (TrustLevel::Sandbox, ModuleCapability::ResourceAllocation { cpu_time, memory }) => {
                // Very restricted resources for sandboxed modules
                *cpu_time <= 1000 && *memory <= 10_000_000 // 10MB
            }
            (TrustLevel::Sandbox, _) => false,
        }
    }
    
    /// Get the maximum resource limits for this trust level
    pub fn resource_limits(&self) -> ResourceLimits {
        match self {
            TrustLevel::System => ResourceLimits::unlimited(),
            TrustLevel::Trusted => ResourceLimits {
                max_memory: 1_000_000_000, // 1GB
                max_cpu_time: 60_000,       // 60 seconds
                max_file_handles: 100,
                max_network_connections: 50,
                max_threads: 10,
            },
            TrustLevel::Untrusted => ResourceLimits {
                max_memory: 100_000_000,   // 100MB
                max_cpu_time: 10_000,      // 10 seconds
                max_file_handles: 10,
                max_network_connections: 0,
                max_threads: 1,
            },
            TrustLevel::Sandbox => ResourceLimits {
                max_memory: 10_000_000,    // 10MB
                max_cpu_time: 1_000,       // 1 second
                max_file_handles: 0,
                max_network_connections: 0,
                max_threads: 0,
            },
        }
    }
}

/// Capabilities that a module might request
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ModuleCapability {
    /// Read access to specific file paths
    FileRead(PathBuf),
    /// Write access to specific file paths
    FileWrite(PathBuf),
    /// Network connection to specific hosts
    NetworkConnect(String),
    /// Ability to spawn processes
    ProcessSpawn,
    /// Ability to call FFI functions
    FFICall,
    /// Ability to make system calls
    SystemCall,
    /// Ability to execute unsafe code
    UnsafeCode,
    /// Resource allocation limits
    ResourceAllocation { cpu_time: u64, memory: usize },
}

/// Resource limits for a module
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_memory: usize,
    pub max_cpu_time: u64,
    pub max_file_handles: usize,
    pub max_network_connections: usize,
    pub max_threads: usize,
}

impl ResourceLimits {
    /// Create unlimited resource limits (for system modules)
    pub fn unlimited() -> Self {
        ResourceLimits {
            max_memory: usize::MAX,
            max_cpu_time: u64::MAX,
            max_file_handles: usize::MAX,
            max_network_connections: usize::MAX,
            max_threads: usize::MAX,
        }
    }
}

/// Security context for a module
#[derive(Debug, Clone)]
pub struct ModuleSecurityContext {
    /// Module path
    pub module_path: ModulePath,
    /// Trust level
    pub trust_level: TrustLevel,
    /// Granted capabilities
    pub capabilities: Vec<ModuleCapability>,
    /// Resource limits
    pub resource_limits: ResourceLimits,
    /// Module signature (for verification)
    pub signature: Option<ModuleSignature>,
    /// Parent module that loaded this module
    pub parent_module: Option<ModulePath>,
}

impl ModuleSecurityContext {
    /// Create a new security context for a module
    pub fn new(module_path: ModulePath, trust_level: TrustLevel) -> Self {
        let resource_limits = trust_level.resource_limits();
        ModuleSecurityContext {
            module_path,
            trust_level,
            capabilities: Vec::new(),
            resource_limits,
            signature: None,
            parent_module: None,
        }
    }
    
    /// Check if a capability is allowed
    pub fn check_capability(&self, capability: &ModuleCapability) -> Result<(), ModuleError> {
        // First check trust level
        if !self.trust_level.allows_capability(capability) {
            return Err(ModuleError::security_violation(format!(
                "Module {} with trust level {:?} cannot use capability {:?}",
                self.module_path, self.trust_level, capability
            )));
        }
        
        // Then check explicit capabilities
        if self.capabilities.contains(capability) {
            Ok(())
        } else {
            Err(ModuleError::security_violation(format!(
                "Module {} does not have capability {:?}",
                self.module_path, capability
            )))
        }
    }
    
    /// Add a capability to this context
    pub fn grant_capability(&mut self, capability: ModuleCapability) -> Result<(), ModuleError> {
        // Check if trust level allows this capability
        if !self.trust_level.allows_capability(&capability) {
            return Err(ModuleError::security_violation(format!(
                "Cannot grant capability {:?} to module with trust level {:?}",
                capability, self.trust_level
            )));
        }
        
        if !self.capabilities.contains(&capability) {
            self.capabilities.push(capability);
        }
        Ok(())
    }
}

/// Module signature for verification
#[derive(Debug, Clone)]
pub struct ModuleSignature {
    /// Cryptographic hash of the module content
    pub content_hash: Vec<u8>,
    /// Digital signature
    pub signature: Vec<u8>,
    /// Public key ID used for signing
    pub key_id: String,
}

/// Module security manager
pub struct ModuleSecurityManager {
    /// Security contexts for loaded modules
    pub contexts: HashMap<ModulePath, ModuleSecurityContext>,
    /// Default trust level for new modules
    default_trust_level: TrustLevel,
    /// Trusted module patterns
    trusted_patterns: Vec<String>,
    /// System module patterns
    system_patterns: Vec<String>,
}

impl ModuleSecurityManager {
    /// Create a new module security manager
    pub fn new() -> Self {
        ModuleSecurityManager {
            contexts: HashMap::new(),
            default_trust_level: TrustLevel::Untrusted,
            trusted_patterns: vec![],
            system_patterns: vec![
                "std.*".to_string(),
                "core.*".to_string(),
            ],
        }
    }
    
    /// Get or create security context for a module
    pub fn get_or_create_context(&mut self, module_path: &ModulePath) -> &mut ModuleSecurityContext {
        let trust_level = self.determine_trust_level(module_path);
        self.contexts.entry(module_path.clone())
            .or_insert_with(|| ModuleSecurityContext::new(module_path.clone(), trust_level))
    }
    
    /// Determine trust level for a module based on patterns
    fn determine_trust_level(&self, module_path: &ModulePath) -> TrustLevel {
        let path_str = module_path.to_string();
        
        // Check system patterns
        for pattern in &self.system_patterns {
            if Self::matches_pattern(&path_str, pattern) {
                return TrustLevel::System;
            }
        }
        
        // Check trusted patterns
        for pattern in &self.trusted_patterns {
            if Self::matches_pattern(&path_str, pattern) {
                return TrustLevel::Trusted;
            }
        }
        
        self.default_trust_level
    }
    
    /// Simple pattern matching (supports * wildcard)
    fn matches_pattern(path: &str, pattern: &str) -> bool {
        if pattern.ends_with('*') {
            let prefix = &pattern[..pattern.len() - 1];
            path.starts_with(prefix)
        } else {
            path == pattern
        }
    }
    
    /// Add a trusted module pattern
    pub fn add_trusted_pattern(&mut self, pattern: String) {
        self.trusted_patterns.push(pattern);
    }
    
    /// Add a system module pattern
    pub fn add_system_pattern(&mut self, pattern: String) {
        self.system_patterns.push(pattern);
    }
    
    /// Set default trust level for new modules
    pub fn set_default_trust_level(&mut self, level: TrustLevel) {
        self.default_trust_level = level;
    }
    
    /// Check if a module can import from another module
    pub fn check_import_permission(
        &self,
        importer: &ModulePath,
        imported: &ModulePath,
    ) -> Result<(), ModuleError> {
        let importer_ctx = self.contexts.get(importer)
            .ok_or_else(|| ModuleError::security_violation(
                format!("No security context for importing module {}", importer)
            ))?;
        
        let imported_ctx = self.contexts.get(imported)
            .ok_or_else(|| ModuleError::security_violation(
                format!("No security context for imported module {}", imported)
            ))?;
        
        // Sandbox modules cannot import from outside their sandbox
        if importer_ctx.trust_level == TrustLevel::Sandbox {
            if imported_ctx.trust_level != TrustLevel::Sandbox {
                return Err(ModuleError::security_violation(
                    format!("Sandboxed module {} cannot import from non-sandboxed module {}", 
                        importer, imported)
                ));
            }
        }
        
        // Untrusted modules cannot import system modules
        if importer_ctx.trust_level == TrustLevel::Untrusted 
            && imported_ctx.trust_level == TrustLevel::System {
            return Err(ModuleError::security_violation(
                format!("Untrusted module {} cannot import system module {}", 
                    importer, imported)
            ));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_trust_level_capabilities() {
        // System level allows everything
        assert!(TrustLevel::System.allows_capability(&ModuleCapability::SystemCall));
        assert!(TrustLevel::System.allows_capability(&ModuleCapability::UnsafeCode));
        
        // Trusted level allows most things
        assert!(TrustLevel::Trusted.allows_capability(&ModuleCapability::FileRead(PathBuf::new())));
        assert!(TrustLevel::Trusted.allows_capability(&ModuleCapability::NetworkConnect("example.com".to_string())));
        assert!(!TrustLevel::Trusted.allows_capability(&ModuleCapability::SystemCall));
        
        // Untrusted level is restricted
        assert!(TrustLevel::Untrusted.allows_capability(&ModuleCapability::FileRead(PathBuf::new())));
        assert!(!TrustLevel::Untrusted.allows_capability(&ModuleCapability::FileWrite(PathBuf::new())));
        assert!(!TrustLevel::Untrusted.allows_capability(&ModuleCapability::NetworkConnect("example.com".to_string())));
        
        // Sandbox level is highly restricted
        assert!(!TrustLevel::Sandbox.allows_capability(&ModuleCapability::FileRead(PathBuf::new())));
        assert!(TrustLevel::Sandbox.allows_capability(&ModuleCapability::ResourceAllocation { 
            cpu_time: 500, 
            memory: 5_000_000 
        }));
    }
    
    #[test]
    fn test_module_security_context() {
        let module_path = ModulePath::from_string("test.module").unwrap();
        let mut ctx = ModuleSecurityContext::new(module_path.clone(), TrustLevel::Trusted);
        
        // Can grant allowed capabilities
        assert!(ctx.grant_capability(ModuleCapability::FileRead(PathBuf::new())).is_ok());
        assert!(ctx.grant_capability(ModuleCapability::NetworkConnect("example.com".to_string())).is_ok());
        
        // Cannot grant disallowed capabilities
        assert!(ctx.grant_capability(ModuleCapability::SystemCall).is_err());
        
        // Check capability works
        assert!(ctx.check_capability(&ModuleCapability::FileRead(PathBuf::new())).is_ok());
        assert!(ctx.check_capability(&ModuleCapability::ProcessSpawn).is_err());
    }
    
    #[test]
    fn test_module_security_manager() {
        let mut manager = ModuleSecurityManager::new();
        
        // System modules get system trust level
        let std_path = ModulePath::from_string("std.io").unwrap();
        let std_ctx = manager.get_or_create_context(&std_path);
        assert_eq!(std_ctx.trust_level, TrustLevel::System);
        
        // Regular modules get default trust level
        let user_path = ModulePath::from_string("myapp.module").unwrap();
        let user_ctx = manager.get_or_create_context(&user_path);
        assert_eq!(user_ctx.trust_level, TrustLevel::Untrusted);
        
        // Add trusted pattern
        manager.add_trusted_pattern("mycompany.*".to_string());
        let company_path = ModulePath::from_string("mycompany.utils").unwrap();
        let company_ctx = manager.get_or_create_context(&company_path);
        assert_eq!(company_ctx.trust_level, TrustLevel::Trusted);
    }
}