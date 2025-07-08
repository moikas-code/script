use super::{BlockId, FunctionId, ValueId};
use crate::source::Span;
use crate::types::Type;
use std::fmt;

/// IR instruction with optional debug location
#[derive(Debug, Clone, PartialEq)]
pub struct InstructionWithLocation {
    pub instruction: Instruction,
    pub source_location: Option<Span>,
}

impl InstructionWithLocation {
    pub fn new(instruction: Instruction) -> Self {
        Self {
            instruction,
            source_location: None,
        }
    }

    pub fn with_location(instruction: Instruction, location: Span) -> Self {
        Self {
            instruction,
            source_location: Some(location),
        }
    }
}

/// IR instruction types
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    /// Constant value
    Const(Constant),

    /// Binary operation
    Binary {
        op: BinaryOp,
        lhs: ValueId,
        rhs: ValueId,
        ty: Type,
    },

    /// Unary operation
    Unary {
        op: UnaryOp,
        operand: ValueId,
        ty: Type,
    },

    /// Comparison operation (always returns bool)
    Compare {
        op: ComparisonOp,
        lhs: ValueId,
        rhs: ValueId,
    },

    /// Type cast
    Cast {
        value: ValueId,
        from_ty: Type,
        to_ty: Type,
    },

    /// Function call
    Call {
        func: FunctionId,
        args: Vec<ValueId>,
        ty: Type,
    },

    /// Allocate memory for a value
    Alloc { ty: Type },

    /// Load value from memory
    Load { ptr: ValueId, ty: Type },

    /// Store value to memory
    Store { ptr: ValueId, value: ValueId },

    /// Get element pointer (for arrays)
    GetElementPtr {
        ptr: ValueId,
        index: ValueId,
        elem_ty: Type,
    },

    /// Get object field pointer by name
    GetFieldPtr {
        object: ValueId,
        field_name: String,
        field_ty: Type,
    },

    /// Load object field by name (direct field access)
    LoadField {
        object: ValueId,
        field_name: String,
        field_ty: Type,
    },

    /// Store value to object field by name
    StoreField {
        object: ValueId,
        field_name: String,
        value: ValueId,
    },

    /// Allocate memory for a struct
    AllocStruct {
        struct_name: String,
        ty: Type,
    },

    /// Construct a struct with field values
    ConstructStruct {
        struct_name: String,
        fields: Vec<(String, ValueId)>,
        ty: Type,
    },

    /// Allocate memory for an enum
    AllocEnum {
        enum_name: String,
        variant_size: u32,  // Max size of all variants
        ty: Type,
    },

    /// Construct an enum variant
    ConstructEnum {
        enum_name: String,
        variant: String,
        tag: u32,           // Discriminant value
        args: Vec<ValueId>, // For tuple/struct variants
        ty: Type,
    },

    /// Get enum discriminant/tag
    GetEnumTag {
        enum_value: ValueId,
    },

    /// Set enum discriminant/tag
    SetEnumTag {
        enum_ptr: ValueId,
        tag: u32,
    },

    /// Extract variant data from enum
    ExtractEnumData {
        enum_value: ValueId,
        variant_index: u32,  // Which field of the variant to extract
        ty: Type,            // Type of the extracted data
    },

    /// Phi node for SSA form
    Phi {
        incoming: Vec<(ValueId, BlockId)>,
        ty: Type,
    },

    /// Return from function
    Return(Option<ValueId>),

    /// Unconditional branch
    Branch(BlockId),

    /// Conditional branch
    CondBranch {
        condition: ValueId,
        then_block: BlockId,
        else_block: BlockId,
    },

    /// Suspend async function execution
    /// Saves state and yields to executor
    Suspend {
        /// State to save before suspension
        state: ValueId,
        /// Continuation block after resume
        resume_block: BlockId,
    },

    /// Poll a future to check if it's ready
    /// Returns a Result-like type with Ready(T) or Pending
    PollFuture {
        /// The future to poll
        future: ValueId,
        /// Type of the future's output
        output_ty: Type,
    },

    /// Create an async state machine
    /// Used to transform async functions into state machines
    CreateAsyncState {
        /// Initial state value
        initial_state: u32,
        /// Size of state storage needed
        state_size: u32,
        /// Return type of the async function
        output_ty: Type,
    },

    /// Store value in async state
    StoreAsyncState {
        /// State machine pointer
        state_ptr: ValueId,
        /// Field offset in state
        offset: u32,
        /// Value to store
        value: ValueId,
    },

    /// Load value from async state
    LoadAsyncState {
        /// State machine pointer
        state_ptr: ValueId,
        /// Field offset in state
        offset: u32,
        /// Type of value to load
        ty: Type,
    },

    /// Get current state of async state machine
    GetAsyncState {
        /// State machine pointer
        state_ptr: ValueId,
    },

    /// Set current state of async state machine
    SetAsyncState {
        /// State machine pointer
        state_ptr: ValueId,
        /// New state value
        new_state: u32,
    },

    /// Bounds check for array indexing (security enhancement)
    /// Validates that index is within array bounds before access
    BoundsCheck {
        /// Array value to check bounds on
        array: ValueId,
        /// Index value to validate
        index: ValueId,
        /// Array length (if known at compile time) 
        length: Option<ValueId>,
        /// Error message for bounds violation
        error_msg: String,
    },

    /// Validate field access for type safety (security enhancement)
    /// Ensures field exists on the type before dynamic access
    ValidateFieldAccess {
        /// Object to access field on
        object: ValueId,
        /// Name of field to validate
        field_name: String,
        /// Type of the object (for validation)
        object_type: Type,
    },

    /// Error propagation (? operator)
    /// Checks if Result/Option is error/None and early returns if so
    ErrorPropagation {
        /// The Result or Option value to check
        value: ValueId,
        /// The type of the value (Result<T, E> or Option<T>)
        value_type: Type,
        /// The success type (T in Result<T, E> or Option<T>)
        success_type: Type,
    },
}

