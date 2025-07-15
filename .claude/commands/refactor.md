# /refactor Command Documentation

## Overview

The `/refactor` command provides safe, automated code refactoring capabilities for the Script programming language project. It leverages semantic analysis and type information to perform transformations that preserve program behavior while improving code quality, maintainability, and performance.

## Purpose

This command enhances development productivity and code quality by:
- Performing safe automated refactoring with semantic validation
- Enabling large-scale codebase transformations with confidence
- Improving code structure and eliminating technical debt
- Standardizing code patterns across the project
- Reducing manual refactoring effort and human error
- Maintaining type safety and semantic correctness throughout transformations

## Usage

### Basic Syntax
```bash
/refactor <transformation>      # Apply specific refactoring
/refactor --analyze            # Analyze refactoring opportunities
/refactor --preview <change>   # Preview transformation without applying
/refactor --batch <pattern>    # Apply transformation to multiple files
```

### Common Refactoring Operations
```bash
/refactor rename <old> <new>           # Rename symbols safely
/refactor extract-function <selection> # Extract code into function
/refactor inline-function <name>       # Inline function calls
/refactor move-module <src> <dest>     # Move module to different location
/refactor split-module <module>        # Split large module into smaller ones
/refactor merge-modules <modules>      # Merge related modules
/refactor remove-dead-code            # Eliminate unused code
/refactor standardize-patterns        # Apply consistent code patterns
```

### Advanced Refactoring Options
```bash
/refactor --semantic-preserving      # Ensure semantic equivalence
/refactor --type-safe               # Maintain type safety guarantees
/refactor --performance-aware       # Consider performance implications
/refactor --security-conscious      # Preserve security properties
/refactor --test-coverage           # Maintain test coverage
/refactor --documentation          # Update related documentation
```

### Scope and Targeting
```bash
/refactor --file <path>             # Refactor specific file
/refactor --module <name>           # Refactor entire module
/refactor --component <type>        # Refactor by component (lexer, parser, etc.)
/refactor --pattern <regex>         # Refactor based on pattern matching
/refactor --project-wide           # Apply transformation across entire project
```

## Refactoring Categories

### 1. Symbol and Naming Refactoring
**Purpose**: Rename symbols safely across the codebase
**Command**: `/refactor rename`

#### Safe Symbol Renaming
```bash
/refactor rename TokenType LexicalToken
```

**Process**:
1. **Semantic Analysis**: Analyze all references to the symbol
2. **Scope Resolution**: Identify all scopes where symbol is visible
3. **Conflict Detection**: Check for naming conflicts in target scopes
4. **Reference Mapping**: Map all references across files and modules
5. **Atomic Application**: Apply all changes atomically

**Example Output**:
```
🔄 Symbol Rename Analysis
=========================
Symbol: TokenType → LexicalToken
Scope: Global (exported from lexer module)

References Found (47):
├── Definitions: 1 (src/lexer/token.rs:15)
├── Type annotations: 23 across 8 files
├── Pattern matches: 12 in parser module
├── Documentation: 8 references in comments
└── Test cases: 3 in test files

Conflict Analysis:
✅ No naming conflicts detected
✅ No shadowing issues
✅ Export compatibility maintained

Files to Modify (8):
├── src/lexer/token.rs (definition + 3 uses)
├── src/lexer/mod.rs (2 exports)
├── src/parser/expression.rs (8 uses)
├── src/parser/statement.rs (12 uses)
├── src/semantic/analyzer.rs (6 uses)
├── tests/lexer_tests.rs (3 uses)
├── tests/parser_tests.rs (5 uses)
└── docs/lexer-design.md (8 documentation refs)

Preview Changes? [Y/n]: Y
Apply Refactoring? [Y/n]: 
```

#### Advanced Renaming Patterns
```bash
/refactor rename --pattern ".*Error$" ".*Exception"  # Rename all Error types to Exception
/refactor rename --scope module::parser "parse_*" "analyze_*"  # Rename functions in parser module
/refactor rename --camelcase-to-snake                # Convert camelCase to snake_case
```

### 2. Function and Method Refactoring
**Purpose**: Restructure functions for better organization and reusability
**Command**: `/refactor extract-function`, `/refactor inline-function`

#### Function Extraction
```bash
/refactor extract-function --selection "src/parser/expression.rs:145-167" --name "parse_binary_operator"
```

