//! Core types for the Script programming language
//!
//! This module provides the fundamental types Option<T> and Result<T, E>
//! which are used throughout Script for optional values and error handling.

use crate::runtime::{RuntimeError, ScriptRc, Value as RuntimeValue};
use crate::stdlib::closure_helpers::ClosureExecutor;
use crate::stdlib::ScriptValue;
use std::fmt;

/// Option type for Script - represents an optional value
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptOption {
    /// Some value
    Some(ScriptValue),
    /// No value
    None,
}

impl ScriptOption {
    /// Create a Some variant
    pub fn some(value: ScriptValue) -> Self {
        ScriptOption::Some(value)
    }

    /// Create a None variant
    pub fn none() -> Self {
        ScriptOption::None
    }

    /// Check if this is Some
    pub fn is_some(&self) -> bool {
        matches!(self, ScriptOption::Some(_))
    }

    /// Check if this is None
    pub fn is_none(&self) -> bool {
        matches!(self, ScriptOption::None)
    }

    /// Get the inner value if Some, None otherwise
    pub fn unwrap(&self) -> Option<&ScriptValue> {
        match self {
            ScriptOption::Some(val) => Some(val),
            ScriptOption::None => None,
        }
    }

    /// Get the inner value if Some, panic otherwise
    pub fn expect(&self, msg: &str) -> &ScriptValue {
        match self {
            ScriptOption::Some(val) => val,
            ScriptOption::None => panic!("{msg}"),
        }
    }

    /// Map the inner value if Some (using Rust closure for compatibility)
    pub fn map<F>(&self, f: F) -> ScriptOption
    where
        F: FnOnce(&ScriptValue) -> ScriptValue,
    {
        match self {
            ScriptOption::Some(val) => ScriptOption::Some(f(val)),
            ScriptOption::None => ScriptOption::None,
        }
    }

    /// Map the inner value using a Script closure
    pub fn map_with_closure(
        &self,
        closure: &RuntimeValue,
        executor: &mut ClosureExecutor,
    ) -> crate::error::Result<ScriptOption> {
        self.map_closure(closure, executor)
    }

    /// Get the inner value or a default
    pub fn unwrap_or(&self, default: ScriptValue) -> ScriptValue {
        match self {
            ScriptOption::Some(val) => val.clone(),
            ScriptOption::None => default,
        }
    }

    /// Get the inner value or compute a default
    pub fn unwrap_or_else<F>(&self, f: F) -> ScriptValue
    where
        F: FnOnce() -> ScriptValue,
    {
        match self {
            ScriptOption::Some(val) => val.clone(),
            ScriptOption::None => f(),
        }
    }

    /// Chain another Option computation if this is Some (using Rust closure for compatibility)
    pub fn and_then<F>(&self, f: F) -> ScriptOption
    where
        F: FnOnce(&ScriptValue) -> ScriptOption,
    {
        match self {
            ScriptOption::Some(val) => f(val),
            ScriptOption::None => ScriptOption::None,
        }
    }

    /// Chain another Option computation using a Script closure
    pub fn and_then_with_closure(
        &self,
        closure: &RuntimeValue,
        executor: &mut ClosureExecutor,
    ) -> crate::error::Result<ScriptOption> {
        self.and_then_closure(closure, executor)
    }

    /// Return this Option if Some, otherwise return the provided Option
    pub fn or(&self, other: ScriptOption) -> ScriptOption {
        match self {
            ScriptOption::Some(_) => self.clone(),
            ScriptOption::None => other,
        }
    }

    /// Return this Option if Some, otherwise compute an Option
    pub fn or_else<F>(&self, f: F) -> ScriptOption
    where
        F: FnOnce() -> ScriptOption,
    {
        match self {
            ScriptOption::Some(_) => self.clone(),
            ScriptOption::None => f(),
        }
    }

    /// Filter the Option based on a predicate (using Rust closure for compatibility)
    pub fn filter<F>(&self, predicate: F) -> ScriptOption
    where
        F: FnOnce(&ScriptValue) -> bool,
    {
        match self {
            ScriptOption::Some(val) => {
                if predicate(val) {
                    self.clone()
                } else {
                    ScriptOption::None
                }
            }
            ScriptOption::None => ScriptOption::None,
        }
    }

