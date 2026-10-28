//! Home and workspace path layout.
//!
//! `~/.bench/` (or `%USERPROFILE%\.bench` on Windows) holds configuration and
//! an encounter index. Encounters and logs are written to either the home
//! directory or a local `./.bench/` workspace directory, depending on config.

use std::path::{Path, PathBuf};

use directories::BaseDirs;

/// Returns the `~/.bench` home directory, creating it if necessary.
pub fn home_dir() -> anyhow::Result<PathBuf> {
    let base = BaseDirs::new()
        .ok_or_else(|| anyhow::anyhow!("could not determine user home directory"))?;
    let path = base.home_dir().join(".bench");
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

/// Returns the local workspace directory for the current working directory.
pub fn workspace_dir(cwd: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
    let path = cwd.as_ref().join(".bench");
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

/// Resolve the active output directory.
///
/// If `output` is `home`, returns `~/.bench/encounters`. If `workspace`,
/// returns `./.bench/encounters` under `cwd`.
pub fn encounters_dir(output: config::OutputTarget, cwd: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
    let base = match output {
        config::OutputTarget::Home => home_dir()?,
        config::OutputTarget::Workspace => workspace_dir(cwd)?,
    };
    let path = base.join("encounters");
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

/// Path to the user-level config file.
pub fn config_file() -> anyhow::Result<PathBuf> {
    Ok(home_dir()?.join("config.toml"))
}

/// Path to the encounter index file.
pub fn index_file() -> anyhow::Result<PathBuf> {
    Ok(home_dir()?.join("index"))
}

use crate::config;
