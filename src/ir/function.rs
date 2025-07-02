use super::{BasicBlock, BlockId};
use crate::types::Type;
use std::collections::HashMap;
use std::fmt;

/// Function identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FunctionId(pub u32);

/// Function parameter
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
}

/// Function in the IR
#[derive(Debug, Clone)]
pub struct Function {
    /// Function identifier
    pub id: FunctionId,
    /// Function name
    pub name: String,
    /// Function parameters
    pub params: Vec<Parameter>,
    /// Return type
    pub return_type: Type,
    /// Basic blocks in this function
    blocks: HashMap<BlockId, BasicBlock>,
    /// Entry block
    pub entry_block: Option<BlockId>,
    /// Next block ID to allocate
    next_block_id: u32,
}

impl Function {
    /// Create a new function
    pub fn new(id: FunctionId, name: String, params: Vec<Parameter>, return_type: Type) -> Self {
        Function {
            id,
            name,
            params,
            return_type,
            blocks: HashMap::new(),
            entry_block: None,
            next_block_id: 0,
        }
    }

    /// Create a new basic block in this function
    pub fn create_block(&mut self, name: String) -> BlockId {
        let block_id = BlockId(self.next_block_id);
        self.next_block_id += 1;

        let block = BasicBlock::new(block_id, name);
        self.blocks.insert(block_id, block);

        // First block is the entry block
        if self.entry_block.is_none() {
            self.entry_block = Some(block_id);
        }

        block_id
    }

    /// Get a basic block by ID
    pub fn get_block(&self, id: BlockId) -> Option<&BasicBlock> {
        self.blocks.get(&id)
    }

    /// Get a mutable reference to a basic block
    pub fn get_block_mut(&mut self, id: BlockId) -> Option<&mut BasicBlock> {
        self.blocks.get_mut(&id)
    }

    /// Get all blocks in this function
    pub fn blocks(&self) -> &HashMap<BlockId, BasicBlock> {
        &self.blocks
    }

    /// Get all blocks in execution order (entry block first)
    pub fn blocks_in_order(&self) -> Vec<&BasicBlock> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();

        if let Some(entry) = self.entry_block {
            self.visit_block_dfs(entry, &mut visited, &mut result);
        }

        // Add any unreachable blocks at the end
        for block in self.blocks.values() {
            if !visited.contains(&block.id) {
                result.push(block);
            }
        }

        result
    }

    /// DFS helper for blocks_in_order
    fn visit_block_dfs<'a>(
        &'a self,
        block_id: BlockId,
        visited: &mut std::collections::HashSet<BlockId>,
        result: &mut Vec<&'a BasicBlock>,
    ) {
        if visited.contains(&block_id) {
            return;
        }

        visited.insert(block_id);

        if let Some(block) = self.get_block(block_id) {
            result.push(block);

            for &successor in &block.successors {
                self.visit_block_dfs(successor, visited, result);
            }
        }
    }

    /// Update predecessor information for all blocks
    pub fn update_predecessors(&mut self) {
        // Clear all predecessors first
        for block in self.blocks.values_mut() {
            block.predecessors.clear();
        }

        // Rebuild predecessor information
        let edges: Vec<(BlockId, Vec<BlockId>)> = self
            .blocks
            .values()
            .map(|block| (block.id, block.successors.clone()))
            .collect();

        for (pred_id, successors) in edges {
            for succ_id in successors {
                if let Some(succ_block) = self.blocks.get_mut(&succ_id) {
                    succ_block.add_predecessor(pred_id);
                }
            }
        }
    }

    /// Get the signature of this function as a type
    pub fn signature_type(&self) -> Type {
        Type::Function {
            params: self.params.iter().map(|p| p.ty.clone()).collect(),
            ret: Box::new(self.return_type.clone()),
        }
    }
}

