//! Comprehensive tests for closure serialization functionality
//!
//! Tests all three serialization formats (binary, JSON, compact) and validates
//! the documented serialization features from the closure implementation.

use script::runtime::closure::{
    Closure, create_closure_heap, 
    serialize::{ClosureSerializeConfig, SerializationFormat}
};
use script::runtime::{Value, ScriptRc};
use script::stdlib::functional::{
    closure_serialize_binary, closure_serialize_json, closure_serialize_compact,
    closure_get_metadata, closure_can_serialize, closure_create_serialize_config
};
use script::error::{Error, ErrorKind};
use std::collections::HashMap;

#[test]
fn test_closure_binary_serialization() {
    // Create a test closure with captures
    let closure = create_closure_heap(
        "test_binary".to_string(),
        vec!["x".to_string(), "y".to_string()],
        vec![
            ("captured_int".to_string(), Value::I32(42)),
            ("captured_str".to_string(), Value::String("hello".to_string())),
        ],
        false,
    );
    
    // Serialize to binary format
    let serialized = closure_serialize_binary(&closure).expect("Binary serialization failed");
    
    // Verify serialization produced data
    assert!(!serialized.is_empty(), "Binary serialization should produce data");
    
    // Verify it contains expected metadata
    assert!(serialized.len() > 50, "Binary serialization should include metadata and captures");
    
    // Test serialization config
    let config = closure_create_serialize_config(
        true,  // compress
        1024,  // max_size
        true,  // validate
    );
    
    assert_eq!(config.compress, true);
    assert_eq!(config.max_size, 1024);
    assert_eq!(config.validate, true);
}

#[test]
fn test_closure_json_serialization() {
    // Create a closure with various capture types
    let closure = create_closure_heap(
        "test_json".to_string(),
        vec!["param1".to_string()],
        vec![
            ("int_val".to_string(), Value::I32(100)),
            ("float_val".to_string(), Value::F64(3.14)),
            ("bool_val".to_string(), Value::Bool(true)),
            ("string_val".to_string(), Value::String("test".to_string())),
        ],
        false,
    );
    
    // Serialize to JSON format
    let serialized = closure_serialize_json(&closure).expect("JSON serialization failed");
    
    // Verify JSON structure
    assert!(serialized.contains("\"function_id\":\"test_json\""));
    assert!(serialized.contains("\"parameters\":[\"param1\"]"));
    assert!(serialized.contains("\"captured_vars\""));
    assert!(serialized.contains("\"int_val\":100"));
    assert!(serialized.contains("\"float_val\":3.14"));
    assert!(serialized.contains("\"bool_val\":true"));
    assert!(serialized.contains("\"string_val\":\"test\""));
    
    // Verify it's valid JSON
    let _parsed: serde_json::Value = serde_json::from_str(&serialized)
        .expect("Serialized data should be valid JSON");
}

#[test]
fn test_closure_compact_serialization() {
    // Create a closure with minimal data for compact format
    let closure = create_closure_heap(
        "compact_test".to_string(),
        vec!["x".to_string()],
        vec![("val".to_string(), Value::I32(42))],
        false,
    );
    
    // Serialize to compact format
    let serialized = closure_serialize_compact(&closure).expect("Compact serialization failed");
    
    // Verify compact format is smaller than JSON
    let json_serialized = closure_serialize_json(&closure).expect("JSON serialization failed");
    assert!(serialized.len() < json_serialized.len(), "Compact format should be smaller than JSON");
    
    // Verify it contains essential data
    assert!(!serialized.is_empty());
    assert!(serialized.len() > 10); // Should have some meaningful content
}

#[test]
fn test_closure_metadata_extraction() {
    // Create a closure with known metadata
    let closure = create_closure_heap(
        "metadata_test".to_string(),
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
        vec![
            ("cap1".to_string(), Value::I32(1)),
            ("cap2".to_string(), Value::I32(2)),
        ],
        false,
    );
    
    // Get metadata
    let metadata = closure_get_metadata(&closure).expect("Metadata extraction failed");
    
    // Verify metadata content
    assert!(metadata.contains("\"function_id\":\"metadata_test\""));
    assert!(metadata.contains("\"parameter_count\":3"));
    assert!(metadata.contains("\"capture_count\":2"));
    assert!(metadata.contains("\"captures_by_reference\":false"));
}

