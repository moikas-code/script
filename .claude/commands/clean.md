# /clean Command Documentation

## Overview

The `/clean` command provides comprehensive project maintenance and cleanup functionality for the Script programming language project. It safely removes temporary files, organizes project structure, updates dependencies, and performs general housekeeping tasks to maintain a clean and efficient development environment.

## Purpose

This command enhances development efficiency and project health by:
- Removing build artifacts, caches, and temporary files
- Organizing and archiving knowledge base entries
- Cleaning up backup files and obsolete code
- Updating and auditing project dependencies
- Optimizing project structure and reducing disk usage
- Maintaining consistent file organization across the project

## Usage

### Basic Syntax
```bash
/clean                          # Interactive cleanup with recommendations
/clean <category>              # Clean specific category of files
/clean --preview               # Show what would be cleaned without doing it
/clean --aggressive            # More thorough cleanup (use with caution)
```

### Cleanup Categories
```bash
/clean build                   # Build artifacts and compilation outputs
/clean cache                   # Various caches (cargo, node, etc.)
/clean temp                    # Temporary files and swap files
/clean backup                  # Backup files and version control artifacts
/clean deps                    # Dependency cleanup and updates
/clean kb                      # Knowledge base organization and archival
/clean logs                    # Log files and debug outputs
/clean assets                  # Unused assets and resources
```

### Cleanup Scope
```bash
/clean --target <path>         # Clean specific directory or file
/clean --recursive             # Clean recursively through subdirectories
/clean --project-wide          # Clean entire project (default)
/clean --safe-only             # Only perform safe, non-destructive cleanup
/clean --force                 # Skip confirmations for safe operations
```

### Advanced Options
```bash
/clean --dry-run              # Show cleanup plan without executing
/clean --report               # Generate cleanup report for knowledge base
/clean --schedule             # Set up automated cleanup schedules
/clean --analyze              # Analyze disk usage and cleanup opportunities
```

## Cleanup Categories

### 1. Build Artifacts Cleanup
**Command**: `/clean build`

#### Build System Cleanup
```bash
/clean build --preview
```

**Cleanup Targets**:
```
🔧 Build Artifacts Analysis
===========================
Scan Complete: 2.3 seconds

Build Artifacts Found:
├── Rust build outputs
│   ├── target/ directory: 1.2GB
│   │   ├── debug/ builds: 523MB
│   │   ├── release/ builds: 445MB
│   │   ├── incremental/ cache: 234MB
│   │   └── deps/ artifacts: 98MB
│   └── Cargo.lock artifacts: 234KB
├── LLVM compilation cache
│   ├── .llvm-cache/: 89MB
│   └── Temporary IR files: 12MB
├── WebAssembly outputs
│   ├── pkg/ directory: 34MB
│   └── wasm build artifacts: 23MB
└── IDE artifacts
    ├── .vscode/ temporary: 5MB
    ├── Language server cache: 15MB
    └── IntelliJ artifacts: 8MB

Total Space: 1.47GB

Safe to Remove:
✅ Debug builds (523MB) - Easily rebuilt
✅ Incremental cache (234MB) - Will regenerate  
✅ Temporary IR files (12MB) - Compilation artifacts
✅ IDE temporary files (28MB) - Editor-specific
✅ Old WebAssembly builds (23MB) - Outdated

Keep (Important):
⚠ Release builds (445MB) - Time-consuming to rebuild
⚠ Core dependencies cache (98MB) - Slow to download
⚠ LLVM cache (89MB) - Expensive to regenerate

Recommended Cleanup:
├── Space to reclaim: 820MB (56% of total)
├── Rebuild time impact: ~3 minutes
├── Download time saved: ~45MB cache preserved
└── Risk level: Very Low

Would you like to proceed with recommended cleanup? [Y/n]:
```

#### Advanced Build Cleanup
```bash
/clean build --aggressive
```

**Aggressive Cleanup Options**:
- Remove all build artifacts including release builds
- Clear all dependency caches (forces fresh downloads)
- Remove compiler caches and optimization data
- Clean generated documentation artifacts

