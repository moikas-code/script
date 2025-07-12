import fs from 'fs/promises';
import path from 'path';
import chalk from 'chalk';
import ora from 'ora';
import Conf from 'conf';

const config = new Conf({ projectName: 'mcp-curator' });

// Template adaptation utilities
export function adaptTemplateForScope(template, targetScope) {
  if (!template || !template.mcpServers) {
    return template;
  }
  
  const adapted = JSON.parse(JSON.stringify(template)); // Deep clone
  
  Object.keys(adapted.mcpServers).forEach(serverName => {
    const server = adapted.mcpServers[serverName];
    
    if (server.args && Array.isArray(server.args)) {
      server.args = server.args.map(arg => {
        if (targetScope === 'project') {
          // Global → Project: Replace HOME-based paths with current directory
          if (arg === process.env.HOME) return '.';
          if (arg.startsWith(path.join(process.env.HOME, 'Documents'))) {
            return '.';
          }
        } else if (targetScope === 'global') {
          // Project → Global: Replace current directory with HOME
          if (arg === '.') return process.env.HOME;
          if (arg === './') return process.env.HOME;
        }
        return arg;
      });
    }
    
    // Adapt command paths if needed
    if (server.command && targetScope === 'project') {
      // Convert absolute paths to relative if they're in common locations
      if (server.command.startsWith(path.join(process.env.HOME, 'Documents/code/'))) {
        // Keep absolute paths for project scope - team members might have different structures
      }
    }
  });
  
  return adapted;
}

// Check if we're in a project with .mcp.json
export async function findProjectConfig() {
  let currentDir = process.cwd();
  
  while (currentDir !== path.dirname(currentDir)) {
    const mcpPath = path.join(currentDir, '.mcp.json');
    
    try {
      await fs.access(mcpPath);
      return mcpPath;
    } catch {
      // Continue searching up the directory tree
    }
    
    currentDir = path.dirname(currentDir);
  }
  
  return null;
}

// Find local project config (.mcp.local.json)
export async function findLocalProjectConfig() {
  let currentDir = process.cwd();
  
  while (currentDir !== path.dirname(currentDir)) {
    const localPath = path.join(currentDir, '.mcp.local.json');
    
    try {
      await fs.access(localPath);
      return localPath;
    } catch {
      // Continue searching up the directory tree
    }
    
    currentDir = path.dirname(currentDir);
  }
  
  return null;
}

// Read project-level MCP configuration (merges .mcp.json and .mcp.local.json)
export async function readProjectConfig() {
  const configPath = await findProjectConfig();
  const localConfigPath = await findLocalProjectConfig();
  
  let config = { mcpServers: {} };
  let paths = [];
  
  // Read shared project config
  if (configPath) {
    try {
      const content = await fs.readFile(configPath, 'utf8');
      const projectConfig = JSON.parse(content);
      config = {
        ...config,
        ...projectConfig,
        mcpServers: {
          ...config.mcpServers,
          ...projectConfig.mcpServers
        }
      };
      paths.push(configPath);
    } catch (error) {
      console.warn(`Warning: Failed to read ${configPath}: ${error.message}`);
    }
  }
  
  // Read local config and merge (local takes precedence)
  if (localConfigPath) {
    try {
      const content = await fs.readFile(localConfigPath, 'utf8');
      const localConfig = JSON.parse(content);
      config = {
        ...config,
        ...localConfig,
        mcpServers: {
          ...config.mcpServers,
          ...localConfig.mcpServers
        }
      };
      paths.push(localConfigPath);
    } catch (error) {
      console.warn(`Warning: Failed to read ${localConfigPath}: ${error.message}`);
    }
  }
  
  if (paths.length === 0) {
    return null;
  }
  
  return {
    path: paths.join(', '),
    config,
    hasLocal: !!localConfigPath,
    hasShared: !!configPath
  };
}

// Save project-level MCP configuration
export async function saveProjectConfig(config, projectPath = process.cwd()) {
  const configPath = path.join(projectPath, '.mcp.json');
  
  try {
    await fs.writeFile(configPath, JSON.stringify(config, null, 2));
    return configPath;
  } catch (error) {
    throw new Error(`Failed to save project config: ${error.message}`);
  }
}

