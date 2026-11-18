//! The committed spell baseline: SRD spell lift at every row (ADR 0020).
//!
//! Every modeled SRD spell is run through `spell_lift::run` against the committed spell panel, and
//! its rows are committed in `data/baseline/spell-lift.json` with the fingerprints of the inputs
//! that produced them. A homebrew run fights only its own spell and the control, and reads the SRD
//! rows from this table; the table says when its inputs are not the run's.

use serde::{Deserialize, Serialize};

use crate::baseline::{self, Fingerprints};
use crate::batch::{BatchError, Party};
use crate::effect::Lighting;
use crate::spell::SpellBook;
use crate::spell_lift::{self, LiftSpec, Row, INTERVAL_METHOD};
use crate::spell_panel::{self, SpellPanel};
use crate::tactics::PolicyConstants;

pub const TABLE_VERSION: u32 = 1;

/// The inputs a spell baseline was computed from: the monster curve's four, plus the effects file,
/// the panel and the fight count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellFingerprints {
    #[serde(flatten)]
    pub inputs: Fingerprints,
    /// Hash of the spell effects file.
    pub effects: String,
    /// Fingerprint of the spell panel's selection.
    pub panel: String,
    /// Fights per panel member in every row.
    pub fights: u32,
}

/// One SRD spell's rows. A row's per-member detail is left out of the table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellRows {
    pub id: String,
    pub name: String,
    pub level: u8,
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellBaseline {
    pub version: u32,
    pub fights_per_member: u32,
    pub max_rounds: u32,
    pub party: String,
    /// How each row's interval is stated.
    pub interval: String,
    pub fingerprints: SpellFingerprints,
    pub spells: Vec<SpellRows>,
}

/// Fingerprints for a run's inputs. The effects file is always the embedded one.
pub fn fingerprints_for(party: &Party, constants: &PolicyConstants, panel: &SpellPanel, fights: u32) -> SpellFingerprints {
    SpellFingerprints { inputs: baseline::fingerprints_for(party, constants), effects: spell_panel::effects_hash(), panel: panel.fingerprint.clone(), fights }
}

impl SpellBaseline {
    pub fn spell(&self, id: &str) -> Option<&SpellRows> {
        self.spells.iter().find(|s| s.id == id)
    }

    /// The row of `id` at one party level and cast level; a cantrip has no cast level.
    pub fn row(&self, id: &str, party_level: u8, cast_level: Option<u8>) -> Option<&Row> {
        self.spell(id)?.rows.iter().find(|r| r.plan.party_level == party_level && r.plan.cast_level == cast_level)
    }

    /// Ways the inputs of a run differ from the ones this table was computed from.
    pub fn stale_reasons(&self, run: &SpellFingerprints) -> Vec<String> {
        let base = &self.fingerprints;
        let mut out = baseline::stale_reasons(&base.inputs, &run.inputs, &self.party, "spell baseline");
        if base.effects != run.effects {
            out.push("the spell effects file changed".to_string());
        }
        if base.panel != run.panel {
            out.push("the spell panel differs from the one the spell baseline was computed against".to_string());
        }
        if base.fights != run.fights {
            out.push(format!("this run fights {} per panel member and the spell baseline used {}", run.fights, base.fights));
        }
        out
    }
}

/// Everything `generate` needs.
pub struct GenerateSpec<'a> {
    pub party_name: String,
    pub party: &'a Party,
    pub panel: &'a SpellPanel,
    /// Fights per panel member in every row.
    pub fights: u32,
    pub max_rounds: u32,
    pub threads: usize,
    /// Only these SRD spell ids, or every modeled spell when empty.
    pub only: Vec<String>,
}

/// Run every modeled SRD spell through the lift runner. Refuses a panel that is stale against the
/// inputs it will be run with, because every row would then be stale at birth. `progress` gets
/// `(done, total, spell id)` as each spell finishes.
pub fn generate(spec: &GenerateSpec, progress: &(dyn Fn(usize, usize, &str) + Sync)) -> Result<SpellBaseline, BatchError> {
    let now = fingerprints_for(spec.party, &PolicyConstants::default(), spec.panel, spec.fights);
    let stale = spec.panel.stale_reasons(&now.inputs);
    if !stale.is_empty() {
        return Err(BatchError::Invalid(format!("the spell panel is stale ({}); regenerate it with `bench baseline panel` first", stale.join("; "))));
    }
    let book = SpellBook::srd().map_err(|e| BatchError::Invalid(e.into()))?;
    let mut entries = book.srd_entries();
    if let Some(unknown) = spec.only.iter().find(|id| !entries.iter().any(|s| &s.id == *id)) {
        return Err(BatchError::NotFound(format!("`{unknown}` is not an SRD spell in the effects file")));
    }
    if !spec.only.is_empty() {
        entries.retain(|s| spec.only.contains(&s.id));
    }
    let spells = baseline::run_parallel(
        &entries,
        spec.threads,
        |s| &s.id,
        |s| {
            let lift = LiftSpec {
                spell_id: s.id.clone(),
                homebrew: vec![],
                party_name: spec.party_name.clone(),
                party: spec.party,
                panel: spec.panel,
                fights: spec.fights,
                max_rounds: spec.max_rounds,
                assumptions: vec![],
                lighting: Lighting::Unspecified,
                threads: 1,
                invocation: None,
            };
            let report = spell_lift::run(&lift, &|_, _, _| {})?;
            let rows = report.rows.into_iter().map(|row| Row { members: vec![], ..row }).collect();
            Ok(SpellRows { id: s.id.clone(), name: s.name.clone(), level: s.level, rows })
        },
        progress,
    )?;
    Ok(SpellBaseline {
        version: TABLE_VERSION,
        fights_per_member: spec.fights,
        max_rounds: spec.max_rounds,
        party: spec.party_name.clone(),
        interval: INTERVAL_METHOD.into(),
        fingerprints: now,
        spells,
    })
}

