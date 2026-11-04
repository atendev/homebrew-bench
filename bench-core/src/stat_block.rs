//! Canonical stat block model shared by monsters, SRD fixtures, and the standard party.

use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

use crate::effect::{Effect, Lighting, ReactionTrigger, Usage};

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
    /// On a reaction, the trigger that opens a reaction window for it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<ReactionTrigger>,
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
    /// Standing traits; the only place a `roll_modifier` may appear, and only as a top-level effect.
    #[serde(default, skip_serializing_if = "Vec::is_empty", deserialize_with = "trait_entries")]
    pub traits: Vec<AbilityEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", deserialize_with = "action_entries")]
    pub actions: Vec<AbilityEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", deserialize_with = "action_entries")]
    pub bonus_actions: Vec<AbilityEntry>,
    /// The only list a `trigger` may appear in.
    #[serde(default, skip_serializing_if = "Vec::is_empty", deserialize_with = "reaction_entries")]
    pub reactions: Vec<AbilityEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", deserialize_with = "action_entries")]
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

fn trait_entries<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<AbilityEntry>, D::Error> {
    let entries = Vec::<AbilityEntry>::deserialize(deserializer)?;
    for entry in &entries {
        check_trigger(entry, false)?;
        for effect in &entry.effects {
            match effect {
                Effect::RollModifier(m) if m.lighting == Lighting::Unspecified => {
                    return Err(serde::de::Error::custom(format!("{}: a roll modifier can't be keyed to unspecified lighting", entry.id)));
                }
                Effect::RollModifier(_) => {}
                nested => refuse_roll_modifier(entry, nested)?,
            }
        }
    }
    Ok(entries)
}

fn action_entries<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<AbilityEntry>, D::Error> {
    checked_entries(deserializer, false)
}

fn reaction_entries<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<AbilityEntry>, D::Error> {
    checked_entries(deserializer, true)
}

fn checked_entries<'de, D: Deserializer<'de>>(deserializer: D, reactions: bool) -> Result<Vec<AbilityEntry>, D::Error> {
    let entries = Vec::<AbilityEntry>::deserialize(deserializer)?;
    for entry in &entries {
        check_trigger(entry, reactions)?;
        for effect in &entry.effects {
            refuse_roll_modifier(entry, effect)?;
        }
    }
    Ok(entries)
}

fn check_trigger<E: serde::de::Error>(entry: &AbilityEntry, reaction: bool) -> Result<(), E> {
    if entry.trigger.is_some() && !reaction {
        return Err(E::custom(format!("{}: a trigger belongs on a reaction", entry.id)));
    }
    let hit_trigger = matches!(entry.trigger, Some(ReactionTrigger::HitByAttackRoll { .. }));
    if !hit_trigger && entry.effects.iter().any(Effect::holds_triggering_attack) {
        return Err(E::custom(format!("{}: a triggering_attack duration needs a hit trigger", entry.id)));
    }
    Ok(())
}

