//! Closure serialization for Script programming language
//!
//! This module provides serialization and deserialization capabilities for closures,
//! enabling persistence, network transmission, and cross-process communication.

use super::{Closure, OptimizedClosure};
use crate::error::{Error, ErrorKind, Result};
use crate::runtime::Value;
use std::collections::HashMap;

/// Serialization format for closures
#[derive(Debug, Clone)]
pub enum SerializationFormat {
    /// Binary format for performance
    Binary,
    /// JSON format for interoperability
    Json,
    /// Custom compact format optimized for closures
    Compact,
}

/// Serialized closure data
#[derive(Debug, Clone)]
pub struct SerializedClosure {
    /// Serialization format used
    pub format: SerializationFormat,
    /// The serialized data
    pub data: Vec<u8>,
    /// Metadata about the closure
    pub metadata: ClosureMetadata,
}

/// Metadata about a serialized closure
#[derive(Debug, Clone)]
pub struct ClosureMetadata {
    /// Function ID
    pub function_id: String,
    /// Parameter count
    pub param_count: usize,
    /// Number of captured variables
    pub capture_count: usize,
    /// Whether captures are by reference
    pub captures_by_ref: bool,
    /// Serialization timestamp
    pub timestamp: u64,
    /// Version of the serialization format
    pub version: u8,
    /// Whether this is an optimized closure
    pub is_optimized: bool,
}

/// Closure serializer with different format support
pub struct ClosureSerializer {
    /// Default serialization format
    default_format: SerializationFormat,
    /// Configuration options
    config: SerializationConfig,
}

/// Configuration for closure serialization
#[derive(Debug, Clone)]
pub struct SerializationConfig {
    /// Include captured variable values in serialization
    pub include_captured_values: bool,
    /// Compress the serialized data
    pub compress: bool,
    /// Maximum size limit for serialized data (bytes)
    pub max_size_bytes: usize,
    /// Include debug information
    pub include_debug_info: bool,
    /// Validate deserialized closures
    pub validate_on_deserialize: bool,
}

impl Default for SerializationConfig {
    fn default() -> Self {
        SerializationConfig {
            include_captured_values: true,
            compress: true,
            max_size_bytes: 1024 * 1024, // 1MB limit
            include_debug_info: false,
            validate_on_deserialize: true,
        }
    }
}

impl ClosureSerializer {
    /// Create a new closure serializer
    pub fn new(format: SerializationFormat) -> Self {
        ClosureSerializer {
            default_format: format,
            config: SerializationConfig::default(),
        }
    }

    /// Create a serializer with custom configuration
    pub fn with_config(format: SerializationFormat, config: SerializationConfig) -> Self {
        ClosureSerializer {
            default_format: format,
            config,
        }
    }

    /// Serialize a closure
    pub fn serialize_closure(&self, closure: &Closure) -> Result<SerializedClosure> {
        let metadata = ClosureMetadata {
            function_id: closure.function_id.clone(),
            param_count: closure.parameters.len(),
            capture_count: closure.captured_vars.len(),
            captures_by_ref: closure.captures_by_ref,
            timestamp: self.current_timestamp(),
            version: 1,
            is_optimized: false,
        };

        let data = match self.default_format {
            SerializationFormat::Binary => self.serialize_closure_binary(closure)?,
            SerializationFormat::Json => self.serialize_closure_json(closure)?,
            SerializationFormat::Compact => self.serialize_closure_compact(closure)?,
        };

        // Check size limit
        if data.len() > self.config.max_size_bytes {
            return Err(Error::new(
                ErrorKind::RuntimeError,
                format!(
                    "Serialized closure exceeds size limit: {} > {} bytes",
                    data.len(),
                    self.config.max_size_bytes
                ),
            ));
        }

        Ok(SerializedClosure {
            format: self.default_format.clone(),
            data,
            metadata,
        })
    }

    /// Serialize an optimized closure
    pub fn serialize_optimized_closure(
        &self,
        closure: &OptimizedClosure,
    ) -> Result<SerializedClosure> {
        let function_name = closure
            .function_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("#{}", closure.function_id));

        let metadata = ClosureMetadata {
            function_id: function_name,
            param_count: closure.parameters.len(),
            capture_count: closure.captured_vars.len(),
            captures_by_ref: closure.captures_by_ref,
            timestamp: self.current_timestamp(),
            version: 1,
            is_optimized: true,
        };

        let data = match self.default_format {
            SerializationFormat::Binary => self.serialize_optimized_closure_binary(closure)?,
            SerializationFormat::Json => self.serialize_optimized_closure_json(closure)?,
            SerializationFormat::Compact => self.serialize_optimized_closure_compact(closure)?,
        };

        // Check size limit
        if data.len() > self.config.max_size_bytes {
            return Err(Error::new(
                ErrorKind::RuntimeError,
                format!(
                    "Serialized optimized closure exceeds size limit: {} > {} bytes",
                    data.len(),
                    self.config.max_size_bytes
                ),
            ));
        }