// Save local project configuration (.mcp.local.json)
export async function saveLocalProjectConfig(config, projectPath = process.cwd()) {
  const configPath = path.join(projectPath, '.mcp.local.json');
  
  try {
    await fs.writeFile(configPath, JSON.stringify(config, null, 2));
    return configPath;
  } catch (error) {
    throw new Error(`Failed to save local project config: ${error.message}`);
  }
}

// Show project configuration
export async function showProjectConfig(options) {
  const spinner = ora('Reading project MCP configuration...').start();
  
  try {
    const projectConfig = await readProjectConfig();
    
    if (!projectConfig) {
      spinner.warn('No project MCP configuration found (.mcp.json)');
      console.log(chalk.gray('Create one with "mcp-curator project init"'));
      return;
    }
    
    spinner.succeed('Project configurations found');
    
    if (projectConfig.hasShared) {
      console.log(chalk.green(`  Shared config: .mcp.json`));
    }
    if (projectConfig.hasLocal) {
      console.log(chalk.blue(`  Local config: .mcp.local.json (git-ignored)`));
    }
    
    if (options.json) {
      console.log(JSON.stringify(projectConfig.config, null, 2));
    } else {
      console.log(chalk.blue('\n📋 Merged Project MCP Configuration:\n'));
      const servers = projectConfig.config.mcpServers || {};
      const serverNames = Object.keys(servers);
      
      if (serverNames.length === 0) {
        console.log(chalk.yellow('No MCP servers configured in project.'));
      } else {
        // Read individual configs to show source
        const sharedConfig = projectConfig.hasShared ? 
          JSON.parse(await fs.readFile(await findProjectConfig(), 'utf8')) : { mcpServers: {} };
        const localConfig = projectConfig.hasLocal ? 
          JSON.parse(await fs.readFile(await findLocalProjectConfig(), 'utf8')) : { mcpServers: {} };
        
        serverNames.forEach(name => {
          const isLocal = localConfig.mcpServers && localConfig.mcpServers[name];
          const isShared = sharedConfig.mcpServers && sharedConfig.mcpServers[name];
          const source = isLocal && isShared ? 'local override' : (isLocal ? 'local' : 'shared');
          
          console.log(chalk.green(`  ${name}`) + chalk.gray(` (${source})`));
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
          
          if (server.env && Object.keys(server.env).length > 0) {
            console.log(chalk.gray(`    Environment: ${Object.keys(server.env).join(', ')}`));
          }
          console.log();
        });
      }
    }
  } catch (error) {
    spinner.fail('Failed to read project configuration');
    console.error(error);
  }
}

// Initialize project configuration
export async function initProjectConfig(options) {
  const spinner = ora('Initializing project MCP configuration...').start();
  
  try {
    const existingConfig = await readProjectConfig();
    
    if (existingConfig && !options.force) {
      spinner.warn('Project configuration already exists');
      console.log(chalk.gray(`Found at: ${existingConfig.path}`));
      console.log(chalk.gray('Use --force to overwrite'));
      return;
    }
    
    const config = {
      mcpServers: {}
    };
    
    if (options.example) {
      // Add example server
      config.mcpServers['example-filesystem'] = {
        command: 'npx',
        args: ['-y', '@modelcontextprotocol/server-filesystem', '.'],
        env: {}
      };
    }
    
    const configPath = await saveProjectConfig(config);
    spinner.succeed(`Project config created at: ${chalk.green(configPath)}`);
    
    if (!options.example) {
      console.log(chalk.gray('\nAdd servers with "mcp-curator project add <name>"'));
    }
  } catch (error) {
    spinner.fail('Failed to initialize project configuration');
    console.error(error);
  }
}

