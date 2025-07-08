//! Code generation module
//!
//! This module is responsible for generating executable code from the IR.
//! It supports multiple backends, starting with Cranelift for JIT compilation.

use crate::codegen::debug::DebugFlags;
use crate::error::Error;
use crate::ir::Module as IrModule;
use crate::semantic::analyzer::SemanticAnalyzer;
use crate::inference::InferenceContext;
use std::time::Instant;

pub mod cranelift;
pub mod debug;
pub mod monomorphization;
pub mod field_layout;
pub mod bounds_check;

pub use monomorphization::{MonomorphizationContext, MonomorphizationStats};
pub use field_layout::{FieldLayout, FieldLayoutRegistry};
pub use bounds_check::{BoundsChecker, BoundsCheckMode};

/// Result type for code generation
pub type CodegenResult<T> = Result<T, Error>;

/// Trait for code generation backends
pub trait CodegenBackend {
    /// The type of executable code produced
    type Output;

    /// Generate executable code from an IR module
    fn generate(&mut self, module: &IrModule) -> CodegenResult<Self::Output>;
}

/// High-level code generator that dispatches to specific backends
pub struct CodeGenerator {
    backend: Box<dyn CodegenBackend<Output = ExecutableModule>>,
    /// Monomorphization context for generic handling
    monomorphization_ctx: MonomorphizationContext,
    /// Compilation statistics
    stats: CodegenStats,
}

/// Statistics for code generation
#[derive(Debug, Default)]
pub struct CodegenStats {
    /// Number of functions generated
    pub functions_generated: usize,
    /// Number of generic functions monomorphized
    pub generic_functions_monomorphized: usize,
    /// Code size in bytes
    pub code_size: usize,
    /// Compilation time in milliseconds
    pub compilation_time_ms: u64,
}

/// Executable module that can be run
pub struct ExecutableModule {
    /// Entry point function name
    pub entry_point: Option<String>,
    /// Whether the entry point is async
    pub is_async: bool,
    /// Internal backend-specific data
    #[allow(dead_code)]
    backend_data: Box<dyn std::any::Any>,
}

impl CodeGenerator {
    /// Create a new code generator with the Cranelift backend
    pub fn new() -> Self {
        CodeGenerator {
            backend: Box::new(cranelift::CraneliftBackend::new()),
            monomorphization_ctx: MonomorphizationContext::new(),
            stats: CodegenStats::default(),
        }
    }

    /// Create a new code generator with debug support
    pub fn with_debug(debug_flags: DebugFlags) -> Self {
        CodeGenerator {
            backend: Box::new(cranelift::CraneliftBackend::with_debug(debug_flags)),
            monomorphization_ctx: MonomorphizationContext::new(),
            stats: CodegenStats::default(),
        }
    }

    /// Create a new code generator with semantic analyzer integration
    pub fn with_semantic_analyzer(semantic_analyzer: SemanticAnalyzer) -> Self {
        CodeGenerator {
            backend: Box::new(cranelift::CraneliftBackend::new()),
            monomorphization_ctx: MonomorphizationContext::new().with_semantic_analyzer(semantic_analyzer),
            stats: CodegenStats::default(),
        }
    }

    /// Create a new code generator with inference context integration
    pub fn with_inference_context(inference_ctx: InferenceContext) -> Self {
        CodeGenerator {
            backend: Box::new(cranelift::CraneliftBackend::new()),
            monomorphization_ctx: MonomorphizationContext::new().with_inference_context(inference_ctx),
            stats: CodegenStats::default(),
        }
    }

    /// Generate executable code from IR
    pub fn generate(&mut self, ir_module: &IrModule) -> CodegenResult<ExecutableModule> {
        let start_time = Instant::now();
        
        // Generate code using the backend directly
        // Note: Monomorphization should have been done earlier in the pipeline
        let result = self.backend.generate(ir_module);
        
        // Update compilation stats
        self.stats.compilation_time_ms = start_time.elapsed().as_millis() as u64;
        
        match result {
            Ok(executable) => {
                self.stats.functions_generated = ir_module.functions().len();
                Ok(executable)
            }
            Err(e) => Err(e),
        }
    }

    /// Get compilation statistics
    pub fn stats(&self) -> &CodegenStats {
        &self.stats
    }

    /// Get monomorphization statistics
    pub fn monomorphization_stats(&self) -> &MonomorphizationStats {
        self.monomorphization_ctx.stats()
    }

