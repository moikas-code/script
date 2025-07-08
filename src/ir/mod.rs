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

pub mod block;
pub mod function;
pub mod instruction;
pub mod layout;
pub mod module;
pub mod optimizer;
pub mod value;

pub use block::{BasicBlock, BlockId};
pub use function::{Function, FunctionId, Parameter};
pub use instruction::{
    BinaryOp, ComparisonOp, Constant, Instruction, InstructionWithLocation, UnaryOp,
};
pub use layout::{
    EnumLayout, FieldLayout, LayoutCalculator, StructLayout, TypeLayout, VariantDataLayout,
    VariantLayout,
};
pub use module::Module;
pub use value::{Value, ValueId};

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
    pub fn create_function(
        &mut self,
        name: String,
        params: Vec<Parameter>,
        return_type: Type,
    ) -> FunctionId {
        let func_id = self.module.create_function(name, params, return_type);
        self.current_function = Some(func_id);

        // Create entry block for the function
        if let Some(func) = self.module.get_function_mut(func_id) {
            let entry_block = func.create_block("entry".to_string());
            self.current_block = Some(entry_block);
        }

        func_id
    }

    /// Create a new async function
    pub fn create_async_function(
        &mut self,
        name: String,
        params: Vec<Parameter>,
        return_type: Type,
    ) -> FunctionId {
        let func_id = self.module.create_async_function(name, params, return_type);
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

    /// Get a mutable reference to the module
    pub fn module_mut(&mut self) -> &mut Module {
        &mut self.module
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
    pub fn build_binary(
        &mut self,
        op: BinaryOp,
        lhs: ValueId,
        rhs: ValueId,
        ty: Type,
    ) -> Option<ValueId> {
        self.add_instruction(Instruction::Binary { op, lhs, rhs, ty })
    }

    /// Build a unary operation
    pub fn build_unary(&mut self, op: UnaryOp, operand: ValueId, ty: Type) -> Option<ValueId> {
        self.add_instruction(Instruction::Unary { op, operand, ty })
    }

    /// Build a comparison
    pub fn build_compare(
        &mut self,
        op: ComparisonOp,
        lhs: ValueId,
        rhs: ValueId,
    ) -> Option<ValueId> {
        self.add_instruction(Instruction::Compare { op, lhs, rhs })
    }

    /// Build a function call
    pub fn build_call(
        &mut self,
        func: FunctionId,
        args: Vec<ValueId>,
        ty: Type,
    ) -> Option<ValueId> {
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
    pub fn build_cond_branch(
        &mut self,
        condition: ValueId,
        then_block: BlockId,
        else_block: BlockId,
    ) {
        self.add_instruction(Instruction::CondBranch {
            condition,
            then_block,
            else_block,
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

    /// Build a get field pointer instruction
    pub fn build_get_field_ptr(
        &mut self,
        object: ValueId,
        field_name: String,
        field_ty: Type,
    ) -> Option<ValueId> {
        self.add_instruction(Instruction::GetFieldPtr {
            object,
            field_name,
            field_ty,
        })
    }

    /// Build a load field instruction
    pub fn build_load_field(
        &mut self,
        object: ValueId,
        field_name: String,
        field_ty: Type,
    ) -> Option<ValueId> {
        self.add_instruction(Instruction::LoadField {
            object,
            field_name,
            field_ty,
        })
    }

    /// Build a store field instruction
    pub fn build_store_field(&mut self, object: ValueId, field_name: String, value: ValueId) {
        self.add_instruction(Instruction::StoreField {
            object,
            field_name,
            value,
        });
    }

    /// Build an allocate struct instruction
    pub fn build_alloc_struct(&mut self, struct_name: String, ty: Type) -> Option<ValueId> {
        self.add_instruction(Instruction::AllocStruct { struct_name, ty })
    }

    /// Build a construct struct instruction
    pub fn build_construct_struct(
        &mut self,
        struct_name: String,
        fields: Vec<(String, ValueId)>,
        ty: Type,
    ) -> Option<ValueId> {
        self.add_instruction(Instruction::ConstructStruct {
            struct_name,
            fields,
            ty,
        })
    }

    /// Build an allocate enum instruction
    pub fn build_alloc_enum(
        &mut self,
        enum_name: String,
        variant_size: u32,
        ty: Type,
    ) -> Option<ValueId> {
        self.add_instruction(Instruction::AllocEnum {
            enum_name,
            variant_size,
            ty,
        })
    }

    /// Build a construct enum instruction
    pub fn build_construct_enum(
        &mut self,
        enum_name: String,
        variant: String,
        tag: u32,
        args: Vec<ValueId>,
        ty: Type,
    ) -> Option<ValueId> {
        self.add_instruction(Instruction::ConstructEnum {
            enum_name,
            variant,
            tag,
            args,
            ty,
        })
    }

    /// Build a get enum tag instruction
    pub fn build_get_enum_tag(&mut self, enum_value: ValueId) -> Option<ValueId> {
        self.add_instruction(Instruction::GetEnumTag { enum_value })
    }

    /// Build a set enum tag instruction
    pub fn build_set_enum_tag(&mut self, enum_ptr: ValueId, tag: u32) -> Option<ValueId> {
        self.add_instruction(Instruction::SetEnumTag { enum_ptr, tag })
    }

    /// Build an extract enum data instruction
    pub fn build_extract_enum_data(&mut self, enum_value: ValueId, variant_index: u32, ty: Type) -> Option<ValueId> {
        self.add_instruction(Instruction::ExtractEnumData { enum_value, variant_index, ty })
    }

    /// Check if the current block has a terminator instruction
    pub fn current_block_has_terminator(&self) -> bool {
        if let (Some(func_id), Some(block_id)) = (self.current_function, self.current_block) {
            if let Some(func) = self.module.get_function(func_id) {
                if let Some(block) = func.get_block(block_id) {
                    return block.has_terminator();
                }
            }
        }
        false
    }

    /// Build a suspend instruction for async functions
    pub fn build_suspend(&mut self, state: ValueId, resume_block: BlockId) {
        self.add_instruction(Instruction::Suspend { state, resume_block });
    }

    /// Build a poll future instruction
    pub fn build_poll_future(&mut self, future: ValueId, output_ty: Type) -> Option<ValueId> {
        self.add_instruction(Instruction::PollFuture { future, output_ty })
    }

    /// Build a create async state instruction
    pub fn build_create_async_state(
        &mut self,
        initial_state: u32,
        state_size: u32,
        output_ty: Type,
    ) -> Option<ValueId> {
        self.add_instruction(Instruction::CreateAsyncState {
            initial_state,
            state_size,
            output_ty,
        })
    }

    /// Build a store async state instruction
    pub fn build_store_async_state(&mut self, state_ptr: ValueId, offset: u32, value: ValueId) {
        self.add_instruction(Instruction::StoreAsyncState {
            state_ptr,
            offset,
            value,
        });
    }

    /// Build a load async state instruction
    pub fn build_load_async_state(
        &mut self,
        state_ptr: ValueId,
        offset: u32,
        ty: Type,
    ) -> Option<ValueId> {
        self.add_instruction(Instruction::LoadAsyncState {
            state_ptr,
            offset,
            ty,
        })
    }

    /// Build a get async state instruction
    pub fn build_get_async_state(&mut self, state_ptr: ValueId) -> Option<ValueId> {
        self.add_instruction(Instruction::GetAsyncState { state_ptr })
    }

    /// Build a set async state instruction
    pub fn build_set_async_state(&mut self, state_ptr: ValueId, new_state: u32) -> Option<ValueId> {
        self.add_instruction(Instruction::SetAsyncState { state_ptr, new_state })
    }

    /// Build an error propagation instruction (? operator)
    pub fn build_error_propagation(
        &mut self,
        value: ValueId,
        value_type: Type,
        success_type: Type,
    ) -> Option<ValueId> {
        self.add_instruction(Instruction::ErrorPropagation {
            value,
            value_type,
            success_type,
        })
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
        let _func_id = builder.create_function("test".to_string(), vec![], Type::I32);

        // Build a constant
        let const_val = builder.const_value(Constant::I32(42));

        // Return the constant
        builder.build_return(Some(const_val));

        let module = builder.build();
        assert_eq!(module.functions().len(), 1);
    }
}