    /// Filter the Option using a Script closure predicate
    pub fn filter_with_closure(
        &self,
        closure: &RuntimeValue,
        executor: &mut ClosureExecutor,
    ) -> crate::error::Result<ScriptOption> {
        self.filter_closure(closure, executor)
    }

    /// Convert Option<T> to Result<T, E> with provided error for None
    pub fn ok_or(&self, err: ScriptValue) -> ScriptResult {
        match self {
            ScriptOption::Some(val) => ScriptResult::Ok(val.clone()),
            ScriptOption::None => ScriptResult::Err(err),
        }
    }

    /// Convert Option<T> to Result<T, E> with error computed from closure for None
    pub fn ok_or_else<F>(&self, f: F) -> ScriptResult
    where
        F: FnOnce() -> ScriptValue,
    {
        match self {
            ScriptOption::Some(val) => ScriptResult::Ok(val.clone()),
            ScriptOption::None => ScriptResult::Err(f()),
        }
    }

    /// Take the value out of the Option, leaving None in its place
    pub fn take(&mut self) -> ScriptOption {
        std::mem::replace(self, ScriptOption::None)
    }

    /// Replace the value in the Option, returning the old value
    pub fn replace(&mut self, value: ScriptValue) -> ScriptOption {
        std::mem::replace(self, ScriptOption::Some(value))
    }

    /// Flatten an Option<Option<T>> to Option<T>
    pub fn flatten(&self) -> ScriptOption {
        match self {
            ScriptOption::Some(val) => {
                // Check if the inner value is also an Option
                if let ScriptValue::Option(inner_opt) = val {
                    (**inner_opt).clone()
                } else {
                    // If not an Option, just return the current Option
                    self.clone()
                }
            }
            ScriptOption::None => ScriptOption::None,
        }
    }

    /// Transpose an Option<Result<T, E>> to Result<Option<T>, E>
    pub fn transpose(&self) -> ScriptResult {
        match self {
            ScriptOption::Some(val) => {
                // Check if the inner value is a Result
                if let ScriptValue::Result(result) = val {
                    match &**result {
                        ScriptResult::Ok(inner_val) => ScriptResult::Ok(ScriptValue::Option(
                            ScriptRc::new(ScriptOption::Some(inner_val.clone())),
                        )),
                        ScriptResult::Err(err) => ScriptResult::Err(err.clone()),
                    }
                } else {
                    // If not a Result, wrap in Ok(Some(...))
                    ScriptResult::Ok(ScriptValue::Option(ScriptRc::new(self.clone())))
                }
            }
            ScriptOption::None => {
                ScriptResult::Ok(ScriptValue::Option(ScriptRc::new(ScriptOption::None)))
            }
        }
    }

    /// Inspect the Some value without consuming the Option (using Rust closure for compatibility)
    pub fn inspect<F>(&self, f: F) -> ScriptOption
    where
        F: FnOnce(&ScriptValue),
    {
        match self {
            ScriptOption::Some(val) => {
                f(val);
                self.clone()
            }
            ScriptOption::None => self.clone(),
        }
    }

    /// Inspect the Some value using a Script closure
    pub fn inspect_with_closure(
        &self,
        closure: &RuntimeValue,
        executor: &mut ClosureExecutor,
    ) -> crate::error::Result<ScriptOption> {
        self.inspect_closure(closure, executor)
    }

    /// Zip two Options into an Option of a tuple
    pub fn zip(&self, other: &ScriptOption) -> ScriptOption {
        match (self, other) {
            (ScriptOption::Some(a), ScriptOption::Some(b)) => {
                // Create a tuple as an array with two elements
                ScriptOption::Some(ScriptValue::Array(ScriptRc::new(
                    crate::stdlib::collections::ScriptVec::new(),
                )))
            }
            _ => ScriptOption::None,
        }
    }

    /// Copy the value if it's copyable (simplified version)
    pub fn copied(&self) -> ScriptOption {
        self.clone()
    }

