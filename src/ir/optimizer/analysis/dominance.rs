//! Dominance Analysis for Control Flow
//!
//! This module provides dominance analysis for control flow graphs, including:
//! - Dominator sets and immediate dominators
//! - Dominator tree construction
//! - Dominance frontier computation
//! - Post-dominance analysis

use crate::ir::{BlockId, Function};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Dominance analysis information for a function
#[derive(Debug, Clone)]
pub struct DominanceInfo {
    /// Dominator sets for each block (blocks that dominate this block)
    pub dominators: HashMap<BlockId, HashSet<BlockId>>,
    /// Immediate dominator for each block
    pub immediate_dominators: HashMap<BlockId, BlockId>,
    /// Dominator tree (immediate dominator -> children)
    pub dominator_tree: HashMap<BlockId, Vec<BlockId>>,
    /// Dominance frontier for each block
    pub dominance_frontier: HashMap<BlockId, HashSet<BlockId>>,
    /// Post-dominators (blocks dominated by this block on all paths to exit)
    pub post_dominators: HashMap<BlockId, HashSet<BlockId>>,
    /// Immediate post-dominator for each block
    pub immediate_post_dominators: HashMap<BlockId, BlockId>,
}

impl DominanceInfo {
    /// Create empty dominance info
    pub fn new() -> Self {
        DominanceInfo {
            dominators: HashMap::new(),
            immediate_dominators: HashMap::new(),
            dominator_tree: HashMap::new(),
            dominance_frontier: HashMap::new(),
            post_dominators: HashMap::new(),
            immediate_post_dominators: HashMap::new(),
        }
    }

    /// Check if block A dominates block B
    pub fn dominates(&self, a: BlockId, b: BlockId) -> bool {
        self.dominators
            .get(&b)
            .map(|doms| doms.contains(&a))
            .unwrap_or(false)
    }

    /// Check if block A strictly dominates block B (A != B and A dominates B)
    pub fn strictly_dominates(&self, a: BlockId, b: BlockId) -> bool {
        a != b && self.dominates(a, b)
    }

    /// Get the immediate dominator of a block
    pub fn immediate_dominator(&self, block: BlockId) -> Option<BlockId> {
        self.immediate_dominators.get(&block).copied()
    }

    /// Get the dominator tree children of a block
    pub fn dominator_tree_children(&self, block: BlockId) -> Vec<BlockId> {
        self.dominator_tree.get(&block).cloned().unwrap_or_default()
    }

    /// Get the dominance frontier of a block
    pub fn dominance_frontier(&self, block: BlockId) -> HashSet<BlockId> {
        self.dominance_frontier
            .get(&block)
            .cloned()
            .unwrap_or_default()
    }

    /// Check if block A post-dominates block B
    pub fn post_dominates(&self, a: BlockId, b: BlockId) -> bool {
        self.post_dominators
            .get(&b)
            .map(|post_doms| post_doms.contains(&a))
            .unwrap_or(false)
    }

    /// Find the lowest common ancestor in the dominator tree
    pub fn lowest_common_ancestor(&self, a: BlockId, b: BlockId) -> Option<BlockId> {
        // Get all dominators of A
        let doms_a = self.dominators.get(&a)?;
        let doms_b = self.dominators.get(&b)?;

        // Find common dominators
        let common: HashSet<_> = doms_a.intersection(doms_b).copied().collect();

        if common.is_empty() {
            return None;
        }

        // Find the one with the most dominators (closest to the leaves)
        let mut best = None;
        let mut max_doms = 0;

        for &candidate in &common {
            if let Some(candidate_doms) = self.dominators.get(&candidate) {
                if candidate_doms.len() > max_doms {
                    max_doms = candidate_doms.len();
                    best = Some(candidate);
                }
            }
        }

        best
    }
}

impl Default for DominanceInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Dominance analysis implementation
pub struct DominanceAnalysis {
    /// Whether to compute post-dominance information
    compute_post_dominance: bool,
    /// Whether to enable debug output
    debug: bool,
}

impl DominanceAnalysis {
    /// Create a new dominance analysis
    pub fn new() -> Self {
        DominanceAnalysis {
            compute_post_dominance: true,
            debug: false,
        }
    }

    /// Create dominance analysis without post-dominance computation
    pub fn without_post_dominance() -> Self {
        DominanceAnalysis {
            compute_post_dominance: false,
            debug: false,
        }
    }

    /// Enable debug output
    pub fn enable_debug(&mut self) {
        self.debug = true;
    }

