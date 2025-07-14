//! Array bounds checking for safe memory access
//!
//! This module provides bounds checking infrastructure to prevent
//! buffer overflow vulnerabilities in array indexing operations.

use crate::codegen::CodegenResult;
use crate::error::{Error, ErrorKind};
use cranelift::prelude::*;
use cranelift_module::FuncId;

/// Bounds checking mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoundsCheckMode {
    /// Always perform bounds checks (safest, recommended)
    Always,
    /// Only check in debug builds
    Debug,
    /// Never check bounds (unsafe, not recommended)
    Never,
}

impl Default for BoundsCheckMode {
    fn default() -> Self {
        BoundsCheckMode::Always
    }
}

/// Bounds checker for array operations
pub struct BoundsChecker {
    mode: BoundsCheckMode,
    panic_handler: Option<FuncId>,
}

impl BoundsChecker {
    /// Create a new bounds checker
    pub fn new(mode: BoundsCheckMode) -> Self {
        BoundsChecker {
            mode,
            panic_handler: None,
        }
    }

    /// Set the panic handler function
    pub fn set_panic_handler(&mut self, handler: FuncId) {
        self.panic_handler = Some(handler);
    }

    /// Check if bounds checking is enabled
    pub fn is_enabled(&self) -> bool {
        match self.mode {
            BoundsCheckMode::Always => true,
            BoundsCheckMode::Debug => cfg!(debug_assertions),
            BoundsCheckMode::Never => false,
        }
    }

    /// Generate bounds check for array access
    pub fn check_array_bounds(
        &self,
        builder: &mut FunctionBuilder,
        _array_ptr: Value,
        index: Value,
        array_length: Value,
    ) -> CodegenResult<()> {
        if !self.is_enabled() {
            return Ok(());
        }

        // Create blocks for bounds check
        let bounds_ok = builder.create_block();
        let bounds_fail = builder.create_block();

        // Convert index to same type as length for comparison
        let index_unsigned = if builder.func.dfg.value_type(index) == types::I32 {
            // Check for negative index first
            let zero = builder.ins().iconst(types::I32, 0);
            let is_negative = builder.ins().icmp(IntCC::SignedLessThan, index, zero);
            builder
                .ins()
                .brif(is_negative, bounds_fail, &[], bounds_ok, &[]);

            // In bounds_ok block, convert to unsigned
            builder.switch_to_block(bounds_ok);
            builder.seal_block(bounds_ok);
            builder.ins().uextend(types::I64, index)
        } else {
            // Already i64, check if negative
            let zero = builder.ins().iconst(types::I64, 0);
            let is_negative = builder.ins().icmp(IntCC::SignedLessThan, index, zero);
            builder
                .ins()
                .brif(is_negative, bounds_fail, &[], bounds_ok, &[]);

            builder.switch_to_block(bounds_ok);
            builder.seal_block(bounds_ok);
            index
        };

        // Check upper bound
        let in_bounds = builder
            .ins()
            .icmp(IntCC::UnsignedLessThan, index_unsigned, array_length);
        let continue_block = builder.create_block();
        builder
            .ins()
            .brif(in_bounds, continue_block, &[], bounds_fail, &[]);

        // Handle bounds failure
        builder.switch_to_block(bounds_fail);
        builder.seal_block(bounds_fail);
        self.generate_bounds_panic(builder, index, array_length)?;

        // Continue with normal execution
        builder.switch_to_block(continue_block);
        builder.seal_block(continue_block);

        Ok(())
    }

    /// Generate code for bounds check failure
    fn generate_bounds_panic(
        &self,
        builder: &mut FunctionBuilder,
        _index: Value,
        _length: Value,
    ) -> CodegenResult<()> {
        // Use the panic handler if available, otherwise trap
        if let Some(_panic_handler) = self.panic_handler {
            // For now, we cannot directly call the panic handler from here
            // because we don't have access to the module to convert FuncId to FuncRef.
            // This would need to be handled at a higher level in the translator.
            // Fall back to trap for now.
            builder.ins().trap(TrapCode::HEAP_OUT_OF_BOUNDS);
        } else {
            // Fallback to trap if no panic handler is set
            builder.ins().trap(TrapCode::HEAP_OUT_OF_BOUNDS);
        }

        // This block never returns
        builder.ins().return_(&[]);
        Ok(())
    }

    /// Generate optimized bounds check for constant indices
    pub fn check_constant_bounds(
        &self,
        builder: &mut FunctionBuilder,
        index: i64,
        array_length: Value,
    ) -> CodegenResult<()> {
        if !self.is_enabled() {
            return Ok(());
        }

        // Check for negative constant
        if index < 0 {
            return Err(Error::new(
                ErrorKind::CompilationError,
                format!("Array index {} is negative", index),
            ));
        }

        // For constant indices, we can sometimes prove bounds at compile time
        // But we still need runtime check against array length
        let index_val = builder.ins().iconst(types::I64, index);
        let in_bounds = builder
            .ins()
            .icmp(IntCC::UnsignedLessThan, index_val, array_length);

        let bounds_fail = builder.create_block();
        let continue_block = builder.create_block();

        builder
            .ins()
            .brif(in_bounds, continue_block, &[], bounds_fail, &[]);

        // Handle bounds failure
        builder.switch_to_block(bounds_fail);
        builder.seal_block(bounds_fail);
        self.generate_bounds_panic(builder, index_val, array_length)?;

        // Continue with normal execution
        builder.switch_to_block(continue_block);
        builder.seal_block(continue_block);

        Ok(())
    }
}

/// Helper to create bounds checking code for different array types
pub fn insert_array_bounds_check(
    builder: &mut FunctionBuilder,
    checker: &BoundsChecker,
    array_ptr: Value,
    index: Value,
    _array_type: &crate::types::Type,
) -> CodegenResult<()> {
    // Get array length based on array representation
    // For now, assume arrays store length at offset 8
    let length_ptr = builder.ins().iadd_imm(array_ptr, 8);
    let array_length = builder
        .ins()
        .load(types::I64, MemFlags::new(), length_ptr, 0);

    checker.check_array_bounds(builder, array_ptr, index, array_length)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounds_check_mode_default() {
        assert_eq!(BoundsCheckMode::default(), BoundsCheckMode::Always);
    }

    #[test]
    fn test_bounds_checker_enabled() {
        let always = BoundsChecker::new(BoundsCheckMode::Always);
        assert!(always.is_enabled());

        let never = BoundsChecker::new(BoundsCheckMode::Never);
        assert!(!never.is_enabled());

        let debug = BoundsChecker::new(BoundsCheckMode::Debug);
        assert_eq!(debug.is_enabled(), cfg!(debug_assertions));
    }
}
