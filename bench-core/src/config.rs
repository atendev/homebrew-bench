//! User configuration stored in `~/.bench/config.toml`.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Where encounter files and logs are written.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OutputTarget {
    /// Write to `~/.bench/encounters/`.
    #[default]
    Home,
    /// Write to `./.bench/encounters/` in the current working directory.
    Workspace,
}

/// User-level configuration.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Config {
    #[serde(default)]
    pub output: OutputTarget,
    #[serde(default = "default_true")]
    pub keep_encounters: bool,
    #[serde(default = "default_false")]
    pub delete_logs: bool,
    #[serde(default = "default_round_cap")]
    pub round_cap: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            output: OutputTarget::default(),
            keep_encounters: true,
            delete_logs: false,
            round_cap: 20,
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}
fn default_round_cap() -> u32 {
    20
}

impl Config {
    /// Load config from a path, or return defaults if the file is missing.
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&text)?;
        Ok(config)
    }

    /// Save config to a path, ensuring the parent directory exists.
    pub fn save(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self)?;
        std::fs::write(path, text)?;
        Ok(())
    }
}
