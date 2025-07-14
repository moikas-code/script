# Auto-Update System

**Status**: ✅ Fully Implemented (v0.5.0-alpha)

Script includes a built-in auto-update system that automatically downloads and installs new versions from GitHub releases.

## Overview

The auto-update system is built using the `self_update` crate and integrates with GitHub releases to provide seamless updates for Script installations.

## Features

- **Automatic Version Checking**: Check for new releases on GitHub
- **Safe Updates**: Download verification and rollback support
- **Progress Reporting**: Real-time update progress with colored output
- **Version Management**: List available versions and manage installations
- **Cross-platform**: Works on Linux, macOS, and Windows

## Implementation

The auto-update functionality is implemented in `src/update/mod.rs` and includes:

### Core Components

1. **UpdateError**: Comprehensive error handling for update operations
2. **ScriptUpdater**: Main updater implementation
3. **Progress Callbacks**: Real-time update progress reporting
4. **Version Management**: GitHub API integration for version discovery

### Key Functions

#### `check_update() -> Result<Option<String>, UpdateError>`

Checks if a new version is available on GitHub releases.

```rust
use script::update::check_update;

match check_update() {
    Ok(Some(version)) => println!("New version available: {}", version),
    Ok(None) => println!("Already up to date"),
    Err(e) => eprintln!("Update check failed: {}", e),
}
```

#### `update() -> Result<(), UpdateError>`

Downloads and installs the latest version.

```rust
use script::update::update;

match update() {
    Ok(()) => println!("Update completed successfully"),
    Err(e) => eprintln!("Update failed: {}", e),
}
```

#### `list_versions() -> Result<Vec<String>, UpdateError>`

Lists all available versions from GitHub releases.

```rust
use script::update::list_versions;

match list_versions() {
    Ok(versions) => {
        println!("Available versions:");
        for version in versions {
            println!("  - {}", version);
        }
    },
    Err(e) => eprintln!("Failed to list versions: {}", e),
}
```

## CLI Usage

### Check for Updates

```bash
script --check-update
```

### Update to Latest Version

```bash
script --update
```

### List Available Versions

```bash
script --list-versions
```

### Update to Specific Version

```bash
script --update-to v0.5.0-alpha
```

## Configuration

The updater can be configured through environment variables:

```bash
# Custom GitHub repository (default: moikapy/script)
export SCRIPT_UPDATE_REPO="username/script-fork"

# Custom update server URL
export SCRIPT_UPDATE_URL="https://api.github.com"

# Disable update checks
export SCRIPT_NO_UPDATE_CHECK=1
```

## Security

The auto-update system includes several security measures:

1. **HTTPS Verification**: All downloads use HTTPS with certificate verification
2. **Checksum Validation**: Downloaded binaries are verified against GitHub checksums
3. **Backup Creation**: Current binary is backed up before replacement
4. **Rollback Support**: Failed updates can be rolled back automatically

## Implementation Details

### GitHub Integration

The updater integrates with GitHub's releases API:

```rust
pub struct ScriptUpdater {
    repo_owner: String,
    repo_name: String,
    current_version: String,
    target: String,
}

impl ScriptUpdater {
    pub fn new() -> Result<Self, UpdateError> {
        Ok(ScriptUpdater {
            repo_owner: "moikapy".to_string(),
            repo_name: "script".to_string(),
            current_version: cargo_crate_version!().to_string(),
            target: get_target_triple(),
        })
    }
}
```

### Progress Reporting

Updates include colored progress output:

```rust
pub fn update() -> Result<(), UpdateError> {
    println!("{} {}", "Checking for updates...".bright_blue(), "⏳");
    
    let updater = ScriptUpdater::new()?;
    if let Some(latest_version) = updater.check_update()? {
        println!("{} {} -> {}", 
            "Updating".bright_green(), 
            updater.current_version, 
            latest_version
        );
        
        updater.perform_update()?;
        
        println!("{} {}", "Update completed!".bright_green(), "✅");
    } else {
        println!("{} {}", "Already up to date".bright_green(), "✅");
    }
    
    Ok(())
}
```

### Error Handling

Comprehensive error types for update operations:

```rust
#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("GitHub API error: {0}")]
    GitHubApi(String),
    
    #[error("Version parsing error: {0}")]
    Version(#[from] semver::Error),
    
    #[error("File system error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Update verification failed")]
    VerificationFailed,
    
    #[error("No releases found")]
    NoReleases,
    
    #[error("Platform not supported: {0}")]
    UnsupportedPlatform(String),
}
```

## Platform Support

The auto-update system supports multiple platforms:

- **Linux**: x86_64, ARM64
- **macOS**: x86_64, ARM64 (Apple Silicon)
- **Windows**: x86_64

Binaries are automatically selected based on the target platform.

## Testing

The update system includes comprehensive tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        let updater = ScriptUpdater::new().unwrap();
        assert!(updater.parse_version("v0.5.0-alpha").is_ok());
    }

    #[test]
    fn test_platform_detection() {
        let target = get_target_triple();
        assert!(!target.is_empty());
    }
}
```

## Future Enhancements

Planned improvements for the auto-update system:

1. **Delta Updates**: Download only changed parts for faster updates
2. **Update Scheduling**: Automatic background updates
3. **Channel Support**: Stable, beta, and nightly release channels
4. **Signature Verification**: GPG signature verification for enhanced security
5. **Update Policies**: Enterprise-friendly update management

## Troubleshooting

### Common Issues

#### Update Check Fails
```bash
# Check network connectivity
curl -I https://api.github.com/repos/moikapy/script/releases/latest

# Check for proxy issues
export HTTPS_PROXY=your-proxy-url
script --check-update
```

#### Permission Denied
```bash
# On Unix systems, ensure the binary is writable
chmod +w $(which script)
script --update
```

#### Rollback Required
```bash
# Manual rollback (backup is created automatically)
cp script.backup script
chmod +x script
```

### Debug Mode

Enable debug output for troubleshooting:

```bash
SCRIPT_DEBUG=1 script --update
```

## Dependencies

The auto-update system depends on:

- `self_update` (0.39+): Core update functionality
- `reqwest` (0.11+): HTTP client for GitHub API
- `semver` (1.0+): Version parsing and comparison
- `colored` (2.0+): Terminal color output
- `thiserror` (1.0+): Error handling

## API Reference

For programmatic usage, the update API is available:

```rust
use script::update::{ScriptUpdater, UpdateError};

// Create updater instance
let updater = ScriptUpdater::new()?;

// Check for updates
if let Some(version) = updater.check_update()? {
    println!("New version available: {}", version);
    
    // Perform update
    updater.perform_update()?;
}
```

The update system ensures Script users can easily stay current with the latest features and security updates while maintaining a smooth, reliable update experience.