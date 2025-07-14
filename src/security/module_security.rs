//! Module-aware security manager integration
//!
//! This module integrates module-specific security policies with the main
//! security framework, providing per-module resource limits and isolation.

use crate::error::{Error, ErrorKind};
use crate::module::{ModuleCapability, ModulePath, ModuleSecurityContext, TrustLevel};
use crate::security::{SecurityPolicy, SecurityViolation};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Per-module resource tracking
#[derive(Debug, Clone)]
pub struct ModuleResourceUsage {
    /// Memory allocated by this module (bytes)
    pub memory_allocated: usize,
    /// CPU time used (milliseconds)
    pub cpu_time_ms: u64,
    /// Number of open file handles
    pub file_handles: usize,
    /// Number of active network connections
    pub network_connections: usize,
    /// Number of threads spawned
    pub threads_spawned: usize,
    /// Last activity timestamp
    pub last_activity: Instant,
}

impl ModuleResourceUsage {
    pub fn new() -> Self {
        ModuleResourceUsage {
            memory_allocated: 0,
            cpu_time_ms: 0,
            file_handles: 0,
            network_connections: 0,
            threads_spawned: 0,
            last_activity: Instant::now(),
        }
    }
}

/// Module isolation boundary
#[derive(Debug, Clone)]
pub struct ModuleIsolationBoundary {
    /// Module path
    pub module: ModulePath,
    /// Allowed imports
    pub allowed_imports: Vec<ModulePath>,
    /// Denied imports (blacklist)
    pub denied_imports: Vec<ModulePath>,
    /// Maximum call stack depth when calling into this module
    pub max_call_depth: usize,
    /// Whether this module can be called from untrusted code
    pub callable_from_untrusted: bool,
}

/// Module security policy extension
pub struct ModuleSecurityPolicy {
    /// Base security policy
    base_policy: SecurityPolicy,
    /// Per-module policies
    module_policies: HashMap<ModulePath, SecurityPolicy>,
    /// Module resource usage tracking
    resource_usage: Arc<Mutex<HashMap<ModulePath, ModuleResourceUsage>>>,
    /// Module isolation boundaries
    isolation_boundaries: HashMap<ModulePath, ModuleIsolationBoundary>,
    /// Current call stack for cross-module tracking
    call_stack: Arc<Mutex<Vec<ModulePath>>>,
}

