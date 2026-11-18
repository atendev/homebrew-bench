//! Spell lift: the spell value metric (ADR 0020).
//!
//! A tested spell goes onto the caster role in place of that role's spells of the spell's level, and
//! the party fights the spell panel (`spell_panel`) against a control party without them. Each panel
//! fight is paired with a control fight on the same seed (ADR 0007), so the lift is the mean of
//! per-fight differences in winning and in losing a party member. Rows follow the spell: one
//! headline row, one per upcast level, or four cantrip rows.

use serde::{Deserialize, Serialize};

use crate::baseline;
use crate::batch::{self, BatchError, BatchSpec, CastTally, FightResult, Outcome, Party};
use crate::effect::{Cast, Effect, Lighting, ReactionTrigger, SpellCaster, Usage};
use crate::encounter::StandingAssumption;
use crate::spell::{CastingTime, SpellBook, SpellEffect};
use crate::spell_panel::{self, Member, PanelLevel, Shape, SpellPanel};
use crate::stat_block::{AbilityEntry, AbilityKind, StatBlock};

/// Party levels a cantrip gets rows at.
pub const CANTRIP_LEVELS: [u8; 4] = [1, 5, 11, 17];
/// Standard errors either side of a mean that the interval spans (95%).
pub const INTERVAL_Z: f64 = 1.96;
/// How the report states the interval, for ticket-level readers and the JSON.
pub const INTERVAL_METHOD: &str =
    "the mean lift plus and minus 1.96 standard errors of the per-fight paired differences, over every panel fight in the row";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowKind {
    /// The spell cast at its own level, at party level 2L-1.
    Headline,
    /// The spell cast at a level above its own, at party level 2N-1.
    Upcast,
    /// A cantrip at party level 1, 5, 11 or 17.
    Cantrip,
}

/// Where one row sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RowPlan {
    pub kind: RowKind,
    pub party_level: u8,
    /// The slot level of the cast; `None` for a cantrip.
    pub cast_level: Option<u8>,
    /// The spell level the row's control party lacks.
    pub control_spell_level: u8,
}

/// Whether a slot above the spell's level changes its effect or its clause text.
pub fn has_upcast_rule(spell: &SpellEffect) -> bool {
    !spell.scaling.is_empty() || spell.upcast_clause.is_some()
}

/// The rows a spell gets, in party level order.
pub fn plan_rows(spell: &SpellEffect) -> Vec<RowPlan> {
    if spell.level == 0 {
        return CANTRIP_LEVELS.iter().map(|&party_level| RowPlan { kind: RowKind::Cantrip, party_level, cast_level: None, control_spell_level: 0 }).collect();
    }
    let top = if has_upcast_rule(spell) { 9 } else { spell.level };
    (spell.level..=top.min(9))
        .map(|n| RowPlan {
            kind: if n == spell.level { RowKind::Headline } else { RowKind::Upcast },
            party_level: 2 * n - 1,
            cast_level: Some(n),
            control_spell_level: n,
        })
        .collect()
}

/// `block` with every ability that casts the spell dropped and one that casts it at `cast_level`
/// added to the section its casting time names. A `long` spell leaves the block as it was.
pub fn with_spell(block: &StatBlock, spell: &SpellEffect, cast_level: Option<u8>) -> StatBlock {
    let mut out = block.clone();
    for list in [&mut out.traits, &mut out.actions, &mut out.bonus_actions, &mut out.reactions, &mut out.legendary_actions] {
        list.retain(|entry| !crate::stat_block::casts_of(entry).iter().any(|c| c.spell_id == spell.id));
    }
    let counters = spell.effects.iter().any(|e| matches!(e, Effect::CounterSpell(_)));
    let entry = AbilityEntry {
        id: spell.id.clone(),
        name: spell.name.clone(),
        desc: Some(format!("Cast {}.", spell.name)),
        kind: if spell.agent_clauses.is_empty() { AbilityKind::Modeled } else { AbilityKind::PartlyModeled },
        effects: vec![Effect::Cast(Cast { spell_id: spell.id.clone(), slot_level: cast_level, upcast: cast_level.is_some_and(|l| l > spell.level) })],
        agent_clauses: spell.agent_clauses.clone(),
        default_resolution: None,
        usage: (spell.level == 0).then_some(Usage::AtWill),
        trigger: (spell.casting_time == CastingTime::Reaction).then(|| {
            if counters {
                ReactionTrigger::SpellCast { caster: SpellCaster::Enemy }
            } else {
                ReactionTrigger::HitByAttackRoll { attack: crate::effect::AttackFilter::Any }
            }
        }),
    };
    match spell.casting_time {
        CastingTime::Action => out.actions.push(entry),
        CastingTime::BonusAction => out.bonus_actions.push(entry),
        CastingTime::Reaction => out.reactions.push(entry),
        CastingTime::Long => {}
    }
    out
}

/// `control` with the tested spell on its caster role at every level.
pub fn swap_party(control: &Party, spell: &SpellEffect, cast_level: Option<u8>) -> Party {
    control
        .iter()
        .map(|(&level, members)| {
            let members = members.iter().map(|m| if spell_panel::is_caster(m) { with_spell(m, spell, cast_level) } else { m.clone() });
            (level, members.collect())
        })
        .collect()
}

