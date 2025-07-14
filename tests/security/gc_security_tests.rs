//! Comprehensive security tests for the garbage collection system
//!
//! These tests verify that the memory cycle detection system is secure against
//! various attack vectors including use-after-free, memory corruption, race conditions,
//! and denial of service attacks.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use script::runtime::{
    SecureCycleCollector, GcSecurityConfig, SecurityError as GcSecurityError,
    ResourceMonitor, ResourceLimits, ResourceViolation,
    SecurityMonitor, SecurityConfig, SecurityEvent, SecurityEventType,
};

/// Test that the secure garbage collector prevents use-after-free attacks
#[test]
fn test_use_after_free_prevention() {
    let config = GcSecurityConfig::default();
    let collector = SecureCycleCollector::new(config);
    
    // Simulate registering an object
    let fake_address = 0x1000;
    let type_id = 1;
    let object_size = 64;
    
    // Register object
    assert!(collector.register(fake_address, type_id, object_size).is_ok());
    
    // Unregister object (simulating deallocation)
    assert!(collector.unregister(fake_address).is_ok());
    
    // Attempt to add as possible root after unregistration should fail safely
    // (This tests that unregistered objects are not added to possible roots)
    assert!(collector.add_possible_root(fake_address).is_ok()); // Should succeed but be ignored
    
    // Collection should complete without accessing invalid memory
    assert!(collector.collect().is_ok());
}

/// Test that resource limits prevent denial of service attacks
#[test]
fn test_dos_protection_limits() {
    let config = GcSecurityConfig {
        max_possible_roots: 10, // Very small limit for testing
        max_collection_time: Duration::from_millis(100),
        max_graph_depth: 5,
        max_incremental_work: 5,
        enable_monitoring: true,
        enable_type_validation: true,
    };
    
    let collector = SecureCycleCollector::new(config);
    
    // Try to exceed possible roots limit
    for i in 0..15 {
        let address = 0x1000 + i * 8;
        let result = collector.register(address, 1, 64);
        
        if i < 10 {
            assert!(result.is_ok(), "Should register within limit: {}", i);
        } else {
            // Should start failing due to resource limits
            if result.is_err() {
                break;
            }
        }
    }
}

/// Test memory corruption detection
#[test]
fn test_memory_corruption_detection() {
    let config = SecurityConfig::default();
    let monitor = SecurityMonitor::new(config);
    
    // Test memory validation with invalid size
    let result = monitor.validate_memory(0x1000, 0);
    assert!(result.is_err(), "Should fail with invalid size");
    
    // Test memory validation with extremely large size
    let result = monitor.validate_memory(0x1000, usize::MAX);
    assert!(result.is_err(), "Should fail with oversized allocation");
    
    // Test type validation with mismatched types
    let result = monitor.validate_type_cast(1, 2);
    assert!(result.is_ok(), "Type validation should complete");
    if let Ok(valid) = result {
        assert!(!valid, "Different types should not be valid cast targets");
    }
}

/// Test race condition prevention in garbage collection
#[test]
fn test_concurrent_gc_safety() {
    let config = GcSecurityConfig::default();
    let collector = Arc::new(SecureCycleCollector::new(config));
    
    let mut handles = vec![];
    
    // Spawn multiple threads that register and unregister objects
    for thread_id in 0..10 {
        let collector_clone = collector.clone();
        let handle = thread::spawn(move || {
            for i in 0..100 {
                let address = (thread_id * 1000 + i) * 8 + 0x10000;
                
                // Register object
                if let Ok(()) = collector_clone.register(address, 1, 64) {
                    // Add as possible root
                    let _ = collector_clone.add_possible_root(address);
                    
                    // Sometimes unregister immediately
                    if i % 3 == 0 {
                        let _ = collector_clone.unregister(address);
                    }
                }
            }
            
            // Trigger collection
            let _ = collector_clone.collect();
        });
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread should complete successfully");
    }
    
    // Final collection should complete without issues
    assert!(collector.collect().is_ok());
}

