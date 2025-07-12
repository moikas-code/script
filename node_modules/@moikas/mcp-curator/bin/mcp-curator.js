#!/usr/bin/env node

import { program } from 'commander';
import chalk from 'chalk';
import { 
  listConfigs, 
  saveConfig, 
  loadConfig, 
  applyConfig, 
  deleteConfig,
  addServer,
  removeServer,
  showCurrent,
  backupCurrent,
  init,
  applyProjectTemplateGlobally,
  updateSelf
} from '../lib/commands.js';
import {
  showProjectConfig,
  initProjectConfig,
  addProjectServer,
  removeProjectServer,
  saveProjectTemplate,
  loadProjectTemplate,
  listProjectTemplates,
  deleteProjectTemplate,
  applyProjectPreset
} from '../lib/project-config.js';

program
  .name('mcp-curator')
  .description('Curate and manage MCP server configurations across global and project scopes')
  .version('0.0.2');

// Initialize the config store
program
  .command('init')
  .description('Initialize MCP manager and detect Claude config location')
  .action(init);

// List saved configurations
program
  .command('list')
  .alias('ls')
  .description('List all saved MCP configurations')
  .option('-a, --all', 'Show all templates including built-in presets')
  .action(listConfigs);

// Save current configuration
program
  .command('save <name>')
  .description('Save current Claude MCP configuration with a name')
  .option('-d, --description <desc>', 'Add a description for this config')
  .action(saveConfig);

// Load a saved configuration
program
  .command('load <name>')
  .description('Load and apply a saved MCP configuration')
  .option('-b, --backup', 'Backup current config before loading')
  .action(loadConfig);

// Apply the current staged configuration
program
  .command('apply')
  .description('Apply the current configuration to Claude Code')
  .option('-r, --restart', 'Show restart reminder')
  .action(applyConfig);

// Delete a saved configuration
program
  .command('delete <name>')
  .alias('rm')
  .description('Delete a saved MCP configuration')
  .option('-f, --force', 'Skip confirmation')
  .action(deleteConfig);

// Add a server to current configuration
program
  .command('add-server')
  .alias('add')
  .description('Add a new MCP server to current configuration')
  .action(addServer);

// Remove a server from current configuration
program
  .command('remove-server <name>')
  .alias('rm-server')
  .description('Remove an MCP server from current configuration')
  .action(removeServer);

// Show current Claude configuration
program
  .command('show')
  .description('Show current Claude MCP configuration')
  .option('-j, --json', 'Output as JSON')
  .option('-m, --merged', 'Show merged configuration (global + project)')
  .action(showCurrent);

// Backup current configuration
program
  .command('backup')
  .description('Backup current Claude MCP configuration')
  .option('-n, --name <name>', 'Backup with specific name')
  .action(backupCurrent);

// Quick presets
program
  .command('preset <type>')
  .description('Apply a preset configuration (minimal, standard, full)')
  .action(async (type) => {
    const { applyPreset } = await import('../lib/presets.js');
    await applyPreset(type);
  });

// Apply project template to global configuration
program
  .command('apply-project-template <name>')
  .description('Apply a project template to global configuration')
  .option('-b, --backup', 'Backup current global config before applying')
  .action(applyProjectTemplateGlobally);

// Project-level configuration commands
const project = program.command('project')
  .description('Manage project-level MCP configurations (.mcp.json)');

project
  .command('show')
  .description('Show project MCP configuration')
  .option('-j, --json', 'Output as JSON')
  .action(showProjectConfig);

project
  .command('init')
  .description('Initialize project MCP configuration')
  .option('-e, --example', 'Create with example server')
  .option('-f, --force', 'Overwrite existing configuration')
  .action(initProjectConfig);

project
  .command('add <name>')
  .description('Add a server to project configuration')
  .option('-c, --command <cmd>', 'Server command')
  .option('-a, --args <args>', 'Command arguments (comma-separated)')
  .option('-e, --env <vars>', 'Environment variables (KEY=value, comma-separated)')
  .option('-u, --url <url>', 'SSE server URL')
  .option('-h, --headers <headers>', 'HTTP headers (key:value, comma-separated)')
  .option('-l, --local', 'Add to local config (.mcp.local.json) instead of shared')
  .option('-f, --force', 'Overwrite existing server')
  .action(addProjectServer);

project
  .command('remove <name>')
  .description('Remove a server from project configuration')
  .option('-l, --local', 'Remove from local config only')
  .option('-s, --shared', 'Remove from shared config only')
  .action(removeProjectServer);

project
  .command('save <name>')
  .description('Save current project configuration as a template')
  .option('-d, --description <desc>', 'Add a description for this template')
  .option('-f, --force', 'Overwrite existing template')
  .action(saveProjectTemplate);

project
  .command('load <name>')
  .description('Load a saved project template')
  .option('-b, --backup', 'Backup current project config before loading')
  .action(loadProjectTemplate);

project
  .command('list')
  .description('List saved project templates')
  .action(listProjectTemplates);

project
  .command('delete <name>')
  .alias('rm')
  .description('Delete a saved project template')
  .option('-f, --force', 'Skip confirmation')
  .action(deleteProjectTemplate);

project
  .command('preset <type>')
  .description('Apply a global preset to project configuration')
  .option('-b, --backup', 'Backup current project config before applying')
  .action(applyProjectPreset);

// Update command
program
  .command('update')
  .description('Update mcp-curator to the latest version')
  .option('-g, --global', 'Update global installation (default)')
  .option('-c, --check', 'Check for updates without installing')
  .action(updateSelf);

program.parse();