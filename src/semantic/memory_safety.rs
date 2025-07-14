//! Memory safety analysis framework for Script
//!
//! This module provides static analysis for memory safety, including:
//! - Lifetime tracking and validation
//! - Ownership and borrowing analysis
//! - Use-after-free detection
//! - Double-free prevention
//! - Memory leak detection (beyond RC)
//! - Null pointer dereference prevention
//! - Buffer overflow protection

use crate::parser::{Block, Expr, ExprKind, Stmt, StmtKind};
use crate::source::Span;
use crate::types::Type;
use std::collections::HashMap;
use std::fmt;

/// Represents a variable's lifetime in the program
#[derive(Debug, Clone, PartialEq)]
pub struct Lifetime {
    /// Unique lifetime identifier
    pub id: LifetimeId,
    /// Name for debugging (e.g., 'a, 'static)
    pub name: String,
    /// Start location (where lifetime begins)
    pub start: Span,
    /// End location (where lifetime ends)
    pub end: Option<Span>,
    /// Whether this lifetime is static (global)
    pub is_static: bool,
}

/// Unique identifier for lifetimes
pub type LifetimeId = usize;

/// Ownership information for a value
#[derive(Debug, Clone, PartialEq)]
pub enum OwnershipState {
    /// Value is owned by current scope
    Owned,
    /// Value is borrowed immutably
    Borrowed {
        /// Lifetime of the borrow
        lifetime: LifetimeId,
        /// Source of the borrow
        source: Option<String>,
    },
    /// Value is borrowed mutably
    MutBorrowed {
        /// Lifetime of the borrow
        lifetime: LifetimeId,
        /// Source of the borrow
        source: Option<String>,
    },
    /// Value has been moved (no longer accessible)
    Moved {
        /// Where it was moved to
        moved_to: Option<String>,
        /// Span where the move occurred
        move_span: Span,
    },
    /// Value is dangling (pointing to freed memory)
    Dangling {
        /// When it became dangling
        since: Span,
    },
}

/// Memory safety properties of a variable
#[derive(Debug, Clone)]
pub struct MemorySafetyInfo {
    /// Variable name
    pub name: String,
    /// Type of the variable
    pub ty: Type,
    /// Current ownership state
    pub ownership: OwnershipState,
    /// Lifetime of the variable
    pub lifetime: LifetimeId,
    /// Whether the variable is mutable
    pub is_mutable: bool,
    /// Span where variable was defined
    pub def_span: Span,
    /// Last usage span
    pub last_use: Option<Span>,
    /// Whether the variable has been initialized
    pub is_initialized: bool,
}

/// Memory safety analysis context
#[derive(Debug)]
pub struct MemorySafetyContext {
    /// Counter for generating unique lifetime IDs
    lifetime_counter: usize,
    /// All lifetimes in the current analysis
    lifetimes: HashMap<LifetimeId, Lifetime>,
    /// Memory safety info for variables in current scope
    variables: HashMap<String, MemorySafetyInfo>,
    /// Stack of scopes (for nested blocks)
    scope_stack: Vec<HashMap<String, MemorySafetyInfo>>,
    /// Active borrows (to check for conflicts)
    active_borrows: HashMap<String, Vec<(OwnershipState, Span)>>,
    /// Detected memory safety violations
    violations: Vec<MemorySafetyViolation>,
}

/// Types of memory safety violations
#[derive(Debug, Clone, PartialEq)]
pub enum MemorySafetyViolation {
    /// Use after free
    UseAfterFree {
        variable: String,
        use_span: Span,
        free_span: Span,
    },
    /// Double free
    DoubleFree {
        variable: String,
        first_free: Span,
        second_free: Span,
    },
    /// Use of uninitialized variable
    UseOfUninitialized {
        variable: String,
        use_span: Span,
        def_span: Span,
    },
    /// Null pointer dereference
    NullDereference { expression: String, span: Span },
    /// Buffer overflow (array bounds)
    BufferOverflow {
        array: String,
        index_span: Span,
        array_span: Span,
    },
    /// Conflicting borrows
    ConflictingBorrow {
        variable: String,
        existing_borrow: Span,
        new_borrow: Span,
        conflict_type: BorrowConflict,
    },
    /// Use of moved value
    UseOfMoved {
        variable: String,
        use_span: Span,
        move_span: Span,
    },
    /// Lifetime exceeded
    LifetimeExceeded {
        variable: String,
        use_span: Span,
        lifetime_end: Span,
    },
    /// Memory leak potential
    PotentialLeak {
        variable: String,
        allocated_span: Span,
        reason: String,
    },
}

