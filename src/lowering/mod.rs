//! AST to IR lowering module
//!
//! This module is responsible for transforming the typed AST into IR representation.
//! The lowering process preserves type information and translates high-level constructs
//! into simpler IR instructions.

use crate::error::{Error, ErrorKind};
use crate::ir::{Constant, Instruction, IrBuilder, Module as IrModule, Parameter, ValueId};
use crate::parser::{Block, Expr, Program, Stmt, StmtKind};
use crate::semantic::{SymbolTable, analyzer::GenericInstantiation};
use crate::types::Type;
use std::collections::HashMap;
use std::mem;

pub mod context;
pub mod expr;
pub mod stmt;

pub use context::LoweringContext;

/// Result type for lowering operations
pub type LoweringResult<T> = Result<T, Error>;

/// Lower an AST program to IR
pub struct AstLowerer {
    /// IR builder
    builder: IrBuilder,
    /// Lowering context
    context: LoweringContext,
    /// Symbol table from semantic analysis
    symbol_table: SymbolTable,
    /// Type information from semantic analysis
    type_info: HashMap<usize, Type>, // Maps expression IDs to types
    /// Generic instantiations for monomorphization
    generic_instantiations: Vec<GenericInstantiation>,
}

impl AstLowerer {
    /// Create a new AST lowerer
    pub fn new(
        symbol_table: SymbolTable, 
        type_info: HashMap<usize, Type>,
        generic_instantiations: Vec<GenericInstantiation>
    ) -> Self {
        AstLowerer {
            builder: IrBuilder::new(),
            context: LoweringContext::new(),
            symbol_table,
            type_info,
            generic_instantiations,
        }
    }
    
    /// Get the generic instantiations for monomorphization
    pub fn generic_instantiations(&self) -> &[GenericInstantiation] {
        &self.generic_instantiations
    }

    /// Lower a program to IR
    pub fn lower_program(&mut self, program: &Program) -> LoweringResult<IrModule> {
        // First pass: collect all function declarations
        for stmt in &program.statements {
            if let StmtKind::Function {
                name,
                params,
                ret_type,
                is_async,
                ..
            } = &stmt.kind
            {
                let ir_params: Vec<Parameter> = params
                    .iter()
                    .map(|p| Parameter {
                        name: p.name.clone(),
                        ty: self.convert_type_annotation(&p.type_ann),
                    })
                    .collect();

                let base_return_type = ret_type
                    .as_ref()
                    .map(|t| self.convert_type_annotation(t))
                    .unwrap_or(Type::Unknown);

                // For async functions, wrap the return type in Future<T>
                let return_type = if *is_async {
                    Type::Future(Box::new(base_return_type))
                } else {
                    base_return_type
                };

                let func_id = self
                    .builder
                    .create_function(name.clone(), ir_params, return_type);
                self.context.register_function(name.clone(), func_id);
            }
        }

        // Second pass: lower function bodies and global statements
        for stmt in &program.statements {
            match &stmt.kind {
                StmtKind::Function {
                    name, params, body, ..
                } => {
                    self.lower_function(name, params, body)?;
                }
                _ => {
                    // Global statements go into a special main function
                    self.ensure_main_function();
                    self.lower_statement(stmt)?;
                }
            }
        }

        // Finalize main function if it has statements
        self.finalize_main_function();

        Ok(mem::replace(&mut self.builder, IrBuilder::new()).build())
    }

