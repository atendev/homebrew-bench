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

/// What one cast costs in time; `Long` is a minute or more.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CastingTime {
    Action,
    BonusAction,
    Reaction,
    Long,
}

impl CastingTime {
    pub fn as_str(self) -> &'static str {
        match self {
            CastingTime::Action => "action",
            CastingTime::BonusAction => "bonus_action",
            CastingTime::Reaction => "reaction",
            CastingTime::Long => "long",
        }
    }

    /// The stat block section a cast of this length belongs in; `None` for `long`.
    pub fn section(self) -> Option<&'static str> {
        match self {
            CastingTime::Action => Some("actions"),
            CastingTime::BonusAction => Some("bonus_actions"),
            CastingTime::Reaction => Some("reactions"),
            CastingTime::Long => None,
        }
    }
}

/// The `source` value of an entry the user pasted.
pub const HOMEBREW_SOURCE: &str = "homebrew";

/// One entry in `data/spells/effects.json`, or one homebrew spell file.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpellEffect {
    pub id: String,
    pub name: String,
    /// `homebrew` for a pasted spell, else the SRD edition.
    pub source: String,
    /// The user's text, verbatim. Required when `source` is `homebrew`, refused otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pasted_source: Option<String>,
    /// The spell's own level; 0 is a cantrip.
    pub level: u8,
    pub casting_time: CastingTime,
    pub concentration: bool,
    #[serde(deserialize_with = "checked_effects")]
    pub effects: Vec<Effect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scaling: Vec<ScalingRule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub delayed: Vec<DelayedEffect>,
    /// Sentences the engine does not model, which the agent resolves.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agent_clauses: Vec<String>,
    /// What a higher slot does that the engine does not apply, for the agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upcast_clause: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl SpellEffect {
    /// Parses one entry and applies the checks serde can't: `pasted_source` goes with `homebrew` only.
    pub fn parse(value: serde_json::Value) -> Result<Self, String> {
        let spell: Self = serde_json::from_value(value).map_err(|e| e.to_string())?;
        spell.check_source()?;
        Ok(spell)
    }

    fn check_source(&self) -> Result<(), String> {
        match (self.source == HOMEBREW_SOURCE, &self.pasted_source) {
            (true, None) => Err(format!("spell {} is homebrew and needs pasted_source", self.id)),
            (false, Some(_)) => Err(format!("spell {} has pasted_source but its source is not homebrew", self.id)),
            _ => Ok(()),
        }
    }

    pub fn is_homebrew(&self) -> bool {
        self.source == HOMEBREW_SOURCE
    }

    /// Hash of the entry as stored, so two versions of one homebrew file can be told apart.
    pub fn fingerprint(&self) -> String {
        crate::encounter::tamper_hash(&serde_json::to_value(self).unwrap_or_default())
    }

    /// Whether the spell makes an attack roll or forces a save, so the caster needs a save DC and attack bonus.
    pub fn rolls_against_targets(&self) -> bool {
        self.effects.iter().chain(self.delayed.iter().map(|d| &d.effect)).any(Effect::rolls_against_target)
    }
}

fn checked_effects<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Vec<Effect>, D::Error> {
    let effects = Vec::<Effect>::deserialize(deserializer)?;
    effects.iter().try_for_each(Effect::check_dice).map_err(serde::de::Error::custom)?;
    Ok(effects)
}

/// The embedded spell effects file, parsed once and keyed by spell id.
fn catalog() -> Result<&'static HashMap<String, SpellEffect>, &'static str> {
    static CACHE: OnceLock<Result<HashMap<String, SpellEffect>, String>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let value: serde_json::Value = serde_json::from_str(crate::embedded::spell_effects_json()).map_err(|e| format!("invalid spell effects json: {e}"))?;
            let mut map = HashMap::new();
            if let Some(obj) = value.as_object() {
                for (key, entry_value) in obj {
                    let spell = SpellEffect::parse(entry_value.clone()).map_err(|e| format!("spell {key} invalid: {e}"))?;
                    map.insert(key.clone(), spell);
                }
            }
            Ok(map)
        })
        .as_ref()
        .map_err(String::as_str)
}

/// Every spell lookup goes through a book: an encounter's homebrew entries first, then the SRD effects file.
#[derive(Clone, Copy)]
pub struct SpellBook<'a> {
    srd: &'static HashMap<String, SpellEffect>,
    homebrew: &'a [SpellEffect],
}

impl<'a> SpellBook<'a> {
    /// A book over the SRD entries and `homebrew`. Fails when the embedded effects file is unusable.
    pub fn new(homebrew: &'a [SpellEffect]) -> Result<Self, &'static str> {
        Ok(Self { srd: catalog()?, homebrew })
    }

    /// A book of `homebrew` alone, for the unreachable case where the embedded SRD file is unusable.
    pub fn homebrew_only(homebrew: &'a [SpellEffect]) -> Self {
        static NONE: OnceLock<HashMap<String, SpellEffect>> = OnceLock::new();
        Self { srd: NONE.get_or_init(HashMap::new), homebrew }
    }

    /// A book with no homebrew entries.
    pub fn srd() -> Result<SpellBook<'static>, &'static str> {
        SpellBook::new(&[])
    }

    pub fn get(&self, id: &str) -> Option<&'a SpellEffect> {
        self.homebrew.iter().find(|s| s.id == id).or_else(|| self.srd.get(id))
    }

