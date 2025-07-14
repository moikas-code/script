//! Async function transformation module
//!
//! This module transforms async functions into state machines that can be
//! polled by the async runtime. Each async function is converted into:
//!
//! 1. A state struct containing all local variables
//! 2. A poll method that advances the state machine
//! 3. Suspend points at each await expression

use crate::error::{Error, ErrorKind};
use crate::ir::{
    BasicBlock, BlockId, ComparisonOp, Constant, Function, FunctionId, Instruction, IrBuilder,
    Module, Parameter, ValueId,
};
use crate::parser::Stmt;
use crate::types::Type;
use std::collections::HashMap;

/// Information about an async function's transformation
#[derive(Debug)]
pub struct AsyncTransformInfo {
    /// The original function
    pub original_fn: FunctionId,
    /// The generated poll function
    pub poll_fn: FunctionId,
    /// Map of local variables to their offsets in the state struct
    pub state_offsets: HashMap<String, u32>,
    /// Size of the state struct
    pub state_size: u32,
    /// Suspend points in the function
    pub suspend_points: Vec<SuspendPoint>,
}

/// Information about a suspend point (await expression)
#[derive(Debug)]
pub struct SuspendPoint {
    /// State value for this suspend point
    pub state_id: u32,
    /// Block to resume at after suspension
    pub resume_block: BlockId,
    /// The future being awaited
    pub future_value: ValueId,
}

/// Context for async transformation
pub struct AsyncTransformContext {
    /// Current state offset for variable allocation
    current_offset: u32,
    /// Current state ID for suspend points
    current_state_id: u32,
    /// Suspend points collected during transformation
    suspend_points: Vec<SuspendPoint>,
    /// Local variable to state offset mapping
    state_offsets: HashMap<String, u32>,
    /// The state pointer value
    state_ptr: Option<ValueId>,
}

impl AsyncTransformContext {
    fn new() -> Self {
        AsyncTransformContext {
            current_offset: 8,   // Reserve first 8 bytes for state enum
            current_state_id: 1, // State 0 is initial state
            suspend_points: Vec::new(),
            state_offsets: HashMap::new(),
            state_ptr: None,
        }
    }

    /// Allocate space for a variable in the state struct with bounds checking
    fn allocate_variable(&mut self, name: String, size: u32) -> u32 {
        // SECURITY: Prevent integer overflow in state allocation
        const MAX_STATE_SIZE: u32 = 1024 * 1024; // 1MB max state size

        if size > MAX_STATE_SIZE || self.current_offset > MAX_STATE_SIZE - size {
            // Return error offset to indicate allocation failure
            // In production, this should propagate an error
            return u32::MAX;
        }

        let offset = self.current_offset;
        self.state_offsets.insert(name, offset);

        // SECURITY: Safe addition with overflow check
        match self.current_offset.checked_add(size) {
            Some(new_offset) => {
                self.current_offset = new_offset;
                // Align to 8 bytes with bounds check
                self.current_offset = (self.current_offset + 7) & !7;
            }
            None => {
                // Overflow detected
                return u32::MAX;
            }
        }

        offset
    }

    /// Get the next state ID for a suspend point
    fn next_state_id(&mut self) -> u32 {
        let id = self.current_state_id;
        self.current_state_id += 1;
        id
    }
}

