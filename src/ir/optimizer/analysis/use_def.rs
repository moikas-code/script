//! Use-Def Chain Analysis
//!
//! This module provides use-def and def-use chain analysis for tracking
//! variable definitions and uses across the control flow graph.

use crate::ir::{BlockId, Function, Instruction, ValueId};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Information about a definition site
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DefSite {
    /// Block where the definition occurs
    pub block: BlockId,
    /// Value that is defined
    pub value: ValueId,
    /// Instruction that produces the definition
    pub instruction: String, // String representation for simplicity
}

impl DefSite {
    pub fn new(block: BlockId, value: ValueId, instruction: String) -> Self {
        DefSite {
            block,
            value,
            instruction,
        }
    }
}

/// Information about a use site
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UseSite {
    /// Block where the use occurs
    pub block: BlockId,
    /// Value that is used
    pub value: ValueId,
    /// Instruction that uses the value
    pub instruction: String, // String representation for simplicity
    /// Position within the instruction (for multiple uses)
    pub position: usize,
}

impl UseSite {
    pub fn new(block: BlockId, value: ValueId, instruction: String, position: usize) -> Self {
        UseSite {
            block,
            value,
            instruction,
            position,
        }
    }
}

/// Use-def and def-use chain information
#[derive(Debug, Clone)]
pub struct DefUseInfo {
    /// Map from use sites to their reaching definitions
    pub use_def_chains: HashMap<UseSite, Vec<DefSite>>,
    /// Map from definition sites to their uses
    pub def_use_chains: HashMap<DefSite, Vec<UseSite>>,
    /// All definition sites for each value
    pub value_definitions: HashMap<ValueId, Vec<DefSite>>,
    /// All use sites for each value
    pub value_uses: HashMap<ValueId, Vec<UseSite>>,
    /// Reaching definitions at the end of each block
    pub reaching_definitions: HashMap<BlockId, HashMap<ValueId, Vec<DefSite>>>,
}

impl DefUseInfo {
    /// Create empty def-use info
    pub fn new() -> Self {
        DefUseInfo {
            use_def_chains: HashMap::new(),
            def_use_chains: HashMap::new(),
            value_definitions: HashMap::new(),
            value_uses: HashMap::new(),
            reaching_definitions: HashMap::new(),
        }
    }

    /// Get all definitions of a value
    pub fn get_definitions(&self, value: ValueId) -> Vec<DefSite> {
        self.value_definitions
            .get(&value)
            .cloned()
            .unwrap_or_default()
    }

    /// Get all uses of a value
    pub fn get_uses(&self, value: ValueId) -> Vec<UseSite> {
        self.value_uses.get(&value).cloned().unwrap_or_default()
    }

    /// Get the definition sites that reach a use site
    pub fn get_reaching_definitions(&self, use_site: &UseSite) -> Vec<DefSite> {
        self.use_def_chains
            .get(use_site)
            .cloned()
            .unwrap_or_default()
    }

    /// Get the use sites reached by a definition
    pub fn get_reached_uses(&self, def_site: &DefSite) -> Vec<UseSite> {
        self.def_use_chains
            .get(def_site)
            .cloned()
            .unwrap_or_default()
    }

    /// Check if a value has a single definition (SSA property)
    pub fn is_single_definition(&self, value: ValueId) -> bool {
        self.get_definitions(value).len() <= 1
    }

    /// Check if a value is used
    pub fn is_used(&self, value: ValueId) -> bool {
        !self.get_uses(value).is_empty()
    }

    /// Check if a value is dead (defined but never used)
    pub fn is_dead(&self, value: ValueId) -> bool {
        !self.get_definitions(value).is_empty() && self.get_uses(value).is_empty()
    }

    /// Get all values that are defined but never used
    pub fn dead_values(&self) -> Vec<ValueId> {
        self.value_definitions
            .keys()
            .filter(|&value| self.is_dead(*value))
            .copied()
            .collect()
    }
}

impl Default for DefUseInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Use-def chain analyzer
pub struct UseDefChains {
    /// Whether to enable debug output
    debug: bool,
}

impl UseDefChains {
    /// Create a new use-def chain analyzer
    pub fn new() -> Self {
        UseDefChains { debug: false }
    }

    /// Enable debug output
    pub fn enable_debug(&mut self) {
        self.debug = true;
    }

