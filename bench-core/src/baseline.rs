//! The empirical CR baseline curve (ADR 0011).
//!
//! Every graduated SRD fixture fights the standard party at levels 1 to 20
//! under the batch tactics policy. Per-fixture results are committed in
//! `data/baseline/curve.json` together with per-`(CR, level)` cells and the
//! fingerprints of the inputs that produced them. A report compares a monster
//! against these cells and warns when its own inputs differ from the baseline's.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::encounter::StandingAssumption;
use crate::batch::{self, Band, BatchError, BatchSpec, Cell, Party, Spread};
use crate::stat_block::StatBlock;
use crate::tactics::{self, PolicyConstants};

pub const BASELINE_VERSION: u32 = 1;
/// Master seed the committed curve is generated from. Each fixture derives its own from this and its id.
pub const DEFAULT_BASE_SEED: u64 = 20_261_102;
/// Fewest fixtures a `(CR, level)` cell may rest on before it is labeled thin.
pub const MIN_FIXTURES: usize = 3;
/// Challenge ratings in ladder order. Pooling reaches one step either way.
pub const CR_LADDER: [f64; 34] = [
    0.0, 0.125, 0.25, 0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0,
    18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0, 25.0, 26.0, 27.0, 28.0, 29.0, 30.0,
];

/// The inputs a baseline was computed from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fingerprints {
    pub fixture_set: String,
    pub party: String,
    pub policy_id: String,
    pub policy_version: u32,
    pub policy: String,
    pub engine_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Range3 {
    pub p10: f64,
    pub median: f64,
    pub p90: f64,
}

impl From<&Spread> for Range3 {
    fn from(s: &Spread) -> Self {
        Self { p10: s.p10, median: s.median, p90: s.p90 }
    }
}

/// One fixture at one party level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureCell {
    pub level: u8,
    pub fights: u32,
    pub wins: u32,
    pub deaths: u32,
    pub stalemates: u32,
    pub rounds_median: f64,
    pub dpr_median: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureRow {
    pub id: String,
    pub cr: f64,
    /// Spell ids the fixture casts that the effects file lacks; those casts do nothing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_spells: Vec<String>,
    /// Abilities with unmodeled parts or agent clauses; the fixture fought with them left out.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignored_abilities: Vec<String>,
    pub cells: Vec<FixtureCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Excluded {
    pub id: String,
    pub reason: String,
}

/// All fixtures of one CR, or of the CR and its ladder neighbours, at one level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrCell {
    pub cr: f64,
    pub level: u8,
    pub fixtures: u32,
    /// CRs whose fixtures fill this cell; more than one means pooling.
    pub pooled_crs: Vec<f64>,
    pub thin: bool,
    pub fights: u32,
    pub wins: u32,
    pub deaths: u32,
    pub win_rate: f64,
    pub death_rate: f64,
    pub band: Band,
    /// Spread of the per-fixture median rounds.
    pub rounds: Range3,
    /// Spread of the per-fixture median monster damage per round.
    pub dpr: Range3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    pub version: u32,
    pub base_seed: u64,
    pub fights_per_cell: u32,
    pub max_rounds: u32,
    pub party: String,
    pub fingerprints: Fingerprints,
    pub excluded: Vec<Excluded>,
    pub fixtures: Vec<FixtureRow>,
    pub cells: Vec<CrCell>,
}

/// Hex SHA-256 of a value's JSON. The value goes through `serde_json::Value`
/// first so hash-map fields come out in key order and the hash is stable.
fn hash_json<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_value(value).and_then(|v| serde_json::to_vec(&v)).expect("stat blocks and parties serialize");
    hex::encode(Sha256::digest(bytes))
}

/// Hash of a fixture set, independent of order and file formatting.
pub fn fixture_set_hash(blocks: &[StatBlock]) -> String {
    let mut sorted: Vec<&StatBlock> = blocks.iter().collect();
    sorted.sort_by(|a, b| a.id.cmp(&b.id));
    hash_json(&sorted)
}

