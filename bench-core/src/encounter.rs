//! Encounter state model.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::effect::{Duration, Resources};
use crate::rng::RngState;
use crate::stat_block::{Condition, StatBlock};

/// One side in an encounter (party or monsters).
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Side {
    pub id: String,
    pub name: String,
}

/// Coarse range band from one combatant to another side.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RangeBand {
    #[default]
    Engaged,
    Near,
    Far,
}

/// Engagement set for a combatant: which other combatants it is in melee with.
pub type Engagement = Vec<String>;

/// End state of a combatant.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EndState {
    #[default]
    Active,
    Unconscious,
    Stable,
    Dead,
}

/// One applied condition instance on a combatant.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ConditionInstance {
    pub condition: Condition,
    pub duration: Duration,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// One active modifier from a timed effect.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ActiveModifier {
    pub target: crate::effect::ModifierTarget,
    pub change: crate::effect::ModifierChange,
    pub remaining: Duration,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Snapshot of a stat block stored in the encounter file, with a tamper hash.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct StatBlockSnapshot {
    pub id: String,
    pub stat_block: StatBlock,
    pub hash: String,
}

impl StatBlockSnapshot {
    /// Build a snapshot from a stat block, computing a hash of the stat block.
    pub fn from_stat_block(id: String, stat_block: StatBlock) -> Self {
        let value = serde_json::to_value(&stat_block).unwrap_or_default();
        let hash = tamper_hash(&value);
        Self {
            id,
            stat_block,
            hash,
        }
    }

    /// Recompute the hash and check it against `self.hash`.
    pub fn verify(&self) -> bool {
        let value = serde_json::to_value(&self.stat_block).unwrap_or_default();
        tamper_hash(&value) == self.hash
    }
}

/// One combatant in play.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Combatant {
    pub id: String,
    pub stat_block_id: String,
    pub name: String,
    pub side_id: String,
    pub hp: i32,
    pub temp_hp: i32,
    pub max_hp: i32,
    pub conditions: Vec<ConditionInstance>,
    pub concentration: Option<String>,
    pub resources: Resources,
    pub action_economy: ActionEconomy,
    pub engagement: Engagement,
    pub ranges: HashMap<String, RangeBand>,
    pub end_state: EndState,
}

/// What a combatant has already done this turn.
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ActionEconomy {
    pub action_used: bool,
    pub bonus_action_used: bool,
    pub reaction_used: bool,
    pub movement_used_feet: u32,
}

/// A standing assumption applied by the agent at intake.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct StandingAssumption {
    pub path: String,
    pub value: serde_json::Value,
    pub reason: String,
}

/// One recorded mutating command in a live encounter.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CommandRecord {
    pub name: String,
    pub args: serde_json::Value,
    /// User-entered die faces for prompts that draw from the RNG. There may be
    /// more than one face when an action asks for multiple dice.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub user_dice: Vec<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// One active effect in the encounter, such as a zone or lingering modifier.
/// Zones and spawns stay unmodeled in phase 1; this struct is the placeholder
/// for any engine-tracked area effect.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ActiveEffect {
    pub id: String,
    pub source_ability: String,
    pub source_combatant: String,
    pub remaining: Duration,
}

/// Full encounter state stored in `~/.bench/encounters/<id>.json`.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EncounterState {
    pub id: String,
    pub engine_version: String,
    pub origin_directory: String,
    pub stat_blocks: Vec<StatBlockSnapshot>,
    pub sides: Vec<Side>,
    pub combatants: Vec<Combatant>,
    pub round: u32,
    pub initiative_order: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_turn: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_effects: Vec<ActiveEffect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub standing_assumptions: Vec<StandingAssumption>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command_history: Vec<CommandRecord>,
    pub rng: RngState,
    pub tamper_hash: String,
}

impl EncounterState {
    /// Compute the tamper hash for this encounter state. The hash excludes the
    /// existing `tamper_hash` field so the value can be verified after the field
    /// is updated.
    pub fn compute_tamper_hash(&self) -> String {
        let mut value = match serde_json::to_value(self) {
            Ok(v) => v,
            Err(_) => return String::new(),
        };
        if let Some(obj) = value.as_object_mut() {
            obj.insert("tamper_hash".into(), serde_json::Value::String(String::new()));
        }
        tamper_hash(&value)
    }