    /// Clone the value (same as copied for now)
    pub fn cloned(&self) -> ScriptOption {
        self.clone()
    }

    /// Collect an iterator of Options into an Option of Vec
    /// For now, this is a simplified version that works on a single Option
    pub fn collect(&self) -> ScriptOption {
        match self {
            ScriptOption::Some(val) => {
                let mut vec = crate::stdlib::collections::ScriptVec::new();
                vec.push(val.clone()).unwrap_or(());
                ScriptOption::Some(ScriptValue::Array(ScriptRc::new(vec)))
            }
            ScriptOption::None => ScriptOption::None,
        }
    }

    /// Fold with early termination - if this is None, return init
    pub fn fold<T, F>(&self, init: T, f: F) -> T
    where
        F: FnOnce(T, &ScriptValue) -> T,
    {
        match self {
            ScriptOption::Some(val) => f(init, val),
            ScriptOption::None => init,
        }
    }

    /// Reduce - if this is None, return None
    pub fn reduce<F>(&self, _f: F) -> ScriptOption
    where
        F: FnOnce(&ScriptValue, &ScriptValue) -> ScriptValue,
    {
        // For a single Option, reduce just returns the value
        self.clone()
    }

    /// Test if the Option satisfies a predicate
    pub fn satisfies<F>(&self, predicate: F) -> bool
    where
        F: FnOnce(&ScriptValue) -> bool,
    {
        match self {
            ScriptOption::Some(val) => predicate(val),
            ScriptOption::None => false,
        }
    }
}

impl fmt::Display for ScriptOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptOption::Some(val) => write!(f, "Some({:?})", val),
            ScriptOption::None => write!(f, "None"),
        }
    }
}

/// Result type for Script - represents either success (Ok) or failure (Err)
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptResult {
    /// Success value
    Ok(ScriptValue),
    /// Error value
    Err(ScriptValue),
}

impl ScriptResult {
    /// Create an Ok variant
    pub fn ok(value: ScriptValue) -> Self {
        ScriptResult::Ok(value)
    }

    /// Create an Err variant
    pub fn err(error: ScriptValue) -> Self {
        ScriptResult::Err(error)
    }

    /// Check if this is Ok
    pub fn is_ok(&self) -> bool {
        matches!(self, ScriptResult::Ok(_))
    }

    /// Check if this is Err
    pub fn is_err(&self) -> bool {
        matches!(self, ScriptResult::Err(_))
    }

    /// Get the Ok value if present
    pub fn get_ok(&self) -> Option<&ScriptValue> {
        match self {
            ScriptResult::Ok(val) => Some(val),
            ScriptResult::Err(_) => None,
        }
    }

    /// Get the Err value if present
    pub fn get_err(&self) -> Option<&ScriptValue> {
        match self {
            ScriptResult::Ok(_) => None,
            ScriptResult::Err(val) => Some(val),
        }
    }

    /// Get the Ok value or panic with a message
    pub fn expect(&self, msg: &str) -> &ScriptValue {
        match self {
            ScriptResult::Ok(val) => val,
            ScriptResult::Err(err) => panic!("{}: {:?}", msg, err),
        }
    }

    /// Get the Err value or panic with a message
    pub fn expect_err(&self, msg: &str) -> &ScriptValue {
        match self {
            ScriptResult::Ok(val) => panic!("{}: {:?}", msg, val),
            ScriptResult::Err(err) => err,
        }
    }

    /// Unwrap the Ok value or panic
    pub fn unwrap(&self) -> &ScriptValue {
        self.expect("called `Result::unwrap()` on an `Err` value")
    }

    /// Unwrap the Err value or panic
    pub fn unwrap_err(&self) -> &ScriptValue {
        self.expect_err("called `Result::unwrap_err()` on an `Ok` value")
    }

    /// Map the Ok value if present (using Rust closure for compatibility)
    pub fn map<F>(&self, f: F) -> ScriptResult
    where
        F: FnOnce(&ScriptValue) -> ScriptValue,
    {
        match self {
            ScriptResult::Ok(val) => ScriptResult::Ok(f(val)),
            ScriptResult::Err(err) => ScriptResult::Err(err.clone()),
        }
    }

