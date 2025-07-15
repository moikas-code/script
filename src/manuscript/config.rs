/// Configuration management for manuscript
use crate::package::{PackageError, PackageResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManuscriptConfig {
    /// Default registry URL
    pub default_registry: String,

    /// Authentication tokens for registries
    pub tokens: std::collections::HashMap<String, String>,

    /// Author information
    pub author: AuthorConfig,

    /// Build defaults
    pub build: BuildConfig,

    /// Network settings
    pub network: NetworkConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorConfig {
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    pub jobs: Option<usize>,
    pub target_dir: Option<PathBuf>,
    pub default_release: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub timeout: u64,
    pub retries: u32,
    pub proxy: Option<String>,
}

impl Default for ManuscriptConfig {
    fn default() -> Self {
        Self {
            default_registry: "https://packages.script.org".to_string(),
            tokens: std::collections::HashMap::new(),
            author: AuthorConfig {
                name: None,
                email: None,
            },
            build: BuildConfig {
                jobs: None,
                target_dir: None,
                default_release: false,
            },
            network: NetworkConfig {
                timeout: 30,
                retries: 3,
                proxy: None,
            },
        }
    }
}

impl ManuscriptConfig {
    /// Load configuration from file
    pub fn load() -> PackageResult<Self> {
        let config_path = crate::manuscript::config_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&config_path)?;
        toml::from_str(&content)
            .map_err(|e| PackageError::ManifestParse(format!("Invalid config file: {e}")))
    }

    /// Save configuration to file
    pub fn save(&self) -> PackageResult<()> {
        let config_path = crate::manuscript::config_path()?;

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self).map_err(|e| {
            PackageError::ManifestParse(format!("Failed to serialize config: {e}"))
        })?;

        std::fs::write(config_path, content)?;
        Ok(())
    }

    /// Get registry token
    pub fn get_token(&self, registry: &str) -> Option<&str> {
        self.tokens.get(registry).map(|s| s.as_str())
    }

    /// Set registry token
    pub fn set_token(&mut self, registry: String, token: String) {
        self.tokens.insert(registry, token);
    }
}