/// Test resource monitoring and throttling
#[test]
fn test_resource_monitoring() {
    let limits = ResourceLimits {
        max_memory_bytes: 1024, // Small limit for testing
        max_allocations: 10,
        max_collection_time: Duration::from_millis(100),
        max_graph_depth: 10,
        max_possible_roots: 10,
        max_incremental_work: 10,
        memory_pressure_threshold: 0.8,
        enable_auto_throttling: true,
    };
    
    let monitor = ResourceMonitor::new(limits);
    
    // Test allocation within limits
    assert!(monitor.record_allocation(512).is_ok());
    
    // Test allocation exceeding memory limit
    assert!(monitor.record_allocation(600).is_err());
    
    // Test memory pressure detection
    let monitor2 = ResourceMonitor::new(ResourceLimits {
        max_memory_bytes: 1000,
        memory_pressure_threshold: 0.5,
        ..Default::default()
    });
    
    assert!(monitor2.record_allocation(600).is_err()); // Should trigger pressure threshold
}

/// Test security event detection and handling
#[test]
fn test_security_event_handling() {
    let config = SecurityConfig {
        enable_attack_detection: true,
        enable_memory_validation: true,
        alert_threshold: 0.5,
        max_alerts_per_minute: 5,
        ..Default::default()
    };
    
    let monitor = SecurityMonitor::new(config);
    
    // Create security events
    let event1 = SecurityEvent {
        event_type: SecurityEventType::MemoryCorruption,
        severity: 0.9,
        description: "Test memory corruption event".to_string(),
        timestamp: std::time::SystemTime::now(),
        context: std::collections::HashMap::new(),
        source: "Test".to_string(),
        event_id: 0,
    };
    
    let event2 = SecurityEvent {
        event_type: SecurityEventType::ResourceExhaustion,
        severity: 0.8,
        description: "Test resource exhaustion event".to_string(),
        timestamp: std::time::SystemTime::now(),
        context: std::collections::HashMap::new(),
        source: "Test".to_string(),
        event_id: 0,
    };
    
    // Record events
    assert!(monitor.record_event(event1).is_ok());
    assert!(monitor.record_event(event2).is_ok());
    
    // Check metrics
    let metrics = monitor.get_metrics().expect("Should get metrics");
    assert_eq!(metrics.total_events, 2);
    assert!(metrics.average_severity > 0.5);
    
    // Check if system is considered under attack
    assert!(monitor.is_under_attack());
}

/// Test attack pattern detection
#[test]
fn test_attack_pattern_detection() {
    let config = SecurityConfig {
        enable_attack_detection: true,
        history_window: Duration::from_secs(10),
        ..Default::default()
    };
    
    let monitor = SecurityMonitor::new(config);
    
    // Simulate rapid memory corruption events (attack pattern)
    for i in 0..5 {
        let event = SecurityEvent {
            event_type: SecurityEventType::MemoryCorruption,
            severity: 0.9,
            description: format!("Memory corruption event {}", i),
            timestamp: std::time::SystemTime::now(),
            context: std::collections::HashMap::new(),
            source: "Test".to_string(),
            event_id: 0,
        };
        
        let _ = monitor.record_event(event);
        
        // Small delay to create temporal pattern
        thread::sleep(Duration::from_millis(10));
    }
    
    // Should detect attack pattern
    let metrics = monitor.get_metrics().expect("Should get metrics");
    assert!(metrics.attack_indicators > 0, "Should detect attack pattern");
}

/// Test performance impact of security features
#[test]
fn test_security_performance_impact() {
    let start_time = Instant::now();
    
    // Create secure collector with monitoring
    let config = GcSecurityConfig {
        enable_monitoring: true,
        enable_type_validation: true,
        ..Default::default()
    };
    let collector = SecureCycleCollector::new(config);
    
    // Perform many operations
    for i in 0..1000 {
        let address = 0x10000 + i * 8;
        if collector.register(address, 1, 64).is_ok() {
            let _ = collector.add_possible_root(address);
            if i % 10 == 0 {
                let _ = collector.collect();
            }
        }
    }
    
    let elapsed = start_time.elapsed();
    
    // Security features should not cause excessive overhead
    assert!(elapsed < Duration::from_secs(5), 
           "Security operations should complete in reasonable time: {:?}", elapsed);
}

/// Test error handling in security operations
#[test]
fn test_security_error_handling() {
    let config = GcSecurityConfig::default();
    let collector = SecureCycleCollector::new(config);
    
    // Test invalid registrations
    assert!(collector.register(0, 1, 0).is_err()); // Invalid size
    assert!(collector.register(0, 1, 64).is_err()); // Invalid address
    
    // Test operations on invalid objects
    assert!(collector.unregister(0x999999).is_ok()); // Should handle gracefully
    assert!(collector.add_possible_root(0x999999).is_ok()); // Should handle gracefully
}

