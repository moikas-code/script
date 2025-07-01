//! Color types and utilities for Script
//! 
//! This module provides color representation and manipulation:
//! - RGB and RGBA color types
//! - HSL/HSV color space support
//! - Color interpolation
//! - Common color constants
//! - Color conversion utilities

use crate::runtime::{RuntimeError, Result as RuntimeResult, ScriptValue, ScriptRc};
use std::collections::HashMap;

/// RGBA color representation (values in range 0.0 to 1.0)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    /// Create a new RGB color (alpha = 1.0)
    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Color {
            r: r.clamp(0.0, 1.0),
            g: g.clamp(0.0, 1.0),
            b: b.clamp(0.0, 1.0),
            a: 1.0,
        }
    }
    
    /// Create a new RGBA color
    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Color {
            r: r.clamp(0.0, 1.0),
            g: g.clamp(0.0, 1.0),
            b: b.clamp(0.0, 1.0),
            a: a.clamp(0.0, 1.0),
        }
    }
    
    /// Create color from hexadecimal value (0xRRGGBB)
    pub fn from_hex(hex: u32) -> Self {
        let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
        let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
        let b = (hex & 0xFF) as f32 / 255.0;
        Color::rgb(r, g, b)
    }
    
    /// Create color from hexadecimal value with alpha (0xRRGGBBAA)
    pub fn from_hex_alpha(hex: u32) -> Self {
        let r = ((hex >> 24) & 0xFF) as f32 / 255.0;
        let g = ((hex >> 16) & 0xFF) as f32 / 255.0;
        let b = ((hex >> 8) & 0xFF) as f32 / 255.0;
        let a = (hex & 0xFF) as f32 / 255.0;
        Color::rgba(r, g, b, a)
    }
    
    /// Convert to hexadecimal value (0xRRGGBB)
    pub fn to_hex(&self) -> u32 {
        let r = (self.r * 255.0) as u32;
        let g = (self.g * 255.0) as u32;
        let b = (self.b * 255.0) as u32;
        (r << 16) | (g << 8) | b
    }
    
    /// Convert to hexadecimal value with alpha (0xRRGGBBAA)
    pub fn to_hex_alpha(&self) -> u32 {
        let r = (self.r * 255.0) as u32;
        let g = (self.g * 255.0) as u32;
        let b = (self.b * 255.0) as u32;
        let a = (self.a * 255.0) as u32;
        (r << 24) | (g << 16) | (b << 8) | a
    }
    
    /// Create grayscale color
    pub fn gray(value: f32) -> Self {
        Color::rgb(value, value, value)
    }
    
    /// Create color from HSV values (hue in degrees 0-360)
    pub fn from_hsv(h: f32, s: f32, v: f32) -> Self {
        let h = h % 360.0;
        let s = s.clamp(0.0, 1.0);
        let v = v.clamp(0.0, 1.0);
        
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;
        
        let (r, g, b) = if h < 60.0 {
            (c, x, 0.0)
        } else if h < 120.0 {
            (x, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x)
        } else if h < 240.0 {
            (0.0, x, c)
        } else if h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };
        
        Color::rgb(r + m, g + m, b + m)
    }
    
    /// Convert to HSV values (returns (hue, saturation, value))
    pub fn to_hsv(&self) -> (f32, f32, f32) {
        let max = self.r.max(self.g).max(self.b);
        let min = self.r.min(self.g).min(self.b);
        let delta = max - min;
        
        let v = max;
        let s = if max == 0.0 { 0.0 } else { delta / max };
        
        let h = if delta == 0.0 {
            0.0
        } else if max == self.r {
            60.0 * (((self.g - self.b) / delta) % 6.0)
        } else if max == self.g {
            60.0 * ((self.b - self.r) / delta + 2.0)
        } else {
            60.0 * ((self.r - self.g) / delta + 4.0)
        };
        
        let h = if h < 0.0 { h + 360.0 } else { h };
        
        (h, s, v)
    }
    
    /// Create color from HSL values (hue in degrees 0-360)
    pub fn from_hsl(h: f32, s: f32, l: f32) -> Self {
        let h = h % 360.0;
        let s = s.clamp(0.0, 1.0);
        let l = l.clamp(0.0, 1.0);
        
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = l - c / 2.0;
        
        let (r, g, b) = if h < 60.0 {
            (c, x, 0.0)
        } else if h < 120.0 {
            (x, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x)
        } else if h < 240.0 {
            (0.0, x, c)
        } else if h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };
        
        Color::rgb(r + m, g + m, b + m)
    }
    
    /// Convert to HSL values (returns (hue, saturation, lightness))
    pub fn to_hsl(&self) -> (f32, f32, f32) {
        let max = self.r.max(self.g).max(self.b);
        let min = self.r.min(self.g).min(self.b);
        let delta = max - min;
        
        let l = (max + min) / 2.0;
        
        let s = if delta == 0.0 {
            0.0
        } else {
            delta / (1.0 - (2.0 * l - 1.0).abs())
        };
        
        let h = if delta == 0.0 {
            0.0
        } else if max == self.r {
            60.0 * (((self.g - self.b) / delta) % 6.0)
        } else if max == self.g {
            60.0 * ((self.b - self.r) / delta + 2.0)
        } else {
            60.0 * ((self.r - self.g) / delta + 4.0)
        };
        
        let h = if h < 0.0 { h + 360.0 } else { h };
        
        (h, s, l)
    }
    
    /// Get luminance (perceived brightness)
    pub fn luminance(&self) -> f32 {
        0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b
    }
    
    /// Linear interpolation between colors
    pub fn lerp(&self, other: Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        Color {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }
    
    /// Mix two colors (alpha blend)
    pub fn mix(&self, other: Color) -> Color {
        let alpha = other.a;
        let inv_alpha = 1.0 - alpha;
        
        Color {
            r: self.r * inv_alpha + other.r * alpha,
            g: self.g * inv_alpha + other.g * alpha,
            b: self.b * inv_alpha + other.b * alpha,
            a: self.a + other.a * (1.0 - self.a),
        }
    }
    
    /// Brighten the color
    pub fn brighten(&self, amount: f32) -> Color {
        let (h, s, v) = self.to_hsv();
        Color::from_hsv(h, s, (v + amount).clamp(0.0, 1.0))
    }
    
    /// Darken the color
    pub fn darken(&self, amount: f32) -> Color {
        self.brighten(-amount)
    }
    
    /// Saturate the color
    pub fn saturate(&self, amount: f32) -> Color {
        let (h, s, v) = self.to_hsv();
        Color::from_hsv(h, (s + amount).clamp(0.0, 1.0), v)
    }
    
    /// Desaturate the color
    pub fn desaturate(&self, amount: f32) -> Color {
        self.saturate(-amount)
    }
    
    /// Invert the color
    pub fn invert(&self) -> Color {
        Color {
            r: 1.0 - self.r,
            g: 1.0 - self.g,
            b: 1.0 - self.b,
            a: self.a,
        }
    }
    
    /// Get complementary color
    pub fn complement(&self) -> Color {
        let (h, s, v) = self.to_hsv();
        Color::from_hsv((h + 180.0) % 360.0, s, v)
    }
}

