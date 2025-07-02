use crate::error::{Error, Result};
use crate::source::Span;
use std::fmt;

/// Assertion library for Script tests
pub struct Assertion;

/// Error type for assertion failures
#[derive(Debug, Clone)]
pub struct AssertionError {
    pub message: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub location: Option<Span>,
}

impl AssertionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            expected: None,
            actual: None,
            location: None,
        }
    }

    pub fn with_values(mut self, expected: impl fmt::Display, actual: impl fmt::Display) -> Self {
        self.expected = Some(expected.to_string());
        self.actual = Some(actual.to_string());
        self
    }

    pub fn with_location(mut self, span: Span) -> Self {
        self.location = Some(span);
        self
    }
}

impl fmt::Display for AssertionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Assertion failed: {}", self.message)?;
        if let (Some(expected), Some(actual)) = (&self.expected, &self.actual) {
            write!(f, "\n  Expected: {}\n  Actual: {}", expected, actual)?;
        }
        Ok(())
    }
}

impl std::error::Error for AssertionError {}

impl From<AssertionError> for Error {
    fn from(err: AssertionError) -> Self {
        match err.location {
            Some(span) => Error::runtime(err.to_string()).with_location(span.start),
            None => Error::runtime(err.to_string()),
        }
    }
}

/// Core assertion functions that will be available in the Script runtime
impl Assertion {
    /// Assert that a condition is true
    pub fn assert(condition: bool, message: Option<&str>) -> Result<()> {
        if !condition {
            let msg = message.unwrap_or("Assertion failed");
            Err(AssertionError::new(msg).into())
        } else {
            Ok(())
        }
    }

    /// Assert that a condition is true
    pub fn assert_true(value: bool) -> Result<()> {
        if !value {
            Err(AssertionError::new("Expected true, got false").into())
        } else {
            Ok(())
        }
    }

    /// Assert that a condition is false
    pub fn assert_false(value: bool) -> Result<()> {
        if value {
            Err(AssertionError::new("Expected false, got true").into())
        } else {
            Ok(())
        }
    }

    /// Assert that two values are equal
    pub fn assert_eq<T: PartialEq + fmt::Display>(expected: &T, actual: &T) -> Result<()> {
        if expected != actual {
            Err(AssertionError::new("Values are not equal")
                .with_values(expected, actual)
                .into())
        } else {
            Ok(())
        }
    }

    /// Assert that two values are not equal
    pub fn assert_ne<T: PartialEq + fmt::Display>(expected: &T, actual: &T) -> Result<()> {
        if expected == actual {
            Err(AssertionError::new("Values are equal")
                .with_values(expected, actual)
                .into())
        } else {
            Ok(())
        }
    }

    /// Assert that a value is greater than another
    pub fn assert_gt<T: PartialOrd + fmt::Display>(left: &T, right: &T) -> Result<()> {
        if !(left > right) {
            Err(AssertionError::new("Left value is not greater than right")
                .with_values(left, right)
                .into())
        } else {
            Ok(())
        }
    }

    /// Assert that a value is greater than or equal to another
    pub fn assert_ge<T: PartialOrd + fmt::Display>(left: &T, right: &T) -> Result<()> {
        if !(left >= right) {
            Err(
                AssertionError::new("Left value is not greater than or equal to right")
                    .with_values(left, right)
                    .into(),
            )
        } else {
            Ok(())
        }
    }

    /// Assert that a value is less than another
    pub fn assert_lt<T: PartialOrd + fmt::Display>(left: &T, right: &T) -> Result<()> {
        if !(left < right) {
            Err(AssertionError::new("Left value is not less than right")
                .with_values(left, right)
                .into())
        } else {
            Ok(())
        }
    }

    /// Assert that a value is less than or equal to another
    pub fn assert_le<T: PartialOrd + fmt::Display>(left: &T, right: &T) -> Result<()> {
        if !(left <= right) {
            Err(
                AssertionError::new("Left value is not less than or equal to right")
                    .with_values(left, right)
                    .into(),
            )
        } else {
            Ok(())
        }
    }