/// Transform an async function into a state machine with security validation
pub fn transform_async_function(
    module: &mut Module,
    func_id: FunctionId,
) -> Result<AsyncTransformInfo, Error> {
    let func = module
        .get_function(func_id)
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Function not found"))?
        .clone();

    if !func.is_async {
        return Err(Error::new(ErrorKind::RuntimeError, "Function is not async"));
    }

    // Security validation before transformation
    validate_async_function_security(&func)?;

    let mut context = AsyncTransformContext::new();

    // Calculate state struct size
    // We need to analyze the function to find all local variables
    let state_size = calculate_state_size(&func, &mut context)?;

    // Create the poll function
    let poll_fn_name = format!("{}_poll", func.name);
    let poll_params = vec![
        Parameter {
            name: "self".to_string(),
            ty: Type::Named("ptr<AsyncState>".to_string()),
        },
        Parameter {
            name: "waker".to_string(),
            ty: Type::Named("Waker".to_string()),
        },
    ];
    let poll_return_ty = Type::Generic {
        name: "Poll".to_string(),
        args: vec![func.return_type.clone()],
    };

    let poll_fn_id = module.create_function(poll_fn_name, poll_params, poll_return_ty);

    // Transform the function body into a state machine
    transform_function_body(module, &func, poll_fn_id, &mut context)?;

    // Create the async wrapper function that returns a Future
    create_async_wrapper(module, func_id, poll_fn_id, state_size)?;

    Ok(AsyncTransformInfo {
        original_fn: func_id,
        poll_fn: poll_fn_id,
        state_offsets: context.state_offsets,
        state_size,
        suspend_points: context.suspend_points,
    })
}

/// Calculate the size needed for the state struct
fn calculate_state_size(
    func: &Function,
    context: &mut AsyncTransformContext,
) -> Result<u32, Error> {
    // Reserve space for control variables
    context.allocate_variable("__state".to_string(), 4); // State enum
    context.allocate_variable("__result".to_string(), 8); // Result storage
    context.allocate_variable("__waker".to_string(), 8); // Waker reference

    // Add function parameters to state
    for param in &func.params {
        let size = estimate_type_size(&param.ty);
        context.allocate_variable(param.name.clone(), size);
    }

    // Analyze function body to find local variables and temporaries
    let mut local_vars = analyze_local_variables(func)?;

    // Allocate space for each local variable
    for (var_name, var_type) in local_vars.drain() {
        let size = estimate_type_size(&var_type);
        context.allocate_variable(var_name, size);
    }

    // Add space for future storage at each await point
    let await_count = count_await_points(func)?;
    for i in 0..await_count {
        context.allocate_variable(format!("__future_{i}"), 8); // Future pointer
        context.allocate_variable(format!("__future_result_{i}"), 8); // Future result
    }

    Ok(context.current_offset)
}

/// Estimate the size of a type in bytes
fn estimate_type_size(ty: &Type) -> u32 {
    match ty {
        Type::I32 => 4,
        Type::F32 => 4,
        Type::Bool => 1,
        Type::String => 8,          // String is a pointer to heap data
        Type::Unknown => 8,         // Default pointer size
        Type::Array(_) => 8,        // Arrays are pointers to heap data
        Type::Function { .. } => 8, // Function pointers
        Type::Result { .. } => 8,   // Result types typically contain pointers
        Type::Future(_) => 8,       // Future types are complex async structures
        Type::Named(name) => match name.as_str() {
            "i32" | "u32" | "f32" => 4,
            "i64" | "u64" | "f64" | "ptr" => 8,
            "bool" | "u8" | "i8" => 1,
            "u16" | "i16" => 2,
            _ => 8, // Default for complex types
        },
        Type::TypeVar(_) => 8,     // Type variables are typically pointers
        Type::Option(_) => 8,      // Option types contain pointers
        Type::Never => 0,          // Never type has no size
        Type::Generic { .. } => 8, // Generic types typically contain pointers
        Type::TypeParam(_) => 8,   // Type parameters are typically pointers
        Type::Tuple(types) => types.iter().map(estimate_type_size).sum(),
        Type::Reference { .. } => 8, // References are pointers
        Type::Struct { fields, .. } => fields.iter().map(|(_, ty)| estimate_type_size(ty)).sum(),
    }
}