    pub fn contains(&self, id: &str) -> bool {
        self.get(id).is_some()
    }

    /// Every SRD entry in the effects file, in id order.
    pub fn srd_entries(&self) -> Vec<&'static SpellEffect> {
        let mut entries: Vec<&SpellEffect> = self.srd.values().collect();
        entries.sort_by(|a, b| a.id.cmp(&b.id));
        entries
    }
}

/// Serde check for `EncounterState.homebrew_spells`, so a save can't shadow an SRD spell, repeat an id
/// or carry an entry that is not homebrew.
pub(crate) fn checked_homebrew<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Vec<SpellEffect>, D::Error> {
    let spells = Vec::<SpellEffect>::deserialize(deserializer)?;
    check_homebrew_list(&spells).map_err(serde::de::Error::custom)?;
    Ok(spells)
}

/// The loader's rules for a list already parsed: every entry homebrew, no SRD id, no id twice.
pub fn check_homebrew_list(spells: &[SpellEffect]) -> Result<(), String> {
    let srd = catalog()?;
    for (i, spell) in spells.iter().enumerate() {
        spell.check_source()?;
        if !spell.is_homebrew() {
            return Err(format!("spell {} has source {}, and a homebrew spell must be homebrew", spell.id, spell.source));
        }
        if srd.contains_key(&spell.id) {
            return Err(format!("homebrew spell id {} is an SRD spell id", spell.id));
        }
        if spells[..i].iter().any(|s| s.id == spell.id) {
            return Err(format!("homebrew spell id {} appears twice", spell.id));
        }
    }
    Ok(())
}

