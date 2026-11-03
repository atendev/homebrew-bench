//! Canonical stat block model shared by monsters, SRD fixtures, and the standard party.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::effect::{Effect, Usage};

/// Ability score names.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Ability {
    Str,
    Dex,
    Con,
    Int,
    Wis,
    Cha,
}

impl Ability {
    /// Modifier for a raw ability score.
    pub fn modifier(score: i32) -> i32 {
        (score - 10).div_euclid(2)
    }
}

/// Creature size categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Size {
    Tiny,
    Small,
    Medium,
    Large,
    Huge,
    Gargantuan,
}

/// The fifteen conditions in the 2024 rules.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    Blinded,
    Charmed,
    Deafened,
    Exhaustion,
    Frightened,
    Grappled,
    Incapacitated,
    Invisible,
    Paralyzed,
    Petrified,
    Poisoned,
    Prone,
    Restrained,
    Stunned,
    Unconscious,
}

/// Damage types.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum DamageType {
    Acid,
    Bludgeoning,
    Cold,
    Fire,
    Force,
    Lightning,
    Necrotic,
    Piercing,
    Poison,
    Psychic,
    Radiant,
    Slashing,
    Thunder,
}

/// How an unmodeled ability is resolved by default for SRD imports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DefaultResolution {
    /// No combat effect.
    Neutral,
    /// Keeps a shape-changer in its stat-block form.
    BaseForm,
    /// Skipped; report warns that numbers may change.
    Skipped,
}

/// Ability classification.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AbilityKind {
    /// Fully modeled; engine runs all effects.
    Modeled,
    /// Engine runs `effects` plus `agent_clauses` the agent resolves.
    PartlyModeled,
    /// No combat effect; text kept for reference.
    Inert,
    /// No effects; text only.
    #[default]
    Unmodeled,
}

/// Armor class entry. A creature may have several AC sources; the first is the
/// effective one unless an effect changes it.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ArmorClass {
    pub value: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Hit points with average and underlying dice expression.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct HitPoints {
    pub average: i32,
    pub dice: String,
}

/// Speed value for one movement mode, such as walk or fly.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SpeedValue {
    pub feet: u32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hover: bool,
}

/// The six ability scores.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AbilityScores {
    pub strength: i32,
    pub dexterity: i32,
    pub constitution: i32,
    pub intelligence: i32,
    pub wisdom: i32,
    pub charisma: i32,
}

impl AbilityScores {
    pub fn score(&self, ability: Ability) -> i32 {
        match ability {
            Ability::Str => self.strength,
            Ability::Dex => self.dexterity,
            Ability::Con => self.constitution,
            Ability::Int => self.intelligence,
            Ability::Wis => self.wisdom,
            Ability::Cha => self.charisma,
        }
    }

    pub fn modifier(&self, ability: Ability) -> i32 {
        Ability::modifier(self.score(ability))
    }
}

/// Senses block.
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Senses {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub darkvision: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blindsight: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tremorsense: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truesight: Option<u32>,
    pub passive_perception: i32,
}

/// Death save tally for a PC.
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct DeathSaves {
    pub successes: u8,
    pub failures: u8,
}

/// Spellcasting ability and derived numbers for a PC. Spell effects in
/// `data/spells/effects.json` carry no save DC or attack modifier of their own;
/// the caster's block supplies them.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Spellcasting {
    pub ability: Ability,
    pub save_dc: i32,
    pub attack_bonus: i32,
}

/// Intake flag for ambiguous wording or data problems.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct IntakeFlag {
    pub path: String,
    pub note: String,
}

/// One ability on a stat block: trait, action, bonus action, reaction, or
/// legendary action.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AbilityEntry {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
    #[serde(default)]
    pub kind: AbilityKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<Effect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agent_clauses: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_resolution: Option<DefaultResolution>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

impl AbilityEntry {
    /// Flyby and Agile: the creature never provokes an Opportunity Attack by moving.
    pub fn is_no_provoke(&self) -> bool {
        self.effects.iter().any(|e| matches!(e, crate::effect::Effect::NoProvoke))
    }
}