        Ok(SerializedClosure {
            format: self.default_format.clone(),
            data,
            metadata,
        })
    }

    /// Deserialize into a closure
    pub fn deserialize_closure(&self, serialized: &SerializedClosure) -> Result<Closure> {
        if self.config.validate_on_deserialize {
            self.validate_serialized_data(serialized)?;
        }

        match serialized.format {
            SerializationFormat::Binary => self.deserialize_closure_binary(&serialized.data),
            SerializationFormat::Json => self.deserialize_closure_json(&serialized.data),
            SerializationFormat::Compact => self.deserialize_closure_compact(&serialized.data),
        }
    }

    /// Deserialize into an optimized closure
    pub fn deserialize_optimized_closure(
        &self,
        serialized: &SerializedClosure,
    ) -> Result<OptimizedClosure> {
        if self.config.validate_on_deserialize {
            self.validate_serialized_data(serialized)?;
        }

        if !serialized.metadata.is_optimized {
            return Err(Error::new(
                ErrorKind::RuntimeError,
                "Cannot deserialize non-optimized closure as optimized closure",
            ));
        }

        match serialized.format {
            SerializationFormat::Binary => {
                self.deserialize_optimized_closure_binary(&serialized.data)
            }
            SerializationFormat::Json => self.deserialize_optimized_closure_json(&serialized.data),
            SerializationFormat::Compact => {
                self.deserialize_optimized_closure_compact(&serialized.data)
            }
        }
    }

    /// Binary serialization for regular closures
    fn serialize_closure_binary(&self, closure: &Closure) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();

        // Write function ID
        self.write_string(&mut buffer, &closure.function_id)?;

        // Write parameters
        self.write_u32(&mut buffer, closure.parameters.len() as u32)?;
        for param in &closure.parameters {
            self.write_string(&mut buffer, param)?;
        }

        // Write captures flag
        self.write_bool(&mut buffer, closure.captures_by_ref)?;

        // Write captured variables if configured
        if self.config.include_captured_values {
            self.write_u32(&mut buffer, closure.captured_vars.len() as u32)?;
            for (name, value) in &closure.captured_vars {
                self.write_string(&mut buffer, name)?;
                self.serialize_value_binary(&mut buffer, value)?;
            }
        } else {
            self.write_u32(&mut buffer, 0)?; // No captured values
        }

        if self.config.compress {
            self.compress_data(buffer)
        } else {
            Ok(buffer)
        }
    }

    /// Binary serialization for optimized closures
    fn serialize_optimized_closure_binary(&self, closure: &OptimizedClosure) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();

        // Write function ID (as string)
        let function_name = closure
            .function_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("#{}", closure.function_id));
        self.write_string(&mut buffer, &function_name)?;

        // Write parameters
        self.write_u32(&mut buffer, closure.parameters.len() as u32)?;
        for param in closure.parameters.iter() {
            self.write_string(&mut buffer, param)?;
        }

        // Write captures flag
        self.write_bool(&mut buffer, closure.captures_by_ref)?;

        // Write captured variables if configured
        if self.config.include_captured_values {
            self.write_u32(&mut buffer, closure.captured_vars.len() as u32)?;
            for (name, value) in closure.captured_vars.iter() {
                self.write_string(&mut buffer, name)?;
                self.serialize_value_binary(&mut buffer, value)?;
            }
        } else {
            self.write_u32(&mut buffer, 0)?; // No captured values
        }

        if self.config.compress {
            self.compress_data(buffer)
        } else {
            Ok(buffer)
        }
    }

    /// JSON serialization for regular closures
    fn serialize_closure_json(&self, closure: &Closure) -> Result<Vec<u8>> {
        use std::collections::HashMap as StdHashMap;

        let mut json_data = StdHashMap::new();
        json_data.insert("function_id", serde_json::json!(closure.function_id));
        json_data.insert("parameters", serde_json::json!(closure.parameters));
        json_data.insert(
            "captures_by_ref",
            serde_json::json!(closure.captures_by_ref),
        );

        if self.config.include_captured_values {
            let mut captures = StdHashMap::new();
            for (name, value) in &closure.captured_vars {
                captures.insert(name.clone(), self.value_to_json(value)?);
            }
            json_data.insert("captured_vars", serde_json::json!(captures));
        }

        let json_string = serde_json::to_string(&json_data).map_err(|e| {
            Error::new(
                ErrorKind::RuntimeError,
                format!("JSON serialization failed: {e}"),
            )
        })?;

        Ok(json_string.into_bytes())
    }

    /// JSON serialization for optimized closures
    fn serialize_optimized_closure_json(&self, closure: &OptimizedClosure) -> Result<Vec<u8>> {
        use std::collections::HashMap as StdHashMap;

        let mut json_data = StdHashMap::new();
        let function_name = closure
            .function_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("#{}", closure.function_id));

        json_data.insert("function_id", serde_json::json!(function_name));
        json_data.insert("parameters", serde_json::json!(closure.parameters.to_vec()));
        json_data.insert(
            "captures_by_ref",
            serde_json::json!(closure.captures_by_ref),
        );
        json_data.insert("is_optimized", serde_json::json!(true));

        if self.config.include_captured_values {
            let mut captures = StdHashMap::new();
            for (name, value) in closure.captured_vars.iter() {
                captures.insert(name.clone(), self.value_to_json(value)?);
            }
            json_data.insert("captured_vars", serde_json::json!(captures));
        }

        let json_string = serde_json::to_string(&json_data).map_err(|e| {
            Error::new(
                ErrorKind::RuntimeError,
                format!("JSON serialization failed: {e}"),
            )
        })?;

        Ok(json_string.into_bytes())
    }

    /// Compact serialization (custom format)
    fn serialize_closure_compact(&self, closure: &Closure) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();

        // Version and flags byte
        let mut flags = 0u8;
        if closure.captures_by_ref {
            flags |= 0x01;
        }
        if self.config.include_captured_values {
            flags |= 0x02;
        }
        buffer.push(flags);

        // Function ID (length-prefixed)
        let id_bytes = closure.function_id.as_bytes();
        buffer.push(id_bytes.len() as u8);
        buffer.extend_from_slice(id_bytes);

        // Parameters (count + length-prefixed strings)
        buffer.push(closure.parameters.len() as u8);
        for param in &closure.parameters {
            let param_bytes = param.as_bytes();
            buffer.push(param_bytes.len() as u8);
            buffer.extend_from_slice(param_bytes);
        }

        // Captured variables (if included)
        if self.config.include_captured_values {
            buffer.push(closure.captured_vars.len() as u8);
            for (name, value) in &closure.captured_vars {
                let name_bytes = name.as_bytes();
                buffer.push(name_bytes.len() as u8);
                buffer.extend_from_slice(name_bytes);
                self.serialize_value_compact(&mut buffer, value)?;
            }
        }

        Ok(buffer)
    }

    /// Compact serialization for optimized closures
    fn serialize_optimized_closure_compact(&self, closure: &OptimizedClosure) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();

        // Version and flags byte
        let mut flags = 0u8;
        if closure.captures_by_ref {
            flags |= 0x01;
        }
        if self.config.include_captured_values {
            flags |= 0x02;
        }
        flags |= 0x04; // Optimized flag
        buffer.push(flags);

        // Function ID (length-prefixed)
        let function_name = closure
            .function_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("#{}", closure.function_id));
        let id_bytes = function_name.as_bytes();
        buffer.push(id_bytes.len() as u8);
        buffer.extend_from_slice(id_bytes);

        // Parameters (count + length-prefixed strings)
        buffer.push(closure.parameters.len() as u8);
        for param in closure.parameters.iter() {
            let param_bytes = param.as_bytes();
            buffer.push(param_bytes.len() as u8);
            buffer.extend_from_slice(param_bytes);
        }

        // Captured variables (if included)
        if self.config.include_captured_values {
            buffer.push(closure.captured_vars.len() as u8);
            for (name, value) in closure.captured_vars.iter() {
                let name_bytes = name.as_bytes();
                buffer.push(name_bytes.len() as u8);
                buffer.extend_from_slice(name_bytes);
                self.serialize_value_compact(&mut buffer, value)?;
            }
        }

        Ok(buffer)
    }

    /// Deserialize closure from binary format
    fn deserialize_closure_binary(&self, data: &[u8]) -> Result<Closure> {
        let data = if self.config.compress {
            self.decompress_data(data)?
        } else {
            data.to_vec()
        };

        let mut cursor = 0;

        // Read function ID
        let function_id = self.read_string(&data, &mut cursor)?;

        // Read parameters
        let param_count = self.read_u32(&data, &mut cursor)? as usize;
        let mut parameters = Vec::with_capacity(param_count);
        for _ in 0..param_count {
            parameters.push(self.read_string(&data, &mut cursor)?);
        }

        // Read captures flag
        let captures_by_ref = self.read_bool(&data, &mut cursor)?;

        // Read captured variables
        let capture_count = self.read_u32(&data, &mut cursor)? as usize;
        let mut captured_vars = HashMap::new();
        for _ in 0..capture_count {
            let name = self.read_string(&data, &mut cursor)?;
            let value = self.deserialize_value_binary(&data, &mut cursor)?;
            captured_vars.insert(name, value);
        }

        if captures_by_ref {
            Ok(Closure::new_by_ref(function_id, parameters, captured_vars))
        } else {
            Ok(Closure::new(function_id, parameters, captured_vars))
        }
    }

    /// Deserialize optimized closure from binary format
    fn deserialize_optimized_closure_binary(&self, data: &[u8]) -> Result<OptimizedClosure> {
        let data = if self.config.compress {
            self.decompress_data(data)?
        } else {
            data.to_vec()
        };

        let mut cursor = 0;

        // Read function ID
        let function_id = self.read_string(&data, &mut cursor)?;

        // Read parameters
        let param_count = self.read_u32(&data, &mut cursor)? as usize;
        let mut parameters = Vec::with_capacity(param_count);
        for _ in 0..param_count {
            parameters.push(self.read_string(&data, &mut cursor)?);
        }

        // Read captures flag
        let captures_by_ref = self.read_bool(&data, &mut cursor)?;

        // Read captured variables
        let capture_count = self.read_u32(&data, &mut cursor)? as usize;
        let mut captured_vars = Vec::with_capacity(capture_count);
        for _ in 0..capture_count {
            let name = self.read_string(&data, &mut cursor)?;
            let value = self.deserialize_value_binary(&data, &mut cursor)?;
            captured_vars.push((name, value));
        }

        if captures_by_ref {
            Ok(OptimizedClosure::new_by_ref(
                function_id,
                parameters,
                captured_vars,
            ))
        } else {
            Ok(OptimizedClosure::new(
                function_id,
                parameters,
                captured_vars,
            ))
        }
    }

    /// Deserialize closure from JSON format
    fn deserialize_closure_json(&self, data: &[u8]) -> Result<Closure> {
        let json_str = std::str::from_utf8(data)
            .map_err(|e| Error::new(ErrorKind::RuntimeError, format!("Invalid UTF-8: {e}")))?;

        let json_data: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| Error::new(ErrorKind::RuntimeError, format!("JSON parse error: {e}")))?;

        let function_id = json_data["function_id"]
            .as_str()
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Missing function_id"))?
            .to_string();

        let parameters: Vec<String> = json_data["parameters"]
            .as_array()
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Invalid parameters"))?
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_string())
            .collect();

        let captures_by_ref = json_data["captures_by_ref"].as_bool().unwrap_or(false);

        let mut captured_vars = HashMap::new();
        if let Some(captures) = json_data["captured_vars"].as_object() {
            for (name, value) in captures {
                captured_vars.insert(name.clone(), self.json_to_value(value)?);
            }
        }

        if captures_by_ref {
            Ok(Closure::new_by_ref(function_id, parameters, captured_vars))
        } else {
            Ok(Closure::new(function_id, parameters, captured_vars))
        }
    }

    /// Deserialize optimized closure from JSON format
    fn deserialize_optimized_closure_json(&self, data: &[u8]) -> Result<OptimizedClosure> {
        let json_str = std::str::from_utf8(data)
            .map_err(|e| Error::new(ErrorKind::RuntimeError, format!("Invalid UTF-8: {e}")))?;

        let json_data: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| Error::new(ErrorKind::RuntimeError, format!("JSON parse error: {e}")))?;

        let function_id = json_data["function_id"]
            .as_str()
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Missing function_id"))?
            .to_string();

        let parameters: Vec<String> = json_data["parameters"]
            .as_array()
            .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Invalid parameters"))?
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_string())
            .collect();

        let captures_by_ref = json_data["captures_by_ref"].as_bool().unwrap_or(false);

        let mut captured_vars = Vec::new();
        if let Some(captures) = json_data["captured_vars"].as_object() {
            for (name, value) in captures {
                captured_vars.push((name.clone(), self.json_to_value(value)?));
            }
        }

        if captures_by_ref {
            Ok(OptimizedClosure::new_by_ref(
                function_id,
                parameters,
                captured_vars,
            ))
        } else {
            Ok(OptimizedClosure::new(
                function_id,
                parameters,
                captured_vars,
            ))
        }
    }

    /// Deserialize closure from compact format
    fn deserialize_closure_compact(&self, data: &[u8]) -> Result<Closure> {
        if data.is_empty() {
            return Err(Error::new(ErrorKind::RuntimeError, "Empty compact data"));
        }

        let mut cursor = 0;

        // Read flags
        let flags = data[cursor];
        cursor += 1;
        let captures_by_ref = (flags & 0x01) != 0;
        let has_captured_values = (flags & 0x02) != 0;

        // Read function ID
        let id_len = data[cursor] as usize;
        cursor += 1;
        if cursor + id_len > data.len() {
            return Err(Error::new(ErrorKind::RuntimeError, "Invalid compact data"));
        }
        let function_id = String::from_utf8(data[cursor..cursor + id_len].to_vec())
            .map_err(|e| Error::new(ErrorKind::RuntimeError, format!("Invalid UTF-8: {e}")))?;
        cursor += id_len;

        // Read parameters
        if cursor >= data.len() {
            return Err(Error::new(
                ErrorKind::RuntimeError,
                "Truncated compact data",
            ));
        }
        let param_count = data[cursor] as usize;
        cursor += 1;
        let mut parameters = Vec::with_capacity(param_count);
        for _ in 0..param_count {
            if cursor >= data.len() {
                return Err(Error::new(
                    ErrorKind::RuntimeError,
                    "Truncated parameter data",
                ));
            }
            let param_len = data[cursor] as usize;
            cursor += 1;
            if cursor + param_len > data.len() {
                return Err(Error::new(
                    ErrorKind::RuntimeError,
                    "Invalid parameter data",
                ));
            }
            let param =
                String::from_utf8(data[cursor..cursor + param_len].to_vec()).map_err(|e| {
                    Error::new(
                        ErrorKind::RuntimeError,
                        format!("Invalid parameter UTF-8: {e}"),
                    )
                })?;
            parameters.push(param);
            cursor += param_len;
        }

        // Read captured variables
        let mut captured_vars = HashMap::new();
        if has_captured_values {
            if cursor >= data.len() {
                return Err(Error::new(
                    ErrorKind::RuntimeError,
                    "Truncated capture data",
                ));
            }
            let capture_count = data[cursor] as usize;
            cursor += 1;
            for _ in 0..capture_count {
                if cursor >= data.len() {
                    return Err(Error::new(
                        ErrorKind::RuntimeError,
                        "Truncated capture name",
                    ));
                }
                let name_len = data[cursor] as usize;
                cursor += 1;
                if cursor + name_len > data.len() {
                    return Err(Error::new(ErrorKind::RuntimeError, "Invalid capture name"));
                }
                let name =
                    String::from_utf8(data[cursor..cursor + name_len].to_vec()).map_err(|e| {
                        Error::new(
                            ErrorKind::RuntimeError,
                            format!("Invalid capture name UTF-8: {e}"),
                        )
                    })?;
                cursor += name_len;
                let value = self.deserialize_value_compact(data, &mut cursor)?;
                captured_vars.insert(name, value);
            }
        }

        if captures_by_ref {
            Ok(Closure::new_by_ref(function_id, parameters, captured_vars))
        } else {
            Ok(Closure::new(function_id, parameters, captured_vars))
        }
    }

    /// Deserialize optimized closure from compact format
    fn deserialize_optimized_closure_compact(&self, data: &[u8]) -> Result<OptimizedClosure> {
        if data.is_empty() {
            return Err(Error::new(ErrorKind::RuntimeError, "Empty compact data"));
        }

        let mut cursor = 0;

        // Read flags
        let flags = data[cursor];
        cursor += 1;
        let captures_by_ref = (flags & 0x01) != 0;
        let has_captured_values = (flags & 0x02) != 0;
        let _is_optimized = (flags & 0x04) != 0;

        // Read function ID
        let id_len = data[cursor] as usize;
        cursor += 1;
        if cursor + id_len > data.len() {
            return Err(Error::new(ErrorKind::RuntimeError, "Invalid compact data"));
        }
        let function_id = String::from_utf8(data[cursor..cursor + id_len].to_vec())
            .map_err(|e| Error::new(ErrorKind::RuntimeError, format!("Invalid UTF-8: {e}")))?;
        cursor += id_len;

        // Read parameters
        if cursor >= data.len() {
            return Err(Error::new(
                ErrorKind::RuntimeError,
                "Truncated compact data",
            ));
        }
        let param_count = data[cursor] as usize;
        cursor += 1;
        let mut parameters = Vec::with_capacity(param_count);
        for _ in 0..param_count {
            if cursor >= data.len() {
                return Err(Error::new(
                    ErrorKind::RuntimeError,
                    "Truncated parameter data",
                ));
            }
            let param_len = data[cursor] as usize;
            cursor += 1;
            if cursor + param_len > data.len() {
                return Err(Error::new(
                    ErrorKind::RuntimeError,
                    "Invalid parameter data",
                ));
            }
            let param =
                String::from_utf8(data[cursor..cursor + param_len].to_vec()).map_err(|e| {
                    Error::new(
                        ErrorKind::RuntimeError,
                        format!("Invalid parameter UTF-8: {e}"),
                    )
                })?;
            parameters.push(param);
            cursor += param_len;
        }

        // Read captured variables
        let mut captured_vars = Vec::new();
        if has_captured_values {
            if cursor >= data.len() {
                return Err(Error::new(
                    ErrorKind::RuntimeError,
                    "Truncated capture data",
                ));
            }
            let capture_count = data[cursor] as usize;
            cursor += 1;
            for _ in 0..capture_count {
                if cursor >= data.len() {
                    return Err(Error::new(
                        ErrorKind::RuntimeError,
                        "Truncated capture name",
                    ));
                }
                let name_len = data[cursor] as usize;
                cursor += 1;
                if cursor + name_len > data.len() {
                    return Err(Error::new(ErrorKind::RuntimeError, "Invalid capture name"));
                }
                let name =
                    String::from_utf8(data[cursor..cursor + name_len].to_vec()).map_err(|e| {
                        Error::new(
                            ErrorKind::RuntimeError,
                            format!("Invalid capture name UTF-8: {e}"),
                        )
                    })?;
                cursor += name_len;
                let value = self.deserialize_value_compact(data, &mut cursor)?;
                captured_vars.push((name, value));
            }
        }

        if captures_by_ref {
            Ok(OptimizedClosure::new_by_ref(
                function_id,
                parameters,
                captured_vars,
            ))
        } else {
            Ok(OptimizedClosure::new(
                function_id,
                parameters,
                captured_vars,
            ))
        }
    }

    // Helper methods for binary serialization
    fn write_string(&self, buffer: &mut Vec<u8>, s: &str) -> Result<()> {
        let bytes = s.as_bytes();
        self.write_u32(buffer, bytes.len() as u32)?;
        buffer.extend_from_slice(bytes);
        Ok(())
    }

    fn write_u32(&self, buffer: &mut Vec<u8>, value: u32) -> Result<()> {
        buffer.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_bool(&self, buffer: &mut Vec<u8>, value: bool) -> Result<()> {
        buffer.push(if value { 1 } else { 0 });
        Ok(())
    }

    fn read_string(&self, data: &[u8], cursor: &mut usize) -> Result<String> {
        let len = self.read_u32(data, cursor)? as usize;
        if *cursor + len > data.len() {
            return Err(Error::new(ErrorKind::RuntimeError, "Invalid string length"));
        }
        let bytes = &data[*cursor..*cursor + len];
        *cursor += len;
        String::from_utf8(bytes.to_vec())
            .map_err(|e| Error::new(ErrorKind::RuntimeError, format!("Invalid UTF-8: {e}")))
    }

    fn read_u32(&self, data: &[u8], cursor: &mut usize) -> Result<u32> {
        if *cursor + 4 > data.len() {
            return Err(Error::new(
                ErrorKind::RuntimeError,
                "Not enough data for u32",
            ));
        }
        let bytes = [
            data[*cursor],
            data[*cursor + 1],
            data[*cursor + 2],
            data[*cursor + 3],
        ];
        *cursor += 4;
        Ok(u32::from_le_bytes(bytes))
    }

    fn read_bool(&self, data: &[u8], cursor: &mut usize) -> Result<bool> {
        if *cursor >= data.len() {
            return Err(Error::new(
                ErrorKind::RuntimeError,
                "Not enough data for bool",
            ));
        }
        let value = data[*cursor] != 0;
        *cursor += 1;
        Ok(value)
    }

    // Simplified value serialization (would need full implementation based on Value enum)
    fn serialize_value_binary(&self, buffer: &mut Vec<u8>, value: &Value) -> Result<()> {
        match value {
            Value::Null => buffer.push(0),
            Value::Bool(b) => {
                buffer.push(1);
                buffer.push(if *b { 1 } else { 0 });
            }
            Value::I32(i) => {
                buffer.push(2);
                buffer.extend_from_slice(&i.to_le_bytes());
            }
            Value::F32(f) => {
                buffer.push(3);
                buffer.extend_from_slice(&f.to_le_bytes());
            }
            Value::String(s) => {
                buffer.push(4);
                self.write_string(buffer, s)?;
            }
            _ => {
                // For complex values, serialize as JSON string
                buffer.push(255);
                let json_str = format!("{:?}", value); // Simplified
                self.write_string(buffer, &json_str)?;
            }
        }
        Ok(())
    }

    fn serialize_value_compact(&self, buffer: &mut Vec<u8>, value: &Value) -> Result<()> {
        // Simplified compact value serialization
        match value {
            Value::Null => buffer.push(0),
            Value::Bool(b) => {
                buffer.push(1);
                buffer.push(if *b { 1 } else { 0 });
            }
            Value::I32(i) => {
                buffer.push(2);
                buffer.extend_from_slice(&i.to_le_bytes());
            }
            Value::F32(f) => {
                buffer.push(3);
                buffer.extend_from_slice(&f.to_le_bytes());
            }
            Value::String(s) => {
                buffer.push(4);
                let bytes = s.as_bytes();
                buffer.push(bytes.len() as u8);
                buffer.extend_from_slice(bytes);
            }
            _ => {
                buffer.push(255); // Complex type marker
            }
        }
        Ok(())
    }

    fn deserialize_value_binary(&self, data: &[u8], cursor: &mut usize) -> Result<Value> {
        if *cursor >= data.len() {
            return Err(Error::new(
                ErrorKind::RuntimeError,
                "Not enough data for value type",
            ));
        }

        let value_type = data[*cursor];
        *cursor += 1;

        match value_type {
            0 => Ok(Value::Null),
            1 => {
                let b = self.read_bool(data, cursor)?;
                Ok(Value::Bool(b))
            }
            2 => {
                if *cursor + 4 > data.len() {
                    return Err(Error::new(
                        ErrorKind::RuntimeError,
                        "Not enough data for i32",
                    ));
                }
                let bytes = [
                    data[*cursor],
                    data[*cursor + 1],
                    data[*cursor + 2],
                    data[*cursor + 3],
                ];
                *cursor += 4;
                Ok(Value::I32(i32::from_le_bytes(bytes)))
            }
            3 => {
                if *cursor + 4 > data.len() {
                    return Err(Error::new(
                        ErrorKind::RuntimeError,
                        "Not enough data for f32",
                    ));
                }
                let bytes = [
                    data[*cursor],
                    data[*cursor + 1],
                    data[*cursor + 2],
                    data[*cursor + 3],
                ];
                *cursor += 4;
                Ok(Value::F32(f32::from_le_bytes(bytes)))
            }
            4 => {
                let s = self.read_string(data, cursor)?;
                Ok(Value::String(s))
            }
            255 => {
                // Complex value as JSON string
                let _json_str = self.read_string(data, cursor)?;
                // For now, return null for complex values
                Ok(Value::Null)
            }
            _ => Err(Error::new(ErrorKind::RuntimeError, "Unknown value type")),
        }
    }

    fn deserialize_value_compact(&self, data: &[u8], cursor: &mut usize) -> Result<Value> {
        if *cursor >= data.len() {
            return Err(Error::new(
                ErrorKind::RuntimeError,
                "Not enough data for value type",
            ));
        }

        let value_type = data[*cursor];
        *cursor += 1;

        match value_type {
            0 => Ok(Value::Null),
            1 => {
                if *cursor >= data.len() {
                    return Err(Error::new(
                        ErrorKind::RuntimeError,
                        "Not enough data for bool",
                    ));
                }
                let b = data[*cursor] != 0;
                *cursor += 1;
                Ok(Value::Bool(b))
            }
            2 => {
                if *cursor + 4 > data.len() {
                    return Err(Error::new(
                        ErrorKind::RuntimeError,
                        "Not enough data for i32",
                    ));
                }
                let bytes = [
                    data[*cursor],
                    data[*cursor + 1],
                    data[*cursor + 2],
                    data[*cursor + 3],
                ];
                *cursor += 4;
                Ok(Value::I32(i32::from_le_bytes(bytes)))
            }
            3 => {
                if *cursor + 4 > data.len() {
                    return Err(Error::new(
                        ErrorKind::RuntimeError,
                        "Not enough data for f32",
                    ));
                }
                let bytes = [
                    data[*cursor],
                    data[*cursor + 1],
                    data[*cursor + 2],
                    data[*cursor + 3],
                ];
                *cursor += 4;
                Ok(Value::F32(f32::from_le_bytes(bytes)))
            }
            4 => {
                if *cursor >= data.len() {
                    return Err(Error::new(
                        ErrorKind::RuntimeError,
                        "Not enough data for string length",
                    ));
                }
                let len = data[*cursor] as usize;
                *cursor += 1;
                if *cursor + len > data.len() {
                    return Err(Error::new(
                        ErrorKind::RuntimeError,
                        "Not enough data for string",
                    ));
                }
                let s = String::from_utf8(data[*cursor..*cursor + len].to_vec()).map_err(|e| {
                    Error::new(ErrorKind::RuntimeError, format!("Invalid UTF-8: {e}"))
                })?;
                *cursor += len;
                Ok(Value::String(s))
            }
            255 => {
                // Complex value - return null for now
                Ok(Value::Null)
            }
            _ => Err(Error::new(ErrorKind::RuntimeError, "Unknown value type")),
        }
    }

    fn value_to_json(&self, value: &Value) -> Result<serde_json::Value> {
        match value {
            Value::Null => Ok(serde_json::Value::Null),
            Value::Bool(b) => Ok(serde_json::Value::Bool(*b)),
            Value::I32(i) => Ok(serde_json::Value::Number((*i).into())),
            Value::F32(f) => Ok(serde_json::Number::from_f64(*f as f64)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)),
            Value::String(s) => Ok(serde_json::Value::String(s.clone())),
            _ => Ok(serde_json::Value::String(format!("{:?}", value))),
        }
    }

    fn json_to_value(&self, json: &serde_json::Value) -> Result<Value> {
        match json {
            serde_json::Value::Null => Ok(Value::Null),
            serde_json::Value::Bool(b) => Ok(Value::Bool(*b)),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(Value::I32(i as i32))
                } else if let Some(f) = n.as_f64() {
                    Ok(Value::F32(f as f32))
                } else {
                    Ok(Value::Null)
                }
            }
            serde_json::Value::String(s) => Ok(Value::String(s.clone())),
            _ => Ok(Value::Null),
        }
    }

    fn compress_data(&self, data: Vec<u8>) -> Result<Vec<u8>> {
        // Simplified compression - in practice would use a real compression library
        Ok(data)
    }

    fn decompress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Simplified decompression - in practice would use a real compression library
        Ok(data.to_vec())
    }

    fn validate_serialized_data(&self, serialized: &SerializedClosure) -> Result<()> {
        if serialized.data.is_empty() {
            return Err(Error::new(ErrorKind::RuntimeError, "Empty serialized data"));
        }

        if serialized.metadata.version > 1 {
            return Err(Error::new(
                ErrorKind::RuntimeError,
                format!(
                    "Unsupported serialization version: {}",
                    serialized.metadata.version
                ),
            ));
        }

        Ok(())
    }

    fn current_timestamp(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

/// Convenience functions for quick serialization/deserialization
pub fn serialize_closure_binary(closure: &Closure) -> Result<SerializedClosure> {
    let serializer = ClosureSerializer::new(SerializationFormat::Binary);
    serializer.serialize_closure(closure)
}

pub fn serialize_closure_json(closure: &Closure) -> Result<SerializedClosure> {
    let serializer = ClosureSerializer::new(SerializationFormat::Json);
    serializer.serialize_closure(closure)
}

pub fn serialize_closure_compact(closure: &Closure) -> Result<SerializedClosure> {
    let serializer = ClosureSerializer::new(SerializationFormat::Compact);
    serializer.serialize_closure(closure)
}

pub fn deserialize_closure(serialized: &SerializedClosure) -> Result<Closure> {
    let serializer = ClosureSerializer::new(serialized.format.clone());
    serializer.deserialize_closure(serialized)
}

pub fn serialize_optimized_closure_binary(closure: &OptimizedClosure) -> Result<SerializedClosure> {
    let serializer = ClosureSerializer::new(SerializationFormat::Binary);
    serializer.serialize_optimized_closure(closure)
}

pub fn deserialize_optimized_closure(serialized: &SerializedClosure) -> Result<OptimizedClosure> {
    let serializer = ClosureSerializer::new(serialized.format.clone());
    serializer.deserialize_optimized_closure(serialized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_closure_binary_serialization() {
        let mut captured = HashMap::new();
        captured.insert("x".to_string(), Value::I32(42));
        captured.insert("y".to_string(), Value::String("hello".to_string()));

        let closure = Closure::new(
            "test_function".to_string(),
            vec!["param1".to_string(), "param2".to_string()],
            captured,
        );

        let serializer = ClosureSerializer::new(SerializationFormat::Binary);
        let serialized = serializer.serialize_closure(&closure).unwrap();
        let deserialized = serializer.deserialize_closure(&serialized).unwrap();

        assert_eq!(closure.function_id, deserialized.function_id);
        assert_eq!(closure.parameters, deserialized.parameters);
        assert_eq!(closure.captures_by_ref, deserialized.captures_by_ref);
        assert_eq!(
            closure.captured_vars.len(),
            deserialized.captured_vars.len()
        );
    }

    #[test]
    fn test_closure_json_serialization() {
        let mut captured = HashMap::new();
        captured.insert("flag".to_string(), Value::Bool(true));

        let closure = Closure::new_by_ref("json_test".to_string(), vec!["a".to_string()], captured);

        let serializer = ClosureSerializer::new(SerializationFormat::Json);
        let serialized = serializer.serialize_closure(&closure).unwrap();
        let deserialized = serializer.deserialize_closure(&serialized).unwrap();

        assert_eq!(closure.function_id, deserialized.function_id);
        assert_eq!(closure.captures_by_ref, deserialized.captures_by_ref);
    }

    #[test]
    fn test_closure_compact_serialization() {
        let closure = Closure::new(
            "compact_test".to_string(),
            vec!["x".to_string(), "y".to_string()],
            HashMap::new(),
        );

        let serializer = ClosureSerializer::new(SerializationFormat::Compact);
        let serialized = serializer.serialize_closure(&closure).unwrap();
        let deserialized = serializer.deserialize_closure(&serialized).unwrap();

        assert_eq!(closure.function_id, deserialized.function_id);
        assert_eq!(closure.parameters, deserialized.parameters);
    }

    #[test]
    fn test_optimized_closure_serialization() {
        let captured_vars = vec![
            ("x".to_string(), Value::I32(10)),
            ("y".to_string(), Value::F32(3.14)),
        ];

        let closure = OptimizedClosure::new(
            "optimized_test".to_string(),
            vec!["param".to_string()],
            captured_vars,
        );

        let serializer = ClosureSerializer::new(SerializationFormat::Binary);
        let serialized = serializer.serialize_optimized_closure(&closure).unwrap();
        let deserialized = serializer
            .deserialize_optimized_closure(&serialized)
            .unwrap();

        assert_eq!(closure.function_id(), deserialized.function_id());
        assert_eq!(closure.param_count(), deserialized.param_count());
        assert_eq!(closure.capture_count(), deserialized.capture_count());
    }

    #[test]
    fn test_serialization_metadata() {
        let closure = Closure::new(
            "metadata_test".to_string(),
            vec!["a".to_string(), "b".to_string()],
            HashMap::new(),
        );

        let serializer = ClosureSerializer::new(SerializationFormat::Binary);
        let serialized = serializer.serialize_closure(&closure).unwrap();

        assert_eq!(serialized.metadata.function_id, "metadata_test");
        assert_eq!(serialized.metadata.param_count, 2);
        assert_eq!(serialized.metadata.capture_count, 0);
        assert!(!serialized.metadata.captures_by_ref);
        assert!(!serialized.metadata.is_optimized);
        assert_eq!(serialized.metadata.version, 1);
    }

    #[test]
    fn test_serialization_size_limit() {
        let config = SerializationConfig {
            max_size_bytes: 10, // Very small limit
            ..Default::default()
        };

        let mut large_captured = HashMap::new();
        for i in 0..100 {
            large_captured.insert(format!("var_{i}"), Value::I32(i));
        }

        let closure = Closure::new(
            "large_closure".to_string(),
            vec!["param".to_string()],
            large_captured,
        );

        let serializer = ClosureSerializer::with_config(SerializationFormat::Binary, config);
        let result = serializer.serialize_closure(&closure);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("exceeds size limit"));
    }
}
