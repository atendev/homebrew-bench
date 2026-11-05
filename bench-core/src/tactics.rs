//! Deterministic batch tactics policy (ADR 0012).
//!
//! One policy plays both sides of a batch fight. It enumerates legal
//! move-plus-action plans for the acting combatant, scores each by expected
//! damage, control and survival, and returns the best one with a log line
//! that states its reason, target, resource use, movement and area
//! assumptions. Scoring walks stat-block and combatant order only, so the
//! same state always yields the same plan.

use serde::{Deserialize, Serialize};

use crate::effect::{Area, AttackFilter, Duration, Effect, ExtraActionUse, ModifierBearer, ModifierChange, ModifierTarget, MultiattackStep, ReactionTrigger, RollDisposition, TeleportSubject, TimedModifier, Usage};
use crate::encounter::{Combatant, EncounterState, EndState, RangeBand};
use crate::engine::{self, can_act, Within};
use crate::reaction::{ReactionAnswer, ReactionWindow, WindowTrigger};
use crate::stat_block::{AbilityEntry, Condition, StatBlock};

/// Stable policy identifier written into logs, reports and fingerprints.
pub const POLICY_ID: &str = "homebrew-bench.tactics";
/// Policy version. Bump it whenever scoring changes, so baselines go stale.
pub const POLICY_VERSION: u32 = 3;

/// Value of one AC point against one round of incoming attack-roll damage. A
/// d20 lands in an n-point band 5n% of the time, near 0.1n of the expected
/// damage (ADR 0012). The reaction reserve prices Shield and Parry with it.
pub const AC_POINT_VALUE: f64 = 0.1;

/// Rounds a timed modifier is valued over at most (ADR 0012). A concentration or longer duration
/// counts as the whole horizon, cut to the rounds the fight is expected to last and, for
/// concentration, weighted round by round by the chance the caster keeps it.
pub const BUFF_HORIZON: u32 = 2;

/// Share of a bearer's incoming damage that Advantage on saving throws is worth per round
/// (ADR 0012).
pub const SAVE_ADVANTAGE_VALUE: f64 = 0.3;

/// Score weights. Resource cost is 0.0 because a batch fight is one encounter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Weights {
    pub expected_damage: f64,
    pub control: f64,
    pub survival: f64,
    pub resource_cost: f64,
}

/// Assumed creatures caught by an area of each size class.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AreaCounts {
    pub single: u32,
    pub small: u32,
    pub medium: u32,
    pub large: u32,
}

/// Every policy input a standing assumption may override.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PolicyConstants {
    pub weights: Weights,
    /// Living allies are healed only below this fraction of their maximum HP.
    pub heal_threshold: f64,
    /// Non-healers use consumables below this fraction of their maximum HP.
    pub consumable_threshold: f64,
    pub area_counts: AreaCounts,
    /// Downed opponents are legal targets.
    pub target_downed: bool,
}

impl Default for PolicyConstants {
    fn default() -> Self {
        Self {
            weights: Weights { expected_damage: 1.0, control: 1.0, survival: 1.0, resource_cost: 0.0 },
            heal_threshold: 0.5,
            consumable_threshold: 0.25,
            area_counts: AreaCounts { single: 1, small: 2, medium: 3, large: 4 },
            target_downed: false,
        }
    }
}

impl PolicyConstants {
    /// Apply one standing assumption. Returns `Ok(false)` when the path is not
    /// under `policy.`, so callers can route it elsewhere.
    pub fn apply(&mut self, path: &str, value: &serde_json::Value) -> Result<bool, String> {
        let Some(rest) = path.strip_prefix("policy.") else { return Ok(false) };
        let number = || value.as_f64().ok_or_else(|| format!("{path} needs a number"));
        let count = || {
            value.as_u64().filter(|n| *n >= 1).map(|n| n as u32).ok_or_else(|| format!("{path} needs a whole number of at least 1"))
        };
        match rest {
            "weights.expected_damage" => self.weights.expected_damage = number()?,
            "weights.control" => self.weights.control = number()?,
            "weights.survival" => self.weights.survival = number()?,
            "weights.resource_cost" => self.weights.resource_cost = number()?,
            "heal_threshold" => self.heal_threshold = number()?,
            "consumable_threshold" => self.consumable_threshold = number()?,
            "area_counts.single" => self.area_counts.single = count()?,
            "area_counts.small" => self.area_counts.small = count()?,
            "area_counts.medium" => self.area_counts.medium = count()?,
            "area_counts.large" => self.area_counts.large = count()?,
            "target_downed" => self.target_downed = value.as_bool().ok_or_else(|| format!("{path} needs true or false"))?,
            _ => return Err(format!("{path} is not a policy constant")),
        }
        Ok(true)
    }
}

/// Hash of the policy id, version and constants, for baseline fingerprints.
pub fn fingerprint(constants: &PolicyConstants) -> String {
    let value = serde_json::json!({ "id": POLICY_ID, "version": POLICY_VERSION, "constants": constants });
    crate::encounter::tamper_hash(&value)
}

/// Which part of the turn a plan spends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Section {
    Action,
    Bonus,
    Legendary,
    /// The extra action a modifier grants, such as Haste's; only actions the engine allows for it.
    Extra,
}

/// One band change relative to one combatant, priced by the step engine.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Movement {
    pub target_id: String,
    pub band: RangeBand,
    pub feet: u32,
}

/// The chosen move-plus-action plan and why.
#[derive(Debug, Clone, Serialize)]
pub struct Plan {
    pub actor_id: String,
    pub section: Section,
    pub action_id: String,
    pub targets: Vec<String>,
    pub movement: Vec<Movement>,
    /// Multiattack picks in resolution order, handed to the engine.
    pub picks: Vec<Vec<usize>>,
    pub expected_damage: f64,
    pub control: f64,
    pub survival: f64,
    pub score: f64,
    pub reason: String,
    pub resource: Option<String>,
    pub area: Option<String>,
}

impl Plan {
    /// The decision line written into batch logs.
    pub fn log_line(&self) -> String {
        let targets = if self.targets.is_empty() { "none".to_string() } else { self.targets.join(", ") };
        let movement = if self.movement.is_empty() {
            "none".to_string()
        } else {
            self.movement
                .iter()
                .map(|m| format!("{} to {} ({} ft)", band_word(m.band), m.target_id, m.feet))
                .collect::<Vec<_>>()
                .join(", ")
        };
        format!(
            "DECISION {} {}: {} -> {} | reason: {} | resource: {} | move: {} | area: {} | score {:.2}",
            self.actor_id,
            section_word(self.section),
            self.action_id,
            targets,
            self.reason,
            self.resource.as_deref().unwrap_or("none"),
            movement,
            self.area.as_deref().unwrap_or("none"),
            self.score,
        )
    }
}

/// Fallback when nothing is legal: an approach step toward the top-priority
/// opponent when one is affordable, otherwise a logged no-op.
#[derive(Debug, Clone, Serialize)]
pub struct Fallback {
    pub actor_id: String,
    pub movement: Vec<Movement>,
    pub reason: String,
}

impl Fallback {
    pub fn log_line(&self) -> String {
        let movement = self
            .movement
            .iter()
            .map(|m| format!("{} to {} ({} ft)", band_word(m.band), m.target_id, m.feet))
            .collect::<Vec<_>>();
        let movement = if movement.is_empty() { "none".to_string() } else { movement.join(", ") };
        format!("DECISION {} action: no legal plan, takes the defensive no-op | reason: {} | move: {movement}", self.actor_id, self.reason)
    }
}

/// Best plan for `actor_id` in `section`, or `None` when nothing legal scores above zero.
pub fn choose(state: &EncounterState, actor_id: &str, section: Section, constants: &PolicyConstants) -> Option<Plan> {
    let ctx = Ctx { state, constants, threat_nesting: std::cell::Cell::new(0) };
    let actor = ctx.combatant(actor_id)?;
    if !can_act(actor) {
        return None;
    }
    let block = ctx.block(actor)?;
    let extra_uses = match section {
        Section::Extra => Some(engine::extra_action_uses(state, actor_id)?),
        _ => None,
    };
    let abilities: &[AbilityEntry] = match section {
        Section::Action | Section::Extra => &block.actions,
        Section::Bonus => &block.bonus_actions,
        Section::Legendary => &block.legendary_actions,
    };
    let mut candidates: Vec<Plan> = Vec::new();
    for ability in abilities {
        if ability.effects.is_empty() || !engine::ability_ready(state, actor_id, ability) {
            continue;
        }
        if extra_uses.as_ref().is_some_and(|uses| !engine::extra_action_allows(uses, ability)) {
            continue;
        }
        if is_consumable(block, ability) && !ctx.consumable_allowed(actor, block) {
            continue;
        }
        if let Some(plan) = ctx.plan_for(actor, block, ability, section) {
            candidates.push(plan);
        }
    }
    pick_best(block, abilities, candidates)
}

/// Defensive fallback for a turn where [`choose`] found no action plan.
pub fn fallback(state: &EncounterState, actor_id: &str, constants: &PolicyConstants) -> Fallback {
    let ctx = Ctx { state, constants, threat_nesting: std::cell::Cell::new(0) };
    let mut out = Fallback { actor_id: actor_id.into(), movement: vec![], reason: "no ability is ready and in reach".into() };
    let Some(actor) = ctx.combatant(actor_id) else { return out };
    let Some(target) = ctx.ranked_opponents(actor, None).into_iter().next() else {
        out.reason = "no opponent is left".into();
        return out;
    };
    let band = ctx.band(actor, target);
    let left = ctx.movement_left(actor);
    let step = match band {
        RangeBand::Far if left >= 30 => Some(RangeBand::Near),
        RangeBand::Near if left >= 5 => Some(RangeBand::Engaged),
        _ => None,
    };
    if let Some(next) = step {
        out.movement.push(Movement { target_id: target.id.clone(), band: next, feet: left });
        out.reason = format!("closes on {} to act next turn", target.id);
    }
    out
}

/// Decide one reaction window for batch, on the state as the engine presents
/// it. Returns the answer and the reason line the engine logs. A hit reaction
/// is taken only when it turns the hit into a miss; an Opportunity Attack only
/// when its value beats the reactor's reaction reserve.
pub fn decide_reaction(state: &EncounterState, window: &ReactionWindow, constants: &PolicyConstants) -> (ReactionAnswer, String) {
    let ctx = Ctx { state, constants, threat_nesting: std::cell::Cell::new(0) };
    match &window.trigger {
        WindowTrigger::Hit { total, ac, critical, .. } => ctx.decide_hit(window, *total, *ac, *critical),
        WindowTrigger::Opportunity { mover_id } => ctx.decide_opportunity(window, mover_id),
    }
}

/// Highest score wins; ties keep stat-block order. A limited-use plan must beat
/// the best at-will plan, and healing a living ally must beat the best attack
/// or control plan. Revival is exempt and competes on score alone.
fn pick_best(block: &StatBlock, abilities: &[AbilityEntry], candidates: Vec<Plan>) -> Option<Plan> {
    let limited = |id: &str| abilities.iter().find(|a| a.id == id).is_some_and(|a| is_limited(block, a));
    let heal = |p: &Plan| p.reason.starts_with("heals");
    let best_at_will = candidates.iter().filter(|p| !limited(&p.action_id)).map(|p| p.score).fold(f64::NEG_INFINITY, f64::max);
    let best_offense = candidates.iter().filter(|p| !heal(p) && !p.reason.starts_with("revives")).map(|p| p.score).fold(0.0, f64::max);
    let mut best: Option<Plan> = None;
    for plan in candidates {
        let barred = plan.score <= 0.0
            || (limited(&plan.action_id) && plan.score <= best_at_will)
            || (heal(&plan) && plan.score <= best_offense);
        if !barred && best.as_ref().map_or(true, |b| plan.score > b.score + 1e-9) {
            best = Some(plan);
        }
    }
    best
}

struct Ctx<'a> {
    state: &'a EncounterState,
    constants: &'a PolicyConstants,
    /// How many `threat` calls are open. A control effect values itself by the
    /// target's threat, so two combatants with control effects would recurse forever.
    threat_nesting: std::cell::Cell<u8>,
}

/// Open nesting levels past which a control effect, an Opportunity Attack, the
/// reaction reserve and the step-out penalty are valued at zero.
const MAX_THREAT_NESTING: u8 = 2;

/// What a combatant's best ready action puts out: damage alone, and damage plus healing.
#[derive(Debug, Clone, Copy, Default)]
struct ThreatParts {
    damage: f64,
    total: f64,
}

#[derive(Debug, Clone, Copy, Default)]
struct Value {
    damage: f64,
    control: f64,
    heal: f64,
    /// Opportunity Attacks a teleport avoids.
    survival: f64,
}

impl Value {
    fn add(&mut self, other: Value) {
        self.damage += other.damage;
        self.control += other.control;
        self.heal += other.heal;
        self.survival += other.survival;
    }

    fn scaled(self, factor: f64) -> Value {
        Value { damage: self.damage * factor, control: self.control * factor, heal: self.heal * factor, survival: self.survival * factor }
    }

    /// What a Multiattack choice ranks its options by, and what a step-out must beat.
    fn worth(&self) -> f64 {
        self.damage + self.control + self.survival
    }
}

/// Where a plan leaves the actor when it acts: attack dispositions read who is within 5 feet of
/// the actor from it (the target's Prone and the engine's close-combat rule), and a teleport reads
/// which engagements it escapes.
#[derive(Debug, Clone, Copy, Default)]
struct Footing {
    moves: Moves,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Moves {
    /// Engagement as the state holds it.
    #[default]
    Stays,
    /// The plan moves the actor to engage the target first.
    In,
    /// The plan steps the actor out of every engagement first.
    Out,
}

impl Footing {
    const STEPPED_OUT: Footing = Footing { moves: Moves::Out };
}

/// Which rolls an incoming-damage sum counts: attack rolls through a filter, saving throws
/// of one ability (any ability when `None`), or saving throws inside areas only.
#[derive(Debug, Clone, Copy)]
enum Rolls {
    Attacks(AttackFilter),
    Saves(Option<crate::stat_block::Ability>),
    AreaSaves,
}

#[derive(Debug, Clone, Default)]
struct BearerValue {
    damage: f64,
    survival: f64,
    /// Per-round value of an extra action, already counted in `damage`.
    extra_action: f64,
    /// The weighted rounds, horizon and keep chance of the first modifier whose rounds were cut
    /// short or weighted.
    horizon: Option<(f64, f64, f64)>,
    /// Lethargy cost, already taken off `damage`.
    lethargy: f64,
    notes: Vec<String>,
}

/// What a buff's value reads from its caster alone, computed at most once per plan however many
/// bearers it is weighed on: the rounds the fight has left and the caster's keep chance.
struct BuffOutlook<'c> {
    actor: &'c Combatant,
    rounds_left: std::cell::OnceCell<f64>,
    keep: std::cell::OnceCell<f64>,
}

impl<'c> BuffOutlook<'c> {
    fn new(actor: &'c Combatant) -> Self {
        BuffOutlook { actor, rounds_left: std::cell::OnceCell::new(), keep: std::cell::OnceCell::new() }
    }
}

impl BearerValue {
    fn add(&mut self, other: &BearerValue) {
        self.damage += other.damage;
        self.survival += other.survival;
        self.extra_action += other.extra_action;
        self.horizon = self.horizon.or(other.horizon);
        self.lethargy += other.lethargy;
        self.notes.extend(other.notes.iter().cloned());
    }
}

#[derive(Debug, Clone, PartialEq)]
struct OpportunityPick {
    attack_id: String,
    value: f64,
    drops: bool,
}

struct OpportunityDecision {
    pick: Option<OpportunityPick>,
    reserve: f64,
}

impl OpportunityDecision {
    fn taken(&self) -> Option<f64> {
        self.pick.as_ref().map(|p| p.value).filter(|v| *v > self.reserve)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AreaClass {
    Small,
    Medium,
    Large,
}

impl<'a> Ctx<'a> {
    fn combatant(&self, id: &str) -> Option<&'a Combatant> {
        self.state.combatants.iter().find(|c| c.id == id)
    }

    fn block(&self, c: &Combatant) -> Option<&'a StatBlock> {
        self.state.stat_blocks.iter().find(|s| s.id == c.stat_block_id).map(|s| &s.stat_block)
    }

    fn opponents(&self, actor: &Combatant) -> impl Iterator<Item = &'a Combatant> + '_ {
        let side = actor.side_id.clone();
        self.state.combatants.iter().filter(move |c| c.side_id != side)
    }

    fn allies(&self, actor: &Combatant) -> impl Iterator<Item = &'a Combatant> + '_ {
        let side = actor.side_id.clone();
        self.state.combatants.iter().filter(move |c| c.side_id == side)
    }

    fn band(&self, a: &Combatant, b: &Combatant) -> RangeBand {
        if a.engagement.contains(&b.id) || b.engagement.contains(&a.id) {
            return RangeBand::Engaged;
        }
        match a.ranges.get(&b.id).or_else(|| b.ranges.get(&a.id)) {
            Some(RangeBand::Engaged) | None => RangeBand::Near,
            Some(band) => *band,
        }
    }

    fn movement_left(&self, c: &Combatant) -> u32 {
        engine::walk_speed(self.state, c).saturating_sub(c.action_economy.movement_used_feet)
    }

