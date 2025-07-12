import fs from 'fs/promises';
import path from 'path';
import os from 'os';
import chalk from 'chalk';
import ora from 'ora';
import inquirer from 'inquirer';
import Conf from 'conf';
import { readProjectConfig, mergeWithProjectConfig, adaptTemplateForScope } from './project-config.js';

// Initialize config store
const config = new Conf({
  projectName: 'mcp-curator',
  defaults: {
    configs: {},
    claudeConfigPath: null
  }
});

// Get Claude config path based on platform
function getClaudeConfigPath() {
  const platform = process.platform;
  const homeDir = os.homedir();
  
  switch (platform) {
    case 'darwin':
      return path.join(homeDir, 'Library', 'Application Support', 'Claude', 'claude_desktop_config.json');
    case 'win32':
      return path.join(process.env.APPDATA || path.join(homeDir, 'AppData', 'Roaming'), 'Claude', 'claude_desktop_config.json');
    default:
      return path.join(homeDir, '.config', 'Claude', 'claude_desktop_config.json');
  }
}

// Initialize the tool
export async function init() {
  const spinner = ora('Detecting Claude configuration...').start();
  
  try {
    const claudePath = getClaudeConfigPath();
    const exists = await fs.access(claudePath).then(() => true).catch(() => false);
    
    if (exists) {
      config.set('claudeConfigPath', claudePath);
      spinner.succeed(`Claude config found at: ${chalk.green(claudePath)}`);
    } else {
      spinner.warn('Claude config not found. Will create when needed.');
      config.set('claudeConfigPath', claudePath);
    }
    
    console.log(chalk.blue('\n✨ Claude MCP Manager initialized!'));
  } catch (error) {
    spinner.fail('Initialization failed');
    console.error(error);
  }
}

// List all saved configurations
export async function listConfigs(options = {}) {
  const configs = config.get('configs', {});
  const projectTemplates = config.get('projectTemplates', {});
  const configNames = Object.keys(configs);
  const templateNames = Object.keys(projectTemplates);
  
  if (configNames.length === 0 && templateNames.length === 0 && !options.all) {
    console.log(chalk.yellow('No saved configurations found.'));
    console.log(chalk.gray('Use "mcp-curator save <name>" to save your first configuration.'));
    return;
  }
  
  if (options.all || configNames.length > 0) {
    console.log(chalk.blue('\n📦 Global Configurations:\n'));
    
    if (configNames.length === 0) {
      console.log(chalk.gray('  No global configurations saved'));
    } else {
      configNames.forEach(name => {
        const cfg = configs[name];
        const serverCount = Object.keys(cfg.mcpServers || {}).length;
        console.log(chalk.green(`  ${name}`));
        console.log(chalk.gray(`    Servers: ${serverCount}`));
        if (cfg.description) {
          console.log(chalk.gray(`    Description: ${cfg.description}`));
        }
        console.log(chalk.gray(`    Saved: ${new Date(cfg.savedAt).toLocaleString()}`));
        console.log();
      });
    }
  }
  
  if (options.all || templateNames.length > 0) {
    console.log(chalk.blue('\n🏗️ Project Templates:\n'));
    
    if (templateNames.length === 0) {
      console.log(chalk.gray('  No project templates saved'));
    } else {
      templateNames.forEach(name => {
        const template = projectTemplates[name];
        const serverCount = template.servers?.length || 0;
        const created = new Date(template.created).toLocaleDateString();
        
        console.log(chalk.green(`  ${name}`));
        
        if (template.description) {
          console.log(chalk.gray(`    Description: ${template.description}`));
        }
        
        console.log(chalk.gray(`    Servers: ${serverCount} (${template.servers?.join(', ') || 'none'})`));
        console.log(chalk.gray(`    Created: ${created}`));
        console.log();
      });
    }
  }
  
  if (options.all) {
    console.log(chalk.blue('\n🎯 Built-in Presets:\n'));
    const presets = ['minimal', 'standard', 'full', 'development'];
    presets.forEach(preset => {
      console.log(chalk.green(`  ${preset}`));
      console.log(chalk.gray(`    Built-in preset`));
      console.log(chalk.gray(`    Global: "mcp-curator preset ${preset}"`));
      console.log(chalk.gray(`    Project: "mcp-curator project preset ${preset}"`));
      console.log();
    });
  }
  
  if (options.all || (configNames.length > 0 && templateNames.length > 0)) {
    console.log(chalk.gray('\nCross-scope commands:'));
    console.log(chalk.gray('  Apply project template globally: "mcp-curator apply-project-template <name>"'));
  }
}

