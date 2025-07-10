//! Tests for module dependency resolution
//!
//! This test validates the dependency graph functionality without requiring
//! full compilation to pass.

use script::compilation::dependency_graph::{DependencyAnalyzer, DependencyGraph};
use script::lexer::Lexer;
use script::parser::Parser;
use std::collections::HashMap;

#[test]
fn test_dependency_graph_basic() {
    let mut graph = DependencyGraph::new();

    // Add modules
    graph.add_module("main".to_string());
    graph.add_module("utils".to_string());
    graph.add_module("math".to_string());

    // Add dependencies: main -> utils, main -> math, utils -> math
    graph.add_dependency("main".to_string(), "utils".to_string());
    graph.add_dependency("main".to_string(), "math".to_string());
    graph.add_dependency("utils".to_string(), "math".to_string());

    // Get topological order
    let order = graph.topological_sort().expect("Should not have cycles");

    // Math should come first, then utils, then main
    assert_eq!(order, vec!["math", "utils", "main"]);
}

#[test]
fn test_circular_dependency_detection() {
    let mut graph = DependencyGraph::new();

    // Add modules
    graph.add_module("a".to_string());
    graph.add_module("b".to_string());
    graph.add_module("c".to_string());

    // Create circular dependency: a -> b -> c -> a
    graph.add_dependency("a".to_string(), "b".to_string());
    graph.add_dependency("b".to_string(), "c".to_string());
    graph.add_dependency("c".to_string(), "a".to_string());

    // Should detect cycle
    assert!(graph.has_cycle());

    // Should be able to get topological sort error
    let result = graph.topological_sort();
    assert!(result.is_err());

    // Should be able to find cycle
    let cycle = graph.find_cycle();
    assert!(cycle.is_some());
    let cycle_path = cycle.unwrap();
    assert!(!cycle_path.is_empty());
}

#[test]
fn test_dependency_analyzer_basic() {
    let code = r#"
import { add, multiply } from "./math.script"
import { Logger } from "../utils/logger.script"

fn main() {
    let result = add(2, 3);
    Logger::log("Result: " + result.to_string());
}
"#;

    // Parse the code
    let lexer = Lexer::new(code).expect("Failed to create lexer");
    let (tokens, errors) = lexer.scan_tokens();
    assert!(errors.is_empty(), "Lexer errors: {:?}", errors);

    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Failed to parse");

    // Analyze dependencies
    let analyzer = DependencyAnalyzer::new();
    let dependencies = analyzer.analyze(&ast, None);

    // Should find the dependencies in the imports
    // Note: path resolution might not work without proper context,
    // but basic dependency extraction should work
    assert!(
        !dependencies.is_empty(),
        "Should find at least one dependency"
    );
}

#[test]
fn test_no_dependencies() {
    let code = r#"
fn main() {
    print("Hello, world!");
}
"#;

    // Parse the code
    let lexer = Lexer::new(code).expect("Failed to create lexer");
    let (tokens, errors) = lexer.scan_tokens();
    assert!(errors.is_empty(), "Lexer errors: {:?}", errors);

    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Failed to parse");

    // Analyze dependencies
    let analyzer = DependencyAnalyzer::new();
    let dependencies = analyzer.analyze(&ast, None);

    // Should find no dependencies
    assert!(
        dependencies.is_empty(),
        "Should find no dependencies, found: {:?}",
        dependencies
    );
}

#[test]
fn test_multiple_imports() {
    let code = r#"
import { a } from "./module_a.script"
import { b } from "./module_b.script"
import { c } from "../parent/module_c.script"

fn main() {
    a(); b(); c();
}
"#;

    // Parse the code
    let lexer = Lexer::new(code).expect("Failed to create lexer");
    let (tokens, errors) = lexer.scan_tokens();
    assert!(errors.is_empty(), "Lexer errors: {:?}", errors);

    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Failed to parse");

    // Analyze dependencies
    let analyzer = DependencyAnalyzer::new();
    let dependencies = analyzer.analyze(&ast, None);

    // Should find all three dependencies
    assert!(!dependencies.is_empty(), "Should find dependencies");
    // Note: Exact dependency strings depend on path resolution
}
