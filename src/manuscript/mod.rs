/// Manuscript - Package Manager for Script Language
///
/// This module contains the implementation of the manuscript package manager.
/// It provides functionality for:
/// - Package initialization and scaffolding
/// - Dependency management and resolution
/// - Building and compilation
/// - Publishing to package registry
/// - Local package caching
pub mod commands;
pub mod config;
pub mod templates;
pub mod utils;

pub use config::ManuscriptConfig;

use crate::package::{PackageError, PackageResult};
use std::path::{Path, PathBuf};

/// Get the default manuscript home directory
pub fn manuscript_home() -> PackageResult<PathBuf> {
    if let Ok(home) = std::env::var("MANUSCRIPT_HOME") {
        return Ok(PathBuf::from(home));
    }

    dirs::home_dir()
        .map(|home| home.join(".manuscript"))
        .ok_or_else(|| {
            PackageError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not find home directory",
            ))
        })
}

/// Get the global packages directory
pub fn global_packages_dir() -> PackageResult<PathBuf> {
    Ok(manuscript_home()?.join("packages"))
}

/// Get the cache directory
pub fn cache_dir() -> PackageResult<PathBuf> {
    Ok(manuscript_home()?.join("cache"))
}

/// Get the config file path
pub fn config_path() -> PackageResult<PathBuf> {
    Ok(manuscript_home()?.join("config.toml"))
}

/// Initialize manuscript directories if they don't exist
pub fn ensure_manuscript_dirs() -> PackageResult<()> {
    let home = manuscript_home()?;
    std::fs::create_dir_all(&home)?;
    std::fs::create_dir_all(global_packages_dir()?)?;
    std::fs::create_dir_all(cache_dir()?)?;
    Ok(())
}

/// Check if we're in a Script package directory
pub fn find_package_root(start_dir: Option<&Path>) -> Option<PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    let start = start_dir.unwrap_or(&current_dir);
    let mut current = start;

    loop {
        let manifest_path = current.join("script.toml");
        if manifest_path.exists() {
            return Some(current.to_path_buf());
        }

        current = current.parent()?;
    }
}