    /// Analyze use-def chains for a function
    pub fn analyze(&mut self, func: &Function) -> DefUseInfo {
        let mut info = DefUseInfo::new();

        if self.debug {
            eprintln!("Analyzing use-def chains for function: {}", func.name);
        }

        // Step 1: Collect all definitions and uses
        self.collect_def_use_sites(func, &mut info);

        // Step 2: Compute reaching definitions
        self.compute_reaching_definitions(func, &mut info);

        // Step 3: Build use-def chains
        self.build_use_def_chains(func, &mut info);

        // Step 4: Build def-use chains
        self.build_def_use_chains(&mut info);

        info
    }

    /// Collect all definition and use sites in the function
    fn collect_def_use_sites(&self, func: &Function, info: &mut DefUseInfo) {
        for (block_id, block) in func.blocks() {
            for (value_id, inst_with_loc) in &block.instructions {
                let inst_str = format!("{}", inst_with_loc.instruction);

                // Record definition site
                if inst_with_loc.instruction.result_type().is_some() {
                    let def_site = DefSite::new(*block_id, *value_id, inst_str.clone());
                    info.value_definitions
                        .entry(*value_id)
                        .or_default()
                        .push(def_site.clone());
                }

                // Record use sites
                let used_values = self.get_used_values(&inst_with_loc.instruction);
                for (pos, used_value) in used_values.iter().enumerate() {
                    let use_site = UseSite::new(*block_id, *used_value, inst_str.clone(), pos);
                    info.value_uses
                        .entry(*used_value)
                        .or_default()
                        .push(use_site);
                }
            }
        }

        if self.debug {
            eprintln!(
                "  Found {} definitions, {} uses",
                info.value_definitions.len(),
                info.value_uses.len()
            );
        }
    }

    /// Extract values used by an instruction
    fn get_used_values(&self, instruction: &Instruction) -> Vec<ValueId> {
        let mut used = Vec::new();

        match instruction {
            Instruction::Binary { lhs, rhs, .. } => {
                used.push(*lhs);
                used.push(*rhs);
            }
            Instruction::Unary { operand, .. } => {
                used.push(*operand);
            }
            Instruction::Compare { lhs, rhs, .. } => {
                used.push(*lhs);
                used.push(*rhs);
            }
            Instruction::Cast { value, .. } => {
                used.push(*value);
            }
            Instruction::Call { args, .. } => {
                used.extend_from_slice(args);
            }
            Instruction::Load { ptr, .. } => {
                used.push(*ptr);
            }
            Instruction::Store { ptr, value } => {
                used.push(*ptr);
                used.push(*value);
            }
            Instruction::GetElementPtr { ptr, index, .. } => {
                used.push(*ptr);
                used.push(*index);
            }
            Instruction::Phi { incoming, .. } => {
                for (value, _) in incoming {
                    used.push(*value);
                }
            }
            Instruction::Return(Some(value)) => {
                used.push(*value);
            }
            Instruction::CondBranch { condition, .. } => {
                used.push(*condition);
            }
            _ => {} // No values used
        }

        used
    }

