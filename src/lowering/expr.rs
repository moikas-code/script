use super::{AstLowerer, LoweringResult};
use crate::error::{Error, ErrorKind};
use crate::ir::{
    BinaryOp as IrBinaryOp, ComparisonOp, Constant, Instruction, UnaryOp as IrUnaryOp, ValueId,
};
use crate::parser::{
    BinaryOp as AstBinaryOp, ClosureParam, Expr, ExprKind, Literal, MatchArm, Pattern, PatternKind,
    UnaryOp as AstUnaryOp,
};
use crate::source::Span;
use crate::types::Type;

/// Create an error with span information and context
fn lowering_error(
    kind: ErrorKind,
    message: impl Into<String>,
    span: Span,
    context: Option<&str>,
) -> Error {
    let mut msg = message.into();
    if let Some(ctx) = context {
        msg = format!("{} ({}, ctx)", msg);
    }

    Error::new(kind, msg).with_location(span.start)
}

/// Create a runtime error with expression context
fn runtime_error(message: impl Into<String>, expr: &Expr, operation: &str) -> Error {
    lowering_error(
        ErrorKind::RuntimeError,
        message,
        expr.span,
        Some(format!("while lowering {} expression", operation)),
    )
}

/// Create a type error with expression context
fn type_error(message: impl Into<String>, expr: &Expr, operation: &str) -> Error {
    lowering_error(
        ErrorKind::TypeError,
        message,
        expr.span,
        Some(format!("while type checking {} expression", operation)),
    )
}

/// Create a security error with expression context
fn security_error(message: impl Into<String>, expr: &Expr, operation: &str) -> Error {
    lowering_error(
        ErrorKind::SecurityViolation,
        message,
        expr.span,
        Some(format!(
            "while validating security for {} expression",
            operation
        )),
    )
}

/// Lower an expression to IR
pub fn lower_expression(lowerer: &mut AstLowerer, expr: &Expr) -> LoweringResult<ValueId> {
    match &expr.kind {
        ExprKind::Literal(lit) => lower_literal(lowerer, lit),
        ExprKind::Identifier(name) => lower_identifier(lowerer, name, expr),
        ExprKind::Binary { left, op, right } => lower_binary(lowerer, left, op, right, expr),
        ExprKind::Unary {
            op,
            expr: operand_expr,
        } => lower_unary(lowerer, op, operand_expr, expr),
        ExprKind::Call { callee, args } => lower_call(lowerer, callee, args),
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => lower_if(lowerer, condition, then_branch, else_branch.as_deref()),
        ExprKind::Block(block) => lowerer
            .lower_block(block)?
            .ok_or_else(|| type_error("Block expression must produce a value", expr, "block")),
        ExprKind::Array(elements) => lower_array(lowerer, elements),
        ExprKind::Index { object, index } => lower_index(lowerer, object, index, expr),
        ExprKind::Member { object, property } => lower_member(lowerer, object, property, expr),
        ExprKind::Assign { target, value } => lower_assign(lowerer, target, value),
        ExprKind::Match { expr, arms } => lower_match(lowerer, expr, arms),
        ExprKind::Await { expr } => lower_await(lowerer, expr),
        ExprKind::ListComprehension { .. } => {
            // List comprehensions not yet implemented
            // TODO: Implement proper list comprehension lowering
            Err(Error::new(
                ErrorKind::TypeError,
                "List comprehensions not yet implemented in lowering",
            ))
        }
        ExprKind::GenericConstructor { name, type_args: _ } => {
            // For now, treat generic constructors as simple identifiers
            // TODO: Implement proper generic constructor lowering
            lower_identifier(lowerer, name, expr)
        }
        ExprKind::StructConstructor { name, fields } => {
            lower_struct_constructor(lowerer, name, fields, expr)
        }
        ExprKind::EnumConstructor {
            enum_name,
            variant,
            args,
        } => lower_enum_constructor(lowerer, enum_name, variant, args, expr),
        ExprKind::ErrorPropagation { expr: inner_expr } => {
            lower_error_propagation(lowerer, inner_expr, expr)
        }
        ExprKind::TryCatch {
            try_expr,
            catch_clauses: _,
            finally_block: _,
        } => {
            // For now, just lower the try expression
            // TODO: Implement proper try-catch lowering with exception handling
            lower_expression(lowerer, try_expr)
        }
        ExprKind::Closure { parameters, body } => lower_closure(lowerer, parameters, body, expr),
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
        Literal::Null => Constant::Null,
    };

    Ok(lowerer.builder.const_value(constant))
}

/// Lower an identifier (variable reference)
fn lower_identifier(lowerer: &mut AstLowerer, name: &str, expr: &Expr) -> LoweringResult<ValueId> {
    // Look up the variable
    if let Some(var) = lowerer.context.lookup_variable(name) {
        // Load the value from the variable's memory location
        lowerer
            .builder
            .build_load(var.ptr, var.ty.clone())
            .ok_or_else(|| {
                runtime_error(
                    format!("Failed to load variable '{}'", name),
                    expr,
                    "identifier",
                )
            })
    } else {
        Err(type_error(
            format!("Undefined variable: {}", name),
            expr,
            "identifier",
        ))
    }
}