/// A complete stat block. Monsters, SRD fixtures, and the standard party share
/// this schema.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct StatBlock {
    pub id: String,
    pub name: String,
    pub source: String,
    pub schema_version: String,
    pub size: Size,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub size_choice: Vec<Size>,
    #[serde(rename = "type")]
    pub creature_type: String,
    pub alignment: String,
    pub armor_class: Vec<ArmorClass>,
    pub hit_points: HitPoints,
    pub speeds: HashMap<String, SpeedValue>,
    pub ability_scores: AbilityScores,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub saving_throws: HashMap<Ability, i32>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub skills: HashMap<String, i32>,
    pub initiative: i32,
    pub proficiency_bonus: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub damage_vulnerabilities: Vec<DamageType>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub damage_resistances: Vec<DamageType>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub damage_immunities: Vec<DamageType>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub condition_immunities: Vec<Condition>,
    pub senses: Senses,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub challenge_rating: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lair_xp: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub traits: Vec<AbilityEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<AbilityEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bonus_actions: Vec<AbilityEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reactions: Vec<AbilityEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub legendary_actions: Vec<AbilityEntry>,
    #[serde(default)]
    pub legendary_uses_per_round: u32,
    // PC-only fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    /// Spell slot maxima by slot level; index 0 is 1st level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spell_slots: Option<Vec<u32>>,
    #[serde(default)]
    pub dying: bool,
    #[serde(default)]
    pub death_saves: DeathSaves,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gear: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spellcasting: Option<Spellcasting>,
    /// Designer notes with no combat effect, such as the class a standard
    /// party archetype approximates.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pasted_source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub intake_flags: Vec<IntakeFlag>,
}

impl StatBlock {
    /// The first ability, in any list, that stops this creature provoking
    /// Opportunity Attacks.
    pub fn no_provoke_ability(&self) -> Option<&AbilityEntry> {
        self.traits
            .iter()
            .chain(&self.actions)
            .chain(&self.bonus_actions)
            .chain(&self.reactions)
            .chain(&self.legendary_actions)
            .find(|a| a.is_no_provoke())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ability_modifier() {
        assert_eq!(Ability::modifier(10), 0);
        assert_eq!(Ability::modifier(11), 0);
        assert_eq!(Ability::modifier(12), 1);
        assert_eq!(Ability::modifier(9), -1);
    }

    #[test]
    fn round_trip_minimal_stat_block() {
        let block = StatBlock {
            id: "test-goblin".into(),
            name: "Test Goblin".into(),
            source: "test".into(),
            schema_version: "1".into(),
            size: Size::Small,
            size_choice: vec![],
            creature_type: "humanoid".into(),
            alignment: "neutral evil".into(),
            armor_class: vec![ArmorClass {
                value: 15,
                source: Some("leather armor".into()),
            }],
            hit_points: HitPoints {
                average: 7,
                dice: "2d6".into(),
            },
            speeds: {
                let mut m = HashMap::new();
                m.insert("walk".into(), SpeedValue { feet: 30, hover: false });
                m
            },
            ability_scores: AbilityScores {
                strength: 8,
                dexterity: 14,
                constitution: 10,
                intelligence: 10,
                wisdom: 8,
                charisma: 8,
            },
            saving_throws: HashMap::new(),
            skills: HashMap::new(),
            initiative: 2,
            proficiency_bonus: 2,
            damage_vulnerabilities: vec![],
            damage_resistances: vec![],
            damage_immunities: vec![],
            condition_immunities: vec![],
            senses: Senses {
                passive_perception: 9,
                ..Default::default()
            },
            languages: vec![],
            challenge_rating: Some(0.25),
            xp: Some(50),
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
            death_saves: DeathSaves::default(),
            gear: vec![],
            spellcasting: None,
            notes: vec![],
            pasted_source: None,
            intake_flags: vec![],
        };
        let json = serde_json::to_string(&block).unwrap();
        let parsed: StatBlock = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, block.id);
        assert_eq!(parsed.ability_scores.dexterity, 14);
    }
}