### 2. Cache Cleanup
**Command**: `/clean cache`

#### Multi-System Cache Cleanup
```bash
/clean cache
```

**Cache Analysis**:
```
💾 Cache Analysis Report
========================
Total Cache Usage: 453MB across 8 cache types

Cache Categories:
├── Cargo Registry Cache: 156MB
│   ├── Downloaded crates: 134MB
│   ├── Registry index: 22MB
│   └── Last cleanup: 15 days ago ⚠
├── Node.js Cache: 89MB
│   ├── npm cache: 67MB
│   ├── yarn cache: 22MB
│   └── Package-lock artifacts: <1MB
├── Rust Compiler Cache: 78MB
│   ├── rustc incremental: 45MB
│   ├── procedural macro cache: 23MB
│   └── Target cache: 10MB
├── MCP Tools Cache: 34MB
│   ├── Downloaded tools: 28MB
│   ├── Configuration cache: 4MB
│   └── Runtime cache: 2MB
├── Language Server Cache: 28MB
│   ├── Index files: 18MB
│   ├── Symbol cache: 7MB
│   └── Completion cache: 3MB
├── Git Object Cache: 23MB
│   ├── Packed objects: 18MB
│   ├── Loose objects: 3MB
│   └── Refs cache: 2MB
├── Documentation Cache: 15MB
│   ├── Generated docs: 12MB
│   └── Asset cache: 3MB
└── Browser Cache (Dev Tools): 30MB
    ├── Source maps: 18MB
    ├── DevTools workspace: 8MB
    └── Extensions data: 4MB

Cleanup Recommendations:
✅ Old cargo registry (>30 days): 45MB
✅ Stale npm cache: 23MB  
✅ Outdated compiler cache: 12MB
✅ Unused MCP tool cache: 15MB
✅ Old documentation cache: 8MB

Conservative Cleanup: 103MB (safe, no rebuild required)
Aggressive Cleanup: 298MB (faster, requires some rebuilding)
```

### 3. Temporary Files Cleanup
**Command**: `/clean temp`

#### Temporary File Detection
```bash
/clean temp --analyze
```

**Temporary Files Report**:
```
🗂️ Temporary Files Analysis
============================
Scan Coverage: Entire project + system temp

Temporary Files Found:
├── System Temporary Files
│   ├── /tmp/script-compile-*: 23 files (45MB)
│   ├── /tmp/cargo-*: 8 directories (12MB)
│   └── /tmp/node-*: 15 files (8MB)
├── Editor Temporary Files
│   ├── .swp files (Vim): 3 files (24KB)
│   ├── .tmp files (VSCode): 7 files (156KB)
│   ├── ~$* files (Word): 0 files
│   └── .DS_Store (macOS): 12 files (48KB)
├── Build Temporary Files
│   ├── *.o object files: 45 files (67MB)
│   ├── *.tmp compilation temps: 23 files (34MB)
│   └── *.lock temporary locks: 5 files (20KB)
├── Debug/Log Temporary Files
│   ├── debug-*.log: 18 files (15MB)
│   ├── trace-*.json: 5 files (23MB)
│   └── core dumps: 0 files ✅
└── Backup/Recovery Files
    ├── *.backup files: 34 files (89MB)
    ├── *~ backup files: 12 files (23KB)
    ├── .orig merge files: 8 files (45KB)
    └── auto-save files: 15 files (234KB)

Total Temporary Files: 234 files (317MB)

Age Analysis:
├── < 1 hour: 23 files (45MB) ⚠ May be in use
├── 1-24 hours: 67 files (123MB) ✅ Safe to clean
├── 1-7 days: 89 files (98MB) ✅ Definitely safe
├── > 7 days: 55 files (51MB) ✅ Should be cleaned
└── > 30 days: 12 files (8MB) ✅ Overdue for cleanup

Safe Cleanup Candidates: 223 files (280MB)
Risk Assessment: Very Low (no active files)
```

### 4. Backup Files Cleanup
**Command**: `/clean backup`

#### Backup File Management
```bash
/clean backup --organize
```

