use crate::error::Result;
use crate::parser::Stmt;
use crate::runtime::{Runtime, RuntimeConfig};
use crate::testing::{TestCase, TestFailure, TestResult, TestStatus};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Options for running tests
#[derive(Debug, Clone)]
pub struct TestRunOptions {
    /// Run tests in parallel
    pub parallel: bool,
    /// Number of threads for parallel execution
    pub threads: usize,
    /// Stop on first failure
    pub fail_fast: bool,
    /// Capture test output
    pub capture_output: bool,
    /// Verbose output
    pub verbose: bool,
}

impl Default for TestRunOptions {
    fn default() -> Self {
        Self {
            parallel: true,
            threads: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4),
            fail_fast: false,
            capture_output: true,
            verbose: false,
        }
    }
}

/// A test suite containing multiple test cases
#[derive(Debug, Clone)]
pub struct TestSuite {
    pub name: String,
    pub tests: Vec<TestCase>,
    pub setup: Option<Stmt>,
    pub teardown: Option<Stmt>,
}

/// Test runner that executes test cases
pub struct TestRunner {
    options: TestRunOptions,
    runtime: Runtime,
}

impl TestRunner {
    pub fn new() -> Self {
        Self::with_options(TestRunOptions::default())
    }

    pub fn with_options(options: TestRunOptions) -> Self {
        let runtime_config = RuntimeConfig {
            stack_size: 1024 * 1024,         // 1MB stack for tests
            max_heap_size: 16 * 1024 * 1024, // 16MB heap for tests
            gc_threshold: 8 * 1024 * 1024,
            enable_profiling: false,
            enable_gc: true,
            enable_panic_handler: true,
        };

        Self {
            options,
            runtime: Runtime::new(runtime_config),
        }
    }

    /// Run all tests in a suite
    pub fn run_suite(&mut self, suite: &TestSuite) -> Result<Vec<TestResult>> {
        if self.options.parallel && suite.tests.len() > 1 {
            self.run_parallel(suite)
        } else {
            self.run_sequential(suite)
        }
    }

    /// Run tests sequentially
    fn run_sequential(&mut self, suite: &TestSuite) -> Result<Vec<TestResult>> {
        let mut results = Vec::new();

        for test in &suite.tests {
            // Check if we should skip
            if let Some(reason) = test.should_skip() {
                results.push(TestResult {
                    test: test.clone(),
                    status: TestStatus::Skipped(reason.to_string()),
                    duration: Duration::ZERO,
                    output: String::new(),
                });
                continue;
            }

            // Run setup if provided
            if let Some(setup) = &suite.setup {
                self.execute_setup_teardown(setup)?;
            }

            // Run the test
            let result = self.run_single_test(test);

            // Check fail-fast
            if self.options.fail_fast && matches!(result.status, TestStatus::Failed(_)) {
                results.push(result);
                break;
            }

            results.push(result);

            // Run teardown if provided
            if let Some(teardown) = &suite.teardown {
                let _ = self.execute_setup_teardown(teardown); // Ignore teardown errors
            }
        }

        Ok(results)
    }

    /// Run tests in parallel
    fn run_parallel(&mut self, suite: &TestSuite) -> Result<Vec<TestResult>> {
        use std::sync::mpsc;
        use std::thread;

        let (tx, rx) = mpsc::channel();
        let test_count = suite.tests.len();
        let mut handles = vec![];

        for test in &suite.tests {
            let tx = tx.clone();
            let test = test.clone();
            let options = self.options.clone();
            let setup = suite.setup.clone();
            let teardown = suite.teardown.clone();

            let handle = thread::spawn(move || {
                let mut runner = TestRunner::with_options(options);

                // Check if we should skip
                if let Some(reason) = test.should_skip() {
                    let result = TestResult {
                        test: test.clone(),
                        status: TestStatus::Skipped(reason.to_string()),
                        duration: Duration::ZERO,
                        output: String::new(),
                    };
                    tx.send(result).unwrap();
                    return;
                }

                // Run setup
                if let Some(setup) = setup {
                    if let Err(_) = runner.execute_setup_teardown(&setup) {
                        let result = TestResult {
                            test: test.clone(),
                            status: TestStatus::Failed(TestFailure::new("Setup failed")),
                            duration: Duration::ZERO,
                            output: String::new(),
                        };
                        tx.send(result).unwrap();
                        return;
                    }
                }

                // Run test
                let result = runner.run_single_test(&test);
                tx.send(result).unwrap();

                // Run teardown
                if let Some(teardown) = teardown {
                    let _ = runner.execute_setup_teardown(&teardown);
                }
            });

            handles.push(handle);
        }

        drop(tx);

        // Wait for all threads to complete
        for handle in handles {
            let _ = handle.join();
        }

        let results: Vec<TestResult> = rx.iter().take(test_count).collect();
        Ok(results)
    }

