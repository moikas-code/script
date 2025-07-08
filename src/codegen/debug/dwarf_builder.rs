//! DWARF debug information builder
//!
//! This module provides DWARF debug information generation for the Script language.
//! It supports types, functions, variables, line numbers, and lexical scopes.

use crate::error::Error;
use crate::ir::Function as IrFunction;
use crate::source::SourceLocation;
use crate::types::Type as ScriptType;
use std::collections::HashMap;
use super::safe_conversions::{usize_to_u32_add, validate_line_number, validate_column_number, validate_file_count};

/// DWARF debug information entry
#[derive(Debug, Clone)]
pub struct DwarfEntry {
    pub id: u32,
    pub tag: DwarfTag,
    pub attributes: HashMap<DwarfAttribute, DwarfValue>,
    pub children: Vec<u32>,
}

/// DWARF tags for different entry types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DwarfTag {
    CompileUnit,
    Function,
    Variable,
    BaseType,
    ArrayType,
    PointerType,
    StructType,
    LexicalBlock,
    FormalParameter,
}

/// DWARF attributes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DwarfAttribute {
    Name,
    Type,
    LowPc,
    HighPc,
    ByteSize,
    Encoding,
    Location,
    DeclFile,
    DeclLine,
    DeclColumn,
}

/// DWARF attribute values
#[derive(Debug, Clone)]
pub enum DwarfValue {
    String(String),
    Address(u64),
    UInt(u32),
    Encoding(DwarfEncoding),
    Reference(u32),
    Location(DwarfLocation),
}

/// DWARF type encodings
#[derive(Debug, Clone, Copy)]
pub enum DwarfEncoding {
    Boolean,
    SignedInt,
    UnsignedInt,
    Float,
    Address,
}

/// DWARF location expressions (simplified)
#[derive(Debug, Clone)]
pub enum DwarfLocation {
    Register(u32),
    FrameOffset(i32),
    GlobalAddress(u64),
}

/// DWARF debug information builder
pub struct DwarfBuilder {
    /// All DWARF entries
    entries: HashMap<u32, DwarfEntry>,
    /// Map from Script types to DWARF entry IDs
    type_map: HashMap<ScriptType, u32>,
    /// Map from function names to DWARF entry IDs
    function_map: HashMap<String, u32>,
    /// Map from variable names to DWARF entry IDs
    variable_map: HashMap<String, u32>,
    /// Compilation unit entry ID
    compile_unit_id: Option<u32>,
    /// Current lexical scope stack
    scope_stack: Vec<u32>,
    /// Next ID to assign
    next_id: u32,
    /// Line number table
    line_entries: Vec<LineEntry>,
    /// Source files
    source_files: HashMap<String, u32>,
}

/// Line number table entry
#[derive(Debug, Clone)]
pub struct LineEntry {
    pub address: u64,
    pub file_id: u32,
    pub line: u32,
    pub column: u32,
}