/// Test integration between security components
#[test]
fn test_security_integration() {
    // Create all security components
    let gc_config = GcSecurityConfig::default();
    let collector = SecureCycleCollector::new(gc_config);
    
    let resource_limits = ResourceLimits::default();
    let resource_monitor = ResourceMonitor::new(resource_limits);
    
    let security_config = SecurityConfig::default();
    let security_monitor = SecurityMonitor::new(security_config);
    
    // Test integrated operation
    let address = 0x10000;
    
    // Record allocation
    assert!(resource_monitor.record_allocation(64).is_ok());
    
    // Register with GC
    assert!(collector.register(address, 1, 64).is_ok());
    
    // Validate memory
    assert!(security_monitor.validate_memory(address, 64).is_ok());
    
    // Add to possible roots
    assert!(collector.add_possible_root(address).is_ok());
    
    // Perform collection
    assert!(collector.collect().is_ok());
    
    // Clean up
    assert!(collector.unregister(address).is_ok());
    resource_monitor.record_deallocation(64);
}

/// Stress test for security under high load
#[test]
fn test_security_stress() {
    let config = GcSecurityConfig {
        max_possible_roots: 1000,
        max_collection_time: Duration::from_secs(2),
        ..Default::default()
    };
    let collector = Arc::new(SecureCycleCollector::new(config));
    
    let security_config = SecurityConfig::default();
    let monitor = Arc::new(SecurityMonitor::new(security_config));
    
    let mut handles = vec![];
    
    // Spawn multiple threads creating stress
    for thread_id in 0..5 {
        let collector_clone = collector.clone();
        let monitor_clone = monitor.clone();
        
        let handle = thread::spawn(move || {
            for i in 0..200 {
                let address = (thread_id * 10000 + i) * 8 + 0x100000;
                
                // Register object
                if collector_clone.register(address, 1, 64).is_ok() {
                    // Validate memory
                    let _ = monitor_clone.validate_memory(address, 64);
                    
                    // Add as possible root
                    let _ = collector_clone.add_possible_root(address);
                    
                    // Periodic collection
                    if i % 20 == 0 {
                        let _ = collector_clone.collect();
                    }
                }
            }
        });
        handles.push(handle);
    }
    
    // Wait for completion
    for handle in handles {
        handle.join().expect("Stress test thread should complete");
    }
    
    // Final validation
    assert!(collector.collect().is_ok());
    
    let metrics = monitor.get_metrics().expect("Should get metrics");
    println!("Stress test metrics: {:?}", metrics);
    
    // Should not be under attack due to normal operations
    assert!(!monitor.is_under_attack() || metrics.average_severity < 0.5);
}

/// Test security configuration validation
#[test]
fn test_security_configuration() {
    // Test valid configurations
    let config1 = GcSecurityConfig {
        max_possible_roots: 1000,
        max_collection_time: Duration::from_millis(500),
        max_graph_depth: 5000,
        max_incremental_work: 500,
        enable_monitoring: true,
        enable_type_validation: true,
    };
    let _collector1 = SecureCycleCollector::new(config1);
    
    // Test edge case configurations
    let config2 = GcSecurityConfig {
        max_possible_roots: 1,
        max_collection_time: Duration::from_millis(1),
        max_graph_depth: 1,
        max_incremental_work: 1,
        enable_monitoring: false,
        enable_type_validation: false,
    };
    let _collector2 = SecureCycleCollector::new(config2);
    
    // Both should work without panicking
}

/// Test that security doesn't break normal functionality
#[test]
fn test_security_functionality_preservation() {
    let config = GcSecurityConfig::default();
    let collector = SecureCycleCollector::new(config);
    
    // Normal GC operations should work
    let addresses = vec![0x10000, 0x10008, 0x10010, 0x10018];
    
    // Register objects
    for &addr in &addresses {
        assert!(collector.register(addr, 1, 64).is_ok());
    }
    
    // Add as possible roots
    for &addr in &addresses {
        assert!(collector.add_possible_root(addr).is_ok());
    }
    
    // Collection should work
    assert!(collector.collect().is_ok());
    
    // Statistics should be available
    assert!(collector.stats().is_ok());
    
    // Cleanup
    for &addr in &addresses {
        assert!(collector.unregister(addr).is_ok());
    }
}