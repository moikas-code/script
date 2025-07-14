# VS Code/Cursor Setup for Script Language

This guide explains how to configure VS Code or Cursor to recognize Script files (.script) with proper icons and syntax highlighting.

## Quick Setup

The project includes VS Code configuration files that will:
1. Associate `.script` files with Rust syntax highlighting (temporary until full extension)
2. Configure Rust development environment
3. Provide basic file type recognition

## File Icon Support

### Option 1: Use Existing Icon Themes
Many popular VS Code icon themes support custom file associations. Install one of these themes:
- **Material Icon Theme** (most popular)
- **vscode-icons**
- **Monokai Pro Icons**

Then add this to your **user** settings.json:

```json
"material-icon-theme.files.associations": {
  "*.script": "rust"
}
```

Or for vscode-icons:
```json
"vsicons.associations.files": [
  { "icon": "rust", "extensions": ["script"], "format": "svg" }
]
```

### Option 2: Manual Configuration

1. The project includes `.vscode/settings.json` which associates `.script` files with Rust
2. This provides basic syntax highlighting and icon from your current theme

### Option 3: Use the Official Script Extension (RECOMMENDED!)

The Script language now has an official VS Code extension!

#### Install from Marketplace (When Available)
```
ext install script-lang.script-lang
```

#### Install from GitHub
1. Download the latest `.vsix` from [Script VS Code Extension Releases](https://github.com/moikas-code/vscode-script-extension/releases)
2. Install via Command Palette: "Extensions: Install from VSIX..."

#### Build from Source
The extension source is available at: https://github.com/moikas-code/vscode-script-extension
```bash
git clone https://github.com/moikas-code/vscode-script-extension.git
cd vscode-script-extension
npm install
npm run package
code --install-extension script-lang-*.vsix
```

This provides:
- ✅ Custom `script` language identifier
- ✅ Full Script syntax highlighting
- ✅ Proper language configuration
- ✅ File type recognition
- ✅ Community contribution support
- ⏳ Language server integration (85% complete)

**GitHub Repository**: [github.com/moikas-code/vscode-script-extension](https://github.com/moikas-code/vscode-script-extension)

## Current Workaround Features

With the current setup, you get:
- ✅ File type recognition
- ✅ Basic syntax highlighting (using Rust)
- ✅ Icon from your theme's Rust icon
- ✅ File search and quick open support
- ⏳ Language server features (coming soon)

## Installing in Cursor

Cursor uses the same configuration as VS Code:
1. Open Cursor settings (Cmd/Ctrl + ,)
2. Search for "file associations"
3. The `.vscode/settings.json` will automatically apply
4. Install an icon theme extension for better icons

## Custom Icon Setup

If you want to use a custom Script icon:

1. Create an icon file: `.vscode/icons/script-icon.svg`
2. Use the included `.vscode/script-icon-theme.json` as a base
3. Configure your settings to use the custom theme

## Troubleshooting

**Icons not showing:**
- Restart VS Code/Cursor after configuration changes
- Ensure you have an icon theme installed and activated
- Check that file associations are properly set

**Syntax highlighting not working:**
- The temporary Rust association should provide basic highlighting
- Full Script syntax highlighting requires the official extension

**File search not finding .script files:**
- Check that `.script` is not in your `files.exclude` patterns
- Ensure the files are not in .gitignore (they shouldn't be based on current config)

## Future Improvements

Once the official Script VS Code extension is released, it will provide:
- Native Script language syntax highlighting
- Custom Script file icon
- Full IntelliSense with type information
- Integrated REPL
- Debugging support
- Refactoring tools

For now, the Rust association provides a good development experience for Script files since Script's syntax is similar to Rust.