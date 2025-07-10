//! Extensions to the function translator for optimization passes
//!
//! This module provides additional functionality to integrate various
//! optimization passes into the translation process.

use super::closure_optimizer::ClosureOptimizer;
use super::translator::FunctionTranslator;
use crate::codegen::CodegenResult;
use crate::ir::{Instruction, ValueId};
use cranelift::prelude::*;

impl<'a> FunctionTranslator<'a> {
    /// Try to optimize an instruction using available optimization passes
    pub fn try_optimize_instruction(
        &mut self,
        instruction: &Instruction,
        value_id: ValueId,
        builder: &mut FunctionBuilder,
        closure_optimizer: &mut ClosureOptimizer,
    ) -> CodegenResult<bool> {
        match instruction {
            Instruction::CreateClosure { .. } => {
                // Try to optimize closure creation
                closure_optimizer.optimize_closure_creation(self, instruction, value_id, builder)
            }
            Instruction::InvokeClosure { .. } => {
                // Try to optimize closure invocation
                closure_optimizer.optimize_closure_invocation(self, instruction, value_id, builder)
            }
            _ => Ok(false), // No optimization for other instructions
        }
    }
}
