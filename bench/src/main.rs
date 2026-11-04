//! Thin `bench` CLI binary.
//!
//! Every command starts, does one thing, prints a JSON envelope to stdout, and
//! exits. The binary delegates all rules/state work to `bench-core`.

use std::path::PathBuf;
use std::process;

use bench_core::{
    baseline, embedded,
    effect::Lighting,
    batch::{self, BatchError, BatchSpec},
    config::{Config, OutputTarget},
    encounter::{EncounterState, PcDice, RangeBand, StandingAssumption, PARTY_SIDE},
    engine::{LegendaryUse, OpportunityAttack},
    error::{CodedError, ErrorCode},
    live::{self, Command, Entrant, LiveError, StartSpec},
    log, paths,
    schema::{schema_json, validate_value},
    stat_block::StatBlock,
    store::{self, Index, IndexEntry, VersionCheck},
    Envelope, ENGINE_VERSION,
};
use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use serde::Serialize;
use serde_json::Value;

const DEFAULT_ASSUME_REASON: &str = "set on the bench batch command line";

#[derive(Parser)]
#[command(name = "bench")]
#[command(version = ENGINE_VERSION)]
#[command(about = "One-shot CLI for homebrew-bench combat testing")]
#[command(disable_help_subcommand = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Args, Clone)]
struct At {
    /// Encounter id; defaults to the most recently started running encounter.
    #[arg(long, short = 'e')]
    encounter: Option<String>,
}

#[derive(Args, Clone)]
struct Reason {
    /// Why the agent made this call; written to the log.
    #[arg(long)]
    reason: String,
    /// House rule slug, validated against `./.bench/house-rules/` and `~/.bench/house-rules/`.
    #[arg(long)]
    rule: Option<String>,
}

#[derive(Clone, Copy, ValueEnum)]
enum Band {
    Engaged,
    Near,
    Far,
}

