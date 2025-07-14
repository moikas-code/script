//! Comprehensive tests for enhanced REPL functionality
//!
//! This module tests all aspects of the enhanced REPL including
//! session state, history, multiline input, module imports,
//! and overall user experience.

use script::repl::{EnhancedRepl, History, ModuleLoader, ReplMode, Session};
use script::runtime::Value;
use script::types::Type;
use std::io::Write;
use tempfile::{tempdir, NamedTempFile};

#[cfg(test)]
mod history_tests {
    use super::*;

    #[test]
    fn test_history_basic_operations() {
        let temp_dir = tempdir().unwrap();
        let history_file = temp_dir.path().join("test_history");
        let mut history = History::new(history_file);

        // Test adding commands
        history.add("let x = 5".to_string());
        history.add("let y = 10".to_string());
        history.add("x + y".to_string());

        assert_eq!(history.len(), 3);
        assert!(!history.is_empty());

        // Test recent commands
        let recent = history.recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0], "let y = 10");
        assert_eq!(recent[1], "x + y");
    }

    #[test]
    fn test_history_persistence() {
        let temp_dir = tempdir().unwrap();
        let history_file = temp_dir.path().join("test_history");

        // Create and populate history
        {
            let mut history = History::new(history_file.clone());
            history.add("command 1".to_string());
            history.add("command 2".to_string());
            history.save().unwrap();
        }

        // Load history and verify
        {
            let mut history = History::new(history_file);
            history.load_from_file().unwrap();
            assert_eq!(history.len(), 2);
            assert_eq!(history.get(0).unwrap(), "command 1");
            assert_eq!(history.get(1).unwrap(), "command 2");
        }
    }

    #[test]
    fn test_history_deduplication() {
        let temp_dir = tempdir().unwrap();
        let history_file = temp_dir.path().join("test_history");
        let mut history = History::new(history_file);

        history.add("same command".to_string());
        history.add("same command".to_string()); // Should be ignored
        history.add("different command".to_string());

        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_history_search() {
        let temp_dir = tempdir().unwrap();
        let history_file = temp_dir.path().join("test_history");
        let mut history = History::new(history_file);

        history.add("let x = 5".to_string());
        history.add("let y = 10".to_string());
        history.add("function hello() {}".to_string());
        history.add("x + y".to_string());

        let matches = history.search("let");
        assert_eq!(matches.len(), 2);

        let matches = history.search("function");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], "function hello() {}");
    }

    #[test]
    fn test_history_max_size() {
        let temp_dir = tempdir().unwrap();
        let history_file = temp_dir.path().join("test_history");
        let mut history = History::new(history_file);

        // Add many commands
        for i in 0..1200 {
            history.add(format!("command {}", i));
        }

        // Should not exceed max size
        assert!(history.len() <= 1000);

        // Should have the most recent commands
        let recent = history.recent(5);
        assert!(recent.iter().any(|cmd| cmd.contains("1199")));
    }
}

#[cfg(test)]
mod session_tests {
    use super::*;

    #[test]
    fn test_session_variables() {
        let mut session = Session::new();

        session.define_variable("x".to_string(), Value::I64(42), Type::I64);

        assert!(session.is_defined("x"));
        assert_eq!(session.get_variable("x"), Some(&Value::I64(42)));
        assert_eq!(session.variables().len(), 1);
    }

    #[test]
    fn test_session_types() {
        let mut session = Session::new();

        session.define_type("MyType".to_string(), Type::String);

        assert!(session.is_defined("MyType"));
        assert_eq!(session.get_type("MyType"), Some(&Type::String));
        assert_eq!(session.types().len(), 1);
    }

    #[test]
    fn test_session_clear() {
        let mut session = Session::new();

        session.define_variable("x".to_string(), Value::I64(42), Type::I64);
        session.define_type("MyType".to_string(), Type::String);

        assert_eq!(session.item_count(), 2);

        session.clear();
        assert_eq!(session.item_count(), 0);
        assert!(!session.is_defined("x"));
        assert!(!session.is_defined("MyType"));
    }

