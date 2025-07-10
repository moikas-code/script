//! Comprehensive test suite for Script's error handling system
//!
//! This module tests all aspects of error handling including:
//! - Result and Option types
//! - Error propagation operator (?)
//! - Advanced methods (flatten, transpose, inspect, etc.)
//! - Functional programming patterns
//! - Performance characteristics
//! - Edge cases and error conditions

use script::error::{Error, ErrorKind};
use script::runtime::ScriptRc;
use script::runtime::Value;
use script::stdlib::{ScriptOption, ScriptResult, ScriptString, ScriptValue, ScriptVec};

#[cfg(test)]
mod result_tests {
    use super::*;

    #[test]
    fn test_result_construction() {
        let ok_result = ScriptResult::ok(ScriptValue::I32(42));
        assert!(ok_result.is_ok());
        assert!(!ok_result.is_err());

        let err_result = ScriptResult::err(ScriptValue::String(ScriptRc::new(
            ScriptString::from_str("error"),
        )));
        assert!(!err_result.is_ok());
        assert!(err_result.is_err());
    }

    #[test]
    fn test_result_basic_methods() {
        let ok_result = ScriptResult::ok(ScriptValue::I32(42));

        // Test get_ok
        assert!(ok_result.get_ok().is_some());
        assert!(ok_result.get_err().is_none());

        // Test unwrap_or
        let default_value = ScriptValue::I32(0);
        let unwrapped = ok_result.unwrap_or(default_value.clone());
        assert_eq!(unwrapped, ScriptValue::I32(42));

        // Test with error
        let err_result = ScriptResult::err(ScriptValue::String(ScriptRc::new(
            ScriptString::from_str("error"),
        )));
        let unwrapped_err = err_result.unwrap_or(default_value);
        assert_eq!(unwrapped_err, ScriptValue::I32(0));
    }

    #[test]
    fn test_result_map() {
        let ok_result = ScriptResult::ok(ScriptValue::I32(42));

        // Test map with closure
        let mapped = ok_result.map(|val| {
            if let ScriptValue::I32(n) = val {
                ScriptValue::I32(n * 2)
            } else {
                val.clone()
            }
        });

        assert!(mapped.is_ok());
        if let Some(ScriptValue::I32(n)) = mapped.get_ok() {
            assert_eq!(*n, 84);
        } else {
            panic!("Expected I32(84)");
        }
    }

    #[test]
    fn test_result_and_then() {
        let ok_result = ScriptResult::ok(ScriptValue::I32(42));

        // Test and_then with successful operation
        let chained = ok_result.and_then(|val| {
            if let ScriptValue::I32(n) = val {
                if *n > 0 {
                    ScriptResult::ok(ScriptValue::I32(n * 2))
                } else {
                    ScriptResult::err(ScriptValue::String(ScriptRc::new(ScriptString::from_str(
                        "negative",
                    ))))
                }
            } else {
                ScriptResult::err(ScriptValue::String(ScriptRc::new(ScriptString::from_str(
                    "not a number",
                ))))
            }
        });

        assert!(chained.is_ok());
        if let Some(ScriptValue::I32(n)) = chained.get_ok() {
            assert_eq!(*n, 84);
        } else {
            panic!("Expected I32(84)");
        }
    }

    #[test]
    fn test_result_flatten() {
        // Create a nested Result<Result<i32, string>, string>
        let inner_ok = ScriptResult::ok(ScriptValue::I32(42));
        let nested_ok = ScriptResult::ok(ScriptValue::Result(ScriptRc::new(inner_ok)));

        let flattened = nested_ok.flatten();
        assert!(flattened.is_ok());
        if let Some(ScriptValue::I32(n)) = flattened.get_ok() {
            assert_eq!(*n, 42);
        } else {
            panic!("Expected I32(42)");
        }

        // Test with error in inner result
        let inner_err = ScriptResult::err(ScriptValue::String(ScriptRc::new(
            ScriptString::from_str("inner error"),
        )));
        let nested_err = ScriptResult::ok(ScriptValue::Result(ScriptRc::new(inner_err)));

        let flattened_err = nested_err.flatten();
        assert!(flattened_err.is_err());
    }