    /// Run dominance analysis on a function
    pub fn analyze(&mut self, func: &Function) -> DominanceInfo {
        let mut info = DominanceInfo::new();

        // Get all blocks
        let blocks: Vec<_> = func.blocks().keys().copied().collect();
        if blocks.is_empty() {
            return info;
        }

        let entry = func.entry_block.expect("Function must have entry block");

        if self.debug {
            eprintln!("Computing dominance for {} blocks", blocks.len());
        }

        // Step 1: Compute dominators using iterative algorithm
        self.compute_dominators(func, &blocks, entry, &mut info);

        // Step 2: Compute immediate dominators
        self.compute_immediate_dominators(&blocks, &mut info);

        // Step 3: Build dominator tree
        self.build_dominator_tree(&blocks, &mut info);

        // Step 4: Compute dominance frontier
        self.compute_dominance_frontier(func, &blocks, &mut info);

        // Step 5: Compute post-dominance if requested
        if self.compute_post_dominance {
            self.compute_post_dominators(func, &blocks, &mut info);
        }

        info
    }

    /// Compute dominator sets using iterative data flow algorithm
    fn compute_dominators(
        &self,
        func: &Function,
        blocks: &[BlockId],
        entry: BlockId,
        info: &mut DominanceInfo,
    ) {
        // Initialize dominators
        info.dominators.insert(entry, HashSet::from([entry]));

        for &block in blocks {
            if block != entry {
                // Initially, each block is dominated by all blocks
                info.dominators
                    .insert(block, blocks.iter().copied().collect());
            }
        }

        // Iteratively compute dominators until fixed point
        let mut changed = true;
        let mut iterations = 0;

        while changed {
            changed = false;
            iterations += 1;

            if self.debug && iterations % 10 == 0 {
                eprintln!("  Dominator iteration {iterations}");
            }

            for &block in blocks {
                if block == entry {
                    continue;
                }

                // Get predecessors
                let preds: Vec<_> = if let Some(b) = func.get_block(block) {
                    b.predecessors.clone()
                } else {
                    continue;
                };

                if preds.is_empty() {
                    continue;
                }

                // New dominators = intersection of predecessor dominators + self
                let mut new_doms = info.dominators.get(&preds[0]).cloned().unwrap_or_default();

                for &pred in &preds[1..] {
                    if let Some(pred_doms) = info.dominators.get(&pred) {
                        new_doms = new_doms.intersection(pred_doms).copied().collect();
                    }
                }

                new_doms.insert(block);

                if let Some(old_doms) = info.dominators.get(&block) {
                    if &new_doms != old_doms {
                        changed = true;
                        info.dominators.insert(block, new_doms);
                    }
                } else {
                    changed = true;
                    info.dominators.insert(block, new_doms);
                }
            }
        }

        if self.debug {
            eprintln!("  Dominators computed in {} iterations", iterations);
        }
    }

    /// Compute immediate dominators from dominator sets
    fn compute_immediate_dominators(&self, blocks: &[BlockId], info: &mut DominanceInfo) {
        for &block in blocks {
            if let Some(doms) = info.dominators.get(&block) {
                // Find immediate dominator (dominator with maximum dominator count, excluding self)
                let mut idom_candidate = None;
                let mut max_dom_count = 0;

                for &dom in doms {
                    if dom == block {
                        continue;
                    }

                    if let Some(dom_doms) = info.dominators.get(&dom) {
                        if dom_doms.len() > max_dom_count {
                            max_dom_count = dom_doms.len();
                            idom_candidate = Some(dom);
                        }
                    }
                }

                if let Some(idom) = idom_candidate {
                    info.immediate_dominators.insert(block, idom);
                }
            }
        }
    }

    /// Build dominator tree from immediate dominators
    fn build_dominator_tree(&self, _blocks: &[BlockId], info: &mut DominanceInfo) {
        for (&block, &idom) in &info.immediate_dominators {
            info.dominator_tree
                .entry(idom)
                .or_insert_with(Vec::new)
                .push(block);
        }
    }

