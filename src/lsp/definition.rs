use crate::lexer::Lexer;
use crate::parser::{Expr, ExprKind, ImportSpecifier, Parser, Program, Stmt, StmtKind};
use crate::semantic::{SemanticAnalyzer, SymbolTable};
use crate::source::{SourceLocation, Span};
use std::path::PathBuf;
use tower_lsp::lsp_types::{Location, Position, Url};

/// Generate definition location for a symbol at the given position
pub fn goto_definition(content: &str, position: Position, uri: &Url) -> Option<Location> {
    // Parse the document
    let lexer = Lexer::new(content);
    let (tokens, _errors) = lexer.scan_tokens();

    let mut parser = Parser::new(tokens);
    let program = match parser.parse() {
        Ok(program) => program,
        Err(_) => return None,
    };

    // Perform semantic analysis to build symbol table
    let mut analyzer = SemanticAnalyzer::new();
    if analyzer.analyze_program(&program).is_err() {
        return None;
    }

    // Find the identifier at the given position
    let identifier_info = find_identifier_at_position(&program, position)?;

    // Look up the symbol
    let symbol = analyzer.symbol_table().lookup(&identifier_info.name)?;

    // Convert symbol's definition span to LSP location
    let location = Location {
        uri: determine_uri_for_symbol(&symbol, uri, analyzer.symbol_table()),
        range: span_to_range(&symbol.def_span),
    };

    Some(location)
}

/// Information about an identifier found at a position
struct IdentifierInfo {
    name: String,
    span: Span,
}

/// Find identifier at the given position in the AST
fn find_identifier_at_position(program: &Program, position: Position) -> Option<IdentifierInfo> {
    let target_location = SourceLocation::new(
        (position.line + 1) as usize,
        (position.character + 1) as usize,
        0, // We don't have offset info from position alone
    );

    // Search through all statements
    for stmt in &program.statements {
        if let Some(info) = find_identifier_in_stmt(stmt, &target_location) {
            return Some(info);
        }
    }

    None
}

/// Find identifier in a statement
fn find_identifier_in_stmt(stmt: &Stmt, target: &SourceLocation) -> Option<IdentifierInfo> {
    match &stmt.kind {
        StmtKind::Let { name, init, .. } => {
            // Check if target is on the variable name
            if stmt.span.contains_location(target) {
                // For now, assume it's the variable name if within the let statement
                // This is a simplification - we'd need more precise span info
                return Some(IdentifierInfo {
                    name: name.clone(),
                    span: stmt.span,
                });
            }

            // Check in the initializer expression
            if let Some(init_expr) = init {
                return find_identifier_in_expr(init_expr, target);
            }
        }
        StmtKind::Function { name, body, .. } => {
            // Check if target is on the function name
            if stmt.span.contains_location(target) {
                return Some(IdentifierInfo {
                    name: name.clone(),
                    span: stmt.span,
                });
            }

            // Check in function body
            for body_stmt in &body.statements {
                if let Some(info) = find_identifier_in_stmt(body_stmt, target) {
                    return Some(info);
                }
            }
        }
        StmtKind::Return(expr) => {
            if let Some(expr) = expr {
                return find_identifier_in_expr(expr, target);
            }
        }
        StmtKind::While {
            condition, body, ..
        } => {
            if let Some(info) = find_identifier_in_expr(condition, target) {
                return Some(info);
            }

            for body_stmt in &body.statements {
                if let Some(info) = find_identifier_in_stmt(body_stmt, target) {
                    return Some(info);
                }
            }
        }
        StmtKind::For {
            variable,
            iterable,
            body,
            ..
        } => {
            // Check if target is on the loop variable
            if stmt.span.contains_location(target) {
                return Some(IdentifierInfo {
                    name: variable.clone(),
                    span: stmt.span,
                });
            }

            if let Some(info) = find_identifier_in_expr(iterable, target) {
                return Some(info);
            }

            for body_stmt in &body.statements {
                if let Some(info) = find_identifier_in_stmt(body_stmt, target) {
                    return Some(info);
                }
            }
        }
        StmtKind::Expression(expr) => {
            return find_identifier_in_expr(expr, target);
        }
        StmtKind::Import { imports, .. } => {
            // Check import specifiers
            for spec in imports {
                // Need to check if position is on the import name
                if stmt.span.contains_location(target) {
                    let local_name = match spec {
                        ImportSpecifier::Default { name } => name.clone(),
                        ImportSpecifier::Named { alias, name, .. } => {
                            alias.as_ref().unwrap_or(name).clone()
                        }
                        ImportSpecifier::Namespace { alias } => alias.clone(),
                    };
                    return Some(IdentifierInfo {
                        name: local_name,
                        span: stmt.span,
                    });
                }
            }
        }
        StmtKind::Export { .. } => {
            // Handle exports if needed
        }
        StmtKind::Struct { name, .. } => {
            // Check if target is on the struct name
            if stmt.span.contains_location(target) {
                return Some(IdentifierInfo {
                    name: name.clone(),
                    span: stmt.span,
                });
            }
        }
        StmtKind::Enum { name, .. } => {
            // Check if target is on the enum name
            if stmt.span.contains_location(target) {
                return Some(IdentifierInfo {
                    name: name.clone(),
                    span: stmt.span,
                });
            }
        }
    }

    None
}