    /// Opponents in target-priority order: next-cycle threat, then the given
    /// immediate expected damage, then lowest HP fraction, then id.
    fn ranked_opponents(&self, actor: &Combatant, immediate: Option<&dyn Fn(&Combatant) -> f64>) -> Vec<&'a Combatant> {
        let mut rows: Vec<(f64, f64, f64, &Combatant)> = self
            .opponents(actor)
            .filter(|c| c.end_state == EndState::Active || (self.constants.target_downed && c.end_state != EndState::Dead))
            .map(|c| (self.threat(c), immediate.map_or(0.0, |f| f(c)), hp_fraction(c), c))
            .collect();
        rows.sort_by(|a, b| {
            b.0.total_cmp(&a.0)
                .then(b.1.total_cmp(&a.1))
                .then(a.2.total_cmp(&b.2))
                .then(a.3.id.cmp(&b.3.id))
        });
        rows.into_iter().map(|r| r.3).collect()
    }

    /// Expected damage plus healing the combatant can put out on its next
    /// action, against its living opponents on average.
    fn threat(&self, c: &Combatant) -> f64 {
        self.threat_parts(c).total
    }

    /// `threat` without healing, each foe's share capped at its hit points: what the combatant
    /// takes off its opponents on its next action.
    fn damage_threat(&self, c: &Combatant) -> f64 {
        self.threat_parts(c).damage
    }

    fn threat_parts(&self, c: &Combatant) -> ThreatParts {
        let open = self.threat_nesting.get();
        self.threat_nesting.set(open + 1);
        let value = self.threat_unnested(c);
        self.threat_nesting.set(open);
        value
    }

    fn threat_unnested(&self, c: &Combatant) -> ThreatParts {
        let mut best = ThreatParts::default();
        if !can_act(c) {
            return best;
        }
        let Some(block) = self.block(c) else { return best };
        let foes: Vec<&Combatant> = self.opponents(c).filter(|o| o.end_state == EndState::Active).collect();
        if foes.is_empty() {
            return best;
        }
        let n = foes.len() as f64;
        for a in block.actions.iter().filter(|a| !a.effects.is_empty() && engine::ability_ready(self.state, &c.id, a)) {
            let mut parts = ThreatParts::default();
            for f in &foes {
                let v = self.ability_value(c, block, a, f, 0, Footing::default()).0;
                parts.damage += v.damage.min(hit_points(f)) / n;
                parts.total += (v.damage + v.heal) / n;
            }
            best.damage = best.damage.max(parts.damage);
            best.total = best.total.max(parts.total);
        }
        best
    }

    /// Rounds the fight has left: the actor's active opponents' hit points over the damage its
    /// side's able combatants deal per round, or [`BUFF_HORIZON`] when the side deals none.
    fn rounds_left(&self, actor: &Combatant) -> f64 {
        let hp: f64 = self.opponents(actor).filter(|o| o.end_state == EndState::Active).map(hit_points).sum();
        let damage: f64 = self.allies(actor).filter(|a| can_act(a)).map(|a| self.damage_threat(a)).sum();
        if damage > 0.0 {
            hp / damage
        } else {
            f64::from(BUFF_HORIZON)
        }
    }

    /// Whether `bearer`'s next turn, walking the initiative order on from the actor's, comes
    /// before every able opponent's. Never for the actor itself or an actor outside the order.
    fn acts_before_opponents(&self, actor: &Combatant, bearer: &Combatant) -> bool {
        let order = &self.state.initiative_order;
        let Some(at) = order.iter().position(|id| *id == actor.id) else { return false };
        if bearer.id == actor.id {
            return false;
        }
        for id in order.iter().cycle().skip(at + 1).take(order.len() - 1) {
            if *id == bearer.id {
                return true;
            }
            if self.combatant(id).is_some_and(|c| c.side_id != actor.side_id && can_act(c)) {
                return false;
            }
        }
        false
    }

    /// The chance `caster` keeps concentration through one round of its opponents' turns (ADR
    /// 0012). An able opponent that ranks the caster first counts its attack-roll and save damage
    /// on it; any other counts only its area saves. Each uses the ready action with the most
    /// counted damage. Each expected hit, and each save effect, is one Constitution save at
    /// `max(10, damage / 2)`. 0 when the counted damage would drop the caster.
    fn concentration_keep(&self, caster: &Combatant) -> f64 {
        let save_at = |damage: f64| self.save_success(caster, crate::stat_block::Ability::Con, ((damage / 2.0).floor() as i32).max(10));
        let mut counted = 0.0;
        let mut keep = 1.0;
        for foe in self.opponents(caster).filter(|o| can_act(o)) {
            let Some(block) = self.block(foe) else { continue };
            let focused = self.ranked_opponents(foe, None).first().is_some_and(|t| t.id == caster.id);
            let mut best: Option<((f64, f64), f64)> = None;
            for a in block.actions.iter().filter(|a| !a.effects.is_empty() && engine::ability_ready(self.state, &foe.id, a)) {
                let attacks = if focused { self.roll_value(foe, block, &a.effects, caster, Rolls::Attacks(AttackFilter::Any), 0) } else { (0.0, 0.0) };
                let saves = self.roll_value(foe, block, &a.effects, caster, if focused { Rolls::Saves(None) } else { Rolls::AreaSaves }, 0).0;
                if best.map_or(true, |(b, s)| attacks.0 + saves > b.0 + s + 1e-9) {
                    best = Some((attacks, saves));
                }
            }
            let Some(((d, a), s)) = best else { continue };
            counted += d + s;
            if d > 0.0 && a > 0.0 {
                keep *= save_at(a).powf(d / a);
            }
            if s > 0.0 {
                keep *= save_at(s);
            }
        }
        if counted >= hit_points(caster) {
            0.0
        } else {
            keep
        }
    }

    /// Sum of every living opponent's threat: what could land before `c` acts again.
    fn incoming(&self, c: &Combatant) -> f64 {
        self.opponents(c).filter(|o| o.end_state == EndState::Active).map(|o| self.threat(o)).sum()
    }

    /// A healer heals others through spells; a bare `heal` effect, like Second Wind, heals only its user.
    fn is_healer(&self, block: &StatBlock) -> bool {
        block.actions.iter().chain(&block.bonus_actions).any(|a| self.heals_others(block, &a.effects))
    }

    fn heals_others(&self, block: &StatBlock, effects: &[Effect]) -> bool {
        effects.iter().any(|e| match e {
            Effect::Cast(cast) => engine::bound_cast_effects(block, cast).is_some_and(|(fx, _)| self.heals(block, &fx)),
            Effect::Area(area) => self.heals(block, std::slice::from_ref(&*area.effect)),
            _ => false,
        })
    }

    /// Casting a concentration spell would end one whose conditions still hold an opponent,
    /// or whose concentration modifiers are still on a combatant (a Haste on an ally).
    fn breaks_own_concentration(&self, actor: &Combatant, block: &StatBlock, ability: &AbilityEntry) -> bool {
        let concentration = ability.effects.iter().any(|e| match e {
            Effect::Cast(c) => engine::bound_cast_effects(block, c).is_some_and(|(_, conc)| conc),
            _ => false,
        });
        let own = Some(actor.id.as_str());
        let holds_modifier = self
            .state
            .active_modifiers
            .iter()
            .any(|m| m.source.as_deref() == own && matches!(m.remaining, Duration::Concentration));
        concentration
            && actor.concentration.is_some()
            && (holds_modifier || self.opponents(actor).any(|o| o.conditions.iter().any(|c| c.source.as_deref() == own)))
    }

    fn heals(&self, block: &StatBlock, effects: &[Effect]) -> bool {
        effects.iter().any(|e| match e {
            Effect::Heal(_) => true,
            Effect::Cast(cast) => engine::bound_cast_effects(block, cast).is_some_and(|(fx, _)| self.heals(block, &fx)),
            Effect::Area(area) => self.heals(block, std::slice::from_ref(&*area.effect)),
            _ => false,
        })
    }

    fn consumable_allowed(&self, actor: &Combatant, block: &StatBlock) -> bool {
        if self.is_healer(block) {
            return true;
        }
        let low = hp_fraction(actor) < self.constants.consumable_threshold;
        let lethal = self.incoming(actor) >= f64::from(actor.hp + actor.temp_hp);
        let ally_out = self
            .allies(actor)
            .any(|a| a.id != actor.id && a.end_state == EndState::Active && !can_act(a));
        low || lethal || ally_out
    }

    fn plan_for(&self, actor: &Combatant, block: &StatBlock, ability: &AbilityEntry, section: Section) -> Option<Plan> {
        let resource = usage_word(block, ability);
        if self.breaks_own_concentration(actor, block, ability) {
            return None;
        }
        if self.heals(block, &ability.effects) {
            return self.heal_plan(actor, block, ability, section, resource);
        }
        let (modifiers, only_modifiers) = top_modifiers(block, &ability.effects, 0);
        if !modifiers.is_empty() && only_modifiers {
            return self.modifier_plan(actor, block, ability, section, resource, &modifiers);
        }
        if only_self_teleports(block, &ability.effects, 0) {
            return Some(self.teleport_plan(actor, block, ability, section, resource));
        }
        let own = self.bearer_value(&BuffOutlook::new(actor), actor, modifiers.iter().filter(|m| m.bearer == ModifierBearer::Actor));
        if let Some((area, effect)) = area_of(block, &ability.effects) {
            return self.area_plan(actor, block, ability, section, resource, &area, &effect, false);
        }
        if let Some((area, effect)) = prose_area(ability) {
            return self.area_plan(actor, block, ability, section, resource, &area, &effect, true);
        }
        let melee = needs_engaged(block, &ability.effects, 0);
        let ranged = has_ranged_attack(block, &ability.effects, 0);
        let immediate = |t: &Combatant| self.ability_value(actor, block, ability, t, 0, Footing::default()).0.damage;
        let ranked = self.ranked_opponents(actor, Some(&immediate));
        for target in ranked {
            let band = self.band(actor, target);
            let mut movement = Vec::new();
            let mut penalty = 0.0;
            if melee && band != RangeBand::Engaged {
                let cost = band_cost(band, RangeBand::Engaged);
                if self.movement_left(actor) < cost || section == Section::Legendary {
                    continue;
                }
                movement.push(Movement { target_id: target.id.clone(), band: RangeBand::Engaged, feet: self.movement_left(actor) });
            }
            let engaged_foes: Vec<&Combatant> = self
                .opponents(actor)
                .filter(|o| o.end_state == EndState::Active && self.band(actor, o) == RangeBand::Engaged)
                .collect();
            let footing = Footing { moves: if movement.is_empty() { Moves::Stays } else { Moves::In } };
            let (mut value, mut picks) = self.ability_value(actor, block, ability, target, 0, footing);
            if ranged && !melee && !engaged_foes.is_empty() && section != Section::Legendary {
                let cost = 5 * engaged_foes.len() as u32;
                if self.movement_left(actor) >= cost {
                    let (stepped, stepped_picks) = self.ability_value(actor, block, ability, target, 0, Footing::STEPPED_OUT);
                    let oa = self.step_out_penalty(actor, &engaged_foes);
                    if stepped.worth() - oa > value.worth() {
                        value = stepped;
                        picks = stepped_picks;
                        penalty = oa;
                        let left = self.movement_left(actor);
                        movement = engaged_foes
                            .iter()
                            .map(|o| Movement { target_id: o.id.clone(), band: RangeBand::Near, feet: left })
                            .collect();
                    }
                }
            }
            value.damage = value.damage.min(hit_points(target));
            let kill = if value.damage >= hit_points(target) { self.threat(target) } else { 0.0 };
            let survival = kill - penalty + value.survival + own.survival;
            let w = &self.constants.weights;
            let cost = if is_limited(block, ability) { 1.0 } else { 0.0 };
            let score = w.expected_damage * (value.damage + own.damage) + w.control * value.control + w.survival * survival - w.resource_cost * cost;
            let mut reason = format!(
                "top-priority target {} (threat {:.1}), expected damage {:.1}",
                target.id,
                self.threat(target),
                value.damage
            );
            if value.control > 0.0 {
                reason.push_str(&format!(", control {:.1}", value.control));
            }
            if kill > 0.0 {
                reason.push_str(", likely drops it");
            }
            if penalty > 0.0 {
                reason.push_str(&format!(", steps out of melee despite {penalty:.1} opportunity damage"));
            }
            if value.survival > 0.0 {
                reason.push_str(&format!(", teleports clear of {:.1} opportunity damage", value.survival));
            }
            return Some(Plan {
                actor_id: actor.id.clone(),
                section,
                action_id: ability.id.clone(),
                targets: vec![target.id.clone()],
                movement,
                picks,
                expected_damage: value.damage,
                control: value.control,
                survival,
                score,
                reason,
                resource,
                area: None,
            });
        }
        None
    }

    /// A plan for an ability whose effects are all teleports of the actor. It targets nobody and
    /// is worth the Opportunity Attacks the actor avoids.
    fn teleport_plan(&self, actor: &Combatant, block: &StatBlock, ability: &AbilityEntry, section: Section, resource: Option<String>) -> Plan {
        let avoided = self.teleport_value(actor, Footing::default());
        let w = &self.constants.weights;
        let cost = if is_limited(block, ability) { 1.0 } else { 0.0 };
        let mut reason = "teleports".to_string();
        if avoided > 0.0 {
            reason.push_str(&format!(" clear of {avoided:.1} opportunity damage"));
        }
        Plan {
            actor_id: actor.id.clone(),
            section,
            action_id: ability.id.clone(),
            targets: Vec::new(),
            movement: Vec::new(),
            picks: Vec::new(),
            expected_damage: 0.0,
            control: 0.0,
            survival: avoided,
            score: w.survival * avoided - w.resource_cost * cost,
            reason,
            resource,
            area: None,
        }
    }

    /// A plan for an ability whose effects are all timed modifiers. `self` modifiers are valued
    /// on the actor; `target` ones on the actor or a living ally, whichever scores best (ties to
    /// the actor, then the lower id), and the plan targets that bearer.
    fn modifier_plan(&self, actor: &Combatant, block: &StatBlock, ability: &AbilityEntry, section: Section, resource: Option<String>, modifiers: &[TimedModifier]) -> Option<Plan> {
        let targeted = modifiers.iter().any(|m| m.bearer == ModifierBearer::Target);
        let mut bearers = vec![actor];
        if targeted {
            let mut allies: Vec<&Combatant> = self.allies(actor).filter(|a| a.id != actor.id && a.end_state == EndState::Active).collect();
            allies.sort_by(|a, b| a.id.cmp(&b.id));
            bearers.extend(allies);
        }
        let outlook = BuffOutlook::new(actor);
        let own = self.bearer_value(&outlook, actor, modifiers.iter().filter(|m| m.bearer == ModifierBearer::Actor));
        let w = &self.constants.weights;
        let cost = if is_limited(block, ability) { 1.0 } else { 0.0 };
        let mut best: Option<(f64, &Combatant, BearerValue)> = None;
        for bearer in bearers {
            let mut value = self.bearer_value(&outlook, bearer, modifiers.iter().filter(|m| m.bearer == ModifierBearer::Target));
            value.add(&own);
            let score = w.expected_damage * value.damage + w.survival * value.survival - w.resource_cost * cost;
            if best.as_ref().map_or(true, |(b, _, _)| score > b + 1e-9) {
                best = Some((score, bearer, value));
            }
        }
        let (score, bearer, value) = best?;
        let mut notes = value.notes.clone();
        if value.extra_action > 0.0 {
            notes.push(format!("extra action {:.1}", value.extra_action));
        }
        if let Some((rounds, h, k)) = value.horizon {
            notes.push(format!("{rounds:.1} of {h:.1} rounds at keep {k:.2}"));
        }
        if value.lethargy > 0.0 {
            notes.push(format!("lethargy -{:.1}", value.lethargy));
        }
        Some(Plan {
            actor_id: actor.id.clone(),
            section,
            action_id: ability.id.clone(),
            targets: if targeted { vec![bearer.id.clone()] } else { vec![] },
            movement: vec![],
            picks: vec![],
            expected_damage: value.damage,
            control: 0.0,
            survival: value.survival,
            score,
            reason: format!("{} on {} ({})", ability.id, bearer.id, notes.join(", ")),
            resource,
            area: None,
        })
    }

    /// What `actor`'s `modifiers` are worth on `bearer` over the buff horizon (ADR 0012), cut to
    /// the rounds the fight has left and, for concentration, weighted round by round by the
    /// chance the actor keeps concentration. A concentration modifier whose `on_end` stops the
    /// bearer acting costs its threat times the chance concentration breaks in the horizon. A
    /// modifier scores 0 on a bearer that already has one from the same `source_ability`.
    fn bearer_value<'m>(&self, outlook: &BuffOutlook, bearer: &Combatant, modifiers: impl Iterator<Item = &'m TimedModifier>) -> BearerValue {
        let actor = outlook.actor;
        let mut out = BearerValue::default();
        for m in modifiers {
            let held = m.source_ability.as_deref().is_some_and(|ability| {
                self.state.active_modifiers.iter().any(|a| a.bearer() == Some(bearer.id.as_str()) && a.source_ability.as_deref() == Some(ability))
            });
            let cap = f64::from(horizon(&m.duration));
            if held || cap == 0.0 {
                continue;
            }
            let h = cap.min(*outlook.rounds_left.get_or_init(|| self.rounds_left(actor)));
            let concentration = matches!(m.duration, Duration::Concentration);
            let k = if concentration { *outlook.keep.get_or_init(|| self.concentration_keep(actor)) } else { 1.0 };
            let rounds = weighted_rounds(h, k, self.acts_before_opponents(actor, bearer));
            if (k < 1.0 || h < cap) && out.horizon.is_none() {
                out.horizon = Some((rounds, h, k));
            }
            let lethargic = m.on_end.iter().any(|e| matches!(e, Effect::Condition(c) if engine::STOPS_ACTING.contains(&c.condition)));
            if concentration && lethargic && can_act(bearer) {
                let cost = self.threat(bearer) * (1.0 - k.powf(h));
                out.damage -= cost;
                out.lethargy += cost;
            }
            match (&m.target, &m.change) {
                (ModifierTarget::ArmorClass, ModifierChange::Add(n)) if *n > 0 => {
                    let attacks = self.incoming_rolls(bearer, Rolls::Attacks(AttackFilter::Any)).0;
                    out.survival += attacks * AC_POINT_VALUE * f64::from(*n) * rounds;
                    out.notes.push(format!("+{n} AC against {attacks:.1} attack damage"));
                }
                (ModifierTarget::SavingThrows { ability }, ModifierChange::Advantage) => {
                    let saves = self.incoming_rolls(bearer, Rolls::Saves(*ability)).0;
                    out.survival += saves * SAVE_ADVANTAGE_VALUE * rounds;
                    out.notes.push(match ability {
                        Some(a) => format!("advantage on {0} saves against {saves:.1} {0} save damage", ability_word(*a)),
                        None => format!("advantage on saves against {saves:.1} save damage"),
                    });
                }
                (ModifierTarget::AttackRolls, ModifierChange::Add(n)) if *n > 0 => {
                    out.damage += self.threat(bearer) * AC_POINT_VALUE * f64::from(*n) * rounds;
                    out.notes.push(format!("+{n} to attack rolls"));
                }
                (ModifierTarget::ExtraAction { only }, _) => {
                    let per_round = self.extra_action_value(bearer, only);
                    out.damage += per_round * rounds;
                    out.extra_action += per_round;
                }
                _ => {}
            }
        }
        out
    }

    /// The bearer's best expected damage from one action its extra action allows, against its
    /// top-priority opponent, capped at that opponent's hit points; 0 when it can't act now,
    /// as in Haste's lethargy.
    fn extra_action_value(&self, bearer: &Combatant, only: &[ExtraActionUse]) -> f64 {
        if !can_act(bearer) {
            return 0.0;
        }
        let Some(block) = self.block(bearer) else { return 0.0 };
        let Some(target) = self.ranked_opponents(bearer, None).into_iter().next() else { return 0.0 };
        block
            .actions
            .iter()
            .filter(|a| !a.effects.is_empty() && engine::extra_action_allows(only, a) && engine::ability_ready(self.state, &bearer.id, a))
            .map(|a| self.ability_value(bearer, block, a, target, 0, Footing::default()).0.damage.min(hit_points(target)))
            .fold(0.0, f64::max)
    }

    fn heal_plan(&self, actor: &Combatant, block: &StatBlock, ability: &AbilityEntry, section: Section, resource: Option<String>) -> Option<Plan> {
        let amount = self.heal_amount(block, &ability.effects);
        if amount <= 0.0 {
            return None;
        }
        let self_only = !self.heals_others(block, &ability.effects);
        let healer = !self_only && self.is_healer(block);
        let enemies_left = self.opponents(actor).any(|o| o.end_state == EndState::Active);
        if !enemies_left {
            return None;
        }
        let downed: Vec<&Combatant> = self
            .allies(actor)
            .filter(|a| a.end_state == EndState::Unconscious || a.end_state == EndState::Stable)
            .collect();
        let w = &self.constants.weights;
        if healer {
            let best = downed
                .iter()
                .map(|a| (self.revived_threat(a), *a))
                .filter(|(t, _)| *t > 0.0)
                .max_by(|x, y| x.0.total_cmp(&y.0).then(y.1.id.cmp(&x.1.id)));
            if let Some((contribution, ally)) = best {
                let survival = 2.0 * contribution;
                return Some(Plan {
                    actor_id: actor.id.clone(),
                    section,
                    action_id: ability.id.clone(),
                    targets: vec![ally.id.clone()],
                    movement: vec![],
                    picks: vec![],
                    expected_damage: 0.0,
                    control: 0.0,
                    survival,
                    score: w.survival * survival,
                    reason: format!("revives downed {} (expected {:.1} per round over two rounds)", ally.id, contribution),
                    resource,
                    area: None,
                });
            }
        }
        let mut best: Option<(f64, &Combatant)> = None;
        for ally in self.allies(actor).filter(|a| a.end_state == EndState::Active && (!self_only || a.id == actor.id)) {
            if hp_fraction(ally) >= self.constants.heal_threshold {
                continue;
            }
            let likely_downed = self.incoming(ally) >= f64::from(ally.hp + ally.temp_hp);
            if !likely_downed {
                continue;
            }
            let restored = amount.min(f64::from(ally.max_hp - ally.hp));
            if best.map_or(true, |(b, c)| restored > b || (restored == b && ally.id < c.id)) {
                best = Some((restored, ally));
            }
        }
        let (restored, ally) = best?;
        Some(Plan {
            actor_id: actor.id.clone(),
            section,
            action_id: ability.id.clone(),
            targets: vec![ally.id.clone()],
            movement: vec![],
            picks: vec![],
            expected_damage: 0.0,
            control: 0.0,
            survival: restored,
            score: w.survival * restored,
            reason: format!("heals {} below {:.0}% HP and likely downed before its next turn, restoring {restored:.1}", ally.id, self.constants.heal_threshold * 100.0),
            resource,
            area: None,
        })
    }

    /// Threat a downed ally would carry once back up, measured as if it could act.
    fn revived_threat(&self, ally: &Combatant) -> f64 {
        let mut up = ally.clone();
        up.end_state = EndState::Active;
        up.hp = up.hp.max(1);
        self.threat(&up)
    }

    fn heal_amount(&self, block: &StatBlock, effects: &[Effect]) -> f64 {
        effects
            .iter()
            .map(|e| match e {
                Effect::Heal(h) => h.dice.as_deref().map_or(0.0, dice_average) + f64::from(h.modifier),
                Effect::Cast(cast) => engine::bound_cast_effects(block, cast).map_or(0.0, |(fx, _)| self.heal_amount(block, &fx)),
                Effect::Area(area) => self.heal_amount(block, std::slice::from_ref(&*area.effect)),
                _ => 0.0,
            })
            .sum()
    }

    #[allow(clippy::too_many_arguments)]
    fn area_plan(
        &self,
        actor: &Combatant,
        block: &StatBlock,
        ability: &AbilityEntry,
        section: Section,
        resource: Option<String>,
        area: &Area,
        effect: &Effect,
        from_prose: bool,
    ) -> Option<Plan> {
        let class = area_class(area);
        let counts = &self.constants.area_counts;
        let count = match class {
            AreaClass::Small => counts.small,
            AreaClass::Medium => counts.medium,
            AreaClass::Large => counts.large,
        } as usize;
        let per_target = |t: &Combatant| {
            let mut v = self.effect_value(actor, block, effect, t, 0, Footing::default()).0;
            v.damage = v.damage.min(hit_points(t));
            v
        };
        let immediate = |t: &Combatant| per_target(t).damage;
        let ranked = self.ranked_opponents(actor, Some(&immediate));
        if ranked.is_empty() {
            return None;
        }
        let enemies: Vec<&Combatant> = ranked.iter().take(count).copied().collect();
        let caught_allies: Vec<&Combatant> = if class == AreaClass::Small {
            vec![]
        } else {
            self.allies(actor)
                .filter(|a| a.id != actor.id && a.end_state == EndState::Active)
                .filter(|a| enemies.iter().any(|e| self.band(a, e) == RangeBand::Engaged))
                .take(count.saturating_sub(enemies.len()).max(if enemies.len() > 1 { 1 } else { 0 }))
                .collect()
        };
        let sum = |list: &[&Combatant]| list.iter().fold(Value::default(), |mut acc, t| {
            acc.add(per_target(t));
            acc
        });
        let full = sum(&enemies);
        let harm = sum(&caught_allies);
        let full_net = full.damage + full.control - harm.damage - harm.control;
        let tight = per_target(ranked[0]);
        let tight_net = tight.damage + tight.control;
        let wasted = count.saturating_sub(enemies.len() + caught_allies.len());
        let (value, harm_damage, targets, area_note) = if tight_net > full_net && !caught_allies.is_empty() {
            (tight, 0.0, vec![ranked[0].id.clone()], format!("{} area placed on {} alone to spare allies (1 target assumed)", class_word(class), ranked[0].id))
        } else {
            let mut ids: Vec<String> = enemies.iter().map(|c| c.id.clone()).collect();
            ids.extend(caught_allies.iter().map(|c| c.id.clone()));
            let note = format!(
                "{} area, {count} targets assumed ({} opponents, {} allies, {wasted} wasted)",
                class_word(class),
                enemies.len(),
                caught_allies.len()
            );
            (full, harm.damage + harm.control, ids, note)
        };
        let area_note = if from_prose { format!("{area_note}; shape read from the ability text") } else { area_note };
        let w = &self.constants.weights;
        let survival = -harm_damage;
        let score = w.expected_damage * value.damage + w.control * value.control + w.survival * survival
            - w.resource_cost * if is_limited(block, ability) { 1.0 } else { 0.0 };
        Some(Plan {
            actor_id: actor.id.clone(),
            section,
            action_id: ability.id.clone(),
            targets,
            movement: vec![],
            picks: vec![],
            expected_damage: value.damage,
            control: value.control,
            survival,
            score,
            reason: format!("area effect, expected damage {:.1}, control {:.1}, ally harm {harm_damage:.1}", value.damage, value.control),
            resource,
            area: Some(area_note),
        })
    }

    fn ability_value(&self, actor: &Combatant, block: &StatBlock, ability: &AbilityEntry, target: &Combatant, depth: u8, footing: Footing) -> (Value, Vec<Vec<usize>>) {
        let mut total = Value::default();
        let mut picks = Vec::new();
        for effect in &ability.effects {
            let (v, p) = self.effect_value(actor, block, effect, target, depth, footing);
            total.add(v);
            picks.extend(p);
        }
        (total, picks)
    }

    fn action_value(&self, actor: &Combatant, block: &StatBlock, action_id: &str, target: &Combatant, depth: u8, footing: Footing) -> Value {
        if depth > 4 {
            return Value::default();
        }
        block
            .actions
            .iter()
            .find(|a| a.id == action_id)
            .map(|a| self.ability_value(actor, block, a, target, depth + 1, footing).0)
            .unwrap_or_default()
    }

    fn effect_value(&self, actor: &Combatant, block: &StatBlock, effect: &Effect, target: &Combatant, depth: u8, footing: Footing) -> (Value, Vec<Vec<usize>>) {
        let Some(target_block) = self.block(target) else { return (Value::default(), vec![]) };
        let value = match effect {
            Effect::Attack(attack) => {
                let ac = target_block.armor_class.first().map_or(10, |a| a.value);
                let within = match footing.moves {
                    Moves::Stays => Within::Held,
                    Moves::In => Within::HeldAnd(&target.id),
                    Moves::Out => Within::Nobody,
                };
                let disposition = engine::disposition_with(self.state, actor, target, attack, within);
                let base = (f64::from(21 - (ac - attack.modifier)) / 20.0).clamp(0.05, 0.95);
                let (p_hit, p_crit) = match disposition {
                    RollDisposition::Advantage => (1.0 - (1.0 - base).powi(2), 1.0 - 0.95f64.powi(2)),
                    RollDisposition::Disadvantage => (base * base, 0.0025),
                    RollDisposition::Normal => (base, 0.05),
                };
                let mut dmg = 0.0;
                let mut crit_dice = 0.0;
                for d in attack.damage.iter().filter(|d| damage_applies(target, &actor.id, d)) {
                    dmg += adjusted(target_block, d.damage_type, dice_average(&d.dice) + f64::from(d.modifier));
                    crit_dice += adjusted(target_block, d.damage_type, dice_average(&d.dice));
                }
                let mut v = Value { damage: p_hit * dmg + p_crit * crit_dice, ..Default::default() };
                for on_hit in &attack.on_hit {
                    v.add(self.effect_value(actor, block, on_hit, target, depth + 1, footing).0.scaled(p_hit));
                }
                v
            }
            Effect::SavingThrow(save) => {
                let p_success = self.save_success(target, save.ability, save.dc);
                let p_fail = 1.0 - p_success;
                let dmg: f64 = save
                    .damage
                    .iter()
                    .filter(|d| damage_applies(target, &actor.id, d))
                    .map(|d| adjusted(target_block, d.damage_type, dice_average(&d.dice) + f64::from(d.modifier)))
                    .sum();
                let on_success = if save.no_damage_on_success { 0.0 } else if save.half_on_success { 0.5 } else { 1.0 };
                let mut v = Value { damage: dmg * (p_fail + p_success * on_success), ..Default::default() };
                for fail in &save.on_failure {
                    v.add(self.effect_value(actor, block, fail, target, depth + 1, footing).0.scaled(p_fail));
                }
                v
            }
            Effect::Damage(d) => {
                let amount = if damage_applies(target, &actor.id, d) {
                    adjusted(target_block, d.damage_type, dice_average(&d.dice) + f64::from(d.modifier))
                } else {
                    0.0
                };
                Value { damage: amount, ..Default::default() }
            }
            Effect::Condition(c) => {
                let immune = target_block.condition_immunities.contains(&c.condition);
                let has = target.conditions.iter().any(|x| x.condition == c.condition);
                let factor = if immune || has || depth > 3 { 0.0 } else { condition_factor(c.condition) };
                let target_threat = if factor == 0.0 || self.threat_nesting.get() >= MAX_THREAT_NESTING { 0.0 } else { self.threat(target) };
                Value { control: factor * target_threat, ..Default::default() }
            }
            Effect::Heal(h) => Value { heal: h.dice.as_deref().map_or(0.0, dice_average) + f64::from(h.modifier), ..Default::default() },
            Effect::MoveThenAttack(m) => self.action_value(actor, block, &m.attack_id, target, depth, footing),
            Effect::UseAction(u) => self.action_value(actor, block, &u.action_id, target, depth, footing),
            Effect::Cast(cast) => match engine::bound_cast_effects(block, cast) {
                Some((effects, _)) if depth < 4 => effects.iter().fold(Value::default(), |mut acc, e| {
                    acc.add(self.effect_value(actor, block, e, target, depth + 1, footing).0);
                    acc
                }),
                _ => Value::default(),
            },
            Effect::Area(area) => self.effect_value(actor, block, &area.effect, target, depth + 1, footing).0,
            Effect::Multiattack(multi) => return self.multiattack_value(actor, block, &multi.steps, target, depth, footing),
            Effect::ForcedMove(m) => {
                let factor = if depth > 3 { 0.0 } else { self.forced_move_factor(target, target_block, m) };
                let target_threat = if factor == 0.0 || self.threat_nesting.get() >= MAX_THREAT_NESTING { 0.0 } else { self.threat(target) };
                Value { control: factor * target_threat, ..Default::default() }
            }
            Effect::Teleport(teleport) if teleport.subject == TeleportSubject::Actor => Value { survival: self.teleport_value(actor, footing), ..Default::default() },
            Effect::Teleport(_) => Value::default(),
            Effect::TempHp(_) | Effect::TimedModifier(_) | Effect::RollModifier(_) | Effect::NoProvoke => Value::default(),
        };
        (value, vec![])
    }

    /// The chance `saver` succeeds on a save of `ability` against `dc`, from every source the engine
    /// reads: the stat block's save, timed save bonuses, Advantage and Disadvantage, and the
    /// automatic Strength and Dexterity failures. A save has no natural 1 or 20 rule.
    fn save_success(&self, saver: &Combatant, ability: crate::stat_block::Ability, dc: i32) -> f64 {
        if engine::automatic_save_failure(saver, ability).is_some() {
            return 0.0;
        }
        let Some(block) = self.block(saver) else { return 0.0 };
        let modifier = block.saving_throws.get(&ability).copied().unwrap_or_else(|| block.ability_scores.modifier(ability))
            + engine::save_modifier(self.state, saver, ability);
        let p = (f64::from(21 - (dc - modifier)) / 20.0).clamp(0.0, 1.0);
        match engine::save_disposition(self.state, saver, ability) {
            RollDisposition::Advantage => 1.0 - (1.0 - p).powi(2),
            RollDisposition::Disadvantage => p * p,
            RollDisposition::Normal => p,
        }
    }

    /// A teleport's value (ADR 0012): the Opportunity Attacks the actor avoids by teleporting
    /// rather than walking out of the engagements the plan leaves it in, priced by the step-out
    /// penalty. Nothing when the plan already stepped out or nothing is engaged.
    fn teleport_value(&self, actor: &Combatant, footing: Footing) -> f64 {
        if footing.moves == Moves::Out {
            return 0.0;
        }
        let engaged: Vec<&Combatant> = self
            .opponents(actor)
            .filter(|o| o.end_state == EndState::Active && self.band(actor, o) == RangeBand::Engaged)
            .collect();
        if engaged.is_empty() {
            return 0.0;
        }
        self.step_out_penalty(actor, &engaged)
    }

    /// Share of the target's threat a push denies (ADR 0012): 1.0 for a push of 35 feet or more
    /// to Far on a target slower than 35 feet, 0.25 for a push of 5 feet or more, and 0 for a
    /// pull or a target with any attack that doesn't need engagement.
    fn forced_move_factor(&self, target: &Combatant, target_block: &StatBlock, m: &crate::effect::ForcedMove) -> f64 {
        let attacks: Vec<Effect> = target_block.actions.iter().chain(&target_block.bonus_actions).flat_map(|a| a.effects.iter().cloned()).collect();
        if m.direction.eq_ignore_ascii_case("pull") || m.feet < 5 || !needs_engaged(target_block, &attacks, 0) {
            return 0.0;
        }
        if m.feet >= 35 && engine::walk_speed(self.state, target) < 35 {
            1.0
        } else {
            0.25
        }
    }

    /// Value of a multiattack and the picks that reach it: the best `pick`
    /// options of each `Choice` and the best set of each `Alternatives`.
    fn multiattack_value(&self, actor: &Combatant, block: &StatBlock, steps: &[MultiattackStep], target: &Combatant, depth: u8, footing: Footing) -> (Value, Vec<Vec<usize>>) {
        let mut total = Value::default();
        let mut picks = Vec::new();
        let slot = |action_id: &str, cast: Option<&crate::effect::Cast>| match cast {
            Some(c) => self.effect_value(actor, block, &Effect::Cast(c.clone()), target, depth + 1, footing).0,
            None => self.action_value(actor, block, action_id, target, depth, footing),
        };
        for step in steps {
            match step {
                MultiattackStep::Action { action_id, count, cast } => {
                    total.add(slot(action_id, cast.as_ref()).scaled(f64::from(*count)));
                }
                MultiattackStep::Choice { options, pick } => {
                    let mut scored: Vec<(f64, usize, Value)> = options
                        .iter()
                        .enumerate()
                        .map(|(i, o)| {
                            let v = slot(&o.action_id, o.cast.as_ref()).scaled(f64::from(o.count));
                            (v.worth(), i, v)
                        })
                        .collect();
                    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
                    let mut chosen: Vec<usize> = Vec::new();
                    for (_, i, v) in scored.into_iter().take(*pick as usize) {
                        total.add(v);
                        chosen.push(i);
                    }
                    chosen.sort_unstable();
                    picks.push(chosen);
                }
                MultiattackStep::Alternatives { sets } => {
                    let mut best: Option<(f64, usize, Value, Vec<Vec<usize>>)> = None;
                    for (i, set) in sets.iter().enumerate() {
                        let (v, p) = self.multiattack_value(actor, block, set, target, depth, footing);
                        let s = v.worth();
                        if best.as_ref().map_or(true, |b| s > b.0 + 1e-9) {
                            best = Some((s, i, v, p));
                        }
                    }
                    if let Some((_, i, v, p)) = best {
                        total.add(v);
                        picks.push(vec![i]);
                        picks.extend(p);
                    }
                }
            }
        }
        (total, picks)
    }

    fn nested<T: Default>(&self, f: impl FnOnce() -> T) -> T {
        let open = self.threat_nesting.get();
        if open >= MAX_THREAT_NESTING {
            return T::default();
        }
        self.threat_nesting.set(open + 1);
        let value = f();
        self.threat_nesting.set(open);
        value
    }

    fn step_out_penalty(&self, mover: &Combatant, reactors: &[&Combatant]) -> f64 {
        self.nested(|| {
            reactors
                .iter()
                .filter(|r| engine::opportunity_eligible(self.state, &r.id, &mover.id))
                .filter_map(|r| self.opportunity_decision(r, mover).taken())
                .sum()
        })
    }

    fn opportunity_decision(&self, reactor: &Combatant, mover: &Combatant) -> OpportunityDecision {
        let pick = self.opportunity_value(reactor, mover);
        let reserve = self.reaction_reserve(reactor);
        OpportunityDecision { pick, reserve }
    }

    fn opportunity_value(&self, reactor: &Combatant, mover: &Combatant) -> Option<OpportunityPick> {
        self.nested(|| {
            let block = self.block(reactor)?;
            let mut best: Option<(f64, usize, &AbilityEntry)> = None;
            for attack in engine::opportunity_attack_candidates(block) {
                let damage = self.ability_value(reactor, block, attack, mover, 0, Footing::default()).0.damage;
                let order = block.actions.iter().position(|a| a.id == attack.id).unwrap_or(usize::MAX);
                let better = best.map_or(true, |(d, o, _)| damage > d + 1e-9 || ((damage - d).abs() <= 1e-9 && order < o));
                if better {
                    best = Some((damage, order, attack));
                }
            }
            let (damage, _, attack) = best?;
            let drops = damage >= hit_points(mover);
            let value = damage.min(hit_points(mover)) + if drops { self.threat(mover) } else { 0.0 };
            Some(OpportunityPick { attack_id: attack.id.clone(), value, drops })
        })
    }

    fn reaction_reserve(&self, reactor: &Combatant) -> f64 {
        self.nested(|| {
            let Some(block) = self.block(reactor) else { return 0.0 };
            block
                .reactions
                .iter()
                .filter_map(|r| r.trigger.map(|ReactionTrigger::HitByAttackRoll { attack }| (r, attack)))
                .filter(|(r, _)| engine::ability_ready(self.state, &reactor.id, r))
                .map(|(r, filter)| {
                    let n = ac_bonus(block, r);
                    if n <= 0 {
                        return 0.0;
                    }
                    let (d, h) = self.incoming_rolls(reactor, Rolls::Attacks(filter));
                    (AC_POINT_VALUE * f64::from(n) * d).min(h)
                })
                .fold(0.0, f64::max)
        })
    }

    /// Expected damage `reactor`'s opponents deal it through the rolls `kind` selects, each
    /// opponent using its best ready action for them, and the largest average attack hit among them.
    fn incoming_rolls(&self, reactor: &Combatant, kind: Rolls) -> (f64, f64) {
        let mut expected = 0.0;
        let mut largest: f64 = 0.0;
        for foe in self.opponents(reactor).filter(|o| can_act(o)) {
            let Some(block) = self.block(foe) else { continue };
            let best = block
                .actions
                .iter()
                .filter(|a| !a.effects.is_empty() && engine::ability_ready(self.state, &foe.id, a))
                .map(|a| self.roll_value(foe, block, &a.effects, reactor, kind, 0))
                .fold((0.0, 0.0), |b, v| if v.0 > b.0 + 1e-9 { v } else { b });
            expected += best.0;
            largest = largest.max(best.1);
        }
        (expected, largest)
    }

    fn roll_value(&self, actor: &Combatant, block: &StatBlock, effects: &[Effect], target: &Combatant, kind: Rolls, depth: u8) -> (f64, f64) {
        if depth > 4 {
            return (0.0, 0.0);
        }
        let nested = |id: &str| {
            block
                .actions
                .iter()
                .find(|a| a.id == id)
                .map_or((0.0, 0.0), |a| self.roll_value(actor, block, &a.effects, target, kind, depth + 1))
        };
        let mut total = (0.0, 0.0f64);
        let mut add = |v: (f64, f64), count: f64| {
            total.0 += v.0 * count;
            total.1 = total.1.max(v.1);
        };
        for effect in effects {
            match effect {
                Effect::Attack(a) if matches!(kind, Rolls::Attacks(filter) if filter.matches(a)) => {
                    let expected = self.effect_value(actor, block, effect, target, depth, Footing::default()).0.damage;
                    add((expected, self.hit_average(actor, a, target)), 1.0);
                }
                Effect::SavingThrow(save) if matches!(kind, Rolls::Saves(ability) if ability.map_or(true, |a| a == save.ability)) => {
                    add((self.effect_value(actor, block, effect, target, depth, Footing::default()).0.damage, 0.0), 1.0);
                }
                Effect::Area(area) if matches!(kind, Rolls::Saves(_) | Rolls::AreaSaves) => {
                    let inner = if matches!(kind, Rolls::AreaSaves) { Rolls::Saves(None) } else { kind };
                    add(self.roll_value(actor, block, std::slice::from_ref(&*area.effect), target, inner, depth + 1), 1.0);
                }
                Effect::UseAction(u) => add(nested(&u.action_id), 1.0),
                Effect::MoveThenAttack(m) => add(nested(&m.attack_id), 1.0),
                Effect::Cast(c) => {
                    if let Some((fx, _)) = engine::bound_cast_effects(block, c) {
                        add(self.roll_value(actor, block, &fx, target, kind, depth + 1), 1.0);
                    }
                }
                Effect::Multiattack(m) => {
                    for step in &m.steps {
                        add(self.multiattack_step_rolls(actor, block, step, target, kind, depth + 1), 1.0);
                    }
                }
                _ => {}
            }
        }
        total
    }

    fn multiattack_step_rolls(&self, actor: &Combatant, block: &StatBlock, step: &MultiattackStep, target: &Combatant, kind: Rolls, depth: u8) -> (f64, f64) {
        let slot = |action_id: &str, cast: Option<&crate::effect::Cast>| match cast {
            Some(c) => self.roll_value(actor, block, &[Effect::Cast(c.clone())], target, kind, depth),
            None => self.roll_value(actor, block, &[Effect::UseAction(crate::effect::UseAction { action_id: action_id.into() })], target, kind, depth),
        };
        match step {
            MultiattackStep::Action { action_id, count, cast } => {
                let v = slot(action_id, cast.as_ref());
                (v.0 * f64::from(*count), v.1)
            }
            MultiattackStep::Choice { options, pick } => {
                let mut scored: Vec<(usize, (f64, f64))> = options
                    .iter()
                    .enumerate()
                    .map(|(i, o)| {
                        let v = slot(&o.action_id, o.cast.as_ref());
                        (i, (v.0 * f64::from(o.count), v.1))
                    })
                    .collect();
                scored.sort_by(|a, b| b.1 .0.total_cmp(&a.1 .0).then(a.0.cmp(&b.0)));
                scored.into_iter().take(*pick as usize).fold((0.0, 0.0), |acc, (_, v)| (acc.0 + v.0, acc.1.max(v.1)))
            }
            MultiattackStep::Alternatives { sets } => sets
                .iter()
                .map(|set| {
                    set.iter()
                        .map(|s| self.multiattack_step_rolls(actor, block, s, target, kind, depth + 1))
                        .fold((0.0, 0.0), |acc, v| (acc.0 + v.0, f64::max(acc.1, v.1)))
                })
                .fold((0.0, 0.0), |b, v| if v.0 > b.0 + 1e-9 { v } else { b }),
        }
    }

    fn hit_average(&self, actor: &Combatant, attack: &crate::effect::Attack, target: &Combatant) -> f64 {
        let Some(target_block) = self.block(target) else { return 0.0 };
        attack
            .damage
            .iter()
            .filter(|d| damage_applies(target, &actor.id, d))
            .map(|d| adjusted(target_block, d.damage_type, dice_average(&d.dice) + f64::from(d.modifier)))
            .sum()
    }

    fn decide_hit(&self, window: &ReactionWindow, total: i32, ac: i32, critical: bool) -> (ReactionAnswer, String) {
        let reactor = &window.reactor_id;
        let hold = |why: String| (ReactionAnswer::Pass, format!("{reactor}: holds its reaction ({why})"));
        let Some(block) = self.combatant(reactor).and_then(|c| self.block(c)) else { return hold("no stat block".into()) };
        let offered: Vec<(usize, &AbilityEntry, i32)> = window
            .options
            .iter()
            .enumerate()
            .filter_map(|(i, o)| block.reactions.iter().find(|r| r.id == o.id).map(|r| (i, r, ac_bonus(block, r))))
            .collect();
        if critical {
            return hold(format!("{total} is a critical hit, which no reaction turns into a miss"));
        }
        let cheapest = offered
            .iter()
            .filter(|(_, _, n)| *n > 0 && total < ac + n)
            .min_by_key(|(i, r, _)| (u8::from(is_limited(block, r)), slot_level(block, r), *i));
        if let Some((_, r, n)) = cheapest {
            return (ReactionAnswer::Take(r.id.clone()), format!("{reactor}: {} ({total} vs AC {ac}+{n} turns the hit into a miss)", r.name));
        }
        match offered.iter().map(|(_, _, n)| *n).max().filter(|n| *n > 0) {
            Some(n) => hold(format!("{total} vs AC {ac}+{n} still hits")),
            None => hold("no reaction offered raises AC".into()),
        }
    }

    fn decide_opportunity(&self, window: &ReactionWindow, mover_id: &str) -> (ReactionAnswer, String) {
        let reactor_id = &window.reactor_id;
        let hold = |why: String| (ReactionAnswer::Pass, format!("{reactor_id}: holds its reaction against {mover_id} ({why})"));
        let (Some(reactor), Some(mover)) = (self.combatant(reactor_id), self.combatant(mover_id)) else {
            return hold("no such combatant".into());
        };
        let decision = self.opportunity_decision(reactor, mover);
        let Some(pick) = decision.pick.as_ref().filter(|p| window.option(&p.attack_id).is_some()) else {
            return hold("no at-will melee attack".into());
        };
        let drops = if pick.drops { ", likely drops it" } else { "" };
        if decision.taken().is_some() {
            let line = format!(
                "{reactor_id}: Opportunity Attack on {mover_id} ({}, expected {:.1} > reserve {:.1}{drops})",
                pick.attack_id, pick.value, decision.reserve
            );
            return (ReactionAnswer::Take(pick.attack_id.clone()), line);
        }
        hold(format!("{}, expected {:.1} <= reserve {:.1}{drops}", pick.attack_id, pick.value, decision.reserve))
    }
}

