//! Deterministic encounter execution.
//!
//! The engine deliberately accepts structured requests rather than ability
//! prose.  This keeps the rules path shared by live and batch callers.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::effect::{
    recharge_minimum, Attack, Duration, Effect, ExtraActionUse, Lighting, ModifierBearer, ModifierChange, ModifierTarget, ReactionTrigger, SpellCaster, RollDisposition, TeleportSubject, Usage,
};
use crate::encounter::{
    ActionEconomy, ActiveModifier, Combatant, ConditionInstance, EndState, EncounterState,
    PcDice, PendingPrompt, RangeBand,
};
use crate::reaction::{ReactionAnswer, ReactionOption, ReactionWindow, WindowTrigger};
use crate::rng::Stream;
use crate::stat_block::{Ability, Condition, DamageType, DeathSaves, StatBlock};

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StepRequest {
    Action {
        actor_id: String,
        action_id: String,
        target_ids: Vec<String>,
        /// Cast level for every leveled Cast in the action (`bench action --slot`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        slot_level: Option<u8>,
        /// Spend the extra action a modifier grants (Haste) instead of the action.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        extra: bool,
    },
    Move {
        actor_id: String,
        target_id: Option<String>,
        band: RangeBand,
        feet: u32,
        /// Opportunity Attacks taken on this move, resolved in this order
        /// right before the band changes (ADR 0015).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        opportunity: Vec<OpportunityAttack>,
        /// Reactors whose Opportunity Attack window was declined (ADR 0018).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        declined: Vec<String>,
    },
    /// The Disengage action: spends the action, or the extra action with `extra`, and sets
    /// the turn flag.
    Disengage {
        actor_id: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        extra: bool,
    },
    /// Ends the active turn. The legendary actions listed resolve in order
    /// after the turn ends and before the next one starts.
    EndTurn {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        legendary: Vec<LegendaryUse>,
    },
    Respond { values: Vec<u32>, accept: bool },
    RollInitiative(InitiativeOptions),
}

impl StepRequest {
    /// An end-turn with no legendary actions.
    pub fn end_turn() -> Self {
        StepRequest::EndTurn { legendary: vec![] }
    }
}

/// One legendary action taken in the window between two turns. Each costs one
/// of the holder's uses this round.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct LegendaryUse {
    pub holder_id: String,
    pub action_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_ids: Vec<String>,
}

/// A creature that may take a legendary action when the active turn ends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegendaryHolder {
    pub id: String,
    pub uses_left: u32,
    /// Legendary actions whose usage allows them now, in stat-block order.
    pub ready_actions: Vec<String>,
}

/// A reactor that takes its Opportunity Attack on a move. Without `attack_id`
/// it uses its first at-will melee attack in stat-block order, looking inside
/// Multiattack steps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct OpportunityAttack {
    pub reactor_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack_id: Option<String>,
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
        StepRequest::Action { actor_id, action_id, target_ids, slot_level, extra } => {
            resolve_action(state, &actor_id, &action_id, &target_ids, slot_level, extra)?
        }
        StepRequest::Move { actor_id, target_id, band, feet, opportunity, declined } => {
            move_combatant(state, &actor_id, target_id.as_deref(), band, feet, &opportunity, &declined)?
        }
        StepRequest::Disengage { actor_id, extra } => disengage(state, &actor_id, extra)?,
        StepRequest::EndTurn { legendary } => end_turn_events(state, &legendary)?,
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
    slot_level: Option<u8>,
    extra: bool,
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
    if extra {
        if section != ActionSection::Action {
            return Err(EngineError::Illegal(format!("{actor_id}'s extra action can't take {action_id}, which is not an action")));
        }
        let uses = extra_action_left(state, actor_id)?;
        if !extra_action_allows(&uses, &action) {
            return Err(extra_action_refused(actor_id, &uses));
        }
    } else {
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
    let effects = match slot_level {
        Some(level) => with_slot(state, actor_index, &action, level)?,
        None => action.effects.clone(),
    };
    if let Some(usage) = &action.usage {
        spend_usage(state, actor_index, action_id, usage)?;
    }
    let mut stream = Stream::from_state(state.rng.clone());
    let mut events = Vec::new();
    if extra {
        events.push(event("extra_action", format!("{actor_id} uses its extra action"), actor_id, actor_id));
    }
    let was_reacting = state.reactions.reacting;
    state.reactions.reacting |= section == ActionSection::Reaction;
    for effect in effects {
        if matches!(effect, Effect::Disengage) {
            events.push(set_disengaged(state, actor_id, Some(&action.name))?);
            continue;
        }
        let resolved = resolve_effect(state, &mut stream, actor_id, effect, target_ids, &mut events);
        if resolved.is_err() {
            state.reactions.reacting = was_reacting;
        }
        resolved?;
        if state.pending.is_some() {
            break;
        }
    }
    state.reactions.reacting = was_reacting;
    state.rng = stream.state();
    let economy = &mut state.combatants[actor_index].action_economy;
    match section {
        _ if extra => economy.extra_action_used = true,
        ActionSection::Action => economy.action_used = true,
        ActionSection::Bonus => economy.bonus_action_used = true,
        ActionSection::Reaction => economy.reaction_used = true,
    }
    Ok(events)
}

/// The action's effects with every leveled Cast set to the chosen slot level. Refused
/// unless the caster has spell slots, the action casts a leveled spell no higher than
/// the slot, and a slot of that level is left.
fn with_slot(
    state: &EncounterState,
    actor_index: usize,
    action: &crate::stat_block::AbilityEntry,
    level: u8,
) -> Result<Vec<Effect>, EngineError> {
    let actor = &state.combatants[actor_index];
    let block = stat_block(state, actor)?;
    if block.spell_slots.is_none() {
        return Err(EngineError::Invalid(format!(
            "{} has no spell slots to choose from",
            actor.id
        )));
    }
    let spells = load_spell_effects()?;
    let mut effects = action.effects.clone();
    let mut leveled = false;
    for effect in &mut effects {
        let Effect::Cast(cast) = effect else { continue };
        let Some(spell) = spells.get(&cast.spell_id).filter(|s| s.level > 0) else {
            continue;
        };
        cast.slot_level = Some(level);
        cast_level(spell, cast)?;
        leveled = true;
    }
    if !leveled {
        return Err(EngineError::Invalid(format!(
            "{} casts no leveled spell, so a slot level does not apply",
            action.id
        )));
    }
    spend_slot(actor, block, level)?;
    Ok(effects)
}

/// The Disengage action. The flag lasts until the actor's turn ends.
fn disengage(state: &mut EncounterState, actor_id: &str, extra: bool) -> Result<Vec<EngineEvent>, EngineError> {
    let i = index_of(state, actor_id)?;
    if !can_act(&state.combatants[i]) {
        return Err(EngineError::Illegal(format!("{actor_id} cannot act")));
    }
    if state.active_turn.as_deref().is_some_and(|active| active != actor_id) {
        return Err(EngineError::Illegal(format!("{actor_id} is not the active combatant")));
    }
    if extra {
        let uses = extra_action_left(state, actor_id)?;
        if !(uses.is_empty() || uses.contains(&ExtraActionUse::Disengage)) {
            return Err(extra_action_refused(actor_id, &uses));
        }
        let economy = &mut state.combatants[i].action_economy;
        economy.extra_action_used = true;
        economy.disengaged = true;
        return Ok(vec![
            event("extra_action", format!("{actor_id} uses its extra action"), actor_id, actor_id),
            event("disengage", format!("{actor_id} takes the Disengage action"), actor_id, actor_id),
        ]);
    }
    let economy = &mut state.combatants[i].action_economy;
    if economy.action_used {
        return Err(EngineError::Illegal("action already used".into()));
    }
    economy.action_used = true;
    economy.disengaged = true;
    Ok(vec![event("disengage", format!("{actor_id} takes the Disengage action"), actor_id, actor_id)])
}

fn set_disengaged(state: &mut EncounterState, actor_id: &str, by: Option<&str>) -> Result<EngineEvent, EngineError> {
    let i = index_of(state, actor_id)?;
    state.combatants[i].action_economy.disengaged = true;
    let message = match by {
        Some(name) => format!("{actor_id} uses {name} to Disengage"),
        None => format!("{actor_id} disengages"),
    };
    Ok(event("disengage", message, actor_id, actor_id))
}

fn extra_action_grant(state: &EncounterState, actor_id: &str) -> Option<Vec<ExtraActionUse>> {
    let mut uses = Vec::new();
    let mut granted = false;
    let mut any = false;
    for m in state.modifiers_on(actor_id) {
        let ModifierTarget::ExtraAction { only } = &m.target else { continue };
        granted = true;
        any |= only.is_empty();
        for u in only {
            if !uses.contains(u) {
                uses.push(*u);
            }
        }
    }
    granted.then(|| if any { vec![] } else { uses })
}

fn extra_action_left(state: &EncounterState, actor_id: &str) -> Result<Vec<ExtraActionUse>, EngineError> {
    let uses = extra_action_grant(state, actor_id).ok_or_else(|| EngineError::Illegal(format!("{actor_id} has no extra action")))?;
    if state.combatants[index_of(state, actor_id)?].action_economy.extra_action_used {
        return Err(EngineError::Illegal("extra action already used".into()));
    }
    Ok(uses)
}

fn extra_action_refused(actor_id: &str, uses: &[ExtraActionUse]) -> EngineError {
    let uses: Vec<&str> = uses.iter().map(|u| u.as_str()).collect();
    EngineError::Illegal(format!("{actor_id}'s extra action allows only {}", uses.join(", ")))
}

/// The uses of the extra action `actor_id` can take now, or `None` when it has none: no
/// `extra_action` modifier applies to it, it spent the extra action this turn, or it can't act.
/// An empty list allows any action. Several applying modifiers give one extra action whose uses
/// join, and any empty list among them allows any action.
pub fn extra_action_uses(state: &EncounterState, actor_id: &str) -> Option<Vec<ExtraActionUse>> {
    let c = state.combatants.iter().find(|c| c.id == actor_id)?;
    if !can_act(c) || c.action_economy.extra_action_used {
        return None;
    }
    extra_action_grant(state, actor_id)
}

/// Whether an extra action limited to `uses` may take `action` from the stat block's actions.
/// `one_attack` allows an action whose effects are exactly one Attack with one target; an
/// empty list allows any action. Disengage is the `disengage` use and not an action entry.
pub fn extra_action_allows(uses: &[ExtraActionUse], action: &crate::stat_block::AbilityEntry) -> bool {
    uses.is_empty()
        || (uses.contains(&ExtraActionUse::OneAttack) && matches!(action.effects.as_slice(), [Effect::Attack(a)] if a.target_count == 1))
}

/// The resource a usage draws on: its group's pool when it names one, else the ability's own.
fn usage_key(actor_id: &str, ability_id: &str, usage: &Usage) -> String {
    match usage {
        Usage::Recharge { group: Some(group), .. } | Usage::PerDay { group: Some(group), .. } => format!("{actor_id}:{group}"),
        _ => format!("{actor_id}:{ability_id}"),
    }
}

fn spend_usage(state: &mut EncounterState, actor_index: usize, action_id: &str, usage: &Usage) -> Result<(), EngineError> {
    let actor_id = state.combatants[actor_index].id.clone();
    let key = usage_key(&actor_id, action_id, usage);
    let resources = &mut state.combatants[actor_index].resources;
    match usage {
        Usage::AtWill => {}
        Usage::Recharge { .. } => {
            let remaining = resources.get(&key).copied().unwrap_or(1);
            if remaining <= 0 {
                return Err(EngineError::Illegal(format!("{action_id} is not recharged")));
            }
            resources.insert(key, remaining - 1);
        }
        Usage::PerDay { uses, .. } | Usage::PerRest { uses, .. } => {
            let used = resources.get(&key).copied().unwrap_or(0);
            if used >= *uses as i32 {
                return Err(EngineError::Illegal(format!("{action_id} has no uses remaining")));
            }
            resources.insert(key, used + 1);
        }
        Usage::CooldownUntilNextTurn => {
            let remaining = resources.get(&key).copied().unwrap_or(1);
            if remaining <= 0 {
                return Err(EngineError::Illegal(format!("{action_id} is on cooldown")));
            }
            resources.insert(key, 0);
        }
    }
    Ok(())
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
                let advantage = disposition(state, &actor, &target, &attack);
                let mut sources = lighting_sources(state, &actor, Roll::Attack);
                if in_close_combat(state, &actor, &target, &attack, Within::Held) {
                    sources.push((RollDisposition::Disadvantage, CLOSE_COMBAT.into()));
                }
                let note = disposition_note(advantage, sources);
                let d20a = stream.roll_die(20);
                let d20a = player_roll(state, actor_id, 20, d20a);
                let d20b = stream.roll_die(20);
                let d20b = if matches!(advantage, RollDisposition::Normal) { d20b } else { player_roll_in(state, actor_id, 20, d20b, false) };
                let natural = match advantage {
                    RollDisposition::Advantage => d20a.max(d20b),
                    RollDisposition::Disadvantage => d20a.min(d20b),
                    RollDisposition::Normal => d20a,
                };
                let (die, die_note) = bonus_dice(state, stream, actor_id, |t| *t == ModifierTarget::AttackRolls)?;
                let note = note + &die_note;
                let total = natural as i32 + attack.modifier + attack_modifier(state, &actor)? + die;
                let base_ac = stat_block(state, &target)?.armor_class.first().map(|a| a.value).unwrap_or(10);
                let ac = base_ac + ac_modifier(state, &target);
                let natural_20 = natural == 20;
                let mut hit = natural_20 || (natural != 1 && total >= ac);
                let automatic = automatic_critical(&actor, &target);
                let critical = natural_20 || automatic.is_some();
                let why_critical = match automatic {
                    Some(why) if hit && !natural_20 => format!(" (critical: {why}, within 5 feet)"),
                    _ => String::new(),
                };
                let marked = state.player_dice.all_entered_since(actor_id);
                events.push(EngineEvent {
                    kind: "attack".into(),
                    message: format!("{} attacks {}: {} vs AC {} ({}){why_critical}{note}", actor.name, target.name, total, ac, if hit { "hit" } else { "miss" }),
                    actor_id: Some(actor_id.into()),
                    target_id: Some(target_id.clone()),
                    roll: Some(natural),
                    total: Some(total),
                    damage: None,
                    player_dice: marked,
                });
                if hit && state.settings.reaction_windows {
                    let trigger = WindowTrigger::Hit { attacker_id: actor_id.into(), total, ac, critical };
                    if let Some(window) = hit_window(state, target_index, &attack, trigger) {
                        let answer = ask(state, window.clone(), events);
                        if let Some(ReactionAnswer::Take(id)) = answer {
                            let name = window.option(&id).ok_or_else(|| EngineError::Invalid(format!("{target_id} has no reaction {id} for this hit")))?.name.clone();
                            events.push(event("reaction", format!("REACTION {target_id}: {name} (hit by {actor_id})"), target_id, actor_id));
                            state.rng = stream.state();
                            let resolved = resolve_action(state, target_id, &id, &[actor_id.to_string()], None, false);
                            events.extend(blame_reaction(state, target_id, &id, resolved)?);
                            *stream = Stream::from_state(state.rng.clone());
                            let ac = base_ac + ac_modifier(state, &state.combatants[target_index]);
                            hit = natural_20 || (natural != 1 && total >= ac);
                            events.push(EngineEvent {
                                kind: "attack_rechecked".into(),
                                message: format!("{} attacks {}: {} vs AC {} ({}) after {name}", actor.name, target.name, total, ac, if hit { "hit" } else { "miss" }),
                                actor_id: Some(actor_id.into()),
                                target_id: Some(target_id.clone()),
                                roll: None,
                                total: Some(total),
                                damage: None,
                                player_dice: false,
                            });
                            let clauses = stat_block(state, &state.combatants[target_index])?.reactions.iter().find(|a| a.id == id).map(|a| a.agent_clauses.clone()).unwrap_or_default();
                            for clause in clauses {
                                events.push(event("agent_clause", format!("{target_id} {name}, agent clause: {clause}"), target_id, actor_id));
                            }
                        }
                    }
                }
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
                end_triggering_attack(state, stream, target_id, events)?;
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
                        started_round: state.round,
                    });
                    events.push(event("condition", format!("{target_id} is {}", format!("{:?}", condition.condition).to_lowercase()), actor_id, target_id));
                    if incapacitates(condition.condition) {
                        break_concentration(state, stream, target_id, events)?;
                    }
                }
            }
        }
        Effect::TimedModifier(modifier) => {
            let on_actor = modifier.bearer == ModifierBearer::Actor || matches!(modifier.duration, Duration::TriggeringAttack);
            let bearers = if on_actor { vec![actor_id.to_string()] } else { target_ids.to_vec() };
            if bearers.is_empty() {
                let ability = modifier.source_ability.as_deref().unwrap_or("this ability");
                return Err(EngineError::Illegal(format!("{ability} needs a target for its modifier")));
            }
            let what = modifier.source_ability.as_deref().unwrap_or("a timed modifier");
            for bearer in bearers {
                index_of(state, &bearer)?;
                events.push(event("modifier", format!("{bearer} gains {what} from {actor_id}"), actor_id, &bearer));
                state.active_modifiers.push(ActiveModifier {
                    target: modifier.target.clone(),
                    change: modifier.change.clone(),
                    remaining: modifier.duration.clone(),
                    source: Some(actor_id.into()),
                    bearer: Some(bearer),
                    source_ability: modifier.source_ability.clone(),
                    ends_on_damage: modifier.ends_on_damage,
                    on_end: modifier.on_end.clone(),
                    started_round: state.round,
                });
            }
        }
        Effect::ForcedMove(movement) => {
            for target_id in target_ids {
                apply_forced_move(state, actor_id, target_id, movement.feet, &movement.direction, events)?;
            }
        }
        Effect::Heal(heal) => {
            let rolled = match &heal.dice {
                Some(dice) => roll_dice(state, stream, actor_id, dice)? as i32,
                None => 0,
            };
            let amount = rolled + heal.modifier;
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
        Effect::Teleport(teleport) => {
            let subjects = match teleport.subject {
                TeleportSubject::Actor => vec![actor_id.to_string()],
                TeleportSubject::Target => target_ids.to_vec(),
            };
            if subjects.is_empty() {
                return Err(EngineError::Illegal("a teleport of its target needs a target".into()));
            }
            for subject in subjects {
                let i = index_of(state, &subject)?;
                if state.settings.teleport_near {
                    let mut freed = state.combatants[i].engagement.clone();
                    freed.extend(state.combatants.iter().filter(|c| c.engagement.contains(&subject)).map(|c| c.id.clone()));
                    for other in freed {
                        if let Some(band) = state.combatants[i].ranges.get_mut(&other) {
                            *band = RangeBand::Near;
                        }
                        if let Some(band) = state.combatants.iter_mut().find(|c| c.id == other).and_then(|c| c.ranges.get_mut(&subject)) {
                            *band = RangeBand::Near;
                        }
                    }
                }
                state.combatants[i].engagement.clear();
                for c in &mut state.combatants {
                    c.engagement.retain(|id| id != &subject);
                }
                events.push(event("teleport", format!("{subject} teleports up to {} feet", teleport.range), actor_id, &subject));
            }
        }
        Effect::MoveThenAttack(movement) => {
            let i = index_of(state, actor_id)?;
            let speed = walk_speed(state, &state.combatants[i]);
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
        Effect::RollModifier(_) | Effect::NoProvoke | Effect::SeeInvisible => {}
        Effect::CounterSpell(counter) => {
            for target_id in target_ids {
                if !save_succeeds(state, stream, actor_id, target_id, Ability::Con, counter.dc, events)? {
                    let spell = state.reactions.casting.clone().unwrap_or_else(|| "the spell".into());
                    events.push(event("spell_countered", format!("{target_id}'s {spell} is countered by {actor_id} (slot not expended)"), actor_id, target_id));
                    state.reactions.countered = true;
                }
            }
        }
        Effect::Disengage => events.push(set_disengaged(state, actor_id, None)?),
        Effect::Area(area) => {
            events.push(event("area", format!("{} creates an area effect", actor_id), actor_id, actor_id));
            let count = area.target_count.map_or(target_ids.len(), |n| n.max(1) as usize);
            let hit: Vec<String> = target_ids.iter().take(count).cloned().collect();
            resolve_effect(state, stream, actor_id, *area.effect, &hit, events)?;
        }
    }
    Ok(())
}

/// One saving throw by `target_id` against `dc`, logged as a `save` event.
fn save_succeeds(
    state: &mut EncounterState,
    stream: &mut Stream,
    actor_id: &str,
    target_id: &str,
    ability: Ability,
    dc: i32,
    events: &mut Vec<EngineEvent>,
) -> Result<bool, EngineError> {
    let target = state.combatants[index_of(state, target_id)?].clone();
    let block = stat_block(state, &target)?;
    let modifier = block.saving_throws.get(&ability).copied()
        .unwrap_or_else(|| block.ability_scores.modifier(ability))
        + save_modifier(state, &target, ability);
    if let Some(message) = automatic_save_failure(&target, ability) {
        events.push(event("save", message, actor_id, target_id));
        return Ok(false);
    }
    let (roll, note) = save_d20(state, stream, target_id, ability)?;
    let (die, die_note) = save_dice(state, stream, target_id, ability)?;
    let modifier = modifier + die;
    let success = roll as i32 + modifier >= dc;
    let marked = state.player_dice.all_entered_since(target_id);
    events.push(EngineEvent {
        kind: "save".into(),
        message: format!("{target_id} {} the save{note}{die_note}", if success { "passes" } else { "fails" }),
        actor_id: Some(actor_id.into()),
        target_id: Some(target_id.into()),
        roll: Some(roll),
        total: Some(roll as i32 + modifier),
        damage: None,
        player_dice: marked,
    });
    Ok(success)
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
    let success = save_succeeds(state, stream, actor_id, target_id, save.ability, save.dc, events)?;
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
    let level = cast_level(&spell, cast)?;
    let effects = scaled_effects(&spell, level)?;
    let i = index_of(state, actor_id)?;
    if spell.level > 0 && state.settings.spell_cast_windows && !state.reactions.reacting && spell_cast_countered(state, stream, i, spell_id, level, target_ids, events)? {
        return Ok(());
    }
    let ctx = {
        let block = stat_block(state, &state.combatants[i])?;
        spend_slot(&state.combatants[i], block, level)?;
        cast_context(block, spell.level == 0)
    };
    if level > 0
        && stat_block(state, &state.combatants[i])?
            .spell_slots
            .is_some()
    {
        *state.combatants[i]
            .resources
            .entry(format!("{actor_id}:slot:{level}"))
            .or_insert(0) += 1;
    }
    if spell.concentration {
        if state.combatants[i].concentration.is_some() {
            break_concentration(state, stream, actor_id, events)?;
        }
        state.combatants[i].concentration = Some(spell_id.into());
        events.push(event("concentration", format!("{} begins concentrating on {}", actor_id, spell_id), actor_id, actor_id));
    }
    if let Some(clause) = spell
        .upcast_clause
        .as_deref()
        .filter(|_| level > spell.level)
    {
        events.push(event(
            "upcast_clause",
            format!("{actor_id} casts {spell_id} at level {level}; agent clause: {clause}"),
            actor_id,
            actor_id,
        ));
    }
    for effect in effects {
        let effect = match &ctx {
            Some(ctx) => bind_effect(effect, ctx, spell.level == 0),
            None => effect,
        };
        resolve_effect(state, stream, actor_id, effect, target_ids, events)?;
    }
    Ok(())
}