// Save current configuration
export async function saveConfig(name, options) {
  const spinner = ora('Reading current Claude configuration...').start();
  
  try {
    const claudePath = config.get('claudeConfigPath') || getClaudeConfigPath();
    const claudeConfig = JSON.parse(await fs.readFile(claudePath, 'utf8'));
    
    const savedConfig = {
      ...claudeConfig,
      description: options.description,
      savedAt: new Date().toISOString()
    };
    
    const configs = config.get('configs', {});
    configs[name] = savedConfig;
    config.set('configs', configs);
    
    spinner.succeed(`Configuration saved as "${chalk.green(name)}"`);
  } catch (error) {
    spinner.fail('Failed to save configuration');
    if (error.code === 'ENOENT') {
      console.log(chalk.yellow('No Claude configuration found. Start Claude Code first.'));
    } else {
      console.error(error);
    }
  }
}

// Load a saved configuration
export async function loadConfig(name, options) {
  const configs = config.get('configs', {});
  
  if (!configs[name]) {
    console.log(chalk.red(`Configuration "${name}" not found.`));
    console.log(chalk.gray('Use "mcp-curator list" to see available configurations.'));
    return;
  }
  
  if (options.backup) {
    await backupCurrent({ name: `backup-${Date.now()}` });
  }
  
  const spinner = ora(`Loading configuration "${name}"...`).start();
  
  try {
    const claudePath = config.get('claudeConfigPath') || getClaudeConfigPath();
    const configToLoad = { ...configs[name] };
    
    // Remove metadata before writing
    delete configToLoad.description;
    delete configToLoad.savedAt;
    
    // Ensure directory exists
    await fs.mkdir(path.dirname(claudePath), { recursive: true });
    
    // Write configuration
    await fs.writeFile(claudePath, JSON.stringify(configToLoad, null, 2));
    
    spinner.succeed(`Configuration "${chalk.green(name)}" loaded successfully!`);
    console.log(chalk.yellow('\n🔄 Please restart Claude Code to apply changes.'));
  } catch (error) {
    spinner.fail('Failed to load configuration');
    console.error(error);
  }
}

// Apply current configuration to Claude
export async function applyConfig(options) {
  // This is handled by load command
  console.log(chalk.yellow('Use "mcp-curator load <name>" to apply a saved configuration.'));
}

// Delete a saved configuration
export async function deleteConfig(name, options) {
  const configs = config.get('configs', {});
  
  if (!configs[name]) {
    console.log(chalk.red(`Configuration "${name}" not found.`));
    return;
  }
  
  if (!options.force) {
    const { confirm } = await inquirer.prompt([{
      type: 'confirm',
      name: 'confirm',
      message: `Delete configuration "${name}"?`,
      default: false
    }]);
    
    if (!confirm) {
      console.log('Cancelled.');
      return;
    }
  }
  
  delete configs[name];
  config.set('configs', configs);
  console.log(chalk.green(`Configuration "${name}" deleted.`));
}

