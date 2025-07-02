//! Common Subexpression Elimination (CSE) optimization pass
//!
//! This pass identifies and eliminates redundant computations within basic blocks.
//! When the same expression is computed multiple times with the same operands,
//! later occurrences are replaced with references to the first computation.

use super::OptimizationPass;
use crate::ir::{BasicBlock, Function, Instruction, Module as IrModule, ValueId};
use std::collections::HashMap;

/// Common Subexpression Elimination optimization pass
#[derive(Debug)]
pub struct CommonSubexpressionElimination {
    /// Statistics for debugging
    eliminated_count: usize,
}

impl CommonSubexpressionElimination {
    /// Create a new CSE pass
    pub fn new() -> Self {
        CommonSubexpressionElimination {
            eliminated_count: 0,
        }
    }

    /// Get the number of expressions eliminated
    pub fn eliminated_count(&self) -> usize {
        self.eliminated_count
    }
}

impl OptimizationPass for CommonSubexpressionElimination {
    fn optimize(&mut self, module: &mut IrModule) -> bool {
        let mut changed = false;
        self.eliminated_count = 0;

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
        "CommonSubexpressionElimination"
    }
}

impl CommonSubexpressionElimination {
    /// Optimize a single function
    fn optimize_function(&mut self, function: &mut Function) -> bool {
        let mut changed = false;

        // Process each basic block independently
        // CSE within a block is simpler and safer than across blocks
        let block_ids: Vec<_> = function.blocks().keys().cloned().collect();
        for block_id in block_ids {
            if let Some(block) = function.get_block_mut(block_id) {
                changed |= self.optimize_block(block);
            }
        }

        changed
    }

    /// Optimize a single basic block
    fn optimize_block(&mut self, block: &mut BasicBlock) -> bool {
        // Map from expression hash to the ValueId that computes it
        let mut expression_map: HashMap<ExpressionKey, ValueId> = HashMap::new();
        // Map from old ValueId to new ValueId for replacements
        let mut replacements: HashMap<ValueId, ValueId> = HashMap::new();
        let mut changed = false;

        // First pass: identify common subexpressions
        for (value_id, inst_with_loc) in &block.instructions {
            // Skip instructions with side effects
            if has_side_effects(&inst_with_loc.instruction) {
                // Clear expression map for safety when we encounter side effects
                // This prevents optimizing across instructions that might change memory
                if matches!(
                    &inst_with_loc.instruction,
                    Instruction::Store { .. } | Instruction::Call { .. }
                ) {
                    expression_map.clear();
                }
                continue;
            }

            // Create a key for this expression
            if let Some(key) = create_expression_key(&inst_with_loc.instruction, &replacements) {
                if let Some(&existing_value) = expression_map.get(&key) {
                    // Found a common subexpression
                    replacements.insert(*value_id, existing_value);
                    self.eliminated_count += 1;
                    changed = true;
                } else {
                    // First occurrence of this expression
                    expression_map.insert(key, *value_id);
                }
            }
        }

        // Second pass: apply replacements
        if changed {
            for (_, inst_with_loc) in &mut block.instructions {
                apply_replacements(&mut inst_with_loc.instruction, &replacements);
            }
        }

        changed
    }
}

/// Key for identifying equivalent expressions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ExpressionKey {
    Binary {
        op: BinaryOpHash,
        lhs: ValueId,
        rhs: ValueId,
        ty: String, // Type as string for hashing
    },
    Unary {
        op: UnaryOpHash,
        operand: ValueId,
        ty: String,
    },
    Cast {
        value: ValueId,
        from_ty: String,
        to_ty: String,
    },
    Compare {
        op: ComparisonOpHash,
        lhs: ValueId,
        rhs: ValueId,
    },
}

// Wrapper types to add Hash implementation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BinaryOpHash {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum UnaryOpHash {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ComparisonOpHash {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl From<crate::ir::BinaryOp> for BinaryOpHash {
    fn from(op: crate::ir::BinaryOp) -> Self {
        use crate::ir::BinaryOp;
        match op {
            BinaryOp::Add => BinaryOpHash::Add,
            BinaryOp::Sub => BinaryOpHash::Sub,
            BinaryOp::Mul => BinaryOpHash::Mul,
            BinaryOp::Div => BinaryOpHash::Div,
            BinaryOp::Mod => BinaryOpHash::Mod,
            BinaryOp::And => BinaryOpHash::And,
            BinaryOp::Or => BinaryOpHash::Or,
        }
    }
}

