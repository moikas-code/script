//! Standard library implementation for Result<T, E> type
//!
//! This module provides the core methods and functionality for Result<T, E>,
//! following Rust's Result API design for familiarity and consistency.

use crate::types::Type;
use std::fmt;

/// Core Result<T, E> methods that will be available in the Script language
/// This serves as documentation and reference for implementation
#[derive(Debug, Clone)]
pub enum ResultMethod {
    /// Returns true if the result is Ok
    IsOk,
    /// Returns true if the result is Err
    IsErr,
    /// Maps a Result<T, E> to Result<U, E> by applying a function to Ok value
    Map,
    /// Maps a Result<T, E> to Result<T, F> by applying a function to Err value
    MapErr,
    /// Returns the Ok value or panics with the Err value
    Unwrap,
    /// Returns the Ok value or the provided default
    UnwrapOr,
    /// Returns the Ok value or computes it from a closure
    UnwrapOrElse,
    /// Returns the Ok value or panics with a custom message
    Expect,
    /// Calls a function with the Ok value if Ok, otherwise returns the original Err
    AndThen,
    /// Returns the result if Ok, otherwise returns the provided result
    Or,
    /// Returns the result if Ok, otherwise calls a function to get another result
    OrElse,
}

impl fmt::Display for ResultMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            ResultMethod::IsOk => "is_ok",
            ResultMethod::IsErr => "is_err",
            ResultMethod::Map => "map",
            ResultMethod::MapErr => "map_err",
            ResultMethod::Unwrap => "unwrap",
            ResultMethod::UnwrapOr => "unwrap_or",
            ResultMethod::UnwrapOrElse => "unwrap_or_else",
            ResultMethod::Expect => "expect",
            ResultMethod::AndThen => "and_then",
            ResultMethod::Or => "or",
            ResultMethod::OrElse => "or_else",
        };
        write!(f, "{}", name)
    }
}

/// Type signature for Result<T, E> methods
pub struct ResultMethodSignature {
    pub method: ResultMethod,
    pub params: Vec<Type>,
    pub return_type: Type,
}

impl ResultMethodSignature {
    /// Get the method signature for a given Result method
    pub fn for_method(method: ResultMethod, ok_type: &Type, err_type: &Type) -> Self {
        match method {
            ResultMethod::IsOk | ResultMethod::IsErr => {
                ResultMethodSignature {
                    method,
                    params: vec![],
                    return_type: Type::Bool,
                }
            }
            ResultMethod::Map => {
                // map<U>(self, f: fn(T) -> U) -> Result<U, E>
                // For now, we'll use a placeholder for the closure type
                ResultMethodSignature {
                    method,
                    params: vec![Type::Function {
                        params: vec![ok_type.clone()],
                        return_type: Box::new(Type::TypeParam("U".to_string())),
                        is_async: false,
                    }],
                    return_type: Type::Result {
                        ok: Box::new(Type::TypeParam("U".to_string())),
                        err: Box::new(err_type.clone()),
                    },
                }
            }
            ResultMethod::MapErr => {
                // map_err<F>(self, f: fn(E) -> F) -> Result<T, F>
                ResultMethodSignature {
                    method,
                    params: vec![Type::Function {
                        params: vec![err_type.clone()],
                        return_type: Box::new(Type::TypeParam("F".to_string())),
                        is_async: false,
                    }],
                    return_type: Type::Result {
                        ok: Box::new(ok_type.clone()),
                        err: Box::new(Type::TypeParam("F".to_string())),
                    },
                }
            }
            ResultMethod::Unwrap => {
                ResultMethodSignature {
                    method,
                    params: vec![],
                    return_type: ok_type.clone(),
                }
            }
            ResultMethod::UnwrapOr => {
                ResultMethodSignature {
                    method,
                    params: vec![ok_type.clone()],
                    return_type: ok_type.clone(),
                }
            }
            ResultMethod::UnwrapOrElse => {
                ResultMethodSignature {
                    method,
                    params: vec![Type::Function {
                        params: vec![err_type.clone()],
                        return_type: Box::new(ok_type.clone()),
                        is_async: false,
                    }],
                    return_type: ok_type.clone(),
                }
            }
            ResultMethod::Expect => {
                ResultMethodSignature {
                    method,
                    params: vec![Type::String],
                    return_type: ok_type.clone(),
                }
            }
            ResultMethod::AndThen => {
                // and_then<U>(self, f: fn(T) -> Result<U, E>) -> Result<U, E>
                ResultMethodSignature {
                    method,
                    params: vec![Type::Function {
                        params: vec![ok_type.clone()],
                        return_type: Box::new(Type::Result {
                            ok: Box::new(Type::TypeParam("U".to_string())),
                            err: Box::new(err_type.clone()),
                        }),
                        is_async: false,
                    }],
                    return_type: Type::Result {
                        ok: Box::new(Type::TypeParam("U".to_string())),
                        err: Box::new(err_type.clone()),
                    },
                }
            }
            ResultMethod::Or => {
                ResultMethodSignature {
                    method,
                    params: vec![Type::Result {
                        ok: Box::new(ok_type.clone()),
                        err: Box::new(Type::TypeParam("F".to_string())),
                    }],
                    return_type: Type::Result {
                        ok: Box::new(ok_type.clone()),
                        err: Box::new(Type::TypeParam("F".to_string())),
                    },
                }
            }
            ResultMethod::OrElse => {
                // or_else<F>(self, f: fn(E) -> Result<T, F>) -> Result<T, F>
                ResultMethodSignature {
                    method,
                    params: vec![Type::Function {
                        params: vec![err_type.clone()],
                        return_type: Box::new(Type::Result {
                            ok: Box::new(ok_type.clone()),
                            err: Box::new(Type::TypeParam("F".to_string())),
                        }),
                        is_async: false,
                    }],
                    return_type: Type::Result {
                        ok: Box::new(ok_type.clone()),
                        err: Box::new(Type::TypeParam("F".to_string())),
                    },
                }
            }
        }
    }
}

