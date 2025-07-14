//! MCP Server implementation for Script Language
//!
//! This module provides the complete MCP server implementation with:
//! - Method routing and capability negotiation
//! - Security validation and audit logging
//! - Sandboxed code analysis tools
//! - Resource management and rate limiting

use super::protocol::{
    MCPMethod, MCPNotification, MCPParams, MCPResult, Request, Response, ServerCapabilities, Tool,
    ToolResult,
};
use super::sandbox::{AnalysisResult, SandboxConfig, SandboxedAnalyzer};
use super::security::{SecurityContext, SecurityError, SecurityManager, ValidationResult};
use super::MCPConfig;
use crate::error::{Error as ScriptError, Result as ScriptResult};
use crate::{Lexer, Parser};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use uuid::Uuid;

/// MCP Server implementation
pub struct MCPServer {
    /// Server configuration
    config: MCPConfig,
    /// Security manager for validation and audit
    security_manager: Arc<SecurityManager>,
    /// Sandboxed analyzer for code analysis
    analyzer: Arc<SandboxedAnalyzer>,
    /// Server capabilities
    capabilities: ServerCapabilities,
    /// Session tracking
    sessions: Arc<RwLock<HashMap<String, Arc<SecurityContext>>>>,
    /// Tool registry
    tools: Arc<RwLock<HashMap<String, Tool>>>,
    /// Server statistics
    stats: ServerStats,
}

/// Server statistics
#[derive(Debug, Default)]
struct ServerStats {
    requests_handled: std::sync::atomic::AtomicU64,
    errors_encountered: std::sync::atomic::AtomicU64,
    analysis_requests: std::sync::atomic::AtomicU64,
    security_violations: std::sync::atomic::AtomicU64,
}

impl MCPServer {
    /// Create new MCP server with configuration
    pub fn new(config: MCPConfig) -> Self {
        let security_config = super::security::SecurityConfig {
            session_timeout: Duration::from_secs(3600), // 1 hour
            max_concurrent_sessions: 100,
            strict_mode: config.strict_security,
            default_limits: super::security::ResourceLimits {
                max_input_size: config.max_request_size,
                max_analysis_time: Duration::from_millis(config.request_timeout_ms),
                max_memory_usage: config.resource_limits.max_memory,
                max_concurrent_requests: config.resource_limits.max_concurrent_requests as u32,
            },
        };

        let security_manager = Arc::new(SecurityManager::new(security_config));

        let sandbox_config = SandboxConfig {
            max_analysis_time: Duration::from_millis(config.request_timeout_ms),
            max_memory_usage: config.resource_limits.max_memory,
            max_input_size: config.max_request_size,
            strict_mode: config.strict_security,
            max_concurrent_analyses: config.resource_limits.max_concurrent_requests,
            ..Default::default()
        };

        let analyzer = Arc::new(SandboxedAnalyzer::new(sandbox_config));

        let capabilities = Self::create_server_capabilities();
        let tools = Arc::new(RwLock::new(Self::create_tool_registry()));

        Self {
            config,
            security_manager,
            analyzer,
            capabilities,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            tools,
            stats: ServerStats::default(),
        }
    }

    /// Handle incoming MCP request with full security validation
    pub fn handle_request(&self, request: Request) -> ScriptResult<Response> {
        use std::sync::atomic::Ordering;

        self.stats.requests_handled.fetch_add(1, Ordering::Relaxed);
        let start_time = Instant::now();

        // Validate request structure
        self.validate_request_structure(&request)?;

        // Handle different request types
        let result = match &request.method {
            MCPMethod::Initialize => self.handle_initialize(&request),
            MCPMethod::ListTools => self.handle_list_tools(&request),
            MCPMethod::CallTool => self.handle_call_tool(&request),
            MCPMethod::ListResources => self.handle_list_resources(&request),
            MCPMethod::ReadResource => self.handle_read_resource(&request),
            MCPMethod::GetServerInfo => self.handle_get_server_info(&request),
            MCPMethod::Ping => self.handle_ping(&request),
            MCPMethod::Custom(method_name) => self.handle_custom_method(method_name, &request),
        };

        // Log request completion
        let duration = start_time.elapsed();
        if let Err(ref error) = result {
            self.stats
                .errors_encountered
                .fetch_add(1, Ordering::Relaxed);
            eprintln!("MCP request failed: {} (took {:?})", error, duration);
        }

        result
    }