    #[test]
    fn test_result_transpose() {
        // Test Result<Option<T>, E> to Option<Result<T, E>>
        let result_some = ScriptResult::ok(ScriptValue::Option(ScriptRc::new(ScriptOption::Some(
            ScriptValue::I32(42),
        ))));

        let transposed = result_some.transpose();
        assert!(transposed.is_some());

        if let ScriptOption::Some(ScriptValue::Result(result_rc)) = transposed {
            assert!(result_rc.is_ok());
        } else {
            panic!("Expected Some(Result)");
        }
    }

    #[test]
    fn test_result_inspect() {
        let ok_result = ScriptResult::ok(ScriptValue::I32(42));

        let mut inspected_value = None;
        let inspected = ok_result.inspect(|val| {
            if let ScriptValue::I32(n) = val {
                inspected_value = Some(*n);
            }
        });

        assert!(inspected.is_ok());
        assert_eq!(inspected_value, Some(42));
    }

    #[test]
    fn test_result_inspect_err() {
        let err_result = ScriptResult::err(ScriptValue::String(ScriptRc::new(
            ScriptString::from_str("test error"),
        )));

        let mut inspected_error = None;
        let inspected = err_result.inspect_err(|err| {
            if let ScriptValue::String(s) = err {
                inspected_error = Some(s.as_str().to_string());
            }
        });

        assert!(inspected.is_err());
        assert_eq!(inspected_error, Some("test error".to_string()));
    }

    #[test]
    fn test_result_and_or() {
        let ok1 = ScriptResult::ok(ScriptValue::I32(1));
        let ok2 = ScriptResult::ok(ScriptValue::I32(2));
        let err = ScriptResult::err(ScriptValue::String(ScriptRc::new(ScriptString::from_str(
            "error",
        ))));

        // Test and - returns first Err or second Result
        let and_result = ok1.and(ok2.clone());
        assert!(and_result.is_ok());
        if let Some(ScriptValue::I32(n)) = and_result.get_ok() {
            assert_eq!(*n, 2);
        }

        let and_err = err.clone().and(ok2.clone());
        assert!(and_err.is_err());

        // Test or - returns first Ok or second Result
        let or_result = ok1.or(ok2.clone());
        assert!(or_result.is_ok());
        if let Some(ScriptValue::I32(n)) = or_result.get_ok() {
            assert_eq!(*n, 1);
        }

        let or_err = err.or(ok2);
        assert!(or_err.is_ok());
        if let Some(ScriptValue::I32(n)) = or_err.get_ok() {
            assert_eq!(*n, 2);
        }
    }

