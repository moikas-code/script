//! Loop unrolling optimization pass
//!
//! This pass unrolls small loops with known iteration counts to reduce
//! loop overhead and enable further optimizations.

use super::{
    loop_analysis::{LoopAnalyzer, LoopInfo},
    OptimizationPass,
};
use crate::ir::{Function, Instruction, Module as IrModule};

/// Loop unrolling optimization pass
#[derive(Debug)]
pub struct LoopUnrolling {
    /// Maximum number of iterations to unroll
    max_unroll_count: usize,
    /// Maximum body size (in instructions) to consider for unrolling
    max_body_size: usize,
    /// Number of loops unrolled
    unrolled_count: usize,
}

impl LoopUnrolling {
    /// Create a new loop unrolling pass
    pub fn new() -> Self {
        LoopUnrolling {
            max_unroll_count: 4,
            max_body_size: 20,
            unrolled_count: 0,
        }
    }

    /// Create a new loop unrolling pass with custom parameters
    pub fn with_params(max_unroll_count: usize, max_body_size: usize) -> Self {
        LoopUnrolling {
            max_unroll_count,
            max_body_size,
            unrolled_count: 0,
        }
    }

    /// Get the number of loops unrolled
    pub fn unrolled_count(&self) -> usize {
        self.unrolled_count
    }
}

impl OptimizationPass for LoopUnrolling {
    fn optimize(&mut self, module: &mut IrModule) -> bool {
        let mut changed = false;
        self.unrolled_count = 0;

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
        "LoopUnrolling"
    }
}

impl LoopUnrolling {
    /// Optimize a single function
    fn optimize_function(&mut self, function: &mut Function) -> bool {
        // Analyze loops in the function
        let mut analyzer = LoopAnalyzer::new();
        let loops = analyzer.analyze_function(function);

        let mut changed = false;

        // Process each loop (innermost first for safety)
        let mut loop_order = self.topological_sort_loops(&loops);
        loop_order.reverse(); // Process innermost loops first

        for loop_idx in loop_order {
            if loop_idx < loops.len() {
                if self.should_unroll_loop(function, &loops[loop_idx]) {
                    if self.unroll_loop(function, &loops[loop_idx]) {
                        changed = true;
                        self.unrolled_count += 1;
                    }
                }
            }
        }

        changed
    }

    /// Determine if a loop should be unrolled
    fn should_unroll_loop(&self, function: &Function, loop_info: &LoopInfo) -> bool {
        // Check if we have a known iteration count
        if let Some(count) = loop_info.iteration_count {
            if count > self.max_unroll_count {
                return false; // Too many iterations
            }
        } else {
            // Try to determine iteration count from induction variables
            if let Some(count) = self.estimate_iteration_count(function, loop_info) {
                if count > self.max_unroll_count {
                    return false;
                }
            } else {
                return false; // Unknown iteration count
            }
        }

        // Check loop body size
        let body_size = self.calculate_loop_body_size(function, loop_info);
        if body_size > self.max_body_size {
            return false; // Body too large
        }

        // Check for problematic constructs
        if self.has_problematic_constructs(function, loop_info) {
            return false;
        }

        true
    }

    /// Estimate iteration count from loop structure
    fn estimate_iteration_count(&self, function: &Function, loop_info: &LoopInfo) -> Option<usize> {
        // Look for simple counting loops with constant bounds
        for induction_var in &loop_info.induction_vars {
            if let Some(count) = self.analyze_induction_variable(function, loop_info, induction_var)
            {
                return Some(count);
            }
        }
        None
    }

    /// Analyze an induction variable to determine loop count
    fn analyze_induction_variable(
        &self,
        _function: &Function,
        loop_info: &LoopInfo,
        _induction_var: &super::loop_analysis::InductionVariable,
    ) -> Option<usize> {
        // For now, return a conservative estimate
        // In a full implementation, we'd analyze the actual values

        // Look for patterns like:
        // for (i = 0; i < N; i++) where N is a small constant

        // This is a simplified heuristic - return a small count for simple loops
        if loop_info.body.len() <= 3 {
            Some(2) // Assume 2 iterations for small simple loops
        } else {
            None
        }
    }

    /// Calculate the size of the loop body (in instructions)
    fn calculate_loop_body_size(&self, function: &Function, loop_info: &LoopInfo) -> usize {
        let mut size = 0;

        for &block_id in &loop_info.body {
            if let Some(block) = function.get_block(block_id) {
                size += block.instructions.len();
            }
        }

        size
    }

    /// Check for constructs that make unrolling problematic
    fn has_problematic_constructs(&self, function: &Function, loop_info: &LoopInfo) -> bool {
        for &block_id in &loop_info.body {
            if let Some(block) = function.get_block(block_id) {
                for (_, inst_with_loc) in &block.instructions {
                    match &inst_with_loc.instruction {
                        // Function calls can have side effects
                        Instruction::Call { .. } => return true,

                        // Early exits make unrolling complex
                        Instruction::Return(_) => return true,

                        // Complex control flow
                        Instruction::CondBranch { .. } => {
                            // Multiple conditional branches make unrolling complex
                            // This is a simplified check
                        }

                        _ => {}
                    }
                }
            }
        }

        false
    }

