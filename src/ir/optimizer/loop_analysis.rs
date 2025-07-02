//! Loop analysis and detection for optimization passes
//!
//! This module identifies loops in the control flow graph and collects
//! information about them for use by optimization passes.

use crate::ir::{BlockId, Function, Instruction, ValueId};
use std::collections::{HashMap, HashSet, VecDeque};

/// Information about a single loop
#[derive(Debug, Clone)]
pub struct LoopInfo {
    /// Loop header block (entry point)
    pub header: BlockId,
    /// Blocks that are part of the loop body
    pub body: HashSet<BlockId>,
    /// Back-edge blocks (blocks that jump back to header)
    pub back_edges: Vec<BlockId>,
    /// Loop exit blocks
    pub exits: Vec<BlockId>,
    /// Parent loop (if this is a nested loop)
    pub parent: Option<usize>,
    /// Child loops (nested within this loop)
    pub children: Vec<usize>,
    /// Loop induction variables
    pub induction_vars: Vec<InductionVariable>,
    /// Estimated iteration count (if known)
    pub iteration_count: Option<usize>,
}

/// Information about a loop induction variable
#[derive(Debug, Clone)]
pub struct InductionVariable {
    /// The PHI node that defines the induction variable
    pub phi_value: ValueId,
    /// Initial value
    pub init_value: ValueId,
    /// Step value (how much it increments each iteration)
    pub step_value: ValueId,
    /// The block containing the increment
    pub increment_block: BlockId,
}

/// Loop analyzer that finds and analyzes loops in functions
pub struct LoopAnalyzer {
    /// Dominance information
    dominators: HashMap<BlockId, HashSet<BlockId>>,
    /// Immediate dominator for each block
    idom: HashMap<BlockId, BlockId>,
    /// Dominator tree children
    dom_tree: HashMap<BlockId, Vec<BlockId>>,
}

impl LoopAnalyzer {
    /// Create a new loop analyzer
    pub fn new() -> Self {
        LoopAnalyzer {
            dominators: HashMap::new(),
            idom: HashMap::new(),
            dom_tree: HashMap::new(),
        }
    }

    /// Analyze loops in a function
    pub fn analyze_function(&mut self, func: &Function) -> Vec<LoopInfo> {
        // Step 1: Build dominance information
        self.compute_dominators(func);

        // Step 2: Find natural loops
        let mut loops = self.find_natural_loops(func);

        // Step 3: Analyze loop properties
        for loop_info in &mut loops {
            self.analyze_loop_properties(func, loop_info);
        }

        // Step 4: Build loop nesting hierarchy
        self.build_loop_hierarchy(&mut loops);

        loops
    }

    /// Compute dominator sets for all blocks
    fn compute_dominators(&mut self, func: &Function) {
        let blocks: Vec<_> = func.blocks().keys().copied().collect();
        let entry = func.entry_block.expect("Function must have entry block");

        // Initialize dominators
        self.dominators.clear();
        self.dominators.insert(entry, HashSet::from([entry]));

        for &block in &blocks {
            if block != entry {
                // Initially, each block is dominated by all blocks
                self.dominators
                    .insert(block, blocks.iter().copied().collect());
            }
        }

        // Iteratively compute dominators
        let mut changed = true;
        while changed {
            changed = false;

            for &block in &blocks {
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
                let mut new_doms = self.dominators.get(&preds[0]).cloned().unwrap_or_default();

                for &pred in &preds[1..] {
                    if let Some(pred_doms) = self.dominators.get(&pred) {
                        new_doms = new_doms.intersection(pred_doms).copied().collect();
                    }
                }

                new_doms.insert(block);

                if let Some(old_doms) = self.dominators.get(&block) {
                    if &new_doms != old_doms {
                        changed = true;
                        self.dominators.insert(block, new_doms);
                    }
                } else {
                    changed = true;
                    self.dominators.insert(block, new_doms);
                }
            }
        }

        // Compute immediate dominators
        self.compute_immediate_dominators(&blocks);

        // Build dominator tree
        self.build_dominator_tree(&blocks);
    }