**Backup Organization**:
```
📚 Backup Files Management
==========================
Backup Strategy: Organize by age and importance

Backup Files Inventory:
├── Source Code Backups
│   ├── *.rs.backup: 45 files (234KB)
│   ├── *.backup.rs: 23 files (123KB)
│   ├── *.orig: 12 files (67KB)
│   └── Version control artifacts: 34 files (456KB)
├── Configuration Backups
│   ├── *.toml.backup: 8 files (23KB)
│   ├── *.json.backup: 15 files (45KB)
│   ├── *.yaml.backup: 5 files (12KB)
│   └── Environment backups: 3 files (8KB)
├── Documentation Backups
│   ├── *.md.backup: 67 files (345KB)
│   ├── Draft documents: 23 files (123KB)
│   └── Auto-saved versions: 45 files (234KB)
└── Build Script Backups
    ├── Cargo.toml versions: 12 files (34KB)
    ├── Build.rs backups: 5 files (23KB)
    └── CI configuration: 8 files (45KB)

Organization Plan:
├── Archive Recent (< 7 days): Move to .archive/recent/
├── Archive Old (7-30 days): Move to .archive/monthly/
├── Remove Ancient (> 90 days): 15 files for deletion
├── Keep Important: Version control and CI configs
└── Compress Large: Backup files > 10KB

Actions:
✅ Create organized backup structure
✅ Move 156 files to appropriate archives
✅ Remove 15 obsolete backup files (8MB)
✅ Compress 23 large backup files (saving 67MB)
✅ Update backup retention policy documentation

Space Savings: 75MB organized, 8MB reclaimed
Organization Benefit: Easy to find specific backups
```

### 5. Dependency Cleanup
**Command**: `/clean deps`

#### Dependency Audit and Cleanup
```bash
/clean deps --audit
```

**Dependency Analysis**:
```
📦 Dependency Cleanup Analysis
==============================
Analysis Date: 2025-07-15T16:30:00Z

Rust Dependencies (Cargo):
├── Total dependencies: 234 (direct: 23, transitive: 211)
├── Outdated packages: 12 ⚠
├── Unused dependencies: 3 ❌
├── Security advisories: 0 ✅
├── License issues: 0 ✅
└── Cache size: 156MB

Node.js Dependencies (npm):
├── Total packages: 145 (direct: 15, transitive: 130)
├── Outdated packages: 8 ⚠
├── Vulnerabilities: 0 ✅
├── Unused packages: 2 ❌
├── Cache size: 89MB
└── node_modules size: 234MB

System Dependencies:
├── LLVM tools: Current ✅
├── Platform libraries: Current ✅
├── Development tools: 2 updates available ⚠
└── Documentation tools: Current ✅

Cleanup Opportunities:
1. 🗑️ Remove unused Rust dependencies
   ├── lazy_static (not used): Remove from Cargo.toml
   ├── regex (redundant): Covered by other deps
   └── old_version_crate: Replace with updated alternative

2. 🗑️ Remove unused Node.js packages
   ├── @types/unused: Leftover from old feature
   └── old-tool: Replaced by new implementation

3. 📦 Update outdated dependencies
   ├── High priority: 5 packages (security/performance)
   ├── Medium priority: 8 packages (features/bug fixes)
   └── Low priority: 7 packages (minor improvements)

4. 🧹 Clean dependency caches
   ├── Old cargo registry entries: 45MB
   ├── Stale npm cache: 23MB
   └── Unused target artifacts: 67MB

Estimated Cleanup:
├── Dependencies removed: 5 packages
├── Disk space reclaimed: 234MB
├── Build time improvement: 8-12%
├── Security posture: Maintained ✅
└── Functionality: No impact ✅

Update Plan:
Phase 1: Remove unused dependencies (low risk)
Phase 2: Update non-breaking changes (medium risk)
Phase 3: Major version updates (high risk, requires testing)
```

### 6. Knowledge Base Organization
**Command**: `/clean kb`

#### Knowledge Base Maintenance
```bash
/clean kb --organize
```