/// Rounds of benefit over a horizon of `h` rounds when each round holds with chance `keep`:
/// `Σ_{i=1..ceil(h)} keep^(i - f) × clamp(h - (i - 1), 0, 1)`, with `f` 1 when the first
/// round's benefit lands before any opponent acts.
fn weighted_rounds(h: f64, keep: f64, first_is_safe: bool) -> f64 {
    let f = i32::from(first_is_safe);
    (1..=h.ceil() as i32).map(|i| keep.powi(i - f) * (h - f64::from(i - 1)).clamp(0.0, 1.0)).sum()
}

/// Damage that can still land; anything past this is wasted.
fn hit_points(c: &Combatant) -> f64 {
    f64::from(c.hp.max(1) + c.temp_hp)
}

fn hp_fraction(c: &Combatant) -> f64 {
    if c.max_hp <= 0 {
        return 0.0;
    }
    f64::from(c.hp.max(0)) / f64::from(c.max_hp)
}

/// A consumable is carried gear with a per-day or per-rest limit, such as a potion.
fn is_consumable(block: &StatBlock, ability: &AbilityEntry) -> bool {
    block.gear.contains(&ability.id) && matches!(ability.usage, Some(Usage::PerDay { .. } | Usage::PerRest { .. }))
}

fn is_limited(block: &StatBlock, ability: &AbilityEntry) -> bool {
    slot_level(block, ability) > 0 || !matches!(ability.usage, None | Some(Usage::AtWill))
}

