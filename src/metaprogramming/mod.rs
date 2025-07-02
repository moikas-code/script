use crate::error::{Error, ErrorKind, Result};
use crate::parser::{Stmt, StmtKind};

pub mod const_eval;
pub mod derive;
pub mod generate;

use const_eval::ConstEvaluator;
use derive::DeriveProcessor;
use generate::GenerateProcessor;

#[cfg(test)]
mod tests;

/// Main metaprogramming processor that handles all attributes
pub struct MetaprogrammingProcessor {
    derive_processor: DeriveProcessor,
    const_evaluator: ConstEvaluator,
    generate_processor: GenerateProcessor,
}

impl MetaprogrammingProcessor {
    pub fn new() -> Self {
        Self {
            derive_processor: DeriveProcessor::new(),
            const_evaluator: ConstEvaluator::new(),
            generate_processor: GenerateProcessor::new(),
        }
    }

    /// Process attributes on a statement and return any generated code
    pub fn process_statement(&mut self, stmt: &mut Stmt) -> Result<Vec<Stmt>> {
        let mut generated_stmts = Vec::new();

        for attr in &stmt.attributes {
            match attr.name.as_str() {
                "derive" => {
                    // Process derive attributes for automatic trait implementations
                    if let StmtKind::Function { name, .. } = &stmt.kind {
                        let derived = self.derive_processor.process_derive(attr, name, stmt)?;
                        generated_stmts.extend(derived);
                    } else {
                        return Err(Error::new(
                            ErrorKind::SemanticError,
                            "@derive can only be applied to functions or types",
                        ));
                    }
                }
                "const" => {
                    // Mark function for compile-time evaluation
                    if let StmtKind::Function { .. } = &stmt.kind {
                        self.const_evaluator.register_const_function(stmt)?;
                    } else {
                        return Err(Error::new(
                            ErrorKind::SemanticError,
                            "@const can only be applied to functions",
                        ));
                    }
                }
                "generate" => {
                    // Generate code using external processors
                    let generated = self.generate_processor.process_generate(attr, stmt)?;
                    generated_stmts.extend(generated);
                }
                _ => {
                    // Unknown attribute - could be custom or future extension
                    // For now, we'll just ignore it
                }
            }
        }

        Ok(generated_stmts)
    }
}

impl Default for MetaprogrammingProcessor {
    fn default() -> Self {
        Self::new()
    }
}