// Add a new server interactively
export async function addServer() {
  const answers = await inquirer.prompt([
    {
      type: 'input',
      name: 'name',
      message: 'Server name:',
      validate: input => input.length > 0
    },
    {
      type: 'list',
      name: 'type',
      message: 'Server type:',
      choices: [
        { name: 'Local executable', value: 'local' },
        { name: 'NPX package', value: 'npx' },
        { name: 'Custom command', value: 'custom' }
      ]
    }
  ]);
  
  let serverConfig;
  
  switch (answers.type) {
    case 'local':
      const localAnswers = await inquirer.prompt([
        {
          type: 'input',
          name: 'command',
          message: 'Path to executable:',
          validate: input => input.length > 0
        },
        {
          type: 'input',
          name: 'args',
          message: 'Arguments (comma-separated):',
          filter: input => input ? input.split(',').map(s => s.trim()) : []
        }
      ]);
      serverConfig = {
        command: localAnswers.command,
        args: localAnswers.args,
        env: {}
      };
      break;
      
    case 'npx':
      const npxAnswers = await inquirer.prompt([
        {
          type: 'input',
          name: 'package',
          message: 'NPM package name:',
          validate: input => input.length > 0
        },
        {
          type: 'input',
          name: 'args',
          message: 'Additional arguments (comma-separated):',
          filter: input => input ? input.split(',').map(s => s.trim()) : []
        }
      ]);
      serverConfig = {
        command: 'npx',
        args: ['-y', npxAnswers.package, ...npxAnswers.args],
        env: {}
      };
      break;
      
    case 'custom':
      const customAnswers = await inquirer.prompt([
        {
          type: 'input',
          name: 'command',
          message: 'Command:',
          validate: input => input.length > 0
        },
        {
          type: 'input',
          name: 'args',
          message: 'Arguments (comma-separated):',
          filter: input => input ? input.split(',').map(s => s.trim()) : []
        }
      ]);
      serverConfig = {
        command: customAnswers.command,
        args: customAnswers.args,
        env: {}
      };
      break;
  }
  
  // Add environment variables if needed
  const { hasEnv } = await inquirer.prompt([{
    type: 'confirm',
    name: 'hasEnv',
    message: 'Add environment variables?',
    default: false
  }]);
  
  if (hasEnv) {
    const { envVars } = await inquirer.prompt([{
      type: 'input',
      name: 'envVars',
      message: 'Environment variables (KEY=value, comma-separated):',
      filter: input => {
        const env = {};
        input.split(',').forEach(pair => {
          const [key, value] = pair.trim().split('=');
          if (key && value) env[key] = value;
        });
        return env;
      }
    }]);
    serverConfig.env = envVars;
  }
  
  console.log(chalk.green(`\nServer configuration for "${answers.name}":`));
  console.log(JSON.stringify(serverConfig, null, 2));
  
  const { save } = await inquirer.prompt([{
    type: 'confirm',
    name: 'save',
    message: 'Add this server to a configuration?',
    default: true
  }]);
  
  if (save) {
    // TODO: Implement adding to specific config
    console.log(chalk.yellow('To add this server, edit your configuration manually or load a config first.'));
  }
}

// Remove a server
export async function removeServer(name) {
  console.log(chalk.yellow(`Remove server "${name}" - Not implemented yet`));
  console.log(chalk.gray('Load a configuration, edit it manually, then save it again.'));
}

// Show current configuration
export async function showCurrent(options) {
  const spinner = ora('Reading current Claude configuration...').start();
  
  try {
    const claudePath = config.get('claudeConfigPath') || getClaudeConfigPath();
    let claudeConfig = JSON.parse(await fs.readFile(claudePath, 'utf8'));
    
    // Check for project configuration
    const projectConfig = await readProjectConfig();
    const hasProjectConfig = !!projectConfig;
    
    // Merge with project config if requested
    if (options.merged && hasProjectConfig) {
      claudeConfig = await mergeWithProjectConfig(claudeConfig);
    }
    
    spinner.stop();
    
    if (options.json) {
      console.log(JSON.stringify(claudeConfig, null, 2));
    } else {
      console.log(chalk.blue('\n📋 Current Claude MCP Configuration:\n'));
      
      if (hasProjectConfig && !options.merged) {
        console.log(chalk.yellow('ℹ️  Project configuration detected. Use --merged to see combined config.\n'));
      }
      
      const servers = claudeConfig.mcpServers || {};
      const serverNames = Object.keys(servers);
      
      if (serverNames.length === 0) {
        console.log(chalk.yellow('No MCP servers configured.'));
      } else {
        serverNames.forEach(name => {
          console.log(chalk.green(`  ${name}`));
          const server = servers[name];
          
          if (server.command) {
            console.log(chalk.gray(`    Command: ${server.command}`));
            if (server.args?.length > 0) {
              console.log(chalk.gray(`    Args: ${server.args.join(' ')}`));
            }
          } else if (server.type === 'sse' && server.url) {
            console.log(chalk.gray(`    Type: Server-Sent Events (SSE)`));
            console.log(chalk.gray(`    URL: ${server.url}`));
          }
          
          console.log();
        });
      }
      
      if (hasProjectConfig && options.merged) {
        console.log(chalk.gray('\n✅ Showing merged configuration (global + project)'));
      }
    }
  } catch (error) {
    spinner.fail('Failed to read current configuration');
    if (error.code === 'ENOENT') {
      console.log(chalk.yellow('No Claude configuration found. Start Claude Code first.'));
    } else {
      console.error(error);
    }
  }
}

// Backup current configuration
export async function backupCurrent(options) {
  const name = options.name || `backup-${new Date().toISOString().replace(/[:.]/g, '-')}`;
  await saveConfig(name, { description: 'Automatic backup' });
}