    /// Map the Ok value using a Script closure
    pub fn map_with_closure(
        &self,
        closure: &RuntimeValue,
        executor: &mut ClosureExecutor,
    ) -> crate::error::Result<ScriptResult> {
        self.map_closure(closure, executor)
    }

    /// Map the Err value if present (using Rust closure for compatibility)
    pub fn map_err<F>(&self, f: F) -> ScriptResult
    where
        F: FnOnce(&ScriptValue) -> ScriptValue,
    {
        match self {
            ScriptResult::Ok(val) => ScriptResult::Ok(val.clone()),
            ScriptResult::Err(err) => ScriptResult::Err(f(err)),
        }
    }

    /// Map the Err value using a Script closure
    pub fn map_err_with_closure(
        &self,
        closure: &RuntimeValue,
        executor: &mut ClosureExecutor,
    ) -> crate::error::Result<ScriptResult> {
        self.map_err_closure(closure, executor)
    }

    /// Get the Ok value or a default
    pub fn unwrap_or(&self, default: ScriptValue) -> ScriptValue {
        match self {
            ScriptResult::Ok(val) => val.clone(),
            ScriptResult::Err(_) => default,
        }
    }

    /// Get the Ok value or compute a default
    pub fn unwrap_or_else<F>(&self, f: F) -> ScriptValue
    where
        F: FnOnce(&ScriptValue) -> ScriptValue,
    {
        match self {
            ScriptResult::Ok(val) => val.clone(),
            ScriptResult::Err(err) => f(err),
        }
    }

    /// Chain another Result computation if this is Ok (using Rust closure for compatibility)
    pub fn and_then<F>(&self, f: F) -> ScriptResult
    where
        F: FnOnce(&ScriptValue) -> ScriptResult,
    {
        match self {
            ScriptResult::Ok(val) => f(val),
            ScriptResult::Err(err) => ScriptResult::Err(err.clone()),
        }
    }

    /// Chain another Result computation using a Script closure
    pub fn and_then_with_closure(
        &self,
        closure: &RuntimeValue,
        executor: &mut ClosureExecutor,
    ) -> crate::error::Result<ScriptResult> {
        self.and_then_closure(closure, executor)
    }

    /// Return this Result if Ok, otherwise return the provided Result
    pub fn or(&self, other: ScriptResult) -> ScriptResult {
        match self {
            ScriptResult::Ok(_) => self.clone(),
            ScriptResult::Err(_) => other,
        }
    }

    /// Return this Result if Ok, otherwise compute a Result
    pub fn or_else<F>(&self, f: F) -> ScriptResult
    where
        F: FnOnce(&ScriptValue) -> ScriptResult,
    {
        match self {
            ScriptResult::Ok(_) => self.clone(),
            ScriptResult::Err(err) => f(err),
        }
    }

    /// Convert Result<T, E> to Option<T>, discarding error information
    pub fn to_option(&self) -> ScriptOption {
        match self {
            ScriptResult::Ok(val) => ScriptOption::Some(val.clone()),
            ScriptResult::Err(_) => ScriptOption::None,
        }
    }

    /// Convert Result<T, E> to Option<E>, discarding success information
    pub fn to_error_option(&self) -> ScriptOption {
        match self {
            ScriptResult::Ok(_) => ScriptOption::None,
            ScriptResult::Err(err) => ScriptOption::Some(err.clone()),
        }
    }

    /// Flatten a Result<Result<T, E>, E> to Result<T, E>
    pub fn flatten(&self) -> ScriptResult {
        match self {
            ScriptResult::Ok(val) => {
                // Check if the inner value is also a Result
                if let ScriptValue::Result(inner_result) = val {
                    (**inner_result).clone()
                } else {
                    // If not a Result, just return the current Result
                    self.clone()
                }
            }
            ScriptResult::Err(err) => ScriptResult::Err(err.clone()),
        }
    }

