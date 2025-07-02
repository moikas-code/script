//! Core types for the Script programming language
//!
//! This module provides the fundamental types Option<T> and Result<T, E>
//! which are used throughout Script for optional values and error handling.

use crate::runtime::{RuntimeError, ScriptRc};
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
            ScriptOption::None => panic!("{}", msg),
        }
    }

    /// Map the inner value if Some
    pub fn map<F>(&self, f: F) -> ScriptOption
    where
        F: FnOnce(&ScriptValue) -> ScriptValue,
    {
        match self {
            ScriptOption::Some(val) => ScriptOption::Some(f(val)),
            ScriptOption::None => ScriptOption::None,
        }
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

    /// Map the Ok value if present
    pub fn map<F>(&self, f: F) -> ScriptResult
    where
        F: FnOnce(&ScriptValue) -> ScriptValue,
    {
        match self {
            ScriptResult::Ok(val) => ScriptResult::Ok(f(val)),
            ScriptResult::Err(err) => ScriptResult::Err(err.clone()),
        }
    }

    /// Map the Err value if present
    pub fn map_err<F>(&self, f: F) -> ScriptResult
    where
        F: FnOnce(&ScriptValue) -> ScriptValue,
    {
        match self {
            ScriptResult::Ok(val) => ScriptResult::Ok(val.clone()),
            ScriptResult::Err(err) => ScriptResult::Err(f(err)),
        }
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
