use super::Type;
use crate::parser::{BinaryOp, Literal, TypeAnn, TypeKind, UnaryOp};

/// Convert AST type annotations to internal type representation
pub fn type_from_ast(type_ann: &TypeAnn) -> Type {
    match &type_ann.kind {
        TypeKind::Named(name) => match name.as_str() {
            "i32" => Type::I32,
            "f32" => Type::F32,
            "bool" => Type::Bool,
            "string" => Type::String,
            "unknown" => Type::Unknown,
            _ => Type::Named(name.clone()),
        },
        TypeKind::Array(elem_type) => Type::Array(Box::new(type_from_ast(elem_type))),
        TypeKind::Function { params, ret } => Type::Function {
            params: params.iter().map(type_from_ast).collect(),
            ret: Box::new(type_from_ast(ret)),
        },
        TypeKind::Generic { name, args } => {
            // Convert generic type with arguments
            let converted_args: Vec<Type> = args.iter().map(type_from_ast).collect();
            Type::Generic {
                name: name.clone(),
                args: converted_args,
            }
        }
        TypeKind::TypeParam(name) => {
            // Convert type parameter
            Type::TypeParam(name.clone())
        }
        TypeKind::Tuple(types) => {
            // Convert tuple type
            Type::Tuple(types.iter().map(type_from_ast).collect())
        }
        TypeKind::Reference { mutable, inner } => {
            // Convert reference type
            Type::Reference {
                mutable: *mutable,
                inner: Box::new(type_from_ast(inner)),
            }
        }
    }
}

/// Alias for backward compatibility with existing code
pub fn type_ann_to_type(type_ann: &TypeAnn) -> Type {
    type_from_ast(type_ann)
}

/// Infer the type of a literal from the AST
pub fn infer_literal_type(literal: &Literal) -> Type {
    match literal {
        Literal::Number(_) => Type::F32, // Default to f32 for now
        Literal::String(_) => Type::String,
        Literal::Boolean(_) => Type::Bool,
        Literal::Null => Type::Option(Box::new(Type::Unknown)), // Null represents an optional type
    }
}

/// Get the result type of a binary operation
pub fn binary_op_result_type(left: &Type, right: &Type, op: &BinaryOp) -> Result<Type, String> {
    match op {
        // Arithmetic operations
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
            match (left, right) {
                (Type::I32, Type::I32) => Ok(Type::I32),
                (Type::F32, Type::F32) => Ok(Type::F32),
                (Type::Unknown, _) | (_, Type::Unknown) => Ok(Type::Unknown),
                _ => Err(format!(
                    "Cannot apply arithmetic operation to types {} and {}",
                    left, right
                )),
            }
        }

        // Comparison operations
        BinaryOp::Equal
        | BinaryOp::NotEqual
        | BinaryOp::Less
        | BinaryOp::Greater
        | BinaryOp::LessEqual
        | BinaryOp::GreaterEqual => {
            if left.is_comparable() && right.is_comparable() && left.equals(right) {
                Ok(Type::Bool)
            } else if matches!(left, Type::Unknown) || matches!(right, Type::Unknown) {
                Ok(Type::Bool)
            } else {
                Err(format!("Cannot compare types {} and {}", left, right))
            }
        }

        // Logical operations
        BinaryOp::And | BinaryOp::Or => match (left, right) {
            (Type::Bool, Type::Bool) => Ok(Type::Bool),
            (Type::Unknown, _) | (_, Type::Unknown) => Ok(Type::Bool),
            _ => Err(format!(
                "Logical operations require boolean operands, got {} and {}",
                left, right
            )),
        },
    }
}