/// Find identifier in an expression
fn find_identifier_in_expr(expr: &Expr, target: &SourceLocation) -> Option<IdentifierInfo> {
    if !expr.span.contains_location(target) {
        return None;
    }

    match &expr.kind {
        ExprKind::Identifier(name) => Some(IdentifierInfo {
            name: name.clone(),
            span: expr.span,
        }),
        ExprKind::Binary { left, right, .. } => {
            find_identifier_in_expr(left, target).or_else(|| find_identifier_in_expr(right, target))
        }
        ExprKind::Unary { expr, .. } => find_identifier_in_expr(expr, target),
        ExprKind::Call { callee, args } => find_identifier_in_expr(callee, target).or_else(|| {
            for arg in args {
                if let Some(info) = find_identifier_in_expr(arg, target) {
                    return Some(info);
                }
            }
            None
        }),
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => find_identifier_in_expr(condition, target)
            .or_else(|| find_identifier_in_expr(then_branch, target))
            .or_else(|| {
                else_branch
                    .as_ref()
                    .and_then(|expr| find_identifier_in_expr(expr, target))
            }),
        ExprKind::Block(block) => {
            for stmt in &block.statements {
                if let Some(info) = find_identifier_in_stmt(stmt, target) {
                    return Some(info);
                }
            }
            // Also check final expression if any
            block
                .final_expr
                .as_ref()
                .and_then(|expr| find_identifier_in_expr(expr, target))
        }
        ExprKind::Assign {
            target: assign_target,
            value,
        } => {
            // Check if we're on the assignment target
            if let ExprKind::Identifier(name) = &assign_target.kind {
                if assign_target.span.contains_location(target) {
                    return Some(IdentifierInfo {
                        name: name.clone(),
                        span: assign_target.span,
                    });
                }
            }

            find_identifier_in_expr(value, target)
        }
        ExprKind::Member { object, property } => {
            find_identifier_in_expr(object, target).or_else(|| {
                // For member access, we might want to handle the member name too
                if expr.span.contains_location(target) {
                    Some(IdentifierInfo {
                        name: property.clone(),
                        span: expr.span,
                    })
                } else {
                    None
                }
            })
        }
        ExprKind::Index { object, index } => find_identifier_in_expr(object, target)
            .or_else(|| find_identifier_in_expr(index, target)),
        ExprKind::Array(elements) => {
            for elem in elements {
                if let Some(info) = find_identifier_in_expr(elem, target) {
                    return Some(info);
                }
            }
            None
        }
        ExprKind::Match {
            expr: match_expr,
            arms,
        } => find_identifier_in_expr(match_expr, target).or_else(|| {
            for arm in arms {
                if let Some(info) = find_identifier_in_expr(&arm.body, target) {
                    return Some(info);
                }
            }
            None
        }),
        ExprKind::ListComprehension { element, .. } => find_identifier_in_expr(element, target),
        ExprKind::Await { expr } => find_identifier_in_expr(expr, target),
        ExprKind::GenericConstructor { name, type_args: _ } => {
            if expr.span.contains_location(target) {
                Some(IdentifierInfo {
                    name: name.clone(),
                    span: expr.span.clone(),
                })
            } else {
                None
            }
        }
        ExprKind::StructConstructor { name, fields } => {
            // Check if target is on the struct name
            if expr.span.contains_location(target) {
                // Could be either the struct name or a field
                // For now, assume it's the struct name
                return Some(IdentifierInfo {
                    name: name.clone(),
                    span: expr.span,
                });
            }
            // Check fields for identifiers
            for (_, field_expr) in fields {
                if let Some(info) = find_identifier_in_expr(field_expr, target) {
                    return Some(info);
                }
            }
            None
        }
        ExprKind::EnumConstructor {
            enum_name,
            variant,
            args,
        } => {
            // Check if target is on the enum or variant name
            if expr.span.contains_location(target) {
                // For now, return the variant name
                return Some(IdentifierInfo {
                    name: variant.clone(),
                    span: expr.span,
                });
            }
            // Check constructor arguments
            match args {
                crate::parser::EnumConstructorArgs::Unit => None,
                crate::parser::EnumConstructorArgs::Tuple(exprs) => {
                    for arg_expr in exprs {
                        if let Some(info) = find_identifier_in_expr(arg_expr, target) {
                            return Some(info);
                        }
                    }
                    None
                }
                crate::parser::EnumConstructorArgs::Struct(fields) => {
                    for (_, field_expr) in fields {
                        if let Some(info) = find_identifier_in_expr(field_expr, target) {
                            return Some(info);
                        }
                    }
                    None
                }
            }
        }
        ExprKind::Literal(_) => None,
    }
}

