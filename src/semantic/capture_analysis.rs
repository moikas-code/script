use crate::parser::{Block, ClosureParam, Expr, ExprKind};
use crate::semantic::symbol::{Symbol, SymbolId};
use crate::semantic::symbol_table::{ScopeId, SymbolTable};
use crate::types::Type;
use std::collections::{HashMap, HashSet};

/// Information about a captured variable
#[derive(Debug, Clone, PartialEq)]
pub struct CaptureInfo {
    /// Name of the captured variable
    pub name: String,
    /// Symbol ID of the captured variable
    pub symbol_id: SymbolId,
    /// Type of the captured variable
    pub ty: Type,
    /// How the variable is captured
    pub capture_mode: CaptureMode,
    /// Scope where the variable is defined
    pub definition_scope: ScopeId,
}

/// How a variable is captured by a closure
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    /// Capture by value (copy/move)
    ByValue,
    /// Capture by reference
    ByReference,
}

/// Context for tracking variables across scopes during capture analysis
#[derive(Debug)]
struct ScopeContext {
    /// Current scope being analyzed
    current_scope: ScopeId,
    /// Variables defined in the closure (parameters)
    closure_locals: HashSet<String>,
    /// Free variables found so far
    free_variables: HashMap<String, SymbolId>,
}

/// Analyzes closures to determine captured variables
pub struct CaptureAnalyzer<'a> {
    /// Reference to the symbol table
    symbol_table: &'a SymbolTable,
}

impl<'a> CaptureAnalyzer<'a> {
    /// Create a new capture analyzer
    pub fn new(symbol_table: &'a SymbolTable) -> Self {
        Self { symbol_table }
    }

    /// Analyze a closure expression to determine captured variables
    pub fn analyze_closure(
        &self,
        parameters: &[ClosureParam],
        body: &Expr,
        closure_scope: ScopeId,
    ) -> Vec<CaptureInfo> {
        // Create context for analysis
        let mut context = ScopeContext {
            current_scope: closure_scope,
            closure_locals: HashSet::new(),
            free_variables: HashMap::new(),
        };

        // Add closure parameters to local variables
        for param in parameters {
            context.closure_locals.insert(param.name.clone());
        }

        // Find free variables in the closure body
        self.find_free_variables(body, &mut context);

        // Determine capture mode for each free variable
        let mut captures = Vec::new();
        for (name, symbol_id) in context.free_variables {
            if let Some(symbol) = self.symbol_table.get_symbol(symbol_id) {
                let capture_mode = self.determine_capture_mode(symbol);
                captures.push(CaptureInfo {
                    name,
                    symbol_id,
                    ty: symbol.ty.clone(),
                    capture_mode,
                    definition_scope: symbol.scope_id,
                });
            }
        }

        captures
    }