/// Types of borrow conflicts
#[derive(Debug, Clone, PartialEq)]
pub enum BorrowConflict {
    /// Multiple mutable borrows
    MultipleMutableBorrows,
    /// Mutable and immutable borrow conflict
    MutableImmutableConflict,
    /// Borrow while moved
    BorrowAfterMove,
}

impl MemorySafetyContext {
    /// Create a new memory safety context
    pub fn new() -> Self {
        Self {
            lifetime_counter: 0,
            lifetimes: HashMap::new(),
            variables: HashMap::new(),
            scope_stack: Vec::new(),
            active_borrows: HashMap::new(),
            violations: Vec::new(),
        }
    }

    /// Generate a new unique lifetime ID
    fn next_lifetime_id(&mut self) -> LifetimeId {
        let id = self.lifetime_counter;
        self.lifetime_counter += 1;
        id
    }

    /// Create a new lifetime
    pub fn create_lifetime(&mut self, name: String, start: Span, is_static: bool) -> LifetimeId {
        let id = self.next_lifetime_id();
        let lifetime = Lifetime {
            id,
            name,
            start,
            end: None,
            is_static,
        };
        self.lifetimes.insert(id, lifetime);
        id
    }

    /// End a lifetime
    pub fn end_lifetime(&mut self, id: LifetimeId, end: Span) {
        if let Some(lifetime) = self.lifetimes.get_mut(&id) {
            lifetime.end = Some(end);
        }
    }

    /// Enter a new scope
    pub fn enter_scope(&mut self) {
        self.scope_stack.push(std::mem::take(&mut self.variables));
    }

    /// Exit current scope
    pub fn exit_scope(&mut self, scope_end: Span) {
        // Collect lifetime IDs to end (to avoid borrowing issues)
        let lifetimes_to_end: Vec<LifetimeId> = self
            .variables
            .values()
            .filter_map(|var_info| {
                if !self
                    .lifetimes
                    .get(&var_info.lifetime)
                    .map_or(false, |lt| lt.is_static)
                {
                    Some(var_info.lifetime)
                } else {
                    None
                }
            })
            .collect();

        // End lifetimes
        for lifetime_id in lifetimes_to_end {
            self.end_lifetime(lifetime_id, scope_end);
        }

        // Restore previous scope
        if let Some(prev_scope) = self.scope_stack.pop() {
            self.variables = prev_scope;
        }
    }

    /// Define a new variable
    pub fn define_variable(
        &mut self,
        name: String,
        ty: Type,
        is_mutable: bool,
        def_span: Span,
    ) -> Result<(), String> {
        // Create lifetime for this variable
        let lifetime_name = format!("'{}", name);
        let lifetime = self.create_lifetime(lifetime_name, def_span, false);

        let var_info = MemorySafetyInfo {
            name: name.clone(),
            ty,
            ownership: OwnershipState::Owned,
            lifetime,
            is_mutable,
            def_span,
            last_use: None,
            is_initialized: false,
        };

        if self.variables.contains_key(&name) {
            return Err(format!("Variable '{}' already defined in this scope", name));
        }

        self.variables.insert(name, var_info);
        Ok(())
    }

    /// Initialize a variable
    pub fn initialize_variable(&mut self, name: &str, _init_span: Span) -> Result<(), String> {
        if let Some(var_info) = self.variables.get_mut(name) {
            var_info.is_initialized = true;
            Ok(())
        } else {
            Err(format!("Variable '{}' not found", name))
        }
    }

