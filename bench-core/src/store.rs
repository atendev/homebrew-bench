//! Encounter files on disk: the state file, its plain-text log, and the
//! `~/.bench/index` that lets any directory find any encounter.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::encounter::EncounterState;
use crate::error::{CodedError, ErrorCode};
use crate::{log, paths, ENGINE_VERSION};

/// One encounter's line in the index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub id: String,
    pub state_path: PathBuf,
    pub log_path: PathBuf,
    pub origin_directory: String,
    pub started_unix: u64,
    #[serde(default)]
    pub ended: bool,
}

/// The encounter index stored as JSON at `~/.bench/index`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Index {
    pub encounters: Vec<IndexEntry>,
}

fn io(context: impl std::fmt::Display) -> impl FnOnce(std::io::Error) -> CodedError {
    move |e| CodedError::new(ErrorCode::Internal, format!("{context}: {e}"))
}

impl Index {
    pub fn load() -> Result<Self, CodedError> {
        let path = paths::index_file()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path).map_err(io(format!("failed to read {}", path.display())))?;
        if text.trim().is_empty() {
            return Ok(Self::default());
        }
        serde_json::from_str(&text)
            .map_err(|e| CodedError::new(ErrorCode::InvalidJson, format!("{} is not a valid index: {e}", path.display())))
    }

    pub fn save(&self) -> Result<(), CodedError> {
        let path = paths::index_file()?;
        let text = serde_json::to_string_pretty(self).map_err(|e| CodedError::new(ErrorCode::Internal, e.to_string()))?;
        std::fs::write(&path, text).map_err(io(format!("failed to write {}", path.display())))
    }

    pub fn find(&self, id: &str) -> Option<&IndexEntry> {
        self.encounters.iter().find(|e| e.id == id)
    }

    /// The named encounter, or the most recently started one still running.
    pub fn resolve(&self, id: Option<&str>) -> Result<&IndexEntry, CodedError> {
        match id {
            Some(id) => self
                .find(id)
                .ok_or_else(|| CodedError::new(ErrorCode::NotFound, format!("no encounter `{id}` in the index; see `bench list`"))),
            None => self
                .encounters
                .iter()
                .rev()
                .find(|e| !e.ended)
                .ok_or_else(|| CodedError::new(ErrorCode::NotFound, "no running encounter; pass --encounter <id> or `bench start` one")),
        }
    }

    /// A readable id, such as `goblin-3`, that is not in the index and has
    /// no state file or kept log in `dir`.
    pub fn next_id(&self, stem: &str, dir: &Path) -> String {
        let stem = slug(stem);
        (1..)
            .map(|n| format!("{stem}-{n}"))
            .find(|id| {
                let (state, log) = files_for(dir, id);
                self.find(id).is_none() && !state.exists() && !log.exists()
            })
            .expect("free id")
    }

    pub fn upsert(&mut self, entry: IndexEntry) {
        match self.encounters.iter_mut().find(|e| e.id == entry.id) {
            Some(existing) => *existing = entry,
            None => self.encounters.push(entry),
        }
    }
}

fn slug(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() {
        "encounter".into()
    } else {
        out
    }
}

/// Seconds since the Unix epoch.
pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

/// Paths for a new encounter's state file and log in `dir`.
pub fn files_for(dir: &Path, id: &str) -> (PathBuf, PathBuf) {
    (dir.join(format!("{id}.json")), dir.join(format!("{id}.log")))
}

/// Write the state file and re-render its plain-text log.
pub fn save(state: &EncounterState, entry: &IndexEntry) -> Result<(), CodedError> {
    let text = serde_json::to_string_pretty(state).map_err(|e| CodedError::new(ErrorCode::Internal, e.to_string()))?;
    std::fs::write(&entry.state_path, text).map_err(io(format!("failed to write {}", entry.state_path.display())))?;
    std::fs::write(&entry.log_path, log::render(&state.log) + "\n")
        .map_err(io(format!("failed to write {}", entry.log_path.display())))
}

/// How strictly `load` checks the engine version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionCheck {
    Require,
    Ignore,
}

