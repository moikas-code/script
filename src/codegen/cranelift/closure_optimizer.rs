//! Optimized closure calling conventions for Cranelift code generation
//!
//! This module provides fast-path implementations for closure creation and
//! invocation, including direct calls when the target is known at compile time.

use cranelift::codegen::ir::MemFlags;
use cranelift::prelude::*;
use cranelift_module::FuncId;

use crate::codegen::CodegenResult;
use crate::error::{Error, ErrorKind};
use crate::ir::{Instruction, ValueId};
use crate::types::Type;

use super::translator::FunctionTranslator;
use std::collections::HashMap;

/// Closure optimization pass for the Cranelift backend
pub struct ClosureOptimizer {
    /// Known closure targets (function_id -> FuncId)
    known_closures: HashMap<String, FuncId>,
    /// Map from ValueId to function_id for direct call optimization
    closure_values: HashMap<ValueId, String>,
    /// Simple closures that can be inlined (function_id -> IR instructions)
    inlinable_closures: HashMap<String, InlinableClosureInfo>,
    /// Statistics for optimization decisions
    stats: OptimizationStats,
}

/// Information about a closure that can be inlined
#[derive(Clone, Debug)]
struct InlinableClosureInfo {
    /// Parameter names for binding
    parameters: Vec<String>,
    /// Captured variables
    captures: HashMap<String, ValueId>,
    /// Whether this is a simple enough closure to inline
    is_simple: bool,
    /// Estimated instruction count
    instruction_count: usize,
}

#[derive(Default, Debug)]
pub struct OptimizationStats {
    /// Number of direct calls optimized
    pub direct_calls: usize,
    /// Number of fast-path calls (<=4 params)
    pub fast_path_calls: usize,
    /// Number of inlined closures
    pub inlined_closures: usize,
    /// Number of tail calls optimized
    pub tail_calls: usize,
}

impl ClosureOptimizer {
    pub fn new() -> Self {
        ClosureOptimizer {
            known_closures: HashMap::new(),
            closure_values: HashMap::new(),
            inlinable_closures: HashMap::new(),
            stats: OptimizationStats::default(),
        }
    }

