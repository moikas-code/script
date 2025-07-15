/// Core updater implementation using self_update crate
use self_update::{backends::github::Update, cargo_crate_version, Status};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UpdateError {
    #[error("Self-update error: {0}")]
    SelfUpdate(#[from] self_update::errors::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Version parsing error: {0}")]
    Version(String),

    #[error("Rollback failed: {0}")]
    Rollback(String),
}

pub struct ScriptUpdater {
    repo_owner: String,
    repo_name: String,
    bin_name: String,
    backup_dir: PathBuf,
}

impl ScriptUpdater {
    pub fn new() -> Result<Self, UpdateError> {
        let backup_dir = Self::get_backup_dir()?;
        fs::create_dir_all(&backup_dir)?;

        Ok(Self {
            repo_owner: "moikapy".to_string(),
            repo_name: "script".to_string(),
            bin_name: "script".to_string(),
            backup_dir,
        })
    }

    /// Get the latest version available
    pub fn get_latest_version(&self) -> Result<Option<String>, UpdateError> {
        let releases = self.create_update_builder().build()?.get_latest_release()?;

        Ok(Some(releases.version))
    }

    /// Get all available versions
    pub fn get_available_versions(&self) -> Result<Vec<String>, UpdateError> {
        // For now, we'll just return a few example versions
        // In a real implementation, this would query GitHub API directly
        let mut versions = vec![cargo_crate_version!().to_string(), "0.1.0".to_string()];

        // Sort versions in descending order (newest first)
        versions.sort_by(
            |a, b| match (semver::Version::parse(a), semver::Version::parse(b)) {
                (Ok(a_ver), Ok(b_ver)) => b_ver.cmp(&a_ver),
                _ => b.cmp(a),
            },
        );

        Ok(versions)
    }

    /// Update to the latest version
    pub fn update(&self) -> Result<Status, UpdateError> {
        // Backup current binary before updating
        self.backup_current_binary()?;

        let status = self.create_update_builder().build()?.update()?;

        Ok(status)
    }

    /// Update to a specific version
    pub fn update_to_version(&self, version: &str) -> Result<Status, UpdateError> {
        // Validate version format
        let target_version = if version.starts_with('v') {
            version.to_string()
        } else {
            format!("v{version}")
        };

        // Backup current binary before updating
        self.backup_current_binary()?;

        let status = self
            .create_update_builder()
            .target(&target_version)
            .build()?
            .update()?;

        Ok(status)
    }

    /// Rollback to the previous version
    pub fn rollback(&self) -> Result<Option<String>, UpdateError> {
        let backup_path = self.get_latest_backup()?;

        if let Some(backup) = backup_path {
            let current_exe = std::env::current_exe()
                .map_err(|e| UpdateError::Rollback(format!("Failed to get current exe: {e}")))?;

            // Copy backup over current executable
            fs::copy(&backup, &current_exe)
                .map_err(|e| UpdateError::Rollback(format!("Failed to restore backup: {e}")))?;

            // Extract version from backup filename
            let version = backup
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.split('_').nth(1))
                .map(|s| s.to_string())
                .ok_or_else(|| {
                    UpdateError::Rollback("Failed to parse backup version".to_string())
                })?;

            Ok(Some(version))
        } else {
            Ok(None)
        }
    }

    /// Create the update builder with common configuration
    fn create_update_builder(&self) -> self_update::backends::github::UpdateBuilder {
        let mut builder = Update::configure();
        builder
            .repo_owner(&self.repo_owner)
            .repo_name(&self.repo_name)
            .bin_name(&self.bin_name)
            .show_download_progress(true)
            .current_version(cargo_crate_version!())
            .no_confirm(true);
        builder
    }

    /// Backup the current binary
    fn backup_current_binary(&self) -> Result<(), UpdateError> {
        let current_exe = std::env::current_exe()?;
        let current_version = cargo_crate_version!();

        let backup_name = format!("script_{}.backup", current_version);
        let backup_path = self.backup_dir.join(backup_name);

        fs::copy(&current_exe, &backup_path)?;

        // Keep only the last 3 backups
        self.cleanup_old_backups(3)?;

        Ok(())
    }

    /// Get the most recent backup
    fn get_latest_backup(&self) -> Result<Option<PathBuf>, UpdateError> {
        let mut backups: Vec<_> = fs::read_dir(&self.backup_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "backup")
                    .unwrap_or(false)
            })
            .collect();

        backups.sort_by_key(|entry| {
            entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });

        Ok(backups.last().map(|entry| entry.path()))
    }

    /// Clean up old backups, keeping only the specified number
    fn cleanup_old_backups(&self, keep_count: usize) -> Result<(), UpdateError> {
        let mut backups: Vec<_> = fs::read_dir(&self.backup_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "backup")
                    .unwrap_or(false)
            })
            .collect();

        if backups.len() <= keep_count {
            return Ok(());
        }

        backups.sort_by_key(|entry| {
            entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });

        // Remove old backups
        let remove_count = backups.len() - keep_count;
        for entry in backups.into_iter().take(remove_count) {
            fs::remove_file(entry.path())?;
        }

        Ok(())
    }

    /// Get the backup directory
    fn get_backup_dir() -> Result<PathBuf, UpdateError> {
        let base_dir = dirs::data_local_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join(".local").join("share")))
            .ok_or_else(|| {
                UpdateError::Rollback("Failed to determine data directory".to_string())
            })?;

        Ok(base_dir.join("script").join("backups"))
    }
}