/// Hash of a party table.
pub fn party_hash(party: &Party) -> String {
    hash_json(party)
}

/// Every embedded SRD fixture, parsed.
pub fn embedded_fixtures() -> Result<Vec<StatBlock>, BatchError> {
    crate::embedded::FIXTURES
        .iter()
        .map(|(path, text)| serde_json::from_str(text).map_err(|e| BatchError::Invalid(format!("embedded fixture {path}: {e}"))))
        .collect()
}

fn embedded_fixture_set_hash() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| fixture_set_hash(&embedded_fixtures().expect("embedded fixtures parse")))
}

/// Fingerprints for a run's inputs. The fixture set is always the embedded one.
pub fn fingerprints_for(party: &Party, constants: &PolicyConstants) -> Fingerprints {
    Fingerprints {
        fixture_set: embedded_fixture_set_hash().to_string(),
        party: party_hash(party),
        policy_id: tactics::POLICY_ID.into(),
        policy_version: tactics::POLICY_VERSION,
        policy: tactics::fingerprint(constants),
        engine_version: crate::ENGINE_VERSION.into(),
    }
}

/// The committed baseline embedded at build time, if there was one.
pub fn embedded() -> Option<Result<Baseline, BatchError>> {
    crate::embedded::baseline_json()
        .map(|text| serde_json::from_str(text).map_err(|e| BatchError::Invalid(format!("embedded baseline is invalid: {e}"))))
}

fn same_cr(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

pub fn cr_word(cr: f64) -> String {
    match cr {
        c if same_cr(c, 0.125) => "1/8".into(),
        c if same_cr(c, 0.25) => "1/4".into(),
        c if same_cr(c, 0.5) => "1/2".into(),
        c => format!("{c}"),
    }
}

fn band_rank(band: Band) -> u8 {
    match band {
        Band::Low => 0,
        Band::Moderate => 1,
        Band::High => 2,
    }
}

impl Baseline {
    pub fn cell(&self, cr: f64, level: u8) -> Option<&CrCell> {
        self.cells.iter().find(|c| same_cr(c.cr, cr) && c.level == level)
    }

    /// Ways the inputs of a run differ from the ones this baseline was computed from.
    pub fn stale_reasons(&self, run: &Fingerprints) -> Vec<String> {
        let base = &self.fingerprints;
        let mut out = Vec::new();
        if base.fixture_set != run.fixture_set {
            out.push("the embedded fixture set changed".to_string());
        }
        if base.party != run.party {
            out.push(format!("the party differs from the `{}` party the curve was computed against", self.party));
        }
        if base.policy_id != run.policy_id || base.policy_version != run.policy_version {
            out.push(format!(
                "the tactics policy is {} v{} and the curve used {} v{}",
                run.policy_id, run.policy_version, base.policy_id, base.policy_version
            ));
        } else if base.policy != run.policy {
            out.push("the tactics policy constants differ from the curve's".to_string());
        }
        if base.engine_version != run.engine_version {
            out.push(format!("the engine is {} and the curve used {}", run.engine_version, base.engine_version));
        }
        out
    }
}

/// How a run reads against the baseline, in four neutral parts plus the SRD facts kept in the summary.
#[derive(Debug, Clone, Serialize)]
pub struct Comparison {
    /// `matches`, `harder`, `easier`, `no_threat` or `unavailable`.
    pub verdict_kind: String,
    pub verdict: String,
    pub effective_cr: String,
    pub anomalies: Vec<String>,
    /// Baseline cells the comparison used that rest on thin data.
    pub thin_cells: Vec<String>,
    pub basis: String,
    pub stale: Vec<String>,
}

/// Inputs to `compare` from a finished run.
pub struct RunReading<'a> {
    pub claimed_cr: Option<f64>,
    pub cells: &'a [Cell],
    pub representative_level: Option<u8>,
    pub effective_cr: &'a str,
    pub fingerprints: &'a Fingerprints,
}

