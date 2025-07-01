//! AST to IR lowering module
//! 
//! This module is responsible for transforming the typed AST into IR representation.
//! The lowering process preserves type information and translates high-level constructs
//! into simpler IR instructions.

use crate::parser::{Program, Stmt, StmtKind, Expr, Block};
use crate::ir::{IrBuilder, Module as IrModule, ValueId, Parameter, Constant};
use crate::types::Type;
use crate::semantic::SymbolTable;
use crate::error::{Error, ErrorKind};
use std::collections::HashMap;
use std::mem;

pub mod context;
pub mod expr;
pub mod stmt;

pub use context::LoweringContext;

/// Result type for lowering operations
pub type LoweringResult<T> = Result<T, Error>;

/// Lower an AST program to IR
pub struct AstLowerer {
    /// IR builder
    builder: IrBuilder,
    /// Lowering context
    context: LoweringContext,
    /// Symbol table from semantic analysis
    #[allow(dead_code)]
    symbol_table: SymbolTable,
    /// Type information from semantic analysis
    #[allow(dead_code)]
    type_info: HashMap<usize, Type>, // Maps expression IDs to types
}

impl AstLowerer {
    /// Create a new AST lowerer
    pub fn new(symbol_table: SymbolTable, type_info: HashMap<usize, Type>) -> Self {
        AstLowerer {
            builder: IrBuilder::new(),
            context: LoweringContext::new(),
            symbol_table,
            type_info,
        }
    }
    
    /// Lower a program to IR
    pub fn lower_program(&mut self, program: &Program) -> LoweringResult<IrModule> {
        // First pass: collect all function declarations
        for stmt in &program.statements {
            if let StmtKind::Function { name, params, ret_type, .. } = &stmt.kind {
                let ir_params: Vec<Parameter> = params.iter()
                    .map(|p| Parameter {
                        name: p.name.clone(),
                        ty: self.convert_type_annotation(&p.type_ann),
                    })
                    .collect();
                
                let return_type = ret_type.as_ref()
                    .map(|t| self.convert_type_annotation(t))
                    .unwrap_or(Type::Unknown);
                
                let func_id = self.builder.create_function(name.clone(), ir_params, return_type);
                self.context.register_function(name.clone(), func_id);
            }
        }
        
        // Second pass: lower function bodies and global statements
        for stmt in &program.statements {
            match &stmt.kind {
                StmtKind::Function { name, body, .. } => {
                    self.lower_function(name, body)?;
                }
                _ => {
                    // Global statements go into a special main function
                    self.ensure_main_function();
                    self.lower_statement(stmt)?;
                }
            }
        }
        
        // Finalize main function if it has statements
        self.finalize_main_function();
        
        Ok(mem::replace(&mut self.builder, IrBuilder::new()).build())
    }
    
    /// Lower a function body
    fn lower_function(&mut self, name: &str, body: &Block) -> LoweringResult<()> {
        let func_id = self.context.get_function(name)
            .ok_or_else(|| Error::new(ErrorKind::TypeError, format!("Function '{}' not found", name)))?;
        
        self.builder.set_current_function(func_id);
        self.context.enter_function(func_id);
        
        // Register function parameters as variables
        // For now, we'll skip this as we don't have proper parameter handling yet
        // TODO: Implement proper parameter value handling
        
        // Lower the function body
        self.lower_block(body)?;
        
        // Ensure the function has a return
        self.ensure_return();
        
        self.context.exit_function();
        Ok(())
    }
    
    /// Lower a block
    fn lower_block(&mut self, block: &Block) -> LoweringResult<Option<ValueId>> {
        self.context.push_scope();
        
        // Lower statements
        for stmt in &block.statements {
            self.lower_statement(stmt)?;
        }
        
        // Lower final expression if present
        let result = if let Some(final_expr) = &block.final_expr {
            Some(self.lower_expression(final_expr)?)
        } else {
            None
        };
        
        self.context.pop_scope();
        Ok(result)
    }
    
    /// Lower a statement
    fn lower_statement(&mut self, stmt: &Stmt) -> LoweringResult<()> {
        match &stmt.kind {
            StmtKind::Let { name, init, .. } => {
                if let Some(init_expr) = init {
                    let value = self.lower_expression(init_expr)?;
                    let ty = self.get_expression_type(init_expr)?;
                    
                    // Allocate memory for the variable
                    let ptr = self.builder.build_alloc(ty.clone())
                        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to allocate variable"))?;
                    
                    // Store the initial value
                    self.builder.build_store(ptr, value);
                    
                    // Register the variable
                    self.context.define_variable(name.clone(), ptr, ty);
                }
            }
            
            StmtKind::Expression(expr) => {
                self.lower_expression(expr)?;
            }
            
            StmtKind::Return(expr) => {
                let value = expr.as_ref()
                    .map(|e| self.lower_expression(e))
                    .transpose()?;
                self.builder.build_return(value);
            }
            
            StmtKind::While { condition, body } => {
                self.lower_while(condition, body)?;
            }
            
            StmtKind::For { variable, iterable, body } => {
                self.lower_for(variable, iterable, body)?;
            }
            
            StmtKind::Function { .. } => {
                // Functions are handled in the first pass
            }
        }
        
        Ok(())
    }
    