// Add server to project configuration
export async function addProjectServer(name, options) {
  const spinner = ora('Adding server to project configuration...').start();
  
  try {
    // Determine target file
    const isLocal = options.local;
    const targetFile = isLocal ? '.mcp.local.json' : '.mcp.json';
    
    // Read the specific config file
    const configPath = isLocal ? await findLocalProjectConfig() : await findProjectConfig();
    let config = { mcpServers: {} };
    
    if (configPath) {
      try {
        const content = await fs.readFile(configPath, 'utf8');
        config = JSON.parse(content);
      } catch {
        // File might not exist yet
      }
    }
    
    // Initialize mcpServers if not present
    if (!config.mcpServers) {
      config.mcpServers = {};
    }
    
    if (config.mcpServers[name] && !options.force) {
      spinner.warn(`Server "${name}" already exists in ${targetFile}`);
      console.log(chalk.gray('Use --force to overwrite'));
      return;
    }
    
    // Build server configuration
    const serverConfig = {};
    
    if (options.url) {
      // SSE server configuration
      serverConfig.type = 'sse';
      serverConfig.url = options.url;
      
      if (options.headers) {
        serverConfig.headers = {};
        options.headers.split(',').forEach(header => {
          const [key, value] = header.split(':').map(s => s.trim());
          if (key && value) {
            serverConfig.headers[key] = value;
          }
        });
      }
    } else {
      // Command-based server configuration
      serverConfig.command = options.command;
      serverConfig.args = options.args ? options.args.split(',').map(arg => arg.trim()) : [];
      serverConfig.env = {};
      
      if (options.env) {
        options.env.split(',').forEach(envVar => {
          const [key, value] = envVar.split('=');
          if (key && value) {
            serverConfig.env[key] = value;
          }
        });
      }
    }
    
    // Add to configuration
    config.mcpServers[name] = serverConfig;
    
    // Save configuration to appropriate file
    if (isLocal) {
      await saveLocalProjectConfig(config);
      await updateGitignore();
      spinner.succeed(`Server "${chalk.green(name)}" added to local project configuration`);
      console.log(chalk.blue('\nℹ️  This server is only available to you (.mcp.local.json is git-ignored).'));
    } else {
      await saveProjectConfig(config);
      spinner.succeed(`Server "${chalk.green(name)}" added to shared project configuration`);
      console.log(chalk.yellow('\n⚠️  Project members will need to approve this server when they first use it.'));
    }
  } catch (error) {
    spinner.fail('Failed to add server');
    console.error(error);
  }
}

// Remove server from project configuration
export async function removeProjectServer(name, options = {}) {
  const spinner = ora('Removing server from project configuration...').start();
  
  try {
    // Read individual config files to determine where the server exists
    const sharedPath = await findProjectConfig();
    const localPath = await findLocalProjectConfig();
    
    let sharedConfig = { mcpServers: {} };
    let localConfig = { mcpServers: {} };
    let foundInShared = false;
    let foundInLocal = false;
    
    if (sharedPath) {
      try {
        const content = await fs.readFile(sharedPath, 'utf8');
        sharedConfig = JSON.parse(content);
        foundInShared = !!(sharedConfig.mcpServers && sharedConfig.mcpServers[name]);
      } catch {}
    }
    
    if (localPath) {
      try {
        const content = await fs.readFile(localPath, 'utf8');
        localConfig = JSON.parse(content);
        foundInLocal = !!(localConfig.mcpServers && localConfig.mcpServers[name]);
      } catch {}
    }
    
    if (!foundInShared && !foundInLocal) {
      spinner.fail(`Server "${name}" not found in project configuration`);
      return;
    }
    
    // If server exists in both, ask which to remove (unless --local or --shared specified)
    if (foundInShared && foundInLocal && !options.local && !options.shared) {
      spinner.info(`Server "${name}" exists in both shared and local configs`);
      console.log(chalk.yellow('Removing from both configurations...'));
      
      // Remove from both
      delete sharedConfig.mcpServers[name];
      delete localConfig.mcpServers[name];
      
      await saveProjectConfig(sharedConfig);
      await saveLocalProjectConfig(localConfig);
      
      spinner.succeed(`Server "${chalk.green(name)}" removed from both shared and local configurations`);
    } else if (options.local || (!options.shared && foundInLocal && !foundInShared)) {
      // Remove from local only
      if (!foundInLocal) {
        spinner.fail(`Server "${name}" not found in local configuration`);
        return;
      }
      
      delete localConfig.mcpServers[name];
      await saveLocalProjectConfig(localConfig);
      
      spinner.succeed(`Server "${chalk.green(name)}" removed from local configuration`);
    } else {
      // Remove from shared only
      if (!foundInShared) {
        spinner.fail(`Server "${name}" not found in shared configuration`);
        return;
      }
      
      delete sharedConfig.mcpServers[name];
      await saveProjectConfig(sharedConfig);
      
      spinner.succeed(`Server "${chalk.green(name)}" removed from shared configuration`);
    }
  } catch (error) {
    spinner.fail('Failed to remove server');
    console.error(error);
  }
}

