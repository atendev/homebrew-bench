//! Live encounters: one command in, one log entry and one response out.
//!
//! `start` builds an encounter and its initial snapshot. `execute` gates a
//! command on the encounter's pending prompt and end state, applies it,
//! appends it to the command history, and writes its log entry. `undo` and
//! `replay` re-execute the history from the initial snapshot, so the state is
//! always a fold over the commands that made it.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::digest::{self, wounded};
use crate::encounter::{
    ActionEconomy, CommandRecord, Combatant, EncounterEnd, EncounterSettings, EncounterState, EndState,
    PcDice, PendingPrompt, PlayerDice, RangeBand, Side, StandingAssumption, StatBlockSnapshot, PARTY_SIDE,
};
use crate::engine::{self, EngineError, EngineEvent, InitiativeOptions, LegendaryUse, OpportunityAttack, StepRequest};
use crate::log::{LogEntry, LogLine};
use crate::override_path;
use crate::reaction::{ReactionAnswer, ReactionDecider, ReactionWindow, Reactions, WindowTrigger};
use crate::rng::{RngState, Stream};
use crate::stat_block::StatBlock;
use crate::effect::Lighting;
use crate::ENGINE_VERSION;

/// One mutating live command, as stored in the command history.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Command {
    Action {
        actor: String,
        action: String,
        #[serde(default)]
        targets: Vec<String>,
        /// Spell slot level for every leveled Cast in the action.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        slot: Option<u8>,
        /// Spend the extra action a modifier grants (Haste) instead of the action.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        extra: bool,
    },
    Move {
        actor: String,
        #[serde(default)]
        target: Option<String>,
        band: RangeBand,
        #[serde(default)]
        feet: Option<u32>,
        /// Reactors that take an Opportunity Attack on this move, in order.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        opportunity: Vec<OpportunityAttack>,
    },
    Respond {
        #[serde(default)]
        values: Vec<u32>,
        #[serde(default)]
        decline: bool,
        /// Take the offered reaction with this id; empty when only one is offered.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        take: Option<String>,
    },
    Disengage {
        actor: String,
        /// Spend the extra action a modifier grants (Haste) instead of the action.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        extra: bool,
    },
    /// End the active turn. Legendary actions listed here resolve in order
    /// between the end of this turn and the start of the next.
    EndTurn {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        legendary: Vec<LegendaryUse>,
    },
    End {
        reason: String,
    },
    Override {
        path: String,
        /// New value, or `None` to clear the field.
        #[serde(default)]
        value: Option<Value>,
        reason: String,
        #[serde(default)]
        rule: Option<String>,
        /// Record the override as a standing assumption and tag the lines it affects.
        #[serde(default)]
        standing: bool,
    },
    Roll {
        expression: String,
        reason: String,
        #[serde(default)]
        rule: Option<String>,
    },
    Add {
        stat_block: Box<StatBlock>,
        side: String,
        #[serde(default)]
        band: Option<RangeBand>,
        #[serde(default)]
        engaged_with: Option<String>,
    },
    Remove {
        id: String,
        reason: String,
    },
    Undo,
}

impl Command {
    /// An end-turn with no legendary actions.
    pub fn end_turn() -> Self {
        Command::EndTurn { legendary: vec![] }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Command::Action { .. } => "action",
            Command::Move { .. } => "move",
            Command::Disengage { .. } => "disengage",
            Command::Respond { .. } => "respond",
            Command::EndTurn { .. } => "end-turn",
            Command::End { .. } => "end",
            Command::Override { .. } => "override",
            Command::Roll { .. } => "roll",
            Command::Add { .. } => "add",
            Command::Remove { .. } => "remove",
            Command::Undo => "undo",
        }
    }

    fn reason(&self) -> Option<String> {
        match self {
            Command::End { reason }
            | Command::Override { reason, .. }
            | Command::Roll { reason, .. }
            | Command::Remove { reason, .. } => Some(reason.clone()),
            _ => None,
        }
    }

    fn user_dice(&self) -> Vec<u32> {
        match self {
            Command::Respond { values, .. } => values.clone(),
            _ => vec![],
        }
    }
}

/// Why a live command was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveError {
    /// Bad input: unknown dice expression, bad override value, and so on.
    Invalid(String),
    /// A combatant, action or stat block does not exist.
    NotFound(String),
    /// The rules forbid the command right now.
    Illegal(String),
    /// A pending prompt must be answered first.
    Pending(String),
    /// The encounter has ended.
    Ended(String),
}

impl std::fmt::Display for LiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(s) | Self::NotFound(s) | Self::Illegal(s) | Self::Pending(s) | Self::Ended(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for LiveError {}

impl From<EngineError> for LiveError {
    fn from(err: EngineError) -> Self {
        match err {
            EngineError::NotFound(s) => Self::NotFound(s),
            EngineError::Invalid(s) => Self::Invalid(s),
            EngineError::Pending(s) => Self::Pending(s),
            EngineError::Illegal(s) => Self::Illegal(s),
        }
    }
}

/// One event in a response, with its ready-made log line.
#[derive(Debug, Clone, Serialize)]
pub struct EventOut {
    #[serde(flatten)]
    pub event: EngineEvent,
    pub line: String,
}

/// The response to every live command.
#[derive(Debug, Clone, Serialize)]
pub struct Outcome {
    pub encounter: String,
    pub events: Vec<EventOut>,
    pub digest: Value,
    pub pending: Option<PendingPrompt>,
    pub log_entry: usize,
}

/// One combatant entering at `bench start`.
#[derive(Debug, Clone)]
pub struct Entrant {
    pub stat_block: StatBlock,
    pub side: String,
    pub band: RangeBand,
}

/// Everything `bench start` needs to build an encounter.
#[derive(Debug, Clone)]
pub struct StartSpec {
    pub id: String,
    pub origin_directory: String,
    pub entrants: Vec<Entrant>,
    pub seed: u64,
    /// Combatant ids or side ids that start surprised.
    pub surprised: Vec<String>,
    pub individual_initiative: bool,
    pub pc_dice: PcDice,
    pub lighting: Lighting,
    /// Open reaction windows (ADR 0018). `bench start` sets it; batch leaves it off.
    pub reaction_windows: bool,
    /// Open spell-cast windows (ADR 0019).
    pub spell_cast_windows: bool,
    /// A teleport leaves the pairs it frees at Near (ADR 0005).
    pub teleport_near: bool,
    /// Minutes, hours and days count against the round clock (ADR 0005).
    pub long_durations: bool,
}

/// Result of `bench replay`.
#[derive(Debug, Clone, Serialize)]
pub struct ReplayReport {
    pub commands: usize,
    pub matches: bool,
    /// Index into the command history of the first command whose re-derived
    /// state differs from the recorded one.
    pub first_divergence: Option<usize>,
    /// Log entry number of that command, as `bench log` and `UNDO` lines count.
    pub first_divergence_log_entry: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub divergence_error: Option<String>,
    pub recorded_engine_version: String,
    pub running_engine_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_warning: Option<String>,
}

struct Applied {
    items: Vec<(EngineEvent, LogLine)>,
}

impl Applied {
    fn none() -> Self {
        Self { items: vec![] }
    }

    fn line(kind: &str, actor: Option<&str>, line: LogLine) -> Self {
        let mut applied = Self::none();
        applied.push(kind, actor, line);
        applied
    }

    fn push(&mut self, kind: &str, actor: Option<&str>, line: LogLine) {
        self.items.push((
            EngineEvent {
                kind: kind.into(),
                message: line.text.clone(),
                actor_id: actor.map(str::to_string),
                target_id: None,
                roll: None,
                total: None,
                damage: None,
                player_dice: false,
            },
            line,
        ));
    }
}

/// Build a new encounter, roll initiative, and write log entry 0.
pub fn start(spec: StartSpec) -> Result<(EncounterState, Outcome), LiveError> {
    if spec.entrants.is_empty() {
        return Err(LiveError::Invalid("an encounter needs at least one combatant".into()));
    }
    let mut snapshots: Vec<StatBlockSnapshot> = Vec::new();
    let mut counts: HashMap<String, usize> = HashMap::new();
    for entrant in &spec.entrants {
        *counts.entry(entrant.stat_block.id.clone()).or_default() += 1;
        add_snapshot(&mut snapshots, &entrant.stat_block)?;
    }
    let mut taken: HashSet<String> = counts.iter().filter(|(_, &n)| n == 1).map(|(id, _)| id.clone()).collect();
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut sides: Vec<Side> = Vec::new();
    let mut combatants = Vec::new();
    let mut bands = Vec::new();
    for entrant in &spec.entrants {
        if entrant.band == RangeBand::Engaged {
            return Err(LiveError::Invalid("starting bands are near or far; engage with `bench move`".into()));
        }
        let block = &entrant.stat_block;
        let (id, name) = if counts[&block.id] > 1 {
            let n = seen.entry(block.id.clone()).or_default();
            *n += 1;
            while taken.contains(&format!("{}-{n}", block.id)) {
                *n += 1;
            }
            let id = format!("{}-{n}", block.id);
            taken.insert(id.clone());
            (id, format!("{} {n}", block.name))
        } else {
            (block.id.clone(), block.name.clone())
        };
        if !sides.iter().any(|s| s.id == entrant.side) {
            sides.push(side(&entrant.side));
        }
        combatants.push(new_combatant(id, name, block, &entrant.side));
        bands.push(entrant.band);
    }
    for i in 0..combatants.len() {
        for j in 0..combatants.len() {
            if combatants[i].side_id != combatants[j].side_id {
                let band = farther(bands[i], bands[j]);
                let other = combatants[j].id.clone();
                combatants[i].ranges.insert(other, band);
            }
        }
    }
    let mut surprised = Vec::new();
    for name in &spec.surprised {
        let matched: Vec<String> = combatants
            .iter()
            .filter(|c| &c.id == name || &c.side_id == name)
            .map(|c| c.id.clone())
            .collect();
        if matched.is_empty() {
            return Err(LiveError::NotFound(format!("surprised `{name}` is neither a combatant nor a side")));
        }
        surprised.extend(matched);
    }

    let mut state = EncounterState {
        id: spec.id.clone(),
        engine_version: ENGINE_VERSION.into(),
        origin_directory: spec.origin_directory.clone(),
        stat_blocks: snapshots,
        sides,
        combatants,
        round: 0,
        initiative_order: vec![],
        active_turn: None,
        active_effects: vec![],
        active_modifiers: vec![],
        standing_assumptions: vec![],
        command_history: vec![],
        pending: None,
        rng: RngState::new(spec.seed),
        initiative_rolls: HashMap::new(),
        settings: EncounterSettings { pc_dice: spec.pc_dice, lighting: spec.lighting, reaction_windows: spec.reaction_windows, spell_cast_windows: spec.spell_cast_windows, teleport_near: spec.teleport_near, long_durations: spec.long_durations },
        ended: None,
        initial_snapshot: None,
        log: vec![],
        tamper_hash: String::new(),
        player_dice: PlayerDice::default(),
        reactions: Default::default(),
    };
    let mut applied = Applied::none();
    let pc_dice = match spec.pc_dice {
        PcDice::Engine => "engine",
        PcDice::Ask => "ask",
    };
    let lighting = spec.lighting.as_str();
    applied.push(
        "start",
        None,
        LogLine::replaced(
            format!("START encounter {} (seed {}, pc dice {pc_dice}, lighting {lighting}, engine {ENGINE_VERSION})", spec.id, spec.seed),
            format!("START encounter {} (pc dice {pc_dice}, lighting {lighting}, engine {ENGINE_VERSION})", spec.id),
        ),
    );
    for c in &state.combatants {
        let band = state.combatants.iter().find_map(|o| c.ranges.get(&o.id)).copied().unwrap_or_default();
        let text = format!("{} ({}) on {}, {}", c.id, c.name, c.side_id, band_word(band));
        let line = if state.is_party(&c.id) {
            LogLine::same(format!("{text}, {}/{} HP", c.hp, c.max_hp))
        } else {
            LogLine::same(text)
        };
        applied.push("combatant", Some(&c.id), line);
    }
    if !surprised.is_empty() {
        applied.push("surprise", None, LogLine::same(format!("surprised: {}", surprised.join(", "))));
    }
    let initiative = run_engine(
        &mut state,
        Held::new(StepRequest::RollInitiative(InitiativeOptions {
            individual: spec.individual_initiative,
            surprised: surprised.clone(),
        })),
    )?;
    applied.items.extend(initiative.items);
    state.initial_snapshot = Some(Box::new(state.bare()));
    let outcome = write_entry(&mut state, None, applied, 0, None);
    Ok((state, outcome))
}

/// Run one mutating command against a loaded encounter.
pub fn execute(state: &mut EncounterState, cmd: Command) -> Result<Outcome, LiveError> {
    if matches!(cmd, Command::Undo) {
        return undo(state);
    }
    if let Some(end) = &state.ended {
        return Err(LiveError::Ended(format!(
            "the encounter ended in round {} ({}); only undo and read-only commands work",
            end.round, end.reason
        )));
    }
    if let Some(prompt) = &state.pending {
        if !matches!(cmd, Command::Respond { .. } | Command::Override { .. }) {
            return Err(LiveError::Pending(format!(
                "answer the pending prompt `{}` with `bench respond` first: {}",
                prompt.id, prompt.message
            )));
        }
    }
    let cmd = with_sole_option(state, cmd);
    let round = state.round;
    let turn = state.active_turn.clone();
    let applied = apply(state, &cmd)?;
    Ok(write_entry(state, Some(&cmd), applied, round, turn))
}

/// `respond --take` with no id names the one reaction on offer, so the history
/// records which reaction was taken.
fn with_sole_option(state: &EncounterState, cmd: Command) -> Command {
    let Command::Respond { values, decline, take: Some(id) } = cmd else { return cmd };
    let sole = state
        .pending
        .as_ref()
        .filter(|p| p.kind == "reaction" && id.is_empty())
        .and_then(|p| serde_json::from_value::<Held>(p.metadata.clone()).ok())
        .and_then(|held| held.window)
        .filter(|w| w.options.len() == 1)
        .map(|w| w.options[0].id.clone());
    Command::Respond { values, decline, take: Some(sole.unwrap_or(id)) }
}

/// Re-execute the command history and check each command's state hash.
pub fn replay(state: &EncounterState) -> ReplayReport {
    let version_warning = (state.engine_version != ENGINE_VERSION).then(|| {
        format!(
            "recorded with engine {} and replayed with {ENGINE_VERSION}; rules changes can make the replay diverge",
            state.engine_version
        )
    });
    let mut report = ReplayReport {
        commands: state.command_history.len(),
        matches: false,
        first_divergence: None,
        first_divergence_log_entry: None,
        divergence_error: None,
        recorded_engine_version: state.engine_version.clone(),
        running_engine_version: ENGINE_VERSION.into(),
        version_warning,
    };
    let Some(initial) = state.initial_snapshot.as_deref() else {
        report.first_divergence = Some(0);
        report.divergence_error = Some("the encounter has no initial snapshot".into());
        return report;
    };
    match fold(initial, &state.command_history) {
        Ok(folded) => {
            report.first_divergence = folded
                .hashes
                .iter()
                .zip(&state.command_history)
                .position(|(hash, record)| *hash != record.state_hash);
            if report.first_divergence.is_none() && folded.state.game_hash() != state.game_hash() {
                report.first_divergence = Some(state.command_history.len().saturating_sub(1));
            }
        }
        Err((index, err)) => {
            report.first_divergence = Some(index);
            report.divergence_error = Some(err.to_string());
        }
    }
    report.matches = report.first_divergence.is_none();
    report.first_divergence_log_entry = report.first_divergence.map(|i| i + 1);
    report
}

/// The current digest, DM or public.
pub fn digest_of(state: &EncounterState, public: bool) -> Value {
    if public {
        digest::public_digest(state)
    } else {
        digest::digest(state)
    }
}

fn undo(state: &mut EncounterState) -> Result<Outcome, LiveError> {
    let initial = state
        .initial_snapshot
        .as_deref()
        .ok_or_else(|| LiveError::Invalid("the encounter has no initial snapshot to undo from".into()))?;
    let mut history = state.command_history.clone();
    history.push(record_for(&Command::Undo, String::new()));
    let folded = fold(initial, &history).map_err(|(_, err)| err)?;
    let reverted = folded.reverted.ok_or_else(|| LiveError::Invalid("there is no command to undo".into()))?;
    let reverted_name = state.command_history[reverted].name.clone();

    let round = state.round;
    let turn = state.active_turn.clone();
    let mut next = folded.state;
    next.initial_snapshot = state.initial_snapshot.take();
    next.log = std::mem::take(&mut state.log);
    next.command_history = std::mem::take(&mut state.command_history);
    *state = next;
    let applied = Applied::line(
        "undo",
        None,
        LogLine::same(format!("UNDO reverts entry [{}] ({reverted_name})", reverted + 1)),
    );
    Ok(write_entry(state, Some(&Command::Undo), applied, round, turn))
}

struct Folded {
    state: EncounterState,
    hashes: Vec<String>,
    reverted: Option<usize>,
}

fn fold(initial: &EncounterState, history: &[CommandRecord]) -> Result<Folded, (usize, LiveError)> {
    let mut state = initial.bare();
    let mut stack: Vec<(EncounterState, usize)> = Vec::new();
    let mut hashes = Vec::with_capacity(history.len());
    let mut reverted = None;
    for (index, record) in history.iter().enumerate() {
        let cmd: Command = serde_json::from_value(record.args.clone())
            .map_err(|e| (index, LiveError::Invalid(format!("history entry {index} is unreadable: {e}"))))?;
        if matches!(cmd, Command::Undo) {
            let (previous, undone) = stack
                .pop()
                .ok_or((index, LiveError::Invalid("there is no command to undo".into())))?;
            state = previous;
            reverted = Some(undone);
        } else {
            stack.push((state.clone(), index));
            apply(&mut state, &cmd).map_err(|e| (index, e))?;
        }
        hashes.push(state.game_hash());
    }
    Ok(Folded { state, hashes, reverted })
}

fn record_for(cmd: &Command, state_hash: String) -> CommandRecord {
    CommandRecord {
        name: cmd.name().into(),
        args: serde_json::to_value(cmd).unwrap_or(Value::Null),
        user_dice: cmd.user_dice(),
        reason: cmd.reason(),
        state_hash,
    }
}

fn write_entry(
    state: &mut EncounterState,
    cmd: Option<&Command>,
    applied: Applied,
    round: u32,
    turn: Option<String>,
) -> Outcome {
    let state_hash = state.game_hash();
    if let Some(cmd) = cmd {
        state.command_history.push(record_for(cmd, state_hash.clone()));
    }
    let index = state.log.len();
    state.log.push(LogEntry {
        index,
        command: cmd.map_or("start", Command::name).into(),
        round,
        turn,
        lines: applied.items.iter().map(|(_, line)| line.clone()).collect(),
        state_hash,
    });
    state.update_tamper_hash();
    Outcome {
        encounter: state.id.clone(),
        events: applied
            .items
            .into_iter()
            .map(|(event, line)| EventOut { event, line: line.text })
            .collect(),
        digest: digest::digest(state),
        pending: state.pending.clone(),
        log_entry: index,
    }
}

fn apply(state: &mut EncounterState, cmd: &Command) -> Result<Applied, LiveError> {
    match cmd {
        Command::Action {
            actor,
            action,
            targets,
            slot,
            extra,
        } => run_engine(
            state,
            Held::new(StepRequest::Action {
                actor_id: actor.clone(),
                action_id: action.clone(),
                target_ids: targets.clone(),
                slot_level: *slot,
                extra: *extra,
            }),
        ),
        Command::Move { actor, target, band, feet, opportunity } => {
            let feet = match feet {
                Some(f) => *f,
                None => remaining_speed(state, actor)?,
            };
            run_engine(
                state,
                Held::new(StepRequest::Move {
                    actor_id: actor.clone(),
                    target_id: target.clone(),
                    band: *band,
                    feet,
                    opportunity: opportunity.clone(),
                    declined: vec![],
                }),
            )
        }
        Command::Disengage { actor, extra } => run_engine(state, Held::new(StepRequest::Disengage { actor_id: actor.clone(), extra: *extra })),
        Command::EndTurn { legendary } => run_engine(state, Held::new(StepRequest::EndTurn { legendary: legendary.clone() })),
        Command::Respond { values, decline, take } => respond(state, values, *decline, take.as_deref()),
        Command::End { reason } => {
            state.ended = Some(EncounterEnd { round: state.round, reason: reason.clone() });
            Ok(Applied::line(
                "end",
                None,
                LogLine::same(format!("END encounter ended by agent judgment: {reason}")),
            ))
        }
        Command::Override { path, value, reason, rule, standing } => atomically(state, |state| {
            let previous = state.combatants.clone();
            let before: Vec<(String, bool, Option<String>)> = previous
                .iter()
                .map(|c| (c.id.clone(), engine::cannot_concentrate(c), c.concentration.clone()))
                .collect();
            let target = override_path::apply(state, path, value.clone()).map_err(LiveError::Invalid)?;
            let count = |list: &[Combatant], id: &str| list.iter().filter(|c| c.id == id).count();
            if let Some(twice) = state.combatants.iter().find(|c| {
                let now = count(&state.combatants, &c.id);
                now > 1 && now > count(&previous, &c.id)
            }) {
                return Err(LiveError::Invalid(format!(
                    "{} is in combatants twice; each combatant needs its own id, so use `bench add` for another one",
                    twice.id
                )));
            }
            if let Some(added) = state.combatants.iter().find(|c| !previous.iter().any(|p| p.id == c.id)) {
                return Err(LiveError::Invalid(format!(
                    "{} is not in the encounter; an override can't add a combatant, use `bench add`, which rolls its initiative",
                    added.id
                )));
            }
            let removed: Vec<&Combatant> = previous.iter().filter(|c| !state.combatants.iter().any(|d| d.id == c.id)).collect();
            if let (Some(prompt), Some(first)) = (&state.pending, removed.first()) {
                return Err(LiveError::Pending(format!(
                    "answer the pending prompt `{}` with `bench respond` before removing {}: {}",
                    prompt.id, first.id, prompt.message
                )));
            }
            if let Some(active) = state.active_turn.as_deref().filter(|a| removed.iter().any(|c| c.id == *a)) {
                return Err(LiveError::Invalid(format!(
                    "{active} holds the active turn; use `bench remove {active}`, which ends the turn first"
                )));
            }
            let mut concentration_events = Vec::new();
            for leaving in &removed {
                state.combatants.push((*leaving).clone());
                concentration_events.extend(engine::end_concentration_on_leaving(state, std::slice::from_ref(&leaving.id))?);
                state.combatants.retain(|c| c.id != leaving.id);
                forget_combatant(state, &leaving.id);
            }
            let dropped: Vec<String> = state
                .combatants
                .iter()
                .filter(|c| c.concentration.is_some() && engine::cannot_concentrate(c))
                .filter(|c| before.iter().find(|(id, ..)| id == &c.id).map_or(true, |(_, unable, held)| !unable || held != &c.concentration))
                .map(|c| c.id.clone())
                .collect();
            concentration_events.extend(engine::end_concentration(state, &dropped)?);
            let mut tag = rule_tag(rule);
            state.standing_assumptions.retain(|a| &a.path != path);
            if *standing {
                state.standing_assumptions.push(StandingAssumption {
                    path: path.clone(),
                    value: value.clone().unwrap_or(Value::Null),
                    reason: reason.clone(),
                });
                tag.push_str(" [standing assumption]");
            }
            let change = match value {
                Some(v) => format!("{path} = {}", compact(v)),
                None => format!("{path} cleared"),
            };
            let text = format!("OVERRIDE {change} (reason: {reason}){tag}");
            let line = if target.hides_value(state) {
                LogLine::replaced(text, format!("OVERRIDE {path} (reason: {reason}){tag}"))
            } else {
                LogLine::same(text)
            };
            let mut applied = Applied::line("override", target.combatant.as_deref(), line);
            for event in concentration_events {
                let line = event_line(state, &event).tagged(&assumption_tag(state, &event));
                applied.items.push((event, line));
            }
            Ok(applied)
        }),
        Command::Roll { expression, reason, rule } => {
            let mut stream = Stream::from_state(state.rng.clone());
            let (total, faces) = roll_expression(&mut stream, expression)?;
            state.rng = stream.state();
            let faces = faces.iter().map(u32::to_string).collect::<Vec<_>>().join(", ");
            let mut applied = Applied::line(
                "agent_roll",
                None,
                LogLine::same(format!(
                    "AGENT JUDGMENT roll {expression} = {total} [{faces}] (reason: {reason}){}",
                    rule_tag(rule)
                )),
            );
            applied.items[0].0.total = Some(total);
            Ok(applied)
        }
        Command::Add { stat_block, side: side_id, band, engaged_with } => {
            if state.settings.pc_dice == PcDice::Ask && stat_block.class.is_some() {
                let mut dry_run = state.clone();
                add_combatant(&mut dry_run, stat_block, side_id, *band, engaged_with.as_deref(), Some(1))?;
                let owner = dry_run.combatants.last().map(|c| c.id.clone()).unwrap_or_default();
                return Ok(add_prompt(state, cmd, &owner));
            }
            add_combatant(state, stat_block, side_id, *band, engaged_with.as_deref(), None)
        }
        Command::Remove { id, reason } => atomically(state, |state| remove_combatant(state, id, reason)),
        Command::Undo => Err(LiveError::Invalid("undo is not replayable as a single step".into())),
    }
}

/// A request a pending prompt holds, with everything its rerun needs. Stored as
/// the prompt's metadata; fields at their defaults are left out, so dice
/// prompts read and write as they did before reaction windows.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Held {
    #[serde(deserialize_with = "held_request")]
    request: StepRequest,
    #[serde(default)]
    faces: Vec<u32>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    engine_fills_rest: bool,
    /// Hit-window answers so far, in window order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    answers: Vec<ReactionAnswer>,
    /// Reaction windows answered so far in this command.
    #[serde(default, skip_serializing_if = "is_zero")]
    answered: u32,
    /// The window a reaction prompt offers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    window: Option<ReactionWindow>,
}

