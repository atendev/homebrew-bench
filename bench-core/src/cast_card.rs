//! The read-only cast card `bench spell` prints (ADR 0022), and the target count it shares with
//! `bench action`.

use serde::Serialize;

use crate::effect::{Duration, Effect};
use crate::encounter::{Combatant, EncounterState};
use crate::engine;
use crate::spell::{CastingTime, SpellEffect};
use crate::stat_block::{AbilityEntry, StatBlock};

/// How many target ids a cast takes on `bench action`; `max` is `None` when any number past `min` is taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TargetCount {
    pub min: u32,
    pub max: Option<u32>,
}

impl TargetCount {
    /// Read from the effects a cast resolves. An attack or an area with a `target_count` resolves
    /// against the first that many ids, any other effect on targets against every id, temporary HP
    /// with no target lands on the caster, and an effect on the caster alone takes none.
    pub fn of(effects: &[Effect]) -> Self {
        let (mut needs, mut open, mut cap) = (false, false, 0);
        for effect in effects {
            match effect {
                Effect::Attack(a) => {
                    needs = true;
                    cap = cap.max(a.target_count.max(1));
                }
                Effect::Area(a) => {
                    needs = true;
                    match a.target_count {
                        Some(n) => cap = cap.max(n.max(1)),
                        None => open = true,
                    }
                }
                Effect::SavingThrow(_) | Effect::Damage(_) | Effect::Condition(_) | Effect::Heal(_) | Effect::ForcedMove(_) | Effect::CounterSpell(_) => {
                    needs = true;
                    open = true;
                }
                Effect::TimedModifier(m) if m.bearer.is_target() && !matches!(m.duration, Duration::TriggeringAttack) => {
                    needs = true;
                    open = true;
                }
                Effect::Teleport(t) if !t.subject.is_actor() => {
                    needs = true;
                    open = true;
                }
                Effect::TempHp(_) | Effect::MoveThenAttack(_) | Effect::Multiattack(_) | Effect::UseAction(_) | Effect::Cast(_) => open = true,
                _ => {}
            }
        }
        Self { min: u32::from(needs), max: (!open).then_some(cap) }
    }

    pub fn accepts(&self, given: usize) -> bool {
        given >= self.min as usize && self.max.map_or(true, |max| given <= max as usize)
    }
}

impl std::fmt::Display for TargetCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.min, self.max) {
            (_, Some(0)) => write!(f, "takes no target ids"),
            (_, Some(1)) => write!(f, "takes 1 target id"),
            (min, Some(max)) => write!(f, "takes {min} to {max} target ids"),
            (min, None) => write!(f, "takes {min} or more target ids"),
        }
    }
}

/// The refusal `bench action` gives when its target ids don't fit a spell the action casts. The
/// count comes from [`TargetCount::of`] over [`engine::cast_effects`], as on the cast card. Anything
/// else wrong with the action is left for the engine to report.
pub fn check_action_targets(state: &EncounterState, actor_id: &str, action_id: &str, slot: Option<u8>, given: usize) -> Result<(), String> {
    let Some(actor) = state.combatants.iter().find(|c| c.id == actor_id) else { return Ok(()) };
    let Some(block) = block_of(state, actor) else { return Ok(()) };
    if slot.is_some() && block.class.is_none() {
        return Ok(());
    }
    let Some(action) = sections(block).find(|a| a.id == action_id) else { return Ok(()) };
    let Ok(book) = state.spell_book() else { return Ok(()) };
    for effect in &action.effects {
        let Effect::Cast(cast) = effect else { continue };
        let Some(spell) = book.get(&cast.spell_id) else { continue };
        let mut cast = cast.clone();
        if spell.level > 0 && slot.is_some() {
            cast.slot_level = slot;
        }
        let Ok(level) = engine::cast_level(spell, &cast) else { continue };
        let Ok(effects) = engine::cast_effects(spell, level, Some(block)) else { continue };
        let count = TargetCount::of(&effects);
        if !count.accepts(given) {
            return Err(format!("{action_id} casts {} at level {level}, which {count}; {given} given", spell.id));
        }
    }
    Ok(())
}

/// A live caster the card reads numbers from.
#[derive(Clone, Copy)]
pub struct CardCaster<'a> {
    pub combatant: &'a Combatant,
    pub block: &'a StatBlock,
}

