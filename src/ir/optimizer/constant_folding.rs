//! Constant Folding Optimization Pass
//!
//! This pass evaluates constant expressions at compile time to improve performance.
//! It handles arithmetic, comparison, and logical operations on constant values.

use super::OptimizationPass;
use crate::ir::{
    BasicBlock, BinaryOp, BlockId, ComparisonOp, Constant, Function, Instruction,
    InstructionWithLocation, Module as IrModule, UnaryOp, ValueId,
};
use std::collections::HashMap;

/// Constant folding optimization pass
pub struct ConstantFolding {
    /// Map from ValueId to constant values for constant propagation
    constants: HashMap<ValueId, Constant>,
}

impl ConstantFolding {
    /// Create a new constant folding pass
    pub fn new() -> Self {
        ConstantFolding {
            constants: HashMap::new(),
        }
    }

    /// Clear the constants map for a new function
    fn clear_constants(&mut self) {
        self.constants.clear();
    }

    /// Try to get a constant value for a ValueId
    fn get_constant(&self, value_id: ValueId) -> Option<&Constant> {
        self.constants.get(&value_id)
    }

    /// Evaluate a binary operation on constants
    fn eval_binary_op(op: BinaryOp, lhs: &Constant, rhs: &Constant) -> Option<Constant> {
        match (op, lhs, rhs) {
            // Integer arithmetic
            (BinaryOp::Add, Constant::I32(a), Constant::I32(b)) => {
                Some(Constant::I32(a.wrapping_add(*b)))
            }
            (BinaryOp::Sub, Constant::I32(a), Constant::I32(b)) => {
                Some(Constant::I32(a.wrapping_sub(*b)))
            }
            (BinaryOp::Mul, Constant::I32(a), Constant::I32(b)) => {
                Some(Constant::I32(a.wrapping_mul(*b)))
            }
            (BinaryOp::Div, Constant::I32(a), Constant::I32(b)) => {
                if *b != 0 {
                    Some(Constant::I32(a / b))
                } else {
                    None // Division by zero - don't fold
                }
            }
            (BinaryOp::Mod, Constant::I32(a), Constant::I32(b)) => {
                if *b != 0 {
                    Some(Constant::I32(a % b))
                } else {
                    None // Modulo by zero - don't fold
                }
            }

            // Float arithmetic
            (BinaryOp::Add, Constant::F32(a), Constant::F32(b)) => Some(Constant::F32(a + b)),
            (BinaryOp::Sub, Constant::F32(a), Constant::F32(b)) => Some(Constant::F32(a - b)),
            (BinaryOp::Mul, Constant::F32(a), Constant::F32(b)) => Some(Constant::F32(a * b)),
            (BinaryOp::Div, Constant::F32(a), Constant::F32(b)) => {
                if *b != 0.0 {
                    Some(Constant::F32(a / b))
                } else {
                    None // Division by zero - don't fold
                }
            }
            (BinaryOp::Mod, Constant::F32(a), Constant::F32(b)) => {
                if *b != 0.0 {
                    Some(Constant::F32(a % b))
                } else {
                    None // Modulo by zero - don't fold
                }
            }

            // Boolean logical operations
            (BinaryOp::And, Constant::Bool(a), Constant::Bool(b)) => Some(Constant::Bool(*a && *b)),
            (BinaryOp::Or, Constant::Bool(a), Constant::Bool(b)) => Some(Constant::Bool(*a || *b)),

            _ => None, // Type mismatch or unsupported operation
        }
    }

    /// Evaluate a unary operation on a constant
    fn eval_unary_op(op: UnaryOp, operand: &Constant) -> Option<Constant> {
        match (op, operand) {
            (UnaryOp::Neg, Constant::I32(n)) => Some(Constant::I32(-n)),
            (UnaryOp::Neg, Constant::F32(f)) => Some(Constant::F32(-f)),
            (UnaryOp::Not, Constant::Bool(b)) => Some(Constant::Bool(!b)),
            _ => None, // Type mismatch
        }
    }