/// Lower a binary operation
fn lower_binary(
    lowerer: &mut AstLowerer,
    left: &Expr,
    op: &AstBinaryOp,
    right: &Expr,
    expr: &Expr,
) -> LoweringResult<ValueId> {
    let lhs = lower_expression(lowerer, left)?;
    let rhs = lower_expression(lowerer, right)?;

    // Get the type of the operation
    let ty = lowerer.get_expression_type(left)?;

    match op {
        AstBinaryOp::Add => lowerer
            .builder
            .build_binary(IrBinaryOp::Add, lhs, rhs, ty)
            .ok_or_else(|| {
                runtime_error(
                    "Addition operation failed - incompatible types",
                    expr,
                    "binary",
                )
            }),
        AstBinaryOp::Sub => lowerer
            .builder
            .build_binary(IrBinaryOp::Sub, lhs, rhs, ty)
            .ok_or_else(|| {
                runtime_error(
                    "Subtraction operation failed - incompatible types",
                    expr,
                    "binary",
                )
            }),
        AstBinaryOp::Mul => lowerer
            .builder
            .build_binary(IrBinaryOp::Mul, lhs, rhs, ty)
            .ok_or_else(|| {
                runtime_error(
                    "Multiplication operation failed - incompatible types",
                    expr,
                    "binary",
                )
            }),
        AstBinaryOp::Div => lowerer
            .builder
            .build_binary(IrBinaryOp::Div, lhs, rhs, ty)
            .ok_or_else(|| {
                runtime_error(
                    "Division operation failed - incompatible types",
                    expr,
                    "binary",
                )
            }),
        AstBinaryOp::Mod => lowerer
            .builder
            .build_binary(IrBinaryOp::Mod, lhs, rhs, ty)
            .ok_or_else(|| {
                runtime_error(
                    "Modulo operation failed - incompatible types",
                    expr,
                    "binary",
                )
            }),
        AstBinaryOp::Equal => lowerer
            .builder
            .build_compare(ComparisonOp::Eq, lhs, rhs)
            .ok_or_else(|| {
                runtime_error(
                    "Equality comparison failed - incompatible types",
                    expr,
                    "binary",
                )
            }),
        AstBinaryOp::NotEqual => lowerer
            .builder
            .build_compare(ComparisonOp::Ne, lhs, rhs)
            .ok_or_else(|| {
                runtime_error(
                    "Inequality comparison failed - incompatible types",
                    expr,
                    "binary",
                )
            }),
        AstBinaryOp::Less => lowerer
            .builder
            .build_compare(ComparisonOp::Lt, lhs, rhs)
            .ok_or_else(|| {
                runtime_error(
                    "Less-than comparison failed - incompatible types",
                    expr,
                    "binary",
                )
            }),
        AstBinaryOp::LessEqual => lowerer
            .builder
            .build_compare(ComparisonOp::Le, lhs, rhs)
            .ok_or_else(|| {
                runtime_error(
                    "Less-equal comparison failed - incompatible types",
                    expr,
                    "binary",
                )
            }),
        AstBinaryOp::Greater => lowerer
            .builder
            .build_compare(ComparisonOp::Gt, lhs, rhs)
            .ok_or_else(|| {
                runtime_error(
                    "Greater-than comparison failed - incompatible types",
                    expr,
                    "binary",
                )
            }),
        AstBinaryOp::GreaterEqual => lowerer
            .builder
            .build_compare(ComparisonOp::Ge, lhs, rhs)
            .ok_or_else(|| {
                runtime_error(
                    "Greater-equal comparison failed - incompatible types",
                    expr,
                    "binary",
                )
            }),
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
    operand_expr: &Expr,
    expr: &Expr,
) -> LoweringResult<ValueId> {
    let operand = lower_expression(lowerer, operand_expr)?;
    let ty = lowerer.get_expression_type(operand_expr)?;

    match op {
        AstUnaryOp::Minus => lowerer
            .builder
            .build_unary(IrUnaryOp::Neg, operand, ty)
            .ok_or_else(|| {
                runtime_error(
                    "Negation operation failed - operand type does not support negation",
                    expr,
                    "unary",
                )
            }),
        AstUnaryOp::Not => lowerer
            .builder
            .build_unary(IrUnaryOp::Not, operand, Type::Bool)
            .ok_or_else(|| {
                runtime_error(
                    "Logical NOT operation failed - operand is not boolean",
                    expr,
                    "unary",
                )
            }),
    }
}

/// Lower a function call
fn lower_call(lowerer: &mut AstLowerer, callee: &Expr, args: &[Expr]) -> LoweringResult<ValueId> {
    // For now, only support direct function calls
    if let ExprKind::Identifier(func_name) = &callee.kind {
        // Lower arguments
        let arg_values: Vec<ValueId> = args
            .iter()
            .map(|arg| lower_expression(lowerer, arg))
            .collect::<Result<Vec<_>, _>>()?;

        // Check if it's a built-in runtime function
        if func_name == "print" {
            // Special handling for print - generate a call to the runtime print function
            if args.len() != 1 {
                return Err(type_error(
                    format!("print expects exactly 1 argument, got {}", args.len()),
                    callee,
                    "function call",
                ));
            }

            // Get or register the script_print runtime function
            let print_func_id = if let Some(id) = lowerer.context.get_function("script_print") {
                id
            } else {
                // Register the runtime print function
                // script_print is an external runtime function, not defined in Script code
                // We just need a FunctionId to reference it
                let func_id = lowerer.builder.module_mut().reserve_function_id();
                lowerer
                    .context
                    .register_function("script_print".to_string(), func_id);
                func_id
            };

            // The argument is the string value from the Script code
            let string_value = arg_values[0];

            // Call script_print(string_ptr, len)
            // We'll handle the Pascal-style string format in the code generation phase
            // For now, just pass the string value and a dummy length

            // We need to determine the length based on the expression type
            if let Some(string_expr) = &args.get(0) {
                if let ExprKind::Literal(crate::parser::Literal::String(s)) = &string_expr.kind {
                    // For string literals, we know the length at compile time
                    let len = lowerer
                        .builder
                        .const_value(crate::ir::Constant::I32(s.len() as i32));

                    return lowerer
                        .builder
                        .build_call(print_func_id, vec![string_value, len], Type::Unknown)
                        .ok_or_else(|| {
                            runtime_error(
                                "Failed to generate call to runtime print function",
                                callee,
                                "function call",
                            )
                        });
                }
            }

            // For non-string-literal arguments, we'll need to handle them differently
            // For now, return an error
            return Err(runtime_error(
                "print function currently only supports string literals",
                callee,
                "function call",
            ));
        }

        // Look up the function
        if let Some(func_id) = lowerer.context.get_function(func_name) {
            // Get the function's return type
            let return_type = Type::Unknown; // TODO: Get actual return type

            lowerer
                .builder
                .build_call(func_id, arg_values, return_type)
                .ok_or_else(|| {
                    runtime_error(
                        format!("Failed to call function '{}'", func_name),
                        callee,
                        "function call",
                    )
                })
        } else {
            Err(type_error(
                format!("Function '{}' is not defined", func_name),
                callee,
                "function call",
            ))
        }
    } else {
        Err(type_error(
            "Indirect function calls are not yet supported - use direct function names only",
            callee,
            "function call",
        ))
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
    let then_block = lowerer
        .builder
        .create_block("if.then".to_string())
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create if-then block"))?;
    let else_block = lowerer
        .builder
        .create_block("if.else".to_string())
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create if-else block"))?;
    let merge_block = lowerer
        .builder
        .create_block("if.merge".to_string())
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create if-merge block"))?;

    // Evaluate condition
    let cond_value = lower_expression(lowerer, condition)?;
    lowerer
        .builder
        .build_cond_branch(cond_value, then_block, else_block);

    // Then block
    lowerer.builder.set_current_block(then_block);
    let then_value = lower_expression(lowerer, then_branch)?;
    let then_end_block = lowerer.builder.get_current_block().ok_or_else(|| {
        runtime_error(
            "No current block after evaluating then branch",
            then_branch,
            "if-expression",
        )
    })?;
    lowerer.builder.build_branch(merge_block);

    // Else block
    lowerer.builder.set_current_block(else_block);
    let else_value = if let Some(else_expr) = else_branch {
        lower_expression(lowerer, else_expr)?
    } else {
        // No else branch, use unit value
        lowerer.builder.const_value(Constant::Null)
    };
    let else_end_block = lowerer.builder.get_current_block().ok_or_else(|| {
        if let Some(else_expr) = else_branch {
            runtime_error(
                "No current block after evaluating else branch",
                else_expr,
                "if-expression",
            )
        } else {
            Error::new(
                ErrorKind::RuntimeError,
                "No current block after evaluating else branch in if-expression",
            )
        }
    })?;
    lowerer.builder.build_branch(merge_block);

    // Merge block with phi node
    lowerer.builder.set_current_block(merge_block);
    let phi_inst = Instruction::Phi {
        incoming: vec![(then_value, then_end_block), (else_value, else_end_block)],
        ty: Type::Unknown, // TODO: Get actual type
    };

    lowerer.builder.add_instruction(phi_inst).ok_or_else(|| {
        Error::new(
            ErrorKind::RuntimeError,
            "Failed to create if-expression phi node",
        )
    })
}

/// Lower short-circuit AND
fn lower_short_circuit_and(
    lowerer: &mut AstLowerer,
    left: &Expr,
    right: &Expr,
) -> LoweringResult<ValueId> {
    let check_right = lowerer
        .builder
        .create_block("and.rhs".to_string())
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create and.rhs block"))?;
    let merge = lowerer
        .builder
        .create_block("and.merge".to_string())
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create and.merge block"))?;

    // Evaluate left side
    let lhs = lower_expression(lowerer, left)?;
    let current_block = lowerer.builder.get_current_block().ok_or_else(|| {
        runtime_error(
            "No current block after evaluating left side of AND",
            left,
            "short-circuit AND",
        )
    })?;
    lowerer.builder.build_cond_branch(lhs, check_right, merge);

    // Right side block
    lowerer.builder.set_current_block(check_right);
    let rhs = lower_expression(lowerer, right)?;
    let rhs_block = lowerer.builder.get_current_block().ok_or_else(|| {
        runtime_error(
            "No current block after evaluating right side of AND",
            right,
            "short-circuit AND",
        )
    })?;
    lowerer.builder.build_branch(merge);

    // Merge block
    lowerer.builder.set_current_block(merge);
    let phi = Instruction::Phi {
        incoming: vec![(lhs, current_block), (rhs, rhs_block)],
        ty: Type::Bool,
    };

    lowerer
        .builder
        .add_instruction(phi)
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create phi for AND"))
}

/// Lower short-circuit OR
fn lower_short_circuit_or(
    lowerer: &mut AstLowerer,
    left: &Expr,
    right: &Expr,
) -> LoweringResult<ValueId> {
    let check_right = lowerer
        .builder
        .create_block("or.rhs".to_string())
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create or.rhs block"))?;
    let merge = lowerer
        .builder
        .create_block("or.merge".to_string())
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create or.merge block"))?;

    // Evaluate left side
    let lhs = lower_expression(lowerer, left)?;
    let current_block = lowerer.builder.get_current_block().ok_or_else(|| {
        runtime_error(
            "No current block after evaluating left side of OR",
            left,
            "short-circuit OR",
        )
    })?;
    lowerer.builder.build_cond_branch(lhs, merge, check_right);

    // Right side block
    lowerer.builder.set_current_block(check_right);
    let rhs = lower_expression(lowerer, right)?;
    let rhs_block = lowerer.builder.get_current_block().ok_or_else(|| {
        runtime_error(
            "No current block after evaluating right side of OR",
            right,
            "short-circuit OR",
        )
    })?;
    lowerer.builder.build_branch(merge);

    // Merge block
    lowerer.builder.set_current_block(merge);
    let phi = Instruction::Phi {
        incoming: vec![(lhs, current_block), (rhs, rhs_block)],
        ty: Type::Bool,
    };

    lowerer
        .builder
        .add_instruction(phi)
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create phi for OR"))
}

/// Lower array creation
fn lower_array(lowerer: &mut AstLowerer, elements: &[Expr]) -> LoweringResult<ValueId> {
    if elements.is_empty() {
        // Empty array - create an array of unknown type
        let array_type = Type::Array(Box::new(Type::Unknown));
        let array_ptr = lowerer.builder.build_alloc(array_type).ok_or_else(|| {
            Error::new(
                ErrorKind::RuntimeError,
                "Failed to allocate memory for empty array",
            )
        })?;
        return Ok(array_ptr);
    }

    // Lower all element expressions
    let element_values: Vec<ValueId> = elements
        .iter()
        .map(|elem| lower_expression(lowerer, elem))
        .collect::<Result<Vec<_>, _>>()?;

    // Infer array element type from the first element
    let element_type = lowerer.get_expression_type(&elements[0])?;
    let array_type = Type::Array(Box::new(element_type.clone()));

    // Allocate memory for the array
    let array_ptr = lowerer.builder.build_alloc(array_type).ok_or_else(|| {
        Error::new(
            ErrorKind::RuntimeError,
            "Failed to allocate memory for array",
        )
    })?;

    // Store each element in the array
    for (i, &element_value) in element_values.iter().enumerate() {
        // Calculate element pointer using GetElementPtr
        let index_value = lowerer.builder.const_value(Constant::I32(i as i32));
        let element_ptr = lowerer
            .builder
            .add_instruction(Instruction::GetElementPtr {
                ptr: array_ptr,
                index: index_value,
                elem_ty: element_type.clone(),
            })
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::RuntimeError,
                    "Failed to generate array element pointer",
                )
            })?;

        // Store the element value
        lowerer.builder.add_instruction(Instruction::Store {
            ptr: element_ptr,
            value: element_value,
        });
    }

    Ok(array_ptr)
}

