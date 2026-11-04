//! Batch runner: one monster against a party across levels (ADR 0002, 0007, 0012).
//!
//! Every fight is a fresh encounter on its own ChaCha8 stream, seeded from the
//! master seed and a global fight index (`(level - 1) * fights + i`), so a run
//! is order-independent and `--fight <index>` replays one fight bit for bit.
//! The runner writes nothing itself; the caller saves the HTML and logs.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::baseline::{self, Baseline, Comparison};
use crate::effect::{Effect, Lighting};

use crate::encounter::{EncounterState, EndState, PcDice, RangeBand, StandingAssumption, PARTY_SIDE};
use crate::engine::{self, EngineEvent, StepRequest};
use crate::live::{self, Entrant, StartSpec};
use crate::reaction::ReactionDecider;
use crate::rng::derive_fight_seed;
use crate::stat_block::{AbilityEntry, AbilityKind, DefaultResolution, StatBlock};
use crate::tactics::{self, PolicyConstants, Section};

pub const MONSTER_SIDE: &str = "monsters";
pub const DEFAULT_FIGHTS: u32 = 100;
pub const DEFAULT_MAX_ROUNDS: u32 = 20;

/// Party stat blocks by level.
pub type Party = BTreeMap<u8, Vec<StatBlock>>;

/// Everything one batch run needs.
#[derive(Debug, Clone)]
pub struct BatchSpec {
    pub monster: StatBlock,
    pub party_name: String,
    pub party: Party,
    pub levels: Vec<u8>,
    pub fights: u32,
    pub max_rounds: u32,
    pub master_seed: u64,
    pub assumptions: Vec<StandingAssumption>,
    /// Every fight's encounter lighting, from `--lighting`.
    pub lighting: Lighting,
    /// The curve to compare against; `None` compares against party level only.
    pub baseline: Option<Baseline>,
    /// The arguments the batch was named by, so each sample's replay command reruns it.
    /// `None` names the monster by id and the party by `party_name`.
    pub invocation: Option<Invocation>,
}

/// The `bench batch` arguments that `BatchSpec` holds only in parsed form.
#[derive(Debug, Clone)]
pub struct Invocation {
    /// The monster argument as passed: a stat block path or fixture id.
    pub monster: String,
    /// The `--party` folder name as passed; `None` is the built-in standard party.
    pub party: Option<String>,
    /// The `--assumptions` file as passed.
    pub assumptions_file: Option<String>,
    /// Each `--assume path=value` as passed, in order.
    pub assume: Vec<String>,
    /// `--assume-reason`, when it differs from the default.
    pub assume_reason: Option<String>,
}

/// Why a batch cannot run.
#[derive(Debug, Clone, PartialEq)]
pub enum BatchError {
    /// Abilities with no resolution, by id.
    Unresolved(Vec<String>),
    Invalid(String),
    NotFound(String),
}

impl std::fmt::Display for BatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unresolved(ids) => write!(
                f,
                "unresolved unmodeled abilities or agent clauses: {}; add `--assume abilities.<id>=ignored` or a standing assumption for each",
                ids.join(", ")
            ),
            Self::Invalid(s) | Self::NotFound(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for BatchError {}

/// How one fight ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    PartyWin,
    MonsterWin,
    /// The round cap came first. Counted apart from wins and losses.
    Stalemate,
}

#[derive(Debug, Clone, Serialize)]
pub struct FightResult {
    pub level: u8,
    pub index: u64,
    pub seed: u64,
    pub outcome: Outcome,
    pub rounds: u32,
    pub monster_damage: i32,
    pub pc_deaths: u32,
    #[serde(skip)]
    pub log: Option<String>,
}

impl FightResult {
    pub fn dpr(&self) -> f64 {
        f64::from(self.monster_damage) / f64::from(self.rounds.max(1))
    }
}

/// Median, mean and P10 to P90 spread.
#[derive(Debug, Clone, Serialize)]
pub struct Spread {
    pub median: f64,
    pub mean: f64,
    pub p10: f64,
    pub p90: f64,
}

impl Spread {
    pub(crate) fn of(values: &[f64]) -> Self {
        let mut sorted = values.to_vec();
        sorted.sort_by(f64::total_cmp);
        let rank = |p: f64| {
            if sorted.is_empty() {
                return 0.0;
            }
            let i = ((p * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len()) - 1;
            sorted[i]
        };
        let median = if sorted.is_empty() {
            0.0
        } else if sorted.len() % 2 == 1 {
            sorted[sorted.len() / 2]
        } else {
            (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
        };
        let mean = if sorted.is_empty() { 0.0 } else { sorted.iter().sum::<f64>() / sorted.len() as f64 };
        Self { median: round2(median), mean: round2(mean), p10: round2(rank(0.1)), p90: round2(rank(0.9)) }
    }
}

/// Difficulty band from the project calibration thresholds (ticket 015).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Band {
    Low,
    Moderate,
    High,
}

impl Band {
    pub fn of(win_rate: f64, death_rate: f64) -> Self {
        if win_rate >= 0.95 && death_rate < 0.02 {
            Band::Low
        } else if win_rate >= 0.75 && death_rate < 0.15 {
            Band::Moderate
        } else {
            Band::High
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            Band::Low => "Low",
            Band::Moderate => "Moderate",
            Band::High => "High",
        }
    }
}

/// Results for one party level.
#[derive(Debug, Clone, Serialize)]
pub struct Cell {
    pub level: u8,
    pub fights: u32,
    pub wins: u32,
    pub losses: u32,
    pub stalemates: u32,
    /// Fights with at least one PC death.
    pub deaths: u32,
    pub win_rate: f64,
    pub rounds: Spread,
    pub monster_dpr: Spread,
    pub band: Band,
    pub flags: Vec<String>,
}

/// One ability's resolution as the report lists it.
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub ability_id: String,
    pub name: String,
    /// `agent_clause`, `skipped`, `neutral`, `base_form`, `ignored`, `assumed`, `inert` or `lighting`.
    pub kind: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Sample {
    pub selection: String,
    pub level: u8,
    pub index: u64,
    pub seed: u64,
    pub file: String,
    pub replay: String,
}

/// The JSON summary printed on stdout.
#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub monster: String,
    pub monster_name: String,
    pub claimed_cr: Option<f64>,
    pub party: String,
    pub engine_version: String,
    pub policy: PolicyInfo,
    pub master_seed: u64,
    pub fights_per_level: u32,
    pub max_rounds: u32,
    pub lighting: Lighting,
    pub levels: Vec<Cell>,
    pub effective_cr: String,
    pub representative_level: Option<u8>,
    pub standing_assumptions: Vec<StandingAssumption>,
    pub findings: Vec<Finding>,
    pub warnings: Vec<String>,
    pub samples: Vec<Sample>,
    pub comparison: Option<Comparison>,
    pub srd: SrdFacts,
}