**Process**:
1. **Selection Analysis**: Parse selected code for extraction viability
2. **Dependency Analysis**: Identify parameters and return values needed
3. **Scope Analysis**: Determine appropriate function placement
4. **Name Conflict Resolution**: Ensure function name is available
5. **Automatic Parameter Detection**: Extract necessary parameters
6. **Return Type Inference**: Determine appropriate return type

**Example Extraction**:
```rust
// Before extraction (selected code):
fn parse_expression(&mut self) -> Result<Expression> {
    let left = self.parse_primary()?;
    
    // Selected code for extraction:
    while let Some(op) = self.current_token.as_binary_operator() {
        let precedence = op.precedence();
        if precedence < min_precedence {
            break;
        }
        self.advance();
        let right = self.parse_expression_with_precedence(precedence + 1)?;
        left = Expression::Binary { left: Box::new(left), op, right: Box::new(right) };
    }
    // End selection
    
    Ok(left)
}

// After extraction:
fn parse_expression(&mut self) -> Result<Expression> {
    let left = self.parse_primary()?;
    let result = self.parse_binary_operator(left, 0)?;
    Ok(result)
}

fn parse_binary_operator(&mut self, mut left: Expression, min_precedence: i32) -> Result<Expression> {
    while let Some(op) = self.current_token.as_binary_operator() {
        let precedence = op.precedence();
        if precedence < min_precedence {
            break;
        }
        self.advance();
        let right = self.parse_expression_with_precedence(precedence + 1)?;
        left = Expression::Binary { left: Box::new(left), op, right: Box::new(right) };
    }
    Ok(left)
}
```

#### Function Inlining
```bash
/refactor inline-function simple_getter_function
```

**Safety Checks**:
- Verify function has single responsibility
- Ensure no side effects that shouldn't be duplicated
- Check performance implications of inlining
- Validate all call sites are compatible

### 3. Module and Structure Refactoring
**Purpose**: Reorganize code structure for better modularity
**Command**: `/refactor move-module`, `/refactor split-module`

#### Module Splitting
```bash
/refactor split-module src/semantic/analyzer.rs --strategy "by-functionality"
```

**Splitting Strategies**:
- **By Functionality**: Group related functions together
- **By Type**: Separate different analysis types
- **By Complexity**: Isolate complex algorithms
- **By Dependencies**: Minimize inter-module dependencies

**Example Split Output**:
```
📁 Module Split Analysis
========================
Source: src/semantic/analyzer.rs (2,847 lines)
Strategy: By Functionality

Proposed Split:
├── src/semantic/type_inference.rs (847 lines)
│   ├── infer_expression_type()
│   ├── unify_types()
│   ├── resolve_type_variables()
│   └── 23 related functions
├── src/semantic/scope_resolution.rs (654 lines)
│   ├── resolve_symbol()
│   ├── enter_scope()
│   ├── exit_scope()
│   └── 18 related functions
├── src/semantic/constraint_solving.rs (923 lines)
│   ├── solve_constraints()
│   ├── add_constraint()
│   ├── check_constraint_satisfaction()
│   └── 31 related functions
└── src/semantic/error_reporting.rs (423 lines)
    ├── report_type_error()
    ├── format_error_message()
    ├── collect_diagnostic_info()
    └── 15 related functions

Dependencies Analysis:
├── Internal dependencies: All resolvable
├── Circular dependencies: None detected
├── Public API impact: Minimal (re-exports added)
└── Test updates required: 12 test files

Estimated Benefits:
├── Compilation parallelization: +25% faster
├── Code readability: Significantly improved
├── Maintenance: Easier to navigate and modify
└── Testing: More focused test suites
```

### 4. Dead Code Elimination
**Purpose**: Remove unused code safely
**Command**: `/refactor remove-dead-code`

#### Comprehensive Dead Code Analysis
```bash
/refactor remove-dead-code --aggressive
```

**Detection Categories**:
- **Unused Functions**: Functions never called
- **Unused Types**: Types never instantiated or referenced
- **Unused Variables**: Variables defined but never used
- **Unused Imports**: Import statements for unused symbols
- **Unreachable Code**: Code paths that can never execute
- **Obsolete Comments**: Comments referencing removed code