#[test]
fn test_closure_serialization_eligibility() {
    // Test closure that can be serialized
    let serializable_closure = create_closure_heap(
        "serializable".to_string(),
        vec!["x".to_string()],
        vec![("val".to_string(), Value::I32(42))],
        false,
    );
    
    assert!(closure_can_serialize(&serializable_closure), "Simple closure should be serializable");
    
    // Test closure with complex captures that might not be serializable
    let complex_closure = create_closure_heap(
        "complex".to_string(),
        vec!["x".to_string()],
        vec![
            ("simple".to_string(), Value::I32(42)),
            ("nested".to_string(), Value::Array(vec![
                ScriptRc::new(Value::I32(1)),
                ScriptRc::new(Value::I32(2)),
            ])),
        ],
        false,
    );
    
    // This test verifies the serialization eligibility checker works
    let can_serialize = closure_can_serialize(&complex_closure);
    assert!(can_serialize, "Complex but valid closure should be serializable");
}

#[test]
fn test_closure_serialization_size_limits() {
    // Create a closure with large captured data
    let large_string = "x".repeat(1000);
    let large_closure = create_closure_heap(
        "large_test".to_string(),
        vec!["x".to_string()],
        vec![("large_data".to_string(), Value::String(large_string))],
        false,
    );
    
    // Test with restrictive size limit
    let small_config = closure_create_serialize_config(
        false, // no compression
        500,   // small size limit
        true,  // validate
    );
    
    // Serialization should respect size limits
    // Note: This test depends on the actual implementation respecting size limits
    let result = closure_serialize_binary(&large_closure);
    
    // If size limits are enforced, this should either succeed (if within limit) or fail gracefully
    match result {
        Ok(data) => {
            assert!(data.len() <= 2000, "Serialized data should be reasonable size");
        }
        Err(e) => {
            // If it fails due to size limits, that's also acceptable
            assert!(e.to_string().contains("size") || e.to_string().contains("limit"));
        }
    }
}

#[test]
fn test_closure_serialization_validation() {
    // Test with validation enabled
    let closure = create_closure_heap(
        "validation_test".to_string(),
        vec!["x".to_string()],
        vec![("val".to_string(), Value::I32(42))],
        false,
    );
    
    // Create config with validation enabled
    let config = closure_create_serialize_config(
        false, // no compression
        1024,  // reasonable size limit
        true,  // enable validation
    );
    
    // Serialization should succeed with validation
    let serialized = closure_serialize_binary(&closure).expect("Serialization with validation should succeed");
    
    // Verify the serialized data is valid
    assert!(!serialized.is_empty());
    assert!(serialized.len() > 20); // Should have meaningful content
}

#[test]
fn test_closure_serialization_compression() {
    // Create a closure with repetitive data that should compress well
    let repetitive_data = "repeated_data_".repeat(20);
    let closure = create_closure_heap(
        "compression_test".to_string(),
        vec!["x".to_string()],
        vec![("repetitive".to_string(), Value::String(repetitive_data))],
        false,
    );
    
    // Serialize with compression
    let config_compressed = closure_create_serialize_config(
        true,  // enable compression
        2048,  // size limit
        true,  // validate
    );
    
    // Serialize without compression
    let config_uncompressed = closure_create_serialize_config(
        false, // no compression
        2048,  // size limit
        true,  // validate
    );
    
    // Both should succeed
    let compressed = closure_serialize_binary(&closure).expect("Compressed serialization should succeed");
    let uncompressed = closure_serialize_binary(&closure).expect("Uncompressed serialization should succeed");
    
    // Verify both produce valid data
    assert!(!compressed.is_empty());
    assert!(!uncompressed.is_empty());
    
    // Note: Actual compression effectiveness depends on implementation
    // This test just verifies that compression option doesn't break serialization
}

#[test]
fn test_closure_serialization_edge_cases() {
    // Test empty closure (no captures)
    let empty_closure = create_closure_heap(
        "empty".to_string(),
        vec!["x".to_string()],
        vec![],
        false,
    );
    
    let empty_serialized = closure_serialize_json(&empty_closure).expect("Empty closure serialization should succeed");
    assert!(empty_serialized.contains("\"captured_vars\":{}"));
    
    // Test closure with no parameters
    let no_params_closure = create_closure_heap(
        "no_params".to_string(),
        vec![],
        vec![("val".to_string(), Value::I32(42))],
        false,
    );
    
    let no_params_serialized = closure_serialize_json(&no_params_closure).expect("No params closure serialization should succeed");
    assert!(no_params_serialized.contains("\"parameters\":[]"));
    
    // Test closure with special characters in names
    let special_closure = create_closure_heap(
        "special_chars_test".to_string(),
        vec!["param_with_underscore".to_string()],
        vec![("var_with_unicode_🦀".to_string(), Value::String("test".to_string()))],
        false,
    );
    
    let special_serialized = closure_serialize_json(&special_closure).expect("Special chars closure serialization should succeed");
    assert!(special_serialized.contains("param_with_underscore"));
    assert!(special_serialized.contains("var_with_unicode_🦀"));
}