/// SRD 5.2.1 reference numbers for the claimed CR, kept apart from the project-derived reading.
#[derive(Debug, Clone, Serialize)]
pub struct SrdFacts {
    pub xp: Option<u64>,
    pub proficiency_bonus: Option<u32>,
    pub level: Option<u8>,
    /// Four-character XP budget at `level`: Low, Moderate, High.
    pub budget: Option<[u64; 3]>,
    /// Which budget the claimed CR's XP fits under, as a plain statement.
    pub within: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PolicyInfo {
    pub id: String,
    pub version: u32,
    pub fingerprint: String,
    pub constants: PolicyConstants,
}

/// A finished batch: summary, report page and the sampled logs by file name.
#[derive(Debug, Clone)]
pub struct BatchReport {
    pub summary: Summary,
    pub html: String,
    pub logs: Vec<(String, String)>,
}

/// Split standing assumptions into policy constants and ability resolutions.
pub fn resolve_assumptions(assumptions: &[StandingAssumption]) -> Result<(PolicyConstants, BTreeMap<String, String>), BatchError> {
    let mut constants = PolicyConstants::default();
    let mut abilities = BTreeMap::new();
    for a in assumptions {
        if constants.apply(&a.path, &a.value).map_err(BatchError::Invalid)? {
            continue;
        }
        match a.path.strip_prefix("abilities.") {
            Some(id) if !id.is_empty() => {
                let value = a.value.as_str().map_or_else(|| a.value.to_string(), str::to_string);
                abilities.insert(id.to_string(), value);
            }
            _ => {
                return Err(BatchError::Invalid(format!(
                    "`{}` is not a batch assumption path; use `policy.<constant>` or `abilities.<id>`",
                    a.path
                )))
            }
        }
    }
    Ok((constants, abilities))
}

fn all_abilities(block: &StatBlock) -> impl Iterator<Item = &AbilityEntry> {
    block
        .traits
        .iter()
        .chain(&block.actions)
        .chain(&block.bonus_actions)
        .chain(&block.reactions)
        .chain(&block.legendary_actions)
}

/// Ids of abilities batch refuses: unmodeled or carrying agent clauses, with
/// no imported default resolution and no standing assumption.
pub fn unresolved(monster: &StatBlock, resolutions: &BTreeMap<String, String>) -> Vec<String> {
    all_abilities(monster)
        .filter(|a| a.kind == AbilityKind::Unmodeled || !a.agent_clauses.is_empty())
        .filter(|a| a.default_resolution.is_none() && !resolutions.contains_key(&a.id))
        .map(|a| a.id.clone())
        .collect()
}

fn findings(monster: &StatBlock, resolutions: &BTreeMap<String, String>, lighting: Lighting) -> Vec<Finding> {
    let mut out = Vec::new();
    for a in all_abilities(monster) {
        let (kind, note) = if let Some(value) = resolutions.get(&a.id) {
            let kind = if value == "ignored" { "ignored" } else { "assumed" };
            (kind, format!("Resolved by standing assumption: {value}."))
        } else if let Some(line) = lighting_line(a, lighting) {
            let kind = if a.default_resolution == Some(DefaultResolution::Skipped) { "skipped" } else { "lighting" };
            match a.agent_clauses.is_empty() {
                true => (kind, line),
                false => (kind, format!("{line} Agent judgment left out: {}", a.agent_clauses.join(" "))),
            }
        } else if let Some(resolution) = a.default_resolution {
            match resolution {
                DefaultResolution::Skipped => ("skipped", "Not modeled in phase 1; results may understate or overstate the monster.".into()),
                DefaultResolution::Neutral => ("neutral", "SRD default: no combat effect.".into()),
                DefaultResolution::BaseForm => ("base_form", "SRD default: the monster stays in its stat-block form.".into()),
            }
        } else if a.is_no_provoke() {
            ("skipped", "Not modeled in phase 1; results may understate or overstate the monster.".into())
        } else if a.kind == AbilityKind::Inert {
            ("inert", "No combat effect in this encounter.".into())
        } else if !a.agent_clauses.is_empty() {
            ("agent_clause", format!("Modeled effects run; agent judgment left out: {}", a.agent_clauses.join(" ")))
        } else {
            continue;
        };
        out.push(Finding { ability_id: a.id.clone(), name: a.name.clone(), kind: kind.into(), note });
    }
    out
}

fn lighting_line(ability: &AbilityEntry, lighting: Lighting) -> Option<String> {
    let required = ability.effects.iter().find_map(|e| match e {
        Effect::RollModifier(m) => Some(m.lighting),
        _ => None,
    })?;
    let on = if lighting.meets(required) { "on" } else { "off" };
    Some(format!("Applies in {}; {on} in this run.", light_words(required)))
}

fn light_words(lighting: Lighting) -> &'static str {
    match lighting {
        Lighting::Unspecified => "unspecified light",
        Lighting::DimLight => "Dim Light",
        Lighting::BrightLight => "Bright Light",
        Lighting::Sunlight => "sunlight",
    }
}

fn lighting_row(lighting: Lighting) -> &'static str {
    match lighting {
        Lighting::Unspecified => "unspecified, lighting traits off",
        Lighting::DimLight => "dim light",
        Lighting::BrightLight => "bright light",
        Lighting::Sunlight => "sunlight (also Bright Light)",
    }
}

/// Run the whole batch.
pub fn run(spec: &BatchSpec) -> Result<BatchReport, BatchError> {
    let (constants, resolutions) = prepare(spec)?;
    let results = play(spec, &constants)?;
    let cells: Vec<Cell> = spec
        .levels
        .iter()
        .map(|&level| cell(level, &results.iter().filter(|r| r.level == level).collect::<Vec<_>>()))
        .collect();
    let representative_level = representative_level(spec);
    let mut samples = Vec::new();
    let mut logs = Vec::new();
    for (selection, index) in pick_samples(&results, representative_level) {
        let level = level_of(index, spec.fights);
        let replayed = fight(spec, &constants, level, index, true)?;
        let file = log_file_name(&spec.monster.id, level, index);
        samples.push(Sample {
            selection: selection.into(),
            level,
            index,
            seed: replayed.seed,
            file: format!("logs/{file}"),
            replay: replay_command(spec, index),
        });
        logs.push((file, replayed.log.unwrap_or_default()));
    }
    let findings = findings(&spec.monster, &resolutions, spec.lighting);
    let mut warnings: Vec<String> = findings
        .iter()
        .filter(|f| f.kind == "skipped")
        .map(|f| format!("Skipped mechanic: {} ({}) is not modeled; results may change.", f.name, f.ability_id))
        .collect();
    for spell in engine::missing_spells(&spec.monster) {
        warnings.push(format!("Spell `{spell}` is not in the spell effects file; a cast of it does nothing, so results may understate this monster."));
    }
    for (spell, level, clause) in engine::unapplied_upcasts(&spec.monster) {
        let clause = clause.trim_end_matches('.');
        warnings.push(format!("Spell `{spell}` at level {level}: {clause} is not applied; results may understate this monster."));
    }
    let effective = effective_cr(&cells);
    let fingerprints = baseline::fingerprints_for(&spec.party, &constants);
    let comparison = spec.baseline.as_ref().map(|b| {
        let mut c = baseline::compare(
            b,
            &baseline::RunReading {
                claimed_cr: spec.monster.challenge_rating,
                cells: &cells,
                representative_level,
                effective_cr: &effective,
                fingerprints: &fingerprints,
            },
        );
        if spec.lighting != Lighting::Unspecified {
            c.basis.push_str(" The baseline curve's fixtures fought with lighting unspecified.");
        }
        c
    });
    match &comparison {
        None => warnings.push("No baseline curve was supplied; the claimed-CR comparison uses party level only.".into()),
        Some(c) if !c.stale.is_empty() => {
            warnings.push(format!("Stale baseline: {}. The comparison below may not hold for this run.", c.stale.join("; ")))
        }
        Some(_) => {}
    }
    if spec.fights < DEFAULT_FIGHTS {
        warnings.push(format!("Thin data: {} fights per level, below the 100-fight reference cell.", spec.fights));
    }
    let summary = Summary {
        monster: spec.monster.id.clone(),
        monster_name: spec.monster.name.clone(),
        claimed_cr: spec.monster.challenge_rating,
        party: spec.party_name.clone(),
        engine_version: crate::ENGINE_VERSION.into(),
        policy: PolicyInfo {
            id: tactics::POLICY_ID.into(),
            version: tactics::POLICY_VERSION,
            fingerprint: tactics::fingerprint(&constants),
            constants,
        },
        master_seed: spec.master_seed,
        fights_per_level: spec.fights,
        max_rounds: spec.max_rounds,
        lighting: spec.lighting,
        effective_cr: effective,
        levels: cells,
        representative_level,
        standing_assumptions: spec.assumptions.clone(),
        findings,
        warnings,
        samples,
        comparison,
        srd: srd_facts(spec.monster.challenge_rating, representative_level),
    };
    let html = render_html(&summary);
    Ok(BatchReport { summary, html, logs })
}

/// Play every fight without building a report or logs.
pub fn run_fights(spec: &BatchSpec) -> Result<Vec<FightResult>, BatchError> {
    let (constants, _) = prepare(spec)?;
    play(spec, &constants)
}

fn play(spec: &BatchSpec, constants: &PolicyConstants) -> Result<Vec<FightResult>, BatchError> {
    let mut results = Vec::new();
    for &level in &spec.levels {
        for i in 0..spec.fights {
            let index = fight_index(level, i, spec.fights);
            results.push(fight(spec, constants, level, index, false)?);
        }
    }
    Ok(results)
}

/// Replay one fight by its global index, with its full log.
pub fn replay_fight(spec: &BatchSpec, index: u64) -> Result<FightResult, BatchError> {
    let (constants, _) = prepare(spec)?;
    let level = level_of(index, spec.fights);
    if !spec.party.contains_key(&level) || index >= 20 * u64::from(spec.fights) {
        return Err(BatchError::NotFound(format!("fight {index} is outside levels 1-20 at {} fights per level", spec.fights)));
    }
    fight(spec, &constants, level, index, true)
}

fn prepare(spec: &BatchSpec) -> Result<(PolicyConstants, BTreeMap<String, String>), BatchError> {
    if spec.fights == 0 || spec.max_rounds == 0 {
        return Err(BatchError::Invalid("fights and max rounds must be at least 1".into()));
    }
    let (constants, resolutions) = resolve_assumptions(&spec.assumptions)?;
    let refused = unresolved(&spec.monster, &resolutions);
    if !refused.is_empty() {
        return Err(BatchError::Unresolved(refused));
    }
    let missing: Vec<String> = spec.levels.iter().filter(|l| !spec.party.contains_key(l)).map(u8::to_string).collect();
    if !missing.is_empty() {
        return Err(BatchError::NotFound(format!("party `{}` has no stat blocks for level {}", spec.party_name, missing.join(", "))));
    }
    Ok((constants, resolutions))
}

pub fn fight_index(level: u8, i: u32, fights: u32) -> u64 {
    u64::from(level - 1) * u64::from(fights) + u64::from(i)
}

pub fn level_of(index: u64, fights: u32) -> u8 {
    (index / u64::from(fights.max(1)) + 1).min(255) as u8
}

