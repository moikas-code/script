//! Helper utilities for testing generic functionality in Script
//!
//! This module provides common utilities for testing generic structs, enums,
//! and functions throughout the test suite.

use script::error::Error;
use script::lexer::Lexer;
use script::parser::{Parser, Program};
use script::semantic::{GenericInstantiation, SemanticAnalyzer};
use script::types::Type;
use std::collections::HashMap;

/// Result of analyzing a generic program
#[derive(Debug)]
pub struct AnalyzedProgram {
    pub ast: Program,
    pub analyzer: SemanticAnalyzer,
    pub errors: Vec<Error>,
}

/// A monomorphized type with its original generic name and concrete types
#[derive(Debug, Clone, PartialEq)]
pub struct MonomorphizedType {
    pub generic_name: String,
    pub type_args: Vec<Type>,
    pub specialized_name: String,
}

/// Helper to compile and analyze a generic program
pub fn compile_generic_program(source: &str) -> Result<AnalyzedProgram, Error> {
    // Lex the source
    let lexer = Lexer::new(source);
    let (tokens, lex_errors) = lexer.scan_tokens();

    if !lex_errors.is_empty() {
        return Err(lex_errors[0].clone());
    }

    // Parse the tokens
    let mut parser = Parser::new(tokens);
    let ast = parser.parse()?;

    // Analyze the program
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze_program(&ast);

    // Collect any errors but don't fail immediately
    let errors = if result.is_err() {
        vec![result.unwrap_err()]
    } else {
        analyzer.errors().to_vec()
    };

    Ok(AnalyzedProgram {
        ast,
        analyzer,
        errors,
    })
}

/// Extract monomorphized types from the semantic analyzer
pub fn get_monomorphized_types(analyzer: &SemanticAnalyzer) -> Vec<MonomorphizedType> {
    analyzer
        .generic_instantiations()
        .iter()
        .map(|inst| {
            let specialized_name = generate_specialized_name(&inst.function_name, &inst.type_args);
            MonomorphizedType {
                generic_name: inst.function_name.clone(),
                type_args: inst.type_args.clone(),
                specialized_name,
            }
        })
        .collect()
}

/// Generate a specialized name for a monomorphized type
fn generate_specialized_name(base_name: &str, type_args: &[Type]) -> String {
    if type_args.is_empty() {
        return base_name.to_string();
    }

    let type_suffix = type_args
        .iter()
        .map(type_to_mangle_string)
        .collect::<Vec<_>>()
        .join("_");

    format!("{}_{}", base_name, type_suffix)
}

/// Convert a type to a mangled string for naming
fn type_to_mangle_string(ty: &Type) -> String {
    match ty {
        Type::I32 => "i32".to_string(),
        Type::F32 => "f32".to_string(),
        Type::Bool => "bool".to_string(),
        Type::String => "string".to_string(),
        Type::Array(elem) => format!("array_{}", type_to_mangle_string(elem)),
        Type::Option(inner) => format!("option_{}", type_to_mangle_string(inner)),
        Type::Result { ok, err } => format!(
            "result_{}_{}",
            type_to_mangle_string(ok),
            type_to_mangle_string(err)
        ),
        Type::Named(name) => name.replace("::", "_"),
        _ => "unknown".to_string(),
    }
}

/// Verify that a specific type instantiation occurred
pub fn assert_type_instantiated(program: &AnalyzedProgram, expected: &str) -> bool {
    let monomorphized = get_monomorphized_types(&program.analyzer);
    monomorphized.iter().any(|m| m.specialized_name == expected)
}

/// Helper to create a generic struct program
pub fn create_generic_struct_program(
    name: &str,
    params: &[&str],
    fields: &[(&str, &str)],
) -> String {
    let params_str = if params.is_empty() {
        String::new()
    } else {
        format!("<{}>", params.join(", "))
    };

    let fields_str = fields
        .iter()
        .map(|(field_name, field_type)| format!("    {}: {}", field_name, field_type))
        .collect::<Vec<_>>()
        .join(",\n");

    format!("struct {}{} {{\n{}\n}}", name, params_str, fields_str)
}

