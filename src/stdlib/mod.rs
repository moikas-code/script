//! Standard library for the Script programming language
//! 
//! This module provides the core functionality that Script programs can use,
//! including I/O operations, string manipulation, collections, and core types.
//! 
//! All functions in this module are designed to be called from Script code
//! and integrate with the Script runtime system.

pub mod io;
pub mod string;
pub mod core_types;
pub mod collections;
pub mod math;
pub mod game;

// Re-export commonly used items
pub use io::{print, println, eprintln, read_line, read_file, write_file};
pub use string::{ScriptString, StringOps};
pub use core_types::{ScriptOption, ScriptResult};
pub use collections::{ScriptVec, ScriptHashMap};

use crate::runtime::{ScriptRc, RuntimeError};
use crate::types::Type;
use std::collections::HashMap;

/// Standard library function registry
/// Maps function names to their implementations and type signatures
pub struct StdLib {
    functions: HashMap<String, StdLibFunction>,
}

/// A standard library function that can be called from Script
pub struct StdLibFunction {
    /// The name of the function
    pub name: String,
    /// The type signature of the function
    pub signature: Type,
    /// The implementation of the function
    pub implementation: fn(&[ScriptValue]) -> Result<ScriptValue, RuntimeError>,
}

/// A value in the Script runtime
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptValue {
    /// 32-bit integer
    I32(i32),
    /// 32-bit float
    F32(f32),
    /// Boolean
    Bool(bool),
    /// String
    String(ScriptRc<ScriptString>),
    /// Array
    Array(ScriptRc<ScriptVec>),
    /// HashMap
    HashMap(ScriptRc<ScriptHashMap>),
    /// Option type
    Option(ScriptRc<ScriptOption>),
    /// Result type
    Result(ScriptRc<ScriptResult>),
    /// Unit/void type
    Unit,
    /// Object type (for vectors, matrices, etc.)
    Object(ScriptRc<HashMap<String, ScriptValue>>),
}

impl ScriptValue {
    /// Get the type of this value
    pub fn get_type(&self) -> Type {
        match self {
            ScriptValue::I32(_) => Type::I32,
            ScriptValue::F32(_) => Type::F32,
            ScriptValue::Bool(_) => Type::Bool,
            ScriptValue::String(_) => Type::String,
            ScriptValue::Array(_) => Type::Named("Array".to_string()),
            ScriptValue::HashMap(_) => Type::Named("HashMap".to_string()),
            ScriptValue::Option(_) => Type::Named("Option".to_string()),
            ScriptValue::Result(_) => Type::Named("Result".to_string()),
            ScriptValue::Unit => Type::Named("unit".to_string()),
            ScriptValue::Object(_) => Type::Named("Object".to_string()),
        }
    }
    
    /// Convert to i32 if possible
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            ScriptValue::I32(val) => Some(*val),
            _ => None,
        }
    }
    
    /// Convert to f32 if possible
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            ScriptValue::F32(val) => Some(*val),
            _ => None,
        }
    }
    
    /// Convert to bool if possible
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ScriptValue::Bool(val) => Some(*val),
            _ => None,
        }
    }
    
    /// Convert to string if possible
    pub fn as_string(&self) -> Option<&ScriptString> {
        match self {
            ScriptValue::String(val) => Some(val),
            _ => None,
        }
    }
    
    /// Convert to array if possible
    pub fn as_array(&self) -> Option<&ScriptVec> {
        match self {
            ScriptValue::Array(val) => Some(val),
            _ => None,
        }
    }
    
    /// Check if this is a unit value
    pub fn is_unit(&self) -> bool {
        matches!(self, ScriptValue::Unit)
    }
    
    /// Convert to f32 with type coercion from i32
    pub fn to_f32(&self) -> Result<f32, RuntimeError> {
        match self {
            ScriptValue::F32(val) => Ok(*val),
            ScriptValue::I32(val) => Ok(*val as f32),
            _ => Err(RuntimeError::InvalidOperation(
                format!("Cannot convert {:?} to f32", self.get_type())
            )),
        }
    }
    
    /// Convert to i32 with type coercion from f32
    pub fn to_i32(&self) -> Result<i32, RuntimeError> {
        match self {
            ScriptValue::I32(val) => Ok(*val),
            ScriptValue::F32(val) => Ok(*val as i32),
            _ => Err(RuntimeError::InvalidOperation(
                format!("Cannot convert {:?} to i32", self.get_type())
            )),
        }
    }
}