pub fn log_file_name(monster: &str, level: u8, index: u64) -> String {
    format!("{monster}-level-{level}-fight-{index}.txt")
}

/// The POSIX shell command that replays fight `index` of this batch with its full log.
pub fn replay_command(spec: &BatchSpec, index: u64) -> String {
    let inv = spec.invocation.as_ref();
    let monster = inv.map_or(&spec.monster.id, |i| &i.monster);
    let party = match inv {
        Some(i) => i.party.as_ref(),
        None => Some(&spec.party_name).filter(|p| p.as_str() != "standard"),
    };
    let mut cmd = format!("bench batch {} --seed {} --fight {index}", shell_quote(monster), spec.master_seed);
    if spec.fights != DEFAULT_FIGHTS {
        cmd.push_str(&format!(" --fights {}", spec.fights));
    }
    if spec.max_rounds != DEFAULT_MAX_ROUNDS {
        cmd.push_str(&format!(" --max-rounds {}", spec.max_rounds));
    }
    if let Some(party) = party {
        cmd.push_str(&format!(" --party {}", shell_quote(party)));
    }
    if (1..=20).any(|l| !spec.party.contains_key(&l)) {
        cmd.push_str(&format!(" --levels {}", level_of(index, spec.fights)));
    }
    if spec.lighting != Lighting::Unspecified {
        cmd.push_str(&format!(" --lighting {}", spec.lighting.as_str()));
    }
    let Some(inv) = inv else { return cmd };
    if let Some(file) = &inv.assumptions_file {
        cmd.push_str(&format!(" --assumptions {}", shell_quote(file)));
    }
    for a in &inv.assume {
        cmd.push_str(&format!(" --assume {}", shell_quote(a)));
    }
    if let Some(reason) = &inv.assume_reason {
        cmd.push_str(&format!(" --assume-reason {}", shell_quote(reason)));
    }
    cmd
}

fn shell_quote(word: &str) -> String {
    let plain = |c: char| c.is_ascii_alphanumeric() || "-_./=:,+@%".contains(c);
    if !word.is_empty() && word.chars().all(plain) {
        word.to_string()
    } else {
        format!("'{}'", word.replace('\'', r"'\''"))
    }
}

fn fight(spec: &BatchSpec, constants: &PolicyConstants, level: u8, index: u64, with_log: bool) -> Result<FightResult, BatchError> {
    let seed = derive_fight_seed(spec.master_seed, index);
    let mut entrants = vec![Entrant { stat_block: spec.monster.clone(), side: MONSTER_SIDE.into(), band: RangeBand::Near }];
    for pc in &spec.party[&level] {
        entrants.push(Entrant { stat_block: pc.clone(), side: PARTY_SIDE.into(), band: RangeBand::Near });
    }
    let id = format!("{}-level-{level}-fight-{index}", spec.monster.id);
    let (mut state, _) = live::start(StartSpec {
        id,
        origin_directory: String::new(),
        entrants,
        seed,
        surprised: vec![],
        individual_initiative: true,
        pc_dice: PcDice::Engine,
        lighting: spec.lighting,
        reaction_windows: true,
    })
    .map_err(|e| BatchError::Invalid(e.to_string()))?;
    state.reactions.decider = ReactionDecider::Policy(constants.clone());
    state.log.clear();
    state.initial_snapshot = None;
    state.player_dice.retarget_downed = true;
    let mut log = Log::new(with_log);
    log.header(spec, constants, level, index, seed);
    log.line(format!("INITIATIVE {}", state.initiative_order.join(", ")));
    let mut monster_damage = 0;
    let mut printed_round = 0;
    let outcome = loop {
        if let Some(outcome) = decided(&state) {
            break outcome;
        }
        if state.round > spec.max_rounds {
            break Outcome::Stalemate;
        }
        if state.round != printed_round {
            printed_round = state.round;
            log.line(format!("ROUND {}", state.round));
        }
        let actor = state.active_turn.clone().ok_or_else(|| BatchError::Invalid("fight has no active turn".into()))?;
        take_turn(&mut state, &actor, constants, &mut log, &mut monster_damage);
        end_turn(&mut state, constants, &mut log, &mut monster_damage)?;
    };
    let rounds = state.round.min(spec.max_rounds).max(1);
    let pc_deaths = state
        .combatants
        .iter()
        .filter(|c| c.side_id == PARTY_SIDE)
        .filter(|c| c.end_state == EndState::Dead || (outcome == Outcome::MonsterWin && c.end_state == EndState::Unconscious))
        .count() as u32;
    log.result(outcome, rounds, monster_damage, pc_deaths, spec);
    Ok(FightResult { level, index, seed, outcome, rounds, monster_damage, pc_deaths, log: log.finish() })
}

fn decided(state: &EncounterState) -> Option<Outcome> {
    let standing = |side_is_party: bool| {
        state
            .combatants
            .iter()
            .filter(|c| (c.side_id == PARTY_SIDE) == side_is_party)
            .any(|c| c.end_state == EndState::Active)
    };
    match (standing(true), standing(false)) {
        (true, false) => Some(Outcome::PartyWin),
        (false, _) => Some(Outcome::MonsterWin),
        _ => None,
    }
}

fn take_turn(state: &mut EncounterState, actor: &str, constants: &PolicyConstants, log: &mut Log, damage: &mut i32) {
    for section in [Section::Action, Section::Bonus] {
        if decided(state).is_some() {
            return;
        }
        let mut attempts = 0;
        loop {
            attempts += 1;
            match tactics::choose(state, actor, section, constants) {
                Some(plan) => {
                    log.line(format!("  {}", plan.log_line()));
                    match execute(state, &plan) {
                        Ok(events) => {
                            log.events(state, &events, damage);
                            break;
                        }
                        Err(e) if attempts < 2 => log.line(format!("  plan failed ({e}); recomputing from the updated state")),
                        Err(e) => {
                            log.line(format!("  plan failed again ({e}); no action taken"));
                            break;
                        }
                    }
                }
                None if section == Section::Action => {
                    let fallback = tactics::fallback(state, actor, constants);
                    log.line(format!("  {}", fallback.log_line()));
                    for m in &fallback.movement {
                        let request = StepRequest::Move { actor_id: actor.into(), target_id: Some(m.target_id.clone()), band: m.band, feet: m.feet, opportunity: vec![], declined: vec![] };
                        if let Ok(events) = engine::step_events(state, request) {
                            log.events(state, &events, damage);
                        }
                    }
                    break;
                }
                None => break,
            }
        }
    }
}

fn execute(state: &mut EncounterState, plan: &tactics::Plan) -> Result<Vec<EngineEvent>, engine::EngineError> {
    let mut events = Vec::new();
    for m in &plan.movement {
        let left = m.feet;
        events.extend(engine::step_events(
            state,
            StepRequest::Move { actor_id: plan.actor_id.clone(), target_id: Some(m.target_id.clone()), band: m.band, feet: left, opportunity: vec![], declined: vec![] },
        )?);
    }
    state.player_dice.picks = plan.picks.iter().cloned().collect();
    let result = match plan.section {
        Section::Legendary => engine::use_legendary_action(state, &plan.actor_id, &plan.action_id, &plan.targets),
        _ => engine::step_events(
            state,
            StepRequest::Action {
                actor_id: plan.actor_id.clone(),
                action_id: plan.action_id.clone(),
                target_ids: plan.targets.clone(),
                slot_level: None,
            },
        ),
    };
    state.player_dice.picks.clear();
    events.extend(result?);
    Ok(events)
}

/// Finish the active turn, open the legendary window, then start the next
/// turn, so legendary actions resolve before start-of-turn resets and rolls.
fn end_turn(state: &mut EncounterState, constants: &PolicyConstants, log: &mut Log, damage: &mut i32) -> Result<(), BatchError> {
    let invalid = |e: engine::EngineError| BatchError::Invalid(e.to_string());
    let events = engine::finish_turn(state).map_err(invalid)?;
    log.events(state, &events, damage);
    if decided(state).is_none() {
        legendary_window(state, constants, log, damage);
    }
    let events = engine::start_next_turn(state).map_err(invalid)?;
    log.events(state, &events, damage);
    Ok(())
}

/// Each creature that may take a legendary action spends one use when a plan
/// scores. Unused uses lapse when its own turn starts.
fn legendary_window(state: &mut EncounterState, constants: &PolicyConstants, log: &mut Log, damage: &mut i32) {
    let holders: Vec<String> = engine::legendary_holders(state).into_iter().map(|h| h.id).collect();
    for id in holders {
        if decided(state).is_some() || engine::legendary_uses_left(state, &id).is_err() {
            continue;
        }
        if let Some(plan) = tactics::choose(state, &id, Section::Legendary, constants) {
            log.line(format!("  {}", plan.log_line()));
            match execute(state, &plan) {
                Ok(events) => log.events(state, &events, damage),
                Err(e) => log.line(format!("  legendary action failed ({e})")),
            }
        }
    }
}

struct Log {
    lines: Option<Vec<String>>,
}

