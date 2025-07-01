use crate::parser::{Expr, ExprKind, Literal, BinaryOp as AstBinaryOp, UnaryOp as AstUnaryOp, MatchArm, Pattern, PatternKind};
use crate::ir::{ValueId, Constant, BinaryOp as IrBinaryOp, UnaryOp as IrUnaryOp, ComparisonOp, Instruction};
use crate::types::Type;
use crate::error::{Error, ErrorKind};
use super::{AstLowerer, LoweringResult};

/// Lower an expression to IR
pub fn lower_expression(lowerer: &mut AstLowerer, expr: &Expr) -> LoweringResult<ValueId> {
    match &expr.kind {
        ExprKind::Literal(lit) => lower_literal(lowerer, lit),
        ExprKind::Identifier(name) => lower_identifier(lowerer, name),
        ExprKind::Binary { left, op, right } => lower_binary(lowerer, left, op, right),
        ExprKind::Unary { op, expr } => lower_unary(lowerer, op, expr),
        ExprKind::Call { callee, args } => lower_call(lowerer, callee, args),
        ExprKind::If { condition, then_branch, else_branch } => {
            lower_if(lowerer, condition, then_branch, else_branch.as_deref())
        }
        ExprKind::Block(block) => {
            lowerer.lower_block(block)?
                .ok_or_else(|| Error::new(ErrorKind::TypeError, "Block expression must produce a value"))
        }
        ExprKind::Array(elements) => lower_array(lowerer, elements),
        ExprKind::Index { object, index } => lower_index(lowerer, object, index),
        ExprKind::Member { object, property } => lower_member(lowerer, object, property),
        ExprKind::Assign { target, value } => lower_assign(lowerer, target, value),
        ExprKind::Match { expr, arms } => lower_match(lowerer, expr, arms),
    }
}

/// Lower a literal to IR
fn lower_literal(lowerer: &mut AstLowerer, literal: &Literal) -> LoweringResult<ValueId> {
    let constant = match literal {
        Literal::Number(n) => {
            // Determine if it's an integer or float based on presence of decimal
            if n.fract() == 0.0 && n.abs() <= i32::MAX as f64 {
                Constant::I32(*n as i32)
            } else {
                Constant::F32(*n as f32)
            }
        }
        Literal::String(s) => Constant::String(s.clone()),
        Literal::Boolean(b) => Constant::Bool(*b),
    };
    
    Ok(lowerer.builder.const_value(constant))
}

/// Lower an identifier (variable reference)
fn lower_identifier(lowerer: &mut AstLowerer, name: &str) -> LoweringResult<ValueId> {
    // Look up the variable
    if let Some(var) = lowerer.context.lookup_variable(name) {
        // Load the value from the variable's memory location
        lowerer.builder.build_load(var.ptr, var.ty.clone())
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to load variable"))
    } else {
        Err(Error::new(ErrorKind::TypeError, format!("Undefined variable: {}", name)))
    }
}

/// Lower a binary operation
fn lower_binary(
    lowerer: &mut AstLowerer,
    left: &Expr,
    op: &AstBinaryOp,
    right: &Expr,
) -> LoweringResult<ValueId> {
    let lhs = lower_expression(lowerer, left)?;
    let rhs = lower_expression(lowerer, right)?;
    
    // Get the type of the operation
    let ty = lowerer.get_expression_type(left)?;
    
    match op {
        AstBinaryOp::Add => {
            lowerer.builder.build_binary(IrBinaryOp::Add, lhs, rhs, ty)
                .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to build add"))
        }
        AstBinaryOp::Subtract => {
            lowerer.builder.build_binary(IrBinaryOp::Sub, lhs, rhs, ty)
                .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to build sub"))
        }
        AstBinaryOp::Multiply => {
            lowerer.builder.build_binary(IrBinaryOp::Mul, lhs, rhs, ty)
                .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to build mul"))
        }
        AstBinaryOp::Divide => {
            lowerer.builder.build_binary(IrBinaryOp::Div, lhs, rhs, ty)
                .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to build div"))
        }
        AstBinaryOp::Modulo => {
            lowerer.builder.build_binary(IrBinaryOp::Mod, lhs, rhs, ty)
                .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to build mod"))
        }
        AstBinaryOp::Equal => {
            lowerer.builder.build_compare(ComparisonOp::Eq, lhs, rhs)
                .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to build eq"))
        }
        AstBinaryOp::NotEqual => {
            lowerer.builder.build_compare(ComparisonOp::Ne, lhs, rhs)
                .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to build ne"))
        }
        AstBinaryOp::Less => {
            lowerer.builder.build_compare(ComparisonOp::Lt, lhs, rhs)
                .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to build lt"))
        }
        AstBinaryOp::LessEqual => {
            lowerer.builder.build_compare(ComparisonOp::Le, lhs, rhs)
                .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to build le"))
        }
        AstBinaryOp::Greater => {
            lowerer.builder.build_compare(ComparisonOp::Gt, lhs, rhs)
                .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to build gt"))
        }
        AstBinaryOp::GreaterEqual => {
            lowerer.builder.build_compare(ComparisonOp::Ge, lhs, rhs)
                .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to build ge"))
        }
        AstBinaryOp::And => {
            // Short-circuit AND
            lower_short_circuit_and(lowerer, left, right)
        }
        AstBinaryOp::Or => {
            // Short-circuit OR
            lower_short_circuit_or(lowerer, left, right)
        }
    }
}

