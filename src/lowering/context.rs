use crate::ir::{BlockId, FunctionId, ValueId};
use crate::types::Type;
use std::collections::HashMap;

/// Variable information in the lowering context
#[derive(Debug, Clone)]
pub struct Variable {
    /// Pointer to the variable's memory location
    pub ptr: ValueId,
    /// Type of the variable
    pub ty: Type,
}

/// Lowering context maintains state during AST to IR lowering
#[derive(Debug)]
pub struct LoweringContext {
    /// Stack of variable scopes
    scopes: Vec<HashMap<String, Variable>>,
    /// Function name to ID mapping
    functions: HashMap<String, FunctionId>,
    /// Current function being lowered
    current_function: Option<FunctionId>,
    /// Loop stack for break/continue targets
    loop_stack: Vec<LoopContext>,
}

/// Context for loop constructs
#[derive(Debug, Clone)]
pub struct LoopContext {
    /// Block to jump to for continue
    pub continue_block: BlockId,
    /// Block to jump to for break
    pub break_block: BlockId,
}

impl LoweringContext {
    /// Create a new lowering context
    pub fn new() -> Self {
        LoweringContext {
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
            current_function: None,
            loop_stack: Vec::new(),
        }
    }

    /// Push a new variable scope
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop the current variable scope
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Define a variable in the current scope
    pub fn define_variable(&mut self, name: String, ptr: ValueId, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, Variable { ptr, ty });
        }
    }

    /// Look up a variable
    pub fn lookup_variable(&self, name: &str) -> Option<&Variable> {
        // Search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            if let Some(var) = scope.get(name) {
                return Some(var);
            }
        }
        None
    }

    /// Register a function
    pub fn register_function(&mut self, name: String, id: FunctionId) {
        self.functions.insert(name, id);
    }

    /// Get a function by name
    pub fn get_function(&self, name: &str) -> Option<FunctionId> {
        self.functions.get(name).copied()
    }

    /// Enter a function
    pub fn enter_function(&mut self, func_id: FunctionId) {
        self.current_function = Some(func_id);
        // Create a new scope for function parameters and locals
        self.push_scope();
    }

    /// Exit the current function
    pub fn exit_function(&mut self) {
        self.current_function = None;
        self.pop_scope();
    }

    /// Get the current function
    pub fn current_function(&self) -> Option<FunctionId> {
        self.current_function
    }

    /// Push a loop context
    pub fn push_loop(&mut self, continue_block: BlockId, break_block: BlockId) {
        self.loop_stack.push(LoopContext {
            continue_block,
            break_block,
        });
    }

    /// Pop a loop context
    pub fn pop_loop(&mut self) {
        self.loop_stack.pop();
    }

    /// Get the current loop context
    pub fn current_loop(&self) -> Option<&LoopContext> {
        self.loop_stack.last()
    }
}

impl Default for LoweringContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_scoping() {
        let mut ctx = LoweringContext::new();

        // Define in outer scope
        ctx.define_variable("x".to_string(), ValueId(0), Type::I32);
        assert!(ctx.lookup_variable("x").is_some());

        // Push new scope
        ctx.push_scope();

        // Can still see outer variable
        assert!(ctx.lookup_variable("x").is_some());

        // Shadow in inner scope
        ctx.define_variable("x".to_string(), ValueId(1), Type::F32);
        let var = ctx.lookup_variable("x").unwrap();
        assert_eq!(var.ptr, ValueId(1));
        assert_eq!(var.ty, Type::F32);

        // Define new variable in inner scope
        ctx.define_variable("y".to_string(), ValueId(2), Type::Bool);
        assert!(ctx.lookup_variable("y").is_some());

        // Pop inner scope
        ctx.pop_scope();

        // Back to outer x
        let var = ctx.lookup_variable("x").unwrap();
        assert_eq!(var.ptr, ValueId(0));
        assert_eq!(var.ty, Type::I32);

        // y is gone
        assert!(ctx.lookup_variable("y").is_none());
    }

    #[test]
    fn test_function_registry() {
        let mut ctx = LoweringContext::new();

        ctx.register_function("main".to_string(), FunctionId(0));
        ctx.register_function("add".to_string(), FunctionId(1));

        assert_eq!(ctx.get_function("main"), Some(FunctionId(0)));
        assert_eq!(ctx.get_function("add"), Some(FunctionId(1)));
        assert_eq!(ctx.get_function("unknown"), None);
    }

    #[test]
    fn test_loop_context() {
        let mut ctx = LoweringContext::new();

        // No loop initially
        assert!(ctx.current_loop().is_none());

        // Push a loop
        ctx.push_loop(BlockId(1), BlockId(2));
        let loop_ctx = ctx.current_loop().unwrap();
        assert_eq!(loop_ctx.continue_block, BlockId(1));
        assert_eq!(loop_ctx.break_block, BlockId(2));

        // Nested loop
        ctx.push_loop(BlockId(3), BlockId(4));
        let loop_ctx = ctx.current_loop().unwrap();
        assert_eq!(loop_ctx.continue_block, BlockId(3));
        assert_eq!(loop_ctx.break_block, BlockId(4));

        // Pop inner loop
        ctx.pop_loop();
        let loop_ctx = ctx.current_loop().unwrap();
        assert_eq!(loop_ctx.continue_block, BlockId(1));
        assert_eq!(loop_ctx.break_block, BlockId(2));

        // Pop outer loop
        ctx.pop_loop();
        assert!(ctx.current_loop().is_none());
    }
}