// Common color constants
impl Color {
    pub const BLACK: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const WHITE: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const RED: Color = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const GREEN: Color = Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 };
    pub const BLUE: Color = Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };
    pub const YELLOW: Color = Color { r: 1.0, g: 1.0, b: 0.0, a: 1.0 };
    pub const CYAN: Color = Color { r: 0.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const MAGENTA: Color = Color { r: 1.0, g: 0.0, b: 1.0, a: 1.0 };
    pub const TRANSPARENT: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
}

// Conversion functions for Script integration
impl Color {
    /// Convert to ScriptValue object
    pub fn to_script_value(&self) -> ScriptValue {
        let mut map = HashMap::new();
        map.insert("r".to_string(), ScriptValue::F32(self.r));
        map.insert("g".to_string(), ScriptValue::F32(self.g));
        map.insert("b".to_string(), ScriptValue::F32(self.b));
        map.insert("a".to_string(), ScriptValue::F32(self.a));
        ScriptValue::Object(ScriptRc::new(map))
    }
    
    /// Convert from ScriptValue object
    pub fn from_script_value(value: &ScriptValue) -> RuntimeResult<Color> {
        match value {
            ScriptValue::Object(obj) => {
                let r = obj.get("r")
                    .ok_or_else(|| RuntimeError::InvalidOperation("Color missing r component".to_string()))?
                    .to_f32()?;
                let g = obj.get("g")
                    .ok_or_else(|| RuntimeError::InvalidOperation("Color missing g component".to_string()))?
                    .to_f32()?;
                let b = obj.get("b")
                    .ok_or_else(|| RuntimeError::InvalidOperation("Color missing b component".to_string()))?
                    .to_f32()?;
                let a = obj.get("a")
                    .ok_or_else(|| RuntimeError::InvalidOperation("Color missing a component".to_string()))?
                    .to_f32()?;
                Ok(Color::rgba(r, g, b, a))
            }
            _ => Err(RuntimeError::InvalidOperation("Expected Color object".to_string())),
        }
    }
}

// Script function implementations