/// Reads homebrew spell files for `--spell`. A file that is not a homebrew entry, an id equal to an
/// SRD id, and two files with one id are each an error naming the file and the id.
pub fn load_homebrew<P: AsRef<std::path::Path>>(paths: &[P]) -> Result<Vec<SpellEffect>, String> {
    let srd = catalog().map_err(str::to_string)?;
    let mut loaded: Vec<(String, SpellEffect)> = Vec::new();
    for path in paths {
        let name = path.as_ref().display().to_string();
        let text = std::fs::read_to_string(path).map_err(|e| format!("{name}: failed to read: {e}"))?;
        let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{name}: invalid json: {e}"))?;
        let spell = SpellEffect::parse(value).map_err(|e| format!("{name}: {e}"))?;
        if spell.source != HOMEBREW_SOURCE {
            return Err(format!("{name}: spell {} has source {}, and a spell file must be homebrew", spell.id, spell.source));
        }
        if srd.contains_key(&spell.id) {
            return Err(format!("{name}: homebrew id {} is an SRD spell id, so pick a different id", spell.id));
        }
        if let Some((first, _)) = loaded.iter().find(|(_, s)| s.id == spell.id) {
            return Err(format!("{name}: homebrew id {} is already loaded from {first}", spell.id));
        }
        loaded.push((name, spell));
    }
    Ok(loaded.into_iter().map(|(_, s)| s).collect())
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
            let result = SpellEffect::parse(entry_value.clone());
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

    fn srd_entry() -> serde_json::Value {
        effects_file()["shield"].clone()
    }

    fn homebrew_entry() -> serde_json::Value {
        let mut entry = srd_entry();
        entry["id"] = "ward-of-embers".into();
        entry["source"] = "homebrew".into();
        entry["pasted_source"] = "Ward of Embers. 1st-level abjuration...".into();
        entry
    }

    #[test]
    fn every_srd_entry_has_a_casting_time() {
        for (key, value) in effects_file().as_object().expect("object") {
            assert!(value.get("casting_time").is_some(), "{key} has no casting_time");
        }
    }

    #[test]
    fn a_homebrew_entry_without_a_casting_time_fails_to_parse() {
        let mut entry = homebrew_entry();
        entry.as_object_mut().unwrap().remove("casting_time");
        let err = SpellEffect::parse(entry).unwrap_err();
        assert!(err.contains("casting_time"), "{err}");
    }

    #[test]
    fn pasted_source_goes_with_homebrew_only() {
        assert!(SpellEffect::parse(homebrew_entry()).is_ok());
        let mut missing = homebrew_entry();
        missing.as_object_mut().unwrap().remove("pasted_source");
        assert!(SpellEffect::parse(missing).unwrap_err().contains("needs pasted_source"));
        let mut stray = srd_entry();
        stray["pasted_source"] = "text".into();
        assert!(SpellEffect::parse(stray).unwrap_err().contains("not homebrew"));
    }

    #[test]
    fn an_undeclared_key_fails_to_parse() {
        let mut entry = srd_entry();
        entry["agent_clause"] = serde_json::json!(["typo"]);
        assert!(SpellEffect::parse(entry).unwrap_err().contains("agent_clause"));
    }

    #[test]
    fn agent_clauses_survive_a_round_trip() {
        let mut entry = homebrew_entry();
        entry["agent_clauses"] = serde_json::json!(["The target glows for a minute."]);
        let spell = SpellEffect::parse(entry).expect("parses");
        assert_eq!(spell.agent_clauses, vec!["The target glows for a minute."]);
        let again = SpellEffect::parse(serde_json::to_value(&spell).unwrap()).expect("round trip");
        assert_eq!(again.agent_clauses, spell.agent_clauses);
        assert_eq!(again.pasted_source, spell.pasted_source);
        assert_eq!(again.casting_time, spell.casting_time);
    }

    #[test]
    fn the_schema_names_every_field() {
        let schema = serde_json::to_value(crate::schema::schema_for::<SpellEffect>()).unwrap();
        let props = schema["properties"].as_object().expect("properties");
        for field in [
            "id", "name", "source", "pasted_source", "level", "casting_time", "concentration",
            "effects", "scaling", "delayed", "agent_clauses", "upcast_clause", "notes",
        ] {
            assert!(props.contains_key(field), "schema lacks {field}");
        }
        assert_eq!(schema["additionalProperties"], serde_json::json!(false));
    }

    fn spell_file(dir: &str, name: &str, entry: &serde_json::Value) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bench-spell-book-{}-{dir}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, serde_json::to_string(entry).unwrap()).unwrap();
        path
    }

    #[test]
    fn a_book_resolves_homebrew_first_and_falls_back_to_the_srd() {
        let ward = SpellEffect::parse(homebrew_entry()).unwrap();
        let book = SpellBook::new(std::slice::from_ref(&ward)).unwrap();
        assert_eq!(book.get("ward-of-embers").map(|s| s.source.as_str()), Some("homebrew"));
        assert!(book.get("shield").is_some_and(|s| s.source != "homebrew"));
        assert!(book.get("no-such-spell").is_none());
        assert!(SpellBook::srd().unwrap().get("ward-of-embers").is_none());
    }

    #[test]
    fn a_homebrew_entry_wins_over_an_srd_entry_of_the_same_id_in_a_book() {
        let mut entry = homebrew_entry();
        entry["id"] = "shield".into();
        let shadow = SpellEffect::parse(entry).unwrap();
        let book = SpellBook::new(std::slice::from_ref(&shadow)).unwrap();
        assert_eq!(book.get("shield").map(|s| s.source.as_str()), Some("homebrew"));
    }

    #[test]
    fn a_saved_homebrew_list_is_held_to_the_loader_rules() {
        #[derive(Debug, Deserialize)]
        struct Saved {
            #[serde(deserialize_with = "checked_homebrew")]
            spells: Vec<SpellEffect>,
        }
        let load = |entries: Vec<serde_json::Value>| serde_json::from_value::<Saved>(serde_json::json!({ "spells": entries }));
        assert_eq!(load(vec![homebrew_entry()]).unwrap().spells.len(), 1);
        let mut clash = homebrew_entry();
        clash["id"] = "shield".into();
        assert!(load(vec![clash]).unwrap_err().to_string().contains("SRD"));
        assert!(load(vec![homebrew_entry(), homebrew_entry()]).unwrap_err().to_string().contains("twice"));
        let mut srd = srd_entry();
        srd["id"] = "ward-of-embers".into();
        assert!(load(vec![srd]).unwrap_err().to_string().contains("homebrew"));
    }

    #[test]
    fn a_loaded_file_joins_the_book() {
        let path = spell_file("ok", "ward.json", &homebrew_entry());
        let loaded = load_homebrew(&[&path]).unwrap();
        assert_eq!(loaded.len(), 1);
        assert!(SpellBook::new(&loaded).unwrap().contains("ward-of-embers"));
    }

    #[test]
    fn a_homebrew_id_equal_to_an_srd_id_is_refused_naming_file_and_id() {
        let mut entry = homebrew_entry();
        entry["id"] = "shield".into();
        let path = spell_file("clash", "shield.json", &entry);
        let err = load_homebrew(&[&path]).unwrap_err();
        assert!(err.contains("shield.json") && err.contains("shield") && err.contains("SRD"), "{err}");
    }

    #[test]
    fn two_files_with_one_id_are_an_error_naming_both() {
        let one = spell_file("dup", "one.json", &homebrew_entry());
        let two = spell_file("dup", "two.json", &homebrew_entry());
        let err = load_homebrew(&[&one, &two]).unwrap_err();
        assert!(err.contains("two.json") && err.contains("one.json") && err.contains("ward-of-embers"), "{err}");
    }

    #[test]
    fn a_spell_file_that_is_not_homebrew_is_refused() {
        let mut entry = srd_entry();
        entry["id"] = "ward-of-embers".into();
        let path = spell_file("srd", "ward.json", &entry);
        let err = load_homebrew(&[&path]).unwrap_err();
        assert!(err.contains("ward.json") && err.contains("homebrew"), "{err}");
    }
}