impl From<crate::ir::UnaryOp> for UnaryOpHash {
    fn from(op: crate::ir::UnaryOp) -> Self {
        use crate::ir::UnaryOp;
        match op {
            UnaryOp::Neg => UnaryOpHash::Neg,
            UnaryOp::Not => UnaryOpHash::Not,
        }
    }
}

impl From<crate::ir::ComparisonOp> for ComparisonOpHash {
    fn from(op: crate::ir::ComparisonOp) -> Self {
        use crate::ir::ComparisonOp;
        match op {
            ComparisonOp::Eq => ComparisonOpHash::Eq,
            ComparisonOp::Ne => ComparisonOpHash::Ne,
            ComparisonOp::Lt => ComparisonOpHash::Lt,
            ComparisonOp::Le => ComparisonOpHash::Le,
            ComparisonOp::Gt => ComparisonOpHash::Gt,
            ComparisonOp::Ge => ComparisonOpHash::Ge,
        }
    }
}

/// Create an expression key for CSE, applying any existing replacements
fn create_expression_key(
    inst: &Instruction,
    replacements: &HashMap<ValueId, ValueId>,
) -> Option<ExpressionKey> {
    match inst {
        Instruction::Binary { op, lhs, rhs, ty } => {
            let lhs = replacements.get(lhs).cloned().unwrap_or(*lhs);
            let rhs = replacements.get(rhs).cloned().unwrap_or(*rhs);

            // For commutative operations, normalize the order
            let (lhs, rhs) = if is_commutative(*op) && lhs.0 > rhs.0 {
                (rhs, lhs)
            } else {
                (lhs, rhs)
            };

            Some(ExpressionKey::Binary {
                op: (*op).into(),
                lhs,
                rhs,
                ty: format!("{:?}", ty), // Simple string representation
            })
        }
        Instruction::Unary { op, operand, ty } => {
            let operand = replacements.get(operand).cloned().unwrap_or(*operand);
            Some(ExpressionKey::Unary {
                op: (*op).into(),
                operand,
                ty: format!("{:?}", ty),
            })
        }
        Instruction::Cast {
            value,
            from_ty,
            to_ty,
        } => {
            let value = replacements.get(value).cloned().unwrap_or(*value);
            Some(ExpressionKey::Cast {
                value,
                from_ty: format!("{:?}", from_ty),
                to_ty: format!("{:?}", to_ty),
            })
        }
        Instruction::Compare { op, lhs, rhs } => {
            let lhs = replacements.get(lhs).cloned().unwrap_or(*lhs);
            let rhs = replacements.get(rhs).cloned().unwrap_or(*rhs);

            // For equality comparisons, normalize the order
            let (op, lhs, rhs) = if matches!(
                op,
                crate::ir::ComparisonOp::Eq | crate::ir::ComparisonOp::Ne
            ) && lhs.0 > rhs.0
            {
                ((*op).into(), rhs, lhs)
            } else {
                ((*op).into(), lhs, rhs)
            };

            Some(ExpressionKey::Compare { op, lhs, rhs })
        }
        _ => None, // Other instructions are not candidates for CSE
    }
}

/// Check if an instruction has side effects
fn has_side_effects(inst: &Instruction) -> bool {
    matches!(
        inst,
        Instruction::Store { .. }
            | Instruction::Call { .. }
            | Instruction::Return(_)
            | Instruction::Branch(_)
            | Instruction::CondBranch { .. }
    )
}

/// Check if a binary operation is commutative
fn is_commutative(op: crate::ir::BinaryOp) -> bool {
    use crate::ir::BinaryOp;
    matches!(
        op,
        BinaryOp::Add | BinaryOp::Mul | BinaryOp::And | BinaryOp::Or
    )
}

