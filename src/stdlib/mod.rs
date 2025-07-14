//! Standard library for the Script programming language
//!
//! This module provides the core functionality that Script programs can use,
//! including I/O operations, string manipulation, collections, and core types.
//!
//! All functions in this module are designed to be called from Script code
//! and integrate with the Script runtime system.

pub mod async_functional;
pub mod async_std;
pub mod closure_helpers;
pub mod collections;
pub mod core_types;
pub mod error;
pub mod functional;
pub mod functional_advanced;
pub mod game;
pub mod io;
pub mod iterators;
pub mod math;
pub mod network;
pub mod parallel;
pub mod random;
pub mod string;
pub mod time;

// Re-export commonly used items
pub use async_functional::{AsyncFunctionalConfig, AsyncFunctionalOps, FutureCombinators};
pub use async_std::{interval, join_all, race, sleep, timeout, yield_now};
pub use collections::{ScriptHashMap, ScriptHashSet, ScriptVec};
pub use core_types::{ScriptOption, ScriptResult};
pub use functional::{FunctionComposition, FunctionalExecutor, FunctionalOps};
pub use io::{
    append_file, copy_file, create_dir, delete_dir, delete_file, dir_exists, eprintln, file_exists,
    file_metadata, list_dir, print, println, read_file, read_line, write_file,
};
pub use iterators::{Generators, RangeIterator, ScriptIterator, VecIterator};
pub use network::{ScriptTcpListener, ScriptTcpStream, ScriptUdpSocket};
pub use parallel::{ParallelConfig, ParallelExecutor};
pub use string::{ScriptString, StringOps};

use crate::runtime::{RuntimeError, ScriptRc};
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
#[derive(Debug, Clone)]
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
    /// HashSet
    HashSet(ScriptRc<ScriptHashSet>),
    /// Option type
    Option(ScriptRc<ScriptOption>),
    /// Result type
    Result(ScriptRc<ScriptResult>),
    /// Unit/void type
    Unit,
    /// Object type (for vectors, matrices, etc.)
    Object(ScriptRc<HashMap<String, ScriptValue>>),
    /// Iterator type for lazy evaluation
    Iterator(ScriptRc<Box<dyn iterators::ScriptIterator>>),
    /// Closure type for functional programming
    Closure(ScriptRc<crate::runtime::closure::Closure>),
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
            ScriptValue::HashSet(_) => Type::Named("HashSet".to_string()),
            ScriptValue::Option(_) => Type::Named("Option".to_string()),
            ScriptValue::Result(_) => Type::Named("Result".to_string()),
            ScriptValue::Unit => Type::Named("unit".to_string()),
            ScriptValue::Object(_) => Type::Named("Object".to_string()),
            ScriptValue::Iterator(_) => Type::Named("Iterator".to_string()),
            ScriptValue::Closure(_) => Type::Function {
                params: vec![Type::Unknown],  // TODO: Extract actual parameter types
                ret: Box::new(Type::Unknown), // TODO: Extract actual return type
            },
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

    /// Convert to closure if possible
    pub fn as_closure(&self) -> Option<&crate::runtime::closure::Closure> {
        match self {
            ScriptValue::Closure(val) => Some(val),
            _ => None,
        }
    }

    /// Convert to iterator if possible
    pub fn as_iterator(&self) -> Option<&Box<dyn iterators::ScriptIterator>> {
        match self {
            ScriptValue::Iterator(val) => Some(val),
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
            _ => Err(RuntimeError::InvalidOperation(format!(
                "Cannot convert {:?} to f32",
                self.get_type()
            ))),
        }
    }

    /// Convert to i32 with type coercion from f32
    pub fn to_i32(&self) -> Result<i32, RuntimeError> {
        match self {
            ScriptValue::I32(val) => Ok(*val),
            ScriptValue::F32(val) => Ok(*val as i32),
            _ => Err(RuntimeError::InvalidOperation(format!(
                "Cannot convert {:?} to i32",
                self.get_type()
            ))),
        }
    }
}