/// Helper to create a generic enum program
pub fn create_generic_enum_program(
    name: &str,
    params: &[&str],
    variants: &[(&str, Vec<&str>)],
) -> String {
    let params_str = if params.is_empty() {
        String::new()
    } else {
        format!("<{}>", params.join(", "))
    };

    let variants_str = variants
        .iter()
        .map(|(variant_name, args)| {
            if args.is_empty() {
                format!("    {}", variant_name)
            } else {
                format!("    {}({})", variant_name, args.join(", "))
            }
        })
        .collect::<Vec<_>>()
        .join(",\n");

    format!("enum {}{} {{\n{}\n}}", name, params_str, variants_str)
}

/// Create a test program with generic struct usage
pub fn create_struct_usage_program(struct_def: &str, constructor_exprs: &[&str]) -> String {
    let usage = constructor_exprs
        .iter()
        .enumerate()
        .map(|(i, expr)| format!("    let var{} = {}", i, expr))
        .collect::<Vec<_>>()
        .join("\n");

    format!("{}\n\nfn main() {{\n{}\n}}", struct_def, usage)
}

/// Create a test program with generic enum usage
pub fn create_enum_usage_program(enum_def: &str, constructor_exprs: &[&str]) -> String {
    let usage = constructor_exprs
        .iter()
        .enumerate()
        .map(|(i, expr)| format!("    let var{} = {}", i, expr))
        .collect::<Vec<_>>()
        .join("\n");

    format!("{}\n\nfn main() {{\n{}\n}}", enum_def, usage)
}

/// Assert that a program compiles without errors
pub fn assert_no_errors(program: &AnalyzedProgram) {
    if !program.errors.is_empty() {
        panic!(
            "Expected no errors, but found {}:\n{}",
            program.errors.len(),
            program
                .errors
                .iter()
                .map(|e| format!("  - {}", e))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// Assert that a program has specific error
pub fn assert_has_error_containing(program: &AnalyzedProgram, expected_text: &str) {
    let has_error = program
        .errors
        .iter()
        .any(|e| e.to_string().contains(expected_text));

    if !has_error {
        panic!(
            "Expected error containing '{}', but found:\n{}",
            expected_text,
            if program.errors.is_empty() {
                "  No errors".to_string()
            } else {
                program
                    .errors
                    .iter()
                    .map(|e| format!("  - {}", e))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        );
    }
}

/// Count the number of monomorphized instances for a given generic type
pub fn count_monomorphized_instances(program: &AnalyzedProgram, generic_name: &str) -> usize {
    program
        .analyzer
        .generic_instantiations()
        .iter()
        .filter(|inst| inst.function_name.starts_with(generic_name))
        .count()
}

/// Get all type arguments used for a specific generic type
pub fn get_type_args_for_generic(program: &AnalyzedProgram, generic_name: &str) -> Vec<Vec<Type>> {
    program
        .analyzer
        .generic_instantiations()
        .iter()
        .filter(|inst| inst.function_name.starts_with(generic_name))
        .map(|inst| inst.type_args.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_generic_struct() {
        let struct_def = create_generic_struct_program("Box", &["T"], &[("value", "T")]);
        assert_eq!(struct_def, "struct Box<T> {\n    value: T\n}");
    }

    #[test]
    fn test_create_generic_enum() {
        let enum_def =
            create_generic_enum_program("Option", &["T"], &[("Some", vec!["T"]), ("None", vec![])]);
        assert_eq!(enum_def, "enum Option<T> {\n    Some(T),\n    None\n}");
    }

    #[test]
    fn test_type_mangling() {
        assert_eq!(type_to_mangle_string(&Type::I32), "i32");
        assert_eq!(type_to_mangle_string(&Type::String), "string");
        assert_eq!(
            type_to_mangle_string(&Type::Array(Box::new(Type::I32))),
            "array_i32"
        );
        assert_eq!(
            type_to_mangle_string(&Type::Option(Box::new(Type::Bool))),
            "option_bool"
        );
    }
}
