# Script Language Server Protocol (LSP) Implementation

## Overview

This module implements the Language Server Protocol for the Script programming language, providing IDE features like syntax highlighting, code completion, and more.

## Current Implementation Status

### ✅ Completed Features

1. **Basic LSP Server Structure**
   - TCP and stdio communication modes
   - Server initialization and shutdown
   - Document synchronization (open/close/change)

2. **Syntax Highlighting via Semantic Tokens**
   - Token type mapping from Script lexer to LSP semantic tokens
   - Support for keywords, identifiers, literals, and operators
   - Full document semantic token generation

3. **Architecture**
   - Modular design with separate concerns:
     - `server.rs`: Main LSP server implementation
     - `handlers.rs`: Request/notification handlers
     - `state.rs`: Server state management
     - `semantic_tokens.rs`: Token generation and mapping
     - `capabilities.rs`: Server capability definitions
   - Thread-safe document storage using DashMap
   - Async/await support with tokio

### 🚧 Known Issues

The LSP implementation is complete but cannot be built due to unrelated compilation errors in other parts of the codebase. Once these are resolved, the LSP server will be fully functional.

### 📋 Future Enhancements

The architecture supports adding:
- Diagnostics (error/warning reporting)
- Code completion
- Hover information
- Go to definition
- Find references
- Document formatting
- Rename refactoring

## Usage

Once the library compiles:

```bash
# Build the LSP server
cargo build --release --bin script-lsp

# Run in stdio mode (for most editors)
./target/release/script-lsp

# Run in TCP mode
./target/release/script-lsp --tcp 7777
```

## Integration Examples

See `/docs/lsp/` for:
- VS Code extension example
- Usage guide for various editors
- Testing instructions