//! Standard library implementation for Option<T> type
//!
//! This module provides the core methods and functionality for Option<T>,
//! following Rust's Option API design for familiarity and consistency.

use crate::types::Type;
use std::fmt;

/// Core Option<T> methods that will be available in the Script language
#[derive(Debug, Clone, PartialEq)]
pub enum OptionMethod {
    /// Returns true if the option is Some
    IsSome,
    /// Returns true if the option is None
    IsNone,
    /// Maps an Option<T> to Option<U> by applying a function to the Some value
    Map,
    /// Returns the Some value or panics with a message
    Unwrap,
    /// Returns the Some value or the provided default
    UnwrapOr,
    /// Returns the Some value or computes it from a closure
    UnwrapOrElse,
    /// Returns the Some value or panics with a custom message
    Expect,
    /// Calls a function with the Some value if Some, otherwise returns None
    AndThen,
    /// Returns the option if Some, otherwise returns the provided option
    Or,
    /// Returns the option if Some, otherwise calls a function to get another option
    OrElse,
    /// Converts Option<T> to Result<T, E> with provided error for None
    OkOr,
    /// Converts Option<T> to Result<T, E> with error computed from closure for None
    OkOrElse,
    /// Filters the option based on a predicate
    Filter,
    /// Takes the value out of the option, leaving None in its place
    Take,
    /// Replaces the actual value in the option by the value given in parameter
    Replace,
}

impl fmt::Display for OptionMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            OptionMethod::IsSome => "is_some",
            OptionMethod::IsNone => "is_none",
            OptionMethod::Map => "map",
            OptionMethod::Unwrap => "unwrap",
            OptionMethod::UnwrapOr => "unwrap_or",
            OptionMethod::UnwrapOrElse => "unwrap_or_else",
            OptionMethod::Expect => "expect",
            OptionMethod::AndThen => "and_then",
            OptionMethod::Or => "or",
            OptionMethod::OrElse => "or_else",
            OptionMethod::OkOr => "ok_or",
            OptionMethod::OkOrElse => "ok_or_else",
            OptionMethod::Filter => "filter",
            OptionMethod::Take => "take",
            OptionMethod::Replace => "replace",
        };
        write!(f, "{}", name)
    }
}

/// Type signature for Option<T> methods
pub struct OptionMethodSignature {
    pub method: OptionMethod,
    pub params: Vec<Type>,
    pub return_type: Type,
}