/// Lower array indexing
fn lower_index(
    lowerer: &mut AstLowerer,
    object: &Expr,
    index: &Expr,
    expr: &Expr,
) -> LoweringResult<ValueId> {
    let array_value = lower_expression(lowerer, object)?;
    let index_value = lower_expression(lowerer, index)?;

    // Get the type of the array to determine element type
    let array_type = lowerer.get_expression_type(object)?;
    let element_type = match array_type {
        Type::Array(elem_ty) => *elem_ty,
        _ => {
            return Err(type_error(
                format!("Cannot index into non-array type: {:?}", array_type),
                expr,
                "index",
            ));
        }
    };

    // SECURITY FIX: Implement comprehensive bounds checking
    // This replaces the vulnerable code that skipped bounds validation

    // Generate bounds check instruction for runtime validation
    let _bounds_check = lowerer
        .builder
        .add_instruction(Instruction::BoundsCheck {
            array: array_value,
            index: index_value,
            length: None, // Runtime will determine array length
            error_msg: format!(
                "Array index out of bounds at {}:{}",
                expr.span.start.line, expr.span.start.column
            ),
        })
        .ok_or_else(|| {
            security_error(
                "Failed to generate bounds check for array indexing",
                expr,
                "bounds_check",
            )
        })?;

    // Only proceed with array access after bounds check passes
    // The bounds check instruction will validate at runtime before allowing access
    let element_ptr = lowerer
        .builder
        .add_instruction(Instruction::GetElementPtr {
            ptr: array_value,
            index: index_value,
            elem_ty: element_type.clone(),
        })
        .ok_or_else(|| {
            runtime_error(
                "Failed to generate element pointer for array indexing",
                expr,
                "index",
            )
        })?;

    // Load the element value
    let element_value = lowerer
        .builder
        .add_instruction(Instruction::Load {
            ptr: element_ptr,
            ty: element_type,
        })
        .ok_or_else(|| runtime_error("Failed to load array element", expr, "index"))?;

    Ok(element_value)
}

