//! Conversion utilities between Result<T, E> and Option<T> types
//!
//! This module provides utility functions for converting between different
//! error handling types, making it easier to work with mixed APIs.

use crate::types::Type;
use super::ScriptValue;
use crate::runtime::RuntimeError;

/// Conversion methods between Result and Option types
#[derive(Debug, Clone, PartialEq)]
pub enum ConversionMethod {
    /// Convert Option<T> to Result<T, E> with provided error for None
    OptionToResult,
    /// Convert Result<T, E> to Option<T>, discarding error information
    ResultToOption,
    /// Transpose Option<Result<T, E>> to Result<Option<T>, E>
    TransposeOptionResult,
    /// Transpose Result<Option<T>, E> to Option<Result<T, E>>
    TransposeResultOption,
}

/// Type signature for conversion methods
pub struct ConversionSignature {
    pub method: ConversionMethod,
    pub input_type: Type,
    pub output_type: Type,
    pub additional_params: Vec<Type>,
}

impl ConversionSignature {
    /// Get the signature for a conversion method
    pub fn for_method(method: ConversionMethod, inner_type: &Type) -> Self {
        match method {
            ConversionMethod::OptionToResult => {
                ConversionSignature {
                    method,
                    input_type: Type::Option(Box::new(inner_type.clone()),
                    output_type: Type::Result {
                        ok: Box::new(inner_type.clone()),
                        err: Box::new(Type::TypeParam("E".to_string())),
                    },
                    additional_params: vec![Type::TypeParam("E".to_string())],
                }
            }
            ConversionMethod::ResultToOption => {
                ConversionSignature {
                    method,
                    input_type: Type::Result {
                        ok: Box::new(inner_type.clone()),
                        err: Box::new(Type::TypeParam("E".to_string())),
                    },
                    output_type: Type::Option(Box::new(inner_type.clone()),
                    additional_params: vec![],
                }
            }
            ConversionMethod::TransposeOptionResult => {
                ConversionSignature {
                    method,
                    input_type: Type::Option(Box::new(Type::Result {
                        ok: Box::new(inner_type.clone()),
                        err: Box::new(Type::TypeParam("E".to_string())),
                    })),
                    output_type: Type::Result {
                        ok: Box::new(Type::Option(Box::new(inner_type.clone())),
                        err: Box::new(Type::TypeParam("E".to_string())),
                    },
                    additional_params: vec![],
                }
            }
            ConversionMethod::TransposeResultOption => {
                ConversionSignature {
                    method,
                    input_type: Type::Result {
                        ok: Box::new(Type::Option(Box::new(inner_type.clone())),
                        err: Box::new(Type::TypeParam("E".to_string())),
                    },
                    output_type: Type::Option(Box::new(Type::Result {
                        ok: Box::new(inner_type.clone()),
                        err: Box::new(Type::TypeParam("E".to_string())),
                    })),
                    additional_params: vec![],
                }
            }
        }
    }
}

/// Convert Option to Result with provided error for None case
pub fn option_to_result_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            "option_to_result requires exactly 2 arguments (option, error)".to_string(),
        ));
    }

    // Extract Option and error value
    let option_value = &args[0];
    let error_value = args[1].clone();

    match option_value {
        ScriptValue::Option(opt_rc) => {
            match &**opt_rc {
                super::core_types::ScriptOption::Some(value) => {
                    // Return Ok(value)
                    Ok(ScriptValue::Result(
                        crate::runtime::ScriptRc::new(super::core_types::ScriptResult::Ok(
                            value.clone(),
                        )),
                    ))
                }
                super::core_types::ScriptOption::None => {
                    // Return Err(error_value)
                    Ok(ScriptValue::Result(
                        crate::runtime::ScriptRc::new(super::core_types::ScriptResult::Err(
                            error_value,
                        )),
                    ))
                }
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "First argument must be an Option".to_string(),
        )),
    }
}

/// Convert Result to Option, discarding error information
pub fn result_to_option_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            "result_to_option requires exactly 1 argument (result)".to_string(),
        ));
    }

    let result_value = &args[0];

    match result_value {
        ScriptValue::Result(result_rc) => {
            match &**result_rc {
                super::core_types::ScriptResult::Ok(value) => {
                    // Return Some(value)
                    Ok(ScriptValue::Option(
                        crate::runtime::ScriptRc::new(super::core_types::ScriptOption::Some(
                            value.clone(),
                        )),
                    ))
                }
                super::core_types::ScriptResult::Err(_) => {
                    // Return None
                    Ok(ScriptValue::Option(
                        crate::runtime::ScriptRc::new(super::core_types::ScriptOption::None),
                    ))
                }
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "First argument must be a Result".to_string(),
        )),
    }
}

