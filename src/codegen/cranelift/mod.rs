//! Cranelift backend for code generation
//!
//! This module implements JIT compilation using Cranelift.

use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};

use crate::codegen::debug::{DebugContext, DebugFlags};
use crate::error::{Error, ErrorKind};
use crate::ir::{Function as IrFunction, Module as IrModule};
use crate::types::Type as ScriptType;

use super::{CodegenBackend, CodegenResult, ExecutableModule};
use std::collections::HashMap;

pub mod closure_optimizer;
pub mod runtime;
pub mod translator;
pub mod translator_extensions;

pub use closure_optimizer::{ClosureOptimizer, OptimizationStats};
pub use runtime::RuntimeSupport;
pub use translator::FunctionTranslator;

/// Cranelift backend for JIT compilation
pub struct CraneliftBackend {
    /// JIT module
    module: JITModule,
    /// Cranelift context
    ctx: codegen::Context,
    /// Function name to ID mapping
    func_ids: HashMap<String, FuncId>,
    /// Debug information context
    debug_context: Option<DebugContext>,
    /// Debug compilation flags
    debug_flags: DebugFlags,
    /// Field layout registry for struct types
    field_layouts: crate::codegen::FieldLayoutRegistry,
    /// Bounds checker for array operations
    bounds_checker: crate::codegen::BoundsChecker,
    /// Closure optimizer for performance enhancements
    closure_optimizer: ClosureOptimizer,
}

impl CraneliftBackend {
    /// Create a new Cranelift backend
    pub fn new() -> Self {
        use cranelift::codegen::isa;

        // Get the native target
        let target_isa = isa::lookup(target_lexicon::HOST)
            .expect("Failed to lookup native target")
            .finish(cranelift::codegen::settings::Flags::new(
                cranelift::codegen::settings::builder(),
            ))
            .expect("Failed to create ISA");

        // Create JIT builder
        let mut jit_builder =
            JITBuilder::with_isa(target_isa, cranelift_module::default_libcall_names());

        // Register runtime functions
        runtime::register_runtime_functions(&mut jit_builder);

        // Create JIT module
        let module = JITModule::new(jit_builder);

        // Create codegen context
        let ctx = module.make_context();

        CraneliftBackend {
            module,
            ctx,
            func_ids: HashMap::new(),
            debug_context: None,
            debug_flags: DebugFlags::default(),
            field_layouts: crate::codegen::FieldLayoutRegistry::new(),
            bounds_checker: crate::codegen::BoundsChecker::new(
                crate::codegen::BoundsCheckMode::Always,
            ),
            closure_optimizer: ClosureOptimizer::new(),
        }
    }

    /// Create a new Cranelift backend with debug support
    pub fn with_debug(debug_flags: DebugFlags) -> Self {
        let mut backend = Self::new();
        backend.debug_flags = debug_flags;

        // Initialize debug context if debug info is enabled
        if backend.debug_flags.debug_info {
            backend.debug_context = Some(DebugContext::new(
                std::env::current_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                "Script Language Compiler 0.1.0".to_string(),
            ));
        }

        backend
    }

    /// Set debug flags
    pub fn set_debug_flags(&mut self, flags: DebugFlags) {
        self.debug_flags = flags;

        // Initialize or clear debug context based on flags
        if self.debug_flags.debug_info && self.debug_context.is_none() {
            self.debug_context = Some(DebugContext::new(
                std::env::current_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                "Script Language Compiler 0.1.0".to_string(),
            ));
        } else if !self.debug_flags.debug_info {
            self.debug_context = None;
        }
    }

    /// Compile an IR module
    fn compile_module(&mut self, ir_module: &IrModule) -> CodegenResult<()> {
        // Declare runtime functions
        self.declare_runtime_functions()?;

        // First pass: declare all functions
        for (_, func) in ir_module.functions() {
            self.declare_function(func)?;
        }

        // Second pass: compile function bodies
        for (_, func) in ir_module.functions() {
            self.compile_function(func, ir_module)?;
        }

        // Finalize the module
        self.module.finalize_definitions().map_err(|e| {
            Error::new(
                ErrorKind::RuntimeError,
                format!("Failed to finalize module: {}", e),
            )
        })?;

        Ok(())
    }

