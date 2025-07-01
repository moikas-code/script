use std::fmt;

pub use super::instruction::Constant;

/// Value identifier in the IR
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueId(pub u32);

/// Value in the IR
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Instruction result
    Instruction(ValueId),
    /// Function parameter
    Parameter(u32),
    /// Constant value
    Constant(Constant),
}

impl fmt::Display for ValueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%{}", self.0)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Instruction(id) => write!(f, "{}", id),
            Value::Parameter(idx) => write!(f, "%arg{}", idx),
            Value::Constant(c) => write!(f, "{}", c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_value_id_display() {
        let id = ValueId(42);
        assert_eq!(id.to_string(), "%42");
    }
    
    #[test]
    fn test_value_display() {
        let inst_val = Value::Instruction(ValueId(10));
        assert_eq!(inst_val.to_string(), "%10");
        
        let param_val = Value::Parameter(2);
        assert_eq!(param_val.to_string(), "%arg2");
        
        let const_val = Value::Constant(Constant::I32(100));
        assert_eq!(const_val.to_string(), "100i32");
    }
}