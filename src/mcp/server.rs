//! MCP Server implementation

use super::{MCPConfig, Request, Response};
use crate::error::Result;

/// MCP Server
pub struct MCPServer {
    config: MCPConfig,
}

impl MCPServer {
    pub fn new(config: MCPConfig) -> Self {
        Self { config }
    }

    pub fn handle_request(&self, request: Request) -> Result<Response> {
        // TODO: Implement request handling
        unimplemented!("MCP server request handling")
    }
}
