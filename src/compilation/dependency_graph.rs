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
                    if let Some(cycle_start) = path.iter().position(|m| m == dep) {
                        return Some(path[cycle_start..].to_vec());
                    } else {
                        // This should never happen, but handle gracefully
                        return Some(vec![dep.clone()]);
                    }
                }
            }
        }

        rec_stack.remove(module);
        path.pop();
        None
    }
}

/// Analyzes AST to extract module dependencies with proper path resolution
pub struct DependencyAnalyzer {
    /// Base path for resolving relative imports
    base_path: Option<std::path::PathBuf>,
}

impl DependencyAnalyzer {
    /// Create a new dependency analyzer
    pub fn new() -> Self {
        Self { base_path: None }
    }

    /// Create analyzer with a base path for relative imports
    pub fn with_base_path(base_path: std::path::PathBuf) -> Self {
        Self {
            base_path: Some(base_path),
        }
    }

    /// Extract dependencies from a parsed program with proper path resolution
    pub fn analyze(
        &self,
        ast: &Program,
        current_module_path: Option<&std::path::Path>,
    ) -> HashSet<String> {
        let mut dependencies = HashSet::new();

        for stmt in &ast.statements {
            if let StmtKind::Import {
                module, imports, ..
            } = &stmt.kind
            {
                // Resolve module path properly
                let resolved_module = self.resolve_module_path(module, current_module_path);

                match resolved_module {
                    Ok(resolved_path) => {
                        dependencies.insert(resolved_path);
                    }
                    Err(err) => {
                        // Log error but continue processing other imports
                        eprintln!("Warning: Failed to resolve import '{}': {}", module, err);
                    }
                }

                // Handle selective imports if needed
                if !imports.is_empty() {
                    // For selective imports like `import { a, b } from module`
                    // We still depend on the module, already added above
                    // Could add validation here that the imported items exist
                }
            }
        }

        dependencies
    }

    /// Resolve a module path to a canonical module name
    fn resolve_module_path(
        &self,
        module_path: &str,
        current_module_path: Option<&std::path::Path>,
    ) -> std::result::Result<String, String> {
        use std::path::Path;

        // Handle different import patterns
        if module_path.starts_with("./") || module_path.starts_with("../") {
            // Relative import
            if let Some(current_path) = current_module_path {
                let current_dir = current_path.parent().ok_or_else(|| {
                    format!("Cannot resolve relative import from root: {}", module_path)
                })?;

                let relative_path = Path::new(module_path);
                let resolved = current_dir.join(relative_path);

                // Normalize and convert to canonical module name
                let canonical = resolved
                    .canonicalize()
                    .map_err(|e| format!("Cannot resolve path '{}': {}", module_path, e))?;

                // Convert path to module name (remove .script extension, use :: separator)
                self.path_to_module_name(&canonical)
            } else {
                Err(format!(
                    "Relative import '{}' used without current module context",
                    module_path
                ))
            }
        } else if module_path.starts_with("/") {
            // Absolute import from project root
            let base = self.base_path.as_ref().ok_or_else(|| {
                format!("Absolute import '{}' used without base path", module_path)
            })?;

            let absolute_path = base.join(&module_path[1..]); // Remove leading /
            let canonical = absolute_path
                .canonicalize()
                .map_err(|e| format!("Cannot resolve absolute path '{}': {}", module_path, e))?;

            self.path_to_module_name(&canonical)
        } else {
            // Module name (library import or simple name)
            // For now, assume it's a valid module name
            Ok(module_path.to_string())
        }
    }

    /// Convert a file path to a module name
    fn path_to_module_name(&self, path: &std::path::Path) -> std::result::Result<String, String> {
        // Remove .script extension if present
        let path_str = path
            .to_str()
            .ok_or_else(|| "Invalid UTF-8 in module path".to_string())?;

        let module_path = if path_str.ends_with(".script") {
            &path_str[..path_str.len() - 7] // Remove .script
        } else {
            path_str
        };

        // Convert path separators to module separators
        let module_name = module_path.replace(std::path::MAIN_SEPARATOR, "::");

        // Remove base path if present
        if let Some(base) = &self.base_path {
            if let Some(base_str) = base.to_str() {
                let prefix = format!("{}::", base_str.replace(std::path::MAIN_SEPARATOR, "::"));
                if let Some(relative) = module_name.strip_prefix(&prefix) {
                    return Ok(relative.to_string());
                }
            }
        }

        Ok(module_name)
    }

    /// Legacy method for backward compatibility
    pub fn analyze_legacy(ast: &Program) -> HashSet<String> {
        let analyzer = Self::new();
        analyzer.analyze(ast, None)
    }

    /// Build a complete dependency graph from multiple modules with proper path resolution
    pub fn build_graph_with_paths(
        modules: &HashMap<String, (Program, std::path::PathBuf)>,
        base_path: Option<std::path::PathBuf>,
    ) -> DependencyGraph {
        let mut graph = DependencyGraph::new();
        let analyzer = Self::with_base_path(
            base_path.unwrap_or_else(|| std::env::current_dir().unwrap_or_default()),
        );

        // Add all modules
        for module_name in modules.keys() {
            graph.add_module(module_name.clone());
        }

        // Add dependencies with proper path resolution
        for (module_name, (ast, module_path)) in modules {
            let deps = analyzer.analyze(ast, Some(module_path));
            for dep in deps {
                // Only add if the dependency is an internal module
                if modules.contains_key(&dep) {
                    graph.add_dependency(module_name.clone(), dep);
                }
            }
        }

        graph
    }

    /// Build a complete dependency graph from multiple modules (legacy method)
    pub fn build_graph(modules: &HashMap<String, Program>) -> DependencyGraph {
        let mut graph = DependencyGraph::new();
        let analyzer = Self::new();

        // Add all modules
        for module_name in modules.keys() {
            graph.add_module(module_name.clone());
        }

        // Add dependencies using legacy analysis
        for (module_name, ast) in modules {
            let deps = analyzer.analyze(ast, None);
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
