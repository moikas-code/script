use cranelift::codegen::ir::Function;
use cranelift::prelude::*;
use cranelift_module::{DataDescription, FuncId, Linkage as ModuleLinkage, Module};

use crate::error::{Error, ErrorKind};
use crate::ir::{BasicBlock, BlockId, Constant, Function as IrFunction, Instruction, ValueId};
use crate::ir::{BinaryOp, ComparisonOp, UnaryOp};

use super::{script_type_to_cranelift, CodegenResult};
use std::collections::HashMap;

/// Translates IR functions to Cranelift IR
pub struct FunctionTranslator<'a> {
    /// Module for looking up functions
    module: &'a mut dyn Module,
    /// Function name to ID mapping
    func_ids: &'a HashMap<String, FuncId>,
    /// IR module for function lookups
    ir_module: &'a crate::ir::Module,
    /// Value mapping from IR to Cranelift
    values: HashMap<ValueId, Value>,
    /// Type mapping for values (for runtime type checking)
    value_types: HashMap<ValueId, crate::types::Type>,
    /// Block mapping from IR to Cranelift
    blocks: HashMap<BlockId, Block>,
    /// Variable counter for SSA construction
    #[allow(dead_code)]
    var_counter: usize,
    /// Track which blocks have been processed
    processed_blocks: std::collections::HashSet<BlockId>,
    /// String constants for this function
    string_constants: Vec<(String, cranelift_module::DataId)>,
}

impl<'a> FunctionTranslator<'a> {
    /// Create a new function translator
    pub fn new(
        module: &'a mut dyn Module,
        func_ids: &'a HashMap<String, FuncId>,
        ir_module: &'a crate::ir::Module,
    ) -> Self {
        FunctionTranslator {
            module,
            func_ids,
            ir_module,
            values: HashMap::new(),
            value_types: HashMap::new(),
            blocks: HashMap::new(),
            var_counter: 0,
            processed_blocks: std::collections::HashSet::new(),
            string_constants: Vec::new(),
        }
    }

    /// Translate an IR function to Cranelift IR
    pub fn translate_function(
        &mut self,
        ir_func: &IrFunction,
        cranelift_func: &mut Function,
    ) -> CodegenResult<()> {
        let mut fn_builder_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(cranelift_func, &mut fn_builder_ctx);

        // Create entry block
        let entry_block = builder.create_block();

        // Add block parameters for function parameters
        for (i, param) in ir_func.params.iter().enumerate() {
            let ty = script_type_to_cranelift(&param.ty);
            builder.append_block_param(entry_block, ty);

            // Map parameter to value
            let param_val = builder.block_params(entry_block)[i];
            // Parameters use ValueIds starting at 1000 (as set in lowering/mod.rs)
            let param_value_id = ValueId(i as u32 + 1000);
            self.values.insert(param_value_id, param_val);
            // Track parameter type
            self.value_types.insert(param_value_id, param.ty.clone());
        }

        // Switch to entry block (but don't seal it yet)
        builder.switch_to_block(entry_block);

        // First pass: create all blocks
        for (block_id, _ir_block) in ir_func.blocks() {
            if Some(*block_id) != ir_func.entry_block {
                let cranelift_block = builder.create_block();
                self.blocks.insert(*block_id, cranelift_block);
            } else {
                self.blocks.insert(*block_id, entry_block);
            }
        }

        // Second pass: translate blocks
        for ir_block in ir_func.blocks_in_order() {
            self.translate_block(ir_block, &mut builder)?;
        }

        // Finalize the function
        builder.finalize();

        Ok(())
    }

