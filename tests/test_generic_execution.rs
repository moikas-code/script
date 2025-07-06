//! Test for generic function execution after monomorphization

use script::codegen::{CodeGenerator, MonomorphizationContext};
use script::lexer::Lexer;
use script::parser::Parser;
use script::lowering::AstLowerer;
use script::semantic::{SemanticAnalyzer, SymbolTable};
use std::collections::HashMap;

#[test]
fn test_generic_function_execution() {
    let source = r#"
        fn identity<T>(x: T) -> T {
            return x;
        }
        
        fn main() -> i32 {
            let result = identity(42);
            return result;
        }
    "#;
    
    // Lexical analysis
    let lexer = Lexer::new(source);
    let (tokens, errors) = lexer.scan_tokens();
    assert!(errors.is_empty(), "Lexer should not produce errors");
    
    // Parsing
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parsing should succeed");
    
    // Semantic analysis
    let mut semantic_analyzer = SemanticAnalyzer::new();
    semantic_analyzer.analyze_program(&ast)
        .expect("Semantic analysis should succeed");
    
    // Get type info and generic instantiations
    let type_info = semantic_analyzer.extract_type_info();
    let generic_instantiations = semantic_analyzer.generic_instantiations().to_vec();
    
    // Lower to IR
    let symbol_table = SymbolTable::new();
    let mut lowerer = AstLowerer::new(symbol_table, type_info, generic_instantiations.clone());
    let mut ir_module = lowerer.lower_program(&ast)
        .expect("Lowering should succeed");
    
    // Monomorphization
    let mut mono_ctx = MonomorphizationContext::new();
    mono_ctx.initialize_from_semantic_analysis(&generic_instantiations, &HashMap::new());
    mono_ctx.monomorphize(&mut ir_module)
        .expect("Monomorphization should succeed");
    
    // Code generation
    let mut codegen = CodeGenerator::new();
    let executable = codegen.generate(&ir_module)
        .expect("Code generation should succeed");
    
    // Execute
    let result = executable.execute()
        .expect("Execution should succeed");
    
    assert_eq!(result, 42, "Generic function should return the correct value");
}

#[test]
fn test_multiple_generic_instantiations() {
    let source = r#"
        fn identity<T>(x: T) -> T {
            return x;
        }
        
        fn add<T>(x: T, y: T) -> T {
            return x + y;
        }
        
        fn main() -> i32 {
            let a = identity(10);
            let b = identity(32);
            let sum = add(a, b);
            return sum;
        }
    "#;
    
    // Full compilation pipeline
    let lexer = Lexer::new(source);
    let (tokens, errors) = lexer.scan_tokens();
    assert!(errors.is_empty(), "Lexer should not produce errors");
    
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parsing should succeed");
    
    let mut semantic_analyzer = SemanticAnalyzer::new();
    semantic_analyzer.analyze_program(&ast)
        .expect("Semantic analysis should succeed");
    
    let type_info = semantic_analyzer.extract_type_info();
    let generic_instantiations = semantic_analyzer.generic_instantiations().to_vec();
    
    let symbol_table = SymbolTable::new();
    let mut lowerer = AstLowerer::new(symbol_table, type_info, generic_instantiations.clone());
    let mut ir_module = lowerer.lower_program(&ast)
        .expect("Lowering should succeed");
    
    let mut mono_ctx = MonomorphizationContext::new();
    mono_ctx.initialize_from_semantic_analysis(&generic_instantiations, &HashMap::new());
    mono_ctx.monomorphize(&mut ir_module)
        .expect("Monomorphization should succeed");
    
    let mut codegen = CodeGenerator::new();
    let executable = codegen.generate(&ir_module)
        .expect("Code generation should succeed");
    
    let result = executable.execute()
        .expect("Execution should succeed");
    
    assert_eq!(result, 42, "Generic functions should work correctly");
}

#[test]
fn test_generic_with_multiple_parameters() {
    let source = r#"
        fn swap<T, U>(x: T, y: U) -> T {
            // Just return the first parameter for now
            // (full swap would require tuple support)
            return x;
        }
        
        fn main() -> i32 {
            let result = swap(42, 3.14);
            return result;
        }
    "#;
    
    // Full compilation pipeline
    let lexer = Lexer::new(source);
    let (tokens, errors) = lexer.scan_tokens();
    assert!(errors.is_empty(), "Lexer should not produce errors");
    
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parsing should succeed");
    
    let mut semantic_analyzer = SemanticAnalyzer::new();
    semantic_analyzer.analyze_program(&ast)
        .expect("Semantic analysis should succeed");
    
    let type_info = semantic_analyzer.extract_type_info();
    let generic_instantiations = semantic_analyzer.generic_instantiations().to_vec();
    
    let symbol_table = SymbolTable::new();
    let mut lowerer = AstLowerer::new(symbol_table, type_info, generic_instantiations.clone());
    let mut ir_module = lowerer.lower_program(&ast)
        .expect("Lowering should succeed");
    
    let mut mono_ctx = MonomorphizationContext::new();
    mono_ctx.initialize_from_semantic_analysis(&generic_instantiations, &HashMap::new());
    mono_ctx.monomorphize(&mut ir_module)
        .expect("Monomorphization should succeed");
    
    let mut codegen = CodeGenerator::new();
    let executable = codegen.generate(&ir_module)
        .expect("Code generation should succeed");
    
    let result = executable.execute()
        .expect("Execution should succeed");
    
    assert_eq!(result, 42, "Generic function with multiple type parameters should work");
}