/// Module compilation infrastructure
///
/// This module provides the infrastructure for compiling multiple Script files
/// as a cohesive project, handling module dependencies and cross-module references.
mod context;
mod dependency_graph;
pub mod module_loader;
mod optimized_context;
pub mod resource_limits;

pub use context::{CompilationContext, CompilationUnit};
pub use dependency_graph::{DependencyAnalyzer, DependencyGraph};
pub use module_loader::{CompilationModulePath, ModuleLoader};
pub use optimized_context::{CacheStats, OptimizationConfig, OptimizedCompilationContext};
pub use resource_limits::{ResourceLimits, ResourceLimitsBuilder, ResourceMonitor, ResourceStats};

use crate::error::Result;
use crate::ir::Module as IrModule;
use std::path::Path;

/// Compile a Script project from a directory or single file
pub fn compile_project(path: &Path) -> Result<IrModule> {
    let mut context = CompilationContext::new();

    if path.is_dir() {
        context.compile_directory(path)
    } else {
        context.compile_file(path)
    }
}