/// The slot a cast uses: its named slot level, or, when it names none and the caster has
/// spell slots, the spell's own level, as the engine's `cast_level` and `spend_slot` read it.
fn cast_slot(block: &StatBlock, cast: &crate::effect::Cast) -> Option<u8> {
    let level = match cast.slot_level {
        Some(level) => level,
        None if block.spell_slots.is_some() => crate::spell::catalog().ok()?.get(&cast.spell_id)?.level,
        None => 0,
    };
    (level > 0).then_some(level)
}

/// The AC bonus n a hit-trigger reaction gives: its `armor_class` timed
/// modifiers, read through a cast's bound effects. 0 when it gives none.
pub(crate) fn ac_bonus(block: &StatBlock, reaction: &AbilityEntry) -> i32 {
    fn walk(block: &StatBlock, effects: &[Effect], depth: u8) -> i32 {
        if depth > 4 {
            return 0;
        }
        effects
            .iter()
            .map(|e| match e {
                Effect::TimedModifier(m) if m.target == ModifierTarget::ArmorClass => match m.change {
                    ModifierChange::Add(n) => n,
                    _ => 0,
                },
                Effect::Cast(c) => engine::bound_cast_effects(block, c).map_or(0, |(fx, _)| walk(block, &fx, depth + 1)),
                _ => 0,
            })
            .sum()
    }
    walk(block, &reaction.effects, 0)
}

fn slot_level(block: &StatBlock, ability: &AbilityEntry) -> u8 {
    ability
        .effects
        .iter()
        .filter_map(|e| match e {
            Effect::Cast(c) => cast_slot(block, c),
            _ => None,
        })
        .max()
        .unwrap_or(0)
}

fn usage_word(block: &StatBlock, ability: &AbilityEntry) -> Option<String> {
    let slot = ability.effects.iter().find_map(|e| match e {
        Effect::Cast(c) => cast_slot(block, c).map(|l| format!("level {l} slot")),
        _ => None,
    });
    let usage = match &ability.usage {
        None | Some(Usage::AtWill) => None,
        Some(Usage::Recharge { dice, .. }) => Some(format!("recharge {dice}")),
        Some(Usage::PerDay { uses }) => Some(format!("{uses}/day")),
        Some(Usage::PerRest { uses, .. }) => Some(format!("{uses}/rest")),
        Some(Usage::CooldownUntilNextTurn) => Some("cooldown".into()),
    };
    match (slot, usage) {
        (Some(s), Some(u)) => Some(format!("{s}, {u}")),
        (s, u) => s.or(u),
    }
}

/// The ability's top-level area effect, looking through casts.
fn area_of(block: &StatBlock, effects: &[Effect]) -> Option<(Area, Effect)> {
    effects.iter().find_map(|e| match e {
        Effect::Area(a) => Some((a.area.clone(), (*a.effect).clone())),
        Effect::Cast(c) => engine::bound_cast_effects(block, c).and_then(|(fx, _)| area_of(block, &fx)),
        _ => None,
    })
}

/// Area for a saving-throw ability whose effects carry no shape but whose
/// text says "each creature in a 60-foot Cone", as imported SRD breath weapons do.
fn prose_area(ability: &AbilityEntry) -> Option<(Area, Effect)> {
    let effect = match ability.effects.as_slice() {
        [e @ Effect::SavingThrow(_)] => e.clone(),
        _ => return None,
    };
    let desc = ability.desc.as_deref()?;
    let words: Vec<&str> = desc.split_whitespace().collect();
    for pair in words.windows(2) {
        let size = pair[0].strip_suffix("-foot").or_else(|| pair[0].strip_suffix("-foot-radius"));
        let Some(feet) = size.and_then(|n| n.parse::<u32>().ok()) else { continue };
        let shape = pair[1].trim_end_matches(|c: char| !c.is_ascii_alphabetic());
        let area = match shape {
            "Cone" => Area::Cone { size: feet },
            "Line" => Area::Line { length: feet, width: 5 },
            "Emanation" => Area::Emanation { radius: feet },
            "Sphere" => Area::Sphere { radius: feet },
            "Cube" => Area::Cube { size: feet },
            "Cylinder" => Area::Cylinder { radius: feet, height: feet },
            _ => continue,
        };
        return Some((area, effect));
    }
    None
}

/// Whether the ability's attacks are all melee, so the actor must be engaged.
fn needs_engaged(block: &StatBlock, effects: &[Effect], depth: u8) -> bool {
    let mut melee = false;
    let mut other = false;
    walk_attacks(block, effects, depth, &mut |a| {
        if a.range.is_none() {
            melee = true;
        } else {
            other = true;
        }
    });
    melee && !other
}

fn has_ranged_attack(block: &StatBlock, effects: &[Effect], depth: u8) -> bool {
    let mut ranged = false;
    walk_attacks(block, effects, depth, &mut |a| ranged |= a.range.is_some());
    ranged
}