    /// Translate a basic block
    fn translate_block(
        &mut self,
        block: &BasicBlock,
        builder: &mut FunctionBuilder,
    ) -> CodegenResult<()> {
        // Check if this block has already been processed
        if self.processed_blocks.contains(&block.id) {
            return Ok(());
        }

        let cranelift_block = *self
            .blocks
            .get(&block.id)
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Block not found"))?;

        // Switch to this block
        builder.switch_to_block(cranelift_block);

        // Translate instructions
        for (value_id, inst_with_loc) in &block.instructions {
            // Track the type of this instruction's result if it produces one
            if let Some(result_type) = inst_with_loc.instruction.result_type() {
                self.value_types.insert(*value_id, result_type);
            }
            
            self.translate_instruction(*value_id, &inst_with_loc.instruction, builder)?;
        }

        // Mark this block as processed
        self.processed_blocks.insert(block.id);

        // Seal the block if all predecessors have been processed
        // In a real implementation, this would track predecessor processing
        builder.seal_block(cranelift_block);

        Ok(())
    }

    /// Translate an instruction
    fn translate_instruction(
        &mut self,
        value_id: ValueId,
        inst: &Instruction,
        builder: &mut FunctionBuilder,
    ) -> CodegenResult<()> {
        match inst {
            Instruction::Const(constant) => {
                let val = self.translate_constant(constant, builder)?;
                self.values.insert(value_id, val);
            }

            Instruction::Binary {
                op,
                lhs,
                rhs,
                ty: _,
            } => {
                let lhs_val = self.get_value(*lhs)?;
                let rhs_val = self.get_value(*rhs)?;
                let result = self.translate_binary_op(*op, lhs_val, rhs_val, builder)?;
                self.values.insert(value_id, result);
            }

            Instruction::Unary { op, operand, ty: _ } => {
                let operand_val = self.get_value(*operand)?;
                let result = self.translate_unary_op(*op, operand_val, builder)?;
                self.values.insert(value_id, result);
            }

            Instruction::Compare { op, lhs, rhs } => {
                let lhs_val = self.get_value(*lhs)?;
                let rhs_val = self.get_value(*rhs)?;
                let result = self.translate_comparison(*op, lhs_val, rhs_val, builder)?;
                self.values.insert(value_id, result);
            }

            Instruction::Return(value) => {
                if let Some(val_id) = value {
                    let val = self.get_value(*val_id)?;
                    builder.ins().return_(&[val]);
                } else {
                    builder.ins().return_(&[]);
                }
            }

            Instruction::Branch(target) => {
                let target_block = self.get_block(*target)?;
                builder.ins().jump(target_block, &[]);
            }

            Instruction::CondBranch {
                condition,
                then_block,
                else_block,
            } => {
                let cond_val = self.get_value(*condition)?;
                let then_blk = self.get_block(*then_block)?;
                let else_blk = self.get_block(*else_block)?;
                builder.ins().brif(cond_val, then_blk, &[], else_blk, &[]);
            }

            Instruction::Cast {
                value,
                from_ty,
                to_ty,
            } => {
                let val = self.get_value(*value)?;
                let result = self.translate_cast(val, from_ty, to_ty, builder)?;
                self.values.insert(value_id, result);
            }

            Instruction::Call {
                func,
                args,
                ty,
            } => {
                // Look up the function in the IR module to get its name and signature
                let ir_func = self.ir_module.get_function(*func).ok_or_else(|| {
                    Error::new(
                        ErrorKind::RuntimeError,
                        format!("Internal error: Function ID {:?} not found", func),
                    )
                })?;

                // Validate argument count
                let expected_arg_count = ir_func.params.len();
                if args.len() != expected_arg_count {
                    return Err(Error::new(
                        ErrorKind::TypeError,
                        format!(
                            "Function '{}' expects {} argument{}, but {} {} provided",
                            ir_func.name,
                            expected_arg_count,
                            if expected_arg_count == 1 { "" } else { "s" },
                            args.len(),
                            if args.len() == 1 { "was" } else { "were" }
                        ),
                    ));
                }

                // Get the Cranelift function ID from our mapping using the function name
                let cranelift_func_id = self.func_ids.get(&ir_func.name).ok_or_else(|| {
                    Error::new(
                        ErrorKind::RuntimeError,
                        format!("Function '{}' is not available in this context", ir_func.name),
                    )
                })?;

                // Get the function reference
                let func_ref = self.module.declare_func_in_func(*cranelift_func_id, builder.func);

                // Collect and validate argument values
                let mut arg_vals = Vec::with_capacity(args.len());
                for (i, arg) in args.iter().enumerate() {
                    let val = self.get_value(*arg)?;
                    
                    // Validate argument type if we have type information
                    if let Some(arg_type) = self.value_types.get(arg) {
                        let expected_type = &ir_func.params[i].ty;
                        
                        // Basic type compatibility check
                        if !self.types_compatible(arg_type, expected_type) {
                            return Err(Error::new(
                                ErrorKind::TypeError,
                                format!(
                                    "Type mismatch in function call '{}': parameter {} expects {:?}, but {:?} was provided",
                                    ir_func.name, i + 1, expected_type, arg_type
                                ),
                            ));
                        }
                    }
                    
                    arg_vals.push(val);
                }

                // Call the function
                let inst = builder.ins().call(func_ref, &arg_vals);
                
                // Handle return value
                let result = if ty != &crate::types::Type::Unknown {
                    // Function has a return value
                    let results = builder.inst_results(inst);
                    if results.is_empty() {
                        return Err(Error::new(
                            ErrorKind::RuntimeError,
                            format!("Function '{}' should return a value but didn't", ir_func.name),
                        ));
                    }
                    results[0]
                } else {
                    // Void function - create a dummy value for SSA form
                    builder.ins().iconst(types::I32, 0)
                };
                
                self.values.insert(value_id, result);
            }

            Instruction::Alloc { ty } => {
                let result = self.translate_alloc(ty, builder)?;
                self.values.insert(value_id, result);
            }

            Instruction::Load { ptr, ty } => {
                let ptr_val = self.get_value(*ptr)?;
                let result = self.translate_load(ptr_val, ty, builder)?;
                self.values.insert(value_id, result);
            }

            Instruction::Store { ptr, value } => {
                let ptr_val = self.get_value(*ptr)?;
                let val = self.get_value(*value)?;
                self.translate_store(ptr_val, val, builder)?;
                // Store doesn't produce a value, so we don't insert into values map
            }

            Instruction::GetElementPtr {
                ptr,
                index,
                elem_ty,
            } => {
                let ptr_val = self.get_value(*ptr)?;
                let index_val = self.get_value(*index)?;
                let result = self.translate_gep(ptr_val, index_val, elem_ty, builder)?;
                self.values.insert(value_id, result);
            }

            Instruction::GetFieldPtr {
                object,
                field_name,
                field_ty,
            } => {
                let object_val = self.get_value(*object)?;
                let result =
                    self.translate_get_field_ptr(object_val, field_name, field_ty, builder)?;
                self.values.insert(value_id, result);
            }

            Instruction::LoadField {
                object,
                field_name,
                field_ty,
            } => {
                let object_val = self.get_value(*object)?;
                let result =
                    self.translate_load_field(object_val, field_name, field_ty, builder)?;
                self.values.insert(value_id, result);
            }

            Instruction::StoreField {
                object,
                field_name,
                value,
            } => {
                let object_val = self.get_value(*object)?;
                let val = self.get_value(*value)?;
                self.translate_store_field(object_val, field_name, val, builder)?;
                // Store doesn't produce a value, so we don't insert into values map
            }

            Instruction::Phi { incoming, ty } => {
                let result = self.translate_phi(incoming, ty, builder)?;
                self.values.insert(value_id, result);
            }
        }

        Ok(())
    }

