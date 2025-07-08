//! Tests for enhanced module context and error reporting

use script::module::{
    ModulePath, ImportPath, ModuleContext, ModuleDependencyChain,
    VisibilityContext, PrivateAccessAttempt, ModuleContextStack,
};
use script::source::{Span, SourceLocation};
use script::types::Type;
use std::path::PathBuf;

#[test]
fn test_module_dependency_chain() {
    let root = ModulePath::from_string("app.main").unwrap();
    let mut chain = ModuleDependencyChain::new(root.clone());
    
    // Add dependencies
    let utils = ModulePath::from_string("app.utils").unwrap();
    let utils_import = ImportPath::from_string("./utils").unwrap();
    let span1 = create_test_span(1, 1, 1, 20);
    chain.push(utils.clone(), utils_import, span1);
    
    let helpers = ModulePath::from_string("app.helpers").unwrap();
    let helpers_import = ImportPath::from_string("./helpers").unwrap();
    let span2 = create_test_span(2, 1, 2, 22);
    chain.push(helpers.clone(), helpers_import, span2);
    
    // Check chain
    assert_eq!(chain.chain.len(), 3);
    assert_eq!(chain.imports.len(), 2);
    assert_eq!(chain.locations.len(), 2);
    
    // Check cycle detection
    assert!(!chain.would_create_cycle(&ModulePath::from_string("app.config").unwrap()));
    assert!(chain.would_create_cycle(&root));
    assert!(chain.would_create_cycle(&utils));
    assert!(chain.would_create_cycle(&helpers));
    
    // Format chain
    let formatted = chain.format_chain();
    assert!(formatted.contains("app.main"));
    assert!(formatted.contains("app.utils"));
    assert!(formatted.contains("app.helpers"));
    assert!(formatted.contains("imports"));
}

#[test]
fn test_module_context() {
    let module = ModulePath::from_string("test.module").unwrap();
    let mut ctx = ModuleContext::new(module.clone());
    
    // Add source file mapping
    ctx.source_files.insert(module.clone(), PathBuf::from("test/module.script"));
    
    // Record import resolution
    let import = ImportPath::from_string("dependency").unwrap();
    let resolved = ModulePath::from_string("test.dependency").unwrap();
    let span = create_test_span(5, 1, 5, 30);
    
    ctx.record_resolution(import.clone(), Some(resolved.clone()), span, None, 100);
    
    assert_eq!(ctx.resolution_history.len(), 1);
    let step = &ctx.resolution_history[0];
    assert_eq!(step.import_stmt, import);
    assert_eq!(step.resolved_path, Some(resolved));
    assert_eq!(step.resolution_time, 100);
    assert!(step.error.is_none());
    
    // Record failed resolution
    let failed_import = ImportPath::from_string("missing").unwrap();
    ctx.record_resolution(failed_import, None, span, Some("Module not found".to_string()), 50);
    
    assert_eq!(ctx.resolution_history.len(), 2);
    assert!(ctx.resolution_history[1].error.is_some());
}

#[test]
fn test_module_context_stack() {
    let mut stack = ModuleContextStack::new();
    
    // Initially empty
    assert!(stack.current().is_none());
    
    // Push contexts
    let ctx1 = ModuleContext::new(ModulePath::from_string("module1").unwrap());
    let ctx2 = ModuleContext::new(ModulePath::from_string("module2").unwrap());
    
    stack.push(ctx1);
    assert_eq!(stack.current().unwrap().current_module.to_string(), "module1");
    
    stack.push(ctx2);
    assert_eq!(stack.current().unwrap().current_module.to_string(), "module2");
    assert_eq!(stack.stack().len(), 2);
    
    // Pop contexts
    let popped = stack.pop();
    assert!(popped.is_some());
    assert_eq!(popped.unwrap().current_module.to_string(), "module2");
    assert_eq!(stack.current().unwrap().current_module.to_string(), "module1");
    
    stack.pop();
    assert!(stack.current().is_none());
}

#[test]
fn test_visibility_context() {
    let mut vis_ctx = VisibilityContext::new();
    
    // Record exports
    let module1 = ModulePath::from_string("module1").unwrap();
    vis_ctx.record_exports(module1.clone(), vec!["public_fn".to_string(), "PublicType".to_string()]);
    
    let module2 = ModulePath::from_string("module2").unwrap();
    vis_ctx.record_exports(module2.clone(), vec!["another_fn".to_string()]);
    
    // Check exports
    assert_eq!(vis_ctx.exports.get(&module1).unwrap().len(), 2);
    assert_eq!(vis_ctx.exports.get(&module2).unwrap().len(), 1);
    
    // Record private access attempt
    let attempt = PrivateAccessAttempt {
        symbol: "private_fn".to_string(),
        from_module: module2.clone(),
        target_module: module1.clone(),
        location: create_test_span(10, 5, 10, 20),
    };
    vis_ctx.record_private_access(attempt);
    
    assert_eq!(vis_ctx.access_attempts.len(), 1);
    assert_eq!(vis_ctx.access_attempts[0].symbol, "private_fn");
}

#[test]
fn test_module_context_error_formatting() {
    let module = ModulePath::from_string("app.main").unwrap();
    let mut context = ModuleContext::new(module.clone());
    
    // Add some resolution history
    context.record_resolution(
        ImportPath::from_string("missing").unwrap(),
        None,
        create_test_span(5, 1, 5, 20),
        Some("Module not found".to_string()),
        50
    );
    
    // Add source file
    context.source_files.insert(module.clone(), PathBuf::from("app/main.script"));
    
    // Verify resolution history
    assert_eq!(context.resolution_history.len(), 1);
    assert!(context.resolution_history[0].error.is_some());
}


// Helper function to create test spans
fn create_test_span(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Span {
    Span::new(
        SourceLocation::new(start_line, start_col, 0),
        SourceLocation::new(end_line, end_col, 0),
    )
}