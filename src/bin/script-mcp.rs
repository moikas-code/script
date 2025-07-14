//! Script MCP Server Binary
//!
//! This binary provides the Model Context Protocol (MCP) server for the Script language,
//! offering AI-powered code analysis tools with enterprise-grade security.

use clap::{Arg, ArgAction, Command};
use script::mcp::{MCPConfig, MCPServer, ResourceLimits};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use uuid::Uuid;

/// Server configuration from CLI arguments and config files
#[derive(Debug, Clone)]
struct ServerConfig {
    /// Transport mode (stdio or tcp)
    transport: TransportMode,
    /// TCP port (if using TCP transport)
    port: u16,
    /// Security level
    security_level: SecurityLevel,
    /// Enable verbose logging
    verbose: bool,
    /// Maximum concurrent connections
    max_connections: usize,
    /// Configuration file path
    config_file: Option<String>,
}

/// Transport modes supported by the server
#[derive(Debug, Clone)]
enum TransportMode {
    /// Standard input/output (default for MCP)
    Stdio,
    /// TCP socket server
    Tcp,
}

/// Security levels
#[derive(Debug, Clone)]
enum SecurityLevel {
    /// Maximum security (default)
    Strict,
    /// Balanced security
    Standard,
    /// Minimal security (development only)
    Relaxed,
}

/// Server statistics
#[derive(Debug, Default)]
struct ServerStatistics {
    requests_processed: std::sync::atomic::AtomicU64,
    connections_handled: std::sync::atomic::AtomicU64,
    errors_encountered: std::sync::atomic::AtomicU64,
    uptime_start: std::time::Instant,
}

impl ServerStatistics {
    fn new() -> Self {
        Self {
            requests_processed: std::sync::atomic::AtomicU64::new(0),
            connections_handled: std::sync::atomic::AtomicU64::new(0),
            errors_encountered: std::sync::atomic::AtomicU64::new(0),
            uptime_start: std::time::Instant::now(),
        }
    }

    fn get_stats(&self) -> serde_json::Value {
        json!({
            "requests_processed": self.requests_processed.load(Ordering::Relaxed),
            "connections_handled": self.connections_handled.load(Ordering::Relaxed),
            "errors_encountered": self.errors_encountered.load(Ordering::Relaxed),
            "uptime_seconds": self.uptime_start.elapsed().as_secs()
        })
    }
}

fn main() {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    // Parse command line arguments
    let matches = Command::new("script-mcp")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Script Language MCP Server - AI-native code analysis with enterprise security")
        .arg(
            Arg::new("transport")
                .long("transport")
                .short('t')
                .value_name("MODE")
                .help("Transport mode: stdio (default) or tcp")
                .default_value("stdio")
                .value_parser(["stdio", "tcp"]),
        )
        .arg(
            Arg::new("port")
                .long("port")
                .short('p')
                .value_name("PORT")
                .help("TCP port number (when using tcp transport)")
                .default_value("8080")
                .value_parser(clap::value_parser!(u16)),
        )
        .arg(
            Arg::new("security")
                .long("security")
                .short('s')
                .value_name("LEVEL")
                .help("Security level: strict (default), standard, or relaxed")
                .default_value("strict")
                .value_parser(["strict", "standard", "relaxed"]),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .help("Enable verbose logging")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("max-connections")
                .long("max-connections")
                .value_name("COUNT")
                .help("Maximum concurrent connections")
                .default_value("10")
                .value_parser(clap::value_parser!(usize)),
        )
        .arg(
            Arg::new("config")
                .long("config")
                .short('c')
                .value_name("FILE")
                .help("Configuration file path"),
        )
        .arg(
            Arg::new("strict-mode")
                .long("strict-mode")
                .help("Enable strict security mode (maximum validation)")
                .action(ArgAction::SetTrue),
        )
        .get_matches();

    // Parse configuration
    let transport = match matches.get_one::<String>("transport").unwrap().as_str() {
        "stdio" => TransportMode::Stdio,
        "tcp" => TransportMode::Tcp,
        _ => unreachable!(),
    };

    let security_level = match matches.get_one::<String>("security").unwrap().as_str() {
        "strict" => SecurityLevel::Strict,
        "standard" => SecurityLevel::Standard,
        "relaxed" => SecurityLevel::Relaxed,
        _ => unreachable!(),
    };

    let config = ServerConfig {
        transport,
        port: *matches.get_one::<u16>("port").unwrap(),
        security_level: security_level.clone(),
        verbose: matches.get_flag("verbose"),
        max_connections: *matches.get_one::<usize>("max-connections").unwrap(),
        config_file: matches.get_one::<String>("config").cloned(),
    };

    // Override security level if strict-mode is specified
    let strict_mode = matches.get_flag("strict-mode");

    // Setup graceful shutdown
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let shutdown_flag_clone = shutdown_flag.clone();

    ctrlc::set_handler(move || {
        eprintln!("\nReceived shutdown signal, gracefully stopping server...");
        shutdown_flag_clone.store(true, Ordering::SeqCst);
    })
    .expect("Failed to set signal handler");

    // Create MCP configuration
    let mcp_config = create_mcp_config(&config, strict_mode);

    // Initialize server statistics
    let stats = Arc::new(ServerStatistics::new());

    // Print startup information
    print_startup_info(&config, &mcp_config);

    // Start server based on transport mode
    match config.transport {
        TransportMode::Stdio => {
            if let Err(e) = run_stdio_server(mcp_config, shutdown_flag, stats) {
                eprintln!("Stdio server error: {}", e);
                std::process::exit(1);
            }
        }
        TransportMode::Tcp => {
            if let Err(e) = run_tcp_server(config, mcp_config, shutdown_flag, stats) {
                eprintln!("TCP server error: {}", e);
                std::process::exit(1);
            }
        }
    }

    eprintln!("Script MCP Server shut down gracefully");
}

