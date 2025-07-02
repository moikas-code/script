//! Integration tests for the debugger system
//!
//! These tests demonstrate the complete breakpoint system functionality
//! including initialization, breakpoint management, and runtime integration.

use script::debugger::{
    get_debugger, initialize_debugger, shutdown_debugger, BreakpointCondition, BreakpointType,
    ExecutionContext, RuntimeDebugInterface,
};
use script::runtime::value::Value;
use script::source::SourceLocation;
use std::collections::HashMap;

/// Test basic debugger initialization and shutdown
#[test]
fn test_debugger_lifecycle() {
    // Clean up any existing debugger
    let _ = shutdown_debugger();

    // Test initialization
    assert!(initialize_debugger().is_ok());

    // Test getting debugger
    let debugger = get_debugger().unwrap();
    assert!(!debugger.is_enabled());

    // Enable debugging
    debugger.set_enabled(true);
    assert!(debugger.is_enabled());

    // Test shutdown
    assert!(shutdown_debugger().is_ok());
}

/// Test breakpoint manager functionality
#[test]
fn test_breakpoint_management() {
    let _ = shutdown_debugger();
    initialize_debugger().unwrap();

    let debugger = get_debugger().unwrap();
    let manager = debugger.breakpoint_manager();

    // Test adding breakpoints
    let line_bp_id = manager
        .add_line_breakpoint("test.script".to_string(), 10)
        .unwrap();
    let func_bp_id = manager
        .add_function_breakpoint("main".to_string(), None)
        .unwrap();
    let addr_bp_id = manager.add_address_breakpoint(0x1000).unwrap();
    let exc_bp_id = manager
        .add_exception_breakpoint(Some("RuntimeError".to_string()))
        .unwrap();

    // Test getting breakpoints
    assert_eq!(manager.get_all_breakpoints().len(), 4);

    let line_bp = manager.get_breakpoint(line_bp_id).unwrap();
    assert!(matches!(
        line_bp.breakpoint_type,
        BreakpointType::Line { .. }
    ));

    let func_bp = manager.get_breakpoint(func_bp_id).unwrap();
    assert!(matches!(
        func_bp.breakpoint_type,
        BreakpointType::Function { .. }
    ));

    // Test breakpoint enable/disable
    assert!(manager.disable_breakpoint(line_bp_id).is_ok());
    let disabled_bp = manager.get_breakpoint(line_bp_id).unwrap();
    assert!(!disabled_bp.enabled);

    assert!(manager.enable_breakpoint(line_bp_id).is_ok());
    let enabled_bp = manager.get_breakpoint(line_bp_id).unwrap();
    assert!(enabled_bp.enabled);

    // Test breakpoint conditions
    let condition = BreakpointCondition::new("x > 10".to_string(), true);
    assert!(manager
        .set_breakpoint_condition(line_bp_id, condition)
        .is_ok());

    let conditional_bp = manager.get_breakpoint(line_bp_id).unwrap();
    assert!(conditional_bp.condition.is_some());

    // Test removing breakpoints
    assert!(manager.remove_breakpoint(line_bp_id).is_ok());
    assert!(manager.get_breakpoint(line_bp_id).is_err());
    assert_eq!(manager.get_all_breakpoints().len(), 3);

    // Test clearing all breakpoints
    assert!(manager.clear_all_breakpoints().is_ok());
    assert_eq!(manager.get_all_breakpoints().len(), 0);

    shutdown_debugger().unwrap();
}

/// Test breakpoint location matching
#[test]
fn test_breakpoint_location_matching() {
    let _ = shutdown_debugger();
    initialize_debugger().unwrap();

    let debugger = get_debugger().unwrap();
    let manager = debugger.breakpoint_manager();

    // Add test breakpoints
    let _line_bp = manager
        .add_line_breakpoint("test.script".to_string(), 42)
        .unwrap();
    let _func_bp = manager
        .add_function_breakpoint("main".to_string(), None)
        .unwrap();

    // Test location matching
    let location = SourceLocation::new(42, 1, 0);

    // Should match line breakpoint
    assert!(manager.should_break_at_file_location("test.script", location, None));

    // Should not match different file
    assert!(!manager.should_break_at_file_location("other.script", location, None));

    // Should not match different line
    let other_location = SourceLocation::new(43, 1, 0);
    assert!(!manager.should_break_at_file_location("test.script", other_location, None));

    // Should match function breakpoint
    assert!(manager.should_break_at_file_location("test.script", location, Some("main")));

    // Should not match different function
    assert!(!manager.should_break_at_file_location("test.script", location, Some("other")));

    shutdown_debugger().unwrap();
}

