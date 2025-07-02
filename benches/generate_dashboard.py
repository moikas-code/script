#!/usr/bin/env python3
"""
Generate a performance dashboard from benchmark results
"""

import os
import re
import json
import glob
from datetime import datetime
from pathlib import Path
from collections import defaultdict

# Try to import matplotlib, but don't fail if it's not available
try:
    import matplotlib
    matplotlib.use('Agg')  # Use non-interactive backend
    import matplotlib.pyplot as plt
    import matplotlib.dates as mdates
    MATPLOTLIB_AVAILABLE = True
except ImportError:
    MATPLOTLIB_AVAILABLE = False
    print("Warning: matplotlib not installed. Charts will not be generated.")
    print("To install matplotlib, run: pip install matplotlib")

def parse_benchmark_results(results_dir):
    """Parse all benchmark results in a directory"""
    all_results = []
    
    # Check if results directory exists
    if not os.path.exists(results_dir):
        print(f"Error: Results directory '{results_dir}' does not exist.")
        print(f"Please run benchmarks first with: cargo bench")
        return []
    
    # Find all timestamp directories
    timestamp_dirs = sorted(glob.glob(f"{results_dir}/*"))
    if not timestamp_dirs:
        print(f"Warning: No benchmark results found in '{results_dir}'")
        return []
    
    for timestamp_dir in timestamp_dirs:
        if not os.path.isdir(timestamp_dir):
            continue
            
        timestamp = os.path.basename(timestamp_dir)
        try:
            dt = datetime.strptime(timestamp, "%Y%m%d_%H%M%S")
        except ValueError:
            continue
            
        run_results = {
            'timestamp': dt,
            'benchmarks': defaultdict(dict)
        }
        
        # Parse each benchmark file
        bench_files = glob.glob(f"{timestamp_dir}/*.txt")
        if not bench_files:
            print(f"Warning: No benchmark files found in '{timestamp_dir}'")
            continue
            
        for bench_file in bench_files:
            bench_name = Path(bench_file).stem
            
            try:
                with open(bench_file, 'r') as f:
                    content = f.read()
            except Exception as e:
                print(f"Warning: Failed to read '{bench_file}': {e}")
                continue
                
            # Validate and extract benchmark timings
            if not content.strip():
                print(f"Warning: Empty benchmark file '{bench_file}'")
                continue
                
            # Pattern to match benchmark results with various time units including Unicode µ
            pattern = r'(\w+)\s+time:\s+\[([0-9.]+)\s+([µμ]?[a-zA-Z]+)\s+([0-9.]+)\s+([µμ]?[a-zA-Z]+)\s+([0-9.]+)\s+([µμ]?[a-zA-Z]+)\]'
            matches = re.findall(pattern, content)
            
            if not matches:
                print(f"Warning: No benchmark results found in '{bench_file}'")
                continue
            
            for match in matches:
                try:
                    test_name = match[0]
                    mean_time = float(match[3])
                    unit = match[4]
                    
                    # Validate time value
                    if mean_time < 0:
                        print(f"Warning: Invalid negative time for {test_name} in {bench_file}")
                        continue
                    
                    # Convert to nanoseconds for consistency
                    if unit == 'ns':
                        pass  # Already in nanoseconds
                    elif unit in ['us', 'µs', 'μs']:  # Handle different microsecond representations
                        mean_time *= 1000
                    elif unit == 'ms':
                        mean_time *= 1000000
                    elif unit == 's':
                        mean_time *= 1000000000
                    else:
                        print(f"Warning: Unknown time unit '{unit}' for {test_name} in {bench_file}")
                        continue
                        
                    run_results['benchmarks'][bench_name][test_name] = mean_time
                    
                except (ValueError, IndexError) as e:
                    print(f"Warning: Failed to parse benchmark result '{match}' in {bench_file}: {e}")
                    continue
                
        if run_results['benchmarks']:
            all_results.append(run_results)
            
    return all_results

