//! Control Flow Graph Analysis
//!
//! This module provides control flow graph construction and analysis utilities
//! for IR optimization passes.

use crate::ir::{BasicBlock, BlockId, Function, Instruction};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

/// Control flow graph edge type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CfgEdgeType {
    /// Unconditional control flow
    Unconditional,
    /// Conditional control flow (true branch)
    ConditionalTrue,
    /// Conditional control flow (false branch)
    ConditionalFalse,
    /// Exception/error handling edge
    Exception,
}

/// Edge in the control flow graph
#[derive(Debug, Clone)]
pub struct CfgEdge {
    /// Source block
    pub from: BlockId,
    /// Target block
    pub to: BlockId,
    /// Type of edge
    pub edge_type: CfgEdgeType,
    /// Edge weight (for optimization heuristics)
    pub weight: f32,
}

impl CfgEdge {
    /// Create a new CFG edge
    pub fn new(from: BlockId, to: BlockId, edge_type: CfgEdgeType) -> Self {
        CfgEdge {
            from,
            to,
            edge_type,
            weight: 1.0,
        }
    }

    /// Create an unconditional edge
    pub fn unconditional(from: BlockId, to: BlockId) -> Self {
        Self::new(from, to, CfgEdgeType::Unconditional)
    }

    /// Create a conditional true edge
    pub fn conditional_true(from: BlockId, to: BlockId) -> Self {
        Self::new(from, to, CfgEdgeType::ConditionalTrue)
    }

    /// Create a conditional false edge
    pub fn conditional_false(from: BlockId, to: BlockId) -> Self {
        Self::new(from, to, CfgEdgeType::ConditionalFalse)
    }
}

/// Node in the control flow graph
#[derive(Debug, Clone)]
pub struct CfgNode {
    /// Block ID
    pub block_id: BlockId,
    /// Incoming edges
    pub incoming: Vec<CfgEdge>,
    /// Outgoing edges
    pub outgoing: Vec<CfgEdge>,
    /// Whether this is an entry node
    pub is_entry: bool,
    /// Whether this is an exit node (has no successors)
    pub is_exit: bool,
    /// Loop depth (0 = not in loop, 1 = one level deep, etc.)
    pub loop_depth: usize,
}

impl CfgNode {
    /// Create a new CFG node
    pub fn new(block_id: BlockId) -> Self {
        CfgNode {
            block_id,
            incoming: Vec::new(),
            outgoing: Vec::new(),
            is_entry: false,
            is_exit: false,
            loop_depth: 0,
        }
    }

    /// Get predecessor blocks
    pub fn predecessors(&self) -> Vec<BlockId> {
        self.incoming.iter().map(|edge| edge.from).collect()
    }

    /// Get successor blocks
    pub fn successors(&self) -> Vec<BlockId> {
        self.outgoing.iter().map(|edge| edge.to).collect()
    }

    /// Check if this node has multiple predecessors
    pub fn has_multiple_predecessors(&self) -> bool {
        self.incoming.len() > 1
    }

    /// Check if this node has multiple successors
    pub fn has_multiple_successors(&self) -> bool {
        self.outgoing.len() > 1
    }
}

/// Control Flow Graph representation
#[derive(Debug, Clone)]
pub struct ControlFlowGraph {
    /// Nodes in the graph
    pub nodes: HashMap<BlockId, CfgNode>,
    /// Entry block
    pub entry: Option<BlockId>,
    /// Exit blocks
    pub exits: Vec<BlockId>,
    /// All edges in the graph
    pub edges: Vec<CfgEdge>,
}

