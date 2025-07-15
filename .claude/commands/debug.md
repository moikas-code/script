# /debug Command Documentation

## Overview

The `/debug` command provides comprehensive debugging and troubleshooting assistance for Script language development. It helps developers diagnose compilation errors, runtime issues, type system problems, and performance bottlenecks through intelligent analysis and automated debugging workflows.

## Purpose

This command accelerates development and problem resolution by:
- Providing intelligent compilation error diagnosis and fix suggestions
- Debugging REPL issues and interactive development problems
- Investigating runtime errors and memory management issues
- Analyzing type system problems and constraint solver failures
- Detecting performance bottlenecks and optimization opportunities
- Offering guided debugging workflows for complex issues

## Usage

### Basic Syntax
```bash
/debug                          # Interactive debugging session
/debug <error_type>            # Debug specific error category
/debug <file_path>             # Debug specific file
/debug --trace <issue>         # Trace execution for specific issue
```

### Error Category Debugging
```bash
/debug compilation             # Compilation pipeline issues
/debug lexer                   # Lexical analysis problems
/debug parser                  # Parsing and syntax errors
/debug semantic                # Type checking and analysis issues
/debug codegen                 # Code generation problems
/debug runtime                 # Runtime execution issues
/debug memory                  # Memory management debugging
/debug performance            # Performance analysis and optimization
/debug repl                   # REPL-specific debugging
/debug mcp                    # MCP integration issues
```

### Advanced Debugging Options
```bash
/debug --interactive          # Launch interactive debugging session
/debug --trace-execution      # Trace program execution step-by-step
/debug --memory-analysis      # Deep memory usage analysis
/debug --performance-profile  # Performance profiling and hotspot detection
/debug --type-inference       # Type inference debugging
/debug --constraint-solver    # Constraint solver debugging
/debug --ast-dump            # AST visualization and analysis
/debug --ir-dump             # IR analysis and optimization debugging
```

### File-Specific Debugging
```bash
/debug src/parser/expression.rs    # Debug specific source file
/debug tests/failing_test.rs       # Debug failing test cases
/debug examples/problematic.script # Debug Script language files
```

## Debugging Categories

### 1. Compilation Debugging
**Purpose**: Diagnose and resolve compilation pipeline issues
**Command**: `/debug compilation`

#### Common Issues Handled:
- **Lexer Errors**: Invalid tokens, character encoding issues
- **Parser Errors**: Syntax errors, grammar conflicts, precedence issues
- **Semantic Errors**: Type mismatches, undefined symbols, scope resolution
- **Codegen Errors**: IR generation failures, optimization problems

#### Example Workflow:
```bash
# User reports compilation failure
/debug compilation

# System analyzes error and provides:
🔍 Compilation Error Analysis
============================
Error: Type mismatch in function call
Location: src/examples/test.script:15:8

Detailed Analysis:
├── Expected type: int
├── Actual type: string
├── Function: calculate(value)
└── Call site: calculate("hello")

Root Cause:
Function `calculate` expects numeric input but received string literal.

Suggested Fixes:
1. Change call to: calculate(42)
2. Overload function for string input
3. Add type conversion: calculate(int("hello"))

Related Issues:
├── Similar error in line 23
└── Type system documentation: kb/docs/type-system.md

Would you like me to:
[A] Apply suggested fix #1
[B] Show more context around error
[C] Explain type conversion options
[D] Create comprehensive fix plan
```

### 2. Type System Debugging
**Purpose**: Debug complex type inference and constraint solving issues
**Command**: `/debug semantic`

#### Advanced Type Debugging:
```bash
/debug --type-inference
```

**Output Example**:
```
🧩 Type Inference Debug Session
===============================
Function: complex_generic_function<T, U>
Location: src/semantic/test_case.rs:45

Type Variable Assignments:
├── T = ?0 (unresolved)
├── U = int (resolved)
└── Return = Result<?0, string>

Constraint Set:
├── ?0 implements Display ✓
├── ?0 implements Clone ✓
├── ?0: Send + Sync ⚠ (unverified)
└── ?0 != () ✓

Inference Steps:
1. T assigned fresh type variable ?0
2. U unified with int from context
3. Display constraint added from format! usage
4. Clone constraint from .clone() call
5. Send+Sync required by async boundary

Blocking Constraint:
└── Cannot verify Send+Sync for ?0

Suggestions:
1. Add explicit bound: <T: Display + Clone + Send + Sync>
2. Remove async requirement
3. Use Arc<T> for shared ownership

Debug Commands:
[constraints] Show all active constraints
[unify ?0 SomeType] Test unification
[trace] Step through inference
```

