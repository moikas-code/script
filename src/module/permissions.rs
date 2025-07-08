//! Fine-grained permission system for module operations
//! 
//! This module provides a capability-based permission system that controls
//! what operations modules can perform, including file system access,
//! network operations, FFI calls, and resource allocation.

use crate::module::{ModulePath, ModuleError};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Permission set for a module
#[derive(Debug, Clone)]
pub struct ModulePermissions {
    /// Module identifier
    pub module: ModulePath,
    /// Granted permissions
    pub permissions: HashSet<Permission>,
    /// Permission inheritance from parent modules
    pub inherit_from: Option<ModulePath>,
    /// Custom permission rules
    pub custom_rules: Vec<PermissionRule>,
}

/// Individual permission types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Permission {
    /// File system permissions
    FileSystem(FileSystemPermission),
    /// Network permissions
    Network(NetworkPermission),
    /// Process permissions
    Process(ProcessPermission),
    /// FFI permissions
    FFI(FFIPermission),
    /// Resource permissions
    Resource(ResourcePermission),
    /// Module interaction permissions
    ModuleInteraction(ModuleInteractionPermission),
}

/// File system permission types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FileSystemPermission {
    /// Read files from specific paths
    Read(PathPattern),
    /// Write files to specific paths
    Write(PathPattern),
    /// Create directories
    CreateDirectory(PathPattern),
    /// Delete files or directories
    Delete(PathPattern),
    /// Execute files
    Execute(PathPattern),
    /// List directory contents
    List(PathPattern),
}

/// Path pattern for file system permissions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathPattern {
    /// Exact path
    Exact(PathBuf),
    /// Path prefix (directory and subdirectories)
    Prefix(PathBuf),
    /// Glob pattern
    Glob(String),
    /// Any path (unrestricted)
    Any,
}

impl PathPattern {
    /// Check if a path matches this pattern
    pub fn matches(&self, path: &Path) -> bool {
        match self {
            PathPattern::Exact(exact) => path == exact,
            PathPattern::Prefix(prefix) => path.starts_with(prefix),
            PathPattern::Glob(pattern) => {
                // Simple glob matching (in real implementation, use glob crate)
                if pattern == "*" {
                    true
                } else if pattern.ends_with("/*") {
                    let prefix = &pattern[..pattern.len() - 2];
                    path.to_string_lossy().starts_with(prefix)
                } else {
                    path.to_string_lossy().as_ref() == pattern
                }
            }
            PathPattern::Any => true,
        }
    }
}

/// Network permission types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NetworkPermission {
    /// Connect to specific hosts
    Connect(HostPattern),
    /// Listen on specific ports
    Listen(PortRange),
    /// DNS resolution
    DNSResolve,
    /// Raw socket access
    RawSocket,
}

/// Host pattern for network permissions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HostPattern {
    /// Exact hostname or IP
    Exact(String),
    /// Domain and subdomains
    Domain(String),
    /// IP range (CIDR notation)
    IPRange(String),
    /// Any host
    Any,
}

/// Port range for network permissions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

/// Process permission types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProcessPermission {
    /// Spawn new processes
    Spawn(ProcessPattern),
    /// Send signals to processes
    Signal,
    /// Access environment variables
    Environment(EnvVarPattern),
    /// Change working directory
    ChangeDirectory,
}

/// Process pattern for spawn permissions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProcessPattern {
    /// Specific executable
    Exact(PathBuf),
    /// Any executable in PATH
    InPath(String),
    /// Any process
    Any,
}

/// Environment variable pattern
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EnvVarPattern {
    /// Specific variable
    Exact(String),
    /// Variable prefix
    Prefix(String),
    /// Any variable
    Any,
}

/// FFI permission types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FFIPermission {
    /// Call functions from specific libraries
    Call(LibraryPattern),
    /// Load dynamic libraries
    LoadLibrary(PathPattern),
    /// Access raw memory
    RawMemory,
    /// Unsafe operations
    Unsafe,
}

/// Library pattern for FFI permissions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LibraryPattern {
    /// Specific library
    Exact(String),
    /// Library prefix
    Prefix(String),
    /// System libraries only
    SystemOnly,
    /// Any library
    Any,
}

/// Resource permission types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResourcePermission {
    /// Memory allocation limit
    Memory(usize),
    /// CPU time limit (milliseconds)
    CPUTime(u64),
    /// Thread creation
    Threads(usize),
    /// File handle limit
    FileHandles(usize),
}

/// Module interaction permission types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ModuleInteractionPermission {
    /// Import from specific modules
    Import(ModulePattern),
    /// Export to specific modules
    Export(ModulePattern),
    /// Reflection/introspection
    Reflection,
    /// Dynamic code loading
    DynamicLoad,
}

