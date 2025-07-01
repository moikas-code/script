# Compiler Pipeline

## Table of Contents

- [Pipeline Overview](#pipeline-overview)
- [Phase 1: Lexical Analysis](#phase-1-lexical-analysis)
- [Phase 2: Syntax Analysis](#phase-2-syntax-analysis)
- [Phase 3: Semantic Analysis](#phase-3-semantic-analysis)
- [Phase 4: Type Inference](#phase-4-type-inference)
- [Phase 5: IR Generation](#phase-5-ir-generation)
- [Phase 6: Code Generation](#phase-6-code-generation)
- [Pipeline Integration](#pipeline-integration)
- [Error Handling Strategy](#error-handling-strategy)
- [Optimization Opportunities](#optimization-opportunities)

## Pipeline Overview

The Script compiler follows a traditional multi-phase architecture, transforming source code through several intermediate representations before generating executable code:

```
Source Code
    │
    ▼
┌─────────────────┐
│  Phase 1: Lex   │ ── Tokens ──┐
└─────────────────┘             │
                                ▼
┌─────────────────┐         ┌─────────────────┐
│ Phase 2: Parse  │ ◀────── │  Error Recovery │
└─────────────────┘         └─────────────────┘
    │ AST
    ▼
┌─────────────────┐
│Phase 3: Semantic│ ── Symbol Table + Typed AST
└─────────────────┘
    │
    ▼
┌─────────────────┐
│Phase 4: Infer   │ ── Fully Typed AST + Type Constraints
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Phase 5: Lower  │ ── SSA IR Module
└─────────────────┘
    │
    ▼
┌─────────────────┐
│Phase 6: CodeGen │ ── Executable Machine Code
└─────────────────┘
```

Each phase builds upon the previous phase's output, with comprehensive error handling and recovery at each stage.

## Phase 1: Lexical Analysis

**Module**: `src/lexer/`  
**Input**: Raw source code (String)  
**Output**: Token stream (Vec<Token>)

### Purpose
Transform raw source text into a sequence of meaningful tokens, handling Unicode characters, keywords, operators, literals, and comments.

### Key Components

#### Scanner (`src/lexer/scanner.rs`)
The core tokenization engine that processes characters one at a time:

```rust
pub struct Scanner {
    source: Vec<char>,    // Unicode characters
    tokens: Vec<Token>,   // Output token stream
    start: usize,         // Start of current token
    current: usize,       // Current character position
    line: u32,           // Current line number
    column: u32,         // Current column number
}
```

#### Token Types (`src/lexer/token.rs`)
Comprehensive token classification:

```rust
pub enum TokenKind {
    // Literals
    Number(f64),
    String(String),
    Boolean(bool),
    
    // Identifiers and Keywords
    Identifier(String),
    Let, Fn, If, Else, While, For, Return,
    
    // Operators
    Plus, Minus, Star, Slash, Percent,
    Equal, EqualEqual, BangEqual,
    Greater, GreaterEqual, Less, LessEqual,
    
    // Delimiters
    LeftParen, RightParen, LeftBrace, RightBrace,
    Comma, Semicolon, Arrow,
    
    // Special
    Newline, Eof, Error(String),
}
```

### Processing Steps

1. **Character Reading**: Unicode-aware character processing
2. **Token Recognition**: Pattern matching for different token types
3. **Keyword Identification**: Reserved word recognition
4. **Number Parsing**: Integer and floating-point literal parsing
5. **String Processing**: String literal parsing with escape sequences
6. **Error Recovery**: Continue scanning after lexical errors
7. **Position Tracking**: Maintain line and column information for error reporting

### Features

- **Unicode Support**: Full Unicode character set support
- **Error Recovery**: Continue tokenization after errors to find multiple issues
- **Source Positions**: Accurate line/column tracking for error reporting
- **Keyword Recognition**: Efficient keyword identification with hash map lookup
- **Number Literals**: Support for integers, floats, and scientific notation
- **String Literals**: String parsing with escape sequence support

### Example Flow

```rust
// Source: "let x = 42 + y"
// Tokens produced:
[
    Token { kind: Let, span: Span(0, 3) },
    Token { kind: Identifier("x"), span: Span(4, 5) },
    Token { kind: Equal, span: Span(6, 7) },
    Token { kind: Number(42.0), span: Span(8, 10) },
    Token { kind: Plus, span: Span(11, 12) },
    Token { kind: Identifier("y"), span: Span(13, 14) },
    Token { kind: Eof, span: Span(14, 14) },
]
```

## Phase 2: Syntax Analysis

**Module**: `src/parser/`  
**Input**: Token stream (Vec<Token>)  
**Output**: Abstract Syntax Tree (Program)

### Purpose
Build a structured representation of the program's syntax, validating grammatical correctness and creating an AST that represents the program's structure.

### Key Components

#### Parser (`src/parser/parser.rs`)
Recursive descent parser with Pratt parsing for expressions:

```rust
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    had_error: bool,
    panic_mode: bool,
}
```

#### AST Nodes (`src/parser/ast.rs`)
Comprehensive AST node definitions:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal { value: LiteralValue, span: Span },
    Identifier { name: String, span: Span },
    Binary { left: Box<Expr>, op: BinaryOp, right: Box<Expr>, span: Span },
    Unary { op: UnaryOp, expr: Box<Expr>, span: Span },
    Call { callee: Box<Expr>, args: Vec<Expr>, span: Span },
    If { condition: Box<Expr>, then_branch: Box<Expr>, else_branch: Option<Box<Expr>>, span: Span },
    Block { statements: Vec<Stmt>, final_expr: Option<Box<Expr>>, span: Span },
    Match { expr: Box<Expr>, arms: Vec<MatchArm>, span: Span },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let { name: String, type_ann: Option<TypeAnn>, init: Option<Expr>, span: Span },
    Function { name: String, params: Vec<Parameter>, ret_type: Option<TypeAnn>, body: Block, span: Span },
    Expression(Expr),
    Return(Option<Expr>),
    While { condition: Expr, body: Block },
    For { variable: String, iterable: Expr, body: Block },
}
```

### Parsing Strategy

#### Recursive Descent
Most language constructs use recursive descent parsing:
- Statements: `parse_statement()`
- Declarations: `parse_let_declaration()`, `parse_function()`
- Control Flow: `parse_if()`, `parse_while()`, `parse_for()`

#### Pratt Parsing (Operator Precedence)
Expression parsing uses Pratt parsing for proper operator precedence:

```rust
fn parse_expression(&mut self) -> Result<Expr> {
    self.parse_precedence(Precedence::Assignment)
}

fn parse_precedence(&mut self, precedence: Precedence) -> Result<Expr> {
    let prefix = self.parse_primary()?;
    let mut left = prefix;
    
    while precedence <= self.current_precedence() {
        left = self.parse_infix(left)?;
    }
    
    Ok(left)
}
```

#### Precedence Table
```rust
enum Precedence {
    None,
    Assignment,  // =
    Or,          // ||
    And,         // &&
    Equality,    // == !=
    Comparison,  // > >= < <=
    Term,        // + -
    Factor,      // * / %
    Unary,       // ! -
    Call,        // . () []
    Primary,
}
```

### Error Recovery

The parser implements panic-mode recovery:

1. **Error Detection**: When an unexpected token is encountered
2. **Panic Mode**: Set panic flag and skip tokens
3. **Synchronization**: Find statement boundaries (semicolons, keywords)
4. **Recovery**: Resume normal parsing
5. **Multiple Errors**: Continue parsing to find additional errors

### Features

- **Expression-Oriented**: Everything is an expression that can return a value
- **Error Recovery**: Panic-mode recovery with synchronization points
- **Source Spans**: Every AST node includes source location information
- **Type Annotations**: Optional type annotations on variables and functions
- **Pattern Matching**: Match expressions with guards and destructuring
- **Block Expressions**: Blocks that can return values

## Phase 3: Semantic Analysis

**Module**: `src/semantic/`  
**Input**: Abstract Syntax Tree (Program)  
**Output**: Symbol Table + Semantic validation

### Purpose
Perform semantic validation, symbol resolution, scope analysis, and prepare for type inference by building symbol tables and validating program semantics.

### Key Components

#### SemanticAnalyzer (`src/semantic/analyzer.rs`)
Main semantic analysis engine:

```rust
pub struct SemanticAnalyzer {
    symbol_table: SymbolTable,
    current_scope: ScopeId,
    errors: Vec<SemanticError>,
    in_function: bool,
    current_function_return_type: Option<Type>,
}
```

#### SymbolTable (`src/semantic/symbol_table.rs`)
Manages symbols and scopes:

```rust
pub struct SymbolTable {
    scopes: Vec<Scope>,
    current_scope: ScopeId,
}

pub struct Scope {
    parent: Option<ScopeId>,
    symbols: HashMap<String, Symbol>,
    is_function_scope: bool,
}
```

#### Symbol (`src/semantic/symbol.rs`)
Symbol representation:

```rust
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub ty: Option<Type>,
    pub span: Span,
    pub is_mutable: bool,
}

pub enum SymbolKind {
    Variable,
    Function(FunctionSignature),
    Parameter,
    Type,
}
```

### Analysis Steps

1. **Scope Management**: Create and manage nested scopes
2. **Symbol Declaration**: Record variable and function declarations
3. **Symbol Resolution**: Resolve identifier references
4. **Type Compatibility**: Check basic type compatibility
5. **Control Flow**: Validate return statements, break/continue
6. **Function Analysis**: Validate function signatures and calls
7. **Pattern Analysis**: Validate pattern matching constructs

### Semantic Validations

#### Variable Analysis
- Undefined variable detection
- Variable shadowing warnings
- Mutability checks
- Initialization before use

#### Function Analysis
- Function signature validation
- Parameter type checking
- Return type consistency
- Recursive function handling

#### Scope Analysis
- Nested scope management
- Symbol visibility rules
- Scope-based error recovery

#### Control Flow Analysis
- Return statement validation
- Unreachable code detection
- Loop control validation

### Error Types

```rust
pub enum SemanticErrorKind {
    UndefinedVariable(String),
    RedefinedVariable(String),
    TypeMismatch { expected: Type, found: Type },
    InvalidOperation(String),
    ReturnTypeMismatch { expected: Type, found: Type },
    InvalidReturn,
    UndefinedFunction(String),
    ArgumentCountMismatch { expected: usize, found: usize },
}
```

## Phase 4: Type Inference

**Module**: `src/inference/`  
**Input**: Typed AST + Symbol Table  
**Output**: Fully typed AST with inferred types

### Purpose
Implement Hindley-Milner type inference with gradual typing support to infer types for expressions without explicit annotations.

### Key Components

#### InferenceEngine (`src/inference/inference_engine.rs`)
Main type inference engine:

```rust
pub struct InferenceEngine {
    context: InferenceContext,
    type_env: TypeEnv,
    constraints: Vec<Constraint>,
}
```

#### Constraint System (`src/inference/constraint.rs`)
Type constraints for unification:

```rust
pub struct Constraint {
    pub kind: ConstraintKind,
    pub span: Span,
}

pub enum ConstraintKind {
    Equality(Type, Type),
}
```

#### Unification (`src/inference/unification.rs`)
Type unification algorithm:

```rust
pub fn unify(t1: &Type, t2: &Type, span: Span) -> Result<Substitution> {
    match (t1, t2) {
        (Type::TypeVar(id), ty) | (ty, Type::TypeVar(id)) => {
            // Occurs check and substitution
        }
        (Type::Function { params: p1, ret: r1 }, Type::Function { params: p2, ret: r2 }) => {
            // Function type unification
        }
        // ... other cases
    }
}
```

### Inference Algorithm

#### Constraint Generation
1. **Traverse AST**: Visit each expression and statement
2. **Generate Type Variables**: Create fresh type variables for unknowns
3. **Collect Constraints**: Generate equality constraints based on language rules
4. **Function Application**: Constrain argument and parameter types

#### Constraint Solving
1. **Unification**: Apply unification algorithm to constraint pairs
2. **Substitution**: Build substitution mapping type variables to concrete types
3. **Occurs Check**: Prevent infinite types in recursive definitions
4. **Error Reporting**: Report unification failures with source locations

#### Gradual Typing Support
- **Unknown Type**: Special type that unifies with any other type
- **Gradual Conversion**: Automatic insertion of runtime type checks
- **Migration Path**: Allow gradual addition of type annotations

### Type Rules

#### Expression Typing
```rust
// Binary operations
if e1: T1 && e2: T2 && T1 == T2 && T1 is numeric
then (e1 + e2): T1

// Function calls
if f: (T1, T2, ...) -> Tr && args: (T1, T2, ...)
then f(args): Tr

// If expressions
if cond: bool && then_expr: T && else_expr: T
then (if cond then_expr else else_expr): T
```

#### Statement Typing
```rust
// Let bindings
if init: T
then let x = init; x: T

// Function definitions
if params: (T1, T2, ...) && body: Tr
then fn f(params) -> Tr { body }; f: (T1, T2, ...) -> Tr
```

## Phase 5: IR Generation

**Module**: `src/lowering/` and `src/ir/`  
**Input**: Fully typed AST  
**Output**: SSA Intermediate Representation

### Purpose
Transform the high-level AST into a lower-level intermediate representation suitable for optimization and code generation.

### Key Components

#### AstLowerer (`src/lowering/mod.rs`)
Main AST-to-IR translation:

```rust
pub struct AstLowerer {
    builder: IrBuilder,
    context: LoweringContext,
    symbol_table: SymbolTable,
    type_info: HashMap<usize, Type>,
}
```

#### IR Builder (`src/ir/mod.rs`)
SSA IR construction utilities:

```rust
pub struct IrBuilder {
    module: Module,
    current_function: Option<FunctionId>,
    current_block: Option<BlockId>,
    next_value_id: u32,
}
```

#### SSA Instructions (`src/ir/instruction.rs`)
SSA instruction set:

```rust
pub enum Instruction {
    // Arithmetic
    Binary { op: BinaryOp, lhs: ValueId, rhs: ValueId, ty: Type },
    Unary { op: UnaryOp, operand: ValueId, ty: Type },
    
    // Memory
    Alloc { ty: Type },
    Load { ptr: ValueId, ty: Type },
    Store { ptr: ValueId, value: ValueId },
    
    // Control Flow
    Branch(BlockId),
    CondBranch { condition: ValueId, then_block: BlockId, else_block: BlockId },
    Return(Option<ValueId>),
    
    // Function Calls
    Call { func: FunctionId, args: Vec<ValueId>, ty: Type },
    
    // Constants
    Const(Constant),
}
```

### Lowering Process

#### Expression Lowering
1. **Literals**: Convert to constant instructions
2. **Variables**: Load from allocated storage
3. **Binary Operations**: Generate binary instructions with type information
4. **Function Calls**: Generate call instructions with argument marshaling
5. **Control Flow**: Generate conditional branches and blocks

#### Statement Lowering
1. **Let Bindings**: Allocate memory and store initial values
2. **Assignments**: Generate store instructions
3. **Control Flow**: Generate appropriate branch instructions
4. **Return Statements**: Generate return instructions with optional values

#### Control Flow Translation
```rust
// While loop translation
while condition {
    body
}

// Becomes:
entry_block:
    br loop_header

loop_header:
    %cond = <evaluate condition>
    cond_br %cond, loop_body, loop_exit

loop_body:
    <translate body>
    br loop_header

loop_exit:
    <continue with rest of function>
```

### SSA Properties

#### Single Assignment
- Each value is assigned exactly once
- Enables straightforward optimization passes
- Simplifies data flow analysis

#### φ (Phi) Functions
- Handle values that can come from multiple paths
- Placed at control flow merge points
- Essential for proper SSA form

#### Type Preservation
- All instructions maintain type information
- Enables type-based optimizations
- Supports gradual typing requirements

## Phase 6: Code Generation

**Module**: `src/codegen/`  
**Input**: SSA IR Module  
**Output**: Executable machine code

### Purpose
Generate native machine code from the IR using the Cranelift JIT compiler backend.

### Key Components

#### CodeGenerator (`src/codegen/mod.rs`)
High-level code generation interface:

```rust
pub struct CodeGenerator {
    backend: Box<dyn CodegenBackend<Output = ExecutableModule>>,
}
```

#### Cranelift Backend (`src/codegen/cranelift/`)
Cranelift-specific code generation:

```rust
pub struct CraneliftBackend {
    builder_context: FunctionBuilderContext,
    ctx: cranelift::Context,
    module: JITModule,
}
```

### Code Generation Process

#### Module Translation
1. **Function Declaration**: Declare all functions in the Cranelift module
2. **Function Implementation**: Translate each function's IR to Cranelift IR
3. **Type Mapping**: Map Script types to Cranelift types
4. **Memory Layout**: Determine data layout and calling conventions

#### Function Translation
1. **Basic Block Creation**: Create Cranelift basic blocks for each IR block
2. **Instruction Translation**: Map IR instructions to Cranelift instructions
3. **Value Mapping**: Maintain mapping from IR values to Cranelift values
4. **Type Conversion**: Handle type conversions and runtime type checks

#### Runtime Integration
1. **Memory Management**: Integrate with ScriptRc reference counting
2. **Function Calls**: Handle calling conventions for Script functions
3. **Standard Library**: Link with standard library implementations
4. **Error Handling**: Integrate runtime error handling

### Cranelift Integration

#### Type Mapping
```rust
fn map_type(script_type: &Type) -> cranelift::Type {
    match script_type {
        Type::I32 => types::I32,
        Type::F32 => types::F32,
        Type::Bool => types::I8,  // Boolean as byte
        Type::String => types::I64,  // Pointer to ScriptRc<String>
        Type::Array(_) => types::I64,  // Pointer to ScriptRc<Array>
        Type::Function { .. } => types::I64,  // Function pointer
        _ => types::I64,  // Default to pointer size
    }
}
```

#### Instruction Translation
```rust
fn translate_instruction(inst: &Instruction, builder: &mut FunctionBuilder) {
    match inst {
        Instruction::Binary { op, lhs, rhs, .. } => {
            let lhs_val = get_value(*lhs);
            let rhs_val = get_value(*rhs);
            let result = match op {
                BinaryOp::Add => builder.ins().iadd(lhs_val, rhs_val),
                BinaryOp::Sub => builder.ins().isub(lhs_val, rhs_val),
                BinaryOp::Mul => builder.ins().imul(lhs_val, rhs_val),
                BinaryOp::Div => builder.ins().sdiv(lhs_val, rhs_val),
            };
            register_value(result);
        }
        // ... other instructions
    }
}
```

### Executable Module

#### JIT Compilation
- **Just-In-Time**: Compile functions as they're needed
- **Fast Startup**: Minimal compilation overhead
- **Runtime Optimization**: Profile-guided optimization opportunities

#### Memory Management Integration
- **Reference Counting**: Automatic increment/decrement of reference counts
- **Cycle Detection**: Integration with cycle collector
- **Garbage Collection**: Cooperative garbage collection support

#### Function Linking
- **Internal Calls**: Direct function calls within the module
- **External Calls**: Calls to standard library functions
- **Runtime Calls**: Calls to runtime system functions

## Pipeline Integration

### End-to-End Flow

```rust
pub fn compile_and_run(source: &str) -> Result<i32> {
    // Phase 1: Lexical Analysis
    let lexer = Lexer::new(source);
    let (tokens, lex_errors) = lexer.scan_tokens();
    
    // Phase 2: Syntax Analysis
    let mut parser = Parser::new(tokens);
    let ast = parser.parse()?;
    
    // Phase 3: Semantic Analysis
    let mut semantic_analyzer = SemanticAnalyzer::new();
    semantic_analyzer.analyze_program(&ast)?;
    
    // Phase 4: Type Inference
    let mut inference_engine = InferenceEngine::new();
    let typed_ast = inference_engine.infer_types(&ast)?;
    
    // Phase 5: IR Generation
    let mut lowerer = AstLowerer::new(
        semantic_analyzer.symbol_table(),
        inference_engine.type_info()
    );
    let ir_module = lowerer.lower_program(&typed_ast)?;
    
    // Phase 6: Code Generation
    let mut codegen = CodeGenerator::new();
    let executable = codegen.generate(&ir_module)?;
    
    // Execute
    executable.execute()
}
```

### Error Aggregation

Each phase can produce multiple errors:

```rust
pub struct CompilationResult {
    pub executable: Option<ExecutableModule>,
    pub lexical_errors: Vec<LexicalError>,
    pub syntax_errors: Vec<SyntaxError>,
    pub semantic_errors: Vec<SemanticError>,
    pub type_errors: Vec<TypeError>,
    pub lowering_errors: Vec<LoweringError>,
    pub codegen_errors: Vec<CodegenError>,
}
```

### Incremental Compilation

Future support for incremental compilation:
- **Change Detection**: Track source file modifications
- **Dependency Analysis**: Understand module dependencies
- **Selective Recompilation**: Only recompile changed modules
- **Cache Management**: Persist compilation artifacts

## Error Handling Strategy

### Error Philosophy
- **Early Detection**: Catch errors as early as possible in the pipeline
- **Error Recovery**: Continue processing to find multiple errors
- **Rich Context**: Provide detailed error messages with source locations
- **User-Friendly**: Clear, actionable error messages

### Error Categories

#### Lexical Errors
- Invalid characters
- Malformed number literals
- Unterminated strings
- Invalid escape sequences

#### Syntax Errors
- Unexpected tokens
- Missing delimiters
- Invalid expression structure
- Malformed statements

#### Semantic Errors
- Undefined variables
- Type mismatches
- Invalid operations
- Scope violations

#### Type Errors
- Unification failures
- Constraint violations
- Type annotation conflicts
- Gradual typing issues

### Error Recovery Strategies

#### Lexical Recovery
- Skip invalid characters
- Continue tokenization
- Report multiple errors per scan

#### Syntax Recovery
- Panic-mode recovery
- Synchronization on statement boundaries
- Multiple error reporting per parse

#### Semantic Recovery
- Continue analysis after errors
- Build partial symbol tables
- Enable downstream phases when possible

## Optimization Opportunities

### Current Optimizations

#### SSA Benefits
- Dead code elimination
- Constant propagation
- Common subexpression elimination
- Loop invariant code motion

#### Cranelift Optimizations
- Register allocation
- Instruction scheduling
- Basic block optimization
- Target-specific optimizations

### Future Optimization Passes

#### High-Level Optimizations
- Function inlining
- Loop unrolling
- Tail call optimization
- Pattern matching optimization

#### Memory Optimizations
- Reference count optimization
- Escape analysis
- Stack allocation for local objects
- Copy elimination

#### Runtime Optimizations
- Polymorphic inline caching
- Type specialization
- Profile-guided optimization
- Adaptive compilation

### Optimization Pipeline

```rust
pub struct OptimizationPipeline {
    passes: Vec<Box<dyn OptimizationPass>>,
}

pub trait OptimizationPass {
    fn run(&self, module: &mut IrModule) -> Result<bool>;
    fn name(&self) -> &str;
}
```

This comprehensive pipeline architecture provides a solid foundation for the Script programming language, with clear phase separation, robust error handling, and opportunities for future optimization enhancements.