/// Create MCP configuration from server config
fn create_mcp_config(config: &ServerConfig, strict_mode: bool) -> MCPConfig {
    let (max_memory, max_request_size, max_concurrent) = match config.security_level {
        SecurityLevel::Strict => (50 * 1024 * 1024, 1 * 1024 * 1024, 5), // 50MB, 1MB, 5 requests
        SecurityLevel::Standard => (100 * 1024 * 1024, 5 * 1024 * 1024, 10), // 100MB, 5MB, 10 requests
        SecurityLevel::Relaxed => (200 * 1024 * 1024, 10 * 1024 * 1024, 20), // 200MB, 10MB, 20 requests
    };

    MCPConfig {
        max_request_size,
        request_timeout_ms: 30_000, // 30 seconds
        strict_security: strict_mode || matches!(config.security_level, SecurityLevel::Strict),
        resource_limits: ResourceLimits {
            max_memory,
            max_cpu_time_ms: 10_000, // 10 seconds
            max_concurrent_requests: max_concurrent,
        },
    }
}

/// Print startup information
fn print_startup_info(config: &ServerConfig, mcp_config: &MCPConfig) {
    eprintln!("🚀 Script MCP Server v{}", env!("CARGO_PKG_VERSION"));
    eprintln!("📡 Transport: {:?}", config.transport);

    if let TransportMode::Tcp = config.transport {
        eprintln!("🌐 Port: {}", config.port);
    }

    eprintln!("🔒 Security Level: {:?}", config.security_level);
    eprintln!(
        "📊 Max Memory: {}MB",
        mcp_config.resource_limits.max_memory / (1024 * 1024)
    );
    eprintln!(
        "📝 Max Request Size: {}MB",
        mcp_config.max_request_size / (1024 * 1024)
    );
    eprintln!(
        "⚡ Max Concurrent: {}",
        mcp_config.resource_limits.max_concurrent_requests
    );
    eprintln!(
        "⏱️  Request Timeout: {}s",
        mcp_config.request_timeout_ms / 1000
    );
    eprintln!("🛡️  Strict Security: {}", mcp_config.strict_security);
    eprintln!();
    eprintln!("Available tools:");
    eprintln!("  • script_analyzer - Comprehensive code analysis");
    eprintln!("  • script_formatter - Code formatting");
    eprintln!("  • script_lexer - Tokenization analysis");
    eprintln!("  • script_parser - AST analysis");
    eprintln!("  • script_semantic - Type and symbol analysis");
    eprintln!("  • script_quality - Code quality metrics");
    eprintln!("  • script_dependencies - Import/export analysis");
    eprintln!();
    eprintln!("Server ready. Send MCP requests to begin analysis.");

    if config.verbose {
        eprintln!("🔍 Verbose logging enabled");
    }
}

