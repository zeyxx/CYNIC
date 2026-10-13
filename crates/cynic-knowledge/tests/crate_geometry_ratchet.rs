use cynic_knowledge::{WORKSPACE_GEOMETRY_SCHEMA_V1, load_workspace_geometry};
use std::path::Path;

#[test]
fn workspace_geometry_ratchet_holds() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.parent().unwrap().parent().unwrap();

    let geom = load_workspace_geometry(repo_root).expect("Should load workspace geometry");

    // Conformance
    assert_eq!(geom.schema, WORKSPACE_GEOMETRY_SCHEMA_V1);
    assert!(
        geom.total_crates >= 40,
        "Expected at least 40 crates, found {}",
        geom.total_crates
    );
    assert!(
        geom.total_loc > 10_000,
        "Expected > 10,000 LOC across crates, found {}",
        geom.total_loc
    );

    // Foundation Layer L0 must exist and contain core types
    let l0_crates = geom.crates_in_layer("L0");
    assert!(
        !l0_crates.is_empty(),
        "L0 foundation layer must not be empty"
    );
    let l0_names: Vec<&str> = l0_crates.iter().map(|c| c.name.as_str()).collect();
    assert!(
        l0_names.contains(&"cynic-types"),
        "cynic-types must be in L0"
    );

    // Layer L1 (Pure Domain)
    let l1_crates = geom.crates_in_layer("L1");
    assert!(
        !l1_crates.is_empty(),
        "L1 pure domain layer must not be empty"
    );

    // Conversion to DocumentChunk must be sound
    for c in geom.crates.values() {
        let chunk = c.to_chunk();
        assert!(chunk.source_path.starts_with("crate://"));
        assert!(!chunk.content.is_empty());
    }
}
