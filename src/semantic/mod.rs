mod symbol;
mod symbol_table;
mod analyzer;
mod error;

pub use symbol::{Symbol, SymbolKind, FunctionSignature};
pub use symbol_table::{SymbolTable, ScopeId};
pub use analyzer::SemanticAnalyzer;
pub use error::{SemanticError, SemanticErrorKind};

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