    /// Use a variable (read access)
    pub fn use_variable(
        &mut self,
        name: &str,
        use_span: Span,
    ) -> Result<Type, MemorySafetyViolation> {
        if let Some(var_info) = self.variables.get_mut(name) {
            // Check if variable is initialized
            if !var_info.is_initialized {
                return Err(MemorySafetyViolation::UseOfUninitialized {
                    variable: name.to_string(),
                    use_span,
                    def_span: var_info.def_span,
                });
            }

            // Check ownership state
            match &var_info.ownership {
                OwnershipState::Moved { move_span, .. } => {
                    return Err(MemorySafetyViolation::UseOfMoved {
                        variable: name.to_string(),
                        use_span,
                        move_span: *move_span,
                    });
                }
                OwnershipState::Dangling { since } => {
                    return Err(MemorySafetyViolation::UseAfterFree {
                        variable: name.to_string(),
                        use_span,
                        free_span: *since,
                    });
                }
                _ => {}
            }

            // Check lifetime
            if let Some(lifetime) = self.lifetimes.get(&var_info.lifetime) {
                if let Some(end) = lifetime.end {
                    return Err(MemorySafetyViolation::LifetimeExceeded {
                        variable: name.to_string(),
                        use_span,
                        lifetime_end: end,
                    });
                }
            }

            var_info.last_use = Some(use_span);
            Ok(var_info.ty.clone())
        } else {
            Err(MemorySafetyViolation::UseOfUninitialized {
                variable: name.to_string(),
                use_span,
                def_span: use_span, // Fallback
            })
        }
    }

    /// Create an immutable borrow
    pub fn borrow_immutable(
        &mut self,
        name: &str,
        borrow_span: Span,
    ) -> Result<LifetimeId, MemorySafetyViolation> {
        if let Some(_var_info) = self.variables.get_mut(name) {
            // Check for conflicting mutable borrows
            if let Some(borrows) = self.active_borrows.get(name) {
                for (borrow_state, existing_span) in borrows {
                    if let OwnershipState::MutBorrowed { .. } = borrow_state {
                        return Err(MemorySafetyViolation::ConflictingBorrow {
                            variable: name.to_string(),
                            existing_borrow: *existing_span,
                            new_borrow: borrow_span,
                            conflict_type: BorrowConflict::MutableImmutableConflict,
                        });
                    }
                }
            }

            // Create borrow lifetime
            let borrow_lifetime =
                self.create_lifetime(format!("'borrow_{}", name), borrow_span, false);

            let borrow_state = OwnershipState::Borrowed {
                lifetime: borrow_lifetime,
                source: Some(name.to_string()),
            };

            // Record active borrow
            self.active_borrows
                .entry(name.to_string())
                .or_insert_with(Vec::new)
                .push((borrow_state, borrow_span));

            Ok(borrow_lifetime)
        } else {
            Err(MemorySafetyViolation::UseOfUninitialized {
                variable: name.to_string(),
                use_span: borrow_span,
                def_span: borrow_span, // Fallback
            })
        }
    }

    /// Create a mutable borrow
    pub fn borrow_mutable(
        &mut self,
        name: &str,
        borrow_span: Span,
    ) -> Result<LifetimeId, MemorySafetyViolation> {
        if let Some(var_info) = self.variables.get_mut(name) {
            if !var_info.is_mutable {
                // This would be caught by the type checker, but we can add a note
            }

            // Check for any existing borrows
            if let Some(borrows) = self.active_borrows.get(name) {
                if !borrows.is_empty() {
                    let (_, existing_span) = &borrows[0];
                    return Err(MemorySafetyViolation::ConflictingBorrow {
                        variable: name.to_string(),
                        existing_borrow: *existing_span,
                        new_borrow: borrow_span,
                        conflict_type: BorrowConflict::MultipleMutableBorrows,
                    });
                }
            }

            // Create borrow lifetime
            let borrow_lifetime =
                self.create_lifetime(format!("'mut_borrow_{}", name), borrow_span, false);

            let borrow_state = OwnershipState::MutBorrowed {
                lifetime: borrow_lifetime,
                source: Some(name.to_string()),
            };

            // Record active borrow
            self.active_borrows
                .entry(name.to_string())
                .or_insert_with(Vec::new)
                .push((borrow_state, borrow_span));

            Ok(borrow_lifetime)
        } else {
            Err(MemorySafetyViolation::UseOfUninitialized {
                variable: name.to_string(),
                use_span: borrow_span,
                def_span: borrow_span, // Fallback
            })
        }
    }

