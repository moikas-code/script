mod analyzer;
mod error;
pub mod memory_safety;
mod symbol;
mod symbol_table;

pub use analyzer::SemanticAnalyzer;
pub use error::{SemanticError, SemanticErrorKind};
pub use memory_safety::{MemorySafetyContext, MemorySafetyViolation};
pub use symbol::{FunctionSignature, Symbol, SymbolKind};
pub use symbol_table::{ScopeId, SymbolTable};

use crate::parser::Program;
use crate::Result;

/// Perform semantic analysis on a parsed program
pub fn analyze(program: &Program) -> Result<SemanticAnalyzer> {
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze_program(program)?;
    Ok(analyzer)
}

#[cfg(test)]
mod tests;
