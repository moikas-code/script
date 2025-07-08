# Script KB MCP Server

An MCP (Model Context Protocol) server for managing the Script programming language knowledge base. This server allows Claude Code to read, update, and search documentation in the `/kb` directory, providing better context and memory for development tasks.

## Features

- **Read KB Files**: Access any markdown file in the knowledge base
- **Update KB Files**: Create or modify documentation files
- **Search KB**: Find content across all documentation
- **List Directories**: Browse the KB structure
- **Status Reports**: Get parsed implementation status
- **Issues Tracking**: Access current known issues

## Installation

1. Navigate to the Script project root:
   ```bash
   cd /path/to/script
   ```

2. Install dependencies:
   ```bash
   cd script-kb-mcp
   npm install
   ```

3. Build the project:
   ```bash
   npm run build
   ```

## Usage

### Running the Server

For development:
```bash
npm run dev
```

For production:
```bash
npm start
```

### Claude Desktop Integration

Add the following to your Claude Desktop configuration:

**For macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**For Windows**: `%APPDATA%/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "script-kb": {
      "command": "node",
      "args": ["/path/to/script/script-kb-mcp/dist/index.js"],
      "env": {
        "SCRIPT_PROJECT_ROOT": "/path/to/script"
      }
    }
  }
}
```

## Environment Variables

- `SCRIPT_PROJECT_ROOT`: Path to the Script project root directory (defaults to `../../..` from the server location)

## Available Tools

### `kb_read`
Read a file from the knowledge base.

**Parameters:**
- `path` (string): Path relative to `kb/` directory (e.g., `"active/KNOWN_ISSUES.md"`)

**Example:**
```
Use the kb_read tool to read the file "active/KNOWN_ISSUES.md"
```

### `kb_list`
List files and directories in the knowledge base.

**Parameters:**
- `directory` (string, optional): Directory path relative to `kb/` (defaults to root)

**Example:**
```
Use the kb_list tool to see what's in the "status" directory
```

### `kb_update`
Create or update a file in the knowledge base.

**Parameters:**
- `path` (string): Path relative to `kb/` directory
- `content` (string): Markdown content to write

**Example:**
```
Use the kb_update tool to create a new file "active/NEW_FEATURE.md" with documentation about the new feature
```

### `kb_delete`
Delete a file from the knowledge base.

**Parameters:**
- `path` (string): Path relative to `kb/` directory

**Example:**
```
Use the kb_delete tool to remove the outdated file "legacy/OLD_DOCS.md"
```

### `kb_search`
Search for content in knowledge base files.

**Parameters:**
- `query` (string): Text to search for
- `directory` (string, optional): Directory to search in (searches all if not specified)

**Example:**
```
Use the kb_search tool to find all mentions of "async" in the knowledge base
```

### `kb_status`
Get the current implementation status of the Script language.

**Parameters:** None

**Example:**
```
Use the kb_status tool to get the current implementation status
```

### `kb_issues`
Get the current known issues in the Script language implementation.

**Parameters:** None

**Example:**
```
Use the kb_issues tool to see what issues are currently being tracked
```

## Security Features

- **Path Validation**: Prevents directory traversal attacks
- **File Type Restrictions**: Only allows `.md` files
- **KB Scope**: All operations are restricted to the `kb/` directory
- **Category Validation**: Ensures new files are created in valid categories

## Valid KB Categories

Files can be created in these categories:
- `active/` - Current development areas
- `completed/` - Finished features
- `legacy/` - Historical documentation
- `status/` - Status tracking files
- `compliance/` - SOC2 and security compliance
- `architecture/` - Design decisions

## Development

### Building
```bash
npm run build
```

### Type Checking
```bash
npm run type-check
```

### Development Mode
```bash
npm run dev
```

## Example Usage with Claude Code

Once configured, you can use natural language to interact with the knowledge base:

- "What's the current implementation status of the Script language?"
- "Show me the known issues in the async implementation"
- "Update the known issues file with this new bug I found"
- "Search for all mentions of memory management in the docs"
- "List all files in the active development folder"

## Troubleshooting

### Server Not Starting
- Check that the `SCRIPT_PROJECT_ROOT` environment variable points to the correct directory
- Ensure the `kb/` directory exists in the project root
- Verify Node.js version is compatible (Node 18+ recommended)

### Permission Errors
- Ensure the server has read/write permissions to the `kb/` directory
- Check that the project root path is correct

### Claude Desktop Not Connecting
- Verify the configuration file path is correct for your OS
- Check that the server path in the config matches your installation
- Restart Claude Desktop after configuration changes

## Contributing

1. Make changes to the TypeScript source files in `src/`
2. Test your changes with `npm run dev`
3. Build the project with `npm run build`
4. Update documentation if needed

## License

MIT License - see the main Script project for details.