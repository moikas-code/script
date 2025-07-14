use crate::parser::{ExportKind, ExportSpecifier, ImportSpecifier};
use crate::source::Span;
use crate::types::Type;
use std::collections::HashMap;

use super::symbol::{EnumInfo, FunctionSignature, StructInfo, Symbol, SymbolId, SymbolKind};

/// A unique identifier for a scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(pub usize);

/// A unique identifier for a module
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleId(pub usize);

/// Information about an imported symbol
#[derive(Debug, Clone)]
pub struct ImportedSymbol {
    /// Original symbol ID in the source module
    pub source_symbol_id: SymbolId,
    /// Source module ID
    pub source_module: ModuleId,
    /// Original name in source module
    pub original_name: String,
    /// Local name (after aliasing)
    pub local_name: String,
    /// Import span for error reporting
    pub import_span: Span,
}

/// Information about an exported symbol
#[derive(Debug, Clone)]
pub struct ExportedSymbol {
    /// Local symbol ID being exported
    pub symbol_id: SymbolId,
    /// External name (after aliasing)
    pub external_name: String,
    /// Export span for error reporting
    pub export_span: Span,
}

/// Module-level information
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// Module identifier
    pub id: ModuleId,
    /// Module name/path
    pub name: String,
    /// Root scope for this module
    pub root_scope: ScopeId,
    /// Imported symbols by local name
    pub imports: HashMap<String, ImportedSymbol>,
    /// Exported symbols by external name
    pub exports: HashMap<String, ExportedSymbol>,
    /// Namespace imports (import * as name)
    pub namespace_imports: HashMap<String, ModuleId>,
}

/// Represents a scope in the symbol table
#[derive(Debug, Clone)]
struct Scope {
    /// Unique identifier for this scope
    _id: ScopeId,
    /// Parent scope (None for global scope)
    parent: Option<ScopeId>,
    /// Module this scope belongs to
    module_id: ModuleId,
    /// Symbols defined in this scope (name -> symbol IDs)
    /// Multiple IDs per name for function overloading
    symbols: HashMap<String, Vec<SymbolId>>,
    /// Child scopes
    children: Vec<ScopeId>,
}

/// Symbol table for managing symbols and scopes
#[derive(Debug, Clone)]
pub struct SymbolTable {
    /// All symbols by their ID
    symbols: HashMap<SymbolId, Symbol>,
    /// All scopes by their ID
    scopes: HashMap<ScopeId, Scope>,
    /// All modules by their ID
    modules: HashMap<ModuleId, ModuleInfo>,
    /// Current scope
    current_scope: ScopeId,
    /// Current module
    current_module: ModuleId,
    /// Next available symbol ID
    next_symbol_id: usize,
    /// Next available scope ID
    next_scope_id: usize,
    /// Next available module ID
    next_module_id: usize,
}

impl SymbolTable {
    /// Create a new symbol table with a global scope
    pub fn new() -> Self {
        let global_module_id = ModuleId(0);
        let global_scope_id = ScopeId(0);

        let mut scopes = HashMap::new();
        let global_scope = Scope {
            _id: global_scope_id,
            parent: None,
            module_id: global_module_id,
            symbols: HashMap::new(),
            children: Vec::new(),
        };
        scopes.insert(global_scope_id, global_scope);

        let mut modules = HashMap::new();
        let global_module = ModuleInfo {
            id: global_module_id,
            name: "<global>".to_string(),
            root_scope: global_scope_id,
            imports: HashMap::new(),
            exports: HashMap::new(),
            namespace_imports: HashMap::new(),
        };
        modules.insert(global_module_id, global_module);

        SymbolTable {
            symbols: HashMap::new(),
            scopes,
            modules,
            current_scope: global_scope_id,
            current_module: global_module_id,
            next_symbol_id: 1,
            next_scope_id: 1,
            next_module_id: 1,
        }
    }