### 3. Runtime Debugging
**Purpose**: Debug runtime execution issues and memory problems
**Command**: `/debug runtime`

#### Runtime Issue Categories:
- **Memory Leaks**: Garbage collection failures, reference cycles
- **Panic Recovery**: Stack unwinding issues, error propagation
- **Async Runtime**: Deadlocks, task scheduling, resource contention
- **FFI Issues**: Foreign function interface safety and errors
- **Performance**: Slow execution, excessive allocations

#### Memory Debugging Example:
```bash
/debug --memory-analysis
```

**Output**:
```
🧠 Memory Analysis Report
=========================
Execution: test_complex_algorithm()
Duration: 2.4 seconds

Memory Usage Timeline:
├── 0.0s: 12MB (baseline)
├── 0.5s: 45MB (+33MB) - Large allocation in vector.rs:234
├── 1.2s: 89MB (+44MB) - String concatenation loop
├── 2.0s: 127MB (+38MB) - HashMap growth
└── 2.4s: 15MB (-112MB) - GC sweep

Potential Issues:
❌ Memory leak detected in closure captures
⚠ Excessive string allocations (1,247 calls)
⚠ HashMap resize overhead (23% of execution time)

Memory Hotspots:
1. src/algorithms/string_ops.rs:67 (45MB allocated)
2. src/collections/dynamic_array.rs:123 (23MB allocated)
3. src/closure/capture.rs:89 (12MB never freed)

Recommendations:
├── Use string builders instead of concatenation
├── Pre-size HashMap with capacity
└── Fix closure capture leak (add Drop impl)

Commands:
[leak] Focus on memory leak analysis
[alloc] Show allocation patterns
[gc] Analyze garbage collection efficiency
```

### 4. REPL Debugging
**Purpose**: Debug interactive development environment issues
**Command**: `/debug repl`

#### REPL-Specific Issues:
- **Session State**: Variable persistence, import resolution
- **Error Recovery**: Graceful handling of malformed input
- **Performance**: Slow evaluation, memory accumulation
- **Display Issues**: Output formatting, unicode handling

#### REPL Debug Session:
```bash
/debug repl

# Interactive REPL debugging
🎯 REPL Debug Mode
==================
Current Session State:
├── Variables: 7 defined
├── Imports: 3 modules loaded
├── Memory: 4.2MB used
└── History: 23 commands

Last Error:
└── Parse error: Unexpected token 'async' at position 15

Debug Options:
[state] Show complete session state
[parse "input"] Test parsing specific input
[eval "expression"] Test evaluation
[reset] Reset REPL session
[trace] Enable execution tracing

Enter debug command or 'exit':
> parse "async fn test() { }"
✓ Parsed successfully: AsyncFunctionDecl { name: "test", ... }

> eval "undefined_variable"
❌ Error: Undefined variable 'undefined_variable'
   Available in scope: [x, y, result, calculate, ...]

> state
Session Variables:
├── x: int = 42
├── y: string = "hello"
├── result: Option<int> = Some(15)
└── calculate: fn(int) -> int
```

### 5. Performance Debugging
**Purpose**: Identify and resolve performance bottlenecks
**Command**: `/debug performance`

#### Performance Analysis Features:
- **Hotspot Detection**: CPU-intensive code paths
- **Memory Profiling**: Allocation patterns and lifetimes
- **Compilation Speed**: Slow compile times and optimizations
- **Runtime Efficiency**: Execution speed analysis

#### Performance Debug Example:
```bash
/debug --performance-profile
```

