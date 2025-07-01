//! Game-oriented utilities for Script
//! 
//! This module provides utilities commonly needed in game development:
//! - Vector math (Vec2, Vec3, Vec4)
//! - Math helpers (lerp, clamp, smoothstep)
//! - Random number generation
//! - Time utilities

use crate::runtime::{RuntimeError, ScriptRc};
use crate::stdlib::ScriptValue;
use std::collections::HashMap;

// Implementation functions for the stdlib registry

/// Create a 2D vector
pub(crate) fn vec2_new(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            format!("vec2 expects 2 arguments, got {}", args.len())
        ));
    }
    
    let x = args[0].to_f32()?;
    let y = args[1].to_f32()?;
    
    let mut vec = HashMap::new();
    vec.insert("x".to_string(), ScriptValue::F32(x));
    vec.insert("y".to_string(), ScriptValue::F32(y));
    
    Ok(ScriptValue::Object(ScriptRc::new(vec)))
}

/// Create a 3D vector
pub(crate) fn vec3_new(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(
            format!("vec3 expects 3 arguments, got {}", args.len())
        ));
    }
    
    let x = args[0].to_f32()?;
    let y = args[1].to_f32()?;
    let z = args[2].to_f32()?;
    
    let mut vec = HashMap::new();
    vec.insert("x".to_string(), ScriptValue::F32(x));
    vec.insert("y".to_string(), ScriptValue::F32(y));
    vec.insert("z".to_string(), ScriptValue::F32(z));
    
    Ok(ScriptValue::Object(ScriptRc::new(vec)))
}

/// Create a 4D vector
pub(crate) fn vec4_new(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 4 {
        return Err(RuntimeError::InvalidOperation(
            format!("vec4 expects 4 arguments, got {}", args.len())
        ));
    }
    
    let x = args[0].to_f32()?;
    let y = args[1].to_f32()?;
    let z = args[2].to_f32()?;
    let w = args[3].to_f32()?;
    
    let mut vec = HashMap::new();
    vec.insert("x".to_string(), ScriptValue::F32(x));
    vec.insert("y".to_string(), ScriptValue::F32(y));
    vec.insert("z".to_string(), ScriptValue::F32(z));
    vec.insert("w".to_string(), ScriptValue::F32(w));
    
    Ok(ScriptValue::Object(ScriptRc::new(vec)))
}

/// Add two 2D vectors
pub(crate) fn vec2_add(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            format!("vec2_add expects 2 arguments, got {}", args.len())
        ));
    }
    
    match (&args[0], &args[1]) {
        (ScriptValue::Object(a), ScriptValue::Object(b)) => {
            let ax = a.get("x").and_then(|v| v.as_f32()).ok_or_else(|| {
                RuntimeError::InvalidOperation("First argument is not a valid vec2".to_string())
            })?;
            let ay = a.get("y").and_then(|v| v.as_f32()).ok_or_else(|| {
                RuntimeError::InvalidOperation("First argument is not a valid vec2".to_string())
            })?;
            let bx = b.get("x").and_then(|v| v.as_f32()).ok_or_else(|| {
                RuntimeError::InvalidOperation("Second argument is not a valid vec2".to_string())
            })?;
            let by = b.get("y").and_then(|v| v.as_f32()).ok_or_else(|| {
                RuntimeError::InvalidOperation("Second argument is not a valid vec2".to_string())
            })?;
            
            let mut result = HashMap::new();
            result.insert("x".to_string(), ScriptValue::F32(ax + bx));
            result.insert("y".to_string(), ScriptValue::F32(ay + by));
            
            Ok(ScriptValue::Object(ScriptRc::new(result)))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "vec2_add expects two vec2 objects".to_string()
        )),
    }
}

/// Dot product of two 2D vectors
pub(crate) fn vec2_dot(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            format!("vec2_dot expects 2 arguments, got {}", args.len())
        ));
    }
    
    match (&args[0], &args[1]) {
        (ScriptValue::Object(a), ScriptValue::Object(b)) => {
            let ax = a.get("x").and_then(|v| v.as_f32()).ok_or_else(|| {
                RuntimeError::InvalidOperation("First argument is not a valid vec2".to_string())
            })?;
            let ay = a.get("y").and_then(|v| v.as_f32()).ok_or_else(|| {
                RuntimeError::InvalidOperation("First argument is not a valid vec2".to_string())
            })?;
            let bx = b.get("x").and_then(|v| v.as_f32()).ok_or_else(|| {
                RuntimeError::InvalidOperation("Second argument is not a valid vec2".to_string())
            })?;
            let by = b.get("y").and_then(|v| v.as_f32()).ok_or_else(|| {
                RuntimeError::InvalidOperation("Second argument is not a valid vec2".to_string())
            })?;
            
            Ok(ScriptValue::F32(ax * bx + ay * by))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "vec2_dot expects two vec2 objects".to_string()
        )),
    }
}

