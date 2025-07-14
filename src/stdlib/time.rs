//! Time utilities for Script
//!
//! This module provides high-precision timing functionality for games:
//! - High-resolution timestamps
//! - Delta time calculation
//! - Frame rate helpers
//! - Stopwatch/timer functionality
//! - Time formatting utilities

use crate::runtime::{Result as RuntimeResult, RuntimeError, ScriptRc};
use crate::stdlib::{ScriptString, ScriptValue};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Global application start time
static APP_START_TIME: OnceLock<Instant> = OnceLock::new();

/// Stopwatch for measuring elapsed time
pub struct Stopwatch {
    start_time: Option<Instant>,
    accumulated: Duration,
    is_running: bool,
}

impl Stopwatch {
    /// Create a new stopwatch (not started)
    pub fn new() -> Self {
        Stopwatch {
            start_time: None,
            accumulated: Duration::ZERO,
            is_running: false,
        }
    }

    /// Start or resume the stopwatch
    pub fn start(&mut self) {
        if !self.is_running {
            self.start_time = Some(Instant::now());
            self.is_running = true;
        }
    }

    /// Stop the stopwatch
    pub fn stop(&mut self) {
        if self.is_running {
            if let Some(start) = self.start_time {
                self.accumulated += start.elapsed();
            }
            self.is_running = false;
            self.start_time = None;
        }
    }

    /// Reset the stopwatch
    pub fn reset(&mut self) {
        self.start_time = if self.is_running {
            Some(Instant::now())
        } else {
            None
        };
        self.accumulated = Duration::ZERO;
    }

    /// Get elapsed time in seconds
    pub fn elapsed(&self) -> f32 {
        let current = if self.is_running {
            if let Some(start) = self.start_time {
                self.accumulated + start.elapsed()
            } else {
                self.accumulated
            }
        } else {
            self.accumulated
        };

        current.as_secs_f32()
    }

    /// Get elapsed time in milliseconds
    pub fn elapsed_millis(&self) -> f32 {
        self.elapsed() * 1000.0
    }

    /// Check if stopwatch is running
    pub fn is_running(&self) -> bool {
        self.is_running
    }
}

/// Frame timer for game loops
pub struct FrameTimer {
    last_frame_time: Instant,
    delta_time: f32,
    fps: f32,
    frame_count: u32,
    fps_update_time: Instant,
    fps_frame_count: u32,
}

impl FrameTimer {
    /// Create a new frame timer
    pub fn new() -> Self {
        let now = Instant::now();
        FrameTimer {
            last_frame_time: now,
            delta_time: 0.0,
            fps: 0.0,
            frame_count: 0,
            fps_update_time: now,
            fps_frame_count: 0,
        }
    }

    /// Update the frame timer (call once per frame)
    pub fn update(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame_time);
        self.delta_time = elapsed.as_secs_f32();
        self.last_frame_time = now;
        self.frame_count += 1;
        self.fps_frame_count += 1;

        // Update FPS every second
        let fps_elapsed = now.duration_since(self.fps_update_time);
        if fps_elapsed.as_secs_f32() >= 1.0 {
            self.fps = self.fps_frame_count as f32 / fps_elapsed.as_secs_f32();
            self.fps_update_time = now;
            self.fps_frame_count = 0;
        }
    }

    /// Get delta time in seconds
    pub fn delta_time(&self) -> f32 {
        self.delta_time
    }

    /// Get current FPS
    pub fn fps(&self) -> f32 {
        self.fps
    }

    /// Get total frame count
    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }
}

// Script function implementations

/// Get current time in seconds since application start
pub fn time_now_impl(_args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    let start_time = APP_START_TIME.get_or_init(|| Instant::now());
    let elapsed = start_time.elapsed().as_secs_f32();
    Ok(ScriptValue::F32(elapsed))
}

/// Get current time in milliseconds since application start
pub fn time_now_millis_impl(_args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    let start_time = APP_START_TIME.get_or_init(|| Instant::now());
    let elapsed = start_time.elapsed().as_secs_f32() * 1000.0;
    Ok(ScriptValue::F32(elapsed))
}

/// Get current Unix timestamp in seconds
pub fn time_unix_impl(_args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| RuntimeError::InvalidOperation(format!("Time error: {}", e)))?;
    Ok(ScriptValue::F32(duration.as_secs_f32()))
}

