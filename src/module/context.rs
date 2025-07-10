//! Enhanced module context for rich error reporting and debugging
//!
//! This module provides comprehensive context tracking for module operations,
//! enabling detailed error messages and import chain visualization.

use crate::module::{ImportPath, ModuleError, ModulePath};
use crate::source::Span;
use std::collections::HashMap;
use std::path::PathBuf;

/// Import resolution step in the module loading process
#[derive(Debug, Clone)]
pub struct ImportResolutionStep {
    /// The import statement that triggered this resolution
    pub import_stmt: ImportPath,
    /// The module that contains the import statement
    pub importing_module: ModulePath,
    /// The resolved module path (if successful)
    pub resolved_path: Option<ModulePath>,
    /// The span in source code where this import appears
    pub span: Span,
    /// Error that occurred during resolution (if any)
    pub error: Option<String>,
    /// Time taken for resolution (in microseconds)
    pub resolution_time: u64,
}

/// Module dependency chain for tracking import relationships
#[derive(Debug, Clone)]
pub struct ModuleDependencyChain {
    /// The chain of modules from root to current
    pub chain: Vec<ModulePath>,
    /// Import statements that connect the chain
    pub imports: Vec<ImportPath>,
    /// Source locations for each import
    pub locations: Vec<Span>,
}

impl ModuleDependencyChain {
    /// Create a new dependency chain starting from a root module
    pub fn new(root: ModulePath) -> Self {
        ModuleDependencyChain {
            chain: vec![root],
            imports: Vec::new(),
            locations: Vec::new(),
        }
    }

    /// Add a new module to the chain
    pub fn push(&mut self, module: ModulePath, import: ImportPath, location: Span) {
        self.chain.push(module);
        self.imports.push(import);
        self.locations.push(location);
    }

    /// Create a sub-chain from the current chain
    pub fn extend(&self, module: ModulePath, import: ImportPath, location: Span) -> Self {
        let mut new_chain = self.clone();
        new_chain.push(module, import, location);
        new_chain
    }

    /// Check if adding a module would create a cycle
    pub fn would_create_cycle(&self, module: &ModulePath) -> bool {
        self.chain.contains(module)
    }

    /// Get a formatted string representation of the chain
    pub fn format_chain(&self) -> String {
        let mut result = String::new();
        for (i, module) in self.chain.iter().enumerate() {
            result.push_str(&format!("{}{}", "  ".repeat(i), module));
            if i < self.imports.len() {
                result.push_str(&format!(" imports {}", self.imports[i]));
            }
            result.push('\n');
        }
        result
    }
}

/// Enhanced module context for comprehensive error reporting
#[derive(Debug)]
pub struct ModuleContext {
    /// Current module being processed
    pub current_module: ModulePath,
    /// Full dependency chain from root
    pub dependency_chain: ModuleDependencyChain,
    /// Import resolution history
    pub resolution_history: Vec<ImportResolutionStep>,
    /// Module source file mapping
    pub source_files: HashMap<ModulePath, PathBuf>,
    /// Cached source content for error display
    pub source_cache: HashMap<PathBuf, String>,
    /// Symbol visibility context
    pub visibility_context: VisibilityContext,
}

impl ModuleContext {
    /// Create a new module context
    pub fn new(module: ModulePath) -> Self {
        ModuleContext {
            current_module: module.clone(),
            dependency_chain: ModuleDependencyChain::new(module),
            resolution_history: Vec::new(),
            source_files: HashMap::new(),
            source_cache: HashMap::new(),
            visibility_context: VisibilityContext::new(),
        }
    }

    /// Create a child context for a sub-module
    pub fn create_child(&self, module: ModulePath, import: ImportPath, location: Span) -> Self {
        ModuleContext {
            current_module: module.clone(),
            dependency_chain: self.dependency_chain.extend(module, import, location),
            resolution_history: self.resolution_history.clone(),
            source_files: self.source_files.clone(),
            source_cache: self.source_cache.clone(),
            visibility_context: self.visibility_context.clone(),
        }
    }

    /// Record an import resolution attempt
    pub fn record_resolution(
        &mut self,
        import: ImportPath,
        resolved: Option<ModulePath>,
        span: Span,
        error: Option<String>,
        time_us: u64,
    ) {
        self.resolution_history.push(ImportResolutionStep {
            import_stmt: import,
            importing_module: self.current_module.clone(),
            resolved_path: resolved,
            span,
            error,
            resolution_time: time_us,
        });
    }

    /// Get source content with caching
    pub fn get_source_content(&mut self, file: &PathBuf) -> Option<&str> {
        if !self.source_cache.contains_key(file) {
            if let Ok(content) = std::fs::read_to_string(file) {
                self.source_cache.insert(file.clone(), content);
            }
        }
        self.source_cache.get(file).map(|s| s.as_str())
    }

