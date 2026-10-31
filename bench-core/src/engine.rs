//! Deterministic encounter execution.
//!
//! The engine deliberately accepts structured requests rather than ability
//! prose.  This keeps the rules path shared by live and batch callers.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::effect::{
    recharge_minimum, Duration, Effect, ModifierChange, ModifierTarget, RollDisposition, Usage,
};
use crate::encounter::{
    ActionEconomy, ActiveModifier, Combatant, ConditionInstance, EndState, EncounterState,
    PendingPrompt, RangeBand,
};
use crate::rng::Stream;
use crate::stat_block::{Ability, Condition, DamageType, DeathSaves, StatBlock};

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StepRequest {
    Action { actor_id: String, action_id: String, target_ids: Vec<String> },
    Move { actor_id: String, target_id: Option<String>, band: RangeBand, feet: u32 },
    EndTurn,
    Respond { values: Vec<u32>, accept: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EngineEvent {
    pub kind: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roll: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub damage: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct StepResponse {
    pub events: Vec<EngineEvent>,
    pub digest: serde_json::Value,
    pub pending: Option<PendingPrompt>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct InitiativeOptions {
    #[serde(default)]
    pub individual: bool,
    #[serde(default)]
    pub surprised: Vec<String>,
    #[serde(default)]
    pub pc_dice: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    NotFound(String),
    Invalid(String),
    Pending(String),
    Illegal(String),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(s) | Self::Invalid(s) | Self::Pending(s) | Self::Illegal(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for EngineError {}

/// Roll initiative and initialize the first turn.
pub fn roll_initiative(
    state: &mut EncounterState,
    options: InitiativeOptions,
) -> Result<Vec<EngineEvent>, EngineError> {
    let mut stream = Stream::from_state(state.rng.clone());
    let mut rolls: HashMap<String, i32> = HashMap::new();
    let mut grouped_rolls: HashMap<String, u32> = HashMap::new();
    for c in &state.combatants {
        let block = stat_block(state, c)?;
        let d20 = if !options.individual {
            *grouped_rolls.entry(c.stat_block_id.clone()).or_insert_with(|| {
                let first = stream.roll_die(20);
                if options.surprised.iter().any(|id| id == &c.id) {
                    first.min(stream.roll_die(20))
                } else {
                    first
                }
            })
        } else {
            let first = stream.roll_die(20);
            if options.surprised.iter().any(|id| id == &c.id) {
                first.min(stream.roll_die(20))
            } else {
                first
            }
        };
        rolls.insert(c.id.clone(), d20 as i32 + block.initiative);
    }
    state.rng = stream.state();
    state.initiative_order = initiative_order(state, &rolls, !options.individual);
    state.active_turn = state.initiative_order.first().cloned();
    if state.round == 0 {
        state.round = 1;
    }
    let events = state
        .initiative_order
        .iter()
        .map(|id| EngineEvent {
            kind: "initiative".into(),
            message: format!("{id} rolled {}", rolls[id]),
            actor_id: Some(id.clone()),
            target_id: None,
            roll: None,
            total: rolls.get(id).copied(),
            damage: None,
        })
        .collect();
    Ok(events)
}

fn initiative_order(state: &EncounterState, rolls: &HashMap<String, i32>, grouped: bool) -> Vec<String> {
    let mut ids = state.combatants.iter().map(|c| c.id.clone()).collect::<Vec<_>>();
    ids.sort_by_key(|id| {
        let c = state.combatants.iter().find(|c| &c.id == id).expect("id from combatants");
        let group = if grouped { c.stat_block_id.as_str() } else { id.as_str() };
        (-(rolls[id]), group.to_string(), id.clone())
    });
    ids
}

/// Resolve one whole action or lifecycle command.
pub fn step(state: &mut EncounterState, request: StepRequest) -> Result<StepResponse, EngineError> {
    if state.pending.is_some() && !matches!(request, StepRequest::Respond { .. }) {
        return Err(EngineError::Pending("answer the pending prompt first".into()));
    }
    let events = match request {
        StepRequest::Action { actor_id, action_id, target_ids } => {
            resolve_action(state, &actor_id, &action_id, &target_ids)?
        }
        StepRequest::Move { actor_id, target_id, band, feet } => {
            move_combatant(state, &actor_id, target_id.as_deref(), band, feet)?
        }
        StepRequest::EndTurn => end_turn_events(state)?,
        StepRequest::Respond { values, accept } => respond(state, values, accept)?,
    };
    Ok(response(state, events))
}

fn resolve_action(
    state: &mut EncounterState,
    actor_id: &str,
    action_id: &str,
    target_ids: &[String],
) -> Result<Vec<EngineEvent>, EngineError> {
    let actor_index = index_of(state, actor_id)?;
    if !can_act(&state.combatants[actor_index]) {
        return Err(EngineError::Illegal(format!("{actor_id} cannot act")));
    }
    if let Some(active) = &state.active_turn {
        if active != actor_id {
            return Err(EngineError::Illegal(format!("{actor_id} is not the active combatant")));
        }
    }
    let action = {
        let block = stat_block(state, &state.combatants[actor_index])?;
        block.actions.iter().find(|a| a.id == action_id).cloned()
            .ok_or_else(|| EngineError::NotFound(format!("action {action_id} not found")))?
    };
    if state.combatants[actor_index].action_economy.action_used {
        return Err(EngineError::Illegal("action already used this turn".into()));
    }
    // Check and decrement usage limits
    if let Some(usage) = &action.usage {
        let key = match usage {
            Usage::Recharge { group, .. } => {
                format!("{actor_id}:{}", group.as_deref().unwrap_or(action_id))
            }
            _ => format!("{actor_id}:{action_id}"),
        };
        match usage {
            Usage::AtWill => {}
            Usage::Recharge { .. } => {
                let remaining = state.combatants[actor_index].resources.get(&key).copied().unwrap_or(1);
                if remaining <= 0 {
                    return Err(EngineError::Illegal(format!("{action_id} is not recharged")));
                }
                state.combatants[actor_index].resources.insert(key, remaining - 1);
            }
            Usage::PerDay { uses } => {
                let used = state.combatants[actor_index].resources.get(&key).copied().unwrap_or(0);
                if used >= *uses as i32 {
                    return Err(EngineError::Illegal(format!("{action_id} has no uses remaining")));
                }
                state.combatants[actor_index].resources.insert(key, used + 1);
            }
            Usage::PerRest { uses, .. } => {
                let used = state.combatants[actor_index].resources.get(&key).copied().unwrap_or(0);
                if used >= *uses as i32 {
                    return Err(EngineError::Illegal(format!("{action_id} has no uses remaining")));
                }
                state.combatants[actor_index].resources.insert(key, used + 1);
            }
            Usage::CooldownUntilNextTurn => {
                let remaining = state.combatants[actor_index].resources.get(&key).copied().unwrap_or(1);
                if remaining <= 0 {
                    return Err(EngineError::Illegal(format!("{action_id} is on cooldown")));
                }
                state.combatants[actor_index].resources.insert(key, 0);
            }
        }
    }
    let mut stream = Stream::from_state(state.rng.clone());
    let mut events = Vec::new();
    for effect in action.effects {
        resolve_effect(state, &mut stream, actor_id, effect, target_ids, &mut events)?;
        if state.pending.is_some() {
            break;
        }
    }
    state.rng = stream.state();
    state.combatants[actor_index].action_economy.action_used = true;
    Ok(events)
}

fn resolve_effect(
    state: &mut EncounterState,
    stream: &mut Stream,
    actor_id: &str,
    effect: Effect,
    target_ids: &[String],
    events: &mut Vec<EngineEvent>,
) -> Result<(), EngineError> {
    match effect {
        Effect::Attack(attack) => {
            for target_id in target_ids.iter().take(attack.target_count.max(1) as usize) {
                let target_index = index_of(state, target_id)?;
                let target = state.combatants[target_index].clone();
                let actor = state.combatants[index_of(state, actor_id)?].clone();
                let advantage = disposition(state, &actor, &target, RollDisposition::Normal);
                let d20a = stream.roll_die(20);
                let d20b = stream.roll_die(20);
                let natural = match advantage {
                    RollDisposition::Advantage => d20a.max(d20b),
                    RollDisposition::Disadvantage => d20a.min(d20b),
                    RollDisposition::Normal => d20a,
                };
                let total = natural as i32 + attack.modifier + attack_modifier(state, &actor)?;
                let ac = stat_block(state, &target)?.armor_class.first().map(|a| a.value).unwrap_or(10) + ac_modifier(state, &target);
                let critical = natural == 20;
                let hit = critical || (natural != 1 && total >= ac);
                events.push(EngineEvent {
                    kind: "attack".into(),
                    message: format!("{} attacks {}: {} vs AC {} ({})", actor.name, target.name, total, ac, if hit { "hit" } else { "miss" }),
                    actor_id: Some(actor_id.into()),
                    target_id: Some(target_id.clone()),
                    roll: Some(natural),
                    total: Some(total),
                    damage: None,
                });
                if hit {
                    let mut damage = 0;
                    for die in attack.damage.iter().filter(|d| damage_applies(&target, actor_id, d)) {
                        let mut amount = roll_dice(stream, &die.dice)? as i32 + die.modifier;
                        if critical {
                            amount += roll_dice(stream, &die.dice)? as i32;
                        }
                        damage += adjust_damage(state, &target, amount, die.damage_type);
                    }
                    apply_damage(state, &target.id, damage, Some(actor_id), events)?;
                    for effect in &attack.on_hit {
                        resolve_effect(state, stream, actor_id, effect.clone(), &[target_id.clone()], events)?;
                    }
                }
            }
        }
        Effect::Damage(damage) => {
            let amount = roll_dice(stream, &damage.dice)? as i32 + damage.modifier;
            for target_id in target_ids {
                let target = state.combatants[index_of(state, target_id)?].clone();
                if !damage_applies(&target, actor_id, &damage) {
                    continue;
                }
                let amount = adjust_damage(state, &target, amount, damage.damage_type);
                apply_damage(state, target_id, amount, Some(actor_id), events)?;
            }
        }
        Effect::SavingThrow(save) => {
            for target_id in target_ids {
                resolve_save(state, stream, actor_id, target_id, &save, events)?;
            }
        }
        Effect::Condition(condition) => {
            for target_id in target_ids {
                let index = index_of(state, target_id)?;
                let immune = stat_block(state, &state.combatants[index])?.condition_immunities.contains(&condition.condition);
                if !immune {
                    state.combatants[index].conditions.push(ConditionInstance {
                        condition: condition.condition,
                        duration: condition.duration.clone(),
                        source: Some(actor_id.into()),
                    });
                    events.push(event("condition", format!("{target_id} is {}", format!("{:?}", condition.condition).to_lowercase()), actor_id, target_id));
                }
            }
        }
        Effect::TimedModifier(modifier) => {
            events.push(event("modifier", format!("{} gains a timed modifier", actor_id), actor_id, actor_id));
            state.active_modifiers.push(ActiveModifier {
                target: modifier.target.clone(),
                change: modifier.change.clone(),
                remaining: modifier.duration.clone(),
                source: Some(actor_id.into()),
            });
        }
        Effect::ForcedMove(movement) => {
            for target_id in target_ids {
                apply_forced_move(state, actor_id, target_id, movement.feet, &movement.direction, events)?;
            }
        }
        Effect::Heal(heal) => {
            let amount = roll_dice(stream, &heal.dice)? as i32 + heal.modifier;
            for target_id in target_ids {
                let i = index_of(state, target_id)?;
                state.combatants[i].hp = (state.combatants[i].hp + amount).min(state.combatants[i].max_hp);
                // Restore from unconscious if healed above 0
                if state.combatants[i].hp > 0 && state.combatants[i].end_state == EndState::Unconscious {
                    state.combatants[i].end_state = EndState::Active;
                    state.combatants[i].death_saves = DeathSaves::default();
                    events.push(event("revive", format!("{target_id} regains consciousness"), actor_id, target_id));
                }
                events.push(event("heal", format!("{target_id} regains {amount} HP"), actor_id, target_id));
            }
        }
        Effect::TempHp(temp) => {
            let amount = roll_dice(stream, &temp.dice)? as i32 + temp.modifier;
            for target_id in if target_ids.is_empty() {
                vec![actor_id.to_string()]
            } else {
                target_ids.to_vec()
            } {
                let i = index_of(state, &target_id)?;
                state.combatants[i].temp_hp = state.combatants[i].temp_hp.max(amount);
            }
        }
        Effect::Teleport(_) => {
            for target_id in target_ids {
                let i = index_of(state, target_id)?;
                state.combatants[i].engagement.clear();
                for c in &mut state.combatants {
                    c.engagement.retain(|id| id != target_id);
                }
            }
        }
        Effect::MoveThenAttack(movement) => {
            let i = index_of(state, actor_id)?;
            let speed = stat_block(state, &state.combatants[i])?.speeds.get("walk").map(|s| s.feet).unwrap_or(0);
            if movement.move_feet > speed.saturating_sub(state.combatants[i].action_economy.movement_used_feet) {
                return Err(EngineError::Illegal("movement is unaffordable".into()));
            }
            state.combatants[i].action_economy.movement_used_feet += movement.move_feet;
            resolve_nested_action(state, stream, actor_id, &movement.attack_id, target_ids, events)?;
        }
        Effect::Multiattack(multi) => {
            resolve_multiattack_steps(state, stream, actor_id, multi.steps, target_ids, events)?;
        }
        Effect::UseAction(use_action) => {
            resolve_nested_action(state, stream, actor_id, &use_action.action_id, target_ids, events)?;
        }
        Effect::Cast(cast) => {
            events.push(event("cast", format!("{} casts {}", actor_id, cast.spell_id), actor_id, actor_id));
            resolve_cast(state, stream, actor_id, &cast.spell_id, target_ids, events)?;
        }
        Effect::RollModifier(roll_mod) => {
            state.active_modifiers.push(ActiveModifier {
                target: roll_mod.target.clone(),
                change: match roll_mod.disposition {
                    RollDisposition::Advantage => ModifierChange::Advantage,
                    RollDisposition::Disadvantage => ModifierChange::Disadvantage,
                    RollDisposition::Normal => ModifierChange::None,
                },
                remaining: Duration::Concentration,
                source: Some(actor_id.into()),
            });
        }
        Effect::Area(area) => {
            events.push(event("area", format!("{} creates an area effect", actor_id), actor_id, actor_id));
            let _ = area;
        }
    }
    Ok(())
}

fn resolve_save(
    state: &mut EncounterState,
    stream: &mut Stream,
    actor_id: &str,
    target_id: &str,
    save: &crate::effect::SavingThrow,
    events: &mut Vec<EngineEvent>,
) -> Result<(), EngineError> {
    let target = state.combatants[index_of(state, target_id)?].clone();
    let block = stat_block(state, &target)?;
    let roll = stream.roll_die(20);
    let modifier = block.saving_throws.get(&save.ability).copied()
        .unwrap_or_else(|| block.ability_scores.modifier(save.ability))
        + save_modifier(state, &target, save.ability);
    let success = roll as i32 + modifier >= save.dc;
    events.push(EngineEvent {
        kind: "save".into(),
        message: format!("{target_id} {} the save", if success { "passes" } else { "fails" }),
        actor_id: Some(actor_id.into()),
        target_id: Some(target_id.into()),
        roll: Some(roll),
        total: Some(roll as i32 + modifier),
        damage: None,
    });
    if !success {
        for damage in save.damage.iter().filter(|d| damage_applies(&target, actor_id, d)) {
            let amount = roll_dice(stream, &damage.dice)? as i32 + damage.modifier;
            let amount = adjust_damage(state, &target, amount, damage.damage_type);
            apply_damage(state, target_id, amount, Some(actor_id), events)?;
        }
    } else if save.no_damage_on_success {
        // no damage on success
    } else if save.half_on_success {
        for damage in save.damage.iter().filter(|d| damage_applies(&target, actor_id, d)) {
            let rolled = roll_dice(stream, &damage.dice)? as i32 + damage.modifier;
            let amount = adjust_damage(state, &target, rolled, damage.damage_type) / 2;
            apply_damage(state, target_id, amount, Some(actor_id), events)?;
        }
    } else {
        for damage in save.damage.iter().filter(|d| damage_applies(&target, actor_id, d)) {
            let amount = roll_dice(stream, &damage.dice)? as i32 + damage.modifier;
            let amount = adjust_damage(state, &target, amount, damage.damage_type);
            apply_damage(state, target_id, amount, Some(actor_id), events)?;
        }
    }
    Ok(())
}

fn resolve_multiattack_steps(
    state: &mut EncounterState,
    stream: &mut Stream,
    actor_id: &str,
    steps: Vec<crate::effect::MultiattackStep>,
    target_ids: &[String],
    events: &mut Vec<EngineEvent>,
) -> Result<(), EngineError> {
    use crate::effect::MultiattackStep;
    for step in steps {
        match step {
            MultiattackStep::Action { action_id, count, cast } => {
                for _ in 0..count {
                    resolve_multiattack_slot(state, stream, actor_id, &action_id, cast.as_ref(), target_ids, events)?;
                }
            }
            MultiattackStep::Choice { options, pick } => {
                // Phase 1 default: pick the first `pick` distinct options deterministically.
                // Live mode will later pause with a pending prompt for these picks; the batch
                // tactics policy will supply them as part of its plan. When that lands, the
                // engine will use those picks instead of this deterministic fallback.
                let pick = pick.min(options.len() as u32);
                for opt in options.into_iter().take(pick as usize) {
                    let label = opt.cast.as_ref().map_or(opt.action_id.as_str(), |c| c.spell_id.as_str());
                    events.push(multiattack_choice_event(actor_id, label));
                    for _ in 0..opt.count {
                        resolve_multiattack_slot(state, stream, actor_id, &opt.action_id, opt.cast.as_ref(), target_ids, events)?;
                    }
                }
            }
            MultiattackStep::Alternatives { sets } => {
                // Same deterministic fallback as `Choice`: the first set, until live prompts
                // and the batch tactics policy supply the pick.
                let Some(set) = sets.into_iter().next() else { continue };
                events.push(multiattack_choice_event(actor_id, "alternative 1"));
                resolve_multiattack_steps(state, stream, actor_id, set, target_ids, events)?;
            }
        }
    }
    Ok(())
}

fn multiattack_choice_event(actor_id: &str, choice: &str) -> EngineEvent {
    EngineEvent {
        kind: "multiattack_choice".into(),
        message: format!("{actor_id} chooses {choice} for multiattack"),
        actor_id: Some(actor_id.into()),
        target_id: None,
        roll: None,
        total: None,
        damage: None,
    }
}

fn resolve_multiattack_slot(
    state: &mut EncounterState,
    stream: &mut Stream,
    actor_id: &str,
    action_id: &str,
    cast: Option<&crate::effect::Cast>,
    target_ids: &[String],
    events: &mut Vec<EngineEvent>,
) -> Result<(), EngineError> {
    let Some(cast) = cast else {
        return resolve_nested_action(state, stream, actor_id, action_id, target_ids, events);
    };
    let i = index_of(state, actor_id)?;
    if !stat_block(state, &state.combatants[i])?.actions.iter().any(|a| a.id == action_id) {
        return Err(EngineError::NotFound(format!("action {action_id} not found")));
    }
    resolve_effect(state, stream, actor_id, Effect::Cast(cast.clone()), target_ids, events)
}

fn resolve_cast(
    state: &mut EncounterState,
    stream: &mut Stream,
    actor_id: &str,
    spell_id: &str,
    target_ids: &[String],
    events: &mut Vec<EngineEvent>,
) -> Result<(), EngineError> {
    // In phase 1, spell effects are resolved from the embedded spell effects file.
    // If the spell is not found, the cast is treated as unmodeled and logged only.
    let spell_effects = match load_spell_effects() {
        Ok(effects) => effects,
        Err(_) => {
            events.push(event("unmodeled_cast", format!("{} casts {} (spell effects file unavailable)", actor_id, spell_id), actor_id, actor_id));
            return Ok(());
        }
    };
    let spell = match spell_effects.get(spell_id) {
        Some(s) => s.clone(),
        None => {
            events.push(event("unmodeled_cast", format!("{} casts {} (not in effects file)", actor_id, spell_id), actor_id, actor_id));
            return Ok(());
        }
    };
    if spell.concentration {
        let i = index_of(state, actor_id)?;
        if state.combatants[i].concentration.is_some() {
            break_concentration(state, actor_id, events)?;
        }
        state.combatants[i].concentration = Some(spell_id.into());
        events.push(event("concentration", format!("{} begins concentrating on {}", actor_id, spell_id), actor_id, actor_id));
    }
    for effect in spell.effects {
        resolve_effect(state, stream, actor_id, effect, target_ids, events)?;
    }
    Ok(())
}

fn load_spell_effects() -> Result<HashMap<String, crate::spell::SpellEffect>, EngineError> {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let root = std::path::Path::new(manifest)
        .parent()
        .ok_or_else(|| EngineError::Invalid("workspace root not found".into()))?;
    let path = root.join("data/spells/effects.json");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| EngineError::Invalid(format!("failed to read spell effects: {e}")))?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| EngineError::Invalid(format!("invalid spell effects json: {e}")))?;
    let mut map = HashMap::new();
    if let Some(obj) = value.as_object() {
        for (key, entry_value) in obj {
            let spell: crate::spell::SpellEffect = serde_json::from_value(entry_value.clone())
                .map_err(|e| EngineError::Invalid(format!("spell {key} invalid: {e}")))?;
            map.insert(key.clone(), spell);
        }
    }
    Ok(map)
}

fn move_combatant(
    state: &mut EncounterState,
    actor_id: &str,
    target_id: Option<&str>,
    band: RangeBand,
    feet: u32,
) -> Result<Vec<EngineEvent>, EngineError> {
    let i = index_of(state, actor_id)?;
    let current = target_id
        .and_then(|id| state.combatants[i].ranges.get(id).copied())
        .unwrap_or(RangeBand::Engaged);
    let cost = match (current, band) {
        (RangeBand::Engaged, RangeBand::Engaged)
        | (RangeBand::Near, RangeBand::Near)
        | (RangeBand::Far, RangeBand::Far) => 0,
        (RangeBand::Engaged, RangeBand::Near) | (RangeBand::Near, RangeBand::Engaged) => 5,
        (RangeBand::Near, RangeBand::Far) | (RangeBand::Far, RangeBand::Near) => 30,
        (RangeBand::Engaged, RangeBand::Far) | (RangeBand::Far, RangeBand::Engaged) => 35,
    };
    if feet < cost {
        return Err(EngineError::Illegal(format!("movement requires {cost} feet")));
    }
    state.combatants[i].action_economy.movement_used_feet += cost;
    let mut events = vec![event("movement", format!("{actor_id} moves to {}", format!("{band:?}").to_lowercase()), actor_id, target_id.unwrap_or(actor_id))];
    if let Some(target_id) = target_id {
        let was_engaged = state.combatants[i].engagement.contains(&target_id.to_string());
        state.combatants[i].ranges.insert(target_id.into(), band);
        if band == RangeBand::Engaged {
            if !state.combatants[i].engagement.contains(&target_id.to_string()) {
                state.combatants[i].engagement.push(target_id.into());
            }
            let j = index_of(state, target_id)?;
            if !state.combatants[j].engagement.contains(&actor_id.to_string()) {
                state.combatants[j].engagement.push(actor_id.into());
            }
        } else {
            state.combatants[i].engagement.retain(|id| id != target_id);
            if let Ok(j) = index_of(state, target_id) {
                state.combatants[j].engagement.retain(|id| id != actor_id);
            }
            // Leaving engagement opens Opportunity Attack reaction window
            if was_engaged {
                if let Ok(j) = index_of(state, target_id) {
                    if state.combatants[j].action_economy.reaction_used {
                        events.push(event("opportunity_attack", format!("{target_id} has no reaction available"), target_id, actor_id));
                    } else {
                        events.push(event("opportunity_attack", format!("{target_id} may use an Opportunity Attack on {actor_id}"), target_id, actor_id));
                    }
                }
            }
        }
    }
    Ok(events)
}

fn end_turn_events(state: &mut EncounterState) -> Result<Vec<EngineEvent>, EngineError> {
    let current = state.active_turn.clone().ok_or_else(|| EngineError::Invalid("encounter has no active turn".into()))?;
    let mut events = vec![event("end_turn", format!("{current} ends its turn"), &current, &current)];

    // Decrement durations on active modifiers
    decrement_durations(state, &mut events);

    let current_index = state.initiative_order.iter().position(|id| id == &current).unwrap_or(0);
    let order_len = state.initiative_order.len().max(1);
    let mut next_index = (current_index + 1) % order_len;

    // Skip dead, unconscious, or stable combatants; wrap rounds
    let mut wrapped = false;
    for _ in 0..order_len {
        if let Some(id) = state.initiative_order.get(next_index) {
            if let Ok(i) = index_of(state, id) {
                match state.combatants[i].end_state {
                    EndState::Dead | EndState::Unconscious | EndState::Stable => {
                        events.push(event("forfeit", format!("{id} is skipped"), id, id));
                        next_index = (next_index + 1) % order_len;
                        if next_index == 0 { wrapped = true; }
                        continue;
                    }
                    EndState::Active => {}
                }
            }
        }
        break;
    }
    if next_index <= current_index && (current_index != next_index || wrapped) {
        state.round += 1;
    }
    state.active_turn = state.initiative_order.get(next_index).cloned();

    // Start-of-turn for next combatant
    if let Some(next) = state.active_turn.clone() {
        if let Ok(i) = index_of(state, &next) {
            state.combatants[i].action_economy = ActionEconomy::default();
            // Reset reaction
            state.combatants[i].action_economy.reaction_used = false;
            // Reset CooldownUntilNextTurn abilities
            reset_cooldowns(state, &next);
            // Roll recharge for abilities with Recharge usage
            roll_recharge(state, &next, &mut events)?;
            // Roll death saves for dying PCs
            roll_death_saves(state, &next, &mut events)?;
            // Repeat saves
            process_repeat_saves(state, &next, &mut events)?;
        }
        events.push(event("start_turn", format!("{next} starts its turn"), &next, &next));
    }
    Ok(events)
}

fn decrement_durations(state: &mut EncounterState, events: &mut Vec<EngineEvent>) {
    state.active_modifiers.retain_mut(|modifier| {
        let keep = match &mut modifier.remaining {
            Duration::Rounds(n) => {
                *n = n.saturating_sub(1);
                *n > 0
            }
            Duration::Minutes(n) => {
                *n = n.saturating_sub(1);
                *n > 0
            }
            Duration::Hours(n) => {
                *n = n.saturating_sub(1);
                *n > 0
            }
            Duration::Days(n) => {
                *n = n.saturating_sub(1);
                *n > 0
            }
            Duration::Concentration | Duration::Instant | Duration::Permanent | Duration::UntilSave { .. } => true,
        };
        if !keep {
            events.push(event("modifier_expire", "a timed modifier expired".into(), "engine", "engine"));
        }
        keep
    });
    // Decrement condition durations on each combatant
    for combatant in &mut state.combatants {
        let mut expired = Vec::new();
        for (idx, condition) in combatant.conditions.iter_mut().enumerate() {
            let keep = match &mut condition.duration {
                Duration::Rounds(n) => {
                    *n = n.saturating_sub(1);
                    *n > 0
                }
                Duration::Minutes(n) => {
                    *n = n.saturating_sub(1);
                    *n > 0
                }
                Duration::Hours(n) => {
                    *n = n.saturating_sub(1);
                    *n > 0
                }
                Duration::Days(n) => {
                    *n = n.saturating_sub(1);
                    *n > 0
                }
                Duration::Concentration | Duration::Instant | Duration::Permanent | Duration::UntilSave { .. } => true,
            };
            if !keep {
                expired.push(idx);
            }
        }
        for idx in expired.into_iter().rev() {
            let condition = combatant.conditions.remove(idx);
            events.push(event("condition_expire", format!("{} loses {}", combatant.name, format!("{:?}", condition.condition).to_lowercase()), &combatant.id, &combatant.id));
        }
    }
}

fn roll_recharge(state: &mut EncounterState, combatant_id: &str, events: &mut Vec<EngineEvent>) -> Result<(), EngineError> {
    let i = index_of(state, combatant_id)?;
    let block = stat_block(state, &state.combatants[i])?;
    let abilities: Vec<(String, String, Option<Usage>)> = block.actions.iter()
        .chain(block.traits.iter())
        .chain(block.bonus_actions.iter())
        .chain(block.reactions.iter())
        .map(|a| (a.id.clone(), a.name.clone(), a.usage.clone()))
        .collect();
    let mut stream = Stream::from_state(state.rng.clone());
    let mut recharged_groups = HashMap::<String, bool>::new();
    for (id, name, usage) in abilities {
        if let Some(Usage::Recharge { dice, group }) = usage {
            let group_key = group.clone().unwrap_or_else(|| id.clone());
            if recharged_groups.contains_key(&group_key) {
                continue;
            }
            let minimum = recharge_minimum(&dice)
                .ok_or_else(|| EngineError::Illegal(format!("recharge dice '{dice}' on {name} is not 'N-6' or 'N'")))?;
            let recharged = stream.roll_die(6) >= minimum;
            recharged_groups.insert(group_key.clone(), recharged);
            if recharged {
                let key = format!("{combatant_id}:{group_key}");
                if let Some(count) = state.combatants[i].resources.get_mut(&key) {
                    *count = 1;
                }
                events.push(event("recharge", format!("{combatant_id} recharges {}", name), combatant_id, combatant_id));
            }
        }
    }
    state.rng = stream.state();
    Ok(())
}

fn roll_death_saves(state: &mut EncounterState, combatant_id: &str, events: &mut Vec<EngineEvent>) -> Result<(), EngineError> {
    let i = index_of(state, combatant_id)?;
    if state.combatants[i].end_state != EndState::Unconscious {
        return Ok(());
    }
    let is_pc = stat_block(state, &state.combatants[i])?.class.is_some();
    if !is_pc {
        return Ok(());
    }
    let mut stream = Stream::from_state(state.rng.clone());
    let roll = stream.roll_die(20);
    state.rng = stream.state();
    if roll == 20 {
        state.combatants[i].hp = 1;
        state.combatants[i].end_state = EndState::Active;
        state.combatants[i].death_saves = DeathSaves::default();
        events.push(event("death_save", format!("{combatant_id} rolls a natural 20 and regains consciousness"), combatant_id, combatant_id));
    } else if roll == 1 {
        state.combatants[i].death_saves.failures += 2;
        events.push(event("death_save", format!("{combatant_id} rolls a natural 1: two failures"), combatant_id, combatant_id));
    } else if roll >= 10 {
        state.combatants[i].death_saves.successes += 1;
        events.push(event("death_save", format!("{combatant_id} succeeds a death save"), combatant_id, combatant_id));
    } else {
        state.combatants[i].death_saves.failures += 1;
        events.push(event("death_save", format!("{combatant_id} fails a death save"), combatant_id, combatant_id));
    }
    if state.combatants[i].death_saves.successes >= 3 {
        state.combatants[i].end_state = EndState::Stable;
        state.combatants[i].death_saves = DeathSaves::default();
        events.push(event("stabilize", format!("{combatant_id} stabilizes"), combatant_id, combatant_id));
    } else if state.combatants[i].death_saves.failures >= 3 {
        state.combatants[i].end_state = EndState::Dead;
        events.push(event("dies", format!("{combatant_id} dies"), combatant_id, combatant_id));
    }
    Ok(())
}

fn process_repeat_saves(state: &mut EncounterState, combatant_id: &str, events: &mut Vec<EngineEvent>) -> Result<(), EngineError> {
    let i = index_of(state, combatant_id)?;
    let conditions_to_check: Vec<usize> = state.combatants[i].conditions.iter().enumerate()
        .filter(|(_, c)| matches!(c.duration, Duration::UntilSave { .. }))
        .map(|(idx, _)| idx)
        .collect();
    let mut stream = Stream::from_state(state.rng.clone());
    for idx in conditions_to_check.into_iter().rev() {
        if let Duration::UntilSave { ability } = state.combatants[i].conditions[idx].duration {
            let block = stat_block(state, &state.combatants[i])?;
            let roll = stream.roll_die(20);
            let modifier = block.saving_throws.get(&ability)
                .copied()
                .unwrap_or_else(|| block.ability_scores.modifier(ability));
            let dc = 10;
            let success = roll as i32 + modifier >= dc;
            events.push(EngineEvent {
                kind: "repeat_save".into(),
                message: format!("{combatant_id} {} repeat save DC {dc}", if success { "passes" } else { "fails" }),
                actor_id: Some(combatant_id.into()),
                target_id: Some(combatant_id.into()),
                roll: Some(roll),
                total: Some(roll as i32 + modifier),
                damage: None,
            });
            if success {
                state.combatants[i].conditions.remove(idx);
            }
        }
    }
    state.rng = stream.state();
    Ok(())
}

fn reset_cooldowns(state: &mut EncounterState, combatant_id: &str) {
    let i = match index_of(state, combatant_id) {
        Ok(i) => i,
        Err(_) => return,
    };
    let prefix = format!("{combatant_id}:");
    let keys: Vec<String> = state.combatants[i].resources.keys()
        .filter(|k| k.starts_with(&prefix))
        .cloned()
        .collect();
    for key in keys {
        state.combatants[i].resources.insert(key, 1);
    }
}

fn respond(state: &mut EncounterState, values: Vec<u32>, accept: bool) -> Result<Vec<EngineEvent>, EngineError> {
    let prompt = state.pending.take().ok_or_else(|| EngineError::Invalid("no pending prompt".into()))?;
    if !accept {
        return Ok(vec![event("prompt_declined", prompt.message, &prompt.actor_id, &prompt.actor_id)]);
    }
    if let Some(sides) = prompt.sides {
        if values.iter().any(|v| *v == 0 || *v > sides) || values.len() < prompt.remaining as usize {
            state.pending = Some(prompt);
            return Err(EngineError::Invalid("prompt die face is outside its range".into()));
        }
    }
    Ok(vec![event("prompt_answered", format!("{} answered", prompt.id), &prompt.actor_id, &prompt.actor_id)])
}

fn apply_damage(
    state: &mut EncounterState,
    target_id: &str,
    amount: i32,
    source_id: Option<&str>,
    events: &mut Vec<EngineEvent>,
) -> Result<(), EngineError> {
    let i = index_of(state, target_id)?;
    let mut remaining = amount.max(0);
    let absorbed = remaining.min(state.combatants[i].temp_hp);
    state.combatants[i].temp_hp -= absorbed;
    remaining -= absorbed;
    state.combatants[i].hp -= remaining;
    if amount > 0 {
        events.push(EngineEvent {
            kind: "damage".into(),
            message: format!("{target_id} takes {amount} damage"),
            actor_id: source_id.map(|s| s.into()),
            target_id: Some(target_id.into()),
            roll: None,
            total: None,
            damage: Some(amount),
        });
    }
    if remaining > 0 && is_concentrating(&state.combatants[i]) {
        // Constitution save DC = max(10, damage / 2) to maintain concentration
        let dc = (remaining / 2).max(10);
        let mut stream = Stream::from_state(state.rng.clone());
        let block = stat_block(state, &state.combatants[i])?;
        let roll = stream.roll_die(20);
        let modifier = block.saving_throws.get(&Ability::Con)
            .copied()
            .unwrap_or_else(|| block.ability_scores.modifier(Ability::Con));
        let success = roll as i32 + modifier >= dc;
        state.rng = stream.state();
        events.push(EngineEvent {
            kind: "concentration_save".into(),
            message: format!("{target_id} {} concentration save DC {dc}", if success { "passes" } else { "fails" }),
            actor_id: source_id.map(|s| s.into()),
            target_id: Some(target_id.into()),
            roll: Some(roll),
            total: Some(roll as i32 + modifier),
            damage: None,
        });
        if !success {
            break_concentration(state, target_id, events)?;
        }
    }
    if state.combatants[i].hp <= 0 {
        let is_pc = stat_block(state, &state.combatants[i])?.class.is_some();
        if is_pc {
            state.combatants[i].end_state = EndState::Unconscious;
            state.combatants[i].death_saves = DeathSaves::default();
        } else {
            state.combatants[i].end_state = EndState::Dead;
        }
    }
    Ok(())
}

fn apply_forced_move(
    state: &mut EncounterState,
    actor_id: &str,
    target_id: &str,
    feet: u32,
    direction: &str,
    events: &mut Vec<EngineEvent>,
) -> Result<(), EngineError> {
    let i = index_of(state, target_id)?;
    if feet >= 5 {
        // Only clear engagement between the source and target
        state.combatants[i].engagement.retain(|id| id != actor_id);
        if let Ok(j) = index_of(state, actor_id) {
            state.combatants[j].engagement.retain(|id| id != target_id);
        }
    }
    let is_pull = direction.eq_ignore_ascii_case("pull");
    // Only modify the range band between source and target
    if let Some(band) = state.combatants[i].ranges.get_mut(actor_id) {
        if feet >= 35 {
            if is_pull && *band == RangeBand::Far {
                *band = RangeBand::Engaged;
            } else if !is_pull && *band == RangeBand::Engaged {
                *band = RangeBand::Far;
            }
        } else if !is_pull && *band == RangeBand::Engaged {
            *band = RangeBand::Near;
        }
    }
    // Mirror the range band on the source
    if let Ok(j) = index_of(state, actor_id) {
        if let Some(source_band) = state.combatants[j].ranges.get(target_id).copied() {
            let new_band = if is_pull && feet >= 35 {
                if source_band == RangeBand::Far { RangeBand::Engaged } else { source_band }
            } else if !is_pull && feet >= 35 {
                if source_band == RangeBand::Engaged { RangeBand::Far } else { source_band }
            } else if !is_pull && feet >= 5 {
                if source_band == RangeBand::Engaged { RangeBand::Near } else { source_band }
            } else {
                source_band
            };
            state.combatants[j].ranges.insert(target_id.into(), new_band);
        }
    }
    if is_pull && feet >= 35 {
        if let Some(band) = state.combatants[i].ranges.get(actor_id) {
            if *band == RangeBand::Engaged {
                if !state.combatants[i].engagement.contains(&actor_id.to_string()) {
                    state.combatants[i].engagement.push(actor_id.into());
                }
                if let Ok(j) = index_of(state, actor_id) {
                    if !state.combatants[j].engagement.contains(&target_id.to_string()) {
                        state.combatants[j].engagement.push(target_id.into());
                    }
                }
            }
        }
    }
    events.push(event("forced_move", format!("{target_id} is moved {feet} feet"), target_id, target_id));
    Ok(())
}

/// Whether a damage entry applies to this target: unconditional entries always do, gated entries need the
/// condition on the target (and, for `if_target_from_actor`, applied by the damaging creature).
fn damage_applies(target: &Combatant, actor_id: &str, damage: &crate::effect::Damage) -> bool {
    match damage.if_target {
        None => true,
        Some(condition) => target.conditions.iter().any(|c| {
            c.condition == condition && (!damage.if_target_from_actor || c.source.as_deref() == Some(actor_id))
        }),
    }
}

fn adjust_damage(state: &EncounterState, target: &Combatant, amount: i32, damage_type: DamageType) -> i32 {
    let block = stat_block(state, target).expect("combatant references a stat block");
    if block.damage_immunities.contains(&damage_type) { 0 }
    else if block.damage_vulnerabilities.contains(&damage_type) { amount * 2 }
    else if block.damage_resistances.contains(&damage_type) { amount / 2 }
    else { amount }
}

/// Compute roll disposition for an attack roll, accounting for conditions,
/// timed modifiers, and roll modifiers on the encounter lighting setting.
fn disposition(
    state: &EncounterState,
    actor: &Combatant,
    target: &Combatant,
    default: RollDisposition,
) -> RollDisposition {
    let mut d = default;
    // Actor conditions that impose disadvantage on attacks
    if actor.conditions.iter().any(|c| matches!(c.condition, Condition::Blinded | Condition::Frightened | Condition::Poisoned | Condition::Prone)) {
        d = worsen(d);
    }
    // Target conditions that impose advantage on attacks against it
    if target.conditions.iter().any(|c| matches!(c.condition, Condition::Paralyzed | Condition::Prone | Condition::Restrained | Condition::Stunned | Condition::Unconscious)) {
        d = improve(d);
    }
    // Target Invisible gives attacker disadvantage
    if target.conditions.iter().any(|c| matches!(c.condition, Condition::Invisible)) {
        d = worsen(d);
    }
    // Actor Invisible gives advantage
    if actor.conditions.iter().any(|c| matches!(c.condition, Condition::Invisible)) {
        d = improve(d);
    }
    // Apply timed modifiers and roll modifiers
    d = apply_modifier_target_disposition(state, actor, ModifierTarget::AttackRolls, d);
    d
}

fn improve(d: RollDisposition) -> RollDisposition {
    match d {
        RollDisposition::Disadvantage => RollDisposition::Normal,
        RollDisposition::Normal => RollDisposition::Advantage,
        RollDisposition::Advantage => RollDisposition::Advantage,
    }
}

fn worsen(d: RollDisposition) -> RollDisposition {
    match d {
        RollDisposition::Advantage => RollDisposition::Normal,
        RollDisposition::Normal => RollDisposition::Disadvantage,
        RollDisposition::Disadvantage => RollDisposition::Disadvantage,
    }
}

fn apply_modifier_target_disposition(
    state: &EncounterState,
    combatant: &Combatant,
    target: ModifierTarget,
    default: RollDisposition,
) -> RollDisposition {
    let mut d = default;
    for modifier in &state.active_modifiers {
        if modifier.source.as_deref() == Some(&combatant.id) {
            if modifier.target == target {
                match modifier.change {
                    ModifierChange::Advantage => d = improve(d),
                    ModifierChange::Disadvantage => d = worsen(d),
                    _ => {}
                }
            }
        }
    }
    d
}

/// Compute extra attack modifier from timed modifiers.
fn attack_modifier(state: &EncounterState, actor: &Combatant) -> Result<i32, EngineError> {
    let mut total = 0;
    for modifier in &state.active_modifiers {
        if modifier.source.as_deref() == Some(&actor.id) {
            if modifier.target == ModifierTarget::AttackRolls {
                if let ModifierChange::Add(n) = modifier.change {
                    total += n;
                }
            }
        }
    }
    Ok(total)
}

/// Compute save modifier from timed modifiers.
fn save_modifier(state: &EncounterState, target: &Combatant, ability: Ability) -> i32 {
    let mut total = 0;
    for modifier in &state.active_modifiers {
        if modifier.source.as_deref() == Some(&target.id) {
            let matches = match modifier.target {
                ModifierTarget::SavingThrows { ability: Some(a) } => a == ability,
                ModifierTarget::SavingThrows { ability: None } => true,
                _ => false,
            };
            if matches {
                if let ModifierChange::Add(n) = modifier.change {
                    total += n;
                }
            }
        }
    }
    total
}

/// Compute AC modifier from timed modifiers.
fn ac_modifier(state: &EncounterState, target: &Combatant) -> i32 {
    let mut total = 0;
    for modifier in &state.active_modifiers {
        if modifier.source.as_deref() == Some(&target.id) {
            if modifier.target == ModifierTarget::ArmorClass {
                if let ModifierChange::Add(n) = modifier.change {
                    total += n;
                }
            }
        }
    }
    total
}

/// True if the combatant is currently concentrating on a spell.
fn is_concentrating(c: &Combatant) -> bool {
    c.concentration.is_some()
}

/// Break concentration and log it.
fn break_concentration(
    state: &mut EncounterState,
    target_id: &str,
    events: &mut Vec<EngineEvent>,
) -> Result<(), EngineError> {
    let i = index_of(state, target_id)?;
    if state.combatants[i].concentration.take().is_some() {
        events.push(event("concentration", format!("{target_id} loses concentration"), target_id, target_id));
    }
    Ok(())
}

/// Prompt for user-entered dice faces.
fn dice_prompt(state: &mut EncounterState, actor_id: &str, sides: u32, remaining: u32, message: &str) {
    state.pending = Some(PendingPrompt {
        id: format!("dice-{actor_id}-{}", state.round),
        kind: "dice".into(),
        actor_id: actor_id.into(),
        message: message.into(),
        sides: Some(sides),
        remaining,
        metadata: serde_json::Value::Null,
    });
}

fn roll_die_with_prompt(
    stream: &mut Stream,
    _state: &mut EncounterState,
    _actor_id: &str,
    sides: u32,
    _reason: &str,
) -> Option<u32> {
    // For phase 1, engine rolls by default; pc_dice=ask would create a pending prompt here.
    // The prompt mechanism is wired but the full pc_dice flow is left for live-mode integration.
    Some(stream.roll_die(sides))
}

fn roll_dice(stream: &mut Stream, expression: &str) -> Result<u32, EngineError> {
    let (count, sides) = expression.split_once('d').ok_or_else(|| EngineError::Invalid(format!("invalid dice expression {expression}")))?;
    let count: u32 = count.parse().map_err(|_| EngineError::Invalid(format!("invalid dice expression {expression}")))?;
    let sides: u32 = sides.parse().map_err(|_| EngineError::Invalid(format!("invalid dice expression {expression}")))?;
    if count == 0 || sides == 0 { return Err(EngineError::Invalid(format!("invalid dice expression {expression}"))); }
    Ok((0..count).map(|_| stream.roll_die(sides)).sum())
}

fn can_act(c: &Combatant) -> bool {
    c.end_state == EndState::Active
        && !c.conditions.iter().any(|x| matches!(x.condition,
            Condition::Incapacitated | Condition::Paralyzed | Condition::Petrified | Condition::Stunned | Condition::Unconscious))
}
fn index_of(state: &EncounterState, id: &str) -> Result<usize, EngineError> { state.combatants.iter().position(|c| c.id == id).ok_or_else(|| EngineError::NotFound(format!("combatant {id} not found"))) }
fn stat_block<'a>(state: &'a EncounterState, c: &Combatant) -> Result<&'a StatBlock, EngineError> { state.stat_blocks.iter().find(|s| s.id == c.stat_block_id).map(|s| &s.stat_block).ok_or_else(|| EngineError::NotFound(format!("stat block {} not found", c.stat_block_id))) }
fn event(kind: &str, message: String, actor_id: &str, target_id: &str) -> EngineEvent { EngineEvent { kind: kind.into(), message, actor_id: Some(actor_id.into()), target_id: Some(target_id.into()), roll: None, total: None, damage: None } }
fn response(state: &EncounterState, events: Vec<EngineEvent>) -> StepResponse { StepResponse { events, digest: serde_json::to_value(state).unwrap_or(serde_json::Value::Null), pending: state.pending.clone() } }

fn resolve_nested_action(
    state: &mut EncounterState,
    stream: &mut Stream,
    actor_id: &str,
    action_id: &str,
    target_ids: &[String],
    events: &mut Vec<EngineEvent>,
) -> Result<(), EngineError> {
    let i = index_of(state, actor_id)?;
    let action = stat_block(state, &state.combatants[i])?
        .actions
        .iter()
        .find(|a| a.id == action_id)
        .cloned()
        .ok_or_else(|| EngineError::NotFound(format!("action {action_id} not found")))?;
    for effect in action.effects {
        resolve_effect(state, stream, actor_id, effect, target_ids, events)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::{Attack, Damage};
    use crate::encounter::{Side, StatBlockSnapshot};
    use crate::rng::RngState;
    use crate::stat_block::{
        AbilityScores, ArmorClass, HitPoints, Senses, Size, SpeedValue,
    };

    fn test_stat_block(id: &str, name: &str, hp: i32, ac: i32) -> StatBlock {
        StatBlock {
            id: id.into(),
            name: name.into(),
            source: "test".into(),
            schema_version: "1".into(),
            size: Size::Medium,
            size_choice: vec![],
            creature_type: "humanoid".into(),
            alignment: "neutral".into(),
            armor_class: vec![ArmorClass { value: ac, source: None }],
            hit_points: HitPoints { average: hp, dice: format!("{hp}d8") },
            speeds: {
                let mut m = HashMap::new();
                m.insert("walk".into(), SpeedValue { feet: 30, hover: false });
                m
            },
            ability_scores: AbilityScores {
                strength: 10,
                dexterity: 10,
                constitution: 10,
                intelligence: 10,
                wisdom: 10,
                charisma: 10,
            },
            saving_throws: HashMap::new(),
            skills: HashMap::new(),
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
            death_saves: DeathSaves::default(),
            gear: vec![],
            spellcasting: None,
            notes: vec![],
            pasted_source: None,
            intake_flags: vec![],
        }
    }

    fn test_encounter(
        stat_blocks: Vec<StatBlock>,
        combatants: Vec<Combatant>,
    ) -> EncounterState {
        let stat_block_snapshots: Vec<StatBlockSnapshot> = stat_blocks
            .into_iter()
            .map(|s| StatBlockSnapshot::from_stat_block(s.id.clone(), s))
            .collect();
        let mut state = EncounterState {
            id: "test-enc".into(),
            engine_version: "0.1.0".into(),
            origin_directory: "/tmp".into(),
            stat_blocks: stat_block_snapshots,
            sides: vec![
                Side { id: "ally".into(), name: "Allies".into() },
                Side { id: "enemy".into(), name: "Enemies".into() },
            ],
            combatants,
            round: 1,
            initiative_order: vec![],
            active_turn: None,
            active_effects: vec![],
            active_modifiers: vec![],
            standing_assumptions: vec![],
            command_history: vec![],
            pending: None,
            rng: RngState::new(1),
            tamper_hash: String::new(),
        };
        state.initiative_order = state.combatants.iter().map(|c| c.id.clone()).collect();
        state.active_turn = state.initiative_order.first().cloned();
        state.update_tamper_hash();
        state
    }

    fn test_combatant(id: &str, stat_block_id: &str, hp: i32, side: &str) -> Combatant {
        Combatant {
            id: id.into(),
            stat_block_id: stat_block_id.into(),
            name: id.into(),
            side_id: side.into(),
            hp,
            temp_hp: 0,
            max_hp: hp,
            conditions: vec![],
            concentration: None,
            resources: HashMap::new(),
            action_economy: ActionEconomy::default(),
            engagement: vec![],
            ranges: HashMap::new(),
            end_state: EndState::Active,
            death_saves: DeathSaves::default(),
        }
    }

    fn recharge_state(dice: &str, seed: u64) -> EncounterState {
        let mut block = test_stat_block("drake", "Drake", 20, 12);
        block.actions.push(crate::stat_block::AbilityEntry {
            id: "breath".into(),
            name: "Breath".into(),
            desc: None,
            kind: crate::stat_block::AbilityKind::Modeled,
            effects: vec![],
            agent_clauses: vec![],
            default_resolution: None,
            usage: Some(Usage::Recharge { dice: dice.into(), group: None }),
        });
        let mut drake = test_combatant("drake", "drake", 20, "enemy");
        drake.resources.insert("drake:breath".into(), 0);
        let mut state = test_encounter(vec![block], vec![drake]);
        state.rng = RngState::new(seed);
        state
    }

    #[test]
    fn recharge_dice_ranges_set_the_minimum_roll() {
        for dice in ["2-6", "3-6", "4-6", "5-6", "6", "4"] {
            let minimum = recharge_minimum(dice).unwrap();
            for seed in 1..40 {
                let roll = Stream::from_state(RngState::new(seed)).roll_die(6);
                let mut state = recharge_state(dice, seed);
                let mut events = vec![];
                roll_recharge(&mut state, "drake", &mut events).unwrap();
                assert_eq!(state.combatants[0].resources["drake:breath"] == 1, roll >= minimum, "{dice} seed {seed}");
                assert_eq!(!events.is_empty(), roll >= minimum);
            }
        }
    }

    #[test]
    fn roll_recharge_errors_on_unparseable_dice() {
        let mut state = recharge_state("1d6", 1);
        assert!(roll_recharge(&mut state, "drake", &mut vec![]).is_err());
    }

    #[test]
    fn dice_parser_rejects_malformed_input() {
        assert!(roll_dice(&mut Stream::from_seed(1), "d6").is_err());
        assert!(roll_dice(&mut Stream::from_seed(1), "1d0").is_err());
    }

    #[test]
    fn all_conditions_are_closed_vocabulary() {
        let json = serde_json::to_string(&Condition::Unconscious).unwrap();
        assert_eq!(json, "\"unconscious\"");
    }

    #[test]
    fn attack_hits_and_deals_damage() {
        let goblin_block = test_stat_block("goblin", "Goblin", 7, 15);
        let mut goblin_block_with_action = goblin_block.clone();
        goblin_block_with_action.actions.push(crate::stat_block::AbilityEntry {
            id: "scimitar".into(),
            name: "Scimitar".into(),
            desc: None,
            kind: crate::stat_block::AbilityKind::Modeled,
            effects: vec![Effect::Attack(Attack {
                modifier: 4,
                reach: Some(5),
                range: None,
                target_count: 1,
                damage: vec![Damage { dice: "1d6".into(), damage_type: DamageType::Slashing, modifier: 2, magical: false, if_target: None, if_target_from_actor: false }],
                on_hit: vec![],
            })],
            agent_clauses: vec![],
            default_resolution: None,
            usage: None,
        });
        let fighter = test_combatant("fighter", "fighter", 20, "ally");
        let goblin = test_combatant("goblin", "goblin", 7, "enemy");
        let fighter_block = test_stat_block("fighter", "Fighter", 20, 5);
        let mut state = test_encounter(
            vec![fighter_block, goblin_block_with_action],
            vec![fighter, goblin],
        );
        state.combatants[0].engagement.push("goblin".into());
        state.combatants[1].engagement.push("fighter".into());
        state.active_turn = Some("goblin".into());
        let req = StepRequest::Action { actor_id: "goblin".into(), action_id: "scimitar".into(), target_ids: vec!["fighter".into()] };
        let resp = step(&mut state, req).unwrap();
        assert!(!resp.events.is_empty());
        let attack_event = resp.events.iter().find(|e| e.kind == "attack").expect("attack event");
        assert!(attack_event.total.is_some());
        let damage_event = resp.events.iter().find(|e| e.kind == "damage");
        assert!(damage_event.is_some());
    }

    #[test]
    fn pc_drops_to_zero_becomes_unconscious() {
        let goblin_block = test_stat_block("goblin", "Goblin", 7, 15);
        let mut goblin_block_with_action = goblin_block.clone();
        goblin_block_with_action.actions.push(crate::stat_block::AbilityEntry {
            id: "smash".into(),
            name: "Smash".into(),
            desc: None,
            kind: crate::stat_block::AbilityKind::Modeled,
            effects: vec![Effect::Attack(Attack {
                modifier: 10,
                reach: Some(5),
                range: None,
                target_count: 1,
                damage: vec![Damage { dice: "10d6".into(), damage_type: DamageType::Bludgeoning, modifier: 10, magical: false, if_target: None, if_target_from_actor: false }],
                on_hit: vec![],
            })],
            agent_clauses: vec![],
            default_resolution: None,
            usage: None,
        });
        let mut fighter_block = test_stat_block("fighter", "Fighter", 20, 16);
        fighter_block.class = Some("fighter".into());
        let fighter = test_combatant("fighter", "fighter", 20, "ally");
        let goblin = test_combatant("goblin", "goblin", 7, "enemy");
        let mut state = test_encounter(
            vec![fighter_block, goblin_block_with_action],
            vec![fighter, goblin],
        );
        state.combatants[0].engagement.push("goblin".into());
        state.combatants[1].engagement.push("fighter".into());
        state.active_turn = Some("goblin".into());
        let req = StepRequest::Action { actor_id: "goblin".into(), action_id: "smash".into(), target_ids: vec!["fighter".into()] };
        step(&mut state, req).unwrap();
        assert_eq!(state.combatants[0].end_state, EndState::Unconscious);
    }

    #[test]
    fn monster_dies_at_zero() {
        let mut fighter_block = test_stat_block("fighter", "Fighter", 20, 16);
        fighter_block.actions.push(crate::stat_block::AbilityEntry {
            id: "smash".into(),
            name: "Smash".into(),
            desc: None,
            kind: crate::stat_block::AbilityKind::Modeled,
            effects: vec![Effect::Attack(Attack {
                modifier: 10,
                reach: Some(5),
                range: None,
                target_count: 1,
                damage: vec![Damage { dice: "10d6".into(), damage_type: DamageType::Bludgeoning, modifier: 10, magical: false, if_target: None, if_target_from_actor: false }],
                on_hit: vec![],
            })],
            agent_clauses: vec![],
            default_resolution: None,
            usage: None,
        });
        let goblin_block = test_stat_block("goblin", "Goblin", 7, 15);
        let fighter = test_combatant("fighter", "fighter", 20, "ally");
        let goblin = test_combatant("goblin", "goblin", 7, "enemy");
        let mut state = test_encounter(
            vec![fighter_block, goblin_block],
            vec![fighter, goblin],
        );
        state.combatants[0].engagement.push("goblin".into());
        state.combatants[1].engagement.push("fighter".into());
        let req = StepRequest::Action { actor_id: "fighter".into(), action_id: "smash".into(), target_ids: vec!["goblin".into()] };
        step(&mut state, req).unwrap();
        assert_eq!(state.combatants[1].end_state, EndState::Dead);
    }

    #[test]
    fn end_turn_skips_dead() {
        let fighter = test_combatant("fighter", "fighter", 20, "ally");
        let goblin = test_combatant("goblin", "goblin", 7, "enemy");
        let mut state = test_encounter(
            vec![test_stat_block("fighter", "Fighter", 20, 16), test_stat_block("goblin", "Goblin", 7, 15)],
            vec![fighter, goblin],
        );
        state.combatants[1].end_state = EndState::Dead;
        let req = StepRequest::EndTurn;
        let resp = step(&mut state, req).unwrap();
        assert!(resp.events.iter().any(|e| e.kind == "forfeit"));
    }

    #[test]
    fn movement_leaves_engagement() {
        let fighter = test_combatant("fighter", "fighter", 20, "ally");
        let goblin = test_combatant("goblin", "goblin", 7, "enemy");
        let mut state = test_encounter(
            vec![test_stat_block("fighter", "Fighter", 20, 16), test_stat_block("goblin", "Goblin", 7, 15)],
            vec![fighter, goblin],
        );
        state.combatants[0].ranges.insert("goblin".into(), RangeBand::Engaged);
        state.combatants[0].engagement.push("goblin".into());
        state.combatants[1].engagement.push("fighter".into());
        let req = StepRequest::Move { actor_id: "fighter".into(), target_id: Some("goblin".into()), band: RangeBand::Near, feet: 5 };
        let resp = step(&mut state, req).unwrap();
        assert!(resp.events.iter().any(|e| e.kind == "movement"));
        assert!(resp.events.iter().any(|e| e.kind == "opportunity_attack"));
        assert!(!state.combatants[0].engagement.contains(&"goblin".to_string()));
    }

    #[test]
    fn timed_modifier_applies_to_attack() {
        let fighter = test_combatant("fighter", "fighter", 20, "ally");
        let goblin = test_combatant("goblin", "goblin", 7, "enemy");
        let mut state = test_encounter(
            vec![test_stat_block("fighter", "Fighter", 20, 16), test_stat_block("goblin", "Goblin", 7, 15)],
            vec![fighter, goblin],
        );
        state.active_modifiers.push(ActiveModifier {
            target: ModifierTarget::AttackRolls,
            change: ModifierChange::Add(5),
            remaining: Duration::Rounds(1),
            source: Some("fighter".into()),
        });
        let modifier = attack_modifier(&state, &state.combatants[0]).unwrap();
        assert_eq!(modifier, 5);
    }

    #[test]
    fn disposition_blinded_gives_disadvantage() {
        let mut fighter = test_combatant("fighter", "fighter", 20, "ally");
        let goblin = test_combatant("goblin", "goblin", 7, "enemy");
        fighter.conditions.push(ConditionInstance {
            condition: Condition::Blinded,
            duration: Duration::Rounds(1),
            source: None,
        });
        let state = test_encounter(
            vec![test_stat_block("fighter", "Fighter", 20, 16), test_stat_block("goblin", "Goblin", 7, 15)],
            vec![fighter, goblin],
        );
        let d = disposition(&state, &state.combatants[0], &state.combatants[1], RollDisposition::Normal);
        assert_eq!(d, RollDisposition::Disadvantage);
    }

    #[test]
    fn initiative_rolls_with_grouping() {
        let fighter = test_combatant("fighter", "fighter", 20, "ally");
        let goblin_a = test_combatant("goblin_a", "goblin", 7, "enemy");
        let goblin_b = test_combatant("goblin_b", "goblin", 7, "enemy");
        let mut state = test_encounter(
            vec![test_stat_block("fighter", "Fighter", 20, 16), test_stat_block("goblin", "Goblin", 7, 15)],
            vec![fighter, goblin_a, goblin_b],
        );
        let opts = InitiativeOptions { individual: false, surprised: vec![], pc_dice: false };
        let events = roll_initiative(&mut state, opts).unwrap();
        assert_eq!(events.len(), 3);
        // Grouped initiative means goblin_a and goblin_b share a roll value
        let goblin_a_total = events.iter().find(|e| e.actor_id.as_deref() == Some("goblin_a")).unwrap().total.unwrap();
        let goblin_b_total = events.iter().find(|e| e.actor_id.as_deref() == Some("goblin_b")).unwrap().total.unwrap();
        assert_eq!(goblin_a_total, goblin_b_total);
    }

    #[test]
    fn multiattack_choice_resolves_k_distinct_picks() {
        let mut attacker_block = test_stat_block("attacker", "Attacker", 30, 15);
        attacker_block.actions.push(crate::stat_block::AbilityEntry {
            id: "multiattack".into(),
            name: "Multiattack".into(),
            desc: None,
            kind: crate::stat_block::AbilityKind::Modeled,
            effects: vec![Effect::Multiattack(crate::effect::Multiattack {
                steps: vec![
                    crate::effect::MultiattackStep::Action { action_id: "claw".into(), count: 1, cast: None },
                    crate::effect::MultiattackStep::Choice {
                        options: vec![
                            crate::effect::ChoiceOption { action_id: "claw".into(), count: 1, cast: None },
                            crate::effect::ChoiceOption { action_id: "bite".into(), count: 1, cast: None },
                        ],
                        pick: 1,
                    },
                ],
            })],
            agent_clauses: vec![],
            default_resolution: None,
            usage: None,
        });
        attacker_block.actions.push(crate::stat_block::AbilityEntry {
            id: "claw".into(),
            name: "Claw".into(),
            desc: None,
            kind: crate::stat_block::AbilityKind::Modeled,
            effects: vec![Effect::Attack(Attack {
                modifier: 5,
                reach: Some(5),
                range: None,
                target_count: 1,
                damage: vec![Damage { dice: "1d6".into(), damage_type: DamageType::Slashing, modifier: 3, magical: false, if_target: None, if_target_from_actor: false }],
                on_hit: vec![],
            })],
            agent_clauses: vec![],
            default_resolution: None,
            usage: None,
        });
        attacker_block.actions.push(crate::stat_block::AbilityEntry {
            id: "bite".into(),
            name: "Bite".into(),
            desc: None,
            kind: crate::stat_block::AbilityKind::Modeled,
            effects: vec![Effect::Attack(Attack {
                modifier: 5,
                reach: Some(5),
                range: None,
                target_count: 1,
                damage: vec![Damage { dice: "1d8".into(), damage_type: DamageType::Piercing, modifier: 3, magical: false, if_target: None, if_target_from_actor: false }],
                on_hit: vec![],
            })],
            agent_clauses: vec![],
            default_resolution: None,
            usage: None,
        });
        let target_block = test_stat_block("target", "Target", 50, 10);
        let attacker = test_combatant("attacker", "attacker", 30, "enemy");
        let target = test_combatant("target", "target", 50, "ally");
        let mut state = test_encounter(
            vec![target_block, attacker_block],
            vec![target, attacker],
        );
        state.combatants[0].engagement.push("attacker".into());
        state.combatants[1].engagement.push("target".into());
        state.active_turn = Some("attacker".into());
        let req = StepRequest::Action { actor_id: "attacker".into(), action_id: "multiattack".into(), target_ids: vec!["target".into()] };
        let resp = step(&mut state, req).unwrap();
        let attack_events: Vec<_> = resp.events.iter().filter(|e| e.kind == "attack").collect();
        assert_eq!(attack_events.len(), 2, "fixed 1 attack plus choice 1 attack");
    }

    #[test]
    fn multiattack_alternatives_and_spell_slots() {
        use crate::effect::{Cast, ChoiceOption, Multiattack, MultiattackStep};
        let melee = |id: &str, dice: &str| crate::stat_block::AbilityEntry {
            id: id.into(),
            name: id.into(),
            desc: None,
            kind: crate::stat_block::AbilityKind::Modeled,
            effects: vec![Effect::Attack(Attack {
                modifier: 5,
                reach: Some(5),
                range: None,
                target_count: 1,
                damage: vec![Damage { dice: dice.into(), damage_type: DamageType::Slashing, modifier: 3, magical: false, if_target: None, if_target_from_actor: false }],
                on_hit: vec![],
            })],
            agent_clauses: vec![],
            default_resolution: None,
            usage: None,
        };
        let fog = Cast { spell_id: "fog-cloud".into(), slot_level: None, upcast: false };
        let mut attacker_block = test_stat_block("attacker", "Attacker", 30, 15);
        attacker_block.actions.push(crate::stat_block::AbilityEntry {
            effects: vec![Effect::Multiattack(Multiattack {
                steps: vec![
                    MultiattackStep::Alternatives {
                        sets: vec![
                            vec![MultiattackStep::Action { action_id: "claw".into(), count: 2, cast: None }],
                            vec![MultiattackStep::Action { action_id: "bite".into(), count: 3, cast: None }],
                        ],
                    },
                    MultiattackStep::Choice {
                        options: vec![ChoiceOption { action_id: "spellcasting".into(), count: 1, cast: Some(fog.clone()) }],
                        pick: 1,
                    },
                    MultiattackStep::Alternatives {
                        sets: vec![vec![MultiattackStep::Action { action_id: "spellcasting".into(), count: 1, cast: Some(fog) }], vec![]],
                    },
                    MultiattackStep::Alternatives { sets: vec![] },
                ],
            })],
            ..melee("multiattack", "1d4")
        });
        attacker_block.actions.push(melee("claw", "1d6"));
        attacker_block.actions.push(melee("bite", "1d8"));
        attacker_block.actions.push(crate::stat_block::AbilityEntry { effects: vec![], ..melee("spellcasting", "1d4") });
        let target_block = test_stat_block("target", "Target", 50, 10);
        let attacker = test_combatant("attacker", "attacker", 30, "enemy");
        let target = test_combatant("target", "target", 50, "ally");
        let mut state = test_encounter(vec![target_block, attacker_block], vec![target, attacker]);
        state.combatants[0].engagement.push("attacker".into());
        state.combatants[1].engagement.push("target".into());
        state.active_turn = Some("attacker".into());
        let req = StepRequest::Action { actor_id: "attacker".into(), action_id: "multiattack".into(), target_ids: vec!["target".into()] };
        let resp = step(&mut state, req).unwrap();
        let count = |kind: &str| resp.events.iter().filter(|e| e.kind == kind).count();
        assert_eq!(count("attack"), 2, "first alternative: two claws");
        assert_eq!(count("cast"), 2, "spell choice option plus the optional cast");
        assert!(resp.events.iter().any(|e| e.kind == "multiattack_choice" && e.message.contains("fog-cloud")));
    }

    #[test]
    fn multiattack_cast_requires_the_named_action() {
        use crate::effect::{Cast, Multiattack, MultiattackStep};
        let mut attacker_block = test_stat_block("attacker", "Attacker", 30, 15);
        attacker_block.actions.push(crate::stat_block::AbilityEntry {
            id: "multiattack".into(),
            name: "Multiattack".into(),
            desc: None,
            kind: crate::stat_block::AbilityKind::Modeled,
            effects: vec![Effect::Multiattack(Multiattack {
                steps: vec![MultiattackStep::Action {
                    action_id: "spellcasting".into(),
                    count: 1,
                    cast: Some(Cast { spell_id: "fog-cloud".into(), slot_level: None, upcast: false }),
                }],
            })],
            agent_clauses: vec![],
            default_resolution: None,
            usage: None,
        });
        let attacker = test_combatant("attacker", "attacker", 30, "enemy");
        let target = test_combatant("target", "target", 50, "ally");
        let mut state = test_encounter(vec![test_stat_block("target", "Target", 50, 10), attacker_block], vec![target, attacker]);
        state.active_turn = Some("attacker".into());
        let req = StepRequest::Action { actor_id: "attacker".into(), action_id: "multiattack".into(), target_ids: vec!["target".into()] };
        assert!(matches!(step(&mut state, req), Err(EngineError::NotFound(_))));
    }

    #[test]
    fn concentration_breaks_on_damage() {
        let mut fighter_block = test_stat_block("fighter", "Fighter", 20, 16);
        fighter_block.class = Some("fighter".into());
        let mut goblin_block = test_stat_block("goblin", "Goblin", 7, 15);
        goblin_block.actions.push(crate::stat_block::AbilityEntry {
            id: "arrow".into(),
            name: "Arrow".into(),
            desc: None,
            kind: crate::stat_block::AbilityKind::Modeled,
            effects: vec![Effect::Attack(Attack {
                modifier: 10,
                reach: None,
                range: Some((30, 120)),
                target_count: 1,
                damage: vec![Damage { dice: "2d6".into(), damage_type: DamageType::Piercing, modifier: 2, magical: false, if_target: None, if_target_from_actor: false }],
                on_hit: vec![],
            })],
            agent_clauses: vec![],
            default_resolution: None,
            usage: None,
        });
        let fighter = test_combatant("fighter", "fighter", 20, "ally");
        let goblin = test_combatant("goblin", "goblin", 7, "enemy");
        let mut state = test_encounter(
            vec![fighter_block, goblin_block],
            vec![fighter, goblin],
        );
        state.combatants[0].concentration = Some("shield-of-faith".into());
        state.combatants[0].ranges.insert("goblin".into(), RangeBand::Near);
        state.combatants[1].ranges.insert("fighter".into(), RangeBand::Near);
        state.active_turn = Some("goblin".into());
        let req = StepRequest::Action { actor_id: "goblin".into(), action_id: "arrow".into(), target_ids: vec!["fighter".into()] };
        let resp = step(&mut state, req).unwrap();
        assert!(resp.events.iter().any(|e| e.kind == "concentration_save" || e.kind == "concentration"));
    }

    fn gated(dice: &str, modifier: i32, damage_type: DamageType, condition: Option<Condition>, from_actor: bool) -> Damage {
        Damage { dice: dice.into(), damage_type, modifier, magical: false, if_target: condition, if_target_from_actor: from_actor }
    }

    /// A goblin that cannot miss a fighter with an attack made of `damage` and `on_hit` effects.
    fn attack_state(damage: Vec<Damage>, on_hit: Vec<Effect>, setup: impl FnOnce(&mut StatBlock, &mut Combatant)) -> EncounterState {
        let mut goblin_block = test_stat_block("goblin", "Goblin", 7, 15);
        goblin_block.actions.push(crate::stat_block::AbilityEntry {
            id: "bite".into(),
            name: "Bite".into(),
            desc: None,
            kind: crate::stat_block::AbilityKind::Modeled,
            effects: vec![Effect::Attack(Attack { modifier: 100, reach: Some(5), range: None, target_count: 1, damage, on_hit })],
            agent_clauses: vec![],
            default_resolution: None,
            usage: None,
        });
        let mut fighter_block = test_stat_block("fighter", "Fighter", 100, 5);
        let mut fighter = test_combatant("fighter", "fighter", 100, "ally");
        setup(&mut fighter_block, &mut fighter);
        let goblin = test_combatant("goblin", "goblin", 7, "enemy");
        let mut state = test_encounter(vec![fighter_block, goblin_block], vec![fighter, goblin]);
        state.combatants[0].engagement.push("goblin".into());
        state.combatants[1].engagement.push("fighter".into());
        state.active_turn = Some("goblin".into());
        state
    }

    /// Hit points the fighter lost to one bite, asserting the attack was a plain (non-critical) hit.
    fn bite_damage(state: &mut EncounterState) -> i32 {
        let resp = step(state, StepRequest::Action { actor_id: "goblin".into(), action_id: "bite".into(), target_ids: vec!["fighter".into()] }).unwrap();
        let attack = resp.events.iter().find(|e| e.kind == "attack").expect("attack event");
        assert_ne!(attack.roll, Some(20), "seed produced a critical hit; pick another seed");
        assert!(attack.message.ends_with("(hit)"), "{}", attack.message);
        100 - state.combatants.iter().find(|c| c.id == "fighter").unwrap().hp
    }

    fn condition_from(condition: Condition, source: &str) -> ConditionInstance {
        ConditionInstance { condition, duration: Duration::Permanent, source: Some(source.into()) }
    }

    #[test]
    fn conditional_damage_is_skipped_without_the_condition() {
        let mut state = attack_state(
            vec![gated("1d1", 4, DamageType::Piercing, None, false), gated("1d1", 9, DamageType::Piercing, Some(Condition::Grappled), false)],
            vec![],
            |_, _| {},
        );
        assert_eq!(bite_damage(&mut state), 5);
    }

    #[test]
    fn conditional_damage_applies_with_the_condition() {
        let mut state = attack_state(
            vec![gated("1d1", 4, DamageType::Piercing, None, false), gated("1d1", 9, DamageType::Piercing, Some(Condition::Grappled), false)],
            vec![],
            |_, fighter| fighter.conditions.push(condition_from(Condition::Grappled, "someone-else")),
        );
        assert_eq!(bite_damage(&mut state), 15);
    }

    #[test]
    fn conditional_damage_from_actor_needs_the_condition_applied_by_the_actor() {
        let damage = || vec![gated("1d1", 4, DamageType::Piercing, None, false), gated("1d1", 9, DamageType::Piercing, Some(Condition::Grappled), true)];
        let mut by_other = attack_state(damage(), vec![], |_, fighter| fighter.conditions.push(condition_from(Condition::Grappled, "ogre")));
        assert_eq!(bite_damage(&mut by_other), 5);
        let mut by_actor = attack_state(damage(), vec![], |_, fighter| fighter.conditions.push(condition_from(Condition::Grappled, "goblin")));
        assert_eq!(bite_damage(&mut by_actor), 15);
    }

    #[test]
    fn resistance_applies_to_each_damage_entry_separately() {
        let mut state = attack_state(
            vec![gated("1d1", 9, DamageType::Piercing, None, false), gated("1d1", 9, DamageType::Slashing, None, false)],
            vec![],
            |block, _| block.damage_resistances.push(DamageType::Slashing),
        );
        assert_eq!(bite_damage(&mut state), 10 + 5);
    }

    #[test]
    fn on_hit_damage_effect_resolves_after_the_attack() {
        let mut state = attack_state(
            vec![gated("1d1", 4, DamageType::Piercing, None, false)],
            vec![Effect::Damage(gated("1d1", 3, DamageType::Fire, None, false))],
            |_, _| {},
        );
        assert_eq!(bite_damage(&mut state), 5 + 4);
    }

    /// The goblin's "bite" is a saving throw against the fighter; returns the hit points the fighter lost.
    fn save_damage(dc: i32, half_on_success: bool, damage: Vec<Damage>, setup: impl FnOnce(&mut StatBlock, &mut Combatant)) -> i32 {
        let mut state = attack_state(vec![], vec![], setup);
        let save = crate::effect::SavingThrow {
            ability: Ability::Dex,
            dc,
            half_on_success,
            no_damage_on_success: false,
            repeat_save: None,
            two_stage: None,
            immunity_after_success: false,
            hp_gate: None,
            creature_type_gate: None,
            damage,
            on_failure: vec![],
        };
        let goblin = state.stat_blocks.iter_mut().find(|s| s.id == "goblin").unwrap();
        goblin.stat_block.actions[0].effects = vec![Effect::SavingThrow(save)];
        step(&mut state, StepRequest::Action { actor_id: "goblin".into(), action_id: "bite".into(), target_ids: vec!["fighter".into()] }).unwrap();
        100 - state.combatants.iter().find(|c| c.id == "fighter").unwrap().hp
    }

    #[test]
    fn conditional_damage_gates_failed_save_damage() {
        let damage = || vec![gated("1d1", 4, DamageType::Fire, None, false), gated("1d1", 9, DamageType::Fire, Some(Condition::Restrained), false)];
        assert_eq!(save_damage(1000, false, damage(), |_, _| {}), 5);
        let restrained = |_: &mut StatBlock, fighter: &mut Combatant| fighter.conditions.push(condition_from(Condition::Restrained, "goblin"));
        assert_eq!(save_damage(1000, false, damage(), restrained), 15);
    }

    #[test]
    fn resistance_applies_before_halving_on_a_successful_save() {
        let damage = vec![gated("1d1", 9, DamageType::Slashing, None, false)];
        assert_eq!(save_damage(-1000, true, damage.clone(), |_, _| {}), 5);
        assert_eq!(save_damage(-1000, true, damage, |block, _| block.damage_resistances.push(DamageType::Slashing)), 2);
    }

    #[test]
    fn conditional_effect_damage_skips_targets_without_the_condition() {
        let damage = || vec![gated("1d1", 4, DamageType::Piercing, None, false)];
        let on_hit = || vec![Effect::Damage(gated("1d1", 9, DamageType::Fire, Some(Condition::Prone), false))];
        assert_eq!(bite_damage(&mut attack_state(damage(), on_hit(), |_, _| {})), 5);
        let prone = |_: &mut StatBlock, fighter: &mut Combatant| fighter.conditions.push(condition_from(Condition::Prone, "goblin"));
        assert_eq!(bite_damage(&mut attack_state(damage(), on_hit(), prone)), 15);
    }

    #[test]
    fn conditional_damage_serializes_only_when_set() {
        let plain = serde_json::to_value(gated("1d6", 0, DamageType::Fire, None, false)).unwrap();
        assert!(plain.get("if_target").is_none() && plain.get("if_target_from_actor").is_none());
        let json = r#"{"dice":"1d6","type":"fire","if_target":"grappled","if_target_from_actor":true}"#;
        let parsed: Damage = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.if_target, Some(Condition::Grappled));
        assert!(parsed.if_target_from_actor);
    }
}
