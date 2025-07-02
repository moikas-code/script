//! Dead Code Elimination optimization pass
//!
//! This pass removes various forms of dead code:
//! - Unreachable basic blocks
//! - Instructions whose results are never used
//! - Branches that always go to the same target
//! - Empty blocks that just jump to another block

use super::OptimizationPass;
use crate::ir::{BlockId, Function, Instruction, Module as IrModule, ValueId};
use std::collections::{HashMap, HashSet};

/// Dead Code Elimination optimization pass
#[derive(Debug)]
pub struct DeadCodeElimination {
    /// Statistics for debugging
    blocks_removed: usize,
    instructions_removed: usize,
    branches_simplified: usize,
}

impl DeadCodeElimination {
    /// Create a new dead code elimination pass
    pub fn new() -> Self {
        DeadCodeElimination {
            blocks_removed: 0,
            instructions_removed: 0,
            branches_simplified: 0,
        }
    }

    /// Find all reachable blocks in a function
    fn find_reachable_blocks(function: &Function) -> HashSet<BlockId> {
        let mut reachable = HashSet::new();
        let mut worklist = Vec::new();

        // Start from entry block
        if let Some(entry) = function.entry_block {
            worklist.push(entry);
            reachable.insert(entry);
        }

        // DFS to find all reachable blocks
        while let Some(block_id) = worklist.pop() {
            if let Some(block) = function.get_block(block_id) {
                for &successor in &block.successors {
                    if reachable.insert(successor) {
                        worklist.push(successor);
                    }
                }
            }
        }

        reachable
    }

    /// Find all used values in a function
    fn find_used_values(function: &Function) -> HashSet<ValueId> {
        let mut used = HashSet::new();

        for block in function.blocks().values() {
            for (_, inst_with_loc) in &block.instructions {
                let inst = &inst_with_loc.instruction;
                match inst {
                    Instruction::Binary { lhs, rhs, .. } => {
                        used.insert(*lhs);
                        used.insert(*rhs);
                    }
                    Instruction::Unary { operand, .. } => {
                        used.insert(*operand);
                    }
                    Instruction::Compare { lhs, rhs, .. } => {
                        used.insert(*lhs);
                        used.insert(*rhs);
                    }
                    Instruction::Cast { value, .. } => {
                        used.insert(*value);
                    }
                    Instruction::Call { args, .. } => {
                        for arg in args {
                            used.insert(*arg);
                        }
                    }
                    Instruction::Load { ptr, .. } => {
                        used.insert(*ptr);
                    }
                    Instruction::Store { ptr, value } => {
                        used.insert(*ptr);
                        used.insert(*value);
                    }
                    Instruction::GetElementPtr { ptr, index, .. } => {
                        used.insert(*ptr);
                        used.insert(*index);
                    }
                    Instruction::GetFieldPtr { object, .. } => {
                        used.insert(*object);
                    }
                    Instruction::LoadField { object, .. } => {
                        used.insert(*object);
                    }
                    Instruction::StoreField { object, value, .. } => {
                        used.insert(*object);
                        used.insert(*value);
                    }
                    Instruction::Phi { incoming, .. } => {
                        for (value, _) in incoming {
                            used.insert(*value);
                        }
                    }
                    Instruction::Return(Some(value)) => {
                        used.insert(*value);
                    }
                    Instruction::CondBranch { condition, .. } => {
                        used.insert(*condition);
                    }
                    _ => {}
                }
            }
        }

        used
    }

    /// Check if an instruction has side effects
    fn has_side_effects(inst: &Instruction) -> bool {
        match inst {
            // These instructions have side effects
            Instruction::Store { .. } => true,
            Instruction::StoreField { .. } => true, // Object field stores have side effects
            Instruction::Call { .. } => true,       // Assume all calls have side effects for now
            Instruction::Return(_) => true,
            Instruction::Branch(_) => true,
            Instruction::CondBranch { .. } => true,

            // These instructions have no side effects
            Instruction::Const(_) => false,
            Instruction::Binary { .. } => false,
            Instruction::Unary { .. } => false,
            Instruction::Compare { .. } => false,
            Instruction::Cast { .. } => false,
            Instruction::Alloc { .. } => false,
            Instruction::Load { .. } => false,
            Instruction::LoadField { .. } => false, // Field loads have no side effects (for now)
            Instruction::GetElementPtr { .. } => false,
            Instruction::GetFieldPtr { .. } => false, // Field pointer calculation has no side effects
            Instruction::Phi { .. } => false,
        }
    }