/// Constant values
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    I32(i32),
    F32(f32),
    Bool(bool),
    String(String),
    Null,
}

/// Binary operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,

    // Logical
    And,
    Or,
}

/// Unary operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    // Arithmetic
    Neg,

    // Logical
    Not,
}

/// Comparison operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl Instruction {
    /// Get the type produced by this instruction
    pub fn result_type(&self) -> Option<Type> {
        match self {
            Instruction::Const(c) => Some(c.get_type()),
            Instruction::Binary { ty, .. } => Some(ty.clone()),
            Instruction::Unary { ty, .. } => Some(ty.clone()),
            Instruction::Compare { .. } => Some(Type::Bool),
            Instruction::Cast { to_ty, .. } => Some(to_ty.clone()),
            Instruction::Call { ty, .. } => Some(ty.clone()),
            Instruction::Alloc { ty } => Some(Type::Named(format!("ptr<{}>", ty))),
            Instruction::Load { ty, .. } => Some(ty.clone()),
            Instruction::Store { .. } => None,
            Instruction::GetElementPtr { elem_ty, .. } => {
                Some(Type::Named(format!("ptr<{}>", elem_ty)))
            }
            Instruction::GetFieldPtr { field_ty, .. } => {
                Some(Type::Named(format!("ptr<{}>", field_ty)))
            }
            Instruction::LoadField { field_ty, .. } => Some(field_ty.clone()),
            Instruction::StoreField { .. } => None,
            Instruction::AllocStruct { ty, .. } => Some(Type::Named(format!("ptr<{}>", ty))),
            Instruction::ConstructStruct { ty, .. } => Some(ty.clone()),
            Instruction::AllocEnum { ty, .. } => Some(Type::Named(format!("ptr<{}>", ty))),
            Instruction::ConstructEnum { ty, .. } => Some(ty.clone()),
            Instruction::GetEnumTag { .. } => Some(Type::I32),
            Instruction::SetEnumTag { .. } => None,
            Instruction::ExtractEnumData { ty, .. } => Some(ty.clone()),
            Instruction::Phi { ty, .. } => Some(ty.clone()),
            Instruction::Return(_) => None,
            Instruction::Branch(_) => None,
            Instruction::CondBranch { .. } => None,
            Instruction::Suspend { .. } => None,
            Instruction::PollFuture { output_ty, .. } => {
                // Returns Poll<T> which we'll represent as an enum
                Some(Type::Generic { 
                    name: "Poll".to_string(),
                    args: vec![output_ty.clone()]
                })
            }
            Instruction::CreateAsyncState { output_ty, .. } => {
                // Returns a Future<T>
                Some(Type::Future(Box::new(output_ty.clone())))
            }
            Instruction::StoreAsyncState { .. } => None,
            Instruction::LoadAsyncState { ty, .. } => Some(ty.clone()),
            Instruction::GetAsyncState { .. } => Some(Type::I32), // State is represented as u32/i32
            Instruction::SetAsyncState { .. } => None,
            Instruction::BoundsCheck { .. } => Some(Type::Bool), // Returns true if bounds check passes
            Instruction::ValidateFieldAccess { .. } => Some(Type::Bool), // Returns true if field access is valid
        }
    }

    /// Check if this is a terminator instruction
    pub fn is_terminator(&self) -> bool {
        matches!(
            self,
            Instruction::Return(_) | Instruction::Branch(_) | Instruction::CondBranch { .. } | Instruction::Suspend { .. }
        )
    }
}