    /// Compute reaching definitions using data flow analysis
    fn compute_reaching_definitions(&self, func: &Function, info: &mut DefUseInfo) {
        // Initialize reaching definitions for each block
        let mut gen_sets: HashMap<BlockId, HashMap<ValueId, Vec<DefSite>>> = HashMap::new();
        let mut kill_sets: HashMap<BlockId, HashSet<ValueId>> = HashMap::new();

        // Compute GEN and KILL sets for each block
        for (block_id, block) in func.blocks() {
            let mut gen_set: HashMap<ValueId, Vec<DefSite>> = HashMap::new();
            let mut kill_set = HashSet::new();

            for (value_id, inst_with_loc) in &block.instructions {
                if inst_with_loc.instruction.result_type().is_some() {
                    // This instruction generates a definition
                    let def_site = DefSite::new(
                        *block_id,
                        *value_id,
                        format!("{}", inst_with_loc.instruction),
                    );
                    gen_set.entry(*value_id).or_default().push(def_site);
                    kill_set.insert(*value_id);
                }
            }

            gen_sets.insert(*block_id, gen_set);
            kill_sets.insert(*block_id, kill_set);
        }

        // Initialize IN and OUT sets
        let mut in_sets: HashMap<BlockId, HashMap<ValueId, Vec<DefSite>>> = HashMap::new();
        let mut out_sets: HashMap<BlockId, HashMap<ValueId, Vec<DefSite>>> = HashMap::new();

        for block_id in func.blocks().keys() {
            in_sets.insert(*block_id, HashMap::new());
            out_sets.insert(*block_id, HashMap::new());
        }

        // Iterative data flow analysis
        let mut changed = true;
        let mut iterations = 0;

        while changed {
            changed = false;
            iterations += 1;

            if self.debug && iterations % 10 == 0 {
                eprintln!("  Reaching definitions iteration {iterations}");
            }

            for (block_id, block) in func.blocks() {
                // IN[B] = union of OUT[P] for all predecessors P of B
                let mut new_in: HashMap<ValueId, Vec<DefSite>> = HashMap::new();

                for &pred in &block.predecessors {
                    if let Some(pred_out) = out_sets.get(&pred) {
                        for (value, defs) in pred_out {
                            new_in.entry(*value).or_default().extend(defs.clone());
                        }
                    }
                }

                // Remove duplicates
                for (_, defs) in &mut new_in {
                    defs.sort_by_key(|def| (def.block, def.value));
                    defs.dedup();
                }

                // Check if IN changed
                if let Some(old_in) = in_sets.get(block_id) {
                    if &new_in != old_in {
                        changed = true;
                    }
                } else {
                    changed = true;
                }

                in_sets.insert(*block_id, new_in.clone());

                // OUT[B] = (IN[B] - KILL[B]) union GEN[B]
                let mut new_out = new_in;

                // Remove killed definitions
                if let Some(kill_set) = kill_sets.get(block_id) {
                    for killed_value in kill_set {
                        new_out.remove(killed_value);
                    }
                }

                // Add generated definitions
                if let Some(gen_set) = gen_sets.get(block_id) {
                    for (value, defs) in gen_set {
                        new_out.entry(*value).or_default().extend(defs.clone());
                    }
                }

                // Remove duplicates
                for (_, defs) in &mut new_out {
                    defs.sort_by_key(|def| (def.block, def.value));
                    defs.dedup();
                }

                out_sets.insert(*block_id, new_out);
            }
        }

        // Store reaching definitions
        info.reaching_definitions = out_sets;

        if self.debug {
            eprintln!(
                "  Reaching definitions computed in {} iterations",
                iterations
            );
        }
    }

    /// Build use-def chains from reaching definitions
    fn build_use_def_chains(&self, func: &Function, info: &mut DefUseInfo) {
        for (block_id, block) in func.blocks() {
            // Get reaching definitions at the start of this block
            let mut current_defs: HashMap<ValueId, Vec<DefSite>> = HashMap::new();

            // Start with reaching definitions from predecessors
            for &pred in &block.predecessors {
                if let Some(pred_out) = info.reaching_definitions.get(&pred) {
                    for (value, defs) in pred_out {
                        current_defs.entry(*value).or_default().extend(defs.clone());
                    }
                }
            }

            // Process instructions in the block
            for (value_id, inst_with_loc) in &block.instructions {
                let inst_str = format!("{}", inst_with_loc.instruction);

                // For each value used by this instruction
                let used_values = self.get_used_values(&inst_with_loc.instruction);
                for (pos, used_value) in used_values.iter().enumerate() {
                    let use_site = UseSite::new(*block_id, *used_value, inst_str.clone(), pos);

                    // Find reaching definitions for this use
                    if let Some(reaching_defs) = current_defs.get(used_value) {
                        info.use_def_chains.insert(use_site, reaching_defs.clone());
                    }
                }

                // If this instruction defines a value, update current definitions
                if inst_with_loc.instruction.result_type().is_some() {
                    let def_site = DefSite::new(*block_id, *value_id, inst_str);
                    current_defs.insert(*value_id, vec![def_site]);
                }
            }
        }
    }

    /// Build def-use chains from use-def chains
    fn build_def_use_chains(&self, info: &mut DefUseInfo) {
        for (use_site, def_sites) in &info.use_def_chains {
            for def_site in def_sites {
                info.def_use_chains
                    .entry(def_site.clone())
                    .or_default()
                    .push(use_site.clone());
            }
        }

        // Remove duplicates
        for (_, use_sites) in &mut info.def_use_chains {
            use_sites.sort_by_key(|use_site| (use_site.block, use_site.value, use_site.position));
            use_sites.dedup();
        }
    }
}

impl Default for UseDefChains {
    fn default() -> Self {
        Self::new()
    }
}

/// Def-use chain analyzer (alias for UseDefChains)
pub type DefUseChains = UseDefChains;