/// The committed table embedded at build time, if there was one.
pub fn embedded() -> Option<Result<SpellBaseline, BatchError>> {
    crate::embedded::spell_lift_json()
        .map(|text| serde_json::from_str(text).map_err(|e| BatchError::Invalid(format!("embedded spell baseline is invalid: {e}"))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::batch;
    use crate::spell_lift::RowStatus;

    fn panel() -> SpellPanel {
        spell_panel::embedded().expect("the spell panel is committed").expect("the panel parses")
    }

    fn spec<'a>(party: &'a Party, panel: &'a SpellPanel, only: &[&str]) -> GenerateSpec<'a> {
        GenerateSpec {
            party_name: "standard".into(),
            party,
            panel,
            fights: 2,
            max_rounds: batch::DEFAULT_MAX_ROUNDS,
            threads: 2,
            only: only.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn table() -> SpellBaseline {
        let panel = panel();
        let party = batch::standard_party().unwrap();
        let fingerprints = fingerprints_for(&party, &PolicyConstants::default(), &panel, 100);
        SpellBaseline { version: TABLE_VERSION, fights_per_member: 100, max_rounds: 20, party: "standard".into(), interval: INTERVAL_METHOD.into(), fingerprints, spells: vec![] }
    }

    #[test]
    fn a_table_is_current_against_its_own_inputs_and_names_each_input_that_moved() {
        let table = table();
        let now = table.fingerprints.clone();
        assert!(table.stale_reasons(&now).is_empty());
        let moved = |edit: &dyn Fn(&mut SpellFingerprints)| {
            let mut run = now.clone();
            edit(&mut run);
            table.stale_reasons(&run)
        };
        assert_eq!(moved(&|f| f.effects = "other".into()), vec!["the spell effects file changed"]);
        assert_eq!(moved(&|f| f.panel = "other".into()), vec!["the spell panel differs from the one the spell baseline was computed against"]);
        assert_eq!(moved(&|f| f.fights = 40), vec!["this run fights 40 per panel member and the spell baseline used 100"]);
        assert!(moved(&|f| f.inputs.party = "other".into())[0].contains("spell baseline"));
        assert!(moved(&|f| f.inputs.policy_version += 1)[0].contains("tactics policy"));
        assert!(moved(&|f| f.inputs.engine_version = "0.0.1".into())[0].contains("the engine is 0.0.1"));
        assert!(moved(&|f| f.inputs.fixture_set = "other".into())[0].contains("fixture set"));
    }

    #[test]
    fn generate_gives_each_spell_its_rows_without_member_detail() {
        let (panel, party) = (panel(), batch::standard_party().unwrap());
        let table = generate(&spec(&party, &panel, &["sleep", "magic-missile"]), &|_, _, _| {}).unwrap();
        assert_eq!(table.spells.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), vec!["magic-missile", "sleep"]);
        let sleep = table.row("sleep", 1, Some(1)).expect("sleep has its headline row");
        assert!(sleep.members.is_empty());
        assert_eq!(sleep.status, RowStatus::Measured);
        assert!(table.row("sleep", 3, Some(2)).is_none(), "sleep has no upcast rule");
        assert_eq!(table.spell("magic-missile").unwrap().rows.len(), 9, "a spell with an upcast rule keeps one row per cast level");
        assert_eq!(table.fingerprints, fingerprints_for(&party, &PolicyConstants::default(), &panel, 2));
        assert!(table.stale_reasons(&fingerprints_for(&party, &PolicyConstants::default(), &panel, 2)).is_empty());
        assert_eq!(table.fights_per_member, 2);
    }

    #[test]
    fn a_table_round_trips_through_json_and_a_row_matches_a_direct_run() {
        let (panel, party) = (panel(), batch::standard_party().unwrap());
        let table = generate(&spec(&party, &panel, &["sleep"]), &|_, _, _| {}).unwrap();
        let again: SpellBaseline = serde_json::from_str(&serde_json::to_string(&table).unwrap()).unwrap();
        assert_eq!(serde_json::to_value(&again).unwrap(), serde_json::to_value(&table).unwrap());
        let lift = LiftSpec {
            spell_id: "sleep".into(),
            homebrew: vec![],
            party_name: "standard".into(),
            party: &party,
            panel: &panel,
            fights: 2,
            max_rounds: batch::DEFAULT_MAX_ROUNDS,
            assumptions: vec![],
            lighting: Lighting::Unspecified,
            threads: 1,
            invocation: None,
        };
        let direct = spell_lift::run(&lift, &|_, _, _| {}).unwrap();
        let row = table.row("sleep", 1, Some(1)).unwrap();
        assert_eq!(row.win_lift, direct.rows[0].win_lift);
        assert_eq!(row.control_win_rate, direct.rows[0].control_win_rate);
    }

    #[test]
    fn generate_refuses_a_stale_panel_and_an_unknown_spell() {
        let (mut panel, party) = (panel(), batch::standard_party().unwrap());
        let err = generate(&spec(&party, &panel, &["nonesuch"]), &|_, _, _| {}).unwrap_err();
        assert!(err.to_string().contains("nonesuch"), "{err}");
        panel.inputs.engine_version = "0.0.1".into();
        let err = generate(&spec(&party, &panel, &["sleep"]), &|_, _, _| {}).unwrap_err();
        assert!(err.to_string().contains("bench baseline panel"), "{err}");
    }
}
