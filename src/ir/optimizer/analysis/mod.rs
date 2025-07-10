//! Analysis Infrastructure for IR Optimization
//!
//! This module provides reusable analysis components that optimization passes can leverage.
//! Analysis results are cached and shared between optimization passes to improve compilation performance.

pub mod control_flow;
pub mod data_flow;
pub mod dominance;
pub mod liveness;
pub mod use_def;

pub use control_flow::{CfgEdge, CfgNode, ControlFlowGraph};
pub use data_flow::{DataFlowAnalysis, DataFlowDirection, DataFlowProblem, DataFlowSolver};
pub use dominance::{DominanceAnalysis, DominanceInfo};
pub use liveness::{LivenessAnalysis, LivenessInfo};
pub use use_def::{DefUseChains, DefUseInfo, UseDefChains};

use crate::ir::{Function, Module as IrModule};
use std::collections::HashMap;
use std::fmt;

/// Analysis manager that caches and coordinates analysis results
#[derive(Debug)]
pub struct AnalysisManager {
    /// Cached dominance analysis results
    dominance_cache: HashMap<String, DominanceInfo>,
    /// Cached control flow graph results
    cfg_cache: HashMap<String, ControlFlowGraph>,
    /// Cached use-def chain results
    use_def_cache: HashMap<String, DefUseInfo>,
    /// Cached liveness analysis results
    liveness_cache: HashMap<String, LivenessInfo>,
    /// Whether to enable debug output
    debug: bool,
}

impl AnalysisManager {
    /// Create a new analysis manager
    pub fn new() -> Self {
        AnalysisManager {
            dominance_cache: HashMap::new(),
            cfg_cache: HashMap::new(),
            use_def_cache: HashMap::new(),
            liveness_cache: HashMap::new(),
            debug: false,
        }
    }

    /// Enable debug output
    pub fn enable_debug(&mut self) {
        self.debug = true;
    }

    /// Clear all cached analysis results
    pub fn clear_cache(&mut self) {
        self.dominance_cache.clear();
        self.cfg_cache.clear();
        self.use_def_cache.clear();
        self.liveness_cache.clear();
    }

    /// Get or compute dominance analysis for a function
    pub fn get_dominance_analysis(&mut self, func: &Function) -> &DominanceInfo {
        let func_name = func.name.clone();

        if !self.dominance_cache.contains_key(&func_name) {
            if self.debug {
                eprintln!("Computing dominance analysis for function: {func_name}");
            }

            let mut analysis = DominanceAnalysis::new();
            let info = analysis.analyze(func);
            self.dominance_cache.insert(func_name.clone(), info);
        }

        &self.dominance_cache[&func_name]
    }

    /// Get or compute control flow graph for a function
    pub fn get_control_flow_graph(&mut self, func: &Function) -> &ControlFlowGraph {
        let func_name = func.name.clone();

        if !self.cfg_cache.contains_key(&func_name) {
            if self.debug {
                eprintln!("Computing control flow graph for function: {func_name}");
            }

            let cfg = ControlFlowGraph::build(func);
            self.cfg_cache.insert(func_name.clone(), cfg);
        }

        &self.cfg_cache[&func_name]
    }

    /// Get or compute use-def chains for a function
    pub fn get_use_def_chains(&mut self, func: &Function) -> &DefUseInfo {
        let func_name = func.name.clone();

        if !self.use_def_cache.contains_key(&func_name) {
            if self.debug {
                eprintln!("Computing use-def chains for function: {func_name}");
            }

            let mut chains = UseDefChains::new();
            let info = chains.analyze(func);
            self.use_def_cache.insert(func_name.clone(), info);
        }

        &self.use_def_cache[&func_name]
    }

