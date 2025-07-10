use crate::error::{Error, ErrorKind, Result};
use crate::parser::{BinaryOp, Expr, ExprKind, Literal, Stmt, StmtKind, UnaryOp};
use std::collections::HashMap;

/// Evaluator for @const functions that can be evaluated at compile time
pub struct ConstEvaluator {
    /// Registry of const functions
    const_functions: HashMap<String, Stmt>,
    /// Cache of evaluated const expressions
    cache: HashMap<String, ConstValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    Number(f64),
    String(String),
    Boolean(bool),
    Array(Vec<ConstValue>),
}

impl ConstEvaluator {
    pub fn new() -> Self {
        Self {
            const_functions: HashMap::new(),
            cache: HashMap::new(),
        }
    }

    /// Register a function marked with @const
    pub fn register_const_function(&mut self, stmt: &Stmt) -> Result<()> {
        if let StmtKind::Function { name, .. } = &stmt.kind {
            // Validate that the function can be const-evaluated
            self.validate_const_function(stmt)?;
            self.const_functions.insert(name.clone(), stmt.clone());
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::SemanticError,
                "@const can only be applied to functions",
            ))
        }
    }

    /// Evaluate a const expression at compile time
    pub fn evaluate_expr(&mut self, expr: &Expr) -> Result<ConstValue> {
        match &expr.kind {
            ExprKind::Literal(lit) => self.evaluate_literal(lit),
            ExprKind::Binary { left, op, right } => {
                let left_val = self.evaluate_expr(left)?;
                let right_val = self.evaluate_expr(right)?;
                self.evaluate_binary_op(left_val, op, right_val)
            }
            ExprKind::Unary { op, expr } => {
                let val = self.evaluate_expr(expr)?;
                self.evaluate_unary_op(op, val)
            }
            ExprKind::Array(elements) => {
                let mut values = Vec::new();
                for elem in elements {
                    values.push(self.evaluate_expr(elem)?);
                }
                Ok(ConstValue::Array(values))
            }
            ExprKind::Call { callee, args } => {
                if let ExprKind::Identifier(name) = &callee.kind {
                    if let Some(func) = self.const_functions.get(name).cloned() {
                        self.evaluate_const_function_call(&func, args)
                    } else {
                        Err(Error::new(
                            ErrorKind::SemanticError,
                            &format!("Function '{}' is not marked as @const", name),
                        ))
                    }
                } else {
                    Err(Error::new(
                        ErrorKind::SemanticError,
                        "Cannot evaluate complex function calls at compile time",
                    ))
                }
            }
            _ => Err(Error::new(
                ErrorKind::SemanticError,
                "Expression cannot be evaluated at compile time",
            )),
        }
    }

    fn evaluate_literal(&self, lit: &Literal) -> Result<ConstValue> {
        match lit {
            Literal::Number(n) => Ok(ConstValue::Number(*n)),
            Literal::String(s) => Ok(ConstValue::String(s.clone())),
            Literal::Boolean(b) => Ok(ConstValue::Boolean(*b)),
            Literal::Null => Err(Error::new(
                ErrorKind::SemanticError,
                "Null cannot be used in const expressions",
            )),
        }
    }

    fn evaluate_binary_op(
        &self,
        left: ConstValue,
        op: &BinaryOp,
        right: ConstValue,
    ) -> Result<ConstValue> {
        match (left, right) {
            (ConstValue::Number(l), ConstValue::Number(r)) => match op {
                BinaryOp::Add => Ok(ConstValue::Number(l + r)),
                BinaryOp::Sub => Ok(ConstValue::Number(l - r)),
                BinaryOp::Mul => Ok(ConstValue::Number(l * r)),
                BinaryOp::Div => {
                    if r == 0.0 {
                        Err(Error::new(
                            ErrorKind::SemanticError,
                            "Division by zero in const expression",
                        ))
                    } else {
                        Ok(ConstValue::Number(l / r))
                    }
                }
                BinaryOp::Mod => Ok(ConstValue::Number(l % r)),
                BinaryOp::Equal => Ok(ConstValue::Boolean(l == r)),
                BinaryOp::NotEqual => Ok(ConstValue::Boolean(l != r)),
                BinaryOp::Less => Ok(ConstValue::Boolean(l < r)),
                BinaryOp::Greater => Ok(ConstValue::Boolean(l > r)),
                BinaryOp::LessEqual => Ok(ConstValue::Boolean(l <= r)),
                BinaryOp::GreaterEqual => Ok(ConstValue::Boolean(l >= r)),
                _ => Err(Error::new(
                    ErrorKind::SemanticError,
                    "Invalid operation for numbers in const expression",
                )),
            },
            (ConstValue::String(l), ConstValue::String(r)) => match op {
                BinaryOp::Add => Ok(ConstValue::String(l + &r)),
                BinaryOp::Equal => Ok(ConstValue::Boolean(l == r)),
                BinaryOp::NotEqual => Ok(ConstValue::Boolean(l != r)),
                _ => Err(Error::new(
                    ErrorKind::SemanticError,
                    "Invalid operation for strings in const expression",
                )),
            },
            (ConstValue::Boolean(l), ConstValue::Boolean(r)) => match op {
                BinaryOp::And => Ok(ConstValue::Boolean(l && r)),
                BinaryOp::Or => Ok(ConstValue::Boolean(l || r)),
                BinaryOp::Equal => Ok(ConstValue::Boolean(l == r)),
                BinaryOp::NotEqual => Ok(ConstValue::Boolean(l != r)),
                _ => Err(Error::new(
                    ErrorKind::SemanticError,
                    "Invalid operation for booleans in const expression",
                )),
            },
            _ => Err(Error::new(
                ErrorKind::SemanticError,
                "Type mismatch in const expression",
            )),
        }
    }

    fn evaluate_unary_op(&self, op: &UnaryOp, val: ConstValue) -> Result<ConstValue> {
        match (op, val) {
            (UnaryOp::Not, ConstValue::Boolean(b)) => Ok(ConstValue::Boolean(!b)),
            (UnaryOp::Minus, ConstValue::Number(n)) => Ok(ConstValue::Number(-n)),
            _ => Err(Error::new(
                ErrorKind::SemanticError,
                "Invalid unary operation in const expression",
            )),
        }
    }

    fn evaluate_const_function_call(&mut self, _func: &Stmt, _args: &[Expr]) -> Result<ConstValue> {
        // Simple const function evaluation - in a real implementation,
        // this would need to handle parameters, local variables, etc.
        Err(Error::new(
            ErrorKind::SemanticError,
            "Const function evaluation not yet fully implemented",
        ))
    }

    fn validate_const_function(&self, _stmt: &Stmt) -> Result<()> {
        // Validate that the function only uses const-evaluable expressions
        // For now, we'll accept all functions and validate during evaluation
        Ok(())
    }
}