pub fn compare(baseline: &Baseline, run: &RunReading) -> Comparison {
    let stale = baseline.stale_reasons(run.fingerprints);
    let mut out = Comparison {
        verdict_kind: "unavailable".into(),
        verdict: String::new(),
        effective_cr: run.effective_cr.to_string(),
        anomalies: vec![],
        thin_cells: vec![],
        basis: String::new(),
        stale,
    };
    let Some(cr) = run.claimed_cr else {
        out.verdict = "The stat block states no challenge rating, so there is no claimed-CR verdict.".into();
        out.basis = basis_line(baseline, None);
        return out;
    };
    let name = cr_word(cr);
    let rep = run.representative_level.and_then(|l| run.cells.iter().find(|c| c.level == l));
    let reference = rep.and_then(|c| baseline.cell(cr, c.level).map(|b| (c, b)));
    match reference {
        None => {
            out.verdict = format!("The baseline has no fixtures near CR {name}, so there is no claimed-CR verdict.");
        }
        Some((mine, base)) => {
            let (kind, text) = verdict(mine, base, &name);
            out.verdict_kind = kind.into();
            out.verdict = text;
            if base.thin {
                out.thin_cells.push(thin_label(base));
            }
        }
    }
    for c in run.cells {
        let Some(base) = baseline.cell(cr, c.level) else { continue };
        let before = out.anomalies.len();
        out.anomalies.extend(anomaly("rounds", c.level, c.rounds.median, base.rounds, base, &name));
        out.anomalies.extend(anomaly("monster damage per round", c.level, c.monster_dpr.median, base.dpr, base, &name));
        if base.thin && out.anomalies.len() > before {
            let label = thin_label(base);
            if !out.thin_cells.contains(&label) {
                out.thin_cells.push(label);
            }
        }
    }
    out.basis = basis_line(baseline, Some(cr));
    out
}

fn thin_label(base: &CrCell) -> String {
    format!("CR {} at level {} ({} {})", cr_word(base.cr), base.level, base.fixtures, if base.fixtures == 1 { "fixture" } else { "fixtures" })
}

fn verdict(mine: &Cell, base: &CrCell, name: &str) -> (&'static str, String) {
    let pct = |x: f64| format!("{:.0}%", x * 100.0);
    let facts = format!(
        "win rate {} and PC deaths in {} of fights for this monster, against {} and {} for the fixtures",
        pct(mine.win_rate),
        pct(f64::from(mine.deaths) / f64::from(mine.fights.max(1))),
        pct(base.win_rate),
        pct(base.death_rate)
    );
    let level = mine.level;
    let (a, b) = (band_rank(mine.band), band_rank(base.band));
    if a == b {
        ("matches", format!("At party level {level} this monster reads {}, the same band as SRD fixtures of CR {name} ({facts}).", mine.band.word()))
    } else if a > b {
        ("harder", format!("At party level {level} this monster reads {}, above the {} band of SRD fixtures of CR {name} ({facts}).", mine.band.word(), base.band.word()))
    } else if mine.wins == mine.fights && mine.deaths == 0 {
        ("no_threat", format!("At party level {level} this monster is no threat: the party won every fight with no PC death, where SRD fixtures of CR {name} read {} ({facts}).", base.band.word()))
    } else {
        ("easier", format!("At party level {level} this monster reads {}, below the {} band of SRD fixtures of CR {name} ({facts}).", mine.band.word(), base.band.word()))
    }
}

fn anomaly(what: &str, level: u8, value: f64, spread: Range3, base: &CrCell, name: &str) -> Option<String> {
    // One fixture has no spread, so any difference would read as an anomaly.
    if base.fixtures < 2 {
        return None;
    }
    let direction = if value < spread.p10 {
        "below"
    } else if value > spread.p90 {
        "above"
    } else {
        return None;
    };
    let thin = if base.thin { format!("; thin data, {} fixtures", base.fixtures) } else { String::new() };
    Some(format!(
        "Level {level}: median {what} {value:.1} is {direction} the P10 to P90 spread {:.1} to {:.1} of SRD fixtures of CR {name}{thin}.",
        spread.p10, spread.p90
    ))
}

