//! Sandboxed Analysis Environment for MCP Server
//!
//! This module provides a secure, isolated environment for analyzing Script code
//! with comprehensive resource constraints and security monitoring.

use crate::compilation::DependencyAnalyzer;
use crate::error::Error as ScriptError;
use crate::lexer::Lexer;
use crate::parser::{Parser, Program};
use crate::semantic::SemanticAnalyzer;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Maximum analysis time per request (30 seconds)
const MAX_ANALYSIS_TIME: Duration = Duration::from_secs(30);

/// Maximum memory usage per analysis (10MB)
const MAX_MEMORY_USAGE: usize = 10 * 1024 * 1024;

/// Maximum input size for analysis (1MB)
const MAX_INPUT_SIZE: usize = 1024 * 1024;

/// Sandboxed analysis configuration
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Maximum analysis time
    pub max_analysis_time: Duration,
    /// Maximum memory usage
    pub max_memory_usage: usize,
    /// Maximum input size
    pub max_input_size: usize,
    /// Enable strict mode (extra security checks)
    pub strict_mode: bool,
    /// Maximum concurrent analyses
    pub max_concurrent_analyses: usize,
    /// Enable detailed logging
    pub enable_logging: bool,
    /// Allowed file extensions
    pub allowed_extensions: Vec<String>,
    /// Blocked patterns in code
    pub blocked_patterns: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            max_analysis_time: MAX_ANALYSIS_TIME,
            max_memory_usage: MAX_MEMORY_USAGE,
            max_input_size: MAX_INPUT_SIZE,
            strict_mode: true,
            max_concurrent_analyses: 10,
            enable_logging: true,
            allowed_extensions: vec![
                ".script".to_string(),
                ".txt".to_string(),
                ".md".to_string(),
                ".toml".to_string(),
                ".json".to_string(),
            ],
            blocked_patterns: vec![
                "eval(".to_string(),
                "exec(".to_string(),
                "system(".to_string(),
                "shell(".to_string(),
                "import os".to_string(),
                "import subprocess".to_string(),
                "__import__".to_string(),
                "file://".to_string(),
                "javascript:".to_string(),
            ],
        }
    }
}

/// Analysis context for tracking resources
#[derive(Debug)]
pub struct AnalysisContext {
    /// Unique analysis ID
    pub analysis_id: Uuid,
    /// Analysis start time
    pub start_time: Instant,
    /// Memory usage tracker
    pub memory_usage: AtomicUsize,
    /// Analysis status
    pub status: AtomicU64, // 0: running, 1: completed, 2: failed, 3: timeout
    /// Cancellation flag
    pub cancelled: AtomicBool,
}

impl AnalysisContext {
    pub fn new() -> Self {
        Self {
            analysis_id: Uuid::new_v4(),
            start_time: Instant::now(),
            memory_usage: AtomicUsize::new(0),
            status: AtomicU64::new(0), // running
            cancelled: AtomicBool::new(false),
        }
    }

    /// Check if analysis has timed out
    pub fn is_timed_out(&self, max_time: Duration) -> bool {
        self.start_time.elapsed() > max_time
    }

    /// Update memory usage
    pub fn update_memory(&self, new_usage: usize) {
        self.memory_usage.store(new_usage, Ordering::Relaxed);
    }

    /// Get current memory usage
    pub fn get_memory_usage(&self) -> usize {
        self.memory_usage.load(Ordering::Relaxed)
    }

    /// Mark as completed
    pub fn mark_completed(&self) {
        self.status.store(1, Ordering::SeqCst);
    }

    /// Mark as failed
    pub fn mark_failed(&self) {
        self.status.store(2, Ordering::SeqCst);
    }

    /// Mark as timed out
    pub fn mark_timeout(&self) {
        self.status.store(3, Ordering::SeqCst);
    }