impl PartialEq for ScriptValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ScriptValue::I32(a), ScriptValue::I32(b)) => a == b,
            (ScriptValue::F32(a), ScriptValue::F32(b)) => a == b,
            (ScriptValue::Bool(a), ScriptValue::Bool(b)) => a == b,
            (ScriptValue::String(a), ScriptValue::String(b)) => a == b,
            (ScriptValue::Array(a), ScriptValue::Array(b)) => a == b,
            (ScriptValue::HashMap(a), ScriptValue::HashMap(b)) => a == b,
            (ScriptValue::HashSet(a), ScriptValue::HashSet(b)) => a == b,
            (ScriptValue::Option(a), ScriptValue::Option(b)) => a == b,
            (ScriptValue::Result(a), ScriptValue::Result(b)) => a == b,
            (ScriptValue::Unit, ScriptValue::Unit) => true,
            (ScriptValue::Object(a), ScriptValue::Object(b)) => a == b,
            // Iterators and Closures cannot be compared for equality
            (ScriptValue::Iterator(_), ScriptValue::Iterator(_)) => false,
            (ScriptValue::Closure(_), ScriptValue::Closure(_)) => false,
            _ => false,
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
        stdlib.register_functional_programming_functions();
        stdlib.register_math_functions();
        stdlib.register_game_functions();
        stdlib.register_network_functions();
        stdlib.register_random_functions();
        stdlib.register_time_functions();

        stdlib
    }

    /// Register a function in the standard library
    fn register_function(
        &mut self,
        name: &str,
        signature: Type,
        implementation: fn(&[ScriptValue]) -> Result<ScriptValue, RuntimeError>,
    ) {
        self.functions.insert(
            name.to_string(),
            StdLibFunction {
                name: name.to_string(),
                signature,
                implementation,
            },
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

        // file_exists function: (string) -> Result<bool, string>
        self.register_function(
            "file_exists",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::Result {
                    ok: Box::new(Type::Bool),
                    err: Box::new(Type::String),
                }),
            },
            io::file_exists_impl,
        );

        // dir_exists function: (string) -> Result<bool, string>
        self.register_function(
            "dir_exists",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::Result {
                    ok: Box::new(Type::Bool),
                    err: Box::new(Type::String),
                }),
            },
            io::dir_exists_impl,
        );

        // create_dir function: (string) -> Result<unit, string>
        self.register_function(
            "create_dir",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::Result {
                    ok: Box::new(Type::Named("unit".to_string())),
                    err: Box::new(Type::String),
                }),
            },
            io::create_dir_impl,
        );

        // delete_file function: (string) -> Result<unit, string>
        self.register_function(
            "delete_file",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::Result {
                    ok: Box::new(Type::Named("unit".to_string())),
                    err: Box::new(Type::String),
                }),
            },
            io::delete_file_impl,
        );

        // copy_file function: (string, string) -> Result<unit, string>
        self.register_function(
            "copy_file",
            Type::Function {
                params: vec![Type::String, Type::String],
                ret: Box::new(Type::Result {
                    ok: Box::new(Type::Named("unit".to_string())),
                    err: Box::new(Type::String),
                }),
            },
            io::copy_file_impl,
        );

        // append_file function: (string, string) -> Result<unit, string>
        self.register_function(
            "append_file",
            Type::Function {
                params: vec![Type::String, Type::String],
                ret: Box::new(Type::Result {
                    ok: Box::new(Type::Named("unit".to_string())),
                    err: Box::new(Type::String),
                }),
            },
            io::append_file_impl,
        );

        // delete_dir function: (string) -> Result<unit, string>
        self.register_function(
            "delete_dir",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::Result {
                    ok: Box::new(Type::Named("unit".to_string())),
                    err: Box::new(Type::String),
                }),
            },
            io::delete_dir_impl,
        );

        // list_dir function: (string) -> Result<Array<string>, string>
        self.register_function(
            "list_dir",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::Result {
                    ok: Box::new(Type::Array(Box::new(Type::String))),
                    err: Box::new(Type::String),
                }),
            },
            io::list_dir_impl,
        );

        // file_metadata function: (string) -> Result<Object, string>
        self.register_function(
            "file_metadata",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::Result {
                    ok: Box::new(Type::Named("Object".to_string())),
                    err: Box::new(Type::String),
                }),
            },
            io::file_metadata_impl,
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

        // String join
        self.register_function(
            "join",
            Type::Function {
                params: vec![Type::String, Type::Array(Box::new(Type::String))],
                ret: Box::new(Type::String),
            },
            string::string_join_impl,
        );

        // String padding
        self.register_function(
            "pad_left",
            Type::Function {
                params: vec![Type::String, Type::I32, Type::String],
                ret: Box::new(Type::String),
            },
            string::string_pad_left_impl,
        );

        self.register_function(
            "pad_right",
            Type::Function {
                params: vec![Type::String, Type::I32, Type::String],
                ret: Box::new(Type::String),
            },
            string::string_pad_right_impl,
        );

        self.register_function(
            "center",
            Type::Function {
                params: vec![Type::String, Type::I32, Type::String],
                ret: Box::new(Type::String),
            },
            string::string_center_impl,
        );

        // String strip prefix/suffix
        self.register_function(
            "strip_prefix",
            Type::Function {
                params: vec![Type::String, Type::String],
                ret: Box::new(Type::String),
            },
            string::string_strip_prefix_impl,
        );

        self.register_function(
            "strip_suffix",
            Type::Function {
                params: vec![Type::String, Type::String],
                ret: Box::new(Type::String),
            },
            string::string_strip_suffix_impl,
        );

        // String case conversion
        self.register_function(
            "capitalize",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::String),
            },
            string::string_capitalize_impl,
        );

        self.register_function(
            "title_case",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::String),
            },
            string::string_title_case_impl,
        );

        // String analysis
        self.register_function(
            "count_matches",
            Type::Function {
                params: vec![Type::String, Type::String],
                ret: Box::new(Type::I32),
            },
            string::string_count_matches_impl,
        );

        self.register_function(
            "lines",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::Array(Box::new(Type::String))),
            },
            string::string_lines_impl,
        );

        self.register_function(
            "reverse",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::String),
            },
            string::string_reverse_impl,
        );

        // String predicates
        self.register_function(
            "is_alphabetic",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::Bool),
            },
            string::string_is_alphabetic_impl,
        );

        self.register_function(
            "is_numeric",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::Bool),
            },
            string::string_is_numeric_impl,
        );

        // String formatting
        self.register_function(
            "truncate",
            Type::Function {
                params: vec![Type::String, Type::I32, Type::String],
                ret: Box::new(Type::String),
            },
            string::string_truncate_impl,
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

        // Additional Option methods
        self.register_function(
            "option_and_then",
            Type::Function {
                params: vec![Type::Named("Option".to_string()), Type::Unknown],
                ret: Box::new(Type::Named("Option".to_string())),
            },
            core_types::option_and_then_impl,
        );

        self.register_function(
            "option_or",
            Type::Function {
                params: vec![
                    Type::Named("Option".to_string()),
                    Type::Named("Option".to_string()),
                ],
                ret: Box::new(Type::Named("Option".to_string())),
            },
            core_types::option_or_impl,
        );

        self.register_function(
            "option_unwrap_or",
            Type::Function {
                params: vec![Type::Named("Option".to_string()), Type::Unknown],
                ret: Box::new(Type::Unknown),
            },
            core_types::option_unwrap_or_impl,
        );

        self.register_function(
            "option_expect",
            Type::Function {
                params: vec![Type::Named("Option".to_string()), Type::String],
                ret: Box::new(Type::Unknown),
            },
            core_types::option_expect_impl,
        );

        self.register_function(
            "option_filter",
            Type::Function {
                params: vec![Type::Named("Option".to_string()), Type::Unknown],
                ret: Box::new(Type::Named("Option".to_string())),
            },
            core_types::option_filter_impl,
        );

        self.register_function(
            "option_ok_or",
            Type::Function {
                params: vec![Type::Named("Option".to_string()), Type::Unknown],
                ret: Box::new(Type::Named("Result".to_string())),
            },
            core_types::option_ok_or_impl,
        );

        // Additional Result methods
        self.register_function(
            "result_and_then",
            Type::Function {
                params: vec![Type::Named("Result".to_string()), Type::Unknown],
                ret: Box::new(Type::Named("Result".to_string())),
            },
            core_types::result_and_then_impl,
        );

        self.register_function(
            "result_or",
            Type::Function {
                params: vec![
                    Type::Named("Result".to_string()),
                    Type::Named("Result".to_string()),
                ],
                ret: Box::new(Type::Named("Result".to_string())),
            },
            core_types::result_or_impl,
        );

        self.register_function(
            "result_unwrap_or",
            Type::Function {
                params: vec![Type::Named("Result".to_string()), Type::Unknown],
                ret: Box::new(Type::Unknown),
            },
            core_types::result_unwrap_or_impl,
        );

        self.register_function(
            "result_expect",
            Type::Function {
                params: vec![Type::Named("Result".to_string()), Type::String],
                ret: Box::new(Type::Unknown),
            },
            core_types::result_expect_impl,
        );

        self.register_function(
            "result_map",
            Type::Function {
                params: vec![Type::Named("Result".to_string()), Type::Unknown],
                ret: Box::new(Type::Named("Result".to_string())),
            },
            core_types::result_map_impl,
        );

        self.register_function(
            "result_map_err",
            Type::Function {
                params: vec![Type::Named("Result".to_string()), Type::Unknown],
                ret: Box::new(Type::Named("Result".to_string())),
            },
            core_types::result_map_err_impl,
        );
    }

    /// Register functional programming functions
    fn register_functional_programming_functions(&mut self) {
        // Vector functional operations
        self.register_function(
            "vec_map",
            Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::Unknown)),
                    Type::Named("Closure".to_string()),
                ],
                ret: Box::new(Type::Array(Box::new(Type::Unknown))),
            },
            functional::vec_map_impl,
        );

        self.register_function(
            "vec_filter",
            Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::Unknown)),
                    Type::Named("Closure".to_string()),
                ],
                ret: Box::new(Type::Array(Box::new(Type::Unknown))),
            },
            functional::vec_filter_impl,
        );

        self.register_function(
            "vec_reduce",
            Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::Unknown)),
                    Type::Named("Closure".to_string()),
                    Type::Unknown,
                ],
                ret: Box::new(Type::Unknown),
            },
            functional::vec_reduce_impl,
        );

        self.register_function(
            "vec_for_each",
            Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::Unknown)),
                    Type::Named("Closure".to_string()),
                ],
                ret: Box::new(Type::Named("unit".to_string())),
            },
            functional::vec_for_each_impl,
        );

        self.register_function(
            "vec_find",
            Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::Unknown)),
                    Type::Named("Closure".to_string()),
                ],
                ret: Box::new(Type::Named("Option".to_string())),
            },
            functional::vec_find_impl,
        );

        self.register_function(
            "vec_every",
            Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::Unknown)),
                    Type::Named("Closure".to_string()),
                ],
                ret: Box::new(Type::Bool),
            },
            functional::vec_every_impl,
        );

        self.register_function(
            "vec_some",
            Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::Unknown)),
                    Type::Named("Closure".to_string()),
                ],
                ret: Box::new(Type::Bool),
            },
            functional::vec_some_impl,
        );

        // Function composition utilities
        self.register_function(
            "compose",
            Type::Function {
                params: vec![
                    Type::Named("Closure".to_string()),
                    Type::Named("Closure".to_string()),
                ],
                ret: Box::new(Type::Named("Closure".to_string())),
            },
            functional::compose_impl,
        );

        self.register_function(
            "partial",
            Type::Function {
                params: vec![
                    Type::Named("Closure".to_string()),
                    Type::Array(Box::new(Type::Unknown)),
                ],
                ret: Box::new(Type::Named("Closure".to_string())),
            },
            functional::partial_impl,
        );

        self.register_function(
            "curry",
            Type::Function {
                params: vec![Type::Named("Closure".to_string())],
                ret: Box::new(Type::Named("Closure".to_string())),
            },
            functional::curry_impl,
        );

        // Iterator functions
        self.register_function(
            "range",
            Type::Function {
                params: vec![Type::I32, Type::I32, Type::I32],
                ret: Box::new(Type::Named("Iterator".to_string())),
            },
            functional::range_impl,
        );

        self.register_function(
            "iter_collect",
            Type::Function {
                params: vec![Type::Named("Iterator".to_string())],
                ret: Box::new(Type::Array(Box::new(Type::Unknown))),
            },
            functional::iter_collect_impl,
        );

        self.register_function(
            "iter_take",
            Type::Function {
                params: vec![Type::Named("Iterator".to_string()), Type::I32],
                ret: Box::new(Type::Named("Iterator".to_string())),
            },
            functional::iter_take_impl,
        );

        self.register_function(
            "iter_skip",
            Type::Function {
                params: vec![Type::Named("Iterator".to_string()), Type::I32],
                ret: Box::new(Type::Named("Iterator".to_string())),
            },
            functional::iter_skip_impl,
        );

        // Advanced combinators
        self.register_function(
            "vec_flat_map",
            Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::Unknown)),
                    Type::Named("Closure".to_string()),
                ],
                ret: Box::new(Type::Array(Box::new(Type::Unknown))),
            },
            functional::vec_flat_map_impl,
        );

        self.register_function(
            "vec_zip",
            Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::Unknown)),
                    Type::Array(Box::new(Type::Unknown)),
                ],
                ret: Box::new(Type::Array(Box::new(Type::Unknown))),
            },
            functional::vec_zip_impl,
        );

        self.register_function(
            "vec_chain",
            Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::Unknown)),
                    Type::Array(Box::new(Type::Unknown)),
                ],
                ret: Box::new(Type::Array(Box::new(Type::Unknown))),
            },
            functional::vec_chain_impl,
        );

        self.register_function(
            "vec_take_while",
            Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::Unknown)),
                    Type::Named("Closure".to_string()),
                ],
                ret: Box::new(Type::Array(Box::new(Type::Unknown))),
            },
            functional::vec_take_while_impl,
        );

        self.register_function(
            "vec_drop_while",
            Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::Unknown)),
                    Type::Named("Closure".to_string()),
                ],
                ret: Box::new(Type::Array(Box::new(Type::Unknown))),
            },
            functional::vec_drop_while_impl,
        );

        self.register_function(
            "vec_partition",
            Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::Unknown)),
                    Type::Named("Closure".to_string()),
                ],
                ret: Box::new(Type::Array(Box::new(Type::Unknown))),
            },
            functional::vec_partition_impl,
        );

        self.register_function(
            "vec_group_by",
            Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::Unknown)),
                    Type::Named("Closure".to_string()),
                ],
                ret: Box::new(Type::Array(Box::new(Type::Unknown))),
            },
            functional::vec_group_by_impl,
        );

        // Parallel operations
        self.register_function(
            "vec_parallel_map",
            Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::Unknown)),
                    Type::Named("Closure".to_string()),
                ],
                ret: Box::new(Type::Array(Box::new(Type::Unknown))),
            },
            parallel::vec_parallel_map_impl,
        );

        self.register_function(
            "vec_parallel_filter",
            Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::Unknown)),
                    Type::Named("Closure".to_string()),
                ],
                ret: Box::new(Type::Array(Box::new(Type::Unknown))),
            },
            parallel::vec_parallel_filter_impl,
        );

        self.register_function(
            "vec_parallel_reduce",
            Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::Unknown)),
                    Type::Named("Closure".to_string()),
                    Type::Unknown,
                ],
                ret: Box::new(Type::Unknown),
            },
            parallel::vec_parallel_reduce_impl,
        );

        self.register_function(
            "parallel_config_create",
            Type::Function {
                params: vec![Type::I32, Type::I32, Type::Bool, Type::I32],
                ret: Box::new(Type::Named("Object".to_string())),
            },
            parallel::parallel_config_create_impl,
        );

        // Async operations
        self.register_function(
            "vec_async_map",
            Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::Unknown)),
                    Type::Named("Closure".to_string()),
                ],
                ret: Box::new(Type::Array(Box::new(Type::Unknown))),
            },
            async_functional::vec_async_map_impl,
        );

        self.register_function(
            "vec_async_filter",
            Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::Unknown)),
                    Type::Named("Closure".to_string()),
                ],
                ret: Box::new(Type::Array(Box::new(Type::Unknown))),
            },
            async_functional::vec_async_filter_impl,
        );

        self.register_function(
            "future_join_all",
            Type::Function {
                params: vec![Type::Array(Box::new(Type::Unknown))],
                ret: Box::new(Type::Array(Box::new(Type::Unknown))),
            },
            async_functional::future_join_all_impl,
        );

        self.register_function(
            "future_race",
            Type::Function {
                params: vec![Type::Array(Box::new(Type::Unknown))],
                ret: Box::new(Type::Unknown),
            },
            async_functional::future_race_impl,
        );

        self.register_function(
            "future_timeout",
            Type::Function {
                params: vec![Type::Unknown, Type::I32],
                ret: Box::new(Type::Unknown),
            },
            async_functional::future_timeout_impl,
        );

        // Async generators
        self.register_function(
            "async_generate",
            Type::Function {
                params: vec![Type::Named("Closure".to_string())],
                ret: Box::new(Type::Named("AsyncGenerator".to_string())),
            },
            async_functional::async_generate_impl,
        );
        self.register_function(
            "async_yield",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Unknown),
            },
            async_functional::async_yield_impl,
        );
        self.register_function(
            "async_collect",
            Type::Function {
                params: vec![Type::Named("AsyncGenerator".to_string())],
                ret: Box::new(Type::Array(Box::new(Type::Unknown))),
            },
            async_functional::async_collect_impl,
        );

        // Distributed computing
        self.register_function(
            "remote_execute",
            Type::Function {
                params: vec![
                    Type::String,
                    Type::Named("Closure".to_string()),
                    Type::Array(Box::new(Type::Unknown)),
                ],
                ret: Box::new(Type::Named("Future".to_string())),
            },
            crate::runtime::distributed::remote_execute_impl,
        );
        self.register_function(
            "distribute_map",
            Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::Unknown)),
                    Type::Named("Closure".to_string()),
                ],
                ret: Box::new(Type::Array(Box::new(Type::Unknown))),
            },
            crate::runtime::distributed::distribute_map_impl,
        );
        self.register_function(
            "cluster_reduce",
            Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::Unknown)),
                    Type::Named("Closure".to_string()),
                    Type::Unknown,
                ],
                ret: Box::new(Type::Unknown),
            },
            crate::runtime::distributed::cluster_reduce_impl,
        );

        // Advanced functional utilities
        self.register_function(
            "transduce",
            Type::Function {
                params: vec![
                    Type::Named("Transducer".to_string()),
                    Type::Array(Box::new(Type::Unknown)),
                    Type::Unknown,
                ],
                ret: Box::new(Type::Unknown),
            },
            functional_advanced::transduce_impl,
        );
        self.register_function(
            "lazy_seq",
            Type::Function {
                params: vec![Type::Named("Closure".to_string())],
                ret: Box::new(Type::Named("LazySeq".to_string())),
            },
            functional_advanced::lazy_seq_impl,
        );
        self.register_function(
            "memoize_with_ttl",
            Type::Function {
                params: vec![Type::Named("Closure".to_string()), Type::I32],
                ret: Box::new(Type::Named("Closure".to_string())),
            },
            functional_advanced::memoize_with_ttl_impl,
        );
        self.register_function(
            "lazy_take",
            Type::Function {
                params: vec![Type::Named("LazySeq".to_string()), Type::I32],
                ret: Box::new(Type::Array(Box::new(Type::Unknown))),
            },
            functional_advanced::lazy_take_impl,
        );
        self.register_function(
            "lazy_force",
            Type::Function {
                params: vec![Type::Named("LazySeq".to_string()), Type::I32],
                ret: Box::new(Type::Array(Box::new(Type::Unknown))),
            },
            functional_advanced::lazy_force_impl,
        );

        // Sandboxing
        self.register_function(
            "sandbox_execute",
            Type::Function {
                params: vec![
                    Type::Named("Closure".to_string()),
                    Type::Array(Box::new(Type::Unknown)),
                    Type::Named("Object".to_string()),
                ],
                ret: Box::new(Type::Unknown),
            },
            crate::runtime::sandbox::sandbox_execute_impl,
        );
        self.register_function(
            "sandbox_create",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Named("Sandbox".to_string())),
            },
            crate::runtime::sandbox::sandbox_create_impl,
        );

        // Formal verification
        self.register_function(
            "verify_closure",
            Type::Function {
                params: vec![
                    Type::Named("Closure".to_string()),
                    Type::Named("ClosureSpec".to_string()),
                ],
                ret: Box::new(Type::Named("VerificationResult".to_string())),
            },
            crate::verification::closure_verifier::verify_closure_impl,
        );
        self.register_function(
            "create_spec",
            Type::Function {
                params: vec![Type::Named("Object".to_string())],
                ret: Box::new(Type::Named("ClosureSpec".to_string())),
            },
            crate::verification::closure_verifier::create_spec_impl,
        );

        // Debug functions
        self.register_function(
            "debug_init_closure_debugger",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Named("unit".to_string())),
            },
            functional::debug_init_closure_debugger_impl,
        );

        self.register_function(
            "debug_print_closure",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::Named("unit".to_string())),
            },
            functional::debug_print_closure_impl,
        );

        self.register_function(
            "debug_print_closure_report",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Named("unit".to_string())),
            },
            functional::debug_print_closure_report_impl,
        );

        // Closure serialization functions
        self.register_function(
            "closure_serialize_binary",
            Type::Function {
                params: vec![Type::Named("Closure".to_string())],
                ret: Box::new(Type::Array(Box::new(Type::I32))),
            },
            functional::closure_serialize_binary_impl,
        );
        self.register_function(
            "closure_serialize_json",
            Type::Function {
                params: vec![Type::Named("Closure".to_string())],
                ret: Box::new(Type::String),
            },
            functional::closure_serialize_json_impl,
        );
        self.register_function(
            "closure_serialize_compact",
            Type::Function {
                params: vec![Type::Named("Closure".to_string())],
                ret: Box::new(Type::Array(Box::new(Type::I32))),
            },
            functional::closure_serialize_compact_impl,
        );
        self.register_function(
            "optimized_closure_serialize_binary",
            Type::Function {
                params: vec![Type::Named("OptimizedClosure".to_string())],
                ret: Box::new(Type::Array(Box::new(Type::I32))),
            },
            functional::optimized_closure_serialize_binary_impl,
        );
        self.register_function(
            "closure_get_metadata",
            Type::Function {
                params: vec![Type::Named("Closure".to_string())],
                ret: Box::new(Type::Named("Object".to_string())),
            },
            functional::closure_get_metadata_impl,
        );
        self.register_function(
            "closure_can_serialize",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Bool),
            },
            functional::closure_can_serialize_impl,
        );
        self.register_function(
            "closure_create_serialize_config",
            Type::Function {
                params: vec![Type::Bool, Type::Bool, Type::I32, Type::Bool, Type::Bool],
                ret: Box::new(Type::Named("Object".to_string())),
            },
            functional::closure_create_serialize_config_impl,
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
                params: vec![
                    Type::Named("HashMap".to_string()),
                    Type::String,
                    Type::Unknown,
                ],
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

        // HashSet functions
        self.register_function(
            "HashSet::new",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Named("HashSet".to_string())),
            },
            collections::hashset_new_impl,
        );

        self.register_function(
            "hashset_insert",
            Type::Function {
                params: vec![Type::Named("HashSet".to_string()), Type::Unknown],
                ret: Box::new(Type::Bool),
            },
            collections::hashset_insert_impl,
        );

        self.register_function(
            "hashset_contains",
            Type::Function {
                params: vec![Type::Named("HashSet".to_string()), Type::Unknown],
                ret: Box::new(Type::Bool),
            },
            collections::hashset_contains_impl,
        );

        self.register_function(
            "hashset_remove",
            Type::Function {
                params: vec![Type::Named("HashSet".to_string()), Type::Unknown],
                ret: Box::new(Type::Bool),
            },
            collections::hashset_remove_impl,
        );

        self.register_function(
            "hashset_len",
            Type::Function {
                params: vec![Type::Named("HashSet".to_string())],
                ret: Box::new(Type::I32),
            },
            collections::hashset_len_impl,
        );

        self.register_function(
            "hashset_is_empty",
            Type::Function {
                params: vec![Type::Named("HashSet".to_string())],
                ret: Box::new(Type::Bool),
            },
            collections::hashset_is_empty_impl,
        );

        self.register_function(
            "hashset_union",
            Type::Function {
                params: vec![
                    Type::Named("HashSet".to_string()),
                    Type::Named("HashSet".to_string()),
                ],
                ret: Box::new(Type::Named("HashSet".to_string())),
            },
            collections::hashset_union_impl,
        );

        self.register_function(
            "hashset_intersection",
            Type::Function {
                params: vec![
                    Type::Named("HashSet".to_string()),
                    Type::Named("HashSet".to_string()),
                ],
                ret: Box::new(Type::Named("HashSet".to_string())),
            },
            collections::hashset_intersection_impl,
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
                params: vec![
                    Type::Named("Object".to_string()),
                    Type::Named("Object".to_string()),
                ],
                ret: Box::new(Type::Named("Object".to_string())),
            },
            game::vec2_add,
        );

        self.register_function(
            "vec2_dot",
            Type::Function {
                params: vec![
                    Type::Named("Object".to_string()),
                    Type::Named("Object".to_string()),
                ],
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

    /// Register network functions
    fn register_network_functions(&mut self) {
        // TCP functions
        self.register_function(
            "tcp_connect",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::Named("Result".to_string())),
            },
            network::tcp_connect_impl,
        );

        self.register_function(
            "tcp_bind",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::Named("Result".to_string())),
            },
            network::tcp_bind_impl,
        );

        // UDP functions
        self.register_function(
            "udp_bind",
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::Named("Result".to_string())),
            },
            network::udp_bind_impl,
        );

        // Note: Additional network operations like tcp_read, tcp_write, tcp_accept,
        // udp_send, udp_recv, etc. would need implementation functions that handle
        // the network object handles properly. This is a simplified initial implementation.
    }

    /// Register random number generation functions
    fn register_random_functions(&mut self) {
        // Basic random functions
        self.register_function(
            "random",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::F32),
            },
            random::random_impl,
        );

        self.register_function(
            "random_range",
            Type::Function {
                params: vec![Type::F32, Type::F32],
                ret: Box::new(Type::F32),
            },
            random::random_range_impl,
        );

        self.register_function(
            "random_int",
            Type::Function {
                params: vec![Type::I32, Type::I32],
                ret: Box::new(Type::I32),
            },
            random::random_int_impl,
        );

        self.register_function(
            "random_bool",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::Bool),
            },
            random::random_bool_impl,
        );

        self.register_function(
            "random_seed",
            Type::Function {
                params: vec![Type::I32],
                ret: Box::new(Type::String),
            },
            random::random_seed_impl,
        );

        // Array shuffle
        self.register_function(
            "shuffle",
            Type::Function {
                params: vec![Type::Array(Box::new(Type::Unknown))],
                ret: Box::new(Type::Array(Box::new(Type::Unknown))),
            },
            random::shuffle_impl,
        );

        self.register_function(
            "pick_random",
            Type::Function {
                params: vec![Type::Array(Box::new(Type::Unknown))],
                ret: Box::new(Type::Unknown),
            },
            random::pick_random_impl,
        );

        // Vector generation
        self.register_function(
            "random_unit_vec2",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Named("Object".to_string())),
            },
            random::random_unit_vec2_impl,
        );

        self.register_function(
            "random_unit_vec3",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Named("Object".to_string())),
            },
            random::random_unit_vec3_impl,
        );

        self.register_function(
            "random_in_circle",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::Named("Object".to_string())),
            },
            random::random_in_circle_impl,
        );

        // Weighted random
        self.register_function(
            "weighted_random",
            Type::Function {
                params: vec![Type::Array(Box::new(Type::F32))],
                ret: Box::new(Type::I32),
            },
            random::weighted_random_impl,
        );
    }

    /// Register time-related functions
    fn register_time_functions(&mut self) {
        // Basic time functions
        self.register_function(
            "time_now",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::F32),
            },
            time::time_now_impl,
        );

        self.register_function(
            "time_now_millis",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::F32),
            },
            time::time_now_millis_impl,
        );

        self.register_function(
            "time_unix",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::F32),
            },
            time::time_unix_impl,
        );

        self.register_function(
            "time_unix_millis",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::F32),
            },
            time::time_unix_millis_impl,
        );

        self.register_function(
            "time_delta",
            Type::Function {
                params: vec![Type::F32, Type::F32],
                ret: Box::new(Type::F32),
            },
            time::time_delta_impl,
        );

        self.register_function(
            "sleep",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::Named("unit".to_string())),
            },
            time::sleep_impl,
        );

        // Stopwatch functions
        self.register_function(
            "stopwatch_new",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Named("Object".to_string())),
            },
            time::stopwatch_new_impl,
        );

        self.register_function(
            "stopwatch_start",
            Type::Function {
                params: vec![Type::Named("Object".to_string())],
                ret: Box::new(Type::Named("unit".to_string())),
            },
            time::stopwatch_start_impl,
        );

        self.register_function(
            "stopwatch_stop",
            Type::Function {
                params: vec![Type::Named("Object".to_string())],
                ret: Box::new(Type::Named("unit".to_string())),
            },
            time::stopwatch_stop_impl,
        );

        self.register_function(
            "stopwatch_reset",
            Type::Function {
                params: vec![Type::Named("Object".to_string())],
                ret: Box::new(Type::Named("unit".to_string())),
            },
            time::stopwatch_reset_impl,
        );

        self.register_function(
            "stopwatch_elapsed",
            Type::Function {
                params: vec![Type::Named("Object".to_string())],
                ret: Box::new(Type::F32),
            },
            time::stopwatch_elapsed_impl,
        );

        // Frame timer functions
        self.register_function(
            "frame_timer_new",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Named("Object".to_string())),
            },
            time::frame_timer_new_impl,
        );

        self.register_function(
            "frame_timer_update",
            Type::Function {
                params: vec![Type::Named("Object".to_string())],
                ret: Box::new(Type::Named("unit".to_string())),
            },
            time::frame_timer_update_impl,
        );

        self.register_function(
            "frame_timer_delta",
            Type::Function {
                params: vec![Type::Named("Object".to_string())],
                ret: Box::new(Type::F32),
            },
            time::frame_timer_delta_impl,
        );

        self.register_function(
            "frame_timer_fps",
            Type::Function {
                params: vec![Type::Named("Object".to_string())],
                ret: Box::new(Type::F32),
            },
            time::frame_timer_fps_impl,
        );

        // Time formatting
        self.register_function(
            "format_time",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::String),
            },
            time::format_time_impl,
        );

        self.register_function(
            "format_time_millis",
            Type::Function {
                params: vec![Type::F32],
                ret: Box::new(Type::String),
            },
            time::format_time_millis_impl,
        );

        // Performance measurement
        self.register_function(
            "perf_counter",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::F32),
            },
            time::perf_counter_impl,
        );

        self.register_function(
            "measure_time",
            Type::Function {
                params: vec![Type::Unknown], // Function type
                ret: Box::new(Type::F32),
            },
            time::measure_time_impl,
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