impl fmt::Display for FunctionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", self.0)
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Function signature
        write!(f, "fn {} {}(", self.id, self.name)?;

        for (i, param) in self.params.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "%arg{}: {}", i, param.ty)?;
        }

        writeln!(f, ") -> {} {{", self.return_type)?;

        // Function body (blocks in order)
        for block in self.blocks_in_order() {
            write!(f, "{}", block)?;
            writeln!(f)?;
        }

        writeln!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::instruction::{BinaryOp, Instruction};
    use crate::ir::ValueId;

    #[test]
    fn test_function_creation() {
        let params = vec![
            Parameter {
                name: "x".to_string(),
                ty: Type::I32,
            },
            Parameter {
                name: "y".to_string(),
                ty: Type::I32,
            },
        ];

        let func = Function::new(FunctionId(0), "add".to_string(), params, Type::I32);

        assert_eq!(func.name, "add");
        assert_eq!(func.params.len(), 2);
        assert_eq!(func.return_type, Type::I32);
        assert!(func.blocks.is_empty());
        assert!(func.entry_block.is_none());
    }

    #[test]
    fn test_create_blocks() {
        let mut func = Function::new(FunctionId(0), "test".to_string(), vec![], Type::I32);

        let entry = func.create_block("entry".to_string());
        assert_eq!(func.entry_block, Some(entry));

        let bb1 = func.create_block("bb1".to_string());
        let bb2 = func.create_block("bb2".to_string());

        assert_eq!(func.blocks.len(), 3);
        assert_ne!(entry, bb1);
        assert_ne!(bb1, bb2);
    }

    #[test]
    fn test_update_predecessors() {
        let mut func = Function::new(FunctionId(0), "test".to_string(), vec![], Type::I32);

        let entry = func.create_block("entry".to_string());
        let bb1 = func.create_block("bb1".to_string());
        let bb2 = func.create_block("bb2".to_string());

        // Create control flow: entry -> bb1, bb1 -> bb2
        if let Some(entry_block) = func.get_block_mut(entry) {
            entry_block.add_instruction(ValueId(0), Instruction::Branch(bb1));
        }

        if let Some(bb1_block) = func.get_block_mut(bb1) {
            bb1_block.add_instruction(ValueId(1), Instruction::Branch(bb2));
        }

        func.update_predecessors();

        // Check predecessors
        assert!(func.get_block(entry).unwrap().predecessors.is_empty());
        assert_eq!(func.get_block(bb1).unwrap().predecessors, vec![entry]);
        assert_eq!(func.get_block(bb2).unwrap().predecessors, vec![bb1]);
    }

    #[test]
    fn test_blocks_in_order() {
        let mut func = Function::new(FunctionId(0), "test".to_string(), vec![], Type::I32);

        let entry = func.create_block("entry".to_string());
        let bb1 = func.create_block("bb1".to_string());
        let bb2 = func.create_block("bb2".to_string());
        let unreachable = func.create_block("unreachable".to_string());

        // Create control flow: entry -> bb1 -> bb2, unreachable is not connected
        if let Some(entry_block) = func.get_block_mut(entry) {
            entry_block.add_instruction(ValueId(0), Instruction::Branch(bb1));
        }

        if let Some(bb1_block) = func.get_block_mut(bb1) {
            bb1_block.add_instruction(ValueId(1), Instruction::Branch(bb2));
        }

        let blocks_ordered = func.blocks_in_order();
        assert_eq!(blocks_ordered.len(), 4);

        // Entry should be first
        assert_eq!(blocks_ordered[0].id, entry);

        // Reachable blocks should come before unreachable ones
        let unreachable_pos = blocks_ordered
            .iter()
            .position(|b| b.id == unreachable)
            .unwrap();
        assert!(unreachable_pos > 0);
    }

    #[test]
    fn test_function_display() {
        let params = vec![
            Parameter {
                name: "x".to_string(),
                ty: Type::I32,
            },
            Parameter {
                name: "y".to_string(),
                ty: Type::I32,
            },
        ];

        let mut func = Function::new(FunctionId(0), "add".to_string(), params, Type::I32);

        let entry = func.create_block("entry".to_string());

        if let Some(entry_block) = func.get_block_mut(entry) {
            // %0 = x + y
            entry_block.add_instruction(
                ValueId(0),
                Instruction::Binary {
                    op: BinaryOp::Add,
                    lhs: ValueId(100), // Would be parameter refs in real code
                    rhs: ValueId(101),
                    ty: Type::I32,
                },
            );

            // return %0
            entry_block.add_instruction(ValueId(1), Instruction::Return(Some(ValueId(0))));
        }

        let output = func.to_string();
        assert!(output.contains("fn @0 add(%arg0: i32, %arg1: i32) -> i32"));
        assert!(output.contains("bb0:  ; entry"));
        assert!(output.contains("%0 = add i32"));
        assert!(output.contains("return %0"));
    }
}