impl From<Band> for RangeBand {
    fn from(band: Band) -> Self {
        match band {
            Band::Engaged => RangeBand::Engaged,
            Band::Near => RangeBand::Near,
            Band::Far => RangeBand::Far,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum DiceMode {
    Engine,
    Ask,
}

#[derive(Clone, Copy, ValueEnum)]
#[value(rename_all = "snake_case")]
enum LightingArg {
    Unspecified,
    DimLight,
    BrightLight,
    Sunlight,
}

impl From<LightingArg> for Lighting {
    fn from(arg: LightingArg) -> Self {
        match arg {
            LightingArg::Unspecified => Lighting::Unspecified,
            LightingArg::DimLight => Lighting::DimLight,
            LightingArg::BrightLight => Lighting::BrightLight,
            LightingArg::Sunlight => Lighting::Sunlight,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum Output {
    Home,
    Workspace,
}

#[derive(Subcommand)]
enum Commands {
    /// Show help. Use `--json` for the machine-readable command catalog.
    Help {
        /// Output machine-readable command catalog.
        #[arg(long)]
        json: bool,
    },
    /// Print JSON Schema or validate a monster stat block file.
    Schema {
        #[command(subcommand)]
        action: SchemaAction,
    },
    /// Start a live encounter and roll initiative.
    Start {
        /// Combatants as `[side=]path[@near|far]`. Side defaults to `party` for
        /// stat blocks with a class and `monsters` otherwise.
        #[arg(required = true)]
        combatants: Vec<String>,
        /// Combatant id or side id that starts surprised. Repeatable.
        #[arg(long)]
        surprised: Vec<String>,
        /// Seed for the encounter's ChaCha8 stream; fresh entropy if omitted.
        #[arg(long)]
        seed: Option<u64>,
        /// Who rolls PC dice.
        #[arg(long, value_enum, default_value = "engine")]
        pc_dice: DiceMode,
        /// The whole encounter's lighting; lighting traits are off when unspecified.
        #[arg(long, value_enum, default_value = "unspecified")]
        lighting: LightingArg,
        /// Where to write the state file and log; defaults to `config.toml`.
        #[arg(long, value_enum)]
        output: Option<Output>,
        /// Roll one initiative per side instead of per combatant.
        #[arg(long)]
        group_initiative: bool,
        /// Readable encounter id; derived from the first monster if omitted.
        #[arg(long)]
        id: Option<String>,
    },
    /// Take an action, such as an attack, on the active combatant's turn.
    Action {
        actor: String,
        action: String,
        /// Target combatant ids.
        targets: Vec<String>,
        /// Spell slot level for every leveled spell the action casts, for a stat block with spell slots.
        #[arg(long)]
        slot: Option<u8>,
        /// Spend the extra action a modifier grants (Haste) instead of the action; Haste's allows one attack.
        #[arg(long)]
        extra: bool,
        #[command(flatten)]
        at: At,
    },
    /// Move a combatant to a range band, relative to one target or to everyone.
    Move {
        actor: String,
        #[arg(value_enum)]
        band: Band,
        /// Move relative to this combatant only. Required to move to engaged.
        #[arg(long)]
        target: Option<String>,
        /// Feet spent; defaults to the remaining walking speed.
        #[arg(long)]
        feet: Option<u32>,
        /// A creature whose reach the move leaves takes its Opportunity Attack,
        /// with its first at-will melee attack or the named one. Repeat to name
        /// several; they resolve in the order given, before the move.
        #[arg(long, value_name = "REACTOR[:ATTACK]", value_parser = parse_opportunity)]
        opportunity: Vec<OpportunityAttack>,
        #[command(flatten)]
        at: At,
    },
    /// Take the Disengage action: the actor's moves this turn provoke no Opportunity Attacks.
    Disengage {
        actor: String,
        /// Spend the extra action a modifier grants (Haste) instead of the action.
        #[arg(long)]
        extra: bool,
        #[command(flatten)]
        at: At,
    },
    /// Answer the pending prompt.
    Respond {
        /// Die faces or choices the prompt asks for.
        values: Vec<u32>,
        /// Decline the prompt; for dice prompts the engine rolls the rest, and a
        /// reaction prompt passes its window.
        #[arg(long)]
        decline: bool,
        /// Take the reaction a reaction prompt offers. Name its id when more than one is offered.
        #[arg(long, value_name = "ID", num_args = 0..=1, default_missing_value = "")]
        take: Option<String>,
        #[command(flatten)]
        at: At,
    },
    /// End the active turn. Legendary actions named with --legendary resolve
    /// after the turn ends and before the next one starts.
    EndTurn {
        /// A legendary action taken between the turns, at the listed targets.
        /// Repeat for several; they resolve in the order given and each costs
        /// one use. Any that is not allowed fails the whole command.
        #[arg(long, value_name = "HOLDER:ACTION[=TARGET,...]", value_parser = parse_legendary)]
        legendary: Vec<LegendaryUse>,
        #[command(flatten)]
        at: At,
    },
    /// End the encounter early as agent judgment.
    End {
        #[arg(long)]
        reason: String,
        #[command(flatten)]
        at: At,
    },
    /// Print the encounter digest.
    State {
        /// Apply the public view filter.
        #[arg(long)]
        public: bool,
        #[command(flatten)]
        at: At,
    },
    /// List encounters in the index.
    List,
    /// Prune ended encounters per `config.toml`.
    Clean,
    /// Override a state field at a dotted path, such as `goblin-2.hp 14`.
    Override {
        path: String,
        /// New value as JSON; bare words are strings.
        #[arg(required_unless_present = "clear", conflicts_with = "clear")]
        value: Option<String>,
        /// Clear the field instead of setting it.
        #[arg(long)]
        clear: bool,
        /// Record the override as a standing assumption; later log lines about its subject name it.
        #[arg(long)]
        standing: bool,
        #[command(flatten)]
        why: Reason,
        #[command(flatten)]
        at: At,
    },
    /// Roll a dice expression from the encounter's stream as agent judgment.
    Roll {
        expression: String,
        #[command(flatten)]
        why: Reason,
        #[command(flatten)]
        at: At,
    },
    /// Add a combatant mid-fight.
    Add {
        stat_block: String,
        #[arg(long)]
        side: String,
        #[arg(long, value_enum, conflicts_with = "engaged_with")]
        band: Option<Band>,
        #[arg(long)]
        engaged_with: Option<String>,
        #[command(flatten)]
        at: At,
    },
    /// Remove a combatant mid-fight.
    Remove {
        id: String,
        #[arg(long)]
        reason: String,
        #[command(flatten)]
        at: At,
    },
    /// Revert the last mutating command.
    Undo {
        #[command(flatten)]
        at: At,
    },
    /// Re-execute the command history and compare state hashes.
    Replay {
        #[command(flatten)]
        at: At,
    },
    /// Run a batch of fights for one monster against a party and write the report.
    Batch {
        /// Monster stat block path, or an SRD fixture id such as `ogre`.
        monster: String,
        /// Party level or range, such as `5` or `1-20`.
        #[arg(long, default_value = "1-20")]
        levels: String,
        /// Fights per level.
        #[arg(long, default_value_t = batch::DEFAULT_FIGHTS)]
        fights: u32,
        /// Round cap; a fight that reaches it is a stalemate.
        #[arg(long, default_value_t = batch::DEFAULT_MAX_ROUNDS)]
        max_rounds: u32,
        /// Party folder name, from `./.bench/parties/<name>/` then `~/.bench/parties/<name>/`.
        #[arg(long)]
        party: Option<String>,
        /// Master seed; fresh entropy if omitted. Always echoed in the output.
        #[arg(long)]
        seed: Option<u64>,
        /// Replay one fight by its global index and print its full log. Needs `--seed`.
        #[arg(long, requires = "seed")]
        fight: Option<u64>,
        /// Standing assumption `path=value`, such as `abilities.swallow=ignored` or `policy.weights.control=0.5`. Repeatable.
        #[arg(long)]
        assume: Vec<String>,
        /// Reason recorded for every `--assume`.
        #[arg(long, default_value = DEFAULT_ASSUME_REASON)]
        assume_reason: String,
        /// JSON file holding a list of `{path, value, reason}` standing assumptions.
        #[arg(long)]
        assumptions: Option<String>,
        /// Every fight's lighting; lighting traits are off when unspecified.
        #[arg(long, value_enum, default_value = "unspecified")]
        lighting: LightingArg,
        /// Folder for `report.html` and `logs/`; defaults to `reports/<monster>-seed-<seed>/` under the output directory.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Where the default report folder lives; defaults to `config.toml`.
        #[arg(long, value_enum)]
        output: Option<Output>,
    },
    /// Read-only view of the SRD fixtures embedded in `bench`.
    Fixture {
        #[command(subcommand)]
        action: FixtureAction,
    },
    /// Read-only view of the standard party embedded in `bench`.
    Party {
        #[command(subcommand)]
        action: PartyAction,
    },
    /// Inspect or regenerate the CR baseline curve.
    Baseline {
        #[command(subcommand)]
        action: BaselineAction,
    },
    /// Print the plain-text log.
    Log {
        /// Apply the public view filter.
        #[arg(long)]
        public: bool,
        #[command(flatten)]
        at: At,
    },
}

#[derive(Subcommand)]
enum FixtureAction {
    /// List embedded fixture ids with name and challenge rating.
    List,
    /// Print one embedded fixture stat block.
    Show {
        /// Fixture id, such as `ogre`.
        id: String,
    },
}

#[derive(Subcommand)]
enum PartyAction {
    /// Print the embedded standard party stat blocks.
    Show {
        /// Only this party level.
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=20))]
        level: Option<u8>,
    },
}

#[derive(Subcommand)]
enum BaselineAction {
    /// Compare the embedded baseline's input fingerprints with the ones this binary computes now.
    Status,
    /// Fight every embedded fixture against the standard party and write the curve. Rebuild `bench` after changing `data/` first.
    Generate {
        /// Where to write the curve.
        #[arg(long, default_value = "data/baseline/curve.json")]
        out: PathBuf,
        /// Fights per fixture and party level.
        #[arg(long, default_value_t = batch::DEFAULT_FIGHTS)]
        fights: u32,
        /// Base seed; each fixture derives its own from it and its id.
        #[arg(long, default_value_t = baseline::DEFAULT_BASE_SEED)]
        seed: u64,
        /// Worker threads; defaults to the available cores.
        #[arg(long)]
        threads: Option<usize>,
    },
}

#[derive(Subcommand)]
enum SchemaAction {
    /// Print the JSON Schema for the monster stat block.
    Monster,
    /// Validate a stat block JSON file.
    Validate {
        /// Path to the stat block JSON file.
        path: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Some(Commands::Help { json: true }) => ok(command_catalog()),
        Some(Commands::Help { json: false }) => Err(CodedError::new(
            ErrorCode::InvalidArgument,
            "use --help for usage, or help --json for the command catalog",
        )),
        Some(Commands::Schema { action }) => run_schema(action),
        Some(command) => run(command),
        None => Err(CodedError::new(ErrorCode::InvalidArgument, "no command given; use --help or help --json")),
    };
    match result {
        Ok(output) => {
            println!("{output}");
            process::exit(0);
        }
        Err(err) => {
            let envelope: Envelope<Value> = err.envelope();
            println!("{}", serde_json::to_string_pretty(&envelope).unwrap());
            process::exit(err.code.exit_code());
        }
    }
}

fn ok<T: Serialize>(data: T) -> Result<String, CodedError> {
    serde_json::to_string_pretty(&Envelope::ok(data))
        .map_err(|e| CodedError::new(ErrorCode::Internal, format!("failed to serialize envelope: {e}")))
}

fn cwd() -> Result<PathBuf, CodedError> {
    std::env::current_dir().map_err(|e| CodedError::new(ErrorCode::Internal, format!("no working directory: {e}")))
}

fn config() -> Result<Config, CodedError> {
    Ok(Config::load(paths::config_file()?)?)
}

fn coded(err: LiveError, override_cmd: bool) -> CodedError {
    let code = match &err {
        LiveError::Invalid(_) if override_cmd => ErrorCode::InvalidOverride,
        LiveError::Invalid(_) => ErrorCode::InvalidArgument,
        LiveError::NotFound(_) => ErrorCode::NotFound,
        LiveError::Illegal(_) => ErrorCode::IllegalAction,
        LiveError::Pending(_) => ErrorCode::PendingPrompt,
        LiveError::Ended(_) => ErrorCode::EncounterEnded,
    };
    CodedError::new(code, err.to_string())
}

fn read_stat_block(path: &str) -> Result<StatBlock, CodedError> {
    if !std::path::Path::new(path).exists() {
        if let Some(text) = embedded::stat_block_json(path) {
            return serde_json::from_str(text)
                .map_err(|e| CodedError::new(ErrorCode::Internal, format!("embedded stat block `{path}` is invalid: {e}")));
        }
    }
    let text = std::fs::read_to_string(path)
        .map_err(|e| CodedError::new(ErrorCode::NotFound, format!("failed to read {path}: {e}")))?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|e| CodedError::new(ErrorCode::InvalidJson, format!("invalid json in {path}: {e}")))?;
    let errors = validate_value::<StatBlock>(&value)
        .map_err(|e| CodedError::new(ErrorCode::Internal, format!("failed to validate {path}: {e}")))?;
    if !errors.is_empty() {
        let details = serde_json::to_value(errors).unwrap_or_default();
        return Err(CodedError::new(ErrorCode::SchemaValidationFailed, format!("{path} failed schema validation"))
            .with_details(details));
    }
    serde_json::from_value(value)
        .map_err(|e| CodedError::new(ErrorCode::SchemaValidationFailed, format!("{path} is not a stat block: {e}")))
}

fn parse_opportunity(spec: &str) -> Result<OpportunityAttack, String> {
    let (reactor, attack) = match spec.split_once(':') {
        Some((reactor, attack)) => (reactor, Some(attack)),
        None => (spec, None),
    };
    if reactor.is_empty() || attack.is_some_and(str::is_empty) {
        return Err(format!("`{spec}` is not REACTOR or REACTOR:ATTACK"));
    }
    Ok(OpportunityAttack { reactor_id: reactor.into(), attack_id: attack.map(Into::into) })
}

fn parse_legendary(spec: &str) -> Result<LegendaryUse, String> {
    let bad = || format!("`{spec}` is not HOLDER:ACTION or HOLDER:ACTION=TARGET,...");
    let (declared, targets) = match spec.split_once('=') {
        Some((declared, targets)) => (declared, Some(targets)),
        None => (spec, None),
    };
    let (holder, action) = declared.split_once(':').ok_or_else(bad)?;
    let target_ids: Vec<String> = targets.map(|t| t.split(',').map(str::to_string).collect()).unwrap_or_default();
    if holder.is_empty() || action.is_empty() || target_ids.iter().any(String::is_empty) {
        return Err(bad());
    }
    Ok(LegendaryUse { holder_id: holder.into(), action_id: action.into(), target_ids })
}

fn parse_entrant(spec: &str) -> Result<Entrant, CodedError> {
    let (side, rest) = match spec.split_once('=') {
        Some((side, rest)) => (Some(side.to_string()), rest),
        None => (None, spec),
    };
    let (path, band) = match rest.rsplit_once('@') {
        Some((path, "near")) => (path, RangeBand::Near),
        Some((path, "far")) => (path, RangeBand::Far),
        Some((_, other)) => {
            return Err(CodedError::new(
                ErrorCode::InvalidArgument,
                format!("`{other}` in `{spec}` is not a starting band; use near or far"),
            ))
        }
        None => (rest, RangeBand::Near),
    };
    let stat_block = read_stat_block(path)?;
    let side = side.unwrap_or_else(|| if stat_block.class.is_some() { PARTY_SIDE } else { "monsters" }.into());
    Ok(Entrant { stat_block, side, band })
}

fn run(command: Commands) -> Result<String, CodedError> {
    match command {
        Commands::Start { combatants, surprised, seed, pc_dice, lighting, output, group_initiative, id } => {
            let entrants = combatants.iter().map(|c| parse_entrant(c)).collect::<Result<Vec<_>, _>>()?;
            let cwd = cwd()?;
            let config = config()?;
            let output = match output {
                Some(Output::Home) => OutputTarget::Home,
                Some(Output::Workspace) => OutputTarget::Workspace,
                None => config.output,
            };
            let dir = paths::encounters_dir(output, &cwd)?;
            let mut index = Index::load()?;
            if !config.keep_encounters {
                prune(&mut index, config.delete_logs)?;
            }
            let id = match id {
                Some(id) if index.find(&id).is_some() || store::files_for(&dir, &id).1.exists() => {
                    return Err(CodedError::new(ErrorCode::InvalidArgument, format!("encounter `{id}` already exists")))
                }
                Some(id) => id,
                None => {
                    let stem = entrants
                        .iter()
                        .find(|e| e.side != PARTY_SIDE)
                        .or(entrants.first())
                        .map(|e| e.stat_block.id.clone())
                        .unwrap_or_default();
                    index.next_id(&stem, &dir)
                }
            };
            let spec = StartSpec {
                id: id.clone(),
                origin_directory: cwd.display().to_string(),
                entrants,
                seed: seed.unwrap_or_else(rand::random),
                surprised,
                individual_initiative: !group_initiative,
                pc_dice: match pc_dice {
                    DiceMode::Engine => PcDice::Engine,
                    DiceMode::Ask => PcDice::Ask,
                },
                lighting: lighting.into(),
                reaction_windows: true,
            };
            let (state, outcome) = live::start(spec).map_err(|e| coded(e, false))?;
            let (state_path, log_path) = store::files_for(&dir, &id);
            let entry = IndexEntry {
                id,
                state_path,
                log_path,
                origin_directory: state.origin_directory.clone(),
                started_unix: store::now_unix(),
                ended: false,
            };
            store::save(&state, &entry)?;
            index.upsert(entry);
            index.save()?;
            ok(outcome)
        }
        Commands::Action {
            actor,
            action,
            targets,
            slot,
            extra,
            at,
        } => step(
            &at,
            Command::Action {
                actor,
                action,
                targets,
                slot,
                extra,
            },
        ),
        Commands::Disengage { actor, extra, at } => step(&at, Command::Disengage { actor, extra }),
        Commands::Move { actor, band, target, feet, opportunity, at } => {
            step(&at, Command::Move { actor, target, band: band.into(), feet, opportunity })
        }
        Commands::Respond { values, decline, take, at } => step(&at, Command::Respond { values, decline, take }),
        Commands::EndTurn { legendary, at } => step(&at, Command::EndTurn { legendary }),
        Commands::End { reason, at } => step(&at, Command::End { reason }),
        Commands::Override { path, value, clear: _, standing, why, at } => {
            check_rule(&why.rule)?;
            let value = value.map(|v| serde_json::from_str(&v).unwrap_or(Value::String(v)));
            step(&at, Command::Override { path, value, reason: why.reason, rule: why.rule, standing })
        }
        Commands::Roll { expression, why, at } => {
            check_rule(&why.rule)?;
            step(&at, Command::Roll { expression, reason: why.reason, rule: why.rule })
        }
        Commands::Add { stat_block, side, band, engaged_with, at } => {
            let stat_block = Box::new(read_stat_block(&stat_block)?);
            step(&at, Command::Add { stat_block, side, band: band.map(Into::into), engaged_with })
        }
        Commands::Remove { id, reason, at } => step(&at, Command::Remove { id, reason }),
        Commands::Undo { at } => step(&at, Command::Undo),
        Commands::State { public, at } => {
            let (_, state) = open(&at, VersionCheck::Require)?;
            ok(serde_json::json!({
                "encounter": state.id,
                "digest": live::digest_of(&state, public),
                "pending": if public { None } else { state.pending.clone() },
            }))
        }
        Commands::Log { public, at } => {
            let (entry, state) = open(&at, VersionCheck::Require)?;
            let text = if public { log::render_public(&state.log) } else { log::render(&state.log) };
            let mut out = serde_json::json!({ "encounter": state.id, "public": public, "log": text });
            if !public {
                out["log_path"] = serde_json::to_value(&entry.log_path).unwrap_or_default();
            }
            ok(out)
        }
        Commands::Replay { at } => {
            let (_, state) = open(&at, VersionCheck::Ignore)?;
            ok(live::replay(&state))
        }
        Commands::List => {
            let index = Index::load()?;
            let rows: Vec<Value> = index
                .encounters
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "id": e.id,
                        "origin_directory": e.origin_directory,
                        "started_unix": e.started_unix,
                        "ended": e.ended,
                        "state_path": e.state_path,
                        "log_path": e.log_path,
                        "state_exists": e.state_path.exists(),
                    })
                })
                .collect();
            ok(serde_json::json!({ "encounters": rows }))
        }
        Commands::Clean => {
            let config = config()?;
            let mut index = Index::load()?;
            let removed = prune(&mut index, config.delete_logs)?;
            ok(serde_json::json!({ "removed": removed, "logs_deleted": config.delete_logs }))
        }
        Commands::Batch { monster, levels, fights, max_rounds, party, seed, fight, assume, assume_reason, assumptions, lighting, out, output } => {
            run_batch(BatchArgs { monster, levels, fights, max_rounds, party, seed, fight, assume, assume_reason, assumptions, lighting, out, output })
        }
        Commands::Fixture { action } => run_fixture(action),
        Commands::Party { action } => run_party(action),
        Commands::Baseline { action } => run_baseline(action),
        Commands::Help { .. } | Commands::Schema { .. } => unreachable!("handled in main"),
    }
}

