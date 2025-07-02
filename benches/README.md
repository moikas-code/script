# Script Language Benchmarks

This directory contains comprehensive performance benchmarks for the Script programming language, covering all aspects from lexing and parsing to runtime execution and tooling.

## ⚠️ Current Status and Limitations

**IMPORTANT**: Many benchmark features test **unimplemented or partially implemented** language features. This benchmark suite is designed to support the full development lifecycle of the Script language.

### Implementation Status:
- ✅ **Lexer Benchmarks**: Fully functional
- ✅ **Parser Benchmarks**: Fully functional  
- 🚧 **Compilation Benchmarks**: Partially functional (semantic analysis and type inference work, but code generation is limited)
- ❌ **Runtime Execution**: Most runtime benchmarks will fail as the execution engine is not yet implemented
- ❌ **Advanced Features**: Pattern matching, async/await, generics, and modules are not yet implemented
- ❌ **LSP Operations**: Language server features are planned but not implemented
- ❌ **Documentation Generation**: Not yet implemented
- ❌ **Test Framework**: Not yet implemented

### Benchmarks Testing Future Features:
- `features.rs` - Tests pattern matching, async operations, higher-order functions (all unimplemented)
- `scenarios.rs` - Tests complex runtime scenarios (requires full runtime)
- `memory.rs` - Tests garbage collection and memory management (not implemented)
- `tooling.rs` - Tests LSP, documentation, and testing framework (not implemented)

## Overview

The benchmark suite is designed to:
- Measure performance of core language operations
- Track performance regressions during development
- Compare different implementation strategies
- Validate optimization efforts
- Provide real-world performance metrics
- **Test unimplemented features to guide development priorities**

## Benchmark Categories

### 1. **Lexer Benchmarks** (`lexer.rs`)
- Small program tokenization
- Large program tokenization (100+ functions)
- String-heavy programs
- Unicode handling

### 2. **Parser Benchmarks** (`parser.rs`)
- Expression parsing
- Full program parsing
- Deeply nested structures
- Many statements parsing

### 3. **Compilation Pipeline** (`compilation.rs`)
- Full compilation (source → executable)
- Individual stage benchmarks:
  - Lexing
  - Parsing
  - Semantic analysis
  - Type inference
  - IR lowering
  - Code generation
- Incremental compilation
- Parallel module compilation

### 4. **Language Features** (`features.rs`)
- Pattern matching (simple, nested, with guards)
- Async/await operations
- Function calls (direct, recursive, higher-order)
- Collections (arrays, hashmaps, sets)
- String operations
- Mathematical computations

### 5. **Real-world Scenarios** (`scenarios.rs`)
- Fibonacci (iterative vs recursive vs memoized)
- Sorting algorithms (quicksort, mergesort)
- Tree traversal (DFS, BFS)
- Web server simulation
- Game loop simulation (60 FPS)
- Data processing pipeline

### 6. **Memory Usage** (`memory.rs`)
- Compilation memory consumption
- Runtime allocation patterns
- Garbage collection performance
- Memory fragmentation
- Cyclic reference handling

### 7. **Tooling Performance** (`tooling.rs`)
- LSP operations:
  - Code completion
  - Go to definition
  - Semantic tokens
  - Incremental updates
- Documentation generation
- Test framework:
  - Test discovery
  - Sequential execution
  - Parallel execution
- Code formatting
- Package management

## Setup Instructions

### Prerequisites
1. **Rust**: Ensure you have Rust 1.70+ installed
2. **Python 3.8+**: Required for benchmark dashboard generation
3. **Python Dependencies**: Install using the provided requirements file

### Setting Up Python Environment
```bash
# Navigate to benches directory
cd benches/

# Create virtual environment (recommended)
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate

# Install dependencies
pip install -r requirements.txt
```

### Alternative: System-wide Installation
```bash
# Install matplotlib and numpy system-wide
pip install matplotlib>=3.5.0 numpy>=1.21.0
```

### About requirements.txt
The `requirements.txt` file contains minimal Python dependencies needed for benchmark visualization:
- **matplotlib>=3.5.0**: For generating performance charts and graphs
- **numpy>=1.21.0**: Required by matplotlib for numerical computations

These dependencies are only needed if you want to generate the visual performance dashboard. The core Rust benchmarks work without any Python dependencies.

### Verifying Setup
```bash
# Test Python dependencies
python -c "import matplotlib, numpy; print('Dependencies OK')"

# Test Rust benchmarks
cargo bench --bench lexer -- --help
```

## Running Benchmarks

### Run All Benchmarks
```bash
./run_benchmarks.sh
```

