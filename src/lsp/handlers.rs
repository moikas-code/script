use crate::lsp::completion::generate_completions;
use crate::lsp::definition::goto_definition;
use crate::lsp::semantic_tokens::generate_semantic_tokens;
use crate::lsp::state::ServerState;
use tower_lsp::jsonrpc::{Error, Result};
use tower_lsp::lsp_types::{
    CompletionParams, CompletionResponse, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, GotoDefinitionParams, GotoDefinitionResponse, SemanticTokens,
    SemanticTokensParams, SemanticTokensResult,
};

/// Handle textDocument/didOpen notification
pub async fn handle_did_open(state: &ServerState, params: DidOpenTextDocumentParams) -> Result<()> {
    state.open_document(params.text_document);
    Ok(())
}

/// Handle textDocument/didChange notification
pub async fn handle_did_change(
    state: &ServerState,
    params: DidChangeTextDocumentParams,
) -> Result<()> {
    let uri = params.text_document.uri;
    let version = params.text_document.version;

    // For now, we only support full document sync
    if let Some(change) = params.content_changes.into_iter().next() {
        // TextDocumentContentChangeEvent has a text field for full document sync
        if change.range.is_none() {
            // This is a full document update
            state.update_document(uri, version, change.text);
        } else {
            // Incremental changes not yet supported
            return Err(Error::invalid_request());
        }
    }

    Ok(())
}

/// Handle textDocument/didClose notification
pub async fn handle_did_close(
    state: &ServerState,
    params: DidCloseTextDocumentParams,
) -> Result<()> {
    state.close_document(&params.text_document.uri);
    Ok(())
}

/// Handle textDocument/semanticTokens/full request
pub async fn handle_semantic_tokens_full(
    state: &ServerState,
    params: SemanticTokensParams,
) -> Result<Option<SemanticTokensResult>> {
    let uri = params.text_document.uri;

    // Get the document content
    let document = state
        .get_document(&uri)
        .ok_or_else(|| Error::invalid_params("Document not found"))?;

    // Generate semantic tokens
    let tokens = generate_semantic_tokens(&document.content);

    // Convert to LSP format
    let result = SemanticTokens {
        result_id: None,
        data: tokens,
    };

    Ok(Some(SemanticTokensResult::Tokens(result)))
}

/// Handle textDocument/completion request
pub async fn handle_completion(
    state: &ServerState,
    params: CompletionParams,
) -> Result<Option<CompletionResponse>> {
    let uri = params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;

    // Get the document content
    let document = state
        .get_document(&uri)
        .ok_or_else(|| Error::invalid_params("Document not found"))?;

    // Get trigger kind
    let trigger_kind = params
        .context
        .as_ref()
        .map(|ctx| ctx.trigger_kind)
        .unwrap_or(tower_lsp::lsp_types::CompletionTriggerKind::INVOKED);

    // Generate completions
    let completions = generate_completions(&document.content, position, trigger_kind);

    Ok(Some(completions))
}

/// Handle textDocument/definition request
pub async fn handle_goto_definition(
    state: &ServerState,
    params: GotoDefinitionParams,
) -> Result<Option<GotoDefinitionResponse>> {
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    // Get the document content
    let document = state
        .get_document(&uri)
        .ok_or_else(|| Error::invalid_params("Document not found"))?;

    // Find definition
    let location = goto_definition(&document.content, position, &uri);

    // Convert to response format
    Ok(location.map(GotoDefinitionResponse::Scalar))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::TextDocumentItem;
    use url::Url;

    fn create_test_state() -> ServerState {
        ServerState::new()
    }

    fn create_test_document() -> TextDocumentItem {
        TextDocumentItem {
            uri: Url::parse("file:///test.script").unwrap(),
            language_id: "script".to_string(),
            version: 1,
            text: "let x = 42;".to_string(),
        }
    }

    #[tokio::test]
    async fn test_handle_did_open() {
        let state = create_test_state();
        let doc = create_test_document();
        let uri = doc.uri.clone();

        let params = DidOpenTextDocumentParams { text_document: doc };

        handle_did_open(&state, params).await.unwrap();

        // Verify document was opened
        assert!(state.get_document(&uri).is_some());
    }

    #[tokio::test]
    async fn test_handle_did_close() {
        let state = create_test_state();
        let doc = create_test_document();
        let uri = doc.uri.clone();

        // First open the document
        state.open_document(doc);
        assert!(state.get_document(&uri).is_some());

        // Then close it
        let params = DidCloseTextDocumentParams {
            text_document: tower_lsp::lsp_types::TextDocumentIdentifier { uri: uri.clone() },
        };

        handle_did_close(&state, params).await.unwrap();

        // Verify document was closed
        assert!(state.get_document(&uri).is_none());
    }
}
