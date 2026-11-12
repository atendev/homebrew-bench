//! The control party and the spell panel (ADR 0020).
//!
//! Spell lift compares the standard party with a control party: the same party with the caster
//! role's spells of one level removed. Each party level a spell lift row sits at has a fixed panel
//! of baseline-curve fixtures the control party wins about 40 to 80% of the time, fought alone and
//! as a group. The panel is committed in `data/baseline/spell-panel.json` with a fingerprint of its
//! selection and of the inputs it was measured with, so the spell baseline and homebrew runs share it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::baseline::{self, Excluded, Fingerprints};
use crate::batch::{self, BatchError, BatchSpec, Party};
use crate::effect::Lighting;
use crate::spell::SpellBook;
use crate::stat_block::{AbilityEntry, StatBlock};

pub const PANEL_VERSION: u32 = 1;
/// Master seed the committed panel is measured from. Each member derives its own from this, its id and its shape.
pub const DEFAULT_BASE_SEED: u64 = 20_261_111;
/// Party levels with a spell lift row: 2L-1 for spell levels 1 to 9. The cantrip rows at 1, 5, 11 and 17 fall among them.
pub const PANEL_LEVELS: [u8; 9] = [1, 3, 5, 7, 9, 11, 13, 15, 17];
/// Control win rates a fixture must fall within, inclusive.
pub const BAND: (f64, f64) = (0.40, 0.80);
/// The win rate selection prefers: the middle of `BAND`.
pub const BAND_CENTER: f64 = 0.60;
/// Most fixtures in each shape at one party level.
pub const PER_SHAPE: usize = 3;
/// Copies of one fixture in the group shape, one per party member.
pub const GROUP_COPIES: u32 = 4;

/// How many copies of a fixture a panel fight fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    Single,
    /// `GROUP_COPIES` copies of a fixture the control party beats too often alone.
    Group,
}

impl Shape {
    pub fn copies(self) -> u32 {
        match self {
            Shape::Single => 1,
            Shape::Group => GROUP_COPIES,
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            Shape::Single => "single",
            Shape::Group => "group",
        }
    }
}

/// One fixture in one shape, fought by the control party at one party level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Member {
    pub id: String,
    pub cr: f64,
    pub copies: u32,
    /// Master seed of its fights. A spell run fights it with the same seed, so each fight pairs with a control fight (ADR 0007).
    pub seed: u64,
    /// It casts a spell the SRD effects file models.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub spellcaster: bool,
    pub fights: u32,
    pub wins: u32,
    /// Fights with at least one PC death.
    pub deaths: u32,
    pub win_rate: f64,
}

/// The panel at one party level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelLevel {
    pub party_level: u8,
    /// The spell level the control party lacks when the panel is measured.
    pub control_spell_level: u8,
    pub single: Vec<Member>,
    pub group: Vec<Member>,
    /// The single-shape fixture that casts spells, or `None` when no fixture in the band does;
    /// counter spells then read "not measured" at this level.
    pub spellcasting_fixture: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellPanel {
    pub version: u32,
    /// Hash of the selection: each level's control spell level and each member's shape, id, copies and seed.
    pub fingerprint: String,
    pub base_seed: u64,
    pub fights: u32,
    pub max_rounds: u32,
    pub band: [f64; 2],
    pub party: String,
    /// The inputs the control runs were measured with. The party is the standard party the control is taken from.
    pub inputs: Fingerprints,
    /// Hash of the spell effects file, which the control transform reads spell levels from.
    pub effects: String,
    pub excluded: Vec<Excluded>,
    pub levels: Vec<PanelLevel>,
}

impl SpellPanel {
    pub fn level(&self, party_level: u8) -> Option<&PanelLevel> {
        self.levels.iter().find(|l| l.party_level == party_level)
    }

    /// Ways the inputs of a run differ from the ones this panel was measured with.
    pub fn stale_reasons(&self, run: &Fingerprints) -> Vec<String> {
        let mut out = baseline::stale_reasons(&self.inputs, run, &self.party, "panel");
        if self.effects != effects_hash() {
            out.push("the spell effects file changed".to_string());
        }
        if self.fingerprint != fingerprint(&self.levels) {
            out.push("the panel's members no longer match its fingerprint".to_string());
        }
        out
    }
}

/// The spell level the panel at `party_level` is measured without: the highest the caster has a slot for.
pub fn control_spell_level(party_level: u8) -> u8 {
    party_level.div_ceil(2)
}

