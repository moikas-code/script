use crate::parser::{Block, Stmt, StmtKind};
use crate::source::Span;
use std::fmt;
use std::time::Duration;

/// Represents a single test case
#[derive(Debug, Clone)]
pub struct TestCase {
    /// Name of the test function
    pub name: String,
    /// The test function body
    pub body: Block,
    /// Source location of the test
    pub span: Span,
    /// Test attributes (e.g., @test(skip = "reason"))
    pub attributes: TestAttributes,
}

/// Test attributes parsed from @test decorator
#[derive(Debug, Clone, Default)]
pub struct TestAttributes {
    /// Whether to skip this test
    pub skip: Option<String>,
    /// Expected panic message
    pub should_panic: Option<String>,
    /// Test timeout in milliseconds
    pub timeout: Option<u64>,
    /// Test tags for filtering
    pub tags: Vec<String>,
}

impl TestAttributes {
    pub fn from_attribute(attr: &crate::parser::Attribute) -> Self {
        let mut attrs = TestAttributes::default();

        // Parse attribute arguments
        for arg in &attr.args {
            if let Some((key, value)) = arg.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"');

                match key {
                    "skip" => attrs.skip = Some(value.to_string()),
                    "should_panic" => attrs.should_panic = Some(value.to_string()),
                    "timeout" => {
                        if let Ok(ms) = value.parse::<u64>() {
                            attrs.timeout = Some(ms);
                        }
                    }
                    "tag" => attrs.tags.push(value.to_string()),
                    _ => {} // Ignore unknown attributes
                }
            } else if arg.trim() == "should_panic" {
                attrs.should_panic = Some(String::new());
            }
        }

        attrs
    }
}

/// Result of running a test case
#[derive(Debug, Clone)]
pub struct TestResult {
    /// The test case that was run
    pub test: TestCase,
    /// Status of the test
    pub status: TestStatus,
    /// Time taken to run the test
    pub duration: Duration,
    /// Any output produced by the test
    pub output: String,
}

/// Status of a test execution
#[derive(Debug, Clone)]
pub enum TestStatus {
    /// Test passed successfully
    Passed,
    /// Test failed with an error
    Failed(TestFailure),
    /// Test was skipped
    Skipped(String),
    /// Test panicked
    Panicked(String),
}

/// Details about a test failure
#[derive(Debug, Clone)]
pub struct TestFailure {
    /// Error message
    pub message: String,
    /// Location where the failure occurred
    pub location: Option<Span>,
    /// Expected value (for assertions)
    pub expected: Option<String>,
    /// Actual value (for assertions)
    pub actual: Option<String>,
}

impl TestFailure {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            location: None,
            expected: None,
            actual: None,
        }
    }

    pub fn with_location(mut self, span: Span) -> Self {
        self.location = Some(span);
        self
    }

    pub fn with_comparison(
        mut self,
        expected: impl fmt::Display,
        actual: impl fmt::Display,
    ) -> Self {
        self.expected = Some(expected.to_string());
        self.actual = Some(actual.to_string());
        self
    }
}

impl fmt::Display for TestStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestStatus::Passed => write!(f, "PASSED"),
            TestStatus::Failed(failure) => write!(f, "FAILED: {}", failure.message),
            TestStatus::Skipped(reason) => write!(f, "SKIPPED: {}", reason),
            TestStatus::Panicked(msg) => write!(f, "PANICKED: {}", msg),
        }
    }
}

impl TestCase {
    /// Create a new test case from a function statement
    pub fn from_function(stmt: &Stmt) -> Option<Self> {
        if let StmtKind::Function { name, body, .. } = &stmt.kind {
            // Check for @test attribute
            let test_attr = stmt.attributes.iter().find(|attr| attr.name == "test")?;
            let attributes = TestAttributes::from_attribute(test_attr);

            Some(TestCase {
                name: name.clone(),
                body: body.clone(),
                span: stmt.span,
                attributes,
            })
        } else {
            None
        }
    }

    /// Check if this test should be skipped
    pub fn should_skip(&self) -> Option<&str> {
        self.attributes.skip.as_deref()
    }

    /// Check if this test should panic
    pub fn should_panic(&self) -> Option<&str> {
        self.attributes.should_panic.as_deref()
    }

    /// Get the timeout for this test
    pub fn timeout(&self) -> Duration {
        self.attributes
            .timeout
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_secs(60)) // Default 60 second timeout
    }
}