    /// Cancel analysis
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Check if cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// Analysis result types
#[derive(Debug, Clone)]
pub enum AnalysisResult {
    /// Lexical analysis result
    Lexical {
        tokens: Vec<String>,
        token_count: usize,
        has_errors: bool,
        error_messages: Vec<String>,
    },
    /// Parse analysis result
    Parse {
        ast_summary: String,
        node_count: usize,
        has_errors: bool,
        error_messages: Vec<String>,
    },
    /// Semantic analysis result
    Semantic {
        type_info: HashMap<String, String>,
        symbol_count: usize,
        has_errors: bool,
        error_messages: Vec<String>,
    },
    /// Code quality analysis
    Quality {
        complexity_score: f64,
        maintainability_score: f64,
        security_score: f64,
        suggestions: Vec<String>,
    },
    /// Dependency analysis
    Dependencies {
        imports: Vec<String>,
        exports: Vec<String>,
        dependency_graph: HashMap<String, Vec<String>>,
    },
}

/// Sandbox error types
#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    #[error("Analysis timed out after {timeout_secs} seconds")]
    Timeout { timeout_secs: u64 },
    #[error("Memory limit exceeded: {usage} bytes > {limit} bytes")]
    MemoryLimitExceeded { usage: usize, limit: usize },
    #[error("Input too large: {size} bytes > {max_size} bytes")]
    InputTooLarge { size: usize, max_size: usize },
    #[error("Too many concurrent analyses: {current} >= {limit}")]
    TooManyAnalyses { current: usize, limit: usize },
    #[error("Blocked pattern detected: {pattern}")]
    BlockedPattern { pattern: String },
    #[error("Invalid file extension: {extension}")]
    InvalidExtension { extension: String },
    #[error("Analysis cancelled")]
    Cancelled,
    #[error("Script error: {0}")]
    ScriptError(#[from] ScriptError),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Sandboxed analyzer implementation
pub struct SandboxedAnalyzer {
    /// Configuration
    config: SandboxConfig,
    /// Active analyses
    active_analyses: Arc<RwLock<HashMap<Uuid, Arc<AnalysisContext>>>>,
    /// Analysis counter
    analysis_counter: AtomicUsize,
}

impl SandboxedAnalyzer {
    /// Create new sandboxed analyzer
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            config,
            active_analyses: Arc::new(RwLock::new(HashMap::new())),
            analysis_counter: AtomicUsize::new(0),
        }
    }

    /// Validate input before analysis
    fn validate_input(&self, input: &str, file_path: Option<&Path>) -> Result<(), SandboxError> {
        // Check input size
        if input.len() > self.config.max_input_size {
            return Err(SandboxError::InputTooLarge {
                size: input.len(),
                max_size: self.config.max_input_size,
            });
        }

        // Check file extension if provided
        if let Some(path) = file_path {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext_with_dot = format!(".{}", ext);
                if !self.config.allowed_extensions.contains(&ext_with_dot) {
                    return Err(SandboxError::InvalidExtension {
                        extension: ext_with_dot,
                    });
                }
            }
        }