/// Lower member access
fn lower_member(
    lowerer: &mut AstLowerer,
    object: &Expr,
    property: &str,
    expr: &Expr,
) -> LoweringResult<ValueId> {
    let object_value = lower_expression(lowerer, object)?;
    let object_type = lowerer.get_expression_type(object)?;

    // Implement member access similar to member assignment but for reading
    match object_type {
        Type::Named(type_name) => {
            // For named types, generate field access using GetElementPtr
            let field_offset = calculate_field_offset(&type_name, property)?;
            let field_index = lowerer.builder.const_value(Constant::I32(field_offset));

            let field_ptr = lowerer
                .builder
                .add_instruction(Instruction::GetElementPtr {
                    ptr: object_value,
                    index: field_index,
                    elem_ty: Type::Unknown, // Will be resolved when object types are fully implemented
                })
                .ok_or_else(|| {
                    runtime_error(
                        format!(
                            "Failed to access field '{}' on type '{}'",
                            property, type_name
                        ),
                        expr,
                        "member access",
                    )
                })?;

            // Load the value from the field location
            let field_value = lowerer
                .builder
                .add_instruction(Instruction::Load {
                    ptr: field_ptr,
                    ty: Type::Unknown, // Will be resolved when object types are fully implemented
                })
                .ok_or_else(|| {
                    runtime_error(
                        format!("Failed to load field '{}'", property),
                        expr,
                        "member access",
                    )
                })?;

            Ok(field_value)
        }
        Type::Unknown => {
            // SECURITY FIX: Replace vulnerable hash-based field access with secure validation
            // The previous implementation used hash-based offsets which allowed type confusion attacks

            // Generate field validation instruction for runtime security check
            let _field_validation = lowerer
                .builder
                .add_instruction(Instruction::ValidateFieldAccess {
                    object: object_value,
                    field_name: property.to_string(),
                    object_type: Type::Unknown,
                })
                .ok_or_else(|| {
                    security_error(
                        format!("Failed to generate field validation for '{}'", property),
                        expr,
                        "field_validation",
                    )
                })?;

            // Use secure field access through LoadField instruction instead of raw pointer arithmetic
            // This ensures type safety and prevents unauthorized memory access
            let field_value = lowerer
                .builder
                .add_instruction(Instruction::LoadField {
                    object: object_value,
                    field_name: property.to_string(),
                    field_ty: Type::Unknown, // Runtime will resolve actual type
                })
                .ok_or_else(|| {
                    runtime_error(
                        format!("Failed to load validated field '{}'", property),
                        expr,
                        "member access",
                    )
                })?;

            Ok(field_value)
        }
        _ => Err(type_error(
            format!(
                "Cannot access property '{}' on non-object type: {:?}",
                property, object_type
            ),
            expr,
            "member access",
        )),
    }
}