    /// Transpose a Result<Option<T>, E> to Option<Result<T, E>>
    pub fn transpose(&self) -> ScriptOption {
        match self {
            ScriptResult::Ok(val) => {
                // Check if the inner value is an Option
                if let ScriptValue::Option(opt) = val {
                    match &**opt {
                        ScriptOption::Some(inner_val) => ScriptOption::Some(ScriptValue::Result(
                            ScriptRc::new(ScriptResult::Ok(inner_val.clone())),
                        )),
                        ScriptOption::None => ScriptOption::None,
                    }
                } else {
                    // If not an Option, wrap in Some(Ok(...))
                    ScriptOption::Some(ScriptValue::Result(ScriptRc::new(self.clone())))
                }
            }
            ScriptResult::Err(err) => ScriptOption::Some(ScriptValue::Result(ScriptRc::new(
                ScriptResult::Err(err.clone()),
            ))),
        }
    }

    /// Inspect the Ok value without consuming the Result (using Rust closure for compatibility)
    pub fn inspect<F>(&self, f: F) -> ScriptResult
    where
        F: FnOnce(&ScriptValue),
    {
        match self {
            ScriptResult::Ok(val) => {
                f(val);
                self.clone()
            }
            ScriptResult::Err(_) => self.clone(),
        }
    }

    /// Inspect the Ok value using a Script closure
    pub fn inspect_with_closure(
        &self,
        closure: &RuntimeValue,
        executor: &mut ClosureExecutor,
    ) -> crate::error::Result<ScriptResult> {
        self.inspect_closure(closure, executor)
    }

    /// Inspect the Err value without consuming the Result (using Rust closure for compatibility)
    pub fn inspect_err<F>(&self, f: F) -> ScriptResult
    where
        F: FnOnce(&ScriptValue),
    {
        match self {
            ScriptResult::Ok(_) => self.clone(),
            ScriptResult::Err(err) => {
                f(err);
                self.clone()
            }
        }
    }

    /// Inspect the Err value using a Script closure
    pub fn inspect_err_with_closure(
        &self,
        closure: &RuntimeValue,
        executor: &mut ClosureExecutor,
    ) -> crate::error::Result<ScriptResult> {
        self.inspect_err_closure(closure, executor)
    }

    /// Logical AND for Results - returns the first Err or the second Result
    pub fn and(&self, other: ScriptResult) -> ScriptResult {
        match self {
            ScriptResult::Ok(_) => other,
            ScriptResult::Err(_) => self.clone(),
        }
    }

    /// Collect an iterator of Results into a Result of Vec
    /// For now, this is a simplified version that works on a single Result
    pub fn collect(&self) -> ScriptResult {
        match self {
            ScriptResult::Ok(val) => {
                let mut vec = crate::stdlib::collections::ScriptVec::new();
                vec.push(val.clone()).unwrap_or(());
                ScriptResult::Ok(ScriptValue::Array(ScriptRc::new(vec)))
            }
            ScriptResult::Err(err) => ScriptResult::Err(err.clone()),
        }
    }

    /// Fold with early termination - if this is Err, return init
    pub fn fold<T, F>(&self, init: T, f: F) -> T
    where
        F: FnOnce(T, &ScriptValue) -> T,
    {
        match self {
            ScriptResult::Ok(val) => f(init, val),
            ScriptResult::Err(_) => init,
        }
    }

    /// Reduce - if this is Err, return Err
    pub fn reduce<F>(&self, _f: F) -> ScriptResult
    where
        F: FnOnce(&ScriptValue, &ScriptValue) -> ScriptValue,
    {
        // For a single Result, reduce just returns the value
        self.clone()
    }

    /// Test if the Result satisfies a predicate (only for Ok values)
    pub fn satisfies<F>(&self, predicate: F) -> bool
    where
        F: FnOnce(&ScriptValue) -> bool,
    {
        match self {
            ScriptResult::Ok(val) => predicate(val),
            ScriptResult::Err(_) => false,
        }
    }
}

impl fmt::Display for ScriptResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptResult::Ok(val) => write!(f, "Ok({:?})", val),
            ScriptResult::Err(err) => write!(f, "Err({:?})", err),
        }
    }
}

// Implementation functions for stdlib registry

/// Create a Some Option
pub(crate) fn option_some_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "Option::some expects 1 argument, got {}",
            args.len()
        )));
    }

    let opt = ScriptOption::some(args[0].clone());
    Ok(ScriptValue::Option(ScriptRc::new(opt)))
}