impl Constant {
    /// Get the type of this constant
    pub fn get_type(&self) -> Type {
        match self {
            Constant::I32(_) => Type::I32,
            Constant::F32(_) => Type::F32,
            Constant::Bool(_) => Type::Bool,
            Constant::String(_) => Type::String,
            Constant::Null => Type::Unknown,
        }
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::Const(c) => write!(f, "const {}", c),
            Instruction::Binary { op, lhs, rhs, ty } => {
                write!(f, "{} {} {}, {} : {}", op, ty, lhs, rhs, ty)
            }
            Instruction::Unary { op, operand, ty } => {
                write!(f, "{} {} {} : {}", op, ty, operand, ty)
            }
            Instruction::Compare { op, lhs, rhs } => {
                write!(f, "{} {}, {}", op, lhs, rhs)
            }
            Instruction::Cast {
                value,
                from_ty,
                to_ty,
            } => {
                write!(f, "cast {} : {} to {}", value, from_ty, to_ty)
            }
            Instruction::Call { func, args, ty } => {
                write!(f, "call {:?}(", func)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ") : {}", ty)
            }
            Instruction::Alloc { ty } => write!(f, "alloc {}", ty),
            Instruction::Load { ptr, ty } => write!(f, "load {} : {}", ptr, ty),
            Instruction::Store { ptr, value } => write!(f, "store {}, {}", value, ptr),
            Instruction::GetElementPtr {
                ptr,
                index,
                elem_ty,
            } => {
                write!(f, "getelementptr {}, {} : {}", ptr, index, elem_ty)
            }
            Instruction::GetFieldPtr {
                object,
                field_name,
                field_ty,
            } => {
                write!(
                    f,
                    "getfieldptr {}, \"{}\" : {}",
                    object, field_name, field_ty
                )
            }
            Instruction::LoadField {
                object,
                field_name,
                field_ty,
            } => {
                write!(f, "loadfield {}, \"{}\" : {}", object, field_name, field_ty)
            }
            Instruction::StoreField {
                object,
                field_name,
                value,
            } => {
                write!(f, "storefield {}, \"{}\", {}", object, field_name, value)
            }
            Instruction::AllocStruct { struct_name, ty } => {
                write!(f, "allocstruct {} : {}", struct_name, ty)
            }
            Instruction::ConstructStruct { struct_name, fields, ty } => {
                write!(f, "constructstruct {} {{ ", struct_name)?;
                for (i, (name, val)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", name, val)?;
                }
                write!(f, " }} : {}", ty)
            }
            Instruction::AllocEnum { enum_name, variant_size, ty } => {
                write!(f, "allocenum {} [size={}] : {}", enum_name, variant_size, ty)
            }
            Instruction::ConstructEnum { enum_name, variant, tag, args, ty } => {
                write!(f, "constructenum {}::{} [tag={}]", enum_name, variant, tag)?;
                if !args.is_empty() {
                    write!(f, "(")?;
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, ")")?;
                }
                write!(f, " : {}", ty)
            }
            Instruction::GetEnumTag { enum_value } => {
                write!(f, "getenumtag {}", enum_value)
            }
            Instruction::SetEnumTag { enum_ptr, tag } => {
                write!(f, "setenumtag {}, {}", enum_ptr, tag)
            }
            Instruction::ExtractEnumData { enum_value, variant_index, ty } => {
                write!(f, "extractenumdata {}, [index={}] : {}", enum_value, variant_index, ty)
            }
            Instruction::Phi { incoming, ty } => {
                write!(f, "phi ")?;
                for (i, (val, block)) in incoming.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "[{}, {:?}]", val, block)?;
                }
                write!(f, " : {}", ty)
            }
            Instruction::Return(None) => write!(f, "return"),
            Instruction::Return(Some(val)) => write!(f, "return {}", val),
            Instruction::Branch(block) => write!(f, "br {:?}", block),
            Instruction::CondBranch {
                condition,
                then_block,
                else_block,
            } => {
                write!(f, "br {}, {:?}, {:?}", condition, then_block, else_block)
            }
            Instruction::Suspend { state, resume_block } => {
                write!(f, "suspend {}, resume at {:?}", state, resume_block)
            }
            Instruction::PollFuture { future, output_ty } => {
                write!(f, "poll {} : {}", future, output_ty)
            }
            Instruction::CreateAsyncState {
                initial_state,
                state_size,
                output_ty,
            } => {
                write!(
                    f,
                    "create_async_state {}, size={} : Future<{}>",
                    initial_state, state_size, output_ty
                )
            }
            Instruction::StoreAsyncState { state_ptr, offset, value } => {
                write!(f, "store_async_state {}, offset={}, {}", state_ptr, offset, value)
            }
            Instruction::LoadAsyncState { state_ptr, offset, ty } => {
                write!(f, "load_async_state {}, offset={} : {}", state_ptr, offset, ty)
            }
            Instruction::GetAsyncState { state_ptr } => {
                write!(f, "get_async_state {}", state_ptr)
            }
            Instruction::SetAsyncState { state_ptr, new_state } => {
                write!(f, "set_async_state {}, {}", state_ptr, new_state)
            }
            Instruction::BoundsCheck { array, index, length, error_msg } => {
                match length {
                    Some(len) => write!(f, "bounds_check {}, {}, {} : \"{}\"", array, index, len, error_msg),
                    None => write!(f, "bounds_check {}, {} : \"{}\"", array, index, error_msg),
                }
            }
            Instruction::ValidateFieldAccess { object, field_name, object_type } => {
                write!(f, "validate_field_access {}, \"{}\" : {}", object, field_name, object_type)
            }
        }
    }
}

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Constant::I32(n) => write!(f, "{}i32", n),
            Constant::F32(n) => write!(f, "{}f32", n),
            Constant::Bool(b) => write!(f, "{}", b),
            Constant::String(s) => write!(f, "\"{}\"", s),
            Constant::Null => write!(f, "null"),
        }
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op_str = match self {
            BinaryOp::Add => "add",
            BinaryOp::Sub => "sub",
            BinaryOp::Mul => "mul",
            BinaryOp::Div => "div",
            BinaryOp::Mod => "mod",
            BinaryOp::And => "and",
            BinaryOp::Or => "or",
        };
        write!(f, "{}", op_str)
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op_str = match self {
            UnaryOp::Neg => "neg",
            UnaryOp::Not => "not",
        };
        write!(f, "{}", op_str)
    }
}