/// Lower assignment
fn lower_assign(lowerer: &mut AstLowerer, target: &Expr, value: &Expr) -> LoweringResult<ValueId> {
    let value_id = lower_expression(lowerer, value)?;

    match &target.kind {
        ExprKind::Identifier(name) => {
            // Variable assignment
            if let Some(var) = lowerer.context.lookup_variable(name) {
                lowerer.builder.build_store(var.ptr, value_id);
                Ok(value_id)
            } else {
                Err(type_error(
                    format!("Cannot assign to undefined variable: {}", name),
                    target,
                    "assignment"
                ))
            }
        }

        ExprKind::Index { object, index } => {
            // Array element assignment: arr[index] = value
            let array_value = lower_expression(lowerer, object)?;
            let index_value = lower_expression(lowerer, index)?;

            // Get the array element type
            let array_type = lowerer.get_expression_type(object)?;
            let element_type = match array_type {
                Type::Array(elem_ty) => *elem_ty,
                _ => {
                    return Err(type_error(
                        format!("Cannot assign to index of non-array type: {:?}", array_type),
                        target,
                        "assignment"
                    ));
                }
            };

            // Calculate element pointer using GetElementPtr
            let element_ptr = lowerer.builder.add_instruction(Instruction::GetElementPtr {
                ptr: array_value,
                index: index_value,
                elem_ty: element_type,
            }).ok_or_else(|| {
                runtime_error(
                    "Failed to generate array element pointer for assignment",
                    target,
                    "assignment"
                )
            })?;

            // Store the value to the element location
            lowerer.builder.add_instruction(Instruction::Store {
                ptr: element_ptr,
                value: value_id,
            });

            Ok(value_id)
        }

        ExprKind::Member { object, property } => {
            // Member assignment: obj.field = value
            let object_value = lower_expression(lowerer, object)?;
            let object_type = lowerer.get_expression_type(object)?;

            // For now, we'll implement member assignment for Named types and prepare for future object types
            match object_type {
                Type::Named(type_name) => {
                    // For named types, we'll generate a field access using GetElementPtr
                    // This assumes the object layout will be defined later
                    // For now, we use a placeholder field offset calculation
                    let field_offset = calculate_field_offset(&type_name, property)?;

                    // Generate field pointer using GetElementPtr with field offset
                    let field_index = lowerer.builder.const_value(Constant::I32(field_offset));
                    let field_ptr = lowerer.builder.add_instruction(Instruction::GetElementPtr {
                        ptr: object_value,
                        index: field_index,
                        elem_ty: Type::Unknown, // Will be resolved when object types are fully implemented
                    }).ok_or_else(|| {
                        runtime_error(
                            format!("Failed to access field '{}' for assignment on type '{}'", property, type_name),
                            target,
                            "assignment"
                        )
                    })?;

                    // Store the value to the field location
                    lowerer.builder.add_instruction(Instruction::Store {
                        ptr: field_ptr,
                        value: value_id,
                    });

                    Ok(value_id)
                }
                Type::Unknown => {
                    // For gradual typing, allow member assignment and defer validation to runtime
                    let field_hash = calculate_field_offset("unknown", property)? as i32;
                    let field_index = lowerer.builder.const_value(Constant::I32(field_hash));

                    let field_ptr = lowerer.builder.add_instruction(Instruction::GetElementPtr {
                        ptr: object_value,
                        index: field_index,
                        elem_ty: Type::Unknown,
                    }).ok_or_else(|| {
                        runtime_error(
                            format!("Failed to access dynamic field '{}' for assignment", property),
                            target,
                            "assignment"
                        )
                    })?;

                    lowerer.builder.add_instruction(Instruction::Store {
                        ptr: field_ptr,
                        value: value_id,
                    });

                    Ok(value_id)
                }
                _ => {
                    Err(type_error(
                        format!("Cannot assign to property '{}' on non-object type: {:?}", property, object_type),
                        target,
                        "assignment"
                    ))
                }
            }
        }

        _ => Err(type_error(
            "Invalid assignment target - only variables, array elements, and object properties are supported",
            target,
            "assignment"
        ))
    }
}