        // Check for blocked patterns in strict mode
        if self.config.strict_mode {
            for pattern in &self.config.blocked_patterns {
                if input.to_lowercase().contains(&pattern.to_lowercase()) {
                    return Err(SandboxError::BlockedPattern {
                        pattern: pattern.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Check analysis limits
    fn check_limits(&self) -> Result<(), SandboxError> {
        let active_count = {
            let analyses = self.active_analyses.read().unwrap();
            analyses.len()
        };

        if active_count >= self.config.max_concurrent_analyses {
            return Err(SandboxError::TooManyAnalyses {
                current: active_count,
                limit: self.config.max_concurrent_analyses,
            });
        }

        Ok(())
    }

    /// Start analysis with resource tracking
    fn start_analysis(&self) -> Result<Arc<AnalysisContext>, SandboxError> {
        self.check_limits()?;

        let context = Arc::new(AnalysisContext::new());
        let analysis_id = context.analysis_id;

        {
            let mut analyses = self.active_analyses.write().unwrap();
            analyses.insert(analysis_id, context.clone());
        }

        self.analysis_counter.fetch_add(1, Ordering::Relaxed);
        Ok(context)
    }

    /// Finish analysis and cleanup
    fn finish_analysis(&self, context: &AnalysisContext) {
        let mut analyses = self.active_analyses.write().unwrap();
        analyses.remove(&context.analysis_id);
    }

    /// Monitor analysis with timeout and resource checks
    fn monitor_analysis<F, R>(&self, context: &AnalysisContext, f: F) -> Result<R, SandboxError>
    where
        F: FnOnce() -> Result<R, SandboxError>,
    {
        // Check timeout before starting
        if context.is_timed_out(self.config.max_analysis_time) {
            context.mark_timeout();
            return Err(SandboxError::Timeout {
                timeout_secs: self.config.max_analysis_time.as_secs(),
            });
        }

        // Check cancellation
        if context.is_cancelled() {
            return Err(SandboxError::Cancelled);
        }

        // Run the analysis
        let result = f();

        // Update status based on result
        match &result {
            Ok(_) => context.mark_completed(),
            Err(_) => context.mark_failed(),
        }

        result
    }

    /// Perform lexical analysis
    pub fn analyze_lexical(&self, input: &str) -> Result<AnalysisResult, SandboxError> {
        self.validate_input(input, None)?;
        let context = self.start_analysis()?;

        let result = self.monitor_analysis(&context, || {
            let lexer = Lexer::new(input).map_err(SandboxError::from)?;

            // Update memory usage estimate
            context.update_memory(input.len() * 2); // Rough estimate

            // Check for cancellation before processing
            if context.is_cancelled() {
                return Err(SandboxError::Cancelled);
            }

            // Check timeout
            if context.is_timed_out(self.config.max_analysis_time) {
                return Err(SandboxError::Timeout {
                    timeout_secs: self.config.max_analysis_time.as_secs(),
                });
            }

            let (tokens, errors) = lexer.scan_tokens();
            let has_errors = !errors.is_empty();
            let error_messages: Vec<String> = errors.iter().map(|e| format!("{}", e)).collect();

            let token_strings: Vec<String> = tokens.iter().map(|t| format!("{:?}", t)).collect();

            // Check memory usage
            let estimated_memory = token_strings.len() * 50; // Rough estimate per token
            context.update_memory(estimated_memory);
            if estimated_memory > self.config.max_memory_usage {
                return Err(SandboxError::MemoryLimitExceeded {
                    usage: estimated_memory,
                    limit: self.config.max_memory_usage,
                });
            }

            Ok(AnalysisResult::Lexical {
                tokens: token_strings.clone(),
                token_count: token_strings.len(),
                has_errors,
                error_messages,
            })
        });

        self.finish_analysis(&context);
        result
    }

    /// Perform parse analysis
    pub fn analyze_parse(&self, input: &str) -> Result<AnalysisResult, SandboxError> {
        self.validate_input(input, None)?;
        let context = self.start_analysis()?;

        let result = self.monitor_analysis(&context, || {
            let lexer = Lexer::new(input).map_err(SandboxError::from)?;
            let (tokens, lexer_errors) = lexer.scan_tokens();

            let mut parser = Parser::new(tokens);

            // Update memory usage estimate
            context.update_memory(input.len() * 3); // Rough estimate for AST

            let program_result = parser.parse();
            let mut error_messages = Vec::new();
            let mut has_errors = !lexer_errors.is_empty();

            // Add lexer errors
            for error in &lexer_errors {
                error_messages.push(format!("{}", error));
            }

            let (ast_summary, node_count) = match program_result {
                Ok(program) => {
                    let node_count = Self::count_ast_nodes(&program);
                    let summary = format!("Program with {} statements", program.statements.len());

                    // Check memory usage
                    let estimated_memory = node_count * 100; // Rough estimate per node
                    context.update_memory(estimated_memory);
                    if estimated_memory > self.config.max_memory_usage {
                        return Err(SandboxError::MemoryLimitExceeded {
                            usage: estimated_memory,
                            limit: self.config.max_memory_usage,
                        });
                    }

                    (summary, node_count)
                }
                Err(parse_error) => {
                    has_errors = true;
                    error_messages.push(format!("{}", parse_error));
                    ("Failed to parse".to_string(), 0)
                }
            };

            Ok(AnalysisResult::Parse {
                ast_summary,
                node_count,
                has_errors,
                error_messages,
            })
        });

        self.finish_analysis(&context);
        result
    }

    /// Perform semantic analysis
    pub fn analyze_semantic(&self, input: &str) -> Result<AnalysisResult, SandboxError> {
        self.validate_input(input, None)?;
        let context = self.start_analysis()?;

        let result = self.monitor_analysis(&context, || {
            let lexer = Lexer::new(input).map_err(SandboxError::from)?;
            let (tokens, lexer_errors) = lexer.scan_tokens();

            let mut parser = Parser::new(tokens);
            let program = parser.parse().map_err(SandboxError::from)?;

            // Update memory usage estimate
            context.update_memory(input.len() * 4); // Rough estimate for semantic analysis

            let mut analyzer = SemanticAnalyzer::new();

            // Perform semantic analysis
            let analysis_result = analyzer.analyze_program(&program);

            let mut error_messages = Vec::new();
            let mut has_errors = !lexer_errors.is_empty();

            // Add lexer errors
            for error in &lexer_errors {
                error_messages.push(format!("{}", error));
            }

            if let Err(ref error) = analysis_result {
                has_errors = true;
                error_messages.push(format!("{}", error));
            }

            let (type_info, symbol_count) = match analysis_result {
                Ok(_) => {
                    let symbol_table = analyzer.symbol_table();
                    let symbol_count = symbol_table.all_symbols().count();

                    // Extract type information
                    let mut type_info = HashMap::new();
                    for (name, symbol) in symbol_table.all_symbols() {
                        type_info.insert(name.clone(), format!("{:?}", symbol.ty));
                    }

                    // Check memory usage
                    let estimated_memory = symbol_count * 200; // Rough estimate per symbol
                    context.update_memory(estimated_memory);
                    if estimated_memory > self.config.max_memory_usage {
                        return Err(SandboxError::MemoryLimitExceeded {
                            usage: estimated_memory,
                            limit: self.config.max_memory_usage,
                        });
                    }

                    (type_info, symbol_count)
                }
                Err(_) => (HashMap::new(), 0),
            };

            Ok(AnalysisResult::Semantic {
                type_info,
                symbol_count,
                has_errors,
                error_messages,
            })
        });

        self.finish_analysis(&context);
        result
    }

    /// Perform code quality analysis
    pub fn analyze_quality(&self, input: &str) -> Result<AnalysisResult, SandboxError> {
        self.validate_input(input, None)?;
        let context = self.start_analysis()?;

        let result = self.monitor_analysis(&context, || {
            // Update memory usage estimate
            context.update_memory(input.len() * 2);

            // Simple quality metrics
            let line_count = input.lines().count();
            let char_count = input.chars().count();
            let avg_line_length = if line_count > 0 {
                char_count as f64 / line_count as f64
            } else {
                0.0
            };

            // Calculate basic metrics
            let complexity_score = Self::calculate_complexity_score(input);
            let maintainability_score =
                Self::calculate_maintainability_score(input, avg_line_length);
            let security_score =
                Self::calculate_security_score(input, &self.config.blocked_patterns);

            let suggestions = Self::generate_quality_suggestions(
                complexity_score,
                maintainability_score,
                security_score,
                avg_line_length,
            );

            Ok(AnalysisResult::Quality {
                complexity_score,
                maintainability_score,
                security_score,
                suggestions,
            })
        });

        self.finish_analysis(&context);
        result
    }

    /// Perform dependency analysis
    pub fn analyze_dependencies(&self, input: &str) -> Result<AnalysisResult, SandboxError> {
        self.validate_input(input, None)?;
        let context = self.start_analysis()?;

        let result = self.monitor_analysis(&context, || {
            let lexer = Lexer::new(input).map_err(SandboxError::from)?;
            let (tokens, _lexer_errors) = lexer.scan_tokens();

            let mut parser = Parser::new(tokens);
            let program = parser.parse().map_err(SandboxError::from)?;

            // Update memory usage estimate
            context.update_memory(input.len() * 3);

            let analyzer = DependencyAnalyzer::new();
            let dependencies = analyzer.analyze(&program, None);

            // Convert HashSet to Vec for JSON serialization
            let imports: Vec<String> = dependencies.into_iter().collect();
            let exports = Vec::new(); // Placeholder for exports
            let dep_map = HashMap::new(); // Placeholder for dependency map

            Ok(AnalysisResult::Dependencies {
                imports,
                exports,
                dependency_graph: dep_map,
            })
        });

        self.finish_analysis(&context);
        result
    }

    /// Cancel analysis by ID
    pub fn cancel_analysis(&self, analysis_id: Uuid) -> bool {
        let analyses = self.active_analyses.read().unwrap();
        if let Some(context) = analyses.get(&analysis_id) {
            context.cancel();
            true
        } else {
            false
        }
    }

    /// Get active analysis count
    pub fn active_analysis_count(&self) -> usize {
        let analyses = self.active_analyses.read().unwrap();
        analyses.len()
    }

    /// Get total analysis count
    pub fn total_analysis_count(&self) -> usize {
        self.analysis_counter.load(Ordering::Relaxed)
    }

    /// Cleanup expired analyses
    pub fn cleanup_expired_analyses(&self) -> usize {
        let mut analyses = self.active_analyses.write().unwrap();
        let initial_count = analyses.len();

        analyses.retain(|_, context| {
            if context.is_timed_out(self.config.max_analysis_time) {
                context.mark_timeout();
                false
            } else {
                true
            }
        });

        initial_count - analyses.len()
    }

    // Helper methods for quality analysis

    fn count_ast_nodes(program: &Program) -> usize {
        // Simple node counting - in a real implementation, this would recursively count all nodes
        program.statements.len() * 10 // Rough estimate
    }

    fn calculate_complexity_score(input: &str) -> f64 {
        let line_count = input.lines().count() as f64;
        let nesting_level = Self::calculate_max_nesting(input) as f64;
        let condition_count = Self::count_conditions(input) as f64;

        // Simple complexity formula
        let base_score = 100.0;
        let complexity_penalty =
            (nesting_level * 5.0) + (condition_count * 2.0) + (line_count * 0.1);
        (base_score - complexity_penalty).max(0.0).min(100.0)
    }

    fn calculate_maintainability_score(input: &str, avg_line_length: f64) -> f64 {
        let line_count = input.lines().count() as f64;
        let comment_ratio = Self::calculate_comment_ratio(input);

        let base_score = 100.0;
        let length_penalty = if avg_line_length > 80.0 { 10.0 } else { 0.0 };
        let size_penalty = if line_count > 500.0 { 20.0 } else { 0.0 };
        let comment_bonus = comment_ratio * 20.0;

        (base_score - length_penalty - size_penalty + comment_bonus)
            .max(0.0)
            .min(100.0)
    }

    fn calculate_security_score(input: &str, blocked_patterns: &[String]) -> f64 {
        let mut violations = 0;
        for pattern in blocked_patterns {
            if input.to_lowercase().contains(&pattern.to_lowercase()) {
                violations += 1;
            }
        }

        let base_score = 100.0;
        let security_penalty = violations as f64 * 20.0;
        (base_score - security_penalty).max(0.0).min(100.0)
    }

    fn calculate_max_nesting(input: &str) -> u32 {
        let mut max_nesting = 0;
        let mut current_nesting: u32 = 0;

        for line in input.lines() {
            let trimmed = line.trim();
            if trimmed.contains('{') {
                current_nesting += 1;
                max_nesting = max_nesting.max(current_nesting);
            }
            if trimmed.contains('}') {
                current_nesting = current_nesting.saturating_sub(1);
            }
        }

        max_nesting
    }

    fn count_conditions(input: &str) -> u32 {
        let mut count = 0;
        for line in input.lines() {
            if line.contains("if ") || line.contains("while ") || line.contains("for ") {
                count += 1;
            }
        }
        count
    }

    fn calculate_comment_ratio(input: &str) -> f64 {
        let total_lines = input.lines().count() as f64;
        if total_lines == 0.0 {
            return 0.0;
        }

        let comment_lines = input
            .lines()
            .filter(|line| line.trim_start().starts_with("//"))
            .count() as f64;

        comment_lines / total_lines
    }

    fn generate_quality_suggestions(
        complexity: f64,
        maintainability: f64,
        security: f64,
        avg_line_length: f64,
    ) -> Vec<String> {
        let mut suggestions = Vec::new();

        if complexity < 60.0 {
            suggestions.push(
                "Consider reducing code complexity by breaking down large functions".to_string(),
            );
        }

        if maintainability < 70.0 {
            suggestions.push(
                "Improve maintainability by adding comments and reducing function size".to_string(),
            );
        }

        if security < 80.0 {
            suggestions.push(
                "Review code for potential security issues and remove dangerous patterns"
                    .to_string(),
            );
        }

        if avg_line_length > 80.0 {
            suggestions.push("Consider breaking long lines to improve readability".to_string());
        }

        if suggestions.is_empty() {
            suggestions.push("Code quality looks good!".to_string());
        }

        suggestions
    }
}

/// Sandbox statistics
#[derive(Debug, Clone)]
pub struct SandboxStats {
    pub active_analyses: usize,
    pub total_analyses: usize,
    pub total_memory_usage: usize,
}

impl SandboxedAnalyzer {
    /// Get sandbox statistics
    pub fn get_stats(&self) -> SandboxStats {
        let analyses = self.active_analyses.read().unwrap();
        let total_memory: usize = analyses.values().map(|ctx| ctx.get_memory_usage()).sum();

        SandboxStats {
            active_analyses: analyses.len(),
            total_analyses: self.total_analysis_count(),
            total_memory_usage: total_memory,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_config_default() {
        let config = SandboxConfig::default();
        assert_eq!(config.max_analysis_time, MAX_ANALYSIS_TIME);
        assert_eq!(config.max_memory_usage, MAX_MEMORY_USAGE);
        assert_eq!(config.max_input_size, MAX_INPUT_SIZE);
        assert!(config.strict_mode);
    }

    #[test]
    fn test_analysis_context() {
        let context = AnalysisContext::new();
        assert!(!context.is_timed_out(Duration::from_secs(1)));
        assert_eq!(context.get_memory_usage(), 0);
        assert!(!context.is_cancelled());

        context.update_memory(1024);
        assert_eq!(context.get_memory_usage(), 1024);

        context.cancel();
        assert!(context.is_cancelled());
    }

    #[test]
    fn test_input_validation() {
        let config = SandboxConfig::default();
        let analyzer = SandboxedAnalyzer::new(config);

        // Valid input
        assert!(analyzer.validate_input("let x = 5", None).is_ok());

        // Input too large
        let large_input = "x".repeat(MAX_INPUT_SIZE + 1);
        assert!(analyzer.validate_input(&large_input, None).is_err());

        // Blocked pattern
        assert!(analyzer
            .validate_input("eval('malicious code')", None)
            .is_err());
    }

    #[test]
    fn test_lexical_analysis() {
        let config = SandboxConfig::default();
        let analyzer = SandboxedAnalyzer::new(config);

        let input = "let x = 42\nprint(x)";
        let result = analyzer.analyze_lexical(input).unwrap();

        match result {
            AnalysisResult::Lexical {
                tokens,
                token_count,
                has_errors,
                ..
            } => {
                assert!(token_count > 0);
                assert!(!tokens.is_empty());
                assert!(!has_errors);
            }
            _ => panic!("Expected lexical result"),
        }
    }

    #[test]
    fn test_parse_analysis() {
        let config = SandboxConfig::default();
        let analyzer = SandboxedAnalyzer::new(config);

        let input = "let x = 42\nprint(x)";
        let result = analyzer.analyze_parse(input).unwrap();

        match result {
            AnalysisResult::Parse {
                node_count,
                has_errors,
                ..
            } => {
                assert!(node_count > 0);
                assert!(!has_errors);
            }
            _ => panic!("Expected parse result"),
        }
    }

    #[test]
    fn test_quality_analysis() {
        let config = SandboxConfig::default();
        let analyzer = SandboxedAnalyzer::new(config);

        let input = "// Good code\nlet x = 42\nprint(x)";
        let result = analyzer.analyze_quality(input).unwrap();

        match result {
            AnalysisResult::Quality {
                complexity_score,
                maintainability_score,
                security_score,
                suggestions,
            } => {
                assert!(complexity_score >= 0.0 && complexity_score <= 100.0);
                assert!(maintainability_score >= 0.0 && maintainability_score <= 100.0);
                assert!(security_score >= 0.0 && security_score <= 100.0);
                assert!(!suggestions.is_empty());
            }
            _ => panic!("Expected quality result"),
        }
    }

    #[test]
    fn test_concurrent_analysis_limit() {
        let mut config = SandboxConfig::default();
        config.max_concurrent_analyses = 1;
        let analyzer = SandboxedAnalyzer::new(config);

        // First analysis should succeed
        let _context1 = analyzer.start_analysis().unwrap();

        // Second analysis should fail due to limit
        assert!(analyzer.start_analysis().is_err());
    }

    #[test]
    fn test_sandbox_stats() {
        let config = SandboxConfig::default();
        let analyzer = SandboxedAnalyzer::new(config);

        let stats = analyzer.get_stats();
        assert_eq!(stats.active_analyses, 0);
        assert_eq!(stats.total_analyses, 0);
        assert_eq!(stats.total_memory_usage, 0);

        // Start an analysis
        let _context = analyzer.start_analysis().unwrap();

        let stats = analyzer.get_stats();
        assert_eq!(stats.active_analyses, 1);
        assert_eq!(stats.total_analyses, 1);
    }
}