    /// Format an error with full module context
    pub fn format_error(&mut self, error: &ModuleError, span: Span) -> String {
        let mut output = String::new();

        // Error message
        output.push_str(format!("error: {}\n", error));

        // Source location with context
        if let Some(file) = self.source_files.get(&self.current_module).cloned() {
            if let Some(source) = self.get_source_content(&file) {
                output.push_str(&format_source_context(source, span));
            }
        }

        // Import chain if relevant
        if self.dependency_chain.chain.len() > 1 {
            output.push_str("\nImport chain:\n");
            output.push_str(&self.dependency_chain.format_chain());
        }

        // Recent resolution attempts if relevant
        if !self.resolution_history.is_empty() {
            output.push_str("\nRecent import resolutions:\n");
            for step in self.resolution_history.iter().rev().take(5) {
                output.push_str(format!("  {} → ", step.import_stmt));
                if let Some(resolved) = &step.resolved_path {
                    output.push_str(format!("{} ({}μs, step.resolution_time)\n", resolved));
                } else if let Some(error) = &step.error {
                    output.push_str(format!("failed: {}\n", error));
                } else {
                    output.push_str("unresolved\n");
                }
            }
        }

        output
    }
}

/// Symbol visibility tracking for better error messages
#[derive(Debug, Clone)]
pub struct VisibilityContext {
    /// Exported symbols by module
    pub exports: HashMap<ModulePath, Vec<String>>,
    /// Private symbols that were attempted to be accessed
    pub access_attempts: Vec<PrivateAccessAttempt>,
}

impl VisibilityContext {
    pub fn new() -> Self {
        VisibilityContext {
            exports: HashMap::new(),
            access_attempts: Vec::new(),
        }
    }

    /// Record exported symbols for a module
    pub fn record_exports(&mut self, module: ModulePath, symbols: Vec<String>) {
        self.exports.insert(module, symbols);
    }

    /// Record an attempt to access a private symbol
    pub fn record_private_access(&mut self, attempt: PrivateAccessAttempt) {
        self.access_attempts.push(attempt);
    }
}

/// Record of an attempt to access a private symbol
#[derive(Debug, Clone)]
pub struct PrivateAccessAttempt {
    pub symbol: String,
    pub from_module: ModulePath,
    pub target_module: ModulePath,
    pub location: Span,
}

/// Format source code context around a span
fn format_source_context(source: &str, span: Span) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let start_line = span.start.line.saturating_sub(1);
    let end_line = (span.end.line as usize).min(lines.len());

    let mut output = String::new();

    // Show a few lines of context
    let context_start = start_line.saturating_sub(2);
    let context_end = (end_line + 2).min(lines.len());

    for (i, line) in lines[context_start..context_end].iter().enumerate() {
        let line_num = context_start + i + 1;
        output.push_str(format!("{:4} | {}\n", line_num, line));

        // Highlight the error span
        if line_num >= span.start.line as usize && line_num <= span.end.line as usize {
            let start_col = if line_num == span.start.line as usize {
                span.start.column as usize
            } else {
                0
            };
            let end_col = if line_num == span.end.line as usize {
                span.end.column as usize
            } else {
                line.len()
            };

            output.push_str("     | ");
            output.push_str(&" ".repeat(start_col));
            output.push_str(&"^".repeat((end_col - start_col).max(1)));
            output.push('\n');
        }
    }

    output
}

/// Module context stack for tracking nested contexts
pub struct ModuleContextStack {
    contexts: Vec<ModuleContext>,
}

impl ModuleContextStack {
    pub fn new() -> Self {
        ModuleContextStack {
            contexts: Vec::new(),
        }
    }

    /// Push a new context onto the stack
    pub fn push(&mut self, context: ModuleContext) {
        self.contexts.push(context);
    }

    /// Pop the current context
    pub fn pop(&mut self) -> Option<ModuleContext> {
        self.contexts.pop()
    }

    /// Get the current context
    pub fn current(&self) -> Option<&ModuleContext> {
        self.contexts.last()
    }

    /// Get the current context mutably
    pub fn current_mut(&mut self) -> Option<&mut ModuleContext> {
        self.contexts.last_mut()
    }

    /// Get the full context stack for debugging
    pub fn stack(&self) -> &[ModuleContext] {
        &self.contexts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceLocation;

    #[test]
    fn test_dependency_chain() {
        let root = ModulePath::from_string("app.main").unwrap();
        let mut chain = ModuleDependencyChain::new(root.clone());

        let utils = ModulePath::from_string("app.utils").unwrap();
        let import = ImportPath::new("utils").unwrap();
        let span = Span::new(SourceLocation::new(1, 1, 0), SourceLocation::new(1, 10, 9));

        chain.push(utils.clone(), import, span);

        assert_eq!(chain.chain.len(), 2);
        assert!(!chain.would_create_cycle(&ModulePath::from_string("app.helpers").unwrap()));
        assert!(chain.would_create_cycle(&root));
        assert!(chain.would_create_cycle(&utils));
    }

    #[test]
    fn test_module_context() {
        let module = ModulePath::from_string("test.module").unwrap();
        let mut ctx = ModuleContext::new(module.clone());

        // Record a resolution
        let import = ImportPath::new("dependency").unwrap();
        let resolved = ModulePath::from_string("test.dependency").unwrap();
        let span = Span::new(SourceLocation::new(1, 1, 0), SourceLocation::new(1, 20, 19));

        ctx.record_resolution(import, Some(resolved), span, None, 100);

        assert_eq!(ctx.resolution_history.len(), 1);
        assert_eq!(ctx.resolution_history[0].resolution_time, 100);
    }
}
