//! MCP Protocol implementation for Script Language
//!
//! Implements the Model Context Protocol (MCP) for AI integration

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// MCP Protocol version
pub const MCP_VERSION: &str = "2024-11-05";

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,
    /// Request ID for correlation
    pub id: String,
    /// Method being called
    pub method: MCPMethod,
    /// Method parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,
    /// Request ID that this response corresponds to
    pub id: String,
    /// Response result (present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<MCPResult>,
    /// Error information (present on error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
}

/// MCP Methods
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MCPMethod {
    Initialize,
    #[serde(rename = "tools/list")]
    ListTools,
    #[serde(rename = "tools/call")]
    CallTool,
    #[serde(rename = "resources/list")]
    ListResources,
    #[serde(rename = "resources/read")]
    ReadResource,
    #[serde(rename = "server/info")]
    GetServerInfo,
    Ping,
    #[serde(untagged)]
    Custom(String),
}

/// MCP Result types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MCPResult {
    Initialize {
        protocol_version: String,
        capabilities: ServerCapabilities,
        server_info: serde_json::Value,
    },
    ListTools {
        tools: Vec<Tool>,
    },
    CallTool {
        result: ToolResult,
    },
    ListResources {
        resources: Vec<Resource>,
    },
    ReadResource {
        contents: Vec<ResourceContent>,
    },
    GetServerInfo {
        info: serde_json::Value,
    },
    Ping {
        timestamp: i64,
    },
}

/// JSON-RPC Error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorObject {
    /// Error code
    pub code: i32,
    /// Human-readable error message
    pub message: String,
    /// Additional error data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Standard JSON-RPC error codes
#[allow(dead_code)]
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    // Server error codes: -32000 to -32099
    pub const SERVER_ERROR: i32 = -32000;
    pub const UNAUTHORIZED: i32 = -32001;
    pub const FORBIDDEN: i32 = -32002;
    pub const TIMEOUT: i32 = -32003;
    pub const RESOURCE_EXHAUSTED: i32 = -32004;
}

/// MCP Initialization request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeRequest {
    /// Protocol version
    pub protocol_version: String,
    /// Client capabilities
    pub capabilities: ClientCapabilities,
    /// Client information
    pub client_info: ClientInfo,
}

/// Client capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCapabilities {
    /// Whether client supports tools
    #[serde(default)]
    pub tools: bool,
    /// Whether client supports resources
    #[serde(default)]
    pub resources: bool,
    /// Whether client supports prompts
    #[serde(default)]
    pub prompts: bool,
}

/// Client information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    /// Client name
    pub name: String,
    /// Client version
    pub version: String,
}

/// MCP Initialization response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResponse {
    /// Protocol version supported by server
    pub protocol_version: String,
    /// Server capabilities
    pub capabilities: ServerCapabilities,
    /// Server information
    pub server_info: ServerInfo,
}

/// Server capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    /// Tools provided by the server
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<bool>,
    /// Resources provided by the server  
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<bool>,
    /// Prompts provided by the server
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<bool>,
    /// Experimental capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<serde_json::Value>,
}

/// Server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// Server name
    pub name: String,
    /// Server version
    pub version: String,
}

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Input schema (JSON Schema)
    pub input_schema: serde_json::Value,
}

/// Tool execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolRequest {
    /// Tool name to execute
    pub name: String,
    /// Tool arguments
    #[serde(default)]
    pub arguments: HashMap<String, serde_json::Value>,
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Tool output content
    pub content: Vec<serde_json::Value>,
    /// Whether execution resulted in error
    #[serde(default)]
    pub is_error: bool,
}

/// Text content block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    /// Content type (always "text")
    #[serde(rename = "type")]
    pub content_type: String,
    /// Text content
    pub text: String,
}

/// Resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// Resource URI
    pub uri: String,
    /// Resource name
    pub name: String,
    /// Resource description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Resource MIME type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Resource content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContent {
    /// Resource URI
    pub uri: String,
    /// Content blocks
    pub contents: Vec<ResourceContentBlock>,
}

/// Resource content block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResourceContentBlock {
    #[serde(rename = "text")]
    Text {
        /// Text content
        text: String,
    },
    #[serde(rename = "blob")]
    Blob {
        /// Binary data (base64 encoded)
        blob: String,
        /// MIME type
        mime_type: String,
    },
}

/// MCP notification (no response expected)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPNotification {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,
    /// Method name
    pub method: String,
    /// Method parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// MCP parameters (alias for Value)
pub type MCPParams = serde_json::Value;

impl Request {
    /// Create a new request
    pub fn new(id: String, method: MCPMethod, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method,
            params,
        }
    }
}

impl Response {
    /// Create a successful response
    pub fn success(id: String, result: MCPResult) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response
    pub fn error(id: String, error: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

impl ErrorObject {
    /// Create a new error object
    pub fn new(code: i32, message: String) -> Self {
        Self {
            code,
            message,
            data: None,
        }
    }

    /// Create an error with additional data
    pub fn with_data(code: i32, message: String, data: serde_json::Value) -> Self {
        Self {
            code,
            message,
            data: Some(data),
        }
    }
}

impl TextContent {
    /// Create a new text content block
    pub fn new(text: String) -> Self {
        Self {
            content_type: "text".to_string(),
            text,
        }
    }
}
