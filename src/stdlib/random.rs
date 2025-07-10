//! Random number generation for Script
//!
//! This module provides pseudo-random number generation with support for:
//! - Seeded RNG for deterministic sequences
//! - Various random value types (integers, floats, booleans)
//! - Range-based generation
//! - Shuffle operations
//! - Common game-oriented random utilities

use crate::runtime::{Result as RuntimeResult, RuntimeError, ScriptRc};
use crate::stdlib::{ScriptString, ScriptValue};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

thread_local! {
    /// Thread-local RNG instance
    static RNG: RefCell<StdRng> = RefCell::new(StdRng::from_entropy());
}

/// Script-accessible RNG handle
pub struct ScriptRng {
    rng: Rc<RefCell<StdRng>>,
}

impl ScriptRng {
    /// Create a new RNG with a random seed
    pub fn new() -> Self {
        ScriptRng {
            rng: Rc::new(RefCell::new(StdRng::from_entropy())),
        }
    }

    /// Create a new RNG with a specific seed
    pub fn new_with_seed(seed: u64) -> Self {
        ScriptRng {
            rng: Rc::new(RefCell::new(StdRng::seed_from_u64(seed))),
        }
    }

    /// Generate a random f32 between 0.0 and 1.0
    pub fn random(&self) -> f32 {
        self.rng.borrow_mut().gen::<f32>()
    }

    /// Generate a random f32 in the given range
    pub fn random_range(&self, min: f32, max: f32) -> f32 {
        if min >= max {
            return min;
        }
        self.rng.borrow_mut().gen_range(min..max)
    }

    /// Generate a random i32 in the given range (inclusive)
    pub fn random_int(&self, min: i32, max: i32) -> i32 {
        if min >= max {
            return min;
        }
        self.rng.borrow_mut().gen_range(min..=max)
    }

    /// Generate a random boolean with given probability of being true
    pub fn random_bool(&self, probability: f32) -> bool {
        self.random() < probability
    }

    /// Pick a random index from 0 to count-1
    pub fn random_index(&self, count: usize) -> usize {
        if count == 0 {
            return 0;
        }
        self.rng.borrow_mut().gen_range(0..count)
    }

    /// Shuffle a vector in place
    pub fn shuffle<T>(&self, items: &mut Vec<T>) {
        use rand::seq::SliceRandom;
        items.shuffle(&mut *self.rng.borrow_mut());
    }

    /// Generate a random unit vector (normalized)
    pub fn random_unit_vec2(&self) -> (f32, f32) {
        let angle = self.random_range(0.0, std::f32::consts::TAU);
        (angle.cos(), angle.sin())
    }

    /// Generate a random unit vector in 3D (uniform distribution on sphere)
    pub fn random_unit_vec3(&self) -> (f32, f32, f32) {
        // Use sphere point picking algorithm
        let theta = self.random_range(0.0, std::f32::consts::TAU);
        let phi = (1.0 - 2.0 * self.random()).acos();

        let sin_phi = phi.sin();
        (sin_phi * theta.cos(), sin_phi * theta.sin(), phi.cos())
    }

    /// Generate a random point in a circle
    pub fn random_in_circle(&self, radius: f32) -> (f32, f32) {
        // Use rejection sampling for uniform distribution
        loop {
            let x = self.random_range(-radius, radius);
            let y = self.random_range(-radius, radius);
            if x * x + y * y <= radius * radius {
                return (x, y);
            }
        }
    }

    /// Generate a random point in a sphere
    pub fn random_in_sphere(&self, radius: f32) -> (f32, f32, f32) {
        // Use rejection sampling for uniform distribution
        loop {
            let x = self.random_range(-radius, radius);
            let y = self.random_range(-radius, radius);
            let z = self.random_range(-radius, radius);
            if x * x + y * y + z * z <= radius * radius {
                return (x, y, z);
            }
        }
    }

    /// Perlin noise (simplified, non-standard implementation for games)
    pub fn noise(&self, x: f32, y: f32) -> f32 {
        // This is a simplified noise function, not true Perlin noise
        // For production use, consider a proper noise library
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;
        let xf = x - xi as f32;
        let yf = y - yi as f32;

        // Generate pseudo-random gradients
        let n00 = self.gradient_noise(xi, yi, xf, yf);
        let n10 = self.gradient_noise(xi + 1, yi, xf - 1.0, yf);
        let n01 = self.gradient_noise(xi, yi + 1, xf, yf - 1.0);
        let n11 = self.gradient_noise(xi + 1, yi + 1, xf - 1.0, yf - 1.0);

        // Interpolate
        let u = xf * xf * (3.0 - 2.0 * xf);
        let v = yf * yf * (3.0 - 2.0 * yf);

        let x0 = n00 * (1.0 - u) + n10 * u;
        let x1 = n01 * (1.0 - u) + n11 * u;

        x0 * (1.0 - v) + x1 * v
    }

