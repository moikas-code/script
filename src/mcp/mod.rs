//! Model Context Protocol (MCP) Implementation
//!
//! This module provides secure AI model integration for the Script language.

pub mod protocol;
pub mod sandbox;
pub mod security;
pub mod server;

// Re-export main types
pub use protocol::{Request, Response};
pub use sandbox::{AnalysisResult, SandboxConfig, SandboxedAnalyzer};
pub use security::SecurityContext;
pub use server::MCPServer;

/// MCP configuration
#[derive(Debug, Clone)]
pub struct MCPConfig {
    /// Maximum request size in bytes
    pub max_request_size: usize,
    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,
    /// Enable strict security mode
    pub strict_security: bool,
    /// Resource limits
    pub resource_limits: ResourceLimits,
}

#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum memory usage in bytes
    pub max_memory: usize,
    /// Maximum CPU time in milliseconds
    pub max_cpu_time_ms: u64,
    /// Maximum number of concurrent requests
    pub max_concurrent_requests: usize,
}

impl Default for MCPConfig {
    fn default() -> Self {
        Self {
            max_request_size: 10 * 1024 * 1024, // 10MB
            request_timeout_ms: 30_000,         // 30 seconds
            strict_security: true,
            resource_limits: ResourceLimits::default(),
        }
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory: 100 * 1024 * 1024, // 100MB
            max_cpu_time_ms: 10_000,       // 10 seconds
            max_concurrent_requests: 10,
        }
    }
}
