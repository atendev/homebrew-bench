//! The committed CR baseline curve and the spellcasting honesty check (tickets 032, 023).

use bench_core::baseline::{self, Baseline, GenerateSpec};
use bench_core::batch::{self, Band, BatchSpec};
use bench_core::embedded;
use bench_core::engine;
use bench_core::stat_block::StatBlock;
use bench_core::tactics::PolicyConstants;

fn fixture(id: &str) -> StatBlock {
    serde_json::from_str(embedded::fixture_json(id).unwrap_or_else(|| panic!("no fixture {id}"))).unwrap()
}

fn committed() -> Baseline {
    baseline::embedded().expect("data/baseline/curve.json is committed").expect("curve parses")
}

fn spec(id: &str, levels: Vec<u8>, fights: u32) -> BatchSpec {
    let monster = fixture(id);
    let (_, assumptions) = baseline::ignoring_unresolved(&monster);
    BatchSpec {
        monster,
        party_name: "standard".into(),
        party: batch::standard_party().unwrap(),
        levels,
        fights,
        max_rounds: batch::DEFAULT_MAX_ROUNDS,
        master_seed: 7,
        assumptions,
        baseline: None,
        invocation: None,
    }
}

/// Ticket 057 bumped the engine, ticket 058 changed the fixture set and ticket
/// 059 regenerated the standard party, all without regenerating the curve.
/// Ticket 056 regenerates it once and drops this allowance.
fn stale_beyond_the_engine(reasons: Vec<String>) -> Vec<String> {
    reasons
        .into_iter()
        .filter(|r| {
            !r.starts_with("the engine is ")
                && r != "the embedded fixture set changed"
                && r != "the party differs from the `standard` party the curve was computed against"
        })
        .collect()
}

#[test]
fn committed_curve_matches_the_inputs_it_was_generated_from() {
    let curve = committed();
    let party = batch::standard_party().unwrap();
    let now = baseline::fingerprints_for(&party, &PolicyConstants::default());
    let stale = stale_beyond_the_engine(curve.stale_reasons(&now));
    assert!(stale.is_empty(), "regenerate with `bench baseline generate`: {stale:?}");
    assert_eq!(curve.fixtures.len() + curve.excluded.len(), embedded::FIXTURES.len());
    assert_eq!(serde_json::to_value(&curve.cells).unwrap(), serde_json::to_value(baseline::build_cells(&curve.fixtures)).unwrap());
    assert_eq!(curve.fights_per_cell, 100);
}

#[test]
fn a_fixture_regenerates_to_its_committed_row() {
    let curve = committed();
    let party = batch::standard_party().unwrap();
    let again = baseline::generate(
        &GenerateSpec {
            fixtures: vec![fixture("goblin-warrior")],
            party_name: "standard".into(),
            party: &party,
            base_seed: curve.base_seed,
            fights: curve.fights_per_cell,
            max_rounds: curve.max_rounds,
            threads: 1,
        },
        &|_, _, _| {},
    )
    .unwrap();
    let committed_row = curve.fixtures.iter().find(|f| f.id == "goblin-warrior").unwrap();
    assert_eq!(serde_json::to_value(&again.fixtures[0]).unwrap(), serde_json::to_value(committed_row).unwrap());
}

#[test]
fn control_effects_on_both_sides_do_not_recurse_forever() {
    let report = batch::run(&spec("dryad", (1..=20).collect(), 2)).unwrap();
    assert_eq!(report.summary.levels.len(), 20);
}

fn band_at(id: &str, level: u8) -> Band {
    batch::run(&spec(id, vec![level], 30)).unwrap().summary.levels[0].band
}

#[test]
fn spellcasting_fixtures_keep_their_difficulty_bands() {
    use Band::*;
    let expected: &[(&str, u8, &[Band])] = &[
        ("archmage", 1, &[High]),
        ("archmage", 20, &[Low, Moderate]),
        ("mage", 2, &[High]),
        ("mage", 12, &[Low]),
        ("priest", 1, &[High, Moderate]),
        ("priest", 10, &[Low]),
        ("couatl", 1, &[High]),
        ("couatl", 10, &[Low]),
        ("lich", 10, &[High]),
    ];
    for (id, level, allowed) in expected {
        let band = band_at(id, *level);
        assert!(allowed.contains(&band), "{id} at level {level} reads {band:?}, expected one of {allowed:?}");
    }
}

#[test]
fn modeled_spells_cast_and_missing_ones_are_named() {
    for id in ["archmage", "mage", "priest"] {
        let s = spec(id, vec![8], 6);
        for i in 0..6 {
            let log = batch::replay_fight(&s, batch::fight_index(8, i, 6)).unwrap().log.unwrap();
            assert!(!log.contains("not in effects file") && !log.contains("effects file unavailable"), "{id}: {log}");
        }
    }
    let curve = committed();
    for block in baseline::embedded_fixtures().unwrap() {
        let row = curve.fixtures.iter().find(|f| f.id == block.id).unwrap();
        assert_eq!(row.missing_spells, engine::missing_spells(&block), "{} missing spells drifted from the curve", block.id);
    }
    let giant = batch::run(&spec("cloud-giant", vec![5], 2)).unwrap();
    assert!(giant.summary.warnings.iter().any(|w| w.contains("fog-cloud") && w.contains("not in the spell effects file")));
    assert!(batch::run(&spec("archmage", vec![5], 2)).unwrap().summary.warnings.iter().all(|w| !w.contains("effects file")));
}

#[test]
fn a_batch_with_the_curve_states_a_verdict_and_flags_a_changed_party() {
    let mut s = spec("ogre", vec![2], 20);
    s.baseline = Some(committed());
    let report = batch::run(&s).unwrap();
    let comparison = report.summary.comparison.as_ref().unwrap();
    assert!(!comparison.verdict.is_empty() && comparison.basis.starts_with("Basis: "));
    let stale = stale_beyond_the_engine(comparison.stale.clone());
    assert!(stale.is_empty(), "{stale:?}");
    assert!(report.html.contains("Claimed-CR verdict") && report.html.contains("Encounter budget at level 2"));
    s.party.get_mut(&2).unwrap().pop();
    let stale = batch::run(&s).unwrap();
    assert!(stale.summary.warnings.iter().any(|w| w.starts_with("Stale baseline") && w.contains("party differs")));
}