impl<'a> CardCaster<'a> {
    /// The combatant `id` in `state` with its stat block.
    pub fn find(state: &'a EncounterState, id: &str) -> Option<Self> {
        let combatant = state.combatants.iter().find(|c| c.id == id)?;
        Some(Self { combatant, block: block_of(state, combatant)? })
    }
}

/// What a caster brings to the cast.
#[derive(Debug, Clone, Serialize)]
pub struct CasterNumbers {
    pub id: String,
    pub save_dc: Option<i32>,
    pub attack_bonus: Option<i32>,
    /// Spell slots left per level, for a caster with slots.
    pub slots: Vec<SlotsLeft>,
    /// Stat block actions that cast the spell, by id.
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SlotsLeft {
    pub level: u8,
    pub left: i32,
    pub of: u32,
}

/// The cast card: the entry as a cast at `cast_level` resolves it.
#[derive(Debug, Clone, Serialize)]
pub struct CastCard {
    pub spell_id: String,
    pub name: String,
    pub source: String,
    pub level: u8,
    pub cast_level: u8,
    pub casting_time: CastingTime,
    pub concentration: bool,
    /// `save`, `attack`, `save and attack` or `none`.
    pub rolls: &'static str,
    /// Effects at the cast level, with the caster's numbers bound in when a caster is given.
    pub effects: Vec<Effect>,
    pub targets: TargetCount,
    pub agent_clauses: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upcast_clause: Option<String>,
    /// Whether the cast level is above the spell's own, so the upcast clause applies.
    pub upcast: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caster: Option<CasterNumbers>,
    pub notes: Vec<String>,
    /// The card as text, for the agent to show the user.
    pub text: String,
}

/// The card for `spell` cast at `slot`, or at the level the caster's stat block casts it, else its own level.
pub fn card(spell: &SpellEffect, caster: Option<CardCaster>, slot: Option<u8>) -> Result<CastCard, String> {
    if let (Some(c), Some(_)) = (caster, slot) {
        if c.block.class.is_none() {
            return Err(format!("{} is a monster and casts at the slot level its stat block names, so --slot is for PCs only", c.combatant.id));
        }
    }
    let actions: Vec<&AbilityEntry> = caster.map(|c| sections(c.block).filter(|a| casts(a, &spell.id).is_some()).collect()).unwrap_or_default();
    let chosen = slot;
    let slot = slot.or_else(|| actions.iter().find_map(|a| casts(a, &spell.id)?.slot_level)).filter(|_| spell.level > 0);
    let cast = crate::effect::Cast { spell_id: spell.id.clone(), slot_level: slot, upcast: false };
    let cast_level = engine::cast_level(spell, &cast).map_err(|e| e.to_string())?;
    if chosen.is_some() {
        let refusals: Vec<String> = actions
            .iter()
            .filter_map(|a| engine::check_homebrew_upcast(&a.id, spell, casts(a, &spell.id)?, cast_level).err())
            .map(|e| e.to_string())
            .collect();
        if !actions.is_empty() && refusals.len() == actions.len() {
            return Err(refusals[0].clone());
        }
    }
    let effects = engine::cast_effects(spell, cast_level, caster.map(|c| c.block)).map_err(|e| e.to_string())?;
    let targets = TargetCount::of(&effects);
    let rolls = rolls(&effects);
    let caster = caster.map(|c| numbers(c, &actions));
    let mut notes = Vec::new();
    if caster.is_none() && rolls != "none" {
        notes.push("The save DC and attack bonus come from the caster's spellcasting block; pass --caster to see them.".to_string());
    }
    if caster.is_none() && spell.is_homebrew() && cast_level > spell.level {
        notes.push("`bench action` casts a homebrew spell above its own level only from a stat block cast with `upcast: true`.".to_string());
    }
    if caster.is_none() && spell.level == 0 && has_damage(&effects) {
        notes.push("A cantrip's damage dice multiply at caster levels 5, 11 and 17; pass --caster to see them.".to_string());
    }
    let mut card = CastCard {
        spell_id: spell.id.clone(),
        name: spell.name.clone(),
        source: spell.source.clone(),
        level: spell.level,
        cast_level,
        casting_time: spell.casting_time,
        concentration: spell.concentration,
        rolls,
        effects,
        targets,
        agent_clauses: spell.agent_clauses.clone(),
        upcast_clause: spell.upcast_clause.clone(),
        upcast: cast_level > spell.level,
        caster,
        notes,
        text: String::new(),
    };
    card.text = render(&card);
    Ok(card)
}

fn block_of<'a>(state: &'a EncounterState, c: &Combatant) -> Option<&'a StatBlock> {
    state.stat_blocks.iter().find(|s| s.id == c.stat_block_id).map(|s| &s.stat_block)
}

