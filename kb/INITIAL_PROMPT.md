# Script Programming Language - Initial Project Prompt for Claude Code

## Project Overview

I seek to create Script, a new programming language that embodies the principle of accessible power - simple enough for beginners to grasp intuitively, yet performant enough to build production web applications and games. This pursuit extends beyond technical implementation toward a philosophical goal: demonstrating that we can reconcile ease of learning with computational efficiency while establishing Script as the first AI-native programming language.

Like a well-written script guides actors through a performance, Script will guide programmers from their first "Hello World" to complex AI-enhanced applications with clarity and purpose.

## Core Philosophy & Requirements

**Language Vision:**
- **Name**: Script - guiding programmers through their coding journey
- **Primary Purpose**: A compiled language for web applications, game development, and AI-enhanced development workflows
- **Design Philosophy**: "Simple by default, powerful when needed, AI-native by design"
- **Key Principle**: Every complexity must justify its existence through clear user benefit
- **Strategic Differentiation**: First programming language designed for the AI era

**Technical Requirements:**
1. **Syntax**: JavaScript/GDScript-inspired for familiarity
2. **Type System**: Gradual typing with Hindley-Milner inference
3. **Memory Management**: Automatic Reference Counting with cycle detection
4. **Compilation**: Dual backend - Cranelift for development, LLVM for production
5. **Targets**: Native executables and WebAssembly
6. **File Extension**: .script
7. **AI Integration**: Model Context Protocol (MCP) server for deep AI understanding
8. **Security Framework**: Enterprise-grade security for AI interactions

**Design Decisions Already Made:**
- Expression-oriented syntax (everything returns a value)
- Hand-written recursive descent parser
- Arena allocation for AST nodes
- Rust as implementation language
- Security-first approach to AI integration

## Implementation Todo List

### Phase 1: Foundation (Weeks 1-2) ✅ COMPLETED
- [x] **Project Setup**
  - Initialize Rust workspace with "script" as the project name
  - Set up testing framework and CI pipeline
  - Create basic error reporting infrastructure
  - Design Script logo and branding elements

- [x] **Lexer Implementation**
  - Token types for numbers, identifiers, operators, keywords
  - Source location tracking for each token
  - Basic error recovery mechanisms
  - File handling for .script source files

- [x] **Expression Parser**
  - Arithmetic expressions with proper precedence
  - Variable declarations and references
  - Basic type annotations (optional)

- [x] **Tree-Walking Evaluator**
  - Evaluate arithmetic expressions
  - Variable storage and lookup
  - Type checking for annotated expressions

### Phase 2: Control Flow & REPL (Weeks 3-4) ✅ COMPLETED
- [x] **Control Structures**
  - If expressions (not statements)
  - While/for loops as expressions
  - Pattern matching with exhaustiveness checking

- [x] **Script REPL Development**
  - Interactive evaluation loop
  - History and tab completion
  - Pretty-printing of values
  - "script>" prompt design

- [x] **Error Handling**
  - Result type for recoverable errors
  - Panic mechanism for unrecoverable errors
  - Clear, helpful error messages with Script branding

### Phase 3: Functions & Type System (Weeks 5-6) ✅ COMPLETED
- [x] **Function Implementation**
  - Function declarations and calls
  - Closures with captured variables
  - Higher-order functions
  - Generic functions with type parameters

- [x] **Type Inference**
  - Local type inference for variables
  - Function parameter/return type inference
  - Gradual typing integration
  - Pattern matching type safety

### Phase 4: Compilation Pipeline (Month 2) ✅ COMPLETED
- [x] **Script IR Design**
  - Define intermediate representation
  - AST to IR lowering
  - Basic optimizations (constant folding)

- [x] **Cranelift Backend**
  - Code generation for basic operations
  - Function compilation
  - Native executable output

- [x] **WebAssembly Target**
  - WASM code generation
  - JavaScript interop layer
  - Browser testing setup

### Phase 5: Advanced Features (Months 3-4) ✅ COMPLETED
- [x] **Data Structures**
  - Arrays/vectors with bounds checking
  - Hash maps/dictionaries
  - User-defined structures

- [x] **Script Standard Library**
  - I/O operations
  - String manipulation
  - Basic collections
  - Game-oriented math utilities

