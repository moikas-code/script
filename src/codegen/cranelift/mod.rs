//! Cranelift backend for code generation
//! 
//! This module implements JIT compilation using Cranelift.

use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Module, FuncId, Linkage};

use crate::ir::{Module as IrModule, Function as IrFunction};
use crate::types::Type as ScriptType;
use crate::error::{Error, ErrorKind};

use super::{CodegenResult, CodegenBackend, ExecutableModule};
use std::collections::HashMap;

pub mod translator;
pub mod runtime;

pub use translator::FunctionTranslator;
pub use runtime::RuntimeSupport;

/// Cranelift backend for JIT compilation
pub struct CraneliftBackend {
    /// JIT module
    module: JITModule,
    /// Cranelift context
    ctx: codegen::Context,
    /// Function name to ID mapping
    func_ids: HashMap<String, FuncId>,
}

impl CraneliftBackend {
    /// Create a new Cranelift backend
    pub fn new() -> Self {
        use cranelift::codegen::isa;
        
        // Get the native target
        let target_isa = isa::lookup(target_lexicon::HOST)
            .expect("Failed to lookup native target")
            .finish(cranelift::codegen::settings::Flags::new(cranelift::codegen::settings::builder()))
            .expect("Failed to create ISA");
        
        // Create JIT builder
        let mut jit_builder = JITBuilder::with_isa(target_isa, cranelift_module::default_libcall_names());
        
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
        }
    }
    
    /// Compile an IR module
    fn compile_module(&mut self, ir_module: &IrModule) -> CodegenResult<()> {
        // First pass: declare all functions
        for (_, func) in ir_module.functions() {
            self.declare_function(func)?;
        }
        
        // Second pass: compile function bodies
        for (_, func) in ir_module.functions() {
            self.compile_function(func)?;
        }
        
        // Finalize the module
        self.module.finalize_definitions().map_err(|e| {
            Error::new(ErrorKind::RuntimeError, format!("Failed to finalize module: {}", e))
        })?;
        
        Ok(())
    }
    
    /// Declare a function
    fn declare_function(&mut self, func: &IrFunction) -> CodegenResult<()> {
        let sig = self.create_function_signature(func);
        
        let func_id = self.module
            .declare_function(&func.name, Linkage::Local, &sig)
            .map_err(|e| Error::new(ErrorKind::RuntimeError, format!("Failed to declare function: {}", e)))?;
        
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
    fn compile_function(&mut self, func: &IrFunction) -> CodegenResult<()> {
        let func_id = self.func_ids.get(&func.name)
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, format!("Function {} not declared", func.name)))?;
        
        // Clear the context for this function
        self.ctx.clear();
        
        // Set the function signature
        let sig = self.create_function_signature(func);
        self.ctx.func.signature = sig;
        
        // Create function translator
        let mut translator = FunctionTranslator::new(&self.module);
        
        // Translate the function
        translator.translate_function(func, &mut self.ctx.func)?;
        
        // Compile the function
        self.module
            .define_function(*func_id, &mut self.ctx)
            .map_err(|e| Error::new(ErrorKind::RuntimeError, format!("Failed to compile function: {}", e)))?;
        
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
        
        // Create executable module
        // For simplicity, create a new empty module
        let empty_module = {
            use cranelift::codegen::isa;
            let target_isa = isa::lookup(target_lexicon::HOST)
                .expect("Failed to lookup native target")
                .finish(cranelift::codegen::settings::Flags::new(cranelift::codegen::settings::builder()))
                .expect("Failed to create ISA");
            let jit_builder = JITBuilder::with_isa(target_isa, cranelift_module::default_libcall_names());
            JITModule::new(jit_builder)
        };
        
        let backend_data = Box::new(CraneliftModuleData {
            module: std::mem::replace(&mut self.module, empty_module),
            func_ids: std::mem::take(&mut self.func_ids),
        });
        
        // Find entry point (main function if it exists)
        let entry_point = if module.get_function_by_name("main").is_some() {
            Some("main".to_string())
        } else {
            None
        };
        
        Ok(ExecutableModule::new(entry_point, backend_data))
    }
}

/// Backend-specific data for Cranelift
#[allow(dead_code)]
struct CraneliftModuleData {
    module: JITModule,
    func_ids: HashMap<String, FuncId>,
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
        ScriptType::Named(_) => types::I64, // Pointer to named type
        ScriptType::TypeVar(_) => types::I64, // Should be resolved by now
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