// Merge project configuration with global configuration
export async function mergeWithProjectConfig(globalConfig) {
  const projectConfig = await readProjectConfig();
  
  if (!projectConfig) {
    return globalConfig;
  }
  
  // Create merged configuration with project servers taking precedence
  const merged = {
    ...globalConfig,
    mcpServers: {
      ...globalConfig.mcpServers,
      ...projectConfig.config.mcpServers
    }
  };
  
  return merged;
}

// Add .mcp.local.json to .gitignore if not already present
export async function updateGitignore() {
  const gitignorePath = path.join(process.cwd(), '.gitignore');
  const entry = '.mcp.local.json';
  
  try {
    let content = '';
    try {
      content = await fs.readFile(gitignorePath, 'utf8');
    } catch {
      // .gitignore doesn't exist yet
    }
    
    // Check if entry already exists
    const lines = content.split('\n');
    if (lines.some(line => line.trim() === entry)) {
      console.log(chalk.gray(`✓ ${entry} already in .gitignore`));
      return;
    }
    
    // Add entry
    const newContent = content.trim() + (content.trim() ? '\n\n' : '') + 
      '# Local MCP configuration (user-specific)\n' + entry + '\n';
    
    await fs.writeFile(gitignorePath, newContent);
    console.log(chalk.green(`✓ Added ${entry} to .gitignore`));
  } catch (error) {
    console.error(chalk.red(`Failed to update .gitignore: ${error.message}`));
  }
}

// Save current project configuration as a template
export async function saveProjectTemplate(name, options = {}) {
  const spinner = ora('Saving project template...').start();
  
  try {
    const projectConfig = await readProjectConfig();
    
    if (!projectConfig) {
      spinner.fail('No project configuration found to save');
      console.log(chalk.gray('Create one with "mcp-curator project init"'));
      return;
    }
    
    const templates = config.get('projectTemplates', {});
    
    if (templates[name] && !options.force) {
      spinner.warn(`Template "${name}" already exists`);
      console.log(chalk.gray('Use --force to overwrite'));
      return;
    }
    
    templates[name] = {
      config: projectConfig.config,
      description: options.description || '',
      created: new Date().toISOString(),
      servers: Object.keys(projectConfig.config.mcpServers || {})
    };
    
    config.set('projectTemplates', templates);
    
    spinner.succeed(`Template "${chalk.green(name)}" saved`);
    
    if (options.description) {
      console.log(chalk.gray(`Description: ${options.description}`));
    }
    
    const serverCount = templates[name].servers.length;
    console.log(chalk.gray(`Servers: ${serverCount} (${templates[name].servers.join(', ')})`));
  } catch (error) {
    spinner.fail('Failed to save project template');
    console.error(error);
  }
}

