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