/// `block` without each ability that casts a spell of `spell_level`. Spell slots and every other
/// ability stay. A cast of a spell `book` lacks stays, since its level is unknown.
pub fn without_spell_level(block: &StatBlock, spell_level: u8, book: &SpellBook) -> StatBlock {
    let casts_level = |entry: &AbilityEntry| {
        crate::stat_block::casts_of(entry).iter().any(|c| book.get(&c.spell_id).is_some_and(|s| s.level == spell_level))
    };
    let mut out = block.clone();
    for list in [&mut out.traits, &mut out.actions, &mut out.bonus_actions, &mut out.reactions, &mut out.legendary_actions] {
        list.retain(|entry| !casts_level(entry));
    }
    out
}

pub(crate) fn is_caster(block: &StatBlock) -> bool {
    crate::party::role_rank(&block.id).is_some_and(|rank| crate::party::ARCHETYPES[rank] == "caster")
}

/// The control party: `party` with the caster role's spells of `spell_level` removed at every level.
pub fn control_party(party: &Party, spell_level: u8, book: &SpellBook) -> Party {
    party
        .iter()
        .map(|(&level, members)| {
            let members = members.iter().map(|m| if is_caster(m) { without_spell_level(m, spell_level, book) } else { m.clone() });
            (level, members.collect())
        })
        .collect()
}

/// Hash of the panel's selection. Measured win rates are left out, so only a change of member,
/// shape, copies, seed or control spell level changes it.
pub fn fingerprint(levels: &[PanelLevel]) -> String {
    #[derive(Serialize)]
    struct Key<'a> {
        party_level: u8,
        control_spell_level: u8,
        shape: Shape,
        id: &'a str,
        copies: u32,
        seed: u64,
    }
    let keys: Vec<Key> = levels
        .iter()
        .flat_map(|l| {
            [(Shape::Single, &l.single), (Shape::Group, &l.group)].into_iter().flat_map(move |(shape, members)| {
                members.iter().map(move |m| Key {
                    party_level: l.party_level,
                    control_spell_level: l.control_spell_level,
                    shape,
                    id: &m.id,
                    copies: m.copies,
                    seed: m.seed,
                })
            })
        })
        .collect();
    baseline::hash_json(&keys)
}

/// Hash of the embedded spell effects file, independent of key order and formatting.
pub fn effects_hash() -> String {
    let value: serde_json::Value = serde_json::from_str(crate::embedded::spell_effects_json()).expect("the embedded effects file parses");
    baseline::hash_json(&value)
}

/// Master seed of a member's fights.
pub fn member_seed(base: u64, id: &str, shape: Shape) -> u64 {
    baseline::fixture_seed(base, &format!("{id}:{}", shape.word()))
}

pub(crate) fn in_band(win_rate: f64) -> bool {
    (BAND.0 - 1e-9..=BAND.1 + 1e-9).contains(&win_rate)
}

/// Members in the band, nearest `BAND_CENTER` first, then by id.
fn ranked(pool: &[Member]) -> Vec<&Member> {
    let mut out: Vec<&Member> = pool.iter().filter(|m| in_band(m.win_rate)).collect();
    out.sort_by(|a, b| (a.win_rate - BAND_CENTER).abs().total_cmp(&(b.win_rate - BAND_CENTER).abs()).then_with(|| a.id.cmp(&b.id)));
    out
}

/// The panel at one party level from the control measurements of every fixture in each shape.
/// Each shape keeps up to `PER_SHAPE` members in the band, nearest its middle. The single shape
/// keeps the best-placed spellcaster in the band in place of its last pick when none of its picks casts.
/// A group member's CR is below every single member's.
pub fn select(party_level: u8, singles: &[Member], groups: &[Member]) -> PanelLevel {
    let pick = |pool: &[Member]| ranked(pool).into_iter().take(PER_SHAPE).cloned().collect::<Vec<_>>();
    let mut single = pick(singles);
    if !single.iter().any(|m| m.spellcaster) {
        if let Some(caster) = ranked(singles).into_iter().find(|m| m.spellcaster) {
            single.pop();
            single.push(caster.clone());
        }
    }
    let spellcasting_fixture = single.iter().find(|m| m.spellcaster).map(|m| m.id.clone());
    let lowest = single.iter().map(|m| m.cr).fold(f64::INFINITY, f64::min);
    let lower: Vec<Member> = groups.iter().filter(|m| m.cr < lowest).cloned().collect();
    PanelLevel { party_level, control_spell_level: control_spell_level(party_level), single, group: pick(&lower), spellcasting_fixture }
}

/// Everything `generate` needs.
pub struct GenerateSpec<'a> {
    pub fixtures: Vec<StatBlock>,
    pub party_name: String,
    pub party: &'a Party,
    pub levels: Vec<u8>,
    pub base_seed: u64,
    pub fights: u32,
    pub max_rounds: u32,
    pub threads: usize,
}

