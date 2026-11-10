//! Engine-owned spell effects file model.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::effect::{Duration, Effect};
use crate::stat_block::DamageType;

/// Upcast or slot-level scaling rule attached to one effect.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ScalingRule {
    pub effect_index: usize,
    pub per_slot: Scaling,
}

/// What one slot level above the spell's own level adds to its effect.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Scaling {
    /// Adds dice of the same size to the damage entry of `damage_type` (else the
    /// first one), or to Heal or TempHp dice. Looks inside an Area.
    ExtraDice {
        dice: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        damage_type: Option<DamageType>,
    },
    /// Adds a flat amount to Heal, TempHp or the first damage entry.
    ExtraModifier { amount: i32 },
    /// Raises `target_count` on an Attack or Area; other effects already take every target passed.
    ExtraTargets { count: u32 },
    /// Appends copies of the effect, as for darts and rays.
    ExtraEffects { count: u32 },
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
    /// The spell's own level; 0 is a cantrip.
    pub level: u8,
    pub concentration: bool,
    #[serde(deserialize_with = "checked_effects")]
    pub effects: Vec<Effect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scaling: Vec<ScalingRule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub delayed: Vec<DelayedEffect>,
    /// What a higher slot does that the engine does not apply, for the agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upcast_clause: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

fn checked_effects<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Vec<Effect>, D::Error> {
    let effects = Vec::<Effect>::deserialize(deserializer)?;
    effects.iter().try_for_each(Effect::check_dice).map_err(serde::de::Error::custom)?;
    Ok(effects)
}

/// The embedded spell effects file, parsed once and keyed by spell id.
pub(crate) fn catalog() -> Result<&'static HashMap<String, SpellEffect>, &'static str> {
    static CACHE: OnceLock<Result<HashMap<String, SpellEffect>, String>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let value: serde_json::Value = serde_json::from_str(crate::embedded::spell_effects_json()).map_err(|e| format!("invalid spell effects json: {e}"))?;
            let mut map = HashMap::new();
            if let Some(obj) = value.as_object() {
                for (key, entry_value) in obj {
                    let spell: SpellEffect = serde_json::from_value(entry_value.clone()).map_err(|e| format!("spell {key} invalid: {e}"))?;
                    map.insert(key.clone(), spell);
                }
            }
            Ok(map)
        })
        .as_ref()
        .map_err(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn effects_file() -> serde_json::Value {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .to_path_buf();
        let path = root.join("data/spells/effects.json");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("effects.json is not valid json: {e}"))
    }

    fn entries() -> Vec<SpellEffect> {
        let value = effects_file();
        value
            .as_object()
            .expect("object")
            .values()
            .map(|v| serde_json::from_value(v.clone()).expect("valid entry"))
            .collect()
    }

    fn die_size(dice: &str) -> u32 {
        crate::effect::parse_dice(dice)
            .unwrap_or_else(|| panic!("{dice} does not parse"))
            .1
    }

    fn sizes(effect: &Effect) -> Vec<(Option<DamageType>, u32)> {
        match effect {
            Effect::Attack(a) => a
                .damage
                .iter()
                .map(|d| (Some(d.damage_type), die_size(&d.dice)))
                .collect(),
            Effect::SavingThrow(s) => s
                .damage
                .iter()
                .map(|d| (Some(d.damage_type), die_size(&d.dice)))
                .collect(),
            Effect::Damage(d) => vec![(Some(d.damage_type), die_size(&d.dice))],
            Effect::Heal(h) => h.dice.iter().map(|d| (None, die_size(d))).collect(),
            Effect::TempHp(t) => vec![(None, die_size(&t.dice))],
            Effect::Area(a) => sizes(&a.effect),
            _ => vec![],
        }
    }

    fn every_dice(value: &serde_json::Value, out: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(dice) = map.get("dice").and_then(serde_json::Value::as_str) {
                    out.push(dice.to_string());
                }
                map.values().for_each(|v| every_dice(v, out));
            }
            serde_json::Value::Array(items) => items.iter().for_each(|v| every_dice(v, out)),
            _ => {}
        }
    }

    #[test]
    fn every_dice_string_in_the_file_parses() {
        let mut dice = Vec::new();
        every_dice(&effects_file(), &mut dice);
        let bad: Vec<_> = dice
            .iter()
            .filter(|d| crate::effect::parse_dice(d).is_none())
            .collect();
        assert!(bad.is_empty(), "unparseable dice: {bad:?}");
    }

    #[test]
    fn every_scaling_rule_fits_the_effect_it_names() {
        let mut faults = Vec::new();
        for spell in entries() {
            for rule in &spell.scaling {
                let Some(effect) = spell.effects.get(rule.effect_index) else {
                    faults.push(format!(
                        "{}: effect_index {} is missing",
                        spell.id, rule.effect_index
                    ));
                    continue;
                };
                let inner = match effect {
                    Effect::Area(a) => a.effect.as_ref(),
                    other => other,
                };
                let fits = match &rule.per_slot {
                    Scaling::ExtraDice { dice, damage_type } => {
                        let size = die_size(dice);
                        let targets = sizes(effect);
                        let target = targets
                            .iter()
                            .find(|(t, _)| damage_type.is_some() && t == damage_type)
                            .or(targets.first());
                        target.is_some_and(|(_, s)| *s == size)
                    }
                    Scaling::ExtraModifier { .. } => matches!(
                        inner,
                        Effect::Heal(_)
                            | Effect::TempHp(_)
                            | Effect::Damage(_)
                            | Effect::Attack(_)
                            | Effect::SavingThrow(_)
                    ),
                    Scaling::ExtraTargets { .. } | Scaling::ExtraEffects { .. } => true,
                };
                if !fits {
                    faults.push(format!(
                        "{}: {:?} does not fit effect {}",
                        spell.id, rule.per_slot, rule.effect_index
                    ));
                }
            }
        }
        assert!(faults.is_empty(), "{}", faults.join("\n"));
    }

    #[test]
    fn effects_file_validates_against_schema() {
        let value = effects_file();
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