/// Run stdio-based MCP server
fn run_stdio_server(
    mcp_config: MCPConfig,
    shutdown_flag: Arc<AtomicBool>,
    stats: Arc<ServerStatistics>,
) -> Result<(), Box<dyn std::error::Error>> {
    let server = MCPServer::new(mcp_config);
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    eprintln!("📺 Listening on stdin/stdout for MCP requests...");

    for line in stdin.lock().lines() {
        // Check for shutdown signal
        if shutdown_flag.load(Ordering::SeqCst) {
            break;
        }

        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        // Parse JSON-RPC request
        match serde_json::from_str::<script::mcp::Request>(&line) {
            Ok(request) => {
                stats.requests_processed.fetch_add(1, Ordering::Relaxed);

                if let Some(response) = handle_request(&server, request, &stats) {
                    let response_json = serde_json::to_string(&response)?;
                    writeln!(stdout, "{}", response_json)?;
                    stdout.flush()?;
                }
            }
            Err(e) => {
                stats.errors_encountered.fetch_add(1, Ordering::Relaxed);
                eprintln!("Failed to parse JSON-RPC request: {}", e);

                // Send error response
                let error_response = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {
                        "code": -32700,
                        "message": "Parse error",
                        "data": format!("{}", e)
                    }
                });

                let response_json = serde_json::to_string(&error_response)?;
                writeln!(stdout, "{}", response_json)?;
                stdout.flush()?;
            }
        }
    }

    Ok(())
}