    /// Declare runtime functions
    fn declare_runtime_functions(&mut self) -> CodegenResult<()> {
        // Declare script_print(ptr: i64, len: i64)
        {
            let mut sig = self.module.make_signature();
            sig.params.push(AbiParam::new(types::I64)); // ptr
            sig.params.push(AbiParam::new(types::I64)); // len

            let func_id = self
                .module
                .declare_function("script_print", Linkage::Import, &sig)
                .map_err(|e| {
                    Error::new(
                        ErrorKind::RuntimeError,
                        format!("Failed to declare runtime function script_print: {}", e),
                    )
                })?;

            self.func_ids.insert("script_print".to_string(), func_id);
        }

        // Declare script_alloc(size: i64) -> i64
        {
            let mut sig = self.module.make_signature();
            sig.params.push(AbiParam::new(types::I64)); // size
            sig.returns.push(AbiParam::new(types::I64)); // ptr

            let func_id = self
                .module
                .declare_function("script_alloc", Linkage::Import, &sig)
                .map_err(|e| {
                    Error::new(
                        ErrorKind::RuntimeError,
                        format!("Failed to declare runtime function script_alloc: {}", e),
                    )
                })?;

            self.func_ids.insert("script_alloc".to_string(), func_id);
        }

        // Declare script_free(ptr: i64)
        {
            let mut sig = self.module.make_signature();
            sig.params.push(AbiParam::new(types::I64)); // ptr

            let func_id = self
                .module
                .declare_function("script_free", Linkage::Import, &sig)
                .map_err(|e| {
                    Error::new(
                        ErrorKind::RuntimeError,
                        format!("Failed to declare runtime function script_free: {}", e),
                    )
                })?;

            self.func_ids.insert("script_free".to_string(), func_id);
        }

        // Declare script_panic(msg: i64, len: i64) -> !
        {
            let mut sig = self.module.make_signature();
            sig.params.push(AbiParam::new(types::I64)); // msg ptr
            sig.params.push(AbiParam::new(types::I64)); // len

            let func_id = self
                .module
                .declare_function("script_panic", Linkage::Import, &sig)
                .map_err(|e| {
                    Error::new(
                        ErrorKind::RuntimeError,
                        format!("Failed to declare runtime function script_panic: {}", e),
                    )
                })?;

            self.func_ids.insert("script_panic".to_string(), func_id);
        }

        Ok(())
    }

    /// Declare a function
    fn declare_function(&mut self, func: &IrFunction) -> CodegenResult<()> {
        let sig = self.create_function_signature(func);

        let func_id = self
            .module
            .declare_function(&func.name, Linkage::Local, &sig)
            .map_err(|e| {
                Error::new(
                    ErrorKind::RuntimeError,
                    format!("Failed to declare function: {}", e),
                )
            })?;

        self.func_ids.insert(func.name.clone(), func_id);

        Ok(())
    }

    /// Create a function signature from IR function
    fn create_function_signature(&self, func: &IrFunction) -> Signature {
        let mut sig = self.module.make_signature();

        // Add parameters
        for param in &func.params {
            let ty = script_type_to_cranelift(&param.ty);
            sig.params.push(AbiParam::new(ty));
        }

        // Add return type
        if func.return_type != ScriptType::Unknown {
            let ty = script_type_to_cranelift(&func.return_type);
            sig.returns.push(AbiParam::new(ty));
        }

        sig
    }

    /// Compile a function
    fn compile_function(&mut self, func: &IrFunction, ir_module: &IrModule) -> CodegenResult<()> {
        let func_id = self.func_ids.get(&func.name).ok_or_else(|| {
            Error::new(
                ErrorKind::RuntimeError,
                format!("Function {} not declared", func.name),
            )
        })?;

        // Clear the context for this function
        self.ctx.clear();

        // Set the function signature
        let sig = self.create_function_signature(func);
        self.ctx.func.signature = sig;

        // Create function translator
        let mut translator = FunctionTranslator::new(&mut self.module, &self.func_ids, ir_module);

        // Translate the function
        translator.translate_function(func, &mut self.ctx.func, &mut self.closure_optimizer)?;

        // Add debug information for the function if enabled
        if let Some(ref mut debug_ctx) = self.debug_context {
            if self.debug_flags.debug_info {
                // Get function address range (placeholder for now)
                let start_address = 0u64; // This would be filled in by the JIT
                let end_address = 0u64; // This would be filled in by the JIT

                debug_ctx.add_function(func, start_address, end_address)?;
            }
        }

        // Compile the function
        self.module
            .define_function(*func_id, &mut self.ctx)
            .map_err(|e| {
                Error::new(
                    ErrorKind::RuntimeError,
                    format!("Failed to compile function: {}", e),
                )
            })?;

        Ok(())
    }
}