/// Module pattern for interaction permissions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ModulePattern {
    /// Exact module path
    Exact(ModulePath),
    /// Module prefix
    Prefix(String),
    /// Modules matching regex
    Regex(String),
    /// Any module
    Any,
}

/// Custom permission rule for complex logic
#[derive(Debug, Clone)]
pub struct PermissionRule {
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: String,
    /// Evaluation function
    pub evaluator: Arc<dyn Fn(&PermissionContext) -> bool + Send + Sync>,
}

/// Context for permission evaluation
#[derive(Debug)]
pub struct PermissionContext {
    /// Module requesting permission
    pub module: ModulePath,
    /// Operation being performed
    pub operation: String,
    /// Operation arguments
    pub args: HashMap<String, String>,
    /// Current time
    pub timestamp: std::time::SystemTime,
}

/// Permission manager for the module system
pub struct PermissionManager {
    /// Permissions by module
    permissions: Arc<RwLock<HashMap<ModulePath, ModulePermissions>>>,
    /// Default permissions for new modules
    default_permissions: HashSet<Permission>,
    /// Permission audit log
    audit_log: Arc<RwLock<Vec<PermissionAuditEntry>>>,
}

/// Audit log entry for permission checks
#[derive(Debug, Clone)]
pub struct PermissionAuditEntry {
    pub timestamp: std::time::SystemTime,
    pub module: ModulePath,
    pub permission: Permission,
    pub granted: bool,
    pub reason: Option<String>,
}

