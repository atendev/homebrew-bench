//! Every committed SRD fixture must deserialize into the canonical stat block schema.

use bench_core::stat_block::StatBlock;
use std::path::Path;

#[test]
fn srd_fixtures_validate_against_stat_block_schema() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/fixtures");
    let mut count = 0;
    let mut failures = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("data/fixtures exists") {
        let path = entry.expect("readable directory entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("readable fixture");
        if let Err(err) = serde_json::from_str::<StatBlock>(&text) {
            failures.push(format!("{}: {err}", path.display()));
        }
        count += 1;
    }
    assert!(count > 0, "no fixtures found in {}", dir.display());
    assert!(failures.is_empty(), "fixtures failing the schema:\n{}", failures.join("\n"));
}

fn action_clauses(fixture: &str, action_id: &str) -> Vec<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/fixtures").join(format!("{fixture}.json"));
    let text = std::fs::read_to_string(&path).expect("readable fixture");
    let block: serde_json::Value = serde_json::from_str(&text).expect("fixture is JSON");
    let action = block["actions"]
        .as_array()
        .expect("actions array")
        .iter()
        .find(|a| a["id"] == action_id)
        .unwrap_or_else(|| panic!("{fixture} has no action {action_id}"));
    action["agent_clauses"]
        .as_array()
        .map(|clauses| clauses.iter().map(|c| c.as_str().expect("string clause").to_string()).collect())
        .unwrap_or_default()
}

#[test]
fn hit_sentence_tail_effects_become_agent_clauses() {
    assert_eq!(
        action_clauses("clay-golem", "slam"),
        ["The target\u{2019}s Hit Point maximum decreases by an amount equal to the Acid damage taken."]
    );
    let otyugh = action_clauses("otyugh", "bite");
    assert_eq!(otyugh.len(), 2, "{otyugh:?}");
    assert_eq!(otyugh[0], "The target has the Poisoned condition.");
    let roper = action_clauses("roper", "tentacle");
    assert_eq!(roper.len(), 2, "{roper:?}");
    assert_eq!(
        roper[0],
        "The target has the Grappled condition (escape DC 14) from one of six tentacles, and the target has the Poisoned condition until the grapple ends."
    );
    assert_eq!(
        action_clauses("sprite", "enchanting-bow"),
        ["The target has the Charmed condition until the start of the sprite\u{2019}s next turn."]
    );
}

#[test]
fn conditional_damage_tails_are_not_duplicated_as_effects() {
    assert_eq!(
        action_clauses("goblin-warrior", "scimitar"),
        ["Conditional damage, not modeled: plus 2 (1d4) Slashing damage if the attack roll had Advantage."]
    );
}

fn multiattack_steps(fixture: &str) -> Vec<bench_core::effect::MultiattackStep> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/fixtures").join(format!("{fixture}.json"));
    let block: StatBlock = serde_json::from_str(&std::fs::read_to_string(&path).expect("readable fixture")).expect("valid fixture");
    let action = block.actions.into_iter().find(|a| a.id == "multiattack").expect("multiattack action");
    match action.effects.into_iter().next() {
        Some(bench_core::effect::Effect::Multiattack(multi)) => multi.steps,
        other => panic!("{fixture} multiattack is {other:?}"),
    }
}

#[test]
fn all_srd_monsters_graduate() {
    let count = std::fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/fixtures"))
        .expect("data/fixtures exists")
        .filter(|e| e.as_ref().is_ok_and(|e| e.path().extension().is_some_and(|x| x == "json")))
        .count();
    assert_eq!(count, 341);
}

#[test]
fn multiattack_spell_options_cast_through_spellcasting() {
    use bench_core::effect::MultiattackStep;
    let steps = multiattack_steps("adult-black-dragon");
    let MultiattackStep::Choice { options, pick: 1 } = &steps[1] else { panic!("{steps:?}") };
    let cast = options[1].cast.as_ref().expect("acid arrow option casts");
    assert_eq!((options[1].action_id.as_str(), cast.spell_id.as_str(), cast.slot_level, cast.upcast), ("spellcasting", "acid-arrow", Some(3), true));
}

#[test]
fn grouped_and_optional_multiattack_steps_become_alternatives() {
    use bench_core::effect::MultiattackStep;
    let medusa = multiattack_steps("medusa");
    let [MultiattackStep::Alternatives { sets }] = medusa.as_slice() else { panic!("{medusa:?}") };
    assert_eq!(sets.iter().map(Vec::len).collect::<Vec<_>>(), [2, 1]);
    let erinyes = multiattack_steps("erinyes");
    let MultiattackStep::Alternatives { sets } = &erinyes[1] else { panic!("{erinyes:?}") };
    assert!(sets[1].is_empty(), "Entangling Rope is optional");
}