/// Why a row can't be measured, when it can't.
pub fn unmeasured_reason(spell: &SpellEffect, plan: &RowPlan, panel: &PanelLevel) -> Option<String> {
    if spell.casting_time == CastingTime::Long {
        return Some("its casting time is a minute or more, so batch can't cast it during a fight".into());
    }
    if plan.kind == RowKind::Upcast && !spell.is_homebrew() {
        if let Some(clause) = &spell.upcast_clause {
            return Some(format!("the engine does not apply its upcast clause: {}", clause.trim_end_matches('.')));
        }
    }
    let counters = spell.effects.iter().any(|e| matches!(e, Effect::CounterSpell(_)));
    if counters && panel.spellcasting_fixture.is_none() {
        return Some(format!("a counter spell needs a spellcaster to stop, and the panel at party level {} has none", plan.party_level));
    }
    None
}

/// What one fight changed, for pairing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct FightRecord {
    pub index: u64,
    /// The fight's own seed; the control and spell fight of one index share it.
    pub seed: u64,
    pub win: bool,
    /// At least one party member died.
    pub casualty: bool,
    pub casts: u32,
    pub damage: i32,
    pub healing: i32,
}

fn record(result: &FightResult, spell_id: &str) -> FightRecord {
    let tally = result.party_casts.get(spell_id).copied().unwrap_or(CastTally::default());
    FightRecord {
        index: result.index,
        seed: result.seed,
        win: result.outcome == Outcome::PartyWin,
        casualty: result.pc_deaths > 0,
        casts: tally.casts,
        damage: tally.damage,
        healing: tally.healing,
    }
}

/// A mean difference with its 95% interval.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Estimate {
    pub mean: f64,
    pub low: f64,
    pub high: f64,
    pub fights: u32,
}

fn round4(x: f64) -> f64 {
    (x * 10_000.0).round() / 10_000.0
}

/// Mean of `diffs` and the interval `INTERVAL_METHOD` names.
pub fn estimate(diffs: &[f64]) -> Option<Estimate> {
    let n = diffs.len();
    if n == 0 {
        return None;
    }
    let mean = diffs.iter().sum::<f64>() / n as f64;
    let variance = if n > 1 { diffs.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / (n - 1) as f64 } else { 0.0 };
    let half = INTERVAL_Z * (variance / n as f64).sqrt();
    Some(Estimate { mean: round4(mean), low: round4(mean - half), high: round4(mean + half), fights: n as u32 })
}

/// One panel member's paired fights in one row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberLift {
    pub id: String,
    pub shape: Shape,
    pub copies: u32,
    pub control_win_rate: f64,
    pub spell_win_rate: f64,
    pub win_lift: f64,
    pub control_casualty_rate: f64,
    pub spell_casualty_rate: f64,
    /// Spell minus control; below zero is fewer fights with a death.
    pub casualty_change: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowStatus {
    Measured,
    /// The policy never cast the spell in any fight of the row.
    NeverCast,
    NotMeasured,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    #[serde(flatten)]
    pub plan: RowPlan,
    pub status: RowStatus,
    /// Why the row is not measured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The control party's win rate over the row's panel fights.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub control_win_rate: Option<f64>,
    /// The control's win rate sits in the panel's 40 to 80% band. Only the headline control is selected into it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub control_in_band: Option<bool>,
    /// Change in win rate with the spell, over every panel fight.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub win_lift: Option<Estimate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single_lift: Option<Estimate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_lift: Option<Estimate>,
    /// Change in the rate of fights with a party death; below zero is better.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub casualty_change: Option<Estimate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub casts: Option<u32>,
    /// Diagnostic: damage dealt by the casts, per cast.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub damage_per_cast: Option<f64>,
    /// Diagnostic: hit points restored by the casts, per cast.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub healing_per_cast: Option<f64>,
    #[serde(default)]
    pub members: Vec<MemberLift>,
    #[serde(default)]
    pub notes: Vec<String>,
}

/// A fight the spell turned from a loss into a win, with its log.
#[derive(Debug, Clone, Serialize)]
pub struct Sample {
    pub party_level: u8,
    pub member: String,
    pub shape: Shape,
    pub index: u64,
    pub seed: u64,
    pub file: String,
    pub replay: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiftReport {
    pub spell: String,
    pub spell_name: String,
    pub spell_level: u8,
    pub homebrew: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    pub party: String,
    pub engine_version: String,
    pub panel_fingerprint: String,
    pub fights_per_member: u32,
    pub max_rounds: u32,
    pub interval: &'static str,
    pub standing_assumptions: Vec<StandingAssumption>,
    pub rows: Vec<Row>,
    pub warnings: Vec<String>,
    pub samples: Vec<Sample>,
    #[serde(skip)]
    pub logs: Vec<(String, String)>,
}

/// Everything one lift run needs.
pub struct LiftSpec<'a> {
    pub spell_id: String,
    /// Homebrew entries from `--spell`; the tested spell may be one of them.
    pub homebrew: Vec<SpellEffect>,
    pub party_name: String,
    pub party: &'a Party,
    pub panel: &'a SpellPanel,
    /// Fights per panel member in every row.
    pub fights: u32,
    pub max_rounds: u32,
    pub assumptions: Vec<StandingAssumption>,
    pub lighting: Lighting,
    pub threads: usize,
    /// The arguments the run was named by, so each sample's replay command reruns it.
    pub invocation: Option<batch::Invocation>,
}