**KB Organization Plan**:
```
📋 Knowledge Base Organization
==============================
Current KB Status: 234 files across 12 directories

Directory Analysis:
├── kb/active/: 27 files ✅ (current issues)
├── kb/completed/: 156 files ⚠ (many old entries)
├── kb/archived/: 23 files ✅ (properly archived)
├── kb/status/: 12 files ✅ (current status)
├── kb/reports/: 34 files ⚠ (some outdated)
├── kb/planning/: 15 files ⚠ (mix of current/old)
├── kb/documentation/: 45 files ✅ (well organized)
├── kb/security/: 18 files ✅ (current)
├── kb/legacy/: 67 files ❌ (needs cleanup)
├── kb/temp/: 8 files ❌ (should not exist)
├── kb/development/: 23 files ⚠ (needs review)
└── kb/compliance/: 6 files ✅ (current)

Organization Issues Detected:
1. ❌ Temporary files in kb/temp/
   ├── Should be moved to appropriate directories
   ├── Or removed if obsolete
   └── 8 files requiring action

2. ⚠ Outdated completed items
   ├── 67 files older than 6 months in kb/completed/
   ├── Should be archived or deleted
   └── Many superseded by newer implementations

3. ⚠ Mixed planning content
   ├── kb/planning/ contains both active and completed plans
   ├── Need to separate active from historical
   └── 8 files need categorization

4. ❌ Legacy folder bloat
   ├── kb/legacy/ has grown to 67 files
   ├── Many files are duplicates or superseded
   ├── Needs aggressive cleanup
   └── Estimated reduction: 70% of content

5. ⚠ Report retention
   ├── kb/reports/ has reports from 8 months ago
   ├── Old reports should be archived
   └── Keep only last 3 months of reports

Organization Actions:
├── Move 8 temp files to proper locations
├── Archive 67 old completed items
├── Clean up 47 legacy files (keeping 20 important ones)
├── Archive 18 old reports
├── Reorganize 8 planning files
├── Create missing index files
├── Update cross-references
└── Validate all remaining links

Estimated Results:
├── Files removed: 89 (no information loss)
├── Files moved: 34 (better organization)
├── Disk space saved: 23MB
├── Navigation improved: Significantly better
└── Maintenance reduced: Easier to find content
```

### 7. Log Files Cleanup
**Command**: `/clean logs`

#### Log Management
```bash
/clean logs --retention-policy
```

**Log Cleanup Strategy**:
```
📋 Log Files Management
=======================
Log Retention Policy: Keep 30 days detailed, 90 days summary

Log Categories:
├── Compilation Logs
│   ├── cargo-build-*.log: 45 files (234MB)
│   ├── rustc-*.log: 23 files (89MB)
│   └── llvm-*.log: 12 files (45MB)
├── Runtime Logs
│   ├── script-runtime-*.log: 67 files (345MB)
│   ├── async-executor-*.log: 34 files (123MB)
│   └── gc-debug-*.log: 23 files (67MB)
├── Test Logs
│   ├── test-results-*.log: 89 files (456MB)
│   ├── benchmark-*.log: 23 files (234MB)
│   └── integration-*.log: 15 files (89MB)
├── Development Logs
│   ├── debug-session-*.log: 34 files (123MB)
│   ├── repl-history-*.log: 56 files (234MB)
│   └── language-server-*.log: 23 files (89MB)
└── System Logs
    ├── error-reports-*.log: 12 files (67MB)
    ├── crash-dumps-*.log: 3 files (234MB)
    └── performance-*.log: 45 files (345MB)

Retention Analysis:
├── Current (< 7 days): 145 files (1.2GB) ✅ Keep all
├── Recent (7-30 days): 89 files (897MB) ✅ Keep detailed
├── Medium (30-90 days): 67 files (456MB) ⚠ Keep summaries only
├── Old (90+ days): 123 files (1.1GB) ❌ Archive or remove
└── Ancient (180+ days): 45 files (678MB) ❌ Remove

Cleanup Plan:
1. Compress old detailed logs (30-90 days)
2. Create summary reports for medium-age logs
3. Archive important diagnostic logs (>90 days)
4. Remove routine logs (>180 days)
5. Set up automatic log rotation

Space Savings: 1.4GB reclaimed, 456MB compressed
Performance Impact: Faster log searches, reduced I/O
```