**Safety Considerations**:
```
🧹 Dead Code Analysis
=====================
Analysis Scope: Entire project
Analysis Mode: Aggressive (removes more aggressively)

Unused Code Detected:
├── Functions: 23 candidates
│   ├── Safe to remove: 18 ✅
│   ├── Exported but unused: 3 ⚠ (may be API)
│   └── Test utilities: 2 ⚠ (keep for future tests)
├── Types: 7 candidates
│   ├── Internal types: 5 ✅ (safe to remove)
│   └── Public types: 2 ⚠ (may break API)
├── Variables: 45 candidates
│   ├── Local variables: 42 ✅ (safe to remove)
│   └── Static variables: 3 ⚠ (may have side effects)
├── Imports: 15 unused imports ✅
└── Unreachable code: 8 blocks ✅

Conservative Removal (Safe):
├── 18 unused functions
├── 5 unused internal types  
├── 42 unused local variables
├── 15 unused imports
└── 8 unreachable code blocks

Estimated Impact:
├── Code reduction: -1,247 lines (-8.3%)
├── Compilation speed: +12% faster
├── Binary size: -156KB smaller
└── Maintenance burden: Significantly reduced

Review Required:
├── API compatibility for public items
├── Test coverage for removed functionality
├── Documentation updates needed
└── Deprecation warnings for removed public API
```

### 5. Pattern Standardization
**Purpose**: Apply consistent code patterns across the project
**Command**: `/refactor standardize-patterns`

#### Code Pattern Analysis
```bash
/refactor standardize-patterns --pattern "error-handling"
```

**Common Pattern Standardizations**:
- **Error Handling**: Consistent use of Result types and error propagation
- **Resource Management**: Standardized RAII patterns and cleanup
- **Async Patterns**: Consistent async/await usage and error handling
- **Testing Patterns**: Uniform test structure and assertion styles
- **Documentation Patterns**: Consistent doc comment format and content

**Example Pattern Standardization**:
```rust
// Before: Inconsistent error handling
fn parse_number(input: &str) -> Option<i32> {
    match input.parse() {
        Ok(n) => Some(n),
        Err(_) => None,
    }
}

fn parse_identifier(input: &str) -> Result<String, String> {
    if input.is_empty() {
        return Err("Empty identifier".to_string());
    }
    Ok(input.to_string())
}

// After: Standardized error handling
fn parse_number(input: &str) -> Result<i32, ParseError> {
    input.parse().map_err(|e| ParseError::InvalidNumber {
        input: input.to_string(),
        source: e,
    })
}

fn parse_identifier(input: &str) -> Result<String, ParseError> {
    if input.is_empty() {
        return Err(ParseError::EmptyIdentifier);
    }
    Ok(input.to_string())
}
```

## Advanced Refactoring Features

### 1. Semantic-Preserving Transformations
**Command**: `/refactor --semantic-preserving`

#### Verification Process
1. **AST Comparison**: Compare abstract syntax trees before/after
2. **Type Checking**: Ensure all type constraints are preserved
3. **Control Flow Analysis**: Verify execution paths remain equivalent
4. **Side Effect Analysis**: Confirm side effects are unchanged
5. **Test Validation**: Run comprehensive test suite to verify behavior

#### Example Semantic Preservation
```rust
// Original code:
fn calculate_sum(numbers: &[i32]) -> i32 {
    let mut total = 0;
    for i in 0..numbers.len() {
        total += numbers[i];
    }
    total
}

// Refactored (semantically equivalent):
fn calculate_sum(numbers: &[i32]) -> i32 {
    numbers.iter().sum()
}

// Verification:
✅ Return type preserved: i32
✅ Input type preserved: &[i32]
✅ Behavior preserved: Sum calculation identical
✅ Side effects preserved: None (pure function)
✅ Performance: Improved (vectorized operations)
✅ Tests pass: All 15 test cases green
```

### 2. Type-Safe Refactoring
**Command**: `/refactor --type-safe`

#### Type Safety Guarantees
- **Type Preservation**: All type annotations remain valid
- **Generic Constraints**: Generic bounds are maintained
- **Lifetime Preservation**: Lifetime annotations stay correct
- **Trait Bounds**: Trait constraints are preserved
- **Memory Safety**: No introduction of memory safety issues

### 3. Performance-Aware Refactoring
**Command**: `/refactor --performance-aware`

#### Performance Considerations
```bash
/refactor extract-function --performance-aware
```

**Performance Analysis**:
- **Inlining Impact**: Consider function call overhead
- **Memory Layout**: Preserve cache-friendly data structures
- **Allocation Patterns**: Avoid introducing unnecessary allocations
- **Algorithmic Complexity**: Maintain or improve Big-O characteristics
- **Optimization Barriers**: Don't prevent compiler optimizations

