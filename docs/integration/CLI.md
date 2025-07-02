# Command-Line Interface Reference

This guide provides comprehensive documentation for the Script programming language command-line interface (CLI).

## Table of Contents

- [Installation](#installation)
- [Basic Usage](#basic-usage)
- [Command Reference](#command-reference)
- [REPL Mode](#repl-mode)
- [File Execution](#file-execution)
- [Compilation Options](#compilation-options)
- [Environment Variables](#environment-variables)
- [Configuration Files](#configuration-files)
- [Debugging Options](#debugging-options)
- [Performance Tuning](#performance-tuning)
- [Examples](#examples)
- [Exit Codes](#exit-codes)

## Installation

### From Source

```bash
git clone https://github.com/moikapy/script.git
cd script
cargo build --release
sudo cp target/release/script /usr/local/bin/
```

### Using Cargo

```bash
cargo install script
```

### Package Managers

```bash
# Homebrew (macOS/Linux)
brew install script

# Scoop (Windows)
scoop install script

# APT (Ubuntu/Debian)
sudo apt install script
```

## Basic Usage

### Command Syntax

```
script [OPTIONS] [SCRIPT_FILE] [SCRIPT_ARGS...]
```

### Quick Examples

```bash
# Start interactive REPL
script

# Execute a script file
script hello.script

# Show tokens for a script
script hello.script --tokens

# Run with specific optimization level
script hello.script --optimize 3

# Enable debug mode
script hello.script --debug
```

## Command Reference

### General Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--help` | `-h` | Show help message | - |
| `--version` | `-V` | Show version information | - |
| `--verbose` | `-v` | Enable verbose output | off |
| `--quiet` | `-q` | Suppress non-error output | off |
| `--color` | - | Control colored output | auto |

### Execution Modes

| Option | Description | Example |
|--------|-------------|---------|
| `--tokens` | Show tokenization output only | `script file.script --tokens` |
| `--parse` | Show AST parsing output only | `script file.script --parse` |
| `--run` | Execute the script (default) | `script file.script --run` |
| `--compile` | Compile to bytecode/native code | `script file.script --compile` |
| `--check` | Check syntax without execution | `script file.script --check` |

### Optimization Options

| Option | Description | Values | Default |
|--------|-------------|--------|---------|
| `--optimize` | Set optimization level | 0-3 | 0 |
| `--jit` | Enable JIT compilation | true/false | true |
| `--jit-threshold` | JIT compilation threshold | number | 1000 |
| `--inline` | Enable function inlining | true/false | true |
| `--dead-code` | Remove dead code | true/false | true |

### Memory Options

| Option | Description | Format | Default |
|--------|-------------|--------|---------|
| `--heap-size` | Initial heap size | size (e.g., 64MB) | 64MB |
| `--max-heap` | Maximum heap size | size (e.g., 1GB) | unlimited |
| `--stack-size` | Stack size per thread | size (e.g., 8KB) | 8KB |
| `--gc-threshold` | GC trigger threshold | size (e.g., 16MB) | 16MB |

### Debugging Options

| Option | Description | Default |
|--------|-------------|---------|
| `--debug` | Enable debug mode | off |
| `--debug-info` | Include debug information | off |
| `--trace` | Enable execution tracing | off |
| `--profile` | Enable profiling | off |
| `--time` | Show execution time | off |
| `--memory-debug` | Enable memory debugging | off |

### Output Options

| Option | Description | Format |
|--------|-------------|--------|
| `--output` | Output file for compilation | filename |
| `--format` | Output format | json, yaml, text |
| `--ast-format` | AST output format | json, yaml, tree |
| `--emit` | What to emit | tokens, ast, ir, asm |

### Advanced Options

| Option | Description | Values |
|--------|-------------|--------|
| `--target` | Compilation target | native, wasm, js |
| `--features` | Enable language features | feature1,feature2 |
| `--sandbox` | Enable sandbox mode | true/false |
| `--no-stdlib` | Disable standard library | - |
| `--lib-path` | Additional library paths | path1:path2 |

## REPL Mode

### Starting the REPL

```bash
# Start default REPL (parse mode)
script

# Start in token mode
script --tokens

# Start with specific configuration
script --heap-size 128MB --jit true
```

### REPL Commands

| Command | Description | Example |
|---------|-------------|---------|
| `:help` | Show help | `:help` |
| `:exit` | Exit REPL | `:exit` |
| `:quit` | Exit REPL | `:quit` |
| `:tokens` | Switch to token mode | `:tokens` |
| `:parse` | Switch to parse mode | `:parse` |
| `:run` | Switch to run mode | `:run` |
| `:clear` | Clear screen | `:clear` |
| `:history` | Show command history | `:history` |
| `:save` | Save session to file | `:save session.script` |
| `:load` | Load script file | `:load example.script` |
| `:reset` | Reset REPL state | `:reset` |
| `:env` | Show environment variables | `:env` |
| `:memory` | Show memory statistics | `:memory` |
| `:profile` | Show profiling data | `:profile` |
| `:time` | Toggle timing display | `:time` |
| `:debug` | Toggle debug mode | `:debug` |

### REPL Configuration

```bash
# Set REPL configuration via environment
export SCRIPT_REPL_HISTORY_SIZE=1000
export SCRIPT_REPL_AUTO_SAVE=true
export SCRIPT_REPL_PROMPT=">>> "
export SCRIPT_REPL_MULTILINE=true

script
```

### REPL Example Session

```
$ script
Script v0.1.0 - The Script Programming Language
Type 'exit' to quit
Type ':tokens' to switch to token mode
Type ':parse' to switch to parse mode (default)

script> let x = 42
AST:
────────────────────────────────────────────────────────────
Program {
  statements: [
    Let {
      name: "x",
      type_annotation: None,
      initializer: Some(Literal(Number(42))),
    },
  ],
}
────────────────────────────────────────────────────────────

script> fn add(a: i32, b: i32) -> i32 { return a + b }
AST:
────────────────────────────────────────────────────────────
Program {
  statements: [
    Function {
      name: "add",
      parameters: [
        Parameter { name: "a", type_annotation: Some(I32) },
        Parameter { name: "b", type_annotation: Some(I32) },
      ],
      return_type: Some(I32),
      body: Block([
        Return(Some(Binary {
          left: Identifier("a"),
          operator: Plus,
          right: Identifier("b"),
        })),
      ]),
    },
  ],
}
────────────────────────────────────────────────────────────

script> :tokens
Switched to token mode

tokens> add(10, 20)
Tokens:
────────────────────────────────────────────────────────────
   1:1    Identifier           add
   1:4    LeftParen            (
   1:5    Number               10
   1:7    Comma                ,
   1:8    Number               20
   1:11   RightParen           )
   1:12   Eof                  
────────────────────────────────────────────────────────────

tokens> :exit
Goodbye!
```

## File Execution

### Basic File Execution

```bash
# Execute a script file
script hello.script

# Execute with arguments
script calculator.script 10 + 5

# Execute with environment variables
SCRIPT_DEBUG=1 script debug_example.script
```

### File Extensions

Script recognizes the following file extensions:

| Extension | Description |
|-----------|-------------|
| `.script` | Standard Script source files |
| `.sc` | Short Script source files |
| `.spt` | Alternative Script extension |

### Input/Output Redirection

```bash
# Read from stdin
echo "print('Hello')" | script

# Redirect output
script hello.script > output.txt

# Redirect both stdout and stderr
script hello.script &> all_output.txt

# Pipe to other commands
script data_processor.script | grep "result"
```

### Batch Processing

```bash
# Process multiple files
script *.script

# Process files in directory
find scripts/ -name "*.script" -exec script {} \;

# Parallel execution
parallel script ::: *.script
```

## Compilation Options

### Compilation Modes

```bash
# Compile to bytecode
script --compile hello.script

# Compile to native code
script --compile --target native hello.script

# Compile to WebAssembly
script --compile --target wasm hello.script

# Compile to JavaScript
script --compile --target js hello.script
```

### Optimization Levels

```bash
# No optimization (debug)
script --optimize 0 hello.script

# Basic optimization
script --optimize 1 hello.script

# Standard optimization
script --optimize 2 hello.script

# Aggressive optimization
script --optimize 3 hello.script
```

### Output Control

```bash
# Specify output file
script --compile --output hello.exe hello.script

# Emit intermediate representations
script --emit tokens hello.script
script --emit ast hello.script
script --emit ir hello.script
script --emit asm hello.script

# Multiple outputs
script --emit tokens,ast,ir hello.script
```

## Environment Variables

### Runtime Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `SCRIPT_HEAP_SIZE` | Initial heap size | 64MB |
| `SCRIPT_MAX_HEAP` | Maximum heap size | unlimited |
| `SCRIPT_STACK_SIZE` | Stack size | 8KB |
| `SCRIPT_GC_THRESHOLD` | GC threshold | 16MB |

### Compilation Settings

| Variable | Description | Default |
|----------|-------------|---------|
| `SCRIPT_OPTIMIZE` | Optimization level | 0 |
| `SCRIPT_JIT` | Enable JIT | true |
| `SCRIPT_JIT_THRESHOLD` | JIT threshold | 1000 |
| `SCRIPT_TARGET` | Compilation target | native |

### Debugging Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `SCRIPT_DEBUG` | Enable debug mode | false |
| `SCRIPT_TRACE` | Enable tracing | false |
| `SCRIPT_PROFILE` | Enable profiling | false |
| `SCRIPT_LOG_LEVEL` | Log level | info |

### System Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `SCRIPT_PATH` | Script library paths | system default |
| `SCRIPT_CONFIG` | Configuration file path | ~/.script/config |
| `SCRIPT_CACHE_DIR` | Cache directory | ~/.script/cache |
| `SCRIPT_TEMP_DIR` | Temporary directory | system temp |

### Usage Examples

```bash
# Set heap size
export SCRIPT_HEAP_SIZE=128MB
script memory_intensive.script

# Enable debugging
export SCRIPT_DEBUG=1
export SCRIPT_LOG_LEVEL=debug
script debug_example.script

# Configure JIT
export SCRIPT_JIT=true
export SCRIPT_JIT_THRESHOLD=500
script performance_test.script

# Set library paths
export SCRIPT_PATH="/usr/local/lib/script:/home/user/script_libs"
script app_with_libs.script
```

## Configuration Files

### Global Configuration

Location: `~/.script/config.toml`

```toml
[runtime]
heap_size = "64MB"
max_heap = "1GB"
stack_size = "8KB"
gc_threshold = "16MB"
enable_jit = true
jit_threshold = 1000

[compilation]
optimize = 2
target = "native"
emit_debug_info = false
inline_functions = true
dead_code_elimination = true

[debugging]
enable_debug = false
enable_trace = false
enable_profile = false
log_level = "info"

[repl]
history_size = 1000
auto_save = true
prompt = "script> "
multiline = true
color = true

[paths]
library_paths = ["/usr/local/lib/script", "~/.script/lib"]
cache_dir = "~/.script/cache"
temp_dir = "/tmp/script"
```

### Project Configuration

Location: `script.toml` (in project directory)

```toml
[project]
name = "my-project"
version = "1.0.0"
description = "My Script project"
authors = ["Your Name <your.email@example.com>"]

[dependencies]
math = "1.0"
json = "0.5"

[build]
target = "native"
optimize = 2
features = ["async", "networking"]

[runtime]
heap_size = "128MB"
enable_jit = true

[scripts]
main = "src/main.script"
test = "tests/test.script"
```

### Environment-Specific Configuration

```toml
# Development environment
[profiles.dev]
optimize = 0
enable_debug = true
enable_trace = true

# Production environment
[profiles.prod]
optimize = 3
enable_debug = false
strip_debug_info = true

# Testing environment
[profiles.test]
optimize = 1
enable_profile = true
memory_debug = true
```

## Debugging Options

### Debug Mode

```bash
# Enable debug mode
script --debug hello.script

# Enable with specific log level
script --debug --log-level debug hello.script

# Enable tracing
script --debug --trace hello.script
```

### Memory Debugging

```bash
# Enable memory debugging
script --memory-debug hello.script

# Show memory statistics
script --memory-debug --verbose hello.script

# Memory leak detection
script --memory-debug --leak-check hello.script
```

### Performance Profiling

```bash
# Enable profiling
script --profile hello.script

# Profile with specific options
script --profile --profile-output profile.json hello.script

# CPU profiling
script --profile --profile-cpu hello.script

# Memory profiling
script --profile --profile-memory hello.script
```

### Execution Tracing

```bash
# Basic tracing
script --trace hello.script

# Detailed tracing
script --trace --trace-level verbose hello.script

# Trace specific operations
script --trace --trace-ops "call,return,assign" hello.script
```

## Performance Tuning

### Memory Tuning

```bash
# Large heap for memory-intensive applications
script --heap-size 512MB --max-heap 2GB hello.script

# Small heap for memory-constrained environments
script --heap-size 16MB --max-heap 64MB hello.script

# Tune garbage collection
script --gc-threshold 32MB --gc-frequency 1000 hello.script
```

### Compilation Tuning

```bash
# Fast compilation, slower execution
script --optimize 0 --jit false hello.script

# Slow compilation, fast execution
script --optimize 3 --jit true --jit-threshold 100 hello.script

# Balanced performance
script --optimize 2 --jit true --jit-threshold 500 hello.script
```

### JIT Tuning

```bash
# Aggressive JIT compilation
script --jit true --jit-threshold 10 hello.script

# Conservative JIT compilation
script --jit true --jit-threshold 10000 hello.script

# Disable JIT for debugging
script --jit false hello.script
```

## Examples

### Hello World

```bash
# Create hello.script
echo 'print("Hello, World!")' > hello.script

# Run it
script hello.script
```

### Calculator

```bash
# Create calculator.script
cat > calculator.script << 'EOF'
fn main(args: [string]) -> i32 {
    if args.length < 3 {
        print("Usage: calculator <num1> <op> <num2>")
        return 1
    }
    
    let a = parse_float(args[0])
    let op = args[1]
    let b = parse_float(args[2])
    
    let result = match op {
        "+" => a + b,
        "-" => a - b,
        "*" => a * b,
        "/" => a / b,
        _ => {
            print("Unknown operator: " + op)
            return 1
        }
    }
    
    print(a + " " + op + " " + b + " = " + result)
    return 0
}
EOF

# Use it
script calculator.script 10 + 5
# Output: 10 + 5 = 15
```

### Fibonacci

```bash
# Create fibonacci.script
cat > fibonacci.script << 'EOF'
fn fibonacci(n: i32) -> i32 {
    if n <= 1 {
        return n
    }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

fn main(args: [string]) -> i32 {
    let n = if args.length > 0 {
        parse_int(args[0])
    } else {
        10
    }
    
    print("Fibonacci(" + n + ") = " + fibonacci(n))
    return 0
}
EOF

# Run with different optimizations
script --optimize 0 fibonacci.script 30  # Slow
script --optimize 3 fibonacci.script 30  # Fast
```

### Debugging Example

```bash
# Create debug_example.script
cat > debug_example.script << 'EOF'
fn buggy_function(x: i32) -> i32 {
    let y = x * 2
    print("Debug: x = " + x + ", y = " + y)
    return y / (x - 5)  // Division by zero when x = 5
}

fn main() -> i32 {
    for i in 1..10 {
        let result = buggy_function(i)
        print("Result: " + result)
    }
    return 0
}
EOF

# Debug it
script --debug --trace debug_example.script
```

### Profiling Example

```bash
# Create performance_test.script
cat > performance_test.script << 'EOF'
fn expensive_computation(n: i32) -> i32 {
    let sum = 0
    for i in 1..n {
        sum += i * i
    }
    return sum
}

fn main() -> i32 {
    let start = time_now()
    let result = expensive_computation(1000000)
    let end = time_now()
    
    print("Result: " + result)
    print("Time: " + (end - start) + "ms")
    return 0
}
EOF

# Profile it
script --profile --time performance_test.script
```

## Exit Codes

The Script CLI uses standard exit codes:

| Code | Meaning | Description |
|------|---------|-------------|
| 0 | Success | Script executed successfully |
| 1 | General error | Generic error occurred |
| 2 | Parse error | Syntax error in script |
| 3 | Runtime error | Error during script execution |
| 4 | Type error | Type checking failed |
| 5 | IO error | File I/O error |
| 6 | Memory error | Out of memory or memory corruption |
| 7 | System error | System-level error |
| 8 | Configuration error | Invalid configuration |
| 9 | Interrupt | Script was interrupted (Ctrl+C) |
| 10 | Timeout | Script execution timed out |

### Handling Exit Codes

```bash
# Check exit code
script hello.script
if [ $? -eq 0 ]; then
    echo "Script succeeded"
else
    echo "Script failed with code $?"
fi

# Use in conditional
if script test.script; then
    echo "Tests passed"
    script deploy.script
else
    echo "Tests failed"
    exit 1
fi
```

This comprehensive CLI reference provides complete documentation for using the Script programming language command-line interface effectively in various scenarios.