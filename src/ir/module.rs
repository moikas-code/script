use super::{Function, FunctionId, Parameter};
use crate::types::Type;
use crate::error::{Error, ErrorKind};
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
        module.declare_external_function(
            "print".to_string(),
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::Unknown),
            },
        );

        module
    }

    /// Create a module with a specific name
    pub fn with_name(name: String) -> Self {
        let mut module = Self::new();
        module.name = name;
        module
    }

    /// Create a new function in this module
    pub fn create_function(
        &mut self,
        name: String,
        params: Vec<Parameter>,
        return_type: Type,
    ) -> FunctionId {
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
        self.function_names
            .get(name)
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

    /// Add a function to the module
    pub fn add_function(&mut self, function: Function) -> Result<FunctionId, Error> {
        let func_id = function.id;
        let name = function.name.clone();
        
        // Check if function ID already exists
        if self.functions.contains_key(&func_id) {
            return Err(Error::new(
                ErrorKind::ModuleError,
                format!("Function ID {} already exists", func_id)
            ));
        }
        
        // Check if function name already exists
        if self.function_names.contains_key(&name) {
            return Err(Error::new(
                ErrorKind::ModuleError,
                format!("Function name '{}' already exists", name)
            ));
        }
        
        self.functions.insert(func_id, function);
        self.function_names.insert(name, func_id);
        
        // Update next function ID if needed
        if func_id.0 >= self.next_function_id {
            self.next_function_id = func_id.0 + 1;
        }
        
        Ok(func_id)
    }

    /// Add a specialized function to the module (allows name conflicts)
    pub fn add_specialized_function(&mut self, function: Function) -> Result<FunctionId, Error> {
        let func_id = function.id;
        let name = function.name.clone();
        
        // Check if function ID already exists
        if self.functions.contains_key(&func_id) {
            return Err(Error::new(
                ErrorKind::ModuleError,
                format!("Function ID {} already exists", func_id)
            ));
        }
        
        // For specialized functions, we allow name conflicts by overriding
        // This is needed for monomorphization where we replace generic functions
        if let Some(old_id) = self.function_names.get(&name) {
            // Remove the old function if it exists
            self.functions.remove(old_id);
        }
        
        self.functions.insert(func_id, function);
        self.function_names.insert(name, func_id);
        
        // Update next function ID if needed
        if func_id.0 >= self.next_function_id {
            self.next_function_id = func_id.0 + 1;
        }
        
        Ok(func_id)
    }

    /// Remove a function from the module
    pub fn remove_function(&mut self, func_id: FunctionId) -> Result<Function, Error> {
        let function = self.functions.remove(&func_id)
            .ok_or_else(|| Error::new(
                ErrorKind::ModuleError,
                format!("Function ID {} not found", func_id)
            ))?;
        
        // Remove from name mapping
        self.function_names.remove(&function.name);
        
        Ok(function)
    }

    /// Remove a function by name
    pub fn remove_function_by_name(&mut self, name: &str) -> Result<Function, Error> {
        let func_id = self.function_names.remove(name)
            .ok_or_else(|| Error::new(
                ErrorKind::ModuleError,
                format!("Function '{}' not found", name)
            ))?;
        
        let function = self.functions.remove(&func_id)
            .ok_or_else(|| Error::new(
                ErrorKind::ModuleError,
                format!("Function ID {} not found", func_id)
            ))?;
        
        Ok(function)
    }

    /// Get function name from ID
    pub fn get_function_name(&self, func_id: FunctionId) -> Option<&str> {
        self.functions.get(&func_id).map(|f| f.name.as_str())
    }

    /// Set function name (updates both the function and name mapping)
    pub fn set_function_name(&mut self, func_id: FunctionId, new_name: String) -> Result<(), Error> {
        let function = self.functions.get_mut(&func_id)
            .ok_or_else(|| Error::new(
                ErrorKind::ModuleError,
                format!("Function ID {} not found", func_id)
            ))?;
        
        let old_name = function.name.clone();
        
        // Check if new name already exists
        if self.function_names.contains_key(&new_name) {
            return Err(Error::new(
                ErrorKind::ModuleError,
                format!("Function name '{}' already exists", new_name)
            ));
        }
        
        // Update function name
        function.name = new_name.clone();
        
        // Update name mapping
        self.function_names.remove(&old_name);
        self.function_names.insert(new_name, func_id);
        
        Ok(())
    }

    /// Clone a function for specialization
    pub fn clone_function(&self, func_id: FunctionId) -> Result<Function, Error> {
        let function = self.functions.get(&func_id)
            .ok_or_else(|| Error::new(
                ErrorKind::ModuleError,
                format!("Function ID {} not found", func_id)
            ))?;
        
        Ok(function.clone())
    }

    /// Clone a function by name for specialization
    pub fn clone_function_by_name(&self, name: &str) -> Result<Function, Error> {
        let func_id = self.function_names.get(name)
            .ok_or_else(|| Error::new(
                ErrorKind::ModuleError,
                format!("Function '{}' not found", name)
            ))?;
        
        self.clone_function(*func_id)
    }

    /// Get all function names
    pub fn function_names(&self) -> Vec<String> {
        self.function_names.keys().cloned().collect()
    }

    /// Get function ID to name mapping
    pub fn function_id_to_name_map(&self) -> HashMap<FunctionId, String> {
        self.functions.iter()
            .map(|(id, func)| (*id, func.name.clone()))
            .collect()
    }

    /// Get next available function ID
    pub fn next_function_id(&self) -> FunctionId {
        FunctionId(self.next_function_id)
    }

    /// Reserve a function ID for future use
    pub fn reserve_function_id(&mut self) -> FunctionId {
        let id = FunctionId(self.next_function_id);
        self.next_function_id += 1;
        id
    }

    /// Check if a function exists by ID
    pub fn has_function(&self, func_id: FunctionId) -> bool {
        self.functions.contains_key(&func_id)
    }

    /// Check if a function exists by name
    pub fn has_function_by_name(&self, name: &str) -> bool {
        self.function_names.contains_key(name)
    }

    /// Get all function IDs
    pub fn function_ids(&self) -> Vec<FunctionId> {
        self.functions.keys().cloned().collect()
    }

    /// Replace a function with a new one (preserves ID)
    pub fn replace_function(&mut self, func_id: FunctionId, mut new_function: Function) -> Result<Function, Error> {
        let old_function = self.functions.get(&func_id)
            .ok_or_else(|| Error::new(
                ErrorKind::ModuleError,
                format!("Function ID {} not found", func_id)
            ))?
            .clone();
        
        // Update name mapping if name changed
        if old_function.name != new_function.name {
            self.function_names.remove(&old_function.name);
            
            // Check if new name already exists
            if self.function_names.contains_key(&new_function.name) {
                return Err(Error::new(
                    ErrorKind::ModuleError,
                    format!("Function name '{}' already exists", new_function.name)
                ));
            }
            
            self.function_names.insert(new_function.name.clone(), func_id);
        }
        
        // Ensure the new function has the correct ID
        new_function.id = func_id;
        
        self.functions.insert(func_id, new_function);
        
        Ok(old_function)
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
    use crate::ir::instruction::{Constant, Instruction};
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

        let params = vec![Parameter {
            name: "x".to_string(),
            ty: Type::I32,
        }];

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
        module.declare_external_function(
            "malloc".to_string(),
            Type::Function {
                params: vec![Type::I32],
                ret: Box::new(Type::Named("ptr".to_string())),
            },
        );

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

        let params = vec![Parameter {
            name: "n".to_string(),
            ty: Type::I32,
        }];

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

    #[test]
    fn test_add_function() {
        let mut module = Module::new();
        
        let func = Function::new(
            FunctionId(100),
            "test_func".to_string(),
            vec![Parameter {
                name: "x".to_string(),
                ty: Type::I32,
            }],
            Type::I32,
        );
        
        let func_id = module.add_function(func).unwrap();
        assert_eq!(func_id, FunctionId(100));
        
        assert!(module.has_function(func_id));
        assert!(module.has_function_by_name("test_func"));
        assert_eq!(module.get_function_name(func_id), Some("test_func"));
    }

    #[test]
    fn test_add_function_duplicate_id() {
        let mut module = Module::new();
        
        let func1 = Function::new(
            FunctionId(100),
            "func1".to_string(),
            vec![],
            Type::I32,
        );
        
        let func2 = Function::new(
            FunctionId(100),
            "func2".to_string(),
            vec![],
            Type::I32,
        );
        
        module.add_function(func1).unwrap();
        assert!(module.add_function(func2).is_err());
    }

    #[test]
    fn test_add_function_duplicate_name() {
        let mut module = Module::new();
        
        let func1 = Function::new(
            FunctionId(100),
            "duplicate_name".to_string(),
            vec![],
            Type::I32,
        );
        
        let func2 = Function::new(
            FunctionId(101),
            "duplicate_name".to_string(),
            vec![],
            Type::I32,
        );
        
        module.add_function(func1).unwrap();
        assert!(module.add_function(func2).is_err());
    }

    #[test]
    fn test_add_specialized_function() {
        let mut module = Module::new();
        
        let generic_func = Function::new(
            FunctionId(100),
            "generic_func".to_string(),
            vec![],
            Type::I32,
        );
        
        let specialized_func = Function::new(
            FunctionId(101),
            "generic_func".to_string(), // Same name as generic
            vec![],
            Type::I32,
        );
        
        module.add_function(generic_func).unwrap();
        
        // This should override the generic function
        let result = module.add_specialized_function(specialized_func).unwrap();
        assert_eq!(result, FunctionId(101));
        
        // Generic function should be replaced
        assert!(!module.has_function(FunctionId(100)));
        assert!(module.has_function(FunctionId(101)));
        assert_eq!(module.get_function_id("generic_func"), Some(FunctionId(101)));
    }

    #[test]
    fn test_remove_function() {
        let mut module = Module::new();
        
        let func = Function::new(
            FunctionId(100),
            "removable_func".to_string(),
            vec![],
            Type::I32,
        );
        
        let func_id = module.add_function(func).unwrap();
        assert!(module.has_function(func_id));
        
        let removed_func = module.remove_function(func_id).unwrap();
        assert_eq!(removed_func.name, "removable_func");
        assert!(!module.has_function(func_id));
        assert!(!module.has_function_by_name("removable_func"));
    }

    #[test]
    fn test_remove_function_by_name() {
        let mut module = Module::new();
        
        let func = Function::new(
            FunctionId(100),
            "removable_func".to_string(),
            vec![],
            Type::I32,
        );
        
        module.add_function(func).unwrap();
        
        let removed_func = module.remove_function_by_name("removable_func").unwrap();
        assert_eq!(removed_func.id, FunctionId(100));
        assert!(!module.has_function(FunctionId(100)));
        assert!(!module.has_function_by_name("removable_func"));
    }

    #[test]
    fn test_clone_function() {
        let mut module = Module::new();
        
        let func = Function::new(
            FunctionId(100),
            "cloneable_func".to_string(),
            vec![Parameter {
                name: "x".to_string(),
                ty: Type::I32,
            }],
            Type::I32,
        );
        
        let func_id = module.add_function(func).unwrap();
        let cloned_func = module.clone_function(func_id).unwrap();
        
        assert_eq!(cloned_func.name, "cloneable_func");
        assert_eq!(cloned_func.id, func_id);
        assert_eq!(cloned_func.params.len(), 1);
    }

    #[test]
    fn test_clone_function_by_name() {
        let mut module = Module::new();
        
        let func = Function::new(
            FunctionId(100),
            "cloneable_func".to_string(),
            vec![],
            Type::I32,
        );
        
        module.add_function(func).unwrap();
        let cloned_func = module.clone_function_by_name("cloneable_func").unwrap();
        
        assert_eq!(cloned_func.name, "cloneable_func");
        assert_eq!(cloned_func.id, FunctionId(100));
    }

    #[test]
    fn test_set_function_name() {
        let mut module = Module::new();
        
        let func = Function::new(
            FunctionId(100),
            "old_name".to_string(),
            vec![],
            Type::I32,
        );
        
        let func_id = module.add_function(func).unwrap();
        
        module.set_function_name(func_id, "new_name".to_string()).unwrap();
        
        assert_eq!(module.get_function_name(func_id), Some("new_name"));
        assert!(!module.has_function_by_name("old_name"));
        assert!(module.has_function_by_name("new_name"));
        assert_eq!(module.get_function_id("new_name"), Some(func_id));
    }

    #[test]
    fn test_replace_function() {
        let mut module = Module::new();
        
        let old_func = Function::new(
            FunctionId(100),
            "replaceable_func".to_string(),
            vec![],
            Type::I32,
        );
        
        let new_func = Function::new(
            FunctionId(999), // Different ID, should be overridden
            "replaceable_func".to_string(),
            vec![Parameter {
                name: "x".to_string(),
                ty: Type::String,
            }],
            Type::String,
        );
        
        let func_id = module.add_function(old_func).unwrap();
        let replaced_func = module.replace_function(func_id, new_func).unwrap();
        
        assert_eq!(replaced_func.return_type, Type::I32);
        assert_eq!(replaced_func.params.len(), 0);
        
        let current_func = module.get_function(func_id).unwrap();
        assert_eq!(current_func.id, func_id); // ID should be preserved
        assert_eq!(current_func.return_type, Type::String);
        assert_eq!(current_func.params.len(), 1);
    }

    #[test]
    fn test_function_id_management() {
        let mut module = Module::new();
        
        let next_id = module.next_function_id();
        assert_eq!(next_id, FunctionId(0));
        
        let reserved_id = module.reserve_function_id();
        assert_eq!(reserved_id, FunctionId(0));
        
        let next_id_after = module.next_function_id();
        assert_eq!(next_id_after, FunctionId(1));
        
        // Add function with high ID
        let func = Function::new(
            FunctionId(500),
            "high_id_func".to_string(),
            vec![],
            Type::I32,
        );
        
        module.add_function(func).unwrap();
        
        // Next ID should be updated
        let next_id_high = module.next_function_id();
        assert_eq!(next_id_high, FunctionId(501));
    }

    #[test]
    fn test_function_queries() {
        let mut module = Module::new();
        
        let func1 = Function::new(
            FunctionId(100),
            "func1".to_string(),
            vec![],
            Type::I32,
        );
        
        let func2 = Function::new(
            FunctionId(101),
            "func2".to_string(),
            vec![],
            Type::String,
        );
        
        module.add_function(func1).unwrap();
        module.add_function(func2).unwrap();
        
        let names = module.function_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"func1".to_string()));
        assert!(names.contains(&"func2".to_string()));
        
        let ids = module.function_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&FunctionId(100)));
        assert!(ids.contains(&FunctionId(101)));
        
        let id_to_name = module.function_id_to_name_map();
        assert_eq!(id_to_name.get(&FunctionId(100)), Some(&"func1".to_string()));
        assert_eq!(id_to_name.get(&FunctionId(101)), Some(&"func2".to_string()));
    }
}