    /// Lower a function body
    fn lower_function(
        &mut self,
        name: &str,
        params: &[crate::parser::Param],
        body: &Block,
    ) -> LoweringResult<()> {
        let func_id = self.context.get_function(name).ok_or_else(|| {
            Error::new(
                ErrorKind::TypeError,
                format!("Function '{}' not found", name),
            )
        })?;

        self.builder.set_current_function(func_id);
        self.context.enter_function(func_id);

        // Register function parameters as variables in the current scope
        for (i, param) in params.iter().enumerate() {
            // Convert the parameter type annotation to our Type system
            let param_type = self.convert_type_annotation(&param.type_ann);

            // Create a parameter value ID - this will map to the actual parameter
            // value that gets passed in at runtime via the function's entry block
            let param_value_id = ValueId(i as u32 + 1000); // Use high numbers for params

            // Allocate memory for the parameter
            let param_ptr = self
                .builder
                .build_alloc(param_type.clone())
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::RuntimeError,
                        format!("Failed to allocate parameter '{}'", param.name),
                    )
                })?;

            // Store the parameter value (this creates the store instruction)
            self.builder.build_store(param_ptr, param_value_id);

            // Register the parameter as a variable in the current scope
            self.context
                .define_variable(param.name.clone(), param_ptr, param_type);
        }

        // Lower the function body
        self.lower_block(body)?;

        // Ensure the function has a return
        self.ensure_return();

        self.context.exit_function();
        Ok(())
    }

    /// Lower a block
    fn lower_block(&mut self, block: &Block) -> LoweringResult<Option<ValueId>> {
        self.context.push_scope();

        // Lower statements
        for stmt in &block.statements {
            self.lower_statement(stmt)?;
        }

        // Lower final expression if present
        let result = if let Some(final_expr) = &block.final_expr {
            Some(self.lower_expression(final_expr)?)
        } else {
            None
        };

        self.context.pop_scope();
        Ok(result)
    }

    /// Lower a statement
    fn lower_statement(&mut self, stmt: &Stmt) -> LoweringResult<()> {
        match &stmt.kind {
            StmtKind::Let { name, init, .. } => {
                if let Some(init_expr) = init {
                    let value = self.lower_expression(init_expr)?;
                    let ty = self.get_expression_type(init_expr)?;

                    // Allocate memory for the variable
                    let ptr = self.builder.build_alloc(ty.clone()).ok_or_else(|| {
                        Error::new(ErrorKind::RuntimeError, "Failed to allocate variable")
                    })?;

                    // Store the initial value
                    self.builder.build_store(ptr, value);

                    // Register the variable
                    self.context.define_variable(name.clone(), ptr, ty);
                }
            }

            StmtKind::Expression(expr) => {
                self.lower_expression(expr)?;
            }

            StmtKind::Return(expr) => {
                let value = expr
                    .as_ref()
                    .map(|e| self.lower_expression(e))
                    .transpose()?;
                self.builder.build_return(value);
            }

            StmtKind::While { condition, body } => {
                self.lower_while(condition, body)?;
            }

            StmtKind::For {
                variable,
                iterable,
                body,
            } => {
                self.lower_for(variable, iterable, body)?;
            }

            StmtKind::Function { .. } => {
                // Functions are handled in the first pass
            }

            StmtKind::Import { .. } => {
                // Imports are handled during semantic analysis
                // For now, we skip them in lowering
            }

            StmtKind::Export { .. } => {
                // Exports are handled during semantic analysis
                // For now, we skip them in lowering
            }

            StmtKind::Struct { .. } => {
                // TODO: Implement struct definition lowering
                // Structs are handled during semantic analysis
            }

            StmtKind::Enum { .. } => {
                // TODO: Implement enum definition lowering
                // Enums are handled during semantic analysis
            }
            
            StmtKind::Impl(_) => {
                // TODO: Implement impl block lowering
                // Impl blocks are handled during semantic analysis
            }
        }

        Ok(())
    }

    /// Lower an expression
    fn lower_expression(&mut self, expr: &Expr) -> LoweringResult<ValueId> {
        expr::lower_expression(self, expr)
    }

    /// Lower a while loop
    fn lower_while(&mut self, condition: &Expr, body: &Block) -> LoweringResult<()> {
        // Create loop blocks
        let cond_block = self
            .builder
            .create_block("while.cond".to_string())
            .ok_or_else(|| {
                Error::new(ErrorKind::RuntimeError, "Failed to create condition block")
            })?;
        let body_block = self
            .builder
            .create_block("while.body".to_string())
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create body block"))?;
        let after_block = self
            .builder
            .create_block("while.after".to_string())
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to create after block"))?;

        // Jump to condition block
        self.builder.build_branch(cond_block);

        // Condition block
        self.builder.set_current_block(cond_block);
        let cond_value = self.lower_expression(condition)?;
        self.builder
            .build_cond_branch(cond_value, body_block, after_block);

        // Body block
        self.builder.set_current_block(body_block);
        self.lower_block(body)?;
        self.builder.build_branch(cond_block);

        // After block
        self.builder.set_current_block(after_block);

        Ok(())
    }

    /// Lower a for loop
    fn lower_for(&mut self, variable: &str, iterable: &Expr, body: &Block) -> LoweringResult<()> {
        // Lower the iterable expression
        let iter_value = self.lower_expression(iterable)?;
        let iter_type = self.get_expression_type(iterable)?;

        // Handle different iterable types
        match iter_type {
            Type::Array(element_type) => {
                self.lower_array_for_loop(variable, iter_value, *element_type, body)
            }
            Type::I32 => {
                // Treat as range iteration (0..n)
                self.lower_range_for_loop(variable, iter_value, body)
            }
            _ => {
                // For other types, we'll implement a generic iterator protocol later
                Err(Error::new(
                    ErrorKind::TypeError,
                    format!("Cannot iterate over type: {:?}", iter_type),
                ))
            }
        }
    }

    /// Lower array for-loop iteration
    fn lower_array_for_loop(
        &mut self,
        variable: &str,
        array_value: ValueId,
        element_type: Type,
        body: &Block,
    ) -> LoweringResult<()> {
        // Create loop blocks
        let init_block = self
            .builder
            .create_block("for.init".to_string())
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::RuntimeError,
                    "Failed to create for-loop init block",
                )
            })?;
        let cond_block = self
            .builder
            .create_block("for.cond".to_string())
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::RuntimeError,
                    "Failed to create for-loop condition block",
                )
            })?;
        let body_block = self
            .builder
            .create_block("for.body".to_string())
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::RuntimeError,
                    "Failed to create for-loop body block",
                )
            })?;
        let inc_block = self
            .builder
            .create_block("for.inc".to_string())
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::RuntimeError,
                    "Failed to create for-loop increment block",
                )
            })?;
        let after_block = self
            .builder
            .create_block("for.after".to_string())
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::RuntimeError,
                    "Failed to create for-loop after block",
                )
            })?;

        // Push loop context for break/continue support
        self.context.push_loop(inc_block, after_block);

        // Jump to initialization
        self.builder.build_branch(init_block);

        // Init block: initialize loop index to 0
        self.builder.set_current_block(init_block);
        let index_init = self.builder.const_value(Constant::I32(0));
        let index_ptr = self
            .builder
            .build_alloc(Type::I32)
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to allocate loop index"))?;
        self.builder.build_store(index_ptr, index_init);
        self.builder.build_branch(cond_block);

        // Condition block: check if index < array_length
        // For now, we'll assume a fixed array length or skip bounds checking
        // In a real implementation, we'd load the array length from array metadata
        self.builder.set_current_block(cond_block);
        let current_index = self
            .builder
            .build_load(index_ptr, Type::I32)
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to load loop index"))?;

        // For simplicity, we'll use a placeholder length check
        // TODO: Implement proper array length checking
        let array_length = self.builder.const_value(Constant::I32(10)); // Placeholder
        let cond_result = self
            .builder
            .build_compare(crate::ir::ComparisonOp::Lt, current_index, array_length)
            .ok_or_else(|| {
                Error::new(ErrorKind::RuntimeError, "Failed to generate loop condition")
            })?;

        self.builder
            .build_cond_branch(cond_result, body_block, after_block);

        // Body block: load current element and execute loop body
        self.builder.set_current_block(body_block);
        self.context.push_scope();

        // Load current array element
        let current_index = self
            .builder
            .build_load(index_ptr, Type::I32)
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::RuntimeError,
                    "Failed to load current index for element access",
                )
            })?;

        let element_ptr = self
            .builder
            .add_instruction(Instruction::GetElementPtr {
                ptr: array_value,
                index: current_index,
                elem_ty: element_type.clone(),
            })
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::RuntimeError,
                    "Failed to generate element pointer in for-loop",
                )
            })?;

        let element_value = self
            .builder
            .add_instruction(Instruction::Load {
                ptr: element_ptr,
                ty: element_type.clone(),
            })
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::RuntimeError,
                    "Failed to load array element in for-loop",
                )
            })?;

        // Create loop variable and bind to current element
        let var_ptr = self
            .builder
            .build_alloc(element_type.clone())
            .ok_or_else(|| {
                Error::new(ErrorKind::RuntimeError, "Failed to allocate loop variable")
            })?;
        self.builder.build_store(var_ptr, element_value);
        self.context
            .define_variable(variable.to_string(), var_ptr, element_type);

        // Lower the loop body
        self.lower_block(body)?;
        self.context.pop_scope();
        self.builder.build_branch(inc_block);

        // Increment block: increment index
        self.builder.set_current_block(inc_block);
        let current_index = self
            .builder
            .build_load(index_ptr, Type::I32)
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::RuntimeError,
                    "Failed to load index for increment",
                )
            })?;
        let one = self.builder.const_value(Constant::I32(1));
        let next_index = self
            .builder
            .build_binary(crate::ir::BinaryOp::Add, current_index, one, Type::I32)
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to increment loop index"))?;
        self.builder.build_store(index_ptr, next_index);
        self.builder.build_branch(cond_block);

        // After block
        self.builder.set_current_block(after_block);
        self.context.pop_loop();

        Ok(())
    }

    /// Lower range for-loop iteration (for i in 0..n)
    fn lower_range_for_loop(
        &mut self,
        variable: &str,
        limit_value: ValueId,
        body: &Block,
    ) -> LoweringResult<()> {
        // Create loop blocks
        let init_block = self
            .builder
            .create_block("range.init".to_string())
            .ok_or_else(|| {
                Error::new(ErrorKind::RuntimeError, "Failed to create range init block")
            })?;
        let cond_block = self
            .builder
            .create_block("range.cond".to_string())
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::RuntimeError,
                    "Failed to create range condition block",
                )
            })?;
        let body_block = self
            .builder
            .create_block("range.body".to_string())
            .ok_or_else(|| {
                Error::new(ErrorKind::RuntimeError, "Failed to create range body block")
            })?;
        let inc_block = self
            .builder
            .create_block("range.inc".to_string())
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::RuntimeError,
                    "Failed to create range increment block",
                )
            })?;
        let after_block = self
            .builder
            .create_block("range.after".to_string())
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::RuntimeError,
                    "Failed to create range after block",
                )
            })?;

        // Push loop context
        self.context.push_loop(inc_block, after_block);

        // Jump to initialization
        self.builder.build_branch(init_block);

        // Init block: initialize counter to 0
        self.builder.set_current_block(init_block);
        let counter_init = self.builder.const_value(Constant::I32(0));
        let counter_ptr = self.builder.build_alloc(Type::I32).ok_or_else(|| {
            Error::new(ErrorKind::RuntimeError, "Failed to allocate range counter")
        })?;
        self.builder.build_store(counter_ptr, counter_init);
        self.builder.build_branch(cond_block);

        // Condition block: check if counter < limit
        self.builder.set_current_block(cond_block);
        let current_counter = self
            .builder
            .build_load(counter_ptr, Type::I32)
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Failed to load range counter"))?;
        let cond_result = self
            .builder
            .build_compare(crate::ir::ComparisonOp::Lt, current_counter, limit_value)
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::RuntimeError,
                    "Failed to generate range condition",
                )
            })?;
        self.builder
            .build_cond_branch(cond_result, body_block, after_block);

        // Body block
        self.builder.set_current_block(body_block);
        self.context.push_scope();

        // Create loop variable bound to current counter value
        let current_counter = self
            .builder
            .build_load(counter_ptr, Type::I32)
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::RuntimeError,
                    "Failed to load counter for loop variable",
                )
            })?;
        let var_ptr = self.builder.build_alloc(Type::I32).ok_or_else(|| {
            Error::new(
                ErrorKind::RuntimeError,
                "Failed to allocate range loop variable",
            )
        })?;
        self.builder.build_store(var_ptr, current_counter);
        self.context
            .define_variable(variable.to_string(), var_ptr, Type::I32);

        // Lower the loop body
        self.lower_block(body)?;
        self.context.pop_scope();
        self.builder.build_branch(inc_block);

        // Increment block
        self.builder.set_current_block(inc_block);
        let current_counter = self
            .builder
            .build_load(counter_ptr, Type::I32)
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::RuntimeError,
                    "Failed to load counter for increment",
                )
            })?;
        let one = self.builder.const_value(Constant::I32(1));
        let next_counter = self
            .builder
            .build_binary(crate::ir::BinaryOp::Add, current_counter, one, Type::I32)
            .ok_or_else(|| {
                Error::new(ErrorKind::RuntimeError, "Failed to increment range counter")
            })?;
        self.builder.build_store(counter_ptr, next_counter);
        self.builder.build_branch(cond_block);

        // After block
        self.builder.set_current_block(after_block);
        self.context.pop_loop();

        Ok(())
    }

    /// Convert a type annotation to a Type
    fn convert_type_annotation(&self, type_ann: &crate::parser::TypeAnn) -> Type {
        use crate::parser::TypeKind;

        match &type_ann.kind {
            TypeKind::Named(name) => match name.as_str() {
                "i32" => Type::I32,
                "f32" => Type::F32,
                "bool" => Type::Bool,
                "string" => Type::String,
                "String" => Type::String,
                _ => Type::Named(name.clone()),
            },
            TypeKind::Array(element_type) => {
                let element = self.convert_type_annotation(element_type);
                Type::Array(Box::new(element))
            }
            TypeKind::Function { params, ret } => {
                let param_types: Vec<Type> = params
                    .iter()
                    .map(|param| self.convert_type_annotation(param))
                    .collect();
                let return_type = self.convert_type_annotation(ret);
                Type::Function {
                    params: param_types,
                    ret: Box::new(return_type),
                }
            }
            TypeKind::Generic { name, args } => {
                let arg_types: Vec<Type> = args
                    .iter()
                    .map(|arg| self.convert_type_annotation(arg))
                    .collect();
                Type::Generic {
                    name: name.clone(),
                    args: arg_types,
                }
            }
            TypeKind::TypeParam(name) => Type::TypeParam(name.clone()),
            TypeKind::Tuple(types) => {
                let element_types: Vec<Type> = types
                    .iter()
                    .map(|t| self.convert_type_annotation(t))
                    .collect();
                Type::Tuple(element_types)
            }
            TypeKind::Reference { mutable, inner } => Type::Reference {
                mutable: *mutable,
                inner: Box::new(self.convert_type_annotation(inner)),
            },
        }
    }

    /// Get the type of an expression
    fn get_expression_type(&self, expr: &Expr) -> LoweringResult<Type> {
        // First, try to get the type from semantic analysis
        if let Some(type_) = self.type_info.get(&expr.id) {
            return Ok(type_.clone());
        }
        
        // Fallback to basic type inference if type information is not available
        use crate::parser::ExprKind;

        match &expr.kind {
            ExprKind::Literal(literal) => {
                use crate::parser::Literal;
                Ok(match literal {
                    Literal::Number(n) => {
                        if n.fract() == 0.0 && n.abs() <= i32::MAX as f64 {
                            Type::I32
                        } else {
                            Type::F32
                        }
                    }
                    Literal::String(_) => Type::String,
                    Literal::Boolean(_) => Type::Bool,
                    Literal::Null => Type::Option(Box::new(Type::Unknown)),
                })
            }
            ExprKind::Identifier(name) => {
                // Look up variable type from context
                if let Some(var) = self.context.lookup_variable(name) {
                    Ok(var.ty.clone())
                } else {
                    // Check if it's a function
                    if self.context.get_function(name).is_some() {
                        // For now, return function type - TODO: get actual signature
                        Ok(Type::Function {
                            params: vec![],
                            ret: Box::new(Type::Unknown),
                        })
                    } else {
                        Ok(Type::Unknown)
                    }
                }
            }
            ExprKind::Binary { left, op, right: _ } => {
                use crate::parser::BinaryOp;
                match op {
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Mod => {
                        // Arithmetic operations preserve the type of operands
                        self.get_expression_type(left)
                    }
                    BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual
                    | BinaryOp::And
                    | BinaryOp::Or => {
                        // Comparison and logical operations return bool
                        Ok(Type::Bool)
                    }
                }
            }
            ExprKind::Unary { op, expr: _ } => {
                use crate::parser::UnaryOp;
                match op {
                    UnaryOp::Minus => {
                        // Negation preserves numeric type
                        self.get_expression_type(expr)
                    }
                    UnaryOp::Not => {
                        // Logical not returns bool
                        Ok(Type::Bool)
                    }
                }
            }
            ExprKind::Call { callee, args: _ } => {
                // For function calls, we'd need to look up the function signature
                // For now, try to get the type from the callee
                if let ExprKind::Identifier(func_name) = &callee.kind {
                    // Handle built-in functions
                    match func_name.as_str() {
                        "print" => Ok(Type::Unknown), // print returns unit/void
                        _ => {
                            // For user-defined functions, we'd need to look up their return type
                            // TODO: Get actual return type from function registry
                            Ok(Type::Unknown)
                        }
                    }
                } else {
                    Ok(Type::Unknown)
                }
            }
            ExprKind::If {
                then_branch,
                else_branch,
                ..
            } => {
                // If expression type is the common type of both branches
                let then_type = self.get_expression_type(then_branch)?;
                if let Some(else_expr) = else_branch {
                    let else_type = self.get_expression_type(else_expr)?;
                    // For simplicity, if both types are the same, use that type
                    if then_type == else_type {
                        Ok(then_type)
                    } else {
                        Ok(Type::Unknown)
                    }
                } else {
                    // If without else returns unit, but in our case we'll treat as the then type
                    Ok(then_type)
                }
            }
            ExprKind::Block(_) => {
                // Block expression type would be determined by its final expression
                // For now, return unknown
                Ok(Type::Unknown)
            }
            ExprKind::Array(elements) => {
                if elements.is_empty() {
                    Ok(Type::Array(Box::new(Type::Unknown)))
                } else {
                    // Infer element type from first element
                    let element_type = self.get_expression_type(&elements[0])?;
                    Ok(Type::Array(Box::new(element_type)))
                }
            }
            ExprKind::Index { object, index: _ } => {
                // Array indexing returns the element type
                let object_type = self.get_expression_type(object)?;
                match object_type {
                    Type::Array(element_type) => Ok(*element_type),
                    _ => Ok(Type::Unknown),
                }
            }
            ExprKind::Member {
                object: _,
                property: _,
            } => {
                // Member access type depends on object type and property
                // For now, return unknown
                Ok(Type::Unknown)
            }
            ExprKind::Assign { value, target: _ } => {
                // Assignment expression returns the type of the assigned value
                self.get_expression_type(value)
            }
            ExprKind::Match { expr: _, arms: _ } => {
                // Match expression type would be the common type of all arms
                // For now, return unknown
                Ok(Type::Unknown)
            }
            ExprKind::Await { expr } => {
                // Await unwraps Future<T> to T
                let expr_type = self.get_expression_type(expr)?;
                match expr_type {
                    Type::Future(inner_type) => Ok(*inner_type),
                    _ => Ok(Type::Unknown),
                }
            }
            ExprKind::ListComprehension { .. } => {
                // List comprehensions return arrays
                Ok(Type::Array(Box::new(Type::Unknown)))
            }
            ExprKind::GenericConstructor { name, type_args: _ } => {
                // Generic constructors are treated as named types
                // NOTE: With monomorphization complete, this may be the correct approach
                Ok(Type::Named(name.clone()))
            }
            ExprKind::StructConstructor { name, fields: _ } => {
                // TODO: Implement struct constructor type inference
                // For now, return the struct type
                Ok(Type::Named(name.clone()))
            }
            ExprKind::EnumConstructor {
                enum_name,
                variant: _,
                args: _,
            } => {
                // TODO: Implement enum constructor type inference
                // For now, return the enum type if known
                if let Some(enum_name) = enum_name {
                    Ok(Type::Named(enum_name.clone()))
                } else {
                    Ok(Type::Unknown)
                }
            }
        }
    }

    /// Ensure main function exists
    fn ensure_main_function(&mut self) {
        if self.context.get_function("main").is_none() {
            let main_id = self
                .builder
                .create_function("main".to_string(), vec![], Type::Unknown);
            self.context.register_function("main".to_string(), main_id);
            self.builder.set_current_function(main_id);
        }
    }

    /// Finalize main function
    fn finalize_main_function(&mut self) {
        if let Some(main_id) = self.context.get_function("main") {
            let current = self.builder.current_function();
            self.builder.set_current_function(main_id);
            self.ensure_return();
            if let Some(prev) = current {
                self.builder.set_current_function(prev);
            }
        }
    }

    /// Ensure the current function has a return
    fn ensure_return(&mut self) {
        // Check if the current block already has a terminator
        // If not, add a return
        if !self.builder.current_block_has_terminator() {
            self.builder.build_return(None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn lower_source(source: &str) -> LoweringResult<IrModule> {
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.scan_tokens();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Failed to parse");

        // Create dummy symbol table and type info for testing
        let symbol_table = SymbolTable::new();
        let type_info = HashMap::new();
        let generic_instantiations = Vec::new();

        let mut lowerer = AstLowerer::new(symbol_table, type_info, generic_instantiations);
        lowerer.lower_program(&program)
    }

    #[test]
    fn test_lower_empty_program() {
        let module = lower_source("").unwrap();
        assert_eq!(module.functions().len(), 0);
    }

    #[test]
    fn test_lower_simple_function() {
        let source = r#"
            fn add(x: i32, y: i32) -> i32 {
                return 42
            }
        "#;

        let module = lower_source(source).unwrap();
        assert_eq!(module.functions().len(), 1);

        let func = module.get_function_by_name("add").unwrap();
        assert_eq!(func.name, "add");
        assert_eq!(func.params.len(), 2);
    }
}
