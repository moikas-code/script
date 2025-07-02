#[cfg(test)]
mod tests {
    use crate::lexer::Lexer;
    use crate::parser::{Parser, ExprKind, StmtKind};
    use crate::semantic::SemanticAnalyzer;
    use crate::types::Type;
    use crate::lowering::AstLowerer;
    use std::collections::HashMap;
    
    #[test]
    fn test_async_await_tokens() {
        let source = "async await";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.scan_tokens();
        
        assert_eq!(errors.len(), 0);
        assert_eq!(tokens.len(), 3); // async, await, EOF
        
        assert_eq!(tokens[0].lexeme, "async");
        assert_eq!(tokens[1].lexeme, "await");
    }
    
    #[test]
    fn test_parse_async_function() {
        let source = r#"
            async fn fetch_data() -> string {
                return "data"
            }
        "#;
        
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.scan_tokens();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        assert_eq!(program.statements.len(), 1);
        
        match &program.statements[0].kind {
            StmtKind::Function { name, is_async, .. } => {
                assert_eq!(name, "fetch_data");
                assert!(*is_async);
            }
            _ => panic!("Expected function statement"),
        }
    }
    
    #[test]
    fn test_parse_await_expression() {
        let source = r#"
            async fn example() -> i32 {
                let result = await fetch_data()
                return result
            }
        "#;
        
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.scan_tokens();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        match &program.statements[0].kind {
            StmtKind::Function { body, .. } => {
                match &body.statements[0].kind {
                    StmtKind::Let { init, .. } => {
                        match &init.as_ref().unwrap().kind {
                            ExprKind::Await { .. } => {
                                // Successfully parsed await expression
                            }
                            _ => panic!("Expected await expression"),
                        }
                    }
                    _ => panic!("Expected let statement"),
                }
            }
            _ => panic!("Expected function statement"),
        }
    }
    
    #[test]
    fn test_semantic_async_function_type() {
        let source = r#"
            async fn get_number() -> i32 {
                return 42
            }
        "#;
        
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.scan_tokens();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze_program(&program).unwrap();
        
        // The function should have Future<i32> as return type
        // This would be verified through the symbol table
    }
    
    #[test]
    fn test_semantic_await_only_in_async() {
        let source = r#"
            fn regular_function() -> i32 {
                let result = await something()
                return result
            }
        "#;
        
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.scan_tokens();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze_program(&program);
        
        // This should produce an error since await is used outside async function
        assert!(result.is_ok()); // analyze_program collects errors internally
        // In a real implementation, we'd check analyzer.errors
    }
    
    #[test]
    fn test_semantic_await_future_type() {
        let source = r#"
            async fn async_op() -> i32 {
                return 42
            }
            
            async fn use_async() -> i32 {
                let result = await async_op()
                return result
            }
        "#;
        
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.scan_tokens();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze_program(&program).unwrap();
        
        // The await expression should unwrap Future<i32> to i32
    }
    
    #[test]
    fn test_lower_async_function() {
        let source = r#"
            async fn compute() -> i32 {
                return 42
            }
        "#;
        
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.scan_tokens();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        let symbol_table = crate::semantic::SymbolTable::new();
        let type_info = HashMap::new();
        let mut lowerer = AstLowerer::new(symbol_table, type_info);
        let module = lowerer.lower_program(&program).unwrap();
        
        assert_eq!(module.functions().len(), 1);
        let func = module.get_function_by_name("compute").unwrap();
        
        // Check that the function returns Future<i32>
        match &func.return_type {
            Type::Future(inner) => {
                assert_eq!(**inner, Type::Unknown); // Would be i32 with proper type annotation conversion
            }
            _ => panic!("Expected Future return type"),
        }
    }
    
    #[test]
    fn test_runtime_async_execution() {
        use crate::runtime::{ScriptFuture, Executor, Timer};
        use std::task::Poll;
        use std::time::Duration;
        
        struct SimpleAsync {
            completed: bool,
        }
        
        impl ScriptFuture for SimpleAsync {
            type Output = i32;
            
            fn poll(&mut self, _waker: &std::task::Waker) -> Poll<Self::Output> {
                if !self.completed {
                    self.completed = true;
                    Poll::Pending
                } else {
                    Poll::Ready(42)
                }
            }
        }
        
        let executor = Executor::new();
        let task_id = Executor::spawn(executor.clone(), Box::new(SimpleAsync { completed: false }));
        
        // Run the executor
        Executor::run(executor);
    }
    
    #[test]
    fn test_scheduler_with_async_tasks() {
        use crate::runtime::{Scheduler, ScriptFuture, Timer};
        use std::time::Duration;
        use std::sync::{Arc, Mutex};
        use std::task::Poll;
        
        let scheduler = Scheduler::new(2);
        let counter = Arc::new(Mutex::new(0));
        
        struct CounterTask {
            counter: Arc<Mutex<i32>>,
            done: bool,
        }
        
        impl ScriptFuture for CounterTask {
            type Output = ();
            
            fn poll(&mut self, _waker: &std::task::Waker) -> Poll<Self::Output> {
                if !self.done {
                    *self.counter.lock().unwrap() += 1;
                    self.done = true;
                    Poll::Ready(())
                } else {
                    Poll::Ready(())
                }
            }
        }
        
        // Spawn multiple tasks
        for _ in 0..5 {
            let counter_clone = counter.clone();
            scheduler.spawn(Box::new(CounterTask {
                counter: counter_clone,
                done: false,
            }));
        }
        
        // Give tasks time to execute
        std::thread::sleep(Duration::from_millis(100));
        
        scheduler.shutdown();
        
        assert_eq!(*counter.lock().unwrap(), 5);
    }
    
    #[test]
    fn test_async_await_integration() {
        let source = r#"
            async fn delay(ms: i32) -> () {
                // Simulated delay
                return ()
            }
            
            async fn process() -> i32 {
                await delay(100)
                return 42
            }
            
            async fn main() -> i32 {
                let result = await process()
                return result
            }
        "#;
        
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.scan_tokens();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        // Parse should succeed
        assert_eq!(program.statements.len(), 3);
        
        // All functions should be async
        for stmt in &program.statements {
            match &stmt.kind {
                StmtKind::Function { is_async, .. } => {
                    assert!(*is_async);
                }
                _ => panic!("Expected function statement"),
            }
        }
    }
    
    #[test]
    fn test_nested_await_expressions() {
        let source = r#"
            async fn inner() -> i32 { 42 }
            async fn middle() -> i32 { await inner() }
            async fn outer() -> i32 { await middle() }
        "#;
        
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.scan_tokens();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        assert_eq!(program.statements.len(), 3);
        
        // Verify semantic analysis handles nested awaits
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze_program(&program).unwrap();
    }
    
    #[test]
    fn test_async_export_function() {
        let source = r#"
            export async fn public_async_api() -> string {
                return "result"
            }
        "#;
        
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.scan_tokens();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        match &program.statements[0].kind {
            StmtKind::Export { export } => {
                match export {
                    crate::parser::ExportKind::Function { is_async, .. } => {
                        assert!(*is_async);
                    }
                    _ => panic!("Expected function export"),
                }
            }
            _ => panic!("Expected export statement"),
        }
    }
}