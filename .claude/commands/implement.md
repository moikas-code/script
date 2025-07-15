# /implement Command

## Overview

The `/implement` command is a comprehensive feature implementation tool for the Script programming language project. It systematically guides the development of new features from planning through testing, ensuring consistency with project architecture and best practices.

## Purpose

This command streamlines feature development by:
- Creating structured implementation plans with clear milestones
- Following Script language conventions and security practices
- Integrating with existing systems (lexer, parser, semantic analyzer, etc.)
- Ensuring comprehensive testing and documentation
- Maintaining code quality and performance standards

## Usage

### Basic Syntax
```
/implement <feature_description>
```

### Examples
```
/implement pattern matching for enums
/implement async/await syntax support
/implement module system with imports
/implement generic trait bounds
/implement memory-safe closures
```

## Implementation Process

### Phase 1: Analysis & Planning
1. **Feature Analysis**
   - Parse feature requirements from description
   - Identify affected components (lexer, parser, semantic, codegen, runtime)
   - Check for existing partial implementations
   - Assess complexity and dependencies

2. **Architecture Planning**
   - Design AST node structures
   - Plan semantic analysis requirements
   - Design IR representations
   - Plan runtime support needs

3. **Security & Safety Review**
   - Identify potential security implications
   - Plan memory safety requirements
   - Design resource limit enforcement
   - Plan error handling strategies

### Phase 2: Implementation Strategy
1. **Create Knowledge Base Entry**
   - Document implementation plan in `kb/active/IMPLEMENT_<FEATURE>.md`
   - Include milestones, affected files, and test requirements
   - Track progress and decision points

2. **Breaking Down Work**
   - Lexer changes (new tokens, keywords)
   - Parser changes (new grammar rules, AST nodes)
   - Semantic analysis (type checking, validation)
   - Code generation (IR lowering, optimization)
   - Runtime support (new primitives, memory management)
   - Testing (unit, integration, security)

### Phase 3: Systematic Implementation
1. **Lexer Implementation**
   - Add new tokens/keywords to `src/lexer/token.rs`
   - Update lexer scanning logic
   - Add lexer tests

2. **Parser Implementation**
   - Add AST node definitions to `src/parser/ast.rs`
   - Implement parsing rules in `src/parser/mod.rs`
   - Add parser tests with comprehensive coverage

3. **Semantic Analysis**
   - Add type checking in `src/semantic/analyzer.rs`
   - Update symbol table as needed
   - Implement validation and error reporting
   - Add semantic tests

4. **Code Generation**
   - Add IR instructions to `src/ir/instruction.rs`
   - Implement lowering in `src/lowering/mod.rs`
   - Add codegen optimization passes
   - Add codegen tests

5. **Runtime Support**
   - Implement runtime primitives in `src/runtime/`
   - Add memory management support
   - Implement security enforcement
   - Add runtime tests

### Phase 4: Integration & Testing
1. **Integration Testing**
   - End-to-end feature tests
   - Cross-component interaction tests
   - Performance benchmarks
   - Security validation tests

2. **Documentation**
   - Update language specification
   - Add examples and tutorials
   - Update API documentation
   - Create migration guides if needed

## Command Implementation

### Step 1: Feature Recognition
The command analyzes the feature description using pattern matching:

```typescript
// Pattern examples:
- "pattern matching" → Implement match expressions and exhaustiveness checking
- "async/await" → Implement async functions and await expressions
- "generic constraints" → Implement trait bounds and where clauses
- "module system" → Implement import/export and module resolution
```

### Step 2: Template Selection
Based on feature type, select appropriate implementation template:

- **Language Construct**: Full pipeline (lexer → parser → semantic → codegen → runtime)
- **Standard Library**: Primarily runtime implementation with some semantic support
- **Tooling Feature**: Focus on development tools and utilities
- **Security Feature**: Emphasis on validation and resource limits

### Step 3: File Generation
Create necessary files and boilerplate:

```bash
# Example for pattern matching implementation:
src/parser/pattern.rs          # Pattern AST nodes
src/semantic/pattern_check.rs  # Exhaustiveness checking
src/lowering/pattern_lower.rs  # Pattern IR lowering
tests/pattern_matching_tests.rs # Comprehensive tests
kb/active/IMPLEMENT_PATTERN_MATCHING.md # Progress tracking
```

### Step 4: Implementation Guidance
Provide step-by-step implementation guidance:

1. **Show current status** of related systems
2. **Generate boilerplate code** following project conventions
3. **Create test scaffolding** with security considerations
4. **Update build system** and dependencies as needed
5. **Provide implementation checklist** with validation criteria

