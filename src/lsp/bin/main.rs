use script::lsp::ScriptLanguageServer;
use std::env;

#[tokio::main]
async fn main() {
    // Simple logging setup
    env_logger::init();

    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && args[1] == "--tcp" {
        // TCP mode: script-lsp --tcp [port]
        let port = args
            .get(2)
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(7777);

        let addr = format!("127.0.0.1:{}", port));

        if let Err(e) = ScriptLanguageServer::run_tcp(&addr).await {
            eprintln!("Failed to run TCP server: {e}");
            std::process::exit(1);
        }
    } else {
        // Default: stdio mode
        ScriptLanguageServer::run_stdio().await;
    }
}
