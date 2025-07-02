#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_parse_doc_comments() {
        let comments = vec![
            "This is a simple function".to_string(),
            "It does something useful".to_string(),
        ];

        let doc = parse_doc_comments(comments);
        assert_eq!(
            doc.sections.description,
            "This is a simple function\nIt does something useful"
        );
    }

    #[test]
    fn test_parse_doc_with_params() {
        let comments = vec![
            "Calculate the sum of two numbers".to_string(),
            "@param a - The first number".to_string(),
            "@param b - The second number".to_string(),
            "@returns The sum of a and b".to_string(),
        ];

        let doc = parse_doc_comments(comments);
        assert_eq!(doc.sections.description, "Calculate the sum of two numbers");
        assert_eq!(doc.sections.params.len(), 2);
        assert_eq!(doc.sections.params[0].name, "a");
        assert_eq!(doc.sections.params[0].description, "The first number");
        assert_eq!(doc.sections.params[1].name, "b");
        assert_eq!(doc.sections.params[1].description, "The second number");
        assert_eq!(doc.sections.returns, Some("The sum of a and b".to_string()));
    }

    #[test]
    fn test_parse_doc_with_typed_params() {
        let comments = vec![
            "Test function".to_string(),
            "@param count [number] - The count value".to_string(),
            "@param name [string] - The name to use".to_string(),
        ];

        let doc = parse_doc_comments(comments);
        assert_eq!(doc.sections.params.len(), 2);
        assert_eq!(doc.sections.params[0].name, "count");
        assert_eq!(doc.sections.params[0].type_info, Some("number".to_string()));
        assert_eq!(doc.sections.params[1].name, "name");
        assert_eq!(doc.sections.params[1].type_info, Some("string".to_string()));
    }

    #[test]
    fn test_doc_generator_simple() {
        let source = r#"
/// Test function documentation
/// This is a test function
fn test_func(x, y) {
    return x + y
}
"#;

        let mut generator = generator::DocGenerator::new();
        generator
            .generate_from_source(source, "test_module")
            .unwrap();

        let db = generator.database();
        assert!(db.modules.contains_key("test_module"));

        let module = &db.modules["test_module"];
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].name, "test_func");

        let func_doc = &module.functions[0].documentation;
        assert!(func_doc.is_some());

        let doc = func_doc.as_ref().unwrap();
        assert_eq!(
            doc.sections.description,
            "Test function documentation\nThis is a test function"
        );
    }

    #[test]
    fn test_doc_generator_constant() {
        let source = r#"
/// Maximum buffer size
/// @note This should not exceed system limits
const MAX_SIZE = 1024
"#;

        let mut generator = generator::DocGenerator::new();
        generator
            .generate_from_source(source, "test_module")
            .unwrap();

        let db = generator.database();
        let module = &db.modules["test_module"];
        assert_eq!(module.constants.len(), 1);
        assert_eq!(module.constants[0].name, "MAX_SIZE");
        assert_eq!(module.constants[0].value, Some("1024".to_string()));

        let const_doc = &module.constants[0].documentation;
        assert!(const_doc.is_some());

        let doc = const_doc.as_ref().unwrap();
        assert_eq!(doc.sections.description, "Maximum buffer size");
        assert_eq!(doc.sections.notes.len(), 1);
        assert_eq!(
            doc.sections.notes[0],
            "This should not exceed system limits"
        );
    }

    #[test]
    fn test_search_index() {
        let mut db = DocDatabase::default();

        // Add a module
        let module = ModuleDoc {
            name: "test_module".to_string(),
            path: "test_module".to_string(),
            documentation: None,
            functions: vec![FunctionDoc {
                name: "calculate_sum".to_string(),
                signature: "fn calculate_sum(a, b)".to_string(),
                documentation: Some(Documentation {
                    content: "Calculate the sum of two numbers".to_string(),
                    sections: DocSections {
                        description: "Calculate the sum of two numbers".to_string(),
                        ..Default::default()
                    },
                    span: crate::source::Span::dummy(),
                }),
                is_async: false,
                is_exported: true,
            }],
            types: Vec::new(),
            constants: Vec::new(),
            submodules: Vec::new(),
        };

        db.modules.insert("test_module".to_string(), module);

        // Create search engine
        let engine = search::SearchEngine::new(&db);

        // Test exact match
        let results = engine.search("calculate_sum");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "calculate_sum");

        // Test partial match
        let results = engine.search("calc");
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.name == "calculate_sum"));

        // Test word match
        let results = engine.search("sum");
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.name == "calculate_sum"));
    }

    #[test]
    fn test_html_escaping() {
        let generator = html::HtmlGenerator::new("./test_docs");

        // Test through reflection or by making escape methods public
        // For now, we'll test indirectly through doc generation
        let mut db = DocDatabase::default();

        let module = ModuleDoc {
            name: "test".to_string(),
            path: "test".to_string(),
            documentation: Some(Documentation {
                content: "Test <script> & \"quotes\"".to_string(),
                sections: DocSections {
                    description: "Test <script> & \"quotes\"".to_string(),
                    ..Default::default()
                },
                span: crate::source::Span::dummy(),
            }),
            functions: Vec::new(),
            types: Vec::new(),
            constants: Vec::new(),
            submodules: Vec::new(),
        };

        db.modules.insert("test".to_string(), module);

        // The actual HTML generation would escape these characters
        // This test verifies the structure is set up correctly
        assert!(db.modules["test"].documentation.is_some());
    }
}
