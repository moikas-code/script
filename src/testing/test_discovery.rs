use crate::error::{Error, Result};
use crate::parser::{Attribute, Program, Stmt, StmtKind};
use crate::testing::{TestCase, TestSuite};

/// Discovers and collects tests from a program
pub struct TestDiscovery {
    /// Filter for test names (if any)
    name_filter: Option<String>,
    /// Filter for test tags
    tag_filter: Vec<String>,
}

impl TestDiscovery {
    pub fn new() -> Self {
        Self {
            name_filter: None,
            tag_filter: Vec::new(),
        }
    }

    pub fn with_name_filter(mut self, filter: String) -> Self {
        self.name_filter = Some(filter);
        self
    }

    pub fn with_tag_filter(mut self, tags: Vec<String>) -> Self {
        self.tag_filter = tags;
        self
    }

    /// Discover all tests in a program
    pub fn discover_tests(&self, program: &Program) -> Result<TestSuite> {
        let mut collector = TestCollector::new();
        collector.collect_from_program(program)?;

        let mut tests = collector.tests;

        // Apply filters
        if let Some(name_filter) = &self.name_filter {
            tests.retain(|test| test.name.contains(name_filter));
        }

        if !self.tag_filter.is_empty() {
            tests.retain(|test| {
                self.tag_filter
                    .iter()
                    .any(|tag| test.attributes.tags.contains(tag))
            });
        }

        Ok(TestSuite {
            name: "main".to_string(),
            tests,
            setup: collector.setup,
            teardown: collector.teardown,
        })
    }
}

/// Collects tests from AST nodes
pub struct TestCollector {
    pub tests: Vec<TestCase>,
    pub setup: Option<Stmt>,
    pub teardown: Option<Stmt>,
    current_module: String,
}

impl TestCollector {
    pub fn new() -> Self {
        Self {
            tests: Vec::new(),
            setup: None,
            teardown: None,
            current_module: String::new(),
        }
    }

    /// Collect tests from a program
    pub fn collect_from_program(&mut self, program: &Program) -> Result<()> {
        for stmt in &program.statements {
            self.visit_statement(stmt)?;
        }
        Ok(())
    }

    /// Visit a statement and collect any tests
    fn visit_statement(&mut self, stmt: &Stmt) -> Result<()> {
        // Check for test functions
        if let Some(test_case) = self.check_for_test(stmt) {
            self.tests.push(test_case);
            return Ok(());
        }

        // Check for setup/teardown functions
        if self.check_for_setup(stmt) {
            if self.setup.is_some() {
                return Err(Error::semantic(
                    "Multiple @setup functions found".to_string(),
                ));
            }
            self.setup = Some(stmt.clone());
            return Ok(());
        }

        if self.check_for_teardown(stmt) {
            if self.teardown.is_some() {
                return Err(Error::semantic(
                    "Multiple @teardown functions found".to_string(),
                ));
            }
            self.teardown = Some(stmt.clone());
            return Ok(());
        }

        // Recursively check in blocks
        match &stmt.kind {
            StmtKind::Expression(expr) => {
                if let crate::parser::ExprKind::Block(block) = &expr.kind {
                    for s in &block.statements {
                        self.visit_statement(s)?;
                    }
                }
            }
            StmtKind::While { body, .. } | StmtKind::For { body, .. } => {
                for s in &body.statements {
                    self.visit_statement(s)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Check if a statement is a test function
    fn check_for_test(&self, stmt: &Stmt) -> Option<TestCase> {
        TestCase::from_function(stmt)
    }

    /// Check if a statement is a setup function
    fn check_for_setup(&self, stmt: &Stmt) -> bool {
        stmt.attributes.iter().any(|attr| attr.name == "setup")
    }

    /// Check if a statement is a teardown function
    fn check_for_teardown(&self, stmt: &Stmt) -> bool {
        stmt.attributes.iter().any(|attr| attr.name == "teardown")
    }
}

/// Test module organization
pub struct TestModule {
    pub name: String,
    pub path: String,
    pub suites: Vec<TestSuite>,
}

impl TestModule {
    /// Discover tests in multiple files
    pub fn discover_in_directory(path: &str) -> Result<Vec<TestModule>> {
        use std::fs;
        use std::path::Path;

        let mut modules = Vec::new();

        // Find all .script files in directory
        let dir = Path::new(path);
        if !dir.is_dir() {
            return Err(Error::io(format!("Not a directory: {}", path)));
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("script") {
                // Skip non-test files
                let file_name = path.file_name().unwrap().to_str().unwrap();
                if !file_name.contains("test") && !file_name.contains("spec") {
                    continue;
                }

                // Parse and discover tests
                let source = fs::read_to_string(&path)?;
                let lexer = crate::Lexer::new(&source);
                let (tokens, errors) = lexer.scan_tokens();

                if !errors.is_empty() {
                    continue; // Skip files with lexer errors
                }

                let mut parser = crate::Parser::new(tokens);
                if let Ok(program) = parser.parse() {
                    let discovery = TestDiscovery::new();
                    if let Ok(suite) = discovery.discover_tests(&program) {
                        if !suite.tests.is_empty() {
                            modules.push(TestModule {
                                name: file_name.to_string(),
                                path: path.to_string_lossy().to_string(),
                                suites: vec![suite],
                            });
                        }
                    }
                }
            }
        }

        Ok(modules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Block;
    use crate::source::Span;

    fn create_test_stmt(name: &str) -> Stmt {
        Stmt {
            kind: StmtKind::Function {
                name: name.to_string(),
                params: vec![],
                ret_type: None,
                body: Block {
                    statements: vec![],
                    final_expr: None,
                },
                is_async: false,
                generic_params: None,
            },
            span: Span::dummy(),
            attributes: vec![Attribute {
                name: "test".to_string(),
                args: vec![],
                span: Span::dummy(),
            }],
        }
    }

    #[test]
    fn test_collector_finds_test_functions() {
        let program = Program {
            statements: vec![create_test_stmt("test_foo"), create_test_stmt("test_bar")],
        };

        let mut collector = TestCollector::new();
        collector.collect_from_program(&program).unwrap();

        assert_eq!(collector.tests.len(), 2);
        assert_eq!(collector.tests[0].name, "test_foo");
        assert_eq!(collector.tests[1].name, "test_bar");
    }
}