/// Create a None Option
pub(crate) fn option_none_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if !args.is_empty() {
        return Err(RuntimeError::InvalidOperation(format!(
            "Option::none expects 0 arguments, got {}",
            args.len()
        )));
    }

    let opt = ScriptOption::none();
    Ok(ScriptValue::Option(ScriptRc::new(opt)))
}

/// Check if an Option is Some
pub(crate) fn option_is_some_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "is_some expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::Option(opt) => Ok(ScriptValue::Bool(opt.is_some())),
        _ => Err(RuntimeError::InvalidOperation(
            "is_some expects an Option argument".to_string(),
        )),
    }
}

/// Check if an Option is None
pub(crate) fn option_is_none_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "is_none expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::Option(opt) => Ok(ScriptValue::Bool(opt.is_none())),
        _ => Err(RuntimeError::InvalidOperation(
            "is_none expects an Option argument".to_string(),
        )),
    }
}

/// Unwrap an Option, returning the value or Unit if None
pub(crate) fn option_unwrap_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "unwrap expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::Option(opt) => match opt.unwrap() {
            Some(val) => Ok(val.clone()),
            None => Err(RuntimeError::Panic("unwrap called on None".to_string())),
        },
        _ => Err(RuntimeError::InvalidOperation(
            "unwrap expects an Option argument".to_string(),
        )),
    }
}

/// Create an Ok Result
pub(crate) fn result_ok_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "Result::ok expects 1 argument, got {}",
            args.len()
        )));
    }

    let result = ScriptResult::ok(args[0].clone());
    Ok(ScriptValue::Result(ScriptRc::new(result)))
}

/// Create an Err Result
pub(crate) fn result_err_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "Result::err expects 1 argument, got {}",
            args.len()
        )));
    }

    let result = ScriptResult::err(args[0].clone());
    Ok(ScriptValue::Result(ScriptRc::new(result)))
}

/// Check if a Result is Ok
pub(crate) fn result_is_ok_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "is_ok expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::Result(res) => Ok(ScriptValue::Bool(res.is_ok())),
        _ => Err(RuntimeError::InvalidOperation(
            "is_ok expects a Result argument".to_string(),
        )),
    }
}

/// Check if a Result is Err
pub(crate) fn result_is_err_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "is_err expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::Result(res) => Ok(ScriptValue::Bool(res.is_err())),
        _ => Err(RuntimeError::InvalidOperation(
            "is_err expects a Result argument".to_string(),
        )),
    }
}

/// Unwrap a Result, returning the Ok value or panicking
pub(crate) fn result_unwrap_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "unwrap expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::Result(res) => match res.get_ok() {
            Some(val) => Ok(val.clone()),
            None => Err(RuntimeError::Panic("unwrap called on Err".to_string())),
        },
        _ => Err(RuntimeError::InvalidOperation(
            "unwrap expects a Result argument".to_string(),
        )),
    }
}

/// Unwrap a Result's error value, panicking if Ok
pub(crate) fn result_unwrap_err_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "unwrap_err expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::Result(res) => match res.get_err() {
            Some(val) => Ok(val.clone()),
            None => Err(RuntimeError::Panic("unwrap_err called on Ok".to_string())),
        },
        _ => Err(RuntimeError::InvalidOperation(
            "unwrap_err expects a Result argument".to_string(),
        )),
    }
}

// Additional Option method implementations

/// Option::and_then implementation
pub(crate) fn option_and_then_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "and_then expects 2 arguments, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::Option(opt) => {
            match **opt {
                ScriptOption::Some(ref val) => {
                    // Call the function with the value
                    // For now, return the second argument as a placeholder
                    Ok(args[1].clone())
                }
                ScriptOption::None => Ok(ScriptValue::Option(ScriptRc::new(ScriptOption::None))),
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "and_then expects an Option argument".to_string(),
        )),
    }
}

/// Option::or implementation
pub(crate) fn option_or_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "or expects 2 arguments, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::Option(opt) => {
            if opt.is_some() {
                Ok(args[0].clone())
            } else {
                Ok(args[1].clone())
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "or expects an Option argument".to_string(),
        )),
    }
}