fn basis_line(baseline: &Baseline, cr: Option<f64>) -> String {
    let fixtures = match cr {
        Some(cr) => {
            let any = baseline.cells.iter().find(|c| same_cr(c.cr, cr));
            match any {
                Some(c) if c.pooled_crs.len() > 1 => {
                    let list: Vec<String> = c.pooled_crs.iter().map(|x| cr_word(*x)).collect();
                    format!("the SRD fixtures of CR {} pooled with their neighbours (CR {})", cr_word(cr), list.join(", "))
                }
                Some(_) => format!("the SRD fixtures of CR {}", cr_word(cr)),
                None => "the SRD fixtures".into(),
            }
        }
        None => "the SRD fixtures".into(),
    };
    format!(
        "Basis: {fixtures}, each fought {} times per party level by the {} party under {} v{}; the bands are Homebrew Bench calibration, not SRD tables.",
        baseline.fights_per_cell, baseline.party, baseline.fingerprints.policy_id, baseline.fingerprints.policy_version
    )
}

/// Everything `generate` needs.
pub struct GenerateSpec<'a> {
    pub fixtures: Vec<StatBlock>,
    pub party_name: String,
    pub party: &'a Party,
    pub base_seed: u64,
    pub fights: u32,
    pub max_rounds: u32,
    pub threads: usize,
}

fn fixture_seed(base: u64, id: &str) -> u64 {
    let digest = Sha256::digest(format!("{base}:{id}"));
    u64::from_le_bytes(digest[..8].try_into().expect("digest has 32 bytes"))
}

enum Done {
    Row(FixtureRow),
    Excluded(Excluded),
}

/// Ids of the abilities only an agent could resolve, with the standing assumptions that leave them
/// out. The curve is the monster as the engine plays it, the same as `--assume abilities.<id>=ignored`.
pub fn ignoring_unresolved(block: &StatBlock) -> (Vec<String>, Vec<StandingAssumption>) {
    let ids = batch::unresolved(block, &BTreeMap::new());
    let assumptions = ids
        .iter()
        .map(|id| StandingAssumption {
            path: format!("abilities.{id}"),
            value: "ignored".into(),
            reason: "baseline fixtures leave out what only an agent could resolve".into(),
        })
        .collect();
    (ids, assumptions)
}

fn run_fixture(spec: &GenerateSpec, block: &StatBlock) -> Result<Done, BatchError> {
    let exclude = |reason: String| Ok(Done::Excluded(Excluded { id: block.id.clone(), reason }));
    let Some(cr) = block.challenge_rating else { return exclude("no challenge rating".into()) };
    let (ignored_abilities, assumptions) = ignoring_unresolved(block);
    let run = BatchSpec {
        monster: block.clone(),
        party_name: spec.party_name.clone(),
        party: spec.party.clone(),
        levels: (1..=20).collect(),
        fights: spec.fights,
        max_rounds: spec.max_rounds,
        master_seed: fixture_seed(spec.base_seed, &block.id),
        assumptions,
        baseline: None,
    };
    let results = batch::run_fights(&run)?;
    let cells = run
        .levels
        .iter()
        .map(|&level| {
            let c = batch::cell(level, &results.iter().filter(|r| r.level == level).collect::<Vec<_>>());
            FixtureCell {
                level,
                fights: c.fights,
                wins: c.wins,
                deaths: c.deaths,
                stalemates: c.stalemates,
                rounds_median: c.rounds.median,
                dpr_median: c.monster_dpr.median,
            }
        })
        .collect();
    Ok(Done::Row(FixtureRow { id: block.id.clone(), cr, missing_spells: crate::engine::missing_spells(block), ignored_abilities, cells }))
}