/// Delete ended encounters, and entries whose state file is gone.
fn prune(index: &mut Index, delete_logs: bool) -> Result<Vec<String>, CodedError> {
    let mut removed = Vec::new();
    for entry in &index.encounters {
        if entry.ended || !entry.state_path.exists() {
            store::delete(entry, delete_logs)?;
            removed.push(entry.id.clone());
        }
    }
    index.encounters.retain(|e| !removed.contains(&e.id));
    index.save()?;
    Ok(removed)
}

fn check_rule(rule: &Option<String>) -> Result<(), CodedError> {
    match rule {
        Some(slug) => store::validate_rule(slug, &cwd()?),
        None => Ok(()),
    }
}

fn open(at: &At, version: VersionCheck) -> Result<(IndexEntry, EncounterState), CodedError> {
    let index = Index::load()?;
    let entry = index.resolve(at.encounter.as_deref())?.clone();
    let state = store::load(&entry.state_path, version)?;
    Ok((entry, state))
}

fn step(at: &At, cmd: Command) -> Result<String, CodedError> {
    let override_cmd = matches!(cmd, Command::Override { .. });
    let (mut entry, mut state) = open(at, VersionCheck::Require)?;
    let outcome = live::execute(&mut state, cmd).map_err(|e| coded(e, override_cmd))?;
    store::save(&state, &entry)?;
    if entry.ended != state.ended.is_some() {
        entry.ended = state.ended.is_some();
        let mut index = Index::load()?;
        index.upsert(entry);
        index.save()?;
    }
    ok(outcome)
}