impl PermissionManager {
    /// Create a new permission manager
    pub fn new() -> Self {
        PermissionManager {
            permissions: Arc::new(RwLock::new(HashMap::new())),
            default_permissions: Self::minimal_permissions(),
            audit_log: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Get minimal safe permissions
    fn minimal_permissions() -> HashSet<Permission> {
        let mut perms = HashSet::new();
        // Allow basic memory allocation
        perms.insert(Permission::Resource(ResourcePermission::Memory(10_000_000))); // 10MB
        // Allow limited CPU time
        perms.insert(Permission::Resource(ResourcePermission::CPUTime(5000))); // 5 seconds
        perms
    }
    
    /// Register a module with permissions
    pub fn register_module(&self, permissions: ModulePermissions) {
        let mut perms = self.permissions.write().unwrap();
        perms.insert(permissions.module.clone(), permissions);
    }
    
    /// Check if a module has a specific permission
    pub fn check_permission(
        &self,
        module: &ModulePath,
        permission: &Permission,
    ) -> Result<(), ModuleError> {
        let perms = self.permissions.read().unwrap();
        
        // Get module permissions, applying inheritance if needed
        let module_perms = self.get_effective_permissions(module, &perms);
        
        // Check if permission is granted
        let granted = module_perms.contains(permission) || 
                     self.check_custom_rules(module, permission);
        
        // Log the check
        self.log_permission_check(module, permission, granted);
        
        if granted {
            Ok(())
        } else {
            Err(ModuleError::security_violation(format!(
                "Module {} lacks permission {:?}",
                module, permission
            )))
        }
    }
    
    /// Get effective permissions including inheritance
    fn get_effective_permissions(
        &self,
        module: &ModulePath,
        perms: &HashMap<ModulePath, ModulePermissions>,
    ) -> HashSet<Permission> {
        let mut effective = self.default_permissions.clone();
        
        // Walk up the inheritance chain
        let mut current = Some(module.clone());
        while let Some(mod_path) = current {
            if let Some(mod_perms) = perms.get(&mod_path) {
                effective.extend(mod_perms.permissions.iter().cloned());
                current = mod_perms.inherit_from.clone();
            } else {
                break;
            }
        }
        
        effective
    }
    
    /// Check custom permission rules
    fn check_custom_rules(&self, module: &ModulePath, permission: &Permission) -> bool {
        let perms = self.permissions.read().unwrap();
        
        if let Some(mod_perms) = perms.get(module) {
            let context = PermissionContext {
                module: module.clone(),
                operation: format!("{:?}", permission),
                args: HashMap::new(),
                timestamp: std::time::SystemTime::now(),
            };
            
            for rule in &mod_perms.custom_rules {
                if (rule.evaluator)(&context) {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Log a permission check
    fn log_permission_check(&self, module: &ModulePath, permission: &Permission, granted: bool) {
        let entry = PermissionAuditEntry {
            timestamp: std::time::SystemTime::now(),
            module: module.clone(),
            permission: permission.clone(),
            granted,
            reason: None,
        };
        
        let mut log = self.audit_log.write().unwrap();
        log.push(entry);
        
        // Keep only recent entries (e.g., last 10000)
        if log.len() > 10000 {
            log.drain(0..1000);
        }
    }
    
    /// Grant a permission to a module
    pub fn grant_permission(
        &self,
        module: &ModulePath,
        permission: Permission,
    ) -> Result<(), ModuleError> {
        let mut perms = self.permissions.write().unwrap();
        
        let mod_perms = perms.entry(module.clone())
            .or_insert_with(|| ModulePermissions {
                module: module.clone(),
                permissions: HashSet::new(),
                inherit_from: None,
                custom_rules: Vec::new(),
            });
        
        mod_perms.permissions.insert(permission);
        Ok(())
    }
    
    /// Revoke a permission from a module
    pub fn revoke_permission(
        &self,
        module: &ModulePath,
        permission: &Permission,
    ) -> Result<(), ModuleError> {
        let mut perms = self.permissions.write().unwrap();
        
        if let Some(mod_perms) = perms.get_mut(module) {
            mod_perms.permissions.remove(permission);
        }
        
        Ok(())
    }
    
    /// Get audit log entries
    pub fn get_audit_log(&self) -> Vec<PermissionAuditEntry> {
        self.audit_log.read().unwrap().clone()
    }
    
    /// Create a permission set for trusted modules
    pub fn trusted_permissions() -> HashSet<Permission> {
        let mut perms = HashSet::new();
        
        // File system
        perms.insert(Permission::FileSystem(FileSystemPermission::Read(PathPattern::Any)));
        perms.insert(Permission::FileSystem(FileSystemPermission::Write(PathPattern::Prefix(PathBuf::from("/tmp")))));
        
        // Network
        perms.insert(Permission::Network(NetworkPermission::Connect(HostPattern::Any)));
        perms.insert(Permission::Network(NetworkPermission::DNSResolve));
        
        // Resources
        perms.insert(Permission::Resource(ResourcePermission::Memory(1_000_000_000))); // 1GB
        perms.insert(Permission::Resource(ResourcePermission::CPUTime(60_000))); // 60s
        perms.insert(Permission::Resource(ResourcePermission::Threads(10)));
        
        // Module interaction
        perms.insert(Permission::ModuleInteraction(ModuleInteractionPermission::Import(ModulePattern::Any)));
        
        perms
    }
    
    /// Create a permission set for system modules
    pub fn system_permissions() -> HashSet<Permission> {
        let mut perms = Self::trusted_permissions();
        
        // Additional system permissions
        perms.insert(Permission::FileSystem(FileSystemPermission::Write(PathPattern::Any)));
        perms.insert(Permission::FileSystem(FileSystemPermission::Delete(PathPattern::Any)));
        perms.insert(Permission::FileSystem(FileSystemPermission::Execute(PathPattern::Any)));
        
        perms.insert(Permission::Process(ProcessPermission::Spawn(ProcessPattern::Any)));
        perms.insert(Permission::Process(ProcessPermission::Signal));
        
        perms.insert(Permission::FFI(FFIPermission::Call(LibraryPattern::Any)));
        perms.insert(Permission::FFI(FFIPermission::LoadLibrary(PathPattern::Any)));
        perms.insert(Permission::FFI(FFIPermission::Unsafe));
        
        perms.insert(Permission::ModuleInteraction(ModuleInteractionPermission::Reflection));
        perms.insert(Permission::ModuleInteraction(ModuleInteractionPermission::DynamicLoad));
        
        perms
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_path_pattern_matching() {
        let exact = PathPattern::Exact(PathBuf::from("/tmp/test.txt"));
        assert!(exact.matches(Path::new("/tmp/test.txt")));
        assert!(!exact.matches(Path::new("/tmp/other.txt")));
        
        let prefix = PathPattern::Prefix(PathBuf::from("/tmp"));
        assert!(prefix.matches(Path::new("/tmp/test.txt")));
        assert!(prefix.matches(Path::new("/tmp/subdir/file.txt")));
        assert!(!prefix.matches(Path::new("/var/test.txt")));
        
        let glob = PathPattern::Glob("/tmp/*".to_string());
        assert!(glob.matches(Path::new("/tmp/test.txt")));
        assert!(glob.matches(Path::new("/tmp/subdir")));
    }
    
    #[test]
    fn test_permission_manager() {
        let manager = PermissionManager::new();
        let module = ModulePath::from_string("test.module").unwrap();
        
        // Should fail without permissions
        let perm = Permission::FileSystem(FileSystemPermission::Read(PathPattern::Any));
        assert!(manager.check_permission(&module, &perm).is_err());
        
        // Grant permission
        manager.grant_permission(&module, perm.clone()).unwrap();
        
        // Should succeed now
        assert!(manager.check_permission(&module, &perm).is_ok());
        
        // Check audit log
        let log = manager.get_audit_log();
        assert!(!log.is_empty());
    }
}