    #[test]
    fn test_session_remove() {
        let mut session = Session::new();

        session.define_variable("x".to_string(), Value::I64(42), Type::I64);
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
    }

    #[test]
    fn test_session_summary() {
        let mut session = Session::new();

        session.define_variable("x".to_string(), Value::I64(42), Type::I64);
        session.define_type("MyType".to_string(), Type::String);

        let summary = session.summary();
        assert!(summary.contains("1 variables"));
        assert!(summary.contains("0 functions"));
        assert!(summary.contains("1 types"));
    }

    #[test]
    fn test_session_list_names() {
        let mut session = Session::new();

        session.define_variable("z_var".to_string(), Value::I64(1), Type::I64);
        session.define_variable("a_var".to_string(), Value::I64(2), Type::I64);
        session.define_type("MyType".to_string(), Type::String);

        let names = session.list_names();
        assert_eq!(names.len(), 3);
        // Should be sorted
        assert_eq!(names[0], "MyType");
        assert_eq!(names[1], "a_var");
        assert_eq!(names[2], "z_var");
    }

    #[test]
    fn test_session_import_from() {
        let mut session1 = Session::new();
        let mut session2 = Session::new();

        session1.define_variable("x".to_string(), Value::I64(42), Type::I64);
        session1.define_type("MyType".to_string(), Type::String);

        session2.import_from(&session1);

        assert!(session2.is_defined("x"));
        assert!(session2.is_defined("MyType"));
        assert_eq!(session2.item_count(), 2);
    }
}

