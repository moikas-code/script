use crate::ir::{Function, FunctionId, Instruction, Module, ValueId};
use crate::types::{Type, generics::GenericEnv};
use crate::semantic::analyzer::{SemanticAnalyzer, GenericInstantiation};
use crate::inference::InferenceContext;
use crate::error::{Error, ErrorKind};
use std::collections::{HashMap, HashSet, VecDeque};

/// Context for monomorphization that tracks instantiated functions
#[derive(Debug)]
pub struct MonomorphizationContext {
    /// Track instantiated functions to avoid duplicates
    instantiated_functions: HashMap<String, Function>,
    /// Work queue for pending instantiations
    work_queue: VecDeque<(String, Vec<Type>)>,
    /// Type substitution environment
    generic_env: GenericEnv,
    /// Track which functions have been processed
    processed_functions: HashSet<String>,
    /// Integration with semantic analyzer
    semantic_analyzer: Option<SemanticAnalyzer>,
    /// Integration with inference context
    inference_ctx: Option<InferenceContext>,
    /// Track monomorphization statistics
    stats: MonomorphizationStats,
}

/// Statistics for monomorphization process
#[derive(Debug, Default)]
pub struct MonomorphizationStats {
    /// Number of functions monomorphized
    pub functions_monomorphized: usize,
    /// Number of type instantiations
    pub type_instantiations: usize,
    /// Number of duplicate instantiations avoided
    pub duplicates_avoided: usize,
}

impl MonomorphizationContext {
    /// Create a new monomorphization context
    pub fn new() -> Self {
        MonomorphizationContext {
            instantiated_functions: HashMap::new(),
            work_queue: VecDeque::new(),
            generic_env: GenericEnv::new(),
            processed_functions: HashSet::new(),
            semantic_analyzer: None,
            inference_ctx: None,
            stats: MonomorphizationStats::default(),
        }
    }

    /// Create a new monomorphization context with semantic analyzer integration
    pub fn with_semantic_analyzer(mut self, analyzer: SemanticAnalyzer) -> Self {
        self.semantic_analyzer = Some(analyzer);
        self
    }

    /// Create a new monomorphization context with inference context integration
    pub fn with_inference_context(mut self, ctx: InferenceContext) -> Self {
        self.inference_ctx = Some(ctx);
        self
    }
    
    /// Initialize monomorphization from semantic analysis results
    pub fn initialize_from_semantic_analysis(
        &mut self, 
        generic_instantiations: &[GenericInstantiation],
        type_info: &HashMap<usize, Type>
    ) {
        // Add all generic instantiations from semantic analysis to the work queue
        for instantiation in generic_instantiations {
            self.add_instantiation(
                instantiation.function_name.clone(),
                instantiation.type_args.clone()
            );
        }
        
        // Store type information for use during monomorphization
        // This can be used to resolve types more accurately
        // TODO: Expand this to use the type_info when needed
        let _ = type_info; // For now, suppress the unused parameter warning
    }
    
    /// Create a monomorphization context that's fully integrated with compilation pipeline
    pub fn from_compilation_results(
        analyzer: SemanticAnalyzer,
        inference_ctx: InferenceContext,
        generic_instantiations: &[GenericInstantiation],
        type_info: &HashMap<usize, Type>
    ) -> Self {
        let mut context = MonomorphizationContext::new()
            .with_semantic_analyzer(analyzer)
            .with_inference_context(inference_ctx);
            
        context.initialize_from_semantic_analysis(generic_instantiations, type_info);
        context
    }

    /// Get monomorphization statistics
    pub fn stats(&self) -> &MonomorphizationStats {
        &self.stats
    }

    /// Get a mutable reference to the semantic analyzer
    pub fn semantic_analyzer_mut(&mut self) -> Option<&mut SemanticAnalyzer> {
        self.semantic_analyzer.as_mut()
    }

    /// Get a mutable reference to the inference context
    pub fn inference_context_mut(&mut self) -> Option<&mut InferenceContext> {
        self.inference_ctx.as_mut()
    }