/// Test hit recording functionality
#[test]
fn test_hit_recording() {
    let _ = shutdown_debugger();
    initialize_debugger().unwrap();

    let debugger = get_debugger().unwrap();
    let manager = debugger.breakpoint_manager();

    let bp_id = manager
        .add_line_breakpoint("test.script".to_string(), 10)
        .unwrap();
    let location = SourceLocation::new(10, 1, 0);

    // Record some hits
    assert!(manager
        .record_hit(bp_id, location, Some("main".to_string()), Some(1))
        .is_ok());
    assert!(manager
        .record_hit(bp_id, location, Some("main".to_string()), Some(1))
        .is_ok());

    // Check hit count
    let bp = manager.get_breakpoint(bp_id).unwrap();
    assert_eq!(bp.hit_count, 2);

    // Check hit history
    let history = manager.get_hit_history();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].breakpoint.id, bp_id);
    assert_eq!(history[0].location, location);

    // Clear history
    assert!(manager.clear_hit_history().is_ok());
    assert_eq!(manager.get_hit_history().len(), 0);

    shutdown_debugger().unwrap();
}

/// Test debugger session management
#[test]
fn test_session_management() {
    let _ = shutdown_debugger();
    initialize_debugger().unwrap();

    let debugger = get_debugger().unwrap();

    // Create sessions
    let session1_id = debugger
        .create_session(
            "test_session1".to_string(),
            Some("test1.script".to_string()),
        )
        .unwrap();
    let session2_id = debugger
        .create_session("test_session2".to_string(), None)
        .unwrap();

    // Test session retrieval
    let session1 = debugger.get_session(session1_id).unwrap();
    assert_eq!(session1.name, "test_session1");
    assert_eq!(session1.file, Some("test1.script".to_string()));
    assert!(session1.active);

    let session2 = debugger.get_session(session2_id).unwrap();
    assert_eq!(session2.name, "test_session2");
    assert_eq!(session2.file, None);

    // Test listing sessions
    let sessions = debugger.list_sessions();
    assert_eq!(sessions.len(), 2);

    // Test session removal
    assert!(debugger.remove_session(session1_id).is_ok());
    assert_eq!(debugger.list_sessions().len(), 1);
    assert!(debugger.get_session(session1_id).is_err());

    shutdown_debugger().unwrap();
}

/// Test runtime debug interface
#[test]
fn test_runtime_debug_interface() {
    let _ = shutdown_debugger();
    initialize_debugger().unwrap();

    let debugger = get_debugger().unwrap();
    debugger.set_enabled(true);

    let manager = debugger.breakpoint_manager();
    let _bp_id = manager
        .add_line_breakpoint("test.script".to_string(), 10)
        .unwrap();

    // Create runtime debug interface
    let debug_interface = RuntimeDebugInterface::new();

    // Create execution context
    let location = SourceLocation::new(10, 1, 0);
    let mut context = ExecutionContext::with_file(location, "test.script".to_string());
    context.add_variable("x".to_string(), Value::I32(42));

    // Test execution control
    // Should not continue when breakpoint is hit
    assert!(!debug_interface.should_continue_execution(&context));

    // Test different location (no breakpoint)
    let other_location = SourceLocation::new(20, 1, 0);
    let other_context = ExecutionContext::with_file(other_location, "test.script".to_string());

    // Should continue when no breakpoint is hit
    assert!(debug_interface.should_continue_execution(&other_context));

    // Test hook methods (should not panic)
    debug_interface.after_execution(&context, Some(&Value::I32(42)));
    debug_interface.on_function_enter(&context);
    debug_interface.on_function_exit(&context, Some(&Value::I32(0)));
    debug_interface.on_exception(&context, "TestError", "Test exception message");
    debug_interface.on_variable_assignment(&context, "y", None, &Value::I32(100));

    shutdown_debugger().unwrap();
}

