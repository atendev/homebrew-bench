//! Engine-owned spell effects file model.

use serde::{Deserialize, Serialize};

use crate::effect::{Duration, Effect};
use crate::stat_block::DamageType;

/// Upcast or slot-level scaling rule attached to one effect.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ScalingRule {
    pub effect_index: usize,
    pub per_slot: Scaling,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Scaling {
    ExtraDice { dice: String },
    ExtraTargets { count: u32 },
    IncreaseDuration { rounds: u32 },
    AddDamageType { damage_type: DamageType, dice: String },
}

/// A delayed effect triggered after some duration or condition.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct DelayedEffect {
    pub delay: Duration,
    pub effect: Effect,
}

/// One entry in `data/spells/effects.json`.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SpellEffect {
    pub id: String,
    pub name: String,
    pub source: String,
    pub concentration: bool,
    pub effects: Vec<Effect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scaling: Vec<ScalingRule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub delayed: Vec<DelayedEffect>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn effects_file_validates_against_schema() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .to_path_buf();
        let path = root.join("data/spells/effects.json");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let value: serde_json::Value = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("effects.json is not valid json: {e}"));
        let map = value.as_object().expect("effects.json must be an object");
        let mut failures = Vec::new();
        for (key, entry_value) in map {
            let result: Result<SpellEffect, _> = serde_json::from_value(entry_value.clone());
            if let Err(e) = result {
                failures.push(format!("{key}: {e}"));
            }
        }
        if !failures.is_empty() {
            panic!(
                "{} spell effect entries failed schema validation:\n{}",
                failures.len(),
                failures.join("\n")
            );
        }
    }
}