    /// Evaluate a comparison operation on constants
    fn eval_comparison_op(op: ComparisonOp, lhs: &Constant, rhs: &Constant) -> Option<Constant> {
        match (lhs, rhs) {
            // Integer comparisons
            (Constant::I32(a), Constant::I32(b)) => {
                let result = match op {
                    ComparisonOp::Eq => a == b,
                    ComparisonOp::Ne => a != b,
                    ComparisonOp::Lt => a < b,
                    ComparisonOp::Le => a <= b,
                    ComparisonOp::Gt => a > b,
                    ComparisonOp::Ge => a >= b,
                };
                Some(Constant::Bool(result))
            }

            // Float comparisons
            (Constant::F32(a), Constant::F32(b)) => {
                let result = match op {
                    ComparisonOp::Eq => a == b,
                    ComparisonOp::Ne => a != b,
                    ComparisonOp::Lt => a < b,
                    ComparisonOp::Le => a <= b,
                    ComparisonOp::Gt => a > b,
                    ComparisonOp::Ge => a >= b,
                };
                Some(Constant::Bool(result))
            }

            // Boolean comparisons (only equality)
            (Constant::Bool(a), Constant::Bool(b)) => {
                match op {
                    ComparisonOp::Eq => Some(Constant::Bool(a == b)),
                    ComparisonOp::Ne => Some(Constant::Bool(a != b)),
                    _ => None, // Other comparisons don't make sense for booleans
                }
            }

            // String comparisons
            (Constant::String(a), Constant::String(b)) => {
                let result = match op {
                    ComparisonOp::Eq => a == b,
                    ComparisonOp::Ne => a != b,
                    ComparisonOp::Lt => a < b,
                    ComparisonOp::Le => a <= b,
                    ComparisonOp::Gt => a > b,
                    ComparisonOp::Ge => a >= b,
                };
                Some(Constant::Bool(result))
            }

            _ => None, // Type mismatch or unsupported comparison
        }
    }

    /// Optimize a single function
    fn optimize_function(&mut self, function: &mut Function) -> bool {
        self.clear_constants();
        let mut changed = false;

        // Process each block
        let block_ids: Vec<BlockId> = function.blocks().keys().copied().collect();
        for block_id in block_ids {
            if let Some(block) = function.get_block_mut(block_id) {
                changed |= self.optimize_block(block);
            }
        }

        changed
    }

