# VS Code Extension Example for Script LSP

This document shows how to create a VS Code extension that uses the Script LSP server.

## package.json

```json
{
  "name": "scriptuage-support",
  "displayName": "Script Language Support",
  "description": "Language support for Script programming language",
  "version": "0.1.0",
  "engines": {
    "vscode": "^1.75.0"
  },
  "categories": ["Programming Languages"],
  "activationEvents": [
    "onLanguage:script"
  ],
  "main": "./out/extension.js",
  "contributes": {
    "languages": [{
      "id": "script",
      "aliases": ["Script", "script"],
      "extensions": [".script"],
      "configuration": "./language-configuration.json"
    }],
    "grammars": [{
      "language": "script",
      "scopeName": "source.script",
      "path": "./syntaxes/script.tmLanguage.json"
    }]
  },
  "scripts": {
    "vscode:prepublish": "npm run compile",
    "compile": "tsc -p ./",
    "watch": "tsc -watch -p ./"
  },
  "dependencies": {
    "vscode-languageclient": "^9.0.0"
  },
  "devDependencies": {
    "@types/vscode": "^1.75.0",
    "@types/node": "^20.0.0",
    "typescript": "^5.0.0"
  }
}
```

## extension.ts

```typescript
import * as path from 'path';
import { workspace, ExtensionContext } from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: ExtensionContext) {
    // Path to the Script LSP server executable
    const serverModule = context.asAbsolutePath(
        path.join('server', 'script-lsp')
    );
    
    // Server options
    const serverOptions: ServerOptions = {
        run: { module: serverModule, transport: TransportKind.stdio },
        debug: {
            module: serverModule,
            transport: TransportKind.stdio,
            options: { env: { RUST_LOG: 'debug' } }
        }
    };
    
    // Client options
    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'script' }],
        synchronize: {
            fileEvents: workspace.createFileSystemWatcher('**/*.script')
        }
    };
    
    // Create and start the language client
    client = new LanguageClient(
        'scriptLanguageServer',
        'Script Language Server',
        serverOptions,
        clientOptions
    );
    
    client.start();
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
```

## language-configuration.json

```json
{
    "comments": {
        "lineComment": "//",
        "blockComment": ["/*", "*/"]
    },
    "brackets": [
        ["{", "}"],
        ["[", "]"],
        ["(", ")"]
    ],
    "autoClosingPairs": [
        ["{", "}"],
        ["[", "]"],
        ["(", ")"],
        ["\"", "\""],
        ["'", "'"]
    ],
    "surroundingPairs": [
        ["{", "}"],
        ["[", "]"],
        ["(", ")"],
        ["\"", "\""],
        ["'", "'"]
    ]
}
```

## Building and Testing

1. Build the Script LSP server:
   ```bash
   cargo build --release --bin script-lsp
   ```

2. Copy the binary to your extension's server directory:
   ```bash
   cp target/release/script-lsp path/to/extension/server/
   ```

3. Compile the extension:
   ```bash
   npm install
   npm run compile
   ```

4. Test in VS Code:
   - Press F5 in VS Code to launch a new Extension Development Host
   - Open a `.script` file to test syntax highlighting

## Features Supported

- **Syntax Highlighting**: Semantic token-based highlighting for all Script language constructs
- **Document Synchronization**: Real-time updates as you type
- **Future Features**: The LSP server structure supports adding:
  - Auto-completion
  - Go to definition
  - Find references
  - Hover information
  - Diagnostics (errors/warnings)
  - Code formatting
  - Refactoring support