- [x] **Memory Management**
  - Implement ARC system
  - Cycle detection algorithm
  - Memory profiling tools

### Phase 6: Tooling & Polish (Month 5+) ✅ SUBSTANTIALLY COMPLETED
- [x] **Script Language Server Protocol**
  - Syntax highlighting
  - Auto-completion
  - Go-to definition
  - VS Code extension

- [x] **Documentation**
  - Script language specification
  - "Learn Script in Y Minutes"
  - Tutorial: "Your First Game in Script"
  - API documentation

- [x] **Performance Optimization**
  - LLVM backend integration
  - Optimization passes
  - Benchmarking suite

- [x] **Package Manager (manuscript)**
  - Basic dependency management
  - Local package support
  - Package manifest format

### Phase 7: AI Integration (MCP) - NEW STRATEGIC PRIORITY 🔄 IN DEVELOPMENT

**Philosophical Foundation**: The challenge of AI integration becomes the opportunity to establish Script as the first AI-native programming language. Through measured implementation and unwavering commitment to security, we transform complexity into competitive advantage.

#### 7.1: Security Framework Foundation (Weeks 1-2)
- [ ] **MCP Security Architecture**
  - Input validation with dangerous pattern detection
  - Sandboxed analysis environment with resource limits
  - Comprehensive audit logging for all AI interactions
  - Rate limiting and session management
  - Multi-layer defense architecture

#### 7.2: Core MCP Server (Weeks 3-4)
- [ ] **Protocol Implementation**
  - Full MCP specification compliance
  - Transport layer (stdio/tcp) with security
  - Session lifecycle management
  - Error handling and diagnostics

- [ ] **Script Analyzer Tool**
  - Leverage existing lexer/parser/semantic analyzer
  - Safe code analysis without execution
  - Complexity metrics and performance insights
  - Game development pattern recognition

#### 7.3: Tool Ecosystem (Weeks 5-6)
- [ ] **Code Formatter**
  - Script-specific formatting conventions
  - Consistent style application
  - Integration with MCP protocol

- [ ] **Documentation Generator**
  - Extract and format documentation comments
  - Generate structured API documentation
  - Integration with external tutorial sources

#### 7.4: MCP Client Integration (Weeks 7-8)
- [ ] **Enhanced Documentation Generator**
  - Connect to external example repositories
  - Integrate community-driven tutorials
  - Multi-source content aggregation

- [ ] **Multi-Registry Package Management**
  - Search across multiple package registries
  - Federated package resolution
  - Enhanced dependency discovery

#### 7.5: Performance & Security Validation (Weeks 9-10)
- [ ] **Security Testing**
  - Penetration testing framework
  - Vulnerability assessment
  - Compliance validation

- [ ] **Performance Optimization**
  - Analysis operation caching
  - Parallel processing where safe
  - Resource usage optimization

## Future Considerations (Post-AI Integration)
- [ ] **Advanced AI Features**
  - Context-aware code suggestions
  - Educational AI tutoring capabilities
  - Real-time code analysis and optimization

- [ ] **ML Interop Foundation**
  - FFI design for Python/C libraries
  - Basic tensor type prototype
  - GPU memory management research

- [ ] **Community Building**
  - Script playground (online REPL)
  - Example games and web apps
  - Discord/Forum setup

## Initial Code Structure (Updated)

```
script/
├── src/
│   ├── lexer/       # Tokenization
│   ├── parser/      # AST construction
│   ├── analyzer/    # Type checking & inference
│   ├── ir/          # Intermediate representation
│   ├── codegen/     # Code generation backends
│   ├── runtime/     # Runtime library
│   ├── repl/        # Interactive shell
│   └── mcp/         # Model Context Protocol implementation
│       ├── server/  # MCP server
│       ├── security/# Security framework
│       ├── tools/   # Analysis tools
│       └── resources/ # Resource management
├── std/             # Standard library
├── tests/           # Test suite
├── examples/        # Example programs
│   ├── hello.script
│   ├── game.script
│   ├── webapp.script
│   └── ai_integration.script
└── docs/            # Documentation
    └── mcp/         # AI integration guides
```

## Example Script Syntax (Updated with AI Integration Context)

