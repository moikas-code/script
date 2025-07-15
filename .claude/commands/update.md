# Update Command Documentation

The `/update` command for the Script programming language provides comprehensive update functionality including binary updates and documentation synchronization.

## Available Commands

### Binary Updates
```bash
script update                    # Check and update to latest version (with prompt)
script update --check           # Check for updates without installing
script update --force           # Force update without prompt
script update --version <ver>   # Update to specific version
script update --list            # List available versions
script update --rollback        # Rollback to previous version
```

### Documentation Updates (NEW)
```bash
script update --docs                  # Sync documentation with current project state
script update --check-consistency    # Check documentation consistency without fixing
```

## Documentation Synchronization Features

The new documentation synchronization system provides:

### Version Synchronization
- Automatically syncs version numbers from `Cargo.toml` to all documentation files
- Tracks version references in:
  - `README.md`
  - `CLAUDE.md`
  - `kb/status/*.md` files

### Command Documentation Sync
- Extracts and validates CLI command examples
- Ensures consistency between `CLAUDE.md` and `README.md`
- Tracks development commands (cargo build, test, etc.)

### Feature Completion Tracking
- Monitors implementation status across the codebase
- Updates completion percentages automatically
- Tracks major system status

### Knowledge Base Integration
- Manages `kb/active/` and `kb/completed/` issue tracking
- Auto-updates status files based on implementation progress
- Validates cross-references and documentation links

## Implementation Details

### Files Tracked
- `/Cargo.toml` (source of truth for version)
- `/README.md` (main project documentation)
- `/CLAUDE.md` (AI assistant guidance)
- `/kb/status/OVERALL_STATUS.md` (implementation status)
- `/kb/active/KNOWN_ISSUES.md` (current issues)
- All command examples and binary documentation

### Validation Rules
- Version consistency across all files
- Required command documentation presence
- Sync validation between paired files
- Feature completion percentage accuracy

## Usage Examples

### Check Documentation Consistency
```bash
script update --check-consistency
```
Output shows any inconsistencies found:
```
🔍 Checking documentation consistency...
⚠ Found 2 issues:
  • Version mismatch in README.md: expected 0.5.0-alpha, found 0.4.9-alpha
  • Missing command documentation: cargo bench
💡 Run 'script update --docs' to fix these issues.
```

### Sync Documentation
```bash
script update --docs
```
Output shows files updated:
```
📚 Updating documentation...
✓ Updated 3 files:
  • README.md
  • CLAUDE.md
  • kb/status/OVERALL_STATUS.md
```

## Error Handling

The system provides detailed error messages for:
- Missing files or directories
- Parse errors in configuration files
- I/O errors during file operations
- Validation failures

## Future Enhancements

Planned improvements include:
- Automatic completion percentage calculation based on test coverage
- Integration with git hooks for pre-commit validation
- Cross-repository documentation sync for multi-project setups
- CLI help text generation from documentation