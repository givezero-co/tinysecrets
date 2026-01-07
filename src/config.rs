//! Local project configuration for TinySecrets
//!
//! Reads `.tinysecrets.toml` from the current directory to provide
//! default project and environment values.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const CONFIG_FILE: &str = ".tinysecrets.toml";

/// Local project configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Default project name
    pub project: Option<String>,
    /// Default environment
    pub environment: Option<String>,
}

impl Config {
    /// Find and load config from current directory or ancestors
    pub fn load() -> Result<Option<Self>> {
        let path = Self::find_config_file()?;
        match path {
            Some(p) => {
                let contents = std::fs::read_to_string(&p)
                    .with_context(|| format!("Failed to read {}", p.display()))?;
                let config: Config = toml::from_str(&contents)
                    .with_context(|| format!("Failed to parse {}", p.display()))?;
                Ok(Some(config))
            }
            None => Ok(None),
        }
    }

    /// Find config file by walking up from current directory
    fn find_config_file() -> Result<Option<PathBuf>> {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        let mut dir = cwd.as_path();

        loop {
            let config_path = dir.join(CONFIG_FILE);
            if config_path.exists() {
                return Ok(Some(config_path));
            }

            match dir.parent() {
                Some(parent) => dir = parent,
                None => return Ok(None),
            }
        }
    }

    /// Get the config file path in current directory
    pub fn config_path() -> Result<PathBuf> {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        Ok(cwd.join(CONFIG_FILE))
    }

    /// Save config to the current directory
    pub fn save(&self) -> Result<PathBuf> {
        let path = Self::config_path()?;
        let contents = toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(&path, contents)
            .with_context(|| format!("Failed to write {}", path.display()))?;
        Ok(path)
    }

    /// Initialize a new config file in current directory
    pub fn init(project: &str, environment: Option<&str>) -> Result<PathBuf> {
        let config = Config {
            project: Some(project.to_string()),
            environment: environment.map(String::from),
        };
        config.save()
    }

    /// Get the path to the found config file (if any)
    pub fn found_path() -> Result<Option<PathBuf>> {
        Self::find_config_file()
    }
}

/// Helper to resolve project/environment from CLI args or config
pub struct ConfigResolver {
    config: Option<Config>,
}

impl ConfigResolver {
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        Ok(Self { config })
    }

    /// Resolve project: use CLI arg if provided, otherwise config
    pub fn project(&self, cli_arg: Option<&str>) -> Result<String> {
        match cli_arg {
            Some(p) => Ok(p.to_string()),
            None => self
                .config
                .as_ref()
                .and_then(|c| c.project.clone())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "No project specified. Use -p/--project or create a {} file",
                        CONFIG_FILE
                    )
                }),
        }
    }

    /// Resolve environment: use CLI arg if provided, otherwise config
    pub fn environment(&self, cli_arg: Option<&str>) -> Result<String> {
        match cli_arg {
            Some(e) => Ok(e.to_string()),
            None => self
                .config
                .as_ref()
                .and_then(|c| c.environment.clone())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "No environment specified. Use -e/--environment or create a {} file",
                        CONFIG_FILE
                    )
                }),
        }
    }

    /// Get the loaded config (if any)
    pub fn config(&self) -> Option<&Config> {
        self.config.as_ref()
    }
}

