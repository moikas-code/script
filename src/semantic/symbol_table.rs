use crate::source::Span;
use crate::types::Type;
use std::collections::HashMap;

use super::symbol::{Symbol, SymbolId, SymbolKind, FunctionSignature};

/// A unique identifier for a scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(pub usize);

/// Represents a scope in the symbol table
#[derive(Debug)]
struct Scope {
    /// Unique identifier for this scope
    _id: ScopeId,
    /// Parent scope (None for global scope)
    parent: Option<ScopeId>,
    /// Symbols defined in this scope (name -> symbol IDs)
    /// Multiple IDs per name for function overloading
    symbols: HashMap<String, Vec<SymbolId>>,
    /// Child scopes
    children: Vec<ScopeId>,
}

/// Symbol table for managing symbols and scopes
#[derive(Debug)]
pub struct SymbolTable {
    /// All symbols by their ID
    symbols: HashMap<SymbolId, Symbol>,
    /// All scopes by their ID
    scopes: HashMap<ScopeId, Scope>,
    /// Current scope
    current_scope: ScopeId,
    /// Next available symbol ID
    next_symbol_id: usize,
    /// Next available scope ID
    next_scope_id: usize,
}

impl SymbolTable {
    /// Create a new symbol table with a global scope
    pub fn new() -> Self {
        let mut scopes = HashMap::new();
        let global_scope = Scope {
            _id: ScopeId(0),
            parent: None,
            symbols: HashMap::new(),
            children: Vec::new(),
        };
        scopes.insert(ScopeId(0), global_scope);

        SymbolTable {
            symbols: HashMap::new(),
            scopes,
            current_scope: ScopeId(0),
            next_symbol_id: 1,
            next_scope_id: 1,
        }
    }

    /// Enter a new scope
    pub fn enter_scope(&mut self) -> ScopeId {
        let new_scope_id = ScopeId(self.next_scope_id);
        self.next_scope_id += 1;

        let new_scope = Scope {
            _id: new_scope_id,
            parent: Some(self.current_scope),
            symbols: HashMap::new(),
            children: Vec::new(),
        };

        // Add to parent's children
        if let Some(parent_scope) = self.scopes.get_mut(&self.current_scope) {
            parent_scope.children.push(new_scope_id);
        }

        self.scopes.insert(new_scope_id, new_scope);
        self.current_scope = new_scope_id;
        new_scope_id
    }

    /// Exit the current scope
    pub fn exit_scope(&mut self) -> Option<ScopeId> {
        if let Some(scope) = self.scopes.get(&self.current_scope) {
            if let Some(parent_id) = scope.parent {
                self.current_scope = parent_id;
                return Some(parent_id);
            }
        }
        None
    }

    /// Get the current scope ID
    pub fn current_scope(&self) -> ScopeId {
        self.current_scope
    }

    /// Define a variable in the current scope
    pub fn define_variable(
        &mut self,
        name: String,
        ty: Type,
        def_span: Span,
        is_mutable: bool,
    ) -> Result<SymbolId, String> {
        // Check if already defined in current scope (no shadowing in same scope)
        if let Some(scope) = self.scopes.get(&self.current_scope) {
            if let Some(existing_ids) = scope.symbols.get(&name) {
                if !existing_ids.is_empty() {
                    return Err(format!("Variable '{}' already defined in this scope", name));
                }
            }
        }

        let symbol_id = SymbolId(self.next_symbol_id);
        self.next_symbol_id += 1;

        let symbol = Symbol::variable(
            symbol_id,
            name.clone(),
            ty,
            def_span,
            is_mutable,
            self.current_scope,
        );

        self.symbols.insert(symbol_id, symbol);

        // Add to current scope
        if let Some(scope) = self.scopes.get_mut(&self.current_scope) {
            scope.symbols.entry(name).or_insert_with(Vec::new).push(symbol_id);
        }

        Ok(symbol_id)
    }

    /// Define a function in the current scope
    pub fn define_function(
        &mut self,
        name: String,
        signature: FunctionSignature,
        def_span: Span,
    ) -> Result<SymbolId, String> {
        // Check for conflicting overloads
        if let Some(scope) = self.scopes.get(&self.current_scope) {
            if let Some(existing_ids) = scope.symbols.get(&name) {
                for id in existing_ids {
                    if let Some(existing_symbol) = self.symbols.get(id) {
                        if let Some(existing_sig) = existing_symbol.function_signature() {
                            if !signature.is_compatible_for_overload(existing_sig) {
                                return Err(format!(
                                    "Function '{}' with same signature already defined in this scope",
                                    name
                                ));
                            }
                        } else {
                            // Non-function symbol with same name
                            return Err(format!(
                                "Cannot define function '{}': a {} with this name already exists",
                                name, existing_symbol.kind
                            ));
                        }
                    }
                }
            }
        }

        let symbol_id = SymbolId(self.next_symbol_id);
        self.next_symbol_id += 1;

        let symbol = Symbol::function(
            symbol_id,
            name.clone(),
            signature,
            def_span,
            self.current_scope,
        );

        self.symbols.insert(symbol_id, symbol);

        // Add to current scope
        if let Some(scope) = self.scopes.get_mut(&self.current_scope) {
            scope.symbols.entry(name).or_insert_with(Vec::new).push(symbol_id);
        }

        Ok(symbol_id)
    }