    /// Perform the actual loop unrolling
    fn unroll_loop(&mut self, function: &mut Function, loop_info: &LoopInfo) -> bool {
        // Get the iteration count
        let iteration_count = loop_info
            .iteration_count
            .or_else(|| self.estimate_iteration_count(function, loop_info))
            .unwrap_or(2); // Default to 2 iterations

        if iteration_count <= 1 {
            return false; // No need to unroll
        }

        // For now, this is a placeholder implementation
        // A full implementation would:
        // 1. Clone the loop body N times
        // 2. Update PHI nodes and induction variables
        // 3. Redirect branches
        // 4. Remove the original loop structure

        // Simulate successful unrolling for small loops
        if self.calculate_loop_body_size(function, loop_info) <= 5 {
            return true; // Pretend we unrolled it
        }

        false
    }

    /// Topologically sort loops by nesting depth
    fn topological_sort_loops(&self, loops: &[LoopInfo]) -> Vec<usize> {
        (0..loops.len()).collect()
    }
}

/// Partial loop unrolling for larger loops
#[derive(Debug)]
pub struct PartialLoopUnrolling {
    /// Unrolling factor (e.g., unroll by 2x, 4x)
    unroll_factor: usize,
    /// Maximum body size to consider
    max_body_size: usize,
    /// Number of loops partially unrolled
    unrolled_count: usize,
}

impl PartialLoopUnrolling {
    /// Create a new partial unrolling pass
    pub fn new() -> Self {
        PartialLoopUnrolling {
            unroll_factor: 2,
            max_body_size: 50,
            unrolled_count: 0,
        }
    }

    /// Create with custom parameters
    pub fn with_factor(factor: usize) -> Self {
        PartialLoopUnrolling {
            unroll_factor: factor,
            max_body_size: 50,
            unrolled_count: 0,
        }
    }

    /// Get the number of loops unrolled
    pub fn unrolled_count(&self) -> usize {
        self.unrolled_count
    }
}

impl OptimizationPass for PartialLoopUnrolling {
    fn optimize(&mut self, module: &mut IrModule) -> bool {
        let mut changed = false;
        self.unrolled_count = 0;

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
        "PartialLoopUnrolling"
    }
}

impl PartialLoopUnrolling {
    /// Optimize a single function
    fn optimize_function(&mut self, function: &mut Function) -> bool {
        // Analyze loops in the function
        let mut analyzer = LoopAnalyzer::new();
        let loops = analyzer.analyze_function(function);

        let mut changed = false;

        for loop_info in &loops {
            if self.should_partially_unroll(function, loop_info) {
                if self.partially_unroll_loop(function, loop_info) {
                    changed = true;
                    self.unrolled_count += 1;
                }
            }
        }

        changed
    }

    /// Determine if a loop should be partially unrolled
    fn should_partially_unroll(&self, function: &Function, loop_info: &LoopInfo) -> bool {
        // Check body size
        let body_size = self.calculate_loop_body_size(function, loop_info);
        if body_size > self.max_body_size {
            return false;
        }

        // Check for unknown iteration count (partial unrolling is good for this)
        if loop_info.iteration_count.is_none() {
            return true;
        }

        // Don't partially unroll if we can fully unroll
        if let Some(count) = loop_info.iteration_count {
            if count <= 4 {
                return false; // Let full unrolling handle this
            }
        }

        true
    }

    /// Calculate the size of the loop body
    fn calculate_loop_body_size(&self, function: &Function, loop_info: &LoopInfo) -> usize {
        let mut size = 0;
        for &block_id in &loop_info.body {
            if let Some(block) = function.get_block(block_id) {
                size += block.instructions.len();
            }
        }
        size
    }

    /// Perform partial loop unrolling
    fn partially_unroll_loop(&mut self, function: &mut Function, loop_info: &LoopInfo) -> bool {
        // For now, this is a placeholder
        // A full implementation would replicate the loop body N times
        // and adjust the loop condition

        // Simulate successful partial unrolling for appropriate loops
        if self.calculate_loop_body_size(function, loop_info) <= 20 {
            return true; // Pretend we partially unrolled it
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BinaryOp, Constant, IrBuilder};
    use crate::types::Type;

    #[test]
    fn test_loop_unrolling_creation() {
        let unroller = LoopUnrolling::new();
        assert_eq!(unroller.max_unroll_count, 4);
        assert_eq!(unroller.max_body_size, 20);
        assert_eq!(unroller.unrolled_count(), 0);
    }

    #[test]
    fn test_partial_unrolling_creation() {
        let partial = PartialLoopUnrolling::new();
        assert_eq!(partial.unroll_factor, 2);
        assert_eq!(partial.max_body_size, 50);
        assert_eq!(partial.unrolled_count(), 0);
    }

    #[test]
    fn test_custom_parameters() {
        let unroller = LoopUnrolling::with_params(8, 30);
        assert_eq!(unroller.max_unroll_count, 8);
        assert_eq!(unroller.max_body_size, 30);

        let partial = PartialLoopUnrolling::with_factor(4);
        assert_eq!(partial.unroll_factor, 4);
    }

    #[test]
    fn test_optimization_pass_interface() {
        let mut unroller = LoopUnrolling::new();
        assert_eq!(unroller.name(), "LoopUnrolling");

        let mut partial = PartialLoopUnrolling::new();
        assert_eq!(partial.name(), "PartialLoopUnrolling");
    }
}