def generate_charts(results, output_dir):
    """Generate performance charts"""
    if not MATPLOTLIB_AVAILABLE:
        print("Skipping chart generation (matplotlib not available)")
        return
        
    try:
        os.makedirs(output_dir, exist_ok=True)
    except Exception as e:
        print(f"Error: Failed to create output directory '{output_dir}': {e}")
        return
    
    # Group results by benchmark suite
    benchmarks = defaultdict(lambda: defaultdict(list))
    timestamps = []
    
    for run in results:
        timestamps.append(run['timestamp'])
        for suite, tests in run['benchmarks'].items():
            for test, time in tests.items():
                benchmarks[suite][test].append(time)
    
    # Generate chart for each benchmark suite
    for suite, tests in benchmarks.items():
        try:
            fig, ax = plt.subplots(figsize=(12, 8))
            
            # Check if we have any valid data for this suite
            has_data = False
            for test_name, times in tests.items():
                # Pad with None for missing data points
                padded_times = []
                for i, run in enumerate(results):
                    if suite in run['benchmarks'] and test_name in run['benchmarks'][suite]:
                        padded_times.append(run['benchmarks'][suite][test_name])
                        has_data = True
                    else:
                        padded_times.append(None)
                        
                if has_data:
                    ax.plot(timestamps[:len(padded_times)], padded_times, marker='o', label=test_name)
            
            if not has_data:
                print(f"Warning: No data found for benchmark suite '{suite}'")
                plt.close()
                continue
            
            ax.set_xlabel('Date')
            ax.set_ylabel('Time (ns)')
            ax.set_title(f'{suite.capitalize()} Benchmark Performance')
            ax.legend(bbox_to_anchor=(1.05, 1), loc='upper left')
            ax.grid(True, alpha=0.3)
            
            # Format x-axis
            ax.xaxis.set_major_formatter(mdates.DateFormatter('%Y-%m-%d'))
            ax.xaxis.set_major_locator(mdates.DayLocator())
            plt.xticks(rotation=45)
            
            plt.tight_layout()
            
            chart_path = f"{output_dir}/{suite}_performance.png"
            plt.savefig(chart_path, dpi=150)
            plt.close()
            print(f"Generated chart: {chart_path}")
            
        except Exception as e:
            print(f"Error generating chart for '{suite}': {e}")
            plt.close()  # Ensure we close the figure even on error