impl DwarfBuilder {
    /// Create a new DWARF builder
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            type_map: HashMap::new(),
            function_map: HashMap::new(),
            variable_map: HashMap::new(),
            compile_unit_id: None,
            scope_stack: Vec::new(),
            next_id: 1,
            line_entries: Vec::new(),
            source_files: HashMap::new(),
        }
    }

    /// Create the compilation unit entry
    pub fn create_compilation_unit(&mut self, name: &str, _producer: &str, _language: u32) -> u32 {
        let mut attributes = HashMap::new();
        attributes.insert(DwarfAttribute::Name, DwarfValue::String(name.to_string()));

        let entry = DwarfEntry {
            id: self.next_id,
            tag: DwarfTag::CompileUnit,
            attributes,
            children: Vec::new(),
        };

        let id = self.next_id;
        self.next_id += 1;
        self.entries.insert(id, entry);
        self.compile_unit_id = Some(id);
        self.scope_stack.push(id);
        id
    }

    /// Add a source file
    pub fn add_source_file(&mut self, filename: &str) -> Result<u32, Error> {
        if let Some(&existing_id) = self.source_files.get(filename) {
            return Ok(existing_id);
        }

        // Validate file count is within limits
        validate_file_count(self.source_files.len())?;
        
        // Safely convert and add 1
        let file_id = usize_to_u32_add(self.source_files.len(), 1)?;
        self.source_files.insert(filename.to_string(), file_id);
        Ok(file_id)
    }

    /// Add a line number entry
    pub fn add_line_entry(&mut self, address: u64, location: &SourceLocation, filename: &str) -> Result<(), Error> {
        let file_id = self.add_source_file(filename)?;
        
        // Validate and convert line and column numbers
        let line = validate_line_number(location.line)?;
        let column = validate_column_number(location.column)?;

        self.line_entries.push(LineEntry {
            address,
            file_id,
            line,
            column,
        });
        Ok(())
    }

    /// Add a base type to the DWARF info
    pub fn add_base_type(&mut self, script_type: &ScriptType) -> u32 {
        if let Some(&existing_id) = self.type_map.get(script_type) {
            return existing_id;
        }

        let (name, byte_size, encoding) = match script_type {
            ScriptType::I32 => ("i32", 4, DwarfEncoding::SignedInt),
            ScriptType::F32 => ("f32", 4, DwarfEncoding::Float),
            ScriptType::Bool => ("bool", 1, DwarfEncoding::Boolean),
            ScriptType::String => ("string", 8, DwarfEncoding::Address), // Pointer to string data
            // No null type in Script - use unknown instead
            ScriptType::Unknown => ("unknown", 8, DwarfEncoding::Address),
            _ => ("complex_type", 8, DwarfEncoding::Address),
        };

        let mut attributes = HashMap::new();
        attributes.insert(DwarfAttribute::Name, DwarfValue::String(name.to_string()));
        attributes.insert(DwarfAttribute::ByteSize, DwarfValue::UInt(byte_size));
        attributes.insert(DwarfAttribute::Encoding, DwarfValue::Encoding(encoding));

        let entry = DwarfEntry {
            id: self.next_id,
            tag: DwarfTag::BaseType,
            attributes,
            children: Vec::new(),
        };

        let type_id = self.next_id;
        self.next_id += 1;
        self.entries.insert(type_id, entry);
        self.type_map.insert(script_type.clone(), type_id);

        // Add to compilation unit children
        if let Some(cu_id) = self.compile_unit_id {
            if let Some(cu_entry) = self.entries.get_mut(&cu_id) {
                cu_entry.children.push(type_id);
            }
        }

        type_id
    }

    /// Add an array type to the DWARF info
    pub fn add_array_type(&mut self, element_type: &ScriptType) -> u32 {
        let elem_type_id = self.add_base_type(element_type);

        let mut attributes = HashMap::new();
        attributes.insert(DwarfAttribute::Type, DwarfValue::Reference(elem_type_id));

        let entry = DwarfEntry {
            id: self.next_id,
            tag: DwarfTag::ArrayType,
            attributes,
            children: Vec::new(),
        };

        let array_type_id = self.next_id;
        self.next_id += 1;
        self.entries.insert(array_type_id, entry);

        array_type_id
    }

    /// Add a function type to the DWARF info
    pub fn add_function_type(&mut self, _params: &[ScriptType], _return_type: &ScriptType) -> u32 {
        let type_id = self.next_id;
        self.next_id += 1;
        type_id
    }

    /// Add a function to the DWARF info
    pub fn add_function(
        &mut self,
        function: &IrFunction,
        start_address: u64,
        end_address: u64,
        source_file: Option<&str>,
        source_line: Option<u32>,
    ) -> Result<u32, Error> {
        let mut attributes = HashMap::new();
        attributes.insert(
            DwarfAttribute::Name,
            DwarfValue::String(function.name.clone()),
        );
        attributes.insert(DwarfAttribute::LowPc, DwarfValue::Address(start_address));
        attributes.insert(DwarfAttribute::HighPc, DwarfValue::Address(end_address));

        if let Some(filename) = source_file {
            let file_id = self.add_source_file(filename)?;
            attributes.insert(DwarfAttribute::DeclFile, DwarfValue::UInt(file_id));
        }

        if let Some(line) = source_line {
            attributes.insert(DwarfAttribute::DeclLine, DwarfValue::UInt(line));
        }

        let entry = DwarfEntry {
            id: self.next_id,
            tag: DwarfTag::Function,
            attributes,
            children: Vec::new(),
        };

        let func_id = self.next_id;
        self.next_id += 1;
        self.entries.insert(func_id, entry);
        self.function_map.insert(function.name.clone(), func_id);

        // Add function parameters
        for param in &function.params {
            let param_id = self.add_function_parameter(&param.name, &param.ty, func_id);
            if let Some(func_entry) = self.entries.get_mut(&func_id) {
                func_entry.children.push(param_id);
            }
        }

        // Add to compilation unit children
        if let Some(cu_id) = self.compile_unit_id {
            if let Some(cu_entry) = self.entries.get_mut(&cu_id) {
                cu_entry.children.push(func_id);
            }
        }

        self.scope_stack.push(func_id);
        Ok(func_id)
    }

    /// Add a function parameter
    fn add_function_parameter(
        &mut self,
        name: &str,
        param_type: &ScriptType,
        _parent_id: u32,
    ) -> u32 {
        let type_id = self.add_base_type(param_type);

        let mut attributes = HashMap::new();
        attributes.insert(DwarfAttribute::Name, DwarfValue::String(name.to_string()));
        attributes.insert(DwarfAttribute::Type, DwarfValue::Reference(type_id));

        let entry = DwarfEntry {
            id: self.next_id,
            tag: DwarfTag::FormalParameter,
            attributes,
            children: Vec::new(),
        };

        let param_id = self.next_id;
        self.next_id += 1;
        self.entries.insert(param_id, entry);

        param_id
    }

    /// Add a variable to the DWARF info
    pub fn add_variable(
        &mut self,
        name: &str,
        var_type: &ScriptType,
        location: Option<DwarfLocation>,
        source_file: Option<&str>,
        source_line: Option<u32>,
    ) -> Result<u32, Error> {
        let type_id = self.add_base_type(var_type);

        let mut attributes = HashMap::new();
        attributes.insert(DwarfAttribute::Name, DwarfValue::String(name.to_string()));
        attributes.insert(DwarfAttribute::Type, DwarfValue::Reference(type_id));

        if let Some(loc) = location {
            attributes.insert(DwarfAttribute::Location, DwarfValue::Location(loc));
        }

        if let Some(filename) = source_file {
            let file_id = self.add_source_file(filename)?;
            attributes.insert(DwarfAttribute::DeclFile, DwarfValue::UInt(file_id));
        }

        if let Some(line) = source_line {
            attributes.insert(DwarfAttribute::DeclLine, DwarfValue::UInt(line));
        }

        let entry = DwarfEntry {
            id: self.next_id,
            tag: DwarfTag::Variable,
            attributes,
            children: Vec::new(),
        };

        let var_id = self.next_id;
        self.next_id += 1;
        self.entries.insert(var_id, entry);
        self.variable_map.insert(name.to_string(), var_id);

        // Add to current scope
        if let Some(&current_scope) = self.scope_stack.last() {
            if let Some(scope_entry) = self.entries.get_mut(&current_scope) {
                scope_entry.children.push(var_id);
            }
        }

        Ok(var_id)
    }

    /// Add a lexical block (for local scopes)
    pub fn add_lexical_block(&mut self, start_address: u64, end_address: u64) -> u32 {
        let mut attributes = HashMap::new();
        attributes.insert(DwarfAttribute::LowPc, DwarfValue::Address(start_address));
        attributes.insert(DwarfAttribute::HighPc, DwarfValue::Address(end_address));

        let entry = DwarfEntry {
            id: self.next_id,
            tag: DwarfTag::LexicalBlock,
            attributes,
            children: Vec::new(),
        };

        let block_id = self.next_id;
        self.next_id += 1;
        self.entries.insert(block_id, entry);

        // Add to current scope
        if let Some(&current_scope) = self.scope_stack.last() {
            if let Some(scope_entry) = self.entries.get_mut(&current_scope) {
                scope_entry.children.push(block_id);
            }
        }

        self.scope_stack.push(block_id);
        block_id
    }

    /// Close the current lexical scope
    pub fn close_scope(&mut self) {
        self.scope_stack.pop();
    }

    /// Get all DWARF entries
    pub fn get_entries(&self) -> &HashMap<u32, DwarfEntry> {
        &self.entries
    }

    /// Get line number entries
    pub fn get_line_entries(&self) -> &[LineEntry] {
        &self.line_entries
    }

    /// Get source files
    pub fn get_source_files(&self) -> &HashMap<String, u32> {
        &self.source_files
    }

    /// Get the compilation unit ID
    pub fn get_compilation_unit_id(&self) -> Option<u32> {
        self.compile_unit_id
    }

    /// Get a function ID by name
    pub fn get_function_id(&self, name: &str) -> Option<u32> {
        self.function_map.get(name).copied()
    }

    /// Get a variable ID by name
    pub fn get_variable_id(&self, name: &str) -> Option<u32> {
        self.variable_map.get(name).copied()
    }

    /// Get a type ID by Script type
    pub fn get_type_id(&self, script_type: &ScriptType) -> Option<u32> {
        self.type_map.get(script_type).copied()
    }
}