    fn gradient_noise(&self, ix: i32, iy: i32, x: f32, y: f32) -> f32 {
        // Simple hash function for gradient
        let hash = ((ix * 73856093) ^ (iy * 19349663)) as u32;
        let angle = (hash as f32) * 2.399963229728653; // (2 * PI / golden ratio)
        x * angle.cos() + y * angle.sin()
    }
}

// Global RNG functions (using thread-local RNG)

/// Generate a random f32 between 0.0 and 1.0
pub fn random_impl(_args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    RNG.with(|rng| Ok(ScriptValue::F32(rng.borrow_mut().gen::<f32>())))
}

/// Generate a random f32 in the given range
pub fn random_range_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "random_range expects 2 arguments, got {}",
            args.len()
        )));
    }

    let min = args[0].to_f32()?;
    let max = args[1].to_f32()?;

    if min >= max {
        return Ok(ScriptValue::F32(min));
    }

    RNG.with(|rng| Ok(ScriptValue::F32(rng.borrow_mut().gen_range(min..max))))
}

/// Generate a random i32 in the given range (inclusive)
pub fn random_int_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "random_int expects 2 arguments, got {}",
            args.len()
        )));
    }

    let min = args[0].to_i32()?;
    let max = args[1].to_i32()?;

    if min >= max {
        return Ok(ScriptValue::I32(min));
    }

    RNG.with(|rng| Ok(ScriptValue::I32(rng.borrow_mut().gen_range(min..=max))))
}

/// Generate a random boolean with given probability
pub fn random_bool_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    let probability = if args.is_empty() {
        0.5
    } else if args.len() == 1 {
        args[0].to_f32()?
    } else {
        return Err(RuntimeError::InvalidOperation(format!(
            "random_bool expects 0 or 1 argument, got {}",
            args.len()
        )));
    };

    RNG.with(|rng| {
        Ok(ScriptValue::Bool(
            rng.borrow_mut().gen::<f32>() < probability,
        ))
    })
}

/// Set the global RNG seed
pub fn random_seed_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "random_seed expects 1 argument, got {}",
            args.len()
        )));
    }

    let seed = args[0].to_i32()? as u64;

    RNG.with(|rng| {
        *rng.borrow_mut() = StdRng::seed_from_u64(seed);
    });

    Ok(ScriptValue::Unit)
}

/// Create a new RNG instance
pub fn rng_new_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    let rng = if args.is_empty() {
        ScriptRng::new()
    } else if args.len() == 1 {
        let seed = args[0].to_i32()? as u64;
        ScriptRng::new_with_seed(seed)
    } else {
        return Err(RuntimeError::InvalidOperation(format!(
            "rng_new expects 0 or 1 argument, got {}",
            args.len()
        )));
    };

    let mut map = HashMap::new();
    // Store RNG as a marker in the object - actual RNG would need proper wrapper
    map.insert(
        "_rng_marker".to_string(),
        ScriptValue::String(ScriptRc::new(ScriptString::from_str(
            "RandomNumberGenerator",
        ))),
    );
    Ok(ScriptValue::Object(ScriptRc::new(map)))
}

/// Generate random from RNG instance
pub fn rng_random_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "rng.random expects 1 argument (self), got {}",
            args.len()
        )));
    }

    let rng = get_rng_from_object(&args[0])?;
    Ok(ScriptValue::F32(rng.random()))
}

/// Generate random range from RNG instance
pub fn rng_random_range_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(format!(
            "rng.random_range expects 3 arguments, got {}",
            args.len()
        )));
    }

    let rng = get_rng_from_object(&args[0])?;
    let min = args[1].to_f32()?;
    let max = args[2].to_f32()?;

    Ok(ScriptValue::F32(rng.random_range(min, max)))
}

/// Generate random int from RNG instance
pub fn rng_random_int_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(format!(
            "rng.random_int expects 3 arguments, got {}",
            args.len()
        )));
    }

    let rng = get_rng_from_object(&args[0])?;
    let min = args[1].to_i32()?;
    let max = args[2].to_i32()?;

    Ok(ScriptValue::I32(rng.random_int(min, max)))
}

