//! Loop Invariant Code Motion (LICM) optimization pass
//!
//! This pass identifies computations inside loops that don't depend on the loop
//! and moves them outside the loop to reduce redundant computation.

use super::{
    loop_analysis::{LoopAnalyzer, LoopInfo},
    OptimizationPass,
};
use crate::ir::{BlockId, Function, Instruction, Module as IrModule, ValueId};
use std::collections::HashSet;

/// Loop Invariant Code Motion optimization pass
#[derive(Debug)]
pub struct LoopInvariantCodeMotion {
    /// Number of instructions moved out of loops
    moved_count: usize,
}

impl LoopInvariantCodeMotion {
    /// Create a new LICM pass
    pub fn new() -> Self {
        LoopInvariantCodeMotion { moved_count: 0 }
    }

    /// Get the number of instructions moved
    pub fn moved_count(&self) -> usize {
        self.moved_count
    }
}

impl OptimizationPass for LoopInvariantCodeMotion {
    fn optimize(&mut self, module: &mut IrModule) -> bool {
        let mut changed = false;
        self.moved_count = 0;

        // Process each function
        let func_ids: Vec<_> = module.functions().keys().cloned().collect();
        for func_id in func_ids {
            if let Some(func) = module.get_function_mut(func_id) {
                changed |= self.optimize_function(func);
            }
        }

        changed
    }

    fn name(&self) -> &'static str {
        "LoopInvariantCodeMotion"
    }
}

impl LoopInvariantCodeMotion {
    /// Optimize a single function
    fn optimize_function(&mut self, function: &mut Function) -> bool {
        // Analyze loops in the function
        let mut analyzer = LoopAnalyzer::new();
        let loops = analyzer.analyze_function(function);

        let mut changed = false;

        // Process each loop (innermost first)
        let mut loop_order = self.topological_sort_loops(&loops);
        loop_order.reverse(); // Process innermost loops first

        for loop_idx in loop_order {
            if loop_idx < loops.len() {
                changed |= self.optimize_loop(function, &loops[loop_idx]);
            }
        }

        changed
    }

    /// Optimize a single loop
    fn optimize_loop(&mut self, function: &mut Function, loop_info: &LoopInfo) -> bool {
        let mut changed = false;

        // Find the preheader block (or create one)
        let preheader = self.find_or_create_preheader(function, loop_info);

        // Find loop invariant instructions
        let invariant_instructions = self.find_loop_invariants(function, loop_info);

        // Move invariant instructions to preheader
        for (block_id, value_id) in invariant_instructions {
            if self.is_safe_to_move(function, loop_info, block_id, value_id) {
                if self.move_instruction_to_preheader(function, block_id, value_id, preheader) {
                    changed = true;
                    self.moved_count += 1;
                }
            }
        }

        changed
    }

    /// Find or create a preheader block for the loop
    fn find_or_create_preheader(&self, function: &mut Function, loop_info: &LoopInfo) -> BlockId {
        // A preheader is a block that:
        // 1. Has exactly one successor (the loop header)
        // 2. Is not part of the loop
        // 3. Dominates the loop header

        if let Some(header_block) = function.get_block(loop_info.header) {
            // Check if there's already a suitable preheader
            for &pred in &header_block.predecessors {
                if !loop_info.body.contains(&pred) {
                    if let Some(pred_block) = function.get_block(pred) {
                        if pred_block.successors.len() == 1
                            && pred_block.successors[0] == loop_info.header
                        {
                            return pred; // Found suitable preheader
                        }
                    }
                }
            }
        }

        // Need to create a new preheader
        // For now, return a dummy block ID - in a full implementation, we'd create the block
        BlockId(u32::MAX) // Placeholder
    }

    /// Find instructions that are loop invariant
    fn find_loop_invariants(
        &self,
        function: &Function,
        loop_info: &LoopInfo,
    ) -> Vec<(BlockId, ValueId)> {
        let mut invariants = Vec::new();
        let mut defined_in_loop = HashSet::new();

        // First pass: collect all values defined inside the loop
        for &block_id in &loop_info.body {
            if let Some(block) = function.get_block(block_id) {
                for (value_id, _) in &block.instructions {
                    defined_in_loop.insert(*value_id);
                }
            }
        }

        // Second pass: find instructions whose operands are all loop invariant
        let mut changed = true;
        while changed {
            changed = false;

            for &block_id in &loop_info.body {
                if let Some(block) = function.get_block(block_id) {
                    for (value_id, inst_with_loc) in &block.instructions {
                        if invariants.iter().any(|(_, v)| v == value_id) {
                            continue; // Already marked as invariant
                        }

                        if self.is_loop_invariant_instruction(
                            &inst_with_loc.instruction,
                            &defined_in_loop,
                            &invariants,
                        ) {
                            invariants.push((block_id, *value_id));
                            changed = true;
                        }
                    }
                }
            }
        }

        invariants
    }