/// Apply value replacements to an instruction
fn apply_replacements(inst: &mut Instruction, replacements: &HashMap<ValueId, ValueId>) {
    match inst {
        Instruction::Binary { lhs, rhs, .. } => {
            if let Some(&new_lhs) = replacements.get(lhs) {
                *lhs = new_lhs;
            }
            if let Some(&new_rhs) = replacements.get(rhs) {
                *rhs = new_rhs;
            }
        }
        Instruction::Unary { operand, .. } => {
            if let Some(&new_operand) = replacements.get(operand) {
                *operand = new_operand;
            }
        }
        Instruction::Compare { lhs, rhs, .. } => {
            if let Some(&new_lhs) = replacements.get(lhs) {
                *lhs = new_lhs;
            }
            if let Some(&new_rhs) = replacements.get(rhs) {
                *rhs = new_rhs;
            }
        }
        Instruction::Cast { value, .. } => {
            if let Some(&new_value) = replacements.get(value) {
                *value = new_value;
            }
        }
        Instruction::Call { args, .. } => {
            for arg in args {
                if let Some(&new_arg) = replacements.get(arg) {
                    *arg = new_arg;
                }
            }
        }
        Instruction::Load { ptr, .. } => {
            if let Some(&new_ptr) = replacements.get(ptr) {
                *ptr = new_ptr;
            }
        }
        Instruction::Store { ptr, value } => {
            if let Some(&new_ptr) = replacements.get(ptr) {
                *ptr = new_ptr;
            }
            if let Some(&new_value) = replacements.get(value) {
                *value = new_value;
            }
        }
        Instruction::GetElementPtr { ptr, index, .. } => {
            if let Some(&new_ptr) = replacements.get(ptr) {
                *ptr = new_ptr;
            }
            if let Some(&new_index) = replacements.get(index) {
                *index = new_index;
            }
        }
        Instruction::Phi { incoming, .. } => {
            for (value, _) in incoming {
                if let Some(&new_value) = replacements.get(value) {
                    *value = new_value;
                }
            }
        }
        Instruction::Return(Some(value)) => {
            if let Some(&new_value) = replacements.get(value) {
                *value = new_value;
            }
        }
        Instruction::CondBranch { condition, .. } => {
            if let Some(&new_condition) = replacements.get(condition) {
                *condition = new_condition;
            }
        }
        _ => {} // Other instructions don't have value operands
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BinaryOp, ComparisonOp, Constant, IrBuilder, UnaryOp};
    use crate::types::Type;

    #[test]
    fn test_cse_binary_operations() {
        let mut builder = IrBuilder::new();
        let func = builder.create_function("test".to_string(), vec![], Type::I32);

        // Create two identical additions
        let a = builder.const_value(Constant::I32(10));
        let b = builder.const_value(Constant::I32(20));
        let v1 = builder
            .build_binary(BinaryOp::Add, a, b, Type::I32)
            .unwrap();
        let v2 = builder
            .build_binary(BinaryOp::Add, a, b, Type::I32)
            .unwrap(); // Should be eliminated
        let result = builder
            .build_binary(BinaryOp::Add, v1, v2, Type::I32)
            .unwrap();
        builder.build_return(Some(result));

        let mut module = builder.build();

        // Run CSE
        let mut cse = CommonSubexpressionElimination::new();
        let changed = cse.optimize(&mut module);

        assert!(changed);
        assert_eq!(cse.eliminated_count(), 1);

        // Verify that v2 was replaced with v1 in the final addition
        if let Some(func) = module.get_function(func) {
            if let Some(block) = func.blocks().values().next() {
                // Find the final addition instruction
                for (_, inst_with_loc) in &block.instructions {
                    if let Instruction::Binary {
                        op: BinaryOp::Add,
                        lhs,
                        rhs,
                        ..
                    } = &inst_with_loc.instruction
                    {
                        if *lhs == v1 {
                            // The second operand should now be v1 instead of v2
                            assert_eq!(*rhs, v1);
                            return;
                        }
                    }
                }
            }
        }
        panic!("Expected to find the optimized addition");
    }

    #[test]
    fn test_cse_commutative_operations() {
        let mut builder = IrBuilder::new();
        let func = builder.create_function("test".to_string(), vec![], Type::I32);

        let a = builder.const_value(Constant::I32(10));
        let b = builder.const_value(Constant::I32(20));

        // Create a + b and b + a (should be recognized as the same)
        let v1 = builder
            .build_binary(BinaryOp::Add, a, b, Type::I32)
            .unwrap();
        let v2 = builder
            .build_binary(BinaryOp::Add, b, a, Type::I32)
            .unwrap(); // Should be eliminated
        let result = builder
            .build_binary(BinaryOp::Add, v1, v2, Type::I32)
            .unwrap();
        builder.build_return(Some(result));

        let mut module = builder.build();

        let mut cse = CommonSubexpressionElimination::new();
        let changed = cse.optimize(&mut module);

        assert!(changed);
        assert_eq!(cse.eliminated_count(), 1);
    }

    #[test]
    fn test_cse_unary_operations() {
        let mut builder = IrBuilder::new();
        let func = builder.create_function("test".to_string(), vec![], Type::I32);

        let a = builder.const_value(Constant::I32(10));
        let v1 = builder.build_unary(UnaryOp::Neg, a, Type::I32).unwrap();
        let v2 = builder.build_unary(UnaryOp::Neg, a, Type::I32).unwrap(); // Should be eliminated
        let result = builder
            .build_binary(BinaryOp::Add, v1, v2, Type::I32)
            .unwrap();
        builder.build_return(Some(result));

        let mut module = builder.build();

        let mut cse = CommonSubexpressionElimination::new();
        let changed = cse.optimize(&mut module);

        assert!(changed);
        assert_eq!(cse.eliminated_count(), 1);
    }

    #[test]
    fn test_cse_cast_operations() {
        let mut builder = IrBuilder::new();
        let func = builder.create_function("test".to_string(), vec![], Type::F32);

        let a = builder.const_value(Constant::I32(10));
        let v1 = builder
            .add_instruction(Instruction::Cast {
                value: a,
                from_ty: Type::I32,
                to_ty: Type::F32,
            })
            .unwrap();
        let v2 = builder
            .add_instruction(Instruction::Cast {
                value: a,
                from_ty: Type::I32,
                to_ty: Type::F32,
            })
            .unwrap(); // Should be eliminated
        let result = builder
            .build_binary(BinaryOp::Add, v1, v2, Type::F32)
            .unwrap();
        builder.build_return(Some(result));

        let mut module = builder.build();

        let mut cse = CommonSubexpressionElimination::new();
        let changed = cse.optimize(&mut module);

        assert!(changed);
        assert_eq!(cse.eliminated_count(), 1);
    }

    #[test]
    fn test_cse_no_optimization_with_side_effects() {
        let mut builder = IrBuilder::new();
        let func = builder.create_function("test".to_string(), vec![], Type::I32);

        let a = builder.const_value(Constant::I32(10));
        let b = builder.const_value(Constant::I32(20));

        // Create two additions separated by a store (side effect)
        let v1 = builder
            .build_binary(BinaryOp::Add, a, b, Type::I32)
            .unwrap();
        let ptr = builder.build_alloc(Type::I32).unwrap();
        builder.build_store(ptr, v1);
        let v2 = builder
            .build_binary(BinaryOp::Add, a, b, Type::I32)
            .unwrap(); // Should NOT be eliminated
        builder.build_return(Some(v2));

        let mut module = builder.build();

        let mut cse = CommonSubexpressionElimination::new();
        let changed = cse.optimize(&mut module);

        // No optimization should occur because of the intervening store
        assert!(!changed);
        assert_eq!(cse.eliminated_count(), 0);
    }

    #[test]
    fn test_cse_comparison_operations() {
        let mut builder = IrBuilder::new();
        let func = builder.create_function("test".to_string(), vec![], Type::Bool);

        let a = builder.const_value(Constant::I32(10));
        let b = builder.const_value(Constant::I32(20));

        let v1 = builder.build_compare(ComparisonOp::Eq, a, b).unwrap();
        let v2 = builder.build_compare(ComparisonOp::Eq, a, b).unwrap(); // Should be eliminated
        let v3 = builder.build_compare(ComparisonOp::Eq, b, a).unwrap(); // Should also be eliminated (commutative)

        let result = builder
            .build_binary(BinaryOp::Or, v1, v2, Type::Bool)
            .unwrap();
        let final_result = builder
            .build_binary(BinaryOp::Or, result, v3, Type::Bool)
            .unwrap();
        builder.build_return(Some(final_result));

        let mut module = builder.build();

        let mut cse = CommonSubexpressionElimination::new();
        let changed = cse.optimize(&mut module);

        assert!(changed);
        assert_eq!(cse.eliminated_count(), 2); // Both v2 and v3 should be eliminated
    }
}