def generate_html_dashboard(results, output_dir):
    """Generate HTML dashboard"""
    try:
        os.makedirs(output_dir, exist_ok=True)
    except Exception as e:
        print(f"Error: Failed to create output directory '{output_dir}': {e}")
        return
    html = """
<!DOCTYPE html>
<html>
<head>
    <title>Script Language Performance Dashboard</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        h1 {{ color: #333; }}
        .benchmark-section {{ margin: 20px 0; }}
        .chart {{ margin: 10px 0; }}
        table {{ border-collapse: collapse; width: 100%; margin: 20px 0; }}
        th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}
        th {{ background-color: #f2f2f2; }}
        .improvement {{ color: green; }}
        .regression {{ color: red; }}
        .timestamp {{ font-size: 0.9em; color: #666; }}
    </style>
</head>
<body>
    <h1>Script Language Performance Dashboard</h1>
    <p class="timestamp">Last updated: {timestamp}</p>
    
    <h2>Performance Trends</h2>
    {charts}
    
    <h2>Latest Results</h2>
    {latest_results}
    
    <h2>Performance Comparisons</h2>
    {comparisons}
</body>
</html>
"""
    
    # Generate charts section
    charts_html = ""
    for suite in ['lexer', 'parser', 'compilation', 'features', 'scenarios', 'memory', 'tooling']:
        if os.path.exists(f"{output_dir}/{suite}_performance.png"):
            charts_html += f'<div class="chart"><h3>{suite.capitalize()}</h3><img src="{suite}_performance.png" width="800"></div>\n'
    
    # Generate latest results table
    if results:
        latest = results[-1]
        latest_html = "<table><tr><th>Benchmark</th><th>Test</th><th>Time</th></tr>"
        
        for suite, tests in sorted(latest['benchmarks'].items()):
            for test, time in sorted(tests.items()):
                time_str = format_time(time)
                latest_html += f"<tr><td>{suite}</td><td>{test}</td><td>{time_str}</td></tr>"
        
        latest_html += "</table>"
    else:
        latest_html = "<p>No results available</p>"
    
    # Generate comparison table (last vs previous)
    comparison_html = ""
    if len(results) >= 2:
        latest = results[-1]
        previous = results[-2]
        
        comparison_html = "<table><tr><th>Benchmark</th><th>Test</th><th>Previous</th><th>Latest</th><th>Change</th></tr>"
        
        for suite in latest['benchmarks']:
            if suite not in previous['benchmarks']:
                continue
                
            for test in latest['benchmarks'][suite]:
                if test not in previous['benchmarks'][suite]:
                    continue
                    
                prev_time = previous['benchmarks'][suite][test]
                curr_time = latest['benchmarks'][suite][test]
                
                # Prevent division by zero
                if prev_time == 0:
                    change = 0 if curr_time == 0 else float('inf')
                    change_str = "N/A" if curr_time == 0 else "+∞%"
                    change_class = ""
                else:
                    change = ((curr_time - prev_time) / prev_time) * 100
                    change_str = f"{change:+.1f}%" if abs(change) > 0.1 else "~"
                    change_class = "improvement" if change < 0 else "regression" if change > 5 else ""
                
                comparison_html += f"""<tr>
                    <td>{suite}</td>
                    <td>{test}</td>
                    <td>{format_time(prev_time)}</td>
                    <td>{format_time(curr_time)}</td>
                    <td class="{change_class}">{change_str}</td>
                </tr>"""
        
        comparison_html += "</table>"
    else:
        comparison_html = "<p>Need at least 2 runs for comparison</p>"
    
    # Generate final HTML
    final_html = html.format(
        timestamp=datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
        charts=charts_html,
        latest_results=latest_html,
        comparisons=comparison_html
    )
    
    try:
        dashboard_path = f"{output_dir}/dashboard.html"
        with open(dashboard_path, 'w') as f:
            f.write(final_html)
        print(f"Generated dashboard: {dashboard_path}")
    except Exception as e:
        print(f"Error: Failed to write dashboard HTML: {e}")

def format_time(nanoseconds):
    """Format time in appropriate units"""
    if nanoseconds < 1000:
        return f"{nanoseconds:.0f} ns"
    elif nanoseconds < 1000000:
        return f"{nanoseconds/1000:.1f} μs"
    elif nanoseconds < 1000000000:
        return f"{nanoseconds/1000000:.1f} ms"
    else:
        return f"{nanoseconds/1000000000:.2f} s"

def main():
    """Main function to generate the performance dashboard"""
    results_dir = "target/benchmark-results"
    output_dir = "target/benchmark-dashboard"
    
    print("Script Language Performance Dashboard Generator")
    print("=" * 50)
    
    print("Parsing benchmark results...")
    results = parse_benchmark_results(results_dir)
    
    if not results:
        print("No valid benchmark results found!")
        print("\nTo generate benchmark results, run:")
        print("  cargo bench")
        print("  ./benches/run_benchmarks.sh")
        return 1
    
    print(f"Found {len(results)} benchmark runs")
    
    # Generate charts if matplotlib is available
    if MATPLOTLIB_AVAILABLE:
        print("Generating performance charts...")
        generate_charts(results, output_dir)
    else:
        print("Skipping chart generation (matplotlib not available)")
    
    print("Generating HTML dashboard...")
    generate_html_dashboard(results, output_dir)
    
    dashboard_path = os.path.abspath(f"{output_dir}/dashboard.html")
    if os.path.exists(dashboard_path):
        print(f"\nDashboard generated successfully!")
        print(f"Open: file://{dashboard_path}")
        return 0
    else:
        print(f"\nError: Dashboard generation failed!")
        return 1

if __name__ == '__main__':
    import sys
    sys.exit(main())