    /// Define a parameter in the current scope
    pub fn define_parameter(
        &mut self,
        name: String,
        ty: Type,
        def_span: Span,
    ) -> Result<SymbolId, String> {
        // Parameters can't shadow in the same scope
        if let Some(scope) = self.scopes.get(&self.current_scope) {
            if let Some(existing_ids) = scope.symbols.get(&name) {
                if !existing_ids.is_empty() {
                    return Err(format!("Parameter '{}' already defined in this scope", name));
                }
            }
        }

        let symbol_id = SymbolId(self.next_symbol_id);
        self.next_symbol_id += 1;

        let symbol = Symbol::parameter(
            symbol_id,
            name.clone(),
            ty,
            def_span,
            self.current_scope,
        );

        self.symbols.insert(symbol_id, symbol);

        // Add to current scope
        if let Some(scope) = self.scopes.get_mut(&self.current_scope) {
            scope.symbols.entry(name).or_insert_with(Vec::new).push(symbol_id);
        }

        Ok(symbol_id)
    }

    /// Look up a symbol by name, searching from current scope to global
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.lookup_all(name).into_iter().next()
    }

    /// Look up all symbols with a given name (for overloaded functions)
    pub fn lookup_all(&self, name: &str) -> Vec<&Symbol> {
        let mut result = Vec::new();
        let mut current = Some(self.current_scope);

        while let Some(scope_id) = current {
            if let Some(scope) = self.scopes.get(&scope_id) {
                if let Some(symbol_ids) = scope.symbols.get(name) {
                    for id in symbol_ids {
                        if let Some(symbol) = self.symbols.get(id) {
                            result.push(symbol);
                        }
                    }
                    // Found symbols in this scope, don't look in parent scopes
                    // This implements shadowing
                    break;
                }
                current = scope.parent;
            } else {
                break;
            }
        }

        result
    }

    /// Look up a function by name and argument types
    pub fn lookup_function(
        &self,
        name: &str,
        arg_types: &[Type],
    ) -> Option<&Symbol> {
        let candidates = self.lookup_all(name);
        
        for symbol in candidates {
            if let Some(sig) = symbol.function_signature() {
                // Check if parameter count matches
                if sig.params.len() != arg_types.len() {
                    continue;
                }
                
                // Check if all parameter types match
                let params_match = sig.params.iter()
                    .zip(arg_types.iter())
                    .all(|((_, param_ty), arg_ty)| arg_ty.is_assignable_to(param_ty));
                
                if params_match {
                    return Some(symbol);
                }
            }
        }
        
        None
    }

    /// Get a symbol by ID
    pub fn get_symbol(&self, id: SymbolId) -> Option<&Symbol> {
        self.symbols.get(&id)
    }

    /// Get a mutable reference to a symbol by ID
    pub fn get_symbol_mut(&mut self, id: SymbolId) -> Option<&mut Symbol> {
        self.symbols.get_mut(&id)
    }

    /// Mark a symbol as used
    pub fn mark_used(&mut self, id: SymbolId) {
        if let Some(symbol) = self.symbols.get_mut(&id) {
            symbol.mark_used();
        }
    }

    /// Get all unused symbols
    pub fn get_unused_symbols(&self) -> Vec<&Symbol> {
        self.symbols
            .values()
            .filter(|s| !s.is_used && !matches!(s.kind, SymbolKind::Function(_)))
            .collect()
    }

    /// Get all symbols in the current scope
    pub fn get_current_scope_symbols(&self) -> Vec<&Symbol> {
        if let Some(scope) = self.scopes.get(&self.current_scope) {
            scope.symbols
                .values()
                .flat_map(|ids| ids.iter())
                .filter_map(|id| self.symbols.get(id))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Check if we're in the global scope
    pub fn is_global_scope(&self) -> bool {
        self.current_scope == ScopeId(0)
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{SourceLocation, Span};

    fn make_span() -> Span {
        Span::new(
            SourceLocation::new(1, 1, 0),
            SourceLocation::new(1, 5, 5),
        )
    }

    #[test]
    fn test_symbol_table_creation() {
        let table = SymbolTable::new();
        assert_eq!(table.current_scope, ScopeId(0));
        assert!(table.is_global_scope());
    }

    #[test]
    fn test_variable_definition_and_lookup() {
        let mut table = SymbolTable::new();
        
        let _x_id = table.define_variable(
            "x".to_string(),
            Type::I32,
            make_span(),
            true,
        ).unwrap();

        let x_symbol = table.lookup("x").unwrap();
        assert_eq!(x_symbol.name, "x");
        assert_eq!(x_symbol.ty, Type::I32);
        assert!(x_symbol.is_mutable);

        // Undefined variable
        assert!(table.lookup("y").is_none());
    }

    #[test]
    fn test_scope_management() {
        let mut table = SymbolTable::new();
        
        // Define in global scope
        table.define_variable("x".to_string(), Type::I32, make_span(), true).unwrap();
        
        // Enter new scope
        let _inner_scope = table.enter_scope();
        assert!(!table.is_global_scope());
        
        // Can see outer variable
        assert!(table.lookup("x").is_some());
        
        // Shadow in inner scope
        table.define_variable("x".to_string(), Type::F32, make_span(), false).unwrap();
        let inner_x = table.lookup("x").unwrap();
        assert_eq!(inner_x.ty, Type::F32);
        assert!(!inner_x.is_mutable);
        
        // Define new variable in inner scope
        table.define_variable("y".to_string(), Type::Bool, make_span(), true).unwrap();
        
        // Exit scope
        table.exit_scope();
        assert!(table.is_global_scope());
        
        // Back to outer x
        let outer_x = table.lookup("x").unwrap();
        assert_eq!(outer_x.ty, Type::I32);
        
        // Inner variable is gone
        assert!(table.lookup("y").is_none());
    }

    #[test]
    fn test_function_overloading() {
        let mut table = SymbolTable::new();
        
        // Define function with one parameter
        let sig1 = FunctionSignature {
            params: vec![("x".to_string(), Type::I32)],
            return_type: Type::I32,
            is_const: false,
            is_async: false,
        };
        table.define_function("add".to_string(), sig1, make_span()).unwrap();
        
        // Define overload with two parameters
        let sig2 = FunctionSignature {
            params: vec![("x".to_string(), Type::I32), ("y".to_string(), Type::I32)],
            return_type: Type::I32,
            is_const: false,
            is_async: false,
        };
        table.define_function("add".to_string(), sig2, make_span()).unwrap();
        
        // Look up function with one argument
        let func1 = table.lookup_function("add", &[Type::I32]).unwrap();
        assert_eq!(func1.function_signature().unwrap().params.len(), 1);
        
        // Look up function with two arguments
        let func2 = table.lookup_function("add", &[Type::I32, Type::I32]).unwrap();
        assert_eq!(func2.function_signature().unwrap().params.len(), 2);
        
        // No matching overload
        assert!(table.lookup_function("add", &[Type::String]).is_none());
    }

    #[test]
    fn test_duplicate_definition_error() {
        let mut table = SymbolTable::new();
        
        // Define variable
        table.define_variable("x".to_string(), Type::I32, make_span(), true).unwrap();
        
        // Try to define again in same scope
        let result = table.define_variable("x".to_string(), Type::F32, make_span(), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already defined"));
    }

    #[test]
    fn test_conflicting_function_signatures() {
        let mut table = SymbolTable::new();
        
        // Define function
        let sig1 = FunctionSignature {
            params: vec![("x".to_string(), Type::I32)],
            return_type: Type::I32,
            is_const: false,
            is_async: false,
        };
        table.define_function("foo".to_string(), sig1.clone(), make_span()).unwrap();
        
        // Try to define with same signature (different return type doesn't matter)
        let sig2 = FunctionSignature {
            params: vec![("y".to_string(), Type::I32)], // Different param name doesn't matter
            return_type: Type::F32,
            is_const: true,
            is_async: false,
        };
        let result = table.define_function("foo".to_string(), sig2, make_span());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("same signature"));
    }

    #[test]
    fn test_unused_symbols() {
        let mut table = SymbolTable::new();
        
        let x_id = table.define_variable("x".to_string(), Type::I32, make_span(), true).unwrap();
        let _y_id = table.define_variable("y".to_string(), Type::I32, make_span(), true).unwrap();
        
        // Mark x as used
        table.mark_used(x_id);
        
        let unused = table.get_unused_symbols();
        assert_eq!(unused.len(), 1);
        assert_eq!(unused[0].name, "y");
    }
}