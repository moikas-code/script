pub mod capabilities;
pub mod completion;
pub mod definition;
pub mod handlers;
pub mod semantic_tokens;
pub mod server;
pub mod state;

#[cfg(test)]
mod tests;

pub use server::ScriptLanguageServer;
pub use state::ServerState;