impl Default for CraneliftBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CodegenBackend for CraneliftBackend {
    type Output = ExecutableModule;

    fn generate(&mut self, module: &IrModule) -> CodegenResult<Self::Output> {
        // Compile the module
        self.compile_module(module)?;

        // Generate debug information if enabled
        let debug_info = if let Some(ref mut debug_ctx) = self.debug_context {
            Some(debug_ctx.generate_sections()?)
        } else {
            None
        };

        // Create executable module
        // For simplicity, create a new empty module
        let empty_module = {
            use cranelift::codegen::isa;
            let target_isa = isa::lookup(target_lexicon::HOST)
                .expect("Failed to lookup native target")
                .finish(cranelift::codegen::settings::Flags::new(
                    cranelift::codegen::settings::builder(),
                ))
                .expect("Failed to create ISA");
            let jit_builder =
                JITBuilder::with_isa(target_isa, cranelift_module::default_libcall_names());
            JITModule::new(jit_builder)
        };

        let backend_data = Box::new(CraneliftModuleData {
            module: std::mem::replace(&mut self.module, empty_module),
            func_ids: std::mem::take(&mut self.func_ids),
            debug_info,
        });

        // Find entry point (prefer __script_main__ for top-level code, then user's main)
        let (entry_point, is_async) =
            if let Some(script_main) = module.get_function_by_name("__script_main__") {
                (Some("__script_main__".to_string()), script_main.is_async)
            } else if let Some(main_func) = module.get_function_by_name("main") {
                (Some("main".to_string()), main_func.is_async)
            } else {
                (None, false)
            };

        if is_async {
            Ok(ExecutableModule::new_async(entry_point, backend_data))
        } else {
            Ok(ExecutableModule::new(entry_point, backend_data))
        }
    }
}

/// Backend-specific data for Cranelift
pub struct CraneliftModuleData {
    pub module: JITModule,
    pub func_ids: HashMap<String, FuncId>,
    pub debug_info: Option<Vec<u8>>,
}

/// Convert Script type to Cranelift type
fn script_type_to_cranelift(ty: &ScriptType) -> types::Type {
    match ty {
        ScriptType::I32 => types::I32,
        ScriptType::F32 => types::F32,
        ScriptType::Bool => types::I8, // Booleans are represented as i8
        ScriptType::String => types::I64, // Pointer type
        ScriptType::Unknown => types::I64, // Default to pointer-sized
        ScriptType::Array(_) => types::I64, // Pointer to array
        ScriptType::Function { .. } => types::I64, // Function pointer
        ScriptType::Result { .. } => types::I64, // Pointer to result
        ScriptType::Future(_) => types::I64, // Pointer to future
        ScriptType::Option(_) => types::I64, // Pointer to option
        ScriptType::Never => types::I64, // Never type (should not occur at runtime)
        ScriptType::Struct { .. } => types::I64, // Pointer to struct
        ScriptType::Named(_) => types::I64, // Pointer to named type
        ScriptType::TypeVar(_) => types::I64, // Should be resolved by now
        ScriptType::Generic { .. } => types::I64, // Should be resolved by now
        ScriptType::TypeParam(_) => types::I64, // Should be resolved by now
        ScriptType::Tuple(_) => types::I64, // TODO: Tuples are not yet implemented in codegen
        ScriptType::Reference { .. } => types::I64, // TODO: References are not yet implemented in codegen
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::IrBuilder;

    #[test]
    fn test_cranelift_backend_creation() {
        let backend = CraneliftBackend::new();
        // Just ensure it can be created
        drop(backend);
    }

    #[test]
    fn test_compile_empty_module() {
        let mut backend = CraneliftBackend::new();
        let builder = IrBuilder::new();
        let module = builder.build();

        let result = backend.generate(&module);
        assert!(result.is_ok());
    }
}