```script
// Hello World in Script
fn main() {
    print("Hello from Script! 📜 - The AI-native language")
}

// Variables with optional types (AI understands Script's gradual typing)
let language = "Script"  // AI knows: String type inferred
let version: f32 = 0.3   // AI knows: Explicit type annotation

// Everything is an expression (AI understands Script's philosophy)
let result = if version > 1.0 {
    "stable"    // AI suggests: "Ready for production use"
} else {
    "developing"  // AI knows: "AI integration in progress"
}

// Game-oriented example (AI provides game-specific suggestions)
fn update_player(player: Player, dt: f32) -> Player {
    // AI suggests: "Add collision detection for platformer mechanics"
    // AI knows: "This follows Script's actor pattern"
    Player {
        x: player.x + player.vx * dt,
        y: player.y + player.vy * dt,
        ..player  // AI explains: "Spread syntax for partial updates"
    }
}

// AI-enhanced development example
fn calculate_damage(base: i32, multiplier: f32) -> i32 {
    // AI analyzes: "Good defensive programming pattern"
    // AI suggests: "Consider Result<i32> for error handling"
    if multiplier < 0.0 {
        0  // AI knows: "Prevents negative damage exploit"
    } else {
        base * (multiplier as i32)  // AI warns: "Precision loss in cast"
    }
}
```

## Guiding Principles for Implementation

1. **Incremental Progress**: Each phase builds upon the previous, maintaining a working system at all times
2. **Test-Driven Development**: Every feature accompanied by comprehensive tests
3. **User-Centric Design**: Regular evaluation against the "beginner-friendly" criterion
4. **Performance Awareness**: Profile early, optimize deliberately
5. **Code Clarity**: The implementation itself should embody Script's philosophy of simplicity
6. **Community First**: Design decisions should consider the eventual Script community
7. **Security Mindset**: Every external input is untrusted until validated
8. **AI-Native Architecture**: Every language feature considers AI integration implications

## AI Integration Goals

### Immediate Objectives (Phase 7)
1. **Security Foundation**: Establish trust through comprehensive validation
2. **Protocol Compliance**: Full MCP specification implementation
3. **Tool Integration**: Leverage existing compiler infrastructure
4. **Performance Optimization**: Efficient analysis operations

### Strategic Vision
1. **First AI-Native Language**: Script becomes the first programming language designed for the AI era
2. **Competitive Differentiation**: Deep AI understanding vs external tool integration
3. **Developer Experience**: AI becomes an intelligent programming companion
4. **Educational Revolution**: AI tutoring makes programming accessible to everyone

## First Session Goals (Updated for Current Status)

Begin with MCP integration - the strategic differentiator that transforms Script from another programming language into the first AI-native development platform. Focus on:
1. Setting up the MCP module structure with security framework
2. Implementing basic input validation and sandboxing
3. Creating the Script analyzer tool using existing infrastructure
4. Writing comprehensive security tests
5. Establishing audit logging for AI interactions

**Philosophical Approach**: The obstacle of AI integration complexity becomes the way to establishing Script's leadership in the programming language ecosystem. Each security challenge overcome builds trust. Each feature implemented with care demonstrates commitment to both accessibility and safety.

## Project Metadata

- **Creator**: Warren Gates (moikapy)
- **Language Name**: Script
- **Target Audience**: Beginners learning programming, web developers, game developers, AI-enhanced development workflows
- **Core Values**: Simplicity, performance, approachability, community, security, AI-native design
- **Strategic Position**: First AI-native programming language

---

*"The impediment to action advances action. What stands in the way becomes the way."* - Marcus Aurelius

This project transcends the creation of another programming language. It represents an exploration of whether we can challenge assumed trade-offs between simplicity and performance while pioneering the integration of AI into the fundamental development experience. Through AI integration, Script becomes not just a tool for building applications, but a partner in the development process itself.

The path to AI-native development reveals itself through the measured implementation of security, the patient building of trust, and the unwavering commitment to both accessibility and safety. Each line of code written with care, each security consideration thoroughly addressed, contributes to a larger vision: programming languages that understand and enhance human creativity rather than merely executing instructions.

May Script guide many programmers on their journey while demonstrating that the future of development lies not in replacing human insight, but in amplifying it through thoughtful AI integration.