mod assertions;
mod test_case;
mod test_discovery;
mod test_reporter;
mod test_runner;

pub use assertions::{Assertion, AssertionError};
pub use test_case::{TestCase, TestFailure, TestResult, TestStatus};
pub use test_discovery::{TestCollector, TestDiscovery};
pub use test_reporter::{ConsoleReporter, ReportFormat, TestReporter};
pub use test_runner::{TestRunOptions, TestRunner, TestSuite};

use crate::error::Result;
use crate::parser::Stmt;
use std::fmt;
use std::time::Duration;

/// Main testing framework entry point
pub struct TestingFramework {
    discovery: TestDiscovery,
    runner: TestRunner,
    reporter: Box<dyn TestReporter>,
}

impl TestingFramework {
    pub fn new() -> Self {
        Self {
            discovery: TestDiscovery::new(),
            runner: TestRunner::new(),
            reporter: Box::new(ConsoleReporter::new()),
        }
    }

    pub fn with_reporter(mut self, reporter: Box<dyn TestReporter>) -> Self {
        self.reporter = reporter;
        self
    }

    /// Run all tests in a program
    pub fn run_tests(&mut self, program: &crate::parser::Program) -> Result<TestSummary> {
        // Discover tests
        let suite = self.discovery.discover_tests(program)?;

        // Run tests
        let results = self.runner.run_suite(&suite)?;

        // Report results
        self.reporter.report_results(&results)?;

        // Return summary
        Ok(TestSummary::from_results(&results))
    }

    /// Check if a statement is a test function
    pub fn is_test_function(stmt: &Stmt) -> bool {
        stmt.attributes.iter().any(|attr| attr.name == "test")
    }
}

/// Summary of test execution
#[derive(Debug, Clone)]
pub struct TestSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration: Duration,
}

impl TestSummary {
    pub fn from_results(results: &[TestResult]) -> Self {
        let total = results.len();
        let passed = results
            .iter()
            .filter(|r| matches!(r.status, TestStatus::Passed))
            .count();
        let failed = results
            .iter()
            .filter(|r| matches!(r.status, TestStatus::Failed(_)))
            .count();
        let skipped = results
            .iter()
            .filter(|r| matches!(r.status, TestStatus::Skipped(_)))
            .count();
        let duration = results.iter().map(|r| r.duration).sum();

        Self {
            total,
            passed,
            failed,
            skipped,
            duration,
        }
    }

    pub fn all_passed(&self) -> bool {
        self.failed == 0 && self.total > 0
    }
}

impl fmt::Display for TestSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Test Summary: {} total, {} passed, {} failed, {} skipped ({:.2}s)",
            self.total,
            self.passed,
            self.failed,
            self.skipped,
            self.duration.as_secs_f64()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_all_passed() {
        let summary = TestSummary {
            total: 10,
            passed: 10,
            failed: 0,
            skipped: 0,
            duration: Duration::from_millis(100),
        };
        assert!(summary.all_passed());
    }

    #[test]
    fn test_summary_with_failures() {
        let summary = TestSummary {
            total: 10,
            passed: 7,
            failed: 2,
            skipped: 1,
            duration: Duration::from_millis(200),
        };
        assert!(!summary.all_passed());
    }
}