struct BatchArgs {
    monster: String,
    levels: String,
    fights: u32,
    max_rounds: u32,
    party: Option<String>,
    seed: Option<u64>,
    fight: Option<u64>,
    assume: Vec<String>,
    assume_reason: String,
    assumptions: Option<String>,
    lighting: LightingArg,
    out: Option<PathBuf>,
    output: Option<Output>,
}

fn batch_error(err: BatchError) -> CodedError {
    match &err {
        BatchError::Unresolved(ids) => CodedError::new(ErrorCode::UnmodeledAbilityRefused, err.to_string())
            .with_details(serde_json::json!({ "abilities": ids })),
        BatchError::Invalid(_) => CodedError::new(ErrorCode::InvalidArgument, err.to_string()),
        BatchError::NotFound(_) => CodedError::new(ErrorCode::NotFound, err.to_string()),
    }
}

fn run_batch(args: BatchArgs) -> Result<String, CodedError> {
    let cwd = cwd()?;
    let monster = if std::path::Path::new(&args.monster).is_file() || embedded::stat_block_json(&args.monster).is_some() {
        read_stat_block(&args.monster)?
    } else {
        return Err(CodedError::new(ErrorCode::NotFound, format!("`{}` is neither a stat block file nor an SRD fixture id", args.monster)));
    };
    let (party_name, party) = match &args.party {
        None => ("standard".to_string(), batch::standard_party().map_err(batch_error)?),
        Some(name) => {
            let dir = paths::party_dirs(name, &cwd)?.into_iter().find(|d| d.is_dir()).ok_or_else(|| {
                CodedError::new(ErrorCode::NotFound, format!("party `{name}` not found in ./.bench/parties/ or ~/.bench/parties/"))
            })?;
            (name.clone(), batch::load_party_dir(&dir).map_err(batch_error)?)
        }
    };
    let mut assumptions: Vec<StandingAssumption> = Vec::new();
    if let Some(file) = &args.assumptions {
        let text = std::fs::read_to_string(file)
            .map_err(|e| CodedError::new(ErrorCode::NotFound, format!("failed to read {file}: {e}")))?;
        let list: Vec<StandingAssumption> = serde_json::from_str(&text)
            .map_err(|e| CodedError::new(ErrorCode::InvalidJson, format!("{file} is not a list of {{path, value, reason}}: {e}")))?;
        assumptions.extend(list);
    }
    for item in &args.assume {
        let (path, value) = item.split_once('=').ok_or_else(|| {
            CodedError::new(ErrorCode::InvalidArgument, format!("`{item}` is not `path=value`"))
        })?;
        let value = serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.to_string()));
        assumptions.push(StandingAssumption { path: path.to_string(), value, reason: args.assume_reason.clone() });
    }
    let spec = BatchSpec {
        monster,
        party_name,
        party,
        levels: batch::parse_levels(&args.levels).map_err(batch_error)?,
        fights: args.fights,
        max_rounds: args.max_rounds,
        master_seed: args.seed.unwrap_or_else(rand::random),
        assumptions,
        lighting: args.lighting.into(),
        baseline: match baseline::embedded() {
            Some(Ok(b)) => Some(b),
            Some(Err(e)) => return Err(batch_error(e)),
            None => None,
        },
        invocation: Some(batch::Invocation {
            monster: args.monster.clone(),
            party: args.party.clone(),
            assumptions_file: args.assumptions.clone(),
            assume: args.assume.clone(),
            assume_reason: (args.assume_reason != DEFAULT_ASSUME_REASON).then(|| args.assume_reason.clone()),
        }),
    };
    if let Some(index) = args.fight {
        let result = batch::replay_fight(&spec, index).map_err(batch_error)?;
        let mut out = serde_json::to_value(&result).unwrap_or_default();
        out["master_seed"] = spec.master_seed.into();
        out["replay"] = batch::replay_command(&spec, index).into();
        out["log"] = result.log.unwrap_or_default().into();
        return ok(out);
    }
    let report = batch::run(&spec).map_err(batch_error)?;
    let dir = match args.out {
        Some(dir) => dir,
        None => {
            let output = match args.output {
                Some(Output::Home) => OutputTarget::Home,
                Some(Output::Workspace) => OutputTarget::Workspace,
                None => config()?.output,
            };
            paths::reports_dir(output, &cwd)?.join(format!("{}-seed-{}", spec.monster.id, spec.master_seed))
        }
    };
    let logs_dir = dir.join("logs");
    std::fs::create_dir_all(&logs_dir)
        .map_err(|e| CodedError::new(ErrorCode::Internal, format!("cannot create {}: {e}", logs_dir.display())))?;
    let write = |path: &std::path::Path, text: &str| {
        std::fs::write(path, text).map_err(|e| CodedError::new(ErrorCode::Internal, format!("cannot write {}: {e}", path.display())))
    };
    let report_path = dir.join("report.html");
    write(&report_path, &report.html)?;
    let mut log_paths = Vec::new();
    for (file, text) in &report.logs {
        let path = logs_dir.join(file);
        write(&path, text)?;
        log_paths.push(path.display().to_string());
    }
    let mut out = serde_json::to_value(&report.summary).unwrap_or_default();
    out["report_path"] = report_path.display().to_string().into();
    out["log_paths"] = log_paths.into();
    ok(out)
}