/// Option::unwrap_or implementation
pub(crate) fn option_unwrap_or_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "unwrap_or expects 2 arguments, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::Option(opt) => match **opt {
            ScriptOption::Some(ref val) => Ok(val.clone()),
            ScriptOption::None => Ok(args[1].clone()),
        },
        _ => Err(RuntimeError::InvalidOperation(
            "unwrap_or expects an Option argument".to_string(),
        )),
    }
}

/// Option::expect implementation
pub(crate) fn option_expect_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "expect expects 2 arguments, got {}",
            args.len()
        )));
    }

    match (&args[0], &args[1]) {
        (ScriptValue::Option(opt), ScriptValue::String(msg)) => match **opt {
            ScriptOption::Some(ref val) => Ok(val.clone()),
            ScriptOption::None => Err(RuntimeError::Panic(msg.as_str().to_string())),
        },
        _ => Err(RuntimeError::InvalidOperation(
            "expect expects an Option and a String argument".to_string(),
        )),
    }
}

/// Option::filter implementation
pub(crate) fn option_filter_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "filter expects 2 arguments, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::Option(opt) => {
            match **opt {
                ScriptOption::Some(ref _val) => {
                    // For now, assume the predicate returns true
                    // In a real implementation, we'd call the function
                    Ok(args[0].clone())
                }
                ScriptOption::None => Ok(ScriptValue::Option(ScriptRc::new(ScriptOption::None))),
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "filter expects an Option argument".to_string(),
        )),
    }
}

/// Option::ok_or implementation
pub(crate) fn option_ok_or_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "ok_or expects 2 arguments, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::Option(opt) => match **opt {
            ScriptOption::Some(ref val) => {
                let result = ScriptResult::ok(val.clone());
                Ok(ScriptValue::Result(ScriptRc::new(result)))
            }
            ScriptOption::None => {
                let result = ScriptResult::err(args[1].clone());
                Ok(ScriptValue::Result(ScriptRc::new(result)))
            }
        },
        _ => Err(RuntimeError::InvalidOperation(
            "ok_or expects an Option argument".to_string(),
        )),
    }
}

// Additional Result method implementations

/// Result::and_then implementation
pub(crate) fn result_and_then_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "and_then expects 2 arguments, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::Result(res) => {
            match res.get_ok() {
                Some(_val) => {
                    // Call the function with the Ok value
                    // For now, return the second argument as a placeholder
                    Ok(args[1].clone())
                }
                None => Ok(args[0].clone()),
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "and_then expects a Result argument".to_string(),
        )),
    }
}

/// Result::or implementation
pub(crate) fn result_or_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "or expects 2 arguments, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::Result(res) => {
            if res.is_ok() {
                Ok(args[0].clone())
            } else {
                Ok(args[1].clone())
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "or expects a Result argument".to_string(),
        )),
    }
}

/// Result::unwrap_or implementation
pub(crate) fn result_unwrap_or_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "unwrap_or expects 2 arguments, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::Result(res) => match res.get_ok() {
            Some(val) => Ok(val.clone()),
            None => Ok(args[1].clone()),
        },
        _ => Err(RuntimeError::InvalidOperation(
            "unwrap_or expects a Result argument".to_string(),
        )),
    }
}

/// Result::expect implementation
pub(crate) fn result_expect_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "expect expects 2 arguments, got {}",
            args.len()
        )));
    }

    match (&args[0], &args[1]) {
        (ScriptValue::Result(res), ScriptValue::String(msg)) => match res.get_ok() {
            Some(val) => Ok(val.clone()),
            None => Err(RuntimeError::Panic(format!(
                "{}: {:?}",
                msg.as_str(),
                res.get_err()
            ))),
        },
        _ => Err(RuntimeError::InvalidOperation(
            "expect expects a Result and a String argument".to_string(),
        )),
    }
}