fn sections(block: &StatBlock) -> impl Iterator<Item = &AbilityEntry> {
    block.actions.iter().chain(&block.bonus_actions).chain(&block.reactions)
}

fn casts<'a>(action: &'a AbilityEntry, spell_id: &str) -> Option<&'a crate::effect::Cast> {
    action.effects.iter().find_map(|e| match e {
        Effect::Cast(c) if c.spell_id == spell_id => Some(c),
        _ => None,
    })
}

fn numbers(c: CardCaster, actions: &[&AbilityEntry]) -> CasterNumbers {
    let sc = c.block.spellcasting.as_ref();
    let slots = c.block.spell_slots.iter().flatten().zip(1u8..).filter(|(of, _)| **of > 0).map(|(of, level)| {
        let used = c.combatant.resources.get(&format!("{}:slot:{level}", c.combatant.id)).copied().unwrap_or(0);
        SlotsLeft { level, left: *of as i32 - used, of: *of }
    });
    CasterNumbers {
        id: c.combatant.id.clone(),
        save_dc: sc.map(|s| s.save_dc),
        attack_bonus: sc.map(|s| s.attack_bonus),
        slots: slots.collect(),
        actions: actions.iter().map(|a| a.id.clone()).collect(),
    }
}

fn rolls(effects: &[Effect]) -> &'static str {
    fn walk(effect: &Effect, save: &mut bool, attack: &mut bool) {
        match effect {
            Effect::Attack(_) => *attack = true,
            Effect::SavingThrow(_) => *save = true,
            Effect::Area(a) => walk(&a.effect, save, attack),
            _ => {}
        }
    }
    let (mut save, mut attack) = (false, false);
    effects.iter().for_each(|e| walk(e, &mut save, &mut attack));
    match (save, attack) {
        (true, true) => "save and attack",
        (true, false) => "save",
        (false, true) => "attack",
        (false, false) => "none",
    }
}

fn has_damage(effects: &[Effect]) -> bool {
    effects.iter().any(|e| match e {
        Effect::Attack(a) => !a.damage.is_empty(),
        Effect::SavingThrow(s) => !s.damage.is_empty(),
        Effect::Damage(_) => true,
        Effect::Area(a) => has_damage(std::slice::from_ref(&a.effect)),
        _ => false,
    })
}