    /// Translate a constant
    fn translate_constant(
        &mut self,
        constant: &Constant,
        builder: &mut FunctionBuilder,
    ) -> CodegenResult<Value> {
        match constant {
            Constant::I32(n) => Ok(builder.ins().iconst(types::I32, *n as i64)),
            Constant::F32(f) => Ok(builder.ins().f32const(*f)),
            Constant::Bool(b) => Ok(builder.ins().iconst(types::I8, if *b { 1 } else { 0 })),
            Constant::String(s) => {
                // For now, we'll create a simple string pointer
                // In a real implementation, this would need proper string management
                self.translate_string_constant(s, builder)
            }
            Constant::Null => Ok(builder.ins().iconst(types::I64, 0)),
        }
    }

    /// Translate a binary operation
    fn translate_binary_op(
        &mut self,
        op: BinaryOp,
        lhs: Value,
        rhs: Value,
        builder: &mut FunctionBuilder,
    ) -> CodegenResult<Value> {
        Ok(match op {
            BinaryOp::Add => builder.ins().iadd(lhs, rhs),
            BinaryOp::Sub => builder.ins().isub(lhs, rhs),
            BinaryOp::Mul => builder.ins().imul(lhs, rhs),
            BinaryOp::Div => builder.ins().sdiv(lhs, rhs),
            BinaryOp::Mod => builder.ins().srem(lhs, rhs),
            BinaryOp::And => builder.ins().band(lhs, rhs),
            BinaryOp::Or => builder.ins().bor(lhs, rhs),
        })
    }

