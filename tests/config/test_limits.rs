//! Test resource limits and configuration
//!
//! This module provides centralized configuration for test resource usage,
//! allowing tests to scale based on environment (CI vs development).

use std::env;

/// Test intensity levels
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TestIntensity {
    /// Minimal resource usage for CI/fast testing
    Low,
    /// Moderate resource usage for development
    Medium,
    /// Full resource usage for thorough testing
    High,
}

impl TestIntensity {
    /// Get test intensity from environment variable
    pub fn from_env() -> Self {
        match env::var("SCRIPT_TEST_INTENSITY").as_deref() {
            Ok("low") => TestIntensity::Low,
            Ok("medium") => TestIntensity::Medium,
            Ok("high") => TestIntensity::High,
            _ => {
                // Default to low intensity in CI environments
                if is_ci_environment() {
                    TestIntensity::Low
                } else {
                    TestIntensity::Medium
                }
            }
        }
    }
}

/// Test resource limits configuration
#[derive(Debug, Clone)]
pub struct TestLimits {
    /// Maximum type variables for DoS tests
    pub max_type_variables: usize,
    /// Maximum constraints for DoS tests
    pub max_constraints: usize,
    /// Maximum specializations for monomorphization tests
    pub max_specializations: usize,
    /// Maximum work queue items
    pub max_work_queue_items: usize,
    /// Maximum memory allocation per test (bytes)
    pub max_memory_per_test: usize,
    /// Maximum test timeout (seconds)
    pub max_timeout_secs: u64,
    /// Maximum iterations for stress tests
    pub max_stress_iterations: usize,
    /// Maximum concurrent threads for concurrency tests
    pub max_concurrent_threads: usize,
}

impl TestLimits {
    /// Get test limits based on current intensity
    pub fn for_intensity(intensity: TestIntensity) -> Self {
        match intensity {
            TestIntensity::Low => Self {
                max_type_variables: 500,        // Was 15,000
                max_constraints: 2000,          // Was 60,000  
                max_specializations: 100,       // Was 1,500
                max_work_queue_items: 500,      // Was 12,000
                max_memory_per_test: 1024 * 1024, // 1MB
                max_timeout_secs: 5,            // 5 seconds
                max_stress_iterations: 10,      // Very limited
                max_concurrent_threads: 2,      // Minimal concurrency
            },
            TestIntensity::Medium => Self {
                max_type_variables: 2000,       // Moderate
                max_constraints: 8000,          // Moderate
                max_specializations: 300,       // Moderate
                max_work_queue_items: 2000,     // Moderate
                max_memory_per_test: 4 * 1024 * 1024, // 4MB
                max_timeout_secs: 15,           // 15 seconds
                max_stress_iterations: 50,      // Moderate
                max_concurrent_threads: 4,      // Moderate concurrency
            },
            TestIntensity::High => Self {
                max_type_variables: 5000,       // High but reasonable
                max_constraints: 20000,         // High but reasonable
                max_specializations: 800,       // High but reasonable
                max_work_queue_items: 5000,     // High but reasonable
                max_memory_per_test: 10 * 1024 * 1024, // 10MB
                max_timeout_secs: 30,           // 30 seconds
                max_stress_iterations: 200,     // High stress
                max_concurrent_threads: 8,      // High concurrency
            },
        }
    }

    /// Get current test limits based on environment
    pub fn current() -> Self {
        let intensity = TestIntensity::from_env();
        Self::for_intensity(intensity)
    }

    /// Check if memory allocation is within limits
    pub fn check_memory_allocation(&self, size: usize) -> Result<(), String> {
        if size > self.max_memory_per_test {
            Err(format!(
                "Memory allocation {} bytes exceeds limit {} bytes",
                size, self.max_memory_per_test
            ))
        } else {
            Ok(())
        }
    }

    /// Get safe iteration count for stress tests
    pub fn safe_iteration_count(&self, base_count: usize) -> usize {
        std::cmp::min(base_count, self.max_stress_iterations)
    }

    /// Get safe timeout duration
    pub fn safe_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.max_timeout_secs)
    }
}

/// Resource usage monitor for tests
#[derive(Debug)]
pub struct ResourceMonitor {
    start_time: std::time::Instant,
    limits: TestLimits,
    memory_allocated: usize,
}

impl ResourceMonitor {
    /// Create new resource monitor
    pub fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
            limits: TestLimits::current(),
            memory_allocated: 0,
        }
    }

    /// Check if allocation is allowed
    pub fn check_allocation(&mut self, size: usize) -> Result<(), String> {
        if self.memory_allocated + size > self.limits.max_memory_per_test {
            return Err(format!(
                "Total memory allocation would exceed limit: {} + {} > {}",
                self.memory_allocated, size, self.limits.max_memory_per_test
            ));
        }
        self.memory_allocated += size;
        Ok(())
    }

    /// Release allocated memory
    pub fn release_memory(&mut self, size: usize) {
        self.memory_allocated = self.memory_allocated.saturating_sub(size);
    }

    /// Check if time limit exceeded
    pub fn check_timeout(&self) -> Result<(), String> {
        let elapsed = self.start_time.elapsed();
        let limit = self.limits.safe_timeout();
        
        if elapsed > limit {
            Err(format!(
                "Test timeout exceeded: {:?} > {:?}",
                elapsed, limit
            ))
        } else {
            Ok(())
        }
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    /// Get memory usage
    pub fn memory_usage(&self) -> usize {
        self.memory_allocated
    }
}