    /// Refresh `self.tamper_hash` to match the rest of the state.
    pub fn update_tamper_hash(&mut self) {
        self.tamper_hash = self.compute_tamper_hash();
    }

    /// Verify that `self.tamper_hash` matches the rest of the state.
    pub fn verify_tamper_hash(&self) -> bool {
        self.compute_tamper_hash() == self.tamper_hash
    }
}

/// Compute a simple tamper hash of a serialized value. Not cryptographic; it is
/// enough to catch accidental edits to the encounter file.
pub fn tamper_hash(value: &serde_json::Value) -> String {
    use sha2::{Digest, Sha256};
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let digest = Sha256::digest(&bytes);
    hex::encode(&digest[..16])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Stream;

    #[test]
    fn tamper_hash_stable_for_same_value() {
        let v = serde_json::json!({"a": 1, "b": "two"});
        assert_eq!(tamper_hash(&v), tamper_hash(&v));
    }

    #[test]
    fn tamper_hash_changes_for_different_value() {
        let a = serde_json::json!({"a": 1});
        let b = serde_json::json!({"a": 2});
        assert_ne!(tamper_hash(&a), tamper_hash(&b));
    }

    #[test]
    fn rng_state_round_trip() {
        let mut stream = Stream::from_seed(123);
        stream.draw_u32();
        let state = stream.state();
        let restored = Stream::from_state(state.clone());
        assert_eq!(restored.state(), state);
    }

    #[test]
    fn stat_block_snapshot_verifies() {
        use crate::stat_block::{AbilityScores, ArmorClass, HitPoints, Senses, Size};
        let stat_block = StatBlock {
            id: "test".into(),
            name: "Test".into(),
            source: "test".into(),
            schema_version: "1".into(),
            size: Size::Medium,
            creature_type: "humanoid".into(),
            alignment: "neutral".into(),
            armor_class: vec![ArmorClass { value: 10, source: None }],
            hit_points: HitPoints { average: 10, dice: "1d8".into() },
            speeds: Default::default(),
            ability_scores: AbilityScores {
                strength: 10,
                dexterity: 10,
                constitution: 10,
                intelligence: 10,
                wisdom: 10,
                charisma: 10,
            },
            saving_throws: Default::default(),
            skills: Default::default(),
            initiative: 0,
            proficiency_bonus: 2,
            damage_vulnerabilities: vec![],
            damage_resistances: vec![],
            damage_immunities: vec![],
            condition_immunities: vec![],
            senses: Senses { passive_perception: 10, ..Default::default() },
            languages: vec![],
            challenge_rating: None,
            xp: None,
            lair_xp: None,
            traits: vec![],
            actions: vec![],
            bonus_actions: vec![],
            reactions: vec![],
            legendary_actions: vec![],
            legendary_uses_per_round: 0,
            class: None,
            level: None,
            spell_slots: None,
            dying: false,
            death_saves: Default::default(),
            gear: vec![],
            pasted_source: None,
            intake_flags: vec![],
        };
        let snapshot = StatBlockSnapshot::from_stat_block("test".into(), stat_block.clone());
        assert!(snapshot.verify());

        let mut altered = stat_block.clone();
        altered.name = "Altered".into();
        let altered_snapshot = StatBlockSnapshot::from_stat_block("test".into(), altered);
        assert_ne!(snapshot.hash, altered_snapshot.hash);
    }

    #[test]
    fn encounter_tamper_hash_excludes_itself() {
        let mut state = EncounterState {
            id: "enc".into(),
            engine_version: "0.1.0".into(),
            origin_directory: "/tmp".into(),
            stat_blocks: vec![],
            sides: vec![],
            combatants: vec![],
            round: 1,
            initiative_order: vec![],
            active_turn: None,
            active_effects: vec![],
            standing_assumptions: vec![],
            command_history: vec![],
            rng: RngState::new(1),
            tamper_hash: String::new(),
        };
        state.update_tamper_hash();
        let first = state.tamper_hash.clone();
        assert!(state.verify_tamper_hash());
        state.round = 2;
        assert!(!state.verify_tamper_hash());
        state.round = 1;
        state.update_tamper_hash();
        assert_eq!(state.tamper_hash, first);
    }
}