    /// Translate a unary operation
    fn translate_unary_op(
        &mut self,
        op: UnaryOp,
        operand: Value,
        builder: &mut FunctionBuilder,
    ) -> CodegenResult<Value> {
        Ok(match op {
            UnaryOp::Neg => builder.ins().ineg(operand),
            UnaryOp::Not => {
                // For boolean not, compare with 0
                let zero = builder.ins().iconst(types::I8, 0);
                builder.ins().icmp(IntCC::Equal, operand, zero)
            }
        })
    }

    /// Translate a comparison operation
    fn translate_comparison(
        &mut self,
        op: ComparisonOp,
        lhs: Value,
        rhs: Value,
        builder: &mut FunctionBuilder,
    ) -> CodegenResult<Value> {
        let cc = match op {
            ComparisonOp::Eq => IntCC::Equal,
            ComparisonOp::Ne => IntCC::NotEqual,
            ComparisonOp::Lt => IntCC::SignedLessThan,
            ComparisonOp::Le => IntCC::SignedLessThanOrEqual,
            ComparisonOp::Gt => IntCC::SignedGreaterThan,
            ComparisonOp::Ge => IntCC::SignedGreaterThanOrEqual,
        };

        Ok(builder.ins().icmp(cc, lhs, rhs))
    }

