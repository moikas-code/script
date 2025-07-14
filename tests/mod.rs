//! Integration tests for the Script language

// Module organization
mod edge_cases;
mod integration;
mod property;
mod regression;
mod runtime;
mod security;
mod utils;

// Individual test files
mod async_integration_test;
mod async_security_test;
mod async_transform_security_test;
mod async_vulnerability_test;
mod basic_module_parse_test;
mod cross_module_type_checking_test;
mod debugger_integration_test;
mod end_to_end_generics_test;
mod error_handling_comprehensive;
mod error_handling_test;
mod generic_integration_test;
mod generic_types_test;
mod generics_parsing_test;
mod memory_safety_integration_tests;
mod memory_safety_params_test;
mod module_dependency_test;
mod module_examples_test;
mod module_tests;
mod monomorphization_integration_test;
mod pattern_matching_tests;
mod resource_limits_test;
mod runtime_tests;
mod semantic_integration_tests;
mod stdlib_integration_test;
mod test_generic_execution;
mod type_inference_test;
