use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;

use crate::runtime::Value;
use crate::semantic::{FunctionSignature, SymbolTable};
use crate::types::Type;

/// REPL session state that persists between commands
pub struct Session {
    /// Variables defined in the session
    variables: HashMap<String, Value>,
    /// Function definitions
    functions: HashMap<String, FunctionSignature>,
    /// Type definitions
    types: HashMap<String, Type>,
    /// Symbol table for the session
    symbol_table: SymbolTable,
    /// Session file path for persistence
    session_file: Option<PathBuf>,
}

impl Session {
    /// Create a new session
    pub fn new() -> Self {
        Session {
            variables: HashMap::new(),
            functions: HashMap::new(),
            types: HashMap::new(),
            symbol_table: SymbolTable::new(),
            session_file: Self::get_session_file_path().ok(),
        }
    }

    /// Get the default session file path
    fn get_session_file_path() -> io::Result<PathBuf> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Home directory not found"))?;

        Ok(home_dir.join(".script_session"))
    }

    /// Load session from file if it exists
    pub fn load_or_create() -> io::Result<Self> {
        let mut session = Self::new();

        if let Some(session_file) = &session.session_file.clone() {
            if session_file.exists() {
                session.load_from_file()?;
            }
        }

        Ok(session)
    }

    /// Load session state from file
    fn load_from_file(&mut self) -> io::Result<()> {
        if let Some(session_file) = &self.session_file {
            if session_file.exists() {
                let content = fs::read_to_string(session_file)?;
                if !content.trim().is_empty() {
                    match self.deserialize_session(&content) {
                        Ok(()) => {
                            println!("Loaded session with {} items", self.item_count());
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to parse session file: {e}");
                            // Don't fail completely, just use empty session
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Save session state to file
    pub fn save(&self) -> io::Result<()> {
        if let Some(session_file) = &self.session_file {
            // Create parent directory if needed
            if let Some(parent) = session_file.parent() {
                fs::create_dir_all(parent)?;
            }

            let serialized = self.serialize_session()?;
            fs::write(session_file, serialized)?;
        }
        Ok(())
    }

    /// Serialize session state to JSON
    fn serialize_session(&self) -> io::Result<String> {
        let mut session_data = serde_json::Map::new();

        // Serialize variables (simplified - would need proper Value serialization)
        let mut vars = serde_json::Map::new();
        for (name, value) in &self.variables {
            // For now, just store as debug string
            // In production, we'd need proper Value serialization
            vars.insert(
                name.clone(),
                serde_json::Value::String(format!("{:?}", value)),
            );
        }
        session_data.insert("variables".to_string(), serde_json::Value::Object(vars));

        // Serialize types (simplified)
        let mut types = serde_json::Map::new();
        for (name, type_def) in &self.types {
            types.insert(
                name.clone(),
                serde_json::Value::String(format!("{:?}", type_def)),
            );
        }
        session_data.insert("types".to_string(), serde_json::Value::Object(types));

        // Serialize functions (simplified)
        let mut funcs = serde_json::Map::new();
        for (name, signature) in &self.functions {
            funcs.insert(
                name.clone(),
                serde_json::Value::String(format!("{:?}", signature)),
            );
        }
        session_data.insert("functions".to_string(), serde_json::Value::Object(funcs));

        // Add metadata
        session_data.insert(
            "version".to_string(),
            serde_json::Value::String("0.5.0-alpha".to_string()),
        );
        session_data.insert(
            "timestamp".to_string(),
            serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
        );

        let json_value = serde_json::Value::Object(session_data);
        serde_json::to_string_pretty(&json_value)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Deserialize session state from JSON
    fn deserialize_session(&mut self, content: &str) -> Result<(), String> {
        let json: serde_json::Value =
            serde_json::from_str(content).map_err(|e| format!("JSON parse error: {e}"))?;

        if let Some(obj) = json.as_object() {
            // Check version compatibility
            if let Some(version) = obj.get("version").and_then(|v| v.as_str()) {
                if version != "0.5.0-alpha" {
                    return Err(format!(
                        "Session version mismatch: expected 0.5.0-alpha, found {}",
                        version
                    ));
                }
            }

            // For now, just acknowledge the load
            // In a full implementation, we'd deserialize all the data structures
            // This would require proper serialization/deserialization for Value, Type, etc.
            println!("Session file format recognized (simplified loading)");
        }

        Ok(())
    }

    /// Define a variable in the session
    pub fn define_variable(&mut self, name: String, value: Value, _var_type: Type) {
        self.variables.insert(name.clone(), value);

        // Also add to symbol table for type checking
        // Note: This is a simplified implementation
        // In practice, we'd need proper scope management
    }

    /// Get a variable from the session
    pub fn get_variable(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }

    /// Define a function in the session
    pub fn define_function(&mut self, name: String, signature: FunctionSignature) {
        self.functions.insert(name, signature);
    }

    /// Get a function from the session
    pub fn get_function(&self, name: &str) -> Option<&FunctionSignature> {
        self.functions.get(name)
    }

    /// Define a type in the session
    pub fn define_type(&mut self, name: String, type_def: Type) {
        self.types.insert(name, type_def);
    }

    /// Get a type from the session
    pub fn get_type(&self, name: &str) -> Option<&Type> {
        self.types.get(name)
    }

    /// Get all variables (for display)
    pub fn variables(&self) -> &HashMap<String, Value> {
        &self.variables
    }

    /// Get all functions (for display)
    pub fn functions(&self) -> &HashMap<String, FunctionSignature> {
        &self.functions
    }

    /// Get all types (for display)
    pub fn types(&self) -> &HashMap<String, Type> {
        &self.types
    }

    /// Get the symbol table
    pub fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }

    /// Get a mutable reference to the symbol table
    pub fn symbol_table_mut(&mut self) -> &mut SymbolTable {
        &mut self.symbol_table
    }

    /// Clear all session state
    pub fn clear(&mut self) {
        self.variables.clear();
        self.functions.clear();
        self.types.clear();
        self.symbol_table = SymbolTable::new();
    }

    /// Check if a name is already defined
    pub fn is_defined(&self, name: &str) -> bool {
        self.variables.contains_key(name)
            || self.functions.contains_key(name)
            || self.types.contains_key(name)
    }

    /// Get count of defined items
    pub fn item_count(&self) -> usize {
        self.variables.len() + self.functions.len() + self.types.len()
    }

    /// Import definitions from another session
    pub fn import_from(&mut self, other: &Session) {
        // Import variables
        for (name, value) in &other.variables {
            self.variables.insert(name.clone(), value.clone());
        }

        // Import functions
        for (name, signature) in &other.functions {
            self.functions.insert(name.clone(), signature.clone());
        }

        // Import types
        for (name, type_def) in &other.types {
            self.types.insert(name.clone(), type_def.clone());
        }
    }

    /// Export session state as a summary string
    pub fn summary(&self) -> String {
        let var_count = self.variables.len();
        let func_count = self.functions.len();
        let type_count = self.types.len();

        format!(
            "Session: {} variables, {} functions, {} types",
            var_count, func_count, type_count
        )
    }

    /// List all defined names
    pub fn list_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        names.extend(self.variables.keys().cloned());
        names.extend(self.functions.keys().cloned());
        names.extend(self.types.keys().cloned());
        names.sort();
        names
    }

    /// Remove a definition by name
    pub fn remove(&mut self, name: &str) -> bool {
        let removed_var = self.variables.remove(name).is_some();
        let removed_func = self.functions.remove(name).is_some();
        let removed_type = self.types.remove(name).is_some();

        removed_var || removed_func || removed_type
    }

    /// Validate session consistency
    pub fn validate(&self) -> Result<(), String> {
        // Check for naming conflicts
        let mut all_names = std::collections::HashSet::new();

        for name in self.variables.keys() {
            if !all_names.insert(name) {
                return Err(format!("Duplicate definition: {name}"));
            }
        }

        for name in self.functions.keys() {
            if !all_names.insert(name) {
                return Err(format!("Duplicate definition: {name}"));
            }
        }

        for name in self.types.keys() {
            if !all_names.insert(name) {
                return Err(format!("Duplicate definition: {name}"));
            }
        }

        Ok(())
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Type;

    #[test]
    fn test_session_variables() {
        let mut session = Session::new();

        session.define_variable("x".to_string(), Value::I32(42), Type::I32);

        assert!(session.is_defined("x"));
        assert_eq!(session.get_variable("x"), Some(&Value::I32(42)));
        assert_eq!(session.variables().len(), 1);
    }

    #[test]
    fn test_session_clear() {
        let mut session = Session::new();

        session.define_variable("x".to_string(), Value::I32(42), Type::I32);
        session.define_type("MyType".to_string(), Type::String);

        assert_eq!(session.item_count(), 2);

        session.clear();
        assert_eq!(session.item_count(), 0);
        assert!(!session.is_defined("x"));
    }

    #[test]
    fn test_session_remove() {
        let mut session = Session::new();

        session.define_variable("x".to_string(), Value::I32(42), Type::I32);
        assert!(session.is_defined("x"));

        assert!(session.remove("x"));
        assert!(!session.is_defined("x"));

        // Removing non-existent item should return false
        assert!(!session.remove("y"));
    }

    #[test]
    fn test_session_validation() {
        let session = Session::new();
        assert!(session.validate().is_ok());

        // Note: In the current implementation, we don't have naming conflicts
        // because each definition type uses a separate HashMap
        // This test would be more relevant if we change the implementation
    }

    #[test]
    fn test_session_summary() {
        let mut session = Session::new();

        session.define_variable("x".to_string(), Value::I32(42), Type::I32);
        session.define_type("MyType".to_string(), Type::String);

        let summary = session.summary();
        assert!(summary.contains("1 variables"));
        assert!(summary.contains("0 functions"));
        assert!(summary.contains("1 types"));
    }
}
