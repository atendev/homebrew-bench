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
