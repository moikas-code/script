import fs from 'fs/promises';
import path from 'path';
import chalk from 'chalk';
import ora from 'ora';
import Conf from 'conf';

const config = new Conf({ projectName: 'mcp-curator' });

const presets = {
  minimal: {
    mcpServers: {
      "filesystem": {
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-filesystem", process.env.HOME],
        "env": {}
      }
    }
  },
  
  standard: {
    mcpServers: {
      "filesystem": {
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-filesystem", process.env.HOME],
        "env": {}
      },
      "memory": {
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-memory"],
        "env": {}
      },
      "sequential-thinking": {
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-sequential-thinking"],
        "env": {}
      }
    }
  },
  
  full: {
    mcpServers: {
      "filesystem": {
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-filesystem", process.env.HOME],
        "env": {}
      },
      "memory": {
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-memory"],
        "env": {}
      },
      "sequential-thinking": {
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-sequential-thinking"],
        "env": {}
      },
      "kb-mcp": {
        "command": "npx",
        "args": ["kb", "serve"],
        "env": {}
      }
    }
  },
  
  development: {
    mcpServers: {
      "filesystem": {
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-filesystem", path.join(process.env.HOME, "Documents")],
        "env": {}
      },
      "memory": {
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-memory"],
        "env": {}
      },
      "sequential-thinking": {
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-sequential-thinking"],
        "env": {}
      },
      "code-audit": {
        "command": "npx",
        "args": ["-y", "@moikas-code/code-audit-mcp", "start", "--stdio"],
        "env": {}
      },
      "kb-mcp": {
        "command": "npx",
        "args": ["kb", "serve"],
        "env": {}
      }
    }
  }
};

// Export function to get a preset configuration
export function getPreset(type) {
  return presets[type] || null;
}

export async function applyPreset(type) {
  if (!presets[type]) {
    console.log(chalk.red(`Unknown preset: ${type}`));
    console.log(chalk.gray('Available presets: minimal, standard, full, development'));
    return;
  }
  
  const spinner = ora(`Applying ${type} preset...`).start();
  
  try {
    const claudePath = config.get('claudeConfigPath') || getClaudeConfigPath();
    
    // Ensure directory exists
    await fs.mkdir(path.dirname(claudePath), { recursive: true });
    
    // Write preset configuration
    await fs.writeFile(claudePath, JSON.stringify(presets[type], null, 2));
    
    spinner.succeed(`${chalk.green(type)} preset applied!`);
    
    console.log(chalk.blue('\nIncluded servers:'));
    Object.keys(presets[type].mcpServers).forEach(server => {
      console.log(chalk.gray(`  - ${server}`));
    });
    
    console.log(chalk.yellow('\n🔄 Please restart Claude Code to apply changes.'));
    
    // Offer to save as named config
    console.log(chalk.gray(`\nTip: Save this preset with "mcp-curator save ${type}-config"`));
  } catch (error) {
    spinner.fail(`Failed to apply ${type} preset`);
    console.error(error);
  }
}

function getClaudeConfigPath() {
  const platform = process.platform;
  const homeDir = process.env.HOME || process.env.USERPROFILE;
  
  switch (platform) {
    case 'darwin':
      return path.join(homeDir, 'Library', 'Application Support', 'Claude', 'claude_desktop_config.json');
    case 'win32':
      return path.join(process.env.APPDATA || path.join(homeDir, 'AppData', 'Roaming'), 'Claude', 'claude_desktop_config.json');
    default:
      return path.join(homeDir, '.config', 'Claude', 'claude_desktop_config.json');
  }
}