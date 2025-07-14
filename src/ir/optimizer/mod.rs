//! IR Optimization Module
//!
//! This module implements various optimization passes for the Script language IR.
//! The optimizer transforms IR modules to improve performance while preserving semantics.

use crate::ir::Module as IrModule;

pub mod analysis;
pub mod common_subexpression_elimination;
pub mod constant_folding;
pub mod dead_code_elimination;
pub mod loop_analysis;
pub mod loop_invariant_code_motion;
pub mod loop_unrolling;

pub use analysis::AnalysisManager;
pub use common_subexpression_elimination::CommonSubexpressionElimination;
pub use constant_folding::ConstantFolding;
pub use dead_code_elimination::DeadCodeElimination;
pub use loop_analysis::LoopAnalyzer;
pub use loop_invariant_code_motion::LoopInvariantCodeMotion;
pub use loop_unrolling::{LoopUnrolling, PartialLoopUnrolling};

/// Trait for optimization passes
pub trait OptimizationPass {
    /// Run the optimization pass on the module
    /// Returns true if any changes were made
    fn optimize(&mut self, module: &mut IrModule) -> bool;

    /// Get the name of this optimization pass
    fn name(&self) -> &'static str;
}

/// Optimizer that manages and runs optimization passes
pub struct Optimizer {
    /// List of optimization passes to run
    passes: Vec<Box<dyn OptimizationPass>>,
    /// Maximum number of iterations to run passes
    max_iterations: usize,
    /// Whether to enable debug output
    debug: bool,
}

impl Optimizer {
    /// Create a new optimizer with default settings
    pub fn new() -> Self {
        Optimizer {
            passes: Vec::new(),
            max_iterations: 10,
            debug: false,
        }
    }

    /// Create an optimizer with all standard optimization passes
    pub fn with_standard_passes() -> Self {
        let mut optimizer = Self::new();
        optimizer.add_pass(Box::new(ConstantFolding::new()));
        optimizer.add_pass(Box::new(CommonSubexpressionElimination::new()));
        optimizer.add_pass(Box::new(LoopInvariantCodeMotion::new()));
        optimizer.add_pass(Box::new(LoopUnrolling::new()));
        optimizer.add_pass(Box::new(DeadCodeElimination::new())); // Run DCE last to clean up
        optimizer
    }

    /// Add an optimization pass
    pub fn add_pass(&mut self, pass: Box<dyn OptimizationPass>) {
        self.passes.push(pass);
    }

    /// Set the maximum number of iterations
    pub fn set_max_iterations(&mut self, max: usize) {
        self.max_iterations = max;
    }

    /// Enable debug output
    pub fn enable_debug(&mut self) {
        self.debug = true;
    }

    /// Run all optimization passes on the module
    /// Returns the total number of changes made
    pub fn optimize(&mut self, module: &mut IrModule) -> usize {
        let mut total_changes = 0;
        let mut iteration = 0;

        // Run passes until we reach a fixed point or max iterations
        loop {
            let mut changed = false;

            for pass in &mut self.passes {
                if self.debug {
                    eprintln!("Running optimization pass: {pass.name(}"));
                }

                if pass.optimize(module) {
                    changed = true;
                    total_changes += 1;

                    if self.debug {
                        eprintln!("  Pass {} made changes", pass.name());
                    }
                }
            }

            iteration += 1;

            // Stop if no changes were made or we've hit the iteration limit
            if !changed || iteration >= self.max_iterations {
                break;
            }
        }

        if self.debug {
            eprintln!(
                "Optimization complete: {} total changes in {} iterations",
                total_changes, iteration
            );
        }

        total_changes
    }
}

impl Default for Optimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Optimization level configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    /// No optimizations
    None,
    /// Basic optimizations (fast compilation)
    O1,
    /// Standard optimizations (balanced)
    O2,
    /// Aggressive optimizations (slower compilation, faster code)
    O3,
}

impl OptimizationLevel {
    /// Create an optimizer configured for this optimization level
    pub fn create_optimizer(self) -> Optimizer {
        match self {
            OptimizationLevel::None => Optimizer::new(),
            OptimizationLevel::O1 => {
                let mut opt = Optimizer::new();
                opt.add_pass(Box::new(ConstantFolding::new()));
                opt.set_max_iterations(3);
                opt
            }
            OptimizationLevel::O2 => {
                let mut opt = Optimizer::with_standard_passes();
                opt.set_max_iterations(5);
                opt
            }
            OptimizationLevel::O3 => {
                let mut opt = Optimizer::with_standard_passes();
                // Add aggressive loop optimizations for O3
                opt.add_pass(Box::new(PartialLoopUnrolling::new()));
                opt.set_max_iterations(10);
                opt
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimizer_creation() {
        let opt = Optimizer::new();
        assert_eq!(opt.passes.len(), 0);
        assert_eq!(opt.max_iterations, 10);

        let opt = Optimizer::with_standard_passes();
        assert_eq!(opt.passes.len(), 5); // CF, CSE, LICM, unrolling, DCE
    }

    #[test]
    fn test_optimization_levels() {
        let opt = OptimizationLevel::None.create_optimizer();
        assert_eq!(opt.passes.len(), 0);

        let opt = OptimizationLevel::O1.create_optimizer();
        assert_eq!(opt.passes.len(), 1);
        assert_eq!(opt.max_iterations, 3);

        let opt = OptimizationLevel::O2.create_optimizer();
        assert_eq!(opt.passes.len(), 5);
        assert_eq!(opt.max_iterations, 5);

        let opt = OptimizationLevel::O3.create_optimizer();
        assert_eq!(opt.passes.len(), 6); // Standard passes + partial unrolling
        assert_eq!(opt.max_iterations, 10);
    }
}
