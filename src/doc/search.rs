use super::*;

/// Search functionality for documentation
pub struct SearchEngine {
    /// The search index
    index: SearchIndex,
}

impl SearchEngine {
    /// Create a new search engine from a documentation database
    pub fn new(database: &DocDatabase) -> Self {
        let mut engine = Self {
            index: SearchIndex::default(),
        };

        // Build the search index
        engine.build_index(database);

        engine
    }

    /// Build the search index from the documentation database
    fn build_index(&mut self, database: &DocDatabase) {
        for (module_path, module) in &database.modules {
            // Index module name
            self.add_module_to_index(module_path, module);

            // Index functions
            for func in &module.functions {
                self.add_function_to_index(module_path, func);
            }

            // Index types
            for type_doc in &module.types {
                self.add_type_to_index(module_path, type_doc);
            }

            // Index constants
            for const_doc in &module.constants {
                self.add_constant_to_index(module_path, const_doc);
            }
        }
    }

    /// Add a module to the search index
    fn add_module_to_index(&mut self, path: &str, module: &ModuleDoc) {
        let summary = module
            .documentation
            .as_ref()
            .map(|d| d.sections.description.lines().next().unwrap_or(""))
            .unwrap_or("")
            .to_string();

        let result = SearchResult {
            path: path.to_string(),
            name: module.name.clone(),
            kind: ItemKind::Module,
            summary,
        };

        self.add_search_terms(&module.name, result);
    }

    /// Add a function to the search index
    fn add_function_to_index(&mut self, module_path: &str, func: &FunctionDoc) {
        let summary = func
            .documentation
            .as_ref()
            .map(|d| d.sections.description.lines().next().unwrap_or(""))
            .unwrap_or("")
            .to_string();

        let result = SearchResult {
            path: format!("{}::{module_path, func.name}"),
            name: func.name.clone(),
            kind: ItemKind::Function,
            summary,
        };

        self.add_search_terms(&func.name, result);
    }

    /// Add a type to the search index
    fn add_type_to_index(&mut self, module_path: &str, type_doc: &TypeDoc) {
        let summary = type_doc
            .documentation
            .as_ref()
            .map(|d| d.sections.description.lines().next().unwrap_or(""))
            .unwrap_or("")
            .to_string();

        let result = SearchResult {
            path: format!("{}::{module_path, type_doc.name}"),
            name: type_doc.name.clone(),
            kind: ItemKind::Type,
            summary,
        };

        self.add_search_terms(&type_doc.name, result.clone());

        // Also index methods
        for method in &type_doc.methods {
            let method_result = SearchResult {
                path: format!("{}::{}::{module_path, type_doc.name, method.name}"),
                name: method.name.clone(),
                kind: ItemKind::Method,
                summary: method
                    .documentation
                    .as_ref()
                    .map(|d| d.sections.description.lines().next().unwrap_or(""))
                    .unwrap_or("")
                    .to_string(),
            };

            self.add_search_terms(&method.name, method_result);
        }
    }

    /// Add a constant to the search index
    fn add_constant_to_index(&mut self, module_path: &str, const_doc: &ConstantDoc) {
        let summary = const_doc
            .documentation
            .as_ref()
            .map(|d| d.sections.description.lines().next().unwrap_or(""))
            .unwrap_or("")
            .to_string();

        let result = SearchResult {
            path: format!("{}::{module_path, const_doc.name}"),
            name: const_doc.name.clone(),
            kind: ItemKind::Constant,
            summary,
        };

        self.add_search_terms(&const_doc.name, result);
    }

    /// Add search terms for an item
    fn add_search_terms(&mut self, name: &str, result: SearchResult) {
        // Index by full name
        let terms = self
            .index
            .terms
            .entry(name.to_lowercase())
            .or_insert_with(Vec::new);
        if !terms.iter().any(|r| r.path == result.path) {
            terms.push(result.clone());
        }

        // Index by parts (split on underscore and camelCase)
        let parts = self.split_identifier(name);
        for part in parts {
            if part.len() > 2 {
                let terms = self
                    .index
                    .terms
                    .entry(part.to_lowercase())
                    .or_insert_with(Vec::new);
                if !terms.iter().any(|r| r.path == result.path) {
                    terms.push(result.clone());
                }
            }
        }
    }

    /// Split an identifier into searchable parts
    fn split_identifier(&self, name: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut prev_lower = false;

        for ch in name.chars() {
            if ch == '_' {
                if !current.is_empty() {
                    parts.push(current.clone());
                    current.clear();
                }
                prev_lower = false;
            } else if ch.is_uppercase() && prev_lower {
                // camelCase boundary
                if !current.is_empty() {
                    parts.push(current.clone());
                    current.clear();
                }
                current.push(ch);
                prev_lower = false;
            } else {
                current.push(ch);
                prev_lower = ch.is_lowercase();
            }
        }

        if !current.is_empty() {
            parts.push(current);
        }

        parts
    }

    /// Search for items matching a query
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Exact matches first
        if let Some(exact_matches) = self.index.terms.get(&query_lower) {
            for result in exact_matches {
                if seen.insert(result.path.clone()) {
                    results.push(result.clone());
                }
            }
        }

        // Prefix matches
        for (term, term_results) in &self.index.terms {
            if term.starts_with(&query_lower) && term != &query_lower {
                for result in term_results {
                    if seen.insert(result.path.clone()) {
                        results.push(result.clone());
                    }
                }
            }
        }

        // Substring matches
        for (term, term_results) in &self.index.terms {
            if term.contains(&query_lower) && !term.starts_with(&query_lower) {
                for result in term_results {
                    if seen.insert(result.path.clone()) {
                        results.push(result.clone());
                    }
                }
            }
        }

        // Sort by relevance
        results.sort_by(|a, b| {
            // Exact name matches first
            let a_exact = a.name.to_lowercase() == query_lower;
            let b_exact = b.name.to_lowercase() == query_lower;
            if a_exact && !b_exact {
                return std::cmp::Ordering::Less;
            }
            if !a_exact && b_exact {
                return std::cmp::Ordering::Greater;
            }

            // Then by name length (shorter is better)
            let a_len = a.name.len();
            let b_len = b.name.len();
            if a_len != b_len {
                return a_len.cmp(&b_len);
            }

            // Finally alphabetically
            a.name.cmp(&b.name)
        });

        results
    }

    /// Get the search index
    pub fn index(&self) -> &SearchIndex {
        &self.index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_identifier() {
        let engine = SearchEngine::new(&DocDatabase::default());

        // Test underscore splitting
        let parts = engine.split_identifier("hello_world_test");
        assert_eq!(parts, vec!["hello", "world", "test"]);

        // Test camelCase splitting
        let parts = engine.split_identifier("helloWorldTest");
        assert_eq!(parts, vec!["hello", "World", "Test"]);

        // Test mixed
        let parts = engine.split_identifier("hello_worldTest");
        assert_eq!(parts, vec!["hello", "world", "Test"]);
    }
}