/// Get the result type of a unary operation
pub fn unary_op_result_type(operand: &Type, op: &UnaryOp) -> Result<Type, String> {
    match op {
        UnaryOp::Not => match operand {
            Type::Bool => Ok(Type::Bool),
            Type::Unknown => Ok(Type::Bool),
            _ => Err(format!("Cannot apply logical NOT to type {}", operand)),
        },
        UnaryOp::Minus => match operand {
            Type::I32 => Ok(Type::I32),
            Type::F32 => Ok(Type::F32),
            Type::Unknown => Ok(Type::Unknown),
            _ => Err(format!("Cannot negate type {}", operand)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{SourceLocation, Span};

    fn dummy_span() -> Span {
        Span {
            start: SourceLocation::initial(),
            end: SourceLocation::initial(),
        }
    }

    #[test]
    fn test_type_from_ast_basic() {
        let i32_ann = TypeAnn {
            kind: TypeKind::Named("i32".to_string()),
            span: dummy_span(),
        };
        assert_eq!(type_from_ast(&i32_ann), Type::I32);

        let bool_ann = TypeAnn {
            kind: TypeKind::Named("bool".to_string()),
            span: dummy_span(),
        };
        assert_eq!(type_from_ast(&bool_ann), Type::Bool);

        let custom_ann = TypeAnn {
            kind: TypeKind::Named("MyType".to_string()),
            span: dummy_span(),
        };
        assert_eq!(
            type_from_ast(&custom_ann),
            Type::Named("MyType".to_string())
        );
    }

    #[test]
    fn test_type_from_ast_array() {
        let array_ann = TypeAnn {
            kind: TypeKind::Array(Box::new(TypeAnn {
                kind: TypeKind::Named("i32".to_string()),
                span: dummy_span(),
            })),
            span: dummy_span(),
        };
        assert_eq!(type_from_ast(&array_ann), Type::Array(Box::new(Type::I32)));
    }

    #[test]
    fn test_type_from_ast_function() {
        let fn_ann = TypeAnn {
            kind: TypeKind::Function {
                params: vec![
                    TypeAnn {
                        kind: TypeKind::Named("i32".to_string()),
                        span: dummy_span(),
                    },
                    TypeAnn {
                        kind: TypeKind::Named("bool".to_string()),
                        span: dummy_span(),
                    },
                ],
                ret: Box::new(TypeAnn {
                    kind: TypeKind::Named("string".to_string()),
                    span: dummy_span(),
                }),
            },
            span: dummy_span(),
        };

        let expected = Type::Function {
            params: vec![Type::I32, Type::Bool],
            ret: Box::new(Type::String),
        };
        assert_eq!(type_from_ast(&fn_ann), expected);
    }

    #[test]
    fn test_infer_literal_type() {
        assert_eq!(infer_literal_type(&Literal::Number(42.0)), Type::F32);
        assert_eq!(
            infer_literal_type(&Literal::String("hello".to_string())),
            Type::String
        );
        assert_eq!(infer_literal_type(&Literal::Boolean(true)), Type::Bool);
    }

    #[test]
    fn test_binary_op_arithmetic() {
        // Valid operations
        assert_eq!(
            binary_op_result_type(&Type::I32, &Type::I32, &BinaryOp::Add),
            Ok(Type::I32)
        );
        assert_eq!(
            binary_op_result_type(&Type::F32, &Type::F32, &BinaryOp::Mul),
            Ok(Type::F32)
        );

        // Invalid operations
        assert!(binary_op_result_type(&Type::I32, &Type::F32, &BinaryOp::Add).is_err());
        assert!(binary_op_result_type(&Type::Bool, &Type::I32, &BinaryOp::Add).is_err());

        // Unknown type
        assert_eq!(
            binary_op_result_type(&Type::Unknown, &Type::I32, &BinaryOp::Add),
            Ok(Type::Unknown)
        );
    }

    #[test]
    fn test_binary_op_comparison() {
        assert_eq!(
            binary_op_result_type(&Type::I32, &Type::I32, &BinaryOp::Less),
            Ok(Type::Bool)
        );
        assert_eq!(
            binary_op_result_type(&Type::String, &Type::String, &BinaryOp::Equal),
            Ok(Type::Bool)
        );

        // Different types cannot be compared
        assert!(binary_op_result_type(&Type::I32, &Type::String, &BinaryOp::Equal).is_err());
    }

    #[test]
    fn test_binary_op_logical() {
        assert_eq!(
            binary_op_result_type(&Type::Bool, &Type::Bool, &BinaryOp::And),
            Ok(Type::Bool)
        );

        // Non-boolean types cannot use logical operators
        assert!(binary_op_result_type(&Type::I32, &Type::I32, &BinaryOp::Or).is_err());
    }

    #[test]
    fn test_unary_op_result() {
        assert_eq!(
            unary_op_result_type(&Type::Bool, &UnaryOp::Not),
            Ok(Type::Bool)
        );
        assert_eq!(
            unary_op_result_type(&Type::I32, &UnaryOp::Minus),
            Ok(Type::I32)
        );
        assert_eq!(
            unary_op_result_type(&Type::F32, &UnaryOp::Minus),
            Ok(Type::F32)
        );

        // Invalid operations
        assert!(unary_op_result_type(&Type::String, &UnaryOp::Not).is_err());
        assert!(unary_op_result_type(&Type::Bool, &UnaryOp::Minus).is_err());
    }
}