fn run_fixture(action: FixtureAction) -> Result<String, CodedError> {
    match action {
        FixtureAction::List => {
            let rows = baseline::embedded_fixtures()
                .map_err(batch_error)?
                .into_iter()
                .map(|b| serde_json::json!({ "id": b.id, "name": b.name, "challenge_rating": b.challenge_rating }))
                .collect::<Vec<_>>();
            ok(serde_json::json!({ "count": rows.len(), "fixtures": rows }))
        }
        FixtureAction::Show { id } => {
            let text = embedded::fixture_json(&id)
                .ok_or_else(|| CodedError::new(ErrorCode::NotFound, format!("`{id}` is not an embedded fixture; try `bench fixture list`")))?;
            ok(serde_json::from_str::<Value>(text)
                .map_err(|e| CodedError::new(ErrorCode::Internal, format!("embedded fixture `{id}` is invalid: {e}")))?)
        }
    }
}

fn run_party(action: PartyAction) -> Result<String, CodedError> {
    let PartyAction::Show { level } = action;
    let party = batch::standard_party().map_err(batch_error)?;
    match level {
        Some(level) => {
            let members = party.get(&level).ok_or_else(|| CodedError::new(ErrorCode::NotFound, format!("no standard party members at level {level}")))?;
            ok(serde_json::json!({ "party": "standard", "level": level, "members": members }))
        }
        None => ok(serde_json::json!({ "party": "standard", "levels": party })),
    }
}

