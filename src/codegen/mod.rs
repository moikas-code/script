//! Code generation module
//!
//! This module is responsible for generating executable code from the IR.
//! It supports multiple backends, starting with Cranelift for JIT compilation.

use crate::codegen::debug::DebugFlags;
use crate::error::Error;
use crate::ir::Module as IrModule;

pub mod cranelift;
pub mod debug;

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
}

/// Executable module that can be run
pub struct ExecutableModule {
    /// Entry point function name
    pub entry_point: Option<String>,
    /// Internal backend-specific data
    #[allow(dead_code)]
    backend_data: Box<dyn std::any::Any>,
}

impl CodeGenerator {
    /// Create a new code generator with the Cranelift backend
    pub fn new() -> Self {
        CodeGenerator {
            backend: Box::new(cranelift::CraneliftBackend::new()),
        }
    }

    /// Create a new code generator with debug support
    pub fn with_debug(debug_flags: DebugFlags) -> Self {
        CodeGenerator {
            backend: Box::new(cranelift::CraneliftBackend::with_debug(debug_flags)),
        }
    }

    /// Generate executable code from IR
    pub fn generate(&mut self, ir_module: &IrModule) -> CodegenResult<ExecutableModule> {
        self.backend.generate(ir_module)
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
            backend_data,
        }
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

            // Look up the function ID
            let func_id = cranelift_data.func_ids.get(entry_name).ok_or_else(|| {
                Error::new(
                    crate::error::ErrorKind::RuntimeError,
                    format!("Entry point function '{}' not found", entry_name),
                )
            })?;

            // Get the function pointer from the JIT module
            let func_ptr = cranelift_data.module.get_finalized_function(*func_id);

            // Cast to function pointer and call
            // For now, assume entry point takes no parameters and returns i32
            let entry_fn: extern "C" fn() -> i32 = unsafe { std::mem::transmute(func_ptr) };

            Ok(entry_fn())
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
}
