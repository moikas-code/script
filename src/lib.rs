pub mod error;
pub mod lexer;
pub mod parser;
pub mod source;
pub mod types;
pub mod inference;
pub mod semantic;
pub mod ir;
pub mod lowering;
pub mod codegen;
pub mod runtime;
pub mod stdlib;

pub use error::{Error, Result};
pub use lexer::{Lexer, Token, TokenKind};
pub use parser::{Parser, Program, Stmt, Expr};
pub use source::SourceLocation;
pub use types::{Type, TypeEnv};
pub use inference::{InferenceEngine, InferenceResult};
pub use semantic::{SemanticAnalyzer, Symbol, SymbolTable};
pub use ir::{IrBuilder, Module as IrModule};
pub use lowering::AstLowerer;
pub use codegen::{CodeGenerator, ExecutableModule};
pub use runtime::{Runtime, RuntimeConfig, ScriptRc, ScriptWeak};

// Re-export macros are defined in respective modules