### Run Specific Benchmark Suite
```bash
cargo bench --bench lexer      # ✅ Works
cargo bench --bench parser     # ✅ Works  
cargo bench --bench compilation # 🚧 Partially works
cargo bench --bench features    # ❌ Will fail (unimplemented)
cargo bench --bench scenarios   # ❌ Will fail (unimplemented)
cargo bench --bench memory      # ❌ Will fail (unimplemented)
cargo bench --bench tooling     # ❌ Will fail (unimplemented)
```

### Run Specific Benchmark
```bash
cargo bench --bench features -- pattern_matching
```

### Compare Against Baseline
```bash
# Save baseline
cargo bench --bench lexer -- --save-baseline my-baseline

# Compare against baseline
cargo bench --bench lexer -- --baseline my-baseline
```

### Generate Performance Dashboard
```bash
# After running benchmarks, generate visual dashboard
python generate_dashboard.py

# Dashboard will be created at target/benchmark-dashboard/dashboard.html
```

**Note**: Dashboard generation requires matplotlib. If matplotlib is not installed, the script will still generate an HTML dashboard without charts.

## Benchmark Fixtures

The `fixtures/` directory contains test programs used by benchmarks:

- `fibonacci_recursive.script` - Classic recursive algorithm
- `large_program.script` - Complex program with modules, classes, etc.
- `async_heavy.script` - Async/await intensive operations
- `pattern_matching.script` - Complex pattern matching scenarios
- `game_simulation.script` - Real-time game loop simulation

## Common Helper Module

The `common/` directory contains shared utilities used across benchmark files:

### `common/mod.rs`
- **`CompilationContext`**: Provides a complete compilation context with symbol table, type registry, and inference engine
- **`compile_to_ir()`**: Full compilation pipeline from source to IR
- **`parse_source()`**: Parse source code to AST
- **`analyze_ast()`**: Run semantic analysis on AST
- **`lower_ast_to_ir()`**: Lower AST to IR with proper context
- **`BenchmarkFixture`**: Reusable fixture with programs of different sizes
- **`sample_programs`**: Collection of sample programs for testing:
  - `ARITHMETIC`: Simple arithmetic operations
  - `FUNCTIONS`: Function definitions and calls
  - `CONTROL_FLOW`: Loops and conditionals
  - `COMPLEX`: Complex program with multiple features
- **Memory measurement utilities**: Helpers for memory benchmarking

### Usage Example:
```rust
use common::{CompilationContext, parse_source, BenchmarkFixture};

// Use the common compilation context
let source = "let x = 42;";
let ast = parse_source(source)?;

// Use pre-built fixtures
let fixture = BenchmarkFixture::new();
let program = fixture.large_source;
```

This shared module ensures consistency across benchmarks and reduces code duplication.

## Understanding Results

Criterion generates detailed HTML reports in `target/criterion/`. Each benchmark shows:

- **Mean time**: Average execution time
- **Standard deviation**: Consistency of results
- **Throughput**: Operations per second
- **Comparison**: Change from baseline (if set)

### Interpreting Memory Benchmarks

Memory benchmarks report in "time" units but actually measure bytes:
- The reported "time" is actually memory usage in nanoseconds
- 1 ns = 1 byte of memory
- This allows using Criterion's visualization tools

## Performance Goals

Target performance metrics:

| Operation | Target | Current |
|-----------|--------|---------|
| Lexing | 1M tokens/sec | TBD |
| Parsing | 500K nodes/sec | TBD |
| Compilation | 100K lines/sec | TBD |
| Function calls | 10M calls/sec | TBD |
| Pattern matching | 5M matches/sec | TBD |
| GC pause | < 10ms | TBD |

## Continuous Performance Tracking

The `run_benchmarks.sh` script generates timestamped results that can be tracked over time:

1. Results are saved to `target/benchmark-results/YYYYMMDD_HHMMSS/`
2. Use `track_performance.py` to extract metrics
3. Compare runs with `compare.sh`

## Adding New Benchmarks

To add a new benchmark:

1. Create or modify a benchmark file in `benches/`
2. Add the benchmark function using Criterion's API
3. Update `Cargo.toml` if creating a new file:
   ```toml
   [[bench]]
   name = "your_benchmark"
   harness = false
   ```
4. Add fixture files if needed
5. Update this README

## Best Practices

1. **Use `black_box`** to prevent compiler optimizations from affecting results
2. **Warm up** the benchmark with `iter_custom` for JIT-compiled code
3. **Test realistic scenarios** not just micro-benchmarks
4. **Monitor both time and memory** for comprehensive analysis
5. **Set appropriate measurement time** for slow operations
6. **Document assumptions** and test conditions

## Troubleshooting

### Common Issues and Solutions