// Load a project template
export async function loadProjectTemplate(name, options = {}) {
  const spinner = ora('Loading project template...').start();
  
  try {
    const templates = config.get('projectTemplates', {});
    
    if (!templates[name]) {
      spinner.fail(`Template "${name}" not found`);
      console.log(chalk.gray('Use "mcp-curator project list" to see available templates'));
      return;
    }
    
    // Backup current config if requested
    if (options.backup) {
      const currentConfig = await readProjectConfig();
      if (currentConfig) {
        const backupName = `backup-${Date.now()}`;
        await saveProjectTemplate(backupName, { description: 'Auto-backup before loading template' });
        console.log(chalk.blue(`Current config backed up as "${backupName}"`));
      }
    }
    
    const template = templates[name];
    const configPath = await saveProjectConfig(template.config);
    
    spinner.succeed(`Template "${chalk.green(name)}" loaded`);
    console.log(chalk.gray(`Config saved to: ${configPath}`));
    
    if (template.description) {
      console.log(chalk.gray(`Description: ${template.description}`));
    }
    
    console.log(chalk.blue('\nLoaded servers:'));
    template.servers.forEach(server => {
      console.log(chalk.gray(`  - ${server}`));
    });
    
    console.log(chalk.yellow('\n🔄 Restart Claude Code to apply project configuration changes.'));
  } catch (error) {
    spinner.fail('Failed to load project template');
    console.error(error);
  }
}

// List saved project templates
export async function listProjectTemplates() {
  const spinner = ora('Loading project templates...').start();
  
  try {
    const templates = config.get('projectTemplates', {});
    const templateNames = Object.keys(templates);
    
    if (templateNames.length === 0) {
      spinner.warn('No project templates saved');
      console.log(chalk.gray('Save a template with "mcp-curator project save <name>"'));
      return;
    }
    
    spinner.succeed(`Found ${templateNames.length} project template${templateNames.length === 1 ? '' : 's'}`);
    
    console.log(chalk.blue('\n📦 Saved Project Templates:\n'));
    
    templateNames.forEach(name => {
      const template = templates[name];
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
    
    console.log(chalk.gray('Load a template with "mcp-curator project load <name>"'));
  } catch (error) {
    spinner.fail('Failed to list project templates');
    console.error(error);
  }
}

// Delete a project template
export async function deleteProjectTemplate(name, options = {}) {
  const spinner = ora('Deleting project template...').start();
  
  try {
    const templates = config.get('projectTemplates', {});
    
    if (!templates[name]) {
      spinner.fail(`Template "${name}" not found`);
      return;
    }
    
    if (!options.force) {
      spinner.stop();
      console.log(chalk.yellow(`Are you sure you want to delete template "${name}"? (y/N)`));
      // Note: In a real implementation, you'd want to add readline for confirmation
      // For now, require --force flag
      console.log(chalk.gray('Use --force to skip confirmation'));
      return;
    }
    
    delete templates[name];
    config.set('projectTemplates', templates);
    
    spinner.succeed(`Template "${chalk.green(name)}" deleted`);
  } catch (error) {
    spinner.fail('Failed to delete project template');
    console.error(error);
  }
}

// Apply a global preset to project configuration
export async function applyProjectPreset(type, options = {}) {
  const spinner = ora(`Applying ${type} preset to project...`).start();
  
  try {
    // Import presets from presets.js
    const { getPreset } = await import('./presets.js');
    const preset = getPreset(type);
    
    if (!preset) {
      spinner.fail(`Unknown preset: ${type}`);
      console.log(chalk.gray('Available presets: minimal, standard, full, development'));
      return;
    }
    
    // Backup current config if requested
    if (options.backup) {
      const currentConfig = await readProjectConfig();
      if (currentConfig) {
        const backupName = `backup-${Date.now()}`;
        await saveProjectTemplate(backupName, { description: 'Auto-backup before applying preset' });
        console.log(chalk.blue(`Current config backed up as "${backupName}"`));
      }
    }
    
    // Adapt preset for project scope
    const adaptedPreset = adaptTemplateForScope(preset, 'project');
    
    // Save the adapted preset to project configuration
    const configPath = await saveProjectConfig(adaptedPreset);
    
    spinner.succeed(`${chalk.green(type)} preset applied to project!`);
    console.log(chalk.gray(`Config saved to: ${configPath}`));
    
    console.log(chalk.blue('\nIncluded servers:'));
    Object.keys(adaptedPreset.mcpServers).forEach(server => {
      console.log(chalk.gray(`  - ${server}`));
    });
    
    console.log(chalk.yellow('\n🔄 Restart Claude Code to apply project configuration changes.'));
  } catch (error) {
    spinner.fail(`Failed to apply ${type} preset to project`);
    console.error(error);
  }
}