impl LiftSpec<'_> {
    fn spell(&self) -> Result<SpellEffect, BatchError> {
        let book = SpellBook::new(&self.homebrew).map_err(|e| BatchError::Invalid(e.into()))?;
        book.get(&self.spell_id)
            .cloned()
            .ok_or_else(|| BatchError::NotFound(format!("`{}` is neither an SRD spell in the effects file nor a loaded `--spell` file", self.spell_id)))
    }

    fn fixture(member: &Member) -> Result<StatBlock, BatchError> {
        let text = crate::embedded::fixture_json(&member.id).ok_or_else(|| BatchError::NotFound(format!("panel fixture `{}` is not embedded", member.id)))?;
        serde_json::from_str(text).map_err(|e| BatchError::Invalid(format!("embedded fixture {}: {e}", member.id)))
    }

    /// One member's fights in one row: the control party, or with the tested spell swapped on.
    fn batch_spec(&self, plan: &RowPlan, member: &Member, shape: Shape, spell: &SpellEffect, swapped: bool) -> Result<BatchSpec, BatchError> {
        let fixture = Self::fixture(member)?;
        let book = SpellBook::new(&self.homebrew).map_err(|e| BatchError::Invalid(e.into()))?;
        let one = Party::from([(
            plan.party_level,
            self.party.get(&plan.party_level).ok_or_else(|| BatchError::NotFound(format!("party `{}` has no stat blocks for level {}", self.party_name, plan.party_level)))?.clone(),
        )]);
        let control = spell_panel::control_party(&one, plan.control_spell_level, &book);
        let (party, homebrew_spells) = if swapped { (swap_party(&control, spell, plan.cast_level), self.homebrew.clone()) } else { (control, vec![]) };
        let (_, mut assumptions) = baseline::ignoring_unresolved(&fixture);
        assumptions.extend(self.assumptions.iter().filter(|a| swapped || !a.path.starts_with(batch::SPELL_PATH)).cloned());
        Ok(BatchSpec {
            monster: fixture,
            monster_copies: member.copies,
            party_name: self.party_name.clone(),
            party,
            levels: vec![plan.party_level],
            fights: self.fights,
            max_rounds: self.max_rounds,
            master_seed: member.seed,
            assumptions,
            lighting: self.lighting,
            baseline: None,
            invocation: None,
            homebrew_spells,
        })
    }
}

/// One member's fights with the control party (`swapped` false) or with the spell on the caster.
pub fn member_fights(spec: &LiftSpec, plan: &RowPlan, member: &Member, shape: Shape, swapped: bool) -> Result<Vec<FightRecord>, BatchError> {
    let spell = spec.spell()?;
    let run = spec.batch_spec(plan, member, shape, &spell, swapped)?;
    Ok(batch::run_fights(&run)?.iter().map(|r| record(r, &spell.id)).collect())
}

/// Replays one spell fight of a row by its global index, with its full log.
pub fn replay_fight(spec: &LiftSpec, party_level: u8, member_id: &str, shape: Shape, index: u64) -> Result<FightResult, BatchError> {
    let spell = spec.spell()?;
    let plan = plan_rows(&spell).into_iter().find(|p| p.party_level == party_level).ok_or_else(|| BatchError::NotFound(format!("`{}` has no row at party level {party_level}", spell.id)))?;
    let level = spec.panel.level(party_level).ok_or_else(|| BatchError::NotFound(format!("the spell panel has no party level {party_level}")))?;
    let pool = if shape == Shape::Single { &level.single } else { &level.group };
    let member = pool.iter().find(|m| m.id == member_id).ok_or_else(|| BatchError::NotFound(format!("the panel at party level {party_level} has no {} member `{member_id}`", shape.word())))?;
    let run = spec.batch_spec(&plan, member, shape, &spell, true)?;
    batch::replay_fight(&run, index)
}

fn rate(count: usize, of: usize) -> f64 {
    round4(count as f64 / of.max(1) as f64)
}

fn member_lift(member: &Member, shape: Shape, control: &[FightRecord], spell: &[FightRecord]) -> MemberLift {
    let (cw, sw) = (control.iter().filter(|f| f.win).count(), spell.iter().filter(|f| f.win).count());
    let (cc, sc) = (control.iter().filter(|f| f.casualty).count(), spell.iter().filter(|f| f.casualty).count());
    let n = control.len();
    MemberLift {
        id: member.id.clone(),
        shape,
        copies: member.copies,
        control_win_rate: rate(cw, n),
        spell_win_rate: rate(sw, n),
        win_lift: round4(rate(sw, n) - rate(cw, n)),
        control_casualty_rate: rate(cc, n),
        spell_casualty_rate: rate(sc, n),
        casualty_change: round4(rate(sc, n) - rate(cc, n)),
    }
}

struct Paired {
    member: Member,
    shape: Shape,
    control: Vec<FightRecord>,
    spell: Vec<FightRecord>,
}

