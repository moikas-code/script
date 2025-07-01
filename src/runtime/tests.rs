//! Integration tests for runtime components

#[cfg(test)]
mod runtime_integration_tests {
    use crate::runtime::{self, Runtime, RuntimeConfig, ScriptRc};
    
    #[test]
    fn test_runtime_initialization() {
        // Ensure clean state
        let _ = runtime::shutdown();
        
        // Test initialization
        assert!(!runtime::is_initialized());
        assert!(runtime::initialize().is_ok());
        assert!(runtime::is_initialized());
        
        // Test double initialization
        assert!(runtime::initialize().is_err());
        
        // Test shutdown
        assert!(runtime::shutdown().is_ok());
        assert!(!runtime::is_initialized());
    }
    
    #[test]
    fn test_scriptrc_basic() {
        let _ = runtime::shutdown();
        runtime::initialize().unwrap();
        
        // Create RC
        let rc = ScriptRc::new(42);
        assert_eq!(*rc, 42);
        assert_eq!(rc.strong_count(), 1);
        
        // Clone
        let rc2 = rc.clone();
        assert_eq!(rc.strong_count(), 2);
        assert_eq!(rc2.strong_count(), 2);
        
        runtime::shutdown().unwrap();
    }
    
    #[test]
    fn test_weak_references() {
        let _ = runtime::shutdown();
        runtime::initialize().unwrap();
        
        let rc = ScriptRc::new(vec![1, 2, 3]);
        let weak = rc.downgrade();
        
        // Upgrade while strong ref exists
        assert!(weak.upgrade().is_some());
        
        drop(rc);
        
        // Upgrade after strong ref dropped
        assert!(weak.upgrade().is_none());
        
        runtime::shutdown().unwrap();
    }
    
    #[test]
    fn test_memory_tracking() {
        let _ = runtime::shutdown();
        
        let mut config = RuntimeConfig::default();
        config.enable_profiling = true;
        
        runtime::initialize().unwrap();
        Runtime::initialize_with_config(config).unwrap();
        
        // Create some allocations
        let _rc1 = ScriptRc::new(100);
        let _rc2 = ScriptRc::new("test string".to_string());
        
        // Get runtime stats
        let rt = runtime::core::runtime().unwrap();
        let stats = rt.stats();
        
        assert!(stats.memory.total_allocations > 0);
        
        runtime::shutdown().unwrap();
    }
    
    #[test]
    fn test_panic_handling() {
        let _ = runtime::shutdown();
        
        let mut config = RuntimeConfig::default();
        config.enable_panic_handler = true;
        
        runtime::initialize().unwrap();
        Runtime::initialize_with_config(config).unwrap();
        
        let rt = runtime::core::runtime().unwrap();
        
        // Test successful execution
        let result = rt.execute_protected(|| 2 + 2);
        assert_eq!(result.unwrap(), 4);
        
        // Test panic recovery
        let result = rt.execute_protected(|| {
            panic!("Test panic");
        });
        assert!(result.is_err());
        
        runtime::shutdown().unwrap();
    }
}