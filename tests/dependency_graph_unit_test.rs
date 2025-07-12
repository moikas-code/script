use script::compilation::{DependencyAnalyzer, DependencyGraph};
use script::parser::{Expr, ExprKind, Program, Stmt, StmtKind};
use std::collections::HashSet;

#[test]
fn test_dependency_graph_basic() {
    let mut graph = DependencyGraph::new();

    graph.add_module("main".to_string());
    graph.add_module("utils".to_string());
    graph.add_module("math".to_string());

    graph.add_dependency("main".to_string(), "utils".to_string());
    graph.add_dependency("main".to_string(), "math".to_string());
    graph.add_dependency("math".to_string(), "utils".to_string());

    // Test topological sort
    let order = graph.topological_sort().expect("Should successfully sort");

    // utils should come first, then math, then main
    assert_eq!(order, vec!["utils", "math", "main"]);
}

#[test]
fn test_dependency_graph_circular() {
    let mut graph = DependencyGraph::new();

    graph.add_module("a".to_string());
    graph.add_module("b".to_string());
    graph.add_module("c".to_string());

    // Create circular dependency
    graph.add_dependency("a".to_string(), "b".to_string());
    graph.add_dependency("b".to_string(), "c".to_string());
    graph.add_dependency("c".to_string(), "a".to_string());

    // Should detect circular dependency
    assert!(graph.topological_sort().is_err());

    // Should find the cycle
    let cycle = graph.find_cycle();
    assert!(cycle.is_some());
    let cycle_path = cycle.unwrap();
    assert_eq!(cycle_path.len(), 3);
}

#[test]
fn test_dependency_analyzer_basic() {
    let analyzer = DependencyAnalyzer::new();

    // Create a simple AST with import statements
    let import_stmt = Stmt {
        kind: StmtKind::Import {
            module: "math".to_string(),
            imports: None,
        },
        id: 1,
    };

    let program = Program {
        statements: vec![import_stmt],
    };

    let deps = analyzer.analyze(&program, None);

    assert!(deps.contains("math"));
}

#[test]
fn test_dependency_graph_empty() {
    let graph = DependencyGraph::new();

    // Empty graph should sort successfully to empty vec
    let order = graph.topological_sort().expect("Empty graph should sort");
    assert!(order.is_empty());
}

#[test]
fn test_dependency_graph_single_module() {
    let mut graph = DependencyGraph::new();
    graph.add_module("solo".to_string());

    let order = graph.topological_sort().expect("Single module should sort");
    assert_eq!(order, vec!["solo"]);
}

#[test]
fn test_dependency_analyzer_multiple_imports() {
    let analyzer = DependencyAnalyzer::new();

    let import1 = Stmt {
        kind: StmtKind::Import {
            module: "math".to_string(),
            imports: None,
        },
        id: 1,
    };

    let import2 = Stmt {
        kind: StmtKind::Import {
            module: "utils".to_string(),
            imports: None,
        },
        id: 2,
    };

    let program = Program {
        statements: vec![import1, import2],
    };

    let deps = analyzer.analyze(&program, None);

    assert!(deps.contains("math"));
    assert!(deps.contains("utils"));
    assert_eq!(deps.len(), 2);
}
