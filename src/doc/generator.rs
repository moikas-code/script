use super::*;
use crate::error::Result;
use crate::lexer::{Lexer, TokenKind};
use crate::parser::{Expr, ExprKind, Parser, Program, Stmt, StmtKind};
use std::collections::HashMap;

/// Documentation generator for Script programs
pub struct DocGenerator {
    /// Database to store all documentation
    database: DocDatabase,
    /// Current module being processed
    current_module: String,
    /// Pending documentation comments
    pending_docs: Vec<String>,
}

impl DocGenerator {
    pub fn new() -> Self {
        Self {
            database: DocDatabase::default(),
            current_module: String::new(),
            pending_docs: Vec::new(),
        }
    }

    /// Generate documentation for a program
    pub fn generate_from_program(&mut self, program: &Program, module_name: &str) -> Result<()> {
        self.current_module = module_name.to_string();

        // Create module entry
        let module = self
            .database
            .modules
            .entry(module_name.to_string())
            .or_insert_with(|| ModuleDoc {
                name: module_name.to_string(),
                path: module_name.to_string(),
                documentation: None,
                functions: Vec::new(),
                types: Vec::new(),
                constants: Vec::new(),
                submodules: Vec::new(),
            });

        // Process all statements
        for stmt in &program.statements {
            self.process_statement(stmt, &module_name);
        }

        Ok(())
    }

    /// Generate documentation from source code
    pub fn generate_from_source(&mut self, source: &str, module_name: &str) -> Result<()> {
        // First, collect all tokens including doc comments
        let lexer = Lexer::new(source)?;
        let (tokens, errors) = lexer.scan_tokens();

        if !errors.is_empty() {
            return Err(errors.into_iter().next().unwrap());
        }

        // Parse the program while tracking doc comments
        let mut parser = Parser::new(tokens.clone());
        let program = parser.parse()?;

        // Process tokens and AST together to associate doc comments
        self.process_with_doc_comments(&tokens, &program, module_name)?;

        Ok(())
    }

    /// Process tokens and AST together to extract documentation
    fn process_with_doc_comments(
        &mut self,
        tokens: &[crate::lexer::Token],
        program: &Program,
        module_name: &str,
    ) -> Result<()> {
        self.current_module = module_name.to_string();

        let module = self
            .database
            .modules
            .entry(module_name.to_string())
            .or_insert_with(|| ModuleDoc {
                name: module_name.to_string(),
                path: module_name.to_string(),
                documentation: None,
                functions: Vec::new(),
                types: Vec::new(),
                constants: Vec::new(),
                submodules: Vec::new(),
            });

        // Track doc comments and their positions
        let mut doc_comments: Vec<(usize, String)> = Vec::new();
        let mut token_positions: HashMap<usize, usize> = HashMap::new();

        // Collect doc comments and map token positions
        for (idx, token) in tokens.iter().enumerate() {
            token_positions.insert(token.span.start.byte_offset, idx);

            if let TokenKind::DocComment(content) = &token.kind {
                doc_comments.push((idx, content.clone()));
            }
        }

        // Process statements with their preceding doc comments
        for stmt in &program.statements {
            // Find doc comments immediately before this statement
            let stmt_token_idx = token_positions.get(&stmt.span.start.byte_offset);

            if let Some(&idx) = stmt_token_idx {
                // Collect all doc comments that appear right before this statement
                let mut stmt_docs = Vec::new();
                for &(doc_idx, ref content) in doc_comments.iter().rev() {
                    if doc_idx < idx {
                        // Check if there are only newlines between doc and statement
                        let between_tokens = &tokens[doc_idx + 1..idx];
                        let only_newlines = between_tokens
                            .iter()
                            .all(|t| matches!(t.kind, TokenKind::Newline));

                        if only_newlines {
                            stmt_docs.push(content.clone());
                        } else {
                            break;
                        }
                    }
                }

                // Reverse to get correct order
                stmt_docs.reverse();

                if !stmt_docs.is_empty() {
                    self.pending_docs = stmt_docs;
                }
            }

            self.process_statement(stmt, &module_name);
            self.pending_docs.clear();
        }

        Ok(())
    }