impl Held {
    fn new(request: StepRequest) -> Self {
        Self { request, faces: vec![], engine_fills_rest: false, answers: vec![], answered: 0, window: None }
    }
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

fn held_request<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<StepRequest, D::Error> {
    stored_request(&Value::deserialize(deserializer)?).map_err(serde::de::Error::custom)
}

/// Resolve an engine request on a copy. If a PC roll found no face, or a
/// reaction window found no answer, the copy is discarded and a prompt holds
/// the request until `bench respond` answers it and it runs again. Prompts come
/// in the order the engine met them.
fn run_engine(state: &mut EncounterState, held: Held) -> Result<Applied, LiveError> {
    let mut trial = state.clone();
    if !matches!(held.request, StepRequest::Respond { .. }) {
        trial.pending = None;
    }
    trial.player_dice = PlayerDice { faces: held.faces.clone().into(), engine_fills_rest: held.engine_fills_rest, ..Default::default() };
    trial.reactions = Reactions { decider: ReactionDecider::Queued(held.answers.clone().into()), ..Default::default() };
    let response = engine::step(&mut trial, held.request.clone())?;
    if let Some(window) = trial.reactions.waiting.take() {
        return Ok(reaction_prompt(state, held, window));
    }
    if let Some((sides, owner, count)) = trial.player_dice.next_prompt() {
        let message = format!("{owner}: enter {count} d{sides} face{}", if count == 1 { "" } else { "s" });
        state.pending = Some(PendingPrompt {
            id: format!("dice-{}-{}", state.rng.cursor, held.faces.len()),
            kind: "dice".into(),
            actor_id: owner.clone(),
            message: message.clone(),
            sides: Some(sides),
            remaining: count,
            metadata: serde_json::to_value(&held).unwrap_or_default(),
        });
        return Ok(Applied::line("prompt", Some(&owner), LogLine::same(format!("PROMPT {message}"))));
    }
    trial.player_dice = PlayerDice::default();
    trial.reactions = Reactions::default();
    *state = trial;
    let mut applied = Applied::none();
    for event in response.events {
        let line = event_line(state, &event).tagged(&assumption_tag(state, &event));
        applied.items.push((event, line));
    }
    Ok(applied)
}

/// Hold the request on a `reaction` prompt for one window. A monster's window
/// stays out of the public view unless it takes the reaction.
fn reaction_prompt(state: &mut EncounterState, held: Held, window: ReactionWindow) -> Applied {
    let options = window.options.iter().map(|o| o.id.as_str()).collect::<Vec<_>>();
    let message = match &window.trigger {
        WindowTrigger::Opportunity { mover_id } => {
            format!("{}: Opportunity Attack on {mover_id} ({})?", window.reactor_id, options.join(", "))
        }
        WindowTrigger::Hit { attacker_id, total, ac, critical } => format!(
            "{}: hit by {attacker_id} (attack {total} vs AC {ac}{}), take {}?",
            window.reactor_id,
            if *critical { ", critical hit" } else { "" },
            options.join(" or ")
        ),
        WindowTrigger::SpellCast { caster_id, spell_id, level, .. } => format!(
            "{}: {caster_id} is casting {spell_id} (level {level}), take {}?",
            window.reactor_id,
            options.join(" or ")
        ),
    };
    let reactor = window.reactor_id.clone();
    let line = format!("PROMPT {message}");
    let line = if state.is_party(&reactor) { LogLine::same(line) } else { LogLine::hidden(line) };
    state.pending = Some(PendingPrompt {
        id: format!("reaction-{}-{}", state.rng.cursor, held.answered),
        kind: "reaction".into(),
        actor_id: reactor.clone(),
        message,
        sides: None,
        remaining: 1,
        metadata: serde_json::to_value(Held { window: Some(window), ..held }).unwrap_or_default(),
    });
    Applied::line("prompt", Some(&reactor), line)
}

fn respond(state: &mut EncounterState, values: &[u32], decline: bool, take: Option<&str>) -> Result<Applied, LiveError> {
    let prompt = state.pending.clone().ok_or_else(|| LiveError::Invalid("there is no pending prompt".into()))?;
    if prompt.kind == "reaction" {
        return respond_reaction(state, &prompt, values, decline, take);
    }
    if take.is_some() {
        return Err(LiveError::Invalid(format!("--take answers a reaction prompt; `{}` asks for dice", prompt.id)));
    }
    if prompt.kind != "dice" {
        let mut applied = run_engine(state, Held::new(StepRequest::Respond { values: values.to_vec(), accept: !decline }))?;
        let answer = if decline { "declined".to_string() } else { format!("answered {values:?}") };
        applied.items.insert(0, prompt_line(&prompt, &answer, false));
        return Ok(applied);
    }
    let sides = prompt.sides.unwrap_or(0);
    if !decline {
        if values.len() != prompt.remaining as usize {
            return Err(LiveError::Invalid(format!("the prompt asks for {} d{sides} faces, got {}", prompt.remaining, values.len())));
        }
        if let Some(bad) = values.iter().find(|v| **v == 0 || **v > sides) {
            return Err(LiveError::Invalid(format!("{bad} is not a face of a d{sides}")));
        }
    }
    let answer = if decline {
        "declined, the engine rolls the rest".to_string()
    } else {
        format!("answered {} (player dice)", values.iter().map(u32::to_string).collect::<Vec<_>>().join(", "))
    };
    if let Some(add) = prompt.metadata.get("add") {
        let Command::Add { stat_block, side, band, engaged_with } = serde_json::from_value(add.clone()).map_err(unreadable)? else {
            return Err(LiveError::Invalid("the pending prompt does not hold an add".into()));
        };
        let face = if decline { None } else { values.first().copied() };
        state.pending = None;
        let mut applied = add_combatant(state, &stat_block, &side, band, engaged_with.as_deref(), face)?;
        applied.items.insert(0, prompt_line(&prompt, &answer, false));
        return Ok(applied);
    }
    let mut held: Held = serde_json::from_value(prompt.metadata.clone()).map_err(unreadable)?;
    if !decline {
        held.faces.extend_from_slice(values);
    }
    held.engine_fills_rest |= decline;
    state.pending = None;
    let mut applied = run_engine(state, held)?;
    applied.items.insert(0, prompt_line(&prompt, &answer, false));
    Ok(applied)
}

fn unreadable(e: serde_json::Error) -> LiveError {
    LiveError::Invalid(format!("the pending prompt is unreadable: {e}"))
}

/// Answer a reaction window. An Opportunity Attack answer joins the held move's
/// `opportunity` or `declined` list; a hit answer joins the queued answers.
/// Either way the held request runs again from the state before the command.
fn respond_reaction(
    state: &mut EncounterState,
    prompt: &PendingPrompt,
    values: &[u32],
    decline: bool,
    take: Option<&str>,
) -> Result<Applied, LiveError> {
    if !values.is_empty() {
        return Err(LiveError::Invalid("a reaction prompt takes `--take [<id>]` or `--decline`, not dice faces".into()));
    }
    let mut held: Held = serde_json::from_value(prompt.metadata.clone()).map_err(unreadable)?;
    let window = held.window.take().ok_or_else(|| LiveError::Invalid("the pending prompt holds no reaction window".into()))?;
    let listed = || window.options.iter().map(|o| o.id.as_str()).collect::<Vec<_>>().join(", ");
    let answer = match (decline, take) {
        (true, Some(_)) => return Err(LiveError::Invalid("choose `--take` or `--decline`, not both".into())),
        (false, None) => return Err(LiveError::Invalid(format!("answer with `--take [<id>]` or `--decline`; the options are {}", listed()))),
        (true, None) => ReactionAnswer::Pass,
        (false, Some("")) if window.options.len() == 1 => ReactionAnswer::Take(window.options[0].id.clone()),
        (false, Some("")) => return Err(LiveError::Invalid(format!("several reactions are on offer; name one with `--take <id>`: {}", listed()))),
        (false, Some(id)) if window.option(id).is_some() => ReactionAnswer::Take(id.into()),
        (false, Some(id)) => return Err(LiveError::Invalid(format!("{id} is not on offer; the options are {}", listed()))),
    };
    match (&window.trigger, &mut held.request) {
        (WindowTrigger::Opportunity { .. }, StepRequest::Move { opportunity, declined, .. }) => match &answer {
            ReactionAnswer::Take(id) => opportunity.push(OpportunityAttack { reactor_id: window.reactor_id.clone(), attack_id: Some(id.clone()) }),
            ReactionAnswer::Pass => declined.push(window.reactor_id.clone()),
        },
        (WindowTrigger::Opportunity { .. }, _) => return Err(LiveError::Invalid("an Opportunity Attack prompt must hold a move".into())),
        (WindowTrigger::Hit { .. } | WindowTrigger::SpellCast { .. }, _) => held.answers.push(answer.clone()),
    }
    held.answered += 1;
    state.pending = None;
    let (text, hidden) = match &answer {
        ReactionAnswer::Take(id) => (format!("answered: takes {}", window.option(id).map_or(id.as_str(), |o| o.name.as_str())), false),
        ReactionAnswer::Pass => ("declined".to_string(), !state.is_party(&window.reactor_id)),
    };
    let mut applied = run_engine(state, held)?;
    applied.items.insert(0, prompt_line(prompt, &text, hidden));
    Ok(applied)
}

/// The request a prompt holds. Prompts saved before end-turn took
/// legendary actions hold a bare `"end_turn"`.
fn stored_request(value: &Value) -> Result<StepRequest, serde_json::Error> {
    if value.as_str() == Some("end_turn") {
        return Ok(StepRequest::end_turn());
    }
    serde_json::from_value(value.clone())
}

/// Hold a PC's `bench add` until the user enters its initiative d20.
fn add_prompt(state: &mut EncounterState, cmd: &Command, owner: &str) -> Applied {
    let message = format!("{owner}: enter 1 d20 face");
    state.pending = Some(PendingPrompt {
        id: format!("dice-{}-0", state.rng.cursor),
        kind: "dice".into(),
        actor_id: owner.into(),
        message: message.clone(),
        sides: Some(20),
        remaining: 1,
        metadata: serde_json::json!({ "add": cmd }),
    });
    Applied::line("prompt", Some(owner), LogLine::same(format!("PROMPT {message}")))
}

fn prompt_line(prompt: &PendingPrompt, answer: &str, hidden: bool) -> (EngineEvent, LogLine) {
    let text = format!("PROMPT {} {answer}", prompt.message);
    let line = if hidden { LogLine::hidden(text.clone()) } else { LogLine::same(text.clone()) };
    (
        EngineEvent {
            kind: "prompt_answered".into(),
            message: text.clone(),
            actor_id: Some(prompt.actor_id.clone()),
            target_id: None,
            roll: None,
            total: None,
            damage: None,
            player_dice: false,
        },
        line,
    )
}

fn event_line(state: &EncounterState, e: &EngineEvent) -> LogLine {
    let mut text = e.message.clone();
    if let Some(roll) = e.roll {
        text.push_str(&format!(" [d20 {roll}]"));
    }
    if e.player_dice {
        text.push_str(" (player dice)");
    }
    let target = e.target_id.as_deref().unwrap_or_default();
    let monster_target = !target.is_empty() && !state.is_party(target);
    let word = state.combatants.iter().find(|c| c.id == target).map(|c| wounded(c).word());
    match (e.kind.as_str(), monster_target, word) {
        ("damage", true, Some(word)) => {
            LogLine::replaced(text, format!("{target} takes damage ({word})"))
        }
        ("heal", true, Some(word)) => LogLine::replaced(text, format!("{target} regains HP ({word})")),
        ("temp_hp", true, Some(_)) => LogLine::replaced(text, format!("{target} gains temporary HP")),
        ("recharge", _, _) if !state.is_party(e.actor_id.as_deref().unwrap_or_default()) => LogLine::hidden(text),
        ("legendary", _, _) if !state.is_party(e.actor_id.as_deref().unwrap_or_default()) => {
            let public = text.rsplit_once(" (").map_or(text.as_str(), |(head, _)| head).to_string();
            LogLine::replaced(text, public)
        }
        _ => LogLine::same(text),
    }
}

/// Names the standing assumptions whose subject is the event's actor or target.
fn assumption_tag(state: &EncounterState, e: &EngineEvent) -> String {
    let involved = |id: &str| [e.actor_id.as_deref(), e.target_id.as_deref()].contains(&Some(id));
    let paths: Vec<&str> = state
        .standing_assumptions
        .iter()
        .filter(|a| {
            let mut segments = a.path.split('.');
            match (segments.next(), segments.next()) {
                (Some("stat_blocks"), Some(block)) => {
                    state.combatants.iter().any(|c| c.stat_block_id == block && involved(&c.id))
                }
                (Some(first), _) => involved(first),
                _ => false,
            }
        })
        .map(|a| a.path.as_str())
        .collect();
    if paths.is_empty() {
        String::new()
    } else {
        format!(" [standing assumption: {}]", paths.join(", "))
    }
}

fn add_combatant(
    state: &mut EncounterState,
    block: &StatBlock,
    side_id: &str,
    band: Option<RangeBand>,
    engaged_with: Option<&str>,
    face: Option<u32>,
) -> Result<Applied, LiveError> {
    if band.is_some() && engaged_with.is_some() {
        return Err(LiveError::Invalid("give either a band or --engaged-with, not both".into()));
    }
    if band == Some(RangeBand::Engaged) {
        return Err(LiveError::Invalid("use --engaged-with <id> to add a combatant in melee".into()));
    }
    if let Some(other) = engaged_with {
        let target = state
            .combatants
            .iter()
            .find(|c| c.id == other)
            .ok_or_else(|| LiveError::NotFound(format!("combatant {other} not found")))?;
        if target.side_id == side_id {
            return Err(LiveError::Invalid(format!("{other} is on the same side")));
        }
    }
    add_snapshot(&mut state.stat_blocks, block)?;
    let taken = |id: &str| state.combatants.iter().any(|c| c.id == id);
    let (id, name) = if !taken(&block.id) && !taken(&format!("{}-1", block.id)) {
        (block.id.clone(), block.name.clone())
    } else {
        let n = (2..).find(|n| !taken(&format!("{}-{n}", block.id))).expect("free id");
        (format!("{}-{n}", block.id), format!("{} {n}", block.name))
    };
    if !state.sides.iter().any(|s| s.id == side_id) {
        state.sides.push(side(side_id));
    }
    let mut combatant = new_combatant(id.clone(), name.clone(), block, side_id);
    let band = band.unwrap_or(RangeBand::Near);
    let opponents: Vec<String> = state
        .combatants
        .iter()
        .filter(|c| c.side_id != side_id)
        .map(|c| c.id.clone())
        .collect();
    for other in &opponents {
        let here = match engaged_with {
            Some(e) if e == other => RangeBand::Engaged,
            Some(_) => RangeBand::Near,
            None => band,
        };
        combatant.ranges.insert(other.clone(), here);
        if let Some(o) = state.combatants.iter_mut().find(|c| &c.id == other) {
            o.ranges.insert(id.clone(), here);
            if here == RangeBand::Engaged {
                o.engagement.push(id.clone());
                combatant.engagement.push(other.clone());
            }
        }
    }

    let mut stream = Stream::from_state(state.rng.clone());
    let drawn = stream.roll_die(20);
    let natural = face.unwrap_or(drawn);
    state.rng = stream.state();
    let total = natural as i32 + block.initiative;
    let slot = state
        .initiative_order
        .iter()
        .position(|other| state.initiative_rolls.get(other).copied().unwrap_or(i32::MIN) < total)
        .unwrap_or(state.initiative_order.len());
    state.initiative_order.insert(slot, id.clone());
    state.initiative_rolls.insert(id.clone(), total);
    state.combatants.push(combatant);

    let position = match engaged_with {
        Some(other) => format!("engaged with {other}"),
        None => band_word(band).into(),
    };
    let mut applied = Applied::line(
        "add",
        Some(&id),
        LogLine::same(format!(
            "ADD {id} ({name}) joins {side_id}, initiative {total}, {position}{}",
            if face.is_some() { " (player dice)" } else { "" }
        )),
    );
    applied.items[0].0.player_dice = face.is_some();
    applied.items[0].0.roll = Some(natural);
    applied.items[0].0.total = Some(total);
    Ok(applied)
}

fn atomically(
    state: &mut EncounterState,
    f: impl FnOnce(&mut EncounterState) -> Result<Applied, LiveError>,
) -> Result<Applied, LiveError> {
    let mut next = state.clone();
    let applied = f(&mut next)?;
    *state = next;
    Ok(applied)
}

fn remove_combatant(state: &mut EncounterState, id: &str, reason: &str) -> Result<Applied, LiveError> {
    if !state.combatants.iter().any(|c| c.id == id) {
        return Err(LiveError::NotFound(format!("combatant {id} not found")));
    }
    let mut applied = Applied::line("remove", Some(id), LogLine::same(format!("REMOVE {id} (reason: {reason})")));
    if state.active_turn.as_deref() == Some(id) && state.initiative_order.len() > 1 {
        let turn = run_engine(state, Held { engine_fills_rest: true, ..Held::new(StepRequest::end_turn()) })?;
        applied.items.extend(turn.items);
    }
    for event in engine::end_concentration_on_leaving(state, &[id.to_string()])? {
        let line = event_line(state, &event).tagged(&assumption_tag(state, &event));
        applied.items.push((event, line));
    }
    state.combatants.retain(|c| c.id != id);
    forget_combatant(state, id);
    Ok(applied)
}

fn forget_combatant(state: &mut EncounterState, id: &str) {
    state.initiative_order.retain(|c| c != id);
    state.initiative_rolls.remove(id);
    for c in &mut state.combatants {
        c.engagement.retain(|e| e != id);
        c.ranges.remove(id);
    }
    if state.active_turn.as_deref() == Some(id) {
        state.active_turn = None;
    }
}

fn add_snapshot(snapshots: &mut Vec<StatBlockSnapshot>, block: &StatBlock) -> Result<(), LiveError> {
    let snapshot = StatBlockSnapshot::from_stat_block(block.id.clone(), block.clone());
    match snapshots.iter().find(|s| s.id == block.id) {
        Some(existing) if existing.hash == snapshot.hash => Ok(()),
        Some(_) => Err(LiveError::Invalid(format!(
            "two different stat blocks share the id `{}`; give one a new id",
            block.id
        ))),
        None => {
            snapshots.push(snapshot);
            Ok(())
        }
    }
}

fn new_combatant(id: String, name: String, block: &StatBlock, side_id: &str) -> Combatant {
    Combatant {
        id,
        stat_block_id: block.id.clone(),
        name,
        side_id: side_id.into(),
        hp: block.hit_points.average,
        temp_hp: 0,
        max_hp: block.hit_points.average,
        conditions: vec![],
        concentration: None,
        resources: HashMap::new(),
        action_economy: ActionEconomy::default(),
        engagement: vec![],
        ranges: HashMap::new(),
        end_state: if block.hit_points.average > 0 { EndState::Active } else { EndState::Dead },
        death_saves: block.death_saves.clone(),
    }
}

fn side(id: &str) -> Side {
    let name = match id {
        PARTY_SIDE => "Party".to_string(),
        "monsters" => "Monsters".to_string(),
        other => other.to_string(),
    };
    Side { id: id.into(), name }
}

fn farther(a: RangeBand, b: RangeBand) -> RangeBand {
    let rank = |band| match band {
        RangeBand::Engaged => 0,
        RangeBand::Near => 1,
        RangeBand::Far => 2,
    };
    if rank(a) >= rank(b) {
        a
    } else {
        b
    }
}

fn band_word(band: RangeBand) -> &'static str {
    match band {
        RangeBand::Engaged => "engaged",
        RangeBand::Near => "near",
        RangeBand::Far => "far",
    }
}

