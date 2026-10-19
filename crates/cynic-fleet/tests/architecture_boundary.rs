//! Architecture boundary tests for cynic-fleet.
//!
//! The whole crate stays zero-I/O: no filesystem, no processes, no network,
//! no terminal handling, no higher-layer cynic crates. Interactive rendering
//! belongs to `cynic-tui`.

use std::path::Path;

#[test]
fn no_dependency_on_higher_layer_crates() {
    let manifest = include_str!("../Cargo.toml");
    for forbidden in [
        "cynic-host",
        "cynic-kernel",
        "cynic-node",
        "cynic-cli",
        "cynic-inference",
        "cynic-intel",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "cynic-fleet must not depend on {forbidden}"
        );
    }
}

#[test]
fn no_io_in_sources() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    for file in ["src/types.rs", "src/report.rs", "src/lib.rs"] {
        let content = std::fs::read_to_string(Path::new(manifest_dir).join(file)).unwrap();
        for needle in [
            "std::fs",
            "std::process",
            "reqwest",
            "tokio::spawn",
            "tokio::fs",
        ] {
            assert!(
                !content.contains(needle),
                "{file} must not contain {needle}"
            );
        }
    }
}