    /// Process a statement and extract documentation
    fn process_statement(&mut self, stmt: &Stmt, module_name: &str) {
        match &stmt.kind {
            StmtKind::Function {
                name,
                generic_params: _, // TODO: Include generic params in docs
                params,
                ret_type,
                body: _,
                is_async,
                where_clause: _, // TODO: Include where clause in docs
            } => {
                let mut signature = String::new();

                if *is_async {
                    signature.push_str("async ");
                }
                signature.push_str("fn ");
                signature.push_str(name);
                signature.push('(');

                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        signature.push_str(", ");
                    }
                    signature.push_str(&param.name);
                    signature.push_str(": ");
                    signature.push_str(&format!("{:?}", param.type_ann)); // TODO: proper type formatting
                }

                signature.push(')');

                if let Some(ret) = ret_type {
                    signature.push_str(" -> ");
                    signature.push_str(&format!("{:?}", ret)); // TODO: proper type formatting
                }

                let documentation = if !self.pending_docs.is_empty() {
                    Some(parse_doc_comments(self.pending_docs.clone()))
                } else {
                    None
                };

                let func_doc = FunctionDoc {
                    name: name.clone(),
                    signature,
                    documentation,
                    is_async: *is_async,
                    is_exported: stmt.attributes.iter().any(|a| a.name == "export"),
                };

                // Add to module - get mutable reference in limited scope
                {
                    let module = self.database.modules.get_mut(module_name).unwrap();
                    module.functions.push(func_doc);
                }

                // Add to search index - clone docs to avoid borrow conflict
                let docs_clone = self.pending_docs.clone();
                self.add_to_search_index(name, ItemKind::Function, &docs_clone);
            }

            StmtKind::Let {
                name,
                type_ann,
                init,
            } => {
                // Check if this is a constant (all uppercase name)
                if name.chars().all(|c| c.is_uppercase() || c == '_') {
                    let type_info = if let Some(type_ann) = type_ann {
                        format!("{:?}", type_ann) // TODO: proper type formatting
                    } else {
                        "unknown".to_string()
                    };

                    // Process expression value before borrowing module
                    let value = if let Some(expr) = init {
                        Some(self.expr_to_string(expr))
                    } else {
                        None
                    };

                    let documentation = if !self.pending_docs.is_empty() {
                        Some(parse_doc_comments(self.pending_docs.clone()))
                    } else {
                        None
                    };

                    let const_doc = ConstantDoc {
                        name: name.clone(),
                        type_info,
                        value,
                        documentation,
                    };

                    // Add to module - get mutable reference in limited scope
                    {
                        let module = self.database.modules.get_mut(module_name).unwrap();
                        module.constants.push(const_doc);
                    }

                    // Add to search index - clone docs to avoid borrow conflict
                    let docs_clone = self.pending_docs.clone();
                    self.add_to_search_index(name, ItemKind::Constant, &docs_clone);
                }
            }

            _ => {
                // Other statement types not yet supported for documentation
            }
        }
    }

    /// Convert an expression to a string representation
    fn expr_to_string(&self, expr: &Expr) -> String {
        match &expr.kind {
            ExprKind::Literal(lit) => format!("{:?}", lit),
            ExprKind::Identifier(name) => name.clone(),
            _ => "<expression>".to_string(),
        }
    }

    /// Add an item to the search index
    fn add_to_search_index(&mut self, name: &str, kind: ItemKind, docs: &[String]) {
        let summary = if !docs.is_empty() {
            // Take first line of documentation as summary
            docs[0].lines().next().unwrap_or("").to_string()
        } else {
            String::new()
        };

        let result = SearchResult {
            path: format!("{}::{}", self.current_module, name),
            name: name.to_string(),
            kind,
            summary,
        };

        // Add search terms
        let terms = self
            .database
            .search_index
            .terms
            .entry(name.to_lowercase())
            .or_insert_with(Vec::new);
        terms.push(result.clone());

        // Also index by parts of the name
        for part in name.split('_') {
            if part.len() > 2 {
                let terms = self
                    .database
                    .search_index
                    .terms
                    .entry(part.to_lowercase())
                    .or_insert_with(Vec::new);
                terms.push(result.clone());
            }
        }
    }

    /// Get the documentation database
    pub fn database(&self) -> &DocDatabase {
        &self.database
    }

    /// Take ownership of the documentation database
    pub fn into_database(self) -> DocDatabase {
        self.database
    }
}