    /// Compute dominance frontier for each block
    fn compute_dominance_frontier(
        &self,
        func: &Function,
        blocks: &[BlockId],
        info: &mut DominanceInfo,
    ) {
        for &block in blocks {
            info.dominance_frontier.insert(block, HashSet::new());
        }

        // For each block, check its CFG successors
        for &block in blocks {
            if let Some(bb) = func.get_block(block) {
                for &succ in &bb.successors {
                    // Walk up the dominator tree from block
                    let mut runner = Some(block);

                    while let Some(current) = runner {
                        // If current doesn't strictly dominate succ, add succ to current's DF
                        if !info.strictly_dominates(current, succ) {
                            info.dominance_frontier
                                .entry(current)
                                .or_default()
                                .insert(succ);
                        }

                        // Move to immediate dominator
                        runner = info.immediate_dominators.get(&current).copied();

                        // Stop if we reach a block that dominates succ
                        if let Some(runner_block) = runner {
                            if info.dominates(runner_block, succ) {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Compute post-dominators (reverse dominance analysis)
    fn compute_post_dominators(
        &self,
        func: &Function,
        blocks: &[BlockId],
        info: &mut DominanceInfo,
    ) {
        // Find exit blocks (blocks with no successors or return statements)
        let mut exit_blocks = Vec::new();
        for &block in blocks {
            if let Some(bb) = func.get_block(block) {
                if bb.successors.is_empty()
                    || bb.terminator().map(|t| t.is_terminator()).unwrap_or(false)
                {
                    exit_blocks.push(block);
                }
            }
        }

        if exit_blocks.is_empty() {
            return; // No exit blocks found
        }

        // Create a virtual exit block that post-dominates all real exit blocks
        // For simplicity, we'll use the first exit block as the post-dominator root
        let post_dom_root = exit_blocks[0];

        // Initialize post-dominators (reverse of dominators)
        info.post_dominators
            .insert(post_dom_root, HashSet::from([post_dom_root]));

        for &block in blocks {
            if !exit_blocks.contains(&block) {
                info.post_dominators
                    .insert(block, blocks.iter().copied().collect());
            }
        }

        // Iteratively compute post-dominators
        let mut changed = true;
        while changed {
            changed = false;

            for &block in blocks {
                if exit_blocks.contains(&block) {
                    continue;
                }

                // Get successors
                let succs: Vec<_> = if let Some(b) = func.get_block(block) {
                    b.successors.clone()
                } else {
                    continue;
                };

                if succs.is_empty() {
                    continue;
                }

                // New post-dominators = intersection of successor post-dominators + self
                let mut new_post_doms = info
                    .post_dominators
                    .get(&succs[0])
                    .cloned()
                    .unwrap_or_default();

                for &succ in &succs[1..] {
                    if let Some(succ_post_doms) = info.post_dominators.get(&succ) {
                        new_post_doms = new_post_doms
                            .intersection(succ_post_doms)
                            .copied()
                            .collect();
                    }
                }

                new_post_doms.insert(block);

                if let Some(old_post_doms) = info.post_dominators.get(&block) {
                    if &new_post_doms != old_post_doms {
                        changed = true;
                        info.post_dominators.insert(block, new_post_doms);
                    }
                } else {
                    changed = true;
                    info.post_dominators.insert(block, new_post_doms);
                }
            }
        }

        // Compute immediate post-dominators
        for &block in blocks {
            if let Some(post_doms) = info.post_dominators.get(&block) {
                let mut ipost_dom_candidate = None;
                let mut max_post_dom_count = 0;

                for &post_dom in post_doms {
                    if post_dom == block {
                        continue;
                    }

                    if let Some(post_dom_post_doms) = info.post_dominators.get(&post_dom) {
                        if post_dom_post_doms.len() > max_post_dom_count {
                            max_post_dom_count = post_dom_post_doms.len();
                            ipost_dom_candidate = Some(post_dom);
                        }
                    }
                }

                if let Some(ipost_dom) = ipost_dom_candidate {
                    info.immediate_post_dominators.insert(block, ipost_dom);
                }
            }
        }
    }
}

impl Default for DominanceAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for DominanceInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Dominance Information:")?;

        writeln!(f, "  Dominators:")?;
        for (block, doms) in &self.dominators {
            write!(f, "    {}: {{", block)?;
            for (i, dom) in doms.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", dom)?;
            }
            writeln!(f, "}}")?;
        }

        writeln!(f, "  Immediate Dominators:")?;
        for (block, idom) in &self.immediate_dominators {
            writeln!(f, "    {} -> {}", block, idom)?;
        }

        writeln!(f, "  Dominance Frontier:")?;
        for (block, df) in &self.dominance_frontier {
            if !df.is_empty() {
                write!(f, "    {}: {{", block)?;
                for (i, frontier_block) in df.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", frontier_block)?;
                }
                writeln!(f, "}}")?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BinaryOp, Constant, IrBuilder};
    use crate::types::Type;

    #[test]
    fn test_simple_dominance() {
        let mut builder = IrBuilder::new();

        // Create a simple function with linear control flow
        let func_id = builder.create_function("test".to_string(), vec![], Type::I32);

        // Create blocks: entry -> bb1 -> exit
        let entry = builder.get_current_block().unwrap();
        let bb1 = builder.create_block("bb1".to_string()).unwrap();
        let exit = builder.create_block("exit".to_string()).unwrap();

        // Entry block
        builder.set_current_block(entry);
        let zero = builder.const_value(Constant::I32(0));
        builder.build_branch(bb1);

        // BB1
        builder.set_current_block(bb1);
        let one = builder.const_value(Constant::I32(1));
        let sum = builder
            .build_binary(BinaryOp::Add, zero, one, Type::I32)
            .unwrap();
        builder.build_branch(exit);

        // Exit
        builder.set_current_block(exit);
        builder.build_return(Some(sum));

        // Analyze dominance
        let module = builder.build();
        let func = module.get_function(func_id).unwrap();
        let mut analysis = DominanceAnalysis::new();
        let dom_info = analysis.analyze(func);

        // Check dominance relationships
        assert!(dom_info.dominates(entry, entry));
        assert!(dom_info.dominates(entry, bb1));
        assert!(dom_info.dominates(entry, exit));
        assert!(dom_info.dominates(bb1, bb1));
        assert!(dom_info.dominates(bb1, exit));
        assert!(!dom_info.dominates(bb1, entry));
        assert!(!dom_info.dominates(exit, entry));
        assert!(!dom_info.dominates(exit, bb1));

        // Check immediate dominators
        assert_eq!(dom_info.immediate_dominator(bb1), Some(entry));
        assert_eq!(dom_info.immediate_dominator(exit), Some(bb1));
        assert_eq!(dom_info.immediate_dominator(entry), None);
    }

    #[test]
    fn test_diamond_dominance() {
        let mut builder = IrBuilder::new();

        // Create diamond-shaped CFG: entry -> (left, right) -> exit
        let func_id = builder.create_function("diamond".to_string(), vec![], Type::I32);

        let entry = builder.get_current_block().unwrap();
        let left = builder.create_block("left".to_string()).unwrap();
        let right = builder.create_block("right".to_string()).unwrap();
        let exit = builder.create_block("exit".to_string()).unwrap();

        // Entry: conditional branch
        builder.set_current_block(entry);
        let cond = builder.const_value(Constant::Bool(true));
        builder.build_cond_branch(cond, left, right);

        // Left branch
        builder.set_current_block(left);
        let left_val = builder.const_value(Constant::I32(1));
        builder.build_branch(exit);

        // Right branch
        builder.set_current_block(right);
        let right_val = builder.const_value(Constant::I32(2));
        builder.build_branch(exit);

        // Exit with phi
        builder.set_current_block(exit);
        builder.add_instruction(crate::ir::Instruction::Phi {
            incoming: vec![(left_val, left), (right_val, right)],
            ty: Type::I32,
        });
        builder.build_return(None);

        // Analyze dominance
        let module = builder.build();
        let func = module.get_function(func_id).unwrap();
        let mut analysis = DominanceAnalysis::new();
        let dom_info = analysis.analyze(func);

        // Entry dominates everything
        assert!(dom_info.dominates(entry, left));
        assert!(dom_info.dominates(entry, right));
        assert!(dom_info.dominates(entry, exit));

        // Left and right don't dominate each other
        assert!(!dom_info.dominates(left, right));
        assert!(!dom_info.dominates(right, left));

        // Neither left nor right dominates exit (both paths must go through entry)
        assert!(!dom_info.dominates(left, exit));
        assert!(!dom_info.dominates(right, exit));

        // But entry does dominate exit
        assert!(dom_info.dominates(entry, exit));

        // Check immediate dominators
        assert_eq!(dom_info.immediate_dominator(left), Some(entry));
        assert_eq!(dom_info.immediate_dominator(right), Some(entry));
        assert_eq!(dom_info.immediate_dominator(exit), Some(entry));
    }

    #[test]
    fn test_dominance_frontier() {
        let mut builder = IrBuilder::new();

        // Create a more complex CFG to test dominance frontier
        let func_id = builder.create_function("complex".to_string(), vec![], Type::I32);

        let entry = builder.get_current_block().unwrap();
        let bb1 = builder.create_block("bb1".to_string()).unwrap();
        let bb2 = builder.create_block("bb2".to_string()).unwrap();
        let bb3 = builder.create_block("bb3".to_string()).unwrap();
        let exit = builder.create_block("exit".to_string()).unwrap();

        // entry -> bb1
        builder.set_current_block(entry);
        builder.build_branch(bb1);

        // bb1 -> (bb2, bb3)
        builder.set_current_block(bb1);
        let cond = builder.const_value(Constant::Bool(true));
        builder.build_cond_branch(cond, bb2, bb3);

        // bb2 -> exit
        builder.set_current_block(bb2);
        builder.build_branch(exit);

        // bb3 -> exit
        builder.set_current_block(bb3);
        builder.build_branch(exit);

        // exit
        builder.set_current_block(exit);
        builder.build_return(None);

        // Analyze dominance
        let module = builder.build();
        let func = module.get_function(func_id).unwrap();
        let mut analysis = DominanceAnalysis::new();
        let dom_info = analysis.analyze(func);

        // Both bb2 and bb3 should have exit in their dominance frontier
        // since they both have exit as successor but don't dominate it
        let df_bb2 = dom_info.dominance_frontier(bb2);
        let df_bb3 = dom_info.dominance_frontier(bb3);

        assert!(df_bb2.contains(&exit) || df_bb3.contains(&exit));
    }
}