/// Shuffle an array
pub fn shuffle_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "shuffle expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::Array(arr) => {
            let mut items = (**arr).clone();

            RNG.with(|rng| {
                items
                    .shuffle(&mut *rng.borrow_mut())
                    .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))
            })
            .map_err(|e| e)?;

            Ok(ScriptValue::Array(ScriptRc::new(items)))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "shuffle expects an array".to_string(),
        )),
    }
}

/// Pick a random element from an array
pub fn pick_random_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "pick_random expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::Array(arr) => {
            if arr.is_empty() {
                return Err(RuntimeError::InvalidOperation(
                    "Cannot pick from empty array".to_string(),
                ));
            }

            let index = RNG.with(|rng| rng.borrow_mut().gen_range(0..arr.len()));

            arr.get(index)
                .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?
                .ok_or_else(|| RuntimeError::InvalidOperation("Index out of bounds".to_string()))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "pick_random expects an array".to_string(),
        )),
    }
}

/// Generate a random unit vector 2D
pub fn random_unit_vec2_impl(_args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    let (x, y) = RNG.with(|rng| {
        let angle = rng.borrow_mut().gen_range(0.0..std::f32::consts::TAU);
        (angle.cos(), angle.sin())
    });

    let mut map = HashMap::new();
    map.insert("x".to_string(), ScriptValue::F32(x));
    map.insert("y".to_string(), ScriptValue::F32(y));
    Ok(ScriptValue::Object(ScriptRc::new(map)))
}

/// Generate a random unit vector 3D
pub fn random_unit_vec3_impl(_args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    let (x, y, z) = RNG.with(|rng| {
        let mut r = &mut *rng.borrow_mut();
        let theta = r.gen_range(0.0..std::f32::consts::TAU);
        let phi = (1.0 - 2.0 * r.gen::<f32>()).acos();

        let sin_phi = phi.sin();
        (sin_phi * theta.cos(), sin_phi * theta.sin(), phi.cos())
    });

    let mut map = HashMap::new();
    map.insert("x".to_string(), ScriptValue::F32(x));
    map.insert("y".to_string(), ScriptValue::F32(y));
    map.insert("z".to_string(), ScriptValue::F32(z));
    Ok(ScriptValue::Object(ScriptRc::new(map)))
}

/// Generate a random point in a circle
pub fn random_in_circle_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    let radius = if args.is_empty() {
        1.0
    } else if args.len() == 1 {
        args[0].to_f32()?
    } else {
        return Err(RuntimeError::InvalidOperation(format!(
            "random_in_circle expects 0 or 1 argument, got {}",
            args.len()
        )));
    };

    let (x, y) = RNG.with(|rng| {
        let mut r = &mut *rng.borrow_mut();
        loop {
            let x = r.gen_range(-radius..radius);
            let y = r.gen_range(-radius..radius);
            if x * x + y * y <= radius * radius {
                return (x, y);
            }
        }
    });

    let mut map = HashMap::new();
    map.insert("x".to_string(), ScriptValue::F32(x));
    map.insert("y".to_string(), ScriptValue::F32(y));
    Ok(ScriptValue::Object(ScriptRc::new(map)))
}

/// Generate weighted random selection
pub fn weighted_random_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "weighted_random expects 2 arguments, got {}",
            args.len()
        )));
    }

    let items = match &args[0] {
        ScriptValue::Array(arr) => arr,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "weighted_random expects an array as first argument".to_string(),
            ))
        }
    };

    let weights = match &args[1] {
        ScriptValue::Array(arr) => arr,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "weighted_random expects an array as second argument".to_string(),
            ))
        }
    };

    if items.len() != weights.len() {
        return Err(RuntimeError::InvalidOperation(
            "Items and weights arrays must have same length".to_string(),
        ));
    }

    if items.is_empty() {
        return Err(RuntimeError::InvalidOperation(
            "Cannot pick from empty array".to_string(),
        ));
    }

    // Calculate total weight
    let mut total_weight = 0.0f32;
    for i in 0..weights.len() {
        let w = weights
            .get(i)
            .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?
            .ok_or_else(|| {
                RuntimeError::InvalidOperation("Weight index out of bounds".to_string())
            })?;
        total_weight += w
            .as_f32()
            .ok_or_else(|| RuntimeError::InvalidOperation("Weight must be a number".to_string()))?;
    }

    if total_weight <= 0.0 {
        return Err(RuntimeError::InvalidOperation(
            "Total weight must be positive".to_string(),
        ));
    }

    // Pick random value and find corresponding item
    let pick = RNG.with(|rng| rng.borrow_mut().gen_range(0.0..total_weight));

    let mut accumulated = 0.0;
    for i in 0..weights.len() {
        let w = weights
            .get(i)
            .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?
            .ok_or_else(|| {
                RuntimeError::InvalidOperation("Weight index out of bounds".to_string())
            })?;
        accumulated += w
            .as_f32()
            .ok_or_else(|| RuntimeError::InvalidOperation("Weight must be a number".to_string()))?;
        if pick <= accumulated {
            return items
                .get(i)
                .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?
                .ok_or_else(|| {
                    RuntimeError::InvalidOperation("Item index out of bounds".to_string())
                });
        }
    }

    // Fallback (should not reach here)
    items
        .get(items.len() - 1)
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?
        .ok_or_else(|| RuntimeError::InvalidOperation("No items in array".to_string()))
}

