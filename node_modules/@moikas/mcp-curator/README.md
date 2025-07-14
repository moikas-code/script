# MCP Curator

[![npm version](https://badge.fury.io/js/@moikas%2Fmcp-curator.svg)](https://www.npmjs.com/package/@moikas/mcp-curator)
[![CI](https://github.com/moikas-code/mcp-curator/actions/workflows/ci.yml/badge.svg)](https://github.com/moikas-code/mcp-curator/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A powerful CLI tool for curating and managing MCP (Model Context Protocol) server configurations across global and project scopes. Seamlessly save, load, and manage multiple MCP configurations with intelligent template adaptation.

## Features

- 📦 Save and load multiple MCP configurations (global and project-level)
- 🔄 Switch between different setups quickly
- 🎯 Preset configurations (minimal, standard, full, development)
- 💾 Backup and restore configurations
- 🛠️ Interactive server management
- 🏗️ Project-level configuration templates
- 🌍 Cross-platform support (macOS, Windows, Linux)

## Installation

```bash
# Install globally from npm
npm install -g @moikas/mcp-curator

# Or clone and install locally
git clone https://github.com/moikas-code/mcp-curator.git
cd mcp-curator
npm install
npm link

# Initialize
mcp-curator init
```

## Quick Start

```bash
# Save your current Claude configuration
mcp-curator save my-config

# Apply a preset
mcp-curator preset development

# List saved configurations
mcp-curator list

# Load a saved configuration
mcp-curator load my-config

# Show current configuration
mcp-curator show
```

## Commands

### `mcp-curator init`
Initialize the tool and detect Claude configuration location.

### `mcp-curator save <name>`
Save the current Claude MCP configuration.
- Options:
  - `-d, --description <desc>`: Add a description

### `mcp-curator load <name>`
Load and apply a saved configuration.
- Options:
  - `-b, --backup`: Backup current config before loading

### `mcp-curator list`
List all saved configurations.
- Options:
  - `-a, --all`: Show all templates including built-in presets

### `mcp-curator show`
Display current Claude MCP configuration.
- Options:
  - `-j, --json`: Output as JSON
  - `-m, --merged`: Show merged configuration (global + project)

### `mcp-curator preset <type>`
Apply a preset configuration to global Claude Desktop:
- `minimal`: Basic filesystem access
- `standard`: Filesystem, memory, and sequential thinking
- `full`: All official MCP servers
- `development`: Development setup with code-audit and knowledge base

### `mcp-curator apply-project-template <name>`
Apply a project template to global configuration.
- Options:
  - `-b, --backup`: Backup current global config before applying

### `mcp-curator backup`
Backup current configuration.
- Options:
  - `-n, --name <name>`: Specify backup name

### `mcp-curator delete <name>`
Delete a saved configuration.
- Options:
  - `-f, --force`: Skip confirmation

### `mcp-curator add-server`
Interactive server configuration (coming soon).

## Project-Level Commands

### `mcp-curator project init`
Initialize project MCP configuration in `.mcp.json`.
- Options:
  - `-e, --example`: Create with example server
  - `-f, --force`: Overwrite existing configuration

### `mcp-curator project show`
Display current project MCP configuration.
- Options:
  - `-j, --json`: Output as JSON

### `mcp-curator project add <name>`
Add a server to project configuration.
- Options:
  - `-c, --command <cmd>`: Server command
  - `-a, --args <args>`: Command arguments (comma-separated)
  - `-e, --env <vars>`: Environment variables (KEY=value, comma-separated)
  - `-u, --url <url>`: SSE server URL
  - `-h, --headers <headers>`: HTTP headers (key:value, comma-separated)
  - `-l, --local`: Add to local config (.mcp.local.json) instead of shared
  - `-f, --force`: Overwrite existing server

### `mcp-curator project remove <name>`
Remove a server from project configuration.
- Options:
  - `-l, --local`: Remove from local config only
  - `-s, --shared`: Remove from shared config only
- Note: If server exists in both configs and no option is specified, it will be removed from both

### `mcp-curator project save <name>`
Save current project configuration as a template.
- Options:
  - `-d, --description <desc>`: Add a description for this template
  - `-f, --force`: Overwrite existing template

### `mcp-curator project load <name>`
Load a saved project template.
- Options:
  - `-b, --backup`: Backup current project config before loading

### `mcp-curator project list`
List saved project templates.

### `mcp-curator project delete <name>`
Delete a saved project template.
- Options:
  - `-f, --force`: Skip confirmation

### `mcp-curator project preset <type>`
Apply a global preset to project configuration.
- Options:
  - `-b, --backup`: Backup current project config before applying

## Project-Level Configuration

Claude Code supports project-level MCP configurations via `.mcp.json` files. These configurations are scoped to your project and can be committed to version control for team collaboration. Project configurations merge with global configurations, with project servers taking precedence.

### Local Configuration (.mcp.local.json)

In addition to shared project configuration, you can create a `.mcp.local.json` file for personal MCP servers that shouldn't be committed to version control. This is useful for:
- Personal development servers
- Local database connections
- API keys and sensitive configurations
- Experimental servers

The `.mcp.local.json` file:
- Is automatically added to `.gitignore`
- Takes precedence over `.mcp.json` for conflicting server names
- Is merged with shared configuration when Claude Code loads
- Uses the same format as `.mcp.json`

### Template Management
Save and load project configurations as reusable templates:
- `mcp-curator project save <name>` - Save current project config as template
- `mcp-curator project load <name>` - Load a saved template  
- `mcp-curator project list` - List all saved templates
- `mcp-curator project delete <name>` - Delete a template

## Example Workflows

### Global Configuration Management
```bash
# 1. Initialize the tool
mcp-curator init

# 2. Save your current setup
mcp-curator save work-setup -d "My work environment"

# 3. Try a development preset
mcp-curator preset development

# 4. Save the development setup
mcp-curator save dev-setup -d "Development with all tools"

# 5. Switch between configurations
mcp-curator load work-setup
# Restart Claude Code
mcp-curator load dev-setup
# Restart Claude Code

# 6. List all your configurations
mcp-curator list
```

### Project-Level Configuration
```bash
# 1. Apply a global preset to project (NEW!)
mcp-curator project preset standard -b

# 2. Add project-specific servers
mcp-curator project add db-server -c "npx" -a "my-db-mcp" -e "DB_URL=localhost"

# 3. Add personal/local servers (git-ignored)
mcp-curator project add my-local-api --local -c "node" -a "./api-server.js" -e "API_KEY=${MY_API_KEY}"

# 4. Save as template for similar projects
mcp-curator project save backend-template -d "Backend project with DB access"

# 5. In a new project, load the template
mcp-curator project load backend-template

# 6. View merged configuration (global + project + local)
mcp-curator show -m

# 7. List available project templates
mcp-curator project list
```

### Unified Template System
```bash
# 1. See everything available
mcp-curator list --all

# 2. Apply project template to global configuration (NEW!)
mcp-curator apply-project-template backend-template -b

# 3. Use any preset at project level (NEW!)
mcp-curator project preset development

# 4. Cross-scope template sharing
mcp-curator project save shared-template -d "Works for both global and project"
mcp-curator apply-project-template shared-template
```

## Configuration Storage

### Global Configurations
Saved configurations are stored in:
- **macOS**: `~/Library/Preferences/mcp-curator-nodejs/`
- **Windows**: `%APPDATA%\mcp-curator-nodejs\`
- **Linux**: `~/.config/mcp-curator-nodejs/`

### Project Configurations
Project-level configurations are stored in `.mcp.json` files at the project root. These files:
- Are scoped to the specific project
- Can be committed to version control for team collaboration
- Are merged with global configurations (project takes precedence)
- Support environment variable expansion (e.g., `${API_KEY}`)
- Are recognized automatically by Claude Code

### Project Templates
Project templates are stored alongside global configurations and can be reused across multiple projects.

### Configuration Hierarchy
When Claude Code loads MCP servers, it follows this precedence:
1. **Local scope** (project-specific user settings)
2. **Project scope** (`.mcp.json` - team shared)
3. **User scope** (global configuration)

Within project scope, `.mcp.local.json` takes precedence over `.mcp.json` for any conflicting server names.

## Adding Custom Servers

Edit your Claude configuration manually and save it:

```json
{
  "mcpServers": {
    "my-custom-server": {
      "command": "/path/to/server",
      "args": ["--arg1", "--arg2"],
      "env": {
        "MY_ENV": "value"
      }
    }
  }
}
```

Then save it as a configuration:
```bash
mcp-curator save custom-setup
```

## Claude Code Integration

MCP Curator is fully compatible with Claude Code's built-in MCP system:

1. **Project-level configs** (`.mcp.json`) created by MCP Curator are automatically recognized by Claude Code
2. **Use alongside `claude mcp` commands** - MCP Curator complements the built-in CLI
3. **Security prompts** - Claude Code will prompt for approval when using project-scoped servers
4. **Template system** - Quickly apply consistent configurations across projects

## Tips

1. **Always restart Claude Code** after loading a configuration
2. **Backup before experimenting** with `claude-mcp load -b <name>`
3. **Use descriptive names** for your configurations
4. **Document your setups** with descriptions

## Troubleshooting

### Configuration not found
Run `claude-mcp init` to detect your Claude installation.

### Changes not applying
Make sure to fully restart Claude Code after loading a configuration.

### Permission errors
The tool needs write access to your Claude configuration directory.

## License

MIT