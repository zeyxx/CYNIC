//! Architecture boundary tests for cynic-traces.
//!
//! Tranche 1 owns zero I/O: pure domain entities plus the session aggregate.
//! Adapters (filesystem, SQLite, processes) land in tranche 2 behind ports.

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
        "cynic-knowledge",
        "rusqlite",
        "tokio",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "cynic-traces (tranche 1) must not depend on {forbidden}"
        );
    }
}

#[test]
fn no_io_in_sources() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    for file in [
        "src/domain.rs",
        "src/session.rs",
        "src/conservation.rs",
        "src/ports.rs",
        "src/kernel/contract.rs",
        "src/kernel/errors.rs",
        "src/kernel/ids.rs",
        "src/kernel/mod.rs",
        "src/lib.rs",
    ] {
        let content = std::fs::read_to_string(Path::new(manifest_dir).join(file)).unwrap();
        for needle in [
            "std::fs",
            "std::process",
            "std::net",
            "tokio::spawn",
            "tokio::fs",
            "rusqlite",
        ] {
            assert!(!content.contains(needle), "{file} must not contain {needle}");
        }
    }
}