/// Lower a unary operation
fn lower_unary(
    lowerer: &mut AstLowerer,
    op: &AstUnaryOp,
    expr: &Expr,
) -> LoweringResult<ValueId> {
    let operand = lower_expression(lowerer, expr)?;
    let ty = lowerer.get_expression_type(expr)?;
    
    match op {
        AstUnaryOp::Negate => {
            lowerer.builder.build_unary(IrUnaryOp::Neg, operand, ty)
                .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to build neg"))
        }
        AstUnaryOp::Not => {
            lowerer.builder.build_unary(IrUnaryOp::Not, operand, Type::Bool)
                .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to build not"))
        }
    }
}

/// Lower a function call
fn lower_call(
    lowerer: &mut AstLowerer,
    callee: &Expr,
    args: &[Expr],
) -> LoweringResult<ValueId> {
    // For now, only support direct function calls
    if let ExprKind::Identifier(func_name) = &callee.kind {
        // Lower arguments
        let arg_values: Vec<ValueId> = args.iter()
            .map(|arg| lower_expression(lowerer, arg))
            .collect::<Result<Vec<_>, _>>()?;
        
        // Check if it's a built-in function
        if func_name == "print" {
            // Special handling for print
            if args.len() != 1 {
                return Err(Error::new(ErrorKind::TypeError, "print expects exactly one argument"));
            }
            
            // For now, just return the argument
            // In a real implementation, this would generate a call to the runtime print function
            return Ok(arg_values[0]);
        }
        
        // Look up the function
        if let Some(func_id) = lowerer.context.get_function(func_name) {
            // Get the function's return type
            let return_type = Type::Unknown; // TODO: Get actual return type
            
            lowerer.builder.build_call(func_id, arg_values, return_type)
                .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to build call"))
        } else {
            Err(Error::new(ErrorKind::TypeError, format!("Undefined function: {}", func_name)))
        }
    } else {
        Err(Error::new(ErrorKind::TypeError, "Indirect function calls not yet supported"))
    }
}

/// Lower an if expression
fn lower_if(
    lowerer: &mut AstLowerer,
    condition: &Expr,
    then_branch: &Expr,
    else_branch: Option<&Expr>,
) -> LoweringResult<ValueId> {
    // Create blocks
    let then_block = lowerer.builder.create_block("if.then".to_string())
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create then block"))?;
    let else_block = lowerer.builder.create_block("if.else".to_string())
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create else block"))?;
    let merge_block = lowerer.builder.create_block("if.merge".to_string())
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create merge block"))?;
    
    // Evaluate condition
    let cond_value = lower_expression(lowerer, condition)?;
    lowerer.builder.build_cond_branch(cond_value, then_block, else_block);
    
    // Then block
    lowerer.builder.set_current_block(then_block);
    let then_value = lower_expression(lowerer, then_branch)?;
    let then_end_block = lowerer.builder.get_current_block().unwrap(); // Block might have changed
    lowerer.builder.build_branch(merge_block);
    
    // Else block
    lowerer.builder.set_current_block(else_block);
    let else_value = if let Some(else_expr) = else_branch {
        lower_expression(lowerer, else_expr)?
    } else {
        // No else branch, use unit value
        lowerer.builder.const_value(Constant::Null)
    };
    let else_end_block = lowerer.builder.get_current_block().unwrap();
    lowerer.builder.build_branch(merge_block);
    
    // Merge block with phi node
    lowerer.builder.set_current_block(merge_block);
    let phi_inst = Instruction::Phi {
        incoming: vec![(then_value, then_end_block), (else_value, else_end_block)],
        ty: Type::Unknown, // TODO: Get actual type
    };
    
    lowerer.builder.add_instruction(phi_inst)
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create phi node"))
}