/// Length of a 2D vector
pub(crate) fn vec2_length(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("vec2_length expects 1 argument, got {}", args.len())
        ));
    }
    
    match &args[0] {
        ScriptValue::Object(vec) => {
            let x = vec.get("x").and_then(|v| v.as_f32()).ok_or_else(|| {
                RuntimeError::InvalidOperation("Argument is not a valid vec2".to_string())
            })?;
            let y = vec.get("y").and_then(|v| v.as_f32()).ok_or_else(|| {
                RuntimeError::InvalidOperation("Argument is not a valid vec2".to_string())
            })?;
            
            Ok(ScriptValue::F32((x * x + y * y).sqrt()))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "vec2_length expects a vec2 object".to_string()
        )),
    }
}

/// Linear interpolation
pub(crate) fn lerp(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(
            format!("lerp expects 3 arguments, got {}", args.len())
        ));
    }
    
    let a = args[0].to_f32()?;
    let b = args[1].to_f32()?;
    let t = args[2].to_f32()?;
    
    Ok(ScriptValue::F32(a + (b - a) * t))
}

/// Clamp a value between min and max
pub(crate) fn clamp(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(
            format!("clamp expects 3 arguments, got {}", args.len())
        ));
    }
    
    let value = args[0].to_f32()?;
    let min = args[1].to_f32()?;
    let max = args[2].to_f32()?;
    
    Ok(ScriptValue::F32(value.max(min).min(max)))
}

/// Smooth interpolation (smoothstep)
pub(crate) fn smoothstep(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(
            format!("smoothstep expects 3 arguments, got {}", args.len())
        ));
    }
    
    let edge0 = args[0].to_f32()?;
    let edge1 = args[1].to_f32()?;
    let x = args[2].to_f32()?;
    
    let t = ((x - edge0) / (edge1 - edge0)).max(0.0).min(1.0);
    let smooth = t * t * (3.0 - 2.0 * t);
    
    Ok(ScriptValue::F32(smooth))
}

/// Generate a random float between 0 and 1
pub(crate) fn random(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 0 {
        return Err(RuntimeError::InvalidOperation(
            format!("random expects 0 arguments, got {}", args.len())
        ));
    }
    
    use rand::Rng;
    let mut rng = rand::thread_rng();
    Ok(ScriptValue::F32(rng.gen::<f32>()))
}

/// Generate a random float between min and max
pub(crate) fn random_range(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            format!("random_range expects 2 arguments, got {}", args.len())
        ));
    }
    
    let min = args[0].to_f32()?;
    let max = args[1].to_f32()?;
    
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let value = min + (max - min) * rng.gen::<f32>();
    
    Ok(ScriptValue::F32(value))
}

/// Generate a random integer between min and max (inclusive)
pub(crate) fn random_int(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            format!("random_int expects 2 arguments, got {}", args.len())
        ));
    }
    
    let min = args[0].to_i32()?;
    let max = args[1].to_i32()?;
    
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let value = rng.gen_range(min..=max);
    
    Ok(ScriptValue::I32(value))
}

/// Get the current time in seconds
pub(crate) fn time_now(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 0 {
        return Err(RuntimeError::InvalidOperation(
            format!("time_now expects 0 arguments, got {}", args.len())
        ));
    }
    
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| RuntimeError::InvalidOperation(format!("Time error: {}", e)))?;
    
    Ok(ScriptValue::F32(duration.as_secs_f32()))
}

/// Convert degrees to radians
pub(crate) fn deg_to_rad(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("deg_to_rad expects 1 argument, got {}", args.len())
        ));
    }
    
    let degrees = args[0].to_f32()?;
    Ok(ScriptValue::F32(degrees.to_radians()))
}

/// Convert radians to degrees
pub(crate) fn rad_to_deg(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("rad_to_deg expects 1 argument, got {}", args.len())
        ));
    }
    
    let radians = args[0].to_f32()?;
    Ok(ScriptValue::F32(radians.to_degrees()))
}