impl Default for ResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Test utilities for resource-safe operations
pub struct SafeTestOps;

impl SafeTestOps {
    /// Perform safe memory allocation with limits
    pub fn safe_alloc(size: usize, monitor: &mut ResourceMonitor) -> Result<Vec<u8>, String> {
        monitor.check_allocation(size)?;
        monitor.check_timeout()?;
        
        let allocation = vec![0u8; size];
        Ok(allocation)
    }

    /// Perform safe iteration with limits
    pub fn safe_iterate<F>(
        max_iterations: usize,
        monitor: &mut ResourceMonitor,
        mut operation: F,
    ) -> Result<usize, String>
    where
        F: FnMut(usize) -> Result<(), String>,
    {
        let limits = TestLimits::current();
        let safe_max = limits.safe_iteration_count(max_iterations);
        
        for i in 0..safe_max {
            monitor.check_timeout()?;
            operation(i)?;
            
            // Check every 100 iterations for timeout
            if i % 100 == 0 {
                monitor.check_timeout()?;
            }
        }
        
        Ok(safe_max)
    }

    /// Generate safe test string with size limits
    pub fn safe_string_generation(
        base_size: usize,
        count: usize,
        monitor: &mut ResourceMonitor,
    ) -> Result<Vec<String>, String> {
        let limits = TestLimits::current();
        let total_size = base_size * count;
        
        monitor.check_allocation(total_size)?;
        monitor.check_timeout()?;
        
        let safe_count = std::cmp::min(count, limits.max_stress_iterations);
        let safe_size = std::cmp::min(base_size, 1024); // Max 1KB per string
        
        let mut results = Vec::new();
        for i in 0..safe_count {
            let content = format!("test_string_{}", i);
            let padded = format!("{:width$}", content, width = safe_size);
            results.push(padded);
            
            if i % 10 == 0 {
                monitor.check_timeout()?;
            }
        }
        
        Ok(results)
    }
}

/// Check if running in CI environment
fn is_ci_environment() -> bool {
    env::var("CI").is_ok()
        || env::var("GITHUB_ACTIONS").is_ok()
        || env::var("GITLAB_CI").is_ok()
        || env::var("JENKINS_URL").is_ok()
        || env::var("BUILDKITE").is_ok()
}

/// Test configuration helper macros
#[macro_export]
macro_rules! with_test_limits {
    ($test_name:expr, $code:block) => {{
        let limits = $crate::config::test_limits::TestLimits::current();
        let mut monitor = $crate::config::test_limits::ResourceMonitor::new();
        
        println!("Running test '{}' with limits: {:?}", $test_name, limits);
        
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $code));
        
        println!(
            "Test '{}' completed in {:?}, memory: {} bytes",
            $test_name,
            monitor.elapsed(),
            monitor.memory_usage()
        );
        
        result.unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    }};
}

#[macro_export]
macro_rules! safe_stress_test {
    ($iterations:expr, $monitor:expr, $operation:expr) => {{
        $crate::config::test_limits::SafeTestOps::safe_iterate($iterations, $monitor, $operation)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intensity_parsing() {
        std::env::set_var("SCRIPT_TEST_INTENSITY", "low");
        assert_eq!(TestIntensity::from_env(), TestIntensity::Low);
        
        std::env::set_var("SCRIPT_TEST_INTENSITY", "high");
        assert_eq!(TestIntensity::from_env(), TestIntensity::High);
    }

    #[test]
    fn test_resource_limits() {
        let low_limits = TestLimits::for_intensity(TestIntensity::Low);
        let high_limits = TestLimits::for_intensity(TestIntensity::High);
        
        assert!(low_limits.max_type_variables < high_limits.max_type_variables);
        assert!(low_limits.max_memory_per_test < high_limits.max_memory_per_test);
    }

    #[test]
    fn test_resource_monitor() {
        let mut monitor = ResourceMonitor::new();
        
        // Should allow small allocation
        assert!(monitor.check_allocation(1024).is_ok());
        
        // Should reject massive allocation
        assert!(monitor.check_allocation(1024 * 1024 * 1024).is_err());
    }

    #[test]
    fn test_safe_operations() {
        let mut monitor = ResourceMonitor::new();
        
        // Safe allocation should work
        let result = SafeTestOps::safe_alloc(1024, &mut monitor);
        assert!(result.is_ok());
        
        // Safe iteration should work
        let mut count = 0;
        let result = SafeTestOps::safe_iterate(10, &mut monitor, |_| {
            count += 1;
            Ok(())
        });
        assert!(result.is_ok());
        assert!(count <= 10);
    }
}