/// Lower short-circuit AND
fn lower_short_circuit_and(
    lowerer: &mut AstLowerer,
    left: &Expr,
    right: &Expr,
) -> LoweringResult<ValueId> {
    let check_right = lowerer.builder.create_block("and.rhs".to_string())
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create and.rhs block"))?;
    let merge = lowerer.builder.create_block("and.merge".to_string())
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create and.merge block"))?;
    
    // Evaluate left side
    let lhs = lower_expression(lowerer, left)?;
    let current_block = lowerer.builder.get_current_block().unwrap();
    lowerer.builder.build_cond_branch(lhs, check_right, merge);
    
    // Right side block
    lowerer.builder.set_current_block(check_right);
    let rhs = lower_expression(lowerer, right)?;
    let rhs_block = lowerer.builder.get_current_block().unwrap();
    lowerer.builder.build_branch(merge);
    
    // Merge block
    lowerer.builder.set_current_block(merge);
    let phi = Instruction::Phi {
        incoming: vec![(lhs, current_block), (rhs, rhs_block)],
        ty: Type::Bool,
    };
    
    lowerer.builder.add_instruction(phi)
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create phi for AND"))
}

/// Lower short-circuit OR
fn lower_short_circuit_or(
    lowerer: &mut AstLowerer,
    left: &Expr,
    right: &Expr,
) -> LoweringResult<ValueId> {
    let check_right = lowerer.builder.create_block("or.rhs".to_string())
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create or.rhs block"))?;
    let merge = lowerer.builder.create_block("or.merge".to_string())
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create or.merge block"))?;
    
    // Evaluate left side
    let lhs = lower_expression(lowerer, left)?;
    let current_block = lowerer.builder.get_current_block().unwrap();
    lowerer.builder.build_cond_branch(lhs, merge, check_right);
    
    // Right side block
    lowerer.builder.set_current_block(check_right);
    let rhs = lower_expression(lowerer, right)?;
    let rhs_block = lowerer.builder.get_current_block().unwrap();
    lowerer.builder.build_branch(merge);
    
    // Merge block
    lowerer.builder.set_current_block(merge);
    let phi = Instruction::Phi {
        incoming: vec![(lhs, current_block), (rhs, rhs_block)],
        ty: Type::Bool,
    };
    
    lowerer.builder.add_instruction(phi)
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create phi for OR"))
}

/// Lower array creation
fn lower_array(lowerer: &mut AstLowerer, elements: &[Expr]) -> LoweringResult<ValueId> {
    // For now, return a placeholder
    // In a real implementation, this would allocate an array and initialize elements
    let _element_values: Vec<ValueId> = elements.iter()
        .map(|elem| lower_expression(lowerer, elem))
        .collect::<Result<Vec<_>, _>>()?;
    
    // TODO: Implement proper array creation
    Ok(lowerer.builder.const_value(Constant::Null))
}

/// Lower array indexing
fn lower_index(
    lowerer: &mut AstLowerer,
    object: &Expr,
    index: &Expr,
) -> LoweringResult<ValueId> {
    let _array_value = lower_expression(lowerer, object)?;
    let _index_value = lower_expression(lowerer, index)?;
    
    // TODO: Implement proper array indexing
    Ok(lowerer.builder.const_value(Constant::Null))
}

/// Lower member access
fn lower_member(
    lowerer: &mut AstLowerer,
    object: &Expr,
    _property: &str,
) -> LoweringResult<ValueId> {
    let _object_value = lower_expression(lowerer, object)?;
    
    // TODO: Implement proper member access
    Ok(lowerer.builder.const_value(Constant::Null))
}

