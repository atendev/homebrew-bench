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
    PcDice, PendingPrompt, RangeBand,
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
    RollInitiative(InitiativeOptions),
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
    /// Every die behind this event came from a face the user entered.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub player_dice: bool,
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

/// Roll initiative and initialize the first turn. PC d20s go through the
/// player dice like every other PC roll, so `pc_dice = ask` prompts for them.
pub fn roll_initiative(
    state: &mut EncounterState,
    options: InitiativeOptions,
) -> Result<Vec<EngineEvent>, EngineError> {
    let mut stream = Stream::from_state(state.rng.clone());
    let mut rolls: HashMap<String, i32> = HashMap::new();
    let mut grouped_rolls: HashMap<String, (u32, bool)> = HashMap::new();
    let mut entered: HashMap<String, bool> = HashMap::new();
    let entrants = state
        .combatants
        .iter()
        .map(|c| Ok((c.id.clone(), c.stat_block_id.clone(), stat_block(state, c)?.initiative)))
        .collect::<Result<Vec<_>, EngineError>>()?;
    for (id, block_id, bonus) in entrants {
        let (d20, marked) = match grouped_rolls.get(&block_id) {
            Some(shared) if !options.individual => *shared,
            _ => {
                let first = stream.roll_die(20);
                let first = player_roll(state, &id, 20, first);
                let d20 = if options.surprised.contains(&id) {
                    let second = stream.roll_die(20);
                    first.min(player_roll_in(state, &id, 20, second, false))
                } else {
                    first
                };
                let shared = (d20, state.player_dice.all_entered_since(&id));
                grouped_rolls.insert(block_id, shared);
                shared
            }
        };
        entered.insert(id.clone(), marked);
        rolls.insert(id, d20 as i32 + bonus);
    }
    state.rng = stream.state();
    state.initiative_order = initiative_order(state, &rolls, !options.individual);
    state.initiative_rolls = rolls.clone();
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
            player_dice: entered[id],
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
    let events = step_events(state, request)?;
    Ok(response(state, events))
}