impl fmt::Display for ComparisonOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op_str = match self {
            ComparisonOp::Eq => "eq",
            ComparisonOp::Ne => "ne",
            ComparisonOp::Lt => "lt",
            ComparisonOp::Le => "le",
            ComparisonOp::Gt => "gt",
            ComparisonOp::Ge => "ge",
        };
        write!(f, "{}", op_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_result_type() {
        let const_inst = Instruction::Const(Constant::I32(42));
        assert_eq!(const_inst.result_type(), Some(Type::I32));

        let add_inst = Instruction::Binary {
            op: BinaryOp::Add,
            lhs: ValueId(0),
            rhs: ValueId(1),
            ty: Type::I32,
        };
        assert_eq!(add_inst.result_type(), Some(Type::I32));

        let cmp_inst = Instruction::Compare {
            op: ComparisonOp::Eq,
            lhs: ValueId(0),
            rhs: ValueId(1),
        };
        assert_eq!(cmp_inst.result_type(), Some(Type::Bool));

        let ret_inst = Instruction::Return(None);
        assert_eq!(ret_inst.result_type(), None);
    }

    #[test]
    fn test_is_terminator() {
        assert!(Instruction::Return(None).is_terminator());
        assert!(Instruction::Branch(BlockId(0)).is_terminator());
        assert!(Instruction::CondBranch {
            condition: ValueId(0),
            then_block: BlockId(1),
            else_block: BlockId(2),
        }
        .is_terminator());

        assert!(!Instruction::Const(Constant::I32(42)).is_terminator());
    }

    #[test]
    fn test_constant_type() {
        assert_eq!(Constant::I32(42).get_type(), Type::I32);
        assert_eq!(Constant::F32(3.14).get_type(), Type::F32);
        assert_eq!(Constant::Bool(true).get_type(), Type::Bool);
        assert_eq!(
            Constant::String("test".to_string()).get_type(),
            Type::String
        );
        assert_eq!(Constant::Null.get_type(), Type::Unknown);
    }
}
