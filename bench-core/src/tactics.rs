//! Deterministic batch tactics policy (ADR 0012).
//!
//! One policy plays both sides of a batch fight. It enumerates legal
//! move-plus-action plans for the acting combatant, scores each by expected
//! damage, control and survival, and returns the best one with a log line
//! that states its reason, target, resource use, movement and area
//! assumptions. Scoring walks stat-block and combatant order only, so the
//! same state always yields the same plan.

use serde::{Deserialize, Serialize};

use crate::effect::{Area, Effect, MultiattackStep, Usage};
use crate::encounter::{Combatant, EncounterState, EndState, RangeBand};
use crate::engine;
use crate::stat_block::{AbilityEntry, Condition, StatBlock};

/// Stable policy identifier written into logs, reports and fingerprints.
pub const POLICY_ID: &str = "homebrew-bench.tactics";
/// Policy version. Bump it whenever scoring changes, so baselines go stale.
pub const POLICY_VERSION: u32 = 2;

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
    let abilities: &[AbilityEntry] = match section {
        Section::Action => &block.actions,
        Section::Bonus => &block.bonus_actions,
        Section::Legendary => &block.legendary_actions,
    };
    let mut candidates: Vec<Plan> = Vec::new();
    for ability in abilities {
        if ability.effects.is_empty() || !engine::ability_ready(state, actor_id, ability) {
            continue;
        }
        if is_consumable(block, ability) && !ctx.consumable_allowed(actor, block) {
            continue;
        }
        if let Some(plan) = ctx.plan_for(actor, block, ability, section) {
            candidates.push(plan);
        }
    }
    pick_best(abilities, candidates)
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