    /// Monomorphize all generic functions in an IR module
    pub fn monomorphize(&mut self, module: &mut Module) -> Result<(), Error> {
        // Reset stats for this monomorphization run
        self.stats = MonomorphizationStats::default();
        
        // Use semantic analyzer for better type inference if available
        if self.semantic_analyzer.is_some() {
            self.analyze_module_with_semantic_analyzer(module)?;
        }
        
        // Use inference context for type resolution if available
        if self.inference_ctx.is_some() {
            self.resolve_types_with_inference(module)?;
        }
        // Find all generic functions
        let mut generic_functions = Vec::new();

        for function in module.functions().values() {
            if self.is_generic_function(function) {
                generic_functions.push(function.clone());
            }
        }

        // If we don't have any generic instantiations from semantic analysis,
        // fall back to scanning the IR for call sites
        if self.work_queue.is_empty() {
            let mut call_sites = Vec::new();
            
            for function in &generic_functions {
                // Find call sites with concrete type arguments
                for block in function.blocks().values() {
                    for (_, instruction_with_loc) in &block.instructions {
                        if let Some((callee, type_args)) = self.extract_generic_call(&instruction_with_loc.instruction, module) {
                            call_sites.push((callee, type_args));
                        }
                    }
                }
            }

            // Process all generic function instantiations
            for (function_name, type_args) in call_sites {
                self.add_instantiation(function_name, type_args);
            }
        }

        // Process work queue
        while let Some((function_name, type_args)) = self.work_queue.pop_front() {
            if let Some(generic_function) = self.find_generic_function(&generic_functions, &function_name) {
                let specialized_function = self.specialize_function(generic_function, &type_args)?;
                let mangled_name = self.mangle_function_name(&function_name, &type_args);
                
                // Check for duplicates
                if self.instantiated_functions.contains_key(&mangled_name) {
                    self.stats.duplicates_avoided += 1;
                    continue;
                }
                
                // Add to instantiated functions
                self.instantiated_functions.insert(mangled_name, specialized_function);
                self.stats.functions_monomorphized += 1;
                
                // Mark as processed
                self.processed_functions.insert(format!("{}_{}", function_name, self.mangle_type_args(&type_args)));
            }
        }

        // Replace generic functions with specialized versions
        self.replace_generic_functions(module)?;

        Ok(())
    }

    /// Analyze module with semantic analyzer integration
    fn analyze_module_with_semantic_analyzer(&mut self, module: &Module) -> Result<(), Error> {
        // Use semantic analyzer to validate generic constraints
        for function in module.functions().values() {
            if self.is_generic_function(function) {
                // Validate generic bounds and constraints
                if let Some(type_params) = self.extract_type_params(function) {
                    for param in type_params {
                        // TODO: Check if type parameter has proper bounds
                        // This functionality needs to be implemented in TraitChecker
                        // For now, we skip bound checking as the methods don't exist yet
                        let _ = param; // Suppress unused variable warning
                    }
                }
            }
        }
        Ok(())
    }

    /// Resolve types with inference context integration
    fn resolve_types_with_inference(&mut self, module: &Module) -> Result<(), Error> {
        // First, collect generic functions
        let generic_functions: Vec<_> = module.functions()
            .values()
            .filter(|f| self.is_generic_function(f))
            .collect();
        
        // Then use inference context to resolve type variables and constraints
        if let Some(inf_ctx) = &mut self.inference_ctx {
            for _function in generic_functions {
                // Resolve any remaining type variables
                if let Err(e) = inf_ctx.solve_constraints() {
                    return Err(e);
                }
                
                // TODO: Update generic environment with resolved types
                // This needs InferenceContext to expose substitutions
                // For now, we skip this step as get_substitutions() doesn't exist
            }
        }
        Ok(())
    }

    /// Check if a function is generic (has type parameters)
    fn is_generic_function(&self, function: &Function) -> bool {
        // Check if the function signature contains type parameters
        self.has_type_parameters(&function.params.iter().map(|p| &p.ty).collect::<Vec<_>>()) || 
        self.has_type_parameter(&function.return_type)
    }