pub fn rgb_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(
            format!("rgb expects 3 arguments, got {}", args.len())
        ));
    }
    
    let r = args[0].to_f32()?;
    let g = args[1].to_f32()?;
    let b = args[2].to_f32()?;
    
    Ok(Color::rgb(r, g, b).to_script_value())
}

pub fn rgba_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 4 {
        return Err(RuntimeError::InvalidOperation(
            format!("rgba expects 4 arguments, got {}", args.len())
        ));
    }
    
    let r = args[0].to_f32()?;
    let g = args[1].to_f32()?;
    let b = args[2].to_f32()?;
    let a = args[3].to_f32()?;
    
    Ok(Color::rgba(r, g, b, a).to_script_value())
}

pub fn gray_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("gray expects 1 argument, got {}", args.len())
        ));
    }
    
    let value = args[0].to_f32()?;
    Ok(Color::gray(value).to_script_value())
}

pub fn hex_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("hex expects 1 argument, got {}", args.len())
        ));
    }
    
    let hex = args[0].to_i32()? as u32;
    Ok(Color::from_hex(hex).to_script_value())
}

pub fn hsv_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(
            format!("hsv expects 3 arguments, got {}", args.len())
        ));
    }
    
    let h = args[0].to_f32()?;
    let s = args[1].to_f32()?;
    let v = args[2].to_f32()?;
    
    Ok(Color::from_hsv(h, s, v).to_script_value())
}

pub fn hsl_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(
            format!("hsl expects 3 arguments, got {}", args.len())
        ));
    }
    
    let h = args[0].to_f32()?;
    let s = args[1].to_f32()?;
    let l = args[2].to_f32()?;
    
    Ok(Color::from_hsl(h, s, l).to_script_value())
}

pub fn color_to_hex_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("color_to_hex expects 1 argument, got {}", args.len())
        ));
    }
    
    let color = Color::from_script_value(&args[0])?;
    Ok(ScriptValue::I32(color.to_hex() as i32))
}

pub fn color_to_hsv_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("color_to_hsv expects 1 argument, got {}", args.len())
        ));
    }
    
    let color = Color::from_script_value(&args[0])?;
    let (h, s, v) = color.to_hsv();
    
    let mut map = HashMap::new();
    map.insert("h".to_string(), ScriptValue::F32(h));
    map.insert("s".to_string(), ScriptValue::F32(s));
    map.insert("v".to_string(), ScriptValue::F32(v));
    Ok(ScriptValue::Object(ScriptRc::new(map)))
}

pub fn color_to_hsl_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("color_to_hsl expects 1 argument, got {}", args.len())
        ));
    }
    
    let color = Color::from_script_value(&args[0])?;
    let (h, s, l) = color.to_hsl();
    
    let mut map = HashMap::new();
    map.insert("h".to_string(), ScriptValue::F32(h));
    map.insert("s".to_string(), ScriptValue::F32(s));
    map.insert("l".to_string(), ScriptValue::F32(l));
    Ok(ScriptValue::Object(ScriptRc::new(map)))
}

pub fn color_lerp_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(
            format!("color_lerp expects 3 arguments, got {}", args.len())
        ));
    }
    
    let color1 = Color::from_script_value(&args[0])?;
    let color2 = Color::from_script_value(&args[1])?;
    let t = args[2].to_f32()?;
    
    Ok(color1.lerp(color2, t).to_script_value())
}

pub fn color_mix_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            format!("color_mix expects 2 arguments, got {}", args.len())
        ));
    }
    
    let color1 = Color::from_script_value(&args[0])?;
    let color2 = Color::from_script_value(&args[1])?;
    
    Ok(color1.mix(color2).to_script_value())
}

pub fn color_brighten_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            format!("color_brighten expects 2 arguments, got {}", args.len())
        ));
    }
    
    let color = Color::from_script_value(&args[0])?;
    let amount = args[1].to_f32()?;
    
    Ok(color.brighten(amount).to_script_value())
}

pub fn color_darken_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            format!("color_darken expects 2 arguments, got {}", args.len())
        ));
    }
    
    let color = Color::from_script_value(&args[0])?;
    let amount = args[1].to_f32()?;
    
    Ok(color.darken(amount).to_script_value())
}

pub fn color_saturate_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            format!("color_saturate expects 2 arguments, got {}", args.len())
        ));
    }
    
    let color = Color::from_script_value(&args[0])?;
    let amount = args[1].to_f32()?;
    
    Ok(color.saturate(amount).to_script_value())
}

pub fn color_desaturate_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            format!("color_desaturate expects 2 arguments, got {}", args.len())
        ));
    }
    
    let color = Color::from_script_value(&args[0])?;
    let amount = args[1].to_f32()?;
    
    Ok(color.desaturate(amount).to_script_value())
}

