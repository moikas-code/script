use crate::error::{Error, ErrorKind, Result};
use crate::parser::{Program, StmtKind};
use std::collections::{HashMap, HashSet, VecDeque};

/// Represents the dependency graph of modules
#[derive(Debug)]
pub struct DependencyGraph {
    /// Adjacency list representation: module -> set of dependencies
    dependencies: HashMap<String, HashSet<String>>,
    /// Reverse dependencies: module -> set of modules that depend on it
    dependents: HashMap<String, HashSet<String>>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        DependencyGraph {
            dependencies: HashMap::new(),
            dependents: HashMap::new(),
        }
    }

    /// Add a module to the graph
    pub fn add_module(&mut self, module: String) {
        self.dependencies
            .entry(module.clone())
            .or_insert_with(HashSet::new);
        self.dependents.entry(module).or_insert_with(HashSet::new);
    }

    /// Add a dependency: `module` depends on `dependency`
    pub fn add_dependency(&mut self, module: String, dependency: String) {
        self.dependencies
            .entry(module.clone())
            .or_insert_with(HashSet::new)
            .insert(dependency.clone());

        self.dependents
            .entry(dependency)
            .or_insert_with(HashSet::new)
            .insert(module);
    }

    /// Get all direct dependencies of a module
    pub fn get_dependencies(&self, module: &str) -> Option<&HashSet<String>> {
        self.dependencies.get(module)
    }

    /// Get all modules that depend on the given module
    pub fn get_dependents(&self, module: &str) -> Option<&HashSet<String>> {
        self.dependents.get(module)
    }

    /// Perform topological sort to get compilation order
    pub fn topological_sort(&self) -> Result<Vec<String>> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        // Calculate in-degrees
        for module in self.dependencies.keys() {
            in_degree.insert(module.clone(), 0);
        }

        for deps in self.dependencies.values() {
            for dep in deps {
                *in_degree.entry(dep.clone()).or_insert(0) += 1;
            }
        }

        // Find all nodes with in-degree 0
        for (module, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(module.clone());
            }
        }

        // Process queue
        while let Some(module) = queue.pop_front() {
            result.push(module.clone());

            if let Some(deps) = self.dependencies.get(&module) {
                for dep in deps {
                    if let Some(degree) = in_degree.get_mut(dep) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dep.clone());
                        }
                    }
                }
            }
        }

        // Check for cycles
        if result.len() != self.dependencies.len() {
            return Err(Error::new(
                ErrorKind::CompilationError,
                "Circular dependency detected in module graph",
            ));
        }

        // Reverse to get correct compilation order (dependencies first)
        result.reverse();
        Ok(result)
    }

    /// Check for circular dependencies
    pub fn has_cycle(&self) -> bool {
        self.topological_sort().is_err()
    }

    /// Find a cycle if one exists (for better error reporting)
    pub fn find_cycle(&self) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for module in self.dependencies.keys() {
            if !visited.contains(module) {
                if let Some(cycle) =
                    self.dfs_find_cycle(module, &mut visited, &mut rec_stack, &mut path)
                {
                    return Some(cycle);
                }
            }
        }

        None
    }

    /// DFS helper to find cycles
    fn dfs_find_cycle(
        &self,
        module: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(module.to_string());
        rec_stack.insert(module.to_string());
        path.push(module.to_string());

        if let Some(deps) = self.dependencies.get(module) {
            for dep in deps {
                if !visited.contains(dep) {
                    if let Some(cycle) = self.dfs_find_cycle(dep, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(dep) {
                    // Found a cycle
                    let cycle_start = path.iter().position(|m| m == dep).unwrap();
                    return Some(path[cycle_start..].to_vec());
                }
            }
        }

        rec_stack.remove(module);
        path.pop();
        None
    }
}

/// Analyzes AST to extract module dependencies
pub struct DependencyAnalyzer;

impl DependencyAnalyzer {
    /// Extract dependencies from a parsed program
    pub fn analyze(ast: &Program) -> HashSet<String> {
        let mut dependencies = HashSet::new();

        for stmt in &ast.statements {
            if let StmtKind::Import { module, .. } = &stmt.kind {
                // Extract module name from import path
                // For now, assume the path is the module name
                dependencies.insert(module.clone());
            }
        }

        dependencies
    }

    /// Build a complete dependency graph from multiple modules
    pub fn build_graph(modules: &HashMap<String, Program>) -> DependencyGraph {
        let mut graph = DependencyGraph::new();

        // Add all modules
        for module_name in modules.keys() {
            graph.add_module(module_name.clone());
        }

        // Add dependencies
        for (module_name, ast) in modules {
            let deps = Self::analyze(ast);
            for dep in deps {
                // Only add if the dependency is an internal module
                if modules.contains_key(&dep) {
                    graph.add_dependency(module_name.clone(), dep);
                }
            }
        }

        graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_dependency_graph() {
        let mut graph = DependencyGraph::new();
        graph.add_module("main".to_string());
        graph.add_module("math".to_string());
        graph.add_module("utils".to_string());

        graph.add_dependency("main".to_string(), "math".to_string());
        graph.add_dependency("main".to_string(), "utils".to_string());
        graph.add_dependency("math".to_string(), "utils".to_string());

        let order = graph.topological_sort().unwrap();

        // utils should come first, then math, then main
        assert_eq!(order, vec!["utils", "math", "main"]);
    }

    #[test]
    fn test_circular_dependency() {
        let mut graph = DependencyGraph::new();
        graph.add_module("a".to_string());
        graph.add_module("b".to_string());
        graph.add_module("c".to_string());

        graph.add_dependency("a".to_string(), "b".to_string());
        graph.add_dependency("b".to_string(), "c".to_string());
        graph.add_dependency("c".to_string(), "a".to_string());

        assert!(graph.has_cycle());

        let cycle = graph.find_cycle().unwrap();
        assert_eq!(cycle.len(), 3);
    }
}
