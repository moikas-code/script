use crate::error::{Error, ErrorKind, Result};
use crate::parser::{
    Attribute, Block, Expr, ExprKind, Literal, Param, Stmt, StmtKind, TypeAnn, TypeKind,
};
use crate::source::Span;
use std::collections::HashMap;

/// Processor for @derive attributes that generates trait implementations
pub struct DeriveProcessor {
    /// Registered derive handlers
    handlers: HashMap<String, Box<dyn DeriveHandler>>,
}

impl DeriveProcessor {
    pub fn new() -> Self {
        let mut processor = Self {
            handlers: HashMap::new(),
        };

        // Register built-in derive handlers
        processor.register_handler("Debug", Box::new(DebugDeriveHandler));
        processor.register_handler("Serialize", Box::new(SerializeDeriveHandler));

        processor
    }

    /// Register a new derive handler
    pub fn register_handler(&mut self, name: &str, handler: Box<dyn DeriveHandler>) {
        self.handlers.insert(name.to_string(), handler);
    }

    /// Process a derive attribute and generate implementations
    pub fn process_derive(
        &self,
        attr: &Attribute,
        type_name: &str,
        original: &Stmt,
    ) -> Result<Vec<Stmt>> {
        let mut generated = Vec::new();

        for arg in &attr.args {
            if let Some(handler) = self.handlers.get(arg) {
                let stmts = handler.generate(type_name, original)?;
                generated.extend(stmts);
            } else {
                return Err(Error::new(
                    ErrorKind::SemanticError,
                    &format!("Unknown derive trait: {}", arg),
                ));
            }
        }

        Ok(generated)
    }
}

/// Trait for implementing derive handlers
pub trait DeriveHandler: Send + Sync {
    /// Generate implementation for the given type
    fn generate(&self, type_name: &str, original: &Stmt) -> Result<Vec<Stmt>>;
}

/// Handler for @derive(Debug)
struct DebugDeriveHandler;

impl DeriveHandler for DebugDeriveHandler {
    fn generate(&self, type_name: &str, original: &Stmt) -> Result<Vec<Stmt>> {
        // Generate a debug() method that returns a string representation
        let debug_fn = Stmt {
            kind: StmtKind::Function {
                name: format!("{}_debug", type_name),
                generic_params: None,
                params: vec![Param {
                    name: "self".to_string(),
                    type_ann: TypeAnn {
                        kind: TypeKind::Named(type_name.to_string()),
                        span: Span::dummy(),
                    },
                }],
                ret_type: Some(TypeAnn {
                    kind: TypeKind::Named("String".to_string()),
                    span: Span::dummy(),
                }),
                body: Block {
                    statements: vec![],
                    final_expr: Some(Box::new(Expr {
                        kind: ExprKind::Literal(Literal::String(format!("{}{{...}}", type_name))),
                        span: Span::dummy(),
                    })),
                },
                is_async: false,
            },
            span: Span::dummy(),
            attributes: vec![],
        };

        Ok(vec![debug_fn])
    }
}

/// Handler for @derive(Serialize)
struct SerializeDeriveHandler;

impl DeriveHandler for SerializeDeriveHandler {
    fn generate(&self, type_name: &str, original: &Stmt) -> Result<Vec<Stmt>> {
        // Generate a serialize() method that converts to JSON
        let serialize_fn = Stmt {
            kind: StmtKind::Function {
                name: format!("{}_serialize", type_name),
                generic_params: None,
                params: vec![Param {
                    name: "self".to_string(),
                    type_ann: TypeAnn {
                        kind: TypeKind::Named(type_name.to_string()),
                        span: Span::dummy(),
                    },
                }],
                ret_type: Some(TypeAnn {
                    kind: TypeKind::Named("String".to_string()),
                    span: Span::dummy(),
                }),
                body: Block {
                    statements: vec![],
                    final_expr: Some(Box::new(Expr {
                        kind: ExprKind::Literal(Literal::String("{}".to_string())),
                        span: Span::dummy(),
                    })),
                },
                is_async: false,
            },
            span: Span::dummy(),
            attributes: vec![],
        };

        Ok(vec![serialize_fn])
    }
}