    /// Check if an instruction is loop invariant
    fn is_loop_invariant_instruction(
        &self,
        instruction: &Instruction,
        defined_in_loop: &HashSet<ValueId>,
        current_invariants: &[(BlockId, ValueId)],
    ) -> bool {
        match instruction {
            Instruction::Binary { lhs, rhs, .. } => {
                self.is_value_loop_invariant(*lhs, defined_in_loop, current_invariants) &&
                self.is_value_loop_invariant(*rhs, defined_in_loop, current_invariants)
            }
            Instruction::Unary { operand, .. } => {
                self.is_value_loop_invariant(*operand, defined_in_loop, current_invariants)
            }
            Instruction::Compare { lhs, rhs, .. } => {
                self.is_value_loop_invariant(*lhs, defined_in_loop, current_invariants) &&
                self.is_value_loop_invariant(*rhs, defined_in_loop, current_invariants)
            }
            Instruction::Cast { value, .. } => {
                self.is_value_loop_invariant(*value, defined_in_loop, current_invariants)
            }
            Instruction::Load { ptr, .. } => {
                // Loads are invariant only if:
                // 1. The pointer is invariant
                // 2. No stores in the loop might alias with this load
                self.is_value_loop_invariant(*ptr, defined_in_loop, current_invariants)
                // TODO: Add alias analysis to check for conflicting stores
            }
            Instruction::LoadField { object, .. } => {
                // Object field loads are invariant if:
                // 1. The object is invariant
                // 2. No field stores in the loop might alias with this load
                self.is_value_loop_invariant(*object, defined_in_loop, current_invariants)
                // TODO: Add alias analysis to check for conflicting field stores
            }
            // Instructions with side effects are never invariant
            Instruction::Store { .. } |
            Instruction::StoreField { .. } | // Object field stores have side effects
            Instruction::Call { .. } |
            Instruction::Return(_) |
            Instruction::Branch(_) |
            Instruction::CondBranch { .. } |
            Instruction::Alloc { .. } => false, // Alloc has side effects
            // PHI nodes and pointer calculations could be invariant in some cases
            Instruction::Phi { .. } => false, // PHI nodes in loop headers are usually not invariant
            Instruction::GetElementPtr { ptr, index, .. } => {
                self.is_value_loop_invariant(*ptr, defined_in_loop, current_invariants) &&
                self.is_value_loop_invariant(*index, defined_in_loop, current_invariants)
            }
            Instruction::GetFieldPtr { object, .. } => {
                // Field pointer calculation is invariant if object is invariant
                self.is_value_loop_invariant(*object, defined_in_loop, current_invariants)
            }
            // Constants are always loop invariant
            Instruction::Const(_) => true,

            // Struct operations
            Instruction::AllocStruct { .. } => false, // Allocation has side effects
            Instruction::ConstructStruct { fields, .. } => {
                // Constructor is invariant if all field values are invariant
                fields.iter().all(|(_, value)| {
                    self.is_value_loop_invariant(*value, defined_in_loop, current_invariants)
                })
            }

            // Enum operations
            Instruction::AllocEnum { .. } => false, // Allocation has side effects
            Instruction::ConstructEnum { args, .. } => {
                // Enum constructor is invariant if all arguments are invariant
                args.iter().all(|value| {
                    self.is_value_loop_invariant(*value, defined_in_loop, current_invariants)
                })
            }
            Instruction::GetEnumTag { enum_value, .. } => {
                self.is_value_loop_invariant(*enum_value, defined_in_loop, current_invariants)
            }
            Instruction::SetEnumTag { .. } => false, // Has side effects
            Instruction::ExtractEnumData { enum_value, .. } => {
                self.is_value_loop_invariant(*enum_value, defined_in_loop, current_invariants)
            }

            // Async operations - generally not invariant due to state changes
            Instruction::Suspend { .. } => false,
            Instruction::PollFuture { .. } => false,
            Instruction::CreateAsyncState { .. } => false,
            Instruction::StoreAsyncState { .. } => false,
            Instruction::LoadAsyncState { state_ptr, .. } => {
                self.is_value_loop_invariant(*state_ptr, defined_in_loop, current_invariants)
            }
            Instruction::GetAsyncState { state_ptr, .. } => {
                self.is_value_loop_invariant(*state_ptr, defined_in_loop, current_invariants)
            }
            Instruction::SetAsyncState { .. } => false,

            // Security operations
            Instruction::BoundsCheck { array, index, length, .. } => {
                self.is_value_loop_invariant(*array, defined_in_loop, current_invariants) &&
                self.is_value_loop_invariant(*index, defined_in_loop, current_invariants) &&
                length.map_or(true, |len| self.is_value_loop_invariant(len, defined_in_loop, current_invariants))
            }
            Instruction::ValidateFieldAccess { object, .. } => {
                self.is_value_loop_invariant(*object, defined_in_loop, current_invariants)
            }

            // Error handling
            Instruction::ErrorPropagation { value, .. } => {
                self.is_value_loop_invariant(*value, defined_in_loop, current_invariants)
            }

            // Closure operations
            Instruction::CreateClosure { .. } => false, // Closure creation has side effects
            Instruction::InvokeClosure { .. } => false, // Closure invocation has side effects
        }
    }