    /// Optimize closure creation with known targets
    pub fn optimize_closure_creation<'a>(
        &mut self,
        translator: &mut FunctionTranslator<'a>,
        instruction: &Instruction,
        value_id: ValueId,
        builder: &mut FunctionBuilder,
    ) -> CodegenResult<bool> {
        if let Instruction::CreateClosure {
            function_id,
            parameters,
            captured_vars,
            captures_by_ref,
        } = instruction
        {
            // Track closure creation for direct call optimization
            self.closure_values.insert(value_id, function_id.clone());

            // Check if we have a Cranelift function for this closure
            let closure_func_name = format!("closure_{}", function_id);
            if let Some(func_id) = translator.func_ids.get(&closure_func_name) {
                self.known_closures.insert(function_id.clone(), *func_id);
            }

            // Check if this closure is simple enough to inline
            if self.is_inlinable_closure(parameters, captured_vars) {
                let inline_info = InlinableClosureInfo {
                    parameters: parameters.clone(),
                    captures: captured_vars.iter().cloned().collect(),
                    is_simple: true,
                    instruction_count: self.estimate_closure_size(function_id),
                };
                self.inlinable_closures
                    .insert(function_id.clone(), inline_info);
            }

            // Check if this is an optimized closure candidate
            if self.is_optimizable_closure(parameters, captured_vars) {
                // Use optimized creation path
                return self.create_optimized_closure(
                    translator,
                    value_id,
                    function_id,
                    parameters,
                    captured_vars,
                    *captures_by_ref,
                    builder,
                );
            }
        }
        Ok(false)
    }

    /// Optimize closure invocation with fast paths
    pub fn optimize_closure_invocation<'a>(
        &mut self,
        translator: &mut FunctionTranslator<'a>,
        instruction: &Instruction,
        value_id: ValueId,
        builder: &mut FunctionBuilder,
    ) -> CodegenResult<bool> {
        if let Instruction::InvokeClosure {
            closure,
            args,
            return_type,
        } = instruction
        {
            // Try inlining first (if enabled and suitable)
            if let Some(function_id) = self.closure_values.get(closure) {
                if let Some(inline_info) = self.inlinable_closures.get(function_id).cloned() {
                    if self.should_inline_closure(&inline_info, args.len()) {
                        self.stats.inlined_closures += 1;
                        return self.inline_closure(
                            translator,
                            value_id,
                            &inline_info,
                            args,
                            return_type,
                            builder,
                        );
                    }
                }
            }

            // Try direct call optimization next
            if let Some(target_func) = self.try_resolve_direct_call(translator, *closure) {
                self.stats.direct_calls += 1;
                return self.emit_direct_call(
                    translator,
                    value_id,
                    target_func,
                    args,
                    return_type,
                    builder,
                );
            }

            // Check for tail call optimization
            if self.is_tail_call_position(translator, builder) {
                if let Some(tail_optimized) = self.try_tail_call_optimization(
                    translator,
                    value_id,
                    *closure,
                    args,
                    return_type,
                    builder,
                )? {
                    self.stats.tail_calls += 1;
                    return Ok(tail_optimized);
                }
            }

            // Try fast-path for small argument counts
            if args.len() <= 4 {
                self.stats.fast_path_calls += 1;
                return self.emit_fast_path_call(
                    translator,
                    value_id,
                    *closure,
                    args,
                    return_type,
                    builder,
                );
            }
        }
        Ok(false)
    }

    /// Check if a closure is optimizable
    fn is_optimizable_closure(
        &self,
        parameters: &[String],
        captured_vars: &[(String, ValueId)],
    ) -> bool {
        // Optimize closures with:
        // - 4 or fewer parameters (fits in registers)
        // - 4 or fewer captures (uses inline storage)
        parameters.len() <= 4 && captured_vars.len() <= 4
    }

    /// Create an optimized closure using fast allocation
    fn create_optimized_closure<'a>(
        &mut self,
        translator: &mut FunctionTranslator<'a>,
        value_id: ValueId,
        function_id: &str,
        parameters: &[String],
        captured_vars: &[(String, ValueId)],
        captures_by_ref: bool,
        builder: &mut FunctionBuilder,
    ) -> CodegenResult<bool> {
        // Get the runtime allocation function
        let alloc_func = translator.import_runtime_function(
            "script_alloc",
            &[types::I64],    // size
            Some(types::I64), // returns pointer
            builder,
        )?;

        // Calculate closure size:
        // - 8 bytes for function pointer/ID
        // - 4 bytes for parameter count
        // - 4 bytes for capture count
        // - 8 bytes per capture (inline storage for up to 4 captures)
        let base_size = 8 + 4 + 4;
        let capture_size = captured_vars.len().min(4) * 8;
        let total_size = base_size + capture_size;

        // Allocate the closure
        let size_val = builder.ins().iconst(types::I64, total_size as i64);
        let closure_ptr = builder.ins().call(alloc_func, &[size_val]);
        let closure_ptr = builder.inst_results(closure_ptr)[0];

        // Store function ID (interned)
        let func_id_val = self.intern_function_id(function_id, builder);
        let memflags = MemFlags::new();
        builder.ins().store(memflags, func_id_val, closure_ptr, 0);

        // Store parameter count
        let param_count = builder.ins().iconst(types::I32, parameters.len() as i64);
        builder.ins().store(memflags, param_count, closure_ptr, 8);

        // Store capture count
        let capture_count = builder.ins().iconst(types::I32, captured_vars.len() as i64);
        builder
            .ins()
            .store(memflags, capture_count, closure_ptr, 12);

        // Store captured values inline (up to 4)
        let mut offset = 16;
        for (i, (_name, capture_id)) in captured_vars.iter().enumerate() {
            if i >= 4 {
                break; // Only store first 4 captures inline
            }

            let capture_val = translator.get_value(*capture_id)?;

            // If capturing by reference, store the address; otherwise store the value
            let stored_val = if captures_by_ref {
                // Create a stack slot to hold the reference
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    8,
                    8,
                ));
                let addr = builder.ins().stack_addr(types::I64, slot, 0);
                builder.ins().store(memflags, capture_val, addr, 0);
                addr
            } else {
                capture_val
            };

            builder
                .ins()
                .store(memflags, stored_val, closure_ptr, offset);
            offset += 8;
        }

        // Store the closure pointer as the result
        translator.insert_value(value_id, closure_ptr);

        Ok(true)
    }

    /// Try to resolve a direct call target at compile time
    fn try_resolve_direct_call<'a>(
        &self,
        translator: &FunctionTranslator<'a>,
        closure_id: ValueId,
    ) -> Option<FuncId> {
        // Check if we tracked this closure creation
        if let Some(function_id) = self.closure_values.get(&closure_id) {
            // Check if we have a Cranelift function for this closure
            if let Some(func_id) = self.known_closures.get(function_id) {
                return Some(*func_id);
            }
            // Try to find it in the module
            let closure_func_name = format!("closure_{}", function_id);
            if let Some(func_id) = translator.func_ids.get(&closure_func_name) {
                return Some(*func_id);
            }
        }
        None
    }

    /// Emit a direct call to a known closure target
    fn emit_direct_call<'a>(
        &mut self,
        translator: &mut FunctionTranslator<'a>,
        value_id: ValueId,
        target_func: FuncId,
        args: &[ValueId],
        return_type: &Type,
        builder: &mut FunctionBuilder,
    ) -> CodegenResult<bool> {
        // Get function reference
        let func_ref = translator
            .module
            .declare_func_in_func(target_func, builder.func);

        // Translate arguments
        let mut arg_values = Vec::with_capacity(args.len());
        for arg in args {
            arg_values.push(translator.get_value(*arg)?);
        }

        // Emit direct call
        let call_inst = builder.ins().call(func_ref, &arg_values);

        // Handle return value
        let results = builder.inst_results(call_inst);
        if !results.is_empty() {
            let result = results[0];
            translator.insert_value(value_id, result);
        } else if !matches!(return_type, Type::Tuple(v) if v.is_empty()) {
            // For non-void returns, we need a result
            return Err(Error::new(
                ErrorKind::RuntimeError,
                "Direct call returned no value for non-unit return type",
            ));
        }

        Ok(true)
    }

    /// Emit a fast-path call for closures with ≤4 arguments
    fn emit_fast_path_call<'a>(
        &mut self,
        translator: &mut FunctionTranslator<'a>,
        value_id: ValueId,
        closure_id: ValueId,
        args: &[ValueId],
        return_type: &Type,
        builder: &mut FunctionBuilder,
    ) -> CodegenResult<bool> {
        // Import the fast-path invocation function
        let invoke_func = translator.import_runtime_function(
            "script_invoke_closure_fast",
            &[
                types::I64, // closure pointer
                types::I64, // arg0 (or 0)
                types::I64, // arg1 (or 0)
                types::I64, // arg2 (or 0)
                types::I64, // arg3 (or 0)
                types::I32, // actual arg count
            ],
            Some(types::I64),
            builder,
        )?;

        // Get the closure pointer
        let closure_ptr = translator.get_value(closure_id)?;

        // Prepare arguments (up to 4 with padding)
        let mut arg_values = Vec::with_capacity(4);
        for i in 0..4 {
            if i < args.len() {
                arg_values.push(translator.get_value(args[i])?);
            } else {
                // Pad with null pointers
                arg_values.push(builder.ins().iconst(types::I64, 0));
            }
        }

        // Add actual argument count
        let arg_count = builder.ins().iconst(types::I32, args.len() as i64);

        // Call the fast-path runtime function
        let mut call_args = vec![closure_ptr];
        call_args.extend(arg_values);
        call_args.push(arg_count);

        let call_inst = builder.ins().call(invoke_func, &call_args);
        let result_ptr = builder.inst_results(call_inst)[0];

        // Load the result based on return type
        let result = match return_type {
            Type::I32 => builder
                .ins()
                .load(types::I32, MemFlags::new(), result_ptr, 0),
            Type::F32 => builder
                .ins()
                .load(types::F32, MemFlags::new(), result_ptr, 0),
            Type::Bool => builder
                .ins()
                .load(types::I8, MemFlags::new(), result_ptr, 0),
            _ => result_ptr, // For other types, return the pointer itself
        };

        translator.insert_value(value_id, result);
        Ok(true)
    }

    /// Intern a function ID for fast comparison
    fn intern_function_id(&mut self, function_id: &str, builder: &mut FunctionBuilder) -> Value {
        // In a real implementation, this would use the global function ID cache
        // For now, return a hash of the function ID
        let hash = function_id
            .bytes()
            .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
        builder.ins().iconst(types::I64, hash as i64)
    }

    /// Pack captured values into an inline array
    fn pack_inline_captures<'a>(
        &self,
        _translator: &mut FunctionTranslator<'a>,
        _captured_vars: &[(String, ValueId)],
        builder: &mut FunctionBuilder,
    ) -> CodegenResult<Value> {
        // Allocate space for up to 4 captures
        let array_size = 4 * 8; // 4 values * 8 bytes each
        let stack_slot = builder.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            array_size,
            8,
        ));
        let array_ptr = builder.ins().stack_addr(types::I64, stack_slot, 0);

        // In a real implementation, this would:
        // 1. Get values from translator's value mapping
        // 2. Store them in the allocated array
        // For now, just return the array pointer
        Ok(array_ptr)
    }

    /// Get optimization statistics
    pub fn stats(&self) -> &OptimizationStats {
        &self.stats
    }

    /// Check if a closure should be inlined
    fn should_inline_closure(&self, info: &InlinableClosureInfo, arg_count: usize) -> bool {
        // Inline if:
        // - Closure is marked as simple
        // - Has correct number of arguments
        // - Instruction count is small (< 10 instructions)
        // - Not too many captures (< 3)
        info.is_simple
            && info.parameters.len() == arg_count
            && info.instruction_count < 10
            && info.captures.len() < 3
    }

    /// Check if a closure is simple enough to inline
    fn is_inlinable_closure(
        &self,
        parameters: &[String],
        captured_vars: &[(String, ValueId)],
    ) -> bool {
        // Inline closures with:
        // - 2 or fewer parameters
        // - 2 or fewer captures
        // - No nested closures (would need deeper analysis)
        parameters.len() <= 2 && captured_vars.len() <= 2
    }

    /// Estimate the size of a closure for inlining decisions
    fn estimate_closure_size(&self, _function_id: &str) -> usize {
        // In a real implementation, this would analyze the closure body
        // For now, return a conservative estimate
        5
    }

    /// Inline a closure at the call site
    fn inline_closure<'a>(
        &mut self,
        _translator: &mut FunctionTranslator<'a>,
        _value_id: ValueId,
        info: &InlinableClosureInfo,
        args: &[ValueId],
        _return_type: &Type,
        _builder: &mut FunctionBuilder,
    ) -> CodegenResult<bool> {
        // Create a mapping from parameters to arguments
        let mut param_mapping = HashMap::new();
        for (_i, (param_name, arg_id)) in info.parameters.iter().zip(args.iter()).enumerate() {
            // In a real implementation, we'd clone the value or create a phi node
            // For now, just map the parameter to the argument
            param_mapping.insert(param_name.clone(), *arg_id);
        }

        // In a real implementation, we would:
        // 1. Clone the closure body instructions
        // 2. Substitute parameters with arguments
        // 3. Substitute captured variables with their values
        // 4. Insert the instructions at the call site
        // 5. Replace the return with a value assignment

        // For now, fall back to regular call
        // This is a placeholder that shows the structure
        Ok(false)
    }

    /// Check if we're in tail call position
    fn is_tail_call_position<'a>(
        &self,
        _translator: &FunctionTranslator<'a>,
        _builder: &FunctionBuilder,
    ) -> bool {
        // In tail position if:
        // 1. Current instruction is followed only by a return
        // 2. Return value would be the result of this call
        //
        // In Cranelift, we'd need to peek ahead in the instruction stream
        // For now, return false as this requires deeper integration
        false
    }

    /// Try to optimize a closure call as a tail call
    fn try_tail_call_optimization<'a>(
        &mut self,
        translator: &mut FunctionTranslator<'a>,
        _value_id: ValueId,
        closure_id: ValueId,
        _args: &[ValueId],
        _return_type: &Type,
        _builder: &mut FunctionBuilder,
    ) -> CodegenResult<Option<bool>> {
        // Tail call optimization for closures is complex because:
        // 1. Need to ensure stack frame can be reused
        // 2. Must handle captured variables correctly
        // 3. Return types must match exactly

        // Check if we can do a direct tail call
        if let Some(_target_func) = self.try_resolve_direct_call(translator, closure_id) {
            // In Cranelift, we'd use builder.ins().return_call() instead of call()
            // This reuses the current stack frame

            // For now, we don't have return_call in our abstraction
            // In a real implementation:
            // let func_ref = translator.module.declare_func_in_func(target_func, builder.func);
            // let mut arg_values = vec![];
            // for arg in args {
            //     arg_values.push(translator.get_value(*arg)?);
            // }
            // builder.ins().return_call(func_ref, &arg_values);
            // return Ok(Some(true));
        }

        // Can't optimize as tail call
        Ok(None)
    }
}