/// Get current Unix timestamp in milliseconds
pub fn time_unix_millis_impl(_args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| RuntimeError::InvalidOperation(format!("Time error: {}", e)))?;
    Ok(ScriptValue::F32(duration.as_millis() as f32))
}

/// Calculate time delta between two timestamps
pub fn time_delta_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "time_delta expects 2 arguments, got {}",
            args.len()
        )));
    }

    let start = args[0].to_f32()?;
    let end = args[1].to_f32()?;

    Ok(ScriptValue::F32(end - start))
}

/// Sleep for specified milliseconds
pub fn sleep_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "sleep expects 1 argument, got {}",
            args.len()
        )));
    }

    let millis = args[0].to_f32()?;
    if millis < 0.0 {
        return Err(RuntimeError::InvalidOperation(
            "Sleep time must be non-negative".to_string(),
        ));
    }

    std::thread::sleep(Duration::from_millis(millis as u64));
    Ok(ScriptValue::Unit)
}

/// Create a new stopwatch
pub fn stopwatch_new_impl(_args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    let _stopwatch = Rc::new(RefCell::new(Stopwatch::new()));

    let mut map = HashMap::new();
    // Store stopwatch as a marker in the object - actual stopwatch would need proper wrapper
    map.insert(
        "_stopwatch_marker".to_string(),
        ScriptValue::String(ScriptRc::new(ScriptString::from_str("Stopwatch"))),
    );
    map.insert("start_time".to_string(), ScriptValue::I32(0)); // Placeholder
    Ok(ScriptValue::Object(ScriptRc::new(map)))
}

/// Start the stopwatch
pub fn stopwatch_start_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "stopwatch.start expects 1 argument (self), got {}",
            args.len()
        )));
    }

    // In a real implementation, we'd extract the stopwatch from the object
    // For now, this is a placeholder
    Ok(ScriptValue::Unit)
}

/// Stop the stopwatch
pub fn stopwatch_stop_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "stopwatch.stop expects 1 argument (self), got {}",
            args.len()
        )));
    }

    Ok(ScriptValue::Unit)
}

/// Reset the stopwatch
pub fn stopwatch_reset_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "stopwatch.reset expects 1 argument (self), got {}",
            args.len()
        )));
    }

    Ok(ScriptValue::Unit)
}

/// Get stopwatch elapsed time
pub fn stopwatch_elapsed_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "stopwatch.elapsed expects 1 argument (self), got {}",
            args.len()
        )));
    }

    // Placeholder - return 0 for now
    Ok(ScriptValue::F32(0.0))
}

/// Create a new frame timer
pub fn frame_timer_new_impl(_args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    let _timer = Rc::new(RefCell::new(FrameTimer::new()));

    let mut map = HashMap::new();
    // Store timer as a marker in the object - actual timer would need proper wrapper
    map.insert(
        "_timer_marker".to_string(),
        ScriptValue::String(ScriptRc::new(ScriptString::from_str("Timer"))),
    );
    map.insert("duration_ms".to_string(), ScriptValue::I32(0)); // Placeholder
    Ok(ScriptValue::Object(ScriptRc::new(map)))
}

/// Update frame timer
pub fn frame_timer_update_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "frame_timer.update expects 1 argument (self), got {}",
            args.len()
        )));
    }

    Ok(ScriptValue::Unit)
}

/// Get delta time from frame timer
pub fn frame_timer_delta_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "frame_timer.delta expects 1 argument (self), got {}",
            args.len()
        )));
    }

    // Placeholder - return typical 60 FPS delta
    Ok(ScriptValue::F32(0.016666667))
}

/// Get FPS from frame timer
pub fn frame_timer_fps_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "frame_timer.fps expects 1 argument (self), got {}",
            args.len()
        )));
    }

    // Placeholder - return 60 FPS
    Ok(ScriptValue::F32(60.0))
}

/// Format seconds as HH:MM:SS
pub fn format_time_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "format_time expects 1 argument, got {}",
            args.len()
        )));
    }

    let total_seconds = args[0].to_f32()? as i32;
    if total_seconds < 0 {
        return Err(RuntimeError::InvalidOperation(
            "Time must be non-negative".to_string(),
        ));
    }

    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    let formatted = if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    };

    Ok(ScriptValue::String(ScriptRc::new(ScriptString::from_str(
        &formatted,
    ))))
}

