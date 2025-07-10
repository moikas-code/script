# Development Tools

This document describes the development tools available in the Script programming language project.

## Overview

The `tools/` directory contains utilities to assist with development tasks, code maintenance, and project management.

## Directory Structure

```
tools/
├── README.md                  # Tools documentation
├── fix_rust_format.py        # CLI for Rust format fixing
└── devutils/                 # Python package with utilities
    ├── __init__.py
    └── rust_format_fixer.py  # Rust format string fixer
```

## Available Tools

### Rust Format Fixer

**Purpose**: Fix common Rust format string issues, especially when migrating to newer Rust editions or dealing with format string syntax changes.

**Location**: `tools/devutils/rust_format_fixer.py`

**Features**:
- Fix inline format arguments (Rust 2021 edition migration)
- Fix missing or extra parentheses in `format!`, `panic!`, `println!`, and `eprintln!` macros
- Fix multiline format statements with incorrect parentheses
- Analyze codebase for issues without making changes
- Automatic backup creation before modifications

**Usage**:

```bash
# Analyze the codebase for format issues
python tools/fix_rust_format.py analyze

# Fix all format issues in src/ directory
python tools/fix_rust_format.py fix

# Fix a specific file
python tools/fix_rust_format.py fix -f src/parser/parser.rs

# Fix without creating backups
python tools/fix_rust_format.py fix --no-backup

# Process a different directory
python tools/fix_rust_format.py fix -d tests/
```

**Common Patterns Fixed**:

1. **Inline Format Arguments**:
   ```rust
   // Before
   format!("Error: {}", msg)
   
   // After (Rust 2021)
   format!("Error: {msg}")
   ```

2. **Missing Parentheses**:
   ```rust
   // Before
   Error::key_not_found(format!("Key {}", key)
   
   // After
   Error::key_not_found(format!("Key {}", key))
   ```

3. **Extra Parentheses**:
   ```rust
   // Before
   return Err(error))));
   
   // After
   return Err(error));
   ```

## Adding New Tools

To add a new development tool:

1. **Create the utility module** in `tools/devutils/`:
   ```python
   # tools/devutils/my_utility.py
   class MyUtility:
       def process(self, ...):
           # Implementation
   ```

2. **Update the package** `__init__.py`:
   ```python
   from .my_utility import MyUtility
   __all__ = ['RustFormatFixer', 'MyUtility']
   ```

3. **Create a CLI wrapper** (optional):
   ```python
   #!/usr/bin/env python3
   # tools/my_tool.py
   import sys
   import os
   sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
   from devutils.my_utility import MyUtility
   
   def main():
       # CLI implementation
   
   if __name__ == "__main__":
       main()
   ```

4. **Update documentation** in `tools/README.md` and this file

## Best Practices

1. **Always create backups** before modifying source files
2. **Test on a single file** before running on entire directories
3. **Use analyze mode** first to understand what will be changed
4. **Commit changes** before running automated fixes
5. **Review changes** after running tools (use `git diff`)

## Requirements

- Python 3.6 or higher
- No external Python dependencies (uses only standard library)
- Read/write access to source files

## Maintenance History

- **2025-01-10**: Created initial development tools structure
  - Consolidated 14 individual fix scripts into reusable `RustFormatFixer`
  - Cleaned up root directory by moving functionality to `tools/`
  - Added analyze mode for non-destructive issue detection