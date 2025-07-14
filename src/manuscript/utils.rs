/// Utility functions for manuscript
use crate::package::{PackageError, PackageResult};
use std::path::Path;
use std::process::Command;

/// Check if git is available
pub fn is_git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Get current git branch
pub fn get_git_branch() -> Option<String> {
    let output = Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .ok()
            .map(|s| s.trim().to_string())
    } else {
        None
    }
}

/// Check if working directory is clean
pub fn is_git_clean() -> bool {
    Command::new("git")
        .args(&["diff", "--quiet", "--exit-code"])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Format file size for display
pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {size as u64, UNITS[unit_index]}")
    } else {
        format!("{:.1} {size, UNITS[unit_index]}")
    }
}

/// Calculate directory size recursively
pub fn dir_size(path: &Path) -> PackageResult<u64> {
    let mut total = 0;

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;

        if metadata.is_dir() {
            total += dir_size(&entry.path())?;
        } else {
            total += metadata.len();
        }
    }

    Ok(total)
}

/// Create a temporary directory for operations
pub fn temp_dir() -> PackageResult<tempfile::TempDir> {
    tempfile::TempDir::new().map_err(|e| PackageError::Io(e))
}

/// Validate semantic version string
pub fn is_valid_version(version: &str) -> bool {
    semver::Version::parse(version).is_ok()
}

/// Get system information for user agent
pub fn get_system_info() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    format!("{}-{os, arch}")
}