    /// Assert that a string contains a substring
    pub fn assert_contains(haystack: &str, needle: &str) -> Result<()> {
        if !haystack.contains(needle) {
            Err(
                AssertionError::new(format!("String does not contain '{}'", needle))
                    .with_values(format!("contains '{}'", needle), format!("'{}'", haystack))
                    .into(),
            )
        } else {
            Ok(())
        }
    }

    /// Assert that a collection is empty
    pub fn assert_empty<T>(collection: &[T]) -> Result<()> {
        if !collection.is_empty() {
            Err(AssertionError::new("Collection is not empty")
                .with_values("empty", format!("length {}", collection.len()))
                .into())
        } else {
            Ok(())
        }
    }

    /// Assert that a collection has a specific length
    pub fn assert_len<T>(collection: &[T], expected_len: usize) -> Result<()> {
        let actual_len = collection.len();
        if actual_len != expected_len {
            Err(AssertionError::new("Collection has wrong length")
                .with_values(expected_len, actual_len)
                .into())
        } else {
            Ok(())
        }
    }

    /// Assert that a value is approximately equal to another (for floating point)
    pub fn assert_approx_eq(expected: f64, actual: f64, tolerance: Option<f64>) -> Result<()> {
        let tolerance = tolerance.unwrap_or(1e-10);
        let diff = (expected - actual).abs();

        if diff > tolerance {
            Err(
                AssertionError::new(format!("Values differ by more than {}", tolerance))
                    .with_values(expected, actual)
                    .into(),
            )
        } else {
            Ok(())
        }
    }

    /// Assert that a closure panics
    pub fn assert_panic<F: FnOnce() + std::panic::UnwindSafe>(
        f: F,
        expected_msg: Option<&str>,
    ) -> Result<()> {
        use std::panic;

        let result = panic::catch_unwind(f);

        match result {
            Err(panic_info) => {
                if let Some(expected) = expected_msg {
                    // Check if panic message contains expected string
                    let panic_msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                        s.as_str()
                    } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                        s
                    } else {
                        return Ok(()); // Panicked but couldn't get message
                    };

                    if !panic_msg.contains(expected) {
                        Err(
                            AssertionError::new("Panic message does not contain expected text")
                                .with_values(expected, panic_msg)
                                .into(),
                        )
                    } else {
                        Ok(())
                    }
                } else {
                    Ok(()) // Panicked as expected
                }
            }
            Ok(_) => Err(AssertionError::new("Expected panic but none occurred").into()),
        }
    }
}

/// Runtime assertion functions for Script values
pub mod runtime_assertions {
    use super::*;
    use crate::runtime::Value;

    /// Assert equality for Script runtime values
    pub fn assert_value_eq(expected: &Value, actual: &Value) -> Result<()> {
        if !values_equal(expected, actual) {
            Err(AssertionError::new("Values are not equal")
                .with_values(format!("{:?}", expected), format!("{:?}", actual))
                .into())
        } else {
            Ok(())
        }
    }

    /// Check if two Script values are equal
    fn values_equal(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => (a - b).abs() < f64::EPSILON,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (Value::Array(a), Value::Array(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| values_equal(x, y))
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_true() {
        assert!(Assertion::assert_true(true).is_ok());
        assert!(Assertion::assert_true(false).is_err());
    }

    #[test]
    fn test_assert_eq() {
        assert!(Assertion::assert_eq(&42, &42).is_ok());
        assert!(Assertion::assert_eq(&"hello", &"hello").is_ok());
        assert!(Assertion::assert_eq(&42, &43).is_err());
    }

    #[test]
    fn test_assert_contains() {
        assert!(Assertion::assert_contains("hello world", "world").is_ok());
        assert!(Assertion::assert_contains("hello world", "foo").is_err());
    }

    #[test]
    fn test_assert_approx_eq() {
        assert!(Assertion::assert_approx_eq(1.0, 1.0000000001, None).is_ok());
        assert!(Assertion::assert_approx_eq(1.0, 1.1, Some(0.05)).is_err()); // 0.1 > 0.05 tolerance
        assert!(Assertion::assert_approx_eq(1.0, 1.01, Some(0.1)).is_ok()); // 0.01 < 0.1 tolerance
    }
}