fn remaining_speed(state: &EncounterState, actor: &str) -> Result<u32, LiveError> {
    let c = state
        .combatants
        .iter()
        .find(|c| c.id == actor)
        .ok_or_else(|| LiveError::NotFound(format!("combatant {actor} not found")))?;
    Ok(crate::engine::walk_speed(state, c).saturating_sub(c.action_economy.movement_used_feet))
}

fn rule_tag(rule: &Option<String>) -> String {
    rule.as_ref().map(|r| format!(" [house rule: {r}]")).unwrap_or_default()
}

fn compact(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Roll a dice expression such as `2d6+3`, `d20` or `1d4-1+1d6`.
pub fn roll_expression(stream: &mut Stream, expression: &str) -> Result<(i32, Vec<u32>), LiveError> {
    let bad = || LiveError::Invalid(format!("`{expression}` is not a dice expression like 2d6+3"));
    let compact: String = expression.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.is_empty() {
        return Err(bad());
    }
    let mut terms = Vec::new();
    let mut sign = 1;
    let mut current = String::new();
    let mut signed = false;
    for ch in compact.chars() {
        if ch == '+' || ch == '-' {
            if !current.is_empty() {
                terms.push((sign, std::mem::take(&mut current)));
            } else if signed || !terms.is_empty() {
                return Err(bad());
            }
            signed = true;
            sign = if ch == '-' { -1 } else { 1 };
        } else {
            signed = false;
            current.push(ch);
        }
    }
    if current.is_empty() {
        return Err(bad());
    }
    terms.push((sign, current));
    let mut total = 0i32;
    let mut faces = Vec::new();
    for (sign, term) in terms {
        if let Some((count, sides)) = term.split_once(['d', 'D']) {
            let count: u32 = if count.is_empty() { 1 } else { count.parse().map_err(|_| bad())? };
            let sides: u32 = sides.parse().map_err(|_| bad())?;
            if count == 0 || count > 1000 || sides == 0 {
                return Err(bad());
            }
            for _ in 0..count {
                let face = stream.roll_die(sides);
                faces.push(face);
                total += sign * face as i32;
            }
        } else {
            let flat: i32 = term.parse().map_err(|_| bad())?;
            total += sign * flat;
        }
    }
    Ok((total, faces))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reaction::{ReactionWindow, WindowTrigger};

    fn block(rel: &str) -> StatBlock {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data").join(rel);
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
    }

    fn spec(pc_dice: PcDice) -> StartSpec {
        StartSpec {
            id: "test-1".into(),
            origin_directory: "/tmp".into(),
            entrants: vec![
                Entrant { stat_block: block("party/standard/standard-striker-03.json"), side: PARTY_SIDE.into(), band: RangeBand::Near },
                Entrant { stat_block: block("fixtures/goblin-warrior.json"), side: "monsters".into(), band: RangeBand::Near },
            ],
            seed: 42,
            surprised: vec![],
            individual_initiative: true,
            pc_dice,
            lighting: Lighting::Unspecified,
            reaction_windows: true,
            spell_cast_windows: true,
            teleport_near: true,
            long_durations: true,
        }
    }

    fn attack(state: &EncounterState) -> Command {
        let actor = state.active_turn.clone().expect("active turn");
        let (action, target) = if actor == "standard-striker-03" {
            ("longbow", "goblin-warrior")
        } else {
            ("shortbow", "standard-striker-03")
        };
        Command::Action {
            actor,
            action: action.into(),
            targets: vec![target.into()],
            slot: None,
            extra: false,
        }
    }

    fn lines(outcome: &Outcome) -> Vec<String> {
        outcome.events.iter().map(|e| e.line.clone()).collect()
    }

    fn caster_spec() -> StartSpec {
        let mut spec = spec(PcDice::Engine);
        spec.entrants.insert(
            0,
            Entrant {
                stat_block: block("party/standard/standard-caster-05.json"),
                side: PARTY_SIDE.into(),
                band: RangeBand::Near,
            },
        );
        spec
    }

    fn turn_of(state: &mut EncounterState, id: &str) {
        while state.active_turn.as_deref() != Some(id) {
            execute(state, Command::end_turn()).unwrap();
        }
    }

    fn cast_at(actor: &str, action: &str, slot: Option<u8>) -> Command {
        Command::Action {
            actor: actor.into(),
            action: action.into(),
            targets: vec!["goblin-warrior".into()],
            slot,
            extra: false,
        }
    }

    const CASTER: &str = "standard-caster-05";

    #[test]
    fn slot_spends_the_named_slot_and_scales_the_spell() {
        let (mut state, _) = start(caster_spec()).unwrap();
        execute(
            &mut state,
            Command::Override {
                path: "goblin-warrior.hp".into(),
                value: Some(Value::from(500)),
                reason: "test".into(),
                rule: None,
                standing: false,
            },
        )
        .unwrap();
        turn_of(&mut state, CASTER);
        let outcome = execute(&mut state, cast_at(CASTER, "magic-missile", Some(3))).unwrap();
        let darts = outcome
            .events
            .iter()
            .filter(|e| e.event.kind == "damage")
            .count();
        assert_eq!(darts, 5, "{:?}", lines(&outcome));
        let caster = state.combatants.iter().find(|c| c.id == CASTER).unwrap();
        assert_eq!(caster.resources.get(&format!("{CASTER}:slot:3")), Some(&1));
        assert_eq!(caster.resources.get(&format!("{CASTER}:slot:1")), None);
        assert!(replay(&state).matches);
    }

    #[test]
    fn slot_refusals_give_their_reason() {
        let (mut state, _) = start(caster_spec()).unwrap();
        turn_of(&mut state, CASTER);
        let refused =
            |state: &mut EncounterState, cmd: Command| execute(state, cmd).unwrap_err().to_string();
        assert!(refused(&mut state, cast_at(CASTER, "fire-bolt", Some(1)))
            .contains("casts no leveled spell"));
        assert!(refused(&mut state, cast_at(CASTER, "fireball", Some(2))).contains("level 3 spell"));
        assert!(
            refused(&mut state, cast_at(CASTER, "magic-missile", Some(4)))
                .contains("no level 4 spell slot left")
        );
        turn_of(&mut state, "standard-striker-03");
        assert!(refused(
            &mut state,
            cast_at("standard-striker-03", "longbow", Some(1))
        )
        .contains("has no spell slots"));
    }

    #[test]
    fn a_history_without_a_slot_replays() {
        let (mut state, _) = start(caster_spec()).unwrap();
        turn_of(&mut state, CASTER);
        execute(&mut state, cast_at(CASTER, "magic-missile", None)).unwrap();
        let record = state.command_history.last().unwrap();
        assert!(
            record.args.to_string().contains("magic-missile")
                && !record.args.to_string().contains("slot"),
            "{}",
            record.args
        );
        assert!(replay(&state).matches);
    }

    #[test]
    fn public_view_hides_monster_temp_hp_amounts() {
        let (state, _) = start(spec(PcDice::Engine)).unwrap();
        let gained = |target: &str| EngineEvent {
            kind: "temp_hp".into(),
            message: format!("{target} gains 7 temporary HP"),
            actor_id: Some(target.into()),
            target_id: Some(target.into()),
            roll: None,
            total: None,
            damage: None,
            player_dice: false,
        };
        let monster = event_line(&state, &gained("goblin-warrior"));
        assert_eq!(monster, LogLine::replaced("goblin-warrior gains 7 temporary HP", "goblin-warrior gains temporary HP"));
        let pc = event_line(&state, &gained("standard-striker-03"));
        assert_eq!(pc, LogLine::same("standard-striker-03 gains 7 temporary HP"));
    }

    #[test]
    fn undo_restores_the_dice_stream() {
        let (mut state, _) = start(spec(PcDice::Engine)).unwrap();
        let first = { let cmd = attack(&state); execute(&mut state, cmd) }.unwrap();
        let undone = execute(&mut state, Command::Undo).unwrap();
        assert!(undone.events[0].line.starts_with("UNDO reverts entry [1] (action)"), "{:?}", lines(&undone));
        let again = { let cmd = attack(&state); execute(&mut state, cmd) }.unwrap();
        assert_eq!(lines(&first), lines(&again));
        assert_eq!(state.command_history.len(), 3);
        assert!(replay(&state).matches);
    }

    #[test]
    fn replay_matches_and_finds_divergence() {
        let (mut state, _) = start(spec(PcDice::Engine)).unwrap();
        { let cmd = attack(&state); execute(&mut state, cmd) }.unwrap();
        execute(&mut state, Command::end_turn()).unwrap();
        execute(
            &mut state,
            Command::Override {
                path: "goblin-warrior.hp".into(),
                value: Some(Value::from(3)),
                reason: "test".into(),
                rule: None,
                standing: false,
            },
        )
        .unwrap();
        execute(&mut state, Command::Roll { expression: "1d6".into(), reason: "luck".into(), rule: None }).unwrap();
        let report = replay(&state);
        assert!(report.matches, "{report:?}");
        assert_eq!(report.commands, 4);

        state.command_history[1].state_hash = "bogus".into();
        assert_eq!(replay(&state).first_divergence, Some(1));
    }

    #[test]
    fn ask_mode_prompts_for_pc_initiative_before_round_one() {
        let striker = block("party/standard/standard-striker-03.json");
        let (mut state, started) = start(spec(PcDice::Ask)).unwrap();
        let prompt = started.pending.clone().expect("initiative prompt");
        assert_eq!((prompt.kind.as_str(), prompt.actor_id.as_str(), prompt.sides, prompt.remaining), ("dice", "standard-striker-03", Some(20), 1));
        assert!(lines(&started).iter().any(|l| l.starts_with("PROMPT ")), "{:?}", lines(&started));
        assert_eq!((state.round, state.active_turn.clone()), (0, None));
        assert!(matches!(execute(&mut state, Command::end_turn()), Err(LiveError::Pending(_))));

        let answered = execute(&mut state, Command::Respond { values: vec![17], decline: false, take: None }).unwrap();
        assert!(state.pending.is_none());
        assert_eq!(state.initiative_rolls["standard-striker-03"], 17 + striker.initiative);
        assert_eq!(state.round, 1);
        assert!(state.active_turn.is_some());
        assert!(lines(&answered).iter().any(|l| l.contains("answered 17 (player dice)")), "{:?}", lines(&answered));
        let rolled = format!("standard-striker-03 rolled {} (player dice)", 17 + striker.initiative);
        assert!(lines(&answered).contains(&rolled), "{:?}", lines(&answered));
        assert!(replay(&state).matches);

        execute(&mut state, Command::Undo).unwrap();
        assert_eq!(state.pending.as_ref().map(|p| p.id.clone()), Some(prompt.id));
    }

    #[test]
    fn surprised_pc_enters_both_initiative_dice_and_keeps_the_lower() {
        let striker = block("party/standard/standard-striker-03.json");
        let mut surprised = spec(PcDice::Ask);
        surprised.surprised = vec!["standard-striker-03".into()];
        let (mut state, started) = start(surprised).unwrap();
        assert_eq!(started.pending.as_ref().map(|p| p.remaining), Some(2));
        execute(&mut state, Command::Respond { values: vec![15, 4], decline: false, take: None }).unwrap();
        assert_eq!(state.initiative_rolls["standard-striker-03"], 4 + striker.initiative);
        assert!(replay(&state).matches);
    }

    #[test]
    fn repeated_standing_override_replaces_the_assumption() {
        let (mut state, _) = start(spec(PcDice::Engine)).unwrap();
        for hp in [5, 3] {
            execute(
                &mut state,
                Command::Override {
                    path: "goblin-warrior.hp".into(),
                    value: Some(Value::from(hp)),
                    reason: format!("hp {hp}"),
                    rule: None,
                    standing: true,
                },
            )
            .unwrap();
        }
        assert_eq!(state.standing_assumptions.len(), 1);
        assert_eq!(state.standing_assumptions[0].value, Value::from(3));
        let fought = { let cmd = attack(&state); execute(&mut state, cmd) }.unwrap();
        assert!(lines(&fought).iter().all(|l| l.ends_with(" [standing assumption: goblin-warrior.hp]")), "{:?}", lines(&fought));
    }

    #[test]
    fn declined_pc_initiative_is_not_marked_as_player_dice() {
        let mut two = spec(PcDice::Ask);
        let striker = two.entrants[0].clone();
        two.entrants.insert(1, striker);
        let (mut state, started) = start(two).unwrap();
        assert_eq!(started.pending.as_ref().map(|p| p.actor_id.as_str()), Some("standard-striker-03-1"));
        execute(&mut state, Command::Respond { values: vec![12], decline: false, take: None }).unwrap();
        assert_eq!(state.pending.as_ref().map(|p| p.actor_id.as_str()), Some("standard-striker-03-2"));
        let declined = execute(&mut state, Command::Respond { values: vec![], decline: true, take: None }).unwrap();
        let line = |id: &str| lines(&declined).into_iter().find(|l| l.starts_with(&format!("{id} rolled "))).expect(id);
        assert!(line("standard-striker-03-1").ends_with("(player dice)"), "{:?}", lines(&declined));
        assert!(!line("standard-striker-03-2").contains("(player dice)"), "{:?}", lines(&declined));
    }

    #[test]
    fn declined_damage_after_an_entered_attack_roll_is_not_marked() {
        let (mut state, _) = start(spec(PcDice::Ask)).unwrap();
        execute(&mut state, Command::Respond { values: vec![10], decline: false, take: None }).unwrap();
        if state.active_turn.as_deref() != Some("standard-striker-03") {
            execute(&mut state, Command::end_turn()).unwrap();
        }
        let asked = { let cmd = attack(&state); execute(&mut state, cmd) }.unwrap();
        let d20s = asked.pending.expect("attack roll prompt").remaining as usize;
        let hit = execute(&mut state, Command::Respond { values: vec![20; d20s], decline: false, take: None }).unwrap();
        assert_eq!(hit.pending.as_ref().map(|p| p.kind.as_str()), Some("dice"), "{:?}", lines(&hit));
        let resolved = lines(&execute(&mut state, Command::Respond { values: vec![], decline: true, take: None }).unwrap());
        let attack_line = resolved.iter().find(|l| l.contains(" attacks ")).expect("attack line");
        assert!(attack_line.ends_with("(player dice)"), "{resolved:?}");
        let damage_line = resolved.iter().find(|l| l.starts_with("goblin-warrior takes ")).expect("damage line");
        assert!(!damage_line.contains("(player dice)"), "{resolved:?}");
    }

    #[test]
    fn grouped_ask_initiative_marks_every_pc_sharing_the_roll() {
        let mut grouped = spec(PcDice::Ask);
        let striker = grouped.entrants[0].clone();
        grouped.entrants.insert(1, striker);
        grouped.individual_initiative = false;
        let (mut state, started) = start(grouped).unwrap();
        assert_eq!(started.pending.as_ref().map(|p| p.remaining), Some(1));
        let answered = lines(&execute(&mut state, Command::Respond { values: vec![13], decline: false, take: None }).unwrap());
        assert!(state.pending.is_none());
        for id in ["standard-striker-03-1", "standard-striker-03-2"] {
            let line = answered.iter().find(|l| l.starts_with(&format!("{id} rolled "))).expect(id);
            assert!(line.ends_with("(player dice)"), "{answered:?}");
        }
        let goblin = answered.iter().find(|l| l.starts_with("goblin-warrior rolled ")).unwrap();
        assert!(!goblin.contains("(player dice)"), "{answered:?}");
    }

    #[test]
    fn plain_override_clears_a_standing_assumption_on_the_same_path() {
        let (mut state, _) = start(spec(PcDice::Engine)).unwrap();
        for standing in [true, false] {
            execute(
                &mut state,
                Command::Override {
                    path: "goblin-warrior.hp".into(),
                    value: Some(Value::from(5)),
                    reason: "t".into(),
                    rule: None,
                    standing,
                },
            )
            .unwrap();
        }
        assert!(state.standing_assumptions.is_empty());
        let fought = { let cmd = attack(&state); execute(&mut state, cmd) }.unwrap();
        assert!(!lines(&fought).iter().any(|l| l.contains("standing assumption")), "{:?}", lines(&fought));
    }

    #[test]
    fn ask_mode_prompts_for_an_added_pc_initiative() {
        let striker = block("party/standard/standard-striker-03.json");
        let (mut state, _) = start(spec(PcDice::Ask)).unwrap();
        execute(&mut state, Command::Respond { values: vec![10], decline: false, take: None }).unwrap();
        let add = |block: &StatBlock, side: &str| Command::Add {
            stat_block: Box::new(block.clone()),
            side: side.into(),
            band: None,
            engaged_with: None,
        };
        let goblin = block("fixtures/goblin-warrior.json");
        assert!(execute(&mut state, add(&goblin, "monsters")).unwrap().pending.is_none());

        let asked = execute(&mut state, add(&striker, PARTY_SIDE)).unwrap();
        let prompt = asked.pending.expect("initiative prompt for the added PC");
        assert_eq!((prompt.sides, prompt.remaining), (Some(20), 1));
        assert!(!state.combatants.iter().any(|c| c.id == "standard-striker-03-2"));
        assert!(matches!(execute(&mut state, Command::end_turn()), Err(LiveError::Pending(_))));

        let added = lines(&execute(&mut state, Command::Respond { values: vec![14], decline: false, take: None }).unwrap());
        assert_eq!(state.initiative_rolls["standard-striker-03-2"], 14 + striker.initiative);
        let line = added.iter().find(|l| l.starts_with("ADD standard-striker-03-2 ")).expect("add line");
        assert!(line.ends_with("(player dice)"), "{added:?}");
        assert!(replay(&state).matches);

        execute(&mut state, Command::Undo).unwrap();
        assert!(state.pending.is_some());
        let declined = lines(&execute(&mut state, Command::Respond { values: vec![], decline: true, take: None }).unwrap());
        let line = declined.iter().find(|l| l.starts_with("ADD standard-striker-03-2 ")).expect("add line");
        assert!(!line.contains("(player dice)"), "{declined:?}");
    }

    #[test]
    fn engine_mode_rolls_pc_initiative_without_a_prompt() {
        let (state, started) = start(spec(PcDice::Engine)).unwrap();
        assert!(started.pending.is_none());
        assert_eq!(state.round, 1);
        assert_eq!(state.initiative_order.len(), 2);
    }

    #[test]
    fn standing_override_records_an_assumption_and_tags_affected_lines() {
        let (mut state, _) = start(spec(PcDice::Engine)).unwrap();
        let set = execute(
            &mut state,
            Command::Override {
                path: "goblin-warrior.hp".into(),
                value: Some(Value::from(5)),
                reason: "wounded scout".into(),
                rule: None,
                standing: true,
            },
        )
        .unwrap();
        assert_eq!(state.standing_assumptions.len(), 1);
        assert_eq!(state.standing_assumptions[0].path, "goblin-warrior.hp");
        assert_eq!(state.standing_assumptions[0].value, Value::from(5));
        assert_eq!(state.standing_assumptions[0].reason, "wounded scout");
        assert!(lines(&set)[0].ends_with("(reason: wounded scout) [standing assumption]"), "{:?}", lines(&set));

        let fought = { let cmd = attack(&state); execute(&mut state, cmd) }.unwrap();
        assert!(
            lines(&fought).iter().all(|l| l.ends_with(" [standing assumption: goblin-warrior.hp]")),
            "{:?}",
            lines(&fought)
        );
        let public = crate::log::render_public(&state.log);
        assert!(public.contains("OVERRIDE goblin-warrior.hp (reason: wounded scout) [standing assumption]"), "{public}");
        assert!(replay(&state).matches);

        execute(&mut state, Command::Undo).unwrap();
        execute(&mut state, Command::Undo).unwrap();
        assert!(state.standing_assumptions.is_empty());
    }

    #[test]
    fn public_view_keeps_the_assumption_tag_on_monster_damage() {
        let (mut state, _) = start(spec(PcDice::Engine)).unwrap();
        execute(
            &mut state,
            Command::Override {
                path: "goblin-warrior.max_hp".into(),
                value: Some(Value::from(100)),
                reason: "elite".into(),
                rule: None,
                standing: true,
            },
        )
        .unwrap();
        let damage = |state: &EncounterState| {
            crate::log::render_public(&state.log).lines().find(|l| l.contains("goblin-warrior takes damage (")).map(str::to_string)
        };
        for _ in 0..20 {
            if damage(&state).is_some() {
                break;
            }
            if state.active_turn.as_deref() == Some("standard-striker-03") {
                let cmd = attack(&state);
                execute(&mut state, cmd).unwrap();
            }
            execute(&mut state, Command::end_turn()).unwrap();
        }
        let line = damage(&state).expect("the striker hits the goblin within ten rounds");
        assert!(line.ends_with(" [standing assumption: goblin-warrior.max_hp]"), "{line}");
        let full = crate::log::render(&state.log);
        assert!(full.lines().any(|l| l.contains("goblin-warrior takes ") && l.ends_with(" damage [standing assumption: goblin-warrior.max_hp]")), "{full}");
    }

    #[test]
    fn plain_override_sets_no_assumption() {
        let (mut state, _) = start(spec(PcDice::Engine)).unwrap();
        let set = execute(
            &mut state,
            Command::Override { path: "goblin-warrior.hp".into(), value: Some(Value::from(5)), reason: "t".into(), rule: None, standing: false },
        )
        .unwrap();
        assert!(state.standing_assumptions.is_empty());
        assert!(!lines(&set)[0].contains("standing assumption"));
        let fought = { let cmd = attack(&state); execute(&mut state, cmd) }.unwrap();
        assert!(!lines(&fought).iter().any(|l| l.contains("standing assumption")));
    }

    #[test]
    fn stat_block_assumption_tags_every_combatant_using_the_block() {
        let (mut state, _) = start(spec(PcDice::Engine)).unwrap();
        execute(
            &mut state,
            Command::Override {
                path: "stat_blocks.goblin-warrior.stat_block.armor_class".into(),
                value: Some(serde_json::json!([{ "value": 30 }])),
                reason: "shield wall".into(),
                rule: None,
                standing: true,
            },
        )
        .unwrap();
        let fought = { let cmd = attack(&state); execute(&mut state, cmd) }.unwrap();
        let tag = " [standing assumption: stat_blocks.goblin-warrior.stat_block.armor_class]";
        assert!(lines(&fought).iter().all(|l| l.ends_with(tag)), "{:?}", lines(&fought));
    }

    const STRIKER: &str = "standard-striker-03";
    const GOBLIN: &str = "goblin-warrior";

    /// Engage the two combatants, then end turns until `mover` is active.
    fn engage_then_turn_of(state: &mut EncounterState, mover: &str) {
        let actor = state.active_turn.clone().expect("active turn");
        let other = if actor == STRIKER { GOBLIN } else { STRIKER };
        let engage = Command::Move { actor, target: Some(other.into()), band: RangeBand::Engaged, feet: None, opportunity: vec![] };
        execute(state, engage).unwrap();
        while state.active_turn.as_deref() != Some(mover) || state.combatants.iter().any(|c| c.id == mover && c.action_economy.movement_used_feet > 0) {
            execute(state, Command::end_turn()).unwrap();
        }
    }

    fn move_away(mover: &str, reactor: &str) -> Command {
        Command::Move {
            actor: mover.into(),
            target: Some(reactor.into()),
            band: RangeBand::Near,
            feet: None,
            opportunity: vec![OpportunityAttack { reactor_id: reactor.into(), attack_id: None }],
        }
    }

    #[test]
    fn opportunity_attack_on_a_move_undoes_and_replays() {
        let (mut state, _) = start(spec(PcDice::Engine)).unwrap();
        engage_then_turn_of(&mut state, STRIKER);
        let first = execute(&mut state, move_away(STRIKER, GOBLIN)).unwrap();
        let first_lines = lines(&first);
        assert_eq!(first_lines[0], "OPPORTUNITY ATTACK goblin-warrior on standard-striker-03 (reaction)", "{first_lines:?}");
        assert!(first_lines[1].starts_with("Goblin Warrior attacks"), "{first_lines:?}");
        assert!(state.combatants.iter().find(|c| c.id == GOBLIN).unwrap().action_economy.reaction_used);
        let stored = state.command_history.last().unwrap().args.clone();
        assert_eq!(stored["opportunity"][0]["reactor_id"], GOBLIN);

        execute(&mut state, Command::Undo).unwrap();
        assert!(!state.combatants.iter().find(|c| c.id == GOBLIN).unwrap().action_economy.reaction_used);
        let again = execute(&mut state, move_away(STRIKER, GOBLIN)).unwrap();
        assert_eq!(first_lines, lines(&again));
        assert!(replay(&state).matches);
    }

    #[test]
    fn disengage_stops_provocation_and_undoes_and_replays() {
        let (mut state, _) = start(spec(PcDice::Engine)).unwrap();
        engage_then_turn_of(&mut state, STRIKER);
        let out = execute(&mut state, Command::Disengage { actor: STRIKER.into(), extra: false }).unwrap();
        assert!(lines(&out)[0].contains("Disengage"), "{:?}", lines(&out));
        assert!(state.combatants.iter().find(|c| c.id == STRIKER).unwrap().action_economy.disengaged);
        let refused = execute(&mut state, move_away(STRIKER, GOBLIN));
        assert!(matches!(&refused, Err(e) if format!("{e:?}").contains("has disengaged")), "{refused:?}");
        let moved = execute(&mut state, Command::Move { actor: STRIKER.into(), target: Some(GOBLIN.into()), band: RangeBand::Near, feet: None, opportunity: vec![] }).unwrap();
        assert!(lines(&moved).iter().all(|l| !l.contains("Opportunity Attack")), "{:?}", lines(&moved));
        assert!(replay(&state).matches);
        execute(&mut state, Command::Undo).unwrap();
        execute(&mut state, Command::Undo).unwrap();
        assert!(!state.combatants.iter().find(|c| c.id == STRIKER).unwrap().action_economy.disengaged);
        assert!(replay(&state).matches);
    }

    #[test]
    fn ask_mode_holds_the_move_on_a_pc_reactors_dice() {
        let (mut state, _) = start(spec(PcDice::Ask)).unwrap();
        execute(&mut state, Command::Respond { values: vec![10], decline: false, take: None }).unwrap();
        engage_then_turn_of(&mut state, GOBLIN);
        let before = state.combatants.clone();
        let asked = execute(&mut state, move_away(GOBLIN, STRIKER)).unwrap();
        let prompt = asked.pending.clone().expect("dice prompt");
        assert_eq!((prompt.kind.as_str(), prompt.actor_id.as_str(), prompt.sides), ("dice", STRIKER, Some(20)));
        assert_eq!(serde_json::to_value(&state.combatants).unwrap(), serde_json::to_value(&before).unwrap());

        let mut answered = vec![];
        while let Some(p) = state.pending.clone() {
            let out = execute(&mut state, Command::Respond { values: vec![p.sides.unwrap(); p.remaining as usize], decline: false, take: None }).unwrap();
            answered.extend(lines(&out));
        }
        assert!(answered.contains(&"OPPORTUNITY ATTACK standard-striker-03 on goblin-warrior (reaction)".to_string()), "{answered:?}");
        assert!(answered.iter().any(|l| l.contains("attacks") && l.contains("(player dice)")), "{answered:?}");
        assert!(state.combatants.iter().find(|c| c.id == STRIKER).unwrap().action_economy.reaction_used);
        assert!(replay(&state).matches);
    }

    #[test]
    fn pending_dice_prompt_gates_commands() {
        let (mut state, _) = start(spec(PcDice::Ask)).unwrap();
        execute(&mut state, Command::Respond { values: vec![10], decline: false, take: None }).unwrap();
        if state.active_turn.as_deref() != Some("standard-striker-03") {
            execute(&mut state, Command::end_turn()).unwrap();
        }
        let asked = { let cmd = attack(&state); execute(&mut state, cmd) }.unwrap();
        let prompt = asked.pending.clone().expect("dice prompt");
        assert_eq!(prompt.kind, "dice");
        assert!(asked.events[0].line.starts_with("PROMPT "));
        assert!(matches!(execute(&mut state, Command::end_turn()), Err(LiveError::Pending(_))));
        assert!(matches!(
            execute(&mut state, Command::Respond { values: vec![1; prompt.remaining as usize + 1], decline: false, take: None }),
            Err(LiveError::Invalid(_))
        ));

        let mut answered = vec![];
        while let Some(p) = state.pending.clone() {
            let face = p.sides.unwrap();
            let out = execute(&mut state, Command::Respond { values: vec![face; p.remaining as usize], decline: false, take: None }).unwrap();
            answered.extend(lines(&out));
        }
        assert!(answered.iter().any(|l| l.contains("(player dice)")), "{answered:?}");
        execute(&mut state, Command::end_turn()).unwrap();
        let report = replay(&state);
        let names: Vec<_> = state.command_history.iter().map(|r| (r.name.clone(), r.args.clone())).collect();
        assert!(report.matches, "{report:?} {names:?}");
    }

    #[test]
    fn public_view_hides_monster_numbers() {
        let (mut state, _) = start(spec(PcDice::Engine)).unwrap();
        execute(
            &mut state,
            Command::Override {
                path: "goblin-warrior.hp".into(),
                value: Some(Value::from(5)),
                reason: "earlier fight".into(),
                rule: None,
                standing: false,
            },
        )
        .unwrap();
        let public = digest_of(&state, true);
        let goblin = public["combatants"].as_array().unwrap().iter().find(|c| c["id"] == "goblin-warrior").unwrap();
        assert_eq!(goblin["wounded"], "bloodied");
        assert!(goblin.get("hp").is_none());
        assert!(goblin.get("ac").is_some());
        let pc = public["combatants"].as_array().unwrap().iter().find(|c| c["id"] == "standard-striker-03").unwrap();
        assert_eq!(pc["hp"], 28);

        let text = crate::log::render_public(&state.log);
        assert!(text.contains("OVERRIDE goblin-warrior.hp (reason: earlier fight)"), "{text}");
        assert!(!text.contains("= 5"), "{text}");
        assert!(crate::log::render(&state.log).contains("goblin-warrior.hp = 5"));
    }

    #[test]
    fn override_clears_one_array_element() {
        let (mut state, _) = start(spec(PcDice::Engine)).unwrap();
        let set = |state: &mut EncounterState, value: Option<Value>, path: &str| {
            execute(state, Command::Override { path: path.into(), value, reason: "t".into(), rule: None, standing: false })
        };
        set(&mut state, Some(serde_json::json!(["prone", "poisoned"])), "goblin-warrior.conditions").unwrap();
        set(&mut state, None, "goblin-warrior.conditions.0").unwrap();
        let goblin = state.combatants.iter().find(|c| c.id == "goblin-warrior").unwrap();
        assert_eq!(goblin.conditions.len(), 1);
        assert!(set(&mut state, None, "goblin-warrior.hp").is_err());
        assert!(set(&mut state, Some(Value::from(1)), "goblin-warrior.id").is_err());
        assert!(set(&mut state, Some(Value::from(1)), "engine_version").is_err());
    }

    #[test]
    fn override_ending_concentration_with_an_incapacitating_condition() {
        let set = |state: &mut EncounterState, path: &str, value: Value| {
            execute(state, Command::Override { path: path.into(), value: Some(value), reason: "t".into(), rule: None, standing: false })
        };
        let concentrating = || {
            let (mut state, _) = start(spec(PcDice::Engine)).unwrap();
            state.combatants.iter_mut().find(|c| c.id == "goblin-warrior").unwrap().concentration = Some("bless".into());
            state
        };
        let goblin = |state: &EncounterState| state.combatants.iter().find(|c| c.id == "goblin-warrior").unwrap().concentration.clone();

        let mut stunned = concentrating();
        stunned.combatants.iter_mut().find(|c| c.id == "standard-striker-03").unwrap().conditions.push(crate::encounter::ConditionInstance {
            condition: crate::stat_block::Condition::Blinded,
            duration: crate::effect::Duration::Concentration,
            source: Some("goblin-warrior".into()),
            ends_on_damage: false,
            started_round: 0,
        });
        let outcome = set(&mut stunned, "goblin-warrior.conditions", serde_json::json!(["stunned"])).unwrap();
        assert_eq!(goblin(&stunned), None);
        assert!(stunned.combatants.iter().find(|c| c.id == "standard-striker-03").unwrap().conditions.is_empty(), "what it held ends");
        assert!(lines_of(&outcome).iter().any(|l| l == "goblin-warrior loses concentration"), "{:?}", lines_of(&outcome));

        let mut prone = concentrating();
        set(&mut prone, "goblin-warrior.conditions", serde_json::json!(["prone"])).unwrap();
        assert_eq!(goblin(&prone), Some("bless".into()), "Prone doesn't incapacitate");

        let mut other = concentrating();
        set(&mut other, "goblin-warrior.hp", Value::from(3)).unwrap();
        assert_eq!(goblin(&other), Some("bless".into()), "an unrelated override leaves it");
    }

    #[test]
    fn override_ending_concentration_for_end_state_and_concentration_overrides() {
        let set = |state: &mut EncounterState, path: &str, value: Value| {
            execute(state, Command::Override { path: path.into(), value: Some(value), reason: "t".into(), rule: None, standing: false })
        };
        let fresh = || start(spec(PcDice::Engine)).unwrap().0;
        let concentrating = || {
            let mut state = fresh();
            state.combatants.iter_mut().find(|c| c.id == "goblin-warrior").unwrap().concentration = Some("bless".into());
            state
        };
        let goblin = |state: &EncounterState| state.combatants.iter().find(|c| c.id == "goblin-warrior").unwrap().concentration.clone();

        for end_state in ["unconscious", "dead", "stable"] {
            let mut state = concentrating();
            let outcome = set(&mut state, "goblin-warrior.end_state", Value::from(end_state)).unwrap();
            assert_eq!(goblin(&state), None, "{end_state}");
            assert!(replay(&state).matches, "{end_state}");
            assert!(lines_of(&outcome).iter().any(|l| l == "goblin-warrior loses concentration"), "{end_state}: {:?}", lines_of(&outcome));
        }

        let mut active = concentrating();
        set(&mut active, "goblin-warrior.end_state", Value::from("active")).unwrap();
        assert_eq!(goblin(&active), Some("bless".into()), "Active leaves it");

        let mut stunned = fresh();
        set(&mut stunned, "goblin-warrior.conditions", serde_json::json!(["stunned"])).unwrap();
        let outcome = set(&mut stunned, "goblin-warrior.concentration", Value::from("bless")).unwrap();
        assert_eq!(goblin(&stunned), None, "an incapacitated creature can't hold it");
        assert!(lines_of(&outcome).iter().any(|l| l == "goblin-warrior loses concentration"), "{:?}", lines_of(&outcome));

        assert!(replay(&stunned).matches);

        let mut able = fresh();
        set(&mut able, "goblin-warrior.concentration", Value::from("bless")).unwrap();
        assert_eq!(goblin(&able), Some("bless".into()), "an able creature keeps it");

        let mut holding = concentrating();
        holding.combatants.iter_mut().find(|c| c.id == "standard-striker-03").unwrap().conditions.push(crate::encounter::ConditionInstance {
            condition: crate::stat_block::Condition::Blinded,
            duration: crate::effect::Duration::Concentration,
            source: Some("goblin-warrior".into()),
            ends_on_damage: false,
            started_round: 0,
        });
        set(&mut holding, "goblin-warrior.end_state", Value::from("dead")).unwrap();
        assert!(holding.combatants.iter().find(|c| c.id == "standard-striker-03").unwrap().conditions.is_empty(), "what it held ends");

        let mut stale = concentrating();
        stale.combatants.iter_mut().find(|c| c.id == "goblin-warrior").unwrap().end_state = EndState::Dead;
        set(&mut stale, "settings.lighting", Value::from("dim_light")).unwrap();
        assert_eq!(goblin(&stale), Some("bless".into()), "an override elsewhere leaves a creature it didn't touch");

        let mut by_id = concentrating();
        set(&mut by_id, "combatants.goblin-warrior.end_state", Value::from("dead")).unwrap();
        assert_eq!(goblin(&by_id), None, "the combatants.<id> path");
        assert!(replay(&by_id).matches);

        let mut by_index = concentrating();
        let index = by_index.combatants.iter().position(|c| c.id == "goblin-warrior").unwrap();
        set(&mut by_index, &format!("combatants.{index}.conditions"), serde_json::json!(["stunned"])).unwrap();
        assert_eq!(goblin(&by_index), None, "the combatants.<index> path");
    }

    #[test]
    fn removal_ending_concentration() {
        let override_cmd = |path: &str, value: Option<Value>| Command::Override { path: path.into(), value, reason: "t".into(), rule: None, standing: false };
        let holding = || {
            let mut state = start(spec(PcDice::Engine)).unwrap().0;
            execute(&mut state, override_cmd("goblin-warrior.concentration", Some(Value::from("haste")))).unwrap();
            execute(
                &mut state,
                override_cmd(
                    "standard-striker-03.conditions",
                    Some(serde_json::json!([{"condition": "blinded", "duration": "concentration", "source": "goblin-warrior"}])),
                ),
            )
            .unwrap();
            execute(
                &mut state,
                override_cmd(
                    "active_modifiers",
                    Some(serde_json::json!([{
                        "target": {"extra_action": {"only": ["one_attack"]}},
                        "change": "none",
                        "remaining": "concentration",
                        "source": "goblin-warrior",
                        "bearer": "standard-striker-03",
                        "on_end": [{"condition": {"condition": "incapacitated", "duration": "until_end_of_next_turn", "source_ability": "haste-lethargy"}}]
                    }])),
                ),
            )
            .unwrap();
            state
        };
        let striker = |state: &EncounterState| {
            let c = state.combatants.iter().find(|c| c.id == "standard-striker-03").unwrap();
            c.conditions.iter().map(|x| x.condition).collect::<Vec<_>>()
        };
        let held = |state: &EncounterState| state.active_modifiers.iter().any(|m| m.source.as_deref() == Some("goblin-warrior"));
        let lethargy = vec![crate::stat_block::Condition::Incapacitated];
        let clear = |state: &mut EncounterState, path: &str| execute(state, override_cmd(path, None));

        let before = holding();
        assert_eq!(striker(&before), vec![crate::stat_block::Condition::Blinded]);
        assert!(held(&before));

        let mut removed = holding();
        let outcome = execute(&mut removed, Command::Remove { id: "goblin-warrior".into(), reason: "flees".into() }).unwrap();
        assert_eq!(striker(&removed), lethargy, "bench remove ends the held condition and fires the modifier's on_end");
        assert!(!held(&removed), "bench remove ends the held modifier");
        let lines = lines_of(&outcome);
        assert_eq!(lines.first().map(String::as_str), Some("REMOVE goblin-warrior (reason: flees)"), "{lines:?}");
        assert!(lines.iter().any(|l| l == "goblin-warrior loses concentration"), "{lines:?}");
        assert!(replay(&removed).matches, "bench remove replays");
        execute(&mut removed, Command::Undo).unwrap();
        assert_eq!(striker(&removed), vec![crate::stat_block::Condition::Blinded], "undo restores what it held");
        assert!(held(&removed));
        assert_eq!(removed.combatants.iter().find(|c| c.id == "goblin-warrior").unwrap().concentration, Some("haste".into()));

        let mut by_id = holding();
        let outcome = clear(&mut by_id, "combatants.goblin-warrior").unwrap();
        assert!(!by_id.combatants.iter().any(|c| c.id == "goblin-warrior"));
        assert_eq!(striker(&by_id), lethargy, "clearing combatants.<id> ends what it held");
        assert!(!held(&by_id));
        assert!(lines_of(&outcome).iter().any(|l| l == "goblin-warrior loses concentration"), "{:?}", lines_of(&outcome));
        assert!(replay(&by_id).matches, "clearing combatants.<id> replays");

        let mut by_index = holding();
        let index = by_index.combatants.iter().position(|c| c.id == "goblin-warrior").unwrap();
        clear(&mut by_index, &format!("combatants.{index}")).unwrap();
        assert!(!by_index.combatants.iter().any(|c| c.id == "goblin-warrior"));
        assert_eq!(striker(&by_index), lethargy, "clearing combatants.<index> ends what it held");
        assert!(replay(&by_index).matches);
        execute(&mut by_index, Command::Undo).unwrap();
        assert_eq!(striker(&by_index), vec![crate::stat_block::Condition::Blinded], "undo of an override removal restores what it held");
        assert!(held(&by_index));

        for remove in [true, false] {
            let mut cleared = holding();
            execute(&mut cleared, override_cmd("goblin-warrior.concentration", None)).unwrap();
            assert!(held(&cleared), "clearing the field alone leaves what it held");
            let outcome = if remove {
                execute(&mut cleared, Command::Remove { id: "goblin-warrior".into(), reason: "flees".into() }).unwrap()
            } else {
                clear(&mut cleared, "combatants.goblin-warrior").unwrap()
            };
            assert_eq!(striker(&cleared), lethargy, "remove {remove}: a creature that leaves ends what it held");
            assert!(!held(&cleared), "remove {remove}");
            let lines = lines_of(&outcome);
            assert!(lines.iter().any(|l| l == "goblin-warrior leaves; the effects it held by concentration end"), "{lines:?}");
            assert!(!lines.iter().any(|l| l == "goblin-warrior loses concentration"), "{lines:?}");
            assert!(replay(&cleared).matches, "remove {remove}");
        }

        let mut replaced = holding();
        let others: Vec<Value> = replaced
            .combatants
            .iter()
            .filter(|c| c.id != "goblin-warrior")
            .map(|c| serde_json::to_value(c).unwrap())
            .collect();
        let outcome = execute(&mut replaced, override_cmd("combatants", Some(Value::Array(others)))).unwrap();
        assert!(!replaced.combatants.iter().any(|c| c.id == "goblin-warrior"));
        assert_eq!(striker(&replaced), lethargy, "replacing combatants without it ends the condition it held on the bearer");
        assert!(!held(&replaced), "replacing combatants without it ends what it held");
        assert!(lines_of(&outcome).iter().any(|l| l == "goblin-warrior loses concentration"), "{:?}", lines_of(&outcome));
        assert!(replay(&replaced).matches, "replacing combatants replays");

        let mut other = holding();
        let striker_index = other.combatants.iter().position(|c| c.id == "standard-striker-03").unwrap();
        let outcome = clear(&mut other, &format!("combatants.{striker_index}.conditions")).unwrap();
        assert_eq!(other.combatants.iter().find(|c| c.id == "goblin-warrior").unwrap().concentration, Some("haste".into()), "an override that removes nobody leaves it");
        assert!(held(&other));
        assert!(!lines_of(&outcome).iter().any(|l| l.contains("loses concentration")));

        let mut failed = holding();
        let snapshot = serde_json::to_value(&failed).unwrap();
        assert!(execute(&mut failed, override_cmd("combatants.goblin-warrior.hp", Some(Value::from("many")))).is_err());
        assert_eq!(serde_json::to_value(&failed).unwrap(), snapshot, "a refused override changes nothing");

        let mut plain = start(spec(PcDice::Engine)).unwrap().0;
        execute(&mut plain, Command::Remove { id: "goblin-warrior".into(), reason: "flees".into() }).unwrap();
        assert!(replay(&plain).matches, "bench remove of a creature that isn't concentrating replays");
        let mut plain = start(spec(PcDice::Engine)).unwrap().0;
        clear(&mut plain, "combatants.goblin-warrior").unwrap();
        assert!(replay(&plain).matches, "an override removal of a creature that isn't concentrating replays");
    }

    #[test]
    fn override_removal_cleans_up_the_encounter() {
        let override_cmd = |path: &str, value: Option<Value>| Command::Override { path: path.into(), value, reason: "t".into(), rule: None, standing: false };
        let mentions = |state: &EncounterState, id: &str| {
            state.initiative_order.iter().any(|c| c == id)
                || state.initiative_rolls.contains_key(id)
                || state.active_turn.as_deref() == Some(id)
                || state.combatants.iter().any(|c| c.engagement.iter().any(|e| e == id) || c.ranges.contains_key(id))
        };
        for form in ["id", "index", "list"] {
            let mut state = start(spec(PcDice::Engine)).unwrap().0;
            let victim = state.initiative_order.iter().find(|id| Some(id.as_str()) != state.active_turn.as_deref()).cloned().unwrap();
            let other = state.combatants.iter().find(|c| c.id != victim).unwrap().id.clone();
            state.combatants.iter_mut().find(|c| c.id == other).unwrap().engagement.push(victim.clone());
            state.combatants.iter_mut().find(|c| c.id == other).unwrap().ranges.insert(victim.clone(), RangeBand::Near);
            let before = state.active_turn.clone();
            let remove = match form {
                "id" => override_cmd(&format!("combatants.{victim}"), None),
                "index" => override_cmd(&format!("combatants.{}", state.combatants.iter().position(|c| c.id == victim).unwrap()), None),
                _ => {
                    let rest: Vec<Value> = state.combatants.iter().filter(|c| c.id != victim).map(|c| serde_json::to_value(c).unwrap()).collect();
                    override_cmd("combatants", Some(Value::Array(rest)))
                }
            };
            execute(&mut state, remove).unwrap();
            assert!(!state.combatants.iter().any(|c| c.id == victim));
            assert!(!mentions(&state, &victim), "{form}: nothing keeps `{victim}`");
            assert_eq!(state.active_turn, before, "{form}: the active turn stays");
            assert!(replay(&state).matches, "{form} replays");
            execute(&mut state, Command::Undo).unwrap();
            assert!(state.combatants.iter().any(|c| c.id == victim) && state.initiative_order.contains(&victim), "{form}: undo restores it");
        }

        for form in ["id", "index", "list"] {
            let mut state = start(spec(PcDice::Engine)).unwrap().0;
            let active = state.active_turn.clone().unwrap();
            let remove = match form {
                "id" => override_cmd(&format!("combatants.{active}"), None),
                "index" => override_cmd(&format!("combatants.{}", state.combatants.iter().position(|c| c.id == active).unwrap()), None),
                _ => {
                    let rest: Vec<Value> = state.combatants.iter().filter(|c| c.id != active).map(|c| serde_json::to_value(c).unwrap()).collect();
                    override_cmd("combatants", Some(Value::Array(rest)))
                }
            };
            let snapshot = serde_json::to_value(&state).unwrap();
            let err = execute(&mut state, remove).unwrap_err();
            assert!(matches!(err, LiveError::Invalid(ref m) if m.contains("bench remove")), "{form}: {err:?}");
            assert_eq!(serde_json::to_value(&state).unwrap(), snapshot, "{form}: a refused override changes nothing");
            execute(&mut state, Command::Remove { id: active.clone(), reason: "t".into() }).unwrap();
            assert!(!mentions(&state, &active), "{form}");
        }

        let mut prompted = start(spec(PcDice::Ask)).unwrap().0;
        assert!(prompted.pending.is_some());
        let snapshot = serde_json::to_value(&prompted).unwrap();
        let err = execute(&mut prompted, override_cmd("combatants.goblin-warrior", None)).unwrap_err();
        assert!(matches!(err, LiveError::Pending(ref m) if m.contains("bench respond")), "{err:?}");
        assert_eq!(serde_json::to_value(&prompted).unwrap(), snapshot, "a refused override changes nothing");
        execute(&mut prompted, override_cmd("goblin-warrior.hp", Some(Value::from(3)))).unwrap();
        assert!(prompted.pending.is_some(), "an override that removes nobody still works while a prompt is pending");
    }

    #[test]
    fn override_refuses_added_ids() {
        let override_cmd = |path: &str, value: Option<Value>| Command::Override { path: path.into(), value, reason: "t".into(), rule: None, standing: false };
        for form in ["list", "rename", "index"] {
            let mut state = start(spec(PcDice::Engine)).unwrap().0;
            let add = match form {
                "list" => {
                    let mut all: Vec<Value> = state.combatants.iter().map(|c| serde_json::to_value(c).unwrap()).collect();
                    let mut copy = all[0].clone();
                    copy["id"] = Value::from("goblin-copy");
                    all.push(copy);
                    override_cmd("combatants", Some(Value::Array(all)))
                }
                "index" => {
                    let mut entry = serde_json::to_value(&state.combatants[0]).unwrap();
                    entry["id"] = Value::from("goblin-copy");
                    override_cmd("combatants.0", Some(entry))
                }
                _ => {
                    let mut all: Vec<Value> = state.combatants.iter().map(|c| serde_json::to_value(c).unwrap()).collect();
                    let goblin = all.iter_mut().find(|c| c["id"] == "goblin-warrior").unwrap();
                    goblin["id"] = Value::from("goblin-copy");
                    override_cmd("combatants", Some(Value::Array(all)))
                }
            };
            let snapshot = serde_json::to_value(&state).unwrap();
            let err = execute(&mut state, add).unwrap_err();
            assert!(matches!(err, LiveError::Invalid(ref m) if m.contains("goblin-copy") && m.contains("bench add")), "{form}: {err:?}");
            assert_eq!(serde_json::to_value(&state).unwrap(), snapshot, "{form}: a refused override changes nothing");
        }

        let mut state = start(spec(PcDice::Engine)).unwrap().0;
        let same: Vec<Value> = state.combatants.iter().map(|c| serde_json::to_value(c).unwrap()).collect();
        execute(&mut state, override_cmd("combatants", Some(Value::Array(same)))).unwrap();
        execute(&mut state, override_cmd("combatants.goblin-warrior.hp", Some(Value::from(3)))).unwrap();
        assert_eq!(state.combatants.iter().find(|c| c.id == "goblin-warrior").unwrap().hp, 3);
        assert!(replay(&state).matches);
    }

    #[test]
    fn override_refuses_duplicate_ids() {
        let override_cmd = |path: &str, value: Option<Value>| Command::Override { path: path.into(), value, reason: "t".into(), rule: None, standing: false };
        for form in ["list", "index"] {
            let mut state = start(spec(PcDice::Engine)).unwrap().0;
            let goblin = state.combatants.iter().position(|c| c.id == "goblin-warrior").unwrap();
            let other = (goblin + 1) % state.combatants.len();
            let duplicate = match form {
                "list" => {
                    let mut all: Vec<Value> = state.combatants.iter().map(|c| serde_json::to_value(c).unwrap()).collect();
                    all.push(all[goblin].clone());
                    override_cmd("combatants", Some(Value::Array(all)))
                }
                _ => override_cmd(&format!("combatants.{other}"), Some(serde_json::to_value(&state.combatants[goblin]).unwrap())),
            };
            let snapshot = serde_json::to_value(&state).unwrap();
            let err = execute(&mut state, duplicate).unwrap_err();
            assert!(matches!(err, LiveError::Invalid(ref m) if m.contains("goblin-warrior") && m.contains("twice")), "{form}: {err:?}");
            assert_eq!(serde_json::to_value(&state).unwrap(), snapshot, "{form}: a refused override changes nothing");
        }

        let mut state = start(spec(PcDice::Engine)).unwrap().0;
        let same: Vec<Value> = state.combatants.iter().map(|c| serde_json::to_value(c).unwrap()).collect();
        execute(&mut state, override_cmd("combatants", Some(Value::Array(same)))).unwrap();
        execute(&mut state, override_cmd("combatants.goblin-warrior.hp", Some(Value::from(3)))).unwrap();
        assert_eq!(state.combatants.iter().find(|c| c.id == "goblin-warrior").unwrap().hp, 3);

        let copy = state.combatants.iter().find(|c| c.id == "goblin-warrior").unwrap().clone();
        state.combatants.push(copy);
        execute(&mut state, override_cmd("combatants.goblin-warrior.hp", Some(Value::from(5)))).unwrap();
        assert_eq!(state.combatants.iter().find(|c| c.id == "goblin-warrior").unwrap().hp, 5, "a duplicate the override didn't make doesn't block it");
    }

    #[test]
    fn start_suffixes_past_ids_already_taken() {
        let goblin = block("fixtures/goblin-warrior.json");
        let mut second = goblin.clone();
        second.id = "goblin-warrior-2".into();
        second.name = "Goblin Boss".into();
        let monster = |stat_block: StatBlock| Entrant { stat_block, side: "monsters".into(), band: RangeBand::Near };
        let mut spec = spec(PcDice::Engine);
        spec.entrants.truncate(1);
        spec.entrants.extend([monster(goblin.clone()), monster(second), monster(goblin)]);
        let (state, _) = start(spec).unwrap();
        let ids: Vec<(&str, &str)> = state.combatants.iter().map(|c| (c.id.as_str(), c.name.as_str())).collect();
        assert_eq!(
            ids,
            [
                ("standard-striker-03", "Standard Striker (Level 3)"),
                ("goblin-warrior-1", "Goblin Warrior 1"),
                ("goblin-warrior-2", "Goblin Boss"),
                ("goblin-warrior-3", "Goblin Warrior 3"),
            ]
        );
        assert_eq!(state.initiative_order.len(), 4);
        assert_eq!(state.combatants[0].ranges.len(), 3);
    }

    #[test]
    fn roll_expression_parses_terms() {
        let mut stream = Stream::from_seed(5);
        let (total, faces) = roll_expression(&mut stream, "2d6+3").unwrap();
        assert_eq!(faces.len(), 2);
        assert_eq!(total, faces.iter().sum::<u32>() as i32 + 3);
        let (total, faces) = roll_expression(&mut stream, "d20 - 1").unwrap();
        assert_eq!(faces.len(), 1);
        assert_eq!(total, faces[0] as i32 - 1);
        for bad in ["", "2x6", "d", "2d", "3+", "+-2", "0d6"] {
            assert!(roll_expression(&mut stream, bad).is_err(), "{bad}");
        }
    }

    /// The goblin gets a legendary Kick and the striker a legendary Volley, one use each.
    fn legendary_spec(pc_dice: PcDice) -> StartSpec {
        let mut spec = spec(pc_dice);
        for (entrant, from, id) in [(0, "shortsword", "volley"), (1, "scimitar", "kick")] {
            let block = &mut spec.entrants[entrant].stat_block;
            let mut action = block.actions.iter().find(|a| a.id == from).unwrap().clone();
            action.id = id.into();
            action.name = id.into();
            block.legendary_actions.push(action);
            block.legendary_uses_per_round = 1;
        }
        spec
    }

    fn until_turn(state: &mut EncounterState, id: &str) {
        while state.active_turn.as_deref() != Some(id) {
            execute(state, Command::end_turn()).unwrap();
        }
    }

    fn legendary(holder: &str, action: &str, target: &str) -> Command {
        Command::EndTurn {
            legendary: vec![LegendaryUse { holder_id: holder.into(), action_id: action.into(), target_ids: vec![target.into()] }],
        }
    }

    #[test]
    fn end_turn_takes_legendary_actions_between_the_turns_and_undo_reverts_it_all() {
        let (mut state, _) = start(legendary_spec(PcDice::Engine)).unwrap();
        until_turn(&mut state, STRIKER);
        let before = (serde_json::to_value(&state.combatants).unwrap(), state.round, state.active_turn.clone());
        let out = execute(&mut state, legendary(GOBLIN, "kick", STRIKER)).unwrap();
        let lines = lines(&out);
        let at = |needle: &str| lines.iter().position(|l| l.contains(needle)).unwrap_or_else(|| panic!("{needle}: {lines:?}"));
        assert!(at("ends its turn") < at("LEGENDARY ACTION goblin-warrior: kick (1 of 1)"), "{lines:?}");
        assert!(at("LEGENDARY ACTION") < at("Goblin Warrior attacks"), "{lines:?}");
        assert!(at("Goblin Warrior attacks") < at("starts its turn"), "{lines:?}");
        assert_eq!(state.command_history.last().unwrap().args["legendary"][0]["holder_id"], GOBLIN);

        execute(&mut state, Command::Undo).unwrap();
        assert_eq!((serde_json::to_value(&state.combatants).unwrap(), state.round, state.active_turn.clone()), before);
        execute(&mut state, legendary(GOBLIN, "kick", STRIKER)).unwrap();
        assert!(replay(&state).matches);
    }

    #[test]
    fn an_ineligible_legendary_declaration_fails_the_end_turn() {
        let (mut state, _) = start(legendary_spec(PcDice::Engine)).unwrap();
        until_turn(&mut state, GOBLIN);
        let history = state.command_history.len();
        let err = execute(&mut state, legendary(GOBLIN, "kick", STRIKER)).unwrap_err();
        assert!(matches!(&err, LiveError::Illegal(why) if why.contains("turn is ending")), "{err:?}");
        assert_eq!(state.command_history.len(), history);
        assert_eq!(state.active_turn.as_deref(), Some(GOBLIN));
    }

    #[test]
    fn digest_lists_who_may_take_a_legendary_action_next() {
        let (mut state, _) = start(legendary_spec(PcDice::Engine)).unwrap();
        until_turn(&mut state, GOBLIN);
        let digest = digest::digest(&state);
        assert_eq!(digest["legendary"], serde_json::json!([{"id": STRIKER, "uses_left": 1, "ready_actions": ["volley"]}]));
        assert!(digest::public_digest(&state).get("legendary").is_none());
    }

    #[test]
    fn ask_mode_holds_the_whole_end_turn_on_a_pc_legendary_attack() {
        let (mut state, _) = start(legendary_spec(PcDice::Ask)).unwrap();
        execute(&mut state, Command::Respond { values: vec![10], decline: false, take: None }).unwrap();
        until_turn(&mut state, GOBLIN);
        let before = (serde_json::to_value(&state.combatants).unwrap(), state.round);
        let asked = execute(&mut state, legendary(STRIKER, "volley", GOBLIN)).unwrap();
        assert_eq!(asked.pending.as_ref().map(|p| p.actor_id.as_str()), Some(STRIKER));
        assert_eq!((serde_json::to_value(&state.combatants).unwrap(), state.round), before);
        assert_eq!(state.active_turn.as_deref(), Some(GOBLIN));

        let mut answered = vec![];
        while let Some(p) = state.pending.clone() {
            let out = execute(&mut state, Command::Respond { values: vec![p.sides.unwrap(); p.remaining as usize], decline: false, take: None }).unwrap();
            answered.extend(lines(&out));
        }
        assert_eq!(answered.iter().filter(|l| l.contains("ends its turn")).count(), 1, "{answered:?}");
        assert!(answered.iter().any(|l| l == "LEGENDARY ACTION standard-striker-03: volley (1 of 1)"), "{answered:?}");
        assert_eq!(state.active_turn.as_deref(), Some(STRIKER));
        assert!(replay(&state).matches);
    }

    #[test]
    fn a_bare_end_turn_still_reads_and_writes_the_same() {
        let bare = serde_json::json!({"command": "end_turn"});
        let cmd: Command = serde_json::from_value(bare.clone()).unwrap();
        assert!(matches!(&cmd, Command::EndTurn { legendary } if legendary.is_empty()));
        assert_eq!(serde_json::to_value(&cmd).unwrap(), bare);
        let held: StepRequest = stored_request(&serde_json::json!("end_turn")).unwrap();
        assert!(matches!(held, StepRequest::EndTurn { legendary } if legendary.is_empty()));
    }

    #[test]
    fn lighting_is_set_at_start_overridden_undone_and_replayed() {
        let mut lit = spec(PcDice::Engine);
        lit.lighting = Lighting::Sunlight;
        let (mut state, started) = start(lit).unwrap();
        assert_eq!(state.settings.lighting, Lighting::Sunlight);
        assert_eq!(started.digest["lighting"], "sunlight");
        assert_eq!(digest::public_digest(&state)["lighting"], "sunlight");
        assert!(lines(&started)[0].contains("lighting sunlight"), "{:?}", lines(&started));
        let overridden = execute(
            &mut state,
            Command::Override { path: "settings.lighting".into(), value: Some(Value::from("dim_light")), reason: "dusk".into(), rule: None, standing: false },
        )
        .unwrap();
        assert_eq!(state.settings.lighting, Lighting::DimLight);
        assert_eq!(overridden.digest["lighting"], "dim_light");
        assert!(replay(&state).matches);
        execute(&mut state, Command::Undo).unwrap();
        assert_eq!(state.settings.lighting, Lighting::Sunlight);
        assert!(replay(&state).matches);
    }

    #[test]
    fn a_pre_lighting_state_file_passes_the_tamper_check() {
        let (state, _) = start(spec(PcDice::Engine)).unwrap();
        let value = serde_json::to_value(&state).unwrap();
        assert!(value["settings"].get("lighting").is_none());
        assert!(value["initial_snapshot"]["settings"].get("lighting").is_none());
        let path = std::env::temp_dir().join(format!("bench-pre-lighting-{}.json", std::process::id()));
        for lighting in [Lighting::Unspecified, Lighting::Sunlight] {
            let mut lit = spec(PcDice::Engine);
            lit.lighting = lighting;
            let (state, _) = start(lit).unwrap();
            std::fs::write(&path, serde_json::to_string(&state).unwrap()).unwrap();
            let loaded = crate::store::load(&path, crate::store::VersionCheck::Require);
            assert_eq!(loaded.map(|s| s.settings.lighting).ok(), Some(lighting));
        }
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn old_state_files_load_with_their_lighting() {
        let (state, _) = start(spec(PcDice::Engine)).unwrap();
        let mut value = serde_json::to_value(&state).unwrap();
        value["settings"].as_object_mut().unwrap().remove("lighting");
        let old: EncounterState = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(old.settings.lighting, Lighting::Unspecified);
        value["settings"]["lighting"] = "normal".into();
        let renamed: EncounterState = serde_json::from_value(value).unwrap();
        assert_eq!(renamed.settings.lighting, Lighting::Unspecified);
    }

    fn reaction_spec(entrants: &[(&str, &str)], seed: u64, pc_dice: PcDice) -> StartSpec {
        StartSpec {
            entrants: entrants
                .iter()
                .map(|(rel, side)| Entrant { stat_block: block(rel), side: side.to_string(), band: RangeBand::Near })
                .collect(),
            seed,
            ..spec(pc_dice)
        }
    }

    fn take(id: &str) -> Command {
        Command::Respond { values: vec![], decline: false, take: Some(id.into()) }
    }

    fn decline() -> Command {
        Command::Respond { values: vec![], decline: true, take: None }
    }

    fn step_to(actor: &str, band: RangeBand, target: Option<&str>) -> Command {
        Command::Move { actor: actor.into(), target: target.map(Into::into), band, feet: None, opportunity: vec![] }
    }

    fn swing(actor: &str, action: &str, target: &str) -> Command {
        Command::Action { actor: actor.into(), action: action.into(), targets: vec![target.into()], slot: None, extra: false }
    }

    fn window(state: &EncounterState) -> ReactionWindow {
        let held: Held = serde_json::from_value(state.pending.as_ref().expect("prompt").metadata.clone()).unwrap();
        held.window.expect("reaction window")
    }

    const CASTER_FILE: &str = "party/standard/standard-caster-05.json";

    /// An ogre's greatclub hit that Shield would turn into a miss, held on its prompt.
    fn ogre_hits_the_caster() -> (EncounterState, i32) {
        for seed in 1..300 {
            let (mut state, _) = start(reaction_spec(&[(CASTER_FILE, PARTY_SIDE), ("fixtures/ogre.json", "monsters")], seed, PcDice::Engine)).unwrap();
            turn_of(&mut state, "ogre");
            execute(&mut state, step_to("ogre", RangeBand::Engaged, Some(CASTER))).unwrap();
            execute(&mut state, swing("ogre", "greatclub", CASTER)).unwrap();
            if state.pending.is_none() {
                continue;
            }
            let WindowTrigger::Hit { total, critical, .. } = window(&state).trigger else { panic!("hit window") };
            if !critical && total < 17 {
                return (state, total);
            }
        }
        panic!("no seed");
    }

    #[test]
    fn a_hit_holds_the_action_on_a_reaction_prompt_and_take_answers_it() {
        let (mut state, total) = ogre_hits_the_caster();
        let prompt = state.pending.clone().unwrap();
        assert_eq!(prompt.kind, "reaction");
        assert_eq!(prompt.actor_id, CASTER);
        assert!(prompt.id.starts_with("reaction-"), "{}", prompt.id);
        assert_eq!(prompt.message, format!("{CASTER}: hit by ogre (attack {total} vs AC 12), take shield?"));
        assert_eq!(state.log.last().unwrap().lines[0].text, format!("PROMPT {}", prompt.message));
        let hp = state.combatants.iter().find(|c| c.id == CASTER).unwrap().max_hp;

        let taken = execute(&mut state, take("")).unwrap();
        assert_eq!(state.command_history.last().unwrap().args["take"], "shield");
        let lines = lines(&taken);
        assert_eq!(lines[0], format!("PROMPT {} answered: takes Shield", prompt.message));
        assert!(lines.contains(&format!("REACTION {CASTER}: Shield (hit by ogre)")), "{lines:?}");
        assert!(lines.iter().any(|l| l.ends_with("vs AC 17 (miss) after Shield")), "{lines:?}");
        let caster = state.combatants.iter().find(|c| c.id == CASTER).unwrap();
        assert_eq!(caster.hp, hp);
        assert!(caster.action_economy.reaction_used);
        assert!(state.pending.is_none());
        assert!(replay(&state).matches);

        execute(&mut state, Command::Undo).unwrap();
        assert_eq!(state.pending.as_ref().map(|p| p.id.clone()), Some(prompt.id.clone()));
        let declined = execute(&mut state, decline()).unwrap();
        assert_eq!(lines_of(&declined)[0], format!("PROMPT {} declined", prompt.message));
        assert!(state.combatants.iter().find(|c| c.id == CASTER).unwrap().hp < hp);
        assert!(replay(&state).matches);
        let public = crate::log::render_public(&state.log);
        assert!(public.contains(&format!("PROMPT {} declined", prompt.message)), "a PC's window stays public: {public}");
    }

    fn lines_of(outcome: &Outcome) -> Vec<String> {
        lines(outcome)
    }

    #[test]
    fn a_reaction_prompt_refuses_dice_bad_ids_and_mixed_answers() {
        let (mut state, _) = ogre_hits_the_caster();
        let refused = |state: &mut EncounterState, cmd: Command| match execute(state, cmd) {
            Err(LiveError::Invalid(why)) => why,
            other => panic!("{other:?}"),
        };
        assert!(refused(&mut state, Command::Respond { values: vec![3], decline: false, take: None }).contains("not dice faces"));
        assert!(refused(&mut state, take("parry")).contains("parry is not on offer; the options are shield"));
        assert!(refused(&mut state, Command::Respond { values: vec![], decline: true, take: Some("shield".into()) }).contains("not both"));
        assert!(refused(&mut state, Command::Respond { values: vec![], decline: false, take: None }).contains("--take"));
        assert_eq!(state.pending.as_ref().unwrap().kind, "reaction");
    }

    #[test]
    fn take_on_a_dice_prompt_is_refused() {
        let (mut state, _) = start(spec(PcDice::Ask)).unwrap();
        assert_eq!(state.pending.as_ref().unwrap().kind, "dice");
        assert!(matches!(execute(&mut state, take("")), Err(LiveError::Invalid(why)) if why.contains("asks for dice")));
    }

    #[test]
    fn opportunity_windows_ask_each_reactor_in_initiative_order_and_nest_shield() {
        let goblin = "fixtures/goblin-warrior.json";
        for seed in 1..300 {
            let (mut state, _) = start(reaction_spec(&[(CASTER_FILE, PARTY_SIDE), (goblin, "monsters"), (goblin, "monsters")], seed, PcDice::Engine)).unwrap();
            turn_of(&mut state, CASTER);
            for g in ["goblin-warrior-1", "goblin-warrior-2"] {
                execute(&mut state, step_to(CASTER, RangeBand::Engaged, Some(g))).unwrap();
            }
            let order: Vec<String> = state.initiative_order.iter().filter(|id| id.starts_with("goblin")).cloned().collect();
            let asked = execute(&mut state, step_to(CASTER, RangeBand::Near, None)).unwrap();
            let first = state.pending.clone().expect("first window");
            assert_eq!(first.actor_id, order[0]);
            assert_eq!(first.message, format!("{}: Opportunity Attack on {CASTER} (scimitar)?", order[0]));
            assert_eq!(lines(&asked), [format!("PROMPT {}", first.message)]);
            let second = execute(&mut state, take("")).unwrap();
            let next = state.pending.clone().expect("second window");
            assert_eq!(next.actor_id, order[1]);
            assert_ne!(next.id, first.id);
            assert!(lines(&second).iter().all(|l| !l.starts_with("OPPORTUNITY ATTACK")), "asked before any attack: {:?}", lines(&second));
            assert!(digest_of(&state, true)["pending"].is_null(), "a monster's open window stays private");
            assert_eq!(digest_of(&state, false)["pending"]["kind"], "reaction");
            let declined_from = state.log.len();
            let resolved = execute(&mut state, decline()).unwrap();
            let Some(shield) = state.pending.clone() else { continue };
            assert_eq!((shield.kind.as_str(), shield.actor_id.as_str()), ("reaction", CASTER), "{:?}", lines(&resolved));
            let nested = execute(&mut state, take("shield")).unwrap();
            let nested = lines(&nested);
            assert_eq!(nested.iter().filter(|l| l.starts_with("OPPORTUNITY ATTACK")).count(), 1, "{nested:?}");
            assert!(nested.contains(&format!("REACTION {CASTER}: Shield (hit by {})", order[0])), "{nested:?}");
            assert!(nested.iter().any(|l| l == &format!("{CASTER} moves to near")), "{nested:?}");
            let declined = state.combatants.iter().find(|c| c.id == order[1]).unwrap();
            assert!(!declined.action_economy.reaction_used);
            assert!(replay(&state).matches, "{:?}", replay(&state));

            let public = crate::log::render_public(&state.log);
            assert!(public.contains(&format!("PROMPT {} answered: takes Scimitar", first.message)), "{public}");
            assert!(!public.contains(&next.message), "a declined monster window stays private: {public}");
            let after_decline = crate::log::render_public(&state.log[declined_from..]);
            assert!(!after_decline.contains(&order[1]), "nothing about the declined reactor: {after_decline}");
            assert!(crate::log::render(&state.log).contains(&format!("PROMPT {} declined", next.message)));

            execute(&mut state, Command::Undo).unwrap();
            assert_eq!(state.pending.as_ref().map(|p| p.id.clone()), Some(shield.id));
            assert!(replay(&state).matches);
            return;
        }
        panic!("no seed where the first goblin hits");
    }

    #[test]
    fn a_legendary_hit_holds_the_end_turn_on_shield_and_ticks_durations_once() {
        let mut ogre = block("fixtures/ogre.json");
        let mut club = ogre.actions[0].clone();
        club.id = "club".into();
        ogre.legendary_actions.push(club);
        ogre.legendary_uses_per_round = 1;
        for seed in 1..300 {
            let mut spec = reaction_spec(&[(CASTER_FILE, PARTY_SIDE)], seed, PcDice::Engine);
            spec.entrants.push(Entrant { stat_block: ogre.clone(), side: "monsters".into(), band: RangeBand::Near });
            let (mut state, _) = start(spec).unwrap();
            turn_of(&mut state, CASTER);
            let round = state.round;
            let ending = Command::EndTurn { legendary: vec![LegendaryUse { holder_id: "ogre".into(), action_id: "club".into(), target_ids: vec![CASTER.into()] }] };
            execute(&mut state, ending).unwrap();
            let Some(prompt) = state.pending.clone() else { continue };
            assert_eq!(prompt.kind, "reaction");
            assert_eq!(state.active_turn.as_deref(), Some(CASTER), "the end-turn is held");
            let answered = lines(&execute(&mut state, take("shield")).unwrap());
            assert_eq!(answered.iter().filter(|l| l.ends_with("ends its turn")).count(), 1, "{answered:?}");
            assert_eq!(state.active_turn.as_deref(), Some("ogre"));
            assert!(state.round == round || state.round == round + 1);
            let shield = state.active_modifiers.iter().find(|m| m.source.as_deref() == Some(CASTER)).expect("shield");
            assert!(matches!(shield.remaining, crate::effect::Duration::Rounds(1)));
            assert!(replay(&state).matches);
            return;
        }
        panic!("no seed hit");
    }

    fn striker_vs_knight(seed: u64) -> EncounterState {
        let (mut state, _) = start(reaction_spec(&[("party/standard/standard-striker-03.json", PARTY_SIDE), ("fixtures/knight.json", "monsters")], seed, PcDice::Ask)).unwrap();
        execute(&mut state, Command::Respond { values: vec![10], decline: false, take: None }).unwrap();
        while state.active_turn.as_deref() != Some("standard-striker-03") {
            execute(&mut state, Command::end_turn()).unwrap();
            while state.pending.is_some() {
                execute(&mut state, decline()).unwrap();
            }
        }
        state
    }

    #[test]
    fn ask_mode_interleaves_dice_and_a_monster_parry_window_keeping_the_faces() {
        let mut state = striker_vs_knight(1);
        let attack = execute(&mut state, swing("standard-striker-03", "shortsword", "knight")).unwrap();
        assert_eq!(state.pending.as_ref().unwrap().kind, "dice", "{:?}", lines(&attack));
        execute(&mut state, Command::Respond { values: vec![19], decline: false, take: None }).unwrap();
        let parry = state.pending.clone().expect("parry window");
        assert_eq!((parry.kind.as_str(), parry.actor_id.as_str()), ("reaction", "knight"));
        assert!(parry.message.starts_with("knight: hit by standard-striker-03"), "{}", parry.message);
        let taken = execute(&mut state, take("parry")).unwrap();
        let mut all = lines(&taken);
        while let Some(p) = state.pending.clone() {
            assert_eq!(p.kind, "dice");
            all.extend(lines(&execute(&mut state, Command::Respond { values: vec![1; p.remaining as usize], decline: false, take: None }).unwrap()));
        }
        let attack = all.iter().find(|l| l.contains("attacks Knight")).expect("attack line");
        assert!(attack.contains("[d20 19] (player dice)"), "{all:?}");
        assert!(all.iter().any(|l| l.contains("after Parry")), "{all:?}");
        assert!(replay(&state).matches);
    }

    #[test]
    fn a_declined_dice_prompt_stays_engine_filled_through_a_reaction_window() {
        for seed in 1..300 {
            let mut state = striker_vs_knight(seed);
            execute(&mut state, swing("standard-striker-03", "shortsword", "knight")).unwrap();
            execute(&mut state, decline()).unwrap();
            let Some(parry) = state.pending.clone() else { continue };
            assert_eq!(parry.kind, "reaction");
            assert_eq!(parry.metadata["engine_fills_rest"], true);
            execute(&mut state, decline()).unwrap();
            assert!(state.pending.is_none(), "the declined dice are not asked again: {:?}", state.pending);
            assert!(replay(&state).matches);
            return;
        }
        panic!("no seed hit");
    }

    #[test]
    fn a_dice_prompt_holds_only_the_request_and_faces_and_old_ones_still_load() {
        let (state, _) = start(spec(PcDice::Ask)).unwrap();
        let metadata = state.pending.unwrap().metadata;
        let keys: Vec<&String> = metadata.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["faces", "request"]);
        let old: Held = serde_json::from_value(serde_json::json!({"request": "end_turn", "faces": [3]})).unwrap();
        assert!(matches!(old.request, StepRequest::EndTurn { .. }) && old.faces == [3] && !old.engine_fills_rest);
        let bare = serde_json::json!({"command": "respond", "values": [4]});
        let cmd: Command = serde_json::from_value(bare.clone()).unwrap();
        assert!(matches!(&cmd, Command::Respond { take: None, .. }));
        assert_eq!(serde_json::to_value(&cmd).unwrap(), serde_json::json!({"command": "respond", "values": [4], "decline": false}));
    }

    fn old_state() -> (std::path::PathBuf, EncounterState) {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/engine-0.1.4-state.json");
        let state = crate::store::load(&path, crate::store::VersionCheck::Require).expect("an engine 0.1.4 state loads");
        (path, state)
    }

    #[test]
    fn an_engine_0_1_4_state_passes_the_tamper_check_and_refolds_without_windows() {
        let (path, state) = old_state();
        crate::store::check(&state, &path).unwrap();
        assert_eq!(state.engine_version, "0.1.4");
        assert!(!state.settings.reaction_windows);
        let on_disk: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(serde_json::to_value(&state).unwrap(), on_disk, "re-serializes unchanged");
        let names: Vec<&str> = state.command_history.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["move", "action", "end-turn", "move"]);
        assert!(crate::log::render(&state.log).contains("ogre may use an Opportunity Attack on standard-caster-05"));
        let report = replay(&state);
        assert!(report.matches, "{report:?}");
        let mut state = state;
        for _ in 0..2 {
            execute(&mut state, Command::Undo).unwrap();
            assert!(state.pending.is_none());
            assert!(replay(&state).matches);
        }
        let caster = state.combatants.iter().find(|c| c.id == CASTER).unwrap();
        assert!(caster.hp < caster.max_hp, "the ogre's hit stands without a Shield window");
    }

    #[test]
    fn an_engine_0_1_5_state_reads_each_modifier_bearer_as_its_source() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/engine-0.1.5-state.json");
        let state = crate::store::load(&path, crate::store::VersionCheck::Require).expect("an engine 0.1.5 state loads");
        crate::store::check(&state, &path).unwrap();
        assert_eq!(state.engine_version, "0.1.5");
        let on_disk: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(serde_json::to_value(&state).unwrap(), on_disk, "re-serializes unchanged");
        assert_eq!(state.active_modifiers.len(), 3);
        assert!(state.active_modifiers.iter().all(|m| m.is_legacy() && m.bearer() == Some(CASTER)));
        assert_eq!(remaining_speed(&state, CASTER).unwrap(), 30, "a legacy Speed modifier stays inert, as on 0.1.5");
        assert_eq!(remaining_speed(&state, "standard-striker-03").unwrap(), 30);
    }

    #[test]
    fn an_engine_0_1_6_state_loads_and_replays_until_its_haste_cast() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/engine-0.1.6-state.json");
        let state = crate::store::load(&path, crate::store::VersionCheck::Require).expect("an engine 0.1.6 state loads");
        crate::store::check(&state, &path).unwrap();
        assert_eq!(state.engine_version, "0.1.6");
        let on_disk: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(serde_json::to_value(&state).unwrap(), on_disk, "re-serializes unchanged");
        let names: Vec<&str> = state.command_history.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["end-turn", "disengage", "end-turn", "end-turn", "action"]);
        assert!(state.command_history.iter().all(|r| r.args.get("extra").is_none()));
        assert_eq!(state.active_modifiers.len(), 3, "0.1.6 Haste has no extra action modifier");
        assert_eq!(crate::engine::extra_action_uses(&state, "standard-striker-03"), None);
        let report = replay(&state);
        assert_eq!(report.first_divergence, Some(4), "the history replays until Haste, which now grants an extra action: {report:?}");
    }

    #[test]
    fn an_engine_0_1_24_goblin_refolds_with_its_unmodeled_nimble_escape() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/engine-0.1.24-state.json");
        let state = crate::store::load(&path, crate::store::VersionCheck::Require).expect("an engine 0.1.24 state loads");
        crate::store::check(&state, &path).unwrap();
        assert_eq!(state.engine_version, "0.1.24");
        let names: Vec<&str> = state.command_history.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["action", "move", "action", "end-turn"]);
        let goblin = state.stat_blocks.iter().find(|s| s.id == "goblin-warrior").unwrap();
        assert!(goblin.stat_block.bonus_actions[0].effects.is_empty(), "the save holds the old unmodeled Nimble Escape");
        assert!(!crate::log::render(&state.log).contains("Disengage"));
        let report = replay(&state);
        assert!(report.matches, "{report:?}");
    }

    #[test]
    fn a_hasted_striker_spends_its_extra_action_with_extra() {
        const STRIKER: &str = "standard-striker-03";
        let mut spec = reaction_spec(&[(CASTER_FILE, PARTY_SIDE), ("fixtures/ogre.json", "monsters")], 1, PcDice::Engine);
        spec.entrants.push(Entrant { stat_block: block("party/standard/standard-striker-03.json"), side: PARTY_SIDE.into(), band: RangeBand::Near });
        let (mut state, _) = start(spec).unwrap();
        turn_of(&mut state, CASTER);
        execute(&mut state, Command::Action { actor: CASTER.into(), action: "haste".into(), targets: vec![STRIKER.into()], slot: None, extra: false }).unwrap();
        turn_of(&mut state, STRIKER);
        execute(&mut state, Command::Move { actor: STRIKER.into(), target: Some("ogre".into()), band: RangeBand::Engaged, feet: None, opportunity: vec![] }).unwrap();
        let out = execute(&mut state, swing(STRIKER, "shortsword", "ogre")).unwrap();
        assert_eq!(out.digest["extra_action"], serde_json::json!(["one_attack", "disengage"]));
        let extra = Command::Action { actor: STRIKER.into(), action: "shortsword".into(), targets: vec!["ogre".into()], slot: None, extra: true };
        let out = execute(&mut state, extra.clone()).unwrap();
        assert!(lines(&out).contains(&format!("{STRIKER} uses its extra action")), "{:?}", lines(&out));
        assert!(out.digest.get("extra_action").is_none());
        assert!(matches!(execute(&mut state, extra), Err(LiveError::Illegal(m)) if m == "extra action already used"));
        assert_eq!(state.command_history.last().unwrap().args["extra"], true);
        assert!(replay(&state).matches);
    }

    #[test]
    fn haste_cast_on_an_ally_lets_it_close_from_far() {
        const STRIKER: &str = "standard-striker-03";
        let mut spec = reaction_spec(&[(CASTER_FILE, PARTY_SIDE), ("fixtures/ogre.json", "monsters")], 1, PcDice::Engine);
        spec.entrants.push(Entrant { stat_block: block("party/standard/standard-striker-03.json"), side: PARTY_SIDE.into(), band: RangeBand::Far });
        let (mut state, _) = start(spec).unwrap();
        let close = Command::Move { actor: STRIKER.into(), target: Some("ogre".into()), band: RangeBand::Engaged, feet: None, opportunity: vec![] };
        let mut slow = state.clone();
        turn_of(&mut slow, STRIKER);
        assert!(matches!(execute(&mut slow, close.clone()), Err(LiveError::Illegal(m)) if m.contains("requires 35 feet")), "35 feet is beyond a 30-foot Speed");
        turn_of(&mut state, CASTER);
        let haste = Command::Action { actor: CASTER.into(), action: "haste".into(), targets: vec![STRIKER.into()], slot: None, extra: false };
        assert!(lines(&execute(&mut state, haste).unwrap()).contains(&format!("{STRIKER} gains haste from {CASTER}")));
        assert_eq!((remaining_speed(&state, CASTER).unwrap(), remaining_speed(&state, STRIKER).unwrap()), (30, 60));
        turn_of(&mut state, STRIKER);
        execute(&mut state, close).unwrap();
        let striker = state.combatants.iter().find(|c| c.id == STRIKER).unwrap();
        assert!(striker.engagement.contains(&"ogre".to_string()));
        assert_eq!(remaining_speed(&state, STRIKER).unwrap(), 25);
    }

    #[test]
    fn a_spell_cast_window_holds_the_cast_and_undo_and_replay_cover_it() {
        let mut spec = reaction_spec(&[(CASTER_FILE, PARTY_SIDE), ("fixtures/knight.json", "monsters")], 1, PcDice::Engine);
        let mut knight = spec.entrants[1].stat_block.clone();
        knight.spell_slots = Some(vec![0, 0, 3, 0, 0, 0, 0, 0, 0]);
        knight.spellcasting = Some(crate::stat_block::Spellcasting { ability: crate::stat_block::Ability::Int, save_dc: 40, attack_bonus: 0 });
        knight.reactions.push(
            serde_json::from_value(serde_json::json!({
                "id": "counterspell", "name": "Counterspell", "kind": "modeled",
                "effects": [{"cast": {"spell_id": "counterspell", "slot_level": 3, "upcast": false}}],
                "trigger": {"spell_cast": {"caster": "enemy"}}
            }))
            .unwrap(),
        );
        spec.entrants[1].stat_block = knight;
        let (mut state, _) = start(spec).unwrap();
        while state.active_turn.as_deref() != Some(CASTER) {
            execute(&mut state, Command::end_turn()).unwrap();
        }
        let before = state.clone();
        execute(&mut state, swing(CASTER, "magic-missile", "knight")).unwrap();
        let prompt = state.pending.clone().expect("spell-cast window");
        assert_eq!(prompt.kind, "reaction");
        assert_eq!(prompt.actor_id, "knight");
        assert!(prompt.message.contains("standard-caster-05 is casting magic-missile (level 1)"), "{}", prompt.message);
        assert!(!state.combatants.iter().any(|c| c.resources.contains_key(&format!("{CASTER}:slot:1"))));
        let taken = execute(&mut state, take("counterspell")).unwrap();
        let text = lines(&taken).join("\n");
        assert!(text.contains("PROMPT") && text.contains("answered: takes Counterspell"), "{text}");
        assert!(text.contains("REACTION knight: Counterspell (cast by standard-caster-05)"), "{text}");
        assert!(text.contains("is countered by knight (slot not expended)"), "{text}");
        assert!(state.pending.is_none());
        assert!(!state.combatants.iter().find(|c| c.id == CASTER).unwrap().resources.contains_key(&format!("{CASTER}:slot:1")));
        assert!(replay(&state).matches);
        execute(&mut state, Command::Undo).unwrap();
        assert_eq!(state.pending.as_ref().map(|p| p.kind.as_str()), Some("reaction"), "undo returns to the open window");
        assert_eq!(state.round, before.round);
        assert!(replay(&state).matches);
        execute(&mut state, decline()).unwrap();
        assert!(state.pending.is_none());
        assert!(state.combatants.iter().find(|c| c.id == CASTER).unwrap().resources.contains_key(&format!("{CASTER}:slot:1")));
    }

    #[test]
    fn an_old_state_leaves_spell_cast_windows_off() {
        let (_, state) = old_state();
        assert!(!state.settings.spell_cast_windows);
    }

    #[test]
    fn an_old_state_leaves_teleport_near_off_and_a_new_one_records_it() {
        let (_, state) = old_state();
        assert!(!state.settings.teleport_near);
        let (state, _) = start(spec(PcDice::Engine)).unwrap();
        assert!(state.settings.teleport_near);
        assert!(serde_json::to_string(&state.settings).unwrap().contains(r#""teleport_near":true"#));
    }

    #[test]
    fn an_old_state_leaves_long_durations_off_and_a_new_one_records_it() {
        let (_, state) = old_state();
        assert!(!state.settings.long_durations);
        let (state, _) = start(spec(PcDice::Engine)).unwrap();
        assert!(state.settings.long_durations);
        assert!(serde_json::to_string(&state.settings).unwrap().contains(r#""long_durations":true"#));
    }

    #[test]
    fn an_aura_die_added_by_override_rolls_and_replays_and_a_misplaced_one_is_refused() {
        const STRIKER: &str = "standard-striker-03";
        let (mut state, _) = start(spec(PcDice::Engine)).unwrap();
        let aura = |target: Value| {
            serde_json::json!([{"target": target, "change": {"add_dice": "1d4"}, "remaining": "permanent",
                "source": "goblin-warrior", "bearer": STRIKER, "source_ability": "aura-of-luck"}])
        };
        let set = |state: &mut EncounterState, value: Value| {
            execute(state, Command::Override { path: "active_modifiers".into(), value: Some(value), reason: "inside the aura".into(), rule: None, standing: false })
        };
        let refused = set(&mut state, aura(Value::from("armor_class"))).unwrap_err();
        assert!(refused.to_string().contains("add_dice 1d4 can only change attack_rolls or saving_throws, not armor_class"), "{refused}");
        assert!(state.active_modifiers.is_empty());
        set(&mut state, aura(Value::from("attack_rolls"))).unwrap();
        turn_of(&mut state, STRIKER);
        let out = execute(&mut state, swing(STRIKER, "longbow", "goblin-warrior")).unwrap();
        let attack = lines(&out).into_iter().find(|l| l.contains(" vs AC ")).unwrap();
        assert!(attack.contains(" (+1d4 aura-of-luck: "), "{attack}");
        assert!(crate::log::render_public(&state.log).contains(" (+1d4 aura-of-luck: "), "the die shows where the attack total does");
        let clear = Command::Override { path: "active_modifiers.0".into(), value: None, reason: "left the aura".into(), rule: None, standing: false };
        execute(&mut state, clear).unwrap();
        assert!(state.active_modifiers.is_empty());
        assert!(replay(&state).matches);
    }
}
