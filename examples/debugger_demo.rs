//! Demonstration of the Script language debugger system
//!
//! This example shows how to use the comprehensive breakpoint management
//! system that has been implemented for the Script language debugger.

use script::debugger::{
    get_debugger, initialize_debugger, shutdown_debugger, BreakpointCondition, DebugEvent,
    ExecutionContext, RuntimeDebugInterface,
};
use script::runtime::value::Value;
use script::source::SourceLocation;

fn main() {
    println!("=== Script Language Debugger Demo ===\n");

    // Initialize the debugger
    if let Err(e) = initialize_debugger() {
        eprintln!("Failed to initialize debugger: {}", e);
        return;
    }

    println!("✓ Debugger initialized successfully");

    // Get the debugger instance
    let debugger = match get_debugger() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to get debugger: {}", e);
            return;
        }
    };

    // Enable debugging
    debugger.set_enabled(true);
    println!("✓ Debugging enabled");

    // Demo 1: Breakpoint Management
    println!("\n--- Demo 1: Breakpoint Management ---");

    let manager = debugger.breakpoint_manager();

    // Add various types of breakpoints
    let line_bp = manager
        .add_line_breakpoint("demo.script".to_string(), 42)
        .expect("Failed to add line breakpoint");
    println!("✓ Added line breakpoint {} at demo.script:42", line_bp);

    let func_bp = manager
        .add_function_breakpoint("main".to_string(), None)
        .expect("Failed to add function breakpoint");
    println!("✓ Added function breakpoint {} for 'main'", func_bp);

    let addr_bp = manager
        .add_address_breakpoint(0x1000)
        .expect("Failed to add address breakpoint");
    println!("✓ Added address breakpoint {} at 0x1000", addr_bp);

    let exc_bp = manager
        .add_exception_breakpoint(Some("RuntimeError".to_string()))
        .expect("Failed to add exception breakpoint");
    println!("✓ Added exception breakpoint {} for RuntimeError", exc_bp);

    // Demo 2: Conditional Breakpoints
    println!("\n--- Demo 2: Conditional Breakpoints ---");

    let condition = BreakpointCondition::new("x > 10".to_string(), true);
    manager
        .set_breakpoint_condition(line_bp, condition)
        .expect("Failed to set breakpoint condition");
    println!("✓ Added condition 'x > 10' to breakpoint {}", line_bp);

    // Demo 3: Breakpoint Information
    println!("\n--- Demo 3: Breakpoint Information ---");

    let breakpoints = manager.get_all_breakpoints();
    println!("Total breakpoints: {}", breakpoints.len();

    for bp in &breakpoints {
        println!("  - {}", bp);
    }

    // Demo 4: Breakpoint Operations
    println!("\n--- Demo 4: Breakpoint Operations ---");

    // Disable a breakpoint
    manager
        .disable_breakpoint(func_bp)
        .expect("Failed to disable breakpoint");
    println!("✓ Disabled function breakpoint {}", func_bp);

    // Enable it back
    manager
        .enable_breakpoint(func_bp)
        .expect("Failed to enable breakpoint");
    println!("✓ Re-enabled function breakpoint {}", func_bp);

    // Demo 5: Runtime Integration
    println!("\n--- Demo 5: Runtime Integration ---");

    let debug_interface = RuntimeDebugInterface::new();
    let location = SourceLocation::new(42, 1, 0);
    let mut context = ExecutionContext::with_file(location, "demo.script".to_string());
    context.add_variable("x".to_string(), Value::I32(15));
    context.add_variable(
        "message".to_string(),
        Value::String("Hello, debugger!".to_string()),
    );

    println!("Checking if execution should continue at demo.script:42...");
    let should_continue = debug_interface.should_continue_execution(&context);
    println!("✓ Should continue: {} (breakpoint hit!)", should_continue);

    // Simulate hitting the breakpoint
    if !should_continue {
        println!("✓ Breakpoint hit! Execution paused");

        // Show current variables
        println!("Current variables:");
        for (name, value) in &context.local_variables {
            println!("  {} = {}", name, value);
        }
    }

    // Demo 6: Hit Recording
    println!("\n--- Demo 6: Hit Recording ---");

    manager
        .record_hit(line_bp, location, Some("main".to_string()), Some(1))
        .expect("Failed to record hit");
    println!("✓ Recorded hit for breakpoint {}", line_bp);

    let bp_after_hit = manager
        .get_breakpoint(line_bp)
        .expect("Failed to get breakpoint");
    println!(
        "✓ Breakpoint {} hit count: {}",
        line_bp, bp_after_hit.hit_count
    );

    // Demo 7: Statistics
    println!("\n--- Demo 7: Statistics ---");

    let stats = manager.get_statistics();
    println!("{}", stats);

    // Demo 8: Session Management
    println!("\n--- Demo 8: Session Management ---");

    let session_id = debugger
        .create_session("demo_session".to_string(), Some("demo.script".to_string()))
        .expect("Failed to create session");
    println!(
        "✓ Created debug session {} named 'demo_session'",
        session_id
    );

    let session = debugger
        .get_session(session_id)
        .expect("Failed to get session");
    println!(
        "✓ Session info: {} (file: {:?})",
        session.name, session.file
    );

    // Demo 9: Debug Events
    println!("\n--- Demo 9: Debug Events ---");

    let events = vec![
        DebugEvent::ExecutionStarted {
            file: "demo.script".to_string(),
            entry_point: "main".to_string(),
        },
        DebugEvent::FunctionEntered {
            name: "main".to_string(),
            location,
            parameters: context.local_variables.clone(),
        },
        DebugEvent::BreakpointHit {
            breakpoint_id: line_bp,
            location,
            function_name: Some("main".to_string()),
        },
        DebugEvent::VariableChanged {
            name: "x".to_string(),
            old_value: Some(Value::I32(10)),
            new_value: Value::I32(15),
            location,
        },
    ];

    for event in events {
        debug_interface.emit_debug_event(&event);
    }

    // Demo 10: Cleanup
    println!("\n--- Demo 10: Cleanup ---");

    manager
        .clear_all_breakpoints()
        .expect("Failed to clear breakpoints");
    println!("✓ Cleared all breakpoints");

    debugger
        .remove_session(session_id)
        .expect("Failed to remove session");
    println!("✓ Removed debug session");

    // Shutdown the debugger
    if let Err(e) = shutdown_debugger() {
        eprintln!("Failed to shutdown debugger: {}", e);
        return;
    }

    println!("✓ Debugger shutdown successfully");

    println!("\n=== Demo Complete ===");
    println!("\nThe Script language debugger provides:");
    println!("• Comprehensive breakpoint management (line, function, address, exception)");
    println!("• Conditional breakpoints with expression evaluation");
    println!("• Runtime integration hooks for execution control");
    println!("• Session management for multiple debugging contexts");
    println!("• Hit recording and statistics");
    println!("• Thread-safe operations for concurrent debugging");
    println!("• CLI integration for interactive debugging");
    println!("• Full integration with the Script language runtime");
}