/// Standard library functions for Result<T, E>
pub fn get_result_methods() -> Vec<ResultMethod> {
    vec![
        ResultMethod::IsOk,
        ResultMethod::IsErr,
        ResultMethod::Map,
        ResultMethod::MapErr,
        ResultMethod::Unwrap,
        ResultMethod::UnwrapOr,
        ResultMethod::UnwrapOrElse,
        ResultMethod::Expect,
        ResultMethod::AndThen,
        ResultMethod::Or,
        ResultMethod::OrElse,
    ]
}

/// Check if a method name is a valid Result method
pub fn is_result_method(method_name: &str) -> Option<ResultMethod> {
    match method_name {
        "is_ok" => Some(ResultMethod::IsOk),
        "is_err" => Some(ResultMethod::IsErr),
        "map" => Some(ResultMethod::Map),
        "map_err" => Some(ResultMethod::MapErr),
        "unwrap" => Some(ResultMethod::Unwrap),
        "unwrap_or" => Some(ResultMethod::UnwrapOr),
        "unwrap_or_else" => Some(ResultMethod::UnwrapOrElse),
        "expect" => Some(ResultMethod::Expect),
        "and_then" => Some(ResultMethod::AndThen),
        "or" => Some(ResultMethod::Or),
        "or_else" => Some(ResultMethod::OrElse),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_method_display() {
        assert_eq!(ResultMethod::IsOk.to_string(), "is_ok");
        assert_eq!(ResultMethod::Map.to_string(), "map");
        assert_eq!(ResultMethod::Unwrap.to_string(), "unwrap");
    }

    #[test]
    fn test_is_result_method() {
        assert!(matches!(is_result_method("is_ok"), Some(ResultMethod::IsOk)));
        assert!(matches!(is_result_method("map"), Some(ResultMethod::Map)));
        assert!(is_result_method("invalid_method").is_none());
    }

    #[test]
    fn test_get_result_methods() {
        let methods = get_result_methods();
        assert!(methods.len() > 0);
        assert!(methods.contains(&ResultMethod::IsOk));
        assert!(methods.contains(&ResultMethod::Map));
    }

    #[test]
    fn test_method_signatures() {
        let ok_type = Type::I32;
        let err_type = Type::String;
        
        let is_ok_sig = ResultMethodSignature::for_method(ResultMethod::IsOk, &ok_type, &err_type);
        assert_eq!(is_ok_sig.return_type, Type::Bool);
        assert_eq!(is_ok_sig.params.len(), 0);
        
        let unwrap_sig = ResultMethodSignature::for_method(ResultMethod::Unwrap, &ok_type, &err_type);
        assert_eq!(unwrap_sig.return_type, Type::I32);
        assert_eq!(unwrap_sig.params.len(), 0);
    }
}