## Security Considerations

### Memory Safety
- Implement bounds checking for new data structures
- Add resource limits for complex operations
- Validate user input at all levels
- Use safe abstractions over unsafe code

### DoS Protection
- Limit compilation complexity for new features
- Add timeout mechanisms for expensive operations
- Implement resource usage tracking
- Prevent exponential algorithm complexity

### Type Safety
- Ensure sound type system integration
- Validate generic instantiations
- Check for type confusion vulnerabilities
- Implement proper error propagation

## Quality Standards

### Code Quality
- Follow existing code conventions
- Implement comprehensive error handling
- Add detailed logging and diagnostics
- Use DRY principles and avoid duplication

### Testing Requirements
- Unit tests for each component
- Integration tests for feature interaction
- Security tests for vulnerability prevention
- Performance tests for resource usage
- Regression tests for existing functionality

### Documentation Standards
- Clear API documentation with examples
- Architecture decision records for complex features
- User-facing documentation updates
- Migration guides for breaking changes

## Integration with Existing Systems

### Lexer Integration
- Follow existing token naming conventions
- Integrate with error reporting system
- Maintain lexer performance characteristics
- Support Unicode and internationalization

### Parser Integration
- Use existing precedence and associativity rules
- Integrate with error recovery mechanisms
- Follow AST design patterns
- Support syntax highlighting and IDE features

### Semantic Integration
- Use existing type system infrastructure
- Integrate with symbol table management
- Follow error reporting conventions
- Support incremental compilation

### Runtime Integration
- Use existing memory management system
- Integrate with garbage collection
- Follow concurrency safety patterns
- Support debugging and profiling

## Example Implementation Workflow

```bash
# Start implementation
/implement pattern matching for enums

# Command will:
1. Analyze "pattern matching" feature requirements
2. Create kb/active/IMPLEMENT_PATTERN_MATCHING.md
3. Generate boilerplate files:
   - src/parser/pattern.rs
   - src/semantic/pattern_check.rs
   - tests/pattern_matching_tests.rs
4. Provide step-by-step implementation guide
5. Create test scaffolding with security checks
6. Update build configuration
7. Track progress through knowledge base

# Follow guided implementation:
Step 1: Implement Pattern AST nodes ✓
Step 2: Add pattern parsing logic ✓
Step 3: Implement exhaustiveness checking ✓
Step 4: Add pattern lowering to IR ✓
Step 5: Implement runtime pattern matching ✓
Step 6: Add comprehensive tests ✓
Step 7: Update documentation ✓

# Move completed implementation to kb/completed/
```

## Command Flags

### Development Flags
- `--prototype`: Create minimal prototype implementation
- `--security-first`: Prioritize security features and testing
- `--performance`: Focus on performance optimization
- `--breaking`: Allow breaking changes to existing APIs

### Integration Flags
- `--lexer-only`: Implement only lexer changes
- `--parser-only`: Implement only parser changes
- `--semantic-only`: Implement only semantic analysis
- `--runtime-only`: Implement only runtime features

### Testing Flags
- `--with-benchmarks`: Include performance benchmarks
- `--with-security-tests`: Include comprehensive security testing
- `--with-integration-tests`: Include end-to-end integration tests
- `--minimal-tests`: Create minimal test coverage

## Error Handling

### Implementation Errors
- Feature conflicts with existing implementations
- Insufficient information in feature description
- Missing dependencies or prerequisites
- Security risks identified during planning

### Recovery Strategies
- Suggest alternative implementation approaches
- Provide additional context gathering prompts
- Recommend prerequisite feature implementations
- Offer security mitigation strategies

## Best Practices

### Planning Phase
- Always create detailed implementation plan
- Identify and resolve dependencies early
- Consider backward compatibility impact
- Plan for comprehensive testing

### Implementation Phase
- Follow incremental development approach
- Test each component independently
- Integrate security checks throughout
- Maintain code quality standards

### Integration Phase
- Test feature interactions thoroughly
- Validate performance characteristics
- Ensure security properties hold
- Update all relevant documentation

## Maintenance and Evolution

### Version Compatibility
- Track feature compatibility across versions
- Provide migration paths for breaking changes
- Maintain feature flag support for gradual rollout
- Support feature deprecation lifecycle

### Continuous Improvement
- Gather feedback on implemented features
- Monitor performance and security metrics
- Refine implementation based on usage patterns
- Update best practices based on lessons learned

This `/implement` command provides a systematic, security-conscious approach to feature development that maintains the high quality standards of the Script programming language project.