impl Log {
    fn new(enabled: bool) -> Self {
        Self { lines: enabled.then(Vec::new) }
    }

    fn line(&mut self, text: String) {
        if let Some(lines) = &mut self.lines {
            lines.push(text);
        }
    }

    fn header(&mut self, spec: &BatchSpec, constants: &PolicyConstants, level: u8, index: u64, seed: u64) {
        if self.lines.is_none() {
            return;
        }
        self.line("HOMEBREW BENCH BATCH FIGHT LOG".into());
        self.line(format!("Monster: {} ({})", spec.monster.name, spec.monster.id));
        self.line(format!("Party: {} at level {level}", spec.party_name));
        self.line(format!("Fight index: {index}"));
        self.line(format!("Master seed: {}", spec.master_seed));
        self.line(format!("Fight seed: {seed}"));
        self.line(format!("Engine: {}", crate::ENGINE_VERSION));
        self.line(format!("Policy: {} v{} ({})", tactics::POLICY_ID, tactics::POLICY_VERSION, tactics::fingerprint(constants)));
        self.line("STANDING ASSUMPTIONS".into());
        self.line("  standard batch rules: one monster against the party, both sides start near, round cap counts as a stalemate".into());
        self.line(format!("  lighting = {}", spec.lighting.as_str()));
        if spec.assumptions.is_empty() {
            self.line("  none beyond policy defaults".into());
        }
        for a in &spec.assumptions {
            self.line(format!("  {} = {} ({})", a.path, a.value, a.reason));
        }
        self.line(String::new());
    }

    fn events(&mut self, state: &EncounterState, events: &[EngineEvent], damage: &mut i32) {
        for e in events {
            let monster_source = e
                .actor_id
                .as_deref()
                .and_then(|id| state.combatants.iter().find(|c| c.id == id))
                .is_some_and(|c| c.side_id != PARTY_SIDE);
            if e.kind == "damage" && monster_source {
                *damage += e.damage.unwrap_or(0);
            }
            if self.lines.is_some() && !matches!(e.kind.as_str(), "end_turn" | "start_turn") {
                self.line(format!("    {}", e.message));
            }
        }
    }

    fn result(&mut self, outcome: Outcome, rounds: u32, damage: i32, deaths: u32, spec: &BatchSpec) {
        if self.lines.is_none() {
            return;
        }
        self.line(String::new());
        self.line("RESULT".into());
        self.line(format!(
            "  {} after {rounds} rounds.",
            match outcome {
                Outcome::PartyWin => "Party victory",
                Outcome::MonsterWin => "Monster victory",
                Outcome::Stalemate => "Stalemate at the round cap",
            }
        ));
        self.line(format!("  Monster damage per round: {:.1}.", f64::from(damage) / f64::from(rounds.max(1))));
        self.line(format!("  PC deaths: {deaths}."));
        for a in all_abilities(&spec.monster).filter(|a| a.default_resolution == Some(DefaultResolution::Skipped) || a.is_no_provoke()) {
            self.line(format!("  Skipped mechanic warning: {} may change this result.", a.name));
        }
    }

    fn finish(self) -> Option<String> {
        self.lines.map(|l| l.join("\n") + "\n")
    }
}

pub(crate) fn cell(level: u8, results: &[&FightResult]) -> Cell {
    let n = results.len() as u32;
    let wins = results.iter().filter(|r| r.outcome == Outcome::PartyWin).count() as u32;
    let losses = results.iter().filter(|r| r.outcome == Outcome::MonsterWin).count() as u32;
    let deaths = results.iter().filter(|r| r.pc_deaths > 0).count() as u32;
    let win_rate = f64::from(wins) / f64::from(n.max(1));
    let death_rate = f64::from(deaths) / f64::from(n.max(1));
    let rounds: Vec<f64> = results.iter().map(|r| f64::from(r.rounds)).collect();
    let dpr: Vec<f64> = results.iter().map(|r| r.dpr()).collect();
    let mut flags = Vec::new();
    if n < DEFAULT_FIGHTS {
        flags.push("Thin data".into());
    }
    if n - wins - losses > 0 {
        flags.push(format!("{} stalemates", n - wins - losses));
    }
    Cell {
        level,
        fights: n,
        wins,
        losses,
        stalemates: n - wins - losses,
        deaths,
        win_rate: round2(win_rate),
        rounds: Spread::of(&rounds),
        monster_dpr: Spread::of(&dpr),
        band: Band::of(win_rate, death_rate),
        flags,
    }
}

/// Lowest level that reads Low, phrased for the ends of the range.
pub fn effective_cr(cells: &[Cell]) -> String {
    let Some(first) = cells.iter().find(|c| c.band == Band::Low) else {
        return match cells.last() {
            Some(c) if c.level == 20 => "over 20".into(),
            Some(c) => format!("over {} (levels run)", c.level),
            None => "unknown".into(),
        };
    };
    if first.level == 1 {
        "around 1 or trivial".into()
    } else {
        first.level.to_string()
    }
}

/// Party level matching the claimed CR, clamped to the levels run, else the middle level.
fn representative_level(spec: &BatchSpec) -> Option<u8> {
    let target = spec.monster.challenge_rating.map(|cr| cr.round().clamp(1.0, 20.0) as u8);
    match target {
        Some(t) => spec.levels.iter().copied().min_by_key(|l| (l.abs_diff(t), *l)),
        None => spec.levels.get(spec.levels.len() / 2).copied(),
    }
}

/// Hybrid sample rule, all ties to the lowest fight index: representative is
/// the fight nearest the representative cell's median rounds; high variance
/// is the fight in that cell furthest from its median DPR; casualty edge is
/// the first fight with a PC death, else a stalemate, else the longest fight.
fn pick_samples(results: &[FightResult], level: Option<u8>) -> Vec<(&'static str, u64)> {
    let Some(level) = level else { return vec![] };
    let in_cell: Vec<&FightResult> = results.iter().filter(|r| r.level == level).collect();
    if in_cell.is_empty() {
        return vec![];
    }
    let rounds = Spread::of(&in_cell.iter().map(|r| f64::from(r.rounds)).collect::<Vec<_>>()).median;
    let dpr = Spread::of(&in_cell.iter().map(|r| r.dpr()).collect::<Vec<_>>()).median;
    let mut chosen: Vec<(&'static str, u64)> = Vec::new();
    let taken = |chosen: &Vec<(&str, u64)>, i: u64| chosen.iter().any(|c| c.1 == i);
    let nearest = in_cell
        .iter()
        .min_by(|a, b| (f64::from(a.rounds) - rounds).abs().total_cmp(&(f64::from(b.rounds) - rounds).abs()).then(a.index.cmp(&b.index)))
        .map(|r| r.index);
    if let Some(i) = nearest {
        chosen.push(("Representative", i));
    }
    let varied = in_cell
        .iter()
        .filter(|r| !taken(&chosen, r.index))
        .max_by(|a, b| (a.dpr() - dpr).abs().total_cmp(&(b.dpr() - dpr).abs()).then(b.index.cmp(&a.index)))
        .map(|r| r.index);
    if let Some(i) = varied {
        chosen.push(("High variance", i));
    }
    let free: Vec<&FightResult> = results.iter().filter(|r| !taken(&chosen, r.index)).collect();
    let edge = free
        .iter()
        .find(|r| r.pc_deaths > 0)
        .or_else(|| free.iter().find(|r| r.outcome == Outcome::Stalemate))
        .or_else(|| free.iter().max_by(|a, b| a.rounds.cmp(&b.rounds).then(b.index.cmp(&a.index))))
        .map(|r| r.index);
    if let Some(i) = edge {
        chosen.push(("Casualty edge", i));
    }
    chosen
}

/// Parse `--levels`: `n` or `a-b` within 1 to 20.
pub fn parse_levels(text: &str) -> Result<Vec<u8>, BatchError> {
    let bad = || BatchError::Invalid(format!("`{text}` is not a level or range such as 5 or 1-20"));
    let (a, b) = match text.split_once('-') {
        Some((a, b)) => (a.trim().parse::<u8>().map_err(|_| bad())?, b.trim().parse::<u8>().map_err(|_| bad())?),
        None => {
            let n = text.trim().parse::<u8>().map_err(|_| bad())?;
            (n, n)
        }
    };
    if a < 1 || b > 20 || a > b {
        return Err(bad());
    }
    Ok((a..=b).collect())
}

/// The standard party, embedded from `data/party/standard/`.
pub fn standard_party() -> Result<Party, BatchError> {
    let blocks = crate::embedded::standard_party_json()
        .map(|(path, text)| serde_json::from_str(text).map_err(|e| BatchError::Invalid(format!("embedded party file {path}: {e}"))))
        .collect::<Result<Vec<StatBlock>, _>>()?;
    group_party(blocks, "the embedded standard party")
}

/// A party folder: one stat block JSON file per member, each with `level`.
/// Members group by level; a folder may cover any subset of levels 1 to 20.
pub fn load_party_dir(dir: &Path) -> Result<Party, BatchError> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| BatchError::NotFound(format!("cannot read party folder {}: {e}", dir.display())))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    let mut blocks = Vec::new();
    for path in files {
        let text = std::fs::read_to_string(&path).map_err(|e| BatchError::Invalid(format!("{}: {e}", path.display())))?;
        blocks.push(serde_json::from_str(&text).map_err(|e| BatchError::Invalid(format!("{} is not a stat block: {e}", path.display())))?);
    }
    group_party(blocks, &format!("party folder {}", dir.display()))
}