fn as_number(flag: bool) -> f64 {
    f64::from(u8::from(flag))
}

/// Pools paired fights into a measured row, or a never-cast row.
fn measured_row(plan: RowPlan, pairs: &[&Paired], notes: Vec<String>) -> Row {
    let diffs = |select: &dyn Fn(&FightRecord) -> bool, shape: Option<Shape>| -> Vec<f64> {
        pairs
            .iter()
            .filter(|p| shape.is_none_or(|s| p.shape == s))
            .flat_map(|p| p.control.iter().zip(&p.spell).map(|(c, s)| as_number(select(s)) - as_number(select(c))))
            .collect()
    };
    let all_control: Vec<&FightRecord> = pairs.iter().flat_map(|p| &p.control).collect();
    let control_win_rate = rate(all_control.iter().filter(|f| f.win).count(), all_control.len());
    let casts: u32 = pairs.iter().flat_map(|p| &p.spell).map(|f| f.casts).sum();
    let members = pairs.iter().map(|p| member_lift(&p.member, p.shape, &p.control, &p.spell)).collect();
    let base = Row {
        plan,
        status: RowStatus::NeverCast,
        reason: None,
        control_win_rate: Some(control_win_rate),
        control_in_band: Some(spell_panel::in_band(control_win_rate)),
        win_lift: None,
        single_lift: None,
        group_lift: None,
        casualty_change: None,
        casts: Some(casts),
        damage_per_cast: None,
        healing_per_cast: None,
        members,
        notes,
    };
    if casts == 0 {
        return base;
    }
    let sum = |f: &dyn Fn(&FightRecord) -> i32| pairs.iter().flat_map(|p| &p.spell).map(f).sum::<i32>();
    let per_cast = |total: i32| (f64::from(total) / f64::from(casts) * 100.0).round() / 100.0;
    let win = |f: &FightRecord| f.win;
    Row {
        status: RowStatus::Measured,
        win_lift: estimate(&diffs(&win, None)),
        single_lift: estimate(&diffs(&win, Some(Shape::Single))),
        group_lift: estimate(&diffs(&win, Some(Shape::Group))),
        casualty_change: estimate(&diffs(&|f| f.casualty, None)),
        damage_per_cast: Some(per_cast(sum(&|f| f.damage))),
        healing_per_cast: Some(per_cast(sum(&|f| f.healing))),
        ..base
    }
}

fn unmeasured_row(plan: RowPlan, reason: String) -> Row {
    Row {
        plan,
        status: RowStatus::NotMeasured,
        reason: Some(reason),
        control_win_rate: None,
        control_in_band: None,
        win_lift: None,
        single_lift: None,
        group_lift: None,
        casualty_change: None,
        casts: None,
        damage_per_cast: None,
        healing_per_cast: None,
        members: vec![],
        notes: vec![],
    }
}

struct Job<'a> {
    row: usize,
    plan: RowPlan,
    member: &'a Member,
    shape: Shape,
    label: String,
}

fn row_notes(spell: &SpellEffect, plan: &RowPlan) -> Vec<String> {
    let mut notes = Vec::new();
    if spell.casting_time == CastingTime::Reaction {
        let counters = spell.effects.iter().any(|e| matches!(e, Effect::CounterSpell(_)));
        notes.push(if counters {
            "Cast as a reaction when an enemy casts a spell.".into()
        } else {
            "Cast as a reaction when its caster is hit by an attack roll; the entry names no trigger.".into()
        });
    }
    if plan.kind == RowKind::Upcast && spell.is_homebrew() {
        if let Some(clause) = &spell.upcast_clause {
            notes.push(format!("The upcast clause \"{}\" is played as the standing assumption says; the engine does not apply it.", clause.trim_end_matches('.')));
        }
    }
    notes
}