    /// Compute immediate dominators from dominator sets
    fn compute_immediate_dominators(&mut self, blocks: &[BlockId]) {
        self.idom.clear();

        for &block in blocks {
            if let Some(doms) = self.dominators.get(&block) {
                // Find immediate dominator (closest dominator)
                let mut idom_candidate = None;
                let mut min_dom_count = usize::MAX;

                for &dom in doms {
                    if dom == block {
                        continue;
                    }

                    if let Some(dom_doms) = self.dominators.get(&dom) {
                        if dom_doms.len() < min_dom_count {
                            min_dom_count = dom_doms.len();
                            idom_candidate = Some(dom);
                        }
                    }
                }

                if let Some(idom) = idom_candidate {
                    self.idom.insert(block, idom);
                }
            }
        }
    }

    /// Build dominator tree from immediate dominators
    fn build_dominator_tree(&mut self, blocks: &[BlockId]) {
        self.dom_tree.clear();

        for &block in blocks {
            if let Some(&idom) = self.idom.get(&block) {
                self.dom_tree
                    .entry(idom)
                    .or_insert_with(Vec::new)
                    .push(block);
            }
        }
    }

    /// Find natural loops using back edges
    fn find_natural_loops(&self, func: &Function) -> Vec<LoopInfo> {
        let mut loops = Vec::new();
        let mut back_edges = Vec::new();

        // Find back edges (edges from B to A where A dominates B)
        for (_, block) in func.blocks() {
            for &succ in &block.successors {
                if self.dominates(succ, block.id) {
                    back_edges.push((block.id, succ));
                }
            }
        }

        // For each back edge, find the natural loop
        for (tail, header) in back_edges {
            let mut loop_body = HashSet::new();
            loop_body.insert(header);
            loop_body.insert(tail);

            // Find all blocks in the loop using BFS from tail
            let mut queue = VecDeque::new();
            queue.push_back(tail);

            while let Some(block) = queue.pop_front() {
                if let Some(b) = func.get_block(block) {
                    for &pred in &b.predecessors {
                        if !loop_body.contains(&pred) {
                            loop_body.insert(pred);
                            if pred != header {
                                queue.push_back(pred);
                            }
                        }
                    }
                }
            }

            // Find exit blocks
            let mut exits = Vec::new();
            for &block in &loop_body {
                if let Some(b) = func.get_block(block) {
                    for &succ in &b.successors {
                        if !loop_body.contains(&succ) {
                            exits.push(block);
                            break;
                        }
                    }
                }
            }

            loops.push(LoopInfo {
                header,
                body: loop_body,
                back_edges: vec![tail],
                exits,
                parent: None,
                children: Vec::new(),
                induction_vars: Vec::new(),
                iteration_count: None,
            });
        }

        loops
    }

    /// Check if block A dominates block B
    fn dominates(&self, a: BlockId, b: BlockId) -> bool {
        self.dominators
            .get(&b)
            .map(|doms| doms.contains(&a))
            .unwrap_or(false)
    }

    /// Analyze properties of a loop
    fn analyze_loop_properties(&self, func: &Function, loop_info: &mut LoopInfo) {
        // Find induction variables
        self.find_induction_variables(func, loop_info);

        // Try to determine iteration count
        self.analyze_iteration_count(func, loop_info);
    }