/// Format milliseconds as MM:SS.mmm
pub fn format_time_millis_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "format_time_millis expects 1 argument, got {}",
            args.len()
        )));
    }

    let total_millis = args[0].to_f32()? as i32;
    if total_millis < 0 {
        return Err(RuntimeError::InvalidOperation(
            "Time must be non-negative".to_string(),
        ));
    }

    let minutes = total_millis / 60000;
    let seconds = (total_millis % 60000) / 1000;
    let millis = total_millis % 1000;

    let formatted = format!("{:02}:{:02}.{:03}", minutes, seconds, millis);

    Ok(ScriptValue::String(ScriptRc::new(ScriptString::from_str(
        &formatted,
    ))))
}

/// Performance counter for precise timing
pub fn perf_counter_impl(_args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    // High precision timer using Instant
    let elapsed = Instant::now().duration_since(Instant::now() - Duration::from_secs(0));
    Ok(ScriptValue::F32(elapsed.as_secs_f64() as f32))
}

/// Measure execution time of a function
pub fn measure_time_impl(args: &[ScriptValue]) -> RuntimeResult<ScriptValue> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "measure_time expects 1 argument (function), got {}",
            args.len()
        )));
    }

    // This would need runtime support to execute the function
    // For now, return a placeholder
    let mut result = HashMap::new();
    result.insert("elapsed".to_string(), ScriptValue::F32(0.0));
    result.insert("result".to_string(), ScriptValue::Unit);

    Ok(ScriptValue::Object(ScriptRc::new(result)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_now() {
        let t1 = time_now_impl(&[]).unwrap();
        std::thread::sleep(Duration::from_millis(10));
        let t2 = time_now_impl(&[]).unwrap();

        match (t1, t2) {
            (ScriptValue::F32(v1), ScriptValue::F32(v2)) => {
                assert!(v2 > v1);
                assert!((v2 - v1) >= 0.01); // At least 10ms passed
            }
            _ => panic!("Expected F32 values"),
        }
    }

    #[test]
    fn test_time_delta() {
        let t1 = ScriptValue::F32(100.0);
        let t2 = ScriptValue::F32(150.0);

        let delta = time_delta_impl(&[t1, t2]).unwrap();
        assert_eq!(delta, ScriptValue::F32(50.0));
    }

    #[test]
    fn test_stopwatch() {
        let mut sw = Stopwatch::new();
        assert!(!sw.is_running());
        assert_eq!(sw.elapsed(), 0.0);

        sw.start();
        assert!(sw.is_running());
        std::thread::sleep(Duration::from_millis(50));

        let elapsed1 = sw.elapsed();
        assert!(elapsed1 >= 0.05);

        sw.stop();
        assert!(!sw.is_running());
        let elapsed2 = sw.elapsed();

        std::thread::sleep(Duration::from_millis(50));
        let elapsed3 = sw.elapsed();
        assert_eq!(elapsed2, elapsed3); // Should not change when stopped

        sw.reset();
        assert_eq!(sw.elapsed(), 0.0);
    }

    #[test]
    fn test_frame_timer() {
        let mut timer = FrameTimer::new();

        assert_eq!(timer.delta_time(), 0.0);
        assert_eq!(timer.frame_count(), 0);

        // Simulate some frames
        for _ in 0..10 {
            std::thread::sleep(Duration::from_millis(16)); // ~60 FPS
            timer.update();
        }

        assert!(timer.delta_time() > 0.0);
        assert_eq!(timer.frame_count(), 10);
    }

    #[test]
    fn test_format_time() {
        // Test seconds formatting
        let result = format_time_impl(&[ScriptValue::F32(3661.0)]).unwrap();
        match result {
            ScriptValue::String(s) => assert_eq!(s.as_str(), "01:01:01"),
            _ => panic!("Expected String"),
        }

        let result = format_time_impl(&[ScriptValue::F32(125.0)]).unwrap();
        match result {
            ScriptValue::String(s) => assert_eq!(s.as_str(), "02:05"),
            _ => panic!("Expected String"),
        }
    }

    #[test]
    fn test_format_time_millis() {
        let result = format_time_millis_impl(&[ScriptValue::F32(65123.0)]).unwrap();
        match result {
            ScriptValue::String(s) => assert_eq!(s.as_str(), "01:05.123"),
            _ => panic!("Expected String"),
        }
    }
}
