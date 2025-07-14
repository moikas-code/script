use crate::error::{Error, ErrorKind, Result};
use crate::parser::{Attribute, Stmt};
use std::collections::HashMap;
use std::io::Write;
use std::process::Command;

/// Processor for @generate attributes that invoke external code generators
pub struct GenerateProcessor {
    /// Registered generators
    generators: HashMap<String, Box<dyn Generator>>,
}

impl GenerateProcessor {
    pub fn new() -> Self {
        let mut processor = Self {
            generators: HashMap::new(),
        };

        // Register built-in generators
        processor.register_generator("sql", Box::new(SqlGenerator));
        processor.register_generator("api", Box::new(ApiGenerator));

        processor
    }

    /// Register a new generator
    pub fn register_generator(&mut self, name: &str, generator: Box<dyn Generator>) {
        self.generators.insert(name.to_string(), generator);
    }

    /// Process a generate attribute
    pub fn process_generate(&self, attr: &Attribute, stmt: &Stmt) -> Result<Vec<Stmt>> {
        if attr.args.is_empty() {
            return Err(Error::new(
                ErrorKind::SemanticError,
                "@generate requires at least one argument specifying the generator",
            ));
        }

        let generator_name = &attr.args[0];
        let generator_args = &attr.args[1..];

        if let Some(generator) = self.generators.get(generator_name) {
            generator.generate(stmt, generator_args)
        } else {
            Err(Error::new(
                ErrorKind::SemanticError,
                &format!("Unknown generator: {generator_name}"),
            ))
        }
    }
}

/// Trait for implementing code generators
pub trait Generator: Send + Sync {
    /// Generate code based on the input statement
    fn generate(&self, stmt: &Stmt, args: &[String]) -> Result<Vec<Stmt>>;
}

/// SQL generator for database access code
struct SqlGenerator;

impl Generator for SqlGenerator {
    fn generate(&self, _stmt: &Stmt, args: &[String]) -> Result<Vec<Stmt>> {
        // Example: @generate(sql, "users") generates CRUD operations
        if args.is_empty() {
            return Err(Error::new(
                ErrorKind::SemanticError,
                "SQL generator requires table name argument",
            ));
        }

        let _table_name = &args[0];

        // In a real implementation, this would:
        // 1. Connect to database or read schema file
        // 2. Generate appropriate CRUD functions
        // 3. Parse the generated code back into AST

        // For now, return a placeholder
        Ok(vec![])
    }
}

/// API generator for REST endpoints
struct ApiGenerator;

impl Generator for ApiGenerator {
    fn generate(&self, _stmt: &Stmt, args: &[String]) -> Result<Vec<Stmt>> {
        // Example: @generate(api, "openapi.yaml") generates client code from OpenAPI spec
        if args.is_empty() {
            return Err(Error::new(
                ErrorKind::SemanticError,
                "API generator requires specification file argument",
            ));
        }

        let _spec_file = &args[0];

        // In a real implementation, this would:
        // 1. Read the OpenAPI/Swagger specification
        // 2. Generate client functions for each endpoint
        // 3. Parse the generated code back into AST

        // For now, return a placeholder
        Ok(vec![])
    }
}

/// Helper to execute external generator programs
pub fn execute_external_generator(command: &str, input: &str) -> Result<String> {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            Error::new(
                ErrorKind::SemanticError,
                &format!("Failed to execute generator: {e}"),
            )
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes()).map_err(|e| {
            Error::new(
                ErrorKind::SemanticError,
                &format!("Failed to write to generator: {e}"),
            )
        })?;
    }

    let output = child.wait_with_output().map_err(|e| {
        Error::new(
            ErrorKind::SemanticError,
            &format!("Generator failed: {e}"),
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::new(
            ErrorKind::SemanticError,
            &format!("Generator failed: {stderr}"),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