struct Job<'a> {
    level: u8,
    block: &'a StatBlock,
    shape: Shape,
    label: String,
}

impl<'a> Job<'a> {
    fn new(level: u8, block: &'a StatBlock, shape: Shape) -> Self {
        Self { level, block, shape, label: format!("level {level} {} {}", block.id, shape.word()) }
    }
}

fn measure(spec: &GenerateSpec, controls: &BTreeMap<u8, Party>, job: &Job, book: &SpellBook) -> Result<(u8, Member), BatchError> {
    let (_, assumptions) = baseline::ignoring_unresolved(job.block);
    let seed = member_seed(spec.base_seed, &job.block.id, job.shape);
    let run = BatchSpec {
        monster: job.block.clone(),
        monster_copies: job.shape.copies(),
        party_name: spec.party_name.clone(),
        party: controls[&job.level].clone(),
        levels: vec![job.level],
        fights: spec.fights,
        max_rounds: spec.max_rounds,
        master_seed: seed,
        assumptions,
        lighting: Lighting::Unspecified,
        baseline: None,
        invocation: None,
        homebrew_spells: vec![],
    };
    let results = batch::run_fights(&run)?;
    let cell = batch::cell(job.level, &results.iter().collect::<Vec<_>>());
    let spellcaster = serde_json::to_value(job.block)
        .map(|v| crate::effect::casts_in(&v).iter().any(|c| book.contains(&c.spell_id)))
        .unwrap_or(false);
    Ok((
        job.level,
        Member {
            id: job.block.id.clone(),
            cr: job.block.challenge_rating.unwrap_or_default(),
            copies: job.shape.copies(),
            seed,
            spellcaster,
            fights: cell.fights,
            wins: cell.wins,
            deaths: cell.deaths,
            win_rate: (cell.win_rate * 10_000.0).round() / 10_000.0,
        },
    ))
}

/// Fight every fixture alone against the control party at each level, then fight the fixtures it
/// beats too often alone as groups, and select the panel. `progress` gets `(done, total, label)`
/// as each fixture and shape finishes, counted per pass.
pub fn generate(spec: &GenerateSpec, progress: &(dyn Fn(usize, usize, &str) + Sync)) -> Result<SpellPanel, BatchError> {
    let book = SpellBook::srd().map_err(|e| BatchError::Invalid(e.into()))?;
    let mut controls = BTreeMap::new();
    for &level in &spec.levels {
        let members = spec.party.get(&level).ok_or_else(|| BatchError::NotFound(format!("party `{}` has no stat blocks for level {level}", spec.party_name)))?;
        let one = Party::from([(level, members.clone())]);
        controls.insert(level, control_party(&one, control_spell_level(level), &book));
    }
    let (rated, unrated): (Vec<&StatBlock>, Vec<&StatBlock>) = spec.fixtures.iter().partition(|b| b.challenge_rating.is_some());
    let mut excluded: Vec<Excluded> = unrated.iter().map(|b| Excluded { id: b.id.clone(), reason: "no challenge rating".into() }).collect();
    excluded.sort_by(|a, b| a.id.cmp(&b.id));

    let alone: Vec<Job> = spec.levels.iter().flat_map(|&l| rated.iter().map(move |b| Job::new(l, b, Shape::Single))).collect();
    let singles = baseline::run_parallel(&alone, spec.threads, |j| &j.label, |j| measure(spec, &controls, j, &book), progress)?;
    let too_easy: Vec<Job> = alone
        .iter()
        .zip(&singles)
        .filter(|(_, (_, m))| m.win_rate > BAND.1 + 1e-9)
        .map(|(j, _)| Job::new(j.level, j.block, Shape::Group))
        .collect();
    let groups = baseline::run_parallel(&too_easy, spec.threads, |j| &j.label, |j| measure(spec, &controls, j, &book), progress)?;

    let at = |all: &[(u8, Member)], level: u8| all.iter().filter(|(l, _)| *l == level).map(|(_, m)| m.clone()).collect::<Vec<_>>();
    let levels: Vec<PanelLevel> = spec.levels.iter().map(|&l| select(l, &at(&singles, l), &at(&groups, l))).collect();
    Ok(SpellPanel {
        version: PANEL_VERSION,
        fingerprint: fingerprint(&levels),
        base_seed: spec.base_seed,
        fights: spec.fights,
        max_rounds: spec.max_rounds,
        band: [BAND.0, BAND.1],
        party: spec.party_name.clone(),
        inputs: baseline::fingerprints_for(spec.party, &Default::default()),
        effects: effects_hash(),
        excluded,
        levels,
    })
}

