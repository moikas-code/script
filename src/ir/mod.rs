//! Intermediate Representation (IR) for the Script language
//! 
//! This module defines the IR used as an intermediate step between the AST
//! and machine code generation. The IR is designed to be:
//! - Simple and explicit (no implicit operations)
//! - Type-preserving (maintains type information from the frontend)
//! - SSA-based (Static Single Assignment) for easier optimization
//! - Backend-agnostic (can target Cranelift, LLVM, or other backends)

use crate::types::Type;
use std::fmt;

pub mod instruction;
pub mod value;
pub mod block;
pub mod function;
pub mod module;

pub use instruction::{Instruction, BinaryOp, UnaryOp, ComparisonOp};
pub use value::{Value, ValueId, Constant};
pub use block::{BasicBlock, BlockId};
pub use function::{Function, FunctionId, Parameter};
pub use module::Module;

/// IR Builder context for constructing IR
#[derive(Debug)]
pub struct IrBuilder {
    /// Current module being built
    module: Module,
    /// Current function being built
    current_function: Option<FunctionId>,
    /// Current basic block being built
    current_block: Option<BlockId>,
    /// Next value ID to allocate
    next_value_id: u32,
}

impl IrBuilder {
    /// Create a new IR builder
    pub fn new() -> Self {
        IrBuilder {
            module: Module::new(),
            current_function: None,
            current_block: None,
            next_value_id: 0,
        }
    }
    
    /// Get the built module
    pub fn build(self) -> Module {
        self.module
    }
    
    /// Create a new function
    pub fn create_function(&mut self, name: String, params: Vec<Parameter>, return_type: Type) -> FunctionId {
        let func_id = self.module.create_function(name, params, return_type);
        self.current_function = Some(func_id);
        
        // Create entry block for the function
        if let Some(func) = self.module.get_function_mut(func_id) {
            let entry_block = func.create_block("entry".to_string());
            self.current_block = Some(entry_block);
        }
        
        func_id
    }
    
    /// Get current function
    pub fn current_function(&self) -> Option<FunctionId> {
        self.current_function
    }
    
    /// Set current function
    pub fn set_current_function(&mut self, func_id: FunctionId) {
        self.current_function = Some(func_id);
    }
    
    /// Create a new basic block in the current function
    pub fn create_block(&mut self, name: String) -> Option<BlockId> {
        if let Some(func_id) = self.current_function {
            if let Some(func) = self.module.get_function_mut(func_id) {
                let block_id = func.create_block(name);
                return Some(block_id);
            }
        }
        None
    }
    
    /// Set current block
    pub fn set_current_block(&mut self, block: BlockId) {
        self.current_block = Some(block);
    }
    
    /// Get current block
    pub fn get_current_block(&self) -> Option<BlockId> {
        self.current_block
    }
    
    /// Generate a new value ID
    pub fn next_value_id(&mut self) -> ValueId {
        let id = ValueId(self.next_value_id);
        self.next_value_id += 1;
        id
    }
    
    /// Add an instruction to the current block
    pub fn add_instruction(&mut self, inst: Instruction) -> Option<ValueId> {
        if let (Some(func_id), Some(block_id)) = (self.current_function, self.current_block) {
            let value_id = self.next_value_id();
            if let Some(func) = self.module.get_function_mut(func_id) {
                if let Some(block) = func.get_block_mut(block_id) {
                    block.add_instruction(value_id, inst);
                    return Some(value_id);
                }
            }
        }
        None
    }
    
    /// Build a constant value
    pub fn const_value(&mut self, constant: Constant) -> ValueId {
        let value_id = self.next_value_id();
        let inst = Instruction::Const(constant);
        self.add_instruction(inst).unwrap_or(value_id)
    }
    
    /// Build a binary operation
    pub fn build_binary(&mut self, op: BinaryOp, lhs: ValueId, rhs: ValueId, ty: Type) -> Option<ValueId> {
        self.add_instruction(Instruction::Binary { op, lhs, rhs, ty })
    }
    
    /// Build a unary operation
    pub fn build_unary(&mut self, op: UnaryOp, operand: ValueId, ty: Type) -> Option<ValueId> {
        self.add_instruction(Instruction::Unary { op, operand, ty })
    }
    
    /// Build a comparison
    pub fn build_compare(&mut self, op: ComparisonOp, lhs: ValueId, rhs: ValueId) -> Option<ValueId> {
        self.add_instruction(Instruction::Compare { op, lhs, rhs })
    }
    
    /// Build a function call
    pub fn build_call(&mut self, func: FunctionId, args: Vec<ValueId>, ty: Type) -> Option<ValueId> {
        self.add_instruction(Instruction::Call { func, args, ty })
    }
    
    /// Build a return instruction
    pub fn build_return(&mut self, value: Option<ValueId>) {
        self.add_instruction(Instruction::Return(value));
    }
    
    /// Build an unconditional branch
    pub fn build_branch(&mut self, target: BlockId) {
        self.add_instruction(Instruction::Branch(target));
    }
    
    /// Build a conditional branch
    pub fn build_cond_branch(&mut self, condition: ValueId, then_block: BlockId, else_block: BlockId) {
        self.add_instruction(Instruction::CondBranch { 
            condition, 
            then_block, 
            else_block 
        });
    }
    
    /// Build an allocation
    pub fn build_alloc(&mut self, ty: Type) -> Option<ValueId> {
        self.add_instruction(Instruction::Alloc { ty })
    }
    
    /// Build a load
    pub fn build_load(&mut self, ptr: ValueId, ty: Type) -> Option<ValueId> {
        self.add_instruction(Instruction::Load { ptr, ty })
    }
    
    /// Build a store
    pub fn build_store(&mut self, ptr: ValueId, value: ValueId) {
        self.add_instruction(Instruction::Store { ptr, value });
    }
}

impl Default for IrBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// IR validation error
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    /// Use of undefined value
    UndefinedValue(ValueId),
    /// Type mismatch
    TypeMismatch { expected: Type, found: Type },
    /// Invalid instruction
    InvalidInstruction(String),
    /// Control flow error
    ControlFlowError(String),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::UndefinedValue(id) => write!(f, "Undefined value: {}", id),
            ValidationError::TypeMismatch { expected, found } => {
                write!(f, "Type mismatch: expected {}, found {}", expected, found)
            }
            ValidationError::InvalidInstruction(msg) => write!(f, "Invalid instruction: {}", msg),
            ValidationError::ControlFlowError(msg) => write!(f, "Control flow error: {}", msg),
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod builder_tests {
    use super::*;
    
    #[test]
    fn test_ir_builder_basic() {
        let mut builder = IrBuilder::new();
        
        // Create a simple function
        let _func_id = builder.create_function(
            "test".to_string(),
            vec![],
            Type::I32
        );
        
        // Build a constant
        let const_val = builder.const_value(Constant::I32(42));
        
        // Return the constant
        builder.build_return(Some(const_val));
        
        let module = builder.build();
        assert_eq!(module.functions().len(), 1);
    }
}