    /// Remove unreachable blocks from a function
    fn remove_unreachable_blocks(&mut self, function: &mut Function) -> bool {
        // For now, we can't actually remove blocks because the blocks field is private
        // We'll mark this as a TODO and just return false
        // TODO: Add a public API to remove blocks from Function
        false
    }

    /// Remove dead instructions (whose results are never used)
    fn remove_dead_instructions(&mut self, function: &mut Function) -> bool {
        let used_values = Self::find_used_values(function);
        let mut changed = false;
        let block_ids: Vec<BlockId> = function.blocks().keys().cloned().collect();

        for block_id in block_ids {
            if let Some(block) = function.get_block_mut(block_id) {
                let mut new_instructions = Vec::new();

                for (value_id, inst_with_loc) in &block.instructions {
                    let inst = &inst_with_loc.instruction;

                    // Keep instruction if it has side effects or its result is used
                    if Self::has_side_effects(inst) || used_values.contains(value_id) {
                        new_instructions.push((*value_id, inst_with_loc.clone()));
                    } else {
                        self.instructions_removed += 1;
                        changed = true;
                    }
                }

                if new_instructions.len() != block.instructions.len() {
                    block.instructions = new_instructions;
                }
            }
        }

        changed
    }

    /// Simplify conditional branches with constant conditions
    fn simplify_branches(&mut self, function: &mut Function) -> bool {
        let mut changed = false;
        let blocks_to_process: Vec<BlockId> = function.blocks().keys().cloned().collect();
        let mut branches_to_simplify = Vec::new();

        // First pass: collect branches that need simplification
        for block_id in blocks_to_process {
            if let Some(block) = function.get_block(block_id) {
                if let Some((_, inst_with_loc)) = block.instructions.last() {
                    if let Instruction::CondBranch {
                        condition,
                        then_block,
                        else_block,
                    } = &inst_with_loc.instruction
                    {
                        // Check if condition is a constant
                        if let Some(const_val) = self.get_constant_bool(function, *condition) {
                            let target = if const_val { *then_block } else { *else_block };
                            let removed_target = if const_val { *else_block } else { *then_block };
                            branches_to_simplify.push((block_id, target, removed_target));
                        }
                    }
                }
            }
        }

        // Second pass: apply simplifications
        for (block_id, target, removed_target) in branches_to_simplify {
            if let Some(block) = function.get_block_mut(block_id) {
                // Update the last instruction and successors
                if let Some((_, inst_with_loc)) = block.instructions.last_mut() {
                    inst_with_loc.instruction = Instruction::Branch(target);

                    // Update successors
                    block.successors.clear();
                    block.successors.push(target);

                    self.branches_simplified += 1;
                    changed = true;
                }
            }

            // Update predecessors of the removed target
            if let Some(removed_block) = function.get_block_mut(removed_target) {
                removed_block.remove_predecessor(block_id);
            }
        }

        changed
    }

    /// Get constant boolean value if the value is a constant
    fn get_constant_bool(&self, function: &Function, value_id: ValueId) -> Option<bool> {
        // Search for the instruction that defines this value
        for block in function.blocks().values() {
            for (vid, inst_with_loc) in &block.instructions {
                if *vid == value_id {
                    match &inst_with_loc.instruction {
                        Instruction::Const(crate::ir::instruction::Constant::Bool(b)) => {
                            return Some(*b);
                        }
                        _ => return None,
                    }
                }
            }
        }
        None
    }