    /// Find induction variables in the loop
    fn find_induction_variables(&self, func: &Function, loop_info: &mut LoopInfo) {
        if let Some(header_block) = func.get_block(loop_info.header) {
            // Look for PHI nodes in the header
            for (value_id, inst_with_loc) in &header_block.instructions {
                if let Instruction::Phi { incoming, .. } = &inst_with_loc.instruction {
                    // Check if this is an induction variable pattern
                    if incoming.len() == 2 {
                        // Find which incoming value is from outside the loop
                        let mut init_value = None;
                        let mut loop_value = None;
                        let mut loop_block = None;

                        for &(val, block) in incoming {
                            if loop_info.body.contains(&block) {
                                loop_value = Some(val);
                                loop_block = Some(block);
                            } else {
                                init_value = Some(val);
                            }
                        }

                        // Check if the loop value is an increment of the PHI
                        if let (Some(init), Some(loop_val), Some(block)) =
                            (init_value, loop_value, loop_block)
                        {
                            if self.is_simple_increment(func, *value_id, loop_val, block) {
                                loop_info.induction_vars.push(InductionVariable {
                                    phi_value: *value_id,
                                    init_value: init,
                                    step_value: loop_val,
                                    increment_block: block,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    /// Check if a value is a simple increment of another value
    fn is_simple_increment(
        &self,
        func: &Function,
        _base: ValueId,
        value: ValueId,
        block: BlockId,
    ) -> bool {
        if let Some(b) = func.get_block(block) {
            for (vid, inst_with_loc) in &b.instructions {
                if *vid == value {
                    if let Instruction::Binary {
                        op: crate::ir::BinaryOp::Add,
                        ..
                    } = &inst_with_loc.instruction
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Try to determine the iteration count of a loop
    fn analyze_iteration_count(&self, func: &Function, loop_info: &mut LoopInfo) {
        // Simple analysis: look for loops with constant bounds
        // This is a simplified version - real compilers would do more sophisticated analysis

        // Check if we have a single exit with a comparison
        if loop_info.exits.len() == 1 {
            let exit_block = loop_info.exits[0];
            if let Some(block) = func.get_block(exit_block) {
                if let Some(term) = block.terminator() {
                    if let Instruction::CondBranch { condition, .. } = term {
                        // Try to analyze the condition
                        // This is where we'd check for patterns like i < N
                        // For now, we'll leave this unimplemented
                        let _ = condition;
                    }
                }
            }
        }
    }

    /// Build the loop nesting hierarchy
    fn build_loop_hierarchy(&self, loops: &mut Vec<LoopInfo>) {
        let n = loops.len();

        for i in 0..n {
            for j in 0..n {
                if i != j {
                    // Check if loop i is nested inside loop j
                    let loop_i_header = loops[i].header;
                    if loops[j].body.contains(&loop_i_header) {
                        // Check if this is the immediate parent
                        let mut is_immediate_parent = true;
                        for k in 0..n {
                            if k != i && k != j {
                                if loops[k].body.contains(&loop_i_header)
                                    && loops[j].body.contains(&loops[k].header)
                                {
                                    is_immediate_parent = false;
                                    break;
                                }
                            }
                        }

                        if is_immediate_parent {
                            loops[i].parent = Some(j);
                            loops[j].children.push(i);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BinaryOp, ComparisonOp, Constant, IrBuilder};
    use crate::types::Type;

    #[test]
    fn test_simple_loop_detection() {
        let mut builder = IrBuilder::new();

        // Create a function with a simple loop
        let func_id = builder.create_function("test_loop".to_string(), vec![], Type::I32);

        // Create blocks
        let entry = builder.get_current_block().unwrap();
        let loop_header = builder.create_block("loop_header".to_string()).unwrap();
        let loop_body = builder.create_block("loop_body".to_string()).unwrap();
        let exit = builder.create_block("exit".to_string()).unwrap();

        // Entry block: initialize i = 0
        builder.set_current_block(entry);
        let zero = builder.const_value(Constant::I32(0));
        builder.build_branch(loop_header);

        // Loop header: phi node and condition
        builder.set_current_block(loop_header);
        let i = builder
            .add_instruction(Instruction::Phi {
                incoming: vec![(zero, entry), (ValueId(99), loop_body)], // Placeholder for increment
                ty: Type::I32,
            })
            .unwrap();
        let limit = builder.const_value(Constant::I32(10));
        let cond = builder.build_compare(ComparisonOp::Lt, i, limit).unwrap();
        builder.build_cond_branch(cond, loop_body, exit);

        // Loop body: increment i
        builder.set_current_block(loop_body);
        let one = builder.const_value(Constant::I32(1));
        let inc = builder
            .build_binary(BinaryOp::Add, i, one, Type::I32)
            .unwrap();
        builder.build_branch(loop_header);

        // Exit block
        builder.set_current_block(exit);
        builder.build_return(Some(i));

        // Analyze loops
        let module = builder.build();
        if let Some(func) = module.get_function(func_id) {
            let mut analyzer = LoopAnalyzer::new();
            let loops = analyzer.analyze_function(func);

            assert_eq!(loops.len(), 1);
            assert_eq!(loops[0].header, loop_header);
            assert!(loops[0].body.contains(&loop_header));
            assert!(loops[0].body.contains(&loop_body));
            assert_eq!(loops[0].exits, vec![loop_header]);
        }
    }
}