    /// Find all free variables referenced in an expression
    fn find_free_variables(&self, expr: &Expr, context: &mut ScopeContext) {
        match &expr.kind {
            ExprKind::Identifier(name) => {
                // Check if it's a local variable or needs to be captured
                if !context.closure_locals.contains(name) {
                    // Try to find the variable in outer scopes
                    if let Some(symbol) = self
                        .symbol_table
                        .lookup_in_scope(name, context.current_scope)
                    {
                        context.free_variables.insert(name.clone(), symbol.id);
                    }
                }
            }
            ExprKind::Binary { left, right, .. } => {
                self.find_free_variables(left, context);
                self.find_free_variables(right, context);
            }
            ExprKind::Unary { expr, .. } => {
                self.find_free_variables(expr, context);
            }
            ExprKind::Call { callee, args } => {
                self.find_free_variables(callee, context);
                for arg in args {
                    self.find_free_variables(arg, context);
                }
            }
            // Method calls are represented as Member + Call in this AST
            ExprKind::Member { object, .. } => {
                self.find_free_variables(object, context);
            }
            ExprKind::Index { object, index } => {
                self.find_free_variables(object, context);
                self.find_free_variables(index, context);
            }
            ExprKind::Array(elements) => {
                for element in elements {
                    self.find_free_variables(element, context);
                }
            }
            // Tuples are not a separate variant in this AST
            ExprKind::StructConstructor { fields, .. } => {
                for (_, field_expr) in fields {
                    self.find_free_variables(field_expr, context);
                }
            }
            ExprKind::EnumConstructor { args, .. } => match args {
                crate::parser::EnumConstructorArgs::Unit => {}
                crate::parser::EnumConstructorArgs::Tuple(exprs) => {
                    for expr in exprs {
                        self.find_free_variables(expr, context);
                    }
                }
                crate::parser::EnumConstructorArgs::Struct(fields) => {
                    for (_, expr) in fields {
                        self.find_free_variables(expr, context);
                    }
                }
            },
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.find_free_variables(condition, context);
                self.find_free_variables(then_branch, context);
                if let Some(else_expr) = else_branch {
                    self.find_free_variables(else_expr, context);
                }
            }
            // While loops are statements, not expressions in this AST
            // For loops are statements, not expressions in this AST
            ExprKind::Match { expr, arms } => {
                self.find_free_variables(expr, context);
                for arm in arms {
                    // Note: patterns introduce new bindings in arm scope
                    if let Some(guard) = &arm.guard {
                        self.find_free_variables(guard, context);
                    }
                    self.find_free_variables(&arm.body, context);
                }
            }
            ExprKind::Block(block) => {
                self.find_free_variables_in_block(block, context);
            }
            ExprKind::Assign { target, value } => {
                self.find_free_variables(target, context);
                self.find_free_variables(value, context);
            }
            // Compound assignments are not a separate variant in this AST
            // Return is a statement, not an expression in this AST
            ExprKind::Closure {
                parameters: _,
                body,
            } => {
                // Nested closure - analyze it separately with its own context
                // For now, we just analyze the body in the current context
                // TODO: Handle nested closures properly
                self.find_free_variables(body, context);
            }
            // References are not a separate variant in this AST
            // Dereferences are not a separate variant in this AST
            // Type casts are not a separate variant in this AST
            // Ranges are not a separate variant in this AST
            ExprKind::TryCatch {
                try_expr,
                catch_clauses,
                finally_block,
            } => {
                self.find_free_variables(try_expr, context);
                for clause in catch_clauses {
                    if let Some(cond) = &clause.condition {
                        self.find_free_variables(cond, context);
                    }
                    self.find_free_variables_in_block(&clause.handler, context);
                }
                if let Some(finally) = finally_block {
                    self.find_free_variables_in_block(finally, context);
                }
            }
            ExprKind::ErrorPropagation { expr } => {
                self.find_free_variables(expr, context);
            }
            ExprKind::Await { expr } => {
                self.find_free_variables(expr, context);
            }
            ExprKind::ListComprehension {
                element,
                iterable,
                condition,
                ..
            } => {
                // Note: variable introduces new binding in comprehension scope
                self.find_free_variables(iterable, context);
                if let Some(cond) = condition {
                    self.find_free_variables(cond, context);
                }
                self.find_free_variables(element, context);
            }
            ExprKind::Literal(_) => {
                // No variables to capture
            }
            ExprKind::GenericConstructor { .. } => {
                // Type constructor, no runtime variables
            }
        }
    }

    /// Find free variables in a block
    fn find_free_variables_in_block(&self, block: &Block, context: &mut ScopeContext) {
        // Note: Blocks introduce new scopes, but for capture analysis
        // we're interested in variables from outside the closure
        for stmt in &block.statements {
            match &stmt.kind {
                crate::parser::StmtKind::Expression(expr) => {
                    self.find_free_variables(expr, context);
                }
                crate::parser::StmtKind::Let { init, .. } => {
                    if let Some(init_expr) = init {
                        self.find_free_variables(init_expr, context);
                    }
                    // Note: The variable being defined is added to the current scope,
                    // not captured
                }
                crate::parser::StmtKind::Return(expr) => {
                    if let Some(return_expr) = expr {
                        self.find_free_variables(return_expr, context);
                    }
                }
                crate::parser::StmtKind::While { condition, body } => {
                    self.find_free_variables(condition, context);
                    self.find_free_variables_in_block(body, context);
                }
                crate::parser::StmtKind::For { iterable, body, .. } => {
                    self.find_free_variables(iterable, context);
                    self.find_free_variables_in_block(body, context);
                }
                _ => {
                    // Other statement kinds don't contain expressions that could
                    // reference variables
                }
            }
        }
    }

    /// Determine how a variable should be captured
    fn determine_capture_mode(&self, symbol: &Symbol) -> CaptureMode {
        // Simple heuristic for now:
        // - Mutable variables are captured by reference
        // - Immutable variables are captured by value
        if symbol.is_mutable {
            CaptureMode::ByReference
        } else {
            CaptureMode::ByValue
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::analyzer::SemanticAnalyzer;
    use crate::{Lexer, Parser};

    fn setup_test(code: &str) -> (SemanticAnalyzer, Expr) {
        let lexer = Lexer::new(code).unwrap();
        let (tokens, _) = lexer.scan_tokens();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze_program(&program).unwrap();

        // Extract the first expression (assuming it's our closure)
        let expr = match &program.statements[0].kind {
            crate::parser::StmtKind::Expression(expr) => expr.clone(),
            _ => panic!("Expected expression statement"),
        };

        (analyzer, expr)
    }

    #[test]
    fn test_simple_capture() {
        let code = r#"
            let x = 42;
            let closure = |y| x + y;
        "#;

        let (analyzer, expr) = setup_test(code);

        // Extract closure from let statement
        let closure_expr = match &expr.kind {
            ExprKind::Block(block) => match &block.statements[1].kind {
                crate::parser::StmtKind::Let { init, .. } => init.as_ref().unwrap(),
                _ => panic!("Expected let statement"),
            },
            _ => panic!("Expected block"),
        };

        match &closure_expr.kind {
            ExprKind::Closure { parameters, body } => {
                let capture_analyzer = CaptureAnalyzer::new(analyzer.symbol_table());
                let captures = capture_analyzer.analyze_closure(
                    parameters,
                    body,
                    analyzer.symbol_table().current_scope(),
                );

                assert_eq!(captures.len(), 1);
                assert_eq!(captures[0].name, "x");
                assert_eq!(captures[0].capture_mode, CaptureMode::ByValue);
            }
            _ => panic!("Expected closure"),
        }
    }

    #[test]
    fn test_mutable_capture() {
        let code = r#"
            let mut counter = 0;
            let increment = || {
                counter = counter + 1;
            };
        "#;

        let (analyzer, expr) = setup_test(code);

        // Extract closure from let statement
        let closure_expr = match &expr.kind {
            ExprKind::Block(block) => match &block.statements[1].kind {
                crate::parser::StmtKind::Let { init, .. } => init.as_ref().unwrap(),
                _ => panic!("Expected let statement"),
            },
            _ => panic!("Expected block"),
        };

        match &closure_expr.kind {
            ExprKind::Closure { parameters, body } => {
                let capture_analyzer = CaptureAnalyzer::new(analyzer.symbol_table());
                let captures = capture_analyzer.analyze_closure(
                    parameters,
                    body,
                    analyzer.symbol_table().current_scope(),
                );

                assert_eq!(captures.len(), 1);
                assert_eq!(captures[0].name, "counter");
                assert_eq!(captures[0].capture_mode, CaptureMode::ByReference);
            }
            _ => panic!("Expected closure"),
        }
    }

    #[test]
    fn test_no_captures() {
        let code = r#"
            let closure = |x, y| x + y;
        "#;

        let (analyzer, expr) = setup_test(code);

        // Extract closure from let statement
        let closure_expr = match &expr.kind {
            ExprKind::Block(block) => match &block.statements[0].kind {
                crate::parser::StmtKind::Let { init, .. } => init.as_ref().unwrap(),
                _ => panic!("Expected let statement"),
            },
            _ => panic!("Expected block"),
        };

        match &closure_expr.kind {
            ExprKind::Closure { parameters, body } => {
                let capture_analyzer = CaptureAnalyzer::new(analyzer.symbol_table());
                let captures = capture_analyzer.analyze_closure(
                    parameters,
                    body,
                    analyzer.symbol_table().current_scope(),
                );

                assert_eq!(captures.len(), 0);
            }
            _ => panic!("Expected closure"),
        }
    }
}