    /// Enter a new scope
    pub fn enter_scope(&mut self) -> ScopeId {
        let new_scope_id = ScopeId(self.next_scope_id);
        self.next_scope_id += 1;

        let new_scope = Scope {
            _id: new_scope_id,
            parent: Some(self.current_scope),
            module_id: self.current_module,
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
            scope
                .symbols
                .entry(name)
                .or_insert_with(Vec::new)
                .push(symbol_id);
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
            scope
                .symbols
                .entry(name)
                .or_insert_with(Vec::new)
                .push(symbol_id);
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
                    return Err(format!(
                        "Parameter '{}' already defined in this scope",
                        name
                    ));
                }
            }
        }

        let symbol_id = SymbolId(self.next_symbol_id);
        self.next_symbol_id += 1;

        let symbol = Symbol::parameter(symbol_id, name.clone(), ty, def_span, self.current_scope);

        self.symbols.insert(symbol_id, symbol);

        // Add to current scope
        if let Some(scope) = self.scopes.get_mut(&self.current_scope) {
            scope
                .symbols
                .entry(name)
                .or_insert_with(Vec::new)
                .push(symbol_id);
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
    pub fn lookup_function(&self, name: &str, arg_types: &[Type]) -> Option<&Symbol> {
        let candidates = self.lookup_all(name);

        for symbol in candidates {
            if let Some(sig) = symbol.function_signature() {
                // Check if parameter count matches
                if sig.params.len() != arg_types.len() {
                    continue;
                }

                // Check if all parameter types match
                let params_match = sig
                    .params
                    .iter()
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

    /// Look up a symbol by name starting from a specific scope
    pub fn lookup_in_scope(&self, name: &str, start_scope: ScopeId) -> Option<&Symbol> {
        let mut current = Some(start_scope);

        while let Some(scope_id) = current {
            if let Some(scope) = self.scopes.get(&scope_id) {
                // Check if symbol exists in this scope
                if let Some(symbol_ids) = scope.symbols.get(name) {
                    if let Some(&symbol_id) = symbol_ids.first() {
                        return self.symbols.get(&symbol_id);
                    }
                }
                // Move to parent scope
                current = scope.parent;
            } else {
                break;
            }
        }

        None
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
            scope
                .symbols
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

    /// Define a struct type in the current scope
    pub fn define_struct(
        &mut self,
        name: String,
        info: StructInfo,
        def_span: Span,
    ) -> Result<SymbolId, String> {
        // Check if already defined in current scope
        if let Some(scope) = self.scopes.get(&self.current_scope) {
            if let Some(existing_ids) = scope.symbols.get(&name) {
                if !existing_ids.is_empty() {
                    return Err(format!("Struct '{}' already defined in this scope", name));
                }
            }
        }

        let symbol_id = SymbolId(self.next_symbol_id);
        self.next_symbol_id += 1;

        let symbol =
            Symbol::struct_type(symbol_id, name.clone(), info, def_span, self.current_scope);

        // Add to symbols map
        self.symbols.insert(symbol_id, symbol);

        // Add to current scope
        if let Some(scope) = self.scopes.get_mut(&self.current_scope) {
            scope
                .symbols
                .entry(name)
                .or_insert_with(Vec::new)
                .push(symbol_id);
        }

        Ok(symbol_id)
    }

    /// Define an enum type in the current scope
    pub fn define_enum(
        &mut self,
        name: String,
        info: EnumInfo,
        def_span: Span,
    ) -> Result<SymbolId, String> {
        // Check if already defined in current scope
        if let Some(scope) = self.scopes.get(&self.current_scope) {
            if let Some(existing_ids) = scope.symbols.get(&name) {
                if !existing_ids.is_empty() {
                    return Err(format!("Enum '{}' already defined in this scope", name));
                }
            }
        }

        let symbol_id = SymbolId(self.next_symbol_id);
        self.next_symbol_id += 1;

        let symbol = Symbol::enum_type(symbol_id, name.clone(), info, def_span, self.current_scope);

        // Add to symbols map
        self.symbols.insert(symbol_id, symbol);

        // Add to current scope
        if let Some(scope) = self.scopes.get_mut(&self.current_scope) {
            scope
                .symbols
                .entry(name)
                .or_insert_with(Vec::new)
                .push(symbol_id);
        }

        Ok(symbol_id)
    }

    /// Create a new module
    pub fn create_module(&mut self, name: String) -> ModuleId {
        let module_id = ModuleId(self.next_module_id);
        self.next_module_id += 1;

        // Create root scope for the module
        let root_scope_id = ScopeId(self.next_scope_id);
        self.next_scope_id += 1;

        let root_scope = Scope {
            _id: root_scope_id,
            parent: None,
            module_id,
            symbols: HashMap::new(),
            children: Vec::new(),
        };

        let module_info = ModuleInfo {
            id: module_id,
            name,
            root_scope: root_scope_id,
            imports: HashMap::new(),
            exports: HashMap::new(),
            namespace_imports: HashMap::new(),
        };

        self.scopes.insert(root_scope_id, root_scope);
        self.modules.insert(module_id, module_info);

        module_id
    }

    /// Register an existing module's symbols for import resolution
    pub fn register_module(&mut self, module_name: &str, source_table: &SymbolTable) {
        // Create a module entry if it doesn't exist
        let module_id = self.find_or_create_module(module_name);

        // Copy all symbols from the source table's global scope to this module
        if let Some(source_global_scope) = source_table.scopes.values().find(|s| s.parent.is_none())
        {
            // Get the module info for updating
            if let Some(module_info) = self.modules.get_mut(&module_id) {
                // Copy symbols from source global scope to our module's root scope
                if let Some(target_scope) = self.scopes.get_mut(&module_info.root_scope) {
                    for (name, symbol_ids) in &source_global_scope.symbols {
                        for &symbol_id in symbol_ids {
                            if let Some(source_symbol) = source_table.symbols.get(&symbol_id) {
                                // Create a new symbol in our table
                                let new_symbol_id = SymbolId(self.next_symbol_id);
                                self.next_symbol_id += 1;

                                let new_symbol = Symbol {
                                    id: new_symbol_id,
                                    name: source_symbol.name.clone(),
                                    kind: source_symbol.kind.clone(),
                                    ty: source_symbol.ty.clone(),
                                    def_span: source_symbol.def_span,
                                    is_mutable: source_symbol.is_mutable,
                                    is_used: false,
                                    scope_id: self.current_scope,
                                };

                                // Add to our symbols and scope
                                self.symbols.insert(new_symbol_id, new_symbol);
                                target_scope
                                    .symbols
                                    .entry(name.clone())
                                    .or_insert_with(Vec::new)
                                    .push(new_symbol_id);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Switch to a different module context
    pub fn enter_module(&mut self, module_id: ModuleId) -> Result<(), String> {
        if let Some(module) = self.modules.get(&module_id) {
            self.current_module = module_id;
            self.current_scope = module.root_scope;
            Ok(())
        } else {
            Err(format!("Module {:?} not found", module_id))
        }
    }

    /// Get current module ID
    pub fn current_module(&self) -> ModuleId {
        self.current_module
    }

    /// Get module information
    pub fn get_module(&self, module_id: ModuleId) -> Option<&ModuleInfo> {
        self.modules.get(&module_id)
    }

    /// Get mutable module information
    pub fn get_module_mut(&mut self, module_id: ModuleId) -> Option<&mut ModuleInfo> {
        self.modules.get_mut(&module_id)
    }

    /// Process an import statement
    pub fn process_import(
        &mut self,
        imports: &[ImportSpecifier],
        module_path: &str,
        span: Span,
    ) -> Result<(), String> {
        // For now, we'll create a placeholder module if it doesn't exist
        // In a full implementation, this would load and parse the module
        let source_module_id = self.find_or_create_module(module_path);

        for import in imports {
            match import {
                ImportSpecifier::Named { name, alias } => {
                    self.process_named_import_item(name, alias.as_ref(), source_module_id, span)?;
                }
                ImportSpecifier::Namespace { alias } => {
                    self.process_namespace_import(alias, source_module_id, span)?;
                }
                ImportSpecifier::Default { name } => {
                    self.process_default_import(name, source_module_id, span)?;
                }
            }
        }

        Ok(())
    }

    /// Process an export statement
    pub fn process_export(&mut self, export: &ExportKind, span: Span) -> Result<(), String> {
        match export {
            ExportKind::Named { specifiers } => {
                for item in specifiers {
                    self.process_named_export_item(&item.name, item.alias.as_ref(), span)?;
                }
            }
            ExportKind::Function { name, .. } | ExportKind::Variable { name, .. } => {
                // For declaration exports, the symbol should already be in the table
                self.process_named_export_item(name, None, span)?;
            }
            ExportKind::Default { .. } => {
                // Create a special symbol for default export
                let symbol_id = SymbolId(self.next_symbol_id);
                self.next_symbol_id += 1;

                let symbol = Symbol {
                    id: symbol_id,
                    name: "default".to_string(),
                    kind: SymbolKind::Variable,
                    ty: Type::Unknown, // Type will be set during semantic analysis
                    def_span: span,
                    is_mutable: false,
                    is_used: false,
                    scope_id: self.current_scope,
                };

                self.symbols.insert(symbol_id, symbol);

                // Mark it as exported from current module
                if let Some(module) = self.modules.get_mut(&self.current_module) {
                    module.exports.insert(
                        "default".to_string(),
                        ExportedSymbol {
                            symbol_id,
                            external_name: "default".to_string(),
                            export_span: span,
                        },
                    );
                }
            }
            ExportKind::Declaration(_) => {
                // Declaration exports are handled by the analyzer
                // which analyzes the declaration first, then marks it as exported
            }
        }
        Ok(())
    }

    /// Look up a symbol with module-aware resolution
    pub fn lookup_with_modules(&self, name: &str) -> Option<&Symbol> {
        // First check current module scope chain
        if let Some(symbol) = self.lookup(name) {
            return Some(symbol);
        }

        // Check imports in current module
        if let Some(module) = self.modules.get(&self.current_module) {
            if let Some(imported) = module.imports.get(name) {
                return self.symbols.get(&imported.source_symbol_id);
            }

            // Check namespace imports
            let parts: Vec<&str> = name.split('.').collect();
            if parts.len() == 2 {
                if let Some(&namespace_module_id) = module.namespace_imports.get(parts[0]) {
                    // Look up the symbol in the namespace module
                    return self.lookup_in_module(parts[1], namespace_module_id);
                }
            }
        }

        None
    }

    /// Helper method to find or create a module
    fn find_or_create_module(&mut self, module_path: &str) -> ModuleId {
        // Look for existing module
        for module in self.modules.values() {
            if module.name == module_path {
                return module.id;
            }
        }

        // For now, still create placeholder modules for missing imports
        // In a proper implementation, this should return an error
        self.create_module(module_path.to_string())
    }

    /// Find a module by name without creating a placeholder
    fn find_module(&self, module_path: &str) -> Option<ModuleId> {
        for module in self.modules.values() {
            if module.name == module_path {
                return Some(module.id);
            }
        }
        None
    }

    /// Helper method to process named imports
    fn process_named_import_item(
        &mut self,
        name: &str,
        alias: Option<&String>,
        source_module_id: ModuleId,
        span: Span,
    ) -> Result<(), String> {
        let local_name = alias.map(|s| s.as_str()).unwrap_or(name);

        // Check if the source module actually has the requested symbol
        if let Some(source_module) = self.modules.get(&source_module_id) {
            if let Some(source_scope) = self.scopes.get(&source_module.root_scope) {
                // Look for the symbol in the source module's scope
                if let Some(symbol_ids) = source_scope.symbols.get(name) {
                    if symbol_ids.is_empty() {
                        return Err(format!(
                            "Symbol '{}' not found in module '{}'",
                            name, source_module.name
                        ));
                    }

                    // For now, just take the first symbol (could be improved for overloading)
                    let source_symbol_id = symbol_ids[0];

                    if let Some(source_symbol) = self.symbols.get(&source_symbol_id) {
                        // Create imported symbol record
                        let imported_symbol = ImportedSymbol {
                            source_symbol_id,
                            source_module: source_module_id,
                            original_name: name.to_string(),
                            local_name: local_name.to_string(),
                            import_span: span,
                        };

                        // Add to current module's imports
                        if let Some(current_module) = self.modules.get_mut(&self.current_module) {
                            current_module
                                .imports
                                .insert(local_name.to_string(), imported_symbol);
                        }

                        // Add symbol to current scope with local name
                        let new_symbol_id = SymbolId(self.next_symbol_id);
                        self.next_symbol_id += 1;

                        let new_symbol = Symbol {
                            id: new_symbol_id,
                            name: local_name.to_string(),
                            kind: source_symbol.kind.clone(),
                            ty: source_symbol.ty.clone(),
                            def_span: span,
                            is_mutable: source_symbol.is_mutable,
                            is_used: false,
                            scope_id: self.current_scope,
                        };

                        self.symbols.insert(new_symbol_id, new_symbol);

                        // Add to current scope
                        if let Some(current_scope) = self.scopes.get_mut(&self.current_scope) {
                            current_scope
                                .symbols
                                .entry(local_name.to_string())
                                .or_insert_with(Vec::new)
                                .push(new_symbol_id);
                        }

                        return Ok(());
                    }
                }
            }
        }

        // Symbol not found in registered module
        Err(format!("Symbol '{}' not found in module", name))
    }

    /// Helper method to process namespace imports
    fn process_namespace_import(
        &mut self,
        local_name: &str,
        source_module_id: ModuleId,
        _span: Span,
    ) -> Result<(), String> {
        if let Some(module) = self.modules.get_mut(&self.current_module) {
            if module.namespace_imports.contains_key(local_name) {
                return Err(format!("Namespace '{}' is already imported", local_name));
            }
            module
                .namespace_imports
                .insert(local_name.to_string(), source_module_id);
        }

        Ok(())
    }

    /// Helper method to process default imports
    fn process_default_import(
        &mut self,
        local_name: &str,
        source_module_id: ModuleId,
        span: Span,
    ) -> Result<(), String> {
        // Look for default export in source module
        if let Some(source_symbol_id) = self
            .lookup_in_module("default", source_module_id)
            .map(|s| s.id)
        {
            let imported_symbol = ImportedSymbol {
                source_symbol_id,
                source_module: source_module_id,
                original_name: "default".to_string(),
                local_name: local_name.to_string(),
                import_span: span,
            };

            if let Some(module) = self.modules.get_mut(&self.current_module) {
                if module.imports.contains_key(local_name) {
                    return Err(format!("Symbol '{}' is already imported", local_name));
                }
                module
                    .imports
                    .insert(local_name.to_string(), imported_symbol);
            }
        } else {
            return Err(format!("No default export found in module"));
        }

        Ok(())
    }

    /// Helper method to process named exports
    fn process_named_export_item(
        &mut self,
        name: &str,
        alias: Option<&String>,
        span: Span,
    ) -> Result<(), String> {
        let external_name = alias.map(|s| s.as_str()).unwrap_or(name);

        // Check if the symbol exists in current module
        if let Some(symbol) = self.lookup(name) {
            let exported_symbol = ExportedSymbol {
                symbol_id: symbol.id,
                external_name: external_name.to_string(),
                export_span: span,
            };

            if let Some(module) = self.modules.get_mut(&self.current_module) {
                if module.exports.contains_key(external_name) {
                    return Err(format!("Symbol '{}' is already exported", external_name));
                }
                module
                    .exports
                    .insert(external_name.to_string(), exported_symbol);
            }
        } else {
            return Err(format!("Symbol '{}' not found for export", name));
        }

        Ok(())
    }

    /// Helper method to process re-exports
    fn process_re_export(
        &mut self,
        item: &ExportSpecifier,
        source_module_id: ModuleId,
        span: Span,
    ) -> Result<(), String> {
        let external_name = item.alias.as_ref().unwrap_or(&item.name);

        // Check if the symbol exists in the source module
        if let Some(source_symbol_id) = self
            .lookup_in_module(&item.name, source_module_id)
            .map(|s| s.id)
        {
            let exported_symbol = ExportedSymbol {
                symbol_id: source_symbol_id,
                external_name: external_name.to_string(),
                export_span: span,
            };

            if let Some(module) = self.modules.get_mut(&self.current_module) {
                if module.exports.contains_key(external_name) {
                    return Err(format!("Symbol '{}' is already exported", external_name));
                }
                module
                    .exports
                    .insert(external_name.to_string(), exported_symbol);
            }
        } else {
            return Err(format!(
                "Symbol '{}' not found in source module for re-export",
                item.name
            ));
        }

        Ok(())
    }

    /// Look up a symbol in a specific module
    fn lookup_in_module(&self, name: &str, module_id: ModuleId) -> Option<&Symbol> {
        if let Some(module) = self.modules.get(&module_id) {
            // Search in module's root scope
            let root_scope = &module.root_scope;
            self.lookup_in_scope(name, *root_scope)
        } else {
            None
        }
    }

    /// Get all symbols in the table for export extraction
    pub fn all_symbols(&self) -> impl Iterator<Item = (&String, &Symbol)> + '_ {
        self.symbols.values().filter_map(move |symbol| {
            // Find the symbol's name by searching through all scopes
            for scope in self.scopes.values() {
                for (name, symbol_ids) in &scope.symbols {
                    if symbol_ids.contains(&symbol.id) {
                        return Some((name, symbol));
                    }
                }
            }
            None
        })
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
        Span::new(SourceLocation::new(1, 1, 0), SourceLocation::new(1, 5, 5))
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

        let _x_id = table
            .define_variable("x".to_string(), Type::I32, make_span(), true)
            .unwrap();

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
        table
            .define_variable("x".to_string(), Type::I32, make_span(), true)
            .unwrap();

        // Enter new scope
        let _inner_scope = table.enter_scope();
        assert!(!table.is_global_scope());

        // Can see outer variable
        assert!(table.lookup("x").is_some());

        // Shadow in inner scope
        table
            .define_variable("x".to_string(), Type::F32, make_span(), false)
            .unwrap();
        let inner_x = table.lookup("x").unwrap();
        assert_eq!(inner_x.ty, Type::F32);
        assert!(!inner_x.is_mutable);

        // Define new variable in inner scope
        table
            .define_variable("y".to_string(), Type::Bool, make_span(), true)
            .unwrap();

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
            generic_params: None,
            params: vec![("x".to_string(), Type::I32)],
            return_type: Type::I32,
            is_const: false,
            is_async: false,
            generic_params: vec![],
        };
        table
            .define_function("add".to_string(), sig1, make_span())
            .unwrap();

        // Define overload with two parameters
        let sig2 = FunctionSignature {
            generic_params: None,
            params: vec![("x".to_string(), Type::I32), ("y".to_string(), Type::I32)],
            return_type: Type::I32,
            is_const: false,
            is_async: false,
            generic_params: vec![],
        };
        table
            .define_function("add".to_string(), sig2, make_span())
            .unwrap();

        // Look up function with one argument
        let func1 = table.lookup_function("add", &[Type::I32]).unwrap();
        assert_eq!(func1.function_signature().unwrap().params.len(), 1);

        // Look up function with two arguments
        let func2 = table
            .lookup_function("add", &[Type::I32, Type::I32])
            .unwrap();
        assert_eq!(func2.function_signature().unwrap().params.len(), 2);

        // No matching overload
        assert!(table.lookup_function("add", &[Type::String]).is_none());
    }

    #[test]
    fn test_duplicate_definition_error() {
        let mut table = SymbolTable::new();

        // Define variable
        table
            .define_variable("x".to_string(), Type::I32, make_span(), true)
            .unwrap();

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
            generic_params: None,
            params: vec![("x".to_string(), Type::I32)],
            return_type: Type::I32,
            is_const: false,
            is_async: false,
            generic_params: vec![],
        };
        table
            .define_function("foo".to_string(), sig1.clone(), make_span())
            .unwrap();

        // Try to define with same signature (different return type doesn't matter)
        let sig2 = FunctionSignature {
            generic_params: None,
            params: vec![("y".to_string(), Type::I32)], // Different param name doesn't matter
            return_type: Type::F32,
            is_const: true,
            is_async: false,
            generic_params: vec![],
        };
        let result = table.define_function("foo".to_string(), sig2, make_span());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("same signature"));
    }

    #[test]
    fn test_unused_symbols() {
        let mut table = SymbolTable::new();

        let x_id = table
            .define_variable("x".to_string(), Type::I32, make_span(), true)
            .unwrap();
        let _y_id = table
            .define_variable("y".to_string(), Type::I32, make_span(), true)
            .unwrap();

        // Mark x as used
        table.mark_used(x_id);

        let unused = table.get_unused_symbols();
        assert_eq!(unused.len(), 1);
        assert_eq!(unused[0].name, "y");
    }
}