    /// Get a mutable reference to the monomorphization context
    pub fn monomorphization_context_mut(&mut self) -> &mut MonomorphizationContext {
        &mut self.monomorphization_ctx
    }
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutableModule {
    /// Create a new executable module
    pub fn new(entry_point: Option<String>, backend_data: Box<dyn std::any::Any>) -> Self {
        ExecutableModule {
            entry_point,
            is_async: false,
            backend_data,
        }
    }
    
    /// Create a new executable module with async entry point
    pub fn new_async(entry_point: Option<String>, backend_data: Box<dyn std::any::Any>) -> Self {
        ExecutableModule {
            entry_point,
            is_async: true,
            backend_data,
        }
    }
    
    /// Check if the entry point is async
    pub fn is_async_entry_point(&self) -> bool {
        self.is_async
    }

    /// Execute the module's entry point
    pub fn execute(&self) -> CodegenResult<i32> {
        use crate::codegen::cranelift::CraneliftModuleData;

        // Try to downcast to Cranelift backend data
        if let Some(cranelift_data) = self.backend_data.downcast_ref::<CraneliftModuleData>() {
            // Get the entry point function name
            let entry_name = self.entry_point.as_ref().ok_or_else(|| {
                Error::new(
                    crate::error::ErrorKind::RuntimeError,
                    "No entry point defined",
                )
            })?;

            // Check if this is an async main function
            // In a complete implementation, we'd check the function's metadata
            let is_async_main = entry_name.ends_with("_async") || self.is_async_entry_point();

            // Look up the function ID
            let func_id = cranelift_data.func_ids.get(entry_name).ok_or_else(|| {
                Error::new(
                    crate::error::ErrorKind::RuntimeError,
                    format!("Entry point function '{}' not found", entry_name),
                )
            })?;

            // Get the function pointer from the JIT module
            let func_ptr = cranelift_data.module.get_finalized_function(*func_id);

            if is_async_main {
                // For async main, we need to:
                // 1. Call the async main function to get a Future
                // 2. Block on that future using the runtime
                
                // Cast to async main signature (returns a Future pointer)
                let async_main_fn: extern "C" fn() -> *mut std::ffi::c_void = 
                    unsafe { std::mem::transmute(func_ptr) };
                
                let future_ptr = async_main_fn();
                
                // Call the runtime's block_on function
                // In a real implementation, this would be properly typed
                #[allow(improper_ctypes_definitions)]
                extern "C" {
                    fn script_block_on(future: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
                    fn script_run_executor();
                }
                
                unsafe {
                    // Run the executor to completion
                    script_run_executor();
                }
                
                // For now, return success
                Ok(0)
            } else {
                // Regular synchronous main
                let entry_fn: extern "C" fn() -> i32 = unsafe { std::mem::transmute(func_ptr) };
                Ok(entry_fn())
            }
        } else {
            Err(Error::new(
                crate::error::ErrorKind::RuntimeError,
                "Unsupported backend for execution",
            ))
        }
    }

    /// Get a function pointer by name
    pub fn get_function<T>(&self, name: &str) -> Option<*const T> {
        use crate::codegen::cranelift::CraneliftModuleData;

        // Try to downcast to Cranelift backend data
        if let Some(cranelift_data) = self.backend_data.downcast_ref::<CraneliftModuleData>() {
            // Look up the function ID
            if let Some(func_id) = cranelift_data.func_ids.get(name) {
                // Get the function pointer from the JIT module
                let func_ptr = cranelift_data.module.get_finalized_function(*func_id);
                Some(func_ptr as *const T)
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::IrBuilder;

    #[test]
    fn test_code_generator_creation() {
        let gen = CodeGenerator::new();
        // Just ensure it can be created
        drop(gen);
    }

    #[test]
    fn test_generate_empty_module() {
        let mut gen = CodeGenerator::new();
        let builder = IrBuilder::new();
        let module = builder.build();

        let result = gen.generate(&module);
        assert!(result.is_ok());
    }

    #[test]
    fn test_code_generator_with_semantic_analyzer() {
        let semantic_analyzer = SemanticAnalyzer::new();
        let gen = CodeGenerator::with_semantic_analyzer(semantic_analyzer);
        
        // Verify the generator was created successfully
        assert!(gen.monomorphization_ctx.semantic_analyzer_mut().is_some());
    }

    #[test]
    fn test_code_generator_with_inference_context() {
        let inference_ctx = InferenceContext::new();
        let gen = CodeGenerator::with_inference_context(inference_ctx);
        
        // Verify the generator was created successfully
        assert!(gen.monomorphization_ctx.inference_context_mut().is_some());
    }

    #[test]
    fn test_compilation_stats() {
        let mut gen = CodeGenerator::new();
        let builder = IrBuilder::new();
        let module = builder.build();

        let _result = gen.generate(&module);
        
        // Check that stats were updated
        let stats = gen.stats();
        assert!(stats.compilation_time_ms > 0);
        assert_eq!(stats.functions_generated, module.functions().len());
    }
}