impl StdLib {
    /// Create a new standard library instance with all built-in functions
    pub fn new() -> Self {
        let mut stdlib = StdLib {
            functions: HashMap::new(),
        };
        
        // Register all standard library functions
        stdlib.register_io_functions();
        stdlib.register_string_functions();
        stdlib.register_core_type_functions();
        stdlib.register_collection_functions();
        stdlib.register_math_functions();
        stdlib.register_game_functions();
        
        stdlib
    }
    
    /// Register a function in the standard library
    fn register_function(
        &mut self, 
        name: &str, 
        signature: Type,
        implementation: fn(&[ScriptValue]) -> Result<ScriptValue, RuntimeError>
    ) {
        self.functions.insert(
            name.to_string(),
            StdLibFunction {
                name: name.to_string(),
                signature,
                implementation,
            }
        );
    }
    
    /// Get a function by name
    pub fn get_function(&self, name: &str) -> Option<&StdLibFunction> {
        self.functions.get(name)
    }
    
    /// Get all function names
    pub fn function_names(&self) -> Vec<&str> {
        self.functions.keys().map(|s| s.as_str()).collect()
    }
    
    /// Register I/O functions
    fn register_io_functions(&mut self) {
        // print function: (string) -> unit
        self.register_function(
            "print",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::Named("unit".to_string())),
            },
            io::print_impl,
        );
        
        // println function: (string) -> unit
        self.register_function(
            "println",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::Named("unit".to_string())),
            },
            io::println_impl,
        );
        
        // eprintln function: (string) -> unit
        self.register_function(
            "eprintln",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::Named("unit".to_string())),
            },
            io::eprintln_impl,
        );
        
        // read_line function: () -> Result<string, string>
        self.register_function(
            "read_line",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Result {
                    ok: Box::new(Type::String),
                    err: Box::new(Type::String),
                }),
            },
            io::read_line_impl,
        );
        
        // read_file function: (string) -> Result<string, string>
        self.register_function(
            "read_file",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::Result {
                    ok: Box::new(Type::String),
                    err: Box::new(Type::String),
                }),
            },
            io::read_file_impl,
        );
        
        // write_file function: (string, string) -> Result<unit, string>
        self.register_function(
            "write_file",
            Type::Function {
                params: vec![Type::String, Type::String],
                ret: Box::new(Type::Result {
                    ok: Box::new(Type::Named("unit".to_string())),
                    err: Box::new(Type::String),
                }),
            },
            io::write_file_impl,
        );
    }
    
    /// Register string manipulation functions
    fn register_string_functions(&mut self) {
        // String length
        self.register_function(
            "string_len",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::I32),
            },
            string::string_len_impl,
        );
        
        // String case conversion
        self.register_function(
            "to_uppercase",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::String),
            },
            string::string_to_uppercase_impl,
        );
        
        self.register_function(
            "to_lowercase",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::String),
            },
            string::string_to_lowercase_impl,
        );
        
        // String trimming
        self.register_function(
            "trim",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::String),
            },
            string::string_trim_impl,
        );
        
        // String split
        self.register_function(
            "split",
            Type::Function {
                params: vec![Type::String, Type::String],
                ret: Box::new(Type::Array(Box::new(Type::String))),
            },
            string::string_split_impl,
        );
        
        // String contains
        self.register_function(
            "contains",
            Type::Function {
                params: vec![Type::String, Type::String],
                ret: Box::new(Type::Bool),
            },
            string::string_contains_impl,
        );
        
        // String replace
        self.register_function(
            "replace",
            Type::Function {
                params: vec![Type::String, Type::String, Type::String],
                ret: Box::new(Type::String),
            },
            string::string_replace_impl,
        );
    }
    
    /// Register core type functions
    fn register_core_type_functions(&mut self) {
        // Option functions
        self.register_function(
            "Option::some",
            Type::Function {
                params: vec![Type::Unknown], // Generic T
                ret: Box::new(Type::Named("Option".to_string())),
            },
            core_types::option_some_impl,
        );
        
        self.register_function(
            "Option::none",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Named("Option".to_string())),
            },
            core_types::option_none_impl,
        );
        
        self.register_function(
            "is_some",
            Type::Function {
                params: vec![Type::Named("Option".to_string())],
                ret: Box::new(Type::Bool),
            },
            core_types::option_is_some_impl,
        );
        
        self.register_function(
            "is_none",
            Type::Function {
                params: vec![Type::Named("Option".to_string())],
                ret: Box::new(Type::Bool),
            },
            core_types::option_is_none_impl,
        );
        
        self.register_function(
            "option_unwrap",
            Type::Function {
                params: vec![Type::Named("Option".to_string())],
                ret: Box::new(Type::Unknown),
            },
            core_types::option_unwrap_impl,
        );
        
        // Result functions
        self.register_function(
            "Result::ok",
            Type::Function {
                params: vec![Type::Unknown], // Generic T
                ret: Box::new(Type::Named("Result".to_string())),
            },
            core_types::result_ok_impl,
        );
        
        self.register_function(
            "Result::err",
            Type::Function {
                params: vec![Type::Unknown], // Generic E
                ret: Box::new(Type::Named("Result".to_string())),
            },
            core_types::result_err_impl,
        );
        
        self.register_function(
            "is_ok",
            Type::Function {
                params: vec![Type::Named("Result".to_string())],
                ret: Box::new(Type::Bool),
            },
            core_types::result_is_ok_impl,
        );
        
        self.register_function(
            "is_err",
            Type::Function {
                params: vec![Type::Named("Result".to_string())],
                ret: Box::new(Type::Bool),
            },
            core_types::result_is_err_impl,
        );
        
        self.register_function(
            "result_unwrap",
            Type::Function {
                params: vec![Type::Named("Result".to_string())],
                ret: Box::new(Type::Unknown),
            },
            core_types::result_unwrap_impl,
        );
        
        self.register_function(
            "unwrap_err",
            Type::Function {
                params: vec![Type::Named("Result".to_string())],
                ret: Box::new(Type::Unknown),
            },
            core_types::result_unwrap_err_impl,
        );
    }
    
    /// Register collection functions
    fn register_collection_functions(&mut self) {
        // Vec functions
        self.register_function(
            "Vec::new",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Array(Box::new(Type::Unknown))),
            },
            collections::vec_new_impl,
        );
        
        self.register_function(
            "vec_len",
            Type::Function {
                params: vec![Type::Array(Box::new(Type::Unknown))],
                ret: Box::new(Type::I32),
            },
            collections::vec_len_impl,
        );
        
        self.register_function(
            "vec_push",
            Type::Function {
                params: vec![Type::Array(Box::new(Type::Unknown)), Type::Unknown],
                ret: Box::new(Type::Named("unit".to_string())),
            },
            collections::vec_push_impl,
        );
        
        self.register_function(
            "vec_pop",
            Type::Function {
                params: vec![Type::Array(Box::new(Type::Unknown))],
                ret: Box::new(Type::Named("Option".to_string())),
            },
            collections::vec_pop_impl,
        );
        
        self.register_function(
            "vec_get",
            Type::Function {
                params: vec![Type::Array(Box::new(Type::Unknown)), Type::I32],
                ret: Box::new(Type::Named("Option".to_string())),
            },
            collections::vec_get_impl,
        );
        
        // HashMap functions
        self.register_function(
            "HashMap::new",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Named("HashMap".to_string())),
            },
            collections::hashmap_new_impl,
        );
        
        self.register_function(
            "hashmap_insert",
            Type::Function {
                params: vec![Type::Named("HashMap".to_string()), Type::String, Type::Unknown],
                ret: Box::new(Type::Named("Option".to_string())),
            },
            collections::hashmap_insert_impl,
        );
        
        self.register_function(
            "hashmap_get",
            Type::Function {
                params: vec![Type::Named("HashMap".to_string()), Type::String],
                ret: Box::new(Type::Named("Option".to_string())),
            },
            collections::hashmap_get_impl,
        );
        
        self.register_function(
            "hashmap_contains_key",
            Type::Function {
                params: vec![Type::Named("HashMap".to_string()), Type::String],
                ret: Box::new(Type::Bool),
            },
            collections::hashmap_contains_key_impl,
        );
    }
    
    /// Register math functions
    fn register_math_functions(&mut self) {
        // Basic operations
        self.register_function(
            "abs",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::F32),
            },
            math::abs_impl,
        );
        
        self.register_function(
            "min",
            Type::Function {
                params: vec![Type::F32, Type::F32],
                ret: Box::new(Type::F32),
            },
            math::min_impl,
        );
        
        self.register_function(
            "max",
            Type::Function {
                params: vec![Type::F32, Type::F32],
                ret: Box::new(Type::F32),
            },
            math::max_impl,
        );
        
        self.register_function(
            "sign",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::F32),
            },
            math::sign_impl,
        );
        
        // Power and roots
        self.register_function(
            "pow",
            Type::Function {
                params: vec![Type::F32, Type::F32],
                ret: Box::new(Type::F32),
            },
            math::pow_impl,
        );
        
        self.register_function(
            "sqrt",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::F32),
            },
            math::sqrt_impl,
        );
        
        self.register_function(
            "cbrt",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::F32),
            },
            math::cbrt_impl,
        );
        
        // Exponential and logarithm
        self.register_function(
            "exp",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::F32),
            },
            math::exp_impl,
        );
        
        self.register_function(
            "log",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::F32),
            },
            math::log_impl,
        );
        
        self.register_function(
            "log10",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::F32),
            },
            math::log10_impl,
        );
        
        self.register_function(
            "log2",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::F32),
            },
            math::log2_impl,
        );
        
        // Trigonometry
        self.register_function(
            "sin",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::F32),
            },
            math::sin_impl,
        );
        
        self.register_function(
            "cos",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::F32),
            },
            math::cos_impl,
        );
        
        self.register_function(
            "tan",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::F32),
            },
            math::tan_impl,
        );
        
        self.register_function(
            "asin",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::F32),
            },
            math::asin_impl,
        );
        
        self.register_function(
            "acos",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::F32),
            },
            math::acos_impl,
        );
        
        self.register_function(
            "atan",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::F32),
            },
            math::atan_impl,
        );
        
        self.register_function(
            "atan2",
            Type::Function {
                params: vec![Type::F32, Type::F32],
                ret: Box::new(Type::F32),
            },
            math::atan2_impl,
        );
        
        // Hyperbolic functions
        self.register_function(
            "sinh",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::F32),
            },
            math::sinh_impl,
        );
        
        self.register_function(
            "cosh",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::F32),
            },
            math::cosh_impl,
        );
        
        self.register_function(
            "tanh",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::F32),
            },
            math::tanh_impl,
        );
        
        // Rounding
        self.register_function(
            "floor",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::F32),
            },
            math::floor_impl,
        );
        
        self.register_function(
            "ceil",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::F32),
            },
            math::ceil_impl,
        );
        
        self.register_function(
            "round",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::F32),
            },
            math::round_impl,
        );
        
        self.register_function(
            "trunc",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::F32),
            },
            math::trunc_impl,
        );
    }
    
    /// Register game-oriented utility functions
    fn register_game_functions(&mut self) {
        // Vector constructors
        self.register_function(
            "vec2",
            Type::Function {
                params: vec![Type::F32, Type::F32],
                ret: Box::new(Type::Named("Object".to_string())),
            },
            game::vec2_new,
        );
        
        self.register_function(
            "vec3",
            Type::Function {
                params: vec![Type::F32, Type::F32, Type::F32],
                ret: Box::new(Type::Named("Object".to_string())),
            },
            game::vec3_new,
        );
        
        self.register_function(
            "vec4",
            Type::Function {
                params: vec![Type::F32, Type::F32, Type::F32, Type::F32],
                ret: Box::new(Type::Named("Object".to_string())),
            },
            game::vec4_new,
        );
        
        // Vector operations
        self.register_function(
            "vec2_add",
            Type::Function {
                params: vec![Type::Named("Object".to_string()), Type::Named("Object".to_string())],
                ret: Box::new(Type::Named("Object".to_string())),
            },
            game::vec2_add,
        );
        
        self.register_function(
            "vec2_dot",
            Type::Function {
                params: vec![Type::Named("Object".to_string()), Type::Named("Object".to_string())],
                ret: Box::new(Type::F32),
            },
            game::vec2_dot,
        );
        
        self.register_function(
            "vec2_length",
            Type::Function {
                params: vec![Type::Named("Object".to_string())],
                ret: Box::new(Type::F32),
            },
            game::vec2_length,
        );
        
        // Math helpers
        self.register_function(
            "lerp",
            Type::Function {
                params: vec![Type::F32, Type::F32, Type::F32],
                ret: Box::new(Type::F32),
            },
            game::lerp,
        );
        
        self.register_function(
            "clamp",
            Type::Function {
                params: vec![Type::F32, Type::F32, Type::F32],
                ret: Box::new(Type::F32),
            },
            game::clamp,
        );
        
        self.register_function(
            "smoothstep",
            Type::Function {
                params: vec![Type::F32, Type::F32, Type::F32],
                ret: Box::new(Type::F32),
            },
            game::smoothstep,
        );
        
        // Random functions
        self.register_function(
            "random",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::F32),
            },
            game::random,
        );
        
        self.register_function(
            "random_range",
            Type::Function {
                params: vec![Type::F32, Type::F32],
                ret: Box::new(Type::F32),
            },
            game::random_range,
        );
        
        self.register_function(
            "random_int",
            Type::Function {
                params: vec![Type::I32, Type::I32],
                ret: Box::new(Type::I32),
            },
            game::random_int,
        );
        
        // Time functions
        self.register_function(
            "time_now",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::F32),
            },
            game::time_now,
        );
        
        // Angle conversion
        self.register_function(
            "deg_to_rad",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::F32),
            },
            game::deg_to_rad,
        );
        
        self.register_function(
            "rad_to_deg",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::F32),
            },
            game::rad_to_deg,
        );
    }
}

impl Default for StdLib {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_stdlib_creation() {
        let stdlib = StdLib::new();
        
        // Check that basic I/O functions are registered
        assert!(stdlib.get_function("print").is_some());
        assert!(stdlib.get_function("println").is_some());
        assert!(stdlib.get_function("eprintln").is_some());
        assert!(stdlib.get_function("read_line").is_some());
        assert!(stdlib.get_function("read_file").is_some());
        assert!(stdlib.get_function("write_file").is_some());
    }
    
    #[test]
    fn test_script_value_types() {
        let int_val = ScriptValue::I32(42);
        assert_eq!(int_val.get_type(), Type::I32);
        assert_eq!(int_val.as_i32(), Some(42));
        assert_eq!(int_val.as_f32(), None);
        
        let float_val = ScriptValue::F32(3.14);
        assert_eq!(float_val.get_type(), Type::F32);
        assert_eq!(float_val.as_f32(), Some(3.14));
        assert_eq!(float_val.as_i32(), None);
        
        let bool_val = ScriptValue::Bool(true);
        assert_eq!(bool_val.get_type(), Type::Bool);
        assert_eq!(bool_val.as_bool(), Some(true));
        
        let unit_val = ScriptValue::Unit;
        assert!(unit_val.is_unit());
    }
}