fn run_baseline(action: BaselineAction) -> Result<String, CodedError> {
    let party = batch::standard_party().map_err(batch_error)?;
    let current = baseline::fingerprints_for(&party, &Default::default());
    match action {
        BaselineAction::Status => {
            let Some(loaded) = baseline::embedded() else {
                return ok(serde_json::json!({ "embedded": false, "current": false, "reasons": ["no baseline was embedded at build time"] }));
            };
            let loaded = loaded.map_err(batch_error)?;
            let reasons = loaded.stale_reasons(&current);
            ok(serde_json::json!({
                "embedded": true,
                "current": reasons.is_empty(),
                "reasons": reasons,
                "fights_per_cell": loaded.fights_per_cell,
                "fixtures": loaded.fixtures.len(),
                "excluded": loaded.excluded,
                "cells": loaded.cells.len(),
                "committed": loaded.fingerprints,
                "computed_now": current,
            }))
        }
        BaselineAction::Generate { out, fights, seed, threads } => {
            let spec = baseline::GenerateSpec {
                fixtures: baseline::embedded_fixtures().map_err(batch_error)?,
                party_name: "standard".into(),
                party: &party,
                base_seed: seed,
                fights,
                max_rounds: batch::DEFAULT_MAX_ROUNDS,
                threads: threads.unwrap_or_else(|| std::thread::available_parallelism().map_or(1, usize::from)),
            };
            let curve = baseline::generate(&spec, &|done, total, id| eprintln!("[{done}/{total}] {id}")).map_err(batch_error)?;
            if let Some(dir) = out.parent().filter(|d| !d.as_os_str().is_empty()) {
                std::fs::create_dir_all(dir).map_err(|e| CodedError::new(ErrorCode::Internal, format!("cannot create {}: {e}", dir.display())))?;
            }
            let text = serde_json::to_string_pretty(&curve)
                .map_err(|e| CodedError::new(ErrorCode::Internal, format!("cannot serialize the curve: {e}")))?;
            std::fs::write(&out, text + "\n")
                .map_err(|e| CodedError::new(ErrorCode::Internal, format!("cannot write {}: {e}", out.display())))?;
            ok(serde_json::json!({
                "path": out.display().to_string(),
                "fixtures": curve.fixtures.len(),
                "excluded": curve.excluded,
                "cells": curve.cells.len(),
                "thin_cells": curve.cells.iter().filter(|c| c.thin).count(),
                "fingerprints": curve.fingerprints,
            }))
        }
    }
}