/// Lower assignment
fn lower_assign(
    lowerer: &mut AstLowerer,
    target: &Expr,
    value: &Expr,
) -> LoweringResult<ValueId> {
    let value_id = lower_expression(lowerer, value)?;
    
    match &target.kind {
        ExprKind::Identifier(name) => {
            if let Some(var) = lowerer.context.lookup_variable(name) {
                lowerer.builder.build_store(var.ptr, value_id);
                Ok(value_id)
            } else {
                Err(Error::new(ErrorKind::TypeError, format!("Undefined variable: {}", name)))
            }
        }
        _ => Err(Error::new(ErrorKind::TypeError, "Invalid assignment target")),
    }
}

/// Lower a match expression
fn lower_match(
    lowerer: &mut AstLowerer,
    expr: &Expr,
    arms: &[MatchArm],
) -> LoweringResult<ValueId> {
    // Evaluate the expression being matched
    let match_value = lower_expression(lowerer, expr)?;
    
    // Create blocks for the match arms and the final merge block
    let merge_block = lowerer.builder.create_block("match.merge".to_string())
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create match merge block"))?;
    
    // We'll store the values from each arm that need to be phi'd together
    let mut phi_incoming = Vec::new();
    
    // Create a block for the first arm test
    let mut current_test_block = lowerer.builder.get_current_block()
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "No current block"))?;
    
    for (i, arm) in arms.iter().enumerate() {
        let is_last_arm = i == arms.len() - 1;
        
        // Create blocks for this arm
        let arm_body_block = lowerer.builder.create_block(format!("match.arm{}.body", i))
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create arm body block"))?;
        
        let next_test_block = if !is_last_arm {
            Some(lowerer.builder.create_block(format!("match.arm{}.next", i))
                .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create next test block"))?)
        } else {
            None
        };
        
        // Set current block for pattern testing
        lowerer.builder.set_current_block(current_test_block);
        
        // Test the pattern and generate conditional branch
        let pattern_matches = lower_pattern_test(lowerer, &arm.pattern, match_value)?;
        
        // Test guard if present
        let final_condition = if let Some(guard) = &arm.guard {
            let guard_value = lower_expression(lowerer, guard)?;
            // AND the pattern match with the guard
            lowerer.builder.build_binary(IrBinaryOp::And, pattern_matches, guard_value, Type::Bool)
                .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to build guard AND"))?
        } else {
            pattern_matches
        };
        
        // Branch based on the final condition
        if let Some(next_block) = next_test_block {
            lowerer.builder.build_cond_branch(final_condition, arm_body_block, next_block);
        } else {
            // Last arm - if it doesn't match, we have a non-exhaustive match error
            // For now, just branch to the body (assuming exhaustive matching)
            lowerer.builder.build_branch(arm_body_block);
        }
        
        // Generate the arm body
        lowerer.builder.set_current_block(arm_body_block);
        
        // Bind pattern variables to the current scope
        bind_pattern_variables(lowerer, &arm.pattern, match_value)?;
        
        // Lower the arm body expression
        let arm_result = lower_expression(lowerer, &arm.body)?;
        let arm_end_block = lowerer.builder.get_current_block().unwrap();
        
        // Branch to merge block
        lowerer.builder.build_branch(merge_block);
        
        // Add to phi incoming values
        phi_incoming.push((arm_result, arm_end_block));
        
        // Move to next test block for the next iteration
        if let Some(next_block) = next_test_block {
            current_test_block = next_block;
        }
    }
    
    // Create the merge block with phi node
    lowerer.builder.set_current_block(merge_block);
    let phi_inst = Instruction::Phi {
        incoming: phi_incoming,
        ty: Type::Unknown, // TODO: Get actual result type from type inference
    };
    
    lowerer.builder.add_instruction(phi_inst)
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create match phi node"))
}