/// Transpose Option<Result<T, E>> to Result<Option<T>, E>
pub fn transpose_option_result_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            "transpose_option_result requires exactly 1 argument".to_string(),
        ));
    }

    let option_value = &args[0];

    match option_value {
        ScriptValue::Option(opt_rc) => {
            match &**opt_rc {
                super::core_types::ScriptOption::Some(inner_value) => {
                    // Inner value should be a Result
                    match inner_value {
                        ScriptValue::Result(result_rc) => {
                            match &**result_rc {
                                super::core_types::ScriptResult::Ok(ok_value) => {
                                    // Some(Ok(value)) -> Ok(Some(value))
                                    Ok(ScriptValue::Result(
                                        crate::runtime::ScriptRc::new(
                                            super::core_types::ScriptResult::Ok(
                                                ScriptValue::Option(
                                                    crate::runtime::ScriptRc::new(
                                                        super::core_types::ScriptOption::Some(
                                                            ok_value.clone(),
                                                        ),
                                                    ),
                                                ),
                                            ),
                                        ),
                                    ))
                                }
                                super::core_types::ScriptResult::Err(err_value) => {
                                    // Some(Err(error)) -> Err(error)
                                    Ok(ScriptValue::Result(
                                        crate::runtime::ScriptRc::new(
                                            super::core_types::ScriptResult::Err(err_value.clone()),
                                        ),
                                    ))
                                }
                            }
                        }
                        _ => Err(RuntimeError::InvalidOperation(
                            "Inner value of Option must be a Result for transpose".to_string(),
                        )),
                    }
                }
                super::core_types::ScriptOption::None => {
                    // None -> Ok(None)
                    Ok(ScriptValue::Result(
                        crate::runtime::ScriptRc::new(super::core_types::ScriptResult::Ok(
                            ScriptValue::Option(
                                crate::runtime::ScriptRc::new(super::core_types::ScriptOption::None),
                            ),
                        )),
                    ))
                }
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "Argument must be an Option for transpose".to_string(),
        )),
    }
}

/// Transpose Result<Option<T>, E> to Option<Result<T, E>>
pub fn transpose_result_option_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            "transpose_result_option requires exactly 1 argument".to_string(),
        ));
    }

    let result_value = &args[0];

    match result_value {
        ScriptValue::Result(result_rc) => {
            match &**result_rc {
                super::core_types::ScriptResult::Ok(ok_value) => {
                    // Inner value should be an Option
                    match ok_value {
                        ScriptValue::Option(opt_rc) => {
                            match &**opt_rc {
                                super::core_types::ScriptOption::Some(inner_value) => {
                                    // Ok(Some(value)) -> Some(Ok(value))
                                    Ok(ScriptValue::Option(
                                        crate::runtime::ScriptRc::new(
                                            super::core_types::ScriptOption::Some(
                                                ScriptValue::Result(
                                                    crate::runtime::ScriptRc::new(
                                                        super::core_types::ScriptResult::Ok(
                                                            inner_value.clone(),
                                                        ),
                                                    ),
                                                ),
                                            ),
                                        ),
                                    ))
                                }
                                super::core_types::ScriptOption::None => {
                                    // Ok(None) -> None
                                    Ok(ScriptValue::Option(
                                        crate::runtime::ScriptRc::new(
                                            super::core_types::ScriptOption::None,
                                        ),
                                    ))
                                }
                            }
                        }
                        _ => Err(RuntimeError::InvalidOperation(
                            "Inner value of Result must be an Option for transpose".to_string(),
                        )),
                    }
                }
                super::core_types::ScriptResult::Err(err_value) => {
                    // Err(error) -> Some(Err(error))
                    Ok(ScriptValue::Option(
                        crate::runtime::ScriptRc::new(super::core_types::ScriptOption::Some(
                            ScriptValue::Result(
                                crate::runtime::ScriptRc::new(
                                    super::core_types::ScriptResult::Err(err_value.clone()),
                                ),
                            ),
                        )),
                    ))
                }
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "Argument must be a Result for transpose".to_string(),
        )),
    }
}

/// Check if a function name is a conversion utility
pub fn is_conversion_function(function_name: &str) -> Option<ConversionMethod> {
    match function_name {
        "option_to_result" | "ok_or" => Some(ConversionMethod::OptionToResult),
        "result_to_option" => Some(ConversionMethod::ResultToOption),
        "transpose_option_result" => Some(ConversionMethod::TransposeOptionResult),
        "transpose_result_option" => Some(ConversionMethod::TransposeResultOption),
        _ => None,
    }
}

/// Get all available conversion functions
pub fn get_conversion_functions() -> Vec<(&'static str, ConversionMethod)> {
    vec![
        ("option_to_result", ConversionMethod::OptionToResult),
        ("ok_or", ConversionMethod::OptionToResult), // Alias for option_to_result
        ("result_to_option", ConversionMethod::ResultToOption),
        ("transpose_option_result", ConversionMethod::TransposeOptionResult),
        ("transpose_result_option", ConversionMethod::TransposeResultOption),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversion_function_recognition() {
        assert!(matches!(
            is_conversion_function("option_to_result"),
            Some(ConversionMethod::OptionToResult)
        ));
        assert!(matches!(
            is_conversion_function("ok_or"),
            Some(ConversionMethod::OptionToResult)
        ));
        assert!(matches!(
            is_conversion_function("result_to_option"),
            Some(ConversionMethod::ResultToOption)
        ));
        assert!(is_conversion_function("invalid_function").is_none());
    }

    #[test]
    fn test_get_conversion_functions() {
        let functions = get_conversion_functions();
        assert!(functions.len() >= 4);
        assert!(functions
            .iter()
            .any(|(name, _)| *name == "option_to_result"));
        assert!(functions.iter().any(|(name, _)| *name == "ok_or"));
    }

    #[test]
    fn test_conversion_signatures() {
        let inner_type = Type::I32;
        
        let opt_to_res_sig = ConversionSignature::for_method(
            ConversionMethod::OptionToResult,
            &inner_type,
        );
        
        assert!(matches!(opt_to_res_sig.input_type, Type::Option(_)));
        assert!(matches!(opt_to_res_sig.output_type, Type::Result { .. }));
        assert_eq!(opt_to_res_sig.additional_params.len(), 1);
    }
}