#### 1. Benchmark Compilation Errors
**Problem**: Compilation errors in unimplemented benchmark files
```
error[E0433]: failed to resolve: use of undeclared crate or module
```

**Solutions**:
- Run only implemented benchmarks: `cargo bench --bench lexer --bench parser`
- Comment out unimplemented imports in benchmark files
- Focus on lexer and parser benchmarks during early development

#### 2. Runtime Execution Failures
**Problem**: Benchmarks calling `runtime.execute()` panic or fail
```
thread 'main' panicked at 'Runtime execution not implemented'
```

**Solutions**:
- These failures are expected - the runtime is not implemented yet
- Use `cargo bench --bench compilation` to test compilation stages only
- Skip runtime-dependent benchmarks until execution engine is ready

#### 3. Python Dashboard Generation Issues
**Problem**: `generate_dashboard.py` fails to run
```
ImportError: No module named 'matplotlib'
```

**Solutions**:
```bash
# Install Python dependencies
pip install -r requirements.txt

# Or install manually
pip install matplotlib>=3.5.0 numpy>=1.21.0

# Check installation
python -c "import matplotlib; print(matplotlib.__version__)"
```

#### 4. Missing Benchmark Results
**Problem**: Dashboard shows "No results available"

**Solutions**:
```bash
# Ensure benchmarks have been run first
cargo bench --bench lexer

# Check results directory exists
ls -la target/benchmark-results/

# Run the shell script to generate results
./run_benchmarks.sh
```

#### 5. Performance Issues

**Benchmarks Take Too Long**:
Adjust measurement time in benchmark code:
```rust
group.measurement_time(Duration::from_secs(10));
```

**Inconsistent Results**:
- Check for background processes: `htop` or `task manager`
- Disable CPU frequency scaling: `sudo cpufreq-set -g performance`
- Run with release profile: `cargo bench --profile release`
- Increase iteration count for stable results

**Memory Benchmarks Show Strange Results**:
- Ensure the custom allocator is working properly
- Check for memory leaks in test programs
- Verify cleanup between benchmark iterations
- Monitor system memory: `free -h` (Linux) or Activity Monitor (macOS)

#### 6. LSP and Tooling Benchmark Failures
**Problem**: LSP-related benchmarks fail to compile
```
error[E0433]: failed to resolve: use of undeclared crate or module `lsp`
```

**Solutions**:
- LSP features are not implemented yet
- Skip tooling benchmarks: `cargo bench --exclude tooling`
- Focus on core language benchmarks first

#### 7. Fixture File Issues
**Problem**: Benchmark fixtures are missing or invalid

**Solutions**:
```bash
# Check fixture directory
ls -la benches/fixtures/

# Verify fixture content
head benches/fixtures/fibonacci_recursive.script

# Regenerate fixtures if needed (they contain placeholder content for unimplemented features)
```

### Getting Help

If you encounter issues not covered here:

1. **Check Implementation Status**: Review the status table at the top of this README
2. **Run Minimal Tests**: Start with `cargo bench --bench lexer` 
3. **Check Error Messages**: Most errors indicate unimplemented features
4. **Focus on Working Benchmarks**: Use lexer and parser benchmarks during development
5. **Create Issues**: Report unexpected failures (not unimplemented features) on GitHub

### Expected Behavior

During early development, expect:
- ✅ Lexer benchmarks to work fully
- ✅ Parser benchmarks to work fully
- 🚧 Compilation benchmarks to partially work
- ❌ Runtime, memory, and tooling benchmarks to fail

This is normal and expected! The benchmark suite is designed to grow with the language implementation.

## Future Improvements

### Near-term (as language features are implemented):
- [ ] Implement runtime execution benchmarks (requires execution engine)
- [ ] Add memory management and GC benchmarks (requires runtime)
- [ ] Pattern matching performance tests (requires pattern matching implementation)
- [ ] Async/await benchmarks (requires async runtime)

### Medium-term:
- [ ] LSP operation benchmarks (requires LSP server)
- [ ] Documentation generation benchmarks (requires doc system)
- [ ] Test framework performance tests (requires test system)
- [ ] Module system benchmarks (requires module resolution)

### Long-term optimizations:
- [ ] Add flame graphs for profiling
- [ ] Integrate with CI for regression detection
- [ ] Add startup time benchmarks
- [ ] Benchmark WASM compilation target
- [ ] Add energy consumption metrics
- [ ] Incremental compilation benchmarks
- [ ] Cross-platform performance comparisons

### Known Issues to Address:
- [ ] Fix compilation errors in unimplemented benchmark files
- [ ] Add graceful fallbacks for missing features
- [ ] Improve error messages for expected failures
- [ ] Add feature flags to conditionally compile benchmarks
- [ ] Create separate benchmark profiles for different development stages