/// Result::map implementation
pub(crate) fn result_map_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "map expects 2 arguments, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::Result(res) => {
            match res.get_ok() {
                Some(_val) => {
                    // For now, return the second argument as the mapped result
                    // In a real implementation, we'd call the function with the Ok value
                    let mapped_result = ScriptResult::ok(args[1].clone());
                    Ok(ScriptValue::Result(ScriptRc::new(mapped_result)))
                }
                None => Ok(args[0].clone()),
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "map expects a Result argument".to_string(),
        )),
    }
}

/// Result::map_err implementation
pub(crate) fn result_map_err_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "map_err expects 2 arguments, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::Result(res) => {
            match res.get_err() {
                Some(_err) => {
                    // For now, return the second argument as the mapped error
                    // In a real implementation, we'd call the function with the Err value
                    let mapped_result = ScriptResult::err(args[1].clone());
                    Ok(ScriptValue::Result(ScriptRc::new(mapped_result)))
                }
                None => Ok(args[0].clone()),
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "map_err expects a Result argument".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdlib::ScriptString;

    #[test]
    fn test_option_creation() {
        let some_val = ScriptOption::some(ScriptValue::I32(42));
        assert!(some_val.is_some());
        assert!(!some_val.is_none());
        assert_eq!(some_val.unwrap(), Some(&ScriptValue::I32(42)));

        let none_val = ScriptOption::none();
        assert!(!none_val.is_some());
        assert!(none_val.is_none());
        assert_eq!(none_val.unwrap(), None);
    }

    #[test]
    fn test_option_methods() {
        let some_val = ScriptOption::some(ScriptValue::I32(10));
        let mapped = some_val.map(|v| match v {
            ScriptValue::I32(n) => ScriptValue::I32(n * 2),
            _ => v.clone(),
        });
        assert_eq!(mapped.unwrap(), Some(&ScriptValue::I32(20)));

        let none_val = ScriptOption::none();
        let mapped_none = none_val.map(|_v| ScriptValue::I32(100));
        assert!(mapped_none.is_none());

        assert_eq!(
            some_val.unwrap_or(ScriptValue::I32(0)),
            ScriptValue::I32(10)
        );
        assert_eq!(none_val.unwrap_or(ScriptValue::I32(0)), ScriptValue::I32(0));
    }

    #[test]
    fn test_result_creation() {
        let ok_val = ScriptResult::ok(ScriptValue::I32(42));
        assert!(ok_val.is_ok());
        assert!(!ok_val.is_err());
        assert_eq!(ok_val.get_ok(), Some(&ScriptValue::I32(42)));
        assert_eq!(ok_val.get_err(), None);

        let err_val = ScriptResult::err(ScriptValue::String(ScriptRc::new(
            ScriptString::from_str("error"),
        )));
        assert!(!err_val.is_ok());
        assert!(err_val.is_err());
        assert_eq!(err_val.get_ok(), None);
        assert!(err_val.get_err().is_some());
    }

    #[test]
    fn test_result_methods() {
        let ok_val = ScriptResult::ok(ScriptValue::I32(10));
        let mapped = ok_val.map(|v| match v {
            ScriptValue::I32(n) => ScriptValue::I32(n * 2),
            _ => v.clone(),
        });
        assert_eq!(mapped.get_ok(), Some(&ScriptValue::I32(20)));

        let err_val = ScriptResult::err(ScriptValue::String(ScriptRc::new(
            ScriptString::from_str("error"),
        )));
        let mapped_err = err_val.map(|_v| ScriptValue::I32(100));
        assert!(mapped_err.is_err());

        assert_eq!(ok_val.unwrap_or(ScriptValue::I32(0)), ScriptValue::I32(10));
        assert_eq!(err_val.unwrap_or(ScriptValue::I32(0)), ScriptValue::I32(0));
    }

    #[test]
    #[should_panic(expected = "test message")]
    fn test_option_expect() {
        let none_val = ScriptOption::none();
        none_val.expect("test message");
    }

    #[test]
    #[should_panic(expected = "called `Result::unwrap()` on an `Err` value")]
    fn test_result_unwrap_panic() {
        let err_val = ScriptResult::err(ScriptValue::String(ScriptRc::new(
            ScriptString::from_str("error"),
        )));
        err_val.unwrap();
    }
}