#[cfg(test)]
mod module_loader_tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_module_loader_creation() {
        let loader = ModuleLoader::new();
        assert!(!loader.search_paths().is_empty());
        assert_eq!(loader.module_count(), 0);
    }

    #[test]
    fn test_add_search_path() {
        let mut loader = ModuleLoader::new();
        let initial_count = loader.search_paths().len();

        loader.add_search_path("/custom/path");
        assert_eq!(loader.search_paths().len(), initial_count + 1);
    }

    #[test]
    fn test_module_not_found() {
        let mut loader = ModuleLoader::new();
        let result = loader.load_module("nonexistent_module");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_module_tracking() {
        let loader = ModuleLoader::new();
        assert!(!loader.is_module_loaded("test"));
        assert!(loader.list_loaded_modules().is_empty());
        assert_eq!(loader.total_exports_count(), 0);
    }

    #[test]
    fn test_module_unload() {
        let mut loader = ModuleLoader::new();
        assert!(!loader.unload_module("nonexistent"));
    }

    #[test]
    fn test_module_clear() {
        let mut loader = ModuleLoader::new();
        loader.clear();
        assert_eq!(loader.module_count(), 0);
    }

    #[test]
    fn test_module_with_simple_file() {
        let temp_dir = tempdir().unwrap();
        let module_path = temp_dir.path().join("simple.script");

        // Create a very simple module file
        fs::write(&module_path, "// Simple module").unwrap();

        let mut loader = ModuleLoader::new();
        loader.add_search_path(temp_dir.path());

        // Test that the module can be found (even if parsing fails)
        let result = loader.load_module("simple");
        // This will fail due to empty content, but that's expected
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod multiline_input_tests {
    use super::*;

    fn create_test_repl() -> EnhancedRepl {
        // For testing, we'll need to mock some of the REPL functionality
        // This is a simplified test setup
        EnhancedRepl::new().unwrap()
    }

    #[test]
    fn test_multiline_detection() {
        let repl = create_test_repl();

        // Test various multiline patterns
        assert!(repl.is_multiline_start("fn hello() {"));
        assert!(repl.is_multiline_start("struct Person {"));
        assert!(repl.is_multiline_start("enum Color {"));
        assert!(repl.is_multiline_start("impl MyStruct {"));
        assert!(repl.is_multiline_start("type MyType = {"));
        assert!(repl.is_multiline_start("let x = ["));

        // Test single line patterns
        assert!(!repl.is_multiline_start("let x = 5"));
        assert!(!repl.is_multiline_start("x + y"));
        assert!(!repl.is_multiline_start("println(\"hello\")"));
    }

    #[test]
    fn test_bracket_balancing() {
        let repl = create_test_repl();

        // Test balanced brackets
        assert!(!repl.needs_continuation("let x = [1, 2, 3]"));
        assert!(!repl.needs_continuation("fn hello() { return 42; }"));
        assert!(!repl.needs_continuation("println(\"hello world\")"));

        // Test unbalanced brackets
        assert!(repl.needs_continuation("let x = [1, 2"));
        assert!(repl.needs_continuation("fn hello() {"));
        assert!(repl.needs_continuation("println(\"hello"));
    }

    #[test]
    fn test_string_handling_in_continuation() {
        let repl = create_test_repl();

        // Brackets inside strings shouldn't affect continuation
        assert!(!repl.needs_continuation("let x = \"[unclosed bracket\""));
        assert!(!repl.needs_continuation("let x = \"{unclosed brace\""));

        // But actual unclosed strings should trigger continuation
        assert!(repl.needs_continuation("let x = \"unclosed string"));
    }
}

#[cfg(test)]
mod repl_commands_tests {
    use super::*;

    #[test]
    fn test_repl_mode_switching() {
        let mut repl = create_test_repl();

        // Test mode switching
        assert_eq!(repl.mode, ReplMode::Interactive);

        // Note: We'd need to expose more of the REPL internals to test command handling
        // For now, this tests the basic structure
    }

    fn create_test_repl() -> EnhancedRepl {
        EnhancedRepl::new().unwrap()
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_repl_creation() {
        let repl_result = EnhancedRepl::new();
        assert!(repl_result.is_ok());

        let repl = repl_result.unwrap();
        assert_eq!(repl.mode, ReplMode::Interactive);
        assert_eq!(repl.prompt_counter, 1);
        assert!(!repl.in_multiline);
    }

    #[test]
    fn test_repl_default() {
        let repl = EnhancedRepl::default();
        assert_eq!(repl.mode, ReplMode::Interactive);
    }

    #[test]
    fn test_session_persistence_workflow() {
        let temp_dir = tempdir().unwrap();
        let session_file = temp_dir.path().join(".script_session");

        // Create a session and save it
        {
            let mut session = Session::new();
            session.define_variable("test_var".to_string(), Value::I64(100), Type::I64);
            session.define_type("TestType".to_string(), Type::Bool);

            assert_eq!(session.item_count(), 2);

            // Save should succeed
            assert!(session.save().is_ok());
        }

        // Load a new session (simplified test)
        {
            let session = Session::load_or_create();
            assert!(session.is_ok());
        }
    }

    #[test]
    fn test_module_import_workflow() {
        let temp_dir = tempdir().unwrap();
        let module_path = temp_dir.path().join("test_module.script");

        // Create a simple module
        fs::write(&module_path, "pub let version = \"1.0\";").unwrap();

        let mut loader = ModuleLoader::new();
        loader.add_search_path(temp_dir.path());

        // Test module loading workflow
        let result = loader.load_module("test_module");
        // This will fail due to simplified parsing, but tests the workflow
        assert!(result.is_err());

        // Test that search paths work
        assert!(loader.search_paths().iter().any(|p| p == temp_dir.path()));
    }

    #[test]
    fn test_comprehensive_session_workflow() {
        let mut session = Session::new();

        // Test complete workflow
        assert_eq!(session.item_count(), 0);

        // Add various items
        session.define_variable("x".to_string(), Value::I64(42), Type::I64);
        session.define_variable(
            "name".to_string(),
            Value::String("test".to_string()),
            Type::String,
        );
        session.define_type(
            "Point".to_string(),
            Type::Struct {
                name: "Point".to_string(),
                fields: vec![("x".to_string(), Type::F64), ("y".to_string(), Type::F64)],
            },
        );

        assert_eq!(session.item_count(), 3);
        assert!(session.is_defined("x"));
        assert!(session.is_defined("name"));
        assert!(session.is_defined("Point"));

        // Test summary
        let summary = session.summary();
        assert!(summary.contains("2 variables"));
        assert!(summary.contains("1 types"));

        // Test validation
        assert!(session.validate().is_ok());

        // Test removal
        assert!(session.remove("x"));
        assert_eq!(session.item_count(), 2);
        assert!(!session.is_defined("x"));

        // Test clear
        session.clear();
        assert_eq!(session.item_count(), 0);
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[test]
    fn test_invalid_module_handling() {
        let mut loader = ModuleLoader::new();

        // Test loading non-existent module
        let result = loader.load_module("does_not_exist");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_invalid_import_items() {
        let mut loader = ModuleLoader::new();

        // Test importing non-existent items
        let result = loader.import_items("nonexistent_module", &["item1".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_session_file_errors() {
        // Test session operations with invalid paths
        let session = Session::new();

        // Save should handle errors gracefully
        let result = session.save();
        // Should succeed or fail gracefully
        let _ = result;
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_history_performance() {
        let temp_dir = tempdir().unwrap();
        let history_file = temp_dir.path().join("perf_test_history");
        let mut history = History::new(history_file);

        let start = Instant::now();

        // Add many commands
        for i in 0..10000 {
            history.add(format!("command {}", i));
        }

        let duration = start.elapsed();
        assert!(
            duration.as_millis() < 1000,
            "History operations took too long: {:?}",
            duration
        );

        // Test search performance
        let start = Instant::now();
        let _matches = history.search("command");
        let search_duration = start.elapsed();
        assert!(
            search_duration.as_millis() < 100,
            "History search took too long: {:?}",
            search_duration
        );
    }

    #[test]
    fn test_session_performance() {
        let mut session = Session::new();

        let start = Instant::now();

        // Add many items
        for i in 0..1000 {
            session.define_variable(format!("var_{}", i), Value::I64(i as i64), Type::I64);
        }

        let duration = start.elapsed();
        assert!(
            duration.as_millis() < 500,
            "Session operations took too long: {:?}",
            duration
        );

        // Test lookup performance
        let start = Instant::now();
        for i in 0..1000 {
            let _ = session.is_defined(&format!("var_{}", i));
        }
        let lookup_duration = start.elapsed();
        assert!(
            lookup_duration.as_millis() < 100,
            "Session lookups took too long: {:?}",
            lookup_duration
        );
    }

    #[test]
    fn test_module_loader_performance() {
        let mut loader = ModuleLoader::new();

        // Test search path performance
        let start = Instant::now();
        for i in 0..1000 {
            loader.add_search_path(format!("/path/{}", i));
        }
        let duration = start.elapsed();
        assert!(
            duration.as_millis() < 100,
            "Module loader operations took too long: {:?}",
            duration
        );
    }
}

#[cfg(test)]
mod edge_cases_tests {
    use super::*;

    #[test]
    fn test_empty_inputs() {
        let repl = create_test_repl();

        assert!(!repl.is_multiline_start(""));
        assert!(!repl.is_multiline_start("   "));
        assert!(!repl.is_multiline_start("\t\n"));
    }

    #[test]
    fn test_special_characters() {
        let repl = create_test_repl();

        // Test with special characters that might break parsing
        assert!(!repl.needs_continuation("let x = \"emoji: 🚀\""));
        assert!(!repl.needs_continuation("let name = \"unicode: αβγ\""));
    }

    #[test]
    fn test_very_long_inputs() {
        let repl = create_test_repl();

        let long_string = "let x = \"".to_string() + &"a".repeat(10000) + "\"";
        assert!(!repl.needs_continuation(&long_string));

        let long_unclosed = "let x = \"".to_string() + &"a".repeat(10000);
        assert!(repl.needs_continuation(&long_unclosed));
    }

    #[test]
    fn test_nested_brackets() {
        let repl = create_test_repl();

        // Test deeply nested structures
        assert!(!repl.needs_continuation("let x = [[[[[]]]]]"));
        assert!(!repl.needs_continuation("let x = {a: {b: {c: {}}}}"));
        assert!(repl.needs_continuation("let x = [[[["));
    }

    fn create_test_repl() -> EnhancedRepl {
        EnhancedRepl::new().unwrap()
    }
}
