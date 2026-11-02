//! Reference data compiled into the binary (ADR 0013).
//!
//! Tables hold `(path relative to the data root, file text)` pairs, sorted by
//! path. The tables come from `build.rs`.

mod tables {
    include!(concat!(env!("OUT_DIR"), "/embedded_data.rs"));
}

pub use tables::{BASELINE, FIXTURES, PARTY, SPELLS};

fn find(table: &'static [(&'static str, &'static str)], path: &str) -> Option<&'static str> {
    table.binary_search_by(|(p, _)| (*p).cmp(path)).ok().map(|i| table[i].1)
}

fn valid_slug(slug: &str) -> bool {
    !slug.is_empty() && slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Text of the SRD fixture with this id.
pub fn fixture_json(id: &str) -> Option<&'static str> {
    valid_slug(id).then(|| find(FIXTURES, &format!("{id}.json"))).flatten()
}

/// Text of the standard party member with this id.
pub fn party_member_json(id: &str) -> Option<&'static str> {
    valid_slug(id).then(|| find(PARTY, &format!("standard/{id}.json"))).flatten()
}

/// A bare slug resolved against fixtures, then standard party members.
pub fn stat_block_json(slug: &str) -> Option<&'static str> {
    fixture_json(slug).or_else(|| party_member_json(slug))
}

/// The engine-owned spell effects file.
pub fn spell_effects_json() -> &'static str {
    find(SPELLS, "effects.json").expect("data/spells/effects.json is embedded")
}

/// The committed baseline curve, when `data/baseline/curve.json` existed at build time.
pub fn baseline_json() -> Option<&'static str> {
    find(BASELINE, "curve.json")
}

/// Standard party stat block files.
pub fn standard_party_json() -> impl Iterator<Item = (&'static str, &'static str)> {
    PARTY.iter().filter(|(p, _)| p.starts_with("standard/")).map(|(p, t)| (*p, *t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_are_sorted_and_populated() {
        for table in [FIXTURES, SPELLS, PARTY, BASELINE] {
            assert!(table.windows(2).all(|w| w[0].0 < w[1].0));
        }
        assert!(FIXTURES.len() >= 341);
        assert_eq!(standard_party_json().count(), 80);
        assert!(fixture_json("ogre").is_some());
        assert!(party_member_json("standard-defender-01").is_some());
        assert!(stat_block_json("../ogre").is_none());
        assert!(spell_effects_json().starts_with('{'));
    }
}
