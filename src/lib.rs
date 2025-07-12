pub mod codegen;
pub mod compilation;
pub mod debugger;
pub mod doc;
pub mod error;
pub mod inference;
pub mod ir;
pub mod lexer;
pub mod lowering;
pub mod lsp;
pub mod manuscript;
#[cfg(feature = "mcp")]
pub mod mcp;
pub mod metaprogramming;
pub mod module;
pub mod package;
pub mod parser;
pub mod runtime;
pub mod security;
pub mod semantic;
pub mod source;
pub mod stdlib;
pub mod testing;
pub mod types;
pub mod update;
pub mod verification;

#[cfg(test)]
mod tests {
    mod async_tests;
}

pub use codegen::{CodeGenerator, ExecutableModule};
pub use compilation::{DependencyAnalyzer, DependencyGraph};
pub use debugger::{
    get_debugger, initialize_debugger, is_debugger_initialized, shutdown_debugger, Breakpoint,
    BreakpointCondition, BreakpointId, BreakpointManager, BreakpointType, DebugEvent, DebugHook,
    DebugSession, Debugger, DebuggerState, ExecutionContext, RuntimeDebugInterface,
};
pub use error::{Error, Result};
pub use inference::{InferenceEngine, InferenceResult};
pub use ir::{IrBuilder, Module as IrModule};
pub use lexer::{Lexer, Token, TokenKind};
pub use lowering::AstLowerer;
pub use metaprogramming::MetaprogrammingProcessor;
pub use module::{
    FileSystemResolver, ImportPath, ModuleCompilationPipeline, ModuleError, ModulePath,
    ModuleRegistry, ModuleResolver, ModuleResult, ResolvedModule,
};
pub use package::{Package, PackageManager, PackageManifest, Version};
pub use parser::{Expr, Parser, Program, Stmt};
pub use runtime::{Runtime, RuntimeConfig, ScriptRc, ScriptWeak};
pub use semantic::{SemanticAnalyzer, Symbol, SymbolTable};
pub use source::SourceLocation;
pub use testing::{
    Assertion, AssertionError, ConsoleReporter, TestCase, TestDiscovery, TestReporter, TestResult,
    TestRunOptions, TestRunner, TestStatus, TestSuite, TestingFramework,
};
pub use types::{Type, TypeEnv};

// Re-export macros are defined in respective modules