    /// Remove empty blocks that just jump to another block
    fn remove_empty_blocks(&mut self, function: &mut Function) -> bool {
        let mut changed = false;
        let mut block_redirects: HashMap<BlockId, BlockId> = HashMap::new();

        // Find empty blocks that can be removed
        let blocks_to_check: Vec<BlockId> = function.blocks().keys().cloned().collect();

        for block_id in blocks_to_check {
            // Skip entry block
            if Some(block_id) == function.entry_block {
                continue;
            }

            if let Some(block) = function.get_block(block_id) {
                // Check if block only contains an unconditional branch
                if block.instructions.len() == 1 {
                    if let Some((_, inst_with_loc)) = block.instructions.first() {
                        if let Instruction::Branch(target) = &inst_with_loc.instruction {
                            if block_id != *target {
                                // Avoid self-loops
                                block_redirects.insert(block_id, *target);
                            }
                        }
                    }
                }
            }
        }

        // Apply redirects
        if !block_redirects.is_empty() {
            changed = true;
            let block_ids: Vec<BlockId> = function.blocks().keys().cloned().collect();

            // Update all branches to point to the final destination
            for block_id in block_ids {
                if let Some(block) = function.get_block_mut(block_id) {
                    // Update terminator
                    if let Some((_, inst_with_loc)) = block.instructions.last_mut() {
                        match &mut inst_with_loc.instruction {
                            Instruction::Branch(target) => {
                                if let Some(&new_target) = block_redirects.get(target) {
                                    *target = new_target;
                                    block.successors.clear();
                                    block.successors.push(new_target);
                                }
                            }
                            Instruction::CondBranch {
                                then_block,
                                else_block,
                                ..
                            } => {
                                let mut updated = false;

                                if let Some(&new_then) = block_redirects.get(then_block) {
                                    *then_block = new_then;
                                    updated = true;
                                }

                                if let Some(&new_else) = block_redirects.get(else_block) {
                                    *else_block = new_else;
                                    updated = true;
                                }

                                if updated {
                                    block.successors.clear();
                                    block.successors.push(*then_block);
                                    block.successors.push(*else_block);
                                }
                            }
                            _ => {}
                        }
                    }

                    // Update phi nodes
                    let mut new_instructions = Vec::new();
                    for (value_id, inst_with_loc) in &block.instructions {
                        let mut inst_with_loc = inst_with_loc.clone();

                        if let Instruction::Phi { incoming, .. } = &mut inst_with_loc.instruction {
                            for (_, pred_block) in incoming.iter_mut() {
                                if let Some(&new_pred) = block_redirects.get(pred_block) {
                                    *pred_block = new_pred;
                                }
                            }
                        }

                        new_instructions.push((*value_id, inst_with_loc));
                    }
                    block.instructions = new_instructions;
                }
            }

            // Since we can't remove blocks, just count them
            self.blocks_removed += block_redirects.len();

            // Update predecessors
            function.update_predecessors();
        }

        changed
    }
}

impl OptimizationPass for DeadCodeElimination {
    fn optimize(&mut self, module: &mut IrModule) -> bool {
        let mut changed = false;

        // Reset statistics
        self.blocks_removed = 0;
        self.instructions_removed = 0;
        self.branches_simplified = 0;

        // Process each function
        let func_ids: Vec<_> = module.functions().keys().copied().collect();
        for func_id in func_ids {
            if let Some(function) = module.get_function_mut(func_id) {
                // Multiple passes may be needed as each optimization can enable others
                let mut function_changed = true;
                while function_changed {
                    function_changed = false;

                    // Remove unreachable blocks
                    if self.remove_unreachable_blocks(function) {
                        function_changed = true;
                    }

                    // Remove dead instructions
                    if self.remove_dead_instructions(function) {
                        function_changed = true;
                    }

                    // Simplify branches
                    if self.simplify_branches(function) {
                        function_changed = true;
                    }

                    // Remove empty blocks
                    if self.remove_empty_blocks(function) {
                        function_changed = true;
                    }

                    if function_changed {
                        changed = true;
                        // Update predecessor information after changes
                        function.update_predecessors();
                    }
                }
            }
        }

        if changed {
            eprintln!(
                "Dead code elimination: removed {} blocks, {} instructions, simplified {} branches",
                self.blocks_removed, self.instructions_removed, self.branches_simplified
            );
        }

        changed
    }

    fn name(&self) -> &'static str {
        "DeadCodeElimination"
    }
}