/// The committed panel embedded at build time, if there was one.
pub fn embedded() -> Option<Result<SpellPanel, BatchError>> {
    crate::embedded::spell_panel_json()
        .map(|text| serde_json::from_str(text).map_err(|e| BatchError::Invalid(format!("embedded spell panel is invalid: {e}"))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book() -> SpellBook<'static> {
        SpellBook::srd().unwrap()
    }

    fn cast_levels(block: &StatBlock) -> Vec<(String, u8)> {
        let book = book();
        [&block.actions, &block.bonus_actions, &block.reactions]
            .into_iter()
            .flatten()
            .flat_map(|e| crate::stat_block::casts_of(e).into_iter().map(|c| (e.id.clone(), book.get(&c.spell_id).unwrap().level)))
            .collect()
    }

    fn lower(pool: &[Member]) -> Vec<Member> {
        pool.iter().map(|m| Member { cr: 0.5, ..m.clone() }).collect()
    }

    fn member(id: &str, win_rate: f64, spellcaster: bool) -> Member {
        Member { id: id.into(), cr: 1.0, copies: 1, seed: 7, spellcaster, fights: 100, wins: (win_rate * 100.0) as u32, deaths: 0, win_rate }
    }

    #[test]
    fn control_spell_level_is_the_highest_slot_the_caster_has() {
        let levels: Vec<u8> = PANEL_LEVELS.iter().map(|&l| control_spell_level(l)).collect();
        assert_eq!(levels, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn the_control_removes_only_the_casters_spells_of_one_level_and_keeps_its_slots() {
        let party = batch::standard_party().unwrap();
        let control = control_party(&party, 2, &book());
        let before = party[&5].iter().find(|b| is_caster(b)).unwrap();
        let after = control[&5].iter().find(|b| is_caster(b)).unwrap();
        assert!(cast_levels(before).iter().any(|(_, l)| *l == 2), "the level 5 caster knows level 2 spells");
        let kept = cast_levels(after);
        assert!(kept.iter().all(|(_, l)| *l != 2), "{kept:?}");
        let expected: Vec<(String, u8)> = cast_levels(before).into_iter().filter(|(_, l)| *l != 2).collect();
        assert_eq!(kept, expected, "every spell of another level stays, cantrips and the Shield reaction included");
        assert_eq!(after.spell_slots, before.spell_slots);
        assert!(after.actions.iter().any(|a| a.id == "dagger"));
        for (level, members) in &party {
            for (a, b) in members.iter().zip(&control[level]) {
                if !is_caster(a) {
                    assert_eq!(serde_json::to_value(a).unwrap(), serde_json::to_value(b).unwrap(), "{} changed", a.id);
                }
            }
        }
    }

    #[test]
    fn a_level_one_control_drops_the_shield_reaction() {
        let party = batch::standard_party().unwrap();
        let caster = control_party(&party, 1, &book())[&5].iter().find(|b| is_caster(b)).unwrap().clone();
        assert!(caster.reactions.iter().all(|r| r.id != "shield"));
        assert!(caster.actions.iter().any(|a| a.id == "fire-bolt"), "cantrips are level 0 and stay");
    }

    #[test]
    fn selection_keeps_members_in_the_band_nearest_its_middle() {
        let pool = vec![
            member("a", 0.39, false),
            member("b", 0.40, false),
            member("c", 0.62, false),
            member("d", 0.80, false),
            member("e", 0.81, false),
            member("f", 0.55, false),
            member("g", 0.58, false),
        ];
        let level = select(5, &pool, &lower(&pool));
        let ids = |ms: &[Member]| ms.iter().map(|m| m.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(&level.single), vec!["c", "g", "f"]);
        assert_eq!(ids(&level.group), vec!["c", "g", "f"]);
        assert_eq!(level.control_spell_level, 3);
        let edges = select(5, &[member("a", 0.39, false), member("b", 0.40, false), member("d", 0.80, false), member("e", 0.81, false)], &[]);
        assert_eq!(ids(&edges.single), vec!["b", "d"], "the band's ends count");
        assert!(edges.group.is_empty());
    }

    #[test]
    fn group_members_sit_below_every_single_members_cr() {
        let at = |id: &str, cr: f64| Member { cr, ..member(id, 0.60, false) };
        let level = select(7, &[at("big", 8.0), at("bigger", 11.0)], &[at("low", 3.0), at("even", 8.0), at("high", 12.0)]);
        assert_eq!(level.group.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), vec!["low"]);
        let no_singles = select(7, &[], &[at("high", 12.0)]);
        assert_eq!(no_singles.group.len(), 1, "with no single member there is no CR to stay under");
    }

    #[test]
    fn selection_keeps_a_spellcaster_in_the_band_or_records_there_is_none() {
        let pool = vec![member("a", 0.60, false), member("b", 0.61, false), member("c", 0.62, false), member("mage", 0.75, true), member("lich", 0.90, true)];
        let level = select(9, &pool, &[]);
        assert_eq!(level.single.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), vec!["a", "b", "mage"]);
        assert_eq!(level.spellcasting_fixture.as_deref(), Some("mage"));
        let none = select(9, &pool[..3], &[]);
        assert_eq!(none.spellcasting_fixture, None);
        let text = serde_json::to_string(&none).unwrap();
        assert!(text.contains(r#""spellcasting_fixture":null"#), "the absence is written out: {text}");
    }

    #[test]
    fn the_fingerprint_changes_with_the_selection_and_not_with_measurements() {
        let pool = vec![member("a", 0.60, false), member("b", 0.50, false)];
        let levels = vec![select(3, &pool, &lower(&pool))];
        let base = fingerprint(&levels);
        let mut remeasured = levels.clone();
        remeasured[0].single[0].win_rate = 0.70;
        remeasured[0].single[0].wins = 70;
        assert_eq!(fingerprint(&remeasured), base);
        let changes: [fn(&mut PanelLevel); 5] = [
            |l| l.single[0].id = "z".into(),
            |l| l.single[0].seed = 8,
            |l| l.group[0].copies = 5,
            |l| l.control_spell_level = 1,
            |l| {
                let m = l.single.remove(0);
                l.group.push(m);
            },
        ];
        for change in changes {
            let mut changed = levels.clone();
            change(&mut changed[0]);
            assert_ne!(fingerprint(&changed), base);
        }
    }

    #[test]
    fn member_seeds_differ_by_shape_and_fixture() {
        assert_ne!(member_seed(1, "ogre", Shape::Single), member_seed(1, "ogre", Shape::Group));
        assert_ne!(member_seed(1, "ogre", Shape::Single), member_seed(1, "orc", Shape::Single));
        assert_eq!(member_seed(1, "ogre", Shape::Group), member_seed(1, "ogre", Shape::Group));
    }

    fn fixture(id: &str) -> StatBlock {
        serde_json::from_str(crate::embedded::fixture_json(id).unwrap()).unwrap()
    }

    #[test]
    fn generate_fights_groups_only_of_fixtures_too_easy_alone_and_pairs_seeds() {
        let party = batch::standard_party().unwrap();
        let spec = GenerateSpec {
            fixtures: vec![fixture("goblin-warrior"), fixture("ogre"), fixture("mage")],
            party_name: "standard".into(),
            party: &party,
            levels: vec![5],
            base_seed: 3,
            fights: 20,
            max_rounds: batch::DEFAULT_MAX_ROUNDS,
            threads: 2,
        };
        let panel = generate(&spec, &|_, _, _| {}).unwrap();
        assert_eq!(panel.levels.len(), 1);
        let level = &panel.levels[0];
        assert_eq!(level.control_spell_level, 3);
        for m in level.single.iter().chain(&level.group) {
            assert!(in_band(m.win_rate), "{} at {}", m.id, m.win_rate);
            assert_eq!(m.fights, 20);
        }
        for m in &level.group {
            assert_eq!(m.copies, GROUP_COPIES);
            assert_eq!(m.seed, member_seed(3, &m.id, Shape::Group));
        }
        assert_eq!(panel.fingerprint, fingerprint(&panel.levels));
        assert!(panel.stale_reasons(&baseline::fingerprints_for(&party, &Default::default())).is_empty());
    }

    #[test]
    fn a_control_fight_and_a_spell_fight_with_one_seed_share_their_fight_seeds() {
        let party = batch::standard_party().unwrap();
        let run = |party: Party| {
            let spec = BatchSpec {
                monster: fixture("ogre"),
                monster_copies: 2,
                party_name: "standard".into(),
                party,
                levels: vec![3],
                fights: 5,
                max_rounds: batch::DEFAULT_MAX_ROUNDS,
                master_seed: member_seed(1, "ogre", Shape::Group),
                assumptions: vec![],
                lighting: Lighting::Unspecified,
                baseline: None,
                invocation: None,
                homebrew_spells: vec![],
            };
            batch::run_fights(&spec).unwrap().iter().map(|r| (r.index, r.seed)).collect::<Vec<_>>()
        };
        assert_eq!(run(control_party(&party, 2, &book())), run(party.clone()));
    }
}