fn render(card: &CastCard) -> String {
    let mut lines = vec![format!("{} ({}, {}), level {} cast at level {}", card.name, card.spell_id, card.source, card.level, card.cast_level)];
    lines.push(format!(
        "Casting time: {}. Concentration: {}. Rolls: {}.",
        card.casting_time.as_str(),
        if card.concentration { "yes" } else { "no" },
        card.rolls
    ));
    if let Some(c) = &card.caster {
        let mut line = format!("Caster {}:", c.id);
        match (c.save_dc, c.attack_bonus) {
            (Some(dc), Some(bonus)) => line.push_str(&format!(" save DC {dc}, attack {bonus:+}.")),
            _ => line.push_str(" no spellcasting block."),
        }
        if !c.slots.is_empty() {
            let slots: Vec<String> = c.slots.iter().map(|s| format!("level {} {}/{}", s.level, s.left, s.of)).collect();
            line.push_str(&format!(" Slots left: {}.", slots.join(", ")));
        }
        lines.push(line);
    }
    lines.push(format!("Effects at level {}:", card.cast_level));
    lines.extend(card.effects.iter().map(|e| format!("- {}", serde_json::to_string(e).unwrap_or_default())));
    let command = match &card.caster {
        Some(c) if !c.actions.is_empty() => format!("`bench action {} {}`", c.id, c.actions.join("|")),
        _ => "`bench action`".to_string(),
    };
    lines.push(format!("Targets: {command} {}.", card.targets));
    if !card.agent_clauses.is_empty() {
        lines.push("Agent clauses:".into());
        lines.extend(card.agent_clauses.iter().map(|c| format!("- {c}")));
    }
    if let Some(clause) = &card.upcast_clause {
        let when = if card.upcast { "applies at this level" } else { "applies above its own level" };
        lines.push(format!("Upcast clause ({when}): {clause}"));
    }
    lines.extend(card.notes.iter().map(|n| format!("Note: {n}")));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caster_block(rel: &str) -> StatBlock {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data").join(rel);
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
    }

    fn combatant(id: &str, block: &StatBlock) -> Combatant {
        serde_json::from_value(serde_json::json!({
            "id": id, "stat_block_id": block.id, "name": id, "side_id": "party", "hp": 10, "temp_hp": 0, "max_hp": 10,
            "conditions": [], "concentration": null, "resources": {}, "action_economy": {
                "action_used": false, "bonus_action_used": false, "reaction_used": false, "movement_used_feet": 0, "disengaged": false, "extra_action_used": false
            }, "engagement": [], "ranges": {}, "end_state": "active",
        }))
        .unwrap()
    }

    fn cinder_wave() -> SpellEffect {
        SpellEffect::parse(serde_json::json!({
            "id": "cinder-wave", "name": "Cinder Wave", "source": "homebrew", "pasted_source": "Cinder Wave, as pasted",
            "level": 2, "casting_time": "action", "concentration": false,
            "effects": [{"saving_throw": {"ability": "dex", "dc": 0, "half_on_success": true, "no_damage_on_success": false, "damage": [{"dice": "3d6", "type": "fire"}]}}],
            "scaling": [{"effect_index": 0, "per_slot": {"extra_dice": {"dice": "1d6"}}}],
            "agent_clauses": ["Unattended flammable objects in the line ignite."],
            "upcast_clause": "At level 4 or higher the line is 10 feet wide.",
        }))
        .unwrap()
    }

    fn save_dice(card: &CastCard) -> String {
        match &card.effects[0] {
            Effect::SavingThrow(s) => s.damage[0].dice.clone(),
            other => panic!("not a save: {other:?}"),
        }
    }

    #[test]
    fn a_homebrew_save_spell_at_its_level_prints_every_clause() {
        let card = card(&cinder_wave(), None, None).unwrap();
        assert_eq!((card.level, card.cast_level, card.rolls), (2, 2, "save"));
        assert_eq!(save_dice(&card), "3d6");
        assert!(!card.upcast);
        assert!(card.text.contains("Unattended flammable objects in the line ignite."), "{}", card.text);
        assert!(card.text.contains("At level 4 or higher the line is 10 feet wide."), "{}", card.text);
        assert!(card.text.contains("takes 1 or more target ids"), "{}", card.text);
    }

    #[test]
    fn an_upcast_scales_the_dice_as_the_engine_does() {
        let card = card(&cinder_wave(), None, Some(4)).unwrap();
        assert_eq!((card.cast_level, card.upcast), (4, true));
        assert_eq!(save_dice(&card), "5d6");
        assert!(card.text.contains("applies at this level"), "{}", card.text);
    }

    #[test]
    fn a_homebrew_upcast_follows_the_casts_upcast_flag_as_bench_action_does() {
        assert!(card(&cinder_wave(), None, Some(3)).unwrap().notes.iter().any(|n| n.contains("upcast: true")));
        let mut block = caster_block("party/standard/standard-caster-05.json");
        let entry = |upcast: bool| -> AbilityEntry {
            serde_json::from_value(serde_json::json!({
                "id": "cinder-wave", "name": "Cinder Wave", "desc": "Cast Cinder Wave.", "kind": "modeled", "usage": "at_will",
                "effects": [{"cast": {"slot_level": 2, "spell_id": "cinder-wave", "upcast": upcast}}]
            }))
            .unwrap()
        };
        block.actions.push(entry(false));
        let c = combatant("standard-caster-05", &block);
        let err = card(&cinder_wave(), Some(CardCaster { combatant: &c, block: &block }), Some(3)).unwrap_err();
        assert!(err.contains("has upcast false"), "{err}");
        block.actions.pop();
        block.actions.push(entry(true));
        let up = card(&cinder_wave(), Some(CardCaster { combatant: &c, block: &block }), Some(3)).unwrap();
        assert_eq!(save_dice(&up), "4d6");
        assert!(up.notes.is_empty(), "{:?}", up.notes);
    }

    #[test]
    fn a_slot_below_the_spell_level_is_refused() {
        assert!(card(&cinder_wave(), None, Some(1)).unwrap_err().contains("level 2 spell"));
    }

    #[test]
    fn a_cantrip_scales_at_the_casters_level() {
        let fire_bolt = crate::spell::SpellBook::srd().unwrap().get("fire-bolt").unwrap();
        let block = caster_block("party/standard/standard-caster-05.json");
        let c = combatant("standard-caster-05", &block);
        let card = card(fire_bolt, Some(CardCaster { combatant: &c, block: &block }), None).unwrap();
        let Effect::Attack(a) = &card.effects[0] else { panic!("fire bolt is an attack") };
        assert_eq!(a.damage[0].dice, "2d10");
        assert_eq!(a.modifier, 7);
        let bare = super::card(fire_bolt, None, None).unwrap();
        let Effect::Attack(a) = &bare.effects[0] else { panic!("fire bolt is an attack") };
        assert_eq!(a.damage[0].dice, "1d10");
        assert!(bare.notes.iter().any(|n| n.contains("caster levels 5, 11 and 17")));
    }

    #[test]
    fn caster_numbers_come_from_the_stat_block_and_spent_slots() {
        let block = caster_block("party/standard/standard-caster-05.json");
        let mut c = combatant("standard-caster-05", &block);
        c.resources.insert("standard-caster-05:slot:1".into(), 1);
        let magic_missile = crate::spell::SpellBook::srd().unwrap().get("magic-missile").unwrap();
        let card = card(magic_missile, Some(CardCaster { combatant: &c, block: &block }), None).unwrap();
        let numbers = card.caster.as_ref().unwrap();
        assert_eq!((numbers.save_dc, numbers.attack_bonus), (Some(15), Some(7)));
        assert_eq!(numbers.slots.iter().map(|s| (s.level, s.left, s.of)).collect::<Vec<_>>(), vec![(1, 3, 4), (2, 3, 3), (3, 2, 2)]);
        assert_eq!(numbers.actions, vec!["magic-missile"]);
        assert!(card.text.contains("Caster standard-caster-05: save DC 15, attack +7. Slots left: level 1 3/4"), "{}", card.text);
        assert!(card.text.contains("`bench action standard-caster-05 magic-missile` takes 1 or more target ids"), "{}", card.text);
    }

    #[test]
    fn a_monster_caster_takes_no_slot() {
        let block = caster_block("fixtures/goblin-warrior.json");
        let c = combatant("goblin-warrior", &block);
        let err = card(&cinder_wave(), Some(CardCaster { combatant: &c, block: &block }), Some(3)).unwrap_err();
        assert!(err.contains("--slot is for PCs only"), "{err}");
    }

    #[test]
    fn target_counts_read_the_effects() {
        let fx = |v: serde_json::Value| -> Vec<Effect> { serde_json::from_value(v).unwrap() };
        let single = fx(serde_json::json!([{"attack": {"modifier": 0, "target_count": 1, "damage": []}}]));
        assert_eq!(TargetCount::of(&single).to_string(), "takes 1 target id");
        let triple = fx(serde_json::json!([{"attack": {"modifier": 0, "target_count": 3, "damage": []}}]));
        assert_eq!(TargetCount::of(&triple).to_string(), "takes 1 to 3 target ids");
        assert!(!TargetCount::of(&triple).accepts(4) && !TargetCount::of(&triple).accepts(0));
        let none = fx(serde_json::json!([{"timed_modifier": {"bearer": "self", "change": {"add": 5}, "duration": {"rounds": 1}, "target": {"armor_class": null}}}]));
        let count = TargetCount::of(&none);
        assert_eq!(count.to_string(), "takes no target ids");
        assert!(count.accepts(0) && !count.accepts(1));
    }
}
