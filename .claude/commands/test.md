# /test Command Documentation

## Overview

The `/test` command provides comprehensive testing functionality for the Script programming language project. It orchestrates all testing workflows from unit tests to security validation, performance benchmarks, and regression testing.

## Purpose

This command enhances development productivity and code quality by:
- Running targeted test suites for rapid feedback
- Generating test scaffolding for new features
- Providing test coverage analysis and reporting
- Automating regression testing for language features
- Tracking performance benchmarks and detecting regressions
- Integrating security testing into the development workflow

## Usage

### Basic Syntax
```bash
/test                           # Run all tests with smart selection
/test <suite>                   # Run specific test suite
/test <pattern>                 # Run tests matching pattern
/test --create <feature>        # Generate test scaffolding for feature
```

### Test Suite Selection
```bash
/test unit                      # Run all unit tests
/test integration               # Run integration tests
/test security                  # Run security-specific tests
/test benchmarks                # Run performance benchmarks
/test lexer                     # Run lexer-specific tests
/test parser                    # Run parser tests
/test semantic                  # Run semantic analysis tests
/test codegen                   # Run code generation tests
/test runtime                   # Run runtime system tests
/test mcp                       # Run MCP integration tests
```

### Advanced Testing Options
```bash
/test --coverage               # Run tests with coverage analysis
/test --regression             # Run regression test suite
/test --performance            # Focus on performance validation
/test --security-audit         # Run comprehensive security tests
/test --ci                     # Run tests in CI mode (fast, essential only)
/test --dev                    # Run tests in development mode (verbose output)
```

### Test Generation
```bash
/test --create pattern_matching    # Generate pattern matching tests
/test --create async_syntax        # Generate async/await tests
/test --create module_system       # Generate module system tests
/test --scaffold <feature>        # Create full test scaffolding
```

## Test Suite Categories

### 1. Unit Tests
**Purpose**: Test individual components in isolation
**Location**: `tests/` directory, module-specific test files
**Command**: `/test unit`

**Coverage Areas**:
- Lexer token recognition and error handling
- Parser grammar rules and AST construction
- Semantic analysis and type checking
- Code generation and IR optimization
- Runtime primitive operations

### 2. Integration Tests
**Purpose**: Test component interactions and end-to-end workflows
**Location**: `tests/integration/`
**Command**: `/test integration`

**Coverage Areas**:
- Complete compilation pipeline (source → IR → execution)
- Cross-module type checking and imports
- Generic type instantiation and monomorphization
- Async operation coordination
- Memory management and garbage collection

### 3. Security Tests
**Purpose**: Validate security properties and DoS protection
**Location**: `tests/security/`
**Command**: `/test security`

**Coverage Areas**:
- Resource limit enforcement
- Input sanitization and validation
- Path traversal prevention
- Memory safety guarantees
- Compilation timeout protection

### 4. Performance Benchmarks
**Purpose**: Track performance characteristics and detect regressions
**Location**: `benches/`
**Command**: `/test benchmarks`

**Benchmark Categories**:
- Compilation speed (lexing, parsing, semantic analysis)
- Runtime performance (execution speed, memory usage)
- Type inference complexity
- Code generation efficiency
- Garbage collection performance

### 5. Regression Tests
**Purpose**: Ensure previously fixed issues remain resolved
**Location**: `tests/regression/`
**Command**: `/test regression`

**Test Types**:
- Bug reproduction tests
- Security vulnerability prevention
- Performance regression detection
- API compatibility validation

## Test Generation Features

### Automatic Scaffolding
The `/test --create <feature>` command generates comprehensive test scaffolding:

```rust
// Generated test structure for pattern matching:
tests/
├── pattern_matching/
│   ├── mod.rs                 # Test module structure
│   ├── basic_patterns.rs      # Basic pattern matching tests
│   ├── exhaustiveness.rs      # Exhaustiveness checking tests
│   ├── performance.rs         # Pattern matching performance tests
│   ├── security.rs           # Security-related pattern tests
│   └── regression.rs         # Regression prevention tests
```

### Test Templates
Smart test generation based on feature type:

```rust
// Example generated test for async feature:
#[tokio::test]
async fn test_async_function_basic() {
    let source = r#"
        async fn example() -> int {
            let result = await some_async_operation();
            return result + 1;
        }
    "#;
    
    let result = compile_and_run_async(source).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

#[test]
fn test_async_syntax_security() {
    // Test for DoS through deep async nesting
    let source = generate_deeply_nested_async(1000);
    
    let result = compile_with_limits(source);
    assert!(result.is_err());
    assert!(result.unwrap_err().is_resource_limit_exceeded());
}
```

## Coverage Analysis

### Coverage Reporting
```bash
/test --coverage
```

**Output Format**:
```
📊 Test Coverage Report
======================
Overall Coverage: 87.3%

By Component:
├── Lexer:          94.2% ✓
├── Parser:         91.7% ✓  
├── Semantic:       84.1% ⚠
├── Codegen:        82.6% ⚠
├── Runtime:        89.4% ✓
└── MCP:           78.9% ⚠

Critical Paths:
├── Error handling:  96.1% ✓
├── Security:        92.8% ✓
└── Memory safety:   88.7% ✓

Uncovered Areas:
├── src/semantic/constraint_solver.rs:142-158
├── src/codegen/optimization.rs:89-95
└── src/runtime/async_ffi.rs:201-210

📝 Full report: kb/active/TEST_COVERAGE_REPORT.md
```

### Coverage Targets
- **Minimum acceptable**: 80% overall, 90% for security-critical paths
- **Target goal**: 90% overall, 95% for security-critical paths
- **Critical components**: 95%+ (error handling, resource limits, validation)

## Performance Tracking

### Benchmark Execution
```bash
/test benchmarks
```

**Output includes**:
- Compilation time benchmarks
- Runtime performance metrics
- Memory usage analysis
- Regression detection alerts

**Example Output**:
```
🏃 Performance Benchmarks
=========================
Compilation Benchmarks:
├── Lexer:          1.2ms  (-0.1ms) ✓
├── Parser:         4.8ms  (+0.3ms) ⚠
├── Semantic:      12.4ms  (+1.2ms) ❌
└── Codegen:        8.6ms  (-0.2ms) ✓

Runtime Benchmarks:
├── Function calls: 15.2ns (+0.8ns) ⚠
├── Memory alloc:   42.1ns (-2.1ns) ✓
└── Async yield:    89.3ns (+5.2ns) ❌

⚠ Regressions detected in 2 areas
📊 Full report: kb/active/PERFORMANCE_REGRESSION_REPORT.md
```

## Security Testing Integration

### Automated Security Validation
```bash
/test security
```

**Security Test Categories**:
1. **Input Validation**: Malformed source code, edge cases
2. **Resource Limits**: DoS protection, memory exhaustion
3. **Path Traversal**: Module import security
4. **Type Safety**: Memory corruption prevention
5. **Concurrency**: Race condition detection

**Example Security Tests**:
```rust
#[test]
fn test_compilation_timeout_protection() {
    let malicious_source = generate_exponential_compilation_time();
    
    let start = Instant::now();
    let result = compile_with_production_limits(malicious_source);
    let duration = start.elapsed();
    
    // Should timeout within reasonable bounds
    assert!(duration < Duration::from_secs(10));
    assert!(result.is_err());
    assert!(result.unwrap_err().is_timeout());
}

#[test]
fn test_memory_limit_enforcement() {
    let memory_bomb = generate_memory_exhaustion_source();
    
    let result = compile_with_memory_limits(memory_bomb);
    assert!(result.is_err());
    assert!(result.unwrap_err().is_memory_limit_exceeded());
}
```

## CI/CD Integration

### CI Mode
```bash
/test --ci
```

**CI-Optimized Behavior**:
- Fast test subset (essential tests only)
- Parallel execution where safe
- Structured output for CI parsing
- Fail-fast on critical errors
- Performance baseline validation

### Development Mode
```bash
/test --dev
```

**Development-Optimized Behavior**:
- Verbose output with debugging information
- Watch mode for continuous testing
- Detailed failure diagnostics
- Interactive failure investigation
- Hot reload capability