    /// Get or compute liveness analysis for a function
    pub fn get_liveness_analysis(&mut self, func: &Function) -> &LivenessInfo {
        let func_name = func.name.clone();

        if !self.liveness_cache.contains_key(&func_name) {
            if self.debug {
                eprintln!("Computing liveness analysis for function: {func_name}");
            }

            // We need the control flow graph for liveness analysis
            let cfg = self.get_control_flow_graph(func).clone();

            let mut analysis = LivenessAnalysis::new();
            let info = analysis.analyze(func, &cfg);
            self.liveness_cache.insert(func_name.clone(), info);
        }

        &self.liveness_cache[&func_name]
    }

    /// Invalidate analysis results for a function (call after modifications)
    pub fn invalidate_function(&mut self, func_name: &str) {
        self.dominance_cache.remove(func_name);
        self.cfg_cache.remove(func_name);
        self.use_def_cache.remove(func_name);
        self.liveness_cache.remove(func_name);

        if self.debug {
            eprintln!("Invalidated analysis cache for function: {func_name}");
        }
    }

    /// Invalidate all analysis results for a module
    pub fn invalidate_module(&mut self, _module: &IrModule) {
        self.clear_cache();

        if self.debug {
            eprintln!("Invalidated all analysis cache");
        }
    }
}

impl Default for AnalysisManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Analysis error type
#[derive(Debug, Clone, PartialEq)]
pub enum AnalysisError {
    /// Invalid IR structure
    InvalidIr(String),
    /// Missing required information
    MissingInfo(String),
    /// Analysis computation failed
    ComputationFailed(String),
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnalysisError::InvalidIr(msg) => write!(f, "Invalid IR: {}", msg),
            AnalysisError::MissingInfo(msg) => write!(f, "Missing information: {}", msg),
            AnalysisError::ComputationFailed(msg) => {
                write!(f, "Analysis computation failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for AnalysisError {}

/// Result type for analysis operations
pub type AnalysisResult<T> = Result<T, AnalysisError>;

/// Trait for analysis passes that can be cached and reused
pub trait Analysis<T> {
    /// Run the analysis and return the result
    fn analyze(&mut self, func: &Function) -> T;

    /// Get the name of this analysis
    fn name(&self) -> &'static str;

    /// Check if the analysis result is still valid after modifications
    fn is_valid(&self, _func: &Function) -> bool {
        // Default to invalid (recompute every time)
        // Specific analyses can override this to be more intelligent
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Constant, IrBuilder};
    use crate::types::Type;

    #[test]
    fn test_analysis_manager_creation() {
        let manager = AnalysisManager::new();
        assert_eq!(manager.dominance_cache.len(), 0);
        assert_eq!(manager.cfg_cache.len(), 0);
        assert_eq!(manager.use_def_cache.len(), 0);
        assert_eq!(manager.liveness_cache.len(), 0);
    }

    #[test]
    fn test_cache_invalidation() {
        let mut manager = AnalysisManager::new();

        // Create a simple function
        let mut builder = IrBuilder::new();
        let func_id = builder.create_function("test".to_string(), vec![], Type::I32);

        let const_val = builder.const_value(Constant::I32(42));
        builder.build_return(Some(const_val));

        let module = builder.build();
        let func = module.get_function(func_id).unwrap();

        // Get some analysis results (this will populate the cache)
        let _dom = manager.get_dominance_analysis(func);
        let _cfg = manager.get_control_flow_graph(func);

        assert!(manager.dominance_cache.len() > 0);
        assert!(manager.cfg_cache.len() > 0);

        // Invalidate the function
        manager.invalidate_function("test");

        assert_eq!(manager.dominance_cache.len(), 0);
        assert_eq!(manager.cfg_cache.len(), 0);
    }

    #[test]
    fn test_clear_cache() {
        let mut manager = AnalysisManager::new();

        // Manually insert something to test clearing
        manager
            .dominance_cache
            .insert("test".to_string(), DominanceInfo::new());
        manager
            .cfg_cache
            .insert("test".to_string(), ControlFlowGraph::new());

        assert!(manager.dominance_cache.len() > 0);
        assert!(manager.cfg_cache.len() > 0);

        manager.clear_cache();

        assert_eq!(manager.dominance_cache.len(), 0);
        assert_eq!(manager.cfg_cache.len(), 0);
    }
}
