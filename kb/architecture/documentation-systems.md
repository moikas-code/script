# Documentation Systems in Script

The Script project contains two distinct documentation systems that serve different purposes and audiences. Understanding the distinction between these systems is crucial for developers working on the project.

## Overview

### 1. Knowledge Base (kb/)
**Purpose**: Internal development documentation and project management
**Audience**: Developers working on the Script language compiler
**Location**: `/kb/`

The knowledge base is our primary system for tracking:
- Implementation status and progress
- Known issues and bugs
- Architecture decisions
- Development notes and TODOs
- Security audits and compliance documentation

**Key Features**:
- Integrated with MCP (Model Context Protocol) tools
- Organized into categories (active/, completed/, status/, etc.)
- Searchable via MCP commands
- Version controlled with the project
- Written in Markdown

**Usage**:
```bash
# Access via MCP tools
kb_read "active/KNOWN_ISSUES.md"
kb_search "async implementation"
kb_update "status/OVERALL_STATUS.md" "content"
```

### 2. Documentation Generator (src/doc/)
**Purpose**: Generate user-facing API documentation from Script source code
**Audience**: Users of the Script programming language
**Location**: `/src/doc/`

The documentation generator is a built-in tool that:
- Parses documentation comments from Script source files
- Extracts structured information (parameters, returns, examples)
- Generates static HTML documentation with search functionality
- Creates browsable API references for Script code

**Key Features**:
- Parses `///` and `/** */` style doc comments
- Supports structured tags (@param, @returns, @example, etc.)
- Generates responsive HTML with syntax highlighting
- Includes JavaScript-powered search functionality
- Hierarchical module organization

**Usage**:
```bash
# Generate documentation for Script code
script doc ./my_project ./docs
```

## When to Use Each System

### Use kb/ when:
- Tracking implementation tasks or bugs
- Documenting architecture decisions
- Recording development progress
- Managing security audits
- Communicating with other developers about the compiler

### Use src/doc/ when:
- Documenting Script language APIs
- Creating user guides for Script libraries
- Generating reference documentation for Script modules
- Building searchable documentation websites

## Example Comparison

### kb/ Example (Developer Documentation)
```markdown
# Module System Implementation Status

## Current Issues
- Import resolution fails for circular dependencies
- Generic type parameters not properly resolved in exports
- Memory leak in module cache under certain conditions

## Implementation Notes
The module resolver uses a two-pass algorithm...
```

### src/doc/ Example (User Documentation)
```script
/// Calculate the factorial of a number recursively
/// @param n - The number to calculate factorial for
/// @returns The factorial of n (n!)
/// @example
/// ```
/// let result = factorial(5)  // returns 120
/// ```
fn factorial(n) {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}
```

## Architecture Details

### kb/ Structure
```
kb/
├── README.md           # This file with usage instructions
├── active/            # Current work items and issues
├── completed/         # Resolved items for reference
├── status/            # Project status tracking
├── architecture/      # Design documents (like this one)
├── compliance/        # Security and compliance docs
└── archive/           # Historical documentation
```

### src/doc/ Components
```
src/doc/
├── mod.rs            # Core documentation types and parsing
├── generator.rs      # Documentation generation logic
├── html.rs           # HTML output generation
├── search.rs         # Search index building
└── tests.rs          # Documentation system tests
```

## Integration Points

While these systems serve different purposes, they can complement each other:

1. **Cross-referencing**: kb/ documentation can reference generated API docs
2. **Build Process**: Documentation generation can be tracked in kb/
3. **Examples**: Code examples in kb/ can be validated against actual API
4. **Migration**: Historical API changes documented in kb/ inform documentation updates

## Best Practices

1. **Keep Them Separate**: Don't mix internal development notes with user-facing documentation
2. **Use Appropriate Detail**: kb/ can contain implementation details; src/doc/ should focus on usage
3. **Maintain Both**: Updates to the language should be reflected in both systems
4. **Version Awareness**: kb/ tracks what's in development; src/doc/ documents what's released

## Future Considerations

- Consider automating some kb/ updates based on code changes
- Potentially integrate documentation coverage metrics
- Explore generating some kb/ content from code analysis
- Consider publishing selected kb/ content as developer guides