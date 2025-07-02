#!/bin/bash
# Script to run all benchmarks and generate reports

set -e

BENCH_OUTPUT_DIR="target/benchmark-results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULTS_DIR="$BENCH_OUTPUT_DIR/$TIMESTAMP"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Script Language Performance Benchmark Suite${NC}"
echo "============================================"
echo ""

# Create results directory
mkdir -p "$RESULTS_DIR"

# Function to run a benchmark and save results
run_benchmark() {
    local bench_name=$1
    echo -e "${YELLOW}Running benchmark: $bench_name${NC}"
    
    # Run the benchmark and save both stdout and the criterion output
    cargo bench --bench "$bench_name" 2>&1 | tee "$RESULTS_DIR/${bench_name}.txt"
    
    # Copy criterion's HTML report if it exists
    if [ -d "target/criterion/$bench_name" ]; then
        cp -r "target/criterion/$bench_name" "$RESULTS_DIR/"
    fi
    
    echo -e "${GREEN}✓ Completed $bench_name${NC}\n"
}

# Run all benchmarks
BENCHMARKS=(
    "lexer"
    "parser"
    "compilation"
    "features"
    "scenarios"
    "memory"
    "tooling"
)

echo "Running ${#BENCHMARKS[@]} benchmark suites..."
echo ""

for bench in "${BENCHMARKS[@]}"; do
    run_benchmark "$bench"
done

# Generate summary report
echo -e "${YELLOW}Generating summary report...${NC}"

cat > "$RESULTS_DIR/summary.md" << EOF
# Script Language Benchmark Results

Generated on: $(date)

## Benchmark Suites

### 1. Lexer Performance
Tests tokenization speed for various program sizes and complexities.

### 2. Parser Performance
Measures AST generation speed for different code patterns.

### 3. Compilation Pipeline
Benchmarks the full compilation process from source to executable.

### 4. Language Features
Tests performance of specific language features:
- Pattern matching
- Async/await operations
- Function calls
- Collections (arrays, hashmaps, sets)
- String operations
- Mathematical computations

### 5. Real-world Scenarios
Benchmarks based on realistic usage patterns:
- Fibonacci calculation (iterative vs recursive)
- Sorting algorithms
- Tree traversal
- Web server simulation
- Game loop simulation
- Data processing pipeline

### 6. Memory Usage
Measures memory consumption and garbage collection performance:
- Compilation memory usage
- Runtime memory allocation
- GC performance with cyclic references
- Memory fragmentation patterns

### 7. Tooling Performance
Tests development tool performance:
- LSP operations (completion, goto definition)
- Documentation generation
- Test discovery and execution
- Code formatting
- Package management

## Results

See individual benchmark files for detailed results.

## Criterion Reports

HTML reports with graphs are available in the subdirectories.
EOF

# Create comparison script
cat > "$RESULTS_DIR/compare.sh" << 'EOF'
#!/bin/bash
# Compare benchmark results between runs

if [ $# -lt 2 ]; then
    echo "Usage: $0 <baseline-dir> <comparison-dir>"
    exit 1
fi

BASELINE=$1
COMPARISON=$2

echo "Comparing benchmarks:"
echo "Baseline: $BASELINE"
echo "Comparison: $COMPARISON"
echo ""

for bench in lexer parser compilation features scenarios memory tooling; do
    echo "=== $bench ==="
    if [ -f "$BASELINE/$bench.txt" ] && [ -f "$COMPARISON/$bench.txt" ]; then
        # Extract timing information and compare
        grep -E "time:.*\[(.*)\]" "$BASELINE/$bench.txt" | head -5
        echo "vs"
        grep -E "time:.*\[(.*)\]" "$COMPARISON/$bench.txt" | head -5
    else
        echo "Missing benchmark results"
    fi
    echo ""
done
EOF

chmod +x "$RESULTS_DIR/compare.sh"

# Generate performance tracking CSV
cat > "$RESULTS_DIR/track_performance.py" << 'EOF'
#!/usr/bin/env python3
"""
Track performance over time by parsing benchmark results
"""

import os
import re
import csv
import json
from datetime import datetime
from pathlib import Path

def parse_criterion_output(file_path):
    """Parse criterion benchmark output file"""
    results = {}
    
    with open(file_path, 'r') as f:
        content = f.read()
        
    # Extract benchmark results using regex
    pattern = r'(\w+)\s+time:\s+\[([0-9.]+)\s+(\w+)'
    matches = re.findall(pattern, content)
    
    for name, time, unit in matches:
        results[name] = {
            'time': float(time),
            'unit': unit
        }
    
    return results

def main():
    results_dir = Path('.')
    timestamp = results_dir.name
    
    all_results = {
        'timestamp': timestamp,
        'benchmarks': {}
    }
    
    # Parse all benchmark files
    for bench_file in results_dir.glob('*.txt'):
        if bench_file.name != 'summary.txt':
            bench_name = bench_file.stem
            results = parse_criterion_output(bench_file)
            all_results['benchmarks'][bench_name] = results
    
    # Save as JSON for further processing
    with open('performance_data.json', 'w') as f:
        json.dump(all_results, f, indent=2)
    
    print(f"Performance data saved to performance_data.json")

if __name__ == '__main__':
    main()
EOF

chmod +x "$RESULTS_DIR/track_performance.py"

echo -e "${GREEN}✅ All benchmarks completed!${NC}"
echo ""
echo "Results saved to: $RESULTS_DIR"
echo ""
echo "To view HTML reports:"
echo "  open $RESULTS_DIR/*/report/index.html"
echo ""
echo "To compare with another run:"
echo "  $RESULTS_DIR/compare.sh <baseline-dir> <comparison-dir>"
echo ""
echo "To track performance over time:"
echo "  cd $RESULTS_DIR && python3 track_performance.py"