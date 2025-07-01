use super::{Instruction, ValueId};
use std::fmt;

/// Basic block identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

/// Basic block in the control flow graph
#[derive(Debug, Clone)]
pub struct BasicBlock {
    /// Block identifier
    pub id: BlockId,
    /// Block name (for debugging)
    pub name: String,
    /// Instructions in this block (ValueId, Instruction pairs)
    pub instructions: Vec<(ValueId, Instruction)>,
    /// Predecessor blocks
    pub predecessors: Vec<BlockId>,
    /// Successor blocks (computed from terminator)
    pub successors: Vec<BlockId>,
}

impl BasicBlock {
    /// Create a new basic block
    pub fn new(id: BlockId, name: String) -> Self {
        BasicBlock {
            id,
            name,
            instructions: Vec::new(),
            predecessors: Vec::new(),
            successors: Vec::new(),
        }
    }
    
    /// Add an instruction to this block
    pub fn add_instruction(&mut self, value_id: ValueId, inst: Instruction) {
        // Update successors if this is a terminator
        if inst.is_terminator() {
            self.successors.clear();
            match &inst {
                Instruction::Branch(target) => {
                    self.successors.push(*target);
                }
                Instruction::CondBranch { then_block, else_block, .. } => {
                    self.successors.push(*then_block);
                    self.successors.push(*else_block);
                }
                _ => {}
            }
        }
        
        self.instructions.push((value_id, inst));
    }
    
    /// Get the terminator instruction of this block
    pub fn terminator(&self) -> Option<&Instruction> {
        self.instructions.last().map(|(_, inst)| inst)
            .filter(|inst| inst.is_terminator())
    }
    
    /// Check if this block has a terminator
    pub fn has_terminator(&self) -> bool {
        self.terminator().is_some()
    }
    
    /// Add a predecessor
    pub fn add_predecessor(&mut self, pred: BlockId) {
        if !self.predecessors.contains(&pred) {
            self.predecessors.push(pred);
        }
    }
    
    /// Remove a predecessor
    pub fn remove_predecessor(&mut self, pred: BlockId) {
        self.predecessors.retain(|&p| p != pred);
    }
}

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

impl fmt::Display for BasicBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}:  ; {}", self.id, self.name)?;
        
        for (value_id, inst) in &self.instructions {
            if inst.result_type().is_some() {
                writeln!(f, "    {} = {}", value_id, inst)?;
            } else {
                writeln!(f, "    {}", inst)?;
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::instruction::{BinaryOp, Constant};
    use crate::types::Type;
    
    #[test]
    fn test_basic_block_creation() {
        let block = BasicBlock::new(BlockId(0), "entry".to_string());
        assert_eq!(block.id, BlockId(0));
        assert_eq!(block.name, "entry");
        assert!(block.instructions.is_empty());
        assert!(block.predecessors.is_empty());
        assert!(block.successors.is_empty());
    }
    
    #[test]
    fn test_add_instruction() {
        let mut block = BasicBlock::new(BlockId(0), "entry".to_string());
        
        let inst = Instruction::Const(Constant::I32(42));
        block.add_instruction(ValueId(0), inst);
        
        assert_eq!(block.instructions.len(), 1);
        assert!(!block.has_terminator());
    }
    
    #[test]
    fn test_terminator_updates_successors() {
        let mut block = BasicBlock::new(BlockId(0), "entry".to_string());
        
        // Add a branch instruction
        let branch = Instruction::Branch(BlockId(1));
        block.add_instruction(ValueId(0), branch);
        
        assert!(block.has_terminator());
        assert_eq!(block.successors, vec![BlockId(1)]);
        
        // Add a conditional branch (replaces the previous terminator in practice)
        let cond_branch = Instruction::CondBranch {
            condition: ValueId(0),
            then_block: BlockId(2),
            else_block: BlockId(3),
        };
        block.add_instruction(ValueId(1), cond_branch);
        
        assert_eq!(block.successors, vec![BlockId(2), BlockId(3)]);
    }
    
    #[test]
    fn test_predecessors() {
        let mut block = BasicBlock::new(BlockId(1), "bb1".to_string());
        
        block.add_predecessor(BlockId(0));
        assert_eq!(block.predecessors, vec![BlockId(0)]);
        
        // Adding same predecessor again should not duplicate
        block.add_predecessor(BlockId(0));
        assert_eq!(block.predecessors, vec![BlockId(0)]);
        
        block.add_predecessor(BlockId(2));
        assert_eq!(block.predecessors, vec![BlockId(0), BlockId(2)]);
        
        block.remove_predecessor(BlockId(0));
        assert_eq!(block.predecessors, vec![BlockId(2)]);
    }
    
    #[test]
    fn test_block_display() {
        let mut block = BasicBlock::new(BlockId(0), "entry".to_string());
        
        let const_inst = Instruction::Const(Constant::I32(42));
        block.add_instruction(ValueId(0), const_inst);
        
        let add_inst = Instruction::Binary {
            op: BinaryOp::Add,
            lhs: ValueId(0),
            rhs: ValueId(0),
            ty: Type::I32,
        };
        block.add_instruction(ValueId(1), add_inst);
        
        let ret_inst = Instruction::Return(Some(ValueId(1)));
        block.add_instruction(ValueId(2), ret_inst);
        
        let output = block.to_string();
        assert!(output.contains("bb0:  ; entry"));
        assert!(output.contains("%0 = const 42i32"));
        assert!(output.contains("%1 = add i32 %0, %0 : i32"));
        assert!(output.contains("return %1"));
    }
}