**Output**:
```
⚡ Performance Profile Analysis
==============================
Target: compilation of large_project.script
Total Time: 8.7 seconds

Compilation Phases:
├── Lexing:     0.8s (9.2%)  ✓ Normal
├── Parsing:    2.1s (24.1%) ⚠ Slow
├── Semantic:   4.2s (48.3%) ❌ Very Slow
├── Codegen:    1.1s (12.6%) ✓ Normal
└── Linking:    0.5s (5.8%)  ✓ Fast

Hotspots Detected:
1. Type inference: 3.1s (35.6% of total)
   └── Location: src/semantic/type_inference.rs:234-567
   └── Issue: O(n³) constraint solving algorithm

2. AST traversal: 1.8s (20.7% of total)
   └── Location: src/semantic/visitor.rs:89-156
   └── Issue: Redundant tree walks

3. Symbol resolution: 0.9s (10.3% of total)
   └── Location: src/semantic/scope.rs:45-78
   └── Issue: Linear search in symbol table

Optimization Suggestions:
├── Implement incremental type inference
├── Cache AST traversal results
├── Use HashMap for symbol lookup
└── Enable parallel semantic analysis

Estimated Improvement: 65% faster compilation
```

## Interactive Debugging Features

### Step-by-Step Execution Tracing
```bash
/debug --trace-execution
```

**Interactive Trace Session**:
```
🔍 Execution Trace Debug
========================
Program: fibonacci(10)

Step 1: Function call fibonacci(10)
├── Entering function at src/examples/fib.script:5
├── Parameters: n = 10
└── Local variables: none

Step 2: Conditional check n <= 1
├── Evaluating: 10 <= 1
├── Result: false
└── Taking else branch

Step 3: Recursive call fibonacci(9)
├── Stack depth: 2
├── Memory usage: +24 bytes
└── Entering nested call...

[Continue] [Step Into] [Step Over] [Show Stack] [Show Memory]
> Show Stack

Call Stack:
├── fibonacci(10) @ src/examples/fib.script:5
└── fibonacci(9) @ src/examples/fib.script:7 (tail call)

> Show Memory
Memory State:
├── Stack: 156 bytes (2 frames)
├── Heap: 0 bytes (no allocations)
└── Registers: n=9, return_addr=0x7fff...
```

### Constraint Solver Debugging
```bash
/debug --constraint-solver
```

**Constraint Debug Session**:
```
🔗 Constraint Solver Debug
===========================
Current Problem: Generic function resolution
Function: map<T, U>(list: List<T>, f: fn(T) -> U) -> List<U>
Call Site: map(numbers, |x| x.to_string())

Active Constraints:
├── T = int (from numbers: List<int>)
├── U = string (from to_string() return type)
├── fn(T) -> U ≡ fn(int) -> string ✓
└── List<U> = List<string> ✓

Solver Steps:
1. Unify T with int from context ✓
2. Infer U from closure return type ✓
3. Check function type compatibility ✓
4. Construct result type List<string> ✓

Result: Successfully resolved all constraints
Final Type: List<string>

Debug Commands:
[step] Step through solver
[constraints] Show all constraints
[unify T U] Test unification
[backtrack] Show solver backtracking
```

## Error Pattern Recognition

### Common Error Patterns
The debug command recognizes and provides specialized help for common error patterns:

#### 1. Ownership and Borrowing Issues
```rust
// Pattern: Use after move
let x = vec![1, 2, 3];
let y = x;  // Move occurs here
println!("{:?}", x);  // Error: use after move

// Debug analysis:
🔍 Ownership Error Detected
===========================
Issue: Use of moved value 'x'
Pattern: Classic move semantic violation

Explanation:
├── Line 1: x owns Vec<int>
├── Line 2: Ownership transferred to y
└── Line 3: Attempt to use moved value x

Solutions:
1. Clone before move: let y = x.clone();
2. Use reference: let y = &x;
3. Use x after y: swap lines 2 and 3
4. Use y instead: println!("{:?}", y);
```

#### 2. Type Mismatch Patterns
```rust
// Pattern: Generic constraint failure
fn process<T: Display>(value: T) {
    println!("{}", value);
}

process(SomeStruct {}); // Error: SomeStruct doesn't implement Display

// Debug analysis:
🔍 Trait Bound Error
===================
Issue: Type doesn't satisfy constraint
Missing: Display implementation for SomeStruct

Quick Fixes:
1. Add #[derive(Display)] to SomeStruct
2. Implement Display manually
3. Use Debug instead: <T: Debug> and {:?}
4. Convert to string: value.to_string()
```

## Integration with Knowledge Base

### Debug Session Logging
All debugging sessions are automatically logged to the knowledge base:

```markdown
# Debug Session Report
**Date**: 2025-07-15T14:32:00Z
**Issue**: Type inference failure in generic function
**Resolution**: Added explicit type bounds
**Files Modified**: 
- src/semantic/type_inference.rs
- tests/type_system/generic_bounds.rs

## Problem Analysis
The type inference system failed to resolve constraints for a complex generic function with multiple trait bounds.

## Solution Applied
Added explicit where clauses to disambiguate type relationships.

## Prevention Strategy
- Add more comprehensive constraint solver tests
- Improve error messages for constraint failures
- Document common generic programming patterns
```

### Issue Tracking Integration
- Failed debug sessions create issues in `kb/active/DEBUG_<issue>.md`
- Resolved issues move to `kb/completed/`
- Recurring patterns update knowledge base patterns
- Performance improvements tracked in benchmarks

## Advanced Debugging Tools

### AST Visualization
```bash
/debug --ast-dump <file>
```

**Visual AST Output**:
```
🌳 Abstract Syntax Tree
========================
Program
├── FunctionDecl: fibonacci
│   ├── Parameters: [n: int]
│   ├── ReturnType: int
│   └── Body: Block
│       └── IfElse
│           ├── Condition: BinaryOp(<=)
│           │   ├── Left: Identifier(n)
│           │   └── Right: Literal(1)
│           ├── ThenBranch: Return(Literal(1))
│           └── ElseBranch: Return(BinaryOp(+))
│               ├── Left: Call(fibonacci)
│               │   └── Arg: BinaryOp(-)
│               │       ├── Left: Identifier(n)
│               │       └── Right: Literal(1)
│               └── Right: Call(fibonacci)
│                   └── Arg: BinaryOp(-)
│                       ├── Left: Identifier(n)
│                       └── Right: Literal(2)
```

### IR Analysis
```bash
/debug --ir-dump <function>
```

**IR Debug Output**:
```
🔧 Intermediate Representation
==============================
Function: fibonacci(n: int) -> int

Basic Blocks:
BB0: Entry
├── %0 = param n: int
├── %1 = const 1: int
├── %2 = icmp_le %0, %1
└── br %2, BB1, BB2

BB1: Base Case
├── %3 = const 1: int
└── ret %3

BB2: Recursive Case
├── %4 = const 1: int
├── %5 = isub %0, %4
├── %6 = call fibonacci(%5)
├── %7 = const 2: int
├── %8 = isub %0, %7
├── %9 = call fibonacci(%8)
├── %10 = iadd %6, %9
└── ret %10

Optimization Opportunities:
⚠ Tail recursion not optimized
⚠ Memoization possible for fibonacci
✓ No memory leaks detected
✓ No undefined behavior
```

## Command Integration

### Synergy with Other Commands
- `/debug` + `/test`: Debug failing tests with detailed analysis
- `/debug` + `/audit`: Debug security issues found in audits
- `/debug` + `/implement`: Debug issues during feature implementation
- `/debug` + `/status`: Debug build and compilation issues

### Workflow Examples
```bash
# Complete debugging workflow:
/test semantic                      # Run semantic tests
# Test failure detected
/debug semantic                     # Debug the semantic issues
# Issue identified and fixed
/test semantic                      # Re-run tests to verify fix
/audit src/semantic/fixed_code.rs  # Security audit the fix
```

## Best Practices

### Effective Debugging
- Start with high-level error categories, drill down to specifics
- Use interactive mode for complex issues
- Save debug sessions to knowledge base for future reference
- Combine multiple debugging approaches for difficult problems

### Performance Debugging
- Profile before optimizing
- Focus on algorithmic improvements over micro-optimizations
- Test performance fixes with realistic workloads
- Monitor for performance regressions

### Security Debugging
- Always consider security implications of debug information
- Don't log sensitive data in debug sessions
- Verify fixes don't introduce new vulnerabilities
- Use security-focused test cases for validation

## Limitations and Future Enhancements

### Current Limitations
- Limited support for distributed debugging
- Basic integration with external debuggers
- Manual interpretation of some complex error patterns
- Platform-specific debugging features vary

### Planned Enhancements
- Real-time collaborative debugging
- AI-powered error pattern recognition
- Integration with LLDB/GDB for runtime debugging
- Visual debugging interface for complex data structures
- Automated fix suggestion and application

This `/debug` command provides comprehensive debugging support that enables developers to quickly identify, understand, and resolve issues throughout the Script language development process.