/// Highest score wins; ties keep stat-block order. A limited-use plan must beat
/// the best at-will plan, and healing a living ally must beat the best attack
/// or control plan. Revival is exempt and competes on score alone.
fn pick_best(abilities: &[AbilityEntry], candidates: Vec<Plan>) -> Option<Plan> {
    let limited = |id: &str| abilities.iter().find(|a| a.id == id).is_some_and(is_limited);
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

/// Open `threat` calls past which a control effect is valued at zero.
const MAX_THREAT_NESTING: u8 = 2;

#[derive(Debug, Clone, Copy, Default)]
struct Value {
    damage: f64,
    control: f64,
    heal: f64,
}

impl Value {
    fn add(&mut self, other: Value) {
        self.damage += other.damage;
        self.control += other.control;
        self.heal += other.heal;
    }

    fn scaled(self, factor: f64) -> Value {
        Value { damage: self.damage * factor, control: self.control * factor, heal: self.heal * factor }
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
        let speed = self.block(c).and_then(|b| b.speeds.get("walk")).map_or(0, |s| s.feet);
        let restrained = c.conditions.iter().any(|x| matches!(x.condition, Condition::Grappled | Condition::Restrained));
        if restrained {
            return 0;
        }
        speed.saturating_sub(c.action_economy.movement_used_feet)
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
        let open = self.threat_nesting.get();
        self.threat_nesting.set(open + 1);
        let value = self.threat_unnested(c);
        self.threat_nesting.set(open);
        value
    }

    fn threat_unnested(&self, c: &Combatant) -> f64 {
        if !can_act(c) {
            return 0.0;
        }
        let Some(block) = self.block(c) else { return 0.0 };
        let foes: Vec<&Combatant> = self.opponents(c).filter(|o| o.end_state == EndState::Active).collect();
        if foes.is_empty() {
            return 0.0;
        }
        block
            .actions
            .iter()
            .filter(|a| !a.effects.is_empty() && engine::ability_ready(self.state, &c.id, a))
            .map(|a| {
                let total: f64 = foes
                    .iter()
                    .map(|f| {
                        let v = self.ability_value(c, block, a, f, 0, false).0;
                        v.damage + v.heal
                    })
                    .sum();
                total / foes.len() as f64
            })
            .fold(0.0, f64::max)
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

    /// Casting a concentration spell would end one whose conditions still hold an opponent.
    fn breaks_own_concentration(&self, actor: &Combatant, block: &StatBlock, ability: &AbilityEntry) -> bool {
        let concentration = ability.effects.iter().any(|e| match e {
            Effect::Cast(c) => engine::bound_cast_effects(block, c).is_some_and(|(_, conc)| conc),
            _ => false,
        });
        concentration
            && actor.concentration.is_some()
            && self.opponents(actor).any(|o| o.conditions.iter().any(|c| c.source.as_deref() == Some(actor.id.as_str())))
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
        let resource = usage_word(ability);
        if self.breaks_own_concentration(actor, block, ability) {
            return None;
        }
        if self.heals(block, &ability.effects) {
            return self.heal_plan(actor, block, ability, section, resource);
        }
        if let Some((area, effect)) = area_of(block, &ability.effects) {
            return self.area_plan(actor, block, ability, section, resource, &area, &effect, false);
        }
        if let Some((area, effect)) = prose_area(ability) {
            return self.area_plan(actor, block, ability, section, resource, &area, &effect, true);
        }
        let melee = needs_engaged(block, &ability.effects, 0);
        let ranged = has_ranged_attack(block, &ability.effects, 0);
        let immediate = |t: &Combatant| self.ability_value(actor, block, ability, t, 0, false).0.damage;
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
            let (mut value, mut picks) = self.ability_value(actor, block, ability, target, 0, !engaged_foes.is_empty() && ranged);
            if ranged && !melee && !engaged_foes.is_empty() && section != Section::Legendary {
                let cost = 5 * engaged_foes.len() as u32;
                if self.movement_left(actor) >= cost {
                    let (stepped, stepped_picks) = self.ability_value(actor, block, ability, target, 0, false);
                    let oa: f64 = engaged_foes
                        .iter()
                        .filter(|o| !o.action_economy.reaction_used)
                        .map(|o| self.best_single_attack(o, actor))
                        .sum();
                    if stepped.damage + stepped.control - oa > value.damage + value.control {
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
            let survival = kill - penalty;
            let w = &self.constants.weights;
            let cost = if is_limited(ability) { 1.0 } else { 0.0 };
            let score = w.expected_damage * value.damage + w.control * value.control + w.survival * survival - w.resource_cost * cost;
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
                Effect::Heal(h) => dice_average(&h.dice) + f64::from(h.modifier),
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
            let mut v = self.effect_value(actor, block, effect, t, 0, false).0;
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
            - w.resource_cost * if is_limited(ability) { 1.0 } else { 0.0 };
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

    fn best_single_attack(&self, attacker: &Combatant, target: &Combatant) -> f64 {
        let Some(block) = self.block(attacker) else { return 0.0 };
        block
            .actions
            .iter()
            .flat_map(|a| a.effects.iter())
            .filter_map(|e| match e {
                Effect::Attack(_) => Some(self.effect_value(attacker, block, e, target, 0, false).0.damage),
                _ => None,
            })
            .fold(0.0, f64::max)
    }

    fn ability_value(&self, actor: &Combatant, block: &StatBlock, ability: &AbilityEntry, target: &Combatant, depth: u8, ranged_in_melee: bool) -> (Value, Vec<Vec<usize>>) {
        let mut total = Value::default();
        let mut picks = Vec::new();
        for effect in &ability.effects {
            let (v, p) = self.effect_value(actor, block, effect, target, depth, ranged_in_melee);
            total.add(v);
            picks.extend(p);
        }
        (total, picks)
    }

    fn action_value(&self, actor: &Combatant, block: &StatBlock, action_id: &str, target: &Combatant, depth: u8, ranged_in_melee: bool) -> Value {
        if depth > 4 {
            return Value::default();
        }
        block
            .actions
            .iter()
            .find(|a| a.id == action_id)
            .map(|a| self.ability_value(actor, block, a, target, depth + 1, ranged_in_melee).0)
            .unwrap_or_default()
    }

    fn effect_value(&self, actor: &Combatant, block: &StatBlock, effect: &Effect, target: &Combatant, depth: u8, ranged_in_melee: bool) -> (Value, Vec<Vec<usize>>) {
        let Some(target_block) = self.block(target) else { return (Value::default(), vec![]) };
        let value = match effect {
            Effect::Attack(attack) => {
                let ac = target_block.armor_class.first().map_or(10, |a| a.value);
                let mut disposition = self.disposition(actor, target);
                if ranged_in_melee && attack.range.is_some() && attack.reach.is_none() {
                    disposition -= 1;
                }
                let base = (f64::from(21 - (ac - attack.modifier)) / 20.0).clamp(0.05, 0.95);
                let (p_hit, p_crit) = match disposition.signum() {
                    1 => (1.0 - (1.0 - base).powi(2), 1.0 - 0.95f64.powi(2)),
                    -1 => (base * base, 0.0025),
                    _ => (base, 0.05),
                };
                let mut dmg = 0.0;
                let mut crit_dice = 0.0;
                for d in attack.damage.iter().filter(|d| damage_applies(target, &actor.id, d)) {
                    dmg += adjusted(target_block, d.damage_type, dice_average(&d.dice) + f64::from(d.modifier));
                    crit_dice += adjusted(target_block, d.damage_type, dice_average(&d.dice));
                }
                let mut v = Value { damage: p_hit * dmg + p_crit * crit_dice, ..Default::default() };
                for on_hit in &attack.on_hit {
                    v.add(self.effect_value(actor, block, on_hit, target, depth + 1, ranged_in_melee).0.scaled(p_hit));
                }
                v
            }
            Effect::SavingThrow(save) => {
                let modifier = target_block
                    .saving_throws
                    .get(&save.ability)
                    .copied()
                    .unwrap_or_else(|| target_block.ability_scores.modifier(save.ability));
                let p_success = (f64::from(21 - (save.dc - modifier)) / 20.0).clamp(0.0, 1.0);
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
                    v.add(self.effect_value(actor, block, fail, target, depth + 1, ranged_in_melee).0.scaled(p_fail));
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
            Effect::Heal(h) => Value { heal: dice_average(&h.dice) + f64::from(h.modifier), ..Default::default() },
            Effect::MoveThenAttack(m) => self.action_value(actor, block, &m.attack_id, target, depth, ranged_in_melee),
            Effect::UseAction(u) => self.action_value(actor, block, &u.action_id, target, depth, ranged_in_melee),
            Effect::Cast(cast) => match engine::bound_cast_effects(block, cast) {
                Some((effects, _)) if depth < 4 => effects.iter().fold(Value::default(), |mut acc, e| {
                    acc.add(self.effect_value(actor, block, e, target, depth + 1, ranged_in_melee).0);
                    acc
                }),
                _ => Value::default(),
            },
            Effect::Area(area) => self.effect_value(actor, block, &area.effect, target, depth + 1, ranged_in_melee).0,
            Effect::Multiattack(multi) => return self.multiattack_value(actor, block, &multi.steps, target, depth, ranged_in_melee),
            Effect::TempHp(_) | Effect::TimedModifier(_) | Effect::RollModifier(_) | Effect::ForcedMove(_) | Effect::Teleport(_) => Value::default(),
        };
        (value, vec![])
    }

    /// Value of a multiattack and the picks that reach it: the best `pick`
    /// options of each `Choice` and the best set of each `Alternatives`.
    fn multiattack_value(&self, actor: &Combatant, block: &StatBlock, steps: &[MultiattackStep], target: &Combatant, depth: u8, ranged_in_melee: bool) -> (Value, Vec<Vec<usize>>) {
        let mut total = Value::default();
        let mut picks = Vec::new();
        let slot = |action_id: &str, cast: Option<&crate::effect::Cast>| match cast {
            Some(c) => self.effect_value(actor, block, &Effect::Cast(c.clone()), target, depth + 1, ranged_in_melee).0,
            None => self.action_value(actor, block, action_id, target, depth, ranged_in_melee),
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
                            (v.damage + v.control, i, v)
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
                        let (v, p) = self.multiattack_value(actor, block, set, target, depth, ranged_in_melee);
                        let s = v.damage + v.control;
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

    /// +1 advantage, -1 disadvantage, 0 straight, from conditions alone.
    fn disposition(&self, actor: &Combatant, target: &Combatant) -> i32 {
        let has = |c: &Combatant, list: &[Condition]| c.conditions.iter().any(|x| list.contains(&x.condition));
        let mut d = 0;
        if has(actor, &[Condition::Blinded, Condition::Frightened, Condition::Poisoned, Condition::Prone]) {
            d -= 1;
        }
        if has(target, &[Condition::Paralyzed, Condition::Prone, Condition::Restrained, Condition::Stunned, Condition::Unconscious]) {
            d += 1;
        }
        if has(target, &[Condition::Invisible]) {
            d -= 1;
        }
        if has(actor, &[Condition::Invisible]) {
            d += 1;
        }
        d.clamp(-1, 1)
    }
}

fn can_act(c: &Combatant) -> bool {
    c.end_state == EndState::Active
        && !c.conditions.iter().any(|x| {
            matches!(
                x.condition,
                Condition::Incapacitated | Condition::Paralyzed | Condition::Petrified | Condition::Stunned | Condition::Unconscious
            )
        })
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

fn is_limited(ability: &AbilityEntry) -> bool {
    let leveled_cast = ability.effects.iter().any(|e| matches!(e, Effect::Cast(c) if c.slot_level.unwrap_or(0) > 0));
    leveled_cast || !matches!(ability.usage, None | Some(Usage::AtWill))
}

fn usage_word(ability: &AbilityEntry) -> Option<String> {
    let slot = ability.effects.iter().find_map(|e| match e {
        Effect::Cast(c) => c.slot_level.filter(|l| *l > 0).map(|l| format!("level {l} slot")),
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
    }
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
    fn bare_heal_is_self_only_and_never_revives() {
        let mut state = duel(["1d8", "1d8"], [30, 30]);
        let second_wind = AbilityEntry {
            effects: vec![Effect::Heal(crate::effect::Heal { dice: "1d10".into(), modifier: 1 })],
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
}