    /// Get a value by ID
    fn get_value(&self, id: ValueId) -> CodegenResult<Value> {
        self.values
            .get(&id)
            .copied()
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, format!("Value {:?} not found", id)))
    }

    /// Check if two types are compatible
    fn types_compatible(&self, actual: &crate::types::Type, expected: &crate::types::Type) -> bool {
        use crate::types::Type;
        
        match (actual, expected) {
            // Exact match
            (a, e) if a == e => true,
            
            // Unknown type is compatible with anything (gradual typing)
            (Type::Unknown, _) | (_, Type::Unknown) => true,
            
            // Named types need string comparison
            (Type::Named(a), Type::Named(e)) => a == e,
            
            // Array types need element type compatibility
            (Type::Array(a), Type::Array(e)) => self.types_compatible(a, e),
            
            // Function types need full signature compatibility
            (Type::Function { params: a_params, ret: a_ret }, 
             Type::Function { params: e_params, ret: e_ret }) => {
                a_params.len() == e_params.len() &&
                a_params.iter().zip(e_params.iter()).all(|(a, e)| self.types_compatible(a, e)) &&
                self.types_compatible(a_ret, e_ret)
            }
            
            // No other conversions are allowed
            _ => false,
        }
    }

    /// Get a block by ID
    fn get_block(&self, id: BlockId) -> CodegenResult<Block> {
        self.blocks
            .get(&id)
            .copied()
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, format!("Block {:?} not found", id)))
    }

    /// Translate a type cast
    fn translate_cast(
        &mut self,
        value: Value,
        from_ty: &crate::types::Type,
        to_ty: &crate::types::Type,
        builder: &mut FunctionBuilder,
    ) -> CodegenResult<Value> {
        use crate::types::Type;

        match (from_ty, to_ty) {
            // Integer to integer casts
            (Type::I32, Type::I32) => Ok(value), // No-op

            // Integer to float casts
            (Type::I32, Type::F32) => Ok(builder.ins().fcvt_from_sint(types::F32, value)),

            // Float to integer casts
            (Type::F32, Type::I32) => Ok(builder.ins().fcvt_to_sint_sat(types::I32, value)),

            // Float to float casts
            (Type::F32, Type::F32) => Ok(value), // No-op

            // Boolean to integer casts
            (Type::Bool, Type::I32) => Ok(builder.ins().uextend(types::I32, value)),

            // Integer to boolean casts (non-zero = true)
            (Type::I32, Type::Bool) => {
                let zero = builder.ins().iconst(types::I32, 0);
                Ok(builder.ins().icmp(IntCC::NotEqual, value, zero))
            }

            // For unknown or unsupported casts, return the value as-is
            _ => Ok(value),
        }
    }

    /// Translate memory allocation
    fn translate_alloc(
        &mut self,
        ty: &crate::types::Type,
        builder: &mut FunctionBuilder,
    ) -> CodegenResult<Value> {
        // For now, we'll use stack allocation (alloca-style)
        // In a real implementation, this would need proper memory management
        let cranelift_ty = script_type_to_cranelift(ty);

        // Create a stack slot for the allocation
        let stack_slot = builder.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            cranelift_ty.bytes() as u32,
            3, // alignment (8-byte alignment = 2^3)
        ));

        // Get the address of the stack slot
        Ok(builder.ins().stack_addr(types::I64, stack_slot, 0))
    }

    /// Translate memory load
    fn translate_load(
        &mut self,
        ptr: Value,
        ty: &crate::types::Type,
        builder: &mut FunctionBuilder,
    ) -> CodegenResult<Value> {
        let cranelift_ty = script_type_to_cranelift(ty);
        let memflags = MemFlags::new();

        Ok(builder.ins().load(cranelift_ty, memflags, ptr, 0))
    }

    /// Translate memory store
    fn translate_store(
        &mut self,
        ptr: Value,
        value: Value,
        builder: &mut FunctionBuilder,
    ) -> CodegenResult<()> {
        let memflags = MemFlags::new();
        builder.ins().store(memflags, value, ptr, 0);
        Ok(())
    }

    /// Translate get element pointer (array indexing)
    fn translate_gep(
        &mut self,
        ptr: Value,
        index: Value,
        elem_ty: &crate::types::Type,
        builder: &mut FunctionBuilder,
    ) -> CodegenResult<Value> {
        let cranelift_elem_ty = script_type_to_cranelift(elem_ty);
        let elem_size = cranelift_elem_ty.bytes() as i64;

        // Calculate offset: index * element_size
        let size_const = builder.ins().iconst(types::I64, elem_size);
        // For simplicity, assume index is i32 and extend to i64
        let index_64 = builder.ins().sextend(types::I64, index);
        let offset = builder.ins().imul(index_64, size_const);

        // Add offset to base pointer
        Ok(builder.ins().iadd(ptr, offset))
    }

    /// Translate phi node
    fn translate_phi(
        &mut self,
        incoming: &[(ValueId, BlockId)],
        ty: &crate::types::Type,
        builder: &mut FunctionBuilder,
    ) -> CodegenResult<Value> {
        // For phi nodes, we need to use Cranelift's variable system
        // This is a simplified implementation - a full implementation would
        // need proper SSA variable management

        let cranelift_ty = script_type_to_cranelift(ty);
        let var = Variable::new(self.var_counter);
        self.var_counter += 1;

        builder.declare_var(var, cranelift_ty);

        // For now, just use the first incoming value as a placeholder
        // A proper implementation would set up the phi properly
        if let Some((first_val_id, _)) = incoming.first() {
            let first_val = self.get_value(*first_val_id)?;
            builder.def_var(var, first_val);
            Ok(builder.use_var(var))
        } else {
            // No incoming values - create a default
            let default_val = match ty {
                crate::types::Type::I32 => builder.ins().iconst(types::I32, 0),
                crate::types::Type::F32 => builder.ins().f32const(0.0),
                crate::types::Type::Bool => builder.ins().iconst(types::I8, 0),
                _ => builder.ins().iconst(types::I64, 0),
            };
            builder.def_var(var, default_val);
            Ok(builder.use_var(var))
        }
    }

    /// Translate a string constant
    fn translate_string_constant(
        &mut self,
        s: &str,
        builder: &mut FunctionBuilder,
    ) -> CodegenResult<Value> {
        // Check if we've already created a data section for this string
        for (existing_str, data_id) in &self.string_constants {
            if existing_str == s {
                // Reuse existing string constant
                let global_value = self.module.declare_data_in_func(*data_id, builder.func);
                return Ok(builder.ins().global_value(types::I64, global_value));
            }
        }

        // Create a unique data ID for this string constant
        let data_name = format!("str_const_{}", self.string_constants.len());
        
        // Declare the data in the module
        let data_id = self.module
            .declare_data(&data_name, ModuleLinkage::Local, false, false)
            .map_err(|e| Error::new(
                ErrorKind::RuntimeError,
                format!("Failed to declare string data: {}", e),
            ))?;

        // Create the data content
        let mut data_desc = DataDescription::new();
        let string_bytes = s.as_bytes();
        
        // Store length followed by the string data (Pascal-style string)
        let mut contents = Vec::with_capacity(8 + string_bytes.len());
        contents.extend_from_slice(&(string_bytes.len() as u64).to_le_bytes());
        contents.extend_from_slice(string_bytes);
        
        data_desc.define(contents.into_boxed_slice());
        
        // Define the data in the module
        self.module
            .define_data(data_id, &data_desc)
            .map_err(|e| Error::new(
                ErrorKind::RuntimeError,
                format!("Failed to define string data: {}", e),
            ))?;

        // Remember this string constant
        self.string_constants.push((s.to_string(), data_id));

        // Get a reference to the data in the current function
        let global_value = self.module.declare_data_in_func(data_id, builder.func);
        
        // Return a pointer to the string data
        Ok(builder.ins().global_value(types::I64, global_value))
    }

    /// Translate get field pointer (object field access)
    fn translate_get_field_ptr(
        &mut self,
        object: Value,
        field_name: &str,
        field_ty: &crate::types::Type,
        builder: &mut FunctionBuilder,
    ) -> CodegenResult<Value> {
        // For object field access, we need to:
        // 1. Calculate the field offset based on the field name
        // 2. Add the offset to the object pointer

        // For now, use a simple hash-based offset calculation
        // In a real implementation, this would use proper object layout info
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::Hasher;
        hasher.write(field_name.as_bytes());
        let hash = hasher.finish();
        let field_offset = (hash % 256) as i64; // Use a reasonable range for offsets

        let offset_const = builder.ins().iconst(types::I64, field_offset);
        Ok(builder.ins().iadd(object, offset_const))
    }

    /// Translate load field (direct object field load)
    fn translate_load_field(
        &mut self,
        object: Value,
        field_name: &str,
        field_ty: &crate::types::Type,
        builder: &mut FunctionBuilder,
    ) -> CodegenResult<Value> {
        // For loading a field directly from an object:
        // 1. Calculate field pointer
        // 2. Load value from that pointer

        let field_ptr = self.translate_get_field_ptr(object, field_name, field_ty, builder)?;
        self.translate_load(field_ptr, field_ty, builder)
    }

    /// Translate store field (direct object field store)
    fn translate_store_field(
        &mut self,
        object: Value,
        field_name: &str,
        value: Value,
        builder: &mut FunctionBuilder,
    ) -> CodegenResult<()> {
        // For storing a field directly to an object:
        // 1. Calculate field pointer (we need the field type, so we'll use Unknown for now)
        // 2. Store value to that pointer

        let field_ptr = self.translate_get_field_ptr(
            object,
            field_name,
            &crate::types::Type::Unknown,
            builder,
        )?;
        self.translate_store(field_ptr, value, builder)
    }
}
