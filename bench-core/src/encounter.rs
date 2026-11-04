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
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ends_on_damage: bool,
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
    #[serde(default)]
    pub death_saves: crate::stat_block::DeathSaves,
}

/// What a combatant has already done this turn.
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ActionEconomy {
    pub action_used: bool,
    pub bonus_action_used: bool,
    pub reaction_used: bool,
    pub movement_used_feet: u32,
    /// Took the Disengage action this turn; its moves provoke nothing until the turn ends (ADR 0015).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub disengaged: bool,
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
    /// Game hash of the state after this command, compared by `bench replay`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub state_hash: String,
}

/// Who rolls a PC's dice in a live encounter.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PcDice {
    #[default]
    Engine,
    Ask,
}

/// Per-encounter settings fixed at `bench start`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EncounterSettings {
    #[serde(default)]
    pub pc_dice: PcDice,
    /// The whole encounter's lighting, read by roll modifiers on traits. Left out when
    /// `unspecified`, so state files written before it existed keep their hashes.
    #[serde(default, skip_serializing_if = "crate::effect::Lighting::is_unspecified")]
    pub lighting: crate::effect::Lighting,
}

/// Record of an encounter ended early through `bench end`.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EncounterEnd {
    pub round: u32,
    pub reason: String,
}

/// A prompt that must be answered before the current action can finish.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PendingPrompt {
    pub id: String,
    pub kind: String,
    pub actor_id: String,
    pub message: String,
    pub sides: Option<u32>,
    pub remaining: u32,
    #[serde(default)]
    pub metadata: serde_json::Value,
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
    pub active_modifiers: Vec<ActiveModifier>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub standing_assumptions: Vec<StandingAssumption>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command_history: Vec<CommandRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending: Option<PendingPrompt>,
    pub rng: RngState,
    /// Initiative totals by combatant id, used to slot mid-fight additions.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub initiative_rolls: HashMap<String, i32>,
    #[serde(default)]
    pub settings: EncounterSettings,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended: Option<EncounterEnd>,
    /// State right after `bench start`, with empty history and log. Undo and
    /// replay re-execute `command_history` from here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_snapshot: Option<Box<EncounterState>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub log: Vec<crate::log::LogEntry>,
    pub tamper_hash: String,
    /// User-entered faces for the command being resolved. Never stored.
    #[serde(skip)]
    #[schemars(skip)]
    pub player_dice: PlayerDice,
}

/// Faces the user entered for PC rolls, consumed in draw order while one
/// command resolves.
#[derive(Debug, Clone, Default)]
pub struct PlayerDice {
    pub faces: std::collections::VecDeque<u32>,
    /// PC rolls that found no face, in draw order.
    pub missing: Vec<MissingDie>,
    /// Multiattack picks for the next action, one entry per `Choice` (option indexes) or
    /// `Alternatives` (one set index) step in resolution order. Empty means the first
    /// options. Supplied by the batch tactics policy; never stored.
    pub picks: std::collections::VecDeque<Vec<usize>>,
    /// Batch mode: attacks aimed at a downed creature move to a living opponent.
    pub retarget_downed: bool,
    /// Per owner since its last event: (some die used an entered face, some die did not).
    pub since_event: HashMap<String, (bool, bool)>,
    /// Use the engine's draw for any roll past the entered faces.
    pub engine_fills_rest: bool,
    pub next_group: u32,
}

/// A PC roll still waiting for a user-entered face. Dice in one group (one
/// damage expression, or an advantage pair) are always drawn together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingDie {
    pub sides: u32,
    pub owner: String,
    pub group: u32,
}

impl PlayerDice {
    pub fn take(&mut self, owner: &str, sides: u32, drawn: u32, group_start: bool) -> u32 {
        if group_start {
            self.next_group += 1;
        }
        let missing = MissingDie { sides, owner: owner.into(), group: self.next_group };
        if !self.missing.is_empty() {
            self.missing.push(missing);
            return drawn;
        }
        match self.faces.pop_front() {
            Some(face) => {
                self.since_event.entry(owner.into()).or_default().0 = true;
                face
            }
            None => {
                self.since_event.entry(owner.into()).or_default().1 = true;
                if !self.engine_fills_rest {
                    self.missing.push(missing);
                }
                drawn
            }
        }
    }

    /// Whether every die `owner` rolled since its last event used an entered
    /// face, then start counting afresh for its next event.
    pub fn all_entered_since(&mut self, owner: &str) -> bool {
        matches!(self.since_event.remove(owner), Some((true, false)))
    }

    /// The dice to ask for next: the first missing group.
    pub fn next_prompt(&self) -> Option<(u32, String, u32)> {
        let first = self.missing.first()?;
        let count = self.missing.iter().take_while(|d| d.group == first.group).count() as u32;
        Some((first.sides, first.owner.clone(), count))
    }
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

    /// Hash of the game state alone, leaving out the log, command history,
    /// initial snapshot and tamper hash. Log entries and command records carry
    /// it, and replay compares it command by command.
    pub fn game_hash(&self) -> String {
        let mut value = match serde_json::to_value(self) {
            Ok(v) => v,
            Err(_) => return String::new(),
        };
        if let Some(obj) = value.as_object_mut() {
            for key in ["tamper_hash", "log", "command_history", "initial_snapshot"] {
                obj.remove(key);
            }
        }
        tamper_hash(&value)
    }

    /// Copy of the state with bookkeeping cleared, used as the replay origin.
    pub fn bare(&self) -> EncounterState {
        let mut bare = self.clone();
        bare.log.clear();
        bare.command_history.clear();
        bare.initial_snapshot = None;
        bare.tamper_hash.clear();
        bare
    }

    /// Whether a combatant id belongs to the party side, which the public view
    /// shows in full.
    pub fn is_party(&self, combatant_id: &str) -> bool {
        self.combatants
            .iter()
            .find(|c| c.id == combatant_id)
            .is_some_and(|c| c.side_id == PARTY_SIDE)
    }
}

/// Side id of the player party. Every other side is hidden in the public view.
pub const PARTY_SIDE: &str = "party";

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
            size_choice: vec![],
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
            spellcasting: None,
            notes: vec![],
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
            active_modifiers: vec![],
            standing_assumptions: vec![],
            command_history: vec![],
            pending: None,
            rng: RngState::new(1),
            initiative_rolls: HashMap::new(),
            settings: EncounterSettings::default(),
            ended: None,
            initial_snapshot: None,
            log: vec![],
            tamper_hash: String::new(),
            player_dice: Default::default(),
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

#[cfg(test)]
mod disengage_tests {
    use super::*;

    #[test]
    fn action_economy_without_the_disengaged_flag_still_loads() {
        let old = r#"{"action_used":true,"bonus_action_used":false,"reaction_used":false,"movement_used_feet":10}"#;
        let economy: ActionEconomy = serde_json::from_str(old).unwrap();
        assert!(economy.action_used && !economy.disengaged);
        let unchanged = serde_json::to_value(&economy).unwrap();
        assert!(unchanged.get("disengaged").is_none(), "{unchanged}");
        assert_eq!(serde_json::from_value::<ActionEconomy>(unchanged).unwrap().movement_used_feet, 10);
        let set = ActionEconomy { disengaged: true, ..economy };
        assert_eq!(serde_json::to_value(&set).unwrap()["disengaged"], true);
    }
}
