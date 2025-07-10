use crate::error::Result;
use crate::testing::{TestFailure, TestResult, TestStatus};
use colored::*;
use std::io::{self, Write};
use std::time::Duration;

/// Trait for test result reporting
pub trait TestReporter: Send + Sync {
    /// Report the start of test execution
    fn on_test_start(&mut self, name: &str) -> Result<()>;

    /// Report a single test result
    fn on_test_complete(&mut self, result: &TestResult) -> Result<()>;

    /// Report all test results
    fn report_results(&mut self, results: &[TestResult]) -> Result<()>;

    /// Report test suite summary
    fn report_summary(
        &mut self,
        total: usize,
        passed: usize,
        failed: usize,
        skipped: usize,
        duration: Duration,
    ) -> Result<()>;
}

/// Output format for test results
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReportFormat {
    /// Human-readable console output
    Console,
    /// JSON output for tooling
    Json,
    /// JUnit XML format
    JUnit,
    /// Minimal output (dots)
    Minimal,
}

/// Console reporter with colored output
pub struct ConsoleReporter {
    format: ReportFormat,
    verbose: bool,
    show_output: bool,
    current_module: Option<String>,
}

impl ConsoleReporter {
    pub fn new() -> Self {
        Self {
            format: ReportFormat::Console,
            verbose: false,
            show_output: false,
            current_module: None,
        }
    }

    pub fn with_format(mut self, format: ReportFormat) -> Self {
        self.format = format;
        self
    }

    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    pub fn show_output(mut self, show: bool) -> Self {
        self.show_output = show;
        self
    }

    fn print_test_header(&self, name: &str) {
        if self.verbose {
            print!("{} {} ... ", "test".cyan(), name);
            io::stdout().flush().unwrap();
        }
    }

    fn print_test_result(&self, result: &TestResult) {
        match &result.status {
            TestStatus::Passed => {
                if self.verbose {
                    println!(
                        "{} ({:.3}s)",
                        "ok".green().bold(),
                        result.duration.as_secs_f64()
                    );
                } else if self.format == ReportFormat::Minimal {
                    print!("{}", ".".green());
                    io::stdout().flush().unwrap();
                }
            }
            TestStatus::Failed(failure) => {
                if self.verbose || self.format == ReportFormat::Console {
                    println!(
                        "{} ({:.3}s)",
                        "FAILED".red().bold(),
                        result.duration.as_secs_f64()
                    );
                    self.print_failure_details(&result.test.name, failure);
                } else if self.format == ReportFormat::Minimal {
                    print!("{}", "F".red().bold());
                    io::stdout().flush().unwrap();
                }
            }
            TestStatus::Skipped(reason) => {
                if self.verbose {
                    println!("{} - {}", "skipped".yellow(), reason);
                } else if self.format == ReportFormat::Minimal {
                    print!("{}", "s".yellow());
                    io::stdout().flush().unwrap();
                }
            }
            TestStatus::Panicked(msg) => {
                if self.verbose || self.format == ReportFormat::Console {
                    println!(
                        "{} ({:.3}s)",
                        "PANICKED".red().bold(),
                        result.duration.as_secs_f64()
                    );
                    println!("  {}: {}", "panic".red(), msg);
                } else if self.format == ReportFormat::Minimal {
                    print!("{}", "P".red().bold());
                    io::stdout().flush().unwrap();
                }
            }
        }

        // Show captured output if requested
        if self.show_output && !result.output.is_empty() {
            println!("\n{}", "---- output ----".dimmed());
            println!("{result.output}");
            println!("{}", "----------------".dimmed());
        }
    }