// Helper function to extract RNG from object
fn get_rng_from_object(_obj: &ScriptValue) -> RuntimeResult<&ScriptRng> {
    // This is a placeholder - in a real implementation, we'd extract the RNG
    // from the NativeData field in the object
    Err(RuntimeError::InvalidOperation(
        "RNG object access not fully implemented".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_range() {
        for _ in 0..100 {
            let result =
                random_range_impl(&[ScriptValue::F32(0.0), ScriptValue::F32(1.0)]).unwrap();
            match result {
                ScriptValue::F32(val) => {
                    assert!(val >= 0.0 && val < 1.0);
                }
                _ => panic!("Expected F32"),
            }
        }
    }

    #[test]
    fn test_random_int() {
        for _ in 0..100 {
            let result = random_int_impl(&[ScriptValue::I32(1), ScriptValue::I32(6)]).unwrap();
            match result {
                ScriptValue::I32(val) => {
                    assert!(val >= 1 && val <= 6);
                }
                _ => panic!("Expected I32"),
            }
        }
    }

    #[test]
    fn test_random_bool() {
        let mut true_count = 0;
        let iterations = 1000;

        for _ in 0..iterations {
            let result = random_bool_impl(&[ScriptValue::F32(0.7)]).unwrap();
            match result {
                ScriptValue::Bool(val) => {
                    if val {
                        true_count += 1;
                    }
                }
                _ => panic!("Expected Bool"),
            }
        }

        // Should be approximately 70% true
        let ratio = true_count as f32 / iterations as f32;
        assert!(ratio > 0.65 && ratio < 0.75);
    }

    #[test]
    fn test_shuffle() {
        let mut values = vec![
            ScriptValue::I32(1),
            ScriptValue::I32(2),
            ScriptValue::I32(3),
            ScriptValue::I32(4),
            ScriptValue::I32(5),
        ];

        let original = values.clone();
        let arr = ScriptValue::Array(ScriptRc::new(values));

        let shuffled = shuffle_impl(&[arr]).unwrap();

        match shuffled {
            ScriptValue::Array(arr) => {
                // Check same length
                assert_eq!(arr.len(), original.len());

                // Check all elements are present
                for val in original.iter() {
                    assert!(arr.contains(val));
                }
            }
            _ => panic!("Expected Array"),
        }
    }

    #[test]
    fn test_random_unit_vec2() {
        let result = random_unit_vec2_impl(&[]).unwrap();

        match result {
            ScriptValue::Object(obj) => {
                let x = obj.get("x").unwrap().to_f32().unwrap();
                let y = obj.get("y").unwrap().to_f32().unwrap();

                // Check it's a unit vector
                let length = (x * x + y * y).sqrt();
                assert!((length - 1.0).abs() < 0.0001);
            }
            _ => panic!("Expected Object"),
        }
    }

    #[test]
    fn test_seeded_rng() {
        // Set seed
        random_seed_impl(&[ScriptValue::I32(12345)]).unwrap();

        // Generate some values
        let val1 = random_impl(&[]).unwrap();
        let val2 = random_impl(&[]).unwrap();

        // Reset seed to same value
        random_seed_impl(&[ScriptValue::I32(12345)]).unwrap();

        // Should get same values
        let val3 = random_impl(&[]).unwrap();
        let val4 = random_impl(&[]).unwrap();

        assert_eq!(val1, val3);
        assert_eq!(val2, val4);
    }
}