**Example Performance-Aware Refactoring**:
```rust
// Before: Frequent small function calls in hot path
fn process_tokens(tokens: &[Token]) -> Vec<ProcessedToken> {
    tokens.iter().map(|token| {
        let normalized = normalize_token(token);  // Small function call
        let validated = validate_token(&normalized);  // Another call
        ProcessedToken::new(validated)
    }).collect()
}

// After: Inlined for performance (hot path optimization)
fn process_tokens(tokens: &[Token]) -> Vec<ProcessedToken> {
    tokens.iter().map(|token| {
        // Inlined normalize_token logic
        let normalized = Token {
            kind: token.kind.to_lowercase(),
            value: token.value.trim(),
            position: token.position,
        };
        
        // Inlined validate_token logic
        let validated = if normalized.value.is_empty() {
            return ProcessedToken::invalid();
        } else {
            normalized
        };
        
        ProcessedToken::new(validated)
    }).collect()
}

// Performance Impact:
// ├── Function call overhead: -95% (eliminated 2 calls per iteration)
// ├── Cache locality: +15% (better data layout)
// ├── Compiler optimization: Enhanced (better inlining opportunities)
// └── Benchmark improvement: +23% faster for large token arrays
```

### 4. Security-Conscious Refactoring
**Command**: `/refactor --security-conscious`

#### Security Preservation
- **Input Validation**: Maintain input sanitization and validation
- **Resource Limits**: Preserve DoS protection mechanisms
- **Error Information**: Don't expose sensitive data in error messages
- **Access Control**: Maintain proper encapsulation and visibility
- **Cryptographic Properties**: Preserve security-critical invariants

## Batch and Large-Scale Refactoring

### 1. Project-Wide Transformations
```bash
/refactor --project-wide rename "Error" "Exception"
```

**Batch Processing Features**:
- **Parallel Processing**: Refactor multiple files concurrently
- **Progress Tracking**: Real-time progress updates
- **Rollback Capability**: Atomic transactions with rollback
- **Conflict Resolution**: Handle merge conflicts intelligently
- **Impact Analysis**: Comprehensive impact assessment before changes

### 2. Pattern-Based Refactoring
```bash
/refactor --pattern "fn (\w+)_test\(\)" "fn test_$1()"
```

**Regex-Based Transformations**:
- **Safe Pattern Matching**: Semantic validation of pattern matches
- **Scope-Aware Replacement**: Respect language scoping rules
- **Multi-Line Patterns**: Support for complex multi-line transformations
- **Conditional Application**: Apply patterns based on context

## Refactoring Analysis and Planning

### 1. Refactoring Opportunity Analysis
```bash
/refactor --analyze
```

**Analysis Output**:
```
🔍 Refactoring Opportunity Analysis
===================================
Analysis Date: 2025-07-15 14:45:00 UTC
Scope: Entire project (23,847 lines across 156 files)

Code Quality Metrics:
├── Complexity Score: 7.3/10 (target: <6.0)
├── Duplication: 12.4% (target: <8%)
├── Naming Consistency: 89% (target: >95%)
├── Function Length: Avg 23 lines (target: <20)
└── Module Cohesion: 8.1/10 (good)

High-Impact Opportunities:
1. 🎯 Extract Common Pattern (Impact: High)
   ├── Pattern: Error handling in parser functions
   ├── Occurrences: 47 similar code blocks
   ├── Potential reduction: 340 lines
   └── Effort: 2-3 hours

2. 🎯 Split Large Module (Impact: High)
   ├── Module: src/semantic/analyzer.rs (2,847 lines)
   ├── Complexity: Very high
   ├── Compilation impact: +25% faster builds
   └── Effort: 1-2 days

3. 🎯 Standardize Naming (Impact: Medium)
   ├── Inconsistent patterns: 23 violations
   ├── Affected files: 15
   ├── Readability improvement: Significant
   └── Effort: 4-6 hours

4. 🎯 Remove Dead Code (Impact: Medium)
   ├── Unused functions: 18
   ├── Unused types: 5
   ├── Code reduction: 8.3%
   └── Effort: 2-3 hours

5. 🎯 Inline Small Functions (Impact: Low)
   ├── Functions under 5 lines: 34
   ├── Hot path functions: 12
   ├── Performance gain: 5-8%
   └── Effort: 1-2 hours

Recommended Sequence:
1. Remove dead code (low risk, quick wins)
2. Standardize naming (improves readability for other refactoring)
3. Extract common patterns (reduces duplication)
4. Split large modules (improves build times)
5. Performance optimizations (final polish)

Estimated Total Effort: 1-2 weeks
Estimated Benefits:
├── Code maintainability: +40%
├── Build performance: +25%
├── Runtime performance: +8%
└── Developer productivity: +30%
```