impl fmt::Display for DefUseInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Def-Use Information:")?;

        writeln!(f, "  Value Definitions:")?;
        for (value, def_sites) in &self.value_definitions {
            writeln!(f, "    {} -> {} definitions", value, def_sites.len())?;
        }

        writeln!(f, "  Value Uses:")?;
        for (value, use_sites) in &self.value_uses {
            writeln!(f, "    {} -> {} uses", value, use_sites.len())?;
        }

        writeln!(f, "  Dead Values:")?;
        for value in self.dead_values() {
            writeln!(f, "    {} (defined but never used)", value)?;
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
    fn test_simple_use_def() {
        let mut builder = IrBuilder::new();

        // Create a simple function: x = 1; y = x + 2; return y
        let func_id = builder.create_function("simple".to_string(), vec![], Type::I32);

        let entry = builder.get_current_block().unwrap();
        builder.set_current_block(entry);

        let x = builder.const_value(Constant::I32(1));
        let two = builder.const_value(Constant::I32(2));
        let y = builder
            .build_binary(BinaryOp::Add, x, two, Type::I32)
            .unwrap();
        builder.build_return(Some(y));

        // Analyze use-def chains
        let module = builder.build();
        let func = module.get_function(func_id).unwrap();
        let mut analyzer = UseDefChains::new();
        let info = analyzer.analyze(func);

        // Check that x is defined and used
        assert!(!info.is_dead(x));
        assert!(info.is_used(x));
        assert!(info.is_single_definition(x));

        // Check that y is defined and used
        assert!(!info.is_dead(y));
        assert!(info.is_used(y));
        assert!(info.is_single_definition(y));

        // Check that two is defined but might be considered used
        assert!(info.is_single_definition(two));

        // Verify use-def chains exist
        assert!(!info.use_def_chains.is_empty());
        assert!(!info.def_use_chains.is_empty());
    }

    #[test]
    fn test_dead_value_detection() {
        let mut builder = IrBuilder::new();

        // Create a function with a dead value
        let func_id = builder.create_function("dead_value".to_string(), vec![], Type::I32);

        let entry = builder.get_current_block().unwrap();
        builder.set_current_block(entry);

        let x = builder.const_value(Constant::I32(1));
        let _dead = builder.const_value(Constant::I32(999)); // This is never used
        builder.build_return(Some(x));

        // Analyze use-def chains
        let module = builder.build();
        let func = module.get_function(func_id).unwrap();
        let mut analyzer = UseDefChains::new();
        let info = analyzer.analyze(func);

        // Check for dead values
        let dead_values = info.dead_values();
        assert!(!dead_values.is_empty());

        // x should not be dead (it's returned)
        assert!(!info.is_dead(x));
    }

    #[test]
    fn test_phi_node_use_def() {
        let mut builder = IrBuilder::new();

        // Create a function with a phi node
        let func_id = builder.create_function("phi_test".to_string(), vec![], Type::I32);

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
        let phi_result = builder
            .add_instruction(Instruction::Phi {
                incoming: vec![(left_val, left), (right_val, right)],
                ty: Type::I32,
            })
            .unwrap();
        builder.build_return(Some(phi_result));

        // Analyze use-def chains
        let module = builder.build();
        let func = module.get_function(func_id).unwrap();
        let mut analyzer = UseDefChains::new();
        let info = analyzer.analyze(func);

        // Check that left_val and right_val are used by the phi
        assert!(info.is_used(left_val));
        assert!(info.is_used(right_val));

        // Check that phi result is used in return
        assert!(info.is_used(phi_result));

        // None should be dead
        assert!(!info.is_dead(left_val));
        assert!(!info.is_dead(right_val));
        assert!(!info.is_dead(phi_result));
    }

    #[test]
    fn test_reaching_definitions() {
        let mut builder = IrBuilder::new();

        // Create a function to test reaching definitions
        let func_id = builder.create_function("reaching_defs".to_string(), vec![], Type::I32);

        let entry = builder.get_current_block().unwrap();
        let bb1 = builder.create_block("bb1".to_string()).unwrap();

        // Entry block: define x
        builder.set_current_block(entry);
        let x1 = builder.const_value(Constant::I32(1));
        builder.build_branch(bb1);

        // BB1: redefine x and use it
        builder.set_current_block(bb1);
        let x2 = builder.const_value(Constant::I32(2));
        let result = builder
            .build_binary(BinaryOp::Add, x2, x2, Type::I32)
            .unwrap();
        builder.build_return(Some(result));

        // Analyze use-def chains
        let module = builder.build();
        let func = module.get_function(func_id).unwrap();
        let mut analyzer = UseDefChains::new();
        let info = analyzer.analyze(func);

        // x1 should be defined but not used (killed by x2)
        // x2 should be defined and used
        let x1_uses = info.get_uses(x1);
        let x2_uses = info.get_uses(x2);

        // x1 might be dead since it's redefined
        // x2 should be used
        assert!(!x2_uses.is_empty());
    }
}
