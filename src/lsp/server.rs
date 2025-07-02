use crate::lsp::{capabilities::get_server_capabilities, handlers::*, state::ServerState};
use tokio::io::{AsyncRead, AsyncWrite};
use tower_lsp::jsonrpc::Result;
use tower_lsp::{lsp_types::*, LanguageServer, LspService, Server};

/// The main Script Language Server implementation
#[derive(Debug)]
pub struct ScriptLanguageServer {
    state: ServerState,
}

impl ScriptLanguageServer {
    pub fn new() -> Self {
        Self {
            state: ServerState::new(),
        }
    }

    /// Create a new LSP service for this server
    pub fn create_service() -> (LspService<Self>, tower_lsp::ClientSocket) {
        LspService::new(|client| {
            // We can use the client for sending notifications/requests to the editor
            let _client = client;
            ScriptLanguageServer::new()
        })
    }

    /// Run the server over stdio
    pub async fn run_stdio() {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, socket) = Self::create_service();
        Server::new(stdin, stdout, socket).serve(service).await;
    }

    /// Run the server over TCP
    pub async fn run_tcp(addr: &str) -> std::io::Result<()> {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        eprintln!("Script LSP Server listening on {}", addr);

        let (service, socket) = Self::create_service();
        let (stream, _) = listener.accept().await?;
        let (read, write) = stream.into_split();

        Server::new(read, write, socket).serve(service).await;

        Ok(())
    }

    /// Run the server with custom streams (for testing)
    pub async fn run_custom<R, W>(
        read: R,
        write: W,
        service: LspService<Self>,
        socket: tower_lsp::ClientSocket,
    ) where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        Server::new(read, write, socket).serve(service).await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for ScriptLanguageServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: get_server_capabilities(),
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        // Called after successful initialization
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        if let Err(e) = handle_did_open(&self.state, params).await {
            eprintln!("Error in did_open: {:?}", e);
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Err(e) = handle_did_change(&self.state, params).await {
            eprintln!("Error in did_change: {:?}", e);
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        if let Err(e) = handle_did_close(&self.state, params).await {
            eprintln!("Error in did_close: {:?}", e);
        }
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        handle_semantic_tokens_full(&self.state, params).await
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        handle_completion(&self.state, params).await
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        handle_goto_definition(&self.state, params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let server = ScriptLanguageServer::new();
        // Basic sanity check
        assert!(server.state.documents.is_empty());
    }

    #[tokio::test]
    async fn test_initialize() {
        let server = ScriptLanguageServer::new();
        let params = InitializeParams::default();

        let result = server.initialize(params).await.unwrap();

        // Check that we have the expected capabilities
        assert!(result.capabilities.text_document_sync.is_some());
        assert!(result.capabilities.semantic_tokens_provider.is_some());
    }
}