/// Run every fixture against the party and build the curve. `progress` gets
/// `(done, total, fixture id)` as each fixture finishes.
pub fn generate(spec: &GenerateSpec, progress: &(dyn Fn(usize, usize, &str) + Sync)) -> Result<Baseline, BatchError> {
    let total = spec.fixtures.len();
    let next = AtomicUsize::new(0);
    let finished = AtomicUsize::new(0);
    let done: Mutex<Vec<Done>> = Mutex::new(Vec::new());
    let failure: Mutex<Option<BatchError>> = Mutex::new(None);
    std::thread::scope(|scope| {
        for _ in 0..spec.threads.max(1) {
            // Fights recurse deeply enough to overflow the 2 MiB default thread stack.
            let worker = std::thread::Builder::new().stack_size(256 << 20);
            let spawned = worker.spawn_scoped(scope, || loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                let Some(block) = spec.fixtures.get(i) else { break };
                if failure.lock().expect("lock").is_some() {
                    break;
                }
                match run_fixture(spec, block) {
                    Ok(d) => done.lock().expect("lock").push(d),
                    Err(e) => {
                        *failure.lock().expect("lock") = Some(e);
                        break;
                    }
                }
                progress(finished.fetch_add(1, Ordering::SeqCst) + 1, total, &block.id);
            });
            if let Err(e) = spawned {
                *failure.lock().expect("lock") = Some(BatchError::Invalid(format!("cannot start a worker thread: {e}")));
                break;
            }
        }
    });
    if let Some(e) = failure.into_inner().expect("lock") {
        return Err(e);
    }
    let mut fixtures = Vec::new();
    let mut excluded = Vec::new();
    for d in done.into_inner().expect("lock") {
        match d {
            Done::Row(r) => fixtures.push(r),
            Done::Excluded(e) => excluded.push(e),
        }
    }
    fixtures.sort_by(|a, b| a.id.cmp(&b.id));
    excluded.sort_by(|a, b| a.id.cmp(&b.id));
    let cells = build_cells(&fixtures);
    Ok(Baseline {
        version: BASELINE_VERSION,
        base_seed: spec.base_seed,
        fights_per_cell: spec.fights,
        max_rounds: spec.max_rounds,
        party: spec.party_name.clone(),
        fingerprints: fingerprints_for(spec.party, &PolicyConstants::default()),
        excluded,
        fixtures,
        cells,
    })
}