/// Hold a leveled cast by `caster` for each enemy's spell-cast window, in initiative order
/// (ADR 0019), and say whether a counter cancelled it. A taken counter that fails its save
/// leaves the cast live and the later reactors are still asked. A window that waits ends the
/// run, which the caller discards.
fn spell_cast_countered(
    state: &mut EncounterState,
    stream: &mut Stream,
    caster: usize,
    spell_id: &str,
    level: u8,
    target_ids: &[String],
    events: &mut Vec<EngineEvent>,
) -> Result<bool, EngineError> {
    let caster_id = state.combatants[caster].id.clone();
    for window in spell_cast_windows(state, caster, spell_id, level, target_ids) {
        let reactor_id = window.reactor_id.clone();
        let id = match ask(state, window.clone(), events) {
            Some(ReactionAnswer::Take(id)) => id,
            Some(ReactionAnswer::Pass) => continue,
            None => return Ok(true),
        };
        let name = window
            .option(&id)
            .ok_or_else(|| EngineError::Invalid(format!("{reactor_id} has no reaction {id} for this cast")))?
            .name
            .clone();
        events.push(event("reaction", format!("REACTION {reactor_id}: {name} (cast by {caster_id})"), &reactor_id, &caster_id));
        state.rng = stream.state();
        state.reactions.casting = Some(spell_id.to_string());
        state.reactions.countered = false;
        let resolved = resolve_action(state, &reactor_id, &id, std::slice::from_ref(&caster_id), None, false);
        state.reactions.casting = None;
        events.extend(blame_reaction(state, &reactor_id, &id, resolved)?);
        *stream = Stream::from_state(state.rng.clone());
        if !state.player_dice.missing.is_empty() {
            // The counter's save waits on a player's die: hold the cast like a waiting window.
            return Ok(true);
        }
        let countered = std::mem::take(&mut state.reactions.countered);
        let clauses = stat_block(state, &state.combatants[index_of(state, &reactor_id)?])?
            .reactions
            .iter()
            .find(|a| a.id == id)
            .map(|a| a.agent_clauses.clone())
            .unwrap_or_default();
        for clause in clauses {
            events.push(event("agent_clause", format!("{reactor_id} {name}, agent clause: {clause}"), &reactor_id, &caster_id));
        }
        if countered {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The windows a leveled cast opens: one per enemy of the caster that can react and has a
/// ready reaction with a spell-cast trigger, in initiative order. Sight is not read.
fn spell_cast_windows(state: &EncounterState, caster: usize, spell_id: &str, level: u8, target_ids: &[String]) -> Vec<ReactionWindow> {
    let caster = &state.combatants[caster];
    let rank = |id: &str| state.initiative_order.iter().position(|x| x == id).unwrap_or(usize::MAX);
    let mut windows: Vec<ReactionWindow> = state
        .combatants
        .iter()
        .filter(|r| r.side_id != caster.side_id && can_react(state, r).is_ok())
        .filter_map(|r| {
            let options: Vec<ReactionOption> = stat_block(state, r)
                .ok()?
                .reactions
                .iter()
                .filter(|a| matches!(a.trigger, Some(ReactionTrigger::SpellCast { caster: SpellCaster::Enemy })))
                .filter(|a| ability_ready(state, &r.id, a))
                .map(|a| ReactionOption { id: a.id.clone(), name: a.name.clone() })
                .collect();
            (!options.is_empty()).then(|| ReactionWindow {
                reactor_id: r.id.clone(),
                trigger: WindowTrigger::SpellCast { caster_id: caster.id.clone(), spell_id: spell_id.to_string(), level, target_ids: target_ids.to_vec() },
                options,
            })
        })
        .collect();
    windows.sort_by_key(|w| rank(&w.reactor_id));
    windows
}

/// The level a cast resolves at: its slot level, or the spell's own level when it names none.
fn cast_level(
    spell: &crate::spell::SpellEffect,
    cast: &crate::effect::Cast,
) -> Result<u8, EngineError> {
    match cast.slot_level {
        None => Ok(spell.level),
        Some(_) if spell.level == 0 => Err(EngineError::Invalid(format!(
            "{} is a cantrip and takes no spell slot",
            spell.id
        ))),
        Some(slot) if slot < spell.level => Err(EngineError::Invalid(format!(
            "{} is a level {} spell and cannot be cast with a level {slot} slot",
            spell.id, spell.level
        ))),
        Some(slot) => Ok(slot),
    }
}

/// Cast level of a cast the effects file may not hold; an unknown spell keeps its slot level.
fn cast_level_of(cast: &crate::effect::Cast) -> Result<u8, EngineError> {
    match load_spell_effects()?.get(&cast.spell_id) {
        Some(spell) => cast_level(spell, cast),
        None => Ok(cast.slot_level.unwrap_or(0)),
    }
}

/// The effects a spell resolves at `cast_level`, with each scaling rule applied once
/// per level above the spell's own (ADR 0014).
fn scaled_effects(
    spell: &crate::spell::SpellEffect,
    cast_level: u8,
) -> Result<Vec<Effect>, EngineError> {
    use crate::spell::Scaling;
    let steps = u32::from(cast_level.saturating_sub(spell.level));
    let mut effects = spell.effects.clone();
    if steps == 0 {
        return Ok(effects);
    }
    let missing = |index: usize| {
        EngineError::Invalid(format!("{} scaling names missing effect {index}", spell.id))
    };
    for rule in &spell.scaling {
        let effect = effects
            .get_mut(rule.effect_index)
            .ok_or_else(|| missing(rule.effect_index))?;
        match &rule.per_slot {
            Scaling::ExtraDice { dice, damage_type } => add_dice(effect, dice, *damage_type, steps)
                .ok_or_else(|| {
                    EngineError::Invalid(format!(
                        "{} cannot add {dice} to effect {}",
                        spell.id, rule.effect_index
                    ))
                })?,
            Scaling::ExtraModifier { amount } => add_modifier(effect, amount * steps as i32),
            Scaling::ExtraTargets { count } => add_targets(effect, count * steps),
            Scaling::ExtraEffects { .. } => {}
        }
    }
    for rule in &spell.scaling {
        if let Scaling::ExtraEffects { count } = rule.per_slot {
            let copy = effects
                .get(rule.effect_index)
                .cloned()
                .ok_or_else(|| missing(rule.effect_index))?;
            effects.extend(std::iter::repeat(copy).take((count * steps) as usize));
        }
    }
    Ok(effects)
}

fn grow_dice(base: &str, extra: &str, times: u32) -> Option<String> {
    let (count, sides) = crate::effect::parse_dice(base)?;
    let (more, extra_sides) = crate::effect::parse_dice(extra)?;
    (sides == extra_sides).then(|| format!("{}d{sides}", count + more * times))
}

fn add_dice(
    effect: &mut Effect,
    dice: &str,
    damage_type: Option<DamageType>,
    times: u32,
) -> Option<()> {
    let pick = |damage: &mut Vec<crate::effect::Damage>| -> Option<()> {
        let index = damage
            .iter()
            .position(|d| Some(d.damage_type) == damage_type)
            .unwrap_or(0);
        let entry = damage.get_mut(index)?;
        entry.dice = grow_dice(&entry.dice, dice, times)?;
        Some(())
    };
    match effect {
        Effect::Attack(a) => pick(&mut a.damage),
        Effect::SavingThrow(s) => pick(&mut s.damage),
        Effect::Damage(d) => {
            d.dice = grow_dice(&d.dice, dice, times)?;
            Some(())
        }
        Effect::Heal(h) => {
            h.dice = Some(match &h.dice {
                Some(base) => grow_dice(base, dice, times)?,
                None => grow_dice(dice, dice, times - 1)?,
            });
            Some(())
        }
        Effect::TempHp(t) => {
            t.dice = grow_dice(&t.dice, dice, times)?;
            Some(())
        }
        Effect::Area(area) => add_dice(&mut area.effect, dice, damage_type, times),
        _ => None,
    }
}

fn add_modifier(effect: &mut Effect, amount: i32) {
    match effect {
        Effect::Heal(h) => h.modifier += amount,
        Effect::TempHp(t) => t.modifier += amount,
        Effect::Damage(d) => d.modifier += amount,
        Effect::Attack(crate::effect::Attack { damage, .. })
        | Effect::SavingThrow(crate::effect::SavingThrow { damage, .. }) => {
            if let Some(first) = damage.first_mut() {
                first.modifier += amount;
            }
        }
        Effect::Area(area) => add_modifier(&mut area.effect, amount),
        _ => {}
    }
}

fn add_targets(effect: &mut Effect, count: u32) {
    match effect {
        Effect::Attack(a) => a.target_count += count,
        Effect::Area(area) => {
            if let Some(n) = area.target_count.as_mut() {
                *n += count;
            }
        }
        _ => {}
    }
}

/// A PC with spell slots can cast a leveled spell only while a slot of that level is left.
fn spend_slot(actor: &Combatant, block: &StatBlock, level: u8) -> Result<(), EngineError> {
    let Some(slots) = block.spell_slots.as_ref().filter(|_| level > 0) else {
        return Ok(());
    };
    let max = slots.get(usize::from(level) - 1).copied().unwrap_or(0) as i32;
    let used = actor.resources.get(&format!("{}:slot:{level}", actor.id)).copied().unwrap_or(0);
    if used >= max {
        return Err(EngineError::Illegal(format!("{} has no level {level} spell slot left", actor.id)));
    }
    Ok(())
}

/// Name the reaction behind a failed resolution on `state.reactions.fault`; the innermost one wins.
fn blame_reaction<T>(state: &mut EncounterState, reactor_id: &str, ability_id: &str, result: Result<T, EngineError>) -> Result<T, EngineError> {
    if result.is_err() && state.reactions.fault.is_none() {
        state.reactions.fault = Some(crate::reaction::ReactionFault { reactor_id: reactor_id.into(), ability_id: ability_id.into() });
    }
    result
}

fn load_spell_effects() -> Result<&'static HashMap<String, crate::spell::SpellEffect>, EngineError> {
    crate::spell::catalog().map_err(|e| EngineError::Invalid(e.to_string()))
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

fn casts_in(block: &StatBlock) -> Vec<crate::effect::Cast> {
    serde_json::to_value(block).map(|v| crate::effect::casts_in(&v)).unwrap_or_default()
}

/// Spell ids a stat block casts that the spell effects file does not hold. A cast
/// of one is logged but does nothing, so a batch names each one.
pub fn missing_spells(block: &StatBlock) -> Vec<String> {
    let Ok(known) = load_spell_effects() else {
        return vec![];
    };
    let missing: std::collections::BTreeSet<String> = casts_in(block)
        .into_iter()
        .map(|c| c.spell_id)
        .filter(|id| !known.contains_key(id))
        .collect();
    missing.into_iter().collect()
}

/// Upcasts in a stat block whose spell has an `upcast_clause` the engine does not
/// apply: spell id, cast level and clause, once each.
pub fn unapplied_upcasts(block: &StatBlock) -> Vec<(String, u8, String)> {
    let Ok(known) = load_spell_effects() else {
        return vec![];
    };
    let found: std::collections::BTreeSet<(String, u8, String)> = casts_in(block)
        .into_iter()
        .filter_map(|cast| {
            let spell = known.get(&cast.spell_id)?;
            let level = cast_level(spell, &cast).ok()?;
            let clause = spell
                .upcast_clause
                .clone()
                .filter(|_| level > spell.level)?;
            Some((cast.spell_id, level, clause))
        })
        .collect();
    found.into_iter().collect()
}

/// Effects of a cast with the caster's numbers bound in, and whether the spell needs concentration.
pub fn bound_cast_effects(block: &StatBlock, cast: &crate::effect::Cast) -> Option<(Vec<Effect>, bool)> {
    let spell = load_spell_effects().ok()?.get(&cast.spell_id)?;
    let cantrip = spell.level == 0;
    let effects = scaled_effects(spell, cast_level(spell, cast).ok()?).ok()?;
    let effects = match cast_context(block, cantrip) {
        Some(ctx) => effects
            .into_iter()
            .map(|e| bind_effect(e, &ctx, cantrip))
            .collect(),
        None => effects,
    };
    Some((effects, spell.concentration))
}

/// Whether the ability can be used right now: its usage allows it and any spell slot is left.
pub fn ability_ready(state: &EncounterState, actor_id: &str, ability: &crate::stat_block::AbilityEntry) -> bool {
    let Ok(i) = index_of(state, actor_id) else { return false };
    let actor = &state.combatants[i];
    let Ok(block) = stat_block(state, actor) else { return false };
    let left = |usage: &Usage, unspent: i32| actor.resources.get(&usage_key(actor_id, &ability.id, usage)).copied().unwrap_or(unspent);
    let ready = match &ability.usage {
        None | Some(Usage::AtWill) => true,
        Some(usage @ (Usage::Recharge { .. } | Usage::CooldownUntilNextTurn)) => left(usage, 1) > 0,
        Some(usage @ (Usage::PerDay { uses, .. } | Usage::PerRest { uses, .. })) => left(usage, 0) < *uses as i32,
    };
    ready
        && ability.effects.iter().all(|e| match e {
            Effect::Cast(c) => cast_level_of(c)
                .and_then(|level| spend_slot(actor, block, level))
                .is_ok(),
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

fn cast_context(block: &StatBlock, cantrip: bool) -> Option<CastContext> {
    let sc = block.spellcasting.as_ref()?;
    let dice_multiplier = match (cantrip, block.level) {
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
        Effect::CounterSpell(mut counter) => {
            if counter.dc == 0 {
                counter.dc = ctx.save_dc;
            }
            Effect::CounterSpell(counter)
        }
        Effect::Heal(mut h) => {
            if h.spellcasting_modifier {
                h.modifier += ctx.heal_bonus;
            }
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
    opportunity: &[OpportunityAttack],
    declined: &[String],
) -> Result<Vec<EngineEvent>, EngineError> {
    let i = index_of(state, actor_id)?;
    // (combatant id, whether it prices the move)
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
    let provoked: Vec<String> = if band == RangeBand::Engaged {
        vec![]
    } else {
        others
            .iter()
            .map(|(id, _)| id.clone())
            .filter(|id| state.combatants[i].engagement.contains(id))
            .collect()
    };
    let mut attacks = opportunity
        .iter()
        .enumerate()
        .map(|(n, oa)| {
            if opportunity[..n].iter().any(|earlier| earlier.reactor_id == oa.reactor_id) {
                return Err(EngineError::Invalid(format!("{} is named twice", oa.reactor_id)));
            }
            let r = index_of(state, &oa.reactor_id)?;
            if !provoked.contains(&oa.reactor_id) {
                return Err(EngineError::Illegal(format!(
                    "{} cannot make an Opportunity Attack: this move does not leave its reach",
                    oa.reactor_id
                )));
            }
            opportunity_eligibility(state, r, i).map_err(|why| {
                EngineError::Illegal(format!("{} cannot make an Opportunity Attack: {}", oa.reactor_id, why.reason()))
            })?;
            let attack = opportunity_attack_action(stat_block(state, &state.combatants[r])?, oa.attack_id.as_deref())
                .map_err(|why| EngineError::Illegal(format!("{} cannot make an Opportunity Attack: {why}", oa.reactor_id)))?;
            if let Some(usage) = &attack.usage {
                spend_usage(&mut state.clone(), r, &attack.id, usage)?;
            }
            Ok((r, attack))
        })
        .collect::<Result<Vec<_>, EngineError>>()?;
    let mut attacking: Vec<String> = opportunity.iter().map(|oa| oa.reactor_id.clone()).collect();
    let mut reasons = Vec::new();
    let mut held: Vec<String> = Vec::new();
    if state.settings.reaction_windows {
        let open: Vec<String> = provoked.iter().filter(|id| !attacking.contains(id) && !declined.contains(id)).cloned().collect();
        for window in opportunity_windows(state, i, &open) {
            let logged = reasons.len();
            match ask(state, window.clone(), &mut reasons) {
                None => return Ok(vec![]),
                Some(ReactionAnswer::Pass) if reasons.len() > logged => held.push(window.reactor_id),
                Some(ReactionAnswer::Pass) => {}
                Some(ReactionAnswer::Take(id)) => {
                    let reactor_id = window.reactor_id;
                    if window.options.iter().all(|o| o.id != id) {
                        return Err(EngineError::Invalid(format!("{reactor_id} has no Opportunity Attack {id}")));
                    }
                    let r = index_of(state, &reactor_id)?;
                    let attack = opportunity_attack_action(stat_block(state, &state.combatants[r])?, Some(&id))
                        .map_err(|why| EngineError::Illegal(format!("{reactor_id} cannot make an Opportunity Attack: {why}")))?;
                    attacks.push((r, attack));
                    attacking.push(reactor_id);
                }
            }
        }
    }

    let mut events = reasons;
    let mut stream = Stream::from_state(state.rng.clone());
    for (r, attack) in attacks {
        let reactor_id = state.combatants[r].id.clone();
        if let Some(usage) = &attack.usage {
            let spent = spend_usage(state, r, &attack.id, usage);
            blame_reaction(state, &reactor_id, &attack.id, spent)?;
        }
        events.push(event("reaction", format!("OPPORTUNITY ATTACK {reactor_id} on {actor_id} (reaction)"), &reactor_id, actor_id));
        for effect in attack.effects {
            let resolved = resolve_effect(state, &mut stream, &reactor_id, effect, &[actor_id.to_string()], &mut events);
            blame_reaction(state, &reactor_id, &attack.id, resolved)?;
        }
        state.combatants[r].action_economy.reaction_used = true;
        if !can_act(&state.combatants[i]) {
            state.rng = stream.state();
            events.push(event("move_stopped", format!("{actor_id} does not finish its move"), actor_id, actor_id));
            return Ok(events);
        }
    }
    state.rng = stream.state();

    state.combatants[i].action_economy.movement_used_feet += cost;
    events.push(event("movement", format!("{actor_id} moves to {}", format!("{band:?}").to_lowercase()), actor_id, target_id.unwrap_or(actor_id)));
    for (other, _) in &others {
        set_band(state, i, actor_id, other, band)?;
    }
    for reactor_id in provoked.iter().filter(|id| !attacking.contains(id) && !declined.contains(id) && !held.contains(id)) {
        let r = index_of(state, reactor_id)?;
        match opportunity_eligibility(state, r, i) {
            Ok(()) => events.push(event(
                "opportunity_attack",
                format!("{reactor_id} may use an Opportunity Attack on {actor_id}"),
                reactor_id,
                actor_id,
            )),
            Err(Ineligible::Unseen(why)) => events.push(event(
                "assumption",
                format!("ASSUMPTION {why}, so it makes no Opportunity Attack on {actor_id}"),
                reactor_id,
                actor_id,
            )),
            Err(Ineligible::Rule(_)) => {}
        }
    }
    Ok(events)
}

/// Why a creature may not make an Opportunity Attack against a mover.
enum Ineligible {
    Rule(String),
    /// The rule needs a creature the reactor can see; the engine reads sight
    /// through ADR 0017's one rule, which assumes line of sight otherwise.
    Unseen(String),
}

impl Ineligible {
    fn reason(&self) -> String {
        match self {
            Ineligible::Rule(why) => why.clone(),
            Ineligible::Unseen(why) => format!("{why} (assumption)"),
        }
    }
}

/// The one eligibility rule behind both the `opportunity_attack` notice and
/// the resolver, so the notice never names a reactor the resolver would refuse.
/// A mover that took Disengage this turn, or has a no-provoke trait (Flyby,
/// Agile), provokes nothing. Flyby is not limited to flying out of reach.
fn opportunity_eligibility(state: &EncounterState, reactor: usize, mover: usize) -> Result<(), Ineligible> {
    let r = &state.combatants[reactor];
    let m = &state.combatants[mover];
    if r.side_id == m.side_id {
        return Err(Ineligible::Rule(format!("{} is on {}'s side", r.id, m.id)));
    }
    can_react(state, r).map_err(Ineligible::Rule)?;
    if m.action_economy.disengaged {
        return Err(Ineligible::Rule(format!("{} has disengaged", m.id)));
    }
    if let Some(block) = state.stat_blocks.iter().find(|s| s.id == m.stat_block_id) {
        if let Some(t) = block.stat_block.no_provoke_ability() {
            return Err(Ineligible::Rule(format!("{} does not provoke ({})", m.id, t.name)));
        }
    }
    match unseen_reason(state, r, m) {
        Some(why) => Err(Ineligible::Unseen(why)),
        None => Ok(()),
    }
}

/// Whether `reactor_id` may make an Opportunity Attack on `mover_id` under the
/// engine's one eligibility rule. The batch policy prices step-outs with it.
pub(crate) fn opportunity_eligible(state: &EncounterState, reactor_id: &str, mover_id: &str) -> bool {
    match (index_of(state, reactor_id), index_of(state, mover_id)) {
        (Ok(r), Ok(m)) => opportunity_eligibility(state, r, m).is_ok(),
        _ => false,
    }
}

/// The checks every reaction shares: the reactor can act, nothing forbids it
/// reactions, and its reaction is unspent.
fn can_react(state: &EncounterState, r: &Combatant) -> Result<(), String> {
    if !can_act(r) {
        return Err(format!("{} cannot act", r.id));
    }
    if state.modifiers_on(&r.id).any(|x| x.target == ModifierTarget::Reactions) {
        return Err(format!("{} cannot take reactions", r.id));
    }
    if r.action_economy.reaction_used {
        return Err(format!("{} has already used its reaction", r.id));
    }
    Ok(())
}

/// The Opportunity Attack windows a move opens: one per reactor in `reactors`
/// that passes [`opportunity_eligibility`] and has a melee attack to make, in
/// initiative order. The default attack is offered first, then every other
/// ready single melee attack in stat-block order.
fn opportunity_windows(state: &EncounterState, mover: usize, reactors: &[String]) -> Vec<ReactionWindow> {
    let mover_id = &state.combatants[mover].id;
    let mut windows: Vec<ReactionWindow> = reactors
        .iter()
        .filter_map(|reactor_id| {
            let r = index_of(state, reactor_id).ok()?;
            opportunity_eligibility(state, r, mover).ok()?;
            let block = stat_block(state, &state.combatants[r]).ok()?;
            let first = opportunity_attack_action(block, None).ok();
            let others = block.actions.iter().filter(|a| {
                is_melee_attack(a) && !matches!(&first, Some(f) if f.id == a.id) && ability_ready(state, reactor_id, a)
            });
            let options: Vec<ReactionOption> = first.iter().chain(others).map(|a| ReactionOption { id: a.id.clone(), name: a.name.clone() }).collect();
            (!options.is_empty()).then(|| ReactionWindow {
                reactor_id: reactor_id.clone(),
                trigger: WindowTrigger::Opportunity { mover_id: mover_id.clone() },
                options,
            })
        })
        .collect();
    let rank = |id: &str| state.initiative_order.iter().position(|x| x == id).unwrap_or(usize::MAX);
    windows.sort_by_key(|w| rank(&w.reactor_id));
    windows
}

/// The window a hit opens for the creature hit: its ready reactions whose hit
/// trigger matches the attack, if it can react at all. A critical hit opens it too.
fn hit_window(state: &EncounterState, target: usize, attack: &crate::effect::Attack, trigger: WindowTrigger) -> Option<ReactionWindow> {
    let reactor = &state.combatants[target];
    if matches!(&trigger, WindowTrigger::Hit { attacker_id, .. } if *attacker_id == reactor.id) {
        return None;
    }
    can_react(state, reactor).ok()?;
    let options: Vec<ReactionOption> = stat_block(state, reactor)
        .ok()?
        .reactions
        .iter()
        .filter(|a| matches!(a.trigger, Some(ReactionTrigger::HitByAttackRoll { attack: filter }) if filter.matches(attack)))
        .filter(|a| ability_ready(state, &reactor.id, a))
        .map(|a| ReactionOption { id: a.id.clone(), name: a.name.clone() })
        .collect();
    (!options.is_empty()).then(|| ReactionWindow { reactor_id: reactor.id.clone(), trigger, options })
}

/// Ask the encounter's decider about one window. `None` means the window waits
/// for an answer; the engine records it, and the caller throws the run away and
/// prompts. Once a prompt is owed, by an earlier window or a missing PC die,
/// every later window passes unasked. A decider's reason becomes a
/// `reaction_reason` event.
fn ask(state: &mut EncounterState, window: ReactionWindow, events: &mut Vec<EngineEvent>) -> Option<ReactionAnswer> {
    if state.reactions.waiting.is_some() {
        return None;
    }
    if !state.player_dice.missing.is_empty() {
        return Some(ReactionAnswer::Pass);
    }
    let mut decider = std::mem::take(&mut state.reactions.decider);
    let decision = decider.decide(state, &window);
    state.reactions.decider = decider;
    match decision {
        crate::reaction::Decision::Answer { answer, reason } => {
            if let Some(reason) = reason {
                events.push(event("reaction_reason", reason, &window.reactor_id, &window.reactor_id));
            }
            Some(answer)
        }
        crate::reaction::Decision::Wait => {
            state.reactions.waiting = Some(window);
            None
        }
    }
}

/// End what lasted only for the attack against `target_id` (Parry's AC).
fn end_triggering_attack(state: &mut EncounterState, stream: &mut Stream, target_id: &str, events: &mut Vec<EngineEvent>) -> Result<(), EngineError> {
    let ends = |d: &Duration, source: &Option<String>| matches!(d, Duration::TriggeringAttack) && source.as_deref() == Some(target_id);
    let ended = take_modifiers(state, |m| ends(&m.remaining, &m.source));
    for c in &mut state.combatants {
        c.conditions.retain(|x| !ends(&x.duration, &x.source));
    }
    fire_on_end(state, stream, ended, events)
}

fn take_modifiers(state: &mut EncounterState, ends: impl Fn(&ActiveModifier) -> bool) -> Vec<ActiveModifier> {
    let (ended, kept) = std::mem::take(&mut state.active_modifiers).into_iter().partition(|m| ends(m));
    state.active_modifiers = kept;
    ended
}

fn fire_on_end(state: &mut EncounterState, stream: &mut Stream, ended: Vec<ActiveModifier>, events: &mut Vec<EngineEvent>) -> Result<(), EngineError> {
    for m in ended {
        let Some(bearer) = m.bearer().map(str::to_string) else { continue };
        if m.on_end.is_empty() || index_of(state, &bearer).is_err() {
            continue;
        }
        let source = m.source.clone().unwrap_or_else(|| bearer.clone());
        let what = m.source_ability.as_deref().unwrap_or("a timed modifier");
        events.push(event("modifier_end", format!("{what} ends on {bearer}"), &source, &bearer));
        for effect in m.on_end {
            resolve_effect(state, stream, &source, effect, std::slice::from_ref(&bearer), events)?;
        }
    }
    Ok(())
}

/// One melee attack: an action whose only effect is an Attack with no range.
fn is_melee_attack(action: &crate::stat_block::AbilityEntry) -> bool {
    matches!(action.effects.as_slice(), [Effect::Attack(a)] if a.range.is_none())
}

/// The attack a reactor uses. A named action may be limited-use; the default is
/// the first at-will melee attack in stat-block order, reading Multiattack steps
/// in place (first Choice option first) and skipping steps that cast.
fn opportunity_attack_action(
    block: &StatBlock,
    attack_id: Option<&str>,
) -> Result<crate::stat_block::AbilityEntry, String> {
    if let Some(id) = attack_id {
        let action = block.actions.iter().find(|a| a.id == id).ok_or_else(|| format!("action {id} not found"))?;
        if !is_melee_attack(action) {
            return Err(format!("{id} is not a single melee attack"));
        }
        return Ok(action.clone());
    }
    opportunity_attack_candidates(block)
        .into_iter()
        .next()
        .cloned()
        .ok_or_else(|| "it has no at-will melee attack".to_string())
}

/// Every at-will single melee attack a reactor could make as its Opportunity
/// Attack, in the order [`opportunity_attack_action`] reads them: each action
/// in stat-block order, then the leaves of its Multiattack steps (first Choice
/// option first), skipping steps that cast. The first is the default attack.
pub(crate) fn opportunity_attack_candidates(block: &StatBlock) -> Vec<&crate::stat_block::AbilityEntry> {
    let at_will = |id: &str| {
        block
            .actions
            .iter()
            .find(|a| a.id == id && is_melee_attack(a) && matches!(a.usage, None | Some(Usage::AtWill)))
    };
    fn leaves<'a>(steps: &'a [crate::effect::MultiattackStep], out: &mut Vec<&'a str>) {
        use crate::effect::MultiattackStep;
        for step in steps {
            match step {
                MultiattackStep::Action { action_id, cast: None, .. } => out.push(action_id),
                MultiattackStep::Action { .. } => {}
                MultiattackStep::Choice { options, .. } => {
                    out.extend(options.iter().filter(|o| o.cast.is_none()).map(|o| o.action_id.as_str()))
                }
                MultiattackStep::Alternatives { sets } => sets.iter().for_each(|set| leaves(set, out)),
            }
        }
    }
    let mut found: Vec<&crate::stat_block::AbilityEntry> = Vec::new();
    for action in &block.actions {
        let mut ids = vec![action.id.as_str()];
        for effect in &action.effects {
            if let Effect::Multiattack(multi) = effect {
                leaves(&multi.steps, &mut ids);
            }
        }
        for attack in ids.into_iter().filter_map(at_will) {
            if !found.iter().any(|f| f.id == attack.id) {
                found.push(attack);
            }
        }
    }
    found
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
) -> Result<(), EngineError> {
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
    }
    Ok(())
}

fn end_turn_events(state: &mut EncounterState, legendary: &[LegendaryUse]) -> Result<Vec<EngineEvent>, EngineError> {
    if legendary.is_empty() {
        let mut events = finish_turn(state)?;
        events.extend(start_next_turn(state)?);
        return Ok(events);
    }
    let mut work = state.clone();
    let mut events = finish_turn(&mut work)?;
    events.extend(legendary_window(&mut work, legendary)?);
    events.extend(start_next_turn(&mut work)?);
    *state = work;
    Ok(events)
}

/// The first half of an end-turn: end-of-turn effects for the active creature.
/// The active turn stays set, so the legendary window that follows knows whose
/// turn ended. Call [`start_next_turn`] after it.
pub fn finish_turn(state: &mut EncounterState) -> Result<Vec<EngineEvent>, EngineError> {
    let current = state.active_turn.clone().ok_or_else(|| EngineError::Invalid("encounter has no active turn".into()))?;
    let mut events = vec![event("end_turn", format!("{current} ends its turn"), &current, &current)];
    if let Ok(i) = index_of(state, &current) {
        state.combatants[i].action_economy.disengaged = false;
    }
    let mut ended = end_of_turn(state, &current, &mut events);
    ended.extend(decrement_durations(state, &mut events));
    if ended.iter().any(|m| !m.on_end.is_empty()) {
        let mut stream = Stream::from_state(state.rng.clone());
        fire_on_end(state, &mut stream, ended, &mut events)?;
        state.rng = stream.state();
    }
    Ok(events)
}

fn end_of_turn(state: &mut EncounterState, bearer: &str, events: &mut Vec<EngineEvent>) -> Vec<ActiveModifier> {
    let ended = take_modifiers(state, |m| matches!(m.remaining, Duration::UntilEndOfTurn) && m.bearer() == Some(bearer));
    for _ in &ended {
        events.push(event("modifier_expire", format!("a timed modifier on {bearer} expired"), bearer, bearer));
    }
    if let Ok(i) = index_of(state, bearer) {
        let combatant = &mut state.combatants[i];
        let (gone, kept) = std::mem::take(&mut combatant.conditions).into_iter().partition::<Vec<_>, _>(|c| matches!(c.duration, Duration::UntilEndOfTurn));
        combatant.conditions = kept;
        for condition in gone {
            events.push(event("condition_expire", format!("{} loses {}", combatant.name, format!("{:?}", condition.condition).to_lowercase()), bearer, bearer));
        }
    }
    ended
}

fn start_of_turn(state: &mut EncounterState, bearer: &str) {
    let next = |d: &mut Duration| {
        if matches!(d, Duration::UntilEndOfNextTurn) {
            *d = Duration::UntilEndOfTurn;
        }
    };
    for m in state.active_modifiers.iter_mut().filter(|m| m.bearer() == Some(bearer)) {
        next(&mut m.remaining);
    }
    if let Ok(i) = index_of(state, bearer) {
        state.combatants[i].conditions.iter_mut().for_each(|c| next(&mut c.duration));
    }
}

fn legendary_window(state: &mut EncounterState, uses: &[LegendaryUse]) -> Result<Vec<EngineEvent>, EngineError> {
    let mut scratch = state.clone();
    for u in uses {
        for target in &u.target_ids {
            index_of(state, target)?;
        }
        spend_legendary(&mut scratch, &u.holder_id, &u.action_id)?;
    }
    let mut events = Vec::new();
    let mut lost: Vec<&str> = Vec::new();
    for u in uses {
        if lost.contains(&u.holder_id.as_str()) {
            continue;
        }
        if !can_act(&state.combatants[index_of(state, &u.holder_id)?]) {
            events.push(event(
                "legendary_lost",
                format!("{} can no longer act and loses its remaining legendary actions", u.holder_id),
                &u.holder_id,
                &u.holder_id,
            ));
            lost.push(&u.holder_id);
            continue;
        }
        events.extend(use_legendary_action(state, &u.holder_id, &u.action_id, &u.target_ids)?);
    }
    Ok(events)
}

/// The second half of an end-turn: pass over creatures that are down, advance
/// the round when the order wraps, and run start-of-turn effects for the next
/// creature.
pub fn start_next_turn(state: &mut EncounterState) -> Result<Vec<EngineEvent>, EngineError> {
    let current = state.active_turn.clone().ok_or_else(|| EngineError::Invalid("encounter has no active turn".into()))?;
    let mut events = Vec::new();
    let current_index = state.initiative_order.iter().position(|id| id == &current).unwrap_or(0);
    let order_len = state.initiative_order.len().max(1);
    let mut next_index = (current_index + 1) % order_len;
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
        let ended = expire_long_durations(state, &mut events);
        if ended.iter().any(|m| !m.on_end.is_empty()) {
            let mut stream = Stream::from_state(state.rng.clone());
            fire_on_end(state, &mut stream, ended, &mut events)?;
            state.rng = stream.state();
        }
    }
    state.active_turn = state.initiative_order.get(next_index).cloned();
    if let Some(next) = state.active_turn.clone() {
        if let Ok(i) = index_of(state, &next) {
            state.combatants[i].action_economy = ActionEconomy::default();
            start_of_turn(state, &next);
            reset_cooldowns(state, &next);
            roll_recharge(state, &next, &mut events)?;
            roll_death_saves(state, &next, &mut events)?;
            process_repeat_saves(state, &next, &mut events)?;
        }
        events.push(event("start_turn", format!("{next} starts its turn"), &next, &next));
    }
    Ok(events)
}

fn decrement_durations(state: &mut EncounterState, events: &mut Vec<EngineEvent>) -> Vec<ActiveModifier> {
    let long = state.settings.long_durations;
    let mut ended = Vec::new();
    state.active_modifiers.retain_mut(|modifier| {
        let keep = match &mut modifier.remaining {
            Duration::Rounds(n) => {
                *n = n.saturating_sub(1);
                *n > 0
            }
            Duration::Minutes(_) | Duration::Hours(_) | Duration::Days(_) if long => true,
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
            Duration::Concentration | Duration::Instant | Duration::Permanent | Duration::UntilSave { .. } | Duration::TriggeringAttack | Duration::UntilEndOfNextTurn | Duration::UntilEndOfTurn => true,
        };
        if !keep {
            events.push(event("modifier_expire", "a timed modifier expired".into(), "engine", "engine"));
            ended.push(modifier.clone());
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
                Duration::Minutes(_) | Duration::Hours(_) | Duration::Days(_) if long => true,
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
                Duration::Concentration | Duration::Instant | Duration::Permanent | Duration::UntilSave { .. } | Duration::TriggeringAttack | Duration::UntilEndOfNextTurn | Duration::UntilEndOfTurn => true,
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
    ended.extend(expire_long_durations(state, events));
    ended
}

fn round_length(duration: &Duration) -> Option<u32> {
    match duration {
        Duration::Minutes(n) => Some(n.saturating_mul(10)),
        Duration::Hours(n) => Some(n.saturating_mul(600)),
        Duration::Days(n) => Some(n.saturating_mul(14_400)),
        _ => None,
    }
}

fn expire_long_durations(state: &mut EncounterState, events: &mut Vec<EngineEvent>) -> Vec<ActiveModifier> {
    let mut ended = Vec::new();
    if !state.settings.long_durations {
        return ended;
    }
    let round = state.round;
    let lapsed = |duration: &Duration, started: u32| round_length(duration).is_some_and(|len| round >= started.saturating_add(len));
    state.active_modifiers.retain(|modifier| {
        let keep = !lapsed(&modifier.remaining, modifier.started_round);
        if !keep {
            events.push(event("modifier_expire", "a timed modifier expired".into(), "engine", "engine"));
            ended.push(modifier.clone());
        }
        keep
    });
    for combatant in &mut state.combatants {
        let (gone, kept) = std::mem::take(&mut combatant.conditions).into_iter().partition::<Vec<_>, _>(|c| lapsed(&c.duration, c.started_round));
        combatant.conditions = kept;
        for condition in gone {
            events.push(event("condition_expire", format!("{} loses {}", combatant.name, format!("{:?}", condition.condition).to_lowercase()), &combatant.id, &combatant.id));
        }
    }
    ended
}

fn roll_recharge(state: &mut EncounterState, combatant_id: &str, events: &mut Vec<EngineEvent>) -> Result<(), EngineError> {
    let i = index_of(state, combatant_id)?;
    let block = stat_block(state, &state.combatants[i])?;
    let abilities: Vec<(String, String, Option<Usage>)> = block.actions.iter()
        .chain(block.traits.iter())
        .chain(block.bonus_actions.iter())
        .chain(block.reactions.iter())
        .chain(block.legendary_actions.iter())
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
    let roll = player_roll(state, combatant_id, 20, roll);
    // Only dice reach a death save: flat no-ability save modifiers stay off it so saves from
    // before dice refold unchanged (ticket 093).
    let (die, die_note) = bonus_dice(state, &mut stream, combatant_id, |t| matches!(t, ModifierTarget::SavingThrows { ability: None }))?;
    state.rng = stream.state();
    let marked = state.player_dice.all_entered_since(combatant_id);
    let total = roll as i32 + die;
    let message = if roll == 20 {
        state.combatants[i].hp = 1;
        state.combatants[i].end_state = EndState::Active;
        state.combatants[i].death_saves = DeathSaves::default();
        format!("{combatant_id} rolls a natural 20 and regains consciousness{die_note}")
    } else if roll == 1 {
        state.combatants[i].death_saves.failures += 2;
        format!("{combatant_id} rolls a natural 1: two failures{die_note}")
    } else if total >= 10 {
        state.combatants[i].death_saves.successes += 1;
        format!("{combatant_id} succeeds a death save{die_note}")
    } else {
        state.combatants[i].death_saves.failures += 1;
        format!("{combatant_id} fails a death save{die_note}")
    };
    let total = (!die_note.is_empty()).then_some(total);
    events.push(EngineEvent { roll: Some(roll), total, player_dice: marked, ..event("death_save", message, combatant_id, combatant_id) });
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
            if let Some(message) = automatic_save_failure(&state.combatants[i], ability) {
                events.push(event("repeat_save", message, combatant_id, combatant_id));
                continue;
            }
            let block = stat_block(state, &state.combatants[i])?;
            let modifier = block.saving_throws.get(&ability)
                .copied()
                .unwrap_or_else(|| block.ability_scores.modifier(ability));
            let (roll, note) = save_d20(state, &mut stream, combatant_id, ability)?;
            let (die, die_note) = save_dice(state, &mut stream, combatant_id, ability)?;
            let modifier = modifier + die;
            let dc = 10;
            let success = roll as i32 + modifier >= dc;
            let marked = state.player_dice.all_entered_since(combatant_id);
            events.push(EngineEvent {
                kind: "repeat_save".into(),
                message: format!("{combatant_id} {} repeat save DC {dc}{note}{die_note}", if success { "passes" } else { "fails" }),
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

/// The legendary uses `holder_id` has left for the window that follows the
/// active turn, or why it can take none. This is the one eligibility rule for
/// live declarations, the batch policy and `digest.legendary`. The creature
/// whose turn is ending is refused; the one whose turn comes next is not,
/// because its budget only resets when its turn starts.
pub fn legendary_uses_left(state: &EncounterState, holder_id: &str) -> Result<u32, String> {
    let c = state.combatants.iter().find(|c| c.id == holder_id).ok_or_else(|| format!("{holder_id} is not in the encounter"))?;
    let block = stat_block(state, c).map_err(|e| e.to_string())?;
    if block.legendary_uses_per_round == 0 || block.legendary_actions.is_empty() {
        return Err(format!("{holder_id} has no legendary actions"));
    }
    if state.active_turn.as_deref() == Some(holder_id) {
        return Err(format!("{holder_id} is the creature whose turn is ending"));
    }
    if !can_act(c) {
        return Err(format!("{holder_id} cannot act"));
    }
    let used = c.resources.get(&format!("{holder_id}:legendary")).copied().unwrap_or(0).max(0) as u32;
    if used >= block.legendary_uses_per_round {
        return Err(format!("{holder_id} has no legendary actions left this round"));
    }
    Ok(block.legendary_uses_per_round - used)
}

/// Every creature that may take a legendary action when the active turn ends,
/// in combatant order.
pub fn legendary_holders(state: &EncounterState) -> Vec<LegendaryHolder> {
    state
        .combatants
        .iter()
        .filter_map(|c| {
            let uses_left = legendary_uses_left(state, &c.id).ok()?;
            let block = stat_block(state, c).ok()?;
            let ready_actions = block
                .legendary_actions
                .iter()
                .filter(|a| ability_ready(state, &c.id, a))
                .map(|a| a.id.clone())
                .collect();
            Some(LegendaryHolder { id: c.id.clone(), uses_left, ready_actions })
        })
        .collect()
}

fn spend_legendary(
    state: &mut EncounterState,
    holder_id: &str,
    action_id: &str,
) -> Result<(crate::stat_block::AbilityEntry, u32, u32), EngineError> {
    let i = index_of(state, holder_id)?;
    let uses_left = legendary_uses_left(state, holder_id)
        .map_err(|why| EngineError::Illegal(format!("{holder_id} cannot take a legendary action: {why}")))?;
    let (action, budget) = {
        let block = stat_block(state, &state.combatants[i])?;
        let action = block.legendary_actions.iter().find(|a| a.id == action_id).cloned()
            .ok_or_else(|| EngineError::NotFound(format!("legendary action {action_id} not found")))?;
        (action, block.legendary_uses_per_round)
    };
    if let Some(usage) = &action.usage {
        spend_usage(state, i, action_id, usage)?;
    }
    let used = budget - uses_left + 1;
    state.combatants[i].resources.insert(format!("{holder_id}:legendary"), used as i32);
    Ok((action, used, budget))
}

/// Spend one legendary action in the window after the active turn ends. Every
/// legendary action costs one use, and its own usage (recharge, cooldown, uses
/// per day) is spent too.
pub fn use_legendary_action(
    state: &mut EncounterState,
    holder_id: &str,
    action_id: &str,
    target_ids: &[String],
) -> Result<Vec<EngineEvent>, EngineError> {
    let (action, used, budget) = spend_legendary(state, holder_id, action_id)?;
    let mut stream = Stream::from_state(state.rng.clone());
    let mut events = vec![event(
        "legendary",
        format!("LEGENDARY ACTION {holder_id}: {} ({used} of {budget})", action.name),
        holder_id,
        holder_id,
    )];
    for effect in action.effects {
        resolve_effect(state, &mut stream, holder_id, effect, target_ids, &mut events)?;
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
    let down = matches!(state.combatants[i].end_state, EndState::Unconscious | EndState::Stable);
    if down && amount > 0 {
        if state.combatants[i].end_state == EndState::Stable {
            state.combatants[i].end_state = EndState::Unconscious;
            events.push(event("destabilize", format!("{target_id} is no longer stable"), source_id.unwrap_or(target_id), target_id));
        }
        let remaining = soak_temp_hp(&mut state.combatants[i], amount);
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
        if remaining >= state.combatants[i].max_hp {
            return die_from_massive_damage(state, stream, target_id, events);
        }
        let failures = if critical { 2 } else { 1 };
        state.combatants[i].death_saves.failures += failures;
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
        let ended = take_modifiers(state, |m| m.ends_on_damage && m.bearer() == Some(target_id));
        if state.combatants[i].conditions.len() < before {
            events.push(event("condition_expire", format!("{target_id} is roused by the damage"), target_id, target_id));
        } else if !ended.is_empty() {
            events.push(event("modifier_expire", format!("the damage ends a timed modifier on {target_id}"), target_id, target_id));
        }
        fire_on_end(state, stream, ended, events)?;
    }
    let remaining = soak_temp_hp(&mut state.combatants[i], amount.max(0));
    state.combatants[i].hp -= remaining;
    let drops = state.combatants[i].hp <= 0 && state.combatants[i].end_state == EndState::Active;
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
    if remaining > 0 && !drops && is_concentrating(&state.combatants[i]) {
        // Constitution save DC = max(10, damage / 2) to maintain concentration
        let dc = (remaining / 2).max(10);
        let block = stat_block(state, &state.combatants[i])?;
        let modifier = block.saving_throws.get(&Ability::Con)
            .copied()
            .unwrap_or_else(|| block.ability_scores.modifier(Ability::Con));
        let (roll, note) = save_d20(state, stream, target_id, Ability::Con)?;
        let (die, die_note) = save_dice(state, stream, target_id, Ability::Con)?;
        let modifier = modifier + die;
        let success = roll as i32 + modifier >= dc;
        let marked = state.player_dice.all_entered_since(target_id);
        events.push(EngineEvent {
            kind: "concentration_save".into(),
            message: format!("{target_id} {} concentration save DC {dc}{note}{die_note}", if success { "passes" } else { "fails" }),
            actor_id: source_id.map(|s| s.into()),
            target_id: Some(target_id.into()),
            roll: Some(roll),
            total: Some(roll as i32 + modifier),
            damage: None,
            player_dice: marked,
        });
        if !success {
            break_concentration(state, stream, target_id, events)?;
        }
    }
    if drops {
        let dying = stat_block(state, &state.combatants[i])?.dying;
        if dying && -state.combatants[i].hp >= state.combatants[i].max_hp {
            return die_from_massive_damage(state, stream, target_id, events);
        } else if dying {
            state.combatants[i].hp = 0;
            state.combatants[i].end_state = EndState::Unconscious;
            state.combatants[i].death_saves = DeathSaves::default();
            events.push(event("unconscious", format!("{target_id} falls unconscious"), target_id, target_id));
        } else {
            state.combatants[i].end_state = EndState::Dead;
            events.push(event("dies", format!("{target_id} dies"), target_id, target_id));
        }
        break_concentration(state, stream, target_id, events)?;
    }
    Ok(())
}

fn soak_temp_hp(c: &mut Combatant, amount: i32) -> i32 {
    let absorbed = amount.min(c.temp_hp);
    c.temp_hp -= absorbed;
    amount - absorbed
}

fn die_from_massive_damage(state: &mut EncounterState, stream: &mut Stream, target_id: &str, events: &mut Vec<EngineEvent>) -> Result<(), EngineError> {
    let i = index_of(state, target_id)?;
    state.combatants[i].end_state = EndState::Dead;
    events.push(event("dies", format!("{target_id} dies from massive damage"), target_id, target_id));
    break_concentration(state, stream, target_id, events)
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

/// Roll disposition for an attack roll with `attack` from every source (PHB Appendix C and ch. 1,
/// read through ADR 0017): the actor's and target's conditions, Invisible on either side as sight
/// allows, a ranged attack in close combat, the actor's timed modifiers, and lighting roll
/// modifiers on its traits. "Within 5 feet" is the actor's engagement as the state holds it.
pub(crate) fn disposition(state: &EncounterState, actor: &Combatant, target: &Combatant, attack: &Attack) -> RollDisposition {
    disposition_with(state, actor, target, attack, Within::Held)
}

/// Who is within 5 feet of an attacker when it rolls (ADR 0017: engagement, never range bands).
/// The batch policy scores a plan from where the plan leaves the attacker.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Within<'a> {
    /// Engagement as the state holds it, recorded on either side.
    Held,
    /// Engagement as held, plus the combatant with this id (the attacker moves in to engage it).
    HeldAnd(&'a str),
    /// Nobody: the attacker steps out of every engagement first.
    Nobody,
}

impl Within<'_> {
    fn reaches(self, actor: &Combatant, other: &Combatant) -> bool {
        match self {
            Within::Held => within_5_feet(actor, other),
            Within::HeldAnd(id) => other.id == id || within_5_feet(actor, other),
            Within::Nobody => false,
        }
    }
}

/// `disposition` with "within 5 feet" read from `within` instead of the state's engagement, for
/// both the target's Prone and the close-combat rule.
pub(crate) fn disposition_with(state: &EncounterState, actor: &Combatant, target: &Combatant, attack: &Attack, within: Within) -> RollDisposition {
    use RollDisposition::{Advantage, Disadvantage};
    let has = |c: &Combatant, f: fn(Condition) -> bool| c.conditions.iter().any(|i| f(i.condition));
    let grappled_away = actor
        .conditions
        .iter()
        .any(|i| i.condition == Condition::Grappled && i.source.as_deref() != Some(target.id.as_str()));
    let frightened = actor.conditions.iter().any(|i| {
        i.condition == Condition::Frightened
            && match &i.source {
                None => true,
                Some(source) => state.combatants.iter().find(|c| &c.id == source).is_some_and(|c| sees(state, actor, c)),
            }
    });
    let conditions = [
        (has(actor, |c| matches!(c, Condition::Blinded | Condition::Poisoned | Condition::Prone | Condition::Restrained)), Disadvantage),
        (grappled_away || frightened, Disadvantage),
        (has(target, |c| matches!(c, Condition::Blinded | Condition::Paralyzed | Condition::Petrified | Condition::Restrained | Condition::Stunned)), Advantage),
        (counts_as_unconscious(target), Advantage),
        (counts_as_prone(target), if within.reaches(actor, target) { Advantage } else { Disadvantage }),
        (!sees(state, actor, target), Disadvantage),
        (!sees(state, target, actor), Advantage),
        (in_close_combat(state, actor, target, attack, within), Disadvantage),
    ];
    combined_disposition(
        conditions
            .into_iter()
            .filter_map(|(applies, d)| applies.then_some(d))
            .chain(modifier_dispositions(state, actor, ModifierTarget::AttackRolls))
            .chain(lighting_modifiers(state, actor, Roll::Attack).into_iter().map(|(d, _)| d)),
    )
}

const CLOSE_COMBAT: &str = "ranged attack in close combat";

/// "Ranged Attacks in Close Combat" (PHB ch. 1, p. 25): a ranged attack roll has Disadvantage while
/// an enemy within 5 feet of the attacker can see it and isn't Incapacitated. An enemy is a
/// combatant on another side; a dead or downed one can't act, so it never counts.
fn in_close_combat(state: &EncounterState, actor: &Combatant, target: &Combatant, attack: &Attack, within: Within) -> bool {
    attack.is_ranged(within.reaches(actor, target))
        && state
            .combatants
            .iter()
            .any(|e| e.side_id != actor.side_id && within.reaches(actor, e) && can_act(e) && sees(state, e, actor))
}

/// "Within 5 feet" (ADR 0017): either combatant holds the other in its engagement list.
/// Range bands are not read, because a missing band defaults to engaged.
fn within_5_feet(a: &Combatant, b: &Combatant) -> bool {
    a.engagement.contains(&b.id) || b.engagement.contains(&a.id)
}

/// The one sight rule (ADR 0017): `observer` sees `observed` unless `observed` is Invisible and
/// `observer` has no Blindsight, Truesight or See Invisible, `observer` is Blinded without Blindsight, or the
/// lighting is `darkness` and `observer` has no Darkvision, Blindsight or Truesight. Sense ranges are
/// ignored and Tremorsense doesn't see. Each refusal names one reason, in that order.
fn sees(state: &EncounterState, observer: &Combatant, observed: &Combatant) -> bool {
    unseen_reason(state, observer, observed).is_none()
}

fn unseen_reason(state: &EncounterState, observer: &Combatant, observed: &Combatant) -> Option<String> {
    let has = |c: &Combatant, condition: Condition| c.conditions.iter().any(|x| x.condition == condition);
    let block = stat_block(state, observer).ok();
    let senses = block.map(|b| &b.senses);
    let blindsight = senses.is_some_and(|s| s.blindsight.is_some());
    let truesight = senses.is_some_and(|s| s.truesight.is_some());
    let darkvision = senses.is_some_and(|s| s.darkvision.is_some());
    let see_invisible = || block.is_some_and(StatBlock::has_see_invisible) || state.modifiers_on(&observer.id).any(|m| m.target == ModifierTarget::SeeInvisible);
    if has(observed, Condition::Invisible) && !blindsight && !truesight && !see_invisible() {
        return Some(format!("{} cannot see {}, who is invisible", observer.id, observed.id));
    }
    if has(observer, Condition::Blinded) && !blindsight {
        return Some(format!("{} is blinded and cannot see {}", observer.id, observed.id));
    }
    if state.settings.lighting == Lighting::Darkness && !darkvision && !blindsight && !truesight {
        return Some(format!("{} cannot see {} in darkness", observer.id, observed.id));
    }
    None
}

/// A combatant counts as Unconscious for attacks when it has the condition or is down
/// (`end_state` Unconscious or Stable). Nothing is stored in `conditions` (ADR 0017).
pub(crate) fn counts_as_unconscious(c: &Combatant) -> bool {
    matches!(c.end_state, EndState::Unconscious | EndState::Stable) || c.conditions.iter().any(|i| i.condition == Condition::Unconscious)
}

/// Prone for attacks: the Prone condition, or anything that counts as Unconscious.
fn counts_as_prone(c: &Combatant) -> bool {
    counts_as_unconscious(c) || c.conditions.iter().any(|i| i.condition == Condition::Prone)
}

/// Why a hit on `target` is a Critical Hit whatever the roll: the target is Paralyzed or counts as
/// Unconscious and the attacker is within 5 feet of it.
fn automatic_critical(actor: &Combatant, target: &Combatant) -> Option<&'static str> {
    if !within_5_feet(actor, target) {
        return None;
    }
    if target.conditions.iter().any(|i| i.condition == Condition::Paralyzed) {
        Some("paralyzed")
    } else if counts_as_unconscious(target) {
        Some("unconscious")
    } else {
        None
    }
}

/// The event message when `saver` fails a save with `ability` without rolling: Paralyzed, Petrified,
/// Stunned and Unconscious (or downed, as for attacks) fail Strength and Dexterity saves.
pub(crate) fn automatic_save_failure(saver: &Combatant, ability: Ability) -> Option<String> {
    let word = match ability {
        Ability::Str => "Strength",
        Ability::Dex => "Dexterity",
        _ => return None,
    };
    let has = |condition: Condition| saver.conditions.iter().any(|i| i.condition == condition);
    let why = [(Condition::Paralyzed, "paralyzed"), (Condition::Petrified, "petrified"), (Condition::Stunned, "stunned")]
        .into_iter()
        .find(|(c, _)| has(*c))
        .map(|(_, why)| why)
        .or(counts_as_unconscious(saver).then_some("unconscious"))?;
    Some(format!("{} fails the save automatically ({why}, {word})", saver.id))
}

fn save_d20(state: &mut EncounterState, stream: &mut Stream, saver_id: &str, ability: Ability) -> Result<(u32, String), EngineError> {
    let saver = &state.combatants[index_of(state, saver_id)?];
    let sources = save_sources(state, saver, ability);
    let advantage = combined_disposition(sources.iter().map(|(d, _)| *d));
    let note = disposition_note(advantage, sources);
    let first = stream.roll_die(20);
    let first = player_roll(state, saver_id, 20, first);
    let roll = match advantage {
        RollDisposition::Normal => first,
        kept => {
            let second = stream.roll_die(20);
            let second = player_roll_in(state, saver_id, 20, second, false);
            if kept == RollDisposition::Advantage { first.max(second) } else { first.min(second) }
        }
    };
    Ok((roll, note))
}

pub(crate) fn save_disposition(state: &EncounterState, target: &Combatant, ability: Ability) -> RollDisposition {
    combined_disposition(save_sources(state, target, ability).into_iter().map(|(d, _)| d))
}

/// Every Advantage and Disadvantage source on a save, named for the event note: lighting traits,
/// Restrained on Dexterity, and the saver's timed modifiers. Legacy modifiers (saved before 0.1.6)
/// give none, as on 0.1.5.
fn save_sources(state: &EncounterState, target: &Combatant, ability: Ability) -> Vec<(RollDisposition, String)> {
    let mut sources = lighting_sources(state, target, Roll::Save(ability));
    if ability == Ability::Dex && target.conditions.iter().any(|i| i.condition == Condition::Restrained) {
        sources.push((RollDisposition::Disadvantage, "restrained".into()));
    }
    let timed = state.modifiers_on(&target.id).filter(|m| !m.is_legacy() && reaches(&m.target, Roll::Save(ability)));
    sources.extend(timed.filter_map(|m| {
        let name = m.source_ability.clone().unwrap_or_else(|| "timed modifier".into());
        match m.change {
            ModifierChange::Advantage => Some((RollDisposition::Advantage, name)),
            ModifierChange::Disadvantage => Some((RollDisposition::Disadvantage, name)),
            _ => None,
        }
    }));
    sources
}

#[derive(Debug, Clone, Copy)]
enum Roll {
    Attack,
    Save(Ability),
}

fn lighting_modifiers<'a>(state: &'a EncounterState, combatant: &Combatant, roll: Roll) -> Vec<(RollDisposition, &'a str)> {
    let lighting = state.settings.lighting;
    let Ok(block) = stat_block(state, combatant) else { return vec![] };
    if lighting == Lighting::Unspecified {
        return vec![];
    }
    block
        .traits
        .iter()
        .flat_map(|t| t.effects.iter().map(move |e| (t, e)))
        .filter_map(|(t, e)| match e {
            Effect::RollModifier(m) if lighting.meets(m.lighting) && reaches(&m.target, roll) => Some((m.disposition, t.name.as_str())),
            _ => None,
        })
        .collect()
}

fn reaches(target: &ModifierTarget, roll: Roll) -> bool {
    match (target, roll) {
        (ModifierTarget::AttackRolls, Roll::Attack) => true,
        (ModifierTarget::SavingThrows { ability }, Roll::Save(rolled)) => ability.map_or(true, |a| a == rolled),
        _ => false,
    }
}

fn lighting_sources(state: &EncounterState, combatant: &Combatant, roll: Roll) -> Vec<(RollDisposition, String)> {
    let light = state.settings.lighting.as_str().replace('_', " ");
    lighting_modifiers(state, combatant, roll).into_iter().map(|(d, name)| (d, format!("{name}, {light}"))).collect()
}

/// The event note for a roll made with `combined`: each distinct source that points the same way,
/// as ` (Advantage: Sun Glow, sunlight)`. A straight roll gets no note, even when sources cancelled.
fn disposition_note(combined: RollDisposition, sources: Vec<(RollDisposition, String)>) -> String {
    let word = match combined {
        RollDisposition::Advantage => "Advantage",
        RollDisposition::Disadvantage => "Disadvantage",
        RollDisposition::Normal => return String::new(),
    };
    let mut named: Vec<String> = vec![];
    for (_, name) in sources.into_iter().filter(|(d, _)| *d == combined) {
        if !named.contains(&name) {
            named.push(name);
        }
    }
    named.into_iter().map(|name| format!(" ({word}: {name})")).collect()
}

/// The PHB rule for Advantage and Disadvantage (glossary "Advantage", p. 359): a roll has at most one
/// of each however many sources grant them, and having both makes it a straight roll.
fn combined_disposition(sources: impl IntoIterator<Item = RollDisposition>) -> RollDisposition {
    let (mut advantage, mut disadvantage) = (false, false);
    for source in sources {
        match source {
            RollDisposition::Advantage => advantage = true,
            RollDisposition::Disadvantage => disadvantage = true,
            RollDisposition::Normal => {}
        }
    }
    match (advantage, disadvantage) {
        (true, false) => RollDisposition::Advantage,
        (false, true) => RollDisposition::Disadvantage,
        _ => RollDisposition::Normal,
    }
}

fn modifier_dispositions<'a>(
    state: &'a EncounterState,
    combatant: &'a Combatant,
    target: ModifierTarget,
) -> impl Iterator<Item = RollDisposition> + 'a {
    state
        .modifiers_on(&combatant.id)
        .filter(move |m| m.target == target)
        .filter_map(|m| match m.change {
            ModifierChange::Advantage => Some(RollDisposition::Advantage),
            ModifierChange::Disadvantage => Some(RollDisposition::Disadvantage),
            _ => None,
        })
}

/// Compute extra attack modifier from timed modifiers.
fn attack_modifier(state: &EncounterState, actor: &Combatant) -> Result<i32, EngineError> {
    Ok(added(state, &actor.id, |t| *t == ModifierTarget::AttackRolls))
}

/// Compute save modifier from timed modifiers.
pub(crate) fn save_modifier(state: &EncounterState, target: &Combatant, ability: Ability) -> i32 {
    added(state, &target.id, |t| reaches(t, Roll::Save(ability)))
}

/// Compute AC modifier from timed modifiers.
fn ac_modifier(state: &EncounterState, target: &Combatant) -> i32 {
    added(state, &target.id, |t| *t == ModifierTarget::ArmorClass)
}

/// Every flat and die bonus on `actor`'s attack rolls, with each die read as its mean, for the
/// policy's hit chance.
pub(crate) fn mean_attack_bonus(state: &EncounterState, actor: &Combatant) -> f64 {
    mean_added(state, &actor.id, |t| *t == ModifierTarget::AttackRolls)
}

/// Every flat and die bonus on `target`'s saves of `ability`, with each die read as its mean, for
/// the policy's save chance.
pub(crate) fn mean_save_bonus(state: &EncounterState, target: &Combatant, ability: Ability) -> f64 {
    mean_added(state, &target.id, |t| reaches(t, Roll::Save(ability)))
}

fn mean_added(state: &EncounterState, bearer: &str, on: impl Fn(&ModifierTarget) -> bool) -> f64 {
    state.modifiers_on(bearer).filter(|m| on(&m.target)).map(|m| m.change.mean_bonus()).sum()
}

/// Rolls each die modifier on `bearer` whose target `on` accepts, one per modifier in
/// `active_modifiers` order, for a roll whose d20s are already drawn. The bearer rolls, so a PC
/// bearer in ask mode is prompted. Returns the signed total and the event note, as
/// ` (+1d4 Bless: 3)`; nothing is drawn when no die applies.
fn bonus_dice(state: &mut EncounterState, stream: &mut Stream, bearer: &str, on: impl Fn(&ModifierTarget) -> bool) -> Result<(i32, String), EngineError> {
    let dice: Vec<(i32, String, String)> = state
        .modifiers_on(bearer)
        .filter(|m| !m.is_legacy() && on(&m.target))
        .filter_map(|m| {
            let (sign, dice) = m.change.die()?;
            Some((sign, dice.to_string(), m.source_ability.clone().unwrap_or_else(|| "timed modifier".into())))
        })
        .collect();
    let (mut total, mut note) = (0, String::new());
    for (sign, dice, name) in dice {
        let rolled = roll_dice(state, stream, bearer, &dice)? as i32;
        total += sign * rolled;
        note.push_str(&format!(" ({}{dice} {name}: {rolled})", if sign > 0 { '+' } else { '-' }));
    }
    Ok((total, note))
}

fn save_dice(state: &mut EncounterState, stream: &mut Stream, saver_id: &str, ability: Ability) -> Result<(i32, String), EngineError> {
    bonus_dice(state, stream, saver_id, |t| reaches(t, Roll::Save(ability)))
}

fn added(state: &EncounterState, bearer: &str, on: impl Fn(&ModifierTarget) -> bool) -> i32 {
    state
        .modifiers_on(bearer)
        .filter(|m| on(&m.target))
        .map(|m| match m.change {
            ModifierChange::Add(n) => n,
            _ => 0,
        })
        .sum()
}

/// Walk speed after the Speed modifiers on the combatant: the lowest `set` wins outright;
/// otherwise the stat block's walk speed plus every `add`, doubled once per `double` and then
/// halved once per `halve` (rounding down), never below 0. Grappled and Restrained give 0.
/// Legacy modifiers (saved before 0.1.6) change no Speed, as before.
pub(crate) fn walk_speed(state: &EncounterState, c: &Combatant) -> u32 {
    if c.conditions.iter().any(|x| matches!(x.condition, Condition::Grappled | Condition::Restrained)) {
        return 0;
    }
    let base = state.stat_blocks.iter().find(|s| s.id == c.stat_block_id).and_then(|s| s.stat_block.speeds.get("walk")).map_or(0, |s| s.feet);
    let speed: Vec<&ModifierChange> = state.modifiers_on(&c.id).filter(|m| m.target == ModifierTarget::Speed && !m.is_legacy()).map(|m| &m.change).collect();
    if let Some(set) = speed.iter().filter_map(|m| match m { ModifierChange::Set(n) => Some(*n), _ => None }).min() {
        return set;
    }
    let mut feet = i64::from(base) + speed.iter().map(|m| match m { ModifierChange::Add(n) => i64::from(*n), _ => 0 }).sum::<i64>();
    for change in &speed {
        if matches!(change, ModifierChange::Double) {
            feet *= 2;
        }
    }
    for change in &speed {
        if matches!(change, ModifierChange::Halve) {
            feet = feet.div_euclid(2);
        }
    }
    feet.clamp(0, i64::from(u32::MAX)) as u32
}

/// True if the combatant is currently concentrating on a spell.
fn is_concentrating(c: &Combatant) -> bool {
    c.concentration.is_some()
}

/// Break concentration and log it.
fn break_concentration(
    state: &mut EncounterState,
    stream: &mut Stream,
    target_id: &str,
    events: &mut Vec<EngineEvent>,
) -> Result<(), EngineError> {
    let i = index_of(state, target_id)?;
    if state.combatants[i].concentration.take().is_some() {
        events.push(event("concentration", format!("{target_id} loses concentration"), target_id, target_id));
        end_held(state, stream, target_id, events)?;
    }
    Ok(())
}

fn end_held(state: &mut EncounterState, stream: &mut Stream, source_id: &str, events: &mut Vec<EngineEvent>) -> Result<(), EngineError> {
    let held = |d: &Duration, source: &Option<String>| matches!(d, Duration::Concentration) && source.as_deref() == Some(source_id);
    for c in &mut state.combatants {
        c.conditions.retain(|x| !held(&x.duration, &x.source));
    }
    let ended = take_modifiers(state, |m| held(&m.remaining, &m.source));
    fire_on_end(state, stream, ended, events)
}

fn holds_by_concentration(state: &EncounterState, source_id: &str) -> bool {
    let held = |d: &Duration, source: &Option<String>| matches!(d, Duration::Concentration) && source.as_deref() == Some(source_id);
    state.combatants.iter().any(|c| c.conditions.iter().any(|x| held(&x.duration, &x.source)))
        || state.active_modifiers.iter().any(|m| held(&m.remaining, &m.source))
}

/// End everything each listed combatant holds before it leaves the encounter:
/// its concentration, logged as `<id> loses concentration`, and any
/// concentration-duration condition or modifier it is still the source of,
/// logged as `<id> leaves; the effects it held by concentration end` when its
/// concentration was already cleared.
pub fn end_concentration_on_leaving(state: &mut EncounterState, ids: &[String]) -> Result<Vec<EngineEvent>, EngineError> {
    let mut stream = Stream::from_state(state.rng.clone());
    let mut events = Vec::new();
    for id in ids {
        let i = index_of(state, id)?;
        if state.combatants[i].concentration.is_some() {
            break_concentration(state, &mut stream, id, &mut events)?;
        } else if holds_by_concentration(state, id) {
            events.push(event("concentration", format!("{id} leaves; the effects it held by concentration end"), id, id));
            end_held(state, &mut stream, id, &mut events)?;
        }
    }
    state.rng = stream.state();
    Ok(events)
}

/// True if the combatant holds a condition that incapacitates it.
fn is_incapacitated_by_condition(c: &Combatant) -> bool {
    c.conditions.iter().any(|x| incapacitates(x.condition))
}

/// True if the combatant can't hold concentration: it is down or dead, or an
/// incapacitating condition is on it.
pub(crate) fn cannot_concentrate(c: &Combatant) -> bool {
    c.end_state != EndState::Active || is_incapacitated_by_condition(c)
}

/// Break the concentration of each listed combatant that still concentrates.
/// `bench override` calls this after it leaves a concentrating creature unable
/// to concentrate, which skips the engine's own paths.
pub fn end_concentration(state: &mut EncounterState, ids: &[String]) -> Result<Vec<EngineEvent>, EngineError> {
    let mut stream = Stream::from_state(state.rng.clone());
    let mut events = Vec::new();
    for id in ids {
        break_concentration(state, &mut stream, id, &mut events)?;
    }
    state.rng = stream.state();
    Ok(events)
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
    let (count, sides) = crate::effect::parse_dice(expression).ok_or_else(|| EngineError::Invalid(format!("invalid dice expression {expression}")))?;
    let mut total = 0;
    for i in 0..count {
        let drawn = stream.roll_die(sides);
        total += player_roll_in(state, owner, sides, drawn, i == 0);
    }
    Ok(total)
}

/// The combatant is neither dead nor down and has no condition in `STOPS_ACTING`.
pub(crate) fn can_act(c: &Combatant) -> bool {
    c.end_state == EndState::Active && !c.conditions.iter().any(|x| incapacitates(x.condition))
}

/// Conditions under which a creature takes no actions: Incapacitated and those that include it.
pub(crate) const STOPS_ACTING: [Condition; 5] = [Condition::Incapacitated, Condition::Paralyzed, Condition::Petrified, Condition::Stunned, Condition::Unconscious];

fn incapacitates(condition: Condition) -> bool {
    STOPS_ACTING.contains(&condition)
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
    use crate::reaction::{ReactionAnswer, ReactionOption, WindowTrigger};
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
            reactions: Default::default(),
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
            trigger: None,
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
            trigger: None,
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
        let req = StepRequest::Action {
            actor_id: "goblin".into(),
            action_id: "scimitar".into(),
            target_ids: vec!["fighter".into()],
            slot_level: None,
            extra: false,
        };
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
            trigger: None,
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
        let req = StepRequest::Action {
            actor_id: "goblin".into(),
            action_id: "smash".into(),
            target_ids: vec!["fighter".into()],
            slot_level: None,
            extra: false,
        };
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
            trigger: None,
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
        let req = StepRequest::Action {
            actor_id: "fighter".into(),
            action_id: "smash".into(),
            target_ids: vec!["goblin".into()],
            slot_level: None,
            extra: false,
        };
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
        let req = StepRequest::end_turn();
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
        let req = StepRequest::Move { actor_id: "fighter".into(), target_id: Some("goblin".into()), band: RangeBand::Near, feet: 5, opportunity: vec![], declined: vec![] };
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
            bearer: None,
            source_ability: None,
            ends_on_damage: false,
            on_end: vec![],
            started_round: 0,
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
            started_round: 0,
        });
        let state = test_encounter(
            vec![test_stat_block("fighter", "Fighter", 20, 16), test_stat_block("goblin", "Goblin", 7, 15)],
            vec![fighter, goblin],
        );
        let d = disposition(&state, &state.combatants[0], &state.combatants[1], &sword());
        assert_eq!(d, RollDisposition::Disadvantage);
    }

    #[test]
    fn mixed_sources_cancel_to_a_straight_roll() {
        use RollDisposition::{Advantage as A, Disadvantage as D};
        assert_eq!(combined_disposition([A, A, D]), RollDisposition::Normal);
        assert_eq!(combined_disposition([A, D, D]), RollDisposition::Normal);
        assert_eq!(combined_disposition([A, A, D, D]), RollDisposition::Normal);
    }

    #[test]
    fn repeated_sources_of_one_kind_do_not_stack() {
        use RollDisposition::{Advantage as A, Disadvantage as D, Normal as N};
        assert_eq!(combined_disposition([A, A]), RollDisposition::Advantage);
        assert_eq!(combined_disposition([D, N, D]), RollDisposition::Disadvantage);
        assert_eq!(combined_disposition([]), RollDisposition::Normal);
    }

    #[test]
    fn combined_disposition_ignores_source_order() {
        use RollDisposition::{Advantage as A, Disadvantage as D, Normal as N};
        fn permutations(items: &[RollDisposition]) -> Vec<Vec<RollDisposition>> {
            if items.len() <= 1 {
                return vec![items.to_vec()];
            }
            let mut out = Vec::new();
            for i in 0..items.len() {
                let mut rest = items.to_vec();
                let head = rest.remove(i);
                for mut tail in permutations(&rest) {
                    tail.insert(0, head);
                    out.push(tail);
                }
            }
            out
        }
        for (sources, expected) in [
            (vec![D, N, A], N),
            (vec![A, A, D, N], N),
            (vec![D, D, A, A], N),
            (vec![A, N, A], A),
            (vec![D, N, D], D),
        ] {
            for order in permutations(&sources) {
                assert_eq!(combined_disposition(order.clone()), expected, "order {order:?}");
            }
        }
    }

    fn with_condition(mut combatant: Combatant, condition: Condition) -> Combatant {
        combatant.conditions.push(ConditionInstance { condition, duration: Duration::Rounds(1), source: None, ends_on_damage: false, started_round: 0 });
        combatant
    }

    #[test]
    fn invisible_poisoned_attacker_against_paralyzed_target_rolls_straight() {
        let fighter = with_condition(with_condition(test_combatant("fighter", "fighter", 20, "ally"), Condition::Invisible), Condition::Poisoned);
        let goblin = with_condition(test_combatant("goblin", "goblin", 7, "enemy"), Condition::Paralyzed);
        let state = test_encounter(
            vec![test_stat_block("fighter", "Fighter", 20, 16), test_stat_block("goblin", "Goblin", 7, 15)],
            vec![fighter, goblin],
        );
        assert_eq!(disposition(&state, &state.combatants[0], &state.combatants[1], &sword()), RollDisposition::Normal);
    }

    #[test]
    fn invisible_attacker_against_paralyzed_target_keeps_advantage() {
        let fighter = with_condition(test_combatant("fighter", "fighter", 20, "ally"), Condition::Invisible);
        let goblin = with_condition(test_combatant("goblin", "goblin", 7, "enemy"), Condition::Paralyzed);
        let state = test_encounter(
            vec![test_stat_block("fighter", "Fighter", 20, 16), test_stat_block("goblin", "Goblin", 7, 15)],
            vec![fighter, goblin],
        );
        assert_eq!(disposition(&state, &state.combatants[0], &state.combatants[1], &sword()), RollDisposition::Advantage);
    }

    #[test]
    fn timed_advantage_and_a_disadvantage_condition_roll_straight() {
        let fighter = with_condition(test_combatant("fighter", "fighter", 20, "ally"), Condition::Blinded);
        let goblin = test_combatant("goblin", "goblin", 7, "enemy");
        let mut state = test_encounter(
            vec![test_stat_block("fighter", "Fighter", 20, 16), test_stat_block("goblin", "Goblin", 7, 15)],
            vec![fighter, goblin],
        );
        state.active_modifiers.push(ActiveModifier {
            target: ModifierTarget::AttackRolls,
            change: ModifierChange::Advantage,
            remaining: Duration::Rounds(1),
            source: Some("fighter".into()),
            bearer: None,
            source_ability: None,
            ends_on_damage: false,
            on_end: vec![],
            started_round: 0,
        });
        state.active_modifiers.push(ActiveModifier {
            target: ModifierTarget::AttackRolls,
            change: ModifierChange::Advantage,
            remaining: Duration::Rounds(1),
            source: Some("fighter".into()),
            bearer: None,
            source_ability: None,
            ends_on_damage: false,
            on_end: vec![],
            started_round: 0,
        });
        assert_eq!(disposition(&state, &state.combatants[0], &state.combatants[1], &sword()), RollDisposition::Normal);
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
            trigger: None,
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
            trigger: None,
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
            trigger: None,
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
        let req = StepRequest::Action {
            actor_id: "attacker".into(),
            action_id: "multiattack".into(),
            target_ids: vec!["target".into()],
            slot_level: None,
            extra: false,
        };
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
            trigger: None,
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
        let req = StepRequest::Action {
            actor_id: "attacker".into(),
            action_id: "multiattack".into(),
            target_ids: vec!["target".into()],
            slot_level: None,
            extra: false,
        };
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
            trigger: None,
        });
        let attacker = test_combatant("attacker", "attacker", 30, "enemy");
        let target = test_combatant("target", "target", 50, "ally");
        let mut state = test_encounter(vec![test_stat_block("target", "Target", 50, 10), attacker_block], vec![target, attacker]);
        state.active_turn = Some("attacker".into());
        let req = StepRequest::Action { actor_id: "attacker".into(), action_id: "multiattack".into(), target_ids: vec!["target".into()], slot_level: None, extra: false };
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
            trigger: None,
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
        let req = StepRequest::Action {
            actor_id: "goblin".into(),
            action_id: "arrow".into(),
            target_ids: vec!["fighter".into()],
            slot_level: None,
            extra: false,
        };
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
            trigger: None,
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
        let resp = step(
            &mut state,
            StepRequest::Action {
                actor_id: "caster".into(),
                action_id: "burst".into(),
                target_ids: targets,
                slot_level: None,
                extra: false,
            },
        )
        .unwrap();
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
        let resp = step(&mut state, StepRequest::Action { actor_id: "goblin".into(), action_id: "bite".into(), target_ids: vec!["fighter".into()], slot_level: None, extra: false }).unwrap();
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
            trigger: None,
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
        let resp = step(state, StepRequest::Action { actor_id: "goblin".into(), action_id: "bite".into(), target_ids: vec!["fighter".into()], slot_level: None, extra: false }).unwrap();
        let attack = resp.events.iter().find(|e| e.kind == "attack").expect("attack event");
        assert_ne!(attack.roll, Some(20), "seed produced a critical hit; pick another seed");
        assert!(attack.message.ends_with("(hit)"), "{}", attack.message);
        100 - state.combatants.iter().find(|c| c.id == "fighter").unwrap().hp
    }

    fn condition_from(condition: Condition, source: &str) -> ConditionInstance {
        ConditionInstance { condition, duration: Duration::Permanent, source: Some(source.into()), ends_on_damage: false, started_round: 0 }
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
        step(&mut state, StepRequest::Action { actor_id: "goblin".into(), action_id: "bite".into(), target_ids: vec!["fighter".into()], slot_level: None, extra: false }).unwrap();
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
            trigger: None,
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
        step_events(
            state,
            StepRequest::Action {
                actor_id: actor.into(),
                action_id: action.into(),
                target_ids,
                slot_level: None,
                extra: false,
            },
        )
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

    fn damage_dice(effect: &Effect) -> Vec<(String, i32)> {
        let list = match effect {
            Effect::Attack(a) => a.damage.clone(),
            Effect::SavingThrow(s) => s.damage.clone(),
            Effect::Damage(d) => vec![d.clone()],
            Effect::Area(a) => return damage_dice(&a.effect),
            _ => vec![],
        };
        list.into_iter().map(|d| (d.dice, d.modifier)).collect()
    }

    fn bound(spell: &str, slot: Option<u8>) -> Vec<Effect> {
        bound_cast_effects(&caster_block(1), &cast(spell, slot))
            .expect("bound")
            .0
    }

    fn slotted_caster(spell: &str, slot: Option<u8>) -> StatBlock {
        let mut block = caster_block(17);
        block.spell_slots = Some(vec![4, 3, 3, 3, 3, 2, 2, 1, 1]);
        block
            .actions
            .push(ability(spell, vec![Effect::Cast(cast(spell, slot))], None));
        block
    }

    #[test]
    fn fireball_at_level_5_rolls_10d6() {
        assert_eq!(
            damage_dice(&bound("fireball", Some(5))[0]),
            [("10d6".to_string(), 0)]
        );
        assert_eq!(
            damage_dice(&bound("fireball", Some(3))[0]),
            [("8d6".to_string(), 0)]
        );
    }

    #[test]
    fn magic_missile_at_level_3_makes_five_darts() {
        let effects = bound("magic-missile", Some(3));
        assert_eq!(effects.len(), 5);
        assert!(
            effects
                .iter()
                .all(|e| damage_dice(e) == [("1d4".to_string(), 1)]),
            "{effects:?}"
        );
    }

    #[test]
    fn scorching_ray_at_level_3_makes_four_rays() {
        let effects = bound("scorching-ray", Some(3));
        assert_eq!(effects.len(), 4);
        assert!(effects
            .iter()
            .all(|e| matches!(e, Effect::Attack(a) if a.target_count == 1)));
    }

    #[test]
    fn cure_wounds_at_level_2_heals_4d8_plus_the_modifier() {
        let effects = bound("cure-wounds", Some(2));
        assert!(
            matches!(&effects[0], Effect::Heal(h) if h.dice.as_deref() == Some("4d8") && h.modifier == 3),
            "{effects:?}"
        );
    }

    #[test]
    fn disintegrate_deals_10d6_plus_40() {
        assert_eq!(
            damage_dice(&bound("disintegrate", None)[0]),
            [("10d6".to_string(), 40)]
        );
        assert_eq!(
            damage_dice(&bound("disintegrate", Some(7))[0]),
            [("13d6".to_string(), 40)]
        );
    }

    #[test]
    fn heal_at_level_7_heals_exactly_80_without_dice() {
        let mut ally = test_combatant("pc", "pc", 200, "ally");
        ally.hp = 1;
        let mut state = test_encounter(
            vec![slotted_caster("heal", Some(7)), pc_block("pc", 200)],
            vec![test_combatant("mage", "mage", 30, "ally"), ally],
        );
        state.settings.pc_dice = PcDice::Ask;
        let cursor = state.rng.cursor;
        let events = act(&mut state, "mage", "heal", &["pc"]).unwrap();
        assert!(state.pending.is_none(), "{events:?}");
        assert_eq!(state.rng.cursor, cursor, "no dice drawn");
        assert_eq!(state.combatants[1].hp, 81);
        assert_eq!(state.combatants[0].resources["mage:slot:7"], 1);
    }

    #[test]
    fn an_extra_targets_rule_raises_an_attack_target_count() {
        let spell: crate::spell::SpellEffect = serde_json::from_value(serde_json::json!({
            "id": "test-bolts", "name": "Test Bolts", "source": "test", "level": 2, "casting_time": "action", "concentration": false,
            "effects": [{"attack": {"modifier": 0, "target_count": 1, "damage": [{"dice": "1d6", "type": "fire"}]}}],
            "scaling": [{"effect_index": 0, "per_slot": {"extra_targets": {"count": 1}}}],
        }))
        .unwrap();
        let effects = scaled_effects(&spell, 4).unwrap();
        assert!(
            matches!(&effects[0], Effect::Attack(a) if a.target_count == 3),
            "{effects:?}"
        );
        assert!(
            matches!(&scaled_effects(&spell, 2).unwrap()[0], Effect::Attack(a) if a.target_count == 1)
        );
    }

    #[test]
    fn hold_monster_at_level_6_resolves_against_every_target() {
        let mut blocks = vec![slotted_caster("hold-monster", Some(6))];
        let mut combatants = vec![test_combatant("mage", "mage", 30, "ally")];
        for id in ["a", "b", "c"] {
            blocks.push(test_stat_block(id, id, 20, 10));
            combatants.push(test_combatant(id, id, 20, "enemy"));
        }
        let mut state = test_encounter(blocks, combatants);
        let events = act(&mut state, "mage", "hold-monster", &["a", "b", "c"]).unwrap();
        let saves: Vec<_> = events
            .iter()
            .filter(|e| e.kind == "save")
            .filter_map(|e| e.target_id.clone())
            .collect();
        assert_eq!(saves, ["a", "b", "c"]);
    }

    #[test]
    fn a_slot_below_the_spell_level_or_on_a_cantrip_is_refused() {
        let mut state = test_encounter(
            vec![
                slotted_caster("fireball", Some(2)),
                test_stat_block("ogre", "Ogre", 50, 10),
            ],
            vec![
                test_combatant("mage", "mage", 30, "ally"),
                test_combatant("ogre", "ogre", 50, "enemy"),
            ],
        );
        let err = act(&mut state, "mage", "fireball", &["ogre"]).unwrap_err();
        assert!(
            matches!(&err, EngineError::Invalid(m) if m.contains("level 3") && m.contains("level 2")),
            "{err:?}"
        );
        assert!(!state.combatants[0].resources.contains_key("mage:slot:2"));
        let mut state = test_encounter(
            vec![
                slotted_caster("fire-bolt", Some(1)),
                test_stat_block("ogre", "Ogre", 50, 10),
            ],
            vec![
                test_combatant("mage", "mage", 30, "ally"),
                test_combatant("ogre", "ogre", 50, "enemy"),
            ],
        );
        let err = act(&mut state, "mage", "fire-bolt", &["ogre"]).unwrap_err();
        assert!(
            matches!(&err, EngineError::Invalid(m) if m.contains("cantrip")),
            "{err:?}"
        );
    }

    #[test]
    fn a_leveled_spell_with_no_slot_level_does_not_scale_with_caster_level() {
        let (effects, _) = bound_cast_effects(&caster_block(17), &cast("fireball", None)).unwrap();
        assert_eq!(damage_dice(&effects[0]), [("8d6".to_string(), 0)]);
        let (effects, _) = bound_cast_effects(&caster_block(17), &cast("fire-bolt", None)).unwrap();
        assert_eq!(damage_dice(&effects[0]), [("4d10".to_string(), 0)]);
    }

    #[test]
    fn an_upcast_clause_event_appears_only_on_an_upcast() {
        let clause_lines = |slot: Option<u8>| {
            let mut state = test_encounter(
                vec![
                    slotted_caster("acid-arrow", slot),
                    test_stat_block("ogre", "Ogre", 500, 10),
                ],
                vec![
                    test_combatant("mage", "mage", 30, "ally"),
                    test_combatant("ogre", "ogre", 500, "enemy"),
                ],
            );
            let events = act(&mut state, "mage", "acid-arrow", &["ogre"]).unwrap();
            events
                .into_iter()
                .filter(|e| e.kind == "upcast_clause")
                .map(|e| e.message)
                .collect::<Vec<_>>()
        };
        assert_eq!(clause_lines(Some(4)), ["mage casts acid-arrow at level 4; agent clause: The later damage increases by 1d4 for each spell slot level above 2."]);
        assert!(clause_lines(Some(2)).is_empty());
        assert!(clause_lines(None).is_empty());
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
        end_turn_events(&mut state, &[]).unwrap();
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
        end_turn_events(&mut state, &[]).unwrap();
        assert_eq!(state.active_turn.as_deref(), Some("dragon"));
        end_turn_events(&mut state, &[]).unwrap();
        assert_eq!(legendary_uses_left(&state, "dragon"), Ok(2), "the budget reset at the dragon's turn start");
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
        end_turn_events(&mut state, &[]).unwrap();
        end_turn_events(&mut state, &[]).unwrap();
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
        state.combatants[1].conditions.push(ConditionInstance { condition: Condition::Unconscious, duration: Duration::Concentration, source: Some("a".into()), ends_on_damage: true, started_round: 0 });
        state.combatants[1].conditions.push(ConditionInstance { condition: Condition::Prone, duration: Duration::Permanent, source: None, ends_on_damage: false, started_round: 0 });
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
        state.combatants[1].conditions.push(ConditionInstance { condition: Condition::Paralyzed, duration: Duration::Concentration, source: Some("a".into()), ends_on_damage: false, started_round: 0 });
        state.combatants[1].conditions.push(ConditionInstance { condition: Condition::Prone, duration: Duration::Concentration, source: Some("b".into()), ends_on_damage: false, started_round: 0 });
        state.active_modifiers.push(ActiveModifier { target: ModifierTarget::ArmorClass, change: ModifierChange::Add(2), remaining: Duration::Concentration, source: Some("a".into()), bearer: None, source_ability: None, ends_on_damage: false, on_end: vec![], started_round: 0 });
        let mut events = Vec::new();
        break_concentration(&mut state, &mut Stream::from_state(RngState::new(1)), "a", &mut events).unwrap();
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
        move_combatant(&mut state, "a", Some("b"), RangeBand::Far, 30, &[], &[]).unwrap();
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
        let events = move_combatant(&mut state, "a", None, RangeBand::Far, 35, &[], &[]).unwrap();
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
        let err = move_combatant(&mut state, "a", None, RangeBand::Far, 30, &[], &[]).unwrap_err();
        assert!(err.to_string().contains("35 feet"));
        assert_eq!(state.combatants[0].ranges["b"], RangeBand::Engaged);
        move_combatant(&mut state, "a", None, RangeBand::Far, 35, &[], &[]).unwrap();
        assert_eq!(state.combatants[0].action_economy.movement_used_feet, 35);
    }

    #[test]
    fn untargeted_move_skips_combatants_that_are_down() {
        let mut state = three_way_encounter();
        state.combatants[1].end_state = EndState::Dead;
        let events = move_combatant(&mut state, "a", None, RangeBand::Far, 30, &[], &[]).unwrap();
        assert!(events.iter().all(|e| e.kind != "opportunity_attack"));
        assert_eq!(state.combatants[0].ranges["b"], RangeBand::Far);
        assert_eq!(state.combatants[0].ranges["c"], RangeBand::Far);
        assert!(state.combatants[0].engagement.is_empty());
        assert!(state.combatants[1].engagement.is_empty());
    }

    #[test]
    fn untargeted_move_cannot_engage() {
        let mut state = three_way_encounter();
        assert!(move_combatant(&mut state, "a", None, RangeBand::Engaged, 30, &[], &[]).is_err());
    }

    fn weapon(id: &str, modifier: i32, damage: i32, range: Option<(u32, u32)>, usage: Option<Usage>) -> crate::stat_block::AbilityEntry {
        crate::stat_block::AbilityEntry {
            id: id.into(),
            name: id.into(),
            desc: None,
            kind: crate::stat_block::AbilityKind::Modeled,
            effects: vec![Effect::Attack(Attack {
                modifier,
                reach: range.is_none().then_some(5),
                range,
                target_count: 1,
                damage: vec![Damage { dice: "1d4".into(), damage_type: DamageType::Slashing, modifier: damage, magical: false, if_target: None, if_target_from_actor: false }],
                on_hit: vec![],
            })],
            agent_clauses: vec![],
            default_resolution: None,
            usage,
            trigger: None,
        }
    }

    /// `a` (ally) is engaged with enemies `b` and `c`; each enemy has one sure-hit claw.
    fn provoking_encounter(claw_damage: i32) -> EncounterState {
        let mut state = three_way_encounter();
        for id in ["b", "c"] {
            let snapshot = state.stat_blocks.iter_mut().find(|s| s.id == id).unwrap();
            snapshot.stat_block.actions.push(weapon("claw", 100, claw_damage, None, None));
        }
        state.combatants[0].ranges.insert("c".into(), RangeBand::Engaged);
        state.combatants[2].ranges.insert("a".into(), RangeBand::Engaged);
        state.combatants[0].engagement.push("c".into());
        state.combatants[2].engagement.push("a".into());
        state.initiative_order = vec!["a".into(), "b".into(), "c".into()];
        state.active_turn = Some("a".into());
        state
    }

    fn oa(reactor: &str, attack: Option<&str>) -> OpportunityAttack {
        OpportunityAttack { reactor_id: reactor.into(), attack_id: attack.map(Into::into) }
    }

    fn no_provoke_trait() -> crate::stat_block::AbilityEntry {
        crate::stat_block::AbilityEntry {
            id: "flyby".into(),
            name: "Flyby".into(),
            desc: None,
            kind: crate::stat_block::AbilityKind::Modeled,
            effects: vec![Effect::NoProvoke],
            agent_clauses: vec![],
            default_resolution: None,
            usage: None,
            trigger: None,
        }
    }

    #[test]
    fn disengage_spends_the_action_and_the_next_move_provokes_nothing() {
        let mut state = provoking_encounter(0);
        let events = step_events(&mut state, StepRequest::Disengage { actor_id: "a".into(), extra: false }).unwrap();
        assert!(events.iter().any(|e| e.kind == "disengage"), "{events:?}");
        assert!(state.combatants[0].action_economy.action_used);
        assert!(state.combatants[0].action_economy.disengaged);
        let events = move_combatant(&mut state, "a", None, RangeBand::Far, 35, &[], &[]).unwrap();
        assert!(events.iter().all(|e| e.kind != "opportunity_attack"), "{events:?}");
        assert_eq!(state.combatants[0].ranges["b"], RangeBand::Far);
    }

    #[test]
    fn disengage_refuses_a_named_reactor_with_the_reason() {
        let mut state = provoking_encounter(0);
        step_events(&mut state, StepRequest::Disengage { actor_id: "a".into(), extra: false }).unwrap();
        let err = move_combatant(&mut state, "a", None, RangeBand::Far, 35, &[oa("b", None)], &[]).unwrap_err();
        assert!(err.to_string().contains("b cannot make an Opportunity Attack: a has disengaged"), "{err}");
        assert!(!state.combatants[1].action_economy.reaction_used);
    }

    #[test]
    fn disengage_needs_the_active_turn_and_an_unused_action() {
        let mut state = provoking_encounter(0);
        let err = step_events(&mut state, StepRequest::Disengage { actor_id: "b".into(), extra: false }).unwrap_err();
        assert!(err.to_string().contains("not the active combatant"), "{err}");
        step_events(&mut state, StepRequest::Disengage { actor_id: "a".into(), extra: false }).unwrap();
        let err = step_events(&mut state, StepRequest::Disengage { actor_id: "a".into(), extra: false }).unwrap_err();
        assert!(err.to_string().contains("action already used"), "{err}");
        state.combatants[0].action_economy = ActionEconomy::default();
        state.combatants[0].end_state = EndState::Dead;
        assert!(step_events(&mut state, StepRequest::Disengage { actor_id: "a".into(), extra: false }).is_err());
    }

    fn nimble_escape() -> crate::stat_block::AbilityEntry {
        crate::stat_block::AbilityEntry {
            id: "nimble-escape".into(),
            name: "Nimble Escape".into(),
            effects: vec![Effect::Disengage],
            ..no_provoke_trait()
        }
    }

    #[test]
    fn a_bonus_disengage_needs_no_target_and_leaves_the_action_unspent() {
        let mut state = provoking_encounter(0);
        state.stat_blocks.iter_mut().find(|s| s.id == "a").unwrap().stat_block.bonus_actions.push(nimble_escape());
        let request = || StepRequest::Action { actor_id: "a".into(), action_id: "nimble-escape".into(), target_ids: vec![], slot_level: None, extra: false };
        let events = step_events(&mut state, request()).unwrap();
        let messages: Vec<&str> = events.iter().map(|e| e.message.as_str()).collect();
        assert_eq!(messages, ["a uses Nimble Escape to Disengage"]);
        assert_eq!(events[0].kind, "disengage");
        let economy = &state.combatants[0].action_economy;
        assert!(economy.disengaged && economy.bonus_action_used && !economy.action_used);
        let err = move_combatant(&mut state, "a", None, RangeBand::Far, 35, &[oa("b", None)], &[]).unwrap_err();
        assert!(err.to_string().contains("a has disengaged"), "{err}");
        assert!(step_events(&mut state, request()).unwrap_err().to_string().contains("bonus action already used"));
    }

    #[test]
    fn disengage_ends_with_the_movers_turn() {
        let mut state = provoking_encounter(0);
        step_events(&mut state, StepRequest::Disengage { actor_id: "a".into(), extra: false }).unwrap();
        step_events(&mut state, StepRequest::end_turn()).unwrap();
        assert!(!state.combatants[0].action_economy.disengaged, "the flag ends with the turn");
        for _ in 0..2 {
            step_events(&mut state, StepRequest::end_turn()).unwrap();
        }
        assert_eq!(state.active_turn.as_deref(), Some("a"));
        assert!(!state.combatants[0].action_economy.disengaged);
    }

    #[test]
    fn a_no_provoke_trait_means_no_notice_and_no_named_reactor() {
        let mut state = provoking_encounter(0);
        let snapshot = state.stat_blocks.iter_mut().find(|s| s.id == "a").unwrap();
        snapshot.stat_block.traits.push(no_provoke_trait());
        let err = move_combatant(&mut state.clone(), "a", None, RangeBand::Far, 35, &[oa("b", None)], &[]).unwrap_err();
        assert!(err.to_string().contains("b cannot make an Opportunity Attack: a does not provoke (Flyby)"), "{err}");
        let events = move_combatant(&mut state, "a", None, RangeBand::Far, 35, &[], &[]).unwrap();
        assert!(events.iter().all(|e| e.kind != "opportunity_attack"), "{events:?}");
        assert!(!state.combatants[1].action_economy.reaction_used);
    }

    #[test]
    fn a_no_provoke_effect_outside_traits_still_counts() {
        let mut state = provoking_encounter(0);
        let snapshot = state.stat_blocks.iter_mut().find(|s| s.id == "a").unwrap();
        snapshot.stat_block.bonus_actions.push(no_provoke_trait());
        let err = move_combatant(&mut state, "a", None, RangeBand::Far, 35, &[oa("b", None)], &[]).unwrap_err();
        assert!(err.to_string().contains("a does not provoke (Flyby)"), "{err}");
    }

    #[test]
    fn named_reactor_attacks_before_the_move_and_spends_its_reaction() {
        let mut state = provoking_encounter(0);
        let events = move_combatant(&mut state, "a", None, RangeBand::Far, 35, &[oa("b", None)], &[]).unwrap();
        let kinds: Vec<_> = events.iter().map(|e| e.kind.as_str()).collect();
        let reaction = kinds.iter().position(|k| *k == "reaction").expect("reaction line");
        let movement = kinds.iter().position(|k| *k == "movement").expect("movement line");
        assert!(reaction < movement && kinds[reaction + 1] == "attack", "{kinds:?}");
        assert_eq!(events[reaction].message, "OPPORTUNITY ATTACK b on a (reaction)");
        assert!(state.combatants[1].action_economy.reaction_used);
        let notices: Vec<_> = events.iter().filter(|e| e.kind == "opportunity_attack").collect();
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].actor_id.as_deref(), Some("c"));
        assert_eq!(state.combatants[0].ranges["b"], RangeBand::Far);
        assert_eq!(state.combatants[0].action_economy.movement_used_feet, 35);
    }

    #[test]
    fn a_miss_spends_the_reaction_too() {
        let mut state = provoking_encounter(0);
        let snapshot = state.stat_blocks.iter_mut().find(|s| s.id == "b").unwrap();
        snapshot.stat_block.actions[0] = weapon("claw", -100, 0, None, None);
        for seed in 1..20 {
            let mut state = state.clone();
            state.rng = RngState::new(seed);
            let events = move_combatant(&mut state, "a", Some("b"), RangeBand::Near, 5, &[oa("b", None)], &[]).unwrap();
            let attack = events.iter().find(|e| e.kind == "attack").unwrap();
            if attack.message.ends_with("(miss)") {
                assert!(state.combatants[1].action_economy.reaction_used);
                return;
            }
        }
        panic!("no seed missed");
    }

    #[test]
    fn the_reaction_comes_back_on_the_reactors_own_turn() {
        let mut state = provoking_encounter(0);
        move_combatant(&mut state, "a", Some("b"), RangeBand::Near, 5, &[oa("b", None)], &[]).unwrap();
        step(&mut state, StepRequest::end_turn()).unwrap();
        assert_eq!(state.active_turn.as_deref(), Some("b"));
        assert!(!state.combatants[1].action_economy.reaction_used);
    }

    #[test]
    fn a_downed_mover_stops_and_later_reactors_keep_their_reaction() {
        let mut state = provoking_encounter(50);
        let events = move_combatant(&mut state, "a", None, RangeBand::Far, 35, &[oa("b", None), oa("c", None)], &[]).unwrap();
        assert_ne!(state.combatants[0].end_state, EndState::Active);
        assert_eq!(events.last().unwrap().kind, "move_stopped");
        assert!(events.iter().all(|e| e.kind != "movement" && e.kind != "opportunity_attack"));
        assert_eq!(state.combatants[0].action_economy.movement_used_feet, 0);
        assert_eq!(state.combatants[0].ranges["b"], RangeBand::Engaged);
        assert!(state.combatants[0].engagement.contains(&"c".to_string()));
        assert!(!state.combatants[2].action_economy.reaction_used);
    }

    #[test]
    fn an_ineligible_named_reactor_fails_the_whole_move() {
        let base = provoking_encounter(0);
        let refused = |state: &mut EncounterState, target: Option<&str>, band: RangeBand, named: &[OpportunityAttack], why: &str| {
            let before = serde_json::to_value(&*state).unwrap();
            let err = move_combatant(state, "a", target, band, 35, named, &[]).unwrap_err();
            assert!(matches!(err, EngineError::Illegal(_) | EngineError::Invalid(_)), "{err:?}");
            assert!(err.to_string().contains(why), "{err}");
            assert_eq!(serde_json::to_value(&*state).unwrap(), before);
        };
        let mut used = base.clone();
        used.combatants[1].action_economy.reaction_used = true;
        refused(&mut used, None, RangeBand::Far, &[oa("c", None), oa("b", None)], "already used its reaction");
        refused(&mut base.clone(), Some("c"), RangeBand::Near, &[oa("b", None)], "does not leave its reach");
        refused(&mut base.clone(), None, RangeBand::Far, &[oa("b", None), oa("b", None)], "named twice");
        let mut ally = base.clone();
        ally.combatants[1].side_id = "ally".into();
        refused(&mut ally, None, RangeBand::Far, &[oa("b", None)], "on a's side");
        let mut stunned = base.clone();
        stunned.combatants[1].conditions.push(ConditionInstance { condition: Condition::Stunned, duration: Duration::Rounds(1), source: None, ends_on_damage: false, started_round: 0 });
        refused(&mut stunned, None, RangeBand::Far, &[oa("b", None)], "cannot act");
        let mut slowed = base.clone();
        slowed.active_modifiers.push(ActiveModifier { target: ModifierTarget::Reactions, change: ModifierChange::None, remaining: Duration::Rounds(1), source: Some("b".into()), bearer: None, source_ability: None, ends_on_damage: false, on_end: vec![], started_round: 0 });
        refused(&mut slowed, None, RangeBand::Far, &[oa("b", None)], "cannot take reactions");
        let mut unseen = base.clone();
        unseen.combatants[0].conditions.push(ConditionInstance { condition: Condition::Invisible, duration: Duration::Rounds(1), source: None, ends_on_damage: false, started_round: 0 });
        refused(&mut unseen, None, RangeBand::Far, &[oa("b", None)], "who is invisible (assumption)");
        refused(&mut base.clone(), None, RangeBand::Far, &[oa("b", Some("bite"))], "action bite not found");
    }

    #[test]
    fn notices_name_only_eligible_reactors() {
        let mut state = provoking_encounter(0);
        state.combatants[1].action_economy.reaction_used = true;
        state.combatants[2].conditions.push(ConditionInstance { condition: Condition::Blinded, duration: Duration::Rounds(1), source: None, ends_on_damage: false, started_round: 0 });
        let events = move_combatant(&mut state, "a", None, RangeBand::Far, 35, &[], &[]).unwrap();
        assert!(events.iter().all(|e| e.kind != "opportunity_attack"), "{events:?}");
        let assumption = events.iter().find(|e| e.kind == "assumption").expect("sight assumption");
        assert_eq!(assumption.message, "ASSUMPTION c is blinded and cannot see a, so it makes no Opportunity Attack on a");
    }

    #[test]
    fn the_default_attack_is_the_first_at_will_melee_leaf() {
        use crate::effect::{ChoiceOption, Multiattack, MultiattackStep};
        let mut block = test_stat_block("m", "M", 20, 10);
        block.actions = vec![
            crate::stat_block::AbilityEntry {
                id: "multiattack".into(),
                name: "Multiattack".into(),
                desc: None,
                kind: crate::stat_block::AbilityKind::Modeled,
                effects: vec![Effect::Multiattack(Multiattack {
                    steps: vec![MultiattackStep::Choice {
                        options: vec![
                            ChoiceOption { action_id: "spit".into(), count: 1, cast: None },
                            ChoiceOption { action_id: "gore".into(), count: 1, cast: None },
                            ChoiceOption { action_id: "bite".into(), count: 1, cast: None },
                        ],
                        pick: 1,
                    }],
                })],
                agent_clauses: vec![],
                default_resolution: None,
                usage: None,
                trigger: None,
            },
            weapon("bite", 4, 2, None, None),
            weapon("spit", 4, 2, Some((30, 60)), None),
            weapon("gore", 4, 2, None, Some(Usage::PerDay { uses: 1, group: None })),
        ];
        assert_eq!(opportunity_attack_action(&block, None).unwrap().id, "bite");
        assert_eq!(opportunity_attack_action(&block, Some("gore")).unwrap().id, "gore");
        assert!(opportunity_attack_action(&block, Some("spit")).unwrap_err().contains("not a single melee attack"));
        assert!(opportunity_attack_action(&block, Some("multiattack")).is_err());
        block.actions.remove(1);
        assert!(opportunity_attack_action(&block, None).unwrap_err().contains("no at-will melee attack"));
    }

    #[test]
    fn a_named_limited_attack_spends_its_use() {
        let mut state = provoking_encounter(0);
        let snapshot = state.stat_blocks.iter_mut().find(|s| s.id == "b").unwrap();
        snapshot.stat_block.actions.push(weapon("gore", 100, 0, None, Some(Usage::PerDay { uses: 1, group: None })));
        state.combatants[1].resources.insert("b:gore".into(), 1);
        let err = move_combatant(&mut state.clone(), "a", Some("b"), RangeBand::Near, 5, &[oa("b", Some("gore"))], &[]).unwrap_err();
        assert!(err.to_string().contains("no uses remaining"), "{err}");
        state.combatants[1].resources.remove("b:gore");
        move_combatant(&mut state, "a", Some("b"), RangeBand::Near, 5, &[oa("b", Some("gore"))], &[]).unwrap();
        assert_eq!(state.combatants[1].resources["b:gore"], 1);
    }

    #[test]
    fn a_spent_named_attack_fails_before_any_attack_resolves() {
        let mut state = provoking_encounter(0);
        let snapshot = state.stat_blocks.iter_mut().find(|s| s.id == "c").unwrap();
        snapshot.stat_block.actions.push(weapon("gore", 100, 0, None, Some(Usage::PerDay { uses: 1, group: None })));
        state.combatants[2].resources.insert("c:gore".into(), 1);
        let before = serde_json::to_value(&state).unwrap();
        let err = move_combatant(&mut state, "a", None, RangeBand::Far, 35, &[oa("b", None), oa("c", Some("gore"))], &[]).unwrap_err();
        assert!(err.to_string().contains("no uses remaining"), "{err}");
        assert_eq!(serde_json::to_value(&state).unwrap(), before);
    }

    fn blast(id: &str, damage: i32, usage: Option<Usage>) -> crate::stat_block::AbilityEntry {
        let damage = Damage { dice: "1d1".into(), damage_type: DamageType::Force, modifier: damage - 1, magical: false, if_target: None, if_target_from_actor: false };
        ability(id, vec![Effect::Damage(damage)], usage)
    }

    /// Initiative `pc`, `dragon`, `knight`. The dragon has two legendary uses.
    fn legendary_encounter() -> EncounterState {
        let mut dragon = test_stat_block("dragon", "Dragon", 100, 18);
        dragon.legendary_uses_per_round = 2;
        dragon.legendary_actions = vec![
            blast("tail", 1, None),
            blast("wing", 1, Some(Usage::Recharge { dice: "6".into(), group: None })),
            blast("roar", 1, Some(Usage::PerDay { uses: 1, group: None })),
            blast("bite", 1, Some(Usage::CooldownUntilNextTurn)),
        ];
        test_encounter(
            vec![pc_block("pc", 200), dragon, pc_block("knight", 200)],
            vec![
                test_combatant("pc", "pc", 200, "ally"),
                test_combatant("dragon", "dragon", 100, "enemy"),
                test_combatant("knight", "knight", 200, "ally"),
            ],
        )
    }

    fn lu(holder: &str, action: &str, targets: &[&str]) -> LegendaryUse {
        LegendaryUse { holder_id: holder.into(), action_id: action.into(), target_ids: targets.iter().map(|t| t.to_string()).collect() }
    }

    fn end_turn_with(state: &mut EncounterState, legendary: Vec<LegendaryUse>) -> Result<Vec<EngineEvent>, EngineError> {
        step_events(state, StepRequest::EndTurn { legendary })
    }

    #[test]
    fn a_legendary_action_resolves_between_the_end_and_the_start_of_turns() {
        let mut state = legendary_encounter();
        let events = end_turn_with(&mut state, vec![lu("dragon", "tail", &["pc"])]).unwrap();
        let kinds: Vec<&str> = events.iter().map(|e| e.kind.as_str()).collect();
        assert_eq!(kinds, ["end_turn", "legendary", "damage", "start_turn"], "{events:?}");
        assert_eq!(events[1].message, "LEGENDARY ACTION dragon: tail (1 of 2)");
        assert_eq!(state.combatants[0].hp, 199);
        assert_eq!(state.active_turn.as_deref(), Some("dragon"));
        assert!(!state.combatants[1].resources.contains_key("dragon:legendary"), "the next creature's budget resets after it spends");
    }

    fn refused(state: &mut EncounterState, legendary: Vec<LegendaryUse>) -> String {
        let before = serde_json::to_value(&*state).unwrap();
        let err = end_turn_with(state, legendary).unwrap_err();
        assert_eq!(serde_json::to_value(&*state).unwrap(), before, "a refused end-turn changes nothing");
        err.to_string()
    }

    #[test]
    fn an_ineligible_declaration_fails_the_whole_end_turn() {
        let mut state = legendary_encounter();
        let err = refused(&mut state, vec![lu("dragon", "tail", &["pc"]), lu("dragon", "tail", &["pc"]), lu("dragon", "tail", &["pc"])]);
        assert!(err.contains("no legendary actions left this round"), "{err}");
        let err = refused(&mut state, vec![lu("knight", "tail", &["dragon"])]);
        assert!(err.contains("knight has no legendary actions"), "{err}");
        let err = refused(&mut state, vec![lu("dragon", "breath", &["pc"])]);
        assert!(err.contains("legendary action breath not found"), "{err}");
        let err = refused(&mut state, vec![lu("dragon", "tail", &["nobody"])]);
        assert!(err.contains("nobody not found"), "{err}");

        state.active_turn = Some("dragon".into());
        let err = refused(&mut state, vec![lu("dragon", "tail", &["pc"])]);
        assert!(err.contains("turn is ending"), "{err}");

        state.active_turn = Some("pc".into());
        state.combatants[1].conditions.push(ConditionInstance { condition: Condition::Stunned, source: None, duration: Duration::Permanent, ends_on_damage: false, started_round: 0 });
        let err = refused(&mut state, vec![lu("dragon", "tail", &["pc"])]);
        assert!(err.contains("dragon cannot act"), "{err}");
    }

    #[test]
    fn legendary_actions_spend_their_own_usage() {
        for (action, why) in [("wing", "is not recharged"), ("roar", "no uses remaining"), ("bite", "is on cooldown")] {
            let mut state = legendary_encounter();
            let err = refused(&mut state, vec![lu("dragon", action, &["pc"]), lu("dragon", action, &["pc"])]);
            assert!(err.contains(why), "{action}: {err}");
            end_turn_with(&mut state, vec![lu("dragon", action, &["pc"])]).unwrap();
        }
    }

    #[test]
    fn a_holder_that_drops_partway_loses_its_remaining_uses() {
        let mut state = legendary_encounter();
        let mut knight = pc_block("knight", 200);
        knight.legendary_uses_per_round = 2;
        knight.legendary_actions = vec![blast("smite", 100, None)];
        state.stat_blocks[2] = StatBlockSnapshot::from_stat_block("knight".into(), knight);
        state.stat_blocks[1].stat_block.legendary_actions.push(blast("crush", 300, None));
        let events = end_turn_with(
            &mut state,
            vec![lu("dragon", "crush", &["knight"]), lu("knight", "smite", &["dragon"]), lu("knight", "smite", &["dragon"])],
        )
        .unwrap();
        let lost: Vec<&EngineEvent> = events.iter().filter(|e| e.kind == "legendary_lost").collect();
        assert_eq!(lost.len(), 1, "{events:?}");
        assert_eq!(lost[0].message, "knight can no longer act and loses its remaining legendary actions");
        assert_eq!(state.combatants[1].hp, 100, "the knight's smites never land");
    }

    #[test]
    fn one_window_opens_before_the_round_advances_even_past_skipped_creatures() {
        let mut state = legendary_encounter();
        state.active_turn = Some("dragon".into());
        state.combatants[0].end_state = EndState::Dead;
        state.combatants[2].end_state = EndState::Dead;
        state.combatants.push(test_combatant("wyrm", "dragon", 100, "enemy"));
        state.initiative_order.push("wyrm".into());
        let events = end_turn_with(&mut state, vec![lu("wyrm", "tail", &["dragon"])]).unwrap();
        let kinds: Vec<&str> = events.iter().map(|e| e.kind.as_str()).collect();
        assert_eq!(kinds, ["end_turn", "legendary", "damage", "forfeit", "start_turn"], "{events:?}");
        assert_eq!(state.round, 1);
        let events = end_turn_with(&mut state, vec![lu("dragon", "tail", &["wyrm"])]).unwrap();
        let kinds: Vec<&str> = events.iter().map(|e| e.kind.as_str()).collect();
        assert_eq!(kinds, ["end_turn", "legendary", "damage", "forfeit", "start_turn"], "{events:?}");
        assert_eq!(state.round, 2);
    }

    #[test]
    fn legendary_holders_list_uses_left_and_ready_actions() {
        let mut state = legendary_encounter();
        state.combatants[1].resources.insert("dragon:legendary".into(), 1);
        state.combatants[1].resources.insert("dragon:wing".into(), 0);
        let holders = legendary_holders(&state);
        assert_eq!(holders, vec![LegendaryHolder { id: "dragon".into(), uses_left: 1, ready_actions: vec!["tail".into(), "roar".into(), "bite".into()] }]);
        state.active_turn = Some("dragon".into());
        assert!(legendary_holders(&state).is_empty());
    }

    #[test]
    fn a_legendary_recharge_rolls_at_the_holders_turn_start() {
        let mut state = legendary_encounter();
        end_turn_with(&mut state, vec![lu("dragon", "wing", &["pc"])]).unwrap();
        assert_eq!(state.combatants[1].resources["dragon:wing"], 0);
        let mut recharged = false;
        for _ in 0..30 {
            let events = end_turn_with(&mut state, vec![]).unwrap();
            recharged |= events.iter().any(|e| e.kind == "recharge" && e.message.contains("wing"));
        }
        assert!(recharged);
    }

    fn fixture_block(id: &str) -> StatBlock {
        serde_json::from_str(crate::embedded::fixture_json(id).unwrap()).unwrap()
    }

    fn lit(monster: StatBlock, lighting: crate::effect::Lighting) -> EncounterState {
        let id = monster.id.clone();
        let mut state = test_encounter(
            vec![monster, test_stat_block("fighter", "Fighter", 100, 10)],
            vec![test_combatant(&id, &id, 50, "enemy"), test_combatant("fighter", "fighter", 100, "ally")],
        );
        state.settings.lighting = lighting;
        state
    }

    fn attack_disposition(monster: &str, lighting: crate::effect::Lighting) -> RollDisposition {
        let state = lit(fixture_block(monster), lighting);
        disposition(&state, &state.combatants[0], &state.combatants[1], &sword())
    }

    #[test]
    fn a_kobold_attacks_with_disadvantage_only_in_sunlight() {
        use crate::effect::Lighting::*;
        assert_eq!(attack_disposition("kobold-warrior", Sunlight), RollDisposition::Disadvantage);
        for lighting in [Unspecified, DimLight, BrightLight] {
            assert_eq!(attack_disposition("kobold-warrior", lighting), RollDisposition::Normal, "{lighting:?}");
        }
    }

    #[test]
    fn a_cloaker_attacks_with_disadvantage_in_bright_light_and_sunlight() {
        use crate::effect::Lighting::*;
        assert_eq!(attack_disposition("cloaker", BrightLight), RollDisposition::Disadvantage);
        assert_eq!(attack_disposition("cloaker", Sunlight), RollDisposition::Disadvantage);
        assert_eq!(attack_disposition("cloaker", DimLight), RollDisposition::Normal);
    }

    #[test]
    fn a_lighting_attack_names_the_trait_and_others_do_not() {
        let dagger = |state: &mut EncounterState| {
            let effect = stat_block(state, &state.combatants[0]).unwrap().actions[0].effects[0].clone();
            let mut events = vec![];
            resolve_effect(state, &mut Stream::from_seed(3), "kobold-warrior", effect, &["fighter".into()], &mut events).unwrap();
            events.into_iter().find(|e| e.kind == "attack").unwrap().message
        };
        let sunlit = dagger(&mut lit(fixture_block("kobold-warrior"), crate::effect::Lighting::Sunlight));
        assert!(sunlit.ends_with("(Disadvantage: Sunlight Sensitivity, sunlight)"), "{sunlit}");
        let plain = dagger(&mut lit(fixture_block("kobold-warrior"), crate::effect::Lighting::Unspecified));
        assert!(!plain.contains("Sunlight"), "{plain}");
    }

    fn dark(darkvision: bool) -> EncounterState {
        let mut state = lit(fixture_block("kobold-warrior"), crate::effect::Lighting::Darkness);
        state.stat_blocks[0].stat_block.senses.darkvision = darkvision.then_some(60);
        state
    }

    #[test]
    fn darkness_blinds_a_creature_without_darkvision_to_everyone() {
        let state = dark(true);
        let (monster, fighter) = (&state.combatants[0], &state.combatants[1]);
        assert!(sees(&state, monster, fighter));
        assert_eq!(unseen_reason(&state, fighter, monster).unwrap(), "fighter cannot see kobold-warrior in darkness");
        assert_eq!(disposition(&state, monster, fighter, &sword()), RollDisposition::Advantage);
        assert_eq!(disposition(&state, fighter, monster, &sword()), RollDisposition::Disadvantage);
        let blind = dark(false);
        assert_eq!(disposition(&blind, &blind.combatants[0], &blind.combatants[1], &sword()), RollDisposition::Normal);
        assert_eq!(disposition(&blind, &blind.combatants[1], &blind.combatants[0], &sword()), RollDisposition::Normal);
    }

    #[test]
    fn darkness_names_one_reason_after_invisible_and_blinded() {
        let mut state = dark(false);
        let cond = |condition| ConditionInstance { condition, duration: Duration::Rounds(1), source: None, ends_on_damage: false, started_round: 0 };
        state.combatants[0].conditions.push(cond(Condition::Invisible));
        assert!(unseen_reason(&state, &state.combatants[1], &state.combatants[0]).unwrap().ends_with("who is invisible"));
        state.combatants[1].conditions.push(cond(Condition::Blinded));
        state.combatants[0].conditions.clear();
        assert!(unseen_reason(&state, &state.combatants[1], &state.combatants[0]).unwrap().contains("is blinded"));
        state.combatants[1].conditions.clear();
        state.stat_blocks[1].stat_block.senses.truesight = Some(30);
        assert!(sees(&state, &state.combatants[1], &state.combatants[0]));
    }

    #[test]
    fn darkness_stops_an_opportunity_attack_from_a_reactor_without_sight() {
        let mut state = provoking_encounter(0);
        state.settings.lighting = crate::effect::Lighting::Darkness;
        let err = opportunity_eligibility(&state, 1, 0).unwrap_err();
        assert!(matches!(&err, Ineligible::Unseen(why) if why.contains("in darkness")));
        state.stat_blocks.iter_mut().for_each(|b| b.stat_block.senses.darkvision = Some(60));
        assert!(opportunity_eligibility(&state, 1, 0).is_ok());
    }

    #[test]
    fn darkness_frees_a_ranged_attacker_whose_adjacent_enemy_cannot_see() {
        let mut state = dark(false);
        state.combatants[0].engagement.push("fighter".into());
        assert_eq!(disposition(&state, &state.combatants[0], &state.combatants[1], &bow()), RollDisposition::Normal);
        state.stat_blocks[1].stat_block.senses.darkvision = Some(60);
        assert_eq!(disposition(&state, &state.combatants[0], &state.combatants[1], &bow()), RollDisposition::Disadvantage);
    }

    fn dex_save() -> crate::effect::SavingThrow {
        crate::effect::SavingThrow {
            ability: Ability::Dex,
            dc: 15,
            half_on_success: false,
            no_damage_on_success: false,
            repeat_save: None,
            two_stage: None,
            immunity_after_success: false,
            hp_gate: None,
            creature_type_gate: None,
            damage: vec![],
            on_failure: vec![],
        }
    }

    fn save_of(state: &mut EncounterState, saver: &str, seed: u64) -> (EngineEvent, RngState) {
        let mut stream = Stream::from_seed(seed);
        let mut events = vec![];
        resolve_save(state, &mut stream, "fighter", saver, &dex_save(), &mut events).unwrap();
        (events.into_iter().find(|e| e.kind == "save").unwrap(), stream.state())
    }

    #[test]
    fn a_shadow_saves_with_disadvantage_in_sunlight_keeping_the_lower_of_two_d20s() {
        for seed in 0..20 {
            let mut probe = Stream::from_seed(seed);
            let (a, b) = (probe.roll_die(20), probe.roll_die(20));
            let (event, after) = save_of(&mut lit(fixture_block("shadow"), crate::effect::Lighting::Sunlight), "shadow", seed);
            assert_eq!(event.roll, Some(a.min(b)));
            assert_eq!(after, probe.state());
            assert!(event.message.ends_with("(Disadvantage: Sunlight Weakness, sunlight)"), "{}", event.message);
        }
    }

    #[test]
    fn a_straight_save_draws_one_d20() {
        for lighting in [crate::effect::Lighting::Unspecified, crate::effect::Lighting::BrightLight] {
            let mut probe = Stream::from_seed(9);
            let a = probe.roll_die(20);
            let (event, after) = save_of(&mut lit(fixture_block("shadow"), lighting), "shadow", 9);
            assert_eq!((event.roll, after), (Some(a), probe.state()));
            assert_eq!(event.message, format!("shadow {} the save", if event.total.unwrap() >= 15 { "passes" } else { "fails" }));
        }
    }

    fn save_trait(name: &str, disposition: &str) -> crate::stat_block::AbilityEntry {
        let mut entry = ability(name, vec![fx(serde_json::json!({"roll_modifier": {"lighting": "sunlight", "disposition": disposition, "target": {"saving_throws": {"ability": null}}}}))], None);
        entry.name = name.into();
        entry
    }

    #[test]
    fn save_sources_cancel_whatever_their_count_or_order() {
        let traits = [("a", "disadvantage"), ("b", "disadvantage"), ("c", "advantage")];
        for flip in [false, true] {
            for rotation in 0..3 {
                let mut block = test_stat_block("saver", "Saver", 20, 10);
                block.traits = traits.iter().map(|(n, d)| {
                    let other = if *d == "advantage" { "disadvantage" } else { "advantage" };
                    save_trait(n, if flip { other } else { d })
                }).collect();
                block.traits.rotate_left(rotation);
                let state = lit(block, crate::effect::Lighting::Sunlight);
                assert_eq!(save_disposition(&state, &state.combatants[0], Ability::Dex), RollDisposition::Normal, "flip {flip} rotation {rotation}");
            }
        }
    }

    #[test]
    fn a_pc_save_with_disadvantage_asks_for_two_d20s() {
        let mut block = pc_block("pc", 20);
        block.traits.push(save_trait("Glare", "disadvantage"));
        let mut state = lit(block, crate::effect::Lighting::Sunlight);
        state.settings.pc_dice = PcDice::Ask;
        save_of(&mut state, "pc", 1);
        assert_eq!(state.player_dice.next_prompt(), Some((20, "pc".to_string(), 2)));
    }

    #[test]
    fn a_sunlight_sensitive_concentration_save_rolls_with_disadvantage() {
        let concentration_save = |lighting: crate::effect::Lighting, seed: u64| {
            let mut block = test_stat_block("adept", "Adept", 50, 10);
            block.traits.push(save_trait("Sunlight Sensitivity", "disadvantage"));
            let mut state = lit(block, lighting);
            state.combatants[0].concentration = Some("bless".into());
            let mut stream = Stream::from_seed(seed);
            let mut events = vec![];
            apply_damage(&mut state, &mut stream, "adept", 5, Some("fighter"), false, false, &mut events).unwrap();
            (events.into_iter().find(|e| e.kind == "concentration_save").unwrap(), stream.state())
        };
        for seed in 0..10 {
            let mut probe = Stream::from_seed(seed);
            let (a, b) = (probe.roll_die(20), probe.roll_die(20));
            let (event, after) = concentration_save(crate::effect::Lighting::Sunlight, seed);
            assert_eq!((event.roll, after), (Some(a.min(b)), probe.state()));
            assert!(event.message.ends_with("(Disadvantage: Sunlight Sensitivity, sunlight)"), "{}", event.message);
            let mut probe = Stream::from_seed(seed);
            let a = probe.roll_die(20);
            let (event, after) = concentration_save(crate::effect::Lighting::Unspecified, seed);
            assert_eq!((event.roll, after), (Some(a), probe.state()));
        }
    }

    fn party_block(id: &str) -> StatBlock {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../data/party/standard/{id}.json"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    const CASTER: &str = "standard-caster-05";

    /// `attacker` (a fixture) on its turn beside `defender`, with reaction windows on.
    fn windowed(attacker: StatBlock, defender: StatBlock, defender_side: &str, seed: u64) -> EncounterState {
        let (a, d) = (attacker.id.clone(), defender.id.clone());
        let (a_hp, d_hp) = (attacker.hit_points.average, defender.hit_points.average);
        let mut state = test_encounter(
            vec![attacker, defender],
            vec![test_combatant(&a, &a, a_hp, "enemy"), test_combatant(&d, &d, d_hp, defender_side)],
        );
        state.settings.reaction_windows = true;
        state.rng = RngState::new(seed);
        state
    }

    fn answering(state: &EncounterState, answers: Vec<ReactionAnswer>) -> EncounterState {
        let mut state = state.clone();
        state.reactions.decider = crate::reaction::ReactionDecider::Queued(answers.into());
        state
    }

    fn attack_roll(events: &[EngineEvent]) -> (u32, i32) {
        let e = events.iter().find(|e| e.kind == "attack").expect("attack");
        (e.roll.unwrap(), e.total.unwrap())
    }

    /// The first seed whose ogre greatclub on the caster rolls a d20 that `keep` accepts.
    fn ogre_on_caster(keep: impl Fn(u32, i32) -> bool) -> EncounterState {
        for seed in 1..2000 {
            let state = windowed(fixture_block("ogre"), party_block(CASTER), "ally", seed);
            let mut probe = state.clone();
            probe.settings.reaction_windows = false;
            let (roll, total) = attack_roll(&act(&mut probe, "ogre", "greatclub", &[CASTER]).unwrap());
            if keep(roll, total) {
                return state;
            }
        }
        panic!("no seed");
    }

    fn shieldable() -> EncounterState {
        ogre_on_caster(|roll, total| roll != 20 && (12..17).contains(&total))
    }

    #[test]
    fn a_hit_the_caster_can_shield_waits_on_a_window_naming_shield() {
        let mut state = answering(&shieldable(), vec![]);
        let events = act(&mut state, "ogre", "greatclub", &[CASTER]).unwrap();
        let window = state.reactions.waiting.clone().expect("window");
        assert_eq!(window.reactor_id, CASTER);
        assert_eq!(window.options, vec![ReactionOption { id: "shield".into(), name: "Shield".into() }]);
        let (_, total) = attack_roll(&events);
        assert_eq!(window.trigger, WindowTrigger::Hit { attacker_id: "ogre".into(), total, ac: 12, critical: false });
    }

    #[test]
    fn taking_shield_turns_the_same_hit_into_a_miss_and_spends_the_reaction_and_slot() {
        let base = shieldable();
        let mut taken = answering(&base, vec![ReactionAnswer::Take("shield".into())]);
        let events = act(&mut taken, "ogre", "greatclub", &[CASTER]).unwrap();
        let kinds: Vec<&str> = events.iter().map(|e| e.kind.as_str()).collect();
        let reaction = kinds.iter().position(|k| *k == "reaction").expect("reaction");
        assert_eq!(events[reaction].message, format!("REACTION {CASTER}: Shield (hit by ogre)"));
        let recheck = events.iter().find(|e| e.kind == "attack_rechecked").expect("recheck");
        assert!(recheck.message.ends_with("vs AC 17 (miss) after Shield"), "{}", recheck.message);
        assert!(events.iter().all(|e| e.kind != "damage"), "{kinds:?}");
        let caster = &taken.combatants[1];
        assert!(caster.action_economy.reaction_used);
        assert_eq!(caster.resources[&format!("{CASTER}:slot:1")], 1);
        assert_eq!(caster.hp, caster.max_hp);

        let mut declined = answering(&base, vec![ReactionAnswer::Pass]);
        act(&mut declined, "ogre", "greatclub", &[CASTER]).unwrap();
        let mut before = base.clone();
        before.settings.reaction_windows = false;
        act(&mut before, "ogre", "greatclub", &[CASTER]).unwrap();
        assert!(declined.combatants[1].hp < declined.combatants[1].max_hp);
        assert_eq!(declined.combatants[1].hp, before.combatants[1].hp);
        assert_eq!(declined.rng, before.rng);
        assert!(!declined.combatants[1].action_economy.reaction_used);
    }

    #[test]
    fn a_natural_20_still_hits_after_shield() {
        let mut state = answering(&ogre_on_caster(|roll, _| roll == 20), vec![ReactionAnswer::Take("shield".into())]);
        let events = act(&mut state, "ogre", "greatclub", &[CASTER]).unwrap();
        assert!(events.iter().any(|e| e.kind == "reaction"));
        let recheck = events.iter().find(|e| e.kind == "attack_rechecked").unwrap();
        assert!(recheck.message.contains("(hit) after Shield"), "{}", recheck.message);
        assert!(events.iter().any(|e| e.kind == "damage"));
    }

    #[test]
    fn a_caster_that_cannot_react_or_cast_gets_no_window() {
        let base = shieldable();
        let setups: [fn(&mut EncounterState); 4] = [
            |s| s.combatants[1].action_economy.reaction_used = true,
            |s| {
                s.combatants[1].resources.insert(format!("{CASTER}:slot:1"), 4);
            },
            |s| s.combatants[1].conditions.push(ConditionInstance { condition: Condition::Incapacitated, duration: Duration::Permanent, source: None, ends_on_damage: false, started_round: 0 }),
            |s| s.active_modifiers.push(ActiveModifier { target: ModifierTarget::Reactions, change: ModifierChange::None, remaining: Duration::Rounds(1), source: Some(CASTER.into()), bearer: None, source_ability: None, ends_on_damage: false, on_end: vec![], started_round: 0 }),
        ];
        for setup in setups {
            let mut state = answering(&base, vec![]);
            setup(&mut state);
            act(&mut state, "ogre", "greatclub", &[CASTER]).unwrap();
            assert!(state.reactions.waiting.is_none());
        }
    }

    fn knight_state(seed: u64) -> EncounterState {
        windowed(fixture_block("ogre"), fixture_block("knight"), "ally", seed)
    }

    #[test]
    fn parry_opens_on_a_melee_hit_and_not_on_a_ranged_one() {
        for seed in 1..200 {
            let mut melee = answering(&knight_state(seed), vec![]);
            let events = act(&mut melee, "ogre", "greatclub", &["knight"]).unwrap();
            let hit = events.iter().any(|e| e.kind == "attack" && e.message.contains("(hit)"));
            assert_eq!(melee.reactions.waiting.as_ref().map(|w| w.options[0].id.as_str()), hit.then_some("parry"));
            let mut ranged = answering(&knight_state(seed), vec![]);
            act(&mut ranged, "ogre", "javelin", &["knight"]).unwrap();
            assert!(ranged.reactions.waiting.is_none());
        }
    }

    #[test]
    fn parry_raises_ac_against_the_triggering_attack_only() {
        let mut ogre = fixture_block("ogre");
        ogre.actions.push(ability("pummel", vec![fx(serde_json::json!({"multiattack": {"steps": [{"action": {"action_id": "fist", "count": 2}}]}}))], None));
        ogre.actions.push(smash(100, "1d4", 0));
        ogre.actions.last_mut().unwrap().id = "fist".into();
        let mut state = answering(&windowed(ogre, fixture_block("knight"), "ally", 1), vec![ReactionAnswer::Take("parry".into())]);
        let events = act(&mut state, "ogre", "pummel", &["knight"]).unwrap();
        let attacks: Vec<&str> = events.iter().filter(|e| e.kind.starts_with("attack")).map(|e| e.message.as_str()).collect();
        assert_eq!(attacks.len(), 3, "{attacks:?}");
        assert!(attacks[1].contains("vs AC 20 (hit) after Parry"), "{attacks:?}");
        assert!(attacks[2].contains("vs AC 18 ("), "{attacks:?}");
        assert!(state.active_modifiers.is_empty(), "{:?}", state.active_modifiers);
    }

    /// `a` moves away from `b` and `c`, initiative a, b, c, with windows on.
    fn windowed_move(claw_damage: i32) -> EncounterState {
        let mut state = provoking_encounter(claw_damage);
        state.settings.reaction_windows = true;
        answering(&state, vec![])
    }

    fn move_away(state: &mut EncounterState, opportunity: &[OpportunityAttack], declined: &[&str]) -> Vec<EngineEvent> {
        let declined: Vec<String> = declined.iter().map(|d| d.to_string()).collect();
        move_combatant(state, "a", None, RangeBand::Far, 35, opportunity, &declined).unwrap()
    }

    #[test]
    fn riposte_shows_its_agent_clause_after_the_recheck() {
        for seed in 1..400 {
            let mut state = answering(&windowed(fixture_block("ogre"), fixture_block("pirate-captain"), "ally", seed), vec![ReactionAnswer::Take("riposte".into())]);
            let events = act(&mut state, "ogre", "greatclub", &["pirate-captain"]).unwrap();
            let Some(recheck) = events.iter().position(|e| e.kind == "attack_rechecked") else { continue };
            let clause = events.iter().position(|e| e.kind == "agent_clause").expect("clause");
            assert_eq!(clause, recheck + 1);
            assert!(events[clause].message.starts_with("pirate-captain Riposte, agent clause: On a miss"), "{}", events[clause].message);
            return;
        }
        panic!("no seed hit");
    }

    #[test]
    fn opportunity_windows_open_in_initiative_order_before_any_attack() {
        let base = windowed_move(1);
        let mut first = base.clone();
        assert!(move_away(&mut first, &[], &[]).is_empty());
        assert_eq!(first.reactions.waiting.as_ref().unwrap().reactor_id, "b");
        assert_eq!(first.reactions.waiting.as_ref().unwrap().trigger, WindowTrigger::Opportunity { mover_id: "a".into() });
        let mut second = base.clone();
        assert!(move_away(&mut second, &[oa("b", Some("claw"))], &[]).is_empty(), "b's attack waits for c's window");
        assert_eq!(second.reactions.waiting.as_ref().unwrap().reactor_id, "c");
        assert_eq!(second.combatants[0].hp, base.combatants[0].hp);
    }

    #[test]
    fn a_reactor_named_on_the_move_gets_no_window() {
        let mut state = windowed_move(1);
        move_away(&mut state, &[oa("c", None)], &[]);
        assert_eq!(state.reactions.waiting.as_ref().unwrap().reactor_id, "b");
    }

    #[test]
    fn taking_one_window_and_declining_the_other_makes_one_attack() {
        let mut state = windowed_move(1);
        let events = move_away(&mut state, &[oa("b", Some("claw"))], &["c"]);
        assert!(state.reactions.waiting.is_none());
        assert_eq!(events.iter().filter(|e| e.kind == "attack").count(), 1);
        assert!(state.combatants[1].action_economy.reaction_used);
        assert!(!state.combatants[2].action_economy.reaction_used);
        assert!(events.iter().all(|e| e.actor_id.as_deref() != Some("c")), "a declined reactor leaves no notice: {events:?}");
        assert_eq!(state.combatants[0].ranges["b"], RangeBand::Far);
    }

    #[test]
    fn a_mover_downed_by_a_window_attack_stops() {
        let mut state = windowed_move(50);
        let events = move_away(&mut state, &[oa("b", Some("claw")), oa("c", Some("claw"))], &[]);
        assert_eq!(events.last().unwrap().kind, "move_stopped");
        assert!(!state.combatants[2].action_economy.reaction_used);
        assert_eq!(state.combatants[0].ranges["b"], RangeBand::Engaged);
    }

    #[test]
    fn passing_every_window_moves_like_before_with_the_notices() {
        let mut passing = provoking_encounter(1);
        passing.settings.reaction_windows = true;
        let mut before = provoking_encounter(1);
        let passed = move_away(&mut passing, &[], &[]);
        let old = move_away(&mut before, &[], &[]);
        assert_eq!(serde_json::to_value(&passed).unwrap(), serde_json::to_value(&old).unwrap());
        assert_eq!(passed.iter().filter(|e| e.kind == "opportunity_attack").count(), 2);
        assert_eq!(passing.game_hash(), { before.settings.reaction_windows = true; before.game_hash() });
    }

    #[test]
    fn a_window_attack_that_hits_the_moving_caster_opens_shield_inside_the_move() {
        for seed in 1..400 {
            let mut state = windowed(fixture_block("ogre"), party_block(CASTER), "ally", seed);
            state.active_turn = Some(CASTER.into());
            set_band(&mut state, 1, CASTER, "ogre", RangeBand::Engaged).unwrap();
            let state = answering(&state, vec![]);
            let mut run = state.clone();
            let mover = |s: &mut EncounterState, oas: &[OpportunityAttack]| move_combatant(s, CASTER, None, RangeBand::Near, 5, oas, &[]).unwrap();
            assert!(mover(&mut run, &[]).is_empty());
            assert_eq!(run.reactions.waiting.as_ref().unwrap().reactor_id, "ogre");
            let mut taken = state.clone();
            let events = mover(&mut taken, &[oa("ogre", Some("greatclub"))]);
            if !events.iter().any(|e| e.kind == "attack" && e.message.contains("(hit)")) {
                continue;
            }
            let window = taken.reactions.waiting.expect("shield window");
            assert_eq!((window.reactor_id.as_str(), window.options[0].id.as_str()), (CASTER, "shield"));
            let mut shielded = answering(&state, vec![ReactionAnswer::Take("shield".into())]);
            let events = mover(&mut shielded, &[oa("ogre", Some("greatclub"))]);
            let kinds: Vec<&str> = events.iter().map(|e| e.kind.as_str()).collect();
            assert!(kinds.windows(2).any(|w| w == ["attack", "reaction"]), "{kinds:?}");
            assert!(kinds.contains(&"movement"), "{kinds:?}");
            return;
        }
        panic!("no seed hit");
    }

    #[test]
    fn windows_stay_shut_when_the_encounter_does_not_open_them() {
        let mut state = answering(&shieldable(), vec![]);
        state.settings.reaction_windows = false;
        act(&mut state, "ogre", "greatclub", &[CASTER]).unwrap();
        assert!(state.reactions.waiting.is_none());
        let mut moving = provoking_encounter(1);
        moving.reactions.decider = crate::reaction::ReactionDecider::Queued(Default::default());
        assert!(!move_away(&mut moving, &[], &[]).is_empty());
        assert!(moving.reactions.waiting.is_none());
    }

    #[test]
    fn an_old_stored_move_without_declined_still_loads() {
        let old = serde_json::json!({"move": {"actor_id": "a", "target_id": null, "band": "far", "feet": 35}});
        let request: StepRequest = serde_json::from_value(old.clone()).unwrap();
        assert!(matches!(&request, StepRequest::Move { declined, .. } if declined.is_empty()));
        assert_eq!(serde_json::to_value(&request).unwrap(), old);
    }

    fn by_policy(state: &EncounterState) -> EncounterState {
        let mut state = state.clone();
        state.reactions.decider = crate::reaction::ReactionDecider::Policy(Default::default());
        state
    }

    fn reasons(events: &[EngineEvent]) -> Vec<&str> {
        events.iter().filter(|e| e.kind == "reaction_reason").map(|e| e.message.as_str()).collect()
    }

    #[test]
    fn the_policy_shields_a_hit_inside_the_band_and_misses_it() {
        let mut state = by_policy(&shieldable());
        let events = act(&mut state, "ogre", "greatclub", &[CASTER]).unwrap();
        let (_, total) = attack_roll(&events);
        assert_eq!(reasons(&events), vec![format!("{CASTER}: Shield ({total} vs AC 12+5 turns the hit into a miss)")]);
        assert!(events.iter().any(|e| e.kind == "attack_rechecked" && e.message.contains("(miss) after Shield")));
        let caster = &state.combatants[1];
        assert!(caster.action_economy.reaction_used);
        assert_eq!(caster.resources[&format!("{CASTER}:slot:1")], 1);
        assert_eq!(caster.hp, caster.max_hp);
    }

    #[test]
    fn the_policy_holds_shield_on_a_hit_it_cannot_stop_or_a_critical() {
        let unstoppable = |roll: u32, total: i32| roll != 20 && total >= 17;
        let critical = |roll: u32, _: i32| roll == 20;
        for (state, why) in [(ogre_on_caster(unstoppable), "still hits"), (ogre_on_caster(critical), "critical hit")] {
            let mut state = by_policy(&state);
            let events = act(&mut state, "ogre", "greatclub", &[CASTER]).unwrap();
            let lines = reasons(&events);
            assert!(lines.len() == 1 && lines[0].starts_with(&format!("{CASTER}: holds its reaction (")) && lines[0].contains(why), "{lines:?}");
            assert!(!state.combatants[1].action_economy.reaction_used);
            assert!(state.combatants[1].hp < state.combatants[1].max_hp);
        }
        let mut spent = by_policy(&shieldable());
        spent.combatants[1].resources.insert(format!("{CASTER}:slot:1"), 4);
        let events = act(&mut spent, "ogre", "greatclub", &[CASTER]).unwrap();
        assert!(reasons(&events).is_empty() && !spent.combatants[1].action_economy.reaction_used);
        assert!(spent.combatants[1].hp < spent.combatants[1].max_hp);
    }

    #[test]
    fn the_policy_parries_melee_hits_inside_its_band_only() {
        let (mut inside, mut outside) = (false, false);
        for seed in 1..300 {
            let mut melee = by_policy(&knight_state(seed));
            let events = act(&mut melee, "ogre", "greatclub", &["knight"]).unwrap();
            let lines = reasons(&events);
            if let Some(attack) = events.iter().find(|e| e.kind == "attack" && e.message.contains("(hit)")) {
                let (roll, total) = (attack.roll.unwrap(), attack.total.unwrap());
                if roll != 20 && total < 20 {
                    assert_eq!(lines, vec![format!("knight: Parry ({total} vs AC 18+2 turns the hit into a miss)")]);
                    assert!(melee.combatants[1].action_economy.reaction_used);
                    inside = true;
                } else {
                    assert!(lines[0].starts_with("knight: holds its reaction"), "{lines:?}");
                    assert!(!melee.combatants[1].action_economy.reaction_used);
                    outside = true;
                }
            }
            let mut ranged = by_policy(&knight_state(seed));
            let events = act(&mut ranged, "ogre", "javelin", &["knight"]).unwrap();
            assert!(reasons(&events).is_empty());
        }
        assert!(inside && outside);
    }

    #[test]
    fn of_two_reactions_that_stop_the_hit_the_one_without_a_slot_wins() {
        let mut caster = party_block(CASTER);
        let mut deflect = fixture_block("knight").reactions[0].clone();
        deflect.id = "deflect".into();
        deflect.name = "Deflect".into();
        deflect.trigger = Some(crate::effect::ReactionTrigger::HitByAttackRoll { attack: crate::effect::AttackFilter::Any });
        caster.reactions.push(deflect);
        for seed in 1..2000 {
            let state = windowed(fixture_block("ogre"), caster.clone(), "ally", seed);
            let mut probe = state.clone();
            probe.settings.reaction_windows = false;
            let (roll, total) = attack_roll(&act(&mut probe, "ogre", "greatclub", &[CASTER]).unwrap());
            if roll == 20 || !(12..14).contains(&total) {
                continue;
            }
            let mut state = by_policy(&state);
            let events = act(&mut state, "ogre", "greatclub", &[CASTER]).unwrap();
            assert_eq!(reasons(&events), vec![format!("{CASTER}: Deflect ({total} vs AC 12+2 turns the hit into a miss)")]);
            assert!(!state.combatants[1].resources.contains_key(&format!("{CASTER}:slot:1")));
            return;
        }
        panic!("no seed");
    }

    #[test]
    fn a_mage_casts_shield_through_protective_magic_when_it_stops_the_hit() {
        for seed in 1..2000 {
            let state = windowed(fixture_block("ogre"), fixture_block("mage"), "ally", seed);
            let ac = fixture_block("mage").armor_class[0].value;
            let mut probe = state.clone();
            probe.settings.reaction_windows = false;
            let (roll, total) = attack_roll(&act(&mut probe, "ogre", "greatclub", &["mage"]).unwrap());
            if roll == 20 || !(ac..ac + 5).contains(&total) {
                continue;
            }
            let mut state = by_policy(&state);
            let events = act(&mut state, "ogre", "greatclub", &["mage"]).unwrap();
            assert_eq!(reasons(&events), vec![format!("mage: Protective Magic (Shield) ({total} vs AC {ac}+5 turns the hit into a miss)")]);
            assert!(events.iter().any(|e| e.kind == "attack_rechecked" && e.message.contains("(miss) after Protective Magic")), "{events:?}");
            assert!(!events.iter().any(|e| e.kind == "agent_clause"), "{events:?}");
            assert_eq!(state.combatants[1].resources.get("mage:protective-magic"), Some(&1));
            assert!(state.combatants[1].action_economy.reaction_used);
            return;
        }
        panic!("no seed");
    }

    #[test]
    fn a_mage_out_of_protective_magic_uses_opens_no_hit_window() {
        for seed in 1..2000 {
            let mut state = windowed(fixture_block("ogre"), fixture_block("mage"), "ally", seed);
            state.combatants[1].resources.insert("mage:protective-magic".into(), 3);
            let mut state = by_policy(&state);
            let events = act(&mut state, "ogre", "greatclub", &["mage"]).unwrap();
            if events.iter().any(|e| e.kind == "attack" && e.message.contains("(hit)")) {
                assert!(reasons(&events).is_empty(), "{events:?}");
                assert!(!state.combatants[1].action_economy.reaction_used);
                return;
            }
        }
        panic!("no seed");
    }

    /// `mover` (on its turn) engaged with `reactor`, both from blocks, windows on, the policy deciding.
    fn stepping_out(mover: StatBlock, reactor: StatBlock) -> EncounterState {
        let (m, r) = (mover.id.clone(), reactor.id.clone());
        let mut state = windowed(mover, reactor, "ally", 3);
        set_band(&mut state, 0, &m, &r, RangeBand::Engaged).unwrap();
        state.active_turn = Some(m);
        by_policy(&state)
    }

    fn step_out(state: &mut EncounterState) -> Vec<EngineEvent> {
        let mover = state.combatants[0].id.clone();
        move_combatant(state, &mover, None, RangeBand::Near, 5, &[], &[]).unwrap()
    }

    #[test]
    fn a_goblin_with_no_hit_reaction_attacks_a_caster_stepping_out() {
        let mut state = stepping_out(party_block(CASTER), fixture_block("goblin-warrior"));
        let events = step_out(&mut state);
        let lines = reasons(&events);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].starts_with(&format!("goblin-warrior: Opportunity Attack on {CASTER} (scimitar, expected ")) && lines[0].ends_with("> reserve 0.0)"), "{lines:?}");
        assert!(events.iter().any(|e| e.kind == "reaction" && e.message.starts_with("OPPORTUNITY ATTACK goblin-warrior")));
        assert!(state.combatants[1].action_economy.reaction_used);
    }

    #[test]
    fn a_caster_holds_its_reaction_for_shield_until_its_slots_run_out() {
        let base = stepping_out(fixture_block("ogre"), party_block(CASTER));
        let mut held = base.clone();
        let events = step_out(&mut held);
        let lines = reasons(&events);
        assert!(lines.len() == 1 && lines[0].starts_with(&format!("{CASTER}: holds its reaction against ogre (dagger, expected ")), "{lines:?}");
        assert!(events.iter().all(|e| e.kind != "opportunity_attack"), "a held reaction leaves no notice: {events:?}");
        assert!(!held.combatants[1].action_economy.reaction_used);
        let mut spent = base.clone();
        spent.combatants[1].resources.insert(format!("{CASTER}:slot:1"), 4);
        let events = step_out(&mut spent);
        assert!(reasons(&events)[0].starts_with(&format!("{CASTER}: Opportunity Attack on ogre (dagger, expected ")), "{:?}", reasons(&events));
        assert!(spent.combatants[1].action_economy.reaction_used);
    }

    #[test]
    fn the_opportunity_attack_is_the_higher_damage_melee_leaf() {
        let mut reactor = fixture_block("goblin-warrior");
        reactor.actions.insert(0, smash(4, "1d4", 0));
        reactor.actions[0].id = "jab".into();
        reactor.actions.push(smash(4, "2d10", 2));
        reactor.actions.last_mut().unwrap().id = "slam".into();
        let mut state = stepping_out(party_block(CASTER), reactor);
        let events = step_out(&mut state);
        assert!(reasons(&events)[0].contains("(slam, expected "), "{:?}", reasons(&events));
    }

    fn borne(bearer: &str, source: &str, ability: Option<&str>, target: ModifierTarget, change: ModifierChange) -> ActiveModifier {
        ActiveModifier {
            target,
            change,
            remaining: Duration::Concentration,
            source: Some(source.into()),
            bearer: Some(bearer.into()),
            source_ability: ability.map(Into::into),
            ends_on_damage: false,
            on_end: vec![],
            started_round: 0,
        }
    }

    fn sure_caster(id: &str, spell: &str) -> StatBlock {
        let mut block = slotted_caster(spell, Some(3));
        block.id = id.into();
        block.spellcasting.as_mut().unwrap().save_dc = 40;
        block
    }

    fn caster_and(ally: bool, spell: &str) -> EncounterState {
        let side = if ally { "ally" } else { "enemy" };
        test_encounter(
            vec![sure_caster("mage", spell), test_stat_block("other", "Other", 40, 12)],
            vec![test_combatant("mage", "mage", 30, "ally"), test_combatant("other", "other", 40, side)],
        )
    }

    fn speed_of(state: &EncounterState, id: &str) -> u32 {
        walk_speed(state, &state.combatants[index_of(state, id).unwrap()])
    }

    fn ac_of(state: &EncounterState, id: &str) -> i32 {
        ac_modifier(state, &state.combatants[index_of(state, id).unwrap()])
    }

    #[test]
    fn slow_lands_on_the_target_that_fails_and_not_on_the_caster() {
        let mut state = caster_and(false, "slow");
        act(&mut state, "mage", "slow", &["other"]).unwrap();
        let other = &state.combatants[1];
        assert_eq!(ac_of(&state, "other"), -2);
        assert_eq!(save_modifier(&state, other, Ability::Dex), -2);
        assert_eq!(speed_of(&state, "other"), 15);
        assert_eq!(can_react(&state, other).unwrap_err(), "other cannot take reactions");
        let mage = &state.combatants[0];
        assert_eq!((ac_of(&state, "mage"), save_modifier(&state, mage, Ability::Dex), speed_of(&state, "mage")), (0, 0, 30));
        assert!(can_react(&state, mage).is_ok());
        assert!(state.active_modifiers.iter().all(|m| m.source.as_deref() == Some("mage") && m.bearer() == Some("other")));
    }

    #[test]
    fn haste_on_an_ally_buffs_the_ally_until_the_caster_loses_concentration() {
        let mut state = caster_and(true, "haste");
        let events = act(&mut state, "mage", "haste", &["other"]).unwrap();
        assert!(events.iter().any(|e| e.message == "other gains haste from mage"), "{events:?}");
        assert_eq!((ac_of(&state, "other"), speed_of(&state, "other")), (2, 60));
        assert_eq!((ac_of(&state, "mage"), speed_of(&state, "mage")), (0, 30));
        drop_concentration(&mut state, "mage");
        assert_eq!((ac_of(&state, "other"), speed_of(&state, "other")), (0, 0), "lethargy roots it");
        assert_eq!(state.active_modifiers.len(), 1);
        assert_eq!(state.active_modifiers[0].source_ability.as_deref(), Some("haste-lethargy"));
    }

    #[test]
    fn shield_still_lands_on_its_caster() {
        let mut state = caster_and(false, "shield");
        act(&mut state, "mage", "shield", &["other"]).unwrap();
        assert_eq!((ac_of(&state, "mage"), ac_of(&state, "other")), (5, 0));
        assert_eq!(state.active_modifiers[0].bearer.as_deref(), Some("mage"));
    }

    #[test]
    fn a_top_level_target_modifier_with_no_target_is_refused() {
        let mut state = caster_and(true, "haste");
        let err = act(&mut state, "mage", "haste", &[]).unwrap_err();
        assert!(matches!(&err, EngineError::Illegal(m) if m == "haste needs a target for its modifier"), "{err:?}");
    }

    #[test]
    fn a_triggering_attack_modifier_lands_on_its_reactor_even_without_a_bearer() {
        let mut state = caster_and(false, "shield");
        let parry = fx(serde_json::json!({"timed_modifier": {"target": "armor_class", "change": {"add": 2}, "duration": "triggering_attack"}}));
        let mut stream = Stream::from_state(state.rng.clone());
        resolve_effect(&mut state, &mut stream, "mage", parry, &["other".into()], &mut vec![]).unwrap();
        assert_eq!((ac_of(&state, "mage"), ac_of(&state, "other")), (2, 0));
    }

    #[test]
    fn two_hastes_on_one_bearer_apply_once_and_the_other_survives_either_ending() {
        for ending in ["m1", "m2"] {
            let mut state = test_encounter(
                vec![sure_caster("m1", "haste"), sure_caster("m2", "haste"), test_stat_block("other", "Other", 40, 12)],
                vec![test_combatant("m1", "m1", 30, "ally"), test_combatant("m2", "m2", 30, "ally"), test_combatant("other", "other", 40, "ally")],
            );
            act(&mut state, "m1", "haste", &["other"]).unwrap();
            state.active_turn = Some("m2".into());
            act(&mut state, "m2", "haste", &["other"]).unwrap();
            assert_eq!(state.active_modifiers.len(), 8);
            assert_eq!((ac_of(&state, "other"), speed_of(&state, "other")), (2, 60));
            drop_concentration(&mut state, ending);
            assert_eq!(state.active_modifiers.iter().filter(|m| m.source_ability.as_deref() == Some("haste")).count(), 4);
            assert_eq!(ac_of(&state, "other"), 2, "after {ending} stops");
            assert!(extra_action_uses(&state, "other").is_none(), "lethargy incapacitates it");
        }
    }

    #[test]
    fn only_the_most_potent_of_one_ability_on_a_bearer_applies() {
        let mut state = caster_and(true, "haste");
        let ac = |n| borne("other", "mage", Some("ward"), ModifierTarget::ArmorClass, ModifierChange::Add(n));
        state.active_modifiers = vec![ac(1), ac(3)];
        assert_eq!(ac_of(&state, "other"), 3);
        state.active_modifiers = vec![ac(3), ac(1)];
        assert_eq!(ac_of(&state, "other"), 3);
        state.active_modifiers.remove(0);
        assert_eq!(ac_of(&state, "other"), 1);
        state.active_modifiers = vec![ac(2), ac(2)];
        assert_eq!(ac_of(&state, "other"), 2);
        let mut unnamed = ac(2);
        unnamed.source_ability = None;
        state.active_modifiers = vec![unnamed.clone(), unnamed];
        assert_eq!(ac_of(&state, "other"), 4, "modifiers with no source ability always apply");
        let set = |n| borne("other", "mage", Some("root"), ModifierTarget::Speed, ModifierChange::Set(n));
        state.active_modifiers = vec![set(10), set(0), set(20)];
        assert_eq!(speed_of(&state, "other"), 0);
        state.active_modifiers = vec![set(60), set(40)];
        assert_eq!(speed_of(&state, "other"), 60, "a set above walk speed is a buff, so the higher is more potent");
        state.active_modifiers[1].source_ability = Some("stride".into());
        assert_eq!(speed_of(&state, "other"), 40, "across abilities the lowest set still wins outright");
    }

    #[test]
    fn the_most_potent_penalty_is_the_largest() {
        let mut state = caster_and(false, "slow");
        let ac = |n| borne("other", "mage", Some("curse"), ModifierTarget::ArmorClass, ModifierChange::Add(n));
        state.active_modifiers = vec![ac(-3), ac(-1)];
        assert_eq!(ac_of(&state, "other"), -3);
        state.active_modifiers = vec![ac(-1), ac(-3)];
        assert_eq!(ac_of(&state, "other"), -3);
        state.active_modifiers.pop();
        assert_eq!(ac_of(&state, "other"), -1);
    }

    #[test]
    fn a_legacy_speed_modifier_stays_inert() {
        let mut state = caster_and(false, "slow");
        let mut legacy = borne("mage", "mage", Some("slow"), ModifierTarget::Speed, ModifierChange::Halve);
        legacy.bearer = None;
        let mut ac = borne("mage", "mage", Some("slow"), ModifierTarget::ArmorClass, ModifierChange::Add(-2));
        ac.bearer = None;
        state.active_modifiers = vec![legacy, ac];
        assert_eq!((speed_of(&state, "mage"), ac_of(&state, "mage")), (30, -2), "as engine 0.1.5 read it");
    }

    #[test]
    fn hypnotic_patterns_speed_0_ends_with_its_conditions_on_damage() {
        let mut state = caster_and(false, "hypnotic-pattern");
        act(&mut state, "mage", "hypnotic-pattern", &["other"]).unwrap();
        assert_eq!(speed_of(&state, "other"), 0);
        assert_eq!(state.combatants[1].conditions.len(), 2);
        let mut stream = Stream::from_state(state.rng.clone());
        apply_damage(&mut state, &mut stream, "other", 3, Some("mage"), false, false, &mut vec![]).unwrap();
        assert!(state.combatants[1].conditions.is_empty());
        assert!(state.active_modifiers.is_empty());
        assert_eq!(speed_of(&state, "other"), 30);
    }

    #[test]
    fn walk_speed_reads_set_add_double_and_halve() {
        let mut state = caster_and(true, "haste");
        let speed = |n: &str, change| borne("other", "mage", Some(n), ModifierTarget::Speed, change);
        for (changes, feet) in [
            (vec![speed("a", ModifierChange::Halve)], 15),
            (vec![speed("a", ModifierChange::Double)], 60),
            (vec![speed("a", ModifierChange::Halve), speed("b", ModifierChange::Double)], 30),
            (vec![speed("a", ModifierChange::Double), speed("b", ModifierChange::Set(0))], 0),
            (vec![speed("a", ModifierChange::Add(-40))], 0),
            (vec![speed("a", ModifierChange::Add(10)), speed("b", ModifierChange::Double)], 80),
        ] {
            state.active_modifiers = changes;
            assert_eq!(speed_of(&state, "other"), feet, "{:?}", state.active_modifiers);
        }
        state.active_modifiers = vec![speed("a", ModifierChange::Double)];
        state.combatants[1].conditions.push(ConditionInstance { condition: Condition::Grappled, duration: Duration::Permanent, source: None, ends_on_damage: false, started_round: 0 });
        assert_eq!(speed_of(&state, "other"), 0);
    }

    #[test]
    fn a_hasted_creature_moves_60_feet_before_it_attacks() {
        let mut runner = test_stat_block("runner", "Runner", 20, 12);
        runner.actions.push(smash(4, "1d4", 0));
        runner.actions.push(ability("charge", vec![fx(serde_json::json!({"move_then_attack": {"move_feet": 60, "attack_id": "smash"}}))], None));
        let mut state = test_encounter(
            vec![runner, test_stat_block("ogre", "Ogre", 50, 10)],
            vec![test_combatant("runner", "runner", 20, "ally"), test_combatant("ogre", "ogre", 50, "enemy")],
        );
        let err = act(&mut state.clone(), "runner", "charge", &["ogre"]).unwrap_err();
        assert!(matches!(err, EngineError::Illegal(_)), "{err:?}");
        state.active_modifiers.push(borne("runner", "ogre", Some("haste"), ModifierTarget::Speed, ModifierChange::Double));
        act(&mut state, "runner", "charge", &["ogre"]).unwrap();
        assert_eq!(state.combatants[0].action_economy.movement_used_feet, 60);
    }

    #[test]
    fn a_modifier_serializes_without_default_bearer_fields() {
        let mut legacy = borne("a", "a", None, ModifierTarget::ArmorClass, ModifierChange::Add(2));
        legacy.bearer = None;
        let keys: Vec<String> = serde_json::to_value(&legacy).unwrap().as_object().unwrap().keys().cloned().collect();
        assert_eq!(keys, ["change", "remaining", "source", "target"]);
        let own = serde_json::to_value(borne("a", "a", None, ModifierTarget::ArmorClass, ModifierChange::Add(2))).unwrap();
        let keys: Vec<&String> = own.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["bearer", "change", "remaining", "source", "target"]);
        let mut other = borne("b", "a", Some("haste"), ModifierTarget::ArmorClass, ModifierChange::Add(2));
        other.ends_on_damage = true;
        let value = serde_json::to_value(&other).unwrap();
        assert_eq!((value["bearer"].as_str(), value["source_ability"].as_str(), value["ends_on_damage"].as_bool()), (Some("b"), Some("haste"), Some(true)));
        let effect = serde_json::to_value(fx(serde_json::json!({"timed_modifier": {"target": "armor_class", "change": {"add": 2}, "duration": "concentration"}}))).unwrap();
        let keys: Vec<&String> = effect["timed_modifier"].as_object().unwrap().keys().collect();
        assert_eq!(keys, ["change", "duration", "target"]);
    }

    #[test]
    fn fly_casts_with_no_modeled_effects() {
        let mut state = caster_and(true, "fly");
        let events = act(&mut state, "mage", "fly", &["other"]).unwrap();
        assert!(events.iter().any(|e| e.message == "mage begins concentrating on fly"), "{events:?}");
        assert!(state.active_modifiers.is_empty());
    }

    fn hasted(casters: &[&str]) -> EncounterState {
        let mut other = test_stat_block("other", "Other", 40, 12);
        other.actions.push(smash(4, "1d4", 0));
        other.actions.push(ability("flurry", vec![fx(serde_json::json!({"multiattack": {"steps": [{"action": {"action_id": "smash", "count": 2}}]}}))], None));
        other.actions.push(ability("bolt", vec![Effect::Cast(cast("fire-bolt", None))], None));
        other.actions.push(ability("sweep", vec![fx(serde_json::json!({"attack": {"modifier": 4, "reach": 5, "target_count": 2, "damage": [{"dice": "1d4", "type": "slashing"}]}}))], None));
        let mut blocks: Vec<StatBlock> = casters.iter().map(|id| sure_caster(id, "haste")).collect();
        blocks.extend([other, test_stat_block("foe", "Foe", 500, 1)]);
        let mut combatants: Vec<Combatant> = casters.iter().map(|id| test_combatant(id, id, 30, "ally")).collect();
        combatants.extend([test_combatant("other", "other", 40, "ally"), test_combatant("foe", "foe", 500, "enemy")]);
        let mut state = test_encounter(blocks, combatants);
        for id in casters {
            state.active_turn = Some(id.to_string());
            act(&mut state, id, "haste", &["other"]).unwrap();
        }
        state
    }

    fn extra(state: &mut EncounterState, actor: &str, action: &str, targets: &[&str]) -> Result<Vec<EngineEvent>, EngineError> {
        let target_ids = targets.iter().map(|t| t.to_string()).collect();
        step_events(state, StepRequest::Action { actor_id: actor.into(), action_id: action.into(), target_ids, slot_level: None, extra: true })
    }

    fn refusal(result: Result<Vec<EngineEvent>, EngineError>) -> String {
        match result {
            Err(EngineError::Illegal(m)) => m,
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    fn drop_concentration(state: &mut EncounterState, id: &str) -> Vec<EngineEvent> {
        let mut stream = Stream::from_state(state.rng.clone());
        let mut events = vec![];
        break_concentration(state, &mut stream, id, &mut events).unwrap();
        state.rng = stream.state();
        events
    }

    fn end_turn(state: &mut EncounterState) {
        step_events(state, StepRequest::end_turn()).unwrap();
    }

    fn incapacitated(state: &EncounterState, id: &str) -> bool {
        state.combatants[index_of(state, id).unwrap()].conditions.iter().any(|c| c.condition == Condition::Incapacitated)
    }

    #[test]
    fn a_hasted_ally_makes_one_extra_attack_after_its_action() {
        let mut state = hasted(&["mage"]);
        state.active_turn = Some("other".into());
        assert_eq!(extra_action_uses(&state, "other"), Some(vec![ExtraActionUse::OneAttack, ExtraActionUse::Disengage]));
        act(&mut state, "other", "smash", &["foe"]).unwrap();
        let events = extra(&mut state, "other", "smash", &["foe"]).unwrap();
        assert!(events.iter().any(|e| e.message == "other uses its extra action"), "{events:?}");
        assert_eq!(events.iter().filter(|e| e.kind == "attack").count(), 1);
        let economy = &state.combatants[1].action_economy;
        assert!(economy.action_used && economy.extra_action_used);
        assert_eq!(extra_action_uses(&state, "other"), None);
        assert_eq!(refusal(extra(&mut state, "other", "smash", &["foe"])), "extra action already used");
    }

    #[test]
    fn the_extra_action_refuses_a_multiattack_a_spell_and_a_two_target_attack() {
        let mut state = hasted(&["mage"]);
        state.active_turn = Some("other".into());
        for action in ["flurry", "bolt", "sweep"] {
            assert_eq!(refusal(extra(&mut state, "other", action, &["foe"])), "other's extra action allows only one_attack, disengage", "{action}");
        }
        assert!(!state.combatants[1].action_economy.extra_action_used);
    }

    #[test]
    fn disengage_as_the_extra_action_leaves_the_action_unused() {
        let mut state = hasted(&["mage"]);
        state.active_turn = Some("other".into());
        step_events(&mut state, StepRequest::Disengage { actor_id: "other".into(), extra: true }).unwrap();
        let economy = &state.combatants[1].action_economy;
        assert!(economy.disengaged && economy.extra_action_used && !economy.action_used);
        act(&mut state, "other", "smash", &["foe"]).unwrap();
    }

    #[test]
    fn without_haste_the_extra_action_is_refused() {
        let mut state = hasted(&[]);
        state.active_turn = Some("other".into());
        assert_eq!(extra_action_uses(&state, "other"), None);
        assert_eq!(refusal(extra(&mut state, "other", "smash", &["foe"])), "other has no extra action");
        let disengage = step_events(&mut state, StepRequest::Disengage { actor_id: "other".into(), extra: true });
        assert_eq!(refusal(disengage), "other has no extra action");
    }

    #[test]
    fn lethargy_from_the_casters_turn_lasts_through_the_allys_next_turn() {
        let mut state = hasted(&["mage"]);
        let events = drop_concentration(&mut state, "mage");
        assert!(events.iter().any(|e| e.message == "haste ends on other"), "{events:?}");
        assert!(incapacitated(&state, "other"));
        assert_eq!(speed_of(&state, "other"), 0);
        assert_eq!(ac_of(&state, "other"), 0);
        assert_eq!(extra_action_uses(&state, "other"), None);
        end_turn(&mut state);
        assert_eq!(state.active_turn.as_deref(), Some("other"));
        assert!(incapacitated(&state, "other"));
        assert_eq!(refusal(act(&mut state, "other", "smash", &["foe"])), "other cannot act");
        end_turn(&mut state);
        assert!(!incapacitated(&state, "other"));
        assert_eq!(speed_of(&state, "other"), 30);
        assert!(state.active_modifiers.is_empty());
    }

    #[test]
    fn lethargy_from_the_allys_own_turn_lasts_through_its_following_turn() {
        let mut state = hasted(&["mage"]);
        state.active_turn = Some("other".into());
        drop_concentration(&mut state, "mage");
        end_turn(&mut state);
        assert!(incapacitated(&state, "other"), "the turn it began on does not count");
        end_turn(&mut state);
        end_turn(&mut state);
        assert_eq!(state.active_turn.as_deref(), Some("other"));
        assert!(incapacitated(&state, "other"));
        assert_eq!(speed_of(&state, "other"), 0);
        end_turn(&mut state);
        assert!(!incapacitated(&state, "other"));
        assert_eq!(speed_of(&state, "other"), 30);
    }

    #[test]
    fn two_hastes_give_one_extra_action_and_either_ending_brings_lethargy() {
        let mut state = hasted(&["m1", "m2"]);
        state.active_turn = Some("other".into());
        extra(&mut state, "other", "smash", &["foe"]).unwrap();
        assert_eq!(refusal(extra(&mut state, "other", "smash", &["foe"])), "extra action already used");
        drop_concentration(&mut state, "m1");
        assert!(incapacitated(&state, "other"));
        assert_eq!(ac_of(&state, "other"), 2, "the other Haste still applies");
        assert_eq!(speed_of(&state, "other"), 0);
    }

    #[test]
    fn an_empty_extra_action_list_allows_a_multiattack() {
        let mut state = hasted(&[]);
        let quicken = fx(serde_json::json!({"timed_modifier": {"target": {"extra_action": {"only": []}}, "change": "none", "duration": "until_end_of_turn", "bearer": "self"}}));
        let block = &mut state.stat_blocks[0];
        block.stat_block.bonus_actions.push(ability("quicken", vec![quicken], None));
        block.hash = StatBlockSnapshot::from_stat_block(block.id.clone(), block.stat_block.clone()).hash;
        state.active_turn = Some("other".into());
        act(&mut state, "other", "quicken", &[]).unwrap();
        assert_eq!(extra_action_uses(&state, "other"), Some(vec![]));
        let events = extra(&mut state, "other", "flurry", &["foe"]).unwrap();
        assert_eq!(events.iter().filter(|e| e.kind == "attack").count(), 2);
        end_turn(&mut state);
        assert!(state.active_modifiers.is_empty(), "until_end_of_turn ends with the bearer's turn");
    }

    #[test]
    fn haste_carries_lethargy_on_its_extra_action_modifier_only() {
        let state = hasted(&["mage"]);
        let with_end: Vec<&ActiveModifier> = state.active_modifiers.iter().filter(|m| !m.on_end.is_empty()).collect();
        assert_eq!(state.active_modifiers.len(), 4);
        assert_eq!(with_end.len(), 1);
        assert!(matches!(with_end[0].target, ModifierTarget::ExtraAction { .. }));
    }

    /// `fighter` (ally) attacks `goblin` (enemy); neither is engaged unless `setup` engages them.
    fn duel(setup: impl FnOnce(&mut Vec<StatBlock>, &mut Vec<Combatant>)) -> EncounterState {
        let mut blocks = vec![test_stat_block("fighter", "Fighter", 20, 16), test_stat_block("goblin", "Goblin", 7, 15)];
        let mut combatants = vec![test_combatant("fighter", "fighter", 20, "ally"), test_combatant("goblin", "goblin", 7, "enemy")];
        setup(&mut blocks, &mut combatants);
        test_encounter(blocks, combatants)
    }

    fn duel_disposition(setup: impl FnOnce(&mut Vec<StatBlock>, &mut Vec<Combatant>)) -> RollDisposition {
        let state = duel(setup);
        disposition(&state, &state.combatants[0], &state.combatants[1], &sword())
    }

    fn engage_duel(combatants: &mut [Combatant]) {
        combatants[0].engagement.push("goblin".into());
        combatants[1].engagement.push("fighter".into());
    }

    fn sourced(condition: Condition, source: Option<&str>) -> ConditionInstance {
        ConditionInstance { condition, duration: Duration::Permanent, source: source.map(Into::into), ends_on_damage: false, started_round: 0 }
    }

    #[test]
    fn blinded_and_petrified_targets_give_advantage() {
        assert_eq!(duel_disposition(|_, c| c[1].conditions.push(sourced(Condition::Blinded, None))), RollDisposition::Advantage);
        assert_eq!(duel_disposition(|_, c| c[1].conditions.push(sourced(Condition::Petrified, None))), RollDisposition::Advantage);
    }

    #[test]
    fn a_restrained_attacker_has_disadvantage() {
        assert_eq!(duel_disposition(|_, c| c[0].conditions.push(sourced(Condition::Restrained, None))), RollDisposition::Disadvantage);
    }

    #[test]
    fn a_blinded_attacker_with_blindsight_keeps_disadvantage() {
        let d = duel_disposition(|b, c| {
            b[0].senses.blindsight = Some(10);
            c[0].conditions.push(sourced(Condition::Blinded, None));
        });
        assert_eq!(d, RollDisposition::Disadvantage);
    }

    #[test]
    fn a_prone_target_gives_advantage_only_within_engagement() {
        let prone = |c: &mut Vec<Combatant>| c[1].conditions.push(sourced(Condition::Prone, None));
        assert_eq!(duel_disposition(|_, c| { prone(c); engage_duel(c); }), RollDisposition::Advantage);
        assert_eq!(
            duel_disposition(|_, c| {
                prone(c);
                c[0].ranges.insert("goblin".into(), RangeBand::Near);
                c[1].ranges.insert("fighter".into(), RangeBand::Near);
            }),
            RollDisposition::Disadvantage
        );
        assert_eq!(duel_disposition(|_, c| { prone(c); c[1].engagement.push("fighter".into()); }), RollDisposition::Advantage);
    }

    #[test]
    fn a_grappled_attacker_rolls_straight_only_against_its_grappler() {
        assert_eq!(duel_disposition(|_, c| c[0].conditions.push(sourced(Condition::Grappled, Some("goblin")))), RollDisposition::Normal);
        assert_eq!(duel_disposition(|_, c| c[0].conditions.push(sourced(Condition::Grappled, Some("ogre")))), RollDisposition::Disadvantage);
        assert_eq!(duel_disposition(|_, c| c[0].conditions.push(sourced(Condition::Grappled, None))), RollDisposition::Disadvantage);
    }

    #[test]
    fn a_frightened_attacker_has_disadvantage_while_it_sees_the_source() {
        let with_ogre = |b: &mut Vec<StatBlock>, c: &mut Vec<Combatant>, source: Option<&str>| {
            b.push(test_stat_block("ogre", "Ogre", 30, 11));
            c.push(test_combatant("ogre", "ogre", 30, "enemy"));
            c[0].conditions.push(sourced(Condition::Frightened, source));
        };
        assert_eq!(duel_disposition(|b, c| with_ogre(b, c, Some("ogre"))), RollDisposition::Disadvantage);
        assert_eq!(
            duel_disposition(|b, c| {
                with_ogre(b, c, Some("ogre"));
                c[2].conditions.push(sourced(Condition::Invisible, None));
            }),
            RollDisposition::Normal
        );
        assert_eq!(duel_disposition(|b, c| with_ogre(b, c, Some("gone"))), RollDisposition::Normal);
        assert_eq!(
            duel_disposition(|b, c| {
                with_ogre(b, c, Some("ogre"));
                c[2].end_state = EndState::Dead;
            }),
            RollDisposition::Disadvantage
        );
        assert_eq!(duel_disposition(|b, c| with_ogre(b, c, None)), RollDisposition::Disadvantage);
    }

    #[test]
    fn invisible_counts_only_against_an_observer_without_blindsight_or_truesight() {
        let attacker_invisible = |c: &mut Vec<Combatant>| c[0].conditions.push(sourced(Condition::Invisible, None));
        let target_invisible = |c: &mut Vec<Combatant>| c[1].conditions.push(sourced(Condition::Invisible, None));
        assert_eq!(duel_disposition(|_, c| attacker_invisible(c)), RollDisposition::Advantage);
        assert_eq!(duel_disposition(|b, c| { b[1].senses.truesight = Some(120); attacker_invisible(c); }), RollDisposition::Normal);
        assert_eq!(duel_disposition(|b, c| { b[1].senses.blindsight = Some(10); attacker_invisible(c); }), RollDisposition::Normal);
        assert_eq!(duel_disposition(|b, c| { b[1].senses.tremorsense = Some(60); attacker_invisible(c); }), RollDisposition::Advantage);
        assert_eq!(duel_disposition(|_, c| target_invisible(c)), RollDisposition::Disadvantage);
        assert_eq!(duel_disposition(|b, c| { b[0].senses.blindsight = Some(10); target_invisible(c); }), RollDisposition::Normal);
        assert_eq!(duel_disposition(|b, c| { b[0].senses.tremorsense = Some(60); target_invisible(c); }), RollDisposition::Disadvantage);
    }

    fn sees_invisible_trait(block: &mut StatBlock) {
        block.traits.push(crate::stat_block::AbilityEntry { effects: vec![Effect::SeeInvisible], ..no_provoke_trait() });
    }

    fn see_invisibility(state: &mut EncounterState, bearer: &str) {
        state.active_modifiers.push(borne(bearer, bearer, Some("see-invisibility"), ModifierTarget::SeeInvisible, ModifierChange::None));
    }

    #[test]
    fn see_invisible_counts_from_a_trait_or_a_modifier_on_the_observer() {
        let attacker_invisible = |c: &mut Vec<Combatant>| c[0].conditions.push(sourced(Condition::Invisible, None));
        assert_eq!(duel_disposition(|b, c| { sees_invisible_trait(&mut b[1]); attacker_invisible(c); }), RollDisposition::Normal);

        let mut cast = duel(|_, c| attacker_invisible(c));
        see_invisibility(&mut cast, "goblin");
        assert_eq!(disposition(&cast, &cast.combatants[0], &cast.combatants[1], &sword()), RollDisposition::Normal);

        let mut wrong_bearer = duel(|_, c| attacker_invisible(c));
        see_invisibility(&mut wrong_bearer, "fighter");
        assert_eq!(disposition(&wrong_bearer, &wrong_bearer.combatants[0], &wrong_bearer.combatants[1], &sword()), RollDisposition::Advantage);
    }

    #[test]
    fn see_invisible_helps_neither_a_blinded_observer_nor_one_in_darkness() {
        let blinded = duel(|b, c| {
            sees_invisible_trait(&mut b[0]);
            c[0].conditions.push(sourced(Condition::Blinded, None));
        });
        assert_eq!(unseen_reason(&blinded, &blinded.combatants[0], &blinded.combatants[1]).as_deref(), Some("fighter is blinded and cannot see goblin"));

        let mut dark = duel(|b, c| {
            sees_invisible_trait(&mut b[0]);
            c[1].conditions.push(sourced(Condition::Invisible, None));
        });
        assert_eq!(unseen_reason(&dark, &dark.combatants[0], &dark.combatants[1]), None);
        dark.settings.lighting = Lighting::Darkness;
        assert_eq!(unseen_reason(&dark, &dark.combatants[0], &dark.combatants[1]).as_deref(), Some("fighter cannot see goblin in darkness"));
    }

    #[test]
    fn see_invisibility_is_cast_on_the_caster_without_concentration_and_lasts_an_hour_of_rounds() {
        let mut state = duel(|_, c| c[1].conditions.push(sourced(Condition::Invisible, None)));
        state.settings.long_durations = true;
        state.active_turn = Some("fighter".into());
        let cast = Effect::Cast(crate::effect::Cast { spell_id: "see-invisibility".into(), slot_level: None, upcast: false });
        resolve_effect(&mut state, &mut Stream::from_state(RngState::new(1)), "fighter", cast, &[], &mut vec![]).unwrap();
        assert!(state.combatants[0].concentration.is_none());
        assert!(sees(&state, &state.combatants[0], &state.combatants[1]));
        finish_turn(&mut state).unwrap();
        assert!(sees(&state, &state.combatants[0], &state.combatants[1]), "an hour is 600 rounds, so one turn does not end it");
    }

    #[test]
    fn see_invisible_makes_opportunity_attacks_on_an_invisible_mover_eligible() {
        let mut cast = provoking_encounter(0);
        cast.combatants[0].conditions.push(sourced(Condition::Invisible, None));
        assert!(!opportunity_eligible(&cast, "b", "a"));
        see_invisibility(&mut cast, "b");
        assert!(opportunity_eligible(&cast, "b", "a"));
    }

    #[test]
    fn a_downed_target_counts_as_unconscious_and_prone() {
        for down in [EndState::Unconscious, EndState::Stable] {
            assert_eq!(duel_disposition(|_, c| { c[1].end_state = down; engage_duel(c); }), RollDisposition::Advantage, "{down:?}");
            assert_eq!(
                duel_disposition(|_, c| {
                    c[1].end_state = down;
                    c[0].ranges.insert("goblin".into(), RangeBand::Far);
                    c[1].ranges.insert("fighter".into(), RangeBand::Far);
                }),
                RollDisposition::Normal,
                "{down:?}"
            );
            let state = duel(|_, c| c[1].end_state = down);
            assert!(state.combatants[1].conditions.is_empty(), "nothing is pushed into conditions");
        }
    }

    fn bare_attack(reach: Option<u32>, range: Option<(u32, u32)>) -> crate::effect::Attack {
        crate::effect::Attack { modifier: 0, reach, range, target_count: 1, damage: vec![], on_hit: vec![] }
    }

    fn bow() -> crate::effect::Attack {
        bare_attack(None, Some((80, 320)))
    }

    fn sword() -> crate::effect::Attack {
        bare_attack(Some(5), None)
    }

    /// The fighter shoots the goblin with `attack`.
    fn shot_disposition(attack: &crate::effect::Attack, setup: impl FnOnce(&mut Vec<StatBlock>, &mut Vec<Combatant>)) -> RollDisposition {
        let state = duel(setup);
        disposition(&state, &state.combatants[0], &state.combatants[1], attack)
    }

    #[test]
    fn an_engaged_bow_shot_has_disadvantage_and_a_thrown_or_melee_weapon_at_the_engaged_foe_does_not() {
        assert_eq!(shot_disposition(&bow(), |_, c| engage_duel(c)), RollDisposition::Disadvantage);
        assert_eq!(shot_disposition(&bare_attack(Some(5), Some((20, 60))), |_, c| engage_duel(c)), RollDisposition::Normal);
        assert_eq!(shot_disposition(&sword(), |_, c| engage_duel(c)), RollDisposition::Normal);
    }

    #[test]
    fn an_engaged_creature_throwing_at_a_foe_it_is_not_engaged_with_has_disadvantage() {
        let state = duel(|b, c| {
            b.push(test_stat_block("ogre", "Ogre", 30, 11));
            c.push(test_combatant("ogre", "ogre", 30, "enemy"));
            engage_duel(c);
        });
        let javelin = bare_attack(Some(5), Some((30, 120)));
        let (thrower, goblin, ogre) = (&state.combatants[0], &state.combatants[1], &state.combatants[2]);
        assert_eq!(disposition(&state, thrower, ogre, &javelin), RollDisposition::Disadvantage);
        assert_eq!(disposition(&state, thrower, goblin, &javelin), RollDisposition::Normal);
        assert_eq!(disposition_with(&state, thrower, ogre, &javelin, Within::HeldAnd("ogre")), RollDisposition::Normal, "moving in makes it a melee attack");
        let alone = duel(|b, c| {
            b.push(test_stat_block("ogre", "Ogre", 30, 11));
            c.push(test_combatant("ogre", "ogre", 30, "enemy"));
        });
        assert_eq!(disposition(&alone, &alone.combatants[0], &alone.combatants[2], &javelin), RollDisposition::Normal, "nobody engaged");
    }

    #[test]
    fn a_shot_from_the_near_band_rolls_straight() {
        let near = |c: &mut Vec<Combatant>| {
            c[0].ranges.insert("goblin".into(), RangeBand::Near);
            c[1].ranges.insert("fighter".into(), RangeBand::Near);
        };
        assert_eq!(shot_disposition(&bow(), |_, c| near(c)), RollDisposition::Normal);
    }

    #[test]
    fn engagement_recorded_only_on_the_enemy_counts_for_close_combat() {
        assert_eq!(shot_disposition(&bow(), |_, c| c[1].engagement.push("fighter".into())), RollDisposition::Disadvantage);
    }

    #[test]
    fn an_engaged_enemy_that_cannot_act_gives_no_close_combat_disadvantage() {
        for condition in [Condition::Incapacitated, Condition::Paralyzed, Condition::Stunned, Condition::Unconscious, Condition::Petrified] {
            // Each of these also gives Advantage against the goblin when it is the target, so the
            // shot goes at a second enemy, the ogre, from an archer engaged only with the goblin.
            let state = duel(|b, c| {
                b.push(test_stat_block("ogre", "Ogre", 30, 11));
                c.push(test_combatant("ogre", "ogre", 30, "enemy"));
                engage_duel(c);
                c[1].conditions.push(sourced(condition, None));
            });
            assert_eq!(disposition(&state, &state.combatants[0], &state.combatants[2], &bow()), RollDisposition::Normal, "{condition:?}");
        }
        for end in [EndState::Unconscious, EndState::Stable, EndState::Dead] {
            let state = duel(|b, c| {
                b.push(test_stat_block("ogre", "Ogre", 30, 11));
                c.push(test_combatant("ogre", "ogre", 30, "enemy"));
                engage_duel(c);
                c[1].end_state = end;
            });
            assert_eq!(disposition(&state, &state.combatants[0], &state.combatants[2], &bow()), RollDisposition::Normal, "{end:?}");
        }
    }

    #[test]
    fn an_engaged_enemy_that_cannot_see_the_archer_gives_no_close_combat_disadvantage() {
        let invisible = |c: &mut Vec<Combatant>| c[0].conditions.push(sourced(Condition::Invisible, None));
        // An unseen archer keeps Invisible's Advantage and takes no close-combat Disadvantage.
        // A goblin with Truesight sees it: Advantage goes and the close-combat source applies.
        assert_eq!(shot_disposition(&bow(), |_, c| { engage_duel(c); invisible(c); }), RollDisposition::Advantage);
        assert_eq!(shot_disposition(&bow(), |b, c| { b[1].senses.truesight = Some(60); engage_duel(c); invisible(c); }), RollDisposition::Disadvantage);
        let blinded = duel(|b, c| {
            b.push(test_stat_block("ogre", "Ogre", 30, 11));
            c.push(test_combatant("ogre", "ogre", 30, "enemy"));
            engage_duel(c);
            c[1].conditions.push(sourced(Condition::Blinded, None));
        });
        assert_eq!(disposition(&blinded, &blinded.combatants[0], &blinded.combatants[2], &bow()), RollDisposition::Normal);
    }

    #[test]
    fn an_engaged_ally_gives_no_close_combat_disadvantage() {
        let d = shot_disposition(&bow(), |b, c| {
            b.push(test_stat_block("squire", "Squire", 10, 12));
            c.push(test_combatant("squire", "squire", 10, "ally"));
            c[0].engagement.push("squire".into());
        });
        assert_eq!(d, RollDisposition::Normal);
    }

    #[test]
    fn two_engaged_enemies_give_one_disadvantage_that_advantage_cancels() {
        let two = |b: &mut Vec<StatBlock>, c: &mut Vec<Combatant>| {
            b.push(test_stat_block("ogre", "Ogre", 30, 11));
            c.push(test_combatant("ogre", "ogre", 30, "enemy"));
            engage_duel(c);
            c[2].engagement.push("fighter".into());
        };
        assert_eq!(shot_disposition(&bow(), two), RollDisposition::Disadvantage);
        assert_eq!(
            shot_disposition(&bow(), |b, c| {
                two(b, c);
                c[1].conditions.push(sourced(Condition::Restrained, None));
            }),
            RollDisposition::Normal
        );
    }

    #[test]
    fn disposition_with_reads_close_combat_from_where_the_attacker_is_left() {
        let state = duel(|b, c| {
            b.push(test_stat_block("ogre", "Ogre", 30, 11));
            c.push(test_combatant("ogre", "ogre", 30, "enemy"));
            engage_duel(c);
        });
        let (archer, ogre) = (&state.combatants[0], &state.combatants[2]);
        assert_eq!(disposition_with(&state, archer, ogre, &bow(), Within::Held), RollDisposition::Disadvantage);
        assert_eq!(disposition_with(&state, archer, ogre, &bow(), Within::Nobody), RollDisposition::Normal);
        let alone = duel(|b, c| {
            b.push(test_stat_block("ogre", "Ogre", 30, 11));
            c.push(test_combatant("ogre", "ogre", 30, "enemy"));
        });
        let (archer, ogre) = (&alone.combatants[0], &alone.combatants[2]);
        assert_eq!(disposition_with(&alone, archer, ogre, &bow(), Within::Held), RollDisposition::Normal);
        assert_eq!(disposition_with(&alone, archer, ogre, &bow(), Within::HeldAnd("ogre")), RollDisposition::Disadvantage);
    }

    #[test]
    fn a_close_combat_shot_names_its_source_on_the_attack_event() {
        let mut state = duel(|b, c| {
            b[0].actions.push(ability("bow", vec![Effect::Attack(bow())], None));
            engage_duel(c);
        });
        let resp = step(&mut state, StepRequest::Action { actor_id: "fighter".into(), action_id: "bow".into(), target_ids: vec!["goblin".into()], slot_level: None, extra: false }).unwrap();
        let attack = resp.events.iter().find(|e| e.kind == "attack").expect("attack event");
        assert!(attack.message.ends_with("(Disadvantage: ranged attack in close combat)"), "{}", attack.message);
    }

    /// A one-damage bite (1d1) the goblin rolls against the fighter, on the first seed whose
    /// natural d20 satisfies `natural`. Returns the state after it, the attack event and every event.
    fn bite_on(state: &EncounterState, natural: impl Fn(u32) -> bool) -> (EncounterState, EngineEvent, Vec<EngineEvent>) {
        for seed in 1..5000 {
            let mut trial = state.clone();
            trial.rng = RngState::new(seed);
            let resp = step(&mut trial, StepRequest::Action { actor_id: "goblin".into(), action_id: "bite".into(), target_ids: vec!["fighter".into()], slot_level: None, extra: false }).unwrap();
            let attack = resp.events.iter().find(|e| e.kind == "attack").expect("attack event").clone();
            if natural(attack.roll.unwrap()) {
                return (trial, attack, resp.events);
            }
        }
        panic!("no seed rolled the wanted natural d20");
    }

    fn one_damage_bite(setup: impl FnOnce(&mut StatBlock, &mut Combatant)) -> EncounterState {
        attack_state(vec![gated("1d1", 0, DamageType::Piercing, None, false)], vec![], setup)
    }

    fn disengage_bite(state: &mut EncounterState, band: RangeBand) {
        state.combatants[0].engagement.clear();
        state.combatants[1].engagement.clear();
        state.combatants[0].ranges.insert("goblin".into(), band);
        state.combatants[1].ranges.insert("fighter".into(), band);
    }

    fn fighter_of(state: &EncounterState) -> &Combatant {
        state.combatants.iter().find(|c| c.id == "fighter").unwrap()
    }

    #[test]
    fn an_engaged_hit_on_a_paralyzed_target_is_critical() {
        let paralyzed = one_damage_bite(|_, f| f.conditions.push(sourced(Condition::Paralyzed, None)));
        let (after, attack, _) = bite_on(&paralyzed, |n| n == 2);
        assert!(attack.message.contains("(hit) (critical: paralyzed, within 5 feet)"), "{}", attack.message);
        assert_eq!(fighter_of(&after).hp, 98, "a critical rolls the damage dice twice");

        let mut near = paralyzed.clone();
        disengage_bite(&mut near, RangeBand::Near);
        let (after, attack, _) = bite_on(&near, |n| n == 2);
        assert!(attack.message.ends_with("(hit)"), "{}", attack.message);
        assert_eq!(fighter_of(&after).hp, 99);
    }

    #[test]
    fn an_engaged_hit_on_a_downed_pc_is_critical_with_two_failures() {
        let downed = one_damage_bite(|_, f| {
            f.hp = 0;
            f.end_state = EndState::Unconscious;
        });
        let (after, attack, _) = bite_on(&downed, |n| n > 1 && n < 20);
        assert!(attack.message.contains("(critical: unconscious, within 5 feet)"), "{}", attack.message);
        assert_eq!(fighter_of(&after).death_saves.failures, 2);

        let mut far = downed.clone();
        disengage_bite(&mut far, RangeBand::Far);
        let (after, attack, _) = bite_on(&far, |n| n > 1 && n < 20);
        assert!(attack.message.ends_with("(hit)"), "{}", attack.message);
        assert_eq!(fighter_of(&after).death_saves.failures, 1);
    }

    #[test]
    fn damage_to_a_stable_pc_makes_it_dying_with_a_failure() {
        let stable = one_damage_bite(|_, f| {
            f.hp = 0;
            f.end_state = EndState::Stable;
        });
        let mut far = stable.clone();
        disengage_bite(&mut far, RangeBand::Far);
        let (after, _, events) = bite_on(&far, |n| n > 1 && n < 20);
        let fighter = fighter_of(&after);
        assert_eq!((fighter.end_state, fighter.death_saves.failures, fighter.hp), (EndState::Unconscious, 1, 0));
        assert!(events.iter().any(|e| e.message == "fighter is no longer stable"), "{events:?}");

        let (after, _, _) = bite_on(&stable, |n| n > 1 && n < 20);
        assert_eq!((fighter_of(&after).end_state, fighter_of(&after).death_saves.failures), (EndState::Unconscious, 2));

        let mut failing = far.clone();
        failing.combatants[0].death_saves.failures = 2;
        let (after, _, _) = bite_on(&failing, |n| n > 1 && n < 20);
        assert_eq!(fighter_of(&after).end_state, EndState::Dead);
    }

    #[test]
    fn damage_at_or_over_max_hp_kills_a_downed_pc() {
        let downed = |end_state, max_hp, temp_hp| {
            let mut state = one_damage_bite(|_, f| {
                f.hp = 0;
                f.max_hp = max_hp;
                f.temp_hp = temp_hp;
                f.end_state = end_state;
            });
            disengage_bite(&mut state, RangeBand::Far);
            state
        };
        let hit = |n: u32| n > 1 && n < 20;

        let (after, _, events) = bite_on(&downed(EndState::Unconscious, 1, 0), hit);
        assert_eq!(fighter_of(&after).end_state, EndState::Dead);
        assert!(events.iter().any(|e| e.message == "fighter dies from massive damage"), "{events:?}");

        let (after, _, _) = bite_on(&downed(EndState::Unconscious, 2, 0), hit);
        assert_eq!((fighter_of(&after).end_state, fighter_of(&after).death_saves.failures), (EndState::Unconscious, 1));

        let (after, _, _) = bite_on(&downed(EndState::Stable, 1, 0), hit);
        assert_eq!(fighter_of(&after).end_state, EndState::Dead);

        let (after, _, _) = bite_on(&downed(EndState::Unconscious, 1, 1), hit);
        let fighter = fighter_of(&after);
        assert_eq!((fighter.end_state, fighter.death_saves.failures, fighter.temp_hp), (EndState::Unconscious, 1, 0), "temporary HP soaks the hit first");

        let engaged = one_damage_bite(|_, f| {
            f.hp = 0;
            f.max_hp = 2;
            f.end_state = EndState::Unconscious;
        });
        let (after, attack, events) = bite_on(&engaged, hit);
        assert!(attack.message.contains("(critical: unconscious, within 5 feet)"), "{}", attack.message);
        assert_eq!(fighter_of(&after).end_state, EndState::Dead, "the critical's 2 damage meets max HP 2");
        assert!(events.iter().any(|e| e.message == "fighter dies from massive damage"), "{events:?}");
        assert_eq!(fighter_of(&after).death_saves.failures, 0, "massive damage kills without counting failures");

        let soaked = one_damage_bite(|_, f| {
            f.hp = 0;
            f.max_hp = 1;
            f.temp_hp = 1;
            f.end_state = EndState::Unconscious;
        });
        let (after, _, _) = bite_on(&soaked, hit);
        assert_eq!(fighter_of(&after).end_state, EndState::Dead, "1 temporary HP leaves 1 of the critical's 2 damage, meeting max HP 1");
    }

    #[test]
    fn dropping_to_zero_hit_points_ends_concentration_without_a_save() {
        let concentrating = |dying: bool, max_hp: i32| {
            let mut state = one_damage_bite(|b, f| {
                b.dying = dying;
                f.hp = 1;
                f.max_hp = max_hp;
                f.concentration = Some("haste".into());
            });
            disengage_bite(&mut state, RangeBand::Far);
            state
        };
        for (state, natural, end_state) in [
            (concentrating(true, 100), 10, EndState::Unconscious),
            (concentrating(true, 1), 20, EndState::Dead),
            (concentrating(false, 100), 10, EndState::Dead),
        ] {
            let (after, _, events) = bite_on(&state, |n| n == natural);
            let fighter = fighter_of(&after);
            assert_eq!((fighter.end_state, fighter.concentration.as_deref()), (end_state, None), "{events:?}");
            assert!(events.iter().any(|e| e.message == "fighter loses concentration"), "{events:?}");
            assert!(events.iter().all(|e| e.kind != "concentration_save"), "{events:?}");
        }
    }

    #[test]
    fn an_incapacitating_condition_ends_concentration() {
        let condition = |condition| Effect::Condition(crate::effect::ConditionEffect { condition, duration: Duration::Permanent, ends_on_damage: false, source_ability: None });
        let mut state = caster_and(true, "haste");
        act(&mut state, "mage", "haste", &["other"]).unwrap();
        assert!(state.combatants[0].concentration.is_some());
        let mut events = vec![];
        resolve_effect(&mut state, &mut Stream::from_state(RngState::new(1)), "other", condition(Condition::Stunned), &["mage".to_string()], &mut events).unwrap();
        assert_eq!(state.combatants[0].concentration, None);
        assert!(events.iter().any(|e| e.message == "mage loses concentration"), "{events:?}");
        assert!(!state.active_modifiers.iter().any(|m| m.source.as_deref() == Some("mage") && matches!(m.remaining, Duration::Concentration)), "Haste ends");

        let mut prone = caster_and(true, "haste");
        act(&mut prone, "mage", "haste", &["other"]).unwrap();
        resolve_effect(&mut prone, &mut Stream::from_state(RngState::new(1)), "other", condition(Condition::Prone), &["mage".to_string()], &mut vec![]).unwrap();
        assert!(prone.combatants[0].concentration.is_some(), "Prone doesn't incapacitate");
    }

    #[test]
    fn opportunity_attacks_use_the_sight_rule() {
        let mut truesight = provoking_encounter(0);
        truesight.stat_blocks[1].stat_block.senses.truesight = Some(60);
        truesight.combatants[0].conditions.push(sourced(Condition::Invisible, None));
        assert!(opportunity_eligible(&truesight, "b", "a"));
        assert!(!opportunity_eligible(&truesight, "c", "a"));

        let mut blindsight = provoking_encounter(0);
        blindsight.stat_blocks[1].stat_block.senses.blindsight = Some(10);
        blindsight.combatants[1].conditions.push(sourced(Condition::Blinded, None));
        assert!(opportunity_eligible(&blindsight, "b", "a"));

        let mut blinded = provoking_encounter(0);
        blinded.combatants[1].conditions.push(sourced(Condition::Blinded, None));
        let events = move_combatant(&mut blinded, "a", None, RangeBand::Far, 35, &[], &[]).unwrap();
        assert!(events.iter().any(|e| e.message == "ASSUMPTION b is blinded and cannot see a, so it makes no Opportunity Attack on a"), "{events:?}");
    }

    #[test]
    fn haste_gives_its_bearer_advantage_on_dexterity_saves_and_the_caster_none() {
        let mut state = caster_and(true, "haste");
        act(&mut state, "mage", "haste", &["other"]).unwrap();
        let disposition_of = |id: &str, ability| save_disposition(&state, &state.combatants[index_of(&state, id).unwrap()], ability);
        assert_eq!(disposition_of("other", Ability::Dex), RollDisposition::Advantage);
        assert_eq!(disposition_of("other", Ability::Str), RollDisposition::Normal);
        assert_eq!(disposition_of("mage", Ability::Dex), RollDisposition::Normal);
    }

    fn save_on(ability: Ability) -> crate::effect::SavingThrow {
        crate::effect::SavingThrow { ability, ..dex_save() }
    }

    fn saver_with(conditions: &[Condition]) -> EncounterState {
        let mut state = lit(test_stat_block("saver", "Saver", 50, 10), crate::effect::Lighting::Unspecified);
        state.combatants[0].conditions.extend(conditions.iter().map(|c| sourced(*c, None)));
        state
    }

    /// The `saver` makes `save` against the fighter on `seed`: its save event, every event, and the stream after it.
    fn save_by(state: &mut EncounterState, save: crate::effect::SavingThrow, seed: u64) -> (EngineEvent, Vec<EngineEvent>, RngState) {
        let mut stream = Stream::from_seed(seed);
        let mut events = vec![];
        resolve_save(state, &mut stream, "fighter", "saver", &save, &mut events).unwrap();
        (events.iter().find(|e| e.kind == "save").unwrap().clone(), events, stream.state())
    }

    #[test]
    fn a_restrained_target_has_disadvantage_on_dexterity_saves_only() {
        for seed in 0..10 {
            let mut probe = Stream::from_seed(seed);
            let (a, b) = (probe.roll_die(20), probe.roll_die(20));
            let (event, _, after) = save_by(&mut saver_with(&[Condition::Restrained]), save_on(Ability::Dex), seed);
            assert_eq!((event.roll, after), (Some(a.min(b)), probe.state()));
            let mut probe = Stream::from_seed(seed);
            let a = probe.roll_die(20);
            let (event, _, after) = save_by(&mut saver_with(&[Condition::Restrained]), save_on(Ability::Str), seed);
            assert_eq!((event.roll, after), (Some(a), probe.state()));
        }
    }

    #[test]
    fn an_advantage_source_cancels_restrained_on_a_dexterity_save() {
        let mut sunlit = saver_with(&[Condition::Restrained]);
        sunlit.stat_blocks[0].stat_block.traits.push(save_trait("Sun Glow", "advantage"));
        sunlit.settings.lighting = crate::effect::Lighting::Sunlight;
        let mut hasted = saver_with(&[Condition::Restrained]);
        hasted.active_modifiers.push(borne("saver", "fighter", Some("haste"), ModifierTarget::SavingThrows { ability: Some(Ability::Dex) }, ModifierChange::Advantage));
        for state in [&sunlit, &hasted] {
            assert_eq!(save_disposition(state, &state.combatants[0], Ability::Dex), RollDisposition::Normal);
            let mut probe = Stream::from_seed(4);
            let a = probe.roll_die(20);
            let (event, _, after) = save_by(&mut state.clone(), save_on(Ability::Dex), 4);
            assert_eq!((event.roll, after), (Some(a), probe.state()));
        }
    }

    #[test]
    fn incapacitating_conditions_fail_strength_and_dexterity_saves_without_a_die() {
        let named = [(Condition::Paralyzed, "paralyzed"), (Condition::Petrified, "petrified"), (Condition::Stunned, "stunned"), (Condition::Unconscious, "unconscious")];
        for (condition, name) in named {
            for (ability, word) in [(Ability::Str, "Strength"), (Ability::Dex, "Dexterity")] {
                let (event, _, after) = save_by(&mut saver_with(&[condition]), save_on(ability), 5);
                assert_eq!(after, Stream::from_seed(5).state(), "{name} {word} draws no die");
                assert_eq!(event.message, format!("saver fails the save automatically ({name}, {word})"));
                assert_eq!((event.roll, event.total), (None, None));
            }
        }
    }

    #[test]
    fn a_paralyzed_target_rolls_a_constitution_save() {
        let mut probe = Stream::from_seed(5);
        let a = probe.roll_die(20);
        let (event, _, after) = save_by(&mut saver_with(&[Condition::Paralyzed]), save_on(Ability::Con), 5);
        assert_eq!((event.roll, after), (Some(a), probe.state()));
    }

    #[test]
    fn a_downed_or_stable_pc_fails_a_dexterity_save_automatically() {
        for down in [EndState::Unconscious, EndState::Stable] {
            let mut state = saver_with(&[]);
            state.combatants[0].hp = 0;
            state.combatants[0].end_state = down;
            let (event, _, after) = save_by(&mut state, save_on(Ability::Dex), 5);
            assert_eq!(after, Stream::from_seed(5).state(), "{down:?}");
            assert_eq!(event.message, "saver fails the save automatically (unconscious, Dexterity)");
        }
    }

    #[test]
    fn an_automatically_failed_pc_save_prompts_nothing() {
        let mut state = lit(pc_block("saver", 20), crate::effect::Lighting::Unspecified);
        state.settings.pc_dice = PcDice::Ask;
        state.combatants[0].conditions.push(sourced(Condition::Stunned, None));
        save_by(&mut state, save_on(Ability::Dex), 1);
        assert_eq!(state.player_dice.next_prompt(), None);
    }

    #[test]
    fn an_automatic_failure_applies_the_failed_save_effects() {
        let save = crate::effect::SavingThrow {
            dc: 100,
            damage: vec![gated("1d1", 2, DamageType::Fire, None, false)],
            on_failure: vec![fx(serde_json::json!({"condition": {"condition": "prone", "duration": "permanent"}}))],
            ..dex_save()
        };
        let mut rolled = saver_with(&[]);
        let (event, _, _) = save_by(&mut rolled, save.clone(), 5);
        assert!(event.roll.is_some());
        let mut automatic = saver_with(&[Condition::Paralyzed]);
        let (event, _, _) = save_by(&mut automatic, save, 5);
        assert!(event.roll.is_none());
        for state in [&rolled, &automatic] {
            assert_eq!(state.combatants[0].hp, 47);
            assert!(state.combatants[0].conditions.iter().any(|i| i.condition == Condition::Prone));
        }
    }

    #[test]
    fn a_stunned_creature_fails_its_strength_repeat_save_without_a_die() {
        let mut state = saver_with(&[Condition::Stunned]);
        state.combatants[0].conditions.push(ConditionInstance {
            condition: Condition::Restrained,
            duration: Duration::UntilSave { ability: Ability::Str },
            source: None,
            ends_on_damage: false,
            started_round: 0,
        });
        let before = state.rng.clone();
        let mut events = vec![];
        process_repeat_saves(&mut state, "saver", &mut events).unwrap();
        assert_eq!(state.rng, before);
        assert_eq!(events[0].message, "saver fails the save automatically (stunned, Strength)");
        assert_eq!(events[0].kind, "repeat_save");
        assert_eq!(state.combatants[0].conditions.len(), 2);
    }

    #[test]
    fn a_legacy_save_modifier_gives_no_advantage_as_on_0_1_5() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/engine-0.1.5-state.json");
        let state = crate::store::load(&path, crate::store::VersionCheck::Require).expect("an engine 0.1.5 state loads");
        assert!(state.active_modifiers.iter().any(|m| m.is_legacy() && matches!(m.change, ModifierChange::Advantage)));
        let caster = &state.combatants[index_of(&state, CASTER).unwrap()];
        assert_eq!(save_disposition(&state, caster, Ability::Dex), RollDisposition::Normal);
    }

    #[test]
    fn a_save_note_names_every_source_behind_the_roll_and_nothing_when_they_cancel() {
        let (event, _, _) = save_by(&mut saver_with(&[Condition::Restrained]), save_on(Ability::Dex), 4);
        assert!(event.message.ends_with("the save (Disadvantage: restrained)"), "{}", event.message);

        let mut hasted = saver_with(&[]);
        hasted.active_modifiers.push(borne("saver", "fighter", Some("haste"), ModifierTarget::SavingThrows { ability: Some(Ability::Dex) }, ModifierChange::Advantage));
        let (event, _, _) = save_by(&mut hasted, save_on(Ability::Dex), 4);
        assert!(event.message.ends_with("the save (Advantage: haste)"), "{}", event.message);

        let mut sunlit = saver_with(&[Condition::Restrained]);
        sunlit.stat_blocks[0].stat_block.traits.push(save_trait("Sun Glow", "advantage"));
        sunlit.settings.lighting = crate::effect::Lighting::Sunlight;
        let (event, _, _) = save_by(&mut sunlit, save_on(Ability::Dex), 4);
        assert!(event.message.ends_with("the save"), "{}", event.message);
    }

    #[test]
    fn an_attack_note_is_empty_when_lighting_is_cancelled() {
        let mut state = lit(fixture_block("kobold-warrior"), crate::effect::Lighting::Sunlight);
        state.combatants[1].conditions.push(sourced(Condition::Blinded, None));
        assert_eq!(disposition(&state, &state.combatants[0], &state.combatants[1], &sword()), RollDisposition::Normal);
        let effect = stat_block(&state, &state.combatants[0]).unwrap().actions[0].effects[0].clone();
        let mut events = vec![];
        resolve_effect(&mut state, &mut Stream::from_seed(3), "kobold-warrior", effect, &["fighter".into()], &mut events).unwrap();
        let attack = events.into_iter().find(|e| e.kind == "attack").unwrap().message;
        assert!(!attack.contains("Sunlight"), "{attack}");
    }

    /// A blinker engaged by a fighter and a paladin, with the fighter also engaged by a cleric.
    fn blinker_state(effect: serde_json::Value) -> EncounterState {
        let mut blinker = test_stat_block("blinker", "Blinker", 30, 12);
        blinker.actions.push(ability("blink", vec![fx(effect)], None));
        let blocks = vec![blinker, test_stat_block("fighter", "Fighter", 30, 12), test_stat_block("paladin", "Paladin", 30, 12), test_stat_block("cleric", "Cleric", 30, 12)];
        let mut combatants = vec![
            test_combatant("blinker", "blinker", 30, "enemy"),
            test_combatant("fighter", "fighter", 30, "ally"),
            test_combatant("paladin", "paladin", 30, "ally"),
            test_combatant("cleric", "cleric", 30, "enemy"),
        ];
        for (a, b) in [(0, 1), (0, 2), (1, 3)] {
            let (x, y) = (combatants[a].id.clone(), combatants[b].id.clone());
            combatants[a].engagement.push(y);
            combatants[b].engagement.push(x);
        }
        let mut state = test_encounter(blocks, combatants);
        state.active_turn = Some("blinker".into());
        state
    }

    fn engaged_with(state: &EncounterState, id: &str) -> Vec<String> {
        let mut engaged = state.combatants.iter().find(|c| c.id == id).unwrap().engagement.clone();
        engaged.sort();
        engaged
    }

    const SELF_TELEPORT: &str = r#"{"teleport": {"range": 30, "destination": "unoccupied space you can see"}}"#;

    #[test]
    fn a_self_teleport_frees_the_actor_and_leaves_other_engagements() {
        let mut state = blinker_state(serde_json::from_str(SELF_TELEPORT).unwrap());
        let events = act(&mut state, "blinker", "blink", &["fighter"]).unwrap();
        assert!(engaged_with(&state, "blinker").is_empty());
        assert!(engaged_with(&state, "paladin").is_empty());
        assert_eq!(engaged_with(&state, "fighter"), vec!["cleric"]);
        assert!(events.iter().any(|e| e.kind == "teleport" && e.message == "blinker teleports up to 30 feet"), "{events:?}");
    }

    #[test]
    fn a_self_teleport_with_no_target_moves_the_actor() {
        let mut state = blinker_state(serde_json::from_str(SELF_TELEPORT).unwrap());
        act(&mut state, "blinker", "blink", &[]).unwrap();
        assert!(engaged_with(&state, "blinker").is_empty());
    }

    #[test]
    fn a_self_teleport_in_a_multiattack_moves_the_actor() {
        let mut state = blinker_state(serde_json::json!({"multiattack": {"steps": [{"action": {"action_id": "hop", "count": 1}}]}}));
        let hop = ability("hop", vec![fx(serde_json::from_str(SELF_TELEPORT).unwrap())], None);
        state.stat_blocks.iter_mut().find(|s| s.id == "blinker").unwrap().stat_block.actions.push(hop);
        act(&mut state, "blinker", "blink", &["fighter"]).unwrap();
        assert!(engaged_with(&state, "blinker").is_empty());
        assert!(engaged_with(&state, "paladin").is_empty());
    }

    #[test]
    fn a_target_teleport_under_on_hit_frees_the_hit_target() {
        let mut state = blinker_state(serde_json::json!({"attack": {"modifier": 100, "reach": 5, "target_count": 1,
            "damage": [], "on_hit": [{"teleport": {"range": 30, "destination": "away", "subject": "target"}}]}}));
        act(&mut state, "blinker", "blink", &["fighter"]).unwrap();
        assert!(engaged_with(&state, "fighter").is_empty());
        assert_eq!(engaged_with(&state, "blinker"), vec!["paladin"]);
        assert!(engaged_with(&state, "cleric").is_empty());
    }

    fn ranged_blinker_state(effect: serde_json::Value, teleport_near: bool) -> EncounterState {
        let mut state = blinker_state(effect);
        state.settings.teleport_near = teleport_near;
        for (a, b, band) in [("blinker", "fighter", RangeBand::Engaged), ("blinker", "paladin", RangeBand::Engaged), ("fighter", "cleric", RangeBand::Engaged), ("blinker", "cleric", RangeBand::Far), ("paladin", "cleric", RangeBand::Far)] {
            state.combatants.iter_mut().find(|c| c.id == a).unwrap().ranges.insert(b.into(), band);
            state.combatants.iter_mut().find(|c| c.id == b).unwrap().ranges.insert(a.into(), band);
        }
        state
    }

    fn range(state: &EncounterState, a: &str, b: &str) -> RangeBand {
        state.combatants.iter().find(|c| c.id == a).unwrap().ranges[b]
    }

    #[test]
    fn a_gated_self_teleport_leaves_the_freed_pairs_at_near_and_other_bands_alone() {
        let mut state = ranged_blinker_state(serde_json::from_str(SELF_TELEPORT).unwrap(), true);
        state.combatants.iter_mut().find(|c| c.id == "paladin").unwrap().engagement.clear();
        state.combatants.iter_mut().find(|c| c.id == "blinker").unwrap().engagement.retain(|id| id != "fighter");
        act(&mut state, "blinker", "blink", &[]).unwrap();
        for other in ["fighter", "paladin"] {
            assert_eq!(range(&state, "blinker", other), RangeBand::Near, "{other}");
            assert_eq!(range(&state, other, "blinker"), RangeBand::Near, "{other}");
        }
        assert_eq!(range(&state, "blinker", "cleric"), RangeBand::Far);
        assert_eq!(range(&state, "fighter", "cleric"), RangeBand::Engaged);
        assert_eq!(range(&state, "cleric", "paladin"), RangeBand::Far);
    }

    #[test]
    fn a_gated_target_teleport_leaves_the_target_near_its_freed_pairs() {
        let mut state = ranged_blinker_state(serde_json::json!({"attack": {"modifier": 100, "reach": 5, "target_count": 1,
            "damage": [], "on_hit": [{"teleport": {"range": 30, "destination": "away", "subject": "target"}}]}}), true);
        act(&mut state, "blinker", "blink", &["fighter"]).unwrap();
        for (a, b) in [("fighter", "blinker"), ("blinker", "fighter"), ("fighter", "cleric"), ("cleric", "fighter")] {
            assert_eq!(range(&state, a, b), RangeBand::Near, "{a} to {b}");
        }
        assert_eq!(range(&state, "blinker", "paladin"), RangeBand::Engaged);
    }

    #[test]
    fn an_ungated_teleport_leaves_every_band_as_it_was() {
        let mut state = ranged_blinker_state(serde_json::from_str(SELF_TELEPORT).unwrap(), false);
        let before: Vec<_> = state.combatants.iter().map(|c| c.ranges.clone()).collect();
        act(&mut state, "blinker", "blink", &[]).unwrap();
        assert!(engaged_with(&state, "blinker").is_empty());
        assert_eq!(state.combatants.iter().map(|c| c.ranges.clone()).collect::<Vec<_>>(), before);
    }

    #[test]
    fn a_gated_teleport_adds_no_missing_range_entry() {
        let mut state = blinker_state(serde_json::from_str(SELF_TELEPORT).unwrap());
        state.settings.teleport_near = true;
        act(&mut state, "blinker", "blink", &[]).unwrap();
        assert!(state.combatants.iter().all(|c| c.ranges.is_empty()));
    }

    #[test]
    fn a_top_level_target_teleport_with_no_target_is_refused() {
        let mut state = blinker_state(serde_json::json!({"teleport": {"range": 30, "destination": "away", "subject": "target"}}));
        let err = act(&mut state, "blinker", "blink", &[]).unwrap_err();
        assert!(matches!(&err, EngineError::Illegal(m) if m == "a teleport of its target needs a target"), "{err:?}");
    }

    #[test]
    fn a_snapshot_holding_a_self_teleport_verifies_and_round_trips_without_a_subject() {
        let state = blinker_state(serde_json::from_str(SELF_TELEPORT).unwrap());
        let snapshot = &state.stat_blocks[0];
        let json = serde_json::to_string(snapshot).unwrap();
        assert!(!json.contains("subject"), "{json}");
        let loaded: StatBlockSnapshot = serde_json::from_str(&json).unwrap();
        assert!(loaded.verify());
        assert_eq!(serde_json::to_string(&loaded).unwrap(), json);
    }

    #[test]
    fn a_teleport_subject_defaults_to_self_and_serializes_without_it() {
        let old = r#"{"teleport":{"range":30,"destination":"unoccupied space you can see"}}"#;
        let effect: Effect = serde_json::from_str(old).unwrap();
        assert!(matches!(&effect, Effect::Teleport(t) if t.subject == TeleportSubject::Actor));
        assert_eq!(serde_json::to_string(&effect).unwrap(), old);
        let target: Effect = serde_json::from_str(r#"{"teleport": {"range": 30, "destination": "away", "subject": "target"}}"#).unwrap();
        assert!(serde_json::to_string(&target).unwrap().contains(r#""subject":"target""#));
    }

    fn counterer(id: &str, save_dc: i32) -> StatBlock {
        let mut block = slotted_caster("fireball", Some(3));
        block.id = id.into();
        block.spellcasting.as_mut().unwrap().save_dc = save_dc;
        let mut entry = ability("counterspell", vec![Effect::Cast(cast("counterspell", Some(3)))], None);
        entry.name = "Counterspell".into();
        entry.trigger = Some(ReactionTrigger::SpellCast { caster: SpellCaster::Enemy });
        block.reactions.push(entry);
        block
    }

    fn counter_encounter(counter_dc: i32, gate: bool) -> EncounterState {
        let mut state = test_encounter(
            vec![slotted_caster("fireball", Some(3)), counterer("warlock", counter_dc), test_stat_block("other", "Other", 60, 12)],
            vec![test_combatant("mage", "mage", 30, "enemy"), test_combatant("warlock", "warlock", 30, "ally"), test_combatant("other", "other", 60, "ally")],
        );
        state.settings.spell_cast_windows = gate;
        state
    }

    fn kinds(events: &[EngineEvent]) -> Vec<&str> {
        events.iter().map(|e| e.kind.as_str()).collect()
    }

    #[test]
    fn an_enemys_leveled_cast_holds_on_a_window_before_its_slot() {
        let mut state = answering(&counter_encounter(40, true), vec![]);
        act(&mut state, "mage", "fireball", &["other"]).unwrap();
        let window = state.reactions.waiting.clone().expect("window");
        assert_eq!(window.reactor_id, "warlock");
        assert_eq!(window.options, vec![ReactionOption { id: "counterspell".into(), name: "Counterspell".into() }]);
        assert_eq!(window.trigger, WindowTrigger::SpellCast { caster_id: "mage".into(), spell_id: "fireball".into(), level: 3, target_ids: vec!["other".into()] });
        assert!(!state.combatants[0].resources.contains_key("mage:slot:3"));
    }

    #[test]
    fn a_counter_that_lands_cancels_the_cast_and_keeps_the_slot() {
        let mut state = answering(&counter_encounter(40, true), vec![ReactionAnswer::Take("counterspell".into())]);
        let events = act(&mut state, "mage", "fireball", &["other"]).unwrap();
        assert_eq!(
            kinds(&events).into_iter().filter(|k| *k != "cast").collect::<Vec<_>>(),
            vec!["reaction", "save", "spell_countered"]
        );
        assert!(events.iter().any(|e| e.message == "REACTION warlock: Counterspell (cast by mage)"));
        assert!(events.last().unwrap().message.contains("mage's fireball is countered by warlock (slot not expended)"));
        assert!(!state.combatants[0].resources.contains_key("mage:slot:3"));
        assert!(state.combatants[0].action_economy.action_used);
        assert_eq!(state.combatants[2].hp, 60);
        let warlock = &state.combatants[1];
        assert!(warlock.action_economy.reaction_used);
        assert_eq!(warlock.resources["warlock:slot:3"], 1);
        assert!(state.reactions.waiting.is_none());
    }

    #[test]
    fn a_declined_window_lets_the_cast_resolve() {
        let mut state = answering(&counter_encounter(40, true), vec![ReactionAnswer::Pass]);
        let events = act(&mut state, "mage", "fireball", &["other"]).unwrap();
        assert!(kinds(&events).contains(&"cast"));
        assert!(!kinds(&events).contains(&"spell_countered"));
        assert_eq!(state.combatants[0].resources["mage:slot:3"], 1);
        assert!(!state.combatants[1].action_economy.reaction_used);
    }

    #[test]
    fn a_counter_whose_save_is_passed_leaves_the_cast_live_and_spends_the_reaction() {
        let mut state = answering(&counter_encounter(-100, true), vec![ReactionAnswer::Take("counterspell".into())]);
        let events = act(&mut state, "mage", "fireball", &["other"]).unwrap();
        assert!(!kinds(&events).contains(&"spell_countered"));
        assert_eq!(state.combatants[0].resources["mage:slot:3"], 1);
        assert!(state.combatants[1].action_economy.reaction_used);
    }

    #[test]
    fn the_first_counter_that_lands_ends_the_cast_for_later_reactors() {
        let mut state = counter_encounter(40, true);
        state.combatants.push(test_combatant("warlock2", "warlock2", 30, "ally"));
        state.stat_blocks.push(state.stat_blocks[1].clone());
        state.stat_blocks[3].id = "warlock2".into();
        state.stat_blocks[3].stat_block.id = "warlock2".into();
        let mut state = answering(&state, vec![ReactionAnswer::Take("counterspell".into())]);
        let events = act(&mut state, "mage", "fireball", &["other"]).unwrap();
        assert!(kinds(&events).contains(&"spell_countered"));
        assert!(state.reactions.waiting.is_none());
        assert!(!state.combatants[3].action_economy.reaction_used);
    }

    #[test]
    fn the_gate_cantrips_allies_and_reaction_casts_open_no_window() {
        let mut off = answering(&counter_encounter(40, false), vec![]);
        act(&mut off, "mage", "fireball", &["other"]).unwrap();
        assert!(off.reactions.waiting.is_none());
        assert_eq!(off.combatants[0].resources["mage:slot:3"], 1);

        let mut cantrip_block = slotted_caster("fire-bolt", None);
        cantrip_block.id = "mage".into();
        let mut cantrip = counter_encounter(40, true);
        cantrip.stat_blocks[0].stat_block = cantrip_block;
        let mut cantrip = answering(&cantrip, vec![]);
        act(&mut cantrip, "mage", "fire-bolt", &["other"]).unwrap();
        assert!(cantrip.reactions.waiting.is_none());

        let mut ally = counter_encounter(40, true);
        ally.combatants[0].side_id = "ally".into();
        let mut ally = answering(&ally, vec![]);
        act(&mut ally, "mage", "fireball", &["other"]).unwrap();
        assert!(ally.reactions.waiting.is_none());

        let mut both = counter_encounter(40, true);
        let counter = both.stat_blocks[1].stat_block.reactions[0].clone();
        both.stat_blocks[0].stat_block.reactions.push(counter);
        let mut both = answering(&both, vec![ReactionAnswer::Take("counterspell".into())]);
        act(&mut both, "mage", "fireball", &["other"]).unwrap();
        assert!(both.reactions.waiting.is_none(), "the counter's own cast opens no window");
    }

    #[test]
    fn a_reactor_that_cannot_react_or_has_no_usage_left_gets_no_window() {
        let mut spent = counter_encounter(40, true);
        spent.combatants[1].action_economy.reaction_used = true;
        let mut spent = answering(&spent, vec![]);
        act(&mut spent, "mage", "fireball", &["other"]).unwrap();
        assert!(spent.reactions.waiting.is_none());
        let mut no_slot = counter_encounter(40, true);
        no_slot.combatants[1].resources.insert("warlock:slot:3".into(), 3);
        let mut no_slot = answering(&no_slot, vec![]);
        act(&mut no_slot, "mage", "fireball", &["other"]).unwrap();
        assert!(no_slot.reactions.waiting.is_none());
    }

    #[test]
    fn the_batch_policy_counters_and_logs_its_reason() {
        let mut state = counter_encounter(40, true);
        state.reactions.decider = crate::reaction::ReactionDecider::Policy(crate::tactics::PolicyConstants::default());
        let events = act(&mut state, "mage", "fireball", &["other"]).unwrap();
        assert!(events.iter().any(|e| e.kind == "reaction_reason" && e.message.starts_with("warlock: Counterspell on mage's fireball (expected ")), "{events:?}");
        assert!(kinds(&events).contains(&"spell_countered"));
        assert_eq!(state.combatants[2].hp, 60);
    }

    #[test]
    fn per_day_entries_sharing_a_group_draw_on_one_pool() {
        let shared = || Usage::PerDay { uses: 2, group: Some("magic".into()) };
        let mut block = test_stat_block("mage", "Mage", 30, 12);
        block.reactions.push(ability("ward", vec![], Some(shared())));
        block.reactions.push(ability("counter", vec![], Some(shared())));
        block.reactions.push(ability("alone", vec![], Some(Usage::PerDay { uses: 2, group: None })));
        let mut state = test_encounter(vec![block.clone()], vec![test_combatant("mage", "mage", 30, "enemy")]);
        spend_usage(&mut state, 0, "ward", block.reactions[0].usage.as_ref().unwrap()).unwrap();
        assert!(ability_ready(&state, "mage", &block.reactions[1]));
        spend_usage(&mut state, 0, "counter", block.reactions[1].usage.as_ref().unwrap()).unwrap();
        assert_eq!(state.combatants[0].resources["mage:magic"], 2);
        assert!(!ability_ready(&state, "mage", &block.reactions[0]));
        assert!(!ability_ready(&state, "mage", &block.reactions[1]));
        assert!(spend_usage(&mut state, 0, "ward", block.reactions[0].usage.as_ref().unwrap()).is_err());
        assert!(ability_ready(&state, "mage", &block.reactions[2]), "an entry with no group keeps its own count");
    }

    #[test]
    fn a_per_day_group_is_optional_and_left_out_when_absent() {
        let plain: Usage = serde_json::from_str(r#"{"per_day": {"uses": 3}}"#).unwrap();
        assert!(matches!(&plain, Usage::PerDay { uses: 3, group: None }));
        assert_eq!(serde_json::to_string(&plain).unwrap(), r#"{"per_day":{"uses":3}}"#);
        let grouped: Usage = serde_json::from_str(r#"{"per_day": {"uses": 3, "group": "magic"}}"#).unwrap();
        assert!(matches!(&grouped, Usage::PerDay { group: Some(g), .. } if g == "magic"));
    }

    #[test]
    fn the_spell_cast_trigger_belongs_on_a_reaction_and_counter_spell_binds_the_cast_dc() {
        let on_action = r#"{"id":"x","name":"X","kind":"modeled","trigger":{"spell_cast":{"caster":"enemy"}}}"#;
        let entry: crate::stat_block::AbilityEntry = serde_json::from_str(on_action).unwrap();
        assert_eq!(entry.trigger, Some(ReactionTrigger::SpellCast { caster: SpellCaster::Enemy }));
        let (effects, _) = bound_cast_effects(&counterer("warlock", 19), &cast("counterspell", Some(3))).unwrap();
        assert!(matches!(&effects[0], Effect::CounterSpell(c) if c.dc == 19));
    }

    fn dice(bearer: &str, ability: &str, target: ModifierTarget, change: ModifierChange) -> ActiveModifier {
        borne(bearer, "cleric", Some(ability), target, change)
    }

    fn bless_on(bearer: &str, target: ModifierTarget) -> ActiveModifier {
        dice(bearer, "bless", target, ModifierChange::AddDice("1d4".into()))
    }

    fn all_saves() -> ModifierTarget {
        ModifierTarget::SavingThrows { ability: None }
    }

    /// The face a die note such as ` (+1d4 bless: 3)` names after `label`.
    fn face_after(message: &str, label: &str) -> i32 {
        let at = message.find(label).unwrap_or_else(|| panic!("{label} not in {message}")) + label.len();
        message[at..].split(')').next().unwrap().trim().parse().unwrap()
    }

    fn fighter_swings(state: &mut EncounterState, seed: u64) -> (EngineEvent, RngState) {
        let mut stream = Stream::from_seed(seed);
        let mut events = vec![];
        resolve_effect(state, &mut stream, "fighter", Effect::Attack(sword()), &["goblin".to_string()], &mut events).unwrap();
        (events.into_iter().find(|e| e.kind == "attack").unwrap(), stream.state())
    }

    #[test]
    fn a_die_on_attack_rolls_is_drawn_after_the_d20s_and_added() {
        for seed in 0..8 {
            let mut state = duel(|_, _| {});
            state.active_modifiers.push(bless_on("fighter", ModifierTarget::AttackRolls));
            let (event, after) = fighter_swings(&mut state, seed);
            let mut probe = Stream::from_seed(seed);
            let (d20, _, d4) = (probe.roll_die(20), probe.roll_die(20), probe.roll_die(4));
            assert_eq!(after, probe.state());
            assert_eq!((event.roll, event.total), (Some(d20), Some(d20 as i32 + d4 as i32)));
            assert!(event.message.ends_with(&format!(" (+1d4 bless: {d4})")), "{}", event.message);
        }
    }

    #[test]
    fn an_attack_without_a_die_draws_only_its_d20s() {
        let (event, after) = fighter_swings(&mut duel(|_, _| {}), 3);
        let mut probe = Stream::from_seed(3);
        let d20 = probe.roll_die(20);
        probe.roll_die(20);
        assert_eq!((after, event.total), (probe.state(), Some(d20 as i32)));
        assert!(!event.message.contains("1d4"));
    }

    #[test]
    fn a_natural_one_misses_and_a_natural_twenty_hits_whatever_the_die() {
        for seed in 0..60 {
            let mut state = duel(|_, _| {});
            state.active_modifiers.push(dice("fighter", "boon", ModifierTarget::AttackRolls, ModifierChange::AddDice("10d10".into())));
            state.active_modifiers.push(dice("fighter", "bane", ModifierTarget::AttackRolls, ModifierChange::SubtractDice("1d1".into())));
            let (event, _) = fighter_swings(&mut state, seed);
            match event.roll {
                Some(1) => assert!(event.message.contains("(miss)"), "{}", event.message),
                Some(20) => assert!(event.message.contains("(hit)"), "{}", event.message),
                _ => {}
            }
        }
    }

    #[test]
    fn bless_and_bane_on_one_creature_both_roll_in_modifier_order() {
        let mut state = duel(|_, _| {});
        state.active_modifiers.push(bless_on("fighter", ModifierTarget::AttackRolls));
        state.active_modifiers.push(dice("fighter", "bane", ModifierTarget::AttackRolls, ModifierChange::SubtractDice("1d4".into())));
        let (event, after) = fighter_swings(&mut state, 9);
        let mut probe = Stream::from_seed(9);
        let (d20, _, plus, minus) = (probe.roll_die(20), probe.roll_die(20), probe.roll_die(4), probe.roll_die(4));
        assert_eq!(after, probe.state());
        assert_eq!(event.total, Some(d20 as i32 + plus as i32 - minus as i32));
        assert!(event.message.ends_with(&format!(" (+1d4 bless: {plus}) (-1d4 bane: {minus})")), "{}", event.message);
    }

    #[test]
    fn two_blesses_on_one_creature_give_one_die() {
        let mut state = duel(|_, _| {});
        state.active_modifiers.push(bless_on("fighter", ModifierTarget::AttackRolls));
        state.active_modifiers.push(borne("fighter", "priest", Some("bless"), ModifierTarget::AttackRolls, ModifierChange::AddDice("1d4".into())));
        let (event, after) = fighter_swings(&mut state, 2);
        let mut probe = Stream::from_seed(2);
        (0..3).for_each(|_| { probe.roll_die(20); });
        assert_eq!(after, probe.state(), "two d20s and one d4");
        assert_eq!(event.message.matches("bless:").count(), 1, "{}", event.message);
    }

    #[test]
    fn potency_reads_a_die_as_its_mean_against_flat_bonuses_from_one_source() {
        let cases = [(ModifierChange::Add(3), "add"), (ModifierChange::Add(2), "dice"), (ModifierChange::Add(-3), "add"), (ModifierChange::Add(-2), "dice")];
        for (flat, winner) in cases {
            let mut state = duel(|_, _| {});
            state.active_modifiers.push(bless_on("fighter", all_saves()));
            state.active_modifiers.push(dice("fighter", "bless", all_saves(), flat.clone()));
            let kept: Vec<_> = state.modifiers_on("fighter").collect();
            assert_eq!(kept.len(), 1);
            let dice_won = kept[0].change.die().is_some();
            assert_eq!(dice_won, winner == "dice", "{flat:?} against 1d4");
        }
    }

    #[test]
    fn a_die_on_saves_reaches_every_save_the_saver_rolls() {
        let mut state = saver_with(&[]);
        state.active_modifiers.push(bless_on("saver", all_saves()));
        let (event, _, after) = save_by(&mut state, save_on(Ability::Wis), 6);
        let mut probe = Stream::from_seed(6);
        let (d20, d4) = (probe.roll_die(20), probe.roll_die(4));
        assert_eq!(after, probe.state());
        assert_eq!(event.total, Some(d20 as i32 + d4 as i32));
        assert!(event.message.ends_with(&format!(" (+1d4 bless: {d4})")), "{}", event.message);

        let mut dex_only = saver_with(&[]);
        dex_only.active_modifiers.push(bless_on("saver", ModifierTarget::SavingThrows { ability: Some(Ability::Dex) }));
        let (wis, _, _) = save_by(&mut dex_only, save_on(Ability::Wis), 6);
        assert_eq!(wis.total, Some(d20 as i32), "a Dexterity die stays off a Wisdom save");
    }

    #[test]
    fn an_automatic_save_failure_rolls_no_die() {
        let mut state = saver_with(&[Condition::Paralyzed]);
        state.active_modifiers.push(bless_on("saver", all_saves()));
        let (event, _, after) = save_by(&mut state, save_on(Ability::Dex), 5);
        assert_eq!(after, Stream::from_seed(5).state());
        assert!(!event.message.contains("1d4"), "{}", event.message);
    }

    #[test]
    fn a_die_reaches_concentration_and_repeat_saves() {
        let mut state = saver_with(&[]);
        state.combatants[0].concentration = Some("bless".into());
        state.active_modifiers.push(bless_on("saver", all_saves()));
        let mut stream = Stream::from_seed(4);
        let mut events = vec![];
        resolve_effect(&mut state, &mut stream, "fighter", fx(serde_json::json!({"damage": {"dice": "1d1", "modifier": 1, "type": "fire"}})), &["saver".to_string()], &mut events).unwrap();
        let save = events.iter().find(|e| e.kind == "concentration_save").expect("a concentration save");
        assert_eq!(save.total, Some(save.roll.unwrap() as i32 + face_after(&save.message, "(+1d4 bless:")), "{}", save.message);

        let mut state = saver_with(&[]);
        state.active_modifiers.push(bless_on("saver", all_saves()));
        state.combatants[0].conditions.push(ConditionInstance { condition: Condition::Restrained, duration: Duration::UntilSave { ability: Ability::Str }, source: None, ends_on_damage: false, started_round: 0 });
        let mut events = vec![];
        process_repeat_saves(&mut state, "saver", &mut events).unwrap();
        let save = &events[0];
        assert_eq!(save.total, Some(save.roll.unwrap() as i32 + face_after(&save.message, "(+1d4 bless:")), "{}", save.message);
    }

    fn downed_pc(modifier: Option<ActiveModifier>) -> (EngineEvent, RngState) {
        let mut state = brute_vs_pc(20, smash(30, "1d6", 0), |pc| {
            pc.hp = 0;
            pc.end_state = EndState::Unconscious;
        });
        state.active_modifiers.extend(modifier);
        let mut events = vec![];
        roll_death_saves(&mut state, "pc", &mut events).unwrap();
        (events.into_iter().find(|e| e.kind == "death_save").unwrap(), state.rng)
    }

    #[test]
    fn a_no_ability_save_die_reaches_death_saves_but_a_flat_bonus_does_not() {
        let (plain, plain_rng) = downed_pc(None);
        let (flat, flat_rng) = downed_pc(Some(dice("pc", "aura", all_saves(), ModifierChange::Add(5))));
        assert_eq!((flat.message.clone(), flat.total, flat_rng), (plain.message.clone(), None, plain_rng.clone()));
        let (blessed, blessed_rng) = downed_pc(Some(bless_on("pc", all_saves())));
        assert_eq!(blessed.roll, plain.roll);
        assert_ne!(blessed_rng, plain_rng, "the die draws after the d20");
        let roll = blessed.roll.unwrap();
        if roll != 1 && roll != 20 {
            let total = roll as i32 + face_after(&blessed.message, "(+1d4 bless:");
            assert_eq!(blessed.total, Some(total));
            assert_eq!(blessed.message.contains("succeeds"), total >= 10, "{}", blessed.message);
        }
        let (con_only, _) = downed_pc(Some(bless_on("pc", ModifierTarget::SavingThrows { ability: Some(Ability::Con) })));
        assert_eq!(con_only.message, plain.message, "a die tied to one ability stays off death saves");
    }

    #[test]
    fn an_ask_mode_pc_is_prompted_for_its_die_and_the_attack_is_marked() {
        let mut state = duel(|blocks, _| blocks[0].class = Some("fighter".into()));
        state.settings.pc_dice = PcDice::Ask;
        state.active_modifiers.push(bless_on("fighter", ModifierTarget::AttackRolls));
        state.player_dice.faces = [15].into_iter().collect();
        fighter_swings(&mut state, 1);
        assert_eq!(state.player_dice.next_prompt(), Some((4, "fighter".into(), 1)), "the d4 is asked for after the d20");

        let mut state = duel(|blocks, _| blocks[0].class = Some("fighter".into()));
        state.settings.pc_dice = PcDice::Ask;
        state.active_modifiers.push(bless_on("fighter", ModifierTarget::AttackRolls));
        state.player_dice.faces = [15, 3].into_iter().collect();
        let (event, _) = fighter_swings(&mut state, 1);
        assert_eq!(event.total, Some(18));
        assert!(event.player_dice && event.message.ends_with(" (+1d4 bless: 3)"), "{event:?}");
    }

    fn bane_state(slot: u8, passer: Option<&str>) -> EncounterState {
        let mut blocks = vec![sure_caster("mage", "bane")];
        blocks[0].actions[0].effects = vec![Effect::Cast(cast("bane", Some(slot)))];
        let mut combatants = vec![test_combatant("mage", "mage", 30, "ally")];
        for id in ["a", "b", "c", "d"] {
            let mut block = test_stat_block(id, id, 40, 12);
            if passer == Some(id) {
                block.saving_throws.insert(Ability::Cha, 100);
            }
            blocks.push(block);
            combatants.push(test_combatant(id, id, 40, "enemy"));
        }
        test_encounter(blocks, combatants)
    }

    fn dice_bearers(state: &EncounterState, ability: &str) -> Vec<String> {
        let mut ids: Vec<String> = state.active_modifiers.iter().filter(|m| m.source_ability.as_deref() == Some(ability)).filter_map(|m| m.bearer().map(str::to_string)).collect();
        ids.sort();
        ids.dedup();
        ids
    }

    #[test]
    fn bane_lands_only_on_targets_that_fail_its_charisma_save() {
        let mut state = bane_state(1, Some("b"));
        act(&mut state, "mage", "bane", &["a", "b", "c"]).unwrap();
        assert_eq!(dice_bearers(&state, "bane"), ["a", "c"]);
        assert!(state.active_modifiers.iter().all(|m| matches!(&m.change, ModifierChange::SubtractDice(d) if d == "1d4")));
        assert_eq!(state.active_modifiers.len(), 4, "attack rolls and saves on each");
    }

    #[test]
    fn an_upcast_bane_or_bless_takes_a_fourth_target() {
        let mut state = bane_state(2, None);
        act(&mut state, "mage", "bane", &["a", "b", "c", "d"]).unwrap();
        assert_eq!(dice_bearers(&state, "bane"), ["a", "b", "c", "d"]);

        let mut blocks = vec![sure_caster("mage", "bless")];
        blocks[0].actions[0].effects = vec![Effect::Cast(cast("bless", Some(2)))];
        let mut combatants = vec![test_combatant("mage", "mage", 30, "ally")];
        for id in ["a", "b", "c", "d"] {
            blocks.push(test_stat_block(id, id, 40, 12));
            combatants.push(test_combatant(id, id, 40, "ally"));
        }
        let mut state = test_encounter(blocks, combatants);
        act(&mut state, "mage", "bless", &["a", "b", "c", "d"]).unwrap();
        assert_eq!(dice_bearers(&state, "bless"), ["a", "b", "c", "d"]);
        assert!(state.active_modifiers.iter().all(|m| matches!(&m.change, ModifierChange::AddDice(d) if d == "1d4")));
    }

    #[test]
    fn a_die_on_anything_but_attacks_or_saves_is_refused_at_load() {
        let entry = |target: serde_json::Value, change: serde_json::Value| {
            serde_json::json!({"id": "ward", "name": "Ward", "kind": "modeled", "effects": [{"timed_modifier": {"target": target, "change": change, "duration": "permanent"}}]})
        };
        let mut block = serde_json::to_value(test_stat_block("x", "X", 10, 10)).unwrap();
        block["actions"] = serde_json::json!([entry(serde_json::json!({"armor_class": null}), serde_json::json!({"add_dice": "1d4"}))]);
        let err = serde_json::from_value::<StatBlock>(block.clone()).unwrap_err().to_string();
        assert!(err.contains("ward: add_dice 1d4 can only change attack_rolls or saving_throws, not armor_class"), "{err}");
        block["actions"] = serde_json::json!([entry(serde_json::json!({"attack_rolls": null}), serde_json::json!({"subtract_dice": "d4"}))]);
        let err = serde_json::from_value::<StatBlock>(block.clone()).unwrap_err().to_string();
        assert!(err.contains("d4 is not an NdM dice expression"), "{err}");
        block["actions"] = serde_json::json!([entry(serde_json::json!({"saving_throws": {"ability": "dex"}}), serde_json::json!({"subtract_dice": "1d4"}))]);
        assert!(serde_json::from_value::<StatBlock>(block).is_ok());

        let mut state = serde_json::to_value(duel(|_, _| {})).unwrap();
        state["active_modifiers"] = serde_json::to_value([dice("fighter", "aura", ModifierTarget::Speed, ModifierChange::AddDice("1d4".into()))]).unwrap();
        let err = serde_json::from_value::<EncounterState>(state).unwrap_err().to_string();
        assert!(err.contains("not speed"), "{err}");
    }

    fn long_state(long_durations: bool) -> EncounterState {
        let mut state = test_encounter(
            vec![test_stat_block("fighter", "Fighter", 20, 16), test_stat_block("goblin", "Goblin", 7, 15)],
            vec![test_combatant("fighter", "fighter", 20, "ally"), test_combatant("goblin", "goblin", 7, "enemy")],
        );
        state.settings.long_durations = long_durations;
        state
    }

    fn long_condition(state: &mut EncounterState, duration: Duration) {
        state.combatants[1].conditions.push(ConditionInstance { condition: Condition::Blinded, duration, source: None, ends_on_damage: false, started_round: state.round });
    }

    fn long_modifier(state: &mut EncounterState, duration: Duration) {
        state.active_modifiers.push(ActiveModifier { target: ModifierTarget::ArmorClass, change: ModifierChange::Add(1), remaining: duration, source: Some("fighter".into()), bearer: Some("goblin".into()), source_ability: None, ends_on_damage: false, on_end: vec![], started_round: state.round });
    }

    #[test]
    fn with_long_durations_an_hours_modifier_and_condition_survive_many_turns() {
        let mut state = long_state(true);
        long_condition(&mut state, Duration::Hours(1));
        long_modifier(&mut state, Duration::Hours(1));
        for _ in 0..20 {
            end_turn(&mut state);
        }
        assert_eq!(state.combatants[1].conditions.len(), 1);
        assert_eq!(state.active_modifiers.len(), 1);
        assert!(matches!(state.active_modifiers[0].remaining, Duration::Hours(1)));
    }

    #[test]
    fn with_long_durations_a_minute_clears_after_ten_rounds_and_not_before() {
        let mut state = long_state(true);
        long_condition(&mut state, Duration::Minutes(1));
        long_modifier(&mut state, Duration::Minutes(1));
        for _ in 0..19 {
            end_turn(&mut state);
        }
        assert_eq!(state.round, 10);
        assert_eq!(state.combatants[1].conditions.len(), 1);
        assert_eq!(state.active_modifiers.len(), 1);
        end_turn(&mut state);
        assert_eq!(state.round, 11);
        assert!(state.combatants[1].conditions.is_empty());
        assert!(state.active_modifiers.is_empty());
    }

    #[test]
    fn a_long_duration_counts_from_the_round_it_was_applied() {
        let mut state = long_state(true);
        for _ in 0..6 {
            end_turn(&mut state);
        }
        assert_eq!(state.round, 4);
        long_condition(&mut state, Duration::Minutes(1));
        for _ in 0..19 {
            end_turn(&mut state);
        }
        assert_eq!(state.round, 13);
        assert_eq!(state.combatants[1].conditions.len(), 1);
        end_turn(&mut state);
        assert!(state.combatants[1].conditions.is_empty());
    }

    #[test]
    fn without_long_durations_minutes_hours_and_days_decrement_each_turn_as_before() {
        let mut state = long_state(false);
        long_condition(&mut state, Duration::Minutes(1));
        long_condition(&mut state, Duration::Hours(2));
        long_modifier(&mut state, Duration::Days(2));
        end_turn(&mut state);
        assert_eq!(state.combatants[1].conditions.len(), 1);
        assert!(matches!(state.combatants[1].conditions[0].duration, Duration::Hours(1)));
        assert!(matches!(state.active_modifiers[0].remaining, Duration::Days(1)));
        end_turn(&mut state);
        assert!(state.combatants[1].conditions.is_empty());
        assert!(state.active_modifiers.is_empty());
    }

    #[test]
    fn applying_a_condition_or_modifier_records_the_round_and_a_zero_stays_out_of_the_save() {
        let mut state = long_state(true);
        let mut block = test_stat_block("fighter", "Fighter", 20, 16);
        block.actions.push(ability("blind", vec![fx(serde_json::json!({"condition": {"condition": "blinded", "duration": {"minutes": 1}}}))], None));
        block.actions.push(ability("ward", vec![fx(serde_json::json!({"timed_modifier": {"target": "armor_class", "change": {"add": 1}, "duration": {"hours": 1}, "bearer": "self"}}))], None));
        state.stat_blocks[0] = StatBlockSnapshot::from_stat_block("fighter".into(), block);
        for _ in 0..4 {
            end_turn(&mut state);
        }
        assert_eq!(state.round, 3);
        act(&mut state, "fighter", "blind", &["goblin"]).unwrap();
        state.combatants[0].action_economy = ActionEconomy::default();
        act(&mut state, "fighter", "ward", &[]).unwrap();
        assert_eq!(state.combatants[1].conditions[0].started_round, 3);
        assert_eq!(state.active_modifiers[0].started_round, 3);
        let zero = ConditionInstance { condition: Condition::Blinded, duration: Duration::Permanent, source: None, ends_on_damage: false, started_round: 0 };
        assert!(!serde_json::to_string(&zero).unwrap().contains("started_round"));
        let old: ConditionInstance = serde_json::from_str(r#"{"condition":"blinded","duration":{"minutes":1}}"#).unwrap();
        assert_eq!(old.started_round, 0);
        let settings = serde_json::to_string(&long_state(false).settings).unwrap();
        assert!(!settings.contains("long_durations"));
    }
}