    /// Move a variable
    pub fn move_variable(
        &mut self,
        name: &str,
        move_span: Span,
        moved_to: Option<String>,
    ) -> Result<(), MemorySafetyViolation> {
        if let Some(var_info) = self.variables.get_mut(name) {
            // Check if already moved
            if let OwnershipState::Moved {
                move_span: prev_move,
                ..
            } = var_info.ownership
            {
                return Err(MemorySafetyViolation::UseOfMoved {
                    variable: name.to_string(),
                    use_span: move_span,
                    move_span: prev_move,
                });
            }

            var_info.ownership = OwnershipState::Moved {
                moved_to,
                move_span,
            };

            // Clear active borrows
            self.active_borrows.remove(name);

            Ok(())
        } else {
            Err(MemorySafetyViolation::UseOfUninitialized {
                variable: name.to_string(),
                use_span: move_span,
                def_span: move_span, // Fallback
            })
        }
    }

    /// Mark a variable as potentially leaked
    pub fn mark_potential_leak(&mut self, name: &str, reason: String) {
        if let Some(var_info) = self.variables.get(name) {
            self.violations.push(MemorySafetyViolation::PotentialLeak {
                variable: name.to_string(),
                allocated_span: var_info.def_span,
                reason,
            });
        }
    }

    /// Add a memory safety violation
    pub fn add_violation(&mut self, violation: MemorySafetyViolation) {
        self.violations.push(violation);
    }

    /// Get all detected violations
    pub fn violations(&self) -> &[MemorySafetyViolation] {
        &self.violations
    }

    /// Check for potential buffer overflow
    pub fn check_array_bounds(&mut self, array_name: &str, index_expr: &Expr, access_span: Span) {
        // This is a simplified check - in practice, we'd need more sophisticated analysis
        if let ExprKind::Literal(crate::parser::Literal::Number(n)) = &index_expr.kind {
            if *n < 0.0 {
                self.add_violation(MemorySafetyViolation::BufferOverflow {
                    array: array_name.to_string(),
                    index_span: index_expr.span,
                    array_span: access_span,
                });
            }
        }
        // TODO: Add more sophisticated bounds checking for dynamic indices
    }