    fn print_failure_details(&self, test_name: &str, failure: &TestFailure) {
        println!("\n{}", "---- failure details ----".red().dimmed());
        println!("{}: {}", "test".dimmed(), test_name);
        println!("{}: {}", "error".red(), failure.message);

        if let (Some(expected), Some(actual)) = (&failure.expected, &failure.actual) {
            println!("\n{}", "comparison:".dimmed());
            println!("  {}: {}", "expected".green(), expected);
            println!("  {}: {}", "actual".red(), actual);
        }

        if let Some(location) = &failure.location {
            println!(
                "\n{}: {}:{}:{}",
                "location".dimmed(),
                "test.script", // TODO: Get actual filename
                location.start.line,
                location.start.column
            );
        }

        println!("{}\n", "------------------------".red().dimmed());
    }
}

impl TestReporter for ConsoleReporter {
    fn on_test_start(&mut self, name: &str) -> Result<()> {
        self.print_test_header(name);
        Ok(())
    }

    fn on_test_complete(&mut self, result: &TestResult) -> Result<()> {
        self.print_test_result(result);
        Ok(())
    }

    fn report_results(&mut self, results: &[TestResult]) -> Result<()> {
        match self.format {
            ReportFormat::Console => {
                // Group results by module if available
                for result in results {
                    self.print_test_result(result);
                }
            }
            ReportFormat::Minimal => {
                // Already printed dots during execution
                println!(); // New line after dots
            }
            ReportFormat::Json => {
                self.report_json(results)?;
            }
            ReportFormat::JUnit => {
                self.report_junit(results)?;
            }
        }
        Ok(())
    }

    fn report_summary(
        &mut self,
        total: usize,
        passed: usize,
        failed: usize,
        skipped: usize,
        duration: Duration,
    ) -> Result<()> {
        println!("\n{}", "Test Summary".bold());
        println!("{}", "=".repeat(50));

        let status = if failed == 0 {
            "PASSED".green().bold()
        } else {
            "FAILED".red().bold()
        };

        println!("Status: {status}");
        println!("Total:  {} tests", total);

        if passed > 0 {
            println!(
                "Passed: {} ({}%)",
                passed.to_string().green(),
                (passed * 100 / total).to_string().green()
            );
        }

        if failed > 0 {
            println!(
                "Failed: {} ({}%)",
                failed.to_string().red().bold(),
                (failed * 100 / total).to_string().red()
            );
        }

        if skipped > 0 {
            println!(
                "Skipped: {} ({}%)",
                skipped.to_string().yellow(),
                (skipped * 100 / total).to_string().yellow()
            );
        }

        println!("Duration: {:.3}s", duration.as_secs_f64());
        println!("{"=".repeat(50}");

        Ok(())
    }
}

impl ConsoleReporter {
    fn report_json(&self, results: &[TestResult]) -> Result<()> {
        // Simple JSON output without external dependencies
        println!("{{");
        println!("  \"test_results\": [");

        for (i, r) in results.iter().enumerate() {
            let (status, error) = match &r.status {
                TestStatus::Passed => ("passed", None),
                TestStatus::Failed(f) => ("failed", Some(f.message.as_str())),
                TestStatus::Skipped(reason) => ("skipped", Some(reason.as_str())),
                TestStatus::Panicked(msg) => ("panicked", Some(msg.as_str())),
            };

            print!("    {{");
            print!(" \"name\": \"{}\",", r.test.name);
            print!(" \"status\": \"{}\",", status);
            print!(" \"duration_ms\": {},", r.duration.as_millis());
            if let Some(err) = error {
                print!(" \"error\": \"{}\"", err.replace("\"", "\\\""));
            } else {
                print!(" \"error\": null");
            }
            if self.show_output && !r.output.is_empty() {
                print!(
                    ", \"output\": \"{}\"",
                    r.output.replace("\"", "\\\"").replace("\n", "\\n")
                );
            }
            print!(" }}");
            if i < results.len() - 1 {
                println!(",");
            } else {
                println!();
            }
        }

        println!("  ],");
        println!("  \"summary\": {{");
        println!("    \"total\": {},", results.len());
        println!(
            "    \"passed\": {},",
            results
                .iter()
                .filter(|r| matches!(r.status, TestStatus::Passed))
                .count()
        );
        println!(
            "    \"failed\": {},",
            results
                .iter()
                .filter(|r| matches!(r.status, TestStatus::Failed(_)))
                .count()
        );
        println!(
            "    \"skipped\": {}",
            results
                .iter()
                .filter(|r| matches!(r.status, TestStatus::Skipped(_)))
                .count()
        );
        println!("  }}");
        println!("}}");

        Ok(())
    }