/// Read a state file and refuse it if it was edited outside the engine.
pub fn load(path: &Path, version: VersionCheck) -> Result<EncounterState, CodedError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| CodedError::new(ErrorCode::NotFound, format!("failed to read {}: {e}", path.display())))?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| CodedError::new(ErrorCode::StateTampered, format!("{} is not valid JSON: {e}", path.display())))?;
    let state: EncounterState = serde_path_to_error::deserialize(value).map_err(|e| {
        CodedError::new(
            ErrorCode::StateTampered,
            format!("{} does not match the state schema at `{}`: {}", path.display(), e.path(), e.inner()),
        )
    })?;
    if version == VersionCheck::Require && !compatible(&state.engine_version) {
        return Err(CodedError::new(
            ErrorCode::EngineVersionMismatch,
            format!(
                "{} was written by engine {} and this is engine {ENGINE_VERSION}; finish it with that version or run `bench replay` to see where it diverges",
                path.display(),
                state.engine_version
            ),
        ));
    }
    check(&state, path)?;
    Ok(state)
}

/// Tamper check: the file's own hash must match, and the last log entry must
/// record the game state the file holds.
pub fn check(state: &EncounterState, path: &Path) -> Result<(), CodedError> {
    let tampered = |why: &str| {
        CodedError::new(
            ErrorCode::StateTampered,
            format!("{} was changed outside the engine ({why}); use `bench override` with a reason instead", path.display()),
        )
    };
    if !state.verify_tamper_hash() {
        return Err(tampered("its tamper hash does not match"));
    }
    match state.log.last() {
        Some(entry) if entry.state_hash == state.game_hash() => {}
        Some(_) => return Err(tampered("the state differs from the last state the log recorded")),
        None => return Err(tampered("it has no log")),
    }
    if state.stat_blocks.iter().any(|s| !s.verify()) {
        return Err(tampered("a stat block snapshot hash does not match"));
    }
    Ok(())
}

/// Same major and minor version (or same minor and patch before 1.0).
pub fn compatible(recorded: &str) -> bool {
    let parts = |v: &str| v.split('.').map(str::to_string).collect::<Vec<_>>();
    let (a, b) = (parts(recorded), parts(ENGINE_VERSION));
    if a.len() < 2 || b.len() < 2 {
        return recorded == ENGINE_VERSION;
    }
    if b[0] == "0" {
        a[0] == b[0] && a[1] == b[1]
    } else {
        a[0] == b[0]
    }
}

/// Check a `--rule` slug against both house-rule folders.
pub fn validate_rule(slug: &str, cwd: &Path) -> Result<(), CodedError> {
    let dirs = paths::house_rule_dirs(cwd)?;
    let valid_slug = !slug.is_empty() && !slug.contains(['/', '\\']) && !slug.starts_with('.');
    if valid_slug && dirs.iter().any(|d| d.join(format!("{slug}.md")).is_file()) {
        return Ok(());
    }
    let searched: Vec<String> = dirs.iter().map(|d| d.display().to_string()).collect();
    Err(CodedError::new(
        ErrorCode::UnknownHouseRule,
        format!("no house rule `{slug}.md` in {}", searched.join(" or ")),
    )
    .with_details(serde_json::json!({ "searched": searched })))
}

/// Delete an ended encounter's state file, and its log when `delete_log`.
pub fn delete(entry: &IndexEntry, delete_log: bool) -> Result<(), CodedError> {
    if entry.state_path.exists() {
        std::fs::remove_file(&entry.state_path).map_err(io(format!("failed to delete {}", entry.state_path.display())))?;
    }
    if delete_log && entry.log_path.exists() {
        std::fs::remove_file(&entry.log_path).map_err(io(format!("failed to delete {}", entry.log_path.display())))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_and_next_id() {
        assert_eq!(slug("Young Red Dragon"), "young-red-dragon");
        assert_eq!(slug("!!"), "encounter");
        let mut index = Index::default();
        assert_eq!(index.next_id("goblin", Path::new("/nonexistent")), "goblin-1");
        index.upsert(IndexEntry {
            id: "goblin-1".into(),
            state_path: "a".into(),
            log_path: "b".into(),
            origin_directory: "/".into(),
            started_unix: 0,
            ended: false,
        });
        assert_eq!(index.next_id("goblin", Path::new("/nonexistent")), "goblin-2");
    }

    #[test]
    fn pre_one_versions_need_matching_minor() {
        assert!(compatible(ENGINE_VERSION));
        assert!(!compatible("99.0.0"));
    }
}