    /// Run a single test case
    fn run_single_test(&mut self, test: &TestCase) -> TestResult {
        let start = Instant::now();
        let mut output = String::new();

        // Set up output capture if needed
        let _guard = if self.options.capture_output {
            Some(OutputCapture::new())
        } else {
            None
        };

        // Create a test-specific program with the test function
        let test_program = self.create_test_program(test);

        // Execute the test with timeout
        let timeout = test.timeout();
        let (tx, rx) = std::sync::mpsc::channel();

        let test_clone = test.clone();
        let should_panic = test.should_panic().map(|s| s.to_string());

        thread::spawn(move || {
            let result = std::panic::catch_unwind(|| {
                // This would normally compile and execute the test
                // For now, we'll simulate the execution
                // In a real implementation, this would:
                // 1. Compile the test program
                // 2. Execute it in the runtime
                // 3. Check assertions
                Ok::<(), crate::error::Error>(())
            });

            match result {
                Ok(Ok(())) => {
                    if should_panic.is_some() {
                        tx.send(TestStatus::Failed(TestFailure::new(
                            "Expected panic but test completed successfully",
                        )))
                        .unwrap();
                    } else {
                        tx.send(TestStatus::Passed).unwrap();
                    }
                }
                Ok(Err(e)) => {
                    tx.send(TestStatus::Failed(TestFailure::new(e.to_string())))
                        .unwrap();
                }
                Err(panic) => {
                    let msg = if let Some(s) = panic.downcast_ref::<String>() {
                        s.clone()
                    } else if let Some(s) = panic.downcast_ref::<&str>() {
                        s.to_string()
                    } else {
                        "Unknown panic".to_string()
                    };

                    if let Some(expected) = should_panic {
                        if expected.is_empty() || msg.contains(&expected) {
                            tx.send(TestStatus::Passed).unwrap();
                        } else {
                            tx.send(TestStatus::Failed(TestFailure::new(format!(
                                "Expected panic with '{}', got '{}'",
                                expected, msg
                            ))))
                            .unwrap();
                        }
                    } else {
                        tx.send(TestStatus::Panicked(msg)).unwrap();
                    }
                }
            }
        });

        // Wait for result with timeout
        let status = match rx.recv_timeout(timeout) {
            Ok(status) => status,
            Err(_) => TestStatus::Failed(TestFailure::new(format!(
                "Test timed out after {:?}",
                timeout
            ))),
        };

        // Capture output if guard is active
        if let Some(guard) = _guard {
            output = guard.get_output();
        }

        let duration = start.elapsed();

        TestResult {
            test: test.clone(),
            status,
            duration,
            output,
        }
    }

    /// Execute setup or teardown function
    fn execute_setup_teardown(&mut self, stmt: &Stmt) -> Result<()> {
        // In a real implementation, this would compile and execute the function
        // For now, we'll just return Ok
        Ok(())
    }

    /// Create a test program that includes the test function and assertions
    fn create_test_program(&self, test: &TestCase) -> crate::parser::Program {
        use crate::parser::{Program, Stmt, StmtKind};

        // Create a program with:
        // 1. Import assertion functions
        // 2. The test function
        // 3. A main function that calls the test

        let mut statements = vec![];

        // Add import for assertions
        statements.push(Stmt {
            kind: StmtKind::Import {
                imports: vec![crate::parser::ImportSpecifier::Namespace {
                    alias: "assert".to_string(),
                }],
                module: "std::testing::assertions".to_string(),
            },
            span: test.span,
            attributes: vec![],
        });

        // Add the test function
        statements.push(Stmt {
            kind: StmtKind::Function {
                name: test.name.clone(),
                generic_params: None,
                params: vec![],
                ret_type: None,
                body: test.body.clone(),
                is_async: false,
            },
            span: test.span,
            attributes: vec![],
        });

        // Add main function that calls the test
        statements.push(Stmt {
            kind: StmtKind::Function {
                name: "main".to_string(),
                generic_params: None,
                params: vec![],
                ret_type: None,
                body: crate::parser::Block {
                    statements: vec![Stmt {
                        kind: StmtKind::Expression(crate::parser::Expr {
                            kind: crate::parser::ExprKind::Call {
                                callee: Box::new(crate::parser::Expr {
                                    kind: crate::parser::ExprKind::Identifier(test.name.clone()),
                                    span: test.span,
                                }),
                                args: vec![],
                            },
                            span: test.span,
                        }),
                        span: test.span,
                        attributes: vec![],
                    }],
                    final_expr: None,
                },
                is_async: false,
            },
            span: test.span,
            attributes: vec![],
        });

        Program { statements }
    }
}

/// Output capture for tests
struct OutputCapture {
    original_stdout: Option<Box<dyn std::io::Write + Send>>,
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl OutputCapture {
    fn new() -> Self {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        // In a real implementation, we would redirect stdout/stderr here
        Self {
            original_stdout: None,
            buffer,
        }
    }

    fn get_output(&self) -> String {
        let buffer = self.buffer.lock().unwrap();
        String::from_utf8_lossy(&buffer).to_string()
    }
}

impl Drop for OutputCapture {
    fn drop(&mut self) {
        // Restore original stdout/stderr
    }
}