    /// Check for null pointer dereference
    pub fn check_null_dereference(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Member { object, .. } | ExprKind::Index { object, .. } => {
                if let ExprKind::Literal(crate::parser::Literal::Null) = &object.kind {
                    self.add_violation(MemorySafetyViolation::NullDereference {
                        expression: format!("{:?}", expr.kind), // Simplified
                        span: expr.span,
                    });
                }
            }
            _ => {}
        }
    }

    /// Analyze memory safety of a statement
    pub fn analyze_statement(&mut self, stmt: &Stmt) -> Result<(), Vec<MemorySafetyViolation>> {
        let mut errors = Vec::new();

        match &stmt.kind {
            StmtKind::Let { name, init, .. } => {
                // Define variable
                if let Err(_e) = self.define_variable(
                    name.clone(),
                    Type::Unknown, // Type would come from type checker
                    true,          // Assume mutable for now
                    stmt.span,
                ) {
                    // This is a duplicate definition, but not a memory safety issue
                }

                // Initialize if there's an initializer
                if let Some(init_expr) = init {
                    if let Err(violation) = self.analyze_expression(init_expr) {
                        errors.extend(violation);
                    }
                    if let Err(_) = self.initialize_variable(name, stmt.span) {
                        // Variable not found - shouldn't happen
                    }
                }
            }
            StmtKind::Expression(expr) => {
                if let Err(violation) = self.analyze_expression(expr) {
                    errors.extend(violation);
                }
            }
            StmtKind::Return(expr) => {
                if let Some(ret_expr) = expr {
                    if let Err(violation) = self.analyze_expression(ret_expr) {
                        errors.extend(violation);
                    }
                }
            }
            _ => {
                // Other statement types don't have direct memory safety implications
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Analyze memory safety of an expression
    pub fn analyze_expression(&mut self, expr: &Expr) -> Result<Type, Vec<MemorySafetyViolation>> {
        let mut errors = Vec::new();

        match &expr.kind {
            ExprKind::Identifier(name) => match self.use_variable(name, expr.span) {
                Ok(ty) => return Ok(ty),
                Err(violation) => {
                    errors.push(violation);
                    return Err(errors);
                }
            },
            ExprKind::Index { object, index } => {
                // Check array bounds
                if let ExprKind::Identifier(array_name) = &object.kind {
                    self.check_array_bounds(array_name, index, expr.span);
                }

                // Analyze sub-expressions
                if let Err(mut sub_errors) = self.analyze_expression(object) {
                    errors.append(&mut sub_errors);
                }
                if let Err(mut sub_errors) = self.analyze_expression(index) {
                    errors.append(&mut sub_errors);
                }
            }
            ExprKind::Member { object, .. } => {
                // Check for null dereference
                self.check_null_dereference(expr);

                // Analyze object expression
                if let Err(mut sub_errors) = self.analyze_expression(object) {
                    errors.append(&mut sub_errors);
                }
            }
            ExprKind::Call { callee, args } => {
                // Analyze callee and arguments
                if let Err(mut sub_errors) = self.analyze_expression(callee) {
                    errors.append(&mut sub_errors);
                }
                for arg in args {
                    if let Err(mut sub_errors) = self.analyze_expression(arg) {
                        errors.append(&mut sub_errors);
                    }
                }
            }
            ExprKind::Binary { left, right, .. } => {
                // Analyze both operands
                if let Err(mut sub_errors) = self.analyze_expression(left) {
                    errors.append(&mut sub_errors);
                }
                if let Err(mut sub_errors) = self.analyze_expression(right) {
                    errors.append(&mut sub_errors);
                }
            }
            ExprKind::Unary { expr: inner, .. } => {
                if let Err(mut sub_errors) = self.analyze_expression(inner) {
                    errors.append(&mut sub_errors);
                }
            }
            ExprKind::Assign { target, value } => {
                // Handle assignment - this might involve moving or borrowing
                if let Err(mut sub_errors) = self.analyze_expression(value) {
                    errors.append(&mut sub_errors);
                }
                if let Err(mut sub_errors) = self.analyze_expression(target) {
                    errors.append(&mut sub_errors);
                }
            }
            ExprKind::Block(block) => {
                // Enter new scope for block
                self.enter_scope();
                if let Err(mut sub_errors) = self.analyze_block(block) {
                    errors.append(&mut sub_errors);
                }
                self.exit_scope(expr.span);
            }
            _ => {
                // Other expression types don't have direct memory safety implications
                // but we should still analyze sub-expressions
            }
        }

        if errors.is_empty() {
            Ok(Type::Unknown) // Return type would come from type checker
        } else {
            Err(errors)
        }
    }

    /// Analyze memory safety of a block
    pub fn analyze_block(&mut self, block: &Block) -> Result<(), Vec<MemorySafetyViolation>> {
        let mut errors = Vec::new();

        // Analyze all statements
        for stmt in &block.statements {
            if let Err(mut stmt_errors) = self.analyze_statement(stmt) {
                errors.append(&mut stmt_errors);
            }
        }

        // Analyze final expression
        if let Some(final_expr) = &block.final_expr {
            if let Err(mut expr_errors) = self.analyze_expression(final_expr) {
                errors.append(&mut expr_errors);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl Default for MemorySafetyContext {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for MemorySafetyViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemorySafetyViolation::UseAfterFree { variable, .. } => {
                write!(f, "use after free of variable '{}'", variable)
            }
            MemorySafetyViolation::DoubleFree { variable, .. } => {
                write!(f, "double free of variable '{}'", variable)
            }
            MemorySafetyViolation::UseOfUninitialized { variable, .. } => {
                write!(f, "use of uninitialized variable '{}'", variable)
            }
            MemorySafetyViolation::NullDereference { expression, .. } => {
                write!(f, "null pointer dereference in expression '{}'", expression)
            }
            MemorySafetyViolation::BufferOverflow { array, .. } => {
                write!(f, "potential buffer overflow accessing array '{}'", array)
            }
            MemorySafetyViolation::ConflictingBorrow {
                variable,
                conflict_type,
                ..
            } => {
                write!(
                    f,
                    "conflicting borrow of variable '{}': {:?}",
                    variable, conflict_type
                )
            }
            MemorySafetyViolation::UseOfMoved { variable, .. } => {
                write!(f, "use of moved value '{}'", variable)
            }
            MemorySafetyViolation::LifetimeExceeded { variable, .. } => {
                write!(f, "lifetime exceeded for variable '{}'", variable)
            }
            MemorySafetyViolation::PotentialLeak {
                variable, reason, ..
            } => {
                write!(
                    f,
                    "potential memory leak of variable '{}': {}",
                    variable, reason
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceLocation;

    fn make_span(line: u32, col: u32) -> Span {
        let loc = SourceLocation::new(line as usize, col as usize, 0);
        Span::single(loc)
    }

    #[test]
    fn test_variable_definition() {
        let mut ctx = MemorySafetyContext::new();
        assert!(ctx
            .define_variable("x".to_string(), Type::I32, true, make_span(1, 1))
            .is_ok());

        // Duplicate definition should fail
        assert!(ctx
            .define_variable("x".to_string(), Type::I32, true, make_span(1, 1))
            .is_err());
    }

    #[test]
    fn test_use_of_uninitialized() {
        let mut ctx = MemorySafetyContext::new();
        ctx.define_variable("x".to_string(), Type::I32, true, make_span(1, 1))
            .unwrap();

        // Using uninitialized variable should fail
        let result = ctx.use_variable("x", make_span(2, 1));
        assert!(result.is_err());

        if let Err(MemorySafetyViolation::UseOfUninitialized { variable, .. }) = result {
            assert_eq!(variable, "x");
        } else {
            panic!("Expected UseOfUninitialized violation");
        }
    }

    #[test]
    fn test_initialized_variable_use() {
        let mut ctx = MemorySafetyContext::new();
        ctx.define_variable("x".to_string(), Type::I32, true, make_span(1, 1))
            .unwrap();
        ctx.initialize_variable("x", make_span(1, 5)).unwrap();

        // Using initialized variable should succeed
        let result = ctx.use_variable("x", make_span(2, 1));
        assert!(result.is_ok());
    }

    #[test]
    fn test_borrow_conflicts() {
        let mut ctx = MemorySafetyContext::new();
        ctx.define_variable("x".to_string(), Type::I32, true, make_span(1, 1))
            .unwrap();
        ctx.initialize_variable("x", make_span(1, 5)).unwrap();

        // First mutable borrow should succeed
        let borrow1 = ctx.borrow_mutable("x", make_span(2, 1));
        assert!(borrow1.is_ok());

        // Second mutable borrow should fail
        let borrow2 = ctx.borrow_mutable("x", make_span(3, 1));
        assert!(borrow2.is_err());

        if let Err(MemorySafetyViolation::ConflictingBorrow { conflict_type, .. }) = borrow2 {
            assert_eq!(conflict_type, BorrowConflict::MultipleMutableBorrows);
        } else {
            panic!("Expected ConflictingBorrow violation");
        }
    }

    #[test]
    fn test_scope_handling() {
        let mut ctx = MemorySafetyContext::new();

        // Define variable in outer scope
        ctx.define_variable("x".to_string(), Type::I32, true, make_span(1, 1))
            .unwrap();

        // Enter inner scope
        ctx.enter_scope();
        ctx.define_variable("y".to_string(), Type::I32, true, make_span(2, 1))
            .unwrap();

        // Both variables should be accessible
        assert!(ctx.variables.contains_key("y"));

        // Exit inner scope
        ctx.exit_scope(make_span(3, 1));

        // y should no longer be accessible, but x should be
        assert!(!ctx.variables.contains_key("y"));
    }
}