fn refuse_roll_modifier<E: serde::de::Error>(entry: &AbilityEntry, effect: &Effect) -> Result<(), E> {
    if effect.holds_roll_modifier() {
        return Err(E::custom(format!("{}: a roll modifier is a trait effect", entry.id)));
    }
    Ok(())
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

/// Warnings for stat block JSON whose top-level timed modifiers name no `bearer`. Since engine
/// 0.1.6 such a modifier lands on the ability's target, so a self-buff written before then
/// (a reaction's AC bonus, "the creature gains...") silently moves to its target. A
/// `triggering_attack` modifier is exempt: it always lands on its reactor.
pub fn bearer_warnings(value: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    for section in ["traits", "actions", "bonus_actions", "reactions", "legendary_actions"] {
        for entry in value.get(section).and_then(|v| v.as_array()).into_iter().flatten() {
            let id = entry.get("id").and_then(|v| v.as_str()).unwrap_or("?");
            for effect in entry.get("effects").and_then(|v| v.as_array()).into_iter().flatten() {
                let Some(modifier) = effect.get("timed_modifier") else { continue };
                if modifier.get("bearer").is_some() || modifier.get("duration").and_then(|d| d.as_str()) == Some("triggering_attack") {
                    continue;
                }
                out.push(format!(
                    "{section} {id}: a timed modifier with no \"bearer\" lands on the ability's target; add \"bearer\": \"self\" if it changes the creature itself, or \"bearer\": \"target\" to silence this"
                ));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_top_level_modifier_without_a_bearer_warns() {
        let block = serde_json::json!({
            "reactions": [
                {"id": "brace", "effects": [{"timed_modifier": {"target": "armor_class", "change": {"add": 2}, "duration": {"rounds": 1}}}]},
                {"id": "parry", "effects": [{"timed_modifier": {"target": "armor_class", "change": {"add": 2}, "duration": "triggering_attack"}}]},
            ],
            "actions": [
                {"id": "howl", "effects": [{"timed_modifier": {"target": "speed", "change": "double", "duration": "concentration", "bearer": "self"}}]},
                {"id": "bite", "effects": [{"attack": {"on_hit": [{"timed_modifier": {"target": "speed", "change": "halve", "duration": {"rounds": 1}}}]}}]},
            ],
        });
        let warnings = bearer_warnings(&block);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].starts_with("reactions brace: "));
    }

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

    fn kobold_with(edit: impl FnOnce(&mut serde_json::Value)) -> Result<StatBlock, String> {
        let mut value: serde_json::Value = serde_json::from_str(crate::embedded::fixture_json("kobold-warrior").unwrap()).unwrap();
        edit(&mut value);
        serde_json::from_value(value).map_err(|e| e.to_string())
    }

    fn sunlight() -> serde_json::Value {
        serde_json::json!({"roll_modifier": {"lighting": "sunlight", "disposition": "disadvantage", "target": "attack_rolls"}})
    }

    #[test]
    fn a_roll_modifier_on_a_trait_loads() {
        let block = kobold_with(|v| v["traits"][1]["effects"] = serde_json::json!([sunlight()])).unwrap();
        assert!(matches!(block.traits[1].effects[0], Effect::RollModifier(_)));
    }

    #[test]
    fn a_roll_modifier_outside_a_trait_is_refused_at_load() {
        let refused = [
            kobold_with(|v| v["actions"][0]["effects"] = serde_json::json!([sunlight()])),
            kobold_with(|v| v["actions"][0]["effects"][0]["attack"]["on_hit"] = serde_json::json!([sunlight()])),
            kobold_with(|v| v["traits"][1]["effects"] = serde_json::json!([{"area": {"area": {"sphere": {"radius": 10}}, "origin": "target_point", "effect": sunlight(), "target_count": null}}])),
            kobold_with(|v| v["bonus_actions"] = serde_json::json!([{"id": "glare", "name": "Glare", "effects": [sunlight()]}])),
            kobold_with(|v| v["legendary_actions"] = serde_json::json!([{"id": "glare", "name": "Glare", "effects": [sunlight()]}])),
        ];
        for result in refused {
            let err = result.expect_err("refused");
            assert!(err.contains("a roll modifier is a trait effect"), "{err}");
        }
    }

    #[test]
    fn a_roll_modifier_keyed_to_unspecified_is_refused_at_load() {
        for key in ["unspecified", "normal"] {
            let err = kobold_with(|v| {
                let mut m = sunlight();
                m["roll_modifier"]["lighting"] = key.into();
                v["traits"][1]["effects"] = serde_json::json!([m]);
            })
            .expect_err("refused");
            assert!(err.contains("can't be keyed to unspecified lighting"), "{err}");
        }
    }

    fn parry() -> serde_json::Value {
        serde_json::json!({
            "id": "parry", "name": "Parry", "kind": "modeled",
            "trigger": {"hit_by_attack_roll": {"attack": "melee"}},
            "effects": [{"timed_modifier": {"target": "armor_class", "change": {"add": 2}, "duration": "triggering_attack"}}]
        })
    }

    #[test]
    fn a_hit_trigger_on_a_reaction_loads_and_round_trips() {
        let block = kobold_with(|v| v["reactions"] = serde_json::json!([parry()])).unwrap();
        let trigger = block.reactions[0].trigger.expect("trigger");
        assert_eq!(trigger, ReactionTrigger::HitByAttackRoll { attack: crate::effect::AttackFilter::Melee });
        let value = serde_json::to_value(&block.reactions[0]).unwrap();
        assert_eq!(value["trigger"], parry()["trigger"]);
        assert_eq!(value["effects"][0]["timed_modifier"]["duration"], "triggering_attack");
        assert!(serde_json::to_value(&block.actions[0]).unwrap().get("trigger").is_none());
    }

    #[test]
    fn a_trigger_outside_reactions_is_refused_at_load() {
        let refused = [
            kobold_with(|v| v["actions"] = serde_json::json!([parry()])),
            kobold_with(|v| v["bonus_actions"] = serde_json::json!([parry()])),
            kobold_with(|v| v["legendary_actions"] = serde_json::json!([parry()])),
            kobold_with(|v| v["traits"] = serde_json::json!([parry()])),
        ];
        for result in refused {
            let err = result.expect_err("refused");
            assert!(err.contains("a trigger belongs on a reaction"), "{err}");
        }
    }

    #[test]
    fn a_triggering_attack_duration_without_a_hit_trigger_is_refused_at_load() {
        let untriggered = || {
            let mut p = parry();
            p.as_object_mut().unwrap().remove("trigger");
            p
        };
        let nested = serde_json::json!({"id": "slam", "name": "Slam", "effects": [{"attack": {
            "modifier": 4, "reach": 5, "range": null, "target_count": 1, "damage": [],
            "on_hit": [{"condition": {"condition": "prone", "duration": "triggering_attack"}}]}}]});
        let refused = [
            kobold_with(|v| v["reactions"] = serde_json::json!([untriggered()])),
            kobold_with(|v| v["actions"] = serde_json::json!([untriggered()])),
            kobold_with(|v| v["actions"] = serde_json::json!([nested])),
        ];
        for result in refused {
            let err = result.expect_err("refused");
            assert!(err.contains("a triggering_attack duration needs a hit trigger"), "{err}");
        }
    }

    #[test]
    fn every_fixture_and_party_stat_block_loads() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data");
        let mut dirs = vec![root.join("fixtures"), root.join("party/standard")];
        let mut loaded = 0;
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    dirs.push(path);
                } else if path.extension().is_some_and(|e| e == "json") {
                    let text = std::fs::read_to_string(&path).unwrap();
                    serde_json::from_str::<StatBlock>(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                    loaded += 1;
                }
            }
        }
        assert!(loaded > 341, "{loaded}");
    }
}