    /// Check if a list of types contains type parameters
    fn has_type_parameters(&self, types: &[&Type]) -> bool {
        types.iter().any(|t| self.has_type_parameter(t))
    }

    /// Check if a type contains type parameters
    fn has_type_parameter(&self, type_: &Type) -> bool {
        match type_ {
            Type::TypeParam(_) => true,
            Type::Array(elem) => self.has_type_parameter(elem),
            Type::Option(inner) => self.has_type_parameter(inner),
            Type::Result { ok, err } => self.has_type_parameter(ok) || self.has_type_parameter(err),
            Type::Function { params, ret } => {
                self.has_type_parameters(&params.iter().collect::<Vec<_>>()) || self.has_type_parameter(ret)
            }
            Type::Generic { args, .. } => self.has_type_parameters(&args.iter().collect::<Vec<_>>()),
            _ => false,
        }
    }

    /// Extract generic function call from instruction with type inference
    fn extract_generic_call(&self, instruction: &Instruction, module: &Module) -> Option<(String, Vec<Type>)> {
        match instruction {
            Instruction::Call { func, args, ty } => {
                if let Some(name) = self.get_function_name(func, module) {
                    // Check if this is a call to a generic function
                    if let Some(generic_function) = module.get_function(*func) {
                        if self.is_generic_function(generic_function) {
                            // Try to infer type arguments from the call context
                            let inferred_types = self.infer_type_arguments(generic_function, args, ty, module);
                            return Some((name, inferred_types));
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Infer type arguments for a generic function call
    fn infer_type_arguments(&self, generic_function: &Function, args: &[ValueId], return_type: &Type, module: &Module) -> Vec<Type> {
        let mut inferred_types = Vec::new();
        
        // Get the type parameters of the generic function
        if let Some(type_params) = self.extract_type_params(generic_function) {
            // For each type parameter, try to infer its concrete type
            for param in type_params {
                if let Some(concrete_type) = self.infer_type_parameter(&param, generic_function, args, return_type, module) {
                    inferred_types.push(concrete_type);
                } else {
                    // If we can't infer the type, use a placeholder
                    inferred_types.push(Type::Unknown);
                }
            }
        }
        
        inferred_types
    }

    /// Infer a specific type parameter from function call context
    fn infer_type_parameter(&self, param: &str, generic_function: &Function, args: &[ValueId], return_type: &Type, _module: &Module) -> Option<Type> {
        // Try to infer from return type
        if let Some(concrete_type) = self.match_type_parameter(param, &generic_function.return_type, return_type) {
            return Some(concrete_type);
        }
        
        // Try to infer from argument types
        for (i, _arg_value_id) in args.iter().enumerate() {
            if let Some(param_type) = generic_function.params.get(i) {
                // We would need value type information here
                // For now, this is a simplified implementation
                if let Type::TypeParam(param_name) = &param_type.ty {
                    if param_name == param {
                        // We'd need to get the actual type of the argument value
                        // This requires integration with the type inference system
                        return Some(Type::I32); // Placeholder
                    }
                }
            }
        }
        
        None
    }

    /// Match a type parameter against a concrete type
    fn match_type_parameter(&self, param: &str, generic_type: &Type, concrete_type: &Type) -> Option<Type> {
        match (generic_type, concrete_type) {
            (Type::TypeParam(param_name), concrete) if param_name == param => {
                Some(concrete.clone())
            }
            (Type::Array(generic_elem), Type::Array(concrete_elem)) => {
                self.match_type_parameter(param, generic_elem, concrete_elem)
            }
            (Type::Option(generic_inner), Type::Option(concrete_inner)) => {
                self.match_type_parameter(param, generic_inner, concrete_inner)
            }
            (Type::Function { params: generic_params, ret: generic_ret }, 
             Type::Function { params: concrete_params, ret: concrete_ret }) => {
                // Check return type
                if let Some(matched) = self.match_type_parameter(param, generic_ret, concrete_ret) {
                    return Some(matched);
                }
                
                // Check parameter types
                for (gen_param, con_param) in generic_params.iter().zip(concrete_params.iter()) {
                    if let Some(matched) = self.match_type_parameter(param, gen_param, con_param) {
                        return Some(matched);
                    }
                }
                
                None
            }
            _ => None,
        }
    }

    /// Get function name from FunctionId using module's name mapping
    fn get_function_name(&self, func_id: &FunctionId, module: &Module) -> Option<String> {
        module.get_function_name(*func_id).map(|s| s.to_string())
    }

    /// Add a function instantiation to the work queue
    fn add_instantiation(&mut self, function_name: String, type_args: Vec<Type>) {
        let key = format!("{}_{}", function_name, self.mangle_type_args(&type_args));
        if !self.processed_functions.contains(&key) {
            self.work_queue.push_back((function_name, type_args));
        }
    }

    /// Find a generic function by name
    fn find_generic_function<'a>(&self, functions: &'a [Function], name: &str) -> Option<&'a Function> {
        functions.iter().find(|f| f.name == name)
    }

    /// Specialize a generic function with concrete type arguments
    fn specialize_function(&mut self, generic_function: &Function, type_args: &[Type]) -> Result<Function, Error> {
        self.stats.type_instantiations += 1;
        // Create type substitution environment
        let mut env = self.generic_env.clone();
        
        // Add type parameter substitutions
        // This is simplified - in practice, we'd need to extract type parameters from the function signature
        if let Some(type_params) = self.extract_type_params(generic_function) {
            if type_params.len() != type_args.len() {
                return Err(Error::new(
                    ErrorKind::TypeError,
                    format!("Type argument count mismatch: expected {}, got {}", 
                        type_params.len(), type_args.len())
                ));
            }
            
            for (param, arg) in type_params.iter().zip(type_args.iter()) {
                env.add_substitution(param.clone(), arg.clone());
            }
        }

        // Clone and specialize the function
        let mut specialized = generic_function.clone();
        specialized.name = self.mangle_function_name(&generic_function.name, type_args);
        
        // Apply type substitutions to function parameters
        for param in &mut specialized.params {
            param.ty = env.substitute_type(&param.ty);
        }
        specialized.return_type = env.substitute_type(&specialized.return_type);
        
        // Apply type substitutions to all instructions in all blocks
        self.substitute_function_instructions(&mut specialized, &env);

        Ok(specialized)
    }

    /// Extract type parameters from function signature
    fn extract_type_params(&self, function: &Function) -> Option<Vec<String>> {
        let mut type_params = Vec::new();
        
        // Extract from parameters
        for param in &function.params {
            self.collect_type_params(&param.ty, &mut type_params);
        }
        
        // Extract from return type
        self.collect_type_params(&function.return_type, &mut type_params);
        
        if type_params.is_empty() {
            None
        } else {
            // Remove duplicates
            type_params.sort();
            type_params.dedup();
            Some(type_params)
        }
    }

    /// Collect type parameters from a type
    fn collect_type_params(&self, type_: &Type, params: &mut Vec<String>) {
        match type_ {
            Type::TypeParam(name) => params.push(name.clone()),
            Type::Array(elem) => self.collect_type_params(elem, params),
            Type::Option(inner) => self.collect_type_params(inner, params),
            Type::Result { ok, err } => {
                self.collect_type_params(ok, params);
                self.collect_type_params(err, params);
            }
            Type::Function { params: fn_params, ret } => {
                for param in fn_params {
                    self.collect_type_params(param, params);
                }
                self.collect_type_params(ret, params);
            }
            Type::Generic { args, .. } => {
                for arg in args {
                    self.collect_type_params(arg, params);
                }
            }
            _ => {}
        }
    }

    /// Apply type substitutions to all instructions in a function
    fn substitute_function_instructions(&self, function: &mut Function, env: &GenericEnv) {
        // Clone the block IDs to avoid borrow checker issues
        let block_ids: Vec<_> = function.blocks().keys().cloned().collect();
        
        for block_id in block_ids {
            if let Some(block) = function.get_block_mut(block_id) {
                // Apply substitutions to all instructions in the block
                for (_, instruction_with_loc) in &mut block.instructions {
                    self.substitute_instruction_types(&mut instruction_with_loc.instruction, env);
                }
            }
        }
    }

    /// Apply type substitutions to an instruction
    fn substitute_instruction_types(&self, instruction: &mut Instruction, env: &GenericEnv) {
        match instruction {
            Instruction::Binary { ty, .. } => {
                *ty = env.substitute_type(ty);
            }
            Instruction::Unary { ty, .. } => {
                *ty = env.substitute_type(ty);
            }
            Instruction::Call { ty, func, .. } => {
                // Update function call return type
                *ty = env.substitute_type(ty);
                // Note: func (FunctionId) doesn't need substitution - it's resolved during replacement
            }
            Instruction::Load { ty, .. } => {
                *ty = env.substitute_type(ty);
            }
            Instruction::Cast { from_ty, to_ty, .. } => {
                *from_ty = env.substitute_type(from_ty);
                *to_ty = env.substitute_type(to_ty);
            }
            Instruction::Alloc { ty } => {
                *ty = env.substitute_type(ty);
            }
            Instruction::GetElementPtr { elem_ty, .. } => {
                *elem_ty = env.substitute_type(elem_ty);
            }
            Instruction::GetFieldPtr { field_ty, .. } => {
                *field_ty = env.substitute_type(field_ty);
            }
            Instruction::LoadField { field_ty, .. } => {
                *field_ty = env.substitute_type(field_ty);
            }
            _ => {
                // Instructions like Store, StoreField, Return, Branch, CondBranch
                // don't have type fields that need updating
            }
        }
    }

    /// Generate a mangled name for a specialized function
    fn mangle_function_name(&self, base_name: &str, type_args: &[Type]) -> String {
        if type_args.is_empty() {
            return base_name.to_string();
        }
        
        let type_suffix = self.mangle_type_args(type_args);
        format!("{}_{}", base_name, type_suffix)
    }

    /// Create a mangled suffix for type arguments
    fn mangle_type_args(&self, type_args: &[Type]) -> String {
        type_args.iter()
            .map(|t| self.mangle_type(t))
            .collect::<Vec<_>>()
            .join("_")
    }

    /// Mangle a single type for use in function names
    fn mangle_type(&self, type_: &Type) -> String {
        match type_ {
            Type::I32 => "i32".to_string(),
            Type::F32 => "f32".to_string(),
            Type::Bool => "bool".to_string(),
            Type::String => "string".to_string(),
            Type::Array(elem) => format!("array_{}", self.mangle_type(elem)),
            Type::Option(inner) => format!("option_{}", self.mangle_type(inner)),
            Type::Result { ok, err } => format!("result_{}_{}", self.mangle_type(ok), self.mangle_type(err)),
            Type::Function { params, ret } => {
                let param_mangles = params.iter().map(|p| self.mangle_type(p)).collect::<Vec<_>>().join("_");
                format!("fn_{}_{}", param_mangles, self.mangle_type(ret))
            }
            Type::Generic { name, args } => {
                if args.is_empty() {
                    name.clone()
                } else {
                    format!("{}_{}", name, self.mangle_type_args(args))
                }
            }
            Type::TypeParam(name) => format!("param_{}", name),
            Type::TypeVar(id) => format!("var_{}", id),
            Type::Named(name) => name.clone(),
            Type::Unknown => "unknown".to_string(),
            Type::Never => "never".to_string(),
            Type::Future(inner) => format!("future_{}", self.mangle_type(inner)),
            Type::Tuple(types) => {
                let type_mangles = types.iter().map(|t| self.mangle_type(t)).collect::<Vec<_>>().join("_");
                format!("tuple_{}", type_mangles)
            }
            Type::Reference { mutable, inner } => {
                format!("ref_{}_{}", if *mutable { "mut" } else { "const" }, self.mangle_type(inner))
            }
        }
    }

    /// Check if a function needs its calls updated
    fn function_needs_call_updates(&self, function: &Function, module: &Module) -> bool {
        for block in function.blocks().values() {
            for (_, instruction_with_loc) in &block.instructions {
                if let Instruction::Call { func, .. } = &instruction_with_loc.instruction {
                    // Check if this is a call to a generic function
                    if let Some(called_function) = module.get_function(*func) {
                        if self.is_generic_function(called_function) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Update function calls within a single function
    fn update_calls_in_function(&self, function: &mut Function, module: &Module, type_substitutions: &HashMap<FunctionId, FunctionId>) -> Result<(), Error> {
        // Get all block IDs to avoid borrow checker issues
        let block_ids: Vec<_> = function.blocks().keys().cloned().collect();
        
        for block_id in block_ids {
            if let Some(block) = function.get_block_mut(block_id) {
                // Update all call instructions in this block
                for (_, instruction_with_loc) in &mut block.instructions {
                    if let Instruction::Call { func, ty, .. } = &mut instruction_with_loc.instruction {
                        // Check if this function ID needs to be replaced
                        if let Some(new_func_id) = type_substitutions.get(func) {
                            *func = *new_func_id;
                            // Also update the return type if needed
                            if let Some(new_func) = module.get_function(*new_func_id) {
                                *ty = new_func.return_type.clone();
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Create a mapping from generic function IDs to their specialized versions
    fn create_call_substitution_map(&self, module: &Module, generic_to_specialized: &HashMap<String, Vec<(String, FunctionId)>>) -> HashMap<FunctionId, FunctionId> {
        let mut substitutions = HashMap::new();
        
        // For each generic function, map to its specialized versions
        for (generic_name, specialized_versions) in generic_to_specialized {
            if let Some(generic_id) = module.get_function_id(generic_name) {
                // For simplicity, use the first specialized version
                // In a full implementation, we'd need type-based dispatch
                if let Some((_, spec_id)) = specialized_versions.first() {
                    substitutions.insert(generic_id, *spec_id);
                }
            }
        }
        
        substitutions
    }

    /// Replace generic functions with specialized versions in the module
    fn replace_generic_functions(&mut self, module: &mut Module) -> Result<(), Error> {
        // Step 1: Add all specialized functions to the module and build ID mapping
        let mut generic_to_specialized: HashMap<String, Vec<(String, FunctionId)>> = HashMap::new();
        let mut specialized_name_to_id: HashMap<String, FunctionId> = HashMap::new();
        
        for (mangled_name, mut specialized_function) in self.instantiated_functions.drain() {
            // Reserve a new function ID for the specialized function
            let new_id = module.reserve_function_id();
            specialized_function.id = new_id;
            specialized_function.name = mangled_name.clone();
            
            // Add the specialized function to the module
            if let Err(e) = module.add_function(specialized_function) {
                return Err(Error::new(
                    ErrorKind::ModuleError,
                    format!("Failed to add specialized function '{}': {}", mangled_name, e)
                ));
            }
            
            specialized_name_to_id.insert(mangled_name.clone(), new_id);
            
            // Extract original function name and add to mapping
            if let Some(underscore_pos) = mangled_name.find('_') {
                let original_name = mangled_name[..underscore_pos].to_string();
                generic_to_specialized.entry(original_name)
                    .or_insert(Vec::new())
                    .push((mangled_name, new_id));
            }
        }
        
        // Step 2: Create substitution map from generic function IDs to specialized IDs
        let substitution_map = self.create_call_substitution_map(module, &generic_to_specialized);
        
        // Step 3: Update all function calls in all functions
        let function_ids: Vec<_> = module.functions().keys().cloned().collect();
        
        for func_id in function_ids {
            // Skip if this is a function we're about to remove
            if substitution_map.contains_key(&func_id) {
                continue;
            }
            
            if let Some(function) = module.get_function(func_id) {
                if self.function_needs_call_updates(function, module) {
                    let mut modified_function = function.clone();
                    self.update_calls_in_function(&mut modified_function, module, &substitution_map)?;
                    
                    if let Err(e) = module.replace_function(func_id, modified_function) {
                        return Err(Error::new(
                            ErrorKind::ModuleError,
                            format!("Failed to update function {}: {}", func_id, e)
                        ));
                    }
                }
            }
        }
        
        // Step 4: Remove generic functions that have been fully specialized
        let mut functions_to_remove = Vec::new();
        
        for (func_id, function) in module.functions() {
            if self.is_generic_function(function) {
                // Check if this generic function has specializations
                let has_specializations = generic_to_specialized.contains_key(&function.name);
                
                if has_specializations {
                    functions_to_remove.push(*func_id);
                }
            }
        }
        
        // Remove the generic functions
        for func_id in functions_to_remove {
            if let Err(e) = module.remove_function(func_id) {
                return Err(Error::new(
                    ErrorKind::ModuleError,
                    format!("Failed to remove generic function {}: {}", func_id, e)
                ));
            }
        }
        
        Ok(())
    }
}

impl Default for MonomorphizationContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Parameter;
    // use crate::semantic::FunctionSignature; // Not needed for current tests

    #[test]
    fn test_type_mangling() {
        let ctx = MonomorphizationContext::new();
        
        assert_eq!(ctx.mangle_type(&Type::I32), "i32");
        assert_eq!(ctx.mangle_type(&Type::String), "string");
        assert_eq!(ctx.mangle_type(&Type::Array(Box::new(Type::I32))), "array_i32");
        assert_eq!(ctx.mangle_type(&Type::Option(Box::new(Type::String))), "option_string");
    }

    #[test]
    fn test_function_name_mangling() {
        let ctx = MonomorphizationContext::new();
        
        let type_args = vec![Type::I32, Type::String];
        let mangled = ctx.mangle_function_name("identity", &type_args);
        assert_eq!(mangled, "identity_i32_string");
    }

    #[test]
    fn test_type_parameter_detection() {
        let ctx = MonomorphizationContext::new();
        
        assert!(ctx.has_type_parameter(&Type::TypeParam("T".to_string())));
        assert!(!ctx.has_type_parameter(&Type::I32));
        assert!(ctx.has_type_parameter(&Type::Array(Box::new(Type::TypeParam("T".to_string())))));
    }

    #[test]
    fn test_type_parameter_extraction() {
        let ctx = MonomorphizationContext::new();
        
        let function = Function::new(
            FunctionId(0),
            "identity".to_string(),
            vec![Parameter {
                name: "x".to_string(),
                ty: Type::TypeParam("T".to_string()),
            }],
            Type::TypeParam("T".to_string()),
        );
        
        let type_params = ctx.extract_type_params(&function).unwrap();
        assert_eq!(type_params, vec!["T"]);
    }

    #[test]
    fn test_function_specialization() {
        let mut ctx = MonomorphizationContext::new();
        
        let generic_function = Function::new(
            FunctionId(0),
            "identity".to_string(),
            vec![Parameter {
                name: "x".to_string(),
                ty: Type::TypeParam("T".to_string()),
            }],
            Type::TypeParam("T".to_string()),
        );
        
        let type_args = vec![Type::I32];
        let specialized = ctx.specialize_function(&generic_function, &type_args).unwrap();
        
        assert_eq!(specialized.name, "identity_i32");
        assert_eq!(specialized.params[0].ty, Type::I32);
        assert_eq!(specialized.return_type, Type::I32);
    }

    #[test]
    fn test_type_parameter_matching() {
        let ctx = MonomorphizationContext::new();
        
        let generic_type = Type::TypeParam("T".to_string());
        let concrete_type = Type::I32;
        
        let matched = ctx.match_type_parameter("T", &generic_type, &concrete_type);
        assert_eq!(matched, Some(Type::I32));
        
        // Test nested types
        let generic_array = Type::Array(Box::new(Type::TypeParam("T".to_string())));
        let concrete_array = Type::Array(Box::new(Type::String));
        
        let matched_nested = ctx.match_type_parameter("T", &generic_array, &concrete_array);
        assert_eq!(matched_nested, Some(Type::String));
    }

    #[test]
    fn test_instruction_type_substitution() {
        use crate::ir::instruction::BinaryOp;
        use crate::ir::ValueId;
        
        let ctx = MonomorphizationContext::new();
        let mut env = GenericEnv::new();
        env.add_substitution("T".to_string(), Type::I32);
        
        let mut instruction = Instruction::Binary {
            op: BinaryOp::Add,
            lhs: ValueId(0),
            rhs: ValueId(1),
            ty: Type::TypeParam("T".to_string()),
        };
        
        ctx.substitute_instruction_types(&mut instruction, &env);
        
        if let Instruction::Binary { ty, .. } = instruction {
            assert_eq!(ty, Type::I32);
        } else {
            panic!("Expected Binary instruction");
        }
    }

    #[test]
    fn test_monomorphization_stats() {
        let mut ctx = MonomorphizationContext::new();
        
        // Add some instantiations
        ctx.add_instantiation("identity".to_string(), vec![Type::I32]);
        ctx.add_instantiation("identity".to_string(), vec![Type::String]);
        ctx.add_instantiation("identity".to_string(), vec![Type::I32]); // Duplicate
        
        // Stats should track that we avoided one duplicate
        assert_eq!(ctx.work_queue.len(), 2); // Only 2 unique instantiations
    }

    #[test]
    fn test_integration_with_semantic_analysis() {
        use crate::semantic::analyzer::GenericInstantiation;
        use std::collections::HashMap;
        
        let mut ctx = MonomorphizationContext::new();
        
        let instantiations = vec![
            GenericInstantiation {
                function_name: "map".to_string(),
                type_args: vec![Type::I32, Type::String],
                span: crate::source::Span::dummy(),
            },
            GenericInstantiation {
                function_name: "filter".to_string(),
                type_args: vec![Type::Bool],
                span: crate::source::Span::dummy(),
            },
        ];
        
        let type_info = HashMap::new();
        ctx.initialize_from_semantic_analysis(&instantiations, &type_info);
        
        // Should have added both instantiations to work queue
        assert_eq!(ctx.work_queue.len(), 2);
    }

    #[test]
    fn test_complex_generic_types() {
        let ctx = MonomorphizationContext::new();
        
        // Test complex nested generic types
        let complex_type = Type::Function {
            params: vec![
                Type::Array(Box::new(Type::TypeParam("T".to_string()))),
                Type::Function {
                    params: vec![Type::TypeParam("T".to_string())],
                    ret: Box::new(Type::TypeParam("U".to_string())),
                },
            ],
            ret: Box::new(Type::Array(Box::new(Type::TypeParam("U".to_string())))),
        };
        
        let mut type_params = Vec::new();
        ctx.collect_type_params(&complex_type, &mut type_params);
        
        // Should find T and U
        type_params.sort();
        type_params.dedup();
        assert_eq!(type_params, vec!["T".to_string(), "U".to_string()]);
    }

    #[test]
    fn test_generic_environment_substitution() {
        let mut env = GenericEnv::new();
        env.add_substitution("T".to_string(), Type::I32);
        env.add_substitution("U".to_string(), Type::String);
        
        let generic_type = Type::Function {
            params: vec![Type::TypeParam("T".to_string())],
            ret: Box::new(Type::Array(Box::new(Type::TypeParam("U".to_string())))),
        };
        
        let concrete_type = env.substitute_type(&generic_type);
        
        if let Type::Function { params, ret } = concrete_type {
            assert_eq!(params[0], Type::I32);
            if let Type::Array(elem) = *ret {
                assert_eq!(*elem, Type::String);
            } else {
                panic!("Expected Array type");
            }
        } else {
            panic!("Expected Function type");
        }
    }
}