impl ModuleSecurityPolicy {
    /// Create a new module security policy
    pub fn new(base_policy: SecurityPolicy) -> Self {
        ModuleSecurityPolicy {
            base_policy,
            module_policies: HashMap::new(),
            resource_usage: Arc::new(Mutex::new(HashMap::new())),
            isolation_boundaries: HashMap::new(),
            call_stack: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Set policy for a specific module
    pub fn set_module_policy(&mut self, module: ModulePath, policy: SecurityPolicy) {
        self.module_policies.insert(module, policy);
    }

    /// Get effective policy for a module
    pub fn get_effective_policy(&self, module: &ModulePath) -> &SecurityPolicy {
        self.module_policies
            .get(module)
            .unwrap_or(&self.base_policy)
    }

    /// Check resource limits for a module
    pub fn check_resource_limits(
        &self,
        module: &ModulePath,
        context: &ModuleSecurityContext,
    ) -> Result<(), SecurityViolation> {
        let usage = self
            .resource_usage
            .lock()
            .map_err(|_| SecurityViolation::InternalError {
                message: "Failed to acquire lock on resource usage".to_string(),
            })?;
        if let Some(module_usage) = usage.get(module) {
            let limits = &context.resource_limits;

            if module_usage.memory_allocated > limits.max_memory {
                return Err(SecurityViolation::ResourceLimitExceeded {
                    resource: "memory".to_string(),
                    limit: limits.max_memory,
                    used: module_usage.memory_allocated,
                });
            }

            if module_usage.cpu_time_ms > limits.max_cpu_time {
                return Err(SecurityViolation::ResourceLimitExceeded {
                    resource: "cpu_time".to_string(),
                    limit: limits.max_cpu_time as usize,
                    used: module_usage.cpu_time_ms as usize,
                });
            }

            if module_usage.file_handles > limits.max_file_handles {
                return Err(SecurityViolation::ResourceLimitExceeded {
                    resource: "file_handles".to_string(),
                    limit: limits.max_file_handles,
                    used: module_usage.file_handles,
                });
            }

            if module_usage.network_connections > limits.max_network_connections {
                return Err(SecurityViolation::ResourceLimitExceeded {
                    resource: "network_connections".to_string(),
                    limit: limits.max_network_connections,
                    used: module_usage.network_connections,
                });
            }

            if module_usage.threads_spawned > limits.max_threads {
                return Err(SecurityViolation::ResourceLimitExceeded {
                    resource: "threads".to_string(),
                    limit: limits.max_threads,
                    used: module_usage.threads_spawned,
                });
            }
        }

        Ok(())
    }

    /// Track resource allocation
    pub fn track_allocation(
        &self,
        module: &ModulePath,
        resource: &str,
        amount: usize,
    ) -> Result<(), SecurityViolation> {
        let mut usage =
            self.resource_usage
                .lock()
                .map_err(|_| SecurityViolation::InternalError {
                    message: "Failed to acquire lock on resource usage".to_string(),
                })?;
        let module_usage = usage
            .entry(module.clone())
            .or_insert_with(ModuleResourceUsage::new);

        match resource {
            "memory" => module_usage.memory_allocated += amount,
            "file_handle" => module_usage.file_handles += 1,
            "network_connection" => module_usage.network_connections += 1,
            "thread" => module_usage.threads_spawned += 1,
            _ => {}
        }

        module_usage.last_activity = Instant::now();
        Ok(())
    }

    /// Track resource deallocation
    pub fn track_deallocation(&self, module: &ModulePath, resource: &str, amount: usize) {
        let mut usage = match self.resource_usage.lock() {
            Ok(usage) => usage,
            Err(_) => return, // Silently fail on lock error for cleanup operation
        };
        if let Some(module_usage) = usage.get_mut(module) {
            match resource {
                "memory" => {
                    module_usage.memory_allocated =
                        module_usage.memory_allocated.saturating_sub(amount)
                }
                "file_handle" => {
                    module_usage.file_handles = module_usage.file_handles.saturating_sub(1)
                }
                "network_connection" => {
                    module_usage.network_connections =
                        module_usage.network_connections.saturating_sub(1)
                }
                "thread" => {
                    module_usage.threads_spawned = module_usage.threads_spawned.saturating_sub(1)
                }
                _ => {}
            }
            module_usage.last_activity = Instant::now();
        }
    }

    /// Check cross-module call permission
    pub fn check_cross_module_call(
        &self,
        caller: &ModulePath,
        callee: &ModulePath,
        caller_trust: TrustLevel,
    ) -> Result<(), SecurityViolation> {
        // Check isolation boundary
        if let Some(boundary) = self.isolation_boundaries.get(callee) {
            // Check if caller is explicitly denied
            if boundary.denied_imports.contains(caller) {
                return Err(SecurityViolation::CrossModuleViolation {
                    caller: caller.to_string(),
                    callee: callee.to_string(),
                    reason: "explicitly denied access".to_string(),
                });
            }

            // Check if module can be called from untrusted code
            if !boundary.callable_from_untrusted && caller_trust == TrustLevel::Untrusted {
                return Err(SecurityViolation::CrossModuleViolation {
                    caller: caller.to_string(),
                    callee: callee.to_string(),
                    reason: "module not callable from untrusted code".to_string(),
                });
            }

            // Check call stack depth
            let call_stack =
                self.call_stack
                    .lock()
                    .map_err(|_| SecurityViolation::InternalError {
                        message: "Failed to acquire lock on call stack".to_string(),
                    })?;
            let depth = call_stack.iter().filter(|m| *m == callee).count();
            if depth >= boundary.max_call_depth {
                return Err(SecurityViolation::CrossModuleViolation {
                    caller: caller.to_string(),
                    callee: callee.to_string(),
                    reason: format!("maximum call depth {} exceeded", boundary.max_call_depth),
                });
            }
        }

        Ok(())
    }

    /// Push module onto call stack
    pub fn push_call_stack(&self, module: ModulePath) {
        let mut stack = match self.call_stack.lock() {
            Ok(stack) => stack,
            Err(_) => return, // Silently fail on lock error
        };
        stack.push(module);
    }

    /// Pop module from call stack
    pub fn pop_call_stack(&self) {
        let mut stack = match self.call_stack.lock() {
            Ok(stack) => stack,
            Err(_) => return, // Silently fail on lock error
        };
        stack.pop();
    }

    /// Set isolation boundary for a module
    pub fn set_isolation_boundary(&mut self, boundary: ModuleIsolationBoundary) {
        self.isolation_boundaries
            .insert(boundary.module.clone(), boundary);
    }

    /// Get current resource usage for a module
    pub fn get_resource_usage(&self, module: &ModulePath) -> Option<ModuleResourceUsage> {
        let usage = self.resource_usage.lock().ok()?;
        usage.get(module).cloned()
    }

    /// Clean up inactive modules (garbage collection)
    pub fn cleanup_inactive_modules(&self, inactive_threshold: Duration) {
        let mut usage = match self.resource_usage.lock() {
            Ok(usage) => usage,
            Err(_) => return, // Silently fail on lock error for cleanup operation
        };
        let now = Instant::now();

        usage.retain(|_, module_usage| {
            let inactive_duration = now.duration_since(module_usage.last_activity);
            inactive_duration < inactive_threshold
        });
    }
}

/// Module security enforcer - integrates with SecurityManager
pub struct ModuleSecurityEnforcer {
    /// Module security policy
    policy: ModuleSecurityPolicy,
    /// Module security contexts
    contexts: HashMap<ModulePath, ModuleSecurityContext>,
}

impl ModuleSecurityEnforcer {
    pub fn new(base_policy: SecurityPolicy) -> Self {
        ModuleSecurityEnforcer {
            policy: ModuleSecurityPolicy::new(base_policy),
            contexts: HashMap::new(),
        }
    }

    /// Register a module with its security context
    pub fn register_module(&mut self, context: ModuleSecurityContext) {
        // Set up module-specific policy based on trust level
        let module_policy = match context.trust_level {
            TrustLevel::System => SecurityPolicy::permissive(),
            TrustLevel::Trusted => SecurityPolicy::default(),
            TrustLevel::Untrusted => SecurityPolicy::restrictive(),
            TrustLevel::Sandbox => SecurityPolicy::strict(),
            TrustLevel::Unknown => SecurityPolicy::strict(), // Conservative default
        };

        self.policy
            .set_module_policy(context.module_path.clone(), module_policy);
        self.contexts.insert(context.module_path.clone(), context);
    }

    /// Enforce security for a module operation
    pub fn enforce_operation(
        &self,
        module: &ModulePath,
        operation: &str,
        args: &[&str],
    ) -> Result<(), Error> {
        let context = self.contexts.get(module).ok_or_else(|| {
            Error::new(
                ErrorKind::SecurityViolation,
                format!("No security context for module {module}"),
            )
        })?;

        // Check resource limits
        self.policy
            .check_resource_limits(module, context)
            .map_err(|_violation| {
                Error::new(
                    ErrorKind::SecurityViolation,
                    format!("Resource limit exceeded for module {module}"),
                )
            })?;

        // Check operation-specific permissions
        match operation {
            "file_read" => {
                if let Some(path) = args.get(0) {
                    let capability = ModuleCapability::FileRead(path.into());
                    context.check_capability(&capability).map_err(|_e| {
                        Error::new(
                            ErrorKind::ModuleError,
                            format!("File read permission denied for module {module}"),
                        )
                    })?;
                }
            }
            "file_write" => {
                if let Some(path) = args.get(0) {
                    let capability = ModuleCapability::FileWrite(path.into());
                    context.check_capability(&capability).map_err(|_e| {
                        Error::new(
                            ErrorKind::ModuleError,
                            format!("File write permission denied for module {module}"),
                        )
                    })?;
                }
            }
            "network_connect" => {
                if let Some(host) = args.get(0) {
                    let capability = ModuleCapability::NetworkConnect(host.to_string());
                    context.check_capability(&capability).map_err(|_e| {
                        Error::new(
                            ErrorKind::ModuleError,
                            format!("Network connection denied for module {module}"),
                        )
                    })?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Check if a cross-module call is allowed
    pub fn check_module_call(&self, caller: &ModulePath, callee: &ModulePath) -> Result<(), Error> {
        let caller_context = self.contexts.get(caller).ok_or_else(|| {
            Error::new(
                ErrorKind::ModuleError,
                format!("No security context for calling module {caller}"),
            )
        })?;

        self.policy
            .check_cross_module_call(caller, callee, caller_context.trust_level)
            .map_err(|_violation| {
                Error::new(
                    ErrorKind::SecurityViolation,
                    format!("Cross-module call denied from {} to {caller, callee}"),
                )
            })?;

        Ok(())
    }

    /// Track entering a module
    pub fn enter_module(&self, module: ModulePath) {
        self.policy.push_call_stack(module);
    }

    /// Track leaving a module
    pub fn leave_module(&self) {
        self.policy.pop_call_stack();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_resource_tracking() {
        let policy = ModuleSecurityPolicy::new(SecurityPolicy::default());
        let module = ModulePath::from_string("test.module").unwrap();

        // Track some allocations
        policy.track_allocation(&module, "memory", 1000).unwrap();
        policy.track_allocation(&module, "file_handle", 1).unwrap();

        // Check usage
        let usage = policy.get_resource_usage(&module).unwrap();
        assert_eq!(usage.memory_allocated, 1000);
        assert_eq!(usage.file_handles, 1);

        // Track deallocation
        policy.track_deallocation(&module, "memory", 500);
        let usage = policy.get_resource_usage(&module).unwrap();
        assert_eq!(usage.memory_allocated, 500);
    }

    #[test]
    fn test_module_isolation_boundary() {
        let mut policy = ModuleSecurityPolicy::new(SecurityPolicy::default());

        let secure_module = ModulePath::from_string("secure.crypto").unwrap();
        let untrusted_module = ModulePath::from_string("user.plugin").unwrap();

        // Set up isolation boundary
        let boundary = ModuleIsolationBoundary {
            module: secure_module.clone(),
            allowed_imports: vec![],
            denied_imports: vec![untrusted_module.clone()],
            max_call_depth: 2,
            callable_from_untrusted: false,
        };

        policy.set_isolation_boundary(boundary);

        // Check cross-module call
        let result = policy.check_cross_module_call(
            &untrusted_module,
            &secure_module,
            TrustLevel::Untrusted,
        );

        assert!(result.is_err());
    }
}