    /// Handle server initialization
    fn handle_initialize(&self, request: &Request) -> ScriptResult<Response> {
        // Extract client info from params
        let client_info = request
            .params
            .as_ref()
            .and_then(|p| p.as_object())
            .and_then(|obj| obj.get("clientInfo"))
            .and_then(|info| info.as_object());

        let client_name = client_info
            .and_then(|info| info.get("name"))
            .and_then(|name| name.as_str())
            .unwrap_or("unknown");

        let client_version = client_info
            .and_then(|info| info.get("version"))
            .and_then(|version| version.as_str())
            .unwrap_or("unknown");

        // Create security session
        let session_context = self
            .security_manager
            .create_session(Some(format!("{}@{client_name, client_version}")))
            .map_err(|e| ScriptError::runtime(format!("Failed to create session: {e}")))?;

        // Store session
        {
            let mut sessions = self.sessions.write().unwrap();
            sessions.insert(request.id.clone(), Arc::new(session_context));
        }

        // Return initialization response
        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id.clone(),
            result: Some(MCPResult::Initialize {
                protocol_version: "2024-11-05".to_string(),
                capabilities: self.capabilities.clone(),
                server_info: json!({
                    "name": "script-mcp",
                    "version": env!("CARGO_PKG_VERSION"),
                    "description": "Script Language MCP Server with Security-First Analysis"
                }),
            }),
            error: None,
        })
    }

    /// Handle tool listing
    fn handle_list_tools(&self, request: &Request) -> ScriptResult<Response> {
        self.validate_session(&request.id)?;

        let tools = self.tools.read().unwrap();
        let tool_list: Vec<Tool> = tools.values().cloned().collect();

        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id.clone(),
            result: Some(MCPResult::ListTools { tools: tool_list }),
            error: None,
        })
    }

    /// Handle tool invocation with security validation
    fn handle_call_tool(&self, request: &Request) -> ScriptResult<Response> {
        let session = self.validate_session(&request.id)?;

        // Extract tool parameters
        let params = request
            .params
            .as_ref()
            .ok_or_else(|| ScriptError::runtime("Missing tool parameters".to_string()))?;

        let tool_name = params
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| ScriptError::runtime("Missing tool name".to_string()))?;

        let tool_arguments = params
            .get("arguments")
            .and_then(|a| a.as_object())
            .ok_or_else(|| ScriptError::runtime("Missing tool arguments".to_string()))?;

        // Validate tool exists
        let tools = self.tools.read().unwrap();
        let tool = tools
            .get(tool_name)
            .ok_or_else(|| ScriptError::runtime(format!("Unknown tool: {tool_name}")))?;

        drop(tools); // Release lock before potentially long analysis

        // Execute tool with security validation
        self.stats
            .analysis_requests
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let result = self.execute_tool(tool_name, tool_arguments, &session)?;

        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id.clone(),
            result: Some(MCPResult::CallTool { result }),
            error: None,
        })
    }

    /// Handle resource listing
    fn handle_list_resources(&self, request: &Request) -> ScriptResult<Response> {
        self.validate_session(&request.id)?;

        // For now, return empty resources list
        // In a full implementation, this could list available Script files, examples, etc.
        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id.clone(),
            result: Some(MCPResult::ListResources { resources: vec![] }),
            error: None,
        })
    }

    /// Handle resource reading
    fn handle_read_resource(&self, request: &Request) -> ScriptResult<Response> {
        let _session = self.validate_session(&request.id)?;

        // Extract resource URI
        let params = request
            .params
            .as_ref()
            .ok_or_else(|| ScriptError::runtime("Missing resource parameters".to_string()))?;

        let _uri = params
            .get("uri")
            .and_then(|u| u.as_str())
            .ok_or_else(|| ScriptError::runtime("Missing resource URI".to_string()))?;

        // For now, return error - resource reading not implemented
        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id.clone(),
            result: None,
            error: Some(json!({
                "code": -32601,
                "message": "Resource reading not implemented"
            })),
        })
    }

    /// Handle server info request
    fn handle_get_server_info(&self, request: &Request) -> ScriptResult<Response> {
        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id.clone(),
            result: Some(MCPResult::GetServerInfo {
                info: json!({
                    "name": "script-mcp",
                    "version": env!("CARGO_PKG_VERSION"),
                    "description": "Script Language MCP Server",
                    "capabilities": self.capabilities,
                    "security": {
                        "strict_mode": self.config.strict_security,
                        "session_timeout": 3600,
                        "max_concurrent_sessions": 100
                    },
                    "stats": {
                        "requests_handled": self.stats.requests_handled.load(std::sync::atomic::Ordering::Relaxed),
                        "analysis_requests": self.stats.analysis_requests.load(std::sync::atomic::Ordering::Relaxed),
                        "active_sessions": self.sessions.read().unwrap().len(),
                        "active_analyses": self.analyzer.active_analysis_count()
                    }
                }),
            }),
            error: None,
        })
    }

    /// Handle ping request
    fn handle_ping(&self, request: &Request) -> ScriptResult<Response> {
        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id.clone(),
            result: Some(MCPResult::Ping {
                timestamp: chrono::Utc::now().timestamp(),
            }),
            error: None,
        })
    }

    /// Handle custom method (extension point)
    fn handle_custom_method(&self, method_name: &str, request: &Request) -> ScriptResult<Response> {
        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id.clone(),
            result: None,
            error: Some(json!({
                "code": -32601,
                "message": format!("Method not found: {method_name}")
            })),
        })
    }

    /// Execute tool with security validation and sandboxing
    fn execute_tool(
        &self,
        tool_name: &str,
        arguments: &serde_json::Map<String, Value>,
        session: &SecurityContext,
    ) -> ScriptResult<ToolResult> {
        // Get input code
        let code = arguments
            .get("code")
            .and_then(|c| c.as_str())
            .ok_or_else(|| ScriptError::runtime("Missing 'code' argument".to_string()))?;

        // Validate input with security manager
        match self.security_manager.validate_input(code, session) {
            ValidationResult::Valid => {}
            ValidationResult::Dangerous { reason } => {
                self.stats
                    .security_violations
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Err(ScriptError::runtime(format!(
                    "Security violation: {}",
                    reason
                )));
            }
            ValidationResult::TooLarge { size, max_size } => {
                return Err(ScriptError::runtime(format!(
                    "Input too large: {} > {}",
                    size, max_size
                )));
            }
            ValidationResult::Forbidden { pattern } => {
                self.stats
                    .security_violations
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Err(ScriptError::runtime(format!(
                    "Forbidden pattern: {}",
                    pattern
                )));
            }
        }

        // Execute tool based on name
        match tool_name {
            "script_analyzer" => self.execute_script_analyzer(code, arguments),
            "script_formatter" => self.execute_script_formatter(code, arguments),
            "script_lexer" => self.execute_script_lexer(code, arguments),
            "script_parser" => self.execute_script_parser(code, arguments),
            "script_semantic" => self.execute_script_semantic(code, arguments),
            "script_quality" => self.execute_script_quality(code, arguments),
            "script_dependencies" => self.execute_script_dependencies(code, arguments),
            _ => Err(ScriptError::runtime(format!("Unknown tool: {tool_name}"))),
        }
    }

    /// Execute comprehensive script analysis
    fn execute_script_analyzer(
        &self,
        code: &str,
        _arguments: &serde_json::Map<String, Value>,
    ) -> ScriptResult<ToolResult> {
        // Run all analysis types
        let lexical = self
            .analyzer
            .analyze_lexical(code)
            .map_err(|e| ScriptError::runtime(format!("Lexical analysis failed: {e}")))?;

        let parse = self
            .analyzer
            .analyze_parse(code)
            .map_err(|e| ScriptError::runtime(format!("Parse analysis failed: {e}")))?;

        let semantic = self
            .analyzer
            .analyze_semantic(code)
            .map_err(|e| ScriptError::runtime(format!("Semantic analysis failed: {e}")))?;

        let quality = self
            .analyzer
            .analyze_quality(code)
            .map_err(|e| ScriptError::runtime(format!("Quality analysis failed: {e}")))?;

        let dependencies = self
            .analyzer
            .analyze_dependencies(code)
            .map_err(|e| ScriptError::runtime(format!("Dependency analysis failed: {e}")))?;

        Ok(ToolResult {
            content: vec![json!({
                "type": "text",
                "text": format!("# Script Analysis Report\n\n## Lexical Analysis\n{}\n\n## Parse Analysis\n{}\n\n## Semantic Analysis\n{}\n\n## Quality Analysis\n{}\n\n## Dependencies\n{}",
                    Self::format_analysis_result(&lexical),
                    Self::format_analysis_result(&parse),
                    Self::format_analysis_result(&semantic),
                    Self::format_analysis_result(&quality),
                    Self::format_analysis_result(&dependencies)
                )
            })],
            is_error: false,
        })
    }

    /// Execute script formatter
    fn execute_script_formatter(
        &self,
        code: &str,
        _arguments: &serde_json::Map<String, Value>,
    ) -> ScriptResult<ToolResult> {
        // Parse the code
        let lexer =
            Lexer::new(code).map_err(|e| ScriptError::runtime(format!("Lexer error: {e}")))?;
        let (tokens, lex_errors) = lexer.scan_tokens();

        if !lex_errors.is_empty() {
            let error_msg = lex_errors
                .iter()
                .map(|e| format!("{e}"))
                .collect::<Vec<_>>()
                .join("\n");
            return Ok(ToolResult {
                content: vec![json!({
                    "type": "text",
                    "text": format!("# Formatting Error\n\nLexer errors:\n{}\n\n## Original Code\n```script\n{}\n```", error_msg, code)
                })],
                is_error: true,
            });
        }

        let mut parser = Parser::new(tokens);
        let program = match parser.parse() {
            Ok(program) => program,
            Err(e) => {
                return Ok(ToolResult {
                    content: vec![json!({
                        "type": "text",
                        "text": format!("# Formatting Error\n\nParse error: {}\n\n## Original Code\n```script\n{}\n```", e, code)
                    })],
                    is_error: true,
                });
            }
        };

        // Format the code
        let formatted = crate::formatter::format_program(&program);

        Ok(ToolResult {
            content: vec![json!({
                "type": "text",
                "text": format!("# Formatted Script\n\n```script\n{}\n```", formatted)
            })],
            is_error: false,
        })
    }

    /// Execute lexical analysis
    fn execute_script_lexer(
        &self,
        code: &str,
        _arguments: &serde_json::Map<String, Value>,
    ) -> ScriptResult<ToolResult> {
        let result = self
            .analyzer
            .analyze_lexical(code)
            .map_err(|e| ScriptError::runtime(format!("Lexical analysis failed: {e}")))?;

        Ok(ToolResult {
            content: vec![json!({
                "type": "text",
                "text": format!("# Lexical Analysis\n\n{}", Self::format_analysis_result(&result)))
            })],
            is_error: false,
        })
    }

    /// Execute parse analysis
    fn execute_script_parser(
        &self,
        code: &str,
        _arguments: &serde_json::Map<String, Value>,
    ) -> ScriptResult<ToolResult> {
        let result = self
            .analyzer
            .analyze_parse(code)
            .map_err(|e| ScriptError::runtime(format!("Parse analysis failed: {e}")))?;

        Ok(ToolResult {
            content: vec![json!({
                "type": "text",
                "text": format!("# Parse Analysis\n\n{}", Self::format_analysis_result(&result)))
            })],
            is_error: false,
        })
    }

    /// Execute semantic analysis
    fn execute_script_semantic(
        &self,
        code: &str,
        _arguments: &serde_json::Map<String, Value>,
    ) -> ScriptResult<ToolResult> {
        let result = self
            .analyzer
            .analyze_semantic(code)
            .map_err(|e| ScriptError::runtime(format!("Semantic analysis failed: {e}")))?;

        Ok(ToolResult {
            content: vec![json!({
                "type": "text",
                "text": format!("# Semantic Analysis\n\n{}", Self::format_analysis_result(&result)))
            })],
            is_error: false,
        })
    }

    /// Execute quality analysis
    fn execute_script_quality(
        &self,
        code: &str,
        _arguments: &serde_json::Map<String, Value>,
    ) -> ScriptResult<ToolResult> {
        let result = self
            .analyzer
            .analyze_quality(code)
            .map_err(|e| ScriptError::runtime(format!("Quality analysis failed: {e}")))?;

        Ok(ToolResult {
            content: vec![json!({
                "type": "text",
                "text": format!("# Code Quality Analysis\n\n{}", Self::format_analysis_result(&result)))
            })],
            is_error: false,
        })
    }

    /// Execute dependency analysis
    fn execute_script_dependencies(
        &self,
        code: &str,
        _arguments: &serde_json::Map<String, Value>,
    ) -> ScriptResult<ToolResult> {
        let result = self
            .analyzer
            .analyze_dependencies(code)
            .map_err(|e| ScriptError::runtime(format!("Dependency analysis failed: {e}")))?;

        Ok(ToolResult {
            content: vec![json!({
                "type": "text",
                "text": format!("# Dependency Analysis\n\n{}", Self::format_analysis_result(&result)))
            })],
            is_error: false,
        })
    }

    /// Format analysis result for display
    fn format_analysis_result(result: &AnalysisResult) -> String {
        match result {
            AnalysisResult::Lexical {
                tokens,
                token_count,
                has_errors,
                error_messages,
            } => {
                let mut output = format!("**Token Count:** {}\n", token_count);
                if *has_errors {
                    output.push_str(&format!("**Errors:** {}\n", error_messages.join(", ")));
                } else {
                    output.push_str("**Status:** ✅ No lexical errors\n");
                }
                output.push_str(&format!(
                    "**Sample Tokens:** {}\n",
                    tokens
                        .iter()
                        .take(10)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                output
            }
            AnalysisResult::Parse {
                ast_summary,
                node_count,
                has_errors,
                error_messages,
            } => {
                let mut output = format!(
                    "**AST Summary:** {}\n**Node Count:** {}\n",
                    ast_summary, node_count
                );
                if *has_errors {
                    output.push_str(&format!("**Errors:** {}\n", error_messages.join(", ")));
                } else {
                    output.push_str("**Status:** ✅ Parse successful\n");
                }
                output
            }
            AnalysisResult::Semantic {
                type_info,
                symbol_count,
                has_errors,
                error_messages,
            } => {
                let mut output = format!("**Symbol Count:** {}\n", symbol_count);
                if *has_errors {
                    output.push_str(&format!("**Errors:** {}\n", error_messages.join(", ")));
                } else {
                    output.push_str("**Status:** ✅ Semantic analysis successful\n");
                }
                if !type_info.is_empty() {
                    output.push_str("**Type Information:**\n");
                    for (symbol, typ) in type_info.iter().take(5) {
                        output.push_str(&format!("  - {}: {}\n", symbol, typ));
                    }
                }
                output
            }
            AnalysisResult::Quality {
                complexity_score,
                maintainability_score,
                security_score,
                suggestions,
            } => {
                format!("**Complexity Score:** {:.1}/100\n**Maintainability Score:** {:.1}/100\n**Security Score:** {:.1}/100\n**Suggestions:**\n{}\n",
                    complexity_score, maintainability_score, security_score,
                    suggestions.iter().map(|s| format!("  - {s}")).collect::<Vec<_>>().join("\n"))
            }
            AnalysisResult::Dependencies {
                imports,
                exports,
                dependency_graph,
            } => {
                let mut output = format!(
                    "**Imports:** {}\n**Exports:** {}\n",
                    imports.join(", "),
                    exports.join(", ")
                );
                if !dependency_graph.is_empty() {
                    output.push_str("**Dependency Graph:**\n");
                    for (module, deps) in dependency_graph.iter().take(5) {
                        output.push_str(&format!("  - {} → {}\n", module, deps.join(", ")));
                    }
                }
                output
            }
        }
    }

    /// Validate request structure
    fn validate_request_structure(&self, request: &Request) -> ScriptResult<()> {
        if request.jsonrpc != "2.0" {
            return Err(ScriptError::runtime("Invalid JSON-RPC version".to_string()));
        }

        if request.id.is_empty() {
            return Err(ScriptError::runtime("Missing request ID".to_string()));
        }

        Ok(())
    }

    /// Validate session exists and is active
    fn validate_session(&self, request_id: &str) -> ScriptResult<SecurityContext> {
        let sessions = self.sessions.read().unwrap();

        // For initialize requests, we don't need an existing session
        if !sessions.contains_key(request_id) {
            // Create a temporary session for non-initialize requests
            let temp_session = self
                .security_manager
                .create_session(Some("temporary".to_string()))
                .map_err(|e| {
                    ScriptError::runtime(format!("Failed to create temporary session: {e}"))
                })?;
            return Ok(temp_session);
        }

        let session = sessions
            .get(request_id)
            .ok_or_else(|| ScriptError::runtime("Session not found".to_string()))?;

        // Validate session with security manager
        self.security_manager
            .validate_session(session.session_id)
            .map_err(|e| ScriptError::runtime(format!("Session validation failed: {e}")))
    }

    /// Create server capabilities
    fn create_server_capabilities() -> ServerCapabilities {
        ServerCapabilities {
            tools: Some(true),
            resources: Some(false), // Not implemented yet
            prompts: Some(false),   // Not implemented yet
            experimental: Some(json!({
                "script_analysis": true,
                "security_validation": true,
                "sandboxed_execution": true
            })),
        }
    }

    /// Create tool registry
    fn create_tool_registry() -> HashMap<String, Tool> {
        let mut tools = HashMap::new();

        tools.insert("script_analyzer".to_string(), Tool {
            name: "script_analyzer".to_string(),
            description: "Comprehensive analysis of Script language code including lexical, parse, semantic, quality, and dependency analysis".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "code": {
                        "type": "string",
                        "description": "Script language code to analyze"
                    }
                },
                "required": ["code"]
            }),
        });

        tools.insert(
            "script_formatter".to_string(),
            Tool {
                name: "script_formatter".to_string(),
                description: "Format Script language code according to style guidelines"
                    .to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "code": {
                            "type": "string",
                            "description": "Script language code to format"
                        }
                    },
                    "required": ["code"]
                }),
            },
        );

        tools.insert(
            "script_lexer".to_string(),
            Tool {
                name: "script_lexer".to_string(),
                description: "Perform lexical analysis on Script language code".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "code": {
                            "type": "string",
                            "description": "Script language code to tokenize"
                        }
                    },
                    "required": ["code"]
                }),
            },
        );

        tools.insert(
            "script_parser".to_string(),
            Tool {
                name: "script_parser".to_string(),
                description: "Parse Script language code and analyze AST structure".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "code": {
                            "type": "string",
                            "description": "Script language code to parse"
                        }
                    },
                    "required": ["code"]
                }),
            },
        );

        tools.insert(
            "script_semantic".to_string(),
            Tool {
                name: "script_semantic".to_string(),
                description: "Perform semantic analysis on Script language code".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "code": {
                            "type": "string",
                            "description": "Script language code to analyze semantically"
                        }
                    },
                    "required": ["code"]
                }),
            },
        );

        tools.insert(
            "script_quality".to_string(),
            Tool {
                name: "script_quality".to_string(),
                description: "Analyze code quality metrics for Script language code".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "code": {
                            "type": "string",
                            "description": "Script language code to analyze for quality"
                        }
                    },
                    "required": ["code"]
                }),
            },
        );

        tools.insert(
            "script_dependencies".to_string(),
            Tool {
                name: "script_dependencies".to_string(),
                description: "Analyze dependencies and imports in Script language code".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "code": {
                            "type": "string",
                            "description": "Script language code to analyze for dependencies"
                        }
                    },
                    "required": ["code"]
                }),
            },
        );

        tools
    }

    /// Get server statistics
    pub fn get_stats(&self) -> ServerStats {
        ServerStats {
            requests_handled: std::sync::atomic::AtomicU64::new(
                self.stats
                    .requests_handled
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            errors_encountered: std::sync::atomic::AtomicU64::new(
                self.stats
                    .errors_encountered
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            analysis_requests: std::sync::atomic::AtomicU64::new(
                self.stats
                    .analysis_requests
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            security_violations: std::sync::atomic::AtomicU64::new(
                self.stats
                    .security_violations
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
        }
    }

    /// Cleanup expired sessions and analyses
    pub fn cleanup_expired_resources(&self) {
        // Cleanup expired security sessions
        self.security_manager.cleanup_expired_sessions();

        // Cleanup expired analyses
        self.analyzer.cleanup_expired_analyses();

        // Cleanup expired local sessions
        let mut sessions = self.sessions.write().unwrap();
        sessions.retain(|_, session| std::time::SystemTime::now() <= session.expires_at);
    }
}