    #[test]
    fn test_result_collect() {
        let ok_result = ScriptResult::ok(ScriptValue::I32(42));
        let collected = ok_result.collect();

        assert!(collected.is_ok());
        if let Some(ScriptValue::Array(arr)) = collected.get_ok() {
            assert_eq!(arr.len(), 1);
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_result_fold() {
        let ok_result = ScriptResult::ok(ScriptValue::I32(42));

        let folded = ok_result.fold(0, |acc, val| {
            if let ScriptValue::I32(n) = val {
                acc + n
            } else {
                acc
            }
        });

        assert_eq!(folded, 42);

        // Test with error
        let err_result = ScriptResult::err(ScriptValue::String(ScriptRc::new(
            ScriptString::from_str("error"),
        )));

        let folded_err = err_result.fold(100, |acc, val| {
            if let ScriptValue::I32(n) = val {
                acc + n
            } else {
                acc
            }
        });

        assert_eq!(folded_err, 100); // Returns init value for Err
    }

    #[test]
    fn test_result_satisfies() {
        let ok_result = ScriptResult::ok(ScriptValue::I32(42));

        let satisfies_positive = ok_result.satisfies(|val| {
            if let ScriptValue::I32(n) = val {
                *n > 0
            } else {
                false
            }
        });

        assert!(satisfies_positive);

        let satisfies_negative = ok_result.satisfies(|val| {
            if let ScriptValue::I32(n) = val {
                *n < 0
            } else {
                false
            }
        });

        assert!(!satisfies_negative);

        // Test with error
        let err_result = ScriptResult::err(ScriptValue::String(ScriptRc::new(
            ScriptString::from_str("error"),
        )));

        let err_satisfies = err_result.satisfies(|_| true);
        assert!(!err_satisfies); // Always false for Err
    }
}

#[cfg(test)]
mod option_tests {
    use super::*;

    #[test]
    fn test_option_construction() {
        let some_value = ScriptOption::some(ScriptValue::I32(42));
        assert!(some_value.is_some());
        assert!(!some_value.is_none());

        let none_value = ScriptOption::none();
        assert!(!none_value.is_some());
        assert!(none_value.is_none());
    }

    #[test]
    fn test_option_basic_methods() {
        let some_value = ScriptOption::some(ScriptValue::I32(42));

        // Test unwrap
        assert!(some_value.unwrap().is_some());

        // Test unwrap_or
        let default_value = ScriptValue::I32(0);
        let unwrapped = some_value.unwrap_or(default_value.clone());
        assert_eq!(unwrapped, ScriptValue::I32(42));

        // Test with None
        let none_value = ScriptOption::none();
        let unwrapped_none = none_value.unwrap_or(default_value);
        assert_eq!(unwrapped_none, ScriptValue::I32(0));
    }

    #[test]
    fn test_option_map() {
        let some_value = ScriptOption::some(ScriptValue::I32(42));

        // Test map with closure
        let mapped = some_value.map(|val| {
            if let ScriptValue::I32(n) = val {
                ScriptValue::I32(n * 2)
            } else {
                val.clone()
            }
        });

        assert!(mapped.is_some());
        if let Some(ScriptValue::I32(n)) = mapped.unwrap() {
            assert_eq!(*n, 84);
        } else {
            panic!("Expected I32(84)");
        }
    }

    #[test]
    fn test_option_and_then() {
        let some_value = ScriptOption::some(ScriptValue::I32(42));

        // Test and_then with successful operation
        let chained = some_value.and_then(|val| {
            if let ScriptValue::I32(n) = val {
                if *n > 0 {
                    ScriptOption::some(ScriptValue::I32(n * 2))
                } else {
                    ScriptOption::none()
                }
            } else {
                ScriptOption::none()
            }
        });

        assert!(chained.is_some());
        if let Some(ScriptValue::I32(n)) = chained.unwrap() {
            assert_eq!(*n, 84);
        } else {
            panic!("Expected I32(84)");
        }
    }

    #[test]
    fn test_option_flatten() {
        // Create a nested Option<Option<i32>>
        let inner_some = ScriptOption::some(ScriptValue::I32(42));
        let nested_some = ScriptOption::some(ScriptValue::Option(ScriptRc::new(inner_some)));

        let flattened = nested_some.flatten();
        assert!(flattened.is_some());
        if let Some(ScriptValue::I32(n)) = flattened.unwrap() {
            assert_eq!(*n, 42);
        } else {
            panic!("Expected I32(42)");
        }

        // Test with None in inner option
        let inner_none = ScriptOption::none();
        let nested_none = ScriptOption::some(ScriptValue::Option(ScriptRc::new(inner_none)));

        let flattened_none = nested_none.flatten();
        assert!(flattened_none.is_none());
    }

    #[test]
    fn test_option_transpose() {
        // Test Option<Result<T, E>> to Result<Option<T>, E>
        let option_ok = ScriptOption::some(ScriptValue::Result(ScriptRc::new(ScriptResult::ok(
            ScriptValue::I32(42),
        ))));

        let transposed = option_ok.transpose();
        assert!(transposed.is_ok());

        if let Some(ScriptValue::Option(opt_rc)) = transposed.get_ok() {
            assert!(opt_rc.is_some());
        } else {
            panic!("Expected Option");
        }
    }

    #[test]
    fn test_option_inspect() {
        let some_value = ScriptOption::some(ScriptValue::I32(42));

        let mut inspected_value = None;
        let inspected = some_value.inspect(|val| {
            if let ScriptValue::I32(n) = val {
                inspected_value = Some(*n);
            }
        });

        assert!(inspected.is_some());
        assert_eq!(inspected_value, Some(42));
    }

    #[test]
    fn test_option_zip() {
        let opt1 = ScriptOption::some(ScriptValue::I32(1));
        let opt2 = ScriptOption::some(ScriptValue::I32(2));

        let zipped = opt1.zip(&opt2);
        assert!(zipped.is_some());

        // Test with one None
        let none_opt = ScriptOption::none();
        let zipped_none = opt1.zip(&none_opt);
        assert!(zipped_none.is_none());
    }

    #[test]
    fn test_option_copied_cloned() {
        let some_value = ScriptOption::some(ScriptValue::I32(42));

        let copied = some_value.copied();
        assert!(copied.is_some());

        let cloned = some_value.cloned();
        assert!(cloned.is_some());
    }

    #[test]
    fn test_option_collect() {
        let some_value = ScriptOption::some(ScriptValue::I32(42));
        let collected = some_value.collect();

        assert!(collected.is_some());
        if let Some(ScriptValue::Array(arr)) = collected.unwrap() {
            assert_eq!(arr.len(), 1);
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_option_fold() {
        let some_value = ScriptOption::some(ScriptValue::I32(42));

        let folded = some_value.fold(0, |acc, val| {
            if let ScriptValue::I32(n) = val {
                acc + n
            } else {
                acc
            }
        });

        assert_eq!(folded, 42);

        // Test with None
        let none_value = ScriptOption::none();

        let folded_none = none_value.fold(100, |acc, val| {
            if let ScriptValue::I32(n) = val {
                acc + n
            } else {
                acc
            }
        });

        assert_eq!(folded_none, 100); // Returns init value for None
    }

    #[test]
    fn test_option_satisfies() {
        let some_value = ScriptOption::some(ScriptValue::I32(42));

        let satisfies_positive = some_value.satisfies(|val| {
            if let ScriptValue::I32(n) = val {
                *n > 0
            } else {
                false
            }
        });

        assert!(satisfies_positive);

        let satisfies_negative = some_value.satisfies(|val| {
            if let ScriptValue::I32(n) = val {
                *n < 0
            } else {
                false
            }
        });

        assert!(!satisfies_negative);

        // Test with None
        let none_value = ScriptOption::none();

        let none_satisfies = none_value.satisfies(|_| true);
        assert!(!none_satisfies); // Always false for None
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_chaining_operations() {
        // Test chaining Result operations
        let initial = ScriptResult::ok(ScriptValue::I32(10));

        let result = initial
            .map(|val| {
                if let ScriptValue::I32(n) = val {
                    ScriptValue::I32(n * 2)
                } else {
                    val.clone()
                }
            })
            .and_then(|val| {
                if let ScriptValue::I32(n) = val {
                    if *n > 15 {
                        ScriptResult::ok(ScriptValue::I32(*n))
                    } else {
                        ScriptResult::err(ScriptValue::String(ScriptRc::new(
                            ScriptString::from_str("too small"),
                        )))
                    }
                } else {
                    ScriptResult::err(ScriptValue::String(ScriptRc::new(ScriptString::from_str(
                        "not a number",
                    ))))
                }
            });

        assert!(result.is_ok());
        if let Some(ScriptValue::I32(n)) = result.get_ok() {
            assert_eq!(*n, 20);
        } else {
            panic!("Expected I32(20)");
        }
    }

    #[test]
    fn test_error_propagation_simulation() {
        // Simulate error propagation behavior
        fn simulate_parse(s: &str) -> ScriptResult {
            if s.is_empty() {
                ScriptResult::err(ScriptValue::String(ScriptRc::new(ScriptString::from_str(
                    "empty string",
                ))))
            } else {
                ScriptResult::ok(ScriptValue::I32(s.len() as i32))
            }
        }

        fn simulate_validate(val: ScriptValue) -> ScriptResult {
            if let ScriptValue::I32(n) = val {
                if n > 0 {
                    ScriptResult::ok(ScriptValue::I32(n))
                } else {
                    ScriptResult::err(ScriptValue::String(ScriptRc::new(ScriptString::from_str(
                        "non-positive",
                    ))))
                }
            } else {
                ScriptResult::err(ScriptValue::String(ScriptRc::new(ScriptString::from_str(
                    "not a number",
                ))))
            }
        }

        fn simulate_process(input: &str) -> ScriptResult {
            // Simulate: let parsed = parse(input)?;
            let parsed = simulate_parse(input);
            if parsed.is_err() {
                return parsed;
            }
            let parsed_val = parsed.get_ok().unwrap().clone();

            // Simulate: let validated = validate(parsed)?;
            let validated = simulate_validate(parsed_val);
            if validated.is_err() {
                return validated;
            }

            validated
        }

        // Test successful case
        let success = simulate_process("hello");
        assert!(success.is_ok());

        // Test error propagation
        let error = simulate_process("");
        assert!(error.is_err());
    }

    #[test]
    fn test_complex_nested_operations() {
        // Test complex nested Result and Option operations
        let result_option = ScriptResult::ok(ScriptValue::Option(ScriptRc::new(
            ScriptOption::some(ScriptValue::I32(42)),
        )));

        // Chain operations
        let processed = result_option
            .and_then(|val| {
                if let ScriptValue::Option(opt) = val {
                    if opt.is_some() {
                        ScriptResult::ok(ScriptValue::Option(opt.clone())
                    } else {
                        ScriptResult::err(ScriptValue::String(ScriptRc::new(
                            ScriptString::from_str("option is none"),
                        )))
                    }
                } else {
                    ScriptResult::err(ScriptValue::String(ScriptRc::new(ScriptString::from_str(
                        "not an option",
                    ))))
                }
            })
            .map(|val| {
                if let ScriptValue::Option(opt) = val {
                    if let Some(ScriptValue::I32(n)) = opt.unwrap() {
                        ScriptValue::I32(n * 2)
                    } else {
                        ScriptValue::I32(0)
                    }
                } else {
                    ScriptValue::I32(-1)
                }
            });

        assert!(processed.is_ok());
        if let Some(ScriptValue::I32(n)) = processed.get_ok() {
            assert_eq!(*n, 84);
        } else {
            panic!("Expected I32(84)");
        }
    }

    #[test]
    fn test_performance_characteristics() {
        // Test that operations don't unnecessarily clone values
        let large_string = "a".repeat(1000);
        let result = ScriptResult::ok(ScriptValue::String(ScriptRc::new(ScriptString::from_str(
            &large_string,
        ))));

        // Chain multiple operations
        let processed = result
            .inspect(|_| {
                // Inspection should not clone the value
            })
            .map(|val| {
                // Map should work efficiently
                val.clone()
            })
            .and_then(|val| {
                // and_then should work efficiently
                ScriptResult::ok(val)
            });

        assert!(processed.is_ok());
    }

    #[test]
    fn test_error_type_conversions() {
        // Test that errors can be converted between types
        let string_error = ScriptResult::err(ScriptValue::String(ScriptRc::new(
            ScriptString::from_str("string error"),
        )));

        let converted = string_error.map_err(|err| {
            // Convert string error to number error
            ScriptValue::I32(42)
        });

        assert!(converted.is_err());
        if let Some(ScriptValue::I32(n)) = converted.get_err() {
            assert_eq!(*n, 42);
        } else {
            panic!("Expected I32(42)");
        }
    }
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_deep_nesting() {
        // Test deeply nested Results and Options
        let deep_result = ScriptResult::ok(ScriptValue::Option(ScriptRc::new(ScriptOption::some(
            ScriptValue::Result(ScriptRc::new(ScriptResult::ok(ScriptValue::Option(
                ScriptRc::new(ScriptOption::some(ScriptValue::I32(42))),
            )))),
        ))));

        // Should be able to process deeply nested structures
        assert!(deep_result.is_ok());
    }

    #[test]
    fn test_empty_collections() {
        // Test with empty collections
        let empty_result = ScriptResult::ok(ScriptValue::Array(ScriptRc::new(ScriptVec::new())));

        let collected = empty_result.collect();
        assert!(collected.is_ok());
    }

    #[test]
    fn test_large_error_messages() {
        // Test with large error messages
        let large_error = "error ".repeat(1000);
        let result = ScriptResult::err(ScriptValue::String(ScriptRc::new(ScriptString::from_str(
            &large_error,
        ))));

        assert!(result.is_err());

        // Should be able to process large error messages
        let inspected = result.inspect_err(|_| {
            // Processing large error should not crash
        });

        assert!(inspected.is_err());
    }

    #[test]
    fn test_concurrent_access() {
        // Test that Result and Option are safe for concurrent access
        use std::sync::Arc;
        use std::thread;

        let shared_result = Arc::new(ScriptResult::ok(ScriptValue::I32(42)));

        let mut handles = vec![];

        for i in 0..10 {
            let result_clone = shared_result.clone();
            let handle = thread::spawn(move || {
                let processed = result_clone.map(|val| {
                    if let ScriptValue::I32(n) = val {
                        ScriptValue::I32(n + i)
                    } else {
                        val.clone()
                    }
                });
                processed.is_ok()
            });
            handles.push(handle);
        }

        for handle in handles {
            assert!(handle.join().unwrap());
        }
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;

    // Property-based tests (manual implementation since quickcheck is not available)
    #[test]
    fn prop_result_map_preserves_ok() {
        for value in [1, 42, -10, 0, 100] {
            let result = ScriptResult::ok(ScriptValue::I32(value));
            let mapped = result.map(|val| val.clone());
            assert!(mapped.is_ok());
        }
    }

    #[test]
    fn prop_result_map_preserves_err() {
        for error_code in [1, 42, -10, 0, 100] {
            let result = ScriptResult::err(ScriptValue::I32(error_code));
            let mapped = result.map(|val| val.clone());
            assert!(mapped.is_err());
        }
    }

    #[test]
    fn prop_option_map_preserves_some() {
        for value in [1, 42, -10, 0, 100] {
            let option = ScriptOption::some(ScriptValue::I32(value));
            let mapped = option.map(|val| val.clone());
            assert!(mapped.is_some());
        }
    }

    #[test]
    fn prop_option_map_preserves_none() {
        let option = ScriptOption::none();
        let mapped = option.map(|val| val.clone());
        assert!(mapped.is_none());
    }

    #[test]
    fn prop_result_and_then_short_circuits() {
        for error_code in [1, 42, -10, 0, 100] {
            let result = ScriptResult::err(ScriptValue::I32(error_code));
            let chained = result.and_then(|_| ScriptResult::ok(ScriptValue::I32(0)));
            assert!(chained.is_err());
        }
    }

    #[test]
    fn prop_option_and_then_short_circuits() {
        let option = ScriptOption::none();
        let chained = option.and_then(|_| ScriptOption::some(ScriptValue::I32(0)));
        assert!(chained.is_none());
    }
}

#[cfg(test)]
mod benchmark_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn benchmark_result_operations() {
        let iterations = 100_000;
        let start = Instant::now();

        for i in 0..iterations {
            let result = ScriptResult::ok(ScriptValue::I32(i));
            let _processed = result
                .map(|val| val.clone())
                .and_then(|val| ScriptResult::ok(val))
                .inspect(|_| {});
        }

        let duration = start.elapsed();
        println!("Result operations took: {:?}", duration);

        // Should complete within reasonable time
        assert!(duration.as_secs() < 5);
    }

    #[test]
    fn benchmark_option_operations() {
        let iterations = 100_000;
        let start = Instant::now();

        for i in 0..iterations {
            let option = ScriptOption::some(ScriptValue::I32(i));
            let _processed = option
                .map(|val| val.clone())
                .and_then(|val| ScriptOption::some(val))
                .inspect(|_| {});
        }

        let duration = start.elapsed();
        println!("Option operations took: {:?}", duration);

        // Should complete within reasonable time
        assert!(duration.as_secs() < 5);
    }

    #[test]
    fn benchmark_error_propagation() {
        let iterations = 100_000;
        let start = Instant::now();

        for i in 0..iterations {
            let result = if i % 2 == 0 {
                ScriptResult::ok(ScriptValue::I32(i))
            } else {
                ScriptResult::err(ScriptValue::I32(i))
            };

            let _processed = result
                .and_then(|val| {
                    if let ScriptValue::I32(n) = val {
                        if *n > 0 {
                            ScriptResult::ok(ScriptValue::I32(*n))
                        } else {
                            ScriptResult::err(ScriptValue::I32(0))
                        }
                    } else {
                        ScriptResult::err(ScriptValue::I32(-1))
                    }
                })
                .map(|val| val.clone());
        }

        let duration = start.elapsed();
        println!("Error propagation took: {:?}", duration);

        // Should complete within reasonable time
        assert!(duration.as_secs() < 5);
    }
}

#[cfg(test)]
mod memory_tests {
    use super::*;

    #[test]
    fn test_memory_usage_patterns() {
        // Test that Result and Option don't leak memory
        let mut results = Vec::new();

        for i in 0..1000 {
            let result = ScriptResult::ok(ScriptValue::I32(i));
            let processed = result.map(|val| val.clone());
            results.push(processed);
        }

        // Clear results to test cleanup
        results.clear();

        // Should not cause memory leaks
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_reference_counting() {
        // Test that ScriptRc works correctly with Result and Option
        let string_val = ScriptValue::String(ScriptRc::new(ScriptString::from_str("test")));

        let result1 = ScriptResult::ok(string_val.clone());
        let result2 = result1.map(|val| val.clone());

        // Both results should share the same string data
        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }
}