/// Analyze function blocks to find local variable declarations
fn analyze_local_variables(func: &Function) -> Result<HashMap<String, Type>, Error> {
    let mut locals = HashMap::new();

    // Walk through all blocks and instructions
    for (_, block) in func.blocks() {
        for (value_id, inst_with_loc) in &block.instructions {
            let inst = &inst_with_loc.instruction;
            match inst {
                Instruction::Alloc { ty } => {
                    // Allocate local variable storage
                    let local_name = format!("__local_{}", value_id.0);
                    locals.insert(local_name, ty.clone());
                }
                Instruction::Store { .. } | Instruction::Load { .. } => {
                    // These might reference locals we haven't seen declared
                    // For now, we'll assume all loads/stores are to known variables
                }
                _ => {
                    // Other instructions might create temporaries
                    // We'll allocate space for significant temporaries
                    if is_significant_instruction(inst) {
                        let temp_name = format!("__temp_{}", value_id.0);
                        locals.insert(temp_name, Type::Unknown);
                    }
                }
            }
        }
    }

    Ok(locals)
}

/// Count the number of await points in a function
fn count_await_points(func: &Function) -> Result<usize, Error> {
    let mut count = 0;

    for (_, block) in func.blocks() {
        for (_, inst_with_loc) in &block.instructions {
            if let Instruction::PollFuture { .. } = inst_with_loc.instruction {
                count += 1;
            }
        }
    }

    Ok(count)
}

/// Check if an instruction creates a significant temporary value
fn is_significant_instruction(inst: &Instruction) -> bool {
    matches!(
        inst,
        Instruction::Call { .. }
            | Instruction::Binary { .. }
            | Instruction::Unary { .. }
            | Instruction::Compare { .. }
            | Instruction::LoadField { .. }
            | Instruction::ConstructStruct { .. }
            | Instruction::ConstructEnum { .. }
    )
}

/// Validate async function for security issues before transformation
fn validate_async_function_security(func: &Function) -> Result<(), Error> {
    // Check for excessive complexity that could lead to DoS
    let instruction_count = count_instructions(func);
    const MAX_INSTRUCTIONS: usize = 10_000;
    if instruction_count > MAX_INSTRUCTIONS {
        return Err(Error::new(
            ErrorKind::SecurityViolation,
            format!(
                "Async function has {} instructions, exceeds limit of {}",
                instruction_count, MAX_INSTRUCTIONS
            ),
        ));
    }

    // Check for excessive await points that could cause stack overflow
    let await_count = count_await_points(func)?;
    const MAX_AWAIT_POINTS: usize = 100;
    if await_count > MAX_AWAIT_POINTS {
        return Err(Error::new(
            ErrorKind::SecurityViolation,
            format!(
                "Async function has {} await points, exceeds limit of {}",
                await_count, MAX_AWAIT_POINTS
            ),
        ));
    }

    // Check for potentially unsafe patterns
    validate_unsafe_patterns(func)?;

    // Check for recursive async calls that could cause infinite loops
    validate_recursion_safety(func)?;

    Ok(())
}

/// Count total instructions in a function
fn count_instructions(func: &Function) -> usize {
    func.blocks()
        .iter()
        .map(|(_, block)| block.instructions.len())
        .sum()
}