fn walk_attacks(block: &StatBlock, effects: &[Effect], depth: u8, f: &mut dyn FnMut(&crate::effect::Attack)) {
    if depth > 4 {
        return;
    }
    let nested = |id: &str| block.actions.iter().find(|a| a.id == id).map(|a| a.effects.clone()).unwrap_or_default();
    for e in effects {
        match e {
            Effect::Attack(a) => f(a),
            Effect::UseAction(u) => walk_attacks(block, &nested(&u.action_id), depth + 1, f),
            Effect::MoveThenAttack(m) => walk_attacks(block, &nested(&m.attack_id), depth + 1, f),
            Effect::Cast(c) => {
                if let Some((fx, _)) = engine::bound_cast_effects(block, c) {
                    walk_attacks(block, &fx, depth + 1, f);
                }
            }
            Effect::Multiattack(m) => {
                for step in &m.steps {
                    match step {
                        MultiattackStep::Action { action_id, cast: None, .. } => walk_attacks(block, &nested(action_id), depth + 1, f),
                        MultiattackStep::Action { cast: Some(c), .. } => walk_attacks(block, &[Effect::Cast(c.clone())], depth + 1, f),
                        MultiattackStep::Choice { options, .. } => {
                            for o in options {
                                match &o.cast {
                                    Some(c) => walk_attacks(block, &[Effect::Cast(c.clone())], depth + 1, f),
                                    None => walk_attacks(block, &nested(&o.action_id), depth + 1, f),
                                }
                            }
                        }
                        MultiattackStep::Alternatives { sets } => {
                            for set in sets {
                                walk_attacks(block, &[Effect::Multiattack(crate::effect::Multiattack { steps: set.clone() })], depth + 1, f);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn area_class(area: &Area) -> AreaClass {
    let (feet, small, medium) = match area {
        Area::Cone { size } => (*size, 15, 30),
        Area::Line { length, .. } => (*length, 30, 90),
        Area::Emanation { radius } | Area::Sphere { radius } | Area::Cylinder { radius, .. } => (*radius, 10, 20),
        Area::Cube { size } => (*size, 10, 20),
    };
    if feet <= small {
        AreaClass::Small
    } else if feet <= medium {
        AreaClass::Medium
    } else {
        AreaClass::Large
    }
}

fn class_word(class: AreaClass) -> &'static str {
    match class {
        AreaClass::Small => "small",
        AreaClass::Medium => "medium",
        AreaClass::Large => "large",
    }
}

/// Share of a creature's next-turn threat a condition removes.
fn condition_factor(condition: Condition) -> f64 {
    match condition {
        Condition::Incapacitated | Condition::Paralyzed | Condition::Petrified | Condition::Stunned | Condition::Unconscious => 1.0,
        Condition::Restrained | Condition::Blinded | Condition::Frightened | Condition::Poisoned | Condition::Prone | Condition::Grappled => 0.25,
        Condition::Charmed => 0.5,
        _ => 0.0,
    }
}

fn damage_applies(target: &Combatant, actor_id: &str, d: &crate::effect::Damage) -> bool {
    match d.if_target {
        None => true,
        Some(condition) => target
            .conditions
            .iter()
            .any(|c| c.condition == condition && (!d.if_target_from_actor || c.source.as_deref() == Some(actor_id))),
    }
}

fn adjusted(block: &StatBlock, kind: crate::stat_block::DamageType, amount: f64) -> f64 {
    if block.damage_immunities.contains(&kind) {
        0.0
    } else if block.damage_vulnerabilities.contains(&kind) {
        amount * 2.0
    } else if block.damage_resistances.contains(&kind) {
        amount / 2.0
    } else {
        amount
    }
}

/// Average of an `NdM` expression; anything else averages to 0.
pub fn dice_average(expr: &str) -> f64 {
    let Some((n, m)) = expr.split_once('d') else { return 0.0 };
    match (n.parse::<u32>(), m.parse::<u32>()) {
        (Ok(n), Ok(m)) if m > 0 => f64::from(n) * (f64::from(m) + 1.0) / 2.0,
        _ => 0.0,
    }
}

fn band_cost(from: RangeBand, to: RangeBand) -> u32 {
    match (from, to) {
        (a, b) if a == b => 0,
        (RangeBand::Engaged, RangeBand::Near) | (RangeBand::Near, RangeBand::Engaged) => 5,
        (RangeBand::Near, RangeBand::Far) | (RangeBand::Far, RangeBand::Near) => 30,
        _ => 35,
    }
}

fn band_word(band: RangeBand) -> &'static str {
    match band {
        RangeBand::Engaged => "engaged",
        RangeBand::Near => "near",
        RangeBand::Far => "far",
    }
}

fn section_word(section: Section) -> &'static str {
    match section {
        Section::Action => "action",
        Section::Bonus => "bonus action",
        Section::Legendary => "legendary action",
        Section::Extra => "extra action",
    }
}

/// Rounds of a modifier's duration that count toward its value, at most [`BUFF_HORIZON`].
fn horizon(duration: &Duration) -> u32 {
    match duration {
        Duration::Rounds(n) => (*n).min(BUFF_HORIZON),
        Duration::UntilEndOfNextTurn => 1.min(BUFF_HORIZON),
        Duration::Instant | Duration::TriggeringAttack | Duration::UntilEndOfTurn => 0,
        _ => BUFF_HORIZON,
    }
}

fn ability_word(ability: crate::stat_block::Ability) -> String {
    serde_json::to_value(ability).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
}

/// True when every effect, read through a cast's bound effects, teleports the creature using
/// the ability.
fn only_self_teleports(block: &StatBlock, effects: &[Effect], depth: u8) -> bool {
    !effects.is_empty()
        && effects.iter().all(|e| match e {
            Effect::Teleport(t) => t.subject == TeleportSubject::Actor,
            Effect::Cast(c) if depth < 4 => engine::bound_cast_effects(block, c).is_some_and(|(fx, _)| only_self_teleports(block, &fx, depth + 1)),
            _ => false,
        })
}

/// The timed modifiers among an ability's top-level effects, read through a cast's bound
/// effects, and whether those are all its effects. Modifiers under a save, attack or area
/// land on that effect's targets and are left out.
fn top_modifiers(block: &StatBlock, effects: &[Effect], depth: u8) -> (Vec<TimedModifier>, bool) {
    let mut found = Vec::new();
    let mut only = true;
    for e in effects {
        match e {
            Effect::TimedModifier(m) => found.push(m.clone()),
            Effect::Cast(c) if depth < 4 => match engine::bound_cast_effects(block, c) {
                Some((fx, _)) => {
                    let (inner, inner_only) = top_modifiers(block, &fx, depth + 1);
                    found.extend(inner);
                    only &= inner_only;
                }
                None => only = false,
            },
            _ => only = false,
        }
    }
    (found, only)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::{Attack, ChoiceOption, Damage, Multiattack};
    use crate::encounter::{ActionEconomy, Side, StatBlockSnapshot};
    use crate::rng::RngState;
    use crate::stat_block::{AbilityKind, DamageType};
    use std::collections::HashMap;

    fn block(id: &str, hp: i32, ac: i32) -> StatBlock {
        serde_json::from_value(serde_json::json!({
            "id": id, "name": id, "source": "test", "schema_version": "1", "size": "medium",
            "type": "humanoid", "alignment": "neutral", "armor_class": [{"value": ac}],
            "hit_points": {"average": hp, "dice": "1d8"}, "speeds": {"walk": {"feet": 30}},
            "ability_scores": {"strength": 10, "dexterity": 10, "constitution": 10, "intelligence": 10, "wisdom": 10, "charisma": 10},
            "initiative": 0, "proficiency_bonus": 2, "senses": {"passive_perception": 10}
        }))
        .unwrap()
    }

    fn melee(id: &str, modifier: i32, dice: &str) -> AbilityEntry {
        AbilityEntry {
            id: id.into(),
            name: id.into(),
            desc: None,
            kind: AbilityKind::Modeled,
            effects: vec![Effect::Attack(Attack {
                modifier,
                reach: Some(5),
                range: None,
                target_count: 1,
                damage: vec![Damage { dice: dice.into(), damage_type: DamageType::Slashing, modifier: 0, magical: false, if_target: None, if_target_from_actor: false }],
                on_hit: vec![],
            })],
            agent_clauses: vec![],
            default_resolution: None,
            usage: None,
            trigger: None,
        }
    }

    fn fighter(id: &str, side: &str, hp: i32) -> Combatant {
        Combatant {
            id: id.into(),
            stat_block_id: id.into(),
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
            death_saves: Default::default(),
        }
    }

    fn encounter(blocks: Vec<StatBlock>, mut combatants: Vec<Combatant>) -> EncounterState {
        for i in 0..combatants.len() {
            for j in 0..combatants.len() {
                if combatants[i].side_id != combatants[j].side_id {
                    let other = combatants[j].id.clone();
                    combatants[i].ranges.insert(other, RangeBand::Near);
                }
            }
        }
        let order = combatants.iter().map(|c| c.id.clone()).collect::<Vec<_>>();
        EncounterState {
            id: "t".into(),
            engine_version: "0".into(),
            origin_directory: "/tmp".into(),
            stat_blocks: blocks.into_iter().map(|b| StatBlockSnapshot::from_stat_block(b.id.clone(), b)).collect(),
            sides: vec![Side { id: "party".into(), name: "Party".into() }, Side { id: "monsters".into(), name: "Monsters".into() }],
            combatants,
            round: 1,
            active_turn: order.first().cloned(),
            initiative_order: order,
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
        }
    }

    /// A monster with a claw against PCs whose threat is set by their weapon dice.
    fn duel(pc_dice: [&str; 2], pc_hp: [i32; 2]) -> EncounterState {
        let mut m = block("ogre", 50, 12);
        m.actions.push(melee("claw", 5, "2d8"));
        let mut a = block("a", pc_hp[0], 12);
        a.actions.push(melee("sword", 5, pc_dice[0]));
        let mut b = block("b", pc_hp[1], 12);
        b.actions.push(melee("sword", 5, pc_dice[1]));
        encounter(vec![m, a, b], vec![fighter("ogre", "monsters", 50), fighter("a", "party", pc_hp[0]), fighter("b", "party", pc_hp[1])])
    }

    fn choose_action(state: &EncounterState, actor: &str) -> Plan {
        choose(state, actor, Section::Action, &PolicyConstants::default()).expect("a plan")
    }

    #[test]
    fn target_priority_prefers_higher_threat() {
        let state = duel(["1d4", "2d12"], [20, 20]);
        assert_eq!(choose_action(&state, "ogre").targets, vec!["b"]);
    }

    #[test]
    fn target_ties_break_on_immediate_damage() {
        let mut state = duel(["1d8", "1d8"], [20, 20]);
        let b = state.stat_blocks.iter_mut().find(|s| s.id == "b").unwrap();
        b.stat_block.damage_resistances.push(DamageType::Slashing);
        assert_eq!(choose_action(&state, "ogre").targets, vec!["a"]);
    }

    #[test]
    fn target_ties_break_on_lowest_hp_fraction_then_id() {
        let mut state = duel(["1d8", "1d8"], [20, 20]);
        state.combatants[2].hp = 10;
        assert_eq!(choose_action(&state, "ogre").targets, vec!["b"]);
        state.combatants[2].hp = 20;
        assert_eq!(choose_action(&state, "ogre").targets, vec!["a"]);
    }

    #[test]
    fn downed_opponents_are_not_targets_unless_enabled() {
        let mut state = duel(["1d4", "2d12"], [20, 20]);
        state.combatants[2].end_state = EndState::Unconscious;
        state.combatants[2].hp = 0;
        assert_eq!(choose_action(&state, "ogre").targets, vec!["a"]);
        let mut constants = PolicyConstants::default();
        constants.apply("policy.target_downed", &serde_json::json!(true)).unwrap();
        let plan = choose(&state, "ogre", Section::Action, &constants).unwrap();
        assert!(plan.targets == vec!["a"] || plan.targets == vec!["b"]);
    }

    #[test]
    fn melee_plan_moves_in_only_when_needed() {
        let mut state = duel(["1d8", "1d8"], [20, 20]);
        let plan = choose_action(&state, "ogre");
        assert_eq!(plan.movement.len(), 1);
        assert_eq!(plan.movement[0].band, RangeBand::Engaged);
        state.combatants[0].engagement.push(plan.targets[0].clone());
        assert!(choose_action(&state, "ogre").movement.is_empty());
    }

    #[test]
    fn melee_is_illegal_without_movement_and_fallback_closes() {
        let mut state = duel(["1d8", "1d8"], [20, 20]);
        for c in &mut state.combatants {
            for band in c.ranges.values_mut() {
                *band = RangeBand::Far;
            }
        }
        assert!(choose(&state, "ogre", Section::Action, &PolicyConstants::default()).is_none());
        let fb = fallback(&state, "ogre", &PolicyConstants::default());
        assert_eq!(fb.movement[0].band, RangeBand::Near);
        assert!(fb.log_line().contains("no legal plan"));
    }

    #[test]
    fn limited_use_only_when_it_beats_at_will() {
        let mut state = duel(["1d8", "1d8"], [20, 20]);
        let ogre = state.stat_blocks.iter_mut().find(|s| s.id == "ogre").unwrap();
        let mut weak = melee("weak-burst", 5, "1d4");
        weak.usage = Some(Usage::PerDay { uses: 1 });
        let mut big = melee("big-burst", 5, "6d10");
        big.usage = Some(Usage::Recharge { dice: "5-6".into(), group: None });
        ogre.stat_block.actions.push(weak);
        ogre.stat_block.actions.push(big);
        let plan = choose_action(&state, "ogre");
        assert_eq!(plan.action_id, "big-burst");
        assert_eq!(plan.resource.as_deref(), Some("recharge 5-6"));
        state.combatants[0].resources.insert("ogre:big-burst".into(), 0);
        assert_eq!(choose_action(&state, "ogre").action_id, "claw");
    }

    #[test]
    fn multiattack_choice_picks_best_options() {
        let mut state = duel(["1d8", "1d8"], [40, 40]);
        let ogre = state.stat_blocks.iter_mut().find(|s| s.id == "ogre").unwrap();
        ogre.stat_block.actions.push(melee("bite", 5, "4d10"));
        ogre.stat_block.actions.push(AbilityEntry {
            effects: vec![Effect::Multiattack(Multiattack {
                steps: vec![MultiattackStep::Action { action_id: "claw".into(), count: 1, cast: None }, MultiattackStep::Choice {
                    options: vec![
                        ChoiceOption { action_id: "claw".into(), count: 1, cast: None },
                        ChoiceOption { action_id: "bite".into(), count: 1, cast: None },
                    ],
                    pick: 1,
                }],
            })],
            ..melee("multiattack", 0, "1d1")
        });
        let plan = choose_action(&state, "ogre");
        assert_eq!(plan.action_id, "multiattack");
        assert_eq!(plan.picks, vec![vec![1]]);
    }

    #[test]
    fn healer_heals_only_low_ally_likely_downed() {
        let mut state = duel(["1d8", "1d8"], [30, 30]);
        let heal = AbilityEntry {
            effects: vec![Effect::Cast(crate::effect::Cast { spell_id: "cure-wounds".into(), slot_level: Some(1), upcast: false })],
            ..melee("cure", 0, "1d1")
        };
        state.stat_blocks.iter_mut().find(|s| s.id == "b").unwrap().stat_block.actions.push(heal);
        state.active_turn = Some("b".into());
        state.combatants[2].engagement.push("ogre".into());
        assert_eq!(choose_action(&state, "b").action_id, "sword");
        state.combatants[1].hp = 6;
        let plan = choose_action(&state, "b");
        assert_eq!((plan.action_id.as_str(), plan.targets[0].as_str()), ("cure", "a"));
        state.combatants[1].hp = 0;
        state.combatants[1].end_state = EndState::Unconscious;
        let plan = choose_action(&state, "b");
        assert!(plan.reason.starts_with("revives"), "{}", plan.reason);
    }

    #[test]
    fn a_slotless_leveled_cast_uses_a_slot_only_when_the_caster_has_slots() {
        let cast = |spell: &str, slot_level: Option<u8>| AbilityEntry {
            effects: vec![Effect::Cast(crate::effect::Cast { spell_id: spell.into(), slot_level, upcast: false })],
            ..melee("cast", 0, "1d1")
        };
        let mut pc = block("cleric", 20, 10);
        pc.spell_slots = Some(vec![2]);
        let monster = block("fey", 20, 10);
        let cure = cast("cure-wounds", None);
        assert!(is_limited(&pc, &cure));
        assert_eq!(slot_level(&pc, &cure), 1);
        assert_eq!(usage_word(&pc, &cure).as_deref(), Some("level 1 slot"));
        assert!(!is_limited(&monster, &cure));
        assert_eq!(usage_word(&monster, &cure), None);
        let bolt = cast("fire-bolt", None);
        assert!(!is_limited(&pc, &bolt));
        assert_eq!(usage_word(&pc, &bolt), None);
        let upcast = cast("guiding-bolt", Some(2));
        assert!(is_limited(&monster, &upcast));
        assert_eq!(usage_word(&monster, &upcast).as_deref(), Some("level 2 slot"));
    }

    #[test]
    fn bare_heal_is_self_only_and_never_revives() {
        let mut state = duel(["1d8", "1d8"], [30, 30]);
        let second_wind = AbilityEntry {
            effects: vec![Effect::Heal(crate::effect::Heal { dice: Some("1d10".into()), modifier: 1, spellcasting_modifier: true })],
            usage: Some(Usage::PerRest { rest: crate::effect::RestLength::Short, uses: 1 }),
            ..melee("second-wind", 0, "1d1")
        };
        state.stat_blocks.iter_mut().find(|s| s.id == "b").unwrap().stat_block.bonus_actions.push(second_wind);
        state.active_turn = Some("b".into());
        state.combatants[1].hp = 0;
        state.combatants[1].end_state = EndState::Unconscious;
        state.combatants[2].hp = 3;
        let plan = choose(&state, "b", Section::Bonus, &PolicyConstants::default()).unwrap();
        assert_eq!(plan.targets, vec!["b"]);
        state.combatants[2].hp = 30;
        assert!(choose(&state, "b", Section::Bonus, &PolicyConstants::default()).is_none());
    }

    #[test]
    fn area_counts_and_ally_harm_are_logged() {
        let mut caster = block("mage", 20, 10);
        caster.actions.push(AbilityEntry {
            effects: vec![Effect::Area(crate::effect::AreaEffect {
                area: Area::Sphere { radius: 20 },
                origin: crate::effect::AreaOrigin::TargetPoint,
                effect: Box::new(Effect::Damage(Damage { dice: "8d6".into(), damage_type: DamageType::Fire, modifier: 0, magical: true, if_target: None, if_target_from_actor: false })),
                target_count: None,
            })],
            ..melee("fireball", 0, "1d1")
        });
        let mut ogre = block("ogre", 50, 12);
        ogre.actions.push(melee("claw", 5, "2d8"));
        let state = encounter(vec![caster, ogre], vec![fighter("mage", "party", 20), fighter("ogre", "monsters", 50)]);
        let plan = choose_action(&state, "mage");
        assert_eq!(plan.area.as_deref(), Some("medium area, 3 targets assumed (1 opponents, 0 allies, 2 wasted)"));
        let mut constants = PolicyConstants::default();
        constants.apply("policy.area_counts.medium", &serde_json::json!(1)).unwrap();
        let plan = choose(&state, "mage", Section::Action, &constants).unwrap();
        assert!(plan.area.as_deref().unwrap().contains("1 targets assumed"));
        assert!(plan.log_line().contains("area: medium area"));
    }

    #[test]
    fn breath_shape_is_read_from_prose() {
        let breath: AbilityEntry = serde_json::from_value(serde_json::json!({
            "id": "fire-breath", "name": "Fire Breath", "kind": "modeled",
            "desc": "Dexterity Saving Throw: DC 21, each creature in a 60-foot Cone. Failure: 59 (17d6) Fire damage.",
            "effects": [{"saving_throw": {"ability": "dex", "dc": 21, "half_on_success": true, "no_damage_on_success": false,
                "repeat_save": null, "two_stage": null, "hp_gate": null, "creature_type_gate": null,
                "damage": [{"dice": "17d6", "type": "fire"}]}}]
        }))
        .unwrap();
        let mut state = duel(["1d8", "1d8"], [80, 80]);
        state.stat_blocks.iter_mut().find(|s| s.id == "ogre").unwrap().stat_block.actions.push(breath);
        let plan = choose_action(&state, "ogre");
        assert_eq!(plan.action_id, "fire-breath");
        assert_eq!(plan.targets.len(), 2);
        assert!(plan.area.unwrap().contains("large area, 4 targets assumed (2 opponents, 0 allies, 2 wasted); shape read"));
    }

    #[test]
    fn fingerprint_changes_with_any_constant() {
        let base = fingerprint(&PolicyConstants::default());
        assert_eq!(base, fingerprint(&PolicyConstants::default()));
        let mut c = PolicyConstants::default();
        c.apply("policy.weights.control", &serde_json::json!(0.5)).unwrap();
        assert_ne!(base, fingerprint(&c));
        assert_eq!(c.apply("hp", &serde_json::json!(1)), Ok(false));
        assert!(c.apply("policy.nope", &serde_json::json!(1)).is_err());
    }

    #[test]
    fn decisions_are_deterministic() {
        let state = duel(["1d8", "1d8"], [20, 20]);
        let a = choose_action(&state, "ogre").log_line();
        let b = choose_action(&state, "ogre").log_line();
        assert_eq!(a, b);
        assert!(a.starts_with("DECISION ogre action: claw -> a | reason: top-priority target a"), "{a}");
    }

    #[test]
    fn standard_party_blocks_yield_plans() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("data/party/standard");
        let mut ogre = block("ogre", 59, 11);
        ogre.actions.push(melee("club", 6, "2d8"));
        for archetype in crate::party::ARCHETYPES {
            for level in [1u8, 5, 11, 20] {
                let id = crate::party::block_id(archetype, level);
                let text = std::fs::read_to_string(root.join(format!("{id}.json"))).unwrap();
                let pc: StatBlock = serde_json::from_str(&text).unwrap();
                let hp = pc.hit_points.average;
                let state = encounter(vec![pc, ogre.clone()], vec![fighter(&id, "party", hp), fighter("ogre", "monsters", 59)]);
                let plan = choose(&state, &id, Section::Action, &PolicyConstants::default());
                assert!(plan.is_some(), "{id} found no plan");
            }
        }
    }

    fn entry(value: serde_json::Value) -> AbilityEntry {
        serde_json::from_value(value).unwrap()
    }

    fn ranged(id: &str, modifier: i32, dice: &str) -> AbilityEntry {
        entry(serde_json::json!({"id": id, "name": id, "kind": "modeled", "effects": [{"attack": {"modifier": modifier, "range": [80, 320],
            "target_count": 1, "damage": [{"dice": dice, "type": "piercing"}]}}]}))
    }

    fn ac_reaction(id: &str, n: i32, filter: &str) -> AbilityEntry {
        entry(serde_json::json!({"id": id, "name": id, "kind": "modeled", "trigger": {"hit_by_attack_roll": {"attack": filter}},
            "effects": [{"timed_modifier": {"change": {"add": n}, "duration": "triggering_attack", "target": "armor_class"}}]}))
    }

    fn engage(state: &mut EncounterState, a: &str, b: &str) {
        for (x, y) in [(a, b), (b, a)] {
            let c = state.combatants.iter_mut().find(|c| c.id == x).unwrap();
            c.engagement.push(y.into());
            c.ranges.insert(y.into(), RangeBand::Engaged);
        }
    }

    fn snapshot<'a>(state: &'a mut EncounterState, id: &str) -> &'a mut StatBlock {
        &mut state.stat_blocks.iter_mut().find(|s| s.id == id).unwrap().stat_block
    }

    /// An archer with a bow engaged with a brute that has a claw.
    fn skirmish() -> EncounterState {
        let mut archer = block("archer", 20, 12);
        archer.actions.push(ranged("bow", 5, "2d6"));
        let mut brute = block("brute", 30, 12);
        brute.actions.push(melee("claw", 5, "1d8"));
        let mut state = encounter(vec![archer, brute], vec![fighter("archer", "party", 20), fighter("brute", "monsters", 30)]);
        engage(&mut state, "archer", "brute");
        state
    }

    fn ctx<'a>(state: &'a EncounterState, constants: &'a PolicyConstants) -> Ctx<'a> {
        Ctx { state, constants, threat_nesting: std::cell::Cell::new(0) }
    }

    fn penalty(state: &EncounterState) -> f64 {
        let constants = PolicyConstants::default();
        let ctx = ctx(state, &constants);
        ctx.step_out_penalty(ctx.combatant("archer").unwrap(), &[ctx.combatant("brute").unwrap()])
    }

    fn pick(state: &EncounterState, reactor: &str, mover: &str) -> OpportunityPick {
        let constants = PolicyConstants::default();
        let ctx = ctx(state, &constants);
        ctx.opportunity_value(ctx.combatant(reactor).unwrap(), ctx.combatant(mover).unwrap()).unwrap()
    }

    fn reserve(state: &EncounterState, reactor: &str) -> f64 {
        let constants = PolicyConstants::default();
        let ctx = ctx(state, &constants);
        ctx.reaction_reserve(ctx.combatant(reactor).unwrap())
    }

    fn opportunity_window(reactor: &str, mover: &str, options: &[&str]) -> ReactionWindow {
        ReactionWindow {
            reactor_id: reactor.into(),
            trigger: WindowTrigger::Opportunity { mover_id: mover.into() },
            options: options.iter().map(|id| crate::reaction::ReactionOption { id: id.to_string(), name: id.to_string() }).collect(),
        }
    }

    #[test]
    fn the_step_out_penalty_is_the_value_of_the_attacks_the_decider_takes() {
        let mut state = skirmish();
        snapshot(&mut state, "archer").actions = vec![ranged("bow", 5, "4d10")];
        let value = pick(&state, "brute", "archer").value;
        assert!(value > 0.0);
        assert_eq!(penalty(&state), value);
        let (answer, reason) = decide_reaction(&state, &opportunity_window("brute", "archer", &["claw"]), &PolicyConstants::default());
        assert_eq!(answer, ReactionAnswer::Take("claw".into()));
        assert_eq!(reason, format!("brute: Opportunity Attack on archer (claw, expected {value:.1} > reserve 0.0)"));
        let plan = choose_action(&state, "archer");
        assert_eq!(plan.movement.len(), 1, "{}", plan.reason);
        assert!(plan.reason.contains(&format!("despite {value:.1} opportunity damage")), "{}", plan.reason);
        assert!((plan.survival + value).abs() < 1e-9, "{}", plan.survival);
    }

    #[test]
    fn step_outs_that_draw_no_attack_price_zero() {
        let setups: [fn(&mut EncounterState); 5] = [
            |s| s.combatants[0].action_economy.disengaged = true,
            |s| snapshot(s, "archer").traits.push(AbilityEntry { effects: vec![Effect::NoProvoke], ..melee("flyby", 0, "1d1") }),
            |s| s.combatants[1].conditions.push(crate::encounter::ConditionInstance { condition: Condition::Blinded, duration: crate::effect::Duration::Permanent, source: None, ends_on_damage: false }),
            |s| snapshot(s, "brute").actions = vec![ranged("sling", 5, "1d4")],
            |s| {
                snapshot(s, "brute").actions = vec![melee("claw", 5, "1d4")];
                snapshot(s, "brute").reactions.push(ac_reaction("parry", 5, "any"));
                snapshot(s, "archer").actions = vec![ranged("bow", 8, "4d10")];
            },
        ];
        for (i, setup) in setups.into_iter().enumerate() {
            let mut state = skirmish();
            setup(&mut state);
            assert_eq!(penalty(&state), 0.0, "setup {i}");
        }
    }

    #[test]
    fn an_attack_that_drops_the_mover_adds_its_threat() {
        let mut state = skirmish();
        state.combatants[0].hp = 1;
        let constants = PolicyConstants::default();
        let threat = ctx(&state, &constants).threat(&state.combatants[0]);
        let p = pick(&state, "brute", "archer");
        assert!(p.drops && threat > 0.0);
        assert!((p.value - (1.0 + threat)).abs() < 1e-9, "{} vs 1 + {threat}", p.value);
    }

    #[test]
    fn the_opportunity_attack_is_the_best_melee_leaf_with_ties_in_stat_block_order() {
        let mut state = skirmish();
        let brute = snapshot(&mut state, "brute");
        brute.actions.push(melee("bite", 5, "2d10"));
        brute.actions.push(melee("rake", 5, "2d10"));
        brute.actions.insert(0, entry(serde_json::json!({"id": "multiattack", "name": "Multiattack", "kind": "modeled",
            "effects": [{"multiattack": {"steps": [{"action": {"action_id": "rake", "count": 1}}, {"action": {"action_id": "claw", "count": 1}}]}}]})));
        assert_eq!(pick(&state, "brute", "archer").attack_id, "bite");
    }

    #[test]
    fn the_reserve_counts_only_matching_attack_rolls() {
        let mut state = skirmish();
        snapshot(&mut state, "brute").reactions.push(ac_reaction("guard", 2, "any"));
        assert!(reserve(&state, "brute") > 0.0);
        snapshot(&mut state, "archer").actions = vec![entry(serde_json::json!({
            "id": "breath", "name": "Breath", "kind": "modeled",
            "effects": [{"saving_throw": {"ability": "dex", "dc": 15, "half_on_success": true, "no_damage_on_success": false,
                "repeat_save": null, "two_stage": null, "hp_gate": null, "creature_type_gate": null, "damage": [{"dice": "8d6", "type": "fire"}]}}]}))];
        assert_eq!(reserve(&state, "brute"), 0.0, "save damage adds nothing");
        let (answer, _) = decide_reaction(&state, &opportunity_window("brute", "archer", &["claw"]), &PolicyConstants::default());
        assert_eq!(answer, ReactionAnswer::Take("claw".into()));
        snapshot(&mut state, "archer").actions = vec![AbilityEntry {
            effects: vec![Effect::Heal(crate::effect::Heal { dice: Some("2d8".into()), modifier: 3, spellcasting_modifier: false })],
            ..melee("mend", 0, "1d1")
        }];
        assert_eq!(reserve(&state, "brute"), 0.0, "healing adds nothing");

        let mut state = skirmish();
        snapshot(&mut state, "brute").reactions.push(ac_reaction("parry", 2, "melee"));
        assert_eq!(reserve(&state, "brute"), 0.0, "Parry ignores the bow");
        snapshot(&mut state, "archer").actions.push(melee("knife", 5, "1d4"));
        assert!(reserve(&state, "brute") > 0.0, "Parry counts the knife");
    }

    #[test]
    fn a_reserve_never_exceeds_the_largest_single_hit() {
        let mut state = skirmish();
        snapshot(&mut state, "brute").reactions.push(ac_reaction("ward", 5, "any"));
        snapshot(&mut state, "archer").actions = vec![
            ranged("bow", 20, "1d6"),
            entry(serde_json::json!({"id": "volley", "name": "Volley", "kind": "modeled",
                "effects": [{"multiattack": {"steps": [{"action": {"action_id": "bow", "count": 8}}]}}]})),
        ];
        assert!((reserve(&state, "brute") - 3.5).abs() < 1e-9, "{}", reserve(&state, "brute"));
    }

    #[test]
    fn a_knight_with_parry_attacks_the_level_11_party_above_its_largest_hit() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let knight: StatBlock = serde_json::from_str(&std::fs::read_to_string(root.join("data/fixtures/knight.json")).unwrap()).unwrap();
        let mut blocks = vec![knight.clone()];
        let mut combatants = vec![fighter("knight", "monsters", knight.hit_points.average)];
        for archetype in crate::party::ARCHETYPES {
            let id = crate::party::block_id(archetype, 11);
            let pc: StatBlock = serde_json::from_str(&std::fs::read_to_string(root.join(format!("data/party/standard/{id}.json"))).unwrap()).unwrap();
            combatants.push(fighter(&id, "party", pc.hit_points.average));
            blocks.push(pc);
        }
        let mut state = encounter(blocks, combatants);
        let ids: Vec<String> = state.combatants[1..].iter().map(|c| c.id.clone()).collect();
        for id in &ids {
            engage(&mut state, "knight", id);
        }
        state.combatants.iter_mut().find(|c| c.id == "standard-caster-11").unwrap().hp = 1;
        let constants = PolicyConstants::default();
        let ctx = ctx(&state, &constants);
        let knight = ctx.combatant("knight").unwrap();
        let (_, largest) = ctx.incoming_rolls(knight, Rolls::Attacks(AttackFilter::Melee));
        let held = ctx.reaction_reserve(knight);
        assert!(held > 0.0 && held <= largest + 1e-9, "{held} vs {largest}");
        let mut above = 0;
        for id in &ids {
            let decision = ctx.opportunity_decision(knight, ctx.combatant(id).unwrap());
            let value = decision.pick.as_ref().unwrap().value;
            assert_eq!(decision.taken().is_some(), value > held, "{id}");
            if value > largest {
                assert!(decision.taken().is_some(), "{id}");
                above += 1;
            }
        }
        assert!(above > 0, "no party member is worth more than the largest hit {largest}");
    }

    #[test]
    fn the_penalty_and_reserve_stop_at_the_nesting_limit_and_teleports_return() {
        let mut state = skirmish();
        snapshot(&mut state, "brute").reactions.push(ac_reaction("parry", 2, "melee"));
        snapshot(&mut state, "archer").actions.push(entry(serde_json::json!({"id": "misty-step", "name": "Misty Step", "kind": "modeled",
            "effects": [{"teleport": {"range": 30, "destination": "unoccupied space you can see"}}]})));
        assert!(choose(&state, "archer", Section::Action, &PolicyConstants::default()).is_some());
        snapshot(&mut state, "archer").actions.push(melee("knife", 5, "1d4"));
        let constants = PolicyConstants::default();
        let ctx = ctx(&state, &constants);
        let (archer, brute) = (ctx.combatant("archer").unwrap(), ctx.combatant("brute").unwrap());
        assert!(ctx.reaction_reserve(brute) > 0.0);
        ctx.threat_nesting.set(MAX_THREAT_NESTING);
        assert_eq!(ctx.step_out_penalty(archer, &[brute]), 0.0);
        assert_eq!(ctx.reaction_reserve(brute), 0.0);
        assert!(ctx.opportunity_value(brute, archer).is_none());
    }

    #[test]
    fn hit_windows_take_the_cheapest_reaction_that_stops_the_hit() {
        let mut state = skirmish();
        let brute = snapshot(&mut state, "brute");
        brute.reactions.push(ac_reaction("ward", 5, "any"));
        brute.reactions.push(ac_reaction("parry", 2, "any"));
        brute.reactions[0].usage = Some(Usage::PerDay { uses: 1 });
        let hit = |total: i32, critical: bool| ReactionWindow {
            reactor_id: "brute".into(),
            trigger: WindowTrigger::Hit { attacker_id: "archer".into(), total, ac: 12, critical },
            options: ["ward", "parry"].iter().map(|id| crate::reaction::ReactionOption { id: id.to_string(), name: id.to_string() }).collect(),
        };
        let decide = |w: &ReactionWindow| decide_reaction(&state, w, &PolicyConstants::default());
        assert_eq!(decide(&hit(13, false)), (ReactionAnswer::Take("parry".into()), "brute: parry (13 vs AC 12+2 turns the hit into a miss)".into()));
        assert_eq!(decide(&hit(15, false)).0, ReactionAnswer::Take("ward".into()));
        assert_eq!(decide(&hit(17, false)), (ReactionAnswer::Pass, "brute: holds its reaction (17 vs AC 12+5 still hits)".into()));
        assert_eq!(decide(&hit(13, true)).0, ReactionAnswer::Pass);
    }

    #[test]
    fn the_fingerprint_carries_policy_version_3() {
        assert_eq!(POLICY_VERSION, 3);
        let constants = PolicyConstants::default();
        let v2 = crate::encounter::tamper_hash(&serde_json::json!({ "id": POLICY_ID, "version": 2, "constants": constants }));
        assert_ne!(fingerprint(&constants), v2);
    }

    fn shove(feet: u32, direction: &str) -> Effect {
        Effect::ForcedMove(crate::effect::ForcedMove { feet, direction: direction.into() })
    }

    fn forced_move_control(state: &EncounterState, effect: &Effect) -> (f64, f64) {
        let constants = PolicyConstants::default();
        let ctx = ctx(state, &constants);
        let (ogre, a) = (ctx.combatant("ogre").unwrap(), ctx.combatant("a").unwrap());
        let block = ctx.block(ogre).unwrap();
        (ctx.effect_value(ogre, block, effect, a, 0, Footing::default()).0.control, ctx.threat(a))
    }

    #[test]
    fn a_push_scores_a_quarter_of_a_melee_only_targets_threat() {
        let state = duel(["1d8", "1d8"], [20, 20]);
        let (control, threat) = forced_move_control(&state, &shove(10, "push"));
        assert!(threat > 0.0);
        assert_eq!(control, 0.25 * threat);
        assert_eq!(forced_move_control(&state, &shove(0, "push")).0, 0.0);
    }

    #[test]
    fn a_push_to_far_scores_the_whole_threat_of_a_slow_melee_only_target() {
        let mut state = duel(["1d8", "1d8"], [20, 20]);
        let (control, threat) = forced_move_control(&state, &shove(35, "push"));
        assert_eq!(control, threat);
        snapshot(&mut state, "a").speeds.get_mut("walk").unwrap().feet = 35;
        assert_eq!(forced_move_control(&state, &shove(35, "push")).0, 0.25 * threat);
    }

    #[test]
    fn a_pull_or_a_target_with_a_ranged_attack_scores_no_forced_movement() {
        let mut state = duel(["1d8", "1d8"], [20, 20]);
        assert_eq!(forced_move_control(&state, &shove(40, "pull")).0, 0.0);
        snapshot(&mut state, "a").bonus_actions = vec![ranged("dagger-throw", 5, "1d4")];
        assert_eq!(forced_move_control(&state, &shove(10, "push")).0, 0.0);
        snapshot(&mut state, "a").actions = vec![ranged("bow", 5, "1d8")];
        assert_eq!(forced_move_control(&state, &shove(40, "push")).0, 0.0);
    }

    #[test]
    fn a_shove_on_hit_scales_by_the_hit_chance() {
        let mut state = duel(["1d8", "1d8"], [20, 20]);
        let shoving = entry(serde_json::json!({"id": "slam", "name": "slam", "kind": "modeled", "effects": [{"attack": {"modifier": 5, "reach": 5,
            "target_count": 1, "damage": [], "on_hit": [{"forced_move": {"feet": 10, "direction": "push"}}]}}]}));
        snapshot(&mut state, "ogre").actions = vec![shoving];
        let plan = choose_action(&state, "ogre");
        assert_eq!(plan.action_id, "slam");
        assert!(plan.control > 0.0 && plan.reason.contains("control"), "{}", plan.reason);
    }

    fn party_block(archetype: &str, level: u8) -> StatBlock {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("data/party/standard");
        let id = crate::party::block_id(archetype, level);
        serde_json::from_str(&std::fs::read_to_string(root.join(format!("{id}.json"))).unwrap()).unwrap()
    }

    /// The level 5 caster, the given allies and an ogre with a heavy claw.
    fn caster_with(allies: Vec<StatBlock>) -> EncounterState {
        let caster = party_block("caster", 5);
        let mut ogre = block("ogre", 80, 12);
        ogre.actions.push(melee("claw", 6, "3d10"));
        let mut combatants = vec![fighter("ogre", "monsters", 80)];
        let mut fighters: Vec<Combatant> = std::iter::once(&caster).chain(&allies).map(|b| fighter(&b.id, "party", b.hit_points.average)).collect();
        combatants.append(&mut fighters);
        let blocks = std::iter::once(caster).chain(allies).chain(std::iter::once(ogre)).collect();
        encounter(blocks, combatants)
    }

    fn hasted_by(source: &str, bearer: &str) -> Vec<crate::encounter::ActiveModifier> {
        let caster = party_block("caster", 5);
        let haste = caster.actions.iter().find(|a| a.id == "haste").unwrap();
        let Effect::Cast(cast) = &haste.effects[0] else { panic!() };
        engine::bound_cast_effects(&caster, cast)
            .unwrap()
            .0
            .into_iter()
            .map(|e| {
                let Effect::TimedModifier(m) = e else { panic!() };
                crate::encounter::ActiveModifier {
                    target: m.target,
                    change: m.change,
                    remaining: m.duration,
                    source: Some(source.into()),
                    bearer: Some(bearer.into()),
                    source_ability: m.source_ability,
                    ends_on_damage: false,
                    on_end: m.on_end,
                }
            })
            .collect()
    }

    fn only_haste(state: &mut EncounterState) {
        snapshot(state, "standard-caster-05").actions.retain(|a| a.id == "haste");
    }

    #[test]
    fn the_caster_hastes_the_ally_with_the_highest_incoming_damage() {
        let mut squire = block("squire", 30, 5);
        squire.actions.clear();
        let mut wall = block("wall", 30, 22);
        wall.actions.clear();
        let mut state = caster_with(vec![wall, squire]);
        only_haste(&mut state);
        let plan = choose_action(&state, "standard-caster-05");
        assert_eq!(plan.action_id, "haste");
        assert_eq!(plan.targets, ["squire"]);
        assert!(plan.reason.starts_with("haste on squire (+2 AC against "), "{}", plan.reason);
        assert!(plan.survival > 0.0 && plan.expected_damage == 0.0);
    }

    #[test]
    fn ties_between_bearers_go_to_the_actor_then_the_lower_id() {
        let mut twin = block("b-twin", 30, 12);
        twin.actions.clear();
        let mut other = block("a-twin", 30, 12);
        other.actions.clear();
        let mut state = caster_with(vec![twin, other]);
        only_haste(&mut state);
        snapshot(&mut state, "standard-caster-05").armor_class[0].value = 12;
        assert_eq!(choose_action(&state, "standard-caster-05").targets, ["standard-caster-05"]);
        state.active_modifiers = hasted_by("x", "standard-caster-05");
        assert_eq!(choose_action(&state, "standard-caster-05").targets, ["a-twin"]);
    }

    #[test]
    fn a_second_haste_on_an_already_hasted_ally_scores_zero() {
        let mut squire = block("squire", 30, 5);
        squire.actions.clear();
        let mut state = caster_with(vec![squire]);
        only_haste(&mut state);
        state.active_modifiers = hasted_by("someone", "squire");
        let plan = choose_action(&state, "standard-caster-05");
        assert_eq!(plan.targets, ["standard-caster-05"], "{}", plan.reason);
        state.active_modifiers.extend(hasted_by("someone", "standard-caster-05"));
        assert!(choose(&state, "standard-caster-05", Section::Action, &PolicyConstants::default()).is_none());
    }

    #[test]
    fn slows_riders_add_no_buff_value_to_the_caster() {
        let mut state = caster_with(vec![]);
        let slow = entry(serde_json::json!({"id": "slow", "name": "Slow", "kind": "modeled",
            "effects": [{"cast": {"spell_id": "slow", "slot_level": 3, "upcast": false}}]}));
        snapshot(&mut state, "standard-caster-05").actions = vec![slow];
        assert!(choose(&state, "standard-caster-05", Section::Action, &PolicyConstants::default()).is_none());
    }

    #[test]
    fn a_self_modifier_is_valued_on_the_actor_only() {
        let mut squire = block("squire", 30, 5);
        squire.actions.clear();
        let mut state = caster_with(vec![squire]);
        let ward = entry(serde_json::json!({"id": "ward", "name": "Ward", "kind": "modeled",
            "effects": [{"timed_modifier": {"bearer": "self", "change": {"add": 3}, "duration": {"rounds": 1}, "source_ability": "ward", "target": "armor_class"}}]}));
        snapshot(&mut state, "standard-caster-05").actions = vec![ward];
        let plan = choose_action(&state, "standard-caster-05");
        assert!(plan.targets.is_empty());
        assert_eq!(plan.reason, "ward on standard-caster-05 (+3 AC against 13.2 attack damage)");
        let claw = 0.75 * 16.5 + 0.05 * 16.5;
        assert!((plan.survival - claw * 0.3).abs() < 1e-9, "{} vs {claw}", plan.survival);
    }

    fn self_buff(change: serde_json::Value, target: serde_json::Value) -> AbilityEntry {
        entry(serde_json::json!({"id": "ward", "name": "Ward", "kind": "modeled",
            "effects": [{"timed_modifier": {"bearer": "self", "change": change, "duration": {"rounds": 2}, "source_ability": "ward", "target": target}}]}))
    }

    fn breath(ability: &str) -> AbilityEntry {
        entry(serde_json::json!({"id": "breath", "name": "Breath", "kind": "modeled", "effects": [{"saving_throw": {"ability": ability, "dc": 15,
            "damage": [{"dice": "6d10", "type": "fire"}], "half_on_success": true, "no_damage_on_success": false}}]}))
    }

    #[test]
    fn an_ac_buff_is_worth_nothing_against_a_monster_whose_damage_is_all_saves() {
        let mut state = caster_with(vec![]);
        snapshot(&mut state, "standard-caster-05").actions = vec![self_buff(serde_json::json!({"add": 3}), serde_json::json!("armor_class"))];
        assert!(choose(&state, "standard-caster-05", Section::Action, &PolicyConstants::default()).is_some());
        snapshot(&mut state, "ogre").actions = vec![breath("dex")];
        assert!(choose(&state, "standard-caster-05", Section::Action, &PolicyConstants::default()).is_none());
    }

    #[test]
    fn save_advantage_counts_only_damage_from_saves_of_its_ability() {
        let mut state = caster_with(vec![]);
        let nimble = self_buff(serde_json::json!("advantage"), serde_json::json!({"saving_throws": {"ability": "dex"}}));
        snapshot(&mut state, "standard-caster-05").actions = vec![nimble];
        assert!(choose(&state, "standard-caster-05", Section::Action, &PolicyConstants::default()).is_none(), "the claw is no save");
        snapshot(&mut state, "ogre").actions = vec![breath("con")];
        assert!(choose(&state, "standard-caster-05", Section::Action, &PolicyConstants::default()).is_none(), "a Con save");
        snapshot(&mut state, "ogre").actions = vec![breath("dex")];
        let plan = choose_action(&state, "standard-caster-05");
        assert!(plan.reason.starts_with("ward on standard-caster-05 (advantage on dex saves against "), "{}", plan.reason);
        assert!(plan.survival > 0.0);
    }

    #[test]
    fn a_bearer_that_cant_act_gets_no_extra_action_value() {
        let mut state = caster_with(vec![party_block("defender", 5), party_block("striker", 5)]);
        let striker = state.combatants.iter_mut().find(|c| c.id == "standard-striker-05").unwrap();
        striker.conditions.push(crate::encounter::ConditionInstance {
            condition: Condition::Incapacitated,
            duration: Duration::UntilEndOfNextTurn,
            source: Some("standard-caster-05".into()),
            ends_on_damage: false,
        });
        only_haste(&mut state);
        let plan = choose_action(&state, "standard-caster-05");
        assert_eq!(plan.targets, ["standard-defender-05"], "{}", plan.reason);
        state.combatants.retain(|c| c.id != "standard-defender-05");
        let plan = choose_action(&state, "standard-caster-05");
        assert!(!plan.reason.contains("extra action"), "{}", plan.reason);
    }

    /// `caster_with` the allies and a decoy the ogre ranks first, whose flat smite no extra
    /// action allows, against an ogre with 1000 HP: a long fight where the caster is safe.
    fn long_fight(allies: Vec<StatBlock>) -> EncounterState {
        let mut decoy = block("decoy", 60, 22);
        decoy.actions.push(flat("smite", 60));
        let mut state = caster_with(allies.into_iter().chain(std::iter::once(decoy)).collect());
        state.combatants.iter_mut().find(|c| c.id == "ogre").unwrap().hp = 1000;
        state
    }

    #[test]
    fn the_caster_hastes_the_striker_over_itself_once_the_extra_attack_counts() {
        let state = long_fight(vec![party_block("defender", 5), party_block("striker", 5)]);
        let plan = choose_action(&state, "standard-caster-05");
        assert_eq!((plan.action_id.as_str(), plan.targets.as_slice()), ("haste", ["standard-striker-05".to_string()].as_slice()), "{}", plan.reason);
        assert!(plan.reason.contains("extra action "), "{}", plan.reason);
        assert!(plan.expected_damage > 0.0);
        assert!(!plan.reason.contains("rounds at keep") && !plan.reason.contains("lethargy"), "a safe caster in a long fight: {}", plan.reason);
    }

    #[test]
    fn the_caster_doesnt_haste_against_an_ogre_the_level_11_party_kills_within_a_round() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let ogre: StatBlock = serde_json::from_str(&std::fs::read_to_string(root.join("data/fixtures/ogre.json")).unwrap()).unwrap();
        let mut combatants = vec![fighter("ogre", "monsters", ogre.hit_points.average)];
        let mut blocks = vec![ogre];
        for archetype in crate::party::ARCHETYPES {
            let pc = party_block(archetype, 11);
            combatants.push(fighter(&pc.id, "party", pc.hit_points.average));
            blocks.push(pc);
        }
        let state = encounter(blocks, combatants);
        let constants = PolicyConstants::default();
        let ctx = ctx(&state, &constants);
        let caster = ctx.combatant("standard-caster-11").unwrap();
        assert!(ctx.rounds_left(caster) < 1.0, "{}", ctx.rounds_left(caster));
        let plan = choose_action(&state, "standard-caster-11");
        assert_ne!(plan.action_id, "haste", "{}", plan.reason);
    }

    fn haste_modifiers() -> Vec<TimedModifier> {
        let caster = party_block("caster", 5);
        let haste = caster.actions.iter().find(|a| a.id == "haste").unwrap();
        let Effect::Cast(cast) = &haste.effects[0] else { panic!() };
        engine::bound_cast_effects(&caster, cast)
            .unwrap()
            .0
            .into_iter()
            .map(|e| match e {
                Effect::TimedModifier(m) => m,
                _ => panic!(),
            })
            .collect()
    }

    /// In initiative order: a Con +2 caster (80 HP, AC 12) with a flat zap, a striker whose
    /// +30 sword deals exactly its 2d6 average of 7, a decoy with a flat 10, and a brute with
    /// a +5 2d6 claw. The brute ranks the decoy first unless the caster's zap is above 10.
    fn haste_fight(caster_zap: i32, brute_hp: i32) -> EncounterState {
        let mut caster = block("caster", 80, 12);
        caster.ability_scores.constitution = 14;
        caster.actions.push(flat("zap", caster_zap));
        let mut striker = block("striker", 80, 12);
        striker.actions.push(melee("sword", 30, "2d6"));
        let mut decoy = block("decoy", 80, 12);
        decoy.actions.push(flat("smite", DECOY));
        let mut brute = block("brute", brute_hp, 12);
        brute.actions.push(melee("claw", 5, "2d6"));
        let combatants = vec![fighter("caster", "party", 80), fighter("striker", "party", 80), fighter("decoy", "party", 80), fighter("brute", "monsters", brute_hp)];
        encounter(vec![caster, striker, decoy, brute], combatants)
    }

    fn value_on(state: &EncounterState, bearer: &str, modifiers: &[TimedModifier]) -> BearerValue {
        let constants = PolicyConstants::default();
        let ctx = ctx(state, &constants);
        ctx.bearer_value(&BuffOutlook::new(ctx.combatant("caster").unwrap()), ctx.combatant(bearer).unwrap(), modifiers.iter())
    }

    const SWORD: f64 = 7.0;
    const DECOY: i32 = 10;

    #[test]
    fn a_safe_caster_in_a_long_fight_values_haste_over_the_whole_horizon() {
        let value = value_on(&haste_fight(5, 300), "striker", &haste_modifiers());
        assert!((value.extra_action - SWORD).abs() < 1e-9, "{value:?}");
        assert!((value.damage - SWORD * f64::from(BUFF_HORIZON)).abs() < 1e-9, "{value:?}");
        assert!(value.horizon.is_none() && value.lethargy == 0.0, "{value:?}");
    }

    #[test]
    fn a_fight_with_half_a_round_left_halves_the_extra_action() {
        let state = haste_fight(5, 11);
        assert!((rounds_left(&state, "caster") - 11.0 / (5.0 + SWORD + f64::from(DECOY))).abs() < 1e-9);
        let value = value_on(&state, "striker", &haste_modifiers());
        assert!((value.damage - SWORD * 0.5).abs() < 1e-9, "{value:?}");
        let (rounds, h, k) = value.horizon.unwrap();
        assert!((rounds - 0.5).abs() < 1e-9 && (h - 0.5).abs() < 1e-9 && k == 1.0, "{value:?}");
    }

    #[test]
    fn the_first_round_is_safe_when_the_bearer_acts_before_the_opponents() {
        let mut state = haste_fight(25, 300);
        let mut modifiers = haste_modifiers();
        for m in &mut modifiers {
            m.on_end.clear();
        }
        let k = KEEP_DC_10.powf(CLAW_HITS);
        assert!((keep(&state) - k).abs() < 1e-9);
        let value = value_on(&state, "striker", &modifiers);
        assert!((value.damage - SWORD * (1.0 + k)).abs() < 1e-9, "{value:?}");
        assert_eq!(value.lethargy, 0.0, "no on_end, no lethargy");
        state.initiative_order = ["caster", "brute", "striker", "decoy"].map(String::from).to_vec();
        let value = value_on(&state, "striker", &modifiers);
        assert!((value.damage - SWORD * (k + k * k)).abs() < 1e-9, "{value:?}");
    }

    #[test]
    fn lethargy_costs_the_bearers_threat_times_the_chance_concentration_breaks() {
        let state = haste_fight(25, 300);
        let k = KEEP_DC_10.powf(CLAW_HITS);
        let cost = SWORD * (1.0 - k * k);
        let value = value_on(&state, "striker", &haste_modifiers());
        assert!((value.lethargy - cost).abs() < 1e-9, "{value:?}");
        assert!((value.damage - (SWORD * (1.0 + k) - cost)).abs() < 1e-9, "{value:?}");
        let haste = party_block("caster", 5).actions.into_iter().find(|a| a.id == "haste").unwrap();
        let constants = PolicyConstants::default();
        let ctx = ctx(&state, &constants);
        let caster = ctx.combatant("caster").unwrap();
        let plan = ctx.modifier_plan(caster, ctx.block(caster).unwrap(), &haste, Section::Action, None, &haste_modifiers()).unwrap();
        assert_eq!(plan.targets, ["striker"]);
        assert!(plan.reason.ends_with(&format!(", extra action 7.0, {:.1} of 2.0 rounds at keep {k:.2}, lethargy -{cost:.1})", 1.0 + k)), "{}", plan.reason);
    }

    #[test]
    fn a_shield_like_self_buff_is_cut_to_the_rounds_left_without_keep_or_lethargy() {
        let state = haste_fight(25, 26);
        let shield: TimedModifier = serde_json::from_value(serde_json::json!({"bearer": "self", "change": {"add": 5}, "duration": {"rounds": 1},
            "source_ability": "shield", "target": "armor_class"}))
        .unwrap();
        let h = 26.0 / (25.0 + SWORD + f64::from(DECOY));
        let value = value_on(&state, "caster", &[shield]);
        let (rounds, cut, k) = value.horizon.unwrap();
        assert!((rounds - h).abs() < 1e-9 && (cut - h).abs() < 1e-9 && k == 1.0, "{value:?}");
        assert_eq!(value.lethargy, 0.0);
        let claw = 0.70 * 7.0 + 0.05 * 7.0;
        assert!((value.survival - claw * AC_POINT_VALUE * 5.0 * h).abs() < 1e-9, "{value:?}");
    }

    #[test]
    fn the_caster_doesnt_recast_haste_while_it_holds_one() {
        let mut squire = block("squire", 30, 5);
        squire.actions.clear();
        let mut state = caster_with(vec![squire, party_block("striker", 5)]);
        only_haste(&mut state);
        state.active_modifiers = hasted_by("standard-caster-05", "standard-striker-05");
        assert_eq!(choose_action(&state, "standard-caster-05").targets, ["squire"]);
        state.combatants.iter_mut().find(|c| c.id == "standard-caster-05").unwrap().concentration = Some("haste".into());
        assert!(choose(&state, "standard-caster-05", Section::Action, &PolicyConstants::default()).is_none());
    }

    #[test]
    fn an_extra_action_plan_is_one_allowed_attack_and_needs_an_extra_action() {
        let mut state = caster_with(vec![party_block("defender", 5)]);
        let extra = |state: &EncounterState| choose(state, "standard-defender-05", Section::Extra, &PolicyConstants::default());
        assert!(extra(&state).is_none());
        state.active_modifiers = hasted_by("standard-caster-05", "standard-defender-05");
        assert!(choose_action(&state, "standard-defender-05").action_id == "multiattack");
        let plan = extra(&state).expect("an extra action plan");
        assert_eq!((plan.action_id.as_str(), plan.section), ("longsword", Section::Extra));
        assert!(plan.log_line().starts_with("DECISION standard-defender-05 extra action: longsword -> ogre"), "{}", plan.log_line());
        state.combatants.iter_mut().find(|c| c.id == "standard-defender-05").unwrap().action_economy.extra_action_used = true;
        assert!(extra(&state).is_none());
    }

    #[test]
    fn movement_left_reads_speed_modifiers_on_the_bearer() {
        let mut state = duel(["1d8", "1d8"], [20, 20]);
        let hasted = |bearer: Option<&str>| crate::encounter::ActiveModifier {
            target: ModifierTarget::Speed,
            change: ModifierChange::Double,
            remaining: crate::effect::Duration::Concentration,
            source: Some("b".into()),
            bearer: bearer.map(Into::into),
            source_ability: Some("haste".into()),
            ends_on_damage: false,
            on_end: vec![],
        };
        state.active_modifiers.push(hasted(Some("a")));
        state.combatants[1].action_economy.movement_used_feet = 5;
        let constants = PolicyConstants::default();
        let ctx = ctx(&state, &constants);
        assert_eq!(ctx.movement_left(ctx.combatant("a").unwrap()), 55);
        assert_eq!(ctx.movement_left(ctx.combatant("b").unwrap()), 30);
        assert_eq!(ctx.movement_left(ctx.combatant("a").unwrap()), engine::walk_speed(&state, &state.combatants[1]) - 5);
    }

    fn afflict(state: &mut EncounterState, id: &str, condition: Condition) {
        let c = state.combatants.iter_mut().find(|c| c.id == id).unwrap();
        c.conditions.push(crate::encounter::ConditionInstance { condition, duration: Duration::Concentration, source: None, ends_on_damage: false });
    }

    fn expected_damage(state: &EncounterState, actor: &str, effect: &Effect, target: &str) -> f64 {
        let constants = PolicyConstants::default();
        let ctx = ctx(state, &constants);
        let actor = ctx.combatant(actor).unwrap();
        let block = ctx.block(actor).unwrap();
        ctx.effect_value(actor, block, effect, ctx.combatant(target).unwrap(), 0, Footing::default()).0.damage
    }

    #[test]
    fn two_advantage_sources_and_one_disadvantage_score_a_straight_roll() {
        let mut state = duel(["1d8", "1d8"], [20, 20]);
        afflict(&mut state, "a", Condition::Invisible);
        afflict(&mut state, "a", Condition::Poisoned);
        afflict(&mut state, "ogre", Condition::Restrained);
        let sword = melee("sword", 5, "1d8").effects.remove(0);
        // +5 against AC 12 hits on 7 or more: 0.7 × 4.5 plus a 0.05 critical's extra 4.5.
        assert!((expected_damage(&state, "a", &sword, "ogre") - 3.375).abs() < 1e-9);
    }

    #[test]
    fn a_ranged_plan_against_a_prone_target_scores_disadvantage_and_a_melee_plan_advantage() {
        let mut state = skirmish();
        state.combatants.iter_mut().for_each(|c| c.engagement.clear());
        state.combatants.iter_mut().for_each(|c| c.ranges.values_mut().for_each(|b| *b = RangeBand::Near));
        afflict(&mut state, "brute", Condition::Prone);
        let plan = choose_action(&state, "archer");
        // Disadvantage at +5 against AC 12: 0.7² × 7 plus 0.0025 × 7.
        assert!((plan.expected_damage - 3.4475).abs() < 1e-9, "{}", plan.expected_damage);
        snapshot(&mut state, "archer").actions = vec![melee("knife", 5, "1d8")];
        let plan = choose_action(&state, "archer");
        assert_eq!(plan.movement.len(), 1, "{}", plan.reason);
        // Moving in puts the knife within 5 feet: Advantage, 0.91 × 4.5 plus 0.0975 × 4.5.
        assert!((plan.expected_damage - 4.53375).abs() < 1e-9, "{}", plan.expected_damage);
    }

    #[test]
    fn an_engaged_archer_scores_its_shot_at_disadvantage_and_stays_when_stepping_out_does_not_pay() {
        let plan = choose_action(&skirmish(), "archer");
        assert!(plan.movement.is_empty(), "{}", plan.reason);
        // Disadvantage at +5 against AC 12: 0.7² × 7 plus 0.0025 × 7. Stepping out would score
        // 5.25 less a 3.375 Opportunity Attack.
        assert!((plan.expected_damage - 3.4475).abs() < 1e-9, "{}", plan.expected_damage);
    }

    #[test]
    fn an_archer_engaged_only_with_an_incapacitated_brute_shoots_straight_without_stepping_out() {
        let mut state = skirmish();
        afflict(&mut state, "brute", Condition::Incapacitated);
        let plan = choose_action(&state, "archer");
        assert!(plan.movement.is_empty(), "{}", plan.reason);
        // Straight at +5 against AC 12: 0.7 × 7 plus 0.05 × 7.
        assert!((plan.expected_damage - 5.25).abs() < 1e-9, "{}", plan.expected_damage);
    }

    #[test]
    fn a_shot_reads_close_combat_from_where_the_plan_leaves_the_archer() {
        let shot = |state: &EncounterState, moves: Moves| {
            let constants = PolicyConstants::default();
            let ctx = ctx(state, &constants);
            let archer = ctx.combatant("archer").unwrap();
            let bow = ranged("bow", 5, "2d6").effects.remove(0);
            ctx.effect_value(archer, ctx.block(archer).unwrap(), &bow, ctx.combatant("brute").unwrap(), 0, Footing { moves }).0.damage
        };
        // Straight: 0.7 × 7 plus 0.05 × 7. Disadvantage: 0.7² × 7 plus 0.0025 × 7.
        let (straight, disadvantage) = (5.25, 3.4475);
        let engaged = skirmish();
        assert!((shot(&engaged, Moves::Stays) - disadvantage).abs() < 1e-9);
        assert!((shot(&engaged, Moves::Out) - straight).abs() < 1e-9);
        let mut apart = skirmish();
        apart.combatants.iter_mut().for_each(|c| c.engagement.clear());
        apart.combatants.iter_mut().for_each(|c| c.ranges.values_mut().for_each(|b| *b = RangeBand::Near));
        assert!((shot(&apart, Moves::Stays) - straight).abs() < 1e-9);
        assert!((shot(&apart, Moves::In) - disadvantage).abs() < 1e-9);
    }

    #[test]
    fn a_dexterity_save_against_a_paralyzed_target_is_a_certain_failure() {
        let mut state = duel(["1d8", "1d8"], [20, 20]);
        let fire = breath("dex").effects.remove(0);
        afflict(&mut state, "a", Condition::Paralyzed);
        assert!((expected_damage(&state, "ogre", &fire, "a") - 33.0).abs() < 1e-9);
        let constants = PolicyConstants::default();
        let ctx = ctx(&state, &constants);
        let a = ctx.combatant("a").unwrap();
        assert_eq!(ctx.save_success(a, crate::stat_block::Ability::Dex, 15), 0.0);
        assert_eq!(ctx.save_success(a, crate::stat_block::Ability::Con, 15), 0.3);
    }

    #[test]
    fn a_restrained_target_saves_dexterity_at_disadvantage() {
        let mut state = duel(["1d8", "1d8"], [20, 20]);
        afflict(&mut state, "a", Condition::Restrained);
        let constants = PolicyConstants::default();
        let ctx = ctx(&state, &constants);
        let a = ctx.combatant("a").unwrap();
        assert!((ctx.save_success(a, crate::stat_block::Ability::Dex, 15) - 0.09).abs() < 1e-9);
        assert!((ctx.save_success(a, crate::stat_block::Ability::Str, 15) - 0.3).abs() < 1e-9);
    }

    fn misty_step() -> AbilityEntry {
        entry(serde_json::json!({"id": "misty-step", "name": "Misty Step", "kind": "modeled",
            "effects": [{"teleport": {"range": 30, "destination": "unoccupied space you can see"}}]}))
    }

    fn teleport_value(state: &EncounterState) -> f64 {
        let constants = PolicyConstants::default();
        let ctx = ctx(state, &constants);
        let archer = ctx.combatant("archer").unwrap();
        let teleport = misty_step().effects.remove(0);
        ctx.effect_value(archer, ctx.block(archer).unwrap(), &teleport, ctx.combatant("brute").unwrap(), 0, Footing::default()).0.survival
    }

    #[test]
    fn a_teleport_scores_the_opportunity_attacks_it_avoids() {
        let mut state = skirmish();
        snapshot(&mut state, "archer").actions = vec![misty_step()];
        let avoided = pick(&state, "brute", "archer").value;
        assert!(avoided > 0.0);
        assert_eq!(teleport_value(&state), avoided);
        let plan = choose_action(&state, "archer");
        assert!((plan.survival - avoided).abs() < 1e-9, "{}", plan.reason);
        assert!(plan.reason.contains(&format!("teleports clear of {avoided:.1} opportunity damage")), "{}", plan.reason);
    }

    /// The archer holds Misty Step and is engaged by two brutes.
    fn blinker() -> EncounterState {
        let mut archer = block("archer", 20, 12);
        archer.actions.push(misty_step());
        let mut brute = block("brute", 30, 12);
        brute.actions.push(melee("claw", 5, "1d8"));
        let mut other = block("other", 30, 12);
        other.actions.push(melee("claw", 5, "1d8"));
        let mut state = encounter(
            vec![archer, brute, other],
            vec![fighter("archer", "party", 20), fighter("brute", "monsters", 30), fighter("other", "monsters", 30)],
        );
        engage(&mut state, "archer", "brute");
        engage(&mut state, "archer", "other");
        state
    }

    #[test]
    fn a_teleport_plan_targets_nobody_and_credits_exactly_what_the_engine_frees() {
        let mut state = blinker();
        let both = pick(&state, "brute", "archer").value + pick(&state, "other", "archer").value;
        let plan = choose_action(&state, "archer");
        assert!(plan.targets.is_empty(), "{}", plan.reason);
        assert!(plan.reason.contains(&format!("teleports clear of {both:.1} opportunity damage")), "{}", plan.reason);
        assert!((plan.survival - both).abs() < 1e-9, "{} vs {both}", plan.survival);
        engine::step_events(
            &mut state,
            crate::engine::StepRequest::Action { actor_id: plan.actor_id.clone(), action_id: plan.action_id.clone(), target_ids: plan.targets.clone(), slot_level: None, extra: false },
        )
        .unwrap();
        for id in ["archer", "brute", "other"] {
            assert!(state.combatants.iter().find(|c| c.id == id).unwrap().engagement.is_empty(), "{id} still engaged");
        }
    }

    #[test]
    fn a_cast_of_a_teleport_spell_targets_nobody() {
        let mut state = blinker();
        snapshot(&mut state, "archer").actions = vec![entry(serde_json::json!({"id": "step", "name": "Step", "kind": "modeled",
            "effects": [{"cast": {"spell_id": "misty-step", "slot_level": 2, "upcast": false}}]}))];
        let plan = choose_action(&state, "archer");
        assert!(plan.targets.is_empty(), "{}", plan.reason);
        assert!(plan.reason.starts_with("teleports clear of "), "{}", plan.reason);
    }

    #[test]
    fn a_teleport_of_the_target_scores_nothing() {
        let state = skirmish();
        let constants = PolicyConstants::default();
        let ctx = ctx(&state, &constants);
        let archer = ctx.combatant("archer").unwrap();
        let away = entry(serde_json::json!({"id": "away", "name": "Away", "kind": "modeled",
            "effects": [{"teleport": {"range": 30, "destination": "away", "subject": "target"}}]})).effects.remove(0);
        let value = ctx.effect_value(archer, ctx.block(archer).unwrap(), &away, ctx.combatant("brute").unwrap(), 0, Footing::default()).0;
        assert_eq!(value.survival, 0.0);
    }

    #[test]
    fn a_shot_that_teleports_stays_put_rather_than_pay_the_attacks_its_teleport_avoids() {
        let mut state = skirmish();
        snapshot(&mut state, "archer").actions = vec![entry(serde_json::json!({"id": "blink-shot", "name": "Blink Shot", "kind": "modeled", "effects": [
            {"attack": {"modifier": 5, "range": [80, 320], "target_count": 1, "damage": [{"dice": "4d10", "type": "piercing"}]}},
            {"teleport": {"range": 30, "destination": "unoccupied space you can see"}}]}))];
        let avoided = pick(&state, "brute", "archer").value;
        // Stepping out gains 16.5 - 10.835 = 5.665 of bow damage, more than one Opportunity Attack
        // (3.375) but less than paying it and losing the teleport's credit for avoiding it.
        assert!(5.665 > avoided && 5.665 < 2.0 * avoided, "{avoided}");
        let plan = choose_action(&state, "archer");
        assert!(plan.movement.is_empty(), "{}", plan.reason);
        assert!(plan.reason.contains(&format!("teleports clear of {avoided:.1} opportunity damage")), "{}", plan.reason);
    }

    #[test]
    fn a_teleport_avoids_nothing_the_reactors_would_not_take() {
        let mut state = skirmish();
        snapshot(&mut state, "archer").actions = vec![misty_step(), melee("knife", 5, "6d10")];
        snapshot(&mut state, "brute").reactions.push(ac_reaction("parry", 2, "melee"));
        assert!(reserve(&state, "brute") > pick(&state, "brute", "archer").value, "the brute holds its reaction for Parry");
        assert_eq!(teleport_value(&state), 0.0);
        let mut state = skirmish();
        state.combatants.iter_mut().for_each(|c| c.engagement.clear());
        assert_eq!(teleport_value(&state), 0.0, "nothing to avoid unengaged");
    }

    #[test]
    fn a_teleporting_multiattack_against_a_parrying_reactor_does_not_overflow() {
        let blinker = |id: &str| {
            let mut b = block(id, 40, 14);
            b.actions = vec![
                melee("claw", 5, "2d6"),
                misty_step(),
                entry(serde_json::json!({"id": "multiattack", "name": "Multiattack", "kind": "modeled", "effects": [{"multiattack": {"steps": [
                    {"action": {"action_id": "claw", "count": 2}}, {"action": {"action_id": "misty-step", "count": 1}}]}}]})),
            ];
            b.reactions.push(ac_reaction("parry", 2, "melee"));
            b
        };
        let mut state = encounter(vec![blinker("x"), blinker("y")], vec![fighter("x", "party", 40), fighter("y", "monsters", 40)]);
        engage(&mut state, "x", "y");
        for id in ["x", "y"] {
            assert!(choose(&state, id, Section::Action, &PolicyConstants::default()).is_some(), "{id} found no plan");
        }
        let (answer, _) = decide_reaction(&state, &opportunity_window("y", "x", &["claw"]), &PolicyConstants::default());
        assert!(matches!(answer, ReactionAnswer::Take(_) | ReactionAnswer::Pass));
    }

    fn flat(id: &str, amount: i32) -> AbilityEntry {
        entry(serde_json::json!({"id": id, "name": id, "kind": "modeled", "effects": [{"damage": {"dice": "0d1", "type": "force", "modifier": amount}}]}))
    }

    fn mender(id: &str, amount: i32) -> AbilityEntry {
        entry(serde_json::json!({"id": id, "name": id, "kind": "modeled", "effects": [{"heal": {"modifier": amount}}]}))
    }

    /// Two foes of 30 and 20 HP against allies dealing a flat 10 and 15.
    fn short_fight() -> EncounterState {
        let mut a = block("a", 20, 12);
        a.actions.push(flat("zap", 10));
        let mut b = block("b", 20, 12);
        b.actions.push(flat("zap", 15));
        let mut x = block("x", 30, 12);
        x.actions.push(flat("zap", 1));
        let mut y = block("y", 20, 12);
        y.actions.push(flat("zap", 1));
        encounter(vec![a, b, x, y], vec![fighter("a", "party", 20), fighter("b", "party", 20), fighter("x", "monsters", 30), fighter("y", "monsters", 20)])
    }

    fn rounds_left(state: &EncounterState, actor: &str) -> f64 {
        let constants = PolicyConstants::default();
        let ctx = ctx(state, &constants);
        ctx.rounds_left(ctx.combatant(actor).unwrap())
    }

    #[test]
    fn rounds_left_is_the_opponents_hit_points_over_the_sides_damage() {
        let mut state = short_fight();
        assert!((rounds_left(&state, "a") - (30.0 + 20.0) / (10.0 + 15.0)).abs() < 1e-9);
        snapshot(&mut state, "a").actions.push(mender("mend", 40));
        assert!((rounds_left(&state, "a") - 2.0).abs() < 1e-9, "healing doesn't shorten the fight");
        for id in ["a", "b"] {
            snapshot(&mut state, id).actions = vec![mender("mend", 40)];
        }
        assert_eq!(rounds_left(&state, "a"), f64::from(BUFF_HORIZON));
    }

    fn acts_before(state: &EncounterState, actor: &str, bearer: &str) -> bool {
        let constants = PolicyConstants::default();
        let ctx = ctx(state, &constants);
        ctx.acts_before_opponents(ctx.combatant(actor).unwrap(), ctx.combatant(bearer).unwrap())
    }

    #[test]
    fn a_bearer_acts_first_when_its_turn_comes_before_any_able_opponents() {
        let mut state = short_fight();
        let order = |ids: &[&str]| ids.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        state.initiative_order = order(&["a", "b", "x", "y"]);
        assert!(acts_before(&state, "a", "b"));
        assert!(!acts_before(&state, "a", "a"), "the actor's own buff");
        state.initiative_order = order(&["b", "y", "a", "x"]);
        assert!(!acts_before(&state, "a", "b"), "x comes up first, wrapping");
        afflict(&mut state, "x", Condition::Stunned);
        assert!(acts_before(&state, "a", "b"), "a stunned foe is skipped");
        state.initiative_order = order(&["b", "x", "y"]);
        assert!(!acts_before(&state, "a", "b"), "an actor out of the order");
    }

    /// A Con +2 caster with 80 HP and AC 12, an ally and a brute with a +5 2d6 claw. The brute
    /// ranks whichever of the two deals more first.
    fn vigil(caster_first: bool) -> EncounterState {
        let mut caster = block("caster", 80, 12);
        caster.ability_scores.constitution = 14;
        caster.actions.push(flat("zap", if caster_first { 20 } else { 10 }));
        let mut ally = block("ally", 80, 12);
        ally.actions.push(flat("zap", 15));
        let mut brute = block("brute", 300, 12);
        brute.actions.push(melee("claw", 5, "2d6"));
        encounter(vec![caster, ally, brute], vec![fighter("caster", "party", 80), fighter("ally", "party", 80), fighter("brute", "monsters", 300)])
    }

    fn keep(state: &EncounterState) -> f64 {
        let constants = PolicyConstants::default();
        let ctx = ctx(state, &constants);
        ctx.concentration_keep(ctx.combatant("caster").unwrap())
    }

    fn blast(id: &str, amount: i32, area: bool) -> AbilityEntry {
        let save = serde_json::json!({"saving_throw": {"ability": "dex", "dc": 30, "damage": [{"dice": "0d1", "type": "fire", "modifier": amount}],
            "half_on_success": false, "no_damage_on_success": false}});
        let effect = if area { serde_json::json!({"area": {"area": {"cone": {"size": 60}}, "origin": "self_", "target_count": 4, "effect": save}}) } else { save };
        entry(serde_json::json!({"id": id, "name": id, "kind": "modeled", "effects": [effect]}))
    }

    /// Con +2 against DC 10: 13 of 20.
    const KEEP_DC_10: f64 = 0.65;
    /// The claw: 0.70 to hit plus 0.05 crit dice, over an average hit of 7.
    const CLAW_HITS: f64 = (0.70 * 7.0 + 0.05 * 7.0) / 7.0;

    #[test]
    fn an_opponent_aiming_elsewhere_without_saves_leaves_concentration_safe() {
        assert_eq!(keep(&vigil(false)), 1.0);
    }

    #[test]
    fn each_expected_hit_on_the_caster_is_a_dc_10_constitution_save() {
        let k = keep(&vigil(true));
        assert!((k - KEEP_DC_10.powf(CLAW_HITS)).abs() < 1e-9, "{k}");
    }

    #[test]
    fn save_damage_sets_the_dc_at_half_and_can_make_the_save_hopeless() {
        let mut state = vigil(true);
        snapshot(&mut state, "brute").actions = vec![blast("blast", 53, false)];
        assert_eq!(keep(&state), 0.0, "DC 26 against Con +2");
        snapshot(&mut state, "brute").actions = vec![blast("blast", 30, false)];
        let k = keep(&state);
        assert!((k - 8.0 / 20.0).abs() < 1e-9, "DC 15 against Con +2: {k}");
    }

    #[test]
    fn counted_damage_that_would_drop_the_caster_breaks_concentration() {
        let mut state = vigil(true);
        snapshot(&mut state, "brute").actions = vec![blast("blast", 20, false)];
        assert!(keep(&state) > 0.0);
        state.combatants.iter_mut().find(|c| c.id == "caster").unwrap().hp = 20;
        assert_eq!(keep(&state), 0.0);
    }

    #[test]
    fn an_opponent_picks_its_action_by_the_damage_counted_on_the_caster() {
        let mut state = vigil(false);
        let multiattack = entry(serde_json::json!({"id": "multiattack", "name": "Multiattack", "kind": "modeled",
            "effects": [{"multiattack": {"steps": [{"action": {"action_id": "claw", "count": 20}}]}}]}));
        snapshot(&mut state, "brute").actions = vec![multiattack, melee("claw", 5, "2d6"), blast("breath", 53, true)];
        assert_eq!(keep(&state), 0.0, "the breath catches the caster at DC 26 though the brute ranks the ally first");
    }

    #[test]
    fn a_single_target_save_counts_only_from_an_opponent_aiming_at_the_caster() {
        let mut state = vigil(false);
        snapshot(&mut state, "brute").actions = vec![blast("bolt", 30, false)];
        assert_eq!(keep(&state), 1.0);
        snapshot(&mut state, "caster").actions = vec![flat("zap", 20)];
        assert!((keep(&state) - 8.0 / 20.0).abs() < 1e-9, "DC 15 against Con +2");
    }

    #[test]
    fn advantage_on_constitution_saves_squares_the_failure_chance() {
        let mut state = vigil(true);
        state.active_modifiers.push(crate::encounter::ActiveModifier {
            target: ModifierTarget::SavingThrows { ability: Some(crate::stat_block::Ability::Con) },
            change: ModifierChange::Advantage,
            remaining: Duration::Rounds(10),
            source: Some("caster".into()),
            bearer: Some("caster".into()),
            source_ability: Some("ward".into()),
            ends_on_damage: false,
            on_end: vec![],
        });
        let k = keep(&state);
        let fail = 1.0 - KEEP_DC_10;
        assert!((k - (1.0 - fail * fail).powf(CLAW_HITS)).abs() < 1e-9, "{k}");
    }
}