impl Default for DwarfBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Parameter;
    use crate::codegen::debug::safe_conversions;

    #[test]
    fn test_dwarf_builder_creation() {
        let builder = DwarfBuilder::new();

        assert!(builder.type_map.is_empty());
        assert!(builder.function_map.is_empty());
        assert!(builder.variable_map.is_empty());
        assert_eq!(builder.next_id, 1);
    }

    #[test]
    fn test_add_base_type() {
        let mut builder = DwarfBuilder::new();

        let i32_type = ScriptType::I32;
        let type_id1 = builder.add_base_type(&i32_type);
        let type_id2 = builder.add_base_type(&i32_type); // Should return same ID

        assert_eq!(type_id1, type_id2);
        assert_eq!(builder.type_map.len(), 1);
    }

    #[test]
    fn test_add_function() -> Result<(), Error> {
        let mut builder = DwarfBuilder::new();
        builder.create_compilation_unit("test.script", "script-compiler", 0);

        let function = IrFunction::new(
            crate::ir::FunctionId(0),
            "test_function".to_string(),
            vec![Parameter {
                name: "param1".to_string(),
                ty: ScriptType::I32,
            }],
            ScriptType::I32,
        );

        let func_id = builder.add_function(&function, 0x1000, 0x2000, Some("test.script"), Some(5))?;

        assert!(builder.function_map.contains_key("test_function"));
        assert_eq!(builder.function_map["test_function"], func_id);

        let entry = &builder.entries[&func_id];
        assert_eq!(entry.tag, DwarfTag::Function);
        assert_eq!(entry.children.len(), 1); // One parameter
        Ok(())
    }

    #[test]
    fn test_add_variable() -> Result<(), Error> {
        let mut builder = DwarfBuilder::new();
        builder.create_compilation_unit("test.script", "script-compiler", 0);

        let var_id = builder.add_variable(
            "test_var",
            &ScriptType::I32,
            Some(DwarfLocation::FrameOffset(-4)),
            Some("test.script"),
            Some(10),
        )?;

        assert!(builder.variable_map.contains_key("test_var"));
        assert_eq!(builder.variable_map["test_var"], var_id);

        let entry = &builder.entries[&var_id];
        assert_eq!(entry.tag, DwarfTag::Variable);
        Ok(())
    }

    #[test]
    fn test_compilation_unit() {
        let mut builder = DwarfBuilder::new();
        let cu_id = builder.create_compilation_unit("main.script", "script-compiler v1.0", 0);

        assert_eq!(builder.compile_unit_id, Some(cu_id));
        let entry = &builder.entries[&cu_id];
        assert_eq!(entry.tag, DwarfTag::CompileUnit);
    }

    #[test]
    fn test_line_entries() -> Result<(), Error> {
        let mut builder = DwarfBuilder::new();
        let location = SourceLocation::new(10, 5, 100);

        builder.add_line_entry(0x1000, &location, "test.script")?;

        assert_eq!(builder.line_entries.len(), 1);
        let entry = &builder.line_entries[0];
        assert_eq!(entry.address, 0x1000);
        assert_eq!(entry.line, 10);
        assert_eq!(entry.column, 5);
        Ok(())
    }

    #[test]
    fn test_lexical_blocks() {
        let mut builder = DwarfBuilder::new();
        builder.create_compilation_unit("test.script", "script-compiler", 0);

        let block_id = builder.add_lexical_block(0x1000, 0x2000);

        let entry = &builder.entries[&block_id];
        assert_eq!(entry.tag, DwarfTag::LexicalBlock);

        builder.close_scope();
        assert_eq!(builder.scope_stack.len(), 1); // Just compilation unit
    }

    #[test]
    fn test_source_file_overflow() {
        let mut builder = DwarfBuilder::new();
        
        // Add files up to the limit
        for i in 0..safe_conversions::limits::MAX_SOURCE_FILES {
            let file_path = format!("/test/file_{}.script", i);
            builder.source_files.insert(file_path, (i + 1) as u32);
        }
        
        // This should fail as we're at the limit
        let result = builder.add_source_file("/test/overflow.script");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Too many source files"));
    }

    #[test]
    fn test_line_number_overflow() -> Result<(), Error> {
        let mut builder = DwarfBuilder::new();
        
        // Valid line number at limit
        let valid_location = SourceLocation::new(
            safe_conversions::limits::MAX_LINE_NUMBER as usize,
            5,
            100
        );
        assert!(builder.add_line_entry(0x1000, &valid_location, "test.script").is_ok());
        
        // Invalid line number beyond limit
        let invalid_location = SourceLocation::new(
            (safe_conversions::limits::MAX_LINE_NUMBER + 1) as usize,
            5,
            100
        );
        let result = builder.add_line_entry(0x2000, &invalid_location, "test.script");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Line number"));
        
        Ok(())
    }

    #[test]
    fn test_column_number_overflow() -> Result<(), Error> {
        let mut builder = DwarfBuilder::new();
        
        // Valid column number at limit
        let valid_location = SourceLocation::new(
            10,
            safe_conversions::limits::MAX_COLUMN_NUMBER as usize,
            100
        );
        assert!(builder.add_line_entry(0x1000, &valid_location, "test.script").is_ok());
        
        // Invalid column number beyond limit
        let invalid_location = SourceLocation::new(
            10,
            (safe_conversions::limits::MAX_COLUMN_NUMBER + 1) as usize,
            100
        );
        let result = builder.add_line_entry(0x2000, &invalid_location, "test.script");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Column number"));
        
        Ok(())
    }

    #[test]
    fn test_add_source_file_edge_case() -> Result<(), Error> {
        let mut builder = DwarfBuilder::new();
        
        // Test that file IDs start at 1
        let file_id1 = builder.add_source_file("test1.script")?;
        assert_eq!(file_id1, 1);
        
        let file_id2 = builder.add_source_file("test2.script")?;
        assert_eq!(file_id2, 2);
        
        // Test deduplication
        let file_id3 = builder.add_source_file("test1.script")?;
        assert_eq!(file_id3, 1);
        
        Ok(())
    }
}