    /// Lower an expression
    fn lower_expression(&mut self, expr: &Expr) -> LoweringResult<ValueId> {
        expr::lower_expression(self, expr)
    }
    
    /// Lower a while loop
    fn lower_while(&mut self, condition: &Expr, body: &Block) -> LoweringResult<()> {
        // Create loop blocks
        let cond_block = self.builder.create_block("while.cond".to_string())
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create condition block"))?;
        let body_block = self.builder.create_block("while.body".to_string())
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create body block"))?;
        let after_block = self.builder.create_block("while.after".to_string())
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create after block"))?;
        
        // Jump to condition block
        self.builder.build_branch(cond_block);
        
        // Condition block
        self.builder.set_current_block(cond_block);
        let cond_value = self.lower_expression(condition)?;
        self.builder.build_cond_branch(cond_value, body_block, after_block);
        
        // Body block
        self.builder.set_current_block(body_block);
        self.lower_block(body)?;
        self.builder.build_branch(cond_block);
        
        // After block
        self.builder.set_current_block(after_block);
        
        Ok(())
    }
    
    /// Lower a for loop
    fn lower_for(&mut self, variable: &str, iterable: &Expr, body: &Block) -> LoweringResult<()> {
        // For now, we'll implement a simple version
        // In the future, this would handle iterators properly
        
        // Lower the iterable
        let _iter_value = self.lower_expression(iterable)?;
        
        // TODO: Implement proper for loop lowering with iterators
        // For now, just lower the body once as a placeholder
        self.context.push_scope();
        
        // Create a placeholder variable
        let placeholder = self.builder.const_value(Constant::I32(0));
        let ty = Type::I32; // Placeholder type
        let ptr = self.builder.build_alloc(ty.clone())
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to allocate loop variable"))?;
        self.builder.build_store(ptr, placeholder);
        self.context.define_variable(variable.to_string(), ptr, ty);
        
        self.lower_block(body)?;
        
        self.context.pop_scope();
        
        Ok(())
    }
    
    /// Convert a type annotation to a Type
    fn convert_type_annotation(&self, _type_ann: &crate::parser::TypeAnn) -> Type {
        // TODO: Implement proper type annotation conversion
        Type::Unknown
    }
    
    /// Get the type of an expression
    fn get_expression_type(&self, _expr: &Expr) -> LoweringResult<Type> {
        // For now, return a placeholder
        // In a real implementation, this would look up the type from semantic analysis
        Ok(Type::Unknown)
    }
    
    /// Ensure main function exists
    fn ensure_main_function(&mut self) {
        if self.context.get_function("main").is_none() {
            let main_id = self.builder.create_function("main".to_string(), vec![], Type::Unknown);
            self.context.register_function("main".to_string(), main_id);
            self.builder.set_current_function(main_id);
        }
    }
    
    /// Finalize main function
    fn finalize_main_function(&mut self) {
        if let Some(main_id) = self.context.get_function("main") {
            let current = self.builder.current_function();
            self.builder.set_current_function(main_id);
            self.ensure_return();
            if let Some(prev) = current {
                self.builder.set_current_function(prev);
            }
        }
    }
    
    /// Ensure the current function has a return
    fn ensure_return(&mut self) {
        // Check if the current block already has a terminator
        // If not, add a return
        self.builder.build_return(None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use crate::lexer::Lexer;
    
    fn lower_source(source: &str) -> LoweringResult<IrModule> {
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.scan_tokens();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Failed to parse");
        
        // Create dummy symbol table and type info for testing
        let symbol_table = SymbolTable::new();
        let type_info = HashMap::new();
        
        let mut lowerer = AstLowerer::new(symbol_table, type_info);
        lowerer.lower_program(&program)
    }
    
    #[test]
    fn test_lower_empty_program() {
        let module = lower_source("").unwrap();
        assert_eq!(module.functions().len(), 0);
    }
    
    #[test]
    fn test_lower_simple_function() {
        let source = r#"
            fn add(x: i32, y: i32) -> i32 {
                return 42
            }
        "#;
        
        let module = lower_source(source).unwrap();
        assert_eq!(module.functions().len(), 1);
        
        let func = module.get_function_by_name("add").unwrap();
        assert_eq!(func.name, "add");
        assert_eq!(func.params.len(), 2);
    }
}