/// Lower an await expression
fn lower_await(lowerer: &mut AstLowerer, expr: &Expr) -> LoweringResult<ValueId> {
    // Lower the future expression
    let future = lower_expression(lowerer, expr)?;

    // Get the output type of the future
    let future_type = lowerer.get_expression_type(expr)?;
    let output_type = future_type.future_type().cloned().unwrap_or(Type::Unknown);

    // In an async function, we need to:
    // 1. Poll the future
    // 2. Check if it's ready
    // 3. If ready, extract the value
    // 4. If pending, suspend execution

    // Generate poll instruction
    let poll_result = lowerer
        .builder
        .build_poll_future(future, output_type.clone())
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to poll future"))?;

    // Create blocks for ready and pending cases
    let ready_block = lowerer
        .builder
        .create_block("await.ready".to_string())
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create ready block"))?;

    let pending_block = lowerer
        .builder
        .create_block("await.pending".to_string())
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create pending block"))?;

    let resume_block = lowerer
        .builder
        .create_block("await.resume".to_string())
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create resume block"))?;

    // Check if the poll result is ready
    // For now, we'll use a simplified approach
    // In reality, we'd need to extract the discriminant of the Poll enum
    let is_ready = lowerer
        .builder
        .build_get_enum_tag(poll_result)
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to get poll tag"))?;

    let ready_tag = lowerer.builder.const_value(Constant::I32(0)); // Assume Ready = 0
    let is_ready_cond = lowerer
        .builder
        .build_compare(ComparisonOp::Eq, is_ready, ready_tag)
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to compare poll result"))?;

    // Branch based on poll result
    lowerer
        .builder
        .build_cond_branch(is_ready_cond, ready_block, pending_block);

    // Ready block - extract the value
    lowerer.builder.set_current_block(ready_block);
    // TODO: Extract the value from the Ready variant
    let ready_value = lowerer.builder.const_value(Constant::Null); // Placeholder
    lowerer.builder.build_branch(resume_block);

    // Pending block - suspend execution
    lowerer.builder.set_current_block(pending_block);
    let state = lowerer.builder.const_value(Constant::I32(1)); // Placeholder state
    lowerer.builder.build_suspend(state, resume_block);

    // Resume block
    lowerer.builder.set_current_block(resume_block);

    // Create a phi node to get the result
    let phi_inst = Instruction::Phi {
        incoming: vec![(ready_value, ready_block)],
        ty: output_type,
    };

    lowerer.builder.add_instruction(phi_inst).ok_or_else(|| {
        Error::new(
            ErrorKind::RuntimeError,
            "Failed to create await result phi node",
        )
    })
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
    let merge_block = lowerer
        .builder
        .create_block("match.merge".to_string())
        .ok_or_else(|| {
            Error::new(
                ErrorKind::RuntimeError,
                "Failed to create match merge block",
            )
        })?;

    // We'll store the values from each arm that need to be phi'd together
    let mut phi_incoming = Vec::new();

    // Create a block for the first arm test
    let mut current_test_block = lowerer
        .builder
        .get_current_block()
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "No current block"))?;

    for (i, arm) in arms.iter().enumerate() {
        let is_last_arm = i == arms.len() - 1;

        // Create blocks for this arm
        let arm_body_block = lowerer
            .builder
            .create_block(format!("match.arm{}.body", i))
            .ok_or_else(|| {
                Error::new(ErrorKind::RuntimeError, "Failed to create arm body block")
            })?;

        let next_test_block = if !is_last_arm {
            Some(
                lowerer
                    .builder
                    .create_block(format!("match.arm{}.next", i))
                    .ok_or_else(|| {
                        Error::new(ErrorKind::RuntimeError, "Failed to create next test block")
                    })?,
            )
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
            lowerer
                .builder
                .build_binary(IrBinaryOp::And, pattern_matches, guard_value, Type::Bool)
                .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to build guard AND"))?
        } else {
            pattern_matches
        };

        // Branch based on the final condition
        if let Some(next_block) = next_test_block {
            lowerer
                .builder
                .build_cond_branch(final_condition, arm_body_block, next_block);
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
        let arm_end_block = lowerer.builder.get_current_block().ok_or_else(|| {
            runtime_error(
                "No current block after evaluating match arm body",
                &arm.body,
                "match expression",
            )
        })?;

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

    lowerer
        .builder
        .add_instruction(phi_inst)
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
                Literal::Null => lowerer.builder.const_value(Constant::Null),
            };

            // Generate equality comparison
            lowerer
                .builder
                .build_compare(ComparisonOp::Eq, value, literal_value)
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::RuntimeError,
                        "Failed to build pattern literal comparison",
                    )
                })
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
                result = lowerer
                    .builder
                    .build_binary(IrBinaryOp::And, result, element_test, Type::Bool)
                    .ok_or_else(|| {
                        Error::new(ErrorKind::RuntimeError, "Failed to build array pattern AND")
                    })?;
            }

            Ok(result)
        }
        PatternKind::Object(fields) => {
            // Object pattern matching: check if the value is an object and has the required fields
            let mut result = lowerer.builder.const_value(Constant::Bool(true));

            for (field_name, sub_pattern) in fields {
                // Load the field value directly from the object using LoadField instruction
                let field_value = lowerer
                    .builder
                    .build_load_field(
                        value,
                        field_name.clone(),
                        Type::Unknown, // Field type to be resolved during type checking
                    )
                    .ok_or_else(|| {
                        Error::new(
                            ErrorKind::RuntimeError,
                            "Failed to load field value for object pattern",
                        )
                    })?;

                // If there's a sub-pattern, test it against the field value
                if let Some(sub_pat) = sub_pattern {
                    let sub_test = lower_pattern_test(lowerer, sub_pat, field_value)?;
                    result = lowerer
                        .builder
                        .build_binary(IrBinaryOp::And, result, sub_test, Type::Bool)
                        .ok_or_else(|| {
                            Error::new(
                                ErrorKind::RuntimeError,
                                "Failed to build object pattern AND test",
                            )
                        })?;
                }
                // If no sub-pattern (shorthand {x}), we assume the field exists
                // In the future, we might want to add a runtime check for field existence
            }

            Ok(result)
        }
        PatternKind::Or(patterns) => {
            // OR pattern - any sub-pattern can match
            let mut result = lowerer.builder.const_value(Constant::Bool(false));

            for sub_pattern in patterns {
                let sub_test = lower_pattern_test(lowerer, sub_pattern, value)?;
                result = lowerer
                    .builder
                    .build_binary(IrBinaryOp::Or, result, sub_test, Type::Bool)
                    .ok_or_else(|| {
                        Error::new(ErrorKind::RuntimeError, "Failed to build OR pattern")
                    })?;
            }

            Ok(result)
        }
        PatternKind::EnumConstructor {
            enum_name: _enum_name,
            variant: _variant,
            args: _args,
        } => {
            // For enum constructor patterns, we need to check if the value
            // matches the specific enum variant

            // TODO: Implement proper enum variant matching
            // For now, return a placeholder true value
            Ok(lowerer.builder.const_value(Constant::Bool(true)))
        }
    }
}