/// Test breakpoint statistics
#[test]
fn test_breakpoint_statistics() {
    let _ = shutdown_debugger();
    initialize_debugger().unwrap();

    let debugger = get_debugger().unwrap();
    let manager = debugger.breakpoint_manager();

    // Add various types of breakpoints
    let line_id = manager
        .add_line_breakpoint("test.script".to_string(), 10)
        .unwrap();
    let func_id = manager
        .add_function_breakpoint("main".to_string(), None)
        .unwrap();
    let addr_id = manager.add_address_breakpoint(0x1000).unwrap();
    let exc_id = manager
        .add_exception_breakpoint(Some("Error".to_string()))
        .unwrap();

    // Disable one breakpoint
    manager.disable_breakpoint(func_id).unwrap();

    // Add condition to one breakpoint
    let condition = BreakpointCondition::new("x > 0".to_string(), true);
    manager
        .set_breakpoint_condition(line_id, condition)
        .unwrap();

    // Record some hits
    let location = SourceLocation::new(10, 1, 0);
    manager.record_hit(line_id, location, None, None).unwrap();
    manager.record_hit(addr_id, location, None, None).unwrap();

    // Get statistics
    let stats = manager.get_statistics();
    assert_eq!(stats.total_breakpoints, 4);
    assert_eq!(stats.enabled_breakpoints, 3);
    assert_eq!(stats.disabled_breakpoints, 1);
    assert_eq!(stats.line_breakpoints, 1);
    assert_eq!(stats.function_breakpoints, 1);
    assert_eq!(stats.address_breakpoints, 1);
    assert_eq!(stats.exception_breakpoints, 1);
    assert_eq!(stats.conditional_breakpoints, 1);
    assert_eq!(stats.total_hits, 2);

    shutdown_debugger().unwrap();
}

/// Test error conditions
#[test]
fn test_error_conditions() {
    let _ = shutdown_debugger();
    initialize_debugger().unwrap();

    let debugger = get_debugger().unwrap();
    let manager = debugger.breakpoint_manager();

    // Test invalid breakpoint operations
    assert!(manager.get_breakpoint(999).is_err());
    assert!(manager.remove_breakpoint(999).is_err());
    assert!(manager.enable_breakpoint(999).is_err());
    assert!(manager.disable_breakpoint(999).is_err());

    // Test invalid breakpoint creation
    assert!(manager.add_line_breakpoint("".to_string(), 10).is_err());
    assert!(manager
        .add_line_breakpoint("test.script".to_string(), 0)
        .is_err());
    assert!(manager
        .add_function_breakpoint("".to_string(), None)
        .is_err());

    // Test invalid session operations
    assert!(debugger.get_session(999).is_err());
    assert!(debugger.remove_session(999).is_err());

    // Test double initialization
    assert!(initialize_debugger().is_err());

    shutdown_debugger().unwrap();

    // Test operations without initialization
    assert!(get_debugger().is_err());
    assert!(shutdown_debugger().is_err());
}

/// Test execution context functionality
#[test]
fn test_execution_context() {
    let location = SourceLocation::new(10, 5, 100);

    // Test basic context creation
    let context = ExecutionContext::new(location);
    assert_eq!(context.location, location);
    assert!(context.file.is_none());
    assert!(context.function_name.is_none());
    assert_eq!(context.stack_depth, 0);
    assert!(context.thread_id.is_none());

    // Test context with file
    let context = ExecutionContext::with_file(location, "test.script".to_string());
    assert_eq!(context.file, Some("test.script".to_string()));

    // Test context with function
    let context = ExecutionContext::with_function(
        location,
        Some("test.script".to_string()),
        "main".to_string(),
    );
    assert_eq!(context.function_name, Some("main".to_string()));

    // Test context chaining
    let context = ExecutionContext::new(location)
        .with_stack_depth(5)
        .with_thread_id(1);
    assert_eq!(context.stack_depth, 5);
    assert_eq!(context.thread_id, Some(1));

    // Test variable management
    let mut context = ExecutionContext::new(location);
    context.add_variable("x".to_string(), Value::I32(42));
    context.add_variable("y".to_string(), Value::String("hello".to_string()));

    assert_eq!(context.get_variable("x"), Some(&Value::I32(42)));
    assert_eq!(
        context.get_variable("y"),
        Some(&Value::String("hello".to_string()))
    );
    assert!(context.get_variable("z").is_none());

    let removed = context.remove_variable("x");
    assert_eq!(removed, Some(Value::I32(42)));
    assert!(context.get_variable("x").is_none());
}