impl ControlFlowGraph {
    /// Create an empty control flow graph
    pub fn new() -> Self {
        ControlFlowGraph {
            nodes: HashMap::new(),
            entry: None,
            exits: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Build a control flow graph from a function
    pub fn build(func: &Function) -> Self {
        let mut cfg = ControlFlowGraph::new();

        // Set entry block
        cfg.entry = func.entry_block;

        // Create nodes for each basic block
        for (block_id, basic_block) in func.blocks() {
            let mut node = CfgNode::new(*block_id);
            node.is_entry = Some(*block_id) == func.entry_block;

            // Check if this is an exit block
            node.is_exit = basic_block.successors.is_empty()
                || basic_block
                    .terminator()
                    .map(|t| matches!(t, Instruction::Return(_)))
                    .unwrap_or(false);

            if node.is_exit {
                cfg.exits.push(*block_id);
            }

            cfg.nodes.insert(*block_id, node);
        }

        // Create edges based on basic block successors
        for (block_id, basic_block) in func.blocks() {
            cfg.add_edges_from_block(*block_id, basic_block);
        }

        // Compute loop depths
        cfg.compute_loop_depths();

        cfg
    }

    /// Add edges from a basic block based on its terminator
    fn add_edges_from_block(&mut self, block_id: BlockId, basic_block: &BasicBlock) {
        if let Some(terminator) = basic_block.terminator() {
            match terminator {
                Instruction::Branch(target) => {
                    let edge = CfgEdge::unconditional(block_id, *target);
                    self.add_edge(edge);
                }
                Instruction::CondBranch {
                    then_block,
                    else_block,
                    ..
                } => {
                    let true_edge = CfgEdge::conditional_true(block_id, *then_block);
                    let false_edge = CfgEdge::conditional_false(block_id, *else_block);
                    self.add_edge(true_edge);
                    self.add_edge(false_edge);
                }
                Instruction::Return(_) => {
                    // No outgoing edges for return
                }
                _ => {
                    // Handle other terminators if needed
                }
            }
        }
    }

    /// Add an edge to the graph
    pub fn add_edge(&mut self, edge: CfgEdge) {
        // Add to outgoing edges of source node
        if let Some(from_node) = self.nodes.get_mut(&edge.from) {
            from_node.outgoing.push(edge.clone());
        }

        // Add to incoming edges of target node
        if let Some(to_node) = self.nodes.get_mut(&edge.to) {
            to_node.incoming.push(edge.clone());
        }

        // Add to edge list
        self.edges.push(edge);
    }

    /// Get a node by block ID
    pub fn get_node(&self, block_id: BlockId) -> Option<&CfgNode> {
        self.nodes.get(&block_id)
    }

    /// Get predecessors of a block
    pub fn predecessors(&self, block_id: BlockId) -> Vec<BlockId> {
        self.nodes
            .get(&block_id)
            .map(|node| node.predecessors())
            .unwrap_or_default()
    }

    /// Get successors of a block
    pub fn successors(&self, block_id: BlockId) -> Vec<BlockId> {
        self.nodes
            .get(&block_id)
            .map(|node| node.successors())
            .unwrap_or_default()
    }

    /// Perform depth-first search starting from the entry block
    pub fn depth_first_search(&self) -> Vec<BlockId> {
        let mut visited = HashSet::new();
        let mut result = Vec::new();

        if let Some(entry) = self.entry {
            self.dfs_visit(entry, &mut visited, &mut result);
        }

        result
    }

    /// DFS visit helper
    fn dfs_visit(
        &self,
        block_id: BlockId,
        visited: &mut HashSet<BlockId>,
        result: &mut Vec<BlockId>,
    ) {
        if visited.contains(&block_id) {
            return;
        }

        visited.insert(block_id);
        result.push(block_id);

        // Visit successors
        for successor in self.successors(block_id) {
            self.dfs_visit(successor, visited, result);
        }
    }

    /// Perform breadth-first search starting from the entry block
    pub fn breadth_first_search(&self) -> Vec<BlockId> {
        let mut visited = HashSet::new();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();

        if let Some(entry) = self.entry {
            queue.push_back(entry);
            visited.insert(entry);
        }

        while let Some(block_id) = queue.pop_front() {
            result.push(block_id);

            // Add unvisited successors to queue
            for successor in self.successors(block_id) {
                if !visited.contains(&successor) {
                    visited.insert(successor);
                    queue.push_back(successor);
                }
            }
        }

        result
    }

    /// Get blocks in reverse post-order (useful for data flow analysis)
    pub fn reverse_post_order(&self) -> Vec<BlockId> {
        let mut visited = HashSet::new();
        let mut post_order = Vec::new();

        if let Some(entry) = self.entry {
            self.post_order_visit(entry, &mut visited, &mut post_order);
        }

        post_order.reverse();
        post_order
    }

    /// Post-order visit helper
    fn post_order_visit(
        &self,
        block_id: BlockId,
        visited: &mut HashSet<BlockId>,
        result: &mut Vec<BlockId>,
    ) {
        if visited.contains(&block_id) {
            return;
        }

        visited.insert(block_id);

        // Visit successors first
        for successor in self.successors(block_id) {
            self.post_order_visit(successor, visited, result);
        }

        // Add this block after visiting successors
        result.push(block_id);
    }

    /// Compute loop depths for all nodes
    fn compute_loop_depths(&mut self) {
        // Simple implementation: detect back edges and increment depth
        // This is a simplified version - real compilers use more sophisticated algorithms

        let mut depths = HashMap::new();

        // Initialize all depths to 0
        for &block_id in self.nodes.keys() {
            depths.insert(block_id, 0);
        }

        // Find back edges (edges that go to a block that can reach the source)
        for edge in &self.edges {
            if self.can_reach(edge.to, edge.from) {
                // This is a back edge, increment depth of all blocks in the loop
                self.increment_loop_depth(edge.to, edge.from, &mut depths);
            }
        }

        // Update node depths
        for (block_id, depth) in depths {
            if let Some(node) = self.nodes.get_mut(&block_id) {
                node.loop_depth = depth;
            }
        }
    }

    /// Check if we can reach target from source
    fn can_reach(&self, source: BlockId, target: BlockId) -> bool {
        let mut visited = HashSet::new();
        self.can_reach_helper(source, target, &mut visited)
    }

    /// Helper for reachability check
    fn can_reach_helper(
        &self,
        current: BlockId,
        target: BlockId,
        visited: &mut HashSet<BlockId>,
    ) -> bool {
        if current == target {
            return true;
        }

        if visited.contains(&current) {
            return false;
        }

        visited.insert(current);

        for successor in self.successors(current) {
            if self.can_reach_helper(successor, target, visited) {
                return true;
            }
        }

        false
    }

    /// Increment loop depth for blocks in a loop
    fn increment_loop_depth(
        &self,
        header: BlockId,
        latch: BlockId,
        depths: &mut HashMap<BlockId, usize>,
    ) {
        // Simple approach: all blocks reachable from header that can reach latch
        // are in the loop
        let mut visited = HashSet::new();
        self.mark_loop_blocks(header, latch, depths, &mut visited);
    }

    /// Mark blocks in a loop and increment their depth
    fn mark_loop_blocks(
        &self,
        current: BlockId,
        latch: BlockId,
        depths: &mut HashMap<BlockId, usize>,
        visited: &mut HashSet<BlockId>,
    ) {
        if visited.contains(&current) {
            return;
        }

        visited.insert(current);

        // If this block can reach the latch, it's in the loop
        if current == latch || self.can_reach(current, latch) {
            let current_depth = depths.get(&current).unwrap_or(&0);
            depths.insert(current, current_depth + 1);
        }

        // Continue to successors
        for successor in self.successors(current) {
            self.mark_loop_blocks(successor, latch, depths, visited);
        }
    }

    /// Get all blocks that are in loops
    pub fn loop_blocks(&self) -> Vec<BlockId> {
        self.nodes
            .iter()
            .filter(|(_, node)| node.loop_depth > 0)
            .map(|(&block_id, _)| block_id)
            .collect()
    }

    /// Check if a block is in a loop
    pub fn is_in_loop(&self, block_id: BlockId) -> bool {
        self.nodes
            .get(&block_id)
            .map(|node| node.loop_depth > 0)
            .unwrap_or(false)
    }
}

impl Default for ControlFlowGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ControlFlowGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Control Flow Graph:")?;

        if let Some(entry) = self.entry {
            writeln!(f, "  Entry: {}", entry)?;
        }

        writeln!(f, "  Exits: {:?}", self.exits)?;

        writeln!(f, "  Nodes:")?;
        for (block_id, node) in &self.nodes {
            writeln!(f, "    {} (depth: {}):", block_id, node.loop_depth)?;
            writeln!(f, "      Predecessors: {:?}", node.predecessors())?;
            writeln!(f, "      Successors: {:?}", node.successors())?;
        }

        writeln!(f, "  Edges:")?;
        for edge in &self.edges {
            writeln!(f, "    {} -> {} ({:?})", edge.from, edge.to, edge.edge_type)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Constant, IrBuilder};
    use crate::types::Type;

    #[test]
    fn test_linear_cfg() {
        let mut builder = IrBuilder::new();

        // Create linear control flow: entry -> bb1 -> exit
        let func_id = builder.create_function("linear".to_string(), vec![], Type::I32);

        let entry = builder.get_current_block().unwrap();
        let bb1 = builder.create_block("bb1".to_string()).unwrap();
        let exit = builder.create_block("exit".to_string()).unwrap();

        // Entry -> bb1
        builder.set_current_block(entry);
        builder.build_branch(bb1);

        // bb1 -> exit
        builder.set_current_block(bb1);
        builder.build_branch(exit);

        // exit
        builder.set_current_block(exit);
        builder.build_return(None);

        // Build CFG
        let module = builder.build();
        let func = module.get_function(func_id).unwrap();
        let cfg = ControlFlowGraph::build(func);

        // Verify structure
        assert_eq!(cfg.entry, Some(entry));
        assert_eq!(cfg.exits, vec![exit]);
        assert_eq!(cfg.nodes.len(), 3);
        assert_eq!(cfg.edges.len(), 2);

        // Verify node properties
        let entry_node = cfg.get_node(entry).unwrap();
        assert!(entry_node.is_entry);
        assert!(!entry_node.is_exit);
        assert_eq!(entry_node.successors(), vec![bb1]);

        let exit_node = cfg.get_node(exit).unwrap();
        assert!(!exit_node.is_entry);
        assert!(exit_node.is_exit);
        assert_eq!(exit_node.predecessors(), vec![bb1]);
    }

    #[test]
    fn test_diamond_cfg() {
        let mut builder = IrBuilder::new();

        // Create diamond CFG: entry -> (left, right) -> exit
        let func_id = builder.create_function("diamond".to_string(), vec![], Type::I32);

        let entry = builder.get_current_block().unwrap();
        let left = builder.create_block("left".to_string()).unwrap();
        let right = builder.create_block("right".to_string()).unwrap();
        let exit = builder.create_block("exit".to_string()).unwrap();

        // Entry: conditional branch
        builder.set_current_block(entry);
        let cond = builder.const_value(Constant::Bool(true));
        builder.build_cond_branch(cond, left, right);

        // Left -> exit
        builder.set_current_block(left);
        builder.build_branch(exit);

        // Right -> exit
        builder.set_current_block(right);
        builder.build_branch(exit);

        // Exit
        builder.set_current_block(exit);
        builder.build_return(None);

        // Build CFG
        let module = builder.build();
        let func = module.get_function(func_id).unwrap();
        let cfg = ControlFlowGraph::build(func);

        // Verify structure
        assert_eq!(cfg.entry, Some(entry));
        assert_eq!(cfg.exits, vec![exit]);
        assert_eq!(cfg.nodes.len(), 4);
        assert_eq!(cfg.edges.len(), 4); // entry->left, entry->right, left->exit, right->exit

        // Verify entry has two successors
        let entry_node = cfg.get_node(entry).unwrap();
        assert!(entry_node.has_multiple_successors());
        let successors = entry_node.successors();
        assert!(successors.contains(&left));
        assert!(successors.contains(&right));

        // Verify exit has two predecessors
        let exit_node = cfg.get_node(exit).unwrap();
        assert!(exit_node.has_multiple_predecessors());
        let predecessors = exit_node.predecessors();
        assert!(predecessors.contains(&left));
        assert!(predecessors.contains(&right));
    }

    #[test]
    fn test_cfg_traversal() {
        let mut builder = IrBuilder::new();

        // Create simple CFG for traversal testing
        let func_id = builder.create_function("traversal".to_string(), vec![], Type::I32);

        let entry = builder.get_current_block().unwrap();
        let bb1 = builder.create_block("bb1".to_string()).unwrap();
        let bb2 = builder.create_block("bb2".to_string()).unwrap();

        // entry -> bb1 -> bb2
        builder.set_current_block(entry);
        builder.build_branch(bb1);

        builder.set_current_block(bb1);
        builder.build_branch(bb2);

        builder.set_current_block(bb2);
        builder.build_return(None);

        // Build CFG
        let module = builder.build();
        let func = module.get_function(func_id).unwrap();
        let cfg = ControlFlowGraph::build(func);

        // Test DFS
        let dfs_order = cfg.depth_first_search();
        assert_eq!(dfs_order, vec![entry, bb1, bb2]);

        // Test BFS
        let bfs_order = cfg.breadth_first_search();
        assert_eq!(bfs_order, vec![entry, bb1, bb2]);

        // Test reverse post-order
        let rpo = cfg.reverse_post_order();
        assert_eq!(rpo, vec![entry, bb1, bb2]);
    }

    #[test]
    fn test_edge_types() {
        let mut builder = IrBuilder::new();

        let func_id = builder.create_function("edge_types".to_string(), vec![], Type::I32);

        let entry = builder.get_current_block().unwrap();
        let then_block = builder.create_block("then".to_string()).unwrap();
        let else_block = builder.create_block("else".to_string()).unwrap();

        // Conditional branch
        builder.set_current_block(entry);
        let cond = builder.const_value(Constant::Bool(true));
        builder.build_cond_branch(cond, then_block, else_block);

        // Then block
        builder.set_current_block(then_block);
        builder.build_return(None);

        // Else block
        builder.set_current_block(else_block);
        builder.build_return(None);

        // Build CFG
        let module = builder.build();
        let func = module.get_function(func_id).unwrap();
        let cfg = ControlFlowGraph::build(func);

        // Check edge types
        let entry_node = cfg.get_node(entry).unwrap();
        assert_eq!(entry_node.outgoing.len(), 2);

        // One should be conditional true, one conditional false
        let edge_types: HashSet<_> = entry_node
            .outgoing
            .iter()
            .map(|edge| edge.edge_type)
            .collect();

        assert!(edge_types.contains(&CfgEdgeType::ConditionalTrue));
        assert!(edge_types.contains(&CfgEdgeType::ConditionalFalse));
    }
}