/// Like [`step`] but skips the state digest. Batch runs call this thousands of times.
pub fn step_events(state: &mut EncounterState, request: StepRequest) -> Result<Vec<EngineEvent>, EngineError> {
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
        StepRequest::RollInitiative(options) => roll_initiative(state, options)?,
    };
    Ok(events)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionSection {
    Action,
    Bonus,
    Reaction,
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
    let (action, section) = {
        let block = stat_block(state, &state.combatants[actor_index])?;
        block.actions.iter().find(|a| a.id == action_id).map(|a| (a.clone(), ActionSection::Action))
            .or_else(|| block.bonus_actions.iter().find(|a| a.id == action_id).map(|a| (a.clone(), ActionSection::Bonus)))
            .or_else(|| block.reactions.iter().find(|a| a.id == action_id).map(|a| (a.clone(), ActionSection::Reaction)))
            .ok_or_else(|| EngineError::NotFound(format!("action {action_id} not found")))?
    };
    if section != ActionSection::Reaction {
        if let Some(active) = &state.active_turn {
            if active != actor_id {
                return Err(EngineError::Illegal(format!("{actor_id} is not the active combatant")));
            }
        }
    }
    {
        let economy = &state.combatants[actor_index].action_economy;
        let (used, label) = match section {
            ActionSection::Action => (economy.action_used, "action"),
            ActionSection::Bonus => (economy.bonus_action_used, "bonus action"),
            ActionSection::Reaction => (economy.reaction_used, "reaction"),
        };
        if used {
            return Err(EngineError::Illegal(format!("{label} already used")));
        }
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
    let economy = &mut state.combatants[actor_index].action_economy;
    match section {
        ActionSection::Action => economy.action_used = true,
        ActionSection::Bonus => economy.bonus_action_used = true,
        ActionSection::Reaction => economy.reaction_used = true,
    }
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
                let retargeted = retarget_if_down(state, actor_id, target_id);
                let target_id = &retargeted;
                let target_index = index_of(state, target_id)?;
                let target = state.combatants[target_index].clone();
                let actor = state.combatants[index_of(state, actor_id)?].clone();
                let advantage = disposition(state, &actor, &target, RollDisposition::Normal);
                let d20a = stream.roll_die(20);
                let d20a = player_roll(state, actor_id, 20, d20a);
                let d20b = stream.roll_die(20);
                let d20b = if matches!(advantage, RollDisposition::Normal) { d20b } else { player_roll_in(state, actor_id, 20, d20b, false) };
                let natural = match advantage {
                    RollDisposition::Advantage => d20a.max(d20b),
                    RollDisposition::Disadvantage => d20a.min(d20b),
                    RollDisposition::Normal => d20a,
                };
                let total = natural as i32 + attack.modifier + attack_modifier(state, &actor)?;
                let ac = stat_block(state, &target)?.armor_class.first().map(|a| a.value).unwrap_or(10) + ac_modifier(state, &target);
                let critical = natural == 20;
                let hit = critical || (natural != 1 && total >= ac);
                let marked = state.player_dice.all_entered_since(actor_id);
                events.push(EngineEvent {
                    kind: "attack".into(),
                    message: format!("{} attacks {}: {} vs AC {} ({})", actor.name, target.name, total, ac, if hit { "hit" } else { "miss" }),
                    actor_id: Some(actor_id.into()),
                    target_id: Some(target_id.clone()),
                    roll: Some(natural),
                    total: Some(total),
                    damage: None,
                    player_dice: marked,
                });
                if hit {
                    let mut damage = 0;
                    for die in attack.damage.iter().filter(|d| damage_applies(&target, actor_id, d)) {
                        let mut amount = roll_dice(state, stream, actor_id, &die.dice)? as i32 + die.modifier;
                        if critical {
                            amount += roll_dice(state, stream, actor_id, &die.dice)? as i32;
                        }
                        damage += adjust_damage(state, &target, amount, die.damage_type);
                    }
                    let marked = state.player_dice.all_entered_since(actor_id);
                    apply_damage(state, stream, &target.id, damage, Some(actor_id), marked, critical, events)?;
                    for effect in &attack.on_hit {
                        resolve_effect(state, stream, actor_id, effect.clone(), &[target_id.clone()], events)?;
                    }
                }
            }
        }
        Effect::Damage(damage) => {
            let amount = roll_dice(state, stream, actor_id, &damage.dice)? as i32 + damage.modifier;
            let marked = state.player_dice.all_entered_since(actor_id);
            for target_id in target_ids {
                let target = state.combatants[index_of(state, target_id)?].clone();
                if !damage_applies(&target, actor_id, &damage) {
                    continue;
                }
                let amount = adjust_damage(state, &target, amount, damage.damage_type);
                apply_damage(state, stream, target_id, amount, Some(actor_id), marked, false, events)?;
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
                        ends_on_damage: condition.ends_on_damage,
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
            let amount = roll_dice(state, stream, actor_id, &heal.dice)? as i32 + heal.modifier;
            let marked = state.player_dice.all_entered_since(actor_id);
            for target_id in target_ids {
                let i = index_of(state, target_id)?;
                state.combatants[i].hp = (state.combatants[i].hp.max(0) + amount).min(state.combatants[i].max_hp);
                // Restore from unconscious if healed above 0
                if state.combatants[i].hp > 0 && state.combatants[i].end_state == EndState::Unconscious {
                    state.combatants[i].end_state = EndState::Active;
                    state.combatants[i].death_saves = DeathSaves::default();
                    events.push(EngineEvent { player_dice: marked, ..event("revive", format!("{target_id} regains consciousness"), actor_id, target_id) });
                }
                events.push(EngineEvent { player_dice: marked, ..event("heal", format!("{target_id} regains {amount} HP"), actor_id, target_id) });
            }
        }
        Effect::TempHp(temp) => {
            let amount = roll_dice(state, stream, actor_id, &temp.dice)? as i32 + temp.modifier;
            let marked = state.player_dice.all_entered_since(actor_id);
            for target_id in if target_ids.is_empty() {
                vec![actor_id.to_string()]
            } else {
                target_ids.to_vec()
            } {
                let i = index_of(state, &target_id)?;
                state.combatants[i].temp_hp = state.combatants[i].temp_hp.max(amount);
                let message = format!("{target_id} gains {amount} temporary HP");
                events.push(EngineEvent { player_dice: marked, ..event("temp_hp", message, actor_id, &target_id) });
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
            cast_spell(state, stream, actor_id, &cast, target_ids, events)?;
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
            let count = area.target_count.map_or(target_ids.len(), |n| n.max(1) as usize);
            let hit: Vec<String> = target_ids.iter().take(count).cloned().collect();
            resolve_effect(state, stream, actor_id, *area.effect, &hit, events)?;
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
    let roll = player_roll(state, target_id, 20, roll);
    let success = roll as i32 + modifier >= save.dc;
    let marked = state.player_dice.all_entered_since(target_id);
    events.push(EngineEvent {
        kind: "save".into(),
        message: format!("{target_id} {} the save", if success { "passes" } else { "fails" }),
        actor_id: Some(actor_id.into()),
        target_id: Some(target_id.into()),
        roll: Some(roll),
        total: Some(roll as i32 + modifier),
        damage: None,
        player_dice: marked,
    });
    if !success {
        for effect in &save.on_failure {
            resolve_effect(state, stream, actor_id, effect.clone(), &[target_id.to_string()], events)?;
        }
        for damage in save.damage.iter().filter(|d| damage_applies(&target, actor_id, d)) {
            let amount = roll_dice(state, stream, actor_id, &damage.dice)? as i32 + damage.modifier;
            let amount = adjust_damage(state, &target, amount, damage.damage_type);
            let marked = state.player_dice.all_entered_since(actor_id);
            apply_damage(state, stream, target_id, amount, Some(actor_id), marked, false, events)?;
        }
    } else if save.no_damage_on_success {
        // no damage on success
    } else if save.half_on_success {
        for damage in save.damage.iter().filter(|d| damage_applies(&target, actor_id, d)) {
            let rolled = roll_dice(state, stream, actor_id, &damage.dice)? as i32 + damage.modifier;
            let amount = adjust_damage(state, &target, rolled, damage.damage_type) / 2;
            let marked = state.player_dice.all_entered_since(actor_id);
            apply_damage(state, stream, target_id, amount, Some(actor_id), marked, false, events)?;
        }
    } else {
        for damage in save.damage.iter().filter(|d| damage_applies(&target, actor_id, d)) {
            let amount = roll_dice(state, stream, actor_id, &damage.dice)? as i32 + damage.modifier;
            let amount = adjust_damage(state, &target, amount, damage.damage_type);
            let marked = state.player_dice.all_entered_since(actor_id);
            apply_damage(state, stream, target_id, amount, Some(actor_id), marked, false, events)?;
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
                let chosen: Vec<usize> = match state.player_dice.picks.pop_front() {
                    Some(indexes) => indexes.into_iter().filter(|i| *i < options.len()).take(pick as usize).collect(),
                    None => (0..pick as usize).collect(),
                };
                for opt in chosen.into_iter().map(|i| options[i].clone()) {
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
                let index = state.player_dice.picks.pop_front().and_then(|v| v.first().copied()).unwrap_or(0);
                let Some(set) = sets.into_iter().nth(index) else { continue };
                events.push(multiattack_choice_event(actor_id, &format!("alternative {}", index + 1)));
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
        player_dice: false,
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

fn cast_spell(
    state: &mut EncounterState,
    stream: &mut Stream,
    actor_id: &str,
    cast: &crate::effect::Cast,
    target_ids: &[String],
    events: &mut Vec<EngineEvent>,
) -> Result<(), EngineError> {
    let spell_id = cast.spell_id.as_str();
    // In phase 1, spell effects are resolved from the spell effects file.
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
    let i = index_of(state, actor_id)?;
    let ctx = {
        let block = stat_block(state, &state.combatants[i])?;
        spend_slot(&state.combatants[i], block, cast)?;
        cast_context(block, cast)
    };
    if let Some(level) = cast.slot_level.filter(|l| *l > 0) {
        if stat_block(state, &state.combatants[i])?.spell_slots.is_some() {
            *state.combatants[i].resources.entry(format!("{actor_id}:slot:{level}")).or_insert(0) += 1;
        }
    }
    if spell.concentration {
        if state.combatants[i].concentration.is_some() {
            break_concentration(state, actor_id, events)?;
        }
        state.combatants[i].concentration = Some(spell_id.into());
        events.push(event("concentration", format!("{} begins concentrating on {}", actor_id, spell_id), actor_id, actor_id));
    }
    let cantrip = cast.slot_level.unwrap_or(0) == 0;
    for effect in spell.effects {
        let effect = match &ctx {
            Some(ctx) => bind_effect(effect, ctx, cantrip),
            None => effect,
        };
        resolve_effect(state, stream, actor_id, effect, target_ids, events)?;
    }
    Ok(())
}

/// A PC with spell slots can cast a leveled spell only while a slot of that level is left.
fn spend_slot(actor: &Combatant, block: &StatBlock, cast: &crate::effect::Cast) -> Result<(), EngineError> {
    let (Some(slots), Some(level)) = (&block.spell_slots, cast.slot_level.filter(|l| *l > 0)) else {
        return Ok(());
    };
    let max = slots.get(usize::from(level) - 1).copied().unwrap_or(0) as i32;
    let used = actor.resources.get(&format!("{}:slot:{level}", actor.id)).copied().unwrap_or(0);
    if used >= max {
        return Err(EngineError::Illegal(format!("{} has no level {level} spell slot left", actor.id)));
    }
    Ok(())
}

fn load_spell_effects() -> Result<&'static HashMap<String, crate::spell::SpellEffect>, EngineError> {
    static CACHE: std::sync::OnceLock<Result<HashMap<String, crate::spell::SpellEffect>, String>> = std::sync::OnceLock::new();
    CACHE
        .get_or_init(|| {
            let value: serde_json::Value = serde_json::from_str(crate::embedded::spell_effects_json()).map_err(|e| format!("invalid spell effects json: {e}"))?;
            let mut map = HashMap::new();
            if let Some(obj) = value.as_object() {
                for (key, entry_value) in obj {
                    let spell: crate::spell::SpellEffect = serde_json::from_value(entry_value.clone())
                        .map_err(|e| format!("spell {key} invalid: {e}"))?;
                    map.insert(key.clone(), spell);
                }
            }
            Ok(map)
        })
        .as_ref()
        .map_err(|e| EngineError::Invalid(e.clone()))
}

/// Batch fights let the remaining attacks of a multiattack move to a living opponent once the
/// chosen target is down. Live mode leaves the choice to the caller.
fn retarget_if_down(state: &EncounterState, actor_id: &str, target_id: &str) -> String {
    if !state.player_dice.retarget_downed {
        return target_id.to_string();
    }
    let down = |c: &Combatant| c.end_state != EndState::Active;
    let Ok(t) = index_of(state, target_id) else { return target_id.to_string() };
    if !down(&state.combatants[t]) {
        return target_id.to_string();
    }
    let Some(actor_side) = state.combatants.iter().find(|c| c.id == actor_id).map(|c| c.side_id.clone()) else {
        return target_id.to_string();
    };
    state
        .combatants
        .iter()
        .find(|c| c.side_id != actor_side && !down(c))
        .map_or_else(|| target_id.to_string(), |c| c.id.clone())
}

/// Spell ids a stat block casts that the spell effects file does not hold. A cast
/// of one is logged but does nothing, so a batch names each one.
pub fn missing_spells(block: &StatBlock) -> Vec<String> {
    fn walk(value: &serde_json::Value, known: &HashMap<String, crate::spell::SpellEffect>, out: &mut std::collections::BTreeSet<String>) {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(id) = map.get("spell_id").and_then(serde_json::Value::as_str) {
                    if !known.contains_key(id) {
                        out.insert(id.to_string());
                    }
                }
                map.values().for_each(|v| walk(v, known, out));
            }
            serde_json::Value::Array(items) => items.iter().for_each(|v| walk(v, known, out)),
            _ => {}
        }
    }
    let mut out = std::collections::BTreeSet::new();
    if let (Ok(known), Ok(value)) = (load_spell_effects(), serde_json::to_value(block)) {
        walk(&value, known, &mut out);
    }
    out.into_iter().collect()
}

/// Effects of a cast with the caster's numbers bound in, and whether the spell needs concentration.
pub fn bound_cast_effects(block: &StatBlock, cast: &crate::effect::Cast) -> Option<(Vec<Effect>, bool)> {
    let spell = load_spell_effects().ok()?.get(&cast.spell_id)?;
    let cantrip = cast.slot_level.unwrap_or(0) == 0;
    let effects = match cast_context(block, cast) {
        Some(ctx) => spell.effects.iter().cloned().map(|e| bind_effect(e, &ctx, cantrip)).collect(),
        None => spell.effects.clone(),
    };
    Some((effects, spell.concentration))
}

/// Whether the ability can be used right now: its usage allows it and any spell slot is left.
pub fn ability_ready(state: &EncounterState, actor_id: &str, ability: &crate::stat_block::AbilityEntry) -> bool {
    let Ok(i) = index_of(state, actor_id) else { return false };
    let actor = &state.combatants[i];
    let Ok(block) = stat_block(state, actor) else { return false };
    let key = |id: &str| format!("{actor_id}:{id}");
    let ready = match &ability.usage {
        None | Some(Usage::AtWill) => true,
        Some(Usage::Recharge { group, .. }) => {
            actor.resources.get(&key(group.as_deref().unwrap_or(&ability.id))).copied().unwrap_or(1) > 0
        }
        Some(Usage::PerDay { uses }) | Some(Usage::PerRest { uses, .. }) => {
            actor.resources.get(&key(&ability.id)).copied().unwrap_or(0) < *uses as i32
        }
        Some(Usage::CooldownUntilNextTurn) => actor.resources.get(&key(&ability.id)).copied().unwrap_or(1) > 0,
    };
    ready
        && ability.effects.iter().all(|e| match e {
            Effect::Cast(c) => spend_slot(actor, block, c).is_ok(),
            _ => true,
        })
}

/// Spellcasting numbers a caster's stat block lends to the spells it casts.
struct CastContext {
    save_dc: i32,
    attack_bonus: i32,
    heal_bonus: i32,
    dice_multiplier: u32,
}

fn cast_context(block: &StatBlock, cast: &crate::effect::Cast) -> Option<CastContext> {
    let sc = block.spellcasting.as_ref()?;
    let is_cantrip = cast.slot_level.unwrap_or(0) == 0;
    let dice_multiplier = match (is_cantrip, block.level) {
        (true, Some(l)) => 1 + u32::from(l >= 5) + u32::from(l >= 11) + u32::from(l >= 17),
        _ => 1,
    };
    Some(CastContext {
        save_dc: sc.save_dc,
        attack_bonus: sc.attack_bonus,
        heal_bonus: block.ability_scores.modifier(sc.ability),
        dice_multiplier,
    })
}

fn scale_dice(expr: &str, multiplier: u32) -> String {
    if multiplier <= 1 {
        return expr.to_string();
    }
    match expr.split_once('d').and_then(|(n, rest)| n.parse::<u32>().ok().map(|n| (n, rest))) {
        Some((n, rest)) => format!("{}d{rest}", n * multiplier),
        None => expr.to_string(),
    }
}

fn bind_effect(effect: Effect, ctx: &CastContext, cantrip_damage: bool) -> Effect {
    let scale = |damage: &mut Vec<crate::effect::Damage>| {
        if cantrip_damage {
            for d in damage.iter_mut() {
                d.dice = scale_dice(&d.dice, ctx.dice_multiplier);
            }
        }
    };
    match effect {
        Effect::Attack(mut a) => {
            a.modifier += ctx.attack_bonus;
            scale(&mut a.damage);
            Effect::Attack(a)
        }
        Effect::SavingThrow(mut st) => {
            if st.dc == 0 {
                st.dc = ctx.save_dc;
            }
            scale(&mut st.damage);
            Effect::SavingThrow(st)
        }
        Effect::Heal(mut h) => {
            h.modifier += ctx.heal_bonus;
            Effect::Heal(h)
        }
        Effect::Area(mut area) => {
            area.effect = Box::new(bind_effect(*area.effect, ctx, cantrip_damage));
            Effect::Area(area)
        }
        other => other,
    }
}

fn move_combatant(
    state: &mut EncounterState,
    actor_id: &str,
    target_id: Option<&str>,
    band: RangeBand,
    feet: u32,
) -> Result<Vec<EngineEvent>, EngineError> {
    let i = index_of(state, actor_id)?;
    // (combatant id, whether it prices the move and may react to it)
    let others: Vec<(String, bool)> = match target_id {
        Some(id) => vec![(id.to_string(), true)],
        None if band == RangeBand::Engaged => {
            return Err(EngineError::Illegal("an untargeted move cannot engage; pass --target".into()));
        }
        None => state
            .combatants
            .iter()
            .filter(|c| c.id != actor_id)
            .map(|c| (c.id.clone(), c.end_state == EndState::Active))
            .collect(),
    };
    let cost = others
        .iter()
        .filter(|(_, counts)| *counts)
        .map(|(id, _)| band_change_cost(state.combatants[i].ranges.get(id).copied().unwrap_or(RangeBand::Engaged), band))
        .max()
        .unwrap_or(0);
    if feet < cost {
        return Err(EngineError::Illegal(format!("movement requires {cost} feet")));
    }
    state.combatants[i].action_economy.movement_used_feet += cost;
    let mut events = vec![event("movement", format!("{actor_id} moves to {}", format!("{band:?}").to_lowercase()), actor_id, target_id.unwrap_or(actor_id))];
    for (other, may_react) in &others {
        set_band(state, i, actor_id, other, band, *may_react, &mut events)?;
    }
    Ok(events)
}

fn band_change_cost(current: RangeBand, band: RangeBand) -> u32 {
    match (current, band) {
        (RangeBand::Engaged, RangeBand::Engaged)
        | (RangeBand::Near, RangeBand::Near)
        | (RangeBand::Far, RangeBand::Far) => 0,
        (RangeBand::Engaged, RangeBand::Near) | (RangeBand::Near, RangeBand::Engaged) => 5,
        (RangeBand::Near, RangeBand::Far) | (RangeBand::Far, RangeBand::Near) => 30,
        (RangeBand::Engaged, RangeBand::Far) | (RangeBand::Far, RangeBand::Engaged) => 35,
    }
}

fn set_band(
    state: &mut EncounterState,
    i: usize,
    actor_id: &str,
    target_id: &str,
    band: RangeBand,
    may_react: bool,
    events: &mut Vec<EngineEvent>,
) -> Result<(), EngineError> {
    let was_engaged = state.combatants[i].engagement.contains(&target_id.to_string());
    state.combatants[i].ranges.insert(target_id.into(), band);
    if let Ok(j) = index_of(state, target_id) {
        state.combatants[j].ranges.insert(actor_id.into(), band);
    }
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
        if was_engaged && may_react {
            if let Ok(j) = index_of(state, target_id) {
                if state.combatants[j].action_economy.reaction_used {
                    events.push(event("opportunity_attack", format!("{target_id} has no reaction available"), target_id, actor_id));
                } else {
                    events.push(event("opportunity_attack", format!("{target_id} may use an Opportunity Attack on {actor_id}"), target_id, actor_id));
                }
            }
        }
    }
    Ok(())
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
    if !stat_block(state, &state.combatants[i])?.dying {
        return Ok(());
    }
    let mut stream = Stream::from_state(state.rng.clone());
    let roll = stream.roll_die(20);
    state.rng = stream.state();
    let roll = player_roll(state, combatant_id, 20, roll);
    let marked = state.player_dice.all_entered_since(combatant_id);
    let message = if roll == 20 {
        state.combatants[i].hp = 1;
        state.combatants[i].end_state = EndState::Active;
        state.combatants[i].death_saves = DeathSaves::default();
        format!("{combatant_id} rolls a natural 20 and regains consciousness")
    } else if roll == 1 {
        state.combatants[i].death_saves.failures += 2;
        format!("{combatant_id} rolls a natural 1: two failures")
    } else if roll >= 10 {
        state.combatants[i].death_saves.successes += 1;
        format!("{combatant_id} succeeds a death save")
    } else {
        state.combatants[i].death_saves.failures += 1;
        format!("{combatant_id} fails a death save")
    };
    events.push(EngineEvent { roll: Some(roll), player_dice: marked, ..event("death_save", message, combatant_id, combatant_id) });
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
            let roll = player_roll(state, combatant_id, 20, roll);
            let dc = 10;
            let success = roll as i32 + modifier >= dc;
            let marked = state.player_dice.all_entered_since(combatant_id);
            events.push(EngineEvent {
                kind: "repeat_save".into(),
                message: format!("{combatant_id} {} repeat save DC {dc}", if success { "passes" } else { "fails" }),
                actor_id: Some(combatant_id.into()),
                target_id: Some(combatant_id.into()),
                roll: Some(roll),
                total: Some(roll as i32 + modifier),
                damage: None,
                player_dice: marked,
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
    let Ok(block) = stat_block(state, &state.combatants[i]) else { return };
    let keys: Vec<String> = block
        .actions
        .iter()
        .chain(&block.bonus_actions)
        .chain(&block.reactions)
        .chain(&block.legendary_actions)
        .filter(|a| matches!(a.usage, Some(Usage::CooldownUntilNextTurn)))
        .map(|a| format!("{combatant_id}:{}", a.id))
        .collect();
    for key in keys {
        state.combatants[i].resources.insert(key, 1);
    }
    state.combatants[i].resources.remove(&format!("{combatant_id}:legendary"));
}

/// Spend one legendary action between turns. The engine tracks the per-round budget; the
/// ability list has no cost field, so every legendary action costs one use.
pub fn use_legendary_action(
    state: &mut EncounterState,
    actor_id: &str,
    action_id: &str,
    target_ids: &[String],
) -> Result<Vec<EngineEvent>, EngineError> {
    let i = index_of(state, actor_id)?;
    if !can_act(&state.combatants[i]) {
        return Err(EngineError::Illegal(format!("{actor_id} cannot act")));
    }
    let (action, budget) = {
        let block = stat_block(state, &state.combatants[i])?;
        let action = block.legendary_actions.iter().find(|a| a.id == action_id).cloned()
            .ok_or_else(|| EngineError::NotFound(format!("legendary action {action_id} not found")))?;
        (action, block.legendary_uses_per_round as i32)
    };
    let key = format!("{actor_id}:legendary");
    let used = state.combatants[i].resources.get(&key).copied().unwrap_or(0);
    if used >= budget {
        return Err(EngineError::Illegal(format!("{actor_id} has no legendary actions left this round")));
    }
    if let Some(Usage::Recharge { group, .. }) = &action.usage {
        let rkey = format!("{actor_id}:{}", group.as_deref().unwrap_or(action_id));
        if state.combatants[i].resources.get(&rkey).copied().unwrap_or(1) <= 0 {
            return Err(EngineError::Illegal(format!("{action_id} is not recharged")));
        }
        state.combatants[i].resources.insert(rkey, 0);
    }
    state.combatants[i].resources.insert(key, used + 1);
    let mut stream = Stream::from_state(state.rng.clone());
    let mut events = vec![event("legendary", format!("{actor_id} uses legendary action {}", action.name), actor_id, actor_id)];
    for effect in action.effects {
        resolve_effect(state, &mut stream, actor_id, effect, target_ids, &mut events)?;
    }
    state.rng = stream.state();
    Ok(events)
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
    stream: &mut Stream,
    target_id: &str,
    amount: i32,
    source_id: Option<&str>,
    marked: bool,
    critical: bool,
    events: &mut Vec<EngineEvent>,
) -> Result<(), EngineError> {
    let i = index_of(state, target_id)?;
    if state.combatants[i].end_state == EndState::Unconscious && amount > 0 {
        let failures = if critical { 2 } else { 1 };
        state.combatants[i].death_saves.failures += failures;
        events.push(EngineEvent {
            kind: "damage".into(),
            message: format!("{target_id} takes {amount} damage while down"),
            actor_id: source_id.map(|s| s.into()),
            target_id: Some(target_id.into()),
            roll: None,
            total: None,
            damage: Some(amount),
            player_dice: marked,
        });
        events.push(event("death_save", format!("{target_id} suffers {failures} death save failure(s) from the hit"), source_id.unwrap_or(target_id), target_id));
        if state.combatants[i].death_saves.failures >= 3 {
            state.combatants[i].end_state = EndState::Dead;
            events.push(event("dies", format!("{target_id} dies"), target_id, target_id));
        }
        return Ok(());
    }
    if amount > 0 {
        let before = state.combatants[i].conditions.len();
        state.combatants[i].conditions.retain(|c| !c.ends_on_damage);
        if state.combatants[i].conditions.len() < before {
            events.push(event("condition_expire", format!("{target_id} is roused by the damage"), target_id, target_id));
        }
    }
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
            player_dice: marked,
        });
    }
    if remaining > 0 && is_concentrating(&state.combatants[i]) {
        // Constitution save DC = max(10, damage / 2) to maintain concentration
        let dc = (remaining / 2).max(10);
        let block = stat_block(state, &state.combatants[i])?;
        let roll = stream.roll_die(20);
        let modifier = block.saving_throws.get(&Ability::Con)
            .copied()
            .unwrap_or_else(|| block.ability_scores.modifier(Ability::Con));
        let roll = player_roll(state, target_id, 20, roll);
        let success = roll as i32 + modifier >= dc;
        let marked = state.player_dice.all_entered_since(target_id);
        events.push(EngineEvent {
            kind: "concentration_save".into(),
            message: format!("{target_id} {} concentration save DC {dc}", if success { "passes" } else { "fails" }),
            actor_id: source_id.map(|s| s.into()),
            target_id: Some(target_id.into()),
            roll: Some(roll),
            total: Some(roll as i32 + modifier),
            damage: None,
            player_dice: marked,
        });
        if !success {
            break_concentration(state, target_id, events)?;
        }
    }
    if state.combatants[i].hp <= 0 && state.combatants[i].end_state == EndState::Active {
        let dying = stat_block(state, &state.combatants[i])?.dying;
        if dying && -state.combatants[i].hp >= state.combatants[i].max_hp {
            state.combatants[i].end_state = EndState::Dead;
            events.push(event("dies", format!("{target_id} dies from massive damage"), target_id, target_id));
        } else if dying {
            state.combatants[i].hp = 0;
            state.combatants[i].end_state = EndState::Unconscious;
            state.combatants[i].death_saves = DeathSaves::default();
            events.push(event("unconscious", format!("{target_id} falls unconscious"), target_id, target_id));
        } else {
            state.combatants[i].end_state = EndState::Dead;
            events.push(event("dies", format!("{target_id} dies"), target_id, target_id));
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
        let held = |d: &Duration, source: &Option<String>| matches!(d, Duration::Concentration) && source.as_deref() == Some(target_id);
        for c in &mut state.combatants {
            c.conditions.retain(|x| !held(&x.duration, &x.source));
        }
        state.active_modifiers.retain(|m| !held(&m.remaining, &m.source));
    }
    Ok(())
}

/// Draw a die for `owner`, then swap in a user-entered face when the owner is
/// a PC and the encounter asks for player dice. The draw always happens, so
/// the stream position never depends on the dice mode (ADR 0007).
fn player_roll(state: &mut EncounterState, owner: &str, sides: u32, drawn: u32) -> u32 {
    player_roll_in(state, owner, sides, drawn, true)
}

fn player_roll_in(state: &mut EncounterState, owner: &str, sides: u32, drawn: u32, group_start: bool) -> u32 {
    if state.settings.pc_dice != PcDice::Ask {
        return drawn;
    }
    let is_pc = index_of(state, owner)
        .ok()
        .and_then(|i| stat_block(state, &state.combatants[i]).ok())
        .is_some_and(|b| b.class.is_some());
    if !is_pc {
        return drawn;
    }
    state.player_dice.take(owner, sides, drawn, group_start)
}

fn roll_dice(state: &mut EncounterState, stream: &mut Stream, owner: &str, expression: &str) -> Result<u32, EngineError> {
    let (count, sides) = expression.split_once('d').ok_or_else(|| EngineError::Invalid(format!("invalid dice expression {expression}")))?;
    let count: u32 = count.parse().map_err(|_| EngineError::Invalid(format!("invalid dice expression {expression}")))?;
    let sides: u32 = sides.parse().map_err(|_| EngineError::Invalid(format!("invalid dice expression {expression}")))?;
    if count == 0 || sides == 0 { return Err(EngineError::Invalid(format!("invalid dice expression {expression}"))); }
    let mut total = 0;
    for i in 0..count {
        let drawn = stream.roll_die(sides);
        total += player_roll_in(state, owner, sides, drawn, i == 0);
    }
    Ok(total)
}

fn can_act(c: &Combatant) -> bool {
    c.end_state == EndState::Active
        && !c.conditions.iter().any(|x| matches!(x.condition,
            Condition::Incapacitated | Condition::Paralyzed | Condition::Petrified | Condition::Stunned | Condition::Unconscious))
}
fn index_of(state: &EncounterState, id: &str) -> Result<usize, EngineError> { state.combatants.iter().position(|c| c.id == id).ok_or_else(|| EngineError::NotFound(format!("combatant {id} not found"))) }
fn stat_block<'a>(state: &'a EncounterState, c: &Combatant) -> Result<&'a StatBlock, EngineError> { state.stat_blocks.iter().find(|s| s.id == c.stat_block_id).map(|s| &s.stat_block).ok_or_else(|| EngineError::NotFound(format!("stat block {} not found", c.stat_block_id))) }
fn event(kind: &str, message: String, actor_id: &str, target_id: &str) -> EngineEvent { EngineEvent { kind: kind.into(), message, actor_id: Some(actor_id.into()), target_id: Some(target_id.into()), roll: None, total: None, damage: None, player_dice: false } }
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
            initiative_rolls: HashMap::new(),
            settings: Default::default(),
            ended: None,
            initial_snapshot: None,
            log: vec![],
            tamper_hash: String::new(),
            player_dice: Default::default(),
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
        assert!(roll_dice(&mut test_encounter(vec![], vec![]), &mut Stream::from_seed(1), "x", "d6").is_err());
        assert!(roll_dice(&mut test_encounter(vec![], vec![]), &mut Stream::from_seed(1), "x", "1d0").is_err());
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
        fighter_block.dying = true;
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
            ends_on_damage: false,
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
        let opts = InitiativeOptions { individual: false, surprised: vec![] };
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

    #[test]
    fn entered_damage_dice_mark_every_target_of_one_damage_roll() {
        let mut caster = test_stat_block("caster", "Caster", 20, 12);
        caster.class = Some("wizard".into());
        caster.actions.push(crate::stat_block::AbilityEntry {
            id: "burst".into(),
            name: "Burst".into(),
            desc: None,
            kind: crate::stat_block::AbilityKind::Modeled,
            effects: vec![Effect::Damage(gated("1d6", 0, DamageType::Fire, None, false))],
            agent_clauses: vec![],
            default_resolution: None,
            usage: None,
        });
        let combatants = vec![
            test_combatant("caster", "caster", 20, "ally"),
            test_combatant("goblin_a", "goblin", 7, "enemy"),
            test_combatant("goblin_b", "goblin", 7, "enemy"),
        ];
        let mut state = test_encounter(vec![caster, test_stat_block("goblin", "Goblin", 7, 15)], combatants);
        state.settings.pc_dice = PcDice::Ask;
        state.player_dice.faces = vec![4].into();
        let targets = vec!["goblin_a".to_string(), "goblin_b".to_string()];
        let resp = step(&mut state, StepRequest::Action { actor_id: "caster".into(), action_id: "burst".into(), target_ids: targets }).unwrap();
        let damage: Vec<_> = resp.events.iter().filter(|e| e.kind == "damage").collect();
        assert_eq!(damage.len(), 2);
        assert!(damage.iter().all(|e| e.player_dice && e.damage == Some(4)), "{damage:?}");
    }

    #[test]
    fn concentration_save_draws_after_the_action_on_the_same_stream() {
        let damage = vec![gated("1d6", 0, DamageType::Piercing, None, false)];
        let mut state = attack_state(damage, vec![], |_, fighter| fighter.concentration = Some("bless".into()));
        let mut expected = Stream::from_state(state.rng.clone());
        let first = expected.roll_die(20);
        expected.roll_die(20);
        expected.roll_die(6);
        if first == 20 {
            expected.roll_die(6);
        }
        let save_roll = expected.roll_die(20);
        let resp = step(&mut state, StepRequest::Action { actor_id: "goblin".into(), action_id: "bite".into(), target_ids: vec!["fighter".into()] }).unwrap();
        let save = resp.events.iter().find(|e| e.kind == "concentration_save").expect("concentration save");
        assert_eq!(save.roll, Some(save_roll));
        assert_eq!(state.rng, expected.state());
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
        ConditionInstance { condition, duration: Duration::Permanent, source: Some(source.into()), ends_on_damage: false }
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

    fn fx(value: serde_json::Value) -> Effect {
        serde_json::from_value(value).unwrap()
    }

    fn ability(id: &str, effects: Vec<Effect>, usage: Option<Usage>) -> crate::stat_block::AbilityEntry {
        crate::stat_block::AbilityEntry {
            id: id.into(),
            name: id.into(),
            desc: None,
            kind: crate::stat_block::AbilityKind::Modeled,
            effects,
            agent_clauses: vec![],
            default_resolution: None,
            usage,
        }
    }

    fn cast(spell_id: &str, slot_level: Option<u8>) -> crate::effect::Cast {
        crate::effect::Cast { spell_id: spell_id.into(), slot_level, upcast: false }
    }

    fn caster_block(level: u8) -> StatBlock {
        let mut block = test_stat_block("mage", "Mage", 30, 12);
        block.class = Some("caster".into());
        block.dying = true;
        block.level = Some(level);
        block.ability_scores.wisdom = 16;
        block.spellcasting = Some(crate::stat_block::Spellcasting { ability: Ability::Wis, save_dc: 15, attack_bonus: 7 });
        block
    }

    fn pc_block(id: &str, hp: i32) -> StatBlock {
        let mut block = test_stat_block(id, id, hp, 10);
        block.class = Some("fighter".into());
        block.dying = true;
        block
    }

    fn smash(modifier: i32, dice: &str, bonus: i32) -> crate::stat_block::AbilityEntry {
        ability(
            "smash",
            vec![fx(serde_json::json!({"attack": {"modifier": modifier, "reach": 5, "target_count": 1,
                "damage": [{"dice": dice, "type": "bludgeoning", "modifier": bonus}]}}))],
            None,
        )
    }

    /// A brute that attacks a PC; the PC starts in the given state.
    fn brute_vs_pc(pc_hp: i32, attack: crate::stat_block::AbilityEntry, setup: impl FnOnce(&mut Combatant)) -> EncounterState {
        let mut brute = test_stat_block("brute", "Brute", 50, 10);
        brute.actions.push(attack);
        let mut pc = test_combatant("pc", "pc", pc_hp, "ally");
        setup(&mut pc);
        let mut state = test_encounter(vec![pc_block("pc", pc_hp), brute], vec![pc, test_combatant("brute", "brute", 50, "enemy")]);
        state.active_turn = Some("brute".into());
        state
    }

    fn act(state: &mut EncounterState, actor: &str, action: &str, targets: &[&str]) -> Result<Vec<EngineEvent>, EngineError> {
        let target_ids = targets.iter().map(|t| t.to_string()).collect();
        step_events(state, StepRequest::Action { actor_id: actor.into(), action_id: action.into(), target_ids })
    }

    #[test]
    fn casts_take_save_dc_and_attack_bonus_from_the_caster() {
        let block = caster_block(1);
        let (effects, _) = bound_cast_effects(&block, &cast("fireball", Some(3))).unwrap();
        assert!(matches!(&effects[0], Effect::SavingThrow(s) if s.dc == 15));
        let (effects, _) = bound_cast_effects(&block, &cast("fire-bolt", None)).unwrap();
        assert!(matches!(&effects[0], Effect::Attack(a) if a.modifier == 7));
    }

    #[test]
    fn heal_casts_add_the_spellcasting_modifier() {
        let (effects, _) = bound_cast_effects(&caster_block(1), &cast("cure-wounds", Some(1))).unwrap();
        assert!(matches!(&effects[0], Effect::Heal(h) if h.modifier == 3));
    }

    #[test]
    fn cantrip_dice_scale_at_levels_5_11_and_17() {
        for (level, dice) in [(4, "1d10"), (5, "2d10"), (11, "3d10"), (17, "4d10")] {
            let (effects, _) = bound_cast_effects(&caster_block(level), &cast("fire-bolt", None)).unwrap();
            let Effect::Attack(a) = &effects[0] else { panic!("fire bolt is an attack") };
            assert_eq!(a.damage[0].dice, dice, "level {level}");
        }
        let (effects, _) = bound_cast_effects(&caster_block(17), &cast("fireball", Some(3))).unwrap();
        assert!(matches!(&effects[0], Effect::SavingThrow(s) if s.damage[0].dice == "8d6"), "leveled spells do not scale");
    }

    #[test]
    fn leveled_casts_spend_a_slot_and_refuse_without_one() {
        let mut block = caster_block(1);
        block.spell_slots = Some(vec![1]);
        block.actions.push(ability("cure-wounds", vec![Effect::Cast(cast("cure-wounds", Some(1)))], None));
        let mut state = test_encounter(vec![block], vec![test_combatant("mage", "mage", 30, "ally")]);
        let entry = state.stat_blocks[0].stat_block.actions[0].clone();
        assert!(ability_ready(&state, "mage", &entry));
        act(&mut state, "mage", "cure-wounds", &["mage"]).unwrap();
        assert_eq!(state.combatants[0].resources["mage:slot:1"], 1);
        assert!(!ability_ready(&state, "mage", &entry));
        state.combatants[0].action_economy = ActionEconomy::default();
        let err = act(&mut state, "mage", "cure-wounds", &["mage"]).unwrap_err();
        assert!(matches!(err, EngineError::Illegal(m) if m.contains("spell slot")));
    }

    #[test]
    fn bonus_action_runs_once_per_turn_beside_the_action() {
        let mut block = pc_block("pc", 40);
        block.bonus_actions.push(ability("second-wind", vec![fx(serde_json::json!({"heal": {"dice": "1d1", "modifier": 9}}))], None));
        block.actions.push(ability("wait", vec![], None));
        let mut pc = test_combatant("pc", "pc", 40, "ally");
        pc.hp = 10;
        let mut state = test_encounter(vec![block], vec![pc]);
        act(&mut state, "pc", "second-wind", &["pc"]).unwrap();
        assert_eq!(state.combatants[0].hp, 20);
        assert!(state.combatants[0].action_economy.bonus_action_used);
        let err = act(&mut state, "pc", "second-wind", &["pc"]).unwrap_err();
        assert!(matches!(err, EngineError::Illegal(m) if m.contains("bonus action")));
        act(&mut state, "pc", "wait", &[]).unwrap();
    }

    #[test]
    fn reactions_run_off_turn_once_until_the_owner_starts_a_turn() {
        let mut block = caster_block(1);
        block.spell_slots = Some(vec![4]);
        block.reactions.push(ability("shield", vec![Effect::Cast(cast("shield", Some(1)))], None));
        let mut state = test_encounter(
            vec![block, test_stat_block("brute", "Brute", 50, 10)],
            vec![test_combatant("brute", "brute", 50, "enemy"), test_combatant("mage", "mage", 30, "ally")],
        );
        assert_eq!(state.active_turn.as_deref(), Some("brute"));
        act(&mut state, "mage", "shield", &["mage"]).unwrap();
        assert_eq!(ac_modifier(&state, &state.combatants[1].clone()), 5);
        let err = act(&mut state, "mage", "shield", &["mage"]).unwrap_err();
        assert!(matches!(err, EngineError::Illegal(m) if m.contains("reaction")));
        end_turn_events(&mut state).unwrap();
        assert!(!state.combatants[1].action_economy.reaction_used);
    }

    #[test]
    fn a_hit_on_a_downed_pc_adds_death_save_failures() {
        for seed in 1..60 {
            let mut state = brute_vs_pc(20, smash(30, "1d6", 0), |pc| {
                pc.hp = 0;
                pc.end_state = EndState::Unconscious;
            });
            state.rng = RngState::new(seed);
            let events = act(&mut state, "brute", "smash", &["pc"]).unwrap();
            let natural = events.iter().find(|e| e.kind == "attack").and_then(|e| e.roll).unwrap();
            let expected = match natural {
                1 => 0,
                20 => 2,
                _ => 1,
            };
            assert_eq!(state.combatants[0].death_saves.failures, expected, "seed {seed} rolled {natural}");
            assert_eq!(state.combatants[0].hp, 0, "damage while down never goes below 0");
        }
    }

    #[test]
    fn three_failures_from_hits_kill_a_downed_pc() {
        let mut state = brute_vs_pc(20, smash(30, "1d6", 0), |pc| {
            pc.hp = 0;
            pc.end_state = EndState::Unconscious;
            pc.death_saves.failures = 2;
        });
        state.rng = RngState::new(2);
        let events = act(&mut state, "brute", "smash", &["pc"]).unwrap();
        if events.iter().any(|e| e.kind == "damage") {
            assert_eq!(state.combatants[0].end_state, EndState::Dead);
        }
    }

    #[test]
    fn massive_damage_kills_a_pc_outright() {
        let mut state = brute_vs_pc(10, smash(30, "1d1", 40), |_| {});
        state.rng = RngState::new(3);
        let events = act(&mut state, "brute", "smash", &["pc"]).unwrap();
        assert!(events.iter().any(|e| e.kind == "damage"));
        assert_eq!(state.combatants[0].end_state, EndState::Dead);
    }

    #[test]
    fn the_dying_field_not_the_class_decides_who_makes_death_saves() {
        let mut state = brute_vs_pc(10, smash(30, "1d1", 12), |_| {});
        state.stat_blocks[0].stat_block.class = None;
        state.rng = RngState::new(3);
        act(&mut state, "brute", "smash", &["pc"]).unwrap();
        assert_eq!(state.combatants[0].end_state, EndState::Unconscious, "dying creature without a class");

        let mut state = brute_vs_pc(10, smash(30, "1d1", 12), |_| {});
        state.stat_blocks[0].stat_block.dying = false;
        state.rng = RngState::new(3);
        act(&mut state, "brute", "smash", &["pc"]).unwrap();
        assert_eq!(state.combatants[0].end_state, EndState::Dead, "classed creature without the dying rules");
    }

    #[test]
    fn healing_a_downed_pc_restores_consciousness_and_clears_saves() {
        let mut block = caster_block(1);
        block.actions.push(ability("mend", vec![fx(serde_json::json!({"heal": {"dice": "1d1", "modifier": 4}}))], None));
        let mut downed = test_combatant("pc", "pc", 20, "ally");
        downed.hp = 0;
        downed.end_state = EndState::Unconscious;
        downed.death_saves = DeathSaves { successes: 1, failures: 2 };
        let mut state = test_encounter(vec![block, pc_block("pc", 20)], vec![test_combatant("mage", "mage", 30, "ally"), downed]);
        let events = act(&mut state, "mage", "mend", &["pc"]).unwrap();
        assert!(events.iter().any(|e| e.kind == "revive"));
        assert_eq!((state.combatants[1].hp, state.combatants[1].end_state), (5, EndState::Active));
        assert_eq!(state.combatants[1].death_saves.failures, 0);
    }

    fn asking_caster(faces: Vec<u32>, engine_fills_rest: bool) -> EncounterState {
        let mut block = caster_block(1);
        block.actions.push(ability("mend", vec![fx(serde_json::json!({"heal": {"dice": "2d4", "modifier": 1}}))], None));
        block.bonus_actions.push(ability("ward", vec![fx(serde_json::json!({"temp_hp": {"dice": "1d6", "modifier": 2}}))], None));
        let mut ally = test_combatant("pc", "pc", 20, "ally");
        ally.hp = 5;
        let mut state = test_encounter(vec![block, pc_block("pc", 20)], vec![test_combatant("mage", "mage", 30, "ally"), ally]);
        state.settings.pc_dice = PcDice::Ask;
        state.player_dice.faces = faces.into();
        state.player_dice.engine_fills_rest = engine_fills_rest;
        state
    }

    #[test]
    fn heal_and_temp_hp_events_are_marked_when_every_die_was_entered() {
        let mut state = asking_caster(vec![3, 4, 5], false);
        state.combatants[0].hp = 10;
        state.combatants[1].hp = 0;
        state.combatants[1].end_state = EndState::Unconscious;
        let healed = act(&mut state, "mage", "mend", &["pc", "mage"]).unwrap();
        let marks: Vec<_> = healed.iter().filter(|e| e.kind == "heal" || e.kind == "revive").map(|e| (e.kind.as_str(), e.player_dice)).collect();
        assert_eq!(marks, [("revive", true), ("heal", true), ("heal", true)], "{healed:?}");
        assert_eq!((state.combatants[0].hp, state.combatants[1].hp), (18, 8));
        let warded = act(&mut state, "mage", "ward", &["pc"]).unwrap();
        let temp = warded.iter().find(|e| e.kind == "temp_hp").expect("temp hp event");
        assert!(temp.player_dice, "{warded:?}");
        assert_eq!(temp.message, "pc gains 7 temporary HP");
        assert_eq!(state.combatants[1].temp_hp, 7);
    }

    #[test]
    fn heal_and_temp_hp_with_an_engine_die_are_not_marked() {
        let mut state = asking_caster(vec![3], true);
        let healed = act(&mut state, "mage", "mend", &["pc"]).unwrap();
        assert!(!healed.iter().find(|e| e.kind == "heal").unwrap().player_dice, "{healed:?}");
        let warded = act(&mut state, "mage", "ward", &["pc"]).unwrap();
        assert!(!warded.iter().find(|e| e.kind == "temp_hp").unwrap().player_dice, "{warded:?}");
        let mut state = asking_caster(vec![], false);
        state.settings.pc_dice = PcDice::Engine;
        let warded = act(&mut state, "mage", "ward", &[]).unwrap();
        let temp = warded.iter().find(|e| e.kind == "temp_hp").unwrap();
        assert!(!temp.player_dice && temp.target_id.as_deref() == Some("mage"), "{warded:?}");
    }

    #[test]
    fn death_save_events_carry_the_d20_and_mark_an_entered_face() {
        for (face, fills, marked) in [(Some(12), false, true), (Some(1), false, true), (None, true, false)] {
            let mut state = brute_vs_pc(20, smash(30, "1d6", 0), |pc| {
                pc.hp = 0;
                pc.end_state = EndState::Unconscious;
            });
            state.settings.pc_dice = PcDice::Ask;
            state.player_dice.faces = face.into_iter().collect();
            state.player_dice.engine_fills_rest = fills;
            let mut events = vec![];
            roll_death_saves(&mut state, "pc", &mut events).unwrap();
            let save = events.iter().find(|e| e.kind == "death_save").expect("death save");
            assert_eq!(save.player_dice, marked, "{events:?}");
            assert!(save.roll.is_some(), "{events:?}");
            if let Some(face) = face {
                assert_eq!(save.roll, Some(face));
            }
        }
    }

    #[test]
    fn legendary_actions_follow_the_per_round_budget() {
        let mut dragon = test_stat_block("dragon", "Dragon", 100, 18);
        dragon.legendary_uses_per_round = 2;
        dragon.legendary_actions.push(smash(5, "1d1", 0));
        let mut state = test_encounter(
            vec![dragon, pc_block("pc", 200)],
            vec![test_combatant("pc", "pc", 200, "ally"), test_combatant("dragon", "dragon", 100, "enemy")],
        );
        let targets = ["pc".to_string()];
        use_legendary_action(&mut state, "dragon", "smash", &targets).unwrap();
        use_legendary_action(&mut state, "dragon", "smash", &targets).unwrap();
        assert!(matches!(use_legendary_action(&mut state, "dragon", "smash", &targets), Err(EngineError::Illegal(_))));
        end_turn_events(&mut state).unwrap();
        assert_eq!(state.active_turn.as_deref(), Some("dragon"));
        use_legendary_action(&mut state, "dragon", "smash", &targets).unwrap();
    }

    #[test]
    fn cooldown_abilities_come_back_at_the_start_of_the_owners_turn() {
        let mut block = test_stat_block("brute", "Brute", 50, 10);
        let mut slam = smash(5, "1d1", 0);
        slam.usage = Some(Usage::CooldownUntilNextTurn);
        block.actions.push(slam);
        let mut state = test_encounter(vec![block, pc_block("pc", 200)], vec![
            test_combatant("brute", "brute", 50, "enemy"),
            test_combatant("pc", "pc", 200, "ally"),
        ]);
        act(&mut state, "brute", "smash", &["pc"]).unwrap();
        let entry = state.stat_blocks[0].stat_block.actions[0].clone();
        assert!(!ability_ready(&state, "brute", &entry));
        end_turn_events(&mut state).unwrap();
        end_turn_events(&mut state).unwrap();
        assert_eq!(state.active_turn.as_deref(), Some("brute"));
        assert!(ability_ready(&state, "brute", &entry));
    }

    #[test]
    fn area_wrapper_caps_the_targets_it_resolves() {
        let mut block = test_stat_block("drake", "Drake", 50, 10);
        block.actions.push(ability(
            "spit",
            vec![fx(serde_json::json!({"area": {"area": {"cone": {"size": 15}}, "origin": "self_", "target_count": 2,
                "effect": {"damage": {"dice": "1d1", "type": "acid", "modifier": 2}}}}))],
            None,
        ));
        let mut blocks = vec![block];
        let mut combatants = vec![test_combatant("drake", "drake", 50, "enemy")];
        for id in ["a", "b", "c"] {
            blocks.push(pc_block(id, 20));
            combatants.push(test_combatant(id, id, 20, "ally"));
        }
        let mut state = test_encounter(blocks, combatants);
        act(&mut state, "drake", "spit", &["a", "b", "c"]).unwrap();
        let hp: Vec<i32> = state.combatants[1..].iter().map(|c| c.hp).collect();
        assert_eq!(hp, vec![17, 17, 20]);
    }

    #[test]
    fn failed_saves_apply_their_on_failure_effects() {
        let mut block = test_stat_block("gorgon", "Gorgon", 50, 10);
        block.actions.push(ability(
            "gaze",
            vec![fx(serde_json::json!({"saving_throw": {"ability": "con", "dc": 40, "half_on_success": false,
                "no_damage_on_success": true, "damage": [], "on_failure":[{"condition": {"condition": "restrained", "duration": {"rounds": 1}}}]}}))],
            None,
        ));
        let mut state = test_encounter(vec![block, pc_block("pc", 20)], vec![
            test_combatant("gorgon", "gorgon", 50, "enemy"),
            test_combatant("pc", "pc", 20, "ally"),
        ]);
        act(&mut state, "gorgon", "gaze", &["pc"]).unwrap();
        assert!(state.combatants[1].conditions.iter().any(|c| c.condition == Condition::Restrained));
    }

    #[test]
    fn damage_ends_conditions_marked_ends_on_damage() {
        let blocks = vec![test_stat_block("a", "A", 20, 10), test_stat_block("b", "B", 20, 10)];
        let mut state = test_encounter(blocks, vec![test_combatant("a", "a", 20, "ally"), test_combatant("b", "b", 20, "enemy")]);
        state.combatants[1].conditions.push(ConditionInstance { condition: Condition::Unconscious, duration: Duration::Concentration, source: Some("a".into()), ends_on_damage: true });
        state.combatants[1].conditions.push(ConditionInstance { condition: Condition::Prone, duration: Duration::Permanent, source: None, ends_on_damage: false });
        let mut stream = Stream::from_seed(1);
        let mut events = Vec::new();
        apply_damage(&mut state, &mut stream, "b", 3, Some("a"), false, false, &mut events).unwrap();
        let left: Vec<Condition> = state.combatants[1].conditions.iter().map(|c| c.condition).collect();
        assert_eq!(left, vec![Condition::Prone]);
        assert!(events.iter().any(|e| e.message.contains("roused")));
    }

    #[test]
    fn breaking_concentration_ends_what_it_held() {
        let blocks = vec![test_stat_block("a", "A", 20, 10), test_stat_block("b", "B", 20, 10)];
        let mut state = test_encounter(blocks, vec![test_combatant("a", "a", 20, "ally"), test_combatant("b", "b", 20, "enemy")]);
        state.combatants[0].concentration = Some("hold-person".into());
        state.combatants[1].conditions.push(ConditionInstance { condition: Condition::Paralyzed, duration: Duration::Concentration, source: Some("a".into()), ends_on_damage: false });
        state.combatants[1].conditions.push(ConditionInstance { condition: Condition::Prone, duration: Duration::Concentration, source: Some("b".into()), ends_on_damage: false });
        state.active_modifiers.push(ActiveModifier { target: ModifierTarget::ArmorClass, change: ModifierChange::Add(2), remaining: Duration::Concentration, source: Some("a".into()) });
        let mut events = Vec::new();
        break_concentration(&mut state, "a", &mut events).unwrap();
        assert_eq!(state.combatants[1].conditions.len(), 1);
        assert_eq!(state.combatants[1].conditions[0].condition, Condition::Prone);
        assert!(state.active_modifiers.is_empty());
    }

    #[test]
    fn moving_mirrors_the_band_on_the_target() {
        let blocks = vec![test_stat_block("a", "A", 20, 10), test_stat_block("b", "B", 20, 10)];
        let mut state = test_encounter(blocks, vec![test_combatant("a", "a", 20, "ally"), test_combatant("b", "b", 20, "enemy")]);
        state.combatants[0].ranges.insert("b".into(), RangeBand::Near);
        state.combatants[1].ranges.insert("a".into(), RangeBand::Near);
        move_combatant(&mut state, "a", Some("b"), RangeBand::Far, 30).unwrap();
        assert_eq!(state.combatants[1].ranges["a"], RangeBand::Far);
    }

    fn three_way_encounter() -> EncounterState {
        let blocks = vec![test_stat_block("a", "A", 20, 10), test_stat_block("b", "B", 20, 10), test_stat_block("c", "C", 20, 10)];
        let mut state = test_encounter(blocks, vec![test_combatant("a", "a", 20, "ally"), test_combatant("b", "b", 20, "enemy"), test_combatant("c", "c", 20, "enemy")]);
        state.combatants[0].ranges.insert("b".into(), RangeBand::Engaged);
        state.combatants[1].ranges.insert("a".into(), RangeBand::Engaged);
        state.combatants[0].engagement.push("b".into());
        state.combatants[1].engagement.push("a".into());
        state.combatants[0].ranges.insert("c".into(), RangeBand::Near);
        state.combatants[2].ranges.insert("a".into(), RangeBand::Near);
        state
    }

    #[test]
    fn untargeted_move_changes_the_band_to_everyone() {
        let mut state = three_way_encounter();
        let events = move_combatant(&mut state, "a", None, RangeBand::Far, 35).unwrap();
        assert_eq!(state.combatants[0].ranges["b"], RangeBand::Far);
        assert_eq!(state.combatants[0].ranges["c"], RangeBand::Far);
        assert_eq!(state.combatants[1].ranges["a"], RangeBand::Far);
        assert_eq!(state.combatants[2].ranges["a"], RangeBand::Far);
        assert!(state.combatants[0].engagement.is_empty());
        assert!(state.combatants[1].engagement.is_empty());
        let attacks: Vec<_> = events.iter().filter(|e| e.kind == "opportunity_attack").collect();
        assert_eq!(attacks.len(), 1);
        assert!(attacks[0].message.starts_with("b may use"));
    }

    #[test]
    fn untargeted_move_costs_the_largest_single_pair() {
        let mut state = three_way_encounter();
        let err = move_combatant(&mut state, "a", None, RangeBand::Far, 30).unwrap_err();
        assert!(err.to_string().contains("35 feet"));
        assert_eq!(state.combatants[0].ranges["b"], RangeBand::Engaged);
        move_combatant(&mut state, "a", None, RangeBand::Far, 35).unwrap();
        assert_eq!(state.combatants[0].action_economy.movement_used_feet, 35);
    }

    #[test]
    fn untargeted_move_skips_combatants_that_are_down() {
        let mut state = three_way_encounter();
        state.combatants[1].end_state = EndState::Dead;
        let events = move_combatant(&mut state, "a", None, RangeBand::Far, 30).unwrap();
        assert!(events.iter().all(|e| e.kind != "opportunity_attack"));
        assert_eq!(state.combatants[0].ranges["b"], RangeBand::Far);
        assert_eq!(state.combatants[0].ranges["c"], RangeBand::Far);
        assert!(state.combatants[0].engagement.is_empty());
        assert!(state.combatants[1].engagement.is_empty());
    }

    #[test]
    fn untargeted_move_cannot_engage() {
        let mut state = three_way_encounter();
        assert!(move_combatant(&mut state, "a", None, RangeBand::Engaged, 30).is_err());
    }
}