/// Per-`(CR, level)` cells. A CR with fewer than `MIN_FIXTURES` fixtures
/// pools the CRs one ladder step either side; a cell still short of that is thin.
pub fn build_cells(fixtures: &[FixtureRow]) -> Vec<CrCell> {
    let mut cells = Vec::new();
    for (i, &cr) in CR_LADDER.iter().enumerate() {
        let at = |j: usize| fixtures.iter().filter(move |f| same_cr(f.cr, CR_LADDER[j]));
        let own = at(i).count();
        let reach = if own >= MIN_FIXTURES { i..=i } else { i.saturating_sub(1)..=(i + 1).min(CR_LADDER.len() - 1) };
        let pooled_crs: Vec<f64> = reach.clone().filter(|&j| at(j).next().is_some()).map(|j| CR_LADDER[j]).collect();
        let members: Vec<&FixtureRow> = reach.flat_map(|j| at(j)).collect();
        if members.is_empty() {
            continue;
        }
        for level in 1..=20u8 {
            let rows: Vec<&FixtureCell> = members.iter().filter_map(|f| f.cells.iter().find(|c| c.level == level)).collect();
            if rows.is_empty() {
                continue;
            }
            let fights: u32 = rows.iter().map(|c| c.fights).sum();
            let wins: u32 = rows.iter().map(|c| c.wins).sum();
            let deaths: u32 = rows.iter().map(|c| c.deaths).sum();
            let win_rate = f64::from(wins) / f64::from(fights.max(1));
            let death_rate = f64::from(deaths) / f64::from(fights.max(1));
            let rounds = Spread::of(&rows.iter().map(|c| c.rounds_median).collect::<Vec<_>>());
            let dpr = Spread::of(&rows.iter().map(|c| c.dpr_median).collect::<Vec<_>>());
            cells.push(CrCell {
                cr,
                level,
                fixtures: rows.len() as u32,
                thin: rows.len() < MIN_FIXTURES,
                pooled_crs: pooled_crs.clone(),
                fights,
                wins,
                deaths,
                win_rate: (win_rate * 10_000.0).round() / 10_000.0,
                death_rate: (death_rate * 10_000.0).round() / 10_000.0,
                band: Band::of(win_rate, death_rate),
                rounds: Range3::from(&rounds),
                dpr: Range3::from(&dpr),
            });
        }
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, cr: f64, wins: u32, deaths: u32, rounds: f64) -> FixtureRow {
        FixtureRow {
            id: id.into(),
            cr,
            missing_spells: vec![],
            ignored_abilities: vec![],
            cells: vec![FixtureCell { level: 1, fights: 100, wins, deaths, stalemates: 0, rounds_median: rounds, dpr_median: rounds * 2.0 }],
        }
    }

    #[test]
    fn well_covered_cr_does_not_pool() {
        let rows = vec![row("a", 2.0, 100, 0, 3.0), row("b", 2.0, 100, 0, 4.0), row("c", 2.0, 100, 0, 5.0), row("d", 3.0, 0, 90, 9.0)];
        let cells = build_cells(&rows);
        let c = cells.iter().find(|c| same_cr(c.cr, 2.0) && c.level == 1).unwrap();
        assert_eq!((c.fixtures, c.thin, c.pooled_crs.clone()), (3, false, vec![2.0]));
        assert_eq!(c.band, Band::Low);
        assert_eq!(c.rounds.median, 4.0);
    }

    #[test]
    fn short_cr_pools_one_ladder_step_either_way_and_stays_thin_when_short() {
        let rows = vec![row("a", 11.0, 100, 0, 3.0), row("b", 12.0, 60, 40, 5.0), row("c", 13.0, 60, 40, 6.0)];
        let cells = build_cells(&rows);
        let c = cells.iter().find(|c| same_cr(c.cr, 12.0) && c.level == 1).unwrap();
        assert_eq!((c.fixtures, c.thin, c.pooled_crs.clone()), (3, false, vec![11.0, 12.0, 13.0]));
        assert_eq!(c.band, Band::High);
        let ends = cells.iter().find(|c| same_cr(c.cr, 11.0) && c.level == 1).unwrap();
        assert_eq!(ends.pooled_crs, vec![11.0, 12.0]);
        assert!(ends.thin, "two fixtures after pooling stay thin");
        assert!(cells.iter().all(|c| !same_cr(c.cr, 25.0)), "no fixtures near CR 25 means no cell");
    }

    #[test]
    fn fractional_crs_pool_along_the_ladder() {
        let rows = vec![row("a", 0.125, 100, 0, 2.0), row("b", 0.5, 100, 0, 2.0)];
        let cells = build_cells(&rows);
        let c = cells.iter().find(|c| same_cr(c.cr, 0.25) && c.level == 1).unwrap();
        assert_eq!(c.pooled_crs, vec![0.125, 0.5]);
        assert!(c.thin);
    }

    fn mine(level: u8, band_wins: u32, deaths: u32, rounds: f64, dpr: f64) -> Cell {
        let spread = |m: f64| Spread { median: m, mean: m, p10: m, p90: m };
        Cell {
            level,
            fights: 100,
            wins: band_wins,
            losses: 100 - band_wins,
            stalemates: 0,
            deaths,
            win_rate: f64::from(band_wins) / 100.0,
            rounds: spread(rounds),
            monster_dpr: spread(dpr),
            band: Band::of(f64::from(band_wins) / 100.0, f64::from(deaths) / 100.0),
            flags: vec![],
        }
    }

    fn curve(fixtures: usize) -> Baseline {
        let rows: Vec<FixtureRow> = (0..fixtures).map(|i| row(&format!("f{i}"), 1.0, 80, 10, 4.0 + i as f64)).collect();
        let party = batch::standard_party().unwrap();
        Baseline {
            version: BASELINE_VERSION,
            base_seed: 1,
            fights_per_cell: 100,
            max_rounds: 20,
            party: "standard".into(),
            fingerprints: fingerprints_for(&party, &PolicyConstants::default()),
            excluded: vec![],
            cells: build_cells(&rows),
            fixtures: rows,
        }
    }

    fn reading<'a>(cells: &'a [Cell], fp: &'a Fingerprints, cr: Option<f64>) -> RunReading<'a> {
        RunReading { claimed_cr: cr, cells, representative_level: Some(1), effective_cr: "1", fingerprints: fp }
    }

    #[test]
    fn verdicts_read_both_directions_and_no_threat() {
        let b = curve(3);
        let fp = b.fingerprints.clone();
        let kind = |cell: Cell| compare(&b, &reading(&[cell], &fp, Some(1.0))).verdict_kind;
        assert_eq!(b.cell(1.0, 1).unwrap().band, Band::Moderate);
        assert_eq!(kind(mine(1, 80, 10, 5.0, 10.0)), "matches");
        assert_eq!(kind(mine(1, 50, 60, 5.0, 10.0)), "harder");
        assert_eq!(kind(mine(1, 99, 0, 5.0, 10.0)), "easier");
        assert_eq!(kind(mine(1, 100, 0, 5.0, 10.0)), "no_threat");
    }

    #[test]
    fn anomalies_name_thin_cells_and_missing_claims_are_unavailable() {
        let b = curve(2);
        let fp = b.fingerprints.clone();
        let c = compare(&b, &reading(&[mine(1, 80, 10, 40.0, 1.0)], &fp, Some(1.0)));
        assert_eq!(c.anomalies.len(), 2, "{:?}", c.anomalies);
        assert!(c.anomalies[0].contains("above") && c.anomalies[1].contains("below"));
        assert_eq!(c.thin_cells, vec!["CR 1 at level 1 (2 fixtures)".to_string()]);
        assert!(c.basis.starts_with("Basis: ") && c.basis.contains("not SRD tables"));
        assert_eq!(compare(&b, &reading(&[mine(1, 80, 10, 4.0, 8.0)], &fp, None)).verdict_kind, "unavailable");
        assert_eq!(compare(&b, &reading(&[mine(1, 80, 10, 4.0, 8.0)], &fp, Some(25.0))).verdict_kind, "unavailable");
    }

    #[test]
    fn stale_reasons_name_each_changed_input() {
        let b = curve(3);
        let mut run = b.fingerprints.clone();
        assert!(b.stale_reasons(&run).is_empty());
        run.fixture_set = "x".into();
        run.party = "x".into();
        run.policy = "x".into();
        run.engine_version = "9.9.9".into();
        assert_eq!(b.stale_reasons(&run).len(), 4);
        run.policy_version = 99;
        assert!(b.stale_reasons(&run).iter().any(|r| r.contains("v99")));
    }

    #[test]
    fn hashes_do_not_depend_on_hash_map_order() {
        // Each parse builds its own hash maps, so their iteration order differs between the copies.
        let (fixtures_a, party_a) = (embedded_fixtures().unwrap(), batch::standard_party().unwrap());
        let (fixtures_b, party_b) = (embedded_fixtures().unwrap(), batch::standard_party().unwrap());
        assert_eq!(fixture_set_hash(&fixtures_a), fixture_set_hash(&fixtures_b));
        assert_eq!(party_hash(&party_a), party_hash(&party_b));
    }

    #[test]
    fn fingerprints_ignore_nothing_that_changes_inputs() {
        let party = batch::standard_party().unwrap();
        let a = fingerprints_for(&party, &PolicyConstants::default());
        assert_eq!(a, fingerprints_for(&party, &PolicyConstants::default()));
        let mut changed = party.clone();
        changed.get_mut(&1).unwrap()[0].name.push('!');
        assert_ne!(a.party, fingerprints_for(&changed, &PolicyConstants::default()).party);
    }
}
