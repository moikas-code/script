//! Math functions for Script
//! 
//! This module provides standard mathematical functions including:
//! - Basic operations (abs, min, max)
//! - Trigonometry (sin, cos, tan, etc.)
//! - Exponential and logarithmic functions
//! - Rounding functions (floor, ceil, round)

use crate::runtime::RuntimeError;
use crate::stdlib::ScriptValue;

// Implementation functions for the stdlib registry

/// Absolute value
pub(crate) fn abs_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("abs expects 1 argument, got {}", args.len())
        ));
    }
    
    match &args[0] {
        ScriptValue::I32(val) => Ok(ScriptValue::I32(val.abs())),
        ScriptValue::F32(val) => Ok(ScriptValue::F32(val.abs())),
        _ => Err(RuntimeError::InvalidOperation(
            "abs expects a number argument".to_string()
        )),
    }
}

/// Minimum of two values
pub(crate) fn min_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            format!("min expects 2 arguments, got {}", args.len())
        ));
    }
    
    let a = args[0].to_f32()?;
    let b = args[1].to_f32()?;
    Ok(ScriptValue::F32(a.min(b)))
}

/// Maximum of two values
pub(crate) fn max_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            format!("max expects 2 arguments, got {}", args.len())
        ));
    }
    
    let a = args[0].to_f32()?;
    let b = args[1].to_f32()?;
    Ok(ScriptValue::F32(a.max(b)))
}

/// Sign of a number (-1, 0, or 1)
pub(crate) fn sign_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("sign expects 1 argument, got {}", args.len())
        ));
    }
    
    let val = args[0].to_f32()?;
    let sign = if val > 0.0 { 1.0 } else if val < 0.0 { -1.0 } else { 0.0 };
    Ok(ScriptValue::F32(sign))
}

/// Power (x^y)
pub(crate) fn pow_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            format!("pow expects 2 arguments, got {}", args.len())
        ));
    }
    
    let base = args[0].to_f32()?;
    let exp = args[1].to_f32()?;
    Ok(ScriptValue::F32(base.powf(exp)))
}

/// Square root
pub(crate) fn sqrt_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("sqrt expects 1 argument, got {}", args.len())
        ));
    }
    
    let val = args[0].to_f32()?;
    Ok(ScriptValue::F32(val.sqrt()))
}

/// Cube root
pub(crate) fn cbrt_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("cbrt expects 1 argument, got {}", args.len())
        ));
    }
    
    let val = args[0].to_f32()?;
    Ok(ScriptValue::F32(val.cbrt()))
}

/// Exponential (e^x)
pub(crate) fn exp_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("exp expects 1 argument, got {}", args.len())
        ));
    }
    
    let val = args[0].to_f32()?;
    Ok(ScriptValue::F32(val.exp()))
}

/// Natural logarithm
pub(crate) fn log_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("log expects 1 argument, got {}", args.len())
        ));
    }
    
    let val = args[0].to_f32()?;
    Ok(ScriptValue::F32(val.ln()))
}

/// Base 10 logarithm
pub(crate) fn log10_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("log10 expects 1 argument, got {}", args.len())
        ));
    }
    
    let val = args[0].to_f32()?;
    Ok(ScriptValue::F32(val.log10()))
}

/// Base 2 logarithm
pub(crate) fn log2_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("log2 expects 1 argument, got {}", args.len())
        ));
    }
    
    let val = args[0].to_f32()?;
    Ok(ScriptValue::F32(val.log2()))
}

/// Sine
pub(crate) fn sin_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("sin expects 1 argument, got {}", args.len())
        ));
    }
    
    let val = args[0].to_f32()?;
    Ok(ScriptValue::F32(val.sin()))
}

/// Cosine
pub(crate) fn cos_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("cos expects 1 argument, got {}", args.len())
        ));
    }
    
    let val = args[0].to_f32()?;
    Ok(ScriptValue::F32(val.cos()))
}

/// Tangent
pub(crate) fn tan_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("tan expects 1 argument, got {}", args.len())
        ));
    }
    
    let val = args[0].to_f32()?;
    Ok(ScriptValue::F32(val.tan()))
}

/// Arcsine
pub(crate) fn asin_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("asin expects 1 argument, got {}", args.len())
        ));
    }
    
    let val = args[0].to_f32()?;
    Ok(ScriptValue::F32(val.asin()))
}

/// Arccosine
pub(crate) fn acos_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("acos expects 1 argument, got {}", args.len())
        ));
    }
    
    let val = args[0].to_f32()?;
    Ok(ScriptValue::F32(val.acos()))
}

/// Arctangent
pub(crate) fn atan_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("atan expects 1 argument, got {}", args.len())
        ));
    }
    
    let val = args[0].to_f32()?;
    Ok(ScriptValue::F32(val.atan()))
}

/// Two-argument arctangent
pub(crate) fn atan2_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            format!("atan2 expects 2 arguments, got {}", args.len())
        ));
    }
    
    let y = args[0].to_f32()?;
    let x = args[1].to_f32()?;
    Ok(ScriptValue::F32(y.atan2(x)))
}

/// Hyperbolic sine
pub(crate) fn sinh_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("sinh expects 1 argument, got {}", args.len())
        ));
    }
    
    let val = args[0].to_f32()?;
    Ok(ScriptValue::F32(val.sinh()))
}

/// Hyperbolic cosine
pub(crate) fn cosh_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("cosh expects 1 argument, got {}", args.len())
        ));
    }
    
    let val = args[0].to_f32()?;
    Ok(ScriptValue::F32(val.cosh()))
}

/// Hyperbolic tangent
pub(crate) fn tanh_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("tanh expects 1 argument, got {}", args.len())
        ));
    }
    
    let val = args[0].to_f32()?;
    Ok(ScriptValue::F32(val.tanh()))
}

/// Floor (round down)
pub(crate) fn floor_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("floor expects 1 argument, got {}", args.len())
        ));
    }
    
    let val = args[0].to_f32()?;
    Ok(ScriptValue::F32(val.floor()))
}

/// Ceiling (round up)
pub(crate) fn ceil_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("ceil expects 1 argument, got {}", args.len())
        ));
    }
    
    let val = args[0].to_f32()?;
    Ok(ScriptValue::F32(val.ceil()))
}

/// Round to nearest integer
pub(crate) fn round_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("round expects 1 argument, got {}", args.len())
        ));
    }
    
    let val = args[0].to_f32()?;
    Ok(ScriptValue::F32(val.round()))
}

/// Truncate (round towards zero)
pub(crate) fn trunc_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("trunc expects 1 argument, got {}", args.len())
        ));
    }
    
    let val = args[0].to_f32()?;
    Ok(ScriptValue::F32(val.trunc()))
}