# Development Tools

This directory contains development utilities for the Script programming language project.

## Available Tools

### rust_format_fixer

A utility for fixing common Rust format string issues, especially useful when migrating to newer Rust editions.

**Features:**
- Fix inline format arguments (Rust 2021 edition)
- Fix missing or extra parentheses in format! calls
- Fix multiline format! statements
- Analyze codebase for format issues without making changes
- Create backups before modifying files

**Usage:**

```bash
# Analyze the codebase for format issues
python tools/fix_rust_format.py analyze

# Fix all format issues in src/ directory
python tools/fix_rust_format.py fix

# Fix a specific file
python tools/fix_rust_format.py fix -f src/parser/parser.rs

# Fix without creating backups
python tools/fix_rust_format.py fix --no-backup
```

## Development

The utilities are organized as a Python package in `devutils/`. To add new utilities:

1. Create a new module in `devutils/`
2. Add the import to `devutils/__init__.py`
3. Optionally create a command-line wrapper in `tools/`

## Requirements

- Python 3.6+
- No external dependencies (uses only Python standard library)