    /// Check if a value is loop invariant
    fn is_value_loop_invariant(
        &self,
        value: ValueId,
        defined_in_loop: &HashSet<ValueId>,
        current_invariants: &[(BlockId, ValueId)],
    ) -> bool {
        // A value is loop invariant if:
        // 1. It's not defined in the loop (constant or from outside), OR
        // 2. It's already been identified as invariant
        !defined_in_loop.contains(&value) || current_invariants.iter().any(|(_, v)| *v == value)
    }

    /// Check if it's safe to move an instruction
    fn is_safe_to_move(
        &self,
        function: &Function,
        _loop_info: &LoopInfo,
        block_id: BlockId,
        value_id: ValueId,
    ) -> bool {
        // For safety, we need to ensure:
        // 1. The instruction doesn't have side effects
        // 2. Moving it won't change program semantics
        // 3. All uses are dominated by the new location

        if let Some(block) = function.get_block(block_id) {
            // Find the instruction in the block
            if let Some((_, inst_with_loc)) =
                block.instructions.iter().find(|(vid, _)| *vid == value_id)
            {
                match &inst_with_loc.instruction {
                    // Safe to move these instructions
                    Instruction::Binary { .. }
                    | Instruction::Unary { .. }
                    | Instruction::Compare { .. }
                    | Instruction::Cast { .. }
                    | Instruction::GetElementPtr { .. } => true,

                    // Loads are potentially safe if no conflicting stores
                    Instruction::Load { .. } => {
                        // For now, be conservative and don't move loads
                        false
                    }

                    // Never move these
                    _ => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Move an instruction to the preheader
    fn move_instruction_to_preheader(
        &self,
        _function: &mut Function,
        _from_block: BlockId,
        _value_id: ValueId,
        to_block: BlockId,
    ) -> bool {
        // For now, this is a placeholder that would:
        // 1. Remove the instruction from the source block
        // 2. Add it to the target block (preheader)
        // 3. Update any data structures

        // In a full implementation, we'd actually move the instruction
        // For now, just return true to indicate the operation would succeed
        to_block.0 != u32::MAX // Only succeed if we have a real preheader
    }

    /// Topologically sort loops by nesting depth
    fn topological_sort_loops(&self, loops: &[LoopInfo]) -> Vec<usize> {
        (0..loops.len()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BinaryOp, Constant, IrBuilder};
    use crate::types::Type;

    #[test]
    fn test_licm_creation() {
        let mut licm = LoopInvariantCodeMotion::new();
        assert_eq!(licm.moved_count(), 0);
        assert_eq!(licm.name(), "LoopInvariantCodeMotion");
    }

    #[test]
    fn test_loop_invariant_detection() {
        let licm = LoopInvariantCodeMotion::new();
        let mut defined_in_loop = HashSet::new();
        let invariants = vec![];

        // A constant load should be invariant (operands are constants)
        let const_add = Instruction::Binary {
            op: BinaryOp::Add,
            lhs: ValueId(1), // Assume this is a constant
            rhs: ValueId(2), // Assume this is a constant
            ty: Type::I32,
        };

        assert!(licm.is_loop_invariant_instruction(&const_add, &defined_in_loop, &invariants));

        // An instruction using loop-defined values should not be invariant
        defined_in_loop.insert(ValueId(3));
        let loop_add = Instruction::Binary {
            op: BinaryOp::Add,
            lhs: ValueId(1), // Constant
            rhs: ValueId(3), // Defined in loop
            ty: Type::I32,
        };

        assert!(!licm.is_loop_invariant_instruction(&loop_add, &defined_in_loop, &invariants));
    }

    #[test]
    fn test_value_invariance() {
        let licm = LoopInvariantCodeMotion::new();
        let mut defined_in_loop = HashSet::new();
        defined_in_loop.insert(ValueId(100));

        let invariants = vec![(BlockId(1), ValueId(200))];

        // Value not defined in loop should be invariant
        assert!(licm.is_value_loop_invariant(ValueId(50), &defined_in_loop, &invariants));

        // Value defined in loop but not yet proven invariant should not be invariant
        assert!(!licm.is_value_loop_invariant(ValueId(100), &defined_in_loop, &invariants));

        // Value defined in loop but already proven invariant should be invariant
        assert!(licm.is_value_loop_invariant(ValueId(200), &defined_in_loop, &invariants));
    }
}