/// Run every row of the tested spell against the panel. `progress` gets `(done, total, label)` as
/// each panel member finishes.
pub fn run(spec: &LiftSpec, progress: &(dyn Fn(usize, usize, &str) + Sync)) -> Result<LiftReport, BatchError> {
    if spec.fights == 0 || spec.max_rounds == 0 {
        return Err(BatchError::Invalid("fights and max rounds must be at least 1".into()));
    }
    let spell = spec.spell()?;
    let (constants, resolutions) = batch::resolve_assumptions(&spec.assumptions)?;
    let clause_paths: Vec<String> = batch::spell_clauses(&spec.homebrew).into_iter().map(|(path, _)| path).collect();
    if let Some(path) = resolutions.keys().find(|k| k.starts_with(batch::SPELL_PATH) && !clause_paths.contains(k)) {
        return Err(BatchError::Invalid(format!("`{path}` names no clause of a loaded homebrew spell; pass the spell's file with `--spell`")));
    }
    let refused = batch::unresolved_spell_clauses(&spec.homebrew, &resolutions);
    if !refused.is_empty() {
        return Err(BatchError::Unresolved(refused));
    }
    let plans = plan_rows(&spell);
    let mut rows: Vec<Option<Row>> = vec![None; plans.len()];
    let mut jobs = Vec::new();
    for (i, plan) in plans.iter().enumerate() {
        let level = spec.panel.level(plan.party_level).ok_or_else(|| BatchError::NotFound(format!("the spell panel has no party level {}", plan.party_level)))?;
        if let Some(reason) = unmeasured_reason(&spell, plan, level) {
            rows[i] = Some(unmeasured_row(*plan, reason));
            continue;
        }
        for (shape, pool) in [(Shape::Single, &level.single), (Shape::Group, &level.group)] {
            for member in pool {
                let label = format!("party level {} {} {}", plan.party_level, member.id, shape.word());
                jobs.push(Job { row: i, plan: *plan, member, shape, label });
            }
        }
    }
    let done = baseline::run_parallel(
        &jobs,
        spec.threads,
        |j| &j.label,
        |j| {
            Ok(Paired {
                member: j.member.clone(),
                shape: j.shape,
                control: member_fights(spec, &j.plan, j.member, j.shape, false)?,
                spell: member_fights(spec, &j.plan, j.member, j.shape, true)?,
            })
        },
        progress,
    )?;
    let mut samples = Vec::new();
    let mut logs = Vec::new();
    for (i, plan) in plans.iter().enumerate() {
        if rows[i].is_some() {
            continue;
        }
        let pairs: Vec<&Paired> = jobs.iter().zip(&done).filter(|(j, _)| j.row == i).map(|(_, d)| d).collect();
        let row = measured_row(*plan, &pairs, row_notes(&spell, plan));
        if row.status == RowStatus::Measured {
            let swing = pairs.iter().find_map(|p| p.control.iter().zip(&p.spell).find(|(c, s)| !c.win && s.win).map(|(_, s)| (p, s.index)));
            if let Some((p, index)) = swing {
                let result = replay_fight(spec, plan.party_level, &p.member.id, p.shape, index)?;
                let file = format!("{}-party-level-{}-{}-{}-fight-{index}.txt", spell.id, plan.party_level, p.member.id, p.shape.word());
                samples.push(Sample {
                    party_level: plan.party_level,
                    member: p.member.id.clone(),
                    shape: p.shape,
                    index,
                    seed: result.seed,
                    file: format!("logs/{file}"),
                    replay: replay_command(spec, plan.party_level, &p.member.id, p.shape, index),
                });
                logs.push((file, result.log.unwrap_or_default()));
            }
        }
        rows[i] = Some(row);
    }
    let mut warnings = Vec::new();
    let stale = spec.panel.stale_reasons(&baseline::fingerprints_for(spec.party, &constants));
    if !stale.is_empty() {
        warnings.push(format!("Stale spell panel: {}. The lift below may not hold for this run.", stale.join("; ")));
    }
    if spec.fights < batch::DEFAULT_FIGHTS {
        warnings.push(format!("Thin data: {} fights per panel member, below the 100-fight reference.", spec.fights));
    }
    Ok(LiftReport {
        spell: spell.id.clone(),
        spell_name: spell.name.clone(),
        spell_level: spell.level,
        homebrew: spell.is_homebrew(),
        fingerprint: spell.is_homebrew().then(|| spell.fingerprint()),
        party: spec.party_name.clone(),
        engine_version: crate::ENGINE_VERSION.into(),
        panel_fingerprint: spec.panel.fingerprint.clone(),
        fights_per_member: spec.fights,
        max_rounds: spec.max_rounds,
        interval: INTERVAL_METHOD,
        standing_assumptions: spec.assumptions.clone(),
        rows: rows.into_iter().flatten().collect(),
        warnings,
        samples,
        logs,
    })
}