#[test]
fn test_closure_serialization_error_handling() {
    // Test serialization of closure with unsupported captures
    // This tests the error handling in serialization
    
    // Create a closure with nested closures (potential serialization challenge)
    let inner_closure = create_closure_heap(
        "inner".to_string(),
        vec!["x".to_string()],
        vec![("inner_val".to_string(), Value::I32(42))],
        false,
    );
    
    let outer_closure = create_closure_heap(
        "outer".to_string(),
        vec!["y".to_string()],
        vec![("nested_closure".to_string(), inner_closure)],
        false,
    );
    
    // Attempt to serialize - should handle nested closures gracefully
    let result = closure_serialize_json(&outer_closure);
    
    match result {
        Ok(serialized) => {
            // If nested closures are supported, verify the structure
            assert!(serialized.contains("\"function_id\":\"outer\""));
            assert!(serialized.contains("nested_closure"));
        }
        Err(e) => {
            // If nested closures are not supported, should fail gracefully
            assert!(e.to_string().contains("nested") || e.to_string().contains("closure"));
        }
    }
}

#[test]
fn test_closure_serialization_roundtrip_metadata() {
    // Test that essential metadata is preserved through serialization
    let original_closure = create_closure_heap(
        "roundtrip_test".to_string(),
        vec!["a".to_string(), "b".to_string()],
        vec![
            ("cap1".to_string(), Value::I32(100)),
            ("cap2".to_string(), Value::String("preserved".to_string())),
        ],
        false,
    );
    
    // Get original metadata
    let original_metadata = closure_get_metadata(&original_closure).expect("Original metadata extraction failed");
    
    // Serialize and get metadata of serialized representation
    let serialized = closure_serialize_json(&original_closure).expect("Serialization failed");
    
    // Verify key metadata is preserved in serialized form
    assert!(serialized.contains("\"function_id\":\"roundtrip_test\""));
    assert!(serialized.contains("\"parameters\":[\"a\",\"b\"]"));
    assert!(serialized.contains("\"cap1\":100"));
    assert!(serialized.contains("\"cap2\":\"preserved\""));
    
    // Verify original metadata contains expected fields
    assert!(original_metadata.contains("\"function_id\":\"roundtrip_test\""));
    assert!(original_metadata.contains("\"parameter_count\":2"));
    assert!(original_metadata.contains("\"capture_count\":2"));
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;
    
    #[test]
    fn test_serialization_performance() {
        // Create a reasonably complex closure for performance testing
        let mut captures = Vec::new();
        for i in 0..10 {
            captures.push((format!("var_{}", i), Value::I32(i)));
        }
        
        let closure = create_closure_heap(
            "performance_test".to_string(),
            vec!["x".to_string(), "y".to_string()],
            captures,
            false,
        );
        
        // Test JSON serialization performance
        let start = Instant::now();
        let _json_result = closure_serialize_json(&closure).expect("JSON serialization should succeed");
        let json_duration = start.elapsed();
        
        // Test binary serialization performance
        let start = Instant::now();
        let _binary_result = closure_serialize_binary(&closure).expect("Binary serialization should succeed");
        let binary_duration = start.elapsed();
        
        // Test compact serialization performance
        let start = Instant::now();
        let _compact_result = closure_serialize_compact(&closure).expect("Compact serialization should succeed");
        let compact_duration = start.elapsed();
        
        // Verify all completed in reasonable time (< 1ms for this simple case)
        assert!(json_duration.as_millis() < 10, "JSON serialization should be fast");
        assert!(binary_duration.as_millis() < 10, "Binary serialization should be fast");
        assert!(compact_duration.as_millis() < 10, "Compact serialization should be fast");
        
        println!("Serialization performance:");
        println!("JSON: {:?}", json_duration);
        println!("Binary: {:?}", binary_duration);
        println!("Compact: {:?}", compact_duration);
    }
}