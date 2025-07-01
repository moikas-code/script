use cranelift::prelude::*;
use cranelift_module::Module;
use cranelift::codegen::ir::Function;

use crate::ir::{Function as IrFunction, BasicBlock, Instruction, Constant, ValueId, BlockId};
use crate::ir::{BinaryOp, UnaryOp, ComparisonOp};
use crate::error::{Error, ErrorKind};

use super::{CodegenResult, script_type_to_cranelift};
use std::collections::HashMap;

/// Translates IR functions to Cranelift IR
pub struct FunctionTranslator<'a> {
    /// Module for looking up functions
    #[allow(dead_code)]
    module: &'a dyn Module,
    /// Value mapping from IR to Cranelift
    values: HashMap<ValueId, Value>,
    /// Block mapping from IR to Cranelift
    blocks: HashMap<BlockId, Block>,
    /// Variable counter for SSA construction
    #[allow(dead_code)]
    var_counter: usize,
}

impl<'a> FunctionTranslator<'a> {
    /// Create a new function translator
    pub fn new(module: &'a dyn Module) -> Self {
        FunctionTranslator {
            module,
            values: HashMap::new(),
            blocks: HashMap::new(),
            var_counter: 0,
        }
    }
    
    /// Translate an IR function to Cranelift IR
    pub fn translate_function(&mut self, ir_func: &IrFunction, cranelift_func: &mut Function) -> CodegenResult<()> {
        let mut fn_builder_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(cranelift_func, &mut fn_builder_ctx);
        
        // Create entry block
        let entry_block = builder.create_block();
        
        // Add block parameters for function parameters
        for (i, param) in ir_func.params.iter().enumerate() {
            let ty = script_type_to_cranelift(&param.ty);
            builder.append_block_param(entry_block, ty);
            
            // Map parameter to value
            let _param_val = builder.block_params(entry_block)[i];
            // TODO: Create proper parameter ValueId mapping
        }
        
        // Switch to entry block
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);
        
        // First pass: create all blocks
        for (block_id, _ir_block) in ir_func.blocks() {
            if Some(*block_id) != ir_func.entry_block {
                let cranelift_block = builder.create_block();
                self.blocks.insert(*block_id, cranelift_block);
            } else {
                self.blocks.insert(*block_id, entry_block);
            }
        }
        
        // Second pass: translate blocks
        for ir_block in ir_func.blocks_in_order() {
            self.translate_block(ir_block, &mut builder)?;
        }
        
        // Finalize the function
        builder.finalize();
        