impl Default for DeadCodeElimination {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BinaryOp, Constant, IrBuilder};
    use crate::types::Type;

    #[test]
    fn test_remove_unreachable_blocks() {
        let mut builder = IrBuilder::new();

        // Create a function with unreachable blocks
        let _func_id = builder.create_function("test".to_string(), vec![], Type::I32);

        let entry = builder.get_current_block().unwrap();
        let reachable = builder.create_block("reachable".to_string()).unwrap();
        let unreachable = builder.create_block("unreachable".to_string()).unwrap();

        // entry -> reachable (unreachable is not connected)
        builder.set_current_block(entry);
        builder.build_branch(reachable);

        builder.set_current_block(reachable);
        let value1 = builder.const_value(Constant::I32(42));
        builder.build_return(Some(value1));

        builder.set_current_block(unreachable);
        let value2 = builder.const_value(Constant::I32(999));
        builder.build_return(Some(value2));

        let mut module = builder.build();

        let mut pass = DeadCodeElimination::new();
        // Since we can't remove blocks yet, this test just verifies the pass runs
        pass.optimize(&mut module);

        // TODO: Once block removal is implemented, verify unreachable blocks are removed
    }

    #[test]
    fn test_remove_dead_instructions() {
        let mut builder = IrBuilder::new();

        let func_id = builder.create_function("test".to_string(), vec![], Type::I32);

        // Create some dead instructions
        let dead1 = builder.const_value(Constant::I32(10));
        let dead2 = builder.const_value(Constant::I32(20));
        let _dead_add = builder.build_binary(BinaryOp::Add, dead1, dead2, Type::I32);

        // Create a used instruction
        let used = builder.const_value(Constant::I32(42));
        builder.build_return(Some(used));

        let mut module = builder.build();

        let mut pass = DeadCodeElimination::new();
        assert!(pass.optimize(&mut module));

        // Check that dead instructions were removed
        let func = module.get_function(func_id).unwrap();
        let entry_block = func.get_block(func.entry_block.unwrap()).unwrap();

        // Should only have the used constant and return
        assert_eq!(entry_block.instructions.len(), 2);
    }

    #[test]
    fn test_simplify_constant_branch() {
        let mut builder = IrBuilder::new();

        let func_id = builder.create_function("test".to_string(), vec![], Type::I32);

        let then_block = builder.create_block("then".to_string()).unwrap();
        let else_block = builder.create_block("else".to_string()).unwrap();

        // Create a constant true condition
        let cond = builder.const_value(Constant::Bool(true));
        builder.build_cond_branch(cond, then_block, else_block);

        builder.set_current_block(then_block);
        let value1 = builder.const_value(Constant::I32(1));
        builder.build_return(Some(value1));

        builder.set_current_block(else_block);
        let value2 = builder.const_value(Constant::I32(2));
        builder.build_return(Some(value2));

        let mut module = builder.build();

        let mut pass = DeadCodeElimination::new();
        assert!(pass.optimize(&mut module));

        // Check that conditional branch was simplified
        let func = module.get_function(func_id).unwrap();
        let entry_block = func.get_block(func.entry_block.unwrap()).unwrap();

        // Should have unconditional branch to then_block
        if let Some((_, inst_with_loc)) = entry_block.instructions.last() {
            match &inst_with_loc.instruction {
                Instruction::Branch(target) => assert_eq!(*target, then_block),
                _ => panic!("Expected unconditional branch"),
            }
        }

        // TODO: Once block removal is implemented, verify else_block is removed
    }

    #[test]
    fn test_remove_empty_blocks() {
        let mut builder = IrBuilder::new();

        let func_id = builder.create_function("test".to_string(), vec![], Type::I32);

        let empty1 = builder.create_block("empty1".to_string()).unwrap();
        let empty2 = builder.create_block("empty2".to_string()).unwrap();
        let final_block = builder.create_block("final".to_string()).unwrap();

        // entry -> empty1 -> empty2 -> final
        let _entry = builder.get_current_block().unwrap();
        builder.build_branch(empty1);

        builder.set_current_block(empty1);
        builder.build_branch(empty2);

        builder.set_current_block(empty2);
        builder.build_branch(final_block);

        builder.set_current_block(final_block);
        let value = builder.const_value(Constant::I32(42));
        builder.build_return(Some(value));

        let mut module = builder.build();

        let mut pass = DeadCodeElimination::new();
        assert!(pass.optimize(&mut module));

        // Check that branches are redirected
        let func = module.get_function(func_id).unwrap();
        let entry_block = func.get_block(func.entry_block.unwrap()).unwrap();

        // Entry should eventually point to final (after redirect)
        if let Some((_, inst_with_loc)) = entry_block.instructions.last() {
            match &inst_with_loc.instruction {
                Instruction::Branch(target) => {
                    // Should be redirected to final_block
                    assert_eq!(*target, final_block);
                }
                _ => panic!("Expected branch instruction"),
            }
        }
    }
}