    /// Optimize a single basic block
    fn optimize_block(&mut self, block: &mut BasicBlock) -> bool {
        let mut changed = false;
        let mut new_instructions = Vec::new();

        for (value_id, inst_with_loc) in &block.instructions {
            let mut inst = inst_with_loc.instruction.clone();
            let location = inst_with_loc.source_location;

            // Try to fold the instruction
            let folded_inst = match &inst {
                Instruction::Const(c) => {
                    // Record constant value for propagation
                    self.constants.insert(*value_id, c.clone());
                    None // Keep the constant instruction as-is
                }

                Instruction::Binary { op, lhs, rhs, ty } => {
                    // Try to get constant values for operands
                    if let (Some(lhs_const), Some(rhs_const)) =
                        (self.get_constant(*lhs), self.get_constant(*rhs))
                    {
                        // Try to evaluate the operation
                        if let Some(result) = Self::eval_binary_op(*op, lhs_const, rhs_const) {
                            // Record the folded constant
                            self.constants.insert(*value_id, result.clone());
                            Some(Instruction::Const(result))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }

                Instruction::Unary { op, operand, ty } => {
                    // Try to get constant value for operand
                    if let Some(operand_const) = self.get_constant(*operand) {
                        // Try to evaluate the operation
                        if let Some(result) = Self::eval_unary_op(*op, operand_const) {
                            // Record the folded constant
                            self.constants.insert(*value_id, result.clone());
                            Some(Instruction::Const(result))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }

                Instruction::Compare { op, lhs, rhs } => {
                    // Try to get constant values for operands
                    if let (Some(lhs_const), Some(rhs_const)) =
                        (self.get_constant(*lhs), self.get_constant(*rhs))
                    {
                        // Try to evaluate the comparison
                        if let Some(result) = Self::eval_comparison_op(*op, lhs_const, rhs_const) {
                            // Record the folded constant
                            self.constants.insert(*value_id, result.clone());
                            Some(Instruction::Const(result))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }

                _ => None, // Other instructions can't be folded
            };

            // Use folded instruction if available, otherwise keep original
            if let Some(folded) = folded_inst {
                changed = true;
                inst = folded;
            }

            new_instructions.push((
                *value_id,
                InstructionWithLocation {
                    instruction: inst,
                    source_location: location,
                },
            ));
        }

        if changed {
            block.instructions = new_instructions;
        }

        changed
    }
}

impl OptimizationPass for ConstantFolding {
    fn optimize(&mut self, module: &mut IrModule) -> bool {
        let mut changed = false;

        // Optimize each function
        let func_ids: Vec<_> = module.functions().keys().copied().collect();
        for func_id in func_ids {
            if let Some(function) = module.get_function_mut(func_id) {
                changed |= self.optimize_function(function);
            }
        }

        changed
    }

    fn name(&self) -> &'static str {
        "constant-folding"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::IrBuilder;
    use crate::types::Type;

    #[test]
    fn test_arithmetic_folding() {
        let mut builder = IrBuilder::new();

        // Create a function with constant arithmetic
        let func_id = builder.create_function("test".to_string(), vec![], Type::I32);

        // Build: 2 + 3
        let two = builder.const_value(Constant::I32(2));
        let three = builder.const_value(Constant::I32(3));
        let add = builder
            .build_binary(BinaryOp::Add, two, three, Type::I32)
            .unwrap();

        // Build: (2 + 3) * 4
        let four = builder.const_value(Constant::I32(4));
        let mul = builder
            .build_binary(BinaryOp::Mul, add, four, Type::I32)
            .unwrap();

        builder.build_return(Some(mul));

        let mut module = builder.build();

        // Run constant folding
        let mut pass = ConstantFolding::new();
        let changed = pass.optimize(&mut module);
        assert!(changed);

        // Check that constants were folded
        // The pass should have replaced the add and mul instructions with constants
        if let Some(func) = module.get_function(func_id) {
            if let Some(block) = func.get_block(func.entry_block.unwrap()) {
                // Count constant instructions
                let const_count = block
                    .instructions
                    .iter()
                    .filter(|(_, inst)| matches!(&inst.instruction, Instruction::Const(_)))
                    .count();

                // Should have 5 constants: 2, 3, 5 (folded 2+3), 4, and 20 (folded 5*4)
                assert_eq!(const_count, 5);
            }
        }
    }

    #[test]
    fn test_comparison_folding() {
        let mut builder = IrBuilder::new();

        let func_id = builder.create_function("test".to_string(), vec![], Type::Bool);

        // Build: 10 > 5
        let ten = builder.const_value(Constant::I32(10));
        let five = builder.const_value(Constant::I32(5));
        let cmp = builder.build_compare(ComparisonOp::Gt, ten, five).unwrap();

        builder.build_return(Some(cmp));

        let mut module = builder.build();

        // Run constant folding
        let mut pass = ConstantFolding::new();
        let changed = pass.optimize(&mut module);
        assert!(changed);

        // Check that comparison was folded to true
        if let Some(func) = module.get_function(func_id) {
            if let Some(block) = func.get_block(func.entry_block.unwrap()) {
                // Find the folded comparison result
                let has_true = block.instructions.iter().any(|(_, inst)| {
                    matches!(&inst.instruction, Instruction::Const(Constant::Bool(true)))
                });
                assert!(has_true);
            }
        }
    }

    #[test]
    fn test_logical_folding() {
        let mut builder = IrBuilder::new();

        let func_id = builder.create_function("test".to_string(), vec![], Type::Bool);

        // Build: true && false
        let t = builder.const_value(Constant::Bool(true));
        let f = builder.const_value(Constant::Bool(false));
        let and = builder
            .build_binary(BinaryOp::And, t, f, Type::Bool)
            .unwrap();

        // Build: (true && false) || true
        let t2 = builder.const_value(Constant::Bool(true));
        let or = builder
            .build_binary(BinaryOp::Or, and, t2, Type::Bool)
            .unwrap();

        builder.build_return(Some(or));

        let mut module = builder.build();

        // Run constant folding
        let mut pass = ConstantFolding::new();
        let changed = pass.optimize(&mut module);
        assert!(changed);

        // Check that logical operations were folded
        if let Some(func) = module.get_function(func_id) {
            if let Some(block) = func.get_block(func.entry_block.unwrap()) {
                // Should have folded: true && false = false, false || true = true
                let has_final_true = block
                    .instructions
                    .iter()
                    .filter(|(_, inst)| {
                        matches!(&inst.instruction, Instruction::Const(Constant::Bool(true)))
                    })
                    .count()
                    >= 2; // Original true values plus final result
                assert!(has_final_true);
            }
        }
    }

    #[test]
    fn test_unary_folding() {
        let mut builder = IrBuilder::new();

        let func_id = builder.create_function("test".to_string(), vec![], Type::I32);

        // Build: -42
        let n = builder.const_value(Constant::I32(42));
        let neg = builder.build_unary(UnaryOp::Neg, n, Type::I32).unwrap();

        // Build: !true
        let t = builder.const_value(Constant::Bool(true));
        let not = builder.build_unary(UnaryOp::Not, t, Type::Bool).unwrap();

        builder.build_return(Some(neg));

        let mut module = builder.build();

        // Run constant folding
        let mut pass = ConstantFolding::new();
        let changed = pass.optimize(&mut module);
        assert!(changed);

        // Check that unary operations were folded
        if let Some(func) = module.get_function(func_id) {
            if let Some(block) = func.get_block(func.entry_block.unwrap()) {
                // Should have folded -42 and !true
                let has_neg_42 = block.instructions.iter().any(|(_, inst)| {
                    matches!(&inst.instruction, Instruction::Const(Constant::I32(-42)))
                });
                let has_false = block.instructions.iter().any(|(_, inst)| {
                    matches!(&inst.instruction, Instruction::Const(Constant::Bool(false)))
                });
                assert!(has_neg_42);
                assert!(has_false);
            }
        }
    }

    #[test]
    fn test_division_by_zero_not_folded() {
        let mut builder = IrBuilder::new();

        let func_id = builder.create_function("test".to_string(), vec![], Type::I32);

        // Build: 10 / 0 (should not be folded)
        let ten = builder.const_value(Constant::I32(10));
        let zero = builder.const_value(Constant::I32(0));
        let div = builder
            .build_binary(BinaryOp::Div, ten, zero, Type::I32)
            .unwrap();

        builder.build_return(Some(div));

        let mut module = builder.build();
        let original_module = module.clone();

        // Run constant folding
        let mut pass = ConstantFolding::new();
        pass.optimize(&mut module);

        // Check that division by zero was NOT folded
        if let Some(func) = module.get_function(func_id) {
            if let Some(block) = func.get_block(func.entry_block.unwrap()) {
                // Should still have the division instruction
                let has_div = block.instructions.iter().any(|(_, inst)| {
                    matches!(
                        &inst.instruction,
                        Instruction::Binary {
                            op: BinaryOp::Div,
                            ..
                        }
                    )
                });
                assert!(has_div);
            }
        }
    }
}