## Interactive Cleanup Workflows

### 1. Guided Cleanup Session
```bash
/clean
```

**Interactive Cleanup Process**:
```
🧹 Interactive Cleanup Session
==============================
Welcome to the Script Language project cleanup wizard!

Step 1/6: Quick Analysis
├── Scanning project structure... ✅
├── Analyzing disk usage... ✅
├── Identifying cleanup opportunities... ✅
└── Generating recommendations... ✅

Analysis Complete! Found 1.7GB of cleanup opportunities.

📊 Cleanup Summary:
├── 🏗️ Build artifacts: 820MB (low risk)
├── 💾 Cache files: 298MB (medium risk)
├── 📄 Temporary files: 280MB (very low risk)
├── 🗂️ Backup files: 75MB (low risk)
├── 📦 Dependencies: 234MB (medium risk)
└── 📋 Knowledge base: 23MB (low risk)

Step 2/6: Prioritized Recommendations

🎯 High Impact, Low Risk (Recommended):
├── ✅ Clean temporary files (280MB, 30 seconds)
├── ✅ Remove old build artifacts (345MB, 1 minute)
├── ✅ Organize backup files (75MB, 2 minutes)
└── ✅ Clean stale caches (123MB, 30 seconds)

Total: 823MB in ~4 minutes, virtually no risk

Would you like to proceed with recommended cleanup? [Y/n]: Y

Step 3/6: Execution
├── Cleaning temporary files... ✅ (280MB freed)
├── Removing old build artifacts... ✅ (345MB freed)
├── Organizing backup files... ✅ (75MB freed)
└── Cleaning stale caches... ✅ (123MB freed)

Step 4/6: Additional Opportunities

🤔 Medium Impact Options:
├── ⚠️ Update dependencies (may require testing)
├── ⚠️ Aggressive cache cleanup (may slow next build)
├── ⚠️ Archive old knowledge base entries
└── ⚠️ Remove development logs older than 30 days

Would you like to explore these options? [y/N]: n

Step 5/6: Cleanup Report
├── 📊 Space reclaimed: 823MB
├── ⏱️ Time taken: 3m 45s
├── 🛡️ Risk level: Very low
├── 🎯 Next build impact: Minimal (~3 minute rebuild)
└── 📝 Report saved: kb/reports/cleanup-2025-07-15.md

Step 6/6: Maintenance Recommendations
├── 🔄 Set up weekly automated cleanup
├── 📋 Enable build artifact rotation
├── 🗓️ Schedule monthly dependency audits
└── 📊 Monitor disk usage trends

Cleanup session complete! ✨
```

### 2. Cleanup Preview Mode
```bash
/clean --preview --detailed
```

