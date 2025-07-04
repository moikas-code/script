use super::{AstLowerer, LoweringResult};
use crate::error::{Error, ErrorKind};
use crate::ir::Constant;
use crate::parser::{Stmt, StmtKind};

/// Lower a statement
pub fn lower_statement(lowerer: &mut AstLowerer, stmt: &Stmt) -> LoweringResult<()> {
    match &stmt.kind {
        StmtKind::Let { name, init, .. } => lower_let(lowerer, name, init.as_ref()),
        StmtKind::Expression(expr) => {
            lowerer.lower_expression(expr)?;
            Ok(())
        }
        StmtKind::Return(expr) => lower_return(lowerer, expr.as_ref()),
        StmtKind::While { condition, body } => lowerer.lower_while(condition, body),
        StmtKind::For {
            variable,
            iterable,
            body,
        } => lowerer.lower_for(variable, iterable, body),
        StmtKind::Function { .. } => {
            // Functions are handled in the first pass
            Ok(())
        }
        StmtKind::Import { .. } => {
            // Imports are handled during semantic analysis
            Ok(())
        }
        StmtKind::Export { .. } => {
            // Exports are handled during semantic analysis
            Ok(())
        }
        StmtKind::Struct { name, .. } => {
            // TODO: Implement struct definition lowering
            // Structs are handled during semantic analysis
            let _ = name; // suppress warning
            Ok(())
        }
        StmtKind::Enum { name, .. } => {
            // TODO: Implement enum definition lowering
            // Enums are handled during semantic analysis
            let _ = name; // suppress warning
            Ok(())
        }
    }
}

/// Lower a let statement
fn lower_let(
    lowerer: &mut AstLowerer,
    name: &str,
    init: Option<&crate::parser::Expr>,
) -> LoweringResult<()> {
    if let Some(init_expr) = init {
        let value = lowerer.lower_expression(init_expr)?;
        let ty = lowerer.get_expression_type(init_expr)?;

        // Allocate memory for the variable
        let ptr = lowerer
            .builder
            .build_alloc(ty.clone())
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to allocate variable"))?;

        // Store the initial value
        lowerer.builder.build_store(ptr, value);

        // Register the variable
        lowerer.context.define_variable(name.to_string(), ptr, ty);
    } else {
        // Uninitialized variable - allocate with default value
        let ty = crate::types::Type::Unknown; // TODO: Get type from annotation
        let ptr = lowerer
            .builder
            .build_alloc(ty.clone())
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to allocate variable"))?;

        // Initialize with default value
        let default_value = lowerer.builder.const_value(Constant::Null);
        lowerer.builder.build_store(ptr, default_value);

        lowerer.context.define_variable(name.to_string(), ptr, ty);
    }

    Ok(())
}

/// Lower a return statement
fn lower_return(
    lowerer: &mut AstLowerer,
    expr: Option<&crate::parser::Expr>,
) -> LoweringResult<()> {
    let value = expr.map(|e| lowerer.lower_expression(e)).transpose()?;

    lowerer.builder.build_return(value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::{Parser, Stmt};
    use crate::semantic::SymbolTable;
    use std::collections::HashMap;

    fn parse_statement(source: &str) -> Stmt {
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.scan_tokens();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Failed to parse");
        program.statements.into_iter().next().expect("No statement")
    }

    #[test]
    fn test_lower_let_with_init() {
        let stmt = parse_statement("let x = 42");

        let symbol_table = SymbolTable::new();
        let type_info = HashMap::new();
        let mut lowerer = AstLowerer::new(symbol_table, type_info);

        // Create a dummy function context
        lowerer
            .builder
            .create_function("test".to_string(), vec![], crate::types::Type::Unknown);

        let result = lower_statement(&mut lowerer, &stmt);
        assert!(result.is_ok());

        // Check that variable was registered
        assert!(lowerer.context.lookup_variable("x").is_some());
    }

    #[test]
    fn test_lower_return() {
        let stmt = parse_statement("return 42");

        let symbol_table = SymbolTable::new();
        let type_info = HashMap::new();
        let mut lowerer = AstLowerer::new(symbol_table, type_info);

        // Create a dummy function context
        lowerer
            .builder
            .create_function("test".to_string(), vec![], crate::types::Type::I32);

        let result = lower_statement(&mut lowerer, &stmt);
        assert!(result.is_ok());
    }
}