/// Run TCP-based MCP server
fn run_tcp_server(
    config: ServerConfig,
    mcp_config: MCPConfig,
    shutdown_flag: Arc<AtomicBool>,
    stats: Arc<ServerStatistics>,
) -> Result<(), Box<dyn std::error::Error>> {
    let address = format!("127.0.0.1:{}", config.port);
    let listener = TcpListener::bind(&address)?;

    eprintln!("🌐 TCP server listening on {}", address);

    // Set non-blocking to allow shutdown checks
    listener.set_nonblocking(true)?;

    let server = Arc::new(MCPServer::new(mcp_config));
    let mut active_connections = 0;

    loop {
        // Check for shutdown signal
        if shutdown_flag.load(Ordering::SeqCst) {
            eprintln!("🛑 Shutdown signal received, stopping TCP server");
            break;
        }

        // Accept connections
        match listener.accept() {
            Ok((stream, addr)) => {
                if active_connections >= config.max_connections {
                    eprintln!("⚠️  Maximum connections reached, rejecting {}", addr);
                    drop(stream);
                    continue;
                }

                active_connections += 1;
                stats.connections_handled.fetch_add(1, Ordering::Relaxed);

                eprintln!("🔗 New connection from {}", addr);

                let server_clone = server.clone();
                let stats_clone = stats.clone();
                let shutdown_flag_clone = shutdown_flag.clone();

                thread::spawn(move || {
                    if let Err(e) = handle_tcp_connection(
                        stream,
                        server_clone,
                        stats_clone,
                        shutdown_flag_clone,
                    ) {
                        eprintln!("❌ Connection error for {}: {}", addr, e);
                    }
                    eprintln!("🔌 Connection from {} closed", addr);
                });
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                // No connections available, sleep briefly
                thread::sleep(Duration::from_millis(100));
                continue;
            }
            Err(e) => {
                eprintln!("❌ Error accepting connection: {}", e);
                stats.errors_encountered.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    Ok(())
}

/// Handle a single TCP connection
fn handle_tcp_connection(
    mut stream: TcpStream,
    server: Arc<MCPServer>,
    stats: Arc<ServerStatistics>,
    shutdown_flag: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = BufReader::new(&stream);
    let mut line = String::new();

    loop {
        // Check for shutdown signal
        if shutdown_flag.load(Ordering::SeqCst) {
            break;
        }

        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break, // Connection closed
            Ok(_) => {
                if line.trim().is_empty() {
                    continue;
                }

                // Parse and handle request
                match serde_json::from_str::<script::mcp::Request>(&line) {
                    Ok(request) => {
                        stats.requests_processed.fetch_add(1, Ordering::Relaxed);

                        if let Some(response) = handle_request(&server, request, &stats) {
                            let response_json = serde_json::to_string(&response)?;
                            writeln!(stream, "{}", response_json)?;
                        }
                    }
                    Err(e) => {
                        stats.errors_encountered.fetch_add(1, Ordering::Relaxed);

                        let error_response = json!({
                            "jsonrpc": "2.0",
                            "id": null,
                            "error": {
                                "code": -32700,
                                "message": "Parse error",
                                "data": format!("{}", e)
                            }
                        });

                        let response_json = serde_json::to_string(&error_response)?;
                        writeln!(stream, "{}", response_json)?;
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading from TCP stream: {}", e);
                break;
            }
        }
    }

    Ok(())
}

/// Handle MCP request and return response
fn handle_request(
    server: &MCPServer,
    request: script::mcp::Request,
    stats: &Arc<ServerStatistics>,
) -> Option<script::mcp::Response> {
    let request_id = request.id.clone();

    match server.handle_request(request) {
        Ok(response) => {
            if log::log_enabled!(log::Level::Debug) {
                eprintln!("✅ Request {} processed successfully", request_id);
            }
            Some(response)
        }
        Err(e) => {
            stats.errors_encountered.fetch_add(1, Ordering::Relaxed);
            eprintln!("❌ Request {} failed: {}", request_id, e);

            // Create error response
            Some(script::mcp::Response {
                jsonrpc: "2.0".to_string(),
                id: request_id,
                result: None,
                error: Some(json!({
                    "code": -32603,
                    "message": "Internal error",
                    "data": format!("{}", e)
                })),
            })
        }
    }
}

/// Add ctrlc dependency for graceful shutdown
#[cfg(not(test))]
use ctrlc;

/// Mock ctrlc for tests
#[cfg(test)]
mod ctrlc {
    pub fn set_handler<F>(_handler: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: Fn() + 'static + Send,
    {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_creation() {
        let config = ServerConfig {
            transport: TransportMode::Stdio,
            port: 8080,
            security_level: SecurityLevel::Strict,
            verbose: false,
            max_connections: 10,
            config_file: None,
        };

        let mcp_config = create_mcp_config(&config, false);
        assert!(mcp_config.strict_security); // Strict security level enables strict mode
        assert_eq!(mcp_config.resource_limits.max_concurrent_requests, 5);
    }

    #[test]
    fn test_security_levels() {
        let strict_config = ServerConfig {
            transport: TransportMode::Stdio,
            port: 8080,
            security_level: SecurityLevel::Strict,
            verbose: false,
            max_connections: 10,
            config_file: None,
        };

        let relaxed_config = ServerConfig {
            transport: TransportMode::Stdio,
            port: 8080,
            security_level: SecurityLevel::Relaxed,
            verbose: false,
            max_connections: 10,
            config_file: None,
        };

        let strict_mcp = create_mcp_config(&strict_config, false);
        let relaxed_mcp = create_mcp_config(&relaxed_config, false);

        assert!(strict_mcp.resource_limits.max_memory < relaxed_mcp.resource_limits.max_memory);
        assert!(strict_mcp.max_request_size < relaxed_mcp.max_request_size);
        assert!(
            strict_mcp.resource_limits.max_concurrent_requests
                < relaxed_mcp.resource_limits.max_concurrent_requests
        );
    }

    #[test]
    fn test_server_statistics() {
        let stats = ServerStatistics::new();
        assert_eq!(stats.requests_processed.load(Ordering::Relaxed), 0);

        stats.requests_processed.fetch_add(5, Ordering::Relaxed);
        assert_eq!(stats.requests_processed.load(Ordering::Relaxed), 5);

        let stats_json = stats.get_stats();
        assert_eq!(stats_json["requests_processed"], 5);
    }
}