/// Convert a span to an LSP range
fn span_to_range(span: &Span) -> tower_lsp::lsp_types::Range {
    tower_lsp::lsp_types::Range {
        start: Position {
            line: (span.start.line - 1) as u32,
            character: (span.start.column - 1) as u32,
        },
        end: Position {
            line: (span.end.line - 1) as u32,
            character: (span.end.column - 1) as u32,
        },
    }
}

/// Determine the URI for a symbol (handles imports from other files)
fn determine_uri_for_symbol(
    symbol: &crate::semantic::Symbol,
    current_uri: &Url,
    symbol_table: &SymbolTable,
) -> Url {
    // For now, assume all symbols are in the current file
    // In a full implementation, we'd check if this is an imported symbol
    // and resolve the module path to get the correct URI

    // Check if this symbol was imported
    let current_module_id = symbol_table.current_module();
    if let Some(module_info) = symbol_table.get_module(current_module_id) {
        for (_, import) in &module_info.imports {
            if import.source_symbol_id == symbol.id {
                // This is an imported symbol - try to resolve the module path
                if let Some(source_module) = symbol_table.get_module(import.source_module) {
                    // Convert module name to file path
                    let module_path = module_name_to_path(&source_module.name);
                    if let Ok(module_uri) = resolve_module_uri(current_uri, &module_path) {
                        return module_uri;
                    }
                }
            }
        }
    }

    // Default to current file
    current_uri.clone()
}

/// Convert a module name to a file path
fn module_name_to_path(module_name: &str) -> PathBuf {
    // Handle different module name formats
    if module_name.starts_with("./") || module_name.starts_with("../") {
        // Relative path
        PathBuf::from(module_name)
    } else if module_name.starts_with('/') {
        // Absolute path
        PathBuf::from(module_name)
    } else {
        // Module name - add .script extension if not present
        let mut path = PathBuf::from(module_name);
        if path.extension().is_none() {
            path.set_extension("script");
        }
        path
    }
}

/// Resolve a module path relative to the current file
fn resolve_module_uri(current_uri: &Url, module_path: &PathBuf) -> Result<Url, ()> {
    // Get the directory of the current file
    let current_path = current_uri.to_file_path().map_err(|_| ())?;
    let current_dir = current_path.parent().ok_or(())?;

    // Resolve the module path
    let resolved_path = if module_path.is_absolute() {
        module_path.clone()
    } else {
        current_dir.join(module_path)
    };

    // Convert back to URI
    Url::from_file_path(resolved_path).map_err(|_| ())
}

/// Extension methods for Span
trait SpanExt {
    fn contains_location(&self, location: &SourceLocation) -> bool;
}

impl SpanExt for Span {
    fn contains_location(&self, location: &SourceLocation) -> bool {
        // Check if the location is within this span
        if location.line < self.start.line || location.line > self.end.line {
            return false;
        }

        if location.line == self.start.line && location.column < self.start.column {
            return false;
        }

        if location.line == self.end.line && location.column > self.end.column {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::Position;

    #[test]
    fn test_goto_definition_variable() {
        let content = r#"
let x = 42;
let y = x + 1;
"#;

        // Position on 'x' in 'x + 1'
        let position = Position {
            line: 2,
            character: 8,
        };

        let uri = Url::parse("file:///test.script").unwrap();
        let location = goto_definition(content, position, &uri);

        assert!(location.is_some());
        let loc = location.unwrap();
        assert_eq!(loc.uri, uri);
        assert_eq!(loc.range.start.line, 1);
    }

    #[test]
    fn test_goto_definition_function() {
        let content = r#"
fn add(x: i32, y: i32) -> i32 {
    x + y
}

let result = add(1, 2);
"#;

        // Position on 'add' in function call
        let position = Position {
            line: 5,
            character: 13,
        };

        let uri = Url::parse("file:///test.script").unwrap();
        let location = goto_definition(content, position, &uri);

        assert!(location.is_some());
        let loc = location.unwrap();
        assert_eq!(loc.uri, uri);
        assert_eq!(loc.range.start.line, 1);
    }
}