// Apply a project template to global configuration
export async function applyProjectTemplateGlobally(name, options = {}) {
  const spinner = ora('Loading project template for global use...').start();
  
  try {
    const templates = config.get('projectTemplates', {});
    
    if (!templates[name]) {
      spinner.fail(`Project template "${name}" not found`);
      console.log(chalk.gray('Use "mcp-curator project list" to see available templates'));
      return;
    }
    
    // Backup current global config if requested
    if (options.backup) {
      const backupName = `backup-${Date.now()}`;
      await saveConfig(backupName, { description: 'Auto-backup before applying project template' });
      console.log(chalk.blue(`Current global config backed up as "${backupName}"`));
    }
    
    const template = templates[name];
    
    // Adapt template for global scope
    const adaptedTemplate = adaptTemplateForScope(template.config, 'global');
    
    // Apply to global configuration
    const claudePath = config.get('claudeConfigPath') || getClaudeConfigPath();
    
    // Ensure directory exists
    await fs.mkdir(path.dirname(claudePath), { recursive: true });
    
    // Write configuration
    await fs.writeFile(claudePath, JSON.stringify(adaptedTemplate, null, 2));
    
    spinner.succeed(`Project template "${chalk.green(name)}" applied globally!`);
    
    if (template.description) {
      console.log(chalk.gray(`Description: ${template.description}`));
    }
    
    console.log(chalk.blue('\nIncluded servers:'));
    template.servers.forEach(server => {
      console.log(chalk.gray(`  - ${server}`));
    });
    
    console.log(chalk.yellow('\n🔄 Please restart Claude Code to apply global configuration changes.'));
    
    // Offer to save as named global config
    console.log(chalk.gray(`\nTip: Save this configuration with "mcp-curator save ${name}-global"`));
  } catch (error) {
    spinner.fail('Failed to apply project template globally');
    console.error(error);
  }
}

// Update mcp-curator to latest version
export async function updateSelf(options) {
  const spinner = ora('Checking for updates...').start();
  
  try {
    const { execSync } = await import('child_process');
    const packageJson = JSON.parse(await fs.readFile(new URL('../package.json', import.meta.url), 'utf-8'));
    const currentVersion = packageJson.version;
    const packageName = packageJson.name;
    
    // Get latest version from npm
    let latestVersion;
    try {
      const result = execSync(`npm view ${packageName} version`, { encoding: 'utf-8' });
      latestVersion = result.trim();
    } catch (error) {
      spinner.fail('Failed to check for updates');
      console.error(chalk.red('Error: Could not fetch latest version from npm'));
      return;
    }
    
    spinner.succeed();
    console.log(`Current version: ${chalk.cyan(currentVersion)}`);
    console.log(`Latest version: ${chalk.green(latestVersion)}`);
    
    if (options.check) {
      // Just check, don't update
      if (currentVersion === latestVersion) {
        console.log(chalk.green('\n✨ You are running the latest version!'));
      } else {
        console.log(chalk.yellow(`\n📦 Update available: ${currentVersion} → ${latestVersion}`));
        console.log(chalk.gray('Run "mcp-curator update" to install'));
      }
      return;
    }
    
    if (currentVersion === latestVersion) {
      console.log(chalk.green('\n✨ Already up to date!'));
      return;
    }
    
    // Prompt for confirmation
    const { confirmUpdate } = await inquirer.prompt([{
      type: 'confirm',
      name: 'confirmUpdate',
      message: `Update to version ${latestVersion}?`,
      default: true
    }]);
    
    if (!confirmUpdate) {
      console.log(chalk.gray('Update cancelled'));
      return;
    }
    
    // Perform update
    const updateSpinner = ora('Updating mcp-curator...').start();
    
    try {
      // Check if installed globally or locally
      let isGlobal = true;
      try {
        execSync('npm list -g @moikas/mcp-curator', { encoding: 'utf-8' });
      } catch {
        isGlobal = false;
      }
      
      const command = isGlobal 
        ? `npm update -g ${packageName}`
        : `npm update ${packageName}`;
      
      execSync(command, { stdio: 'inherit' });
      
      updateSpinner.succeed('Update complete!');
      console.log(chalk.green(`\n✅ Successfully updated to version ${latestVersion}`));
      console.log(chalk.gray('You may need to restart your terminal for changes to take effect'));
      
    } catch (error) {
      updateSpinner.fail('Update failed');
      console.error(chalk.red('\nError updating:'), error.message);
      console.log(chalk.yellow('\nTry updating manually with:'));
      console.log(chalk.gray(`  npm install -g ${packageName}@latest`));
    }
    
  } catch (error) {
    spinner.fail('Update check failed');
    console.error(error);
  }
}