**Detailed Preview Output**:
```
🔍 Cleanup Preview (Detailed Mode)
===================================
Showing exactly what would be cleaned without executing.

Build Artifacts Cleanup:
┌─────────────────────────────────────────────┐
│ DELETE: target/debug/                       │
│ ├── Size: 523MB                            │
│ ├── Last modified: 2 days ago              │
│ ├── Rebuild time: ~3 minutes               │
│ └── Risk: Very low (easily rebuilt)        │
│                                             │
│ DELETE: target/incremental/                 │
│ ├── Size: 234MB                            │
│ ├── Contents: 1,247 cache files            │
│ ├── Regeneration: Automatic on next build  │
│ └── Risk: None (performance cache only)    │
│                                             │
│ KEEP: target/release/ (445MB)              │
│ └── Reason: Expensive to rebuild (15+ min) │
└─────────────────────────────────────────────┘

Temporary Files Cleanup:
┌─────────────────────────────────────────────┐
│ DELETE: 67 .swp files (24KB)               │
│ ├── Locations: Scattered across src/       │
│ ├── Age: 1-30 days old                     │
│ └── Risk: None (editor artifacts)          │
│                                             │
│ DELETE: 23 *.tmp files (156KB)             │
│ ├── Locations: /tmp/, target/tmp/          │
│ ├── Age: 2-15 days old                     │
│ └── Risk: None (build artifacts)           │
│                                             │
│ DELETE: debug-*.log (15MB)                 │
│ ├── Count: 18 log files                    │
│ ├── Age: > 7 days old                      │
│ └── Risk: Very low (debug information)     │
└─────────────────────────────────────────────┘

Knowledge Base Organization:
┌─────────────────────────────────────────────┐
│ MOVE: kb/temp/* → appropriate directories   │
│ ├── Files: 8 temporary entries             │
│ ├── Action: Categorize and move            │
│ └── Risk: None (organization only)         │
│                                             │
│ ARCHIVE: kb/completed/ (67 old files)      │
│ ├── Criteria: > 6 months old               │
│ ├── Destination: kb/archived/2024/         │
│ └── Risk: None (preserved, just organized) │
└─────────────────────────────────────────────┘

Summary:
├── Total files affected: 1,247
├── Total space reclaimed: 823MB
├── Estimated execution time: ~4 minutes
├── Overall risk level: Very Low
├── Rebuild impact: ~3 minutes on next build
└── No data loss: All important files preserved

Execute this cleanup plan? [Y/n/details]:
```

## Safety Features and Safeguards

### 1. Confirmation and Preview
All potentially destructive operations require confirmation:
- **Preview Mode**: Show what would be cleaned before execution
- **Interactive Confirmation**: Ask before removing large amounts of data
- **Selective Cleaning**: Allow users to choose what to clean
- **Risk Assessment**: Clearly communicate risk levels

### 2. Backup and Recovery
```bash
/clean --create-backup
```

**Backup Strategy**:
- Create snapshots before major cleanup operations
- Preserve important development state
- Enable quick recovery if cleanup causes issues
- Maintain cleanup history for audit purposes

### 3. Rollback Capability
```bash
/clean --rollback <cleanup-session-id>
```

**Rollback Features**:
- Undo recent cleanup operations
- Restore from automatic backups
- Selective restoration of specific files
- Recovery from cleanup mistakes

## Integration with Other Commands

### Command Synergy
- `/clean` → `/test` (ensure cleanup doesn't break tests)
- `/clean` → `/status` (verify project health after cleanup)
- `/clean deps` → `/audit` (security audit after dependency updates)
- `/clean kb` → `/docs` (update documentation after KB organization)

### Automated Integration
- Run cleanup before major builds
- Integrate with CI/CD for regular maintenance
- Schedule periodic cleanup tasks
- Monitor disk usage and trigger cleanup automatically

## Maintenance Scheduling

### Automated Cleanup Schedules
```bash
/clean --schedule daily --safe-only
/clean --schedule weekly --comprehensive
/clean --schedule monthly --aggressive
```

**Schedule Options**:
- **Daily**: Temporary files, small caches (very safe)
- **Weekly**: Build artifacts, larger caches (safe)
- **Monthly**: Dependencies, comprehensive cleanup (requires review)
- **On-demand**: Full cleanup with user oversight

### Monitoring and Alerts
- Disk usage monitoring with automatic alerts
- Cleanup effectiveness reporting
- Performance impact assessment
- Recommendations for cleanup schedule optimization

## Best Practices

### Regular Maintenance
- Run safe cleanup operations regularly (daily/weekly)
- Review cleanup reports for optimization opportunities
- Monitor disk usage trends to anticipate cleanup needs
- Keep cleanup schedules aligned with development cycles

### Safety Guidelines
- Always use preview mode for unfamiliar cleanup operations
- Create backups before aggressive cleanup sessions
- Test builds after dependency cleanup
- Review knowledge base organization periodically

### Performance Optimization
- Schedule cleanup during low-activity periods
- Parallelize safe cleanup operations
- Monitor cleanup performance and optimize accordingly
- Balance cleanup frequency with rebuild costs

This `/clean` command provides comprehensive project maintenance capabilities that keep the Script language development environment clean, organized, and efficient while maintaining safety and preserving important development artifacts.