use super::{Function, FunctionId, Parameter};
use crate::types::Type;
use std::collections::HashMap;
use std::fmt;

/// IR Module containing functions and global data
#[derive(Debug, Clone)]
pub struct Module {
    /// Module name
    pub name: String,
    /// Functions in this module
    functions: HashMap<FunctionId, Function>,
    /// Function name to ID mapping
    function_names: HashMap<String, FunctionId>,
    /// Next function ID to allocate
    next_function_id: u32,
    /// External function declarations
    external_functions: HashMap<String, Type>,
}

impl Module {
    /// Create a new module
    pub fn new() -> Self {
        let mut module = Module {
            name: "main".to_string(),
            functions: HashMap::new(),
            function_names: HashMap::new(),
            next_function_id: 0,
            external_functions: HashMap::new(),
        };
        
        // Register built-in functions
        module.declare_external_function("print".to_string(), Type::Function {
            params: vec![Type::String],
            ret: Box::new(Type::Unknown),
        });
        
        module
    }
    
    /// Create a module with a specific name
    pub fn with_name(name: String) -> Self {
        let mut module = Self::new();
        module.name = name;
        module
    }
    
    /// Create a new function in this module
    pub fn create_function(&mut self, name: String, params: Vec<Parameter>, return_type: Type) -> FunctionId {
        let func_id = FunctionId(self.next_function_id);
        self.next_function_id += 1;
        
        let function = Function::new(func_id, name.clone(), params, return_type);
        self.functions.insert(func_id, function);
        self.function_names.insert(name, func_id);
        
        func_id
    }
    
    /// Get a function by ID
    pub fn get_function(&self, id: FunctionId) -> Option<&Function> {
        self.functions.get(&id)
    }
    
    /// Get a mutable reference to a function
    pub fn get_function_mut(&mut self, id: FunctionId) -> Option<&mut Function> {
        self.functions.get_mut(&id)
    }
    
    /// Get a function by name
    pub fn get_function_by_name(&self, name: &str) -> Option<&Function> {
        self.function_names.get(name)
            .and_then(|id| self.get_function(*id))
    }
    
    /// Get function ID by name
    pub fn get_function_id(&self, name: &str) -> Option<FunctionId> {
        self.function_names.get(name).copied()
    }
    
    /// Get all functions
    pub fn functions(&self) -> &HashMap<FunctionId, Function> {
        &self.functions
    }
    
    /// Declare an external function
    pub fn declare_external_function(&mut self, name: String, ty: Type) {
        self.external_functions.insert(name, ty);
    }
    
    /// Check if a function is external
    pub fn is_external_function(&self, name: &str) -> bool {
        self.external_functions.contains_key(name)
    }
    
    /// Get the type of an external function
    pub fn get_external_function_type(&self, name: &str) -> Option<&Type> {
        self.external_functions.get(name)
    }
    
    /// Validate the module
    pub fn validate(&self) -> Result<(), String> {
        // Check that all functions have terminators in all blocks
        for func in self.functions.values() {
            for block in func.blocks().values() {
                if !block.has_terminator() && !block.instructions.is_empty() {
                    return Err(format!(
                        "Block {} in function {} lacks a terminator",
                        block.name, func.name
                    ));
                }
            }
            
            // Check that entry block exists
            if func.entry_block.is_none() && !func.blocks().is_empty() {
                return Err(format!(
                    "Function {} has blocks but no entry block",
                    func.name
                ));
            }
        }
        
        Ok(())
    }
}

impl Default for Module {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "; Module: {}", self.name)?;
        writeln!(f)?;
        
        // External declarations
        if !self.external_functions.is_empty() {
            writeln!(f, "; External functions:")?;
            for (name, ty) in &self.external_functions {
                writeln!(f, "declare {} {}", ty, name)?;
            }
            writeln!(f)?;
        }
        
        // Functions
        for func in self.functions.values() {
            write!(f, "{}", func)?;
            writeln!(f)?;
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::instruction::{Instruction, Constant};
    use crate::ir::ValueId;
    
    #[test]
    fn test_module_creation() {
        let module = Module::new();
        assert_eq!(module.name, "main");
        assert!(module.functions.is_empty());
        assert!(module.is_external_function("print"));
    }
    
    #[test]
    fn test_create_function() {
        let mut module = Module::new();
        
        let params = vec![
            Parameter { name: "x".to_string(), ty: Type::I32 },
        ];
        
        let func_id = module.create_function("test".to_string(), params, Type::I32);
        
        assert!(module.get_function(func_id).is_some());
        assert!(module.get_function_by_name("test").is_some());
        assert_eq!(module.get_function_id("test"), Some(func_id));
    }
    
    #[test]
    fn test_external_functions() {
        let mut module = Module::new();
        
        // Built-in print function
        assert!(module.is_external_function("print"));
        
        // Add custom external function
        module.declare_external_function("malloc".to_string(), Type::Function {
            params: vec![Type::I32],
            ret: Box::new(Type::Named("ptr".to_string())),
        });
        
        assert!(module.is_external_function("malloc"));
        assert!(module.get_external_function_type("malloc").is_some());
    }
    
    #[test]
    fn test_module_validation() {
        let mut module = Module::new();
        
        // Empty module should validate
        assert!(module.validate().is_ok());
        
        // Create a function with proper terminator
        let func_id = module.create_function("valid".to_string(), vec![], Type::I32);
        
        if let Some(func) = module.get_function_mut(func_id) {
            let entry = func.create_block("entry".to_string());
            if let Some(block) = func.get_block_mut(entry) {
                block.add_instruction(ValueId(0), Instruction::Return(None));
            }
        }
        
        assert!(module.validate().is_ok());
        
        // Create a function without terminator
        let bad_func_id = module.create_function("invalid".to_string(), vec![], Type::I32);
        
        if let Some(func) = module.get_function_mut(bad_func_id) {
            let entry = func.create_block("entry".to_string());
            if let Some(block) = func.get_block_mut(entry) {
                // Add non-terminator instruction
                block.add_instruction(ValueId(0), Instruction::Const(Constant::I32(42)));
            }
        }
        
        assert!(module.validate().is_err());
    }
    
    #[test]
    fn test_module_display() {
        let mut module = Module::with_name("test_module".to_string());
        
        let params = vec![
            Parameter { name: "n".to_string(), ty: Type::I32 },
        ];
        
        let func_id = module.create_function("factorial".to_string(), params, Type::I32);
        
        if let Some(func) = module.get_function_mut(func_id) {
            let entry = func.create_block("entry".to_string());
            if let Some(block) = func.get_block_mut(entry) {
                block.add_instruction(ValueId(0), Instruction::Return(Some(ValueId(999))));
            }
        }
        
        let output = module.to_string();
        assert!(output.contains("Module: test_module"));
        assert!(output.contains("declare (string) -> unknown print"));
        assert!(output.contains("fn @0 factorial"));
    }
}