    fn report_junit(&self, results: &[TestResult]) -> Result<()> {
        println!(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        println!(r#"<testsuites>"#);
        println!(
            r#"  <testsuite name="script-tests" tests="{}" failures="{}" skipped="{}" time="{:.3}">"#,
            results.len(),
            results
                .iter()
                .filter(|r| matches!(r.status, TestStatus::Failed(_)))
                .count(),
            results
                .iter()
                .filter(|r| matches!(r.status, TestStatus::Skipped(_)))
                .count(),
            results
                .iter()
                .map(|r| r.duration.as_secs_f64())
                .sum::<f64>()
        );

        for result in results {
            match &result.status {
                TestStatus::Passed => {
                    println!(
                        r#"    <testcase name="{}" time="{:.3}"/>"#,
                        result.test.name,
                        result.duration.as_secs_f64()
                    );
                }
                TestStatus::Failed(failure) => {
                    println!(
                        r#"    <testcase name="{}" time="{:.3}">"#,
                        result.test.name,
                        result.duration.as_secs_f64()
                    );
                    println!(
                        r#"      <failure message="{}">{}</failure>"#,
                        xml_escape(&failure.message),
                        xml_escape(&format!("{:?}", failure))
                    );
                    println!(r#"    </testcase>"#);
                }
                TestStatus::Skipped(reason) => {
                    println!(r#"    <testcase name="{}" time="0">"#, result.test.name);
                    println!(r#"      <skipped message="{}"/>"#, xml_escape(reason));
                    println!(r#"    </testcase>"#);
                }
                TestStatus::Panicked(msg) => {
                    println!(
                        r#"    <testcase name="{}" time="{:.3}">"#,
                        result.test.name,
                        result.duration.as_secs_f64()
                    );
                    println!(
                        r#"      <error message="panic">{}</error>"#,
                        xml_escape(msg)
                    );
                    println!(r#"    </testcase>"#);
                }
            }
        }

        println!(r#"  </testsuite>"#);
        println!(r#"</testsuites>"#);
        Ok(())
    }
}

fn xml_escape(s: &str) -> String {
    s.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;")
}

/// Progress reporter for real-time test updates
pub struct ProgressReporter {
    inner: Box<dyn TestReporter>,
    current_test: Option<String>,
    test_count: usize,
    completed: usize,
}

impl ProgressReporter {
    pub fn new(inner: Box<dyn TestReporter>) -> Self {
        Self {
            inner,
            current_test: None,
            test_count: 0,
            completed: 0,
        }
    }

    pub fn set_total(&mut self, total: usize) {
        self.test_count = total;
    }
}

impl TestReporter for ProgressReporter {
    fn on_test_start(&mut self, name: &str) -> Result<()> {
        self.current_test = Some(name.to_string());
        print!(
            "\r[{}/{}] Running: {} ",
            self.completed + 1,
            self.test_count,
            name
        );
        io::stdout().flush().unwrap();
        self.inner.on_test_start(name)
    }

    fn on_test_complete(&mut self, result: &TestResult) -> Result<()> {
        self.completed += 1;
        print!("\r{}", " ".repeat(80)); // Clear line
        print!("\r");
        io::stdout().flush().unwrap();
        self.inner.on_test_complete(result)
    }

    fn report_results(&mut self, results: &[TestResult]) -> Result<()> {
        self.inner.report_results(results)
    }

    fn report_summary(
        &mut self,
        total: usize,
        passed: usize,
        failed: usize,
        skipped: usize,
        duration: Duration,
    ) -> Result<()> {
        self.inner
            .report_summary(total, passed, failed, skipped, duration)
    }
}
