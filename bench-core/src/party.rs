//! Standard party naming and generated-data checks.
//!
//! The four archetype progression tables in `data/party/progression/` feed
//! `scripts/generate-party.py`, which writes the 80 stat blocks in
//! `data/party/standard/`. This module owns the naming contract; the tests
//! hold the generated data to the stat block schema.

/// The four standard party archetypes, in report order.
pub const ARCHETYPES: [&str; 4] = ["defender", "striker", "healer", "caster"];

/// Lowest and highest standard party level.
pub const LEVELS: std::ops::RangeInclusive<u8> = 1..=20;

/// Stat block id, and file stem, for one archetype at one level.
pub fn block_id(archetype: &str, level: u8) -> String {
    format!("standard-{archetype}-{level:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::validate_value;
    use crate::spell::SpellEffect;
    use crate::stat_block::{AbilityEntry, AbilityKind, StatBlock};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::process::Command;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .to_path_buf()
    }

    fn load_blocks() -> Vec<(String, u8, StatBlock)> {
        let dir = root().join("data/party/standard");
        let mut blocks = Vec::new();
        for archetype in ARCHETYPES {
            for level in LEVELS {
                let id = block_id(archetype, level);
                let path = dir.join(format!("{id}.json"));
                let text = std::fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
                let value: serde_json::Value = serde_json::from_str(&text)
                    .unwrap_or_else(|e| panic!("{id}: invalid json: {e}"));
                let errors = validate_value::<StatBlock>(&value).expect("validation ran");
                assert!(
                    errors.is_empty(),
                    "{id} failed stat block validation: {errors:?}"
                );
                let block: StatBlock = serde_json::from_value(value).expect("parses");
                blocks.push((archetype.to_string(), level, block));
            }
        }
        blocks
    }

    fn all_abilities(block: &StatBlock) -> impl Iterator<Item = &AbilityEntry> {
        block
            .traits
            .iter()
            .chain(&block.actions)
            .chain(&block.bonus_actions)
            .chain(&block.reactions)
    }

    fn load_effects() -> HashMap<String, SpellEffect> {
        let text = std::fs::read_to_string(root().join("data/spells/effects.json")).unwrap();
        serde_json::from_str(&text).expect("effects file parses")
    }

    #[test]
    fn all_80_blocks_parse_and_validate() {
        let blocks = load_blocks();
        assert_eq!(blocks.len(), 80);
        let dir = root().join("data/party/standard");
        let on_disk = std::fs::read_dir(dir).unwrap().count();
        assert_eq!(on_disk, 80, "unexpected extra files in data/party/standard");
        for (archetype, level, block) in &blocks {
            assert_eq!(block.id, block_id(archetype, *level));
            assert_eq!(block.level, Some(*level));
            assert!(block.class.is_some(), "{} needs a class label", block.id);
        }
    }

    #[test]
    fn blocks_have_no_unmodeled_abilities() {
        for (_, _, block) in load_blocks() {
            for ability in all_abilities(&block) {
                assert_ne!(
                    ability.kind,
                    AbilityKind::Unmodeled,
                    "{}: ability {} is unmodeled",
                    block.id,
                    ability.id
                );
            }
        }
    }

    #[test]
    fn every_cast_resolves_to_the_effects_file() {
        let effects = load_effects();
        let mut casts = 0;
        for (_, _, block) in load_blocks() {
            for ability in all_abilities(&block) {
                for effect in &ability.effects {
                    if let crate::effect::Effect::Cast(cast) = effect {
                        casts += 1;
                        assert!(
                            effects.contains_key(&cast.spell_id),
                            "{}: unresolved spell id {}",
                            block.id,
                            cast.spell_id
                        );
                    }
                }
            }
        }
        assert!(casts > 0, "expected casters to cast something");
    }

    #[test]
    fn party_invariants_hold() {
        let mut previous: HashMap<String, (i32, i32)> = HashMap::new();
        for (archetype, level, block) in load_blocks() {
            assert!(block.dying, "{} must follow death-save rules", block.id);
            assert!(block.challenge_rating.is_none());
            assert_eq!(block.proficiency_bonus, 2 + (i32::from(level) - 1) / 4);
            let casts = matches!(archetype.as_str(), "healer" | "caster");
            assert_eq!(block.spellcasting.is_some(), casts, "{}", block.id);
            match &block.spell_slots {
                Some(slots) => {
                    assert!(casts, "{} should not have spell slots", block.id);
                    assert_eq!(slots.len(), 9, "{}", block.id);
                }
                None => assert!(!casts, "{} needs spell slots", block.id),
            }
            assert!(!block.gear.is_empty(), "{} gear is empty", block.id);
            assert!(
                block.gear.iter().all(|g| !g.contains('+') && !g.contains("magic")),
                "{} lists magic gear",
                block.id
            );
            assert!(!block.notes.is_empty(), "{} needs an archetype note", block.id);
            let hp = block.hit_points.average;
            let ac = block.armor_class[0].value;
            if let Some((prev_hp, prev_ac)) = previous.get(&archetype) {
                assert!(hp > *prev_hp, "{} hp must grow with level", block.id);
                assert!(ac >= *prev_ac, "{} ac must not shrink", block.id);
            }
            previous.insert(archetype, (hp, ac));
        }
    }

    #[test]
    fn committed_blocks_match_generator_output() {
        let output = Command::new("python3")
            .current_dir(root())
            .args(["-I", "scripts/generate-party.py", "--check"])
            .output()
            .expect("python3 is required to run the party generator check");
        assert!(
            output.status.success(),
            "generator output differs from committed blocks; rerun `python3 -I scripts/generate-party.py`:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn find<'a>(blocks: &'a [(String, u8, StatBlock)], archetype: &str, level: u8) -> &'a StatBlock {
        &blocks
            .iter()
            .find(|(a, l, _)| a == archetype && *l == level)
            .expect("block exists")
            .2
    }

    fn cast_ids(block: &StatBlock) -> Vec<String> {
        all_abilities(block)
            .flat_map(|a| a.effects.iter())
            .filter_map(|e| match e {
                crate::effect::Effect::Cast(c) => Some(c.spell_id.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn numbers_match_hand_computed_oracles() {
        let blocks = load_blocks();
        // Defender 5: STR 18, CON 14, d10, PB 3, splint + shield, two longsword attacks.
        let d = find(&blocks, "defender", 5);
        assert_eq!(d.hit_points.average, 10 + 4 * 6 + 5 * 2);
        assert_eq!(d.hit_points.dice, "5d10+10");
        assert_eq!(d.armor_class[0].value, 19);
        assert_eq!(d.saving_throws.len(), 2);
        // Caster 5: INT 18 (+4), PB 3 -> DC 15, attack +7; slots [4,3,2].
        let c = find(&blocks, "caster", 5);
        let sc = c.spellcasting.as_ref().unwrap();
        assert_eq!((sc.save_dc, sc.attack_bonus), (15, 7));
        assert_eq!(c.spell_slots.as_deref(), Some(&[4, 3, 2, 0, 0, 0, 0, 0, 0][..]));
        // Healer 20: WIS 20 (+5), PB 6 -> DC 19.
        let h = find(&blocks, "healer", 20);
        assert_eq!(h.spellcasting.as_ref().unwrap().save_dc, 19);
        assert_eq!(
            h.spell_slots.as_deref(),
            Some(&[4, 3, 3, 3, 3, 2, 2, 1, 1][..])
        );
    }

    #[test]
    fn spell_lists_follow_slot_tiers() {
        let blocks = load_blocks();
        let caster1 = cast_ids(find(&blocks, "caster", 1));
        assert!(caster1.contains(&"fire-bolt".to_string()), "at-will option");
        assert!(caster1.contains(&"magic-missile".to_string()));
        assert!(!caster1.contains(&"fireball".to_string()), "tier 3 spell at level 1");
        let caster20 = cast_ids(find(&blocks, "caster", 20));
        assert!(caster20.contains(&"meteor-swarm".to_string()));
        let healer1 = cast_ids(find(&blocks, "healer", 1));
        assert!(healer1.contains(&"spare-the-dying".to_string()), "at-will option");
        let healing = ["cure-wounds", "healing-word", "goodberry"];
        assert!(healing.iter().filter(|id| healer1.contains(&id.to_string())).count() >= 2);
        // Bonus-action and reaction placement.
        assert!(find(&blocks, "healer", 1).bonus_actions.iter().any(|a| a.id == "healing-word"));
        assert!(find(&blocks, "caster", 1).reactions.iter().any(|a| a.id == "shield"));
        // No cast uses a slot above the block's highest slot tier.
        for (_, _, block) in &blocks {
            let top = block
                .spell_slots
                .as_ref()
                .map_or(0, |s| s.iter().rposition(|n| *n > 0).map_or(0, |i| i + 1));
            for ability in all_abilities(block) {
                for effect in &ability.effects {
                    if let crate::effect::Effect::Cast(c) = effect {
                        assert!(
                            usize::from(c.slot_level.unwrap_or(0)) <= top,
                            "{}: {} above highest slot tier {top}",
                            block.id,
                            c.spell_id
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn only_out_of_combat_spells_are_left_off() {
        // Casting times of 1 minute or longer cannot be used in a fight.
        let expected_skipped = [
            "prayer-of-healing",
            "raise-dead",
            "heroes-feast",
            "regenerate",
            "resurrection",
            "true-resurrection",
        ];
        let standard: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(root().join("data/party/standard-spells.json")).unwrap(),
        )
        .unwrap();
        let blocks = load_blocks();
        for (role, archetype) in [("caster", "caster"), ("healer", "healer")] {
            let level20 = cast_ids(find(&blocks, archetype, 20));
            let mut listed: Vec<String> = standard[role]["at_will"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s["id"].as_str().unwrap().to_string())
                .collect();
            for tier in standard[role]["by_tier"].as_object().unwrap().values() {
                for spell in tier.as_array().unwrap() {
                    listed.push(spell[0].as_str().unwrap().to_string());
                }
            }
            for id in listed {
                let missing = !level20.contains(&id);
                assert_eq!(
                    missing,
                    expected_skipped.contains(&id.as_str()) || id == "foresight",
                    "{role} spell {id}: unexpected presence at level 20"
                );
            }
        }
    }
}