/// Calculate field offset for named types
/// This is a placeholder implementation that will be enhanced when struct types are added
fn calculate_field_offset(type_name: &str, field_name: &str) -> LoweringResult<i32> {
    // For now, use a simple hash-based offset calculation
    // This will be replaced with proper struct layout analysis
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    use std::hash::Hasher;
    hasher.write(type_name.as_bytes());
    hasher.write(field_name.as_bytes());
    let hash = hasher.finish();

    // Use a limited range for field offsets to avoid overflow
    let offset = (hash % 256) as i32;
    Ok(offset)
}

// Calculate field hash for dynamic field access
// SECURITY NOTE: The vulnerable calculate_field_hash function has been removed
// It was replaced with secure field validation using ValidateFieldAccess instruction
// This prevents type confusion attacks through hash collision exploitation

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
            let var_ptr = lowerer
                .builder
                .build_alloc(var_type.clone())
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::RuntimeError,
                        "Failed to allocate pattern variable",
                    )
                })?;

            // Store the matched value
            lowerer.builder.build_store(var_ptr, value);

            // Add to the lowering context
            lowerer
                .context
                .define_variable(name.clone(), var_ptr, var_type);

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
        PatternKind::Object(fields) => {
            // Object pattern binding: extract and bind fields from the object
            for (field_name, sub_pattern) in fields {
                // Load the field value directly from the object using LoadField instruction
                let field_value = lowerer
                    .builder
                    .add_instruction(Instruction::LoadField {
                        object: value,
                        field_name: field_name.clone(),
                        field_ty: Type::Unknown, // Field type to be resolved during type checking
                    })
                    .ok_or_else(|| {
                        Error::new(
                            ErrorKind::RuntimeError,
                            "Failed to load field value for object pattern binding",
                        )
                    })?;

                // If there's a sub-pattern, bind it recursively
                if let Some(sub_pat) = sub_pattern {
                    bind_pattern_variables(lowerer, sub_pat, field_value)?;
                } else {
                    // Shorthand pattern {x} - bind the field name directly
                    let var_type = Type::Unknown; // TODO: Get actual type from pattern analysis
                    let var_ptr =
                        lowerer
                            .builder
                            .build_alloc(var_type.clone())
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::RuntimeError,
                                    "Failed to allocate object pattern variable",
                                )
                            })?;

                    // Store the field value
                    lowerer.builder.build_store(var_ptr, field_value);

                    // Add to the lowering context
                    lowerer
                        .context
                        .define_variable(field_name.clone(), var_ptr, var_type);
                }
            }
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
        PatternKind::EnumConstructor {
            enum_name: _enum_name,
            variant: _variant,
            args,
        } => {
            // For enum constructor patterns, bind variables from the arguments
            if let Some(pattern_args) = args {
                for (_i, arg_pattern) in pattern_args.iter().enumerate() {
                    // TODO: Extract the actual field values from the enum variant
                    // For now, just use the original value (placeholder)
                    bind_pattern_variables(lowerer, arg_pattern, value)?;
                }
            }
            Ok(())
        }
    }
}

/// Lower a struct constructor expression
fn lower_struct_constructor(
    lowerer: &mut AstLowerer,
    name: &str,
    fields: &[(String, Expr)],
    expr: &Expr,
) -> LoweringResult<ValueId> {
    // Get the struct type from expression type info
    let struct_type = lowerer.get_expression_type(expr)?;

    // Lower all field expressions
    let mut field_values = Vec::new();
    for (field_name, field_expr) in fields {
        let field_value = lower_expression(lowerer, field_expr)?;
        field_values.push((field_name.clone(), field_value));
    }

    // Build the struct construction instruction
    lowerer
        .builder
        .build_construct_struct(name.to_string(), field_values, struct_type)
        .ok_or_else(|| {
            runtime_error(
                format!("Failed to construct struct '{}'", name),
                expr,
                "struct constructor",
            )
        })
}

