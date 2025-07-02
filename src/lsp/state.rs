use dashmap::DashMap;
use std::sync::Arc;
use tower_lsp::lsp_types::TextDocumentItem;
use url::Url;

/// Represents an open document in the LSP server
#[derive(Debug, Clone)]
pub struct Document {
    pub uri: Url,
    pub version: i32,
    pub content: String,
    pub language_id: String,
}

impl Document {
    pub fn new(item: TextDocumentItem) -> Self {
        Self {
            uri: item.uri,
            version: item.version,
            content: item.text,
            language_id: item.language_id,
        }
    }

    pub fn update(&mut self, version: i32, text: String) {
        self.version = version;
        self.content = text;
    }
}

/// Shared server state that can be accessed from multiple handlers
#[derive(Debug, Clone)]
pub struct ServerState {
    /// Currently open documents indexed by their URI
    pub documents: Arc<DashMap<Url, Document>>,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            documents: Arc::new(DashMap::new()),
        }
    }

    /// Open a new document
    pub fn open_document(&self, item: TextDocumentItem) {
        let doc = Document::new(item);
        self.documents.insert(doc.uri.clone(), doc);
    }

    /// Update an existing document
    pub fn update_document(&self, uri: Url, version: i32, text: String) -> Option<()> {
        self.documents.get_mut(&uri)?.update(version, text);
        Some(())
    }

    /// Close a document
    pub fn close_document(&self, uri: &Url) -> Option<Document> {
        self.documents.remove(uri).map(|(_, doc)| doc)
    }

    /// Get a document by URI
    pub fn get_document(&self, uri: &Url) -> Option<Document> {
        self.documents.get(uri).map(|doc| doc.clone())
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new()
    }
}