        Ok(())
    }
    
    /// Translate a basic block
    fn translate_block(&mut self, block: &BasicBlock, builder: &mut FunctionBuilder) -> CodegenResult<()> {
        let cranelift_block = *self.blocks.get(&block.id)
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Block not found"))?;
        
        // Switch to this block
        builder.switch_to_block(cranelift_block);
        
        // Translate instructions
        for (value_id, inst) in &block.instructions {
            self.translate_instruction(*value_id, inst, builder)?;
        }
        
        // Seal the block if all predecessors have been processed
        // In a real implementation, this would track predecessor processing
        builder.seal_block(cranelift_block);
        
        Ok(())
    }
    
    /// Translate an instruction
    fn translate_instruction(&mut self, value_id: ValueId, inst: &Instruction, builder: &mut FunctionBuilder) -> CodegenResult<()> {
        match inst {
            Instruction::Const(constant) => {
                let val = self.translate_constant(constant, builder)?;
                self.values.insert(value_id, val);
            }
            
            Instruction::Binary { op, lhs, rhs, ty: _ } => {
                let lhs_val = self.get_value(*lhs)?;
                let rhs_val = self.get_value(*rhs)?;
                let result = self.translate_binary_op(*op, lhs_val, rhs_val, builder)?;
                self.values.insert(value_id, result);
            }
            
            Instruction::Unary { op, operand, ty: _ } => {
                let operand_val = self.get_value(*operand)?;
                let result = self.translate_unary_op(*op, operand_val, builder)?;
                self.values.insert(value_id, result);
            }
            
            Instruction::Compare { op, lhs, rhs } => {
                let lhs_val = self.get_value(*lhs)?;
                let rhs_val = self.get_value(*rhs)?;
                let result = self.translate_comparison(*op, lhs_val, rhs_val, builder)?;
                self.values.insert(value_id, result);
            }
            
            Instruction::Return(value) => {
                if let Some(val_id) = value {
                    let val = self.get_value(*val_id)?;
                    builder.ins().return_(&[val]);
                } else {
                    builder.ins().return_(&[]);
                }
            }
            
            Instruction::Branch(target) => {
                let target_block = self.get_block(*target)?;
                builder.ins().jump(target_block, &[]);
            }
            
            Instruction::CondBranch { condition, then_block, else_block } => {
                let cond_val = self.get_value(*condition)?;
                let then_blk = self.get_block(*then_block)?;
                let else_blk = self.get_block(*else_block)?;
                builder.ins().brif(cond_val, then_blk, &[], else_blk, &[]);
            }
            
            _ => {
                // TODO: Implement remaining instructions
                return Err(Error::new(ErrorKind::RuntimeError, 
                    format!("Instruction translation not implemented: {:?}", inst)));
            }
        }
        
        Ok(())
    }
    
    /// Translate a constant
    fn translate_constant(&mut self, constant: &Constant, builder: &mut FunctionBuilder) -> CodegenResult<Value> {
        match constant {
            Constant::I32(n) => Ok(builder.ins().iconst(types::I32, *n as i64)),
            Constant::F32(f) => Ok(builder.ins().f32const(*f)),
            Constant::Bool(b) => Ok(builder.ins().iconst(types::I8, if *b { 1 } else { 0 })),
            Constant::String(_) => {
                // TODO: Implement string constants
                Err(Error::new(ErrorKind::RuntimeError, "String constants not yet implemented"))
            }
            Constant::Null => Ok(builder.ins().iconst(types::I64, 0)),
        }
    }
    
    /// Translate a binary operation
    fn translate_binary_op(&mut self, op: BinaryOp, lhs: Value, rhs: Value, builder: &mut FunctionBuilder) -> CodegenResult<Value> {
        Ok(match op {
            BinaryOp::Add => builder.ins().iadd(lhs, rhs),
            BinaryOp::Sub => builder.ins().isub(lhs, rhs),
            BinaryOp::Mul => builder.ins().imul(lhs, rhs),
            BinaryOp::Div => builder.ins().sdiv(lhs, rhs),
            BinaryOp::Mod => builder.ins().srem(lhs, rhs),
            BinaryOp::And => builder.ins().band(lhs, rhs),
            BinaryOp::Or => builder.ins().bor(lhs, rhs),
        })
    }
    
    /// Translate a unary operation
    fn translate_unary_op(&mut self, op: UnaryOp, operand: Value, builder: &mut FunctionBuilder) -> CodegenResult<Value> {
        Ok(match op {
            UnaryOp::Neg => builder.ins().ineg(operand),
            UnaryOp::Not => {
                // For boolean not, compare with 0
                let zero = builder.ins().iconst(types::I8, 0);
                builder.ins().icmp(IntCC::Equal, operand, zero)
            }
        })
    }
    
    /// Translate a comparison operation
    fn translate_comparison(&mut self, op: ComparisonOp, lhs: Value, rhs: Value, builder: &mut FunctionBuilder) -> CodegenResult<Value> {
        let cc = match op {
            ComparisonOp::Eq => IntCC::Equal,
            ComparisonOp::Ne => IntCC::NotEqual,
            ComparisonOp::Lt => IntCC::SignedLessThan,
            ComparisonOp::Le => IntCC::SignedLessThanOrEqual,
            ComparisonOp::Gt => IntCC::SignedGreaterThan,
            ComparisonOp::Ge => IntCC::SignedGreaterThanOrEqual,
        };
        
        Ok(builder.ins().icmp(cc, lhs, rhs))
    }
    
    /// Get a value by ID
    fn get_value(&self, id: ValueId) -> CodegenResult<Value> {
        self.values.get(&id)
            .copied()
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, format!("Value {:?} not found", id)))
    }
    
    /// Get a block by ID
    fn get_block(&self, id: BlockId) -> CodegenResult<Block> {
        self.blocks.get(&id)
            .copied()
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, format!("Block {:?} not found", id)))
    }
}