fn run_schema(action: SchemaAction) -> Result<String, CodedError> {
    let data: Value = match action {
        SchemaAction::Monster => {
            let schema = schema_json::<StatBlock>()
                .map_err(|e| CodedError::new(ErrorCode::Internal, format!("failed to generate monster schema: {e:#}")))?;
            serde_json::from_str(&schema)
                .map_err(|e| CodedError::new(ErrorCode::Internal, format!("generated schema is not valid json: {e}")))?
        }
        SchemaAction::Validate { path } => {
            read_stat_block(&path)?;
            let text = match std::fs::read_to_string(&path) {
                Ok(text) => Some(text),
                Err(_) => embedded::stat_block_json(&path).map(str::to_string),
            };
            let value: Value = text.and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
            let warnings = bench_core::stat_block::bearer_warnings(&value);
            if warnings.is_empty() {
                serde_json::json!({"valid": true, "path": path})
            } else {
                serde_json::json!({"valid": true, "path": path, "warnings": warnings})
            }
        }
    };
    ok(data)
}

/// Machine-readable catalog built from the clap definitions, so it cannot
/// drift from the commands the binary accepts.
fn command_catalog() -> Value {
    fn describe(cmd: &clap::Command) -> Value {
        let args: Vec<Value> = cmd
            .get_arguments()
            .filter(|a| !matches!(a.get_id().as_str(), "help" | "version"))
            .map(|a| {
                serde_json::json!({
                    "name": a.get_long().map(|l| format!("--{l}")).unwrap_or_else(|| a.get_id().to_string()),
                    "positional": a.is_positional(),
                    "required": a.is_required_set(),
                    "multiple": a.get_num_args().is_some_and(|n| n.max_values() > 1),
                    "flag": !a.get_action().takes_values(),
                    "values": if a.get_action().takes_values() {
                        a.get_possible_values().iter().map(|v| v.get_name().to_string()).collect::<Vec<_>>()
                    } else {
                        vec![]
                    },
                    "description": a.get_help().map(|h| h.to_string()),
                })
            })
            .collect();
        let mut out = serde_json::json!({
            "name": cmd.get_name(),
            "description": cmd.get_about().map(|h| h.to_string()),
            "args": args,
        });
        let subs: Vec<Value> = cmd.get_subcommands().map(describe).collect();
        if !subs.is_empty() {
            out["subcommands"] = Value::Array(subs);
        }
        out
    }
    let cli = Cli::command();
    serde_json::json!({
        "engine_version": ENGINE_VERSION,
        "commands": cli.get_subcommands().map(describe).collect::<Vec<_>>(),
    })
}