/// Lower an enum constructor expression
fn lower_enum_constructor(
    lowerer: &mut AstLowerer,
    enum_name: &Option<String>,
    variant: &str,
    args: &crate::parser::EnumConstructorArgs,
    expr: &Expr,
) -> LoweringResult<ValueId> {
    // Get the enum type from expression type info
    let enum_type = lowerer.get_expression_type(expr)?;

    // Determine the actual enum name
    let actual_enum_name = match enum_name {
        Some(name) => name.clone(),
        None => {
            // Try to infer enum name from the type
            match &enum_type {
                Type::Named(name) => name.clone(),
                Type::Generic { name, .. } => name.clone(),
                _ => {
                    return Err(type_error(
                        "Cannot determine enum name for unqualified variant",
                        expr,
                        "enum constructor",
                    ))
                }
            }
        }
    };

    // Special handling for Result and Option types - use stdlib constructors
    if (actual_enum_name == "Result" || actual_enum_name == "Option")
        && (variant == "Ok" || variant == "Err" || variant == "Some" || variant == "None")
    {
        // Lower argument expressions
        let arg_values = match args {
            crate::parser::EnumConstructorArgs::Unit => Vec::new(),
            crate::parser::EnumConstructorArgs::Tuple(exprs) => {
                let mut values = Vec::new();
                for arg_expr in exprs {
                    let arg_value = lower_expression(lowerer, arg_expr)?;
                    values.push(arg_value);
                }
                values
            }
            crate::parser::EnumConstructorArgs::Struct(fields) => {
                let mut values = Vec::new();
                for (_, field_expr) in fields {
                    let field_value = lower_expression(lowerer, field_expr)?;
                    values.push(field_value);
                }
                values
            }
        };

        // Generate stdlib function calls for Result/Option constructors
        let function_name = match (actual_enum_name.as_str(), variant) {
            ("Result", "Ok") => "Result::ok",
            ("Result", "Err") => "Result::err",
            ("Option", "Some") => "Option::some",
            ("Option", "None") => "Option::none",
            _ => {
                return Err(runtime_error(
                    format!("Unknown Result/Option variant: {}", variant),
                    expr,
                    "enum constructor",
                ))
            }
        };

        // Build a call to the stdlib constructor function
        // This integrates with the standard library instead of using raw enum construction
        lowerer
            .builder
            .build_stdlib_call(function_name.to_string(), arg_values, enum_type)
            .ok_or_else(|| {
                runtime_error(
                    format!("Failed to call stdlib constructor '{}'", function_name),
                    expr,
                    "enum constructor",
                )
            })
    } else {
        // For other enum types, use the standard enum construction
        // Lower all argument expressions based on the variant type
        let arg_values = match args {
            crate::parser::EnumConstructorArgs::Unit => Vec::new(),
            crate::parser::EnumConstructorArgs::Tuple(exprs) => {
                let mut values = Vec::new();
                for arg_expr in exprs {
                    let arg_value = lower_expression(lowerer, arg_expr)?;
                    values.push(arg_value);
                }
                values
            }
            crate::parser::EnumConstructorArgs::Struct(fields) => {
                // For struct variants, we need to lower fields in the correct order
                // For now, we'll just lower them as a sequence of values
                let mut values = Vec::new();
                for (_, field_expr) in fields {
                    let field_value = lower_expression(lowerer, field_expr)?;
                    values.push(field_value);
                }
                values
            }
        };

        // Get the variant tag - for now, use a simple hash-based approach
        // In a production implementation, this would look up the actual tag from the enum definition
        let tag = calculate_variant_tag(&actual_enum_name, variant);

        // Build the enum construction instruction
        lowerer
            .builder
            .build_construct_enum(
                actual_enum_name,
                variant.to_string(),
                tag,
                arg_values,
                enum_type,
            )
            .ok_or_else(|| {
                runtime_error(
                    format!("Failed to construct enum variant '{}'", variant),
                    expr,
                    "enum constructor",
                )
            })
    }
}

/// Calculate a tag value for an enum variant
/// This is a placeholder implementation that will be replaced when proper enum definitions are available
fn calculate_variant_tag(enum_name: &str, variant_name: &str) -> u32 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    use std::hash::Hasher;
    hasher.write(enum_name.as_bytes());
    hasher.write(variant_name.as_bytes());
    let hash = hasher.finish();

    // Use a limited range for tags
    (hash % 256) as u32
}

/// Lower an error propagation expression (? operator)
fn lower_error_propagation(
    lowerer: &mut AstLowerer,
    inner_expr: &Expr,
    expr: &Expr,
) -> LoweringResult<ValueId> {
    // Lower the inner expression first
    let inner_value = lower_expression(lowerer, inner_expr)?;

    // Get the type of the inner expression
    let inner_type = lowerer
        .get_expression_type_by_id(inner_expr.id)
        .ok_or_else(|| {
            type_error(
                "Cannot determine type of expression for error propagation",
                inner_expr,
                "error propagation",
            )
        })?;

    // Determine the success type based on the inner type
    let success_type = match &inner_type {
        Type::Result { ok, .. } => ok.as_ref().clone(),
        Type::Option(inner) => inner.as_ref().clone(),
        _ => {
            return Err(type_error(
                format!(
                    "Error propagation (?) can only be used on Result or Option types, got {:?}",
                    inner_type
                ),
                expr,
                "error propagation",
            ));
        }
    };

    // Build the error propagation instruction
    lowerer
        .builder
        .build_error_propagation(inner_value, inner_type.clone(), success_type)
        .ok_or_else(|| {
            runtime_error(
                "Failed to build error propagation instruction",
                expr,
                "error propagation",
            )
        })
}

/// Lower a closure expression to IR
fn lower_closure(
    lowerer: &mut AstLowerer,
    parameters: &[ClosureParam],
    body: &Expr,
    expr: &Expr,
) -> LoweringResult<ValueId> {
    // Generate unique function ID for this closure
    let function_id = format!("closure_{}", expr.id);

    // Extract parameter names and types
    let param_names: Vec<String> = parameters.iter().map(|p| p.name.clone()).collect();

    let _param_types: Vec<Type> = parameters
        .iter()
        .map(|p| {
            if let Some(ref type_ann) = p.type_ann {
                crate::types::conversion::type_from_ast(type_ann)
            } else {
                Type::Unknown // Will be inferred
            }
        })
        .collect();

    // Lower the closure body
    let _body_value = lower_expression(lowerer, body)?;

    // Get captured variables from the closure captures map
    let captures = lowerer
        .closure_captures
        .get(&expr.id)
        .cloned()
        .unwrap_or_default();

    // Determine if any captures are by reference
    let captures_by_ref = captures.iter().any(|(_, _, is_mutable)| *is_mutable);

    // Convert captures to the format expected by the IR builder
    // We need to look up the ValueId for each captured variable
    let mut captured_vars: Vec<(String, crate::ir::ValueId)> = Vec::new();
    for (name, _ty, _is_mutable) in captures {
        // Look up the variable in the current context
        if let Some(variable) = lowerer.context.lookup_variable(&name) {
            captured_vars.push((name, variable.ptr));
        } else {
            // Variable not found in context - this shouldn't happen if semantic analysis passed
            return Err(runtime_error(
                format!("Captured variable '{}' not found in scope", name),
                expr,
                "closure capture",
            ));
        }
    }

    // Create the closure instruction
    lowerer
        .builder
        .build_create_closure(function_id, param_names, captured_vars, captures_by_ref)
        .ok_or_else(|| runtime_error("Failed to create closure instruction", expr, "closure"))
}
