//! Live encounters: one command in, one log entry and one response out.
//!
//! `start` builds an encounter and its initial snapshot. `execute` gates a
//! command on the encounter's pending prompt and end state, applies it,
//! appends it to the command history, and writes its log entry. `undo` and
//! `replay` re-execute the history from the initial snapshot, so the state is
//! always a fold over the commands that made it.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::digest::{self, wounded};
use crate::encounter::{
    ActionEconomy, CommandRecord, Combatant, EncounterEnd, EncounterSettings, EncounterState, EndState,
    PcDice, PendingPrompt, PlayerDice, RangeBand, Side, StatBlockSnapshot, PARTY_SIDE,
};
use crate::engine::{self, EngineError, EngineEvent, InitiativeOptions, StepRequest};
use crate::log::{LogEntry, LogLine};
use crate::override_path;
use crate::rng::{RngState, Stream};
use crate::stat_block::StatBlock;
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
    },
    Move {
        actor: String,
        #[serde(default)]
        target: Option<String>,
        band: RangeBand,
        #[serde(default)]
        feet: Option<u32>,
    },
    Respond {
        #[serde(default)]
        values: Vec<u32>,
        #[serde(default)]
        decline: bool,
    },
    EndTurn,
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
    pub fn name(&self) -> &'static str {
        match self {
            Command::Action { .. } => "action",
            Command::Move { .. } => "move",
            Command::Respond { .. } => "respond",
            Command::EndTurn => "end-turn",
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
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut sides: Vec<Side> = Vec::new();
    let mut combatants = Vec::new();
    let mut bands = Vec::new();
    for entrant in &spec.entrants {
        if entrant.band == RangeBand::Engaged {
            return Err(LiveError::Invalid("starting bands are near or far; engage with `bench move`".into()));
        }
        let block = &entrant.stat_block;
        let n = seen.entry(block.id.clone()).or_default();
        *n += 1;
        let (id, name) = if counts[&block.id] > 1 {
            (format!("{}-{n}", block.id), format!("{} {n}", block.name))
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
        settings: EncounterSettings { pc_dice: spec.pc_dice },
        ended: None,
        initial_snapshot: None,
        log: vec![],
        tamper_hash: String::new(),
        player_dice: PlayerDice::default(),
    };
    let initiative = engine::roll_initiative(
        &mut state,
        InitiativeOptions { individual: spec.individual_initiative, surprised: surprised.clone(), pc_dice: false },
    )?;

    let mut applied = Applied::none();
    let pc_dice = match spec.pc_dice {
        PcDice::Engine => "engine",
        PcDice::Ask => "ask",
    };
    applied.push(
        "start",
        None,
        LogLine::replaced(
            format!("START encounter {} (seed {}, pc dice {pc_dice}, engine {ENGINE_VERSION})", spec.id, spec.seed),
            format!("START encounter {} (pc dice {pc_dice}, engine {ENGINE_VERSION})", spec.id),
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
    for e in initiative {
        let line = LogLine::same(e.message.clone());
        applied.items.push((e, line));
    }
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
    let round = state.round;
    let turn = state.active_turn.clone();
    let applied = apply(state, &cmd)?;
    Ok(write_entry(state, Some(&cmd), applied, round, turn))
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
        Command::Action { actor, action, targets } => run_engine(
            state,
            StepRequest::Action { actor_id: actor.clone(), action_id: action.clone(), target_ids: targets.clone() },
            vec![],
            false,
        ),
        Command::Move { actor, target, band, feet } => {
            let feet = match feet {
                Some(f) => *f,
                None => remaining_speed(state, actor)?,
            };
            run_engine(
                state,
                StepRequest::Move { actor_id: actor.clone(), target_id: target.clone(), band: *band, feet },
                vec![],
                false,
            )
        }
        Command::EndTurn => run_engine(state, StepRequest::EndTurn, vec![], false),
        Command::Respond { values, decline } => respond(state, values, *decline),
        Command::End { reason } => {
            state.ended = Some(EncounterEnd { round: state.round, reason: reason.clone() });
            Ok(Applied::line(
                "end",
                None,
                LogLine::same(format!("END encounter ended by agent judgment: {reason}")),
            ))
        }
        Command::Override { path, value, reason, rule } => {
            let target = override_path::apply(state, path, value.clone()).map_err(LiveError::Invalid)?;
            let tag = rule_tag(rule);
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
            Ok(Applied::line("override", target.combatant.as_deref(), line))
        }
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
            add_combatant(state, stat_block, side_id, *band, engaged_with.as_deref())
        }
        Command::Remove { id, reason } => remove_combatant(state, id, reason),
        Command::Undo => Err(LiveError::Invalid("undo is not replayable as a single step".into())),
    }
}

/// Resolve an engine request. With `pc_dice = ask`, the request runs on a copy;
/// if a PC roll found no face, the copy is discarded and a dice prompt holds the
/// request until `bench respond` supplies faces and it runs again.
fn run_engine(
    state: &mut EncounterState,
    request: StepRequest,
    faces: Vec<u32>,
    engine_fills_rest: bool,
) -> Result<Applied, LiveError> {
    let mut trial = state.clone();
    if !matches!(request, StepRequest::Respond { .. }) {
        trial.pending = None;
    }
    trial.player_dice = PlayerDice { faces: faces.clone().into(), engine_fills_rest, ..Default::default() };
    let response = engine::step(&mut trial, request.clone())?;
    if let Some((sides, owner, count)) = trial.player_dice.next_prompt() {
        let message = format!("{owner}: enter {count} d{sides} face{}", if count == 1 { "" } else { "s" });
        state.pending = Some(PendingPrompt {
            id: format!("dice-{}-{}", state.rng.cursor, faces.len()),
            kind: "dice".into(),
            actor_id: owner.clone(),
            message: message.clone(),
            sides: Some(sides),
            remaining: count,
            metadata: serde_json::json!({ "request": request, "faces": faces }),
        });
        return Ok(Applied::line("prompt", Some(&owner), LogLine::same(format!("PROMPT {message}"))));
    }
    let player_dice = trial.player_dice.used > 0;
    trial.player_dice = PlayerDice::default();
    *state = trial;
    let mut applied = Applied::none();
    for event in response.events {
        let line = event_line(state, &event, player_dice);
        applied.items.push((event, line));
    }
    Ok(applied)
}

fn respond(state: &mut EncounterState, values: &[u32], decline: bool) -> Result<Applied, LiveError> {
    let prompt = state.pending.clone().ok_or_else(|| LiveError::Invalid("there is no pending prompt".into()))?;
    if prompt.kind != "dice" {
        let mut applied = run_engine(state, StepRequest::Respond { values: values.to_vec(), accept: !decline }, vec![], false)?;
        let answer = if decline { "declined".to_string() } else { format!("answered {values:?}") };
        applied.items.insert(0, prompt_line(&prompt, &answer));
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
    let request: StepRequest = serde_json::from_value(prompt.metadata["request"].clone())
        .map_err(|e| LiveError::Invalid(format!("the pending prompt is unreadable: {e}")))?;
    let mut faces: Vec<u32> = serde_json::from_value(prompt.metadata["faces"].clone()).unwrap_or_default();
    let answer = if decline {
        "declined, the engine rolls the rest".to_string()
    } else {
        faces.extend_from_slice(values);
        format!("answered {} (player dice)", values.iter().map(u32::to_string).collect::<Vec<_>>().join(", "))
    };
    state.pending = None;
    let mut applied = run_engine(state, request, faces, decline)?;
    applied.items.insert(0, prompt_line(&prompt, &answer));
    Ok(applied)
}

fn prompt_line(prompt: &PendingPrompt, answer: &str) -> (EngineEvent, LogLine) {
    let text = format!("PROMPT {} {answer}", prompt.message);
    (
        EngineEvent {
            kind: "prompt_answered".into(),
            message: text.clone(),
            actor_id: Some(prompt.actor_id.clone()),
            target_id: None,
            roll: None,
            total: None,
            damage: None,
        },
        LogLine::same(text),
    )
}

fn event_line(state: &EncounterState, e: &EngineEvent, player_dice: bool) -> LogLine {
    let owner = match e.kind.as_str() {
        "save" | "concentration_save" => e.target_id.as_deref(),
        _ => e.actor_id.as_deref(),
    };
    let mut text = e.message.clone();
    if let Some(roll) = e.roll {
        text.push_str(&format!(" [d20 {roll}]"));
    }
    let by_pc = owner.is_some_and(|o| state.is_party(o) && is_pc(state, o));
    if player_dice && by_pc && (e.roll.is_some() || e.damage.is_some()) {
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
        ("recharge", _, _) if !state.is_party(e.actor_id.as_deref().unwrap_or_default()) => LogLine::hidden(text),
        _ => LogLine::same(text),
    }
}

fn is_pc(state: &EncounterState, id: &str) -> bool {
    state
        .combatants
        .iter()
        .find(|c| c.id == id)
        .and_then(|c| state.stat_blocks.iter().find(|s| s.id == c.stat_block_id))
        .is_some_and(|s| s.stat_block.class.is_some())
}

fn add_combatant(
    state: &mut EncounterState,
    block: &StatBlock,
    side_id: &str,
    band: Option<RangeBand>,
    engaged_with: Option<&str>,
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
    let natural = stream.roll_die(20);
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
        LogLine::same(format!("ADD {id} ({name}) joins {side_id}, initiative {total}, {position}")),
    );
    applied.items[0].0.roll = Some(natural);
    applied.items[0].0.total = Some(total);
    Ok(applied)
}

fn remove_combatant(state: &mut EncounterState, id: &str, reason: &str) -> Result<Applied, LiveError> {
    if !state.combatants.iter().any(|c| c.id == id) {
        return Err(LiveError::NotFound(format!("combatant {id} not found")));
    }
    let mut applied = Applied::line("remove", Some(id), LogLine::same(format!("REMOVE {id} (reason: {reason})")));
    if state.active_turn.as_deref() == Some(id) && state.initiative_order.len() > 1 {
        let turn = run_engine(state, StepRequest::EndTurn, vec![], true)?;
        applied.items.extend(turn.items);
    }
    state.combatants.retain(|c| c.id != id);
    state.initiative_order.retain(|c| c != id);
    state.initiative_rolls.remove(id);
    for c in &mut state.combatants {
        c.engagement.retain(|e| e != id);
        c.ranges.remove(id);
    }
    if state.active_turn.as_deref() == Some(id) {
        state.active_turn = None;
    }
    Ok(applied)
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
    let walk = state
        .stat_blocks
        .iter()
        .find(|s| s.id == c.stat_block_id)
        .and_then(|s| s.stat_block.speeds.get("walk"))
        .map_or(0, |s| s.feet);
    Ok(walk.saturating_sub(c.action_economy.movement_used_feet))
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
        }
    }

    fn attack(state: &EncounterState) -> Command {
        let actor = state.active_turn.clone().expect("active turn");
        let (action, target) = if actor == "standard-striker-03" {
            ("longbow", "goblin-warrior")
        } else {
            ("shortbow", "standard-striker-03")
        };
        Command::Action { actor, action: action.into(), targets: vec![target.into()] }
    }

    fn lines(outcome: &Outcome) -> Vec<String> {
        outcome.events.iter().map(|e| e.line.clone()).collect()
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
        execute(&mut state, Command::EndTurn).unwrap();
        execute(
            &mut state,
            Command::Override {
                path: "goblin-warrior.hp".into(),
                value: Some(Value::from(3)),
                reason: "test".into(),
                rule: None,
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
    fn pending_dice_prompt_gates_commands() {
        let (mut state, _) = start(spec(PcDice::Ask)).unwrap();
        if state.active_turn.as_deref() != Some("standard-striker-03") {
            execute(&mut state, Command::EndTurn).unwrap();
        }
        let asked = { let cmd = attack(&state); execute(&mut state, cmd) }.unwrap();
        let prompt = asked.pending.clone().expect("dice prompt");
        assert_eq!(prompt.kind, "dice");
        assert!(asked.events[0].line.starts_with("PROMPT "));
        assert!(matches!(execute(&mut state, Command::EndTurn), Err(LiveError::Pending(_))));
        assert!(matches!(
            execute(&mut state, Command::Respond { values: vec![1; prompt.remaining as usize + 1], decline: false }),
            Err(LiveError::Invalid(_))
        ));

        let mut answered = vec![];
        while let Some(p) = state.pending.clone() {
            let face = p.sides.unwrap();
            let out = execute(&mut state, Command::Respond { values: vec![face; p.remaining as usize], decline: false }).unwrap();
            answered.extend(lines(&out));
        }
        assert!(answered.iter().any(|l| l.contains("(player dice)")), "{answered:?}");
        execute(&mut state, Command::EndTurn).unwrap();
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
            execute(state, Command::Override { path: path.into(), value, reason: "t".into(), rule: None })
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
}