impl OptionMethodSignature {
    /// Get the method signature for a given Option method
    pub fn for_method(method: OptionMethod, inner_type: &Type) -> Self {
        match method {
            OptionMethod::IsSome | OptionMethod::IsNone => {
                OptionMethodSignature {
                    method,
                    params: vec![],
                    return_type: Type::Bool,
                }
            }
            OptionMethod::Map => {
                // map<U>(self, f: fn(T) -> U) -> Option<U>
                OptionMethodSignature {
                    method,
                    params: vec![Type::Function {
                        params: vec![inner_type.clone()],
                        return_type: Box::new(Type::TypeParam("U".to_string())),
                        is_async: false,
                    }],
                    return_type: Type::Option(Box::new(Type::TypeParam("U".to_string()))),
                }
            }
            OptionMethod::Unwrap => {
                OptionMethodSignature {
                    method,
                    params: vec![],
                    return_type: inner_type.clone(),
                }
            }
            OptionMethod::UnwrapOr => {
                OptionMethodSignature {
                    method,
                    params: vec![inner_type.clone()],
                    return_type: inner_type.clone(),
                }
            }
            OptionMethod::UnwrapOrElse => {
                OptionMethodSignature {
                    method,
                    params: vec![Type::Function {
                        params: vec![],
                        return_type: Box::new(inner_type.clone()),
                        is_async: false,
                    }],
                    return_type: inner_type.clone(),
                }
            }
            OptionMethod::Expect => {
                OptionMethodSignature {
                    method,
                    params: vec![Type::String],
                    return_type: inner_type.clone(),
                }
            }
            OptionMethod::AndThen => {
                // and_then<U>(self, f: fn(T) -> Option<U>) -> Option<U>
                OptionMethodSignature {
                    method,
                    params: vec![Type::Function {
                        params: vec![inner_type.clone()],
                        return_type: Box::new(Type::Option(Box::new(Type::TypeParam("U".to_string())))),
                        is_async: false,
                    }],
                    return_type: Type::Option(Box::new(Type::TypeParam("U".to_string()))),
                }
            }
            OptionMethod::Or => {
                OptionMethodSignature {
                    method,
                    params: vec![Type::Option(Box::new(inner_type.clone())],
                    return_type: Type::Option(Box::new(inner_type.clone()),
                }
            }
            OptionMethod::OrElse => {
                OptionMethodSignature {
                    method,
                    params: vec![Type::Function {
                        params: vec![],
                        return_type: Box::new(Type::Option(Box::new(inner_type.clone())),
                        is_async: false,
                    }],
                    return_type: Type::Option(Box::new(inner_type.clone()),
                }
            }
            OptionMethod::OkOr => {
                // ok_or<E>(self, err: E) -> Result<T, E>
                OptionMethodSignature {
                    method,
                    params: vec![Type::TypeParam("E".to_string())],
                    return_type: Type::Result {
                        ok: Box::new(inner_type.clone()),
                        err: Box::new(Type::TypeParam("E".to_string())),
                    },
                }
            }
            OptionMethod::OkOrElse => {
                // ok_or_else<E>(self, f: fn() -> E) -> Result<T, E>
                OptionMethodSignature {
                    method,
                    params: vec![Type::Function {
                        params: vec![],
                        return_type: Box::new(Type::TypeParam("E".to_string())),
                        is_async: false,
                    }],
                    return_type: Type::Result {
                        ok: Box::new(inner_type.clone()),
                        err: Box::new(Type::TypeParam("E".to_string())),
                    },
                }
            }
            OptionMethod::Filter => {
                OptionMethodSignature {
                    method,
                    params: vec![Type::Function {
                        params: vec![inner_type.clone()],
                        return_type: Box::new(Type::Bool),
                        is_async: false,
                    }],
                    return_type: Type::Option(Box::new(inner_type.clone()),
                }
            }
            OptionMethod::Take => {
                OptionMethodSignature {
                    method,
                    params: vec![],
                    return_type: Type::Option(Box::new(inner_type.clone()),
                }
            }
            OptionMethod::Replace => {
                OptionMethodSignature {
                    method,
                    params: vec![inner_type.clone()],
                    return_type: Type::Option(Box::new(inner_type.clone()),
                }
            }
        }
    }
}

/// Standard library functions for Option<T>
pub fn get_option_methods() -> Vec<OptionMethod> {
    vec![
        OptionMethod::IsSome,
        OptionMethod::IsNone,
        OptionMethod::Map,
        OptionMethod::Unwrap,
        OptionMethod::UnwrapOr,
        OptionMethod::UnwrapOrElse,
        OptionMethod::Expect,
        OptionMethod::AndThen,
        OptionMethod::Or,
        OptionMethod::OrElse,
        OptionMethod::OkOr,
        OptionMethod::OkOrElse,
        OptionMethod::Filter,
        OptionMethod::Take,
        OptionMethod::Replace,
    ]
}

/// Check if a method name is a valid Option method
pub fn is_option_method(method_name: &str) -> Option<OptionMethod> {
    match method_name {
        "is_some" => Some(OptionMethod::IsSome),
        "is_none" => Some(OptionMethod::IsNone),
        "map" => Some(OptionMethod::Map),
        "unwrap" => Some(OptionMethod::Unwrap),
        "unwrap_or" => Some(OptionMethod::UnwrapOr),
        "unwrap_or_else" => Some(OptionMethod::UnwrapOrElse),
        "expect" => Some(OptionMethod::Expect),
        "and_then" => Some(OptionMethod::AndThen),
        "or" => Some(OptionMethod::Or),
        "or_else" => Some(OptionMethod::OrElse),
        "ok_or" => Some(OptionMethod::OkOr),
        "ok_or_else" => Some(OptionMethod::OkOrElse),
        "filter" => Some(OptionMethod::Filter),
        "take" => Some(OptionMethod::Take),
        "replace" => Some(OptionMethod::Replace),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_option_method_display() {
        assert_eq!(OptionMethod::IsSome.to_string(), "is_some");
        assert_eq!(OptionMethod::Map.to_string(), "map");
        assert_eq!(OptionMethod::Unwrap.to_string(), "unwrap");
    }

    #[test]
    fn test_is_option_method() {
        assert!(matches!(is_option_method("is_some"), Some(OptionMethod::IsSome)));
        assert!(matches!(is_option_method("map"), Some(OptionMethod::Map)));
        assert!(is_option_method("invalid_method").is_none());
    }

    #[test]
    fn test_get_option_methods() {
        let methods = get_option_methods();
        assert!(methods.len() > 0);
        assert!(methods.contains(&OptionMethod::IsSome));
        assert!(methods.contains(&OptionMethod::Map));
    }

    #[test]
    fn test_method_signatures() {
        let inner_type = Type::I32;
        
        let is_some_sig = OptionMethodSignature::for_method(OptionMethod::IsSome, &inner_type);
        assert_eq!(is_some_sig.return_type, Type::Bool);
        assert_eq!(is_some_sig.params.len(), 0);
        
        let unwrap_sig = OptionMethodSignature::for_method(OptionMethod::Unwrap, &inner_type);
        assert_eq!(unwrap_sig.return_type, Type::I32);
        assert_eq!(unwrap_sig.params.len(), 0);
    }
}