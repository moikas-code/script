# Script Language Server Protocol (LSP) Usage Guide

## Overview

The Script LSP server provides language intelligence features for Script language in any LSP-compatible editor.

## Building the LSP Server

```bash
# Debug build
cargo build --bin script-lsp

# Release build (recommended for production)
cargo build --release --bin script-lsp
```

## Running the LSP Server

### Stdio Mode (Default)

Most editors use stdio to communicate with language servers:

```bash
# Run in stdio mode
./target/release/script-lsp

# With debug logging
RUST_LOG=debug ./target/release/script-lsp
```

### TCP Mode

For debugging or special configurations:

```bash
# Run on default port (7777)
./target/release/script-lsp --tcp

# Run on custom port
./target/release/script-lsp --tcp 8080
```

## Current Features

### 1. Syntax Highlighting (Semantic Tokens)

The LSP server provides semantic token information for accurate syntax highlighting:

- **Keywords**: `let`, `fn`, `if`, `else`, `while`, `for`, etc.
- **Identifiers**: Variables, functions, types
- **Literals**: Numbers, strings, booleans
- **Operators**: All arithmetic, logical, and comparison operators
- **Comments**: Single-line comments

### 2. Document Management

- **Open/Close**: Tracks opened documents
- **Change Tracking**: Updates document content in real-time
- **Multi-document**: Supports multiple open documents simultaneously

## Editor Configuration

### VS Code

See `docs/lsp/vscode-extension-example.md` for a complete example.

### Neovim

Using `nvim-lspconfig`:

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

-- Define Script LSP config
if not configs.script_lsp then
  configs.script_lsp = {
    default_config = {
      cmd = {'path/to/script-lsp'},
      filetypes = {'script'},
      root_dir = lspconfig.util.root_pattern('.git', 'Cargo.toml'),
      settings = {},
    },
  }
end

-- Setup Script LSP
lspconfig.script_lsp.setup{}
```

### Helix

Add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "script"
scope = "source.script"
injection-regex = "script"
file-types = ["script"]
comment-token = "//"
language-server = { command = "script-lsp" }
```

## Testing the LSP Server

### Unit Tests

```bash
# Run all LSP tests
cargo test lsp

# Run with output
cargo test lsp -- --nocapture
```

### Manual Testing

1. Start the server in TCP mode:
   ```bash
   ./target/release/script-lsp --tcp 7777
   ```

2. Use an LSP client to connect and send requests:
   ```json
   // Initialize request
   {
     "jsonrpc": "2.0",
     "id": 1,
     "method": "initialize",
     "params": {
       "capabilities": {}
     }
   }
   ```

## Extending the LSP Server

The server is structured to easily add new features:

1. **Add new handlers** in `src/lsp/handlers.rs`
2. **Update capabilities** in `src/lsp/capabilities.rs`
3. **Implement methods** in `src/lsp/server.rs`

Common features to add:
- Diagnostics (error reporting)
- Code completion
- Hover information
- Go to definition
- Find references
- Document formatting

## Performance Considerations

- The server uses `DashMap` for thread-safe document storage
- Semantic tokens are generated on-demand
- Full document sync is currently used (incremental sync planned)
- The lexer is optimized for performance with zero-copy tokenization

## Troubleshooting

### Enable Debug Logging

```bash
RUST_LOG=debug ./target/release/script-lsp
```

### Common Issues

1. **Server not starting**: Check that the binary has execute permissions
2. **No syntax highlighting**: Ensure your editor requests semantic tokens
3. **Connection errors**: Verify the correct transport mode (stdio vs TCP)