pub fn color_invert_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("color_invert expects 1 argument, got {}", args.len())
        ));
    }
    
    let color = Color::from_script_value(&args[0])?;
    Ok(color.invert().to_script_value())
}

pub fn color_complement_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("color_complement expects 1 argument, got {}", args.len())
        ));
    }
    
    let color = Color::from_script_value(&args[0])?;
    Ok(color.complement().to_script_value())
}

pub fn color_luminance_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            format!("color_luminance expects 1 argument, got {}", args.len())
        ));
    }
    
    let color = Color::from_script_value(&args[0])?;
    Ok(ScriptValue::F32(color.luminance()))
}

// Color constants
pub fn color_black_impl(_args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    Ok(Color::BLACK.to_script_value())
}

pub fn color_white_impl(_args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    Ok(Color::WHITE.to_script_value())
}

pub fn color_red_impl(_args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    Ok(Color::RED.to_script_value())
}

pub fn color_green_impl(_args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    Ok(Color::GREEN.to_script_value())
}

pub fn color_blue_impl(_args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    Ok(Color::BLUE.to_script_value())
}

pub fn color_yellow_impl(_args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    Ok(Color::YELLOW.to_script_value())
}

pub fn color_cyan_impl(_args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    Ok(Color::CYAN.to_script_value())
}

pub fn color_magenta_impl(_args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    Ok(Color::MAGENTA.to_script_value())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_color_creation() {
        let c = Color::rgb(1.0, 0.5, 0.0);
        assert_eq!(c.r, 1.0);
        assert_eq!(c.g, 0.5);
        assert_eq!(c.b, 0.0);
        assert_eq!(c.a, 1.0);
        
        // Test clamping
        let c = Color::rgb(2.0, -0.5, 0.5);
        assert_eq!(c.r, 1.0);
        assert_eq!(c.g, 0.0);
        assert_eq!(c.b, 0.5);
    }
    
    #[test]
    fn test_hex_conversion() {
        let c = Color::from_hex(0xFF8000);
        assert!((c.r - 1.0).abs() < 0.01);
        assert!((c.g - 0.5).abs() < 0.01);
        assert!((c.b - 0.0).abs() < 0.01);
        
        let hex = Color::rgb(1.0, 0.5, 0.0).to_hex();
        assert_eq!(hex, 0xFF8000);
    }
    
    #[test]
    fn test_hsv_conversion() {
        // Test red
        let c = Color::from_hsv(0.0, 1.0, 1.0);
        assert!((c.r - 1.0).abs() < 0.01);
        assert!((c.g - 0.0).abs() < 0.01);
        assert!((c.b - 0.0).abs() < 0.01);
        
        // Test conversion roundtrip
        let c1 = Color::rgb(0.7, 0.3, 0.5);
        let (h, s, v) = c1.to_hsv();
        let c2 = Color::from_hsv(h, s, v);
        
        assert!((c1.r - c2.r).abs() < 0.01);
        assert!((c1.g - c2.g).abs() < 0.01);
        assert!((c1.b - c2.b).abs() < 0.01);
    }
    
    #[test]
    fn test_color_lerp() {
        let c1 = Color::rgb(0.0, 0.0, 0.0);
        let c2 = Color::rgb(1.0, 1.0, 1.0);
        
        let mid = c1.lerp(c2, 0.5);
        assert!((mid.r - 0.5).abs() < 0.01);
        assert!((mid.g - 0.5).abs() < 0.01);
        assert!((mid.b - 0.5).abs() < 0.01);
    }
    
    #[test]
    fn test_color_operations() {
        let c = Color::rgb(0.5, 0.5, 0.5);
        
        let brighter = c.brighten(0.2);
        let (_, _, v) = brighter.to_hsv();
        assert!(v > 0.5);
        
        let inverted = c.invert();
        assert!((inverted.r - 0.5).abs() < 0.01);
        assert!((inverted.g - 0.5).abs() < 0.01);
        assert!((inverted.b - 0.5).abs() < 0.01);
    }
    
    #[test]
    fn test_color_mix() {
        let c1 = Color::rgba(1.0, 0.0, 0.0, 1.0);
        let c2 = Color::rgba(0.0, 0.0, 1.0, 0.5);
        
        let mixed = c1.mix(c2);
        assert!((mixed.r - 0.5).abs() < 0.01);
        assert!((mixed.g - 0.0).abs() < 0.01);
        assert!((mixed.b - 0.5).abs() < 0.01);
    }
    
    #[test]
    fn test_script_value_conversion() {
        let c = Color::rgb(0.3, 0.6, 0.9);
        let script_val = c.to_script_value();
        let c2 = Color::from_script_value(&script_val).unwrap();
        
        assert_eq!(c, c2);
    }
}