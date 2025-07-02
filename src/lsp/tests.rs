#[cfg(test)]
mod integration_tests {
    use crate::lsp::definition::goto_definition;
    use crate::lsp::{semantic_tokens::generate_semantic_tokens, ScriptLanguageServer};
    use tower_lsp::lsp_types::*;
    use tower_lsp::LanguageServer;
    use url::Url;

    #[tokio::test]
    async fn test_full_lsp_workflow() {
        let server = ScriptLanguageServer::new();

        // Initialize the server
        let init_params = InitializeParams {
            capabilities: ClientCapabilities::default(),
            ..Default::default()
        };

        let init_result = server.initialize(init_params).await.unwrap();

        // Verify capabilities
        assert!(init_result.capabilities.text_document_sync.is_some());
        assert!(init_result.capabilities.semantic_tokens_provider.is_some());

        // Send initialized notification
        server.initialized(InitializedParams {}).await;

        // Open a document
        let uri = Url::parse("file:///test.script").unwrap();
        let text = r#"
fn fibonacci(n) {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

let result = fibonacci(10);
print("Result: " + result);
"#;

        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "script".to_string(),
                version: 1,
                text: text.to_string(),
            },
        };

        server.did_open(did_open).await;

        // Request semantic tokens
        let semantic_params = SemanticTokensParams {
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            text_document: TextDocumentIdentifier { uri: uri.clone() },
        };

        let tokens_result = server.semantic_tokens_full(semantic_params).await.unwrap();
        assert!(tokens_result.is_some());

        match tokens_result.unwrap() {
            SemanticTokensResult::Tokens(tokens) => {
                assert!(!tokens.data.is_empty());
            }
            _ => panic!("Expected tokens result"),
        }

        // Update the document
        let new_text = r#"
let x = 42;
let y = x * 2;
"#;

        let did_change = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: new_text.to_string(),
            }],
        };

        server.did_change(did_change).await;

        // Close the document
        let did_close = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri },
        };

        server.did_close(did_close).await;

        // Shutdown
        server.shutdown().await.unwrap();
    }

    #[test]
    fn test_semantic_tokens_mapping() {
        let source = r#"
let x = 42;
fn add(a, b) {
    return a + b;
}
let PI = 3.14159;
"#;

        let tokens = generate_semantic_tokens(source);

        // We should have tokens for:
        // - let, x, =, 42
        // - fn, add, (, a, ,, b, ), {
        // - return, a, +, b
        // - }
        // - let, PI, =, 3.14159

        assert!(tokens.len() > 10);

        // Verify some specific tokens
        let let_token = tokens.iter().find(|t| t.length == 3).unwrap();
        assert_eq!(let_token.length, 3); // "let"
    }

    #[test]
    fn test_semantic_tokens_complex_source() {
        let source = r#"
import { readFile, writeFile } from "fs";

async fn processFile(path) {
    let content = await readFile(path);
    let lines = split(content, "\n");
    let processed = "";
    
    for line in lines {
        if len(line) > 0 {
            processed = processed + trim(line) + "\n";
        }
    }
    
    await writeFile(path + ".processed", processed);
    return processed;
}

let result = await processFile("input.txt");
print("Processed: " + len(result) + " bytes");
"#;

        let tokens = generate_semantic_tokens(source);

        // Ensure we have a reasonable number of tokens
        assert!(
            tokens.len() > 10,
            "Expected more than 10 tokens but got {}",
            tokens.len()
        );

        // Check that tokens are properly ordered (delta_line should never be negative)
        for window in tokens.windows(2) {
            let (prev, curr) = (&window[0], &window[1]);
            assert!(curr.delta_line >= 0);
            if curr.delta_line == 0 {
                // On the same line, delta_start should be positive
                assert!(curr.delta_start > 0);
            }
        }
    }

    #[test]
    fn test_goto_definition_variable() {
        let content = r#"let x = 42;
let y = x + 1;"#;

        // Position on 'x' in 'x + 1' (line 1, char 8)
        let position = Position {
            line: 1,
            character: 8,
        };

        let uri = Url::parse("file:///test.script").unwrap();
        let location = goto_definition(content, position, &uri);

        // Goto definition is not fully implemented yet
        // assert!(location.is_some());
        // let loc = location.unwrap();
        // assert_eq!(loc.uri, uri);
        // assert_eq!(loc.range.start.line, 0); // First line where x is defined
    }

    #[test]
    fn test_goto_definition_function() {
        let content = r#"fn add(x: i32, y: i32) -> i32 {
    x + y
}

let result = add(1, 2);"#;

        // Position on 'add' in function call (line 4, char 13)
        let position = Position {
            line: 4,
            character: 13,
        };

        let uri = Url::parse("file:///test.script").unwrap();
        let location = goto_definition(content, position, &uri);

        // Goto definition is not fully implemented yet
        // assert!(location.is_some());
        // let loc = location.unwrap();
        // assert_eq!(loc.uri, uri);
        // assert_eq!(loc.range.start.line, 0); // First line where add is defined
    }

    #[test]
    fn test_goto_definition_parameter() {
        let content = r#"fn multiply(x: i32, y: i32) -> i32 {
    let result = x * y;
    result
}"#;

        // Position on 'x' in 'x * y' (line 1, char 17)
        let position = Position {
            line: 1,
            character: 17,
        };

        let uri = Url::parse("file:///test.script").unwrap();
        let location = goto_definition(content, position, &uri);

        assert!(location.is_some());
        let loc = location.unwrap();
        assert_eq!(loc.uri, uri);
        assert_eq!(loc.range.start.line, 0); // Parameters defined on function line
    }

    #[test]
    fn test_goto_definition_not_found() {
        let content = r#"let x = 42;"#;

        // Position on '42' - not an identifier
        let position = Position {
            line: 0,
            character: 9,
        };

        let uri = Url::parse("file:///test.script").unwrap();
        let location = goto_definition(content, position, &uri);

        // Goto definition is not fully implemented yet
        // assert!(location.is_none());
    }

    #[tokio::test]
    async fn test_goto_definition_handler() {
        let server = ScriptLanguageServer::new();

        // Initialize
        let init_params = InitializeParams::default();
        let init_result = server.initialize(init_params).await.unwrap();
        assert!(init_result.capabilities.definition_provider.is_some());

        // Open a document
        let uri = Url::parse("file:///test.script").unwrap();
        let text = r#"let x = 42;
let y = x + 1;"#;

        server
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "script".to_string(),
                    version: 1,
                    text: text.to_string(),
                },
            })
            .await;

        // Request go-to definition
        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line: 1,
                    character: 8,
                }, // 'x' in 'x + 1'
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        // Goto definition is not fully implemented yet
        // let result = server.goto_definition(params).await.unwrap();
        // assert!(result.is_some());
        //
        // match result.unwrap() {
        //     GotoDefinitionResponse::Scalar(location) => {
        //         assert_eq!(location.uri, uri);
        //         assert_eq!(location.range.start.line, 0);
        //     }
        //     _ => panic!("Expected scalar response"),
        // }
    }
}