impl Default for ClosureOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a closure body is simple enough for inlining
/// Simple closures are those with:
/// - Direct return of expression
/// - Simple arithmetic operations
/// - No loops or complex control flow
fn is_simple_closure_body(instructions: &[Instruction]) -> bool {
    if instructions.is_empty() {
        return false;
    }

    // Check instruction count
    if instructions.len() > 5 {
        return false;
    }

    // Check for complex instructions
    for inst in instructions {
        match inst {
            // Simple instructions that are OK to inline
            Instruction::Const(_)
            | Instruction::Binary { .. }
            | Instruction::Unary { .. }
            | Instruction::Compare { .. }
            | Instruction::LoadField { .. }
            | Instruction::Return(_) => continue,

            // Complex instructions that prevent inlining
            Instruction::Call { .. }
            | Instruction::InvokeClosure { .. }
            | Instruction::CreateClosure { .. }
            | Instruction::Branch(_)
            | Instruction::CondBranch { .. }
            | Instruction::Suspend { .. } => return false,

            // Memory operations need careful handling
            Instruction::Alloc { .. } | Instruction::Load { .. } | Instruction::Store { .. } => {
                return false
            }

            // Other complex operations
            _ => return false,
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_optimizable_closure() {
        let optimizer = ClosureOptimizer::new();

        // Optimizable: few parameters and captures
        assert!(optimizer.is_optimizable_closure(
            &["x".to_string(), "y".to_string()],
            &[("a".to_string(), ValueId(1)), ("b".to_string(), ValueId(2))]
        ));

        // Not optimizable: too many parameters
        assert!(!optimizer.is_optimizable_closure(
            &[
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string(),
                "e".to_string()
            ],
            &[]
        ));

        // Not optimizable: too many captures
        let many_captures: Vec<_> = (0..5).map(|i| (format!("var{}", i), ValueId(i))).collect();
        assert!(!optimizer.is_optimizable_closure(&[], &many_captures));
    }
}