### 2. Refactoring Preview
```bash
/refactor --preview extract-function "src/parser/expression.rs:145-167"
```

**Preview Output**:
```
🔍 Refactoring Preview
======================
Transformation: Extract Function
Source: src/parser/expression.rs:145-167
Target Function: parse_binary_operator

Changes Preview:
┌─ src/parser/expression.rs ─┐
│ @@ -142,25 +142,8 @@        │
│  fn parse_expression(&mut   │
│      let left = self.parse  │
│ -                           │
│ -    while let Some(op) =   │
│ -        let precedence =   │
│ -        if precedence <    │
│ -            break;         │
│ -        }                  │
│ -        self.advance();    │
│ -        let right = self   │
│ -        left = Expression  │
│ -    }                      │
│ +    let result = self.par  │
│                             │
│      Ok(left)               │
│  }                          │
│                             │
│ +fn parse_binary_operator   │
│ +    while let Some(op) =   │
│ +        let precedence =   │
│ +        if precedence <    │
│ +            break;         │
│ +        }                  │
│ +        self.advance();    │
│ +        let right = self   │
│ +        left = Expression  │
│ +    }                      │
│ +    Ok(left)               │
│ +}                          │
└─────────────────────────────┘

Impact Analysis:
├── Lines changed: 25 modified, 18 added
├── Function complexity: Reduced by 40%
├── Reusability: New function can be reused in 3 other locations
├── Test coverage: Maintained (existing tests still pass)
└── Performance: Neutral (no significant change)

Safety Checks:
✅ No naming conflicts
✅ All variables properly scoped
✅ Return type correctly inferred
✅ Error handling preserved
✅ Documentation updated

Would you like to apply this refactoring? [Y/n]:
```

## Integration with Knowledge Base

### Refactoring Documentation
All refactoring activities are logged to the knowledge base:

```markdown
# Refactoring Session Report
**Date**: 2025-07-15T15:20:00Z
**Session Duration**: 2h 15m
**Transformations Applied**: 7

## Summary
Large-scale refactoring session focused on improving parser module organization and reducing code duplication.

## Transformations Applied
1. **Split Module**: src/semantic/analyzer.rs → 4 smaller modules
2. **Extract Function**: Common error handling pattern (12 locations)
3. **Rename Symbol**: TokenType → LexicalToken (project-wide)
4. **Remove Dead Code**: 18 unused functions eliminated
5. **Standardize Patterns**: Error handling consistency (47 fixes)
6. **Inline Functions**: 8 small getter functions
7. **Move Module**: src/utils/common.rs → src/shared/utilities.rs

## Impact Metrics
- Code reduction: -8.3% (1,247 lines removed)
- Compilation speed: +25% improvement
- Function complexity: -40% average reduction
- Code duplication: 12.4% → 6.8%
- Naming consistency: 89% → 97%

## Validation Results
- All 1,247 tests pass ✅
- No type errors introduced ✅
- Performance benchmarks stable ✅
- Security properties preserved ✅
- Documentation updated ✅

## Next Steps
- Monitor build performance over next week
- Collect developer feedback on new module structure
- Consider additional pattern extractions in codegen module
```

### Issue Resolution Tracking
Refactoring activities that resolve knowledge base issues:
- Update issue status in `kb/active/` when refactoring addresses problems
- Move resolved issues to `kb/completed/`
- Track refactoring-related improvements

## Best Practices

### Planning and Preparation
- Always analyze before refactoring large sections
- Use preview mode for complex transformations
- Ensure comprehensive test coverage before starting
- Create backup branches for large refactoring sessions

### Safety and Validation
- Run full test suite after each major transformation
- Validate semantic preservation for critical code paths
- Monitor performance benchmarks for regressions
- Use type-safe and semantic-preserving modes for production code

### Team Coordination
- Communicate large refactoring plans with team
- Coordinate with ongoing feature development
- Document architectural decisions and rationale
- Provide migration guides for API changes

This `/refactor` command provides powerful, safe code transformation capabilities that enable continuous improvement of the Script language codebase while maintaining correctness, performance, and security properties throughout the development process.