/// Lower a pattern test - returns a boolean value indicating if the pattern matches
fn lower_pattern_test(
    lowerer: &mut AstLowerer,
    pattern: &Pattern,
    value: ValueId,
) -> LoweringResult<ValueId> {
    match &pattern.kind {
        PatternKind::Wildcard => {
            // Wildcard always matches
            Ok(lowerer.builder.const_value(Constant::Bool(true)))
        }
        PatternKind::Literal(literal) => {
            // Compare the value with the literal
            let literal_value = match literal {
                Literal::Number(n) => {
                    if n.fract() == 0.0 && n.abs() <= i32::MAX as f64 {
                        lowerer.builder.const_value(Constant::I32(*n as i32))
                    } else {
                        lowerer.builder.const_value(Constant::F32(*n as f32))
                    }
                }
                Literal::String(s) => lowerer.builder.const_value(Constant::String(s.clone())),
                Literal::Boolean(b) => lowerer.builder.const_value(Constant::Bool(*b)),
            };
            
            // Generate equality comparison
            lowerer.builder.build_compare(ComparisonOp::Eq, value, literal_value)
                .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to build pattern literal comparison"))
        }
        PatternKind::Identifier(_name) => {
            // Variable binding pattern always matches
            Ok(lowerer.builder.const_value(Constant::Bool(true)))
        }
        PatternKind::Array(patterns) => {
            // For array patterns, we need to check length and each element
            // This is a simplified implementation
            // TODO: Implement proper array pattern matching with length checks
            
            let mut result = lowerer.builder.const_value(Constant::Bool(true));
            
            for (i, sub_pattern) in patterns.iter().enumerate() {
                // Get array element at index i
                let _index_value = lowerer.builder.const_value(Constant::I32(i as i32));
                
                // TODO: Generate array indexing IR instruction
                // For now, assume the element access succeeds and recursively test the pattern
                let element_test = lower_pattern_test(lowerer, sub_pattern, value)?;
                
                // AND with previous results
                result = lowerer.builder.build_binary(IrBinaryOp::And, result, element_test, Type::Bool)
                    .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to build array pattern AND"))?;
            }
            
            Ok(result)
        }
        PatternKind::Object(_fields) => {
            // Object pattern matching not fully implemented yet
            Ok(lowerer.builder.const_value(Constant::Bool(true)))
        }
        PatternKind::Or(patterns) => {
            // OR pattern - any sub-pattern can match
            let mut result = lowerer.builder.const_value(Constant::Bool(false));
            
            for sub_pattern in patterns {
                let sub_test = lower_pattern_test(lowerer, sub_pattern, value)?;
                result = lowerer.builder.build_binary(IrBinaryOp::Or, result, sub_test, Type::Bool)
                    .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to build OR pattern"))?;
            }
            
            Ok(result)
        }
    }
}

/// Bind pattern variables to the current scope
fn bind_pattern_variables(
    lowerer: &mut AstLowerer,
    pattern: &Pattern,
    value: ValueId,
) -> LoweringResult<()> {
    match &pattern.kind {
        PatternKind::Wildcard => {
            // No bindings for wildcard
            Ok(())
        }
        PatternKind::Literal(_) => {
            // No bindings for literals
            Ok(())
        }
        PatternKind::Identifier(name) => {
            // Create a variable binding
            let var_type = Type::Unknown; // TODO: Get actual type from pattern analysis
            let var_ptr = lowerer.builder.build_alloc(var_type.clone())
                .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to allocate pattern variable"))?;
            
            // Store the matched value
            lowerer.builder.build_store(var_ptr, value);
            
            // Add to the lowering context
            lowerer.context.define_variable(name.clone(), var_ptr, var_type);
            
            Ok(())
        }
        PatternKind::Array(patterns) => {
            // Bind each element of the array pattern
            for (_i, sub_pattern) in patterns.iter().enumerate() {
                // TODO: Extract array element at index i
                // For now, just recursively bind with the same value (simplified)
                bind_pattern_variables(lowerer, sub_pattern, value)?;
            }
            Ok(())
        }
        PatternKind::Object(_fields) => {
            // Object pattern binding not fully implemented yet
            Ok(())
        }
        PatternKind::Or(patterns) => {
            // For OR patterns, we would need to determine which pattern actually matched
            // This is complex and requires runtime support
            // For now, just bind the first pattern's variables
            if let Some(first_pattern) = patterns.first() {
                bind_pattern_variables(lowerer, first_pattern, value)?;
            }
            Ok(())
        }
    }
}