/// The command that replays one sample fight with its full log.
pub fn replay_command(spec: &LiftSpec, party_level: u8, member: &str, shape: Shape, index: u64) -> String {
    let mut cmd = format!("bench batch --test-spell {} --row {party_level} --member {member}:{} --fight {index}", spec.spell_id, shape.word());
    if spec.fights != spec.panel.fights {
        cmd.push_str(&format!(" --fights {}", spec.fights));
    }
    if spec.max_rounds != batch::DEFAULT_MAX_ROUNDS {
        cmd.push_str(&format!(" --max-rounds {}", spec.max_rounds));
    }
    if spec.lighting != Lighting::Unspecified {
        cmd.push_str(&format!(" --lighting {}", spec.lighting.as_str()));
    }
    if let Some(inv) = &spec.invocation {
        cmd.push_str(&batch::invocation_tail(inv));
    }
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    fn srd(id: &str) -> SpellEffect {
        SpellBook::srd().unwrap().get(id).unwrap().clone()
    }

    fn homebrew(id: &str, level: u8, casting_time: &str, extra: serde_json::Value) -> SpellEffect {
        let mut entry = serde_json::json!({
            "id": id, "name": "Test Spell", "source": "homebrew", "pasted_source": "pasted", "level": level,
            "casting_time": casting_time, "concentration": false,
            "effects": [{"saving_throw": {"ability": "dex", "damage": [{"dice": "2d6", "magical": true, "type": "fire"}], "dc": 0, "half_on_success": true, "no_damage_on_success": false}}],
        });
        for (k, v) in extra.as_object().unwrap() {
            entry[k] = v.clone();
        }
        SpellEffect::parse(entry).unwrap()
    }

    fn rows(spell: &SpellEffect) -> Vec<(RowKind, u8, Option<u8>, u8)> {
        plan_rows(spell).iter().map(|p| (p.kind, p.party_level, p.cast_level, p.control_spell_level)).collect()
    }

    #[test]
    fn a_leveled_spell_gets_a_headline_row_and_a_row_per_upcast_level() {
        assert_eq!(rows(&srd("magic-missile")).first(), Some(&(RowKind::Headline, 1, Some(1), 1)));
        let fireball = rows(&srd("fireball"));
        assert_eq!(fireball.iter().map(|r| r.1).collect::<Vec<_>>(), vec![5, 7, 9, 11, 13, 15, 17]);
        assert_eq!(fireball[0], (RowKind::Headline, 5, Some(3), 3));
        assert!(fireball[1..].iter().all(|r| r.0 == RowKind::Upcast && r.1 == 2 * r.2.unwrap() - 1 && r.3 == r.2.unwrap()));
        let flat = homebrew("flat", 2, "action", serde_json::json!({}));
        assert_eq!(rows(&flat), vec![(RowKind::Headline, 3, Some(2), 2)], "no upcast rule, no upcast rows");
        assert_eq!(rows(&srd("meteor-swarm")), vec![(RowKind::Headline, 17, Some(9), 9)]);
    }

    #[test]
    fn a_cantrip_gets_four_rows_with_no_cast_level_and_a_cantrip_free_control() {
        let cantrip = rows(&srd("fire-bolt"));
        assert_eq!(cantrip.iter().map(|r| r.1).collect::<Vec<_>>(), CANTRIP_LEVELS.to_vec());
        assert!(cantrip.iter().all(|r| r.0 == RowKind::Cantrip && r.2.is_none() && r.3 == 0));
    }

    fn names(block: &StatBlock) -> Vec<String> {
        [&block.actions, &block.bonus_actions, &block.reactions].into_iter().flatten().map(|a| a.id.clone()).collect()
    }

    #[test]
    fn the_swap_replaces_the_tested_levels_spells_and_keeps_the_rest_of_the_list() {
        let party = batch::standard_party().unwrap();
        let book = SpellBook::srd().unwrap();
        let spell = homebrew("ember-lance", 3, "action", serde_json::json!({}));
        let control = spell_panel::control_party(&party, 3, &book);
        let swapped = swap_party(&control, &spell, Some(3));
        let caster = |p: &Party| p[&5].iter().find(|b| spell_panel::is_caster(b)).unwrap().clone();
        let (before, control_caster, after) = (caster(&party), caster(&control), caster(&swapped));
        let mut expected = names(&control_caster);
        expected.insert(control_caster.actions.len(), "ember-lance".to_string());
        assert_eq!(names(&after), expected, "the new ability follows the other actions");
        assert!(names(&before).contains(&"fireball".to_string()) && !names(&after).contains(&"fireball".to_string()));
        assert!(names(&after).contains(&"fire-bolt".to_string()) && names(&after).contains(&"shield".to_string()), "other levels stay");
        assert_eq!(after.spell_slots, before.spell_slots);
        for (level, members) in &party {
            for (a, b) in members.iter().zip(&swapped[level]) {
                if !spell_panel::is_caster(a) {
                    assert_eq!(serde_json::to_value(a).unwrap(), serde_json::to_value(b).unwrap());
                }
            }
        }
    }

    #[test]
    fn the_added_ability_sits_in_the_section_its_casting_time_names() {
        let party = batch::standard_party().unwrap();
        let caster = party[&9].iter().find(|b| spell_panel::is_caster(b)).unwrap();
        let bonus = with_spell(caster, &homebrew("quick", 2, "bonus_action", serde_json::json!({})), Some(2));
        assert!(bonus.bonus_actions.iter().any(|a| a.id == "quick") && !bonus.actions.iter().any(|a| a.id == "quick"));
        let reaction = with_spell(caster, &homebrew("flinch", 2, "reaction", serde_json::json!({})), Some(2));
        let entry = reaction.reactions.iter().find(|a| a.id == "flinch").unwrap();
        assert!(matches!(entry.trigger, Some(ReactionTrigger::HitByAttackRoll { .. })));
        let counter = with_spell(caster, &srd("counterspell"), Some(3));
        assert!(matches!(counter.reactions.iter().find(|a| a.id == "counterspell").unwrap().trigger, Some(ReactionTrigger::SpellCast { .. })));
        let cantrip = with_spell(caster, &srd("fire-bolt"), None);
        assert_eq!(cantrip.actions.iter().filter(|a| a.id == "fire-bolt").count(), 1, "a spell the caster already knows is not added twice");
        assert!(matches!(cantrip.actions.iter().find(|a| a.id == "fire-bolt").unwrap().usage, Some(Usage::AtWill)));
        let long = with_spell(caster, &srd("revivify"), Some(3));
        assert!(long.actions.iter().any(|a| a.id == "revivify"));
        let minute = with_spell(caster, &homebrew("ritual", 2, "long", serde_json::json!({})), Some(2));
        assert_eq!(names(&minute), names(caster));
    }

    fn level(spellcasting_fixture: Option<&str>) -> PanelLevel {
        PanelLevel { party_level: 5, control_spell_level: 3, single: vec![], group: vec![], spellcasting_fixture: spellcasting_fixture.map(String::from) }
    }

    #[test]
    fn each_not_measured_reason_names_its_cause() {
        let headline = RowPlan { kind: RowKind::Headline, party_level: 3, cast_level: Some(2), control_spell_level: 2 };
        let ritual = homebrew("ritual", 2, "long", serde_json::json!({}));
        assert!(unmeasured_reason(&ritual, &headline, &level(None)).unwrap().contains("minute or more"));
        let upcast = RowPlan { kind: RowKind::Upcast, party_level: 5, cast_level: Some(3), control_spell_level: 3 };
        let with_clause = srd("acid-arrow");
        assert!(with_clause.upcast_clause.is_some(), "the test needs an SRD spell with an upcast clause");
        assert!(unmeasured_reason(&with_clause, &upcast, &level(None)).unwrap().contains("upcast clause"));
        assert!(unmeasured_reason(&with_clause, &headline, &level(None)).is_none(), "the headline cast needs no clause");
        let own = homebrew("own", 2, "action", serde_json::json!({"upcast_clause": "Two more targets."}));
        assert!(unmeasured_reason(&own, &upcast, &level(None)).is_none(), "a homebrew clause is resolved by an assumption");
        let counter = srd("counterspell");
        assert!(unmeasured_reason(&counter, &headline, &level(None)).unwrap().contains("spellcaster"));
        assert!(unmeasured_reason(&counter, &headline, &level(Some("mage"))).is_none());
    }

    fn fight(win: bool, casualty: bool, casts: u32) -> FightRecord {
        FightRecord { index: 0, seed: 0, win, casualty, casts, damage: 10 * casts as i32, healing: 0 }
    }

    fn pair(shape: Shape, control: Vec<FightRecord>, spell: Vec<FightRecord>) -> Paired {
        Paired { member: Member { id: "ogre".into(), cr: 2.0, copies: shape.copies(), seed: 1, spellcaster: false, fights: 4, wins: 2, deaths: 0, win_rate: 0.5 }, shape, control, spell }
    }

    const PLAN: RowPlan = RowPlan { kind: RowKind::Headline, party_level: 5, cast_level: Some(3), control_spell_level: 3 };

    #[test]
    fn a_spell_that_is_never_cast_reads_never_cast_and_not_zero() {
        let never = pair(Shape::Single, vec![fight(true, false, 0); 4], vec![fight(true, false, 0); 4]);
        let row = measured_row(PLAN, &[&never], vec![]);
        assert_eq!(row.status, RowStatus::NeverCast);
        assert!(row.win_lift.is_none() && row.casualty_change.is_none() && row.damage_per_cast.is_none());
        assert_eq!(row.casts, Some(0));
        assert_eq!(row.control_win_rate, Some(1.0));
    }

    #[test]
    fn lift_pools_paired_fights_by_shape_and_diagnoses_damage_per_cast() {
        let single = pair(Shape::Single, vec![fight(false, true, 0), fight(false, false, 0), fight(true, false, 0), fight(true, false, 0)], vec![fight(true, false, 1), fight(false, false, 1), fight(true, false, 1), fight(true, false, 1)]);
        let group = pair(Shape::Group, vec![fight(true, false, 0); 4], vec![fight(true, false, 2); 4]);
        let row = measured_row(PLAN, &[&single, &group], vec![]);
        assert_eq!(row.status, RowStatus::Measured);
        assert_eq!(row.single_lift.unwrap().mean, 0.25);
        assert_eq!(row.group_lift.unwrap().mean, 0.0);
        assert_eq!(row.win_lift.unwrap().mean, 0.125);
        assert_eq!(row.win_lift.unwrap().fights, 8);
        assert_eq!(row.casualty_change.unwrap().mean, -0.125);
        assert_eq!(row.damage_per_cast, Some(10.0));
        assert_eq!(row.members[0].win_lift, 0.25);
        assert_eq!(row.members[0].casualty_change, -0.25);
        assert_eq!(row.control_win_rate, Some(0.75));
        assert_eq!(row.control_in_band, Some(true));
    }

    #[test]
    fn the_interval_is_the_mean_within_1_96_standard_errors() {
        let diffs: Vec<f64> = [1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0].to_vec();
        let e = estimate(&diffs).unwrap();
        let sd = (diffs.iter().map(|d| (d - 0.375_f64).powi(2)).sum::<f64>() / 7.0).sqrt();
        let half = 1.96 * sd / (8.0_f64).sqrt();
        assert_eq!(e.mean, 0.375);
        assert!((e.high - e.mean - half).abs() < 1e-3 && (e.mean - e.low - half).abs() < 1e-3);
        assert_eq!(estimate(&[0.0; 5]).map(|e| (e.low, e.high)), Some((0.0, 0.0)), "identical fights have no spread");
        assert!(estimate(&[]).is_none());
    }

    fn spec<'a>(spell_id: &str, homebrew: Vec<SpellEffect>, party: &'a Party, panel: &'a SpellPanel, fights: u32) -> LiftSpec<'a> {
        LiftSpec {
            spell_id: spell_id.into(),
            homebrew,
            party_name: "standard".into(),
            party,
            panel,
            fights,
            max_rounds: batch::DEFAULT_MAX_ROUNDS,
            assumptions: vec![],
            lighting: Lighting::Unspecified,
            threads: 2,
            invocation: None,
        }
    }

    #[test]
    fn control_fights_repeat_across_spells_and_pair_with_their_spell_fights() {
        let party = batch::standard_party().unwrap();
        let panel = spell_panel::embedded().expect("a committed panel").unwrap();
        let level = panel.level(5).unwrap();
        let member = &level.single[0];
        let plan = |s: &SpellEffect| plan_rows(s).into_iter().find(|p| p.party_level == 5).unwrap();
        let (fireball, other) = (srd("fireball"), homebrew("ember-lance", 3, "action", serde_json::json!({})));
        let a = member_fights(&spec("fireball", vec![], &party, &panel, 6), &plan(&fireball), member, Shape::Single, false).unwrap();
        let b = member_fights(&spec("ember-lance", vec![other.clone()], &party, &panel, 6), &plan(&other), member, Shape::Single, false).unwrap();
        assert_eq!(a, b, "the control fights do not depend on the tested spell");
        let spell = member_fights(&spec("fireball", vec![], &party, &panel, 6), &plan(&fireball), member, Shape::Single, true).unwrap();
        assert_eq!(a.iter().map(|f| (f.index, f.seed)).collect::<Vec<_>>(), spell.iter().map(|f| (f.index, f.seed)).collect::<Vec<_>>());
        assert!(a.iter().all(|f| f.casts == 0));
    }

    #[test]
    fn casts_and_the_damage_they_raise_are_counted_per_fight() {
        let party = batch::standard_party().unwrap();
        let panel = spell_panel::embedded().expect("a committed panel").unwrap();
        let shatter = srd("shatter");
        let plan = plan_rows(&shatter).into_iter().find(|p| p.party_level == 3).unwrap();
        let level = panel.level(3).unwrap();
        let fights = member_fights(&spec("shatter", vec![], &party, &panel, 8), &plan, &level.group[0], Shape::Group, true).unwrap();
        assert!(fights.iter().any(|f| f.casts > 0 && f.damage > 0), "{fights:?}");
        assert!(fights.iter().all(|f| f.healing == 0));
    }

    #[test]
    fn a_whole_run_returns_one_row_per_plan_and_replays_its_sample() {
        let party = batch::standard_party().unwrap();
        let panel = spell_panel::embedded().expect("a committed panel").unwrap();
        let ritual = homebrew("ritual", 2, "long", serde_json::json!({}));
        let report = run(&spec("ritual", vec![ritual], &party, &panel, 4), &|_, _, _| {}).unwrap();
        assert_eq!(report.rows.len(), 1);
        assert_eq!(report.rows[0].status, RowStatus::NotMeasured);
        assert!(report.rows[0].reason.as_deref().unwrap().contains("minute or more"));
        let bolt = run(&spec("shatter", vec![], &party, &panel, 4), &|_, _, _| {}).unwrap();
        let headline = &bolt.rows[0];
        assert_eq!((headline.plan.party_level, headline.plan.kind), (3, RowKind::Headline));
        assert!(matches!(headline.status, RowStatus::Measured | RowStatus::NeverCast));
        assert_eq!(headline.members.len(), panel.level(3).unwrap().single.len() + panel.level(3).unwrap().group.len());
        assert!(bolt.rows.iter().skip(1).all(|r| r.plan.kind == RowKind::Upcast));
        if let Some(sample) = bolt.samples.first() {
            let shape = if sample.shape == Shape::Single { "single" } else { "group" };
            assert!(sample.replay.contains(&format!("--member {}:{shape} --fight {}", sample.member, sample.index)));
            let again = replay_fight(&spec("shatter", vec![], &party, &panel, 4), sample.party_level, &sample.member, sample.shape, sample.index).unwrap();
            assert_eq!(again.seed, sample.seed);
            assert!(again.log.is_some());
        }
    }

    #[test]
    fn an_unresolved_homebrew_clause_or_a_stray_spell_assumption_refuses_the_run() {
        let party = batch::standard_party().unwrap();
        let panel = spell_panel::embedded().expect("a committed panel").unwrap();
        let clause = homebrew("clause", 2, "action", serde_json::json!({"agent_clauses": ["The target is also dazed."]}));
        let refused = run(&spec("clause", vec![clause.clone()], &party, &panel, 2), &|_, _, _| {});
        assert!(matches!(refused, Err(BatchError::Unresolved(ref ids)) if ids == &["spells.clause.agent_clauses.0".to_string()]), "{refused:?}");
        let mut stray = spec("clause", vec![clause], &party, &panel, 2);
        stray.assumptions = vec![StandingAssumption { path: "spells.nothing.agent_clauses.0".into(), value: "ignored".into(), reason: "t".into() }];
        assert!(matches!(run(&stray, &|_, _, _| {}), Err(BatchError::Invalid(_))));
        let unknown = run(&spec("no-such-spell", vec![], &party, &panel, 2), &|_, _, _| {});
        assert!(matches!(unknown, Err(BatchError::NotFound(_))));
    }
}