## Test Organization

### Directory Structure
```
tests/
├── unit/                   # Unit tests
│   ├── lexer/
│   ├── parser/
│   ├── semantic/
│   ├── codegen/
│   └── runtime/
├── integration/            # Integration tests
│   ├── compilation_pipeline/
│   ├── type_system/
│   └── module_system/
├── security/               # Security-focused tests
│   ├── dos_protection/
│   ├── input_validation/
│   └── memory_safety/
├── regression/             # Regression prevention
│   ├── bug_reproductions/
│   └── security_fixes/
└── performance/            # Performance validation
    ├── compilation_speed/
    └── runtime_efficiency/
```

### Test Naming Conventions
```rust
// Unit tests
fn test_<component>_<functionality>_<scenario>()

// Integration tests  
fn integration_<workflow>_<scenario>()

// Security tests
fn security_<vulnerability_type>_<attack_scenario>()

// Performance tests
fn bench_<operation>_<scale>()

// Regression tests
fn regression_issue_<issue_number>_<description>()
```

## Error Handling and Reporting

### Test Failure Analysis
When tests fail, the command provides:
- Clear failure reason and location
- Suggested fix strategies
- Related documentation links
- Regression risk assessment

### Failure Report Format
```
❌ Test Failure: test_pattern_matching_exhaustiveness
Location: tests/unit/semantic/pattern_matching.rs:127
Failure: Assertion failed: expected exhaustive match, found missing case

Context:
├── Pattern: enum Option { Some(T), None }
├── Match arms: Some(_) 
└── Missing: None

Suggestions:
1. Add missing None case to match expression
2. Review exhaustiveness checking algorithm in src/semantic/pattern_check.rs
3. Check related issue: kb/active/PATTERN_MATCHING_EXHAUSTIVENESS.md

Related Tests:
├── test_pattern_matching_basic ✓
├── test_pattern_matching_nested ✓
└── test_pattern_matching_guards ❌ (also failing)
```

## Knowledge Base Integration

### Test Documentation
All test activities are logged to the knowledge base:
- Test run summaries in `kb/active/TEST_RUN_<timestamp>.md`
- Coverage reports in `kb/active/TEST_COVERAGE_REPORT.md`
- Performance tracking in `kb/active/PERFORMANCE_REGRESSION_REPORT.md`
- Security test results in `kb/active/SECURITY_TEST_RESULTS.md`

### Issue Tracking
Failed tests automatically create or update knowledge base entries:
- New failures create issues in `kb/active/`
- Resolved failures move to `kb/completed/`
- Regression detection reopens closed issues

## Integration with Other Commands

### Command Synergy
- `/audit` → `/test security` (validate audit findings)
- `/implement` → `/test --create <feature>` (generate tests for new features)
- `/test` → `/debug` (investigate test failures)
- `/test --regression` → continuous validation of previous fixes

### Workflow Integration
```bash
# Complete development workflow:
/implement async_syntax          # Implement new feature
/test --create async_syntax      # Generate test scaffolding
/test async                      # Run async-specific tests
/test --coverage                 # Check coverage
/audit src/parser/async.rs       # Security audit
/test security                   # Validate security properties
```

## Best Practices

### Test Development
- Write tests before implementing features (TDD)
- Include security tests for all new functionality
- Add performance benchmarks for critical paths
- Create regression tests for all bug fixes

### Test Maintenance
- Regularly run full test suite
- Monitor coverage trends
- Update tests when APIs change
- Archive obsolete tests appropriately

### Performance Considerations
- Use parameterized tests for edge cases
- Mock expensive operations in unit tests
- Profile test execution time
- Optimize slow tests or move to integration suite

## Limitations

### Current Limitations
- Limited fuzzing integration (planned enhancement)
- Basic property-based testing support
- Manual test oracle creation for complex features
- Platform-specific test variations

### Planned Enhancements
- Automatic test generation from specifications
- AI-assisted test case creation
- Advanced mutation testing
- Cross-platform test validation

This `/test` command provides comprehensive testing support that ensures the Script language maintains high quality, security, and performance standards throughout development.