fn group_party(blocks: Vec<StatBlock>, origin: &str) -> Result<Party, BatchError> {
    let mut party = Party::new();
    for block in blocks {
        let level = block.level.ok_or_else(|| BatchError::Invalid(format!("{} in {origin} has no `level`", block.id)))?;
        party.entry(level).or_default().push(block);
    }
    for members in party.values_mut() {
        members.sort_by_key(|b| {
            let rank = crate::party::ARCHETYPES.iter().position(|a| b.id.contains(a)).unwrap_or(usize::MAX);
            (rank, b.id.clone())
        });
    }
    if party.is_empty() {
        return Err(BatchError::NotFound(format!("{origin} holds no stat blocks")));
    }
    Ok(party)
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn num(x: f64) -> String {
    format!("{x:.1}")
}

use baseline::cr_word;

const STYLE: &str = ":root { color-scheme: light; --ink:#18212b; --muted:#5d6873; --line:#d7dde3; --panel:#f5f7f9; --blue:#075985; --warn:#7c2d12; --good:#166534; }
    * { box-sizing:border-box; }
    body { margin:0; color:var(--ink); background:#fff; font:16px/1.5 system-ui,-apple-system,BlinkMacSystemFont,\"Segoe UI\",sans-serif; }
    main { max-width:1180px; margin:auto; padding:2rem 1rem 4rem; }
    h1,h2,h3 { line-height:1.2; margin:0 0 .75rem; }
    h1 { font-size:clamp(2rem,4vw,3.2rem); }
    h2 { margin-top:2.5rem; border-bottom:2px solid var(--line); padding-bottom:.45rem; }
    h3 { margin-top:1.5rem; }
    p { max-width:78ch; }
    a { color:var(--blue); text-underline-offset:3px; }
    a:focus-visible { outline:3px solid #f59e0b; outline-offset:3px; }
    .eyebrow { color:var(--muted); text-transform:uppercase; letter-spacing:.08em; font-weight:700; font-size:.8rem; }
    .grid { display:grid; grid-template-columns:repeat(auto-fit,minmax(210px,1fr)); gap:1rem; }
    .card { border:1px solid var(--line); border-radius:8px; padding:1rem; background:var(--panel); }
    .card strong { display:block; font-size:1.45rem; }
    .good { color:var(--good); }
    .warn { color:var(--warn); font-weight:700; }
    table { width:100%; border-collapse:collapse; margin:1rem 0; font-variant-numeric:tabular-nums; }
    caption { text-align:left; font-weight:700; margin-bottom:.5rem; }
    th,td { border:1px solid var(--line); padding:.55rem .65rem; text-align:left; vertical-align:top; }
    th { background:#e9eef2; }
    tbody tr:nth-child(even) { background:#fafbfc; }
    .scroll { overflow-x:auto; }
    code { background:#edf1f4; padding:.1rem .3rem; border-radius:3px; }
    .two { display:grid; grid-template-columns:repeat(auto-fit,minmax(320px,1fr)); gap:1.5rem; }
    dt { font-weight:700; margin-top:.7rem; }
    dd { margin-left:0; color:var(--muted); }
    footer { border-top:1px solid var(--line); margin-top:3rem; padding-top:1rem; color:var(--muted); font-size:.9rem; }";

/// The static report page, in the section order of `docs/prototypes/report-format.html`.
pub fn render_html(s: &Summary) -> String {
    let name = esc(&s.monster_name);
    let mut h = String::new();
    h.push_str(&format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"utf-8\">\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n  <title>Batch report: {name}</title>\n  <style>\n    {STYLE}\n  </style>\n</head>\n<body>\n<main>\n"
    ));
    let levels_text = match (s.levels.first(), s.levels.last()) {
        (Some(a), Some(b)) if a.level != b.level => format!("party levels {} through {}", a.level, b.level),
        (Some(a), _) => format!("party level {}", a.level),
        _ => "no party levels".into(),
    };
    h.push_str(&format!(
        "  <header>\n    <p class=\"eyebrow\">Homebrew Bench batch report</p>\n    <h1>{name}</h1>\n    <p>One {name} against the {} party, {} fights per level, for {levels_text}.</p>\n  </header>\n\n",
        esc(&s.party),
        s.fights_per_level
    ));

    let rep = s.representative_level.and_then(|l| s.levels.iter().find(|c| c.level == l));
    let claimed = s.claimed_cr.map_or("None".to_string(), cr_word);
    let xp = s.claimed_cr.and_then(srd_xp).map_or("No SRD XP reference".to_string(), |xp| format!("SRD XP reference: {}", thousands(xp)));
    let comparison = match (s.claimed_cr, s.effective_cr.parse::<f64>().ok()) {
        (Some(c), Some(e)) if e > c.max(1.0) => "Above claimed CR",
        (Some(c), Some(e)) if e < c.max(1.0) => "Below claimed CR",
        (Some(_), Some(_)) => "Matches claimed CR",
        (Some(_), None) if s.effective_cr.starts_with("over") => "Above claimed CR",
        (Some(_), None) => "Below claimed CR",
        _ => "No claimed CR",
    };
    let comparison = match s.comparison.as_ref().map(|c| c.verdict_kind.as_str()) {
        Some("harder") => "Harder than fixtures",
        Some("easier") => "Easier than fixtures",
        Some("no_threat") => "No threat at this level",
        Some("matches") => "Matches fixtures",
        _ => comparison,
    };
    let rep_text = rep.map_or("No representative level".to_string(), |c| format!("At party level {}, this reads {}.", c.level, c.band.word()));
    let band = rep.map_or("—", |c| c.band.word());
    h.push_str("  <section aria-labelledby=\"headline\">\n    <h2 id=\"headline\">Headline reading</h2>\n    <div class=\"grid\">\n");
    h.push_str(&format!("      <div class=\"card\"><span class=\"eyebrow\">Claimed CR</span><strong>{claimed}</strong><span>{xp}</span></div>\n"));
    h.push_str(&format!("      <div class=\"card\"><span class=\"eyebrow\">Effective CR</span><strong>{}</strong><span>Lowest whole level reading Low</span></div>\n", esc(&s.effective_cr)));
    h.push_str(&format!("      <div class=\"card\"><span class=\"eyebrow\">Claimed-CR comparison</span><strong>{comparison}</strong><span>{rep_text}</span></div>\n"));
    h.push_str(&format!("      <div class=\"card\"><span class=\"eyebrow\">Representative band</span><strong>{band}</strong><span>Project calibration, not an SRD threshold</span></div>\n    </div>\n"));
    for w in &s.warnings {
        h.push_str(&format!("    <p class=\"warn\">Warning: {}</p>\n", esc(w)));
    }
    h.push_str("  </section>\n\n");

    h.push_str(&format!(
        "  <section aria-labelledby=\"levels\">\n    <h2 id=\"levels\">Results by party level</h2>\n    <p>Each row represents {} fights. Median is the primary statistic. Mean follows in parentheses. P10–P90 gives the spread. DPR means total monster damage divided by encounter rounds. Stalemates hit the {}-round cap and count as neither wins nor losses.</p>\n    <div class=\"scroll\">\n      <table>\n        <caption>Results for {name} by party level</caption>\n        <thead><tr><th scope=\"col\">Level</th><th scope=\"col\">Wins</th><th scope=\"col\">Deaths</th><th scope=\"col\">Win rate</th><th scope=\"col\">Rounds</th><th scope=\"col\">Monster DPR</th><th scope=\"col\">Band</th><th scope=\"col\">Flags</th></tr></thead>\n        <tbody>\n",
        s.fights_per_level, s.max_rounds
    ));
    let effective_level: Option<u8> = s.effective_cr.parse().ok().or((s.effective_cr == "around 1 or trivial").then_some(1));
    for c in &s.levels {
        let mut flags = c.flags.clone();
        if Some(c.level) == effective_level {
            flags.push("Effective CR".into());
        }
        let flags = if flags.is_empty() { "—".to_string() } else { esc(&flags.join(", ")) };
        h.push_str(&format!(
            "          <tr><th scope=\"row\">{}</th><td>{} / {}</td><td>{} / {}</td><td>{:.0}%</td><td>{} ({})<br>P10–P90: {}–{}</td><td>{} ({})<br>P10–P90: {}–{}</td><td>{}</td><td>{flags}</td></tr>\n",
            c.level, c.wins, c.fights, c.deaths, c.fights, c.win_rate * 100.0,
            num(c.rounds.median), num(c.rounds.mean), num(c.rounds.p10), num(c.rounds.p90),
            num(c.monster_dpr.median), num(c.monster_dpr.mean), num(c.monster_dpr.p10), num(c.monster_dpr.p90),
            c.band.word()
        ));
    }
    h.push_str("        </tbody>\n      </table>\n    </div>\n  </section>\n\n");

    h.push_str("  <section aria-labelledby=\"cr\">\n    <h2 id=\"cr\">CR comparison and reference facts</h2>\n    <div class=\"two\">\n      <div>\n        <h3 id=\"cr-project\">Project-derived reading</h3>\n        <dl>\n");
    h.push_str("          <dt>Difficulty bands</dt><dd>Low: at least 95% party wins and deaths in under 2% of fights. Moderate: at least 75% wins and deaths in under 15%. High: anything riskier. These are Homebrew Bench calibration thresholds.</dd>\n");
    h.push_str("          <dt>Effective CR</dt><dd>Lowest whole party level reading Low. A monster already Low at level 1 reads around 1 or trivial; one never Low by level 20 reads over 20.</dd>\n");
    match &s.comparison {
        None => h.push_str("          <dt>Baseline</dt><dd>No baseline curve was supplied, so this report compares against party level only. Rounds and DPR flag anomalies; they do not grade CR.</dd>\n"),
        Some(c) => {
            h.push_str(&format!("          <dt>Claimed-CR verdict</dt><dd>{}</dd>\n", esc(&c.verdict)));
            if c.anomalies.is_empty() {
                h.push_str("          <dt>Rounds and damage per round</dt><dd>No level falls outside the fixture spread. These never grade CR.</dd>\n");
            } else {
                let items: Vec<String> = c.anomalies.iter().map(|a| format!("<li>{}</li>", esc(a))).collect();
                h.push_str(&format!("          <dt>Rounds and damage per round</dt><dd>Outside the fixture spread, which flags an anomaly and never grades CR:<ul>{}</ul></dd>\n", items.join("")));
            }
            if !c.thin_cells.is_empty() {
                h.push_str(&format!("          <dt>Thin data</dt><dd>{}</dd>\n", esc(&c.thin_cells.join("; "))));
            }
            h.push_str(&format!("          <dt>Basis</dt><dd>{}</dd>\n", esc(c.basis.trim_start_matches("Basis: "))));
            if !c.stale.is_empty() {
                h.push_str(&format!("          <dt>Stale baseline</dt><dd class=\"warn\">{}</dd>\n", esc(&c.stale.join("; "))));
            }
        }
    }
    h.push_str("        </dl>\n      </div>\n      <div>\n        <h3 id=\"cr-srd\">SRD reference facts</h3>\n        <dl>\n");
    match s.claimed_cr {
        Some(cr) => h.push_str(&format!(
            "          <dt>Claimed CR {}</dt><dd>{}. Monster proficiency bonus: +{}.</dd>\n",
            cr_word(cr),
            srd_xp(cr).map_or("No SRD XP entry".to_string(), |xp| format!("{} XP", thousands(xp))),
            proficiency(cr)
        )),
        None => h.push_str("          <dt>Claimed CR</dt><dd>The stat block states no challenge rating.</dd>\n"),
    }
    if let (Some(level), Some([low, moderate, high])) = (s.srd.level, s.srd.budget) {
        h.push_str(&format!(
            "          <dt>Encounter budget at level {level}</dt><dd>For four characters the SRD budget is {} XP Low, {} Moderate, {} High.{}</dd>\n",
            thousands(low), thousands(moderate), thousands(high), s.srd.within.as_ref().map_or(String::new(), |w| format!(" {}", esc(w)))
        ));
    }
    h.push_str("          <dt>SRD anchor</dt><dd>The SRD says a single monster generally presents a low-difficulty challenge for a party of four characters whose level equals its Challenge Rating. The XP budget above is a separate SRD measure, and the two need not agree.</dd>\n        </dl>\n      </div>\n    </div>\n  </section>\n\n");

    h.push_str("  <section aria-labelledby=\"findings\">\n    <h2 id=\"findings\">Wording and coverage findings</h2>\n");
    if s.findings.is_empty() {
        h.push_str("    <p>Every ability is fully modeled.</p>\n");
    } else {
        h.push_str("    <ul>\n");
        for f in &s.findings {
            let label = match f.kind.as_str() {
                "skipped" => "Skipped mechanic",
                "agent_clause" => "Agent clause",
                "inert" => "Inert ability",
                "lighting" => "Lighting trait",
                "ignored" => "Ignored by assumption",
                "assumed" => "Resolved by assumption",
                _ => "Resolved wording",
            };
            h.push_str(&format!("      <li><strong>{label}:</strong> {} — {}</li>\n", esc(&f.name), esc(&f.note)));
        }
        h.push_str("    </ul>\n");
    }
    if s.findings.iter().any(|f| f.kind == "skipped") {
        h.push_str("    <p class=\"warn\">The verdict remains available, but skipped mechanics qualify it.</p>\n");
    }
    h.push_str("  </section>\n\n");

    h.push_str("  <section aria-labelledby=\"assumptions\">\n    <h2 id=\"assumptions\">Standing assumptions</h2>\n    <table>\n      <caption>Inputs held constant for every fight</caption>\n      <thead><tr><th scope=\"col\">Input</th><th scope=\"col\">Value</th></tr></thead>\n      <tbody>\n");
    let rows = [
        ("Party", format!("{} party; no magic items", s.party)),
        ("Ruleset", "SRD 5.2.1".into()),
        ("Tactics", format!("{} v{}, fingerprint <code>{}</code>", s.policy.id, s.policy.version, s.policy.fingerprint)),
        ("Encounter", format!("One {name} against the party; both sides start near; abstract engagement and range bands; {}-round cap", s.max_rounds)),
        ("Randomness", format!("Master seed <code>{}</code>; {} derived fight seeds per level", s.master_seed, s.fights_per_level)),
        ("Lighting", format!("{} (set by <code>--lighting</code>)", lighting_row(s.lighting))),
    ];
    for (k, v) in rows {
        h.push_str(&format!("        <tr><th scope=\"row\">{k}</th><td>{}</td></tr>\n", if matches!(k, "Tactics" | "Randomness" | "Lighting") { v } else { esc(&v) }));
    }
    if s.standing_assumptions.is_empty() {
        h.push_str("        <tr><th scope=\"row\">Overrides</th><td>None.</td></tr>\n");
    }
    for a in &s.standing_assumptions {
        h.push_str(&format!("        <tr><th scope=\"row\"><code>{}</code></th><td>{} ({})</td></tr>\n", esc(&a.path), esc(&a.value.to_string()), esc(&a.reason)));
    }
    h.push_str("      </tbody>\n    </table>\n  </section>\n\n");

    h.push_str("  <section aria-labelledby=\"repro\">\n    <h2 id=\"repro\">Reproducibility and sampled logs</h2>\n    <p>Samples use the hybrid rule: one representative result, one high-variance result, and one casualty or edge result. Each row carries the fight index, its seed and the command that replays it.</p>\n    <table>\n      <caption>Sampled logs</caption>\n      <thead><tr><th scope=\"col\">Selection</th><th scope=\"col\">Level</th><th scope=\"col\">Fight index</th><th scope=\"col\">Seed</th><th scope=\"col\">Replay</th><th scope=\"col\">Log</th></tr></thead>\n      <tbody>\n");
    for sample in &s.samples {
        h.push_str(&format!(
            "        <tr><th scope=\"row\">{}</th><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code></td><td><a href=\"{}\">Read log</a></td></tr>\n",
            sample.selection, sample.level, sample.index, sample.seed, esc(&sample.replay), esc(&sample.file)
        ));
    }
    h.push_str("      </tbody>\n    </table>\n  </section>\n\n");

    h.push_str(&format!(
        "  <section aria-labelledby=\"method\">\n    <h2 id=\"method\">Method and limitations</h2>\n    <p>Every fight is one {name} against the party at one level, played by the deterministic batch tactics policy for both sides with engine-rolled dice. A fight ends when one side is down or at the round cap. When the monster wins, downed PCs count as deaths. Policy v{} gives buffs and other unscored effects no value, takes Shield or Parry only when it turns a hit into a miss and an Opportunity Attack only when it is worth more than the reaction held back, and agent clauses on any stat block are left out. Engine {}.</p>\n  </section>\n\n",
        s.policy.version, esc(&s.engine_version)
    ));
    h.push_str("  <footer>\n    <p>This work includes material from the System Reference Document 5.2.1 (\"SRD 5.2.1\") by Wizards of the Coast LLC, available at <a href=\"https://www.dndbeyond.com/srd\">https://www.dndbeyond.com/srd</a>. The SRD 5.2.1 is licensed under the <a href=\"https://creativecommons.org/licenses/by/4.0/legalcode\">Creative Commons Attribution 4.0 International License</a>. The SRD-derived material is modified here by reformatting and restructuring into a report. The difficulty bands and results above are Homebrew Bench analysis, not SRD tables.</p>\n  </footer>\n</main>\n</body>\n</html>\n");
    h
}

/// SRD 5.2.1 XP budget per character at a party level: Low, Moderate, High.
fn budget_per_character(level: u8) -> Option<[u64; 3]> {
    const TABLE: [[u64; 3]; 20] = [
        [50, 75, 100], [100, 150, 200], [150, 225, 400], [250, 375, 500], [500, 750, 1100], [600, 1000, 1400],
        [750, 1300, 1700], [1000, 1700, 2100], [1300, 2000, 2600], [1600, 2300, 3100], [1900, 2900, 4100],
        [2200, 3700, 4700], [2600, 4200, 5400], [2900, 4900, 6200], [3300, 5400, 7800], [3800, 6100, 9800],
        [4500, 7200, 11700], [5000, 8700, 14200], [5500, 10700, 17200], [6400, 13200, 22000],
    ];
    TABLE.get(usize::from(level).checked_sub(1)?).copied()
}

fn srd_facts(cr: Option<f64>, level: Option<u8>) -> SrdFacts {
    let xp = cr.and_then(srd_xp);
    let budget = level.and_then(budget_per_character).map(|b| b.map(|x| x * 4));
    let within = match (xp, budget) {
        (Some(xp), Some([low, moderate, high])) => Some(
            match xp {
                x if x <= low => "The claimed CR's XP fits the Low budget.",
                x if x <= moderate => "The claimed CR's XP fits the Moderate budget but not Low.",
                x if x <= high => "The claimed CR's XP fits the High budget but not Moderate.",
                _ => "The claimed CR's XP is over the High budget.",
            }
            .to_string(),
        ),
        _ => None,
    };
    SrdFacts { xp, proficiency_bonus: cr.map(proficiency), level, budget, within }
}

/// SRD 5.2.1 XP by challenge rating.
fn srd_xp(cr: f64) -> Option<u64> {
    const TABLE: [(f64, u64); 34] = [
        (0.0, 10), (0.125, 25), (0.25, 50), (0.5, 100), (1.0, 200), (2.0, 450), (3.0, 700), (4.0, 1100),
        (5.0, 1800), (6.0, 2300), (7.0, 2900), (8.0, 3900), (9.0, 5000), (10.0, 5900), (11.0, 7200),
        (12.0, 8400), (13.0, 10000), (14.0, 11500), (15.0, 13000), (16.0, 15000), (17.0, 18000),
        (18.0, 20000), (19.0, 22000), (20.0, 25000), (21.0, 33000), (22.0, 41000), (23.0, 50000),
        (24.0, 62000), (25.0, 75000), (26.0, 90000), (27.0, 105000), (28.0, 120000), (29.0, 135000), (30.0, 155000),
    ];
    TABLE.iter().find(|(c, _)| (c - cr).abs() < 1e-9).map(|(_, xp)| *xp)
}

fn proficiency(cr: f64) -> u32 {
    match cr {
        c if c < 5.0 => 2,
        c if c < 9.0 => 3,
        c if c < 13.0 => 4,
        c if c < 17.0 => 5,
        c if c < 21.0 => 6,
        c if c < 25.0 => 7,
        c if c < 29.0 => 8,
        _ => 9,
    }
}

fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(id: &str) -> StatBlock {
        serde_json::from_str(crate::embedded::fixture_json(id).unwrap()).unwrap()
    }

    fn spec(monster: StatBlock, levels: Vec<u8>, fights: u32) -> BatchSpec {
        BatchSpec {
            monster,
            party_name: "standard".into(),
            party: standard_party().unwrap(),
            levels,
            fights,
            max_rounds: DEFAULT_MAX_ROUNDS,
            master_seed: 42,
            assumptions: vec![],
            lighting: Lighting::Unspecified,
            baseline: None,
            invocation: None,
        }
    }

    #[test]
    fn standard_party_covers_every_level_in_archetype_order() {
        let party = standard_party().unwrap();
        assert_eq!(party.len(), 20);
        let ids: Vec<&str> = party[&3].iter().map(|b| b.id.as_str()).collect();
        assert_eq!(ids, ["standard-defender-03", "standard-striker-03", "standard-healer-03", "standard-caster-03"]);
    }

    #[test]
    fn fight_index_is_global_and_level_major() {
        assert_eq!(fight_index(1, 0, 100), 0);
        assert_eq!(fight_index(3, 7, 100), 207);
        assert_eq!(level_of(207, 100), 3);
        assert_eq!(level_of(99, 100), 1);
    }

    #[test]
    fn batch_is_reproducible_and_replay_matches() {
        let s = spec(fixture("ogre"), vec![1, 2], 4);
        let a = run(&s).unwrap();
        let b = run(&s).unwrap();
        assert_eq!(a.html, b.html);
        assert_eq!(a.logs, b.logs);
        assert_eq!(serde_json::to_string(&a.summary).unwrap(), serde_json::to_string(&b.summary).unwrap());
        let sample = &a.summary.samples[0];
        let replay = replay_fight(&s, sample.index).unwrap();
        let (_, log) = a.logs.iter().find(|(f, _)| sample.file.ends_with(f.as_str())).unwrap();
        assert_eq!(replay.log.as_deref(), Some(log.as_str()));
        assert!(log.contains("STANDING ASSUMPTIONS") && log.contains("DECISION") && log.contains("RESULT"));
        assert_eq!(a.summary.levels.len(), 2);
        assert_eq!(a.summary.samples.len(), 3);
    }

    #[test]
    fn batch_fights_decide_reaction_windows_and_log_why() {
        let s = spec(fixture("knight"), vec![1], 4);
        let logs: Vec<String> = (0..4).map(|i| replay_fight(&s, fight_index(1, i, 4)).unwrap().log.unwrap()).collect();
        let again: Vec<String> = (0..4).map(|i| replay_fight(&s, fight_index(1, i, 4)).unwrap().log.unwrap()).collect();
        assert_eq!(logs, again);
        let all = logs.join("\n");
        assert!(all.contains("Policy: homebrew-bench.tactics v3 ("));
        assert!(all.contains("    knight: Parry (") || all.contains("    knight: holds its reaction ("), "no Parry window was decided");
        assert!(run(&s).unwrap().html.contains("takes Shield or Parry only when it turns a hit into a miss"));
    }

    #[test]
    fn replay_command_quotes_the_invocation_and_names_the_level_for_partial_parties() {
        let mut s = spec(fixture("ogre"), vec![1], 4);
        assert_eq!(replay_command(&s, 2), "bench batch ogre --seed 42 --fight 2 --fights 4");
        s.invocation = Some(Invocation {
            monster: "ogre".into(),
            party: Some("standard".into()),
            assumptions_file: None,
            assume: vec![],
            assume_reason: None,
        });
        assert_eq!(replay_command(&s, 2), "bench batch ogre --seed 42 --fight 2 --fights 4 --party standard");
        s.invocation = Some(Invocation {
            monster: "brews/my ogre.json".into(),
            party: Some("mine".into()),
            assumptions_file: Some("a.json".into()),
            assume: vec!["abilities.x=ignored".into()],
            assume_reason: Some("the DM's call".into()),
        });
        s.party.retain(|&l, _| l == 3);
        s.party_name = "mine".into();
        assert_eq!(
            replay_command(&s, 9),
            r"bench batch 'brews/my ogre.json' --seed 42 --fight 9 --fights 4 --party mine --levels 3 --assumptions a.json --assume abilities.x=ignored --assume-reason 'the DM'\''s call'"
        );
    }

    #[test]
    fn unresolved_abilities_refuse_and_assumptions_clear_them() {
        let mut monster = fixture("ogre");
        monster.actions[0].agent_clauses = vec!["Something unusual happens.".into()];
        monster.actions[0].default_resolution = None;
        let id = monster.actions[0].id.clone();
        let mut s = spec(monster, vec![1], 1);
        assert_eq!(run(&s).unwrap_err(), BatchError::Unresolved(vec![id.clone()]));
        s.assumptions.push(StandingAssumption { path: format!("abilities.{id}"), value: "ignored".into(), reason: "test".into() });
        let report = run(&s).unwrap();
        assert!(report.summary.findings.iter().any(|f| f.ability_id == id && f.kind == "ignored"));
    }

    #[test]
    fn an_upcast_with_an_unapplied_clause_warns() {
        let mut monster = fixture("ogre");
        monster.actions.push(AbilityEntry {
            id: "acid-arrow".into(),
            name: "Acid Arrow".into(),
            desc: None,
            kind: AbilityKind::Modeled,
            effects: vec![crate::effect::Effect::Cast(crate::effect::Cast {
                spell_id: "acid-arrow".into(),
                slot_level: Some(4),
                upcast: true,
            })],
            agent_clauses: vec![],
            default_resolution: None,
            usage: None,
            trigger: None,
        });
        let report = run(&spec(monster, vec![1], 1)).unwrap();
        let expected = "Spell `acid-arrow` at level 4: The later damage increases by 1d4 for each spell slot level above 2 is not applied; results may understate this monster.";
        assert_eq!(
            report
                .summary
                .warnings
                .iter()
                .filter(|w| *w == expected)
                .count(),
            1,
            "{:?}",
            report.summary.warnings
        );
        assert!(run(&spec(fixture("ogre"), vec![1], 1))
            .unwrap()
            .summary
            .warnings
            .iter()
            .all(|w| !w.contains("is not applied")));
    }

    #[test]
    fn skipped_defaults_warn() {
        let mut monster = fixture("ogre");
        monster.traits.push(AbilityEntry {
            id: "odd-aura".into(),
            name: "Odd Aura".into(),
            desc: None,
            kind: AbilityKind::Unmodeled,
            effects: vec![],
            agent_clauses: vec![],
            default_resolution: Some(DefaultResolution::Skipped),
            usage: None,
            trigger: None,
        });
        let report = run(&spec(monster, vec![1], 1)).unwrap();
        assert!(report.summary.warnings.iter().any(|w| w.contains("Odd Aura")));
        assert!(report.html.contains("Skipped mechanic"));
    }

    #[test]
    fn legendary_actions_spend_in_other_turns() {
        let mut monster = fixture("ogre");
        let mut tail = monster.actions[0].clone();
        tail.id = "tail".into();
        tail.name = "Tail".into();
        monster.legendary_actions.push(tail);
        monster.legendary_uses_per_round = 1;
        let s = spec(monster, vec![5], 1);
        let fight = replay_fight(&s, fight_index(5, 0, 1)).unwrap();
        let log = fight.log.unwrap();
        assert!(log.contains("legendary action: tail"), "{log}");
        assert!(log.contains("LEGENDARY ACTION ogre: Tail (1 of 1)"), "{log}");
    }

    #[test]
    fn the_creature_up_next_spends_before_its_budget_resets() {
        let mut monster = fixture("ogre");
        let mut tail = monster.actions[0].clone();
        tail.id = "tail".into();
        tail.name = "Tail".into();
        monster.legendary_actions.push(tail);
        monster.legendary_uses_per_round = 1;
        let s = spec(monster, vec![5], 1);
        let mut entrants = vec![Entrant { stat_block: s.monster.clone(), side: MONSTER_SIDE.into(), band: RangeBand::Near }];
        entrants.extend(s.party[&5].iter().map(|pc| Entrant { stat_block: pc.clone(), side: PARTY_SIDE.into(), band: RangeBand::Near }));
        let (mut state, _) = live::start(StartSpec {
            id: "t".into(),
            origin_directory: String::new(),
            entrants,
            seed: 3,
            surprised: vec![],
            individual_initiative: true,
            pc_dice: PcDice::Engine,
            lighting: Lighting::Unspecified,
            reaction_windows: false,
        })
        .unwrap();
        let ogre = state.combatants.iter().find(|c| c.side_id != PARTY_SIDE).unwrap().id.clone();
        let before = state.initiative_order.iter().position(|id| id == &ogre).unwrap();
        let len = state.initiative_order.len();
        state.active_turn = Some(state.initiative_order[(before + len - 1) % len].clone());
        let party: Vec<String> = state.combatants.iter().filter(|c| c.side_id == PARTY_SIDE).map(|c| c.id.clone()).collect();
        for c in state.combatants.iter_mut() {
            let others: Vec<String> = if c.id == ogre { party.clone() } else { vec![ogre.clone()] };
            for other in others {
                c.ranges.insert(other.clone(), RangeBand::Engaged);
                c.engagement.push(other);
            }
        }
        let mut log = Log::new(true);
        let mut damage = 0;
        end_turn(&mut state, &PolicyConstants::default(), &mut log, &mut damage).unwrap();
        let log = log.finish().unwrap();
        assert!(log.contains("LEGENDARY ACTION"), "{log}");
        assert_eq!(state.active_turn.as_deref(), Some(ogre.as_str()));
        let ogre = state.combatants.iter().find(|c| c.id == ogre).unwrap();
        assert!(!ogre.resources.keys().any(|k| k.ends_with(":legendary")), "the spend came before the reset: {:?}", ogre.resources);
    }

    #[test]
    fn bad_assumption_paths_are_rejected() {
        let mut s = spec(fixture("ogre"), vec![1], 1);
        s.assumptions.push(StandingAssumption { path: "hp".into(), value: 3.into(), reason: "x".into() });
        assert!(matches!(run(&s), Err(BatchError::Invalid(_))));
    }

    #[test]
    fn bands_and_effective_cr() {
        assert_eq!(Band::of(0.96, 0.01), Band::Low);
        assert_eq!(Band::of(0.96, 0.05), Band::Moderate);
        assert_eq!(Band::of(0.7, 0.0), Band::High);
        let s = spec(fixture("ogre"), vec![1, 2], 2);
        let report = run(&s).unwrap();
        assert!(!report.summary.effective_cr.is_empty());
    }

    #[test]
    fn html_follows_prototype_order_with_unique_ids() {
        let report = run(&spec(fixture("ogre"), vec![1, 2], 2)).unwrap();
        let html = &report.html;
        let order = ["id=\"headline\"", "id=\"levels\"", "id=\"cr\"", "id=\"findings\"", "id=\"assumptions\"", "id=\"repro\"", "id=\"method\"", "<footer>"];
        let positions: Vec<usize> = order.iter().map(|m| html.find(m).unwrap_or_else(|| panic!("{m} missing"))).collect();
        assert!(positions.windows(2).all(|w| w[0] < w[1]));
        let mut ids: Vec<&str> = html.match_indices(" id=\"").map(|(i, _)| html[i + 5..].split('"').next().unwrap()).collect();
        let n = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n, "duplicate ids");
        assert!(html.contains("<html lang=\"en\">") && html.contains("<main>") && !html.contains("<script"));
        assert_eq!(html.matches("<caption>").count(), html.matches("<table>").count());
    }

    fn lit_run(monster: &str, lighting: Lighting) -> BatchReport {
        let mut s = spec(fixture(monster), vec![1], 1);
        s.assumptions = baseline::ignoring_unresolved(&s.monster).1;
        s.lighting = lighting;
        s.baseline = Some(baseline::embedded().unwrap().unwrap());
        run(&s).unwrap()
    }

    fn finding<'a>(report: &'a BatchReport, id: &str) -> &'a Finding {
        report.summary.findings.iter().find(|f| f.ability_id == id).unwrap()
    }

    #[test]
    fn the_default_lighting_is_stated_and_turns_lighting_traits_off() {
        let report = lit_run("kobold-warrior", Lighting::Unspecified);
        assert!(report.html.contains("<th scope=\"row\">Lighting</th><td>unspecified, lighting traits off (set by <code>--lighting</code>)</td>"), "{}", report.html);
        assert!(report.logs.iter().all(|(_, log)| log.contains("STANDING ASSUMPTIONS\n  standard batch rules: one monster against the party, both sides start near, round cap counts as a stalemate\n  lighting = unspecified\n")));
        assert_eq!(serde_json::to_value(&report.summary).unwrap()["lighting"], "unspecified");
        assert_eq!(finding(&report, "sunlight-sensitivity").note, "Applies in sunlight; off in this run.");
        assert_eq!(finding(&report, "sunlight-sensitivity").kind, "lighting");
        assert!(report.summary.samples.iter().all(|s| !s.replay.contains("--lighting")));
        assert!(!report.summary.comparison.unwrap().basis.contains("lighting unspecified"));
    }

    #[test]
    fn sunlight_is_stated_and_turns_sunlight_traits_on() {
        let report = lit_run("kobold-warrior", Lighting::Sunlight);
        assert!(report.html.contains("<td>sunlight (also Bright Light) (set by <code>--lighting</code>)</td>"));
        assert!(report.logs.iter().all(|(_, log)| log.contains("  lighting = sunlight\n")));
        assert_eq!(serde_json::to_value(&report.summary).unwrap()["lighting"], "sunlight");
        assert_eq!(finding(&report, "sunlight-sensitivity").note, "Applies in sunlight; on in this run.");
        assert!(report.summary.samples.iter().all(|s| s.replay.ends_with(" --lighting sunlight")), "{:?}", report.summary.samples);
        assert!(report.summary.comparison.unwrap().basis.ends_with(" The baseline curve's fixtures fought with lighting unspecified."));
    }

    #[test]
    fn dim_and_bright_light_rows_and_the_cloaker_line() {
        assert!(lit_run("cloaker", Lighting::DimLight).html.contains("<td>dim light (set by"));
        let bright = lit_run("cloaker", Lighting::BrightLight);
        assert!(bright.html.contains("<td>bright light (set by"));
        assert_eq!(finding(&bright, "light-sensitivity").note, "Applies in Bright Light; on in this run.");
    }

    #[test]
    fn a_vampire_spawn_in_sunlight_reports_its_lighting_and_its_clause() {
        let report = lit_run("vampire-spawn", Lighting::Sunlight);
        let sunlight = finding(&report, "sunlight");
        assert_eq!(sunlight.kind, "skipped");
        assert_eq!(
            sunlight.note,
            "Applies in sunlight; on in this run. Agent judgment left out: The vampire takes 20 Radiant damage if it starts its turn in sunlight."
        );
        assert!(report.summary.warnings.iter().any(|w| w.starts_with("Skipped mechanic: Sunlight (sunlight)")));
    }
}