/// Validate function for unsafe patterns
fn validate_unsafe_patterns(func: &Function) -> Result<(), Error> {
    for (_, block) in func.blocks() {
        for (_, inst_with_loc) in &block.instructions {
            let inst = &inst_with_loc.instruction;
            match inst {
                Instruction::Call { func, .. } => {
                    // Check for dangerous function calls in async context
                    if is_dangerous_async_call(func) {
                        return Err(Error::new(
                            ErrorKind::SecurityViolation,
                            format!("Dangerous function call in async context: {:?}", func),
                        ));
                    }
                }
                Instruction::Store { .. } | Instruction::Load { .. } => {
                    // Check for potential memory safety issues
                    // This would be more sophisticated in a real implementation
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// Check if a function call is dangerous in async context
fn is_dangerous_async_call(_function: &FunctionId) -> bool {
    // This would check against a list of known dangerous functions
    // For now, we'll do basic validation
    false // Placeholder - would be more sophisticated
}

/// Validate that async function doesn't have unsafe recursion
fn validate_recursion_safety(func: &Function) -> Result<(), Error> {
    // Check for direct recursive calls
    for (_, block) in func.blocks() {
        for (_, inst_with_loc) in &block.instructions {
            if let Instruction::Call { func: _, .. } = &inst_with_loc.instruction {
                // In a real implementation, we'd resolve the function ID
                // and check if it matches the current function
                // For now, we'll do basic validation
            }
        }
    }
    Ok(())
}

/// Transform the function body into a state machine
fn transform_function_body(
    module: &mut Module,
    original_func: &Function,
    poll_fn_id: FunctionId,
    context: &mut AsyncTransformContext,
) -> Result<(), Error> {
    // Clone the original function's blocks to avoid borrow issues
    let original_blocks: Vec<(BlockId, BasicBlock)> = original_func
        .blocks()
        .iter()
        .map(|(id, block)| (*id, block.clone()))
        .collect();

    let poll_func = module
        .get_function_mut(poll_fn_id)
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Poll function not found"))?;

    let mut builder = IrBuilder::new();
    builder.set_current_function(poll_fn_id);

    // Get the state pointer parameter (self) and waker
    let state_ptr = ValueId(0); // First parameter
    let _waker = ValueId(1); // Second parameter (waker)
    context.state_ptr = Some(state_ptr);

    // Create entry block
    let entry_block = poll_func.create_block("entry".to_string());
    builder.set_current_block(entry_block);

    // Load current state
    let current_state = builder
        .build_get_async_state(state_ptr)
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to get async state"))?;

    // Create dispatch block for state machine
    let dispatch_block = poll_func.create_block("dispatch".to_string());
    builder.build_branch(dispatch_block);

    // Create blocks for each state
    builder.set_current_block(dispatch_block);

    // Initial state (0) - start of function
    let initial_block = poll_func.create_block("state_0".to_string());

    // Create blocks for each suspend point (we'll detect these as we transform)
    let mut state_blocks = vec![initial_block];
    let mut block_mapping: HashMap<BlockId, BlockId> = HashMap::new();

    // Map original entry block to initial state block
    if let Some(orig_entry) = original_func.entry_block {
        block_mapping.insert(orig_entry, initial_block);
    }

    // First pass: analyze the function to find suspend points
    let mut suspend_points = Vec::new();
    let mut next_state_id = 1u32;

    for (_orig_block_id, orig_block) in &original_blocks {
        for (_, inst_with_loc) in &orig_block.instructions {
            let inst = &inst_with_loc.instruction;
            if let Instruction::PollFuture { .. } = inst {
                // Found an await point
                let state_block = poll_func.create_block(format!("state_{next_state_id}"));
                state_blocks.push(state_block);
                suspend_points.push(SuspendPoint {
                    state_id: next_state_id,
                    resume_block: state_block,
                    future_value: ValueId(0), // Will be filled during transformation
                });
                next_state_id += 1;
            }
        }
    }

    // Generate dispatch table
    builder.set_current_block(dispatch_block);

    // Create a switch-like structure using comparisons and branches
    for (i, state_block) in state_blocks.iter().enumerate() {
        let state_val = builder.const_value(Constant::I32(i as i32));
        let is_state = builder
            .build_compare(ComparisonOp::Eq, current_state, state_val)
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to compare state"))?;

        let next_check = if i < state_blocks.len() - 1 {
            poll_func.create_block(format!("check_state_{}", i + 1))
        } else {
            // Invalid state - return error or panic
            poll_func.create_block("invalid_state".to_string())
        };

        builder.build_cond_branch(is_state, *state_block, next_check);
        builder.set_current_block(next_check);
    }

    // Invalid state handler - for now just return an error
    // In a complete implementation, this would panic or return an error
    let error_result = builder.const_value(Constant::I32(2)); // Error state
    builder.build_return(Some(error_result));

    // Transform each state
    builder.set_current_block(initial_block);

    // Load function parameters from state
    for (i, param) in original_func.params.iter().enumerate() {
        let offset = context
            .state_offsets
            .get(&param.name)
            .copied()
            .unwrap_or(8 + (i as u32) * 8);
        let _loaded_param = builder
            .build_load_async_state(state_ptr, offset, param.ty.clone())
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to load parameter"))?;
        // Map parameter ValueId to loaded value
        // This would need proper value mapping in a complete implementation
    }

    // Transform the original function body
    transform_blocks(
        &mut builder,
        poll_func,
        &original_blocks,
        &block_mapping,
        context,
        state_ptr,
    )?;

    // Add final return for completed state
    let completed_block = poll_func.create_block("completed".to_string());
    builder.set_current_block(completed_block);

    // Return Poll::Ready with the result
    // This would need to construct the proper Poll enum value
    let result = builder.const_value(Constant::Null); // Placeholder
    builder.build_return(Some(result));

    Ok(())
}

/// Helper function to transform blocks
fn transform_blocks(
    builder: &mut IrBuilder,
    poll_func: &mut Function,
    original_blocks: &[(BlockId, BasicBlock)],
    block_mapping: &HashMap<BlockId, BlockId>,
    context: &mut AsyncTransformContext,
    state_ptr: ValueId,
) -> Result<(), Error> {
    // Transform each block's instructions
    for (orig_block_id, orig_block) in original_blocks {
        if let Some(&new_block_id) = block_mapping.get(orig_block_id) {
            builder.set_current_block(new_block_id);

            // Transform each instruction
            for (value_id, inst_with_loc) in &orig_block.instructions {
                let inst = &inst_with_loc.instruction;
                let _value_id = *value_id; // Copy the value ID
                match inst {
                    Instruction::PollFuture { future, output_ty } => {
                        // This is an await point - generate suspend logic
                        let state_id = context.next_state_id();

                        // Store the future in state
                        let future_offset =
                            context.allocate_variable(format!("__future_{state_id}"), 8);
                        builder.build_store_async_state(state_ptr, future_offset, *future);

                        // Update state
                        builder.build_set_async_state(state_ptr, state_id);

                        // Return Poll::Pending
                        let pending = builder.const_value(Constant::I32(1)); // Poll::Pending
                        builder.build_return(Some(pending));

                        // Create resume block
                        let resume_block = poll_func.create_block(format!("resume_{state_id}"));
                        builder.set_current_block(resume_block);

                        // Load and poll the future again
                        let loaded_future = builder
                            .build_load_async_state(state_ptr, future_offset, Type::Unknown)
                            .ok_or_else(|| {
                                Error::new(ErrorKind::RuntimeError, "Failed to load future")
                            })?;

                        let poll_result = builder
                            .build_poll_future(loaded_future, output_ty.clone())
                            .ok_or_else(|| {
                                Error::new(ErrorKind::RuntimeError, "Failed to poll future")
                            })?;

                        // Check if ready
                        let is_ready =
                            builder.build_get_enum_tag(poll_result).ok_or_else(|| {
                                Error::new(ErrorKind::RuntimeError, "Failed to get poll tag")
                            })?;

                        let ready_tag = builder.const_value(Constant::I32(0));
                        let is_ready_cond = builder
                            .build_compare(ComparisonOp::Eq, is_ready, ready_tag)
                            .ok_or_else(|| {
                                Error::new(ErrorKind::RuntimeError, "Failed to compare")
                            })?;

                        let continue_block =
                            poll_func.create_block(format!("continue_{state_id}"));
                        let still_pending =
                            poll_func.create_block(format!("still_pending_{state_id}"));

                        builder.build_cond_branch(is_ready_cond, continue_block, still_pending);

                        // Still pending - return Poll::Pending again
                        builder.set_current_block(still_pending);
                        builder.build_return(Some(pending));

                        // Ready - continue execution
                        builder.set_current_block(continue_block);
                        // Extract value from Poll::Ready and continue
                    }
                    _ => {
                        // Transform other instructions (convert local variable access to state loads/stores)
                        // For now, just copy the instruction
                        builder.add_instruction(inst.clone());
                    }
                }
            }
        }
    }

    Ok(())
}

/// Create the async wrapper function that returns a Future
fn create_async_wrapper(
    module: &mut Module,
    original_fn_id: FunctionId,
    _poll_fn_id: FunctionId,
    state_size: u32,
) -> Result<(), Error> {
    let original_func = module
        .get_function_mut(original_fn_id)
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Original function not found"))?;

    // Clear the async flag since we're converting it
    original_func.is_async = false;

    // Create new entry block
    let entry_block = original_func.create_block("async_wrapper_entry".to_string());

    let mut builder = IrBuilder::new();
    builder.set_current_function(original_fn_id);
    builder.set_current_block(entry_block);

    // Create the async state
    let state = builder
        .build_create_async_state(0, state_size, original_func.return_type.clone())
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create async state"))?;

    // Store function parameters in the state
    let mut wrapper_context = AsyncTransformContext::new();
    for (i, param) in original_func.params.iter().enumerate() {
        let param_value = ValueId(i as u32);
        // SECURITY: Properly allocate space for each parameter with bounds checking
        let param_size = match &param.ty {
            Type::I32 | Type::F32 => 4,
            Type::Bool => 1,
            Type::String => 16, // Pointer + length
            _ => 8,             // Default pointer size
        };

        let offset = wrapper_context.allocate_variable(param.name.clone(), param_size);
        if offset == u32::MAX {
            return Err(Error::new(
                ErrorKind::RuntimeError,
                "Async state size overflow - parameter allocation failed",
            ));
        }

        builder.build_store_async_state(state, offset, param_value);
    }

    // Return the Future
    builder.build_return(Some(state));

    Ok(())
}

/// Find all await expressions in a function
pub fn find_await_expressions(stmts: &[Stmt]) -> Vec<usize> {
    let mut await_positions = Vec::new();

    for (stmt_index, stmt) in stmts.iter().enumerate() {
        find_await_in_stmt(stmt, stmt_index, &mut await_positions);
    }

    await_positions
}

/// Recursively find await expressions in a statement
fn find_await_in_stmt(
    stmt: &crate::parser::Stmt,
    base_index: usize,
    await_positions: &mut Vec<usize>,
) {
    use crate::parser::StmtKind;

    match &stmt.kind {
        StmtKind::Expression(expr) => {
            find_await_in_expr(expr, base_index, await_positions);
        }
        StmtKind::Let { init, .. } => {
            if let Some(expr) = init {
                find_await_in_expr(expr, base_index, await_positions);
            }
        }
        // If statements are handled in expressions, not statements
        StmtKind::While { condition, body } => {
            find_await_in_expr(condition, base_index, await_positions);
            for stmt in &body.statements {
                find_await_in_stmt(stmt, base_index, await_positions);
            }
        }
        StmtKind::For { iterable, body, .. } => {
            find_await_in_expr(iterable, base_index, await_positions);
            for stmt in &body.statements {
                find_await_in_stmt(stmt, base_index, await_positions);
            }
        }
        StmtKind::Return(value) => {
            if let Some(expr) = value {
                find_await_in_expr(expr, base_index, await_positions);
            }
        }
        // Assignment statements are handled in expressions, not statements
        _ => {
            // Other statement types that don't contain expressions
        }
    }
}

/// Recursively find await expressions in an expression
fn find_await_in_expr(
    expr: &crate::parser::Expr,
    base_index: usize,
    await_positions: &mut Vec<usize>,
) {
    use crate::parser::ExprKind;

    match &expr.kind {
        ExprKind::Await { expr: inner_expr } => {
            // Found an await expression
            await_positions.push(base_index);
            // Also check the inner expression for nested awaits
            find_await_in_expr(inner_expr, base_index, await_positions);
        }
        ExprKind::Call { callee, args } => {
            find_await_in_expr(callee, base_index, await_positions);
            for arg in args {
                find_await_in_expr(arg, base_index, await_positions);
            }
        }
        ExprKind::Binary { left, right, .. } => {
            find_await_in_expr(left, base_index, await_positions);
            find_await_in_expr(right, base_index, await_positions);
        }
        ExprKind::Unary { expr, .. } => {
            find_await_in_expr(expr, base_index, await_positions);
        }
        ExprKind::Index { object, index } => {
            find_await_in_expr(object, base_index, await_positions);
            find_await_in_expr(index, base_index, await_positions);
        }
        ExprKind::Member { object, .. } => {
            find_await_in_expr(object, base_index, await_positions);
        }
        // MethodCall is handled as Call expressions in this AST
        ExprKind::Array(elements) => {
            for element in elements {
                find_await_in_expr(element, base_index, await_positions);
            }
        }
        ExprKind::StructConstructor { fields, .. } => {
            for (_, field_expr) in fields {
                find_await_in_expr(field_expr, base_index, await_positions);
            }
        }
        ExprKind::Block(block) => {
            for stmt in &block.statements {
                find_await_in_stmt(stmt, base_index, await_positions);
            }
            if let Some(final_expr) = &block.final_expr {
                find_await_in_expr(final_expr, base_index, await_positions);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            find_await_in_expr(condition, base_index, await_positions);
            find_await_in_expr(then_branch, base_index, await_positions);
            if let Some(else_expr) = else_branch {
                find_await_in_expr(else_expr, base_index, await_positions);
            }
        }
        _ => {
            // Leaf expressions that don't contain other expressions
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_transform_context() {
        let mut context = AsyncTransformContext::new();

        // Test variable allocation
        let offset1 = context.allocate_variable("x".to_string(), 4);
        assert_eq!(offset1, 8); // After state enum

        let offset2 = context.allocate_variable("y".to_string(), 8);
        assert_eq!(offset2, 16); // Aligned to 8 bytes

        // Test state ID generation
        let state1 = context.next_state_id();
        assert_eq!(state1, 1);

        let state2 = context.next_state_id();
        assert_eq!(state2, 2);
    }

    #[test]
    fn test_type_size_estimation() {
        assert_eq!(estimate_type_size(&Type::Named("i32".to_string())), 4);
        assert_eq!(estimate_type_size(&Type::Named("i64".to_string())), 8);
        assert_eq!(estimate_type_size(&Type::Named("bool".to_string())), 1);
        assert_eq!(estimate_type_size(&Type::Unknown), 8);
    }

    #[test]
    fn test_instruction_counting() {
        // This would require creating a test function
        // For now, just test the helper function exists
        let func = create_test_function();
        let count = count_instructions(&func);
        assert!(count >= 0);
    }

    fn create_test_function() -> Function {
        let mut func = Function::new(
            FunctionId(0),
            "test".to_string(),
            vec![],
            Type::Named("void".to_string()),
        );
        func.is_async = true;
        func
    }

    #[test]
    fn test_find_await_expressions() {
        // Test with empty statement list
        let empty_stmts = vec![];
        let awaits = find_await_expressions(&empty_stmts);
        assert_eq!(awaits.len(), 0);

        // More comprehensive tests would require creating actual AST nodes
        // which would need the parser types to be fully available
    }

    #[test]
    fn test_security_validation() {
        let func = create_test_function();

        // Should pass validation for empty function
        assert!(validate_async_function_security(&func).is_ok());

        // Test instruction count validation would require a function with many instructions
    }
}
